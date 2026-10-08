import 'package:flutter/material.dart';

/// All app color tokens. Widgets use `WalletColors.of(context).bg` etc.
/// NEVER hardcode hex values — always reference tokens (flutter-patterns
/// § Theming; hardcoded hex breaks the second theme silently).
///
/// The palette ships WCAG-audited light/dark values; re-skinning is a
/// constants change here, which is the entire point of tokens. The
/// accessibility CONTRACT is the `*_theme_contrast_meets_wcag_aa` tests, not
/// the literals below.
///
/// The presets are the refreshed design's greens (ADR-0564, maintainer ruling A:
/// the SDK's own look adopts the refresh). Values are Relim's design-refresh
/// spec §3.1, mapped onto these names as its §9.3 maps them — one palette, so
/// a host on the SDK presets and Relim on its own theme look the same.
@immutable
class WalletColors extends ThemeExtension<WalletColors> {
  const WalletColors({
    required this.bg,
    required this.bgCard,
    required this.bgHover,
    required this.border,
    required this.accent,
    required this.accentSoft,
    required this.accentText,
    required this.text,
    required this.textMuted,
    required this.textDim,
    required this.green,
    required this.orange,
    required this.red,
    required this.purple,
    required this.cyan,
    required this.bubbleMine,
    required this.bubbleTheirs,
    required this.toggleOff,
    required this.inactiveElement,
    required this.onAccent,
    required this.deep,
    required this.onDeep,
    required this.deepMuted,
    required this.warningOnDeep,
    this.qrInk = const Color(0xFF000000),
    Color? coinFace,
    Color? coinEdge,
    Color? coinRim,
    Color? stage,
    Color? onStage,
  }) : _coinFace = coinFace,
       _coinEdge = coinEdge,
       _coinRim = coinRim,
       _stage = stage,
       _onStage = onStage;

  final Color bg;
  final Color bgCard;
  final Color bgHover;
  final Color border;
  final Color accent;
  final Color accentSoft;
  final Color accentText;
  final Color text;
  final Color textMuted;
  final Color textDim;
  final Color green;
  final Color orange;
  final Color red;
  final Color purple;
  final Color cyan;
  final Color bubbleMine;
  final Color bubbleTheirs;
  final Color toggleOff;
  final Color inactiveElement;

  /// Content (text and glyphs) on an `accent` fill — NOT "always white". The
  /// refreshed dark palettes put a near-black on their bright green, because
  /// white on it measures 2.2:1. Text sits on `accent` (a primary button's
  /// label, the primary action tile), so the contrast test grades this pair
  /// at small-text AA, ≥ 4.5:1 (FR-49 W-1). Valid ONLY on `accent`: never on
  /// `green`, `red` or an avatar fill, whose pairings are their own.
  final Color onAccent;

  /// The brand surface: the wallet balance card's fill (FR-49 W-6).
  final Color deep;

  /// Text and figures on [deep] — the balance total, row values.
  final Color onDeep;

  /// Secondary text on [deep] — labels, the as-of header.
  final Color deepMuted;

  /// A warning note's text or glyph on [deep]. The page's `orange` is tuned
  /// for the light page and fails on the card (1.9:1 light), so the notes that
  /// sit inside the card use this (FR-49 W-8).
  final Color warningOnDeep;

  /// The QR code's dark modules (FR-49 W-10). The tile stays white and keeps
  /// its quiet zone in every theme — a scanner binarises on luminance, so the
  /// code is never inverted. Default black; a host may give it its brand ink
  /// if the ink keeps a high contrast on white.
  final Color qrInk;

  // The balance card's coin (FR-49 W-6, stage S12). OPTIONAL: a host that sets
  // none gets colours derived from [deep] and [accent], so adding the coin
  // broke no host. The presets set the design's own greens.
  final Color? _coinFace;
  final Color? _coinEdge;
  final Color? _coinRim;

