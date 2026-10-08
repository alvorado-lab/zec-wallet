# dialer-tor

A standalone Tor **dialer** over [arti](https://gitlab.torproject.org/tpo/core/arti):
give it `host:port`, get a duplex byte stream that left your machine inside a
Tor circuit. It depends on no other crate of this repository and knows nothing
about the products that use it. Its first consumer is the Zcash wallet SDK's
optional Tor plugin, `zec_wallet_tor`.

**Where it lives, and why here.** `dialer-tor/` in the zec-wallet
repository, as its own cargo workspace — it cannot join the SDK's workspace,
because arti's RustCrypto line (`crypto-common ^0.2.2`) and the Zcash stack's
pre-release pins (`=0.2.0-rc.1`) cannot share one lock. Its lock and
`deny.toml` are its own.

## The API

```rust
use dialer_tor::{BridgeLines, DormantMode, Readiness, TorDialError, TorDialer};

// Build WITHOUT bootstrapping; no network traffic yet.
let dialer = TorDialer::unbootstrapped(state_dir, cache_dir).await?;
//   or: TorDialer::with_bridges(state_dir, cache_dir, BridgeLines::parse(pasted)?).await?
//   or: TorDialer::from_config(dialer_tor::tor_config(state_dir, cache_dir, lines)?).await?

dialer.bootstrap().await?;              // slow, fallible, idempotent; fails closed
match dialer.readiness() {              // a query, no traffic
    Readiness::Ready => {}
    Readiness::Bootstrapping { progress, blockage } => { /* 0.0..=1.0, arti's BlockageKind */ }
}

let stream = dialer.connect("example.com", 443, Some("my-isolation-key")).await?; // TorStream
let boxed  = dialer.dial("example.com", 443, None).await?;                         // Box<dyn AsyncByteStream>

dialer.set_dormant(DormantMode::Soft);  // pause periodic work (see "Dormancy")
assert_eq!(dialer.dormant_mode(), DormantMode::Soft);
```

- **`TorDialer::unbootstrapped` / `with_bridges` / `from_config`** build a
  client with `BootstrapBehavior::Manual`: until `bootstrap()` succeeds, every
  dial returns `TorDialError::NotBootstrapped` — the crate has NO clearnet
  path and never connects some other way.
- **`bootstrap()`** — slow, fallible, idempotent. On failure the dialer stays
  refusing; `readiness()` reports arti's blockage (`Offline`, `Filtering`,
  `CantReachTor`, `ClockSkewed`, `CantBootstrap`, `Disabled`).
- **`readiness()`** — `Ready` iff arti's `ready_for_traffic()`; otherwise
  `Bootstrapping { progress, blockage }`. A query; no traffic.
- **`connect(host, port, isolation_key)`** → `TorStream`, or
  **`dial(...)`** → `Box<dyn AsyncByteStream>` (the shape the wallet SDK's
  `NetDialer` port declares). Equal keys may share a circuit; distinct keys
  are put in distinct circuit-isolation groups; `None` joins the dialer's
  own unkeyed group that no key can join. At most `MAX_ISOLATION_KEYS` (512)
  distinct keys are tracked; past that the registry resets in the safe
  direction (fresh isolation, never a shared circuit). **A bad exit is left,
  not reused:** when a dial fails at or near the exit (it could not resolve
  or reach the destination, timed out, refused, was busy), that key's group
  — the unkeyed one too — gets a fresh token, so the next dial cannot join
  the circuit that failed (arti alone would reuse it for up to 600 s). At
  most once per key per `EXIT_ROTATION_INTERVAL` (60 s), doubling to
  `EXIT_ROTATION_INTERVAL_MAX` (600 s) while failures continue; the failed
  dial is not retried. `TorDialer::exit_rotation(ExitRotation::off())`
  keeps arti's behaviour; `ExitRotation::every(d)` sets the base wait (10 s
  floor). Targets are checked
  before a circuit is spent: an empty or over-long host, port 0, a `.onion`
  name (refused — see the licence carve-out), a local address (arti's own
  refusal, `ForbiddenTarget`).
