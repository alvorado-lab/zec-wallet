//! §4.3a — platform-vault custody of the seed-seal wrap key.
//!
//! Two-layer model: the §4.2a envelope (seal.rs) is THE portable sealing
//! construction on every platform; the vault's only job is custody of the
//! 32-byte `SealKey`. Per platform:
//!
//! - **Android** — a non-exportable AES-256-GCM key generated INSIDE
//!   `AndroidKeyStore` wraps the `SealKey`; the wrap artifact (envelope.rs)
//!   is persisted beside the sealed blob. The adapter lives in the
//!   `zec_wallet` bridge crate (the JNI exports must live in the cdylib that
//!   `System.loadLibrary` loads, and this crate stays
//!   `#![forbid(unsafe_code)]`).
//! - **iOS/macOS** — no wrap layer at all: the `SealKey` IS a keychain item
//!   (`apple.rs`), `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`.
//! - **Tests** — `testvault.rs` simulates the Android shape with a real AEAD
//!   (ChaCha20Poly1305 IETF, same 12-byte IV), so the framing, AAD binding,
//!   rotation, and failure-posture logic under test is the REAL logic; only
//!   the cipher call differs from the device.
//!
//! Failure posture (§4.3a, S6 security fold): **no silent degrade on any
//! axis** — no vault ⇒ `VaultAbsent`, fail-closed, BEFORE a seed is sealed;
//! a software-backed tier is surfaced as data the host must render, never
//! just recorded. There is deliberately NO constructor for an in-memory
//! "vault" outside `cfg(test)` — the forbidden this-session fallback is
//! structurally unreachable, which is what
//! `keychain_unavailable_yields_typed_error_not_fallback` pins.

// The wrap-artifact codec. Two halves: the v1 wrap-key artifact (`WRAP_KEY_DOMAIN`,
// `WrapArtifactV1`, `encode_artifact`/`parse_artifact`, …), whose production
// consumer is the Android vault (`android.rs`, `cfg(target_os = "android")`) and
// which on every other host is reached only through the `cfg(test)` test vault;
// and the Apple SE artifact (`encode_apple_se_artifact`/`parse_apple_se_artifact`,
// `APPLE_SE_ECIES_LEN`, …), which `apple.rs` reads on every store and open. So
// the allow sits on the NINE Android-only items, not on the module (the P3-7
// sweep's first form put it here, and the security review found the module-level
// attribute hiding a new dead item in the Apple half on the only host that runs
// the clippy lane); a dead item in either half is a warning on this host now.
pub(crate) mod bounded;
pub(crate) mod envelope;
pub(crate) mod selftest;

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) mod apple;

#[cfg(target_os = "android")]
pub(crate) mod android;

#[cfg(test)]
pub(crate) mod testvault;

pub(crate) use bounded::BoundedVault;

use std::sync::Arc;

use zeroize::Zeroizing;

use crate::error::WalletError;
use crate::seal::{self, SealKey, SeedPayload, WalletDbKey};

/// Where the wrap key actually lives — recorded AND surfaced (§4.3a per-tier
/// honesty: one blanket claim would overstate the bottom tier).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive] // G2 enum policy: hosts must default-arm
pub enum VaultTier {
    /// Dedicated secure element (Android StrongBox).
    StrongBox,
    /// Hardware-isolated keystore (TEE) — Android default tier.
    Tee,
    /// OS-keystore-daemon-mediated, software root of trust. Honestly weaker:
    /// surfaced as DEGRADED, the SDK refuses to label it "vault-protected".
    SoftwareKeystore,
    /// Apple keychain item, SealKey stored RAW (OS-encrypted at rest,
    /// code-sign-ACL'd; `WhenUnlockedThisDeviceOnly` — never synced, never in
    /// backups). The chunk-1 tier; on Apple it is the EXPLICIT dev/sim fallback
    /// only (FR-14 chunk 2 — `AppleSecureEnclave` is the production default).
    /// Deleting the item is a best-effort erase (a flash-recoverable residual
    /// survives) — `erase_assurance()` is `BestEffort`.
    AppleKeychain,
    /// Apple Secure Enclave (FR-14 chunk 2): the SealKey is ECIES-wrapped under a
    /// NON-EXTRACTABLE SE key; the wrapped blob is on disk, the wrap key is in
    /// hardware. Deleting the wallet deletes the SE key — `erase_assurance()` is
    /// `HardwareKeyDeleted`, which claims no permanence (ADR-0571).
    AppleSecureEnclave,
}

/// What deleting a wallet's custody key does, per tier (ADR-0571). It says what
/// the deletion DID, never that the key cannot come back: neither platform
/// establishes that for our key (Android only for a rollback-resistant key, which
/// we neither request nor check; Apple publishes no per-key equivalent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive] // G2 enum policy: hosts must default-arm (and read an unknown value as `BestEffort`)
pub enum EraseAssurance {
    /// The key was held by secure hardware (Android StrongBox or TEE, the Apple
    /// Secure Enclave), never usable outside it, and has been deleted.
    HardwareKeyDeleted,
    /// The key was stored as data (a raw keychain item, a software keystore) or
    /// there is no keystore, and has been deleted; a copy may remain in storage
    /// until the device reclaims it.
    BestEffort,
}

impl VaultTier {
    /// The §4.3a `VaultTierDegraded` predicate — ONE source of truth for
    /// "must the host render a degraded-custody warning".
    pub fn software_backed(self) -> bool {
        matches!(self, Self::SoftwareKeystore)
    }

    /// FR-14 / ADR-0571 — what deleting THIS tier's custody key does. ONE source
    /// of truth for the `Wallet::wipe` disclosure.
    ///
    /// `HardwareKeyDeleted` ONLY for hardware-held custody — Android
    /// `StrongBox`/`Tee` and `AppleSecureEnclave`. `AppleKeychain` (raw item) and
    /// `SoftwareKeystore` are `BestEffort`: the deleted key may linger in flash free
    /// pages, recoverable under a coerced unlock (AV2).
    ///
    /// The match is EXHAUSTIVE with no wildcard ON PURPOSE: although `VaultTier` is
    /// `#[non_exhaustive]`, within this crate a new variant added here is a COMPILE ERROR
    /// at this arm — forcing the author to make an explicit, reviewed decision rather
    /// than inheriting a silent default. Overclaiming custody is the money-safety
    /// failure direction, so a tier that is not hardware-held must choose `BestEffort`.
    /// (External crates that match this enum still need their own wildcard arm.)
    pub fn erase_assurance(self) -> EraseAssurance {
        match self {
            Self::StrongBox | Self::Tee | Self::AppleSecureEnclave => {
                EraseAssurance::HardwareKeyDeleted
            }
            Self::SoftwareKeystore | Self::AppleKeychain => EraseAssurance::BestEffort,
        }
    }
}

