# 0536 — Incoming-funds arrival is a MINIMAL counts+heights event stream over the scan edge; attribution stays a pull

- **Status:** Accepted — **Decision 4 amended in part by [ADR-0539](0539-memorefresh-means-an-enhancement-pass-changed-a-row.md)** (`MemoRefresh` now also fires on a status write). Every other decision here stands.
- **Date:** 2026-07-19
- **Extends:** FR-1 (the incoming-payment residual — tx history + memo-content read already shipped, S106–S109) shaped by FR-11's constraints (docs/handoff/host-feature-requests.md). Does not supersede any ADR.
- **Links:** docs/specs/wallet-sdk.md §2.5 (DTOs) · §3.3 (streams) · §5.4 (NEVER-log list) · ADR-0018 (background OS entry: NSE never runs wallet code) · ADR-0013 (small SDK surface, keys in Rust) · ADR-0533 (the `SyncStatus` stream + spend-before-sync semantics) · ADR-0534 (rescan rebuilds the data DB — history replays) · ADR-0532 (summary-read throttle: deep restore is O(n²)-sensitive). Pinned crates: `zcash_client_backend 0.23.0` / `zcash_client_sqlite 0.21.0`.

## Context

FR-1's remaining ask is a dedicated "funds arrived" signal: today a host can only
poll `snapshot()` or infer arrival from the coarse `SyncStatus` edges
(`becameSpendable` / `reachedTip`), which is exactly what `zec_wallet_ui` does at
its activity-refresh listener. The spec reserved a rich per-tx surface for this
(`Stream<IncomingTransaction> incoming({sinceCursor})`, §2.5/§3.3) before FR-11
landed. FR-11 reshapes the requirement:

- **Notification paths are hostile to content.** Relim (and any host) may forward
  the event toward OS notification infrastructure. iOS notification content is
  archived by the OS, mirrored to paired devices, and readable on the lock screen;
  §5.4 puts txids, amounts, memos, and addresses on the NEVER-log list precisely
  because a durable {this device ↔ this txid/amount} record is the association the
  shielded protocol exists to prevent. A payload that needs host-side sanitizing
  before forwarding WILL eventually be forwarded unsanitized.
- **Verification is main-app-only.** FR-11: never verify payments in the NSE
  (24 MB jetsam); surface the unlock as a "next time you open the app" event.
  ADR-0018's `nse-slim` profile has no wallet listeners, and the wallet stays
  excluded from `relim-ext`. So the SDK stream cannot and need not wake anything —
  it runs only while the app (and therefore sync) runs.
- **History replays.** A rescan (ADR-0534) or seed-restore re-detects every
  historical receive. FR-10 (host-owned) exists because the host must keep its own
  consumed-txid ledger for exactly-once semantics; an SDK event stream that
  pretends to be exactly-once would be lying.
- **Deep restore is performance-critical.** A 100k-block restore commits thousands
  of batches; ADR-0532 already throttles the summary read because an every-batch
  O(scanned-set) query measured ~74 % of batch time. An every-batch detection
  query would repeat that mistake.
- **What the engine already gives us, free:** `scan_cached_blocks` returns a
  `ScanSummary` (`scanned_range`, `received_sapling_note_count`,
  `received_orchard_note_count`) that `sync::scan_batch` currently discards. And
  librustzcash's audited `v_transactions` view is already the arrival ledger — a
  received payment is a positive `account_balance_delta` row. There is no exposed
  monotonic rowid; the shipped history cursor is a `(mined_height, txid)` keyset,
  and `scanned_tip` (MAX(blocks.height)) is the natural watermark.

## Decision

Ship the FR-1 residual as **`watchIncomingFunds({String? sinceCursor}) →
Stream<IncomingFundsEvent>`** — a second core `tokio::watch` stream following the
`watchSyncStatus` spine (sink trait + detached pump + FRB `StreamSink` adapter),
whose payload is **counts, heights, and an opaque cursor ONLY**:

