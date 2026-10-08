# 0548 — Wallet transport: Tor for a host without a transport is an optional plugin that registers through the dialer contract

- **Status:** Accepted (founder, 2026-09-16, 23:50 — D1 "probably dialer-tor?", D2 "ok, let's do as you suggest", D3 "yes", D4 "yes, make sure you take into account all the iOS/Android limitations and properly find the best solution") — **S283: the FR-5 spec this ADR named "to be written" exists (`docs/specs/tor-plugin.md`, Draft); Decision 3's delegation is answered there (the built-in runtime is REMOVED); this ADR also DISCHARGES [ADR-0547](0547-wallet-transport-the-host-names-its-transport-no-predefined-kinds.md) Decision 4's conditional clause and [ADR-0526](0526-wallet-rides-host-network-infrastructure-via-netdialer.md)'s `tor-builtin` follow-up (both status lines link here; `README.md`'s back-annotation rule)** — **S284 (2026-09-17): Decision 1 is NARROWED by [ADR-0550](0550-wallet-transport-dialer-tor-is-developed-in-this-repository-and-pushed-when-needed.md) — the crate is developed in THIS repository (`sdk/dialer-tor`) and consumed by path first; the public repository and the crates.io publish are the founder's later act ("when we need, we push it"); the destination stands**
- **Date:** 2026-09-16
- **Supersedes:** [ADR-0542](0542-wallet-transport-host-dialer-first-builtin-arti-optional.md)
  Decision 2 **in part**: its gate ("a feature-gated build, `tor-builtin`, off
  by default") and its state surface ("exposed through the same `TorPolicy` /
  `TorState`") are replaced by this ADR; its strongest clause — arti is
  consumed as the shared module both products use, never forked into the SDK
  — **stands**. ADR-0542 Decisions 1 and 3 stand.
- **Links:** [ADR-0543](0543-wallet-transport-crossing-is-a-registered-dialer-no-port.md)
  (the registered dialer — the contract this plugin uses) ·
  [ADR-0544](0544-wallet-transport-crossing-corrections-from-the-design-audit.md)
  Decision 7 (built-in arti yields by precedence — restated below for a
  plugin) · [ADR-0547](0547-wallet-transport-the-host-names-its-transport-no-predefined-kinds.md)
  (the descriptor the plugin fills) · `docs/plan/fr5-session-brief.md` ·
  `docs/specs/host-transport-crossing.md` §3 · the FR-5 spec (to be written:
  `docs/specs/tor-plugin.md`)

## Context

A pub.dev consumer with no transport of its own gets clearnet or must write
native code to register a dialer. ADR-0542 Decision 2 answered that with a
cargo feature inside the SDK. Two facts changed the shape:

1. A cargo feature cannot be chosen per consuming app through cargokit: the
   app-side `cargokit_options.yaml` carries no feature switch, and
   `extra_flags` lives in the plugin's own `cargokit.yaml`, so "off by
   default" would mean either every consumer pays arti's binary size or two
   builds of `zec_wallet` under two names, with a forked UI package.
2. The two products are moving to separate public repositories, and the
   SDK's extraction policy (spec §1.2) allows only crates.io / pub.dev
   dependencies. The shared arti module must therefore be a published crate,
   not a path into the Relim monorepo.

The founder's direction was "the same approach that Relim uses now for Tor":
Relim registers a dialer through the FR-29 contract; its Tor crate
(`crates/transport-tor`) is a standalone dialer over arti whose own module
doc names the SDK's `NetDialer` port as the seam it was shaped for.

## Decision

1. **The shared Tor dialer is the crate `dialer-tor`, in its own public
   repository (`alvorado-lab/dialer-tor`), published to crates.io.** Relim's
   `crates/transport-tor` moves there under that name (arti pinned, onion
   features off, its tests and deny policy with it); both products consume
   it by version. No "flutter" in its name: it is Rust.
2. **Tor for a host without a transport is an optional second Flutter
   plugin, `zec_wallet_tor`**, which bundles `dialer-tor` in its own native
   library, runs its own async runtime, and at its init REGISTERS itself with
   the wallet through the existing contract (`zec_wallet_register_net_dialer`
   with a descriptor named "Tor", readiness = arti's bootstrap, isolation
   `supported`, exposure `hidden`; readiness and retire pushes through
   `zec_wallet_net_dialer_notify`). Whoever adds the package gets Tor;
   whoever does not pays nothing. The host configures
   `TorPolicy.required(runtime: TorRuntimeConfig.hostDialer())` exactly as
   for any registered dialer, and reads the state as `TorRuntimeKind.hostDialer`.
3. **`zec_wallet` stays Tor-free.** Nothing arti-related is compiled into
   it. The core's `tor-builtin` feature and `TorRuntime::BuiltIn` are decided
   by the FR-5 spec: kept only if a pure-Rust host needs them, otherwise
   removed. The SDK spec's §1.8 / §2.3 / §3.2 text describing a
   librustzcash-`tor` runtime inside the SDK is retired by that spec.
4. **The second-Tor-client rule, restated for a plugin.** ADR-0542 said a
   second Tor client beside a host dialer is a defect; ADR-0544 Decision 7
   said built-in arti yields by precedence. For a plugin the SDK cannot see
   arti, only the registry slot, so the rule moves to the plugin's init
   order: **register first, bootstrap only on success.** The registry is
   first-wins and token-gated (the header's rule); a bare `register` into a
   taken slot returns `ZW_RC_OCCUPIED`, and on that code the plugin must not
   start arti and must report itself as not speaking. A host that registers
   its own dialer AND adds the plugin therefore runs one Tor client, never
   two — and which one speaks is decided by who registered first, which the
   host controls through its init order. The FR-5 spec names the test.
5. **The host surface and the mobile behaviour** (D3, D4): bridge lines may
   be supplied by the host (bounded and classified as `dialer-tor` does);
   arti's state and cache live under the wallet's data directory,
   backup-excluded like the DB; bootstrap progress and blockage flow through
   the descriptor into the existing chip. The behaviour across suspend,
   resume, cold and warm bootstrap on iOS and Android is a **researched**
   section of the FR-5 spec, with sources, not a default; the starting
   position is rebuild-on-resume with arti's persisted directory cache, and
   under `required` there is no silent fallback, ever.

## Alternatives considered

- **A cargo feature inside `zec_wallet`** (ADR-0542 Decision 2 as written):
  cannot be selected per consumer through cargokit; every consumer pays the
  size or the SDK forks into two names. Lost.
- **Two builds of `zec_wallet`, with and without Tor:** two package names,
  two import paths, a forked `zec_wallet_ui`. Lost.
- **A build-time environment door** (like the device-timing one): fragile
  for a third party who never reads the SDK's build script. Lost.
- **The shared crate inside the SDK repository:** contradicts "Relim
  messenger will be in another repo" only weakly, but the founder chose a
  separate repository for the shared thing, which also keeps the SDK's
  public surface Tor-free. Lost to D1.
- **librustzcash's own `tor` feature:** a second TLS path, and it bypasses
  the readiness gate, the isolation tokens, the keepalive and the stall
  detector FR-29 built on the `NetDialer` path. Lost (and retired from the
  spec).

## Consequences

- The FR-29 contract gains its second consumer, which is the best test of
  the contract; the fake-host pattern runs in reverse (the SDK's real registry
  with the plugin's dialer over loopback).
- `dialer-tor` must be on crates.io before `zec_wallet_tor` can be published;
  `zec_wallet` does not depend on it and publishes independently. During
  development both consumers use a `[patch.crates-io]` path.
- Relim's side: the crate move and its dependency flip (board row FR-5a,
  task H-15). The plugin is not on Relim's path.
- Binary size and bootstrap time are measured on the plugin, per ABI, and
  published in the release checklist §B before the package is offered
  (ADR-0542's consequence, carried over).
- `docs/specs/wallet-sdk.md` §1.8 / §2.3 / §3.2 carry a supersession marker
  until the FR-5 spec rewrites them.
