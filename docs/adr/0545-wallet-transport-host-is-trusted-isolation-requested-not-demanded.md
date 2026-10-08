# 0545 — The host is trusted: its transport is used as the host uses it; isolation is requested where the host can honor it, never demanded

- **Status:** Accepted
- **Date:** 2026-09-16
- **Supersedes:** [ADR-0544](0544-wallet-transport-crossing-corrections-from-the-design-audit.md)
  Decision 1's "`Required` on a non-isolating transport is REFUSED at
  configuration" and its §Deferred; Decision 6's "the descriptor may only
  make the SDK MORE restrictive" and "attributed wording, never a torness
  claim"; Decision 2's per-purpose token remedy (the swap-key half stands).
  For HOST-PROVIDED dialers only, the `NetDialer` port clause "implementations
  that cannot honor isolation MUST return `DialError::Unsupported` rather
  than silently linking" (`ports.rs:32-36`, spec §2.3, under
  [ADR-0526](0526-wallet-rides-host-network-infrastructure-via-netdialer.md))
  is retired; it stays for the SDK's own dialers and for built-in arti.
  Everything else in ADR-0542/0543/0544 stands.
- **Links:** founder decision, S279 (2026-09-16, ~00:25; verbatim below) ·
  FR-12 / FR-15 (the seed port — the host already supplies the spend key) ·
  the design-audit tables (`docs/handoff/s280-resume.md` §6) ·
  `docs/handoff/host-session-complete-integration.md` H-5, H-12, H-13

## Context

The design audit found that every wallet dial demands isolation and that a
non-isolating host transport (Relim's Shadowsocks dialer) refuses keyed
dials, so the wallet would complete no connection over it. ADR-0544 made
isolation a host-declared capability, refused `Required` on such a
transport at configuration, and deferred `Preferred`/`Off` to the founder.
Several audit remedies also treated the host as a potential adversary at
the seam: the descriptor "may only tighten", the transport state "never a
torness claim", purpose labels hidden from the registry owner.

The founder ruled: *"the host app is treated as trusted. So whatever
transport it provides, it may be used as it is used internally."*

That is already the SDK's trust model at the seed port: a host that hands
the wallet its spend key for every signature (FR-12/FR-15) is trusted
completely. Trusting the host with the seed but not with the destinations
of its own network stack was an inconsistency the audit inherited from the
threat model's third-party framing. The token-gated registry defends against
NON-host code in the same process (an injected or third-party library), which
is cheap and remains.

## Decision

**1. The host's transport is used as the host uses it.** Whatever the host's
dialer does for the messenger it does for the wallet: Tor with per-key
circuit isolation, Shadowsocks or VLESS without isolation, direct. The
three policy arms keep their meaning against the HOST's transport:
`Required` = the host's transport, failing closed only when the host reports
not-ready or retired — never refused at configuration for lack of
isolation; `Preferred` = the host's transport with the visible clearnet
fallback on unreachable/timeout only; `Off` = the SDK's direct dialer.

**2. Isolation is requested, not demanded.** The SDK passes its isolation
key on every dial as today. The descriptor declares whether the transport
honors it; a non-isolating host transport ignores the key and never
refuses for it. The consequence — the wallet's connections are linkable to
each other at a non-isolating proxy, exactly as the messenger's are — is
accepted and RENDERED: the transport state reports the host's kind as fact
and whether isolation is in effect, so the wallet UI can say "via your
app's private path (Shadowsocks); connections can be linked by the proxy".

**3. The descriptor is authoritative.** Kind, readiness and isolation drive
the SDK's policy decisions and its rendering. Its wire shape stays closed and
bounded (closed discriminants, bounded integers, no free text) — that is
memory safety across an FFI boundary, not distrust. What no descriptor can
do: change the SDK's TLS configuration, admit plaintext, or override
fail-closed on not-ready/retired.

**4. Purpose visibility to the host is accepted.** The semantic isolation
labels (`wallet-sync`, `wallet-send-…`) stay. The SDK-minted opaque token
remains for the SWAP key only, and for a different reason: the swap
provider is an untrusted network peer and must not choose a key in the
SDK's isolation namespace (the deposit-address key is still a shipped
defect, still fixed first).

**5. Plane ownership is the host's.** The wallet follows the host's
circumvention switch and its honest-off; wallet bytes count where the host
counts them. One switch covers both planes.

## Consequences

- ADR-0543's "abstracts any network the host provides" is TRUE again with
  no narrowing: the abstraction holds because the SDK stops demanding what
  a transport cannot give.
- The fail-closed matrix collapses to one rule: not-ready or retired →
  `Required` fails closed, `Preferred` falls back visibly, `Off` is
  unaffected. Unreachable/timeout keep today's semantics; `Unsupported`
  can no longer come from a host dialer for isolation.
- Named tests change: "`Required` on a non-isolating descriptor is refused
  at validation" becomes "`Required` on a non-isolating descriptor dials
  without isolation and the state reports it"; "a not-ready descriptor
  fails `Required` closed and marks `Preferred` `fellBack`" stays; "a
  registered dialer cannot influence TLS config" stays.
- Relim's trampoline (H-13) passes the isolation key to backends that honor
  it (Tor) and drops it for those that do not (Shadowsocks), reporting the
  capability in the descriptor; its Shadowsocks dialer's refusal of keyed
  dials is an implementation of the retired clause and is not exposed to
  the wallet (H-12).
- Unchanged by trust, because they are correctness across a boundary, not
  suspicion of the host: the `http://` refusal off the direct dialer, the
  frozen error-code table, the replaceable token-gated registry with the
  trampoline at trusted init, the readiness/retire push, in-session policy
  mutability, `catch_unwind` at every crossing, the generation table and
  buffer ownership, length validation, precedence for built-in arti.
- The audits' "stated residual" (what a registry owner sees) becomes
  documentation for the host's own Network screen, not a defense.
