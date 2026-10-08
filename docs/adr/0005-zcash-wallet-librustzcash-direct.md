# 0005 — Build the ZEC module on librustzcash directly, feature-gated

- **Status:** Accepted (founder review round 2, 2026-06-06) — confirmed after
  a building-block re-examination (Zodl/zingolib/warp/Zallet), with 6 refinements below.
  **Packaging amended by [ADR-0013](0013-wallet-as-extraction-ready-flutter-sdk.md):**
  built as an extraction-ready Flutter Wallet SDK (internal now), not a
  Relim-internal-only crate. **Scope extended by
  [ADR-0014](0014-wallet-swap-onramp-near-intents.md):** adds a ZEC-centric
  swap on-ramp (NEAR Intents) behind a `SwapPort`. **Crate versions amended by
  reference in [ADR-0540](0540-ironwood-nu63-crate-wave-and-sealed-orchard-pool.md):** the
  exact pins named in the Decision paragraph below are the pre-Ironwood set and
  are historical; ADR-0540 carries the current one.
- **Date:** 2026-06-05 (confirmed + refined 2026-06-06)
- **Links:** docs/PRODUCT_VISION.md § ZEC module ·
  [ZEC wallet-blocks re-examination](../research/2026-06-06-zec-wallet-blocks.md)

## Context

The vision commits send/receive/tips in v1.0 inside a decoupled optional
module. Research (verified 2026-06-05): librustzcash is active and
Rust-native (`zcash_client_sqlite` ~0.20.2, commits days old, nu6.2 line);
`zcash-light-client-ffi` exists only to reach Swift/Kotlin — unnecessary for
a Rust host; `zip32` 0.2.1 provides shielded HD derivation incl.
`zip32::arbitrary` (hardened-only, no ecosystem coordination needed). The
backend is mid-transition (zcashd→Zallet, lightwalletd→Zaino, Zashi→Zodl
rebrand); zec.rocks operates public light-wallet endpoints.

## Decision

`relim-wallet` crate consumes **librustzcash directly** (`zcash_client_backend`
0.23.0 + `zcash_client_sqlite` 0.21.0 + `zcash_keys` 0.14.0 + `zip32` 0.2.1 +
`orchard`/`sapling-crypto`; all MIT OR Apache-2.0), feature-gated and
per-jurisdiction flagged. Orchard shielded addresses; wallet derivation
branches from the same BIP39 seed (zip32 inside the module; the messenger
core keeps its HKDF tree per ADR-0001).

### Six refinements from the 2026-06-06 re-examination

1. **Use the built-in `sync` + `tor` features — do NOT hand-roll** the scan
   loop or Tor routing. `zcash_client_backend` ships a sync state machine and
   an Arti Tor client; the `tor` feature directly satisfies "Tor-routable,
   honest-off" (honest-off = stop the traffic, not just the indicator).
2. **Server target = the lightwalletd-compatible gRPC `CompactTxStreamer`
   protocol, not a specific binary.** Public infra is migrating
   lightwalletd+zcashd → **Zaino + Zebra** (Zaino speaks the same gRPC).
   Default to a Zaino-capable public endpoint; user-overridable/self-hosted;
   failover via Nodus; endpoints listed in the signed network manifest.
3. **Memo rail uses standards:** **ZIP 302** typed memos (leading byte ≥0xF5
   for Relim's structured tip/payment envelope, never colliding with UTF-8
   text) + **ZIP 321** payment-request URIs for "request a tip" links.
   ZIP 231 (>512B memo bundles) is future headroom. Shielded recipient
   required for memos → honest-degradation UI when sending to transparent.
4. **Maintenance-risk watch (NEW):** the ECC engineering team resigned
   Jan 2026 and formed ZODL (Feb 2026); crates stay in `zcash/` and release
   on schedule (nu6.2, 2026-06-03), but governance/repo ownership is in flux.
5. **Version-churn discipline:** `zcash_client_backend` went 0.20→0.23 in
   four months (nu6.x semver-major bumps) — pin exact versions, absorb
   breaks in the one adapter crate.
6. **Strategic watch — Project Tachyon** (oblivious sync, ~NU7/summer 2026):
   replaces trial-decryption with private queries (~100× smaller, PQ
   privacy) — keep sync behind a swappable port so it drops in later.

## Alternatives considered (2026-06-06 re-examination)

- **Zodl (ex-Zashi) / zcash-light-client-ffi / mobile SDKs:** rejected —
  Swift/Kotlin only; reusing them adds an FFI hop on top of Rust and puts
  wallet state outside Rust. (Mine the Zashi/Zodl SDK source as a reference
  implementation of "embed librustzcash in a wallet.")
- **Zallet** (ECC's new Rust wallet): rejected — its own repo says "not
  designed to be used as a Rust library"; it's a full-node daemon (OpenRPC),
  the wrong shape for a mobile light client.
- **zingolib:** rejected — not on crates.io (git-only, pins a bleeding-edge
  librustzcash fork → loses our version control); a redundant layer over the
  same crates. Watch for code ideas.
- **warp / zcash-warp (Ywallet fast-sync):** rejected despite genuinely
  faster bulk-parallel sync — **no LICENSE file (`license: null`)** is a hard
  blocker under the MIT-clean policy; single-maintainer, unpublished. Mine
  for sync-speed ideas only.
- **Self-hosted lightwalletd/Zaino as *default*:** rejected as default —
  needs a synced full node behind it; stays a documented self-host option.

## Consequences

- Large, fast-moving dependency surface — pinned versions, isolated module,
  the messenger never links it unless the feature is on.
- **Metadata honesty:** a third-party lightwalletd sees wallet sync
  activity. The module spec must cover: honest-off (zero sync traffic when
  off), endpoint choice surfaced in Expert Mode, Tor/proxy routing option.
- Watch the lightwalletd→Zaino transition; the connectivity backend is a
  swappable adapter behind a port.
- External audit of the module before GA (vision Risk 4); ZCG remains the
  natural funder if grants are ever revisited.
