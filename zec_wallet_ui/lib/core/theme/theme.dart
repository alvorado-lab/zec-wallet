import 'package:flutter/foundation.dart' show defaultTargetPlatform;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'colors.dart';
import 'icons.dart';
import 'shapes.dart';
import 'sheet_layout.dart';
import 'typography.dart';

/// Theme-switch animation length (flutter-patterns § Theming: animated
/// transition via `ThemeExtension.lerp`, ~200 ms — long enough to read as
/// deliberate, short enough to never gate interaction).
const themeAnimationDuration = Duration(milliseconds: 200);

/// System status-bar / nav-bar overlay style for a given scheme.
/// Light theme → dark icons (visible on cream); dark theme → light icons.
/// Applied via the AppBar theme AND globally in the MaterialApp `builder`
/// (AppBar-less screens get no `appBarTheme.systemOverlayStyle`).
///
/// ICON BRIGHTNESS ONLY — `statusBarColor`/`systemNavigationBarColor` are
/// deliberately NOT set: at targetSdk 36+ Flutter forces edge-to-edge and
/// both fields are silent no-ops (mobile review fold); setting them would
/// imply a contract the app cannot keep. On gesture-nav devices the nav
/// brightness only affects the pill; 3-button nav gets full icon tinting.
SystemUiOverlayStyle overlayStyleFor(WalletColors colors) {
  final isDark = colors.bg.computeLuminance() < 0.5;
  return SystemUiOverlayStyle(
    statusBarIconBrightness: isDark ? Brightness.light : Brightness.dark,
    statusBarBrightness: isDark ? Brightness.dark : Brightness.light, // iOS
    systemNavigationBarIconBrightness: isDark
        ? Brightness.light
        : Brightness.dark,
  );
}

/// The wallet UI's type scale, with NO font family (FR-49 W-3, W-9): sizes,
/// weights and tracking from the refreshed design (Relim design-refresh spec
/// §3.6), colours from [colors]. Every slot the SDK's widgets read is
/// defined, so a host that supplies only some styles still gets the SDK's
/// sizes for the rest, never Material's defaults.
TextTheme walletTextScale(WalletColors colors) {
  TextStyle s(
    double size,
    FontWeight weight,
    Color color, {
    double tracking = 0,
    double? height,
  }) => TextStyle(
    fontSize: size,
    fontWeight: weight,
    color: color,
    // Tracking is given in em by the design; Flutter wants logical pixels.
    letterSpacing: tracking * size,
    height: height,
  );

  return TextTheme(
    // The balance figure (the wallet card's total).
    displaySmall: s(38, FontWeight.w700, colors.text, tracking: -0.03),
    // A tab-root large title.
    headlineLarge: s(30, FontWeight.w700, colors.text, tracking: -0.025),
    // A pushed page's title.
    headlineMedium: s(22, FontWeight.w600, colors.text, tracking: -0.01),
    // Sheet titles and step titles.
    headlineSmall: s(24, FontWeight.w700, colors.text, tracking: -0.02),
    // Section titles ("Activity"), empty-state titles.
    titleLarge: s(20, FontWeight.w700, colors.text, tracking: -0.02),
    // Row titles.
    titleMedium: s(16, FontWeight.w600, colors.text),
    // Sub-labels.
    titleSmall: s(14, FontWeight.w600, colors.text),
    // Body text.
    bodyLarge: s(16, FontWeight.w400, colors.text, height: 1.38),
    // Secondary lines and dense rows.
    bodyMedium: s(14, FontWeight.w400, colors.textMuted),
    // Timestamps and meta.
    bodySmall: s(12.5, FontWeight.w400, colors.textMuted),
    // Buttons.
    labelLarge: s(16, FontWeight.w600, colors.text),
    // Chips and label lines.
    labelMedium: s(14, FontWeight.w500, colors.text),
    // Section labels (uppercased by the widget).
    labelSmall: s(12, FontWeight.w600, colors.textMuted, tracking: 0.08),
  );
}

