# zec_wallet_ui example

The full reference app is
[`zec_wallet/example`](https://github.com/alvorado-lab/zec-wallet/tree/main/zec_wallet/example):
its `lib/main.dart` is the whole "page of glue". The minimum a host wires:

```dart
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:zec_wallet/zec_wallet.dart' show RustLib;
import 'package:zec_wallet_ui/zec_wallet_ui.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init(); // see zec_wallet's README for the iOS/macOS loader

  final dbDir = await resolveWalletDbDir();
  final router = GoRouter(
    initialLocation: WalletRoutes.wallet,
    routes: walletRoutes(),
  );

  runApp(
    ProviderScope(
      overrides: walletOnboardingOverrides(
        config: buildWalletConfig(dbDir: dbDir),
      ),
      child: MaterialApp.router(
        theme: buildTheme(WalletColors.light),
        localizationsDelegates: const [
          walletLocalizationsFallbackDelegate,
          // ...your own delegates
        ],
        routerConfig: router,
      ),
    ),
  );
}
```

On Android and iOS, also add `zec_wallet_ui_platform` for screen protection and
backup exclusion (see this package's README).
