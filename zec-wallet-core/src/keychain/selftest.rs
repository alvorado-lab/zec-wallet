//! §4.3a device-E2E custody selftest — the runnable half of the
//! `wrap_key_roundtrip_via_keystore` named gate.
//!
//! Runs the FULL custody round-trip (seal → vault-wrap → unwrap → unseal →
//! binding rejection → wipe severs) through the REAL platform vault, under
//! **selftest-scoped vault identities** (a disjoint Keystore alias
//! namespace / keychain service) so production custody is never touched —
//! a host calling this in a loop can create and wipe only its own
//! selftest entries.
//!
//! The payload is FIXED TEST DATA, not a secret: nothing here ever sees a
//! real seed. The report carries booleans and the measured tier only —
//! nothing secret can cross any boundary above this.
//!
//! On Android this is ALSO the loader-namespace catch-point (§4.3a): a
//! split System.loadLibrary/dlopen mapping leaves the courier's init in
//! the other copy and this fails loudly with `VaultAbsent` — never
//! silently.

// The round-trip's imports and payload are compiled where the round-trip is:
// a vault platform. Elsewhere the selftest returns `VaultAbsent` and uses none.
#[cfg(any(target_os = "android", target_os = "macos", target_os = "ios"))]
use zeroize::Zeroizing;

#[cfg(any(target_os = "android", target_os = "macos", target_os = "ios"))]
use super::SealedSeedVault;
use super::VaultTier;
use crate::error::WalletError;
#[cfg(any(target_os = "android", target_os = "macos", target_os = "ios"))]
use crate::seal::SeedPayload;

/// Non-secret outcome of one custody round-trip (§4.3a device gate).
#[derive(Debug, Clone, Copy)]
pub struct CustodySelftestReport {
    /// Measured tier of the vault that custodied the selftest key.
    pub tier: VaultTier,
    /// The §4.3a degraded-custody predicate, surfaced as data.
    pub degraded: bool,
    /// store → load round-trip recovered the exact payload.
    pub roundtrip_ok: bool,
    /// A tampered blob was rejected at the WRAP layer (the AAD binding),
    /// before the seed envelope was ever consulted.
    pub binding_rejects_tampered_blob: bool,
    /// After wipe, the artifact opens nothing (and a second wipe is the
    /// idempotent no-op).
    pub wipe_severs: bool,
}

/// Fixed, non-secret selftest payload bytes.
#[cfg(any(target_os = "android", target_os = "macos", target_os = "ios"))]
const SELFTEST_SEED: [u8; 32] = [0xA5; 32];
#[cfg(any(target_os = "android", target_os = "macos", target_os = "ios"))]
const SELFTEST_MNEMONIC: &str = "custody selftest payload";

