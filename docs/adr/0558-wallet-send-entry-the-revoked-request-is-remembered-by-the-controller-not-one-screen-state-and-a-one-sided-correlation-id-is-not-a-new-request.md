# 0558 — Wallet send entry: the revoked request is remembered by the root-scoped controller, not by one screen State — and a `correlationId` on one side only does not make a new request

- **Status:** Accepted (stage S8 `deadline`, the DIFF review's fold, S294,
  2026-09-21; `docs/plan/stage-8-payment-identity-and-durable-retry.md` §5
  "S294"). **Supersedes [ADR-0557](0557-wallet-send-entry-a-negative-report-revokes-the-request-reported-and-revoked-are-two-bits.md)
  IN PART:** its Decision's "a property of the request as the screen State
  holds it" and its bare-request rule "the same `correlationId`, or untagged
  with the same fields" (and the As-built line "if either side carries a
  `correlationId` the tags decide"). Everything else in 0557 stands: the two
  bits, the grace revoking before it reports, the controller refusing at both
  seams before the authorizer, no new sealed variant, the other producers of
  `NoTransaction` revoking nothing.
- **Date:** 2026-09-21
- **Links:** ADR-0557 · FR-26 (`docs/handoff/host-feature-requests.md` item 6
  of the S8 entry) · `docs/specs/wallet-sdk.md` §FR-26 · the S8 plan §3.3
  (premise, row 3, "As built") · `docs/REVIEW.md` §3 (the post-build pass
  catches what the design pass structurally cannot)

## Context

The diff review of stage S8 (four angles + refuters on `dc57cb2d..50993e6f`)
found two doors still open on the deadline item, both on the same seam, both
inside the sentence ADR-0557 itself wrote ("nothing that request signed will
be broadcast afterwards — not from a bare host `replace`/`go` onto the send
path with the same request"):

1. **The revoke bit lived in ONE `SendScreen` State instance** (`_expired`),
   carried across a bare re-navigation only through `didUpdateWidget`. A
   FRESH mount of the send route with the same bare request — the user pops
   the expired screen, the host `go`es to the send path with the request it
   holds a final negative for — starts in `initState` with no previous
   request, computes the bit false, and pays. The contract's premise had
   said "a property of the request as the screen State holds it": the
   mechanism was scoped to the sentence, and the sentence was narrower than
   the exit gate's "through any door". Every re-navigation row in the two
   deadline test files started from an already-mounted screen, so no test
   could see it. (REVIEW.md §3's second tell — a mechanism scoped by its own
   premise; found by the post-build pass.)
2. **`sameSendRequest` let a tag on ONE side decide.** `if (tagA != null ||
   tagB != null) return tagA == tagB;` — an untagged request the grace
   revoked, re-navigated bare with identical payment fields plus a
   `correlationId` (a retry wrapper, an idempotency tag), was "a different
   request" and paid. The adjudicator had chosen that reading deliberately
   ("a host that tags has said what identifies them"); the contract's row 3
   named only the two symmetric cases and the asymmetric cell was untested.
   Two refuters split on the remedy; the code reviewer confirmed the
   tightened rule keeps every existing row green and fails only toward a
   spurious refusal — the safe side of a money predicate.

## Decision

**The memory of a revoked request belongs to the root-scoped
`SendController`**, which outlives every screen and every flow: the entry's
grace timer records the request there (`revokeRequest`) BEFORE it revokes the
reporter and reports `NoTransaction`; a screen holding a BARE request (no
channel) asks the controller (`isRevokedRequest`) in `initState` and in
`didUpdateWidget` alike, so a fresh mount and an in-place update read the same
answer. The memory is monotone for the process lifetime and survives a session
flip (as the controller's `_requestRevoked` bit already did). **A live channel
keeps precedence:** a fresh `push` of the same request is a new grant and
pays, whatever the memory holds — the restart the copy promises.

**Two requests are the same request when BOTH carry a `correlationId` and
the tags are equal, or when NOT both carry one and the payment fields are
equal.** A tag on one side only does not make a new request: the host's
identity signal is absent on that side, so the fields decide. (Both tagged and
different tags: a different request, as before — row 789 stands.)

## Alternatives considered

- **Keep the bit in the screen State and thread it through every door.** The
  fresh-mount door has no previous State to thread from; a static registry in
  the channel module would be process-global state no test can scope.
- **Keep the one-side-tag rule and only add the missing test rows** (the
  first refuter's remedy (b)). Coherent, but it keeps the paying direction on
  a money predicate for a host shape nobody has; the cost of the tightened
  rule is one restart via `push`, already priced by 0557.
- **Clear the memory when a fresh `push` of the same request pays.** Rejected:
  the bare door is undocumented and unreported; refusing it after a paid
  restart is the conservative reading, and the documented entry always works.

## Consequences

- Both bare doors are closed; the exit gate's "never reports 'no transaction'
  for a request that can still spend" holds for a fresh mount and for an
  asymmetrically tagged re-navigation. Asserted red-first (three widget rows
  and one unit cell RED on the S293 fold at `sendCount` — the money moved —
  then GREEN) plus one pinned row (a fresh `push` of the revoked request pays);
  five mutants watched by frame, all CAUGHT, registered.
- Lost, stated: a host that re-navigates bare with the same fields and a NEW
  tag after a grace negative is refused where 0557 would have paid; it
  restarts with `push`. The memory holds the revoked requests' fields for the
  process lifetime (address, amount, memo, label — the host's own values,
  never a txid; the S251 root-provider hygiene was about the previous
  identity's on-chain identifiers).
- `WalletSendEntry.push` resolves the send controller through the calling
  context's `ProviderScope` at push time and holds the object for the timer —
  a precondition that already held in practice (every wallet screen reads
  providers), now stated on the entry's doc.

## As built (S294, the diff-review fold)

- `SendController._revokedRequests` / `revokeRequest` / `isRevokedRequest`
  (`send_controller.dart`); `WalletSendEntry.push` resolves the notifier
  before `context.push` and the grace timer calls `revokeRequest(request)`
  before `reporter.revoke()` and the report (`wallet_send_entry.dart`);
  `SendScreen._noteRevocation()` has no `previous` parameter — a bare request
  reads the memory, a channel answers for itself (`send_screen.dart`);
  `sameSendRequest`: `if (tagA != null && tagB != null) return tagA == tagB;`
  else the fields (`wallet_send_request.dart`).
- Rows: `a bare-request re-navigation with the same fields and a correlationId
  the revoked request never carried cannot pay` · `… and the correlationId
  dropped after a revoke cannot pay` · `a fresh mount of the send route with
  the same bare request after a revoke cannot pay` · `a fresh push of a
  request the grace revoked is a new grant and pays (pinned: the live channel
  outranks the revoked memory)` (`wallet_send_deadline_test.dart`) · the
  `sameSendRequest` group (`wallet_send_request_test.dart`).
