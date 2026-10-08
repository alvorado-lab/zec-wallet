import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/backup_exclusion.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const channel = MethodChannel(BackupExclusion.channelName);
  final binding = TestDefaultBinaryMessengerBinding.instance;

  List<MethodCall> mockChannel({Object? Function(MethodCall)? respond}) {
    final calls = <MethodCall>[];
    binding.defaultBinaryMessenger.setMockMethodCallHandler(channel, (
      call,
    ) async {
      calls.add(call);
      return respond?.call(call) ?? true;
    });
    addTearDown(
      () => binding.defaultBinaryMessenger.setMockMethodCallHandler(
        channel,
        null,
      ),
    );
    return calls;
  }

  test('on iOS, excludes the given path via the native channel', () async {
    final calls = mockChannel();
    await const BackupExclusion(isIOS: true).exclude('/wallet/db');

    expect(calls.single.method, 'excludeFromBackup');
    expect(calls.single.arguments, '/wallet/db');
  });

  test('off iOS it is a no-op — never touches the channel', () async {
    final calls = mockChannel();
    await const BackupExclusion(isIOS: false).exclude('/wallet/db');

    expect(calls, isEmpty);
  });

  test(
    'a native failure is SWALLOWED — boot is never blocked on the backup flag',
    () async {
      mockChannel(
        respond: (_) => throw PlatformException(code: 'exclude_failed'),
      );

      // Must complete normally despite the platform throwing.
      await expectLater(
        const BackupExclusion(isIOS: true).exclude('/wallet/db'),
        completes,
      );
    },
  );
}
