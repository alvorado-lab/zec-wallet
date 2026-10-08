# 0541 — Ship the wallet SDK as one Dart package with the native library vendored, and never build Tor into it

- **Status:** Accepted
- **Date:** 2026-09-07
- **Links:** founder decision, S257 ·
  [ADR-0526](0526-wallet-rides-host-network-infrastructure-via-netdialer.md)
  (all wallet traffic rides a host-provided `NetDialer`; this ADR closes the
  question that ADR left open for a Dart-package consumer) ·
  [ADR-0002](0002-consume-vs-copy-mostpost-shared-crates.md) (Relim consumes
  convergence crates as path-deps — the same posture applied to the wallet) ·
  `docs/handoff/host-integration-readiness.md` §"Relim consumes the wallet at
  the RUST level" · `docs/handoff/sdk-publishing-plan.md` ·
  `docs/handoff/zcg-feeler-post-2026-07-12.md` (the public commitment this
  ADR keeps) · `docs/plan/production-readiness.md` Phase 4

## Context

Two questions had no ADR, were answered differently by two documents, and
between them blocked all of the packaging phase.

**1. How is the SDK distributed?** `sdk/zec_wallet` passes
`dart pub publish --dry-run` today and produces a package **nobody can build**.
`pub publish` ships only files inside the package directory, but
`sdk/zec_wallet/rust/Cargo.toml` reaches outside it three ways: four inherited
`[package]` fields, nine inherited `[workspace.dependencies]` entries, and two
path dependencies pointing at the wallet engine — so the engine is not in the
tarball at all. Reconstructing the exact tarball and asking cargo to parse it
gives `failed to find a workspace root`.

The public ZCG post commits to *"the native library **bundled inside the Dart
package** rather than published as separate crates."* The package README says
the remedy is *"inlined manifests **+ a published core crate**."* Those are
different products. Ticket #323 closed by producing a plan, not by deciding, and
none of the forty existing ADRs covers distribution.

**2. Does the SDK carry its own Tor?** ADR-0526 already establishes that the host
owns the transport and that the SDK is protocol-agnostic — it asks for a byte
stream to `host:port` and never knows whether that is routed over Tor, Xray,
VLESS or clearnet. It left the SDK's own arti as *"an OPTIONAL, not-yet-compiled
follow-up"* (FR-5, #398), and the roadmap has carried FR-5 since as "the only
genuinely unshipped feature". Meanwhile the one shipping host — Relim — already
carries a full Tor stack of its own.

The S257 sweep found the practical consequence of leaving this open: the only Tor
runtime a **Dart-package** consumer can express is `TorRuntime::ExternalSocks5`,
it is documented to Dart as shipped, and it does not work. `runtime_dialer`
returns `UnsupportedRuntime` for it, and because `resolve_dialer` propagates with
`?`, the `Preferred` policy fails **before** constructing its fallback — so it
never degrades to clearnet with a visible `fellBack` the way its own doc promises.
The error then folds to a *retryable* sync fault, so the wallet spins forever
behind an indicator blaming the user's Orbot.

## Decision

**1. One package, native library vendored. Not published as separate crates.**
The SDK ships as the Dart package with the engine's source inside it: the four
inherited `[package]` fields and nine workspace dependencies are inlined, and the
two escaping path dependencies are resolved by including the engine in the
package rather than by publishing it to crates.io. This keeps the commitment
already made in public. **`zec-wallet-core` and `zec-wallet-swap-near` are not
published to crates.io**, and the `publish = false` already on the bridge crate
stays.

**Sequencing, and it is part of the decision: not now.** Relim consumes the
wallet at the Rust level (see Decision 2), so nothing Relim needs is blocked by
this. The vendoring work is Phase 4 of `docs/plan/production-readiness.md` and
stays ordered behind the money-path and gate phases.

**2. The SDK never builds Tor in. The host provides the transport.**
`TorRuntime::Dialer` — the host handing the wallet an `Arc<dyn NetDialer>` — is
**the** supported Tor path, and it works today. Relim already carries Tor; a
second copy inside the wallet would cost binary size for a capability the host
has. **FR-5 / #398 (built-in arti) is therefore not wanted for the host
integration** and is removed from the critical path rather than left standing as
an unshipped feature.

**3. A Dart-package consumer has no Tor, by design, and must be told so.**
`TorRuntime::Dialer` is an in-process Rust API and cannot cross the FFI bridge, so
the Rust-consumer path is the only one that gets host-routed traffic. The
consequences the SDK owes:

- `ExternalSocks5` must **fail at configuration validation** with a distinct,
  **non-retryable** error, not at dial time as a retryable sync fault. A
  configuration that cannot work must be refused when it is set, not spun on
  forever.
- Every doc comment describing `ExternalSocks5` as shipped is corrected, including
  `wallet.rs`'s *"Today's supported configs (Off / Dialer) never hit this"* —
  false, because `Dialer` is not expressible from Dart.
- The supported set for a Dart-package consumer is stated plainly as **`Off`**.

## Consequences

- **Phase 4 is unblocked as a decision and still deferred as work.** The next
  session may plan it without asking again.
- **T0-6 shrinks from large to small.** It is no longer "build a SOCKS5 dialer";
  it is "refuse an impossible config honestly and stop claiming it works".
- **The roadmap loses its last unshipped feature.** FR-5 was carried as the only
  genuinely unshipped capability; it is now explicitly out of scope for the host,
  which changes what "feature complete" means for the SDK.
- **A third-party standalone host with no Tor stack of its own gets clearnet
  only.** That is the accepted cost of Decision 2. If such a consumer ever
  materialises, reopening FR-5 is a new ADR, not a revision of this one.
- **`zec_wallet_ui`'s path dependency on `zec_wallet` still blocks its own
  publication** and is unaffected by this ADR; it is Phase 4 work.
