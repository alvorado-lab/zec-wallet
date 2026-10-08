# 0553 — Wallet transport: a private path that never STARTS also switches after the minute

- **Status:** Accepted (founder ruling, S286, 2026-09-18 — asked in user terms
  and answered in one word: **"switch"**). **NOT YET BUILT** — this ADR is the
  record of the decision; the build is the next work item.
  · **Correction 1 (S290, append-only): BUILT — and NARROWER than decided
  here, on the health axis rather than the not-ready code** (stage S1
  `window`, `docs/plan/stage-1-private-path-truth.md` §5; the mechanism is
  `docs/plan/tor-patience-phase-2.md` §2a, the ruling
  `docs/adjudication/s1-window/ruling.md`). A bootstrapping transport refuses
  continuously, so letting `NOT_READY` spend the window would have leaked
  clearnet at every slow start, and never letting it would have stranded a
  wallet whose transport is not going to start; the fact that separates the
  two is the descriptor's `health` (ADR-0549). As built: `NOT_READY` — a
  bootstrap, at ANY readiness, health `Starting` — keeps the ABSOLUTE
  no-clearnet guarantee, however long it lasts; only a `FAILED` declaration
  (`ZW_HEALTH_FAILED`) is switch-eligible. The readiness gate refuses it as
  `DialError::TransportFailed` (the host is never asked), `PolicyDialer`
  counts it as the private path failing, and under `Preferred` a minute of it
  switches, visibly. **The minute of FAILED starts at the declaration** —
  ruled at the S1 `window` adjudication: a switch ON the declaration would
  bank the windows of `Starting` as failing, which is exactly what this
  correction refuses. The plugin's own `BOOTSTRAP_DEADLINE` (180 s) ends in
  `Failed` + `health = FAILED`, which is the state that then qualifies; a
  host that never says FAILED and never starts is a host bug, and the honest
  answer to a host bug is not a silent clearnet leak. `Required` is
  unchanged. What this retracts, quoted: Consequences bullet 1 — "**A host
  that returns `ZW_DIAL_NOT_READY` no longer guarantees that no clearnet
  packet follows.** It guarantees it for the first minute … the dial-code
  table (code 1) and E3 both say 'no — a bootstrap in progress' without
  qualification, and both must carry the window" — it DOES still guarantee
  it, unqualified; the code-1 row and E3 stay unqualified for `NOT_READY` and
  gain the FAILED case instead. Decision 1's "a not-ready refusal becomes
  switch-eligible" is built as "a FAILED declaration becomes switch-eligible";
  Decision 2's t=0 reasoning is what the narrowing keeps. Of "What the build
  owes": item 1 ("`DialError::NotReady` joins the switch-eligible arm") is
  built with `TransportFailed` in that arm and `NotReady` NOT in it; item 6
  ("returning NOT_READY stops being an absolute guarantee of no clearnet")
  is withdrawn — the registrant-facing sentence is that a FAILED declaration
  is switched away from after a minute under `Preferred`
  (`zec_wallet_net_dialer.h`'s health paragraph, corrected at `cfae0e3a`).
  The announcement (Decision 4) is still owed — the stage S1 `truth` item's.
- **Date:** 2026-09-18 (S286)
- **Supersedes:** [ADR-0546](0546-wallet-transport-preferred-falls-back-on-unreachable-or-timeout-only.md)
  **in part — its ERROR SET, and only after the minute.** 0546 froze the
  switch-eligible failures to unreachable-or-timeout and excluded not-ready
  explicitly, for a reason that still holds at t=0: every app launch has a
  bootstrap window, and falling back during it would leak clearnet routinely.
  This ADR keeps that at t=0 and admits a not-ready refusal as switch-eligible
  ONCE `TOR_PATIENCE_SECS` of private-path silence has passed. 0546's exclusion
  of `REFUSED` and `RETIRED` is untouched (see Consequences).
- **Extends:** [ADR-0552](0552-wallet-transport-preferred-insists-on-the-private-path-for-a-minute-then-switches-visibly.md)
  — same minute, same clock, same announcement obligation.

## Context

ADR-0552 settled what a `Preferred` wallet does when the private path goes
quiet MID-SESSION: insist for a minute, then switch and say so. It left an
adjacent case that its own mechanism cannot reach, found while self-reviewing
stage 1 and put to the founder rather than guessed:

**A private path that never STARTS produces no reachability failure at all.**
The readiness gate refuses the dial before it is made
(`net/readiness_gate.rs`) and the dialer returns `DialError::NotReady`, which
ADR-0546 forbids falling back on. So on a network where Tor is blocked at
startup — the commonest censorship shape — the wallet waited indefinitely,
honestly showing "starting", and never switched. A wallet whose Tor worked and
then went quiet switched after a minute; a wallet whose Tor never came up did
not. Same user, same network, two behaviours.

Put in user terms: "on a network where the private path never finishes
starting, should the wallet also give up after a minute and connect directly
with a notification — or keep showing 'starting' until the user changes the
setting?" The founder: **"switch"**.

## Decision

1. **Under `Preferred`, a not-ready refusal becomes switch-eligible once the
   patience window has been spent** — the same `TOR_PATIENCE_SECS` window,
   measured the same way (from the last dial the private path actually
   carried, or from the posture's creation when none ever has). Inside the
   window a not-ready refusal behaves exactly as today: typed, no clearnet.
2. **At t=0 nothing changes.** ADR-0546's reason for excluding not-ready is
   about the bootstrap window every launch has; the minute is what separates
   "starting" from "not going to start".
3. **`Required` is untouched**, permanently: it has no fallback and never
   consults the clock.
4. **The switch is announced**, on ADR-0552's terms — and that obligation is
   still owed there (its notification edge is unbuilt), so this ADR inherits
   an unmet clause rather than adding a second one.

## Consequences

- **A host that returns `ZW_DIAL_NOT_READY` no longer guarantees that no
  clearnet packet follows.** It guarantees it for the first minute. This is a
  HOST-FACING change: `host-transport-crossing.md`'s dial-code table (code 1)
  and E3 both say "no — a bootstrap in progress" without qualification, and
  both must carry the window. Relim is `required_`, so no host is affected
  today; the next registrant would have been.
- **`REFUSED` and `RETIRED` stay excluded, deliberately and for different
  reasons.** `REFUSED` is a host policy statement, not a failure to reach
  anything — switching on it would route around the host's own refusal.
  `RETIRED` is the host saying it tore its backing down and will re-arm behind
  the same trampoline; a retire that is never followed by a re-arm strands the
  wallet in the same way a never-starting path does, so **whether a persistent
  retire should also switch after the minute is the next question of this
  shape** — it was not asked and is not decided here.
- **This makes the window's REACHABILITY the load-bearing part.** With nothing
  ever succeeding, the clock runs from the posture's creation, so the switch
  lands about a minute after the wallet starts — but only if the window is
  reached at all, and stage 1b of `docs/plan/tor-patience-phase-1.md` records
  two ways it is not (a success is a completed DIAL rather than a completed
  request; one window per wallet lets a healthy sync circuit starve the
  broadcast path). **Build stage 1b first or this ADR ships a promise the
  mechanism cannot keep.**
- **The honest state while insisting is still owed** (ADR-0552 correction 2):
  the wallet reports the ordinary unreachable stall, not "still trying your
  private path". A not-ready wallet renders `Bootstrapping{percent}` today,
  which is honest for the first minute and becomes a lie after the switch —
  so the state derivation must read the latch ahead of readiness, as it
  already does for `FellBack`.

## What the build owes (the next session's checklist)

1. `net/dialer.rs`: `DialError::NotReady` joins the switch-eligible arm, but
   ONLY behind `posture.may_switch_to_direct()`. The existing arm's shape makes
   this a guard change, not a new branch.
2. `net/readiness_gate.rs`: the gate refuses BEFORE the dialer sees anything,
   so the not-ready path must reach the posture's decision — either the gate
   consults it, or the refusal flows to `PolicyDialer` and is decided there
   (prefer the latter: one place decides, as today).
3. Tests, each with its mutant: a not-ready refusal at t=0 and t=59 sends no
   clearnet; the first after the minute switches and latches; `Required` never
   switches however long it is not ready; `REFUSED`/`RETIRED` still never
   switch, at any time.
4. `host-transport-crossing.md`: the code-1 row, E3, and §7's timer list.
5. `wallet-sdk.md` §3.2a's policy table, and the §8 register.
6. Tell every registrant: returning NOT_READY stops being an absolute
   guarantee of no clearnet.
