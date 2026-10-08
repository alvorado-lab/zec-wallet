//! §3.2i-2 2e-2b-v-2 — the MANUAL ephemeral-SWEEP recovery summary + §5.4 observability.
//!
//! The async orchestration (enumerate ALL reserved ephemerals → per-ephemeral, in isolation: raw-query
//! its UTXOs → validate + inject → propose at the economic floor → sign → broadcast) lives in
//! [`crate::wallet::Wallet::sweep_ephemeral_funds`] — it needs the network probe + the db lock. This
//! module owns the COUNTS-only tally that method returns and the span it emits, so the §5.4 surface +
//! the outcome ladder are testable WITHOUT a wallet.
//!
//! Live in production now that two-step TEX signing is on (gate-removal, 2e-2b-v-5); a wallet that has
//! made no TEX send reserves no ephemeral ⇒ the enumerate is empty ⇒ a no-op. Harness-tested; the
//! `textest1…` device proof rides 2e-2b-v-5.

/// §5.4 COUNTS-only tally of ONE [`sweep_ephemeral_funds`](crate::wallet::Wallet::sweep_ephemeral_funds)
/// invocation. No address, no per-address amount, no txid — only how many ephemerals were enumerated /
/// swept, the AGGREGATE recovered total (the SINGLE money figure — host-rendered, NEVER logged), and
/// the bounded-work signals.
///
/// `recovered_zat` is the SUM of the net value that lands shielded across the swept ephemerals (each
/// sweep's `summarize_shield(..).change_zat` = gross − fee). It is tallied ONLY for an ephemeral whose
/// sweep tx the endpoint ACCEPTED — a sign-but-broadcast-failed sweep counts `failed` (re-runnable), so
/// the figure never optimistically claims funds an endpoint never took, and a re-run is idempotent (the
/// engine excludes a spent UTXO, so a tx that secretly DID land just yields `Ok(None)` next time). It is
/// PROVISIONAL — accepted ≠ mined: a sweep can be accepted then evicted / expire before mining, so it is
/// "pending, not settled" until the next scan reflects it in the shielded balance (the FFI DTO carries
/// this framing for the host — mirror it here for symmetry).
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct EphemeralSweepSummary {
    /// Ephemerals ENUMERATED this invocation (the wide-window reserved set, before the per-call cap).
    pub scanned: usize,
    /// Of the attempted set, SWEPT — a sweep tx signed, persisted, and ACCEPTED by the endpoint.
    pub swept: usize,
    /// The AGGREGATE net value recovered into the shielded pool (Σ the swept proposals' net shielded
    /// output). The ONLY money figure; an aggregate (never per-address), host-rendered, never logged.
    pub recovered_zat: i64,
    /// Per-ephemeral faults isolated (query / propose-stale / sign / broadcast miss) — COUNT+SKIP,
    /// never aborts the rest; the funds stay on-chain + re-sweepable, so a `failed` is "re-run", not a
    /// loss. A DB-write CORRUPTION is NOT counted here — it PROPAGATES out of `sweep_ephemeral_funds`.
    pub failed: usize,
    /// Ephemerals NOT attempted because the per-invocation cap
    /// ([`EPHEMERAL_SWEEP_MAX_ADDRS`](crate::constants::EPHEMERAL_SWEEP_MAX_ADDRS)) or the time budget
    /// truncated the set — the user re-runs (honest, never a silent drop; the §"no silent caps" rule).
    pub truncated: usize,
}

/// §5.4 REDACTING `Debug` (NOT derived): `recovered_zat` is a money amount, so a stray `?`-render — a
/// future `tracing::debug!(?summary)` or an `assert_eq!` failure message — must never surface it. The
/// counts are §5.4-safe and shown; the amount is redacted (the `emit()` span already omits it).
impl std::fmt::Debug for EphemeralSweepSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EphemeralSweepSummary")
            .field("scanned", &self.scanned)
            .field("swept", &self.swept)
            .field("recovered_zat", &"<redacted §5.4>")
            .field("failed", &self.failed)
            .field("truncated", &self.truncated)
            .finish()
    }
}

impl EphemeralSweepSummary {
    /// The §5.4 `outcome` headline (counts-only, pure + total — unit-testable without a log line).
    /// Ladder by interest: `swept` (money moved — the loudest) > `degraded` (≥ 1 per-ephemeral fault,
    /// none swept) > `truncated` (the cap/budget left work, none swept or faulted) > `clear` (nothing
    /// to recover). All four are §5.4-safe static strings.
    fn outcome(&self) -> &'static str {
        if self.swept > 0 {
            "swept"
        } else if self.failed > 0 {
            "degraded"
        } else if self.truncated > 0 {
            "truncated"
        } else {
            "clear"
        }
    }

    /// Emit the §5.4 `wallet.ephemeral_sweep` span — COUNTS + `outcome` ONLY. `recovered_zat` is a
    /// money amount, so it is DELIBERATELY ABSENT from the span (§5.4 never-LOG): it crosses the FFI to
    /// the host but never a log line. A wholly-empty invocation (nothing enumerated — a wallet that has
    /// reserved no ephemeral) stays SILENT, nothing to observe (mirrors the idle ephemeral-detect /
    /// empty-enhance-backlog silence).
    pub(crate) fn emit(&self) {
        if self.scanned == 0 {
            return;
        }
        tracing::debug!(
            target: "zec_wallet_core",
            scanned = self.scanned,
            swept = self.swept,
            failed = self.failed,
            truncated = self.truncated,
            outcome = self.outcome(),
            "wallet.ephemeral_sweep",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_ladder_is_ordered_by_interest() {
        // Empty ⇒ clear.
        assert_eq!(EphemeralSweepSummary::default().outcome(), "clear");
        // Truncated-only (cap left work) ⇒ truncated, never clear.
        assert_eq!(
            EphemeralSweepSummary {
                scanned: 100,
                truncated: 36,
                ..Default::default()
            }
            .outcome(),
            "truncated",
        );
        // A fault outranks truncation (a flaky/censored link is louder than "more to do").
        assert_eq!(
            EphemeralSweepSummary {
                scanned: 100,
                failed: 1,
                truncated: 36,
                ..Default::default()
            }
            .outcome(),
            "degraded",
        );
        // A swept ephemeral is the loudest — money moved — even alongside faults/truncation.
        assert_eq!(
            EphemeralSweepSummary {
                scanned: 100,
                swept: 1,
                recovered_zat: 50_000,
                failed: 1,
                truncated: 36,
            }
            .outcome(),
            "swept",
        );
    }

    #[test]
    fn emit_is_silent_on_an_empty_invocation() {
        // A wallet that has reserved no ephemeral: nothing enumerated ⇒ no span (asserted via `outcome`
        // parity — an empty summary is `clear` and `emit` early-returns on `scanned == 0`). A
        // non-capturing smoke that `emit` does not panic on either shape.
        EphemeralSweepSummary::default().emit();
        EphemeralSweepSummary {
            scanned: 1,
            swept: 1,
            recovered_zat: 12_345,
            ..Default::default()
        }
        .emit();
    }
}
