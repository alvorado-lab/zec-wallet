import 'dart:io' show Directory;

// `Override` is surfaced from misc.dart in Riverpod 3.x (not the main barrel);
// the provider `.overrideWithValue` methods resolve via the provider types.
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:path_provider/path_provider.dart';
import 'package:zec_wallet/zec_wallet.dart' show WalletConfig;

import 'backup_exclusion.dart';
import 'onboarding/frb_wallet_provisioner.dart';
import 'onboarding/onboarding_providers.dart';
import 'onboarding/shared_prefs_onboarding_store.dart';
import 'onboarding/wallet_provisioner.dart';
import 'wallet_config.dart';
import 'wallet_providers.dart' show walletEndpointHostProvider;

/// The wallet onboarding COMPOSITION ROOT (spec §3.2g iii-B-2-b): the one place
/// that wires the production adapters into the onboarding seams. Building the
/// provisioner, the store, and the screen-security adapter TOGETHER (with the
/// co-wiring assertion below) is what makes them impossible to wire apart — a
/// refactor cannot ship the seed-revealing provisioner without the screenshot
/// protection on a platform that supports it (crypto audit H2).

/// Resolve the wallet data directory to a PLAIN, already-created path — call
/// this in `main()`, fold it into the host's [WalletConfig] (the reference
/// [buildWalletConfig] does this), and pass that config to
/// [walletOnboardingOverrides] BEFORE building the `ProviderScope`. This is a
/// REFERENCE helper (it picks a per-network leaf under the app support dir and
/// wires iOS backup-exclusion); a host with its own data-dir convention
/// resolves its own path and puts it on its `WalletConfig.dbDir`. It must NOT
/// be recomputed inside a provider
/// `build()`: an async re-resolve on every rebuild would spawn a fresh
/// provisioner mid-flight and risk a double-create (the footgun the port and
/// controller both warn about).
///
/// Uses the app SUPPORT directory (not Documents — on iOS Documents is
/// user-visible in the Files app). Backup-exclusion is wired on BOTH platforms:
/// Android off app-wide (`allowBackup=false`), iOS via [BackupExclusion] on this
/// dir (set below). Excluding the DB matters because its key is a `ThisDeviceOnly`
/// Secure-Enclave key that never migrates — a backed-up DB restored onto a new
/// device can't be opened (the needsRecovery trap), so excluding it routes a
/// device-restore to the clean Welcome → Restore flow instead.
Future<String> resolveWalletDbDir() async {
  final support = await getApplicationSupportDirectory();
  // Per-network leaf (`zec_wallet` mainnet / `zec_wallet_testnet` testnet): a
  // v-5c testnet-proof build lives BESIDE a mainnet wallet, never over it — the
  // SDK would refuse the mismatched DB anyway (NetworkMismatch), but the leaf
  // makes the define switch non-destructive in both directions.
  final dir = Directory('${support.path}/${referenceDbDirLeaf()}');
  // The lock-free existence PROBE does not create the dir (a missing dir just
  // reads "no wallet"); create/open DO create it at lock acquisition. Still
  // pre-create it here so the backup-exclusion flag below lands on a real
  // directory before any wallet files exist inside it.
  await dir.create(recursive: true);
  // iOS: exclude the wallet DB from device/iCloud backup (best-effort; never blocks
  // boot). Set AFTER the dir exists; the flag applies to the whole subtree, so all
  // DB files created inside it later are excluded too. (Android: allowBackup=false.)
  await const BackupExclusion().exclude(dir.path);
  return dir.path;
}

/// The co-wiring invariant (crypto audit H2), as a pure predicate so it is
/// directly testable (true AND false cases) without tripping an assert: a REAL
/// provisioner — which can reveal the seed onto the backup screen — must ship
/// with a REAL screen-security adapter on any platform that can block the
/// screen. Where no block exists (iOS/desktop/web), the honest [NoopScreenSecurity]
/// is correct and the rule holds.
bool screenSecurityCoWiringHolds({
  required bool provisionerIsReal,
  required ScreenSecurity security,
  required bool supportsBlock,
}) {
  if (!provisionerIsReal) return true; // no real seed access → no requirement
  if (!supportsBlock) return true; // honest Noop where no native block exists
  // Look THROUGH the refcount decorator (#344): the production seam wraps every
  // adapter in a RefCountedScreenSecurity, and the honest-Noop check is about
  // the EFFECTIVE adapter — a wrapped Noop must still read as a violation. Peel
  // every layer so the check is idempotent even under an (unexpected) double
  // wrap.
  ScreenSecurity effective = security;
  while (effective is RefCountedScreenSecurity) {
    effective = effective.inner;
  }
  return effective is! NoopScreenSecurity;
}

