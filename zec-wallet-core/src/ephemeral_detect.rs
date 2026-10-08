//! §3.2i-2 2e-2b — ephemeral-transparent DETECT (the TEX / ZIP-320 stranded / exchange-return
//! poll). This is the money-safety GUARANTEE half of 2e-2b: funds that a TEX `tx0` unshielded to a
//! wallet-controlled EPHEMERAL transparent address — then never forwarded (tx1 expired → STRANDED),
//! or that an exchange RETURNED to that one-time address — are INVISIBLE to the wallet until the SDK
//! polls them explicitly. The shielded scan is shielded-only and the engine's own `refresh_utxos`
//! queries ONLY the external+internal receivers (`get_transparent_receivers(_, true, true)`), NEVER
//! the ephemeral scope, so there is no passive path — the detect is an EXPLICIT,
//! ISOLATED poll that mirrors Recv-2b's recognition but with a DISTINCT per-ephemeral circuit loop.
//!
//! ## What lives here (the testable POLICY)
//! - [`in_detect_scope`] — the BOUNDED-LOOKBACK scope predicate (pure): poll an ephemeral only if it
//!   was RESERVED (exposed) within [`EPHEMERAL_DETECT_LOOKBACK_BLOCKS`] of the tip AND the engine's
//!   own privacy deferral (`next_check_time`) has passed. The -review correction: NOT the
//!   unbounded full historical set (it grows one-per-send ⇒ unbounds the circuit count + endpoint
//!   fingerprint), NOT active-intent-only (that would drop an exchange-return to a completed send).
//! - [`run_detect_loop`] — the per-ephemeral COUNT+SKIP drive (mirrors `wallet::broadcast_group`):
//!   each ephemeral queried over its OWN fresh circuit after a §5.3 jitter, a transport fault on one
//!   ISOLATED (counted + skipped, never `?` over the loop), bounded by a cancel + open-gate + time
//!   budget. A DB-write fault PROPAGATES (the `enhance` posture: network faults skip, store faults
//!   surface) so the caller swallows + logs it.
//! - [`ephemeral_detect_isolation_key`] — a fresh `{prefix}-{OsRng}` per query (the §5 privacy MUST).
//! - [`EphemeralDetectSummary`] — §5.4 COUNTS-only tallies for the `wallet.ephemeral_detect` span.
//!
//! ## What lives in the caller ([`crate::wallet::Wallet::detect_ephemeral_utxos`])
//! Enumerate via `get_ephemeral_transparent_receivers` (engine), the per-ephemeral fresh-circuit
//! [`EphemeralUtxoProbe`] (network), and the [`crate::transparent::recognise_ephemeral_outputs`] put
//! (db) — the parts that need `Inner`. The detect is LIVE in production now that two-step signing is on
//! (gate-removal, 2e-2b-v-5): a TEX two-step reserves an ephemeral, so a return to one is detected. A
//! wallet that has made no TEX send reserves none ⇒ the enumerate is empty ⇒ a no-op (pinned by an
//! empty-set-invariant test). Hooked in the `after_synced` resubmission path (NOT `run_pass`: the
//! watchdog is torn down there, and ≤10 ephemerals × dial+unary would otherwise trip a false stall).
//!
//! ## Deferred to 2e-2b-ii (with the engine recognised-balance reader they share)
//! The scope's "∪ any ephemeral with a recognised non-zero balance" re-inclusion and the skip-funded
//! battery rule both need the engine `get_transparent_balances` ephemeral read that backs ii's
//! `list_stranded`; and they only become NECESSARY once ii's `Stranded`-row reap can remove an intent
//! row. In slice-i there is NO reap, so no funded ephemeral can be dropped — the exposure-window
//! scope is money-complete on its own (a detected ephemeral's funds are durable in the engine the
//! instant they are put; aging out of the window only stops catching a SECOND deposit to a one-time
//! address). See `EPHEMERAL_DETECT_LOOKBACK_BLOCKS` for the completeness-vs-fingerprint trade.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use zcash_client_backend::wallet::{Exposure, TransparentAddressMetadata};
use zcash_transparent::address::TransparentAddress;

use crate::config::JitterPolicy;
use crate::constants::{EPHEMERAL_DETECT_ISOLATION_KEY_PREFIX, EPHEMERAL_DETECT_LOOKBACK_BLOCKS};
use crate::error::WalletError;
use crate::sync::CancelToken;
use crate::transparent::{TransparentRefreshOutcome, TransparentUtxoRecord};

/// Query the endpoint for the unspent outputs paying ONE ephemeral transparent address, over a
/// FRESH isolated circuit (one call = one circuit — the §5 privacy MUST; never the sync circuit,
/// never a shared batch). The own port (ISP), segregated from [`crate::sync::TransparentUtxoSource`]
/// so the detect's per-ephemeral circuit isolation is not silently collapsed onto the batched
/// single-circuit receive poll. RAW records from an UNTRUSTED endpoint — validated at the §4.6
/// boundary by the caller before anything is put. A transport fault is `Err(())`: per-item
/// COUNT+SKIP, never fails the whole detect (a black-hole endpoint on one ephemeral must not hide a
/// strand on another). A testability seam: the live impl ([`crate::wallet`]) connects a fresh
/// `LightwalletdClient`; the test fake returns canned records / faults per address with no network.
#[async_trait]
pub(crate) trait EphemeralUtxoProbe {
    async fn probe_ephemeral(
        &self,
        address: TransparentAddress,
    ) -> Result<Vec<TransparentUtxoRecord>, ()>;
}

