//! §4.2a — the `SealedKeychain` seed seal. The envelope is FROZEN in the spec
//! so two implementations cannot diverge:
//!
//! ```text
//! [ ver:   1 byte  = 0x01 ]
//! [ nonce: 24 bytes, OsRng ]
//! [ XChaCha20-Poly1305( plaintext, aad = SEED_SEAL_DOMAIN ) ‖ tag: 16 ]
//!
//! plaintext = [ seed_len: u16 LE ][ seed ][ mnemonic_len: u16 LE ][ mnemonic UTF-8 ]
//! ```
//!
//! - **Nonce strategy (crypto-rules: documented-or-blocker):** the random
//!   24-byte OsRng nonce IS the misuse-resistance strategy — XChaCha's
//!   extended nonce makes random collision negligible, and there is no
//!   counter state to corrupt across the §6.3 kill windows.
//! - **`SEED_SEAL_DOMAIN` is an AEAD AAD domain tag, NOT an HKDF label** —
//!   the frozen derivation tree (identity.md §2.1) is untouched. SDK-owned
//!   and append-only versioned: a future envelope gets a NEW `/v2` constant;
//!   this one never changes (label-pin test below).
//! - **Variable, length-prefixed plaintext (HARD-A):** the blob seals the
//!   seed AND (when present) the mnemonic words — that is what makes
//!   `reveal_mnemonic` after restart possible; `RawBytes` wallets seal the
//!   seed only (mnemonic_len = 0). The BIP39 passphrase is NEVER sealed:
//!   reveal returns words only, and the at-rest posture must not silently
//!   include a secret the user believes only they hold.
//! - **Validate-before-use:** version byte + length window checked before
//!   any decrypt attempt; truncation/tamper/wrong-key collapse to ONE typed
//!   error (`SealInvalid`) — no oracle on why an unseal failed.
//!
//! The AEAD comes from RustCrypto `chacha20poly1305` WHOLE (exact-pinned).
//! The wrap key is keychain-random (OsRng), resident under the platform
//! keystore ACL — those adapters are the W3 chunk (Q10); this module is the
//! crypto core they feed.

use std::ops::RangeInclusive;

use chacha20poly1305::aead::rand_core::RngCore;
use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use zeroize::Zeroizing;

use crate::constants::{SEAL_MNEMONIC_MAX_BYTES, SEED_MAX_BYTES, SEED_MIN_BYTES};
use crate::error::WalletError;

/// AEAD AAD domain tag for the §4.2a SEED seal. FROZEN — see module docs.
pub(crate) const SEED_SEAL_DOMAIN: &[u8] = b"zec-wallet/seal-seed/v1";

/// AEAD AAD domain tag for the §4.3 DB-KEY seal. FROZEN, append-only, and
/// DISTINCT from the seed domain — so a seed blob can never authenticate as a
/// DB-key blob (or vice-versa) under the same wrap key (domain-separation test).
pub(crate) const DBKEY_SEAL_DOMAIN: &[u8] = b"zec-wallet/seal-dbkey/v1";

/// Envelope version byte. Append-only: v2 would be 0x02 with its own domain.
pub(crate) const SEAL_VERSION_V1: u8 = 0x01;

const NONCE_LEN: usize = 24;
const TAG_LEN: usize = 16;
const LEN_PREFIX: usize = 2;

// The u16 LE length prefixes are sound ONLY while these bounds hold — pinned
// at compile time so raising a constant cannot silently truncate a prefix
// (W2 review fold).
const _: () = assert!(SEED_MAX_BYTES <= u16::MAX as usize);
const _: () = assert!(SEAL_MNEMONIC_MAX_BYTES <= u16::MAX as usize);

/// v1 envelope overhead around any plaintext: `[ver:1][nonce:24] … [tag:16]`.
const ENVELOPE_OVERHEAD: usize = 1 + NONCE_LEN + TAG_LEN;

/// Inner plaintext length window for the SEED seal: length-prefixed seed, then
/// length-prefixed mnemonic (0-length when absent).
const SEED_PT_MIN: usize = LEN_PREFIX + SEED_MIN_BYTES + LEN_PREFIX;
const SEED_PT_MAX: usize = LEN_PREFIX + SEED_MAX_BYTES + LEN_PREFIX + SEAL_MNEMONIC_MAX_BYTES;

/// Fixed plaintext length for the DB-KEY seal: exactly the 256-bit key.
const DBKEY_PT_LEN: usize = 32;

/// The 32-byte wrap key. Keychain-random; no `Clone`/`Debug` (crypto-rules);
/// zeroizes on drop.
///
/// HEAP-RESIDENT (FR-47): the bytes live in one boxed zeroizing buffer filled
/// in place, so moving a `SealKey` — by value into the bounded key-store
/// worker's job and through its queue — copies a pointer, never the key, and
/// `Drop` wipes the one real location. An inline array would leave un-wiped
/// copies in every closure, box and queue slot it passed through.
pub(crate) struct SealKey(Box<Zeroizing<[u8; 32]>>);

impl SealKey {
    /// Fresh keychain-random wrap key (OsRng straight into the boxed
    /// zeroizing buffer — no unwrapped intermediate copy).
    pub(crate) fn generate() -> Self {
        let mut bytes = Box::new(Zeroizing::new([0u8; 32]));
        OsRng.fill_bytes(&mut **bytes);
        Self(bytes)
    }

