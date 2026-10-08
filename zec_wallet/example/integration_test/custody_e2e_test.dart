import 'dart:io' show Platform;

import 'package:flutter/foundation.dart' show debugPrint;

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_example/core/ffi/wallet_ffi.dart';

/// §4.3a device gate — `wrap_key_roundtrip_via_keystore` (wallet-sdk §8).
///
/// Runs the FULL custody round-trip against THIS device's real vault
/// (selftest-scoped identities — production custody untouched). On
/// Android this is ALSO the loader-namespace catch-point: a split
/// System.loadLibrary/dlopen mapping leaves the courier's context handoff
/// in the other copy of the library, and the run fails loudly with the
/// typed vault-absent error — never silently.
void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  // same static-link loader handling as the app (see core/ffi/wallet_ffi.dart)
  setUpAll(() async => RustLib.init(externalLibrary: walletExternalLibrary()));

  testWidgets('wrap_key_roundtrip_via_keystore', (tester) async {
    // iOS DEVICE note (spec §4.3a): the production vault stores the SE-wrapped
    // seal key in the data-protection keychain, which on a physical iPhone needs
    // a keychain-access-groups entitlement (else SecItemAdd → errSecMissingEntitlement).
    // The example Runner now ships ios/Runner/Runner.entitlements (the default app
    // group), so this succeeds on-device. The catch keeps a regression (entitlement
    // dropped / wrong signing) diagnosable, not a bridge-looking trace.
    late final CustodyReport report;
    try {
      report = await selftestSeedCustody();
    } on WalletApiError catch (e) {
      fail(
        'custody selftest failed before assertions: ${e.kind} '
        '(on a physical iOS device this usually means the example Runner '
        'lost its keychain-access-groups entitlement — see '
        'ios/Runner/Runner.entitlements, wallet-sdk spec §4.3a iOS device row)',
      );
    }
    // The gate record lands in the DEVICE log (adb logcat), not the test
    // runner's stdout — read it there (StrongBox vs TEE differs per
    // device; both 2026-06-12 devices measured strongbox).
    debugPrint('custody_e2e: tier=${report.tier} degraded=${report.degraded}');

    expect(
      report.roundtripOk,
      isTrue,
      reason: 'store→load must recover the exact payload',
    );
    expect(
      report.bindingRejectsTamperedBlob,
      isTrue,
      reason: 'the AAD binding must reject a tampered blob LOUDLY',
    );
    expect(
      report.wipeSevers,
      isTrue,
      reason: 'after wipe the artifact must open nothing (idempotent)',
    );

    if (Platform.isAndroid) {
      // Real hardware must land a hardware tier; software_keystore on a
      // physical device would mean the degraded path is wrongly taken —
      // and it MUST be surfaced as degraded if it ever happens.
      expect(
        ['strongbox', 'tee'],
        contains(report.tier),
        reason:
            'physical Android device must measure a hardware tier '
            '(got ${report.tier})',
      );
      expect(report.degraded, isFalse);
    } else if (Platform.isIOS) {
      // Every iPhone has a Secure Enclave and the production build ships NO
      // apple-insecure-raw-keychain-fallback feature, so the vault is the
      // AppleSecureEnclaveVault → the hardware SE tier (the wipe deletes the
      // SE key; ADR-0571). DEVICE-PROVEN on an iPhone 17 Pro / iOS 26.4.2:
      // tier=apple_secure_enclave, degraded=false, with round-trip + tamper-reject
      // + wipe-sever all green above.
      expect(
        report.tier,
        'apple_secure_enclave',
        reason:
            'a physical iPhone must measure the Secure-Enclave tier '
            '(got ${report.tier})',
      );
      expect(report.degraded, isFalse);
    } else if (Platform.isMacOS) {
      // Apple Silicon may resolve the Secure-Enclave adapter; either Apple tier
      // is a valid, non-degraded custody outcome.
      expect(
        ['apple_keychain', 'apple_secure_enclave'],
        contains(report.tier),
        reason: 'macOS must measure an Apple custody tier (got ${report.tier})',
      );
      expect(report.degraded, isFalse);
    }
  });
}
