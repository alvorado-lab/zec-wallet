import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart' show AppLifecycleState;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';

import '../../core/lifecycle/app_lifecycle_provider.dart';
import 'wallet_providers.dart';
import 'wallet_sync_controller.dart';

/// A signal that the DEVICE's network became usable again — the missing half of
/// the sync loop's recovery story (#404, the device finding).
///
/// ## Why this exists
///
/// The `SyncController`'s retry ladder lives in the spawned loop task, so
/// `stopSync` tears it down and `startSync` spawns a fresh one — which is why
/// [WalletSyncController.tryNow] is a stop+start pair, and why a REAL
/// background→foreground cycle already resets the backoff (backgrounding stops
/// the loop). What has no recovery is a FOREGROUND reconnect: airplane mode off,
/// a wifi switch, walking out of a tunnel while looking at the screen. Measured
/// on device: ~6 minutes offline drove the ladder to 1,2,4,…,256s, and after the
/// network was demonstrably back (ping OK in under 5s) the badge still read
/// "Sync paused" for ~80 seconds — up to the full 256s+ after a longer outage.
/// The copy is honest and the sync sheet has a working "Try now", but a user who
/// reconnects and simply LOOKS sees a wallet that appears stuck for minutes.
///
/// ## The shape (Scenario C, the [ScreenSecurity] precedent)
///
/// `zec_wallet_ui` stays plugin-free: this is a PORT plus an `EventChannel`
/// adapter, and the native halves ship in the optional `zec_wallet_ui_platform`
/// companion. A host that adds no plugin registers no handler, the stream is
/// EMPTY, and nothing happens — no tick, and (since #407 R2) nothing reported to
/// `FlutterError` either. See [EventChannelNetworkReachability] for why that
/// second half needed its own work: the obvious adapter shape leaked a
/// `MissingPluginException` per mount at every host without the plugin.
///
/// WHAT "NO-OP" COSTS THE HOST, stated plainly (#407 R10d corrected the earlier
/// claim here). MOBILE hosts without the plugin keep a real fallback: a genuine
/// background→resume restarts the loop, which resets the ladder. DESKTOP has no
/// such fallback at all — `paused` never fires there, so the loop is never
/// stopped and never respawned — and desktop is also where no native half
/// exists. So on desktop the ONLY recovery from a foreground reconnect remains
/// the sync sheet's manual "Try now". That is the pre-#404 status quo, not a
/// regression, but it is not "self-heals" and must not be written as if it were.
///
/// §5.4: a tick carries NO PAYLOAD. Not the SSID, not the interface, not an
/// address — the only information crossing the boundary is "something changed,
/// it may be worth trying again".
abstract interface class NetworkReachability {
  /// Ticks when the platform reports the device has a usable network again.
  ///
  /// Best-effort and ADVISORY: a tick is permission to retry sooner, never a
  /// claim that the Zcash endpoint is reachable. The retry itself re-derives
  /// the truth. Duplicate and spurious ticks are expected (Android reports per
  /// NETWORK, so wifi and cellular each fire, and a flapping link fires
  /// repeatedly) — [WalletReconnectKick] coalesces them.
  Stream<void> get onAvailable;
}

