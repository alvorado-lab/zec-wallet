# zec_wallet_tor example

The full reference app is
[`zec_wallet/example`](https://github.com/alvorado-lab/zec-wallet/tree/main/zec_wallet/example)
(`lib/core/tor/tor_plugin.dart`). The order at boot:

```dart
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_tor/zec_wallet_tor.dart';

Future<WalletConfig> bootWithTor({
  required String torDir,
  required String dbDir,
}) async {
  // 1. The wallet's native library first: the plugin finds it in the process.
  await RustLib.init(); // see zec_wallet's README for the iOS/macOS loader

  // 2. Then the plugin. It registers with the wallet and starts Tor in the
  //    background; this call does not wait for Tor to be ready. It throws
  //    TorPluginError: switch on `kind` (a closed enum), never the message.
  await ZecWalletTor.init(torDir: torDir);

  // 3. Then open the wallet with the registered transport.
  return WalletConfig(
    dbDir: dbDir,
    network: Network.main,
    endpointUrl: 'https://your-lightwalletd',
    tor: const TorPolicy.required_(runtime: TorRuntimeConfig.hostDialer()),
    seedPersistence: SeedPersistence.sealedKeychain,
    broadcastJitter: const JitterPolicy.uniform(maxMs: 10000),
    machineMemoPrefixes: const [],
  );
}
```

`ZecWalletTor.statusStream` reports bootstrap progress for your UI.
