import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_example/core/theme/example_type.dart';
import 'package:zec_wallet_ui/core/theme/typography.dart';

/// The SDK bundles no font (FR-49 W-9, ADR-0564); this app, as the reference
/// host, bundles the refreshed design's three faces and hands them to the
/// SDK's theme. A family string that is not in the font manifest is a SILENT
/// fallback to the platform face — no error, no other failing test — so this
/// pins every family the example's themes name to a declared, shipped font.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test(
    'every family the example themes name resolves in the font manifest',
    () async {
      final manifest =
          (json.decode(await rootBundle.loadString('FontManifest.json'))
                  as List<dynamic>)
              .cast<Map<String, dynamic>>();

      Set<int> weightsOf(String family) {
        final entry = manifest.singleWhere(
          (e) => e['family'] == family,
          orElse: () => fail(
            '"$family" is not in FontManifest.json — text in it is silently '
            'falling back to the platform face',
          ),
        );
        final fonts = (entry['fonts'] as List<dynamic>)
            .cast<Map<String, dynamic>>();
        for (final font in fonts) {
          expect(font['asset'], startsWith('assets/fonts/'));
        }
        return fonts.map((f) => f['weight'] as int).toSet();
      }

      // The weights the SDK's scale asks of each face must be shipped, or
      // they degrade to a synthetic weight.
      expect(weightsOf(exampleTitleFamily), containsAll({600, 700}));
      expect(weightsOf(exampleUiFamily), containsAll({400, 500, 600}));
      expect(weightsOf(exampleMonoFamily), containsAll({400}));

      // Every theme the app mounts carries those families, not the platform
      // default and not a forked constant.
      for (final theme in [
        exampleLightTheme,
        exampleDarkTheme,
        exampleAmoledDarkTheme,
      ]) {
        expect(theme.textTheme.displaySmall!.fontFamily, exampleTitleFamily);
        expect(theme.textTheme.headlineSmall!.fontFamily, exampleTitleFamily);
        expect(theme.textTheme.bodyLarge!.fontFamily, exampleUiFamily);
        expect(theme.textTheme.labelMedium!.fontFamily, exampleUiFamily);
        expect(
          theme.extension<WalletTypography>()!.mono.fontFamily,
          exampleMonoFamily,
        );
      }
    },
  );
}
