# zec-wallet

A ZEC (Zcash) wallet SDK for Flutter with a Rust core. The core is built on
[librustzcash] components and reaches Dart through [flutter_rust_bridge].
Seeds and spending keys stay in Rust; Dart sees addresses, balances, statuses
and typed errors.

**Status: pre-release (0.0.1).** The SDK holds and spends real funds and has
not had an independent security audit. See [SECURITY.md](SECURITY.md).

## What is in this repository

| path | what it is | published as |
|---|---|---|
| [`zec_wallet/`](zec_wallet) | the SDK: the Rust bridge crate, the generated Dart API, and the example app (`zec_wallet/example`) | [`zec_wallet`](https://pub.dev/packages/zec_wallet) on pub.dev |
| [`zec_wallet_ui/`](zec_wallet_ui) | the wallet screens (onboarding, balance, send, receive, history, shielding, swap, recovery), embeddable in a host app | [`zec_wallet_ui`](https://pub.dev/packages/zec_wallet_ui) |
| [`zec_wallet_ui_platform/`](zec_wallet_ui_platform) | an optional native companion: screenshot protection, backup exclusion, network reachability | [`zec_wallet_ui_platform`](https://pub.dev/packages/zec_wallet_ui_platform) |
| [`zec_wallet_tor/`](zec_wallet_tor) | optional Tor for the wallet's traffic; an app that does not add it carries no Tor code | [`zec_wallet_tor`](https://pub.dev/packages/zec_wallet_tor) |
| [`dialer-tor/`](dialer-tor) | a standalone Tor dialer over arti, used by `zec_wallet_tor` | [`dialer-tor`](https://crates.io/crates/dialer-tor) on crates.io |
| `zec-wallet-core/`, `zec-wallet-swap-near/`, `apple-secure-enclave/` | the Rust crates behind `zec_wallet`: the wallet core, the NEAR swap adapter, the Secure Enclave key store | vendored into the `zec_wallet` package |
| [`docs/`](docs) | the SDK's specs and architecture decisions, at the paths the code cites | — |

Each package has its own README with the integration steps, and its own
CHANGELOG.

## Building

The Rust core compiles from source; there are no prebuilt binaries. You need
a Rust toolchain installed with rustup, **Rust 1.96 or newer** (the
`rust-version` of every workspace here). [Cargokit] runs the Rust build from
`flutter build` and `flutter run`.

There are three cargo workspaces, each with its own `Cargo.lock` and
`deny.toml`: the root (the wallet core, the swap adapter, the Secure Enclave
crate and the bridge), `dialer-tor/`, and `zec_wallet_tor/rust/`. The Tor
crates live apart because arti's dependency versions and the Zcash stack's
cannot share one lock.

```sh
cargo test --workspace                     # the wallet core and the bridge
(cd dialer-tor && cargo test)              # the Tor dialer
(cd zec_wallet_tor/rust && cargo test)     # the Tor plugin's native library
(cd zec_wallet && flutter test)            # likewise in each Flutter package
```

[`.github/workflows/ci.yml`](.github/workflows/ci.yml) lists every check a
change has to pass.

## Platforms

Android and iOS are tested on phones and macOS on a development machine. In
0.0.1 a wallet runs on those three.

**Linux and Windows cannot hold a wallet yet.** The packages build there (Linux
arm64 is compiled and its tests run; Windows has not been built), but the
wallet keeps its keys in the platform's key store, and there is no desktop key
store adapter yet. Creating or opening a wallet on Linux or Windows fails with
`vaultAbsent`: no wallet is created and no key is stored, though the data
directory and its lock file may be left behind. A desktop key store is planned
for a later release.

## Known limitations

- **A light server can lower recorded subtree heights after a rewind.** After
  a chain reorganisation the wallet rewinds a fixed distance and takes new
  completion heights from the server. It refuses an upward change, but
  accepts a downward one larger than any reorg of that depth could cause. The
  fix needs the actual fork point, and the wallet cannot find it yet.
  A server that does this can make the wallet misjudge which funds are
  spendable: a note can be offered before it is safe to spend, or funds can
  look stuck. The server cannot take funds. Until this is fixed, 0.0.1 is not ready to
  hold large amounts; use it with small amounts and a light server you trust.
  [Issue #1](https://github.com/alvorado-lab/zec-wallet/issues/1) tracks it,
  and CI skips its test by name until then.

## Design record

[`docs/`](docs) carries the SDK's specs and its architecture decisions
(`docs/adr/`), so a comment such as `docs/specs/wallet-sdk.md §6.3` or
`ADR-0552` resolves in this repository. [`docs/README.md`](docs/README.md)
explains which references point outside it.

## Security

Please report vulnerabilities privately to **security@relim.io**, not in a
public issue. [SECURITY.md](SECURITY.md) has the details.

## Trademark notice

zec-wallet is an independent, open-source project. It is not affiliated
with, endorsed by, or sponsored by the Zcash Foundation. "Zcash" is a
trademark of the Zcash Foundation; it is used here only to describe the
cryptocurrency this library works with. This project is unrelated to
ZecWallet, the discontinued Zcash wallet application.

## License

MIT; see [LICENSE](LICENSE).

[librustzcash]: https://github.com/zcash/librustzcash
[flutter_rust_bridge]: https://github.com/fzyzcjy/flutter_rust_bridge
[Cargokit]: https://github.com/irondash/cargokit