  /// The coin's two faces.
  Color get coinFace => _coinFace ?? Color.lerp(deep, accent, 0.85)!;

  /// The coin's edge: the stacked layers seen as it turns.
  Color get coinEdge => _coinEdge ?? Color.lerp(deep, accent, 0.5)!;

  /// The ring inset on each face.
  Color get coinRim =>
      _coinRim ?? Color.lerp(coinFace, const Color(0xFFFFFFFF), 0.15)!;

  // The camera stage (S13 §1.5). OPTIONAL, like the coin: a viewfinder stays
  // dark in every theme (the preview is the content), so the defaults are
  // black and white, not the page's bg and text.
  final Color? _stage;
  final Color? _onStage;

  /// The surface around and over a live camera preview: the scanner's page,
  /// its bars, and (translucent) the pills and controls drawn over it.
  Color get stage => _stage ?? const Color(0xFF000000);

  /// Text and glyphs on [stage] — the instruction, the camera-unavailable
  /// note, the manual-entry and close controls.
  Color get onStage => _onStage ?? const Color(0xFFFFFFFF);

  /// Retrieve from context. Throws if not found — with a message that names
  /// the missing wiring, because the raw null-check reads as an inscrutable
  /// crash to a HOST that mounted the wallet routes over its own ThemeData
  /// and skipped the theme step (review N1).
  static WalletColors of(BuildContext context) {
    final colors = Theme.of(context).extension<WalletColors>();
    assert(
      colors != null,
      'No WalletColors ThemeExtension in this ThemeData. A host embedding the '
      'wallet UI must either use the package buildTheme(...) or register a '
      'WalletColors extension in its own theme (zec_wallet_ui README, theme '
      'step).',
    );
    return colors!;
  }

  // --- Dark theme (default) — the refreshed design's DIM dark ---
  // Relim's default dark palette (design-refresh §3.5: AMOLED is opt-in).
  // Shares every non-surface value with [darkAmoled].
  static const dark = WalletColors(
    bg: Color(0xFF0E1411),
    bgCard: Color(0xFF18211C),
    bgHover: Color(0xFF1F2A24),
    border: Color(0xFF26332C),
    accent: Color(0xFF3CC47F),
    accentSoft: Color(0xFF163A2B), // mint
    accentText: Color(0xFF9FE0BD), // mintink — 8.29:1 on accentSoft
    text: Color(0xFFECF2EE),
    textMuted: Color(0xFF8E9C94),
    // Folded into textMuted by the refresh; the field stays for hosts.
    textDim: Color(0xFF8E9C94),
    green: Color(0xFF3CC47F), // success is the brand green
    orange: Color(0xFFE8B04A), // warning
    red: Color(0xFFF0907E), // danger
    // No purple in the refreshed palette; the SDK reads it nowhere, so it
    // carries the nearest role (as Relim feeds it, §9.3).
    purple: Color(0xFF9FE0BD),
    cyan: Color(0xFF34B272),
    // The bubble tokens are unread by the SDK. They hold text-safe tints so
    // the retained bubble gate stays meaningful.
    bubbleMine: Color(0xFF163A2B),
    bubbleTheirs: Color(0xFF18211C),
    toggleOff: Color(0xFF34433B),
    inactiveElement: Color(0xFF8E9C94),
    onAccent: Color(0xFF03170C), // 8.30:1 on accent
    deep: Color(0xFF0B2A1B),
    onDeep: Color(0xFFEAF4EE), // 13.72:1 on deep
    deepMuted: Color(0xFF8FBFA4), // 7.46:1 on deep
    warningOnDeep: Color(0xFFF2C46B), // 9.46:1 on deep
    // The design's coin (Relim spec §9.5 W-6, the template's greens) — the
    // same coin in every mode.
    coinFace: Color(0xFF288E59),
    coinEdge: Color(0xFF0E5C36),
    coinRim: Color(0xFF3AA56C),
  );

