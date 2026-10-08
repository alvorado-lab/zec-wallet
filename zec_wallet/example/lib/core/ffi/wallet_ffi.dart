import 'dart:io' show Platform;

import 'package:flutter/foundation.dart';
import 'package:zec_wallet/zec_wallet.dart';

import '../logging/device_log.dart';

// The define's parser moved beside the stored choice (stage S5 `row`); kept
// reachable here for its existing tests.
export '../logging/device_log_prefs.dart' show deviceLogLevelFor;

/// The FFI init budget (#356-F1): a hung native load must never hold the app
/// on the splash forever — past this, boot proceeds to the honest
/// couldn't-start surface (whose retry re-awaits the SAME in-flight init;
/// see [initWalletFfi]).
const Duration walletFfiInitTimeout = Duration(seconds: 30);

/// The in-flight/settled `RustLib.init` future, memoized so [initWalletFfi]
/// is RETRY-SAFE: FRB forbids a second `init` on the same process, so a retry
/// after a TIMEOUT must re-await the same future rather than re-invoke init.
/// Cleared only on a genuine failure (the future is poisoned — a fresh
/// attempt is both safe and required).
Future<void>? _rustInit;

/// The production init, separated so the memo/deadline/recursion machinery —
/// the trickiest logic in this file — is testable with a scripted init
/// (reliability: a regression here, e.g. the sentinel collapsing back to
/// TimeoutException, would silently re-wedge every retry).
///
/// The device log is armed HERE, straight after init and before anything
/// touches the wallet — where the SDK asks a host to set it: the user's
/// stored choice, else the `ZEC_WALLET_DEVICE_LOG` define, else the build
/// default (`resolveDeviceLogLevel`), then the subscription into the ring
/// Settings shares. A DIAGNOSTIC MUST NEVER COST THE WALLET: this future is
/// what [initWalletFfi] awaits, and a throw here would read as a FAILED init,
/// so everything after `RustLib.init` is fenced ([rustInitThenArmDeviceLog]).
Future<void> _defaultRustInit() => rustInitThenArmDeviceLog(
  rustLibInit: () => RustLib.init(externalLibrary: walletExternalLibrary()),
  log: deviceLog,
);

Future<void> Function() _rustInitFn = _defaultRustInit;

/// TEST-ONLY: reset the memo and (optionally) swap in a scripted init.
@visibleForTesting
void debugResetWalletFfi({Future<void> Function()? rustInit}) {
  _rustInit = null;
  _rustInitFn = rustInit ?? _defaultRustInit;
}

/// FFI bootstrap (app-frame spec §7 / §11 A4): initialize the Rust wallet
/// core exactly once, before `runApp`. Its own delineated stage — the frame
/// budget allows it (the §7 note reserved this slot for "when FFI init
/// lands").
///
/// iOS/macOS use the process loader here. That is correct for THIS app and
/// only because it carries ONE flutter_rust_bridge library: under
/// `use_frameworks!` (both Podfiles) the pod's `-force_load` output IS
/// `zec_wallet.framework`, so the framework the old comment called
/// non-existent does exist, and a host with a SECOND FRB library must open it
/// by name instead — the process lookup answers FRB's seven unprefixed
/// symbols from whichever image loaded first (SDK README § Building, corrected
/// flutter-patterns § flutter_rust_bridge). OWED: switch this to
/// `ExternalLibrary.open('zec_wallet.framework/zec_wallet')` with a macOS
/// integration run as the proof. Android/Linux/Windows use the default loader.
///
/// A failure is NON-FATAL to boot: the app still runs. Since #356-F1 a failed
/// (or timed-out) init routes to the WalletStartupFailedScreen with a retry —
/// never the "arrives in a later build" copy a genuinely-unwired build shows.
/// RETRY-SAFE + BOUNDED: the await is capped at [walletFfiInitTimeout], and
/// repeat calls re-await the one memoized init (FRB forbids double-init) —
/// so a retry after a timeout resolves the moment the slow init lands, and a
/// retry after a real failure starts a fresh attempt.
Future<bool> initWalletFfi() async {
  final wasFresh = _rustInit == null;
  final pending = _rustInit ??= _rustInitFn();
  try {
    // The deadline throws a PRIVATE sentinel, not TimeoutException: `timeout`
    // rethrows the underlying error type, so an init that itself failed with
    // a TimeoutException must not be mistaken for OUR deadline — that would
    // keep a settled-failed future memoized and wedge every retry (
    // security review LOW).
    await pending.timeout(
      walletFfiInitTimeout,
      onTimeout: () => throw const _FfiInitDeadline(),
    );
    return true;
  } on _FfiInitDeadline {
    // OUR deadline fired; the underlying init may still be in flight — keep
    // the memoized future: a later retry re-awaits it (calling RustLib.init
    // again would throw), and if it did eventually complete, that retry
    // returns true immediately.
    debugPrint('wallet FFI init timed out (wallet stays unavailable)');
    return false;
  } catch (e) {
    // A genuinely failed init: clear the memo so a retry attempts fresh.
    // (Caveat, verified against FRB 2.12: only a failure BEFORE the bridge
    // assigns its internal state — library load, codegen sanity — can retry
    // in-process; a Rust-initializer failure after that point makes every
    // re-init throw "initialize twice". That still degrades honestly here —
    // false, same screen — and the screen's copy already says "restart the
    // app" for the keeps-failing case.)
    _rustInit = null;
    // Log the error TYPE only, never the interpolated payload. Init carries
    // no wallet data today, but this catch-shape gets copied onto paths that
    // will (onboarding) — interpolating a bridge error onto a wallet-data
    // path could leak an address/amount/txid (§5.4). Leak-proof by default.
    debugPrint(
      'wallet FFI init failed (wallet stays unavailable): '
      '${e.runtimeType}',
    );
    // A STALE failed memo (an earlier call timed out, the init later failed)
    // just burned this call on the rethrow — run ONE fresh attempt now so the
    // user's retry tap acts instead of silently no-oping (reliability
    // LOW). `wasFresh` bounds the recursion to a single level.
    if (!wasFresh) return initWalletFfi();
    return false;
  }
}

/// The private deadline marker for [initWalletFfi]'s bounded await — distinct
/// from any error the underlying init can throw (see the timeout note above).
class _FfiInitDeadline implements Exception {
  const _FfiInitDeadline();
}

/// The FRB external-library selector. iOS/macOS take the process loader,
/// which is sound ONLY while this app carries a single FRB library (see the
/// library doc above, and the OWED switch to the scoped framework open);
/// other platforms use the default loader.
/// Public so the integration tests can init RustLib exactly as the app does.
ExternalLibrary? walletExternalLibrary() {
  if (Platform.isIOS || Platform.isMacOS) {
    return ExternalLibrary.process(iKnowHowToUseIt: true);
  }
  return null;
}