/// The production adapter: an `EventChannel` the companion plugin answers.
///
/// A missing handler is the DEFAULT case, not an error: every desktop target,
/// the web, and any host that chose not to add the plugin land here. So the
/// stream degrades to empty rather than surfacing — same honesty rule as
/// `MethodChannelScreenSecurity._invoke` mapping a `MissingPluginException` to
/// an honest `false`.
///
/// ## Why this hand-rolls what `receiveBroadcastStream` does (#407 R2)
///
/// **`EventChannel` cannot report an activation failure through its stream.**
/// `receiveBroadcastStream`'s `onListen` awaits `invokeMethod('listen')` and, on
/// failure, calls `FlutterError.reportError` — the stream itself never sees it,
/// so `.handleError()` provably cannot catch it. That was measured, not
/// reasoned: the pre-#407 adapter emitted
/// `MissingPluginException(No implementation found for method listen on channel
/// zec_wallet_ui/network_reachability)` through `FlutterError.onError` on every
/// wallet-screen mount at every host without the companion plugin — i.e. every
/// desktop, web, and non-adopting mobile host. Where a host wires
/// `FlutterError.onError` into Crashlytics/Sentry that is a NON-FATAL REPORT
/// NAMING OUR CHANNEL, sent to a third party once per wallet-screen open. From a
/// privacy wallet that is not an acceptable default, and it flatly contradicted
/// this class's own "genuine no-op" claim.
///
/// The `MethodChannelScreenSecurity` precedent does NOT transfer, which is the
/// trap: a `MethodChannel` returns the `MissingPluginException` to its CALLER,
/// so catching it there is enough. An `EventChannel` does not.
///
/// So: same wire protocol (`listen`/`cancel` over the same name + codec, which
/// is all `receiveBroadcastStream` is), with the activation call awaited HERE
/// where a `MissingPluginException` is an expected answer rather than a fault to
/// report. Nothing about the native contract changes.
class EventChannelNetworkReachability implements NetworkReachability {
  const EventChannelNetworkReachability({EventChannel? channel})
    : _channel = channel ?? const EventChannel(channelName);

  /// Namespaced to THIS package — the contract belongs to the reusable wallet
  /// UI, not to any one app (matches `zec_wallet_ui/screen_security`).
  static const String channelName = 'zec_wallet_ui/network_reachability';

  final EventChannel _channel;

  @override
  Stream<void> get onAvailable {
    final messenger = _channel.binaryMessenger;
    // The METHOD side of the very same channel — `listen`/`cancel` is the whole
    // EventChannel protocol, and reusing the channel's own codec keeps this an
    // adapter rather than a reimplementation.
    final control = MethodChannel(_channel.name, _channel.codec, messenger);
    late final StreamController<void> controller;
    controller = StreamController<void>.broadcast(
      onListen: () async {
        messenger.setMessageHandler(_channel.name, (ByteData? reply) async {
          if (reply == null) {
            // The platform's end-of-stream.
            await controller.close();
          } else {
            try {
              _channel.codec.decodeEnvelope(reply);
              // §5.4: whatever the native half sent is DISCARDED. A tick is a
              // tick; no payload can reach wallet state even from a rogue
              // implementation of this channel.
              controller.add(null);
            } catch (_) {
              // A platform ERROR event. Advisory signal — drop it; the ladder
              // still runs, which is the pre-#404 behaviour.
            }
          }
          return null;
        });
        try {
          await control.invokeMethod<void>('listen');
        } on MissingPluginException {
          // THE DEFAULT CASE, not a fault: no companion plugin. Silence, and
          // nothing reported anywhere.
        } catch (_) {
          // Any other activation fault degrades the same way. This stream can
          // only ever make the wallet retry SOONER, so a platform-side problem
          // must never surface on a money surface for an advisory signal.
        }
      },
      onCancel: () async {
        messenger.setMessageHandler(_channel.name, null);
        try {
          await control.invokeMethod<void>('cancel');
        } on MissingPluginException {
          // Nothing was ever listening natively.
        } catch (_) {
          // Teardown is best-effort by the same argument as activation.
        }
      },
    );
    return controller.stream;
  }
}

/// The reachability source. Defaults to the channel adapter; a host or a test
/// overrides it (the `walletSessionProvider` null-seam idiom).
final walletNetworkReachabilityProvider = Provider<NetworkReachability>(
  (ref) => const EventChannelNetworkReachability(),
);

/// The FIRST cooldown after acting on a tick — DOUBLED per consecutive kick that
/// fails to clear the stall, up to [walletReconnectKickCooldownMax].
///
/// Android's `onAvailable` fires PER NETWORK, so a phone that regains wifi while
/// cellular is up ticks twice within milliseconds, and a flapping link ticks
/// repeatedly. `_inFlight` inside the controller serializes the resulting
/// commands but does not COALESCE them, so without this a flap would queue a
/// stop+start pair per tick. 10s is long enough to swallow a burst and far
/// shorter than the ladder this exists to shortcut.
const Duration walletReconnectKickCooldown = Duration(seconds: 10);

