import 'package:flutter_riverpod/flutter_riverpod.dart';
// `Override` (the list element type) is surfaced from misc.dart in Riverpod 3.x.
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:zec_wallet/zec_wallet.dart';

/// The reference-app swap PROVIDER host-policy (spec §3.5; ADR-0014/0525) — the
/// `enableNearSwap` inputs: the 1Click endpoint, the optional provider JWT, the
/// host's on/off decision, and the §3.5 declared kill severity. Like
/// [buildWalletConfig] and [referenceSwapSlippageBps], these are HOST decisions:
/// the SDK validates the config (e.g. the endpoint URL at construction) but never
/// CHOOSES it. A production host typically derives its swap policy from its own
/// signed config; this file governs ONLY this example app.
///
/// WHY a value type (not loose args): the policy is the single seam the
/// composition root overrides ([swapHostPolicyProvider]) and the one the
/// activation provider reads — keeping the field list in one place (DRY) means a
/// future field (a per-asset cap, a manifest version) is a one-line, reviewable
/// edit pinned by [SwapHostPolicy]'s `==`/`hashCode` rather than threaded through
/// call sites.
class SwapHostPolicy {
  const SwapHostPolicy({
    required this.config,
    required this.enabled,
    this.declaredKill = SwapKill.live,
  });

  /// The §3.5 provider construction config (endpoint + optional JWT) handed to
  /// `enableNearSwap`. The `jwt` is a §5.4 NEVER-log value — the SDK wraps it
  /// `Zeroizing` at the boundary; keep its host-side residency minimal.
  final SwapProviderConfig config;

  /// The host's layer-1 swap decision (§3.5): `true` ⇔ the host WANTS swap on for
  /// this build (the activation provider will attempt `enableNearSwap`); `false` ⇔
  /// swap is held off by host policy and no enable is attempted. NOTE this is the
  /// host INTENT — whether swap actually goes LIVE additionally depends on the
  /// enable SUCCEEDING (a build without the `swap-near` adapter degrades to a
  /// typed `SwapDisabled`; a bad endpoint/credential → `providerUnavailable`), so
  /// the runtime gate ([swapEnabledProvider]) derives from the enable OUTCOME,
  /// not from this flag alone.
  final bool enabled;

  /// The §3.5 declared kill severity the host applies at construction (e.g.
  /// lowered from its signed manifest). `live` for normal operation; the SDK only
  /// ever escalates toward off, so a host can DECLARE `windDown`/`hard` but never
  /// resurrect a killed instance through this field.
  final SwapKill declaredKill;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is SwapHostPolicy &&
          config == other.config &&
          enabled == other.enabled &&
          declaredKill == other.declaredKill;

  @override
  int get hashCode => Object.hash(config, enabled, declaredKill);
}

/// The NEAR 1Click base URL the reference app points `enableNearSwap` at (the
/// public production endpoint; the SDK validates the URL at construction). A
/// named constant so it is pinned by a test (gate 7 — no magic numbers) and a
/// future change is a one-line reviewable edit.
const String referenceSwapEndpoint = 'https://1click.chaindefuser.com';

/// The 1Click integrator JWT, injected at BUILD TIME via
/// `--dart-define=ZEC_WALLET_SWAP_JWT=<token>` (never hardcoded / committed — it lives
/// in the builder's own untracked secrets). Empty by default ⇒ no JWT (quotes
/// against 1Click fail closed back to the form — no money risk). A §5.4 NEVER-log
/// value: it is handed straight to `SwapProviderConfig.jwt`, which the SDK wraps
/// `Zeroizing` at the FFI boundary. This `String.fromEnvironment` seam is the
/// example app's build-time injection ONLY; a production host would inject its
/// JWT from its own signed config.
const String _referenceSwapJwt = String.fromEnvironment('ZEC_WALLET_SWAP_JWT');

/// Build the reference-app swap policy. ENABLED against [referenceSwapEndpoint]
/// with the build-time [_referenceSwapJwt] (if supplied) and a `live` declared
/// kill — the composition attempts the real `enableNearSwap` at launch (maintainer
/// D-2b-2 directive: exercise the real enable path, not a held-off stub).
///
/// ON-DEVICE REALITY (verified): the `swap-near` adapter IS in the default build
/// (cargokit builds default features), so `enableNearSwap` SUCCEEDS and the swap
/// surface goes live — the Swap button appears on the wallet screen. With a JWT
/// (`--dart-define=ZEC_WALLET_SWAP_JWT=…`) real quotes work; WITHOUT one, a quote
/// against 1Click fails ("couldn't be processed") and routes back to the form.
/// Never a money risk — execute is gated behind a successful quote + the §2.6
/// disclosure acknowledgment.
///
/// The foreign-asset menu is now the live `/v0/tokens` registry itself (the
/// shared token picker over `swapListTokens()`), so the asset ids are sourced
/// from 1Click directly — no static curated list to verify against the registry
/// any more (resolved the former manager-carried D-2b-2 reconcile note).
SwapHostPolicy buildReferenceSwapPolicy() => SwapHostPolicy(
  config: SwapProviderConfig(
    endpoint: referenceSwapEndpoint,
    jwt: _referenceSwapJwt.isEmpty ? null : _referenceSwapJwt,
  ),
  enabled: true,
);

/// The host swap-policy seam (spec §3.5 layer 2 — runtime construction). Default
/// `null` ⇔ swap is NOT configured for this build (no enable attempt, no swap
/// surface) — the honest default exactly like [walletProvisionerProvider]
/// defaulting null. The composition root overrides it with
/// [buildReferenceSwapPolicy] (see [walletSwapOverrides]); tests override it
/// directly to drive the enable path on the host VM.
final swapHostPolicyProvider = Provider<SwapHostPolicy?>((ref) => null);

/// The swap composition-root overrides — the ONE place the reference-app swap
/// policy is wired into the provider graph. Spread alongside the onboarding
/// overrides in `main()`. Kept separate from `walletOnboardingOverrides` (which
/// owns the seed-revealing provisioner + screen-security co-wiring) because swap
/// is an independent seam with no key-material coupling: a build that wires
/// onboarding but not swap (or vice versa) is a valid configuration.
///
/// Wiring the policy is INDEPENDENT of the wallet FFI being ready: if the FFI did
/// not initialize, `walletSessionProvider` stays `null`, so [swapActivationProvider]
/// makes no enable call regardless. The activation fires only once a real wallet
/// session exists (the money-safety gate).
List<Override> walletSwapOverrides() => [
  swapHostPolicyProvider.overrideWithValue(buildReferenceSwapPolicy()),
];
