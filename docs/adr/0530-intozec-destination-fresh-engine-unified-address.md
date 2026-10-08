# 0530 — The IntoZec swap destination is a fresh per-swap address derived at a single-use external index (`get_address_for_index`, engine-persisted for detection); detection polls a SCOPED active-swap set, not all receivers

- **Status:** Accepted
- **Date:** 2026-06-23 (revised same day after the `zcash_client_backend 0.23.0` API probe — see Decision 1)
- **Extends:** ADR-0527 (swap-refund address = single-use external-scope counter, live refund detection deferred) and ADR-0528 (transparent receive on external index 0, the refund counter floored at index 1). This ADR puts the IntoZec swap DESTINATION on the SAME single-use external counter as refunds, and additionally engine-persists + detects + shields it (the piece refunds defer). Discharges the ADR-0527-deferred "non-index-0 detection paired with auto-shield" for the destination case. Does not supersede ADR-0527/0528.
- **Links:** docs/specs/wallet-sdk.md §3.3b (IntoZec on-ramp) / §3.3a Recv-2b (the `GetAddressUtxos` poll it scopes) / §2.6 (swap port types, the corrected `SwapDirection::IntoZec` note) / §4.4 (IntoZec drives no deposit) · ADR-0529 (Recv-3 shield, the auto-shield path) · ADR-0014/0525/0526 (swap port / pluggable / NetDialer transport) · ADR-0005 (audited crates WHOLE — Rule Zero)

## Context

