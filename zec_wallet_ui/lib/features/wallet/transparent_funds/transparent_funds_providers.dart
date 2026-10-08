/// The transparent-funds policy seams (§3.2i-3 / #328) — the expert gate, the
/// auto-shield switch, and the two host-wired policy inputs (threshold,
/// power-save). All host-overridable at the `ProviderScope` root, following
/// the `wallet_providers.dart` seam conventions.
library;

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../wallet_providers.dart';
import 'wallet_settings_store.dart';

/// HOST SEAM: where the transparent-funds toggles persist. The default is the
/// shared_preferences-backed store (host-UX booleans, never Rust state — see
/// [WalletSettingsStore]); tests and hosts with their own settings storage
/// override it.
///
/// DURESS/DECOY HOSTS (board A4): the default is DEVICE-GLOBAL, so a decoy
/// identity would render the owner's real posture and either identity could
/// flip the other's policy. The canonical fix derives the namespace from the
/// host's active-identity provider so the two policy notifiers below (which
/// `ref.watch` THIS provider in their `build`) rebuild automatically on a switch:
///
/// ```dart
/// walletSettingsStoreProvider.overrideWith(
///   (ref) => SharedPrefsWalletSettingsStore(
///     namespace: ref.watch(activeIdentityProvider),
///   ),
/// )
/// ```
///
/// Do NOT rely on `ref.invalidate`-ing the two notifiers as the switch
/// mechanism: if the store override is a FIXED value
/// (`overrideWithValue(SharedPrefsWalletSettingsStore(namespace: 'A'))`),
/// invalidating the notifiers just re-reads the SAME store → the decoy still
/// sees the owner's flags and NOTHING errors (review M1). The STORE the
/// provider yields must change (the `overrideWith`-derives-namespace form above,
/// or a scope rebuild). See the [SharedPrefsWalletSettingsStore] namespace +
/// forensic contract.
final walletSettingsStoreProvider = Provider<WalletSettingsStore>(
  (ref) => SharedPrefsWalletSettingsStore(),
);

/// The persisted EXPERT GATE — "Advanced: transparent funds", default OFF
/// (§3.2i-3 (b)). Gate ON reveals the auto-shield switch and the promoted
/// unshield entry in the transparent-funds sheet; it hides NOTHING that
/// existed before it (the Shield button, the balance transparency lines and
/// the overflow Move-to-transparent entry all stay). Hosts that render their
/// own settings switch either override [walletSettingsStoreProvider] or drive
/// this notifier's [WalletExpertTransparentFundsNotifier.set].
///
/// `AsyncValue`: the flag loads from disk; readers treat loading/error as
/// `false` (the gate stays closed — the fail-safe direction).
final walletExpertTransparentFundsProvider =
    AsyncNotifierProvider<WalletExpertTransparentFundsNotifier, bool>(
      WalletExpertTransparentFundsNotifier.new,
    );

class WalletExpertTransparentFundsNotifier extends AsyncNotifier<bool> {
  @override
  Future<bool> build() =>
      ref.watch(walletSettingsStoreProvider).isExpertTransparentFunds();

  /// Persist-then-publish: the switch only reflects a value that durably
  /// stuck. A failed write THROWS (the sheet reverts its switch and says the
  /// save failed) and the state keeps the last persisted value.
  Future<void> set({required bool enabled}) async {
    final store = ref.read(walletSettingsStoreProvider);
    await store.setExpertTransparentFunds(enabled: enabled);
    // Drop a STALE publish if the store was re-keyed to another identity while
    // this write was in flight (a duress flip mid-write): publishing `enabled`
    // onto the switched-to notifier would flash THIS identity's posture on the
    // OTHER identity's UI — the exact decoy leak namespacing prevents (
    // reliability review H1). `ref.mounted` first so a scope-rebuild teardown
    // returns before the throw; then the identical-store re-check catches an
    // override swap. Mirrors the auto-shield spend fence's identity re-check.
    if (!ref.mounted ||
        !identical(ref.read(walletSettingsStoreProvider), store)) {
      return;
    }
    state = AsyncData(enabled);
  }
}

