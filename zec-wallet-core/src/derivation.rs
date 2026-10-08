//! Key derivation (spec §2.2/§3.1/§4.2). Two audited derivations, taken whole
//! (Rule Zero): `bip39` maps a mnemonic ↔ seed, and `zcash_keys`/`zip32`
//! derive the ZIP-32 unified spending key and its default unified address.
//! This module is the boundary that keeps secrets in zeroizing buffers, maps
//! failures typed (never a panic across FFI), and pins both derivations to
//! golden vectors (§8) so a librustzcash/bip39 bump that moves derivation
//! fails in CI, never on a user's funds.
//!
//! Documented residuals (for the crypto audit register, HARD-C class —
//! scope-pinned, function-local, never stored, never across an `.await`):
//! - `bip39::Mnemonic` wipes its word indices on drop (the pinned crate's
//!   `zeroize` feature, on since S7 C3). The phrase is written into one
//!   pre-sized `Zeroizing` buffer ([`phrase_of`]), never through the growing
//!   `to_string()`. What bip39 and its dependencies still leave un-wiped:
//!   the stack arrays inside `parse_in_normalized` (the word indices, bits
//!   and entropy it builds while validating the checksum), PBKDF2's `ipad`/`opad` and
//!   HMAC state inside `to_seed_normalized`, and that function's by-value
//!   `[u8; 64]` return — [`seed_of`] moves it into `Zeroizing` at once, which
//!   is best effort (the compiler may have copied it on the way).
//! - `reveal_mnemonic` hands the phrase to the host as a plain `Vec<String>`
//!   across the bridge — the one sanctioned crossing (the user asked to see
//!   it); those strings are the host's to drop.
//! - The passphrase's NFKD form (stage S2 `recovery`, ADR-0561): the crate's
//!   `normalize_utf8_cow` builds it with `nfkd().to_string()`, a `String`
//!   that GROWS by reallocation, so each outgrown buffer — a partial copy of
//!   the normalized passphrase — is freed un-zeroized inside the crate; the
//!   final buffer lands in `Zeroizing` (`nfkd_passphrase`). Only a
//!   passphrase that is NOT already NFKD takes that path. A pre-sized
//!   buffer would close it but means normalizing outside the crate's own
//!   door — a hardening item, not taken at S2.
//! - `UnifiedSpendingKey` implements **no Zeroize/ZeroizeOnDrop** and is
//!   `Clone + Debug` upstream (§4.2(a)); the instance below is strictly
//!   function-local — never logged, never stored, never `Clone`-reached from
//!   any DTO, dropped at the end of derivation. The un-zeroized residue in
//!   freed memory is the documented residual + an upstream-issue candidate.

// Module layout — two derivation groups at the same layer (pure, sync, no I/O):
//   (1) resolve_seed / parse_mnemonic — BIP39 mnemonic ↔ seed.
//   (2) derive_default_address / default_address_for — ZIP-32 USK → default UA.
// Split into a dedicated `keys.rs` if either group grows past its current pair.

use std::borrow::Cow;

use bip39::{Language, Mnemonic};
use chacha20poly1305::aead::OsRng;
use chacha20poly1305::aead::rand_core::RngCore;
use zcash_address::{ToAddress, ZcashAddress};
use zcash_keys::keys::{
    ReceiverRequirement, UnifiedAddressRequest, UnifiedFullViewingKey, UnifiedSpendingKey,
};
use zcash_protocol::consensus::{MAIN_NETWORK, Parameters, TEST_NETWORK};
use zcash_transparent::address::TransparentAddress;
use zcash_transparent::keys::{AccountPrivKey, IncomingViewingKey, NonHardenedChildIndex};
use zeroize::Zeroizing;
use zip32::AccountId;

use crate::constants::{SEED_MAX_BYTES, SEED_MIN_BYTES};
use crate::error::WalletError;
use crate::money::Network;
use crate::seal::SeedPayload;
use crate::seed::SeedSource;

/// v1 scope cut (§1.6): one account (ZIP-32 account 0) per wallet instance.
const ACCOUNT_ZERO: AccountId = AccountId::ZERO;

/// Resolve a [`SeedSource`] into the seed (+ mnemonic when one exists) that
/// the wallet derives from and — in `SealedKeychain` mode — seals (§4.2a
/// HARD-A). Every secret rides a zeroizing buffer; inputs are consumed.
//
// Consumed by the wallet handle (`Wallet::create_with_vault`); also pinned by
// the §8 vector tests (`kat_bip39_trezor_vector_through_resolve` etc).
pub(crate) fn resolve_seed(source: SeedSource) -> Result<SeedPayload, WalletError> {
    match source {
        SeedSource::Generate => {
            // 256-bit OsRng entropy → 24 words; BIP39 seed with empty
            // passphrase (the third-party create flow — a passphrase implies
            // restore semantics the user must own).
            let mut entropy = Zeroizing::new([0u8; 32]);
            OsRng.fill_bytes(&mut *entropy);
            let mnemonic = Mnemonic::from_entropy_in(Language::English, entropy.as_ref())
                .map_err(|_| WalletError::InvalidMnemonic { word_index: None })?;
            // the empty passphrase is already NFKD; routed through the same
            // door as the restore arm so the two BIP39 paths cannot diverge
            let passphrase = nfkd_passphrase("");
            let seed = seed_of(&mnemonic, &passphrase);
            let phrase = phrase_of(&mnemonic);
            SeedPayload::new(seed, Some(phrase))
        }
        SeedSource::Mnemonic { phrase, passphrase } => {
            let mnemonic = parse_mnemonic(&phrase)?;
            // BIP39 PBKDF2s the NFKD form of the passphrase: a composed "é"
            // and "e" + combining acute are one credential and one wallet.
            let passphrase = nfkd_passphrase(&passphrase);
            let seed = seed_of(&mnemonic, &passphrase);
            // seal the VALIDATED, normalized phrase (what reveal returns);
            // the BIP39 passphrase is never sealed (§4.2a module docs)
            let normalized = phrase_of(&mnemonic);
            SeedPayload::new(seed, Some(normalized))
        }
        // The fresh marker changes CREATE-time behavior (stamp/eager/posture,
        // §3.2f), never the derivation: both raw-bytes shapes resolve to the
        // same mnemonic-less payload, so a fresh-marked and a plain create of
        // the same bytes derive the same accounts.
        SeedSource::RawBytes(bytes) | SeedSource::FreshRawBytes(bytes) => {
            SeedPayload::new(bytes, None)
        }
    }
}

/// The BIP39 passphrase in NFKD, through the pinned crate's own normalizer
/// (`Mnemonic::normalize_utf8_cow`, taken whole). The one owned copy — the
/// normalizer's `Cow::Owned` when the input was not already NFKD, else the
/// borrowed input copied out — lands in a `Zeroizing` buffer; the crate's
/// normalizing `to_seed` would drop that copy un-zeroized.
fn nfkd_passphrase(passphrase: &str) -> Zeroizing<String> {
    let mut cow = Cow::Borrowed(passphrase);
    Mnemonic::normalize_utf8_cow(&mut cow);
    Zeroizing::new(cow.into_owned())
}

/// The longest BIP39 English phrase in bytes: 24 words of at most 8 letters, 23
/// separating spaces. [`phrase_of`] allocates this once so the phrase never grows.
const PHRASE_MAX_BYTES: usize = 24 * 8 + 23;

/// The space-separated phrase, written word by word into ONE pre-sized `Zeroizing`
/// buffer (S7 C3). `Mnemonic::to_string()` grows its `String` by reallocation and frees
/// each outgrown prefix of the phrase un-wiped; this buffer never reallocates, so the
/// only copy is the one that is wiped on drop. Byte-equal to `to_string()` (the BIP39
/// vector tests pin it).
fn phrase_of(mnemonic: &Mnemonic) -> Zeroizing<String> {
    let mut phrase = Zeroizing::new(String::with_capacity(PHRASE_MAX_BYTES));
    for (i, word) in mnemonic.words().enumerate() {
        if i > 0 {
            phrase.push(' ');
        }
        phrase.push_str(word);
    }
    phrase
}

/// The BIP39 seed for an NFKD passphrase (S7 C3). The crate returns the 64 bytes by
/// value; they are moved into `Zeroizing` at once and the heap copy is taken from
/// there, so the named array is wiped (the by-value return itself is best effort — see
/// the module's residual register).
fn seed_of(mnemonic: &Mnemonic, nfkd_passphrase: &str) -> Zeroizing<Vec<u8>> {
    let seed = Zeroizing::new(mnemonic.to_seed_normalized(nfkd_passphrase));
    Zeroizing::new(seed.to_vec())
}

/// BIP39 checksum/word validation. Errors carry the word INDEX only — never
/// the word value (identity-core precedent; §6.1).
fn parse_mnemonic(phrase: &str) -> Result<Mnemonic, WalletError> {
    Mnemonic::parse_in_normalized(Language::English, phrase).map_err(|e| {
        let word_index = match e {
            bip39::Error::UnknownWord(i) => Some(i as u32),
            _ => None,
        };
        WalletError::InvalidMnemonic { word_index }
    })
}

/// Derive account 0's default unified address (spec §3.1). Receiver set
/// (review G7): Orchard **and** Sapling, **NO transparent receiver** — both
/// shielded receivers are present so any shielded sender (Orchard-capable or
/// Sapling-only) can pay this address; transparent is omitted. Swap refunds
/// use fresh external-scope transparent t-addrs ([`derive_transparent_refund_address`],
/// §2.6 HARD-H), never this UA. See [`default_address_for`]
/// for why `Require` (not `Allow`) is load-bearing.
///
/// The seed MUST already be bounds-validated (`SeedSource`/`SeedPayload`
/// enforce 32..=252 — the §4.2(b) upstream-panic guard; `from_seed` PANICS
/// under 32 bytes, and a panic across FFI is a crash, not an error). The bound
/// is RE-enforced here as defense-in-depth so the function is panic-safe for
/// any future caller, not only today's validated paths.
//
// Consumed by the wallet handle (`Wallet::current_address`, W3-inc-2c-i);
// also pinned by the §8 vector tests (`kat_fixed_seed_derives_pinned_unified_address` etc).
pub(crate) fn derive_default_address(network: Network, seed: &[u8]) -> Result<String, WalletError> {
    // Defense-in-depth (security review W3-inc-2): callers feed bounds-validated
    // SeedSource/SeedPayload (32..=252), but `from_seed` PANICS on < 32 bytes —
    // re-enforce HERE so a panic across FFI is structurally impossible regardless
    // of caller discipline.
    if !(SEED_MIN_BYTES..=SEED_MAX_BYTES).contains(&seed.len()) {
        return Err(WalletError::InvalidSeedLength { len: seed.len() });
    }
    match network {
        Network::Main => default_address_for(&MAIN_NETWORK, seed),
        Network::Test => default_address_for(&TEST_NETWORK, seed),
    }
}

fn default_address_for<P: Parameters>(params: &P, seed: &[u8]) -> Result<String, WalletError> {
    // Audited derivation, taken whole (Rule Zero). The USK is function-local
    // (§4.2(a) residual): never logged, never stored, never escapes — it is
    // consumed into the UFVK and dropped here. Upstream errors are mapped
    // typed (a panic must never be the error path, §4.2).
    let usk = UnifiedSpendingKey::from_seed(params, seed, ACCOUNT_ZERO)
        .map_err(|_| WalletError::KeyDerivation)?;
    let ufvk = usk.to_unified_full_viewing_key();
    encode_default_address(params, &ufvk)
    // usk + ufvk drop here.
}

/// Encode a UFVK in the standard unified encoding — the §3.7 D1 EXPORT
/// artifact (`uview1…` main / `uviewtest1…` test). Audited codec taken WHOLE
/// (Rule Zero): the zcash_keys ZIP-316 container is typecoded, F4Jumbled and
/// HRP-network-bound, so the string is self-describing and cross-network
/// decode fails structurally — NO custom envelope of ours on top (a second
/// parser would be a §4.6 finding). The string is TOTAL-HISTORY-VISIBILITY
/// material (§3.7 threat framing): callers are the ONE sanctioned egress
/// (`Wallet::export_ufvk`) and tests — never a log, error, or DTO (§5.4).
//
// Consumed by `Wallet::export_ufvk` (#397); pinned by
// `kat_fixed_seed_derives_pinned_ufvk_encoding` (§3.7 D6, vectors-first).
pub(crate) fn encode_ufvk(network: Network, ufvk: &UnifiedFullViewingKey) -> String {
    match network {
        Network::Main => ufvk.encode(&MAIN_NETWORK),
        Network::Test => ufvk.encode(&TEST_NETWORK),
    }
}

/// Decode a host-supplied UFVK string for `network` — the §3.7 D2 IMPORT
/// door (`Wallet::create_watch_only`). Audited codec whole: the ZIP-316
/// container is typecoded + F4Jumbled + HRP-network-bound, so garbage,
/// tampering/truncation AND a wrong-network `uview…` all fail HERE,
/// structurally, before any store side effect — typed
/// [`WalletError::InvalidViewingKey`], NEVER a panic (§4.6) and NEVER an
/// echo of the offending string (upstream's error string is dropped — §5.4).
//
// Consumed by `Wallet::create_watch_only` (#397); the KAT above pins the
// decode arms (round-trip, cross-net reject, hostile-input floor).
pub(crate) fn decode_ufvk(
    network: Network,
    encoded: &str,
) -> Result<UnifiedFullViewingKey, WalletError> {
    // SIZE-CAP-BEFORE-ALLOC (§4.6; review security M3): the upstream parser
    // collects ∝ input length BEFORE its own length reject, and the bridge
    // forwards an unbounded Dart String — cap HERE so every caller inherits
    // the bound with zero allocation on the reject path.
    if encoded.len() > crate::constants::UFVK_MAX_BYTES {
        return Err(WalletError::InvalidViewingKey);
    }
    let decoded = match network {
        Network::Main => UnifiedFullViewingKey::decode(&MAIN_NETWORK, encoded),
        Network::Test => UnifiedFullViewingKey::decode(&TEST_NETWORK, encoded),
    };
    decoded.map_err(|_| {
        // §3.7 D2 (review M2): distinguish "valid key, WRONG network" from
        // garbage — a probe-decode under the OPPOSITE params (still upstream's
        // total parser, bounded input) succeeding proves a well-formed
        // cross-network artifact, which maps to the SAME `NetworkMismatch`
        // arm the seed restore uses (so the D5 restore-arm UI renders "wrong
        // network" and "not a viewing key" as the distinct errors the spec
        // promises). Neither error ever echoes the string (§5.4).
        let other = match network {
            Network::Main => UnifiedFullViewingKey::decode(&TEST_NETWORK, encoded),
            Network::Test => UnifiedFullViewingKey::decode(&MAIN_NETWORK, encoded),
        };
        if other.is_ok() {
            WalletError::NetworkMismatch
        } else {
            WalletError::InvalidViewingKey
        }
    })
}