/// The production onboarding seam overrides, co-wired.
///
/// [config] is the host's OWN [WalletConfig] — endpoint, network, Tor policy,
/// seed persistence, broadcast jitter, and the resolved `dbDir` all come from
/// the host (the SDK validates the config; it does not choose it). A host with
/// its own lightwalletd/Tor infrastructure constructs its `WalletConfig`
/// directly; a host that wants the reference defaults (zec.rocks, Tor off)
/// passes [buildWalletConfig] verbatim — both reach this seam the same way, so
/// the ONE ergonomic wiring call is no longer coupled to the reference policy.
///
/// [supportsScreenshotBlock] defaults to the real platform capability and is
/// injectable so a test can drive the Android branch on the host VM.
///
/// [decorateProvisioner] lets a host wrap the real provisioner — to run its
/// own steps around a wallet lifecycle call — without re-wiring the seam: the
/// decorator receives the real provisioner and returns the one the wallet UI
/// uses. It must DELEGATE to what it was given (the co-wiring rule above
/// holds because the real provisioner is still the one underneath). The
/// reference use is the example's Tor plugin: `deleteWallet` stops the
/// plugin before the wipe and offers the Tor-identity reset after it.
List<Override> walletOnboardingOverrides({
  required WalletConfig config,
  bool? supportsScreenshotBlock,
  WalletProvisioner Function(WalletProvisioner provisioner)?
  decorateProvisioner,
}) {
  // Programming-error belt: an EMPTY or RELATIVE dbDir is
  // never right — on desktop a relative path would silently create the wallet
  // under the launch cwd, so a funded wallet "disappears" when the app is
  // launched from elsewhere and onboarding offers a fresh create over it.
  // Loud at wiring time; the Rust config door rejects the same shapes typed
  // (InvalidDbDir, RW-CFG-003 — landed with #324, covering direct-SDK
  // consumers too). This belt stays: an ArgumentError at wiring names the
  // bug instantly, where the typed door surfaces later, at create/open.
  if (config.dbDir.isEmpty || !Directory(config.dbDir).isAbsolute) {
    throw ArgumentError.value(
      config.dbDir,
      'config.dbDir',
      'must be an absolute path (see resolveWalletDbDir for the reference '
          'resolution)',
    );
  }
  final supportsBlock =
      supportsScreenshotBlock ?? defaultSupportsScreenshotBlock();
  final real = FrbWalletProvisioner(config: config);
  final provisioner = decorateProvisioner?.call(real) ?? real;
  final store = SharedPrefsOnboardingStore();
  // Refcount-wrap the selected adapter (#344) so stacked secure screens share
  // one flag — via the single wrap chokepoint. The co-wiring assert below
  // unwraps to check the effective adapter.
  final security = refCountedScreenSecurityFor(
    supportsNativeBlock: supportsBlock,
  );

  // Co-wired by construction; the assert is a future-edit tripwire, HONESTLY
  // scoped: it can only ever bite in a host-VM test that injects
  // supportsScreenshotBlock: true (on a real device build the debug deferral
  // makes supportsBlock false in debug, and asserts strip in release). The
  // guard that actually covers a release device is screenSecurityProvider's
  // platform-correct DEFAULT (onboarding_providers.dart), not this assert.
  assert(
    screenSecurityCoWiringHolds(
      provisionerIsReal: true,
      security: security,
      supportsBlock: supportsBlock,
    ),
    'a real WalletProvisioner must co-wire a non-no-op ScreenSecurity where the '
    'platform can block the screen (crypto audit H2)',
  );

  // The sync sheet's "Server" row: the endpoint HOST, derived
  // from the SAME config the provisioner opens with, so the display and the
  // actual connection can never drift. Host only — never the full URL.
  final endpointHost = Uri.tryParse(config.endpointUrl)?.host;

  return [
    walletProvisionerProvider.overrideWithValue(provisioner),
    onboardingStoreProvider.overrideWithValue(store),
    screenSecurityProvider.overrideWithValue(security),
    if (endpointHost != null && endpointHost.isNotEmpty)
      walletEndpointHostProvider.overrideWithValue(endpointHost),
  ];
}
