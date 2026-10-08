# 0546 — `Preferred` falls back to clearnet on unreachable or timeout ONLY; not-ready, refused and retired never trigger it

- **Status:** Accepted · **superseded IN PART (its TIMING only) by [ADR-0552](0552-wallet-transport-preferred-insists-on-the-private-path-for-a-minute-then-switches-visibly.md), 2026-09-18** — the error set below (unreachable or timeout ONLY, and the visibility rule) STANDS unchanged; what 0552 replaces is WHEN a qualifying failure may switch: no longer the first one, but the first after `TOR_PATIENCE_SECS` of private-path silence (the founder's minute)
- **Date:** 2026-09-16 (S280, at the FR-29 spec's one reviewer pass)
- **Supersedes:** ONE sentence of
  [ADR-0545](0545-wallet-transport-host-is-trusted-isolation-requested-not-demanded.md)'s
  Consequences — "The fail-closed matrix collapses to one rule: not-ready or
  retired → `Required` fails closed, **`Preferred` falls back visibly**, `Off`
  is unaffected." The `Required` and `Off` halves of that sentence stand;
  the `Preferred` half is replaced by the rule below. Nothing else in
  ADR-0545 changes; its Decisions 1–5 stand whole.
- **Links:** [ADR-0544](0544-wallet-transport-crossing-corrections-from-the-design-audit.md)
  Decision 5 (the frozen error table: "`NotReady`, which is not a
  reachability failure and never triggers `Preferred`'s fallback") · the
  design audit's HIGH on the error taxonomies (`docs/handoff/s280-resume.md`
  §6: "map `NotReady` to `Unreachable` and every app start leaks clearnet
  during arti bootstrap") · `docs/ROADMAP.md` W5 (1)'s named test
  "`Preferred` `fellBack` visible on unreachable/timeout only" ·
  `docs/handoff/host-session-complete-integration.md` H-12 ("map `NotReady`
  to the SDK's not-ready code, never to unreachable") ·
  `docs/specs/host-transport-crossing.md` §0 A9, §3.3, §6.2

## Context

ADR-0544 Decision 5 froze the dial-code table and said not-ready never
triggers `Preferred`'s fallback. ADR-0545, written the same night to record
the founder's trust ruling, summarised the fail-closed matrix in one
sentence and, in that summary, said `Preferred` falls back on not-ready or
retired. The two sentences disagree. The FR-29 spec's consistency audit
found the disagreement and first ruled on it in a spec table row; its
reviewer refused that mechanism — an accepted ADR is corrected by a new
ADR, never by a spec — which is this file.

The engineering fact behind the rule is the audit's HIGH: a host transport
that is bootstrapping (Tor building its first circuits, ten to sixty
seconds after every app start) reports not-ready; if `Preferred` treated
that as a reachability failure the wallet would dial clearnet at every
launch and latch `fellBack` for the session — the exact leak the policy
exists to make visible, produced silently by the SDK itself. Retired is a
host that tore its backing down to re-arm another (a carrier switch,
honest-off) and refused is a transport that will not carry the request;
neither is the network being unreachable.

## Decision

**`Preferred` falls back to the SDK's direct dialer, visibly (`fellBack`),
on `Unreachable` and `Timeout` only.** `NotReady`, `Refused` and `Retired`
never trigger the fallback under any policy: `Required` fails closed and
`Preferred` WAITS, rendering `Bootstrapping { percent }` (not-ready) or
`Unavailable` (refused, retired) until the host's next descriptor or
completion says otherwise. `Off` is unaffected.

This is the `PolicyDialer` already shipped (`net/dialer.rs`: the fallback
arm matches `Unreachable | Timeout`; every other error is surfaced as-is)
extended by two error variants that fall into the surfaced arm by
construction.

## Consequences

- ADR-0545's collapsed matrix reads, corrected: not-ready or retired →
  `Required` fails closed, `Preferred` waits, `Off` is unaffected;
  unreachable or timeout → `Required` fails closed, `Preferred` falls back
  visibly.
- The host's error mapping (H-12) maps its `NotReady` to the SDK's
  not-ready code, never to unreachable; its retire to the retired code.
- Named tests: `preferred_fell_back_on_unreachable_or_timeout_only` and
  `required_fails_closed_on_not_ready_and_retired` (spec §8 T1, T2).
- A host whose user turns circumvention OFF does not leave the wallet
  waiting: it swaps its DIRECT dialer in behind the trampoline (ADR-0545
  Decision 1 lists direct among the host's transports), and the descriptor
  then says so. The wallet never decides clearnet for the host under
  `Required`; under `Preferred` it decides only on the two reachability
  failures.
