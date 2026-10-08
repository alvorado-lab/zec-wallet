import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/lifecycle/app_lifecycle_provider.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/bip39_wordlist.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_controller.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_address_scanner.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_screen.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

import '../../../support/dialog_reach.dart';

/// The onboarding SCREENS over the shipped money-safety gate (spec §3.2g
/// iii-B-2-a). Driven through the real [OnboardingController] against host-VM
/// fakes (no native lib, no device): the views render each phase and the only
/// path to a deposit-ready wallet runs welcome → reveal → confirm.
///
/// THE invariant under test, at the UI layer: the wallet surface (balance /
/// deposit affordances) appears ONLY after the recovery-phrase backup is
/// confirmed-and-persisted; every earlier phase shows the flow, never funds.
void main() {
  // Resolve l10n from a live element so finders couple to KEYS, not English
  // literals (the wallet_screen_test idiom).
  WalletLocalizations l10nOf(WidgetTester tester) =>
      WalletLocalizations.of(tester.element(find.byType(WalletScreen)));

  Widget harness({
    required FakeWalletProvisioner provisioner,
    required FakeOnboardingStore store,
    FakeScreenSecurity? security,
    List<Override> extraOverrides = const [],
  }) {
    return ProviderScope(
      overrides: [
        walletProvisionerProvider.overrideWithValue(provisioner),
        onboardingStoreProvider.overrideWithValue(store),
        // Deterministic BIP39 validity for the restore field (the real bundled
        // asset is irrelevant to the flow tests; the SDK is the gate).
        bip39WordlistProvider.overrideWith((ref) => testBip39Wordlist),
        if (security != null)
          screenSecurityProvider.overrideWithValue(security),
        ...extraOverrides,
      ],
      child: MaterialApp(
        localizationsDelegates: WalletLocalizations.localizationsDelegates,
        supportedLocales: WalletLocalizations.supportedLocales,
        theme: lightTheme,
        home: const WalletScreen(),
      ),
    );
  }

  // Drive welcome → create → AwaitingBackup → reveal: the shared prefix of the
  // confirm/residue tests. Leaves the words on screen, unconfirmed.
  Future<void> pumpToRevealedBackup(
    WidgetTester tester, {
    required FakeWalletProvisioner provisioner,
    required FakeOnboardingStore store,
    FakeScreenSecurity? security,
  }) async {
    await tester.pumpWidget(
      harness(provisioner: provisioner, store: store, security: security),
    );
    await tester.pumpAndSettle(); // boot probe → Welcome
    final l10n = l10nOf(tester);
    await tester.tap(find.text(l10n.walletCreateButton));
    await tester.pumpAndSettle(); // create → AwaitingBackup
    await tester.tap(find.text(l10n.walletBackupReveal));
    await tester.pumpAndSettle(); // reveal future → words
  }

  // The backup screen is taller than the 800×600 test viewport, so the checkbox
  // and Continue button sit below the fold — scroll them in before tapping.
  Future<void> tapVisible(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
  }

  // Drive to the revealed backup over a CONTROLLABLE lifecycle (the provider is
  // overridden, bypassing the platform binding) so the auto-hide policy can be
  // tested deterministically. Returns the lifecycle notifier; `emit` a state to
  // fire the view's `ref.listen`.
  Future<TestLifecycleNotifier> pumpToRevealedBackupL(
    WidgetTester tester, {
    required FakeWalletProvisioner provisioner,
    required FakeOnboardingStore store,
  }) async {
    final container = ProviderContainer(
      overrides: [
        walletProvisionerProvider.overrideWithValue(provisioner),
        onboardingStoreProvider.overrideWithValue(store),
        appLifecycleProvider.overrideWith(TestLifecycleNotifier.new),
      ],
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
    final l10n = l10nOf(tester);
    await tester.tap(find.text(l10n.walletCreateButton));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.walletBackupReveal));
    await tester.pumpAndSettle();
    return container.read(appLifecycleProvider.notifier)
        as TestLifecycleNotifier;
  }

  group('welcome', () {
    testWidgets('no backend wired → honest not-set-up, NO create offered', (
      tester,
    ) async {
      // No provisioner/store override → controller is Unavailable (the
      // production default); the surface must NOT offer onboarding actions.
      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const WalletScreen(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = l10nOf(tester);
      expect(find.text(l10n.walletNotSetUpTitle), findsOneWidget);
      expect(find.text(l10n.walletCreateButton), findsNothing);
    });

    testWidgets(
      'a wallet-less device offers BOTH create and restore (enabled)',
      (tester) async {
        await tester.pumpWidget(
          harness(
            provisioner: FakeWalletProvisioner(exists: false),
            store: FakeOnboardingStore(),
          ),
        );
        await tester.pumpAndSettle();
        final l10n = l10nOf(tester);

        expect(find.text(l10n.walletOnboardingWelcomeTitle), findsOneWidget);
        expect(find.text(l10n.walletCreateButton), findsOneWidget);
        // Restore is now a live, equal-weight entry — never a dead control.
        final restore = tester.widget<OutlinedButton>(
          find.widgetWithText(OutlinedButton, l10n.walletRestoreButton),
        );
        expect(restore.onPressed, isNotNull);
      },
    );
  });

  group('create → back up → confirm', () {
    testWidgets(
      'create moves to the backup screen, NOT a deposit-ready wallet',
      (tester) async {
        final provisioner = FakeWalletProvisioner(exists: false);
        await tester.pumpWidget(
          harness(provisioner: provisioner, store: FakeOnboardingStore()),
        );
        await tester.pumpAndSettle();
        final l10n = l10nOf(tester);

        await tester.tap(find.text(l10n.walletCreateButton));
        await tester.pumpAndSettle();

        expect(provisioner.createCount, 1);
        expect(find.text(l10n.walletBackupTitle), findsOneWidget);
        // Money-safety: a created-but-unconfirmed wallet shows NO balance and no
        // deposit/sync surface.
        expect(find.text(l10n.walletBalanceLabel), findsNothing);
      },
    );

    testWidgets('generation shows a busy view; no Create affordance in flight', (
      tester,
    ) async {
      // Hold createGenerated in flight to observe the transient OnboardingGenerating
      // window (the seed seal is local but can take a beat on a slow keychain).
      final hold = Completer<void>();
      final provisioner = FakeWalletProvisioner(exists: false)
        ..holdCreate = hold;
      await tester.pumpWidget(
        harness(provisioner: provisioner, store: FakeOnboardingStore()),
      );
      await tester.pumpAndSettle();
      final l10n = l10nOf(tester);

      await tester.tap(find.text(l10n.walletCreateButton));
      await tester.pump(); // → Generating (createGenerated awaiting the hold)

      // The honest busy view is shown and the create is counted exactly once;
      // there is no Create button to double-tap during generation (and the
      // controller's synchronous transition guards a re-entrant call anyway).
      expect(find.text(l10n.walletGeneratingLabel), findsOneWidget);
      expect(provisioner.createCount, 1);
      expect(find.text(l10n.walletCreateButton), findsNothing);

      hold.complete();
      await tester.pumpAndSettle(); // → AwaitingBackup
      expect(find.text(l10n.walletBackupTitle), findsOneWidget);
      expect(provisioner.createCount, 1); // still exactly one wallet created
    });

    testWidgets('words are revealed only on a deliberate tap (shoulder-surf)', (
      tester,
    ) async {
      final provisioner = FakeWalletProvisioner(exists: false);
      await tester.pumpWidget(
        harness(provisioner: provisioner, store: FakeOnboardingStore()),
      );
      await tester.pumpAndSettle();
      final l10n = l10nOf(tester);
      await tester.tap(find.text(l10n.walletCreateButton));
      await tester.pumpAndSettle();

      // Before the reveal tap: words are NOT fetched and NOT shown.
      expect(provisioner.revealCount, 0);
      expect(find.text('art'), findsNothing); // last word of the vector

      await tester.tap(find.text(l10n.walletBackupReveal));
      await tester.pumpAndSettle();

      expect(provisioner.revealCount, 1);
      // 24 vector words: 23 × 'abandon' + 1 × 'art', each in index order.
      expect(find.text('art'), findsOneWidget);
      expect(find.text('abandon'), findsNWidgets(23));
    });

    testWidgets('the recovery words are NOT selectable/copyable', (
      tester,
    ) async {
      await pumpToRevealedBackup(
        tester,
        provisioner: FakeWalletProvisioner(exists: false),
        store: FakeOnboardingStore(),
      );
      // A seed must never reach the shared/synced clipboard.
      expect(find.byType(SelectableText), findsNothing);
    });

    testWidgets('Continue is gated on the explicit confirmation checkbox', (
      tester,
    ) async {
      await pumpToRevealedBackup(
        tester,
        provisioner: FakeWalletProvisioner(exists: false),
        store: FakeOnboardingStore(),
      );
      final l10n = l10nOf(tester);

      // Before checking: Continue is disabled (the gate's deliberate-confirm).
      FilledButton continueBtn() => tester.widget<FilledButton>(
        find.widgetWithText(FilledButton, l10n.walletBackupContinue),
      );
      expect(continueBtn().onPressed, isNull);

      await tapVisible(tester, find.text(l10n.walletBackupConfirmCheckbox));
      await tester.pump();
      expect(continueBtn().onPressed, isNotNull);
    });

    testWidgets('confirm persists FIRST, THEN opens the deposit-ready wallet', (
      tester,
    ) async {
      final provisioner = FakeWalletProvisioner(exists: false);
      final store = FakeOnboardingStore();
      await pumpToRevealedBackup(
        tester,
        provisioner: provisioner,
        store: store,
      );
      final l10n = l10nOf(tester);

      await tapVisible(tester, find.text(l10n.walletBackupConfirmCheckbox));
      await tester.pump();
      await tapVisible(tester, find.text(l10n.walletBackupContinue));
      await tester.pumpAndSettle();

      // The flag was durably persisted true, and only now is the wallet
      // deposit-ready (the live surface — the balance card — is shown; sync
      // runs on its own, no manual control).
      expect(store.persisted, isTrue);
      expect(store.lastSetValue, isTrue);
      expect(find.text(l10n.walletBalanceLabel), findsOneWidget);
      // The backup screen is gone (no leftover recovery words on the surface).
      expect(find.text(l10n.walletBackupTitle), findsNothing);
      expect(find.text('art'), findsNothing);
    });

    testWidgets('a confirmed wallet boots straight to the live surface', (
      tester,
    ) async {
      // The resume fork: a wallet on disk + a confirmed flag = deposit-ready at
      // once (no re-forced backup).
      await tester.pumpWidget(
        harness(
          provisioner: FakeWalletProvisioner(exists: true),
          store: FakeOnboardingStore(confirmed: true),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = l10nOf(tester);

      expect(find.text(l10n.walletBalanceLabel), findsOneWidget);
      expect(find.text(l10n.walletBackupTitle), findsNothing);
    });

    testWidgets(
      'a provisioned-but-UNCONFIRMED wallet resumes into forced backup',
      (tester) async {
        // The crux money-safety invariant at the UI: a wallet on disk whose backup
        // was never confirmed resumes to the backup screen, NEVER deposit-ready.
        await tester.pumpWidget(
          harness(
            provisioner: FakeWalletProvisioner(exists: true),
            store: FakeOnboardingStore(confirmed: false),
          ),
        );
        await tester.pumpAndSettle();
        final l10n = l10nOf(tester);

        expect(find.text(l10n.walletBackupTitle), findsOneWidget);
        expect(find.text(l10n.walletBalanceLabel), findsNothing);
      },
    );
  });

  group('screen security (screenshot/recents protection)', () {
    testWidgets('engaged while the backup screen is shown, released after', (
      tester,
    ) async {
      final provisioner = FakeWalletProvisioner(exists: false);
      final store = FakeOnboardingStore();
      final security = FakeScreenSecurity();
      await pumpToRevealedBackup(
        tester,
        provisioner: provisioner,
        store: store,
        security: security,
      );
      // Protection requested on the backup screen, not yet released.
      expect(security.enableCount, 1);
      expect(security.disableCount, 0);

      final l10n = l10nOf(tester);
      await tapVisible(tester, find.text(l10n.walletBackupConfirmCheckbox));
      await tester.pump();
      await tapVisible(tester, find.text(l10n.walletBackupContinue));
      await tester.pumpAndSettle(); // → Active, backup view leaves the tree

      // Released exactly once when the sensitive screen unmounted.
      expect(security.disableCount, 1);
    });

    testWidgets(
      'where screenshots CAN be blocked (Android), the copy says so',
      (tester) async {
        await pumpToRevealedBackup(
          tester,
          provisioner: FakeWalletProvisioner(exists: false),
          store: FakeOnboardingStore(),
          security: FakeScreenSecurity(isScreenshotBlockSupported: true),
        );
        final l10n = l10nOf(tester);
        expect(find.text(l10n.walletBackupSecureNoteAndroid), findsOneWidget);
        expect(find.text(l10n.walletBackupSecureNoteOther), findsNothing);
      },
    );

    testWidgets(
      'where there is no block (desktop/iOS), it advises a private setting',
      (tester) async {
        await pumpToRevealedBackup(
          tester,
          provisioner: FakeWalletProvisioner(exists: false),
          store: FakeOnboardingStore(),
          security: FakeScreenSecurity(isScreenshotBlockSupported: false),
        );
        final l10n = l10nOf(tester);
        // Honest: no protection it doesn't have — advise privacy instead.
        expect(find.text(l10n.walletBackupSecureNoteOther), findsOneWidget);
        expect(find.text(l10n.walletBackupSecureNoteAndroid), findsNothing);
      },
    );

    testWidgets(
      'a capable platform whose host never wired the native handler stays '
      'on the honest unprotected copy (S149 review B1 — no false claim)',
      (tester) async {
        await pumpToRevealedBackup(
          tester,
          provisioner: FakeWalletProvisioner(exists: false),
          store: FakeOnboardingStore(),
          // Android-shaped capability, but enable() never gets a native ack —
          // the exact shape of a host that skipped the MainActivity wiring.
          security: FakeScreenSecurity(
            isScreenshotBlockSupported: true,
            engages: false,
          ),
        );
        final l10n = l10nOf(tester);
        expect(find.text(l10n.walletBackupSecureNoteOther), findsOneWidget);
        expect(find.text(l10n.walletBackupSecureNoteAndroid), findsNothing);
      },
    );
  });

  group('background safety (shoulder-surf / recents snapshot)', () {
    testWidgets('inactive (foreground focus-noise) does NOT hide the words', (
      tester,
    ) async {
      final provisioner = FakeWalletProvisioner(exists: false);
      final lifecycle = await pumpToRevealedBackupL(
        tester,
        provisioner: provisioner,
        store: FakeOnboardingStore(),
      );
      expect(find.text('art'), findsOneWidget);

      // A notification-shade pull / call banner / Control Center / desktop
      // alt-tab fires `inactive` — a transient foreground interruption that
      // takes NO OS snapshot. The words must STAY: hiding on every interruption
      // would clear them mid-backup (the perverse incentive to screenshot) and
      // make desktop multitasking unusable. (Matches the flutter-patterns rule
      // the sync stream follows: act on the real background, not focus-noise.)
      lifecycle.emit(AppLifecycleState.inactive);
      await tester.pumpAndSettle();
      expect(
        find.text('art'),
        findsOneWidget,
        reason: 'inactive must not hide the seed',
      );
      expect(provisioner.revealCount, 1); // no re-fetch churn either

      lifecycle.emit(AppLifecycleState.resumed);
      await tester.pumpAndSettle();
      expect(find.text('art'), findsOneWidget); // still shown, no re-tap needed
    });

    testWidgets(
      'backgrounding (hidden/paused) hides the words and drops the cache',
      (tester) async {
        final provisioner = FakeWalletProvisioner(exists: false);
        final lifecycle = await pumpToRevealedBackupL(
          tester,
          provisioner: provisioner,
          store: FakeOnboardingStore(),
        );
        final l10n = l10nOf(tester);
        expect(find.text('art'), findsOneWidget); // words visible
        expect(provisioner.revealCount, 1);

        // `hidden` is the OS snapshot moment (iOS app-switcher + desktop minimize);
        // the words must be gone by then. (Both `hidden` and `paused` are the
        // trigger; this asserts the `hidden` arm directly.)
        lifecycle.emit(AppLifecycleState.hidden);
        await tester.pumpAndSettle();

        // Words gone; the deliberate Reveal gate is back.
        expect(find.text('art'), findsNothing);
        expect(find.text(l10n.walletBackupReveal), findsOneWidget);

        // Returning to the foreground does NOT auto-reveal (re-tap required).
        lifecycle.emit(AppLifecycleState.resumed);
        await tester.pumpAndSettle();
        expect(find.text('art'), findsNothing);

        // Re-reveal works AND re-fetches (count 2) — proof the cached copy was
        // dropped on hide, never parked across the background.
        await tester.tap(find.text(l10n.walletBackupReveal));
        await tester.pumpAndSettle();
        expect(find.text('art'), findsOneWidget);
        expect(provisioner.revealCount, 2);

        // `paused` (full Android background) is also the trigger — re-hide.
        lifecycle.emit(AppLifecycleState.paused);
        await tester.pumpAndSettle();
        expect(find.text('art'), findsNothing);
      },
    );
  });

  group('failure surfaces', () {
    testWidgets('reveal failure shows an honest message + a working retry', (
      tester,
    ) async {
      final provisioner = FakeWalletProvisioner(exists: false)
        ..failReveal = const WalletApiError(
          code: 'RW-X',
          message: 'device locked',
          kind: WalletErrorKind.keystoreUnavailable(),
        );
      await tester.pumpWidget(
        harness(provisioner: provisioner, store: FakeOnboardingStore()),
      );
      await tester.pumpAndSettle();
      final l10n = l10nOf(tester);
      await tester.tap(find.text(l10n.walletCreateButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletBackupReveal));
      await tester.pumpAndSettle();

      // Honest, plain-language, no code; words are not shown.
      expect(find.text(l10n.walletBackupRevealFailed), findsOneWidget);
      expect(find.text('art'), findsNothing);

      // Recover: clear the fault, retry → words appear.
      provisioner.failReveal = null;
      await tester.tap(find.text(l10n.walletBackupRetryReveal));
      await tester.pumpAndSettle();
      expect(find.text('art'), findsOneWidget);
    });

    testWidgets('a retryable boot failure shows Try again', (tester) async {
      final provisioner = FakeWalletProvisioner(exists: false)
        ..failWalletExists = const WalletApiError(
          code: 'RW-LOCK',
          message: 'locked',
          kind: WalletErrorKind.keystoreUnavailable(),
        );
      await tester.pumpWidget(
        harness(provisioner: provisioner, store: FakeOnboardingStore()),
      );
      await tester.pumpAndSettle();
      final l10n = l10nOf(tester);

      expect(
        find.text(l10n.walletOnboardingFailedDeviceLocked),
        findsOneWidget,
      );
      expect(find.text(l10n.walletOnboardingRetry), findsOneWidget);

      // Retry after the device unlocks → the probe re-runs → Welcome.
      provisioner.failWalletExists = null;
      await tester.tap(find.text(l10n.walletOnboardingRetry));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletOnboardingWelcomeTitle), findsOneWidget);
    });

    testWidgets('a needs-recovery failure offers NO retry (restore is the path)', (
      tester,
    ) async {
      // A wallet on disk that can't be opened (corrupt store / destroyed key):
      // retry can't help, so no Retry button — the honest next step is restore.
      final provisioner = FakeWalletProvisioner(exists: true)
        ..failOpen = const WalletApiError(
          code: 'RW-STORE',
          message: 'corrupt',
          kind: WalletErrorKind.storeCorrupt(),
        );
      await tester.pumpWidget(
        harness(provisioner: provisioner, store: FakeOnboardingStore()),
      );
      await tester.pumpAndSettle();
      final l10n = l10nOf(tester);

      expect(
        find.text(l10n.walletOnboardingFailedNeedsRecovery),
        findsOneWidget,
      );
      expect(find.text(l10n.walletOnboardingRetry), findsNothing);
    });

    testWidgets('an interrupted-create failure shows the reassuring retry copy', (
      tester,
    ) async {
      // A provisioningIncomplete on open (a TOCTOU remnant) → interruptedSetup:
      // a retryable, reassuring "didn't finish — try again, nothing was lost"
      // (NOT the alarming needs-recovery copy, NOT the generic unknown one).
      final provisioner = FakeWalletProvisioner(exists: true)
        ..failOpen = const WalletApiError(
          code: 'RW-PROV',
          message: 'interrupted',
          kind: WalletErrorKind.provisioningIncomplete(),
        );
      await tester.pumpWidget(
        harness(provisioner: provisioner, store: FakeOnboardingStore()),
      );
      await tester.pumpAndSettle();
      final l10n = l10nOf(tester);

      expect(
        find.text(l10n.walletOnboardingFailedInterruptedSetup),
        findsOneWidget,
      );
      expect(find.text(l10n.walletOnboardingRetry), findsOneWidget);
    });

    testWidgets('a confirm-persist failure keeps the gate CLOSED and says so', (
      tester,
    ) async {
      final provisioner = FakeWalletProvisioner(exists: false);
      final store = FakeOnboardingStore();
      await pumpToRevealedBackup(
        tester,
        provisioner: provisioner,
        store: store,
      );
      final l10n = l10nOf(tester);

      // The create-time false-write already succeeded; make the TRUE write fail.
      store.failSet = const WalletApiError(
        code: 'RW-DISK',
        message: 'disk full',
        kind: WalletErrorKind.diskFull(),
      );
      await tapVisible(tester, find.text(l10n.walletBackupConfirmCheckbox));
      await tester.pump();
      await tapVisible(tester, find.text(l10n.walletBackupContinue));
      // The confirm runs (sync→Confirming, await throws → revert + rethrow →
      // SnackBar). Avoid pumpAndSettle: the SnackBar auto-dismiss timer would be
      // pumped through. Step the frames explicitly.
      await tester.pump(); // confirm microtasks + revert
      await tester.pump(const Duration(milliseconds: 750)); // SnackBar in

      expect(find.text(l10n.walletBackupSaveFailed), findsOneWidget);
      // The gate stayed CLOSED: never deposit-ready, never persisted true.
      expect(find.text(l10n.walletBalanceLabel), findsNothing);
      expect(store.persisted, isFalse);
      // Still on the backup screen (the words survive the revert — same view).
      expect(find.text(l10n.walletBackupTitle), findsOneWidget);
    });
  });

  // -------------------------------------------------------------------------
  // RESTORE screen (the inbound recovery-phrase entry)
  // -------------------------------------------------------------------------
  group('restore screen', () {
    // Drive welcome → tap Restore → land on the words-entry screen.
    Future<WalletLocalizations> pumpToRestore(
      WidgetTester tester, {
      required FakeWalletProvisioner provisioner,
      required FakeOnboardingStore store,
      FakeScreenSecurity? security,
    }) async {
      await tester.pumpWidget(
        harness(provisioner: provisioner, store: store, security: security),
      );
      await tester.pumpAndSettle(); // boot → Welcome
      final l10n = l10nOf(tester);
      await tester.tap(find.text(l10n.walletRestoreButton));
      await tester.pumpAndSettle(); // → RestoreInput
      return l10n;
    }

    // The pill field commits a word on whitespace and ACCUMULATES pills, so a
    // helper ADDS words (each call appends), with a trailing space to commit the
    // last one. `enterText` replaces the in-progress token, never the pills.
    Future<void> addWords(WidgetTester tester, String words) async {
      await tester.enterText(find.byType(TextField), '$words ');
      await tester.pump(); // onChanged commits the pills + refreshes the gate
    }

    testWidgets('welcome → Restore opens the words-entry screen', (
      tester,
    ) async {
      final l10n = await pumpToRestore(
        tester,
        provisioner: FakeWalletProvisioner(exists: false),
        store: FakeOnboardingStore(),
      );
      expect(find.text(l10n.walletRestoreTitle), findsOneWidget);
      // The pill field's inline editor is the one TextField on the screen.
      expect(find.byType(TextField), findsOneWidget);
    });

    testWidgets(
      'Submit enables only at a valid phrase length (pills accumulate)',
      (tester) async {
        final l10n = await pumpToRestore(
          tester,
          provisioner: FakeWalletProvisioner(exists: false),
          store: FakeOnboardingStore(),
        );

        FilledButton submitButton() => tester.widget<FilledButton>(
          find.widgetWithText(FilledButton, l10n.walletRestoreSubmit),
        );
        expect(submitButton().onPressed, isNull, reason: 'empty → disabled');

        // Boundary coverage for kValidMnemonicLengths {12,15,18,21,24}: walk the
        // count up by ADDING pills so a future edit can't silently narrow the set.
        await addWords(
          tester,
          List.filled(11, 'abandon').join(' '),
        ); // 11 pills
        expect(submitButton().onPressed, isNull, reason: '11 words → disabled');
        await addWords(tester, 'abandon'); // → 12
        expect(
          submitButton().onPressed,
          isNotNull,
          reason: '12 words → enabled',
        );
        await addWords(tester, 'abandon'); // → 13
        expect(submitButton().onPressed, isNull, reason: '13 words → disabled');
      },
    );

    testWidgets(
      'a word that is NOT a BIP39 word disables Submit and flags live',
      (tester) async {
        final l10n = await pumpToRestore(
          tester,
          provisioner: FakeWalletProvisioner(exists: false),
          store: FakeOnboardingStore(),
        );
        FilledButton submitButton() => tester.widget<FilledButton>(
          find.widgetWithText(FilledButton, l10n.walletRestoreSubmit),
        );
        // 12 words, but one ('zzzz') is not in the fixture wordlist → known-bad.
        await addWords(tester, '${List.filled(11, 'abandon').join(' ')} zzzz');
        expect(
          submitButton().onPressed,
          isNull,
          reason: 'a known-bad word blocks submit BEFORE the SDK',
        );
        // The live cue names the count of bad words.
        expect(
          find.text(l10n.walletRestoreSomeWordsInvalid(1)),
          findsOneWidget,
        );
      },
    );

    testWidgets(
      'a valid restore reaches the deposit-ready wallet; the SDK receives '
      'LOWERCASED words (the hard host-UI contract, end-to-end)',
      (tester) async {
        final provisioner = FakeWalletProvisioner(exists: false);
        final store = FakeOnboardingStore();
        final l10n = await pumpToRestore(
          tester,
          provisioner: provisioner,
          store: store,
        );

        // 24 valid words entered mixed-case → pills normalize to lowercase.
        await addWords(tester, '${List.filled(23, 'Abandon').join(' ')} ART');
        await tapVisible(tester, find.text(l10n.walletRestoreSubmit));
        await tester.pumpAndSettle();

        expect(provisioner.restoreCount, 1);
        // Deposit-ready: restore goes straight to Active (the user holds the
        // phrase they typed) — the live wallet surface is shown.
        expect(find.text(l10n.walletBalanceLabel), findsOneWidget);
        expect(find.text(l10n.walletRestoreTitle), findsNothing);
        expect(store.persisted, isTrue);
        // The contract: every word reached the SDK lowercased + trimmed.
        expect(
          provisioner.lastRestoreWords,
          everyElement(equals(predicate<String>((w) => w == w.toLowerCase()))),
        );
        expect(provisioner.lastRestoreWords!.first, 'abandon');
        expect(provisioner.lastRestoreWords!.last, 'art');
      },
    );

    testWidgets(
      'a mistyped word returns an inline error AND preserves the entered pills '
      '(no full-screen failure, no retyping 24 words)',
      (tester) async {
        final handle = tester.ensureSemantics();
        final provisioner = FakeWalletProvisioner(exists: false)
          ..failRestore = const WalletApiError(
            code: 'RW-SEED-002',
            message: 'unknown word',
            kind: WalletErrorKind.invalidMnemonic(wordIndex: 6),
          );
        final l10n = await pumpToRestore(
          tester,
          provisioner: provisioner,
          store: FakeOnboardingStore(),
        );

        // All words are valid BIP39 words (so live validity passes) — the FAKE
        // forces the SDK fault, exercising the checksum-class error path.
        await addWords(tester, '${List.filled(23, 'abandon').join(' ')} art');
        await tapVisible(tester, find.text(l10n.walletRestoreSubmit));
        await tester.pumpAndSettle();

        // Still on the restore screen with an inline, actionable error...
        expect(find.text(l10n.walletRestoreTitle), findsOneWidget);
        expect(
          find.text(l10n.walletRestoreFaultInvalidWord(7)),
          findsOneWidget,
        );
        // #397 P3 (review F2): the appearing SDK fault ARMS the status line's
        // live region (the watch-only twin's convention) — while the plain
        // word-count text must stay un-armed (no per-keystroke announcements).
        expect(
          tester
              .getSemantics(find.text(l10n.walletRestoreFaultInvalidWord(7)))
              .flagsCollection
              .isLiveRegion,
          isTrue,
        );
        // ...and the entered pills SURVIVED (the same view stayed mounted) —
        // 'art' is still a pill, so the user need not retype.
        expect(find.text('art'), findsOneWidget);
        expect(find.text('abandon'), findsWidgets);
        // Never deposit-ready on a failed restore.
        expect(find.text(l10n.walletBalanceLabel), findsNothing);
        handle.dispose();
      },
    );

    testWidgets('Back returns to the welcome screen', (tester) async {
      final l10n = await pumpToRestore(
        tester,
        provisioner: FakeWalletProvisioner(exists: false),
        store: FakeOnboardingStore(),
      );

      await tapVisible(tester, find.text(l10n.walletRestoreBack));
      await tester.pumpAndSettle();

      expect(find.text(l10n.walletOnboardingWelcomeTitle), findsOneWidget);
      expect(find.text(l10n.walletRestoreTitle), findsNothing);
    });

    testWidgets('the restore screen requests screenshot protection on show', (
      tester,
    ) async {
      final security = FakeScreenSecurity();
      await pumpToRestore(
        tester,
        provisioner: FakeWalletProvisioner(exists: false),
        store: FakeOnboardingStore(),
        security: security,
      );
      // A phrase can be on screen → protection engaged for its lifetime (the
      // same defence-in-depth the backup screen uses).
      expect(security.enableCount, greaterThanOrEqualTo(1));
    });
  });

  group('the #251 escape — a non-retryable failure is never a dead-end', () {
    testWidgets('needsRecovery shows Restore; confirm → force-clear → restore form', (
      tester,
    ) async {
      // A wallet on disk whose open() fails with a damaged-data kind lands on the
      // non-retryable needsRecovery screen; its plain wipe then reports the key is
      // gone, so the escape force-clears it (the deliberate, user-confirmed path).
      final p = FakeWalletProvisioner(exists: true)
        ..failOpen = const WalletApiError(
          code: 'RW',
          message: 'damaged',
          kind: WalletErrorKind.keystoreInconsistent(
            permanentlyInvalidated: false,
          ),
        )
        ..failDelete = const WalletApiError(
          code: 'RW',
          message: 'key gone',
          kind: WalletErrorKind.keystoreInconsistent(
            permanentlyInvalidated: false,
          ),
        );
      await tester.pumpWidget(
        harness(provisioner: p, store: FakeOnboardingStore(confirmed: true)),
      );
      await tester
          .pumpAndSettle(); // boot probe → OnboardingFailed(needsRecovery)
      final l10n = l10nOf(tester);

      // THE GUARANTEE: an escape action is shown — never a button-less dead-end.
      expect(
        find.text(l10n.walletOnboardingFailedRestoreAction),
        findsOneWidget,
        reason: 'a non-retryable failure MUST offer a way out',
      );

      // Tap → the funds-are-safe confirm dialog → confirm.
      await tester.tap(find.text(l10n.walletOnboardingFailedRestoreAction));
      await tester.pumpAndSettle();
      expect(
        find.text(l10n.walletOnboardingRecoverConfirmTitle),
        findsOneWidget,
      );
      await tester.tap(
        find.descendant(
          of: find.byType(AlertDialog),
          matching: find.text(l10n.walletOnboardingFailedRestoreAction),
        ),
      );
      await tester
          .pumpAndSettle(); // recoverByRestore → force-clear → RestoreInput

      expect(
        p.forceDeleteCount,
        1,
        reason: 'the unreadable remnant was deliberately force-cleared',
      );
      expect(
        find.text(l10n.walletRestoreTitle),
        findsOneWidget,
        reason:
            'escaped to the restore form — the user recovers funds from the phrase',
      );
      expect(find.text(l10n.walletOnboardingFailedTitle), findsNothing);
    });
  });

  group('forced-backup "Start over" (#356-F2 — the escape is never a false '
      'confirmation)', () {
    // Drive welcome → create → AwaitingBackup (no reveal needed — the escape
    // shows pre-reveal too: the mis-tapped-Create user never opens the words).
    Future<void> pumpToAwaitingBackup(
      WidgetTester tester, {
      required FakeWalletProvisioner provisioner,
      required FakeOnboardingStore store,
    }) async {
      await tester.pumpWidget(harness(provisioner: provisioner, store: store));
      await tester.pumpAndSettle(); // boot probe → Welcome
      await tester.tap(find.text(l10nOf(tester).walletCreateButton));
      await tester.pumpAndSettle(); // create → AwaitingBackup
    }

    testWidgets(
      'Start over → confirm → the wallet is deleted and Welcome returns '
      '(no backup confirmation was ever persisted)',
      (tester) async {
        final p = FakeWalletProvisioner();
        final store = FakeOnboardingStore();
        await pumpToAwaitingBackup(tester, provisioner: p, store: store);
        final l10n = l10nOf(tester);

        await tapVisible(tester, find.text(l10n.walletBackupStartOver));
        await tester.pumpAndSettle();
        // The deliberate confirm gate — destructive, so a dialog first.
        expect(
          find.text(l10n.walletBackupStartOverConfirmTitle),
          findsOneWidget,
        );
        await tester.tap(find.text(l10n.walletBackupStartOverConfirm));
        await tester.pumpAndSettle(); // shred → Welcome

        expect(p.deleteCount, 1);
        expect(p.exists, isFalse, reason: 'the wallet is gone from disk');
        expect(find.text(l10n.walletOnboardingWelcomeTitle), findsOneWidget);
        expect(
          await store.isBackupConfirmed(),
          isFalse,
          reason: 'the gate never opened — no false confirmation was written',
        );
      },
    );

    testWidgets('S11 C3 (the review\'s M4): at 2.0x on a 320 dp phone the '
        'start-over fund-loss warning scrolls — the whole warning and BOTH '
        'actions are reachable (the S175 clip)', (tester) async {
      final p = FakeWalletProvisioner();
      final store = FakeOnboardingStore();
      await pumpToAwaitingBackup(tester, provisioner: p, store: store);
      final l10n = l10nOf(tester);

      // The phone and the text size the clip was found at.
      tester.view.physicalSize = const Size(320, 568);
      tester.view.devicePixelRatio = 1.0;
      tester.platformDispatcher.textScaleFactorTestValue = 2.0;
      addTearDown(tester.view.reset);
      addTearDown(tester.platformDispatcher.clearTextScaleFactorTestValue);
      await tester.pumpAndSettle();

      await tapVisible(tester, find.text(l10n.walletBackupStartOver));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expectDialogScrollsToBothActions(
        tester,
        bodyStart: l10n.walletBackupStartOverConfirmBody,
        cancel: l10n.walletBackupStartOverKeep,
        confirm: l10n.walletBackupStartOverConfirm,
      );
      await tester.drag(
        find
            .descendant(
              of: find.byType(AlertDialog),
              matching: find.byType(Scrollable),
            )
            .first,
        const Offset(0, -20000),
      );
      await tester.pumpAndSettle();
      expectDialogScrollsToBothActions(
        tester,
        bodyStart: l10n.walletBackupStartOverConfirmBody,
        cancel: l10n.walletBackupStartOverKeep,
        confirm: l10n.walletBackupStartOverConfirm,
        scrolledToEnd: true,
      );

      // The safe action takes the tap from there, and nothing is deleted.
      await tester.tap(find.text(l10n.walletBackupStartOverKeep));
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsNothing);
      expect(p.deleteCount, 0);
    });

    testWidgets('"Keep this wallet" declines — nothing is deleted', (
      tester,
    ) async {
      final p = FakeWalletProvisioner();
      await pumpToAwaitingBackup(
        tester,
        provisioner: p,
        store: FakeOnboardingStore(),
      );
      final l10n = l10nOf(tester);

      await tapVisible(tester, find.text(l10n.walletBackupStartOver));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletBackupStartOverKeep));
      await tester.pumpAndSettle();

      expect(p.deleteCount, 0);
      expect(p.exists, isTrue);
      // Still on the forced-backup screen — the safe default kept the wallet.
      expect(find.text(l10n.walletBackupTitle), findsOneWidget);
    });

    testWidgets(
      'a FAILED delete recovers the wallet and says so — never a false '
      '"deleted" (the wipe-fault recover-by-reopen path)',
      (tester) async {
        final p = FakeWalletProvisioner()..failDelete = Exception('wedged');
        await pumpToAwaitingBackup(
          tester,
          provisioner: p,
          store: FakeOnboardingStore(),
        );
        final l10n = l10nOf(tester);

        await tapVisible(tester, find.text(l10n.walletBackupStartOver));
        await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.walletBackupStartOverConfirm));
        await tester.pumpAndSettle(); // fault → re-probe → AwaitingBackup

        expect(p.exists, isTrue, reason: 'keychain-first: nothing deleted');
        // Recovered back onto THIS screen, with the honest couldn't-delete cue
        // (the Security screen's twin outcome copy).
        expect(find.text(l10n.walletBackupTitle), findsOneWidget);
        expect(find.text(l10n.securityDeleteFailedSnack), findsOneWidget);
        expect(find.text(l10n.walletOnboardingWelcomeTitle), findsNothing);
      },
    );

    testWidgets(
      'a resumed wallet WITH PRIOR ACTIVITY (lost confirmed flag over an '
      'intact DB) gets NO Start-over — the escape must not offer a delete on '
      'a possibly-funded wallet this screen renders balance-less (S174 HIGH)',
      (tester) async {
        // The wallet exists, has synced before, but the confirmed flag reads
        // false (the silently-reset prefs file) — the boot probe re-forces
        // backup on what may be a FUNDED wallet.
        final p = FakeWalletProvisioner(
          exists: true,
          session: FakeWalletSession(
            snapshotValue: walletStateFixture(
              lastSynced: const SyncStamp(height: 500, at: 1751700000),
            ),
          ),
        );
        await tester.pumpWidget(
          harness(provisioner: p, store: FakeOnboardingStore(confirmed: false)),
        );
        await tester.pumpAndSettle(); // probe → AwaitingBackup(prior-activity)
        final l10n = l10nOf(tester);

        // The forced-backup screen renders (the pre-F2 posture)…
        expect(find.text(l10n.walletBackupTitle), findsOneWidget);
        // …but the destructive escape is suppressed.
        expect(find.text(l10n.walletBackupStartOver), findsNothing);
      },
    );

    testWidgets(
      'Start over is LOCKED while the confirm persist is in flight (the '
      '`_saving` window — a shred must never race the gate write)',
      (tester) async {
        final p = FakeWalletProvisioner();
        final store = FakeOnboardingStore()..holdSet = Completer<void>();
        await tester.pumpWidget(harness(provisioner: p, store: store));
        await tester.pumpAndSettle();
        final l10n = l10nOf(tester);
        await tester.tap(find.text(l10n.walletCreateButton));
        // startCreate's gate-close write parks on holdSet — release it so the
        // create completes, then re-arm the gate for the CONFIRM write.
        store.holdSet!.complete();
        await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.walletBackupReveal));
        await tester.pumpAndSettle();

        store.holdSet = Completer<void>(); // the confirm persist will park
        await tapVisible(tester, find.text(l10n.walletBackupConfirmCheckbox));
        await tester.pumpAndSettle();
        await tapVisible(tester, find.text(l10n.walletBackupContinue));
        await tester.pump(); // mid-persist: _saving true

        final startOver = tester.widget<TextButton>(
          find.ancestor(
            of: find.text(l10n.walletBackupStartOver),
            matching: find.byType(TextButton),
          ),
        );
        expect(
          startOver.onPressed,
          isNull,
          reason: 'Start over must be disabled while the confirm persists',
        );

        store.holdSet!.complete();
        await tester.pumpAndSettle(); // confirm lands → Active
        expect(find.text(l10n.walletBackupTitle), findsNothing);
      },
    );

    testWidgets(
      'the reveal-error RETRY is locked while a Start-over shred is in '
      'flight (no seed re-unseal racing the keychain sever — S174 MED)',
      (tester) async {
        final p = FakeWalletProvisioner()
          ..failReveal = Exception('keychain busy')
          ..holdDelete = Completer<void>();
        await tester.pumpWidget(
          harness(provisioner: p, store: FakeOnboardingStore()),
        );
        await tester.pumpAndSettle();
        final l10n = l10nOf(tester);
        await tester.tap(find.text(l10n.walletCreateButton));
        await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.walletBackupReveal));
        await tester.pumpAndSettle(); // reveal fails → the error arm + retry

        expect(find.text(l10n.walletBackupRetryReveal), findsOneWidget);

        // Start the shred (parks on holdDelete)…
        await tapVisible(tester, find.text(l10n.walletBackupStartOver));
        await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.walletBackupStartOverConfirm));
        await tester.pump(); // _deleting true, delete in flight

        // …and the reveal retry must be dead while it runs.
        final retry = tester.widget<OutlinedButton>(
          find.ancestor(
            of: find.text(l10n.walletBackupRetryReveal),
            matching: find.byType(OutlinedButton),
          ),
        );
        expect(retry.onPressed, isNull);
        final revealsBefore = p.revealCount;

        p.holdDelete!.complete();
        await tester.pumpAndSettle(); // shred lands → Welcome
        expect(find.text(l10n.walletOnboardingWelcomeTitle), findsOneWidget);
        expect(
          p.revealCount,
          revealsBefore,
          reason: 'no reveal ran during the shred',
        );
      },
    );
  });

  // #397 §3.7 D5 — the watch-only import SCREEN over the real controller.
  group('watch-only import screen', () {
    testWidgets('a viewing key is protected like a phrase: screenshot '
        'protection on show, and the field stays off keyboard and autofill '
        'paths', (tester) async {
      final security = FakeScreenSecurity();
      await tester.pumpWidget(
        harness(
          provisioner: FakeWalletProvisioner(exists: false),
          store: FakeOnboardingStore(),
          security: security,
        ),
      );
      await tester.pumpAndSettle();
      final before = security.enableCount;
      await tester.tap(find.byKey(const ValueKey('wallet-welcome-watch-only')));
      await tester.pumpAndSettle();
      // A viewing key reveals the whole history (the S7 retro security
      // review): the import screen takes the restore screen's protection.
      expect(security.enableCount, greaterThan(before));
      final field = tester.widget<TextField>(
        find.byKey(const ValueKey('watch-only-key-field')),
      );
      expect(field.autocorrect, isFalse);
      expect(field.enableSuggestions, isFalse);
      expect(field.enableIMEPersonalizedLearning, isFalse);
      expect(field.autofillHints, isNull);
      expect(field.smartDashesType, SmartDashesType.disabled);
      expect(field.smartQuotesType, SmartQuotesType.disabled);

      // …and releases it on unmount — a leaked count would hold the block on
      // for the whole app.
      final disabledBefore = security.disableCount;
      await tester.pumpWidget(const MaterialApp(home: SizedBox.shrink()));
      await tester.pumpAndSettle();
      expect(security.disableCount, greaterThan(disabledBefore));
    });

    testWidgets(
      'Welcome → "Watch a wallet" → paste a viewing key + submit → the wallet '
      'surface appears watch-only (chrome hides Send)',
      (tester) async {
        final p = FakeWalletProvisioner(exists: false);
        await tester.pumpWidget(
          harness(provisioner: p, store: FakeOnboardingStore()),
        );
        await tester.pumpAndSettle();
        final l10n = l10nOf(tester);

        // Welcome shows the watch-only entry; tap it → the input screen.
        expect(
          find.byKey(const ValueKey('wallet-welcome-watch-only')),
          findsOneWidget,
        );
        await tester.tap(
          find.byKey(const ValueKey('wallet-welcome-watch-only')),
        );
        await tester.pumpAndSettle();
        expect(find.text(l10n.walletWatchOnlyTitle), findsOneWidget);

        // #397 P2 (UX-H1): the date card carries the honest "funds received
        // before then won't appear" warning (a watch-only import always scans
        // from a floor — no full-scan arm — so an older wallet must not silently
        // under-report). The info-icon + orange treatment mirrors the restore
        // screen; the copy landing here is the money-honesty proof.
        expect(find.textContaining('Older wallet?'), findsOneWidget);
        expect(find.textContaining('before then'), findsOneWidget);

        // Submit is disabled on an empty field.
        final submitBefore = tester.widget<FilledButton>(
          find.byKey(const ValueKey('watch-only-submit')),
        );
        expect(submitBefore.onPressed, isNull);

        // Paste a viewing key → submit enables → import.
        await tester.enterText(
          find.byKey(const ValueKey('watch-only-key-field')),
          'uview1testkeyxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx',
        );
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const ValueKey('watch-only-submit')));
        await tester.pumpAndSettle();

        // The wallet surface is live and reports watch-only — Send is hidden.
        expect(p.createWatchOnlyCount, 1);
        expect(
          find.byKey(const ValueKey('wallet-watch-only-badge')),
          findsOneWidget,
        );
        expect(find.byKey(const ValueKey('wallet-action-send')), findsNothing);
        expect(
          find.byKey(const ValueKey('wallet-action-receive')),
          findsOneWidget,
        );
      },
    );

    testWidgets(
      'a wrong-network key returns to the input form with an honest fault, '
      'the pasted key intact',
      (tester) async {
        final p = FakeWalletProvisioner(exists: false)
          ..failCreateWatchOnly = const WalletApiError(
            code: 'RW-CFG-004',
            message: 'network mismatch',
            kind: WalletErrorKind.networkMismatch(),
          );
        await tester.pumpWidget(
          harness(provisioner: p, store: FakeOnboardingStore()),
        );
        await tester.pumpAndSettle();
        final l10n = l10nOf(tester);

        await tester.tap(
          find.byKey(const ValueKey('wallet-welcome-watch-only')),
        );
        await tester.pumpAndSettle();
        const key = 'uview1wrongnetworkkeyxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx';
        await tester.enterText(
          find.byKey(const ValueKey('watch-only-key-field')),
          key,
        );
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const ValueKey('watch-only-submit')));
        await tester.pumpAndSettle();

        // Back on the input form with the honest fault + the key still there.
        expect(find.text(l10n.walletWatchOnlyTitle), findsOneWidget);
        expect(find.byKey(const ValueKey('watch-only-fault')), findsOneWidget);
        expect(
          find.text(l10n.walletWatchOnlyFaultNetworkMismatch),
          findsOneWidget,
        );
        expect(
          find.text(key),
          findsOneWidget,
          reason: 'the pasted key survives',
        );
      },
    );

    testWidgets(
      '#397 P3 UX-M5: the input fault is a LIVE REGION — a screen reader '
      'hears it when it appears, without re-traversing the form',
      (tester) async {
        final handle = tester.ensureSemantics();
        final p = FakeWalletProvisioner(exists: false)
          ..failCreateWatchOnly = const WalletApiError(
            code: 'RW-VIEW-002',
            message: 'invalid viewing key',
            kind: WalletErrorKind.invalidViewingKey(),
          );
        await tester.pumpWidget(
          harness(provisioner: p, store: FakeOnboardingStore()),
        );
        await tester.pumpAndSettle();

        await tester.tap(
          find.byKey(const ValueKey('wallet-welcome-watch-only')),
        );
        await tester.pumpAndSettle();
        await tester.enterText(
          find.byKey(const ValueKey('watch-only-key-field')),
          'uview1badkeyxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx',
        );
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const ValueKey('watch-only-submit')));
        await tester.pumpAndSettle();

        final fault = tester.getSemantics(
          find.byKey(const ValueKey('watch-only-fault')),
        );
        expect(fault.flagsCollection.isLiveRegion, isTrue);
        handle.dispose();
      },
    );

    testWidgets(
      '#397 P4 — NO scan affordance on a camera-less platform (the test host '
      'is desktop): paste stays the only path, no dead button',
      (tester) async {
        await tester.pumpWidget(
          harness(
            provisioner: FakeWalletProvisioner(exists: false),
            store: FakeOnboardingStore(),
          ),
        );
        await tester.pumpAndSettle();
        await tester.tap(
          find.byKey(const ValueKey('wallet-welcome-watch-only')),
        );
        await tester.pumpAndSettle();

        expect(find.byKey(const ValueKey('watch-only-key-field')), findsOne);
        expect(find.byKey(const ValueKey('watch-only-scan')), findsNothing);
      },
    );

    testWidgets(
      '#397 P4 — the scan fills the field with the RAW payload; the ALL-CAPS '
      'case-fold happens at the startWatchOnly chokepoint on submit (S231 F1 — '
      'so a paste of the same uppercase key is treated identically), and the '
      'scanned key imports end-to-end',
      (tester) async {
        const key = 'uview1scannedkeyxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx';
        final p = FakeWalletProvisioner(exists: false);
        await tester.pumpWidget(
          harness(
            provisioner: p,
            store: FakeOnboardingStore(),
            extraOverrides: [
              addressScannerSupportedProvider.overrideWithValue(true),
              // A QR reader commonly returns bech32 UPPERCASE (alphanumeric
              // mode). The view fills the field verbatim; the controller folds.
              viewingKeyScannerProvider.overrideWithValue(
                (context) async => key.toUpperCase(),
              ),
            ],
          ),
        );
        await tester.pumpAndSettle();
        await tester.tap(
          find.byKey(const ValueKey('wallet-welcome-watch-only')),
        );
        await tester.pumpAndSettle();

        await tester.tap(find.byKey(const ValueKey('watch-only-scan')));
        await tester.pumpAndSettle();

        // The field shows the RAW (uppercase) scan — the chokepoint, not the
        // view, owns normalization now.
        expect(find.text(key.toUpperCase()), findsOneWidget);
        await tester.tap(find.byKey(const ValueKey('watch-only-submit')));
        await tester.pumpAndSettle();

        // …but the SDK receives the case-folded key, and it imports.
        expect(p.createWatchOnlyCount, 1);
        expect(p.lastWatchOnlyUfvk, key);
        expect(
          find.byKey(const ValueKey('wallet-watch-only-badge')),
          findsOneWidget,
        );
      },
    );

    testWidgets(
      '#397 P4 — a cancelled scan leaves the field (and any pasted text) '
      'intact',
      (tester) async {
        await tester.pumpWidget(
          harness(
            provisioner: FakeWalletProvisioner(exists: false),
            store: FakeOnboardingStore(),
            extraOverrides: [
              addressScannerSupportedProvider.overrideWithValue(true),
              viewingKeyScannerProvider.overrideWithValue(
                (context) async => null, // user cancelled / manual entry
              ),
            ],
          ),
        );
        await tester.pumpAndSettle();
        await tester.tap(
          find.byKey(const ValueKey('wallet-welcome-watch-only')),
        );
        await tester.pumpAndSettle();

        const pasted = 'uview1alreadypastedxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx';
        await tester.enterText(
          find.byKey(const ValueKey('watch-only-key-field')),
          pasted,
        );
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const ValueKey('watch-only-scan')));
        await tester.pumpAndSettle();

        expect(find.text(pasted), findsOneWidget); // untouched
      },
    );
  });
}
