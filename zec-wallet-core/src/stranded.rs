//! §3.2i-2 2e-2b — the STRANDED-ephemeral SURFACE (the READ half of 2e-2b-ii). The DETECT
//! ([`crate::ephemeral_detect`]) is the money-safety GUARANTEE that RECOGNISES funds a TEX `tx0`
//! unshielded to a wallet-controlled EPHEMERAL transparent address — then never forwarded (tx1
//! expired → STRANDED), or that an exchange RETURNED to that one-time address. This module is the
//! user-facing READ that surfaces the recoverable AMOUNT ([`StrandedAmount`]), plus the
//! terminal-`Stranded` intent-row REAP policy ([`should_reap`]). See ADR-0535.
//!
//! ## The decoupling invariant (the money-safety keystone — ADR-0535 Decision 2)
//! The terminal `Stranded` intent ROW is DECOUPLED from money-visibility. The detect enumerates
//! ephemerals from the ENGINE (`get_ephemeral_transparent_receivers`), and the recoverable amount is
//! read from the ENGINE balance (`get_transparent_balances`, [`crate::account::ephemeral_recoverable_balances`]),
//! NEVER from intent rows. So a `Stranded` row's existence has NO bearing on whether funds are
//! detected or surfaced — the row only (i) stopped the doomed re-broadcast (via its STATE, at
//! `mark_stranded`) and (ii) is an audit artifact. **Consequence:** reaping it ([`should_reap`]) can
//! never un-surface recoverable funds — the surface is the engine balance, not the row.
//!
//! ## §5.4
//! [`StrandedAmount`] carries AMOUNTS (NEVER-LOG — returned to the host to render, never put on a
//! span; only the COUNT of stranded ephemerals is spannable) and DELIBERATELY no address (the
//! ephemeral is NEVER-RENDER, not only never-LOG). [`EphemeralRecoverable`] carries the address for
//! the SDK-internal skip-funded check ONLY; it is dropped before the surface leaves the core.

use crate::constants::REORG_MAX_BLOCKS;
use zcash_transparent::address::TransparentAddress;

/// One ephemeral's RECOGNISED recoverable balance, read from the engine
/// (`get_transparent_balances` filtered to the EPHEMERAL key scope — the strand / exchange-return
/// cases the `excluding_wallet_internal_ephemeral_outputs` 3-way-OR surfaces, ADR-0535). Carries the
/// ADDRESS only for the SDK-internal skip-funded check ([`crate::wallet::Wallet::detect_ephemeral_utxos`]);
/// it is §5.4 NEVER-RENDER and is dropped before the [`StrandedAmount`] surface leaves the core.
pub(crate) struct EphemeralRecoverable {
    pub(crate) address: TransparentAddress,
    /// The ECONOMICALLY-recoverable amount on this ephemeral (`Balance::total` — spendable +
    /// pending-confirmation; uneconomic dust ≤ the marginal fee is EXCLUDED by the engine, by design).
    /// See [`crate::account::ephemeral_recoverable_balances`].
    pub(crate) recoverable_zat: u64,
}

/// The user-facing recoverable amount on ONE stranded/returned ephemeral, with the reorg-finality
/// gate WELDED IN (2e-2b-iii) — AMOUNT + finality ONLY (the ephemeral address is §5.4 never-RENDER, so
/// it is absent here by construction). `is_final` is computed CORE-side from per-output depth (the
/// engine finding: `get_transparent_balances` does NOT split a transparent balance by
/// confirmation depth, so finality cannot come from the amount read — see
/// [`crate::account::ephemeral_recoverable_finality`]). v1 surfaces amount + finality; spend-from-
/// ephemeral (auto-reshield) stays deferred. §5.4: NEVER-LOG — this type's `Debug` render carries no
/// FORBIDDEN token (only `recoverable_zat` + a bool), so the `tracing_guard` value scan would not
/// self-trip on it; the allowlist gate (a new field name forces review) is the guard-enforced defense,
/// and the reader emits no span (see [`crate::wallet::Wallet::list_stranded`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StrandedAmount {
    /// The ECONOMICALLY-recoverable amount on this ephemeral (show the user; uneconomic dust is
    /// excluded by the engine read — never hide ECONOMIC funds by depth). A SUBSET of the user's
    /// DISPLAYED balance (the same engine predicate folds into the wallet summary), so the host
    /// presents it as part of the balance, NEVER as additional funds (ADR-0535 Consequence).
    pub recoverable_zat: u64,
    /// `true` iff the WHOLE `recoverable_zat` is buried beyond [`REORG_MAX_BLOCKS`] (reorg-final). A
    /// shallow/pending portion drops this to `false` — the host must then render the amount as still
    /// confirming, never settled/ready. CONSERVATIVE: never claims settled before the full amount is
    /// irreversible.
    pub is_final: bool,
}