/// Non-secret custody disclosure for a wallet at rest — the PRODUCTION face of
/// the §4.3a per-tier honesty, surfaced as data (the disclosure-as-data pattern
/// §2.6). It is the runtime consumer of [`VaultTier::erase_assurance`]: a host
/// renders an honest pre-[`wipe`](crate::Wallet::wipe) confirmation (a
/// hardware-held key deleted vs best-effort removal) and an ambient custody
/// badge from it, BEFORE the destructive verb. Distinct from the diagnostic
/// [`seed_custody_selftest`](crate::seed_custody_selftest), which provisions
/// selftest identities and is explicitly NOT shippable host UX — this query
/// touches no seed, no key material, and unseals nothing; it reads only the
/// measured vault tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CustodyDisclosure {
    /// The measured custody tier, or `None` when the platform has no key vault
    /// (a headless desktop — `VaultAbsent`; the §9-deferred desktop keystore).
    pub tier: Option<VaultTier>,
    /// What deleting this wallet's custody key does: the
    /// [`VaultTier::erase_assurance`] SSOT, surfaced as data. `BestEffort` when
    /// `tier` is `None` (a `wipe` there is honest file removal — a
    /// flash-recoverable residual survives until the OS reclaims the pages).
    /// No value claims the key can never come back (ADR-0571).
    pub erase_assurance: EraseAssurance,
    /// The §4.3a degraded-custody predicate ([`VaultTier::software_backed`]):
    /// the host must render a degraded-custody warning. `false` when `tier` is
    /// `None` — "no keystore" is a DISTINCT signal from "software-rooted
    /// keystore" (the absent `tier` carries it), never silently folded to
    /// "degraded".
    pub degraded: bool,
}

impl CustodyDisclosure {
    /// Derive the disclosure from a measured tier — the ONE place the three
    /// fields are computed from [`VaultTier`], so `erase_assurance` can never
    /// drift from `tier`.
    pub(crate) fn from_tier(tier: VaultTier) -> Self {
        Self {
            tier: Some(tier),
            erase_assurance: tier.erase_assurance(),
            degraded: tier.software_backed(),
        }
    }

    /// The headless-desktop disclosure (no platform key vault): honest
    /// best-effort (`EraseAssurance::BestEffort`), not degraded-keystore.
    pub(crate) fn absent() -> Self {
        Self {
            tier: None,
            erase_assurance: EraseAssurance::BestEffort,
            degraded: false,
        }
    }
}

/// Non-secret custody status returned by every vault operation — the
/// disclosure-as-data pattern (§2.6): the host receives it on the open path
/// and must render degradation; it cannot un-receive it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VaultStatus {
    pub(crate) tier: VaultTier,
}

impl VaultStatus {
    /// Read by the custody selftest (compiled only where a vault exists) and
    /// the tests.
    #[cfg(any(test, target_os = "android", target_os = "macos", target_os = "ios"))]
    pub(crate) fn degraded(self) -> bool {
        self.tier.software_backed()
    }
}

/// Persistable wrap-key locator: an OPTIONAL vault-independent custody frame
/// (S2) around the backend's own artifact bytes. The frame carries the
/// wallet's [`CustodyId`](crate::custody::CustodyId) — a plaintext locator read BEFORE any key, from
/// which the keychain namespace is derived — and every backend's bytes ride
/// it UNCHANGED (`as_bytes()` is what a vault sees; `file_bytes()` is what
/// lands on disk). Android: the envelope.rs artifact; Apple: a 1-byte version
/// marker (the key lives IN the vault, nothing secret on disk).
/// Validate-before-use: size-capped and frame-checked on construction —
/// bytes from disk never reach a backend oversized or half-framed.
/// `Clone` (FR-47): no key material rides it, and the bounded decorator hands
/// its worker an owned copy.
#[derive(Clone)]
pub(crate) struct WrapArtifact {
    /// The custody locator from the outer frame (`None` = a pre-stage file,
    /// bare backend bytes — the pre-S2 shape, still read today).
    custody: Option<crate::custody::CustodyId>,
    /// The backend's own artifact bytes, handed to the vault verbatim.
    backend: Vec<u8>,
}

/// Hard cap on artifact bytes accepted from disk. The v1 Android artifact
/// is 65 bytes and the Apple marker is 1; the custody frame adds 18; 256
/// leaves headroom for a future /v2 without admitting megabyte garbage
/// (§4.6 size-cap discipline).
pub(crate) const WRAP_ARTIFACT_MAX_BYTES: usize = 256;

impl WrapArtifact {
    /// The ONE door every backend's bytes pass through on the way in from
    /// disk (S2): a file starting with the custody frame byte is parsed as
    /// `[frame_ver][id_len (0|16)][id?][backend bytes]`; anything else is a
    /// pre-stage file — the WHOLE input is the backend artifact, locator
    /// `None`. A malformed frame (wrong id length, empty backend) is
    /// `WrapArtifactInvalid` before any vault call.
    pub(crate) fn from_bytes(bytes: Vec<u8>) -> Result<Self, WalletError> {
        use crate::custody::{CUSTODY_FRAME_V1, CUSTODY_ID_LEN, CustodyId};
        if bytes.is_empty() || bytes.len() > WRAP_ARTIFACT_MAX_BYTES {
            return Err(WalletError::WrapArtifactInvalid);
        }
        if bytes[0] != CUSTODY_FRAME_V1 {
            // A pre-stage file: bare backend bytes, byte-identical to the
            // pre-S2 shape (the disjoint frame byte is what makes the two
            // distinguishable — see CUSTODY_FRAME_V1).
            return Ok(Self {
                custody: None,
                backend: bytes,
            });
        }
        let (_, rest) = bytes.split_first().expect("len checked above");
        let (&id_len, rest) = rest.split_first().ok_or(WalletError::WrapArtifactInvalid)?;
        let (id_bytes, backend) = match usize::from(id_len) {
            0 => (&[][..], rest),
            CUSTODY_ID_LEN => rest.split_at(CUSTODY_ID_LEN),
            _ => return Err(WalletError::WrapArtifactInvalid),
        };
        if backend.is_empty() {
            return Err(WalletError::WrapArtifactInvalid);
        }
        let custody = if id_bytes.is_empty() {
            None
        } else {
            let mut raw = [0u8; CUSTODY_ID_LEN];
            raw.copy_from_slice(id_bytes);
            Some(CustodyId::from_bytes(raw))
        };
        Ok(Self {
            custody,
            backend: backend.to_vec(),
        })
    }

