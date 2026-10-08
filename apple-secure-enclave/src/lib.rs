//! Minimal Apple Secure-Enclave key-lifecycle primitives — the ONE place the
//! wallet SDK uses `unsafe` FFI, isolated here so `zec-wallet-core` stays
//! `#![forbid(unsafe_code)]` (FR-14 chunk 2).
//!
//! **Why this crate exists.** The safe `security-framework =3.7.0` high-level
//! `SecKey::generate` attaches the private-key attribute sub-dictionary
//! (`kSecPrivateKeyAttrs`: permanence + access-control + the searchable
//! application tag) **only under `#[cfg(target_os = "macos")]`** (verified in
//! `key.rs::to_dictionary`). On **iOS** those attributes are built and silently
//! dropped, yielding a NON-PERMANENT, access-control-less, unsearchable SE key —
//! broken for custody, and a bug that would pass a macOS gate and ship broken to
//! iOS. This crate builds the CORRECT `CFDictionary` by hand (the documented
//! Apple pattern) and calls the safe `SecKey::generate(dict)` entry, so the SE
//! key is persistent, device-unlock-gated, and locatable by application tag on
//! BOTH iOS and macOS.
//!
//! **Scope (SRP).** The SE-key LIFECYCLE by application tag only: `generate`,
//! `find`, `delete_all`. The ECIES wrap/unwrap, the wrap envelope, the
//! `VaultTier`, and the SE-availability fallback policy live in `zec-wallet-core`'s
//! audited (safe) keychain adapter. The SE PRIVATE KEY never leaves the enclave;
//! no key material or application tag is held or logged here.
//!
//! **macOS keychain.** This crate does NOT set `kSecUseDataProtectionKeychain`:
//! iOS has only the data-protection keychain; macOS uses the login keychain (no
//! entitlement to choose it, though persisting an SE key on macOS still requires
//! a code-signed binary with a keychain-access-group entitlement — the gate runs
//! via an ad-hoc `codesign` harness). Migrating macOS to the data-protection
//! keychain for full iOS parity is a named row owed with the first signed macOS
//! build, mirroring the chunk-1 `apple.rs` deferral.
//!
//! CANNOT be fully validated on a host build: SE persistence is device-gated
//! (iOS device + the macOS-signed `#[ignore]` gate). Compile-checked everywhere.

#![cfg(any(target_os = "ios", target_os = "macos"))]

use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::{CFType, CFTypeRef, TCFType, ToVoid};
use core_foundation::boolean::CFBoolean;
use core_foundation::data::CFData;
use core_foundation::dictionary::CFMutableDictionary;
use core_foundation::number::CFNumber;

use core_foundation_sys::string::CFStringRef;
use security_framework::access_control::{ProtectionMode, SecAccessControl};
use security_framework::key::SecKey;
use security_framework_sys::base::{SecKeyRef, errSecItemNotFound, errSecSuccess};
use security_framework_sys::item::{
    kSecAttrAccessControl, kSecAttrIsPermanent, kSecAttrKeySizeInBits, kSecAttrKeyType,
    kSecAttrKeyTypeECSECPrimeRandom, kSecAttrTokenID, kSecAttrTokenIDSecureEnclave, kSecClass,
    kSecClassKey, kSecMatchLimit, kSecMatchLimitAll, kSecPrivateKeyAttrs, kSecReturnRef,
};
use security_framework_sys::keychain_item::SecItemCopyMatching;

// `security-framework-sys 2.17.0` omits `kSecAttrApplicationTag` — Apple's
// documented searchable identifier for an application's private key (the SE
// sample-code locator). Declare it directly against the Security framework that
// `security-framework-sys` already links. A CFString*Ref* constant; we only ever
// pass it as a dictionary key (`to_void`), never deref it.
unsafe extern "C" {
    static kSecAttrApplicationTag: CFStringRef;
}

/// `kSecAccessControlPrivateKeyUsage` (SecAccessControl.h, API-frozen `1 << 30`):
/// the SE private key may be used for sign/decrypt. No biometry flag — the key is
/// usable whenever the protection class (device unlocked) permits.
const PRIVATE_KEY_USAGE: core_foundation::base::CFOptionFlags = 1 << 30;

/// P-256 (`kSecAttrKeyTypeECSECPrimeRandom`) — the only curve the Secure Enclave
/// supports.
const EC_KEY_SIZE_BITS: i32 = 256;

/// An SE keychain-op failure. Carries the raw `OSStatus`/`CFError` code so the
/// caller (`zec-wallet-core`'s `apple.rs`) owns ALL the fail-closed policy mapping —
/// this crate makes no `WalletError` decision and renders no message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeError {
    /// `SecKeyCreateRandomKey` / `SecItemCopyMatching` / `SecItemDelete` returned
    /// this `OSStatus` (or a `CFError` carrying it).
    Status(i32),
    /// More than one SE key matched a single application tag — a uniqueness
    /// violation the caller must treat as fail-closed (never silently pick one).
    Ambiguous,
}

