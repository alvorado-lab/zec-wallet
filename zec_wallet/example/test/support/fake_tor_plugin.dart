import 'package:zec_wallet_example/core/tor/tor_plugin.dart';
import 'package:zec_wallet_tor/zec_wallet_tor.dart';
import 'package:zec_wallet_ui/testing.dart';
import 'package:zec_wallet_ui/zec_wallet_ui.dart' show WalletSession;

/// A [TorPluginControl] that records into a log shared with
/// [LoggingProvisioner], so a test reads the ORDER of the plugin's and the
/// wallet's steps from one list.
class FakeTorPlugin implements TorPluginControl {
  FakeTorPlugin(this.log);

  final List<String> log;

  /// When set, the next `init` throws it.
  TorPluginError? failInit;

  /// When set, `dispose` throws it.
  Object? failDispose;

  /// How many `clearState` calls fail before one succeeds.
  int clearStateFailures = 0;

  /// Every `torDir` the example handed over.
  final List<String> torDirs = [];

  @override
  Future<void> init({required String torDir}) async {
    torDirs.add(torDir);
    log.add('tor.init');
    final f = failInit;
    if (f != null) throw f;
  }

  @override
  Future<void> dispose() async {
    log.add('tor.dispose');
    final f = failDispose;
    if (f != null) throw f;
  }

  @override
  Future<void> clearState(String torDir) async {
    if (clearStateFailures > 0) {
      clearStateFailures--;
      log.add('tor.clearState!');
      throw StateError('a file is still held');
    }
    torDirs.add(torDir);
    log.add('tor.clearState');
  }
}

/// The wallet-UI package's fake provisioner, logging the lifecycle calls a
/// test orders against the plugin's.
class LoggingProvisioner extends FakeWalletProvisioner {
  LoggingProvisioner(this.log);

  final List<String> log;

  @override
  Future<WalletSession> open() async {
    log.add('wallet.open');
    return super.open();
  }

  @override
  Future<void> deleteWallet() async {
    log.add('wallet.wipe');
    return super.deleteWallet();
  }

  @override
  Future<void> forceDeleteWallet() async {
    log.add('wallet.forceWipe');
    return super.forceDeleteWallet();
  }
}