impl StrandedAmount {
    /// Weld the surface from the two engine reads ([`crate::account::ephemeral_recoverable_finality`]):
    /// `recoverable_zat` is the economic total (`Balance::total`), `buried_zat` the value of its UTXOs
    /// already buried beyond [`REORG_MAX_BLOCKS`]. `is_final` only when EVERY economic zatoshi is buried
    /// (`buried_zat == recoverable_zat`) — so a host can never render a shallow/reorgable amount as
    /// settled. The `buried_zat <= recoverable_zat` bound is an engine invariant (both reads share the
    /// `excluding_wallet_internal_ephemeral_outputs` predicate + the dust floor); we still gate on
    /// `recoverable_zat > 0` so a zero surface is never "final".
    pub(crate) fn welded(recoverable_zat: u64, buried_zat: u64) -> Self {
        Self {
            recoverable_zat,
            is_final: recoverable_zat > 0 && buried_zat == recoverable_zat,
        }
    }
}

/// The codebase `buried` SSOT (pure): an on-chain output mined at `mined` is IRREVERSIBLY owned by the
/// chain once the tip is at least [`REORG_MAX_BLOCKS`] beyond it — the deepest reorg the wallet
/// auto-recovers. Inclusive `>=` (the same boundary `send.rs`'s delete-on-mined / mark-stranded use).
/// Saturates (a `mined` above `tip` — a scan/clock transient — reads not-buried, never an underflow).
pub(crate) fn buried(mined: u32, tip: u32) -> bool {
    tip.saturating_sub(mined) >= REORG_MAX_BLOCKS
}

/// The host-facing depth-finality SSOT (pure, 2e-2b-iii) the FFI bridge re-exports so the host NEVER
/// hard-codes the reorg horizon for the `transactions()` finality badge. A confirmed tx at
/// `confirmation_depth` (`= tip − mined + 1`) is reorg-FINAL once `depth > REORG_MAX_BLOCKS` — NOT
/// `>=`: `depth == REORG_MAX_BLOCKS` is still one block reorg-reachable (`tip − mined == MAX − 1`), one
/// block short of [`buried`] (the send.rs HOST NOTE pins this off-by-one). Operates on a tx depth, NOT
/// a stranded amount — distinct from the WELDED [`StrandedAmount::is_final`].
pub fn confirmation_depth_is_final(confirmation_depth: u32) -> bool {
    confirmation_depth > REORG_MAX_BLOCKS
}

