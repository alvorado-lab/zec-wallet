# 0537 — Mint public diversified receive UAs in a disjoint high diversifier region

- **Status:** Accepted
- **Date:** 2026-07-20
- **Links:** docs/specs/wallet-sdk.md §3.3a Recv-4 · docs/handoff/host-feature-requests.md FR-8 ·
  builds on [0030](0530-intozec-destination-fresh-engine-unified-address.md) (the
  `get_address_for_index` primitive) · [0027](0527-swap-refund-address-external-scope-engine-feature-deferred.md)
  (the durable-counter + restore-story law) · [0028](0528-transparent-receive-address-external-index-0.md)
  (index-space reservation precedent) · Relim ADR-0026 D7 (the driving consumer decision)

## Context

FR-8 asks for a public "mint the next diversified unified address" API: unlinkable
per-contact / per-invoice receive addresses that all credit account 0, deterministic per
diversifier index. Today only the single default UA is public; the diversified allocator
that exists (`refund_index` + `mint_swap_destination`) is swap-internal.

Three forces constrain where the new addresses may live and how their indices are tracked:

1. **Equal diversifier index ⇒ equal shielded receivers, regardless of receiver set.**
   The swap destination UA (Require orchard / Allow sapling / Require transparent) and a
   would-be public receive UA (G7: Require / Require / Omit) minted at the SAME diversifier
   index encode DIFFERENT UA strings but carry byte-identical orchard/sapling receivers —
   trivially linkable by anyone comparing the encodings. THREE producers already occupy
   parts of the diversifier space: the swap/refund counter at `[1, NON_HARDENED_MAX]`
   (≈ `[1, 2^31)`); the default UA's `default_address` forward search at the lowest valid
   indices; and — the S217 crypto audit catch that re-litigated this ADR's first draft —
   **the engine's own shielded gap allocator**: `zcash_client_sqlite`'s
   `get_next_available_address` bases shielded diversifiers at
   `now_secs + MIN_SHIELDED_DIVERSIFIER_OFFSET (2,817,325,936)`, a band that has sat
   above 2^32 since Zcash genesis (≈ `2^32 + 3.1×10^8` in 2026, climbing one per
   second, crossing 2^40 only around year ~36,000). The SDK never calls that allocator
   (ADR-0530 rejected it), but a region choice must hold even if a future engine-API
   adoption — or another consumer of the same wallet DB — does. A public mint that
   overlaps ANY of the three silently destroys the unlinkability it exists to provide.
2. **ADR-0527's law: any new durable counter must have a restore story from day one**
   (the #387/#390 lessons). But the transparent machinery those lessons produced
   (backfill registration, scoped UTXO polling, user-triggered deep scan) exists because
   transparent detection is address-enumerated. Shielded detection is NOT: trial
   decryption with the account IVK detects funds at ANY diversifier index. A restored
   wallet sees every payment ever sent to every previously-minted diversified UA with no
   counter at all. The only restore harm is RE-ISSUING an index already handed out in the
   pre-restore life (two contacts end up holding the same UA — an attribution/linkage
   defect, never a fund-visibility defect).
3. **The ZIP-32 diversifier space is 88 bits.** The BIP44 non-hardened bound (2^31)
   constrains only indices that must double as transparent child indices. A shielded-only
   (G7) UA has no transparent receiver, so its index may exceed 2^31 freely —
   `zip32::DiversifierIndex` is `From<u64>`.

## Decision

Public diversified receive UAs derive at diversifier indices in the **diversified receive
region `[2^40, 2^40 + 2^31)`** — disjoint by construction from the swap/refund space
(capped at `NON_HARDENED_MAX < 2^31`), from the default UA's forward search (starts at 0
and advances only past sapling-invalid indices; even reaching 2^32 would take 2^32
consecutive invalid indices, probability ≈ 2^-(2^32)), AND from the engine's
timestamp-based shielded allocator band (`now + 2,817,325,936`, which stays below 2^40
until roughly year 36,000 — Context force 1; the first draft's `2^32` base sat INSIDE
that band and was moved before anything shipped). The receiver set is **G7
(Require orchard / Require sapling / Omit transparent)** — the same compatibility posture
as the advertised default UA; a sapling-invalid fixed index is a per-index miss
(`get_address_for_index` → `Ok(None)`, ≈ half of all indices) that burns the index and
advances, never a silent orchard-only degrade (the W3-inc-2 crypto audit lesson).

