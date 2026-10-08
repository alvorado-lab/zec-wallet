# Changelog

## 0.0.1

Initial release. The optional native companion for `zec_wallet_ui`.

- **Android**: `FLAG_SECURE` over the recovery-phrase and other secure
  screens (`zec_wallet_ui/screen_security`), re-applied after an
  Activity-recreating configuration change.
- **iOS**: wallet-directory backup exclusion
  (`zec_wallet_ui/backup_exclusion`) and an app-switcher privacy cover, per
  scene, over the windows that host Flutter only.
- **Android and iOS**: the `zec_wallet_ui/network_reachability` event
  channel, a payload-free tick when the device regains a usable network. On
  Android it adds the normal-level `ACCESS_NETWORK_STATE` permission (see the
  README for the opt-out).
- A contract test pins the channel names to `zec_wallet_ui`'s.
