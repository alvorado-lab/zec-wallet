//! §4.3a / FR-14 chunk 2 — the Apple custody adapters. TWO `KeychainPort` impls:
//!
//! - **[`AppleSecureEnclaveVault`] (PRODUCTION):** the `SealKey` is ECIES-wrapped under a
//!   NON-EXTRACTABLE Secure-Enclave key (located by application tag); the wrapped blob is
//!   the on-disk v2 artifact, the wrap key lives in hardware. Wipe deletes the SE key
//!   (`HardwareKeyDeleted`, Apple parity with Android StrongBox; ADR-0571 — not a
//!   proven permanent erase). The ECIES
//!   wrap/unwrap + the v2 envelope live here (SAFE Security.framework API); the minimal
//!   `unsafe` SE-key lifecycle is the `apple-secure-enclave` leaf crate.
//! - **[`AppleKeychainVault`] (dev/sim FALLBACK, the chunk-1 path):** no wrap layer — the
//!   `SealKey` lives IN the OS vault as a generic-password item. Reachable ONLY via the
//!   `apple-insecure-raw-keychain-fallback` feature (a best-effort erase: the raw key can
//!   linger in flash free pages). The item-layout / per-OS notes below describe THIS path.
//!
//! Per-OS honesty (as-implemented refinement to §4.3a, S6) — the raw fallback path:
//! - **iOS:** the data-protection keychain (the only keychain there), item
//!   under `SecAccessControl(AccessibleWhenUnlockedThisDeviceOnly)` —
//!   foreground-only unseal need ⇒ the tight class; ThisDeviceOnly keeps it
//!   out of iCloud Keychain and backups.
//! - **macOS:** the login keychain (file-based) — OS-encrypted at rest,
//!   unlocked with the user session, ACL'd to the creating app, and never
//!   synced (items are non-synchronizable by default). Accessibility
//!   *classes* don't apply to the file keychain; the iOS-style
//!   data-protection keychain on macOS requires a signed/provisioned app
//!   (`kSecUseDataProtectionKeychain` ⇒ errSecMissingEntitlement on plain
//!   binaries) — migrating the item there is a named row owed WITH the
//!   first signed macOS build. Until then this is the honest macOS posture,
//!   stated, not hidden.
//!
//! Item value layout (validate-before-use on read):
//! `[ver:1 = WRAP_VERSION_V1][seal_key:32][sha256(sealed_blob):32]`
//! The embedded blob hash is defense-in-depth binding parity with the
//! Android AAD (§4.3a): the vault ACL already prevents foreign writes, but
//! a restored/mismatched blob still fails LOUDLY and layer-attributed.

use security_framework::base::Error as SecError;
use security_framework::key::{Algorithm, SecKey};
use security_framework::passwords;
use security_framework::passwords_options::PasswordOptions;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use super::envelope::{
    APPLE_SE_ECIES_LEN, WRAP_VERSION_V1, encode_apple_se_artifact, parse_apple_se_artifact,
};
use super::{KeychainNamespace, KeychainPort, VaultTier, WrapArtifact};
use crate::error::WalletError;
use crate::seal::SealKey;

/// Keychain service/account identity. Extraction-neutral (no `relim`):
/// versioned like the alias on Android — a future layout bumps the account,
/// never mutates this one in place. FR-13: the production account is SUFFIXED
/// with the per-wallet namespace (`wrap-key.v1.<namespace>`) so two wallets on
/// one device hold DISTINCT keychain items instead of overwriting a global one.
///
/// **BACKWARD-COMPAT (FR-13, pre-prod break):** a wallet provisioned BEFORE FR-13
/// holds its item under the bare `wrap-key.v1` account; post-FR-13 code only ever
/// queries `wrap-key.v1.<namespace>`, so the old item is UNREACHABLE — the wallet
/// surfaces a typed `KeystoreInconsistent` (fail-closed, never a wrong open) and
/// the recovery path is restore-from-mnemonic / re-provision. There is NO
/// automatic migration (acceptable: no shipped consumers). The orphaned old item
/// also lingers as keychain residue (a forensic existence oracle — FR-14 crypto-
/// shred territory). A production promotion that needed in-place upgrade would add
/// a fallback: on a namespaced miss, try the bare account, re-store, delete old.
pub(crate) const APPLE_KEYCHAIN_SERVICE: &str = "zec-wallet.seal";
pub(crate) const APPLE_KEYCHAIN_ACCOUNT: &str = "wrap-key.v1";
/// S2 custody: the path-keyed INDEX item's account base — suffixed with the
/// per-wallet namespace like the wrap account (`custody-index.v1.<namespace>`).
/// A small locator record with NO key material; one item per wallet path.
pub(crate) const APPLE_INDEX_ACCOUNT: &str = "custody-index.v1";
/// Device-E2E selftest item — disjoint service, never production custody.
pub(crate) const APPLE_SELFTEST_SERVICE: &str = "zec-wallet.seal.selftest";

/// `errSecItemNotFound` / `errSecInteractionNotAllowed` (Security/SecBase.h)
/// — the two OSStatus values we map to distinct typed errors.
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;
const ERR_SEC_INTERACTION_NOT_ALLOWED: i32 = -25308;
/// `errSecNotAvailable` (no keychain / not unlocked) / `errSecAuthFailed` (auth could
/// not complete) — the rest of the TRANSIENT, retryable set (a LOCKED device, not a
/// permanent fault). Distinguishing these from a genuine decrypt failure is what keeps a
/// locked-screen background load from masquerading as wallet corruption.
const ERR_SEC_NOT_AVAILABLE: i32 = -25291;
const ERR_SEC_AUTH_FAILED: i32 = -25293;

/// Is this `OSStatus`/`CFError` code a TRANSIENT keychain condition (device locked /
/// interaction unavailable) that the caller should RETRY, rather than a permanent fault
/// or a corruption signal? ONE source of truth for the retryable set, shared by the SE
/// decrypt path and (for parity) consistent with the raw `AppleKeychainVault::map_err`.
fn is_transient_keychain_error(code: i32) -> bool {
    matches!(
        code,
        ERR_SEC_INTERACTION_NOT_ALLOWED | ERR_SEC_NOT_AVAILABLE | ERR_SEC_AUTH_FAILED
    )
}

const KEY_LEN: usize = 32;
const HASH_LEN: usize = 32;
const ITEM_LEN: usize = 1 + KEY_LEN + HASH_LEN;

pub(crate) struct AppleKeychainVault {
    service: String,
    account: String,
    /// The S2 custody index item's account (per-namespace, same service).
    index_account: String,
}

impl AppleKeychainVault {
    /// Production custody, NAMESPACED per wallet (FR-13): the account is suffixed
    /// with `namespace` so each wallet's seal is a distinct keychain item — a
    /// second wallet on the same device never overwrites the first's `SealKey`.
    /// The service stays constant (the app-wide seal domain); the account carries
    /// the per-wallet discriminator (the natural keychain shape: service = domain,
    /// account = the specific credential).
    ///
    /// `namespace` is a validated [`KeychainNamespace`] (a fixed-length lowercase-hex
    /// token) — the TYPE enforces the well-formed-account-name invariant in every
    /// build, so no `debug_assert` is owed.
    pub(crate) fn new(namespace: &KeychainNamespace) -> Self {
        Self {
            service: APPLE_KEYCHAIN_SERVICE.to_owned(),
            account: format!("{APPLE_KEYCHAIN_ACCOUNT}.{}", namespace.as_str()),
            index_account: format!("{APPLE_INDEX_ACCOUNT}.{}", namespace.as_str()),
        }
    }

