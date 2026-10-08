# 0560 — Wallet custody: the index outlives every purge, and a wipe that cannot read it fails closed

- **Status:** Accepted (stage S2 `custody`, the DIFF review's fold, S296,
  2026-09-22; `docs/plan/stage-2-the-hosts-lifecycle.md` §5 "S296").
  **Extends [ADR-0559](0559-wallet-custody-a-wallets-identity-is-a-minted-id-in-the-wrap-artifact-not-its-path.md);
  supersedes nothing in it** — every decision there stands. This one records
  three rules the diff review showed ADR-0559's mechanism needs to be true in
  every crash window, and a third residual.
- **Date:** 2026-09-22
- **Links:** ADR-0559 · ADR-0526 · the S2 diff review (four angles + refuters
  on `b2d30176..e144fc7e`; the stage plan's §5 review table) ·
  `docs/REVIEW.md` §3 (the post-build pass catches what the design pass
  structurally cannot)

## Context

ADR-0559 made a path-keyed keychain INDEX the breadcrumb that lets a wipe
find a wallet's custody namespace after the host has deleted its files, and
deferred a migrated wallet's legacy purge to its next open. The diff review
found three ways the breadcrumb could still be lost or misread:

1. **Every backend's `purge_namespace` also deleted the index.** The legacy
   namespace a migration purges IS the path namespace the index lives under.
   A kill between that purge and the `done` rewrite left no index at all —
   and the host's delete-the-files-then-wipe sequence then "succeeded" with
   the wallet's wrap key alive. (The review's first remedy — write `done`
   before the purge — reproduced the finding: the purge deleted it anyway.)
2. **A bare-config wipe read an unreadable index as an absent one.** With the
   files gone, the index is the only thing that can name the namespace; a
   read fault ended in `Ok`, key live. The verify-real-sever guard could not
   see it: it keys on files that are, in exactly this case, absent.
3. **A `pending` record's legacy namespace was purged uncorroborated** — the
   header had just been made to prove itself; the index's `legacy_ns` field
   had not.

## Decision

- **A purge is wrap material only.** `purge_namespace` on every backend
  (Android, Apple raw and Secure Enclave, both test vaults) severs wrap keys
  and nothing else. The index's ONE deleter is the wipe, LAST — after the
  crypto-shred and the file sweep, so it is the last thing standing when
  anything is interrupted. A deferred legacy purge therefore leaves the
  `pending` record in place until the `done` write lands.
- **A wipe that cannot read the index, with the files already gone, fails
  typed** — nothing has been severed, so a retry is safe. `force` overrides
  (a present-but-corrupt index must not make a wallet unwipeable from a bare
  config). An index that reads as ABSENT stays a no-op success: the second
  call of a converged wipe, or a wallet that never existed at the path. With
  the directory present the read stays fail-open (the header can still name
  the namespace, and must prove itself — ADR-0559).
- **Only this path's own `pending` record is acted on.** A record's
  `legacy_ns` is purged — by the wipe, or by the deferred step (5) at open —
  only when it equals this path's namespace, the one value a migration ever
  records (the same equality its resume check already demands).
- **Every Apple keychain item goes through one write door**, pinned
  `WhenUnlockedThisDeviceOnly` on iOS: the Secure Enclave tier's index item
  had been written without it, so it could ride a backup onto another device.

## Consequences

- The index survives every crash window between the migration's commit and
  its settled `done` state; a bare-config wipe either finds the namespace or
  refuses loudly. The `force` escape hatch keeps its existing meaning ("this
  IS my wallet; proceed").
- A host's wipe may now return a typed error it did not before: a keychain
  that cannot read the index while the files are already gone. Retrying
  converges once the keychain answers.
- **Residual (3), stated, not closed here:** a FRESH create at a path whose
  index holds a stray `pending` record (a pre-stage wallet there whose files
  the host deleted without our wipe) overwrites the record with its own
  `done` entry; the dead wallet's legacy key under the path namespace is then
  purged by nothing but a later wipe of the SAME path's namespace — which, for
  the new wallet, no longer runs (its custody has an identifier). It needs a
  pre-stage wallet deleted without a wipe and a new wallet created at the same
  path; the seals that key could open were deleted with the files. Joins
  ADR-0559's two residuals.
