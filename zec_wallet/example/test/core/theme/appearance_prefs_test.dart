import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_example/core/theme/appearance_prefs.dart';
import 'package:zec_wallet_example/core/theme/theme_mode_provider.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('theme_mode_persists_and_unknown_value_falls_back', () async {
    // Corrupt/unknown stored values are cosmetic input → default, never
    // throw (spec §6).
    expect(themeModeFromString(null), ThemeMode.system);
    expect(themeModeFromString('garbage'), ThemeMode.system);
    expect(themeModeFromString('light'), ThemeMode.light);
    expect(themeModeFromString('dark'), ThemeMode.dark);

    // Set → persist → "restart" (the bootstrap read path) → still dark.
    SharedPreferences.setMockInitialValues({});
    final container = ProviderContainer();
    addTearDown(container.dispose);
    expect(container.read(themeModeProvider), ThemeMode.system);
    await container.read(themeModeProvider.notifier).set(ThemeMode.dark);
    expect(container.read(themeModeProvider), ThemeMode.dark);

    final prefs = await SharedPreferences.getInstance();
    final restartMode = themeModeFromString(prefs.getString(themeModePrefsKey));
    final restarted = ProviderContainer(
      overrides: [initialThemeModeProvider.overrideWithValue(restartMode)],
    );
    addTearDown(restarted.dispose);
    expect(restarted.read(themeModeProvider), ThemeMode.dark);
  });

  test('text_scale_composition_clamped_at_extremes', () async {
    // Absolute floor/ceiling hold for ANY OS × user combination (§7).
    expect(
      effectiveTextScale(osFactor: 0.5, userScale: minTextScale, isIos: false),
      minEffectiveTextScale,
    );
    expect(
      effectiveTextScale(osFactor: 3.0, userScale: maxTextScale, isIos: false),
      maxEffectiveTextScale,
    );

    // iOS HIG floor: a default-size user is lifted to 1.12×…
    expect(
      effectiveTextScale(osFactor: 1.0, userScale: 1.0, isIos: true),
      iosHigMinTextScale,
    );
    // …but a user already past the floor keeps their Dynamic Type choice
    // (clamp, not multiply).
    expect(effectiveTextScale(osFactor: 1.3, userScale: 1.0, isIos: true), 1.3);
    // Android is untouched at defaults.
    expect(
      effectiveTextScale(osFactor: 1.0, userScale: 1.0, isIos: false),
      1.0,
    );

    // Hostile/corrupt stored multipliers are clamped at the read boundary.
    SharedPreferences.setMockInitialValues({textScalePrefsKey: 99.0});
    var prefs = await SharedPreferences.getInstance();
    expect(textScaleFromPrefs(prefs), maxTextScale);
    SharedPreferences.setMockInitialValues({textScalePrefsKey: -3.0});
    prefs = await SharedPreferences.getInstance();
    expect(textScaleFromPrefs(prefs), minTextScale);

    // The hardest corrupt double: NaN must clamp to a bound, never
    // propagate (Dart's num.clamp(NaN) returns a bound — pinned here as
    // a DELIBERATE invariant, not an accidental VM property; a NaN
    // textScaler would silently break every Text in the app).
    SharedPreferences.setMockInitialValues({textScalePrefsKey: double.nan});
    prefs = await SharedPreferences.getInstance();
    expect(
      textScaleFromPrefs(prefs),
      isIn([minTextScale, maxTextScale]),
      reason: 'NaN stored scale must clamp, never propagate',
    );
    final nanComposed = effectiveTextScale(
      osFactor: double.nan,
      userScale: 1.0,
      isIos: false,
    );
    expect(nanComposed.isNaN, isFalse);
    expect(nanComposed, isIn([minEffectiveTextScale, maxEffectiveTextScale]));
  });
}