/// Ceiling for the escalated cooldown — the sync loop's OWN backoff cap.
///
/// #407 R3: a FIXED 10s period with no escalation and no total cap defeats the
/// very ladder this kick shortcuts. Every kick is a `stopSync`+`startSync`, and
/// the Rust `run_loop` resets `backoff` to its 1 s initial value on every fresh
/// start — so on a flapping link the wallet never climbs the ladder at all.
/// MEASURED on a 3 s flap with the endpoint unreachable: over 600 s, 50 kicks /
/// 50 `stopSync` / 51 `startSync` — one full teardown+respawn every 12 s,
/// indefinitely, against the ONE attempt per 600 s the ladder is designed to
/// settle at. That is ~200x the connection attempts at the user's lightwalletd,
/// plus continuous radio and CPU in exactly the low-signal condition backoff
/// exists for.
///
/// NOT ADVERSARIAL — ORDINARY. The Android request deliberately omits
/// `NET_CAPABILITY_VALIDATED` (validation must not suppress the tick on the
/// flaky links this exists for), so a captive portal re-fires `onAvailable` each
/// cycle; so do lifts, trains, and the edge of coverage.
///
/// Escalating to the ladder's own cap bounds the worst case at 10, 20, 40, 80,
/// 160, 320, 600, 600 s, and the fast recovery survives intact for the case that
/// matters — a REAL reconnect clears the stall, which resets the escalation.
///
/// It does NOT reduce the cost to parity with sync alone, and an earlier draft
/// of this doc claimed it did (review MINOR, ~10x out). Each kick is a
/// stop+start and every fresh `run_loop` restarts the Rust ladder at its 1 s
/// initial value, so even at the 600 s cap the loop dials at t = 0, 1, 3, 7, 15,
/// 31, 63, 127, 255, 511 — about TEN dials per window against the one a settled
/// ladder would make. The cap bounds the amplification; it does not remove it.
/// `SyncStatusNotifier._subDelivered` solved this same problem next door; this
/// is that pattern.
const Duration walletReconnectKickCooldownMax = Duration(seconds: 600);

/// Turns a network-available tick into a sync-ladder reset — but ONLY on a live
/// stall (#404).
///
/// ## Why the gate is here and not in the controller
///
/// [WalletSyncController] is deliberately ORTHOGONAL to the status stream: it
/// CONTROLS the loop, `SyncStatusNotifier` OBSERVES it, and its class doc states
/// they never call each other. Reading the status inside the controller to
/// decide whether to kick would break that. So this is a separate consumer that
/// reads the status and calls `tryNow()` — structurally the same thing the sync
/// sheet's "Try now" button already does, which is the precedent.
///
/// ## The gate, and why each leg is load-bearing
///
///  * **Stalled only.** `tryNow()` gates on the DRIVE being `running`; it does
///    NOT know the status. A tick while the wallet is scanning happily would
///    restart a healthy loop for nothing. The sheet's button carries the same
///    `Stalled && driving` gate — this is that gate, applied automatically.
///  * **The RAW status, not the display one.** Logic listeners stay on the raw
///    provider by house rule (#399 item 3): the posture dwell is presentation
///    policy, never a truth filter, and a dwelled `Stalled` could hide a stall
///    that is genuinely current.
///  * **Everything else is `tryNow()`'s own belt** and is deliberately NOT
///    re-implemented here: the `running` gate, the host policy check, and the
///    `_wasPaused` re-check inside the serialized link that forecloses a
///    background restart at execution time (review M1). A backgrounded
///    device that regains network must not start scanning.
///
/// Money-safety is unchanged: stop+start loses nothing, because durable scan
/// progress is kept (the chain is the source of truth, SDK §6.3). That is
/// exactly why `tryNow()` is allowed to do it in the first place.
class WalletReconnectKick extends Notifier<int> {
  StreamSubscription<void>? _sub;
  Timer? _cooldown;

  /// Consecutive kicks that did NOT clear the stall — the escalation exponent
  /// (#407 R3). Reset the moment the raw status leaves `Stalled`, because that
  /// is the only evidence a kick actually WORKED.
  int _unresolvedKicks = 0;

