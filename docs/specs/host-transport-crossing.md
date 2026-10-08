# Spec — the host transport crossing (FR-29; the registered dialer)

**Status:** Reviewed — 2026-09-16 (S280; written FROM the three-angle design
audit of 15→16 Sep and ADR-0542/0543/0544/0545, per the founder's "just
enough for production, handling edge cases" — ONE code reviewer pass ran
the same night: three MAJORs and five MINORs, every one folded (§0 A9 →
ADR-0546; §3.2 the op-table registration order, the write re-present check
and the close/completion race with T23–T25; §3.4 the descriptor's path into
`live_tor_state`, §0 A11–A13; the plan's commit boundary, C3b+C3c one chunk;
the header's two bound provenances). No second design round. Next:
`docs/plan/fr29-phase-1.md` (written), the types commit, the wave, the four
angles on the DIFF.) **Revised 2026-09-16 (S281) for ADR-0547 — the host
NAMES its transport, the SDK carries no predefined transport kinds: the
descriptor's closed `kind` is retired for a host-chosen bounded name plus a
closed `exposure` value; `ZW_NET_DIALER_ABI_VERSION` is 2. §1.2 D6, §2, §3.1,
§3.4, §3.5, §4, §5, §6.1 and §8 (T9, T15, T16, T19, T27) carry the change;
the header stays the contract.** **Revised 2026-09-17 (S283) for ADR-0548 /
the FR-5 spec (`tor-plugin.md`): FR-5 is a REGISTRANT of this contract, not a
fourth runtime — §1.2 D8 now reads "the plugin yields by INIT ORDER"; the
`BuiltIn` sentences in §2, §2.1 and §0 A14 are superseded by that spec's §0
A10 and the built-in runtime is removed at its stage 3; the header is
unchanged (ABI 2).** **Revised 2026-09-17 (S285) for ADR-0549 / FR-30 — BUILT at the FR-5 spec's
stage 3 (C1): the descriptor carries a fourth closed integer `health`
(`ZW_HEALTH_STARTING` 0 / `READY` 1 / `FAILED` 2) and `ZW_NET_DIALER_ABI_VERSION`
IS 3. The struct is 52 bytes with `health` APPENDED at offset 48, so every v2
offset is unchanged and the version bump is the only thing a v2 host trips over
(`register` → `ZW_RC_ABI`). `health` never PERMITS a dial (`readiness == 100`
alone does), is fail-closed like not-ready and retired, and never triggers
`Preferred`'s fallback (ADR-0546 unchanged) — **narrowed S290 (ADR-0553 as
built; ADR-0552 correction 5): under `Preferred` a declared FAILED is refused
at the gate as `TransportFailed`, counts as the private path failing, and a
minute of it — from the declaration — is switch-eligible; `NOT_READY` alone
keeps the absolute no-clearnet guarantee; `Required` unchanged**;
`live_tor_state` reads `FAILED`
AHEAD of the readiness arm and renders `Unavailable`. The same chunk carried
FR-30 (a): `TorState::Bootstrapping` and `Unavailable` gained
`transport: Option<HostTransportName>`, so the failing arms name the host's own
transport or name none — the SDK names no transport. §2's struct, §3.1's table,
§3.4's derivation order, §3.5's header text and §8's T15/T26 rows carry it.
**T15, T26 and T27 were re-watched against their registered mutants at C1 and
their citations updated** (T15 `:1997`→`:2011`, T27 `:2528`→`:2684`, T26 with a
NEW mutant for the health case; `host_retire_moves_tor_state` `:481`→`:522`).
**OWED, and stated rather than glossed:** C1 displaced **55** registered
citations in the files it touched (measured by comparing each cited line's text
at `86c4cd9d~1` and `86c4cd9d`); the four above are re-watched, the rest are
re-cited-or-re-watched at their next touch. `check_cited_lines.py` grades only
NEW/CHANGED rows, so it cannot see this — that is its documented scope, not a
gate failure.**
**Implements:** ADR-0542 (the host's dialer first; built-in arti optional,
AFTER this) · ADR-0547 (THE HOST NAMES ITS TRANSPORT — no predefined kinds in
the SDK; `exposure` keeps the "not private" rendering honest; ~~the built-in
Tor, FR-5, is the one transport the SDK names itself~~ (S283: the SDK names no
transport — FR-5 is the `zec_wallet_tor` plugin, a registrant) — supersedes ADR-0544
D6's kind-with-`Other` shape and ADR-0545 D2's "reports the host's KIND") ·
ADR-0546 (`Preferred` falls back on unreachable/timeout ONLY —
supersedes the one ADR-0545 consequence sentence that said otherwise) · ADR-0543 (the crossing is a REGISTERED DIALER — no port opened
by the host; the dialer abstracts every transport the host runs) · ADR-0544
(the audit's corrections: SDK-minted swap token, `http://` refused off the
direct dialer, a REPLACEABLE token-gated registry with a stable trampoline at
trusted init, a frozen error-code table, `catch_unwind` at every crossing,
generation-tagged handles, buffer ownership, ~~built-in arti yields by
precedence~~ — S283: the plugin yields by init order, ADR-0548 D4) · ADR-0545 (THE HOST IS TRUSTED: its transport is used as the
host uses it; isolation REQUESTED where honoured, never demanded; `Required`
fails closed only on not-ready/retired; the descriptor is authoritative;
purpose visibility accepted; plane ownership the host's) · ADR-0526 (the
frozen `NetDialer` port — the crossing REPLACES one implementation of it and
adds no path) · Development Principles 1 (no `unsafe` in core — the `unsafe`
lives in the bridge's cabi module, like FR-15), 5 (metadata), 6 (honest
degradation), 7 (every byte from the network is hostile — here: every
INTEGER from the host is validated; the host is trusted, the boundary is not
assumed correct), 10 (no silent failures).
**Relates to:** `wallet-sdk.md` §2.3 (`TorPolicy`/`TorRuntime` — this spec
adds ONE runtime variant), §2.5 (`TorState` — the state gains the host
transport's name, isolation and exposure), §3 (`set_tor_policy` — CUT, see §0 A2),
§3.2a (the fail-closed matrix — one new column), §5.4 (NEVER-log);
`docs/handoff/host-feature-requests.md` FR-29 (the ask), FR-15/FR-17/FR-18
(the seed-port precedent this mirrors), H-5/H-12/H-13 in
`host-session-complete-integration.md` (the host's side, built against §3.5
of this spec); `docs/plan/fr29-session-brief.md` (the organisation:
CONTRACT FIRST, both sides in parallel).
**Does NOT own:** the Tor plugin (FR-5 — `tor-plugin.md`, a registrant of this contract); `set_tor_policy`
(cut); the loopback-SOCKS5 shape (`ExternalSocks5` stays refused, T0-6);
FR-6 (the in-workspace facade — demand-driven, P3); Relim's adapter (H-13),
its error mapping (H-12) and its device walk (H-6) — the host's, against
the header this spec freezes; any host-side network port.

---

## 0. Consistency audit (operating principle #4)

Read against `wallet-sdk.md`, ADR-0526, ADR-0541…0545, `ports.rs`,
`net/dialer.rs`, `tor_status.rs`, `state.rs`, the bridge's `api/config.rs`
and `convert.rs`, `tests/extraction_policy.rs`, `seed_port_cabi.rs`, the
request file and the host handoff. One source of truth per predicate; the
sentences below are the ones this spec changes or retires.

| # | Where | What it says today | What this spec does |
|---|---|---|---|
| A1 | `wallet-sdk.md` §2.3 `TorRuntime` doc; `config.rs:389-406` | three runtimes: `BuiltIn` (feature-gated), `ExternalSocks5` (refused), `Dialer(Arc<dyn NetDialer>)` (Rust hosts only); "circuit-isolation keys flow through EVERY runtime" | adds `TorRuntime::HostDialer(Arc<dyn HostDialer>)` (§2.1) — the registered, cross-library dialer. Keys still flow through every runtime; a non-isolating HOST transport IGNORES them and never refuses for them (ADR-0545 D2). `Dialer` stays for in-workspace hosts |
| A2 | `wallet-sdk.md` §3 line ~1297 `set_tor_policy` (specified, "not yet wired"); §3.2h G3 reserved obligation `set_tor_policy_resets_the_fell_back_latch` (line ~4012) | a mutable in-session policy is owed | **CUT by the founder (16 Sep).** The policy is FIXED at open; Relim configures `Required{HostDialer}` once and the host's selection (Tor, Shadowsocks, VLESS, direct) flows through the trampoline + descriptor. The spec sentence and the owed test are retired in `wallet-sdk.md` by the spec owner's edit (recorded here; the G3 latch note stays true for `Preferred`: a fallback latches until restart, pre-existing) |
| A3 | `wallet-sdk.md` §3.2a row "`{ ExternalSocks5 }` under either policy — REFUSED AT THE CONFIG DOOR"; its consequence "a Dart host has no Tor option in this build" | true today | the refusal STAYS (T0-6; ADR-0543: the loopback-SOCKS5 shape is OUT). The consequence sentence retires when this ships: `TorRuntimeConfig.hostDialer` is the Dart-expressible runtime. §3.2a gains one column (§6.2 here) |
| A4 | `ports.rs:32-36` `NetDialer::dial` doc: "Implementations that cannot honor isolation MUST return `DialError::Unsupported` rather than silently linking" | the ADR-0526 clause | RETIRED for HOST-PROVIDED dialers (ADR-0545); stays for the SDK's own dialers and built-in arti. Doc patch in `ports.rs` (chunk C3bc) pointing here |
| A5 | `tests/extraction_policy.rs:1163` "`TorRuntime → TorRuntimeConfig` is DELIBERATELY not listed: … Dialer not Dart-expressible" | the deliberate-subset exemption | still a subset (`BuiltIn`, `Dialer` stay non-Dart); `HostDialer` IS Dart-expressible as a unit variant. The comment names it (C3bc) |
| A6 | `api/config.rs` `TorRuntimeConfig` doc: "NOTHING HERE IS WIRED YET, AND THE CONFIG DOOR SAYS SO (T0-6)" | one variant, refused | becomes "`ExternalSocks5` is refused (T0-6); `hostDialer` selects the dialer the host's native library registered" (C3bc) |
| A7 | `sdk/zec_wallet/rust/src/lib.rs` header: "The ONE handwritten exception is `seed_port_cabi.rs`" | one `unsafe` module | TWO: `net_dialer_cabi.rs` joins it under the same posture (C3a patches the header) |
| A8 | `docs/arch/overview.md:200-209` | describes the pre-ADR-0531 in-workspace shape (audit LOW) | the arch doc is updated in the SAME change that lands the crossing (stage 4), naming the dylib seam and the two cabi modules |
| A9 | ADR-0544 D5 "`NotReady` … never triggers `Preferred`'s fallback"; ADR-0545 Consequences "not-ready or retired → … `Preferred` falls back visibly" | the two ADR sentences disagree on `Preferred` + not-ready | **ADR-0546 supersedes the ADR-0545 consequence sentence** (an accepted ADR is corrected by a new ADR, never by a spec row — the reviewer's MAJOR): `Preferred` falls back on `Unreachable`/`Timeout` only, as ADR-0544 D5, the ROADMAP W5 (1) named test and H-12 already say. Not-ready is a bootstrap in progress; falling back on it would leak clearnet at every app start (the audit's HIGH; since ADR-0552 a qualifying failure also needs a MINUTE of private-path silence before it switches, so the app-start leak this HIGH closed is now doubly shut) |
| A10 | `host-feature-requests.md` FR-29 `[ ]`; `host-action-board.md` FR-29 row; ROADMAP W5 (1) | open | the board row gains "contract frozen at `<sha>`" at stage 2; flips at stage 5 (the device walk); W5 (1) folds to one line then |
| A11 | `tor_status.rs:48` `live_tor_state(policy, fell_back, sync)` — pure over three inputs | the transport state derives from the policy, the latch and the sync status | gains a fourth input: `host: Option<HostTransportDescriptor>`, read at the two call sites (`Wallet::tor_state()` and the state stream) from `TorRuntime::HostDialer(h).descriptor()` — `None` for every other runtime AND for a `HostDialer` with nothing registered (§3.4). The function stays pure; the descriptor is a snapshot per derivation |
| A12 | `state.rs:832` `TorRuntimeKind` doc: "payload-free … DTO-safe for Dart" | the kind carries no data | RETIRED: `HostDialer { name, isolation, exposure }` carries the host's bounded, validated display name and two closed enums (still DTO-safe — the name is ≤ 32 bytes of validated UTF-8 with no control characters, display-only and never logged; no key material; the bridge mirrors the two enums and carries the name as a `String`, ADR-0547). The doc sentence is rewritten in the same change |
| A14 | ADR-0544 D6 (the descriptor's `kind` with an `Other` arm); ADR-0545 D2 ("reports the host's KIND as fact"); the header's `ZW_TRANSPORT_KIND_*`; `HostTransportKind` in core, its bridge mirror and the Dart `HostTransportKind` | the SDK carried five predefined transport names | **ADR-0547 (founder, 16 Sep ~11:00): RETIRED.** The host names its transport at registration and in every push (`HostTransportName`, ≤ 32 bytes, UTF-8, no control characters, non-empty); `exposure` (`Unknown` / `Hidden` / `Exposed`) says whether the path hides the device's address from the server, which no name can tell the wallet. ABI version 2 — a host built against the v1 header fails `register` with `-5`. ~~The one name the SDK owns is its own built-in Tor (FR-5, `TorRuntimeKind::BuiltIn`)~~ **S283: the SDK owns no transport name; FR-5 is a registrant (`tor-plugin.md` §0 A10–A11)** |
| A13 | `wallet-sdk.md:12252` (the §8 gate-map log paragraph) still calls `set_tor_policy_resets_the_fell_back_latch` "owed" | history | left as the dated record it is; A2's two live sentences carry the cut |

Nothing else in the corpus carries a second copy of these predicates. The
predicate "plaintext only under `TorPolicy::Off`" has ONE home
(`config::validate_transport`, stage 0) and one runtime restatement at
`LightwalletdClient::connect` — the stage 0 review's verdict on that pair is
folded before this spec is reviewed.

---

## 1. Design decisions

### 1.1 The problem

Relim ships the wallet as an isolated dylib (Relim ADR-0031). A Rust trait
object cannot cross two independently compiled libraries, so
`TorRuntime::Dialer` — the only host-transport path — is unreachable from
Relim and from every Dart-package consumer. The Dart surface's one transport
hook, `ExternalSocks5`, is refused honestly (T0-6). Result, shipped and
user-visible: the messenger rides Tor while the wallet syncs from the real IP
("networkCrossChannelAsymmetric", 16 locales).

### 1.2 The decisions (all founder-ruled; this spec designs the mechanism)

| # | Decision | Ruled by |
|---|---|---|
| D1 | The host REGISTERS a dialer with the SDK across the library boundary, by symbol name from the already-loaded image (the FR-15 pattern). The SDK opens no listener, connects to no loopback proxy | ADR-0543 |
| D2 | The registered dialer is a stable TRAMPOLINE the host installs at trusted init, BEFORE wallet configuration is validated; the host swaps its backing transport (Tor, Shadowsocks, VLESS, direct) behind it. The registry is first-wins, token-gated, REPLACEABLE (replace/clear through the token) | ADR-0544 D4 |
| D3 | The host is TRUSTED. Its transport is used as the host uses it. Isolation keys are passed on every dial (as today); the descriptor says whether the transport honours them; a non-isolating transport ignores the key and never refuses for it. `Required` = the host's transport, failing closed ONLY on not-ready/retired | ADR-0545 |
| D4 | The stream crosses as a COMPLETION-CALLBACK ABI (dial/read/write/close) because two tokio runtimes and two copies of std live in one process: the host completes on its threads; the SDK side only stores a result and wakes a waker, never entering its runtime from a host thread | audit (c); ADR-0544 D6 |
| D5 | Errors cross as a FROZEN numeric table (§3.3). `NotReady`/`Refused`/`Retired` are NOT reachability failures and never trigger `Preferred`'s clearnet fallback; only `Unreachable`/`Timeout` do (S290: plus the gate's own `TransportFailed` — a descriptor declared `health = FAILED`, not a code — after a minute of it; ADR-0553 as built, the paragraph under §3.3's table) | ADR-0544 D5; §0 A9 |
| D6 | The descriptor (the host's NAME for its transport — bounded, validated, display-only — plus readiness, isolation, exposure and, since C1, **health** — four closed integers) is AUTHORITATIVE for policy decisions and rendering. Policy branches on readiness, isolation and **health** (the gate's one predicate is `readiness >= 100 && !is_failed()`, ADR-0549); the name and the exposure are rendered verbatim, never interpreted. What no descriptor can do: change TLS, admit plaintext, override fail-closed on not-ready/retired — and `health` can only ever make the wallet MORE restrictive, never less (S290: towards the HOST's transport — FAILED never permits a dial of it; what a minute of FAILED does under `Preferred` is switch to the SDK's own clearnet, visibly, ADR-0553 as built) | ADR-0545 D3; ADR-0547; ADR-0549 |
| D7 | Every SDK export and every SDK-side completion callback runs behind `catch_unwind` and reports a code; a generation-tagged op/stream table makes a superseded backing fail typed; no SDK buffer is freed while a host operation is outstanding | ADR-0544 D6 |
| D8 | ~~Built-in arti (when it exists) yields by PRECEDENCE: a registered dialer wins; never both~~ **S283:** the SDK's own Tor is the `zec_wallet_tor` PLUGIN, a registrant like any host — it yields by INIT ORDER: it registers first and starts arti only on success; a taken slot stops it (`tor-plugin.md` D6) | ADR-0544 D7 → ADR-0548 D4 |
| D9 | The policy is fixed at open (`set_tor_policy` CUT). A host that wants a different wallet policy re-opens the wallet; a host that wants a different TRANSPORT swaps it behind the trampoline — no wallet action | founder 16 Sep |

### 1.3 Alternatives rejected

- **A SOCKS5 client behind `ExternalSocks5` + a host loopback inbound.** Out
  (ADR-0543): the host must open no port for the SDK; a loopback proxy is a
  port any in-process code can reach, and the isolation key would ride a
  SOCKS credential rather than a typed parameter.
- **A blocking (synchronous) stream ABI** (read blocks a host thread until
  bytes arrive). Simpler to write, and it would pin one SDK worker thread per
  open stream and block the sync runtime behind the host's I/O. Rejected: the
  SDK multiplexes a long-lived sync stream and short per-purpose circuits on
  a small runtime.
- **Passing `Box<dyn AsyncByteStream>` / a `Waker` across the boundary.** Two
  runtimes, two stds — not an ABI. Rejected (audit (c)).
- **Registration refused after the first dial** (ADR-0543 D3). Dropped
  (ADR-0544): it breaks Relim's carrier switching and is a denial primitive.
- **`set_tor_policy` for mid-session transport changes.** Cut: the trampoline
  makes it unnecessary for the host's selection; the wallet's own policy is a
  once-at-open choice.
- **A fuzz target on the boundary, a shares-instance flag, a shipped fake
  registrar for the host, a second design review.** Cut (founder, "just
  enough"): the host is trusted, every integer is validated, the bytes go to
  rustls; Relim has its own test doubles.

### 1.4 Tradeoffs stated

- A hostile IN-PROCESS registrant (not the host) that wins the slot first
  sees every destination, timing and byte volume the wallet dials — never
  content or keys (TLS is SDK-owned). The token-gated, first-wins registry at
  trusted init is the defence; the residual is the same as the seed port's.
- Buffers stay allocated until the host completes or retires an op; a host
  that never completes leaks them for the process life. Documented host
  obligation, not SDK-recoverable (the same class as the seed port's quiesce
  contract).
- A non-isolating host transport links the wallet's connections at the
  proxy; accepted and RENDERED (ADR-0545).

---

## 2. Domain types (Rust core; `zec-wallet-core/src/net/host_dialer.rs`, new)

> **ADR-0547 (founder, 2026-09-16 ~11:00, after the wave landed; built S281):
> the host NAMES its transport; the SDK carries no predefined transport
> kinds.** The wave's `HostTransportKind` (Direct / Tor / Shadowsocks / Vless /
> Other), its Dart mirror and the header's `ZW_TRANSPORT_KIND_*` are RETIRED.
> The descriptor is `{ name, readiness, isolation, exposure }`; the ABI
> version is 2; ~~the built-in Tor (FR-5) is the one transport the SDK names
> itself (`TorRuntimeKind::BuiltIn`)~~ **S283: the SDK names NO transport —
> FR-5 is the `zec_wallet_tor` plugin, a registrant named "Tor" through the
> descriptor like any host; `BuiltIn` is removed (`tor-plugin.md` D3, §0
> A10)**.

```rust
/// `ZW_TRANSPORT_NAME_MAX_BYTES` — the bound on the host's display name.
/// WHY 32: a transport chip holds one or two words ("Tor", "Shadowsocks",
/// "VLESS via Cloudflare"); 32 bytes of UTF-8 is eight CJK characters or a
/// short Latin phrase, and a fixed 32-byte field keeps the C struct
/// fixed-width (no pointer, no lifetime across the call).
pub const HOST_TRANSPORT_NAME_MAX_BYTES: usize = 32;

/// The host's display name for its transport — HOST-CHOSEN, bounded,
/// validated at the crossing, DISPLAY-ONLY: the SDK renders it verbatim and
/// never interprets it; policy never reads it; it is NEVER logged (§5).
/// Fixed-width and `Copy` (it mirrors the header's `uint8_t name[32]` +
/// `name_len`), so the descriptor and `TorRuntimeKind` stay `Copy`.
/// `new` accepts exactly four rules: (1) not empty and not blank; (2) `<= 32`
/// bytes; (3) valid UTF-8; (4) no control or FORMAT character — C0, C1, DEL,
/// NUL (`char::is_control`), the bidi controls U+202A–U+202E and
/// U+2066–U+2069, the zero-width characters U+200B–U+200F and U+FEFF, and
/// the line/paragraph separators U+2028/U+2029 (the four angles' fold: the
/// name is rendered right before the SDK's own privacy sentence and must
/// not be able to re-order, hide or line-break it). `UNATTRIBUTED` is
/// the EMPTY name the core alone can build (a host's empty name is refused
/// with `-6`): the rendering of a descriptor the core does not have (§3.4's
/// caller-bug arm); the UI renders it as its locale's "a private path".
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct HostTransportName { /* [u8; 32] + len, private */ }
impl HostTransportName {
    pub const UNATTRIBUTED: Self;
    pub fn new(raw: &[u8]) -> Option<Self>;   // the four rules above; `None` = `-6`
    pub fn as_str(&self) -> &str;
    pub fn is_unattributed(&self) -> bool;
}

/// Whether the host's transport honours per-key circuit isolation
/// (§3.5 `ZW_ISOLATION_*`). `Unknown` renders as "connections can be linked";
/// it never promises what the host did not declare.
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IsolationSupport { Unknown, Supported, Unsupported }

/// Whether the host's path HIDES the device's network address from the
/// server (§3.5 `ZW_EXPOSURE_*`) — the one privacy fact no name can tell the
/// wallet. `Hidden`: the server sees the transport's exit, not the device
/// (Tor, a proxy, a tunnel). `Exposed`: the server sees the device's address
/// (a plain connection behind the host's trampoline, a forward proxy that
/// passes the client address) — renders "not private". `Unknown`: the host
/// did not declare it — renders with caution, never the protected tone.
/// Policy never branches on it (D6); rendering does.
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransportExposure { Unknown, Hidden, Exposed }

/// Is the transport ALIVE (`ZW_HEALTH_*`; ABI v3, ADR-0549)? `readiness` says
/// how far a bootstrap has come; `health` says whether that bootstrap is still
/// one. `Failed` = the registrant JUDGED its transport failed, not merely slow.
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransportHealth { Starting, Ready, Failed }

/// The host's transport descriptor — AUTHORITATIVE (ADR-0545 D3), bounded
/// (the name ≤ 32 validated bytes; readiness 0..=100; the three enums above),
/// no unbounded text. `readiness < 100` is "not ready": `Required` fails
/// closed and `Preferred` WAITS (never falls back); the state renders
/// `Bootstrapping { percent }`. Built only through `from_raw`, so an
/// in-range descriptor is the only kind that exists on the core side.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct HostTransportDescriptor {
    pub name: HostTransportName,
    pub readiness: u8,           // 0..=100, validated at the crossing
    pub isolation: IsolationSupport,
    pub exposure: TransportExposure,
    pub health: TransportHealth, // v3; independent of readiness, no cross-field rule
}
impl HostTransportDescriptor {
    /// The name bytes (`name[..name_len]`) and the four integers that
    /// crossed; `None` for any violation (`-6`): a bad name, readiness > 100,
    /// an unknown isolation, exposure or health discriminant.
    pub fn from_raw(
        name: &[u8], readiness: u32, isolation: u32, exposure: u32, health: u32,
    ) -> Option<Self>;
    /// Has the registrant declared the transport failed (read AHEAD of
    /// readiness by §3.4)?
    pub fn is_failed(&self) -> bool;
    /// THE gate predicate, shared by `ReadinessGated` and `live_tor_state`:
    /// readiness at 100 AND not declared failed. `health` never PERMITS a dial
    /// — a `Failed` descriptor at readiness 100 is refused (S290:
    /// `TransportFailed`; it was `NOT_READY` from C1 until then).
    pub fn is_ready(&self) -> bool;
}

/// The FROZEN dial code table (§3.3). Numeric values are the ABI; they never
/// change meaning or number. `Ok` is not an error.
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HostDialCode { Ok = 0, NotReady = 1, Unreachable = 2, Timeout = 3, Refused = 4, Retired = 5 }

impl HostDialCode {
    /// The mapping into the port's error — THE ONE place a host code becomes
    /// a `DialError` (one source of truth; the fallback rule reads the
    /// `DialError`, never the code).
    pub fn into_dial_error(self) -> Option<DialError>;   // Ok → None
    /// A raw `u32` from the host: the five codes, or `None` for anything else
    /// (an unknown code is a host bug → `DialError::Io` "protocol violation").
    pub fn from_raw(code: u32) -> Option<Self>;
}

/// The registry accessor the core consumes; the bridge's cabi module
/// implements it over the process-global registry. Core stays
/// `#![forbid(unsafe_code)]`.
pub trait HostDialer: NetDialer {
    /// `None` when nothing is registered (or the registration was cleared):
    /// the door refuses (§6.1 E1) and `resolve_dialer` never falls back.
    fn descriptor(&self) -> Option<HostTransportDescriptor>;
}
```

`DialError` gains two variants (`error.rs`): `NotReady` ("the host's
transport is not ready — not a reachability failure") and `Retired` ("the
host's transport was retired or replaced — in-flight work fails typed").
`PolicyDialer` already surfaces every non-`Unreachable|Timeout` error as-is
(`net/dialer.rs` `Err(other) => Err(other)`), so neither can fall back (still
true of these two at S290; `DialError` since carries the gate's third variant,
`TransportFailed`, the one non-reachability refusal that may — after a minute
of it, ADR-0553 as built).

`TorRuntime` gains `HostDialer(Arc<dyn HostDialer>)` (config.rs; C3bc).
`TorRuntimeKind` gains `HostDialer { name: HostTransportName, isolation: IsolationSupport, exposure: TransportExposure }`
(state.rs; C3bc, revised for ADR-0547) so `TorState::Active { runtime }`
carries the host's name, isolation and exposure without changing `TorState`'s
shape. The bridge mirrors the two enums and carries the name as a `String`
(`api/state.rs`, `convert.rs`; `bridge_enums_cover_core_variants` is the
arbiter for `IsolationSupport` and `TransportExposure`).

### 2.1 Config: `TorRuntimeConfig::HostDialer` (Dart-expressible, unit)

`TorPolicy.required(runtime: TorRuntimeConfig.hostDialer())` — the product's
configuration (H-5). `convert.rs` maps it to
`TorRuntime::HostDialer(Arc::new(CAbiHostDialer))`, the bridge's ZST over the
registry (the `CAbiSeedPort` shape). Validation at the door (§6.1 E1): a
`HostDialer` runtime with nothing registered is refused with
`InvalidEndpoint { reason: "no host dialer registered — register before validating the config" }`
(RW-CFG-001, the T0-6 shape), never a first-dial failure and never a
clearnet fallback. ~~Beside a registered dialer, `BuiltIn` configured is
accepted and the registered dialer WINS (D8; `TorRuntimeKind` reports which
is speaking) — validation refuses only `BuiltIn` with the feature off (as
today).~~ **S283: `BuiltIn` is removed; a second Tor beside a registered
dialer is the `zec_wallet_tor` plugin refused at the slot, arti never started
(`tor-plugin.md` D6).**

---

## 3. Interface design (SOLID)

### 3.1 Inbound (what the host calls — the SDK's exports, `net_dialer_cabi.rs`)

Three verbs, all `extern "C"`, all behind `catch_unwind` (D7), mirroring
`seed_port_cabi.rs`'s posture: `unsafe` confined here; every raw pointer read
under a documented host contract; the registry lock never held across a
host call.

| verb | shape | returns |
|---|---|---|
| `zec_wallet_register_net_dialer(abi_version, dialer, descriptor, auth_out)` | first-wins into an empty slot; mints the 32-byte auth token into `auth_out` (non-null REQUIRED — there is no permanent variant: a dialer that cannot be replaced breaks carrier switching, §1.2 D2) | `0` ok · `-1` lock poisoned · `-2` occupied · `-3` null argument or a null required fn pointer · `-4` SDK panicked · `-5` ABI version mismatch (a v1, v2 OR v3 host against this **v4** SDK lands here — before any pointer is read, which is what makes the appended `health` field safe: a v2 caller never gets far enough for its 48-byte struct to be read at 52. **v3 joined that list at S291 and the reason is different in kind:** no struct grew — what changed is the PRIVACY MEANING of `ZW_HEALTH_FAILED`, which is `Preferred`-switch-eligible from v4, so a v3 host is refused rather than silently inheriting a rule it never agreed to) · `-6` descriptor invalid: a value outside its closed range, or a name that is empty, longer than `ZW_TRANSPORT_NAME_MAX_BYTES`, not UTF-8, or carries a control character (ADR-0547) |
| `zec_wallet_update_net_dialer(auth, dialer_or_null, descriptor_or_null)` | token-gated REPLACE (non-null dialer + descriptor) or CLEAR (both null). Either bumps the GENERATION: every in-flight op and every open stream of the superseded backing fails typed (`Retired`) on its next poll; the state re-derives | `0` · `-1` · `-2` unauthorized (empty slot or wrong token — ONE code, no oracle) · `-3` · `-4` · `-6` |
| `zec_wallet_net_dialer_notify(auth, descriptor, retire)` | the readiness/health/retire PUSH. Stores the descriptor (readiness moves `Bootstrapping`↔`Active`; `health = FAILED` moves it to `Unavailable` at ANY readiness — v3); `retire = 1` bumps the generation exactly as a replace does, WITHOUT changing the registration (the host tore its backing down — honest-off, a carrier switch in progress) | `0` · `-1` · `-2` · `-3` · `-4` · `-6` |

Non-zero is FATAL to that call, never retried (the seed port's rule, FR-15b).

### 3.2 Outbound (what the SDK calls — the host's vtable, §3.5)

`dial`, `read`, `write`, `close`. Each of `dial`/`read`/`write` takes an
SDK-minted `op_id` and an `sdk_ctx` + completion fn, returns `0` ("accepted;
exactly one completion WILL follow, on any host thread, possibly before this
call returns") or a dial code ("refused synchronously; NO completion
follows"). `close(stream)` is the SDK's cancel and release: after it returns,
the host completes every outstanding op on that stream exactly once (code
`Retired`) and never touches an SDK buffer for it again.

The SDK side (`HostStream: AsyncRead + AsyncWrite`, one per dialed stream):

- **Read.** `poll_read` with no outstanding read: allocate an owned buffer of
  `HOST_STREAM_READ_CHUNK_BYTES` (16 384 — one TLS record; larger buys
  nothing, smaller splits records), issue `read(handle, buf, cap, op_id, …)`,
  park the waker, return `Pending`. On completion (`n`): `0 ≤ n ≤ cap` is
  VALIDATED (else `Io("host protocol violation")` and the stream is closed);
  `n == 0` is EOF; the next poll copies `n` bytes into the caller's `ReadBuf`
  (never `advance` past what was written — the audit's read-length UB row).
- **Write.** One outstanding write at a time: `poll_write` copies up to
  `HOST_STREAM_WRITE_CHUNK_BYTES` (16 384) into an owned buffer, issues
  `write`, returns `Pending`; on completion returns `Ready(n)` for the SAME
  bytes the caller re-presents (the tokio contract). `n > len` is a protocol
  violation.
- **Cancel.** Dropping a future never frees its buffer: the op record owns
  it until the completion arrives. Dropping the `HostStream` calls `close`.
  A dial completion whose future is gone (the `DIAL_TIMEOUT_SECS` bound
  fired) `close`s the delivered stream at once — no orphan.
- **Generation.** Every op and stream carries the registry generation it was
  minted under; a completion for an older generation is dropped after
  freeing its buffer; a poll on a stale-generation stream returns
  `Retired` without calling the host (`a_stale_generation_handle_is_refused`).
- **Waking.** The completion callback (`extern "C"`, `catch_unwind`) locks
  the op table, stores the result, takes the waker, unlocks, then wakes.
  Tokio wakers are thread-safe; the SDK never runs its runtime on a host
  thread.
- **Registration order (the reviewer's MAJOR).** A completion may run on a
  host thread BEFORE the verb returns. So the SDK inserts the op record
  (op id, buffer, the current waker) into the table BEFORE calling the host
  verb; a completion always finds its record. After the verb returns the SDK
  re-checks the record under the lock: an already-delivered result is
  consumed at once (never `Pending` over a completed op), a synchronous
  refusal (non-zero return) removes the record and fails the op typed. Each
  record is a one-way state machine `Pending → Completed | Closed`: the
  first transition wins, the second is a no-op — so a `close` racing a
  genuine completion on another thread is exactly-once by construction
  (T25), and a host that both refuses synchronously and completes is a
  protocol violation caught by the removed record (the late completion is
  dropped after freeing nothing — the buffer went with the record).
- **The write re-present rule.** tokio's `AsyncWrite` convention is that a
  caller who saw `Pending` calls again with the same unwritten bytes. The
  adapter does not trust it blindly: on the poll that reports `Ready(n)` it
  compares the caller's first `n` bytes with the copy the host consumed and
  fails the stream `Io("write re-present violation")` on a mismatch — a
  wrong byte is never silently committed to the wire (T24).

Would this work with a different transport? Yes by construction: the host
vtable knows nothing of Tor; Relim's Tor and Shadowsocks dialers and a VLESS
one plug in behind ONE trampoline (H-13). Would it work with a different
host runtime? Yes: completions are plain function calls; the host may use
tokio, libdispatch, a thread pool.

### 3.3 The frozen error table (D5)

| code | name | `DialError` | `Preferred` falls back? | `Required` | state |
|---|---|---|---|---|---|
| 0 | `OK` | — | — | — | — |
| 1 | `NOT_READY` | `NotReady` | **no** — a bootstrap in progress; unqualified, however long (S290: ADR-0553 as built keys the never-starts switch on the descriptor's `health = FAILED`, not on this code — the paragraph below the table) | fail closed, `Stalled{TorUnavailable}` | `Bootstrapping { percent: readiness }` |
| 2 | `UNREACHABLE` | `Unreachable` | yes, visibly (`fellBack`) — **but NOT at once: ADR-0552: only after `TOR_PATIENCE_SECS` (60 s) in which the private path was BOTH silent (no RPC confirmed over it; a bare connect is not evidence) AND continuously failing, measured from the first observed failure of the run (phase 2 step 1, S287 — the earlier "silence measured from the last dial" was stage 1b's, retired). Returning this code does not flip `fellBack` by itself, so a host must not render "fell back" off its own return** | fail closed | `FellBack` / `Unavailable` |
| 3 | `TIMEOUT` | `Timeout` | yes, visibly — **ADR-0552: only after `TOR_PATIENCE_SECS` (60 s) of the private path silent AND failing; see the code-2 row** | fail closed | `FellBack` / `Unavailable` |
| 4 | `REFUSED` | `Unsupported` | **no** — the transport will not carry the request | fail closed | `Unavailable` |
| 5 | `RETIRED` | `Retired` | **no** — the backing was torn down; the host re-arms | fail closed | `Unavailable` until the next descriptor |
| other | — | `Io` ("host protocol violation") | no | fail closed | `Unavailable` |

The host maps its own taxonomy onto these five (H-12: Relim's `NotReady` →
1, never 2). The table lives in ONE Rust enum (`HostDialCode`) and ONE C
header; a test pins them equal (§8 T15).

**One refusal is the SDK's own and not a code (S290; ADR-0553 as built,
ADR-0552 correction 5).** The readiness gate in front of a registered dialer
reads the descriptor before every dial and refuses `health = FAILED` — at any
readiness, 100 included — as `DialError::TransportFailed`
(`net/readiness_gate.rs`); the host is never asked. `Required`: fail closed,
`Unavailable`. `Preferred`: it counts as the private path FAILING, so a full
`TOR_PATIENCE_SECS` of it, counted from the declaration, is switch-eligible —
visibly, `FellBack`; one refusal never is (T26). `DialError` is
`#[non_exhaustive]`; the five codes and their mapping are unchanged.

### 3.4 State derivation (`tor_status.rs`, C3bc)

`live_tor_state(policy, fell_back, sync, host)` — the fourth input is
`Option<HostTransportDescriptor>`, the snapshot the caller reads from
`TorRuntime::HostDialer(h).descriptor()` at derivation time (§0 A11; `None`
for every other runtime; the function stays pure). **Decision order, as
built (the wave's §5.2 departure 1, confirmed by the four angles): the
fell-back latch is checked FIRST, before any host-dialer-only case, so a
mid-session CLEAR can never hide a latched clearnet leak behind
`Unavailable`** — the `live_tor_state_honesty_invariants` proptest pins
"fell_back && !Off ⇒ FellBack" over the whole input space. Then, for
`HostDialer`: `host == None` (nothing registered, or cleared) → `Unavailable`;
`fell_back` → `FellBack` (as today); descriptor `health == Failed` →
`Unavailable`, **read AHEAD of the readiness arm** (ABI v3, ADR-0549 D3: the
registrant judged its transport failed, and rendering a bootstrap that can
never finish was FR-30 (b)); descriptor `readiness < 100` →
`Bootstrapping { percent: Some(readiness) }`; sync `Stalled{TorUnavailable}`
→ `Unavailable`; else `Active { runtime: HostDialer { name, isolation, exposure } }`
(ADR-0547; the name, isolation and exposure are the descriptor's, copied
verbatim). `runtime_kind`'s `HostDialer` arm is reached only with a `Some`
descriptor; a `None` there is a caller bug rendered honestly — `name:
UNATTRIBUTED, isolation: Unknown, exposure: Unknown` — never a panic.
**Both failing arms carry the transport's NAME (FR-30 (a), C1):
`Bootstrapping { percent, transport }` and `Unavailable { transport }`, where
`transport` is `Some(descriptor.name)` when one is registered and `None`
otherwise — so a host that registered "Shadowsocks" never reads the noun
"Tor", and where there is no name the UI renders its own transport-neutral
one. The stall sentence is derived from a `StallReason`, which carries no
name, so it is transport-neutral instead.** The UI renders the three honestly,
and the NAME is the host's: `exposure ==
Exposed` → "Not private (…)" (CAUTION tone since FR-30 (c) — a privacy loss is
never the calm colour — with the direct-connection
explanation) whatever the isolation says; `exposure == Unknown` → the
linkable form with the caution tone and the unverified explanation, whatever
the isolation says (the wave review's MEDIUM, now keyed on exposure); `exposure
== Hidden` → "via your app's private path (<name>)" with the protected tone
when isolation is `Supported`, and "via your app's private path (<name>);
connections can be linked by the proxy" with the caution tone when it is
`Unsupported`/`Unknown`. An empty name (only `UNATTRIBUTED` can produce one)
renders as the locale's "a private path". The wallet never names a transport
the host did not name and never claims an exposure the host did not declare.

### 3.5 THE CONTRACT — `sdk/zec_wallet/rust/include/zec_wallet_net_dialer.h`

The header IS the contract (frozen at this spec's review; a change is a new
ABI version — **v3 since ADR-0549, S285** (v2 was ADR-0547's, S281; corrected S286 — §0 and the header both read 3): the v1 header of the same morning
fails `register` with `-5`, and no host had built against it). It carries,
verbatim: the ABI version; the token width; the name bound
`ZW_TRANSPORT_NAME_MAX_BYTES` (32); the verb return codes; the five dial
codes; the descriptor struct — `uint8_t name[32]`, `uint32_t name_len`
(1..=32 bytes of UTF-8, no terminator counted, no control characters),
`readiness`, `isolation`, `exposure` (`ZW_EXPOSURE_UNKNOWN` 0 /
`ZW_EXPOSURE_HIDDEN` 1 / `ZW_EXPOSURE_EXPOSED` 2) — and its two closed value
sets; the vtable with the five entries and their threading, ownership and
completion rules; the registration lifecycle (trusted init BEFORE
validation; replace/clear/retire semantics; the quiesce obligation: the host
completes every accepted op before unloading the wallet library). §3.1–§3.3
above restate it in prose; the header wins on any disagreement.

**v1 clarifications (the wave review, 2026-09-16; no value or signature
changed, `ZW_NET_DIALER_ABI_VERSION` stays 1):** (a) READINESS — a host that
cannot carry a dial now returns `NOT_READY`/`RETIRED` synchronously, never
parks it; an accepted dial completes within the SDK's dial bound or counts as
a reachability timeout; the SDK itself does not call `dial` while the
descriptor's readiness is below 100 (E3, T26). (b) LIFETIME OF A SUPERSEDED
BACKING — its `ctx` and functions stay callable until every accepted op has
completed, every stream it minted has seen `close`, and no SDK thread can
still be inside a verb (the generation check precedes a verb call without a
lock; a call in flight is not fenced by it). (c) A verb that returns non-zero
hands its buffer back on that return. (d) CLEAR surrenders the slot to
first-wins; REPLACE is the carrier-switch verb.

**(e) WRITE COMPLETION MEANS SENT — a write completion says the bytes are on
the network, not in a buffer. The gap found at S286 (2026-09-18) by Relim's own
trampoline, OWED in the header.** *"WRITE COMPLETION MEANS SENT" is the rule's
NAME, chosen here so both repos cite one clause instead of two paraphrases: the
header paragraph carries that heading (beside its existing `READINESS` and
`WHEN TO PUSH` paragraphs), this is §3.5 (e), and a test on either side names
it — ours asserts a write does not complete while a fake host stream holds
bytes, and that the tail arrives on close.* The header mentions flushing NOWHERE (`grep -ic flush` = 0), the SDK
has no flush verb to call, and `HostStream::poll_flush`
(`net_dialer_cabi.rs:1177-1183`) documents "the host writes are already handed
over: nothing to flush" and returns `Ready(Ok(()))` — a premise that is FALSE
for any host whose stream buffers. arti buffers. Relim's trampoline never
flushes and its close path drops the entry without `shutdown()`, so arti's
`poll_close` is never reached and **the buffered tail is discarded** (ticketed
on Relim as #1169; no test on either side could catch it, because every stream
in either fixture delivers on `poll_write`). `poll_shutdown` (`:1185-1191`)
calls the host's `close` and returns `Ready(Ok(()))` without waiting, so the
SDK cannot recover the tail either. Why it matters beyond a stall: the wallet's
money path writes a transaction submit over this stream, and a discarded tail
makes the submit fail — while a host that flushes on its own later schedule
makes it land AFTER the wallet reported failure, which is the double-send
shape. **The fix is a contract sentence, not an ABI change** (no value or
signature moves, ABI stays 3): a host MUST NOT complete a write until the bytes
have been handed to its transport, and if its stream buffers it flushes before
completing. **CORRECTION (S287): the companion sentence originally read "`close`
may discard nothing", and that is too strong in the DANGEROUS direction.** It is
right for bytes of a COMPLETED write — but by the rule above there are none of
those left to discard, because completion already means sent. Where it bites is
the other case: a write the SDK CANCELLED, or one whose completion reported a
failure. Those bytes are not sent as far as the SDK is concerned, and a host
that flushed them at `close` would land a transaction submit AFTER the wallet
reported failure — the send-late double-send this whole clause exists to
prevent. So the rule is: **a host MUST NOT discard bytes from a completed write
(vacuous, and stated for symmetry) and MUST NOT deliver bytes from a cancelled
or failed one.** A host that therefore declines to `shutdown()` at close is
CORRECT, not cutting a corner. Found by the host's implementer while building
the fix, against my own over-strong sentence; confirmed at
`net_dialer_cabi.rs::poll_shutdown`, which calls `close` and returns
`Ready(Ok(()))` without waiting, and at `poll_flush`, a no-op that only
surfaces a fault — the SDK never relies on close to move a byte.
`poll_flush`'s doc is corrected to say
the SDK relies on that rule and cannot ask. **The ORDER of the two halves is
load-bearing, and it inverts the obvious reading: the flush at the WRITE is the
obligation, while a shutdown on `close` is only the belt for bytes a buggy
write path left behind. A remedy that adds the shutdown WITHOUT fixing the
write is worse than none — it converts a discarded tail into a send-late one,
and a submit that lands after the wallet reported failure is the double-send
shape.** Write the halves in that order wherever this rule is restated (the
header paragraph, a test name, a host's own ticket). The plugin's own C3b design already
flushes before completing a write (`docs/plan/fr5-phase-1.md`), which is where
the gap was first noticed. LANDED in the header at S286's close (`f17da870` — the paragraph carries the
rule's name at `zec_wallet_net_dialer.h`, and `poll_flush`'s doc cites it),
after the peer repo's device-build window closed. **Still OWED: the guard test** — a fake host stream that holds bytes
until flushed, asserting a write cannot complete while bytes are held and that
the tail has arrived after `close`.

**(e2) DIAL COMPLETION MEANS CONNECTED — a dial completion says the stream is
established end to end, not that a local leg accepted. The twin of (e), raised
S291 by the stage-S1 diff review (security review, the universal-SDK angle)
and LANDED in the header the same session** as a v4 clarification beside (e)'s
paragraph: no value and no signature changed, so it is not a bump by the
header's own rule — nothing crossing the boundary moved and no older copy
misreads what it is handed. The rule: completing with `ZW_DIAL_OK` asserts the
byte stream reaches the peer — a proxy has completed its CONNECT reply, a
circuit is built — and a host that cannot promise it MUST complete
`ZW_DIAL_UNREACHABLE` or `ZW_DIAL_TIMEOUT` instead. **Why it stopped being
implicit:** since S1 `truth` the wallet decides WHICH FAILURE IT REPORTS from
whether the dial was accepted (ADR-0554) — an accepted connection that carries
nothing is the far end not answering, a failed dial is the private path being
unavailable. A host completing OK optimistically (a SOCKS front end returning
the socket before the proxy's reply; a plugin building its circuit lazily
behind a duplex pipe) therefore makes a `Required` wallet blame the SERVER for
the host's own dead transport, for the patience minute and then for ever on the
sync-server sentence. The one registered host builds its circuit before
completing, so the field shows nothing — which is exactly why it belongs in the
contract and not in one host's habits. **OWED: the guard test** (a fake host
that completes OK on a stream whose first write never reaches a peer, asserting
the state the wallet then publishes), and the same sentence in the Rust port
doc, which `NetDialer::dial` now carries.

**(f) THE DIAL BUDGET IS A VALUE, not a sentence (S287; the C3a security pass's
A1, HIGH).** The budget a host must honour — how long the SDK waits for an
accepted `dial` before giving up on it — was PROSE in the header's READINESS
paragraph, while `DIAL_TIMEOUT_SECS` was the thing that actually fired and the
Tor plugin restated the figure as its own literal. Three copies, no comparison:
a change to the constant would have reached no host. It is now
`#define ZW_NET_DIALER_DIAL_BUDGET_SECS` in the header, pinned to the core constant by
`the_header_dial_budget_matches_the_core_constant`, which parses the shipped
header text rather than mirroring it in Rust. **The value also MOVED, 30 → 25**,
and the reason is a money one rather than a tuning one: it must stay strictly
below `GRPC_UNARY_TIMEOUT_SECS`, because `send_transaction` wraps the whole lazy
connect in the unary budget and at equality the RPC can cancel the dial future
at the instant the dial bound fires — so the dialer never observes an outcome,
and a `Preferred` wallet whose private path hangs evaluates the ADR-0552 switch
on the broadcast path exactly never. A host's own deadline belongs strictly
below this budget, so its typed error arrives first. **No ABI change** (a new
define, no value or signature moved; ABI stays 3).

**(g) WHICH BRANCH A CARRIER SWITCH TAKES (S287, raised by Relim).** The header
said "use REPLACE for a carrier switch" beside a `retire` doc that described a
carrier switch too, and left which one applies to the reader. It depends on the
host's SHAPE, and the §1.2 D2 shape is the common one: a host that registers ONE
stable trampoline and swaps the backing behind it never changes its vtable, so
it never calls REPLACE — its carrier switch is `notify` with `retire != 0`,
which bumps the generation exactly as a replace does while the registration
stands. REPLACE is for a host that genuinely hands over a different vtable or
`ctx`. Stated in the header at `zec_wallet_update_net_dialer`. Costly to leave
implicit in both directions: the SDK read Relim's fixed-vtable trampoline as
conflating "registered" and "replaced" and proposed a vocabulary change for an
event no shipped build of theirs can emit — a round trip this sentence removes.

---

## 4. Security

- **Trust boundary.** The host is trusted (ADR-0545) — its transport, its
  descriptor, its destinations. The BOUNDARY is not assumed correct: every
  integer that crosses is validated (readiness ≤ 100; the two enums'
  discriminants; `name_len` ≤ 32 BEFORE the name bytes are sliced, then the
  name's four rules — not blank, UTF-8, no control or format character
  (bidi controls, zero-width characters, line separators); `0 ≤ n ≤ cap`
  on every completion; a non-zero stream handle on a successful dial; the
  ABI version; null checks on every pointer and required fn pointer). The
  name is the ONE host-chosen text the SDK carries: bounded and validated at
  the crossing, copied by value, display-only, never logged, never a key,
  never read by policy. A violation fails the op typed and closes the stream;
  it never advances a buffer past what was written and never reads a byte the
  host did not write (the audit's UB row).
- **What a registrant cannot do.** Change TLS (the connector builds
  `GrpcTls::new()` / `RustlsSecurer::new()` from the scheme and the endpoint
  host alone — nullary, no registry input; §8 T13); admit plaintext (`http://`
  is refused off the direct dialer, stage 0); override fail-closed on
  not-ready/retired; suppress `fellBack` (the latch is written by
  `PolicyDialer`, which never sees the descriptor); learn a key (WebPKI
  verification, SNI = endpoint host, no client auth — unchanged).
- **The registry as a substitution primitive.** First-wins + token-gated +
  replaceable only through the token; the host registers at trusted init,
  BEFORE validation, so no in-process racer can win the slot after the host
  (H-13). One `-2` for occupied/unauthorized — no probe oracle. The token is
  in-process capability, zeroized on drop; not key material.
- **Panics.** Every export and every SDK-side completion callback runs
  behind `catch_unwind` (`guarded`, the stage 0 shape); a panic reports a
  code (`-4` from a verb; a completion that panics records `Io` on the op and
  wakes it). The seed-port verbs got the same treatment at stage 0.
- **No `unsafe` in core.** `net/host_dialer.rs` is safe Rust; the `unsafe`
  is confined to `net_dialer_cabi.rs` in the bridge (`lib.rs` header updated
  — §0 A7), with a SAFETY comment per call site and per pointer read.
- **Crypto.** None new. TLS stays rustls + `ring` + webpki, SDK-owned; the
  swap token (stage 0) is HMAC-SHA256 from `ring`, whole.

---

## 5. Privacy & metadata

- **What leaves the device.** Exactly what leaves today, through the host's
  transport instead of the real IP: TLS to lightwalletd (sync, broadcast,
  probe, ephemeral-detect) and TLS to the swap provider. The host's dialer
  sees, per dial: destination host and port, the isolation key (semantic
  labels — `wallet-sync`, `wallet-send-<rand>`, … — accepted, ADR-0545 D4;
  the swap key is the opaque token), timing and byte volume. Never content,
  never keys.
- **Isolation.** Passed on every dial as today. A host transport that honours
  it gives per-purpose circuits (Tor via arti); one that does not links the
  wallet's connections at the proxy exactly as the messenger's — rendered,
  not hidden.
- **Traffic patterns.** Unchanged: no new background traffic; the
  readiness/retire push is a host→SDK function call, not network I/O.
- **Logging.** The fall-back warn stays fields-free — and since S288 it is
  `wallet.private_path_fell_back`, not `wallet.tor_fell_back`: the host names
  its transport (ADR-0547) and it may not be Tor, and once the device log
  ships (`wallet-sdk.md` §5.4, host-switched, default OFF — FR-35) a MESSAGE
  that names a circumvention product is the S287 disclosure by another door.
  Since S288 these events reach a device log ONLY while the host has switched
  the SDK's device log on, on the tag `zec_wallet`, beside one
  `wallet.dial {dial_arm, dial_class, outcome}` line per dial (tag
  `zec_wallet_core`) — the observation that a dial rode this crossing. New
  §5.4-allowlisted events, fields-free: `wallet.host_dialer_registered`,
  `wallet.host_dialer_replaced`, `wallet.host_dialer_cleared`,
  `wallet.host_dialer_retired`, `wallet.host_dialer_descriptor` with
  `{transport_readiness, transport_health}` only — the two MECHANICAL values
  (`transport_health` joined at C1: without it a FAILED registrant logs
  `readiness=100` while the wallet renders `Unavailable`, which is FR-30 (b)
  relocated into the log stream).
  **`transport_isolation` and `transport_exposure` were removed at S287** — a
  joint change with the host, who drops the same pair from its push line.
  Keeping the host's NAME off the line did not close what the name rule was
  for: for the registrant that exists, `(exposure, isolation)` is a BIJECTION
  onto its three transports (Tor, Shadowsocks, Direct), so the log recovered
  which circumvention product a person runs from two integers, with no name
  anywhere — ADR-0547's letter satisfied, its purpose defeated. **Re-encoding
  does not help**, which is the durable part: any field carrying the privacy
  GRADE separates those three, because the grade is exactly what distinguishes
  them. So the split is by CONSENT, not by field — the grade lives in the
  host's debug bundle, which the founder ruled collects broadly WITH consent
  and where joinability is wanted (S392/S393), and the routine device log
  carries only what a lifecycle bug needs. Both fields still CROSS the ABI and
  must: the wallet renders "connections can be linked" from `isolation` and its
  honest privacy line from `exposure`. The guard is T16's FIELD-SET pin, not a
  check over the host's roster: a fourth backing SHARING a pair would narrow
  the disclosure while one taking a DISTINCT pair widens it, so an injectivity
  assert would alarm on the safe direction and sleep through the dangerous one.
  NEVER logged: the host's transport NAME (ADR-0547:
  host-chosen text stays off every log line — a name could carry a hostname
  or a user's label), the privacy grade in ANY encoding, destinations,
  isolation keys, the auth token, op ids
  with hosts, byte contents. The capture-layer guard test's allowlist names
  the four fields and pins `log_descriptor` at its source (§8 T16).

---

## 6. Error handling & degradation

### 6.1 Enumerated

| # | case | recoverable? | what the user sees |
|---|---|---|---|
| E1 | `HostDialer` configured, nothing registered (host bug: registered after validation) | host-side | the wallet does not open: `InvalidEndpoint` RW-CFG-001 with the reason; never a first-dial failure, never clearnet |
| E2 | ABI version mismatch at register | host-side rebuild | `-5`; the host treats as fatal init; the wallet, if configured `HostDialer`, hits E1 |
| E3 | Host not ready (bootstrap) — `NOT_READY`, or a descriptor with `readiness < 100` | yes, when ready | `Required`: sync `Stalled{TorUnavailable}`, state `Bootstrapping{percent}`; `Preferred`: the same — it WAITS (no clearnet). **Enforced on the SDK side, not only by the host's code** (the wave review's MEDIUM, all three angles): `runtime_dialer` wraps a registered dialer in a readiness gate that fails the dial `NotReady` BEFORE the host is called while the descriptor says not ready (`Retired` while nothing is registered) — a host that parked the dial instead of refusing would otherwise run into `DIAL_TIMEOUT_SECS`, a reachability `Timeout` that `Preferred` follows to clearnet at every app start — and since ADR-0552 a qualifying failure switches only after `TOR_PATIENCE_SECS` of private-path silence, so an app start costs no clearnet packet for at least that minute. The header's v1 clarification says the same from the host's side. Test T26. **A descriptor with `health = FAILED` is NOT this case (S290):** the gate refuses it `TransportFailed` at any readiness, the host never asked; `Required` fails closed as before, `Unavailable`; under `Preferred` it counts as the private path failing and a minute of it — from the declaration — switches, visibly (ADR-0553 as built; the paragraph under §3.3's table) |
| E4 | Unreachable / timeout through the host | yes | `Required`: stalled, `Unavailable`; `Preferred`: falls back visibly, `FellBack` (latched until restart, as today) — **ADR-0552: only after `TOR_PATIENCE_SECS` (60 s) of private-path silence; inside that window the wallet reports `Stalled{TorUnavailable}` and sends nothing in the clear, exactly like `Required`** |
| E5 | `REFUSED` (the transport will not carry it) | host policy | `Unsupported` surfaced; no fallback; `Unavailable` |
| E6 | Replace / clear / retire mid-sync | yes — the sync controller's existing retry re-dials on the next tick against the new backing | in-flight streams fail `Retired`; the sync status stalls once and recovers; state re-derives from the new descriptor (or `Unavailable` after a clear) |
| E7 | A host completion violates the contract (`n > cap`, unknown code, zero handle) | no (host bug) | the op fails `Io`, the stream closes, the sync retries; logged once per stream, fields-free |
| E8 | A completion arrives for a finished/cancelled op | — | the buffer is freed, nothing else; a delivered stream is closed at once |
| E9 | A panic inside a verb or a completion | — | a code (`-4` / `Io`); nothing half-committed; never unwinds into the host |
| E10 | Zero connectivity | as today | queued sends stay queued; sync stalls honestly |
| E11 | A hostile lightwalletd through the host's transport | as today | every byte is still validated at the gRPC boundary; the transport changes nothing there |
| E12 | A PERSISTED custom sync server that is `http://` loopback (stored under `Off`) meets a `HostDialer` policy at the next open (the stage 0 reviews' second MEDIUM: `validate_transport` covers the config's endpoint and offered list, not the aux row's stored choice; today no shipped configuration can reach it — T0-6 refuses every non-`Off` Dart policy — so it is folded into THIS feature, C3bc) | yes | the resolver treats the stored choice as one that cannot be honoured: the wallet opens on the DEFAULT with a visible `SyncServerFallback::ChoiceRefusedByTransport` (new variant, mirrored C3bc), the picker shows why; never a permanent stall, never plaintext |
| E13 | A descriptor whose NAME breaks a rule — empty (`name_len == 0`) or blank, `name_len > 32` (even with 32 valid bytes in the array), not UTF-8, a control character (a NUL counted in `name_len` included), or a bidi / zero-width / line-separator format character — at register, replace or notify (ADR-0547) | host-side | `-6`; nothing changes (register: the slot stays empty, `auth_out` untouched; replace/notify: the registration, the generation and the stored descriptor stay as they were); the host treats it as fatal init. `name_len` is bounded BEFORE the bytes are sliced (T27) |

### 6.2 The fail-closed matrix (`wallet-sdk.md` §3.2a gains this column)

| runtime | `Off` | `Preferred` | `Required` |
|---|---|---|---|
| `HostDialer`, nothing registered | n/a (Off never resolves a runtime) | REFUSED at the door (E1) | REFUSED at the door (E1) |
| `HostDialer`, not ready | — | waits; `Bootstrapping` | fail closed; `Bootstrapping` |
| `HostDialer`, declared `health = FAILED` (S290) | — | refused at the gate (`TransportFailed`), counts as failing; a minute of it → clearnet, `FellBack` | fail closed; `Unavailable` |
| `HostDialer`, unreachable/timeout | — | clearnet, `FellBack`, **ADR-0552: only after `TOR_PATIENCE_SECS` (60 s) of the private path silent AND failing (phase 2 step 1, S287); since S1 `window` (S290) an ACCEPTED dial on a class that has kept failing past that minute is left the same way** | fail closed; `Unavailable` |
| `HostDialer`, refused/retired | — | no fallback; `Unavailable` | fail closed; `Unavailable` |
| `HostDialer`, ready, isolation `Unsupported`/`Unknown` | — | dials (key passed, ignored by the host); state says "linkable" | the same |

---

## 7. Performance

- One extra copy per read and per write (the owned chunk buffers) — bounded
  at 16 KiB per direction per stream; the sync stream is one stream, the
  per-purpose circuits are short-lived. No busy polling: every wait is a
  parked waker.
- Beyond the existing `DIAL_TIMEOUT_SECS`, the unary/stream RPC bounds and the
  swap exchange bound there is ONE further timer, added by ADR-0552:
  `TOR_PATIENCE_SECS` (60 s), the window a `Preferred` wallet insists on the
  private path before it will switch (S290: and one further BOUND, not a timer
  a host sees — `FALLBACK_ESTABLISH_BUDGET_SECS` = 5 s on the post-switch
  clearnet leg, TCP and TLS together from the switch instant, every attempt;
  `DIAL_TIMEOUT_SECS + 5 ≤ GRPC_UNARY_TIMEOUT_SECS` is asserted at compile
  time). It is the WALLET's clock and the wallet's
  promise — **a host must not build a competing one** (board row FR-34); the
  host's transport latency (Tor
  circuits) is inside those bounds and is graded on the device (stage 5,
  H-6) — timeouts and HTTP/2 keepalive are TUNED there, not guessed here.
- The registry: one `RwLock` read per dial and per state derivation; the op
  table one `Mutex` lock per poll/completion. No DB.

---

## 8. Testing strategy — the named-test contract (gate map)

Unit tests run against an IN-PROCESS FAKE HOST (`tests/host_dialer_fake.rs`,
bridge crate): a C-ABI vtable over plain TCP driven by a SEPARATE tokio
runtime, so the two-runtime crossing is proven without Relim. No real
network; the fake serves loopback listeners. E2E is stage 5 (H-6).

| # | test | gate | chunk |
|---|---|---|---|
| T1 | `required_fails_closed_on_not_ready_and_retired` — `NOT_READY` then `RETIRED` from the fake: zero clearnet, `Stalled{TorUnavailable}`, state `Bootstrapping`/`Unavailable` | 6 | C3bc |
| T2 | `preferred_fell_back_on_unreachable_or_timeout_only` — `UNREACHABLE`/`TIMEOUT` flip the latch; `NOT_READY`/`REFUSED`/`RETIRED` do not | 6 | C3bc |
| T3 | `an_empty_registry_under_required_never_falls_back` — and under `Preferred`: the door refuses (E1); with a mid-session clear, dials fail `Retired`, never direct | 1, 6 | C3bc |
| T4 | `a_second_registration_is_refused` — `-2`, the first stands, `auth_out` untouched; the sibling refusals beside it, including a wrong ABI version answered `-5` **with every pointer null** (FR-5 C2, S286 — valid pointers could not tell "refused before any read" from "refused after one"; a null check or a read ahead of the version check answers `-3` or faults) | 1 | C3a; revised S286 |
| T5 | BUILT AS THREE named tests (the wave; the reviewer's lockstep note): `host_retire_fails_inflight_streams_typed` (C3a — a replace and a `notify(retire=1)` each fail an open stream's next poll with `Retired` and bump the generation) · `host_retire_moves_tor_state` (C3bc, `tor_status`) · `host_retire_moves_tor_state_through_the_wallet_handle` (C3bc, `wallet`) — the state re-derives on a descriptor flip and a `Retired` dial | 6 | C3a + C3bc |
| T6 | `a_stale_generation_handle_is_refused` — a stream minted under generation N polls after a replace: `Retired` without a host call | 6 | C3a |
| T7 | `cabi_unsupported_is_not_a_reachability_failure_and_never_falls_back` — `REFUSED` → `Unsupported`, `Preferred` does not fall back | 6 | C3bc |
| T8 | `the_isolation_key_survives_the_crossing` — the fake records the key bytes verbatim for `wallet-sync` and a `wallet-send-*` dial | 1 | C3a |
| T9 | `a_non_isolating_descriptor_dials_without_isolation_and_the_state_reports_it` — isolation `Unsupported`: the dial proceeds (key passed), `Active{HostDialer{name: <the host's>, isolation: Unsupported, exposure: Hidden}}` — the state carries the host's name and exposure verbatim (ADR-0547) | 2, 6 | C3bc; revised S281 |
| T10 | `http_endpoint_over_a_host_dialer_is_refused` (+ `_at_the_door`, `_at_connect`) | 1 | stage 0 ✓ |
| T11 | `no_swap_id_byte_reaches_the_dialer_isolation_key` | 1 | stage 0 ✓ |
| T12 | `a_planted_panic_at_every_export_returns_a_code` (the three dialer verbs) + `a_planted_panic_in_a_completion_records_io_and_wakes` (the two completion callbacks); the seed-port sibling shipped at stage 0 | 1 | C3a |
| T13 | `a_registered_dialer_cannot_influence_tls_config` — the connector's TLS config under `HostDialer` equals the one under `Off` (same roots, ALPN, SNI = endpoint host); the TLS builders are nullary | 1, 4 | C3bc |
| T14 | `no_sdk_network_io_outside_the_dialer` — a source scan over `zec-wallet-core/src`, `zec-wallet-swap-near/src`, `zec_wallet/rust/src`: `TcpStream`, `TcpSocket`, `UdpSocket`, `lookup_host`, `connect(` on a socket type appear ONLY in `net/dialer.rs` | 4 | C3bc |
| T15 | `the_header_and_the_rust_table_agree` — the ABI version (**3**), token width, the name bound (`ZW_TRANSPORT_NAME_MAX_BYTES` = `HOST_TRANSPORT_NAME_MAX_BYTES`), verb codes, dial codes and the **three** descriptor value sets (`ZW_ISOLATION_*`, `ZW_EXPOSURE_*`, `ZW_HEALTH_*`) parsed from the header equal the Rust constants, each set closed on the header side; the header defines NO `ZW_TRANSPORT_KIND_*` (ADR-0547); the C struct's size (**52**) and field offsets (`health` at 48) equal the Rust `#[repr(C)]` mirror's; and the INDEPENDENT C fake's `Descriptor` has the same size, alignment and every field offset (FR-5 C2, S286 — at a future v4 an un-updated fake would otherwise be a `ptr::read` of a larger struct from a smaller object) | 4, 7 | C3a; revised S281, S285 (re-watched at C1), S286 (re-watched at C2) |
| T15a | `a_declared_health_crosses_the_c_boundary_and_its_set_is_closed` — ADR-0549 D5 at the C boundary: a registrant that declares `ZW_HEALTH_FAILED` is read back `Failed` at the readiness it MEASURED and is not ready at 40 or at 100 (health forbids a dial, never permits one); `STARTING` at 100 is still ready (the polarity, so the row cannot pass by reading everything as failed); `3` and `u32::MAX` are `ZW_RC_DESCRIPTOR` at `register` and at `notify` with `retire` both ways, storing nothing. **What the closed set guards is a MIS-BUILT v3 host, not a v2 one** — a v2 caller is refused by the version check at `register` before any pointer is read, and can never reach `notify`/`update` because those need a token only a successful v3 `register` mints | 4, 7 | C1 (S285) |
| T16 | `host_dialer_events_are_fields_free` — the §5 events pass the capture-layer allowlist test: `transport_readiness` / **`transport_health`** pass, a `transport_name` (or any name-carrying field) trips, **`transport_exposure` and `transport_isolation` now TRIP** (S287's consent split — the privacy grade is off the routine log in every encoding), and `log_descriptor` is pinned at its source to emit exactly **those two** (two of the fixture's rows push `FAILED` at 100 and at 40 — the v3 case the axis exists for). The field-set pin is the guard that keeps the grade off, so it carries two registry rows: one on the ALLOWLIST, one on the PIN | 5 | C3bc; revised S281, S285, **S287** |
| T17 | `an_out_of_range_completion_fails_the_op_typed` — `n > cap`, an unknown code, a zero handle → `Io`, stream closed, buffer freed, no advance past cap | 1, 6 | C3a |
| T18 | `a_cancelled_op_keeps_its_buffer_until_the_host_completes` — drop the read future, complete late from the fake: no UAF (the buffer is owned by the record), the late completion is discarded | 1 | C3a |
| T19 | Dart: `sync_status_presentation_test.dart` — the host's NAME renders verbatim in the chip; `exposed` renders "not private" whatever the isolation; `unknown` exposure renders linkable with the caution tone and the unverified explanation whatever the isolation; `hidden` + `supported` alone earns the protected tone; `hidden` + `unsupported`/`unknown` isolation renders linkable; an empty name renders as "a private path" (a watched mutant, line count kept). **Since C1 the same file also pins FR-30 (a) and (c)** — `transportPresentation (FR-30 (a))`: each failing arm renders the NAMED variant for a registered transport and the NEUTRAL one for `null` AND `''`, no failing arm a non-Tor host can reach says "Tor" on label or detail (the stall sentence included), and a host that DID register Tor still reads its own noun; and the FR-29 row now pins the TONE ORDERING — the exposed arm never reads calmer than the hidden-linkable one, and only hidden + isolated is protected | 2, 8 | C3bc; revised S281, S285 |
| T20 | `bridge_enums_cover_core_variants` (existing arbiter) green after the mirror; `just wallet-bridge-verify` green post-regen | 4 | C3bc |
| T21 | BUILT AS TWO named tests: `a_persisted_plaintext_choice_under_a_host_dialer_falls_back_visibly` (the pure resolver, `sync_server`) · `…_falls_back_visibly_at_open` (the shipped open path, `wallet`) — E12: a stored `http://127.0.0.1` custom choice + a `HostDialer` policy opens on the default with `ChoiceRefusedByTransport`; the same choice under `Off` is honoured; the aux row is never rewritten | 2, 6 | C3bc |
| T22 | `a_plaintext_custom_server_under_a_host_dialer_is_refused_by_the_probe_typed` — the picker's probe keeps the plaintext reason (`InvalidEndpoint`), never "unreachable" | 2, 6 | stage 0 ✓ |
| T23 | `a_completion_that_fires_before_the_verb_returns_is_not_lost` — the fake host completes INSIDE `dial`/`read`/`write` (same thread, before returning): the op resolves on the next poll, never hangs (§3.2 registration order) | 6 | C3a |
| T24 | `a_write_re_presented_with_different_bytes_fails_the_stream` — after `Pending`, the caller re-polls with a changed prefix: `Io`, nothing more reaches the wire; the honest re-poll returns `Ready(n)` | 1, 6 | C3a |
| T25 | `close_during_an_executing_completion_is_exactly_once` — the fake host completes a read from a second thread while the SDK `close`s the stream: one transition wins, the buffer is freed once, no double wake, no UAF | 1, 6 | C3a |
| T26 | `a_not_ready_descriptor_fails_the_dial_before_the_host_is_called` — E3's SDK-side gate (the wave review's MEDIUM): `readiness < 100` → `NotReady` with zero host dials and the latch untouched; `None` → `Retired`; ready → the host dials once; **and since C1, `health = FAILED` at readiness 100 → `TransportFailed` (S290; `NotReady` from C1 until then) with the host never asked and the latch still false on that dial** — the privacy invariant AT THE SEAM, not at the predicate (a "simplification" that inlined `readiness < 100` into the gate would pass every other row here and send a declared-failed transport to clearnet under `Preferred` on its FIRST refusal; since S290 a MINUTE of such refusals is switch-eligible by design — ADR-0553 as built — and the row pins that one is not) | 1, 6 | the stage 4 fold (`net/dialer.rs`); extended S285 |
| T26a | `a_failed_health_renders_unavailable_and_starting_renders_bootstrapping` — ADR-0549 D5's rendering polarity: `health = FAILED` at readiness 40 renders `Unavailable` (read AHEAD of the readiness arm), `STARTING` at the same readiness renders `Bootstrapping { 0.40 }`; FAILED at 100 is still `Unavailable`; a declared failure is never `FellBack` and a latched leak still outranks it; READY again re-reads `Active`, so the arm is not a one-way latch. Both failing arms carry the registrant's own name | 1, 6 | C1 (S285) |
| T27 | `a_bad_transport_name_is_refused_at_every_verb` (bridge, the cabi module) — E13: an empty name, a blank name, 32 valid bytes with `name_len` 33 (bounding, not truncation), `name_len` far past the array, a NUL inside the counted bytes, an invalid UTF-8 byte, a C0 and a C1 control character, a bidi override and a zero-width space each return `-6` from `register` (slot empty, `auth_out` untouched), from `update` (the registration, the generation and the stored descriptor unchanged) and from `notify` (the same); a 32-byte name of four-byte characters is accepted, and the core's `HostTransportName::new` pins the four rules directly (`the_frozen_dial_code_table_and_descriptor_ranges_are_the_abi`, revised) | 1, 4 | ADR-0547, S281 |

Every row gets a mutant row in `evals/mutants.tsv` citing the line the
watch printed (one mutant per run, base restored, the module re-run).

---

## 9. Multi-platform

- **Android / iOS / macOS:** the host resolves the three exports by name
  from the loaded image (`RTLD_NOLOAD`; `_dyld_get_image_name` on Apple —
  the seed-port precedent, device-confirmed). iOS background: a suspended
  app's host transport stops; the SDK sees `NOT_READY`/`RETIRED` through the
  push or a timeout, stalls honestly, and the sync controller resumes on
  foreground as today. No SDK timer runs in the background.
- **Windows:** `GetModuleHandle` — the registrar arm is the host's and is
  NOT device-confirmed (H-8). The ABI is platform-neutral C.
- **Desktop Linux:** as Android (`dlopen(RTLD_NOLOAD)`).

## 10. Multi-device & sync

No wallet state crosses devices for this feature; the registration, the op
table and the descriptor are per-process. The transport choice is the
HOST's per-device setting (plane ownership, ADR-0545 D5). Nothing here
assumes one user = one device.

---

## 11. Self-review checklist

- [x] No `unsafe` in the crypto path; none in core; audited TLS whole
- [x] No key material crosses: the token is an in-process capability
- [x] No PII / destinations / keys in logs (§5)
- [x] Works offline: queued ≠ error; stalls are honest states
- [x] Honest degradation: every matrix cell has a rendered state
- [x] Accessible: the wording rides the existing sync status sheet (screen-reader label carries the detail line, S278)
- [x] Test vectors: none new (no new crypto)