    /// Exactly 32 bytes — validate, never truncate. The input is owned and
    /// zeroizes even on the reject path. A wrong-shape key from the keystore
    /// cannot open any seal, so it fails as `SealInvalid` (same class, no
    /// extra oracle).
    pub(crate) fn from_bytes(bytes: Zeroizing<Vec<u8>>) -> Result<Self, WalletError> {
        if bytes.len() != 32 {
            return Err(WalletError::SealInvalid);
        }
        let mut key = Box::new(Zeroizing::new([0u8; 32]));
        key.copy_from_slice(&bytes);
        Ok(Self(key))
    }

    fn cipher(&self) -> XChaCha20Poly1305 {
        // The cipher copies the key into its own state, which the crate
        // zeroizes on drop.
        XChaCha20Poly1305::new(Key::from_slice(self.0.as_slice()))
    }

    /// Raw key bytes for the platform vault to wrap/store (§4.3a). The ONLY
    /// sanctioned readers are `KeychainPort` backends — this is the
    /// honest-transit boundary: on Android these bytes become a JNI byte
    /// array for Keystore `Cipher` (zero-filled immediately after, §4.3a);
    /// on Apple they go into a keychain item. Never logged, never crosses
    /// the FRB bridge, never reaches a `pub` DTO (the §8 allowlist test).
    /// Compiled where a reader is: the Android and Apple vaults, and the
    /// `cfg(test)` test vault — a host with no vault (Linux) has none.
    #[cfg(any(test, target_os = "android", target_os = "macos", target_os = "ios"))]
    pub(crate) fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// What the seal protects: the seed and, when the wallet was created from a
/// mnemonic, the phrase (HARD-A). No `Clone`/`Debug`; contents zeroize on
/// drop.
pub(crate) struct SeedPayload {
    seed: Zeroizing<Vec<u8>>,
    mnemonic: Option<Zeroizing<String>>,
}

impl SeedPayload {
    /// Bounds enforced HERE (§2.2 seed window; mnemonic cap §4.2a) — a
    /// payload that can't round-trip the envelope is never constructible.
    pub(crate) fn new(
        seed: Zeroizing<Vec<u8>>,
        mnemonic: Option<Zeroizing<String>>,
    ) -> Result<Self, WalletError> {
        let len = seed.len();
        if !(SEED_MIN_BYTES..=SEED_MAX_BYTES).contains(&len) {
            return Err(WalletError::InvalidSeedLength { len });
        }
        if let Some(m) = &mnemonic
            && (m.is_empty() || m.len() > SEAL_MNEMONIC_MAX_BYTES)
        {
            return Err(WalletError::InvalidMnemonic { word_index: None });
        }
        Ok(Self { seed, mnemonic })
    }

    pub(crate) fn seed(&self) -> &[u8] {
        &self.seed
    }

    pub(crate) fn mnemonic(&self) -> Option<&str> {
        // Zeroizing<String> derefs to String, so as_deref alone yields &String
        self.mnemonic.as_deref().map(String::as_str)
    }
}

/// Assemble the v1 envelope: `[ver][nonce:24][XChaCha20-Poly1305(pt, aad=domain)‖tag]`.
/// The single place the envelope bytes are produced — the seed seal and the
/// §4.3 DB-key seal both ride it, differing ONLY in `domain` + the plaintext.
/// A fresh OsRng nonce per call (the documented misuse-resistance strategy).
fn seal_envelope(key: &SealKey, domain: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, WalletError> {
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    let ct = key
        .cipher()
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext,
                aad: domain,
            },
        )
        .map_err(|_| WalletError::SealInvalid)?;

    let mut blob = Vec::with_capacity(1 + NONCE_LEN + ct.len());
    blob.push(SEAL_VERSION_V1);
    blob.extend_from_slice(&nonce);
    blob.extend_from_slice(&ct);
    Ok(blob)
}

/// Open a v1 envelope under `domain`, returning the zeroizing plaintext.
/// Validate-before-use: an unknown version byte is the ONE distinguished case
/// (`SealVersionUnsupported`); the blob-length window is checked against the
/// EXPECTED plaintext bounds BEFORE the AEAD (a multi-megabyte "blob" never
/// reaches decrypt); tamper/truncation/wrong-key/wrong-domain all collapse to
/// `SealInvalid` — no oracle. Inner-plaintext STRUCTURE is the caller's to
/// validate (the envelope only guarantees authenticated bytes of bounded size).
fn open_envelope(
    key: &SealKey,
    domain: &[u8],
    blob: &[u8],
    pt_len: RangeInclusive<usize>,
) -> Result<Zeroizing<Vec<u8>>, WalletError> {
    if blob.is_empty() {
        return Err(WalletError::SealInvalid);
    }
    if blob[0] != SEAL_VERSION_V1 {
        return Err(WalletError::SealVersionUnsupported { found: blob[0] });
    }
    let min_blob = ENVELOPE_OVERHEAD + *pt_len.start();
    let max_blob = ENVELOPE_OVERHEAD + *pt_len.end();
    if !(min_blob..=max_blob).contains(&blob.len()) {
        return Err(WalletError::SealInvalid);
    }
    let nonce = XNonce::from_slice(&blob[1..1 + NONCE_LEN]);
    let pt = Zeroizing::new(
        key.cipher()
            .decrypt(
                nonce,
                Payload {
                    msg: &blob[1 + NONCE_LEN..],
                    aad: domain,
                },
            )
            .map_err(|_| WalletError::SealInvalid)?,
    );
    Ok(pt)
}