    /// The BACKEND's artifact bytes — the vault-facing view. A backend
    /// parsing these never sees the custody frame. Compiled where a backend
    /// is: the Android and Apple vaults, and the `cfg(test)` test vault.
    #[cfg(any(test, target_os = "android", target_os = "macos", target_os = "ios"))]
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.backend
    }

    /// The FILE form: the custody frame (when a locator is set) around the
    /// backend bytes; with no locator, the bare backend bytes — byte-identical
    /// to the pre-S2 file shape.
    pub(crate) fn file_bytes(&self) -> Vec<u8> {
        use crate::custody::{CUSTODY_FRAME_V1, CUSTODY_ID_LEN};
        match &self.custody {
            None => self.backend.clone(),
            Some(id) => {
                let mut out = Vec::with_capacity(1 + 1 + CUSTODY_ID_LEN + self.backend.len());
                out.push(CUSTODY_FRAME_V1);
                out.push(CUSTODY_ID_LEN as u8);
                out.extend_from_slice(id.as_bytes());
                out.extend_from_slice(&self.backend);
                out
            }
        }
    }

    /// The custody locator from the header, if the file carries one.
    pub(crate) fn custody_id(&self) -> Option<crate::custody::CustodyId> {
        self.custody
    }

    /// Set the locator on backend-built bytes (the store frames a freshly
    /// wrapped artifact before its single commit `write_atomic`).
    pub(crate) fn with_custody_id(mut self, id: &crate::custody::CustodyId) -> Self {
        self.custody = Some(*id);
        self
    }

    // SECURITY: backend-built artifacts ONLY (fixed-size encoder output /
    // the 1-byte Apple marker) — NEVER bytes read from disk; the store
    // layer's loader must go through `from_bytes` (the size-capped,
    // validated constructor). Named loudly so a future persistence adapter
    // cannot reach for it by accident. Compiled where a backend is (the
    // Android and Apple vaults, the `cfg(test)` test vault).
    #[cfg(any(test, target_os = "android", target_os = "macos", target_os = "ios"))]
    pub(crate) fn from_freshly_wrapped(bytes: Vec<u8>) -> Self {
        Self {
            custody: None,
            backend: bytes,
        }
    }
}

/// A validated per-wallet keychain namespace (FR-13): a fixed-length 32-char
/// lowercase-hex token, the ONLY shape the platform vaults accept. Constructed
/// solely through [`Self::new`] (the production producers are
/// `keychain_namespace_for` and the S2 custody derivation in `custody.rs`), so
/// the LOAD-BEARING isolation invariant is unrepresentable-if-violated: the
/// Android `max_generation` alias-prefix scan stays wallet-local ONLY because
/// every namespace is equal length (no namespace's prefix is a prefix of
/// another's), and the Apple account suffix is well-formed. A `debug_assert`
/// would be stripped in release; the type enforces it in EVERY build, with no
/// panic (the validate-before-use idiom, sibling to [`WrapArtifact::from_bytes`]).
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct KeychainNamespace(String);

