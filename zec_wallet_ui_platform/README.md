# zec_wallet_ui_platform

Keep a user's recovery phrase, balance and addresses out of screenshots and
the app switcher, with no native code in your app. Add this package next to
[`zec_wallet_ui`](https://pub.dev/packages/zec_wallet_ui) and the wallet
screens are protected on Android and iOS. There is no Dart API to call.

It is optional. `zec_wallet_ui` carries no native screen-protection code of
its own: without this package it still works, and tells the user plainly
that the screen is not protected so they can make sure no one sees it.

Part of the `zec_wallet` family: [`zec_wallet`](https://pub.dev/packages/zec_wallet)
(the SDK), [`zec_wallet_ui`](https://pub.dev/packages/zec_wallet_ui) (an
optional drop-in wallet UI), this package, and
[`zec_wallet_tor`](https://pub.dev/packages/zec_wallet_tor) (optional Tor).

## Should I use it?

- **Yes**, if you want the wallet's screenshot and snapshot protection to work
  out of the box without copying native snippets into your app.
- **No**, if you already provide your own native handlers for the
  `zec_wallet_ui` channels, or you are building a bespoke integration.
  `zec_wallet_ui` works without this package; where no native handler is
  wired it honestly reports "unprotected, hide your screen".

## What it provides

| Platform | Behaviour |
|---|---|
| **Android** | A `FLAG_SECURE` handler for `zec_wallet_ui/screen_security` (`enableSecure` / `disableSecure`). Blocks live screenshots **and** the recents-thumbnail snapshot while the recovery-phrase, send or swap screens show a secret. Applied to the host Activity via `ActivityAware`. |
| **iOS** | (1) A `zec_wallet_ui/backup_exclusion` handler (`NSURLIsExcludedFromBackupKey` on the wallet DB). (2) An **app-wide app-switcher privacy cover**: an opaque cover added over the foreground window *before* iOS captures the app-switcher snapshot, so the seed, balance, addresses and transaction details never land in the recents thumbnail. |
| **Android and iOS** | The `zec_wallet_ui/network_reachability` event channel: a payload-free tick when the device regains a usable network, so the wallet's sync retries at once instead of waiting out its backoff. Android uses a `ConnectivityManager.NetworkCallback`, so **an Android host that adds this plugin inherits the normal-level `ACCESS_NETWORK_STATE` permission** (granted at install, no prompt; Play lists it as "view network connections"). To drop it, add `<uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" tools:node="remove"/>` to your app's manifest AND override `walletNetworkReachabilityProvider` in `zec_wallet_ui` with your own connectivity source (so the plugin's channel is never listened to); see the plugin's `AndroidManifest.xml`. iOS uses `NWPathMonitor`. |
| **Desktop** | Nothing: `zec_wallet_ui`'s built-in no-op adapter already reports honestly. Desktop capture-blocking (macOS `NSWindow.sharingType`, Windows `SetWindowDisplayAffinity`) is a separate item, not covered here. |

## How to adopt

Add the dependency; that's it. The native handlers register themselves
through standard Flutter plugin registration, so **there is no Dart API to
call.**

```yaml
dependencies:
  zec_wallet_ui: ^0.0.1
  zec_wallet_ui_platform: ^0.0.1
```

Run `flutter pub get` and rebuild the app. You can now delete any hand-written
`MainActivity` FLAG_SECURE handler, `SceneDelegate` cover or `AppDelegate`
backup-exclusion handler you were carrying for the wallet; this package
replaces them.

## iOS cannot block a live screenshot

iOS has **no `FLAG_SECURE`**. The iOS cover protects **only** the app-switcher
snapshot; a live foreground screenshot is still possible. This plugin
therefore registers **no** iOS `screen_security` success handler, so
`zec_wallet_ui` keeps its no-op adapter on iOS and honestly tells the user to
make sure no one can see their screen. Do **not** add an iOS `enableSecure`
handler that returns success: `zec_wallet_ui`'s `_invoke` treats *any* success
reply as "protected", which would switch the seed screen's text to the false
"screenshots are off".

## On-device acceptance test

The cover is race-free by construction (added synchronously at
resign/deactivate, before the snapshot), but iOS snapshot behaviour must be
verified on a device:

1. Build and run on a real device or simulator (iOS 15+).
2. Reveal the recovery phrase (onboarding, or Settings → Security → Recovery
   phrase → reveal).
3. Open the **app switcher**. The wallet thumbnail must show the **cover, not
   the seed words**.
4. Return. The cover clears immediately.
5. Repeat on the **balance**, **receive-address**, and **send / swap** screens.
   Those thumbnails must also be covered (`send`/`swap` have no Dart auto-hide
   fallback, so this is their only iOS snapshot protection).
6. Confirm a foreground screenshot on a non-secret screen still works (the cover
   is background-only).
7. **Transient interruption:** with the seed revealed, pull down Control Center
   (or trigger a notification banner). The cover appears **and clears** when you
   dismiss it. The app never backgrounds here, so this catches a botched
   removal that would leave the cover stuck over the live app. If the brief
   flash on transient interruptions is unwanted, move the *add* to
   `sceneDidEnterBackground` (still before the snapshot); never move the
   *removal* off activation.

Android FLAG_SECURE: reveal the seed, try to screenshot (blocked), and check the
recents thumbnail (blank).

**Also verify a classic-lifecycle host**: an app whose `Info.plist` has **no**
`UIApplicationSceneManifest`. The iOS cover then relies on the app-level
notifications + `connectedScenes` rather than the scene callbacks, a path the
scene-based example does not exercise. Confirm the app-switcher thumbnail is
covered there too.

## Status: pre-release (0.0.1)

Both native halves compile in the example app's Android and iOS builds.
Android `FLAG_SECURE` is verified on a device (`fl=SECURE` on the window,
black screenshots and recents thumbnail, kept across rotation). The iOS
app-switcher cover's on-device acceptance steps above have not all been run
yet, and the classic-lifecycle host is unverified.

## Notes

- The iOS cover is kept **per scene**. Each window scene (iPad multi-window)
  protects its own Flutter content, covers **only** a window that hosts a
  `FlutterViewController` (an add-to-app host's pure-native screens are left
  alone), and a disconnecting scene drops its cover entry so no dead window's
  view is retained. The cover sets `accessibilityViewIsModal`, so VoiceOver
  can't reach the content behind it.
- The cover is **app-wide within a scene** (a brief blur when backgrounding any
  screen). This is intended: it matches banking-app UX and keeps balances and
  addresses out of the snapshot too. A gated "only when a secret is shown"
  variant is possible but reintroduces Dart↔native sync.
- **One wallet engine per Activity window (Android).** `FLAG_SECURE` is a
  per-window flag and this plugin holds no cross-engine refcount, so two
  channel-speaking wallet `FlutterEngine`s sharing a single Activity window (an
  unusual add-to-app layout) could clear each other's flag. Run the wallet UI in
  a single engine per window.

## Independence and trademarks

This package is part of `zec_wallet`, an independent, open-source project.
It is not affiliated with, endorsed by, or sponsored by the Zcash Foundation.
"Zcash" is a trademark of the Zcash Foundation; it is used here only to
describe the cryptocurrency this library works with. No Zcash logo is used.
The project is unrelated to ZecWallet, the discontinued Zcash wallet application.

## License

MIT; see `LICENSE`.
