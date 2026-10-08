/// Deterministic avatar colour derived from a seed string. Single source
/// of truth so every surface renders the same contact with the same
/// colour.
///
/// Divergence from the Mostpost source (app-frame spec §1): the seed is
/// a STABLE CONTACT IDENTIFIER (contact id / fingerprint display string
/// — fixed when Messaging UX lands), never an email and never a PII
/// requirement; Mostpost's multi-inbox `accountDotColorFor` is dropped
/// (a mail affordance).
library;

import 'package:flutter/material.dart';

import '../../core/theme/colors.dart';

/// Avatar IDENTITY colours — deliberately theme-independent literals, NOT
/// `WalletColors` token references: a contact must keep the same colour
/// across light/dark and across any brand re-skin (spec §11 A1), or the
/// colour stops being an identity cue. Five entries coincide with the
/// dark palette's status colours today; that is coincidence, not
/// coupling (arch review fold).
///
/// KNOWN LIMIT (code review fold): contrast on `light.bg` is poor
/// (≤3:1 for most entries) — before avatars render initials on a light
/// surface, Messaging UX must pick a light-optimised treatment (derive
/// the initial colour from fill luminance, or a ring instead of a fill).
const List<Color> _avatarPalette = [
  Color(0xFF4A7CFF),
  Color(0xFF22C58B),
  Color(0xFFFF9F43),
  Color(0xFFB45CFF),
  Color(0xFF00D4AA),
  Color(0xFFFF6B4A),
];

/// Picks a deterministic colour from the palette. Falls back to
/// [WalletColors.textMuted] when `seed` is empty (renders as a neutral
/// avatar rather than throwing on missing data).
Color avatarColorFor(String seed, WalletColors colors) {
  if (seed.isEmpty) return colors.textMuted;
  var hash = 0;
  for (final code in seed.codeUnits) {
    hash = (hash * 31 + code) & 0x7fffffff;
  }
  return _avatarPalette[hash % _avatarPalette.length];
}
