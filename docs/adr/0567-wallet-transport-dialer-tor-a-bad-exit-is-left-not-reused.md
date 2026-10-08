# 0567 — dialer-tor: a bad exit is left, not reused

- **Status:** Accepted. Decided S314 (2026-10-01) under the maintainer's S310 delegation
  ("do whatever is best practice": decide, record, do not queue it for him). Design
  reviewed twice by security and once by arch before the build
  (`docs/plan/fr54-a-bad-exit-does-not-hold-the-private-path.md` §6); Relim's answer at
  both sync points was "re-pin only" (§7).
- **Date:** 2026-10-01
- **Links:** FR-54 (`docs/handoff/host-feature-requests.md`) ·
  `docs/plan/fr54-a-bad-exit-does-not-hold-the-private-path.md` ·
  `docs/specs/tor-plugin.md` (Isolation) ·
  [ADR-0545](0545-wallet-transport-host-is-trusted-isolation-requested-not-demanded.md)
  (isolation keys and what they promise) ·
  [ADR-0550](0550-wallet-transport-dialer-tor-is-developed-in-this-repository-and-pushed-when-needed.md)
  (`dialer-tor` is built here; this lands in 0.1.0, before its first publish) ·
  [ADR-0552](0552-wallet-transport-preferred-insists-on-the-private-path-for-a-minute-then-switches-visibly.md)
  and [ADR-0553](0553-wallet-transport-a-private-path-that-never-starts-also-switches-after-the-minute.md)
  (the minute, and the FAILED health that switches) ·
  [ADR-0554](0554-wallet-transport-a-ready-path-that-carries-nothing-gets-its-own-state-and-the-sdk-does-not-guess-why.md)
  (a ready path that carries nothing)

## Context

After a cold boot on a Galaxy S25 (Relim's S417 report, FR-54), arti bootstrapped in 25 s,
and then 107 of 107 relay dials over 4.5 minutes went down ONE circuit. Its exit could not
resolve or reach the destination. A fresh circuit sat unused the whole time. The wallet's own
sync over the same dialer failed in that window too.

arti 0.45 does not retire a circuit when its exit reports a stream failure. It keeps handing
the circuit to the same isolation group until `max_dirtiness` (600 s from first use).
`dialer-tor` gave each isolation key one fixed `IsolationToken` for the dialer's life, and
gave every unkeyed dial one fixed token too. Every Relim relay dial is unkeyed. So one bad exit
took the whole private path down for up to ten minutes.

## Decision

**After a dial fails in a way that marks its circuit as suspect, the key's isolation group
gets a fresh token, so its next dial cannot join that circuit.** This covers the unkeyed group
too. A fresh `IsolationToken` is unique to the process and compatible only with itself, so it
joins no other group. It is more isolation, never less. To the network it looks like any new
key's first circuit.

1. **The trigger is arti's `ErrorKind`, read before classification** (`error.rs`,
   `circuit_is_suspect`). `TorDialError` has already lost what this needs:
   `RefusedByExitPolicy` folds the exit's own `ExitPolicyRejected` together with the
   consensus-level `NoExit`, and `RelayTooBusy` falls into the catch-all `Tor { kind }`.
   - **Rotate on:** `RemoteHostResolutionFailed`, `RemoteHostNotFound`, `ExitTimeout`,
     `RemoteNetworkTimeout`, `RemoteNetworkFailed`, `RemoteStreamError`,
     `RemoteConnectionRefused`, `RemoteStreamReset`, `ExitPolicyRejected`, `RelayTooBusy`,
     `RemoteStreamClosed`. Each is something the exit said, or failed to say, about the
     stream request. A different exit may answer differently.
   - **Never on:** `NoExit` (no exit in the consensus allows the port, so a new circuit cannot
     help), `CircuitCollapse` (END `DESTROY`: the circuit is already dead),
     `TorProtocolViolation` (could be any hop), and every bootstrap, directory, configuration
     or local kind.
   - The list is positive. A kind a future arti adds does not rotate until someone places it.
     The name is "suspect", not "exit-attributable", because `RemoteNetworkTimeout` is arti's
     own connect timeout around the stream request. A stall at the guard or a middle relay
     produces it too, and leaving a stalled circuit is still the right move.
2. **The swap is a compare-and-swap.** `connect` keeps the token it dialed with. The registry
   replaces the stored token only while it is still that one. N dials failing on one circuit
   rotate once, and a late failure from an old circuit never throws away a fresh token. A key
   that was cleared at the `MAX_ISOLATION_KEYS` ceiling between the dial and its failure is
   not put back. Putting it back would bypass the ceiling.
