# Changelog

## 0.0.6

A security release. Update: the plugin's lock moves `url` to 2.5.9 and
`form_urlencoded` to 1.2.3, which fixes RUSTSEC-2026-0592 (a URL with many
nested `blob:` prefixes overflows the stack in `Url::origin()`). The plugin
never reached it: in its graph only arti's `tor-chanmgr` uses `url`, to parse
proxy URLs it builds itself, and it never calls `origin()`. RUSTSEC-2026-0459
(SQLite before 3.54.0 and a crafted `-wal` file) stays open for arti's
directory cache until an arti release takes the fix; the attack needs a
crafted file placed in the plugin's Tor directory on the device. The lock also
moves `rustix` to 1.1.6 for RUSTSEC-2026-0486; the plugin reaches rustix only
through `tempfile` and nothing in its graph calls the extended-attribute
functions. A few build-time and Windows-only edges move among versions already
in the lock. `deny.toml` also ignores RUSTSEC-2026-0542: `humantime`, which
arti uses for durations and timestamps, is unmaintained, not vulnerable. The
plugin's API and behaviour do not change. `flutter pub upgrade zec_wallet_tor` takes
it, since `^0.0.5` also accepts 0.0.6. The README's install snippet now names
`zec_wallet` 0.0.7.

## 0.0.5

**Android builds work with Gradle 9.** Apps created with current Flutter could
not build 0.0.4 or earlier for Android. Flutter 3.44 sets new apps up with
Gradle 9.1, and Gradle 9 removed a call the plugin's
`cargokit/gradle/plugin.gradle` made, so the build stopped with "Could not find
method exec()". 0.0.5 builds on Gradle 8 and 9; the plugin's API and behaviour
do not change. `flutter pub upgrade zec_wallet_tor` takes it, since `^0.0.4`
also accepts 0.0.5. The README's install snippet now names `zec_wallet` 0.0.6.

## 0.0.4

A security release. Update: the plugin's lock moves to `zstd` 0.14.3,
`zstd-safe` 8.2.0 and `zstd-sys` 2.1.2, which fix RUSTSEC-2026-0391, -0395 to
-0406 and -0410. arti uses zstd to decompress the Tor directory documents it
downloads, so the decoder sees data from the network. The plugin's API and
behaviour do not change.

## 0.0.3

- Built on `dialer-tor` 0.1.3. The plugin's lock moves to `rustls` 0.23.46
  and `rustls-webpki` 0.103.16, the versions `zec_wallet` 0.0.4 uses.
  Nothing changes for the plugin's API or behaviour.
- The plugin's own connections to Tor relays were not exposed to the advisories
  `zec_wallet` 0.0.4 fixes: arti checks a relay by its Tor identity key, not by
  web certificate roots, and turns TLS session resumption off. The wallet's
  traffic over Tor still runs its own TLS inside `zec_wallet`, so update that
  package too.
- The plugin's lock moves to `tokio` 1.53.3 (RUSTSEC-2026-0386). Nothing in its
  graph builds an `AsyncFd`, so the plugin was not exposed.
- Version 0.0.2 was prepared on `dialer-tor` 0.1.2 and never published.

## 0.0.1

Initial release. An arti-based Tor client (the shared `dialer-tor` crate) in
its own native library that registers itself with `zec_wallet` through the
wallet's host-dialer contract (`zec_wallet_net_dialer.h`, ABI v4).

- Dart surface `ZecWalletTor`: `init`, `status`, `statusStream`,
  `setBridges`, `retryBootstrap`, `clearState`, `dispose`. Errors are
  `TorPluginError` with a closed `TorPluginErrorKind`; the status is four
  closed codes (`TorPluginPhase`, readiness, `TorBlockage`,
  `TorFailureClass`). No string crosses the C ABI.
- The Dart side refuses a native library from another release before any
  other call (`pluginAbiMismatch`).
- The plugin follows the app lifecycle itself: quiet on pause, awake (or
  rebuilt after a long pause) on resume.
- Each Tor client runs on its own runtime, shut down when the client is
  retired, so its background tasks end with it (`dialer-tor` 0.1.1's owned
  client). A rebuild (new bridges, a resume after a long pause) starts the
  new client only once the old one has stopped, so two never share the state
  directory. `clearState` waits for that
  shutdown (up to 15 seconds) before it deletes anything, so the identity
  reset is not undone by a late state write. `init` waits the same way and
  throws `stopping` if the old client is still running. A shutdown that
  overruns its bound makes them and `setBridges` throw `restartRequired`
  until the app restarts. A clear refused with `stopping` or
  `restartRequired` is recorded and finished by the next `init`; one that
  cannot be recorded throws `invalidDataDir` instead.
- Update the plugin's Dart package and its native library together: an older
  Dart side reads a return code it does not know (such as `stopping`) as
  `unknown`.
- Bridge lines cross as a byte span valid for the call only; the native copy
  is zeroed before it is freed.
