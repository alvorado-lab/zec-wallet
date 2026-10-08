import 'dart:convert' show jsonDecode;
import 'dart:io' show File;
import 'dart:typed_data' show Uint8List;

import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_config.dart';
import 'package:zec_wallet/zec_wallet.dart';

/// The reference-app wallet host-policy (spec §3.2g iii-B-2-b). Every decision
/// the SDK validates but does not choose is pinned here at its boundary (gate 7
/// — no magic numbers): a drift in the endpoint, network, Tor policy, jitter, or
/// seed-persistence is a deliberate, reviewable change, never an accident.
void main() {
  group('reference host-policy constants', () {
    test('mainnet endpoint is the zec.rocks public server (founder S56)', () {
      // Spelled WITH the port since P3-13, byte-equal to the SDK catalog's
      // `zec-rocks` entry (the core's `the_dart_reference_endpoints_match_the_
      // catalog` pins the equality): the picker marks the row in use by URL
      // equality, so the default and the offered entry must match exactly.
      expect(referenceMainnetEndpoint, 'https://zec.rocks:443');
      expect(referenceTestnetEndpoint, 'https://testnet.zec.rocks:443');
    });

    test('the offered list is the host\'s to pass, unchanged '
        '(ADR-0568: the package ships no gated server and no key)', () {
      // The builder adds nothing: `null` offers nothing beyond the default,
      // and whatever list the host passes — the catalog plus its own servers —
      // goes into the config as given. (The catalog itself needs the FFI;
      // the core's `the_reference_catalog_names_public_servers_only` pins it.)
      expect(buildWalletConfig(dbDir: '/x').syncServers, isNull);
      const offered = [
        SyncServer(
          id: 'zec-rocks',
          label: 'zec.rocks',
          url: 'https://zec.rocks:443',
          authHeader: null,
          authValue: null,
        ),
      ];
      final config = buildWalletConfig(dbDir: '/x', syncServers: offered);
      expect(config.syncServers, offered);
      // …and the RESTORE copy carries it (the FR-28 silent-drop shape).
      expect(config.withBirthdayHeight(2_100_000).syncServers, offered);
    });

    test('Tor policy is OFF / direct (founder S56)', () {
      expect(referenceTorPolicy, const TorPolicy.off());
    });

    test('broadcast jitter window is the documented 0..=10s', () {
      expect(referenceBroadcastJitterMaxMs, 10000);
    });

    test(
      'the Sapling-activation date floor is 2018-10-22 (the scan-picker floor)',
      () {
        // Shared by the restore birthday + rescan date pickers as `firstDate` — the
        // SDK floors anything earlier, so this is the one source of truth for the
        // visible floor (a drift would silently change both pickers).
        expect(kZcashSaplingActivationDate, DateTime(2018, 10, 22));
      },
    );
  });

  group('ZEC_NETWORK build define (the v-5c device-proof switch)', () {
    test(
      'unset/default define is mainnet (this test binary carries no define)',
      () {
        expect(zecNetworkDefine, 'mainnet');
        expect(referenceNetwork(), Network.main);
        expect(referenceDbDirLeaf(), 'zec_wallet');
      },
    );

    test('maps mainnet and testnet, fails LOUD on anything else', () {
      expect(networkForDefine('mainnet'), Network.main);
      expect(networkForDefine('testnet'), Network.test);
      // A typo must never silently run MAINNET during a testnet money proof.
      expect(() => networkForDefine('tesnet'), throwsStateError);
      expect(() => networkForDefine(''), throwsStateError);
    });

    test('endpoint follows the network; ZEC_ENDPOINT overrides either', () {
      expect(
        endpointForNetwork(Network.main, override: ''),
        referenceMainnetEndpoint,
      );
      expect(
        endpointForNetwork(Network.test, override: ''),
        referenceTestnetEndpoint,
      );
      expect(
        endpointForNetwork(Network.test, override: 'https://localhost:9067'),
        'https://localhost:9067',
      );
    });

    test('testnet endpoint is the zec.rocks testnet rail', () {
      expect(referenceTestnetEndpoint, 'https://testnet.zec.rocks:443');
    });

    test('the db-dir leaf keeps mainnet on the historical path and puts a '
        'testnet wallet BESIDE it, never over it', () {
      expect(dbDirLeafForNetwork(Network.main), 'zec_wallet');
      expect(dbDirLeafForNetwork(Network.test), 'zec_wallet_testnet');
    });
  });

  group('buildWalletConfig', () {
    test('threads the resolved dbDir through verbatim', () {
      final config = buildWalletConfig(dbDir: '/data/app/zec_wallet');
      expect(config.dbDir, '/data/app/zec_wallet');
    });

    test('is mainnet over the reference endpoint with Tor off', () {
      final config = buildWalletConfig(dbDir: '/x');
      expect(config.network, Network.main);
      expect(config.endpointUrl, referenceMainnetEndpoint);
      expect(config.tor, referenceTorPolicy);
    });

    test('carries a host-chosen Tor policy verbatim', () {
      const required = TorPolicy.required_(
        runtime: TorRuntimeConfig.hostDialer(),
      );
      expect(buildWalletConfig(dbDir: '/x', tor: required).tor, required);
    });

    test('seals the seed under the keychain (revealMnemonic-capable)', () {
      // sealedKeychain is required for revealMnemonic to work after a restart —
      // the backup flow depends on it.
      expect(
        buildWalletConfig(dbDir: '/x').seedPersistence,
        SeedPersistence.sealedKeychain,
      );
    });

    test('creates at the current tip (null birthday — no restore height)', () {
      expect(buildWalletConfig(dbDir: '/x').birthdayHeight, isNull);
    });

    test('sets the broadcast-decorrelation jitter window', () {
      expect(
        buildWalletConfig(dbDir: '/x').broadcastJitter,
        const JitterPolicy.uniform(maxMs: 10000),
      );
    });

    test('the restore copy carries every OTHER field, the read scope '
        'included — a dropped field here is silent', () {
      // `withBirthdayHeight` is the ONE place a config field is overridden
      // after `buildWalletConfig`, and it rebuilds the whole config by hand
      // (the FRB type has no `copyWith`). A field it forgets is silently
      // defaulted on the RESTORE path only — the shape FR-28 fixed one layer
      // up, where machine-memo bytes vanished at re-compose. FR-27's read scope
      // is the field that would hurt most: a host whose scope disappeared on
      // restore reads "no machine memo" over every payment ever made to it.
      // Not `const`: a prefix is a `Uint8List`, which has no const form. Worth
      // knowing before a host tries to declare its scope as a constant.
      final scoped = WalletConfig(
        dbDir: '/data/zec',
        network: Network.main,
        endpointUrl: 'https://zec.rocks',
        tor: const TorPolicy.off(),
        seedPersistence: SeedPersistence.sealedKeychain,
        birthdayHeight: null,
        broadcastJitter: const JitterPolicy.uniform(maxMs: 10000),
        machineMemoPrefixes: [
          Uint8List.fromList([0x52, 0x4C, 0x4D, 0x01]),
        ],
      );
      final restored = scoped.withBirthdayHeight(2_100_000);

      expect(restored.birthdayHeight, 2100000, reason: 'the override applies');
      expect(
        restored.machineMemoPrefixes,
        scoped.machineMemoPrefixes,
        reason: 'the read scope survives a restore — never silently emptied',
      );
    });

    test('the restore copy carries EVERY field: a config with every field '
        'set survives the copy unchanged', () async {
      // Comparing two copies with each other proves nothing: both drop the
      // same field (the endpoint key was dropped that way, unnoticed). So the
      // copy is compared with the ORIGINAL, every field set to a non-default
      // value, and the field list below is checked against the generated
      // class, so a field added to `WalletConfig` and missing here fails too.
      final servers = [
        const SyncServer(
          id: 'mine',
          label: 'Mine',
          url: 'https://lwd.example:443',
          authHeader: 'x-api-key',
          authValue: 'server-key',
        ),
      ];
      final full = WalletConfig(
        dbDir: '/data/zec',
        network: Network.test,
        endpointUrl: 'https://lwd.example:443',
        endpointAuthHeader: 'x-api-key',
        endpointAuthValue: 'endpoint-key',
        tor: const TorPolicy.off(),
        seedPersistence: SeedPersistence.none,
        birthdayHeight: 1_700_000,
        broadcastJitter: const JitterPolicy.uniform(maxMs: 2500),
        machineMemoPrefixes: [
          Uint8List.fromList([0x52, 0x4C, 0x4D, 0x01]),
        ],
        syncServers: servers,
      );
      const setHere = {
        'dbDir',
        'network',
        'endpointUrl',
        'endpointAuthHeader',
        'endpointAuthValue',
        'tor',
        'seedPersistence',
        'birthdayHeight',
        'broadcastJitter',
        'machineMemoPrefixes',
        'syncServers',
      };

      expect(full.withBirthdayHeight(full.birthdayHeight), full);
      final restored = full.withBirthdayHeight(2_100_000);
      expect(restored.birthdayHeight, 2100000);
      expect(restored.endpointAuthHeader, 'x-api-key');
      expect(restored.endpointAuthValue, 'endpoint-key');
      expect(restored.withBirthdayHeight(1_700_000), full);

      // `flutter test` cannot resolve a package: URI, so read the resolved
      // location from the package config `pub get` wrote.
      final packageConfig = File('.dart_tool/package_config.json');
      final packages =
          (jsonDecode(packageConfig.readAsStringSync())
                  as Map<String, dynamic>)['packages']
              as List<dynamic>;
      final zecWallet = packages.cast<Map<String, dynamic>>().singleWhere(
        (p) => p['name'] == 'zec_wallet',
      );
      final rootUri = zecWallet['rootUri'] as String;
      final root = packageConfig.absolute.uri.resolve(
        rootUri.endsWith('/') ? rootUri : '$rootUri/',
      );
      final text = File.fromUri(
        root.resolve('lib/src/rust/api/config.dart'),
      ).readAsStringSync();
      final start = text.indexOf('class WalletConfig {');
      final end = text.indexOf('\nclass ', start + 1);
      expect(start, isNot(-1), reason: 'the generated class moved');
      final generated = RegExp(r'^  final [^;]+ (\w+);$', multiLine: true)
          .allMatches(text.substring(start, end == -1 ? text.length : end))
          .map((m) => m.group(1)!)
          .toSet();
      expect(
        generated,
        setHere,
        reason: 'a WalletConfig field this test does not set',
      );
      // The constructor's parameter list too: a declaration `dart format` wraps over two
      // lines would slip past the `final` scan above, but every field is still a
      // `this.<name>` parameter.
      final ctor = text.indexOf('const WalletConfig({', start);
      expect(ctor, isNot(-1), reason: 'the generated constructor moved');
      final params = RegExp(r'this\.(\w+)')
          .allMatches(text.substring(ctor, text.indexOf('});', ctor)))
          .map((m) => m.group(1)!)
          .toSet();
      expect(
        params,
        setHere,
        reason: 'a WalletConfig parameter this test does not set',
      );
    });

    test(
      'the whole config equals an explicitly spelled-out reference config',
      () {
        // A single value-equality pin so any field drift fails one obvious test.
        expect(
          buildWalletConfig(dbDir: '/data/zec'),
          const WalletConfig(
            dbDir: '/data/zec',
            network: Network.main,
            endpointUrl: 'https://zec.rocks:443',
            tor: TorPolicy.off(),
            seedPersistence: SeedPersistence.sealedKeychain,
            birthdayHeight: null,
            broadcastJitter: JitterPolicy.uniform(maxMs: 10000),
            // FR-27: the reference app defines no envelope of its own, so it
            // registers no read scope and `machineMemos` stays closed for it.
            machineMemoPrefixes: [],
          ),
        );
      },
    );
  });
}
