# zec_wallet_ui_platform example

Add the dependency next to `zec_wallet_ui`. There is no Dart code to write: the
plugin registers itself through Flutter's generated plugin registrant and
answers the channels `zec_wallet_ui` speaks.

```yaml
dependencies:
  zec_wallet_ui: ^0.0.1
  zec_wallet_ui_platform: ^0.0.1
```

With it, the recovery-phrase screens are blocked from screenshots on Android
(FLAG_SECURE), the wallet directory is excluded from iOS backups, and the iOS
app switcher shows a privacy cover. The full reference app is
[`zec_wallet/example`](https://github.com/alvorado-lab/zec-wallet/tree/main/zec_wallet/example).