/// §5.4 COUNTS-only tallies of ONE detect pass — its OWN observable surface (the
/// `wallet.ephemeral_detect` span), DELIBERATELY distinct from `ResubmitSummary` (the detect is
/// independent of the outbox recovery machine; an operator needs BOTH). No money data, no address,
/// no txid — only how many ephemerals were in scope, queried, and what they yielded.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EphemeralDetectSummary {
    /// Ephemerals in the bounded-lookback scope this pass (post `in_detect_scope` filter). Zero ⇒
    /// no ephemeral in scope (a wallet that has made no recent TEX send) — the pass stays event-silent.
    pub(crate) scope_size: usize,
    /// Of those, actually QUERIED (the loop may stop early on cancel / open-gate / time budget).
    pub(crate) polled: usize,
    /// Of the scope, SKIPPED because the ephemeral ALREADY has a recognised recoverable balance
    /// (§3.2i-2 skip-funded / ADR-0535 Decision 5) — no circuit opened, the funds are already
    /// surfaced by `list_stranded`. Counted distinctly from `polled` so a healthy skip pass is not
    /// mis-read as `incomplete` (a truncation): `scope_size == polled + skipped + deferred + truncated-tail`.
    pub(crate) skipped: usize,
    /// Of the scope, DEFERRED this pass by the per-address probe backoff ([`EphemeralProbeBackoff`],
    /// #334 §3.2i-2 2e-2b-vi contract item 2) — in-scope + unfunded but backed off after a run of cold
    /// probes, so no circuit was opened for it this pass. Counted distinctly (like `skipped`) so a
    /// healthy backoff pass is NOT mis-read as `incomplete`. When a pass probes NOTHING because every
    /// in-scope ephemeral was skipped-or-deferred and ≥ 1 was deferred, the headline is `throttled`
    /// (not `clear`), so a censored link's quiet passes are not laundered as actively-verified-clear.
    pub(crate) deferred: usize,
    /// UTXOs validated + put this pass (idempotent re-puts included — a re-poll of the same unspent
    /// ephemeral output still counts here). `> 0` ⇒ the `outcome = "detected"` headline.
    pub(crate) detected: usize,
    /// Records rejected at the §4.6 boundary (malformed / hostile / not-this-ephemeral) — skipped.
    pub(crate) rejected: usize,
    /// Per-ephemeral TRANSPORT faults (the COUNT+SKIP isolation). A persistent non-zero on a flaky
    /// or censoring link is the `outcome = "degraded"` signal — NOT a stall (the funds, if any, are
    /// on-chain + recovered on a later pass; a DB-write fault is a separate PROPAGATED error).
    pub(crate) errored: usize,
}

impl EphemeralDetectSummary {
    /// Emit the §5.4 `wallet.ephemeral_detect` span (COUNTS only). An empty-scope pass (no ephemeral
    /// in scope) stays SILENT — nothing to observe (mirrors the idle transparent-poll /
    /// empty-enhance-backlog silence). A funded put is NEVER shadowed behind a
    /// benign `wallet.utxo_scan` count — operators read the detect on its own span.
    ///
    /// `outcome` ladder (operational-round fold — fleet alerting keys on this headline, so a
    /// not-all-clear pass must NOT read `"clear"`): `detected` (a put landed — the loudest) >
    /// `degraded` (≥1 per-ephemeral transport fault — a flaky/censored link) > `incomplete` (the
    /// budget/cancel TRUNCATED the scope before every in-window ephemeral was polled — a chronically
    /// under-polled detect on a slow link must be distinguishable from a finished-and-empty one; the
    /// unpolled tail re-polls next pass, shuffled, so it converges) > `clear` (fully polled, nothing
    /// found, no faults). All four are §5.4-safe static strings.
    /// The §5.4 `outcome` headline (the ladder documented on [`Self::emit`]). Pure + total, so the
    /// ladder is unit-testable without capturing a log line.
    fn outcome(&self) -> &'static str {
        if self.detected > 0 {
            "detected"
        } else if self.errored > 0 {
            "degraded"
        } else if self.polled + self.skipped + self.deferred < self.scope_size {
            // The budget/cancel TRUNCATED the scope before every in-window ephemeral was probed,
            // skipped-as-funded, OR deferred-by-backoff — a chronically under-polled detect on a slow
            // link, distinct from a finished pass (where everything was probed, skipped, or deferred).
            // The unpolled tail re-polls next pass, shuffled, so it converges. (A backed-off address is
            // counted in `deferred`, so a healthy backoff pass never masquerades as this truncation.)
            "incomplete"
        } else if self.polled == 0 && self.deferred > 0 {
            // Nothing was PROBED this pass — every in-scope ephemeral was skipped-as-funded or deferred
            // by the backoff, and ≥ 1 was deferred. Distinct from `clear` (actively probed, found
            // nothing) so a censored link whose cold ephemerals are all in backoff is NOT laundered
            // into a healthy `clear`: the fleet reads `throttled` on the quiet passes and `degraded` on
            // the probing ones (#334 security review). Benign-EXPECTED on a wallet whose only
            // reserved ephemerals are long-abandoned — an INFO signal, not a fault.
            "throttled"
        } else {
            "clear"
        }
    }

    pub(crate) fn emit(&self) {
        if self.scope_size == 0 {
            return;
        }
        tracing::debug!(
            target: "zec_wallet_core",
            scope_size = self.scope_size,
            polled = self.polled,
            skipped = self.skipped,
            deferred = self.deferred,
            detected = self.detected,
            rejected = self.rejected,
            errored = self.errored,
            outcome = self.outcome(),
            "wallet.ephemeral_detect",
        );
    }
}

/// A fresh per-EPHEMERAL circuit-isolation key (§2.3 / §3.2i-2): the
/// [`EPHEMERAL_DETECT_ISOLATION_KEY_PREFIX`] + a random `OsRng` token, so each `GetAddressUtxos`
/// query rides its OWN circuit — unlinkable from the sync circuit, from every broadcast, and from
/// every OTHER ephemeral query (so an endpoint cannot cluster N co-timed single-address queries as
/// one wallet's TEX set). The token is RANDOM, never an address: an address is §5.4-sensitive and
/// must never ride the transport credential the host's Tor sees; uniqueness — a distinct circuit —
/// is all §2.3 needs. Tor-OFF: a no-op on clearnet (the query reaches from the wallet's real IP).
pub(crate) fn ephemeral_detect_isolation_key() -> String {
    let token = rand_core::RngCore::next_u64(&mut rand_core::OsRng);
    format!("{EPHEMERAL_DETECT_ISOLATION_KEY_PREFIX}-{token:016x}")
}

/// The BOUNDED-LOOKBACK scope predicate (pure, §3.2i-2 -review correction). Poll an ephemeral
/// this pass iff BOTH:
/// 1. the engine's privacy deferral has PASSED — `next_check_time` is `None` or `<= now` (the engine
///    documents the wallet SHOULD NOT query a public server before it, to avoid linking addresses);
/// 2. it was RESERVED within the return-window — its `Exposed` height is `>= tip −
///    EPHEMERAL_DETECT_LOOKBACK_BLOCKS`. An `Unknown` / `CannotKnow` exposure (a gap-ADVANCED address
///    never reserved for a send, or a restored-wallet address the engine cannot date) is NOT polled:
///    no funds were ever directed there by THIS wallet's send path, and polling the whole historical
///    set is the unbounded fingerprint the correction rejects (the restored-wallet recovery of an
///    undateable ephemeral is the 2e-2b-v leaked/restore mitigation, which needs the recognised
///    balance reader, not a blanket poll).
///
/// `now` and `tip_height` are passed in (never read here) so the predicate is a pure,
/// exhaustively-testable function of (metadata, tip, now) with no clock / chain access. `tip_height`
/// is a bare `u32` (not a `BlockHeight`) to sidestep the core-`money` vs engine-`zcash_protocol`
/// `BlockHeight` split — the engine's `Exposed.at_height` is the `zcash_protocol` type, compared in
/// `u32` so the caller can pass either tip representation via `.into()`.
pub(crate) fn in_detect_scope(
    metadata: &TransparentAddressMetadata,
    tip_height: u32,
    now: SystemTime,
) -> bool {
    // (1) Honour the engine's per-address privacy deferral. A future `next_check_time` ⇒ defer.
    if metadata.next_check_time().is_some_and(|next| next > now) {
        return false;
    }
    // (2) Bounded-lookback over the reservation (exposure) height (saturating — a tip below the
    // window floor cannot underflow).
    match metadata.exposure() {
        Exposure::Exposed { at_height, .. } => {
            let floor = tip_height.saturating_sub(EPHEMERAL_DETECT_LOOKBACK_BLOCKS);
            u32::from(at_height) >= floor
        }
        Exposure::Unknown | Exposure::CannotKnow => false,
    }
}

