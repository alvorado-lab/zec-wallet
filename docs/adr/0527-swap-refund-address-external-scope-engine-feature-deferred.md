# 0527 — Swap refund addresses use BIP44 external scope; engine transparent-inputs deferred until live refund detection

- **Status:** Accepted — **the deferred obligation is DISCHARGED (#368, 2026-07-12):** refund
  detection + auto-shield now ride the ADR-0530 scoped poll (the path the Extended-by note
  reserved). Refund addresses engine-register at quote via `get_address_for_index` at the counter's
  index, byte-equality cross-checked against this ADR's raw BIP44 derivation (the provider-facing
  t-addr is unchanged, so HD recoverability is unchanged); the watch arms at execute (settlement
  window, atomic with the durable swap record) and re-arms when a `Refunded` terminal is pinned;
  every already-allocated index is backfill-registered behind a durable high-water marker — the
  Consequences' named migration obligation — with the marker reset across `rescan_from` (the engine
  rebuild drops address registrations). Decision 4 held: the counter's index sequence was the input
  to registration, not replaced state. (Note: the engine-feature precondition dissolved earlier
  than expected — ADR-0528/Recv-2 turned `transparent-inputs` ON the engine for the receive
  address; #368 supplied the remaining registration + poll-set membership.)
  **#387 addendum (2026-07-13):** the #368 discharge seeded the backfill ceiling from the
  LOCAL aux counter, so the SDK's OWN fresh-seed restore (empty aux → ceiling = floor →
  "done forever") registered nothing — Decision 1's §4.5 recoverability held for
  third-party BIP44 restores but in-app visibility did not. Closed by the
  RESTORE-PESSIMISTIC first seed: counter-row-absent ∧ creation-stamp-absent ∧
  bounds-row-absent (first-wins) ⇒ counter = ceiling = `RESTORE_BACKFILL_BREADTH`
  (64), counter written first (crash-convergent), so the standard backfill
  registers + one-window-watches the first ~63 external indices and post-restore
  allocations start above the swept range (no reuse within the breadth). Residuals
  (≥ breadth invisibility/reuse on heavy pre-restore histories; pre-#387-restored
  wallets keep the floor seed; the forged-stamp tamper asymmetry) are disclosed in
  spec §3.2h item 5.
  **#390 addendum (2026-07-13, design 4-angle-reviewed pre-implementation):** the #387 first
  seed is BOUNDED (breadth 64) to keep the restore-time poll set inside the item-7 privacy
  envelope, so residual (a)'s in-app invisibility persisted for a >63-quote pre-restore
  history — recoverable only via the third-party BIP44 §4.5 path. #390 gives the SDK its OWN
  in-app recovery: a user-triggered DEEP SCAN ("Check older swap addresses" — the shared
  counter means it recovers IntoZec DELIVERIES as much as refunds; CHECK-verb, never
  "recover") RAISES counter+ceiling to `min(counter + STEP, NON_HARDENED_MAX + 1)`
  (STEP ≈ 1024, compiled) in ONE aux `IMMEDIATE` txn — serialized with `reserve_next_index`
  (no reuse), `ceiling ≤ counter` by construction — so the standard backfill registers +
  one-window-watches the band and the scoped poll surfaces the UTXOs (NO chain rescan).
  TYPED REFUSALS: kill-switch active, prior band still registering or its window unexpired
  (one outstanding band per settlement window — retry-safety + the elective-growth cap),
  exhaustion. The swept band = fresh widened indices PLUS retired allocated history above
  the preserved marker (deliberate for recovery; the disclosed user-triggered exception to
  "abandoned never polled" / "envelope only shrinks"). Execute-time arming deletes a
  same-index `backfill:` row (the one-row-per-receiver rule held through the mid-issuance
  race). Observability: a render-only `{coverage, pending}` read (`pending == 0` = band
  registered + polled once); DTO/tracing counts-only (§5.4). Raising the counter BURNS the
  band (closes residual (a)'s reuse half) and the first run heals residual (b). A
  Generate-created wallet pays the full residual-(c)-shaped cost and recovers nothing unless
  its seed lived elsewhere (why the op is NOT provenance-gated). Decision 1's §4.5
  recoverability is now matched in-app. Spec §3.2h item 5 (#390 sub-block) + Exception
  THREE RECURRENCE carry the normative text.
- **Date:** 2026-06-21
- **Extended-by:** ADR-0530 (IntoZec swap-DESTINATION detection — a SCOPED active-swap `GetAddressUtxos` poll over the destination derived at a shared single-use external index via `get_address_for_index` — discharges this ADR's deferred "non-index-0 detection paired with auto-shield" for the destination case; OutOfZec refund auto-shield can later add its addresses to the same scoped poll; does not supersede). **Realized for refunds at #368 (see Status).**
- **Links:** docs/specs/wallet-sdk.md §2.6 (HARD-H) / §3.2h (W-swap-3-a) / §4.5 / §6.3 · ADR-0005 (librustzcash-direct, audited crates WHOLE) · ADR-0014 / ADR-0525 (NEAR-Intents swap in the SDK) · ADR-0013 (wallet as an extraction-ready SDK)

## Context

An `OutOfZec` swap deposits ZEC **from the shielded pool**, so a failed swap is
refunded by the provider to a **transparent** address the wallet controls (the
shielded pool gives the provider no observable on-chain source to refund to —
§2.6). HARD-H requires that refund address be **fresh and never recycled** (reuse
would publicly link two swaps on-chain — a deanonymization vector).

W-swap-3-a (this slice) built that derivation. During its review the
"building-bricks vs reinventing-the-wheel" lens raised a real question: the
audited `zcash_client_backend 0.23.0` already ships
`WalletWrite::reserve_next_n_ephemeral_addresses` + the matching
`WalletRead::get_known_ephemeral_addresses` — a durable, never-recycle,
engine-tracked-and-scanned transparent-address allocator (ZIP-320 **ephemeral**
addresses at scope `m/44'/coin'/account'/2/i`). Did W-swap-3-a reinvent that
wheel with a custom counter + raw `zcash_transparent` derivation?

Two hard constraints shape the answer:

1. **The ephemeral brick is `#[cfg(feature = "transparent-inputs")]` on the
   ENGINE crates** (`zcash_client_backend` / `zcash_client_sqlite`), and its
   durable state lives in the engine's `WalletDb` (the `ephemeral_addresses`
   table on the connection the SDK deliberately has **no accessor into**, per Rule
   Zero / ADR-0005). The SDK keeps `transparent-inputs` **off the engine** to
   preserve the G7 shielded-only UA and the pool-crossing-off send posture that
   `send.rs` / `wallet.rs` actively assert.
2. **Ephemeral scope (2) is a Zcash-specific, non-BIP44 convention** built for the
   two-phase ZIP-320 TEX *spend* lifecycle (the wallet creates the spending tx
   before it can "see" the output). A swap **refund** is the opposite shape — an
   independent *inbound* provider delivery — and must be **HD-recoverable in any
   BIP44 wallet** on a restore-from-seed (the §4.5 wipe-recovery path). The
   standard external receive chain (scope 0, `m/44'/coin'/0'/0/i`) is what a
   third-party BIP44 wallet scans; scope 2 is not.

Separately, the review exposed a **latent spec inconsistency**: §6.3 promises
*live* auto-shield of an `OutOfZec` refund (detect the transparent UTXO →
`BalanceSnapshot.transparent` renderable → shield tx). Detecting a transparent
UTXO requires transparent-output **scanning** — which requires
`transparent-inputs` on the **engine**. With that feature off, the wallet derives
a refund address it cannot yet *see funds at*. The auto-shield-the-refund
guarantee is currently **hollow**.

## Decision

1. **Swap refund addresses derive at the BIP44 EXTERNAL scope**
   `m/44'/coin'/0'/0/index` (via `zcash_transparent::keys`, audited WHOLE), NOT
   the ephemeral scope. External scope is the standard receive chain any BIP44
   wallet scans on restore, so a refund landing after a panic-wipe is recoverable
   from the seed alone. The fresh, never-recycled index comes from a **durable
   monotonic counter in the SDK's aux `wallet.db`** (`refund_index`), which is the
   structurally correct storage given Rule Zero forbids reaching into the engine's
   `WalletDb`.

2. **`transparent-inputs` is enabled ONLY on the leaf `zcash_transparent` crate**
   (the key-derivation types), NOT on the engine crates. The engine keeps its
   shielded-only / pool-crossing-off posture unchanged. The SDK uses only
   external-scope **address derivation** — no transparent spend/scan path is
   enabled.

3. **Live refund DETECTION + auto-shield is DEFERRED** to a future slice that
   explicitly flips `transparent-inputs` on the engine. Until then the
   implemented behavior is **HD-recoverable refund only** (restore from seed
   re-derives the index; a BIP44 wallet scans it), NOT live in-app detection. The
   §6.3 auto-shield-the-refund guarantee is marked deferred in the spec, not
   silently claimed.

4. **The custom counter is NOT throwaway.** When the engine feature flips, the
   counter's durable index sequence is the **input** to engine registration (the
   SDK tells the engine "watch these N external-scope addresses" via the
   transparent-address-metadata API), not state the engine replaces. The
   ephemeral brick remains the wrong tool for refunds (wrong scope, TEX
   semantics); the external-scope counter feeds forward.

## Consequences

**Positive**
- Refunds are HD-recoverable in any BIP44 wallet (the strongest recovery guarantee
  the third-party path can offer — §4.5).
- The engine's G7 shielded-only + pool-crossing-off invariants stay intact; no
  scan-performance cost, no migration-chain change, no pool-crossing surface added
  to the proposal engine — until and unless a deliberate later slice opts in.
- Zero new crates (the feature rides the already-linked `zcash_transparent`).

**Negative / deferred (tracked)**
- **Live auto-shield of `OutOfZec` refunds is not wired** — the spec §6.3 row is
  amended to mark it deferred. Current behavior: a refund is HD-recoverable, not
  in-app-detected. Owed at the engine-feature-flip slice.
- **The engine-feature-flip migration is owed:** when `transparent-inputs` lands on
  the engine, the SDK must register every already-allocated external-scope refund
  index with the engine's transparent-output tracker so it scans them. Recorded as
  a W-swap-3-c / GA obligation.
- **BIP44 gap-limit caveat:** a refund at an external index beyond the standard
  20-address gap that has never received funds may require manual index knowledge
  on a third-party restore — inherent to BIP44, not unique to this design.
- **W-swap-3-b obligation:** the `WalletRefundSource` must expose
  reserve-then-derive as ONE safe operation (so no caller reserves-and-forgets or
  derives-without-reserving — security H1), and surface an exotic-seed derivation
  failure as a distinguishable named error, not a generic `KeyDerivation`.

**Rejected alternative — adopt `reserve_next_n_ephemeral_addresses` now.** It would
require flipping `transparent-inputs` on the engine (changing the scan/spend
posture this milestone keeps off), uses the non-BIP44 ephemeral scope (worse
third-party recoverability for a refund), stores state in the inaccessible engine
`WalletDb`, and carries TEX two-phase semantics that don't fit an inbound refund.
It becomes the right brick for refund *detection/auto-shield* only when the engine
feature is deliberately turned on — at which point this ADR's migration obligation
applies.