- **`set_dormant(DormantMode)` / `dormant_mode()`** — see "Dormancy".
- **Bridges.** `BridgeLines::parse(&str)` takes pasted bridge lines, bounded
  and classified BEFORE arti sees them: at most `MAX_BRIDGE_LINES` (8) lines
  of `MAX_BRIDGE_LINE_BYTES` (256) each, `MAX_BRIDGE_CONFIG_BYTES` (2048) in
  all; a refusal is one of the `CLASS_*` labels (`bridge-config-too-long`,
  `bridge-too-many-lines`, `bridge-line-too-long`,
  `bridge-pluggable-transport-unsupported`, `bridge-line-unusable`;
  `bridge_class(&err)` maps arti's own parse error to the last two) and
  NEVER echoes the line — `BridgeLines` is deliberately not `Debug`,
  `Display` or `Clone`; `len()` / `is_empty()` are its only readers.
  `tor_config(state, cache, lines)` builds the arti config a caller (or a
  test) can inspect.
- **Re-exports** so a consumer never mirrors arti's types: `arti_client`,
  `TorClientConfig`, `ErrorKind`, `DormantMode`, `status::BlockageKind`.

## Errors, their classes, and the clearnet rule

Every refusal is a `TorDialError`; `class()` is the short, stable, PII-free
label to log and count (the `Display` text is for a human and may name the
target). The set is closed; a variant added upstream arrives as
`Tor { kind }` with arti's own discriminant, never merged into a neighbour.

| variant | `class()` | who can cause it |
|---|---|---|
| `NotBootstrapped { progress, blockage }` | `not-bootstrapped` | the bootstrap has not finished |
| `BootstrapFailed { kind, blockage }` | `bootstrap-failed` | the network, a censor |
| `OnionUnsupported` | `onion-unsupported` | the caller (a `.onion` target) |
| `ForbiddenTarget` | `forbidden-target` | the caller (a local address) |
| `BadTarget { problem }` | `bad-target` | the caller |
| `RefusedByExitPolicy` | `refused-by-exit-policy` | the exit |
| `RefusedByHost` | `refused-by-host` | the destination |
| `HostNotFound` | `host-not-found` | DNS at the exit |
| `TorNetworkTimeout` | `tor-network-timeout` | a circuit BUILD timed out — a relay on the path; a stalling exit contributes; the destination cannot |
| `ExitTimeout` | `exit-timeout` | the exit waited too long for the destination |
| `RemoteNetworkTimeout` | `remote-network-timeout` | our timeout waiting on the far end |
| `LocalNetworkError` | `local-network-error` | an IO error on the open GUARD channel — the device's network, the guard, or an on-path reset; **not** "the device has no route" (that is `TorAccessFailed`) |
| `TorAccessFailed` | `tor-access-failed` | Tor could not be reached: the local network, or the chosen relay or bridge, is not working |
| `Setup { kind }` | `setup` | the configuration or the client could not be built; for an owned client also two TEMPORARY causes: no thread for its runtime (`kind` `LocalResourceExhausted`) or a task that panicked on it (`Internal`). Classify `Setup` by its `kind`, never as permanent as a whole |
| `Tor { kind }` | `tor` | anything arti reports that is not listed above |
| `Closed` | `closed` | this owned client was shut down (see "The owned client") |
| `RestartRequired` | `restart-required` | `spawn_owned` refused: an earlier client never finished shutting down (while `ledger.stuck()` is 0 it may still finish, so a later attempt can succeed; once it is not, only a restart helps) |

**The rule for a consumer with a clearnet fallback** (the wallet SDK's
`Preferred` policy follows exactly two codes to clearnet — *unreachable* and
*timeout* — plus a registered dialer declaring its health `FAILED` for a
patience minute): a failure the DESTINATION or an EXIT can cause at will must never
reach those codes, and in arti 0.45.0 `LocalNetworkError` is produced by a
guard-channel reset — an on-path adversary — while a device with no route
produces `TorAccessFailed`. So the SDK's plugin maps ONLY `TorNetworkTimeout`
to its timeout code, maps `LocalNetworkError`, `TorAccessFailed`,
`ExitTimeout`, `RemoteNetworkTimeout` and every refusal to REFUSED (no
fallback), and derives "the device is offline" itself, never from a variant
of this crate. A consumer with no fallback needs only the classes.

The five timeout/network variants split what an earlier version of this code
reported as two (`Timeout`, `NoNetwork`); the split is what makes the rule
expressible.

## Dormancy

`set_dormant(DormantMode::Soft)` is a passthrough to arti's
`TorClient::set_dormant`: periodic background work (directory refreshes,
guard maintenance) stops; channels and sockets stay as they are. Two things
to know: arti flips `Soft` back to `Normal` by itself on ANY use of the
client, so a consumer that dials or even reads `readiness()` while "dormant"
has woken it — `dormant_mode()` reports what was last ASKED, not arti's live
state; and `Soft` switches channel padding off, which the guard can observe.
A consumer that needs "no dials while paused" gates that itself (the SDK's
plugin pushes readiness 0 on pause); dormancy is the battery half.

## The owned client: a lifetime you can end

Dropping a `TorDialer` does not stop Tor. arti runs its background work on
the runtime the client was built on, and some of it keeps the client's
internals alive while a directory download is pending, which on a blocked
network can be indefinitely. arti also writes its guard state when the last
reference to those internals drops. Two things follow: an "Off" switch that
only drops the dialer leaves Tor talking, and deleting the state directory to
reset the device's Tor identity can be undone by a late write.

`TorDialer::spawn_owned` solves both. It builds the client on a runtime of
its own and counts it in a `ClientLedger` the caller holds:

```rust
use std::sync::Arc;
use dialer_tor::{ClientLedger, OwnedOptions, Shutdown, TorDialer, RECOMMENDED_SHUTDOWN_BUDGET};

let ledger = Arc::new(ClientLedger::default());    // one per process
let dialer = TorDialer::spawn_owned(config, &ledger, OwnedOptions::default()).await?;
dialer.bootstrap().await?;
let stream = dialer.connect("example.com", 443, None).await?; // OwnedStream

match dialer.shutdown(RECOMMENDED_SHUTDOWN_BUDGET).await {
    Shutdown::Stopped => { /* nothing of this client runs or writes; safe to delete its state */ }
    Shutdown::Overran => { /* it may still write until the process exits; do not delete yet */ }
}
```

- **`shutdown(budget)`** ends every task the client started, drops the client
  inside that time, and answers `Stopped` or `Overran`. It works through any
  clone: the first call starts it, every caller gets the same answer, and a
  dropped future loses nothing. The shutdown uses the first call's budget, and
  a later caller's wait uses it too, not its own. The wait is bounded:
  past that budget plus `SHUTDOWN_WAIT_GRACE` it answers `Overran`, so await
  it inside a tokio runtime with timers enabled. **`start_shutdown(budget)`**
  starts it without waiting, for code that cannot await or has no timer.
- **After a shutdown** the client's own calls (`bootstrap`, `connect`,
  `dial`, `readiness`, `set_dormant`, `dormant_mode`) answer
  `TorDialError::Closed`, a call in flight ends with `Closed`, and an
  `OwnedStream` answers `NotConnected` without touching Tor, including a read
  or write that was already waiting. Treat `Closed` as a local, retryable
  state: it says nothing about any relay.
