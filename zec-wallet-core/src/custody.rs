//! S2 custody — the wallet's identity, independent of where its files live.
//!
//! A wallet is the same wallet wherever its container sits: an app update that
//! moves the sandbox, a scoped-storage move, a relocation. The identity that
//! makes that true is a random 128-bit [`CustodyId`] minted at create and
//! carried as a plaintext LOCATOR in the `wrap.artifact` header (the one file
//! read before any key); the keychain namespace is derived from it
//! ([`namespace_for`]), so a moved wallet asks its vault for the SAME
//! namespace it was created under — the path no longer decides custody.
//!
//! Integrity is the AEAD's, not the locator's: the wrap key the locator points
//! at either authenticates the artifact or the open is refused typed
//! (`WrapArtifactInvalid` / the keysMissing class) — a tampered locator can
//! produce a refusal and never a wrong wallet. Reverses
//! `keychain_namespace_for`'s documented "a genuinely moved `db_dir` is a new
//! wallet" — ADR-0559.
//!
//! **The path-keyed index** ([`CustodyIndexEntry`]): a small keychain item
//! under today's path-derived namespace that points at the identifier, so a
//! wipe finds the namespace when the host has already deleted the files
//! (Relim's Dart wipe runs after its own `remove_dir_all`). The path is an
//! INDEX, never the identity.
//!
//! **Migration** (a pre-stage wallet, no id in the header) — ONE commit point,
//! every step before it idempotent and resumable, every step after it deferred
//! and recorded (driven by `store::open`):
//! (1) mint the id; (2) write the index `{id, legacy_ns = path-ns, pending}` —
//! a re-run REUSES this id, so no namespace is ever orphaned by a fresh mint;
//! (3) `store_wrap_key` under the id namespace, the legacy item left live;
//! (4) **the commit:** the single `write_atomic` of the wrap artifact carrying
//! the id; (5) on the next successful open under the id namespace, purge the
//! legacy namespace and set the index `done`. A crash in any window re-opens
//! and resumes with the SAME id.
//!
//! **Never logged, never across the FFI** (mirroring `seed_fingerprint`):
//! [`CustodyId`] has no `Display`/`Debug`, no tracing field carries it, no DTO
//! exposes it — a host correlates wallets by `db_dir`, as today. The plaintext
//! locator inherits ADR-0526's existence-oracle debt (a directory that IS a
//! wallet already says so) and does not widen it: stated, cited, not closed
//! here.

use chacha20poly1305::aead::rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};

// Read only by the index codec's `decode`, compiled where a vault is.
#[cfg(any(test, target_os = "android", target_os = "macos", target_os = "ios"))]
use crate::error::WalletError;
use crate::keychain::KeychainNamespace;

/// FROZEN domain-separation tag for the custody namespace derivation
/// (crypto rule: NEVER edit; a scheme change bumps the `/vN` suffix and gets
/// its own migration).
pub(crate) const CUSTODY_DOMAIN: &[u8] = b"zec-wallet/custody/v1";

/// Custody locator length — a random 128-bit identifier.
pub(crate) const CUSTODY_ID_LEN: usize = 16;

/// The wrap-artifact outer custody frame's version byte (FROZEN, append-only).
/// Distinct from every BACKEND artifact version byte (0x01 the v1 Android/raw
/// marker, 0x02 the Apple SE artifact — and any future backend version is a
/// small ordinal, not 0xC1), so a framed file and a bare pre-stage backend
/// artifact are distinguishable at byte 0 and the backend bytes ride the frame
/// UNCHANGED. A rename/renumber would strand every migrated wallet's locator
/// — pinned by `custody_frame_pinned`.
pub(crate) const CUSTODY_FRAME_V1: u8 = 0xC1;

/// The wallet's custody identity: 128 bits of OsRng, minted once at create.
/// Not secret (it names a keychain namespace, it does not open one) but never
/// rendered for a READER: no `Display`, no `Debug`, no tracing field, no DTO —
/// the only honest correlation surface stays `db_dir`, as today. It IS stored
/// in plaintext where custody needs it: the wrap artifact's header (the
/// locator) and the path-keyed index item — on Android that item is
/// presence-encoded in a Keystore alias NAME, so the id is spelled in the alias
/// list, readable only by this app's own process (the same exposure as the
/// header in its sandbox; ADR-0526's existence-oracle scope).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct CustodyId([u8; CUSTODY_ID_LEN]);

