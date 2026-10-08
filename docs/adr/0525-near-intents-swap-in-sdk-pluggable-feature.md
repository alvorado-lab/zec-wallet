# 0525 — NEAR Intents swap ships IN the SDK as a pluggable feature

- **Status:** Accepted (founder, 2026-06-13)
- **Date:** 2026-06-13
- **Links:** [0014](0014-wallet-swap-onramp-near-intents.md) ·
  [0013](0013-wallet-as-extraction-ready-flutter-sdk.md) ·
  `docs/specs/wallet-sdk.md` §1.2/§1.7/§2.6/§3.2/§3.5 ·
  github.com/defuse-protocol/one-click-sdk-rs (reference)

## Context

ADR-0014 chose NEAR Intents 1Click as the swap rail behind a provider-
agnostic `SwapPort`, but **deferred the NEAR adapter itself to v1.x behind
Relim's MiCA gate**. That phasing conflated two different things:

1. **SDK capability** — whether the published Flutter ZEC Wallet SDK ships a
   working swap implementation; and
2. **Host exposure** — whether a given app (Relim included) shows swap to its
   users in a given jurisdiction/store.

The SDK is extraction-ready and community-facing by design (ADR-0013); a
"swap-capable Flutter ZEC SDK" was already named there as the bigger
community gift. Zodl — the leading ZEC wallet, whose swap UX the spec mined
in detail (wallet-sdk §1.7) — ships full NEAR-Intents swap today; an SDK
whose swap story is "bring your own adapter" is not competitive with that.
Meanwhile the W2 design already built everything around the seam: `SwapPort`,
`SwapService` orchestration, the Zodl-parity DTOs (disclosures, under-
deposit, slippage, realized slippage, destination rules), the four kill
layers, and a scriptable `MockSwapProvider` driving the named tests.

## Decision (founder, 2026-06-13)

1. **The NEAR Intents 1Click adapter is an SDK deliverable NOW, not v1.x.**
   It lands as the already-specced separate crate
   (`crates/relim-wallet-swap-near`, wallet-sdk §1.2) and is part of the
   extraction unit. The phasing consequence of ADR-0014 ("NEAR integration
   ships v1.x, gated behind the MiCA opinion") is **superseded by this ADR**;
   everything else in ADR-0014 stands.
2. **Pluggable, not bundled-mandatory.** The `SwapPort` seam is unchanged.
   A host that wants no swap gets none at every layer (§3.5 kill layers,
   unchanged): compiled out ⇒ not one byte; compiled in but no provider
   constructed ⇒ `wallet.swap == null` in Dart, nothing shown, zero swap
   traffic (honest-off at the socket level). A host that enables it gets the
   **full Zodl-parity swap capability** — quote, deposit rules, under-deposit
   handling, bounded status tracking, refunds, realized slippage, history,
   and per-direction privacy disclosures as renderable data.
3. **Regulatory exposure is HOST policy.** The SDK ships the capability and
   the honest disclosures; whether to show swap is each host's regulatory
   call. **Relim-the-app keeps its MiCA per-jurisdiction gate exactly as
   ADR-0014 defined it** (PRODUCT_VISION § ZEC module's "v1.x" exposure
   timing for Relim is unaffected). Third-party SDK consumers own their own
   posture — same as every other wallet SDK on the market.
4. **Client choice ratified: thin hand-audited typed REST client** speaking
   the 4-endpoint 1Click HTTP API (`get_tokens` / `get_quote` /
   `submit_deposit_tx` / `get_execution_status`), riding the SDK's
   `NetDialer`/`TorPolicy` stack (no independent clearnet path — the m5
   review rule). This ratifies the W2 review-H1 decision at ADR level and
   supersedes ADR-0014's "Raw REST unnecessary — a Rust SDK exists" note:
   the official `one-click-sdk-rs` (verified 2026-06-13: defuse-protocol
   GitHub org, OpenAPI-generated, reqwest-based) is **reference material
   only, not a dependency** — a funds path deserves a hand-audited client,
   §4.6 demands bounds-before-DTO-entry the generated client can't give us,
   and an injected-reqwest transport could not honor the NetDialer contract.

## Alternatives considered

- **Keep the v1.x deferral:** rejected by the founder — the SDK without a
  working swap is a weaker community artifact, and the deferral protected
  Relim's regulatory posture, which host-side gating protects equally well.
- **Consume `one-click-sdk-rs` directly:** rejected (review H1, now
  ADR-level) — reqwest-based transport bypasses `NetDialer`/Tor, response
  fields are unbounded `String`s (violates §4.6), and the funds path warrants
  a hand-audited surface. Kept as the wire-shape reference.
- **Bundle the adapter into the core crate behind a feature:** rejected —
  the separate-crate boundary IS kill layer 1's strongest form (not linked ⇒
  not in the binary) and isolates the adapter's HTTP/TLS deps from the
  always-on core.

## Consequences

- The wallet track's next ungated chunk is the adapter crate + its bridge
  feature (`swap-near`) + Dart wiring + example-app demo flow (the
  production-grade plan lives in wallet-sdk §3.2/§8 named rows and the
  wallet-log W-plan).
- Supply-chain pass for the adapter's HTTP/TLS/JSON deps lands WITH the
  crate; the wire contract is pinned by recorded fixtures from the real API
  (UPSTREAM.md discipline, testing-patterns §5).
- External audit before GA (ADR-0014 consequence) **stands** — funds path.
- **Kill-layer prerequisites carried here so the adapter implementer sees
  them in one place** (3-lens review fold, 2026-06-13): layer 3 (remote
  manifest flip) MUST NOT deploy before ADR-0012's manifest revocation
  design lands — an unrevocable "on" flag is a resurrect vector (§3.5
  layer-precedence rule); layers 1+2 are the operative gates until then.
  And the kill-severity plumbing (manifest flag → `SwapService` WindDown/
  Hard) is a NAMED composition gate owed at the bridge/Dart chunk
  (`manifest_kill_severity_reaches_swap_service`, spec §8) — a partially
  delivered manifest must never leave the severity undefined.
- **Desktop is in scope, stated not assumed** (standing desktop discipline):
  the adapter's transport contract is `NetDialer`, identical on every
  platform — a desktop host without `tor-builtin` satisfies it exactly as it
  does for sync (ExternalSocks5, or its own dialer impl); the SDK ships no
  clearnet dialer of its own (§2.3 TorPolicy posture), and swap traffic gets
  no special path.
- JWT integrator-credential handling (config, `Zeroizing`, never logged)
  stands as specced; without it the provider levies its 0.2% fee — hosts
  choose.
- The §1.2 license note changes: the generated client's Unlicense is no
  longer inherited; the adapter's license surface is ours.