/// Derive account 0's transient [`UnifiedSpendingKey`] from the seed (spec §4.2,
/// the create+sign path, inc-2d-2). The audited `from_seed` derivation, taken whole
/// (Rule Zero) — the SAME ACCOUNT_ZERO index `derive_default_address` /
/// `ensure_account` use, so the returned USK's UFVK is the wallet account's own.
///
/// **§4.2 CONFINEMENT IS THE CALLER'S OBLIGATION.** `UnifiedSpendingKey` has NO
/// `Zeroize` and is `Clone + Debug` upstream (the documented §4.2(a) residual), so
/// the returned key MUST live ONLY inside the `spawn_blocking` proving closure —
/// never held across an `.await`, never stored in any struct/field, never placed in
/// a `Debug`-reachable container, never logged. The intended use is: derive here,
/// move into `SpendingKeys::from_unified_spending_key`, sign, drop — all within one
/// synchronous blocking section (see `send::create_signed_core`).
///
/// The seed is bounds-validated (`from_seed` PANICS under 32 bytes — §4.2(b); a panic
/// across FFI is a crash, not an error), re-enforced HERE as defense-in-depth exactly
/// like [`derive_default_address`] so the function is panic-safe for any caller.
//
// Consumed by the create+sign path (`send::create_signed_core` via the wallet handle,
// inc-2d-2); pinned by `derive_spending_key_*` (§8) + the address-equality KAT below.
pub(crate) fn derive_spending_key(
    network: Network,
    seed: &[u8],
) -> Result<UnifiedSpendingKey, WalletError> {
    if !(SEED_MIN_BYTES..=SEED_MAX_BYTES).contains(&seed.len()) {
        return Err(WalletError::InvalidSeedLength { len: seed.len() });
    }
    let usk = match network {
        Network::Main => UnifiedSpendingKey::from_seed(&MAIN_NETWORK, seed, ACCOUNT_ZERO),
        Network::Test => UnifiedSpendingKey::from_seed(&TEST_NETWORK, seed, ACCOUNT_ZERO),
    }
    .map_err(|_| WalletError::KeyDerivation)?;
    Ok(usk)
}

/// Derive a SINGLE-USE transparent refund address at the BIP44 external path
/// `m/44'/<coin_type>'/0'/0/<index>` (spec §2.6 HARD-H, W-swap-3-a). A failed
/// `OutOfZec` swap deposits ZEC from the shielded pool, so the provider has no
/// observable on-chain source to refund to — the wallet MUST hand it a fresh
/// transparent address it controls. "Fresh, never recycled" (HARD-H) is a privacy
/// property: reusing a refund t-addr across swaps would publicly LINK them. The
/// monotonic `index` comes from the durable never-recycle allocator
/// ([`crate::refund_index`]); this function is the pure derivation half.
///
/// **Audited library, taken WHOLE (Rule Zero).** `zcash_transparent::keys` owns the
/// BIP32/secp256k1 math behind its `transparent-inputs` feature (enabled ONLY on this
/// leaf key crate — the engine's `zcash_client_backend`/`_sqlite` keep it OFF, so the
/// shielded-only UA (G7) + pool-crossing-off posture is unchanged; see Cargo.toml).
/// We touch only the EXTERNAL incoming viewing key (a public viewing key) to derive a
/// receive address — never the spend key beyond what `from_seed` transiently needs.
///
/// **HD-recoverable.** The external scope (BIP44 `…/0/i`) is the standard receive chain
/// any BIP44 Zcash wallet scans on restore, so a refund landing after a wipe is
/// re-derivable from the seed + index (§4.5 wipe note) — never stranded.
///
/// `AccountPrivKey` wraps a secp256k1 extended private key; like the §4.2(a) USK it is
/// strictly function-local — never stored, never logged, never `Clone`-reached, dropped
/// here. The seed is bounds-validated (re-enforced as defense-in-depth, exactly like
/// [`derive_default_address`], so the path is panic-safe for any caller). The address
/// is a plain `String` — no key material leaves this function.
///
/// **BIP32 master-seed constraint (honest degradation).** Transparent derivation runs
/// through BIP32 `ExtendedPrivateKey::new`, which accepts ONLY a 16/32/64-byte master
/// seed — narrower than the shielded ZIP-32 path (32..=252). Every production wallet seed
/// is BIP39 (64 bytes) or a 32-byte raw seed, both of which derive; an exotic in-range
/// length (e.g. 48) that the shielded path accepts yields a typed `KeyDerivation` here,
/// never a panic and never a wrong address (so such a wallet simply cannot serve an
/// `OutOfZec` refund — surfaced honestly, never silently mis-derived).
//
// Consumed by the swap `WalletRefundSource` (W-swap-3-b, over the durable index);
// pinned by `kat_fixed_seed_derives_pinned_refund_address` + the §8 row.
#[cfg_attr(not(feature = "swap"), allow(dead_code))]
pub(crate) fn derive_transparent_refund_address(
    network: Network,
    seed: &[u8],
    index: u32,
) -> Result<String, WalletError> {
    if !(SEED_MIN_BYTES..=SEED_MAX_BYTES).contains(&seed.len()) {
        return Err(WalletError::InvalidSeedLength { len: seed.len() });
    }
    match network {
        Network::Main => transparent_refund_address_for(&MAIN_NETWORK, seed, index),
        Network::Test => transparent_refund_address_for(&TEST_NETWORK, seed, index),
    }
}

/// True iff a seed of `seed_len` bytes can derive a transparent refund address
/// at ALL (W-swap-3-b). Transparent derivation runs through BIP32
/// `ExtendedPrivateKey::new`, which accepts ONLY a 16/32/64-byte master seed
/// (narrower than the shielded path's 32..=252 — see
/// [`derive_transparent_refund_address`]). Since the wallet's own `SeedSource`
/// floor is 32 bytes, the only lengths that ACTUALLY reach derivation and
/// succeed are 32 (raw) and 64 (BIP39); an exotic in-range length (48/100/252)
/// is BIP32-unusable. The [`WalletRefundSource`](crate::wallet) checks this BEFORE
/// the allocation loop so an exotic-seed wallet fails FAST (honest "cannot serve
/// OutOfZec refunds") instead of burning the never-recycle index space on a
/// derivation that can never succeed — the per-index miss the loop retries is a
/// DIFFERENT, ~2^-127 condition. This is the ONE source of truth for the
/// predicate (DRY): the §8 test `refund_address_requires_bip32_master_seed_length`
/// pins it against the ACTUAL `derive_transparent_refund_address` outcome (every
/// 32/64 derives, every exotic length yields `KeyDerivation`) so the predicate can
/// never drift from the real derivation.
//
// NAME PRECISION: this answers "is this a BIP32 master-seed length", NOT "will THIS
// wallet derive at this length". 16 is BIP32-valid (so this returns `true`), but it
// is below the wallet's `SEED_MIN_BYTES` (32), so `derive_transparent_refund_address`
// rejects a 16-byte seed as `InvalidSeedLength` BEFORE reaching BIP32 — and a 16-byte
// seed never occurs in the first place (the `SeedSource` floor is 32). 16 is included
// only so the predicate states the true BIP32 rule, not the wallet-narrowed one; the
// refund source only ever sees 32/64 (which derive) or an exotic length (which fails
// the predicate ⇒ fail-fast). A `bip32` crate bump that widened/narrowed the accepted
// set would silently drift this literal — the §8 anti-drift test is the sole guard
// (the crate does not expose the predicate to share a constant with).
//
// Consumed by the swap `WalletRefundSource` (gated) + the §8 test (always on); the
// only build where it is unused is a swap-OFF non-test compile.
#[cfg_attr(not(feature = "swap"), allow(dead_code))]
pub(crate) fn supports_transparent_refund(seed_len: usize) -> bool {
    matches!(seed_len, 16 | 32 | 64)
}

#[cfg_attr(not(feature = "swap"), allow(dead_code))]
fn transparent_refund_address_for<P: Parameters>(
    params: &P,
    seed: &[u8],
    index: u32,
) -> Result<String, WalletError> {
    // A ZIP-32 non-hardened child index is < 2^31; the allocator caps at this bound, but
    // re-enforce here (a hardened bit set ⇒ `None`) so derivation is total for any caller.
    let child = NonHardenedChildIndex::from_index(index).ok_or(WalletError::KeyDerivation)?;
    // Audited derivation, taken whole. The AccountPrivKey is function-local (§4.2(a)
    // residual class): consumed into the account pubkey and dropped here. Upstream errors
    // map typed — a panic must never be the error path.
    let apk = AccountPrivKey::from_seed(params, seed, ACCOUNT_ZERO)
        .map_err(|_| WalletError::KeyDerivation)?;
    let external_ivk = apk
        .to_account_pubkey()
        .derive_external_ivk()
        .map_err(|_| WalletError::KeyDerivation)?;
    // The (cryptographically vanishing) per-index derivation failure is typed, not a
    // panic; W-swap-3-b advances the durable index on the rare miss rather than recycling.
    let taddr = external_ivk
        .derive_address(child)
        .map_err(|_| WalletError::KeyDerivation)?;
    encode_transparent(params, &taddr)
    // apk + external_ivk drop here.
}

/// Encode a derived [`TransparentAddress`] to its network-correct string via the audited
/// `zcash_address` encoder (a DIFFERENT crate from the one that derived the bytes, so a
/// KAT decoding the result cross-checks both). `derive_address`/`from_pubkey` only ever
/// yields P2PKH; a `ScriptHash` cannot arise from key derivation, so it is a typed reject
/// (a refund address we could not spend from would be a bug), never silently encoded.
fn encode_transparent<P: Parameters>(
    params: &P,
    addr: &TransparentAddress,
) -> Result<String, WalletError> {
    match addr {
        TransparentAddress::PublicKeyHash(hash) => {
            Ok(ZcashAddress::from_transparent_p2pkh(params.network_type(), *hash).encode())
        }
        TransparentAddress::ScriptHash(_) => Err(WalletError::KeyDerivation),
    }
}

/// The wallet's default UA from a UFVK — the SINGLE source of truth for the
/// default-address shape, shared by the seed-derivation path ([`default_address_for`])
/// AND the stored-account path (account.rs reads the DB account UFVK and calls
/// [`default_address_from_ufvk`]). Because BOTH paths run this identical
/// `Require`/`Require`/`Omit` request + encode over the SAME UFVK (one derived
/// from the seed, one read from the DB account that was derived from that same
/// seed), the two addresses are byte-equal by construction — the property the
/// handle-UA == account-UA equality test (§8) locks. Re-validates the G7 receiver
/// set as a checked post-condition (below).
fn encode_default_address<P: Parameters>(
    params: &P,
    ufvk: &UnifiedFullViewingKey,
) -> Result<String, WalletError> {
    let (ua, _index) = g7_default_address(ufvk)?;
    Ok(ua.encode(params))
}

/// The default UA and the diversifier index it sits at (`d0`, the first index whose
/// Sapling diversifier is valid) — the ONE derivation behind [`encode_default_address`]
/// and [`default_address_index`], so the index the swap mint skips is the index of the
/// address the wallet publishes, by construction.
fn g7_default_address(
    ufvk: &UnifiedFullViewingKey,
) -> Result<(zcash_keys::address::UnifiedAddress, zip32::DiversifierIndex), WalletError> {
    let request = g7_receive_request()?;
    let (ua, index) = ufvk
        .default_address(request)
        .map_err(|_| WalletError::KeyDerivation)?;

    // Enforced post-condition (crypto audit W3-inc-2 HARDENING): the G7 receiver
    // set is CHECKED, never assumed from the request — a future UFVK reconstructed
    // from an encoding that dropped a subtree would otherwise ship a single-receiver
    // UA silently. A violation is a typed failure, never a wrong address on funds.
    if !ua_satisfies_g7(&ua) {
        return Err(WalletError::KeyDerivation);
    }
    Ok((ua, index))
}

/// The diversifier index of the default UA (S7 C2). A swap destination minted at this
/// index would carry the default address's Orchard and Sapling receivers, linking the
/// swap to the published address (ADR-0537's rule), so the two allocation paths treat
/// it as a burned index. Public data (an index, not a key); §5.4 still holds —
/// never logged.
#[cfg_attr(not(feature = "swap"), allow(dead_code))]
pub(crate) fn default_address_index(
    ufvk: &UnifiedFullViewingKey,
) -> Result<zip32::DiversifierIndex, WalletError> {
    g7_default_address(ufvk).map(|(_ua, index)| index)
}

/// The G7 receiver set (spec §3.1) — the ONE builder for every PUBLIC receive UA:
/// the default address AND the FR-8 diversified mint (Recv-4, ADR-0537) share it, so
/// the two surfaces can never drift on compatibility posture. `Require` (not
/// `Allow`) is load-bearing (crypto audit W3-inc-2): with `Allow`, a sapling-invalid
/// diversifier yields an orchard-ONLY address SILENTLY — a sapling-only sender then
/// cannot pay it. On the default path `Require` makes `find_address` search forward;
/// on the fixed-index mint path it makes the engine return the honest per-index miss
/// (`Ok(None)` — the allocator burns the index and advances). `custom` only errors
/// if both shielded receivers are omitted (not our case) — mapped typed regardless.
pub(crate) fn g7_receive_request() -> Result<UnifiedAddressRequest, WalletError> {
    UnifiedAddressRequest::custom(
        ReceiverRequirement::Require, // orchard
        ReceiverRequirement::Require, // sapling
        ReceiverRequirement::Omit,    // transparent (p2pkh)
    )
    .map_err(|_| WalletError::KeyDerivation)
}

/// The G7 receiver-set CHECK (the W3-inc-2 "checked, never assumed" post-condition),
/// shared by [`encode_default_address`] and the FR-8 diversified mint
/// (`account::mint_diversified_receive`): orchard + sapling present, transparent
/// absent. A violation at either call site is a typed failure, never a wrong or
/// degraded address handed out on funds.
pub(crate) fn ua_satisfies_g7(ua: &zcash_keys::address::UnifiedAddress) -> bool {
    ua.orchard().is_some() && ua.sapling().is_some() && ua.transparent().is_none()
}

/// Encode the default UA for a UFVK read from the stored account (account.rs,
/// W3-inc-2c-iii). Network-dispatching wrapper over [`encode_default_address`]
/// (the SAME G7 request/encode the seed path uses → the DB-sourced UA equals the
/// seed-derived one for the same seed). The seed never enters this path — it
/// derives PURELY from the account's public viewing key, which is what lets
/// `current_address` work in `None`-persistence mode (no seed at rest).
pub(crate) fn default_address_from_ufvk(
    network: Network,
    ufvk: &UnifiedFullViewingKey,
) -> Result<String, WalletError> {
    match network {
        Network::Main => encode_default_address(&MAIN_NETWORK, ufvk),
        Network::Test => encode_default_address(&TEST_NETWORK, ufvk),
    }
}

