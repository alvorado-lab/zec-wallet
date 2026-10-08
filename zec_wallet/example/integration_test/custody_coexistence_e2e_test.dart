import 'dart:io' show Platform;

import 'package:flutter/foundation.dart' show debugPrint;

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:path_provider/path_provider.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_example/core/ffi/wallet_ffi.dart';

/// FR-13 §8 device acceptance — `two_wallets_at_distinct_db_dirs_retain_custody
/// _across_switches` on THIS device's REAL vault (the Android-Keystore leg the
/// host + macOS gates could not cover: distinct namespaced aliases, the
/// per-namespace generation scan, and a purge scoped to exactly one namespace,
/// all against real hardware).
///
/// The proof, platform-honest: create wallet A, then wallet B, then reopen A —
/// A must open AND serve its ORIGINAL receive UA (the same seed unsealed, not
/// merely "no error"). That reopen re-enacts the APPLE pre-FR-13 hazard (the
/// fixed-account delete-first store, where B's create overwrote A's item →
/// `WrapArtifactInvalid`); pre-FR-13 ANDROID would have PASSED it (per-store
/// generations, artifact-pinned loads). The ANDROID regression teeth are the
/// FR-14 compose leg: the loud-sever `wipe` of B must destroy exactly ONE
/// wallet's namespace — an un-namespaced (or namespace-collapsed) purge scan
/// severs EVERY wallet's wrap key — and A still opens afterwards.
///
/// Both wallets live in THROWAWAY sibling leaves (`e2e_fr13_wallet_a`/`_b`) —
/// never the example's production `zec_wallet` leaf — and are crypto-shredded
/// at the end even on assertion failure (each keychain namespace derives from
/// its throwaway dir, so the test can never touch production custody; on Apple
/// the shred ALSO best-effort-sweeps the bare pre-FR-13 legacy item, which is
/// non-production residue by the no-migration decision). §5.4: addresses
/// are compared in memory and asserted as booleans, so no UA reaches logs even
/// on a FAILING expectation.
void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  // same static-link loader handling as the app (see core/ffi/wallet_ffi.dart)
  setUpAll(() async => RustLib.init(externalLibrary: walletExternalLibrary()));

  testWidgets('fr13_two_wallets_retain_custody_across_switches', (
    tester,
  ) async {
    final support = await getApplicationSupportDirectory();
    WalletConfig configFor(String leaf) => WalletConfig(
      dbDir: '${support.path}/$leaf',
      network: Network.main,
      endpointUrl: 'https://zec.rocks:443',
      tor: const TorPolicy.off(),
      seedPersistence: SeedPersistence.sealedKeychain,
      birthdayHeight: null,
      broadcastJitter: const JitterPolicy.none(),
      machineMemoPrefixes: const [], // FR-27 opt-in: no read scope here
    );
    final configA = configFor('e2e_fr13_wallet_a');
    final configB = configFor('e2e_fr13_wallet_b');

    // Idempotent re-entry: shred an aborted prior run's remnants first.
    for (final config in [configA, configB]) {
      if (await WalletHandle.walletExists(config: config)) {
        await WalletHandle.wipeForce(config: config);
      }
    }
    try {
      // Provision A, remember its default UA (in memory only), release it.
      final a1 = await WalletHandle.createGenerated(config: configA);
      final uaA = await a1.currentAddress();
      await a1.close();

      // Provision B — on Apple, pre-FR-13, THIS store overwrote A's
      // fixed-account wrap-key item.
      final b1 = await WalletHandle.createGenerated(config: configB);
      final uaB = await b1.currentAddress();
      await b1.close();
      expect(
        uaB != uaA,
        isTrue,
        reason: 'two generated wallets must hold distinct seeds',
      );

      // THE FR-13 acceptance: A reopens after B's intervening create, and
      // unseals the SAME seed (its original UA), with no rescan/re-provision.
      // (Apple-hazard re-enactment; the Android teeth are the shred leg below.)
      final a2 = await WalletHandle.open(config: configA);
      expect(
        await a2.currentAddress() == uaA,
        isTrue,
        reason:
            "A must reopen with ITS OWN seed after B's create "
            '(pre-FR-13 Apple: WrapArtifactInvalid — the overwrite hazard)',
      );
      await a2.close();

      // Full switch cycle A→B→A: both retain custody across switches.
      final b2 = await WalletHandle.open(config: configB);
      expect(
        await b2.currentAddress() == uaB,
        isTrue,
        reason: 'B retains custody',
      );
      await b2.close();
      final a3 = await WalletHandle.open(config: configA);
      expect(
        await a3.currentAddress() == uaA,
        isTrue,
        reason: 'A retains custody across the full switch cycle',
      );
      await a3.close();

      // Both wallets hold independent REAL custody — tier per platform.
      // (Record lands in the device log via adb logcat, §5.4-clean.)
      for (final (label, config) in [('A', configA), ('B', configB)]) {
        final custody = await WalletHandle.custodyDisclosure(config: config);
        debugPrint(
          'fr13_e2e: wallet=$label tier=${custody.tier} '
          'degraded=${custody.degraded}',
        );
        if (Platform.isAndroid) {
          expect(
            ['strongbox', 'tee'],
            contains(custody.tier),
            reason:
                'physical Android device must measure a hardware tier '
                'for wallet $label (got ${custody.tier})',
          );
          expect(custody.degraded, isFalse);
        } else if (Platform.isIOS) {
          expect(custody.tier, 'apple_secure_enclave');
          expect(custody.degraded, isFalse);
        } else if (Platform.isMacOS) {
          expect([
            'apple_keychain',
            'apple_secure_enclave',
          ], contains(custody.tier));
          expect(custody.degraded, isFalse);
        }
      }

      // FR-14 compose — THE Android leg: the plain (loud) wipe verify-real-
      // severs B's namespace (a zero-sever fails typed, so "the shred really
      // happened" is asserted, not assumed) — and A survives it (a namespace
      // collapse would sever A's wrap key right here).
      await WalletHandle.wipe(config: configB);
      expect(
        await WalletHandle.walletExists(config: configB),
        isFalse,
        reason: 'B is crypto-shredded',
      );
      final a4 = await WalletHandle.open(config: configA);
      expect(
        await a4.currentAddress() == uaA,
        isTrue,
        reason: "A must survive B's crypto-shred (namespace-scoped purge)",
      );
      await a4.close();

      await WalletHandle.wipe(config: configA);
      expect(
        await WalletHandle.walletExists(config: configA),
        isFalse,
        reason: 'both throwaways are crypto-shredded before the test ends',
      );
    } finally {
      // Backstop shred for the assertion-failure paths above; catch-ALL so no
      // cleanup fault of any type can replace the real test failure.
      for (final config in [configA, configB]) {
        try {
          if (await WalletHandle.walletExists(config: config)) {
            await WalletHandle.wipeForce(config: config);
          }
        } catch (e) {
          debugPrint('fr13_e2e: backstop wipe failed: $e');
        }
      }
    }
  });
}
