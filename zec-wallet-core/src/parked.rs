//! §3.2i-2 2e-2b-v-3 — the gate-removal VISIBILITY surfaces: the PARKED-send list + the ephemeral
//! reservation-PRESSURE gauge. Both are READ-ONLY (the user-facing cancel landed SEPARATELY in v-4a as
//! [`Wallet::cancel_parked_send`](crate::Wallet::cancel_parked_send); these readers stay
//! read-only) and exist so gate-removal (v-5, LANDED) never opened a SILENT-HIDE window: a TEX send that
//! cannot complete right now must be VISIBLE ("saved & pending"), and the leaked-reservation ceiling
//! that can brick TEX sends with NO engine remedy must be host-renderable. Post-gate-removal these
//! readers PERSIST and stay useful — the parked CAUSE transformed from the gate-era `Blocked` to the
//! gap-limit `Ceiling`, but the `Queued`-row read that backs both surfaces is identical. Since
//! (#331) the parked list covers EVERY queued send — the plainly-queued single-step rows too, each
//! tagged with a [`ParkedSendKind`] — because an offline-queued single-step send that survived a
//! process death was otherwise on NO surface and uncancellable (a double-pay window).
//!
//! ## Why visibility is the whole job
//! The *behavioral* "known-ceiling backoff" the §3.2i-2 build constraint names is shipped — the queued
//! drain parks a TEX as [`Prepared::Ceiling`](crate::send) WITHOUT retry-churn (`send.rs`) when the
//! ephemeral gap-limit window is full, and the engine offers NO un-reserve. Two DISTINCT slot kinds
//! consume the window: a STRANDED slot (tx0 MINED → its ephemeral is funded AND the on-chain tx0 already
//! advanced the gap; the manual sweep v-2 recovers those FUNDS) and a LEAKED slot (tx0 NEVER mined under
//! a censoring endpoint → NO funds to sweep — and NO self-heal: the engine does NOT free the slot when
//! the unmined tx0 expires (`find_gap_start` advances ONLY on a MINED first-use — the "frees on expiry"
//! hypothesis this doc used to record is REFUTED at the pinned engine source, #315 review); the leaked
//! slot's only remedies are the #315 slice-1 attempt cap (stop the bleeding — see [`ParkedSend::paused`])
//! and the slice-2 self-mint reclaim (reopen the window). These read surfaces make the parked state
//! visible and — via the v-4 cancel + the #315 retry — actionable.
//!
//! ## §5.4
//! [`ParkedSend`] carries the send AMOUNT (host-rendered, NEVER-LOG — its `Debug` REDACTS the amount)
//! and DELIBERATELY no recipient (the TEX address is never-RENDER, absent by construction).
//! [`ReservationPressure`] is COUNTS-only (loggable) and may emit the `wallet.reservation_pressure` span.

use crate::error::WalletError;
use crate::seed::SpendBinding;
use crate::state::SigningBlock;

/// The SHAPE of a parked send — presentation-only nuance for the host's copy (both kinds are
/// equally `Queued`, equally not-yet-attempted, equally cancellable). Chain-free metadata (§5.4
/// loggable — it names a shape, never an amount/recipient).
///
/// Until (#331) the parked surface was TEX-only and a queued SINGLE-STEP send was
/// deliberately invisible ("else a healthy queued send would falsely read as stuck"). That
/// rationale assumed a prompt drain — but the offline queue exists precisely for the window where
/// no drain can run: offline → queue → process death → relaunch still offline left a COMMITTED
/// spend on NO surface and uncancellable, so the user re-enters it and both drain on reconnect (a
/// DOUBLE PAY). Presentation owns the "not stuck" phrasing concern; the reader surfaces every
/// committed row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive] // G2: pub enums grow (a future shape must not break consumers)
pub enum ParkedSendKind {
    /// A TEX (ZIP-320) send the engine lowers to a two-step through a one-time address — it drains
    /// when it can, or gap-limit ceiling-parks (the drain returns [`Prepared::Ceiling`](crate::send)
    /// and leaves the row `Queued`).
    TwoStep,
    /// A plain single-step send (shielded / non-TEX transparent recipient) waiting in the offline
    /// queue — it drains at the next connectivity/drain pass; nothing about it is "stuck".
    SingleStep,
}

