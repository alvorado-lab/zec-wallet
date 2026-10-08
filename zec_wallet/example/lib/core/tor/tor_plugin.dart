import 'dart:io' show Directory, Platform;

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:zec_wallet/zec_wallet.dart'
    show CustodyDisclosure, SyncServerChoice;
import 'package:zec_wallet_tor/zec_wallet_tor.dart';
import 'package:zec_wallet_ui/zec_wallet_ui.dart';

/// The example's use of the optional Tor plugin (`zec_wallet_tor`, FR-5):
/// the settings toggle, the boot order, and the wipe order. Spec
/// `docs/specs/tor-plugin.md` §2.3 and §8 P25/P27; plan
/// `docs/plan/fr5-phase-1.md` C3c.
///
/// OFF by default, and off means the plugin is never touched: no native call,
/// no directory, the wallet opens with `TorPolicy.off()` exactly as before.
/// This app is installed on phones that hold real funds; an upgrade must
/// change nothing for a user who never flips the toggle.
///
/// The one host-side stream rule, for a host that shows the plugin's status:
/// subscribe to `ZecWalletTor.statusStream` from a provider that watches
/// `appLifecycleProvider` and pauses the subscription on
/// `AppLifecycleState.paused`, resuming on a REAL `resumed` (the rule
/// documented on `appLifecycleProvider`). The plugin's own pause and resume
/// do not depend on it. This example does not show the plugin's status: the
/// wallet's own transport state already renders "Tor" and its readiness.

// --- The toggle ------------------------------------------------------

/// The persisted choice. Not a rendering preference like its neighbours in
/// `appearance_prefs.dart`: it chooses the wallet's transport at boot. It is
/// the user's setting, not application data (no wallet state, nothing
/// Rust-owned), and it is read once, before the wallet opens.
const useTorPluginPrefsKey = 'use_tor_plugin';

/// Loaded before `runApp` with the other prefs and injected via override.
final initialUseTorPluginProvider = Provider<bool>((ref) => false);

class UseTorPluginNotifier extends Notifier<bool> {
  @override
  bool build() => ref.read(initialUseTorPluginProvider);

  /// Persists the choice. It takes effect at the next start: the transport
  /// is chosen before the wallet opens and is not swapped under a live one.
  Future<void> set(bool value) async {
    if (value == state) return;
    state = value;
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setBool(useTorPluginPrefsKey, value);
    } catch (_) {
      // Best-effort persist; the in-session value stands.
    }
  }
}

final useTorPluginProvider = NotifierProvider<UseTorPluginNotifier, bool>(
  UseTorPluginNotifier.new,
);

/// The persisted choice, read at boot. Unset is OFF. Unlike the cosmetic
/// prefs, an UNREADABLE store is not defaulted: it THROWS, so the boot attempt
/// fails and retries rather than quietly opening a Tor user's wallet on a
/// direct connection. Bounded like the other pre-`runApp` reads.
Future<bool> readUseTorPlugin({
  Future<SharedPreferences> Function() open = SharedPreferences.getInstance,
}) async {
  final prefs = await open().timeout(const Duration(seconds: 5));
  return prefs.getBool(useTorPluginPrefsKey) ?? false;
}

// --- Where the plugin keeps its state --------------------------------

/// The plugin's directory: a SIBLING of the wallet's `dbDir` — its parent
/// plus `tor` — never `dbDir` itself nor anything inside it. The wallet's
/// wipe deletes every entry of `dbDir`; a Tor tree there would be deleted
/// under arti's open files and leave a fresh guard identifier behind a wipe
/// reported as done (spec §2.3, E22).
String torDirFor(String dbDir) {
  final wallet = Directory(dbDir).absolute;
  final tor = '${wallet.parent.path}${Platform.pathSeparator}tor';
  if (tor == wallet.path ||
      tor.startsWith('${wallet.path}${Platform.pathSeparator}')) {
    throw ArgumentError.value(dbDir, 'dbDir', 'its sibling "tor" is itself');
  }
  return tor;
}

