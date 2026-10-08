# 0559 — Wallet custody: a wallet's identity is a minted id in the wrap artifact's header, not its path

- **Status:** Accepted (stage S2 `custody`, build A's fold, S296, 2026-09-22).
  Reverses the documented decision on `keychain_namespace_for` (FR-13's
  multi-wallet namespace, whose doc comment promised "a genuinely moved
  `db_dir` is a new wallet" — the host re-provisions) and the integration
  guide's "move" sentence. **Supersedes nothing else:** FR-13's per-wallet
  namespace ISOLATION stands unchanged — the namespace's SHAPE (32 lowercase
  hex), its Android alias-prefix scan property and every pinned format row
  are untouched; what changes is the INPUT the namespace is derived from.
- **Date:** 2026-09-22
- **Links:** ADR-0526 (the existence-oracle debt the locator INHERITS, does
  not widen) · `docs/plan/stage-2-the-hosts-lifecycle.md` §3.1 (the
  contract) · the R04 probe
  (`wallet::tests::a_relocated_wallet_directory_keeps_its_custody_identity`)
  · `docs/adjudication/s2-custody/ruling.md` (the adjudication that settled
  the deferral) · Relim's censorship spec D14 clause 7 + their #850 (the
  host's wipe order that made the path unusable as an identity)

## Context

A wallet's seal wrap key lives in the platform keychain under a namespace
derived from the wallet's ABSOLUTE `db_dir` (FR-13 — two wallets in one
process get distinct keychain items). Three facts broke the path as the
identity:

1. **Containers move.** An app update migrates the sandbox; scoped storage
   moves files; the same wallet at a new path opened as
   `KeystoreInconsistent` while the files were perfectly intact — the R04
   probe, red since the 2026-09-20 production-readiness review.
2. **The first host deletes the directory BEFORE our wipe runs.** Relim's
   panic wipe is their Rust `remove_dir_all(wallet_db_dir)` first, then our
   `wipe(config)` from Dart — their spec, their refusal to reorder. With the
   files gone, a path-derived namespace is unreachable: the wrap key would
   outlive every wipe.
3. **The FR-38 ask to reorder their sequence was not ours to grant.** The
   order is the host's own spec; the SDK converges from whatever state it
   finds.

The design review's three refutations killed the first-cut alternative (a
plaintext `custody.id` file beside the DB): no atomic commit across a
separate file plus N seals plus the keychain; no answer when the file is
lost with seals present; no namespace when the host deleted the directory
first. Deriving the id from the seal's random salt is circular (inside the
encrypted artifact, unreadable before the key it would locate).

## Decision

A wallet's custody identity is a random 128-bit `CustodyId` the SDK mints at
create and carries as a **plaintext LOCATOR in a vault-independent outer
frame of the `wrap.artifact` file** (frame byte `0xC1`, disjoint from every
backend's version byte; the backend bytes — the Android envelope, the Apple
SE artifact, the Apple raw marker — and the frozen AAD are unchanged).

- **Integrity is the AEAD's, not the locator's.** The keychain namespace is
  `hex(SHA256(CUSTODY_DOMAIN ‖ id))[..16]` — the same 32-hex shape as
  before, so every pinned format row and the Android alias-prefix scan
  property hold. A tampered locator selects a namespace whose wrap key
  cannot authenticate the artifact: the open is refused typed
  (`WrapArtifactInvalid`), never a silent open of the wrong wallet. A
  tampered locator can only refuse.
- **The path is an INDEX, never the identity.** A path-keyed keychain INDEX
  item (under today's path-derived namespace) records `{id, legacy_ns?,
  state}`; every successful open keeps it current, writing it only when it
  is missing or wrong (on Android an index write is a Keystore key
  generation, so an unconditional write would be one per open). A
  relocation opens under the unchanged id namespace and writes the index at
  the NEW path; the old path's index item is a bounded orphan (one small
  item, no key material) until a wallet is created or wiped at that path.
- **A wipe trusts the keychain over the sandbox.** The index is a keychain
  item; the header is a file anything with write access to the sandbox can
  edit. A wipe severs the header's namespace only when the index agrees
  with it or the header unseals this directory's own seals (the open's own
  check); otherwise the index decides. With the directory already gone the
  index alone resolves the namespace. The index item is deleted LAST (the
  breadcrumb when the host has already deleted the files), and a second
  wipe call is a no-op success.
- **Migration has ONE commit point.** A pre-stage wallet migrates at first
  open: mint the id; write the index `{id, legacy_ns, state: pending}` (a
  re-run REUSES this path's own pending id — no namespace is ever orphaned
  by a fresh mint — and ONLY that: an entry left by another wallet is never
  borrowed); `store_wrap_key` under the id namespace with the legacy item
  left live; **the commit: the single `write_atomic` of the wrap artifact
  carrying the id.** The legacy purge and the index `done` write are
  DEFERRED to the next successful open under the id namespace and are
  best-effort there: a purge that fails (a wedged keystore) is logged, stays
  `pending` — so a wipe still severs both namespaces — and is retried at the
  following open; it never refuses an open of a wallet that has already
  committed. Every crash window converges on the next run, with the same id.
- **The wipe's reach is this wallet's alone**, and the core's `flock`
  (`WalletOpen`) is the authoritative gate for every wipe caller.

## Consequences

- A host may move a wallet's container without re-provisioning; two
  containers remain two wallets; a wipe converges from a bare `WalletConfig`
  with no files under it. The host contract sentence on
  `keychain_namespace_for` (a byte-stable `db_dir` string across
  create/open) now binds only the INDEX and the legacy migration input, not
  the identity.
- iOS wallets have an identity too: the locator's frame is parsed by the
  store, which sees one opaque `WrapArtifact` across the three backend
  shapes. (The S295 contract's seam line named only the Android envelope —
  `envelope.rs`'s `WrapArtifactV1` — a label corrected at the fold, §6.7 of
  the stage plan.)
- The identifier is never logged and never crosses the FFI bare: the type
  has no `Display`, no DTO exposes it, no tracing field carries it; a host
  correlates wallets by `db_dir`, as today. The locator inherits ADR-0526's
  existence-oracle debt without widening it (a directory that IS a wallet
  already says so; the header adds the id, not the fact).
- `CUSTODY_DOMAIN` is FROZEN, append-only, like every derivation label.
- **Residuals, stated, not closed here.** (1) A keychain call that wedges
  is still unbounded: the SDK has no per-call bound on a native keystore
  call (the S2 contract's row 8 assumed one existed; it does not), so a
  host's own timeout cannot cancel it. Owed to its own item. (2) A wallet
  that MOVES while its migration is still `pending` leaves the legacy item
  (and its pending index) at the old path: the new path's index says `done`,
  so neither the next open nor a later wipe there reaches it. It needs a
  crash inside the migration and a relocation before the next open; the
  seals it could unwrap are deleted by that wipe, so what survives is a key
  with nothing on disk to open.
