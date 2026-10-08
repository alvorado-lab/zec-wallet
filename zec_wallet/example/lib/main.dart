import 'dart:async';
import 'dart:io' show Platform;
import 'dart:typed_data' show Uint8List;

import 'package:flutter/material.dart';
// `Override` (the list element type) is surfaced from misc.dart in Riverpod 3.x.
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:shared_preferences/shared_preferences.dart';

import 'package:zec_wallet_tor/zec_wallet_tor.dart' show TorPluginError;
import 'package:zec_wallet_ui/zec_wallet_ui.dart';

import 'core/boot/wallet_boot_gate.dart';
import 'features/diagnostics/machine_memo_screen.dart'
    show kMachineMemoDemoPrefix;
import 'features/settings/tor_identity_reset_dialog.dart';
import 'core/ffi/wallet_ffi.dart';
import 'core/logging/device_log.dart';
import 'core/router/router.dart';
import 'core/theme/appearance_prefs.dart';
import 'core/theme/theme_mode_provider.dart';
import 'core/tor/tor_plugin.dart';

/// Bootstrap budget (app-frame spec §7): the persisted-theme disk read (for
/// a flash-free first frame), then the wallet FFI init stage (the slot §7
/// reserved), then the wallet onboarding wiring. Startup work is BOUNDED
/// (#356-F1 — a hung native load or path_provider can't hold the splash
/// forever) and guarded: a failure never blocks boot — but since #356-F1 it
/// boots into the HONEST couldn't-start screen with a retry, never the
/// unwired seams' "arrives in a later build" copy (false on a build that
/// ships the wallet).
Future<void> main() async {
  final binding = WidgetsFlutterBinding.ensureInitialized();
  // E2E TEST INFRA (#400, the device-walk finding F2). Flutter builds the
  // semantics tree only on demand, so a device run has nothing to assert a11y
  // labels against and `uiautomator dump` returns the bare FrameLayout shell —
  // every device session falls back to screenshot-and-measure, which is slow and
  // mis-taps (the walk hit "Restore" instead of "Create" twice, from
  // estimated coordinates, on a money surface).
  //
  // `--dart-define=ZEC_WALLET_SEMANTICS=1` forces the FRAMEWORK to build the
  // tree, which is what an `integration_test`/driver-attached run reads.
  //
  // ⚠ ON ITS OWN IT DOES NOT MAKE `uiautomator dump` WORK, and the first version
  // of this comment claimed it did. MEASURED on the Samsung (One UI) with this
  // flag AND `settings put secure accessibility_enabled 1`: still nothing but the
  // FrameLayout shell. Publishing to `uiautomator` goes through Android's
  // accessibility node provider, which the engine feeds only when
  // `AccessibilityManager.isEnabled()` — and that needs a genuinely BOUND
  // service, which a settings flag is not. TalkBack is one, but it steals focus
  // and then intercepts `input tap` via explore-by-touch.
  //
  // THE ADB-DRIVEN WALK IS FIXED SEPARATELY, and verified: a debug-only stub
  // service, `android/app/src/debug/.../E2eSemanticsService.kt`, bound over adb.
  // With it the dump returns the real tree. Three notes for whoever uses it:
  //
  //  1. Flutter publishes labels as `content-desc`, NOT `text` — grepping
  //     `text="…"` reads empty even on a healthy tree, which is how an earlier
  //     probe was misread as a failure.
  //  2. A TEXT FIELD's label is not in the dump AT ALL (#401 R3a). Flutter routes
  //     it to `AccessibilityNodeInfo` hint text, and `uiautomator dump` emits
  //     neither `hint` nor anything else carrying it — so an EditText with an
  //     empty `text` and `content-desc` is the NORMAL, healthy shape. Reading that
  //     as "the field has no accessible name" cost a whole fix that had to be
  //     reverted; the semantics tree (a widget test over `getSemanticsData`) is
  //     the authority for labels, not the dump.
  //  3. This flag stays useful independently: it is what an
  //     `integration_test`/driver run needs, with no service bound at all.
  //
  // Off by default and gated on the flag alone: a release build never passes it,
  // and the semantics tree carries user-facing strings — a debugging affordance,
  // not a shipping one.
  if (const bool.fromEnvironment('ZEC_WALLET_SEMANTICS')) {
    binding.ensureSemantics();
  }
  final prefs = await _loadAppearancePrefs();
  // MOBILE: the first attempt runs under the NATIVE splash (the pre-#356
  // frame behavior on the happy path — no extra Flutter splash frame).
  // DESKTOP (reliability MED): there IS no native splash — an awaited
  // slow attempt would be up to 30s of frozen blank window — so runApp goes
  // up immediately and the gate runs the first attempt behind its themed
  // pending shell.
  final mobile = Platform.isAndroid || Platform.isIOS;
  final overrides = mobile ? await _bootWalletOverrides(prefs) : null;
  runApp(
    WalletBootGate(
      prefs: prefs,
      firstAttempt: overrides,
      attemptedPreRunApp: mobile,
      boot: () => _bootWalletOverrides(prefs),
    ),
  );
}

