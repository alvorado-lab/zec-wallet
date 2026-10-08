//! §3.2h inc-2d-2 — the bundled Sapling transaction prover (the create+sign path).
//!
//! The Sapling Groth16 proving parameters (~50 MB, the IMMUTABLE 2018 trusted-setup
//! artifact — it never changes, so there is no update lifecycle) are embedded in the
//! binary via `zcash_proofs/bundled-prover`. This is the maintainer's packaging decision
//! (§3.2h, 2026-06-20: BUNDLE) — a censorship-resistant wallet must not put a network
//! fetch of consensus-critical crypto on the money path, so bundling is the only option
//! with NO send-path failure mode (send works offline, first try, with no spinner). The
//! per-platform binary-size cost is accepted; a deferred-host-asset variant stays open
//! behind the injected-prover seam in `send::create_signed_core`. Orchard spends need no
//! external params (Halo2 has no trusted setup).
//!
//! Audited crate WHOLE (Rule Zero): we construct `LocalTxProver` and hand it to the
//! engine; we write no proving code.

use std::sync::OnceLock;

use zcash_proofs::prover::LocalTxProver;

/// The process-wide bundled Sapling prover.
///
/// Parsed ONCE on first use (the ~50 MB embedded params deserialize in ~1–2 s) via
/// [`OnceLock`] and shared for the process's life. **MUST be called inside a
/// `spawn_blocking` section** — the first call performs the heavy parse (CPU + ~50 MB
/// allocation), which must never run on the async runtime or the UI thread; later calls
/// return the cached reference instantly.
///
/// Resident after first use by design — signing needs it on every send, so re-parsing per
/// send would be wasteful. A future idle-drop (free the ~50 MB after N minutes of no
/// sends, re-parse on demand) is an optimization, not a correctness concern.
///
/// `LocalTxProver` implements both `sapling::prover::SpendProver` and `OutputProver`, so a
/// single value serves as both the spend and output prover for `create_proposed_transactions`.
pub(crate) fn tx_prover() -> &'static LocalTxProver {
    static PROVER: OnceLock<LocalTxProver> = OnceLock::new();
    PROVER.get_or_init(LocalTxProver::bundled)
}
