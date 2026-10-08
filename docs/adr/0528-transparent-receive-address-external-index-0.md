# 0528 — Transparent receive address on external index 0; swap refunds keep the unbounded custom counter (floored at index 1)

- **Status:** Accepted
- **Date:** 2026-06-22
- **Amends:** ADR-0527 (swap refunds on a custom external-scope counter; engine `transparent-inputs` deferred) — refunds STAY on that counter; this ADR enables `transparent-inputs` for the RECEIVE address and reserves external index 0 for it.
- **Extended-by:** ADR-0529 (records the shielding-spend design — synchronous-only, engine-witnessed crash-safety — for the "only transparent spend is privacy-positive shielding" this ADR anticipated; does not supersede). ADR-0530 (the IntoZec swap DESTINATION shares the single-use external counter with refunds, derived via `get_address_for_index` and engine-persisted for a SCOPED active-swap detection poll; does not supersede).
- **Links:** docs/specs/wallet-sdk.md §3.3a (Recv-2) / §2.6 (HARD-H) / §3.2h (W-swap-3-a) / §4.5 / §6.3 · ADR-0005 (librustzcash-direct, audited crates WHOLE) · ADR-0013 (wallet as an extraction-ready SDK) · ADR-0014 / ADR-0525 (NEAR-Intents swap)

## Context

Recv-2 (spec §3.3a, founder-directed 2026-06-22) adds a **transparent receive
address** alongside the shielded UA: CEX withdrawals and swap-in deliveries land
transparent, which a shielded-only wallet cannot receive. The natural home is
the standard BIP44 **external** receive chain `m/44'/<coin>'/0'/0/0`, index 0 —
the account's canonical transparent receiver.

That scope is shared with swap **refunds**. ADR-0527 put refund addresses on a
custom monotonic counter (`refund_index.rs`) deriving at the SAME external scope
from index 0. So a naked external-index-0 receive address is byte-identical to
refund index 0 — two allocators handing out the same address (a privacy +
address-reuse bug on money).

We first evaluated **full engine ownership**: enable `transparent-inputs` on the
engine and move refunds to the engine's ZIP-320 **ephemeral** allocator
(`reserve_next_n_ephemeral_addresses`), freeing external scope for receives. The
founder chose this. Probing the locked engine then surfaced a **disqualifying
constraint**:

- The ephemeral allocator is **gap-limited** (default 10;
  `zcash_keys-0.14.0/.../gap_limits.rs` — `reserve_next_n_addresses` returns
  `ReachedGapLimit` once `gap_limit` unused addresses are outstanding).
- A refund address must be reserved at **quote** time, not execute time: the
  NEAR-Intents quote binds `refundTo` and the service verifies it
  (`swap/service.rs` — `quote.refund_to != sent.refund_address` is rejected). So
  every OutOfZec **quote** reserves one.
- A **successful** swap never funds its refund address, so the ephemeral gap
  never advances. After ~`gap_limit` lifetime OutOfZec swaps the wallet can
  **no longer quote OutOfZec at all** — a hard production ceiling.

The retired custom counter had no such ceiling (unbounded non-hardened index
space). Raising the gap limit only moves the wall and links more ephemeral
addresses at the light-wallet server. So full engine ownership of refund
allocation is unsound; the founder confirmed reverting it (2026-06-22).

## Decision

1. **Transparent RECEIVE address = external scope, index 0**, the account's
   canonical receiver. Derived **purely from the stored account UFVK's
   transparent component** (`UnifiedFullViewingKey::transparent()` →
   `AccountPubKey::derive_external_ivk()` → `default_address()` → P2PKH encode),
   exactly as the shielded `current_address` derives from the stored UFVK — no
   seed, so it works in `None`-persistence. Stable, KAT-pinned, HD-recoverable,
   never key material. New core `Wallet::current_transparent_address()`.

2. **Swap REFUND addresses keep the custom external-scope counter** (ADR-0527,
   `refund_index.rs`) — **unbounded** allocation, HD-recoverable, no engine gap
   limit. The counter is **floored at index 1** (`REFUND_INDEX_FLOOR = 1`): index
   0 is permanently reserved for the receive address, so a refund never lands on
   the user's main receive address (a HARD-H privacy link). Receives (external 0)
   and refunds (external 1, 2, 3, …) are collision-free by construction.

3. **`transparent-inputs` is enabled on the engine crates** (`zcash_keys`,
   `zcash_client_sqlite`, `zcash_client_backend`) so the account UFVK carries a
   transparent component (the receive-address source) and the engine can scan
   transparent UTXOs (Recv-2b). The G7 shielded-only **UA is unchanged**: our
   `encode_default_address` request is still `Require`/`Require`/**`Omit`**.
   Pool-crossing-off on the user send path is unchanged (the only transparent
   spend is privacy-positive shielding — Recv-3).

## Consequences

**Positive**
- Unbounded refund allocation (the reason the custom counter is kept) — a heavy
  swapper never runs out of fresh refund addresses.
- The receive address is the account's canonical engine-tracked external
  receiver, so Recv-2b detection covers it natively.
- Receives and refunds never collide — index 0 reserved, refunds from 1.
- ADR-0527's HD-recoverable, never-recycle refund posture is preserved intact;
  this ADR adds the receive address and the one-index reservation, nothing more.

**Negative / accepted**
- **Enabling `transparent-inputs` narrows the account seed-length contract.**
  `UnifiedSpendingKey::from_seed` now also derives the transparent key (BIP32
  `ExtendedPrivateKey::new`, 16/32/64-byte master seeds only), so the shielded
  account derivation, previously 32..=252, now accepts exactly **{32, 64}** — the
  two real wallet seed lengths (32 raw, 64 BIP39). An exotic in-range length
  (48/100/252) now returns a typed `KeyDerivation`, **never a panic** (the <32
  panic floor is still guarded). This *unifies* the constraint with the refund
  path's pre-existing BIP32 master-seed rule; the seed-bound tests assert it.
- **Refund DETECTION is gap-limited regardless of allocation scope.** The engine
  scans external addresses up to its external gap limit (10) from the last used
  address, so live auto-shield of refunds (§6.3, Recv-2b) covers low-index
  refunds; beyond the gap a refund is HD-recoverable (ADR-0527), not live-
  detected. This is an inherent light-wallet limit, not unique to this design.
- **No installed base to migrate (founder-confirmed).** Pre-GA; no wallet has
  issued an external-scope refund index, so reserving index 0 for receive
  collides with nothing. Wallets provisioned before this change carry a stored
  UFVK without a transparent component and must be re-provisioned to gain a
  transparent receive address — acceptable pre-GA.

**Rejected alternative — full engine ownership (refunds on ephemeral scope).**
Disqualified by the gap-limit ceiling above: per-quote reservation of gap-limited
ephemeral addresses, none funded by successful swaps, permanently blocks OutOfZec
quoting after ~10 lifetime swaps. The audited ephemeral allocator is the right
brick for the wallet's OWN two-phase TEX spends (which fund the address
immediately), not for inbound swap refunds reserved per quote.