/// One boot attempt: FFI init + data-dir resolution + the full ProviderScope
/// override list. Returns `null` when the wallet couldn't start (the boot
/// gate renders [WalletStartupFailedScreen] and retries via this same
/// function — both stages are retry-safe: the FFI init is memoized, the dir
/// resolution is idempotent). Never throws.
Future<List<Override>?> _bootWalletOverrides(AppearanceBootPrefs prefs) async {
  // EVERYTHING rides the try — including the final list assembly — so the
  // "never throws" contract is structural: a throw anywhere here would
  // otherwise die pre-runApp (or strand the retry), the exact dead-boot shape
  // this function exists to prevent.
  try {
    // The two independent stages run CONCURRENTLY (the dir resolution needs
    // no FFI), so the worst-case pre-first-frame wait is ONE 30s budget, not
    // two in series. Both are individually bounded; the FFI init is memoized
    // (wallet_ffi) and the dir resolution is idempotent, so a wasted dir
    // resolve on a failed-FFI attempt is harmless.
    final (ffiReady, dbDir) = await (
      initWalletFfi(),
      resolveWalletDbDir().timeout(const Duration(seconds: 30)),
    ).wait;
    if (!ffiReady) return null;
    // FR-5, the optional Tor plugin: AFTER the wallet FFI init (the plugin
    // finds the wallet's library in the process) and BEFORE the wallet opens
    // (the wallet refuses `hostDialer()` while nothing is registered). Off —
    // the default — touches nothing and keeps `TorPolicy.off()`. A choice that
    // cannot be read, or a plugin that cannot start, fails THIS attempt (the
    // couldn't-start screen and its retry), never a quiet direct connection.
    final useTorPlugin = await readUseTorPlugin();
    final torBoot = await bootTorPlugin(
      enabled: useTorPlugin,
      dbDir: dbDir,
      offerIdentityReset: offerTorIdentityReset,
      reportIdentityResetFailed: reportTorIdentityResetFailed,
    );
    // This reference app rides the reference wallet policy (zec.rocks / Tor off /
    // ZEC_NETWORK+ZEC_ENDPOINT defines) — `buildWalletConfig`. A production host
    // constructs its OWN `WalletConfig` (its endpoint + Tor policy) and passes it
    // to the same seam; the reference builder is a convenience, not the contract.
    final walletOverrides = walletOnboardingOverrides(
      // FR-27: the example registers the DEMO read prefix so the machine-memo
      // round-trip diagnostic can read its own envelope back. Neutral bytes —
      // see `kMachineMemoDemoPrefix`; a real host registers its own magic.
      config: buildWalletConfig(
        dbDir: dbDir,
        machineMemoPrefixes: [Uint8List.fromList(kMachineMemoDemoPrefix)],
        // P3-13: the picker's offered list — the SDK's public reference
        // catalog (a host appends its own servers here, ADR-0568). The FFI
        // is up (`ffiReady` above), which this bridge call needs.
        syncServers: referenceOfferedServers(),
        tor: torBoot.policy,
      ),
      // With the plugin on: a wallet delete stops Tor first and offers the
      // Tor-identity reset after (the plugin's wipe order). Outermost, every
      // Delete — returned or thrown — re-arms the device log, which the SDK
      // closed at the wipe's start (stage S5 `row`; this app has no duress
      // path, and a host with one must not re-arm there).
      decorateProvisioner: (real) => DeviceLogRearmingProvisioner(
        inner: torBoot.decorateProvisioner?.call(real) ?? real,
        rearm: () => rearmDeviceLogAfterWipe(deviceLog),
      ),
    );
    // The COMPLETE override list, assembled once per successful attempt — the
    // boot gate stores it verbatim so rebuilds never hand ProviderScope fresh
    // override instances (an overrideWithValue identity change would re-fire
    // dependents, e.g. the swap activation).
    return [
      initialThemeModeProvider.overrideWithValue(prefs.themeMode),
      initialTextScaleProvider.overrideWithValue(prefs.textScale),
      initialAmoledProvider.overrideWithValue(prefs.amoled),
      initialUseTorPluginProvider.overrideWithValue(useTorPlugin),
      // The wallet package's one wallet→host navigation seam: wire the
      // example's own Appearance screen into the wallet's entry points
      // (a host that owns display settings elsewhere just leaves it null).
      walletAppearanceRoutePathProvider.overrideWithValue(AppRoutes.appearance),
      ...walletOverrides,
      // Swap policy (D-2b-2): the activation fires only once a real wallet
      // session exists, and a build without the `swap-near` adapter degrades
      // the enable to off.
      ...walletSwapOverrides(),
    ];
  } catch (e) {
    // Log the TYPE only (§5.4 — never an interpolated payload on a wallet
    // path). A ParallelWaitError is unwrapped per stage (reliability
    // LOW): the wrapper type alone erases which stage failed and why.
    if (e is ParallelWaitError<(bool?, String?), (AsyncError?, AsyncError?)>) {
      debugPrint(
        'wallet onboarding wiring failed (boot gate): '
        'ffi=${e.errors.$1?.error.runtimeType} '
        'dbDir=${e.errors.$2?.error.runtimeType}',
      );
    } else if (e is TorPluginError) {
      debugPrint(
        'wallet onboarding wiring failed (boot gate): '
        'tor=${e.kind.name}',
      );
    } else {
      debugPrint(
        'wallet onboarding wiring failed (boot gate): ${e.runtimeType}',
      );
    }
    return null;
  }
}

/// Persisted appearance prefs, loaded once before `runApp`.
Future<AppearanceBootPrefs> _loadAppearancePrefs() async {
  ThemeMode mode = ThemeMode.system;
  double scale = defaultTextScale;
  bool amoled = false;
  try {
    // BOUNDED (reliability MED): this is awaited before everything —
    // a wedged prefs channel must not hold the splash forever (F1's whole
    // thesis); past the budget the theme falls to defaults and boot proceeds.
    final prefs = await SharedPreferences.getInstance().timeout(
      const Duration(seconds: 5),
    );
    mode = themeModeFromString(prefs.getString(themeModePrefsKey));
    scale = textScaleFromPrefs(prefs);
    amoled = prefs.getBool(amoledPrefsKey) ?? false;
  } catch (_) {
    // SharedPreferences unavailable (rare) → defaults; cosmetic loss
    // only, the app must still boot (spec §6).
  }
  return AppearanceBootPrefs(themeMode: mode, textScale: scale, amoled: amoled);
}