impl KeychainNamespace {
    /// Build a namespace, validating the 32-char lowercase-hex invariant. Returns
    /// `None` on violation so a malformed namespace can NEVER reach a vault. The
    /// production caller (`keychain_namespace_for`) feeds `hex::encode` of 16 bytes,
    /// which is always valid — so its `.expect` is genuinely unreachable.
    pub(crate) fn new(s: String) -> Option<Self> {
        let ok = s.len() == 32
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        ok.then_some(Self(s))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// The platform-vault seam (§4.3a). Pattern parity with local-storage's
/// `KeychainBackend`, never an import of it (A1: no `relim-*` deps).
///
/// Synchronous BY DESIGN: backends are short blocking native calls
/// (JNI/Security.framework); the async caller (the wallet handle, gated
/// behind W3-inc-2) invokes through `spawn_blocking` — same discipline as
/// the USK proving closure (§4.1).
///
/// Rotation is two-phase against the §6.3 kill windows:
/// `rotate_wrap_key` wraps under a NEW alias generation and returns the new
/// artifact; the caller persists it (write + fsync + rename), THEN calls
/// `finish_rotation` to delete the superseded alias. A crash in between
/// leaves a dangling old alias — harmless (the artifact says which
/// generation to use) and cleaned by the next `finish_rotation`/`rotate`.
pub(crate) trait KeychainPort: Send + Sync {
    /// Fail-closed gate: `VaultAbsent` when no vault exists at all —
    /// checked BEFORE any key is generated or any seed sealed. MUST NOT
    /// create vault state.
    fn probe(&self) -> Result<(), WalletError>;

    /// The MEASURED tier (§4.3a per-tier honesty). On Android this
    /// introspects the live key (`KeyInfo`) — accurate only once vault
    /// material exists, which is why the orchestration reads it AFTER
    /// store/load, never from a prediction.
    fn tier(&self) -> Result<VaultTier, WalletError>;

    /// Take custody of a fresh wrap key, bound to `sealed_blob` (§4.3a AAD
    /// binding; vacuous on Apple where the ACL is the binding).
    ///
    /// BY VALUE (FR-47): the bounded decorator moves the key into its worker's
    /// job, and a caller that stops waiting drops it there. The `SealKey` is
    /// heap-resident, so the move copies a pointer, never the key. No caller
    /// uses the key after custody.
    fn store_wrap_key(&self, key: SealKey, sealed_blob: &[u8])
    -> Result<WrapArtifact, WalletError>;

    /// Recover the wrap key. Substitution/corruption of either artifact or
    /// blob fails as `WrapArtifactInvalid` — loud, layer-attributed.
    fn load_wrap_key(
        &self,
        artifact: &WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<SealKey, WalletError>;

    /// Re-custody the same wrap key under a fresh alias generation (one
    /// alias = one encryption, ever — IV reuse structurally impossible).
    /// Backends without aliases (Apple, where rotation has nothing to
    /// rotate) return the equivalent current artifact.
    ///
    /// STAGED (P3-7): no production caller until the rotation/wipe chunk. Not
    /// deleted, because every vault implements it (Android, Apple, the test
    /// vault) and this host cannot compile two of them — owed to that chunk.
    #[allow(dead_code)]
    fn rotate_wrap_key(
        &self,
        artifact: &WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<WrapArtifact, WalletError>;

    /// Complete a rotation: delete the SUPERSEDED vault material named by
    /// `old`. No-op where rotation is vacuous or `old` already gone.
    /// STAGED with `rotate_wrap_key` (P3-7).
    #[allow(dead_code)]
    fn finish_rotation(&self, old: &WrapArtifact, new: &WrapArtifact) -> Result<(), WalletError>;

    /// Sever custody entirely: after this, no artifact opens the seal
    /// (`wrap_key_wipe_leaves_seal_unopenable`). Idempotent.
    ///
    /// Its caller, `SealedSeedVault::wipe`, is compiled only where a vault
    /// exists; an allow, not a cfg, because a cfg on a trait method would have
    /// to be repeated on every implementor, `BoundedVault` included (built on
    /// every host).
    #[cfg_attr(
        not(any(test, target_os = "android", target_os = "macos", target_os = "ios")),
        allow(dead_code)
    )]
    fn delete_wrap_key(&self, artifact: &WrapArtifact) -> Result<(), WalletError>;

    /// FR-14 crypto-shred: sever ALL vault material under THIS vault's baked-in
    /// namespace (FR-13), **artifact-independent**. Returns the COUNT of items
    /// actually severed — the verify-real-sever signal (`store::destroy` fails
    /// CLOSED when a non-empty store's purge severs ZERO, so a `db_dir`
    /// path-variant that mis-derives the namespace can never silently report a
    /// wipe that left the real wrap key live + the seals flash-recoverable).
    /// Distinct from [`Self::delete_wrap_key`] (one generation BY artifact):
    /// purge also reaches a **failed-provision orphan alias** that has no on-disk
    /// artifact (the `store.rs` provision Step-1 seam note) — a namespace scan can
    /// reach it; an artifact-keyed delete structurally cannot. Idempotent (no
    /// material ⇒ `Ok(0)`). A non-transient partial failure returns `Err` (the
    /// caller must NOT delete files until the sever completes — retry re-purges).
    fn purge_namespace(&self) -> Result<usize, WalletError>;

    /// S2 custody — write the path-keyed INDEX item under this vault's baked-in
    /// namespace (a small locator record, NO key material): `{id, legacy_ns?,
    /// state}`, so a wipe can resolve the custody namespace with the wallet's
    /// directory already deleted by the host. Upsert: replaces any prior item
    /// under the namespace (the rewrite-after-relocation / pending→done
    /// transition). Size-capped by the codec's own invariant.
    fn store_index(&self, entry: &crate::custody::CustodyIndexEntry) -> Result<(), WalletError>;

    /// S2 custody — read the index item under this namespace (`Ok(None)` when
    /// no item exists — a pre-stage wallet, or a namespace nothing was ever
    /// provisioned under). A PRESENT but malformed item is a typed error, never
    /// a guess; the callers' documented posture is fail-open-with-rewrite at
    /// open and fall-to-header at destroy.
    fn load_index(&self) -> Result<Option<crate::custody::CustodyIndexEntry>, WalletError>;

    /// S2 custody — delete the index item under this namespace. Idempotent
    /// (already-gone is the terminal no-op). The WIPE deletes it LAST — it is
    /// the breadcrumb that finds the namespace when the directory is gone.
    fn delete_index(&self) -> Result<(), WalletError>;
}

/// What a resolver hands back: the vault it resolved, EITHER borrowed (a
/// pre-built vault the seam already owns) OR owned (constructed per namespace,
/// the platform shape). One small type because the two resolver families hold
/// their vaults differently and the store's borrows must work through both.
/// (`Borrowed` is constructed only by the test-seam adapters — dead in a
/// non-test build, like they are.)
pub(crate) enum ResolvedVault<'a> {
    #[cfg_attr(not(test), allow(dead_code))]
    Borrowed(&'a dyn KeychainPort),
    Owned(Arc<dyn KeychainPort>),
}

impl ResolvedVault<'_> {
    pub(crate) fn as_port(&self) -> &dyn KeychainPort {
        match self {
            Self::Borrowed(v) => *v,
            Self::Owned(v) => v.as_ref(),
        }
    }
}

/// S2 custody — the namespace→vault factory: the wallet reads the wrap
/// artifact's header FIRST, then asks the resolver for the vault that
/// custodies the namespace the header names (a pre-built vault cannot do
/// this — its namespace is baked in at construction, which is exactly why
/// the `*_with_vault` seams needed a replacement).
pub(crate) trait VaultResolver: Send + Sync {
    /// The vault for `namespace`. Fails closed (`VaultAbsent` on a platform
    /// with no keystore) exactly like the wallet layer's `platform_vault` —
    /// the resolver is the plural of that one call.
    fn vault_for<'a>(
        &'a self,
        namespace: &KeychainNamespace,
    ) -> Result<ResolvedVault<'a>, WalletError>;
}

/// The fixed pre-built-vault resolver: every namespace resolves to the SAME
/// vault (the vault's own baked-in namespace decides where items land — the
/// base tree's shape). This backs the `*_with_vault` test seams so the
/// pre-existing fixtures keep compiling; a test that wants real
/// per-namespace resolution builds a resolver over a shared backend store.
/// CONFLATION CAVEAT (load-bearing for the migration): when the legacy and
/// id namespaces resolve to one vault, the migration's step-(5) legacy purge
/// would sever the JUST-written custody — `store` skips it for the same
/// instance (compared by pointer) for exactly that reason.
/// TEST-SEAM adapter (dead in a non-test build — production resolvers are
/// per-namespace, never fixed).
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct FixedVault(pub(crate) Arc<dyn KeychainPort>);

impl VaultResolver for FixedVault {
    fn vault_for<'a>(
        &'a self,
        _namespace: &KeychainNamespace,
    ) -> Result<ResolvedVault<'a>, WalletError> {
        Ok(ResolvedVault::Borrowed(self.0.as_ref()))
    }
}

/// [`FixedVault`] over a borrowed vault — the adapter behind the kept
/// `store::open` / `store::create_or_repair` / `store::destroy` signatures.
/// TEST-SEAM adapter (dead in a non-test build).
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct BorrowedVault<'a>(pub(crate) &'a dyn KeychainPort);