  /// A reachability tick that arrived while the cooldown was closed (#409 R1).
  /// Replayed once when the window opens; see [_onAvailable].
  bool _pendingTick = false;

  /// The lifecycle arrives as `paused → hidden → inactive → resumed`, so at the
  /// `resumed` event the previous state is `inactive`, NOT `paused` — only a
  /// remembered flag tells a real resume from focus/shade noise. Same reasoning,
  /// and same name, as `WalletSyncController._wasPaused`.
  bool _wasPaused = false;

  /// The number of ticks this has ACTED on. Exposed as the notifier's state so
  /// a test can assert coalescing without reaching into privates; nothing
  /// renders it.
  @override
  int build() {
    // Identity-keyed like the section single-flight: a wallet swap must not
    // inherit the previous life's cooldown.
    ref.watch(walletIdentityProvider);
    // Riverpod REUSES this notifier across a dependency-change rebuild, so both
    // fields must be reset here or a prior life's state is sticky (#330 class).
    _unresolvedKicks = 0;
    _pendingTick = false;
    _wasPaused = false;
    // THE ESCALATION RESET (#407 R3, corrected by #409 R1). A kick that cleared
    // the stall has earned the fast 10s window back; one that did not must keep
    // backing off.
    //
    // ONLY A LIVE, PROGRESSING LOOP COUNTS AS EVIDENCE — see
    // [_provesTheKickWorked]. The first version of this listener accepted any
    // concrete non-`Stalled` value, which made the escalation inert: every kick
    // is a `stopSync`, `SyncController::stop()` ends by publishing `Idle`
    // (`sync_controller.rs:328`), so each kick's own teardown reset its exponent
    // AND cancelled its live cooldown. Measured on device: 16 wifi cycles gave
    // 16 kicks, one per tick, zero coalescing, with `backoff_secs` pinned at 8
    // where the same phone had reached 256 before the kick existed.
    //
    // The old comment justified the `null` guard by claiming a kick's stop+start
    // puts this provider through a value-less loading state. That is FALSE and
    // was never the hazard: `SyncStatusNotifier` returns `loading` only from
    // `build()` and thereafter assigns `AsyncValue.data` exclusively, so it
    // cannot re-enter loading. The value that actually broke this was a concrete
    // `Idle`. The `null` check stays only as a before-first-emit guard.
    ref.listen<AsyncValue<SyncStatus>>(syncStatusProvider, (_, next) {
      if (!_provesTheKickWorked(next.value)) return;
      _unresolvedKicks = 0;
      // Drop a long escalated window too: the NEXT outage is a fresh event and
      // deserves the fast first response, not the tail of the previous one.
      _cooldown?.cancel();
      _cooldown = null;
    });
    // QUIESCE WHILE BACKGROUNDED (#407 R9). The kick provably cannot act there —
    // `tryNow()` returns false under a `suspended` drive, and the `_wasPaused`
    // re-check inside its serialized link forecloses it again at execution time
    // — so a live `NetworkCallback` in the background is pure cost with no
    // reachable benefit. Every sibling here is explicitly quiescent when
    // backgrounded (`SyncStatusNotifier._pause`/`_resume` is the pattern).
    //
    // Cancelling the SUBSCRIPTION is what makes this real rather than cosmetic:
    // it drives the EventChannel's `cancel`, which reaches
    // `unregisterNetworkCallback` on Android and `NWPathMonitor.cancel()` on
    // iOS, so the cost goes away at the OS level and not merely in Dart.
    //
    // It also folds with #407 R3: a fresh subscription on resume is a natural
    // escalation reset, and a real background/resume ALREADY restarts the sync
    // loop (which resets the ladder), so there is nothing left for a kick to
    // shortcut at that moment anyway.
    ref.listen<AppLifecycleState>(appLifecycleProvider, (_, next) {
      switch (next) {
        case AppLifecycleState.paused:
          _wasPaused = true;
          _sub?.cancel();
          _sub = null;
          _cooldown?.cancel();
          _cooldown = null;
          _unresolvedKicks = 0;
          // A tick remembered while foregrounded must not fire on the way back:
          // a real resume restarts the loop anyway, which resets the ladder.
          _pendingTick = false;
        case AppLifecycleState.resumed:
          // Only a REAL paused→resumed re-listens: `inactive/hidden → resumed`
          // is focus/shade noise (the flutter-patterns Stream-Lifecycle rule the
          // sync controller follows), and re-subscribing on it would re-register
          // the native callback on every notification-shade pull.
          if (!_wasPaused) return;
          _wasPaused = false;
          _sub ??= ref
              .read(walletNetworkReachabilityProvider)
              .onAvailable
              .listen((_) => _onAvailable());
        // `hidden` is desktop's deepest state and Android's pre-`paused` step —
        // never act on it, or desktop would lose the signal while merely
        // minimized (where, unlike mobile, the loop keeps running and a kick is
        // still the only automatic recovery).
        case AppLifecycleState.inactive:
        case AppLifecycleState.hidden:
        case AppLifecycleState.detached:
          break;
      }
    });
    final reachability = ref.watch(walletNetworkReachabilityProvider);
    // Born-backgrounded (the Android push-trampoline cold start): do not
    // register a native callback we could not act on.
    if (ref.read(appLifecycleProvider) == AppLifecycleState.paused) {
      _wasPaused = true;
    } else {
      _sub = reachability.onAvailable.listen((_) => _onAvailable());
    }
    ref.onDispose(() {
      _sub?.cancel();
      _sub = null;
      _cooldown?.cancel();
      _cooldown = null;
    });
    return 0;
  }

