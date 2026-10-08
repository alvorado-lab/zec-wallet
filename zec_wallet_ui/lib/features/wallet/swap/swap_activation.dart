import 'dart:async' show TimeoutException;

import 'package:flutter/foundation.dart' show debugPrint;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';

import '../wallet_providers.dart';
import 'swap_config.dart';

/// The one-time swap ACTIVATION (spec §3.5; the `enableNearSwap` on-ramp). Calls
/// `enableNearSwap` exactly once per open wallet handle and reports whether swap
/// is LIVE — the single source of truth the runtime gate ([swapEnabledProvider])
/// derives from.
///
/// WHY a provider (not a one-shot call in `main`): `enableNearSwap` requires an
/// OPEN wallet (a non-null [WalletSession], i.e. `OnboardingActive` — the
/// money-safety gate), which is reached asynchronously AFTER onboarding, not at
/// boot. Watching [walletSessionProvider] means the enable fires the moment a
/// wallet becomes available and RE-fires on a session close/reopen (a new handle
/// needs its own enable).
///
/// RE-FIRE SAFETY: this re-runs whenever a watched dependency changes —
/// [walletSessionProvider] (new handle) or [swapHostPolicyProvider]. Repeat/
/// concurrent `enableNearSwap` calls on the SAME handle are safe by SDK contract:
/// `Wallet::enable_swap` is set-once/first-wins via `OnceLock::get_or_init` (a
/// second call drops the new provider and returns the live service — no race, no
/// panic), so a mid-flight enable that is superseded by a re-fire cannot
/// double-construct. NOTE for a FUTURE dynamic policy source (the real app's
/// signed manifest, W5): a policy that FLICKERS would drive spurious
/// activation re-runs → `swapEnabledProvider` flutter → `SwapStatusNotifier`
/// teardown/rebuild churn; a dynamic source must emit a STABLE (debounced /
/// monotonic-off) policy. The reference policy here is a boot constant, so this
/// is inert today.
///
/// HONEST DEGRADATION (design principle 6): every failure mode resolves to `false`
/// (swap OFF), never an exception bubbling to the UI:
///  - no policy / `enabled == false` / no session → not attempted, `false`;
///  - `SwapDisabled` (built without the `swap-near` adapter) → `false`;
///  - `providerUnavailable` (bad endpoint/credential/transport) → `false`;
///  - `watchOnly` (#397 §3.7 D3 — a watch-only wallet structurally cannot
///    swap; typed RW-SWAP-015, never retryable) → `false`, composing with the
///    watch-only chrome that already hides the whole swap surface.
/// So the swap surface appears ONLY when the provider genuinely constructed —
/// the gate is the real enable OUTCOME, money-honest by construction.
///
/// §5.4 logging: only the coarse, stable error CODE is logged on a degrade
/// (never the endpoint, JWT, or any payload) — enough to diagnose "swap stayed
/// off because X" without leaking a NEVER-log value.
final swapActivationProvider = FutureProvider<bool>((ref) async {
  final session = ref.watch(walletSessionProvider);
  final policy = ref.watch(swapHostPolicyProvider);

  // Not configured, host-disabled, or no open wallet: swap is off, and we make
  // NO provider call (no network, no construction) — the honest no-op.
  if (session == null || policy == null || !policy.enabled) {
    return false;
  }

  try {
    // BOUNDED (UX MED): the enable does provider/network work, and an
    // unresolved outcome holds the action row's RESERVED third slot — a hung
    // transport must degrade to the truthful OFF (the same 30s budget every
    // boot stage rides), never an indefinite empty slot with swap silently
    // never arriving.
    await session
        .enableNearSwap(
          config: policy.config,
          // The guard above already returned for `!policy.enabled`, so this is
          // always `true` here — pass the constant so the intent (the provider,
          // not the SDK, does the on/off filtering) is explicit.
          swapEnabled: true,
          declaredKill: policy.declaredKill,
        )
        .timeout(const Duration(seconds: 30));
    return true;
  } on TimeoutException {
    // §5.4: fixed message only.
    debugPrint('swap enable degraded to off (enable timed out)');
    return false;
  } on SwapApiError catch (e) {
    // The expected degrade paths (no adapter compiled in → SwapDisabled; bad
    // endpoint/transport → providerUnavailable; watch-only wallet → watchOnly,
    // RW-SWAP-015). Swap stays off; surface nothing to the user beyond the
    // absent swap entry. Log the CODE only (§5.4).
    debugPrint('swap enable degraded to off: ${e.code}');
    return false;
  } catch (_) {
    // Defense-in-depth so "every failure mode resolves to off" is literally true:
    // an UNTYPED bridge failure (an FRB panic / generic error, outside the
    // documented typed contract) must still degrade to off — swap-OFF is always
    // the safe state (principle 6). Log a FIXED message only: an untyped error
    // may carry a payload, so it is NEVER interpolated (§5.4).
    debugPrint('swap enable degraded to off (untyped bridge error)');
    return false;
  }
});