impl SeError {
    /// The underlying `OSStatus`, when there is one (`None` for [`Self::Ambiguous`]).
    #[must_use]
    pub fn os_status(self) -> Option<i32> {
        match self {
            Self::Status(code) => Some(code),
            Self::Ambiguous => None,
        }
    }
}

/// Generate a NON-EXTRACTABLE Secure-Enclave P-256 key, persisted in the keychain
/// under `application_tag`, gated by device-unlock (`WhenUnlockedThisDeviceOnly` +
/// `PrivateKeyUsage`, never synced, never in backups). Returns the live [`SecKey`]
/// (the caller derives its public key for the ECIES wrap); the private half never
/// leaves the enclave. Fails (e.g. on a device without an SE, or an unsigned macOS
/// binary) — the caller fail-closes, never silently downgrading.
///
/// The caller MUST [`delete_all`] under this tag FIRST if re-provisioning, since a
/// tag is not a keychain uniqueness key (a crashed prior provision could leave a
/// second key under the same tag → load ambiguity / lockout).
pub fn generate(application_tag: &[u8]) -> Result<SecKey, SeError> {
    // The access control carries the protection class, so kSecAttrAccessible is
    // NOT also set (mutually exclusive — this is the single source of the class).
    let access = SecAccessControl::create_with_protection(
        Some(ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly),
        PRIVATE_KEY_USAGE,
    )
    .map_err(|e| SeError::Status(e.code()))?;

    let tag = CFData::from_buffer(application_tag);
    let private_attrs = CFMutableDictionary::from_CFType_pairs(&[
        (
            unsafe { kSecAttrIsPermanent }.to_void(),
            CFBoolean::true_value().to_void(),
        ),
        (unsafe { kSecAttrApplicationTag }.to_void(), tag.to_void()),
        (unsafe { kSecAttrAccessControl }.to_void(), access.to_void()),
    ]);

    let key_size = CFNumber::from(EC_KEY_SIZE_BITS);
    let attrs = CFMutableDictionary::from_CFType_pairs(&[
        (
            unsafe { kSecAttrKeyType }.to_void(),
            unsafe { kSecAttrKeyTypeECSECPrimeRandom }.to_void(),
        ),
        (
            unsafe { kSecAttrKeySizeInBits }.to_void(),
            key_size.to_void(),
        ),
        (
            unsafe { kSecAttrTokenID }.to_void(),
            unsafe { kSecAttrTokenIDSecureEnclave }.to_void(),
        ),
        (
            unsafe { kSecPrivateKeyAttrs }.to_void(),
            private_attrs.to_void(),
        ),
    ])
    .to_immutable();

    #[allow(deprecated)] // SecKey::generate is the only public entry taking a raw dict
    SecKey::generate(attrs).map_err(|e| SeError::Status(e.code() as i32))
}

/// Find the persisted SE key under `application_tag`. `Ok(None)` if absent;
/// `Err(Ambiguous)` if more than one matches (fail-closed — never guess).
pub fn find(application_tag: &[u8]) -> Result<Option<SecKey>, SeError> {
    let mut keys = copy_matching(application_tag)?;
    match keys.len() {
        0 => Ok(None),
        1 => Ok(keys.pop()),
        _ => Err(SeError::Ambiguous),
    }
}

/// Delete EVERY SE key under `application_tag` (the FR-14 crypto-shred pivot —
/// with the SE key gone this key store can no longer open the on-disk ECIES blob;
/// an earlier copy of the key's keychain item is not proven unusable, ADR-0571).
/// Returns the COUNT actually severed — the load-bearing verify-real-sever signal
/// (`store::destroy` fails closed when a non-empty store severs zero). Idempotent:
/// no key ⇒ `Ok(0)`.
pub fn delete_all(application_tag: &[u8]) -> Result<usize, SeError> {
    let keys = copy_matching(application_tag)?;
    let mut severed = 0usize;
    for key in &keys {
        match key.delete() {
            Ok(()) => severed += 1,
            // Already gone (a concurrent delete / race): not an error, not counted.
            Err(e) if e.code() == errSecItemNotFound => {}
            Err(e) => return Err(SeError::Status(e.code())),
        }
    }
    Ok(severed)
}

