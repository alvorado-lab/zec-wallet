# 0552 — Wallet transport: `Preferred` INSISTS on the private path for about a minute, then switches to a direct connection and TELLS the user

- **Status:** Accepted (founder ruling, S286, 2026-09-18, in his words:
  "keep waiting, but I think 1 minute is enough to switch with notification")
  · **Correction, same day, append-only (the docs consistency pass):** the
  Consequences below predicted that the honest "still trying" state "costs a
  stall-reason variant across the bridge and its 16 locale keys". IT DID NOT.
  The mapping found the wallet already has the right state — while insisting it
  is fail-closed with zero clearnet packets, which IS
  `StallReason::TorUnavailable` → `TorState::Unavailable`, with copy that
  already exists in all 16 locales — so stage 1 shipped with no new DTO
  variant and no new key (`docs/plan/tor-patience-phase-1.md` §4.1, §6a-4).
  The DECISIONS below are unaffected; only that cost estimate was wrong.
  · **Correction 2, same day, and it REVERSES correction 1 (the security and
  crypto angles, both HIGH):** reusing the existing state was wrong, and the
  honesty half built on it has been WITHDRAWN from the tree. Two seams no unit
  test could see: the `DialPlan` is built ONCE for the long-lived sync client,
  so a latch-derived reason freezes at "not yet switched" and keeps asserting
  it after the wallet has gone to clearnet; and that state's copy
  (`walletTorUnavailable`) says "nothing was sent in the clear", so the frozen
  reason would tell a censored user no clearnet traffic occurred at the moment
  their address reached the operator. Correction 1's "the wallet already has
  the right state" confused the right TorState with the right SENTENCE.
  Decision 2's honest "still trying" therefore needs the reason resolved AT
  FAILURE TIME and its OWN copy — both deferred to stage 2 with the
  notification edge. **Decision 3's admissibility clause is consequently UNMET
  in the tree today: the wallet switches and nothing announces it.** That is
  stated rather than quietly carried; stage 2 closes it. **And Decision 2's own
  second clause — "the wallet says it is still trying the private path" — is
  equally unmet today: what it says is the ordinary "endpoint unreachable",
  which claims nothing. Decision 2's FIRST clause (a qualifying failure inside
  the minute does NOT reach clearnet) is built and guarded; only the sentence
  is owed.**
  · **Correction 3 (S287, append-only, NO decision changed): what "the private
  path works again" MEANS, and what the window's scope is.** Stage 1 measured
  decision 4's reset from a completed DIAL. The built-diff review showed that
  makes the ruling unimplemented for the case it was written for: a transport
  that ACCEPTS and then blackholes — active censorship's cheapest move —
  restarts the window on every attempt, so the minute never elapses. The reset
  now requires a completed RPC over a connection the private primary served
  (`net/tor_posture.rs`), which is the first evidence the circuit CARRIED
  anything. In the same pass the window gained a SUBJECT: one per circuit
  family (sync, broadcast, other), because a healthy sync circuit was resetting
  the window a starved send was measuring — a payment that never leaves, under
  a UI reading UpToDate. And two scope facts that were true but unwritten: the
  window is PER SESSION (create/open/rescan each take a fresh posture; only the
  server switch carries one across) and it counts device-AWAKE time. Both err
  towards insisting, which is the private direction. **Decision 3's
  admissibility clause remains UNMET** — stage 2 still owes the announcement.
  The 60 seconds are untouched.
  · **Correction 4 (S287, append-only, and it retracts part of correction 3).**
  Two things. **(a) A timing constant DID move**, which the Consequences below
  deny ("No existing timing constant moves… The per-dial bound stays
  `DIAL_TIMEOUT_SECS` = 30 s"): it is **25 s** since S287, because it must sit
  strictly below `GRPC_UNARY_TIMEOUT_SECS` or a unary call cancels the dial at
  the instant the dial bound fires and the switch is never evaluated on the
  money path. The 60 seconds are still untouched. **(b) Correction 3's
  mechanism was itself defective and has been redesigned.** Measuring the
  window by SILENCE alone — even silence of confirmed RPCs — cannot tell a
  broken private path from a wallet that had nothing to send: `Broadcast`'s
  stamp moves only on a completed payment, so between sends it read the whole
  session age and the first send after a minute of uptime would have gone
  clearnet on its first transient failure, with zero seconds of insisting. A
  four-angle review on the built diff found it. **The window now has two
  conjuncts — a class may switch only when it has been both SILENT and
  continuously FAILING for the minute** — and a failure is either a dial that
  failed or an RPC that failed over a live private connection, because the
  blackhole shape this ADR exists for succeeds at dialling and only fails at
  the RPC. Plan: `docs/plan/tor-patience-phase-2.md`. **The DECISIONS are
  unaffected; what changed is that the minute now means a minute of the path
  NOT WORKING rather than a minute of it not being used**, which is what
  decision 1 says in words.
  · **Correction 5 (S290, append-only, NO decision changed): the window's
  CONSUMER landed — on the path this ADR was written for.** At S289 the stage
  S1 contracts found (F-A, `docs/plan/stage-1-private-path-truth.md` §3.0)
  that `may_switch_to_direct` was read in ONE place, the `Err(Unreachable |
  Timeout)` arm of `PolicyDialer::dial`, and that the production sync client
  dialled ONCE for the wallet's life (its lazy channel built on the first pass
  and kept), so a private path that ACCEPTED and carried nothing — correction
  3's own case — never switched: the predicate turned true and nobody read it.
  Built at S290 (`window`, plan §5; the adjudication is
  `docs/adjudication/s1-window/ruling.md`): `PolicyDialer::dial_attributed`
  decides once per dial AFTER the primary answers, whatever it answered; an
  ACCEPTED private dial is left for clearnet when its class has been silent
  for the minute AND was seen failing past it
  (`TorPosture::kept_failing_past_the_window` — a failure observed
  `TOR_PATIENCE_SECS` or more after the run began, so a bare clock read never
  walks a resumed or idle wallet off a working path; on a sparse driver the
  accept is used once more and the switch lands one RPC later, the
  mechanism's stated cost); and the gRPC client RETIRES a private connection
  whose RPC failed, so the next RPC dials again (`net/grpc.rs`,
  `PrivateRpcWitness::note_failed`) — the second dial F-A said did not exist.
  Still ONE production reader of the predicate. Three facts this correction
  adds to the record: **(a) the minute a user sees dates from the failure
  FIRST OBSERVED** — on an accepting path that is the first RPC timeout,
  `GRPC_UNARY_TIMEOUT_SECS` = 30 s in (up to `GRPC_STREAMING_TIMEOUT_SECS`
  when a pass's first RPC is a stream), so from the user's chair the switch
  reads ~90–160 s after the first hang; the 60 seconds are still untouched,
  only where they are counted from on the shape where nothing fails visibly.
  **(b) The clearnet leg is bounded:** `FALLBACK_ESTABLISH_BUDGET_SECS` = 5 s
  from the switch instant, TCP and the TLS handshake TOGETHER (the handshake
  spends what the connect left), with `DIAL_TIMEOUT_SECS +
  FALLBACK_ESTABLISH_BUDGET_SECS <= GRPC_UNARY_TIMEOUT_SECS` asserted at
  compile time; every attempt gets the same 5 s and every dial asks the
  primary FIRST, after a switch too — there is NO primary-leg skip after the
  latch (ADR-0546's refusals must still be seen), so under an ACCEPTING
  primary the primary leg costs ~0 s per pass and under a HANGING one every
  pass spends `DIAL_TIMEOUT_SECS` on it before the 5 s. **(c) The error set,
  as ADR-0553 built it:** a transport its host declared FAILED
  (`ZW_HEALTH_FAILED`) is refused at the readiness gate as
  `DialError::TransportFailed`, counts as the private path failing, and under
  `Preferred` is switch-eligible after a minute of it — the minute of FAILED
  starts at the declaration; a bootstrap (`NOT_READY`, health `Starting`)
  never reaches clearnet, however long (ADR-0553 correction 1). The `Swap`
  class alone takes a bare connect as evidence; `Other` is strict again.
  `Required` is unchanged: never clearnet. **Consequences retracted by the
  build, quoted:** "**The switch lands on the second or a later failed
  attempt**, about a minute in user time … because the crossing failure
  switches within its own call" — holds for a primary that REFUSES or HANGS;
  on the accepting-and-silent shape the dial that switches is an ACCEPTED one
  (the failures that spent the window were RPC failures over the previous
  connection, which the client retired), and "about a minute in user time"
  reads ~90–160 s there, per (a). "**No existing timing constant moves, and
  no RPC budget grows**" stays retracted as correction 4 (a) left it, and a
  NEW constant now sits beside the 25 s — the 5 s of (b); no RPC budget grew.
  The remaining bullets hold as written, or as corrections 1–4 already left
  them (the plugin bullet's "30 s" reads 25 s since 4 (a)). Decision 3's
  announcement is still owed — it is the stage S1 `truth` item's.
  · **Correction 6 (S291, append-only, NO decision changed) — the SDK half of
  Decision 3, and part of Decision 2's second clause, are DISCHARGED by
  [ADR-0554](0554-wallet-transport-a-ready-path-that-carries-nothing-gets-its-own-state-and-the-sdk-does-not-guess-why.md),
  built at stage S1 `truth`.** This status bullet says in three places that
  Decision 3's clause is UNMET, that nothing announces the switch, and that
  "only the sentence is owed". All three read as the current state and none of
  them is. The annotation is written here because `docs/adr/README.md` makes it
  the required half of "link both ways" for a partial discharge, and because the
  ROADMAP and `wallet-sdk.md` already record this work as BUILT — leaving this
  file the one place in the tree still saying the opposite.
  **What landed, precisely, because the difference is the whole point:**
  (a) Decision 3's SDK HALF — the moment is observable, through `watch_tor_state`
  as a stream of transitions; under `Preferred` the two a host sees are
  `Unanswered` then `FellBack`. **Decision 3 is NOT discharged whole. The
  user-visible announcement is the HOST's obligation, FR-34 is still open on the
  action board, and this correction does not close it.**
  (b) Decision 2's second clause AT OR AFTER the minute — the wallet now says
  what is true, through the `unanswered` copy family, and the reason is resolved
  at FAILURE time (`transport_stall`, keyed on whether the dial was accepted)
  rather than at plan time. **INSIDE the minute is unchanged and still claims
  nothing:** it reports the ordinary `Stalled { EndpointUnreachable }`.
  Nothing below this status line is touched.
- **Date:** 2026-09-18 (S286)
- **Supersedes:** [ADR-0546](0546-wallet-transport-preferred-falls-back-on-unreachable-or-timeout-only.md)
  **in part — its TIMING, not its error set.** ADR-0546 settled WHICH failures
  may move a `Preferred` wallet to clearnet (unreachable or timeout only) and
  that the move is visible; it left WHEN implicit, and the implementation took
  "the first qualifying failure", i.e. after one dial bound (30 s). This ADR
  keeps 0546's error set, its visibility rule and `Required`'s fail-closed
  posture untouched, and replaces the implicit timing with an explicit one.
- **Links:** `docs/specs/host-transport-crossing.md` §3.5 · `docs/specs/wallet-sdk.md`
  §3.2a · `docs/plan/tor-patience-phase-1.md` (the build plan) ·
  `sdk/zec-wallet-core/src/constants.rs` (`TOR_PATIENCE_SECS`,
  `DIAL_TIMEOUT_SECS`) · `sdk/zec-wallet-core/src/net/dialer.rs`
  (`PolicyDialer`) · `docs/plan/fr5-phase-1.md` §5 D-17 (the question this
  answers)

## Context

The question came out of the FR-5 C3a review (S286). The crypto angle found
that a `Preferred` wallet already reaches clearnet after 30 seconds of Tor
silence, because the SDK bounds every accepted dial at `DIAL_TIMEOUT_SECS` and
`Preferred` follows a `Timeout` to its direct fallback. Nobody had decided
that: it fell out of a liveness bound whose purpose is to stop a hung host
dialer wedging the channel worker. The plan's D-17 had recorded the open
question in user terms — "on a network where Tor is blocked, should a wallet
set to 'prefer private' ever give up and connect directly, and on what
evidence" — and the review re-cut it as a timer rather than a blockage, since
arti's `Offline` blockage is itself censor-inducible (that half is unchanged:
no code-2 producer is built).

What makes this a founder decision rather than an engineering one: the two
failure directions harm different users. Switching too eagerly exposes the IP
address of someone on a censored network to the lightwalletd operator, beside
their scan and submit pattern. Never switching leaves a user on a merely flaky
network with a wallet that cannot see its money. Thirty seconds was nobody's
answer to that trade.

## Decision

1. **A `Preferred` wallet keeps insisting on the private path for
   `TOR_PATIENCE_SECS` = 60 seconds before it will switch.** The clock is
   measured from the LAST SUCCESSFUL private dial, or from the dialer's
   creation when none has succeeded yet — not from the first failure, so a
   cancelled attempt (an RPC budget expiring before the dial bound) cannot
   stop the clock from advancing.
2. **A qualifying failure inside that minute does NOT go to clearnet.** It
   returns typed, and the wallet says it is still trying the private path.
3. **The first qualifying failure after the minute switches that dial to the
   direct path, latches the visible fell-back state, and is announced.** The
   SDK makes the MOMENT of the switch observable; the host raises the user
   notification, and the SDK's own example app shows a notice. A switch the
   user is not told about is not an admissible implementation of this ADR.
4. **A successful private dial resets the clock**, so the wallet returns to
   insisting as soon as the private path works again.
5. **`Required` and `Off` are untouched.** `Required` never switches (zero
   clearnet packets, ADR-0542/0546); `Off` is the user's own stated choice.

## Consequences

- **No existing timing constant moves, and no RPC budget grows.** The per-dial
  bound stays `DIAL_TIMEOUT_SECS` = 30 s, so the compile-time relations that
  protect the establishment chain (`3 × DIAL_TIMEOUT_SECS <
  GRPC_STREAMING_TIMEOUT_SECS`, the stagger's quarter-budget rule) hold
  unchanged. Raising the dial bound to 60 s instead would have broken the first
  of those at compile time and slowed every non-Tor failure path on every
  platform — which is why the minute lives in a patience clock, one layer above
  the liveness bound.
- **The switch lands on the second or a later failed attempt**, about a minute
  in user time. With a 30 s dial bound the first failure opens the window and a
  later one crosses it; the wallet does not wait a minute and then spend
  another dial bound before switching, because the crossing failure switches
  within its own call.
- **A queued send may switch in a later resubmission pass.**
  `RESUBMIT_BROADCAST_BUDGET_SECS` is also 60 s and bounds one pass's broadcast
  phase, so a pass can spend its whole budget insisting. The queue is durable
  and re-broadcast is idempotent, so that is a delay, not a lost payment.
- **A new user-visible state.** "Still trying your private path" is not
  "endpoint unreachable", and reporting the second for the first would be a
  dishonest degradation (principle 6). This costs a stall-reason variant across
  the bridge and its 16 locale keys (machine copy ships, founder S272).
- **The plugin side is unaffected.** FR-5 C3b still owes a dial deadline below
  the SDK's 30 s bound and its own liveness rule; this ADR changes what the
  SDK does with the failure the plugin reports, not what the plugin reports.
- **The guard that keeps this true:** named tests at both constants'
  boundaries — no switch before the minute, a switch on the first qualifying
  failure after it, the clock reset by a success, `Required` never switching —
  driven on a paused tokio clock, never wall-clock sleeps.

## Alternatives rejected

- **Raise `DIAL_TIMEOUT_SECS` to 60 s.** The obvious reading of the ruling, and
  wrong: it breaks `3 × DIAL_TIMEOUT_SECS < GRPC_STREAMING_TIMEOUT_SECS` at
  compile time, and it would make every blackholed direct dial — on `Off`,
  `Required` and clearnet alike — take twice as long to fail.
- **Count failures instead of time** ("switch after N failed attempts"). N
  attempts is not a minute: it varies with how fast each attempt fails, so the
  user-visible promise would drift with network conditions.
- **Switch immediately but notify** (keep today's timing, add the notice). It
  answers the notification half and refuses the founder's first two words.
