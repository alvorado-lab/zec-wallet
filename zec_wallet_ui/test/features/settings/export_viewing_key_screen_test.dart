import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/core/lifecycle/app_lifecycle_provider.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/settings/export_viewing_key_screen.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/reveal_authorization.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart'
    show isWatchOnlyProvider, walletSessionProvider;
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/testing.dart';

import '../../support/a11y_activation.dart';

/// #397 §3.7 D5 — the UFVK export screen. The threat model mirrors the backup
/// screen (a Settings reveal on an already-unlocked device): the key stays
/// sealed until a deliberate reveal that PASSES re-auth; a denied re-auth never
/// reads it; the D9 warning is shown BEFORE the reveal; and — the divergence
/// from the seed — once revealed the key IS copyable + QR-able (it is public).

WalletLocalizations _l10n(WidgetTester tester) =>
    WalletLocalizations.of(tester.element(find.byType(ExportViewingKeyScreen)));

/// The fake's export payload — a recognizable "the key is on screen" probe.
const _exportedUfvk = 'uview1testexportedviewingkeyprobexxxxxxxxxxxxxxxxxxxx';

Future<ProviderContainer> _pump(
  WidgetTester tester, {
  FakeWalletProvisioner? provisioner,
  bool provisionerNull = false,
  FakeScreenSecurity? security,
  FakeRevealAuthorizer? authorizer,
  List<ProviderObserver> observers = const [],
}) async {
  final container = ProviderContainer(
    observers: observers,
    overrides: [
      walletProvisionerProvider.overrideWithValue(
        provisionerNull ? null : (provisioner ?? _provisioner()),
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
        home: const ExportViewingKeyScreen(),
      ),
    ),
  );
  await tester.pumpAndSettle();
  return container;
}

FakeWalletProvisioner _provisioner() =>
    FakeWalletProvisioner()..exportedUfvk = _exportedUfvk;

/// Logs every provider value as text, the way a host's debug observer would.
final class _ValueLog extends ProviderObserver {
  final lines = <String>[];

  @override
  void didAddProvider(ProviderObserverContext context, Object? value) =>
      lines.add('$value');

  @override
  void didUpdateProvider(
    ProviderObserverContext context,
    Object? previousValue,
    Object? newValue,
  ) => lines.add('$newValue');
}

