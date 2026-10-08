# 0561 — Wallet derivation: NFKD the BIP39 passphrase, and keep no legacy path

- **Status:** Accepted (stage S2 `recovery`, build B's fold, S297,
  2026-09-23; `docs/plan/stage-2-the-hosts-lifecycle.md` §3.3 revision 3 and
  §5 "S297")
- **Date:** 2026-09-23
- **Links:** the 2026-09-20 production-readiness review's R03
  (`docs/plan/audit-2026-09-20-remediation.md` §2d) · the founder's S261
  ruling (no update path for an SDK nobody uses) · `docs/specs/wallet-sdk.md`
  §3.2g's restore row (its "MANAGER-FLAGGED: un-normalized BIP39 passphrase"
  note is resolved here) · ADR-0526

## Context

BIP39 derives the seed from the passphrase's NFKD form. `resolve_seed` called
the pinned `bip39` crate's `to_seed_normalized`, which PBKDF2s the bytes it
is given, with the raw passphrase: a composed "é" and "e" plus a combining
accent were one credential and two wallets, and a phrase with a non-NFKD
passphrase from any other BIP39 wallet restored an empty wallet here. The
crate's normalizing `to_seed` was available (bip39's `std` pulls in
`unicode-normalization`) but normalizes into a `Cow` it drops un-zeroized.

The review asked for the fix AND a legacy-recovery path for wallets already
funded under the raw derivation. The stage's revision 2 contracted one: an
evidence rule choosing the legacy wallet when the normalized one is empty, a
marker newtype, a "which derivation" field on "the existing restore outcome".

## Decision

1. **The passphrase is NFKD-normalized through the crate's own normalizer
   before PBKDF2**, in both BIP39 arms of `resolve_seed`
   (`nfkd_passphrase`: `Mnemonic::normalize_utf8_cow` → the one owned copy
   into `Zeroizing` → `to_seed_normalized`). The raw-bytes arm (a host-
   supplied seed) is outside the change, pinned by a cross-product known
   answer on Relim's frozen wallet sub-seed and by the #357 fingerprint of a
   pre-change host-seed wallet.
2. **No legacy-derivation path exists.** Dropped at the contract's revision 3,
   before the build: the surface it named did not exist (`restore` returns
   the handle, and the evidence rule needs a sync the restore does not run),
   and the founder's S261 ruling voids migration machinery that protects
   only wallets of an SDK nobody uses. The only wallets it would have saved
   are ones funded through this unpublished SDK's pre-fix restore with a
   non-NFKD passphrase.

## Alternatives considered

- **Swap to the crate's `to_seed`.** One line, correct seeds — and a
  normalized copy of the passphrase freed un-zeroized on every restore with
  a non-NFKD passphrase. Lost to the settled mechanism.
- **The legacy path as contracted.** Would have needed a post-sync
  derivation read the SDK does not have, a second scan of the restore's
  range, a restore option and 16 locales of copy — for no user. If a real
  population of legacy-funded wallets ever exists (a founder call), it is a
  new item with its own surface.
- **Normalize outside the crate into a pre-sized buffer** (closes the
  residual below). Custom glue on a secret buffer where the audited crate
  has a door; not taken at S2, recorded as hardening.

## Consequences

- Every BIP39-conformant phrase + passphrase restores the wallet other BIP39
  wallets restore. ASCII and empty passphrases derive byte-identically to
  before (the vectors are pinned).
- A wallet funded through this SDK's restore with a non-NFKD passphrase
  BEFORE this change is not found by the same phrase after it. By the S261
  ruling there is no such user; the integration guide says what a host that
  derives its own seed from a passphrase must do.
- **Residual (HARD-C class, recorded in `derivation.rs`'s module doc):** the
  crate's normalizer builds the NFKD `String` by growth, so outgrown buffers
  — partial copies of the normalized passphrase — are freed un-zeroized
  inside the crate, for a non-NFKD passphrase only.
- `evals/standing_reds.txt` retires R03; no API, DTO or ABI change.
