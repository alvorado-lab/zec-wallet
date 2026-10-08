import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../reveal_grant.dart';
import 'onboarding_providers.dart';

/// A redaction wrapper around the recovery words. The words are the whole
/// secret, and a host attaching a VALUE-logging [ProviderObserver] (a common
/// debug/analytics footgun — an observer imports nothing, so the barrel fence
/// below does not stop it) would otherwise receive `AsyncData([<24 words>])` and
/// could dump it. This box's [toString] elides the phrase, so an observer that
/// logs `newValue.toString()` captures nothing — the same "seed-bearing values
/// redact Debug" rule the FFI DTOs follow (§5.4). It defends the ACCIDENTAL log
/// path only; a host that deliberately reads [words] is already inside the trust
/// boundary (it could read the provisioner directly).
final class RedactedMnemonic {
  const RedactedMnemonic(this.words);

  /// The BIP39 recovery words, in index order — handed straight to the word
  /// grid for display and NEVER logged / persisted / copied (spec §10).
  final List<String> words;

  @override
  String toString() => 'RedactedMnemonic(${words.length} words, redacted)';
}

/// The seed-reveal provider, alone in its own file and deliberately NOT
/// barrel-exported: this is the seed's only sanctioned crossing into the UI,
/// and it must never be one autocomplete away from a host widget. Importing it
/// by path is still possible, but that is a conscious act a reviewer sees — a
/// stronger fence (the `lib/src` migration) is tracked in #318.

/// The recovery words for the backup screen — the SANCTIONED OUTBOUND key
/// crossing (spec §3.3), surfaced as an `autoDispose` future so the words are
/// dropped the instant the backup screen leaves the tree (key-residue
/// minimization; the §10 Dart exposure is kept as small as possible — Dart
/// memory cannot be zeroized, so the smallest possible lifetime is the
/// mitigation). NEVER parked in the long-lived [OnboardingController] state.
///
/// WHY a provider and not a direct `provisioner.revealMnemonic()` call in the
/// widget: `autoDispose` gives the words a lifetime bounded EXACTLY by the
/// backup screen's subscription — on confirm (→ active) or any navigation away,
/// the screen unmounts, the last listener goes, and Riverpod disposes the
/// cached `List<String>` (on the next microtask after the last listener
/// leaves). A `Future` held in widget state would instead live until the next
/// GC. NEVER add `ref.keepAlive()` here — it would defeat the §10
/// smallest-lifetime mitigation and park the seed indefinitely.
///
/// Reads the same [walletProvisionerProvider] the controller does (the single
/// provisioning seam). Throws [StateError] if no provisioner is wired — but the
/// backup screen is only ever rendered in [OnboardingAwaitingBackup], which is
/// unreachable without a non-null provisioner, so that path is defensive only.
final mnemonicRevealProvider = FutureProvider.autoDispose<RedactedMnemonic>(
  (ref) async {
    final provisioner = ref.watch(walletProvisionerProvider);
    if (provisioner == null) {
      // Unreachable in the awaiting-backup phase (the controller returns
      // Unavailable with no provisioner); fail loud rather than show empty words.
      throw StateError('mnemonicRevealProvider read with no provisioner');
    }
    return RedactedMnemonic(await provisioner.revealMnemonic());
  },
  // NO auto-retry on failure. Riverpod 3 re-runs a failed future by default;
  // for a seed reveal that would silently RE-UNSEAL the seed again and again
  // on a transient fault (a locked device) — extra key material through the
  // bridge and extra Dart-side residue (§10), for a spinner storm the user
  // never asked for. The reveal is unsealed EXACTLY once per deliberate act;
  // the backup/onboarding screens own the explicit user retry (which
  // invalidates this provider), so a re-unseal is always a user decision.
  retry: (retryCount, error) => null,
);

/// The Settings backup screen's reveal — [mnemonicRevealProvider]'s discipline
/// (autoDispose, no auto-retry, the redaction box), KEYED BY the [RevealGrant]
/// the screen's re-auth minted (S2 §3.4, R06: one grant type, two consumers —
/// this and the viewing-key export). The grant is checked immediately before
/// the unseal, and the provisioner is READ, not watched, so nothing but a
/// deliberate act under a current grant ever unseals the words. A void grant
/// throws [RevealGrantVoid] and unseals nothing. The onboarding backup step
/// keeps [mnemonicRevealProvider]: it runs seconds after the create, behind
/// the money-safety gate, with no re-auth to bind.
final grantedMnemonicRevealProvider = FutureProvider.autoDispose
    .family<RedactedMnemonic, RevealGrant>(
      (ref, grant) async {
        if (!revealGrantIsCurrentIn(ref, grant)) throw const RevealGrantVoid();
        final provisioner = ref.read(walletProvisionerProvider);
        if (provisioner == null) {
          // The backup screen renders managed-by-host before reading when no
          // provisioner is wired; fail loud rather than show empty words.
          throw StateError(
            'grantedMnemonicRevealProvider read with no provisioner',
          );
        }
        return RedactedMnemonic(await provisioner.revealMnemonic());
      },
      // No auto-retry, for [mnemonicRevealProvider]'s reason.
      retry: (retryCount, error) => null,
    );
