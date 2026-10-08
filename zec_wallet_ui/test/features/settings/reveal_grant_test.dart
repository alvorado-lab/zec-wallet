import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/core/lifecycle/app_lifecycle_provider.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/settings/backup_screen.dart';
import 'package:zec_wallet_ui/features/settings/export_viewing_key_screen.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_controller.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_state.dart';
import 'package:zec_wallet_ui/features/wallet/reveal_authorization.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart'
    show isWatchOnlyProvider, walletIdentityProvider, walletSessionProvider;
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/testing.dart';

/// Stage S2 `switch` (docs/plan/stage-2-the-hosts-lifecycle.md §3.4): a reveal
/// authorization is a fact about ONE wallet session instance, ONE operation
/// generation and ONE foreground session. If the wallet under a mounted reveal
/// screen changes — another wallet, or the same wallet on a new handle — the
/// standing authorization is void: nothing is read or shown without a fresh
/// prompt, and a prompt answered after the change lands nowhere.
///
/// The two consumers are the export-viewing-key screen and the backup screen.
/// The identity change is driven two ways: a session-only host swapping the
/// session it supplies (the universal host's account switch), and the package
/// gate's own server switch (same wallet, same identity epoch, new session).

const _keyA = 'uview1walletarevealgrantprobexxxxxxxxxxxxxxxxxxxxxxxx';
const _keyB = 'uview1walletbrevealgrantprobexxxxxxxxxxxxxxxxxxxxxxxx';

/// The fake's default reveal payload ends in this unique 24th word.
const _uniqueWord = 'art';

const _other = SyncServerChoice.predefined(id: 'example-gated');

/// A provisioner whose viewing-key export parks until the test answers it —
/// so a switch can land between the read and the render.
class _HeldExportProvisioner extends FakeWalletProvisioner {
  _HeldExportProvisioner({super.exists});

  Completer<String>? heldExport;

  @override
  Future<String> exportUfvk() {
    final held = heldExport;
    if (held == null) return super.exportUfvk();
    exportUfvkCount++;
    return held.future;
  }
}

Widget _app(ProviderContainer container, Widget home) =>
    UncontrolledProviderScope(
      container: container,
      child: MaterialApp(
        theme: lightTheme,
        localizationsDelegates: WalletLocalizations.localizationsDelegates,
        supportedLocales: WalletLocalizations.supportedLocales,
        home: home,
      ),
    );

/// The package gate, booted to an Active wallet, with [home] mounted.
Future<ProviderContainer> _pumpActive(
  WidgetTester tester, {
  required FakeWalletProvisioner provisioner,
  required FakeRevealAuthorizer authorizer,
  required Widget home,
}) async {
  final container = ProviderContainer(
    overrides: [
      walletProvisionerProvider.overrideWithValue(provisioner),
      onboardingStoreProvider.overrideWithValue(
        FakeOnboardingStore(confirmed: true),
      ),
      screenSecurityProvider.overrideWithValue(FakeScreenSecurity()),
      walletRevealAuthorizerProvider.overrideWithValue(authorizer),
      appLifecycleProvider.overrideWith(TestLifecycleNotifier.new),
    ],
  );
  addTearDown(container.dispose);
  container.listen(onboardingControllerProvider, (_, _) {});
  container.listen(walletSessionProvider, (_, _) {});
  await tester.pumpWidget(_app(container, home));
  await tester.pumpAndSettle();
  expect(
    container.read(onboardingControllerProvider),
    isA<OnboardingActive>(),
    reason: 'precondition: an active wallet under the screen',
  );
  return container;
}

/// Switch the package gate's server: same wallet, same identity epoch, a new
/// session instance in the gate.
/// [settle] is false while a spinner is up (a parked prompt or read), which
/// never settles.
Future<void> _switchServer(
  WidgetTester tester,
  ProviderContainer container, {
  bool settle = true,
}) async {
  final before = container.read(walletSessionProvider);
  final epochBefore = container.read(walletIdentityProvider);
  final result = await container
      .read(onboardingControllerProvider.notifier)
      .switchSyncServer(_other);
  if (settle) {
    await tester.pumpAndSettle();
  } else {
    await tester.pump();
    await tester.pump();
  }
  expect(result, isA<SwitchServerSuccess>(), reason: 'precondition');
  expect(
    container.read(walletSessionProvider),
    isNot(same(before)),
    reason: 'precondition: the session instance was replaced',
  );
  expect(
    container.read(walletIdentityProvider),
    epochBefore,
    reason: 'precondition: the identity VALUE is unchanged (same wallet)',
  );
}

