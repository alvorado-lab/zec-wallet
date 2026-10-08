import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/screen_security_channel.dart';

/// The Android `FLAG_SECURE` adapter's Dart half (spec §3.2g iii-B-2-b), over a
/// MOCK platform channel (no real Activity). Pins the two method invocations,
/// the best-effort swallow (a native failure / missing handler never throws into
/// the backup screen), and the honest per-platform capability gate.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  final channel = const MethodChannel(MethodChannelScreenSecurity.channelName);
  final messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;

  tearDown(() => messenger.setMockMethodCallHandler(channel, null));

  test(
    'enable() invokes enableSecure on the channel and reports ENGAGED',
    () async {
      final calls = <String>[];
      messenger.setMockMethodCallHandler(channel, (call) async {
        calls.add(call.method);
        return null;
      });
      expect(await MethodChannelScreenSecurity().enable(), isTrue);
      expect(calls, ['enableSecure']);
    },
  );

  test('an UNWIRED host channel reports NOT engaged (S149 review B1) — the '
      'seed screen must never claim a block that is not running', () async {
    // No mock handler registered: invokeMethod throws MissingPluginException,
    // the exact shape of a host app that skipped the MainActivity wiring.
    expect(await MethodChannelScreenSecurity().enable(), isFalse);
  });

  test('disable() invokes disableSecure on the channel', () async {
    final calls = <String>[];
    messenger.setMockMethodCallHandler(channel, (call) async {
      calls.add(call.method);
      return null;
    });
    await MethodChannelScreenSecurity().disable();
    expect(calls, ['disableSecure']);
  });

  test(
    'a native PlatformException is swallowed (best-effort, never throws)',
    () async {
      messenger.setMockMethodCallHandler(channel, (call) async {
        throw PlatformException(code: 'ERR', message: 'native failure');
      });
      // The protection is defence-in-depth, not the gate — the backup screen must
      // keep working even if the native side errors.
      await expectLater(MethodChannelScreenSecurity().enable(), completes);
    },
  );

  test('a missing handler (non-Android / no host) is swallowed', () async {
    // No mock handler registered → invokeMethod throws MissingPluginException,
    // which the adapter swallows (the capability gate already told the truth).
    messenger.setMockMethodCallHandler(channel, null);
    await expectLater(MethodChannelScreenSecurity().disable(), completes);
  });

  test('reports the screenshot-block capability honestly for this platform', () {
    // FLAG_SECURE is Android-only; the host VM is not Android, so the capability
    // is false here. (On Android the same gate returns true — flutter-patterns
    // § Platform Capability Gating.)
    expect(MethodChannelScreenSecurity().isScreenshotBlockSupported, isFalse);
  });

  test('the NoopScreenSecurity blocks nothing — capability is false always', () {
    // A no-op cannot claim a protection on ANY platform (arch fold): the value
    // must be false unconditionally, so a (never-wired-in-prod) Android no-op
    // can't drive a "screenshots are off" copy that isn't true.
    expect(const NoopScreenSecurity().isScreenshotBlockSupported, isFalse);
  });

  test('the channel name matches the Android MainActivity handler (gate 7)', () {
    // The Dart const and the Kotlin handler name must agree — a rename on one
    // side silently breaks FLAG_SECURE on-device while the mock tests above stay
    // green (they register against this same const). Pin the wire string.
    expect(
      MethodChannelScreenSecurity.channelName,
      'zec_wallet_ui/screen_security',
    );
  });
}
