//! Seed input boundary (spec §2.2) — the ONE door key material enters
//! through. No `Clone`/`Debug`/`Display`/`Serialize` (crypto-rules);
//! constructors take ownership and the contents zeroize on drop.

use zeroize::Zeroizing;

use crate::constants::{SEED_MAX_BYTES, SEED_MIN_BYTES};
use crate::error::WalletError;

/// Seed input. The variant set is the whole consumer story (§3.4):
/// `Generate`/`Mnemonic` are the BIP39 paths a typical app uses; `RawBytes`
/// is the host-derived-seed path — a host derives its own sub-seed under its
/// own KDF and passes the opaque bytes in, in Rust (e.g. a host feeds a 32-byte
/// sub-seed derived via a host label such as `relim/wallet/zip32-seed/v1`, which
/// lives in the host's identity crate and is deliberately ABSENT from this SDK —
/// we see only opaque bytes).
#[non_exhaustive]
pub enum SeedSource {
    /// SDK generates 24 words via OsRng (third-party create flow).
    Generate,
    /// Third-party restore: BIP39 mnemonic + optional BIP39 passphrase.
    /// Checksum-validated before use (at derivation, where the word list is
    /// in hand); errors carry the word INDEX only, never the word.
    Mnemonic {
        phrase: Zeroizing<String>,
        passphrase: Zeroizing<String>,
    },
    /// Raw seed bytes, 32..=252 — BOTH bounds enforced at construction with a
    /// typed error (verified: upstream `from_seed` PANICS under 32 bytes and
    /// ignores the ZIP-32 upper bound; a panic across FFI is a crash). Never
    /// truncated. A raw-bytes wallet stores NO mnemonic → `reveal_mnemonic`
    /// is structurally `NoMnemonic` (§3.1). RESTORE-shaped: create assumes
    /// possible prior chain history (activation-floor lazy import, the §3.2h
    /// restore-pessimistic refund seeding) — the safe default for any seed
    /// whose provenance is unknown.
    RawBytes(Zeroizing<Vec<u8>>),
    /// Raw seed bytes the host ATTESTS were minted for the first time
    /// immediately before this call and have NEVER existed anywhere else
    /// (FR-24 fresh-flag, §3.2f — the T2 host-custody sibling of
    /// [`Generate`](Self::Generate)). Same 32..=252 door and no-mnemonic
    /// contract as [`RawBytes`](Self::RawBytes); the create additionally
    /// takes every freshly-generated behavior `Generate` takes — the #356-F4
    /// creation stamp, the eager offline account import, the fresh birthday
    /// default, the loud eager-failure posture. Repair stays supplied-shaped
    /// (`VerifySupplied`/`NoSeed`, never `ResumeSealed` — the caller CAN
    /// re-supply these bytes).
    ///
    /// **THE ATTESTATION IS LOAD-BEARING FOR MONEY VISIBILITY (§3.2f
    /// lying-host contract):** marking a seed with prior chain history fresh
    /// floors the birthday at the creation stamp (pre-stamp funds invisible;
    /// the stamp SURVIVES rescan and re-floors `rescan(None)` at itself),
    /// suppresses the restore-pessimistic refund-index sweep, AND (Recv-4 /
    /// ADR-0537) suppresses the diversified-receive counter's pessimistic
    /// seed — a falsely-attested seed with a prior minting life re-issues its
    /// earliest diversified receive UAs to NEW contacts (cross-contact
    /// linkage, never a fund loss). When in doubt use
    /// [`RawBytes`](Self::RawBytes) — the only cost is a full activation
    /// scan. Constructor [`raw_bytes_fresh`](Self::raw_bytes_fresh).
    FreshRawBytes(Zeroizing<Vec<u8>>),
}

impl SeedSource {
    pub fn generate() -> Self {
        Self::Generate
    }

    /// Takes ownership; the words live in zeroizing buffers from here on.
    /// (Dart-side residue of the inbound copy is the documented sanctioned
    /// exposure, §3.3.)
    pub fn mnemonic(phrase: String, passphrase: Option<String>) -> Self {
        Self::Mnemonic {
            phrase: Zeroizing::new(phrase),
            passphrase: Zeroizing::new(passphrase.unwrap_or_default()),
        }
    }

