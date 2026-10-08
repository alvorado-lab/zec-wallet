import 'dart:async';
import 'dart:io' show Platform;

import 'package:flutter/foundation.dart'
    show debugPrint, kDebugMode, kIsWeb, visibleForTesting;
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'onboarding_store.dart';
import 'screen_security_channel.dart';
import 'wallet_provisioner.dart';

/// The single home for every onboarding SEAM (spec §3.2g iii-B-2-b): the
/// override points the on-device wiring swaps the production adapters into, plus
/// the screenshot-protection port. Consolidated here (the iii-B-2-b obligation)
/// so a reader finds the whole onboarding boundary in one place — the state
/// machine itself lives in `onboarding_controller.dart`, which reads these.
/// The one deliberate EXCEPTION: the seed-reveal future lives in
/// `mnemonic_reveal.dart`, kept OFF the barrel so it can never be an
/// autocomplete away from a host widget.

/// The wallet provisioner source. PRODUCTION default is `null` in a host-VM /
/// test build; `main()` overrides it with the real `FrbWalletProvisioner`
/// (wrapping the FRB `WalletHandle` + a path_provider `dbDir` + the host-policy
/// `WalletConfig`). Tests inject a fake. Overriding this ONE provider (plus
/// [onboardingStoreProvider] / [screenSecurityProvider], co-wired in the
/// composition root) is the whole wiring point — mirrors the
/// `walletSessionProvider` null seam.
final walletProvisionerProvider = Provider<WalletProvisioner?>((ref) => null);

/// The onboarding-progress store source. PRODUCTION default `null` (see
/// [walletProvisionerProvider]); `main()` overrides it with the
/// shared_preferences-backed `SharedPrefsOnboardingStore`. Tests inject an
/// in-memory fake.
final onboardingStoreProvider = Provider<OnboardingStore?>((ref) => null);

/// Requests platform screenshot/recents protection while a sensitive screen
/// (the recovery-phrase backup) is visible — a port (design invariant 2) so the
/// backup screen's request is host-VM testable with a fake, and the native
/// implementation is swappable per platform.
///
/// The real Android `FLAG_SECURE` adapter (`MethodChannelScreenSecurity`, a
/// platform channel to the host Activity) is wired in the composition root
/// ALONGSIDE the real `WalletProvisioner` (co-wiring assertion, crypto audit
/// H2) — so there is NO build in which real recovery words can be displayed
/// without the protection on a platform that supports it. In a host-VM / test
/// build the default is [NoopScreenSecurity] and the provisioner is unwired, so
/// the backup screen is never reached.
///
/// HONEST PLATFORM TRUTH: `FLAG_SECURE` is Android-only (flutter-patterns
/// § Platform Capability Gating). iOS has no equivalent screenshot block and
/// desktop has none either; on those platforms [enable] is honestly a no-op and
/// the backup screen tells the user to find a private setting instead of
/// implying a protection that does not exist. The screen's copy is ACK-driven —
/// it claims a block only on [enable]'s `true`.
///
/// REFCOUNTED at the seam (#344): the raw ADAPTER contract is a plain
/// window-level on/off switch — Android `FLAG_SECURE` is one Activity flag, so
/// the last [disable] wins. That is unsafe the moment two secure surfaces are
/// alive at once (a secure screen pushes another; a sheet/dialog is raised over
/// one): popping the top would clear the flag while the screen underneath still
/// shows a seed. The production seam therefore hands out every adapter wrapped
/// in a [RefCountedScreenSecurity], which keeps the flag engaged while ANY
/// holder is active — the 5 secure screens keep their plain enable-on-show /
/// disable-on-dispose pair unchanged. (The `_openingBackup` tile guard in
/// `security_screen.dart` is now only a UX double-push guard; the durable
/// screenshot-protection fix lives here.)
abstract interface class ScreenSecurity {
  /// Request that the OS prevent screenshots / hide the screen from the app
  /// switcher while the sensitive screen is visible. Best-effort and idempotent;
  /// a failure never breaks the screen (the protection is defence-in-depth,
  /// not the gate).
  ///
  /// Returns whether the protection actually ENGAGED (the native side
  /// acknowledged). The UI claims "screenshots are off" only on `true` — a
  /// capable-looking platform whose host never wired the native handler must
  /// read as UNPROTECTED, never as protected (extraction review B1: the
  /// old void signature let a missing handler be swallowed into a false
  /// security claim on the seed screen).
  Future<bool> enable();

  /// Release the protection when the sensitive screen leaves the tree.
  Future<void> disable();

  /// Whether THIS platform can actually block screenshots (Android only). The
  /// CAPABILITY signal used for adapter selection and by the test fakes — the
  /// UI copy no longer reads it (engagement truth is [enable]'s return).
  bool get isScreenshotBlockSupported;
}