  void _onAvailable() {
    if (_cooldown != null) {
      // #409 R1 (reliability review MAJOR): a tick inside the window is either
      // BURST NOISE or NEW INFORMATION, and the two need opposite treatment.
      //
      // Android fires `onAvailable` once per NETWORK, so wifi and cellular
      // coming back together produce two or three ticks milliseconds apart —
      // one event, reported repeatedly. Those must be swallowed, which is what
      // the base window is for.
      //
      // A tick arriving LATER than that is a different network transition: the
      // user genuinely left the tunnel. Nothing will re-send it once the link
      // is stable, so dropping it strands recovery on the full Rust ladder —
      // the exact wait this kick exists to shortcut. Dropping was nearly free
      // while the window was effectively pinned at the base 10 s (which is what
      // the broken reset predicate made it); fixing the escalation is what
      // makes 20/40/…/600 s reachable, and so what grows this hole ~60x.
      // Remember ONLY while the window is escalated past its base length.
      // At base length the window IS the burst coalescer: Android reports
      // `onAvailable` per network, so wifi and cellular returning together fire
      // two or three ticks milliseconds apart — one event, reported repeatedly,
      // and remembering those would turn every burst into a second kick. Past
      // base length the window is the ESCALATION, and that is the hole this
      // closes: fixing the escalation is exactly what grew the discard window
      // from 10 s to as much as 600 s. The residual base-length hole is the
      // pre-#409 behaviour, left as it was.
      if (_unresolvedKicks > 1) _pendingTick = true;
      return;
    }
    // The RAW status (see the class doc). `.value` is null before the first
    // emit, which correctly reads as "no live stall to shortcut".
    final status = ref.read(syncStatusProvider).value;
    if (status is! SyncStatus_Stalled) return;
    // Everything the drive owns — running, policy, not-backgrounded — is
    // re-checked inside `tryNow()`; `false` means it declined and no restart
    // was enqueued, so this must not start a cooldown over a no-op.
    if (!ref.read(walletSyncControllerProvider.notifier).tryNow()) return;
    state = state + 1;
    _unresolvedKicks = _unresolvedKicks + 1;
    _cooldown = Timer(_cooldownWindow(), () {
      _cooldown = null;
      // Re-enter ONCE for a tick that arrived while the window was closed. The
      // rate limit is preserved exactly (one kick per window, and the re-entry
      // re-checks every gate — stall, policy, drive, foreground); what changes
      // is that the signal is deferred rather than destroyed.
      if (_pendingTick) {
        _pendingTick = false;
        _onAvailable();
      }
    });
  }