/// Drive the per-ephemeral DETECT over `targets` (the in-scope ephemerals), mirroring
/// `wallet::broadcast_group`'s isolation discipline: for each ephemeral, a §5.3 decorrelation
/// jitter, then a query over its OWN fresh circuit (`probe`), then validate + put (`recognise`).
///
/// FAULT MODEL (the `enhance`-pass posture, NOT a `?` over the loop): a per-ephemeral TRANSPORT fault
/// (`probe` → `Err(())`) is ISOLATED — counted (`errored`) and skipped, so a black-hole / censoring
/// endpoint on one ephemeral never hides a strand on another. A DB-WRITE fault (`recognise` → `Err`)
/// PROPAGATES (the store-corruption door is not swallowed at this layer — the caller logs the code
/// and swallows it so the sync loop never fails; the chain is the source of truth, the next pass
/// retries). BOUNDED: stops STARTING new queries on a cooperative cancel, a wipe/teardown
/// (`!is_open`), or once `deadline` passes (so it never holds the single-writer pass guard — and thus
/// delays the next scan — for the unbounded `scope × unary-timeout`). Generic over the `probe` /
/// `recognise` closures so the policy (isolation, skip, budget, tally) is unit-testable with fakes —
/// no gRPC, no db.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_detect_loop<P, PF, R, RF>(
    targets: Vec<TransparentAddress>,
    pre_skipped: usize,
    pre_deferred: usize,
    jitter: &JitterPolicy,
    cancel: &CancelToken,
    is_open: impl Fn() -> bool,
    deadline: tokio::time::Instant,
    probe: P,
    recognise: R,
) -> Result<EphemeralDetectSummary, WalletError>
where
    P: Fn(TransparentAddress) -> PF,
    PF: Future<Output = Result<Vec<TransparentUtxoRecord>, ()>>,
    R: Fn(TransparentAddress, Vec<TransparentUtxoRecord>) -> RF,
    RF: Future<Output = Result<TransparentRefreshOutcome, WalletError>>,
{
    // `targets` are the in-scope ephemerals the caller ACTUALLY hands the loop to probe — it already
    // removed the `pre_skipped` funded ones (skip-funded, ADR-0535 Decision 5) AND the `pre_deferred`
    // backed-off ones (#334 probe backoff). The total in-scope size is the sum, so the `outcome` ladder
    // can tell a finished pass (all probed, skipped, OR deferred) from a truncated one.
    let mut summary = EphemeralDetectSummary {
        scope_size: targets.len() + pre_skipped + pre_deferred,
        skipped: pre_skipped,
        deferred: pre_deferred,
        ..EphemeralDetectSummary::default()
    };
    for addr in targets {
        // Teardown / starvation guard: a cooperative cancel, a wipe/close (`!is_open`), or the time
        // budget being spent stops STARTING new queries (the rest re-poll next pass — idempotent).
        if cancel.is_cancelled() || !is_open() || tokio::time::Instant::now() >= deadline {
            break;
        }
        // §5.3 decorrelation jitter BEFORE the query (monotonic `tokio::time`, cancel-safe on drop),
        // so an endpoint cannot read N co-timed single-address queries as one wallet's TEX set.
        let delay = jitter.sample_delay();
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
        summary.polled += 1;
        // Per-ephemeral TRANSPORT fault → COUNT + SKIP (never `?` over the loop).
        let records = match probe(addr).await {
            Ok(records) => records,
            Err(()) => {
                summary.errored += 1;
                continue;
            }
        };
        // Validate (single-expected = THIS ephemeral) + put. A DB-write fault PROPAGATES.
        let outcome = recognise(addr, records).await?;
        summary.detected += outcome.put;
        summary.rejected += outcome.rejected;
    }
    Ok(summary)
}

/// Randomly permute `targets` IN PLACE (Fisher–Yates over `rng`) before the detect queries them —
/// the security review fold. Two reasons: (1) it makes the per-pass anti-starvation robust BY
/// CONSTRUCTION — a censoring endpoint cannot deterministically keep one stranded ephemeral at the
/// tail of every pass's iteration (the property no longer rests on `HashMap` iteration order, an
/// internal a future `Vec`/`BTreeMap` refactor could silently make deterministic); (2) it directly
/// satisfies the engine's instruction to query ephemerals "in a fashion that does not reveal that
/// they are controlled by the same wallet" — a FIXED order across passes is itself a fingerprint.
/// The modulo reduction's bias is negligible for ANY realistic in-scope count (bias ≤ N/2^64; the
/// per-pass set is the ephemerals RESERVED within the lookback window — bounded by SENDS-IN-WINDOW,
/// NOT the gap limit of 10, since `exclude_used = false` returns used ones too — so it can run to
/// dozens for a heavy TEX user; the per-pass circuit count is bounded by the time budget, not this
/// shuffle), and this is a privacy shuffle, NOT key material. Generic over the RNG so it is
/// property-testable; production passes `rand_core::OsRng` (the CSPRNG the isolation keys draw from).
pub(crate) fn shuffle_targets<R: rand_core::RngCore>(
    targets: &mut [TransparentAddress],
    rng: &mut R,
) {
    // Fisher–Yates: for i = len-1 .. 1, swap i with a uniform j in [0, i]. Empty / single ⇒ no-op.
    for i in (1..targets.len()).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        targets.swap(i, j);
    }
}