/// The no-op binding: every method is a successful no-op. It blocks NOTHING, so
/// [isScreenshotBlockSupported] is honestly `false` REGARDLESS of platform — a
/// no-op cannot claim a protection. (The real Android block lives in the
/// `MethodChannelScreenSecurity` adapter, which reports `true` only on Android;
/// the composition root co-wires it there, so a block-capable platform is never
/// left with this no-op — but if one ever were, the UI copy stays honest rather
/// than promising a block that isn't running.)
class NoopScreenSecurity implements ScreenSecurity {
  const NoopScreenSecurity();

  @override
  Future<bool> enable() async => false; // a no-op never ENGAGES anything

  @override
  Future<void> disable() async {}

  @override
  bool get isScreenshotBlockSupported => false;
}

/// A reference-counting decorator over a [ScreenSecurity] adapter (#344).
///
/// The underlying protection is a WINDOW-level flag (Android `FLAG_SECURE` is
/// one Activity-window flag — inherently un-counted), but more than one secure
/// screen can be alive at once: a secure screen pushes another, or a sheet /
/// dialog is raised over one. If each screen's `dispose` called the adapter's
/// [disable] directly, popping the TOP screen would clear the flag while a
/// secure screen UNDERNEATH still shows a seed — and still claims "screenshots
/// off" (the stacked-twin race the `_openingBackup` tile guard only
/// partially closed).
///
/// This decorator keeps the adapter engaged while ANY holder is active: the
/// first [enable] (count 0→1) engages it, the last [disable] (1→0) releases it;
/// intermediate calls only move the count. Every call site keeps the plain
/// [enable]/[disable] pair — the counting is invisible to the secure screens.
///
/// Transitions are SERIALIZED on an internal future so a rapid
/// enable→disable→enable can never leave two native calls in flight out of
/// order, and a piggybacked [enable] awaits the in-flight engage before it reads
/// the shared engagement truth (so every stacked screen gets the same honest
/// "screenshots off" copy without re-invoking the native side). A transition
/// that throws is swallowed: it must never poison the chain (that would wedge
/// every future transition), and per the port contract a protection failure
/// never breaks the screen.
class RefCountedScreenSecurity implements ScreenSecurity {
  RefCountedScreenSecurity(this._inner, {Duration? transitionTimeout})
    : _transitionTimeout = transitionTimeout ?? const Duration(seconds: 5);

  final ScreenSecurity _inner;

  /// A native screen-security toggle is a sub-100 ms window operation, but the
  /// native handler is HOST-supplied (the reference `MainActivity.kt` acks, but
  /// a host code path that receives the call and never acks would leave
  /// [_inner.enable] pending FOREVER and WEDGE every future transition queued on
  /// [_tail]). Bound it: a hung call resolves as UNPROTECTED (honest) and the
  /// chain proceeds — the swallow below only catches THROWS, not non-completion,
  /// so the timeout is what actually delivers the "never wedge" guarantee.
  /// Generous vs real latency (mirrors the codebase's other channel timeouts),
  /// so it only ever fires for a truly-stuck host, never as a latency SLA.
  /// Injectable for tests.
  final Duration _transitionTimeout;

  /// The wrapped adapter. Exposed so the co-wiring assertion
  /// (`screenSecurityCoWiringHolds`) and the adapter-selection tests can see the
  /// effective protection THROUGH the decorator — an honest [NoopScreenSecurity]
  /// must stay detectable even when refcount-wrapped.
  ScreenSecurity get inner => _inner;

  /// Live holders. Never driven below zero (see [disable]).
  int _count = 0;

  /// Whether the adapter is currently ENGAGED (its last [enable] ack). Piggyback
  /// enables return this so all stacked screens read the same engagement truth.
  bool _engaged = false;

  /// Serializes adapter transitions (see class doc). Starts settled.
  Future<void> _tail = Future<void>.value();

  /// The live holder count — for tests asserting the coalescing directly.
  @visibleForTesting
  int get activeCount => _count;

  @override
  bool get isScreenshotBlockSupported => _inner.isScreenshotBlockSupported;

  @override
  Future<bool> enable() {
    _count++;
    if (_count == 1) {
      // 0→1: actually engage. Chained so the ack lands in [_engaged] in call
      // order and a concurrent transition can't race ahead of it.
      _tail = _tail.then((_) async {
        try {
          _engaged = await _inner.enable().timeout(
            _transitionTimeout,
            onTimeout: () => false,
          );
        } catch (_) {
          // Best-effort defence-in-depth: a failed engage reads as UNPROTECTED
          // and never poisons the transition chain.
          _engaged = false;
        }
      });
    }
    // First holder or piggyback: resolve after the settled transition and report
    // the shared engagement truth.
    return _tail.then((_) => _engaged);
  }