    /// The ONE raw-bytes bounds door (§2.2) both constructors share — 32..=252
    /// enforced HERE with a typed error; the input is owned and zeroized even
    /// on the reject path. Private so the validation can never fork.
    fn bounded(bytes: Vec<u8>) -> Result<Zeroizing<Vec<u8>>, WalletError> {
        let bytes = Zeroizing::new(bytes); // zeroizes on early return too
        let len = bytes.len();
        if !(SEED_MIN_BYTES..=SEED_MAX_BYTES).contains(&len) {
            return Err(WalletError::InvalidSeedLength { len });
        }
        Ok(bytes)
    }

    /// Enforces 32..=252 (§2.2, [`bounded`](Self::bounded)); zeroizes on
    /// reject. RESTORE-shaped — see the variant doc.
    pub fn raw_bytes(bytes: Vec<u8>) -> Result<Self, WalletError> {
        Ok(Self::RawBytes(Self::bounded(bytes)?))
    }

    /// [`raw_bytes`](Self::raw_bytes) + the FRESHLY-GENERATED attestation
    /// (FR-24, §3.2f) — same bounds door, same zeroize-on-reject. Callers MUST
    /// hold the [`FreshRawBytes`](Self::FreshRawBytes) contract: the bytes
    /// were minted immediately before this call and never existed anywhere
    /// else; a mis-attested restore hides pre-stamp funds (the lying-host
    /// contract on the variant).
    pub fn raw_bytes_fresh(bytes: Vec<u8>) -> Result<Self, WalletError> {
        Ok(Self::FreshRawBytes(Self::bounded(bytes)?))
    }

    /// Was this seed minted fresh at (or immediately before) this create —
    /// `Generate` or host-attested [`FreshRawBytes`](Self::FreshRawBytes)?
    /// THE predicate for every freshly-generated create behavior (§3.2f): the
    /// #356-F4 creation stamp, the eager offline import arm, the loud
    /// eager-failure posture, and the handle's in-session `freshly_generated`.
    /// Deliberately NOT the repair-mode key — that stays `Generate`-only
    /// (`ResumeSealed` exists because Generate entropy cannot be re-supplied).
    pub fn is_freshly_generated(&self) -> bool {
        matches!(self, Self::Generate | Self::FreshRawBytes(_))
    }
}

/// Seed-at-rest mode (§4.2) — the deliberate divergence from upstream's
/// app-holds-the-seed contract (which, in Flutter, would put the seed in
/// Dart).
#[non_exhaustive]
pub enum SeedPersistence {
    /// SDK seals seed + mnemonic under a keychain-random wrap key (§4.2a
    /// pinned envelope). Default for Dart-path consumers: enables
    /// `reveal_mnemonic` after restart and internal `SeedRequired` handling.
    SealedKeychain,
    /// Nothing persisted (the host-supplied-seed path): `SeedRequired` bubbles to the
    /// host, which re-derives via [`WalletSeedPort`] (cheap, Rust-side).
    None,
}

