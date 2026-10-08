# Changelog

## 0.0.1

Initial release. The reusable wallet UI for the `zec_wallet` SDK, embeddable in
a host Flutter app with a page of glue (see the README).

- **The wallet surfaces**: onboarding (create, restore, watch-only), the
  balance card with "Spendable now" and "Arriving", activity and transaction
  details, send (payment URIs, memos, QR scan, the offline queue, a large-send
  confirm, an acknowledgement before a public payment), receive, shielding,
  swap, recovery-phrase backup, viewing-key export, settings, sync status and
  the sync-server picker, reached from the sync status or the menu's "Sync
  server". A custom server in the picker can carry an access key with the
  header it goes in; the key is shown while typed (Hide hides it) and can be
  pasted but never copied. The trust notice tells the user that the key
  identifies them to that server. The host's settings entry reads "Settings".
  The address scanner needs `flutter_zxing` 3.1 or later: with 2.3 an iOS
  profile or release build decoded nothing (hosts need Android compileSdk 36).
- **Host seams**, all exported from the barrel: `walletOnboardingOverrides`
  (with `decorateProvisioner`), the provisioner, onboarding-store and
  screen-security interfaces, the spend and reveal authorizers, `walletRoutes()`
  and `WalletRoutes` for a host `GoRouter`, `walletSyncDriveProvider` for sync
  from the moment the wallet opens, `walletBalanceHiddenProvider`, and the
  reference policy (`buildWalletConfig`, `resolveWalletDbDir`) a host may use
  verbatim or replace.
- **Theming**: `buildTheme` and the `WalletColors` tokens (light, dark and
  true-black presets), `WalletShapes`, `WalletTypography` and `WalletIcons`; a
  host's own theme styles every component. No bundled font.
- A screen never claims a screenshot block the OS did not acknowledge, a
  stale balance says so, and a send's result states only what the core
  reported.
- A busy or briefly unreadable wallet store reads "Sync paused on this device.
  Retrying." in amber, never the restore-from-recovery-phrase copy, which is
  kept for a corrupt store.
- A transaction that may already be saved is never offered a retry: send,
  shield, move-to-transparent and the parked "Send now" land on "check before
  trying again" (or "may already be sending it") when the answer was lost, and
  show what landed when it did, whatever the host's authorizer throws after
  running the spend.
- A swap's review shows the full foreign address: the refund address for a
  swap into ZEC, the payout address for one out of ZEC. Start stays off until
  the user confirms it. A host's spend prompt for a swap deposit gets the
  payout address too, shortened, in `WalletSpendIntent.payoutAbbrev`.
- **16 locales**, each carrying every string, through
  `walletLocalizationsFallbackDelegate`.
- **A test kit** (`package:zec_wallet_ui/testing.dart`, no `flutter_test`
  dependency) that pumps every surface with no native library.
- A wallet runs on Android, iOS and macOS. Linux builds, but `zec_wallet`
  cannot hold a wallet there yet (no key store adapter). Windows is not yet
  built or run.
