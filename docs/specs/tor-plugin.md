# Spec — Tor for a standalone host: the optional plugin `zec_wallet_tor` over the shared crate `dialer-tor` (FR-5)

**Status:** Reviewed — 2026-09-17 (S283). Written from ADR-0548 and
`docs/plan/fr5-session-brief.md` §0/§2, the founder's five answers of 16 Sep
23:50, Relim `main`'s `crates/transport-tor` and `crates/relim-ffi/src/wallet_net_dialer/`
read at `0c252470`, the FR-29 contract as it then stood at ABI v2 (it is v3
since C1, 2026-09-17 — the descriptor's `health` axis, §0 A17/A21), and the sourced platform
survey `docs/research/2026-09-17-embedded-tor-mobile-constraints.md` (§9).
**The design review, the same night (REVIEW.md §2 + docs consistency):**
security review ∥ arch review ∥ crypto audit ∥ docs consistency on
the draft — security 5 HIGH + 5 MEDIUM (the sleep-stopped pause clock, the
accept window, the freed `ctx`, the close/read write-after-free, arti's guard
ids at `warn`; the wipe sweep, `#[non_exhaustive]`, the config-build refusal,
the Apple predicate, backup exclusion); crypto 1 HIGH + 5 MEDIUM (the
destination-controlled `Timeout` fallback, the supply-chain trust shift, the
NULL key, the threshold on the boundary, the rustls invariant,
`compression`'s C decompressors); arch 3 MAJOR + 2 MEDIUM (the sanctioned
dev-dependency list, a live ROADMAP sentence, the crossing spec's `BuiltIn`
sentences, the ADR README rule, the lifecycle-observer exception); docs 7
MAJOR (test placement, a superseded ADR citation, a phantom
back-annotation, H-14, a malformed board row, `wallet-sdk.md` §3.2a, one
quotation — the last refuted at the file) — EVERY one folded (§0 A22–A30,
§3.3, §3.4, §3.6, §4, §5, E20–E23, P26–P28, D-10–D-11). Then code reviewer
on the folded draft — NOT YET: the interim-fallback contradiction across E9 /
E13 / §9.5, the unnamed lock behind the generation check, the "terminal"
phase, two citation spans, and the S266 cuts (`networkChanged()`,
`since_secs`, `bridge_lines`) — folded; then a scoped second reading of
those folds — not yet: the init row's runtime, the `setBridges`
cross-reference, a merged table row — folded verbatim from the reviewer's
own remedies and grep-verified, not re-read by a reviewer. Each pass found less
than the one before; the one unchecked item is §11's public-readability
line. **Stage 2 DONE (S284, 2026-09-17): `docs/plan/fr5-phase-1.md`, whose
Apple census (`docs/plan/probes/apple-duplicate-symbol-census.sh`) corrected
four premises of this spec — dated in place below: the SDK's own example is
the FRAMEWORK case (D8, §3.1, §3.2), P15's needles are crate-anchored (A16,
P15), the `-u` list also names the wallet's verbs and the dynamic-framework
fallback is retired (§1.4). The plan's §5 is the table.** **C0 (the same
day, ADR-0550, at the resolver): arti and the ZEC stack cannot share a cargo
lock (`crypto-common ^0.2.2` vs `=0.2.0-rc.1`), so `dialer-tor` and the
plugin's crate are their OWN workspaces under `sdk/` and the bridge cannot
DEV-depend on the plugin — §8's placement of P2/P3/P19/P28 is re-cut at C3's
start (plan §5 D-7/D-8); §2's `rlib` clause is moot.** Next: C1.
**Implements:** ADR-0548 (Tor for a host without a transport is an OPTIONAL
second plugin registering through the dialer contract, over the shared crate
`dialer-tor`; `zec_wallet` stays Tor-free; the second-Tor-client rule moves to
the plugin's init order; the mobile behaviour is a researched section) ·
ADR-0543/0544/0545/0546/0547 (the contract the plugin is a registrant OF —
unchanged by this spec: ABI stays 2, the header is not touched — *superseded
S286: ADR-0549 moved the contract to ABI **3** at FR-5 C1 (the `health` axis),
and the plugin registers at 3*) · ADR-0542
Decisions 1 and 3 (the host's dialer first; rank) and its surviving clause of
Decision 2 (arti consumed as the shared module, never forked in) · ADR-0526
(the frozen `NetDialer` port) · Development Principles 1 (no `unsafe` in core;
the plugin confines its own to one module), 2 (arti and rustls WHOLE — our code
is glue), 5 (metadata: what the local network sees is Tor use; bridges are the
answer, and bridge lines are never logged), 6 (honest degradation: the wallet
renders the plugin's readiness through the existing chip; `required` never
falls back), 7 (every byte from the network is hostile — arti parses the
directory; the plugin validates every integer that crosses the two seams it
sits between), 9 (scope cut: no onion services, no pluggable transports, no
UI, no persistence beyond arti's own), 10 (no silent failures).
**Relates to:** `docs/specs/host-transport-crossing.md` (the contract; §3.5 the
header; §8 the fake host this spec runs in reverse) · `wallet-sdk.md` §1.2
(the extraction dep policy — crates.io / pub.dev only), §1.8 / §2.3 / §3.2 (the
librustzcash-`tor` design this spec RETIRES, §0 A1–A3), §2.5 (`TorState`), §5.4
(NEVER-log) · `docs/handoff/host-feature-requests.md` FR-5 · `host-action-board.md`
FR-5 / FR-5a · `host-session-complete-integration.md` H-15 (the crate move,
Relim's) · `docs/handoff/release-checklist.md` §B (the numbers this spec owes).
**Does NOT own:** the crate move to `alvorado-lab/dialer-tor` and its publish
(stage 0 — the founder creates the repository, the Relim session moves the
crate, H-15); onion services (`.onion` targets are refused — the LGPL carve-out
`dialer-tor` carries, unchanged); pluggable transports (`pt-client` is off;
an obfs4 line is refused with its class); any wallet UI for the plugin's
status (the existing chip renders readiness; the blockage detail is the
HOST's to render from the plugin's typed status — a `zec_wallet_ui` widget
is a later item if a consumer asks); Relim's path (Relim registers its own
dialer; the plugin is never on it); a Rust-workspace host's Tor (it consumes
`dialer-tor` directly through `TorRuntime::Dialer`, FR-6 when demanded).

---

## 0. Consistency audit (operating principle #4)

Read against `wallet-sdk.md`, ADR-0526/0542…0548, `host-transport-crossing.md`,
the header, `config.rs`, `net/dialer.rs`, `readiness_gate.rs`, `tor_status.rs`,
`state.rs`, `store.rs`, the bridge's `api/config.rs` / `api/state.rs` /
`convert.rs` / `selftest.rs`, `extraction_policy.rs`, the three
`Cargo.toml`s, the UI's `sync_status_presentation.dart` and its tests, the
UI's `app_lifecycle_provider.dart`, the request file, the board, the ROADMAP.
One source of truth per predicate; every sentence below is one this spec
changes, retires or makes true, and the change lands in the chunk named.
**The chunks** (stage 3, `docs/plan/fr5-phase-1.md` writes them out; these
C-numbers are this spec's and not the research file's C1–C4): **C1** = the
`BuiltIn` removal and its doc sweep across core, bridge, Dart, UI and the
specs; **C2** = gates and manifests (the extraction policy, the lock gate,
the symbol guard, `deny.toml`, the justfile legs, the arch doc); **C3** = the
plugin package itself with its tests and the example's toggle. Rows marked
"this commit" landed with the spec.

| # | Where | What it says today | What this spec does |
|---|---|---|---|
| A1 | `wallet-sdk.md` §1.8 row "Tor (built-in runtime ONLY)" (line ~287, already carrying an ADR-0548 supersession marker) | librustzcash's `tor` feature, `connect_to_lightwalletd`, `isolated_client`, `set_dormant`, behind `tor-builtin` | REWRITTEN to one row: "Tor for a host without a transport — the optional plugin `zec_wallet_tor` (this spec) over `dialer-tor` (arti whole), registering through the FR-29 contract; the SDK compiles no arti" (stage 3, C1) |
| A2 | `wallet-sdk.md` §2.3 `TorRuntime::BuiltIn` doc (line ~712, marker present), its §2.5 DTO listing `BuiltIn` (`:1078`), `:1334` and `:3369` ("precedence is moot until tor-builtin"), `:12512`; the sentence "circuit-isolation keys flow through EVERY runtime (BuiltIn → arti circuit groups; …)" at `:745` AND its code copy `config.rs:480` | a fourth runtime the SDK owns | `BuiltIn` is REMOVED (§1.2 D3 below: no consumer exists — a Dart host uses the plugin, a Rust host uses `dialer-tor` through `Dialer`). The list becomes three runtimes; the parenthesis loses its first term in BOTH copies (C1) |
| A3 | `wallet-sdk.md` §3.2 (line ~1650) "Tor via `tor::Client::connect_to_lightwalletd` (features `tor` + …)"; §2.3's comment block "`tor::Client::connect_to_lightwalletd` builds a full tonic HTTP/2 channel over an arti DataStream …" | the June design | RETIRED: the plugin's stream rides the SDK's OWN connector over the registered dialer exactly as Relim's does — one TLS path, the readiness gate, the isolation tokens, the keepalive and the stall detector all apply (C1) |
| A4 | `zec-wallet-core/src/lib.rs:17-23` "the built-in arti runtime (feature `tor-builtin`) is one impl of the seam … Feature gates: … `tor-builtin` (default OFF …)" | the feature exists | the feature is DELETED (`Cargo.toml:213 tor-builtin = []`, the `:95` comment, `sdk/Cargo.toml:100`, the bridge's `Cargo.toml:64` comment); the doc names the plugin as the standalone path (C1) |
| A5 | `config.rs:492-497` `TorRuntime::BuiltIn` under `#[cfg(feature = "tor-builtin")]`; `net/dialer.rs:111-114` its `UnsupportedRuntime` arm; `tor_status.rs:137,367` its `runtime_kind` arms and the `:259,:341` test comments | a cfg'd-out variant with dead arms | REMOVED with the feature; `runtime_dialer` stays exhaustive over three variants (C1) |
| A6 | `state.rs:845-847` `TorRuntimeKind::BuiltIn`; bridge `api/state.rs:923-925` `BuiltIn` ("SDK-owned arti instance"); `convert.rs` its `From` arm; `selftest.rs` its mention; the regenerated Dart `TorRuntimeKind.builtIn`; UI `sync_status_presentation.dart:738` the `TorRuntimeKind_BuiltIn()` arm and its three test mentions (`sync_status_presentation_test.dart`) + one (`wallet_screen_test.dart`) | the kind is enumerable though never produced | REMOVED end to end; `bridge_enums_cover_core_variants` stays the arbiter; the FRB regen and `wallet-bridge-verify` prove the lockstep; the UI arm collapses to `ExternalSocks5 \|\| Dialer` (C1; every cited line the change moves is re-watched) |
| A7 | `tor_status.rs:11-14` "`Bootstrapping` is reachable only for an SDK-DRIVEN runtime (built-in arti, the `tor-builtin` feature) whose progress surfaces as `SyncStatus::Connecting`; a host `Dialer`/`ExternalSocks5` runtime hands the SDK no bootstrap progress"; its `:654` test-helper doc "the two runtimes constructible without `tor-builtin`" | FALSE since S281: a `HostDialer` reaches `Bootstrapping { percent: readiness }` through the descriptor (`live_tor_state`'s fourth input) | REWRITTEN: "`Bootstrapping` is reached through a registered dialer's descriptor (readiness < 100 — the plugin's arti bootstrap arrives this way); `Dialer`/`ExternalSocks5` hand the SDK no progress"; `:654` loses its clause (C1) |
| A8 | `state.rs:55-59` `SyncStatus::Connecting { tor_bootstrap_percent }` doc "First-launch arti bootstrap can take MINUTES … arti exposes progress, we surface it" | a producer that never existed in core (the B-2-d NaN fold recorded it "latent — no producer of `Connecting` yet") | the variant STAYS (a forward seam; removing a public enum arm is a bigger change than this feature needs); its doc is rewritten to say the plugin's progress arrives as `TorState::Bootstrapping` through the descriptor, not through `Connecting` (C1) |
| A9 | `extraction_policy.rs:1164` "BuiltIn not compiled yet; Dialer — a Rust host …" | the deliberate-subset comment | rewritten to name the one non-Dart runtime, `Dialer` (C1) |
| A10 | `host-transport-crossing.md` "Does NOT own: built-in arti (FR-5, its own spec, after)"; §1.2 D8 "Built-in arti (when it exists) yields by PRECEDENCE"; §2's note "the built-in Tor (FR-5) is the one transport the SDK names itself (`TorRuntimeKind::BuiltIn`)"; §2.1 "Beside a registered dialer, `BuiltIn` configured is accepted and the registered dialer WINS"; §0 A14's last sentence | written when FR-5 was a cargo feature | a dated revision note at the crossing spec's head (the ADR-0547 precedent): FR-5 is a REGISTRANT of this contract, not a fourth runtime; D8 becomes "the plugin yields by INIT ORDER (ADR-0548 D4)"; the `BuiltIn` sentences are struck (C1; the crossing spec's status stays `Reviewed`) |
| A11 | ADR-0547 Decision 4 "The one name the SDK owns is its own built-in Tor (FR-5, when it exists): `TorRuntimeKind::BuiltIn` names itself" | conditional on a built-in that ADR-0548 D2/D3 replaced | DISCHARGED, not contradicted: the built-in never exists; the plugin names its transport "Tor" through the descriptor exactly as any registrant does, and the SDK owns NO transport name at all. ADR-0548 D2 already says the state reads `hostDialer`; the ADR-0547 status line gets a dated BACK-ANNOTATION naming the discharged decision — the ADR-0542 / ADR-0544 D7 precedents of S282, which `docs/adr/README.md` had never written down (the arch angle's MEDIUM): the README's Rules gain the back-annotation rule in this commit, so the practice is a rule and not an imitation; no new ADR (0548 IS the superseding record) |
| A12 | `host-feature-requests.md` FR-5 "Acceptance (if it is ever revived): a `--features tor-builtin` build syncs; `TorState::Active{BuiltIn}`" | the 2026-08 acceptance | rewritten to §8's acceptance: the example app with the plugin added, `required(hostDialer)`, syncs to tip and sends once over Tor on both OSes; the state reads `active(hostDialer(name: "Tor", isolation: supported, exposure: hidden))` (this commit) |
| A13 | `docs/ROADMAP.md` W5 item "(4) FR-5 — optional built-in arti (`tor-builtin`, off by default; …)" (was `:1066`; now the plugin wording at `:1067-1070`) and the order entry's "built-in Tor" (`:974`) | the pre-ADR-0548 wording, beside the current 23:30 order entry | LANDED (this commit): the W5 item points at the order entry with the plugin wording; the order entry drops "built-in" — two entries on one concept was the duplicate-is-a-bug rule |
| A14 | `docs/arch/overview.md:201` names `zec_wallet` + `zec-wallet-core` only | the crate map | gains `zec_wallet_tor` (Flutter plugin, cargokit crate `zec_wallet_tor` over `dialer-tor`) in the SAME change that creates the package (stage 3, C2) |
| A15 | `release-checklist.md` §B | the wallet's size table | gains the plugin's row: `libzec_wallet_tor.so` per ABI, the tarball, cold and warm bootstrap on each OS (stage 5) |
| A16 | `justfile` `flutter-ci` legs (`zec_wallet:test`, `zec_wallet_ui:*`), `sdk-gate-core`'s cargo legs, `sdk-swap-off-guard`'s symbol needle, `wallet-bridge-verify` (one FRB crate), `supply_chain_policy::every_sdk_test_target_is_named_by_a_live_carrier` | three packages, one bridge crate | gains `zec_wallet_tor:{format,analyze,test}`, a `cargo test -p zec_wallet_tor` leg, an `ffigen` verify leg for the plugin's own header (the verify recipe's shape: regenerate, diff — §2.1), and CRATE-ANCHORED needles in the swap-off guard (→ P15 "the wallet library carries no arti symbol": `arti_client`, `dialer_tor`, `tor_proto`, `tor_rtcompat`, `tor_netdir`, `tor_guardmgr`, `tor_cell` — MEASURED S284: the bare `arti` and `tor_` this row first named match 3270 and 491 of the Tor-free wallet's symbols today, OpenSSL's `partial`/`factor_`/`generator_`; `fr5-phase-1.md` §0.1 F8); NO new `tests/*.rs` target (§8 — the registry tests live inside the cabi module's test mod), so the live-carrier test is untouched (stage 3, C2) |
| A17 | `include/zec_wallet_net_dialer.h` | ABI v2, "the host resolves the three verbs by NAME from the already loaded wallet image" | The RESOLUTION rule is UNCHANGED and this spec adds NO verb: the plugin is a host in the contract's sense and resolves the same way (§3.2). **The version is NOT unchanged, for a reason outside this spec:** FR-30 (b) / ADR-0549 added the descriptor's `health` axis, so C1 took the header to **v3** (2026-09-17). The plugin registers with `ZW_NET_DIALER_ABI_VERSION` = 3 and pushes `health` (§3.4); nothing else about its use of the contract moves. **S292: = 4** — stage S1 took the header to 4 (ADR-0553 as built: `FAILED` is `Preferred`-switch-eligible after the patience minute), the plugin's §3.4 `Failed` phase is exactly that never-starts case, so the mirror followed with no change to what it pushes |
| A18 | `docs/plan/fr5-session-brief.md` §0 "the core's `tor-builtin` feature and `TorRuntime::BuiltIn` either serve pure-Rust hosts or are removed; the spec says which"; its §0 sentence "bundles `dialer-tor` in its own native library, runs its own tokio runtime" | the open question; the runtime sentence | ANSWERED: removed (§1.2 D3); the runtime sentence is confirmed and sharpened (D7: the plugin's own runtime, no FRB) |
| A19 | `sdk/deny.toml` `[bans]` — `aws-lc-rs`/`aws-lc-sys` only | no arti in the SDK's graph, so no LGPL row | gains the `equix`/`hashx` bans (Relim's rows, §4) in the change that adds `dialer-tor` to the lock (stage 3, C2) |
| A20 | `host-session-complete-integration.md` H-15 "Do:" — the crate's move as a relocation | nothing about the dormant verb | gains ONE line: `dialer-tor` exposes `TorDialer::set_dormant(DormantMode)` + the `DormantMode` re-export (§3.6) — the register Relim reads (this commit); after the crypto angle, a SECOND line: the timeout / network error split (§3.6 item 2) |
| A21 | `host-feature-requests.md` FR-30 (Relim, S401, 17 Sep): (a) the failing arms hard-code "Tor"; (b) a readiness of 0 renders as *starting* for a transport the host declared FAILED — the descriptor has no health axis; (c) the tone map | filed against the SDK the same night this spec was written | the plugin is the SECOND registrant with (b)'s gap (E21: its `Failed` phase renders `Bootstrapping`). The SDK's answer is in the FR-30 entry: (a) carry the descriptor's name on `Bootstrapping`/`Unavailable` (rides this spec's stage-3 regen); (b) RULED by the founder 17 Sep ~02:00 — ADR-0549: a fourth closed descriptor integer `health` at ABI v3, bundled with this spec's stage 3 (C1) and landed before Relim's walk; (c) tone, UI-only. This spec's pushes gain `health = FAILED` in the `Failed` phase (§3.4) and nothing else changes; P3 and P14 gain the field at C1 |
| A22 | `extraction_policy.rs:88-91` — `core_and_bridge_have_no_relim_deps` scans the bridge manifest's EVERY `*dependencies` section (dev included) for `path =` / `git =` against the sanctioned list `["zec-wallet-core", "zec-wallet-swap-near"]` | the bridge may path-depend on two crates | §8's placement makes the bridge DEV-depend on `zec_wallet_tor` by path; the sanctioned list gains `zec_wallet_tor` in the SAME chunk as that line (an extraction-unit-internal edge, the §1.2 rule) — else the existing test goes red on the day the dev-dependency lands (the arch angle's MAJOR; P16 names the line) (stage 3, C2) |
| A23 | `docs/ROADMAP.md` the June steer paragraph — "**FR-5** built-in arti (standalone pkg)" and "the Dialer is the host path (FR-4), `tor-builtin` is standalone-only (FR-5)" (were `:292` / `:296-297`) — a LIVE rule sentence in the retired vocabulary, beside the current order entry | two descriptions of FR-5 in one file | LANDED (this commit): both folded to the plugin wording with a pointer to the order entry (the arch angle's MAJOR — A13 had named the W5 item only) |
| A24 | `sdk/zec_wallet_ui/lib/core/lifecycle/app_lifecycle_provider.dart` doc: "the frame ships the seam so the first FFI consumer has no reason to hand-roll its own observer"; desktop's `hidden` rule | THE lifecycle seam for the UI package's stream consumers | the plugin registers its OWN `AppLifecycleListener` (§3.1) — a stated EXCEPTION, not a lapse: the plugin sits BESIDE `zec_wallet`, below `zec_wallet_ui`, and cannot depend on a Riverpod provider in the UI package (a host that uses no UI package still needs the transport to go quiet before the OS acts); its action is a transport obligation on the native side, not a stream-pause convenience. The provider's desktop rule (`hidden` ≠ pause) is restated, not duplicated as a predicate — one signal, two observers with different duties. The provider's doc gains one sentence naming the exception (stage 3, C3) |
| A25 | `sdk/Cargo.toml`'s "NO [patch]" rule; `extraction_policy.rs:97-129` scans the ROOT manifest's `[patch]`; `sdk/deny.toml` `[sources]` denies unknown registries/git but not a PATH source | a local `.cargo/config.toml` `[patch.crates-io]` is invisible to every gate; a `Cargo.lock` stanza with no `source =` for `dialer-tor` could be committed | a NEW gate, P26: every `[[package]]` in `sdk/Cargo.lock` that is not a workspace member carries `source = "registry+https://github.com/rust-lang/crates.io-index"` — the lock-honesty check §1.4 promised, in `extraction_policy.rs`, mutated by a planted path patch before it is believed (stage 3, C2; the arch angle's MINOR) |
| A26 | `zec-wallet-core/src/store.rs:1039-1071` — `Wallet::wipe`'s sweep removes EVERY entry of `db_dir` (`remove_dir_all` per directory) and a failing final `remove_dir` is warn-and-Ok; `store.rs:657-659` documents `db_dir` as SDK-exclusive | the wallet owns its directory whole | the plugin's tree is a SIBLING (`tor_dir`, §2.3), never inside `db_dir` — a tree under it would be deleted under arti's open handles, re-written, and leave a fresh guard identifier behind a wipe reported as success (the security and crypto angles). The SDK's code does not change; the README's `torDir` rule and P27 carry it |
| A27 | The backup exclusion `db_dir` relies on (`api/config.rs:27-28`: "the host owns backup-exclusion for this dir") is enforced by the SDK nowhere — the example app's manifest sets `allowBackup="false"` and its `AppDelegate` the iOS key; a pub.dev consumer gets Android's default | a host obligation, stated once | the same obligation now covers `tor_dir`, whose files are PLAINTEXT arti formats (guards, cached bridge descriptors — §4); the README and the threat model state the at-rest residual for both directories. FOLLOW-UP, not this spec: a platform courier that hands the SDK and the plugin the no-backup directory (`getNoBackupFilesDir()` / the iOS key applied natively) — filed in the S284 start-here's queue |
| A28 | `wallet-sdk.md` §3.2a's body: `:1803-1809` (the `tor-builtin` TLS-path sentences), `:1821-1825` ("the deferred `tor-builtin` arti … the built-in arti `BuiltIn` runtime is a SECOND, deferred impl"), `:1840`, the matrix row `:1853` "`{ BuiltIn }` — not compiled (feature `tor-builtin` is off by default)", `:1871-1872` ("`ExternalSocks5` and `BuiltIn` (arti) dialers are named follow-ups"), `:1893` | the June connector design's references to a deferred built-in | REWRITTEN with C1: the connector has ONE TLS path and the registered dialer (the plugin included) rides it; the matrix row `{ BuiltIn }` is removed; the follow-up sentence names the plugin (the docs angle's MAJOR — A1–A3 had named §1.8/§2.3/§3.2's heads only) |
| A29 | `wallet-sdk.md` §8 register row `:12346` "`builtin_tor_feature_off_compiles_without_arti` — §2.3 `tor-builtin` gate" | a named test that has NO test fn (listed among the absent names in `docs/plan/production-readiness-phase-3.md:97`) | the row is RETIRED with the feature (C1) — its replacement is P15, the symbol-level proof that the wallet library carries no arti, which exists as a gate rather than a name (the docs angle's MAJOR) |
| A30 | ADR-0526 (`:39,52,70,79`): "Built-in arti (cargo feature `tor-builtin`) … kept as an optional `tor-builtin` impl … Follow-ups: the `ExternalSocks5` and `BuiltIn` (arti) dialer impls" | the original NetDialer ADR's follow-up | its status line gets the dated back-annotation to ADR-0548 (this commit; the docs angle's MINOR); the port and the `Dialer` runtime it froze stand |

Nothing else in the corpus carries a second copy of these predicates. The
predicate "the SDK never dials while a registered dialer's readiness is below
100" has ONE home (`readiness_gate.rs`) and this spec adds no second one: the
plugin's synchronous `NOT_READY` refusal (§3.3) is the header's obligation on
the registrant, not a duplicate of the gate.

---

## 1. Design decisions

### 1.1 The problem

A pub.dev consumer that runs no transport of its own gets clearnet: the
wallet's only Dart-expressible Tor runtime, `hostDialer`, needs a native
registrant, and writing one means C, arti and a second runtime — exactly the
work the SDK exists to spare a host. ADR-0542 Decision 2 answered with a
cargo feature inside `zec_wallet`; ADR-0548 refuted that shape (cargokit
cannot select a feature per consuming app; every consumer would pay arti's
size or the SDK would fork into two names) and chose an OPTIONAL SECOND
PLUGIN that registers Tor through the contract FR-29 built, over the dialer
Relim already ships. This spec designs that plugin.

### 1.2 The decisions (D1–D5 founder-ruled 16 Sep 23:50 and recorded as ADR-0548; D6–D12 are this spec's, within them)

| # | Decision | Ruled by |
|---|---|---|
| D1 | The shared Tor dialer is the crate `dialer-tor` in `alvorado-lab/dialer-tor`, published to crates.io; Relim's `crates/transport-tor` moves there. The plugin consumes it BY VERSION; during development a `[patch.crates-io]` path (§1.4). **S284 (ADR-0550, founder ~10:15 — "make everything locally properly and when we need, we push it"): developed HERE first — `sdk/dialer-tor`, copied from Relim's `main` crate and consumed by PATH; ITS OWN cargo workspace under `sdk/` (measured at the resolver: arti's `crypto-common ^0.2.2` and the ZEC stack's `=0.2.0-rc.1` cannot share a lock), with its own lock, `deny.toml`, audit and lint legs; the public repository and the publish are the founder's later act; the `[patch]` shape is not used** | ADR-0548 D1; ADR-0550 |
| D2 | Tor for a standalone host is the optional Flutter plugin `zec_wallet_tor`: its own native library (cargokit crate `zec_wallet_tor` → `libzec_wallet_tor.{so,a,dylib}`), its own async runtime, registering at init through `zec_wallet_register_net_dialer` with the descriptor `{ name: "Tor", readiness: arti's bootstrap, isolation: SUPPORTED, exposure: HIDDEN }` and pushing through `zec_wallet_net_dialer_notify`. The host configures `TorPolicy.required(runtime: TorRuntimeConfig.hostDialer())` and reads `TorRuntimeKind.hostDialer(name: "Tor", …)` | ADR-0548 D2 |
| D3 | **`zec_wallet` stays Tor-free, and the built-in runtime is REMOVED**: `tor-builtin`, `TorRuntime::BuiltIn`, `TorRuntimeKind::BuiltIn` and every arm, doc and test that names them (§0 A2–A9). No consumer exists: a Dart host adds the plugin; a Rust-workspace host (FR-6, demand-driven) consumes `dialer-tor` directly and hands the wallet `TorRuntime::Dialer(Arc<dyn NetDialer>)` — a forty-line adapter in the host, the direction `dialer-tor`'s own module doc names ("an adapter in the consuming product implements the port over this dialer, never the other way round"). Keeping a cfg'd-out variant for a host that does not exist is dead surface a public 0.0.1 would have to document | ADR-0548 D3 (delegated to this spec) |
| D4 | The host surface: bridge lines may be supplied (bounded and classified by `dialer-tor`, §3.1); arti's `state` and `cache` live under the wallet's data directory (§2.3), backup-excluded by the host exactly as the DB is; bootstrap progress flows through the descriptor into the existing chip; a blockage flows as a readiness that never reaches 100 PLUS a typed status the plugin's Dart surface exposes (§3.1 `TorPluginStatus`) | ADR-0548 D5 (D3 of the brief) |
| D5 | The mobile behaviour is designed from the platform facts in §9 (sourced): a pause pushes readiness DOWN before the OS kills the sockets and sets arti dormant; a resume after a pause long enough to have been suspended REBUILDS the client from arti's persisted cache (a warm bootstrap), and readiness climbs back through the descriptor; under `required` there is no fallback at any point; ~~a dial the plugin accepted before a pause completes `RETIRED`, never `UNREACHABLE`/`TIMEOUT`~~ *(corrected S305, the crypto angle on C3b: this clause contradicted §3.4, which the build follows — a pause does NOT bump the generation, so an op in flight completes on its merits; only a REBUILD retires it)* (§3.4) | ADR-0548 D5 (D4 of the brief); §9 |
| D6 | **Register first, bootstrap only on success** (the second-Tor-client rule for a plugin): `init` calls `register` with readiness 0 BEFORE constructing arti; `ZW_RC_OCCUPIED` ends init typed (`slotOccupied`) with arti never constructed and the plugin reporting `notRegistered`. Which registrant speaks is decided by the host's init order. Named test P2 | ADR-0548 D4; ADR-0544 D7 |
| D7 | The plugin's Dart surface is a thin `dart:ffi` layer over the plugin's OWN small C ABI (`include/zec_wallet_tor.h`: eight verbs and one fixed-width status struct, §3.1), with `ffigen`-generated bindings and a `NativeCallable.listener` for status pushes — NOT a second flutter_rust_bridge library. MEASURED, not preferred: on iOS/macOS the wallet is a `-force_load`ed STATIC library (`ios/zec_wallet.podspec`), FRB 2.12's runtime exports unprefixed `#[no_mangle]` C symbols (`frb_init_frb_dart_api_dl`, `frb_pde_ffi_dispatcher_primary`, `frb_dart_fn_deliver_output`, …) and its codegen has no symbol-prefix option (`flutter_rust_bridge_codegen 2.12.0 generate --help` — none), so a second FRB static library in one Runner is a duplicate-symbol link failure. The plugin owns its tokio runtime; `flutter_rust_bridge` stays out of its graph | this spec (verified 17 Sep) |
| D8 | The plugin resolves the wallet's three verbs BY NAME from the already-loaded wallet image — `dlopen("libzec_wallet.so", RTLD_NOLOAD)` on Android/Linux, on Apple a walk of the loaded mach-o images (`_dyld_image_count` / `_dyld_get_image_name`) for the wallet's image — the main executable when the wallet is statically linked (the SDK's own podspec), the `zec_wallet.framework` binary when a host embeds it as a framework (Relim) — reopened with `RTLD_NOLOAD` and verified by `dladdr` that the resolved address belongs to that image (Relim's `wallet_registrar` anti-interposition check; the seed-port precedent, device-confirmed), `GetModuleHandle` on Windows — never `RTLD_DEFAULT`, never a link-time reference (an undefined symbol in `libzec_wallet_tor.so` would fail `dlopen` on Android where the wallet is a separate library loaded `RTLD_LOCAL`). One code path on every platform, the header's own rule | the header; Relim's `wallet_registrar::resolve_wallet_symbol` |
| D9 | The plugin's HOME is the SDK repository, `sdk/zec_wallet_tor`, beside `zec_wallet`: versioned in lockstep, tested against the ABI in the same workspace through the real registry (the fake-host pattern in reverse, §8 P19), reviewed as one unit, extracted as one unit. The extraction carries four packages; the publish order is `dialer-tor` (Relim's, crates.io) → `zec_wallet` → `zec_wallet_ui` → `zec_wallet_ui_platform` → `zec_wallet_tor` | the brief §0 (confirmed) |
| D10 | The plugin persists NOTHING of its own: bridge lines are the host's setting, re-supplied at every `init`; arti's `state`/`cache` are arti's. No SDK aux row, no plugin database | this spec (scope) |
| D11 | Errors cross to the wallet as the FROZEN five-code table; the plugin's mapping (§3.3) is a POSITIVE allowlist over `dialer_tor::TorDialError` with the unlisted tail mapped to `REFUSED` (fail-closed: never a clearnet fallback for an error the plugin cannot name) | ADR-0544 D5; ADR-0546 |
| D12 | The example app gains the plugin as a path dependency and a settings toggle ("Use the built-in Tor plugin"): on, the example calls `ZecWalletTor.init` before opening the wallet with `required(hostDialer)`; off, the policy is `off`. It is the stage 5 vehicle and the reference for the init order; its native build therefore compiles arti (stated cost: a longer device build, no CI leg builds native) | this spec |

### 1.3 Alternatives rejected

- **A cargo feature in `zec_wallet`** (ADR-0542 D2 as written), **two builds
  of `zec_wallet`**, **a build-time environment door**, **the shared crate
  inside the SDK repository**, **librustzcash's own `tor` feature** — all
  rejected in ADR-0548 § Alternatives; not re-argued here.
- **Keeping `TorRuntime::BuiltIn` for a pure-Rust host.** No such host
  exists; when one does, `dialer-tor` + `TorRuntime::Dialer` is fewer lines
  than a feature gate and carries no arti into the core's dependency graph.
  Rejected (D3).
- **A second flutter_rust_bridge library for the plugin.** One codegen
  discipline across the SDK's packages, typed errors and a stream for free —
  and a duplicate-symbol link failure on every Apple build, because both
  plugins are `-force_load`ed static libraries and FRB's runtime symbols are
  unprefixed with no prefix option in 2.12 (D7, verified). Rejected. The cost
  accepted instead: two Dart-binding disciplines in one repository (the
  wallet's FRB, the plugin's `ffigen` over a C header) and a status stream
  built on `NativeCallable.listener` rather than generated.
- **Link-time resolution of the wallet's verbs** (the plugin's Rust declaring
  `extern "C" fn zec_wallet_register_net_dialer`). Works on Apple (one image)
  and breaks on Android (two `RTLD_LOCAL` libraries). Rejected (D8).
- **Keeping arti warm across every pause on Android** (no rebuild). Cheaper
  per app switch; but a client whose sockets the OS has since killed reports
  `ready_for_traffic()` while every channel is dead, and its first dial
  error maps to a reachability failure — under `Preferred`, a clearnet
  fallback caused by the OS, not the network. Rejected for pauses past the
  suspension threshold; kept for shorter ones (§3.4, §9).
- **Retiring the registration at every pause** (`notify(retire=1)`). Fails
  every in-flight stream at once — right on iOS where they are about to die,
  wrong on Android where a sync in flight can finish in the background.
  Rejected: readiness DOWN (no new dials) plus the lifecycle generation
  (a stale op completes `RETIRED`) gives the same guarantee without abandoning
  work the OS would have let finish.
- **A `zec_wallet_ui` widget for the blockage / bridge entry.** Real, later;
  the chip already renders `Bootstrapping { percent }` and `Unavailable`;
  the host renders the detail from the typed status. Cut (principle 9).
- **Persisting bridge lines in the plugin.** A censorship-sensitive setting
  with a host-visible UI belongs to the host's settings store; the plugin
  would otherwise own a second store and a second wipe path. Rejected (D10).
- **Onion-service support, pluggable transports** (`pt-client`, an obfs4
  binary). The LGPL carve-out and the "a shipped binary a mobile app starts"
  problem are Relim's stated limits too; both stay out. Not a decision this
  spec reopens.

### 1.4 Tradeoffs stated

- **Two Tor clients in one process are possible by host error** (the host
  registers its own dialer AND adds the plugin): the registry decides — the
  second registrant is refused and never starts arti. The plugin cannot see
  the first's transport; it only knows the slot was taken. Accepted (ADR-0548
  D4).
- **The plugin cannot be published before `dialer-tor` is on crates.io**, and
  its committed `Cargo.lock` must resolve from crates.io (the SDK's extraction
  policy). During development the override lives in a LOCAL
  `.cargo/config.toml` `[patch.crates-io]` entry (gitignored; cargo reads
  `[patch]` from config), never in the committed workspace manifest —
  `sdk/Cargo.toml`'s "NO [patch]" rule stands. No existing gate sees a local
  patch (§0 A25), so this spec adds one: P26 asserts every non-member
  package in `sdk/Cargo.lock` carries a crates.io `source`, and it runs on
  every commit through `extraction_policy` — a lock resolved through the
  local path cannot be committed unnoticed. **S284 (ADR-0550): NO `[patch]`
  after all — `dialer-tor` lives at `sdk/dialer-tor` as ITS OWN cargo
  workspace (arti and the ZEC stack cannot share a lock — measured, plan §5
  D-7), with its own committed lock, and the plugin's crate is its own
  workspace too, consuming the dialer by path; P26 guards `sdk/`'s lock as
  written; each of the two other locks is graded by its own `cargo deny
  check` (`[sources]` denies unknown registries and git — it does NOT flag a
  PATH dependency, which carries no `source`), so the plugin's path edge to
  `dialer-tor` is the one thing no gate refuses today: its flip to a version
  at the publish is a C3 tripwire (the plugin's own P26 twin, asserting the
  one permitted sourceless package by name until then). The plugin CAN land
  before the publish; only the pub.dev publish of `zec_wallet_tor` waits on
  it (checklist §D).**
- **The watcher copied from Relim** (§3.4) is a second implementation of one
  behaviour; its retirement — a readiness watcher inside `dialer-tor` that
  both registrants consume — is a stated LATER item on the board's FR-5a row
  (the register for the shared crate), not a wish in this file alone.
- **Every consumer that adds the plugin pays arti's size** (measured at stage
  5, per ABI, into checklist §B) and a bootstrap at first launch (seconds warm,
  longer cold — §9's numbers). A consumer that does not add it pays nothing.
- **Two Rust static libraries in one Apple executable is a LINK-MODEL risk
  beyond FRB** (the crypto angle's observation on D7): `ring` ships C/asm
  objects under fixed `ring_core_0_17_14_*` names, Rust's own `__rust_*` /
  `rust_eh_personality` shims are C-ABI, and both `libzec_wallet.a` and
  `libzec_wallet_tor.a` would carry copies. Whether the linker refuses
  depends on `-force_load` (the SDK's podspec force-loads the wallet; the
  plugin's must NOT force-load its whole archive — it keeps its eight
  `zec_wallet_tor_*` verbs alive with `-u` per symbol so the plugin's `ring`
  objects are never pulled and the wallet's satisfy the references). **The
  plan's FIRST probe (stage 2), before any other code:** link the example
  Runner with both archives and take a duplicate-symbol census. If the
  linker refuses, the plugin ships on Apple as a DYNAMIC framework — the
  shape Relim already uses for the wallet (`zec_wallet.framework`,
  `wallet_registrar.rs:183-186`) — built from the crate's `cdylib` and
  vendored by the podspec; the spec's contract does not change, the
  packaging does. **MEASURED S284 (`docs/plan/probes/apple-duplicate-symbol-census.sh`;
  `fr5-phase-1.md` §0.1):** two force-loaded archives fail with 2933
  duplicates; the wallet force-loaded + the plugin plain + `-u` per verb
  links with one `ring` copy, order-independent, under `-dead_strip` and
  `-ObjC`; without `-u` the verbs are stripped; and as its OWN dynamic
  library from the archive with `-u` (the `use_frameworks!` shape — the SDK's
  example's own, see §3.2) the plugin exports its verbs with its own `ring`
  and no collision. So ONE podspec shape serves both CocoaPods linkages and
  the dynamic-framework fallback is RETIRED. The `-u` list is ELEVEN symbols,
  not eight: a force-loaded, unreferenced verb does not survive `-dead_strip`
  (the wallet's verbs carry no no-dead-strip flag), so the plugin also names
  `zec_wallet_register_net_dialer` / `_update_net_dialer` / `_net_dialer_notify`
  — free, and it roots them for a `:linkage => :static` host that dead-strips.
- **The local network learns that this device uses Tor** (guard connections
  are recognisable) unless bridges are supplied — the standing property of
  Tor without pluggable transports, stated in the README, not hidden.
- **The plugin's watcher is a copy of Relim's readiness watcher** (§3.4:
  the same cadences, confirmations and jitter, with the same WHYs). Two
  products, one behaviour, two copies — accepted for this phase; the shared
  watcher inside `dialer-tor` is the tracked LATER item above.

---

## 2. Domain types (Rust; the plugin crate `sdk/zec_wallet_tor/rust`, package `zec_wallet_tor`)

The crate is `#![deny(unsafe_code)]` with ONE `#[allow(unsafe_code)]` module,
`abi.rs` (§4). It depends on `dialer-tor`, `tokio`, `tracing`, `thiserror`,
`zeroize`, `libloading` and `libc` (the resolver, Relim's choice) — and NOT
on `flutter_rust_bridge` (D7), `zec-wallet-core` or the bridge crate (the
wallet is reached by symbol at runtime, D8). The dependency in the OTHER
direction exists for tests only: the plugin crate is an `rlib` as well as a
`cdylib` / `staticlib`, and the bridge crate names it as a DEV-dependency so
the registry-touching tests can live where the registry's test seam is (§8).
**WITHDRAWN S284 (ADR-0551; the plan's §5 D-7/D-8): the bridge's dev-graph
resolves in `sdk/`'s lock, and arti cannot enter that lock (`crypto-common
^0.2.2` against the ZEC stack's `=0.2.0-rc.1` — measured at C0). The plugin's
crate is its OWN workspace under `sdk/` (`cdylib` + `staticlib`, no `rlib`)
consuming `dialer-tor` by path; the registry-touching rows' placement is
decided at C3's start between (a) a two-library integration test in the
plugin's workspace — the wallet's built cdylib and the plugin's loaded by
symbol, production's shape — and (b) the fake-registry half alone. Not yet
decided; one arch review pass on the choice.** *DECIDED S286 (2026-09-18),
the one arch review pass: (a) for P19, P3's real registration and P2's real
half; (b) for everything (a) cannot reach — plan §5 D-8 carries the reasons
and the stated gaps.*

```rust
/// The name the plugin registers — a proper noun, unlocalized by design
/// (the descriptor's name is rendered verbatim and never localized; Relim
/// registers the same word). ≤ ZW_TRANSPORT_NAME_MAX_BYTES, no control
/// characters — P3 pins that the crossing accepts it.
pub const TRANSPORT_NAME: &str = "Tor";

/// Readiness values the plugin pushes (the header's `readiness` field).
/// READY: arti's `ready_for_traffic()` is true — the ONLY value at which the
/// SDK calls `dial`. CEILING_WHILE_BOOTSTRAPPING: arti's `as_frac()` can read
/// 1.0 before `ready_for_traffic()` (the directory is usable, no channel yet),
/// and 100 is the one value the wallet renders as a working private path —
/// so a bootstrapping client is capped one below it. SUSPENDED: what a pause
/// pushes — 0, not "the last value", so a resumed wallet never sees a stale
/// 100 (§3.4).
pub const READINESS_READY: u32 = 100;
pub const READINESS_CEILING_WHILE_BOOTSTRAPPING: u32 = 99;
pub const READINESS_SUSPENDED: u32 = 0;

/// What the plugin is doing, as the host's Dart reads it (C-ABI mirror
/// `TorPluginPhase`). A CLOSED set; no free text beside it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    /// `init` has not run, or it failed before registration.
    Idle,
    /// Registered; arti is bootstrapping (readiness < 100). `since` in the
    /// status says how long.
    Bootstrapping,
    /// Registered; arti is ready for traffic (readiness 100).
    Ready,
    /// Registered; the bootstrap attempt ended without readiness — the
    /// deadline elapsed or arti reported failure — and the plugin is waiting
    /// out its retry backoff (or a manual `retryBootstrap`). Readiness < 100.
    Failed,
    /// The app is paused (`AppLifecycleState.paused`): readiness pushed to
    /// SUSPENDED, arti dormant. Left on `resumed`.
    Suspended,
    /// `init` was refused at the crossing — the slot was occupied, the ABI
    /// mismatched, the wallet image was not loaded — or `dispose` ran. Arti
    /// was never started, or has been torn down. A later `init` registers
    /// again (§3.1's dispose row names what persists).
    NotRegistered,
}

/// What blocks the bootstrap, in arti's own terms — a 1:1 CLOSED mirror of
/// `arti_client::status::BlockageKind` at the pinned version, plus `Unknown`
/// for the `#[non_exhaustive]` tail. The six variants of the 0.45.0 source
/// (`arti-client-0.45.0/src/status.rs:170-193`, read 17 Sep): `Disabled`
/// (bootstrap not yet asked for — `BootstrapBehavior::Manual`), `Offline`,
/// `Filtering` ("our connections seem to be filtered"), `CantReachTor`,
/// `ClockSkewed`, `CantBootstrap` (a directory problem). P24 pins the list
/// against the locked source. Rendered by the host; never interpreted by the
/// plugin beyond "not ready". Crosses the C ABI as `ZWT_BLOCKAGE_*`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Blockage { Disabled, Offline, Filtering, CantReachTor, ClockSkewed, CantBootstrap, Unknown }

/// The frozen, PII-free class labels the plugin's status and log lines carry
/// (the `TorDialError::class` convention of `dialer-tor`, plus the plugin's
/// own two). ONE home; the Dart `TorPluginStatus.failureClass` carries the
/// same string.
pub const CLASS_BOOTSTRAP_DEADLINE: &str = "bootstrap-deadline";
pub const CLASS_NOT_REGISTERED: &str = "not-registered";
// `dialer-tor`'s own: "bootstrap-failed", "setup", "not-bootstrapped", the
// five bridge classes (`CLASS_CONFIG_TOO_LONG`, `CLASS_TOO_MANY_LINES`,
// `CLASS_LINE_TOO_LONG`, `CLASS_PT_UNSUPPORTED`, `CLASS_UNUSABLE`) — consumed
// by re-export, never re-minted.

/// The typed status (C-ABI mirror `TorPluginStatus`). No path, no bridge, no
/// key, no destination — every field is a closed value or a small integer.
#[derive(Clone, PartialEq, Debug)]
pub struct Status {
    pub phase: Phase,
    /// The readiness LAST PUSHED to the wallet (0..=100) — what the chip shows.
    pub readiness: u32,
    pub blockage: Option<Blockage>,
    /// One of the class constants when `phase == Failed`/`NotRegistered`
    /// (crosses the C ABI as a closed `ZWT_CLASS_*` code — no string crosses;
    /// the Dart side owns the code → name table).
    pub failure_class: Option<&'static str>,
    // `since_secs` and `bridge_lines` were CUT at the design review (the
    // founder's S266 rule): display-only fields no consumer named.
}

/// The plugin's configuration as `init` receives it (C-ABI mirror
/// `TorPluginConfig`).
pub struct Config {
    /// A directory of the PLUGIN'S OWN — a SIBLING of the wallet's `db_dir`,
    /// never `db_dir` itself and never a path inside it (§2.3: the wallet's
    /// wipe sweeps `db_dir` whole). Absolute; the plugin creates
    /// `<tor_dir>/state` and `<tor_dir>/cache` (0700 on unix). Relative or
    /// empty is refused typed (the RW-CFG-003 shape). NEVER logged (a device
    /// path is PII).
    pub tor_dir: String,
    /// Bridge lines as pasted (≤ `MAX_BRIDGE_LINES` × `MAX_BRIDGE_LINE_BYTES`
    /// after `dialer-tor`'s bounds); `None` = arti's directory bootstrap.
    /// Wrapped `Zeroizing` at the boundary; classified, never echoed.
    pub bridges: Option<String>,
}

/// The seam under the plugin — what the trampoline and the lifecycle drive.
/// Production: `ArtiEngine` over `dialer_tor::TorDialer`; tests: a fake with
/// a scripted readiness and a loopback `connect` (§8). Object-safe, `Send +
/// Sync`. `#[async_trait]` per rust-patterns. It returns `dialer_tor`'s OWN
/// error type on purpose (the arch angle's MINOR, accepted): the plugin's
/// taxonomy is the five-code table in §3.3 and lives in ONE `match`; a
/// plugin-owned error enum between the two would be a second taxonomy with
/// its own drift (the `dialer-tor` → Relim `classify` precedent keeps one
/// table per consumer). `#[non_exhaustive]` blocks exhaustive MATCHING
/// downstream, not construction of the existing variants, so the fake can
/// produce every row of §3.3 (P9).
pub trait TorEngine: Send + Sync {
    async fn bootstrap(&self) -> Result<(), dialer_tor::TorDialError>;
    fn readiness(&self) -> dialer_tor::Readiness;
    async fn connect(&self, host: &str, port: u16, isolation_key: Option<&str>)
        -> Result<Box<dyn dialer_tor::AsyncByteStream>, dialer_tor::TorDialError>;
    /// The crate's own vocabulary (S284: `dialer_tor::DormantMode`, `Soft` /
    /// `Normal`), not a bool — the fake takes the same type. `TorDialer` also
    /// answers `dormant_mode()`, what was last ASKED; the plugin's status
    /// reports its own phase, never that query (arti wakes itself on any use).
    fn set_dormant(&self, mode: dialer_tor::DormantMode);
}

/// How an engine is minted — the seam P1/P2 count calls on ("arti was never
/// constructed"). Production builds `TorDialer::with_bridges(state, cache, lines)`.
pub trait EngineFactory: Send + Sync {
    async fn mint(&self, cfg: &EngineConfig) -> Result<Arc<dyn TorEngine>, dialer_tor::TorDialError>;
}

/// The wallet's three verbs, resolved by name (D8). Production: `ImageResolver`;
/// tests: the bridge crate's real functions handed as pointers (P19) or a
/// recording fake (P1–P3, P6, P21).
pub trait VerbResolver: Send + Sync {
    fn resolve(&self) -> Option<Verbs>;   // None = the wallet image is not loaded
}

/// The lifecycle generation: bumped by a pause, a rebuild and a dispose. Every
/// accepted op carries the generation it was accepted under; a completion for
/// an older one reports RETIRED whatever the engine said (§3.4).
pub struct LifecycleGeneration(AtomicU64);
```

### 2.1 The two ABI mirrors (`abi.rs`)

**Toward the wallet:** the plugin declares its OWN `#[repr(C)]` mirrors of
`zw_transport_descriptor` and `zw_net_dialer_v1`, the six `ZW_DIAL_*` codes,
the seven `ZW_RC_*` codes, the three `ZW_HEALTH_*` values,
`ZW_NET_DIALER_ABI_VERSION` (**4** since S292 — stage S1 took the header to 4
for ADR-0553 as built and the mirror sat at 3, P14 red, with no chain a human
types running this crate's tests: the 2026-09-20 review's M02; 3 from C1 to
then), the token width
(32) and the three bounds — independently of the wallet's Rust types (the
wallet is not a dependency), and P14 parses `include/zec_wallet_net_dialer.h`
from the sibling package and pins every value and the struct's size and
offsets, so a drift between the header and the plugin fails a test rather than
a device.

**Toward Dart:** the plugin owns a header of its own,
`sdk/zec_wallet_tor/include/zec_wallet_tor.h` — the eight verbs (§3.1), the
fixed-width `zwt_status` struct, the closed sets `ZWT_RC_*`, `ZWT_PHASE_*`,
`ZWT_BLOCKAGE_*`, `ZWT_CLASS_*`, and the status-callback typedef
*(BUILT at C3a, S286, with three dated departures — plan §5: the header is at
`sdk/zec_wallet_tor/rust/include/`, the wallet's own precedent (D-13); it has
NINE exports, the ninth `zec_wallet_tor_abi_version()` read by name before any
other call, the wallet's FR-33 lesson (D-11); `ZWT_RC_*` gains `NULL_ARG` and
`PANICKED` for the `catch_unwind` edges (D-10); and `ZWT_CLASS_*` carries all
fifteen of `dialer-tor`'s bridge classes, not five, so no refused paste reaches
the host without its reason (D-15))*;
`zwt_status_fn(void *ctx, const zwt_status *)`. `ffigen` generates the Dart
bindings from it (the Flutter FFI-plugin template's shape); a verify leg
regenerates and diffs them exactly as `wallet-bridge-verify` does for FRB;
P14's sibling pins the Rust constants against this header too.

### 2.2 The Dart DTOs (hand-written over the `ffigen` bindings)

`TorPluginConfig { torDir, bridges? }` (crosses as two `(ptr, len)` byte
spans valid for the call only; the bridges span is zeroed after the call —
the Dart `String` behind it cannot be zeroized, the documented §10 exposure
the SDK's swap JWT and endpoint key already carry; the HOST persists the
paste, not the plugin),
`TorPluginStatus { phase, readiness, blockage?, failureClass? }` (read from
one `zwt_status` by value — closed integers only; no string crosses), `TorPluginPhase`, `TorBlockage`, and the typed error
`TorPluginError { kind: TorPluginErrorKind, code: int?, message }` with `kind
∈ { walletNotLoaded, abiMismatch, slotOccupied, registryPoisoned,
descriptorRefused, invalidDataDir, bridgesRefused(class), engineSetup(class),
notInitialized, disposed }` — every kind a closed value mapped from the
`ZWT_RC_*` table, the wallet crossing's `rc` carried as an integer for the
log, never as the discriminator (catch by TYPE, never by text — the standing
gotcha); `message` is COMPOSED on the Dart side from the kind and the code —
no string crosses the C ABI in either direction.

### 2.3 Where arti's files live

```
<tor_dir>/state/    guards, bridge state (arti's `state_dir`)
<tor_dir>/cache/    consensus + microdescriptors (arti's `cache_dir`)
```

`tor_dir` is a directory of the PLUGIN'S OWN, a SIBLING of the wallet's
`db_dir` — never inside it (the security angle's MEDIUM): `Wallet::wipe`
sweeps EVERY entry of `db_dir` with `remove_dir_all` and treats a failing
final `remove_dir` as warn-and-Ok (`zec-wallet-core/src/store.rs:1039-1071`),
so a plugin tree under `db_dir` would be deleted under arti's open handles,
re-written by arti (`guards.json`), and leave a FRESH per-device identifier on
disk behind a wipe the host was told succeeded. The host gives the plugin the
same KIND of directory it gives the wallet — backup-excluded
(`getNoBackupFilesDir()` or `allowBackup="false"` on Android,
`isExcludedFromBackup` on iOS), an obligation the SDK's README states for
`db_dir` and now for `tor_dir`, and which the SDK enforces for neither (§4's
at-rest residual; §0 A27's follow-up). Created by the plugin with `0700`
(unix); a directory the plugin cannot create or open is `InvalidDataDir`
(RW-CFG-003's message shape). `clearState()` removes the plugin's state under
`<tor_dir>`: idempotent, safe after a wallet wipe, refused only while an engine
runs (`dispose()` first). *Corrected S306 (plan §7.6; the C3b code, S305 — the
text said "the whole `<tor_dir>` tree"):* the plugin marks a directory it created with a `.zec_wallet_tor` file, and
`clearState()` acts ONLY on a marked directory — an unmarked one is refused
`InvalidDataDir`, so a mistyped `tor_dir` can never delete a host's files. It
removes `state/`, `cache/` and the marker (a symlink as the entry, never its
target), then the directory itself only if it is then empty; a missing
`<tor_dir>` is success. **The documented wipe order for a host:**
`ZecWalletTor.dispose()` → `wallet.wipe()` → `ZecWalletTor.clearState()` when
the user's intent includes the Tor identity — the guard state is a
per-device identifier by Tor's design (§10), not wallet data, and a wallet
wipe alone leaves it. The example app pins the order (P27).

*(S305, the arch review of C3c)* The example keeps its "Use the built-in Tor
plugin" switch in SharedPreferences — the one non-cosmetic entry that store's
rule admits, recorded at `example/lib/core/theme/appearance_prefs.dart`. It
holds no wallet state and must be read before the wallet exists; the accepted
risk is that it can be lost or restored apart from the wallet, which reads as
OFF — visibly in the switch, never silently mid-session; an UNREADABLE store
fails the boot. A production host keeps its transport choice where it keeps
its own settings.

---

## 3. Interface design (SOLID)

### 3.1 Inbound — the Dart surface (`ZecWalletTor`, `dart:ffi` over `include/zec_wallet_tor.h`)

Each Dart verb is one C verb `zec_wallet_tor_<verb>` returning a `ZWT_RC_*`
code; the Dart layer maps the code to `TorPluginError` by type. The plugin's
library is opened by the Dart side (`DynamicLibrary.open("libzec_wallet_tor.so")`
on Android/Linux, `.process()` on Apple — where the plugin's archive is linked
into its own `zec_wallet_tor.framework` under `use_frameworks!`, or into the
Runner under `:linkage => :static`; `process()` finds it in either image —
the DLL on Windows) — the FFI-plugin template's loader, nothing custom.

| verb | shape | notes |
|---|---|---|
| `Future<TorPluginStatus> init(TorPluginConfig cfg)` | `zec_wallet_tor_init(cfg, on_status, ctx)`: builds the plugin's runtime on the FIRST init only (a `OnceLock`, reused by a later init after `dispose` — §3.3); classifies the bridges AND BUILDS the arti `TorClientConfig` (`dialer_tor::tor_config` — pure-local, no I/O; the refusal arti actually raises for a paste that parses but does not build, `Setup { InvalidConfig }`, is met HERE — the security angle's MEDIUM); resolves the wallet's verbs (D8); REGISTERS with readiness 0 (D6); then mints the engine over the built config and starts the bootstrap task and the readiness watcher | idempotent: a second call returns the current status without re-registering. Throws `walletNotLoaded` (call `RustLib.init()` first), `abiMismatch` (-5), `slotOccupied` (-2, arti never started), `registryPoisoned` (-1), `descriptorRefused` (-6 — a build defect, P3 pins it cannot happen), `invalidDataDir`, `bridgesRefused(class)` (classified and config-built BEFORE registration: a refused paste never leaves the wallet half-registered), `engineSetup(class)` (the mint failed AFTER registration — the plugin retires and CLEARS the slot before throwing, §3.4, so the wallet's door refuses honestly rather than reading `Bootstrapping 0%` forever), `stopping` (-13: a client from before `dispose` is still stopping; the Dart `init` retries for up to `clearStateWaitMax`, 15 s, and yields to a `dispose` that lands meanwhile), `restartRequired` (-14: a client is stuck past `ENGINE_SHUTDOWN_MAX`). With `CLEAR_PENDING_MARKER` present in `tor_dir`, `init` first removes the state as `clearState` would, and fails closed with that removal's code if it cannot (plan §5) |
| `TorPluginStatus get status` | `zec_wallet_tor_status(out)` — one struct by value | cold read; `notInitialized` before `init` |
| `Stream<TorPluginStatus> statusStream()` | fed by a `NativeCallable.listener` (Dart ≥ 3.1; the SDK floor is 3.11) the plugin's Rust invokes on every phase or readiness change from ANY thread — Dart runs it on the isolate's event loop; no polling | the host pauses its SUBSCRIPTION on `AppLifecycleState.paused` and resumes on a REAL `resumed` (the standing rule for every stream); the plugin's OWN lifecycle handling does not depend on it (§3.4). The listener is closed by `dispose` |
| `Future<void> setBridges(String? bridges)` | classify and build the config → readiness to SUSPENDED → §3.4's REBUILD (under `state`: generation bump, every outstanding op marked `Closed`, the old engine dropped) with the new lines → bootstrap → readiness climbs | `bridgesRefused(class)` leaves the running client untouched; `restartRequired` (-14) while a client is stuck past `ENGINE_SHUTDOWN_MAX`, refused before anything is stored, because no new client may start (plan §5) |
| `Future<void> retryBootstrap()` | a user-driven retry: cancels the backoff wait and starts a bootstrap attempt now; the backoff schedule itself is untouched (a `networkChanged()` hint verb was CUT at the design review — E13 covers a network change without it) | no-op unless `phase == Failed` |
| `Future<void> clearState()` | removes `<tor_dir>` — the Tor-identity reset in the host's wipe flow (§2.3's order). *Corrected S286 (plan §5 D-12, D-16): the C verb takes the `tor_dir` span (it must work after a restart with no `init`), and it removes ONLY `<tor_dir>/state` and `<tor_dir>/cache`, then `tor_dir` if left empty — never a recursive delete of a caller-supplied path, which a host passing the wallet's `db_dir` by mistake would lose whole* | idempotent; safe after a wallet wipe; refused (`notInitialized`) while an engine runs — `dispose()` first. *Corrected 2026-10-07 (external review, finding 1): `dispose`'s wait is bounded, so UNREGISTERED is not STOPPED — a mint (an `init`'s or a rebuild's) or a client still referenced past the bound writes `state/` later. The C verb and `init` answer `ZWT_RC_STOPPING` (-13, nothing changed) until every such writer has ended (`PluginState::state_writers_quiet`: no mint in flight, `init`'s included, and every minted client has FINISHED dropping — each is wrapped in `engine::Tracked`, whose guard field drops after the client, so the count falls only once arti's drop-time state write is done; a `Weak` would read dead before it). The Dart `clearState` and `init` retry for up to `clearStateWaitMax` (then 10 s; 15 s since plan §5) before throwing `stopping`. Closed by plan §5 (audit-2026-10-05-fixes.md, 2026-10-07): each client runs on its OWN runtime (`engine::ClientRuntime` then; since §12, `dialer-tor`'s `OwnedDialer`), which arti binds, and whose shutdown on retire ends every task arti started, the circuit manager's drop-time write included; the client is counted from the runtime's construction until that shutdown has finished. A shutdown that overruns `ENGINE_SHUTDOWN_MAX` (10 s) leaves the client counted and stuck: `clear_state` and `init` answer `ZWT_RC_RESTART_REQUIRED` (-14) for the life of the process, and the reset is a clear on the next start. The Dart wait is 15 s, above that bound. The `Tracked` guard above survives only for test engines: a production client is counted by `dialer-tor`'s ledger from its runtime's construction (`ArtiEngine::adopt_live` drops the wrapper's entry). A clear refused with either code writes `CLEAR_PENDING_MARKER` in `tor_dir` (only where the removal itself would run: a real directory holding the plugin's marker, never through a symlink), and the next `init` finishes the reset before Tor starts. Either code is answered only once that record is on disk (file and directory entry synced); a record that cannot be saved, or a directory where none may be written, answers `invalidDataDir` (-6) instead: nothing removed, nothing pending (a record created but not synced is removed again; external review of `962a91de7`). An absent `tor_dir` records nothing: there is nothing to reset, and a client that recreated it outside the plugin's marker would be refused by the next `clearState` (`invalidDataDir`), not silently kept. A record already on disk is not re-synced on the Dart side's 100 ms retries. Tests: `clear_state_and_init_wait_for_a_client_that_outlived_dispose`, `clear_state_refuses_while_an_init_mint_is_in_flight`, `a_tracked_client_counts_live_until_its_drop_has_finished`* |
| `Future<void> dispose()` | `notify(retire=1)`; drain the plugin's OWN outstanding ops — each completes `RETIRED` from its owning task — bounded by `RETIRE_QUIESCE_MAX`; `update(auth, NULL, NULL)` (CLEAR — surrenders the slot); drop the ENGINE (its own runtime then shuts down on a dedicated thread, §3.3 "Threading"; `clearState`/`init` wait for that). The trampoline (`ctx`) is NEVER freed — it lives for the process (§3.3): a verb the wallet still calls after the clear returns `RETIRED` instead of touching freed memory (the header's superseded-backing lifetime rule, whose clause (c) — "no SDK thread can still be inside a verb" — no quiesce can prove; the security angle's HIGH) | after it, `phase == NotRegistered`; a wallet still configured `required(hostDialer)` reads `Unavailable` (E1 of the crossing spec at its next open); a later `init` registers the same process-lifetime trampoline afresh (first-wins is free again). **What persists across a second registration in one process:** the tokio runtime and its thread-default dispatcher (the `OnceLock`; never rebuilt), the trampoline and its vtable (the same object; the wallet copies the vtable at each `register`), the lifecycle generation (keeps counting — never resets, so no op id from the first life can be mistaken for the second's), and an EMPTY op table (the drain); the auth token is overwritten by the new mint (CLEAR invalidated the old one); the descriptor is sent again at readiness 0. The header's quiesce obligation is the plugin's here |

The plugin registers an `AppLifecycleListener` of its own in Dart (through
`WidgetsBinding`) that forwards `paused` / `resumed` to two Rust verbs,
`onPaused()` / `onResumed()` — the lifecycle mechanism (§3.4) lives in Rust
so a host that never subscribes to the status stream still gets correct
behaviour. This is a stated EXCEPTION to the UI package's rule that its
`appLifecycleProvider` is THE lifecycle seam (§0 A24): the plugin cannot
depend on `zec_wallet_ui` (dependency direction — it sits beside
`zec_wallet`, and a host with no UI package still needs the transport to go
quiet before the OS acts), and its duty is a transport obligation, not a
stream-pause convenience; the host's own streams keep following the
provider's rule. Desktop never delivers `paused` (the SDK's own rule: `hidden`
is not a pause), so a desktop client stays warm.

### 3.2 Outbound, plugin → wallet — the three verbs, resolved by name (D8)

`Verbs { register, update, notify }` resolved once at `init` through
`VerbResolver`: `dlopen("libzec_wallet.so", RTLD_NOW | RTLD_NOLOAD)` +
`dlsym` on Android/Linux; on Apple, a walk of the loaded mach-o images
(`_dyld_image_count` / `_dyld_get_image_name`) for the wallet's image — the
`zec_wallet.framework` binary when the wallet is a framework (Relim's embed,
AND — MEASURED S284, `fr5-phase-1.md` §0.1 F1 — the SDK's own example: its
Podfiles say `use_frameworks!`, the pod `-force_load`s `libzec_wallet.a` INTO
`zec_wallet.framework`, and `otool -L Runner` shows
`@rpath/zec_wallet.framework/zec_wallet`; this spec's first wording put the
example in the static case from the podspec's flag alone), the main
executable when a host links its pods statically (`use_frameworks!
:linkage => :static`, or no `use_frameworks!`) — reopened by its resolved path with
`RTLD_NOLOAD`, then `dlsym`; on every unix the resolved address is verified
by `dladdr` to belong to that image (Relim's `wallet_registrar` anti-
interposition check — a preloaded shadow never wins); `GetModuleHandle("zec_wallet.dll")`
+ `GetProcAddress` on Windows. **The Apple match predicate, named** (the
security angle's MEDIUM — a name-shaped predicate has nothing to match in the
static case, where the image is `Runner.app/Runner`): the FRAMEWORK case
matches an in-bundle image whose path ends in `zec_wallet.framework/zec_wallet`
(Relim's predicate, `wallet_registrar.rs:251-345` — `framework_path_shape_ok` at `:339-345`); the STATIC case resolves
through `RTLD_MAIN_ONLY` and accepts the symbol only if `dladdr`'s
`dli_fname` equals `_dyld_get_image_name(0)`, the main executable. The
framework case is tried first — and it is the case the SDK's example
exercises (S284); both matching is ambiguous and fails closed
(Relim's rule); the plan's stage 2 probed the example app's actual link shape
(`otool -L Runner`; the framework binary exports the 14 `frb_*` and the
wallet's verbs) before any code. A miss is
`walletNotLoaded` — the host called
`init` before `RustLib.init()` (the ordering guarantee, D6): the plugin never
loads the wallet library itself (loading it would change who owns its
lifetime). The resolved pointers are held for the process (the wallet
library never unloads; the seed port's precedent).

A non-zero return from any verb is FATAL to that call, never retried (the
seed port's rule, FR-15b; Relim's registrant says the same).

### 3.3 Outbound, wallet → plugin — the vtable (the header's `zw_net_dialer_v1`), and the error table

The plugin installs ONE `zw_net_dialer_v1` whose `ctx` is the plugin's
`Trampoline`, alive for the life of the PROCESS (a `OnceLock`; never freed)
— Relim's shape (`trampoline.rs:109-112`), for two reasons: a rebuild
(bridges changed, a resume) must not pass through a moment with no dialer
registered, and the header's lifetime rule makes freeing `ctx` on the clear
edge a use-after-free (a verb call already in flight is not fenced by the
wallet's generation check). The backing engine swaps behind the trampoline;
a swap bumps the lifecycle generation. **Every state the verbs consult lives
under ONE named lock, `state: Mutex<PluginState>`, whose fields are `phase`,
the lifecycle generation and the op table.** Every "under the lock" and
"under the op lock" below means THIS mutex. Per-stream or per-op locks are
NOT permitted — an implementer's natural throughput optimization would
reopen the race the pause, rebuild and close rules close (the reviewer's
finding): the serialized region is a state check plus a ≤ 16 KiB copy, and
the sync stream is one stream.

- **`dial(ctx, host, host_len, port, key, key_len, op_id, sdk_ctx, complete)`.**
  Validate `host_len ≤ ZW_HOST_NAME_MAX_BYTES` and UTF-8, `key_len ≤
  ZW_ISOLATION_KEY_MAX_BYTES` and UTF-8 (a violation is `REFUSED`
  synchronously — a protocol violation on the wallet's side, never a
  fallback; P18); **`key_len == 0` means NO key: the pointer may be NULL and
  is never dereferenced** (the SDK passes `(null, 0)` for an unkeyed dial,
  `net_dialer_cabi.rs:1231-1235`; `slice::from_raw_parts(null, 0)` is UB and
  an abort, not an unwind — the crypto angle's MEDIUM), and the engine is
  called with `None`, which joins `dialer-tor`'s per-dialer unkeyed group
  rather than a keyed slot for `""`; copy both (the SDK's buffers are valid
  for the CALL only).
  Then, UNDER THE LOCK (the same one `onPaused`, a rebuild and `dispose`
  take): if `phase` is `Suspended`, `Bootstrapping`, `Failed` or
  `NotRegistered`, or the engine's `readiness()` is not `Ready` → return
  `NOT_READY` / `RETIRED` SYNCHRONOUSLY, no completion (the header's
  READINESS rule; P20); else record the op under the CURRENT generation.
  Checking and recording in one critical section closes the accept window
  the security angle found (a dial arriving between a state flip and its
  push): a dial is either recorded before the flip, on its generation, or
  refused after it — never accepted by a plugin that is closing. Outside the
  lock, spawn `engine.connect(host, port, key.as_deref())` on the plugin's
  runtime; complete EXACTLY ONCE with `OK` + a non-zero stream handle, or
  with the mapped code. A completion whose op generation is older than the
  current one reports `RETIRED` whatever the engine said (P7).
- **`read` / `write` — the plugin never hands an SDK pointer to arti** (the
  security angle's HIGH: a cancel at an await point does not JOIN a task
  mid-`read(slice)`, and a `RETIRED` delivered from the closing thread while
  that task still writes is a write into a buffer the SDK has just freed).
  `write`: at accept, under the op lock, copy the SDK's bytes (≤ `len`) into
  a plugin-owned staging buffer — the SDK buffer is not touched again; the
  arti write proceeds from the copy; complete with `n` consumed. `read`: the
  owning task reads from the arti stream into a plugin-owned staging buffer
  (≤ `cap`); then, under `state`, if the op is still `Pending` — which IMPLIES
  the current generation, because a rebuild or a dispose marks every op of
  the superseded generation `Closed` under `state` before the engine is
  dropped (§3.4) — copies into the SDK's buffer and completes with `n` (`0 ≤
  n ≤ cap`; `0` = EOF); if `Closed`, drops the data and completes `RETIRED`.
  So a stale-generation read can never complete `OK` with bytes, and the
  generation is not a second check on the success path. The
  copy-and-complete is ONE critical section that `close` also takes, so
  "never touches an SDK buffer after `close` returns" holds by construction. A completion for an
  io op is delivered ONLY by the task that owns the op; a drop guard completes
  `RETIRED` if the task ends without completing. One extra 16 KiB copy per
  direction on the plugin side (§7). One outstanding read and one outstanding
  write per stream is the SDK's discipline; the plugin does not assume it — a
  second read on a stream with one outstanding is `REFUSED` synchronously.
- **`close(ctx, stream)`.** Idempotent; marks every outstanding op on the
  stream `Closed` under the op lock and returns; each op then completes
  exactly once with `RETIRED` — from its owning task, never from the closing
  thread (the op record is a one-way `Pending → Completed | Closed`, first
  transition wins — P17). After `close` returns no SDK buffer for that stream
  is touched again (the header's rule; P17's canary).
- **Mid-stream failures — the io codes** (the security angle's LOW; Relim's
  `io_code`, `trampoline.rs:563-589`, is the precedent). A `read`/`write` that
  fails inside arti completes with a code graded FIRST by the op's state
  under `state` — a superseded op is already `Closed`, so → `RETIRED` (a
  retired stream's `BrokenPipe` graded against a fresh backing would read as
  a reachability failure) — then by
  the error: ~~a timeout → `TIMEOUT`;~~ EOF → `OK` with `n = 0`; anything else
  — a timeout INCLUDED — → `REFUSED` ("this stream will not carry more").
  *(Corrected S305, the security angle on C3b: an exit can end a stream with
  RELAY_END reason TIMEOUT, which arti surfaces as an io `TimedOut`, so a
  mid-stream timeout is exit-controlled and stays off the fallback codes.)* None of these reaches the
  wallet's fallback arm: a stream fault is an `io::Error` inside `HostStream`
  (`net_dialer_cabi.rs`), and the sync's REDIAL is a new `dial` that meets
  the readiness gate and the lifecycle rules of §3.4. `read`/`write` on a
  live stream stay accepted while `Suspended` (a sync in flight may finish
  where the OS lets it, §1.3); only NEW dials are refused.
- **Threading.** The verbs run on SDK threads and only enqueue; every
  completion is called from the plugin's tokio threads and only calls the
  SDK's completion fn (which stores and wakes — the SDK never re-enters the
  plugin from inside it, by the crossing's D4). The plugin's runtime is its
  OWN — a tokio multi-thread runtime of `PLUGIN_RUNTIME_WORKERS` workers built
  at `init` and held in a `OnceLock` for the process (Relim's `RUNTIME` shape,
  owned rather than borrowed) — and it never runs on an SDK thread. Each arti
  CLIENT is a `dialer-tor` `OwnedDialer` (§12): it runs on a runtime of its
  own, which arti binds at mint and spawns all its background tasks on, and
  every call the plugin makes runs there too. Retiring a client starts its
  shutdown (`TorEngine::retire`), bounded by `ENGINE_SHUTDOWN_MAX`, which ends
  every task arti started and so its last state write; the crate's
  `ClientLedger`, which the plugin holds, counts the client until that
  shutdown has finished (§4's `clearState` row). The client's threads carry
  the plugin's log dispatcher. *(Plan §5 of the audit fixes first built this
  inside the plugin as `engine::ClientRuntime`; §12 moved it into the crate
  before the 0.0.1 publish, founder 2026-10-07.)*

**The error table — `dialer_tor::TorDialError` → `ZW_DIAL_*` (D11; P9).**
The rule behind it (the crypto angle's HIGH): **codes 2 and 3 — the only two
`Preferred` follows to clearnet — are reserved for failures the DEVICE, the
path to Tor, or the Tor network itself produces; a failure the DESTINATION or
an EXIT can produce at will never reaches them.** **NARROWED S284 (the
security angle on C0, at arti 0.45.0's sources): the first wording said "a
relay or the destination". The split closes the DESTINATION's lever entirely
— arti's own BEGIN timeout is `ExitTimeout`/`RemoteNetworkTimeout`, and every
END reason lands in an exit-or-remote kind — but a relay ON THE PATH can
still induce the two codes: `LocalNetworkError`'s only producer is an IO
error on an open channel to the GUARD (tor-proto `ChanIoErr`), and
`TorNetworkTimeout`'s are tor-circmgr's circuit/request timeouts. So a guard,
or an on-path censor that resets the guard flow after the handshake, can make
a `Preferred` wallet fall back — an adversary who already sees the device's
address, and who thereby learns the DESTINATION it talks to. FOLDED the same
hour (the crypto angle's HIGH, at the same sources): `LocalNetworkError` maps
4 `REFUSED` — its producer is the guard channel, and a device with NO ROUTE
produces `TorAccessFailed` instead, so code 2 had exactly the wrong
producer; `UNREACHABLE` now has ONE source, the plugin's own device-offline
derivation (the table's first row; C3 picks it). What remains
third-party-inducible is code 3, `TorNetworkTimeout` — ADR-0546's letter,
RE-CONFIRMED by the founder the same afternoon ("keep it", S284 ~17:30):
`Preferred` means connectivity first, `Required` is the privacy-first
setting, and the plugin maps code 3 exactly as the table says.**
`dialer-tor` today FOLDS arti's kinds into two
coarse variants (`error.rs:159-162`: `ExitTimeout | RemoteNetworkTimeout |
TorNetworkTimeout` → `Timeout`; `TorAccessFailed | LocalNetworkError` →
`NoNetwork`), so a lightwalletd operator — or anyone who can blackhole it for
Tor exits — could stall answers on Tor and move every `Preferred` wallet to a
clearnet broadcast from its real address. The split is the second thing this
spec asks of `dialer-tor` (§3.6); until it lands the folded variants map
FAIL-CLOSED.

| `TorDialError` (after the §3.6 split) | code | why |
|---|---|---|
| `NotBootstrapped { .. }` | 1 `NOT_READY` | a bootstrap in progress; the SDK waits (`Preferred` too, ADR-0546) |
| `BootstrapFailed { .. }` | 1 `NOT_READY` | the plugin's own loop owns the retry; the status carries the blockage. Never `UNREACHABLE`: a censored bootstrap under `Preferred` must render, not leak (S290: the switch ADR-0553 ruled for arrives by the DESCRIPTOR, not this code — the `Failed` phase pushes `health = FAILED`, the SDK's gate refuses it `TransportFailed`, and a `Preferred` wallet leaves a minute after the declaration, visibly; the code stays 1, `Required` never leaves) |
| — (no `TorDialError`): the plugin's OWN device-offline derivation — arti's bootstrap blockage reads `Offline` while a dial is refused, or the OS reachability API says no route (C3 decides which, one source) | 2 `UNREACHABLE` | the DEVICE has no route — the one genuinely local reachability failure; `Preferred` falls back visibly, `Required` stalls. **S284 (the C0 crypto angle's HIGH): no arti error kind may produce this code — see the next row** |
| `LocalNetworkError` | 4 `REFUSED` | **S284, was 2 `UNREACHABLE`:** in arti 0.45.0 this kind is produced by an IO error on an already-open GUARD channel (tor-proto `ChanIoErr`, through tor-circmgr's `Protocol` wrap) — the guard, or an on-path censor resetting the flow after the handshake — while a device with NO ROUTE produces `TorAccessFailed`. Code 2 was therefore reachable by the adversary Tor is deployed against and unreachable by the case it was reserved for; fail-closed here, and "offline" is derived where it is true (the row above) |
| `TorNetworkTimeout` | 3 `TIMEOUT` | a circuit BUILD timed out inside the Tor network (tor-circmgr's `CircTimeout` / `RequestTimeout`): a relay on the path can cause it, a stalling exit contributes (arti retries the build over fresh paths, which blunts any one exit), the destination cannot. **The one third-party-inducible fallback left — RULED by the founder, S284 ~17:30, "keep it": it stays at ADR-0546's letter. A `Preferred` wallet whose circuits keep timing out falls back visibly ("fell back, using a direct connection") and keeps syncing; a censor on the path can provoke that and learn the destination; connectivity over privacy is the policy's meaning, and `Required` is the setting for the other choice** |
| `TorAccessFailed` | 4 `REFUSED` | "the chosen relay or bridge is not working" (arti): a guard down is transient — arti marks it and tries the next; a persistent outage regresses `ready_for_traffic()` and surfaces as readiness < 100 (`Bootstrapping`), never as a fallback |
| `ExitTimeout`, `RemoteNetworkTimeout` | 4 `REFUSED` | the exit or the far end stalled — "trying later, or on a different circuit, might help" (arti); a destination-controlled signal must not decide clearnet |
| `RefusedByExitPolicy`, `RefusedByHost`, `HostNotFound`, `OnionUnsupported`, `ForbiddenTarget`, `BadTarget { .. }` | 4 `REFUSED` | the transport (or the far end) will not carry THIS request; no fallback |
| `Setup { .. }`, `Tor { kind }` (the `#[non_exhaustive]` tail) | 4 `REFUSED` | an arti condition the plugin cannot name — fail-closed: `Preferred` waits (`Unavailable`), the sync retries on its next tick, the plugin logs the class |
| — (op generation stale) | 5 `RETIRED` | a rebuild or a dispose intervened (§3.4) |
| INTERIM, before the split: the folded `Timeout` and `NoNetwork` | 4 `REFUSED` | fail-closed: a `Preferred` wallet on the plugin then never falls back (it behaves as `Required`) rather than fall back on a signal it cannot attribute; P9 pins the interim AND the final table |

Consequence, stated for the founder: under censorship a `Preferred` wallet
on this plugin WAITS (readiness < 100 with the blockage rendered) rather than
leaks (S290: for the length of the bootstrap — once `BOOTSTRAP_DEADLINE` ends
in the `Failed` phase and `health = FAILED` is pushed, a `Preferred` wallet
leaves a minute after the declaration, the founder's ADR-0553 "switch";
`Required` never) — stricter than ADR-0546's letter for a guard that fails after
bootstrap, by the rule above; the codes are the registrant's to assign
(ADR-0544 D5). Relim's registrant maps the same folded variants today (H-12);
the `dialer-tor` split serves both.

The mapping is ONE `match` in `trampoline.rs` *(built at C3a in its own module,
`dial_codes.rs`, which the trampoline calls — S286)* — with the wildcard arm that
`#[non_exhaustive]` FORCES on a downstream crate (`dialer-tor`'s `error.rs:56`;
its own test says the compiler polices the set "in here" only — the security
angle's MEDIUM against this spec's first wording). So the compiler cannot
catch an upstream variant here; P9 does instead: it reads `TorDialError`'s
DECLARATION from the locked `dialer-tor` source (the pattern P24 uses for
`BlockageKind`) and fails when a variant exists that the table does not name
— a new upstream variant is a red test, never a silent `REFUSED`; the runtime
tail stays `REFUSED` (fail-closed) until the table names it.

### 3.4 The lifecycle and readiness mechanism (D5; the numbers are §7's named constants)

**The descriptor the plugin pushes.** `readiness` = `READINESS_READY` iff
`engine.readiness()` is `Ready`; else `min(READINESS_CEILING_WHILE_BOOTSTRAPPING,
floor(progress × 100))` with a non-finite `progress` mapped to 0 (P4);
`SUSPENDED` while paused. `name`, `isolation`, `exposure` never change.
`health` (ABI v3, ADR-0549 — the founder's FR-30 (b) ruling; BUILT at C1,
S285): `STARTING` while bootstrapping or suspended, `READY` at 100, `FAILED`
in the `Failed` phase — pushed with the readiness the plugin MEASURED, never a
floored one, and the wallet then renders `Unavailable` instead of a bootstrap
that never ends (E21). `health` never permits a dial: the gate is still
`readiness == 100`, and `FAILED` only ever forbids one.

**The watcher** (one per engine generation; Relim's, copied with its WHYs —
§1.4): polls the engine's readiness every `READINESS_POLL` until
`SLOW_AFTER`, then every `READINESS_POLL_SLOW`, jittered UP by at most one
part in `JITTER_SHARE` on the device's own clock; a HIGHER value is pushed at
once (a wallet that can dial must be told now), a LOWER one only after
`DOWNWARD_CONFIRMATIONS` consecutive polls agree (arti's readiness flaps, and
telling the wallet its path died abandons a sync in flight); the loop ends
with its generation. The push is `notify(auth, descriptor, retire = 0)`.

**The bootstrap loop.** `init` (and every rebuild) spawns: `engine.bootstrap()`
under `BOOTSTRAP_DEADLINE`; on `Ok` the watcher carries readiness to 100; on
`Err` or the deadline → `phase = Failed { class, blockage }`, readiness stays
< 100 (the wallet's `Required` reads `Bootstrapping`, honest — the gate holds),
and the plugin waits `backoff` (`BOOTSTRAP_RETRY_BASE` × 2ⁿ, capped at
`BOOTSTRAP_RETRY_CAP`, jittered up, own clock; a success HALVES n rather than
zeroing it — the decay rule) before the next attempt. `retryBootstrap()` cuts
the wait and leaves n as it is. No attempt starts while `Suspended`.

**The clock.** Every pause interval is measured on a clock that COUNTS
device sleep — `CLOCK_BOOTTIME` on Android/Linux,
`mach_continuous_time` on iOS/macOS, `GetTickCount64` on Windows (`PAUSE_CLOCK`,
§7) — never Rust's `Instant` (`CLOCK_UPTIME_RAW` on Apple, `CLOCK_MONOTONIC`
on Linux: both STOP while the device sleeps, so a forty-minute deep sleep
would read as two seconds, take the short branch, and re-push a readiness of
100 over dead channels — the security angle's HIGH; D-10 runs unplugged to
catch it, because a device under adb or a USB cable does not deep-sleep).

**Pause.** `onPaused()`: (1) UNDER THE LOCK `dial` records under, flip
`phase = Suspended` — atomic with the dial path, so a dial is either recorded
before the flip, on its generation, or refused `NOT_READY` after it (P20; the
accept window the security angle found between a flip and its push does not
exist); (2) push `SUSPENDED` (retire = 0: streams in flight may finish where
the OS lets them, §1.3 — their reads and writes stay accepted, §3.3); (3)
`engine.set_dormant(soft)` (arti: "background tasks are suspended … attempts
to use the client will wake it back up again" — it stops periodic directory
work, NOT sockets, §9.3; battery, and no traffic pattern from a backgrounded
wallet; readiness 0 is what stops dials); (4) record the instant on
`PAUSE_CLOCK`. **A pause does NOT bump the generation** — only a rebuild and
`dispose` do — so an op in flight across a short pause completes on its
merits (P6).

**Resume.** `onResumed()`: elapsed = now − paused-at on `PAUSE_CLOCK`. If ≥
`REBUILD_AFTER_PAUSE` (the process was plausibly suspended — its sockets are
gone on iOS, §9; on Android the same rule costs one warm bootstrap and buys
the same certainty) → REBUILD: under `state`, bump the generation, mark every
outstanding op `Closed` and set `phase = Bootstrapping`; then drop the old
engine (its arti streams end; each `Closed` op completes `RETIRED` from its
owning task — P7); mint a new engine over the SAME `state`/`cache` (a
warm bootstrap within 27 h of the last directory refresh, §9.3; cold beyond);
start the bootstrap loop and a new watcher; readiness climbs from 0. Else (a
short pause: a quick app switch) → `set_dormant(normal)`, under the lock
`phase` back to `Ready`/`Bootstrapping` per the LIVE readiness, push it.
Either way the wallet's sync controller re-dials on its next tick against a
client that can carry — the sequence the founder's "never dials a dead
client" asks for.

**Network change.** No verb (a `networkChanged()` hint was cut at the design
review — the founder's S266 rule): a dead channel after a foregrounded
network change costs one failed dial mapped by the table — `LocalNetworkError`
→ `REFUSED` (S284: a dead guard channel is not "no route"; the plugin's own
device-offline derivation is what reads `UNREACHABLE`, and a network CHANGE
is not that either — the device has a new route) or `TorAccessFailed` →
`REFUSED` — and the
sync re-dials; the same residual Relim carries (E13).

**A terminal engine failure after registration** (the mint itself fails —
`Setup`: the cache dir unwritable, arti refusing the built config at
creation; the security angle's MEDIUM) is not a bootstrap failure to back off
on: the plugin `notify(retire = 1)`s, CLEARs the slot and reports
`NotRegistered { class: setup }`, so the wallet's door refuses `hostDialer`
honestly (crossing E1) instead of reading `Bootstrapping 0%` for the life of
the process; the host fixes the cause and calls `init` again (E15).

**Dispose.** `dispose()`: under `state`, bump the generation, mark every
outstanding op `Closed` and set `phase = NotRegistered`; `notify(retire = 1)`
(every in-flight op fails typed on the wallet's side); drain the plugin's own
outstanding ops — each completes `RETIRED` from its owning task — bounded by
`RETIRE_QUIESCE_MAX`;
`update(auth, NULL, NULL)`; drop the ENGINE. The trampoline stays (process
lifetime, §3.3): a verb the wallet still calls returns `RETIRED`. The wallet
library is never unloaded by the plugin.

### 3.5 Would this work with a different engine? A different host runtime?

The trampoline knows `TorEngine`, not arti; a fake engine over loopback TCP
proves every rule in §3.3/§3.4 without the network (§8). The wallet knows a
registrant, not a plugin: Relim's trampoline and this one are two
implementations of the same vtable, and the header does not change (§0 A17).
A different host runtime (libdispatch, a thread pool) would work — the plugin
uses tokio because arti is written over it.

### 3.6 What this spec asks of `dialer-tor` (stage 0 — H-15; since S284 the plan's chunk C0, made locally by the SDK session — ADR-0550)

TWO additions to the crate as it moves, both recorded in
`host-session-complete-integration.md` H-15 (the register Relim reads; since
ADR-0550 the SDK session makes both in `sdk/dialer-tor`, and Relim flips to
that one copy when it merges):

1. **`TorDialer::set_dormant(DormantMode)`**, a passthrough to
   `arti_client::TorClient::set_dormant` (arti's own doc: "This can be used
   to conserve CPU usage if you aren't planning on using the client for a
   while, especially on mobile platforms"), with `arti_client::DormantMode`
   re-exported beside `TorClientConfig`. Read on Relim `main` at `0c252470`:
   `crates/transport-tor/src/dialer.rs` holds its `TorClient` privately and
   exposes no accessor and no dormant verb, so the plugin cannot reach it
   today (§3.4's pause depends on it).
2. **The error split** (§3.3's rule; the crypto angle's HIGH): `TorDialError`
   gains `TorNetworkTimeout` and `ExitTimeout`/`RemoteNetworkTimeout` as
   DISTINCT variants where `Timeout` folds them today (`error.rs:159-162`),
   and `LocalNetworkError` / `TorAccessFailed` as distinct variants where
   `NoNetwork` folds them — so a consumer can reserve its fallback-eligible
   codes for failures the device or the Tor network produces and refuse the
   ones a relay or the destination can produce. Relim's own `classify`
   (H-12) benefits identically.

Everything else the plugin needs already exists there: `with_bridges` /
`from_config` / `tor_config`, `bootstrap`, `readiness`, `connect`, the bridge
bounds and classes, `MAX_ISOLATION_KEYS`. The plugin's `TorEngine` compiles
against both additions; until 2 lands the folded variants map `REFUSED`
(§3.3, interim).

*Draft, 2026-10-07:* a THIRD addition, `OwnedDialer` (a client on a runtime of
its own, with an awaited bounded `shutdown`) and `ClientLedger`, is specified
in §12 and recorded as ADR-0572 (Proposed).

---

## 4. Security

- **The threat model shifts, and the spec says so** (the crypto angle's
  MEDIUM/HIGH). ADR-0545 trusts the HOST because the host already holds the
  seed; it names "non-host code in the same process (an injected or
  third-party library)" as what the token-gated registry defends AGAINST. A
  pub.dev plugin is exactly that category, invited in: a SUPPLY-CHAIN
  registrant — the plugin's own code plus arti's ~150 transitive crates —
  that never sees the seed and wins the slot legitimately. What bounds a
  compromised crate in that graph: TLS is end-to-end inside the wallet
  (below); `Required` never falls back; the SDK's supply-chain gates
  (`deny.toml`, `sdk-audit`, the pins) are the control on the graph itself.
  What it can still do: see destinations, timing and volume; refuse or stall
  dials; and — under `Preferred` — pull the fallback lever selectively,
  because the SDK's isolation keys are SEMANTIC labels in the clear
  (`wallet-sync`, `wallet-send-<hex>`, `wallet-ephemeral-detect-…`,
  `constants.rs:463-484`; ADR-0545 D4 kept them for a trusted host): a crate
  that returns `UNREACHABLE` for `wallet-send-*` only sends every BROADCAST
  clearnet while sync stays on Tor, and the latch says "fell back" without
  saying which traffic. Remedies: `Required` is the policy for a host that
  cannot vet the plugin's graph (the README says so); the §3.3 table removes
  the destination-controlled trigger; and FR-31 asks the SDK to make the
  wire-level key an opaque per-process token (an HMAC of the label under a
  process-random key — the isolation contract needs only equal/distinct),
  which removes the label oracle though not the traffic-shape one (a
  broadcast is one short request, a sync a long stream). Filed in the request
  register; this spec proceeds under the stated residual.
- **Trust boundaries — two seams, two postures.** Toward the WALLET the
  plugin is a registrant the wallet TREATS as a host (ADR-0545's contract)
  and the BOUNDARY is validated: every
  length the SDK passes (`host_len`, `key_len`) is bounded and checked before
  a byte is read; every completion the plugin issues carries `0 ≤ n ≤ cap`
  and a non-zero handle; a synchronous refusal never also completes. Toward
  the NETWORK every byte is hostile and arti parses it — the consensus,
  microdescriptors, bridge descriptors and cells are `dialer-tor`'s and
  arti's, consumed WHOLE (`crypto-rules` Rule Zero); the plugin holds no
  parser of its own. The one host-supplied text, the bridge paste, is bounded
  (`MAX_BRIDGE_CONFIG_BYTES`) and classified by `dialer-tor` before arti sees
  it, and refused before the plugin registers (E6).
- **What the plugin can and cannot do to the wallet.** It sees every
  destination, timing, isolation key and byte volume the wallet dials —
  never content or keys: TLS is the wallet's (rustls, WebPKI roots, SNI = the
  endpoint host — the crossing's T13), and the plugin carries opaque bytes.
  It cannot admit plaintext, change the policy, suppress `fellBack` or
  override fail-closed on not-ready/retired — the descriptor only ever makes
  the wallet MORE restrictive (ADR-0545 D3, the live home of that clause).
- **The second-Tor-client rule** is enforced by the registry, not by
  goodwill: a bare `register` into a taken slot is `-2`, and the plugin's
  `EngineFactory` is never called after a non-zero register (P1, P2).
- **`unsafe` posture.** ONE module, `abi.rs`: the vtable functions (called
  by the wallet), the reads through the SDK's buffer pointers under the
  header's ownership rules, the calls to the resolved verbs, and the
  `dlopen`/`dlsym` (or Apple/Windows) resolution. Every site carries a SAFETY
  comment naming the contract clause it relies on; every vtable entry and
  every verb call runs behind `catch_unwind` and reports a code (the crossing's
  D7 from the other side — a panic in the plugin must not unwind into the
  wallet's thread; P-planted-panic is folded into P17's family). The rest of
  the crate is `deny(unsafe_code)`; `dialer-tor` and the core are
  `forbid(unsafe_code)`.
- **Crypto.** None of ours. arti (Tor's handshakes, ntor, the directory
  signatures) and rustls + `ring` (arti's TLS to guards and the wallet's TLS
  to lightwalletd) are taken whole. **The rustls provider — the real
  invariant** (the crypto angle refuted the "two libraries, two globals"
  reading: on Android the two `.so`s are `RTLD_LOCAL` and separate, but on
  Apple both are static libraries in ONE Mach-O with one process-global
  provider): the wallet's TLS never READS the process default — `GrpcTls::new`
  and the swap securer pass an EXPLICIT `ring` provider (`grpc.rs:927-940`,
  `zec-wallet-swap-near/src/http_client.rs:76-88`) — so whichever library
  installs the default cannot change what the wallet verifies with; the
  plugin's `dialer-tor` installs `ring` only if none is installed. P28 pins
  the wallet half (the crossing's T13 sibling). The auth token the wallet
  mints is an in-process capability (zeroized on drop); the isolation keys
  are the SDK's tokens, copied for the call and dropped with the op.
- **The `compression` feature puts two vendored C decompressors on the path
  that parses directory bytes BEFORE signature verification** (the crypto
  angle's MEDIUM): `tor-dirclient/{xz,zstd}` → `async-compression` →
  `liblzma-sys` and `zstd-sys`, both in Relim's lock. A malformed compressed
  consensus from a hostile cache is a memory-safety surface in the process
  that holds the spend key (liblzma is the xz-utils family). Kept, for the
  reason Relim keeps it — an uncompressed bootstrap is megabytes on metered
  mobile — with the two crates named here, `sdk-audit`'s advisory lane
  covering them once they enter the lock, and D-2 recording the uncompressed
  bootstrap's byte cost once so the founder can reverse this with a number.
- **A THIRD C library enters the process with arti: SQLite** (the C0 crypto
  angle's HIGH and the supply-chain angle's MAJOR, S284). arti's directory
  cache (`tor-dirmgr`) is a `rusqlite` database, unconditionally, and in
  `dialer-tor`'s lock `libsqlite3-sys` resolves WITHOUT `bundled` — the
  platform's sqlite (unpinned, invisible to `cargo audit`/`deny`), into which
  arti writes hostile directory data. The wallet ships its OWN sqlite —
  SQLCipher, 285 `sqlite3_*` symbols in `libzec_wallet.a` — and the two must
  never bind to each other's implementation by accident: `dialer-tor` exposes
  `static-sqlite` (arti's non-additive feature; the FINAL consumer chooses —
  the plugin's Android build needs it, the NDK links no sqlite); C3's
  duplicate-symbol census gains a `sqlite3_*` row over BOTH archives (defined
  and undefined) before the podspec is written — under `use_frameworks!` each
  image keeps its own, under `:linkage => :static` the plugin's references
  would bind to the wallet's force-loaded SQLCipher (a superset that answers
  plain-sqlite calls) or to `-lsqlite3`; and the wallet gains a runtime guard
  that its connection answers `PRAGMA cipher_version` non-empty, so a
  mis-bind on the one library that decides whether the DB is encrypted is
  loud, never silent.
- **The guard state is an identifier — and the files are plaintext at
  rest.** `<tor_dir>/state` names this device's guards for months; copied to
  another device it links them; `<tor_dir>/cache` holds bridge descriptors
  when bridges are used. Both are arti's own on-disk formats, unencrypted —
  unlike the wallet's SQLCipher DB. The tree lives in `tor_dir`, a SIBLING of
  `db_dir` (§2.3 — never inside it: the wallet's wipe sweep would delete it
  under open handles and leave a fresh identifier behind a "successful" wipe,
  the security and crypto angles' finding). AT-REST RESIDUAL, stated: backup
  exclusion of `tor_dir` is the HOST's obligation, which the SDK enforces for
  neither `db_dir` nor `tor_dir` (the example app sets `allowBackup="false"`
  and the iOS key; a pub.dev consumer on Android's default would back
  `guards.json` up to Google). The README and the threat model state it; a
  platform courier for the no-backup directory shared with the SDK's own DB
  is the follow-up (§0 A27). `clearState()` removes the tree; the plugin's
  Dart never reads it (P12, P13).
- **Supply chain.** `dialer-tor` pins `arti-client = "=0.45.0"` with
  `default-features = false` and the feature set `tokio, rustls, compression,
  flowctl-cc, bridge-client`; `aws-lc-rs`/`aws-lc-sys` are already banned in
  `sdk/deny.toml`, and it GAINS the `equix`/`hashx` bans Relim's carries (the
  LGPL carve-out — absent today because unreachable in the SDK's graph; with
  arti in the lock a feature slip must fail by crate NAME, not only by the
  licence allow-list); `sdk-audit` runs on the new lock. The plugin adds arti's ~150
  transitive crates to the SDK's lock — the `supply-chain-auditor` pass at
  stage 4 reads them.

---

## 5. Privacy & metadata

- **What leaves the device.** The wallet's traffic (sync, broadcast, probe,
  swap — TLS to lightwalletd / the provider) now leaves inside Tor circuits
  through this device's guard, PLUS arti's own directory traffic: the
  consensus and microdescriptors at bootstrap and on arti's refresh schedule
  while the client is awake. The lightwalletd server sees a Tor exit, never
  the device (exposure `HIDDEN`); the guard sees the device's address and
  timing, never destinations (Tor's own model). **The local network / ISP
  sees Tor use** (guard connections are recognisable without a pluggable
  transport) — bridges reduce, not remove, that; the README says so.
- **Traffic patterns the plugin creates.** Directory refreshes are arti's
  and happen only while the client is awake; the DORMANT mode on pause stops
  them (§3.4) — no background traffic from a backgrounded wallet, ON THE
  CONDITION that nothing touches the client while paused: arti 0.45.0 flips
  `Soft` back to `Normal` on ANY use (`client.rs:2414-2424`), and `Soft` also
  turns channel padding off, which the guard can observe (the C0 crypto
  angle). The plugin's pause therefore stops the watcher's readiness reads
  too (they are `TorClient` calls), and re-reads only on `resumed`. Readiness
  polls while awake are local reads, no traffic. The bootstrap runs on the user's action
  (`init`, a resume) or the plugin's own backoff clock — never on a relay- or
  server-driven edge (anti-amplification).
- **Isolation.** The SDK's per-purpose keys reach arti's circuit isolation
  verbatim (P18): the sync stream, each broadcast and each swap ride distinct
  circuits, so the exit cannot tie a broadcast to the syncing wallet — the
  property the SDK's key design exists for, now honoured by a transport the
  SDK ships. **A bad exit is left, not reused** (FR-54, ADR-0567): when a
  dial fails at or near the exit (`dialer-tor`'s `circuit_is_suspect` —
  arti's `ErrorKind`, read before classification), `dialer-tor` gives that
  key's group, the unkeyed one included, a fresh token, at most once per
  key per `EXIT_ROTATION_INTERVAL` (60 s), backing off to 600 s; the failed
  dial is not retried, and a dial the plugin's `DIAL_DEADLINE` elapses
  rotates nothing (the future is dropped first). The plugin keeps the
  default (`ExitRotation::default()`); no ABI, dial code or descriptor
  change.
- **Logging.** New §5.4-allowlisted events, FIELDS-FREE except closed values:
  `zec_wallet_tor.registered`, `.refused { rc }`, `.readiness { from, to }`,
  `.phase { phase, class }`, `.paused`, `.resumed { rebuilt: bool }`,
  `.disposed`. NEVER logged: bridge lines (the `BridgeLines` type is not
  `Debug`/`Display` — the mechanism, not care), the data-dir path, guard
  fingerprints, destinations, isolation keys, the auth token, op ids with
  hosts, byte contents, arti's own `Display` strings where they could name a
  target (the plugin logs `class` labels, never `to_string()`). **arti's own
  tracing targets (`arti_*`, `tor_*`) are DROPPED in every shipped build**
  (the security angle's HIGH): `warn` is not a safe band — `tor-guardmgr-0.45.0/src/guard.rs:777,780`
  logs `warn!(guard = ?self.id, …)`, and `GuardId` is a plain `Debug` over
  the relay's ed25519 and RSA identities, outside `safelog`'s `sensitive()`
  gate; with `bridge-client` on, `guard.rs:447-455` prints a redacted bridge
  address by design. The plugin's `class` + `blockage` carry the diagnosis a
  wallet needs. The filter is installed where it cannot lose a race for the
  global subscriber (the crypto angle's LOW: the SDK's own Android debug
  subscriber and two probes already call `set_global_default`, and on Apple
  one Mach-O shares the `tracing` global): the plugin's runtime threads get a
  THREAD-DEFAULT dispatcher (`on_thread_start` → `tracing::dispatcher::set_default`,
  the guard leaked per thread) carrying the plugin's layer — arti's spans
  are emitted from those threads and never reach the global. A
  `cfg(feature = "arti-log")` developer feature, off in every shipped build,
  re-enables arti's targets at `warn` for a developer's own device. P13 pins
  that an `arti_client` / `tor_guardmgr` `warn` event carrying a `guard`
  field never reaches the capture layer.

---

## 6. Error handling & degradation

### 6.1 Enumerated

| # | case | recoverable? | what the user sees (through the wallet's chip unless noted) |
|---|---|---|---|
| E1 | `init` before `RustLib.init()` — the wallet image is not loaded | host bug | `walletNotLoaded` typed; nothing registered; the host's own error UI |
| E2 | ABI mismatch (`-5`) — a plugin built against another header | host rebuild | `abiMismatch`; the plugin never starts arti; a wallet configured `hostDialer` is refused at its door (crossing E1) |
| E3 | The slot is occupied (`-2`) — the host registered its own dialer first | by design | `slotOccupied`; arti never constructed; the FIRST registrant speaks; the plugin's status `NotRegistered { class: not-registered }` |
| E4 | The bootstrap deadline elapses (a censored or dead network) | yes — backoff retry, `retryBootstrap`, bridges | chip `Bootstrapping { percent }` throughout (the gate holds; `required` never falls back; `Preferred` WAITS — ADR-0546) (S290: the chip reads `Unavailable` once `health = FAILED` is pushed — E21 — and under `Preferred` a minute of that declaration is switch-eligible: the wallet leaves for clearnet, `fellBack`, ADR-0553 as built; `required` still never); the plugin's status `Failed { class: bootstrap-deadline, blockage }` for the host to render with the next step ("try bridges") |
| E5 | arti reports a bootstrap failure with a blockage | as E4 | as E4 with `class: bootstrap-failed` |
| E6 | A bridge paste is refused (too long, too many lines, a PT line, unusable) | yes — the host edits it | `bridgesRefused(class)` from `init`/`setBridges`, BEFORE registration / with the running client untouched; the host renders the class's sentence |
| E7 | `torDir` is relative, empty or not creatable — or the host passed its `db_dir` (the plugin cannot tell; the README's rule and P27 carry it) | host bug / disk | `invalidDataDir` typed, nothing registered |
| E8 | A dial arrives while not ready / suspended | — | `NOT_READY` synchronously (P20); the SDK's gate normally prevents the call; chip `Bootstrapping` |
| E9 | A dial fails in arti (the table §3.3) | per row | `Required`: stalled, `Unavailable` / `Bootstrapping`; `Preferred`: falls back visibly ONLY on `TorNetworkTimeout` (and on the plugin's own device-offline derivation, which is not an arti error) after the §3.6 split — `LocalNetworkError` is `REFUSED` since S284 (the C0 crypto angle); before the split, on NOTHING (the folded variants are `REFUSED`, §3.3 interim; D-11 pins both polarities) |
| E10 | Pause → resume within the threshold | yes | a brief `Bootstrapping` while readiness is re-pushed; no rebuild; the sync resumes |
| E11 | Pause → resume past the threshold — measured on `PAUSE_CLOCK`, so a deep sleep counts (suspended on iOS; a long background or Doze on Android) | yes | `Bootstrapping { percent }` for the warm bootstrap (seconds), then `Active`; ops still outstanding on the old engine fail `RETIRED` on the wallet's side and the sync re-dials |
| E12 | Android Doze / background restriction kills the network while the app is paused | yes | nothing is dialed while paused (readiness 0); on resume the rule of E10/E11 applies; the stream that died shows as a stalled sync that recovers |
| E13 | Network change while foregrounded — arti 0.45.0 has no network-change API (§9.3), and the plugin has no hint verb (cut) | yes | one failed dial: `LocalNetworkError` → `REFUSED` (S284 — a dead guard channel, not "no route"; no fallback either side of the split) or `TorAccessFailed` → `REFUSED`; `Required` stalls once and re-dials; `Preferred` waits the same one tick |
| E14 | An `.onion` endpoint configured | host choice | `OnionUnsupported` → `REFUSED` → `Unavailable`; the README states onion services are out |
| E15 | The mint fails AFTER registration — disk full, the cache unwritable, arti refusing the built config at creation (`Setup`) | yes, after the host fixes the cause | TERMINAL for this `init`: the plugin retires and CLEARS the slot (§3.4), throws `engineSetup(class)`, `phase == NotRegistered { class: setup }`; the wallet's door refuses `hostDialer` honestly (crossing E1) — never `Bootstrapping 0%` for the life of the process; the host calls `init` again |
| E16 | A panic inside a vtable entry or a completion | — | a code (`REFUSED` from a verb; the op fails typed); the plugin's state is not half-committed; nothing unwinds into the wallet |
| E17 | `dispose()` with streams open | — | `retire` fails them typed on the wallet's side; the plugin drains its ops within `RETIRE_QUIESCE_MAX` then clears the slot; a later `init` re-registers fresh (first-wins is free again) |
| E18 | Zero connectivity | as today | queued sends stay queued; the sync stalls honestly; the bootstrap loop backs off (own clock) and never spins |
| E19 | The host also forgets `dispose()` and the process exits | — | the OS reclaims; arti's state on disk is consistent by arti's own design (it writes atomically) |
| E20 | A lossy network has poisoned arti's `circuit_timeouts.json` (arti issue #2079, §9.3): every bootstrap ends `Failed { CantReachTor }` while the device has connectivity | yes — the host offers "reset the private path": `dispose()`, `clearState()`, `init()` | `Bootstrapping` on the chip; the plugin's status `Failed { class, blockage: CantReachTor }` repeatedly; the plugin never deletes a state file on its own |
| E21 | The plugin's `Failed` phase, on the chip | — | **NO LONGER A STATED LIMIT — C1 LANDED (S285, 2026-09-17).** The plugin pushes `health = FAILED` (`ZW_HEALTH_FAILED` 2u) with the readiness it MEASURED, and the chip reads `Unavailable` with the transport's own name and a next step — `live_tor_state` reads health AHEAD of the readiness arm (ADR-0549 D3). The bootstrap-that-never-ends Relim filed as FR-30 (b) is gone for both registrants (§0 A21) |
| E22 | The host wipes the wallet while the plugin runs | host order | the plugin's tree is untouched (a sibling of `db_dir`); the wallet is gone; the plugin keeps a registered, idle transport until `dispose()`; the documented order is §2.3's (P27) |
| E23 | A deep sleep (screen off, unplugged, forty minutes) between `paused` and `resumed` | yes | the pause reads forty minutes on `PAUSE_CLOCK` → the REBUILD branch; on a sleep-stopped clock it would read seconds and re-push 100 over dead channels — the failure D-10 exists to catch |

### 6.2 The fail-closed matrix (this plugin's column of the crossing spec's §6.2)

| plugin state | `Off` | `Preferred` | `Required` |
|---|---|---|---|
| not registered (E1–E3, E7) | n/a | REFUSED at the wallet's door (crossing E1) | REFUSED at the door |
| bootstrapping / failed / suspended (readiness < 100) | — | WAITS; `Bootstrapping` (S290: `failed` — `health = FAILED`, at any readiness — is refused at the SDK's gate `TransportFailed`, reads `Unavailable`, and a minute of it switches to clearnet, `FellBack`; ADR-0553 as built) | fail closed; `Bootstrapping` (`Unavailable` once FAILED, E21) |
| ready; `TorNetworkTimeout` on a dial, or the plugin's own device-offline derivation (after the §3.6 split — before it, no row reaches here: the folded variants are `REFUSED`) | — | clearnet, `FellBack` | fail closed; `Unavailable` |
| ready; `REFUSED` (exit/remote timeouts, a failing guard, a dead guard channel — `LocalNetworkError` since S284 — a refusal, the tail) / a stale-generation `RETIRED` | — | no fallback; `Unavailable` | fail closed; `Unavailable` |
| ready | — | dials over Tor; `Active { hostDialer("Tor", supported, hidden) }` | the same |

---

## 7. Performance — and every threshold, named (gate 7)

| constant | value | WHY this value |
|---|---|---|
| `READINESS_POLL` | 1 s | Relim's: a local read, no traffic; one second is the resolution the chip's percent needs |
| `READINESS_POLL_SLOW` | 10 s | Relim's: past the bootstrap budget the transport is no longer "starting"; the loop must not end at ready because the expensive miss is the DROP |
| `SLOW_AFTER` | `= BOOTSTRAP_DEADLINE` | symbolized, never a second literal; measured on the runtime's clock (tokio time), which is right for a watcher that only runs while scheduled — only the PAUSE interval needs `PAUSE_CLOCK` |
| `DOWNWARD_CONFIRMATIONS` | 3 | Relim's: arti's readiness is not monotonic; a drop must persist across two cadences before the wallet is told its path died (it delays the ANNOUNCEMENT, never the refusal — `dial` re-reads the live value) |
| `JITTER_SHARE` | 4 | Relim's: up to one part in four, only ever LENGTHENING, own clock — two installs never pace together |
| `BOOTSTRAP_DEADLINE` | 180 s | Relim's figure (three relay-request budgets) so both products give Tor the same patience; a cold bootstrap over a slow link is a multi-MB directory fetch plus several multi-hop circuits; the CENSOR sets attempt speed so no value is "long enough" — past it the honest state is `Failed` + a retry, not a longer spinner |
| `BOOTSTRAP_RETRY_BASE` | 30 s | the shortest interval after which a network that was merely slow can look different. It was derived as "one SDK dial budget"; since ADR-0552 stage 1b that budget is 25 s (`DIAL_TIMEOUT_SECS`/`ZW_NET_DIALER_DIAL_BUDGET_SECS`), and this stays at 30 — a retry base ABOVE one dial budget keeps the derivation's intent (a full attempt has finished before the next begins), so the number is deliberately no longer symbolized to it |
| `BOOTSTRAP_RETRY_CAP` | 10 min | the ceiling of the doubling: a wallet left open on a censored network retries at most six times an hour; longer would leave a user who fixed their network waiting |
| `REBUILD_AFTER_PAUSE` | 2 s | BELOW the sourced boundary, not on it (the crypto angle's MEDIUM): iOS gives `applicationDidEnterBackground` five seconds and suspends "shortly after" it returns (§9 A1) — a process suspended early has no live sockets, and the two costs are asymmetric: a needless rebuild is one warm bootstrap; a missed one is a `Preferred` fallback caused by the OS (§9.6's first falsifier) — since ADR-0552 that fallback is also a MINUTE of private-path silence away, not one dial, so the rule keeps its conclusion with a minute of slack rather than seconds (S290: a minute in which the path was both silent AND seen failing, counted from the first OBSERVED failure — never shorter, so the slack is at least that). Two seconds is long enough that a quick app switch stays on the short branch. Applied on every platform; D-4/D-9 may raise it with a number, never below the measured suspension |
| `PAUSE_CLOCK` | `CLOCK_BOOTTIME` (Android/Linux) · `mach_continuous_time` (iOS/macOS) · `GetTickCount64` (Windows) | a named SOURCE, not a number: the clock that counts device sleep. Rust's `Instant` is `CLOCK_UPTIME_RAW` on Apple and `CLOCK_MONOTONIC` on Linux, both of which stop during sleep — the security angle's HIGH (§3.4) |
| `READINESS_READY` / `READINESS_CEILING_WHILE_BOOTSTRAPPING` / `READINESS_SUSPENDED` | 100 / 99 / 0 | declared with their WHYs in §2: 100 is the one value the wallet dials at; a bootstrapping client is capped one below it because `as_frac()` can read 1.0 before `ready_for_traffic()`; a pause pushes 0, never the last value. A NaN fraction maps to 0 (P4) |
| `RETIRE_QUIESCE_MAX` | 5 s | the header's "bounded delay" for a superseded backing; an arti stream that has not completed after a retire and five seconds is a dead socket the OS will reap |
| `PLUGIN_RUNTIME_WORKERS` | 2 | arti's work is I/O-bound; one worker would serialize a bootstrap behind stream I/O, more than two buys nothing on a phone and costs idle threads the OS must schedule |
| `ENGINE_SHUTDOWN_MAX` | 10 s | a retired client's own runtime shutdown (§3.3 "Threading"), on a dedicated thread: inside it every arti task ends and the last state write runs; overrun → the client stays counted and stuck, and `clear_state`/`init` answer `ZWT_RC_RESTART_REQUIRED` (-14) until a restart. The Dart wait (`clearStateWaitMax`, 15 s) is longer, so a shutdown that finishes at its bound is waited for |
| `TOR_DIR_MARKER` / `CLEAR_PENDING_MARKER` | `.zec_wallet_tor` / `.zec_wallet_tor_clear_pending` | the first marks a directory the plugin created; `clearState` refuses a directory without it, so a wrong path loses nothing. The second records a `clearState` refused with `stopping` or `restartRequired`; the next `init` removes the state before Tor starts, so a reset the host asked for completes even after a force-quit (plan §5). Written and synced before either refusal is answered; unsaved, the answer is `invalidDataDir` |
| `MAX_BRIDGE_LINES` / `MAX_BRIDGE_LINE_BYTES` / `MAX_BRIDGE_CONFIG_BYTES` | 8 / 256 / 2048 | `dialer-tor`'s, consumed by re-export with their WHYs (Tor's distributors hand out three; obfs4's longest real line is ~180 bytes) |
| `MAX_ISOLATION_KEYS` | 512 | `dialer-tor`'s |

- **Battery.** No timer runs while paused (the watcher sleeps on its
  generation; dormant arti runs no periodic task); no busy polling — every
  wait is a parked task. The bootstrap is the expensive event and runs on the
  user's action or the backoff clock, never on a server edge. The stage 5
  walk records the energy gauge during bootstrap and at idle (D-8).
- **Memory.** arti reads its directory by mmap by default and runs memquota
  by default (§9.3); its resident size for this feature set is UNSOURCED and
  is MEASURED at stage 5 (D-1), not estimated here. The plugin adds one 16 KiB
  staging buffer per direction per stream (the no-touch-after-close rule,
  §3.3) — a second copy beside the SDK's own; bounded, and the price of a
  buffer the wallet can free at any moment. The iOS Network Extension's memory limit is
  NOT a constraint here (§9 A4: the plugin runs in the app process, not an
  extension).
- **Latency.** Tor adds circuit latency to every dial; the SDK's dial timeout
  (`ZW_NET_DIALER_DIAL_BUDGET_SECS`, 25 s since ADR-0552 stage 1b — 30 s when these were
  first tuned), unary/stream bounds and HTTP/2 keepalive were tuned for Relim's
  Tor at stage 5 of FR-29 — the same numbers apply; the plugin's device walk
  grades them again (D-5).
- **Storage.** `<tor_dir>/cache` holds the consensus + microdescriptors
  and `state` the guards and timeouts; their sizes are UNSOURCED and measured
  at stage 5 (D-1).

---

## 8. Testing strategy — the named-test contract (gate map)

Unit tests run the plugin's trampoline over a FAKE ENGINE (`FakeTorEngine`:
a scripted readiness, `connect` over loopback TCP on a separate tokio
runtime, a recorded isolation key) against a RECORDING resolver (P1, P4–P8,
P9–P13, P17–P18, P20–P21) — no network, no arti — in the plugin crate's own
tests. **[S284, ADR-0551: the placement in the next sentence is WITHDRAWN — the
bridge cannot dev-depend on the plugin (§2's dated note); P28 needs no
plugin and sits in C2; P2's real half, P3 and P19 are placed at C3's start
between the two shapes §2 names. The paragraph is kept as the record of the
reasoning about the reset seam, which still holds for whichever shape wins.]**
**The registry-touching rows (P2's real-registry half, P3, P19, P28) live
INSIDE the bridge crate's `net_dialer_cabi.rs` `#[cfg(test)] mod tests`**,
beside the fake host — NOT in a `tests/*.rs` target (the docs angle's
MAJOR against this spec's first wording): the wallet's registry reset seam
is `#[cfg(test)]` inside that module (`net_dialer_cabi.rs:1307-1332`), so
only a test compiled THERE can reset the process-global slot between cases
(the reason the fake host is a `#[path]`-included helper module and "NOT a
cargo test target on purpose", its module doc says), and the bridge crate is
`cdylib` + `staticlib` with no `rlib`, so an integration target could not
link it anyway. The plugin crate therefore adds `rlib` to its `crate-type`
(cargokit ignores the extra output; it picks the cdylib / staticlib by name)
and becomes the bridge's DEV-dependency, which pulls arti into the bridge's
TEST binary only — never into `libzec_wallet.a` (P15 stays the proof). P19 is
the integration: the bridge's REAL `register` / `update` / `notify` handed to
the plugin's resolver as function pointers, the SDK's real `CAbiHostDialer`
dialing through the plugin's vtable into the fake engine's loopback listener
— the fake-host pattern in reverse. No new test target, so
`every_sdk_test_target_is_named_by_a_live_carrier` is untouched. Dart tests cover the
init order and the stream's lifecycle rule. Stage 5 is the device walk
(D-1…D-9). Every row gets a mutant row in `evals/mutants.tsv` citing the line
the watch printed (one mutant per run, base restored, the module re-run).

| # | test | gate | chunk |
|---|---|---|---|
| P1 | `the_plugin_registers_before_it_starts_arti` — the factory's call count is 0 when `register` is called and 1 after it returns `0`; the descriptor at registration reads readiness 0 | 1, 6 | C3 |
| P2 | `an_occupied_slot_stops_the_plugin_without_starting_arti` — the recording resolver's `register` returns `-2`: `init` fails `slotOccupied`, the factory is never called, `phase == NotRegistered { class: not-registered }`; with the REAL registry (P19's harness) a prior fake-host registration stays the one that speaks (ADR-0548 D4) | 1, 3, 6 | C3 |
| P3 | `the_plugins_descriptor_is_tor_supported_hidden_and_the_crossing_accepts_it` — name "Tor", isolation `SUPPORTED`, exposure `HIDDEN`, the name array ZERO-FILLED beyond `name_len` (the header's "initialise the whole struct" rule — the struct is copied by value); the real `register` (P19 harness) returns `0`, never `-6` | 4 | C3 |
| P4 | `readiness_is_100_only_when_arti_is_ready_for_traffic` — `Bootstrapping { progress: 1.0 }` → 99; `Ready` → 100; `progress: 0.42` → 42; `progress: NaN` → 0 | 6, 7 | C3 |
| P5 | `a_readiness_rise_is_pushed_at_once_and_a_drop_needs_confirmations` — a paused tokio clock: 40 → 70 pushes on the next poll; 100 → 60 pushes only after three agreeing polls; the cadence slows after `SLOW_AFTER`; the loop ends with its generation | 6, 7 | C3 |
| P6 | `a_pause_flips_the_phase_atomically_pushes_zero_and_sets_arti_dormant_without_retiring` — `onPaused`: exactly one `notify` with readiness 0 and `retire = 0`, `set_dormant(soft)` once, the generation UNCHANGED, `phase == Suspended`; a dial racing the flip from a second thread is either recorded (before) and completes on its merits, or refused `NOT_READY` (after) — never accepted by a suspended plugin (the security angle's accept window) | 1, 6 | C3 |
| P7 | `a_dial_accepted_before_a_rebuild_completes_retired_whatever_arti_said` — a held `connect` released after a rebuild (a long-pause resume, `setBridges`): the completion carries `RETIRED`, not `OK`/`UNREACHABLE`; the stream handle (if any) is closed by the plugin at once; a held read on an old-engine stream completes `RETIRED` from its OWN task | 1, 6 | C3 |
| P8 | `a_resume_after_a_long_pause_rebuilds_from_the_cache_and_a_short_one_does_not` — the pause interval is read from `PAUSE_CLOCK` (a fake clock advances it forty minutes while an `Instant`-style clock advances two seconds → the REBUILD branch, the security angle's HIGH); paused ≥ `REBUILD_AFTER_PAUSE`: a second factory call over the SAME dirs, a bootstrap, readiness climbing from 0; paused < threshold: no factory call, `set_dormant(normal)`, the live readiness re-pushed | 3, 6 | C3 |
| P9 | `every_tor_error_maps_to_one_frozen_dial_code_and_only_device_or_tor_failures_reach_the_fallback_codes` — the §3.3 table row by row, interim AND final: `TorNetworkTimeout` is the ONLY `TorDialError` that reaches 3, and NO `TorDialError` reaches 2 (code 2 is the plugin's own device-offline derivation, asserted separately); exit/remote timeouts, `LocalNetworkError` (S284) and `TorAccessFailed` are `REFUSED`; the folded `Timeout`/`NoNetwork` (before the §3.6 split) are `REFUSED`; the variant list is read from `TorDialError`'s DECLARATION in the locked `dialer-tor` source (P24's pattern), so an upstream variant the table does not name is a red test. **BUILT at C3a (S286)** as `dial_codes.rs`'s one `match`; *the interim rows are MOOT* (plan §5 D-14: the §3.6 split landed with `dialer-tor` at C0 — the test asserts instead that neither folded name is declared) | 1, 6, 7 | C3 |
| P10 | `the_bootstrap_deadline_reports_failed_keeps_readiness_below_ready_and_backs_off` — a never-ready fake: at `BOOTSTRAP_DEADLINE` `phase == Failed { class: bootstrap-deadline }`, readiness < 100 throughout; the next attempts are spaced `BASE × 2ⁿ` (jittered up, ≤ CAP); a success halves n; `retryBootstrap` cuts the wait and leaves n; no attempt while `Suspended` | 6, 7 | C3 |
| P11 | `bridge_lines_are_refused_typed_before_registration_and_never_echoed` — nine lines, a 257-byte line, an obfs4 line each fail `init` with the class and the resolver's `register` is never called; a valid pair reaches the factory as two parsed lines (`dialer-tor`'s own test pins the built config); the error's `Display` carries no line bytes | 1, 6 | C3 |
| P12 | `arti_state_lives_in_the_tor_dir_privately_and_clear_state_removes_it` — `<tor_dir>/state` and `/cache` exist with mode 0700 after `init` (unix); a relative dir fails `invalidDataDir` before registration; `clearState` after `dispose` removes the tree, is idempotent (a second call succeeds on nothing), and succeeds after the sibling `db_dir` has been wiped; `clearState` while running is `notInitialized` | 1 | C3 |
| P13 | `plugin_events_are_fields_free_and_artis_are_dropped` — the capture-layer allowlist: `from`, `to`, `phase`, `class`, `rc`, `rebuilt` pass; a path, a bridge line, a host name, a key trips; an event on target `tor_guardmgr` at `warn` carrying a `guard` field, emitted from a plugin runtime thread, never reaches the capture layer (the thread-default dispatcher, §5); the bridges type is `!Debug + !Display` (static assertion, `dialer-tor`'s) | 5 | C3 |
| P14 | `the_plugins_abi_mirror_agrees_with_the_wallet_header` — the ABI version (**4** since S292: the header moved at stage S1 for ADR-0553 and this test was RED at 3 until the mirror followed — the 2026-09-20 review's M02, and why `sdk-ci` now runs this workspace's tests; 3 since FR-5 C1; *corrected S286: this row said 2*), the token width, the **four** bounds (the transport-name bound included), the seven `ZW_RC_*`, the six `ZW_DIAL_*`, the `ZW_ISOLATION_*`/`ZW_EXPOSURE_*`/**`ZW_HEALTH_*`** sets — each closed on the header side — and the descriptor's size (52) and offsets (`health` at 48) and the vtable's layout, parsed from `../zec_wallet/rust/include/zec_wallet_net_dialer.h`, equal the plugin's constants and `#[repr(C)]` mirror (BUILT C3a); and `the_plugins_own_header_agrees_with_its_rust` — the `ZWT_*` sets and `zwt_status`'s layout parsed from `include/zec_wallet_tor.h` equal the Rust side; the `ffigen` verify leg proves the Dart bindings match the header | 4, 7 | C3 |
| P15 | `the_wallet_library_carries_no_arti_symbol` — the `sdk-swap-off-guard` recipe gains a second stage over the DEFAULT-feature `libzec_wallet.a` with the crate-anchored needles of §0 A16 (`arti_client`, `dialer_tor`, `tor_proto`, `tor_rtcompat`, `tor_netdir`, `tor_guardmgr`, `tor_cell` — never the bare `arti`/`tor_`, which match OpenSSL today; S284), the needle list asserted non-empty against the archive's own symbol count; MUTATED once (a planted `extern "C" fn arti_client_probe` in the bridge) before it is believed (REVIEW.md §6). **BUILT at FR-5 C2 (S286, 2026-09-18):** stage 2 of `sdk/zec_wallet/rust/tests/swap_near_off_no_symbols.sh`; the listing must exceed 100 000 lines (the base read 630 201, every needle 0). *Corrected S286:* this row first named the mutant `tor_probe`, which NO crate-anchored needle matches (`tor_probe` ≠ `tor_proto`) — the watch would have passed green and proved nothing; the plan's `arti_client_probe` is the mutant that was watched | 4 | C2 |
| P16 | `the_plugin_crate_joins_the_extraction_policy` — no `relim-*` dep, every dep publicly resolvable, `publish = false`, the named-constant scan and the pins-lockstep scan extended to `zec_wallet_tor/rust`; AND `core_and_bridge_have_no_relim_deps`'s sanctioned list (`extraction_policy.rs:88-91`) gains `zec_wallet_tor` as the bridge's one extraction-unit-internal DEV edge (§0 A22 — the existing test goes red without it). *Chunk corrected S286: C2 → C3.* It needs the plugin crate to exist, and `fr5-phase-1.md` §2 C3b re-cut it into the plugin's OWN tests (its own P26 twin and constants scan) because the bridge cannot dev-depend on the plugin (§5 D-7/D-8) | 4, 7 | C3 |
| P17 | `close_during_a_completion_is_exactly_once_and_touches_no_sdk_buffer_after_close` + `a_planted_panic_in_a_vtable_entry_returns_a_code_and_in_a_completion_fails_the_op` — the op record's one-way state machine under a race from a second thread; a read held inside the fake engine and released AFTER `close` returned completes `RETIRED` from its own task and writes NOTHING into the SDK's buffer (canary bytes intact — the security angle's write-after-free); the `catch_unwind` posture at both edges | 1, 6 | C3 |
| P18 | `an_isolation_key_reaches_arti_verbatim_an_absent_one_is_none_and_an_invalid_one_is_refused_typed` — the fake records the key bytes for `wallet-sync` and a `wallet-send-<token>` dial; `(NULL, 0)` reaches the engine as `None` with no pointer read (the crypto angle's MEDIUM); a non-UTF-8 key and a 257-byte key return `REFUSED` synchronously with no completion and no engine call | 1 | C3 |
| P19 | `the_real_registry_over_the_plugins_dialer_round_trips_bytes_over_loopback` — the bridge crate's real `register`/`update`/`notify` as the resolver; `CAbiHostDialer::dial` → the plugin's vtable → the fake engine's loopback listener; a request/response round trip through `HostStream`; readiness 0 at registration is refused by the SDK's gate (`NotReady`, zero plugin dials); readiness 100 pushed → the dial proceeds | 1, 4, 6 | C3 |
| P20 | `a_dial_while_suspended_or_not_ready_is_refused_synchronously` — `NOT_READY` returned from `dial` with no completion and no engine call while `Suspended`, `Bootstrapping`, `Failed`; `RETIRED` after `dispose` | 6 | C3 |
| P21 | `dispose_retires_then_clears_and_completes_every_outstanding_op_once_and_the_trampoline_outlives_the_clear` — with two held ops: `notify(retire=1)` first, both ops complete `RETIRED` exactly once within `RETIRE_QUIESCE_MAX`, then `update(auth, NULL, NULL)`; a `dial` called through the OLD vtable pointer after `dispose` returned answers `RETIRED` (the `ctx` is alive — the security angle's HIGH); a later `init` registers fresh | 1, 6 | C3 |
| P22 | Dart `zec_wallet_tor/test/init_order_test.dart` — `init` before `RustLib.init()` throws `TorPluginError(kind: walletNotLoaded)` (caught by TYPE); a second `init` returns the same status | 2, 6 | C3 |
| P23 | Dart `status_stream_test.dart` — the stream emits on a phase change and on a readiness change, not on an identical push; a status delivered from a non-Dart thread arrives on the isolate's event loop (`NativeCallable.listener`); `dispose` closes the listener and the stream; the host-side pause/resume rule is documented on the provider the example uses (the UI package's `appLifecycleProvider` pattern) | 2, 5 | C3 |
| P24 | `blockage_kinds_are_mirrored_one_to_one_from_the_pinned_arti_source` — reads `BlockageKind`'s declaration from the locked `arti-client` source (Relim's "the table is the upstream declaration" pattern) and asserts the plugin's `Blockage` covers each variant plus `Unknown`. **BUILT at C3a (S286)**: the version comes from the plugin's own lock, the source from cargo's registry checkout of exactly that version | 6, 7 | C3 |
| P25 | Dart `example/test/tor_toggle_test.dart` — the toggle on yields `TorPolicy.required(runtime: hostDialer())` and calls `ZecWalletTor.init` BEFORE the wallet opens; off yields `off` and never calls `init` | 2, 3 | C3 |
| P26 | `every_sdk_lock_package_resolves_from_a_registry` (gate, `extraction_policy.rs`) — every `[[package]]` in `sdk/Cargo.lock` that is not a workspace member carries `source = "registry+https://github.com/rust-lang/crates.io-index"`; MUTATED before it is believed: a local `.cargo/config.toml` `[patch.crates-io] hex = { path = … }` + `cargo update -p hex` must turn it red (§0 A25; REVIEW.md §6). **BUILT at FR-5 C2 (S286):** members are read from `sdk/Cargo.toml` and mapped path → package name; anti-vacuity floors (members non-empty, lock > 100 packages, every member present in the lock); a member acquiring a `source` is also refused. *Corrected S286:* the mutant first named `dialer-tor`, which is its OWN workspace (§5 D-7) and never enters `sdk/Cargo.lock` — a patch of it could not reach the lock this gate reads; `hex` (a real `sdk/` dependency) is what was watched | 4 | C2 |
| P27 | Dart `example/test/wipe_order_test.dart` — the example's wipe flow calls `ZecWalletTor.dispose()` BEFORE `wallet.wipe()` and offers `clearState()` after; `clearState()` after a wipe succeeds; the example never passes the wallet's `db_dir` as `torDir` (E22, §2.3) | 2, 3 | C3 |
| P28 | `the_sdk_tls_never_reads_the_process_default_provider` (the crossing's T13 sibling) — `GrpcTls::new` and the swap securer build their `ClientConfig` from an EXPLICIT provider; with a foreign `CryptoProvider` installed as the process default before the connector is built, the wallet's TLS config still carries `ring`'s (the crypto angle's rustls finding). **BUILT at FR-5 C2 (S286), in two halves.** *Behavioural* — one test of this name beside EACH builder (`zec-wallet-core` `net/grpc.rs` and `zec-wallet-swap-near` `http_client.rs`, not the bridge: the builders' `connector` fields are private to those modules): the foreign default is `ring` with every suite removed; the test asserts its own premise (that default IS in effect), then that the builder does not panic and its config carries `ring`'s full suite list. *Source* — `extraction_policy.rs::the_sdk_tls_stack_has_exactly_two_explicit_builder_sites`, widened by the crypto angle on C1 (MEDIUM): over every SDK member's `src/**` (test modules included), every `ClientConfig::builder*` and `ServerConfig::builder*` constructor read by its full name — only `builder_with_provider` allowed (`builder()` and `builder_with_protocol_versions()` read the process default, `builder_with_details()` adds a custom certificate clock), the client form at exactly the two sites — `dangerous(` nowhere, and the swap builder pinned to WebPKI roots + `ring` + no client auth as T13 pins grpc's. *Widened at the C2 review fold (S286):* the committed scan matched `builder(` only and cut every line at its first `//`, so a `builder_with_protocol_versions` site, or a `builder()` after a `https://` string, passed green; the test-module coverage is what keeps the behavioural half's permanent broken default safe for the rest of its shared binary | 1, 4 | C2 |

**Stage 5 — the device measurement (ADR-0542's consequence; the founder reads
the numbers; the release checklist §B rows):**

| # | on both OSes unless noted | passes when |
|---|---|---|
| D-1 | size: `libzec_wallet_tor.so` per ABI (`--split-per-abi`), the iOS static contribution, the pub tarball; `<tor_dir>` after one bootstrap | recorded in §B |
| D-2 | COLD bootstrap (fresh `tor/`) to readiness 100; once, the bytes an UNCOMPRESSED directory fetch would cost (§4's `compression` decision, for the founder) | recorded; the chip showed `Bootstrapping { percent }` throughout |
| D-3 | WARM bootstrap (second launch) | recorded; seconds, not minutes |
| D-4 | pause 10 s (past the threshold) → resume: the rebuild, time to 100 | no `fellBack` at any point under `preferred`; `required` stalls then recovers |
| D-5 | one sync to tip and one send over Tor under `required(hostDialer)` | the state reads `active(hostDialer("Tor", supported, hidden))`; lightwalletd's peer is an exit |
| D-6 | airplane mode mid-sync → `Unavailable`; network back → recovers | never `fellBack` under `required` |
| D-7 | Android: 30 min screen-off background (Doze) → resume | no `fellBack`; the rebuild or the re-push, whichever the threshold chose, reaches 100 |
| D-8 | the energy gauge during bootstrap and at idle for 10 min | recorded |
| D-9 | iOS: 30 s background with the host holding no background task, then resume | the sequence of E11; no dial from a dead client (the wallet's log shows `NotReady` → `Bootstrapping` → `Active`, never `Unreachable`) |
| D-10 | BOTH: unplugged, screen off, no USB and no adb for 40 min (a device on a cable does not deep-sleep — D-7 under adb would pass vacuously), then resume | the REBUILD branch fired (the plugin's log: `resumed { rebuilt: true }`), never a re-push of 100; no `fellBack` under `preferred` |
| D-11 | BOTH, under `preferred(hostDialer)`: a genuine local-network cut mid-session (airplane mode ON, then OFF) — the one row that asserts the `Preferred` polarity | BEFORE the §3.6 split: `Unavailable`, NEVER `fellBack` (the interim, §3.3); AFTER it: `fellBack` visibly, exactly once, and the latch reads as documented (the reviewer's finding: no other row asserts `Preferred` either way) |

**Acceptance (replaces the request file's):** the example app with the plugin
added and `required(hostDialer)` syncs to tip and sends once over Tor on
Android and iOS; D-4/D-6/D-7/D-9 show no `fellBack`; every P-row green by
name; the numbers in §B.

---

## 9. Multi-platform — the researched section (D4)

Written from `docs/research/2026-09-17-embedded-tor-mobile-constraints.md`
(the sourced survey: every finding there carries a URL and a verbatim quote; a
claim without a primary source is marked UNSOURCED, and this section inherits
those marks). The bracketed references are that file's finding numbers.

### 9.1 What iOS permits (A1–A5)

- **Suspension is the default and it is fast.** `applicationDidEnterBackground`
  gets five seconds; "shortly after that method returns, the system puts your
  app into the suspended state" (Apple, *Extending your app's background
  execution time*, A1); suspension "prevents the process from running any
  code" (Apple DTS, forums thread 685525). `beginBackgroundTask` buys "about
  30 seconds" on current systems, undocumented and changeable (DTS, thread
  85066) — the plugin never requests one: a wallet kept awake to hold a circuit
  is a battery cost with no user action behind it.
- **Sockets do not survive it.** "While the app is suspended the system may
  choose to reclaim resources out from underneath a network socket … after
  which all networking operations on the socket will fail. The only thing you
  can do with the socket at this point is to close it" (TN2277). Apple's own
  recommendation: "close the socket when the app goes into the background, and
  reopen it when it comes back into the foreground"; the current guidance is
  the same — a suspended connection "goes deaf" or is defuncted, and the app
  should "create a new connection" (DTS, thread 757385).
- **No background mode covers a Tor client.** `UIBackgroundModes` has fourteen
  values (audio, bluetooth-central, bluetooth-peripheral, external-accessory,
  fetch, location, nearby-interaction, network-authentication,
  newsstand-content, processing, push-to-talk, remote-notification,
  screen-capture, voip); none describes an app-defined long-lived network
  connection, App Review 2.5.4 restricts each to "their intended purposes", and
  `voip` requires CallKit since iOS 13 (A2). `BGAppRefreshTask` gives up to 30
  s when the system chooses; `BGProcessingTask` runs for minutes only while the
  device is idle and is killed when the user picks it up (A3). Neither keeps a
  circuit alive, and the wallet has no background sync to schedule on them.
- **A Network Extension is not something an SDK can ask a host to ship** (A4):
  the `com.apple.developer.networking.networkextension` entitlement on the App
  ID, a second embedded extension target, a separate process with a limit
  Apple DTS measures at 50 MiB on iOS 15–18 — inside which Orbot iOS runs Tor
  and "iOS kills the Network Extension" (Orbot FAQ), its arti/onionmasq path
  "might crash" (orbot-apple README), and a cached start "may use too much
  memory" (Orbot's `TorManager.swift`; tor issue 40832) — and guideline 5.4
  reserves "Apps offering VPN services" to `NEVPNManager` and organization
  accounts. Onion Browser re-added an in-app Tor for iOS 17 "where Orbot is
  too unreliable because of memory constraints", and it STOPS that Tor when its
  background grace expires and launches a fresh one on resume (its 2.2.0
  changelog: "Tor now completely shuts down on background and a fresh Tor
  launched when the app is resumed"). The guidelines mention neither Tor nor
  proxies (A5, an absence checked 17 Sep). The in-process plugin is not bound
  by the 50 MiB (that limit is per extension provider — A4, C4).

### 9.2 What Android permits (B1–B5)

- **No suspension; network gating instead.** Android does not freeze a
  backgrounded process; it runs until LRU-killed for memory (B4 — partly
  UNSOURCED as a positive statement; the documented cut-offs are the mechanisms
  that follow). Doze "suspends network access" and restores it in maintenance
  windows that grow rarer; App Standby's Rare and Restricted buckets read
  "Disabled" in the network column (Restricted since Android 12, entered after
  8 days without interaction on 13+); Data Saver blocks background data per app
  on metered networks (B1, B4).
- **A foreground service would make a warm client durable, and the plugin
  does NOT run one.** Android 14 requires a declared type; `dataSync`, the
  natural fit, is capped at six hours per 24 on Android 15 (Orbot was killed by
  it — orbot-android issue 1263); `specialUse` needs a free-form justification
  and a demo video reviewed in Play Console; a foreground service cannot be
  started from the background since Android 12; its notification is
  user-visible and, since 13/14, dismissable — and stopping it "stops your
  entire app" (B2, B3). Orbot runs as `systemExempted` because it is a
  `VpnService`; Tor Browser binds an embedded `TorService` from its own process
  with no notification (B5). A wallet plugin has none of those licences: it is
  foreground-only, like the wallet sync it carries.
- **Consequence:** a paused plugin on Android keeps its client (nothing
  suspends it), sets it dormant and pushes readiness 0 so nothing is dialed
  into a network Doze may have cut; the resume rule (§3.4, §9.5) decides
  between a re-push and a rebuild.

### 9.3 What arti 0.45.0 offers (C1–C4)

- **`DormantMode::Soft`** — "Background tasks are suspended, conserving CPU
  usage. Attempts to use the client will wake it back up again"; `set_dormant`
  is for "especially … mobile platforms" (docs.rs 0.45.0, C1). It suspends
  periodic tasks; it does NOT close channels or sockets (an absence in the
  rustdoc), and arti's tracking issue #90 still lists "audit background events"
  and "soft wake-up" unchecked — so the plugin's pause never relies on dormancy
  alone: **readiness 0 is what stops dials.**
- **Persistence, and the warm/cold boundary** (C2) — `cache_dir` holds the
  directory (consensus, microdescriptors, bridge descriptors; read by mmap by
  default) and may be deleted "outside of the control of Arti"; `state_dir`
  holds `guards.json` and `circuit_timeouts.json`, and "guard nodes are
  persistent across multiple process invocations" (tor-guardmgr); bootstrap
  "uses cached information when possible". The live network runs a one-hour
  voting interval with three-hour validity, and clients keep a consensus "for
  up to 24 hours after it is no longer valid" (dir-spec): a cached directory is
  LIVE for 3 h after its `valid-after` and REASONABLY LIVE for 27 h. Within
  that window a rebuild skips the directory fetch (a warm bootstrap = guards +
  one channel + circuits); beyond it the rebuild is a cold fetch. **No arti
  cold/warm timings are published** (UNSOURCED; the adjacent figures are "on
  the order of a second" for runtime creation in ECC's Android SDK, "sub 5
  second" starts for C tor with cached state, "~15 seconds native" for an EF
  prototype's cold bootstrap) — D-2/D-3 measure them.
- **`bootstrap()` is not a repair hook, and there is no network-change API**
  (C3). "If the client has already been bootstrapped, returns immediately with
  success"; the 0.45.0 method list has no reset or reconnect verb. After a
  suspension the client's channels are dead and arti learns it on next use by
  timing out; issue #2079 (open) shows a lossy network driving "all guards
  down" through a poisoned `circuit_timeouts.json`, mitigated in 1.5.0. This is
  WHY the plugin rebuilds after a long pause instead of waking a dormant
  client: a rebuild makes fresh channels; a woken client makes its first dial
  fail on a dead one, and under `Preferred` that failure is a clearnet
  fallback.
- **Size and memory are UNSOURCED** (C4): the Tor Project calls binary size
  "still-uncracked"; memquota is on by default since 1.4.6; no figure exists
  for this feature set. D-1 measures; nothing here is estimated.

### 9.4 Prior art (D1–D2)

- **Zashi / ECC's SDKs** (librustzcash `tor::Client` over arti): one
  long-lived runtime per synchronizer, created lazily; per-request lightwalletd
  connections from `isolated_client()`; `set_dormant(Soft)` on background,
  `Normal` on foreground — never rebuilt; and both SDKs had to ADD per-request
  timeouts and Tor-failure classification because "a transient Tor circuit
  problem needed an app restart to clear" (the swift SDK's changelog, quoted
  at C3 of the research file). That is the
  keep-warm path and its price: it suits a policy that retries, and it is
  exactly the path that hands `Preferred` a reachability failure after a
  suspension.
- **Onion Browser** rebuilds (9.1). **Cake Wallet** (embedded C tor) and
  **Stack Wallet** (arti through Cypher Stack's plugin, `set_dormant` exposed)
  make no Tor call on lifecycle at all (Cake's `paused` handler stops trade
  monitoring only).
- The plugin takes Zashi's dormancy and Onion Browser's rebuild, and adds the
  one thing neither needs and the wallet's contract does: readiness DOWN
  before the OS acts, so the SDK's gate never dials a dead client.

### 9.5 The design, derived (§3.4's rules, justified)

| platform | on `paused` | `resumed` after < `REBUILD_AFTER_PAUSE` | `resumed` after ≥ threshold | why |
|---|---|---|---|---|
| iOS | readiness → 0; lifecycle generation bumped; `set_dormant(Soft)` | `Normal`; the live readiness re-pushed | REBUILD from cache (warm within 27 h of the last directory refresh; cold beyond) | suspension follows within seconds of the 5 s grace and reclaims sockets (A1); no background mode or task keeps a circuit (A2, A3); `bootstrap()` cannot repair a dead channel (C3) |
| Android | the same | the same | the same — one rule, one constant | no suspension, but Doze/Standby/Data Saver cut the network without notice (B1, B4) and a network change kills channels; a needless rebuild costs one warm bootstrap, a dead client costs a `Preferred` fallback (a minute out since ADR-0552, not immediate). A longer Android threshold is a one-constant refinement if D-7 shows the rebuild is felt |
| desktop | `paused` never fires (`hidden` is not a pause — the SDK's rule) | — | — | the client stays warm; a network change is one failed dial (E13) |

- `inactive` (the notification shade, control centre, a system dialog) and
  `hidden` are ignored — only `paused` / `resumed` act (the SDK's standing
  lifecycle rule).
- Deliberately NOT built, each for a sourced reason: a Network Extension (A4);
  a background task or the `fetch` / `processing` modes (A2, A3 — the wallet
  has no background sync); an Android foreground service and its Play
  declaration (B2, B3); a keep-warm without the readiness gate (D1's price).
- Residuals, stated: (i) a lossy network can poison arti's
  `circuit_timeouts.json` (issue #2079 — mitigated upstream, not closed); the
  user-level recovery is `clearState()`, and the plugin does not delete state
  files on its own (a heuristic this spec declines — E20); (ii) a foregrounded
  network change without the host's hint costs one failed dial mapped by the
  table (E13); (iii) the Android threshold refinement above; (iv) the Rare /
  Restricted buckets and Data Saver can cut the network while the app is
  foregrounded — `Required` renders `Unavailable`; `Preferred` sees arti's
  `LocalNetworkError` or `TorAccessFailed`, both `REFUSED` (S284 — neither
  is "no route"), and falls back only if the plugin's own device-offline
  derivation says the device has no route at all (clearnet is cut too, so
  nothing leaves); (v) a rolled-back or corrupted
  `cache_dir` can only serve a consensus arti still verifies against its
  pinned authority keys, within the 3 h live / 27 h reasonably-live window
  (C2); the money consequence is bounded to relay-set staleness and metadata
  — the wallet's own TLS is end-to-end (P28, T13), so a stale exit set can
  observe or drop a broadcast, never alter it, and resubmission over a fresh
  key converges (the crypto angle's verdict 6).

### 9.6 What stage 5 must prove on both OSes

D-2/D-3 (cold and warm bootstrap), D-4 (a 10 s pause → the rebuild, no
`fellBack`), D-6 (airplane mode), D-7 (Android: 30 min of Doze → resume), D-9
(iOS: 30 s in the background with no host background task → resume: `NotReady`
→ `Bootstrapping` → `Active`, never `Unreachable`), D-8 (energy), D-1 (size,
and the cache's size on disk). The two falsifiers the founder reads first: a
chip reading `Active` over a dead client after a resume, or a `fellBack` under
`preferred` whose cause was the OS rather than the network.

---

## 10. Multi-device & sync

No wallet state crosses devices for this feature. arti's `state` (guards) is
a PER-DEVICE identifier by Tor's design and must never be synced, backed up
or copied — a shared guard state links two devices at the network layer. The
`cache` (consensus, microdescriptors) is public data and could be shared, but
is not (one rule, one tree, one exclusion). The transport choice (the plugin
added, bridges supplied) is the HOST's per-device setting. Nothing here
assumes one user = one device.

---

## 11. Self-review checklist

- [x] No `unsafe` in the crypto path; none in core; ONE `allow(unsafe_code)` module in the plugin; arti and rustls whole
- [x] No key material crosses: the token is an in-process capability; the isolation keys are opaque tokens; the seed is nowhere near this crate
- [x] No PII / destinations / bridge lines / paths / keys in logs (§5, P13)
- [x] Works offline: `queued` ≠ error; the bootstrap backs off on its own clock; nothing spins
- [x] Honest degradation: every matrix cell renders; `required` never falls back; `Preferred` falls back only on the two reachability rows (S290: and after a minute of a `health = FAILED` declaration — ADR-0553 as built)
- [x] Accessible: no new UI; the chip's wording rides the existing sync status sheet
- [x] Test vectors: none new (no new crypto)
- [x] i18n: no new user-facing copy in `zec_wallet_ui`; the plugin's strings are class labels and the proper noun "Tor"; the example's toggle label goes in the example's own `l10n/app_en.arb` (its existing framework)
- [ ] Public readability: this spec narrates its own review ("the security angle's HIGH", session numbers, "the founder") as provenance for THIS repository's readers. Whether `docs/specs` ships in the public mirror is the extraction plan's call (release checklist §D); if it does, the whole-SDK review's public-observability pass (`docs/plan/fr5-session-brief.md` §4) scrubs the narration into plain design statements. Not a blocker for `Reviewed`

---

## 12. Spec patch — the OWNED client in `dialer-tor` (2026-10-07)

**Status of this section: Implementing — built, its review fan-out on the
built diff pending** (design-reviewed; founder 2026-10-07: "in v1.0b", and
before the 0.0.1 publish). §3.3 "Threading" and §3.6 point here; §3.3 and §4
now describe the built state. Agreed in contract shape with the Relim session (relim-9e, 2026-10-07:
C1–C8 plus C2', and `shutdown(&self)`), and wanted by the founder for Relim's
v1.0b. ADR-0572 (Proposed) records the decision. Built BEFORE the 0.0.1
publish (founder 2026-10-07: all four packages wait for it), so
`zec_wallet_tor` 0.0.1 ships on `dialer-tor` 0.1.1 and the plugin-local code
of §3.3 is never published.

### 12.1 Design decisions

**The problem.** arti binds the runtime its client is built on and spawns
every background task there; some of those tasks hold the circuit and
directory managers across an `.await` while a directory is pending (arti 0.45
`tor-circmgr` lib.rs:746-765). Dropping the client therefore does not stop
them, and the last state write runs whenever they end. Two consequences:

1. A Tor identity reset (delete `state/`) can be undone by that late write.
   The plugin closed this for itself in plan §5 (`engine::ClientRuntime`, a
   runtime per client, shut down on retire).
2. A host's "Off" does not stop Tor traffic. Relim builds its client through
   `TorDialer::with_bridges` (which ends in `from_config`) on its ONE app-wide
   runtime; Off cancels the
   bootstrap and drops its references, and on a censored network the pending
   loop keeps arti talking until the app exits, and Off→On can meet the old
   client's state lock. The plugin's fix does not reach Relim, and copying it
   would be a duplicate (one source of truth).

A third, smaller one: the plugin's rebuild drops the old client after
`RETIRE_QUIESCE_MAX` and mints the new one without waiting for the old
runtime's shutdown, so two clients can share `state/` for up to
`ENGINE_SHUTDOWN_MAX` (audit plan §6, deferred).

**The decision.** `dialer-tor` owns each client's lifetime: an
`OwnedDialer` built on a dedicated runtime of its own, an awaited and bounded
`shutdown`, and a `ClientLedger` the host reads to know whether a client may
still write. Hosts keep everything on disk (the reset record, the deletion)
and the user-facing answers.

**Alternatives rejected.**

- *Keep it per host.* Relim would copy the plugin's runtime code: two copies of
  a subtle lifetime rule, the class of bug the one-source rule exists for.
- *One process-wide Tor runtime with per-client task scopes.* tokio cannot shut
  down a subset of a runtime's tasks, and arti spawns through its runtime handle
  (`tor-rtcompat` `impls/tokio.rs` `spawn_obj`) with no scope or `JoinSet`
  hook, so a scope cannot end arti's internal loops. A rebuild also needs the
  old client shutting down while the new one starts: two runtimes coexist.
- *Rely on arti: cancel the bootstrap, set dormant.* arti 0.45 `TorClient` has
  no close or shutdown verb, and dormancy does not end the pending-directory
  loop. Only ending the tasks does.
- *`shutdown(self)`, consuming.* Relim has no single owner at Off: the dialer
  sits in its relay client's connection pair, and every in-flight verb holds
  its own `Arc`. A consuming verb would make Off wait for the longest request,
  which breaks honest-off.
- *Wait for the other holders, bounded.* Same objection: Off would be bounded
  by a request, not by the shutdown.

**Tradeoffs.** Each live client costs one worker thread and up to
`OWNED_MAX_BLOCKING_THREADS` blocking threads, plus a short-lived shutdown
thread (normally one client; two during a rebuild). Every async call pays one
task spawn and a `JoinHandle` await on the owned runtime: microseconds against
dials of seconds. A STUCK client (shutdown overran) keeps its threads for the
life of the process; no new client may start then, so the cost is bounded to
one.

### 12.2 Domain types (`sdk/dialer-tor`, new module `owned.rs`)

```rust
/// A Tor client on a runtime of its own. `Clone`: every clone is the same
/// client. Built by `TorDialer::spawn_owned`.
#[derive(Clone)]
pub struct OwnedDialer { inner: Arc<Owned> }

struct Owned {
    /// The client. NEVER cloned off the owned runtime: a task reads it
    /// INSIDE itself, on a worker, and a sync method calls arti while holding
    /// this lock. arti writes its guard state in a `Drop`
    /// (`tor-circmgr` lib.rs:1139, `CircMgrInner::drop` →
    /// `store_persistent_state`), on whatever thread drops the last
    /// reference; a clone made on a host thread could be that last one after
    /// `Stopped` (the security and crypto reviews of this patch). Taken out
    /// by the first `shutdown` and dropped ON THE SHUTDOWN THREAD.
    dialer: Mutex<Option<Arc<TorDialer>>>,
    /// The owned runtime's handle; every async method spawns on it.
    handle: tokio::runtime::Handle,
    /// The runtime itself, with its ledger entry and its shutdown bound
    /// ([`OwnedRuntime`]: a drop guard from the moment the runtime exists);
    /// taken by the first `shutdown` (or the last drop, unawaited).
    runtime: Mutex<Option<OwnedRuntime>>,
    /// Set BEFORE the runtime shutdown starts; read by every method entry and
    /// every stream poll. `SeqCst`, as is the gate counter: "increment, then
    /// read the flag" against "set the flag, then wait for zero" is a Dekker
    /// handshake, which weaker orderings break.
    closed: AtomicBool,
    /// Stream polls in progress (the poll gate, §12.3). Decremented by a drop
    /// guard, so a poll that panics cannot pin it.
    polls_in_flight: AtomicUsize,
    /// The one outcome, fanned out to every `shutdown` caller; published only
    /// after the `Terminal` committed it.
    outcome: tokio::sync::watch::Sender<Option<Shutdown>>,
    /// The shutdown's ONE outcome, made when the client starts (it takes the
    /// runtime's ledger entry then), so every `shutdown` caller holds it.
    /// Settled once, by whoever is first: the work, the watchdog at the
    /// budget, or a caller past its bound. Settling releases or strands the
    /// entry and records the outcome under one lock, and answers the
    /// committed outcome, the winner's or its own.
    terminal: Arc<Terminal>,
    /// The log dispatcher captured at construction (never the dropping
    /// thread's: the plugin's security review H1).
    dispatch: tracing::Dispatch,
}

// Also in `Owned`: `parked: Mutex<Slab<Waker>>` (the latest waker of each
// pending stream poll, woken by `start_shutdown`) and `gate_zero: Condvar`
// (notified by the gate's drop guard when `polls_in_flight` reaches zero
// after `closed`).
//
// THE TEST SEAM (the confirmation review): the lifetime machinery — the
// runtime guard, the ledger, the gate, the watch and the shutdown thread —
// lives in `owned::core`, generic over the client value `C` held in the slot
// and the stream `S` behind the gate, and over the shutdown-thread spawner
// (`fn(..) -> io::Result<JoinHandle<()>>`). `OwnedDialer` is
// `core::Owned<TorDialer>` and `OwnedStream` is `core::Gated<TorStream>`; the
// tests instantiate them with probes (a client that records the thread its
// drop ran on, a `Drop` that panics, a stream holding a `Sleep` and an I/O
// registration of the owned runtime) and a spawner that fails.

/// The owned runtime as a value that cannot be lost: its `Drop` (a
/// `spawn_owned` cancelled mid-mint, a client dropped without `shutdown`)
/// takes the same shutdown-thread path as `shutdown`, unawaited, so a runtime
/// is never dropped on an async thread (tokio panics there,
/// `blocking/shutdown.rs:44-51`) and never left running unreachable.
struct OwnedRuntime { rt: Option<Runtime>, entry: Option<LedgerEntry>, budget: Duration, dispatch: Dispatch }

/// How a shutdown ended. Two outcomes, frozen: a shutdown thread that cannot
/// start is folded into `Overran` on purpose (fail closed), so a later split
/// of the reasons is a new field, never a third outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shutdown {
    /// Every task the client started has ended and the client has been
    /// dropped; it writes nothing more and opens no connection.
    Stopped,
    /// The runtime did not finish inside the budget, or its shutdown thread
    /// could not start, or the shutdown thread panicked (a guard publishes
    /// this): the client may still write for the life of the process. Its
    /// ledger entry is never released (`stuck`).
    Overran,
}

/// Per process, per host: how many clients may still write, and how many
/// are stuck. One `Arc` in the host's composition root (Relim) or the
/// plugin's state (zec_wallet_tor). Two hosts in one process hold two
/// ledgers, so at most one stuck client EACH.
#[derive(Default)]
pub struct ClientLedger { live: AtomicUsize, stuck: AtomicUsize, stopping: AtomicUsize, settled: Notify }
impl ClientLedger {
    pub fn live(&self) -> usize;
    pub fn stuck(&self) -> usize;
    /// Shutdowns started and not yet settled: counted BEFORE the gate
    /// closes, so a caller that sees the client closed sees it counted;
    /// each leaves this count only after its entry is released or stranded
    /// and its outcome published. `spawn_owned` waits for it to reach zero,
    /// bounded (the Relim host review, C1; the security review of the fold).
    pub fn stopping(&self) -> usize;
    /// One entry, released on drop, `strand()`-ed on an overrun. Public so a
    /// host's own engine seam (the plugin's test engines) counts in the SAME
    /// ledger: one counter, never a second one beside it.
    pub fn enter(self: &Arc<Self>) -> LedgerEntry;
}
pub struct LedgerEntry { /* Drop: live -= 1; strand(): stuck += 1, never released */ }

/// Construction options.
pub struct OwnedOptions {
    /// The owned runtime's thread name (a host's own, for its logs).
    pub thread_name: &'static str,
    /// ADR-0567's policy; the default is ON, as for `TorDialer`.
    pub exit_rotation: ExitRotation,
    /// The budget an UNAWAITED shutdown uses (the last drop of a client never
    /// shut down, a cancelled `spawn_owned`). Default
    /// `RECOMMENDED_SHUTDOWN_BUDGET`.
    pub drop_budget: Duration,
}

/// The stream an `OwnedDialer` returns: a `TorStream` behind the poll gate.
pub struct OwnedStream { inner: TorStream, owner: Arc<Owned> }

// TorDialError gains two variants (the enum is #[non_exhaustive], error.rs:83,
// so additive for consumers):
//  - `Closed`: this client was shut down. Class "closed". A LOCAL, retryable
//    fact that says nothing about any relay or destination: a host maps it to
//    its own not-ready error (Relim: `DialError::NotReady`), never to a
//    refusal that marks a relay failed and never to a fallback code. The
//    plugin's ZW_* mapping is its own: NOT_READY, as built (§12.3).
//  - `RestartRequired`: `spawn_owned` refused because `ledger.stuck() > 0`.
//    Class "restart-required". Also local; the host shows its own state.
// Their class strings are published surface, frozen like the others. Sites
// that must change together: `class()` (error.rs:177-195, exhaustive on
// purpose), its test mirror `expected_class` (error.rs:322-343), and the
// plugin's `dial_codes.rs` positive allowlist (it reads error.rs by
// `include_str!`, :77). The plugin pins `dialer-tor = "=0.1.0"` (Cargo.toml
// :43, :69); the plugin's adoption moves both pins to =0.1.1 and the
// allowlist in one change.
```

`OwnedDialer::readiness()` and `set_dormant()` return `Result<_,
TorDialError>` (`Err(Closed)` after a shutdown); `TorDialer`'s own signatures
are unchanged. `dialer-tor`'s `Cargo.toml` tokio features grow from `rt` to
`rt`, `rt-multi-thread`, `sync`, `time` (the one-worker runtime, the watch,
the driver); `Cargo.lock` does not record features, and arti already pulls
tokio with them, so no lock entry changes here, in the plugin's lock, or in
Relim's.

Named constants (gate 7), each with its WHY at its one home in
`dialer-tor`'s `constants.rs`:

| Constant | Value | Why |
|---|---|---|
| `OWNED_WORKER_THREADS` | 1 | the owned runtime only drives arti's tasks and its I/O; the host's own runtime runs the host. Moved from the plugin (`ClientRuntime`) |
| `OWNED_MAX_BLOCKING_THREADS` | 2 | arti asks for no blocking threads; a small cap bounds what a STUCK client can hold. Moved from the plugin |
| `RECOMMENDED_SHUTDOWN_BUDGET` | 10 s | the plugin's `ENGINE_SHUTDOWN_MAX` (plan §5). A normal shutdown is a drop path of milliseconds: arti runs no work on a blocking pool. An overrun therefore means a worker is stuck in a synchronous filesystem call inside a task (the directory manager's SQLite store, or the drop-time state write itself); 10 s is a wide margin over that on a loaded phone, and short enough not to hold a wipe. Not measured on a device yet: the stage-5 row measures the shutdown time and looks for exactly those calls. Covers the gate's wait too (§12.3). A host passes its own; the plugin keeps its constant as the value it passes |

### 12.3 Interface

**Inbound (what a host calls).**

| Verb | Contract |
|---|---|
| `TorDialer::spawn_owned(config, &Arc<ClientLedger>, OwnedOptions) -> Result<OwnedDialer, TorDialError>` (async) | first WAITS until `ledger.stopping() == 0` (every shutdown already started in this ledger has settled; each settles within its running budget, by its watchdog). The wait is bounded by the longest running budget handed off in the ledger plus `SHUTDOWN_WAIT_GRACE` (a watchdog's thread may start late), measured from when the wait began and re-read on every wake, so a longer shutdown handed off DURING the wait extends it rather than being cut short (the confirmation review, H-1); a shutdown still unsettled then is answered `RestartRequired` (fail closed; while `stuck()` is 0 a later attempt may succeed, and `RestartRequired`'s doc says so). Budgets are capped at `MAX_SHUTDOWN_BUDGET` (1 h, crate-private) where a shutdown starts, so no deadline sum overflows. It takes a tokio timer only while a shutdown is in flight, never blocks a thread, and is cancel-safe: a dropped future holds nothing but its `Notify` registration (Relim's note on the fold). It then refuses with `RestartRequired` while `ledger.stuck() > 0` (the one home of that rule; a host need not remember it, nor await an outgoing shutdown itself). Without the wait, `start_shutdown` then an immediate `spawn_owned` saw `stuck() == 0` and minted beside a client that could still overrun: a stuck and a live client together, or two stuck (the Relim host review, C1). A `spawn_owned` that starts BEFORE an earlier client's shutdown is handed off is the host's ordering, not this rule's. Otherwise builds the owned runtime INTO an [`OwnedRuntime`] guard and takes its ledger entry FROM THAT MOMENT, builds `Owned` with an EMPTY dialer slot, then spawns a task ON the owned runtime that runs `TorDialer::from_config(config)` (so arti binds the owned runtime, `PreferredRuntime::current()`) and INSTALLS the dialer into `Owned.dialer` itself, returning only `Result<(), TorDialError>`. The client therefore never crosses to the caller, not even as a `JoinHandle`'s output that a cancelled caller would drop on a host thread (the confirmation review: the same class as the two BLOCKERs). A host that builds with bridges composes the config with the public `tor_config(state, cache, bridges)`, which is what `with_bridges` does. Callable from any runtime, multi-thread or current-thread. **Cancel-safe:** if the caller drops this future mid-mint (Relim's Off aborts bring-ups), the guard's `Drop` takes the shutdown-thread path, so the half-built client's work ends and its entry is released or stranded, never left running unreachable and never a runtime dropped on an async thread. A failure after the runtime exists takes the same path. A runtime that cannot be built is `Setup { LocalResourceExhausted }`, counted nowhere |
| `bootstrap()`, `connect(..)`, `dial(..)` (async) | **C2': each runs ON the owned runtime.** The spawned future captures only `Arc<Owned>` and owned copies of its arguments, and reads the dialer INSIDE the task, on a worker; no reference to the client exists on a host thread (`Owned.dialer`'s rule). The caller awaits an ABORT-ON-DROP wrapper of the `JoinHandle`: if the caller gives up (a connect timeout, a bootstrap deadline, an aborted bring-up), the task is aborted and dropped on the worker, so cancellation behaves as it does for a `TorDialer` polled directly (dialer.rs:231: "a dial that is cancelled or dropped before it fails rotates nothing"), and no orphaned task keeps building circuits. A task found cancelled by the shutdown, or one whose dialer slot is already empty, is `TorDialError::Closed`; a panic inside the task is `Setup { Internal }`, logged. No arti future is polled on the caller's executor, so no timer of the owned runtime can fire after its driver is gone (tokio 1.53.1 `time/sleep.rs:467` PANICS on that: `"timer error: …"`) |
| `readiness()`, `set_dormant(..)`, `dormant_mode()` (sync) | lock `Owned.dialer` and call arti WHILE HOLDING it (never a clone out); an empty slot is `Err(Closed)` |
| `start_shutdown(&self, budget)` (sync) | **The whole shutdown hand-off, synchronous, so no cancellation can strand it** (the security review). The FIRST call: sets `closed` (`SeqCst`; the flag only); takes the dialer and then the runtime out of their slots (lock order: `dialer`, then `runtime`, everywhere; a poisoned lock is recovered with `into_inner`, never a panic, so a panic in `readiness` cannot turn the shutdown into one); hands both to a plain std thread under the construction dispatcher, the WATCHDOG. It starts the work on a second thread: wait for `polls_in_flight` to reach zero on a `Condvar` the gate's drop guard notifies at zero, with the remaining budget as its timeout (no spin, no polling; a wait that outlasts the budget is `Overran`), drop the dialer THERE (so arti's drop-time flush logs through the construction dispatcher, never on a host worker), then `shutdown_timeout(remaining)`; `elapsed >= budget` is `Overran` (`shutdown_timeout` reports nothing; the doubtful case fails closed). The watchdog settles the `Terminal` with the work's outcome, or with `Overran` at the budget if the work has not finished (arti's drop-time write has no bound of its own: a drop blocked in the filesystem is stranded at the budget, with no caller awaiting). Settling `Overran` strands the entry BEFORE the outcome is published. A guard settles `Overran` if a thread panics, so no caller waits forever. A thread that cannot start leaks the runtime (`mem::forget`; dropping it on an async thread panics), strands the entry and publishes `Overran`. The shutdown counts in `ledger.stopping()` from BEFORE the gate closes to its settlement (published first, then released): every caller sees the gate closed only once the shutdown is counted, so a racing second caller cannot return, start a client, and find `stopping() == 0` while the first is still handing off (the security review of the fold, B1). A caller that loses the race to close drops its own count. Only AFTER the hand-off does it wake every parked stream poll (`Owned.parked`, below), each waker isolated by `catch_unwind`: a host waker that panics while woken can neither leave the gate closed with nothing handed off (the client then dropping on a host thread) nor stop the other wakers (the Relim host review, C3). Later calls on any clone do nothing. Legal from any thread, a worker of the owned runtime included (the hand-off needs no await) |
| `shutdown(&self, budget) -> Shutdown` (async) | `start_shutdown`, then `subscribe()` to the watch and `wait_for(Option::is_some)`; whichever settler commits the outcome publishes it with `send_replace` (which, unlike `send`, succeeds with no receiver), so a caller arriving after the outcome still reads it. Every caller, on any clone, gets the same outcome; the await is the only suspension point, so dropping this future loses nothing. The shutdown runs with the FIRST caller's budget, recorded before the gate closes, and every caller's await is bounded by that RUNNING budget `+ SHUTDOWN_WAIT_GRACE` (2 s), never by its own `budget` (the security review of `4353b37ba`: a host that calls `start_shutdown(10 s)` at a switch and awaits `shutdown(1 s)` before a mint would otherwise strand a client that stops at 5 s, a false `RestartRequired` for the process). The grace exists because the watchdog's clock starts only when its thread runs. A caller past that bound settles `Overran` through the client's one `Terminal` and answers what it committed: `Overran` with the entry already stranded, or the outcome another settler committed first, even one not yet published. It never answers an outcome of its own, so no caller reads `Overran` while `stuck()` is zero, and no two callers disagree (the external reviews of `0d8d8bbbe` and `ca6c22418`). Called from a task ON the owned runtime it cannot wait (the shutdown cancels it; detected by `Handle::try_current()` compared by `Handle::id()`, stable in tokio 1.53): it starts the shutdown FIRST, logs an error, and settles `Overran` at once (fail closed), the same in every build. The draft's `debug_assert!` fired before the shutdown started, so a debug build stopped nothing on that misuse, and the release arm could not be tested (the Relim host review, C2) |
| `is_closed()` | the flag |
| last `Drop` of a never-shut-down client | `start_shutdown(OwnedOptions::drop_budget)` (held as `OwnedRuntime.budget`), unawaited; the outcome reaches only the ledger (C4). After a shutdown, nothing |
| `OwnedStream` `poll_read`/`poll_write`/`poll_flush`/`poll_shutdown` | **the poll gate** (memory safety; a host's own latch, such as Relim's retire latch that also covers its non-Tor transports, may stay beside it): increment `polls_in_flight` (`SeqCst`) under a drop guard that decrements it, re-check `closed`; if set, return `io::ErrorKind::NotConnected` without touching arti; else poll the inner `TorStream`, and if it returns `Pending`, keep this poll's waker in `Owned.parked` (one slot per stream AND per direction, read or write, so a split stream's reader and writer never overwrite each other's waker; replaced on each poll; woken outside the lock). `start_shutdown` wakes every parked waker, so a reader parked BEFORE the shutdown re-polls, meets the gate and fails at once, instead of hanging until the host's own timeout or until arti's channels happen to close (the confirmation review). `start_shutdown` sets `closed` before the shutdown thread waits for zero, so no poll is inside arti when the runtime goes. **Drop** of an `OwnedStream`, before, during or after the shutdown, drops the arti stream without polling it: its reads and writes are channel messages to a reactor task on the owned runtime, and the drop only closes those channels (required, and pinned by test). Unconditional, not "only if a test shows a panic", so the guarantee does not depend on arti internals that can change between releases. Host requirement, unchanged from `TorDialer`: poll streams inside a tokio context with timers enabled (arti's per-stream sleeps use the POLLING runtime, `tor-rtcompat` `impls/tokio.rs:216`) |

**Outbound (what the crate needs).** tokio, with the features listed in
§12.2, and arti 0.45 as today. No new crate (ADR-0551's workspace stands).

**Would this work with another host or runtime?** Yes: the owned runtime is
the crate's, so the host's runtime flavour is irrelevant (the plugin runs a
multi-thread runtime; Relim its app-wide one; a test a current-thread one).
`TorDialer` and `from_config` stay as they are for hosts that manage their own
lifetime.

**How each host uses it.**

- *zec_wallet_tor (0.0.1, before its publish).* `ArtiEngine` wraps an `OwnedDialer`;
  `engine::ClientRuntime` and `LiveClients` go, and the plugin's
  `Arc<ClientLedger>` replaces the counter (`strand` moves into the crate).
  Only ONE term of `state_writers_quiet` (trampoline.rs:769-771) moves:
  `live_clients.live() == 0` becomes `ledger.live() == 0`; `rebuilds_in_flight
  == 0` and `engine.is_none()` stay the plugin's own (host concepts). The
  plugin's test engines count through `ledger.enter()` in `Tracked`, which
  stays as that thin wrapper. *As built:* `ArtiEngine::adopt_live` STAYS and
  drops the wrapper's entry (an `OwnedDialer` is counted by the crate, and
  without it the wrapper would count it twice: `r6-double-count`).
  `TorEngine` gains `retire()`, which `ArtiEngine` maps to `start_shutdown(
  ENGINE_SHUTDOWN_MAX)`: the plugin's engines are `Arc<dyn TorEngine>`, so it
  starts the shutdown through the trait, not by owning the `OwnedDialer`. The
  rebuild, after its `RETIRE_QUIESCE_MAX` quiesce, calls `old.retire()` and
  drops it; then EVERY rebuild, one that found no engine too (an earlier
  rebuild's retired or half-built client may still hold `state/`), waits
  until `ledger.live() == 0` AND no other mint is in flight (the plugin's
  `rebuilds_in_flight`, which counts `init`'s mint and every rebuild's, is 1:
  itself) BEFORE it mints, which closes the §6 overlap and makes the check and
  the mint one step, since a later rebuild waits on this one's count (the
  code review of the folds).
  It never mints while a client is still counted: a stuck client, or one
  still counted past `REBUILD_WAIT_MAX` (twice `ENGINE_SHUTDOWN_MAX`), sends
  the plugin down its terminal setup path (`NotRegistered`, class `setup`)
  instead (the reviews of the built diff found the first build minting on a
  timeout and on the no-engine path). `dispose` calls `retire()` after its own quiesce
  and returns within its bound as today; `clear_state` and `init` read the
  ledger exactly as they read the plugin's counter before. A dial cancelled
  by the shutdown (`Closed`) completes `ZW_DIAL_NOT_READY`, a local,
  retryable answer (the next client serves the retry); `RestartRequired`
  maps `REFUSED` (only a mint answers it). The C ABI, the Dart surface and
  every `ZWT_RC_*` are unchanged.
- *Relim (its host-side review of this patch).* Every transition that
  replaces the host's Tor dialer calls `start_shutdown` on the outgoing client
  (Relim: its `move_to` funnel, synchronous under a std mutex, so it starts
  the shutdown and cannot await it); Off calls `start_shutdown` too, so Off
  never falls back to the last-drop path, which in-flight references would
  delay (§12.1's objection to waiting on holders). A new TOR client's mint
  awaits the outgoing shutdown BEFORE it opens the state directory (Relim:
  before the mint in its `apply`). A non-Tor mint (Shadowsocks, direct)
  neither waits nor is refused; how this fits the host's mint-first rollback
  is the host's choice.
  Duress wipe: `start_shutdown` at teardown (which runs before the identity
  shred, and must not delay it), `shutdown` awaited right before the `.tor`
  delete; on `Overran` the host's existing wipe marker stays and the Tor data
  is reported unfinished. Relim keeps its own retire latch (honest-off at the
  swap, synchronously, for every transport).

### 12.4 Security

- **The reset cannot be undone** once `Stopped`: no arti task exists, and no
  reference to the client survives anywhere else. arti 0.45 starts no thread
  of its own and installs no global runtime (no `std::thread::spawn`,
  `spawn_blocking` or `block_in_place` in arti-client or the tor-* crates;
  the state lock and fs-mistrust are synchronous; the crypto audit of this
  patch), so its only writer outside a task is the drop of its last
  reference, and the crate keeps every reference on the owned runtime or the
  shutdown thread (`Owned.dialer`'s rule). One premise was not read from the
  arti sources: that a returned stream (`TorStream`, arti's `DataStream`)
  holds nothing that reaches the client's managers, so that a stream
  outliving `Stopped` and dropped later on a host thread writes no state.
  **Verified live on 2026-10-07** by the network test
  `real_stream_after_shutdown_errors_never_panics` (a real stream to a real
  host, open across `Stopped`: read and write fail, a reader parked in it is
  woken, and `state/` is byte-for-byte unchanged three seconds after the
  stream's drop). Structurally it looks true too (the
  Relim session's reading: tor-proto's stream types have no `Drop` impl and no
  path to the circuit, guard or persistence managers); the test settles it.
  If it ever fails, the fallback is NOT to hand a stream to the shutdown
  thread at its own drop (after `Stopped` that thread has exited): it is that
  `start_shutdown` takes EVERY live inner stream out of its `OwnedStream` (each
  inner stream then lives in a slot `Owned` can reach) and drops them on the
  shutdown thread inside the budget, leaving each `OwnedStream` an empty shell
  that answers `NotConnected`. That is an `OwnedStream` shape change; the
  test passed, so 0.1.1 ships without it, and a failure after an arti upgrade
  brings it back. `Overran` is never reported as
  stopped, and its entry is never released.
- **No panic crosses a host boundary** from the lifetime: no arti future is
  polled off the owned runtime (C2'), no stream poll is inside arti when the
  runtime goes (the gate), no runtime is dropped on an async thread (the
  `OwnedRuntime` guard and the shutdown thread, or a leak). The plugin's
  `catch_unwind` at the C ABI stays.
- **Bounded resources:** a stuck client holds at most `OWNED_WORKER_THREADS +
  OWNED_MAX_BLOCKING_THREADS` threads, and `spawn_owned` itself waits out
  every shutdown in flight and then refuses while `stuck() > 0` (no host has
  to remember either), so repeated overruns cannot
  accumulate threads: at most one stuck client per ledger, one ledger per
  host. A caller that gives up aborts its task (abort-on-drop), so timeouts
  cannot pile up orphaned circuit work.
- **Logs** of the owned runtime and the shutdown thread follow the dispatcher
  captured at construction, never the dropping thread's (the plugin's H1).
- **The crate never touches disk** beyond arti's own files: no deletion, no
  record. A host's reset record follows the plugin's rule (`9aa5bbbd0`):
  answer "will finish later" only after the record is durably saved, and only
  where the removal would run.
- No new dependency, no `unsafe`.

### 12.5 Privacy and metadata

- **Honest-off becomes true:** after `Stopped` the client opens no new Tor
  connection and sends nothing more; sockets it had open are closed during
  the shutdown (the OS sends their FIN or RST). A host's Off can say so.
- **After `Overran`** the client may still talk to the Tor network until the
  process exits. A host must not show "Off" as complete then; its surface for
  that is its own (the plugin answers `restartRequired`; Relim reuses its
  failed state, with no FFI change).
- Logs: the outcome (`stopped` / `overran`) and counts only; never a path, a
  bridge line, an isolation key or a destination (§5's policy, unchanged).

### 12.6 Error handling and degradation

| Case | What happens | The host sees |
|---|---|---|
| owned runtime cannot be built | nothing counted | `Setup { LocalResourceExhausted }` |
| `spawn_owned` while `stuck() > 0` | nothing built | `RestartRequired` |
| `spawn_owned` while a shutdown is in flight (`stopping() > 0`) | waits for it to settle (the longest running budget plus the grace at most), then the row above or a normal start | a new client never runs beside one that may still overrun |
| that wait passes its bound (a watchdog thread never ran) | nothing built | `RestartRequired` (fail closed; retryable while `stuck() == 0`) |
| a longer shutdown handed off during that wait | the bound re-read: the wait extends to it | the later shutdown's own verdict |
| a host budget near `Duration::MAX` | cut to `MAX_SHUTDOWN_BUDGET` (1 h) | no overflow panic |
| `spawn_owned` dropped during that wait | nothing counted, its registration removed | nothing |
| `from_config` fails after the runtime exists | the guard takes the shutdown path; counted until it ends | the `Setup` arti produced |
| `spawn_owned` cancelled mid-mint | the guard takes the shutdown path, unawaited | nothing (the future is gone); only the ledger |
| a method after `start_shutdown` | no runtime touched | `TorDialError::Closed` (`readiness`/`set_dormant` included) |
| a call in flight at the shutdown | its task is cancelled on the worker | `Closed` |
| a caller gives up on a call (timeout, abort) | the task is aborted on the worker; nothing rotated | the caller's own cancellation |
| a stream poll after `start_shutdown` | not polled | `io::ErrorKind::NotConnected` |
| a reader parked in `poll_read` when the shutdown starts | its waker is woken; the re-poll meets the gate | `NotConnected` at once |
| `spawn_owned` cancelled after its mint finished | the dialer is already in `Owned`'s slot, never in the caller's hands; the guard takes the shutdown path | nothing; only the ledger |
| a stream dropped before, during or after the shutdown | dropped without a poll | nothing |
| a poll that panics | its drop guard still decrements the gate | the panic, in the host's task, as for `TorDialer` |
| the gate wait outlasts the budget | the wait is charged to the budget; the work then drops the client and shuts the runtime down anyway, so the host poll still inside arti may fail, or panic on the host's polling thread (the overrun's trade-off: waiting longer would hold the shutdown on a poll that may never return; the Relim host review, C4) | `Overran` |
| shutdown inside the budget | entry released | `Stopped` |
| shutdown overruns, its thread cannot start, or it panics | entry stranded, `stuck += 1`; the client may still write AND still talk to the Tor network until the process exits | `Overran`; the next `spawn_owned` answers `RestartRequired`; a host does not report Tor as off, and finishes any reset after a restart |
| arti's drop blocks past the budget (a filesystem stall) | the watchdog settles `Overran` at the budget, entry stranded first, awaited or not; the blocked drop finishes later and changes nothing | `Overran` (fail closed) |
| the outcome is not published within the running budget `+ SHUTDOWN_WAIT_GRACE` (the watchdog's thread started late), or a caller arrives while the first caller is still handing the shutdown off | the caller settles `Overran` through the one `Terminal`; a settlement already committed wins | the committed outcome, the same for every caller |
| `shutdown(b2)` after `start_shutdown(b1)` with `b2 < b1` | the shutdown runs with `b1`; the later caller waits by `b1 + SHUTDOWN_WAIT_GRACE` | the shutdown's own outcome; never an `Overran` made by the shorter wait |
| concurrent `shutdown` on two clones, or a dropped `shutdown` future | one shutdown, started synchronously | every waiting caller gets the same outcome |
| `shutdown` awaited from a task on the owned runtime | in every build: starts it, logs an error, and settles `Overran` through the `Terminal` at once (a misuse: the entry is stranded even if the shutdown would have stopped) | `Overran` (fail closed), `stuck() += 1`, the next `spawn_owned` answers `RestartRequired` |
| last drop without `shutdown` | `start_shutdown(drop_budget)`, unawaited | only the ledger |
| `Closed` / `RestartRequired` at a host's error mapping | LOCAL facts | a retryable not-ready error; never a relay marked failed, never a fallback code |
| zero connectivity | unchanged: a pending bootstrap is exactly the case the shutdown ends | `Stopped` |

### 12.7 Performance

One task spawn and one `JoinHandle` await per async call; one `SeqCst`
increment and decrement per stream poll. No polling, no timer: the shutdown
outcome is pushed through a watch, and the gate's wait is a `Condvar` wait on
the shutdown's work thread, notified at zero and bounded by the budget. Threads
per client as in §12.1, plus two short-lived ones per shutdown (the watchdog
and its work). The plugin's figures
do not change; Relim gains one runtime per live client.

### 12.8 Testing strategy (named tests; each watched against a mutant)

In `sdk/dialer-tor` (through §12.2's seam: `owned::core` generic over the
client, the stream and the shutdown-thread spawner, driven with probes; the
arti binding tested offline):

| Test | Pins |
|---|---|
| `an_owned_runtime_ends_its_tasks_before_the_ledger_releases` | moved from the plugin: a task holding a value across an `.await` is dropped by the shutdown, and the count falls only after that drop |
| `an_overrun_shutdown_stays_counted_and_stuck` | moved: `>=` the budget is `Overran`, live and stuck stay up |
| `the_real_client_is_counted_once_until_its_runtime_shut_down` | moved: offline mint, drop from an async thread, no panic, count back to zero |
| `shutdown_through_any_clone_runs_once_and_every_caller_sees_one_outcome` | concurrent and later calls; one `shutdown_timeout` |
| `a_dropped_shutdown_future_still_finishes_the_shutdown` | cancel-safety: the hand-off is synchronous; a later caller gets the outcome |
| `every_method_after_shutdown_fails_fast_with_closed` | sync and async, no runtime touched |
| `a_call_in_flight_at_shutdown_resolves_closed_never_a_panic` | a call holding a tokio `Sleep` of the owned runtime |
| `no_client_reference_outlives_stopped` | the interleaving the reviews found: a call races the shutdown (check, then spawn after the hand-off); the probe client's drop must run on the owned runtime or the shutdown thread and BEFORE `Stopped` is published, never on the host thread after it |
| `dropping_a_call_aborts_its_task_and_rotates_nothing` | abort-on-drop of the ONE `call` every async verb goes through (`bootstrap`, `connect`, `dial` are each a single `call`): a call dropped mid-flight has its task aborted and dropped on the worker, the client stays open. A bootstrap whose deadline drops it, and a dial dropped mid-flight, are covered by that composition (as-built note: the row's wording until 2026-10-08 named them as if tested separately; the Relim session asked) |
| `a_stream_after_shutdown_errors_on_read_and_write_and_drops_cleanly` | the poll gate over a fake stream holding a `Sleep` and an I/O registration of the owned runtime; read, write, and drop after shutdown (the generic case, no host latch) |
| `a_stream_dropped_before_the_shutdown_is_safe` | drop while the client is live; the shutdown that follows is `Stopped` |
| `a_stream_dropped_during_the_shutdown_is_safe` | drop while `shutdown_timeout` runs |
| `a_reader_parked_before_the_shutdown_wakes_with_an_error` | the parked waker is woken; no hang until a host timeout |
| `the_minted_client_is_never_dropped_on_the_callers_thread` | the client is installed by the task and never handed back, so a caller letting go on its own thread drops nothing of it there (the probe client records its drop thread). (Named `a_mint_finished_then_cancelled…` in the draft; renamed because it cancels nothing.) |
| `a_split_streams_reader_and_writer_are_both_woken` | one waker slot per direction |
| `awaiting_a_shutdown_whose_drop_blocks_reads_overran_in_bounded_time` | the await bound; the `Overran` read is the one outcome, the entry already stranded, and a later reader agrees |
| `a_blocked_shutdown_is_settled_overran_at_its_budget_with_no_waiter` | the watchdog: `start_shutdown` alone strands a blocked drop at the budget |
| `a_later_caller_with_a_shorter_budget_waits_for_the_running_one` | `start_shutdown(BUDGET)` then `shutdown(TIGHT)` over a drop that blocks past `TIGHT + grace` but inside `BUDGET`: `Stopped`, nothing stranded |
| `a_caller_that_finds_the_shutdown_begun_settles_the_one_outcome` | a caller between the first caller's gate close and its hand-off holds the `Terminal`, so its own bound strands the entry |
| `a_later_settler_answers_the_committed_outcome` | a settler that loses answers the winner's outcome, both orders |
| `racing_settlers_agree_with_each_other_and_the_ledger` | settlers racing on threads answer one outcome, with the ledger already holding it |
| `a_shutdown_thread_that_cannot_start_strands_and_answers_overran` | the failing spawner: runtime leaked, entry stranded, `Overran` |
| `a_work_thread_that_cannot_start_strands_and_answers_overran` | the watchdog starts, its work cannot (a spawner failing its 2nd call): client leaked, never dropped; `Overran`, stranded, `stopping()` back to 0 |
| `a_caller_racing_the_first_returns_with_the_shutdown_counted` | the first caller stalled between closing the gate and the hand-off; a second caller returns with `stopping() == 1` |
| `a_spawn_owned_waiting_past_the_bound_is_refused` | a watchdog whose thread starts late: `spawn_owned` answers `RestartRequired` within the bound |
| `a_spawn_owned_dropped_while_it_waits_holds_nothing` | cancel-safety of the wait: nothing counted, the shutdown still settles |
| `a_panicking_waiter_cannot_split_the_outcome_from_the_ledger` | a waiting host's waker that panics when the shutdown settles: every caller still reads the ledger's `Stopped` |
| `a_shutdown_in_flight_is_counted_until_it_settles` | `stopping()` from the hand-off; it falls to 0 only after the entry is stranded |
| `spawn_owned_waits_for_a_shutdown_in_flight_and_refuses_if_it_overran` | `start_shutdown` then an immediate `spawn_owned`: it waits, the old shutdown overruns, `RestartRequired`; one client counted |
| `a_panicking_waker_does_not_stop_the_shutdown_or_the_other_wakers` | wakers woken after the hand-off, each isolated; the shutdown still `Stopped` |
| `a_shutdown_awaited_on_its_own_runtime_starts_it_and_answers_overran` | the own-runtime arm, now the same in every build: started, `Overran`, stranded |
| `a_poll_that_meets_the_gate_closing_wakes_itself` | the re-check after a waker is stored: a poll returning `Pending` as the gate closes is woken, never left parked |
| `no_connection_attempt_follows_stopped` (offline, a local listener) | a client bootstrapping through a bridge that drops every connection retries it at least twice; after `Stopped`, no further attempt for three of its own retry gaps |
| `a_poll_in_progress_holds_the_runtime_until_it_returns` | the shutdown thread waits for the gate counter |
| `a_panicking_poll_does_not_pin_the_gate` | the drop guard decrements; the shutdown is not held |
| `a_gate_wait_past_the_budget_is_overran` | the wait is charged to the budget |
| `the_last_drop_without_shutdown_takes_the_same_path` | entry released, no panic on an async thread |
| `cancelling_spawn_owned_mid_mint_counts_until_the_runtime_ended` | the cancelled bring-up (Relim's Off) |
| `a_failed_mint_stays_counted_until_its_runtime_ended` | the half-way failure |
| `spawn_owned_refuses_while_a_client_is_stuck` | `RestartRequired`, one home |
| `a_panicking_shutdown_thread_publishes_overran` | no caller waits forever |
| `spawn_owned_works_from_a_current_thread_and_a_multi_thread_runtime` | host flavour irrelevant |
| `the_shutdown_thread_logs_through_the_construction_dispatcher` | the H1 regression, the dialer's drop-time flush included |
| `closed_and_restart_required_have_frozen_classes` | the published class strings, through `class()` and its mirror |
| `real_stream_after_shutdown_errors_never_panics` (`#[ignore]`, network) | the same over a REAL `TorStream` mid-transfer, and its drop; run by the nightly and the probe, not on every commit. It cannot pass vacuously: the reader must still be pending before the shutdown (so parked), the snapshot fails on any read error and must hold a non-empty `guards.json` |

In `zec_wallet_tor`: every existing test keeps passing over the ledger;
`a_rebuild_awaits_the_old_clients_shutdown_before_it_mints` closes §6's
deferral, `a_rebuild_that_finds_no_engine_still_waits_for_every_client`
pins the no-engine path, `a_rebuild_after_an_overran_shutdown_mints_nothing`
the stuck one, and `dispose_retires_the_client` the dispose. The rebuild's
`REBUILD_WAIT_MAX` path (a client still counted after 20 s) has no test: it
would hold the suite 20 s. The plugin's P9 error-table test went red until
`Closed` and `RestartRequired` were mapped, by design.

In Relim (its rows, at adoption; each with both polarities): Off mid-bootstrap
on unreachable bridges ends every arti connection (the probe, RED today);
`Overran` on Off is not shown complete AND the next Tor mint is refused, while
`stuck() == 0` mints; Tor→Tor with new bridges stops the old client before the
directory reopens; wipe `Stopped` → the `.tor` delete runs, `Overran` → it is
skipped, the marker kept and the Tor data reported unfinished, with the wait
after the identity shred; a bootstrap deadline ends the owned task; `Closed`
leaves every relay's health untouched; Off mid-deposit over a real client gives
a retryable error, no panic.

As adopted (Relim, 2026-10-08, at `3f864a9a4` and re-run at `97bcf47a1`): the
probe is GREEN (0 arti attempts after Off; the RED control 6–16). One
deliberate deviation, Relim's decision: its in-app WIPE deletes the Tor tree
even on `Overran` (bounded, in `spawn_blocking`), keeps the marker and
finishes at the next launch, because the in-app wipe is the duress path and
an untouched guard and consensus cache is evidence of Tor use; and it caps
its wait after the identity shred at 2 s, so the wallet keystore sever is not
delayed. A stuck client may then rewrite part of that tree until the process
exits; the marker's finish at the next launch removes it. The plugin keeps
the row above (`Overran` → the delete is skipped).

Gates: 1 security (this section's §12.4 rows, the review fan-out) · 2 UX (no
new UI; a host's Overran state is its own copy) · 3 common sense (the Relim
call sites above) · 4 architecture (one home in the crate; hosts own disk) ·
5 observability (the outcome logged once, by the crate) · 6 edge cases
(§12.6, each with a test above) · 7 constants (§12.2's table) · 8 i18n (none).

### 12.9 Multi-platform

Threads behave the same on Android, iOS, macOS, Linux and Windows. A shutdown
interrupted by the OS suspending the app: the bound is measured with
`std::time::Instant`, a monotonic clock that on Apple platforms and Android
does not advance while the device sleeps, so a suspension is not read as an
overrun; the shutdown resumes with the process. (Windows is not built yet;
whether its clock counts sleep is checked when it is.) No
platform-specific code.

### 12.10 Multi-device and sync

None: a Tor client and its state are per device.

### 12.11 Consistency audit

- ADR-0548 (the plugin is optional, through the dialer contract): unchanged.
- ADR-0550 (dialer-tor is developed here, pushed when needed): this change is
  made here; Relim takes it by path; crates.io gets 0.1.1 (the founder's
  publish) before `zec_wallet_tor` 0.0.1 publishes.
- ADR-0551 (own workspaces): no new dependency; the plugin's lock changes
  only `dialer-tor`'s own entry (0.1.0 → 0.1.1, its crates.io checksum).
- ADR-0567 (exit rotation): kept, through `OwnedOptions`.
- §3.3 "Threading", §4's `clearState`/`dispose` rows, §7's
  `ENGINE_SHUTDOWN_MAX`: rewritten in the plugin's adoption change to cite
  the crate instead of `engine::ClientRuntime`, and the §6 deferral closes,
  all before the 0.0.1 publish.
- `wallet-sdk.md`: no change (the wallet contains no Tor).
- Versioning: `dialer-tor` 0.1.1 is additive for consumers (`TorDialError`
  is `#[non_exhaustive]`; the rest is new types), but not free: the crate's
  own exhaustive `class()` and its test mirror, and the plugin's
  `dial_codes.rs` allowlist and its two `=0.1.0` pins, change together
  (§12.2). The tokio features grow; no lock entry changes. No wallet ABI,
  bridge ABI or plugin ABI change.

**Design review of this patch (2026-10-07).** Security ∥ arch ∥ crypto on the
draft, plus the Relim session's host-side review (relim-9e, at Relim
`76e36f58f`): GO with folds. Security and crypto independently found the same
two BLOCKERs, both folded: a reference to the client cloned on a host thread
could be the last one and write the guard state after `Stopped` (arti writes
it in `CircMgrInner::drop`), now no reference leaves the owned runtime and the
shutdown thread; and a `spawn_owned` cancelled mid-mint was unspecified, now
the `OwnedRuntime` guard takes the shutdown path. Folded hardening: the
synchronous, cancel-safe `start_shutdown`; the gate's drop guard, `SeqCst`,
and its wait moved to the shutdown thread and charged to the budget; the
dialer dropped on the shutdown thread; abort-on-drop for calls; the stuck
refusal moved into `spawn_owned` (`RestartRequired`); a panicking shutdown
thread publishes `Overran`; `Closed` mapped as a LOCAL retryable fact; Relim's
call sites as one funnel, Tor-only waits, the wipe wait after the shred; the
tokio features, the four change-together sites, the ledger's one term in the
plugin's predicate, and the 10 s budget's honest WHY. Corrected premise
(security): arti's per-stream sleeps use the POLLING runtime, so the gate is
defence in depth for streams, not the only thing between a host and a panic.
Then code-reviewer on the folded text: no BLOCKER; folded its HARDENING —
the minted client is installed by the mint task itself and never handed to
the caller (a cancelled `spawn_owned` could otherwise drop it on a host
thread: the BLOCKERs' class), the gate's wait is a `Condvar` (no spin), the
generic `owned::core` test seam with a failable spawner, `send_replace` and
`wait_for` on the watch, the lock order and poison policy, parked readers
woken by the shutdown; and its notes (the ADR's refusal wording,
`drop_budget` named, the unverified stream premise stated with its test,
`stuck()` unmoved by the release-mode `Overran`). Then the Relim session's
confirmation over the final text: all nine folds confirmed, GO; its three
wording points folded (Off and the funnel call `start_shutdown`, the mint
awaits; the stream fallback's real scope, since a thread that has exited
cannot take a late drop; a "drop before" test row).

**Review of the BUILT diff (2026-10-07; security ∥ arch ∥ crypto on
`b5698cd09..a7fa59c27`).** One BLOCKER, found by the crypto audit and, as
hardening, by the other two: the plugin's rebuild could mint while an earlier
client was still counted — on its wait's timeout (a parallel clock that a
late shutdown thread outlasts) and on a rebuild that found no engine. Folded:
every rebuild waits, and none mints while a client is counted (§12.3 "As
built"). Folded hardening: one parked waker per stream AND direction (a split
stream lost its reader's), wakes outside the lock, a bounded `shutdown` await
(`SHUTDOWN_WAIT_GRACE`), a gate-closed mint drops its client after the slot's
lock, `OwnedOptions` `#[non_exhaustive]` before the first publish, public
wording in the crate's comments, the README's `Arc` import and its "every
method" sentence, `Overran` stated as possibly still talking to Tor, the live
test snapshotting the whole Tor directory, and three test fixes (a misnamed
test renamed, the parked reader and the racing calls made deterministic).
The live test was watched RED against a planted leak (a stream keeping the
client): `guards.json` and `circuit_timeouts.json` were rewritten after
`Stopped`, and the test failed on exactly that. The code-reviewer on those
folds: no BLOCKER; folded its two HARDENING items — the rebuild's check and
mint made one step (it also waits out any other mint in flight), and the
public `shutdown` doc, README and CHANGELOG now say the bounded wait needs a
tokio timer. Left as noted: the `REBUILD_WAIT_MAX` timeout is untested, and a
rebuild past it ends the registration (the host re-inits), fail closed.

**External reviews of the final diff (2026-10-07/08).** Of `0d8d8bbbe`, one
MEDIUM: a caller whose own bound expired answered `Overran` alone, with
`stuck()` still zero, so `spawn_owned` allowed a new client, and a later
caller could read `Stopped`. Folded in `ca6c22418`: one `Terminal` per
shutdown, settled once; the work moved to a thread of its own, and the
shutdown thread became the watchdog that settles `Overran` at the budget;
settling `Overran` strands the entry before publishing. Of `ca6c22418`, two
MEDIUM races in that fold: the `Terminal` was installed after the gate
closed, so a concurrent caller could hold none and answer alone; and
`settle` released its lock before stranding and publishing, so a losing
caller could answer an `Overran` it made up while the winner published
`Stopped`. Folded: the `Terminal` is made when the client starts, and
settling commits the ledger and the outcome under its lock and answers the
committed outcome. Each fix is watched against a planted mutant (`t1`–`t4`).
Then security-review ∥ code-reviewer on `0d8d8bbbe..4353b37ba`: no BLOCKER.
Folded: every caller's wait is bounded by the budget the shutdown RUNS with,
not its own (a shorter later caller stranded a shutdown that would have
stopped; mutant `t5`); a test's read of the watch waits for the publish,
which follows the ledger commit; the log line moved after the publish; the
own-runtime row corrected (it strands now); `settle` checks before it
replaces; commit hashes out of the public tests. Noted and left: the work
thread drops the client outside a tokio context, as before these folds (a
drop that needed one would panic there and settle `Overran`, fail closed;
the live test passes).

**Relim's host-side review of `9aa5bbbd0..3f864a9a4` (2026-10-08; two
angles, GO with folds, no BLOCKER).** Folded before the publish, since three
change behaviour: C1, `spawn_owned` waits for every shutdown in flight
(`ClientLedger::stopping()`, new) before its stuck check, so `start_shutdown`
then an immediate mint can no longer put a client beside one that later
overruns (the draft's "at most one stuck client per ledger" was false
without it); C2, a shutdown awaited on its own runtime starts the shutdown
first and answers the same in every build (the `debug_assert!` stopped
nothing in a debug build); C3, parked wakers are woken only after the
hand-off, each isolated, so a panicking waker cannot leave the gate closed
with nothing handed off. C4, the gate-wait timeout's trade-off, is now
stated (§12.6). C5, three guards no test could fire, now each have one (the
park re-check, the own-runtime arm, a work thread that cannot start). C6,
the live test can no longer pass vacuously. Host fit: F1, `Setup`'s two
temporary owned-client kinds are listed in the README and CHANGELOG
(classify `Setup` by kind); F2, a wildcard arm compiles but mapping these two
to a failure is wrong; F3, the 10 s budget's "not measured on a device" is
in the README and CHANGELOG; F4, a crate test now counts connection
attempts after `Stopped` against a local bridge listener. Left for 0.1.2
(additive): F5, a public marker type for the post-shutdown stream error,
today a plain `NotConnected`. Then security-review ∥ code-reviewer on that
fold (`3f864a9a4..14177e829`), plus Relim's note on C1: one BLOCKER (B1, a
second caller could return before the shutdown was counted, then start a
client beside it; now counted before the gate closes) and folded hardening:
`spawn_owned`'s wait bounded by the running budgets plus the grace
(`RestartRequired` past it) and shown cancel-safe; the outcome published
before the shutdown leaves `stopping` and the waiters' wake isolated, so a
panicking waker cannot make callers read `Overran` against a `Stopped`
ledger (H1); the bridge test drains the listen backlog and measures the
longest retry gap; a test message and a mutant row's cite corrected; the
ledger's impl blocks merged. Mutants `h8`–`h11` caught; `h1`, `h2`, `h7`
re-watched. Then a confirmation security review of those fixes
(`14177e829..e985b50a2`): no BLOCKER; claims 1, 2, 4 and 5 hold. Folded:
H-1, the wait's bound is re-read on every wake, so a longer shutdown handed
off during the wait is not cut short into a false `RestartRequired`
(`a_longer_shutdown_started_during_the_wait_extends_it`, mutant `h12`), and
`RestartRequired`'s doc and README row say it is retryable while `stuck()`
is 0; H-2, budgets are capped at `MAX_SHUTDOWN_BUDGET` so `Duration::MAX`
cannot overflow a deadline. Its own confirmation review: the new test made
non-vacuous (the wait signals before it reads its bound) and given margin
against an overrun on a loaded machine. Then Relim's report as the second
host (adopted at `3f864a9a4`; its Tor-off probe GREEN, 0 attempts after Off
in 3 runs, the RED control 6–14): of its four frictions, (1) a refusal not
atomic with a shutdown in flight and (4) no wait for a client still counted
are answered by C1/B1 (a client counted but not shutting down is the host's
own); (2) one `setup` class for transient and configuration kinds stays,
classified by `kind` (F1's docs); (3) a clean stop now logs one `info` line,
as an overrun logs one `warn`.

### 12.12 Order of work

Founder 2026-10-07: BEFORE the 0.0.1 publish, and all four packages wait.

1. This patch reviewed (security, arch and crypto angles, then code-reviewer),
   and Relim's host-side review. DONE.
2. The build plan (`docs/plan/tor-owned-client.md`), written before the code.
3. `dialer-tor` 0.1.1 built in a worktree, its §12.8 tests each watched
   against a mutant, with the review fan-out on the diff.
4. `zec_wallet_tor` wraps the crate (by path while building); the plugin's
   own runtime code goes; its tests pass over the ledger.
5. `dialer-tor` 0.1.1 to crates.io (the founder's publish); the plugin pins
   `=0.1.1` and its lock takes the crates.io entry.
6. A new release cut, the local public CI, the founder's push, then the four
   pub.dev publishes on his go.
7. Relim adopts by path whenever its own chunk allows; its probe RED before,
   GREEN after. It does not gate the 0.0.1 publish.