/// Host-custodied seed supply (FR-12 / W5 inc-2c-iv) — the seam a
/// [`SeedPersistence::None`] wallet pulls the seed through, ON DEMAND, for the
/// few operations that genuinely need it. Lets the wallet hold NO seed at rest
/// yet still spend when the host supplies it for exactly one operation, then
/// zeroizes.
///
/// This is standard **view-only / host-custodied-key** wallet support, NOT a
/// host-specific concept: the wallet opens from the persisted account UFVK, so balance,
/// history, and the receive address need NO seed. Only key-DERIVING operations
/// call the port: deriving + persisting the account from the seed (so the UFVK
/// lands in the DB and every later open is seed-free), transaction signing, and
/// fresh-refund-address derivation. The returned bytes are validated against the
/// [`SeedSource::raw_bytes`] 32..=252 contract at the SDK boundary, used inside
/// ONE synchronous blocking section, and zeroize on drop — the SDK never persists
/// them and never holds them past the operation.
///
/// MONEY-SAFETY (SDK-enforced, not host-trusted): before signing, the SDK
/// verifies the supplied seed derives the wallet's STORED account UFVK and
/// rejects a mismatch with a typed [`WalletError::SeedMismatch`] — a wrong seed
/// can never produce a transaction. The seed is pulled + verified BEFORE the
/// one-shot send token is consumed (and before a refund HD-index is reserved), so
/// any failure here leaves the operation fully retryable.
///
/// CONTRACT — the host stages, the SDK pulls; `provide_seed` MUST:
/// - **Be O(1), non-blocking, NON-RE-ENTRANT, and acquire NO wallet lock.** It
///   returns ALREADY-staged bytes; it must not prompt, do I/O, sleep, await, or
///   call back into the `Wallet`. It is invoked from a blocking proving thread
///   and just before the SDK takes a non-reentrant DB lock — a blocking or
///   re-entrant impl parks a pool thread or self-deadlocks.
/// - **Stage narrowly + clear immediately.** Stage the seed right before the SDK
///   call that pulls it, and clear the staged slot the moment that call returns.
///   The **send scope is take-once: one credential confirmation = exactly one
///   signature** (a second pull against a send-stage must return `Unavailable`).
///   NEVER hold the seed across the offline create→first-sync gap (that would put
///   it at rest in host memory, defeating the whole mode) — if a fresh wallet's
///   account import must wait for first sync, re-stage then via a fresh credential
///   gate. (Provisioning/import MAY pull more than once within one user-present
///   moment; the SDK minimizes pulls to one per confined op.)
/// - **Never log the returned bytes** (§5.4 — the SDK cannot enforce the host's
///   impl; the bytes are `Zeroizing`, no `Debug`, so this is a discipline, not a
///   leak in the type).
/// - Return `Err(SeedSupplyError::Unavailable)` when nothing is staged — the SDK
///   surfaces [`WalletError::SeedRequired`] so the host drives its prompt + retry.
///   Return `Err(SeedSupplyError::Denied)` on a deliberate refusal; the SDK still
///   surfaces `SeedRequired` (the two are indistinguishable to the caller — no
///   refusal-vs-unavailable oracle), but the variant lets the host's own impl
///   branch its intent.
///
/// The bytes are the opaque 32-byte sub-seed (the same material `raw_bytes`
/// takes); the SDK does NOT know the host's derivation. `Send + Sync` — the `Arc`
/// is shared across the blocking pool. SYNC by design: signing runs in one
/// `spawn_blocking` section and the §4.2 seed→USK→sign→drop confinement must stay
/// inside it, which an `async` port would break.
pub trait WalletSeedPort: Send + Sync {
    /// Hand the SDK the seed for ONE operation. See the trait-level contract.
    ///
    /// `binding` (FR-17) is the [`SpendBinding`] of the EXACT proposal/row the SDK
    /// is about to sign — `Some` on every spend-signing pull (interactive send,
    /// queued drain, swap deposit; `None` only for legacy pre-FR-17 queued rows),
    /// `None` on the non-recipient pulls (account import, ephemeral sweep, reclaim
    /// self-mint, refund-address derivation). A host that records the binding it
    /// authorized MUST compare with strict `Option` equality (unbound stage matches
    /// ONLY an unbound pull) and return `Unavailable` on any mismatch — that
    /// fail-close is the whole FR-17 property; the SDK presents honestly but cannot
    /// verify for the host.
    fn provide_seed(
        &self,
        binding: Option<&SpendBinding>,
    ) -> Result<Zeroizing<Vec<u8>>, SeedSupplyError>;
}

/// FR-17 spend-binding nonce length. Fixed-size so the C-ABI transport needs no
/// length negotiation and a host can embed it in a fixed slot.
pub const SPEND_BINDING_BYTES: usize = 32;

/// FR-17 — an SDK-minted, opaque, random binding token carried from the review
/// surface to the seed pull. Minted (`OsRng`) at interactive propose, at queue
/// enqueue, and at swap-quote issue; presented verbatim on the matching sign
/// pull ([`WalletSeedPort::provide_seed`]). The SDK never interprets it — its
/// only power is to make a review↔sign mismatch DETECTABLE by the host, which
/// fail-closes the supply. NOT key material: a pure nonce, safe to cross Dart
/// on display DTOs (a hostile in-process actor gains nothing from reading it —
/// substituting it can only cause a refusal, never a wrong signature).
///
/// `Debug` is redacted (§5.4): a logged binding would correlate a proposal with
/// its sign moment across log lines.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SpendBinding([u8; SPEND_BINDING_BYTES]);

impl SpendBinding {
    /// Mint a fresh random binding (CSPRNG — the `ephemeral_detect` isolation-key
    /// precedent).
    pub fn mint() -> Self {
        let mut bytes = [0u8; SPEND_BINDING_BYTES];
        rand_core::RngCore::fill_bytes(&mut rand_core::OsRng, &mut bytes);
        Self(bytes)
    }

