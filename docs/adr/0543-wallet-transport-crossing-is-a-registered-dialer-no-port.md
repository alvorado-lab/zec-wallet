# 0543 — The crossing is a registered dialer: the host opens no port for the SDK, and the dialer abstracts every transport the host runs

- **Status:** Accepted
- **Date:** 2026-09-15
- **Supersedes:** the one clause of
  [ADR-0542](0542-wallet-transport-host-dialer-first-builtin-arti-optional.md)
  Decision 1 that left the crossing's shape to the spec. Everything else in
  ADR-0542 stands.
- **Links:** founder decision, S279 (2026-09-15, ~23:30, verbatim below) ·
  [ADR-0526](0526-wallet-rides-host-network-infrastructure-via-netdialer.md)
  (the port being crossed) · `docs/handoff/host-feature-requests.md` FR-29
  (shape 2), FR-15 (the seed-port registration precedent), FR-18 (its
  first-wins, token-gated hardening) · `seed_port_cabi.rs` · Relim
  `crates/relim-net-dialer/`, `crates/relim-ffi/src/wallet_registrar.rs`

## Context

ADR-0542 made the host's transport reachable across the dylib seam and named
two shapes for the crossing: a SOCKS5 client in the SDK behind
`ExternalSocks5`, with the host running a loopback SOCKS5 inbound, or a C-ABI
dialer registration mirroring the seed port. It left the choice to the spec.

The founder ruled it the same evening: *"the host shouldn't open any network
port to be used by the SDK for Tor. It just registers its own dialer which is
used by the SDK. And this dialer is an abstraction: it abstracts any network
the host provides — Tor, Shadowsocks, VLESS or whatever else the host app
supports."*

## Decision

**1. The crossing is a registered dialer. No port.** The host registers a
dialer with the SDK across the library boundary, by name from the already
loaded image, the way it registers the seed port today (`seed_port_cabi.rs`,
resolved with `RTLD_NOLOAD` in Relim's `wallet_registrar.rs`). The SDK opens
no listener and connects to no loopback proxy on the host's behalf. The
loopback-SOCKS5 shape is **out**; `ExternalSocks5` stays refused at
validation (T0-6) and is not the third-party path either — a host without a
dialer of its own uses the SDK's optional built-in arti (ADR-0542 Decision 2).

**2. The dialer is an abstraction over whatever the host runs.** Its contract
is ADR-0526's: `dial(host, port, isolation_key) → byte stream`. The SDK does
not know or care whether the bytes ride Tor, Shadowsocks, VLESS, a direct
socket or a transport that does not exist yet; that is the host's stack and
the host's choice, per connection. What the SDK needs from the host beside
the stream is an honest **transport descriptor** — the kind and readiness the
host reports — so the wallet can render "via the host's private path" or its
absence truthfully and so `Required` can fail closed when the host's
transport is not ready. The SDK's policy and state vocabulary (`TorPolicy`,
`TorState`) generalises accordingly in the spec; the three policy arms and
their semantics (`Off` honest, `Preferred` with visible `fellBack`,
`Required` fail-closed) do not change.

**3. The registration inherits FR-18's hardening, not FR-15's first draft.**
First-wins, token-gated, refused after the wallet has dialed once. A dialer
registry is a substitution primitive — whoever owns it sees every destination
the wallet dials — so a last-wins or unauthenticated registration is a
security finding, not a convenience.

## Consequences

- The FR-29 spec designs the registration: the C-ABI (dial, read, write,
  close, cancel — async across the boundary, with the `unsafe` confined to the
  cabi module and fuzzed at the boundary per principle 7), the isolation-key
  contract, the transport descriptor, the token, and the fail-closed matrix.
  Named tests owed, beyond ADR-0542's: a registration after first dial is
  refused; a second registration is refused; a descriptor reporting "not
  ready" fails `Required` closed and marks `Preferred` `fellBack`; the
  isolation key arrives intact on the host side.
- Relim's part is exactly its existing crates behind one registration call:
  `relim-net-dialer`'s Tor and Shadowsocks dialers already implement the port
  shape, frozen to the SDK's; VLESS or anything later plugs in on the host
  side with no SDK change.
- Third-party hosts pick one of two paths: register a dialer (any transport),
  or enable built-in arti. There is no third.
