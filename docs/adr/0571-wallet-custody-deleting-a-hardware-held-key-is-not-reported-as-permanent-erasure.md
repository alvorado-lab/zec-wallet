# 0571 — Wallet custody: deleting a hardware-held key is not reported as permanent erasure

- **Status:** Proposed (2026-10-06); accepted at the review of the built diff.
- **Date:** 2026-10-06
- **Links:** `docs/specs/wallet-sdk.md` FR-14 (chunk 1 attack-vector analysis, the H1 custody
  disclosure, chunk 2) · the external code review of 2026-10-05, finding F02 ·
  `sdk/zec-wallet-core/src/keychain/mod.rs` (`VaultTier`, `CustodyDisclosure`)

## Context

`CustodyDisclosure.crypto_erases_on_delete` answered `true` for every hardware tier (Android
StrongBox and TEE, the Apple Secure Enclave), and the wallet's delete confirmation turned that
into "the keys become permanently unrecoverable on this device". FR-14 states the same as fact.

The property behind "permanently" is that the deleted key cannot come back. Neither platform
establishes that for our key:

- **Android.** A Keystore key lives as a key blob that only the device's secure hardware can
  use. `deleteEntry` removes the blob. Unless the key is rollback-resistant, which is a separate
  key characteristic in KeyMint, a copy of the blob taken earlier can be used again on the same
  device. Our key generation does not request rollback resistance and nothing checks for it.
- **Apple.** A Secure Enclave key is stored in the keychain as a blob wrapped by the enclave.
  `SecItemDelete` removes the keychain item; nothing here shows that an earlier copy of the
  wrapped blob is unusable on the same device afterwards. FR-14 quotes Apple as calling data
  wrapped with a deleted SE key "permanently undecryptable… irreversible by design"; that
  quote has no source we can cite, and a search of Apple's developer material (2026-10-06)
  found no per-key deletion guarantee. The quote is withdrawn, not relied on.

What the hardware tiers do establish is narrower and still worth saying: the key never leaves the
secure hardware in usable form, and the wallet deletes it when the wallet is deleted.

## Decision

The disclosure reports what deletion does, not what it guarantees. `crypto_erases_on_delete:
bool` is replaced by `erase_assurance: EraseAssurance`, a `#[non_exhaustive]` enum with two
values:

- `HardwareKeyDeleted` — StrongBox, TEE, Secure Enclave: the key was held by secure hardware and
  has been deleted.
- `BestEffort` — the raw Apple keychain item, a software keystore, or no keystore: the key was
  stored as data and deleted; a copy may remain in storage until the device reclaims it.

No value claims permanence. The delete confirmation for a hardware tier says the key that locks
the wallet is held in the device's secure hardware and is deleted with the wallet (only that
wrapping key is in hardware; the seed and spending keys are sealed on disk and unsealed into
memory to sign). The bridge enum adds `Unknown` for a value a newer core adds, rendered as
best-effort. The duress sever's `Severed` outcome no longer says "nothing on disk can be
decrypted": it says the key that unlocks the files was deleted, with the tier's assurance. FR-14's "true cryptographic
erase" statements are corrected in place to point here.

## Alternatives considered

- **Keep the boolean and only change the UI sentence.** The field's name and its documentation
  are the public claim; a host renders its own text from it. The name has to stop saying
  "erases".
- **Return `false` for every tier.** Loses a real distinction: a hardware-held key and a key
  stored as plain keychain data are not the same custody, and a host should be able to show it.
- **Establish rollback resistance and keep the claim.** On Android this needs the key generated
  as rollback-resistant where the hardware supports it, and the property read back from key
  attestation per key; we found no equivalent per-key property Apple documents. That is a feature of its
  own, after 0.0.1; the enum leaves room for a stronger value when it exists.

## Consequences

- Public API change before the first release: the core's `CustodyDisclosure` field, the bridge
  DTO and its Dart class. The bridge wire contract changes, so the bridge ABI version moves.
- `VaultTier::crypto_erase_on_delete()` becomes `VaultTier::erase_assurance()`, still an
  exhaustive match so a new tier must choose its value; an unknown tier is `BestEffort`.
- In all 16 locales the security screen's custody line changes, and the delete row and the
  delete confirmation stop saying "crypto-shred" and "permanently": they say the wallet and its
  key are deleted from the device.
- Follow-up after 0.0.1: Android rollback resistance through key attestation, which would add
  an assurance value stronger than `HardwareKeyDeleted`.