    /// Reconstruct from persisted/DTO bytes; `None` unless EXACTLY
    /// [`SPEND_BINDING_BYTES`] (validate-never-truncate — a wrong-length blob is
    /// treated as absent, which downstream presents as an unbound pull the host
    /// fail-closes, never as a silently padded token).
    pub fn from_slice(bytes: &[u8]) -> Option<Self> {
        <[u8; SPEND_BINDING_BYTES]>::try_from(bytes).ok().map(Self)
    }

    pub fn as_bytes(&self) -> &[u8; SPEND_BINDING_BYTES] {
        &self.0
    }
}

impl core::fmt::Debug for SpendBinding {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("SpendBinding(..)")
    }
}

/// Why a [`WalletSeedPort`] could not supply the seed. The SDK collapses BOTH
/// arms to [`WalletError::SeedRequired`] for the caller (the host re-supplies) —
/// so the two are indistinguishable downstream, leaving no refusal-vs-unavailable
/// oracle. The variant only lets the host's own impl branch its intent.
/// `#[non_exhaustive]` — new reasons may land without breaking host matches.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SeedSupplyError {
    /// No seed is staged right now — the host should run its credential gate +
    /// re-derive, stage the seed, and the operation is retried.
    #[error("seed not staged")]
    Unavailable,
    /// The host deliberately refused (the user declined, or the host's own policy
    /// yields no seed in this context). Not a transient condition.
    #[error("seed supply denied")]
    Denied,
}

