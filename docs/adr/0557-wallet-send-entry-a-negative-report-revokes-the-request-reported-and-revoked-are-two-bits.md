# 0557 — Wallet send entry: a negative report REVOKES the request — "reported" and "revoked" are two bits, and the mount grace revokes

- **Status:** Accepted (built and landed in the same session — stage S8
  `deadline`, S293, `docs/plan/stage-8-payment-identity-and-durable-retry.md`
  §3.3 and Relim's sync-point-1 answer, §4; the ruling is
  `docs/adjudication/s8-deadline/ruling.md`) · **superseded IN PART by
  [ADR-0558](0558-wallet-send-entry-the-revoked-request-is-remembered-by-the-controller-not-one-screen-state-and-a-one-sided-correlation-id-is-not-a-new-request.md),
  2026-09-21 (the S294 diff review):** the Decision's "a property of the
  request as the screen State holds it" (the memory is the root-scoped
  controller's — a fresh mount read no State and paid) and its bare-request
  rule "the same `correlationId`, or untagged with the same fields" (a tag on
  ONE side only does not make a new request; the fields decide unless BOTH
  sides are tagged). The two bits, the grace revoking before it reports, the
  controller's refusal at both seams before the authorizer, the unchanged
  report family and row 687 all STAND.
- **Date:** 2026-09-20
- **Links:** FR-26 (`docs/handoff/host-feature-requests.md` — the entry
  report; rows 658, 687, 789 of its test file stand) · `docs/specs/wallet-sdk.md`
  §FR-26 (`:5456-5510`) · `docs/reviews/2026-09-20/production-readiness-review.md`
  R05 · `docs/plan/audit-2026-09-20-remediation.md` §2b · Relim's D4 (their
  authorizer is an opaque per-spend bracket that must not learn about the
  entry) and #1029 (they have no `WalletSendReport` switch yet)

## Context

`WalletSendEntry.push` mints a one-shot reporter and a 5-second mount grace;
when the grace runs out before the send screen attached, it reports
`WalletSendNoTransaction`. But the route can still mount afterwards and pay —
from the screen that eventually appears, from a fresh flow (`didUpdateWidget`
re-seeds the form and `resetToForm()` mints a fresh flow id), or from a
bare-request re-navigation (the router builds `SendScreen(prefill:)` with no
reporter). The host then holds an irreversible negative for a payment that
happened on chain with no host record. This is R05, a permanent RED probe by
ruling (`R05 — a send route that mounts after the timeout cannot pay under a
"no transaction" report`).

"Reported" cannot serve as the guard: FR-26 row 687 requires the channel to
stay OPEN after a denied authorization (the flow is still spendable), and
`_closeIdleClaims` reports on retired channels that must not gate anything. A
guard keyed on the reporter is dropped by a bare-request re-navigation; one
keyed on the flow id is dropped by an in-place update.

Relim, asked at sync point 1 whether they wanted a deferred `unresolved`
outcome with a second phase: no — their spec consumes the report three-valued
and composes a record only for "a transaction exists"; an `unresolved` variant
buys nothing without a second-phase consumer their D4 has no place for; a
final, true negative closes the only R05 residual reaching them; the cost is
one extra tap on a slow phone, stated in copy.

## Decision

**The mount grace's negative report is only ever produced together with
revoking the REQUEST's ability to spend.** (The other producers of
`NoTransaction` — a user who left, a declined authorization, a refused
build/sign, a retired channel — revoke nothing: the flow they ended is still
the user's to pay, FR-26 row 687; the adjudicator struck the first draft's
universal, which the contract's own rows denied.) The grace timer marks the
request revoked — a property of
the request as the screen State holds it, surviving `resetToForm` and a null
reporter — and only then reports `NoTransaction`. **The controller reads the
revoke bit at both places the spend is asked for, `confirm()` and
`queueOffline()`, BEFORE the host's authorizer is invoked**, and refuses. A
screen that mounts after the grace shows that the request expired and offers
no way to pay it; the copy states the limit (a phone that took more than five
seconds to open the send screen) and the next step (start again from the app).

