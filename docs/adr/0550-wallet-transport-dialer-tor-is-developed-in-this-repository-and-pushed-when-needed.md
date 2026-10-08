# 0550 — Wallet transport: `dialer-tor` is developed in THIS repository and consumed by path; the public repository and the crates.io publish are the founder's later act

- **Status:** Accepted (founder, 2026-09-17 ~10:15 — *"why you care about
  alvorado-lab/dialer-tor, just make everything locally properly and when we
  need, we push it"*, on the S284 plan's finding that the plugin could not
  land before a crates.io publish) — **S284 (2026-09-17): Decision 1's "a
  MEMBER of the `sdk/` cargo workspace" is NARROWED by
  [ADR-0551](0551-wallet-transport-dialer-tor-and-the-tor-plugin-are-their-own-cargo-workspaces-under-sdk.md)
  (measured at the resolver: the crate is its own workspace root under
  `sdk/`); everything else stands**
- **Date:** 2026-09-17 (S284)
- **Supersedes:** [ADR-0548](0548-wallet-tor-for-standalone-hosts-is-an-optional-plugin-through-the-dialer-contract.md)
  Decision 1 **in part** — the DESTINATION stands (its own public repository
  `alvorado-lab/dialer-tor`, crates.io, consumed by version once published);
  the DEVELOPMENT HOME and the timing change: the crate is built and consumed
  here first. ADR-0548's alternative "the shared crate inside the SDK
  repository — lost to D1" is narrowed: lost as the PUBLISHED shape, taken as
  the development shape. Everything else in ADR-0548 stands.
- **Links:** [ADR-0542](0542-wallet-transport-host-dialer-first-builtin-arti-optional.md)
  Decision 2's surviving clause (arti is consumed as the shared module both
  products use, never forked in — the local crate IS that one module; Relim
  flips to it) · `docs/plan/fr5-phase-1.md` §0.3 (the finding this ADR
  retires) and its chunk C0 · `docs/handoff/host-session-complete-integration.md`
  H-15 (re-cut: the SDK session makes the move locally; Relim flips) ·
  `sdk/zec-wallet-core/tests/extraction_policy.rs`
  (`core_and_bridge_have_no_relim_deps`'s sanctioned list; P26) ·
  `docs/handoff/release-checklist.md` §D (the publish order, unchanged) ·
  `docs/specs/tor-plugin.md` D1 / §1.4 / §3.6 (dated corrections)

## Context

The S284 plan (`fr5-phase-1.md` §0.3) found that the plugin chunk could not
LAND before `dialer-tor` was on crates.io: the committed `sdk/Cargo.lock`
must resolve from the registry, a `[patch.crates-io]` path resolves a stanza
with no `source`, and the nightly's detached worktree carries no local cargo
config. It then made the founder's creation of a GitHub repository a
critical-path date and asked for it. The founder's answer was the ruling
above: build it locally, properly; the push is his later act.

Measured the same morning: on `wallet-track`, `crates/transport-tor` is the
June scaffold (`aceb5a58`, 2026-06-07; one 72-line `lib.rs`, "arti adapter
(v1."), not Relim's dialer. Relim's real crate (1339 lines — `dialer.rs`,
`error.rs`, `bridges.rs`, `isolation.rs`, `stream.rs`, `lib.rs`, a
`tests/carve_out.rs`, `tests/offline_refusals.rs`) lives on `main`
(`e7c2532d`; the crate's last commit `2948a96b`), byte-identical in this
clone and in the Relim checkout. The two branches are 1221 / 1411 commits
apart and the root workspace on `wallet-track` carries no arti pins. So the
"move" is a copy from `main`, not a rename in place.

## Decision

1. **`dialer-tor` lives at `sdk/dialer-tor` on `wallet-track`** — Relim's
   `crates/transport-tor` copied at a NAMED `main` rev, renamed
   `dialer-tor`, `publish = false` until the push, a MEMBER of the `sdk/`
   cargo workspace. This is the `apple-secure-enclave` shape: an
   extraction-unit-internal crate, sanctioned BY NAME in the extraction
   policy, a workspace member in the lock (P26 exempts members by
   construction, so it still guards every other package). The plugin depends
   on it by path (`dialer-tor = { path = "../../dialer-tor" }`). The SDK's
   `[workspace.dependencies]` gains arti's pins copied verbatim from `main`'s
   root (`arti-client = "=0.45.0"` with `default-features = false`,
   `tor-rtcompat`), and `sdk/deny.toml` gains the `equix` / `hashx` bans
   Relim's carries — the LGPL carve-out travels with the crate.
2. **The two H-15 additions are made HERE, by the SDK session, with their
   tests:** `TorDialer::set_dormant(DormantMode)` + the `DormantMode`
   re-export, and the timeout / network error split
   (`TorNetworkTimeout` vs `ExitTimeout` / `RemoteNetworkTimeout`;
   `LocalNetworkError` vs `TorAccessFailed`). This is the ONE copy both
   products then consume (ADR-0542 D2's clause). Relim's `main` flips its
   dependency from `crates/transport-tor` to this crate when Relim's session
   merges it — Relim's item, no date required of anyone.
3. **The public repository and the crates.io publish happen when the founder
   says** ("when we need, we push it"): `sdk/dialer-tor` is then extracted —
   the same operation the SDK itself will undergo — the SDK's dependency
   flips to a version, the sanctioned entry is removed, and the checklist §D
   publish order (`dialer-tor` → `zec_wallet` → `zec_wallet_ui` →
   `zec_wallet_ui_platform` → `zec_wallet_tor`) stands unchanged.
4. **No SDK plan step waits on a founder-owned outward act.** A step that
   needs one (a repository, a publish, an account) is written as HIS later
   step; the in-tree shape that keeps every gate honest is what the plan
   builds against. The `wallet-track` scaffold `crates/transport-tor` is left
   as it is (the root workspace on this branch is the June skeleton, not
   Relim); it is not this crate.

## Alternatives considered

- **A local `[patch.crates-io]` path** (the spec's development shape):
  invisible to every gate, and the nightly's worktree has no local config —
  a lock resolved through it cannot be committed honestly. Lost.
- **A `git = "file:///…"` dependency on a local repository:** machine-specific,
  refused by the extraction policy and by `deny.toml`'s `[sources]`. Lost.
- **A local cargo registry** (a static index on disk): faithful to
  "consumed by version" but tooling for no gain over a member crate that
  every existing gate already covers. Lost.
- **Waiting for the public repository:** the founder's ruling. Lost.

## Consequences

- `docs/plan/fr5-phase-1.md` gains chunk **C0 — the local crate** ahead of
  C1; its §0.3 finding is retired and its §5 D-5 / D-6 rows re-read; nothing
  is built in a worktree "until the publish" any more.
- `docs/specs/tor-plugin.md` D1, §1.4 and §3.6 carry dated corrections; the
  design is untouched.
- H-15 is re-cut: the move and the two additions are the SDK's, locally;
  Relim's flip is Relim's; the repository and the publish are the founder's.
  The board's FR-5a row and the request file follow.
- `sdk/.cargo/` stays gitignored — no patch is needed now, and a stray one
  must never be committed.
- The S284 lesson, recorded in memory: a founder-owned outward act is never
  a gate on landing code.
