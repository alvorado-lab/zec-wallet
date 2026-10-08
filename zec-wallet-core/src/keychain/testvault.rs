//! §4.3a test vault — the Android-shaped backend with a REAL AEAD.
//!
//! Honesty note (why this is not theater): the properties the host tests
//! must prove — envelope framing, the `H(sealed_blob)` AAD binding rejecting
//! substituted artifacts, one-alias-one-encryption rotation, fail-closed
//! no-vault, severing wipe — are AEAD-generic. This backend runs them
//! through `ChaCha20Poly1305` (IETF, the SAME 12-byte IV shape as Keystore
//! GCM, from the already-pinned audited crate — no new dep), sharing the
//! envelope codec with the device adapter byte-for-byte. What it cannot
//! prove — the JNI plumbing and AndroidKeyStore semantics — is exactly what
//! the on-device `wrap_key_roundtrip_via_keystore` manual gate covers.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use chacha20poly1305::aead::rand_core::RngCore;
use chacha20poly1305::aead::{Aead, KeyInit, OsRng, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use super::envelope::{self, WRAP_IV_LEN, WRAP_VERSION_V1};
use super::{KeychainPort, VaultTier, WrapArtifact, seal_key_from_vault};
use crate::custody::CustodyIndexEntry;
use crate::error::WalletError;
use crate::seal::SealKey;

/// One simulated vault "device": per-alias-generation AEAD keys, as the
/// real Keystore holds per-alias AES keys. Two instances = two installs
/// (the substitution test's A/B).
pub(crate) struct TestVault {
    /// alias generation → vault key. `None` value = alias deleted.
    aliases: Mutex<HashMap<u32, Zeroizing<[u8; 32]>>>,
    next_gen: Mutex<u32>,
    tier: Option<VaultTier>,
    store_calls: AtomicUsize,
    /// S2 custody index item under this (namespace-blind) vault. Per-instance
    /// like the aliases — the fixed-vault seams hand every namespace the SAME
    /// instance, so the index rides with it.
    index: Mutex<Option<Vec<u8>>>,
}

impl TestVault {
    pub(crate) fn new(tier: VaultTier) -> Self {
        Self {
            aliases: Mutex::new(HashMap::new()),
            next_gen: Mutex::new(1),
            tier: Some(tier),
            store_calls: AtomicUsize::new(0),
            index: Mutex::new(None),
        }
    }

    /// The no-vault device (headless / keystore-less) — `tier()` fails
    /// closed and nothing else is reachable.
    pub(crate) fn absent() -> Self {
        Self {
            aliases: Mutex::new(HashMap::new()),
            next_gen: Mutex::new(1),
            tier: None,
            store_calls: AtomicUsize::new(0),
            index: Mutex::new(None),
        }
    }

    /// Test observability: fail-closed means the vault was never asked to
    /// custody anything.
    pub(crate) fn store_calls(&self) -> usize {
        self.store_calls.load(Ordering::SeqCst)
    }

    fn fresh_alias(&self) -> (u32, Zeroizing<[u8; 32]>) {
        let mut next = self.next_gen.lock().expect("test lock");
        let this = *next;
        *next += 1;
        let mut key = Zeroizing::new([0u8; 32]);
        OsRng.fill_bytes(&mut *key);
        self.aliases
            .lock()
            .expect("test lock")
            .insert(this, key.clone());
        (this, key)
    }

    fn wrap_under(
        &self,
        alias_key: &[u8; 32],
        alias_gen: u32,
        seal_key: &SealKey,
        sealed_blob: &[u8],
    ) -> Result<WrapArtifact, WalletError> {
        // The vault supplies the IV — same ownership as Keystore GCM.
        let mut iv = [0u8; WRAP_IV_LEN];
        OsRng.fill_bytes(&mut iv);
        let aad = envelope::wrap_aad(sealed_blob);
        let cipher = ChaCha20Poly1305::new(Key::from_slice(alias_key.as_slice()));
        let ct = cipher
            .encrypt(
                Nonce::from_slice(&iv),
                Payload {
                    msg: seal_key.as_bytes(),
                    aad: &aad,
                },
            )
            .map_err(|_| WalletError::KeystoreUnavailable)?;
        Ok(WrapArtifact::from_freshly_wrapped(
            envelope::encode_artifact(alias_gen, &iv, &ct),
        ))
    }
}

impl KeychainPort for TestVault {
    fn probe(&self) -> Result<(), WalletError> {
        self.tier.map(|_| ()).ok_or(WalletError::VaultAbsent)
    }

    fn tier(&self) -> Result<VaultTier, WalletError> {
        self.tier.ok_or(WalletError::VaultAbsent)
    }

    fn store_wrap_key(
        &self,
        key: SealKey,
        sealed_blob: &[u8],
    ) -> Result<WrapArtifact, WalletError> {
        self.probe()?;
        self.store_calls.fetch_add(1, Ordering::SeqCst);
        let (alias_gen, alias_key) = self.fresh_alias();
        self.wrap_under(&alias_key, alias_gen, &key, sealed_blob)
    }

    fn load_wrap_key(
        &self,
        artifact: &WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<SealKey, WalletError> {
        self.probe()?;
        let parsed = envelope::parse_artifact(artifact.as_bytes())?;
        let alias_key = self
            .aliases
            .lock()
            .expect("test lock")
            .get(&parsed.alias_gen)
            .cloned()
            // Alias genuinely absent while a blob exists — the §4.2a
            // keysMissing class, NOT a fresh wallet.
            .ok_or(WalletError::KeystoreInconsistent {
                permanently_invalidated: false,
            })?;
        let aad = envelope::wrap_aad(sealed_blob);
        let cipher = ChaCha20Poly1305::new(Key::from_slice(alias_key.as_slice()));
        let pt = cipher
            .decrypt(
                Nonce::from_slice(&parsed.iv),
                Payload {
                    msg: &parsed.ct,
                    aad: &aad,
                },
            )
            // Tag failure = substitution or corruption — collapsed, but
            // layer-attributed (§4.3a).
            .map_err(|_| WalletError::WrapArtifactInvalid)
            .map(Zeroizing::new)?;
        seal_key_from_vault(pt)
    }

    fn rotate_wrap_key(
        &self,
        artifact: &WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<WrapArtifact, WalletError> {
        // Recover under the OLD generation, re-custody under a NEW one.
        // One alias = one encryption: the new wrap never reuses gen/IV.
        let key = self.load_wrap_key(artifact, sealed_blob)?;
        let (alias_gen, alias_key) = self.fresh_alias();
        self.wrap_under(&alias_key, alias_gen, &key, sealed_blob)
    }

    fn finish_rotation(&self, old: &WrapArtifact, new: &WrapArtifact) -> Result<(), WalletError> {
        let old_gen = envelope::parse_artifact(old.as_bytes())?.alias_gen;
        let new_gen = envelope::parse_artifact(new.as_bytes())?.alias_gen;
        if old_gen == new_gen {
            // Rotation was vacuous (or caller confused old/new) — deleting
            // the live alias would be a lockout, so: no-op.
            return Ok(());
        }
        self.aliases.lock().expect("test lock").remove(&old_gen);
        Ok(())
    }

    fn delete_wrap_key(&self, artifact: &WrapArtifact) -> Result<(), WalletError> {
        let generation = envelope::parse_artifact(artifact.as_bytes())?.alias_gen;
        // Idempotent: already-gone is the terminal no-op, not an error.
        self.aliases.lock().expect("test lock").remove(&generation);
        Ok(())
    }

    fn purge_namespace(&self) -> Result<usize, WalletError> {
        // Android per-device shape: every alias generation under this vault IS this
        // namespace's custody (the prefix scan), so the purge clears them all and
        // returns the count severed (the verify-real-sever signal). NOTE: this models
        // the per-device alias set, NOT Android's prefix SCOPING — a wrong-namespace
        // purge here would still clear the alias, whereas real Android severs 0 under
        // a wrong prefix. Namespace-ISOLATION tests (the B1 fail-closed guard) must use
        // `SharedKeychainVault` (namespace-keyed), which models that correctly.
        let mut aliases = self.aliases.lock().expect("test lock");
        let severed = aliases.len();
        aliases.clear();
        // The S2 index item is NOT swept — a purge is wrap material only, as on
        // every production backend; the wipe deletes the index, LAST.
        Ok(severed)
    }

    fn store_index(&self, entry: &CustodyIndexEntry) -> Result<(), WalletError> {
        *self.index.lock().expect("test lock") = Some(entry.encode());
        Ok(())
    }

    fn load_index(&self) -> Result<Option<CustodyIndexEntry>, WalletError> {
        self.index
            .lock()
            .expect("test lock")
            .as_deref()
            .map(CustodyIndexEntry::decode)
            .transpose()
    }

    fn delete_index(&self) -> Result<(), WalletError> {
        *self.index.lock().expect("test lock") = None;
        Ok(())
    }
}

/// The single simulated OS keychain shared across [`SharedKeychainVault`] instances:
/// per-`namespace` SealKey items (`namespace -> (SealKey bytes, H(sealed_blob))`) plus
/// the S2 custody INDEX items (`namespace -> encoded entry`). Clone it into each
/// per-wallet vault so they all read/write ONE keychain (the cross-wallet sharing the
/// real OS keychain has — which `TestVault`, being per-instance, cannot model).
///
/// TEST-ONLY: the stored `SealKey` bytes are NOT zeroized (a plain `[u8; 32]`). This
/// double exists to witness the FR-13 namespace-isolation property, NOT zeroize
/// discipline — never extract it into non-test code; the real backends + `TestVault`
/// carry the zeroizing handling.
pub(crate) type SharedKeychain = Arc<Mutex<SharedKeychainStore>>;

/// The key items and the S2 index items of one simulated keychain, in TWO maps.
/// The store Derefs to the KEY map (what a test's "how many keychain items"
/// assertion counts), so the index breadcrumbs that ride beside them stay
/// invisible to the pinned FR-14 wipe-row counts (S2 added one index item per
/// wallet path; those counts must not move).
pub(crate) struct SharedKeychainStore {
    items: HashMap<String, ([u8; 32], [u8; 32])>,
    indexes: HashMap<String, Vec<u8>>,
}

impl SharedKeychainStore {
    pub(crate) fn len(&self) -> usize {
        self.items.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub(crate) fn contains_key(&self, ns: &str) -> bool {
        self.items.contains_key(ns)
    }

    pub(crate) fn remove(&mut self, ns: &str) -> Option<([u8; 32], [u8; 32])> {
        self.items.remove(ns)
    }
}

impl std::ops::DerefMut for SharedKeychainStore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}

impl std::ops::Deref for SharedKeychainStore {
    type Target = HashMap<String, ([u8; 32], [u8; 32])>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

/// FR-13 coexistence test double — the SHARED OS keychain, Apple-shaped (the
/// `SealKey` lives IN the vault, the artifact is a 1-byte version marker bound to
/// `H(sealed_blob)`). Multiple instances over ONE shared store but with DISTINCT
/// `namespace`s write to distinct slots ⇒ coexisting custody; the SAME namespace
/// OVERWRITES (the correct one-namespace-one-wallet behavior — and exactly the
/// collision the pre-FR-13 process-global fixed item name suffered). This is what
/// `TestVault` (per-instance, Android alias-generation) cannot witness.
pub(crate) struct SharedKeychainVault {
    namespace: String,
    tier: Option<VaultTier>,
    store: SharedKeychain,
}

impl SharedKeychainVault {
    /// A fresh, empty shared "keychain" — clone it into each per-wallet vault.
    pub(crate) fn shared() -> SharedKeychain {
        Arc::new(Mutex::new(SharedKeychainStore {
            items: HashMap::new(),
            indexes: HashMap::new(),
        }))
    }

    pub(crate) fn new(tier: VaultTier, namespace: &str, store: SharedKeychain) -> Self {
        Self {
            namespace: namespace.to_owned(),
            tier: Some(tier),
            store,
        }
    }
}

impl KeychainPort for SharedKeychainVault {
    fn probe(&self) -> Result<(), WalletError> {
        self.tier.map(|_| ()).ok_or(WalletError::VaultAbsent)
    }

    fn tier(&self) -> Result<VaultTier, WalletError> {
        self.tier.ok_or(WalletError::VaultAbsent)
    }

    fn store_wrap_key(
        &self,
        key: SealKey,
        sealed_blob: &[u8],
    ) -> Result<WrapArtifact, WalletError> {
        self.probe()?;
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(key.as_bytes());
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&Sha256::digest(sealed_blob));
        // Fixed-name-per-namespace: overwrite any prior item under this namespace
        // (the Apple delete-first/add-fresh semantics — same namespace = same wallet).
        self.store
            .lock()
            .expect("test lock")
            .insert(self.namespace.clone(), (key_bytes, hash));
        Ok(WrapArtifact::from_freshly_wrapped(vec![WRAP_VERSION_V1]))
    }

    fn load_wrap_key(
        &self,
        artifact: &WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<SealKey, WalletError> {
        self.probe()?;
        // Model simplification: the real Apple backend distinguishes a recognized-but-
        // unsupported version (`WrapVersionUnsupported`) from a malformed marker; this
        // double collapses both to `WrapArtifactInvalid` since the FR-13 tests only ever
        // pass a `WRAP_VERSION_V1` marker (the version taxonomy is `apple.rs`'s to prove).
        match artifact.as_bytes() {
            [v] if *v == WRAP_VERSION_V1 => {}
            _ => return Err(WalletError::WrapArtifactInvalid),
        }
        let guard = self.store.lock().expect("test lock");
        let (key_bytes, stored_hash) = guard
            .get(&self.namespace)
            // Absent under this namespace while a blob exists — the keysMissing class.
            .ok_or(WalletError::KeystoreInconsistent {
                permanently_invalidated: false,
            })?;
        // Blob↔item binding (the bug witness): if a DIFFERENT wallet overwrote this
        // namespace's slot, this wallet's blob hash ≠ the stored hash ⇒ a LOUD
        // WrapArtifactInvalid, never a silent open (mirrors the Apple binding).
        if stored_hash.as_slice() != Sha256::digest(sealed_blob).as_slice() {
            return Err(WalletError::WrapArtifactInvalid);
        }
        seal_key_from_vault(Zeroizing::new(key_bytes.to_vec()))
    }

    fn rotate_wrap_key(
        &self,
        artifact: &WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<WrapArtifact, WalletError> {
        // Vacuous (Apple shape): verify the pair still opens, return the marker.
        let _ = self.load_wrap_key(artifact, sealed_blob)?;
        Ok(WrapArtifact::from_freshly_wrapped(vec![WRAP_VERSION_V1]))
    }

    fn finish_rotation(&self, _old: &WrapArtifact, _new: &WrapArtifact) -> Result<(), WalletError> {
        Ok(())
    }

    fn delete_wrap_key(&self, _artifact: &WrapArtifact) -> Result<(), WalletError> {
        // Sever this namespace's custody (idempotent — already-gone is a no-op).
        self.store
            .lock()
            .expect("test lock")
            .remove(&self.namespace);
        Ok(())
    }

    fn purge_namespace(&self) -> Result<usize, WalletError> {
        // Apple namespace-keyed shape: one item per namespace, so the purge removes
        // exactly this namespace's slot and returns 1 if it existed, 0 if already
        // gone (the verify-real-sever signal — a namespace mismatch severs 0).
        let mut guard = self.store.lock().expect("test lock");
        let severed = usize::from(guard.remove(&self.namespace).is_some());
        // The S2 index item is NOT swept — a purge is wrap material only, as on
        // every production backend; the wipe deletes the index, LAST.
        Ok(severed)
    }

    fn store_index(&self, entry: &CustodyIndexEntry) -> Result<(), WalletError> {
        self.store
            .lock()
            .expect("test lock")
            .indexes
            .insert(self.namespace.clone(), entry.encode());
        Ok(())
    }

    fn load_index(&self) -> Result<Option<CustodyIndexEntry>, WalletError> {
        self.store
            .lock()
            .expect("test lock")
            .indexes
            .get(&self.namespace)
            .map(|bytes| CustodyIndexEntry::decode(bytes))
            .transpose()
    }

    fn delete_index(&self) -> Result<(), WalletError> {
        self.store
            .lock()
            .expect("test lock")
            .indexes
            .remove(&self.namespace);
        Ok(())
    }
}