impl VaultResolver for BorrowedVault<'_> {
    fn vault_for<'a>(
        &'a self,
        _namespace: &KeychainNamespace,
    ) -> Result<ResolvedVault<'a>, WalletError> {
        Ok(ResolvedVault::Borrowed(self.0))
    }
}

/// What `store` hands back: everything the store layer (W3-inc-2, gated)
/// must persist, plus the custody status the host must see.
/// `store`/`load`/`wipe` below and this struct are compiled where their
/// callers are: the custody selftest (a vault platform) and the tests.
#[cfg(any(test, target_os = "android", target_os = "macos", target_os = "ios"))]
pub(crate) struct StoredSeal {
    pub(crate) sealed_blob: Vec<u8>,
    pub(crate) wrap_artifact: WrapArtifact,
    pub(crate) status: VaultStatus,
}

/// What `store_wallet` hands back (§6.3 provisioning): BOTH sealed blobs and
/// the ONE wrap artifact custodying the single `SealKey` that sealed them.
///
/// **One SealKey, two blobs (W3-inc-2b money-lens pin):** the seed seal (§4.2a)
/// and the DB-key seal (§4.3) are sealed under the SAME custodied wrap key — a
/// SECOND `SealKey` for the DB key would double the orphan / `KeystoreInconsistent`
/// surface (two independent custody lifetimes that can diverge under a §6.3 kill).
/// The wrap artifact's AAD binds to the PRIMARY blob (the seed blob when present,
/// else the DB-key blob); the secondary blob's integrity rests on its own AEAD
/// tag + distinct domain (a swapped secondary blob fails `SealInvalid` at unseal —
/// loud, never a silent open).
pub(crate) struct StoredWalletSeals {
    /// SealedKeychain mode only; `None` on the host's `SeedPersistence::None` path
    /// (where the seed is never persisted — only the random DB key is sealed,
    /// since the DB is unconditionally encrypted at rest, §4.3).
    pub(crate) seed_blob: Option<Vec<u8>>,
    pub(crate) dbkey_blob: Vec<u8>,
    pub(crate) wrap_artifact: WrapArtifact,
    /// Handed back beside the blobs; no reader today (P3-7 measured it — the
    /// handle's custody status comes from `load_wallet`, not from provisioning).
    #[allow(dead_code)]
    pub(crate) status: VaultStatus,
}

/// What `load_wallet` recovers from the on-disk blobs + vault custody.
pub(crate) struct LoadedWalletSeals {
    pub(crate) seed: Option<SeedPayload>,
    pub(crate) db_key: WalletDbKey,
    pub(crate) status: VaultStatus,
}

/// §4.3a orchestration over seal.rs + a `KeychainPort` — the unit the
/// wallet handle consumes once W3-inc-2 unblocks. Owns the ORDER of
/// operations (probe-first fail-closed; seal-then-wrap so the AAD can bind).
pub(crate) struct SealedSeedVault<'a> {
    vault: &'a dyn KeychainPort,
}

impl<'a> SealedSeedVault<'a> {
    pub(crate) fn new(vault: &'a dyn KeychainPort) -> Self {
        Self { vault }
    }

    /// Seal `payload` and place the wrap key in vault custody.
    /// Probe-first: a missing vault fails BEFORE a SealKey exists anywhere
    /// (`keychain_unavailable_yields_typed_error_not_fallback` pins that no
    /// partial state precedes the typed error).
    #[cfg(any(test, target_os = "android", target_os = "macos", target_os = "ios"))]
    pub(crate) fn store(&self, payload: &SeedPayload) -> Result<StoredSeal, WalletError> {
        self.vault.probe()?;
        let key = SealKey::generate();
        let sealed_blob = seal::seal(&key, payload)?;
        let wrap_artifact = self.vault.store_wrap_key(key, &sealed_blob)?;
        // Tier read AFTER the key exists: on Android only a live key can be
        // introspected — a predicted tier would be an overclaim (§4.3a).
        let tier = self.vault.tier()?;
        Ok(StoredSeal {
            sealed_blob,
            wrap_artifact,
            status: VaultStatus { tier },
        })
    }

    /// Recover the payload. The wrap layer fails as `WrapArtifactInvalid`
    /// (layer-attributed); the seed envelope keeps its own §4.2a collapse
    /// (`SealInvalid`) — two layers, two honest error classes.
    #[cfg(any(test, target_os = "android", target_os = "macos", target_os = "ios"))]
    pub(crate) fn load(
        &self,
        sealed_blob: &[u8],
        artifact: &WrapArtifact,
    ) -> Result<(SeedPayload, VaultStatus), WalletError> {
        self.vault.probe()?;
        let key = self.vault.load_wrap_key(artifact, sealed_blob)?;
        let payload = seal::unseal(&key, sealed_blob)?;
        let tier = self.vault.tier()?;
        Ok((payload, VaultStatus { tier }))
    }

    /// Provision custody for a wallet (§6.3): seal the seed (when present) AND
    /// the random DB key under ONE freshly generated `SealKey`, place that key
    /// in vault custody, and return both blobs + the wrap artifact.
    ///
    /// Probe-first (fail-closed: a missing vault fails BEFORE any key exists).
    /// The two seals are produced TOGETHER, in memory, before the single
    /// `store_wrap_key` custody call — so there is no durable state in which the
    /// seed is sealed but the DB key is not (the §6.3 split-brain the money-lens
    /// pin guards against is eliminated BY CONSTRUCTION, not by a resume step).
    /// The wrap artifact AAD-binds to the primary blob (seed if present, else
    /// DB-key — the secondary rides its own AEAD tag + domain).
    pub(crate) fn store_wallet(
        &self,
        seed: Option<&SeedPayload>,
        db_key: &WalletDbKey,
    ) -> Result<StoredWalletSeals, WalletError> {
        self.vault.probe()?;
        let key = SealKey::generate();
        let seed_blob = match seed {
            Some(payload) => Some(seal::seal(&key, payload)?),
            None => None,
        };
        let dbkey_blob = seal::seal_db_key(&key, db_key)?;
        // AAD binding: the primary blob is the seed seal when present, else the
        // DB-key seal (the host's SeedPersistence::None path seals only the DB key).
        let primary: &[u8] = seed_blob.as_deref().unwrap_or(&dbkey_blob);
        let wrap_artifact = self.vault.store_wrap_key(key, primary)?;
        // Tier read AFTER the key exists (Android can only introspect a live key).
        let tier = self.vault.tier()?;
        Ok(StoredWalletSeals {
            seed_blob,
            dbkey_blob,
            wrap_artifact,
            status: VaultStatus { tier },
        })
    }