/// [torDirFor], created and excluded from iOS backup like the wallet's own
/// directory (Android: `allowBackup="false"` app-wide). The plugin's files
/// are unencrypted and name this device's Tor guards.
Future<String> resolveTorDir(String dbDir) async {
  final dir = Directory(torDirFor(dbDir));
  await dir.create(recursive: true);
  await const BackupExclusion().exclude(dir.path);
  return dir.path;
}

// --- The seam over the plugin ----------------------------------------

/// What the example asks of the plugin, so the boot and wipe orders are
/// testable without its native library.
abstract interface class TorPluginControl {
  Future<void> init({required String torDir});
  Future<void> dispose();
  Future<void> clearState(String torDir);
}

/// The production [TorPluginControl]: the plugin's static surface.
final class ZecWalletTorControl implements TorPluginControl {
  const ZecWalletTorControl();

  @override
  Future<void> init({required String torDir}) async {
    await ZecWalletTor.init(torDir: torDir);
  }

  @override
  Future<void> dispose() => ZecWalletTor.dispose();

  @override
  Future<void> clearState(String torDir) => ZecWalletTor.clearState(torDir);
}

// --- Boot ------------------------------------------------------------

/// What the boot attempt needs from the Tor choice.
final class TorBoot {
  const TorBoot({required this.policy, this.decorateProvisioner});

  /// The wallet's Tor policy.
  final TorPolicy policy;

  /// For `walletOnboardingOverrides`: wraps the provisioner so a delete
  /// follows the plugin's wipe order. `null` when the plugin is off.
  final WalletProvisioner Function(WalletProvisioner)? decorateProvisioner;
}

/// The Tor half of a boot attempt. Runs AFTER the wallet's FFI init (the
/// plugin finds the wallet's library in the process) and BEFORE the wallet
/// opens (the wallet refuses `hostDialer()` while nothing is registered).
///
/// [enabled] false → `TorPolicy.off()`, and the plugin is not touched.
/// [enabled] true → `ZecWalletTor.init`, then `TorPolicy.required_` over the
/// registered transport: this app never falls back to a direct connection
/// behind the user's back. A failed init THROWS (a `TorPluginError`), and the
/// boot attempt reports the couldn't-start screen: a user who chose Tor is
/// never silently given a direct connection instead.
Future<TorBoot> bootTorPlugin({
  required bool enabled,
  required String dbDir,
  TorPluginControl tor = const ZecWalletTorControl(),
  Future<String> Function(String dbDir) resolveDir = resolveTorDir,
  Future<bool> Function() offerIdentityReset = _no,
  Future<bool> Function() reportIdentityResetFailed = _no,
}) async {
  if (!enabled) return const TorBoot(policy: TorPolicy.off());
  final torDir = await resolveDir(dbDir);
  await tor.init(torDir: torDir);
  return TorBoot(
    policy: const TorPolicy.required_(runtime: TorRuntimeConfig.hostDialer()),
    decorateProvisioner: (real) => TorAwareProvisioner(
      inner: real,
      tor: tor,
      torDir: torDir,
      offerIdentityReset: offerIdentityReset,
      reportIdentityResetFailed: reportIdentityResetFailed,
    ),
  );
}

Future<bool> _no() async => false;

// --- Wipe ------------------------------------------------------------

/// The wallet's provisioner with the plugin's wipe order around a delete
/// (spec §2.3): `dispose` → the wallet's wipe → OFFER `clearState` → `init`
/// again, so a wallet created next finds its transport registered.
///
/// Every other call passes straight through.
final class TorAwareProvisioner implements WalletProvisioner {
  TorAwareProvisioner({
    required this.inner,
    required this.tor,
    required this.torDir,
    required this.offerIdentityReset,
    required this.reportIdentityResetFailed,
  });

  final WalletProvisioner inner;
  final TorPluginControl tor;
  final String torDir;

