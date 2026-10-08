# ADR-0538 — Watch-only wallets: UFVK import + the ONE sanctioned, gated UFVK export

- **Status:** Accepted (2026-07-21, S224; task #397 — spec §3.7) · Amended
  same-session at implementation (S224) — the Consequences "rescan re-import"
  bullet is struck through in place (the repo's supersede idiom, spec §1.6
  precedent) and replaced; no accepted DECISION changed, only an implementation
  consequence that proved simpler to trade away. The living design detail is
  §3.7 (the spec is the source of truth); this ADR records the decision.
- **Deciders:** founder (direction), worker (design)
- **Supersedes / amends:** the §1.6 "No UFVK/viewing-key export in v1" scope
  cut and the §5.4 HARD-D wording "the UFVK never leaves the device" — both
  RE-CAST, not repealed (see Decision 3). ADR-0005 (librustzcash-direct),
  §3.2f provisioning, and the §4.2 key-lifecycle posture are unchanged.

## Context

The SDK can already run its entire read path — sync, balances, history,
receive-address derivation — from the stored account UFVK with no seed
present (`None`-persistence is "view-only-at-rest"; engine scanning is
UFVK-based by construction). What is missing is the SURFACE: a wallet cannot
hand its full viewing key to another instance, and an instance cannot be
created FROM one. Real adoption scenarios need exactly that pair: a
second-device watch wallet, an accountant/auditor who must verify balances
against the chain independently, portfolio tooling that reads without spend
authority. The upstream stack supports it whole: `zcash_keys`'s
`UnifiedFullViewingKey::{encode, decode}` (the standard Bech2m/F4Jumble
`uview…` encoding) and `zcash_client_sqlite`'s
`import_account_ufvk(…, AccountPurpose::ViewOnly, …)` at our pinned
versions — no custom crypto anywhere in the feature.

The tension: a UFVK is not spend authority, but it IS total-history
surveillance capability — every incoming AND outgoing payment, past and
future (the OVK reveals outgoing). §5.4 froze that as HARD-D: "the UFVK never
leaves the device… any future watch-only/UFVK-export feature is a NEW spec +
review cycle, never a flag." This ADR is that cycle's decision record.

## Decision

1. **Ship the PAIR, one cycle: export AND watch-only import.** Export alone
   serves only users of other tools; import alone is unreachable. Every
   driving scenario needs a producing wallet and a consuming instance, so
   #397 is one feature with two halves (spec §3.7).
2. **Standard encoding, audited libs whole.** The exported artifact is
   exactly `UnifiedFullViewingKey::encode(network)` (`uview1…` /
   `uviewtest1…`); import is `decode` + `import_account_ufvk` with
   `AccountPurpose::ViewOnly`. No custom envelope, no version byte of ours
   (the unified encoding is already typed, jumbled, and network-bound), no
   hand-rolled parsing.
3. **HARD-D is re-cast, not repealed: "the UFVK never leaves the device
   SILENTLY."** Egress exists at exactly ONE method, named for what it is,
   gated at the reference UI behind the SAME deliberate-action bar as the
   backup-phrase reveal (host re-auth + screenshot protection + a
   privacy-warning screen whose copy contract is spec-level, §3.7 D9), and
   carrying a LOUD API contract for headless hosts. Logs, errors, traces,
   DTOs, streams: still zero UFVK (§5.4 unchanged; the extraction-policy
   deny-list gains the two SANCTIONED crossings it was designed to
   grant deliberately — S204).
4. **Watch-only is a first-class wallet kind, honestly disclosed.** A
   watch-only store writes manifest v3 (same 4-byte layout; `persist` and
   `seed_source` gain explicit `WatchOnly` values): a pre-#397 binary
   opening it fails TYPED (version reject — honest "this build cannot"),
   never a half-open. No seed, no fingerprint row, no mnemonic; spend-class
   operations (send/shield/swap/reveal/backup) refuse with a NEW typed
   `WalletError::WatchOnly` — never `SeedRequired` (a lie: no seed will ever
   arrive). `custody_disclosure` reports the kind so hosts render the badge
   and wipe copy honestly. DB-at-rest custody is UNCHANGED (SQLCipher +
   sealed DB key + crypto-shred wipe — the viewing key is still
   privacy-sensitive at rest).
5. **v1 of the feature keeps the smallest honest surface.** Watch-only
   disables the ENTIRE swap surface (both directions) and every spend path;
   re-export from a watch-only wallet is allowed (identity operation — the
   host supplied the string). Relaxations (e.g. swap Buy-side) are question-
   register items, each its own review.

## Consequences

- Adoption capability: multi-device watch / auditor / portfolio scenarios
  work end-to-end inside the SDK, with the privacy consequence stated at the
  moment of export rather than implied.
- The §1.6 scope-cut line and §1.7 "View-only accounts (UFVK import) —
  DEFER post-1.0" row are struck in favor of §3.7.
- The extraction-policy test's deny-list gains sanctioned `ufvk` crossings;
  the §5.4 forbidden-token log guard is UNCHANGED (still bans `ufvk` values
  in any span/event).
- Two live manifest versions (v2 spending, v3 watch-only) — version byte
  doubles as a capability gate; `read_manifest` stays exact-length,
  exact-version.
- ~~A watch-only rescan re-imports the account from the OLD store's UFVK
  (read-before-swap) instead of re-deriving from seed — one new arm at the
  §3.2f provisioning seam, same atomic-rename discipline.~~ **AMENDED at
  implementation (S224):** a watch-only rescan is REFUSED typed (`WatchOnly`);
  the equivalent is wipe + `create_watch_only` with the SAME UFVK at the lower
  birthday (a UFVK is host-re-suppliable by definition, unlike a seed), which
  keeps the atomic-rename rebuild seed-shaped instead of adding a second
  bespoke arm through the temp-DB window (the None-persistence rescan-refusal
  precedent). The UFVK-capture re-import arm stays a recorded follow-up
  candidate, its own review.
