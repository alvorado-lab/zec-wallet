import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/backup_exclusion.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/screen_security_channel.dart';
import 'package:zec_wallet_ui/features/wallet/reconnect_kick.dart';
import 'package:zec_wallet_ui_platform/zec_wallet_ui_platform.dart';

/// Contract test — the whole point of this plugin is to answer the platform
/// channels `zec_wallet_ui` already speaks. If either side renames a channel,
/// the wiring breaks silently at runtime (the seed screen would call a channel
/// nobody answers → honest-but-unprotected). Pin the two names equal so a
/// rename is a red test, not a field bug.
void main() {
  test('screen-security channel name matches the zec_wallet_ui contract', () {
    expect(kScreenSecurityChannel, MethodChannelScreenSecurity.channelName);
  });

  test('backup-exclusion channel name matches the zec_wallet_ui contract', () {
    expect(kBackupExclusionChannel, BackupExclusion.channelName);
  });

  test('network-reachability channel name matches the zec_wallet_ui '
      'contract (#404)', () {
    // A rename here fails SILENTLY in the worst way: the wallet keeps working,
    // the reconnect auto-heal just never fires, and the only symptom is a badge
    // that sits on "Sync paused" for minutes — indistinguishable from the bug
    // this plugin half exists to fix.
    expect(
      kNetworkReachabilityChannel,
      EventChannelNetworkReachability.channelName,
    );
  });
}