/// What a user-paced [`authorize_parked_send`](crate::Wallet::authorize_parked_send) actually did
/// (FR-23-b, spec §1.9.1). The verb signs an ALREADY-COMMITTED queued intent inside the host's
/// authorize-spend bracket, so every arm below describes a money-safe end state — there is no
/// "failed" arm that leaves the row ambiguous. Chain-free (§5.4 loggable — it names an outcome
/// shape, never an amount/recipient/txid).
///
/// HOST CONTRACT (honesty): only [`Signed`](Self::Signed) may be phrased as "sending now".
/// [`StillQueued`](Self::StillQueued) is NOT an error — nothing failed and nothing was lost; the
/// send simply did not become sendable at this moment (a momentary insufficiency, a
/// gap-limit-capped two-step, or a background drain pass that won the atomic claim first). Phrase
/// it as "still waiting", never as a failure, and leave the row on the parked surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive] // G2: pub enums grow (a future outcome must not break consumers)
pub enum ParkedAuthorization {
    /// SIGNED inside the bracket: the intent is now a durable `Sent` transaction (txid recorded)
    /// and its raw group was handed to the detached broadcast kick. The row LEAVES the parked
    /// surface; it appears in activity / in-flight from here. (Broadcast is not confirmed by this
    /// value — the kick is best-effort and the §6.1 `ReBroadcast` arm is the durable fallback, the
    /// same seedless path every signed outbox row rides.)
    Signed,
    /// NOTHING was signed and the row STAYS `Queued`, exactly as it was — funds untouched, no
    /// note reserved, nothing on-chain. Re-authorizable later (and the background drain keeps
    /// trying on its own at a tier that can sign unattended).
    StillQueued,
    /// A deadline-tagged (swap-deposit) row whose quote had lapsed: TERMINAL, the row was deleted
    /// rather than fed to a dead quote (§4.4). Unreachable from the parked surface, which excludes
    /// deposit rows — present for exhaustiveness at the core seam.
    Expired,
    /// No row matched the `(id, created_at)` pin: already sent / cancelled / drained, the rowid was
    /// reused, or it is a deposit row (excluded here exactly as in
    /// [`cancel_parked_send`](crate::Wallet::cancel_parked_send)). Idempotent, never an error —
    /// re-read [`list_parked_sends`](crate::Wallet::list_parked_sends). MUST NOT be presented as
    /// "cancelled" nor invite a re-send: a row that began sending has left this surface, so the
    /// payment may be IN-FLIGHT and a re-send risks a DOUBLE-PAY.
    NotFound,
}