IntoZec (spec §3.3b) lets the user buy ZEC with another asset over the 1Click
rail. The only Rust-missing piece was the **ZEC receive address handed to 1Click**
as `recipient`; the NEAR adapter fail-closes IntoZec quotes until it exists. The
address must be (a) FRESH per swap (a reused address links every swap on-chain),
(b) DETECTABLE (funds the wallet can't see are a footgun — §3.3a Recv-2b), and
(c) cheap to allocate (abandoned/comparison quotes must not exhaust anything).

A `zcash_client_backend 0.23.0` / `zcash_client_sqlite 0.21.0` API probe
(2026-06-23) settled the mechanism and CORRECTED this ADR's first draft:

- **`get_next_available_address` (WalletWrite) AUTO-ADVANCES** the external
  transparent gap on every call ("Generates, persists, and marks as exposed the
  next available diversified address"). Minting one per quote would burn the gap
  window on abandoned/comparison quotes — the exact gap-exhaustion ADR-0528 went
  to lengths to avoid for refunds. **Rejected** (the first draft of this ADR chose
  it — wrong).
- **`get_address_for_index(account, diversifier_index, request)`** derives the
  address at a CHOSEN index via the audited UFVK path AND persists it to the
  `addresses` table (so `put_received_transparent_utxo` recognizes it), WITHOUT
  auto-advancing the gap allocator. This is the right primitive.
- **`get_transparent_receivers(account, …)` returns the WHOLE gap-pregenerated
  external set** (default gap limit 10), not just addresses we handed to a
  provider. Polling it wholesale would disclose the wallet's entire pre-generated
  receiver set to the lightwalletd endpoint — a new off-chain wallet fingerprint
  (security review BLOCKER). **Rejected** for the poll set.
- **`UnifiedFullViewingKey::address(index, request)` is a pure function** of the
  UFVK (no DB state) — the read-only derivation already used for the index-0
  receiver and the refund counter; works in `None`-persistence (no seed).

The existing single-use external-address counter (`refund_index`, ADR-0527/0528:
external `m/44'/coin'/0'/0/i`, floored at `REFUND_INDEX_FLOOR = 1` above the
index-0 receiver) already allocates distinct, never-recycled external indices
concurrency-safely (durable counter, `BEGIN IMMEDIATE`). Refunds derive on it but
do NOT engine-persist or detect (refund detection is ADR-0527-deferred).

## Decision

**1. The IntoZec destination is a fresh per-swap address derived at a DISTINCT
single-use external index via `get_address_for_index`** — NOT
`get_next_available_address` (auto-advance/gap-burn), NOT a stable index-0 reuse
(linkage). The index comes from the SAME single-use external counter as refunds
(generalize `refund_index` to a single-use swap-external-address counter;
refunds and destinations never co-occur for one swap, draw distinct indices, both
floored above the index-0 receiver, never recycled). `get_address_for_index`
persists the derived address to the engine `addresses` table, so it is
engine-tracked and `put_received_transparent_utxo` accepts it — the piece
refunds defer.

**2. Reserve-at-quote, persist-in-the-durable-record.** The destination index +
address are reserved at QUOTE time (the 1Click quote request binds the deposit
address to `recipient`, so it must exist at quote) using the durable counter's one
`BEGIN IMMEDIATE` reservation, and persisted INTO the durable `IssuedQuoteRecord`
(the W-swap-3-c store that already writes fail-closed before the quote returns).
So a crash between quote and execute does NOT strand funds — recovery reconstructs
the destination from the durable record and re-adds it to the detection set.
Distinct index per ISSUED quote ⇒ no reuse, no linkage, even across abandoned
quotes. **No gap burn** — `get_address_for_index` does not advance the engine's
gap allocator, and the custom counter is unbounded (the ADR-0527 property).

**3. Universality / `ends_shielded` honesty (unchanged from the first draft).** A
fresh address that carries the UA's receivers means the provider pays whichever it
supports. 1Click pays the TRANSPARENT receiver in practice (Zodl evidence —
`requestNextShieldedAddress(): WalletAddress.Unified` → `createShieldProposal`
after delivery), so the Recv-3 shield (ADR-0529) finishes the job; a
shielded-paying provider just works via the normal scan, no refactor. The
provider's capability decides per-swap. `ends_shielded: true` means the EXPECTED
end state after the one-tap Recv-3 shield (which the wallet nudges, never silent —
ADR-0529); a sub-threshold delivery (< `SHIELDING_THRESHOLD_ZAT`) honestly stays
transparent until it accumulates — the disclosure says "nudged to shield," not a
guarantee. This SUPERSEDES the 2026-06-10 "no shielded support → bare transparent"
note.

**4. Detection polls a SCOPED active-swap set, NOT all receivers.**
`refresh_transparent_utxos` polls exactly `{ index-0 receive } ∪ { committed
swap-destination addresses whose delivery is not yet shielded }` — a set the
wallet maintains from the durable swap/issued-quote store, NOT
`get_transparent_receivers()` wholesale. A destination retires from the set once
its delivery is shielded (or the swap terminates with no delivery). On a §3.5
`Hard` swap kill the destinations drop out (the set falls back to index-0 only):
honest-off stops the swap-attributable traffic; an in-flight delivery still lands
on-chain and is detected on the next user-initiated full sync. Same unary
`GetAddressUtxos`, same `transparent::validate` §4.6 boundary + M2 lying-endpoint
re-derivation, same sync circuit (Tor fail-closed), same best-effort SWALLOW. This
discharges the ADR-0527-deferred non-index-0 detection for the DESTINATION case.

## Consequences

- **§5 privacy is honest as scoped:** the endpoint sees only `{ index-0 } ∪ { the
  addresses we asked a provider to pay }`, each already disclosed to + paid by that
  provider — no wholesale gap-set disclosure, no new wallet-fingerprint channel,
  rides the existing sync circuit. This is the property the spec claims; the
  wholesale poll did NOT have it. User-opt-in (only by using a swap), honestly
  labeled.
- **Destinations and refunds share ONE single-use external counter** (DRY — no
  second counter answering the same "give a distinct never-recycled external
  address" question). The only difference: destinations engine-persist
  (`get_address_for_index`) + poll + shield; refunds derive read-only + defer
  detection.
- **No gap burn, crash-reconstructable, never-recycled** — the three properties the
  reviewers required, all met by counter + `get_address_for_index` + durable record.
- **Recovery caveat (residual, documented):** `get_address_for_index` persists in
  THIS wallet's DB, so same-device detection always works; but an external index
  beyond the gap window is NOT auto-found by a FRESH seed-restore scan (the same
  HD-recoverable caveat as the ADR-0527 refund counter). Mitigated by PROMPT shield
  (funds move to the recoverable shielded pool quickly; the transparent leg is
  brief). The loss window — funds delivered, wallet lost before shield, restored on
  a fresh device — is narrow and matches the refund posture; not widened here.
- **OutOfZec refund auto-shield can later ride the same scoped poll** (its
  destinations would be added to the active set on the same mechanism) — the
  remaining ADR-0527-deferred piece, no new detection code. **REALIZED (#368,
  2026-07-12):** refund addresses engine-register at quote (`get_address_for_index`
  at the shared counter's index, cross-checked byte-equal to the raw BIP44
  refundTo), enter the `swap_destination` watch at execute (settlement window,
  atomic with the W-swap-5 home row — which also moved the destination
  settlement-extend from `take` to `record_started`, so a refused execute no
  longer parks a 48 h watch), re-arm on a pinned `Refunded`, and pre-#368
  indices backfill-register behind a durable marker (reset across `rescan_from`,
  healing dropped destination registrations too). `prune_expired` now spares
  `funded` rows (a sub-threshold unshielded delivery keeps its shield-source
  membership) and gained the far-future arm. **EXTENDED (#382, 2026-07-12):**
  the scoped-poll membership of an UNRESOLVED swap's watched leg is no longer
  one settlement window — every transparent-refresh pass re-arms it while the
  swap record's outcome is unpinned and undismissed (spec §4.4 item 8; the
  S195 converged HIGH: the watch, the home row, and the Refunded re-arm all
  died on one execute+48 h bound, so a refund landing during a >48 h app
  absence was invisible). The §5 envelope above therefore reads: the set
  shrinks back toward index-0 once no swap is UNRESOLVED (pinned or dismissed)
  — bounded by `MAX_SWAP_RECORDS`, every member still provider-disclosed. A
  chain-observed refund (previously-unfunded OutOfZec watch row turning
  funded) pins `Refunded` first-wins, which converges the re-arm loop; the
  backfill ceiling is seeded at open/rescan-rebuild (quiescent points), closing
  the reserve→persist seed race the S194 text overclaimed as closed.
  **HARDENED (#385, 2026-07-13, the S196-b post-ship fold):** the retired-leg
  convergence is a durable `watch_served` LATCH on the swap record, written
  ATOMICALLY with the destination-row DELETE in one aux txn
  (`retire_and_mark_served` — renamed `debounced_retire_and_mark_served` by
  #386 below; the two-txn first cut left a kill/fault window
  that durably re-opened the retire→re-arm churn), and the latch is a flag,
  not a column clear, so the card-visit `Refunded` re-arm still heals the
  early-retire residual below (the first cut had silently narrowed that
  recovery to a full rescan). The per-pass re-arm extend is write-throttled
  (skipped while the watch still has ≥ half a settlement window),
  `mark_funded` re-marks are zero-write, the set-build's prune and Hard-kill
  clear are best-effort + busy-retried (a hygiene write's fault no longer
  kills index-0 detection for the pass), and `active()` SKIPS an unreadable
  tampered row fail-honest instead of failing every pass `StoreCorrupt`
  forever.
  **HARDENED (#386, 2026-07-13, the S197-b post-ship fold):** the
  funded→empty retire is DEBOUNCED — the Consequences' deferred hardening
  below, promoted because #385 changed its cost: the served latch stops the
  per-pass re-arm and the card-visit heal fires only on a STREAM-observed
  `Refunded`, so a single hostile/reorg "empty" reply now made an early
  retire STICKY — a sub-threshold parked IntoZec delivery stayed
  balance-visible but left the shield source set until a full rescan (the
  pre-#385 retire→re-arm churn had been accidentally self-healing exactly
  this). A funded row retires only after `RETIRE_AFTER_EMPTY_PASSES = 3`
  consecutive empty passes (`empty_streak` column, additive migration; any
  pass that sees the row funded resets it), the bump and the
  threshold-crossing retire+latch commit in the SAME aux txn, and
  `mark_funded` joined the pass's other aux writes as best-effort +
  busy-retried.
- **Key material never crosses FFI** — derivation is UFVK-based; the address is a
  public string.
- **Unlinkability rests on the TRANSPARENT receiver** (the leg 1Click actually pays and
  the §3.3b D2 poll detects), which is provably collision-free: distinct single-use external
  indices ≥ `REFUND_INDEX_FLOOR`, never the index-0 receive address. The destination UA's
  ORCHARD receiver (present for the shielded-paying-provider case) is at the chosen diversifier
  index; it would only share the main receive address's orchard receiver if `default_address()`
  resolved the receive UA to a diversifier index ≥ 1 — which it does not for a real key (index 0
  conforms, orchard has no invalid diversifiers), consistent with the spec's index-0 receive
  posture. Residual is theoretical (effectively nil); revisit only if a future change lets the
  receive `default_address()` roam off index 0. (IZ-1a security review INFO.)
- **IMPLEMENTATION OBLIGATIONS (IZ-1, named tests / crypto-change review + §5 re-review):**
  verify `get_address_for_index` is reachable on the locked `WalletDb` (pub /
  inherent vs trait) — if not, the fallback is pure `UnifiedFullViewingKey::address`
  + the engine's address-registration path; `into_zec_two_executed_swaps_get_distinct_destinations`
  (byte-inequality unlinkability), `concurrent_intozec_quotes_reserve_distinct_destinations`,
  `into_zec_recipient_survives_crash_between_quote_and_execute`,
  `into_zec_destination_is_engine_tracked_and_put_accepts_it`,
  `refresh_transparent_utxos_polls_only_the_scoped_active_set_not_all_receivers`,
  `hard_kill_drops_swap_destinations_from_the_poll_set`. **RETIRE/REPLACE** the shipped
  `intozec_quote_does_not_burn_the_refund_counter` (W-swap-3-b, S76) — it asserts the
  pre-IZ-1 "IntoZec reserves nothing" invariant this ADR inverts (IntoZec now reserves a
  distinct destination index on the shared counter); its successor is
  `concurrent_intozec_quotes_reserve_distinct_destinations`. The DESTINATION PORT is a
  SIBLING `DestinationAddressSource` (leave `RefundAddressSource` unchanged; share the
  single-use counter at the adapter level, not the service trait — their contracts differ:
  refunds derive read-only, destinations engine-persist). See spec §3.3b **L1-L8** for the
  full lifecycle/recovery/observability/UX obligations folded from the final multi-angle
  review (the durable `IssuedQuoteRecord.destination` field, the store-derived scoped-poll
  owner + crash recovery, the retirement triggers, the fresh-restore loss-window disclosure,
  the `wallet.swap_quote` span).
- **STATUS — IZ-1b SHIPPED (S93): the scoped detection poll + shield-source extension.** Delivered
  the detection half: a DEDICATED durable `swap_destination` store (sibling aux-db table) owns the
  scoped set; `refresh_transparent_utxos` polls `{ index-0 } ∪ active()` via a new multi-receiver
  `refresh_account_transparent_scoped` (each UTXO re-derived + matched, §4.6/M2); the wallet adapter
  drives the lifecycle (persist→`upsert_quoted` at quote, take→`mark_executed` extends to the
  `SWAP_SETTLEMENT_MAX_SECS` window at execute); retirement = `prune_expired` (lapse) + a Hard-kill
  `clear_all` (honest-off, drops to index-0) + a reactive funded-then-empty `retire` (the shield
  signal); `propose_shield` now sources `{ index-0 } ∪ active destinations}` (the deliver→detect→
  shield loop closes); recovery is by construction (store-derived) with a `wallet.swap_destination_recovered`
  emit at `enable_swap`; observability adds the `wallet.swap_quote` span + `utxo_scan { set_size }`.
  - **MECHANISM REFINEMENT vs the spec L2 wording (probed during impl, S93).** Decision 4 / L2 named
    `IssuedQuoteStore::active_destinations()` as the owner, assuming a durable swap record survives
    `execute`. It does NOT: `issued_quote_store::take` CONSUMES the issued quote at execute (it is the
    single-flight authority) and the status poll is in-memory — yet the detection window OUTLIVES the
    issued-quote row (funds arrive post-execute, watched through delivery+shield). So the scoped set
    got its OWN durable store with its own lifecycle, distinct from the single-flight claim. This
    HONORS the L2 INTENT (a durable store owns the set, crash-recoverable by construction, retires on
    shield/lapse/Hard-kill, no in-memory `Inner` cache → no lock-order risk against the engine
    `WalletDb`) — only the table differs. The address/index overlap between the two stores is two
    different facts with different lifetimes, not a DRY break. Spec §3.3b L2 records the same.
  - **Hard-kill detection gap (documented honest-off).** A delivery landing AFTER a Hard kill (or
    after the settlement window lapses) is NOT auto-detected by the routine scoped poll (the
    destination was dropped) — it is on-chain + engine-tracked and recovered by a future
    user-initiated transparent rescan (a deliberate, disclosed action), matching the spec's "detected
    on the next user-initiated full sync." Not widened here.
  - Named tests shipped: `refresh_transparent_utxos_polls_only_the_scoped_active_set_not_all_receivers`,
    `hard_kill_drops_swap_destinations_from_the_poll_set`,
    `open_then_poll_includes_a_crash_persisted_intozec_destination`,
    `intozec_destination_retires_from_poll_set_when_quote_lapses_undeposited`,
    `intozec_destination_retires_on_shield`, `into_zec_quote_span_is_5_4_clean`,
    `into_zec_two_executed_swaps_get_distinct_destinations`,
    `propose_shield_sources_a_swap_destination_delivery`, + the `swap_destination_store` unit suite
    (lifecycle / concurrency / `<=` boundary / over-u32 fail-closed / reopen-recovery /
    insert-if-absent crash-window). The execute-span §5.4 coverage is the existing
    `execute_emits_5_4_clean_wallet_swap_spans` (the `wallet.swap` half) + `into_zec_quote_span_is_5_4_clean`
    (the `wallet.swap_quote` half) — together the combined `…quote_and_execute_spans…` obligation.
  - **crypto-change review (crypto audit ∥ security review ∥ arch review → code reviewer): NO
    BLOCKER.** Folds applied: (1) **[security MAJOR] clamp the destination's retirement deadline** to
    `min(quote.expires_at, now + SWAP_DEADLINE_DEFAULT_SECS)` at upsert — the provider's `expires_at`
    does NOT get to keep a never-funded destination in the scoped poll indefinitely (it would defeat
    "idle shrinks to index-0" + accumulate rows the `MAX_ISSUED_QUOTES` cap can't bound, since the
    `swap_destination` row outlives the issued-quote row); the MONEY deadline stays unclamped (gated
    by the durable `deposit_gate`). (2) **[crypto MINOR] close the persist↔upsert crash window** —
    `mark_executed` is now an UPSERT keyed on the durable record's address+index, so execute
    re-creates the detection-set row if the quote-time `upsert_quoted` was lost to a crash (an
    executed swap is never silently un-polled). (3) Made the recovery-emit poison/busy skip explicit;
    documented the single-worker contract behind the active→put→advance lock window.
  - **Accepted residual (documented):** the funded-then-empty retirement is a NEGATIVE proxy for the
    shield (not the spec §3.3b L3(a) positive txid-keyed signal) — a transiently-lying endpoint or a shallow
    reorg can early-retire a still-funded destination. Bounded + NEVER a fund loss (the UTXO is already
    engine-`put` + the address engine-registered ⇒ recovered by a user-initiated full transparent
    rescan, the SAME honest-off posture as the Hard-kill drop). **Future hardening:** debounce retire
    over N consecutive empty passes (a small `empty_streak` column) to absorb a single hostile/reorg
    reply; deferred (no loss, recoverable). **Promoted + SHIPPED by #386** (see the HARDENED note in
    STATUS): the #385 served latch turned this bounded residual sticky for a sub-threshold parked
    delivery, so the deferral's "recoverable" premise narrowed to rescan-only — the debounce restores
    the absorb-one-lie property the churn had provided by accident.
  - **SECOND operational round (building-bricks/ZODL ∥ money-reliability/mobile/unstable ∥
    real-world-edge tests): NO BLOCKER, money core SOUND.** Confirmed every new mechanism reuses an
    audited brick (`propose_shielding` whole, `TxOut::recipient_address`, `Address::decode`, the
    aux-store sibling pattern, the existing poll/`FakeUtxoSource` seam) — nothing reinvented; and
    confirmed ZODL alignment on the three load-bearing properties (fresh UA per swap, provider pays the
    transparent receiver we DETECT, shield-after-delivery), with Relim IMPROVING on ZODL twice: the
    scoped (not wholesale `null`-receiver) shield/poll set avoids the lightwalletd wallet-fingerprint,
    and on-chain `GetAddressUtxos` verification replaces ZODL's trust-the-provider-status detection.
    Money traced sound under crash/fault/suspend: idempotent engine puts (no double-count), wall-clock
    deadlines (correct vs `Instant` for mobile suspend), saturating panic-free clock reads, the
    network-fault ordering (read→put→store-advance, all best-effort SWALLOWED + retried), platform-
    agnostic (desktop parity). +7 real-world-edge tests (transport-fault-survives, mixed multi-dest
    attribution, still-funded-no-retire, index-0+dest attribution, OutOfZec-records-nothing, absent-id
    no-ops) → **666 lib green**. Comment-precision folds applied (the funded-then-empty NEGATIVE-proxy
    framing; the shield-path lock-order cross-ref). **SECOND future-hardening ticket:** fold
    `take`+`mark_executed` into one enlisted aux txn (the outbox `insert_enlisted` pattern) to close the
    sub-ms execute-time crash window where a killed-mid-execute swap reverts to the quote-deadline watch
    window instead of settlement — recoverable by rescan today, so deferred (no loss).

- **STATUS — IZ-1a SHIPPED (S92).** IZ-1a delivered the destination keystone +
  adapter unblock: the `DestinationAddressSource` SIBLING port, `mint_swap_destination`
  (`get_address_for_index` on the shared single-use counter), the durable
  `IssuedQuoteRecord.destination` + columns, `build_into_zec_request`/`map_into_zec_quote`,
  `SwapError::DestinationAddressUnavailable` (RW-SWAP-012), and the RETIRED inverted test.
  Shipped tests: `into_zec_destination_is_engine_tracked_and_put_accepts_it` (the put-accepts
  proof, real engine), `concurrent_intozec_quotes_reserve_distinct_destinations`,
  `into_zec_recipient_survives_crash_between_quote_and_execute`,
  `into_zec_two_quotes_get_distinct_destinations`, the no-account/mint-failure/migration/
  refund-injection tests. The crypto-change review found NO structural blocker; the
  concurrent test caught + fixed a real engine-write↔aux-write SQLite deadlock (hold db→aux
  across the mint). **IZ-1b OWES:** `refresh_transparent_utxos_polls_only_the_scoped_active_set_not_all_receivers`,
  `hard_kill_drops_swap_destinations_from_the_poll_set`, the L2/L3 retirement + recovery tests,
  the Recv-3 shield nudge, and the `wallet.swap_quote` span. **Owed to IZ-3:** ~~the bridge
  `SwapErrorKind`/Dart arm for `DestinationAddressUnavailable`~~ **(LANDED in IZ-2 — the
  `bridge_enums_cover_core_variants` lockstep guard was RED since IZ-1a, so the IZ-2 operational
  round added the variant + convert arm + Dart regen; it no longer degrades to `Unknown`)**
  + memo-requiring IntoZec deposits (the D7 deposit screen + a DTO memo field) + extracting the
  `map_out_of_zec_quote`/`map_into_zec_quote` shared response-parse block (deferred because IZ-3's
  `deposit_memo` divergence would re-split it). A SECOND operational round (building-bricks/ZODL ∥
  money/mobile/unstable ∥ edge-tests) confirmed the money core SOUND + all brick reuse + ZODL
  alignment; folded a no-account fail-fast pre-check (burn no index) and moved the `> u32::MAX`
  destination-index rejection to the store's `take` validation door; +5 real-world-edge tests.

## Alternatives considered

- **`get_next_available_address` (engine auto-allocator)** — the first draft's
  choice; rejected after the probe: it auto-advances the external gap per call, so
  abandoned/comparison quotes burn the gap window and eventually wedge detection
  (`AddressNotRecognized`), the gap-exhaustion ADR-0528 avoided for refunds.
- **`get_transparent_receivers()` wholesale for the poll set** — rejected
  (security BLOCKER): discloses the entire gap-pregenerated receiver set off-chain
  to the endpoint, a new wallet fingerprint, undermining the fresh-per-swap
  unlinkability goal. The scoped active-swap set has the claimed property; this
  did not.
- **`reserve_next_n_ephemeral_addresses` (ZIP-320 ephemeral scope)** — rejected:
  meant for one-time TEX outputs; the 10-slot ephemeral gap stays "exposed" if the
  provider never pays, and it is a different scope from the receive/refund external
  scope. The external counter + `get_address_for_index` is the consistent fit.
- **A bare custom counter WITHOUT engine-persist** (like refunds today) — rejected
  for destinations: not engine-tracked ⇒ `put_received_transparent_utxo` rejects
  ⇒ the ADR-0527-deferred detection problem reappears. `get_address_for_index`
  gives both chosen-index control AND engine tracking.
- **Reuse one "pending" unfunded destination across re-quotes** — rejected: two
  concurrent ISSUED quotes could both execute and deliver to the same address ⇒
  on-chain linkage. Distinct-per-issued-quote is the safe rule.
- **Stable index-0 receiver reused across swaps** — rejected: links every IntoZec
  swap + the user's main receive address on-chain. Unacceptable for a privacy wallet.
