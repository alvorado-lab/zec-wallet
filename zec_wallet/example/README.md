# zec_wallet_example

The example app for the **`zec_wallet`** Flutter SDK: a working ZEC (Zcash)
wallet built entirely on the SDK's Rust core. It is both the SDK's reference
integration and the host for its on-device tests.

The wallet's spending keys never cross into Dart. The seed, spending keys and
DB key live in Rust. Dart sees addresses, balances, sync status, history and
custody disclosures, plus the two sanctioned crossings the SDK README names
(the recovery words on backup and restore, the viewing key for watch-only).
This app shows how a host consumes that boundary.

## What it demonstrates

- **Onboarding** — create a new wallet or restore from a BIP39 phrase, with the
  recovery phrase shown once on a screenshot-protected (`FLAG_SECURE`) screen.
- **Balance, send, receive** — the ZIP-321 send pipeline with money-safety
  confirmations, a QR receive address, and sync status.
- **History** — transaction list with memo content and an incoming-tx watch
  stream.
- **Sync & rescan** — live sync status plus a rescan/change-birthday recovery
  flow.
- **Swap** — an optional ZEC↔asset on-ramp via NEAR Intents (the `swap-near`
  adapter; gated by host config).
- **Key custody & delete** — Settings → Security renders the measured custody
  tier (StrongBox / TEE / Secure Enclave / software) and whether deletion
  removes a key held by secure hardware or is best effort, then performs the
  destructive wipe honestly.
- **Settings** — the wallet overflow menu's "Settings": the Tor switch first,
  then theme mode, text scale and AMOLED.
- **Device log** — Settings → Device log (Off / Errors only / Detailed), and
  **Share device log**. See below.
- **Tor** — the optional `zec_wallet_tor` package, off until the user turns on
  "Use the built-in Tor plugin" at the top of Settings
  (`lib/core/tor/tor_plugin.dart`). Its native library measured 9.8–15.3 MB
  per Android ABI in the release build.

## The device log

The app keeps the SDK's device-log lines (`watchDeviceLog`) in memory, at most
the latest 500. **Share device log** hands them to the OS share sheet.

The level it asks the SDK for, in this order:

1. the user's choice in Settings, once they have made one;
2. otherwise `--dart-define=ZEC_WALLET_DEVICE_LOG=off|errors|detailed`;
3. otherwise the build default: Detailed in a debug build, Errors only in a
   profile or release build.

The row shows the level the SDK answers, not the one asked for. After every
Delete the app re-arms the log. That is housekeeping for an app with no duress
trigger. A host with a duress path must not copy it (see
`lib/core/logging/device_log.dart`).

**Linux, and Windows before 10 RS5, share through a `mailto:` link.** There
the whole text rides in the link's `body=`:

- It is capped at 8 KB. The header is cut to 1 KB first. Then the newest lines
  are kept, and one line says how many were dropped: "N earlier lines left
  out".
- A mail app may cut the body further, without saying so.
- The mail app usually receives the link as a command-line argument. On a
  shared computer, other users can often read a program's arguments while it
  runs. Share there only what you would show them.

## Running

The SDK is an `ffiPlugin`: Cargokit builds and bundles the Rust core into device
builds automatically — no manual Rust step.

```sh
flutter pub get
flutter run            # Android / iOS / macOS / Linux (Windows: not yet built or run)
```

On iOS and macOS the Podfiles use `use_frameworks!`, so the Rust core is
`zec_wallet.framework`. This app gives `RustLib.init` the process loader (see
`lib/core/ffi/wallet_ffi.dart`), which is sound here only because the app
carries a single flutter_rust_bridge library. **A host app should open the
framework by name instead**, as the SDK README's "Building" section shows.

The swap demo needs a 1Click integrator JWT, injected at build time and never
committed. It is compiled into the build, so anyone who unpacks the app can
extract it; it bills and attributes the integrator, it does not reach user
funds:

```sh
flutter run --dart-define=ZEC_WALLET_SWAP_JWT=<token>
```

Without it, quotes fail closed back to the form (no money risk) — everything
else works.

## Tests

- `flutter test` — host-VM widget/unit tests. These never build native code;
  the wallet runs against a fake session, so the suite is fast and offline.
- `flutter test integration_test` — on-device gates: the bridge self-test
  (`bridge_selftest_test.dart`, drives `runBridgeSelftest` from
  `lib/bridge_selftest.dart`) and the custody round-trip
  (`custody_e2e_test.dart`, exercises the device's real keystore). These need a
  connected device or simulator.

## Learn more

See the parent package's [README](https://github.com/alvorado-lab/zec-wallet/blob/main/zec_wallet/README.md) for the SDK API surface and
the keys-in-Rust design.