    /// Recover a wallet's custody (§6.3 open / repair): load the wrap key and
    /// unseal the DB key (always) and the seed (when a seed blob is present).
    /// The wrap layer fails `WrapArtifactInvalid` (layer-attributed); a swapped
    /// secondary blob fails `SealInvalid` at unseal — both loud, never silent.
    pub(crate) fn load_wallet(
        &self,
        seed_blob: Option<&[u8]>,
        dbkey_blob: &[u8],
        artifact: &WrapArtifact,
    ) -> Result<LoadedWalletSeals, WalletError> {
        self.vault.probe()?;
        let primary: &[u8] = seed_blob.unwrap_or(dbkey_blob);
        let key = self.vault.load_wrap_key(artifact, primary)?;
        let db_key = seal::unseal_db_key(&key, dbkey_blob)?;
        let seed = match seed_blob {
            Some(blob) => Some(seal::unseal(&key, blob)?),
            None => None,
        };
        let tier = self.vault.tier()?;
        Ok(LoadedWalletSeals {
            seed,
            db_key,
            status: VaultStatus { tier },
        })
    }

    /// Two-phase rotation; see the port docs for the crash-window contract.
    /// STAGED (P3-7): driven by this module's tests only; no production caller.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn rotate(
        &self,
        sealed_blob: &[u8],
        artifact: &WrapArtifact,
    ) -> Result<WrapArtifact, WalletError> {
        self.vault.rotate_wrap_key(artifact, sealed_blob)
    }

    /// STAGED with `rotate` (P3-7).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn finish_rotation(
        &self,
        old: &WrapArtifact,
        new: &WrapArtifact,
    ) -> Result<(), WalletError> {
        self.vault.finish_rotation(old, new)
    }

    /// Sever the wrap key. Blob deletion is the store layer's job (§6.3
    /// wipe order) — but once this returns Ok, the blob is ciphertext
    /// nobody can open.
    #[cfg(any(test, target_os = "android", target_os = "macos", target_os = "ios"))]
    pub(crate) fn wipe(&self, artifact: &WrapArtifact) -> Result<(), WalletError> {
        self.vault.delete_wrap_key(artifact)
    }

    /// FR-14 crypto-shred: sever ALL wrap-key material under this wallet's
    /// namespace, ARTIFACT-FREE (supersedes `Self::wipe` for the
    /// `Wallet::wipe` / panic-wipe path; `wipe` stays the artifact-keyed
    /// single-generation delete the rotation-finish path uses). Keychain-FIRST:
    /// once this returns Ok, the §4.2a SealKey is deleted, so this key store can no
    /// longer open any on-disk seal or the DB ciphertext (`erase_assurance` says how
    /// strongly, ADR-0571) — file deletion (`store::destroy`) is best-effort cleanup. Returns the count severed (the
    /// verify-real-sever signal forwarded from the port).
    pub(crate) fn purge(&self) -> Result<usize, WalletError> {
        self.vault.purge_namespace()
    }
}

/// Best-effort zeroize helper for key bytes recovered from a vault backend
/// (the §4.3a honest-transit discipline applied on the Rust side too).
/// Consumed by the Android vault and the `cfg(test)` test vault only (P3-7).
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) fn seal_key_from_vault(bytes: Zeroizing<Vec<u8>>) -> Result<SealKey, WalletError> {
    // A wrong-shape key from a vault is a wrap-layer failure (the artifact
    // authenticated but yielded garbage shape — corruption inside the vault
    // item), attributed to the wrap layer, not the seed seal.
    SealKey::from_bytes(bytes).map_err(|_| WalletError::WrapArtifactInvalid)
}

#[cfg(test)]
mod tests {
    use super::testvault::TestVault;
    use super::*;
    use crate::custody::CustodyId;
    use crate::error::WalletError;
    fn payload() -> SeedPayload {
        SeedPayload::new(
            Zeroizing::new(vec![0x42; 32]),
            Some(Zeroizing::new("ride glove window".to_owned())),
        )
        .expect("valid payload")
    }

    // Crypto-rules: the loaded custody carries the seed + DB key — pinned
    // non-Clone/non-Debug at compile time (the seed seal / db-key seal stay
    // unreachable from any container with a transitive Clone/Debug).
    static_assertions::assert_not_impl_any!(LoadedWalletSeals: Clone, std::fmt::Debug);

    /// S2 custody frame: a framed artifact round-trips the locator and hands
    /// the backend bytes to the vault UNCHANGED; a pre-stage file passes
    /// through verbatim with no locator; malformed frames die
    /// validate-before-use. A backend artifact can never masquerade as framed
    /// (its version bytes are 0x01/0x02, never 0xC1) and a framed file with
    /// `id_len = 0` is the honest "no locator" (the tamper-to-none shape),
    /// not an error.
    #[test]
    fn custody_frame_roundtrip_and_legacy_passthrough() {
        let id = CustodyId::from_bytes([0x5A; crate::custody::CUSTODY_ID_LEN]);
        let backend = envelope::encode_artifact(
            7,
            &[1u8; envelope::WRAP_IV_LEN],
            &[2u8; envelope::WRAP_CT_LEN],
        );
        let framed = WrapArtifact::from_freshly_wrapped(backend.clone()).with_custody_id(&id);
        let file = framed.file_bytes();
        assert_eq!(
            file.len(),
            2 + crate::custody::CUSTODY_ID_LEN + backend.len()
        );
        assert_eq!(file[0], crate::custody::CUSTODY_FRAME_V1);
        let parsed = WrapArtifact::from_bytes(file).expect("framed file parses");
        assert!(parsed.custody_id() == Some(id), "the locator round-trips");
        assert_eq!(parsed.as_bytes(), &backend[..], "backend bytes unchanged");

        // A pre-stage file: bare backend bytes, locator None, byte-identical out.
        let legacy = WrapArtifact::from_bytes(backend.clone()).expect("bare parses");
        assert!(legacy.custody_id().is_none());
        assert_eq!(legacy.as_bytes(), &backend[..]);
        assert_eq!(
            legacy.file_bytes(),
            backend,
            "no locator ⇒ the pre-S2 bytes"
        );
        // The Apple raw marker (1 byte) passes through the same way.
        let marker = WrapArtifact::from_bytes(vec![0x01]).expect("marker parses");
        assert!(marker.custody_id().is_none());
        assert_eq!(marker.file_bytes(), vec![0x01]);

        // Framed with id_len = 0 — the "locator edited to none" shape — is
        // the no-locator state, never a parse error (integrity is the AEAD's).
        let mut blanked = framed.file_bytes();
        blanked[1] = 0;
        let mut file = blanked[..2].to_vec();
        file.extend_from_slice(&blanked[2 + crate::custody::CUSTODY_ID_LEN..]);
        let none_parsed = WrapArtifact::from_bytes(file).expect("blanked locator parses");
        assert!(none_parsed.custody_id().is_none());
        assert_eq!(none_parsed.as_bytes(), &backend[..]);

        // Hostile frames never reach a vault: wrong id length, empty backend,
        // truncated header, oversize.
        let mut bad_len = framed.file_bytes();
        bad_len[1] = 15;
        assert!(matches!(
            WrapArtifact::from_bytes(bad_len),
            Err(WalletError::WrapArtifactInvalid)
        ));
        let mut empty_backend = framed.file_bytes();
        empty_backend[1] = 0;
        let truncated = empty_backend[..2].to_vec();
        assert!(matches!(
            WrapArtifact::from_bytes(truncated),
            Err(WalletError::WrapArtifactInvalid)
        ));
        assert!(matches!(
            WrapArtifact::from_bytes(vec![0xC1]),
            Err(WalletError::WrapArtifactInvalid)
        ));
        let mut long = framed.file_bytes();
        long.resize(WRAP_ARTIFACT_MAX_BYTES + 1, 0);
        assert!(matches!(
            WrapArtifact::from_bytes(long),
            Err(WalletError::WrapArtifactInvalid)
        ));
    }