/// Seal `payload` under `key` → the v1 envelope blob (non-secret ciphertext;
/// stored by the keystore/file adapters).
pub(crate) fn seal(key: &SealKey, payload: &SeedPayload) -> Result<Vec<u8>, WalletError> {
    // length-prefixed plaintext, in a zeroizing buffer from allocation
    let mnemonic = payload.mnemonic().unwrap_or("");
    let mut pt = Zeroizing::new(Vec::with_capacity(
        LEN_PREFIX + payload.seed().len() + LEN_PREFIX + mnemonic.len(),
    ));
    // lengths fit u16 by SeedPayload construction (≤ 252 / ≤ 1024)
    pt.extend_from_slice(&(payload.seed().len() as u16).to_le_bytes());
    pt.extend_from_slice(payload.seed());
    pt.extend_from_slice(&(mnemonic.len() as u16).to_le_bytes());
    pt.extend_from_slice(mnemonic.as_bytes());
    seal_envelope(key, SEED_SEAL_DOMAIN, &pt)
}

/// Open a v1 envelope. Validate-before-use; every tamper/truncation/wrong-key
/// failure collapses to `SealInvalid` (no oracle); an unknown version byte is
/// the ONE distinguished case (`SealVersionUnsupported` — the honest
/// app-downgrade signal).
pub(crate) fn unseal(key: &SealKey, blob: &[u8]) -> Result<SeedPayload, WalletError> {
    let pt = open_envelope(key, SEED_SEAL_DOMAIN, blob, SEED_PT_MIN..=SEED_PT_MAX)?;

    // parse the length-prefixed plaintext; exact-consume or reject
    let read_u16 = |at: usize| -> Option<usize> {
        let b = pt.get(at..at + LEN_PREFIX)?;
        Some(u16::from_le_bytes([b[0], b[1]]) as usize)
    };
    let seed_len = read_u16(0).ok_or(WalletError::SealInvalid)?;
    if !(SEED_MIN_BYTES..=SEED_MAX_BYTES).contains(&seed_len) {
        return Err(WalletError::SealInvalid);
    }
    let seed_end = LEN_PREFIX + seed_len;
    let seed_bytes = pt
        .get(LEN_PREFIX..seed_end)
        .ok_or(WalletError::SealInvalid)?;
    let mn_len = read_u16(seed_end).ok_or(WalletError::SealInvalid)?;
    if mn_len > SEAL_MNEMONIC_MAX_BYTES {
        return Err(WalletError::SealInvalid);
    }
    let mn_start = seed_end + LEN_PREFIX;
    let mn_end = mn_start + mn_len;
    if pt.len() != mn_end {
        // trailing or missing bytes — a well-formed v1 plaintext consumes exactly
        return Err(WalletError::SealInvalid);
    }
    let mnemonic = if mn_len == 0 {
        None
    } else {
        let s = std::str::from_utf8(&pt[mn_start..mn_end]).map_err(|_| WalletError::SealInvalid)?;
        Some(Zeroizing::new(s.to_owned()))
    };

    Ok(SeedPayload {
        seed: Zeroizing::new(seed_bytes.to_vec()),
        mnemonic,
    })
}

/// The per-wallet SQLCipher DB key (§4.3): a 256-bit OsRng key, sealed at rest
/// under the keychain wrap key. **NOT seed-derived** (no frozen label owed) and
/// **distinct from the host's DB key** — device seizure (T11) must not yield the
/// funds-adjacent wallet DB. No `Clone`/`Debug`; zeroizes on drop. Reaches
/// SQLCipher ONLY via [`WalletDbKey::pragma_key_statement`]; never logged, never
/// a `pub` DTO, never across the FRB bridge (the §8 allowlist test).
//
// HEAP-RESIDENT, as `SealKey` is (FR-47): the handle's `Inner` carries this key
// (`WalletDb`), and `Inner` is MOVED by value out of its `Arc` — `Wallet::close`
// and the rescan / server-switch drain take it with `Arc::try_unwrap`, which
// copies the value out and never wipes the source. Inline, the 32 key bytes
// would stay un-wiped in the Arc's allocation (alive while any `Weak` is) and in
// every future and task cell the move passed through (the crypto audit's
// HIGH). Boxed, a move copies a pointer and `Drop` wipes the one real location.
pub(crate) struct WalletDbKey(Box<Zeroizing<[u8; 32]>>);

impl WalletDbKey {
    /// Fresh OsRng key straight into the boxed zeroizing buffer (no unwrapped
    /// intermediate copy).
    pub(crate) fn generate() -> Self {
        let mut bytes = Box::new(Zeroizing::new([0u8; 32]));
        OsRng.fill_bytes(&mut **bytes);
        Self(bytes)
    }

    /// Reconstruct from unsealed bytes — exactly 32, validate never truncate.
    fn from_zeroizing(bytes: Zeroizing<Vec<u8>>) -> Result<Self, WalletError> {
        if bytes.len() != DBKEY_PT_LEN {
            return Err(WalletError::SealInvalid);
        }
        let mut key = Box::new(Zeroizing::new([0u8; 32]));
        key.copy_from_slice(&bytes);
        Ok(Self(key))
    }

