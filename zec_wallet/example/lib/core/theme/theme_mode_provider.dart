import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// SharedPreferences key for the persisted theme mode.
const themeModePrefsKey = 'theme_mode';

/// Parse a stored theme-mode string into a [ThemeMode]. Unknown/absent
/// → [ThemeMode.system] (follow the OS), the app default. A corrupt
/// stored value is COSMETIC input — clamp to default, never throw
/// (app-frame spec §6).
ThemeMode themeModeFromString(String? s) => switch (s) {
  'light' => ThemeMode.light,
  'dark' => ThemeMode.dark,
  _ => ThemeMode.system,
};

/// The theme mode loaded from disk BEFORE `runApp`, injected via a
/// ProviderScope override in `main()` so the very first frame already
/// paints the user's chosen theme (no flash of the wrong theme on cold
/// start). Defaults to [ThemeMode.system] when not overridden (tests,
/// widget previews).
final initialThemeModeProvider = Provider<ThemeMode>((ref) => ThemeMode.system);

/// Persisted, user-selectable app theme mode (System / Light / Dark).
///
/// Theme is a *rendering preference*, not application data, so it lives
/// in SharedPreferences (Dart-side k/v) — this does NOT violate the
/// "Rust owns all state" invariant, which governs message/key/sync state
/// (app-frame spec §2.1). No Rust/FFI surface. RULE: only cosmetic
/// display settings may join this mechanism — see the file doc in
/// appearance_prefs.dart (the review gate for new prefs).
class ThemeModeNotifier extends Notifier<ThemeMode> {
  @override
  ThemeMode build() => ref.read(initialThemeModeProvider);

  /// Set + persist the mode. No-op if unchanged (avoids a redundant disk
  /// write + rebuild). The in-memory state updates immediately (instant
  /// UI); the persist is best-effort — a SharedPreferences failure is
  /// swallowed so the theme still switches for this session (it just
  /// won't survive a cold restart), rather than throwing out of a
  /// fire-and-forget onTap callback. Cosmetic-by-construction
  /// (app-frame spec §6).
  Future<void> set(ThemeMode mode) async {
    if (mode == state) return;
    state = mode;
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString(themeModePrefsKey, mode.name);
    } catch (_) {
      // Persist failed (rare) — keep the in-session selection.
    }
  }
}

final themeModeProvider = NotifierProvider<ThemeModeNotifier, ThemeMode>(
  ThemeModeNotifier.new,
);
