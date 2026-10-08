//! §4.3a — the wrap-artifact envelope. An envelope we define, so it carries
//! the full discipline (version byte, documented nonce ownership, AAD
//! domain + binding, validate-before-use):
//!
//! ```text
//! [ ver:       1 byte  = 0x01 ]
//! [ alias_gen: 4 bytes u32 LE ]   which vault alias/generation wrapped it
//! [ iv:       12 bytes        ]   VAULT-supplied (Keystore GCM / test AEAD)
//! [ wrap(SealKey, aad) ‖ tag: 16 ]  = 48 bytes for the 32-byte SealKey
//!
//! aad = WRAP_KEY_DOMAIN ‖ SHA-256(sealed_blob)
//! ```
//!
//! - **`alias_gen` (as-implemented refinement to §4.3a, S6):** rotation is
//!   generate-NEW-alias-then-swap (one alias = one encryption, ever — GCM IV
//!   reuse structurally impossible), so the artifact must say which
//!   generation wrapped it. The header rides OUTSIDE the AAD: a tampered
//!   `alias_gen` selects a key that fails the tag — loud either way.
//! - **The AAD binding is the anti-substitution measure (crypto fold):** an
//!   attacker who swaps BOTH files for an internally-consistent pair from
//!   another install still fails — their wrap was computed over a different
//!   `H(sealed_blob)`. Carried on the WRAP side precisely so the FROZEN
//!   `zec-wallet/seal-seed/v1` AAD is never touched.
//! - **IV ownership:** the VAULT supplies the IV (AndroidKeyStore generates
//!   its own — randomized encryption is REQUIRED, never
//!   `setRandomizedEncryptionRequired(false)`); the codec only carries it.
//! - **Validate-before-use:** version byte, then EXACT length — an artifact
//!   from disk never reaches crypto malformed; a foreign/oversized blob is
//!   `WrapArtifactInvalid` before any vault call.
//! - **Fuzz-target deferral (testing-patterns):** the parser is pure
//!   slicing on length-validated input with a 256-byte cap upstream; the
//!   `artifact_parse_never_panics` proptest covers the full hostile space.
//!   A cargo-fuzz target adds nothing here — documented deferral.

use sha2::{Digest, Sha256};

use crate::error::WalletError;

/// AEAD AAD domain tag for the wrap artifact (§4.3a). FROZEN — append-only
/// versioned like `SEED_SEAL_DOMAIN`; a future envelope gets `/v2`. An AAD
/// tag, NOT an HKDF label — the frozen derivation tree is untouched.
#[cfg_attr(not(target_os = "android"), allow(dead_code))] // Android-vault-only (P3-7 fold): see the module doc
pub(crate) const WRAP_KEY_DOMAIN: &[u8] = b"zec-wallet/wrap-key/v1";

/// Wrap-artifact version byte. Append-only.
pub(crate) const WRAP_VERSION_V1: u8 = 0x01;

/// GCM/IETF-AEAD IV length. AndroidKeyStore GCM emits 12; the test vault's
/// stand-in AEAD (ChaCha20Poly1305 IETF) uses 12 — the envelope shape is
/// identical across backends, which is what lets host tests exercise the
/// REAL framing + binding logic.
#[cfg_attr(not(target_os = "android"), allow(dead_code))] // Android-vault-only (P3-7 fold): see the module doc
pub(crate) const WRAP_IV_LEN: usize = 12;

/// AEAD tag length (GCM-128 / Poly1305).
pub(crate) const WRAP_TAG_LEN: usize = 16;

/// Wrapped 32-byte SealKey + tag.
#[cfg_attr(not(target_os = "android"), allow(dead_code))] // Android-vault-only (P3-7 fold): see the module doc
pub(crate) const WRAP_CT_LEN: usize = 32 + WRAP_TAG_LEN;

#[cfg_attr(not(target_os = "android"), allow(dead_code))] // Android-vault-only (P3-7 fold): see the module doc
const GEN_LEN: usize = 4;

/// A v1 artifact is EXACTLY this long — validated before anything else
/// touches it (`1 + 4 + 12 + 48`).
#[cfg_attr(not(target_os = "android"), allow(dead_code))] // Android-vault-only (P3-7 fold): see the module doc
pub(crate) const WRAP_ARTIFACT_V1_LEN: usize = 1 + GEN_LEN + WRAP_IV_LEN + WRAP_CT_LEN;

/// Parsed v1 artifact body (header + vault inputs). No key material — the
/// ciphertext is opaque until the vault's AEAD speaks.
#[cfg_attr(not(target_os = "android"), allow(dead_code))] // Android-vault-only (P3-7 fold): see the module doc
pub(crate) struct WrapArtifactV1 {
    pub(crate) alias_gen: u32,
    pub(crate) iv: [u8; WRAP_IV_LEN],
    pub(crate) ct: [u8; WRAP_CT_LEN],
}