/// One queued send PARKED in `Queued` — ANY committed send that has not completed yet: a TEX
/// (ZIP-320) two-step awaiting its drain pass or gap-limit ceiling-parked, OR (since #331) a plain
/// single-step send waiting in the offline queue. Surfaced so the host shows it "saved & pending"
/// instead of silence, and so the user can cancel it (v-4). AMOUNT-only (the recipient address is
/// §5.4 never-RENDER, absent by construction); cause-agnostic (`ParkedSend`, never
/// "blocked"/"stuck"); the [`kind`](Self::kind) carries the shape nuance so presentation CAN
/// phrase a one-time-address send differently from a plainly-queued one.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ParkedSend {
    /// The opaque queued-send id (the §3.1 [`QueuedSendId`](crate::state) value). The host passes it back
    /// (with `created_at`) to [`Wallet::cancel_parked_send`](crate::Wallet::cancel_parked_send)
    /// (landed v-4a) to cancel this parked send; carries no money data itself.
    pub id: i64,
    /// The shape of this parked send — presentation-only (see [`ParkedSendKind`]); both kinds are
    /// equally committed and equally cancellable.
    pub kind: ParkedSendKind,
    /// The user's committed OUTBOUND amount in zatoshis (gross — the sum of the request's payment legs,
    /// excludes the fee). Host-rendered ("X saved & pending"); §5.4 NEVER-LOG (the `Debug` redacts it).
    pub amount_zat: i64,
    /// Unix seconds at enqueue (the stored `created_at`) — the host computes the age (no wall-clock in
    /// the reader). Display-only.
    pub created_at: i64,
    /// `true` iff this send is PAUSED at its #315 ephemeral-reserving attempt cap: the drain gave
    /// up re-proposing it (each attempt was leaking an engine gap slot the engine never returns)
    /// and it will NOT send on its own until the user acts. The HONESTY bit the review
    /// required: without it a given-up TEX send renders exactly like a healthy pending one and the
    /// user waits forever. Presentation contract: a `paused` row gets DISTINCT copy ("paused —
    /// your funds are safe; retry or cancel", never "will send when ready") + a RETRY affordance
    /// ([`Wallet::retry_parked_send`](crate::Wallet::retry_parked_send), the leak-cheapest resume —
    /// cancel+re-send restarts the leak budget on a fresh row) alongside the usual cancel. `false`
    /// for every healthy queued row of either kind. Chain-free status metadata (§5.4 loggable).
    pub paused: bool,
    /// `true` iff this row is MID-SIGNATURE (`state = Submitting`): something claimed it and is
    /// building/proving the transaction right now — or was, before the process died (#400 R2).
    ///
    /// Why it is surfaced at all: the claim (`mark_submitting`) commits BEFORE the unbounded ZK
    /// prove, so an OOM kill in that window used to leave a committed spend on NO surface —
    /// the #331 silent-hide shape, and since FR-23-b reachable by a user tap. The §6.3 reconcile
    /// re-queues such a row on the next sync pass; until then (forever, for a sync-off host) this
    /// bit is the only thing that keeps the money visible.
    ///
    /// Presentation contract: PREPARING, never "failed" and never "sending" — no transaction has
    /// been broadcast and none may exist yet. Offer NO affordance at all: every sign verb requires
    /// `Queued` and can only answer `NotFound`, and `cancel_parked_send` is `Queued`-guarded too,
    /// so it can only return `false` — behind a confirm dialog that promises "nothing leaves your
    /// wallet", which is exactly what this row cannot guarantee (see below).
    ///
    /// ⚠ THE BALANCE RELATIONSHIP DOES NOT HOLD FOR THIS ROW. Every other parked row reserves no
    /// notes, so its amount is still fully spendable and the host renders it as an earmark OVER
    /// the balance. A `Submitting` row straddles the create-committed-but-unrecorded window: the
    /// engine's `create` commits the transaction to the WALLET db (notes marked spent) before
    /// `mark_sent_multi` commits `Submitting → Sent` to the AUX db, so past that first commit the
    /// amount is ALREADY out of `spendable_zat`. Presentation must not claim otherwise for this
    /// row — the money is safe either way, but "still part of your balance" can be false.
    ///
    /// `false` for an ordinary parked row.
    pub sending: bool,
    /// FR-17 (#396): the row's spend-binding nonce — the value a host-custody
    /// re-stage flow records when the user re-authorizes THIS parked send, so the
    /// drain's sign pull (which presents the row binding) matches exactly this row
    /// and no other. `None` for a pre-FR-17 row (its pull presents unbound).
    /// Opaque + random, NOT key material; not logged (correlates row↔sign).
    pub binding: Option<SpendBinding>,
    /// WHY the drain will NOT sign this row, if it will not
    /// (`ironwood-nu63-support.md` §3.2/§6.1; GRACE-1 §4p item 2 — the four
    /// states, on [`SigningBlock`]). The sibling honesty bit to
    /// [`Self::paused`], and it exists for the same reason: without it a row the
    /// drain silently skips every pass renders exactly like a healthy pending
    /// one, and the user waits forever for a send that cannot go. Was a bool
    /// ("blocked by network upgrade") — which rendered "waiting for an app
    /// update" for a server that had merely stopped reporting its network,
    /// where an update fixes nothing and the honest step is "switch servers".
    ///
    /// WALLET-level, not row-level — every queued row shares it, because the
    /// condition is about the app or the server, not anything about this
    /// payment. Surfaced per row because that is where the user is looking.
    ///
    /// Presentation contract: DISTINCT copy per variant (the `SigningBlock`
    /// doc) — never "will send when ready" (false: it will not, as things
    /// stand) and never "failed" (false: the money is untouched and the intent
    /// is intact). Cancel stays available; RETRY must not be offered, since
    /// retrying changes nothing until the cause is gone. Chain-free status
    /// metadata (§5.4 loggable).
    pub signing_block: Option<SigningBlock>,
}

// §5.4 belt: a money figure (`amount_zat`) must never `{:?}`-leak into a log. The id + created_at +
// paused are chain-free metadata (loggable); the amount is REDACTED. (Mirrors the sweep summary's
// redacting `Debug`; this type is never spanned either, so the redaction is pure defense-in-depth.)
impl std::fmt::Debug for ParkedSend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParkedSend")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .field("amount_zat", &"<redacted §5.4>")
            .field("created_at", &self.created_at)
            .field("paused", &self.paused)
            .field("sending", &self.sending)
            .field("signing_block", &self.signing_block)
            // FR-17: SpendBinding's own Debug is already redacted; presence-only here.
            .field("binding", &self.binding.map(|_| "SpendBinding(..)"))
            .finish()
    }
}

/// The ephemeral reservation-PRESSURE gauge (the leaked-ephemeral host-recovery surface): a
/// LOWER-BOUND count of how many one-time-address reservations are outstanding, against the engine
/// gap-limit. COUNTS-only (§5.4 loggable). An INDICATOR for the host (render "N of M one-time-address
/// slots in use"), NEVER a money gate — the engine remains the SSOT that enforces the ceiling at
/// reservation time. ⚠ The count is a LOWER BOUND (#315 slice-2 proof discovery): the engine read it
/// wraps is BLIND to created-tx0 leaks (a leaked tx0's output row exists from create-persist, so the
/// `exclude_used` read drops it), so it can read `0` while the window is fully bricked. Treat it as
/// "at least N in use", never as proof the window is clear; the CEILING copy (`TexSendLimitReached`,
/// distinct from this gauge) carries the dual-natured "some may clear / some won't" framing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReservationPressure {
    /// Outstanding ephemeral reservations — a LOWER BOUND (#315 slice-2 proof discovery): the engine
    /// read this wraps is blind to created-tx0 leaks (their output rows exist from create-persist,
    /// unmined, so `exclude_used` drops them) and counts only create-FAULT leaks — the gauge can read
    /// `0` while the window is fully bricked. See `account::outstanding_ephemeral_reservations` for
    /// the mechanism + the upstream-FR disposition. `0` when no TEX two-step is in flight.
    pub outstanding: u32,
    /// The engine ephemeral gap limit ([`EPHEMERAL_GAP_LIMIT`](crate::constants::EPHEMERAL_GAP_LIMIT)) —
    /// the denominator for the host's "N of M" copy. Mirrors the engine default; an observability figure,
    /// never the enforcing bound.
    pub limit: u32,
}

