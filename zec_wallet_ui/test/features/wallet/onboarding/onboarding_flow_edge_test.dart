import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/lifecycle/app_lifecycle_provider.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_screen.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// REAL-WORLD-EDGE widget tests for the ZEC wallet onboarding SCREENS
/// (spec §3.2g iii-B-2-a), the gaps the 18 happy/typical tests in
/// `onboarding_flow_test.dart` do NOT cover. The lens is MONEY RELIABILITY:
/// every edge asserts the deposit-gate consequence, not just the mechanics.
///
/// THE INVARIANT under test, at the UI: a deposit-capable surface (balance +
/// sync controls) appears ONLY after backup is confirmed-AND-persisted
/// (`store.persisted == true`). Where relevant every edge re-asserts the gate
/// stayed CLOSED until that holds.
///
/// Driven through the real `OnboardingController` against host-VM fakes (no
/// native lib, no device); the helpers mirror `onboarding_flow_test.dart`.
void main() {
  // Resolve l10n from a live element so finders couple to KEYS, not English.
  WalletLocalizations l10nOf(WidgetTester tester) =>
      WalletLocalizations.of(tester.element(find.byType(WalletScreen)));

  Widget harness({
    required FakeWalletProvisioner provisioner,
    required FakeOnboardingStore store,
    FakeScreenSecurity? security,
  }) {
    return ProviderScope(
      overrides: [
        walletProvisionerProvider.overrideWithValue(provisioner),
        onboardingStoreProvider.overrideWithValue(store),
        if (security != null)
          screenSecurityProvider.overrideWithValue(security),
      ],
      child: MaterialApp(
        localizationsDelegates: WalletLocalizations.localizationsDelegates,
        supportedLocales: WalletLocalizations.supportedLocales,
        theme: lightTheme,
        home: const WalletScreen(),
      ),
    );
  }

  // welcome → create → AwaitingBackup → reveal. Leaves words on screen,
  // unconfirmed. Same prefix the happy-path file uses.
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

  // The checkbox + Continue sit below the 800×600 fold — scroll in before tap.
  Future<void> tapVisible(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
  }

  // Mount over a CONTROLLABLE lifecycle (the provider is overridden, bypassing
  // the platform binding) so lifecycle tests are deterministic. Leaves the app
  // at the boot result; returns the lifecycle notifier (`emit` to fire the
  // view's `ref.listen`).
  Future<TestLifecycleNotifier> mountL(
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
    return container.read(appLifecycleProvider.notifier)
        as TestLifecycleNotifier;
  }

  // True iff the deposit-capable surface (the balance card) is shown. The
  // single money-safety predicate the edges assert on. (Sync runs on its own —
  // there is no manual control to anchor on; the balance card is the gate.)
  bool depositReady(WidgetTester tester, WalletLocalizations l10n) =>
      find.text(l10n.walletBalanceLabel).evaluate().isNotEmpty;

  // ---------------------------------------------------------------------------
  // 1. WORD-COUNT AGNOSTIC GRID (defends a future restore wiring + a11y)
  // ---------------------------------------------------------------------------
  group('word grid is count-agnostic', () {
    testWidgets('a restored-style 12-word phrase renders all 12, in order', (
      tester,
    ) async {
      // A 12-word phrase (the BIP39 128-bit length a restore flow would feed
      // back). The grid must NOT assume 24 — each word is numbered 1..12.
      const twelve = <String>[
        'legal',
        'winner',
        'thank',
        'year',
        'wave',
        'sausage',
        'worth',
        'useful',
        'legal',
        'winner',
        'thank',
        'yellow',
      ];
      await pumpToRevealedBackup(
        tester,
        provisioner: FakeWalletProvisioner(
          exists: false,
          recoveryWords: twelve,
        ),
        store: FakeOnboardingStore(),
      );

      // Every word present; the numbered index for each (1..12) is rendered.
      // 'legal'/'winner'/'thank' appear twice → assert by count so a dropped
      // or duplicated cell fails.
      expect(find.text('sausage'), findsOneWidget);
      expect(find.text('yellow'), findsOneWidget);
      expect(find.text('legal'), findsNWidgets(2));
      expect(find.text('winner'), findsNWidgets(2));
      expect(find.text('thank'), findsNWidgets(2));
      for (var i = 1; i <= 12; i++) {
        expect(
          find.text('$i'),
          findsOneWidget,
          reason: 'word index $i must be numbered',
        );
      }
      // No phantom 13th index from a 24-assumption.
      expect(find.text('13'), findsNothing);
    });

    testWidgets('a single very long word renders without overflow', (
      tester,
    ) async {
      // A pathological long token (not a real BIP39 word, but the grid must
      // never assume a max width / overflow-paint). The Flexible+Wrap should
      // lay it out clean — overflow throws are caught by the test binding.
      const long = <String>[
        'supercalifragilisticexpialidociousantidisestablishmentarianism',
      ];
      await pumpToRevealedBackup(
        tester,
        provisioner: FakeWalletProvisioner(exists: false, recoveryWords: long),
        store: FakeOnboardingStore(),
      );

      // Rendered (no RenderFlex overflow exception was thrown during layout —
      // that would fail the test), numbered 1, and no phantom 2.
      expect(find.text(long.first), findsOneWidget);
      expect(find.text('1'), findsOneWidget);
      expect(find.text('2'), findsNothing);
      expect(tester.takeException(), isNull);
    });
  });

  // ---------------------------------------------------------------------------
  // 2. DOUBLE-TAP CONTINUE — persist EXACTLY once, activate once (money)
  // ---------------------------------------------------------------------------
  group('double-tap Continue', () {
    testWidgets('rapid double-tap persists exactly once and activates once', (
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

      // Two taps before settling. The view's synchronous `_saving = true`
      // (first tap) disables the button, and the controller's synchronous
      // OnboardingConfirming transition no-ops a re-entrant confirm — so a
      // second wallet-confirm must be impossible at BOTH layers.
      final continueBtn = find.widgetWithText(
        FilledButton,
        l10n.walletBackupContinue,
      );
      await tester.ensureVisible(continueBtn);
      await tester.pumpAndSettle();
      await tester.tap(continueBtn, warnIfMissed: false);
      await tester.tap(continueBtn, warnIfMissed: false);
      await tester.pumpAndSettle();

      // MONEY-SAFETY: the gate was OPENED (a `confirmed: true` write) exactly
      // ONCE — a double-tap did not double-activate. (setCount is 2 overall:
      // the create-time `false` gate-close + this one `true` confirm; the
      // second tap is a no-op at the controller's synchronous-transition guard,
      // so it adds NO further write.)
      expect(
        store.trueSetCount,
        1,
        reason: 'a double-tap must persist the confirmation exactly once',
      );
      expect(
        store.setCount,
        2,
        reason:
            'one create-time false-write + one confirm true-write — '
            'the second tap added nothing',
      );
      expect(store.persisted, isTrue);
      expect(store.lastSetValue, isTrue);
      expect(depositReady(tester, l10n), isTrue);
      expect(find.text(l10n.walletBackupTitle), findsNothing);
    });
  });

  // ---------------------------------------------------------------------------
  // 3. CHECKBOX LOCKED MID-SAVE — gate decision can't change while persisting
  // ---------------------------------------------------------------------------
  group('checkbox locked mid-save', () {
    testWidgets('while persisting, the confirm checkbox is locked + Continue '
        'shows the spinner; release → deposit-ready', (tester) async {
      final hold = Completer<void>();
      final provisioner = FakeWalletProvisioner(exists: false);
      final store = FakeOnboardingStore();
      await pumpToRevealedBackup(
        tester,
        provisioner: provisioner,
        store: store,
      );
      final l10n = l10nOf(tester);

      // Hold ONLY the confirm true-write (the create-time false-write already
      // ran inside the prefix above) so we can observe the saving window.
      store.holdSet = hold;
      await tapVisible(tester, find.text(l10n.walletBackupConfirmCheckbox));
      await tester.pump();
      await tapVisible(tester, find.text(l10n.walletBackupContinue));
      await tester.pump(); // → _saving = true, persist awaiting the hold

      // MID-SAVE: a spinner is shown (CircularProgressIndicator inside the
      // button) — DO NOT pumpAndSettle here, it would hang on the spinner.
      expect(find.byType(CircularProgressIndicator), findsOneWidget);

      // The checkbox is LOCKED (onChanged null) so the gate decision the user
      // committed to can't be flipped mid-persist.
      final checkbox = tester.widget<CheckboxListTile>(
        find.byType(CheckboxListTile),
      );
      expect(
        checkbox.onChanged,
        isNull,
        reason: 'the confirm box must be locked while the persist is in flight',
      );

      // The gate is still CLOSED — the persist has not completed.
      expect(store.persisted, isFalse);
      expect(depositReady(tester, l10n), isFalse);

      // Release the persist → it completes → deposit-ready exactly once.
      hold.complete();
      await tester.pumpAndSettle();
      expect(store.trueSetCount, 1); // the confirm persisted exactly once
      expect(store.persisted, isTrue);
      expect(depositReady(tester, l10n), isTrue);
    });
  });

  // ---------------------------------------------------------------------------
  // 4. CONFIRM-PERSIST FAILURE then a SUCCESSFUL RETRY (flaky-storage path)
  // ---------------------------------------------------------------------------
  group('confirm-persist failure then retry', () {
    testWidgets('failed persist keeps the gate CLOSED; a retry succeeds and '
        'opens the wallet', (tester) async {
      final provisioner = FakeWalletProvisioner(exists: false);
      final store = FakeOnboardingStore();
      await pumpToRevealedBackup(
        tester,
        provisioner: provisioner,
        store: store,
      );
      final l10n = l10nOf(tester);

      // Make the TRUE write fail (the create-time false-write already landed).
      store.failSet = const WalletApiError(
        code: 'RW-DISK',
        message: 'disk full',
        kind: WalletErrorKind.diskFull(),
      );
      await tapVisible(tester, find.text(l10n.walletBackupConfirmCheckbox));
      await tester.pump();
      await tapVisible(tester, find.text(l10n.walletBackupContinue));
      // confirm: sync→Confirming, await throws → revert + rethrow → SnackBar.
      // Step frames; do NOT pumpAndSettle through the SnackBar auto-dismiss.
      await tester.pump(); // confirm microtasks + revert + _saving=false
      await tester.pump(const Duration(milliseconds: 750)); // SnackBar in

      // Honest cue shown, gate CLOSED: never persisted, never deposit-ready.
      expect(find.text(l10n.walletBackupSaveFailed), findsOneWidget);
      expect(store.persisted, isFalse);
      expect(depositReady(tester, l10n), isFalse);
      expect(find.text(l10n.walletBackupTitle), findsOneWidget);
      final failedAttempts = store.setCount; // ≥1 attempt was made

      // The flaky storage recovers; the user re-taps Continue (the box stayed
      // checked across the failure — Continue is re-enabled).
      store.failSet = null;
      // Let the SnackBar timer drain so it can't leave a pending timer.
      await tester.pump(const Duration(seconds: 4));
      await tester.tap(
        find.widgetWithText(FilledButton, l10n.walletBackupContinue),
        warnIfMissed: false,
      );
      await tester.pumpAndSettle();

      // MONEY-SAFETY: the retry persisted true and ONLY NOW is it deposit-ready.
      expect(store.persisted, isTrue);
      expect(store.lastSetValue, isTrue);
      expect(
        store.setCount,
        greaterThan(failedAttempts),
        reason: 'the retry must make a fresh persist attempt',
      );
      expect(depositReady(tester, l10n), isTrue);
      expect(find.text(l10n.walletBackupTitle), findsNothing);
    });
  });

  // ---------------------------------------------------------------------------
  // 5. AUTO-HIDE is a NO-OP before reveal / on non-backup screens
  // ---------------------------------------------------------------------------
  group('auto-hide is scoped to the revealed backup screen', () {
    testWidgets('backgrounding on the welcome screen is a harmless no-op', (
      tester,
    ) async {
      final lifecycle = await mountL(
        tester,
        provisioner: FakeWalletProvisioner(exists: false),
        store: FakeOnboardingStore(),
      );
      final l10n = l10nOf(tester);
      expect(find.text(l10n.walletOnboardingWelcomeTitle), findsOneWidget);

      // The OS backgrounds the app while only the welcome screen is up. The
      // backup view (the auto-hide owner) isn't even in the tree.
      lifecycle.emit(AppLifecycleState.hidden);
      await tester.pumpAndSettle();

      // Nothing crashed, nothing changed; still Welcome, still not deposit-ready.
      expect(tester.takeException(), isNull);
      expect(find.text(l10n.walletOnboardingWelcomeTitle), findsOneWidget);
      expect(depositReady(tester, l10n), isFalse);
    });

    testWidgets('backgrounding on the NOT-yet-revealed backup screen does '
        'nothing (no spurious reveal-fetch, no crash)', (tester) async {
      final provisioner = FakeWalletProvisioner(exists: false);
      final lifecycle = await mountL(
        tester,
        provisioner: provisioner,
        store: FakeOnboardingStore(),
      );
      final l10n = l10nOf(tester);
      await tester.tap(find.text(l10n.walletCreateButton));
      await tester.pumpAndSettle(); // → AwaitingBackup, words NOT revealed

      expect(find.text(l10n.walletBackupReveal), findsOneWidget);
      expect(provisioner.revealCount, 0); // never revealed

      // Background BEFORE any reveal — even a real `hidden`/`paused`. The listen
      // only acts when `_revealed`, so this is inert: no fetch, no crash.
      lifecycle.emit(AppLifecycleState.hidden);
      await tester.pumpAndSettle();
      lifecycle.emit(AppLifecycleState.resumed);
      await tester.pumpAndSettle();

      expect(tester.takeException(), isNull);
      expect(
        provisioner.revealCount,
        0,
        reason: 'backgrounding pre-reveal must not trigger a seed fetch',
      );
      expect(find.text(l10n.walletBackupReveal), findsOneWidget);
      expect(depositReady(tester, l10n), isFalse);
    });
  });

  // ---------------------------------------------------------------------------
  // 6. BACKGROUNDING WHILE SAVING (money + mobile)
  // ---------------------------------------------------------------------------
  group('backgrounding while saving', () {
    testWidgets('a lifecycle inactive mid-persist does not abort the confirm '
        'nor open the gate early; release → deposit-ready exactly once', (
      tester,
    ) async {
      final hold = Completer<void>();
      final provisioner = FakeWalletProvisioner(exists: false);
      final store = FakeOnboardingStore();
      final lifecycle = await mountL(
        tester,
        provisioner: provisioner,
        store: store,
      );
      final l10n = l10nOf(tester);
      await tester.tap(find.text(l10n.walletCreateButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletBackupReveal));
      await tester.pumpAndSettle();

      // Hold ONLY the confirm true-write (the create-time false-write already
      // ran) so the persist is genuinely in flight when we background the app.
      store.holdSet = hold;
      await tapVisible(tester, find.text(l10n.walletBackupConfirmCheckbox));
      await tester.pump();
      await tapVisible(tester, find.text(l10n.walletBackupContinue));
      await tester.pump(); // → Confirming / _saving, persist awaiting the hold

      // The gate has NOT opened yet (persist still in flight). The saving UI
      // (the button spinner) is up.
      expect(store.persisted, isFalse);
      expect(depositReady(tester, l10n), isFalse);
      expect(find.byType(CircularProgressIndicator), findsOneWidget);

      // The OS backgrounds the app WHILE saving (`hidden`, the snapshot moment).
      // The `!_saving` guard means the in-flight persist is NOT torn down: the
      // saving UI survives (the controller owns the confirm, independent of the
      // view flags), and backgrounding can neither open the gate early nor spawn
      // a second write. (Were the guard absent, `hidden` would collapse the
      // reveal UI and the spinner would vanish — so this genuinely exercises
      // `!_saving`.)
      lifecycle.emit(AppLifecycleState.hidden);
      await tester.pump();

      // The save UI survived the background (NOT collapsed to pre-reveal).
      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      // Still CLOSED, still exactly one confirm true-write — no early open, no
      // second write.
      expect(store.persisted, isFalse);
      expect(depositReady(tester, l10n), isFalse);
      expect(store.trueSetCount, 1);

      // The app returns to the foreground and the still-in-flight persist lands.
      lifecycle.emit(AppLifecycleState.resumed);
      await tester.pump();
      hold.complete();
      await tester.pumpAndSettle();

      // MONEY-SAFETY: the confirm reached deposit-ready exactly once; the gate
      // never opened before the durable persist completed.
      expect(
        store.trueSetCount,
        1,
        reason: 'background-mid-save must not double-persist',
      );
      expect(store.persisted, isTrue);
      expect(store.lastSetValue, isTrue);
      expect(depositReady(tester, l10n), isTrue);
      expect(tester.takeException(), isNull);
    });
  });

  // ---------------------------------------------------------------------------
  // 7. RESET-ON-CREATE crash-safety at the UI (stale confirmed flag)
  // ---------------------------------------------------------------------------
  group('reset-on-create crash-safety', () {
    testWidgets('a stale confirmed:true flag from a wiped prior wallet does '
        'NOT leak the new wallet past the gate', (tester) async {
      // No wallet on disk (exists:false → the create path) BUT the store still
      // carries a stale confirmed=true from a wiped prior wallet. The create
      // path MUST reset the gate so the new wallet lands in forced backup,
      // never straight to deposit-ready.
      final provisioner = FakeWalletProvisioner(exists: false);
      final store = FakeOnboardingStore(confirmed: true);
      await tester.pumpWidget(harness(provisioner: provisioner, store: store));
      await tester.pumpAndSettle(); // probe sees !exists → Welcome
      final l10n = l10nOf(tester);
      expect(find.text(l10n.walletOnboardingWelcomeTitle), findsOneWidget);
      // The stale flag is still true at this point (untouched by the probe).
      expect(store.persisted, isTrue);

      await tester.tap(find.text(l10n.walletCreateButton));
      await tester.pumpAndSettle(); // create resets flag false → AwaitingBackup

      // MONEY-SAFETY: the create reset the stale flag to false and landed in
      // FORCED backup — NEVER deposit-ready off a wiped wallet's residue.
      expect(
        store.persisted,
        isFalse,
        reason:
            'create must reset a stale confirmed flag before the wallet exists',
      );
      expect(store.lastSetValue, isFalse);
      expect(find.text(l10n.walletBackupTitle), findsOneWidget);
      expect(depositReady(tester, l10n), isFalse);

      // And the only path forward is a fresh confirm: reveal → check → continue.
      await tester.tap(find.text(l10n.walletBackupReveal));
      await tester.pumpAndSettle();
      await tapVisible(tester, find.text(l10n.walletBackupConfirmCheckbox));
      await tester.pump();
      await tapVisible(tester, find.text(l10n.walletBackupContinue));
      await tester.pumpAndSettle();

      // Now (and only now) deposit-ready, off a fresh durable confirm.
      expect(store.persisted, isTrue);
      expect(depositReady(tester, l10n), isTrue);
    });
  });
}
