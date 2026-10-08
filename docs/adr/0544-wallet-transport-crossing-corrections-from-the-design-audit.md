# 0544 — The registered-dialer crossing, corrected by its design audit: isolation is a capability, the registry is replaceable, built-in arti yields, and two shipped code paths are prerequisites

- **Status:** Accepted (audit-forced corrections to ADR-0542/0543's wording;
  one decision deferred to the founder, §Deferred) — **Decision 7 ("built-in
  arti yields by precedence") is SUPERSEDED by [ADR-0548](0548-wallet-tor-for-standalone-hosts-is-an-optional-plugin-through-the-dialer-contract.md)
  Decision 4 (the plugin yields by init order; back-annotated S283 per
  `README.md`'s back-annotation rule — the S282 fold had recorded this link
  in ADR-0548 only)**
- **Date:** 2026-09-16 (early; the audit ran 15 Sep 23:40 → 16 Sep 00:15 WEST)
- **Supersedes:** [ADR-0543](0543-wallet-transport-crossing-is-a-registered-dialer-no-port.md)
  Decision 2's sentence "the SDK does not know or care whether the bytes
  ride Tor, Shadowsocks, VLESS…", Decision 3's "refused after the wallet has
  dialed once", and its Consequence "Relim's part is exactly its existing
  crates behind one registration call"; and
  [ADR-0542](0542-wallet-transport-host-dialer-first-builtin-arti-optional.md)
  Decision 2's "refused at validation" clause for `tor-builtin` beside a
  registered dialer. Everything else in both ADRs stands: host dialer first,
  no port, the dialer abstracts the host's transports, built-in arti
  optional, FR-29 next.
- **Links:** the three design-audit tables (`docs/handoff/s280-resume.md`
  §6, "The design audit") · `docs/REVIEW.md` · [ADR-0526](0526-wallet-rides-host-network-infrastructure-via-netdialer.md)
  (the isolation contract, unchanged) · `host-feature-requests.md` FR-29,
  FR-15, FR-17, FR-18 · `docs/specs/wallet-sdk.md` §5.4 (the NEVER-log list)

## Context

ADR-0542 and ADR-0543 were written from the founder's rulings in one
evening. The founder then asked for the approach to be audited before the
spec. Three read-only passes (security, architecture, crypto/privacy) on
the ADRs, the pre-spec assessment, the seed-port precedent and Relim's
host code found four BLOCKER-class facts and refuted five sentences. None
overturns a founder ruling; each corrects something I wrote around one.

1. **Every production dial carries an isolation key**, and ADR-0526's port
   requires a transport that cannot isolate to refuse. Relim's Shadowsocks
   dialer refuses every keyed dial. So over Shadowsocks or VLESS the wallet
   as wired today completes zero connections: `Required` fails closed on
   the first dial, `Preferred` falls to clearnet on the first dial. "The
   SDK does not know or care which transport" is false for non-isolating
   transports.
2. **The swap adapter's isolation key IS the provider's deposit address**
   — a §5.4 never-log item — passed verbatim, provider-chosen and
   unvalidated beyond length. Across the boundary it would be copied into
   the host and retained in its isolation table; a hostile provider can
   return a key that collides with the sync circuit. The other keys are
   semantic labels (`wallet-sync`, `wallet-send-…`) that tell a registry
   owner which dial is a broadcast.
3. **The plaintext-loopback exemption survives the crossing.** `http://`
   is allowed for a loopback name and TLS is chosen by scheme alone; under a
   host dialer the name is resolved remotely, so a loopback endpoint would
   ship raw transactions and compact blocks unencrypted through the host's
   transport.
4. **"Refused after the wallet has dialed once" breaks the one shipping
   host**: Relim replaces or tears down its dialer at every carrier switch
   and on honest-off. It is also a denial primitive (one provoked dial
   before registration locks the wallet out) and moot while the wallet's
   policy is fixed at open (`set_tor_policy` is specified, not built).
5. **The seed-port precedent is weaker than the ADRs said**: no
   `catch_unwind` on any SDK export (an RNG failure in the token mint would
   abort the HOST process); and Relim registers only the unbound v1 verb —
   no FR-17 binding, no token — treats the SDK's fatal `-2` as retryable,
   and its panic-wipe deregistration calls a clear FR-18 removed, discarding
   the failing return while its doc says the port is cleared.

## Decision

**1. Isolation is a capability the host declares, decided at validation,
never per dial.** The transport descriptor carries an isolation tri-state
(supported / unsupported / unknown) beside kind and readiness. `Required`
on a non-isolating transport is REFUSED at configuration, naming the
transport — never a per-dial refusal storm, never a fallback. What
`Preferred` and `Off` do on a non-isolating transport is the founder's
(§Deferred). Until ruled, the abstraction claim is narrowed: the registered
dialer serves every transport that can isolate per key (Tor via arti today)
and refuses honestly on those that cannot.

**2. Isolation keys are SDK-minted opaque tokens.** Fixed-width, random,
domain-separated per purpose and per swap, held SDK-side and never derived
from network input, a txid or an address. No caller-meaningful string
crosses the boundary. The swap adapter's use of the deposit address is a
defect in shipped code and is fixed BEFORE the crossing ships (a named test
pins that no address byte reaches `dial`).

**3. No plaintext over a registered dialer.** `http://` endpoints are
refused at configuration whenever the resolved runtime is not the SDK's own
direct dialer. TLS stays inside the SDK with WebPKI roots and SNI equal to
the endpoint host, and a registrant can never influence it — this is what
bounds a hostile dialer to metadata.

**4. The registry is first-wins and token-gated, and REPLACEABLE.** "Refused
after first dial" is dropped. The host registers one stable trampoline
dialer at trusted init — the same point as the seed port, BEFORE wallet
configuration is validated — and swaps its backing transport behind it.
Replace and clear are token-gated (the FR-18 `update` shape); on either,
every in-flight stream fails with a typed error, the descriptor flips, and
the transport state re-derives. The host pushes readiness and retire to the
SDK through the ABI; a retire completes every outstanding operation with an
error. The wallet's policy becomes mutable in-session (`set_tor_policy`
built, or the dialer resolved from the registry at dial time) with the
honest-off test on the TRANSITION.

