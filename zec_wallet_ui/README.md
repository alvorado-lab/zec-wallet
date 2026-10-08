# zec_wallet_ui

A complete shielded Zcash wallet for your Flutter app, as screens you drop
in. Onboarding, backup and restore, balance, send and receive, history,
shielding and swaps are ready to mount under your router, styled by your
theme, in 16 languages. Wiring it takes about a page of code.

It is optional: build your own screens on
[`zec_wallet`](https://pub.dev/packages/zec_wallet) instead if you prefer. The
UI holds no keys and no wallet logic of its own; everything goes through the
SDK, whose Rust core keeps the seed and spending keys out of Dart.

Part of the `zec_wallet` family: [`zec_wallet`](https://pub.dev/packages/zec_wallet)
(the SDK), this package,
[`zec_wallet_ui_platform`](https://pub.dev/packages/zec_wallet_ui_platform)
(optional native screen protection for these screens), and
[`zec_wallet_tor`](https://pub.dev/packages/zec_wallet_tor) (optional Tor).

**Status: pre-release (0.0.1), not yet independently audited.** It moves real
funds through `zec_wallet`; see [`SECURITY.md`](SECURITY.md). The SDK example
app (`zec_wallet/example`) is the reference consumer: its `main.dart` is the
canonical "page of glue", and its thin test suite pins what a consumer relies
on (font-manifest resolution, delegate composition, shell routing).

## What a host wires (the whole glue surface)

1. A `ProviderScope` spreading `walletOnboardingOverrides(config: …)`. You own
   the `WalletConfig`: network, lightwalletd `endpointUrl`, `TorPolicy`,
   `SeedPersistence`, broadcast jitter, and the resolved `dbDir` all come from
   the host (the SDK validates the config; it never chooses it):

   ```dart
   // All config vocabulary (WalletConfig, Network, TorPolicy, …) is reachable
   // through this package's barrel; the full SDK surface (incl. RustLib.init)
   // comes from `package:zec_wallet/zec_wallet.dart`.
   import 'package:zec_wallet_ui/zec_wallet_ui.dart';

   // Bring-your-own policy (production host). dbDir MUST be absolute: a
   // relative path throws at wiring time (on desktop it would otherwise pin
   // the wallet to the launch cwd). On iOS, a bring-your-own dbDir must also
   // be excluded from backups yourself: `await BackupExclusion().exclude(dbDir)`
   // (the reference resolveWalletDbDir does both for you).
   final config = WalletConfig(
     dbDir: myDbDir,                       // your resolved path
     network: Network.main,
     endpointUrl: 'https://your-lightwalletd',
     tor: TorPolicy.required_(runtime: myTorRuntimeConfig),
     seedPersistence: SeedPersistence.sealedKeychain,
     birthdayHeight: null,                 // create at tip; restore overrides
     broadcastJitter: const JitterPolicy.uniform(maxMs: 10000),
     machineMemoPrefixes: const [],        // empty = machineMemos stays closed
   );
   return walletOnboardingOverrides(config: config);

   // Or ride the reference defaults verbatim (zec.rocks / Tor off). CAVEAT:
   // buildWalletConfig fixes the network at BUILD time from --dart-define
   // (ZEC_NETWORK, default mainnet; ZEC_ENDPOINT override), so there is no
   // runtime network selection on this path. Bring your own WalletConfig if
   // your product chooses network/endpoint at runtime.
   final dbDir = await resolveWalletDbDir();               // reference helper
   return walletOnboardingOverrides(config: buildWalletConfig(dbDir: dbDir));
   ```

   `walletOnboardingOverrides` co-wires the provisioner, onboarding store, and
   screen-security adapter together (the H2 co-wiring invariant). For finer
   control, override the individual seams (`walletProvisionerProvider`,
   `onboardingStoreProvider`, `screenSecurityProvider`) directly; the barrel
   exports them all.
2. Swap, only if wanted: `walletSwapOverrides()` is the REFERENCE policy. It
   enables the swap surface against the reference 1Click endpoint. That is a
   product decision, not a default: hosts construct their own
   `SwapHostPolicy` (or skip the override entirely; the swap surface then
   stays honestly disabled).
3. The theme. Either:
   - (a) `buildTheme(WalletColors.light)` (or `.dark` / `.darkAmoled`, or your
     own `WalletColors`), optionally with `textTheme:` and `extensions:`; or
   - (b) your own `ThemeData` with a `WalletColors` extension registered (map
     your palette into the package tokens, all of them, since the constructor
     requires every field).

   Under (b), **your theme styles the wallet's components**: its buttons,
   fields, sheets, dialogs, chips and progress bars read your component
   themes, because no SDK call site sets a size, border or colour of its own
   (the few exceptions are colours that carry meaning, such as a destructive
   red). Keep a 48 dp button minimum or Material's padded tap target: a
   `materialTapTargetSize: shrinkWrap` theme with no `minimumSize` shrinks the
   wallet's Send and Delete buttons under 44 dp. `buildTheme` sets all of
   these for you.

   Four hooks restyle the rest; each is optional and falls back to the
   SDK's default:
   - **`WalletShapes`**: the corner radii, one per role (`hero`, `group`,
     `tile`, `bar`, `notice`, `qr`, `field`, `chip`, `sheet`, `dialog`,
     `progress`). The defaults follow the platform (a group is 26 and a sheet
     38 on iOS, 24 and 28 elsewhere).
   - **Fonts.** The package bundles NO font. Bundle your own and pass a
     `TextTheme` to `buildTheme(textTheme:)`; its families are merged over the
     SDK's sizes. With none, the platform's default face is used.
   - **`WalletTypography`**: the `mono` face for addresses and other
     identifiers (default: the platform's generic monospace).
   - **`WalletIcons(builder:)`**: one function from a `WalletGlyph` (every
     glyph the wallet draws, named for its meaning: `send`, `receive`,
     `shielded`, `transparent`, `syncProblem`, …) to any widget. Return null
     for a glyph you don't map and the SDK draws its Material default. Honour
     the ambient `IconTheme` when size or colour is null, and announce a
     non-null `semanticLabel`.

   `WalletColors.qrInk` colours the QR modules (default black). The tile stays
   white, and an ink paler than 7:1 on white falls back to black, because a
   code that does not scan is an address nobody can pay. The balance card's
   coin takes `coinFace`, `coinEdge` and `coinRim` (optional; derived from
   `deep` and `accent` when unset). The presets are the refreshed green design
   (ADR-0564).

   **Hide balance.** The eye in the wallet header masks every amount on the
   glance surfaces (the balance card, activity, the transaction sheet, the
   in-flight note), and screen readers hear "Balance hidden". It never masks
   an amount the user is acting on (a form, a review, a parked send). The
   state is `walletBalanceHiddenProvider`: session-only by default; override
   it to persist the choice, and read it to mask your own amount surfaces.

   **The sync bar hides when the wallet is synced and healthy**, and the
   sync-status sheet then sits in the overflow menu. It hides only on a
   transport claim that cannot go stale while the tab is open: a direct
   connection, or a protection your `walletHostTransportProvider` override
   declares. A host transport claim overrides every SDK Tor state, the hide
   included, so declare protection only when your transport guarantees it.
4. `walletLocalizationsFallbackDelegate` added to `localizationsDelegates`
   next to the host's own generated class (distinct types, so no collision).
   Use the fallback delegate, NOT the raw `WalletLocalizations.delegate`: with
   a host `supportedLocales` wider than the wallet ARB set, the raw delegate is
   skipped on the untranslated locale and the first wallet frame throws. The
   fallback serves English instead. 16 locales ship
   (`lib/l10n/wallet_*.arb`), each carrying every string.
5. `walletRoutes()` mounted under the host router. go_router is required:
   the wallet screens navigate via go_router context extensions, so they
   must live under the host's `GoRouter`. Navigate via the `WalletRoutes`
   constants. The ONE wallet→host navigation is the settings seam:
   `walletAppearanceRoutePathProvider` (its historical name) defaults to null
   (entry points hidden); override it with your settings route to surface the
   wallet's "Settings" entries.
6. Optional: sync whenever the app is in the foreground. By default the sync
   loop starts the first time the wallet screen renders. To sync from the
   moment your wallet is open, on any screen, listen to the drive once at
   your root:

   ```dart
   ref.listen(walletSyncDriveProvider, (_, __) {});
   ```

   It runs while `walletSessionProvider` is non-null, your sync policy
   (`walletSyncPolicyProvider`) is on and the app is not paused. It suspends
   on `paused` and resumes on return. With no session it is idle and costs
   nothing. It does not open the wallet: when the wallet opens (at launch, at
   unlock, or on first use) is your decision, and sync can start no earlier.

### What the package promises a host about lifecycle, reveals and sends

- **One lifecycle mutation at a time.** Switch server, delete, rescan,
  restore, confirm-backup and re-provision run under one operation
  generation: a second mutation while one is in flight is REFUSED typed (a
  delete during a server switch answers `WalletDeletionOutcome.notDeletable`
  with "finish the server switch first"), and a completion from a superseded
  operation lands nowhere. A host that chains delete-after-switch must await
  the switch.
- **A reveal grant is valid for one wallet session instance, one operation
  generation and one foreground session.** Your `WalletRevealAuthorizer`'s
  shape is unchanged; the package binds what it returns. If the wallet
  identity changes under a mounted reveal screen (a switch, a delete, the
  same wallet re-opened under a fresh session), the grant is void, nothing
  of the new wallet is shown without its own prompt, and a prompt answered
  after the change reveals nothing. Backgrounding still hides a revealed
  secret and re-prompts on resume.
- **A tagged send names what it paid.** When a push you tagged with a
  `correlationId` ends in `WalletSendTransactionCreated`,
  `recipientAmountZat` is the total paid to the send's ONE recipient address,
  in zatoshis, as signed (fee excluded), or `null` when the send paid more
  than one address (a unified address and one of its own receivers count as
  two). It states what was SIGNED, never that it arrived: gate delivery copy
  on `motion` and the wallet's own surfaces, never on the amount's presence.
  An untagged push gets no amount.

## Native handlers: `zec_wallet_ui` + the optional `zec_wallet_ui_platform` plugin

**Why there are two packages.** `zec_wallet_ui`
ships no platform channel handlers of its own. (Its native code comes only
through its dependencies: the Rust core in `zec_wallet` and the QR reader,
`flutter_zxing`.) That is deliberate: it adds no screen-protection policy to
your app, and the package stays usable on **desktop**, where none of the
mobile screen protection applies. But two of the wallet's
guarantees, Android **FLAG_SECURE** and the iOS **backup exclusion +
app-switcher privacy cover**, are native and cannot be expressed
in Dart. Baking a fixed native policy into the UI package would
force it on every host and turn a cross-platform package into a mobile
plugin. So that native code lives in a **separate, optional companion plugin**,
`zec_wallet_ui_platform`, which answers the two platform channels
`zec_wallet_ui` speaks. You pick how to supply them:

- **Most hosts → add the companion plugin.** One `pubspec.yaml` dependency,
  zero native code in your shell (it self-registers through the generated plugin
  registrant). This is the supported default and what the reference consumer
  (`zec_wallet/example`) uses.
- **A host with its own capture policy → omit it and answer the channels
  yourself.** If your app already manages FLAG_SECURE / app-switcher privacy, or
  a duress/decoy shell needs a *different* posture, do not add the plugin
  and wire your own handlers against the channel contracts (see the hand-wire
  note below). Keeping the native side out of `zec_wallet_ui` is what
  makes this option possible.
- **Desktop → nothing to add.** The plugin is Android/iOS-only. On macOS
  `zec_wallet_ui` is all you need, and the `screen_security` channel goes
  unanswered; the UI degrades honestly (see per-channel behavior below). Linux
  and Windows cannot hold a wallet yet: `zec_wallet` has no key store adapter
  for them, so it refuses to create or open one there (`vaultAbsent`) and
  onboarding says the device has no secure key store. Windows is not yet
  built or run.

The companion plugin ships:

- **Android**: the `zec_wallet_ui/screen_security` handler, FLAG_SECURE over
  the recovery-phrase (and other secure-tier) screens, re-asserted across
  Activity-recreating config changes. Device-verified: `fl=SECURE` on the
  window, black screenshots/recents thumbnail, survives rotation.
- **iOS**: the `zec_wallet_ui/backup_exclusion` handler (excludes the wallet
  data dir from iCloud/device backups) plus an app-switcher privacy cover on
  resign-active.
- **Android AND iOS**: the `zec_wallet_ui/network_reachability` EventChannel,
  a payload-free tick when the device regains a usable network, so the sync
  loop resets its retry backoff instead of leaving the badge on "Sync paused"
  for minutes after a *foreground* reconnect (airplane toggle, wifi switch,
  tunnel exit). Android registers a `ConnectivityManager.NetworkCallback`,
  which means an Android host merging the plugin **inherits the normal-level
  `ACCESS_NETWORK_STATE` permission** (install-time, no runtime prompt; Play has
  historically surfaced it as "view network connections", so do not tell your
  users it is invisible). iOS uses `NWPathMonitor`. Android's `NetworkRequest`
  defaults exclude VPN transports, so a VPN-only reconnect over otherwise-stable
  wifi ticks on iOS and not on Android. That is worth knowing for a
  Tor/VPN-heavy userbase; the wallet just falls back to its ladder there. See the plugin's
  `AndroidManifest.xml` for the opt-out.

The reference consumer (`zec_wallet/example`) depends on the plugin and
carries **no** hand-written handlers: its `MainActivity.kt`/
`AppDelegate.swift` are deliberately bare. Do NOT add your own handler on
either channel alongside the plugin: a handler registered after the plugin
registrant silently REPLACES the plugin's (last registration wins on a
method channel).

Why this matters, per channel:

- `screen_security` fails HONEST without a handler: `enable()` reports false
  and the seed screen keeps the "screenshots possible" copy. It never claims
  a protection that isn't running. The plugin is what turns the copy into a
  real OS block.
- `network_reachability` fails QUIETLY BY DESIGN without a handler: the stream
  is empty, and nothing is reported to `FlutterError` either (the adapter awaits
  its own channel activation so a `MissingPluginException` is an expected answer
  rather than a crash-reporter beacon naming our channel). The wallet
  keeps its pre-plugin behaviour. **How much that costs you depends on the
  platform.** On MOBILE a
  real background/resume still self-heals (backgrounding stops the loop, so the
  resume spawns a fresh one with a fresh ladder), and you lose only the automatic
  recovery on a *foreground* reconnect. On DESKTOP there is no such fallback at
  all: `paused` never fires there, so the loop is never stopped and never
  respawned, and the sync sheet's manual "Try now" is the only recovery. That is
  how the wallet behaved before this plugin existed, not a regression, but plan for it. A host that
  already runs its own connectivity listener should override
  `walletNetworkReachabilityProvider` instead of adding a second one.
- `backup_exclusion` fails SILENT without a handler: nothing in the UI tells
  you the wallet DB is riding into iCloud/device backups, and a backed-up DB
  restored onto a new device cannot be opened (its key is a ThisDeviceOnly
  keychain key that does not travel), stranding the user on the
  needs-recovery screen. An iOS host without the plugin MUST implement this
  channel itself.

Only if your shell cannot take the plugin (e.g. a bespoke native embedding):
implement the channels by hand against the contracts in
`screen_security_channel.dart` / `backup_exclusion.dart` /
`reconnect_kick.dart`, using the plugin's
Kotlin/Swift sources as the reference implementation. Keep the
config-change re-assert (Android) and the per-scene cover (iPad multi-window)
behaviors, both of which the naive one-Activity/one-window port drops.

## Testing a host integration

`import 'package:zec_wallet_ui/testing.dart';` gives you `FakeWalletSession`
(a scripted `WalletSession`) and the fake onboarding seams, which let a host pump
every wallet surface in plain widget tests, no native library needed. The
library deliberately imports no `flutter_test`, so it never constrains a
consumer's test-framework solve.

## Platform notes

- No FFI initialization happens at package import; a host without the native
  library gets the honest "wallet unavailable" surface instead of a crash.
- Android: `flutter_zxing` (the on-device QR reader) fails to build on NDK
  r28+ (upstream #225), so pin NDK r27 in the app's android build config, as
  the reference consumer does. The QR scan is additive; paste/type always
  works and is the only path on desktop (on macOS sheets cap at
  `walletSheetMaxWidth` on wide windows; Linux and Windows cannot hold a
  wallet yet).
- iOS: the camera QR scan (the swap address fields AND the watch-only
  viewing-key import) is reachable by default on every mobile build, so the
  host app's `Info.plist` MUST carry `NSCameraUsageDescription`. Without it
  iOS hard-kills the app the moment the reader initializes (a crash, not a
  soft denial). The reference example app carries it; copy its wording or
  write your own. Android needs nothing (the camera permission arrives via
  the plugin manifest and denial degrades to the paste path honestly).
- Keyboards: the recovery-word and viewing-key fields turn off autocorrect,
  suggestions, keyboard learning and autofill, but a third-party keyboard
  still receives every keystroke. On iOS a host can refuse custom keyboards
  app-wide (`application(_:shouldAllowExtensionPointIdentifier:)` returning
  false for `.keyboard`); Android has no equivalent, and keyboard learning can
  only be asked off, not enforced.

## Independence and trademarks

This package is part of `zec_wallet`, an independent, open-source project.
It is not affiliated with, endorsed by, or sponsored by the Zcash Foundation.
"Zcash" is a trademark of the Zcash Foundation; it is used here only to
describe the cryptocurrency this library works with. No Zcash logo is used.
The project is unrelated to ZecWallet, the discontinued Zcash wallet application.

## License

MIT; see `LICENSE`.
