import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/colors.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';

/// WCAG contrast ratio. `computeLuminance()` implements the WCAG relative
/// luminance formula, so this is the normative AA computation.
double _ratio(Color a, Color b) {
  final la = a.computeLuminance();
  final lb = b.computeLuminance();
  final hi = math.max(la, lb);
  final lo = math.min(la, lb);
  return (hi + 0.05) / (lo + 0.05);
}

/// Gate one scheme (app-frame spec §8): the contrast CONTRACT is this
/// test, not the literals in colors.dart — a palette edit that breaks
/// readability fails CI loudly and comes to review with its WHY.
void _gateScheme(String name, WalletColors c) {
  final textTokens = <String, Color>{
    'text': c.text,
    'textMuted': c.textMuted,
    'textDim': c.textDim,
    'green': c.green,
    'orange': c.orange,
    'red': c.red,
    'purple': c.purple,
    'cyan': c.cyan,
  };
  final surfaces = <String, Color>{'bg': c.bg, 'bgCard': c.bgCard};

  // Small-text AA (≥4.5:1) for every text-bearing token on both surfaces.
  for (final MapEntry(key: tok, value: color) in textTokens.entries) {
    for (final MapEntry(key: surf, value: bg) in surfaces.entries) {
      final r = _ratio(color, bg);
      expect(
        r,
        greaterThanOrEqualTo(4.5),
        reason:
            '$name: $tok on $surf is ${r.toStringAsFixed(2)}:1 '
            '(< 4.5:1 small-text AA)',
      );
    }
  }

  // Message text on both bubble surfaces (retained chat-surface tokens).
  // textMuted-on-bubble is deliberately NOT gated: timestamp colour on
  // bubbles is a Messaging-UX design decision (dark textMuted on
  // bubbleMine measures 3.34:1, light 4.02:1 — a dedicated timestamp
  // token or a bubble-palette change must resolve it at that milestone;
  // adding textMuted here would fail CI on correct palette values).
  for (final MapEntry(key: surf, value: bg) in {
    'bubbleMine': c.bubbleMine,
    'bubbleTheirs': c.bubbleTheirs,
  }.entries) {
    final r = _ratio(c.text, bg);
    expect(
      r,
      greaterThanOrEqualTo(4.5),
      reason: '$name: text on $surf is ${r.toStringAsFixed(2)}:1',
    );
  }

  // Accent-pill text on its soft background.
  expect(
    _ratio(c.accentText, c.accentSoft),
    greaterThanOrEqualTo(4.5),
    reason: '$name: accentText on accentSoft',
  );

  // Non-text UI minimum (WCAG 1.4.11, ≥3:1): inactive icons.
  expect(
    _ratio(c.inactiveElement, c.bg),
    greaterThanOrEqualTo(3.0),
    reason: '$name: inactiveElement on bg (WCAG 1.4.11)',
  );
  // FR-49 W-1: TEXT sits on accent (a primary button's label, the primary
  // action tile), so onAccent is graded as small text, not as a glyph.
  expect(
    _ratio(c.onAccent, c.accent),
    greaterThanOrEqualTo(4.5),
    reason: '$name: onAccent on accent (small-text AA — text sits on accent)',
  );

  // The brand surface (FR-49 W-6, W-8): the balance card's figures, labels
  // and in-card warning notes are all text on `deep`.
  for (final MapEntry(key: tok, value: color) in {
    'onDeep': c.onDeep,
    'deepMuted': c.deepMuted,
    'warningOnDeep': c.warningOnDeep,
  }.entries) {
    final r = _ratio(color, c.deep);
    expect(
      r,
      greaterThanOrEqualTo(4.5),
      reason: '$name: $tok on deep is ${r.toStringAsFixed(2)}:1',
    );
  }

  // MATERIAL COLORSCHEME pairs (money-lens fold): wallet success/error
  // surfaces will read colorScheme.onSecondary/onError (snackbars, filled
  // buttons, chips) — gate the pairs a wallet dev actually reaches for,
  // not just the raw tokens. Small-text AA: error text must stay readable.
  final cs = buildTheme(c).colorScheme;
  expect(
    _ratio(cs.onError, cs.error),
    greaterThanOrEqualTo(4.5),
    reason: '$name: colorScheme.onError on error',
  );
  expect(
    _ratio(cs.onSecondary, cs.secondary),
    greaterThanOrEqualTo(4.5),
    reason: '$name: colorScheme.onSecondary on secondary',
  );
  expect(
    _ratio(cs.onPrimary, cs.primary),
    greaterThanOrEqualTo(4.5),
    reason: '$name: colorScheme.onPrimary on primary (button labels)',
  );
  expect(
    _ratio(cs.onPrimaryContainer, cs.primaryContainer),
    greaterThanOrEqualTo(4.5),
    reason: '$name: colorScheme.onPrimaryContainer on primaryContainer',
  );
  expect(
    _ratio(cs.onSurface, cs.surface),
    greaterThanOrEqualTo(4.5),
    reason: '$name: colorScheme.onSurface on surface',
  );
  // CONTAINER pairs: the receive screen's fresh-address note
  // reads onSecondaryContainer/secondaryContainer, the transparent warning
  // onTertiaryContainer/tertiaryContainer — small text on both.
  expect(
    _ratio(cs.onSecondaryContainer, cs.secondaryContainer),
    greaterThanOrEqualTo(4.5),
    reason: '$name: colorScheme.onSecondaryContainer on secondaryContainer',
  );
  expect(
    _ratio(cs.onTertiaryContainer, cs.tertiaryContainer),
    greaterThanOrEqualTo(4.5),
    reason: '$name: colorScheme.onTertiaryContainer on tertiaryContainer',
  );
  expect(
    _ratio(cs.onTertiary, cs.tertiary),
    greaterThanOrEqualTo(3.0),
    reason: '$name: colorScheme.onTertiary on tertiary (UI/large-text AA)',
  );
}

void main() {
  test('light_theme_contrast_meets_wcag_aa', () {
    _gateScheme('light', WalletColors.light);
  });

  // Goes beyond the source palette's own gating (which covered light
  // only): dark AND the AMOLED variant are gated here.
  test('dark_theme_contrast_meets_wcag_aa', () {
    _gateScheme('dark', WalletColors.dark);
    _gateScheme('darkAmoled', WalletColors.darkAmoled);
  });
}