  /// Does this status PROVE the last kick worked (#409 R1)?
  ///
  /// An ALLOWLIST, deliberately, and not the denylist this replaced: the safe
  /// default for a state we have not reasoned about is "no evidence, keep
  /// backing off", never "recovered".
  ///
  /// Written as an EXHAUSTIVE SWITCH over the sealed type rather than a chain of
  /// `is` tests, which is the same argument one step further (review MINOR): a
  /// type-test chain lets a future variant fall silently into the default,
  /// whereas the switch makes it a COMPILE ERROR and forces this decision to be
  /// retaken. Prose is not enforcement.
  ///
  /// Evidence:
  ///  * **`Scanning`** — the loop is alive and consuming blocks. Reports come
  ///    from scan samples downstream of the client connect and tip fetch, so it
  ///    cannot be produced from local cache.
  ///  * **`UpToDate`** — it got there (a clean pass publishes this directly, so
  ///    a no-op recovery is still covered).
  ///
  /// Not evidence:
  ///  * **`Idle` is what a STOP publishes** (`sync_controller.rs:328`), and
  ///    every kick is a stop+start — so accepting `Idle` lets every kick certify
  ///    itself. That is the whole of the #409 R1 defect.
  ///  * `Stalled` is the condition being escalated against.
  ///  * **`Connecting`, `Offline` and `Unknown` HAVE NO PRODUCER in the core
  ///    today** — verified: the only `SyncStatus::Connecting` occurrences are
  ///    readers/mappings in `tor_status.rs`, `wallet.rs` says "*when* `Offline`
  ///    gains an emitter", and `wallet_sync_controller.dart`'s class doc records
  ///    the same for `Connecting`. They are excluded DEFENSIVELY, for whenever
  ///    an emitter lands — not because they are observed. An earlier draft of
  ///    this doc asserted "`Connecting` is what a fresh START publishes"; that
  ///    was false (`SyncController::start` publishes nothing at all) and a
  ///    sibling file already said so. Recorded because the wrong model is what
  ///    let the original defect survive review.
  static bool _provesTheKickWorked(SyncStatus? status) => switch (status) {
    // `UpToDateLimited` proves the kick worked exactly as `UpToDate` does: the
    // question here is "did connectivity come back", and a limited-but-complete
    // scan answers it. Whether this BUILD can read every block is a different
    // axis entirely (`ironwood-nu63-support.md` §6.4) and must not make the
    // reconnect logic retry forever against a healthy link.
    // `UpToDateDegraded` likewise (T0-1b): the pass reached the tip over a live
    // link; that the SERVER under-serves a pool is the same other axis.
    // `EndpointBehind` likewise (T0-1c): the pass completed over a live link;
    // that the SERVER's tip is behind the network is that axis again.
    // `UpToDateUnverified` likewise (GRACE-1): the pass completed; that the
    // SERVER will not say which network it is on is that axis once more.
    SyncStatus_Scanning() ||
    SyncStatus_UpToDate() ||
    SyncStatus_UpToDateLimited() ||
    SyncStatus_UpToDateDegraded() ||
    SyncStatus_EndpointBehind() ||
    SyncStatus_UpToDateUnverified() => true,
    SyncStatus_Idle() ||
    SyncStatus_Stalled() ||
    SyncStatus_Connecting() ||
    SyncStatus_Offline() ||
    SyncStatus_Unknown() => false,
    null => false,
  };

  /// `base << (unresolved - 1)`, capped at [walletReconnectKickCooldownMax].
  ///
  /// The shift is clamped before it is applied — an unbounded exponent on a
  /// device that flaps for hours would overflow into a nonsense (possibly
  /// negative) duration, which is a worse failure than the one being fixed.
  Duration _cooldownWindow() {
    final shift = (_unresolvedKicks - 1).clamp(0, 16);
    final seconds = walletReconnectKickCooldown.inSeconds << shift;
    final capped = walletReconnectKickCooldownMax.inSeconds;
    return Duration(seconds: seconds < capped ? seconds : capped);
  }
}

/// Built by the wallet surface so the subscription exists for the container's
/// life (the [walletSyncControllerProvider] idiom). Nothing renders its value.
final walletReconnectKickProvider = NotifierProvider<WalletReconnectKick, int>(
  WalletReconnectKick.new,
);
