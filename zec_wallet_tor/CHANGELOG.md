# Changelog

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