  @override
  Future<void> disable() {
    if (_count == 0) {
      // Unbalanced disable (more disables than enables). NEVER go negative — a
      // negative count would later skip a real 0→1 engage, leaving a seed screen
      // UNPROTECTED. Balanced by construction (each screen pairs its initState
      // enable with its dispose disable), so this only trips on a future
      // call-site bug: report in debug, clamp in every build, and never throw
      // out of a widget `dispose()`.
      assert(() {
        debugPrint(
          'RefCountedScreenSecurity.disable() called with no active holder '
          '(unbalanced enable/disable) — clamped, flag left as-is',
        );
        return true;
      }());
      return _tail;
    }
    _count--;
    if (_count == 0) {
      // 1→0: release. Swallow a failing release (see [enable]) so the chain
      // stays healthy; the flag is cleared best-effort.
      _tail = _tail.then((_) async {
        try {
          await _inner.disable().timeout(_transitionTimeout, onTimeout: () {});
        } catch (_) {
          // ignore — a failed release must never poison the chain
        } finally {
          _engaged = false;
        }
      });
    }
    return _tail;
  }
}

/// Whether to wire the REAL Android `FLAG_SECURE` block (vs the honest no-op).
/// The one capability gate the composition keys on — kept here so the co-wiring
/// rule and the adapter selection read the same predicate.
///
/// Android-ONLY is correct, not a Platform-gating blind spot (flutter-patterns
/// § Platform Capability Gating): iOS has no screenshot-block equivalent — the
/// instant iOS cover-on-resign-active is manager-carried (the native follow-on),
/// and iOS/desktop already rely on the backup view's auto-hide-on-background.
/// Desktop/web have no block at all.
///
/// DEBUG-DEFERRED (maintainer-authorized): in a DEBUG build the block is deferred so
/// on-device debugging works — `FLAG_SECURE` otherwise blacks out screen-mirroring
/// (scrcpy) and screenshots of the backup screen. `kDebugMode` is compile-time
/// and can NEVER be true in a profile/release build, so EVERY shipped/distributed
/// artifact still protects the seed; only a developer's local debug build defers.
/// When deferred, the composition wires [NoopScreenSecurity], whose honest
/// `isScreenshotBlockSupported == false` drives the "make sure no one can see your
/// screen" copy — the UI never claims a protection that isn't running.
bool defaultSupportsScreenshotBlock() => screenshotBlockActive(
  isAndroidNative: !kIsWeb && Platform.isAndroid,
  isDebug: kDebugMode,
);

/// The pure block-active decision (extracted so the debug-deferral truth table is
/// directly testable without a real platform): the real native block is wired iff
/// the platform can block AND this is not a debug build.
bool screenshotBlockActive({
  required bool isAndroidNative,
  required bool isDebug,
}) => isAndroidNative && !isDebug;

/// The screen-security adapter for a platform: the real Android channel adapter
/// where a block exists, the honest [NoopScreenSecurity] otherwise. Pure so the
/// selection is host-VM testable for both platform answers.
ScreenSecurity screenSecurityFor({required bool supportsNativeBlock}) =>
    supportsNativeBlock
    ? MethodChannelScreenSecurity()
    : const NoopScreenSecurity();

/// The screen-security source for a platform, REFCOUNT-WRAPPED (#344) — the ONE
/// chokepoint that mints a screen-visible [ScreenSecurity]. Every exposure point
/// (the [screenSecurityProvider] default AND the `walletOnboardingOverrides`
/// production override) goes through here, so no path can ship an un-refcounted
/// flag and reintroduce the stacked-twin race. Adapter SELECTION stays in the
/// bare [screenSecurityFor] (the co-wiring predicate and the adapter-selection
/// tests want the raw adapter); this only wraps it.
RefCountedScreenSecurity refCountedScreenSecurityFor({
  required bool supportsNativeBlock,
}) => RefCountedScreenSecurity(
  screenSecurityFor(supportsNativeBlock: supportsNativeBlock),
);

/// The screen-security source. The default is PLATFORM-CORRECT, not a bare
/// no-op: it runs the same [screenSecurityFor] selection the composition root
/// uses, so a RELEASE-ANDROID build gets the real `FLAG_SECURE` channel adapter
/// EVEN IF a host hand-wires [walletProvisionerProvider] and forgets this
/// override (asserts strip in release — a default is the only guard that
/// survives there). Host-VM tests get [NoopScreenSecurity] because
/// `Platform.isAndroid` is false on the test VM; a debug Android build gets
/// the Noop via the debug deferral in [defaultSupportsScreenshotBlock].
/// The composition root still overrides it (co-wired with the real
/// provisioner); tests inject a fake to assert the backup screen requests
/// protection on show and releases it on dispose.
///
/// Wrapped in [RefCountedScreenSecurity] (#344) so stacked secure screens share
/// one refcounted flag — the provider caches this single instance for the
/// container's lifetime, so every screen that reads it shares the same count.
/// That shared-count safety relies on the ONE-scope assumption this app holds:
/// a single top-level [ProviderScope], and NO nested scope that re-overrides
/// this provider around a secure route (a second instance would fragment the
/// count, letting one route's release drop the flag under another). Every mint
/// goes through [refCountedScreenSecurityFor], the single wrap chokepoint.
final screenSecurityProvider = Provider<ScreenSecurity>(
  (ref) => refCountedScreenSecurityFor(
    supportsNativeBlock: defaultSupportsScreenshotBlock(),
  ),
);