/// Shared crate-test doubles for [`WalletSeedPort`] (one source of truth — the
/// mirror of `ports::testing`). The real host port is platform credential code;
/// these stand in for "the host has staged a seed" / "nothing is staged" / "the
/// host refused", so the wiring tests never need a live keystore.
#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A host that has STAGED a seed: every pull returns it (models the
    /// provisioning/import moment, and a single confined sign). Counts pulls so a
    /// test can assert the SDK pulls EXACTLY as many times as the op needs and
    /// never holds the result past it.
    pub(crate) struct StagedSeedPort {
        seed: Zeroizing<Vec<u8>>,
        pulls: AtomicUsize,
        /// One entry per pull: the binding the SDK presented (FR-17 spy).
        bindings: Mutex<Vec<Option<SpendBinding>>>,
    }

    impl StagedSeedPort {
        pub(crate) fn new(seed: Vec<u8>) -> Self {
            Self {
                seed: Zeroizing::new(seed),
                pulls: AtomicUsize::new(0),
                bindings: Mutex::new(Vec::new()),
            }
        }

        /// How many times the SDK has pulled the seed (the confinement witness).
        pub(crate) fn pulls(&self) -> usize {
            self.pulls.load(Ordering::SeqCst)
        }

        /// Every binding the SDK presented, one entry per pull, in order (FR-17
        /// spy — `None` entries are unbound pulls). Lets a test assert WHICH
        /// binding a given operation presented, not just that a pull happened.
        pub(crate) fn presented_bindings(&self) -> Vec<Option<SpendBinding>> {
            self.bindings.lock().expect("bindings spy").clone()
        }
    }

    impl WalletSeedPort for StagedSeedPort {
        fn provide_seed(
            &self,
            binding: Option<&SpendBinding>,
        ) -> Result<Zeroizing<Vec<u8>>, SeedSupplyError> {
            self.pulls.fetch_add(1, Ordering::SeqCst);
            self.bindings
                .lock()
                .expect("bindings spy")
                .push(binding.copied());
            Ok(Zeroizing::new(self.seed.to_vec()))
        }
    }

    /// FR-17 acceptance double — a host that STAGED for exactly one binding and
    /// fail-closes every pull that presents anything else (the strict-`Option`-
    /// equality compare the trait doc requires of real hosts).
    pub(crate) struct BoundSeedPort {
        seed: Zeroizing<Vec<u8>>,
        staged_for: Option<SpendBinding>,
        pulls: AtomicUsize,
    }

    impl BoundSeedPort {
        pub(crate) fn staged_for(seed: Vec<u8>, binding: Option<SpendBinding>) -> Self {
            Self {
                seed: Zeroizing::new(seed),
                staged_for: binding,
                pulls: AtomicUsize::new(0),
            }
        }

        pub(crate) fn pulls(&self) -> usize {
            self.pulls.load(Ordering::SeqCst)
        }
    }

    impl WalletSeedPort for BoundSeedPort {
        fn provide_seed(
            &self,
            binding: Option<&SpendBinding>,
        ) -> Result<Zeroizing<Vec<u8>>, SeedSupplyError> {
            self.pulls.fetch_add(1, Ordering::SeqCst);
            if binding.copied() != self.staged_for {
                return Err(SeedSupplyError::Unavailable);
            }
            Ok(Zeroizing::new(self.seed.to_vec()))
        }
    }

    /// A host with NOTHING staged — every pull is `Unavailable`. The SDK surfaces
    /// `SeedRequired` and the operation stays fully retryable (no token burned, no
    /// index reserved).
    pub(crate) struct UnavailableSeedPort;

    impl WalletSeedPort for UnavailableSeedPort {
        fn provide_seed(
            &self,
            _binding: Option<&SpendBinding>,
        ) -> Result<Zeroizing<Vec<u8>>, SeedSupplyError> {
            Err(SeedSupplyError::Unavailable)
        }
    }

    /// A host that DELIBERATELY refuses (duress / policy) — `Denied`. The SDK still
    /// surfaces `SeedRequired` (the two collapse downstream — no oracle).
    pub(crate) struct DeniedSeedPort;

    impl WalletSeedPort for DeniedSeedPort {
        fn provide_seed(
            &self,
            _binding: Option<&SpendBinding>,
        ) -> Result<Zeroizing<Vec<u8>>, SeedSupplyError> {
            Err(SeedSupplyError::Denied)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Crypto-rules: key-material types expose no Clone/Debug/Display/Serialize.
    // Compile-time pins — a derive added later fails HERE, not in review.
    static_assertions::assert_not_impl_any!(
        SeedSource: Clone, std::fmt::Debug, std::fmt::Display
    );
    static_assertions::assert_not_impl_any!(
        SeedPersistence: Clone, std::fmt::Debug, std::fmt::Display
    );

    #[test]
    fn seed_length_bounds_enforced_before_upstream_no_panic() {
        // §8 named test (the construction half): upstream panics <32B and
        // accepts >252B; OUR boundary rejects both, typed, before any
        // upstream call can exist.
        for len in [0usize, 1, 31] {
            match SeedSource::raw_bytes(vec![0xAA; len]) {
                Err(WalletError::InvalidSeedLength { len: l }) => assert_eq!(l, len),
                _ => panic!("under-length seed must be a typed reject"),
            }
        }
        for len in [253usize, 4096] {
            assert!(matches!(
                SeedSource::raw_bytes(vec![0xAA; len]),
                Err(WalletError::InvalidSeedLength { .. })
            ));
        }
        for len in [32usize, 64, 252] {
            assert!(SeedSource::raw_bytes(vec![0xAA; len]).is_ok());
        }
    }

    #[test]
    fn raw_bytes_fresh_rides_the_same_bounds_door() {
        // FR-24: the fresh constructor is raw_bytes + the attestation —
        // NEVER a second, divergable validation path. Same typed rejects,
        // same acceptance window, and the accepted value is the fresh variant.
        for len in [0usize, 31, 253] {
            assert!(matches!(
                SeedSource::raw_bytes_fresh(vec![0xAA; len]),
                Err(WalletError::InvalidSeedLength { .. })
            ));
        }
        for len in [32usize, 252] {
            match SeedSource::raw_bytes_fresh(vec![0xAA; len]) {
                Ok(SeedSource::FreshRawBytes(_)) => {}
                _ => panic!("in-bounds fresh bytes must construct FreshRawBytes"),
            }
        }
    }

    #[test]
    fn is_freshly_generated_covers_exactly_generate_and_fresh_raw_bytes() {
        // The §3.2f predicate: Generate ∨ FreshRawBytes — a new variant
        // landing in either bucket silently is a money-behavior change, so
        // pin all four current shapes.
        assert!(SeedSource::generate().is_freshly_generated());
        assert!(
            SeedSource::raw_bytes_fresh(vec![0xAA; 32])
                .expect("valid")
                .is_freshly_generated()
        );
        assert!(
            !SeedSource::raw_bytes(vec![0xAA; 32])
                .expect("valid")
                .is_freshly_generated()
        );
        assert!(
            !SeedSource::mnemonic("abandon ".repeat(24).trim_end().to_owned(), None)
                .is_freshly_generated(),
            "a restore-shaped mnemonic is never fresh"
        );
    }
}