/// The terminal-`Stranded` intent-row REAP predicate (pure, §3.2i-2 / ADR-0535 Decision 3): reap a
/// `Stranded` row once its tx0 is [`buried`] beyond [`REORG_MAX_BLOCKS`] — the SAME `buried` predicate
/// the delete-on-mined / mark-stranded decisions use (`send.rs`), so the trigger is a DURABLE
/// on-chain fact, NEVER a poll result (a censoring endpoint's lying-empty can never reach it — the
/// §3.2i-2 reap-decoupling rule). Because `mark_stranded`'s own precondition is an already-buried
/// tx0, a real strand reaps on the next pass; the amount stays visible via the engine balance
/// regardless (the decoupling invariant, module docs). `tx0_mined = None` (tx0 not on-chain —
/// unreachable for a real strand) is FAIL-SAFE: NOT reaped, so a row is never dropped before its
/// funding is irreversible.
pub(crate) fn should_reap(tx0_mined: Option<u32>, tip: u32) -> bool {
    match tx0_mined {
        Some(mined) => buried(mined, tip),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn welded_is_final_only_when_the_whole_amount_is_buried() {
        // The surface is AMOUNT + finality ONLY (no address — never-RENDER, structural). is_final is
        // CONSERVATIVE: it flips true ONLY when every economic zatoshi is buried.
        // Whole amount buried ⇒ final.
        let full = StrandedAmount::welded(40_000, 40_000);
        assert_eq!(full.recoverable_zat, 40_000);
        assert!(full.is_final, "buried_zat == recoverable_zat ⇒ reorg-final");
        // A shallow/pending portion (buried_zat < recoverable_zat) ⇒ NOT final — the host must show it
        // confirming, never settled (a second deposit still maturing, or a fresh strand/return).
        assert!(
            !StrandedAmount::welded(40_000, 39_999).is_final,
            "one zatoshi short of fully buried ⇒ not final"
        );
        assert!(
            !StrandedAmount::welded(15_000, 0).is_final,
            "nothing buried yet ⇒ not final (the shallow case)"
        );
    }

    #[test]
    fn welded_zero_recoverable_is_never_final() {
        // A zero surface (never constructed in practice — the reader skips zero) is not "final"; the
        // `recoverable_zat > 0` guard keeps `0 == 0` from reading as settled.
        assert!(!StrandedAmount::welded(0, 0).is_final);
    }

    #[test]
    fn confirmation_depth_is_final_is_strictly_beyond_reorg_max() {
        // The host depth-finality SSOT: `depth > REORG_MAX_BLOCKS`, NOT `>=`. `depth == REORG_MAX_BLOCKS`
        // is `tip − mined == MAX − 1` — one block reorg-reachable, NOT yet final (the send.rs HOST NOTE
        // off-by-one). One deeper ⇒ final.
        assert!(!confirmation_depth_is_final(REORG_MAX_BLOCKS));
        assert!(confirmation_depth_is_final(REORG_MAX_BLOCKS + 1));
        assert!(!confirmation_depth_is_final(1));
    }

    #[test]
    fn buried_boundary_is_inclusive_and_saturating() {
        let tip = 2_000_000u32;
        // Exactly REORG_MAX_BLOCKS deep ⇒ buried (inclusive `>=`, matching delete-on-mined).
        assert!(buried(tip - REORG_MAX_BLOCKS, tip));
        // One block shallower ⇒ not buried.
        assert!(!buried(tip - REORG_MAX_BLOCKS + 1, tip));
        // A mined height ABOVE the tip (scan/clock transient) saturates to 0 depth ⇒ not buried.
        assert!(!buried(100, 50));
    }

    #[test]
    fn should_reap_only_once_tx0_is_buried_beyond_reorg_max() {
        let tip = 2_000_000u32;
        // Buried EXACTLY at the threshold (tip − mined == REORG_MAX_BLOCKS) ⇒ reap (inclusive `>=`,
        // matching the codebase `buried`).
        assert!(should_reap(Some(tip - REORG_MAX_BLOCKS), tip));
        // One block shallower ⇒ NOT yet (still reorg-reversible).
        assert!(!should_reap(Some(tip - REORG_MAX_BLOCKS + 1), tip));
        // Freshly mined at the tip ⇒ not reaped.
        assert!(!should_reap(Some(tip), tip));
    }

    #[test]
    fn should_reap_is_fail_safe_when_tx0_is_not_on_chain() {
        // An un-mined tx0 (unreachable for a real strand) is NEVER reaped — a row is never dropped
        // before its funding is irreversible.
        assert!(!should_reap(None, 2_000_000));
    }

    #[test]
    fn should_reap_saturates_below_genesis_without_panic() {
        // A mined height ABOVE the tip (a clock/scan transient) saturates to 0 depth ⇒ not buried,
        // not reaped — never an underflow panic.
        assert!(!should_reap(Some(100), 50));
    }
}
