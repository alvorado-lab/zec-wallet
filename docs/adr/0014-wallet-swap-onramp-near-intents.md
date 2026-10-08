# 0014 — Wallet swap on-ramp via NEAR Intents (1Click), behind a SwapPort trait

- **Status:** Accepted (founder, 2026-06-06) — the **phasing consequence**
  ("NEAR integration ships v1.x behind the MiCA gate") is **superseded by
  [0025](0525-near-intents-swap-in-sdk-pluggable-feature.md)** (founder,
  2026-06-13: the adapter ships in-SDK as a pluggable feature; exposure is
  host policy); all other decisions here stand
- **Date:** 2026-06-06
- **Links:** [0005](0005-zcash-wallet-librustzcash-direct.md) · [0013](0013-wallet-as-extraction-ready-flutter-sdk.md) ·
  PRODUCT_VISION.md § ZEC module / § What Relim is NOT · docs.near-intents.org/integration/.../1click-api/sdk

## Context

The wallet should let users **swap any coin → ZEC (and ZEC → out)** so they
can fund/spend private payments from whatever assets they hold. The chosen
rail is **NEAR Intents 1Click** (verified 2026-06-06): native ZEC across 30+
chains, 100+ tokens→ZEC, ~$700M ZEC volume already routed, a native **Rust
SDK** (alongside TS/Go), and it's the rail the leading ZEC wallet **Zodl**
(ex-Zashi) already integrates. Flow: **quote → deposit to a
quote-generated `depositAddress` → solver network settles → monitor
(SUCCESS/REFUNDED/FAILED)**. JWT integrator credential (avoids a 0.2% fee);
**no KYC**. Semi-custodial **during the swap only** — funds are with the
settlement network in-flight; the user's wallet and Relim never custody.

## Decision

- **Add a swap capability to the wallet SDK, ZEC-centric (founder):** the
  wallet stays a **private-payments wallet with a swap on-ramp**, *not* a DEX
  or multi-asset portfolio. ZEC is the home asset; swaps fund/spend it.
- **Behind a `SwapPort` trait** in the Flutter Wallet SDK (ADR-0013).
  **NEAR Intents 1Click (Rust SDK) is the first provider**, but the port
  keeps it swappable/removable — we are not hard-bound to one third-party
  network (consistent with "interchangeable"; not self-hostable, so isolation
  matters). Part of the extraction-ready SDK surface.
- **Keys stay in Rust:** the deposit-send transaction is signed in Rust; the
  Dart API exposes quote / execute / status / history, never keys.
- **Optional, feature-gated, honest-off:** lives in the decoupled ZEC module;
  zero swap traffic when off.
- **Regulatory: folds into the existing MiCA gate (founder).** Same machinery
  as pay-to-contact — per-jurisdiction feature flag; **blocked until the MiCA
  legal opinion clears**, now scoped to cover **swap-as-integrator** (we call
  NEAR's API as a "distribution channel"; we don't custody, match orders, or
  set prices — but the opinion must confirm we're not a CASP). May be **off
  in some iOS App Store builds** (App Store 3.1.5 "exchange only where
  licensed"); full in Android/sideload.
- **Privacy is surfaced honestly (honest-degradation):**
  - swap **into** shielded ZEC is privacy-*positive* (you end up shielded) —
    but the solver still sees the cross-asset link *at the swap boundary*;
  - swap **out** of shielded ZEC **de-shields** and exposes amounts/addresses
    to the solver network;
  - the deposit-address + JWT model means the swap provider sees per-swap
    metadata. The UI states this per swap.

## Alternatives considered

- **Full multi-asset wallet + general swaps:** rejected — turns Relim into a
  Web3/portfolio app, largest regulatory surface + many-chain key management.
- **Defer swaps entirely:** rejected as the answer (kept as the v1.0a posture
  though — see Consequences: the SwapPort is designed early, NEAR integration
  ships v1.x post-MiCA).
- **Raw REST / TS SDK:** unnecessary — a Rust SDK exists; integrate natively.
- **Hard-wire NEAR Intents (no port):** rejected — a single non-self-hostable
  third-party dependency must be abstracted + swappable.

## Consequences

- **Phasing:** the `SwapPort` abstraction is designed with the wallet SDK
  (separable-from-day-one, ADR-0013); the **NEAR Intents integration ships
  v1.x**, gated behind the MiCA opinion (alongside pay-to-contact), not v1.0a.
- New third-party runtime dependency (NEAR Intents solver network) — contained
  to the optional swap feature of the optional wallet module, honest-off,
  port-swappable; never in the messaging trust path.
- Integrator **JWT credential** handling is a config item (distributed/rotated
  via the signed network manifest or app config) — flagged for the wallet spec.
- **Positioning nuance:** "Not a Web3/token project" becomes "ZEC-centric
  wallet with a third-party swap on-ramp — not a DEX/portfolio" (vision
  updated).
- **Strategic upside:** a *swap-capable* Flutter ZEC Wallet SDK is a bigger
  community gift and opens **NEAR's grant/ecosystem** alongside ZCG (revisit
  post-prototype with ADR-0013).
- Crypto/funds-touching → implementation via crypto-change review + external audit
  before GA; the in-flight refund/slippage/failure states are named tests.