- **`Overran`** means the shutdown did not finish within the budget: the
  client may still write its state, and may still be talking to the Tor
  network, until the process exits. It stays counted in `ledger.stuck()`, and
  `spawn_owned` refuses with `RestartRequired` until the app restarts, so a
  second client never shares the first one's state directory. Do not report
  Tor as off after `Overran`. A shutdown still running when its budget ends
  counts as `Overran` at that moment, even if nothing awaits it, and
  `ledger.stuck()` already shows it by the time any caller gets the answer.
  A stream still being read or written inside Tor when the budget ends may
  then fail, or panic on its own polling thread.
- **A new client** waits for any shutdown still under way in its ledger
  (`ledger.stopping()`) before it starts, and is refused if that shutdown
  overran. So switching Tor off and on again, or resetting it, needs no
  extra wait from the host. The wait lasts at most the longest budget
  handed off plus `SHUTDOWN_WAIT_GRACE`; a shutdown still unfinished then
  is answered `RestartRequired` as well.
- `RECOMMENDED_SHUTDOWN_BUDGET` (10 s) is a margin over a shutdown that
  normally takes milliseconds. It has not yet been measured on a device.
- **Dropping** the last clone without a shutdown runs the same shutdown in the
  background, bounded by `OwnedOptions::drop_budget`. Its result reaches only
  the ledger.
- Cancelling a call (a timeout, an aborted task) aborts its work on the
  client's runtime, as cancelling a plain `TorDialer` call would.
- Each live client costs one worker thread and up to two blocking threads.
  Poll its streams inside a tokio context with timers enabled, as for
  `TorDialer`.

## Logging

`tracing` only. Targets: the crate's module paths, plus `dialer_tor::censor`
for a refused bridge line (class only). Never logged: bridge lines, hosts,
isolation keys, paths, arti's `Display` on the bridge-line path. Every log
site carries a closed value (`class`, `kind`, a mode) — see `error.rs` and
`dialer.rs`. A rotation off a suspect circuit logs one `debug` line with the
`kind` only, never the key. Each owned client's shutdown logs one line with
no fields: `info` when it stopped, `warn` when it overran.

## Features

- `static-sqlite` (off): arti's directory cache is a `rusqlite` database and,
  by default, links the PLATFORM's sqlite; a target with none to link
  (Android's NDK) turns this on to vendor sqlite into the artifact. arti
  marks the underlying feature non-additive, so the FINAL consumer decides.

## What is deliberately absent

- **Onion services** (`.onion` targets are refused): arti's onion-service
  features reach `equix`/`hashx`, LGPL-3.0-only crates that cannot ship
  inside a third-party product. `tests/carve_out.rs` and `deny.toml` keep
  them out; a `.onion` target is `OnionUnsupported`.
- **Pluggable transports** (`pt-client`): an obfs4 line is refused with the
  class `bridge-pluggable-transport-unsupported`.
- **A clearnet path of any kind.**

## Supply chain

`deny.toml` in this directory is the policy over this crate's lock: ring-only
TLS (`aws-lc-*` banned), the rustls advisory floor, the LGPL bans, and three
named advisory ignores with their arguments and expiry conditions (`rsa`
0.9.10's Marvin attack — no RSA private-key operation is reachable from a Tor
client; two unmaintained transitive helpers). `cargo audit --ignore
RUSTSEC-2023-0071` and `cargo deny check` run over it in CI.

## Licence

MIT, like the rest of the SDK — see `LICENSE`.