impl CustodyId {
    /// Mint a fresh identifier (OsRng — the same source as `SealKey`).
    pub(crate) fn generate() -> Self {
        let mut bytes = [0u8; CUSTODY_ID_LEN];
        OsRng.fill_bytes(&mut bytes);
        Self(bytes)
    }

    /// Wrap raw bytes (the header parser; a test fixture). Validate-by-shape:
    /// the length is the type's, so nothing shorter can pose as an id.
    pub(crate) fn from_bytes(bytes: [u8; CUSTODY_ID_LEN]) -> Self {
        Self(bytes)
    }

    /// The raw locator bytes (the header codec; the namespace derivation).
    pub(crate) fn as_bytes(&self) -> &[u8; CUSTODY_ID_LEN] {
        &self.0
    }
}

/// The keychain namespace a wallet's wrap key lives under, derived from its
/// identity: `hex(SHA256(CUSTODY_DOMAIN ‖ id)[..16])` — the same shape and
/// collision resistance as the path derivation, domain-separated so an id can
/// never alias a path (or any other SHA-256 use) and the two derivations stay
/// disjoint by construction.
pub(crate) fn namespace_for(id: &CustodyId) -> KeychainNamespace {
    let mut h = Sha256::new();
    h.update(CUSTODY_DOMAIN);
    h.update(id.as_bytes());
    let digest = h.finalize();
    // hex::encode of EXACTLY 16 bytes is always 32 lowercase-hex chars — the
    // `KeychainNamespace` invariant holds by construction, so this never fails.
    KeychainNamespace::new(hex::encode(&digest[..16]))
        .expect("hex::encode of 16 bytes is always 32 lowercase-hex chars")
}

/// Where the migration stands — the index item's recorded state. `pending`
/// names the legacy namespace still holding a live wrap key (the deferred
/// purge target); `done` means the id namespace is the only live custody.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CustodyIndexState {
    /// Migration began but has not finished: the legacy namespace named by
    /// [`CustodyIndexEntry::legacy_ns`] still holds a LIVE wrap key that step
    /// (5) must purge, and a wipe must sever alongside the id namespace.
    Pending,
    /// The id namespace is the wallet's only live custody (a fresh create, or
    /// a completed migration).
    Done,
}

impl CustodyIndexState {
    // The index codec (`to_byte`/`from_byte`, `CUSTODY_INDEX_MAX_BYTES`,
    // `encode`/`decode`) is compiled where its readers are: the Android and
    // Apple vaults' `store_index`/`load_index`, and the `cfg(test)` test vault
    // and codec tests. A host with no vault (Linux) has no index to read.
    #[cfg(any(test, target_os = "android", target_os = "macos", target_os = "ios"))]
    fn to_byte(self) -> u8 {
        match self {
            Self::Pending => 0,
            Self::Done => 1,
        }
    }

    #[cfg(any(test, target_os = "android", target_os = "macos", target_os = "ios"))]
    fn from_byte(b: u8) -> Option<Self> {
        match b {
            0 => Some(Self::Pending),
            1 => Some(Self::Done),
            _ => None,
        }
    }
}

/// The path-keyed index item: `{id, legacy_ns?, state}`, stored under the
/// path-derived namespace (a small keychain item, no key material) so a wipe
/// can resolve the custody namespace with the wallet's directory already
/// gone. `legacy_ns` is present exactly while `state` is `Pending`.
///
/// Deliberately NO `Debug` (the entry embeds a [`CustodyId`], which has none
/// — the never-logged rule rides the type).
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct CustodyIndexEntry {
    pub(crate) id: CustodyId,
    pub(crate) legacy_ns: Option<KeychainNamespace>,
    pub(crate) state: CustodyIndexState,
}