    /// The SQLCipher key PRAGMA, as a COMPLETE statement, in a zeroizing buffer
    /// (the one place the raw key is rendered to text — zeroized on drop). Uses
    /// the **raw-key** form `x'<64 hex>'`: SQLCipher takes the 256-bit key
    /// DIRECTLY (no KDF), correct because this key is already a uniform random
    /// master, not a low-entropy passphrase. MUST be the FIRST statement on a
    /// fresh connection (SQLCipher contract + rust-patterns `PRAGMA key` first).
    pub(crate) fn pragma_key_statement(&self) -> Zeroizing<String> {
        // `PRAGMA key = "x'` (16) + 64 hex + `'";` (3) = 83 bytes
        let mut s = Zeroizing::new(String::with_capacity(83));
        let reserved = s.capacity();
        s.push_str("PRAGMA key = \"x'");
        self.push_key_hex(&mut s);
        s.push_str("'\";");
        // No realloc after the key hex is written (a freed un-zeroized buffer would leak
        // it); the capacity is exact, so this only guards a future edit to the prefix.
        debug_assert_eq!(
            s.capacity(),
            reserved,
            "pragma_key_statement reallocated — key hex may have leaked to a freed buffer"
        );
        s
    }

    /// Append the 32-byte raw key as 64 lowercase hex chars to `s`. The ONE place
    /// key bytes are rendered to text — both SQLCipher statement builders
    /// ([`pragma_key_statement`](Self::pragma_key_statement) and
    /// [`attach_statement`](Self::attach_statement)) share it, so the `x'<hex>'`
    /// encoding can never silently drift between them. The caller owns a zeroizing
    /// buffer; this only appends into it.
    fn push_key_hex(&self, s: &mut String) {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        for b in self.0.iter() {
            s.push(HEX[(b >> 4) as usize] as char);
            s.push(HEX[(b & 0x0f) as usize] as char);
        }
    }

    /// The SQLCipher `ATTACH DATABASE … KEY …` statement for `path` under THIS raw
    /// key, as a COMPLETE statement in a zeroizing buffer (the ADR-0534 rescan
    /// aux-table copy). The raw-key **must** be inlined as `x'<hex>'` SQL — a key
    /// bound as a parameter is treated by SQLCipher as a passphrase (KDF), not a raw
    /// key — so this is the one place it is rendered to text for ATTACH (zeroized on
    /// drop, exactly like `pragma_key_statement`). The two ATTACHed DBs share this
    /// one key (same wallet), so the cipher settings match by construction.
    ///
    /// **READ-ONLY** (money-safety, ADR-0534 crash-atomicity): the source is the LIVE
    /// old `wallet.db`, which must be provably UNTOUCHED until the atomic rename — so it
    /// is attached through a `file:…?mode=ro` URI. Any accidental write to it would fail
    /// loudly (`SQLITE_READONLY`), never silently corrupt the file the seed-recovery path
    /// depends on. The URI form requires `SQLITE_OPEN_URI` on the connection (the caller
    /// opens it with that flag). `path` is percent-encoded for the URI-structural bytes
    /// (`%` FIRST, then `#`/`?`/space) so it can't break out of the path component, then
    /// SQL-escaped (doubled single-quotes) for the surrounding string literal; `alias` is
    /// a FIXED internal schema name (never user input).
    ///
    /// SEAM NOTE — DESKTOP PORTABILITY (Windows, a §9 v2 target; mirrors `store::sync_dir`):
    /// `Path::to_str()` on Windows yields `C:\Users\…\wallet.db`; SQLite normalizes `\`→`/`
    /// in `file:` URIs in practice, but the strict RFC-3986 form is `file:///C:/…` (three
    /// slashes + forward slashes). A `cfg(windows)` branch converting separators + prefixing
    /// the drive letter is the desktop-adapter fix — named here so its author isn't
    /// surprised; no functional change is owed before Windows desktop ships (mobile dirs are
    /// single-leading-slash POSIX paths, the only platforms we ship today).
    pub(crate) fn attach_statement(&self, path: &str, alias: &str) -> Zeroizing<String> {
        // Percent-encode the URI-structural bytes — `%` MUST be first (it is the encoding
        // escape), then the path/query delimiters + space (RFC-3986 completeness; SQLite's
        // parser is lenient on POSIX but this is the one declared place for URI safety). The
        // path is NOT secret (it is `db_dir`), so a plain String is fine; only the key bytes
        // need zeroizing. (A `db_dir` beginning with `//` would render a `file://authority/…`
        // form, but the shipped platforms never produce one and SQLite fails it CLOSED —
        // `SQLITE_CANTOPEN`, never a writable/wrong open — so it is inert, not exploitable.)
        let uri_path = path
            .replace('%', "%25")
            .replace('#', "%23")
            .replace('?', "%3f")
            .replace(' ', "%20");
        // SQLite single-quoted string literal: escape an embedded quote as `''`.
        let literal = format!("file:{uri_path}?mode=ro").replace('\'', "''");
        // Pre-size EXACTLY so the buffer never reallocs after the key hex is written: a
        // realloc would memcpy then free the old buffer — holding the full key hex —
        // WITHOUT zeroizing it (Zeroizing scrubs only the final allocation on drop). Fixed
        // bytes = 17 (`ATTACH DATABASE '`) + 5 (`' AS `) + 8 (` KEY "x'`) + 64 (key hex) +
        // 3 (`'";`) = 97, plus the literal + alias.
        let mut s = Zeroizing::new(String::with_capacity(literal.len() + alias.len() + 97));
        let reserved = s.capacity();
        s.push_str("ATTACH DATABASE '");
        s.push_str(&literal);
        s.push_str("' AS ");
        s.push_str(alias);
        s.push_str(" KEY \"x'");
        self.push_key_hex(&mut s);
        s.push_str("'\";");
        debug_assert_eq!(
            s.capacity(),
            reserved,
            "attach_statement reallocated — key hex may have leaked to a freed buffer"
        );
        s
    }
}