impl ReservationPressure {
    /// Build the gauge from the outstanding count, stamping the engine gap [`limit`](Self::limit).
    pub(crate) fn new(outstanding: u32) -> Self {
        Self {
            outstanding,
            limit: crate::constants::EPHEMERAL_GAP_LIMIT,
        }
    }

    /// `true` iff the engine would refuse a fresh ephemeral reservation right now (outstanding has
    /// reached the gap limit), so the next TEX send parks at the ceiling. CONSERVATIVE `>=` so the host
    /// warns AT the limit, not one past it. An indicator — the engine is the enforcing SSOT.
    pub fn at_ceiling(self) -> bool {
        self.outstanding >= self.limit
    }

    /// Emit the counts-only `wallet.reservation_pressure` observability span (§5.4: counts only — no
    /// address, no amount). SILENT when `outstanding == 0` (no TEX two-step in flight — nothing to
    /// observe); the span fires only once reservations exist (post-gate-removal), where an at-ceiling
    /// state is a genuinely useful fleet signal (TEX sends are availability-bricked).
    pub(crate) fn emit(self) {
        if self.outstanding == 0 {
            return;
        }
        // `target: "zec_wallet_core"` (NOT the span name) so the §5.4 `CaptureLayer` — which filters on
        // `target().starts_with("zec_wallet_core")` — actually SEES this event; the span name rides the
        // MESSAGE position, exactly like every other wallet span (`wallet.ephemeral_sweep` etc).
        tracing::debug!(
            target: "zec_wallet_core",
            outstanding = self.outstanding,
            limit = self.limit,
            at_ceiling = self.at_ceiling(),
            "wallet.reservation_pressure",
        );
    }
}