**5. Errors cross as a frozen numeric table**, one code per SDK `DialError`
variant plus `NotReady`, which is not a reachability failure and never
triggers `Preferred`'s fallback; the table names which codes may reach the
direct dialer. The host maps its own taxonomy onto it.

**6. The boundary is hardened beyond the precedent.** `catch_unwind` at
every SDK export and every SDK-side completion callback, mapped to a typed
code, with a planted-panic test per entry; added to the seed-port exports in
the same change. A generation-tagged handle table; SDK-owned buffers alive
past cancel until the host acknowledges; clear blocks on an outstanding-
operation count. Every length crossing validated (`0 ≤ n ≤ remaining`)
before a read buffer advances; a fuzz target over lengths and a hostile
stream. The descriptor is a fixed-width struct of closed discriminants (kind
with an `Other` arm, readiness 0..=100, isolation tri-state, a "shares a
transport instance with the host" flag); no free text reaches Dart. The
descriptor may only make the SDK MORE restrictive — it never permits a
dial, suppresses `fellBack`, or changes what is passed to `dial`. The
transport state reports attributed wording, never a torness claim.

**7. Built-in arti yields by precedence, not validation.** A registered
dialer wins; built-in arti never starts while one is registered;
`TorRuntimeKind` reports which is speaking. Validation refuses only
`BuiltIn` configured with the feature off. Feature unification is guarded so
a workspace consumer cannot silently link arti; the SDK keeps its explicit
`ring` provider and the FR-19 symbol posture is re-verified with a second
TLS stack resident.

**8. Relim's part is a C-ABI stream shim over its dialers, an error
mapping, the trampoline, and the readiness/retire push** — not "one
registration call". Its Tor and Shadowsocks dialers implement the port's
signature; the error type and the stream type do not cross.

## Deferred to the founder

**Non-isolating transports under `Preferred` and `Off`.** When the messenger
is on Shadowsocks or VLESS, may the wallet ride the same tunnel with its
connections linkable to each other at the proxy (an explicit, visible
opt-out that dials without a key and reports a state that is not
"private"), or does it refuse and say so? The audit priced both; the spec
carries the ruling.

## Consequences

- Two shipped-code prerequisites precede FR-29's crossing: the isolation
  token (Decision 2) and the `http://` refusal (Decision 3), each with its
  named test; plus `catch_unwind` on the existing seed-port exports.
- Named tests owed, beyond ADR-0542/0543's: `Required` on a non-isolating
  descriptor is refused at validation · `cabi_unsupported_is_not_a_
  reachability_failure_and_never_falls_back` · `deregister_with_open_streams_
  fails_them_closed` · `a_stale_generation_handle_is_refused` · `host_retire_
  fails_inflight_streams_and_moves_tor_state` · a second registration refused
  · an empty registry under `Required` never falls back · `http_endpoint_
  over_a_host_dialer_is_refused` · no `SwapId`/address byte reaches `dial` ·
  a planted panic at every export returns a code · a registered dialer cannot
  influence TLS config · a policy change mid-session moves the wallet
  (honest-off on the transition).
- Host obligations filed to Relim (the request file H-11…H-13): adopt the
  bound, token-gated seed-port registration and fix the deregistration;
  refuse dials after retire in its Tor dialer; register the trampoline at
  trusted init and map its errors to the table.
- The stated residual, for the host's Network screen to tell the truth: a
  registry owner sees destination, timing, key and byte volume; never
  content or keys.
