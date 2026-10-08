# 0569 — Wallet sync fetches only the subtree roots it does not already hold

- **Status:** Accepted (2026-10-03: the built diff passed security, arch and crypto on the diff,
  its fold and its final fold, then the code reviewer's land — plan §11). Design reviewed S315
  (security, arch, crypto); revision 2 folds them.
  Amended by [ADR-0570](0570-wallet-sync-incremental-roots-turns-itself-off-for-a-server-that-cannot-serve-it.md) (the timeout and drop rules), 2026-10-03.
- **Date:** 2026-10-02
- **Links:** `docs/plan/s15-f1-subtree-roots-fetch-only-what-is-new.md` (the design, revision 2;
  §10 the review record) · `docs/plan/stage-15-the-tor-plugin-on-devices.md` §5 (S15-F1, the measurements) ·
  `docs/plan/probes/README.md` (`start_index` on two servers) · `docs/specs/wallet-sdk.md` §3.2g
  and iv-d-1 (amended by this ADR) · upstream `zcash_client_backend` 0.24.0 `sync::run`

## Context

On every sync pass the SDK streams every completed subtree root of Sapling, Orchard and Ironwood
from index 0, checks the whole sequence, and writes it all back. That is upstream's reference
shape (`zcash_client_backend::sync::run`), and on a direct connection it costs about 2 s for
roughly 1,900 messages (147 KB on mainnet in October 2026). Over the Tor plugin on an iPhone
(stage S15) it was 85–95 % of every pass: 4–12 s on a good circuit, most of a minute on a slow
one. A pass that should take about a second instead delays incoming payments, costs data and
battery over Tor, and grows with the chain.

The full re-serve is not only a fetch: the root bind (`root_bind.rs`) re-checks every recorded
height, and C6 hash-checks each root's completing block once the wallet has scanned it. Any
change must keep every one of those guarantees.

## Decision

Each pool is fetched from the last root the wallet already holds: `start_index` = the stored
count − 1, earlier when a reorg window, a bracket, or a newly scanned completing block needs a
re-serve, rounded down to a multiple of 64. The stored count is the leading run of indices with
both a completion height and a root hash, so a gap or a missing hash ends the run and is
re-served from there. Every write is still bound against the full virtual sequence (the stored
prefix plus the served suffix). A full fetch from 0 still runs on the first pass of every
session, after a server switch or rescan, on any rewind and on the pass after any relaxed bind,
when nothing is stored, on any fallback, and at least every 180 passes or 3,600 s. A refused,
ignored or mis-served non-zero start retries from 0 in the same pass and keeps that pool on full
fetches for the rest of the session (a single dropped stream does not).

## Alternatives considered

- **Keep the full re-serve (upstream's shape).** Correct and simple, but its cost scales with
  the chain and multiplies Tor's latency; it is the defect.
- **Skip the roots phase when the tip has not moved (phase C).** Cheaper still, but it drops C6
  for a block scanned since the last pass unless more state is tracked. Deferred; built only if
  this decision's own device numbers leave the phase material.
- **Persist the memo across launches.** Would make cold starts cheap too, but adds schema and
  migration obligations and weakens the rule that a fresh session re-measures its endpoint.
  Rejected for now.

## Consequences

- A pass over Tor drops from seconds to about one round trip for the roots, and the cost stops
  growing with the chain.
- The SDK departs from upstream's reference sync on purpose; an upstream change to subtree-root
  handling must be read against this ADR.
- Detection of a server's inconsistency at an interior index (outside the reorg window) waits
  for the next full verification (at most an hour or 180 passes, or the next launch) instead of
  the next pass. Nothing wrong is written meanwhile: those indices are never re-put.
- A reorg deeper than the window (`REORG_MAX_BLOCKS`) that changes a root outside it is caught
  at the next full verification, as a spend whose anchor is off-chain — never a silent loss —
  where today it is a loud refusal on the next pass.
- `start_index` becomes a server-bound field; its justification is in `docs/specs/wallet-sdk.md`
  (the same for every synced wallet in steady state, rounded down to a multiple of 64).