**A bare-request re-navigation with the SAME request** (`const`-equal fields,
or the same `correlationId`) after a revoke is refused — the host holds an
irreversible negative for that request. A bare request with a DIFFERENT
`correlationId` (or none and different fields) is a new host act and pays.

Every refusal keys on the revoke bit and never on `isReported`: a denied
authorization still leaves the flow spendable and the channel open (row 687).
The report family's SHAPE does not change — no new sealed variant, which would
break every exhaustive host of a published SDK to serve one host's preference.

## Alternatives considered

- **Defer at the grace with an `unresolved` outcome and a second phase.**
  Priced in its NON-BREAKING shape only (a reason on the existing
  `WalletSendUnclassified`, never a new variant). It costs a second-phase
  consumer nobody has asked for, and it needs this ADR's revoke anyway. Kept as
  the alternative, not chosen.
- **A new sealed variant.** Breaks exhaustive hosts; the "hard compile break
  Relim wants" was said of `TorState`, not of this family (struck premise).
- **Keying the guard on the reporter or the flow id.** Each is dropped by a
  navigation shape the FR-26 file already exercises (rows 658, 789).

## Consequences

- A final, true negative: nothing a request the grace reported negatively
  signed is broadcast afterwards, through any door, and the refusal is the
  controller's — a host that skips the widget layer sees the same refusal.
- Lost, stated: a slow-but-legitimate redirect that exceeds the grace costs the
  user one restart from the host. `_mountGrace` may be re-priced by the
  adjudicator; on Relim's host the unlock-gate redirect never re-mounts a pushed
  route (READ, not driven — they are ticketing a driven check; if it re-mounts
  they route with a fresh `correlationId` rather than ask us to soften the
  negative).
- Frozen: one new screen state and its strings (16 locales, machine copy at
  beta); the FR-26 handoff entry and the spec's FR-26 shape gain "a request the
  entry's mount grace reported negatively cannot pay afterwards, through any
  door, and a bare host `replace`/`go` with the same request is refused".

## As built (S293; `docs/adjudication/s8-deadline/ruling.md`, adjudicated GREEN)

- **The revoke bit** is `WalletSendReporter.revoke()` / `isRevoked` — a
  separate bit from `isReported`, set only by the entry's grace timer, and set
  BEFORE `report(WalletSendNoTransaction)`. The screen State holds it as a
  property of the request it carries (`_expired`, read from the reporter in
  `initState` and `didUpdateWidget`); with no channel — a bare-request
  re-navigation — the bit carries over exactly while `sameSendRequest(previous,
  prefill)` holds: `identical`, else if either side carries a `correlationId`
  the tags decide, else the payment fields.
- **The controller** lands on the new `SendState.SendRequestExpired` at an
  entry reset of a revoked request (`resetToForm(requestRevoked: true)`) and
  when `confirm()` or `queueOffline()` is asked under one — the refusal
  precedes `authorizeSpend` (asserted: the authorizer's intents list stays
  empty, `hasEnteredSpend` false, `sendCount` 0, never `SendSubmitting`).
- **The screen** renders a full-screen expired state (title, the body that
  states the five-second limit and the next step, Done → wallet root); no
  form, no Review, no Queue. `_mountGrace` stays 5 s.
- **Doors:** a late mount, a nested-navigator late mount, a fresh form via
  `didUpdateWidget`, a bare `replace`/`go` with the same request (same
  `correlationId`, or untagged with the same fields) — all refused; a
  different `correlationId`, untagged different fields, and a fresh `push`
  (a new channel) pay. A denied authorization and a retired channel revoke
  nothing (rows 687, 658 — pinned and green).
- **The probe** (`R05 — …`): its lines 906–917 asserted the base tree's
  witness (`payThrough` → `sendCount 1` → `TransactionCreated`); replaced at
  the adjudication, on the orchestrator's delegation, by contract row 1's
  assertions; the grader printed its NEWS line; the whole send module 293/0.
- **Watched:** 28 Dart mutants by frame (the driver grades cargo shapes
  only), 5 survivors on purpose (pinned rows that discriminate only against
  an over-broad revoke). Registered as 25 observations (three pairs of
  mutants share one frame).