Future<void> _tapReveal(WidgetTester tester) async {
  final reveal = find.text(_l10n(tester).walletExportViewingKeyReveal);
  await tester.ensureVisible(reveal);
  await tester.pumpAndSettle();
  await tester.tap(reveal);
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('the key stays sealed until a deliberate reveal that passes '
      're-auth — and the D9 warning shows first', (tester) async {
    final provisioner = _provisioner();
    await _pump(tester, provisioner: provisioner);
    final l10n = _l10n(tester);

    // Pre-reveal: the warning is shown, the key is NOT, and nothing is read.
    expect(find.text(l10n.walletExportViewingKeyWarning), findsOneWidget);
    expect(find.text(_exportedUfvk), findsNothing);
    expect(find.text(l10n.walletExportViewingKeyReveal), findsOneWidget);
    expect(provisioner.exportUfvkCount, 0);

    await _tapReveal(tester);

    // Post-reveal: the key renders (copyable), the reveal button is gone, and
    // the warning STILL shows (the consequence stays in view while sharing).
    expect(find.text(_exportedUfvk), findsOneWidget);
    expect(find.text(l10n.walletExportViewingKeyReveal), findsNothing);
    expect(find.text(l10n.walletExportViewingKeyWarning), findsOneWidget);
    expect(provisioner.exportUfvkCount, 1);
  });

  testWidgets('a host observer that logs every provider value never records '
      'the viewing key, while the screen still shows it', (tester) async {
    // A value-logging `ProviderObserver` is a common host debug aid. The key
    // reaches provider state in a redacting box, so the log holds the box.
    final log = _ValueLog();
    await _pump(tester, observers: [log]);
    await _tapReveal(tester);

    expect(find.text(_exportedUfvk), findsOneWidget);
    expect(log.lines.join('\n'), contains('RedactedViewingKey(redacted)'));
    expect(
      log.lines.where((l) => l.contains(_exportedUfvk.substring(6, 30))),
      isEmpty,
      reason: 'no logged provider value carries any part of the key',
    );
  });

  testWidgets('#397 P2 — a WATCH-ONLY wallet shows the D9 warning VARIANT: no '
      '"move your funds to a new wallet" (unfollowable — it cannot spend), the '
      'honest "once shared it cannot be un-shared" instead', (tester) async {
    final container = ProviderContainer(
      overrides: [
        walletProvisionerProvider.overrideWithValue(_provisioner()),
        screenSecurityProvider.overrideWithValue(FakeScreenSecurity()),
        walletRevealAuthorizerProvider.overrideWithValue(
          FakeRevealAuthorizer(),
        ),
        appLifecycleProvider.overrideWith(TestLifecycleNotifier.new),
        isWatchOnlyProvider.overrideWithValue(true),
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
          home: const ExportViewingKeyScreen(),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    expect(
      find.text(l10n.walletExportViewingKeyWarningWatchOnly),
      findsOneWidget,
    );
    expect(find.text(l10n.walletExportViewingKeyWarning), findsNothing);
  });

  testWidgets('#397 P3 UX-L2 — the D9 warning is NOT a live region: it is '
      'screen-entry content read in traversal order, and must not re-announce '
      'over the reveal/screen-security rebuilds', (tester) async {
    final handle = tester.ensureSemantics();
    final container = ProviderContainer(
      overrides: [
        walletProvisionerProvider.overrideWithValue(_provisioner()),
        screenSecurityProvider.overrideWithValue(FakeScreenSecurity()),
        walletRevealAuthorizerProvider.overrideWithValue(
          FakeRevealAuthorizer(),
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
          home: const ExportViewingKeyScreen(),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final warning = tester.getSemantics(
      find.byKey(const Key('export-ufvk-warning')),
    );
    expect(warning.flagsCollection.isLiveRegion, isFalse);
    handle.dispose();
  });

  testWidgets('a DENIED re-auth never reads the viewing key', (tester) async {
    final provisioner = _provisioner();
    final authorizer = FakeRevealAuthorizer(deny: true);
    await _pump(tester, provisioner: provisioner, authorizer: authorizer);
    final l10n = _l10n(tester);

    await _tapReveal(tester);

    expect(find.text(_exportedUfvk), findsNothing);
    expect(find.text(l10n.walletExportViewingKeyReveal), findsOneWidget);
    expect(authorizer.calls, 1);
    // The point: a denied re-auth NEVER reads the key from the store.
    expect(provisioner.exportUfvkCount, 0);
  });

  testWidgets(
    'the revealed key is copyable (the divergence from the seed grid)',
    (tester) async {
      // A UFVK is public once shared, so it goes to the clipboard on Copy — a
      // seed never does. Intercept the platform clipboard channel and assert.
      String? copied;
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async {
          if (call.method == 'Clipboard.setData') {
            copied = (call.arguments as Map)['text'] as String?;
          }
          return null;
        },
      );
      addTearDown(
        () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          null,
        ),
      );

      await _pump(tester);
      final l10n = _l10n(tester);
      await _tapReveal(tester);

      // The QR tile encodes the EXACT key (a dropped char would send a watcher
      // to the wrong wallet).
      expect(
        find.byKey(const Key('export-ufvk-qr')),
        findsOneWidget,
        reason: 'a UFVK is public — QR is correct here (unlike the seed)',
      );
      // The QR encodes the key VERBATIM (lowercase) — identical to the
      // displayed + copied text (the copy assert below pins the copy). QrTile
      // keys its inner code by the exact payload. (briefly uppercased this
      // for QR density; reverted — the pinned qr encoder is byte-mode only,
      // so uppercasing gave zero density gain for a third-party-scanner interop
      // risk. QR, display, and copy all carry the one canonical form.)
      expect(
        find.byKey(ValueKey<String>(_exportedUfvk)),
        findsOneWidget,
        reason: 'QR payload equals the (lowercase) displayed/copied key',
      );
      expect(
        find.byKey(ValueKey<String>(_exportedUfvk.toUpperCase())),
        findsNothing,
        reason: 'the QR must NOT ship a non-canonical uppercased key',
      );
      await tester.ensureVisible(find.text(l10n.walletExportViewingKeyCopy));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletExportViewingKeyCopy));
      await tester.pumpAndSettle();
      expect(copied, _exportedUfvk);
      expect(find.text(l10n.walletExportViewingKeyCopied), findsOneWidget);
    },
  );

  // The row above drives Copy with a POINTER event, which hits the live
  // FilledButton underneath and never consults the semantics tree — so it was
  // green while `excludeSemantics: true` dropped the button's own tap action
  // and left a screen reader with an unactivatable button over the one
  // sanctioned UFVK egress. Contract: `test/support/a11y_activation.dart`.
  testWidgets(
    'the revealed key\'s Copy is ACTIVATABLE by a screen reader (the semantics '
    'action, not a pointer, puts the key on the clipboard)',
    (tester) async {
      final handle = tester.ensureSemantics();
      String? copied;
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async {
          if (call.method == 'Clipboard.setData') {
            copied = (call.arguments as Map)['text'] as String?;
          }
          return null;
        },
      );
      addTearDown(
        () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          null,
        ),
      );

      await _pump(tester);
      final l10n = _l10n(tester);
      await _tapReveal(tester);

      expectActivatable(
        tester,
        find.semantics.byLabel(l10n.walletExportViewingKeyCopy),
        reason: 'D5: the ONE sanctioned UFVK egress in the UI',
      );
      await tester.pumpAndSettle();
      expect(copied, _exportedUfvk);
      expect(find.text(l10n.walletExportViewingKeyCopied), findsOneWidget);
      handle.dispose();
    },
  );

  testWidgets('a session-only mount (no provisioner) shows the honest '
      'managed-by-host state, no reveal dead-end', (tester) async {
    await _pump(tester, provisionerNull: true);
    final l10n = _l10n(tester);

    expect(find.text(l10n.walletBackupManagedTitle), findsOneWidget);
    expect(find.text(l10n.walletExportViewingKeyReveal), findsNothing);
  });

  testWidgets('a failed export shows an honest retry that re-reads the key', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    final provisioner = _provisioner()
      ..failExportUfvk = const WalletApiError(
        code: 'RW-STORE-005',
        message: 'store busy',
        kind: WalletErrorKind.storeCorrupt(),
      );
    await _pump(tester, provisioner: provisioner);
    final l10n = _l10n(tester);

    await _tapReveal(tester);

    expect(find.text(l10n.walletExportViewingKeyFailed), findsOneWidget);
    expect(find.text(_exportedUfvk), findsNothing);
    expect(provisioner.exportUfvkCount, 1);
    // #397 P3 (review F1): the fault APPEARS after the async reveal round-trip
    // — it must be a live region so a screen reader hears it.
    expect(
      tester
          .getSemantics(find.text(l10n.walletExportViewingKeyFailed))
          .flagsCollection
          .isLiveRegion,
      isTrue,
    );

    // Clear the fault and retry — the key reads on the second attempt.
    provisioner.failExportUfvk = null;
    await tester.tap(find.text(l10n.walletExportViewingKeyRetry));
    await tester.pumpAndSettle();
    expect(find.text(_exportedUfvk), findsOneWidget);
    expect(provisioner.exportUfvkCount, 2);
    handle.dispose();
  });

  // RED BY RULING — the 2026-09-20 production-readiness review, R06
  // (docs/plan/audit-2026-09-20-remediation.md §2a/§2c): the body is the
  // review's probe (docs/reviews/2026-09-20/probes/ufvk.dart), verbatim; the
  // name is this project's; evals/standing_reds.txt carries it. Today the
  // reveal authorization survives a wallet identity switch and wallet B's key
  // is exported and shown under wallet A's authorization. Green only with the
  // authorization bound to `walletIdentityProvider` and every pending reveal
  // invalidated on identity change (§2c R06). Appended at the end so no cited
  // line above moves.
  testWidgets(
    'R06 — a wallet identity switch invalidates the standing reveal authorization',
    (tester) async {
      const keyB = 'uview1walletbreviewprobexxxxxxxxxxxxxxxxxxxxxxxxxxxx';
      final sessionA = FakeWalletSession()..exportUfvkResult = _exportedUfvk;
      final sessionB = FakeWalletSession()..exportUfvkResult = keyB;
      var currentSession = sessionA;
      final authorizer = FakeRevealAuthorizer();
      final container = ProviderContainer(
        overrides: [
          walletProvisionerProvider.overrideWithValue(null),
          walletSessionProvider.overrideWith((ref) => currentSession),
          isWatchOnlyProvider.overrideWithValue(false),
          screenSecurityProvider.overrideWithValue(FakeScreenSecurity()),
          walletRevealAuthorizerProvider.overrideWithValue(authorizer),
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
            home: const ExportViewingKeyScreen(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await _tapReveal(tester);
      expect(authorizer.calls, 1);
      expect(sessionA.exportUfvkCount, 1);
      expect(sessionB.exportUfvkCount, 0);
      expect(find.text(_exportedUfvk), findsOneWidget);
      final screenState = tester.state(find.byType(ExportViewingKeyScreen));
      currentSession = sessionB;
      container.invalidate(walletSessionProvider);
      await tester.pumpAndSettle();
      expect(
        identical(
          screenState,
          tester.state(find.byType(ExportViewingKeyScreen)),
        ),
        isTrue,
      );
      expect(
        {
          'auth': authorizer.calls,
          'exportsA': sessionA.exportUfvkCount,
          'exportsB': sessionB.exportUfvkCount,
          'keyBVisible': find.text(keyB).evaluate().length,
        },
        {'auth': 1, 'exportsA': 1, 'exportsB': 0, 'keyBVisible': 0},
        reason: 'A reveal authorization must not export another wallet’s key.',
      );
    },
  );
}