/// Classify ONE queued intent as a PARKED send (pure, side-effect-free — no propose, no DB, no
/// network). `None` ⇒ a swap deposit ONLY (its own quote-deadline lifecycle + the SWAP status
/// surface). EVERY other `Queued` row surfaces — it is a committed, not-yet-attempted spend the
/// user must be able to SEE and CANCEL (#331: a queued single-step send invisible across a
/// relaunch was a double-pay window). The [`ParkedSendKind`] carries the shape:
/// [`is_tex_recipient`](crate::send) on ANY leg ⇒ [`TwoStep`](ParkedSendKind::TwoStep) — the SAME
/// SSOT the producer (`normalize_tex_first` + `prepare_queued`/`drain_multi`) uses, so the kind can
/// never drift from what the drain actually builds; otherwise
/// [`SingleStep`](ParkedSendKind::SingleStep). (The surface is a faithful superset in time: a row
/// shows whether it is ceiling-parked, awaiting its next drain pass, or transiently
/// `Stale`/`Retry` — every such row is genuinely `Queued` + not-yet-completed, so "saved &
/// pending" is honest for both kinds.)
///
/// ⚠ KIND TRIPWIRE (softened from the pre-#331 completeness tripwire): the TEX-recipient proxy is
/// sound ONLY while the engine produces a multi-step proposal IFF a TEX recipient is present —
/// pinned by the real-engine shape tests in `send.rs` (`tex_send_builds_a_real_zip320_two_step…` ⇒
/// `steps()==2`; the plain-transparent / shield / single send tests ⇒ `steps()==1`). If a future
/// engine bump introduces a NON-TEX multi-step, this classifier would surface it with the WRONG
/// KIND (`SingleStep`) — since #331 that is a copy nuance, no longer the silent-hide it used to
/// be (the row still surfaces, still cancellable). Those `send.rs` tests remain the regression
/// pin; update this predicate in lockstep when they break.
///
/// A torn at-rest URI (it was validated at enqueue) is corruption ⇒
/// [`StoreCorrupt`](WalletError::StoreCorrupt) — fail-closed + LOUD, never a silent drop (a parked send
/// the user is waiting on must surface, even as an error; §4.6).
///
/// `sending` is the row's `state = Submitting` bit, carried in by the reader
/// ([`list_parkable`](crate::intent_store::list_parkable)) rather than derived here — this
/// function is pure over the intent's own fields and has no view of the row state.
pub(crate) fn classify(
    intent: &crate::intent_store::QueuedIntent,
    sending: bool,
    // WALLET-level, passed in rather than read here: this module is pure row
    // classification with no store access, and deriving "are we stale" a second
    // time would be exactly the duplicated predicate §3.2 forbids
    // (`ConsensusCompatibility::signing_block` is the one derivation).
    signing_block: Option<SigningBlock>,
) -> Result<Option<ParkedSend>, WalletError> {
    // A swap-deposit intent owns the deposit_gate quote-deadline lifecycle (prepare_queued) — it is not a
    // "parked send", so it never appears in this surface. NOTE (money-red-team, RESOLVED at
    // gate-removal 2e-2b-v-5): a swap deposit to a TEX address now DRAINS like any TEX two-step within
    // its deadline (the old "doomed pre-gate-removal" concern is gone); if it transiently ceiling-parks,
    // the deposit_gate self-expires it at the quote deadline. It moves NO funds until it drains, and is
    // surfaced by the SWAP status surface, so excluding it from THIS (one-time-send) surface hides no money.
    if intent.deposit_deadline.is_some() {
        return Ok(None);
    }
    let request =
        zip321::TransactionRequest::from_uri(&intent.uri).map_err(|_| WalletError::StoreCorrupt)?;
    let is_two_step = request
        .payments()
        .values()
        .any(|p| crate::send::is_tex_recipient(p.recipient_address()));
    Ok(Some(ParkedSend {
        id: intent.id.value(),
        kind: if is_two_step {
            ParkedSendKind::TwoStep
        } else {
            ParkedSendKind::SingleStep
        },
        amount_zat: gross_outbound_zat(&request),
        created_at: intent.created_at,
        // The #315 paused bit: the SAME `>=` the drain's cap gate applies (`prepare_queued`), so
        // the surface can never say "will send when ready" about a row the drain will skip. Only
        // an ephemeral-reserving attempt increments the counter, so a single-step row is never
        // paused by construction.
        paused: intent.repropose_attempts >= crate::constants::MAX_TEX_REPROPOSE_ATTEMPTS,
        signing_block,
        // #400 R2 — the row-state bit, straight from the reader. A `Submitting` row can ALSO be
        // at its retry cap (`paused`); presentation resolves that by letting `sending` win, since
        // "preparing" is the true present tense and the cap only governs the NEXT attempt.
        sending,
        binding: intent.binding,
    }))
}

/// The committed gross outbound amount of a stored request (sum of all legs; excludes the fee). A
/// queued send always has amounts (`queue_send` rejects an amount-less donation form), so `None` is
/// unreachable; it contributes 0 defensively. SATURATING — a parsed total is bounded by the supply,
/// decades from i64::MAX, but a hostile/garbled multi-leg URI never overflow-panics. Shared by the
/// parked (`Queued`) and in-flight (`Sent`) surface classifiers — one fold, one truth.
fn gross_outbound_zat(request: &zip321::TransactionRequest) -> i64 {
    request
        .payments()
        .values()
        .filter_map(|p| p.amount())
        .map(u64::from)
        .fold(0i64, |acc, v| {
            acc.saturating_add(i64::try_from(v).unwrap_or(i64::MAX))
        })
}

/// One IN-FLIGHT two-step send (§3.2i-2 #309): a `Sent` intent whose recorded group has ≥2 txs —
/// tx0 (the unshield to the wallet's ephemeral one-time address) was created and the send is not
/// yet complete. The row leaves `Sent` by exactly THREE exits — delete-at-FINAL-BURIED (done),
/// `Stranded` (tx0 mined+buried, tx1 expired — the recoverable surface takes over), or
/// `reset_to_queued` (EVERY tx expired unmined, e.g. a deep reorg — no money moved; back to
/// `Queued` and the parked surface, cancellable again) — so this row's lifetime IS the "payment
/// in motion through a one-time address" window: the host renders the DURABLE don't-re-send cue
/// from it (the interactive result screen's caution is dismissible; this one survives until the
/// send resolves). AMOUNT-only (no recipient, no txid — §5.4 never-RENDER); cause-agnostic and
/// locational, like every 2e-2b surface. NOT cancellable WHILE `Sent` (unlike [`ParkedSend`]) —
/// tx0 is signed/broadcast. NOTE the strand path is NOT a clean sequential handoff: tx1 typically
/// expires (~40 blocks) well before tx0 buries (100), so the recoverable surface can show the
/// funds while this row still waits `Sent`-for-burial — both cues showing together is correct
/// (both conservative: don't re-send + recover now).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct InFlightSend {
    /// The committed gross OUTBOUND amount in zatoshis (sum of the request's payment legs, excludes
    /// the fee). Host-rendered ("X on its way"); §5.4 NEVER-LOG (the `Debug` redacts it).
    pub amount_zat: i64,
    /// Unix seconds at enqueue (the stored `created_at`) — the host computes the age (no wall-clock
    /// in the reader). Display-only.
    pub created_at: i64,
}

