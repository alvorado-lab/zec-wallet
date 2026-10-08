import 'dart:io' show Platform;

import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter/services.dart';

import 'onboarding_providers.dart';

/// The Android `FLAG_SECURE` [ScreenSecurity] adapter (spec §3.2g iii-B-2-b): a
/// platform channel to the host's `MainActivity`, which adds/clears
/// `WindowManager.LayoutParams.FLAG_SECURE` on the window. That both blocks
/// screenshots and excludes the screen from the OS app-switcher/recents snapshot
/// while the recovery phrase is visible.
///
/// HOST OBLIGATION (package README § native handlers): the native side of
/// [channelName] does NOT ship with this Dart-only package. A host either adds
/// the optional `zec_wallet_ui_platform` companion plugin (which registers this
/// handler natively, plus the iOS cover, with zero host code) or registers its
/// own handler in `MainActivity` (the reference consumer's MainActivity.kt is
/// the copyable implementation). HONESTY BY
/// CONSTRUCTION (extraction review B1): a host that skips that wiring is
/// NOT silently "protected" — [enable] resolves `false` when no handler
/// responds, and the seed screen's copy claims a block only on `true`.
///
/// HONEST PLATFORM TRUTH (flutter-patterns § Platform Capability Gating):
/// `FLAG_SECURE` is Android-ONLY — [isScreenshotBlockSupported] reports the
/// platform CAPABILITY, the signal adapter selection and the test fakes key on;
/// it no longer drives any UI copy (the ENGAGEMENT truth the copy claims is
/// [enable]'s return value). iOS / desktop rely on the cross-platform
/// auto-hide-on-background in the backup view; the instant iOS
/// cover-on-resign-active app-switcher snapshot cover ships in the optional
/// `zec_wallet_ui_platform` companion plugin (#342).
///
/// Best-effort defence-in-depth (NOT the money-safety gate): a channel failure
/// never breaks the backup screen — caught by TYPE, never by matching an error
/// string across the boundary (flutter-patterns); no payload is read or logged
/// (§5.4).
class MethodChannelScreenSecurity implements ScreenSecurity {
  MethodChannelScreenSecurity({MethodChannel? channel})
    : _channel = channel ?? const MethodChannel(channelName);

  /// The platform-channel name shared with the embedding app's `MainActivity`
  /// (and the host-VM tests' mock handlers). Namespaced to THIS package — the
  /// contract belongs to the reusable wallet UI, not to any one app.
  static const String channelName = 'zec_wallet_ui/screen_security';

  final MethodChannel _channel;

  @override
  Future<bool> enable() => _invoke('enableSecure');

  @override
  Future<void> disable() => _invoke('disableSecure');

  @override
  bool get isScreenshotBlockSupported => !kIsWeb && Platform.isAndroid;

  /// True iff the native side acknowledged the call.
  Future<bool> _invoke(String method) async {
    try {
      await _channel.invokeMethod<void>(method);
      return true;
    } on MissingPluginException {
      // No handler registered — a non-Android platform, or a HOST that never
      // wired the channel. The protection is NOT running; the caller's copy
      // must not claim it is.
      return false;
    } on PlatformException {
      // The native side reported a failure. Best-effort: the backup screen
      // still functions, but the protection did not engage.
      return false;
    }
  }
}
