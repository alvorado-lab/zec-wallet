# 0554 — Wallet transport: a READY path that carries nothing gets its own state, and the SDK does not guess WHY

- **Status:** Accepted (built and landed in the same session — stage S1 `truth`,
  S290/S291, `docs/plan/stage-1-private-path-truth.md` §3.2; the rulings are
  `docs/adjudication/s1-truth/ruling.md` and `ruling-2.md`).
- **Date:** 2026-09-20
- **Links:** [ADR-0552](0552-wallet-transport-preferred-insists-on-the-private-path-for-a-minute-then-switches-visibly.md)
  (the founder's minute — the SAME clock, not a second one) ·
  [ADR-0553](0553-wallet-transport-a-private-path-that-never-starts-also-switches-after-the-minute.md) ·
  [ADR-0547](0547-wallet-transport-the-host-names-its-transport-no-predefined-kinds.md)
  (the SDK names no transport of its own) ·
  [ADR-0545](0545-wallet-transport-host-is-trusted-isolation-requested-not-demanded.md)
  (a host's declaration outranks the SDK's evidence) ·
  `docs/specs/host-transport-crossing.md` ·
  `docs/specs/wallet-sdk.md` §2.5 · FR-36, FR-37 in
  `docs/handoff/host-feature-requests.md`

## Context

`TorState::Active` attested READINESS and was read as CARRIAGE. The bridge's own
DTO line for it said *"Wallet traffic is riding the named runtime"* — the
strongest carriage claim on the surface, attested by nothing: the derivation
consulted no carriage input at all. Meanwhile the evidence existed and never
reached the state (`PrivateRpcWitness`, whose own doc says *"a dial is not
evidence: a transport that accepts and then blackholes completes connects
forever while carrying nothing"*) and fed only the fall-back window.

The failure shape this leaves invisible is the commonest active-censorship move
and the cheapest: **a transport that ACCEPTS connections and carries nothing.**
The host's own health ladder cannot see it either — a refusal streak needs
refusals, and a blackhole produces none at the host's dialer (FR-36). So on a
blackholed path the wallet read `Active`, the host rendered "ready", and the
user's balance simply never moved.

Under `Required` the gap was not a missing fall but an ATTRIBUTION error in the
other direction: every transport failure and every timeout was stamped
`TorUnavailable`, so a wedged SERVER over a perfectly healthy private path
rendered *"the private path is unavailable"* (`docs/plan/stage-1-private-path-truth.md`
§3.0 F-C). One word was doing two jobs, and it was wrong about the one it was
asked more often.

The honest question — *is it the path or the server?* — the SDK **cannot answer
from inside**. Both look identical from one connection: bytes went out, nothing
came back. The host CAN often answer it, from its own side, at zero extra
traffic (its relay rides the same arti), which is why the probe-the-second-server
alternative below lost.

## Decision

**One new state value, on the SAME clock as the switch, that says exactly what
the SDK knows and no more.**

`TorState::Unanswered { runtime }` — *the configured path is READY, and nothing
has come back over it for the founder's minute after the first observed failure,
while the wallet was trying: it may be the path or it may be the server.* Its
payload is `Active`'s, so a host renders the same transport name with a
different sentence. It carries **no attribution**, because the SDK has none.

Three rules fix its meaning:

1. **"ACCEPTED" IS THE DIAL'S `Ok`** — the `wallet.dial … outcome=connected`
   line — whatever becomes of the connection afterwards. Under `Required` a
   transport failure over a connection whose dial was accepted (a TLS handshake
   that never completes, an accept-then-close, an RPC that hangs) carries
   `EndpointUnreachable` and reaches this value on the first attempt at or
   after the minute; only a dial that FAILED (refused, unreachable, a dial
   timeout, a cleared or `FAILED` descriptor) keeps `TorUnavailable` and reads
   `Unavailable`, promptly. The fail-closed word is a claim about the DIAL, not
   about which timer fired. *"At or after" is exact and is the price of rule 2's
   honesty: the value turns on an OBSERVED failure span, not on a clock, so on a
   real path it lands on the first attempt past the minute rather than on a
   timer — bounded by one RPC budget (`TOR_PATIENCE_SECS +
   GRPC_UNARY_TIMEOUT_SECS`) and pinned by two named rows.*
2. **THE SUBJECT IS THE CONFIRMABLE CLASSES, AND REFUTING IS WIDER THAN
   TRIGGERING.** Only `Sync` and `Broadcast` can RAISE it — a class whose RPCs
   cannot confirm never triggers it — and FAILING is per class. Silence,
   though, is the union over every class whose stamp means an RPC CARRIED
   (`Sync`, `Broadcast`, `Other`): any completed RPC over the path refutes
   "nothing has come back", whatever class asked for it, and the sync-server
   probe and the ephemeral-detect poll are real `Other` traffic over the same
   private path. **`Swap` is in NEITHER set**, and that asymmetry is the whole
   point: its stamp is a bare CONNECT, and a transport that accepts and carries
   nothing completes connects for ever — so admitting `Swap` to the refuting
   side would hand the censor's cheapest move the power to end the state it
   raises. *(Written narrower here at first — "the union over `Sync` and
   `Broadcast`; `Swap` and `Other` never move it" — and corrected during the
   review fold, which is where the `Swap` trap was found: a remedy proposed as
   "take the max over EVERY class" would have reintroduced exactly the hole the
   posture's module header exists to close.)*
3. **PRECEDENCE:** a latched clearnet leak (`FellBack`) outranks everything;
   then the host's declaration (nothing registered, `FAILED`, not-ready —
   ADR-0545: the host is trusted); then the fail-closed stall
   (`TorUnavailable` → `Unavailable`, because a refused dial's EVIDENCE outranks
   a timeout's non-evidence); then this value; then `Active`. The derivation
   stays blind to `Preferred` vs `Required` — a policy branch inside it is
   forbidden, and a proptest says so.

**The host is TOLD, not left to poll.** `watch_tor_state` streams every
transition once (current state first, latest-wins, never closed by a transient
fault); a host that was not listening gets the state from `snapshot()` on
resume. Under `Preferred` the sequence is two events — `Unanswered`, then
`FellBack` at the clearnet dial — and the announcement obligation of ADR-0552
keys on the second.

**And the host can show the route as numbers:** `dial_counts()` returns dials by
arm × outcome since the wallet opened, counted inside the one site that emits
`wallet.dial`, so the number and the device log cannot disagree (FR-37).

## Alternatives considered

- **A carriage FIELD on `Active`** (`Active { carrying: bool }`) — lost. A host
  that does not read the new field keeps rendering the old sentence, silently;
  a new VARIANT is a compile break in an exhaustive consumer, which is what the
  one shipped host asked for and what makes the change impossible to miss.
- **The SDK probing the SECOND sync server to attribute path-vs-server** —
  lost, and the host asked us not to build it (sync point 1). It buys an
  attribution the host can usually make from its own side for free, and it
  spends traffic and a second endpoint's metadata to do it. "The server is not
  answering" is the right sentence for the case it would resolve anyway.
- **A SECOND, shorter clock for doubt** (report "not carrying" before the
  minute) — lost. It is a second constant and therefore a second founder
  sentence, and it would let a host build a competing timer. One clock owns the
  promise. The price is accepted and stated: under `Preferred` the first event
  is brief by construction.
- **Keeping `Unavailable` at ~30 s for timeouts under `Required`** — lost: that
  is F-C, the over-claim this ADR exists to remove.
- **Saying WHICH — path or server** — refused. The SDK has no evidence that
  separates them, and a state that guesses is worse than one that says it does
  not know: the user acts on the sentence.

## Consequences

- **EASIER:** a blackholed private path is now visible, as an event, to any
  host — no polling, no history replay, no transport named that the host did not
  declare. A `Required` wallet stops blaming the private path for a wedged
  server. A host can render the route as numbers beside its own.
- **HARDER / FROZEN:** `TorState` gains a variant, so every exhaustive consumer
  must grow an arm — `BRIDGE_ABI_VERSION` 2 → 3, and a Dart half generated
  against 2 is refused at init. The three ABI artifacts land together. The
  meaning above is now the wire contract; narrowing it later is a new ADR.
- **A BEHAVIOUR CHANGE a host renders:** under `Required`, an established
  connection whose RPCs time out reads `Active` + `Stalled { EndpointUnreachable }`
  from the first observed failure and `Unanswered` on the first attempt at or
  after the minute (see rule 1's caveat — the value turns on observed failure,
  not on a clock, so it can be up to one RPC budget later than the minute
  itself), where it read `Unavailable` at ~30 s. Over `https` the first observed
  failure is the connector's TLS bound (~25 s), so the user-visible minute is
  ~85 s.
- **`Required` now REDIALS:** a private connection whose RPC fails is retired,
  so the next RPC dials again — one `wallet.dial dial_arm=private` line per
  failed RPC where it previously rode one wedged connection for the client's
  life. Zero clearnet, never latched.
- **A new always-readable signal, priced:** the counters cross the bridge only
  on the host's call, are never logged, and carry counts by arm × outcome and
  nothing else — deliberately not bytes, which the SDK cannot count honestly
  above a host-supplied stream.
- **Follow-ups:** the user-facing sentences for the new value and the re-wording
  of the genuinely-down copy (stage S1 `copy`); a future `set_tor_policy` epoch
  must reset the posture's windows, which sharing the wallet posture into
  `Required` makes load-bearing (`net/tor_posture.rs`, the `fell_back` field
  doc).
