# 0534 — Rescan from an earlier birthday (post-restore recovery, no re-seed)

- **Status:** Accepted
- **Date:** 2026-06-24
- **Links:** docs/specs/wallet-sdk.md §3.1 (provisioning lifecycle) / §3.6 (restore birthday) · ADR-0533 (the ~6-month restore-birthday default that creates this need) · account.rs:62-69 (the in-code prescription this implements) · crypto-change review

## Context

ADR-0533 makes the restore birthday default to ~6 months ago so the first sync
is fast. The money-safety follow-up (security review S105): a user restoring an
OLDER wallet who keeps the default gets a fast scan that completes ("synced")
showing a 0/partial balance, with **no in-app way to find the older funds**.
Funds are never cryptographically lost (a lower birthday recovers everything),
but "balance reads zero and sync says done" is indistinguishable from loss.

The birthday is **immutable in place**: once account 0 is imported, a later
`ensure_account` with a different birthday is a silent no-op (the first wins,
`account.rs:62-81`), and `truncate_to_height` (the reorg rewind) cannot go below
the wallet's checkpoint (`sync.rs:913`). librustzcash exposes no "lower the
birthday" / "delete account" API. The only supported way to scan earlier is to
**drop the data DB and re-import account 0 at the lower birthday** — exactly what
`account.rs:69` prescribes.

The seed lives ONLY in the keychain-sealed `seed.seal` (+ `dbkey.seal`), never in
the data DB; everything in `wallet.db`'s zcash tables (account UFVK, notes,
witnesses, scan ranges) is re-derivable from it. So a rescan can keep the seed
and rebuild scan state.

## Decision

Add a first-class **`Wallet::rescan_from(from: Option<BlockHeight>)`** (None =
full history → Sapling activation) that rebuilds ONLY the data DB at the lower
birthday from the ALREADY-SEALED seed — no mnemonic re-entry, **no seed across
the FFI** (the bridge param is a bare `Option<u32>` height).

**Reset vs keep (the load-bearing table):**

| Artifact | Action | Why |
|---|---|---|
| `seed.seal` / `dbkey.seal` / vault wrap key (keychain) | **KEEP, never touched** | the master secret; destroying it bricks the wallet |
| `wallet.db` zcash tables (account, notes, witnesses, scan ranges) | **RESET** (drop + re-import at lower birthday) | birthday immutable in place; seed-derivable |
| `wallet.db` AUX tables (queued_send_intent, refund_index, issued_quote_store, swap_destination) | **PRESERVE** (copy across) | durable money intents / in-flight swap detection — NOT scan-derived |
| `block-cache.db` | **RESET** (disposable) | re-downloaded; validated-or-deleted on open anyway |

**Mechanism (crash-atomic, lock held throughout):**
1. `require_open()`; `sync.stop().await` (stop + join — no scan may race the swap);
   None-persistence ⇒ typed `SeedRequired` up front (no seed to re-import).
2. Take ownership of `Inner` (the `WalletLock` is held continuously — no
   drop/reacquire, so no `WalletAlreadyOpen` race).
3. On the blocking pool, under the held lock: build a fresh data DB at a temp
   path via the existing `provision_db` keyed with the SAME `dbkey`; **copy the
   durable aux tables** into it; `fsync → atomic-rename` over `wallet.db`;
   `fsync(dir)` (the existing `write_atomic` discipline). Seal files + vault are
   never touched. Reset/recreate the disposable BlockCache.
4. Reassemble via `from_open(lock, store::open(...), cache, OpenConfig{birthday:
   from, ...}, freshly_generated=false)`. The reset account is gone, so the next
   sync's lazy `provision_account(client, from)` re-imports at the new birthday
   through the existing, audited bundled-frontier path (`resolve_birthday`) —
   with `freshly_generated=false`, so `None` floors to **activation, never ~tip**
   (the §2.3 silent-fund-loss guard).

A transient `Rescanning` lifecycle phase (`Open→Rescanning→Open`, and
`Rescanning→Wiped`) makes concurrent `snapshot`/`propose` return
`WalletBusy{Rescanning}`, never a torn read.

**FFI:** `WalletHandle::rescan_from(&mut self, from_height: Option<u32>)`, the
take-and-replace pattern mirroring `close`. Pair with the existing pure
`estimate_birthday(approx_date)` for the date-picker UX. NO key material crosses.

**Rejected:** `create_with_vault(SeedSource::Generate, ...)` — it (a) returns
`WalletAlreadyExists` on a complete wallet, (b) sets `freshly_generated=true` so a
`None` birthday resolves to ~tip (re-introduces the silent-loss bug), (c) its
repair mode does not re-import the account, and (d) a blind data-DB delete
orphans the aux money state. `Wallet::wipe` is the wrong tool too — it
crypto-shreds the seed (destroys exactly what rescan must keep).

## Consequences

- Crash-atomic: the seal files + vault are never mutated, so the **seed cannot be
  lost** at any interruption point; the data DB is replaced by a single atomic
  rename (always intact-old or complete-new, never torn); the aux copy is inside
  the temp DB before the rename. Mirrors the proven
  `provisioning_killed_at_any_step_recovers_idempotently` guarantee.
- Money-safety: durable aux state (pending queued sends, swap detection, the
  refund-address index) is preserved across the rescan, or a typed guard fires on
  the destructive path (the `WipeWithPendingSwap` posture).
- UI: an in-app "Rescan from an earlier date / Scan all history" recovery entry
  (date picker reusing `estimate_birthday`) closes the ADR-0533 silent-loss gap.
- crypto-change review + the full multi-angle review (crypto audit ∥ security review
  ∥ arch review, then code reviewer): it reads the sealed seed, re-derives
  spending authority, and deletes+rebuilds money-bearing state.

## Test contract (gate 8)

- `rescan_lowers_birthday_and_finds_older_notes` — provision high (skips an older
  note), `rescan_from(lower)`, the note then appears.
- `rescan_preserves_sealed_seed_and_addresses` — UA + transparent address
  byte-identical before/after; `reveal_mnemonic` unchanged; seal/vault bytes
  untouched.
- `rescan_is_crash_safe` — fault injection at each step: seed always recoverable,
  wallet always opens (old or new birthday), never bricked; retry completes.
- `rescan_full_history_equals_activation` — `rescan_from(None)` floors to Sapling
  activation via `freshly_generated=false`, NEVER ~tip.
- `rescan_preserves_pending_queued_sends_and_swap_state` — a queued_send_intent,
  a refund_index counter, and a swap_destination row survive (or the guard fires).
- `rescan_rejects_birthday_in_future` — height above tip ⇒ `BirthdayInFuture`.
- `rescan_none_persistence_is_seed_required` — no seed at rest ⇒ `SeedRequired`.
- `rescan_stops_sync_first_no_torn_scan`; `rescan_phase_gates_concurrent_calls`;
  `rescan_on_a_closed_handle_is_typed_not_a_panic` (bridge).
