# 0013 — Build the ZEC wallet as an extraction-ready Flutter Wallet SDK (internal now, standalone later)

- **Status:** Accepted (founder, 2026-06-06)
- **Date:** 2026-06-06
- **Links:** [0005](0005-zcash-wallet-librustzcash-direct.md) (librustzcash-direct, amended here) ·
  arch/overview.md §3 · PRODUCT_VISION.md § ZEC module / § Business model

## Context

The Zcash ecosystem has Swift and Kotlin wallet SDKs (over `libzcashlc` →
librustzcash) but **no good Flutter wallet SDK** — Flutter devs hand-roll
FFI. Relim's wallet module already *is* "librustzcash-direct core + a
flutter_rust_bridge Dart surface," so packaging it as a reusable SDK is
near-incremental, fills a real community gap, is a natural Zcash Community
Grants (ZCG) fit, and matches "build reusable bricks" (principle 11). Crucially
it also matches our hardest invariant: a good wallet SDK keeps the **seed and
spending keys in Rust** and exposes only addresses/balances/send/history/
sync-status to Dart — "keys never in Dart," made a selling point.

## Decision

Build the wallet as a **self-contained, extraction-ready Flutter Wallet SDK**
— but **inside the Relim workspace for now** (founder: "internal first,
extract later"), **architected as separable from day one** (founder:
"separable from day one"):

- **Separable-by-design:** the wallet crate(s) have a clean public Dart API
  and **no Relim-specific types/deps leaking in**. The SDK takes a
  seed/spending-key **as input** (or generates one) — Relim feeds it the
  domain-separated `relim/wallet/zip32-seed/v1` sub-secret (ADR-0011/§5.1);
  a third-party app feeds its own mnemonic. Same package, two consumers.
- **Keys stay in Rust** — the Dart API never receives seed/spending keys;
  this is the SDK's headline security property (most Flutter wallet attempts
  leak keys into the JS/Dart heap).
- Relim consumes it behind `WalletSyncPort` (ADR-0005), feature-gated.
- Built on **librustzcash-direct** with the 6 ADR-0005 refinements (built-in
  sync+tor, Zaino-gRPC, ZIP 302/321 memos, version pinning, Tachyon-ready).
- **Extraction is a planned, low-cost future move:** once a prototype proves
  it, split to a standalone open-source repo + pub.dev package; **the ZCG
  grant decision is made then, with a working artifact in hand** (founder:
  "decide after a prototype exists"). The separable-from-day-one discipline
  is what keeps that extraction cheap.

## Alternatives considered

- **Own repo + pub.dev now + apply for ZCG now:** maximum ecosystem signal,
  but pays second-repo + public-API-stability + grant-application overhead
  before the thing is proven. Deferred, not rejected — it's the likely
  endpoint post-prototype.
- **Internal, not separable (just a Relim crate):** fastest to Relim's
  wallet milestone, but bakes in Relim coupling and makes extraction a
  painful refactor that usually never happens. Rejected.

## Consequences

- The wallet crate (working name `relim-wallet`; **public package name TBD at
  extraction** — chosen to be neutral/ecosystem-friendly, not Relim-branded,
  and trademark-checked so it doesn't imply official Zcash endorsement) is
  written to SDK-grade API discipline from the first commit.
- A small, clearly pre-1.0 public Dart API surface; keys-in-Rust enforced and
  documented as the security property.
- Extraction + pub.dev publish + ZCG application are tracked as a
  post-prototype work package (decoupled from Relim's timeline; revisits the
  "no grants now" stance only for this deliverable, only after a prototype).
- Reusable beyond Relim across the Convergence suite and the wider Flutter/
  Zcash ecosystem. **The SDK also carries a `SwapPort` (ADR-0014): a
  ZEC-centric swap on-ramp via NEAR Intents 1Click** — making a swap-capable
  Flutter ZEC Wallet SDK an even bigger community/grant deliverable.
- Crypto-touching → implementation routes through crypto-change review; external
  audit before the wallet ships GA (vision Risk 4).
