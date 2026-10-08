import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

/// Source-level guard on the Android permission posture (IZ-4 security fold).
///
/// `flutter_zxing` → `camera` → `camera_android_camerax` transitively UNIONS
/// `RECORD_AUDIO` + `WRITE_EXTERNAL_STORAGE` into the merged manifest (the
/// camera plugin supports video+audio; the scanner uses neither). A privacy-
/// first, censorship-resistant wallet must never ship an unused microphone
/// permission, so the app manifest strips both with `tools:node="remove"`.
/// This test fails loudly if that suppression (or the CAMERA grant the scanner
/// needs) is ever dropped — e.g. by a future edit or dependency bump.
///
/// It checks the SOURCE manifest, not the merged output (`flutter test` builds
/// no native code); the merged result is verified at device-build time.
void main() {
  test(
    'Android manifest keeps CAMERA + strips RECORD_AUDIO / WRITE_EXTERNAL',
    () {
      final manifest = File(
        'android/app/src/main/AndroidManifest.xml',
      ).readAsStringSync();

      // The tools namespace is required for node="remove" to take effect.
      expect(
        manifest.contains('xmlns:tools='),
        isTrue,
        reason: 'tools namespace needed for node="remove"',
      );

      // The scanner needs CAMERA (required=false so camera-less devices install).
      expect(manifest.contains('android.permission.CAMERA'), isTrue);

      // Both transitive permissions must be explicitly removed.
      expect(
        RegExp(
          r'android\.permission\.RECORD_AUDIO"\s+tools:node="remove"',
        ).hasMatch(manifest),
        isTrue,
        reason: 'RECORD_AUDIO must be stripped — no microphone in the wallet',
      );
      expect(
        RegExp(
          r'android\.permission\.WRITE_EXTERNAL_STORAGE"\s+tools:node="remove"',
        ).hasMatch(manifest),
        isTrue,
        reason: 'WRITE_EXTERNAL_STORAGE (legacy, maxSdk 28) must be stripped',
      );
    },
  );
}