Indices come from a **dedicated durable counter** (`diversified_address_index`, aux DB,
never-reuse, `BEGIN IMMEDIATE` read-modify-write, floor `DIVERSIFIED_INDEX_BASE = 2^40`)
whose day-one restore story is the **#387 restore-pessimistic first seed alone** —
**AMENDED S218-b (post-ship review): the seed is a RANDOMIZED band, not a constant**:
counter-row-absent ∧ not-freshly-generated-here ⇒ seed counter at
`BASE + k·DIVERSIFIED_RESTORE_BAND` (band 4096 ≈ 2048 mints — the G7 sapling walk burns
≈2 indices per mint), OsRng `k ∈ [1, 2^18]`. The original constant `BASE + 1024` had two
defects the S218-b review caught: sized in indices but disclosed in mints (2×
overstatement), and constant ⇒ every CHAINED seed-only restore re-seeded the same floor,
certainly re-issuing the previous restored life's mints after a handful of mints. A
random band collides with a prior life only if it lands inside that life's used span
(≤ ⌈used/4096⌉/2^18 per life, ~2^-18) — probabilistic, replacing certain. No backfill
registration, no deep scan, no scoped polling — force 2 above makes them unnecessary;
bands cost nothing (no per-index scanning). Minted addresses are
engine-persisted via the audited `get_address_for_index` (the ADR-0530 primitive — no gap
burn, engine `addresses` table stays truthful); the engine rows are a courtesy
registration whose loss on rescan/restore is harmless, and **the aux counter is the sole
never-reuse authority**.

`DIVERSIFIED_INDEX_BASE = 2^40` is **FROZEN** the way derivation labels are frozen:
changing it would re-issue shielded receivers already published under the old region.
(The one sanctioned move — `2^32 → 2^40` — happened pre-commit, before any address
existed anywhere, on the S217 audit finding above.)

## Alternatives considered

- **Share the `refund_index` counter** (perfect disjointness for free). Lost: it couples
  public receive semantics to swap machinery — every public mint would inflate the #390
  deep-scan range and the #368 backfill's `covered_swaps` accounting with non-swap
  indices (transparent polling for addresses that never had a transparent receiver), and
  the shared `RESTORE_BACKFILL_BREADTH` (64, cost-bounded by real UTXO polling) is wrong
  for a counter whose breadth is free.
- **Engine gap allocator (`get_next_available_address`).** Lost per ADR-0530: burns the
  gap on abandoned mints and is gap-limit-bounded. (Its shielded arm allocates in the
  `now + offset` timestamp band described in Context — nowhere near the default UA,
  which the first draft of this ADR got wrong; the corrected fact is exactly why the
  public region moved to 2^40.)
- **Allow sapling instead of Require (every index conforms, no misses).** Lost: ~half of
  mints would be orchard-only UAs that a sapling-only sender cannot pay — a silent
  compatibility downgrade from the advertised default UA, the precise failure G7's
  `Require` was chosen to prevent.
- **Derive-only (no engine persistence).** Lost: diverges from the ADR-0530 precedent for
  no saving (the mint already holds the locks), and forfeits the engine's own record of
  issued addresses that future attribution ("which diversifier received") builds on.
- **Host-supplied index (`diversified_address(index)`).** Lost: hands the never-reuse
  invariant to every host to re-implement (ADR-0527's law exists because we got this
  wrong ourselves); the SDK-owned mint-next counter keeps it structural. Deterministic
  re-derivation stays available to a future read-only accessor without weakening this.

## Consequences

- Easier: hosts get unlinkable per-contact/per-invoice UAs from one call; restore needs
  no new scanning machinery; the swap allocator and its accounting stay untouched.
- Harder: the diversifier index crosses the FFI as `u64` (first 64-bit index in the DTO
  surface; Dart `int` is 64-bit, and the region's top `< 2^41` keeps the web narrowing
  exact under 2^53); a second singleton counter table joins the aux schema.
- Frozen: `DIVERSIFIED_INDEX_BASE = 2^40`, the G7 receiver set for public mints, and the
  disjointness contract (public receive region vs swap space vs default-UA search vs the
  engine's timestamp band).
- Follow-ups: coarse attribution-by-address ("which diversifier received") is FR-8's
  optional tail — the mint returns `{address, diversifier_index}` so a host can already
  keep the mapping; an SDK-side report is deferred until a consumer asks.
