import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/core/lifecycle/app_lifecycle_provider.dart';
import 'package:zec_wallet_ui/core/router/wallet_routes.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/settings/backup_screen.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/reveal_authorization.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/testing.dart';

/// #333 — the post-onboarding recovery-phrase backup screen. These tests aim at
/// the REAL threat model of a Settings reveal (a device someone picked up
/// already unlocked), not at restating the widget tree: the words must stay
/// sealed until a deliberate reveal that PASSES re-auth; a denied or faulted
/// re-auth must NEVER unseal them; a host-managed (raw-seed) wallet must be told
/// the honest truth instead of inventing words; and the on-screen words must
/// vanish the moment the app backgrounds (the recents/app-switcher snapshot).

WalletLocalizations _l10n(WidgetTester tester) =>
    WalletLocalizations.of(tester.element(find.byType(BackupScreen)));

/// The fake's default reveal payload ends in this unique 24th word — a reliable
/// "the words are on screen" probe (the other 23 are all "abandon").
const _uniqueWord = 'art';

const _noMnemonic = WalletApiError(
  code: 'RW-SEED-000',
  message: 'raw-seed wallet has no mnemonic',
  kind: WalletErrorKind.noMnemonic(),
);
// A TRANSIENT reveal failure (device momentarily locked) — distinct from
// noMnemonic, so the screen must offer a retry rather than the terminal
// managed-by-host state.
const _deviceLocked = WalletApiError(
  code: 'RW-STORE-005',
  message: 'keystore unavailable',
  kind: WalletErrorKind.keystoreUnavailable(),
);