  /// Asks the user whether to reset the Tor identity too; `true` = yes.
  final Future<bool> Function() offerIdentityReset;

  /// Tells the user a reset they ASKED for did not happen; `true` = try
  /// again. A reset the user chose is never dropped silently (honest
  /// degradation: the per-device relay record is still on disk).
  final Future<bool> Function() reportIdentityResetFailed;

  @override
  Future<void> deleteWallet() => _wipe(inner.deleteWallet);

  @override
  Future<void> forceDeleteWallet() => _wipe(inner.forceDeleteWallet);

  Future<void> _wipe(Future<void> Function() wipe) async {
    // 1. Stop Tor and give the slot back BEFORE the wipe: no Tor connection
    //    is open on the wallet's behalf while its files go. A plugin fault
    //    here never blocks the wipe: the wallet runs `Required`, so a stale
    //    registration can only fail closed.
    try {
      await tor.dispose();
    } catch (e) {
      debugPrint('Tor plugin dispose failed (${_kind(e)}); wiping anyway');
    }
    try {
      // 2. The wallet's wipe.
      await wipe();
    } catch (_) {
      // A faulted wipe deletes nothing and the controller reopens the
      // wallet — which needs its transport back.
      await _reinit();
      rethrow;
    }
    // 3. The Tor identity is not wallet data; a wallet wipe leaves it. Offer
    //    to reset it; never do it unasked. The wallet is already gone here, so
    //    nothing in this step may stop step 4 (a dialog fault included).
    try {
      if (await offerIdentityReset()) await _resetIdentity();
    } catch (e) {
      debugPrint('Tor identity reset step failed (${_kind(e)})');
    }
    // 4. Register again for the next wallet.
    await _reinit();
  }

  Future<void> _resetIdentity() async {
    while (true) {
      try {
        await tor.clearState(torDir);
        return;
      } catch (e) {
        debugPrint('Tor identity reset failed (${_kind(e)})');
        if (!await reportIdentityResetFailed()) return;
        // The plugin refuses clearState while it still runs — the case
        // where step 1's dispose failed. Stop it again before the retry, or
        // "Try again" could never succeed.
        try {
          await tor.dispose();
        } catch (e) {
          debugPrint('Tor plugin dispose failed again (${_kind(e)})');
        }
      }
    }
  }

  Future<void> _reinit() async {
    try {
      await tor.init(torDir: torDir);
    } catch (e) {
      // The next open reports the unregistered transport honestly.
      debugPrint('Tor plugin could not restart (${_kind(e)})');
    }
  }

  static String _kind(Object e) =>
      e is TorPluginError ? e.kind.name : '${e.runtimeType}';

  @override
  Future<bool> walletExists() => inner.walletExists();

  @override
  Future<WalletSession> createGenerated() => inner.createGenerated();

  @override
  Future<WalletSession> open() => inner.open();

  @override
  Future<WalletSession> restore(
    List<String> mnemonicWords, {
    DateTime? approximateCreationTime,
  }) => inner.restore(
    mnemonicWords,
    approximateCreationTime: approximateCreationTime,
  );

  @override
  Future<WalletSession> createWatchOnly(
    String ufvk, {
    required int birthdayHeight,
  }) => inner.createWatchOnly(ufvk, birthdayHeight: birthdayHeight);

  @override
  Future<WalletSession> rescanFrom(RescanTarget target) =>
      inner.rescanFrom(target);

  @override
  Future<WalletSession> switchSyncServer(SyncServerChoice choice) =>
      inner.switchSyncServer(choice);

  @override
  int estimateBirthdayHeight(DateTime time) =>
      inner.estimateBirthdayHeight(time);

  @override
  Future<List<String>> revealMnemonic() => inner.revealMnemonic();

  @override
  Future<String> exportUfvk() => inner.exportUfvk();

  @override
  Future<CustodyDisclosure> custodyDisclosure() => inner.custodyDisclosure();
}
