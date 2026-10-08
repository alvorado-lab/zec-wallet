/// Optional companion plugin that ships the DEFAULT native platform behaviours
/// for the plugin-free [`zec_wallet_ui`] package (the "Scenario C" hybrid).
///
/// Adding this package to a host app wires, with **zero host native code**:
///
///  * **Android** — a `FLAG_SECURE` handler for the `zec_wallet_ui/screen_security`
///    channel, so the recovery-phrase / send / swap screens block live
///    screenshots AND the OS recents-thumbnail snapshot while a secret is shown.
///  * **iOS** — a handler for `zec_wallet_ui/backup_exclusion`
///    (`NSURLIsExcludedFromBackupKey` on the wallet DB), **and** an app-wide
///    app-switcher privacy cover so no on-screen content (seed phrase, balance,
///    addresses, tx detail — app-frame A8) is captured into the iOS recents
///    thumbnail. iOS has no `FLAG_SECURE`, so the cover protects **only** the
///    snapshot — live screenshots stay possible, which is why `zec_wallet_ui`
///    still honestly says "make sure no one can see your screen" on iOS. This
///    plugin deliberately registers **no** iOS `screen_security` success handler
///    (that would flip the copy into the false "screenshots are off").
///  * **Desktop** — nothing: `zec_wallet_ui`'s built-in no-op adapter already
///    reports "unprotected, hide your screen" honestly.
///
/// There is **no Dart API to call**. The native handlers self-register when the
/// host adds this dependency (standard Flutter plugin registration), and
/// `zec_wallet_ui`'s own Dart speaks the channels this plugin answers. The
/// constants below name that channel contract; they MUST stay equal to
/// `zec_wallet_ui`'s `MethodChannelScreenSecurity.channelName` and
/// `BackupExclusion.channelName` — pinned by the contract test in this package.
library zec_wallet_ui_platform;

/// The screen-security channel this plugin answers natively **on Android**
/// (`enableSecure` / `disableSecure` → `FLAG_SECURE`).
const String kScreenSecurityChannel = 'zec_wallet_ui/screen_security';

/// The backup-exclusion channel this plugin answers natively **on iOS**
/// (`excludeFromBackup` → `NSURLIsExcludedFromBackupKey`).
const String kBackupExclusionChannel = 'zec_wallet_ui/backup_exclusion';

/// The reconnect channel this plugin answers natively on **Android AND iOS**
/// (#404) — an `EventChannel` that ticks when the device regains a usable
/// network, so `zec_wallet_ui`'s sync loop can reset its retry backoff instead
/// of leaving the badge on "Sync paused" for minutes after a FOREGROUND
/// reconnect. Android: `ConnectivityManager.NetworkCallback.onAvailable`.
/// iOS: an `NWPathMonitor` unsatisfied→satisfied edge.
///
/// The tick carries NO payload (§5.4) and is ADVISORY — it means the OS has a
/// network, never that the wallet's endpoint answers. Desktop registers nothing,
/// so the stream is simply empty there and the wallet keeps its pre-#404
/// behaviour.
///
/// ANDROID HOSTS INHERIT ONE PERMISSION: `ACCESS_NETWORK_STATE` (normal
/// protection level, granted at install, absent from the Play listing's
/// user-facing permission set). This package's `AndroidManifest.xml` documents
/// the opt-out for a host that would rather feed its own connectivity signal
/// through `walletNetworkReachabilityProvider`.
const String kNetworkReachabilityChannel = 'zec_wallet_ui/network_reachability';