Future<ProviderContainer> _pump(
  WidgetTester tester, {
  FakeWalletProvisioner? provisioner,
  bool provisionerNull = false,
  FakeScreenSecurity? security,
  FakeRevealAuthorizer? authorizer,
  double textScale = 1.0,
}) async {
  final container = ProviderContainer(
    overrides: [
      walletProvisionerProvider.overrideWithValue(
        provisionerNull ? null : (provisioner ?? FakeWalletProvisioner()),
      ),
      screenSecurityProvider.overrideWithValue(
        security ?? FakeScreenSecurity(),
      ),
      walletRevealAuthorizerProvider.overrideWithValue(
        authorizer ?? FakeRevealAuthorizer(),
      ),
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
  return container;
}

Future<void> _tapReveal(WidgetTester tester) async {
  final reveal = find.text(_l10n(tester).walletBackupReveal);
  // Scroll it into view first: at a large text scale on a narrow screen the
  // button sits below the fold (a no-op when already visible).
  await tester.ensureVisible(reveal);
  await tester.pumpAndSettle();
  await tester.tap(reveal);
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('the words stay hidden until a deliberate reveal that passes '
      're-auth', (tester) async {
    final provisioner = FakeWalletProvisioner();
    await _pump(tester, provisioner: provisioner);
    final l10n = _l10n(tester);

    // Pre-reveal: the warning + a reveal button, and NOTHING is unsealed yet
    // (shoulder-surf mitigation — the seed is never on screen by default).
    expect(find.text(l10n.walletBackupReveal), findsOneWidget);
    expect(find.text(_uniqueWord), findsNothing);
    expect(provisioner.revealCount, 0);

    await _tapReveal(tester);

    // Post-reveal: the words render and the reveal button is gone.
    expect(find.text(_uniqueWord), findsOneWidget);
    expect(find.text(l10n.walletBackupReveal), findsNothing);
    expect(provisioner.revealCount, 1);
  });

  testWidgets('a DENIED re-auth keeps the seed sealed — no words, no fault, no '
      'unseal', (tester) async {
    final provisioner = FakeWalletProvisioner();
    final authorizer = FakeRevealAuthorizer(deny: true);
    await _pump(tester, provisioner: provisioner, authorizer: authorizer);
    final l10n = _l10n(tester);

    await _tapReveal(tester);

    // The user cancelled the host prompt: the screen silently stays pre-reveal.
    expect(find.text(_uniqueWord), findsNothing);
    expect(find.text(l10n.walletBackupReveal), findsOneWidget);
    // A cancel is NOT a fault — no error line.
    expect(find.text(l10n.walletBackupRevealFailed), findsNothing);
    expect(authorizer.calls, 1);
    // THE money-safety point: a denied re-auth NEVER unseals the seed.
    expect(provisioner.revealCount, 0);
  });

  testWidgets('a re-auth FAULT (not a cancel) shows an honest retry and reads '
      'no words', (tester) async {
    final provisioner = FakeWalletProvisioner();
    final authorizer = FakeRevealAuthorizer(throwBefore: Exception('boom'));
    await _pump(tester, provisioner: provisioner, authorizer: authorizer);
    final l10n = _l10n(tester);

    await _tapReveal(tester);

    // A re-auth fault reads the re-auth-specific copy (NOT "unlock your device").
    expect(find.text(l10n.walletBackupReauthFailed), findsOneWidget);
    expect(find.text(l10n.walletBackupRetryReveal), findsOneWidget);
    expect(find.text(_uniqueWord), findsNothing);
    expect(provisioner.revealCount, 0);
    // S11 C7: the fault APPEARS after the async re-auth, so a screen reader
    // must hear it (the viewing-key screen's twin always did; this one had
    // no live region until S11).
    expect(
      tester
          .getSemantics(find.text(l10n.walletBackupReauthFailed))
          .flagsCollection
          .isLiveRegion,
      isTrue,
    );
  });

  testWidgets('a transient reveal failure offers a retry that re-runs the '
      'reveal and then shows the words', (tester) async {
    // The device was momentarily locked when the reveal ran; the user unlocks
    // and retries — the real recovery path, not a dead end.
    final provisioner = FakeWalletProvisioner()..failReveal = _deviceLocked;
    await _pump(tester, provisioner: provisioner);
    final l10n = _l10n(tester);

    await _tapReveal(tester);
    expect(find.text(l10n.walletBackupRevealFailed), findsOneWidget);
    expect(find.text(_uniqueWord), findsNothing);
    expect(provisioner.revealCount, 1);

    // Device now unlocked; retry re-runs the reveal.
    provisioner.failReveal = null;
    await tester.tap(find.text(l10n.walletBackupRetryReveal));
    await tester.pumpAndSettle();

    expect(find.text(_uniqueWord), findsOneWidget);
    expect(provisioner.revealCount, 2);
  });

  testWidgets('a session-only host (no provisioner) shows the honest '
      'managed-by-host state with NO reveal button', (tester) async {
    await _pump(tester, provisionerNull: true);
    final l10n = _l10n(tester);

    // Case 2: there is no wallet-local phrase — never a reveal button that
    // couldn't produce words, and never invented words.
    expect(find.text(l10n.walletBackupManagedTitle), findsOneWidget);
    expect(find.text(l10n.walletBackupManagedBody), findsOneWidget);
    expect(find.text(l10n.walletBackupReveal), findsNothing);
  });

  testWidgets('a raw-seed wallet that throws noMnemonic lands on '
      'managed-by-host, not the transient retry', (tester) async {
    final provisioner = FakeWalletProvisioner()..failReveal = _noMnemonic;
    await _pump(tester, provisioner: provisioner);
    final l10n = _l10n(tester);

    await _tapReveal(tester);

    // noMnemonic is a TERMINAL truth (no phrase exists), classified by kind —
    // not the "try again" a transient failure gets.
    expect(find.text(l10n.walletBackupManagedTitle), findsOneWidget);
    expect(find.text(l10n.walletBackupRevealFailed), findsNothing);
    expect(provisioner.revealCount, 1);
  });

  testWidgets('screenshot protection engages on show and releases on unmount', (
    tester,
  ) async {
    final security = FakeScreenSecurity();
    await _pump(tester, security: security);
    expect(security.enableCount, 1);
    expect(security.disableCount, 0);

    // Leaving the screen (any navigation away) releases the protection.
    await tester.pumpWidget(const MaterialApp(home: SizedBox.shrink()));
    await tester.pumpAndSettle();
    expect(security.disableCount, 1);
  });

  testWidgets('the secure note is ACK-DRIVEN: a capable platform whose block '
      'did not engage reads the UNPROTECTED copy', (tester) async {
    // The B1 shape: a host that never wired the FLAG_SECURE handler must
    // NOT read a false "screenshots are off" over a visible seed.
    final security = FakeScreenSecurity(
      isScreenshotBlockSupported: true,
      engages: false,
    );
    await _pump(tester, security: security);
    final l10n = _l10n(tester);

    expect(find.text(l10n.walletBackupSecureNoteOther), findsOneWidget);
    expect(find.text(l10n.walletBackupSecureNoteAndroid), findsNothing);
  });

  testWidgets('an engaged block reads the protected copy', (tester) async {
    final security = FakeScreenSecurity(engages: true);
    await _pump(tester, security: security);
    final l10n = _l10n(tester);

    expect(find.text(l10n.walletBackupSecureNoteAndroid), findsOneWidget);
    expect(find.text(l10n.walletBackupSecureNoteOther), findsNothing);
  });

  testWidgets('backgrounding hides the revealed words (the recents-snapshot '
      'mitigation)', (tester) async {
    final container = await _pump(tester);
    await _tapReveal(tester);
    expect(find.text(_uniqueWord), findsOneWidget);

    // The OS takes an app-switcher/window snapshot on background — the words
    // must be gone by then, on every platform (FLAG_SECURE is Android-only).
    (container.read(appLifecycleProvider.notifier) as TestLifecycleNotifier)
        .emit(AppLifecycleState.paused);
    await tester.pumpAndSettle();

    final l10n = _l10n(tester);
    expect(find.text(_uniqueWord), findsNothing);
    expect(find.text(l10n.walletBackupReveal), findsOneWidget);
  });

  testWidgets('the words wrap without overflow at a 3x accessibility text '
      'scale on a narrow screen', (tester) async {
    // Desktop + a11y: a small window at a large text scale must not overflow —
    // the RecoveryWordGrid is a Wrap, and the body scrolls.
    tester.view.physicalSize = const Size(320, 720);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    await _pump(tester, textScale: 3.0);
    await _tapReveal(tester);

    // No RenderFlex overflow was thrown while laying out the scaled words.
    expect(tester.takeException(), isNull);
    // The unique word is present in the (scrollable) tree.
    expect(find.text(_uniqueWord, skipOffstage: false), findsOneWidget);
  });

  testWidgets('Done closes the view-only backup screen and returns to where it '
      'was pushed from', (tester) async {
    // The screen persists nothing; Done just pops. Verify the whole
    // push → reveal → Done → pop round-trip against a real router.
    final router = GoRouter(
      initialLocation: '/',
      routes: [
        GoRoute(
          path: '/',
          builder: (context, state) => Scaffold(
            body: Center(
              child: ElevatedButton(
                onPressed: () => context.push('/backup'),
                child: const Text('open-backup'),
              ),
            ),
          ),
        ),
        GoRoute(
          path: '/backup',
          builder: (context, state) => const BackupScreen(),
        ),
      ],
    );
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          walletProvisionerProvider.overrideWithValue(FakeWalletProvisioner()),
          screenSecurityProvider.overrideWithValue(FakeScreenSecurity()),
          walletRevealAuthorizerProvider.overrideWithValue(
            FakeRevealAuthorizer(),
          ),
          appLifecycleProvider.overrideWith(TestLifecycleNotifier.new),
        ],
        child: MaterialApp.router(
          theme: lightTheme,
          localizationsDelegates: WalletLocalizations.localizationsDelegates,
          supportedLocales: WalletLocalizations.supportedLocales,
          routerConfig: router,
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text('open-backup'));
    await tester.pumpAndSettle();
    expect(find.byType(BackupScreen), findsOneWidget);

    await _tapReveal(tester);
    expect(find.text(_uniqueWord), findsOneWidget);

    await tester.tap(find.text(_l10n(tester).walletBackupDone));
    await tester.pumpAndSettle();

    // Back on the pushing screen; the backup screen is gone.
    expect(find.byType(BackupScreen), findsNothing);
    expect(find.text('open-backup'), findsOneWidget);
  });

  testWidgets('Done on a DEEP-LINKED backup screen (empty back stack) goes to '
      'the wallet root instead of dead no-op (#333 nav)', (tester) async {
    // A host that deep-links straight to /wallet/security/backup gives the
    // screen NO back stack — a bare pop() would be a silent no-op that strands
    // the user on the seed screen. leaveToWalletRoot() must navigate to the
    // wallet root instead so Done always DOES something.
    final router = GoRouter(
      initialLocation: WalletRoutes.backup,
      routes: [
        GoRoute(
          path: WalletRoutes.wallet,
          builder: (context, state) =>
              const Scaffold(body: Center(child: Text('wallet-root-sentinel'))),
        ),
        GoRoute(
          path: WalletRoutes.backup,
          builder: (context, state) => const BackupScreen(),
        ),
      ],
    );
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          walletProvisionerProvider.overrideWithValue(FakeWalletProvisioner()),
          screenSecurityProvider.overrideWithValue(FakeScreenSecurity()),
          walletRevealAuthorizerProvider.overrideWithValue(
            FakeRevealAuthorizer(),
          ),
          appLifecycleProvider.overrideWith(TestLifecycleNotifier.new),
        ],
        child: MaterialApp.router(
          theme: lightTheme,
          localizationsDelegates: WalletLocalizations.localizationsDelegates,
          supportedLocales: WalletLocalizations.supportedLocales,
          routerConfig: router,
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byType(BackupScreen), findsOneWidget);

    await _tapReveal(tester);
    expect(find.text(_uniqueWord), findsOneWidget);

    await tester.tap(find.text(_l10n(tester).walletBackupDone));
    await tester.pumpAndSettle();

    // Landed on the wallet root; the backup screen is gone (NOT a dead button).
    expect(find.byType(BackupScreen), findsNothing);
    expect(find.text('wallet-root-sentinel'), findsOneWidget);
  });

  testWidgets('a same-frame double-tap does not stack two re-auth prompts', (
    tester,
  ) async {
    // The credential host this seam exists for would otherwise show TWO stacked
    // biometric/passphrase prompts. The re-auth is parked (gate) so it is still
    // in flight when the second tap lands in the SAME frame (before the button's
    // onPressed-disable rebuilds) — only the synchronous guard stops it.
    final gate = Completer<void>();
    final authorizer = FakeRevealAuthorizer(gate: gate);
    await _pump(tester, authorizer: authorizer);
    final reveal = find.text(_l10n(tester).walletBackupReveal);

    await tester.tap(reveal); // parks in authorizeReveal; _authorizing = true
    await tester.tap(reveal); // same frame (no pump) — button not yet disabled
    await tester.pump();
    expect(authorizer.calls, 1);

    gate.complete();
    await tester.pumpAndSettle();
    expect(find.text(_uniqueWord), findsOneWidget);
    expect(authorizer.calls, 1);
  });

  testWidgets('re-revealing after a background requires re-auth AGAIN', (
    tester,
  ) async {
    final authorizer = FakeRevealAuthorizer();
    final container = await _pump(tester, authorizer: authorizer);

    await _tapReveal(tester);
    expect(find.text(_uniqueWord), findsOneWidget);
    expect(authorizer.calls, 1);

    // Background hides the words and returns to the pre-reveal state.
    (container.read(appLifecycleProvider.notifier) as TestLifecycleNotifier)
        .emit(AppLifecycleState.paused);
    await tester.pumpAndSettle();
    expect(find.text(_uniqueWord), findsNothing);

    // Seeing them again must re-run re-auth — not silently re-show.
    await _tapReveal(tester);
    expect(authorizer.calls, 2);
    expect(find.text(_uniqueWord), findsOneWidget);
  });

  testWidgets('a re-auth fault recovers when the user re-authenticates', (
    tester,
  ) async {
    // The host credential step errored (not a cancel); the user fixes it and
    // re-taps — the retry drives all the way to the words, not a dead end.
    final provisioner = FakeWalletProvisioner();
    final authorizer = FakeRevealAuthorizer(throwBefore: Exception('boom'));
    await _pump(tester, provisioner: provisioner, authorizer: authorizer);
    final l10n = _l10n(tester);

    await _tapReveal(tester);
    expect(find.text(l10n.walletBackupReauthFailed), findsOneWidget);
    expect(provisioner.revealCount, 0);

    // Fault cleared; the button now reads "try again".
    authorizer.throwBefore = null;
    await tester.tap(find.text(l10n.walletBackupRetryReveal));
    await tester.pumpAndSettle();

    expect(find.text(_uniqueWord), findsOneWidget);
    expect(authorizer.calls, 2);
    expect(provisioner.revealCount, 1);
  });

  testWidgets('a background WHILE re-auth is in flight does not tear it down', (
    tester,
  ) async {
    // The host prompt is up when the app backgrounds; the SDK gate must just
    // keep awaiting — no crash, and the reveal completes on the way back.
    final gate = Completer<void>();
    final authorizer = FakeRevealAuthorizer(gate: gate);
    final container = await _pump(tester, authorizer: authorizer);

    await tester.tap(find.text(_l10n(tester).walletBackupReveal));
    await tester.pump(); // spinner; re-auth parked on the gate

    (container.read(appLifecycleProvider.notifier) as TestLifecycleNotifier)
        .emit(AppLifecycleState.paused);
    await tester.pump();
    expect(tester.takeException(), isNull);

    gate.complete();
    await tester.pumpAndSettle();
    expect(find.text(_uniqueWord), findsOneWidget);
    expect(authorizer.calls, 1);
  });

  testWidgets('after a cancelled re-auth the Reveal button is still live and a '
      'second attempt shows the words', (tester) async {
    // The "looks like a dead button" concern: a cancel must not leave the user
    // stuck — a later tap (or a credential host that passes the second time)
    // still reveals.
    final provisioner = FakeWalletProvisioner();
    final authorizer = FakeRevealAuthorizer(deny: true);
    await _pump(tester, provisioner: provisioner, authorizer: authorizer);

    await _tapReveal(tester);
    expect(find.text(_uniqueWord), findsNothing);
    expect(provisioner.revealCount, 0);

    authorizer.deny = false;
    await _tapReveal(tester);
    expect(find.text(_uniqueWord), findsOneWidget);
    expect(authorizer.calls, 2);
    expect(provisioner.revealCount, 1);
  });

  testWidgets(
    'an empty word list from a misbehaving host shows managed-by-host, '
    'never blank backup chrome',
    (tester) async {
      // A host provisioner that returns [] instead of throwing noMnemonic must
      // NOT render the backup title/secure-note/Done around zero words.
      final provisioner = FakeWalletProvisioner(recoveryWords: const []);
      await _pump(tester, provisioner: provisioner);
      final l10n = _l10n(tester);

      await _tapReveal(tester);
      expect(find.text(l10n.walletBackupManagedTitle), findsOneWidget);
      expect(find.text(l10n.walletBackupDone), findsNothing);
    },
  );

  testWidgets(
    'the revealed words expose an ORDERED per-word screen-reader label',
    (tester) async {
      // Money-critical accessibility: the copy says write the words "in order",
      // so each word must read as one node pairing its index with the word, not
      // as ~2N unanchored stops.
      final handle = tester.ensureSemantics();
      await _pump(tester);
      await _tapReveal(tester);

      expect(find.bySemanticsLabel('word 1: abandon'), findsOneWidget);
      expect(find.bySemanticsLabel('word 24: art'), findsOneWidget);
      handle.dispose();
    },
  );
}