/// Build a complete ThemeData from a WalletColors scheme.
///
/// The SDK bundles no font (FR-49 W-9, ADR-0564): with no [textTheme] the
/// text is set in the platform's default face. A host that wants its own
/// faces passes a [textTheme]; its non-null values are merged over
/// [walletTextScale], so a host may set only families and keep the SDK's
/// sizes. The mono face for identifiers is [WalletTypography], the glyphs are
/// [WalletIcons], and the corner radii are [WalletShapes]; pass them in
/// [extensions] to override the defaults.
///
/// The component themes (buttons, fields, sheets, dialogs, chips, progress,
/// toggles; Relim design-refresh spec §3.4, stage S11 C5) mirror Relim's own
/// `ThemeData`, so a host on this theme and Relim on its own look alike. The
/// SDK's widgets carry no style literal of their own: in a host with its own
/// theme, the host's component themes decide.
ThemeData buildTheme(
  WalletColors colors, {
  TextTheme? textTheme,
  Iterable<ThemeExtension> extensions = const [],
}) {
  final scale = walletTextScale(colors);
  final resolvedText = textTheme == null ? scale : scale.merge(textTheme);

  final isDark = colors.bg.computeLuminance() < 0.5;

  // The radii the component themes below are cut to: the host's, or the
  // defaults for the platform this ThemeData resolves to (it reads the same
  // `defaultTargetPlatform` when no platform is given).
  final shapes =
      extensions.whereType<WalletShapes>().firstOrNull ??
      WalletShapes.forPlatform(defaultTargetPlatform);

  // One extension per type: a host-supplied WalletTypography, WalletIcons or
  // WalletShapes replaces the SDK default; WalletColors is always [colors].
  final resolvedExtensions = <ThemeExtension>[
    if (!extensions.any((e) => e is WalletTypography))
      WalletTypography.fallback,
    if (!extensions.any((e) => e is WalletIcons)) WalletIcons.material,
    if (!extensions.any((e) => e is WalletShapes)) shapes,
    ...extensions.where((e) => e is! WalletColors),
    colors,
  ];

  // §3.4: every button is a pill at least 48 high, labelled in labelLarge.
  const buttonMin = Size(48, 48);
  const pill = StadiumBorder();
  final buttonLabel = resolvedText.labelLarge;
  OutlineInputBorder field(Color c, double w) => OutlineInputBorder(
    borderRadius: BorderRadius.circular(shapes.field),
    borderSide: BorderSide(color: c, width: w),
  );
  // `outline` is `toggleOff`: the SDK has no `borderStrong`, and S11 adds no
  // colour field (C5).
  final outline = colors.toggleOff;

  return ThemeData(
    brightness: isDark ? Brightness.dark : Brightness.light,
    scaffoldBackgroundColor: colors.bg,
    // The refreshed design's roles (Relim design-refresh spec §3.3).
    colorScheme: ColorScheme(
      brightness: isDark ? Brightness.dark : Brightness.light,
      primary: colors.accent,
      onPrimary: colors.onAccent,
      primaryContainer: colors.accentSoft,
      onPrimaryContainer: colors.accentText,
      // Positive notes and tonal buttons use the brand's soft tint (mint), one
      // colour for one meaning (the refresh folds success into the brand).
      secondary: colors.accent,
      onSecondary: colors.onAccent,
      secondaryContainer: colors.accentSoft,
      onSecondaryContainer: colors.accentText,
      tertiary: colors.orange,
      // MONEY-LENS FOLD: a dark palette's warning/danger are BRIGHT fills, so
      // their on-colours are dark in dark themes and white in light. Gated by
      // the contrast tests — colorScheme pairs included, not just raw tokens.
      onTertiary: isDark ? colors.bg : colors.onAccent,
      // CONTAINER ROLES: a low-alpha tint of the warning hue
      // over the card surface, so a caution note never reads like a positive
      // one. The refreshed spec copies this derivation (§3.3).
      tertiaryContainer: Color.alphaBlend(
        colors.orange.withValues(alpha: isDark ? 0.20 : 0.13),
        colors.bgCard,
      ),
      onTertiaryContainer: colors.text,
      error: colors.red,
      onError: isDark ? colors.bg : colors.onAccent,
      surface: colors.bgCard,
      onSurface: colors.text,
      onSurfaceVariant: colors.textMuted,
      surfaceContainerLowest: colors.bg,
      surfaceContainerLow: colors.bgCard,
      surfaceContainerHigh: colors.bgHover,
      surfaceContainerHighest: colors.bgHover,
      outline: colors.toggleOff,
      outlineVariant: colors.border,
      inverseSurface: colors.text,
      onInverseSurface: colors.bg,
    ),
    textTheme: resolvedText,
    appBarTheme: AppBarTheme(
      backgroundColor: colors.bg,
      foregroundColor: colors.text,
      elevation: 0,
      scrolledUnderElevation: 0,
      systemOverlayStyle: overlayStyleFor(colors),
    ),
    bottomNavigationBarTheme: BottomNavigationBarThemeData(
      backgroundColor: colors.bg,
      selectedItemColor: colors.accent,
      unselectedItemColor: colors.inactiveElement,
      selectedLabelStyle: resolvedText.labelSmall?.copyWith(fontSize: 11),
      unselectedLabelStyle: resolvedText.labelSmall?.copyWith(fontSize: 11),
      type: BottomNavigationBarType.fixed,
      elevation: 0,
    ),
    tabBarTheme: TabBarThemeData(
      labelColor: colors.accent,
      unselectedLabelColor: colors.textMuted,
      indicatorColor: colors.accent,
    ),
    dividerTheme: DividerThemeData(color: colors.border, thickness: 1),
    cardTheme: CardThemeData(
      color: colors.bgCard,
      elevation: 0,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(shapes.group),
      ),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        minimumSize: buttonMin,
        shape: pill,
        textStyle: buttonLabel,
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        foregroundColor: colors.text,
        minimumSize: buttonMin,
        shape: pill,
        side: BorderSide(color: outline, width: 1.5),
        textStyle: buttonLabel,
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(
        minimumSize: buttonMin,
        shape: pill,
        textStyle: buttonLabel,
      ),
    ),
    iconButtonTheme: IconButtonThemeData(
      style: IconButton.styleFrom(minimumSize: const Size(44, 44)),
    ),
    inputDecorationTheme: InputDecorationThemeData(
      filled: true,
      fillColor: colors.bgCard,
      border: field(outline, 1),
      enabledBorder: field(outline, 1),
      focusedBorder: field(colors.accent, 2),
      errorBorder: field(colors.red, 1),
      focusedErrorBorder: field(colors.red, 2),
      disabledBorder: field(colors.border, 1),
    ),
    bottomSheetTheme: BottomSheetThemeData(
      backgroundColor: colors.bgCard,
      modalBackgroundColor: colors.bgCard,
      dragHandleColor: outline,
      constraints: const BoxConstraints(maxWidth: walletSheetMaxWidth),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(shapes.sheet)),
      ),
    ),
    // S13 Build B (DESIGN §6.9): the title 600 at 20 (the `titleLarge` role,
    // re-weighted), the body at 16 (the `bodyLarge` role, in `text`). From the
    // resolved roles, so a host text theme still decides the face and size.
    // The date pickers are Material dialogs of their own (not AlertDialog)
    // and do not read these two styles.
    dialogTheme: DialogThemeData(
      backgroundColor: colors.bgCard,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(shapes.dialog),
      ),
      titleTextStyle: resolvedText.titleLarge?.copyWith(
        fontWeight: FontWeight.w600,
        color: colors.text,
      ),
      contentTextStyle: resolvedText.bodyLarge?.copyWith(color: colors.text),
    ),
    snackBarTheme: SnackBarThemeData(
      behavior: SnackBarBehavior.floating,
      backgroundColor: colors.text,
      contentTextStyle: resolvedText.bodyMedium?.copyWith(color: colors.bg),
      shape: pill,
    ),
    switchTheme: SwitchThemeData(
      trackColor: WidgetStateProperty.resolveWith(
        (states) => states.contains(WidgetState.selected)
            ? colors.accent
            : colors.bgHover,
      ),
      trackOutlineColor: WidgetStateProperty.resolveWith(
        (states) => states.contains(WidgetState.selected)
            ? Colors.transparent
            : colors.textMuted,
      ),
      trackOutlineWidth: const WidgetStatePropertyAll(2),
      thumbColor: WidgetStateProperty.resolveWith(
        (states) => states.contains(WidgetState.selected)
            ? colors.onAccent
            : colors.textMuted,
      ),
    ),
    radioTheme: RadioThemeData(
      fillColor: WidgetStateProperty.resolveWith(
        (states) => states.contains(WidgetState.selected)
            ? colors.accent
            : colors.textMuted,
      ),
    ),
    // Relim's 6 dp checkbox corner has no WalletShapes role, and a literal
    // radius is banned outside shapes.dart, so the box keeps Material's shape.
    checkboxTheme: CheckboxThemeData(
      fillColor: WidgetStateProperty.resolveWith(
        (states) => states.contains(WidgetState.selected)
            ? colors.accent
            : Colors.transparent,
      ),
      checkColor: WidgetStatePropertyAll(colors.onAccent),
    ),
    segmentedButtonTheme: SegmentedButtonThemeData(
      style: SegmentedButton.styleFrom(
        backgroundColor: colors.bgHover,
        foregroundColor: colors.text,
        selectedBackgroundColor: colors.accentSoft,
        selectedForegroundColor: colors.accentText,
        side: BorderSide.none,
        shape: pill,
        textStyle: resolvedText.labelMedium,
      ),
    ),
    chipTheme: ChipThemeData(
      backgroundColor: Colors.transparent,
      selectedColor: colors.accentSoft,
      labelStyle: resolvedText.labelMedium,
      side: BorderSide(color: outline),
      shape: pill,
      checkmarkColor: colors.accentText,
    ),
    // `cyan` on `outline`, 6 high (§9.4 item 3). The sync STATE is carried by
    // the tinted glyph beside the bar, not the bar's colour.
    progressIndicatorTheme: ProgressIndicatorThemeData(
      color: colors.cyan,
      linearTrackColor: outline,
      linearMinHeight: 6,
      borderRadius: BorderRadius.circular(shapes.progress),
    ),
    listTileTheme: const ListTileThemeData(minTileHeight: 52),
    extensions: resolvedExtensions,
  );
}

/// Dark theme (app default) — the refreshed design's dim dark.
final darkTheme = buildTheme(WalletColors.dark);

/// Dark theme, AMOLED true-black variant (Settings → Appearance opt-in).
final amoledDarkTheme = buildTheme(WalletColors.darkAmoled);

/// Light theme.
final lightTheme = buildTheme(WalletColors.light);
