// P27 (docs/specs/tor-plugin.md §8): the example's wipe flow calls
// `ZecWalletTor.dispose()` BEFORE `wallet.wipe()` and offers `clearState()`
// after; `clearState()` after a wipe succeeds; the example never passes the
// wallet's `db_dir` as `torDir` (E22, §2.3).
import 'dart:io' show Platform;

import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_example/core/tor/tor_plugin.dart';

import 'support/fake_tor_plugin.dart';

const _dbDir = '/data/user/0/app/files/zec_wallet';

void main() {
  late List<String> log;
  late FakeTorPlugin tor;
  late LoggingProvisioner wallet;

  TorAwareProvisioner decorated({
    required bool acceptReset,
    List<bool> retries = const [],
  }) {
    final answers = [...retries];
    return TorAwareProvisioner(
      inner: wallet,
      tor: tor,
      torDir: torDirFor(_dbDir),
      offerIdentityReset: () async {
        log.add('offer');
        return acceptReset;
      },
      reportIdentityResetFailed: () async {
        log.add('reset-failed');
        return answers.isNotEmpty && answers.removeAt(0);
      },
    );
  }

  setUp(() {
    log = [];
    tor = FakeTorPlugin(log);
    wallet = LoggingProvisioner(log)..exists = true;
  });

  // security review, L3: a Tor fault must never block a wallet wipe.
  test('a failing Tor dispose does not stop the wipe', () async {
    tor.failDispose = StateError('panicked');
    await decorated(acceptReset: false).forceDeleteWallet();
    expect(log, ['tor.dispose', 'wallet.forceWipe', 'offer', 'tor.init']);
    expect(wallet.exists, isFalse);
  });

  // security review, L2: a reset the user asked for is never dropped
  // silently — it is reported, and a retry runs it again.
  test('a failed identity reset is reported and can be retried', () async {
    tor.clearStateFailures = 1;
    await decorated(acceptReset: true, retries: [true]).deleteWallet();
    expect(log, [
      'tor.dispose',
      'wallet.wipe',
      'offer',
      'tor.clearState!',
      'reset-failed',
      // A retry stops the plugin again first: clearState is refused while
      // it runs, which is the case when step 1's dispose failed.
      'tor.dispose',
      'tor.clearState',
      'tor.init',
    ]);
  });

  // fold review, L5: the wallet is already gone at step 3, so a fault
  // there (a dialog that throws) must not stop the plugin re-registering.
  test('a throwing reset dialog still re-registers the plugin', () async {
    final p = TorAwareProvisioner(
      inner: wallet,
      tor: tor,
      torDir: torDirFor(_dbDir),
      offerIdentityReset: () async => throw StateError('no navigator'),
      reportIdentityResetFailed: () async => false,
    );
    await p.deleteWallet();
    expect(log, ['tor.dispose', 'wallet.wipe', 'tor.init']);
    expect(wallet.exists, isFalse);
  });

  test('declining the retry after a failed reset stops there', () async {
    tor.clearStateFailures = 5;
    await decorated(acceptReset: true).deleteWallet();
    expect(log, [
      'tor.dispose',
      'wallet.wipe',
      'offer',
      'tor.clearState!',
      'reset-failed',
      'tor.init',
    ]);
  });

  test('dispose BEFORE the wipe, the offer after, the reset only on yes, '
      'then the plugin registers again', () async {
    await decorated(acceptReset: true).deleteWallet();
    expect(log, [
      'tor.dispose',
      'wallet.wipe',
      'offer',
      'tor.clearState',
      'tor.init',
    ]);
    expect(wallet.exists, isFalse);
  });

  test('declining the offer keeps the Tor state', () async {
    await decorated(acceptReset: false).deleteWallet();
    expect(log, ['tor.dispose', 'wallet.wipe', 'offer', 'tor.init']);
  });

  test('the force delete follows the same order', () async {
    await decorated(acceptReset: true).forceDeleteWallet();
    expect(log, [
      'tor.dispose',
      'wallet.forceWipe',
      'offer',
      'tor.clearState',
      'tor.init',
    ]);
  });

  test(
    'a faulted wipe brings Tor back for the reopen and offers nothing',
    () async {
      wallet.failDelete = StateError('keychain wedged');
      await expectLater(
        decorated(acceptReset: true).deleteWallet(),
        throwsStateError,
      );
      expect(log, ['tor.dispose', 'wallet.wipe', 'tor.init']);
    },
  );

  test('clearState runs AFTER the wallet is gone, on the Tor directory', () {
    return decorated(acceptReset: true).deleteWallet().then((_) {
      final wipedAt = log.indexOf('wallet.wipe');
      expect(log.indexOf('tor.clearState'), greaterThan(wipedAt));
      expect(tor.torDirs, contains(torDirFor(_dbDir)));
    });
  });

  test('the Tor directory is a SIBLING of the wallet directory, never it or '
      'inside it', () {
    final sep = Platform.pathSeparator;
    final torDir = torDirFor(_dbDir);
    expect(torDir, '/data/user/0/app/files${sep}tor');
    expect(torDir, isNot(_dbDir));
    expect(torDir.startsWith('$_dbDir$sep'), isFalse);
    // Both network leaves share one Tor directory beside them.
    expect(torDirFor('/data/user/0/app/files/zec_wallet_testnet'), torDir);
    // A wallet directory whose sibling would be itself is refused.
    expect(() => torDirFor('/data/user/0/app/files/tor'), throwsArgumentError);
  });

  test(
    'the boot hands the plugin the sibling, not the wallet directory',
    () async {
      final boot = await bootTorPlugin(
        enabled: true,
        dbDir: _dbDir,
        tor: tor,
        resolveDir: (d) async => torDirFor(d),
      );
      expect(tor.torDirs.single, isNot(_dbDir));
      expect(tor.torDirs.single, torDirFor(_dbDir));
      // And the wipe decorator carries the same directory.
      final p = boot.decorateProvisioner!(wallet) as TorAwareProvisioner;
      expect(p.torDir, torDirFor(_dbDir));
    },
  );
}
