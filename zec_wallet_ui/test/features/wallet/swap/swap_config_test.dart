import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_config.dart';
import 'package:zec_wallet/zec_wallet.dart';

/// The swap host-policy boundary (gate 7 — every named constant pinned at its
/// boundary; no magic numbers). These are HOST decisions; the SDK validates but
/// does not choose them, so a drift here is a one-line, reviewable change caught
/// by a failing pin.
void main() {
  group('reference constants', () {
    test('the 1Click endpoint is the public production URL over TLS', () {
      expect(referenceSwapEndpoint, 'https://1click.chaindefuser.com');
      expect(
        referenceSwapEndpoint.startsWith('https://'),
        isTrue,
        reason: 'a plaintext provider endpoint would be a leak',
      );
    });
  });

  group('buildReferenceSwapPolicy', () {
    test('is ENABLED against the reference endpoint, no JWT, live kill', () {
      final p = buildReferenceSwapPolicy();
      expect(
        p.enabled,
        isTrue,
        reason: 'founder D-2b-2: attempt the real enable',
      );
      expect(p.config.endpoint, referenceSwapEndpoint);
      expect(p.config.jwt, isNull, reason: 'no provider credential shipped');
      expect(p.declaredKill, SwapKill.live);
    });
  });

  group('SwapHostPolicy value semantics', () {
    SwapHostPolicy base() => const SwapHostPolicy(
      config: SwapProviderConfig(endpoint: 'https://e'),
      enabled: true,
    );

    test('declaredKill defaults to live', () {
      expect(base().declaredKill, SwapKill.live);
    });

    test('== and hashCode track every field', () {
      expect(base(), equals(base()));
      expect(base().hashCode, base().hashCode);

      const offEndpoint = SwapHostPolicy(
        config: SwapProviderConfig(endpoint: 'https://other'),
        enabled: true,
      );
      const offEnabled = SwapHostPolicy(
        config: SwapProviderConfig(endpoint: 'https://e'),
        enabled: false,
      );
      const offKill = SwapHostPolicy(
        config: SwapProviderConfig(endpoint: 'https://e'),
        enabled: true,
        declaredKill: SwapKill.hard,
      );
      expect(base(), isNot(equals(offEndpoint)));
      expect(base(), isNot(equals(offEnabled)));
      expect(base(), isNot(equals(offKill)));
    });
  });

  group('walletSwapOverrides', () {
    test('wires the reference policy into swapHostPolicyProvider', () {
      final container = ProviderContainer(overrides: walletSwapOverrides());
      addTearDown(container.dispose);
      expect(
        container.read(swapHostPolicyProvider),
        equals(buildReferenceSwapPolicy()),
      );
    });

    test('without the override the policy seam defaults null (swap off)', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      expect(container.read(swapHostPolicyProvider), isNull);
    });
  });
}