/// The AAD that binds a wrap to THIS install's sealed blob.
#[cfg_attr(not(target_os = "android"), allow(dead_code))] // Android-vault-only (P3-7 fold): see the module doc
pub(crate) fn wrap_aad(sealed_blob: &[u8]) -> Vec<u8> {
    let mut aad = Vec::with_capacity(WRAP_KEY_DOMAIN.len() + 32);
    aad.extend_from_slice(WRAP_KEY_DOMAIN);
    aad.extend_from_slice(&Sha256::digest(sealed_blob));
    aad
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))] // Android-vault-only (P3-7 fold): see the module doc
pub(crate) fn encode_artifact(alias_gen: u32, iv: &[u8; WRAP_IV_LEN], ct: &[u8]) -> Vec<u8> {
    debug_assert_eq!(ct.len(), WRAP_CT_LEN);
    let mut out = Vec::with_capacity(WRAP_ARTIFACT_V1_LEN);
    out.push(WRAP_VERSION_V1);
    out.extend_from_slice(&alias_gen.to_le_bytes());
    out.extend_from_slice(iv);
    out.extend_from_slice(ct);
    out
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))] // Android-vault-only (P3-7 fold): see the module doc
pub(crate) fn parse_artifact(bytes: &[u8]) -> Result<WrapArtifactV1, WalletError> {
    // Version FIRST (the downgrade story needs it distinguishable even on a
    // longer future envelope), then exact length.
    let (&ver, rest) = bytes
        .split_first()
        .ok_or(WalletError::WrapArtifactInvalid)?;
    if ver != WRAP_VERSION_V1 {
        return Err(WalletError::WrapVersionUnsupported { found: ver });
    }
    if rest.len() != WRAP_ARTIFACT_V1_LEN - 1 {
        return Err(WalletError::WrapArtifactInvalid);
    }
    let (gen_bytes, rest) = rest.split_at(GEN_LEN);
    let (iv_bytes, ct_bytes) = rest.split_at(WRAP_IV_LEN);

    let mut generation = [0u8; GEN_LEN];
    generation.copy_from_slice(gen_bytes);
    let mut iv = [0u8; WRAP_IV_LEN];
    iv.copy_from_slice(iv_bytes);
    let mut ct = [0u8; WRAP_CT_LEN];
    ct.copy_from_slice(ct_bytes);

    Ok(WrapArtifactV1 {
        alias_gen: u32::from_le_bytes(generation),
        iv,
        ct,
    })
}

// ───────────────────────── FR-14 chunk 2 — the Apple Secure-Enclave (v2) artifact ─────────────────────────
//
// A DIFFERENT shape from the v1 Android/raw artifact: the SE key is non-extractable
// hardware (located by application tag, never on disk), so the ON-DISK artifact is the
// ECIES-wrapped SealKey itself — wrapped blob on disk, wrap key in hardware (the same
// custody shape as Android, different primitive). Apple-target-gated so it stays
// dead-code-clean on Android/desktop builds under `-D warnings`.
//
// ```text
// [ ver:        1 byte = 0x02 ]
// [ blob_hash: 32 bytes        ]   SHA-256(sealed_blob) — the load-bearing blob-bind
// [ ecies:    113 bytes        ]   ECIESEncryptionCofactorVariableIVX963SHA256AESGCM(SealKey)
// ```
//
// The `blob_hash` is an application-layer anti-substitution check (ECIES exposes no AAD
// parameter): verified on load BEFORE any decrypt, it rejects a blob↔artifact swap within
// a namespace. The per-namespace SE key is the real anti-substitution (a foreign artifact
// was encrypted to a different SE public key → `decrypt_data` fails) — the hash is
// defense-in-depth, documented LOAD-BEARING so a refactor can't drop it as "redundant".
//
// Per-item Apple gating (not a sub-module) so each const/fn is referenced — directly by
// `apple.rs` or inside the codec — and stays dead-code-clean on Android/desktop.

/// Apple SE wrap-artifact version byte (FROZEN, append-only). 0x02 distinguishes the
/// SE-wrapped on-disk blob from the v1 Android/raw artifact (0x01). A rename/renumber
/// ORPHANS every SE key and SILENTLY defeats the crypto-shred (the application tag is the
/// only `purge` locator) — pinned by `apple_se_artifact_pinned`.
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) const WRAP_VERSION_APPLE_SE: u8 = 0x02;

