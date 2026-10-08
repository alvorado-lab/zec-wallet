import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'swap_activation.dart';
import 'swap_config.dart';

/// The host's OWN swap kill-state (spec §3.5 layer 2 — runtime construction): the
/// single Dart-side SSOT the swap UI and the status notifier read. `true` ⇔ the
/// swap on-ramp is LIVE for this build/launch; `false` ⇔ swap is off — not
/// configured, not compiled in, host-killed, or the enable failed.
///
/// DERIVED from [swapActivationProvider] (the real `enableNearSwap` OUTCOME), not
/// a manually-set bool — money-honest BY CONSTRUCTION: the swap surface appears
/// iff the provider genuinely constructed (a successful enable), never because a
/// flag was flipped out of step with whether swap actually works. While the
/// activation is in flight, or if it errored, this reads `false` (no surface
/// until swap is confirmed live — honest, never an optimistic flash).
///
/// WHY a host provider (not read from the SDK stream): a killed swap has **no
/// wire signal** — the SDK's status stream simply stops (a pre-terminal
/// completion; the D-2a `SwapStatusNotifier` B1 contract), so the host must render
/// "tracking unavailable" from its OWN state (§3.5). This provider IS that state.
/// It gates the wallet-surface swap entry (a host without live swap shows no swap
/// UI — "remove the feature" = hiding one button, §3.5 UI isolation) AND drives
/// the status notifier's teardown-on-kill: when this flips `false`, the
/// non-autoDispose `swapStatusProvider` re-builds and cancels its poll (it would
/// otherwise keep polling after the screen stops watching — the D-2b-2 leak).
///
/// Composition: the activation provider is wired at the composition root
/// ([walletSwapOverrides] via [swapHostPolicyProvider]); this gate goes live with
/// zero rework. Tests override THIS provider directly to drive the on/off/kill
/// states without the activation chain.
final swapEnabledProvider = Provider<bool>((ref) {
  final activation = ref.watch(swapActivationProvider);
  // ONLY a resolved `data(true)` opens the gate — loading and error both read
  // false (no surface until swap is confirmed live; honest, never optimistic).
  return activation is AsyncData<bool> && activation.value;
});

/// The wallet action row's third-slot LAYOUT state (#356-F3) — a presentation
/// companion to [swapEnabledProvider], NOT a second gate. The gate stays the
/// SSOT for whether any swap surface may act; this only answers "should the
/// action row hold space for one".
///
/// WHY: the activation resolves asynchronously AFTER the active wallet surface
/// first renders, so on a policy-enabled host the Swap button used to POP into
/// the row and shift Send/Receive under the user's finger — a mid-shift tap
/// meant for Receive could land on Swap. Reserving
/// the slot while the outcome is pending keeps Send/Receive fixed without
/// showing an optimistic surface (the [swapEnabledProvider] honesty rule: no
/// Swap BUTTON until swap is confirmed live — a blank slot promises nothing).
enum SwapSlot {
  /// No third slot at all: the host never turned swap on
  /// ([swapHostPolicyProvider] null / `enabled: false`) — the §3.5 "remove the
  /// feature" posture — or the activation RESOLVED to off (built without the
  /// adapter, enable failed). The row lays out as Send/Receive only.
  absent,

  /// Policy says swap is wanted but the enable outcome isn't known yet (the
  /// activation is in flight, including a session-flip re-fire). Hold an EMPTY
  /// equal-width slot: no button, no claim — just stable Send/Receive geometry
  /// for the moment the outcome lands.
  reserved,

  /// The activation resolved live — render the real Swap button in the slot
  /// it already occupies (no shift).
  live,
}

/// Derives [SwapSlot]: `live` comes from [swapEnabledProvider] itself (ONE
/// gate — a test/host that overrides the gate drives this too), and the
/// reserved/absent split comes from the host policy + the activation OUTCOME.
/// The only layout shift left is `reserved` → `absent` on a resolved-off
/// activation — rare (a misconfigured/failed enable), one-shot, and it settles
/// the row to its final truthful shape.
final swapSlotProvider = Provider<SwapSlot>((ref) {
  // The gate is the SSOT for "swap is live" — never re-derived here.
  if (ref.watch(swapEnabledProvider)) return SwapSlot.live;
  final policy = ref.watch(swapHostPolicyProvider);
  if (policy == null || !policy.enabled) return SwapSlot.absent;
  final activation = ref.watch(swapActivationProvider);
  // The gate read false with policy enabled: RESOLVED means the enable is
  // genuinely off (collapse the slot); anything else is still settling —
  // including a session-flip re-fire (an AsyncLoading carrying the previous
  // value), where a previously-LIVE button honestly disappears while the new
  // handle's enable runs but the row's geometry never moves. A re-fire
  // carrying a previous OFF outcome stays settled-absent instead (
  // reliability NIT): a resolved-off host must not flash an empty third slot
  // on every session reopen (rescan/recovery re-keys).
  return switch (activation) {
    AsyncData<bool>() || AsyncError<bool>() => SwapSlot.absent,
    AsyncValue<bool>(hasValue: true, value: false) => SwapSlot.absent,
    // A re-fire carrying a previous ERROR is equally settled-off: on a
    // LISTENED element (every real widget) the reload keeps hasError, so a
    // failed-enable host never re-flashes the empty slot on session re-keys.
    // (Defensive anyway — the activation never throws by construction; every
    // failure resolves to data(false).) Pinned in swap_slot_provider_test.
    AsyncValue<bool>(hasError: true) => SwapSlot.absent,
    _ => SwapSlot.reserved,
  };
});
