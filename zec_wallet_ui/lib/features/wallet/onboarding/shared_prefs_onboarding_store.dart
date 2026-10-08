import 'package:shared_preferences/shared_preferences.dart';

import 'onboarding_store.dart';

/// The production [OnboardingStore] (spec §3.2g iii-B-2-b): the money-safety
/// backup-confirmed gate flag, persisted via shared_preferences (the same
/// rendering-prefs store the frame already uses; a single boolean, never a
/// secret — see [OnboardingStore] for why this is host-UX state, not Rust-owned
/// wallet state).
///
/// DURABILITY (the crash-safe ordering depends on it): [setBackupConfirmed] does
/// not resolve until the platform write completes, and a write that the platform
/// reports as failed THROWS — so the controller treats a failed `true` write as
/// "not deposit-ready" (the gate stays CLOSED) and a failed `false` write as a
/// create that must not proceed. [isBackupConfirmed] reads `false` for an
/// unknown/unset key (fail-safe: an unreadable flag re-forces backup, never
/// skips it).
///
/// ⚠ MIGRATION TRIPWIRE (#356-F6): that durability is a property of the LEGACY
/// `SharedPreferences` API used below — on Android its `setBool` future
/// completes only after a synchronous-durable `commit()`, which is exactly what
/// the controller's write-false-BEFORE-create ordering (and the on-device
/// force-stop proof) rely on. Do NOT migrate this store to
/// `SharedPreferencesAsync`/`SharedPreferencesWithCache`: the Android backend
/// there is Jetpack DataStore, whose write acknowledgment is not the same
/// crash-durability contract, and the gate's crash ordering would silently
/// void while every happy-path test stays green. The store test pins this by
/// mocking ONLY the legacy platform channel — an API migration turns it red.
/// The durable long-term home for this flag is the Rust DB (identity-keyed),
/// tracked in the #356 basket.
class SharedPrefsOnboardingStore implements OnboardingStore {
  /// Namespaced so it can never collide with the appearance prefs in the same
  /// store. One wallet per device in this slice; an identity-keyed flag (for a
  /// future restore/re-create path) is manager-carried.
  static const String backupConfirmedKey = 'wallet.onboarding.backupConfirmed';

  /// The #390 post-restore note lifecycle key (a short string: 'pending' /
  /// 'done'; ABSENT ⇒ notApplicable). Namespaced under `wallet.onboarding.` like
  /// the backup gate; a non-secret host-UX string.
  static const String deepScanRestoreNoteKey =
      'wallet.onboarding.deepScanRestoreNote';

  // NOTE on the fail-safe direction: an unknown/lost key reads `false`
  // and re-forces backup. Since #356-F2 attached a destructive "Start over"
  // escape to that screen, the controller pairs the fail-safe with a
  // prior-activity probe (OnboardingAwaitingBackup.hasPriorActivity) so a
  // silently-reset prefs file over a FUNDED wallet never offers a
  // fresh-create-flavored delete.

  @override
  Future<bool> isBackupConfirmed() async {
    final prefs = await SharedPreferences.getInstance();
    // Unknown/unset → false (fail-safe: the gate stays closed).
    return prefs.getBool(backupConfirmedKey) ?? false;
  }

  @override
  Future<void> setBackupConfirmed({required bool confirmed}) async {
    final prefs = await SharedPreferences.getInstance();
    final ok = await prefs.setBool(backupConfirmedKey, confirmed);
    if (!ok) {
      // The platform reported the write as failed — do NOT pretend it persisted.
      // Payload-free (§5.4). The controller catches this and keeps the gate
      // closed (on a `true` write) or aborts the create (on a `false` write).
      throw Exception('onboarding backup-confirmed write failed');
    }
  }

  @override
  Future<DeepScanRestoreNoteState> deepScanRestoreNoteState() async {
    final prefs = await SharedPreferences.getInstance();
    // Unknown/unset/unrecognised → notApplicable (fail-safe: no note).
    return switch (prefs.getString(deepScanRestoreNoteKey)) {
      'pending' => DeepScanRestoreNoteState.pending,
      'done' => DeepScanRestoreNoteState.done,
      _ => DeepScanRestoreNoteState.notApplicable,
    };
  }

  @override
  Future<void> setDeepScanRestoreNoteState(
    DeepScanRestoreNoteState state,
  ) async {
    // Best-effort — NEVER throws (security MED-2): every call site is
    // fire-and-forget `unawaited` / an un-caught `acknowledge()`, so a rejected
    // `getInstance()` / channel write here would surface as an UNHANDLED async
    // error detached from the caller's try/catch. Swallow it: the note is a
    // nudge, not a money-safety gate, and the fail-safe direction (a lost write
    // ⇒ no note) holds. This makes the "never throws" contract structural.
    try {
      final prefs = await SharedPreferences.getInstance();
      // notApplicable REMOVES the key so a created wallet leaves no residue.
      switch (state) {
        case DeepScanRestoreNoteState.notApplicable:
          await prefs.remove(deepScanRestoreNoteKey);
        case DeepScanRestoreNoteState.pending:
          await prefs.setString(deepScanRestoreNoteKey, 'pending');
        case DeepScanRestoreNoteState.done:
          await prefs.setString(deepScanRestoreNoteKey, 'done');
      }
    } catch (_) {
      // Swallowed by contract (see above).
    }
  }
}
