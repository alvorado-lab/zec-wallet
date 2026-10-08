import Flutter
import Network
import UIKit

/// Default native iOS behaviours for the plugin-free `zec_wallet_ui` package
/// (the "Scenario C" companion). Adding this package to a host app wires, with
/// **zero host native code**:
///
///  1. **Backup exclusion** — a handler for `zec_wallet_ui/backup_exclusion`
///     that sets `NSURLIsExcludedFromBackupKey` on the wallet DB path the Dart
///     side passes. Without it, a device-restore brings back a DB whose
///     ThisDeviceOnly Secure-Enclave key can't be reconstructed → the
///     "couldn't finish setup" (needsRecovery) trap. (Parity with the reference
///     `AppDelegate` handler.)
///  2. **App-switcher privacy cover** — an opaque cover added over the
///     foreground window **before** iOS captures the app-switcher snapshot, so
///     no on-screen content (seed phrase, balance, addresses, tx detail —
///     app-frame A8) lands in the persisted recents thumbnail. Covers are kept
///     **per scene** so each window scene (iPad multi-window) protects its own
///     Flutter content. This folds the #342 gap into the SDK.
///
/// **Honesty:** iOS has no `FLAG_SECURE`, so the cover protects ONLY the
/// snapshot — a live foreground screenshot is still possible. This plugin
/// therefore registers **no** iOS `screen_security` handler; `zec_wallet_ui`
/// uses its no-op adapter on iOS and honestly says "make sure no one can see
/// your screen" (never "screenshots are off"). Do not add an `enableSecure`
/// success handler here — it would flip that copy into a lie.
///
/// Minimum deployment target is iOS 13.0, so the UIScene APIs are always
/// available (no `#available` gating needed).
public class ZecWalletUiPlatformPlugin: NSObject, FlutterPlugin {
  /// Retains the singleton so its NotificationCenter observers outlive
  /// `register(with:)` (selector-based observers are not retained by the center).
  private static var shared: ZecWalletUiPlatformPlugin?

  /// Cover currently shown over each scene's app-switcher snapshot, keyed by the
  /// scene, so a multi-window (iPad) layout protects each scene independently and
  /// a double-fire is idempotent per scene.
  private var covers: [ObjectIdentifier: UIView] = [:]
  private var observing = false

  /// The live `NWPathMonitor` handlers, keyed by the ENGINE that registered
  /// them (#404, corrected by #407 R4).
  ///
  /// WHY PER-ENGINE AND NOT ONE SHARED FIELD. The first cut kept a single
  /// optional and early-returned when it was non-nil. Two bugs fell out of that,
  /// both of which the Kotlin half already names and handles in
  /// `onDetachedFromEngine`:
  ///
  ///  * **The monitor outlived its engine.** `onCancel` fires only when DART
  ///    sends `cancel`; a destroyed `FlutterEngine` never does. So the
  ///    `NWPathMonitor` kept running for the process's life and the handler kept
  ///    a `FlutterEventSink` pointing at a dead messenger.
  ///  * **A SECOND engine got no channel at all.** `shared` survives engine
  ///    destruction, so the `guard reachability == nil` early-return meant an
  ///    add-to-app host (or an engine group) registered the EventChannel exactly
  ///    once, and every later engine silently never ticked — the failure mode
  ///    that looks identical to the bug #404 exists to fix.
  private var reachability: [ObjectIdentifier: NetworkReachabilityStreamHandler] = [:]

  public static func register(with registrar: FlutterPluginRegistrar) {
    let instance = shared ?? ZecWalletUiPlatformPlugin()
    shared = instance
    instance.registerBackupExclusion(with: registrar)
    instance.registerNetworkReachability(with: registrar)
    instance.startPrivacyCoverObserversIfNeeded()
    // Ask Flutter to call `detachFromEngine(for:)` when THIS engine goes away,
    // so the monitor above is torn down at the OS level rather than leaking.
    registrar.publish(instance)
  }

  /// Engine teardown (#407 R4) — the iOS mirror of the Kotlin
  /// `onDetachedFromEngine`. Cancels this engine's `NWPathMonitor` and drops its
  /// sink, so nothing survives the engine that owned it.
  public static func detachFromEngine(for registrar: FlutterPluginRegistrar) {
    shared?.tearDownNetworkReachability(for: registrar)
  }

  // MARK: - Network reachability (#404)