    /// Device/host-E2E vault: same code path, DISJOINT service — the
    /// selftest can store/wipe freely without touching production custody.
    pub(crate) fn selftest() -> Self {
        Self {
            service: APPLE_SELFTEST_SERVICE.to_owned(),
            account: APPLE_KEYCHAIN_ACCOUNT.to_owned(),
            index_account: format!("{APPLE_INDEX_ACCOUNT}.selftest"),
        }
    }

    /// Test-scoped: a unique service name so the smoke test never touches
    /// (or leaves residue in) the real item.
    #[cfg(test)]
    pub(crate) fn with_service(service: &str) -> Self {
        Self {
            service: service.to_owned(),
            account: APPLE_KEYCHAIN_ACCOUNT.to_owned(),
            index_account: APPLE_INDEX_ACCOUNT.to_owned(),
        }
    }

    fn map_err(e: &SecError) -> WalletError {
        match e.code() {
            // Absent while a sealed blob exists — the keysMissing class
            // (§4.2a): surfaced, never treated as a fresh wallet.
            ERR_SEC_ITEM_NOT_FOUND => WalletError::KeystoreInconsistent {
                permanently_invalidated: false,
            },
            // Locked keychain / no interaction allowed — retryable.
            ERR_SEC_INTERACTION_NOT_ALLOWED => WalletError::KeystoreUnavailable,
            _ => WalletError::KeystoreUnavailable,
        }
    }

    fn item_value(key: &SealKey, sealed_blob: &[u8]) -> Zeroizing<Vec<u8>> {
        let mut value = Zeroizing::new(Vec::with_capacity(ITEM_LEN));
        value.push(WRAP_VERSION_V1);
        value.extend_from_slice(key.as_bytes());
        value.extend_from_slice(&Sha256::digest(sealed_blob));
        value
    }

    /// iOS: pin the tight accessibility class via `SecAccessControl`.
    /// macOS: the plain login-keychain add (see module docs for WHY).
    fn write_item(&self, value: &[u8]) -> Result<(), WalletError> {
        self.write_item_at(&self.account, value)
    }

    /// [`Self::write_item`] for an arbitrary account under this vault's
    /// service (the S2 index item rides the same door and the same
    /// accessibility discipline).
    fn write_item_at(&self, account: &str, value: &[u8]) -> Result<(), WalletError> {
        write_keychain_item(&self.service, account, value)
    }
}

/// `kSecAttrAccessible`'s stored value for `WhenUnlockedThisDeviceOnly` (the
/// `pdmn` attribute an item search returns; Security/SecItem.h).
#[cfg(any(test, target_os = "ios"))]
const PDMN_WHEN_UNLOCKED_THIS_DEVICE_ONLY: &str = "aku";

/// How the iOS write door writes, from what already stands under the identity.
#[cfg(any(test, target_os = "ios"))]
#[derive(Debug, PartialEq, Eq)]
enum IosWrite {
    /// No item: add one, pinned.
    Add,
    /// Every matching item pinned: update the value IN PLACE — one `SecItemUpdate`,
    /// so there is never a moment with no item (S6 review LOW on `a58e7cb2`: the
    /// index written delete-then-add, killed between the two, left no index, and a
    /// bare-config wipe after a host file delete answered success with a key left).
    UpdateInPlace,
    /// Any matching item without the pin (a layout from before the door pinned
    /// it, or an accessibility that did not read): delete them all, then add
    /// pinned — the one path with a window, taken once per such identity. Never
    /// an update, which would write the value into the loose item too.
    ReplaceUnpinned,
}

/// `standing` = the accessibility (`pdmn`) of EVERY item matching the identity,
/// `""` where it did not read; empty = no item.
#[cfg(any(test, target_os = "ios"))]
fn ios_write_for(standing: &[String]) -> IosWrite {
    if standing.is_empty() {
        IosWrite::Add
    } else if standing
        .iter()
        .all(|pdmn| pdmn == PDMN_WHEN_UNLOCKED_THIS_DEVICE_ONLY)
    {
        IosWrite::UpdateInPlace
    } else {
        IosWrite::ReplaceUnpinned
    }
}

/// The ONE generic-password write door for this module: on iOS the item is
/// pinned `WhenUnlockedThisDeviceOnly` (never in a backup, never restored onto
/// another device) and a pinned item is updated in place, never deleted first
/// (`IosWrite`); on macOS the plain login-keychain add-or-update. Both vaults'
/// items — the raw wrap key and the S2 index item of either tier — go through
/// here, so none can miss the pin.
fn write_keychain_item(service: &str, account: &str, value: &[u8]) -> Result<(), WalletError> {
    #[cfg(target_os = "ios")]
    {
        use core_foundation::data::CFData;
        use security_framework::access_control::{ProtectionMode, SecAccessControl};
        use security_framework::item::{
            ItemClass, ItemSearchOptions, ItemUpdateOptions, ItemUpdateValue, Limit, update_item,
        };

        let this_item = || {
            let mut search = ItemSearchOptions::new();
            search
                .class(ItemClass::generic_password())
                .service(service)
                .account(account);
            search
        };
        // EVERY match, not the first: an update changes every item the query
        // matches (another access group can hold a second one), so it is taken
        // only when all of them carry the pin.
        let standing: Vec<String> =
            match this_item().load_attributes(true).limit(Limit::All).search() {
                Ok(found) => found
                    .iter()
                    .map(|item| {
                        item.simplify_dict()
                            .and_then(|attrs| attrs.get("pdmn").cloned())
                            .unwrap_or_default()
                    })
                    .collect(),
                Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => Vec::new(),
                Err(e) => return Err(AppleKeychainVault::map_err(&e)),
            };
        let write = ios_write_for(&standing);
        if write == IosWrite::UpdateInPlace {
            // An item deleted since the search stays deleted — never re-added
            // behind whatever deleted it — and the write answers the retryable
            // `KeystoreUnavailable`, not `map_err`'s keysMissing class (a
            // missing item is a READ's finding, not this write's).
            let mut update = ItemUpdateOptions::new();
            update.set_value(ItemUpdateValue::Data(CFData::from_buffer(value)));
            return update_item(&this_item(), &update).map_err(|e| {
                if e.code() == ERR_SEC_ITEM_NOT_FOUND {
                    WalletError::KeystoreUnavailable
                } else {
                    AppleKeychainVault::map_err(&e)
                }
            });
        }
        let access = SecAccessControl::create_with_protection(
            Some(ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly),
            0,
        )
        .map_err(|e| AppleKeychainVault::map_err(&e))?;
        let mut options = PasswordOptions::new_generic_password(service, account);
        options.set_access_control(access);
        if write == IosWrite::ReplaceUnpinned {
            // A delete that failed would leave the loose item, and the add below
            // would fall back to updating it (duplicate → `SecItemUpdate`) and
            // answer Ok: only "deleted" or "already gone" may proceed.
            match this_item().delete() {
                Ok(()) => {}
                Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => {}
                Err(e) => return Err(AppleKeychainVault::map_err(&e)),
            }
        }
        passwords::set_generic_password_options(value, options)
            .map_err(|e| AppleKeychainVault::map_err(&e))
    }
    #[cfg(not(target_os = "ios"))]
    {
        passwords::set_generic_password(service, account, value)
            .map_err(|e| AppleKeychainVault::map_err(&e))
    }
}

