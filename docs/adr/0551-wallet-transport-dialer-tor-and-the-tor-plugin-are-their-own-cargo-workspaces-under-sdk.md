# 0551 — Wallet transport: `dialer-tor` and the Tor plugin's crate are their OWN cargo workspaces under `sdk/`, never members of the SDK's

- **Status:** Accepted (measured at the resolver, S284, 2026-09-17 ~15:10;
  the arch angle on FR-5 C0 asked that this be its own ADR rather than an
  inline correction of [ADR-0550](0550-wallet-transport-dialer-tor-is-developed-in-this-repository-and-pushed-when-needed.md)
  — `docs/adr/README.md`'s back-annotation rule allows a LINK, not a rewrite)
- **Date:** 2026-09-17 (S284)
- **Supersedes:** [ADR-0550](0550-wallet-transport-dialer-tor-is-developed-in-this-repository-and-pushed-when-needed.md)
  Decision 1 **in part** — its "a MEMBER of the `sdk/` cargo workspace (the
  `apple-secure-enclave` shape)" and the P26-exempts-members clause. The
  rest of ADR-0550 stands: developed here, consumed by path, `publish =
  false`, the two H-15 additions made here, the public repository and the
  publish the founder's later act.
- **Links:** `docs/plan/fr5-phase-1.md` §0.3 / §1 / §5 D-7, D-8 / §7.1 (the
  measurement and the run record) · `docs/specs/tor-plugin.md` §2, §8 (the
  test-placement consequence) · `sdk/Cargo.toml` (the `exclude`) ·
  `sdk/dialer-tor/Cargo.toml` (the `[workspace]` root and why) ·
  `sdk/zec-wallet-swap-near/tests/supply_chain_policy.rs::every_excluded_workspace_under_sdk_has_its_own_carriers`
  (the guard this ADR's Consequences owe)

## Context

ADR-0550 placed `dialer-tor` in the `sdk/` workspace as a member. The first
`cargo metadata` refused it: arti 0.45.0's `tor-netdoc` requires `cipher
^0.5.2`, which requires `crypto-common ^0.2.2`; the ZEC stack the SDK pins
(`zcash_primitives =0.30.1`) requires `crypto-common =0.2.0-rc.1`. Cargo
treats the two as one semver line and cannot lock both. It is the gate-0
conflict that carved `sdk/` out of the root workspace (`sdk/Cargo.toml`'s
header), now between `sdk/` and arti. The plugin's crate, which depends on
`dialer-tor`, meets the same wall, and so would the bridge crate the moment
it named the plugin as a dev-dependency — its dev-graph is resolved in
`sdk/`'s lock.

## Decision

1. **`sdk/dialer-tor` is a cargo workspace ROOT** (`[workspace]` in its
   manifest), `exclude`d from `sdk/Cargo.toml`'s workspace, with its own
   committed `Cargo.lock`, its own `deny.toml`, inline pins (arti `=0.45.0`
   with its feature set, `tor-rtcompat =0.45.0`, `rustls =0.23.45` ring-only)
   and a hand-copied lint table. `sdk/zec_wallet_tor/rust` (FR-5 C3) takes
   the same shape for the same reason and consumes the dialer by path.
2. **A crate outside the SDK workspace is carried by hand, and a guard
   checks the hand.** Every `--workspace` gate (`sdk-clippy`, `sdk-lint`'s
   fmt/clippy/doc, `pins_policy`, `extraction_policy`'s scans, P26) and the
   live-carrier guard are blind to it by construction. Each excluded
   workspace gets its own legs — `sdk-gate-core` (test), `sdk-audit`
   (`cargo audit`, `cargo deny check`), `sdk-lint` (fmt, clippy, doc),
   lefthook (`rust-fmt`'s third workspace, a `<name>-clippy` lane) — and
   `every_excluded_workspace_under_sdk_has_its_own_carriers` reads the
   `exclude` array as its sole input and refuses a workspace root that any of
   those lines fails to name.
3. **The bridge crate never dev-depends on the plugin.** The spec's §8
   placement of the registry-touching rows (P2's real half, P3, P19, P28)
   inside the bridge's cabi test module is withdrawn; the plan re-cuts it at
   C3's start between a two-library integration test in the plugin's own
   workspace (both cdylibs loaded by symbol — production's shape) and the
   fake-registry half alone.
4. **Pins outside `sdk/Cargo.toml` are policed where they live.** The
   crate's `deny.toml` bans and floors mirror the SDK's; a lockstep reader
   for its arti pins against Relim's root is owed when Relim's `main` and
   `wallet-track` share a tree (C2 names the landing spot).

## Alternatives considered

- **Keep the ZEC stack and arti in one lock by moving the ZEC stack to the
  stable RustCrypto line.** A money-path crate wave (the Ironwood wave's
  shape), not a transport chunk's call. Lost for now; the day it happens the
  exclusion can be reversed.
- **A `[patch]` or a vendored copy of `crypto-common`.** Forbidden by the
  extraction policy and by `sdk/Cargo.toml`'s own rule. Lost.
- **Leave the excluded workspaces uncarried by any guard.** Two of them are
  about to exist, and the copy-paste of six legs is exactly the kind of hand
  list the live-carrier guard was built to end. Lost.

## Consequences

- `sdk/Cargo.toml` carries `exclude = ["dialer-tor", "zec_wallet_tor/rust"]`
  with the resolver note; `sdk/dialer-tor/Cargo.toml` carries `[workspace]`
  and the same note.
- The guard in Decision 2 lands with this ADR (FR-5 C0's fold), watched red
  against a deleted leg.
- The spec's §2 and §8 carry dated notes; the plan's §0.3 and §5 D-7/D-8
  say the same thing; ADR-0550's status line links here.
- The plugin's test placement is a C3-start decision with one arch review
  pass on the choice.
