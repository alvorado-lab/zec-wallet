/// Persists the transparent-funds POLICY toggles (§3.2i-3) across launches:
/// the expert gate ("Advanced: transparent funds") and the auto-shield switch.
///
/// WHY host-side flags and NOT Rust state: like [OnboardingStore]'s
/// backup-confirmed gate, these are pure host-UX policy booleans — the SDK
/// owns wallet/crypto/message data (design invariant 1), while "which
/// affordances this user asked to see" is the same category as the
/// theme/text-scale rendering prefs. No secret is ever stored.
///
/// FAIL-SAFE DIRECTIONS (each flag has its own honest default):
///  - Expert gate: unknown/unset reads `false` — the gate stays CLOSED, the
///    everyday surface stays clean (§3.2i-3 default posture).
///  - Auto-shield: unknown/unset reads `true` — the maintainer default posture is
///    "everything shielded"; only an EXPLICIT expert opt-out holds a
///    transparent balance. (The loop still refuses to act while the flag is
///    LOADING — see `walletAutoShieldEnabledProvider` — so a slow read can
///    never race an expert's persisted OFF into an unwanted shield.)
///
/// DEVICE-GLOBAL BY DEFAULT, IDENTITY-NAMESPACEABLE (security review
/// MINOR-5 / board A4): with the default constructor these flags are shared
/// across every wallet identity on the device — like the theme prefs. Two
/// consequences a multi-identity (duress/decoy) host must weigh:
/// (a) a decoy identity renders the owner's real posture — "expert ON +
/// auto-shield OFF" in a supposedly-casual decoy is a sophistication
/// fingerprint; (b) whoever holds the device in ANY identity can flip
/// auto-shield OFF, and every identity's future arrivals then stay public
/// (an explicit OFF shows no cue, by design). A duress-capable host closes both
/// by overriding [walletSettingsStoreProvider] with a per-identity store —
/// `SharedPrefsWalletSettingsStore(namespace: activeIdentityId)` — so each
/// identity reads and writes its OWN flags and can never see or flip another's
/// posture. See that constructor for the namespace contract; the package
/// default (`namespace: null`) favors the single-identity common case and stays
/// byte-compatible with every prior release.
///
/// FORENSIC POSTURE (a duress host MUST understand — review): these flags
/// persist in PLAINTEXT shared_preferences and the namespace becomes part of the
/// on-disk key. So:
///  - Pass an OPAQUE, HIGH-ENTROPY discriminator (a per-identity random token or
///    a keyed hash) — NOT a guessable DB row-id or a human-meaningful name. A
///    device seizure reads these key strings; the SDK does not hash them for you.
///  - Even opaque, the COUNT of namespaced entries reveals HOW MANY identities
///    ever toggled a policy. An identity that never changes a policy writes NO
///    key — the only fully deniable state.
///  - On adopting namespacing call [SharedPrefsWalletSettingsStore.clearDeviceGlobalFlags]
///    once: a pre-namespacing install left the owner's real posture in the legacy
///    keys, which a namespaced store neither reads nor clears (they would else sit
///    on disk forever). The owner's own PRIOR expert / auto-shield opt-ins also do
///    not migrate into their new namespace — they read the fail-safe defaults
///    (gate closed / shielded) after the switch.
///
/// A port (design invariant 2) so the controllers/sheets are host-VM testable
/// with an in-memory fake; production uses [SharedPrefsWalletSettingsStore].
library;

import 'package:shared_preferences/shared_preferences.dart';

abstract interface class WalletSettingsStore {
  /// Whether the expert "Advanced: transparent funds" gate is ON.
  /// `false` when unknown/unset (the gate stays closed).
  Future<bool> isExpertTransparentFunds();

  /// Persist the expert gate. Must DURABLY persist before it resolves; a
  /// write the platform reports as failed THROWS (never pretend it stuck).
  Future<void> setExpertTransparentFunds({required bool enabled});

  /// Whether auto-shield is ON. `true` when unknown/unset (the default
  /// posture: everything shielded).
  Future<bool> isAutoShieldEnabled();

  /// Persist the auto-shield switch. Same durability contract as
  /// [setExpertTransparentFunds].
  Future<void> setAutoShieldEnabled({required bool enabled});
}

