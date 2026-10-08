import 'package:flutter/material.dart';
import 'package:zec_wallet_ui/core/theme/colors.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/core/theme/typography.dart';

/// The example's faces — what a host does now that the SDK bundles no font
/// (FR-49 W-9, ADR-0564): bundle its own (pubspec `fonts:`) and hand them to
/// the SDK's theme. Only the FAMILIES are set here; `buildTheme` merges them
/// over the SDK's own scale, so sizes and weights stay the SDK's.
///
/// The refreshed design's three roles: Onest for titles, Geist for everything
/// a user reads or taps, JetBrains Mono for identifiers.
const exampleTitleFamily = 'Onest';
const exampleUiFamily = 'Geist';
const exampleMonoFamily = 'JetBrains Mono';

const _title = TextStyle(fontFamily: exampleTitleFamily);
const _ui = TextStyle(fontFamily: exampleUiFamily);

/// Families per `TextTheme` slot.
const exampleTextTheme = TextTheme(
  displaySmall: _title,
  headlineLarge: _title,
  headlineMedium: _title,
  headlineSmall: _title,
  titleLarge: _title,
  titleMedium: _ui,
  titleSmall: _ui,
  bodyLarge: _ui,
  bodyMedium: _ui,
  bodySmall: _ui,
  labelLarge: _ui,
  labelMedium: _ui,
  labelSmall: _ui,
);

/// The identifier face.
const exampleTypography = WalletTypography(
  mono: TextStyle(fontFamily: exampleMonoFamily),
);

/// The example's three themes: the SDK presets in the example's faces.
ThemeData exampleTheme(WalletColors colors) => buildTheme(
  colors,
  textTheme: exampleTextTheme,
  extensions: const [exampleTypography],
);

final exampleLightTheme = exampleTheme(WalletColors.light);
final exampleDarkTheme = exampleTheme(WalletColors.dark);
final exampleAmoledDarkTheme = exampleTheme(WalletColors.darkAmoled);