// §5.4 belt: the amount must never `{:?}`-leak into a log (mirrors `ParkedSend`'s redacting Debug).
impl std::fmt::Debug for InFlightSend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InFlightSend")
            .field("amount_zat", &"<redacted §5.4>")
            .field("created_at", &self.created_at)
            .finish()
    }
}

/// Classify ONE `Sent` multi-step intent as an IN-FLIGHT send (pure, side-effect-free). `None` ⇒ a
/// swap deposit (its own quote-deadline lifecycle + the SWAP status surface — mirrors
/// [`classify`]'s exclusion; hiding it here hides no money). Unlike the parked classifier there is
/// NO TEX-recipient test: the row's ≥2-tx group is the MONEY fact (the queued-path shape guard
/// admits only the ZIP-320 two-step to signing, so a multi-tx `Sent` group ⟺ a TEX pair — same
/// tripwire as [`classify`]: the `send.rs` real-engine shape tests pin the equivalence). A torn
/// at-rest URI is corruption ⇒ [`StoreCorrupt`](WalletError::StoreCorrupt) — fail-closed + LOUD
/// (an in-motion send must surface, even as an error; §4.6).
pub(crate) fn classify_in_flight(
    intent: &crate::intent_store::SentMultiStepIntent,
) -> Result<Option<InFlightSend>, WalletError> {
    if intent.deposit_deadline.is_some() {
        return Ok(None);
    }
    let request =
        zip321::TransactionRequest::from_uri(&intent.uri).map_err(|_| WalletError::StoreCorrupt)?;
    Ok(Some(InFlightSend {
        amount_zat: gross_outbound_zat(&request),
        created_at: intent.created_at,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent_store::QueuedIntent;
    use crate::state::QueuedSendId;
    use zcash_address::{ToAddress, ZcashAddress};
    use zcash_protocol::consensus::NetworkType;
    use zcash_protocol::value::Zatoshis as ProtoZat;

    fn tex_recipient(hash: [u8; 20]) -> ZcashAddress {
        ZcashAddress::from_tex(NetworkType::Regtest, hash)
    }
    fn transparent_recipient(hash: [u8; 20]) -> ZcashAddress {
        ZcashAddress::from_transparent_p2pkh(NetworkType::Regtest, hash)
    }
    fn sapling_recipient(seed: &[u8]) -> ZcashAddress {
        let extsk = sapling_crypto::zip32::ExtendedSpendingKey::master(seed);
        let (_, addr) = extsk.default_address();
        ZcashAddress::from_sapling(NetworkType::Regtest, addr.to_bytes())
    }

    fn uri(legs: Vec<(ZcashAddress, u64)>) -> String {
        let payments = legs
            .into_iter()
            .map(|(addr, amount)| {
                zip321::Payment::without_memo(addr, ProtoZat::const_from_u64(amount))
            })
            .collect();
        zip321::TransactionRequest::new(payments)
            .expect("valid request")
            .to_uri()
    }

    fn intent(
        id: i64,
        uri: String,
        created_at: i64,
        deposit_deadline: Option<i64>,
    ) -> QueuedIntent {
        QueuedIntent {
            id: QueuedSendId::new(id),
            uri,
            created_at,
            deposit_deadline,
            repropose_attempts: 0,
            binding: None,
        }
    }

    #[test]
    fn classify_surfaces_a_queued_tex_send_amount_only() {
        // A queued TEX send is PARKED by the gate — surfaced with id + gross amount + created_at, NO
        // address (absent from the type by construction), tagged with the two-step kind.
        let it = intent(
            7,
            uri(vec![(tex_recipient([0x04; 20]), 50_000)]),
            1_700_000_123,
            None,
        );
        let parked = classify(&it, false, None)
            .expect("parses")
            .expect("a TEX send is parked");
        assert_eq!(parked.id, 7);
        assert_eq!(parked.kind, ParkedSendKind::TwoStep);
        assert_eq!(parked.amount_zat, 50_000);
        assert_eq!(parked.created_at, 1_700_000_123);
    }

    #[test]
    fn classify_sums_all_legs_of_a_mixed_tex_send() {
        // A mixed TEX + shielded send is still a two-step (the engine builds it around the TEX leg);
        // the surfaced amount is the GROSS committed total across both legs.
        let it = intent(
            8,
            uri(vec![
                (tex_recipient([0x01; 20]), 30_000),
                (sapling_recipient(b"mix-seed"), 12_500),
            ]),
            1_700_000_000,
            None,
        );
        let parked = classify(&it, false, None)
            .expect("parses")
            .expect("a mixed TEX send is parked");
        assert_eq!(
            parked.kind,
            ParkedSendKind::TwoStep,
            "ANY TEX leg makes it a two-step"
        );
        assert_eq!(parked.amount_zat, 42_500, "gross sum of both legs");
    }

    #[test]
    fn classify_surfaces_a_queued_single_step_send_with_its_kind() {
        // #331: a plain shielded / plain (non-TEX) transparent queued send SURFACES too — it is a
        // committed spend the user must be able to see and cancel across a relaunch (invisible, it
        // was a double-pay window: offline-queue → process death → still-offline relaunch → the
        // user re-enters it → both drain on reconnect). The SingleStep kind carries the "drains
        // normally, nothing is stuck" nuance — presentation owns that phrasing, not this reader.
        // (This test REPLACES the pre-#331 pin that these rows were deliberately hidden.)
        let shielded = intent(1, uri(vec![(sapling_recipient(b"s"), 10_000)]), 5, None);
        let s = classify(&shielded, false, None)
            .expect("parses")
            .expect("a queued shielded send surfaces");
        assert_eq!(s.kind, ParkedSendKind::SingleStep);
        assert_eq!((s.id, s.amount_zat, s.created_at), (1, 10_000, 5));
        let transparent = intent(
            2,
            uri(vec![(transparent_recipient([0x09; 20]), 10_000)]),
            6,
            None,
        );
        let t = classify(&transparent, false, None)
            .expect("parses")
            .expect("a queued plain-transparent send surfaces");
        assert_eq!(
            t.kind,
            ParkedSendKind::SingleStep,
            "non-TEX transparent is single-step"
        );
    }

    fn sent_intent(
        uri: String,
        created_at: i64,
        deposit_deadline: Option<i64>,
    ) -> crate::intent_store::SentMultiStepIntent {
        crate::intent_store::SentMultiStepIntent {
            uri,
            created_at,
            deposit_deadline,
        }
    }

    #[test]
    fn classify_in_flight_surfaces_the_gross_amount_and_age() {
        // A Sent two-step (tx0 out, send not complete) surfaces amount + created_at — NO recipient,
        // NO txid (absent from the type by construction, §5.4). The gross sum covers every leg (a
        // mixed TEX + shielded send rode the same two-step).
        let it = sent_intent(
            uri(vec![
                (tex_recipient([0x05; 20]), 80_000),
                (sapling_recipient(b"in-flight"), 5_000),
            ]),
            1_700_000_777,
            None,
        );
        let s = classify_in_flight(&it).expect("parses").expect("in flight");
        assert_eq!(s.amount_zat, 85_000, "gross sum of both legs");
        assert_eq!(s.created_at, 1_700_000_777);
    }

    #[test]
    fn classify_in_flight_excludes_a_swap_deposit() {
        // A swap deposit owns its own quote-deadline lifecycle + the SWAP status surface — never
        // double-surfaced here (mirrors the parked classifier's exclusion).
        let it = sent_intent(
            uri(vec![(tex_recipient([0x06; 20]), 91_000)]),
            1_700_000_000,
            Some(1_700_100_000),
        );
        assert!(classify_in_flight(&it).expect("parses").is_none());
    }

    #[test]
    fn classify_in_flight_fails_closed_on_a_torn_uri() {
        // A torn at-rest URI is corruption — LOUD (an in-motion send must surface, even as an
        // error), never a silent drop.
        let it = sent_intent("zcash:not-a-valid-uri".into(), 1, None);
        assert!(matches!(
            classify_in_flight(&it),
            Err(WalletError::StoreCorrupt)
        ));
    }

    #[test]
    fn in_flight_send_debug_redacts_the_amount() {
        // §5.4 belt: the amount must never {:?}-leak (mirrors ParkedSend's redacting Debug).
        let s = InFlightSend {
            amount_zat: 123_456,
            created_at: 42,
        };
        let dbg = format!("{s:?}");
        assert!(!dbg.contains("123456"), "amount redacted: {dbg}");
        assert!(dbg.contains("redacted"), "redaction marker present: {dbg}");
        assert!(dbg.contains("42"), "created_at stays loggable: {dbg}");
    }

    #[test]
    fn classify_pauses_a_row_at_the_attempt_cap_and_not_below() {
        // The #315 honesty bit: a row AT (or past — defensive) the ephemeral-reserving attempt
        // cap surfaces `paused = true` (the drain will skip it; "will send when ready" would be a
        // lie); a row below the cap — including the fresh 0 — stays a healthy pending send. The
        // SAME `>=` as the drain's gate, so surface and behavior can never disagree.
        let at_cap = |attempts: i64| QueuedIntent {
            repropose_attempts: attempts,
            ..intent(9, uri(vec![(tex_recipient([0x07; 20]), 25_000)]), 50, None)
        };
        let cap = crate::constants::MAX_TEX_REPROPOSE_ATTEMPTS;
        assert!(
            !classify(&at_cap(0), false, None)
                .expect("parses")
                .expect("parked")
                .paused,
            "a fresh TEX intent is not paused"
        );
        assert!(
            !classify(&at_cap(cap - 1), false, None)
                .expect("parses")
                .expect("parked")
                .paused,
            "below the cap the drain still attempts — not paused"
        );
        assert!(
            classify(&at_cap(cap), false, None)
                .expect("parses")
                .expect("parked")
                .paused,
            "AT the cap the drain skips — paused (the same >= as prepare_queued)"
        );
        assert!(
            classify(&at_cap(cap + 1), false, None)
                .expect("parses")
                .expect("parked")
                .paused,
            "past the cap stays paused (defensive)"
        );
        // A single-step row is structurally never paused (only a two-step ever increments), and
        // the paused row keeps its kind — pausing is a status, not a shape.
        let single = intent(1, uri(vec![(sapling_recipient(b"s"), 10_000)]), 5, None);
        assert!(
            !classify(&single, false, None)
                .expect("parses")
                .expect("parked")
                .paused
        );
        assert_eq!(
            classify(&at_cap(cap), false, None)
                .expect("parses")
                .expect("parked")
                .kind,
            ParkedSendKind::TwoStep,
            "a paused row keeps its TwoStep kind"
        );
    }

    #[test]
    fn classify_excludes_a_swap_deposit_even_if_it_is_a_tex_recipient() {
        // A swap-deposit intent (deposit_deadline set) owns its own quote-deadline lifecycle — never a
        // "parked TEX send", regardless of recipient kind.
        let it = intent(
            3,
            uri(vec![(tex_recipient([0x02; 20]), 99_000)]),
            0,
            Some(1_700_100_000),
        );
        assert!(
            classify(&it, false, None).expect("parses").is_none(),
            "a swap deposit is excluded from the parked surface",
        );
    }

    #[test]
    fn classify_propagates_a_torn_uri_as_corruption() {
        // A stored URI that no longer parses (it was validated at enqueue) is corruption — fail-closed +
        // LOUD, never a silent drop (a parked send the user awaits must surface, even as an error).
        let it = intent(4, "not a valid zip321 uri".to_string(), 0, None);
        assert!(matches!(
            classify(&it, false, None),
            Err(WalletError::StoreCorrupt)
        ));
    }

    #[test]
    fn reservation_pressure_stamps_the_engine_gap_limit_and_flags_the_ceiling() {
        let p = ReservationPressure::new(3);
        assert_eq!(p.outstanding, 3);
        assert_eq!(p.limit, crate::constants::EPHEMERAL_GAP_LIMIT);
        assert!(!p.at_ceiling(), "3 of 10 is below the ceiling");
        // AT the limit ⇒ the host warns (conservative `>=`).
        assert!(ReservationPressure::new(crate::constants::EPHEMERAL_GAP_LIMIT).at_ceiling());
        assert!(ReservationPressure::new(crate::constants::EPHEMERAL_GAP_LIMIT + 1).at_ceiling());
        // No TEX in flight: 0 outstanding is never at the ceiling.
        assert!(!ReservationPressure::new(0).at_ceiling());
    }

    #[test]
    fn reservation_pressure_emit_is_silent_with_no_pressure_and_does_not_panic() {
        // No TEX two-step in flight: 0 outstanding ⇒ no span (`emit` early-returns). A non-capturing
        // smoke that `emit` does not panic on either shape (the span target/fields are §5.4-guarded by the
        // `tracing_guard` allowlist test).
        ReservationPressure::new(0).emit();
        ReservationPressure::new(crate::constants::EPHEMERAL_GAP_LIMIT).emit();
    }

    #[test]
    fn parked_send_debug_redacts_the_amount() {
        // §5.4 belt: the money figure must never `{:?}`-leak. The kind is chain-free shape
        // metadata — it stays loggable.
        let dbg = format!(
            "{:?}",
            ParkedSend {
                id: 5,
                kind: ParkedSendKind::SingleStep,
                amount_zat: 123_456,
                created_at: 9,
                paused: false,
                sending: false,
                binding: None,
                signing_block: None,
            }
        );
        assert!(
            dbg.contains("<redacted §5.4>"),
            "amount is redacted in Debug"
        );
        assert!(
            !dbg.contains("123456") && !dbg.contains("123_456"),
            "no raw amount"
        );
        assert!(
            dbg.contains("id: 5"),
            "the id is still shown (loggable metadata)"
        );
        assert!(
            dbg.contains("SingleStep"),
            "the kind is loggable shape metadata"
        );
    }
}