impl KeychainPort for AppleKeychainVault {
    fn probe(&self) -> Result<(), WalletError> {
        // The keychain is a platform constant on Apple targets; per-item
        // failures surface on the operations themselves.
        Ok(())
    }

    fn tier(&self) -> Result<VaultTier, WalletError> {
        Ok(VaultTier::AppleKeychain)
    }

    fn store_wrap_key(
        &self,
        key: SealKey,
        sealed_blob: &[u8],
    ) -> Result<WrapArtifact, WalletError> {
        let value = Self::item_value(&key, sealed_blob);
        self.write_item(&value)?;
        // Nothing secret on disk: the artifact is a 1-byte version marker
        // (the vault item IS the custody).
        Ok(WrapArtifact::from_freshly_wrapped(vec![WRAP_VERSION_V1]))
    }

    fn load_wrap_key(
        &self,
        artifact: &WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<SealKey, WalletError> {
        // Validate the marker before touching the vault (an Android-shaped
        // 65-byte artifact here means crossed wires — loud, attributed).
        match artifact.as_bytes() {
            [v] if *v == WRAP_VERSION_V1 => {}
            [v, ..] if *v != WRAP_VERSION_V1 => {
                return Err(WalletError::WrapVersionUnsupported { found: *v });
            }
            _ => return Err(WalletError::WrapArtifactInvalid),
        }

        let options = PasswordOptions::new_generic_password(&self.service, &self.account);
        let value =
            Zeroizing::new(passwords::generic_password(options).map_err(|e| Self::map_err(&e))?);
        if value.len() != ITEM_LEN || value[0] != WRAP_VERSION_V1 {
            // The vault item itself is malformed — wrap-layer corruption.
            return Err(WalletError::WrapArtifactInvalid);
        }
        let (key_bytes, stored_hash) = value[1..].split_at(KEY_LEN);
        if stored_hash != Sha256::digest(sealed_blob).as_slice() {
            // Blob↔item mismatch (restored blob, swapped file): the same
            // loud, layer-attributed failure as the Android AAD binding.
            return Err(WalletError::WrapArtifactInvalid);
        }
        SealKey::from_bytes(Zeroizing::new(key_bytes.to_vec()))
            .map_err(|_| WalletError::WrapArtifactInvalid)
    }

    fn rotate_wrap_key(
        &self,
        artifact: &WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<WrapArtifact, WalletError> {
        // Rotation is vacuous on Apple (§4.3a): there is no alias key to
        // rotate — the item is re-written in place by the OS atomically and
        // survives passcode changes. Verify the pair still opens, then
        // return the equivalent artifact.
        let _ = self.load_wrap_key(artifact, sealed_blob)?;
        Ok(WrapArtifact::from_freshly_wrapped(vec![WRAP_VERSION_V1]))
    }

    fn finish_rotation(&self, _old: &WrapArtifact, _new: &WrapArtifact) -> Result<(), WalletError> {
        // Nothing superseded exists; deleting here would delete the LIVE
        // item (the no-lockout rule) — structural no-op.
        Ok(())
    }

    fn delete_wrap_key(&self, _artifact: &WrapArtifact) -> Result<(), WalletError> {
        match passwords::delete_generic_password(&self.service, &self.account) {
            Ok(()) => Ok(()),
            // Idempotent: already severed is the terminal no-op.
            Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(()),
            Err(e) => Err(Self::map_err(&e)),
        }
    }

    fn purge_namespace(&self) -> Result<usize, WalletError> {
        // Apple is one item per namespace (no alias generations): severing the
        // namespaced account IS the whole-namespace purge. Count it (1 if it
        // existed, 0 if already gone) — the verify-real-sever signal.
        let severed = match passwords::delete_generic_password(&self.service, &self.account) {
            Ok(()) => 1,
            Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => 0,
            Err(e) => return Err(Self::map_err(&e)),
        };
        // Best-effort legacy sweep (FR-13→FR-14 residue), UNCOUNTED — see the shared helper.
        sweep_bare_legacy_item(&self.service, &self.account);
        // The S2 index item is NOT swept: a purge is wrap material only; the
        // index's one deleter is the wipe, LAST (`delete_index`; ADR-0560).
        Ok(severed)
    }

    fn store_index(&self, entry: &crate::custody::CustodyIndexEntry) -> Result<(), WalletError> {
        self.write_item_at(&self.index_account, &entry.encode())
    }

    fn load_index(&self) -> Result<Option<crate::custody::CustodyIndexEntry>, WalletError> {
        read_index_item(&self.service, &self.index_account)
    }

    fn delete_index(&self) -> Result<(), WalletError> {
        delete_index_item(&self.service, &self.index_account);
        Ok(())
    }
}

/// S2 custody — the shared index-item readers (both Apple vaults store the
/// index the same way: a generic-password item under the vault's service).
/// `Ok(None)` = no item (a pre-stage wallet); a PRESENT-but-malformed item is
/// the typed keysMissing class, never a guess.
fn read_index_item(
    service: &str,
    account: &str,
) -> Result<Option<crate::custody::CustodyIndexEntry>, WalletError> {
    let options = PasswordOptions::new_generic_password(service, account);
    match passwords::generic_password(options) {
        Ok(bytes) => crate::custody::CustodyIndexEntry::decode(&bytes).map(Some),
        Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(None),
        Err(e) => Err(AppleKeychainVault::map_err(&e)),
    }
}

/// S2 custody — idempotent index-item delete; best-effort by design (the
/// crypto-shred does not depend on it, and a wipe must not fail on a
/// keychain entry that is already gone).
fn delete_index_item(service: &str, account: &str) {
    let _ = passwords::delete_generic_password(service, account);
}

/// Best-effort, UNCOUNTED sweep of the pre-FR-13 BARE legacy keychain item (process-
/// global — only one ever existed). Scoped to the production service so a
/// selftest/`with_service`/`with_tag` vault (DISJOINT service) never reaches it. NOT
/// counted toward any verify-real-sever signal — it is not the caller's namespaced
/// load-bearing item, and counting it could mask a namespace mismatch at the fail-closed
/// `store::destroy` gate. Shared by BOTH Apple vaults' `purge_namespace` (DRY: one place
/// owns the bare-account identity + the production-service scoping).
///
/// TODO(prod-promotion): this unconditionally severs the bare item on EVERY namespaced
/// wipe — safe pre-prod (no shipped pre-FR-13 consumers), but once an in-place legacy
/// migration ships, gate it behind a "no live legacy item" sentinel so wiping a NEW
/// wallet can't clobber an unmigrated legacy wallet's custody (FR-13 backward-compat note
/// / the eventual migration ADR).
fn sweep_bare_legacy_item(service: &str, account: &str) {
    if service == APPLE_KEYCHAIN_SERVICE && account != APPLE_KEYCHAIN_ACCOUNT {
        let _ = passwords::delete_generic_password(APPLE_KEYCHAIN_SERVICE, APPLE_KEYCHAIN_ACCOUNT);
    }
}

// ───────────────────────── FR-14 chunk 2 — the Secure-Enclave custody vault ─────────────────────────

/// Secure-Enclave application-tag BASE (FROZEN, append-only). The per-wallet tag is
/// `<base>.<namespace>` — the ONLY `purge` locator for an SE key. A rename/renumber
/// ORPHANS every provisioned SE key and SILENTLY defeats the crypto-shred, so it is
/// pinned by `apple_se_tag_base_pinned`. Extraction-neutral (no `relim`).
pub(crate) const APPLE_SE_TAG_BASE: &str = "zec-wallet.se-wrap.v1";

/// The pinned ECIES variant (§4.3a chunk 2): a per-encryption ephemeral key ⇒ no
/// IV-reuse, the Secure Enclave's only supported curve (P-256). Audited
/// Security.framework primitive WHOLE — no custom crypto (Rule Zero).
const SE_ECIES_ALG: Algorithm = Algorithm::ECIESEncryptionCofactorVariableIVX963SHA256AESGCM;

/// FR-14 chunk 2 SE-error mapping — the ONE source of truth, SE-REQUIRED and
/// FAIL-CLOSED by DEFAULT. Every `SeError::Status` (including `-34018`
/// errSecMissingEntitlement on an unsigned dev binary) collapses to the typed,
/// retryable `KeystoreUnavailable`: there is NO error-code guess that downgrades a
/// capable device to flash-recoverable raw custody (the asymmetric-failure money
/// risk — misclassifying capable→no-SE reopens AV2). An ambiguous tag (>1 SE key, a
/// crashed re-provision could leave one) → `KeystoreInconsistent`: never silently
/// pick one. The raw fallback is a CONSTRUCTION-time choice (the
/// `apple-insecure-raw-keychain-fallback` feature), never a runtime reaction here.
fn map_se_err(e: apple_secure_enclave::SeError) -> WalletError {
    use apple_secure_enclave::SeError;
    match e {
        SeError::Ambiguous => WalletError::KeystoreInconsistent {
            permanently_invalidated: false,
        },
        SeError::Status(_) => WalletError::KeystoreUnavailable,
    }
}

/// ECIES-wrap the 32-byte SealKey to `recipient_pub` (the SE key's public half; a
/// software P-256 public key in the host KAT — the operation and its 113-byte output
/// length are curve-determined, identical either way). The ephemeral key is generated
/// inside Security.framework per call. Returns the exact-length blob; a length other
/// than [`APPLE_SE_ECIES_LEN`] means a wrong algorithm/curve reached the primitive —
/// fail closed, never persist a malformed artifact.
fn ecies_wrap(
    recipient_pub: &SecKey,
    seal_key: &SealKey,
) -> Result<[u8; APPLE_SE_ECIES_LEN], WalletError> {
    let ct = recipient_pub
        .encrypt_data(SE_ECIES_ALG, seal_key.as_bytes())
        .map_err(|_| WalletError::KeystoreUnavailable)?;
    // The measured 113-byte length is an invariant; `try_into` fails closed (never
    // persisting garbage) if a different length means a wrong algorithm/curve reached
    // the primitive, and yields a fixed array the encoder can't misuse.
    ct.try_into().map_err(|_| WalletError::WrapArtifactInvalid)
}

/// ECIES-unwrap the SealKey bytes with the SE PRIVATE key (the decrypt runs INSIDE the
/// enclave; only the 32-byte plaintext returns). Zeroizing on the way out.
///
/// Error fidelity: the caller has ALREADY verified the blob-hash
/// bind and located the SE key under our tag, so a decrypt failure is one of two things —
/// a TRANSIENT locked-keychain/interaction condition (the SE key is `WhenUnlocked`; a
/// background/lock-screen load hits `errSecInteractionNotAllowed`), which is RETRYABLE
/// (`KeystoreUnavailable`) and must NOT masquerade as corruption (that would steer a host
/// to a destructive re-provision of an intact wallet) — or a genuine wrong-key/tampered-
/// ECIES failure (GCM tag), the `WrapArtifactInvalid` anti-substitution signal. The exact
/// `CFError` code carried for the locked case is verified on the iPhone gate; classifying
/// the KNOWN transient codes is strictly safer than today's blanket-corrupt and never
/// regresses (an unknown code still falls to `WrapArtifactInvalid`).
fn ecies_unwrap(se_key: &SecKey, ecies: &[u8]) -> Result<Zeroizing<Vec<u8>>, WalletError> {
    let pt = se_key.decrypt_data(SE_ECIES_ALG, ecies).map_err(|e| {
        if is_transient_keychain_error(e.code() as i32) {
            WalletError::KeystoreUnavailable
        } else {
            WalletError::WrapArtifactInvalid
        }
    })?;
    Ok(Zeroizing::new(pt))
}

/// FR-14 chunk 2 — the PRODUCTION Apple custody backend. The 32-byte SealKey is
/// ECIES-wrapped under a NON-EXTRACTABLE Secure-Enclave P-256 key: the wrapped blob is
/// the on-disk v2 [`WrapArtifact`], the wrap key lives in hardware located by a
/// per-namespace application tag (the same custody shape as Android — wrapped blob on
/// disk, wrap key in the secure element; different primitive). Deleting the SE key
/// (`purge_namespace`) leaves this key store unable to open the on-disk blob (Apple
/// parity with Android StrongBox). An earlier copy of the SE key's keychain item is
/// not proven unusable on the same device, so this is `HardwareKeyDeleted`, not a
/// permanent erase (ADR-0571).
///
/// SE-REQUIRED, fail-closed (see [`map_se_err`]): the ONLY sanctioned downgrade to the
/// raw [`AppleKeychainVault`] is the explicit `apple-insecure-raw-keychain-fallback`
/// build feature for dev/sim — never a runtime decision on a capable device.
pub(crate) struct AppleSecureEnclaveVault {
    /// Per-namespace SE application tag (the only `purge`/`find` locator).
    se_tag: Vec<u8>,
    /// The raw/legacy generic-password identity under THIS namespace — held ONLY so
    /// `purge_namespace` can defensively sweep a raw item (a fallback→SE mode switch,
    /// or an anti-downgrade forged item) and the pre-FR-13 bare legacy item. SE custody
    /// itself writes NO generic-password item (the artifact is the wrapped blob).
    service: String,
    account: String,
    /// The S2 custody index item's account (per-namespace, same service) — SE
    /// custody writes no generic-password WRAP item, but the index is a small
    /// locator record and rides the generic-password store.
    index_account: String,
}

impl AppleSecureEnclaveVault {
    /// Production custody, NAMESPACED per wallet (FR-13): the SE application tag is
    /// `<base>.<namespace>` so each wallet's wrap key is a distinct SE key.
    pub(crate) fn new(namespace: &KeychainNamespace) -> Self {
        Self {
            se_tag: format!("{APPLE_SE_TAG_BASE}.{}", namespace.as_str()).into_bytes(),
            service: APPLE_KEYCHAIN_SERVICE.to_owned(),
            account: format!("{APPLE_KEYCHAIN_ACCOUNT}.{}", namespace.as_str()),
            index_account: format!("{APPLE_INDEX_ACCOUNT}.{}", namespace.as_str()),
        }
    }

    /// Device/host-E2E vault: a DISJOINT tag (the `selftest` suffix is not 32-hex, so it
    /// can never collide with a real `<base>.<32-hex-namespace>` tag) + the disjoint
    /// selftest service — the SE custody round-trip (incl. the SE-key delete purge) runs on
    /// the connected iPhone without touching production custody.
    pub(crate) fn selftest() -> Self {
        Self {
            se_tag: format!("{APPLE_SE_TAG_BASE}.selftest").into_bytes(),
            service: APPLE_SELFTEST_SERVICE.to_owned(),
            account: APPLE_KEYCHAIN_ACCOUNT.to_owned(),
            index_account: format!("{APPLE_INDEX_ACCOUNT}.selftest"),
        }
    }

    /// Test-scoped: a unique SE tag + disjoint service so the macOS-signed gate never
    /// touches (or leaves residue in) real custody.
    #[cfg(test)]
    pub(crate) fn with_tag(tag: &str) -> Self {
        Self {
            se_tag: tag.as_bytes().to_vec(),
            service: format!("{APPLE_SELFTEST_SERVICE}.{tag}"),
            account: APPLE_KEYCHAIN_ACCOUNT.to_owned(),
            index_account: APPLE_INDEX_ACCOUNT.to_owned(),
        }
    }
}

impl KeychainPort for AppleSecureEnclaveVault {
    fn probe(&self) -> Result<(), WalletError> {
        // The keychain is a platform constant; SE-capability failures surface on the
        // store path (fail-closed there), not as a cheap pre-flight that would have to
        // generate a key to be meaningful.
        Ok(())
    }

    fn tier(&self) -> Result<VaultTier, WalletError> {
        // ACTIVE probe (never a static overclaim, mirroring Android's measured_tier): an
        // SE key under this namespace's tag ⇒ AppleSecureEnclave (SE-key delete); none ⇒ the
        // honest AppleKeychain floor. Read AFTER store by the orchestration, so a
        // provisioned wallet always reports AppleSecureEnclave.
        match apple_secure_enclave::find(&self.se_tag).map_err(map_se_err)? {
            Some(_) => Ok(VaultTier::AppleSecureEnclave),
            None => Ok(VaultTier::AppleKeychain),
        }
    }

    fn store_wrap_key(
        &self,
        key: SealKey,
        sealed_blob: &[u8],
    ) -> Result<WrapArtifact, WalletError> {
        // Delete-FIRST: a tag is not a keychain uniqueness key — a crashed prior
        // provision could leave a second SE key under it → load ambiguity / lockout.
        apple_secure_enclave::delete_all(&self.se_tag).map_err(map_se_err)?;
        let se_key = apple_secure_enclave::generate(&self.se_tag).map_err(map_se_err)?;
        // The public key derives in software from the SE key handle; the private half
        // never leaves the enclave.
        let pubkey = se_key
            .public_key()
            .ok_or(WalletError::KeystoreUnavailable)?;
        let ecies = ecies_wrap(&pubkey, &key)?;
        let mut blob_hash = [0u8; 32];
        blob_hash.copy_from_slice(&Sha256::digest(sealed_blob));
        // The artifact IS the on-disk custody (wrapped blob); nothing secret in a
        // generic-password item for SE custody.
        Ok(WrapArtifact::from_freshly_wrapped(
            encode_apple_se_artifact(&blob_hash, &ecies),
        ))
    }

    fn load_wrap_key(
        &self,
        artifact: &WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<SealKey, WalletError> {
        // Validate-before-use: version + exact length (a forged v1 downgrade or a
        // truncated blob dies BEFORE any vault/crypto call).
        let parsed = parse_apple_se_artifact(artifact.as_bytes())?;
        // Blob-bind BEFORE decrypt (application-layer anti-substitution; ECIES has no
        // AAD param). A swapped blob/artifact within a namespace fails here, loud.
        let mut blob_hash = [0u8; 32];
        blob_hash.copy_from_slice(&Sha256::digest(sealed_blob));
        if parsed.blob_hash != blob_hash {
            return Err(WalletError::WrapArtifactInvalid);
        }
        // Anti-downgrade is enforced ABOVE (a forged v1 artifact is rejected by
        // `parse_apple_se_artifact` as `WrapVersionUnsupported` — the SE vault has no raw
        // read path to dispatch to). Locate the SE key: absent while an artifact exists =
        // the keysMissing class (a deleted/orphaned key, OR a data-dir restored to a NEW
        // device where the `ThisDeviceOnly` SE key didn't travel — recover via
        // `wipe_force` + restore-from-seed) — surfaced, never treated as a fresh wallet.
        // HOST CONTRACT: load only with the device UNLOCKED — a `WhenUnlocked` SE key is
        // filtered while locked, so a locked load can spuriously read as keysMissing.
        let se_key = apple_secure_enclave::find(&self.se_tag)
            .map_err(map_se_err)?
            .ok_or(WalletError::KeystoreInconsistent {
                permanently_invalidated: false,
            })?;
        // The real anti-substitution: a foreign artifact was encrypted to a DIFFERENT SE
        // public key → decrypt fails even if the blob hash somehow matched.
        let pt = ecies_unwrap(&se_key, &parsed.ecies)?;
        SealKey::from_bytes(pt).map_err(|_| WalletError::WrapArtifactInvalid)
    }

    fn rotate_wrap_key(
        &self,
        artifact: &WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<WrapArtifact, WalletError> {
        // VACUOUS on Apple SE (§4.3a chunk 2): ECIES uses a fresh ephemeral key per
        // encryption ⇒ no IV budget to exhaust; the SE key is non-extractable ⇒ no
        // compromise-rotate. Verify the pair still opens, then return the SAME artifact.
        let _ = self.load_wrap_key(artifact, sealed_blob)?;
        Ok(WrapArtifact::from_freshly_wrapped(
            artifact.as_bytes().to_vec(),
        ))
    }

    fn finish_rotation(&self, old: &WrapArtifact, new: &WrapArtifact) -> Result<(), WalletError> {
        // Rotation is vacuous, so old MUST equal new. The debug-assert is the guard the
        // spec mandates: a future generations change that let them diverge would delete
        // the LIVE SE key here (violating the no-lockout rule) — caught in debug builds.
        debug_assert_eq!(
            old.as_bytes(),
            new.as_bytes(),
            "Apple SE rotation is vacuous; finish_rotation must not sever a divergent key"
        );
        let _ = (old, new);
        Ok(())
    }

    fn delete_wrap_key(&self, _artifact: &WrapArtifact) -> Result<(), WalletError> {
        // Artifact-keyed single delete (rotation-finish / SealedSeedVault::wipe). On
        // Apple SE there is exactly one key per namespace tag — sever it. Idempotent
        // (delete_all returns Ok(0) when already gone).
        apple_secure_enclave::delete_all(&self.se_tag).map_err(map_se_err)?;
        Ok(())
    }

    fn purge_namespace(&self) -> Result<usize, WalletError> {
        // FR-14 crypto-shred. The load-bearing verify-real-sever COUNT = SE keys
        // actually deleted (the SE-key delete): `store::destroy` fails CLOSED on count==0
        // over a non-empty store, so a mis-derived tag can never report a wipe that left
        // the real wrap key live + the seals flash-recoverable. Sever the SE key(s)
        // FIRST; a non-transient SE failure propagates (files stay until the sever
        // completes). On an unsigned/SE-less build delete_all itself fails → fail-closed.
        // (a) The load-bearing SE-key delete: sever every SE key under the tag, COUNTED.
        let se_severed = apple_secure_enclave::delete_all(&self.se_tag).map_err(map_se_err)?;
        // Defensive UNCOUNTED sweeps (orphan-safety, NOT the load-bearing count, so a
        // raw-item deletion can never mask a missing SE sever at the verify gate):
        //  (b) any namespaced RAW generic-password item — present only if this namespace
        //      was ever provisioned under the raw fallback tier (dev/sim), or a
        //      forged-downgrade artifact tried to plant one;
        //  (c) the pre-FR-13 legacy BARE item (the chunk-1 sweep; shared helper).
        // CROSS-TIER edge (dev/sim only): a wallet provisioned under the raw fallback then
        // wiped by a production SE build severs the real (raw) custody HERE but returns
        // se_severed==0, so `store::destroy` fails closed (`KeystoreInconsistent`) and the
        // host must `wipe_force` — the SAFE direction (the secret IS gone; the gate just
        // refuses to claim success on a count it can't verify).
        let _ = passwords::delete_generic_password(&self.service, &self.account);
        sweep_bare_legacy_item(&self.service, &self.account);
        // The S2 index item is NOT swept: a purge is wrap material only; the
        // index's one deleter is the wipe, LAST (`delete_index`; ADR-0560).
        Ok(se_severed)
    }

    fn store_index(&self, entry: &crate::custody::CustodyIndexEntry) -> Result<(), WalletError> {
        // The index is NOT key material — a generic-password item under the
        // vault's service (see the field doc) — written through the SAME helper
        // as every other item here, so it carries the same
        // `WhenUnlockedThisDeviceOnly` pin on iOS (never in a backup, never
        // restored onto another device).
        write_keychain_item(&self.service, &self.index_account, &entry.encode())
    }

    fn load_index(&self) -> Result<Option<crate::custody::CustodyIndexEntry>, WalletError> {
        read_index_item(&self.service, &self.index_account)
    }

    fn delete_index(&self) -> Result<(), WalletError> {
        delete_index_item(&self.service, &self.index_account);
        Ok(())
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use zeroize::Zeroizing;

    use super::*;
    use crate::keychain::SealedSeedVault;
    use crate::keychain::envelope::WRAP_ARTIFACT_APPLE_SE_LEN;
    use crate::seal::SeedPayload;

    /// Host-runnable software P-256 keypair (`Token::Software`, transient — no SE, no
    /// persistence, works on an UNSIGNED dev Mac) so the ECIES wrap/unwrap + v2 envelope
    /// GLUE is proven WITHOUT a signed binary. The ECIES operation and its 113-byte
    /// output are curve-determined — identical to a real SE key — so this exercises
    /// exactly the production crypto path; only the key's residence differs.
    fn software_p256_keypair() -> SecKey {
        use security_framework::key::{GenerateKeyOptions, KeyType, Token};
        let mut opts = GenerateKeyOptions::default();
        opts.set_key_type(KeyType::ec_sec_prime_random())
            .set_size_in_bits(256)
            .set_token(Token::Software);
        SecKey::new(&opts).expect("software P-256 keygen (unsigned-safe)")
    }

    /// FR-14 chunk 2 frozen-tag pin: a rename orphans every SE key and silently defeats
    /// the crypto-shred (the tag is the only `purge` locator).
    #[test]
    fn apple_se_tag_base_pinned() {
        assert_eq!(APPLE_SE_TAG_BASE, "zec-wallet.se-wrap.v1");
    }

    /// S6 review LOW on `a58e7cb2`: the iOS write door never deletes a PINNED item
    /// before writing it — the custody index rewritten delete-then-add, killed
    /// between the two, left no index for a later bare-config wipe to resolve. A
    /// pinned item is updated in place; any unpinned (older-layout) match, or one
    /// whose accessibility did not read, takes delete-then-add — an update would
    /// write into every match, the loose one included.
    #[test]
    fn ios_write_door_updates_a_pinned_item_in_place() {
        let pdmn = |values: &[&str]| values.iter().map(|v| v.to_string()).collect::<Vec<_>>();
        assert_eq!(PDMN_WHEN_UNLOCKED_THIS_DEVICE_ONLY, "aku");
        assert_eq!(ios_write_for(&pdmn(&["aku"])), IosWrite::UpdateInPlace);
        assert_eq!(
            ios_write_for(&pdmn(&["aku", "aku"])),
            IosWrite::UpdateInPlace
        );
        assert_eq!(ios_write_for(&[]), IosWrite::Add);
        for loose in ["ak", "ck", "cku", "akpu", ""] {
            assert_eq!(
                ios_write_for(&pdmn(&[loose])),
                IosWrite::ReplaceUnpinned,
                "{loose}"
            );
            assert_eq!(
                ios_write_for(&pdmn(&["aku", loose])),
                IosWrite::ReplaceUnpinned,
                "{loose}"
            );
        }
    }

    /// FR-14 chunk 2 — the SE-error mapping is FAIL-CLOSED for every status (the
    /// `se_capability` SSOT; spec). EVERY `Status` (incl. `-34018` errSecMissingEntitlement
    /// on an unsigned dev binary) collapses to the retryable `KeystoreUnavailable` — NO
    /// error code is special-cased into a silent raw-custody downgrade. `Ambiguous` →
    /// `KeystoreInconsistent`.
    #[test]
    fn se_error_mapping_is_fail_closed_for_every_status() {
        use apple_secure_enclave::SeError;
        for code in [-34018, -25291, -25300, -50, 0, 1, i32::MAX, i32::MIN] {
            assert!(
                matches!(
                    map_se_err(SeError::Status(code)),
                    WalletError::KeystoreUnavailable
                ),
                "status {code} must fail closed to retryable KeystoreUnavailable"
            );
        }
        assert!(matches!(
            map_se_err(SeError::Ambiguous),
            WalletError::KeystoreInconsistent {
                permanently_invalidated: false
            }
        ));
    }

    /// FR-14 chunk 2 — the transient-keychain classifier: a
    /// LOCKED-device / interaction-unavailable condition is RETRYABLE; a genuine
    /// fault/corruption is not. Pins the retryable set so an edit can't silently fold a
    /// permanent code into "retry" (which could hide a real custody fault) or drop a
    /// transient code into "corrupt" (which would mislabel a locked load as a dead wallet).
    #[test]
    fn transient_keychain_errors_are_classified_retryable() {
        for code in [
            ERR_SEC_INTERACTION_NOT_ALLOWED,
            ERR_SEC_NOT_AVAILABLE,
            ERR_SEC_AUTH_FAILED,
        ] {
            assert!(
                is_transient_keychain_error(code),
                "{code} must be retryable"
            );
        }
        // A genuine wrong-key/tampered-ECIES (GCM tag), item-not-found, and param errors
        // are NOT transient (they are the WrapArtifactInvalid / keysMissing signals).
        for code in [ERR_SEC_ITEM_NOT_FOUND, -26275, -50, 0] {
            assert!(
                !is_transient_keychain_error(code),
                "{code} must NOT be transient"
            );
        }
    }

    /// FR-14 chunk 2 host KAT (unsigned-runnable): the ECIES wrap/unwrap + v2 envelope
    /// GLUE round-trips a 32-byte SealKey through a software P-256 key, the artifact is
    /// EXACTLY 146 bytes (ECIES exactly 113), validate-before-use rejects a truncated
    /// artifact, and a WRONG key cannot unwrap (the anti-substitution property). The
    /// real-SE persistence + SE-key delete is the device/macOS-signed gate below; this
    /// proves the money-critical glue locally on every `cargo test`.
    #[test]
    fn apple_se_ecies_envelope_roundtrips_and_validates() {
        let key = software_p256_keypair();
        let pubkey = key.public_key().expect("public key");

        let seal_key = SealKey::from_bytes(Zeroizing::new(vec![0x5A; 32])).expect("seal key");
        let sealed_blob = b"the sealed seed+dbkey blob bytes".to_vec();

        let ecies = ecies_wrap(&pubkey, &seal_key).expect("ecies wrap");
        assert_eq!(
            ecies.len(),
            APPLE_SE_ECIES_LEN,
            "measured 113-byte ECIES output"
        );
        let mut blob_hash = [0u8; 32];
        blob_hash.copy_from_slice(&Sha256::digest(&sealed_blob));
        let artifact_bytes = encode_apple_se_artifact(&blob_hash, &ecies);
        assert_eq!(
            artifact_bytes.len(),
            WRAP_ARTIFACT_APPLE_SE_LEN,
            "146-byte v2 artifact"
        );
        let artifact = WrapArtifact::from_bytes(artifact_bytes.clone()).expect("within size cap");

        let parsed = parse_apple_se_artifact(artifact.as_bytes()).expect("parse v2");
        assert_eq!(parsed.blob_hash, blob_hash);
        let recovered = ecies_unwrap(&key, &parsed.ecies).expect("ecies unwrap");
        assert_eq!(recovered.as_slice(), &[0x5A; 32][..], "SealKey round-trips");

        // Validate-before-use: a truncated artifact is rejected before crypto.
        assert!(matches!(
            parse_apple_se_artifact(&artifact_bytes[..WRAP_ARTIFACT_APPLE_SE_LEN - 1]),
            Err(WalletError::WrapArtifactInvalid)
        ));
        // A WRONG key cannot unwrap (GCM tag fails → loud, layer-attributed).
        let other = software_p256_keypair();
        assert!(matches!(
            ecies_unwrap(&other, &parsed.ecies),
            Err(WalletError::WrapArtifactInvalid)
        ));

        // The anti-substitution HOT PATH through the vault's `load_wrap_key`, exercised
        // UNSIGNED: both checks precede the SE `find`, so no SE key is needed.
        let vault = AppleSecureEnclaveVault::with_tag("apple-se-host-kat-blobbind");
        // (1) A tampered/foreign sealed_blob fails the blob-hash bind before decrypt.
        assert!(matches!(
            vault.load_wrap_key(&artifact, b"a different sealed blob entirely"),
            Err(WalletError::WrapArtifactInvalid)
        ));
        // (2) A forged v1 downgrade is rejected at the version check (anti-downgrade).
        let mut forged_v1 = artifact_bytes;
        forged_v1[0] = WRAP_VERSION_V1;
        let forged = WrapArtifact::from_bytes(forged_v1).expect("within size cap");
        assert!(matches!(
            vault.load_wrap_key(&forged, &sealed_blob),
            Err(WalletError::WrapVersionUnsupported { found }) if found == WRAP_VERSION_V1
        ));
    }

    /// FR-14 chunk 2 — the FULL persistent SE custody KAT against a REAL Secure Enclave:
    /// provision (store) → tier==AppleSecureEnclave → reload → the SE key is
    /// NON-EXTRACTABLE → purge severs exactly one SE key (the verify-real-sever count) →
    /// this key store can no longer open the on-disk blob. `#[ignore]`:
    /// persisting an SE key needs the keychain-access-group entitlement (a signed
    /// binary) — `-34018` on a bare unsigned binary. Run via the example app's custody
    /// selftest on the connected iPhone (the PRIMARY gate), or a macOS-signed `just`
    /// recipe. The iOS Simulator's host-file-emulated SE must NOT green this (the
    /// SE-delete assertion is real-hardware only).
    #[test]
    #[ignore = "writes a real Secure-Enclave key — code-signed device gate"]
    fn apple_se_custody_provision_reload_then_purge_is_undecryptable() {
        let tag = format!("{APPLE_SE_TAG_BASE}.test.{}", std::process::id());
        let vault = AppleSecureEnclaveVault::with_tag(&tag);
        let v = SealedSeedVault::new(&vault);
        let payload = SeedPayload::new(Zeroizing::new(vec![0xC7; 32]), None).expect("payload");

        let stored = v.store(&payload).expect("provision via real SE");
        assert_eq!(
            stored.status.tier,
            VaultTier::AppleSecureEnclave,
            "active probe reports the Secure Enclave tier"
        );
        assert_eq!(
            stored.status.tier.erase_assurance(),
            crate::keychain::EraseAssurance::HardwareKeyDeleted,
            "an SE key is hardware-held"
        );
        assert_eq!(
            stored.wrap_artifact.as_bytes().len(),
            146,
            "v2 artifact length"
        );

        let (loaded, _) = v
            .load(&stored.sealed_blob, &stored.wrap_artifact)
            .expect("reload");
        assert_eq!(loaded.seed(), &[0xC7; 32][..]);

        // Non-extractability — the whole SE-key delete claim rests on this.
        let se_key = apple_secure_enclave::find(tag.as_bytes())
            .expect("find")
            .expect("present");
        assert!(
            se_key.external_representation().is_none(),
            "the SE private key must be non-extractable"
        );

        // The crypto-erase: purge severs exactly one SE key...
        assert_eq!(
            vault.purge_namespace().expect("purge"),
            1,
            "one SE key severed"
        );
        // ...and the on-disk blob no longer decrypts (the SealKey is gone).
        assert!(matches!(
            v.load(&stored.sealed_blob, &stored.wrap_artifact),
            Err(WalletError::KeystoreInconsistent { .. })
        ));
        // Idempotent: a second purge severs nothing.
        assert_eq!(vault.purge_namespace().expect("idempotent"), 0);
    }

    /// §4.3a smoke proof against the REAL macOS login keychain — `#[ignore]`
    /// so `just ci` stays hermetic (no developer-keychain writes in CI);
    /// run manually: `cargo test -p zec-wallet-core --lib -- --ignored
    /// wrap_key_roundtrip_via_apple_keychain`. Uses a unique service name
    /// and deletes it on every path.
    #[test]
    #[ignore = "writes to the real macOS keychain — manual gate, run with --ignored"]
    fn wrap_key_roundtrip_via_apple_keychain() {
        let service = format!("zec-wallet.seal.smoke.{}", std::process::id());
        let vault = AppleKeychainVault::with_service(&service);
        let v = SealedSeedVault::new(&vault);

        let payload = SeedPayload::new(
            Zeroizing::new(vec![0x5A; 32]),
            Some(Zeroizing::new("apple smoke phrase".to_owned())),
        )
        .expect("valid payload");

        let stored = v.store(&payload).expect("store via real keychain");
        assert_eq!(stored.status.tier, VaultTier::AppleKeychain);
        assert!(!stored.status.degraded());

        let (loaded, _) = v
            .load(&stored.sealed_blob, &stored.wrap_artifact)
            .expect("load via real keychain");
        assert_eq!(loaded.seed(), &[0x5A; 32][..]);

        // Blob↔item binding holds against the real vault too.
        let mut foreign_blob = stored.sealed_blob.clone();
        let last = foreign_blob.len() - 1;
        foreign_blob[last] ^= 0x01;
        assert!(matches!(
            v.load(&foreign_blob, &stored.wrap_artifact),
            Err(WalletError::WrapArtifactInvalid)
        ));

        // Wipe severs; second wipe is a no-op (idempotent).
        v.wipe(&stored.wrap_artifact).expect("wipe");
        assert!(matches!(
            v.load(&stored.sealed_blob, &stored.wrap_artifact),
            Err(WalletError::KeystoreInconsistent { .. })
        ));
        v.wipe(&stored.wrap_artifact).expect("idempotent wipe");
    }

    /// §8 (FR-13) — two NAMESPACED production vaults coexist in the REAL macOS
    /// keychain: `AppleKeychainVault::new(ns_a)` and `::new(ns_b)` write DISTINCT
    /// items under the shared service, both open, each yields ITS OWN seed (pre-FR-13
    /// B's store would have overwritten A's global item). The host `SharedKeychainVault`
    /// test proves the MODEL; this proves the production `AppleKeychainVault::new`
    /// wiring on the real backend. `#[ignore]` (writes the real keychain) — run
    /// manually: `cargo test -p zec-wallet-core --lib -- --ignored two_namespaces_coexist`.
    /// Test-unique, pid-derived hex namespaces (never a real wallet's), deleted on the
    /// success path.
    #[test]
    #[ignore = "writes to the real macOS keychain — manual gate, run with --ignored"]
    fn two_namespaces_coexist_in_the_real_apple_keychain() {
        let pid = std::process::id();
        let ns_a = KeychainNamespace::new(format!("{pid:032x}")).expect("valid ns");
        let ns_b =
            KeychainNamespace::new(format!("{:032x}", pid.wrapping_add(1))).expect("valid ns");
        let va = AppleKeychainVault::new(&ns_a);
        let vb = AppleKeychainVault::new(&ns_b);
        let sa = SealedSeedVault::new(&va);
        let sb = SealedSeedVault::new(&vb);

        let payload_a = SeedPayload::new(Zeroizing::new(vec![0xA1; 32]), None).expect("payload A");
        let payload_b = SeedPayload::new(Zeroizing::new(vec![0xB2; 32]), None).expect("payload B");

        let stored_a = sa.store(&payload_a).expect("store A");
        // B provisions AFTER A — pre-FR-13 this overwrote A's process-global item.
        let stored_b = sb.store(&payload_b).expect("store B");

        // Both coexist; each opens and recovers ITS OWN seed.
        let (loaded_a, _) = sa
            .load(&stored_a.sealed_blob, &stored_a.wrap_artifact)
            .expect("A still opens after B's intervening store");
        assert_eq!(loaded_a.seed(), &[0xA1; 32][..]);
        let (loaded_b, _) = sb
            .load(&stored_b.sealed_blob, &stored_b.wrap_artifact)
            .expect("B opens");
        assert_eq!(loaded_b.seed(), &[0xB2; 32][..]);

        // Clean up both items (success-path only — a panic above leaves pid-unique,
        // harmless residue that a re-run never re-hits, same posture as the sibling
        // `wrap_key_roundtrip_via_apple_keychain` gate).
        sa.wipe(&stored_a.wrap_artifact).expect("wipe A");
        sb.wipe(&stored_b.wrap_artifact).expect("wipe B");
    }

    /// §8 (FR-14) — `purge_namespace` severs the REAL macOS keychain item (the
    /// artifact-free crypto-shred the `Wallet::wipe` path uses), returning the
    /// count severed, idempotently. The host `wipe_*` tests prove the orchestration
    /// over a model vault; this proves the production `AppleKeychainVault` sever on
    /// the real backend. `#[ignore]` (writes the real keychain) — run manually:
    /// `cargo test -p zec-wallet-core --lib -- --ignored purge_namespace_removes_the_real`.
    #[test]
    #[ignore = "writes to the real macOS keychain — manual gate, run with --ignored"]
    fn purge_namespace_removes_the_real_apple_keychain_item() {
        // pid+2: the sibling coexistence gate claims pid and pid+1 in the SAME
        // process — a shared pid namespace lets this purge sever that test's
        // live item when both `--ignored` gates run in one parallel invocation.
        let pid = std::process::id();
        let ns = KeychainNamespace::new(format!("{:032x}", pid.wrapping_add(2))).expect("valid ns");
        let vault = AppleKeychainVault::new(&ns);
        let v = SealedSeedVault::new(&vault);
        let payload = SeedPayload::new(Zeroizing::new(vec![0xC3; 32]), None).expect("payload");

        let stored = v.store(&payload).expect("store via real keychain");
        assert!(
            v.load(&stored.sealed_blob, &stored.wrap_artifact).is_ok(),
            "the item opens before the purge"
        );

        // The FR-14 artifact-free namespace purge severs the real item (returns 1).
        assert_eq!(
            vault.purge_namespace().expect("purge"),
            1,
            "the namespaced item was severed (the verify-real-sever count)"
        );
        // The seal no longer opens — the SealKey is gone (the crypto-shred).
        assert!(matches!(
            v.load(&stored.sealed_blob, &stored.wrap_artifact),
            Err(WalletError::KeystoreInconsistent { .. })
        ));
        // Idempotent: a second purge severs nothing.
        assert_eq!(
            vault.purge_namespace().expect("idempotent purge"),
            0,
            "already-severed is the terminal no-op"
        );
    }
}