```
IncomingFundsEvent {
  kind:            Replay | Live | MemoRefresh,
  newTxCount:      u32,      // txs in this edge (advisory; coalescible)
  totalTxDetected: u64,      // monotonic since wallet-open (coalesce-proof)
  spanFromHeight:  Option<u32>,  // height span of the detections (as built: u32)
  spanToHeight:    Option<u32>,
  cursor:          String,   // opaque versioned watermark ("w1:<height>" — a
                             // DISTINCT prefix from the HistoryPage keyset
                             // token's "v1:", so a cross-API mix-up rejects
                             // typed instead of mis-parsing)
}
```

No txid, no amount, no memo, no address — the payload is **§5.4-safe by
construction**: a host can log it, forward it to a notification path, or render it
verbatim without a sanitizing step. Attribution (which tx, how much, from whom via
the memo) stays a **pull** through the shipped surface (`transactions()`,
`transactionMemos()`), in the main app, per FR-11 and per the FR-1 audit
correction (attribution comes ONLY from the recipient's own decrypted memo).

**Recorded delegated call (founder may veto, additive to reverse):** the amount
does NOT ride the event. Adding `netAmountZat` later is a non-breaking field
addition if a host demonstrates need; removing it later would be a break. The
lowest-claim payload ships first.

Mechanics, in the order the money moves:

**S210 watermark-honesty amendment (post-ship review):** every stamped
cursor claims only the **fully-scanned frontier** (min over pending
`scan_queue` ranges − 1, clamped by the scan max) — never
`MAX(blocks.height)`, which OVER-claims under the plan's near-tip-first
priority order during a restore; and a per-open **sticky fault floor**
(min `lo − 1` across conservative-fired detect faults) clamps every later
Live stamp, so no subsequent event can claim a span whose diff failed.
The replay's monotonic baseline is captured at subscribe time (before the
replay read), keeping `totalTxDetected` deltas duplicate-or-visible under
edge coalescing.

1. **Detect (the cheap gate).** `sync::scan_batch` stops discarding
   `ScanSummary`. In the `BatchAction::Advance` commit path, ONLY when
   `received_sapling_note_count + received_orchard_note_count > 0` does the core
   run the truth diff — count `v_transactions` rows with positive
   `account_balance_delta` mined inside the batch's scanned span. **As built
   (amended at implementation, same day):** the diff runs AFTER the commit on
   the **aux read connection** — the exact production history-read path — not
   inside the scan's db-lock closure: the engine `WalletDb` exposes no raw-SQL
   door for an in-lock read, and extending the scan's critical section for a
   read would invert the iv-d-2a lock-split discipline. The post-commit read is
   race-free for THIS batch (its blocks are durably committed before the diff
   runs); a BUSY against the NEXT batch's writer degrades to the
   conservative-fire below. Best-effort: a diff read fault emits a count-0
   event WITH the span set ("pull to confirm" — never a silent miss) and never
   fails the sync loop. Empty batches (the overwhelming majority of a deep
   restore) cost zero extra queries. Received-note counts over-approximate
   (change/self-send notes count), so the diff is the filter: a change-only
   batch finds zero positive rows and emits nothing. **Transparent scope (as
   built):** the engine's UTXO refresh counts re-puts of already-known UTXOs,
   so it cannot cheaply signal "previously-unseen" — v1 `Live` spans are
   therefore SHIELDED-scan-edge only; a transparent receive surfaces via the
   `MemoRefresh` nudge (its tx rides the enhancement pass) + the ordinary
   balance/history pulls, and IS covered by `Replay` catch-ups (the diff sees
   its mined `v_transactions` row) — PROVIDED the host's cursor has not
   already passed its mined height when the wallet learns of it (a
   late-learned below-cursor transparent receive surfaces via the
   balance/history pulls only; the S210 honesty qualifier). A dedicated transparent-arrival span is
   the named follow-up below.
2. **Channel (coalesce-lossless).** One `tokio::sync::watch` per wallet holding
   the latest event with **cumulative** `totalTxDetected`. Latest-wins coalescing
   can drop intermediate `newTxCount`/span values but never the cumulative total —
   a slow subscriber that misses edges still sees the correct delta and reacts the
   only way the contract asks: pull history. Delivery is **at-least-once**;
   duplicates are honest (a reorg re-mining a tx IS it arriving again).
