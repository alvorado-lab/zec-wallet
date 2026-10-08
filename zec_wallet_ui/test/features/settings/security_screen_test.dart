import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:zec_wallet_ui/core/lifecycle/app_lifecycle_provider.dart';
import 'package:zec_wallet_ui/core/router/wallet_routes.dart';
import 'package:zec_wallet_ui/core/theme/colors.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/settings/backup_screen.dart';
import 'package:zec_wallet_ui/features/settings/security_screen.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_controller.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_state.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/shared/wallet_info_button.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

import '../../support/dialog_reach.dart';

/// Security-screen widget tests (FR-14 H1): the custody card must tell the
/// truth — a hardware tier reads the hardware-held key line, a best-effort tier
/// reads the residual disclosure, and a probe failure is honest, NEVER a false
/// "protected". The delete confirmation must warn before the destructive action.
WalletLocalizations _l10n(WidgetTester tester) =>
    WalletLocalizations.of(tester.element(find.byType(SecurityScreen)));

/// The (i) carrying [body] (S13 Build B moved the custody erase line and the
/// delete explanation behind an (i) after their label; the honesty rows
/// below assert WHICH line the (i) carries).
Finder _info(String body) =>
    find.byWidgetPredicate((w) => w is WalletInfoButton && w.body == body);

Widget _harness(AsyncValue<CustodyDisclosure> disclosure) {
  return ProviderScope(
    overrides: [
      // The screen re-gates on package custody (a wired provisioner) before it
      // renders anything — these tests' premise is the package-managed wallet,
      // so wire the fake (the session-only host case has its own test below).
      walletProvisionerProvider.overrideWithValue(FakeWalletProvisioner()),
      // Feed a fixed disclosure so the screen renders deterministically (the
      // real probe needs a device keychain). autoDispose override.
      walletCustodyDisclosureProvider.overrideWith(
        (ref) => switch (disclosure) {
          AsyncData(:final value) => Future<CustodyDisclosure>.value(value),
          AsyncError(:final error) => Future<CustodyDisclosure>.error(error),
          _ => Future<CustodyDisclosure>.delayed(const Duration(days: 1)),
        },
      ),
    ],
    child: MaterialApp(
      theme: lightTheme,
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      home: const SecurityScreen(),
    ),
  );
}

const _hardwareKey = CustodyDisclosure(
  tier: 'apple_secure_enclave',
  eraseAssurance: EraseAssurance.hardwareKeyDeleted,
  degraded: false,
);
// A best-effort tier with degraded:false — the screen must STILL render the
// best-effort copy (keying off eraseAssurance, NOT degraded).
const _bestEffortNone = CustodyDisclosure(
  tier: 'none',
  eraseAssurance: EraseAssurance.bestEffort,
  degraded: false,
);

