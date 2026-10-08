# 0570 — Wallet sync's incremental root fetch turns itself off for a server that cannot serve it

- **Status:** Accepted (2026-10-03), together with ADR-0569, at the S15-F1 built diff's review.
  Amends ADR-0569's Decision.
- **Date:** 2026-10-03
- **Links:** [ADR-0569](0569-wallet-sync-fetches-only-the-subtree-roots-it-does-not-hold.md)
  (amended) · `docs/plan/s15-f1-subtree-roots-fetch-only-what-is-new.md` §3.3 step 2 and §11
  (the built-diff review and the final fold) · `docs/specs/wallet-sdk.md` iv-d-1 ·
  `SUBTREE_ROOTS_DROPS_OFF` (`sdk/zec-wallet-core/src/constants.rs`)

## Context

ADR-0569 fetches each pool's subtree roots from a non-zero `start_index` once the pool is
verified in the session. Its Decision says a refused, ignored or mis-served non-zero start
retries from 0 in the same pass and keeps that pool on full fetches for the rest of the session,
and that a single dropped stream does not. It says nothing about two other ways a server can
fail only at a non-zero start, and the built-diff review (security LOW-1 and LOW-2, crypto LOW)
found both:

- **A timeout.** A timeout fails the pass, as it always has, with no retry. An endpoint that
  answers from 0 and stalls only on a non-zero start would then fail every pass of the session:
  a wedge.
- **Repeated drops.** One transport fault is a reset (over Tor, possibly one circuit) and is
  retried from 0 in the same pass. A server that resets every non-zero start would make every
  pass cost two streams.

The review also corrected one word of 0569: an IGNORED start (the server serves from 0
whatever was asked) is not retried; it is re-classified as the full fetch it already is.

## Decision

A timeout on a non-zero start, at the open or mid-drain, still fails the pass with no retry,
and also keeps that pool on full fetches for the rest of the session. Two dropped non-zero
starts in a row for one pool do the same (`SUBTREE_ROOTS_DROPS_OFF` = 2). A drop counts only
when the same pass's retry from 0 was served, and the count resets only when a pass writes the
pool from a non-zero start. A refused or mis-served start already turned it off, and an ignored
one is taken as a full fetch and turns it off the same way.

## Alternatives considered

- **Retry a timeout from 0 in the same pass.** A timeout is already the slowest fault (the
  per-message bound); a second stream after it doubles the worst case. Failing the pass and
  going full next time costs one pass, once.
- **Turn incremental off on the first drop.** Over Tor a single reset is ordinary, and one
  would cost the session its saving for nothing.
- **Count every drop, and reset on any non-zero start drained whole.** A drop whose retry also
  fails says the path is down, not that the server resets non-zero starts; and a serve drained
  whole but refused by the bind is not evidence the server serves incrementally. Both readings
  would let the count say something it did not measure.

## Consequences

- A server that times out a non-zero start costs one failed pass, and one that resets EVERY
  non-zero start costs at most two two-stream passes (counted drops); after either, today's one
  full stream per pass for the session. Not counted, by design: drops broken up by a written
  incremental serve (a link that only sometimes drops keeps the saving), and a drop whose retry
  from 0 also fails (the path, not the server, is down; that pass fails as it would at start 0).
- The memo gains one in-memory counter per pool (`PoolMemo::consecutive_drops`); still nothing
  is persisted, and a new session re-measures its server.
- Residual, accepted: a pass cancelled while its non-zero-start stream is stalled loses the
  timeout signal, so the next pass tries the same start. Performance only and bounded: nothing
  is written, and the time- and pass-bounded full verification of ADR-0569 still runs.
