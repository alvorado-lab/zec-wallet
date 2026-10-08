import 'package:flutter/foundation.dart' show debugPrint;

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:path_provider/path_provider.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_example/core/ffi/wallet_ffi.dart';

/// FR-24 §3.2f device acceptance — the AIRPLANE-MODE create.
///
/// GATES (honest): create + a real shielded mainnet UA in ONE offline
/// shot on the whole device stack (real keystore vault → SQLCipher store →
/// FFI bridge). It does NOT isolate the eager-import arm: this drives
/// `Generate` + `SealedKeychain` via [WalletHandle.createGenerated], where
/// `currentAddress` has a held-seed offline-derivation fallback (core
/// `wallet.rs`) — so a binary with the eager import BROKEN would still serve
/// this UA and stay green. The eager arm itself (account imported at create,
/// no held-seed fallback) is pinned by the core suite
/// (`fresh_raw_bytes_none_create_imports_eagerly_offline` + kin, on the
/// None-persistence path that has no fallback). The T2 fresh-flag FFI flow
/// (`createWithHostSeed(freshlyGenerated: true)`) needs a registered C-ABI
/// seed port this example does not ship → a native test-port device gate is
/// #377. The RUNNER owns the radio state: enable airplane mode via adb BEFORE
/// this test, restore it after — the test itself never dials (the endpoint
/// below is shape-validated only, which is FR-24's whole point; without the
/// runner's toggle it passes online too, so it proves "one-shot offline UA",
/// not "no dial").
///
/// The wallet lives in a THROWAWAY sibling leaf (`e2e_fr24_throwaway`) — never
/// the example's production `zec_wallet` leaf — and is crypto-shredded at the
/// end even on assertion failure (its keychain namespace derives from the
/// throwaway dir, so the shred can never touch production custody).
void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  // same static-link loader handling as the app (see core/ffi/wallet_ffi.dart)
  setUpAll(() async => RustLib.init(externalLibrary: walletExternalLibrary()));

  testWidgets('fr24_airplane_create_serves_receive_address_immediately', (
    tester,
  ) async {
    final support = await getApplicationSupportDirectory();
    final config = WalletConfig(
      dbDir: '${support.path}/e2e_fr24_throwaway',
      network: Network.main,
      endpointUrl: 'https://zec.rocks:443',
      tor: const TorPolicy.off(),
      seedPersistence: SeedPersistence.sealedKeychain,
      birthdayHeight: null,
      broadcastJitter: const JitterPolicy.none(),
      machineMemoPrefixes: const [], // FR-27 opt-in: no read scope here
    );
    // Idempotent re-entry: shred an aborted prior run's remnant first.
    if (await WalletHandle.walletExists(config: config)) {
      await WalletHandle.wipeForce(config: config);
    }
    try {
      final sw = Stopwatch()..start();
      final handle = await WalletHandle.createGenerated(config: config);
      final ua = await handle.currentAddress();
      sw.stop();
      // Timing only — never the address (§5.4).
      debugPrint(
        'fr24_e2e: create+address in ${sw.elapsedMilliseconds}ms with no '
        'connectivity',
      );
      expect(
        ua,
        startsWith('u1'),
        reason: 'a real mainnet UA must serve offline from second zero',
      );
      await handle.close();
      await WalletHandle.wipeForce(config: config);
      expect(
        await WalletHandle.walletExists(config: config),
        isFalse,
        reason: 'the throwaway is crypto-shredded before the test ends',
      );
    } finally {
      // Backstop shred for the assertion-failure paths above; catch-ALL so no
      // cleanup fault of any type can replace the real test failure.
      try {
        if (await WalletHandle.walletExists(config: config)) {
          await WalletHandle.wipeForce(config: config);
        }
      } catch (e) {
        debugPrint('fr24_e2e: backstop wipe failed: $e');
      }
    }
  });
}