  // --- Dark theme, AMOLED true-black variant ---
  // Same as [dark] (incl. all semantics) but with pure-black surfaces so
  // OLED pixels switch fully off. Opt-in via Settings → Appearance (only
  // effective while the dark theme is active). `static final` because
  // copyWith isn't const.
  static final darkAmoled = dark.copyWith(
    bg: const Color(0xFF000000),
    bgCard: const Color(0xFF0D1310),
    bgHover: const Color(0xFF141D18),
    border: const Color(0xFF1C2722),
    toggleOff: const Color(0xFF2B3832),
    bubbleTheirs: const Color(0xFF0D1310),
  );

  // --- Light theme — the refreshed design's warm off-white + brand green ---
  // Every text/element token below is WCAG-verified by
  // `light_theme_contrast_meets_wcag_aa` — that test is the gate, not these
  // literals.
  static const light = WalletColors(
    bg: Color(0xFFF6F5F0),
    bgCard: Color(0xFFFFFFFF),
    bgHover: Color(0xFFEDF2EE),
    border: Color(0xFFE3E8E3),
    accent: Color(0xFF177245), // 5.45:1 on bg
    accentSoft: Color(0xFFD0E2D8), // mint
    accentText: Color(0xFF0F4F30), // mintink — 7.13:1 on accentSoft
    text: Color(0xFF15201A), // 15.35:1 on bg
    textMuted: Color(0xFF5A675F), // 5.43:1 on bg
    textDim: Color(0xFF5A675F),
    green: Color(0xFF177245),
    orange: Color(0xFF8A5A12), // warning — 5.42:1 on bg
    red: Color(0xFFB84A39), // danger — 4.72:1 on bg (DV-1)
    purple: Color(0xFF0F4F30),
    // The SDK draws cyan as TEXT (the sync sheet's transport line, the swap
    // "ends shielded" title); the refresh's light cyan is non-text grade, so
    // light gets the text-grade green of the same hue Relim feeds.
    cyan: Color(0xFF1D7A4B),
    bubbleMine: Color(0xFFD0E2D8),
    bubbleTheirs: Color(0xFFFFFFFF),
    toggleOff: Color(0xFFCDD6CF),
    inactiveElement: Color(0xFF5A675F),
    onAccent: Color(0xFFFFFFFF), // 5.95:1 on accent
    deep: Color(0xFF0F3D27),
    onDeep: Color(0xFFEAF4EE), // 10.87:1 on deep
    deepMuted: Color(0xFFA9CBB8), // 6.94:1 on deep
    warningOnDeep: Color(0xFFF2C46B), // 7.50:1 on deep
    coinFace: Color(0xFF288E59),
    coinEdge: Color(0xFF0E5C36),
    coinRim: Color(0xFF3AA56C),
  );

  @override
  WalletColors copyWith({
    Color? bg,
    Color? bgCard,
    Color? bgHover,
    Color? border,
    Color? accent,
    Color? accentSoft,
    Color? accentText,
    Color? text,
    Color? textMuted,
    Color? textDim,
    Color? green,
    Color? orange,
    Color? red,
    Color? purple,
    Color? cyan,
    Color? bubbleMine,
    Color? bubbleTheirs,
    Color? toggleOff,
    Color? inactiveElement,
    Color? onAccent,
    Color? deep,
    Color? onDeep,
    Color? deepMuted,
    Color? warningOnDeep,
    Color? qrInk,
    Color? coinFace,
    Color? coinEdge,
    Color? coinRim,
    Color? stage,
    Color? onStage,
  }) {
    return WalletColors(
      bg: bg ?? this.bg,
      bgCard: bgCard ?? this.bgCard,
      bgHover: bgHover ?? this.bgHover,
      border: border ?? this.border,
      accent: accent ?? this.accent,
      accentSoft: accentSoft ?? this.accentSoft,
      accentText: accentText ?? this.accentText,
      text: text ?? this.text,
      textMuted: textMuted ?? this.textMuted,
      textDim: textDim ?? this.textDim,
      green: green ?? this.green,
      orange: orange ?? this.orange,
      red: red ?? this.red,
      purple: purple ?? this.purple,
      cyan: cyan ?? this.cyan,
      bubbleMine: bubbleMine ?? this.bubbleMine,
      bubbleTheirs: bubbleTheirs ?? this.bubbleTheirs,
      toggleOff: toggleOff ?? this.toggleOff,
      inactiveElement: inactiveElement ?? this.inactiveElement,
      onAccent: onAccent ?? this.onAccent,
      deep: deep ?? this.deep,
      onDeep: onDeep ?? this.onDeep,
      deepMuted: deepMuted ?? this.deepMuted,
      warningOnDeep: warningOnDeep ?? this.warningOnDeep,
      qrInk: qrInk ?? this.qrInk,
      // An unset coin colour stays unset, so it keeps deriving from the
      // (possibly changed) deep/accent.
      coinFace: coinFace ?? _coinFace,
      coinEdge: coinEdge ?? _coinEdge,
      coinRim: coinRim ?? _coinRim,
      stage: stage ?? _stage,
      onStage: onStage ?? _onStage,
    );
  }