3. **It is bounded per key, with backoff.** A destination that does not exist, or a hostile
   exit, can produce these kinds at will. Without a bound, every failed dial would cost a
   circuit build, at whatever rate the caller retries.
   - A key's first rotation is immediate. The next waits `EXIT_ROTATION_INTERVAL` (60 s). Each
     one after that waits twice as long as the last, up to `EXIT_ROTATION_INTERVAL_MAX`
     (600 s, arti's own `max_dirtiness`).
   - The wait goes back to 60 s only after 600 s with no suspect failure on that key. A
     success does not reset it. Relim's unkeyed group carries many destinations, so a dead
     relay and a live one alternate. A success on the fresh circuit shows the failure was the
     destination's, which is a reason to keep backing off. A destination that stays down costs
     a handful of circuits, not one a minute forever.
   - Elapsed time is `now.saturating_duration_since(last)`, never `last + interval`, which can
     overflow `Instant`. `now` is passed in; the registry has no clock.
   - The exact cost: at most one rotation per key per interval for as long as the key's state
     lives. Only the host can reset that state: a ceiling clear drops it, which costs at most
     one extra circuit per key per clear. A remote party cannot reset it.
   - A destination that keeps failing can PIN a shared group at the cap: every suspect
     failure restarts the quiet period, so on a multi-destination group (Relim's unkeyed one)
     a dead or hostile destination holds the group's interval at 600 s, and recovery from a bad
     exit on a healthy destination in that group waits up to 600 s. That is arti's own
     `max_dirtiness` — never worse than without this decision. Closing it would need state per
     destination, i.e. storing hosts: a privacy cost not worth paying.
4. **A host can set it.** `TorDialer::exit_rotation(self, ExitRotation) -> Self` is a consuming
   builder (not `with_…`, which in this crate names the async constructors that return a
   `Result`). `ExitRotation` is a struct with private fields, so no host can build an interval
   past the constructors:
   - `default()`: 60 s, backing off to 600 s;
   - `every(d)`: clamps to a 10 s floor, arti's `connect_timeout`. An interval of zero would
     leave only the compare-and-swap, one build per failed dial;
   - `off()`: arti's behaviour, unchanged.
5. **Logging:** one `debug!` per rotation, naming the kind and never the key. A wallet's
   isolation key can carry a token.

## Alternatives considered

- **Retry the failed dial once on the fresh token** (FR-54's optional ask 3). Rejected. A
  `RemoteNetworkTimeout` already takes about 10 s, and a retry can cross the plugin's 20 s
  `DIAL_DEADLINE`. An elapse counts toward `LIVENESS_FAILED_DIALS`, then health `FAILED`, then
  ADR-0553's switch. A destination could then drive the private path's health. The caller's
  own next dial gets the fresh circuit (the wallet retries on its own clock). Revisit after
  0.1.0 if a measurement shows it is needed.
- **A `max_dirtiness` knob** (FR-54's optional ask 4). Rejected. It churns circuits for every
  group in the client, not only the failing one. `from_config` callers can already set it, and
  `tor_config()`'s callers (the plugin) keep arti's 600 s.
- **Trigger on `TorDialError`.** Rejected (security review M1, arch review MAJOR). It loses
  the exit/consensus line described above, and it would be a second list built from the
  classifier's output that drifts from the first.
- **Rotate on every failed dial, unbounded.** Rejected. A remote party could then make the
  client build circuits at the caller's retry rate.
- **Reset the backoff on a success.** Rejected. On a multi-destination group, a dead
  destination and a live one alternate, so the backoff would never grow.
- **A drop guard that rotates when a dial is cancelled or times out.** Rejected. The plugin's
  20 s timeout drops the `connect` future before its failure arm runs. An elapse is about
  getting a circuit, not an answer from an exit.

## Consequences

- **The next dial after a rotation needs a clean circuit** (m5). Over a slow bridge with none
  pre-built, it can elapse at 20 s, and an elapse counts toward liveness. That is the Tor
  path's real limit, and a destination does not control it. On a dead local network, rotation
  turns uncounted 10 s timeouts into fresh builds that elapse and count. `FAILED` then becomes
  more honest, not less.
- **A cancelled or timed-out dial rotates nothing** (m6), by design (see the drop-guard
  alternative).
- **A new circuit is not guaranteed a new exit relay.** It can pick the same exit by chance.
  It then fails once more and rotates again after the interval.
- **No change to `TorDialError`'s variants, the dial codes, the C dialer header (ABI v4) or the
  bridge ABI.** The plugin includes `error.rs` for its allowlist, and that allowlist is
  untouched. The plugin and Relim re-pin. Relim confirmed its relay pins no session, queue,
  cursor or auth to an exit IP, so a mid-session exit change reads as a request from a new
  address.
- **Proof owed before FR-54 closes:** the live test of the plan's §4.4 on this Mac, both
  polarities, plus the offline unit tests of the bound in `just ci`.
