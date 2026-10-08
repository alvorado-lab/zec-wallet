import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/sync_server_sheet.dart';
import 'package:zec_wallet_ui/features/wallet/sync_status_sheet.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations_en.dart';
import 'package:zec_wallet_ui/testing.dart';

/// The sync-server picker (`docs/specs/sync-server-picker.md` §8 gate 2):
/// every state the sheet defines, driven through the fake session (the probe)
/// and the fake provisioner (the switch), on the host VM.

const _zecRocks = SyncServer(
  id: 'zec-rocks',
  label: 'zec.rocks',
  url: 'https://zec.rocks:443',
  authHeader: null,
  authValue: null,
);
const _gated = SyncServer(
  id: 'example-gated',
  label: 'Example gated',
  url: 'https://lightwalletd.example.com:443',
  authHeader: 'x-zcash-rpc-key',
  authValue: null,
);

SyncServerStatus _status({
  String effective = 'https://zec.rocks:443',
  String defaultUrl = 'https://zec.rocks:443',
  SyncServerChoice? choice,
  SyncServerFallback? fallback,
}) => SyncServerStatus(
  effectiveUrl: effective,
  defaultUrl: defaultUrl,
  choice: choice,
  fallback: fallback,
);

WalletApiError _refusal(WalletErrorKind kind, String code) =>
    WalletApiError(code: code, message: 'refused', kind: kind);

WalletLocalizations _l10n(WidgetTester tester, Type of) =>
    WalletLocalizations.of(tester.element(find.byType(of)));

/// A SESSION-ONLY harness: the sheet over a fake session, no controller
/// (a switch returns NotActive; the probe and the render are what is under
/// test).
Widget _sessionOnly(FakeWalletSession session, {Widget? body}) {
  return ProviderScope(
    overrides: [walletSessionProvider.overrideWithValue(session)],
    child: MaterialApp(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      theme: lightTheme,
      home: Scaffold(body: body ?? const SyncServerSheet()),
    ),
  );
}

/// The ACTIVE harness: the real onboarding controller booted to Active over a
/// fake provisioner (its session is the picker's), so a switch runs the
/// controller's real swap.
Widget _active(FakeWalletProvisioner provisioner) {
  return ProviderScope(
    overrides: [
      walletProvisionerProvider.overrideWithValue(provisioner),
      onboardingStoreProvider.overrideWithValue(
        FakeOnboardingStore(confirmed: true),
      ),
    ],
    child: MaterialApp(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      theme: lightTheme,
      home: const Scaffold(body: SyncServerSheet()),
    ),
  );
}

