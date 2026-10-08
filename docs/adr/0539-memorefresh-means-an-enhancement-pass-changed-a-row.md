# 0539 — Widen `MemoRefresh` to mean "an enhancement pass changed how a row reads"

- **Status:** Accepted
- **Date:** 2026-09-06
- **Links:** amends [ADR-0536](0536-incoming-funds-minimal-event-stream-over-scan-edge.md)
  Decision 4 (supersedes that paragraph only; the rest of ADR-0536 stands) ·
  `docs/specs/wallet-sdk.md` §3.3 · `evals/incidents.tsv` INC-009 ·
  `docs/handoff/s254-resume.md`

## Context

ADR-0536 Decision 4 defined one cause for the `MemoRefresh` event: *"After an
enhancement pass stores decrypted tx data … attribution data improved — pull
again if you care."* The core emitted it gated on `enhanced > 0`.

INC-009 (fixed in `982f1fb8`) gave the enhancement pass a **second** thing it
can change. Upstream models two txid-keyed request kinds: `Enhancement` is
answered with `decrypt_and_store_transaction`, and `GetStatus` with
`WalletWrite::set_transaction_status`. The loop had folded them and answered
both with the decryptor, which writes no mined height — so two confirmed
on-chain sends rendered as *"expired … still yours to spend."* for six weeks.

A status write stores **no new decryptable data**, but it is exactly what flips
a send out of "expired" and gives it a confirmation count. If the event stayed
gated on `enhanced > 0`, the corrected row would sit stale on the host until
some unrelated event prompted a re-pull — the payoff of the fix would be
invisible for an arbitrary interval. So the gate widened to
`enhanced > 0 || status_set > 0`, and Decision 4's stated meaning no longer
matched the shipped behaviour.

The S254 review caught the gap as a governance defect rather than a behavioural
one: the reference host (`wallet_screen.dart`) treats every event kind as
"advisory — pull the activity list", so nothing misbehaves today. The risk is a
future reader — or a second host — trusting a frozen decision doc that the code
had quietly outgrown.

## Decision

**`MemoRefresh` means "an enhancement pass changed how existing rows read".**
It has two causes, and a host must not assume either one:

1. decrypted transaction data was stored (memos arrive after scan detection), so
   attribution may have improved; or
2. a `GetStatus` request was answered with a chain status, so a send may have
   flipped out of "expired" — no new data to decrypt, a materially different
   rendering.

A host must **not** treat the kind as proof that new memo data exists. The
reaction that is correct under both causes is the one ADR-0536 already
prescribes: pull again. Payload discipline is unchanged — count 0, no
txid/amount/memo/address, §5.4-safe by construction.

## Alternatives considered

- **A new event kind for the status cause.** Rejected: it is a breaking change
  to a stream contract for zero host benefit, since every current consumer
  reacts identically. ADR-0536's own premise is that the stream stays MINIMAL
  and the host re-pulls; a second kind that produces the same reaction adds a
  variant hosts must handle and teaches nothing.
- **Leave the gate at `enhanced > 0`.** Rejected: it makes the INC-009 fix
  silent. The corrected row is the whole point, and a user looking at a send
  that says "expired" would keep seeing it until an unrelated event landed.
- **Amend ADR-0536 in place.** Rejected on the founder's rule (S254): an ADR may
  be edited while it is still uncommitted from the session that wrote it; once
  committed, a change needs a new ADR that supersedes. ADR-0536 shipped long
  before this. It now carries a one-line pointer here, which is a reference, not
  a restatement.

## Consequences

- **Easier:** the INC-009 fix is visible to a user without waiting on an
  unrelated event. The event's meaning is now stated in terms of the effect a
  host cares about (a row reads differently) rather than the mechanism.
- **Harder:** a host that wanted "new memo data exists" no longer has a signal
  for it. None asks for one today; if one does, it is a new event kind and a new
  ADR, not a narrowing of this one.
- **Frozen:** the two causes above are the complete set for `MemoRefresh`. A
  third cause needs an ADR.
- **Follow-up, unrelated to this decision but found beside it:** ADR-0536 also
  carries an inline *"S210 watermark-honesty amendment (post-ship review)"*
  block, written under the same practice this ADR was told not to use. It is
  left in place — removing it would destroy the record — but it is a known
  exception to the append-only rule, not a precedent to copy.
- The `state.rs` `IncomingFundsEventKind::MemoRefresh` doc comment and
  `docs/specs/wallet-sdk.md` §3.3 both state the widened meaning and point here.
