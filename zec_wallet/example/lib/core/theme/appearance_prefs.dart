import 'dart:io' show Platform;

import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// In-app appearance preferences (Settings → Appearance) that sit
/// alongside [themeModeProvider](theme_mode_provider.dart): an app-wide
/// text-size multiplier and an AMOLED true-black dark variant. Both are
/// rendering preferences (SharedPreferences, Dart-side) — no Rust/FFI,
/// no app data. App-frame spec §2.1/§7.
///
/// RULE (review gate): SharedPreferences in this app stays RENDERING
/// PREFERENCES ONLY. Application data — addresses, wallet/balance state,
/// session tokens, "user confirmed backup"-class security steps — MUST
/// NOT be added here: that is Rust-owned state behind FFI (design
/// invariant 1). A new pref that is not a cosmetic display setting needs
/// a spec section + arch review before landing.
///
/// THE TWO RECORDED EXCEPTIONS. The first (arch review of FR-5 C3c;
/// `docs/specs/tor-plugin.md` §2.3): `use_tor_plugin`
/// (`core/tor/tor_plugin.dart`) — the example's choice of transport. It holds
/// no wallet state and no secret; it is read BEFORE the wallet exists, so it
/// cannot live behind the wallet's FFI. Its risk, accepted: the store can be
/// lost or restored apart from the wallet, which reads as OFF — visibly, in the
/// settings switch, and the boot fails loudly when the store cannot be READ.
/// The second (stage S5 `row`; app-frame spec §2.1): `device_log_level`
/// (`core/logging/device_log_prefs.dart`) — the user's device-log level, read
/// straight after the FFI init for the same reason. Unlike the Tor choice, an
/// unreadable store reads as "never chosen" and the boot goes on: it is a
/// diagnostic, and it must never cost the wallet its start. Nothing else
/// non-cosmetic joins them without the same review.

// --- Text size -------------------------------------------------------

const textScalePrefsKey = 'text_scale';

/// Bounds for the in-app text-size multiplier. Clamped so layouts never
/// break: 0.85× keeps small text legible; 1.4× is a meaningful bump
/// without overflowing dense rows (the OS Dynamic Type setting can push
/// higher and still composes on top — see [effectiveTextScale]).
const double minTextScale = 0.85;
const double maxTextScale = 1.40;
const double defaultTextScale = 1.0;

/// iOS HIG body type (~17 pt) runs larger than Material's 14 sp baseline,
/// so we floor the effective scale at 1.12× on iOS before the user
/// multiplier is applied. Clamp — not multiply — preserves Dynamic-Type
/// users: someone already at an accessibility size stays at their
/// preference instead of being over-scaled.
const double iosHigMinTextScale = 1.12;

/// Absolute safety bounds on the FINAL effective text scale (OS × user),
/// so no combination can break layouts. Verified at both extremes by
/// `text_scale_composition_clamped_at_extremes`.
const double minEffectiveTextScale = 0.8;
const double maxEffectiveTextScale = 2.0;

/// THE text-scale composition (single source of truth — the MaterialApp
/// builder calls this; the named test gates it): OS Dynamic Type factor,
/// then the iOS HIG min clamp, then the user's in-app multiplier, then
/// the absolute safety clamp.
double effectiveTextScale({
  required double osFactor,
  required double userScale,
  bool? isIos,
}) {
  var factor = osFactor;
  final ios = isIos ?? (!kIsWeb && Platform.isIOS);
  if (ios && factor < iosHigMinTextScale) {
    factor = iosHigMinTextScale;
  }
  return (factor * userScale).clamp(
    minEffectiveTextScale,
    maxEffectiveTextScale,
  );
}

/// Loaded before `runApp` + injected via override for a flash-free start.
final initialTextScaleProvider = Provider<double>((ref) => defaultTextScale);

double textScaleFromPrefs(SharedPreferences prefs) {
  final v = prefs.getDouble(textScalePrefsKey) ?? defaultTextScale;
  return v.clamp(minTextScale, maxTextScale);
}

class TextScaleNotifier extends Notifier<double> {
  @override
  double build() => ref.read(initialTextScaleProvider);

  Future<void> set(double value) async {
    final clamped = value.clamp(minTextScale, maxTextScale);
    if (clamped == state) return;
    state = clamped;
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setDouble(textScalePrefsKey, clamped);
    } catch (_) {
      // Best-effort persist; keep the in-session value (spec §6).
    }
  }
}

final textScaleProvider = NotifierProvider<TextScaleNotifier, double>(
  TextScaleNotifier.new,
);

// --- AMOLED true-black dark -----------------------------------------

const amoledPrefsKey = 'amoled_dark';

/// Loaded before `runApp` + injected via override for a flash-free start.
final initialAmoledProvider = Provider<bool>((ref) => false);

class AmoledNotifier extends Notifier<bool> {
  @override
  bool build() => ref.read(initialAmoledProvider);

  Future<void> set(bool value) async {
    if (value == state) return;
    state = value;
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setBool(amoledPrefsKey, value);
    } catch (_) {
      // Best-effort persist; keep the in-session value (spec §6).
    }
  }
}

/// True-black dark variant toggle. Only affects the dark theme.
final amoledProvider = NotifierProvider<AmoledNotifier, bool>(
  AmoledNotifier.new,
);