/// `ECIESEncryptionCofactorVariableIVX963SHA256AESGCM` output length for a 32-byte secret
/// over P-256: 65 (uncompressed ephemeral pubkey `0x04‖X‖Y`) + 32 (AES-GCM ciphertext of
/// the SealKey — GCM is not length-expanding beyond the tag) + 16 (GCM tag). The VariableIV
/// is KDF-derived, NOT transmitted. MEASURED on real SE hardware — a curve property,
/// identical whether the recipient key lives in the Secure Enclave or a software keychain,
/// so the host KAT validates it too.
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) const APPLE_SE_ECIES_LEN: usize = 65 + 32 + WRAP_TAG_LEN; // 113

/// A v2 artifact is EXACTLY this long — `[ver:1][sha256(blob):32][ecies:113]` = 146.
/// Validate-before-use: reject any other length BEFORE any `decrypt_data`.
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) const WRAP_ARTIFACT_APPLE_SE_LEN: usize = 1 + 32 + APPLE_SE_ECIES_LEN; // 146

/// Parsed v2 artifact body. No key material — the ECIES blob is opaque until the SE key
/// decrypts it. `blob_hash` is the load-bearing blob-bind, verified before decrypt.
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) struct AppleSeArtifact {
    pub(crate) blob_hash: [u8; 32],
    pub(crate) ecies: [u8; APPLE_SE_ECIES_LEN],
}

// Fixed-array params make a wrong length UNREPRESENTABLE (no debug_assert that a release
// build would strip): a malformed-length ECIES blob can't reach the encoder.
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) fn encode_apple_se_artifact(
    blob_hash: &[u8; 32],
    ecies: &[u8; APPLE_SE_ECIES_LEN],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(WRAP_ARTIFACT_APPLE_SE_LEN);
    out.push(WRAP_VERSION_APPLE_SE);
    out.extend_from_slice(blob_hash);
    out.extend_from_slice(ecies);
    out
}

