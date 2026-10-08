// P25 (docs/specs/tor-plugin.md §8): the toggle on yields
// `TorPolicy.required_(runtime: hostDialer())` and calls `ZecWalletTor.init`
// BEFORE the wallet opens; off yields `off` and never calls `init`. Plus: off
// is the default, and a Tor user is never quietly given a direct connection.
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:zec_wallet_example/core/router/router.dart';
import 'package:zec_wallet_example/core/tor/tor_plugin.dart';
import 'package:zec_wallet_example/app.dart';
import 'package:zec_wallet_example/features/settings/appearance_screen.dart';
import 'package:zec_wallet_tor/zec_wallet_tor.dart';
import 'package:zec_wallet_ui/testing.dart';
import 'package:zec_wallet_ui/zec_wallet_ui.dart';

import 'support/fake_tor_plugin.dart';

const _dbDir = '/data/user/0/app/files/zec_wallet';

Future<String> _noDisk(String dbDir) async => torDirFor(dbDir);

void main() {
  test('the toggle ON inits the plugin BEFORE the wallet opens, over '
      'required(hostDialer)', () async {
    final log = <String>[];
    final tor = FakeTorPlugin(log);
    final boot = await bootTorPlugin(
      enabled: true,
      dbDir: _dbDir,
      tor: tor,
      resolveDir: _noDisk,
    );
    const required = TorPolicy.required_(
      runtime: TorRuntimeConfig.hostDialer(),
    );
    expect(boot.policy, required);
    // The config the wallet opens with carries it.
    expect(buildWalletConfig(dbDir: _dbDir, tor: boot.policy).tor, required);

    // The wallet opens through the (decorated) provisioner, after the boot.
    final provisioner = boot.decorateProvisioner!(LoggingProvisioner(log));
    await provisioner.open();
    expect(log, ['tor.init', 'wallet.open']);
    expect(tor.torDirs, [torDirFor(_dbDir)]);
  });

  test('the toggle OFF yields off and never touches the plugin', () async {
    final log = <String>[];
    var resolved = false;
    final boot = await bootTorPlugin(
      enabled: false,
      dbDir: _dbDir,
      tor: FakeTorPlugin(log),
      resolveDir: (d) async {
        resolved = true;
        return torDirFor(d);
      },
    );
    expect(boot.policy, const TorPolicy.off());
    expect(boot.decorateProvisioner, isNull);
    expect(log, isEmpty);
    expect(resolved, isFalse, reason: 'off creates no Tor directory either');
  });

  test('a plugin that cannot start fails the boot; it never falls back to a '
      'direct connection', () async {
    final tor = FakeTorPlugin([])
      ..failInit = const TorPluginError(TorPluginErrorKind.walletNotLoaded);
    await expectLater(
      bootTorPlugin(
        enabled: true,
        dbDir: _dbDir,
        tor: tor,
        resolveDir: _noDisk,
      ),
      throwsA(isA<TorPluginError>()),
    );
  });

  test('the choice is OFF when never set, and read back once set', () async {
    SharedPreferences.setMockInitialValues({});
    expect(await readUseTorPlugin(), isFalse);
    SharedPreferences.setMockInitialValues({useTorPluginPrefsKey: true});
    expect(await readUseTorPlugin(), isTrue);
  });

  test(
    'an unreadable choice fails the boot attempt instead of reading OFF',
    () async {
      await expectLater(
        readUseTorPlugin(open: () => Future.error(StateError('store down'))),
        throwsStateError,
      );
    },
  );

  testWidgets('the Settings switch starts OFF and persists the choice', (
    tester,
  ) async {
    SharedPreferences.setMockInitialValues({});
    final container = ProviderContainer(
      overrides: [
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
    container.read(routerProvider).go(AppRoutes.appearance);
    await tester.pumpAndSettle();

    final toggle = find.byKey(torPluginToggleKey);
    await tester.scrollUntilVisible(toggle, 200);
    expect(tester.widget<SwitchListTile>(toggle).value, isFalse);

    await tester.tap(toggle);
    await tester.pumpAndSettle();
    expect(container.read(useTorPluginProvider), isTrue);
    final prefs = await SharedPreferences.getInstance();
    expect(prefs.getBool(useTorPluginPrefsKey), isTrue);
  });
}