/// The persisted AUTO-SHIELD switch — default ON (§3.2i-3 (a): the maintainer
/// posture is "everything shielded"; turning it OFF is the expert's deliberate
/// hold-transparent choice, only offered while the expert gate is ON).
///
/// `AsyncValue` semantics are LOAD-BEARING for the loop: while the flag is
/// still LOADING (`value == null`) the auto-shield controller refuses to act,
/// so a slow disk read can never race a persisted OFF into an unwanted shield.
/// Loading/error render as ON in the sheet (the shipped default) but never
/// FIRE as ON.
final walletAutoShieldEnabledProvider =
    AsyncNotifierProvider<WalletAutoShieldEnabledNotifier, bool>(
      WalletAutoShieldEnabledNotifier.new,
    );

class WalletAutoShieldEnabledNotifier extends AsyncNotifier<bool> {
  @override
  Future<bool> build() =>
      ref.watch(walletSettingsStoreProvider).isAutoShieldEnabled();

  /// Persist-then-publish — same contract as the expert gate's `set`, including
  /// the re-keyed-store guard that drops a stale publish onto a switched-to
  /// identity (reliability review H1).
  Future<void> set({required bool enabled}) async {
    final store = ref.read(walletSettingsStoreProvider);
    await store.setAutoShieldEnabled(enabled: enabled);
    if (!ref.mounted ||
        !identical(ref.read(walletSettingsStoreProvider), store)) {
      return;
    }
    state = AsyncData(enabled);
  }
}

/// The EFFECTIVE auto-shield automation story for CLAIM surfaces (#383
/// fold): host-supported AND user-enabled(-or-loading). Any surface that
/// CLAIMS automation to the user — today the move-to-transparent review's
/// "will be shielded right back" warning — must consult THIS, never the
/// persisted switch alone: a host that declared automatic spends unsupported
/// ([walletAutoShieldSupportedProvider] false) hides the toggle and the loop
/// never arms, so a claim gated only on the switch would assert automation
/// that structurally cannot run and direct the user to a control that no
/// longer exists. (The transparent-funds sheet composes its own arms — it
/// needs `supported` and `enabled` SEPARATELY for the toggle vs the claim,
/// and additionally silences its claim on a read ERROR.)
///
/// Loading/error read as the shipped default (ON) — but only when supported:
/// for the move-review warning, over-warning is the safe direction (
/// MAJOR-3), while under `supported == false` the honest value is
/// unconditionally OFF.
final walletAutoShieldEffectiveProvider = Provider<bool>((ref) {
  return ref.watch(walletAutoShieldSupportedProvider) &&
      (ref.watch(walletAutoShieldEnabledProvider).value ?? true);
});

/// The SDK's own shielding floor in zatoshis — a UI-side mirror of the core's
/// `SHIELDING_THRESHOLD_ZAT` (`proposeShield` quietly returns null below it,
/// whatever the host seam says). Rendered copy CLAMPS to this so a host
/// setting a lower threshold can't make the sheet promise a minimum the
/// engine won't honor (UX review m9). If the core floor ever moves,
/// update BOTH (the mismatch direction is copy-only, never money).
const int walletAutoShieldFloorZat = 100000;

/// HOST SEAM: the auto-shield THRESHOLD in zatoshis — the smallest transparent
/// balance the loop proposes into the shielded pool. Shipped default: 0.001
/// ZEC (100 000 zat — maintainer call), which equals the SDK's own shielding
/// floor, so a host can only effectively RAISE it: `proposeShield` quietly
/// returns null below the core floor regardless of this value.
final walletAutoShieldThresholdZatProvider = Provider<int>(
  (ref) => walletAutoShieldFloorZat,
);

/// HOST SEAM: whether the device is in an OS power-save mode (Android Battery
/// Saver / iOS Low Power Mode). While `true` the auto-shield loop DEFERS
/// (maintainer call): arrivals stay honestly visible as transparent and the
/// next normal-power evaluation shields them. The package deliberately takes
/// no battery-plugin dependency — the HOST wires the OS signal; the `false`
/// default is always correct on desktop and means "never defer" when unwired.
final walletPowerSaveActiveProvider = Provider<bool>((ref) => false);