  /// Ticks `zec_wallet_ui/network_reachability` when the device regains a
  /// usable path.
  ///
  /// WHY: `zec_wallet_ui`'s sync retry ladder lives in its spawned loop task, so
  /// a real background/resume already resets it. The gap is a FOREGROUND
  /// reconnect — airplane mode off, a wifi switch, walking out of a tunnel while
  /// looking at the screen — where no lifecycle event fires and the badge can sit
  /// on "Sync paused" for minutes after the network is back.
  ///
  /// ADVISORY: `.satisfied` means iOS has a usable path, NOT that the wallet's
  /// lightwalletd endpoint answers. The Dart side treats a tick as permission to
  /// retry sooner and re-derives the truth from the retry itself.
  ///
  /// §5.4: the event payload is `nil` — no interface name, no address. A tick is
  /// a tick.
  private func registerNetworkReachability(with registrar: FlutterPluginRegistrar) {
    let key = ObjectIdentifier(registrar.messenger())
    // Per ENGINE, not once per process (#407 R4): re-registering for the same
    // messenger is the idempotent case, a NEW messenger is a new engine that
    // needs its own channel.
    guard reachability[key] == nil else { return }
    let handler = NetworkReachabilityStreamHandler()
    let channel = FlutterEventChannel(
      name: "zec_wallet_ui/network_reachability",
      binaryMessenger: registrar.messenger())
    channel.setStreamHandler(handler)
    reachability[key] = handler
  }

  /// Cancel + drop this engine's monitor. Idempotent — a detach for an engine we
  /// never registered (or already tore down) is a no-op, so an extra call from
  /// Flutter cannot fault. Mirrors the Kotlin `stop()` contract.
  private func tearDownNetworkReachability(for registrar: FlutterPluginRegistrar) {
    let key = ObjectIdentifier(registrar.messenger())
    reachability[key]?.stop()
    reachability[key] = nil
  }

  // MARK: - Backup exclusion

  private func registerBackupExclusion(with registrar: FlutterPluginRegistrar) {
    let channel = FlutterMethodChannel(
      name: "zec_wallet_ui/backup_exclusion",
      binaryMessenger: registrar.messenger())
    channel.setMethodCallHandler { call, result in
      guard call.method == "excludeFromBackup",
            let path = call.arguments as? String else {
        result(FlutterMethodNotImplemented)
        return
      }
      var url = URL(fileURLWithPath: path)
      do {
        var values = URLResourceValues()
        values.isExcludedFromBackup = true
        try url.setResourceValues(values)
        result(true)
      } catch {
        result(FlutterError(
          code: "exclude_failed",
          message: error.localizedDescription,
          details: nil))
      }
    }
  }

  // MARK: - App-switcher privacy cover (#342)

  /// Observe BOTH the scene lifecycle (modern, scene-manifest apps) and the
  /// classic app lifecycle (no scene manifest). Whichever the host is on, the
  /// matching pair fires; the per-scene `covers` map makes any double-fire a
  /// no-op. Cover on resign/deactivate (before the snapshot), remove on activate
  /// (every return path, including a transient interruption). A disconnecting
  /// scene drops its cover entry so the map can't retain a dead window's view.
  private func startPrivacyCoverObserversIfNeeded() {
    guard !observing else { return }
    observing = true
    let center = NotificationCenter.default
    center.addObserver(
      self, selector: #selector(sceneCoverUp(_:)),
      name: UIScene.willDeactivateNotification, object: nil)
    center.addObserver(
      self, selector: #selector(sceneCoverDown(_:)),
      name: UIScene.didActivateNotification, object: nil)
    center.addObserver(
      self, selector: #selector(sceneDisconnected(_:)),
      name: UIScene.didDisconnectNotification, object: nil)
    // Classic (no-scene) lifecycle, and a belt for hosts not on scenes.
    center.addObserver(
      self, selector: #selector(appCoverUp),
      name: UIApplication.willResignActiveNotification, object: nil)
    center.addObserver(
      self, selector: #selector(appCoverDown),
      name: UIApplication.didBecomeActiveNotification, object: nil)
  }

  // Scene lifecycle — cover the SPECIFIC scene that is deactivating.
  @objc private func sceneCoverUp(_ note: Notification) {
    guard let scene = note.object as? UIWindowScene else { return }
    coverUp(scene)
  }

  @objc private func sceneCoverDown(_ note: Notification) {
    guard let scene = note.object as? UIWindowScene else { return }
    coverDown(scene)
  }

  @objc private func sceneDisconnected(_ note: Notification) {
    guard let scene = note.object as? UIWindowScene else { return }
    covers[ObjectIdentifier(scene)] = nil
  }

  // Classic app lifecycle — no per-scene object, so cover/uncover every
  // connected window scene (idempotent via the per-scene map).
  @objc private func appCoverUp() {
    for scene in windowScenes() { coverUp(scene) }
  }

  @objc private func appCoverDown() {
    for scene in windowScenes() { coverDown(scene) }
  }

