# 0562 — Wallet send report: a tagged report names the amount it paid its one recipient

- **Status:** Accepted (stage S2 `amount`, FR-46, build B's fold, S297,
  2026-09-23; `docs/plan/stage-2-the-hosts-lifecycle.md` §3.5f)
- **Date:** 2026-09-23
- **Links:** FR-46 and FR-26 (`docs/handoff/host-feature-requests.md`) ·
  ADR-0558 (a one-sided correlation id is not a new request) · the design
  pass on §3.5f (S297, §5) · `wallet_send_report.dart`'s §5.4 payload rule

## Context

FR-26's send report deliberately carries identifiers and counts and "never
the recipient, the amount, the memo" (§5.4). A host that writes a signed
payment record into a chat (Relim's send-to-contact) must state what the
recipient receives, and had only the amount it pushed — which the user can
edit on the send screen before signing, so it may be false by the time a
transaction exists.

## Decision

`WalletSendTransactionCreated` gains `recipientAmountZat` (`int?`), set ONLY
on a report whose push carried a `correlationId`: the total paid to the
send's one recipient address across every transaction the send produced, AS
SIGNED (read from the proposal whose id was passed to `send`), the fee
excluded. It is computed in the core's `send::summarize`, where the figures
the user confirms are already summed, as `SendProposal.singleRecipientZat`:
`null` when the payments name more than one address by `ZcashAddress`
equality (a unified address and one of its own receivers are two), and for a
shield. It states what was signed, never arrival; the report's `motion` and
the wallet's own surfaces remain the authority on delivery.

This is a deliberate exception to §5.4 for TAGGED reports only; the
report's doc says so.

## Alternatives considered

- **Per-transaction amounts.** The host renders one statement per send; a
  per-transaction list invites it to sum and get a TEX two-step wrong.
- **A partial sum for two recipients.** A wrong figure in a money record is
  worse than none; the host's semantics accept `null` (its E15 state).
- **Amount on every report.** An untagged push has no join the host can
  trust; the tag is what makes the figure attributable (ADR-0558).
- **A sibling read keyed by `correlationId`.** A second surface for one
  value, and the report already crosses at the moment the figure is final.

## Consequences

- The bridge `SendProposal` gains `singleRecipientZat` (the ABI 4 → 5 bump
  build B pays); the report DTO gains its field; untagged hosts see nothing
  new.
- A host must not read the amount's presence as delivery: a partly
  broadcast ZIP-320 two-step reports the signed total while its forward leg
  can still strand.
