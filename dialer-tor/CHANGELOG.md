# Changelog — dialer-tor

## 0.1.1

- `TorDialer::spawn_owned` builds a client on a runtime of its own and
  returns an `OwnedDialer`. Its `shutdown(budget)` ends every task the client
  started and answers `Stopped` or `Overran`; `start_shutdown(budget)` starts
  it without waiting. The shutdown uses the first call's budget. If it is
  still running when that budget ends, it counts as `Overran`, awaited or
  not, and every caller gets the same answer. Use it where stopping Tor must
  really stop it, or where the state directory is deleted to reset the Tor
  identity: dropping a `TorDialer` leaves arti's background work running. See
  "The owned client" in `README.md`.
- `ClientLedger` counts the owned clients that may still write their state,
  the ones whose shutdown overran (`stuck()`), and the shutdowns still under
  way (`stopping()`). `spawn_owned` first waits for those shutdowns to
  finish (at most the longest budget plus `SHUTDOWN_WAIT_GRACE`), then
  refuses while a client is stuck, so a new client never runs beside an old
  one.
- `TorDialError` gains `Closed` (class `closed`) and `RestartRequired` (class
  `restart-required`). The enum is `#[non_exhaustive]`, so a consumer's
  wildcard arm still compiles, but a wildcard that maps to a failure is wrong
  for these two: name both, and map them to a local, retryable state, never
  to a relay's failure.
- `Setup` has two new temporary causes on an owned client: no thread for its
  runtime (`kind` `LocalResourceExhausted`) and a task that panicked on it
  (`Internal`). Classify `Setup` by its `kind`, not as permanent.
- New constants: `OWNED_WORKER_THREADS`, `OWNED_MAX_BLOCKING_THREADS`,
  `RECOMMENDED_SHUTDOWN_BUDGET` (10 s, not yet measured on a device),
  `SHUTDOWN_WAIT_GRACE` (2 s: how much
  longer than its budget `shutdown` waits before answering `Overran`; the
  wait is a tokio timer, so await `shutdown` inside a runtime with timers
  enabled, or use `start_shutdown`).
- `OwnedOptions` is `#[non_exhaustive]`: start from `OwnedOptions::default()`
  and set the fields you need.
- Each owned shutdown logs one line with no fields: `info` when it stopped,
  `warn` when it overran.
- `TorDialer` itself is unchanged.

## 0.1.0

First release. A standalone Tor dialer over arti 0.45.0: `host:port` to a
duplex byte stream, per-key circuit isolation, bounded and classified bridge
lines, and no clearnet path of any kind (see `README.md`).

- `TorDialer::unbootstrapped` / `with_bridges` / `from_config`, `bootstrap`,
  `readiness`, `connect` / `dial`.
- `TorDialError` with a closed, PII-free `class()` per variant; the timeout and
  network failures are five distinct variants (`TorNetworkTimeout`,
  `ExitTimeout`, `RemoteNetworkTimeout`, `LocalNetworkError`,
  `TorAccessFailed`), each mapped 1:1 from arti's kind, so a consumer with a
  clearnet fallback can follow only the failures the device or the Tor network
  produces.
- `set_dormant(DormantMode)` / `dormant_mode()`, a passthrough to arti's
  dormancy.
- A bad exit is left, not reused (ADR-0567): when a dial fails at or near
  the exit (the exit could not resolve or reach the destination, timed out,
  refused, was busy), that isolation key's group — the unkeyed one included
  — gets a fresh token, so the next dial cannot join the circuit that
  failed. arti 0.45 alone keeps handing that circuit to the group for up to
  ten minutes. Bounded per key: at most once per `EXIT_ROTATION_INTERVAL`
  (60 s), backing off to `EXIT_ROTATION_INTERVAL_MAX` (600 s). The failed
  dial is not retried. `TorDialer::exit_rotation(ExitRotation)` sets it
  (`ExitRotation::default()`, `every(Duration)` with a 10 s floor, `off()`).
- Feature `static-sqlite` (off): vendors sqlite for arti's directory cache on
  targets with no platform sqlite to link (Android).
- `.onion` targets and pluggable transports are refused (see `README.md`).

The code began as the Tor transport of an earlier project by the same authors;
this is its first release as its own crate.