  private func coverUp(_ scene: UIWindowScene) {
    let key = ObjectIdentifier(scene)
    guard covers[key] == nil, let window = flutterWindow(in: scene) else { return }

    // OPAQUE base is the real guarantee — a high-contrast seed grid must not
    // bleed through the (disk-persisted) thumbnail, so a translucent blur ALONE
    // is not enough. Solid `systemBackground`, with a blur on top only so it
    // looks intentional and adapts to light/dark.
    let cover = UIView(frame: window.bounds)
    cover.backgroundColor = .systemBackground
    cover.autoresizingMask = [.flexibleWidth, .flexibleHeight]
    cover.isUserInteractionEnabled = false
    // Hide what's behind the cover from VoiceOver too: during an on-screen
    // transient interruption (Control Center / banner) with the seed revealed, a
    // VoiceOver user must not be able to swipe to the seed words behind the frost.
    // The empty cover itself is harmless to focus.
    cover.accessibilityViewIsModal = true

    let blur = UIVisualEffectView(effect: UIBlurEffect(style: .systemMaterial))
    blur.frame = cover.bounds
    blur.autoresizingMask = [.flexibleWidth, .flexibleHeight]
    cover.addSubview(blur)

    window.addSubview(cover)
    covers[key] = cover
  }

  private func coverDown(_ scene: UIWindowScene) {
    let key = ObjectIdentifier(scene)
    covers[key]?.removeFromSuperview()
    covers[key] = nil
  }

  private func windowScenes() -> [UIWindowScene] {
    UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
  }

  /// The window HOSTING a `FlutterViewController` within THIS scene (checking the
  /// rootVC and its presented / child hierarchy, so an add-to-app host that
  /// embeds Flutter as a child is still covered) — not merely whatever is key. A
  /// host that presents a secondary `UIWindow` could have that window key at
  /// resign; the hierarchy walk finds the Flutter window regardless.
  ///
  /// Returns nil (→ no cover) when NO window in the scene hosts Flutter. That is
  /// deliberate: in an add-to-app host, once the Flutter VCs are torn down and
  /// the user is back on native screens, there is no wallet content to protect,
  /// so we must NOT blur the host's own native windows. (A blind key/first-window
  /// fallback here would cover unrelated host screens on every background.)
  private func flutterWindow(in scene: UIWindowScene) -> UIWindow? {
    scene.windows.first { hostsFlutter($0.rootViewController) }
  }

  private func hostsFlutter(_ vc: UIViewController?) -> Bool {
    guard let vc = vc else { return false }
    if vc is FlutterViewController { return true }
    if hostsFlutter(vc.presentedViewController) { return true }
    return vc.children.contains { hostsFlutter($0) }
  }
}

/// The `NWPathMonitor` half of #404 — see `registerNetworkReachability`.
///
/// EDGE-TRIGGERED, deliberately. `NWPathMonitor` replays the CURRENT path
/// immediately on start and then reports every change, so a level-triggered
/// forward would tick on the first callback of a perfectly healthy launch and on
/// every unrelated path change (wifi → wifi+VPN, an interface reorder) while
/// already satisfied. Only an unsatisfied→satisfied EDGE is news, which is
/// exactly the reconnect this exists for. The Dart side's own cooldown is the
/// second belt, not the first.
private class NetworkReachabilityStreamHandler: NSObject, FlutterStreamHandler {
  private var monitor: NWPathMonitor?
  private var sink: FlutterEventSink?
  /// Last observed satisfaction. Starts `true` so the monitor's immediate replay
  /// of an already-satisfied path is NOT an edge — a launch is not a reconnect.
  private var satisfied = true
  private let queue = DispatchQueue(label: "dev.zecwallet.ui_platform.reachability")

  func onListen(
    withArguments _: Any?,
    eventSink events: @escaping FlutterEventSink
  ) -> FlutterError? {
    sink = events
    let monitor = NWPathMonitor()
    monitor.pathUpdateHandler = { [weak self] path in
      guard let self = self else { return }
      let nowSatisfied = path.status == .satisfied
      let wasSatisfied = self.satisfied
      self.satisfied = nowSatisfied
      guard nowSatisfied, !wasSatisfied else { return }
      // An EventSink must be used from the platform main thread; the monitor
      // calls back on its own queue.
      DispatchQueue.main.async { self.sink?(nil) }
    }
    monitor.start(queue: queue)
    self.monitor = monitor
    return nil
  }

  func onCancel(withArguments _: Any?) -> FlutterError? {
    stop()
    return nil
  }

  /// Idempotent teardown, shared by `onCancel` and by engine detach (#407 R4).
  ///
  /// `onCancel` alone is NOT sufficient: it fires only when Dart sends `cancel`,
  /// and a destroyed `FlutterEngine` never does — so without the detach path the
  /// `NWPathMonitor` ran for the process's life holding a sink on a dead
  /// messenger. The Kotlin half documents exactly this hazard on its own
  /// `stop()`; this is the mirror it was missing.
  func stop() {
    monitor?.cancel()
    monitor = nil
    sink = nil
    // Re-arm the edge detector: a fresh listen must not treat the first
    // already-satisfied path replay as a reconnect.
    satisfied = true
  }
}