void main() {
  // Stage S1 `copy`, piece 5. `probe_oracle` (`wallet.rs`) maps everything
  // that is not a FAILED private dial onto `syncServerUnreachable`, and since
  // stage S1 a private path that ACCEPTS the dial and then carries nothing no
  // longer reports `TorUnavailable` — on purpose: blaming the path for what
  // cannot be separated from a wedged server is the over-claim the stage
  // removed. So a censored path and a wedged server reach this one arm as the
  // same error. One sentence cannot serve both readers, and the split is on
  // the only fact the UI actually has: did the user TYPE this address?
  test('the unreachable copy splits on who supplied the address, and only '
      'the typed half says to check it', () {
    final l10n = WalletLocalizationsEn();
    final unreachable = _refusal(
      const WalletErrorKind.syncServerUnreachable(),
      'RW-SRV-002',
    );

    final typed = syncServerRefusalCopy(l10n, unreachable, typedAddress: true);
    final offered = syncServerRefusalCopy(
      l10n,
      unreachable,
      typedAddress: false,
    );
    expect(typed, l10n.walletSyncServerUnreachable);
    expect(offered, l10n.walletSyncServerUnreachableOffered);
    expect(typed, isNot(offered));

    // The defect the split exists to remove: a reader who typed NOTHING must
    // not be sent to check an address — under a censored private path it is
    // the one thing that is certainly fine.
    expect(offered.toLowerCase(), isNot(contains('address')));
    expect(typed.toLowerCase(), contains('address'));

    // Neither half picks a cause it cannot attest, and both give a step.
    for (final copy in [typed, offered]) {
      expect(copy, contains("isn't answering"));
      expect(copy, contains("can't reach it"));
      expect(copy, contains('server'));
    }
    expect(offered, contains("can't tell"));

    // Which half a refusal gets is decided by the CHOICE, not by the error:
    // a custom URL is the user's, everything the app offers is not.
    expect(
      addressTypedByUser(
        const SyncServerChoice.custom(url: 'https://x:443', key: null),
      ),
      isTrue,
    );
    for (final choice in const <SyncServerChoice>[
      SyncServerChoice.predefined(id: 'zec-rocks'),
      SyncServerChoice.default_(),
      SyncServerChoice.unknown(),
    ]) {
      expect(addressTypedByUser(choice), isFalse, reason: '$choice');
    }

    // The genuinely-down private path keeps its own sentence on BOTH halves —
    // that arm is decided by the error kind, never by who typed the address.
    final torDown = _refusal(
      const WalletErrorKind.sync_(stall: StallReason.torUnavailable),
      'RW-SYNC-001',
    );
    expect(
      syncServerRefusalCopy(l10n, torDown, typedAddress: true),
      l10n.walletStallTor,
    );
    expect(
      syncServerRefusalCopy(l10n, torDown, typedAddress: false),
      l10n.walletStallTor,
    );
  });

  testWidgets(
    'sync_server_sheet_lists_offered_servers_and_marks_the_one_in_use',
    (tester) async {
      final session = FakeWalletSession()
        ..syncServersResult = const [_zecRocks, _gated]
        ..syncServerStatusResult = _status(
          effective: 'https://lightwalletd.example.com:443',
          choice: const SyncServerChoice.predefined(id: 'example-gated'),
        );
      await tester.pumpWidget(_sessionOnly(session));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester, SyncServerSheet);

      // zec.rocks is both its label and its host (two texts); Example gated's
      // label and host differ.
      expect(find.text('zec.rocks'), findsNWidgets(2));
      expect(find.text('Example gated'), findsOneWidget);
      expect(find.text('lightwalletd.example.com'), findsOneWidget);
      // Exactly one row carries the in-use marker: the effective server's.
      expect(find.text(l10n.walletSyncServerInUse), findsOneWidget);
      final inUseRow = find.ancestor(
        of: find.text(l10n.walletSyncServerInUse),
        matching: find.byKey(const ValueKey('sync-server-example-gated')),
      );
      expect(inUseRow, findsOneWidget);
      // The default IS offered, so no "App default" row.
      expect(find.text(l10n.walletSyncServerAppDefault), findsNothing);
    },
  );

  testWidgets(
    'the_app_default_row_appears_only_when_the_default_is_not_offered',
    (tester) async {
      final session = FakeWalletSession()
        ..syncServersResult = const [_gated]
        ..syncServerStatusResult = _status(
          effective: 'https://my-host.example:443',
          defaultUrl: 'https://my-host.example:443',
        );
      await tester.pumpWidget(_sessionOnly(session));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester, SyncServerSheet);

      expect(find.text(l10n.walletSyncServerAppDefault), findsOneWidget);
      expect(find.text('my-host.example'), findsOneWidget);
      expect(find.byKey(const ValueKey('sync-server-default')), findsOneWidget);
      // …and it is the one in use.
      expect(
        find.ancestor(
          of: find.text(l10n.walletSyncServerInUse),
          matching: find.byKey(const ValueKey('sync-server-default')),
        ),
        findsOneWidget,
      );
    },
  );

  Future<void> openCustomAndType(WidgetTester tester, String url) async {
    await tester.tap(find.byKey(const ValueKey('sync-server-custom-toggle')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('sync-server-custom-url')),
      url,
    );
    await tester.pumpAndSettle();
  }

  /// Tap Check. The FIRST Check of a host shows the trust notice — the probe
  /// is the first packet that reaches the server — and Continue lets it run;
  /// a later Check of the same host shows none (`expectTrustNotice: false`).
  Future<void> check(
    WidgetTester tester,
    WalletLocalizations l10n, {
    bool expectTrustNotice = true,
  }) async {
    await tester.tap(find.byKey(const ValueKey('sync-server-check')));
    await tester.pumpAndSettle();
    if (expectTrustNotice) {
      expect(find.text(l10n.walletSyncServerTrustTitle), findsOneWidget);
      await tester.tap(find.text(l10n.walletSyncServerContinue));
      await tester.pumpAndSettle();
    } else {
      expect(find.text(l10n.walletSyncServerTrustTitle), findsNothing);
    }
  }

  testWidgets('an_invalid_custom_url_is_refused_inline_with_the_sdks_reason', (
    tester,
  ) async {
    // The SDK's door refuses (`invalidEndpoint`); the sheet renders ONE copy
    // for every reason and never echoes the reason string.
    final session = FakeWalletSession()
      ..syncServersResult = const [_zecRocks]
      ..probeThrows = _refusal(
        const WalletErrorKind.invalidEndpoint(reason: 'scheme must be https'),
        'RW-CFG-001',
      );
    await tester.pumpWidget(_sessionOnly(session));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester, SyncServerSheet);

    await openCustomAndType(tester, 'http://evil.example:9067');
    await check(tester, l10n);

    expect(find.text(l10n.walletSyncServerInvalidUrl), findsOneWidget);
    expect(find.textContaining('scheme must be https'), findsNothing);
    expect(find.byKey(const ValueKey('sync-server-use')), findsNothing);
    expect(session.probeCount, 1);
  });

  testWidgets('a_failed_probe_renders_the_unreachable_copy_with_retry', (
    tester,
  ) async {
    final session = FakeWalletSession()
      ..syncServersResult = const [_zecRocks]
      ..probeThrows = _refusal(
        const WalletErrorKind.syncServerUnreachable(),
        'RW-SRV-002',
      );
    await tester.pumpWidget(_sessionOnly(session));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester, SyncServerSheet);

    await openCustomAndType(tester, 'https://down.example:443');
    await check(tester, l10n);
    expect(find.text(l10n.walletSyncServerUnreachable), findsOneWidget);
    expect(find.byKey(const ValueKey('sync-server-use')), findsNothing);

    // Retry: the Check action is live again, and a second probe runs — with
    // NO second trust notice (the same host, accepted once).
    session.probeThrows = null;
    await check(tester, l10n, expectTrustNotice: false);
    expect(session.probeCount, 2);
    expect(find.text(l10n.walletSyncServerUnreachable), findsNothing);
    expect(find.byKey(const ValueKey('sync-server-use')), findsOneWidget);
  });

  testWidgets('a_wrong_network_probe_renders_the_network_copy', (tester) async {
    final session = FakeWalletSession()
      ..syncServersResult = const [_zecRocks]
      ..probeThrows = _refusal(
        const WalletErrorKind.networkMismatch(),
        'RW-STORE-002',
      );
    await tester.pumpWidget(_sessionOnly(session));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester, SyncServerSheet);

    await openCustomAndType(tester, 'https://testnet.zec.rocks:443');
    await check(tester, l10n);
    expect(find.text(l10n.walletSyncServerWrongNetwork), findsOneWidget);
    expect(
      session.lastProbedChoice,
      const SyncServerChoice.custom(
        url: 'https://testnet.zec.rocks:443',
        key: null,
      ),
    );
  });

  testWidgets('a_tor_down_probe_renders_the_tor_copy_not_the_server_copy', (
    tester,
  ) async {
    // The security review's LOW (folded): under a Tor-required policy whose
    // runtime is down the probe reports Tor, and the sheet says so with the
    // badge's own Tor copy — never "check the address".
    final session = FakeWalletSession()
      ..syncServersResult = const [_zecRocks]
      ..probeThrows = _refusal(
        const WalletErrorKind.sync_(stall: StallReason.torUnavailable),
        'RW-SYNC-001',
      );
    await tester.pumpWidget(_sessionOnly(session));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester, SyncServerSheet);

    await openCustomAndType(tester, 'https://mine.example:443');
    await check(tester, l10n);
    expect(find.text(l10n.walletStallTor), findsOneWidget);
    expect(find.text(l10n.walletSyncServerUnreachable), findsNothing);
  });

  testWidgets('the_in_flight_notice_precedes_any_switch', (tester) async {
    // Tapping an offered row shows the notice; Cancel switches nothing.
    final provisioner = FakeWalletProvisioner(exists: true)
      ..session = (FakeWalletSession()
        ..syncServersResult = const [_zecRocks, _gated]
        ..syncServerStatusResult = _status());
    await tester.pumpWidget(_active(provisioner));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester, SyncServerSheet);

    await tester.tap(find.byKey(const ValueKey('sync-server-example-gated')));
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletSyncServerSwitchNotice), findsOneWidget);
    await tester.tap(find.text(l10n.walletSyncServerCancel));
    await tester.pumpAndSettle();
    expect(provisioner.switchCount, 0);
    expect(find.text(l10n.walletSyncServerSwitchNotice), findsNothing);
  });

  testWidgets('the_in_flight_notice_names_the_state_it_interrupts', (
    tester,
  ) async {
    // The walk's two observations, folded at at the tip nothing is
    // "in progress", so the notice says the switch RECONNECTS; mid-scan it
    // says the switch RESTARTS the pass and that funds may read as pending
    // until the new server's scan catches up. Same session, status pushed
    // between the two taps — the sheet reads the CURRENT status at tap time.
    final session =
        FakeWalletSession(current: const SyncStatus.upToDate(tip: 3483379))
          ..syncServersResult = const [_zecRocks, _gated]
          ..syncServerStatusResult = _status();
    final provisioner = FakeWalletProvisioner(exists: true)..session = session;
    await tester.pumpWidget(_active(provisioner));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester, SyncServerSheet);

    await tester.tap(find.byKey(const ValueKey('sync-server-example-gated')));
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletSyncServerSwitchNoticeAtTip), findsOneWidget);
    expect(find.text(l10n.walletSyncServerSwitchNotice), findsNothing);
    await tester.tap(find.text(l10n.walletSyncServerCancel));
    await tester.pumpAndSettle();

    session.push(
      const SyncStatus.scanning(
        from: 3481392,
        to: 3483379,
        percent: 0.96,
        spendableReady: true,
        rewound: false,
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('sync-server-example-gated')));
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletSyncServerSwitchNotice), findsOneWidget);
    expect(find.text(l10n.walletSyncServerSwitchNoticeAtTip), findsNothing);
    await tester.tap(find.text(l10n.walletSyncServerCancel));
    await tester.pumpAndSettle();
    expect(provisioner.switchCount, 0);
  });

  testWidgets('the_trust_notice_precedes_the_first_use_of_a_custom_server', (
    tester,
  ) async {
    // The trust notice gates the FIRST PACKET to a custom host — the Check
    // (the probe), not the switch: Cancel sends nothing; Continue probes;
    // the same host is asked once; "Use this server" then shows only the
    // in-flight notice.
    final session = FakeWalletSession()
      ..syncServersResult = const [_zecRocks]
      ..syncServerStatusResult = _status();
    final provisioner = FakeWalletProvisioner(exists: true)..session = session;
    await tester.pumpWidget(_active(provisioner));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester, SyncServerSheet);

    await openCustomAndType(tester, 'https://mine.example:443');
    await tester.tap(find.byKey(const ValueKey('sync-server-check')));
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletSyncServerTrustTitle), findsOneWidget);
    expect(find.text(l10n.walletSyncServerTrustNotice), findsOneWidget);
    // Cancel: no packet left the device.
    await tester.tap(find.text(l10n.walletSyncServerCancel));
    await tester.pumpAndSettle();
    expect(session.probeCount, 0, reason: 'Cancel sends nothing');
    expect(find.byKey(const ValueKey('sync-server-use')), findsNothing);

    // A cancelled host is asked again; Continue lets the probe run.
    await check(tester, l10n);
    expect(session.probeCount, 1);
    expect(find.byKey(const ValueKey('sync-server-use')), findsOneWidget);

    // Accepted once: a re-Check of the same host shows no notice.
    await check(tester, l10n, expectTrustNotice: false);
    expect(session.probeCount, 2);

    // "Use this server": the in-flight notice ONLY — never the trust notice
    // a second time — and Cancel switches nothing.
    await tester.tap(find.byKey(const ValueKey('sync-server-use')));
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletSyncServerSwitchNotice), findsOneWidget);
    expect(find.text(l10n.walletSyncServerTrustTitle), findsNothing);
    await tester.tap(find.text(l10n.walletSyncServerCancel));
    await tester.pumpAndSettle();
    expect(provisioner.switchCount, 0);

    // And a PREDEFINED row never shows the trust notice.
    await tester.tap(find.byKey(const ValueKey('sync-server-zec-rocks')));
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletSyncServerTrustTitle), findsNothing);
  });

  // ── ADR-0568: a user's own server with a key ─────────────────────────────

  Future<void> typeKey(
    WidgetTester tester,
    String key, {
    String? header,
  }) async {
    await tester.enterText(find.byKey(const ValueKey('sync-server-key')), key);
    await tester.pumpAndSettle();
    if (header != null) {
      await tester.enterText(
        find.byKey(const ValueKey('sync-server-key-header')),
        header,
      );
      await tester.pumpAndSettle();
    }
  }

  ({String url, String? header, String? value}) unpack(SyncServerChoice? c) =>
      switch (c) {
        SyncServerChoice_Custom(:final url, :final key) => (
          url: url,
          header: key?.header,
          value: key?.value,
        ),
        _ => throw StateError('not a custom choice: $c'),
      };

  testWidgets('the_sheet_sends_the_key_with_the_custom_probe_and_switch', (
    tester,
  ) async {
    final session = FakeWalletSession()
      ..syncServersResult = const [_zecRocks]
      ..syncServerStatusResult = _status();
    final provisioner = FakeWalletProvisioner(exists: true)..session = session;
    await tester.pumpWidget(_active(provisioner));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester, SyncServerSheet);

    await openCustomAndType(tester, 'https://mine.example:443');
    await typeKey(tester, 'users-key-1', header: 'x-api-key');
    await check(tester, l10n);
    expect(unpack(session.lastProbedChoice), (
      url: 'https://mine.example:443',
      header: 'x-api-key',
      value: 'users-key-1',
    ));
    await tester.tap(find.byKey(const ValueKey('sync-server-use')));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.walletSyncServerContinue));
    await tester.pumpAndSettle();
    expect(provisioner.switchCount, 1);
    expect(
      unpack(provisioner.lastSwitchChoice),
      (
        url: 'https://mine.example:443',
        header: 'x-api-key',
        value: 'users-key-1',
      ),
      reason: 'the switch sends exactly what the probe verified',
    );
    // The platform's autofill context closed WITHOUT saving the key (iOS
    // would otherwise offer it to the password manager — security review L4).
    final finishes = tester.testTextInput.log
        .where((c) => c.method == 'TextInput.finishAutofillContext')
        .map((c) => c.arguments)
        .toList();
    expect(finishes, isNotEmpty);
    expect(finishes.last, isFalse, reason: 'shouldSave must be false');
    // The key left the field once the wallet held it.
    expect(
      tester
          .widget<TextField>(find.byKey(const ValueKey('sync-server-key')))
          .controller!
          .text,
      isEmpty,
    );
  });

  testWidgets('editing_the_key_unverifies_the_custom_server', (tester) async {
    final session = FakeWalletSession()
      ..syncServersResult = const [_zecRocks]
      ..syncServerStatusResult = _status();
    await tester.pumpWidget(_sessionOnly(session));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester, SyncServerSheet);

    await openCustomAndType(tester, 'https://mine.example:443');
    await typeKey(tester, 'key-one', header: 'x-api-key');
    await check(tester, l10n);
    expect(find.byKey(const ValueKey('sync-server-use')), findsOneWidget);
    // Any edit — the key, then the header — takes "Use" away until re-checked.
    await typeKey(tester, 'key-two');
    expect(find.byKey(const ValueKey('sync-server-use')), findsNothing);
    await check(tester, l10n, expectTrustNotice: false);
    expect(find.byKey(const ValueKey('sync-server-use')), findsOneWidget);
    await tester.enterText(
      find.byKey(const ValueKey('sync-server-key-header')),
      'x-other',
    );
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('sync-server-use')), findsNothing);
  });

  testWidgets('a_key_needs_a_header', (tester) async {
    final session = FakeWalletSession()
      ..syncServersResult = const [_zecRocks]
      ..syncServerStatusResult = _status();
    await tester.pumpWidget(_sessionOnly(session));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester, SyncServerSheet);

    await openCustomAndType(tester, 'https://mine.example:443');
    // No header field until a key is typed.
    expect(find.byKey(const ValueKey('sync-server-key-header')), findsNothing);
    await typeKey(tester, 'key-without-header');
    expect(
      find.byKey(const ValueKey('sync-server-key-header')),
      findsOneWidget,
    );
    await tester.tap(find.byKey(const ValueKey('sync-server-check')));
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletSyncServerKeyHeaderNeeded), findsOneWidget);
    expect(find.text(l10n.walletSyncServerTrustTitle), findsNothing);
    expect(session.probeCount, 0, reason: 'nothing leaves without a header');
  });

  testWidgets('a_bad_key_shows_the_key_copy_not_the_url_copy', (tester) async {
    final session = FakeWalletSession()
      ..syncServersResult = const [_zecRocks]
      ..probeThrows = _refusal(
        const WalletErrorKind.invalidEndpointAuth(
          reason: 'auth header name must not start with grpc-',
        ),
        'RW-CFG-004',
      );
    await tester.pumpWidget(_sessionOnly(session));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester, SyncServerSheet);

    await openCustomAndType(tester, 'https://mine.example:443');
    await typeKey(tester, 'k', header: 'grpc-key');
    await check(tester, l10n);
    expect(find.text(l10n.walletSyncServerKeyInvalid), findsOneWidget);
    expect(find.text(l10n.walletSyncServerInvalidUrl), findsNothing);
    // The SDK's reason is never echoed (the typed header itself, in its
    // field, still reads `grpc-key`).
    expect(
      find.textContaining('must not start with'),
      findsNothing,
      reason: 'no reason echo',
    );
  });

  testWidgets(
    'the_trust_notice_names_identification_and_re_asks_when_a_key_is_added',
    (tester) async {
      final session = FakeWalletSession()
        ..syncServersResult = const [_zecRocks]
        ..syncServerStatusResult = _status();
      await tester.pumpWidget(_sessionOnly(session));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester, SyncServerSheet);

      await openCustomAndType(tester, 'https://mine.example:443');
      // Keyless first: the plain notice, without the identification line.
      await tester.tap(find.byKey(const ValueKey('sync-server-check')));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletSyncServerTrustNotice), findsOneWidget);
      expect(
        find.textContaining(l10n.walletSyncServerTrustNoticeKey),
        findsNothing,
      );
      await tester.tap(find.text(l10n.walletSyncServerContinue));
      await tester.pumpAndSettle();
      // The SAME host with a key: asked again, now with the line.
      await typeKey(tester, 'k-1', header: 'x-api-key');
      await tester.tap(find.byKey(const ValueKey('sync-server-check')));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletSyncServerTrustTitle), findsOneWidget);
      expect(
        find.textContaining(l10n.walletSyncServerTrustNoticeKey),
        findsOneWidget,
      );
    },
  );

  testWidgets('a_keyed_current_custom_server_shows_key_saved', (tester) async {
    final session = FakeWalletSession()
      ..syncServersResult = const [_zecRocks]
      ..syncServerStatusResult = _status(
        effective: 'https://mine.example:443',
        choice: const SyncServerChoice.custom(
          url: 'https://mine.example:443',
          // A RETURNED choice: the header only, the value always empty.
          key: SyncServerKey(header: 'x-api-key', value: ''),
        ),
      );
    await tester.pumpWidget(_sessionOnly(session));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester, SyncServerSheet);
    expect(
      find.text('mine.example · ${l10n.walletSyncServerKeySaved}'),
      findsOneWidget,
    );
    // A keyless custom choice says nothing about a key.
    session.syncServerStatusResult = _status(
      effective: 'https://mine.example:443',
      choice: const SyncServerChoice.custom(
        url: 'https://mine.example:443',
        key: null,
      ),
    );
    await tester.pumpWidget(_sessionOnly(session, body: const SizedBox()));
    await tester.pumpWidget(_sessionOnly(session));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('sync-server-key-saved')), findsNothing);
  });

  test('a_custom_choice_never_prints_its_key', () {
    // ADR-0568 / security review M2: the key rides a PLAIN `SyncServerKey`,
    // so the freezed `custom` toString renders the object, never its value.
    const choice = SyncServerChoice.custom(
      url: 'https://mine.example:443',
      key: SyncServerKey(header: 'x-api-key', value: 'printed-key-9'),
    );
    expect('$choice', isNot(contains('printed-key-9')));
    expect(
      '${const SyncServerKey(header: 'x-api-key', value: 'printed-key-9')}',
      isNot(contains('printed-key-9')),
    );
  });

  testWidgets('the_key_field_never_enables_suggestions_or_learning', (
    tester,
  ) async {
    final session = FakeWalletSession()
      ..syncServersResult = const [_zecRocks]
      ..syncServerStatusResult = _status();
    await tester.pumpWidget(_sessionOnly(session));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester, SyncServerSheet);
    await openCustomAndType(tester, 'https://mine.example:443');

    void expectPrivate() {
      final field = tester.widget<TextField>(
        find.byKey(const ValueKey('sync-server-key')),
      );
      expect(field.enableSuggestions, isFalse);
      expect(field.autocorrect, isFalse);
      expect(field.enableIMEPersonalizedLearning, isFalse);
      expect(field.autofillHints, isNull);
    }

    expectPrivate();
    // No field of the custom server offers an autofill hint, and the three
    // share one autofill group that cancels on dispose (security review L4).
    await typeKey(tester, 'k');
    for (final key in const [
      'sync-server-custom-url',
      'sync-server-key',
      'sync-server-key-header',
    ]) {
      expect(
        tester.widget<TextField>(find.byKey(ValueKey(key))).autofillHints,
        isNull,
        reason: key,
      );
    }
    final group = tester.widget<AutofillGroup>(find.byType(AutofillGroup));
    expect(group.onDisposeAction, AutofillContextAction.cancel);
    expect(
      tester
          .widget<TextField>(find.byKey(const ValueKey('sync-server-key')))
          .obscureText,
      isFalse, // shown by default: no secure field for iOS's Passwords bar
    );
    // Hidden, the field is still private to the keyboard.
    await tester.tap(find.text(l10n.walletSyncServerKeyHide));
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<TextField>(find.byKey(const ValueKey('sync-server-key')))
          .obscureText,
      isTrue,
    );
    expectPrivate();
  });

  testWidgets(
    'a_successful_switch_swaps_the_session_and_the_server_row_reads_the_new_host',
    (tester) async {
      final switched = FakeWalletSession()
        ..syncServersResult = const [_zecRocks, _gated]
        ..syncServerStatusResult = _status(
          effective: 'https://lightwalletd.example.com:443',
          choice: const SyncServerChoice.predefined(id: 'example-gated'),
        );
      final provisioner = FakeWalletProvisioner(exists: true)
        ..session = (FakeWalletSession()
          ..syncServersResult = const [_zecRocks, _gated]
          ..syncServerStatusResult = _status())
        ..switchSession = switched;
      await tester.pumpWidget(_active(provisioner));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester, SyncServerSheet);

      // Before: zec.rocks is in use.
      expect(
        find.ancestor(
          of: find.text(l10n.walletSyncServerInUse),
          matching: find.byKey(const ValueKey('sync-server-zec-rocks')),
        ),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const ValueKey('sync-server-example-gated')));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletSyncServerContinue));
      await tester.pumpAndSettle();

      expect(provisioner.switchCount, 1);
      expect(
        provisioner.lastSwitchChoice,
        const SyncServerChoice.predefined(id: 'example-gated'),
      );
      // After: the providers re-read from the SWAPPED session — the in-use
      // marker moved to Example gated.
      expect(
        find.ancestor(
          of: find.text(l10n.walletSyncServerInUse),
          matching: find.byKey(const ValueKey('sync-server-example-gated')),
        ),
        findsOneWidget,
      );
    },
  );

  testWidgets('a_refused_switch_renders_the_typed_copy_and_changes_nothing', (
    tester,
  ) async {
    final provisioner = FakeWalletProvisioner(exists: true)
      ..session = (FakeWalletSession()
        ..syncServersResult = const [_zecRocks, _gated]
        ..syncServerStatusResult = _status())
      ..failSwitch = _refusal(
        const WalletErrorKind.syncServerUnreachable(),
        'RW-SRV-002',
      );
    await tester.pumpWidget(_active(provisioner));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester, SyncServerSheet);
    // The boot itself opened the wallet once; the refusal must add no open.
    final opensAtBoot = provisioner.openCount;

    await tester.tap(find.byKey(const ValueKey('sync-server-example-gated')));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.walletSyncServerContinue));
    await tester.pumpAndSettle();

    expect(provisioner.switchCount, 1);
    expect(
      provisioner.openCount,
      opensAtBoot,
      reason: 'a pre-swap refusal never re-opens',
    );
    // Stage S1 `copy`: the user tapped a server the APP offered and typed
    // nothing, so the sentence must not send them to check an address they
    // never entered. This is the SAME error kind a censored private path now
    // arrives as — `probe_oracle` maps everything but a FAILED private dial
    // here — which is why the offered half names both causes and neither.
    expect(find.text(l10n.walletSyncServerUnreachableOffered), findsOneWidget);
    expect(find.text(l10n.walletSyncServerUnreachable), findsNothing);
    // Still zec.rocks in use.
    expect(
      find.ancestor(
        of: find.text(l10n.walletSyncServerInUse),
        matching: find.byKey(const ValueKey('sync-server-zec-rocks')),
      ),
      findsOneWidget,
    );
  });

  testWidgets('the_fallback_banner_renders_on_the_sheet_and_the_server_row', (
    tester,
  ) async {
    final session = FakeWalletSession()
      ..syncServersResult = const [_zecRocks]
      ..syncServerStatusResult = _status(
        choice: const SyncServerChoice.predefined(id: 'gone-server'),
        fallback: const SyncServerFallback.choiceNotOffered(id: 'gone-server'),
      );
    // The picker's banner.
    await tester.pumpWidget(_sessionOnly(session));
    await tester.pumpAndSettle();
    var l10n = _l10n(tester, SyncServerSheet);
    expect(
      find.text(l10n.walletSyncServerFallbackNotOffered('zec.rocks')),
      findsOneWidget,
    );
    // The sync sheet's Server row banner, from the SAME status.
    await tester.pumpWidget(
      _sessionOnly(session, body: const SyncStatusSheet()),
    );
    await tester.pumpAndSettle();
    l10n = _l10n(tester, SyncStatusSheet);
    expect(
      find.text(l10n.walletSyncServerFallbackNotOffered('zec.rocks')),
      findsOneWidget,
    );
    expect(find.text('zec.rocks'), findsOneWidget);
  });

  testWidgets('sync_server_sheet_targets_are_44px_and_labelled', (
    tester,
  ) async {
    final session = FakeWalletSession()
      ..syncServersResult = const [_zecRocks, _gated]
      ..syncServerStatusResult = _status();
    await tester.pumpWidget(_sessionOnly(session));
    await tester.pumpAndSettle();

    for (final key in const [
      ValueKey('sync-server-zec-rocks'),
      ValueKey('sync-server-example-gated'),
      ValueKey('sync-server-custom-toggle'),
    ]) {
      final size = tester.getSize(find.byKey(key));
      expect(size.height, greaterThanOrEqualTo(44), reason: '$key height');
      expect(
        tester.getSemantics(find.byKey(key)),
        isSemantics(isButton: true),
        reason: '$key is a button to a screen reader',
      );
    }
    // The in-use row is the SELECTED one to a screen reader.
    expect(
      tester.getSemantics(find.byKey(const ValueKey('sync-server-zec-rocks'))),
      isSemantics(isSelected: true),
    );
  });

  testWidgets('the_sync_sheets_server_row_is_a_button_that_opens_the_picker', (
    tester,
  ) async {
    final session = FakeWalletSession()
      ..syncServersResult = const [_zecRocks]
      ..syncServerStatusResult = _status();
    await tester.pumpWidget(
      _sessionOnly(session, body: const SyncStatusSheet()),
    );
    await tester.pumpAndSettle();
    final l10n = _l10n(tester, SyncStatusSheet);

    final row = find.bySemanticsLabel(
      l10n.walletSyncServerRowSemantics('zec.rocks'),
    );
    expect(row, findsOneWidget);
    expect(
      tester.getSize(row).height,
      greaterThanOrEqualTo(44),
      reason: 'the Server row is a 44 dp target',
    );
    await tester.tap(row);
    await tester.pumpAndSettle();
    expect(find.byType(SyncServerSheet), findsOneWidget);
    expect(find.text(l10n.walletSyncServerSheetTitle), findsOneWidget);
  });

  testWidgets('the_picker_lifts_its_fields_above_the_keyboard', (tester) async {
    // S15 iPhone walk: the custom-server fields sat under the keyboard. The
    // sheet pads its scroll by the keyboard's inset, as the other sheets do.
    final session = FakeWalletSession()
      ..syncServersResult = const [_zecRocks]
      ..syncServerStatusResult = _status();
    await tester.pumpWidget(
      _sessionOnly(
        session,
        body: Builder(
          builder: (context) => MediaQuery(
            data: MediaQuery.of(
              context,
            ).copyWith(viewInsets: const EdgeInsets.only(bottom: 300)),
            child: const SyncServerSheet(),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final scroll = tester.widget<SingleChildScrollView>(
      find
          .descendant(
            of: find.byType(SyncServerSheet),
            matching: find.byType(SingleChildScrollView),
          )
          .first,
    );
    expect(scroll.padding, const EdgeInsets.fromLTRB(20, 16, 20, 324));
  });
}