/// The production [WalletSettingsStore], backed by shared_preferences (the
/// same rendering-prefs store the frame already uses — see
/// [SharedPrefsOnboardingStore] for the precedent and the durability rules).
///
/// IDENTITY NAMESPACING (board A4): pass a [namespace] — the host's active
/// identity discriminator — to give each identity its OWN copy of the flags.
/// With `namespace: null` (the default) the store is DEVICE-GLOBAL and uses the
/// bare [expertKey]/[autoShieldKey], byte-identical to every prior release, so
/// existing single-identity installs read their saved flags with no migration.
/// With a namespace, every key gains a per-identity segment so a decoy identity
/// reads the safe DEFAULT posture (gate closed / auto-shield on) instead of the
/// owner's real one, and no identity can flip another's policy.
///
/// The namespace is a NON-SECRET key segment (these are host-UX booleans, never
/// key material — §10 / design invariant 1), so any stable identity
/// discriminator works; an opaque id (or a hash of one) is preferred so the
/// key strings in the on-disk prefs don't themselves enumerate identities.
///
/// HOST WIRING: the namespace is fixed at construction, so re-key the store on
/// every identity switch — override [walletSettingsStoreProvider] with the new
/// id and `ref.invalidate` the two policy notifiers (or rebuild the
/// `ProviderScope`) so they re-read the switched-to identity's flags. The
/// package already asks a duress host to reset navigation on an identity flip
/// (see the swap-in guide §settings); this rides that same seam.
class SharedPrefsWalletSettingsStore implements WalletSettingsStore {
  /// [namespace] `null` ⇒ the DEVICE-GLOBAL store (the legacy keys). ANY
  /// non-null namespace — INCLUDING the empty string — ⇒ its OWN isolated
  /// keyspace. Empty is deliberately NOT collapsed to device-global: for the
  /// duress threat model the wrong fail direction is reading the OWNER's shared
  /// flags, so a host that momentarily passes `''` (an identity id that has not
  /// resolved yet during a switch window) reads the safe DEFAULTS from an
  /// isolated `wallet..transparentFunds.*` keyspace, never the owner's
  /// device-global posture (security review). Pass `null` (or use the
  /// default constructor) for the intentional device-global store.
  SharedPrefsWalletSettingsStore({String? namespace}) : _namespace = namespace;

  final String? _namespace;

  /// The flag SUFFIXES — the single source of truth for what each toggle is
  /// called, so the device-global and namespaced key forms can never drift.
  static const String _expertSuffix = 'transparentFunds.expert';
  static const String _autoShieldSuffix = 'transparentFunds.autoShield';

  /// The DEVICE-GLOBAL keys (the `namespace: null` form). Public + stable: a
  /// prior release wrote exactly these, and the existing tests pin them.
  /// Namespaced so they can never collide with the appearance/onboarding prefs
  /// in the same store.
  static const String expertKey = 'wallet.$_expertSuffix';
  static const String autoShieldKey = 'wallet.$_autoShieldSuffix';

  /// The one source of truth for the pref-key layout, used by both the instance
  /// reads/writes and the static teardown helpers so the namespaced format can
  /// never drift between them. The namespace occupies a FIXED positional segment
  /// between the `wallet.` prefix and the constant [suffix] tail, so two distinct
  /// namespaces can never collide, and a namespaced key (which always carries the
  /// extra segment) can never collide with a device-global one (which never
  /// does). CONTRACT: [suffix] MUST be one of the fixed suffix constants — never
  /// a caller- or host-controlled string — or the positional-collision guarantee
  /// no longer holds. `null` namespace ⇒ the bare `wallet.<suffix>` legacy key.
  static String _keyFor(String? namespace, String suffix) =>
      namespace == null ? 'wallet.$suffix' : 'wallet.$namespace.$suffix';

  String _key(String suffix) => _keyFor(_namespace, suffix);

  /// Delete the DEVICE-GLOBAL flags. A host adopting identity-namespacing should
  /// call this ONCE at migration time: a pre-namespacing install left the
  /// owner's real posture in the plaintext legacy keys, and those keys are
  /// neither read nor cleared by a namespaced store — so without this they sit
  /// on disk carrying the owner's fingerprint (a device-seizure forensic read),
  /// defeating the decoy the namespacing set up (security/reliability
  /// review). Idempotent; safe if the keys are already absent.
  static Future<void> clearDeviceGlobalFlags() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.remove(expertKey);
    await prefs.remove(autoShieldKey);
  }

  /// Delete one identity's namespaced flags — call on identity teardown so a
  /// retired/ephemeral identity leaves no per-identity keys accumulating in the
  /// (whole-file-rewritten) prefs store, and no residual posture on disk. Pass
  /// the SAME namespace the identity's store was constructed with. Idempotent.
  static Future<void> clearNamespace(String namespace) async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.remove(_keyFor(namespace, _expertSuffix));
    await prefs.remove(_keyFor(namespace, _autoShieldSuffix));
  }

  @override
  Future<bool> isExpertTransparentFunds() async {
    final prefs = await SharedPreferences.getInstance();
    return prefs.getBool(_key(_expertSuffix)) ?? false;
  }

  @override
  Future<void> setExpertTransparentFunds({required bool enabled}) async {
    final prefs = await SharedPreferences.getInstance();
    final ok = await prefs.setBool(_key(_expertSuffix), enabled);
    if (!ok) {
      // Payload-free (§5.4). The sheet catches this, keeps the switch at its
      // persisted value, and tells the user the save failed.
      throw Exception('transparent-funds expert flag write failed');
    }
  }

  @override
  Future<bool> isAutoShieldEnabled() async {
    final prefs = await SharedPreferences.getInstance();
    return prefs.getBool(_key(_autoShieldSuffix)) ?? true;
  }

  @override
  Future<void> setAutoShieldEnabled({required bool enabled}) async {
    final prefs = await SharedPreferences.getInstance();
    final ok = await prefs.setBool(_key(_autoShieldSuffix), enabled);
    if (!ok) {
      throw Exception('auto-shield flag write failed');
    }
  }
}