/// The per-ephemeral probe BACKOFF for the passive DETECT (#334, §3.2i-2 2e-2b-vi CONTRACT slice-1
/// item 2 — the #294/#301 deferral, re-surfaced by the #315 reviews). skip-funded (ADR-0535
/// Decision 5) stops re-querying an ephemeral that ALREADY has a recognised balance; but a
/// persistently-COLD in-window ephemeral — leaked / never funded, or one whose endpoint keeps
/// returning empty (`Ok(vec![])`, censor-forgeable) or faulting (`Err(())`) — was otherwise re-probed
/// EVERY ~20 s pass for the whole [`EPHEMERAL_DETECT_LOOKBACK_BLOCKS`] window. This decays that: each
/// cold probe pushes the address's next-eligible time out by a DOUBLING interval (from `initial`,
/// capped at `max`), so a persistently-cold ephemeral settles to a ~`max` cadence (≈ 1 probe / 10 min
/// vs 1 / 20 s ≈ 30× fewer) instead of hammering the link + battery every pass.
///
/// MONEY-SAFETY: this only ever DELAYS a re-probe — it NEVER marks an address "done", never
/// un-surfaces funds, never feeds the reap (the §3.2i-2 "an empty poll is censor-forgeable" ban).
/// Consulted ONLY by the PASSIVE detect ([`crate::wallet::Wallet::detect_ephemeral_utxos`]); the
/// ACTIVE manual sweep ([`crate::wallet::Wallet::sweep_ephemeral_funds`], which enumerates ALL
/// reserved ephemerals over a WIDE window) MUST NEVER gate on it — a user recovering funds always
/// probes fresh. Worst-case ADDED surfacing latency for a cold ephemeral that later receives funds is
/// `max`; on a frequently-suspended phone that is `max` of ACTIVE time (the monotonic clock freezes in
/// deep sleep), which can span more wall-clock — still money-safe (funds durable on-chain; the manual
/// sweep is the always-fresh escape hatch).
///
/// IN-MEMORY, process-lifetime — a pure battery optimisation keyed on a MONOTONIC
/// `tokio::time::Instant` (meaningless across a restart, so deliberately not persisted; losing the map
/// costs one extra probe pass — harmless). Distinct from the engine's PERSISTED wall-clock
/// `next_check_time` privacy deferral (honoured separately in [`in_detect_scope`]); the two — a
/// battery timer and a privacy deadline — are NOT conflated.
///
/// Pure + clock-injected (`due`/`record_probe` take `now`), like [`in_detect_scope`], so the policy is
/// unit-testable with a paused clock + a fake RNG and no network / DB.
pub(crate) struct EphemeralProbeBackoff {
    entries: HashMap<TransparentAddress, BackoffEntry>,
    initial: Duration,
    max: Duration,
}

struct BackoffEntry {
    /// The earliest instant this address is eligible to be probed again.
    next_probe_at: tokio::time::Instant,
    /// The current (un-jittered) backoff interval; DOUBLES on each consecutive cold probe up to `max`.
    /// The SCHEDULED delay (`next_probe_at − now`) is a downward jitter of this (see `record_probe`).
    interval: Duration,
}

impl EphemeralProbeBackoff {
    pub(crate) fn new(initial: Duration, max: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            initial,
            max,
        }
    }

    /// Is `address` eligible for a probe at `now`? No entry ⇒ yes (never probed, or GC'd after it
    /// left scope — probe promptly). Otherwise only once its backoff window has elapsed.
    pub(crate) fn due(&self, address: &TransparentAddress, now: tokio::time::Instant) -> bool {
        match self.entries.get(address) {
            None => true,
            Some(entry) => now >= entry.next_probe_at,
        }
    }

    /// Record that the passive detect just PROBED `address`. Recording is UNCONDITIONAL — it fires for
    /// an empty result, a transport fault, AND a funds-recognised probe. Money-safe in every arm: a
    /// funds-recognised address is skip-funded + GC'd by [`Self::retain_scope`] next pass (never
    /// backed off); the rare case where the probe returned funds but the `recognise` DB write FAILED
    /// (a broken-store wallet — funds still on-chain) merely carries a bounded ≤`max` re-probe delay,
    /// never a fund loss (the store fault is the terminal condition, and the manual sweep is
    /// unaffected). Grows the backoff one doubling step
    /// ([`next_backoff_interval`]) and schedules the next-eligible time a DOWNWARD-jittered fraction of
    /// it ahead of `now`. The downward jitter (a) breaks the deterministic doubling SHAPE a malicious
    /// endpoint could otherwise use to CLUSTER a wallet's cold ephemerals by their identical re-probe
    /// ramp (§5.3), and (b) being downward-only, can only ever schedule EARLIER — so the `max` latency
    /// budget is never exceeded. RNG-injected like [`shuffle_targets`] (production passes `OsRng`), so
    /// the schedule stays deterministically testable.
    pub(crate) fn record_probe<R: rand_core::RngCore>(
        &mut self,
        address: TransparentAddress,
        now: tokio::time::Instant,
        rng: &mut R,
    ) {
        let entry = self.entries.entry(address).or_insert(BackoffEntry {
            next_probe_at: now,
            interval: Duration::ZERO,
        });
        entry.interval = next_backoff_interval(entry.interval, self.initial, self.max);
        entry.next_probe_at = now + jittered_delay(entry.interval, rng);
    }

    /// Drop entries whose address is no longer an active probe candidate (funded → skip-funded, aged
    /// out of the window, or reorged away), so the map stays bounded by the in-scope set (NOT wallet
    /// lifetime) AND a funded→unfunded (reorg) address carries NO stale backoff — it re-enters with no
    /// entry ⇒ `due` ⇒ probed promptly. Call each pass with the current not-funded-in-window set.
    pub(crate) fn retain_scope(&mut self, active: &HashSet<TransparentAddress>) {
        self.entries.retain(|address, _| active.contains(address));
    }
}

/// One doubling step of the probe backoff, saturating at `max` (mirrors the `sync_controller` /
/// `swap` backoff steps, kept LOCAL per the crate's deliberate-local-duplication convention — this is
/// a per-address map, a genuinely different key space from those per-loop scalars). A zero `prev` (a
/// fresh entry) starts at `initial`; thereafter `prev·2` capped at `max`. Pure + total, so the
/// doubles-then-caps invariant is boundary-tested in isolation.
fn next_backoff_interval(prev: Duration, initial: Duration, max: Duration) -> Duration {
    if prev.is_zero() {
        initial.min(max)
    } else {
        prev.saturating_mul(2).min(max)
    }
}

