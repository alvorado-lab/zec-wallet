# 0526 — Ride the host app's network infrastructure via the `NetDialer` port

- **Status:** Accepted — **the "built-in arti (cargo feature `tor-builtin`)" impl this ADR keeps as an optional follow-up is DISCHARGED by [ADR-0548](0548-wallet-tor-for-standalone-hosts-is-an-optional-plugin-through-the-dialer-contract.md) Decisions 2–3 (back-annotated S283, `README.md`'s back-annotation rule): the standalone path is the `zec_wallet_tor` plugin registering through the FR-29 contract, and the feature is removed by `docs/specs/tor-plugin.md` D3. The `NetDialer` port and the `Dialer` runtime this ADR froze stand unchanged.**
- **Date:** 2026-06-14
- **Links:** docs/specs/wallet-sdk.md §2.3 / §3.2 / §3.2a · ADR-0005 (librustzcash-direct) · ADR-0013 (wallet as extraction-ready SDK) · ADR-0006 (obfuscation engine — sing-box / Xray / VLESS direction) · ADR-0014 / ADR-0525 (swap rides the same stack)

## Context

The wallet must reach a `lightwalletd`/Zaino endpoint over gRPC (sync, broadcast)
and, when swap is enabled, a provider over REST. Relim already runs its **own**
privacy transport — Tor today, and **Xray / VLESS (or other obfuscated
transports) tomorrow** per ADR-0006 — under a single network stack the user has
already chosen, paid the battery/bootstrap cost for, and trusts.

A wallet that spawned its **own** Tor/arti runtime would: duplicate that stack
(two bootstraps, two circuit pools, double the always-on network surface),
**couple the wallet to one specific transport** (it could never ride Xray/VLESS
without new wallet code), and carry heavy arti weight in every build. Third-party
SDK consumers, conversely, may bring Orbot / system Tor, a corporate SOCKS proxy,
or no privacy transport at all. No single built-in transport serves both the
Relim composition and the third-party SDK, and hard-coding "Tor" anywhere in the
wallet would make every future Relim transport a wallet change.

What breaks if we don't decide this explicitly: the gRPC seam (W3-inc-2c-ii) is
the first code to construct a network channel, and it would default to tonic's
own dialer + TLS — a second, parallel network path outside the host's control,
violating the honest-off invariant and the "minimal-server, metadata-minimal"
posture.

## Decision

**All wallet network traffic rides a host-provided `NetDialer` byte-stream port**
— `dial(host, port, isolation_key) -> AsyncByteStream` (spec §3.2). The SDK is
**protocol-agnostic**: it asks for a byte stream to `host:port` and **never knows
or names** whether the host routes it through Tor, Xray, VLESS, a SOCKS proxy, or
clearnet — all are indistinguishable to the wallet.

`TorRuntime::Dialer(Arc<dyn NetDialer>)` is the **first-class path** — Relim hands
the wallet its existing transport. Built-in arti (cargo feature `tor-builtin`) and
`ExternalSocks5` are alternative impls of the **same** `TorPolicy` contract, not
special cases. The sync engine builds its tonic gRPC channels (and the swap REST
client) over whatever the dialer returns via `connect_with_connector`; **the SDK
owns TLS** over the dialed stream (`tokio-rustls` + `webpki-roots`), so tonic's
own TLS is never invoked on the dialer path. There is exactly one place all
wallet traffic enters the network — the connector — so "no independent clearnet
path" is structural, and `TorPolicy::Required` fail-closes by construction.

## Alternatives considered

- **Built-in arti as the only/default runtime.** Duplicates Relim's transport,
  couples the wallet to Tor, and forces arti weight into every build. Rejected as
  the default; **kept as an optional `tor-builtin` impl** of the same contract for
  standalone SDK consumers who want a batteries-included Tor.
- **The SDK opens its own TCP sockets directly.** No host-infra reuse, no Tor/Xray,
  and it makes honest-off unenforceable (traffic outside the host's kill switch).
  Rejected. The one direct dialer that ships is used solely for the explicit
  `TorPolicy::Off` clearnet choice — never a hidden fallback.
- **Per-protocol dialer traits (a `TorDialer`, a `VlessDialer`, …).** Couples the
  SDK to each host protocol and re-introduces exactly the coupling this decision
  removes. Rejected in favor of one opaque `AsyncByteStream` port.
- **tonic's built-in connector + TLS.** A second network path the host can't see
  or route; would need per-transport plumbing anyway. Rejected — we feed tonic a
  pre-dialed (and self-TLS'd) stream via `connect_with_connector`.

## Consequences

- **Easier:** the wallet reuses Relim's single transport (Tor → Xray → VLESS → …)
  with zero duplication and zero wallet changes per new transport; isolation keys
  flow straight through to the host's circuit/path mapping; the SDK carries zero
  arti weight unless `tor-builtin` is enabled; one TLS path (`tokio-rustls`)
  across sync gRPC and swap REST.
- **Harder:** the host owns the torness *attestation* — the SDK can verify it
  handed bytes to the dialer but cannot prove the host actually routed them
  (spec §2.3 trust boundary; `TorState` reports which runtime is speaking so a UI
  can attribute the claim honestly).
- **Frozen:** the `NetDialer` / `AsyncByteStream` port shape; the "SDK owns TLS,
  tonic's TLS unused on the dialer path" rule; `TorPolicy::Required` ⇒ zero
  clearnet, never a fallback.
- **Follow-ups:** the `ExternalSocks5` and `BuiltIn` (arti) dialer impls; the
  connector + policy-resolver code (the W3-inc-2c-ii seam this ADR governs).