/// Query every `kSecClassKey` item under `application_tag`, returning each as an
/// owned [`SecKey`] (retained out of the result array). The shared core of
/// [`find`] and [`delete_all`].
fn copy_matching(application_tag: &[u8]) -> Result<Vec<SecKey>, SeError> {
    let tag = CFData::from_buffer(application_tag);
    let query = CFMutableDictionary::from_CFType_pairs(&[
        (
            unsafe { kSecClass }.to_void(),
            unsafe { kSecClassKey }.to_void(),
        ),
        (unsafe { kSecAttrApplicationTag }.to_void(), tag.to_void()),
        (
            unsafe { kSecReturnRef }.to_void(),
            CFBoolean::true_value().to_void(),
        ),
        (
            unsafe { kSecMatchLimit }.to_void(),
            unsafe { kSecMatchLimitAll }.to_void(),
        ),
    ])
    .to_immutable();

    // Local upper-case bindings so the sys constants can be match patterns
    // (the sys names are lower-case; matching them directly trips non_upper_case_globals).
    const NOT_FOUND: i32 = errSecItemNotFound;
    const SUCCESS: i32 = errSecSuccess;

    let mut result: CFTypeRef = std::ptr::null();
    let status = unsafe { SecItemCopyMatching(query.as_concrete_TypeRef(), &mut result) };
    match status {
        NOT_FOUND => Ok(Vec::new()),
        SUCCESS => {
            if result.is_null() {
                return Ok(Vec::new());
            }
            // kSecMatchLimitAll + kSecReturnRef ⇒ a CFArray of SecKeyRef, owned by us
            // (the Copy rule). Retain each element into an owned SecKey (the get rule);
            // the array releases its own refs on drop.
            let array = unsafe { CFArray::<CFType>::wrap_under_create_rule(result as CFArrayRef) };
            let keys = array
                .get_all_values()
                .into_iter()
                .map(|v| unsafe { SecKey::wrap_under_get_rule(v as SecKeyRef) })
                .collect();
            Ok(keys)
        }
        other => Err(SeError::Status(other)),
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use security_framework::key::Algorithm;

    /// The FULL persistent SE custody KAT against a real Secure Enclave:
    /// generate → find → ECIES-wrap 32 bytes → decrypt → delete_all →
    /// **the blob no longer decrypts** through this key store. Persisting an SE key
    /// needs the `keychain-access-groups` entitlement, which is honored ONLY when
    /// the binary carries a provisioning profile: an **iOS device** build (the
    /// host app's profile grants it for free — the PRIMARY gate) or a macOS build
    /// with Xcode-managed signing + a registered App ID. A bare unsigned binary
    /// gets `-34018 errSecMissingEntitlement`; a Development-signed CLI binary
    /// WITHOUT a profile is SIGKILLed by `amfid` — so this gate does NOT run on a
    /// plain `cargo test` Mac (verified). `#[ignore]` keeps `just ci`
    /// hermetic. What IS host-provable on a dev Mac (the well-formed-dict proof):
    /// an ad-hoc run reaches `generate()` and fails with `-34018` (the entitlement
    /// gate), NOT `-50 errSecParam` — i.e. `SecKeyCreateRandomKey` ACCEPTED the
    /// CFDictionary, so the `kSecPrivateKeyAttrs` construction is correct.
    /// Test-unique pid-derived tag, deleted on the success path.
    #[test]
    #[ignore = "writes a real Secure-Enclave key — code-signed device gate, run via `just se-gate`"]
    fn se_custody_full_cycle_then_delete_is_undecryptable() {
        let tag = format!("apple-secure-enclave.test.{}", std::process::id());
        let tag = tag.as_bytes();
        // Clean slate (delete-first contract).
        let _ = delete_all(tag);

        let key = generate(tag).expect("generate SE key (needs the signed entitlement)");
        let pubkey = key.public_key().expect("public key");

        // Re-find proves the application-tag locator round-trips (the iOS-bug fix).
        let found = find(tag)
            .expect("find")
            .expect("the just-created key is locatable");

        let alg = Algorithm::ECIESEncryptionCofactorVariableIVX963SHA256AESGCM;
        let secret = [0x5Au8; 32];
        let wrapped = pubkey.encrypt_data(alg, &secret).expect("ECIES wrap");
        let unwrapped = found.decrypt_data(alg, &wrapped).expect("ECIES unwrap");
        assert_eq!(unwrapped.as_slice(), &secret[..], "round-trip");

        // Delete the SE key, then prove this key store can no longer open the blob.
        assert_eq!(
            delete_all(tag).expect("delete"),
            1,
            "exactly one key severed"
        );
        assert!(find(tag).expect("find after delete").is_none(), "key gone");
        let key2 = generate(tag).expect("a fresh key for the negative proof");
        assert!(
            key2.decrypt_data(alg, &wrapped).is_err(),
            "the old wrapped blob must NOT decrypt under any surviving/new key"
        );
        let _ = delete_all(tag);
    }
}