/// A DOWNWARD jitter of `interval` — a uniform value in `(interval/2, interval]`. Downward-only so a
/// scheduled probe is never LATER than the un-jittered interval (the `max` latency budget holds),
/// while still perturbing the deterministic doubling shape (§5.3 anti-clustering). Pure over the
/// injected RNG. A sub-2 ns interval (whose half floors to zero) returns unchanged — no panic, no bias.
fn jittered_delay<R: rand_core::RngCore>(interval: Duration, rng: &mut R) -> Duration {
    let half_nanos = (interval.as_nanos() / 2) as u64; // ≤ max/2 = 300 s ⇒ fits u64
    if half_nanos == 0 {
        return interval;
    }
    let cut = rng.next_u64() % half_nanos; // uniform in [0, half)
    interval - Duration::from_nanos(cut) // ∈ (interval/2, interval]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use zcash_client_backend::wallet::GapMetadata;
    use zcash_protocol::consensus::BlockHeight;
    use zcash_transparent::keys::{NonHardenedChildIndex, TransparentKeyScope};

    use crate::constants::{
        BROADCAST_ISOLATION_KEY_PREFIX, EPHEMERAL_DETECT_LOOKBACK_BLOCKS, WALLET_SYNC_ISOLATION_KEY,
    };

    /// A distinct, structurally-valid ephemeral t-address keyed by `tag` (so per-address fakes can
    /// tell two ephemerals apart). The 20-byte hash is `tag`-filled — never all-zero.
    fn addr(tag: u8) -> TransparentAddress {
        TransparentAddress::PublicKeyHash([tag; 20])
    }

    /// Ephemeral metadata at reservation `height` with an optional `next_check_time` deferral —
    /// the `Exposed` shape `get_ephemeral_transparent_receivers` returns for a reserved index.
    fn meta(height: u32, next_check: Option<SystemTime>) -> TransparentAddressMetadata {
        TransparentAddressMetadata::derived(
            TransparentKeyScope::EPHEMERAL,
            NonHardenedChildIndex::ZERO,
            Exposure::Exposed {
                at_height: BlockHeight::from(height),
                gap_metadata: GapMetadata::DerivationUnknown,
            },
            next_check,
        )
    }

    // ── target shuffle (privacy + anti-starvation) ──────────────────────────────────────────────

    #[test]
    fn shuffle_targets_preserves_the_multiset_and_noops_on_small_inputs() {
        use rand_core::OsRng;
        // Empty + single ⇒ no-op (no panic, unchanged) — the censor can't starve a 0/1-element pass.
        let mut empty: Vec<TransparentAddress> = vec![];
        shuffle_targets(&mut empty, &mut OsRng);
        assert!(empty.is_empty());
        let mut one = vec![addr(1)];
        shuffle_targets(&mut one, &mut OsRng);
        assert_eq!(one, vec![addr(1)]);
        // Many ⇒ a PERMUTATION: the exact multiset is preserved (never drops/dupes an ephemeral —
        // a money-visibility invariant), only the order changes.
        let mut many: Vec<_> = (0u8..10).map(addr).collect();
        let mut before = many.clone();
        before.sort();
        shuffle_targets(&mut many, &mut OsRng);
        assert_eq!(many.len(), 10, "no ephemeral is lost or duplicated");
        let mut after = many.clone();
        after.sort();
        assert_eq!(
            before, after,
            "the shuffle is a permutation, never a drop/dupe"
        );
    }

    // ── scope predicate ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn in_scope_polls_an_ephemeral_reserved_inside_the_window() {
        let tip = 2_000_000u32;
        // Reserved 100 blocks ago — well inside the 2_880-block window.
        assert!(in_detect_scope(
            &meta(1_999_900, None),
            tip,
            SystemTime::UNIX_EPOCH,
        ));
    }

    #[test]
    fn in_scope_skips_an_ephemeral_reserved_before_the_window() {
        let tip = 2_000_000u32;
        // Reserved EPHEMERAL_DETECT_LOOKBACK_BLOCKS + 1 blocks ago — just past the floor.
        let too_old = 2_000_000 - EPHEMERAL_DETECT_LOOKBACK_BLOCKS - 1;
        assert!(!in_detect_scope(
            &meta(too_old, None),
            tip,
            SystemTime::UNIX_EPOCH,
        ));
    }

    #[test]
    fn in_scope_includes_the_exact_window_floor() {
        let tip = 2_000_000u32;
        // Reserved EXACTLY at the floor (tip − lookback) — inclusive (`>=`).
        let at_floor = 2_000_000 - EPHEMERAL_DETECT_LOOKBACK_BLOCKS;
        assert!(in_detect_scope(
            &meta(at_floor, None),
            tip,
            SystemTime::UNIX_EPOCH,
        ));
    }

    #[test]
    fn in_scope_defers_until_the_engine_next_check_time_passes() {
        let tip = 2_000_000u32;
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        // In-window, but the engine set a FUTURE next_check_time ⇒ defer (privacy throttle).
        assert!(!in_detect_scope(
            &meta(1_999_950, Some(now + Duration::from_secs(60))),
            tip,
            now,
        ));
        // Same address once the deferral has passed ⇒ poll.
        assert!(in_detect_scope(
            &meta(1_999_950, Some(now - Duration::from_secs(1))),
            tip,
            now,
        ));
    }

    #[test]
    fn in_scope_skips_a_non_exposed_address() {
        let tip = 2_000_000u32;
        let non_exposed = |exposure| {
            TransparentAddressMetadata::derived(
                TransparentKeyScope::EPHEMERAL,
                NonHardenedChildIndex::ZERO,
                exposure,
                None,
            )
        };
        // `Unknown` — a gap-ADVANCED address the wallet never reserved for a send ⇒ never polled.
        assert!(!in_detect_scope(
            &non_exposed(Exposure::Unknown),
            tip,
            SystemTime::UNIX_EPOCH,
        ));
        // `CannotKnow` — a restored-wallet address the engine cannot date ⇒ also never polled in i
        // (the restored-strand recovery is 2e-2b-v, with the recognised-balance reader).
        assert!(!in_detect_scope(
            &non_exposed(Exposure::CannotKnow),
            tip,
            SystemTime::UNIX_EPOCH,
        ));
    }

    // ── isolation key ───────────────────────────────────────────────────────────────────────────

    #[test]
    fn ephemeral_detect_isolation_key_is_prefixed_unique_and_distinct_from_send_and_sync() {
        let a = ephemeral_detect_isolation_key();
        let b = ephemeral_detect_isolation_key();
        assert!(
            a.starts_with(EPHEMERAL_DETECT_ISOLATION_KEY_PREFIX),
            "carries the greppable detect prefix",
        );
        assert_ne!(a, b, "each query gets its OWN random circuit key");
        assert_ne!(a, WALLET_SYNC_ISOLATION_KEY, "never the sync circuit key");
        // Disjoint from the broadcast prefix so a detect circuit is never confused with a send (the
        // "wallet-send" prefix is not a prefix of "wallet-ephemeral-detect" and vice versa).
        assert!(
            !a.starts_with(BROADCAST_ISOLATION_KEY_PREFIX),
            "the detect prefix is disjoint from the broadcast prefix",
        );
    }

    // ── loop policy (fake probe / recognise — no gRPC, no db) ────────────────────────────────────

    /// A far-future deadline so the budget never fires in a fast unit test.
    fn no_deadline() -> tokio::time::Instant {
        tokio::time::Instant::now() + Duration::from_secs(3600)
    }

    /// A `recognise` that reports one `put` per record handed to it (the happy db path), recording
    /// which addresses it was asked to recognise so a test can assert the loop reached them.
    fn counting_recognise(
        seen: &Mutex<Vec<TransparentAddress>>,
    ) -> impl Fn(
        TransparentAddress,
        Vec<TransparentUtxoRecord>,
    ) -> std::future::Ready<Result<TransparentRefreshOutcome, WalletError>>
    + '_ {
        move |a, recs| {
            seen.lock().expect("seen lock").push(a);
            std::future::ready(Ok(TransparentRefreshOutcome {
                put: recs.len(),
                rejected: 0,
            }))
        }
    }

    fn one_record() -> Vec<TransparentUtxoRecord> {
        vec![TransparentUtxoRecord {
            txid: vec![0xAB; 32],
            index: 0,
            script: vec![],
            value_zat: 100_000,
            height: 2_000_000,
        }]
    }

    #[tokio::test]
    async fn loop_empty_scope_is_a_clean_noop() {
        let seen = Mutex::new(Vec::new());
        let summary = run_detect_loop(
            vec![],
            0,
            0,
            &JitterPolicy::None,
            &CancelToken::new(),
            || true,
            no_deadline(),
            |_a| std::future::ready(Ok(one_record())),
            counting_recognise(&seen),
        )
        .await
        .expect("empty loop never errors");
        assert_eq!(summary, EphemeralDetectSummary::default());
        assert!(seen.lock().expect("seen").is_empty(), "nothing probed");
    }

    #[tokio::test]
    async fn loop_detects_and_counts_each_in_scope_ephemeral() {
        let seen = Mutex::new(Vec::new());
        let summary = run_detect_loop(
            vec![addr(1), addr(2)],
            0,
            0,
            &JitterPolicy::None,
            &CancelToken::new(),
            || true,
            no_deadline(),
            |_a| std::future::ready(Ok(one_record())),
            counting_recognise(&seen),
        )
        .await
        .expect("happy path");
        assert_eq!(
            summary,
            EphemeralDetectSummary {
                scope_size: 2,
                polled: 2,
                skipped: 0,
                deferred: 0,
                detected: 2,
                rejected: 0,
                errored: 0,
            },
        );
        assert_eq!(seen.lock().expect("seen").len(), 2);
    }

    #[tokio::test]
    async fn loop_reports_pre_skipped_funded_ephemerals_in_the_full_scope() {
        // skip-funded (ADR-0535 Decision 5): the caller probes only the UNFUNDED targets and passes
        // the already-recognised (funded) count as `pre_skipped`; the summary reflects the FULL
        // in-scope size, so a fully-covered pass reads "clear", never a false "incomplete".
        let seen = Mutex::new(Vec::new());
        let summary = run_detect_loop(
            vec![addr(1)], // one unfunded ephemeral, probed (returns nothing this pass)
            2,             // two already-funded ephemerals, skipped (no circuit opened)
            0,             // none backed off this pass
            &JitterPolicy::None,
            &CancelToken::new(),
            || true,
            no_deadline(),
            |_a| std::future::ready(Ok(vec![])),
            counting_recognise(&seen),
        )
        .await
        .expect("happy path");
        assert_eq!(
            summary,
            EphemeralDetectSummary {
                scope_size: 3,
                polled: 1,
                skipped: 2,
                deferred: 0,
                detected: 0,
                rejected: 0,
                errored: 0,
            },
        );
        assert_eq!(
            summary.outcome(),
            "clear",
            "1 probed (empty) + 2 skipped == scope, no fault ⇒ covered/clear"
        );
        assert_eq!(
            seen.lock().expect("seen").len(),
            1,
            "only the unfunded ephemeral was probed — no circuit for the funded two"
        );
    }

    #[tokio::test]
    async fn loop_isolates_a_per_ephemeral_transport_fault() {
        // THE fault-isolation property: a transport fault on the MIDDLE ephemeral is counted + skipped
        // — the first and third are still polled + detected (NOT a `?` that abandons the rest).
        let seen = Mutex::new(Vec::new());
        let summary = run_detect_loop(
            vec![addr(1), addr(2), addr(3)],
            0,
            0,
            &JitterPolicy::None,
            &CancelToken::new(),
            || true,
            no_deadline(),
            |a| {
                let fault = a == addr(2);
                std::future::ready(if fault { Err(()) } else { Ok(one_record()) })
            },
            counting_recognise(&seen),
        )
        .await
        .expect("a transport fault never propagates");
        assert_eq!(
            summary,
            EphemeralDetectSummary {
                scope_size: 3,
                polled: 3,
                skipped: 0,
                deferred: 0,
                detected: 2,
                rejected: 0,
                errored: 1,
            },
        );
        // recognise saw addr(1) and addr(3) — NOT the faulted addr(2).
        let seen = seen.lock().expect("seen");
        assert_eq!(seen.as_slice(), &[addr(1), addr(3)]);
    }

    #[tokio::test]
    async fn loop_propagates_a_db_write_fault() {
        // The `enhance` posture: a STORE fault (not a transport fault) PROPAGATES so the caller can
        // log + swallow it — it is not silently counted as a benign per-item skip.
        let err = run_detect_loop(
            vec![addr(1)],
            0,
            0,
            &JitterPolicy::None,
            &CancelToken::new(),
            || true,
            no_deadline(),
            |_a| std::future::ready(Ok(one_record())),
            |_a, _recs| std::future::ready(Err(WalletError::StoreCorrupt)),
        )
        .await;
        assert!(matches!(err, Err(WalletError::StoreCorrupt)));
    }

    #[tokio::test]
    async fn loop_stops_starting_queries_once_cancelled() {
        // A cooperative cancel BEFORE the loop runs ⇒ nothing is probed (the teardown guard).
        let cancel = CancelToken::new();
        cancel.cancel();
        let probed = AtomicUsize::new(0);
        let seen = Mutex::new(Vec::new());
        let summary = run_detect_loop(
            vec![addr(1), addr(2)],
            0,
            0,
            &JitterPolicy::None,
            &cancel,
            || true,
            no_deadline(),
            |a| {
                probed.fetch_add(1, Ordering::Relaxed);
                std::future::ready(Ok::<_, ()>({
                    let _ = a;
                    one_record()
                }))
            },
            counting_recognise(&seen),
        )
        .await
        .expect("cancel is not an error");
        assert_eq!(probed.load(Ordering::Relaxed), 0, "no query started");
        assert_eq!(summary.scope_size, 2);
        assert_eq!(summary.polled, 0);
    }

    #[tokio::test]
    async fn loop_stops_starting_queries_when_the_gate_closes() {
        // A wipe/teardown (`!is_open`) mid-pass ⇒ stop the phase (never fire a query during teardown).
        let probed = AtomicUsize::new(0);
        let seen = Mutex::new(Vec::new());
        let summary = run_detect_loop(
            vec![addr(1), addr(2)],
            0,
            0,
            &JitterPolicy::None,
            &CancelToken::new(),
            || false,
            no_deadline(),
            |a| {
                probed.fetch_add(1, Ordering::Relaxed);
                std::future::ready(Ok::<_, ()>({
                    let _ = a;
                    one_record()
                }))
            },
            counting_recognise(&seen),
        )
        .await
        .expect("a closed gate is not an error");
        assert_eq!(
            probed.load(Ordering::Relaxed),
            0,
            "gate closed before any query"
        );
        assert_eq!(summary.polled, 0);
    }

    #[tokio::test]
    async fn loop_stops_starting_queries_past_the_time_budget() {
        // An already-passed deadline ⇒ the budget guard stops the loop before the first query (the
        // single-writer pass guard is never held for the unbounded scope × unary-timeout).
        let probed = AtomicUsize::new(0);
        let seen = Mutex::new(Vec::new());
        let past = tokio::time::Instant::now() - Duration::from_secs(1);
        let summary = run_detect_loop(
            vec![addr(1), addr(2)],
            0,
            0,
            &JitterPolicy::None,
            &CancelToken::new(),
            || true,
            past,
            |a| {
                probed.fetch_add(1, Ordering::Relaxed);
                std::future::ready(Ok::<_, ()>({
                    let _ = a;
                    one_record()
                }))
            },
            counting_recognise(&seen),
        )
        .await
        .expect("the budget is not an error");
        assert_eq!(
            probed.load(Ordering::Relaxed),
            0,
            "budget spent before any query"
        );
        assert_eq!(summary.polled, 0);
    }

    #[test]
    fn empty_scope_summary_emits_nothing() {
        // An empty-scope pass (no ephemeral in scope) is event-SILENT — emit short-circuits on scope_size == 0.
        // (Behavioural: no panic, no field built; the §5.4 guard separately asserts the field set.)
        EphemeralDetectSummary::default().emit();
    }

    #[test]
    fn outcome_ladder_distinguishes_detected_degraded_incomplete_and_clear() {
        // operational-round fold + skip-funded: a TRUNCATED or all-faulted pass must NOT
        // read "clear" to fleet alerting, but a SKIP-funded (healthy) pass must. Precedence:
        // detected > degraded > incomplete > clear, where "complete" = polled + skipped == scope.
        let s = |scope_size, polled, skipped, detected, errored| EphemeralDetectSummary {
            scope_size,
            polled,
            skipped,
            deferred: 0,
            detected,
            rejected: 0,
            errored,
        };
        // A put wins the headline even when the pass also faulted or truncated.
        assert_eq!(s(3, 3, 0, 1, 0).outcome(), "detected");
        assert_eq!(s(3, 2, 0, 1, 1).outcome(), "detected");
        // A transport fault with no detection ⇒ degraded (a flaky/censored link).
        assert_eq!(s(3, 3, 0, 0, 1).outcome(), "degraded");
        // Fully polled, nothing found, no fault ⇒ clear.
        assert_eq!(s(3, 3, 0, 0, 0).outcome(), "clear");
        // The budget/cancel truncated the scope (polled + skipped < scope_size) ⇒ NOT "clear".
        assert_eq!(s(3, 1, 0, 0, 0).outcome(), "incomplete");
        // A degraded link that also truncated still reads degraded (the fault is the louder signal).
        assert_eq!(s(3, 1, 0, 0, 1).outcome(), "degraded");
        // skip-funded — a HEALTHY pass that probed nothing because every in-scope ephemeral is
        // already recognised/funded reads "clear", NOT "incomplete" (skipped counts as covered).
        assert_eq!(s(3, 0, 3, 0, 0).outcome(), "clear");
        // Mixed: 1 probed + 2 skipped == scope ⇒ clear even though polled < scope.
        assert_eq!(s(3, 1, 2, 0, 0).outcome(), "clear");
        // 1 probed + 1 skipped < scope (the 3rd was budget-truncated) ⇒ incomplete.
        assert_eq!(s(3, 1, 1, 0, 0).outcome(), "incomplete");
    }

    #[test]
    fn outcome_ladder_reads_throttled_for_a_probed_nothing_backoff_pass() {
        // #334 a pass that opened NO circuit because every in-scope ephemeral was
        // skipped-as-funded or DEFERRED-by-backoff (with ≥ 1 deferred) reads "throttled", NOT "clear" —
        // so a censored link's quiet passes are not laundered as actively-verified-clear.
        let s = |scope_size, polled, skipped, deferred| EphemeralDetectSummary {
            scope_size,
            polled,
            skipped,
            deferred,
            detected: 0,
            rejected: 0,
            errored: 0,
        };
        // All in-scope backed off, nothing probed ⇒ throttled.
        assert_eq!(s(3, 0, 0, 3).outcome(), "throttled");
        // Mixed skip-funded + deferred, nothing probed (≥ 1 deferred) ⇒ still throttled.
        assert_eq!(s(3, 0, 1, 2).outcome(), "throttled");
        // Defer some, probe the rest, find nothing ⇒ clear (NOT a false incomplete, NOT throttled).
        assert_eq!(s(3, 2, 0, 1).outcome(), "clear");
        // A genuine truncation (a due address neither probed, skipped, nor deferred) ⇒ incomplete.
        assert_eq!(s(3, 1, 0, 1).outcome(), "incomplete");
        // All funded-skip, zero deferred ⇒ clear, never throttled (a healthy funded-skip pass).
        assert_eq!(s(2, 0, 2, 0).outcome(), "clear");
        // A backoff pass that ALSO detected / faulted keeps the louder headline (precedence holds).
        assert_eq!(
            EphemeralDetectSummary {
                scope_size: 2,
                polled: 1,
                skipped: 0,
                deferred: 1,
                detected: 1,
                rejected: 0,
                errored: 0,
            }
            .outcome(),
            "detected",
        );
        assert_eq!(
            EphemeralDetectSummary {
                scope_size: 2,
                polled: 1,
                skipped: 0,
                deferred: 1,
                detected: 0,
                rejected: 0,
                errored: 1,
            }
            .outcome(),
            "degraded",
        );
    }

    // ── per-address probe backoff (#334) ─────────────────────────────────────────────────────────

    #[test]
    fn backoff_interval_doubles_from_initial_then_caps_at_max() {
        let initial = Duration::from_secs(60);
        let max = Duration::from_secs(600);
        // A fresh entry (zero `prev`) starts at `initial`.
        assert_eq!(next_backoff_interval(Duration::ZERO, initial, max), initial);
        // Then doubles on each consecutive cold probe...
        assert_eq!(
            next_backoff_interval(Duration::from_secs(60), initial, max),
            Duration::from_secs(120),
        );
        assert_eq!(
            next_backoff_interval(Duration::from_secs(120), initial, max),
            Duration::from_secs(240),
        );
        assert_eq!(
            next_backoff_interval(Duration::from_secs(240), initial, max),
            Duration::from_secs(480),
        );
        // ...capped at `max` (480·2 = 960 > 600), and stays there.
        assert_eq!(
            next_backoff_interval(Duration::from_secs(480), initial, max),
            max
        );
        assert_eq!(next_backoff_interval(max, initial, max), max);
        // Saturating: a pathological huge `prev` caps, never overflows.
        assert_eq!(next_backoff_interval(Duration::MAX, initial, max), max);
    }

    #[test]
    fn jittered_delay_stays_downward_within_half_to_full() {
        use rand_core::OsRng;
        let interval = Duration::from_secs(600);
        let half = interval / 2;
        let mut saw_strictly_below = false;
        for _ in 0..2000 {
            let d = jittered_delay(interval, &mut OsRng);
            assert!(
                d > half && d <= interval,
                "downward jitter must land in (interval/2, interval], got {d:?}",
            );
            saw_strictly_below |= d < interval;
        }
        // The jitter must actually VARY — a no-op that always returned `interval` would satisfy the
        // bound above yet defeat the §5.3 anti-clustering purpose. 2000 draws over a 300 s span make an
        // all-equal-`interval` run astronomically unlikely (P = (1/half_nanos)^2000).
        assert!(
            saw_strictly_below,
            "at least one of 2000 draws must fall strictly below `interval` — the jitter is not varying",
        );
        // A sub-2 ns interval whose half floors to zero returns unchanged (no panic, no mod-by-zero).
        assert_eq!(
            jittered_delay(Duration::from_nanos(1), &mut OsRng),
            Duration::from_nanos(1),
        );
    }

    #[tokio::test(start_paused = true)]
    async fn backoff_defers_a_probed_address_until_the_window_elapses() {
        use rand_core::OsRng;
        use tokio::time::{Instant, advance};
        let mut bk = EphemeralProbeBackoff::new(Duration::from_secs(60), Duration::from_secs(600));
        let a = addr(1);
        // Never probed ⇒ always due.
        assert!(bk.due(&a, Instant::now()));
        // Probe it ⇒ backed off. The downward jitter schedules the next probe in (30 s, 60 s], so it is
        // NOT due immediately, NOT due at +29 s, and IS due by +60 s (the jitter can only pull EARLIER).
        bk.record_probe(a, Instant::now(), &mut OsRng);
        assert!(!bk.due(&a, Instant::now()), "just probed — backed off");
        advance(Duration::from_secs(29)).await;
        assert!(
            !bk.due(&a, Instant::now()),
            "still inside the (downward-jittered) window at +29 s",
        );
        advance(Duration::from_secs(31)).await; // +60 s total ≥ the un-jittered ceiling
        assert!(
            bk.due(&a, Instant::now()),
            "window elapsed by +60 s ⇒ due again"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn backoff_second_probe_defers_strictly_longer_than_the_first() {
        // The DOUBLING is integrated through `record_probe` (not only the pure `next_backoff_interval`):
        // a SECOND consecutive cold probe schedules a strictly longer window than the first. Discriminates
        // against a broken backoff that kept the interval flat at `initial`.
        use rand_core::OsRng;
        use tokio::time::{Instant, advance};
        let mut bk = EphemeralProbeBackoff::new(Duration::from_secs(60), Duration::from_secs(600));
        let a = addr(1);
        // Probe #1: window (30 s, 60 s]. Let it fully elapse.
        bk.record_probe(a, Instant::now(), &mut OsRng);
        advance(Duration::from_secs(60)).await;
        assert!(
            bk.due(&a, Instant::now()),
            "first window (≤ 60 s) elapsed ⇒ due"
        );
        // Probe #2: the interval DOUBLED to 120 s, so the window is now (120 s, 180 s] from here. At +60 s
        // (t = 120 s) it is NOT yet due — whereas a non-doubling backoff (still ≤ 60 s) WOULD be due by
        // now. That gap is the discriminating assertion, robust to the downward jitter (next_probe_at is
        // strictly > 120 s from the second record).
        bk.record_probe(a, Instant::now(), &mut OsRng);
        advance(Duration::from_secs(60)).await;
        assert!(
            !bk.due(&a, Instant::now()),
            "second deferral doubled (> 60 s) ⇒ still not due where a single 60 s window would be",
        );
        advance(Duration::from_secs(60)).await; // t = 180 s ≥ the 120 s doubled ceiling
        assert!(
            bk.due(&a, Instant::now()),
            "second (≤ 120 s) window elapsed ⇒ due"
        );
    }

    #[tokio::test]
    async fn backoff_retain_scope_gcs_an_address_that_left_the_scope() {
        use rand_core::OsRng;
        let now = tokio::time::Instant::now();
        let mut bk = EphemeralProbeBackoff::new(Duration::from_secs(60), Duration::from_secs(600));
        let (a, b) = (addr(1), addr(2));
        bk.record_probe(a, now, &mut OsRng);
        bk.record_probe(b, now, &mut OsRng);
        assert!(!bk.due(&a, now) && !bk.due(&b, now), "both backed off");
        // GC to a scope holding only `a`: b's entry is dropped ⇒ b is due again (no entry), a stays
        // backed off. A funded / aged-out / reorged-away ephemeral thus never carries a stale backoff.
        let scope: HashSet<_> = [a].into_iter().collect();
        bk.retain_scope(&scope);
        assert!(!bk.due(&a, now), "a is still in scope ⇒ keeps its backoff");
        assert!(
            bk.due(&b, now),
            "b left the scope ⇒ entry GC'd ⇒ probed promptly on re-entry",
        );
    }
}