/// Encode the wallet's transparent RECEIVE address (Recv-2, ADR-0528) from a UFVK's
/// transparent component: the account's canonical external-scope (BIP44 `m/44'/coin'/0'/0/0`)
/// P2PKH receiver. The SINGLE source of truth, shared by the §8 seed KAT (which derives the
/// UFVK from a fixed seed) AND the stored-account path (`account::account_transparent_address`
/// reads the DB account's UFVK and calls this). Because BOTH derive
/// `external-ivk → default_address → encode` over the SAME UFVK (one from the seed, one from
/// the DB account derived from that seed), the two addresses are byte-equal by construction.
///
/// CONTRAST with the shielded [`encode_default_address`]: that one **Omits** the transparent
/// receiver from the UA (G7); THIS exposes the transparent receiver directly as a bare
/// t-address, by explicit user opt-in (the receive-screen toggle, default shielded — §3.3a).
/// The address is PUBLIC (the spend key never leaves Rust, §4.1); §5.4 NEVER-LOG still holds —
/// it is rendered/copied/encoded, never logged.
///
/// **Collision-free from refunds (ADR-0528).** This is external index 0; the swap-refund
/// allocator ([`crate::refund_index`]) reserves index 0 for THIS address and hands out refund
/// t-addrs from index 1 — so a refund never lands on the user's main receive address (a HARD-H
/// privacy link). `ufvk.transparent()` is `None` only for a UFVK serialized before
/// `transparent-inputs` (a pre-ADR-0528 wallet — re-provision to gain it): typed
/// `KeyDerivation`, never a panic, never a wrong address.
/// The wallet's transparent receive RECEIVER (the typed [`TransparentAddress`]) — external
/// index 0 from the account's transparent public viewing key (ADR-0528). The SINGLE derivation
/// the displayed receive address ([`encode_transparent_receive_address`]) AND the Recv-2b UTXO
/// detection ([`crate::transparent`]) both build on, so the address the UI shows, the address
/// queried via `GetAddressUtxos`, and the address each returned UTXO is matched against are ONE
/// derivation — never a decode round-trip that could drift.
fn transparent_receiver_from_ufvk(
    ufvk: &UnifiedFullViewingKey,
) -> Result<TransparentAddress, WalletError> {
    // The transparent AccountPubKey (BIP44 `m/44'/coin'/0'`), present on every UFVK derived
    // since ADR-0528 enabled `transparent-inputs`. Audited derivation, taken WHOLE (Rule Zero).
    let account_pubkey = ufvk.transparent().ok_or(WalletError::KeyDerivation)?;
    let external_ivk = account_pubkey
        .derive_external_ivk()
        .map_err(|_| WalletError::KeyDerivation)?;
    // The engine-canonical default transparent receiver: `default_address` searches for the
    // first VALID external-scope index (index 0 for every real key — pinned in the §8 KAT).
    let (taddr, index) = external_ivk.default_address();
    // The collision-free reservation assumes the receive address is external
    // `refund_index::RECEIVE_EXTERNAL_INDEX` (0), the one index the refund counter floors above.
    // `default_address` returns that index for every real key; the only exception is the ~2^-127
    // case where it fails to derive and the search advances to the next index — which is
    // `REFUND_INDEX_FLOOR`, so it would silently ALIAS the first refund address. Assert the
    // reserved index (typed reject otherwise) so that impossible case can never collide on funds
    // (crypto audit + security review HARDENING fold; re-review DRY fold links it to the SSOT
    // const) — the reservation is enforced, not assumed.
    if index.index() != crate::refund_index::RECEIVE_EXTERNAL_INDEX {
        return Err(WalletError::KeyDerivation);
    }
    Ok(taddr)
    // account_pubkey + external_ivk drop here (public viewing keys; no spend material touched).
}

fn encode_transparent_receive_address<P: Parameters>(
    params: &P,
    ufvk: &UnifiedFullViewingKey,
) -> Result<String, WalletError> {
    encode_transparent(params, &transparent_receiver_from_ufvk(ufvk)?)
}

/// The wallet's transparent receive address from a UFVK read from the stored account
/// (Recv-2, ADR-0528). Network-dispatching wrapper over [`encode_transparent_receive_address`].
/// The seed never enters this path — it derives PURELY from the account's transparent public
/// viewing key, so `current_transparent_address` works in `None`-persistence (no seed at rest),
/// exactly like the shielded [`default_address_from_ufvk`].
//
// Consumed by `account::account_transparent_address` (the `current_transparent_address` core
// method); pinned by `current_transparent_address_is_kat_pinned` + `…_is_hd_recoverable` (§8).
pub(crate) fn transparent_receive_address_from_ufvk(
    network: Network,
    ufvk: &UnifiedFullViewingKey,
) -> Result<String, WalletError> {
    match network {
        Network::Main => encode_transparent_receive_address(&MAIN_NETWORK, ufvk),
        Network::Test => encode_transparent_receive_address(&TEST_NETWORK, ufvk),
    }
}

/// Typed sibling of [`transparent_receive_address_from_ufvk`] — the receive RECEIVER (not the
/// encoded string), for the Recv-2b UTXO-detection match ([`crate::transparent::validate`]).
/// Network-independent (the address bytes are the same; only the encoding differs), so it takes
/// no params; consumed by `account::account_transparent_receiver`.
pub(crate) fn transparent_receiver_from_account_ufvk(
    ufvk: &UnifiedFullViewingKey,
) -> Result<TransparentAddress, WalletError> {
    transparent_receiver_from_ufvk(ufvk)
}

/// Encode a transparent RECEIVER as its canonical t-address string for `network` — the
/// `GetAddressUtxos` query form. The SSOT encoder, network-dispatched, so the queried address
/// byte-matches the displayed receive address (same derivation, same encoder).
pub(crate) fn encode_transparent_receiver(
    network: Network,
    addr: &TransparentAddress,
) -> Result<String, WalletError> {
    match network {
        Network::Main => encode_transparent(&MAIN_NETWORK, addr),
        Network::Test => encode_transparent(&TEST_NETWORK, addr),
    }
}

/// The receiver set for a fresh per-swap IntoZec DESTINATION address (§3.3b D1 /
/// ADR-0530 — the `request` handed to the engine's `get_address_for_index`).
/// UNLIKE the shielded receive UA ([`encode_default_address`], G7 — which OMITS
/// transparent), the destination MUST carry the TRANSPARENT receiver: 1Click pays
/// the transparent leg in practice (Zodl evidence), and the §3.3b D2 scoped
/// active-swap poll detects the delivery at that t-receiver before the Recv-3
/// shield. ORCHARD is `Require` so a shielded-paying provider just works via the
/// normal scan (universality — no 2→1 refactor). SAPLING is `Allow`: included when
/// the diversifier is valid at the chosen index, never blocking derivation —
/// `get_address_for_index` derives at a FIXED index and cannot search forward, and
/// ~half of sapling diversifiers are invalid, so `Require` sapling would churn the
/// counter. Orchard has NO invalid diversifiers and a transparent receiver exists
/// at every non-hardened index, so this request CONFORMS at the first reserved
/// index for every real key — no gap burn. `custom` errors only if BOTH shielded
/// are omitted (not our case); mapped typed regardless.
///
/// RECOVERY CAVEAT (the crate's `get_address_for_index` WARNING, ADR-0530 L4): a
/// `Require transparent` receiver at an external index beyond the wallet's gap window
/// is NOT auto-discovered by a FRESH seed-restore scan (the engine row is device-local).
/// Accepted + documented: the transparent leg is BRIEF — the Recv-3 shield (IZ-1b/ADR-0529)
/// moves the delivery to the seed-recoverable shielded pool promptly; the §5.1 disclosure
/// states the loss window. Not widened here.
#[cfg(feature = "swap")]
pub(crate) fn swap_destination_address_request() -> Result<UnifiedAddressRequest, WalletError> {
    UnifiedAddressRequest::custom(
        ReceiverRequirement::Require, // orchard — universality (a shielded-paying provider works)
        ReceiverRequirement::Allow,   // sapling — opportunistic, never blocks a fixed index
        ReceiverRequirement::Require, // transparent — 1Click pays this; the scoped poll detects it
    )
    .map_err(|_| WalletError::KeyDerivation)
}

/// Encode an engine-derived [`UnifiedAddress`](zcash_keys::address::UnifiedAddress)
/// (from `get_address_for_index` — the IntoZec destination §3.3b D1, or the FR-8
/// diversified receive mint, Recv-4) as its canonical UA string for `network`. The
/// address is PUBLIC (handed to 1Click / rendered on the receive screen); the spend
/// key never leaves Rust (§4.1), and §5.4 NEVER-LOG still holds — it is
/// encoded/sent/rendered, never logged. Un-gated since Recv-4: the diversified mint
/// exists in every build, swap or not.
pub(crate) fn encode_unified_address(
    network: Network,
    ua: &zcash_keys::address::UnifiedAddress,
) -> String {
    match network {
        Network::Main => ua.encode(&MAIN_NETWORK),
        Network::Test => ua.encode(&TEST_NETWORK),
    }
}

/// Decode a stored watched-address string back to its TRANSPARENT receiver — the §3.3b D2
/// scoped poll matches returned UTXOs against this. Two self-minted forms reach it (#368):
/// an IntoZec destination / backfill UA (from our own [`encode_unified_address`] — the
/// address we minted + handed 1Click) and an OutOfZec refund's bare P2PKH t-addr (from
/// [`derive_transparent_refund_address`] — the refundTo we handed the provider,
/// cross-checked at mint against the engine UA's receiver). Decoding either gives the
/// receiver byte-identical to the address the provider was told to pay — no re-derivation
/// drift. A decode failure, a UA with no transparent receiver, or any OTHER address kind
/// (P2SH, bare sapling, TEX) is only reachable via a tampered/corrupt sealed row (we never
/// store those forms) ⇒ fail-closed `StoreCorrupt`, never a panic. Network-dispatched (the
/// HRP/prefix carries the network).
#[cfg(feature = "swap")]
pub(crate) fn transparent_receiver_from_stored_address(
    network: Network,
    addr_str: &str,
) -> Result<TransparentAddress, WalletError> {
    let decoded = match network {
        Network::Main => zcash_keys::address::Address::decode(&MAIN_NETWORK, addr_str),
        Network::Test => zcash_keys::address::Address::decode(&TEST_NETWORK, addr_str),
    };
    match decoded {
        Some(zcash_keys::address::Address::Unified(ua)) => {
            ua.transparent().copied().ok_or(WalletError::StoreCorrupt)
        }
        // The #368 refund form: a bare transparent P2PKH. A ScriptHash cannot come from our
        // own derivation ([`encode_transparent`] rejects it at mint) — tamper-only, fail closed.
        Some(zcash_keys::address::Address::Transparent(
            t @ TransparentAddress::PublicKeyHash(_),
        )) => Ok(t),
        _ => Err(WalletError::StoreCorrupt),
    }
}

/// Derive the transparent receive address straight from the SEED (Recv-2, ADR-0528) — the
/// pre-account-import fallback for `current_transparent_address`, mirroring
/// [`derive_default_address`]. Derives the (function-local, §4.2(a)) USK, takes its UFVK, and
/// runs the SAME shared encoder as the stored-account path, so the seed-derived and
/// DB-account-derived transparent addresses are byte-equal by construction (the §8 KAT locks
/// it). The seed never leaves Rust (borrow-only); only the PUBLIC address string is returned.
///
/// The seed is bounds-validated (the §4.2(b) panic guard) exactly like the shielded path; an
/// in-range-but-non-BIP32 length yields a typed `KeyDerivation` (ADR-0528's unified seed
/// contract — `from_seed` now also derives the transparent key, which BIP32 limits to 16/32/64
/// byte seeds), never a panic.
pub(crate) fn derive_transparent_receive_address(
    network: Network,
    seed: &[u8],
) -> Result<String, WalletError> {
    if !(SEED_MIN_BYTES..=SEED_MAX_BYTES).contains(&seed.len()) {
        return Err(WalletError::InvalidSeedLength { len: seed.len() });
    }
    match network {
        Network::Main => transparent_receive_for(&MAIN_NETWORK, seed),
        Network::Test => transparent_receive_for(&TEST_NETWORK, seed),
    }
}

