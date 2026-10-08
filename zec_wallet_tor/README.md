# zec_wallet_tor

Route your Zcash wallet's traffic through Tor. Add this package next to
[`zec_wallet`](https://pub.dev/packages/zec_wallet), start it at boot, set
the wallet's Tor policy, and the wallet's server sees a Tor exit instead of
your user's IP address.

It is optional. `zec_wallet` itself contains no Tor code, so an app that does
not add this package carries none.

How it works: the package bundles an arti-based Tor client (the Rust
[`dialer-tor`](https://crates.io/crates/dialer-tor) crate) in its own native
library. `ZecWalletTor.init` registers it with the wallet through the
wallet's host-dialer contract, the same contract a host uses to plug in a
transport of its own. The wallet then connects to its server through Tor, and
its UI shows the transport as "Tor" with the state the plugin reports.

Part of the `zec_wallet` family: [`zec_wallet`](https://pub.dev/packages/zec_wallet)
(the SDK), [`zec_wallet_ui`](https://pub.dev/packages/zec_wallet_ui) (an
optional drop-in wallet UI),
[`zec_wallet_ui_platform`](https://pub.dev/packages/zec_wallet_ui_platform)
(optional native screen protection for that UI), and this package.

## Status: pre-release (0.0.1)

Unaudited. See [SECURITY.md](SECURITY.md).

## Use

```yaml
dependencies:
  zec_wallet: ^0.0.1
  zec_wallet_tor: ^0.0.1
```

The order at boot matters:

```dart
// 1. The wallet's native library first: the plugin finds it in the process.
await RustLib.init(externalLibrary: ...);

// 2. Then the plugin. It registers with the wallet at readiness 0 and starts
//    Tor in the background; this call does not wait for Tor to be ready.
await ZecWalletTor.init(torDir: torDir);

// 3. Then open the wallet with the registered transport.
final config = WalletConfig(
  // ...
  tor: const TorPolicy.required_(runtime: TorRuntimeConfig.hostDialer()),
);
```

Calling `init` before `RustLib.init()` throws
`TorPluginError(kind: TorPluginErrorKind.walletNotLoaded)`. Opening a wallet
with `TorRuntimeConfig.hostDialer()` while nothing is registered is refused
by the wallet (`RW-CFG-001`).

Every verb throws `TorPluginError`. Catch it by type and switch on `kind`, a
closed enum. Never match on the message; it is for logs and developers.

| verb | what it does |
|---|---|
| `init({torDir, bridges})` | Registers with the wallet and starts Tor. Idempotent. |
| `status()` | The current `TorPluginStatus`: phase, readiness from 0 to 100, arti's blockage, a failure class. |
| `statusStream` | Every change of status, delivered on your isolate's event loop. |
| `setBridges(String?)` | Replaces the bridge lines and rebuilds the client. A refused paste changes nothing. |
| `retryBootstrap()` | After a failed bootstrap, retry now instead of waiting out the backoff. |
| `dispose()` | Unregisters from the wallet and stops Tor. |
| `clearState(torDir)` | Deletes the plugin's Tor state. For a wipe flow; see below. |

The plugin listens to the app lifecycle itself. When the app is paused it
tells the wallet the transport is not ready and puts arti to sleep. On
resume it wakes arti, or rebuilds the client if the pause was long enough
for the OS to have dropped its sockets. A host that subscribes to
`statusStream` pauses its own subscription on `AppLifecycleState.paused`, as
it does for any stream.

Before its first call into the native library, the Dart side reads the
library's contract version by name. A library from a different release is
refused with `TorPluginErrorKind.pluginAbiMismatch`. Use `zec_wallet` and
`zec_wallet_tor` from the same release.

## `torDir`: a sibling of the wallet's directory, never inside it

Give the plugin a directory of its own, next to the wallet's `dbDir`, for
example `<app support>/zec_wallet` for the wallet and `<app support>/tor` for
the plugin. Never pass `dbDir` itself or a directory inside it. The wallet's
wipe deletes every entry of `dbDir`, and a Tor state tree under it would be
deleted while arti still has it open, rewritten, and left behind after a wipe
reported as successful.

`torDir` must be an absolute path. An empty, relative or uncreatable one is
refused with `invalidDataDir`.

## Exclude both directories from backup

The files under `torDir` are arti's own formats, unencrypted: `state/`
records this device's Tor guards for months, which makes it a per-device
identifier, and `cache/` holds the Tor directory and, with bridges, the
bridge descriptors. Exclude `torDir` from device and cloud backup exactly as
you exclude the wallet's `dbDir`: `allowBackup="false"` (or a no-backup
directory) on Android, `isExcludedFromBackup` on iOS. Neither the SDK nor
this package enforces it; on Android's default settings the guard state
would be backed up to the user's cloud account.

## Wiping

The order is:

1. `ZecWalletTor.dispose()`: stops Tor and unregisters from the wallet.
2. The wallet's wipe.
3. `ZecWalletTor.clearState(torDir)`, if the user also wants a new Tor
   identity. A wallet wipe on its own leaves the guard state, because it is
   not wallet data.

`clearState` removes only the two subtrees the plugin creates
(`<torDir>/state` and `<torDir>/cache`) and then `torDir` if it is left
empty, so a wrong path loses nothing else. It works after a restart with no
`init`, and it is refused (`notInitialized`) while Tor is running.

`dispose` waits at most five seconds, so the Tor client can still be
shutting down when it returns, and it writes its state as it shuts down.
Each client runs Tor's background work on its own runtime, and that runtime
is shut down when the client is retired, so the shutdown includes everything
Tor started for it. `clearState` waits for that to finish (up to
`clearStateWaitMax`, 15 seconds) before it deletes anything, and throws
`stopping` if it is still running then. When `clearState` returns, the state
is gone and no Tor client in the app is still running. Do not call `init`
until the wipe has finished. `init` waits the same way, so a new client never
shares a directory with one that is still shutting down.

If a client does not finish shutting down within 10 seconds, the plugin
keeps refusing until the app restarts: `clearState`, `init` and `setBridges`
throw `restartRequired`. In a wipe flow, tell the user the reset finishes
after a restart.

A clear refused with `stopping` or `restartRequired` is recorded in `torDir`.
The next `init` deletes the old state before Tor starts, even if the host
does not ask again, and even if the app was killed in between. A host cannot
withdraw a reset once it has asked for it. If the plugin cannot record the
reset (a full disk, say), `clearState` throws `invalidDataDir` instead:
nothing was deleted and nothing will be.

## `required` or `preferred`

This package brings arti and roughly 150 crates it depends on into your app's
process. By its interface it is handed only the wallet's TLS ciphertext (TLS
is the wallet's own, end to end), and it sees where the wallet connects,
when, and how much, and it can refuse or stall a connection. Like any code
in your process, a compromised dependency could read anything in memory,
including the seed while the wallet is open, and its build script runs on
your build machine. Under `TorPolicy.preferred`, a compromised crate could choose
which connections fall back to a direct one. **If you cannot vet this
package's dependency graph yourself, use `TorPolicy.required_`**, which never
falls back.

`preferred` also means this, whoever is at fault: on a network that blocks or
breaks Tor (a censor, or the first Tor relay itself), the plugin eventually
reports Tor as failed (a bootstrap that runs out of time, or a ready client
whose connections keep failing), and after the wallet's patience minute a
`preferred` wallet switches to a direct connection and says so. That is what
`preferred` is for: connectivity first. If a direct connection must never
happen, use `required_`, which waits and shows the path as unavailable.

## What the network sees

- The wallet's server sees a Tor exit, never the device.
- The first Tor relay (the guard) sees the device's address and timing,
  never where the wallet connects.
- **The local network and the ISP can see that the device uses Tor.** Guard
  connections are recognisable. Bridges make that harder but do not hide it,
  and this build carries no pluggable transports: a bridge line that names
  one is refused (`TorFailureClass.bridgePtUnsupported`).
- Onion service endpoints are not supported.
- While the app is paused, arti does no directory work.

## Building

The Rust crate compiles from source; there are no prebuilt binaries. You need
a Rust toolchain on the build machine, and [Cargokit] wires the build into
`flutter build` / `flutter run` for Android, iOS, macOS, Linux and Windows.
Windows is not yet built or run; support is planned after 0.0.1. The plugin
builds on Linux, but `zec_wallet` has no Linux key store adapter yet, so
creating or opening a wallet there fails with `vaultAbsent`. A wallet runs on
Android, iOS and macOS.
The crate under `rust/` is its own cargo workspace with its own
`Cargo.lock`. The first build compiles arti and takes several minutes; it
also makes your app larger.

On Android and Linux the Dart side opens `libzec_wallet_tor.so`, on Windows
`zec_wallet_tor.dll`, and on iOS and macOS it looks the exports up in the
process. Under `use_frameworks!` the plugin is its own framework; under
`:linkage => :static` it is linked into the app.

### Static linkage and dead-stripping (iOS, macOS)

The pods link the plugin's archive plainly (never `-force_load`: two
force-loaded Rust archives in one image fail with thousands of duplicate
symbols) and keep each of the plugin's nine exports with `-u`, because
nothing in the app references them at link time and dead-stripping would
remove them.

The plugin finds the wallet's three host-dialer verbs
(`zec_wallet_register_net_dialer`, `zec_wallet_update_net_dialer`,
`zec_wallet_net_dialer_notify`) by name at runtime. Nothing references them
at link time either. Under `use_frameworks!` (Flutter's default, and this
repository's example) the wallet's framework exports them and nothing more is
needed. **A host that links statically (`use_frameworks! :linkage =>
:static`, or no `use_frameworks!`) and turns on dead-stripping must keep
those three symbols in its app link**, for example with
`-Wl,-u,_zec_wallet_register_net_dialer` (and the other two) in the Runner's
`OTHER_LDFLAGS`. Without them `init` fails with `walletNotLoaded` even though
the wallet is in the app.

## Independence and trademarks

`zec_wallet_tor` is an independent, open-source project. It is not
affiliated with, endorsed by, or sponsored by the Zcash Foundation or the Tor
Project. "Zcash" is a trademark of the Zcash Foundation; it is used here only
to describe the cryptocurrency the wallet works with. "Tor" is a trademark of
The Tor Project, Inc.; it is used here only to describe the network this
package connects to.

## License

MIT; see `LICENSE`. The Rust crate under `rust/` is MIT too.

[Cargokit]: https://github.com/irondash/cargokit