  @override
  WalletColors lerp(WalletColors? other, double t) {
    if (other == null) return this;
    return WalletColors(
      bg: Color.lerp(bg, other.bg, t)!,
      bgCard: Color.lerp(bgCard, other.bgCard, t)!,
      bgHover: Color.lerp(bgHover, other.bgHover, t)!,
      border: Color.lerp(border, other.border, t)!,
      accent: Color.lerp(accent, other.accent, t)!,
      accentSoft: Color.lerp(accentSoft, other.accentSoft, t)!,
      accentText: Color.lerp(accentText, other.accentText, t)!,
      text: Color.lerp(text, other.text, t)!,
      textMuted: Color.lerp(textMuted, other.textMuted, t)!,
      textDim: Color.lerp(textDim, other.textDim, t)!,
      green: Color.lerp(green, other.green, t)!,
      orange: Color.lerp(orange, other.orange, t)!,
      red: Color.lerp(red, other.red, t)!,
      purple: Color.lerp(purple, other.purple, t)!,
      cyan: Color.lerp(cyan, other.cyan, t)!,
      bubbleMine: Color.lerp(bubbleMine, other.bubbleMine, t)!,
      bubbleTheirs: Color.lerp(bubbleTheirs, other.bubbleTheirs, t)!,
      toggleOff: Color.lerp(toggleOff, other.toggleOff, t)!,
      inactiveElement: Color.lerp(inactiveElement, other.inactiveElement, t)!,
      onAccent: Color.lerp(onAccent, other.onAccent, t)!,
      deep: Color.lerp(deep, other.deep, t)!,
      onDeep: Color.lerp(onDeep, other.onDeep, t)!,
      deepMuted: Color.lerp(deepMuted, other.deepMuted, t)!,
      warningOnDeep: Color.lerp(warningOnDeep, other.warningOnDeep, t)!,
      qrInk: Color.lerp(qrInk, other.qrInk, t)!,
      // A coin colour neither side set stays unset (it keeps deriving from
      // the interpolated deep/accent), matching copyWith; one either side set
      // interpolates between the two resolved colours.
      coinFace: _coinFace == null && other._coinFace == null
          ? null
          : Color.lerp(coinFace, other.coinFace, t),
      coinEdge: _coinEdge == null && other._coinEdge == null
          ? null
          : Color.lerp(coinEdge, other.coinEdge, t),
      coinRim: _coinRim == null && other._coinRim == null
          ? null
          : Color.lerp(coinRim, other.coinRim, t),
      stage: _stage == null && other._stage == null
          ? null
          : Color.lerp(stage, other.stage, t),
      onStage: _onStage == null && other._onStage == null
          ? null
          : Color.lerp(onStage, other.onStage, t),
    );
  }
}