fn transparent_receive_for<P: Parameters>(params: &P, seed: &[u8]) -> Result<String, WalletError> {
    // Audited derivation, taken whole (Rule Zero). The USK is function-local (§4.2(a) residual):
    // consumed into the UFVK and dropped here — never logged, never stored, never escapes.
    let usk = UnifiedSpendingKey::from_seed(params, seed, ACCOUNT_ZERO)
        .map_err(|_| WalletError::KeyDerivation)?;
    encode_transparent_receive_address(params, &usk.to_unified_full_viewing_key())
    // usk drops here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Vectors FIRST (crypto-rules) ────────────────────────────────────────

    #[test]
    fn kat_bip39_trezor_vector_through_resolve() {
        // BIP39 reference vector (Trezor #1, 24-word all-"abandon" class):
        // entropy 0x00×32 → the canonical phrase; seed with passphrase
        // "TREZOR". Pins OUR resolve() wiring over the audited crate.
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon \
                      abandon abandon abandon abandon abandon abandon abandon abandon \
                      abandon abandon abandon abandon abandon abandon abandon art";
        let source = SeedSource::mnemonic(phrase.to_owned(), Some("TREZOR".to_owned()));
        let payload = resolve_seed(source).expect("vector mnemonic resolves");
        assert_eq!(
            hex::encode(payload.seed()),
            "bda85446c68413707090a52022edd26a1c9462295029f2e60cd7c4f2bbd30971\
             70af7a4d73245cafa9c3cca8d561a7c3de6f5d4a10be8ed2a5e608d68f92fcc8",
            "BIP39 vector seed mismatch — the resolve() wiring moved"
        );
        // the phrase has runs of spaces above (source formatting) — what we
        // seal is the NORMALIZED single-spaced phrase
        let normalized = phrase.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(payload.mnemonic(), Some(normalized.as_str()));
    }

    // ── Behavior tests ──────────────────────────────────────────────────────

    #[test]
    fn seed_source_zeroized_after_create() {
        // §8 named test. Byte-level zeroization is delegated to the audited
        // `zeroize` crate (every secret buffer in resolve_seed is Zeroizing
        // from allocation; inputs are CONSUMED — no caller copy survives),
        // and the secret types are compile-time pinned non-Clone/non-Debug
        // (static asserts in seed.rs/seal.rs), so no copy path exists.
        let bytes = vec![0xAB; 32];
        let source = SeedSource::raw_bytes(bytes).expect("valid");
        let payload = resolve_seed(source).expect("resolves");
        assert_eq!(payload.seed(), &[0xAB; 32][..]);
        assert!(
            payload.mnemonic().is_none(),
            "RawBytes wallets have no mnemonic (§3.1)"
        );
    }

    #[test]
    fn fresh_and_plain_raw_bytes_resolve_identically() {
        // FR-24: the fresh marker changes CREATE-time behavior only —
        // the derivation is byte-identical, so a fresh-marked and a plain
        // create of the same host bytes own the same accounts/funds. A
        // divergence here would strand a wallet's funds behind the flag.
        let fresh = resolve_seed(SeedSource::raw_bytes_fresh(vec![0xCD; 32]).expect("valid"))
            .expect("resolves");
        let plain =
            resolve_seed(SeedSource::raw_bytes(vec![0xCD; 32]).expect("valid")).expect("resolves");
        assert_eq!(fresh.seed(), plain.seed());
        assert!(
            fresh.mnemonic().is_none(),
            "fresh raw bytes keep the no-mnemonic contract (§3.1)"
        );
    }

    #[test]
    fn generate_yields_24_words_and_64_byte_seed() {
        let payload = resolve_seed(SeedSource::generate()).expect("generates");
        assert_eq!(payload.seed().len(), 64, "BIP39 PBKDF2 seed");
        let words = payload
            .mnemonic()
            .expect("generated wallets keep their phrase");
        assert_eq!(words.split_whitespace().count(), 24);
        // two generates never collide (OsRng entropy)
        let second = resolve_seed(SeedSource::generate()).expect("generates");
        assert_ne!(payload.seed(), second.seed());
    }

    #[test]
    fn invalid_mnemonic_carries_word_index_only() {
        // wrong word at index 1 in a valid-COUNT phrase (bip39 checks count
        // first) — the error names the INDEX, never the word
        let source = SeedSource::mnemonic(
            "abandon zzzz abandon abandon abandon abandon \
             abandon abandon abandon abandon abandon about"
                .to_owned(),
            None,
        );
        match resolve_seed(source) {
            Err(WalletError::InvalidMnemonic {
                word_index: Some(1),
            }) => {}
            Err(other) => panic!("expected InvalidMnemonic at index 1, got {}", other.code()),
            Ok(_) => panic!("invalid mnemonic must not resolve"),
        }
        // bad checksum/count (valid words, wrong shape) — typed, no index
        let source = SeedSource::mnemonic("abandon abandon abandon".to_owned(), None);
        assert!(matches!(
            resolve_seed(source),
            Err(WalletError::InvalidMnemonic { .. })
        ));
    }

    // ── USK/UA derivation — golden vectors (§8) ──────────────────────────────
    //
    // The pins are self-generated goldens (churn detection over the audited
    // zcash_keys/zip32, which own ZIP-32/ZIP-316 vector conformance upstream).
    // Each pinned string is ALSO decoded independently via `zcash_address` (a
    // DIFFERENT crate from the one that encoded it) in `assert_g7_receivers`, so
    // a wrong/typo'd pin cannot pass and the RECEIVER SET — not just the bytes —
    // is asserted (crypto audit W3-inc-2: a string-only KAT let an Orchard-only
    // UA slip past the G7 contract).

    const KAT_SEED_32: &[u8] = b"relim-wallet-kat-seed-0123456789";
    const TREZOR_PHRASE: &str = "abandon abandon abandon abandon abandon abandon abandon abandon \
                                 abandon abandon abandon abandon abandon abandon abandon abandon \
                                 abandon abandon abandon abandon abandon abandon abandon art";

    /// Independent cross-crate G7 check: decode `encoded` with `zcash_address`
    /// (not the `zcash_keys` path that produced it) and assert the receiver set
    /// is Orchard + Sapling, NO transparent (p2pkh/p2sh), on the expected net.
    fn assert_g7_receivers(encoded: &str, expect_net: zcash_protocol::consensus::NetworkType) {
        use zcash_address::unified::{Address as Ua, Container, Encoding, Receiver};
        let (net, addr) = Ua::decode(encoded).expect("pinned string decodes as a unified address");
        assert_eq!(net, expect_net, "network mismatch on the pinned UA");
        let (mut orchard, mut sapling, mut transparent) = (false, false, false);
        for r in addr.items() {
            match r {
                Receiver::Orchard(_) => orchard = true,
                Receiver::Sapling(_) => sapling = true,
                Receiver::P2pkh(_) | Receiver::P2sh(_) => transparent = true,
                Receiver::Unknown { .. } => {}
            }
        }
        assert!(orchard, "G7: orchard receiver required");
        assert!(sapling, "G7: sapling receiver required");
        assert!(!transparent, "G7: NO transparent receiver");
    }

    fn trezor_vector_seed() -> SeedPayload {
        resolve_seed(SeedSource::mnemonic(
            TREZOR_PHRASE.to_owned(),
            Some("TREZOR".to_owned()),
        ))
        .expect("vector mnemonic resolves")
    }

    #[test]
    fn kat_fixed_seed_derives_pinned_unified_address() {
        use zcash_protocol::consensus::NetworkType;
        // §8 named test (gate 1 / C-2-adjacent): a fixed 32-byte seed → the
        // EXACT UA string. ANY librustzcash bump that moves derivation fails
        // HERE, in CI — never on a user's funds. The audited crate owns
        // ZIP-32/ZIP-316 vector conformance; this pins OUR wiring (network
        // params, receiver request, encoding) + detects churn. Self-generated
        // golden: derived once at unpark, pinned, never regenerated without
        // a crypto audit (review-recorded, §11).
        assert_eq!(KAT_SEED_32.len(), 32);
        let ua = derive_default_address(Network::Main, KAT_SEED_32).expect("mainnet derivation");
        assert_eq!(
            ua,
            "u1k8lsjzwd95kp4tsx9dve0ls4q4kaqv7g9hpva70yatwk77w8g8h708nw6d6jzykatlsjtum6shdns\
             xmu9eaczhkexvy54gu8qdpedcr8p698gxrw8zlcp8dxzpfvc8s59r3psdffkl9s5kwgjv33vx47u7w9q\
             ha8kuvezjqv8czjrdrc",
            "pinned mainnet UA moved — a dependency bump changed derivation"
        );
        let tua = derive_default_address(Network::Test, KAT_SEED_32).expect("testnet derivation");
        assert_eq!(
            tua,
            "utest1dt3e4jfg3epjmxlaccuw6g2vslad0mue4czser4v9y2m2fs7tr7f9e0e4k0u87vgsxcdt976dezd\
             umg5yv2rcrnjc63evkxcsgflgth8vgzca8henfcuh690eyuqvhd24h4nlvlksyxjfwpmvej86yatyqzpzw\
             ztrt2qq52ulc64wqg4",
            "pinned testnet UA moved — a dependency bump changed derivation"
        );
        // network is a runtime parameter (§2.1) — same seed, different chain
        assert_ne!(ua, tua, "mainnet and testnet UAs must differ");
        assert!(ua.starts_with("u1"), "mainnet UA HRP");
        assert!(tua.starts_with("utest1"), "testnet UA HRP");
        // independent decode: the pinned strings carry the G7 receiver set
        assert_g7_receivers(&ua, NetworkType::Main);
        assert_g7_receivers(&tua, NetworkType::Test);
    }

    #[test]
    fn kat_fixed_seed_derives_pinned_ufvk_encoding() {
        // §3.7 D6, vectors FIRST (crypto-change review step 3): the fixed KAT seed →
        // USK → UFVK → the EXACT `uview…`/`uviewtest…` encoding, pinned — the
        // #397 EXPORT artifact. The audited crate owns ZIP-316 container
        // conformance upstream; this pins OUR wiring (network params, account
        // 0) and detects dependency churn: a silently-moved encoding would
        // strand every watch-only twin created from an old export. Same
        // self-generated-golden discipline as the UA KAT above (derived once,
        // pinned, never regenerated without a crypto audit).
        let usk = derive_spending_key(Network::Main, KAT_SEED_32).expect("mainnet USK");
        let ufvk = usk.to_unified_full_viewing_key();
        let enc = encode_ufvk(Network::Main, &ufvk);
        assert_eq!(
            enc,
            "uview1tt4c7sm6fzp4ukehhdmxqav6eqhnce9ntskwf88cvk0z9vrxtzy7p5lta5dksmzcdesg8\
             7083cm4rm2ckgt8kp6v0psllwnd0cpv4cacn09kv4y5rhgtm24qhhxgz9nfcl8neg8fenffh4kw\
             0jk5k7v2mpsrk6l96c73c24u4rv5p7msae87z5jcfsea8ggktrpx4se6es29tf5skj9e4eyury8\
             8qww2peryff4rdpwawlc9rxg0m3zpsg3pl5qcgkxswfmksdj4zdfr83e98ytqeayrd2jr47pxza\
             jcpx3puaa0l68rqmpm0jp9c7zhfhk5exhxe6rqg26ktagjugjnfw8cez92dke08efmdmz6anpg3\
             z5d5j9uwnmlmw7r3qn8mty76ljdzse99k7xfxgcerskuz2l57w2qg6chday5pfkl9e7w35h6lmc\
             zkgx0yt8znzy97dp6p8ggch97y5xxq52mkmx0gq76kp842nxwmc92glxnefk",
            "pinned mainnet UFVK encoding moved — a dependency bump changed the export artifact"
        );
        assert!(enc.starts_with("uview1"), "mainnet UFVK HRP");

        let tusk = derive_spending_key(Network::Test, KAT_SEED_32).expect("testnet USK");
        let tufvk = tusk.to_unified_full_viewing_key();
        let tenc = encode_ufvk(Network::Test, &tufvk);
        assert_eq!(
            tenc,
            "uviewtest12amv2eelp4hsajp3ptzfmlwvcjwnma7fhpe99y7drtd0hhluvj9tkczmqzfyf4lww\
             r724z46yetf755nmyzfugshdyn8vnrm4qwl8my9e20smz6xclaut3hqeuth9r8kekgaux7sud9r\
             rvexmwl8g806lelrmw2mu48q2jvx8r40zreh7q6ay7gwffgj4kq6z2lnrn5hpcmh8sfhfzhnmkh\
             svfw2sttdkn9azu9g206slv4mz0ulsmkf85cjtsyy4gy578lqhhuc2vpph8qnqqn2nymzj8fs0p\
             nhpaue6auuxl0fyc4paf4ghn95udsrzkhsjvklasaex3jdvpuegmsfwptmudptyean2jzzcer62\
             9mytqhgc72vep40nhez3qgzmaseh9dwgqtnrw7xldasy6ejd9t5xs6j47g96a9y7ygty2462\
             6cxx8p8s7jxrxzjkym8570zaf2zd3mhf4uunex8zej85425qwscrwhk3wy585v2fmgn",
            "pinned testnet UFVK encoding moved — a dependency bump changed the export artifact"
        );
        assert!(tenc.starts_with("uviewtest1"), "testnet UFVK HRP");
        assert_ne!(enc, tenc, "network is part of the artifact");

        // Round-trip — the §3.7 D2 IMPORT path: decode the exported string and
        // prove the SAME account view (identical default UA ⟺ same UFVK), so
        // export → watch-only import can never silently land on a different
        // account than the exporter's.
        let decoded = UnifiedFullViewingKey::decode(&MAIN_NETWORK, &enc)
            .expect("the export artifact decodes under the same network");
        assert_eq!(
            encode_default_address(&MAIN_NETWORK, &decoded).expect("address from decoded UFVK"),
            derive_default_address(Network::Main, KAT_SEED_32).expect("address from seed"),
            "decoded UFVK views the exporter's account"
        );

        // Cross-network decode REJECTS structurally (the HRP/network binding —
        // §3.7 D2: a wrong-net key CANNOT half-import; no custom check needed).
        assert!(
            UnifiedFullViewingKey::decode(&TEST_NETWORK, &enc).is_err(),
            "a mainnet uview must not decode under testnet params"
        );
        assert!(
            UnifiedFullViewingKey::decode(&MAIN_NETWORK, &tenc).is_err(),
            "a testnet uview must not decode under mainnet params"
        );

        // Hostile-input floor (§4.6): garbage and a tampered artifact are typed
        // rejects at the decoder, never panics.
        assert!(UnifiedFullViewingKey::decode(&MAIN_NETWORK, "").is_err());
        assert!(UnifiedFullViewingKey::decode(&MAIN_NETWORK, "uview1garbage").is_err());
        let mut tampered = enc.clone();
        tampered.pop();
        assert!(
            UnifiedFullViewingKey::decode(&MAIN_NETWORK, &tampered).is_err(),
            "a truncated export artifact is a typed reject"
        );
    }

    #[test]
    fn decode_ufvk_maps_each_failure_class_to_its_typed_error() {
        // #397 (code review n1): pin `decode_ufvk`'s three arms directly on the
        // wrapper (the create-pipeline test covers them end-to-end; this
        // localizes a regression to the decoder). A well-formed cross-network
        // artifact ⇒ NetworkMismatch (probe-decode); garbage ⇒
        // InvalidViewingKey; oversized ⇒ InvalidViewingKey (size cap, no probe).
        let main = encode_ufvk(
            Network::Main,
            &derive_spending_key(Network::Main, KAT_SEED_32)
                .expect("usk")
                .to_unified_full_viewing_key(),
        );
        let test = encode_ufvk(
            Network::Test,
            &derive_spending_key(Network::Test, KAT_SEED_32)
                .expect("usk")
                .to_unified_full_viewing_key(),
        );

        // Same-network round-trips.
        assert!(decode_ufvk(Network::Main, &main).is_ok());
        assert!(decode_ufvk(Network::Test, &test).is_ok());
        // Cross-network — both directions — is NetworkMismatch, NOT garbage.
        assert!(matches!(
            decode_ufvk(Network::Test, &main),
            Err(WalletError::NetworkMismatch)
        ));
        assert!(matches!(
            decode_ufvk(Network::Main, &test),
            Err(WalletError::NetworkMismatch)
        ));
        // Garbage decodes under NEITHER network ⇒ InvalidViewingKey.
        for bad in ["", "uview1garbage", "not-a-key", "utest1garbage"] {
            assert!(matches!(
                decode_ufvk(Network::Main, bad),
                Err(WalletError::InvalidViewingKey)
            ));
        }
        // Oversized rejects at the size cap (before any decode/probe).
        let huge = "u".repeat(crate::constants::UFVK_MAX_BYTES + 1);
        assert!(matches!(
            decode_ufvk(Network::Main, &huge),
            Err(WalletError::InvalidViewingKey)
        ));
    }

    /// The 20 MAINNET `unified_fvk` encodings from the OFFICIAL
    /// `zcash/zcash-test-vectors` repo (`test-vectors/json/
    /// unified_full_viewing_keys.json`, master @ `78321beacb0e`,
    /// 2026-07-22) — consumed verbatim; the component-byte fields are NOT
    /// vendored (librustzcash owns container assembly; we consume only the
    /// encodings our import door must accept). TWO of the 20 (vectors 0 and
    /// 13 — per the upstream generator's 1-in-4 coin flip) carry an
    /// UNKNOWN-typecode FVK item, so the round-trip below also proves
    /// unknown-item PRESERVATION (a future-typecode key imported today must
    /// re-export byte-identical, not silently stripped) — do NOT prune those
    /// two rows in any future trim; they are the only carriers of that
    /// property.
    const OFFICIAL_UFVK_VECTORS: &[&str] = &[
        "uview1cgrqnry478ckvpr0f580t6fsahp0a5mj2e9xl7hv2d2jd4ldzy449mwwk2l9yeuts8\
         5wjls6hjtghdsy5vhhvmjdw3jxl3cxhrg3vs296a3czazrycrr5cywjhwc5c3ztfyjdhmz0e\
         xvzzeyejamyp0cr9z8f9wj0953fzht0m4lenk94t70ruwgjxag2tvp63wn9ftzhtkh20gyre\
         3w5s24f6wlgqxnjh40gd2lxe75sf3z8h5y2x0atpxcyf9t3em4h0evvsftluruqne6w4sm06\
         6sw0qe5y8qg423grple5fftxrqyy7xmqmatv7nzd7tcjadu8f7mqz4l83jsyxy4t8pkayyty\
         k7nrp467ds85knekdkvnd7hqkfer8mnqd7pv",
        "uview1672278wdrucacpeujnzt2tuhtdlj5e6ecjlgl72ul9rtud4ycnjactva9clneg2q39\
         sva69kdx5frq0f4h7pk9y40zl3pgjffff9d6n0fxvjhpsk66zx5g5336hf8qrr3q0whm9tt5\
         m58j8gssnykts6rsyl0e6hwulqr7wn6zyjt57d8us0ydqh722xcsg8vnr0sssczrdwk7mv5m\
         eheqg3r0try3druwk8cyeykzy86gucu2jmttvvzxcetclmkulq3ulrppfnd50zwplkvd5eql\
         dmsw9c3ujws0fv",
        "uview18jpf4cjyt5nfa747ua4sawtv9cppl8g576a4utmtslvenzm2ajyefz2fye2w7ljjm6\
         3f6r903fuhdm6fmvg3dnpgxw07tllfq7hcede8qyl2fanaarvsm8d0trz5ckc7k47dne78mf\
         w5lrkgc883akkaw2vt37cdmvy6snapxufr857r5p9vmf9jx3s00w73we6fz4w49yy9wdc3u9\
         2krx0vs05t3c4r6favdtg9uj2tqs94skm5xdn9q4vpwgfmkcgwl3c8sj4epph8f69839q8p7\
         pt86xvd5ejs7k87dn3tlwfenzqzlhalm7vwwaqy56mmdysdnqemmses6s653n6xq4",
        "uview1aqjrk6swt0f8w23ev5gumkz3dn99097fa2ne8gpmtra7z0lt2xv8qp7vzvfgfxmcdz\
         y8jtpftdujuvm7plle92476vws0m95zlffyp7rr0v2rzv6nyhrvh60at9qqswtj8yl44vt9g\
         dwusz5l6p2dzczrm5zml3uqcudg9eeac4rg0v9yehquzh4x4l3wrc67mqu2uvqwt044jsvhv\
         78hu5ghxtd7reycf3lwurxgyr5r872sssj8h6mr4crwc09n0d68c0ylswhxwy6k05vrk0t63\
         k6ydcv578v8y4znastxpnrgwh0pa2hx7g9pr2z7dmwwwtfgcv3lyraxyt9s3eag3h",
        "uview19emmwhm9qyrg3l3twe4l5x0dm4ur6q4nvs7w6mgurkk56nyzqr6j86fg6kwnsh0mah\
         9q54gynmkxfasseck4k8pdvgsx8v2u8jdn08j8msrsnzphrhev8cq0scma9z3kkype8m6jmg\
         jay6ncet9jjlpskzvyzxczfhhtvj47zze0ctrk3pa89j6cnmndn6ncrvfmms4h0s234pf6zm\
         tt6njysvlqm5m44rv04g6d059gj8v3xdr5p2d3rnplm7hzmljjn8t5k89djpyl8r23gwswwy\
         h03lvg6g8u4pan",
        "uview18nq2gepps6tpwl9cagzljngkt334p53akqwkde2nh54xgptf3ccd4tyeh3glek9pmv\
         k6axxks8yl9h75pquntlm9g9f7arhfr63n6907tnfaxs70nw4pu88fxsn3n8awp29cju7r4h\
         5vvv25espnnny3sdkt69hgdgqehkalxwzegur8tz70jzc2ws2rk",
        "uview18dtp7qna4jcl9k8595lqxym4dehzmpsueh029gc7wqkrke6xdk28dyej6mcg20qn3e\
         8hzdyvfy3jugw2yf7krpfkte6ff8adnjka9zx3agmcjpu7f8uvsc72a4acszhkmjgxmsv37v\
         e3uz0sc2dqvuxfk25fphqcngy45teuevy2wvpgx0jt8wqsfqg7g",
        "uview1d06wgkvfsw53jacy4g3aw2kgakfua83vjxdvaz772amdj4d0jk6cdsmqpzrx94qp0l\
         7tn7qfvyz5r3qrdrzn5tdszktgakfym40wkwcqsgk58436phmx2y4qj3js0l0y6mpu9p4r7t\
         gq8edyr8w9xnulpwdjktts0pqxfdnewxla4m6pd3pglu7kjrjw8u9dg5f6aneg5nza79kag0\
         nn5afea7vnjhrqrv46x65qxgl0dd97x0fg0jqgqmyw422my8e5s8lnqyjh5a7sr2pfxac055\
         62wamnc5zfswak",
        "uview1g20gprekfc6tla7udwmql5c4p4nd27r0frxvgkusrf0g60ewf2s5l09s8rg03nvyt6\
         mgksefjan600x967vcyacxft45r6j5x9zusavwe8ckkwn44f44dvd38q7j6q58ejcstx77n5\
         nrdkj5psyh6d0qmaf06vc3ja3s9kh0rw8zgpmw0svtfeg8jw89h",
        "uview1qtdjghnl06nh4tuypvextrazr9uxes52h64wgh7xkwpf5ahvugkaevdswnryh83fwe\
         vdwr905f9gqgm7vx70q7p8grl6cj99zmhjmn7phge5qmp97fy04544kcey9m48yyrdqakl86\
         azp44jmweyn9yy70qhjp0rdyq0j34umhahesy6tsu78syu2s92gwcgexkk53qktsk4cuk85m\
         aezc0xqrx2ky779mgx047206wnq9vs4jvquf6z79v4d86x0hn2gfj3pe7nhusw5n4ryq3u4u\
         rs2k9d0y4lnhxzrdvhsgext5vc26fgt7zpr67vgdn9rlgnv9mkcnnj8pcxe70z4ktvef7mfp\
         235frwavsp49pf0gcz0yxjhdgw3tf9l2eq6pm3vch44xtu3nsdwr6nfuus7887r2hdfrsny4\
         0sjx74cc5k500x009q0gv222af5plg7acngnc96apy696q3fyr34zgk6lfey0948tkrkn7cy\
         r6v5e8",
        "uview1902pmxaumj7fah9sa8uhgtep7cdh2a83zg0zy5egf2ugg7cxzjygt0hmujpkavjunr\
         a2qw7434hws4qqjwnm8k28rz0nrxr5l40shql96uvrwydt8vh4sqp9lxjjagesn4x6e7vtwp\
         na8xcxesl0sjyzgfemtxa5wlnne4f0wggjm8uv9env43kphnnsyaw5whjt59krzwfx7f6du4\
         95cpqtwl9274juj3kl548fp5atljzqmd64tvdl98eg30pp9nhehmtrsvpss349g5rg3c44q2\
         2zh0gkhsaafm7l",
        "uview1gu6u40juyxfpqe5afu6udqj4hkzfqd7xtk7hrhk7yllqsmzdfdrgqq66x94rpfgh0z\
         6en85jqatp0cla2ylvuqwl75rt3qsqdlufk3r65a3rynvzpwwnh0529uczw5x5rhe4qmve0d\
         9p6qjyh9yjkaj0zwjxwk3vh388z4efxy5ushu6upphejg9thhyy",
        "uview1krnvjn9nk9ysyzazqctrwct7xpkp75hr09zu8laz8ae5kuj9tgujr5ufm2fadxmyr9\
         cl2ycsmeednh4jdeyt7ttzq7c7rjhqn7w3wq50l2xec85stczj2wvp7cu6uc2du6ye00q00f\
         g90vnfwrwyuwfctnfzwk6zk489q0avc4juehte32lktanszeu7h8zus0xp3cg6sk8nstahex\
         05watuw05e483qyhnf6r2dxv5qyte3",
        "uview1unwwq6rjtxhw7nhqp9grelqhdgdtg0zs8npa2dl92wmkspjxq50usmlhvjkjln2mnr\
         e5td82uvjz03039fppk0kh68l5krg5v3e0jpgdaz08qhlr2nad9g99f60xydehl76g85hvf0\
         e2ayue9qvuu2ltymq5dm02hdy06hr39sj3pe0pfw3ghvrvlrf0psav8xv2qshzrcm72azxda\
         swqxgy2wyd2ur983vcxwldyfa0hkq6jqz4ld62mvudmhp37c73e78hd0c6zaf35yh2ga38h6\
         qc0sn67c4uh823e0j7kzy3vlt3j44r9uzm3ajygdr84cm9jgyls7aqqs7xa6fzvu2998fkjg\
         pm2pwgrpzprwvp9yh4eyzc523zegrp00qpfv3weyn4lssv4skwun9p9ghmuz0004ztgt3zer\
         f9cldfjrpcxa0gzmp6faym688dqru50z9frvm8wr3awxw6je67fsu9983d62darra6dtwvtc\
         c8vuse37zshng62jfydzsj294mafgtth40y",
        "uview1dgucetjuudl3msv6tcecj0getnxsdpk5j60h74nckdtrrqs5p4gvf8a3s3lflxednp\
         r774sx7tyaqfxw0s8mnnl7nk7h4m5m3y8xl7efgjprar8sdq6k3mha9l9cln5wy3s6k4g7ma\
         0tc7y30x5xwl7ezxrzhvnrgd8q9zzh4l088mta932ml7wll39y3s82qmse58ulr7p6rj59lh\
         9v0g7evcpghu3ntnh6jr2jzj2vdc904ahhlkq6sn0jdyt3l4s0ch6af7nvh4emlspt32hgk5\
         t59mg3nuxnjqpvpx26vwcfvpuhm8vmpy0ray4mhwhtz9de8gvwg423rpw7l899fvyu0wmr5l\
         p5lu46ckufa2gpt9msjpxxach5cnuwaxph00e65tfmelxyed78t57t4g7f8v9q9fq0378g23\
         yql2t9tp78g3mp0xl30q623rmxglhknl79e7ltwurfvpw7w4k57yzv97exq9h0fv0urgvffu\
         6dqmk5",
        "uview1rthj4lgqnkt8dsufstsfcecvm5fnaf8huns3ts73jpluu0npntj2nr8mjuxkch75nv\
         pnrdfmq87h2yugl42jt3mufg2ng9eqr7qp38sj3cn4lfzpka6f9cs77cvwqdq7m6r98q239q\
         a2x62ehwnleennnk396gvmpd04gaeg78les7wp6kqsrrguyaufs",
        "uview1mt6c2qzyuseptz92z54vtg8dflnsyerulrwq9qyq7fvg5vdvl5au7sggfpwg4f7wh3\
         dnxrpwv3htnwsa8dmrgfgeq59s9fspcvzj92wzxmexees6dh62d4x5j230munyrrhsajccew\
         vhqfnxngxpeyqfrj9yxenel6jxpfrq98yhyn2y8n9dsr2pg46sg7lnkh76fkvujutgv3jt5g\
         vwpxmxlerr8lyz25lsvx0amaun8ju8z6zep4umwcg35dssjw84kp4q8au6yq6aqg22n0wwjw\
         pjm6lr9yj59etf",
        "uview1ve092gfgyrcmwsklfeam3wt6pvm0nhp78ca0n7vspkdw6q4l8sknaacvf7vgxc4xdj\
         vpcxrf0z852fnzspqk4h90594ac00q7els0mvqx9wxm9kugu7ywrv9aca4qdrmv2wdl95sr6\
         ax932p02703xtc2xywzparxqxgj952ngf09wdync4569m0hmewxnxfflwr54dx27l4j3yr8j\
         8c002wlqkktcp776u0l7mpxk9apdphj364fcs2tdujca7ecvn8cyxmkqf2m7lx90mlam0fv3\
         vwyx099yeqyy024mw6nm75wym4y56zhq4tkugue3cy3qpgyvm6vrekyuyyljk3e4uk0h9nn7\
         5aerfnfzzd63mcrzfk7kslpnep56f2d2ug4ssxfnt6g",
        "uview1tzj6u70w4tcef9rmqwdzs7jzm0aemgtunfhzhlzk9rcge29tjx8jjz7mj2tya9x3n0\
         wgxs7tk3xkw5rhyyudqr857z0wmyj9l284fdz4g3fc6ntnekuhm47pp85apv9j7j9hgdzl49\
         jnzrjhfgv4h0arzp38nnzy5j379cjrg5g5asg6j2txl2x9tupycukcqp6gqyshwpefglu2vg\
         gasdn23zcu8y7ew6e2x5cvf69gs0gfdyevh0wj2aw6wq2f9ajqhzz6lzm9lrtsthrr5zh959\
         54uydw9ullgafm",
        "uview1wsv73cyjdg8tyqe06nnmqg42dmcvvd8ah8wmkrdaatjv83csf2g73hd962cayrydg0\
         td82uccn32nzl7hwky22fnjas4crh26aj3gqj6hsfazrff3wvusvq40lauh267qamqtu24dh\
         7tzmzh9gmma4emm9ffz76r7qpk6w3eqn6wmp9425yn27thj7t56a0mlu677j0tr3h04tv7hh\
         4m93xmd4s28tp0a79ct72j2upnf76fqdalr496rjdzh62la54akxs9hxmd9rrwr2yswaw6hq\
         vgzt87r6hr50qgvey5q42xn4lqzp349gv4mt2sj64sx0qtarjcsetkrkw4zxq35ny2fctc9z\
         q7qf979qddwfh4v6a5mpq950m62md40n9mw7q8lmdat",
    ];

    #[test]
    fn official_zcash_test_vectors_ufvks_decode_and_round_trip() {
        // crypto-change review step 3, the OFFICIAL half (the KAT above is the
        // self-golden OUR-WIRING pin; it cannot catch a self-consistent
        // encode+decode bug — these vectors can, because the expected
        // encodings were produced by the REFERENCE implementation, not us).
        // For every official vector: (1) it passes OUR import door
        // (`decode_ufvk` — the size cap + typed-error + probe-decode wrapper,
        // not the bare upstream decoder), (2) OUR export encoder re-encodes
        // it BYTE-IDENTICAL (ZIP-316 F4Jumble/typecode/HRP conformance +
        // unknown-item preservation), and (3) under the WRONG network it is
        // the typed `NetworkMismatch` — a well-formed foreign artifact, never
        // conflated with garbage (§3.7 D2). No derivation comparison here on
        // purpose: the vectors compose arbitrary component sets (8 of 20 have
        // no transparent item; vectors 0 + 13 carry an unknown item) while
        // OUR account derivation policy is the fixed seed→USK triple — the
        // KAT above owns that pin.
        for (i, vector) in OFFICIAL_UFVK_VECTORS.iter().enumerate() {
            let decoded = decode_ufvk(Network::Main, vector).unwrap_or_else(|e| {
                panic!("official vector {i} rejected by our import door: {e:?}")
            });
            assert_eq!(
                encode_ufvk(Network::Main, &decoded),
                *vector,
                "official vector {i} did not round-trip byte-identical through our encoder"
            );
            assert!(
                matches!(
                    decode_ufvk(Network::Test, vector),
                    Err(WalletError::NetworkMismatch)
                ),
                "official mainnet vector {i} must be a typed NetworkMismatch under testnet params"
            );
        }
    }

    #[test]
    fn kat_bip39_vector_seed_derives_pinned_ua() {
        use zcash_protocol::consensus::NetworkType;
        // Provenance anchor: feed the seed derived from the OFFICIAL BIP39
        // reference vector (Trezor #1, pinned byte-for-byte in
        // `kat_bip39_trezor_vector_through_resolve`) through UA derivation and
        // pin the result. Ties the two audited derivations end-to-end: a drift
        // in EITHER bip39 resolution OR zcash_keys derivation fails a test.
        // This seed is ALSO the G7 regression vector — at diversifier 0 it has
        // no valid Sapling receiver, so under the old `Allow` request it was
        // Orchard-ONLY; `Require` (see `default_address_for`) forces both.
        let payload = trezor_vector_seed();
        assert_eq!(payload.seed().len(), 64, "BIP39 PBKDF2 seed");
        let ua = derive_default_address(Network::Main, payload.seed()).expect("derivation");
        assert_eq!(
            ua,
            "u1wjvadn0ka6ur2t63h8amhu586fshx49lxh3gh6sn4ag49wvkq2yucl3lxl0s72nxtl4hcnnewypax5\
             0mcae542cw9ktvk8k5jq3tsptyluesxnh49a4xjjwjnv79pnafw26akugdvwwpm00dxzmvtf354vuqmz\
             pejnqxyna52yy0lm8y",
            "UA from the BIP39 vector seed moved — bip39 or zcash_keys derivation drifted"
        );
        assert_g7_receivers(&ua, NetworkType::Main);
    }

    #[test]
    fn default_ua_receiver_set_is_shielded_only() {
        use zcash_protocol::consensus::NetworkType;
        // G7 (spec §3.1): Orchard + Sapling present, NO transparent receiver —
        // asserted on the LIVE-derived UA (complements the pinned-string decode
        // in the KATs) over BOTH the fixed seed AND the BIP39 regression seed.
        // The regression seed is the one that, under the old `Allow` request,
        // produced an Orchard-ONLY UA (crypto audit W3-inc-2); this proves the
        // `Require` fix lands both shielded receivers on the exact failing seed.
        let trezor = trezor_vector_seed();
        for seed in [KAT_SEED_32, trezor.seed()] {
            let main = derive_default_address(Network::Main, seed).expect("mainnet UA");
            assert_g7_receivers(&main, NetworkType::Main);
            let test = derive_default_address(Network::Test, seed).expect("testnet UA");
            assert_g7_receivers(&test, NetworkType::Test);
        }
    }

    // ---- Recv-4 (FR-8, ADR-0537): public diversified receive UAs ----

    /// Account 0's UFVK from a raw seed — the same audited two-step
    /// (`from_seed` → `to_unified_full_viewing_key`) `default_address_for` runs;
    /// test-local so the Recv-4 KATs derive at CHOSEN indices.
    fn kat_ufvk(network: Network, seed: &[u8]) -> UnifiedFullViewingKey {
        derive_spending_key(network, seed)
            .expect("spending key derives")
            .to_unified_full_viewing_key()
    }

    /// Test-side mirror of the Recv-4 allocator walk at the pure UFVK level: from
    /// `start`, the first index whose G7 derivation conforms, plus the encoded UA.
    /// `InvalidSaplingDiversifierIndex` is the honest per-index miss (the engine maps
    /// the same error to `Ok(None)`; the allocator burns the index and advances);
    /// any other error fails the test.
    fn first_diversified_from(
        ufvk: &UnifiedFullViewingKey,
        network: Network,
        start: u64,
    ) -> (u64, String) {
        let request = g7_receive_request().expect("g7 request");
        for index in
            start..start + u64::from(crate::diversified_index::DIVERSIFIED_DERIVE_MAX_ATTEMPTS)
        {
            match ufvk.address(zip32::DiversifierIndex::from(index), request) {
                Ok(ua) => {
                    assert!(ua_satisfies_g7(&ua), "G7 post-condition on the minted UA");
                    return (index, encode_unified_address(network, &ua));
                }
                Err(zcash_keys::keys::AddressGenerationError::InvalidSaplingDiversifierIndex(
                    _,
                )) => continue,
                Err(e) => panic!("unexpected derivation error at a fixed index: {e:?}"),
            }
        }
        panic!("no conforming index within the attempt bound (P ≈ 2^-64)");
    }

    #[test]
    fn kat_fixed_seed_mints_pinned_diversified_address() {
        use crate::diversified_index::DIVERSIFIED_INDEX_BASE;
        use zcash_protocol::consensus::NetworkType;
        // §8 named test (Recv-4): the fixed KAT seed → the EXACT first minted
        // diversified UA + its index, from the FROZEN region base (ADR-0537).
        // Pins region placement, the G7 request, the miss-walk determinism, and
        // the encoding — a dependency bump or a region drift fails HERE, never on
        // a published contact address. Self-generated goldens, pinned once,
        // never regenerated without a crypto audit.
        let (index, ua) = first_diversified_from(
            &kat_ufvk(Network::Main, KAT_SEED_32),
            Network::Main,
            DIVERSIFIED_INDEX_BASE,
        );
        // BASE (2^40) itself conforms for this seed on mainnet.
        assert_eq!(
            index, DIVERSIFIED_INDEX_BASE,
            "first conforming index moved"
        );
        assert_eq!(
            ua,
            "u1tgvn5csuc7z3s288z4xwhma9a227p99m92am0fsh4chxt3jh6ry5m4ref2nzy56dguj7z6jyf4u5v8\
             ydjpk00hgm0muq9ntpvnfsvqaujztnyeajgts77w23hhlpjnua6xzw6pgfch2dg3y43dexvznm0sdr77\
             7ucpu5tqf4qug45y0s",
            "pinned mainnet diversified UA moved — derivation or region drifted"
        );
        assert_g7_receivers(&ua, NetworkType::Main);
        let (tindex, tua) = first_diversified_from(
            &kat_ufvk(Network::Test, KAT_SEED_32),
            Network::Test,
            DIVERSIFIED_INDEX_BASE,
        );
        // The testnet UFVK is a DIFFERENT key (coin type 1), so its sapling
        // validity pattern differs — a five-miss run before the first conforming
        // index, pinning the multi-step walk too.
        assert_eq!(
            tindex,
            DIVERSIFIED_INDEX_BASE + 5,
            "testnet first conforming index moved"
        );
        assert_eq!(
            tua,
            "utest1v4m0h5hn9878t9saerdnpazy34jgn39vvr8lqcnxh2hq830622ypq90mkrk2g0adzgvdhx8xge\
             znjjwtnezat2k349y3jlxv304na6m0ulfgjcpzmzwy4cednzkskqn49t5qqq02y02p6qruqa9jp7ymws\
             zdlykx606xwz7d352cu2n5",
            "pinned testnet diversified UA moved — derivation or region drifted"
        );
        assert_g7_receivers(&tua, NetworkType::Test);
        assert_ne!(ua, tua, "mainnet and testnet diversified UAs must differ");
    }

    #[test]
    fn kat_bip39_vector_seed_mints_pinned_diversified_address() {
        use crate::diversified_index::DIVERSIFIED_INDEX_BASE;
        use zcash_protocol::consensus::NetworkType;
        // Provenance anchor (the `kat_bip39_vector_seed_derives_pinned_ua`
        // sibling): the OFFICIAL BIP39 Trezor #1 vector seed through the Recv-4
        // derivation — ties bip39 resolution + high-region ZIP-32 diversifier
        // derivation end-to-end.
        let payload = trezor_vector_seed();
        let (index, ua) = first_diversified_from(
            &kat_ufvk(Network::Main, payload.seed()),
            Network::Main,
            DIVERSIFIED_INDEX_BASE,
        );
        // BASE and BASE+1 are sapling-invalid for the vector seed — the walk lands
        // at BASE+2, which ALSO pins the miss-advance determinism on a real seed.
        assert_eq!(
            index,
            DIVERSIFIED_INDEX_BASE + 2,
            "vector-seed first conforming index moved"
        );
        assert_eq!(
            ua,
            "u16hpve9lakdpgx44yp3tlfk62g7f8g2cgp6n2fe08javgrfrym5y9rqtn9zey04rg2a4yhfsgzzcqnn\
             9060p3y5xgm0f2n4g86d2q7ymwws6nlf63kv308djdzqnqs8kt2ndjkxw284wc0lh246xfz6k9qaked9\
             rxns2p2qfvrsue7v03",
            "diversified UA from the BIP39 vector seed moved"
        );
        assert_g7_receivers(&ua, NetworkType::Main);
    }

    #[test]
    fn diversified_mint_is_deterministic_per_index() {
        use crate::diversified_index::DIVERSIFIED_INDEX_BASE;
        // The FR-8 acceptance: same seed + same index ⇒ the same UA, every time
        // (what lets a host durably attribute a contact/invoice to an index).
        let ufvk = kat_ufvk(Network::Main, KAT_SEED_32);
        let (index, first) = first_diversified_from(&ufvk, Network::Main, DIVERSIFIED_INDEX_BASE);
        let request = g7_receive_request().expect("g7 request");
        for _ in 0..3 {
            let ua = ufvk
                .address(zip32::DiversifierIndex::from(index), request)
                .expect("a conforming index stays conforming");
            assert_eq!(encode_unified_address(Network::Main, &ua), first);
        }
    }

    #[test]
    fn diversified_address_shares_no_receiver_with_the_default_ua() {
        use crate::diversified_index::DIVERSIFIED_INDEX_BASE;
        use zcash_address::unified::{Address as Ua, Container, Encoding, Receiver};
        // The ADR-0537 disjointness teeth: the minted diversified UA and the
        // advertised default UA must share NO receiver bytes — equal diversifier
        // index would mean byte-identical shielded receivers, so this is the
        // direct observable of region disjointness (linkability = a shared
        // receiver, trivially comparable by anyone holding both encodings).
        let receivers = |encoded: &str| -> Vec<Vec<u8>> {
            let (_, addr) = Ua::decode(encoded).expect("UA decodes");
            addr.items()
                .into_iter()
                .filter_map(|r| match r {
                    Receiver::Orchard(b) => Some(b.to_vec()),
                    Receiver::Sapling(b) => Some(b.to_vec()),
                    _ => None,
                })
                .collect()
        };
        let default_ua = derive_default_address(Network::Main, KAT_SEED_32).expect("default UA");
        let (_, minted) = first_diversified_from(
            &kat_ufvk(Network::Main, KAT_SEED_32),
            Network::Main,
            DIVERSIFIED_INDEX_BASE,
        );
        let (d, m) = (receivers(&default_ua), receivers(&minted));
        assert_eq!(d.len(), 2, "default UA carries both shielded receivers");
        assert_eq!(m.len(), 2, "minted UA carries both shielded receivers");
        if let Some(shared) = d.iter().find(|r| m.contains(r)) {
            panic!(
                "a receiver is SHARED between the default UA and a diversified mint \
                 ({} bytes) — region disjointness is broken",
                shared.len()
            );
        }
    }

    #[cfg(feature = "swap")]
    #[test]
    fn diversified_address_shares_no_receiver_with_a_swap_destination() {
        use crate::diversified_index::DIVERSIFIED_INDEX_BASE;
        use zcash_address::unified::{Address as Ua, Container, Encoding, Receiver};
        // The other half of the ADR-0537 disjointness: a swap DESTINATION at the
        // low region's first index vs the public mint at the high region's first —
        // no shared shielded receiver (a swap counterparty must not be able to link
        // a user's published contact address to a swap it served).
        let ufvk = kat_ufvk(Network::Main, KAT_SEED_32);
        let swap_request = swap_destination_address_request().expect("swap request");
        let swap_ua = ufvk
            .address(zip32::DiversifierIndex::from(1u32), swap_request)
            .expect("swap request conforms at every non-hardened index");
        let swap_encoded = encode_unified_address(Network::Main, &swap_ua);
        let (_, minted) = first_diversified_from(&ufvk, Network::Main, DIVERSIFIED_INDEX_BASE);
        let shielded = |encoded: &str| -> Vec<Vec<u8>> {
            let (_, addr) = Ua::decode(encoded).expect("UA decodes");
            addr.items()
                .into_iter()
                .filter_map(|r| match r {
                    Receiver::Orchard(b) => Some(b.to_vec()),
                    Receiver::Sapling(b) => Some(b.to_vec()),
                    _ => None,
                })
                .collect()
        };
        let (s, m) = (shielded(&swap_encoded), shielded(&minted));
        assert!(
            s.iter().all(|r| !m.contains(r)),
            "a shielded receiver is shared between a swap destination and a public mint"
        );
    }

    #[test]
    fn diversified_mint_advances_past_a_sapling_invalid_index() {
        use crate::diversified_index::{DIVERSIFIED_DERIVE_MAX_ATTEMPTS, DIVERSIFIED_INDEX_BASE};
        // The G7 `Require`-sapling miss is a REAL ~50% path at fixed indices: find
        // the first sapling-invalid index in the region and prove the walk from it
        // lands strictly LATER (the burned-index advance), still G7-conforming.
        let ufvk = kat_ufvk(Network::Main, KAT_SEED_32);
        let request = g7_receive_request().expect("g7 request");
        let miss =
            (DIVERSIFIED_INDEX_BASE
                ..DIVERSIFIED_INDEX_BASE + u64::from(DIVERSIFIED_DERIVE_MAX_ATTEMPTS))
                .find(|&i| {
                    matches!(
                    ufvk.address(zip32::DiversifierIndex::from(i), request),
                    Err(zcash_keys::keys::AddressGenerationError::InvalidSaplingDiversifierIndex(_))
                )
                })
                .expect("a sapling-invalid index exists in the first 64 (P(none) ≈ 2^-64)");
        let (landed, ua) = first_diversified_from(&ufvk, Network::Main, miss);
        assert!(landed > miss, "the walk must burn the miss and advance");
        assert_g7_receivers(&ua, zcash_protocol::consensus::NetworkType::Main);
    }

    #[test]
    fn diversified_mint_enforces_g7_receivers() {
        use crate::diversified_index::DIVERSIFIED_INDEX_BASE;
        use zcash_protocol::consensus::NetworkType;
        // The §8 register test by name: the LIVE minted diversified UA (not only
        // the pinned golden strings) carries EXACTLY the G7 receiver set — proven
        // by the independent cross-crate `zcash_address` decode, over BOTH
        // networks and BOTH KAT seeds (the same independence the default-UA
        // receiver test provides for `current_address`).
        let trezor = trezor_vector_seed();
        for seed in [KAT_SEED_32, trezor.seed()] {
            let (_, main) = first_diversified_from(
                &kat_ufvk(Network::Main, seed),
                Network::Main,
                DIVERSIFIED_INDEX_BASE,
            );
            assert_g7_receivers(&main, NetworkType::Main);
            let (_, test) = first_diversified_from(
                &kat_ufvk(Network::Test, seed),
                Network::Test,
                DIVERSIFIED_INDEX_BASE,
            );
            assert_g7_receivers(&test, NetworkType::Test);
        }
    }

    #[test]
    fn derive_default_address_is_deterministic() {
        // Same seed bytes ⇒ identical UA across calls (no diversifier RNG /
        // hidden state in the default-address path).
        let a = derive_default_address(Network::Main, KAT_SEED_32).expect("a");
        let b = derive_default_address(Network::Main, KAT_SEED_32).expect("b");
        assert_eq!(a, b);
    }

    #[test]
    fn short_seed_is_typed_reject_not_panic() {
        // Defense-in-depth (security review W3-inc-2): the door re-enforces the
        // 32..=252 bound with a single `contains()` check. A < 32-byte seed would
        // PANIC inside `from_seed`; BOTH ends return a typed error instead, so a
        // panic across FFI is structurally impossible for any caller. Both
        // boundaries of SEED_MIN_BYTES/SEED_MAX_BYTES are pinned HERE (this guard
        // is a second enforcement path, distinct from seed.rs) per testing-patterns.
        match derive_default_address(Network::Main, &[0u8; 31]) {
            Err(WalletError::InvalidSeedLength { len: 31 }) => {}
            other => panic!("expected typed InvalidSeedLength{{31}}, got {other:?}"),
        }
        match derive_default_address(Network::Main, &[0u8; 253]) {
            Err(WalletError::InvalidSeedLength { len: 253 }) => {}
            other => panic!("expected typed InvalidSeedLength{{253}}, got {other:?}"),
        }
        // The two real wallet seed lengths derive (32 raw, 64 BIP39).
        assert!(derive_default_address(Network::Main, &[0u8; 32]).is_ok());
        assert!(derive_default_address(Network::Main, &[0u8; 64]).is_ok());
        // ADR-0528: with `transparent-inputs` on, `from_seed` ALSO derives the BIP44 transparent
        // key (BIP32 `ExtendedPrivateKey::new`, 16/32/64-byte master seeds only), so an
        // in-range-but-non-BIP32 length (48/100/252) — which the door admits and the shielded
        // path used to derive — is now a typed `KeyDerivation`, NEVER a panic and NEVER a wrong
        // address. The account derivation seed contract is unified with the refund path's BIP32
        // master-seed rule (`refund_address_requires_bip32_master_seed_length`). No production
        // seed is an exotic length (32 raw / 64 BIP39 are the only two), so no real wallet is hit.
        for odd in [48usize, 100, 252] {
            match derive_default_address(Network::Main, &vec![0u8; odd]) {
                Err(WalletError::KeyDerivation) => {}
                other => {
                    panic!("expected typed KeyDerivation for a {odd}-byte seed, got {other:?}")
                }
            }
        }
    }

    // ── inc-2d-2: the transient spending-key derivation (§4.2) ───────────────

    #[test]
    fn derive_spending_key_matches_the_account_address() {
        // §8: the create+sign USK derives from the SAME seed + ACCOUNT_ZERO index as
        // the wallet's account, so its UFVK's default UA byte-EQUALS the address the
        // account/handle shows. This is the load-bearing tie — a USK that derived a
        // DIFFERENT key would sign with funds the wallet can't see / from an address
        // nobody paid. Proven by deriving the UA two independent ways and comparing.
        for net in [Network::Main, Network::Test] {
            let usk = derive_spending_key(net, KAT_SEED_32).expect("USK derives");
            let from_usk = default_address_from_ufvk(net, &usk.to_unified_full_viewing_key())
                .expect("UA from the derived USK's UFVK");
            let from_seed = derive_default_address(net, KAT_SEED_32).expect("UA from the seed");
            assert_eq!(
                from_usk, from_seed,
                "the spending key's UFVK address == the wallet's default address",
            );
        }
    }

    #[test]
    fn derive_spending_key_rejects_out_of_bounds_seed_typed() {
        // §8 (§4.2(b) panic guard): a seed outside 32..=252 would PANIC inside
        // `from_seed`; both boundaries return a typed `InvalidSeedLength` instead, so a
        // panic across FFI is structurally impossible on the signing path too. A second
        // enforcement door, distinct from seed.rs / the address path (testing-patterns).
        for (bad, len) in [(vec![0u8; 31], 31usize), (vec![0u8; 253], 253usize)] {
            match derive_spending_key(Network::Main, &bad) {
                Err(WalletError::InvalidSeedLength { len: got }) if got == len => {}
                other => panic!("expected typed InvalidSeedLength{{{len}}}, got {other:?}"),
            }
        }
        // the two real wallet seed lengths derive (32 raw, 64 BIP39)
        assert!(derive_spending_key(Network::Main, &[0u8; 32]).is_ok());
        assert!(derive_spending_key(Network::Main, &[0u8; 64]).is_ok());
        // ADR-0528: an in-range-but-non-BIP32 length is a typed `KeyDerivation` on the signing
        // path too (the USK's transparent component fails to derive), never a panic.
        for odd in [48usize, 100, 252] {
            match derive_spending_key(Network::Main, &vec![0u8; odd]) {
                Err(WalletError::KeyDerivation) => {}
                other => {
                    panic!("expected typed KeyDerivation for a {odd}-byte seed, got {other:?}")
                }
            }
        }
    }

    // ── Recv-2 (ADR-0528): the transparent RECEIVE-address derivation (§3.3a) ───────
    //
    // Same KAT discipline as the UA derivation: a fixed seed → the EXACT external-scope
    // (m/44'/coin'/0'/0/0) transparent receive address, derived through the PRODUCTION path
    // (seed → USK → UFVK → `transparent_receive_address_from_ufvk`) so the pin guards the
    // exact wiring the stored-account `current_transparent_address` uses. Each golden is
    // independently re-decoded via `assert_transparent_refund` (the `zcash_address` decode
    // path), asserting a transparent P2PKH on the right network, rejected on the other.

    #[test]
    fn current_transparent_address_is_kat_pinned() {
        // §8 (Recv-2): a fixed seed → the EXACT transparent receive address. ANY librustzcash
        // bump that moves BIP44 transparent derivation fails HERE, in CI, never on a user's
        // receive address. These goldens are external INDEX 0 — the same external chain the
        // refund allocator hands out from index 1 (ADR-0528 reserves index 0 for receive), so
        // a refund never collides with this address. Self-generated golden, cross-checked by an
        // independent transparent decode.
        assert_eq!(KAT_SEED_32.len(), 32);
        for (net, golden) in [
            (Network::Main, "t1hD4Xt5goM4Y9cMBCFbQLQNER68Zh1LDjc"),
            (Network::Test, "tmVR3wVRZRtGH37nGdQmcWSkb3XJ1CxAnXF"),
        ] {
            // The seed path (the pre-import fallback) and the UFVK path (the stored-account
            // `current_transparent_address`) MUST agree — both run the SAME shared encoder, so
            // a receive address shown before sync equals the one shown after.
            let from_seed =
                derive_transparent_receive_address(net, KAT_SEED_32).expect("seed path derives");
            let usk = derive_spending_key(net, KAT_SEED_32).expect("USK derives");
            let from_ufvk =
                transparent_receive_address_from_ufvk(net, &usk.to_unified_full_viewing_key())
                    .expect("UFVK path derives");
            assert_eq!(
                from_seed, from_ufvk,
                "seed-derived and stored-account-derived transparent addresses must be byte-equal"
            );
            assert_eq!(
                from_seed, golden,
                "pinned transparent receive address moved — a dependency bump changed BIP44 derivation"
            );
            assert_transparent_refund(from_seed.as_str(), net);
        }
        let main = derive_transparent_receive_address(Network::Main, KAT_SEED_32).unwrap();
        let test = derive_transparent_receive_address(Network::Test, KAT_SEED_32).unwrap();
        assert_ne!(
            main, test,
            "mainnet and testnet receive addresses must differ"
        );
        assert!(main.starts_with("t1"), "mainnet P2PKH HRP");
        assert!(test.starts_with("tm"), "testnet P2PKH HRP");
        // the receive address (external index 0) is DISTINCT from the first refund address
        // (external index 1) — the collision the index-0 reservation removes.
        let refund1 = derive_transparent_refund_address(Network::Main, KAT_SEED_32, 1).unwrap();
        assert_ne!(
            main, refund1,
            "the receive address (index 0) must differ from the first refund address (index 1)"
        );
    }

    #[test]
    fn current_transparent_address_is_hd_recoverable() {
        // HD-recoverable (ADR-0528 / §4.5): a restore-from-seed re-derives the SAME transparent
        // receive address. Proven by deriving from two INDEPENDENT USK instances off the same
        // seed (the restore path rebuilds the USK from the seed bytes) and asserting equality;
        // and that two DIFFERENT seeds derive DIFFERENT addresses (keyed to the seed).
        for net in [Network::Main, Network::Test] {
            let first = {
                let usk = derive_spending_key(net, KAT_SEED_32).expect("USK 1");
                transparent_receive_address_from_ufvk(net, &usk.to_unified_full_viewing_key())
                    .expect("addr 1")
            };
            let after_restore = {
                let usk = derive_spending_key(net, KAT_SEED_32).expect("USK 2");
                transparent_receive_address_from_ufvk(net, &usk.to_unified_full_viewing_key())
                    .expect("addr 2")
            };
            assert_eq!(
                first, after_restore,
                "a restore-from-seed must re-derive the SAME transparent receive address"
            );
        }
        const OTHER_SEED_32: &[u8] = b"relim-wallet-kat-seed-9876543210";
        assert_eq!(OTHER_SEED_32.len(), 32);
        let a = derive_transparent_receive_address(Network::Main, KAT_SEED_32).unwrap();
        let b = derive_transparent_receive_address(Network::Main, OTHER_SEED_32).unwrap();
        assert_ne!(
            a, b,
            "different seeds must derive different transparent receive addresses"
        );
    }

    #[test]
    fn transparent_receive_address_rejects_out_of_bounds_seed_typed() {
        // §8 (code review fold): the receive-address seed door is its OWN enforcement path,
        // pinned at its boundaries like the sibling derivations (`short_seed_is_typed_reject_not_panic`,
        // `derive_spending_key_rejects_out_of_bounds_seed_typed`). A < 32 / > 252 length is a typed
        // `InvalidSeedLength` (the §4.2(b) panic floor — `from_seed` PANICS under 32); an in-range
        // non-BIP32 length is a typed `KeyDerivation` (ADR-0528's unified seed contract). Never a panic.
        for (bad, len) in [(vec![0u8; 31], 31usize), (vec![0u8; 253], 253usize)] {
            match derive_transparent_receive_address(Network::Main, &bad) {
                Err(WalletError::InvalidSeedLength { len: got }) if got == len => {}
                other => panic!("expected typed InvalidSeedLength{{{len}}}, got {other:?}"),
            }
        }
        // the two real wallet seed lengths derive (32 raw, 64 BIP39)
        assert!(derive_transparent_receive_address(Network::Main, &[0u8; 32]).is_ok());
        assert!(derive_transparent_receive_address(Network::Main, &[0u8; 64]).is_ok());
        // an in-range-but-non-BIP32 length is a typed KeyDerivation, never a panic
        for odd in [48usize, 100, 252] {
            match derive_transparent_receive_address(Network::Main, &vec![0u8; odd]) {
                Err(WalletError::KeyDerivation) => {}
                other => {
                    panic!("expected typed KeyDerivation for a {odd}-byte seed, got {other:?}")
                }
            }
        }
    }

    // ── W-swap-3-a: the fresh transparent refund-address derivation (§2.6 HARD-H) ──
    //
    // Same KAT discipline as the UA derivation above: a fixed seed → the EXACT t-addr at
    // a fixed index (any zcash_transparent/bip32/secp256k1 bump that moves BIP44 transparent
    // derivation fails HERE, never on a user's refund). Each pinned string is independently
    // re-parsed via `memo::Address` (the `zcash_address::try_from_encoded` path — a DIFFERENT
    // decoder than the `ToAddress` encoder that produced it), asserting it is a TRANSPARENT
    // P2PKH on the expected network. Self-generated goldens (derived once at unpark, pinned).

    /// Independent cross-path check: re-parse `encoded` via `memo::Address` (the audited
    /// `zcash_address` decode path, not the `ToAddress` encode path that made it) and assert
    /// it is a transparent, transparent-only address on `net`, and that the OTHER network
    /// rejects it (the network byte is correct, never a cross-net footgun on a refund).
    fn assert_transparent_refund(encoded: &str, net: Network) {
        let parsed =
            crate::memo::Address::parse(encoded, net).expect("refund addr parses on its net");
        assert_eq!(
            parsed.kind(),
            crate::memo::AddressKind::Transparent,
            "a refund address MUST be transparent (the provider refunds on-chain to it)"
        );
        let other = match net {
            Network::Main => Network::Test,
            Network::Test => Network::Main,
        };
        assert!(
            matches!(
                crate::memo::Address::parse(encoded, other),
                Err(WalletError::NetworkMismatch)
            ),
            "the refund t-addr must be rejected on the other network"
        );
    }

    #[test]
    fn kat_fixed_seed_derives_pinned_refund_address() {
        // §8 (gate 1 / C-2-adjacent): fixed seed → EXACT external-scope t-addr at index 0/1.
        assert_eq!(KAT_SEED_32.len(), 32);
        let m0 = derive_transparent_refund_address(Network::Main, KAT_SEED_32, 0).expect("main 0");
        let m1 = derive_transparent_refund_address(Network::Main, KAT_SEED_32, 1).expect("main 1");
        let t0 = derive_transparent_refund_address(Network::Test, KAT_SEED_32, 0).expect("test 0");
        let t1 = derive_transparent_refund_address(Network::Test, KAT_SEED_32, 1).expect("test 1");
        assert_eq!(
            m0, "t1hD4Xt5goM4Y9cMBCFbQLQNER68Zh1LDjc",
            "pinned mainnet refund t-addr (idx 0) moved — a dependency bump changed derivation"
        );
        assert_eq!(
            m1, "t1PYmumjr3s4UJYDXgLfrTrWpFBTWvoKZpT",
            "pinned mainnet idx 1 moved"
        );
        assert_eq!(
            t0, "tmVR3wVRZRtGH37nGdQmcWSkb3XJ1CxAnXF",
            "pinned testnet refund t-addr (idx 0) moved"
        );
        assert_eq!(
            t1, "tmGLAsTYFwpGapAMocUzGVUx7tWUiTLCRCL",
            "pinned testnet idx 1 moved"
        );
        // HRP sanity: mainnet P2PKH = `t1`, testnet P2PKH = `tm`.
        assert!(
            m0.starts_with("t1") && m1.starts_with("t1"),
            "mainnet P2PKH HRP"
        );
        assert!(
            t0.starts_with("tm") && t1.starts_with("tm"),
            "testnet P2PKH HRP"
        );
        // fresh index ⇒ a DIFFERENT address (the HARD-H never-recycle property at the
        // derivation level: two swaps never share a refund t-addr, so they can't be linked).
        assert_ne!(
            m0, m1,
            "index 0 and 1 must derive distinct refund addresses"
        );
        assert_ne!(t0, t1);
        assert_ne!(m0, t0, "mainnet and testnet refund addresses must differ");
        // independent decode: transparent P2PKH on the right network, rejected on the other.
        assert_transparent_refund(&m0, Network::Main);
        assert_transparent_refund(&m1, Network::Main);
        assert_transparent_refund(&t0, Network::Test);
        assert_transparent_refund(&t1, Network::Test);
    }

    #[test]
    fn refund_address_at_index_equals_the_ufvk_external_receiver() {
        // #368 CORNERSTONE (the mint cross-check's test twin): the raw BIP32 refund
        // derivation ([`derive_transparent_refund_address`] — AccountPrivKey::from_seed →
        // external ivk → derive_address(i)) and the UFVK path the engine's
        // `get_address_for_index` receiver rides (USK::from_seed → UFVK.transparent() →
        // external ivk → derive_address(i)) MUST agree byte-for-byte at every index. If
        // they ever diverged, #368's engine registration would watch a DIFFERENT address
        // than the refundTo the provider holds — refunds invisible again, while the code
        // claims visibility. Both derivations are audited crates taken whole; this pins
        // that they remain two routes to ONE address. (The engine-backed half — the real
        // `get_address_for_index` on a provisioned WalletDb — is pinned in wallet.rs.)
        fn ufvk_external<P: Parameters>(p: &P, index: u32) -> String {
            let usk = UnifiedSpendingKey::from_seed(p, KAT_SEED_32, ACCOUNT_ZERO)
                .expect("usk from the KAT seed");
            let ufvk = usk.to_unified_full_viewing_key();
            let ivk = ufvk
                .transparent()
                .expect("transparent component (post-ADR-0528)")
                .derive_external_ivk()
                .expect("external ivk");
            let taddr = ivk
                .derive_address(NonHardenedChildIndex::from_index(index).expect("non-hardened"))
                .expect("derivable at a real key");
            encode_transparent(p, &taddr).expect("p2pkh encodes")
        }
        for network in [Network::Main, Network::Test] {
            for index in [1u32, 2, 7, 1000, (1 << 31) - 1] {
                let raw = derive_transparent_refund_address(network, KAT_SEED_32, index)
                    .expect("raw refund derivation");
                let via_ufvk = match network {
                    Network::Main => ufvk_external(&MAIN_NETWORK, index),
                    Network::Test => ufvk_external(&TEST_NETWORK, index),
                };
                assert_eq!(
                    raw, via_ufvk,
                    "raw BIP32 and UFVK external receivers diverged at index {index} on \
                     {network:?} — #368 registration would watch the wrong address"
                );
            }
        }
    }

    // Pre-existing no-swap test-compile breakage: this
    // test drives swap-gated helpers, so it is swap-scoped — the un-gated Recv-4
    // tests now actually RUN in the no-swap lane.
    #[cfg(feature = "swap")]
    #[test]
    fn stored_address_decoder_accepts_both_self_minted_forms() {
        // #368: the scoped poll's decoder takes a bare refund P2PKH t-addr AND rejects the
        // forms we never store (network mismatch = tamper). The UA form is exercised by the
        // shipped destination round-trip tests; here the NEW t-addr arm is pinned against
        // the actual refund derivation output.
        let taddr =
            derive_transparent_refund_address(Network::Main, KAT_SEED_32, 1).expect("refund");
        let receiver = transparent_receiver_from_stored_address(Network::Main, &taddr)
            .expect("the refund t-addr decodes to its receiver");
        assert_eq!(
            encode_transparent_receiver(Network::Main, &receiver).expect("re-encode"),
            taddr,
            "decode→encode round-trips the exact stored refund address"
        );
        assert!(
            matches!(
                transparent_receiver_from_stored_address(Network::Test, &taddr),
                Err(WalletError::StoreCorrupt)
            ),
            "a wrong-network stored address is tamper — fail-closed"
        );
        assert!(
            matches!(
                transparent_receiver_from_stored_address(Network::Main, "garbage"),
                Err(WalletError::StoreCorrupt)
            ),
            "a non-address string is tamper — fail-closed"
        );
    }

    #[test]
    fn kat_bip39_vector_seed_derives_pinned_refund_address() {
        // Provenance anchor: the OFFICIAL BIP39 (Trezor #1) vector seed → a pinned refund
        // t-addr, tying the audited bip39 resolution AND the audited zcash_transparent BIP44
        // derivation end-to-end (a drift in EITHER fails this test).
        let payload = trezor_vector_seed();
        let a =
            derive_transparent_refund_address(Network::Main, payload.seed(), 0).expect("derive");
        assert_eq!(
            a, "t1RjHX7YceQXr413c87a4nqLt4awgjxwDbX",
            "refund t-addr from the BIP39 vector seed moved — bip39 or zcash_transparent drifted"
        );
        assert_transparent_refund(&a, Network::Main);
    }

    #[test]
    fn refund_address_is_deterministic() {
        // Same seed + index ⇒ identical address (HD-recoverable: a refund landing after a
        // wipe re-derives from seed + the persisted index, §4.5).
        for net in [Network::Main, Network::Test] {
            let a = derive_transparent_refund_address(net, KAT_SEED_32, 7).expect("a");
            let b = derive_transparent_refund_address(net, KAT_SEED_32, 7).expect("b");
            assert_eq!(a, b);
        }
    }

    #[test]
    fn refund_address_rejects_out_of_bounds_seed_typed() {
        // §4.2(b) panic guard (outer door): a seed outside 32..=252 would PANIC deeper in
        // the stack; both boundaries return a typed `InvalidSeedLength` (panic-across-FFI
        // impossible), consistent with the shielded derivations.
        for (bad, len) in [(vec![0u8; 31], 31usize), (vec![0u8; 253], 253usize)] {
            match derive_transparent_refund_address(Network::Main, &bad, 0) {
                Err(WalletError::InvalidSeedLength { len: got }) if got == len => {}
                other => panic!("expected typed InvalidSeedLength{{{len}}}, got {other:?}"),
            }
        }
    }

    #[test]
    fn refund_address_requires_bip32_master_seed_length() {
        // The BIP32 master seed is 16/32/64 bytes only (bip32 `ExtendedPrivateKey::new`),
        // narrower than the shielded path's 32..=252. The two production lengths (32 raw,
        // 64 BIP39) derive; an in-range-but-non-BIP32 length is typed `KeyDerivation`, NEVER
        // a panic and NEVER a wrong address — honest degradation for an exotic seed.
        assert!(derive_transparent_refund_address(Network::Main, &[0u8; 32], 0).is_ok());
        assert!(derive_transparent_refund_address(Network::Main, &[0u8; 64], 0).is_ok());
        for odd in [48usize, 100, 252] {
            match derive_transparent_refund_address(Network::Main, &vec![0u8; odd], 0) {
                Err(WalletError::KeyDerivation) => {}
                other => {
                    panic!("expected typed KeyDerivation for a {odd}-byte seed, got {other:?}")
                }
            }
        }
        // DRY anti-drift (W-swap-3-b): the `supports_transparent_refund` predicate the
        // refund source pre-checks with MUST agree with the actual derivation outcome —
        // it returns `true` exactly for the lengths that derive (32/64, and BIP32's 16)
        // and `false` for the exotic in-range lengths that yield `KeyDerivation`. So the
        // refund source can never fail-fast on a derivable seed nor burn the index space
        // on an underivable one.
        for derivable in [16usize, 32, 64] {
            assert!(
                supports_transparent_refund(derivable),
                "{derivable} is a BIP32 master-seed length"
            );
        }
        for exotic in [0usize, 1, 31, 48, 100, 252, 253] {
            assert!(
                !supports_transparent_refund(exotic),
                "{exotic} is NOT a BIP32 master-seed length"
            );
        }
        // the two wallet-real lengths the predicate accepts DO derive (the predicate is
        // not merely self-consistent — it tracks `derive_transparent_refund_address`):
        for len in [32usize, 64] {
            assert!(supports_transparent_refund(len));
            assert!(derive_transparent_refund_address(Network::Main, &vec![0u8; len], 0).is_ok());
        }
    }

    #[test]
    fn refund_address_rejects_hardened_index_typed() {
        // The ZIP-32 non-hardened child index space is [0, 2^31). The MAX valid index
        // derives; the first hardened value (2^31) is a typed reject, never a panic — the
        // allocator caps below this, but the door re-enforces the boundary for any caller.
        let max_valid = (1u32 << 31) - 1;
        assert!(derive_transparent_refund_address(Network::Main, KAT_SEED_32, max_valid).is_ok());
        match derive_transparent_refund_address(Network::Main, KAT_SEED_32, 1u32 << 31) {
            Err(WalletError::KeyDerivation) => {}
            other => panic!("expected typed KeyDerivation for a hardened index, got {other:?}"),
        }
    }

    #[test]
    fn refund_address_at_the_allocator_ceiling_is_a_valid_parseable_taddr() {
        // Real-world-edge (re-review): the allocator (`refund_index`) hands out indices up
        // to `NON_HARDENED_MAX`, then fails closed. PROVE that ceiling actually ties to a
        // DERIVABLE, parseable address — not just `.is_ok()` but the full transparent-P2PKH /
        // right-network / distinct-from-idx-0 check the low indices get. Ties the allocator's
        // exhaustion boundary to the derivation's boundary as ONE coherent fact, so a future
        // change to either can't silently let `reserve` hand out an index that won't derive.
        let ceiling = crate::refund_index::NON_HARDENED_MAX;
        for net in [Network::Main, Network::Test] {
            let at_ceiling = derive_transparent_refund_address(net, KAT_SEED_32, ceiling)
                .expect("ceiling derives");
            assert_transparent_refund(&at_ceiling, net);
            let at_zero = derive_transparent_refund_address(net, KAT_SEED_32, 0).expect("idx 0");
            assert_ne!(
                at_ceiling, at_zero,
                "the ceiling index must derive a DISTINCT address (no recycle at the top)"
            );
        }
    }

    // ── R03 — the 2026-09-20 production-readiness review ─────────────────────
    // `docs/plan/audit-2026-09-20-remediation.md` §2a/§2d: the body is the
    // review's probe (`docs/reviews/2026-09-20/probes/derivation.rs`),
    // verbatim; the name is this project's. Red by ruling; green
    // since stage S2 `recovery` NFKD-normalizes the passphrase before PBKDF2
    // (ADR-0561). No legacy-recovery path exists: it was dropped at the
    // contract's revision 3 under the maintainer's ruling. Appended at the
    // end of the module so no cited line above moves.

    /// R03 — `bip39 2.2.2`'s `to_seed_normalized` PBKDF2s the passphrase's
    /// bytes as given; the crate's `to_seed` normalizes, but into a `Cow` it
    /// drops un-zeroized, so `resolve_seed` normalizes itself into a
    /// `Zeroizing` buffer first. Two spellings of one passphrase — precomposed
    /// `é` and `e` + combining acute — are one BIP39 credential and recover
    /// one wallet.
    #[test]
    fn nfc_and_nfkd_spellings_of_a_passphrase_recover_the_same_wallet() {
        // Public BIP39 fixture, never a user wallet.
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let nfc = resolve_seed(SeedSource::mnemonic(
            phrase.to_owned(),
            Some("caf\u{e9}".to_owned()),
        ))
        .expect("NFC input");
        let nfkd = resolve_seed(SeedSource::mnemonic(
            phrase.to_owned(),
            Some("cafe\u{301}".to_owned()),
        ))
        .expect("NFKD input");
        let standard = Mnemonic::parse_in_normalized(Language::English, phrase)
            .unwrap()
            .to_seed("caf\u{e9}");
        assert_eq!(
            nfkd.seed(),
            &standard,
            "control: normalized SDK matches BIP39"
        );
        assert_eq!(
            nfc.seed(),
            &standard,
            "equivalent credentials must recover the same wallet"
        );
    }

    // S7 C3 — the phrase is written into one pre-sized `Zeroizing` buffer instead of
    // `to_string()`. Appended at the end of the module so no cited line above moves.

    /// KAT: [`phrase_of`] is byte-equal to the crate's own `to_string()` (and to the
    /// published phrase) for BIP39 reference vectors, 12 and 24 words (Trezor set).
    #[test]
    fn phrase_of_equals_to_string_for_the_bip39_vectors() {
        // Public BIP39 fixtures, never a user wallet.
        let vectors = [
            "legal winner thank year wave sausage worth useful legal winner thank yellow",
            "letter advice cage absurd amount doctor acoustic avoid letter advice cage above",
            "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo \
             zoo zoo zoo vote",
            "beyond stage sleep clip because twist token leaf atom beauty genius food \
             business side grid unable middle armed observe pair crouch tonight away coconut",
        ];
        for vector in vectors {
            let mnemonic = parse_mnemonic(vector).expect("a BIP39 vector parses");
            let phrase = phrase_of(&mnemonic);
            assert_eq!(
                phrase.as_str(),
                mnemonic.to_string(),
                "phrase_of == to_string"
            );
            assert_eq!(
                phrase.as_str(),
                vector,
                "the published vector, byte for byte"
            );
        }
    }

    /// The buffer never reallocates: on the longest phrase a 24-word mnemonic can
    /// spell (23 eight-letter words, the checksum word as long as the entropy allows)
    /// the capacity is still the pre-sized [`PHRASE_MAX_BYTES`] — a grown `String`
    /// would have freed an un-wiped prefix of the phrase.
    #[test]
    fn phrase_of_never_reallocates_on_the_longest_phrase() {
        let words = Language::English.word_list();
        let long = words
            .iter()
            .position(|w| w.len() == 8)
            .expect("an 8-letter word") as u16;
        let longest = (0u8..8)
            .map(|tail| {
                // 23 × the 11-bit index, then 3 free bits; the crate adds the checksum
                let mut bits = Vec::with_capacity(256);
                for _ in 0..23 {
                    bits.extend((0..11).rev().map(|k| (long >> k) & 1 == 1));
                }
                bits.extend((0..3).rev().map(|k| (tail >> k) & 1 == 1));
                let entropy: Vec<u8> = bits
                    .chunks(8)
                    .map(|byte| byte.iter().fold(0u8, |acc, &b| (acc << 1) | u8::from(b)))
                    .collect();
                Mnemonic::from_entropy_in(Language::English, &entropy).expect("256-bit entropy")
            })
            .max_by_key(|m| m.to_string().len())
            .expect("eight candidates");
        let phrase = phrase_of(&longest);
        assert!(
            phrase.len() >= 23 * 8 + 23 + 3,
            "the fixture is near the longest phrase: {} bytes",
            phrase.len()
        );
        assert_eq!(
            phrase.as_str(),
            longest.to_string(),
            "phrase_of == to_string"
        );
        assert_eq!(
            phrase.capacity(),
            PHRASE_MAX_BYTES,
            "the phrase buffer was never reallocated"
        );
    }
}