    /// §4.3a / §8: store→load round-trips through a real-AEAD vault, and the
    /// custody status arrives with the data (disclosure-as-data).
    #[test]
    fn wrap_key_roundtrip_via_vault_with_status() {
        let vault = TestVault::new(VaultTier::Tee);
        let v = SealedSeedVault::new(&vault);
        let stored = v.store(&payload()).expect("store succeeds");
        assert!(!stored.status.degraded());

        let (loaded, status) = v
            .load(&stored.sealed_blob, &stored.wrap_artifact)
            .expect("load succeeds");
        assert_eq!(loaded.seed(), &[0x42; 32][..]);
        assert_eq!(loaded.mnemonic(), Some("ride glove window"));
        assert_eq!(status.tier, VaultTier::Tee);
    }

    /// §6.3 one-SealKey-two-blobs (W3-inc-2b): `store_wallet`/`load_wallet`
    /// seal the seed AND the DB key under a single custodied key and recover
    /// both — in SealedKeychain mode (seed + DB key) and None mode (DB key
    /// only). A swapped SECONDARY (DB-key) blob fails loudly at unseal.
    #[test]
    fn store_wallet_one_seal_key_seals_both_blobs() {
        let vault = TestVault::new(VaultTier::Tee);
        let v = SealedSeedVault::new(&vault);

        // SealedKeychain: seed + DB key under ONE wrap artifact.
        let db_key = WalletDbKey::generate();
        let stored = v
            .store_wallet(Some(&payload()), &db_key)
            .expect("store_wallet (sealed)");
        assert!(stored.seed_blob.is_some(), "sealed mode persists the seed");
        let loaded = v
            .load_wallet(
                stored.seed_blob.as_deref(),
                &stored.dbkey_blob,
                &stored.wrap_artifact,
            )
            .expect("load_wallet (sealed)");
        let seed = loaded.seed.expect("sealed mode recovers the seed");
        assert_eq!(seed.seed(), &[0x42; 32][..]);
        assert_eq!(seed.mnemonic(), Some("ride glove window"));
        // the recovered DB key is byte-identical (compared via the raw-key PRAGMA)
        assert_eq!(
            loaded.db_key.pragma_key_statement().as_str(),
            db_key.pragma_key_statement().as_str(),
            "DB key must round-trip exactly"
        );

        // None-persistence: only the DB key is sealed (no seed blob).
        let db_key2 = WalletDbKey::generate();
        let stored_none = v.store_wallet(None, &db_key2).expect("store_wallet (none)");
        assert!(
            stored_none.seed_blob.is_none(),
            "None mode never persists a seed"
        );
        let loaded_none = v
            .load_wallet(None, &stored_none.dbkey_blob, &stored_none.wrap_artifact)
            .expect("load_wallet (none)");
        assert!(loaded_none.seed.is_none());
        assert_eq!(
            loaded_none.db_key.pragma_key_statement().as_str(),
            db_key2.pragma_key_statement().as_str()
        );

        // Swapped SECONDARY blob: A's primary (seed) + artifact open the wrap
        // key, but B's DB-key blob was sealed under a different key → SealInvalid
        // (loud, typed) at unseal — never a silent open.
        let other = v
            .store_wallet(Some(&payload()), &WalletDbKey::generate())
            .expect("store_wallet B");
        assert!(matches!(
            v.load_wallet(
                stored.seed_blob.as_deref(),
                &other.dbkey_blob, // foreign secondary blob
                &stored.wrap_artifact,
            ),
            Err(WalletError::SealInvalid)
        ));
    }

    /// §4.3a crypto fold (the substitution vector): a valid blob from
    /// install A + a valid artifact from install B — each internally
    /// consistent — must fail LOUDLY and layer-attributed, never open
    /// silently, never report as the seed seal's `SealInvalid`.
    #[test]
    fn swapped_wrap_artifact_fails_loudly() {
        let vault_a = TestVault::new(VaultTier::Tee);
        let vault_b = TestVault::new(VaultTier::Tee);
        let a = SealedSeedVault::new(&vault_a)
            .store(&payload())
            .expect("install A");
        let b = SealedSeedVault::new(&vault_b)
            .store(&payload())
            .expect("install B");

        // Same vault, foreign artifact (the two-file swap on one device).
        let v_a = SealedSeedVault::new(&vault_a);
        assert!(matches!(
            v_a.load(&a.sealed_blob, &b.wrap_artifact),
            Err(WalletError::WrapArtifactInvalid)
        ));
        // Artifact ok, blob swapped: the AAD binding catches it at the WRAP
        // layer (before the seed envelope is ever consulted).
        assert!(matches!(
            v_a.load(&b.sealed_blob, &a.wrap_artifact),
            Err(WalletError::WrapArtifactInvalid)
        ));
        // Tampered blob: same loud wrap-layer failure (the binding makes
        // blob integrity a wrap-layer property too — defense in depth above
        // the seal's own tag).
        let mut tampered = a.sealed_blob.clone();
        let last = tampered.len() - 1;
        tampered[last] ^= 0x01;
        assert!(matches!(
            v_a.load(&tampered, &a.wrap_artifact),
            Err(WalletError::WrapArtifactInvalid)
        ));

        // Second accepted failure arm (crypto fold MINOR-1): a foreign
        // artifact naming a generation ABSENT on this vault surfaces as the
        // keysMissing class instead of the tag failure — same attack, two
        // honest arms, both loud, NEITHER a silent open or `SealInvalid`.
        let b2 = SealedSeedVault::new(&vault_b)
            .rotate(&b.sealed_blob, &b.wrap_artifact)
            .expect("rotate B to gen 2");
        assert!(matches!(
            v_a.load(&a.sealed_blob, &b2),
            Err(WalletError::KeystoreInconsistent { .. })
        ));
    }

