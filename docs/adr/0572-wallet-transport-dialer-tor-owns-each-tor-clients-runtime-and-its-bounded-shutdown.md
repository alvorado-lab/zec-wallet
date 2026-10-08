# 0572 — dialer-tor owns each Tor client's runtime and its bounded shutdown

- **Status:** Proposed
- **Date:** 2026-10-07
- **Links:** docs/specs/tor-plugin.md §12 (the spec patch) · §3.3 "Threading" ·
  docs/plan/audit-2026-10-05-fixes.md §5–§7 · ADR-0548 · ADR-0550 · ADR-0551 ·
  ADR-0567

## Context

arti binds the runtime a client is built on and spawns its background tasks
there; while a directory is pending, some of them hold the circuit and
directory managers across an `.await`. Dropping the client does not end them.
For a host this means two things: a Tor identity reset can be undone by the
client's late state write, and an "Off" does not stop Tor traffic on a
censored network.

The `zec_wallet_tor` plugin fixed the first for itself (a runtime per client,
shut down on retire: audit-fix plan §5), but its rebuild still mints a new client
without waiting for the old one's shutdown, so two can share `state/` for up
to ten seconds. Relim builds its client through `dialer-tor` on its one
app-wide runtime and has the same problem, including the second. Its Off has
no single owner of the client: the dialer sits in its relay client's
connection pair, and in-flight requests hold their own references.

arti also writes its guard state in a `Drop` (`CircMgrInner::drop`), on
whatever thread drops the last reference, so the design must control where
that last reference goes, not only when the tasks end.

## Decision

`dialer-tor` owns each client's lifetime. `TorDialer::spawn_owned` builds the
client on a dedicated runtime of its own and counts it in a host-held
`ClientLedger`, and refuses a new client while one is stuck;
`OwnedDialer::start_shutdown` hands the shutdown off synchronously and
`shutdown(&self, budget)` awaits it, ending every task the client started,
bounded, and reporting `Stopped` or `Overran`. No reference to the client ever
leaves the owned runtime or the shutdown thread; every async call runs on the
owned runtime and is aborted when its caller gives up; returned streams sit
behind a poll gate. Hosts keep everything on disk (the reset record, the
deletion) and the user-facing answers.

## Alternatives considered

- **Keep the fix in each host.** Relim would copy subtle lifetime code; two
  copies of one rule.
- **One process-wide Tor runtime with per-client task scopes.** tokio cannot
  shut down a subset of a runtime's tasks, and arti offers no scope hook; a
  rebuild also needs two clients' runtimes to coexist.
- **Cancel the bootstrap and set dormant.** arti 0.45 has no close verb, and
  dormancy does not end the pending-directory loop.
- **A consuming `shutdown(self)`, or waiting for other holders.** Off would be
  bounded by the longest request, which breaks honest-off.

## Consequences

- Easier: honest-off for Relim; one home for the lifetime rule; the plugin's
  rebuild can await the old client before minting, closing the overlap the
  pre-publication review deferred.
- Harder: one runtime (one worker, up to two blocking threads) per live client;
  one task spawn per async call.
- Frozen: `Shutdown` has exactly two outcomes; `Overran` is permanent for the
  process, and `spawn_owned` refuses a new client while `stuck() > 0`.
- Versioning: `dialer-tor` 0.1.1, additive for consumers (`TorDialError` is
  `#[non_exhaustive]`; it gains `Closed` and `RestartRequired`, both local
  facts a host maps to a retryable not-ready error). The crate's own class
  table and the plugin's error allowlist and pins change together. tokio's
  features grow (`rt-multi-thread`, `sync`, `time`); no lock entry changes. No
  wallet, bridge or plugin ABI change. The plugin adopts it before its 0.0.1
  publish (founder, 2026-10-07: all four packages wait), so the plugin's own
  copy is never published.