/// Seal the wallet DB key under the keychain wrap key → the v1 envelope blob
/// (non-secret ciphertext; stored beside the wallet DB). §4.3 — same envelope
/// as the seed seal, distinct domain.
pub(crate) fn seal_db_key(key: &SealKey, db_key: &WalletDbKey) -> Result<Vec<u8>, WalletError> {
    seal_envelope(key, DBKEY_SEAL_DOMAIN, &db_key.0[..])
}

/// Open a §4.3 DB-key seal. Same validate-before-use + collapsed-error posture
/// as the seed seal: tamper/truncation/wrong-key/wrong-domain → `SealInvalid`
/// (no oracle); an unknown version byte → `SealVersionUnsupported`. The fixed
/// 32-byte window means a well-formed blob is exactly `ENVELOPE_OVERHEAD + 32`.
pub(crate) fn unseal_db_key(key: &SealKey, blob: &[u8]) -> Result<WalletDbKey, WalletError> {
    let pt = open_envelope(key, DBKEY_SEAL_DOMAIN, blob, DBKEY_PT_LEN..=DBKEY_PT_LEN)?;
    WalletDbKey::from_zeroizing(pt)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Vectors FIRST (crypto-rules) ────────────────────────────────────────

    #[test]
    fn kat_xchacha20poly1305_ietf_draft_vector() {
        // draft-irtf-cfrg-xchacha-03 §A.3 — proves the pinned crate, our
        // feature set, and the XChaCha construction produce the canonical
        // bytes (the seal is glue over exactly this primitive).
        let key = hex::decode("808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f")
            .expect("key hex");
        let nonce = hex::decode("404142434445464748494a4b4c4d4e4f5051525354555657").expect("iv");
        let aad = hex::decode("50515253c0c1c2c3c4c5c6c7").expect("aad hex");
        let pt = hex::decode(concat!(
            "4c616469657320616e642047656e746c656d656e206f662074686520636c6173",
            "73206f66202739393a204966204920636f756c64206f6666657220796f75206f",
            "6e6c79206f6e652074697020666f7220746865206675747572652c2073756e73",
            "637265656e20776f756c642062652069742e"
        ))
        .expect("pt hex");
        let want_ct = hex::decode(concat!(
            "bd6d179d3e83d43b9576579493c0e939572a1700252bfaccbed2902c21396cbb",
            "731c7f1b0b4aa6440bf3a82f4eda7e39ae64c6708c54c216cb96b72e1213b452",
            "2f8c9ba40db5d945b11b69b982c1bb9e3f3fac2bc369488f76b2383565d3fff9",
            "21f9664c97637da9768812f615c68b13b52e",
            // ‖ tag
            "c0875924c1c7987947deafd8780acf49"
        ))
        .expect("ct hex");

        let cipher = XChaCha20Poly1305::new(Key::from_slice(&key));
        let got = cipher
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: &pt,
                    aad: &aad,
                },
            )
            .expect("vector encrypt");
        assert_eq!(got, want_ct, "XChaCha20-Poly1305 KAT mismatch");
    }

    #[test]
    fn seed_seal_domain_label_pinned() {
        // Label-pin discipline (testing-patterns): a 1-char diff fails CI.
        assert_eq!(SEED_SEAL_DOMAIN, b"zec-wallet/seal-seed/v1");
        assert_eq!(SEAL_VERSION_V1, 0x01);
    }

    // ── The named test (§8) ─────────────────────────────────────────────────

    fn payload_with_mnemonic() -> SeedPayload {
        SeedPayload::new(
            Zeroizing::new(vec![0x42; 64]),
            Some(Zeroizing::new(
                "abandon abandon abandon abandon abandon abandon abandon abandon \
                 abandon abandon abandon abandon abandon abandon abandon abandon \
                 abandon abandon abandon abandon abandon abandon abandon art"
                    .to_owned(),
            )),
        )
        .expect("valid payload")
    }

    #[test]
    fn wallet_seed_seal_roundtrip_and_tamper_rejected() {
        let key = SealKey::generate();

        // roundtrip: seed-only (the RawBytes shape)
        let seed_only =
            SeedPayload::new(Zeroizing::new(vec![0x07; 32]), None).expect("valid payload");
        let blob = seal(&key, &seed_only).expect("seal");
        assert_eq!(blob[0], SEAL_VERSION_V1);
        let back = unseal(&key, &blob).expect("unseal");
        assert_eq!(back.seed(), &[0x07; 32][..]);
        assert!(back.mnemonic().is_none());

        // roundtrip: seed + mnemonic (HARD-A — the reveal-after-restart shape)
        let full = payload_with_mnemonic();
        let blob = seal(&key, &full).expect("seal");
        let back = unseal(&key, &blob).expect("unseal");
        assert_eq!(back.seed(), full.seed());
        assert_eq!(back.mnemonic(), full.mnemonic());

        // wrong key → the ONE collapsed error
        let wrong = SealKey::generate();
        assert!(matches!(
            unseal(&wrong, &blob),
            Err(WalletError::SealInvalid)
        ));

        // version byte flip → the distinguished downgrade signal
        let mut v = blob.clone();
        v[0] = 0x02;
        assert!(matches!(
            unseal(&key, &v),
            Err(WalletError::SealVersionUnsupported { found: 0x02 })
        ));

        // per-component tamper: nonce, ciphertext body, tag — each rejected
        for idx in [
            1,
            1 + NONCE_LEN / 2,
            1 + NONCE_LEN,
            blob.len() - TAG_LEN,
            blob.len() - 1,
        ] {
            let mut t = blob.clone();
            t[idx] ^= 0x01;
            assert!(
                matches!(unseal(&key, &t), Err(WalletError::SealInvalid)),
                "tamper at byte {idx} must be rejected"
            );
        }

        // truncation / extension
        assert!(matches!(
            unseal(&key, &blob[..blob.len() - 1]),
            Err(WalletError::SealInvalid)
        ));
        assert!(matches!(
            unseal(&key, &blob[..10]),
            Err(WalletError::SealInvalid)
        ));
        assert!(matches!(unseal(&key, b""), Err(WalletError::SealInvalid)));
        let mut ext = blob.clone();
        ext.push(0x00);
        assert!(matches!(unseal(&key, &ext), Err(WalletError::SealInvalid)));
    }

    #[test]
    fn seal_nonce_is_fresh_per_seal() {
        // Two seals of the SAME payload under the SAME key must differ in the
        // nonce region (the documented misuse-resistance strategy is real,
        // not a fixed buffer) — and both must open.
        let key = SealKey::generate();
        let payload = payload_with_mnemonic();
        let a = seal(&key, &payload).expect("seal a");
        let b = seal(&key, &payload).expect("seal b");
        assert_ne!(
            a[1..1 + NONCE_LEN],
            b[1..1 + NONCE_LEN],
            "nonce must be fresh per seal"
        );
        assert!(unseal(&key, &a).is_ok());
        assert!(unseal(&key, &b).is_ok());
    }

    #[test]
    fn seal_aad_domain_is_bound() {
        // A well-formed v1 blob whose ciphertext was produced under a
        // DIFFERENT aad must not open: proves the domain tag is actually
        // authenticated, not decorative.
        let key = SealKey::generate();
        let pt = {
            let mut v = Vec::new();
            v.extend_from_slice(&32u16.to_le_bytes());
            v.extend_from_slice(&[0x07; 32]);
            v.extend_from_slice(&0u16.to_le_bytes());
            v
        };
        let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ct = key
            .cipher()
            .encrypt(
                &nonce,
                Payload {
                    msg: &pt,
                    aad: b"zec-wallet/seal-seed/v2",
                },
            )
            .expect("encrypt");
        let mut blob = vec![SEAL_VERSION_V1];
        blob.extend_from_slice(&nonce);
        blob.extend_from_slice(&ct);
        assert!(matches!(unseal(&key, &blob), Err(WalletError::SealInvalid)));
    }

    #[test]
    fn seal_inner_plaintext_is_validated_not_trusted() {
        // Authenticated-but-malformed plaintexts (a buggy/hostile writer with
        // the right key and domain) are still rejected: seed-length window,
        // length-prefix overruns, trailing bytes, non-UTF-8 mnemonic.
        let key = SealKey::generate();
        let seal_raw = |pt: &[u8]| {
            let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
            let ct = key
                .cipher()
                .encrypt(
                    &nonce,
                    Payload {
                        msg: pt,
                        aad: SEED_SEAL_DOMAIN,
                    },
                )
                .expect("encrypt");
            let mut blob = vec![SEAL_VERSION_V1];
            blob.extend_from_slice(&nonce);
            blob.extend_from_slice(&ct);
            blob
        };

        // seed_len below the window (4)
        let mut pt = Vec::new();
        pt.extend_from_slice(&4u16.to_le_bytes());
        pt.extend_from_slice(&[0xAA; 4]);
        pt.extend_from_slice(&0u16.to_le_bytes());
        // pad so the BLOB length window passes and the parse itself must reject
        pt.extend_from_slice(&[0u8; 40]);
        assert!(matches!(
            unseal(&key, &seal_raw(&pt)),
            Err(WalletError::SealInvalid)
        ));

        // seed_len overruns the actual plaintext
        let mut pt = Vec::new();
        pt.extend_from_slice(&200u16.to_le_bytes());
        pt.extend_from_slice(&[0xAA; 40]);
        assert!(matches!(
            unseal(&key, &seal_raw(&pt)),
            Err(WalletError::SealInvalid)
        ));

        // trailing garbage after a valid mnemonic region
        let mut pt = Vec::new();
        pt.extend_from_slice(&32u16.to_le_bytes());
        pt.extend_from_slice(&[0xAA; 32]);
        pt.extend_from_slice(&1u16.to_le_bytes());
        pt.push(b'x');
        pt.push(0xFF); // trailing
        assert!(matches!(
            unseal(&key, &seal_raw(&pt)),
            Err(WalletError::SealInvalid)
        ));

        // non-UTF-8 mnemonic bytes
        let mut pt = Vec::new();
        pt.extend_from_slice(&32u16.to_le_bytes());
        pt.extend_from_slice(&[0xAA; 32]);
        pt.extend_from_slice(&2u16.to_le_bytes());
        pt.extend_from_slice(&[0xFF, 0xFE]);
        assert!(matches!(
            unseal(&key, &seal_raw(&pt)),
            Err(WalletError::SealInvalid)
        ));
    }

    #[test]
    fn seal_bounds_make_oversize_unconstructible() {
        // The 1MB+-payload row of the crypto test matrix, inverted: oversize
        // can never ENTER this encrypt path — the caps are the contract.
        assert!(matches!(
            SeedPayload::new(Zeroizing::new(vec![0u8; 31]), None),
            Err(WalletError::InvalidSeedLength { len: 31 })
        ));
        assert!(matches!(
            SeedPayload::new(Zeroizing::new(vec![0u8; 253]), None),
            Err(WalletError::InvalidSeedLength { len: 253 })
        ));
        assert!(matches!(
            SeedPayload::new(
                Zeroizing::new(vec![0u8; 32]),
                Some(Zeroizing::new("x".repeat(SEAL_MNEMONIC_MAX_BYTES + 1)))
            ),
            Err(WalletError::InvalidMnemonic { word_index: None })
        ));
        assert!(matches!(
            SeedPayload::new(
                Zeroizing::new(vec![0u8; 32]),
                Some(Zeroizing::new(String::new()))
            ),
            Err(WalletError::InvalidMnemonic { word_index: None })
        ));

        // and the LARGEST constructible payload round-trips
        let key = SealKey::generate();
        let max = SeedPayload::new(
            Zeroizing::new(vec![0x5A; SEED_MAX_BYTES]),
            Some(Zeroizing::new("y".repeat(SEAL_MNEMONIC_MAX_BYTES))),
        )
        .expect("max payload valid");
        let blob = seal(&key, &max).expect("seal max");
        assert_eq!(blob.len(), ENVELOPE_OVERHEAD + SEED_PT_MAX);
        let back = unseal(&key, &blob).expect("unseal max");
        assert_eq!(back.seed().len(), SEED_MAX_BYTES);
        assert_eq!(back.mnemonic().map(str::len), Some(SEAL_MNEMONIC_MAX_BYTES));
    }

    #[test]
    fn seal_key_from_bytes_validates_never_truncates() {
        assert!(SealKey::from_bytes(Zeroizing::new(vec![1u8; 32])).is_ok());
        for len in [0usize, 16, 31, 33, 64] {
            assert!(matches!(
                SealKey::from_bytes(Zeroizing::new(vec![1u8; len])),
                Err(WalletError::SealInvalid)
            ));
        }
    }

    // ── §4.3 DB-key seal ─────────────────────────────────────────────────────

    #[test]
    fn dbkey_seal_domain_label_pinned() {
        // Label-pin discipline (testing-patterns): a 1-char diff fails CI.
        assert_eq!(DBKEY_SEAL_DOMAIN, b"zec-wallet/seal-dbkey/v1");
        // and it MUST stay distinct from the seed domain — the separation guard
        assert_ne!(DBKEY_SEAL_DOMAIN, SEED_SEAL_DOMAIN);
    }

    #[test]
    fn wallet_db_key_sealed_never_plaintext_on_disk() {
        // §8 named test (the SEAL half — the DB key never sits in plaintext in
        // the stored blob; the at-rest DB-FILE encryption half is in db.rs).
        let wrap = SealKey::generate();
        let db_key = WalletDbKey::generate();
        let raw: [u8; 32] = **db_key.0; // copy for the not-present assertion

        let blob = seal_db_key(&wrap, &db_key).expect("seal db key");
        assert_eq!(blob[0], SEAL_VERSION_V1);
        // fixed-size envelope: [ver:1][nonce:24][ct:32][tag:16] = 73
        assert_eq!(blob.len(), ENVELOPE_OVERHEAD + DBKEY_PT_LEN);

        // the raw key bytes appear NOWHERE in the sealed blob
        assert!(
            blob.windows(DBKEY_PT_LEN).all(|w| w != &raw[..]),
            "raw DB key must not appear in the sealed blob"
        );

        // roundtrip recovers the exact key
        let back = unseal_db_key(&wrap, &blob).expect("unseal db key");
        assert_eq!(**back.0, raw, "db key roundtrip");

        // wrong wrap key → the ONE collapsed error (no oracle)
        let wrong = SealKey::generate();
        assert!(matches!(
            unseal_db_key(&wrong, &blob),
            Err(WalletError::SealInvalid)
        ));

        // version byte flip → distinguished downgrade signal
        let mut v = blob.clone();
        v[0] = 0x02;
        assert!(matches!(
            unseal_db_key(&wrap, &v),
            Err(WalletError::SealVersionUnsupported { found: 0x02 })
        ));

        // per-component tamper: nonce, ciphertext body, tag — each rejected
        for idx in [1, 1 + NONCE_LEN, blob.len() - 1] {
            let mut t = blob.clone();
            t[idx] ^= 0x01;
            assert!(
                matches!(unseal_db_key(&wrap, &t), Err(WalletError::SealInvalid)),
                "tamper at byte {idx} must be rejected"
            );
        }

        // truncation / extension
        assert!(matches!(
            unseal_db_key(&wrap, &blob[..blob.len() - 1]),
            Err(WalletError::SealInvalid)
        ));
        let mut ext = blob.clone();
        ext.push(0x00);
        assert!(matches!(
            unseal_db_key(&wrap, &ext),
            Err(WalletError::SealInvalid)
        ));

        // DOMAIN SEPARATION (size-window arm): a real SEED blob (77+ bytes) is
        // rejected by the dbkey size window (expects exactly 73) BEFORE the AEAD.
        let seed_blob = seal(
            &wrap,
            &SeedPayload::new(Zeroizing::new(vec![0x09; 32]), None).expect("payload"),
        )
        .expect("seal seed");
        assert!(matches!(
            unseal_db_key(&wrap, &seed_blob),
            Err(WalletError::SealInvalid)
        ));

        // DOMAIN SEPARATION (AEAD arm): a blob of EXACTLY 32 plaintext bytes
        // sealed under the SEED domain is 73 bytes — it PASSES the dbkey size
        // window, so rejection here proves the AAD domain tag is authenticated,
        // not decorative (mirrors `seal_aad_domain_is_bound`). Without this, the
        // size arm above would let a same-size wrong-domain blob masquerade.
        let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ct = wrap
            .cipher()
            .encrypt(
                &nonce,
                Payload {
                    msg: &[0x5A; DBKEY_PT_LEN],
                    aad: SEED_SEAL_DOMAIN, // WRONG domain for a dbkey blob
                },
            )
            .expect("encrypt");
        let mut wrong_domain = vec![SEAL_VERSION_V1];
        wrong_domain.extend_from_slice(&nonce);
        wrong_domain.extend_from_slice(&ct);
        assert_eq!(wrong_domain.len(), ENVELOPE_OVERHEAD + DBKEY_PT_LEN); // 73 — passes the size window
        assert!(matches!(
            unseal_db_key(&wrap, &wrong_domain),
            Err(WalletError::SealInvalid)
        ));
    }

    #[test]
    fn dbkey_pragma_statement_is_raw_key_hex_and_zeroizing() {
        // The PRAGMA renders the 256-bit key as the raw-key form `x'<64 hex>'`
        // (lowercase), as a complete leading statement. Shape pinned so a future
        // edit can't silently switch to a passphrase form (which would KDF the
        // key and break at-rest interop with an already-provisioned DB).
        let db_key = WalletDbKey::generate();
        let stmt = db_key.pragma_key_statement();
        assert!(stmt.starts_with("PRAGMA key = \"x'"));
        assert!(stmt.ends_with("'\";"));
        let hex = &stmt["PRAGMA key = \"x'".len()..stmt.len() - "'\";".len()];
        assert_eq!(hex.len(), 64, "32 bytes → 64 hex chars");
        assert!(
            hex.bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "lowercase hex only"
        );
    }

    proptest::proptest! {
        // testing-patterns §2: a parser must NEVER panic on arbitrary bytes.
        // Both unseal paths parse externally-stored (validate-before-use) blobs;
        // for ANY input they must return Ok/Err, never panic / index-OOB.
        #[test]
        fn unseal_never_panics_on_arbitrary_bytes(
            blob in proptest::collection::vec(proptest::arbitrary::any::<u8>(), 0..2048),
        ) {
            let key = SealKey::generate();
            let _ = unseal(&key, &blob);
            let _ = unseal_db_key(&key, &blob);
        }
    }

    // Crypto-rules: no Clone/Debug/Display on key-material types — pinned at
    // compile time.
    static_assertions::assert_not_impl_any!(SealKey: Clone, std::fmt::Debug, std::fmt::Display);
    static_assertions::assert_not_impl_any!(
        SeedPayload: Clone, std::fmt::Debug, std::fmt::Display
    );
    static_assertions::assert_not_impl_any!(
        WalletDbKey: Clone, std::fmt::Debug, std::fmt::Display
    );

    #[test]
    fn key_types_are_heap_resident_so_a_move_copies_a_pointer_not_the_key() {
        // FR-47 / both keys ride structures that are MOVED by value (a key-store
        // job; `Inner` taken out of its Arc on close, rescan and server switch). A move
        // of an inline `[u8; 32]` leaves an un-wiped copy wherever it passed; a boxed key
        // moves as one pointer. A key type wider than a pointer has gone back inline.
        let pointer = size_of::<usize>();
        assert_eq!(
            size_of::<WalletDbKey>(),
            pointer,
            "WalletDbKey must be one heap pointer — inline key bytes are copied, un-wiped, by every move of Inner",
        );
        assert_eq!(
            size_of::<SealKey>(),
            pointer,
            "SealKey must be one heap pointer — inline key bytes are copied, un-wiped, through the key-store queue",
        );
    }
}