3. **Replay on subscribe.** The per-subscriber pump prepends ONE `Replay` event
   computed from history: `newTxCount` = positive-delta txs with
   `mined_height > cursor`, span = their height range, then mirrors the live
   channel. `sinceCursor == null` ⇒ a baseline `Replay` with count 0 and the
   current watermark, so every subscriber learns a cursor to persist. A missed
   event is therefore always recoverable from history — the cursor/ack loop the
   task requires.
4. **MemoRefresh.** After an enhancement pass stores decrypted tx data (memos
   arrive AFTER scan detection), the core emits a `MemoRefresh` (count 0,
   unchanged totals): "attribution data improved — pull again if you care."
   *(Decision 4's firing condition is AMENDED by ADR-0539 — `MemoRefresh` now
   also fires on a status write. Read that ADR before relying on this
   paragraph.)*
5. **Rescan/restore honesty — no core-side suppression.** During a rescan or
   restore, `Live` events fire for historical receives with OLD spans. The SDK
   does not guess which arrivals the user has already seen — the host has strictly
   better information (its persisted ack watermark + FR-10 consumed-txid ledger).
   Contract, stated in the API docs: **the stream is a freshness signal, not an
   accounting or unlock ledger**; notification-driving hosts MUST gate on
   `spanToHeight > their persisted watermark` AND their FR-10 ledger.
6. **Observability.** One new tracing span, added to the §5.4 allowlist:
   `wallet.incoming_detect {span_from, span_to, count, outcome}` — heights +
   counts + outcome only, enforced by the existing capture-layer guard.

## Alternatives considered

- **The rich per-tx stream the spec reserved (`IncomingTransaction { summary,
  memos }`).** Lost: every field except heights is NEVER-log material, so any host
  logging or forwarding the event needs a sanitizing layer the SDK cannot enforce;
  a restore would stream thousands of per-tx events (buffering or dropping, both
  bad); and it duplicates the pull surface that already shipped. FR-11 forces the
  host into the main app for verification anyway — the rich payload buys nothing.
- **Amount rides the event.** Lost (recorded as the delegated call above):
  violates §5.4-by-construction, leaks wealth data into OS notification
  infrastructure, and is the one direction that can't be walked back compatibly.
- **`tokio::sync::broadcast` with a buffer.** Lost: introduces `Lagged` handling
  and buffer sizing for no benefit — the host's reaction to ANY event is "pull",
  so cumulative-total coalescing over `watch` is semantically lossless and reuses
  the proven no-lock-pin pump pattern verbatim.
- **Core-side rescan suppression (mute events below the pre-rescan tip).** Lost:
  hidden magic; the pre-rescan tip is unknowable across a process death mid-restore;
  and it double-claims FR-10's job with worse information than the host holds.
- **A dedicated event/ack table in the wallet DB.** Lost: `v_transactions` IS the
  arrival ledger; a second table adds migrations and drift risk to reproduce what
  a watermark query answers. FR-10 deliberately keeps the exactly-once ledger
  host-side.

## Consequences

- Easier: hosts get a real "funds arrived" hook (FR-1 closes); `zec_wallet_ui`
  retires its coarse-edge-only activity refresh — new receives surface on the
  event edge instead of waiting for `reachedTip`; e2e can prove the whole spine
  via replay on the funded device wallet with ZERO money movement (subscribe with
  a low cursor → replay count + watermark, no real send needed).
- Harder / frozen: hosts wanting amount-in-notification must pull first (frozen
  until the founder reverses the delegated call); `newTxCount`/spans are advisory
  under coalescing — `totalTxDetected` and the pull are the truth; the cursor is a
  height watermark, so a deep reorg spanning a process death re-fires events (the
  safe direction — never silently misses; the host watermark absorbs it).
- Follow-ups: the founder call on amount stands open as a veto, not a blocker;
  a dedicated transparent-arrival `Live` span needs an engine-side
  new-vs-known UTXO signal (upstream FR territory — meanwhile the transparent
  leg rides `MemoRefresh` + `Replay`, per the as-built amendment above); the
  full in-crate sync→note→event pipeline pin needs a note-bearing `FakeChain`
  (today the truth diff is pinned over real scanned rows at the history layer,
  the emit/pump/replay over a real wallet, and the engine-row path via the
  transparent-refresh fixture — the sync_once wiring itself is pinned by the
  empty-sync-silence ordering test); §5.4 allowlist gains the
  `wallet.incoming_detect` span (same commit).
