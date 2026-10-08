# 0542 — Wallet transport: the host's dialer first, built-in arti only for hosts without one

- **Status:** Accepted — **Decision 2 superseded in part by [ADR-0548](0548-wallet-tor-for-standalone-hosts-is-an-optional-plugin-through-the-dialer-contract.md)** (the gate is an optional plugin, not a cargo feature; the state rides the FR-29 descriptor; the shared-module clause stands). Decision 1's shape clause was taken by [ADR-0543](0543-wallet-transport-crossing-is-a-registered-dialer-no-port.md) and Decision 2's validation clause by [ADR-0544](0544-wallet-transport-crossing-corrections-from-the-design-audit.md) Decision 7 (back-annotated S282, per `README.md`'s link-both-ways rule).
- **Date:** 2026-09-15
- **Supersedes:** [ADR-0541](0541-wallet-sdk-distribution-vendored-native-no-builtin-tor.md)
  Decisions 2 and 3 ("the SDK never builds Tor in"; "a Dart-package consumer
  has no Tor, by design"). ADR-0541 Decision 1 (one Dart package, native
  library vendored, engine crates not published) **stands**.
- **Links:** founder decision, S279 (2026-09-15, in the founder's words below)
  · [ADR-0526](0526-wallet-rides-host-network-infrastructure-via-netdialer.md)
  (the frozen `NetDialer` port — unchanged) ·
  `docs/handoff/host-feature-requests.md` FR-29 (the dylib-seam finding this
  ADR answers) and FR-5 (built-in arti, de-scoped 2026-08-16, revived here as
  optional) · Relim ADR-0031 (the wallet ships as an isolated dylib) · Relim
  ROADMAP W16 / `#779` · `docs/handoff/host-session-complete-integration.md` §2.5

## Context

ADR-0526 made the host's network stack the wallet's transport through one
frozen port, `NetDialer`. ADR-0541 then ruled that the SDK never carries Tor
itself and that a Dart-package consumer therefore has no Tor at all: the only
transport a Dart host can name, `ExternalSocks5`, was made to fail honestly at
configuration (T0-6, S267), and the supported set for that population was
stated as `Off`.

Two facts have since been established at the tree (FR-29, 2026-09-11):

1. **The host this was designed for cannot reach the supported path.** Relim
   ships the wallet as an isolated dylib (Relim ADR-0031: the Zcash stack pins
   pre-release RustCrypto, the PQ identity is on the release line, one cargo
   lock is impossible). A `dyn NetDialer` is not an ABI across two
   independently compiled libraries, so `TorRuntime::Dialer` — "the supported
   Tor path" of ADR-0541 — is unreachable from Relim. Relim runs a full Tor
   stack and gets clearnet anyway; the Network screen tells the user so in 16
   locales. ADR-0541 priced its accepted cost for hosts WITHOUT a transport;
   it is being paid by the host WITH one.
2. **Third-party hosts split into two populations**, and ADR-0541 served
   neither: those that already run a transport (Orbot, a system Tor, a VPN's
   SOCKS inbound, an Xray client) can express only `Off`; those with no
   transport at all get clearnet by design.

The founder's ruling (S279, 2026-09-15), verbatim in substance: *Tor in the
Relim beta, properly configured. When the SDK is used in the Relim host app it
must not carry its own arti; it uses the dialer the host provides, through the
interface the SDK and the host defined, which other hosts may use too. If
another host does not provide Tor, the SDK must be able to carry its own arti
and expose its configuration to the host.*

## Decision

**1. The host's transport is first-class and reachable from every packaging
model the SDK ships.** When a host provides a dialer, the wallet uses it and
never starts a second Tor client. The in-process form is ADR-0526's frozen
`NetDialer` (`TorRuntime::Dialer`), unchanged, for hosts that compile inside
the SDK's workspace. **For a host across the dylib seam — Relim, and every
Dart-package consumer — the SDK adds a crossing** so that the host-provided
half is reachable. FR-29 prices the two shapes: (1) a SOCKS5 client behind the
already-public `ExternalSocks5` arm (RFC 1928 + RFC 1929 credentials carrying
the isolation key); (2) a C-ABI dialer registration mirroring the seed-port one
(FR-15; first-wins, token-gated). **The shape is the spec's design decision**,
taken with the four review angles on the design (REVIEW.md), with the SDK
side's recommendation being shape (1) first and shape (2) only if the
isolation or loopback properties of (1) prove insufficient. Whichever ships,
`TorPolicy::Required` stays fail-closed and `Preferred`'s visible `fellBack`
behaves as specified; `Off` remains honest.

**2. Built-in arti is an OPTIONAL capability for hosts that have no
transport, never the default and never used beside a host dialer.** FR-5 is
revived as a feature-gated build (`tor-builtin`, off by default) whose
bootstrap state, policy and configuration are exposed to the host through the
same `TorPolicy` / `TorState` surface, so a host without Tor can turn it on and
render it honestly. The 2026-08-16 shape stands: arti is consumed as the shared
module both products use, not forked into the SDK. A host that provides a
dialer and also enables `tor-builtin` is a configuration error the SDK refuses
at validation, not at dial time.

**3. Rank.** FR-29 (the crossing, Decision 1) is the SDK's next work item —
ahead of fork-point discovery and independent of the pub.dev name — because
Relim's v1.0b cut carries it (`#779`). Built-in arti (Decision 2) follows
FR-29 and is not on Relim's path.

## Consequences

- **ADR-0541 Decision 3's "supported set is `Off`" is retired** for a
  Dart-package consumer once the crossing ships; T0-6's honest refusal stays
  until then and becomes the refusal of a MISCONFIGURED `ExternalSocks5`, not
  of the variant.
- **Relim's part is the last step, not the first**: run a loopback SOCKS5
  inbound in front of its existing dialer (shape 1) or register its dialer
  (shape 2), move `kWalletTorPolicy` off `Off`, report the real state in
  `walletHostTransportProvider`, and walk a device with the messenger on Tor
  and no wallet connection on clearnet. Relim ADR-0031 is not reopened.
- **A second Tor client in Relim is a defect**, not a configuration: Decision
  2's validation refusal is the guard, and the spec owes a named test for it.
- **Binary size and bootstrap for `tor-builtin` are the SDK's to measure and
  publish** in `release-checklist.md` §B before the feature is offered; a host
  that does not enable it pays nothing.
- **The spec** (the spec-writing process, four angles on the design, then on the diff —
  transport and FFI are security-touching under REVIEW.md) carries the
  mechanism choice, the isolation-key contract across the seam, the
  fail-closed matrix, and the named tests. It is the next SDK deliverable.
