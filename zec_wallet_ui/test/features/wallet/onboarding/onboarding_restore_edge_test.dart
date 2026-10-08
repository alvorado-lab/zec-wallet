import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart' show RenderParagraph;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/lifecycle/app_lifecycle_provider.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/bip39_wordlist.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_controller.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_state.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_screen.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// SECOND-PASS real-world-edge tests for the RESTORE slice — the money,
/// disposal, and mobile-lifecycle edges the first-pass restore tests in
/// `onboarding_controller_test.dart` (happy path, normalize chokepoint, the
/// per-fault classifier arms, the confirm-open-persist-fails revert) and
/// `onboarding_flow_test.dart` (length gate, end-to-end lowercase, inline
/// fault) do NOT pin. Driven through the real controller against host-VM fakes.
///
/// THE invariant, restated for restore: a deposit-capable `WalletSession` is
/// exposed ONLY in [OnboardingActive], and only after the backup confirmation
/// is durably persisted. Restore goes straight to Active (the user holds the
/// phrase) — so the crash-safe `false`-then-`true` gate-write ordering is the
/// whole money guarantee, and every edge here re-asserts no session leaks
/// before the durable `true` write.
void main() {
  // ===========================================================================
  // PART A — controller-level edges (no widgets)
  // ===========================================================================
  ProviderContainer harness(
    FakeWalletProvisioner provisioner,
    FakeOnboardingStore store,
  ) {
    final container = ProviderContainer(
      overrides: [
        walletProvisionerProvider.overrideWithValue(provisioner),
        onboardingStoreProvider.overrideWithValue(store),
      ],
    );
    addTearDown(container.dispose);
    container.listen(onboardingControllerProvider, (_, _) {});
    container.listen(walletSessionProvider, (_, _) {});
    return container;
  }

  OnboardingState stateOf(ProviderContainer c) =>
      c.read(onboardingControllerProvider);
  WalletSession? sessionOf(ProviderContainer c) =>
      c.read(walletSessionProvider);
  OnboardingController notifierOf(ProviderContainer c) =>
      c.read(onboardingControllerProvider.notifier);

  Future<ProviderContainer> atRestoreInput(
    FakeWalletProvisioner p,
    FakeOnboardingStore store,
  ) async {
    final c = harness(p, store);
    await pumpEventQueue(); // → Welcome
    notifierOf(c).beginRestore();
    return c;
  }

  group('startRestore with an all-whitespace (empty-after-normalize) phrase', () {
    test(
      'an empty word list is SOLE-GUARDED by the controller — it never reaches '
      'the SDK, never touches the gate; the user lands on a fixable fault',
      () async {
        // The chokepoint owns the contract for EVERY caller: an all-whitespace
        // phrase normalizes to [] and the controller short-circuits to a fixable
        // invalid-phrase fault BEFORE touching the gate or the SDK. (The screen
        // also gates word count, but the controller is the sole guard.) No
        // gate-close, no wasted SDK round-trip, never deposit-ready.
        final p = FakeWalletProvisioner(exists: false);
        final store = FakeOnboardingStore();
        final c = await atRestoreInput(p, store);

        notifierOf(c).startRestore(const ['  ', '\t', '\n']);
        await pumpEventQueue();

        // Short-circuited: the SDK was NEVER called, the gate NEVER touched.
        expect(p.restoreCount, 0, reason: 'empty phrase never reaches the SDK');
        expect(store.setCount, 0, reason: 'no gate-close for an empty phrase');
        // Back on the form with a fixable invalid-phrase fault, no session.
        expect(stateOf(c), isA<OnboardingRestoreInput>());
        expect(
          (stateOf(c) as OnboardingRestoreInput).fault,
          RestoreInputFault.invalidWord,
        );
        expect(sessionOf(c), isNull, reason: 'an empty phrase is never funded');
      },
    );
  });

  group('the crash-safe `false` gate-close itself fails', () {
    test('if closing the gate fails, restore() is NEVER called and no session is '
        'exposed — the wallet is not written without a closed gate', () async {
      // The existing restore tests fail the `true` OPEN write (via failSetTrue)
      // and the confirm-revert. This pins the OTHER write: the crash-safe
      // `false` CLOSE that runs BEFORE the wallet is written. If THAT throws,
      // the wallet must never be restored to disk (mirrors startCreate\'s
      // gate-reset-fails test) — else a crash could strand an ungated wallet.
      final p = FakeWalletProvisioner(exists: false);
      final store = FakeOnboardingStore()
        ..failSet = const WalletApiError(
          code: 'RW-PREFS-IO',
          message: 'prefs down',
          kind: WalletErrorKind.io(),
        );
      final c = await atRestoreInput(p, store);

      notifierOf(c).startRestore(const ['abandon', 'ability', 'about']);
      await pumpEventQueue();

      // The gate-close threw FIRST → restore() never ran → no wallet written.
      expect(
        p.restoreCount,
        0,
        reason: 'no wallet restored without a durably-closed gate',
      );
      expect(
        store.lastSetValue,
        isFalse,
        reason: 'the false close was the attempted (and failing) write',
      );
      expect(store.trueSetCount, 0, reason: 'the gate never opened');
      // A non-input-fault provisioning failure → the generic Failed screen.
      expect(stateOf(c), isA<OnboardingFailed>());
      expect(sessionOf(c), isNull);
      expect(store.persisted, isNot(true));
    });

    test(
      'a CONFIRM-open (`true`) write failure after a SUCCESSFUL restore holds at '
      'forced backup with NO session — never deposit-ready on a lost confirm',
      () async {
        // The twin of the false-close test, made explicit at the controller
        // level with the generic failSet (the existing test uses failSetTrue;
        // this drives the same outcome and asserts the gate-closed residue):
        // the false-close lands, the wallet IS restored, but the true-open write
        // fails → the controller holds at AwaitingBackup (gate CLOSED), so the
        // user re-confirms with the phrase rather than getting a funded-but-
        // unconfirmed wallet.
        final p = FakeWalletProvisioner(exists: false);
        final store = FakeOnboardingStore()
          ..failSetTrue = const WalletApiError(
            code: 'RW-PREFS-IO',
            message: 'prefs write failed on open',
            kind: WalletErrorKind.diskFull(),
          );
        final c = await atRestoreInput(p, store);

        notifierOf(c).startRestore(const ['abandon', 'ability', 'about']);
        await pumpEventQueue();

        expect(p.restoreCount, 1, reason: 'the wallet WAS restored to disk');
        expect(
          stateOf(c),
          isA<OnboardingAwaitingBackup>(),
          reason: 'held at forced backup, NOT active',
        );
        expect(sessionOf(c), isNull, reason: 'gate stays closed → no deposit');
        expect(
          store.persisted,
          isFalse,
          reason: 'only the crash-safe false close stuck',
        );
        expect(
          store.trueSetCount,
          1,
          reason: 'the true-open write was ATTEMPTED (and failed)',
        );
      },
    );
  });

  group('beginRestore ↔ cancelRestore rapid cycling stays consistent', () {
    test('begin → cancel → begin → cancel leaves a clean Welcome, no session, '
        'and never touches the provisioner/store', () async {
      // A jittery user (or a flaky tap) bouncing between Welcome and the
      // restore form. Both transitions are synchronous and guarded; the
      // rotation must never expose a session, never provision anything, and
      // land deterministically wherever the last legal action left it.
      final p = FakeWalletProvisioner(exists: false);
      final store = FakeOnboardingStore();
      final c = harness(p, store);
      await pumpEventQueue(); // → Welcome
      final n = notifierOf(c);

      n.beginRestore();
      expect(stateOf(c), isA<OnboardingRestoreInput>());
      n.cancelRestore();
      expect(stateOf(c), isA<OnboardingWelcome>());
      n.beginRestore();
      expect(stateOf(c), isA<OnboardingRestoreInput>());
      n.cancelRestore();
      expect(stateOf(c), isA<OnboardingWelcome>());

      // Stray guarded calls from the wrong phase are no-ops, not crashes.
      n.cancelRestore(); // already Welcome → no-op
      expect(stateOf(c), isA<OnboardingWelcome>());
      n.beginRestore();
      n.beginRestore(); // second begin from RestoreInput → no-op (not Welcome)
      expect(stateOf(c), isA<OnboardingRestoreInput>());

      // Nothing was provisioned and no gate write happened across the churn,
      // and at no point was a session exposed.
      expect(p.restoreCount, 0);
      expect(p.createCount, 0);
      expect(p.openCount, 0);
      expect(store.setCount, 0);
      expect(sessionOf(c), isNull);
    });

    test('a fault carried on a re-entered RestoreInput is CLEARED by a '
        'cancel→begin round-trip (back to a clean first-entry form)', () async {
      // After a mistyped-word fault the state is RestoreInput(fault:…). If the
      // user backs out to Welcome and re-enters restore, the fresh
      // beginRestore must produce a CLEAN RestoreInput (no stale fault), so the
      // screen does not show a phantom error over a brand-new attempt.
      final p = FakeWalletProvisioner(exists: false)
        ..failRestore = const WalletApiError(
          code: 'RW-SEED-002',
          message: 'unknown word',
          kind: WalletErrorKind.invalidMnemonic(wordIndex: 3),
        );
      final c = await atRestoreInput(p, FakeOnboardingStore());

      notifierOf(c).startRestore(const ['abandon', 'ability', 'about']);
      await pumpEventQueue();
      final faulted = stateOf(c);
      expect(faulted, isA<OnboardingRestoreInput>());
      expect(
        (faulted as OnboardingRestoreInput).fault,
        RestoreInputFault.invalidWord,
      );

      // Back out, then re-enter restore: a clean form.
      notifierOf(c).cancelRestore();
      expect(stateOf(c), isA<OnboardingWelcome>());
      notifierOf(c).beginRestore();
      final reentered = stateOf(c);
      expect(reentered, isA<OnboardingRestoreInput>());
      expect(
        (reentered as OnboardingRestoreInput).fault,
        isNull,
        reason: 're-entering restore must not carry the prior fault',
      );
      expect(reentered.invalidWordIndex, isNull);
    });
  });

  group('disposal safety mid-restore (no post-dispose state write)', () {
    test(
      'disposing while restore() is in flight never sets state after dispose '
      '(the _disposed guard after the restore await swallows the continuation)',
      () async {
        // Mirrors the disposing-mid-create / mid-confirm tests for the restore
        // path: a container torn down while provisioner.restore() is held must
        // not let the continuation touch the torn-down ref (a post-dispose
        // `state =` would throw on shutdown).
        final hold = Completer<void>();
        final p = FakeWalletProvisioner(exists: false)..holdRestore = hold;
        final store = FakeOnboardingStore();
        final container = ProviderContainer(
          overrides: [
            walletProvisionerProvider.overrideWithValue(p),
            onboardingStoreProvider.overrideWithValue(store),
          ],
        );
        container.listen(onboardingControllerProvider, (_, _) {});
        container.listen(walletSessionProvider, (_, _) {});
        await pumpEventQueue(); // → Welcome
        container.read(onboardingControllerProvider.notifier).beginRestore();

        final restoring = container
            .read(onboardingControllerProvider.notifier)
            .startRestore(const ['abandon', 'ability', 'about']);
        await pumpEventQueue(); // gate-close persisted; restore() awaiting hold
        expect(
          container.read(onboardingControllerProvider),
          isA<OnboardingRestoring>(),
          reason: 'blocked in restore(), before the confirm-open write',
        );

        container.dispose(); // tear down WHILE restore is in flight
        hold.complete(); // restore resolves AFTER dispose
        await restoring; // the continuation runs; the guard must swallow it
        await pumpEventQueue(); // a post-dispose `state =` would throw here
        // Reaching here without an exception is the assertion. The true-open
        // write must NOT have run (the guard fired right after the restore await,
        // before the confirm-open block).
        expect(
          store.trueSetCount,
          0,
          reason: 'the confirm-open never ran on a torn-down controller',
        );
      },
    );

    test('disposing while the crash-safe `false` gate-close is in flight never '
        'sets state after dispose (and restore() is never reached)', () async {
      // The earliest await in startRestore is the gate-close. Hold THAT, so
      // dispose lands before restore() is even attempted; the guard after the
      // gate-close await must abort the flow with no wallet written.
      final holdSet = Completer<void>();
      final p = FakeWalletProvisioner(exists: false);
      final store = FakeOnboardingStore()..holdSet = holdSet;
      final container = ProviderContainer(
        overrides: [
          walletProvisionerProvider.overrideWithValue(p),
          onboardingStoreProvider.overrideWithValue(store),
        ],
      );
      container.listen(onboardingControllerProvider, (_, _) {});
      container.listen(walletSessionProvider, (_, _) {});
      await pumpEventQueue(); // → Welcome
      container.read(onboardingControllerProvider.notifier).beginRestore();

      final restoring = container
          .read(onboardingControllerProvider.notifier)
          .startRestore(const ['abandon', 'ability', 'about']);
      await pumpEventQueue(); // Restoring; blocked on the held gate-close
      expect(
        container.read(onboardingControllerProvider),
        isA<OnboardingRestoring>(),
      );
      expect(p.restoreCount, 0, reason: 'no wallet restored yet');

      container.dispose(); // tear down WHILE the gate-close is in flight
      holdSet.complete(); // the gate-close resolves AFTER dispose
      await restoring; // continuation runs; the guard must swallow it
      await pumpEventQueue(); // a post-dispose `state =` would throw here

      // The guard fired right after the gate-close await: restore() was never
      // reached, so no wallet was minted into a torn-down session.
      expect(
        p.restoreCount,
        0,
        reason: 'the restore aborted at the _disposed guard, post gate-close',
      );
    });
  });

  group('money-honesty: RestoreInput / Restoring never expose a session', () {
    test('OnboardingRestoreInput (form showing) exposes no session', () async {
      final c = await atRestoreInput(
        FakeWalletProvisioner(exists: false),
        FakeOnboardingStore(),
      );
      expect(stateOf(c), isA<OnboardingRestoreInput>());
      expect(sessionOf(c), isNull);
    });

    test(
      'OnboardingRestoring (restore in flight) exposes no session',
      () async {
        final hold = Completer<void>();
        final p = FakeWalletProvisioner(exists: false)..holdRestore = hold;
        final c = await atRestoreInput(p, FakeOnboardingStore());
        notifierOf(c).startRestore(const ['abandon', 'ability', 'about']);
        await pumpEventQueue(); // blocked in restore() → Restoring
        expect(stateOf(c), isA<OnboardingRestoring>());
        expect(
          sessionOf(c),
          isNull,
          reason: 'never deposit-ready while the restore is in flight',
        );
        hold.complete();
        await pumpEventQueue();
      },
    );
  });

  // ===========================================================================
  // PART B — widget-level edges (the restore SCREEN)
  // ===========================================================================
  WalletLocalizations l10nOf(WidgetTester tester) =>
      WalletLocalizations.of(tester.element(find.byType(WalletScreen)));

  // Mount the restore screen over a CONTROLLABLE lifecycle (provider overridden,
  // platform binding bypassed) and drive Welcome → Restore. Returns the
  // lifecycle notifier so a test can `emit` a background/resume deterministically.
  Future<TestLifecycleNotifier> pumpToRestoreL(
    WidgetTester tester, {
    required FakeWalletProvisioner provisioner,
    required FakeOnboardingStore store,
  }) async {
    final container = ProviderContainer(
      overrides: [
        walletProvisionerProvider.overrideWithValue(provisioner),
        onboardingStoreProvider.overrideWithValue(store),
        bip39WordlistProvider.overrideWith((ref) => testBip39Wordlist),
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
    await tester.pumpAndSettle(); // boot → Welcome
    await tester.tap(find.text(l10nOf(tester).walletRestoreButton));
    await tester.pumpAndSettle(); // → RestoreInput
    return container.read(appLifecycleProvider.notifier)
        as TestLifecycleNotifier;
  }

  Future<WalletLocalizations> pumpToRestore(
    WidgetTester tester, {
    required FakeWalletProvisioner provisioner,
    required FakeOnboardingStore store,
    TextScaler? textScaler,
  }) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          walletProvisionerProvider.overrideWithValue(provisioner),
          onboardingStoreProvider.overrideWithValue(store),
          bip39WordlistProvider.overrideWith((ref) => testBip39Wordlist),
        ],
        child: MaterialApp(
          localizationsDelegates: WalletLocalizations.localizationsDelegates,
          supportedLocales: WalletLocalizations.supportedLocales,
          theme: lightTheme,
          // Force a text scale (accessibility) when a test asks — the overflow
          // pins drive the restore screen at large dynamic text.
          builder: textScaler == null
              ? null
              : (context, child) => MediaQuery(
                  data: MediaQuery.of(context).copyWith(textScaler: textScaler),
                  child: child!,
                ),
          home: const WalletScreen(),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final l10n = l10nOf(tester);
    await tester.tap(find.text(l10n.walletRestoreButton));
    await tester.pumpAndSettle();
    return l10n;
  }

  Future<void> tapVisible(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
  }

  // The pill field commits a word on whitespace and ACCUMULATES pills; a trailing
  // space commits the last word. `enterText` only replaces the in-progress token.
  Future<void> addWords(WidgetTester tester, String words) async {
    await tester.enterText(find.byType(TextField), '$words ');
    await tester.pump();
  }

  group(
    'the typed phrase SURVIVES a background/resume (it must NOT auto-clear)',
    () {
      testWidgets(
        'backgrounding (hidden then paused) and resuming mid-typing leaves the '
        'phrase intact — unlike the backup screen, the restore field has no '
        'auto-hide (it would destroy in-progress input)',
        (tester) async {
          final lifecycle = await pumpToRestoreL(
            tester,
            provisioner: FakeWalletProvisioner(exists: false),
            store: FakeOnboardingStore(),
          );
          // A few words entered as pills (mid-restore).
          await addWords(tester, 'abandon ability able');
          // Three committed pills are on screen.
          expect(find.text('abandon'), findsOneWidget);
          expect(find.text('ability'), findsOneWidget);
          expect(find.text('able'), findsOneWidget);

          // The OS takes the app-switcher snapshot (`hidden`), then fully
          // backgrounds (`paused`) — the moments the backup screen wipes the seed.
          // The restore field must KEEP the user's in-progress pills.
          lifecycle.emit(AppLifecycleState.hidden);
          await tester.pumpAndSettle();
          expect(
            find.text('abandon'),
            findsOneWidget,
            reason: 'a hidden snapshot must not wipe in-progress restore pills',
          );

          lifecycle.emit(AppLifecycleState.paused);
          await tester.pumpAndSettle();
          expect(
            find.text('able'),
            findsOneWidget,
            reason: 'a full background must not wipe in-progress restore pills',
          );

          // Resume: still there, no re-type. No exception anywhere.
          lifecycle.emit(AppLifecycleState.resumed);
          await tester.pumpAndSettle();
          expect(
            find.text('ability'),
            findsOneWidget,
            reason: 'the user returns to exactly the pills they had entered',
          );
          expect(tester.takeException(), isNull);
        },
      );
    },
  );

  group('birthday picker', () {
    testWidgets(
      'the restore birthday DEFAULTS to a recent date (fast first sync), with '
      'an honest change + scan-all option',
      (tester) async {
        final l10n = await pumpToRestore(
          tester,
          provisioner: FakeWalletProvisioner(exists: false),
          store: FakeOnboardingStore(),
        );

        // Maintainer: default to ~6 months ago so the first sync is fast. A date is
        // set out of the box — NOT the full-scan "none" copy — and the change +
        // scan-all affordances are present so an older wallet can opt out.
        expect(find.text(l10n.walletRestoreBirthdayNone), findsNothing);
        expect(find.text(l10n.walletRestoreBirthdayChange), findsOneWidget);
        expect(find.text(l10n.walletRestoreBirthdayClear), findsOneWidget);
        expect(find.text(l10n.walletRestoreBirthdayPick), findsNothing);
      },
    );

    testWidgets(
      'a clean submit threads the default (non-null) birthday to the SDK',
      (tester) async {
        // The fast default must actually REACH the SDK on an untouched submit —
        // not silently drop to a full scan (which would defeat the whole change).
        final provisioner = FakeWalletProvisioner(exists: false);
        final l10n = await pumpToRestore(
          tester,
          provisioner: provisioner,
          store: FakeOnboardingStore(),
        );

        await addWords(tester, List.filled(24, 'abandon').join(' '));
        await tapVisible(tester, find.text(l10n.walletRestoreSubmit));
        await tester.pumpAndSettle();

        expect(
          provisioner.lastRestoreCreationTime,
          isNotNull,
          reason: 'the ~6-months-ago default rides every untouched restore',
        );
      },
    );

    testWidgets(
      'cancelling the date picker leaves the default birthday UNCHANGED',
      (tester) async {
        final l10n = await pumpToRestore(
          tester,
          provisioner: FakeWalletProvisioner(exists: false),
          store: FakeOnboardingStore(),
        );

        // Open the picker from the default (a date is set ⇒ "Change date"), then
        // CANCEL it (returns null).
        await tapVisible(tester, find.text(l10n.walletRestoreBirthdayChange));
        await tester.pumpAndSettle();
        // The Material date picker's Cancel action.
        await tester.tap(find.text('Cancel'));
        await tester.pumpAndSettle();

        // Unchanged: still a date (Change + Scan all history), never reset to the
        // full-scan "none" — a cancelled pick must not drop the default.
        expect(find.text(l10n.walletRestoreBirthdayNone), findsNothing);
        expect(find.text(l10n.walletRestoreBirthdayChange), findsOneWidget);
        expect(find.text(l10n.walletRestoreBirthdayClear), findsOneWidget);
      },
    );

    testWidgets(
      'a fixable fault preserves BOTH the typed phrase AND a previously chosen '
      'birthday (no retype, no re-pick after a single-word typo)',
      (tester) async {
        // The richest real-world recovery: the user picks a birthday to speed
        // the scan, types 24 words, fat-fingers one, and submits. The inline
        // fault must keep the screen mounted with BOTH the phrase AND the chosen
        // date — re-picking a date or re-typing 24 words after a typo is exactly
        // the friction that pushes users to skip the birthday (a slow scan) or
        // give up.
        final provisioner = FakeWalletProvisioner(exists: false)
          ..failRestore = const WalletApiError(
            code: 'RW-SEED-002',
            message: 'unknown word',
            kind: WalletErrorKind.invalidMnemonic(wordIndex: 4),
          );
        final l10n = await pumpToRestore(
          tester,
          provisioner: provisioner,
          store: FakeOnboardingStore(),
        );

        // A birthday is set by default (the ~6-months-ago fast default) — the
        // "Change" affordance + a Clear button are present out of the box.
        expect(find.text(l10n.walletRestoreBirthdayChange), findsOneWidget);
        expect(find.text(l10n.walletRestoreBirthdayClear), findsOneWidget);
        expect(find.text(l10n.walletRestoreBirthdayNone), findsNothing);

        // Enter a 24-word phrase (all valid pills) and submit; the SDK reports a
        // bad word (the checksum-class fault path).
        await addWords(tester, List.filled(24, 'abandon').join(' '));
        await tapVisible(tester, find.text(l10n.walletRestoreSubmit));
        await tester.pumpAndSettle();

        // Still on the restore screen with the inline fault…
        expect(find.text(l10n.walletRestoreTitle), findsOneWidget);
        expect(
          find.text(l10n.walletRestoreFaultInvalidWord(5)),
          findsOneWidget,
        );
        // …the entered pills SURVIVED (the same view stayed mounted)…
        expect(find.text('abandon'), findsWidgets);
        // …AND the chosen birthday SURVIVED (still "Change", not back to "Pick").
        expect(
          find.text(l10n.walletRestoreBirthdayChange),
          findsOneWidget,
          reason: 'a fixable fault must not discard the chosen birthday',
        );
        expect(find.text(l10n.walletRestoreBirthdayNone), findsNothing);
        // Never deposit-ready on a failed restore.
        expect(find.text(l10n.walletBalanceLabel), findsNothing);
        // The chosen birthday was actually threaded to the SDK on submit.
        expect(provisioner.lastRestoreCreationTime, isNotNull);
      },
    );

    // on-device: the two-button row overflowed by 6.4px on a narrow
    // phone. The first fix shared the row with ellipsized labels, and this
    // pin checked only for the overflow, so it passed while the walk
    // (TANK MINI, 332dp at 1.2x) showed "Chan…" and "Scan all hi…". The row
    // now wraps; the pin asserts no overflow AND that neither label is cut.
    // 280dp at 3.0x: a label wider than the whole row wraps INSIDE its button
    // (Flutter bounds an icon button's label with a Flexible), never overflows.
    for (final (width, scale) in const [
      (320.0, 1.4),
      (332.0, 1.2),
      (280.0, 3.0),
    ]) {
      testWidgets(
        'the birthday row (Change date + Scan all history) renders whole, '
        'without overflow or a cut label, at ${width.toInt()}dp and ${scale}x '
        'text',
        (tester) async {
          tester.view.physicalSize = Size(width, 1400);
          tester.view.devicePixelRatio = 1.0;
          addTearDown(tester.view.resetPhysicalSize);
          addTearDown(tester.view.resetDevicePixelRatio);

          final l10n = await pumpToRestore(
            tester,
            provisioner: FakeWalletProvisioner(exists: false),
            store: FakeOnboardingStore(),
            textScaler: TextScaler.linear(scale),
          );

          // The default birthday is set, so the row shows BOTH buttons.
          for (final label in [
            l10n.walletRestoreBirthdayChange,
            l10n.walletRestoreBirthdayClear,
          ]) {
            expect(find.text(label), findsOneWidget);
            expect(
              tester
                  .renderObject<RenderParagraph>(find.text(label))
                  .didExceedMaxLines,
              isFalse,
              reason: '"$label" must render whole, never ellipsized',
            );
          }
          expect(
            tester.takeException(),
            isNull,
            reason: 'no overflow at ${scale}x scale on a ${width}dp width',
          );
        },
      );
    }

    testWidgets(
      'Scan all history clears the default to a full, money-safe scan',
      (tester) async {
        final l10n = await pumpToRestore(
          tester,
          provisioner: FakeWalletProvisioner(exists: false),
          store: FakeOnboardingStore(),
        );
        // The escape hatch for an older wallet: clear the recent default → the
        // slow but money-safe full history scan.
        expect(find.text(l10n.walletRestoreBirthdayChange), findsOneWidget);
        await tapVisible(tester, find.text(l10n.walletRestoreBirthdayClear));
        await tester.pumpAndSettle();

        // Back to "scan everything" → honestly shown, with only the Pick action.
        expect(find.text(l10n.walletRestoreBirthdayNone), findsOneWidget);
        expect(find.text(l10n.walletRestoreBirthdayPick), findsOneWidget);
        expect(find.text(l10n.walletRestoreBirthdayClear), findsNothing);
      },
    );
  });

  group('pasting a whole phrase splits into pills', () {
    testWidgets(
      'a 24-word phrase pasted with mixed Unicode whitespace splits into 24 '
      'pills and enables Submit (the field uses the same normalizer as the SDK)',
      (tester) async {
        final l10n = await pumpToRestore(
          tester,
          provisioner: FakeWalletProvisioner(exists: false),
          store: FakeOnboardingStore(),
        );
        // 24 words glued by NBSP + ideographic space + a regular space — a real
        // cross-app paste. The pill field must split them, not fuse into one.
        final soup = List.generate(
          24,
          (i) => 'Abandon',
        ).join(' '); // NBSP-joined, mixed case
        await tester.enterText(find.byType(TextField), '$soup ');
        await tester.pump();

        expect(
          find.text(l10n.walletRestoreWordCount(24)),
          findsOneWidget,
          reason: 'NBSP-separated words are split into 24 pills, not fused',
        );
        // 24 pills are rendered (the same word repeated).
        expect(find.text('abandon'), findsNWidgets(24));
        final submit = tester.widget<FilledButton>(
          find.widgetWithText(FilledButton, l10n.walletRestoreSubmit),
        );
        expect(
          submit.onPressed,
          isNotNull,
          reason: '24 words → Submit enabled',
        );
      },
    );
  });
}
