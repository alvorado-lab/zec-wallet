# 0532 — Throttle the per-batch wallet-summary read to fix O(n²) deep-restore sync

- **Status:** Proposed
- **Date:** 2026-06-24
- **Links:** docs/specs/wallet-sdk.md §3.2c (the `LightdSyncEngine` increment plan) / §2.5 (authoritative balance + reorg honesty) / §3.2g iv-d-3-b-iii (the in-lock summary read) · ADR-0005 (audited crates WHOLE — Rule Zero) · replaces the never-accepted "pipelined sync + WAL" draft this file began as

## Context

Deep restore from an old wallet birthday was the founder's #1 wallet pain
("stuck at 1% / nothing happens"). On-device (S104, Pixel 10 Pro, **release**):
**~161 blocks/sec**, ~3.5 h for a ~2 M-block restore, **decelerating** as it ran
(measured 387 → 161 → 48 blk/s).

A staged on-device measurement (temporary timing instrumentation in `wallet.rs`,
durations + heights only per §5.4) found the true cause — and it was **none** of
the usual suspects:

- The release `.so` is optimized; **multicore trial-decryption already works**
  (CPU bursts to ~760 % / 8 cores). Not the bottleneck.
- **Subtree roots + chain tip are already seeded**, so the commitment tree is not
  rebuilt leaf-by-leaf. Not the bottleneck.
- **Download is minor** — `dl_ms` ≈ 7–13 % of a batch. A download↔scan pipeline
  would save almost nothing; **rejected**.
- The split that mattered, for a 9-block tip batch:
  `scan_only_ms = 2296` (upstream `scan_cached_blocks`) vs
  **`snap_ms = 6536`** — **74 % of the batch** — spent in **our own per-batch
  `progress_snapshot` → `get_wallet_summary()`** call.

`get_wallet_summary`'s scan-progress SQL (`zcash_client_sqlite`'s
`subtree_scan_progress`) `SUM`s `output_count` over the `blocks` table from a
start height with a correlated `scan_queue` `NOT EXISTS` per row — its cost
**grows with the scanned set**. We called it **once per committed batch** (for
the live percent + `spendable_ready`), so a ~2000-batch restore paid a
linearly-growing cost every batch → **O(n²)**. That is exactly the measured
deceleration. The fix is pure orchestration (our call cadence) — **no crypto,
no tree-math, Rule Zero untouched.**

## Decision

**Refresh the authoritative wallet summary at most once per
`SYNC_SUMMARY_REFRESH_BATCHES` committed batches (default 8), not every batch.**
The per-batch `report()` still fires with the carried summary and the **fresh
scanned height**, so "blocks left" decrements every batch and only the
notes-based percent lags ≤ N batches (it moves slowly anyway). Correctness is
preserved by **forcing an immediate refresh** at the points where the summary can
change meaningfully and a stale value would lie:

1. the **first** committed batch of a pass (real initial percent),
2. a **reorg** (a rewind can LOWER the percent — §2.5 honesty), and
3. reaching the **tip** (the up-to-date arm lands the UI on the exact 100 % +
   final balance).

When taken, the refresh stays in the **same db-lock critical section** as the
scan (the §3.2g in-lock invariant) — except the rare reorg/tip forced refresh,
which is a second acquire, safe under the single-writer pass contract.

## Alternatives considered

- **Cheaper height-based percent for frequent updates** — regresses the §2.5
  notes-based reorg-honest percent; rejected. Throttle the honest read instead.
- **Download↔scan pipeline** — download is 7–13 % of a batch; complexity +
  concurrency risk in the money loop for ~10 % gain. **Rejected** (was the prior
  draft's headline; the measurement killed it).
- **Patch upstream `get_wallet_summary`** — it is an audited crate (ADR-0005 /
  Rule Zero) and the cost is inherent to the progress SQL; we change our **call
  cadence**, not the crate.

## Consequences

- **Deep restore returns to O(n)**; expected ~2× or better (snap was ~74 % of a
  tip batch and grows over a restore; throttling to 1/8 reclaims ~7/8 of it).
  To be confirmed by on-device re-measurement after a reset (validation task).
- The live percent updates every ≤ N batches instead of every batch; "blocks
  left" is unchanged (per-batch). Imperceptible during a multi-hour restore.
- A reorg and the tip force a refresh, so honesty (§2.5) and the final 100 % are
  preserved; covered by the existing reorg/tip-consistency named tests + a new
  throttle regression test.
- **Secondary levers, now measurement-gated and lower priority** (separate
  tickets, not this ADR): WAL + `synchronous=NORMAL` on the wallet DB (cuts the
  DB-write share of `scan_only_ms`) and batch-size tuning. The pipeline is
  **dropped**.
- **Honest ceiling unchanged:** the remaining `scan_only_ms` (upstream
  trial-decryption + shardtree) is the Rule-Zero floor; we do not cross it.

## Test contract (spec gate 8 — maps to overview §12)

- `sync_summary_is_throttled_not_read_every_batch` — over a chain of
  > `SYNC_SUMMARY_REFRESH_BATCHES` batches, `get_wallet_summary` is invoked
  strictly fewer times than the batch count.
- `sync_summary_refreshes_on_first_batch_and_at_tip` — initial percent and the
  final 100 % + balance are exact.
- `sync_summary_refreshes_on_reorg_even_when_throttled` — a rewind that lands on
  a throttled batch still reports the POST-REWIND (lower) percent (§2.5).
- Preserved (already green): `sync_once_carries_the_committed_summary_across_the_next_batch_download`,
  `sync_once_reports_the_authoritative_summary_percent_and_spendable`,
  `snapshot_after_a_sync_reflects_the_tip_consistently_with_balance`,
  the cancel/offline/watchdog/reorg-storm named tests.