/// Run the §4.3a round-trip against this platform's real vault.
/// Fails CLOSED off-vault platforms (`VaultAbsent`) — the honest outcome,
/// not a skip.
///
/// DIAGNOSTIC, not a hardware-delete proof (FR-14 chunk 2): on the iOS **Simulator** the SE
/// is emulated in a host file, so this reports `tier: AppleSecureEnclave`,
/// `wipe_severs: true` WITHOUT a real hardware key delete. That the key store can no
/// longer open the blob after a purge is proven only by the real-hardware-gated `#[ignore]` KAT
/// (`apple_se_custody_provision_reload_then_purge_is_undecryptable`); treat a Simulator
/// report as non-authoritative for the hardware claim.
///
/// CONTRACT — no concurrent callers: `store_wrap_key` is not atomic with
/// respect to the generation scan, so two concurrent selftests race to
/// the same selftest alias and one reports a false FAIL (production
/// custody is untouched either way). Serialize invocations — at most one
/// per diagnostic session.
///
/// Cleanup: every reachable in-process path wipes (pass or fail). A
/// process kill between store and wipe leaves AT MOST ONE orphan
/// selftest alias per crash — the next run takes max+1 and ignores it;
/// orphans carry no secret material and are bounded.
pub fn seed_custody_selftest() -> Result<CustodySelftestReport, WalletError> {
    #[cfg(target_os = "android")]
    let backend = super::android::AndroidKeystoreVault::selftest();
    // Apple (FR-14 chunk 2): the default selftest exercises the SECURE-ENCLAVE custody
    // path — so building the example app on the connected iPhone runs the real SE-delete
    // KAT (provision → reload → purge → undecryptable) on SE hardware. The raw keychain
    // selftest is selected only with the explicit dev/sim fallback feature (a const `cfg!`,
    // matching `platform_vault`). Both backends are constructed (cheap string holders, no
    // I/O) so neither is dead code in either build; the unselected branch is optimized out.
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    let se_backend = super::apple::AppleSecureEnclaveVault::selftest();
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    let raw_backend = super::apple::AppleKeychainVault::selftest();
    #[cfg(not(any(target_os = "android", target_os = "macos", target_os = "ios")))]
    return Err(WalletError::VaultAbsent);

    #[cfg(any(target_os = "android", target_os = "macos", target_os = "ios"))]
    {
        #[cfg(target_os = "android")]
        let backend: std::sync::Arc<dyn super::KeychainPort> = std::sync::Arc::new(backend);
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        let backend: std::sync::Arc<dyn super::KeychainPort> =
            if cfg!(feature = "apple-insecure-raw-keychain-fallback") {
                std::sync::Arc::new(raw_backend)
            } else {
                std::sync::Arc::new(se_backend)
            };
        // FR-47 (S9 diff review, MEDIUM): this diagnostic is host-callable
        // (`selftest_seed_custody` over the FFI), so a wedged key store must
        // not hang it either — it runs on the same process worker, under the
        // same per-call bound, as every production key-store call.
        // No namespace for the tombstone check: a selftest namespace is never
        // one a duress sever names, so its creating calls are never refused.
        let bounded = super::bounded::BoundedVault::new(backend, None);
        let vault = SealedSeedVault::new(&bounded);
        let payload = SeedPayload::new(
            Zeroizing::new(SELFTEST_SEED.to_vec()),
            Some(Zeroizing::new(SELFTEST_MNEMONIC.to_owned())),
        )?;

        let stored = vault.store(&payload)?;

        // Everything after the store cleans up, pass or fail.
        let run = || -> Result<(bool, bool), WalletError> {
            let (loaded, _status) = vault.load(&stored.sealed_blob, &stored.wrap_artifact)?;
            let roundtrip_ok =
                loaded.seed() == SELFTEST_SEED && loaded.mnemonic() == Some(SELFTEST_MNEMONIC);

            let mut tampered = stored.sealed_blob.clone();
            let last = tampered.len() - 1;
            tampered[last] ^= 0x01;
            let binding_rejects_tampered_blob = matches!(
                vault.load(&tampered, &stored.wrap_artifact),
                Err(WalletError::WrapArtifactInvalid)
            );
            Ok((roundtrip_ok, binding_rejects_tampered_blob))
        };
        let checks = run();
        let status = stored.status;

        // Wipe ALWAYS runs (cleanup is part of the contract), and is
        // itself under test: severed artifact opens nothing, second wipe
        // is the idempotent no-op.
        let wipe_severs = vault.wipe(&stored.wrap_artifact).is_ok()
            && matches!(
                vault.load(&stored.sealed_blob, &stored.wrap_artifact),
                Err(WalletError::KeystoreInconsistent { .. })
            )
            && vault.wipe(&stored.wrap_artifact).is_ok();

        let (roundtrip_ok, binding_rejects_tampered_blob) = checks?;
        Ok(CustodySelftestReport {
            tier: status.tier,
            degraded: status.degraded(),
            roundtrip_ok,
            binding_rejects_tampered_blob,
            wipe_severs,
        })
    }
}
