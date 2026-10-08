import Flutter
import UIKit

/// Plain host delegate. The `zec_wallet_ui/backup_exclusion` handler (and the
/// app-switcher privacy cover) that used to live here moved to the
/// `zec_wallet_ui_platform` companion plugin — registered below through the
/// generated plugin registrant, so the example carries ZERO native wallet code.
/// Do NOT re-add a handler on that channel here: a hand-written one registered
/// after the registrant would silently REPLACE the plugin's (last registration
/// wins on a FlutterMethodChannel).
@main
@objc class AppDelegate: FlutterAppDelegate, FlutterImplicitEngineDelegate {
  override func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
  ) -> Bool {
    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }

  func didInitializeImplicitFlutterEngine(_ engineBridge: FlutterImplicitEngineBridge) {
    GeneratedPluginRegistrant.register(with: engineBridge.pluginRegistry)
  }
}
