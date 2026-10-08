# 0533 — Sync UX: mandatory (spend-before-sync) vs quiet historical backfill

- **Status:** Accepted (implemented per the UPDATE block, NOT the original Decision)
- **Date:** 2026-06-24
- **Links:** docs/specs/wallet-sdk.md §2.5 (authoritative balance) / §3.2g (the live progress surface) / §3.3 (the wallet surface) · ADR-0532 (the per-batch summary throttle — complements this)

## Implementation (shipped S105, Dart-only)

`app/relim-flutter/lib/features/wallet/`:
- `wallet_health.dart` — `walletBadgeLevel(status, stale) → {ok, caution, error}`,
  the pure traffic-light rule (GREEN usable+fresh / YELLOW limited-or-stale /
  RED can't-sync). Truth-tabled in `wallet_health_test.dart`.
- `wallet_screen.dart` — `_SyncBadge` (compact, persistent, color-coded; the
  multi-year "blocks left" countdown DROPPED, bar+percent kept); `_SendButton`
  gated on `spendableZat > 0` with an honest disabled-reason; a `ref.listen`
  edge-refresh of the cold snapshot when funds first become spendable / the tip
  is reached (so the balance appears mid-sync without a manual resume — done in
  the screen, NOT `SyncStatusNotifier`, to leave the lifecycle "no wasteful
  re-fetch" counts untouched).

NO SDK / FFI / `recover_until` change (the UPDATE block verified none was
needed — the FFI already exposes `spendable_ready` + the spendable amount).

## ⓘ UPDATE (2026-06-24 — source-level re-research RESOLVED the open question; this CORRECTS the Decision/Context below)

Verified against Zashi's iOS + Android SDKs and app source + librustzcash (S104
re-research report in transcript). Corrections — **read these, not the original
Decision, when implementing:**

1. **Zashi's progress bar is a SINGLE COMBINED ratio**
   `(scan.num + recovery.num) / (scan.denom + recovery.denom)`, checked-div →
   `1.0` at `0/0` (`ScanAction.swift:80-99`, `CompactBlockProcessor.kt:2470`).
   That is EXACTLY our existing S103 `progress_percent` (commit dd02ac9) — so
   that choice was **RIGHT, keep it**; do NOT split into two bars and do NOT
   "retire the combined percent." (Retract the original "S103 fights the design"
   framing below.)
2. **Keep `recover_until = None` — do NOT set it to tip.** Verified: the
   displayed combined bar is identical either way (the recovery denominator —
   years of notes — dwarfs the scan denominator). `recover_until = tip` only
   adds an early `scan-complete` bool that is **misleading** (true from `0/0` at
   restore start — Zashi must AND it with `spendable > 0`). So there is **NO
   account-import change** (retract original Decision #1) and **no scan/recovery
   UI split** (retract #2's "mandatory vs backfill two-window" model).
3. **Gate balance + Send on `spendable_value > 0` + the amount check**
   (`shieldedBalance = spendableValue`; SendForm `.disabled(insufficientFunds)`)
   — NEVER on the bar or a scan-complete bool. `spendable_value` is independent
   of `recover_until` (per-note: scanned shard + confirmed + witnessable,
   `wallet.rs:2656`). We already expose `spendable_ready`; also surface the
   spendable AMOUNT + `value_pending_spendability` (maturing) + total distinctly.
4. **The persistent color badge (Decision #4) STANDS and is OUR improvement** —
   Zashi has NO color badge (fixed purple, icon+copy; dismisses on synced). Our
   green/yellow/red persistent badge + the kept progress bar is strictly more
   honest. Keep it.
5. **Where we beat Zashi:** Zashi throttles progress updates 5× because each
   crosses the native↔Rust FFI (`getWalletSummary`, their TODO #1353); we run
   sync fully in Rust and push every batch over one `StreamSink` at zero
   marshaling cost → smoother bar; plus the real color badge + correct money
   semantics (spendable / pending / total shown distinctly).

**Net corrected plan (much simpler — mostly DART, no SDK progress-math/import
change):** keep the combined progress bar + `recover_until = None`; gate
balance/Send on `spendable_value`; add the persistent compact color badge; show
spendable / pending / total distinctly; expose the spendable AMOUNT over FFI if
not already. Accept + implement next session (#228).

## Context (original — see the UPDATE above for the corrected decisions)

Founder directive (S104): "be on par with ZODL — the user sees balance + can
transact ASAP after install; mandatory sync is clearly shown; the historical
backfill runs with less-noticeable indication. The scan badge must also be
shorter / take less space." On-device our restore shows ONE bar crawling at a
low % through the multi-year history (the sandblasting spam era at ~14 blk/s,
8-core-saturated — inherently slow for ZODL too; an ECC dev confirms Zashi
"processes 100 blocks at a time in sandblasted ranges").

Research (librustzcash source + Zashi/ECC SDK + blog/forum, both reports in the
S104 transcript) found the library is **designed for a two-phase UX we are not
using**:

- `WalletSummary::progress()` splits into **`scan()`** (recovery-height → tip =
  the recent / near-tip "mandatory" window) and **`recovery()`** (birthday →
  recovery-height = the historical backfill). `is_synced()` = 100% INCLUDING
  backfill — it must NOT gate Send.
- **Spend-before-sync:** `spendable_value()` becomes > 0 as soon as the
  *near-tip* commitment-tree shards are scanned + confirmations clear —
  independent of the historic backfill. `ScanPriority` (`Verify` > `ChainTip` >
  `FoundNote` > … > `Historic`) makes `suggest_scan_ranges` scan the near-tip
  ranges FIRST (we already consume it in priority order, `sync.rs:next_batch`).
- Zashi's SDK exposes `syncing(progress, areFundsSpendable)` and gates balance +
  Send on the **spendable flag, never on 100%**; the long backfill rides a
  subtle tappable status banner.

Our two divergences: (1) we import with **`recover_until = None`**
(`account.rs:245`), so `recovery()` is `None` and `scan()` spans the ENTIRE
birthday→tip — there is no mandatory/backfill split, hence the one scary
crawling bar; (2) the S103 "combine the two windows into one percent" choice
actively fights the design (it hides a usable wallet behind backfill). The
honest truth: old-seed restore is slow for everyone; ZODL's advantage is **UX
sequencing**, not raw speed.

## Decision

Adopt the library's two-phase model end to end:

1. **Set `recover_until = live chain tip at import`** (provision already fetches
   it, `provision.rs:109`) so `Progress` splits into a mandatory `scan()` window
   and a backfill `recovery()` window. Thread it through
   `birthday_from_treestate(treestate, recover_until)`.
2. **Expose three signals** from the SDK `ProgressSnapshot` instead of one
   percent: `mandatory_percent` (`scan()`), `backfill_percent: Option` (`recovery()`),
   and `spendable_ready` (kept). The combined `percent` is removed from the UI
   contract (the FFI/Dart consume the split); the pure `progress_percent`
   combiner and its test are retired.
3. **Gate balance + Send on `spendable_ready`** (and the mandatory window being
   complete), NEVER on `is_synced()`/100%.
4. **A persistent, compact, color-coded status badge** (`wallet_screen`) — it
   STAYS in the UI at all times (not hide-on-synced) and communicates state by
   color (traffic-light), short text, and a quiet sub-line:
   - **GREEN — healthy / "Up to date":** mandatory sync complete, balance fresh,
     Send/Swap available. If the historical backfill is still running it shows a
     quiet sub-line "Restoring history… X%" but STAYS GREEN — the backfill does
     not limit operations and does not make the balance stale (it only makes it
     *grow* as old notes are found).
   - **YELLOW — limited / stale:** EITHER the mandatory phase is still running so
     operations are limited (`spendable_ready` false / can't Send yet → "Syncing…"),
     OR the balance is stale (sync stalled / offline / last reload failed →
     "Balance may be out of date"). Yellow = "usable with a caveat, here's the
     caveat."
   - **RED — broken (proposed extension):** cannot reach the network at all /
     hard sync error → "Can't sync" with a retry. (Founder specified green/yellow;
     red is the honest-degradation arm for a total failure — confirm at design.)
     **AMENDED #399 (2026-07-19):** "cannot reach the network" moved to YELLOW —
     `Stalled{endpointUnreachable}` is the spec §3.2a "(normal offline)" family
     (airplane mode is deliberate and routine; the core cannot tell it from a
     down server, and it is the lowest-claim fallback for unmapped transients).
     RED now means a hard stall that needs the user: Tor required-but-down
     (fail-closed privacy is never calm), storage full, a local store fault, or
     an unknown stall.
   The single badge replaces the tall card; this is where the "shorter / less
   space" requirement lands. It also removes the multi-year "blocks left" number
   from the headline (the source of the founder's "stuck then jumps once per few
   minutes" — per-batch granularity in the slow backfill, now demoted to the
   green badge's quiet sub-line). The badge is tappable for detail (Zashi
   parity).
5. **iOS:** the mandatory phase finishes in a foreground session; the backfill
   rides opportunistic `BGProcessingTask` (ADR-0018 native background entry,
   host-app), best-effort — the same posture as Zashi.

## Alternatives considered

- **Keep one combined percent (S103)** — rejected; it's the bug. It conflates a
  usable wallet with a multi-year backfill and crawls near 0% during restore.
- **Gate on `is_synced()`/100%** — rejected; the user would wait the entire
  backfill before seeing balance/Send. Upstream explicitly says don't.
- **Make the backfill fast enough to not need the split** — not achievable; the
  sandblasting trial-decryption is at the hardware ceiling (8-core-saturated),
  same as ZODL. UX sequencing is the actual lever.

## Consequences

- A freshly-restored wallet shows balance + enables Send as soon as a recent
  note is found (spend-before-sync); a wallet whose only funds are OLD still
  surfaces them only when the backfill reaches them — same as Zashi, and honest.
- `recover_until = tip` puts the wallet in upstream "recovery mode" until
  birthday→tip is scanned. **Verify (test owed):** recovery mode must NOT
  suppress `spendable_value()` for near-tip notes (else spend-before-sync
  breaks). This is the load-bearing semantic to pin before merge.
- The `birthday` stays immutable after first import (existing invariant);
  `recover_until` is likewise set once at first import.
- FFI surface (`SyncStatus`) gains the split fields; Dart `wallet_screen`
  re-gates balance/Send and restructures the badge. Money-display change →
  multi-angle review (security ∥ arch ∥ code reviewer).
- Complements ADR-0532: the throttle keeps the (now backfill-phase) summary read
  O(n); this ADR decides what to DISPLAY from it.

## Test contract (gate 8) — as implemented (supersedes the retracted Rust list)

The original contract below the line pinned the rejected two-window/`recover_until=tip`
model; the UPDATE block retracted it (no SDK change), so the shipped contract is
Dart-side. NO new Rust tests are owed (the `spendable_ready` / spendable-amount
FFI surface is unchanged and already covered).

`test/features/wallet/wallet_health_test.dart` — the gate-8 truth table:
- every `SyncStatus` arm × the `stale` flag → the exact `WalletBadgeLevel`
  (UpToDate→ok; UpToDate+stale→caution; Scanning+spendableReady→ok;
  Scanning+!spendableReady→caution; Scanning+spendableReady+stale→caution;
  Connecting/Idle/Offline→caution; Stalled(chainReorg)→caution;
  Stalled(endpointUnreachable)→caution (#399 — the normal-offline demotion);
  every other Stalled→error; Unknown→caution; hard-stall stays error even
  when stale).

`test/features/wallet/wallet_screen_test.dart`:
- Send ENABLED as soon as `spendableReady` (before 100%, no reason line).
- Send DISABLED while syncing with nothing spendable → `walletSendWaitingForFunds`.
- Send DISABLED on a synced empty wallet → `walletSendNoSpendableYet`.
- balance + Send refresh when funds become spendable mid-sync (the edge-refresh,
  no manual resume).
- the multi-year "blocks left" countdown is never shown on the badge.
- (retained) never reads 100% before done; opaque-phase animated bar + catching-up;
  spendable-ready cue; merged-semantics node; offline/stall/connecting copy.

--- ORIGINAL (RETRACTED — see the UPDATE block; not implemented) ---

- `recover_until_splits_scan_and_recovery_windows` — after import at tip H with
  birthday B<H, `progress().recovery()` is `Some` over B→H and `scan()` covers
  H→tip.
- `spendable_is_not_suppressed_in_recovery_mode` — a near-tip note is spendable
  while the historic backfill is still pending (the spend-before-sync guarantee).
- `mandatory_done_before_backfill_complete` — mandatory window can read complete
  while `backfill_percent` < 100%.
- `fresh_wallet_has_no_backfill_indicator` — `recover_until ≈ birthday ≈ tip` ⇒
  `backfill_percent` is `None`/complete, no "restoring history" shown.
