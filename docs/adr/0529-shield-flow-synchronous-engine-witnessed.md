# 0529 — The shield flow is synchronous-only; crash-safety is engine-witnessed (spent-UTXO exclusion), not the §6.3 intent guard

- **Status:** Accepted
- **Date:** 2026-06-23
- **Extends:** ADR-0528 (transparent receive on external index 0). ADR-0528 enabled `transparent-inputs` for the RECEIVE address and noted "the only transparent spend is privacy-positive shielding"; THIS ADR records the shielding spend's design. Does not supersede ADR-0528.
- **Links:** docs/specs/wallet-sdk.md §3.3a (Recv-3) / §3.2h (the send pipeline it reuses) / §3.1 (`propose_shield`) / §6.2–§6.3 (offline intent + crash-atomicity) · ADR-0005 (audited crates WHOLE — Rule Zero) · ADR-0527/0528 (transparent posture)

## Context

Recv-3 (spec §3.3a) adds `propose_shield()`: move detected transparent funds
(exchange withdrawals + swap-in deliveries land transparent, §1.7) into the
wallet's own shielded pool, gated at `SHIELDING_THRESHOLD_ZAT` (0.001 ZEC). It
wraps the audited upstream `zcash_client_backend::data_api::wallet::
propose_shielding` + `create_proposed_transactions` (Rule Zero — we select no
input, compute no fee, build no output, sign nothing ourselves).

Two design questions had to be settled, both load-bearing for money-safety:

1. **Does a shield ride the offline-first queue (`queue_send` → the
   `intent_store` + the §6.3 double-send guard + the resubmission machinery), or
   the synchronous `propose → send` path?**

2. **How is the registry token typed**, given `propose_shielding` returns
   `Proposal<FeeRule, Infallible>` (its inputs are transparent UTXOs, so it
   references NO shielded notes — the `NoteRef` is the uninhabited `Infallible`),
   whereas a transfer proposal is `Proposal<StandardFeeRule, ReceivedNoteId>`?

The §6.3 crash-atomicity machinery (ADR-0021 / spec §3.2h inc-2d-3-b-ii) makes
an intent never mint two txs by persisting a CLAIM — the chain-stable
`(txid, ShieldedProtocol, output_index)` of every selected SHIELDED note —
before create, then witnessing with `get_spendable_note`. That witness is
inherently about shielded notes; a shield spends transparent UTXOs, so the claim
mechanism does not directly apply.

## Decision

**1. A shield is SYNCHRONOUS-ONLY.** It reuses the §3.2h send pipeline whole —
`propose_shield` returns a one-shot `SendProposal` token retained in the SAME
`ProposalRegistry`, and the host confirms then feeds its `proposalId` to the
existing `send`/`send_by_id` (consume → derive USK in `spawn_blocking` →
`create_signed_core` → broadcast). A shield is NOT enqueued in the
`intent_store`; there is no offline re-propose for it.

Rationale: shielding is a **hygiene action the user triggers in-app**, not an
offline-first payment that must survive a process kill mid-queue. Reusing the
immediate path is the smallest correct surface (KISS + DRY); its
broadcast-failure posture is IDENTICAL to a synchronous `send` (best-effort over
a fresh per-tx circuit; a failure is `TxSubmitResult` DATA; the engine's
~40-block expiry safety net frees the UTXO for a bounded re-shield — never a
fund loss). This is recorded structurally as `Retained.request: Option<…>` =
`None` for a shield (no re-proposable `PaymentRequest`).

**2. Crash-safety is ENGINE-WITNESSED via spent-UTXO exclusion**, not the §6.3
shielded-note claim. Once `create_proposed_transactions` persists the shield tx
(the engine marks the input UTXO spent + writes the tx atomically), upstream
`get_spendable_transparent_outputs` excludes any UTXO spent by an unexpired tx
(`zcash_client_sqlite` `spent_utxos_clause` — even unmined). So a re-
`propose_shielding` selects zero inputs → the engine's own `InsufficientFunds`
→ our `Ok(None)`. A kill between persist and broadcast cannot double-shield: the
UTXO is already spent in local DB state, and an un-broadcast tx expires (~40
blocks) → the UTXO frees → a fresh shield is possible. This is the same
"engine-owns-the-witness" principle as the §6.3 guard, applied to transparent
inputs (which the engine tracks natively) instead of shielded notes. Verified
end-to-end by `shield_proposal_over_threshold_signs_and_does_not_resurrect`
(funded harness, real bundled prover).

**3. The registry token is an enum `RetainedProposal { Transfer(…), Shield(…) }`.**
`create_proposed_transactions`'s `NoteRef` type parameter is free of
`DbT::NoteRef`, so `summarize`/`create_signed_core` are genuinely generic over
both; the enum exists ONLY because the two concrete `NoteRef`s differ. The
alternative (a second registry) would be more duplication, not less. `put_shield`
/ `put_retained` share the one bounded-insert path (TTL sweep + eviction) with
the transfer `put`.

## Consequences

- **No new crash-recovery code** for shields — the engine's spent-UTXO state is
  the witness; we add no `intent_store` row, no claim, no reconcile arm.
- **The shield's `Retained.request` is `None`.** OBLIGATION for inc-2d-3 (when
  the broadcast/outbox re-propose path wires `.request`): it MUST branch on
  `None` (skip re-enqueue for a shield token — synchronous-only), never
  `.unwrap()`. A named test for the None-for-shield contract is owed there.
- **A shield broadcast failure is bounded-delay, never loss** — identical to a
  synchronous send; if neither in-app retry nor expiry-then-re-shield is
  acceptable for a future "auto-shield" UX, that auto-shield can ride the
  `intent_store` queue separately (it is the ADR-0527-deferred auto-shield
  surface), without changing this synchronous in-app shield.
- **Refund DETECTION + auto-shield stay ADR-0527-deferred** — this ADR covers
  only the user-triggered shield of the index-0 receive address.

## Alternatives considered

- **Queue the shield through `intent_store`** (offline-first, §6.3-guarded):
  rejected — over-engineered for a user-triggered hygiene action, and the §6.3
  claim is shielded-note-shaped (a shield has none), so it would need a parallel
  transparent-input claim mechanism for a path that does not need it (the engine
  already excludes spent UTXOs). Revisit ONLY if a background auto-shield lands.
- **A second `ProposalRegistry` for shields** (avoid the enum): rejected — more
  duplication, two TTL/eviction code paths, two id spaces; the enum is the
  minimal factoring.