WalletLocalizations _l10nOf(WidgetTester tester, Type screen) =>
    WalletLocalizations.of(tester.element(find.byType(screen)));

Future<void> _tap(
  WidgetTester tester,
  String label, {
  bool settle = true,
}) async {
  final target = find.text(label);
  await tester.ensureVisible(target);
  await tester.pumpAndSettle();
  await tester.tap(target);
  if (settle) {
    await tester.pumpAndSettle();
  } else {
    await tester.pump();
  }
}

void main() {
  testWidgets(
    'a reveal authorized before an identity switch completes as void',
    (tester) async {
      // A session-only host: the session it supplies IS the wallet identity.
      final sessionA = FakeWalletSession()..exportUfvkResult = _keyA;
      final sessionB = FakeWalletSession()..exportUfvkResult = _keyB;
      WalletSession current = sessionA;
      final prompt = Completer<void>();
      final authorizer = FakeRevealAuthorizer(gate: prompt);
      final container = ProviderContainer(
        overrides: [
          walletProvisionerProvider.overrideWithValue(null),
          walletSessionProvider.overrideWith((ref) => current),
          isWatchOnlyProvider.overrideWithValue(false),
          screenSecurityProvider.overrideWithValue(FakeScreenSecurity()),
          walletRevealAuthorizerProvider.overrideWithValue(authorizer),
          appLifecycleProvider.overrideWith(TestLifecycleNotifier.new),
        ],
      );
      addTearDown(container.dispose);
      await tester.pumpWidget(_app(container, const ExportViewingKeyScreen()));
      await tester.pumpAndSettle();
      final l10n = _l10nOf(tester, ExportViewingKeyScreen);

      // The prompt is up for wallet A…
      await _tap(tester, l10n.walletExportViewingKeyReveal, settle: false);
      expect(authorizer.calls, 1);

      // …the wallet under the screen changes while it is up…
      current = sessionB;
      container.invalidate(walletSessionProvider);
      await tester.pump();

      // …and the user answers it.
      prompt.complete();
      await tester.pumpAndSettle();

      expect(
        {
          'exportsA': sessionA.exportUfvkCount,
          'exportsB': sessionB.exportUfvkCount,
          'keyAVisible': find.text(_keyA).evaluate().length,
          'keyBVisible': find.text(_keyB).evaluate().length,
        },
        {'exportsA': 0, 'exportsB': 0, 'keyAVisible': 0, 'keyBVisible': 0},
        reason: 'a completion for a wallet no longer on screen reads nothing',
      );
      // The screen shows the new wallet's un-revealed state.
      expect(find.text(l10n.walletExportViewingKeyReveal), findsOneWidget);

      // B is revealable — with its OWN prompt.
      await _tap(tester, l10n.walletExportViewingKeyReveal);
      expect(authorizer.calls, 2);
      expect(sessionB.exportUfvkCount, 1);
      expect(find.text(_keyB), findsOneWidget);
    },
  );

  testWidgets(
    'a replaced session instance for the same wallet voids the standing '
    'authorization',
    (tester) async {
      final provisioner = FakeWalletProvisioner(exists: true)
        ..exportedUfvk = _keyA;
      final authorizer = FakeRevealAuthorizer();
      final container = await _pumpActive(
        tester,
        provisioner: provisioner,
        authorizer: authorizer,
        home: const ExportViewingKeyScreen(),
      );
      final l10n = _l10nOf(tester, ExportViewingKeyScreen);

      await _tap(tester, l10n.walletExportViewingKeyReveal);
      expect(authorizer.calls, 1);
      expect(find.text(_keyA), findsOneWidget);
      final screenState = tester.state(find.byType(ExportViewingKeyScreen));

      await _switchServer(tester, container);

      expect(
        identical(
          screenState,
          tester.state(find.byType(ExportViewingKeyScreen)),
        ),
        isTrue,
        reason: 'the screen stayed mounted across the switch',
      );
      expect(
        find.text(_keyA),
        findsNothing,
        reason: 'the grant was for the replaced session instance',
      );
      expect(find.text(l10n.walletExportViewingKeyReveal), findsOneWidget);
      expect(provisioner.exportUfvkCount, 1, reason: 'nothing re-read');

      // Showing it again costs one re-prompt — the safe side.
      await _tap(tester, l10n.walletExportViewingKeyReveal);
      expect(authorizer.calls, 2);
      expect(find.text(_keyA), findsOneWidget);
    },
  );

  testWidgets("the backup screen's reveal is bound to the same grant", (
    tester,
  ) async {
    final provisioner = FakeWalletProvisioner(exists: true);
    final prompt = Completer<void>();
    final authorizer = FakeRevealAuthorizer();
    final container = await _pumpActive(
      tester,
      provisioner: provisioner,
      authorizer: authorizer,
      home: const BackupScreen(),
    );
    final l10n = _l10nOf(tester, BackupScreen);

    // A standing reveal is voided by a new session instance under the screen.
    await _tap(tester, l10n.walletBackupReveal);
    expect(find.text(_uniqueWord), findsOneWidget);
    expect(provisioner.revealCount, 1);

    await _switchServer(tester, container);

    expect(find.text(_uniqueWord), findsNothing);
    expect(find.text(l10n.walletBackupReveal), findsOneWidget);
    expect(provisioner.revealCount, 1, reason: 'no unseal without a prompt');

    // A prompt answered after the session changed reads nothing.
    authorizer.gate = prompt;
    await _tap(tester, l10n.walletBackupReveal, settle: false);
    expect(authorizer.calls, 2);
    await _switchServer(tester, container, settle: false);
    prompt.complete();
    await tester.pumpAndSettle();

    expect(
      provisioner.revealCount,
      1,
      reason: 'the late grant unsealed nothing',
    );
    expect(find.text(_uniqueWord), findsNothing);
    expect(find.text(l10n.walletBackupReveal), findsOneWidget);

    // A fresh prompt on the current session reveals.
    authorizer.gate = null;
    await _tap(tester, l10n.walletBackupReveal);
    expect(authorizer.calls, 3);
    expect(provisioner.revealCount, 2);
    expect(find.text(_uniqueWord), findsOneWidget);
  });

  testWidgets(
    'a backgrounded reveal hides and re-authorizes on resume (pinned)',
    (tester) async {
      // The export screen's twin of backup_screen_test's two background rows.
      final provisioner = FakeWalletProvisioner(exists: true)
        ..exportedUfvk = _keyA;
      final authorizer = FakeRevealAuthorizer();
      final container = await _pumpActive(
        tester,
        provisioner: provisioner,
        authorizer: authorizer,
        home: const ExportViewingKeyScreen(),
      );
      final l10n = _l10nOf(tester, ExportViewingKeyScreen);
      final lifecycle =
          container.read(appLifecycleProvider.notifier)
              as TestLifecycleNotifier;

      await _tap(tester, l10n.walletExportViewingKeyReveal);
      expect(find.text(_keyA), findsOneWidget);
      expect(authorizer.calls, 1);

      for (final background in [
        AppLifecycleState.hidden,
        AppLifecycleState.paused,
      ]) {
        lifecycle.emit(background);
        await tester.pumpAndSettle();
        expect(find.text(_keyA), findsNothing, reason: '$background hides');
        lifecycle.emit(AppLifecycleState.resumed);
        await tester.pumpAndSettle();
        expect(
          find.text(_keyA),
          findsNothing,
          reason: 'a grant does not outlive the foreground session',
        );
        final callsBefore = authorizer.calls;
        await _tap(tester, l10n.walletExportViewingKeyReveal);
        expect(authorizer.calls, callsBefore + 1, reason: 're-authorized');
        expect(find.text(_keyA), findsOneWidget);
      }
    },
  );

  // Not named by §3.4's assertions: the SECOND check (before the render). The
  // grant was valid when the read began; the session is replaced while the read
  // is in flight; the answer must not render.
  testWidgets('a reveal read that answers after a switch is not rendered', (
    tester,
  ) async {
    final held = Completer<String>();
    final provisioner = _HeldExportProvisioner(exists: true)..heldExport = held;
    final authorizer = FakeRevealAuthorizer();
    final container = await _pumpActive(
      tester,
      provisioner: provisioner,
      authorizer: authorizer,
      home: const ExportViewingKeyScreen(),
    );
    final l10n = _l10nOf(tester, ExportViewingKeyScreen);

    await _tap(tester, l10n.walletExportViewingKeyReveal, settle: false);
    await tester.pump();
    expect(authorizer.calls, 1);
    expect(provisioner.exportUfvkCount, 1, reason: 'the read is in flight');

    await _switchServer(tester, container, settle: false);
    held.complete(_keyA);
    await tester.pumpAndSettle();

    expect(find.text(_keyA), findsNothing);
    expect(find.text(l10n.walletExportViewingKeyReveal), findsOneWidget);
  });
}