/// Hard cap on an encoded index item — validate-before-use: a backend hands
/// the codec bytes from a keychain slot and never allocates unbounded.
#[cfg(any(test, target_os = "android", target_os = "macos", target_os = "ios"))]
pub(crate) const CUSTODY_INDEX_MAX_BYTES: usize = 64;

impl CustodyIndexEntry {
    /// The steady-state item: a fresh create, or a completed migration.
    pub(crate) fn done(id: &CustodyId) -> Self {
        Self {
            id: *id,
            legacy_ns: None,
            state: CustodyIndexState::Done,
        }
    }

    /// The migration checkpoint (step 2): the id to reuse on any re-run, and
    /// the legacy namespace step (5) must purge.
    pub(crate) fn pending(id: &CustodyId, legacy_ns: &KeychainNamespace) -> Self {
        Self {
            id: *id,
            legacy_ns: Some(legacy_ns.clone()),
            state: CustodyIndexState::Pending,
        }
    }

    pub(crate) fn is_pending(&self) -> bool {
        self.state == CustodyIndexState::Pending
    }

    /// Encode: `[ver:1][state:1][id:16][legacy_len:1 (0|32)][legacy_ns]`.
    /// Validate-by-shape: the encoder cannot produce an out-of-cap item.
    #[cfg(any(test, target_os = "android", target_os = "macos", target_os = "ios"))]
    pub(crate) fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(CUSTODY_INDEX_MAX_BYTES);
        out.push(CUSTODY_FRAME_V1); // the index codec shares the frozen frame tag
        out.push(self.state.to_byte());
        out.extend_from_slice(self.id.as_bytes());
        match &self.legacy_ns {
            None => out.push(0),
            Some(ns) => {
                debug_assert_eq!(ns.as_str().len(), 32);
                out.push(32);
                out.extend_from_slice(ns.as_str().as_bytes());
            }
        }
        out
    }

    /// Decode with validate-before-use: exact version, in-range state, exact
    /// id length, a legacy namespace that is BOTH well-formed 32-hex AND
    /// present exactly while pending. Anything else is a corrupt item, never
    /// guessed at.
    #[cfg(any(test, target_os = "android", target_os = "macos", target_os = "ios"))]
    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, WalletError> {
        let invalid = || WalletError::KeystoreInconsistent {
            permanently_invalidated: false,
        };
        if bytes.len() > CUSTODY_INDEX_MAX_BYTES {
            return Err(invalid());
        }
        let (&ver, rest) = bytes.split_first().ok_or_else(invalid)?;
        if ver != CUSTODY_FRAME_V1 {
            return Err(invalid());
        }
        let (&state_b, rest) = rest.split_first().ok_or_else(invalid)?;
        let state = CustodyIndexState::from_byte(state_b).ok_or_else(invalid)?;
        if rest.len() < CUSTODY_ID_LEN {
            return Err(invalid());
        }
        let (id_bytes, rest) = rest.split_at(CUSTODY_ID_LEN);
        let mut id = [0u8; CUSTODY_ID_LEN];
        id.copy_from_slice(id_bytes);
        let (&legacy_len, rest) = rest.split_first().ok_or_else(invalid)?;
        let legacy_ns = match legacy_len {
            0 => {
                if !rest.is_empty() {
                    return Err(invalid());
                }
                None
            }
            32 => {
                let ns = KeychainNamespace::new(
                    std::str::from_utf8(rest).map_err(|_| invalid())?.to_owned(),
                )
                .ok_or_else(invalid)?;
                Some(ns)
            }
            _ => return Err(invalid()),
        };
        // Present exactly while pending — a `done` item naming a legacy
        // namespace would tell a wipe to sever an namespace it has no right
        // to; a `pending` item without one has no purge target.
        if legacy_ns.is_some() != (state == CustodyIndexState::Pending) {
            return Err(invalid());
        }
        Ok(Self {
            id: CustodyId(id),
            legacy_ns,
            state,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Frozen-label pins (testing-patterns: every frozen label gets one) — a
    /// one-character diff fails CI.
    #[test]
    fn custody_labels_pinned() {
        assert_eq!(CUSTODY_DOMAIN, b"zec-wallet/custody/v1");
        assert_eq!(CUSTODY_FRAME_V1, 0xC1);
        assert_eq!(CUSTODY_ID_LEN, 16);
    }

    /// The namespace derivation: stable per id, distinct across ids, and the
    /// `KeychainNamespace` 32-lowercase-hex invariant — and DOMAIN-SEPARATED
    /// from the path derivation (an id namespace can never equal the
    /// namespace of the path it was minted under).
    #[test]
    fn custody_namespace_is_stable_distinct_and_domain_separated() {
        let a = CustodyId::from_bytes([0x11; CUSTODY_ID_LEN]);
        let b = CustodyId::from_bytes([0x22; CUSTODY_ID_LEN]);
        let ns_a = namespace_for(&a);
        assert_eq!(ns_a.as_str(), namespace_for(&a).as_str(), "stable per id");
        assert_ne!(ns_a.as_str(), namespace_for(&b).as_str(), "distinct ids");
        assert_eq!(ns_a.as_str().len(), 32);
        assert!(
            ns_a.as_str()
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        );
        let path_ns = crate::wallet::keychain_namespace_for(std::path::Path::new(
            "/data/dev.zecwallet.example/wallet",
        ));
        assert_ne!(
            ns_a.as_str(),
            path_ns.as_str(),
            "the id derivation is domain-separated from the path derivation"
        );
    }

    /// The index codec: round-trips both states, rejects every malformed
    /// shape before use (unknown version/state, short id, wrong legacy shape,
    /// a done item naming a legacy namespace, oversized).
    #[test]
    fn index_entry_roundtrip_and_validate_before_use() {
        let id = CustodyId::from_bytes([0x42; CUSTODY_ID_LEN]);
        let legacy = crate::wallet::keychain_namespace_for(std::path::Path::new("/x/wallet"));
        let pending = CustodyIndexEntry::pending(&id, &legacy);
        assert!(pending.is_pending());
        let enc = pending.encode();
        assert!(enc.len() <= CUSTODY_INDEX_MAX_BYTES);
        let dec = CustodyIndexEntry::decode(&enc).expect("pending round-trips");
        assert!(dec == pending, "pending entry round-trips exactly");
        assert_eq!(
            dec.legacy_ns.as_ref().map(|n| n.as_str()),
            Some(legacy.as_str())
        );

        let done = CustodyIndexEntry::done(&id);
        assert!(!done.is_pending());
        let dec_done = CustodyIndexEntry::decode(&done.encode()).expect("done round-trips");
        assert!(dec_done == done, "done entry round-trips exactly");
        assert!(dec_done.legacy_ns.is_none());

        // Hostile shapes never parse.
        assert!(CustodyIndexEntry::decode(&[]).is_err());
        assert!(CustodyIndexEntry::decode(&[0x01, 1, 0x42, 0]).is_err()); // wrong frame tag
        let mut bad_state = done.encode();
        bad_state[1] = 9;
        assert!(CustodyIndexEntry::decode(&bad_state).is_err());
        let mut done_with_legacy = done.encode();
        done_with_legacy.push(32); // trailing junk on a done item
        assert!(CustodyIndexEntry::decode(&done_with_legacy).is_err());
        // A pending item without a legacy namespace is corrupt, never guessed.
        let mut pending_no_legacy = pending.encode();
        let cut = pending_no_legacy.len() - 33;
        pending_no_legacy.truncate(cut);
        pending_no_legacy.push(0);
        assert!(CustodyIndexEntry::decode(&pending_no_legacy).is_err());
        // Non-hex legacy bytes are corrupt.
        let mut bad_ns = pending.encode();
        let n = bad_ns.len();
        for i in &mut bad_ns[n - 32..] {
            *i = b'Z';
        }
        assert!(CustodyIndexEntry::decode(&bad_ns).is_err());
        // Oversized is rejected before any allocation depends on it.
        let mut oversized = vec![0u8; CUSTODY_INDEX_MAX_BYTES + 1];
        oversized[0] = CUSTODY_FRAME_V1;
        oversized[1] = 1;
        assert!(CustodyIndexEntry::decode(&oversized).is_err());
    }
}