/// Validate-before-use: version FIRST (a forged-downgrade `0x01` is distinguishable),
/// then EXACT length, then split. A foreign/oversized blob is `WrapArtifactInvalid`
/// before any vault or crypto call.
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) fn parse_apple_se_artifact(bytes: &[u8]) -> Result<AppleSeArtifact, WalletError> {
    let (&ver, rest) = bytes
        .split_first()
        .ok_or(WalletError::WrapArtifactInvalid)?;
    if ver != WRAP_VERSION_APPLE_SE {
        return Err(WalletError::WrapVersionUnsupported { found: ver });
    }
    if rest.len() != WRAP_ARTIFACT_APPLE_SE_LEN - 1 {
        return Err(WalletError::WrapArtifactInvalid);
    }
    let (hash_bytes, ecies_bytes) = rest.split_at(32);
    let mut blob_hash = [0u8; 32];
    blob_hash.copy_from_slice(hash_bytes);
    let mut ecies = [0u8; APPLE_SE_ECIES_LEN];
    ecies.copy_from_slice(ecies_bytes);
    Ok(AppleSeArtifact { blob_hash, ecies })
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// §4.3a label pin — a one-character diff fails CI (testing-patterns:
    /// every frozen label gets a pinning test).
    #[test]
    fn wrap_key_domain_label_pinned() {
        assert_eq!(WRAP_KEY_DOMAIN, b"zec-wallet/wrap-key/v1");
        assert_eq!(WRAP_VERSION_V1, 0x01);
        assert_eq!(WRAP_ARTIFACT_V1_LEN, 65);
    }

    /// FR-14 chunk 2 frozen-format pin (testing-patterns: a 1-char/1-byte diff fails
    /// CI). The 0x02 version byte and the 146/113 lengths are append-only — a change to
    /// any of them orphans every provisioned SE key and silently defeats the crypto-shred.
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    #[test]
    fn apple_se_artifact_pinned() {
        assert_eq!(WRAP_VERSION_APPLE_SE, 0x02);
        assert_eq!(APPLE_SE_ECIES_LEN, 113);
        assert_eq!(WRAP_ARTIFACT_APPLE_SE_LEN, 146);
        // The two known version bytes are DISTINCT (no silent v1↔v2 collision).
        assert_ne!(WRAP_VERSION_V1, WRAP_VERSION_APPLE_SE);

        // Round-trip + validate-before-use on the pure codec (no SE needed).
        let blob_hash = [0x11u8; 32];
        let ecies = [0x22u8; APPLE_SE_ECIES_LEN];
        let enc = encode_apple_se_artifact(&blob_hash, &ecies);
        assert_eq!(enc.len(), WRAP_ARTIFACT_APPLE_SE_LEN);
        let parsed = parse_apple_se_artifact(&enc).expect("well-formed v2 parses");
        assert_eq!(parsed.blob_hash, blob_hash);
        assert_eq!(parsed.ecies, ecies);

        // A forged v1 downgrade is distinguished, not misparsed as v2.
        let mut v1 = enc.clone();
        v1[0] = WRAP_VERSION_V1;
        assert!(matches!(
            parse_apple_se_artifact(&v1),
            Err(WalletError::WrapVersionUnsupported { found: 0x01 })
        ));
        // Truncated / extended / empty: rejected before any crypto.
        assert!(matches!(
            parse_apple_se_artifact(&enc[..WRAP_ARTIFACT_APPLE_SE_LEN - 1]),
            Err(WalletError::WrapArtifactInvalid)
        ));
        let mut long = enc;
        long.push(0);
        assert!(matches!(
            parse_apple_se_artifact(&long),
            Err(WalletError::WrapArtifactInvalid)
        ));
        assert!(matches!(
            parse_apple_se_artifact(&[]),
            Err(WalletError::WrapArtifactInvalid)
        ));
    }

    #[test]
    fn artifact_roundtrip_and_validate_before_use() {
        let iv = [7u8; WRAP_IV_LEN];
        let ct = [9u8; WRAP_CT_LEN];
        let enc = encode_artifact(3, &iv, &ct);
        assert_eq!(enc.len(), WRAP_ARTIFACT_V1_LEN);
        let parsed = parse_artifact(&enc).expect("well-formed v1 artifact parses");
        assert_eq!(parsed.alias_gen, 3);
        assert_eq!(parsed.iv, iv);
        assert_eq!(parsed.ct, ct);

        // Unknown version: distinguished (the downgrade story).
        let mut v2 = enc.clone();
        v2[0] = 0x02;
        assert!(matches!(
            parse_artifact(&v2),
            Err(WalletError::WrapVersionUnsupported { found: 0x02 })
        ));

        // Truncated / extended: rejected before any crypto.
        assert!(matches!(
            parse_artifact(&enc[..WRAP_ARTIFACT_V1_LEN - 1]),
            Err(WalletError::WrapArtifactInvalid)
        ));
        let mut long = enc;
        long.push(0);
        assert!(matches!(
            parse_artifact(&long),
            Err(WalletError::WrapArtifactInvalid)
        ));
        assert!(matches!(
            parse_artifact(&[]),
            Err(WalletError::WrapArtifactInvalid)
        ));
    }

    #[test]
    fn wrap_aad_binds_to_blob_identity() {
        let aad_a = wrap_aad(b"blob-a");
        let aad_b = wrap_aad(b"blob-b");
        assert_ne!(aad_a, aad_b, "different blobs must produce different AADs");
        assert!(aad_a.starts_with(WRAP_KEY_DOMAIN));
        assert_eq!(aad_a.len(), WRAP_KEY_DOMAIN.len() + 32);
        // Deterministic: same blob, same AAD (the binding must be stable
        // across restarts or every reopen would fail).
        assert_eq!(aad_a, wrap_aad(b"blob-a"));
    }

    proptest! {
        /// Hostile-input discipline: random bytes never panic, never
        /// misparse as v1 unless they ARE shape-exact v1.
        #[test]
        fn artifact_parse_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..257)) {
            match parse_artifact(&bytes) {
                Ok(_) => {
                    prop_assert_eq!(bytes.len(), WRAP_ARTIFACT_V1_LEN);
                    prop_assert_eq!(bytes[0], WRAP_VERSION_V1);
                }
                Err(WalletError::WrapVersionUnsupported { found }) => {
                    prop_assert_eq!(found, bytes[0]);
                    prop_assert_ne!(found, WRAP_VERSION_V1);
                }
                Err(WalletError::WrapArtifactInvalid) => {}
                Err(other) => prop_assert!(false, "unexpected error class: {other:?}"),
            }
        }
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    proptest! {
        /// Hostile-input discipline for the v2 (Apple SE) parser — parity with the v1
        /// proptest (testing-patterns §1: every parser gets one). Random bytes never
        /// panic, never misparse as v2 unless they ARE shape-exact v2.
        #[test]
        fn apple_se_artifact_parse_never_panics(
            bytes in proptest::collection::vec(any::<u8>(), 0..257),
        ) {
            match parse_apple_se_artifact(&bytes) {
                Ok(_) => {
                    prop_assert_eq!(bytes.len(), WRAP_ARTIFACT_APPLE_SE_LEN);
                    prop_assert_eq!(bytes[0], WRAP_VERSION_APPLE_SE);
                }
                Err(WalletError::WrapVersionUnsupported { found }) => {
                    prop_assert_eq!(found, bytes[0]);
                    prop_assert_ne!(found, WRAP_VERSION_APPLE_SE);
                }
                Err(WalletError::WrapArtifactInvalid) => {}
                Err(other) => prop_assert!(false, "unexpected error class: {other:?}"),
            }
        }
    }
}