    /// §4.3a rotation: new alias generation opens the seal; the superseded
    /// artifact dies with `finish_rotation`; the crash window (rotated but
    /// not finished) keeps BOTH artifacts working — no lockout.
    #[test]
    fn wrap_key_rotation_preserves_seal_openability() {
        let vault = TestVault::new(VaultTier::StrongBox);
        let v = SealedSeedVault::new(&vault);
        let stored = v.store(&payload()).expect("store");

        let rotated = v
            .rotate(&stored.sealed_blob, &stored.wrap_artifact)
            .expect("rotate");
        assert_ne!(rotated.as_bytes(), stored.wrap_artifact.as_bytes());

        // Crash window: old AND new both open (the §6.3 no-lockout rule).
        assert!(v.load(&stored.sealed_blob, &rotated).is_ok());
        assert!(v.load(&stored.sealed_blob, &stored.wrap_artifact).is_ok());

        // Finish: the superseded alias is gone; the new one still opens.
        v.finish_rotation(&stored.wrap_artifact, &rotated)
            .expect("finish");
        assert!(v.load(&stored.sealed_blob, &rotated).is_ok());
        assert!(matches!(
            v.load(&stored.sealed_blob, &stored.wrap_artifact),
            Err(WalletError::KeystoreInconsistent { .. })
        ));
    }

    /// §4.3a security fold: the software tier is SURFACED as degraded on
    /// every operation that returns status — recorded-but-silent is the
    /// posture lie the fold closed.
    #[test]
    fn keystore_software_tier_is_surfaced_not_silent() {
        let vault = TestVault::new(VaultTier::SoftwareKeystore);
        let v = SealedSeedVault::new(&vault);
        let stored = v.store(&payload()).expect("store");
        assert!(
            stored.status.degraded(),
            "store must surface the software tier"
        );
        let (_, status) = v
            .load(&stored.sealed_blob, &stored.wrap_artifact)
            .expect("load");
        assert!(status.degraded(), "load must surface the software tier");
    }

    /// §4.3a fail-closed: no vault ⇒ typed `VaultAbsent` BEFORE any seal
    /// exists — and the in-memory-this-session fallback is structurally
    /// unreachable (no non-test vault constructor exists in this crate; the
    /// error is the ONLY thing `store` can produce here).
    #[test]
    fn keychain_unavailable_yields_typed_error_not_fallback() {
        let vault = TestVault::absent();
        let v = SealedSeedVault::new(&vault);
        let Err(err) = v.store(&payload()) else {
            panic!("no vault must fail closed");
        };
        assert!(matches!(err, WalletError::VaultAbsent));
        assert_eq!(
            vault.store_calls(),
            0,
            "fail-closed means the vault was never asked to custody anything"
        );
    }

    /// §4.3a wipe: severing the wrap key leaves the (still present) blob
    /// unopenable through this key store — the §6.3 "seed dies first" ordering.
    #[test]
    fn wrap_key_wipe_leaves_seal_unopenable() {
        let vault = TestVault::new(VaultTier::Tee);
        let v = SealedSeedVault::new(&vault);
        let stored = v.store(&payload()).expect("store");
        v.wipe(&stored.wrap_artifact).expect("wipe");
        assert!(matches!(
            v.load(&stored.sealed_blob, &stored.wrap_artifact),
            Err(WalletError::KeystoreInconsistent { .. })
        ));
        // Idempotent (rust-patterns state-machine rule: terminal is a no-op).
        v.wipe(&stored.wrap_artifact)
            .expect("second wipe is a no-op");
    }

    /// Cross-backend artifact confusion: an Apple-shaped marker fed to an
    /// Android-shaped backend (or garbage from disk) dies in
    /// validate-before-use, never reaching the vault.
    #[test]
    fn foreign_artifact_shapes_rejected_before_vault() {
        let vault = TestVault::new(VaultTier::Tee);
        let v = SealedSeedVault::new(&vault);
        let stored = v.store(&payload()).expect("store");

        let apple_marker = WrapArtifact::from_bytes(vec![0x01]).expect("size-valid");
        assert!(matches!(
            v.load(&stored.sealed_blob, &apple_marker),
            Err(WalletError::WrapArtifactInvalid)
        ));
        assert!(matches!(
            WrapArtifact::from_bytes(vec![0u8; WRAP_ARTIFACT_MAX_BYTES + 1]),
            Err(WalletError::WrapArtifactInvalid)
        ));
        // Accept boundary too (testing-patterns: constants tested on BOTH
        // sides): exactly MAX is admissible.
        assert!(
            WrapArtifact::from_bytes(vec![0u8; WRAP_ARTIFACT_MAX_BYTES]).is_ok(),
            "exact-max must be accepted"
        );
        assert!(matches!(
            WrapArtifact::from_bytes(Vec::new()),
            Err(WalletError::WrapArtifactInvalid)
        ));

        // Downgrade honesty at the ORCHESTRATION layer (crypto fold
        // MINOR-5): a v2-shaped artifact is surfaced as version-unsupported
        // through SealedSeedVault::load, never guessed at.
        let mut v2 = stored.wrap_artifact.as_bytes().to_vec();
        v2[0] = 0x02;
        let v2_artifact = WrapArtifact::from_bytes(v2).expect("size-valid");
        assert!(matches!(
            v.load(&stored.sealed_blob, &v2_artifact),
            Err(WalletError::WrapVersionUnsupported { found: 0x02 })
        ));
    }
}
