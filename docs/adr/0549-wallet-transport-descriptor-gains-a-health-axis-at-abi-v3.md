# 0549 — Wallet transport: the descriptor gains a closed `health` axis at ABI v3, so a registrant can say FAILED

- **Status:** Accepted (founder, 2026-09-17 ~02:00 — "yes introduce fourth
  closed descriptor and so on", on FR-30 (b) as priced in
  `docs/handoff/host-feature-requests.md` § FR-30 "SDK answer")
  · **Correction 1 (S290, append-only; the axis and the ABI unchanged):**
  Decision 3's last sentence — "`health` never permits a dial, never
  suppresses `fellBack`, never triggers `Preferred`'s fallback (ADR-0546):
  FAILED is fail-closed, like NOT_READY and RETIRED" — and the Links line's
  "FAILED is not a reachability failure and never triggers `Preferred`'s
  fallback" no longer hold in their third clause. ADR-0553, built S290 on
  THIS axis (its correction 1), makes a FAILED declaration the one refusal
  that counts as the private path failing: the gate refuses it as
  `DialError::TransportFailed`, and under `Preferred` a minute of it — from
  the declaration — is switched away from, visibly. The first two clauses
  stand (it never permits a dial of the host's transport, never suppresses
  the latch); `NOT_READY` and `RETIRED` keep the absolute form; `Required`
  is unchanged.
- **Date:** 2026-09-17 (S283)
- **Supersedes:** [ADR-0547](0547-wallet-transport-the-host-names-its-transport-no-predefined-kinds.md)
  Decision 3's count — "With `readiness`, three closed integers remain" —
  becomes four. Everything else in ADR-0547 stands (the host names its
  transport; no predefined kinds; `exposure` and `isolation` closed).
- **Links:** FR-30 (Relim, S401 — the finding: a readiness of 0 renders as
  *starting* for a transport the host declared FAILED) ·
  [ADR-0545](0545-wallet-transport-host-is-trusted-isolation-requested-not-demanded.md)
  Decision 3 (the descriptor is authoritative and only makes the wallet MORE
  restrictive — unchanged) · [ADR-0546](0546-wallet-transport-preferred-falls-back-on-unreachable-or-timeout-only.md)
  (unchanged: FAILED is not a reachability failure and never triggers
  `Preferred`'s fallback) · [ADR-0548](0548-wallet-tor-for-standalone-hosts-is-an-optional-plugin-through-the-dialer-contract.md)
  (the second registrant that needs this — `docs/specs/tor-plugin.md` E21)
  · `docs/specs/host-transport-crossing.md` §2 / §3.1 / §3.4 / §3.5 (the
  shape this ADR changes at the chunk that builds it) ·
  `sdk/zec_wallet/rust/include/zec_wallet_net_dialer.h` (the contract; the
  version becomes 3 when the field lands)

## Context

The v2 descriptor is `{ name, readiness, isolation, exposure }`. Its only
health axis is `readiness`, and the state derivation maps every value below
100 to `TorState::Bootstrapping` — so a host whose transport has FAILED can
only floor readiness to 0, and the wallet renders *"starting"* with no end.
Relim's registrant does exactly that (`FAILING_READINESS = 0`) and ships the
result as a stated limit; the SDK's own Tor plugin (ADR-0548) has the same
gap in its `Failed` phase. FR-30 (b) named it; the SDK priced two options —
a closed axis at a new ABI version, or the stated limit — and recommended
the axis, bundled with the plugin's build so both registrants and the walk
land on one final contract.

## Decision

1. **The descriptor gains a fourth closed integer, `health`:**
   `ZW_HEALTH_STARTING` (0) — the transport is coming up; `ZW_HEALTH_READY`
   (1) — carrying; `ZW_HEALTH_FAILED` (2) — the registrant has judged its
   transport failed and is not merely slow. Every other value is
   `ZW_RC_DESCRIPTOR`. `readiness` keeps its meaning (0..=100; the SDK dials
   only at 100): a host that declares readiness 40 still reads a bootstrap in
   progress; `health` says whether that bootstrap is alive.
2. **`ZW_NET_DIALER_ABI_VERSION` becomes 3.** A v2 copy of the header fails
   `register` with `ZW_RC_ABI` before any pointer is read (the header's own
   rule: a change to any value or rule is a new version). Relim builds
   H-13 / H-12 / H-5 against v2 now and rebuilds against v3 before H-6, so
   the founder-attended walk runs once, on the final contract.
3. **Rendering.** `live_tor_state` reads `health == FAILED` AHEAD of the
   readiness arm and renders `TorState::Unavailable` — with the descriptor's
   name once FR-30 (a) lands, and a neutral next step ("turn the private path
   off, or check your app's network settings"). `health` never permits a
   dial, never suppresses `fellBack`, never triggers `Preferred`'s fallback
   (ADR-0546): FAILED is fail-closed, like NOT_READY and RETIRED.
4. **Where it lands: the FR-5 spec's stage 3, chunk C1** (the same
   regeneration that removes the `BuiltIn` runtime and carries FR-30 (a) and
   (c)) — one regen, one review. The plugin pushes `health = FAILED` in its
   `Failed` phase and nothing else about it changes; Relim's `descriptor.rs`
   sends `health = FAILED` with its MEASURED readiness instead of the floor.
5. **Named tests move with the shape:** the header/Rust agreement (T15), the
   descriptor refusals (T27), the plugin's mirror (P14) re-watched; a new row
   pins "FAILED renders `Unavailable` at readiness 40" and its polarity
   "STARTING at readiness 40 renders `Bootstrapping`".

## Alternatives considered

- **A `readiness` sentinel** (0 = failed, 1..=99 = starting). Changes a
  frozen field's rule without a new version — the header forbids it — and a
  v2 host that pushes 0 for "from zero" would suddenly read as failed. Lost.
- **A flag on `notify` beside `retire`.** Also v3, and it moves health out
  of the descriptor the wallet renders from. Lost.
- **The stated limit.** Two registrants would carry the same lie into the
  first device walk, and a second walk on v3 would follow. Lost — the
  founder's ruling.

## Consequences

- The crossing spec's descriptor shape, its §3.1 verb table, §3.4's
  derivation order and §3.5's header text are revised at C1, not before;
  until then the spec's head note records this ADR.
- `docs/handoff/host-session-complete-integration.md` H-13 carries the
  notice for Relim (build against v2 now, v3 before H-6).
- The plugin spec's E21 stops being a stated limit at C1.
