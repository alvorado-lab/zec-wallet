# 0547 — The host names its transport at registration; the SDK carries no predefined transport kinds

- **Status:** Accepted — **Decision 4's conditional clause ("the one name the SDK owns is its own built-in Tor, FR-5, when it exists") is DISCHARGED by [ADR-0548](0548-wallet-tor-for-standalone-hosts-is-an-optional-plugin-through-the-dialer-contract.md) Decisions 2–3 (back-annotated S283, per `README.md`'s link-both-ways rule): the built-in never exists; FR-5 is a registrant that names its transport "Tor" through the descriptor like any host, and the SDK owns no transport name (`docs/specs/tor-plugin.md` §0 A11). Decision 3's count ("three closed integers remain") is SUPERSEDED by [ADR-0549](0549-wallet-transport-descriptor-gains-a-health-axis-at-abi-v3.md): a fourth, `health`, at ABI v3 (founder, 2026-09-17).**
- **Date:** 2026-09-16 (S280, after the FR-29 wave landed and before any host built against the header)
- **Supersedes:** [ADR-0544](0544-wallet-transport-crossing-corrections-from-the-design-audit.md)
  Decision 6's "kind with an `Other` arm" in the descriptor's shape, and
  [ADR-0545](0545-wallet-transport-host-is-trusted-isolation-requested-not-demanded.md)
  Decision 2's "the transport state reports the host's KIND as fact" —
  the host's transport is still reported as fact; its NAME is the host's,
  not a closed enum of the SDK's. Everything else in both ADRs stands.
- **Links:** founder, S280 (2026-09-16, ~11:00, verbatim below) ·
  `docs/specs/host-transport-crossing.md` §2, §3.4, §3.5 (the descriptor —
  to be revised) · `sdk/zec_wallet/rust/include/zec_wallet_net_dialer.h`
  (the contract — revision notice at its head) · FR-5 (the SDK's own arti,
  the one transport the SDK names itself)

## Context

The wave shipped the descriptor as three closed integers — `kind` (Direct,
Tor, Shadowsocks, Vless, Other), `readiness`, `isolation` — and the reference
UI rendered the kind's name ("via your app's private path (Shadowsocks)").
Asked why the wallet UI knows about types of connection, the founder ruled:

*"I think that host app defines the name of the transport and set it while
registering in wallet sdk. So there should not be predefined names I guess.
Only for a scenario when wallet sdk carries its own tor support."*

No wallet decision depends on the kind (the spec says policy branches on
readiness and isolation only); the kind existed to print a name — host
vocabulary that couples the SDK to transport types and needs an SDK release
to name a new one.

## Decision

1. **The transport's name is the host's.** The host supplies a bounded,
   validated display name (UTF-8, ≤ `ZW_TRANSPORT_NAME_MAX_BYTES`, no control
   characters, non-empty) when it registers and whenever it pushes a
   descriptor; the SDK renders it verbatim and never interprets it.
2. **No predefined transport kinds in the SDK.** The closed `kind` enum
   (`HostTransportKind` in core, its Dart mirror, the header's
   `ZW_TRANSPORT_KIND_*`) is retired.
3. **The privacy semantics stay closed.** The wallet still needs two facts it
   cannot infer from a name: whether the path hides the device's address
   from the server (`exposure`: hidden / exposed / unknown — a plain
   connection behind the host's trampoline is "exposed" and renders "not
   private") and whether connections are isolated from each other
   (`isolation`, unchanged). With `readiness`, three closed integers remain.
4. **The one name the SDK owns is its own built-in Tor** (FR-5, when it
   exists): `TorRuntimeKind::BuiltIn` names itself; a host-registered dialer
   never does.

## Consequences

- The contract changes before any host builds against it: the header's
  descriptor struct gains the name (a fixed `uint8_t name[…]` + `name_len`)
  and `exposure`, loses `kind`; the ABI version becomes 2 so a stale copy of
  this morning's header fails loudly at registration (`ZW_RC_ABI`). Relim's
  H-13/H-12/H-5 build the verbs and the vtable now and the descriptor
  against v2 (the handoff carries the notice).
- Core: `HostTransportDescriptor { name, readiness, isolation, exposure }`;
  `TorRuntimeKind::HostDialer { name, isolation, exposure }`; the door and
  `live_tor_state` unchanged in shape. Bridge: the mirror carries a
  `String` name (host-chosen text is DTO-safe: bounded, validated, never
  logged). UI: the copy keeps its `{transport}` parameter, fed by the host's
  name; "not private" renders from `exposure`, not from a kind; the
  unattributable forward-compat case collapses to `exposure == unknown`
  (caution). The §5 descriptor event logs readiness, isolation and exposure
  — never the name.
- Named tests move with the shape (T9, T13, T15, T19 and the header
  agreement); every cited line the change moves is re-watched.
- Sized S–M; the first item of the next SDK session, ahead of stage 5's
  device walk, so the host never builds a struct the SDK is about to retire.