void main() {
  testWidgets('a hardware tier says its key is held by secure hardware', (
    tester,
  ) async {
    await tester.pumpWidget(_harness(const AsyncData(_hardwareKey)));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    expect(find.text(l10n.securityCustodyTierSecureEnclave), findsOneWidget);
    expect(_info(l10n.securityCustodyHardwareKey), findsOneWidget);
    // The best-effort caution must NOT appear for a hardware tier.
    expect(_info(l10n.securityCustodyBestEffort), findsNothing);
  });

  testWidgets(
    'a best-effort tier (degraded:false) STILL renders the residual disclosure',
    (tester) async {
      await tester.pumpWidget(_harness(const AsyncData(_bestEffortNone)));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      expect(find.text(l10n.securityCustodyTierNone), findsOneWidget);
      // The honesty keys off eraseAssurance, not degraded: a "none" tier with
      // degraded:false must STILL warn (the AV2-honesty point the review caught).
      expect(_info(l10n.securityCustodyBestEffort), findsOneWidget);
      expect(_info(l10n.securityCustodyHardwareKey), findsNothing);
    },
  );

  testWidgets('a probe failure is honest, never a silent "protected"', (
    tester,
  ) async {
    await tester.pumpWidget(
      _harness(AsyncError(StateError('locked'), StackTrace.current)),
    );
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    expect(find.text(l10n.securityCustodyProbeError), findsOneWidget);
    expect(_info(l10n.securityCustodyHardwareKey), findsNothing);
  });

  testWidgets(
    'an unrecognised (newer-core) tier falls back to Unknown + best-effort',
    (tester) async {
      // Forward-compat: a tier label the screen does not know maps to "Unknown",
      // and (since the core is honest about eraseAssurance) reads best-effort.
      const unknownTier = CustodyDisclosure(
        tier: 'quantum_vault_9000',
        eraseAssurance: EraseAssurance.bestEffort,
        degraded: false,
      );
      await tester.pumpWidget(_harness(const AsyncData(unknownTier)));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      expect(find.text(l10n.securityCustodyTierUnknown), findsOneWidget);
      expect(_info(l10n.securityCustodyBestEffort), findsOneWidget);
      expect(_info(l10n.securityCustodyHardwareKey), findsNothing);
    },
  );

  testWidgets(
    'a software keystore (degraded:true) still branches on eraseAssurance',
    (tester) async {
      // degraded:true must NOT change the erase honesty — it keys off
      // eraseAssurance only (a degraded keystore is still best-effort).
      const software = CustodyDisclosure(
        tier: 'software_keystore',
        eraseAssurance: EraseAssurance.bestEffort,
        degraded: true,
      );
      await tester.pumpWidget(_harness(const AsyncData(software)));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      expect(find.text(l10n.securityCustodyTierSoftware), findsOneWidget);
      expect(_info(l10n.securityCustodyBestEffort), findsOneWidget);
      expect(_info(l10n.securityCustodyHardwareKey), findsNothing);
    },
  );

  testWidgets(
    'Delete confirmation RESTATES the per-tier custody line; cancel dismisses',
    (tester) async {
      await tester.pumpWidget(_harness(const AsyncData(_hardwareKey)));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      // The destructive button is present and labelled.
      expect(find.text(l10n.securityDeleteWalletButton), findsOneWidget);

      // Scroll it into view first: the #397 export-viewing-key tile grew the
      // screen, so the delete button sits below the fold on the test viewport.
      await tester.ensureVisible(find.text(l10n.securityDeleteWalletButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.securityDeleteWalletButton));
      await tester.pumpAndSettle();

      // The confirmation must warn BEFORE anything destructive runs — and (FR-14 H1
      // honesty) RESTATE the tier's custody line inside the dialog itself,
      // not only on the card the user may not have read.
      expect(find.text(l10n.securityDeleteDialogTitle), findsOneWidget);
      expect(
        find.descendant(
          of: find.byType(AlertDialog),
          matching: find.textContaining(l10n.securityDeleteDialogBody),
        ),
        findsOneWidget,
      );
      expect(
        find.descendant(
          of: find.byType(AlertDialog),
          matching: find.textContaining(l10n.securityCustodyHardwareKey),
        ),
        findsOneWidget,
        reason: 'the dialog restates the tier-specific custody line',
      );

      // S11 C3: deleting the wallet is a DESTRUCTIVE confirm — a red text
      // button, never the filled button a forward step gets (§6.7).
      final confirmLabel = find.descendant(
        of: find.byType(AlertDialog),
        matching: find.text(l10n.securityDeleteDialogConfirm),
      );
      expect(
        find.ancestor(of: confirmLabel, matching: find.byType(FilledButton)),
        findsNothing,
        reason: 'a destructive confirm is never a filled button',
      );
      final confirmButton = tester.widget<TextButton>(
        find.ancestor(of: confirmLabel, matching: find.byType(TextButton)),
      );
      expect(
        confirmButton.style?.foregroundColor?.resolve(<WidgetState>{}),
        lightTheme.extension<WalletColors>()!.red,
      );

      // Cancel backs out with nothing wiped (the screen is still shown).
      await tester.tap(find.text(l10n.securityDeleteDialogCancel));
      await tester.pumpAndSettle();
      expect(find.text(l10n.securityDeleteDialogTitle), findsNothing);
      expect(find.byType(SecurityScreen), findsOneWidget);
    },
  );

  testWidgets('S11 C3 (the review\'s M4): at 2.0x on a 320 dp phone the delete '
      'dialog scrolls — the whole warning and BOTH actions are reachable', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(320, 568);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          walletProvisionerProvider.overrideWithValue(FakeWalletProvisioner()),
          walletCustodyDisclosureProvider.overrideWith(
            (ref) => Future<CustodyDisclosure>.value(_bestEffortNone),
          ),
        ],
        child: MaterialApp(
          theme: lightTheme,
          localizationsDelegates: WalletLocalizations.localizationsDelegates,
          supportedLocales: WalletLocalizations.supportedLocales,
          builder: (context, app) => MediaQuery.withClampedTextScaling(
            minScaleFactor: 2.0,
            maxScaleFactor: 2.0,
            child: app!,
          ),
          home: const SecurityScreen(),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);
    // The screen is a lazy list: at this size the delete button is past the
    // first build range, so scroll it INTO the tree.
    await tester.scrollUntilVisible(
      find.text(l10n.securityDeleteWalletButton),
      200,
      scrollable: find.byType(Scrollable).first,
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.securityDeleteWalletButton));
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    expectDialogScrollsToBothActions(
      tester,
      bodyStart: l10n.securityDeleteDialogBody,
      cancel: l10n.securityDeleteDialogCancel,
      confirm: l10n.securityDeleteDialogConfirm,
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
      bodyStart: l10n.securityDeleteDialogBody,
      cancel: l10n.securityDeleteDialogCancel,
      confirm: l10n.securityDeleteDialogConfirm,
      scrolledToEnd: true,
    );
    // The safe action takes the tap from there.
    await tester.tap(find.text(l10n.securityDeleteDialogCancel));
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsNothing);
  });

  testWidgets('#397 P2 — a WATCH-ONLY wallet delete copy drops the '
      'recovery-phrase promise (it has none) — subtitle AND dialog body say '
      're-import from the viewing key; the tier erase line still shows', (
    tester,
  ) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          walletProvisionerProvider.overrideWithValue(FakeWalletProvisioner()),
          walletCustodyDisclosureProvider.overrideWith(
            (ref) => Future<CustodyDisclosure>.value(_hardwareKey),
          ),
          isWatchOnlyProvider.overrideWithValue(true),
        ],
        child: MaterialApp(
          theme: lightTheme,
          localizationsDelegates: WalletLocalizations.localizationsDelegates,
          supportedLocales: WalletLocalizations.supportedLocales,
          home: const SecurityScreen(),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    // Subtitle: the watch-only variant, never the recovery-phrase one.
    expect(_info(l10n.securityDeleteWalletSubtitleWatchOnly), findsOneWidget);
    expect(_info(l10n.securityDeleteWalletSubtitle), findsNothing);

    await tester.ensureVisible(find.text(l10n.securityDeleteWalletButton));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.securityDeleteWalletButton));
    await tester.pumpAndSettle();

    // Dialog body: the watch-only variant, and NOT the recovery-phrase one —
    // but the tier erase line still appends (the DB wrap key is shredded
    // for both kinds).
    expect(
      find.descendant(
        of: find.byType(AlertDialog),
        matching: find.textContaining(l10n.securityDeleteDialogBodyWatchOnly),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: find.byType(AlertDialog),
        matching: find.textContaining(l10n.securityCustodyHardwareKey),
      ),
      findsOneWidget,
    );
  });

  testWidgets(
    '#317: the screen BACK-LOCKS while a delete is in flight (PopScope), then '
    'releases when the shred resolves — a mid-delete exit can never strand '
    'the outcome routing',
    (tester) async {
      // A real onboarding controller booted to Active (a confirmed wallet) so
      // the delete path actually runs, with the shred HELD in flight and then
      // faulting into recover-by-reopen (stays on THIS screen — no navigation,
      // so no go_router needed).
      final hold = Completer<void>();
      final provisioner = FakeWalletProvisioner(exists: true)
        ..holdDelete = hold
        ..failDelete = const WalletApiError(
          code: 'RW-KEYSTORE',
          message: 'wipe faulted',
          kind: WalletErrorKind.keystoreUnavailable(),
        );
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            walletProvisionerProvider.overrideWithValue(provisioner),
            onboardingStoreProvider.overrideWithValue(
              FakeOnboardingStore(confirmed: true),
            ),
            walletCustodyDisclosureProvider.overrideWith(
              (ref) => Future<CustodyDisclosure>.value(_hardwareKey),
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
      // Boot the onboarding controller to Active (a confirmed wallet) — the
      // SecurityScreen only reads `.notifier` at delete time, so without a
      // live watcher the boot probe never runs and deleteWallet would no-op
      // `notDeletable`.
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
      final l10n = _l10n(tester);

      PopScope popScope() => tester.widget<PopScope>(
        find.ancestor(
          of: find.byType(Scaffold),
          matching: find.byType(PopScope),
        ),
      );
      expect(popScope().canPop, isTrue, reason: 'idle: back is allowed');

      // Confirm the delete → the shred is now held in flight. (Scroll the
      // button into view — the #397 export tile pushed it below the fold.)
      await tester.ensureVisible(find.text(l10n.securityDeleteWalletButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.securityDeleteWalletButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.securityDeleteDialogConfirm));
      await tester.pump(); // enter the in-flight (_deleting) state

      expect(
        popScope().canPop,
        isFalse,
        reason: 'mid-shred: the system back gesture is locked out',
      );

      // Resolve the (faulting) shred → recover-by-reopen keeps the screen; the
      // lock releases.
      hold.complete();
      await tester.pumpAndSettle();
      expect(popScope().canPop, isTrue, reason: 'resolved: back allowed again');
      expect(find.byType(SecurityScreen), findsOneWidget);
    },
  );

  testWidgets(
    'SESSION-ONLY host (no package provisioner): honest unavailable body, '
    'NO delete button, NO custody card (S151 swap-in seam)',
    (tester) async {
      // A host with its own custody overrides walletSessionProvider and leaves
      // walletProvisionerProvider unwired. The overflow-menu entry hides there,
      // but a host-mounted deep link can still land here — the screen must
      // re-gate itself: a delete button that silently no-ops (deleteWallet →
      // notDeletable) would be a broken promise on a destructive action.
      await tester.pumpWidget(
        ProviderScope(
          // No provisioner override — the seam's production default (null).
          child: MaterialApp(
            theme: lightTheme,
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            home: const SecurityScreen(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      expect(find.text(l10n.securityUnavailableBody), findsOneWidget);
      expect(find.text(l10n.securityDeleteWalletButton), findsNothing);
      expect(find.text(l10n.securityCustodySectionTitle), findsNothing);
    },
  );

  testWidgets(
    'double-tapping the backup tile pushes only ONE backup screen (UX guard; '
    'the FLAG_SECURE stacked-twin race is now handled by the refcounted port, '
    '#344)',
    (tester) async {
      final router = GoRouter(
        initialLocation: WalletRoutes.security,
        routes: [
          GoRoute(
            path: WalletRoutes.security,
            builder: (context, state) => const SecurityScreen(),
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
            walletProvisionerProvider.overrideWithValue(
              FakeWalletProvisioner(),
            ),
            walletCustodyDisclosureProvider.overrideWith(
              (ref) => Future<CustodyDisclosure>.value(_hardwareKey),
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
      final l10n = _l10n(tester);
      final tile = find.text(l10n.walletBackupTileTitle);

      // Two taps in one frame (no settle between) — the synchronous guard must
      // collapse them so exactly one BackupScreen is on the stack. Count
      // offstage too: a stacked twin would sit offstage under the top one.
      await tester.tap(tile);
      await tester.tap(tile, warnIfMissed: false);
      await tester.pumpAndSettle();

      expect(find.byType(BackupScreen, skipOffstage: false), findsOneWidget);
    },
  );
}
