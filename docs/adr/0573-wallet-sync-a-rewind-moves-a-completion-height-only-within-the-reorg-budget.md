# 0573 — After a rewind, a recorded completion height moves only within the reorg budget

- **Status:** Proposed, ON HOLD (2026-10-09: the maintainer chose to reconsider
  after the design review corrected the cost; issue #1 stays open)
- **Date:** 2026-10-09
- **Links:** docs/specs/wallet-sdk.md §3.2 "REW-B" · public issue #1 ·
  docs/intent/fork-discovery.md · docs/plan/rew-b-reorg-budget-floor.md (the plan
  and the review record) · docs/plan/production-readiness-phase-1.md §4d Q4,
  §4e-R2(a) (C15), §4g · `constants::REORG_MAX_BLOCKS`

## Context

The wallet records, per pool, the heights at which the light server says each
note-commitment subtree completed. Upstream's spendability decisions read those
heights. After a continuity error the wallet rewinds a fixed distance and buys one
relaxed pass in which the bind refuses only an UPWARD move of a recorded height.
An endpoint that induces a ten-block rewind can therefore move a height down by
any amount; repeated, that walks a height thousands of blocks down. The effect is a
wrong spendability reading (INC-020's class), never theft. The plant
`a_rewind_does_not_license_a_move_no_reorg_could_produce` demonstrates it and
has been red on purpose since S279; the README and issue #1 tell users to hold
small amounts until it is fixed.

The intent file proposed fork-point discovery as the fix. Measured at the code on
2026-10-09, it is not one: the endpoint chooses the block hashes it serves below
the mismatch, so it can always present a fork with no common block, and the
intent's own fallback for "depth unknown" was today's direction floor, which
accepts the move. C15, ruled honest in §4e-R2(a), models a fork thousands of
blocks below a tip the same endpoint reported.

## Proposal

After a rewind, a served completion height `s` for an index recorded at `r` is
accepted iff `s == r` or `A − REORG_MAX_BLOCKS < s <= r`, where `A` anchors the
chain position below which a reorg inside the budget cannot reach. Soundness: a
reorg of at most `R` blocks from a tip at or above `A` forks above `A − R`; a
subtree completed at or below the fork keeps its height, and one completed above
it completes above it on the new branch too. The `s <= r` half is not a reorg
property (a thinner branch can raise a height honestly); it is the inherited
defence against erasing the tree cap.

## Design review, 2026-10-09 (security ∥ arch ∥ crypto)

The first draft (`A` = the highest completion height ever accepted, per pool)
was not safe to build:

- **The rewind deletes the rows the rule binds.** A checkpointed truncation
  re-puts shards from the landing point up without heights, including the top
  index; the rule skips an index with no row, so a compressed re-serve of it is
  accepted (crypto BLOCKER-1).
- **Two-rewind walks.** A row the bracket ledger refutes skips the rule, and a
  BIND-1 withdrawal or a correction from an unverifiable anchor could lower the
  anchor; either reopens the plant's 7,749-block move over two induced rewinds
  (security B1, B2; arch MAJOR-1).
- **The anchor was endpoint-attested** (a first-contact height bounded only by
  the endpoint's tip), and a raise after the put on the separate aux connection
  left it stale on the permissive side (security H2, H3; arch MAJOR-2).
- **The stated cost was wrong.** The draft said a reorg deeper than 100 blocks
  already needs a rescan. It does not: `PRUNING_DEPTH` counts checkpoints, not
  blocks, upstream lets a truncation go below them when no witness is lost, and
  the reorg arm's per-pass cap bounds nothing across passes, so today the wallet
  follows such a reorg. Zebra's own rollback window is 1,000 blocks
  (`MAX_BLOCK_REORG_HEIGHT`, a local policy).

A revised design (v2) answers every finding: the anchor is the highest block the
wallet itself has scanned, persisted and cleared only by a rescan; a rewind
detected more than `R` below it relaxes nothing (it is itself proof of a fork
deeper than the budget); an aux copy of the accepted per-index heights, written
before the put, binds rows the rewind deleted; the floor applies to refuted rows
too; only a BIND-1 correction from a verified anchor lowers a stored height; and
`REORG_MAX_BLOCKS <= PRUNING_DEPTH` is pinned by a const assertion.

## Consequence of v2 that is not free

An honest reorg deeper than 100 blocks, which the wallet follows today across
passes, would leave it stuck until a rescan or a server switch (Ironwood
degraded, Sapling and Orchard stopped). The maintainer put the item on hold on
2026-10-09 to reconsider that cost; the alternative named then was v2 plus an
automatic rescan after such a reorg, which needs its own design.

## Alternatives considered

- **Fork-point discovery alone.** Does not close the attack (above).
- **A move-size threshold between 5,149 and 7,749.** A guess, ruled unsound in
  §4d Q4. The proposal bounds a chain position after a rewind, not a distance.
- **A second server corroborating any post-rewind change.** Needs the host to
  configure two servers (ADR-0568), fails closed when the second is down, and
  both may share an operator.
- **Refusing every post-rewind move above the scanned frontier.** Deadlocks
  Sapling and Orchard: the refusal stops the pass before the scan that would
  correct the record runs.
- **The endpoint's reported tip as the anchor.** Endpoint-attested; the bind's
  design forbids it.
