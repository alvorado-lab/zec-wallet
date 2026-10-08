import 'dart:typed_data' show Uint8List;

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/frb_wallet_provisioner.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/screen_security_channel.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/shared_prefs_onboarding_store.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/wallet_provisioner.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_composition.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_config.dart';
import 'package:zec_wallet/zec_wallet.dart';

/// The wallet onboarding COMPOSITION ROOT (spec §3.2g iii-B-2-b) — the co-wiring
/// invariant (crypto audit H2): a real seed-revealing provisioner can never ship
/// without the screenshot protection where the platform supports a block. The
/// invariant is a pure predicate so BOTH the holds and the violation are
/// directly asserted (an assert can only prove the happy path).
void main() {
  group('screenSecurityCoWiringHolds (crypto audit H2)', () {
    test('real provisioner + block-capable + Noop security = VIOLATION', () {
      // The exact mis-wiring the assertion exists to catch: a real provisioner
      // shipped with the no-op adapter on Android would show the seed without
      // FLAG_SECURE.
      expect(
        screenSecurityCoWiringHolds(
          provisionerIsReal: true,
          security: const NoopScreenSecurity(),
          supportsBlock: true,
        ),
        isFalse,
      );
    });

    test('real provisioner + block-capable + real adapter = holds', () {
      expect(
        screenSecurityCoWiringHolds(
          provisionerIsReal: true,
          security: MethodChannelScreenSecurity(),
          supportsBlock: true,
        ),
        isTrue,
      );
    });

    test('sees THROUGH the refcount decorator — a wrapped Noop is still a '
        'VIOLATION (#344)', () {
      // The seam refcount-wraps every adapter; the co-wiring check must unwrap,
      // or a mis-wired Noop would hide behind the decorator and read as safe.
      expect(
        screenSecurityCoWiringHolds(
          provisionerIsReal: true,
          security: RefCountedScreenSecurity(const NoopScreenSecurity()),
          supportsBlock: true,
        ),
        isFalse,
      );
      expect(
        screenSecurityCoWiringHolds(
          provisionerIsReal: true,
          security: RefCountedScreenSecurity(MethodChannelScreenSecurity()),
          supportsBlock: true,
        ),
        isTrue,
      );
      // Idempotent under an unexpected double wrap — a nested Noop is still a
      // VIOLATION (the unwrap peels every layer).
      expect(
        screenSecurityCoWiringHolds(
          provisionerIsReal: true,
          security: RefCountedScreenSecurity(
            RefCountedScreenSecurity(const NoopScreenSecurity()),
          ),
          supportsBlock: true,
        ),
        isFalse,
      );
    });

    test('real provisioner + no native block = holds (honest Noop)', () {
      // iOS / desktop / web: no FLAG_SECURE equivalent, so the honest Noop is
      // correct and the rule holds (the auto-hide-on-background covers them).
      expect(
        screenSecurityCoWiringHolds(
          provisionerIsReal: true,
          security: const NoopScreenSecurity(),
          supportsBlock: false,
        ),
        isTrue,
      );
    });

    test('no real provisioner = holds (no seed access, no requirement)', () {
      expect(
        screenSecurityCoWiringHolds(
          provisionerIsReal: false,
          security: const NoopScreenSecurity(),
          supportsBlock: true,
        ),
        isTrue,
      );
    });
  });

  group('screenshotBlockActive (the debug-deferral gate)', () {
    test('release Android wires the real FLAG_SECURE block', () {
      expect(
        screenshotBlockActive(isAndroidNative: true, isDebug: false),
        isTrue,
      );
    });

    test('DEBUG Android DEFERS the block (founder-authorized — debugging)', () {
      // FLAG_SECURE blacks out scrcpy/screenshots of the backup screen; a debug
      // build defers it. kDebugMode is compile-time false in release, so every
      // shipped artifact still protects the seed.
      expect(
        screenshotBlockActive(isAndroidNative: true, isDebug: true),
        isFalse,
      );
    });

    test('non-Android never blocks regardless of build mode', () {
      expect(
        screenshotBlockActive(isAndroidNative: false, isDebug: false),
        isFalse,
      );
      expect(
        screenshotBlockActive(isAndroidNative: false, isDebug: true),
        isFalse,
      );
    });
  });

  group('screenSecurityFor', () {
    test('block-capable platform → the real Android channel adapter', () {
      expect(
        screenSecurityFor(supportsNativeBlock: true),
        isA<MethodChannelScreenSecurity>(),
      );
    });

    test('no native block → the honest NoopScreenSecurity', () {
      expect(
        screenSecurityFor(supportsNativeBlock: false),
        isA<NoopScreenSecurity>(),
      );
    });
  });

  group('walletOnboardingOverrides', () {
    test(
      'co-wires the real provisioner, store, and (Android) screen security',
      () {
        final container = ProviderContainer(
          overrides: walletOnboardingOverrides(
            config: buildWalletConfig(dbDir: '/tmp/zec'),
            supportsScreenshotBlock: true,
          ),
        );
        addTearDown(container.dispose);
        expect(
          container.read(walletProvisionerProvider),
          isA<FrbWalletProvisioner>(),
        );
        expect(
          container.read(onboardingStoreProvider),
          isA<SharedPrefsOnboardingStore>(),
        );
        // The real provisioner is co-wired with a NON-no-op screen-security
        // adapter (the H2 invariant, end-to-end through the override list). The
        // seam refcount-wraps it (#344), so the effective adapter is read
        // THROUGH the decorator's `inner`.
        final security = container.read(screenSecurityProvider);
        expect(security, isA<RefCountedScreenSecurity>());
        expect(
          (security as RefCountedScreenSecurity).inner,
          isA<MethodChannelScreenSecurity>(),
        );
      },
    );

    test('uses the honest Noop where the platform cannot block the screen', () {
      final container = ProviderContainer(
        overrides: walletOnboardingOverrides(
          config: buildWalletConfig(dbDir: '/tmp/zec'),
          supportsScreenshotBlock: false,
        ),
      );
      addTearDown(container.dispose);
      final security = container.read(screenSecurityProvider);
      expect(security, isA<RefCountedScreenSecurity>());
      expect(
        (security as RefCountedScreenSecurity).inner,
        isA<NoopScreenSecurity>(),
      );
    });

    test('accepts a host-OWNED WalletConfig, not just the reference policy '
        '(the 3rd-party seam)', () {
      // A production host brings its own endpoint / network / Tor / jitter —
      // none of the reference constants. The seam must wire it verbatim.
      final hostConfig = WalletConfig(
        dbDir: '/tmp/host-zec',
        network: Network.test,
        endpointUrl: 'https://lightwalletd.my-host.example',
        tor: const TorPolicy.off(),
        seedPersistence: SeedPersistence.sealedKeychain,
        birthdayHeight: null,
        broadcastJitter: const JitterPolicy.none(),
        // FR-27: a host bringing its own machine-memo envelope registers its
        // read scope on the SAME config. It must reach the adapter verbatim
        // like every other field — a seam that dropped it would leave
        // `machineMemos` closed and the host reading "no memo" forever.
        machineMemoPrefixes: [
          Uint8List.fromList([0x52, 0x4C, 0x4D, 0x01]),
        ],
      );
      final container = ProviderContainer(
        overrides: walletOnboardingOverrides(
          config: hostConfig,
          supportsScreenshotBlock: true,
        ),
      );
      addTearDown(container.dispose);
      // The host's config must reach the adapter VERBATIM (generated ==) —
      // not just "a real provisioner wired": a seam that quietly rebuilt a
      // reference config would pass an isA check but fail this one.
      final provisioner =
          container.read(walletProvisionerProvider)! as FrbWalletProvisioner;
      expect(provisioner.config, hostConfig);
      // the sync sheet's "Server" row is wired from the SAME config the
      // provisioner opens with (host only — never the full URL), so the
      // display and the actual connection cannot drift.
      expect(
        container.read(walletEndpointHostProvider),
        'lightwalletd.my-host.example',
      );
    });

    test('rejects an EMPTY or RELATIVE dbDir LOUD at wiring time', () {
      // On desktop a relative dbDir would pin the wallet to the launch cwd —
      // a funded wallet "vanishes" when launched from elsewhere and onboarding
      // offers a fresh create over it. A programming error, thrown as one.
      WalletConfig withDir(String dbDir) => WalletConfig(
        dbDir: dbDir,
        network: Network.test,
        endpointUrl: 'https://lightwalletd.my-host.example',
        tor: const TorPolicy.off(),
        seedPersistence: SeedPersistence.sealedKeychain,
        birthdayHeight: null,
        broadcastJitter: const JitterPolicy.none(),
        machineMemoPrefixes: const [],
      );
      expect(
        () => walletOnboardingOverrides(config: withDir('')),
        throwsArgumentError,
      );
      expect(
        () => walletOnboardingOverrides(config: withDir('relative/wallet')),
        throwsArgumentError,
      );
      // The absolute form wires fine (proven by the sibling tests).
    });

    test('decorateProvisioner receives the REAL provisioner and its result is '
        'the one wired', () {
      WalletProvisioner? given;
      final decorated = _Decorated();
      final container = ProviderContainer(
        overrides: walletOnboardingOverrides(
          config: buildWalletConfig(dbDir: '/tmp/zec'),
          supportsScreenshotBlock: true,
          decorateProvisioner: (real) {
            given = real;
            return decorated;
          },
        ),
      );
      addTearDown(container.dispose);
      expect(given, isA<FrbWalletProvisioner>());
      expect(container.read(walletProvisionerProvider), same(decorated));
    });
  });
}

/// A decorator stand-in: only its identity is under test.
class _Decorated implements WalletProvisioner {
  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}
