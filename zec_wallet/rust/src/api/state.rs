//! Wallet state DTOs as Dart sees them (spec §2.5, frozen pre-W3 per review
//! G1/G2). Amounts cross the bridge as `int` **zatoshis** — field names carry
//! the `zat` suffix so the unit is visible at every Dart call site (max
//! supply ≈ 2.1e15 < 2^53, so the value is exact even under JS-number
//! semantics; spec §2.1).

/// Balance breakdown. `transparentZat` is REAL, privacy-relevant user state
/// (drives the host's shield banner); a nonzero value after a failed
/// auto-shield must stay visible.
pub struct BalanceSnapshot {
    /// Confirmed at depth, spendable now (zatoshis).
    pub spendable_zat: i64,
    /// Received but not yet spendable (zatoshis): still below the confirmation
    /// depth, OR waiting for more of the chain to be scanned before its witness
    /// can be built — the common case while the wallet is catching up, whatever
    /// the note's confirmation count. Render as "arriving" / "not yet spendable",
    /// never as "confirming" (core `BalanceSnapshot::pending_incoming`).
    pub pending_incoming_zat: i64,
    /// Our own change in flight (zatoshis).
    pub pending_change_zat: i64,
    /// Unshielded funds (zatoshis). Any funds reported by
    /// [WalletHandle.recoverableEphemeralFunds] are ALREADY folded in here (and into `totalZat`) —
    /// that surface is a SUBSET of this value, NEVER additional funds. See
    /// [RecoverableEphemeralFunds] for the never-additive presentation contract.
    pub transparent_zat: i64,
    /// Total of the above (zatoshis).
    pub total_zat: i64,
}

/// Funds recoverable on ONE wallet-controlled *one-time* (ephemeral) transparent address — a TEX
/// transfer whose forwarding step expired, OR an exchange that returned a deposit to that single-use
/// address. AMOUNT-ONLY: the address is wallet-internal and is NEVER surfaced.
///
/// PRESENTATION CONTRACT: `recoverableZat` is a SUBSET of the balance the user already sees
/// ([BalanceSnapshot.transparentZat]/[BalanceSnapshot.totalZat] fold the SAME funds). Render it as
/// part of the balance — e.g. "X of your balance is on a one-time address (recover it)" — NEVER as
/// "+X recoverable": adding it to the balance over-counts holdings 2×. Cause-agnostic by design (it is
/// an exchange return as much as an expired transfer), so the copy stays locational, never
/// "stranded"/"bounced".
pub struct RecoverableEphemeralFunds {
    /// Economically-recoverable amount on this one-time address (zatoshis; uneconomic dust excluded).
    /// A SUBSET of the displayed balance — never additive.
    pub recoverable_zat: i64,
    /// `true` iff the WHOLE `recoverableZat` is buried beyond the reorg horizon (reorg-final). When
    /// `false` the amount is still confirming — render it as pending recovery, NEVER as settled/ready.
    /// Finality is relative to the wallet's LAST-SYNCED tip (conservative: a behind sync only ever
    /// under-reports finality, never over), so gate any "recover now" affordance on an up-to-date
    /// [SyncStatus] and RE-PULL rather than caching this bool to observe the `false → true` transition.
    pub is_final: bool,
}

/// COUNTS-only outcome of one [WalletHandle.sweepEphemeralFunds] invocation — the manual recovery of
/// funds stranded on wallet-controlled *one-time* (ephemeral) transparent addresses (the funds
/// [RecoverableEphemeralFunds] surfaces, PLUS late returns + a 2nd deposit the automatic surface cannot
/// see — it RAW-enumerates on-chain rather than reading the recognised balance). The one-time addresses
/// are wallet-internal and NEVER cross the bridge (absent by construction).
///
/// `recoveredZat` is the AGGREGATE net (gross − fee) consolidated into the wallet's own SHIELDED
/// balance — the ONLY money figure here. It is counted ONLY for an ephemeral whose sweep tx the
/// endpoint ACCEPTED (so it never optimistically claims funds that did not move; a re-run is idempotent
/// — the engine excludes already-spent UTXOs). It is **PROVISIONAL — pending, not settled**: a sweep
/// can be accepted then evicted / expire before mining, so treat `recoveredZat` like
/// [RecoverableEphemeralFunds] before `isFinal` — render it as "recovery submitted, pending" until the
/// next sync reflects it in the scanned shielded balance.
///
/// PRESENTATION — DO NOT reconcile the balance yourself from `recoveredZat`; the **next sync's scanned
/// balance is the SSOT**. The funds move into the shielded pool, but how the displayed TOTAL changes
/// depends on the case: a recognised stranded amount (already folded into [BalanceSnapshot.transparentZat])
/// moves transparent→shielded with NO change to the total; a late return / 2nd deposit the wallet had
/// NEVER recognised was NOT in the displayed total, so the total genuinely RISES. So render
/// `recoveredZat` as "recovered X (pending)" and let the scanned balance settle the headline — never
/// add it to, nor assume it leaves unchanged, the displayed balance.
///
/// `truncated > 0` ⇒ the per-invocation cap/budget left work; run it again (nothing was silently
/// dropped). `failed > 0` ⇒ a transport/transient fault left some funds on-chain + re-runnable (NOT a
/// loss; usually a flaky/censored link — run recovery again).
pub struct EphemeralSweepSummary {
    /// One-time addresses enumerated this invocation (the full reserved set, before the cap).
    pub scanned: u32,
    /// Of those, SWEPT — a recovery tx signed, persisted, and accepted by the endpoint (PROVISIONAL —
    /// pending confirmation in the scanned balance).
    pub swept: u32,
    /// AGGREGATE net recovered into the shielded balance (zatoshis) — PROVISIONAL until the next scan.
    /// The only money figure; never per-address. Do NOT reconcile the displayed balance from it (the
    /// scanned balance is the SSOT — a recognised amount moves transparent→shielded, an unrecognised
    /// late return RAISES the total).
    pub recovered_zat: i64,
    /// Per-address faults isolated (query/sign/broadcast). The funds stay on-chain + re-runnable —
    /// NEVER a loss; a non-zero count means "run recovery again" (usually a transport/link issue).
    pub failed: u32,
    /// Addresses NOT reached because the per-invocation cap/budget truncated the set — run again to
    /// recover the rest (never a silent drop).
    pub truncated: u32,
}

/// The outcome of one [WalletHandle.reclaimEphemeralSlots] (#315) — reopening a one-time-address
/// (TEX) send window that leaked reservations have bricked, by self-minting a small amount from the
/// wallet's own shielded balance to the highest provably-abandoned one-time address (mining it frees
/// the whole window at once). §5.4: COUNTS / one-fixed-amount only — no address or txid crosses.
///
/// HOST CONTRACT (money-relevant): reclaim is EXPLICIT + user-initiated — gate it behind the #327
/// authorizer with the honest-cost disclosure (the mint + the later recovery are TWO transactions,
/// ~4 network fees from the user's funds; the minted principal returns to the wallet). It unblocks
/// the WINDOW only — it NEVER re-sends a paused/queued payment (no double-pay); a paused send stays
/// paused until the user retries it. Do NOT auto-loop it.
pub enum ReclaimOutcome {
    /// Nothing to reclaim right now: either no one-time-address send ever reserved an address, or
    /// every outstanding reservation is too recent to be provably abandoned — the window is
    /// genuinely transient (those slots may still free on their own, and minting on one could cancel
    /// a legitimate in-flight payment, so the wallet declines). Render "nothing to recover"; NO money
    /// moved.
    NothingToReclaim,
    /// A recovery mint was signed, saved, and ACCEPTED by the network, paying `amountZat` from your
    /// shielded balance. This is INITIATED, not finished: one-time-address sends start working again
    /// once it CONFIRMS (a few minutes), and the `amountZat` principal returns to your wallet via
    /// Recover funds once confirmed. Disclose the honest cost (~4 network fees across two
    /// transactions) and the "recover the moved amount once it confirms" step — never claim it is
    /// done, and never imply the funds vanished.
    Minted {
        /// The recovery-mint principal moved this call (a small fixed amount; host-rendered, never
        /// logged). It is NOT a fee — it returns to the wallet; only the two transactions' fees cost.
        amount_zat: i64,
    },
    /// The recovery mint was prepared but could not reach the network (a flaky/censored link).
    /// MONEY-SAFE: it was never sent, expires on its own, and frees the funds — just try again.
    /// `amountZat` never left the wallet.
    NotBroadcast {
        /// The attempted mint principal (never spent — the transaction did not reach the network).
        amount_zat: i64,
    },
    /// Forward-compatibility arm — an outcome this bridge does not know yet (the core
    /// `ReclaimOutcome` is `#[non_exhaustive]`). Unreachable in a lockstep build: CI's
    /// `bridge_enums_cover_core_variants` fails until this bridge learns any new core variant, so
    /// the `_ =>` converter arm never silently absorbs one. Render it as a neutral "recovery
    /// finished — check your one-time-address sends", never an error.
    Unknown,
}

/// The result of one [WalletHandle.checkOlderSwapAddresses] deep scan (#390) — the
/// user-triggered "Check older swap addresses" recovery of a seed-only-restore's older
/// swap deposits/refunds. COUNTS ONLY (§5.4): no address or index ever crosses.
///
/// HOST CONTRACT: the scan does NOT itself find funds — it WIDENS the range the wallet
/// watches, then the normal sync surfaces any older swap deposits/refunds into the
/// balance over the next minutes. So render "we're checking older swap addresses as your
/// wallet syncs — anything found will appear in your balance", NEVER "found X". A refusal
/// throws typed ([WalletErrorKind.swapAddressCheckRefused]) instead of returning this —
/// it never silently no-ops.
pub struct SwapAddressCheckReport {
    /// How many more single-use addresses this run added to the checked range. `> 0` on
    /// every accepted run (reruns go deeper — the op is not idempotent).
    pub widened_by: u32,
    /// How many of the wallet's swaps the checked range now spans (each swap — either
    /// direction, including quotes the user never completed — used one address). The
    /// coverage line's "your first {n} swaps" number.
    pub covered_swaps: u32,
    /// Addresses still to be registered + polled before the newly-widened range is fully
    /// checked. `0` = the checked range is registered and has been polled at least once
    /// (found funds surface via the normal balance path, never asserted here).
    pub pending: u32,
}

/// The render-only coverage read for the "Check older swap addresses" sheet (#390) —
/// how far the wallet has already checked, for the sheet's coverage line and its
/// Check/Check-deeper affordance. COUNTS ONLY (§5.4).
pub struct SwapAddressCoverage {
    /// How many of the wallet's swaps the checked range spans (see
    /// [SwapAddressCheckReport.coveredSwaps]).
    pub covered_swaps: u32,
    /// Addresses still to be registered + polled (`0` = the honest "done" — nothing
    /// outstanding right now).
    pub pending: u32,
}

/// What `authorizeParkedSend` actually did (FR-23-b). Every arm is a money-SAFE end state — there
/// is no "failed" arm that leaves the send ambiguous, so a host never has to guess whether money
/// moved.
///
/// HOST CONTRACT (honesty — money-relevant): only `signed` may be phrased as "sending now".
/// `stillQueued` is NOT an error — nothing failed, nothing was lost; the send just did not become
/// sendable at this moment. Phrase it "still waiting" and LEAVE the row on the parked surface;
/// phrasing it as a failure invites the user to re-enter the payment, which is a DOUBLE-PAY.
pub enum ParkedAuthorization {
    /// SIGNED inside your authorization bracket: the send is now a real transaction on its way to
    /// the network. It LEAVES the parked list and appears in activity / in-flight from here.
    /// (Network delivery itself is still best-effort + retried — the wallet owns that.)
    Signed,
    /// Nothing was signed; the send STAYS parked exactly as it was — funds untouched, nothing
    /// on-chain, still cancellable, and authorizable again later. Common causes: not enough
    /// spendable balance at this instant, a one-time-address window that is full, or the wallet's
    /// own background pass having just claimed the row.
    StillQueued,
    /// A deadline-tagged (swap-deposit) row whose quote had already lapsed: TERMINAL, discarded
    /// rather than sent late. Not reachable from `listParkedSends` (which excludes deposit rows).
    Expired,
    /// No parked send matched the `(id, createdAt)` you passed — already sent / cancelled / its
    /// rowid was reused. Idempotent, never an error: re-read `listParkedSends`. Do NOT present it
    /// as "cancelled" nor invite a re-send — a row that began sending has LEFT this surface, so
    /// the payment may be IN-FLIGHT and a re-send risks a DOUBLE-PAY.
    NotFound,
    /// Forward-compatibility arm — an outcome this bridge doesn't know yet. Treat it exactly like
    /// `stillQueued`: re-read the parked list and say nothing about money having moved.
    Unknown,
}

/// The SHAPE of a parked send — presentation-only nuance for the host's copy. Both kinds are
/// equally committed, equally not-yet-attempted, and equally cancellable; nothing about either is
/// an error state.
pub enum ParkedSendKind {
    /// A one-time-address (TEX/ZIP-320) send the wallet performs as a two-step — it sends when it
    /// can, or waits while the one-time-address window is full. May sit here for a while even
    /// online; "saved & pending" copy fits.
    TwoStep,
    /// A plain send waiting in the offline queue — it sends at the next connectivity. Nothing is
    /// "stuck"; phrase it as an ordinary queued payment.
    SingleStep,
    /// Forward-compatibility arm — a shape this bridge doesn't know yet. Render the generic
    /// "saved & pending" copy (the row is still a committed, cancellable queued send); never an
    /// error state.
    Unknown,
}

/// One queued send sitting in `Queued` that has NOT completed yet — a one-time-address (TEX)
/// two-step awaiting its drain pass / ceiling-parked, OR a plain single-step send waiting in the
/// offline queue (`kind` says which — presentation-only). Surfaced so the host shows it "saved &
/// pending" instead of NOTHING: it has no on-chain transaction yet, so it appears NOWHERE in the
/// activity / transaction list — across an app relaunch this surface is the ONLY place the
/// committed spend exists (hidden, it is a DOUBLE-PAY window: the user re-enters the payment and
/// both send on reconnect). AMOUNT-only — the recipient address NEVER crosses the bridge (absent
/// by construction); the copy is cause-agnostic (never "blocked"/"stuck"). These DTOs are
/// read-only; the SDK CANCEL (`cancelParkedSend`) landed in v-4a (pass back this `id` +
/// `createdAt`; it is kind-agnostic) — the host cancel button / dismiss UI is v-4b.
///
/// ⚠ DOUBLE-PAY CAUTION (host contract — money-relevant). A parked send is a DURABLE intent that
/// auto-broadcasts on its own once it can drain (a reservation slot frees) — it is NOT inert and NOT
/// abandoned. Present it as a committed send-that-will-go, with NO "saved" framing that invites the user to "try
/// another way": re-sending the same payment by another route risks PAYING TWICE when the path turns on.
/// Do NOT offer a re-send / alternative-route action against a row shown here. The SAFE counter-affordance
/// is CANCEL (`cancelParkedSend`, landed v-4a) — DISCARD the intent (it deletes the still-`Queued` send,
/// which has reserved no notes and moved no funds), NEVER re-route it. A `false` from `cancelParkedSend`
/// does NOT mean "cancelled": post-enable the send may already be IN-FLIGHT (it left this surface) — re-read
/// here, but never present `false` as cancelled nor invite a re-send (direct the user to the activity list).
///
/// BALANCE RELATIONSHIP (both directions — mirror [RecoverableEphemeralFunds]'s two-sided precision). A
/// queued intent RESERVES NO notes and has NO tx0 (the proposal/sign happens at send time, not queue
/// time), so `amountZat` is STILL fully present in [BalanceSnapshot.spendableZat]. Render it as an
/// EARMARK shown OVER the existing balance — NEVER add it (it is not extra funds) and NEVER deduct/earmark
/// it OUT of spendable (that would hide the user's own still-accessible funds). "Committed" here means
/// "committed to send", not "reserved/locked".
pub struct ParkedSend {
    /// Opaque queued-send id — pass it (with `createdAt`) to `cancelParkedSend`; carries no money data
    /// itself.
    pub id: i64,
    /// The shape of this parked send — drives PHRASING only (see [ParkedSendKind]); both kinds are
    /// equally committed and equally cancellable.
    pub kind: ParkedSendKind,
    /// The committed GROSS outbound amount (zatoshis; the sum of the request's legs, excludes the fee).
    /// Render "X saved & pending" as an earmark over the balance; the amount is STILL in spendable (no
    /// note is reserved) — NEVER add it to, NOR deduct it from, the displayed spendable balance.
    pub amount_zat: i64,
    /// Unix seconds at enqueue — DISPLAY-ONLY; compute the age host-side.
    pub created_at: i64,
    /// `true` iff this send is PAUSED: the wallet gave up auto-retrying it (each retry of a
    /// one-time-address send permanently uses up one of a small number of address slots, so
    /// retries are capped) and it will NOT send on its own until the user acts. HOST CONTRACT
    /// (honesty — money-relevant): a paused row MUST be phrased differently from a healthy
    /// pending one — "paused — your funds are safe; retry or cancel", NEVER "will send when
    /// ready" (a paused send never sends by itself; a user told to wait waits forever). Offer
    /// BOTH affordances: `retryParkedSend` (resume the SAME send — the preferred path; funds are
    /// untouched either way) and `cancelParkedSend` (discard it). Do NOT auto-retry a paused
    /// send, and NEVER re-create it as a fresh send while this row exists (double-pay + it
    /// silently evades the retry cap). `false` for every healthy queued row of either kind.
    pub paused: bool,
    /// `true` iff this row is MID-SIGNATURE: the wallet claimed it and is building/proving the
    /// transaction right now — or was, until the app was killed (#400 R2).
    ///
    /// WHY THE HOST SEES IT. The claim is written BEFORE the proving step, which is the slow,
    /// memory-hungry part of a send and therefore the most likely place for the OS to kill a
    /// mobile app. Before this flag such a row appeared on NO surface — not in the parked list,
    /// not in `listInFlightSends`, and not in the transaction list (no transaction exists yet) —
    /// so a committed spend went invisible and the user re-entered it: a DOUBLE PAY.
    ///
    /// HOST CONTRACT (honesty — money-relevant): render it as PREPARING ("preparing to send…"),
    /// never as "failed" (nothing failed) and never as "sent" (nothing was broadcast; there may
    /// not even be a transaction).
    ///
    /// OFFER NO ACTION ON THIS ROW. `authorizeParkedSend` requires a still-queued row and can only
    /// answer `notFound`. `cancelParkedSend` is queued-guarded too, so it can only return `false` —
    /// and it sits behind a confirm dialog that promises the payment "hasn't been sent, so nothing
    /// leaves your wallet". For this row that promise may be untrue (see the balance note), and a
    /// `false` return then contradicts the dialog the user just accepted.
    ///
    /// ⚠ THE [BALANCE RELATIONSHIP] ABOVE DOES NOT HOLD FOR A `sending` ROW. Every other parked row
    /// reserves no notes, so its `amountZat` is still fully present in [BalanceSnapshot.spendableZat]
    /// and you render it as an earmark OVER the balance. This row can be past the point where the
    /// wallet committed the transaction locally and marked its notes spent — so the amount may
    /// ALREADY be out of spendable. The funds are safe either way; just do not tell the user the
    /// amount is still part of their balance on this row specifically.
    ///
    /// The wallet recovers such a row by itself on its next completed sync pass (it re-queues an
    /// unfinished claim), so this normally clears without the user doing anything. A host that
    /// has turned syncing OFF has no next pass — for it, the row stays `sending` until syncing
    /// is re-enabled, which is what the copy must let the user understand.
    pub sending: bool,
    /// FR-17 (#396): the row's spend-binding nonce (32 opaque bytes), or null for a
    /// row queued before FR-17. A host-custody re-stage flow records it when the user
    /// re-authorizes THIS parked send — the background drain's native seed pull
    /// presents the row's value, so the stage can never be consumed signing another
    /// row. Hosts without a native seed port can ignore it. Do not log it.
    ///
    /// The re-stage handoff (host-custody offline queue): a bound host that wants the
    /// background drain to sign a queued send must record THIS row's `binding` (keyed
    /// by `id`, the value `queueSend` returned) at its re-authorize step, then stage
    /// the seed FOR that binding. A stage recorded for any other row (or unbound) is
    /// refused by strict `Option` equality — the send stays parked (funds safe), never
    /// wrongly signed. Requires the BOUND C-ABI registration (see `SendProposal`).
    pub binding: Option<Vec<u8>>,
    /// WHY the wallet will NOT sign this row, if it will not — null when the
    /// row will send on its own on the next sync pass that allows it (also the
    /// reading for a wallet that has not completed its first pass yet: that
    /// pass checks the network before it drains). Every queued row carries the
    /// same value — the condition is about the APP VERSION or THIS SERVER, not
    /// this payment.
    ///
    /// HOST CONTRACT (honesty — money-relevant): phrase each [SigningBlock]
    /// distinctly — "waiting for an app update" ONLY for
    /// [SigningBlock.networkUpgrade]; "waiting for a server that reports the
    /// network version — switch servers" for [SigningBlock.graceExpired].
    /// NEVER "will send when ready" (it will not, as things stand) and NEVER
    /// "failed" (the money is untouched and the intent is intact). Keep
    /// `cancelParkedSend` available; do NOT offer `retryParkedSend` while a
    /// block stands — retrying changes nothing until the cause is gone.
    pub signing_block: Option<SigningBlock>,
}

/// One IN-FLIGHT two-step (TEX/ZIP-320) send: its first transaction (the unshield to a
/// wallet-controlled one-time address) is signed + broadcast and the send has NOT completed — the
/// user's money is IN MOTION through that one-time address. From `listInFlightSends`; exists so the
/// host renders a DURABLE "on its way — don't send it again" cue: the post-send result screen's
/// caution is dismissible, and without this surface an in-flight two-step is indistinguishable from
/// an ordinary pending send exactly during the window a worried user is most tempted to re-pay.
///
/// LIFECYCLE (when a row appears/disappears — the host never computes this): it appears once the
/// two-step is signed (interactive send or queued drain), and leaves by exactly THREE exits:
/// (a) the whole chain mining + settling (the send completed); (b) the strand transition (the
/// forward expired after the unshield mined — the funds surface via `recoverableEphemeralFunds` /
/// the recover-now sweep, which owns the user's next step; NOTE the two surfaces can OVERLAP:
/// the forward typically expires well before the unshield settles, so the recoverable funds may
/// show while this row is still listed — both cues together is correct, both conservative);
/// (c) a RETURN TO THE QUEUE (every tx expired unmined, e.g. a deep reorg — NO money moved; the
/// send reappears in `listParkedSends`, where it is cancellable again). So "disappeared from this
/// list" does NOT by itself mean "delivered" — check the other two surfaces before reporting
/// success. NOT cancellable while listed HERE (unlike [ParkedSend] — the first leg is broadcast).
///
/// DOUBLE-PAY CAUTION (host contract): render it as a payment in progress + "don't send it again";
/// NEVER offer a re-send / retry / alternative-route action against a row shown here. Cause-agnostic,
/// purely locational copy ("moving through a one-time address your wallet controls").
///
/// AMOUNT-ONLY (§5.4): no recipient, no txid, no address crosses the bridge. The amount is the
/// committed gross outbound total; the corresponding tx0 also appears in `transactions` as a pending
/// send, so do NOT double-count this as additional outbound money — it annotates that same payment.
pub struct InFlightSend {
    /// The committed GROSS outbound amount (zatoshis; the sum of the request's legs, excludes the
    /// fee). Host-rendered ("X on its way"); never logged.
    pub amount_zat: i64,
    /// Unix seconds at enqueue — DISPLAY-ONLY; compute the age host-side.
    pub created_at: i64,
}

/// The ephemeral reservation-PRESSURE gauge — a LOWER-BOUND count of how close the wallet is to the
/// engine's one-time-address (ZIP-320 ephemeral) gap-limit ceiling, beyond which a TEX send cannot
/// mint a fresh one-time address and PARKS. ⚠ `outstanding` is a LOWER BOUND, NOT a faithful "N of M"
/// (#315): the underlying engine read is BLIND to slots held by sends that were CREATED then never
/// confirmed (the common stuck shape — their output row exists from create-persist, so the read drops
/// them), so it can read `0` while the window is actually full. So NEVER drive an availability state
/// from it as if it were exact — surface it as a proactive "at least N of M in use" hint, and let the
/// engine's `TexSendLimitReached` at send time be the real gate. (The engine CEILING itself IS
/// dual-natured — some slots clear as transfers confirm, some never do — which is why the ceiling
/// COPY, distinct from this gauge, promises neither "just wait" nor doom.) COUNTS-only. An INDICATOR
/// for display, NEVER a money gate (the engine enforces the ceiling at reservation time); `0`
/// outstanding in the common case (no TEX send has reserved an address).
///
/// INDICATOR, NOT A GATE (host contract — like [RecoverableEphemeralFunds]'s `isFinal`). Drive any
/// availability STATE off `atCeiling`, never off the raw ratio. Do NOT use `atCeiling` to DISABLE the
/// send path — `limit` is a mirror of the engine default, so a stale-low mirror can flip `atCeiling`
/// EARLY and a hard-disabled button would block a send the engine would actually accept. Let the user
/// attempt; the engine's `TexSendLimitReached` is the real gate, and a send that does park is recoverable
/// (it appears in [WalletHandle.listParkedSends]). When rendering "N of M", CLAMP N to M for display —
/// if a future engine raises its real gap above the mirror, `outstanding` can exceed `limit` and a
/// verbatim "12 of 10" is incoherent (`atCeiling` stays correct via the welded `>=`).
pub struct ReservationPressure {
    /// Outstanding one-time-address reservations — a LOWER BOUND (#315): the underlying engine read
    /// cannot see slots held by sends that were created and then never confirmed (the common stuck
    /// shape), so this can read `0` while the window is actually full. Treat it as "at least N in
    /// use", never as proof the window is clear; the real gate stays the typed
    /// [WalletErrorKind.texSendLimitReached] at send time. MAY exceed `limit` if the engine default
    /// ever rises above the mirror — clamp to `limit` for an "N of M" display.
    pub outstanding: u32,
    /// The engine gap-limit denominator (the "N of M" M). An observability figure, not the enforcing bound.
    pub limit: u32,
    /// `true` iff `outstanding >= limit` — the next TEX send would park at the ceiling. Render the
    /// dual-natured ceiling WARNING (see the type doc — never a bare "temporarily unavailable", and
    /// never disable the send path). Welded in so the host can never render a money-relevant
    /// availability state by skipping the comparison.
    pub at_ceiling: bool,
}

/// A freshly minted PUBLIC diversified receive Unified Address (FR-8 / Recv-4, ADR-0537):
/// a distinct, unlinkable per-contact/per-invoice address that still credits the one wallet
/// account — payments to it are detected by the normal shielded scan (trial decryption is
/// diversifier-agnostic), including after a seed-only restore. Same receiver set as the
/// default address (Orchard + Sapling, no transparent), so no compatibility downgrade.
/// Every mint returns a NEW address (never-recycle counter, restore-surviving). Restore
/// behavior: each seed-only restore starts minting in a fresh RANDOM band (~2048 mints
/// wide), so no prior life's addresses are re-issued — deterministic for lives up to
/// ~2048 mints; a life that minted beyond that carries a ≤ ~2^-18 per-restore chance of
/// band overlap (an attribution/linkage caveat, never a fund risk). Minting is local,
/// offline, and cheap. Deterministic per index: `diversifierIndex`
/// re-derives the same address from the same seed forever — keep it as the host-side
/// attribution key (contact/invoice ↔ index). Treat the index as WALLET-INTERNAL data:
/// it is a sequential mint ordinal, so sharing it with a counterparty (or embedding it in
/// an outward-facing invoice identifier) leaks mint count and lets two counterparties
/// link their addresses to one wallet. §5.4: BOTH fields are display/store values — never
/// log them.
pub struct MintedDiversifiedAddress {
    /// The canonical encoded UA — render/copy/QR it exactly like the default address.
    pub address: String,
    /// The ZIP-32 diversifier index the address was derived at (≥ 2^40 — the SDK's frozen
    /// public-receive region; opaque to hosts beyond its use as a stable, wallet-internal
    /// attribution key).
    pub diversifier_index: u64,
}

/// Whether a confirmed transaction at `confirmationDepth` is reorg-FINAL — buried beyond the deepest
/// reorg the wallet auto-recovers. The SDK owns the reorg horizon: pass the `depth` from
/// [TxStatus.confirmed] and never hard-code a block count. (`depth = tip − mined + 1`, so finality is
/// `depth > horizon`, one block past the horizon.) Pure + sub-millisecond ⇒ `#[frb(sync)]`.
#[flutter_rust_bridge::frb(sync)]
pub fn tx_confirmation_is_final(confirmation_depth: u32) -> bool {
    crate::convert::tx_confirmation_is_final(confirmation_depth)
}

/// When the balances/history shown were last chain-confirmed. `at` is unix
/// seconds, DISPLAY-ONLY (the SDK's internal timing is monotonic).
pub struct SyncStamp {
    /// Chain height of the stamp.
    pub height: u32,
    /// Unix seconds, display-only (`i64`: fits until year ~292e9; keeps the
    /// Dart side a plain `int` instead of `BigInt`).
    pub at: i64,
}

/// What `syncFor` reports once its one pass returns (FR-40). Both heights are
/// read from the wallet's database after the pass.
pub struct BoundedSync {
    /// Every block at or below this height is scanned (the fully-scanned
    /// frontier). `null` before any block is scanned.
    pub scanned_to: Option<u32>,
    /// The chain tip the wallet has recorded — this pass's, once it reached
    /// the server. `null` before any pass has.
    pub tip: Option<u32>,
    /// The pass reached the tip before its budget ran out (and no `stopSync`
    /// cut it). `false` is not an error: the progress made is kept, and the
    /// next pass resumes from `scannedTo`.
    pub finished: bool,
    /// The resubmission of queued and in-flight sends ran to its end. `false`
    /// when the budget left after the pass did not cover its worst case, or
    /// the pass did not finish — the next `startSync` pass carries it.
    pub resubmitted: bool,
}

/// Honest-degradation sync state. Every variant is renderable; a silent hang
/// is a bug by contract (streams never die — faults surface as data).
///
/// SDK upgrades may ADD variants: always keep a default arm when switching
/// (render [SyncStatus.unknown] and anything unmatched neutrally).
pub enum SyncStatus {
    /// Wallet open, sync not started.
    Idle,
    /// Connecting; on a first launch with built-in Tor, bootstrap can take
    /// minutes — `torBootstrapPercent` keeps it honest instead of opaque.
    /// CONTRACT for the core (sync engine, increment 2+): every percent
    /// field crossing this bridge is FINITE — a NaN would silently break
    /// the generated Dart value-equality (IEEE NaN != NaN), turning every
    /// state update into a spurious UI rebuild.
    Connecting { tor_bootstrap_percent: Option<f32> },
    /// Scanning. `percent` is MONOTONIC scanned/total — never range position.
    /// `spendableReady` flips early (spend-before-sync): funds can be usable
    /// before 100%.
    Scanning {
        from: u32,
        to: u32,
        percent: f32,
        spendable_ready: bool,
        /// THIS pass has rewound at least once (a chain reorg un-scanned a
        /// span). First-class so a host header/latch reacts to the rewind
        /// explicitly instead of inferring it from a shrinking `to`; resets
        /// with the pass (the next pass starts `false`).
        rewound: bool,
    },
    /// Synced to `tip`.
    UpToDate { tip: u32 },
    /// Scanned as far as the chain goes, but this build could NOT fully
    /// interpret every block it passed: the network runs consensus rules this
    /// version does not implement.
    ///
    /// **Do not render this as "Up to date".** Balance shown alongside it is a
    /// FLOOR, not a total — value in a pool this version cannot model is
    /// invisible, and memos on transactions mined after the upgrade are
    /// unavailable. The honest copy is "caught up as far as this version can
    /// read — update to see everything"; sending is separately refused with
    /// `WalletErrorKind.networkUpgradeUnsupported` — the other two signing
    /// refusals (grace expired, never evaluated) are
    /// [SyncStatus.upToDateUnverified]'s, never this variant's.
    UpToDateLimited { tip: u32 },
    /// Scanned to the chain tip, but THIS SERVER did not fully serve every
    /// shielded pool: a pool it refuses, or serves impossible completion
    /// heights for, cannot have its notes witnessed from this server — funds
    /// received in that pool stay unspendable while everything else looks
    /// synced.
    ///
    /// **Do not render this as "Up to date".** The honest copy is "synced, but
    /// this server does not serve the affected pool — switch servers to use
    /// funds received there"; `pools` says which pool and how. The next step is
    /// SWITCH SERVERS — not "update the app" ([SyncStatus.upToDateLimited]) and
    /// not "check your connection" ([StallReason.endpointUnreachable]). Balance
    /// shown alongside is a FLOOR for the unserved pool. Clears by itself on the
    /// next pass on which every pool is served; a new wallet handle (the way an
    /// endpoint is changed) starts fresh.
    UpToDateDegraded { tip: u32, pools: PoolServiceReport },
    /// Scanned to THIS SERVER's reported tip, and that tip is BELOW a block
    /// height this wallet already knows the chain reached — the newest height
    /// the signed data this app ships with carries for the network, or the
    /// wallet's own last scanned height less a reorg allowance: the server is
    /// behind the chain (a node still syncing, stuck or forked, or a server
    /// under-reporting its height), so the balance is current only as of `tip`
    /// — a block the network passed before this version was built, or before
    /// this wallet last synced.
    ///
    /// **Do not render this as "Up to date".** The honest copy is "caught up
    /// with this server, but the server is behind the network — switch
    /// servers": payments received after `tip` are not visible from here, and a
    /// send built against this server's tip may not go through. `newestKnown`
    /// is the newest height this wallet knows the chain reached — a public
    /// constant of the app, or this wallet's own last scanned height less the
    /// reorg allowance (a height `upToDate.tip` already exposes) — so
    /// `newestKnown - tip` is a LOWER bound on how far behind the server is.
    /// `pools` is the same per-pool report [SyncStatus.upToDateDegraded]
    /// carries (null when the pass made no pool claim), so a withheld or
    /// refused pool stays visible beside the behind claim. The next step is
    /// SWITCH SERVERS — not "update the app" ([SyncStatus.upToDateLimited])
    /// and not "check your connection" ([StallReason.endpointUnreachable]).
    /// Clears by itself on the next pass on which the server reports a tip at
    /// or above that height; a new wallet handle starts fresh.
    EndpointBehind {
        tip: u32,
        newest_known: u32,
        pools: Option<PoolServiceReport>,
    },
    /// Scanned to the tip, but THIS SERVER will not say which network it is
    /// on, so this app cannot confirm that a payment it signs will be accepted.
    /// Sending is allowed for a short grace — `grace` says how much is left
    /// (whichever of blocks and time runs out first; `secsLeft` is null when
    /// the device clock cannot be trusted for it, then show the blocks alone)
    /// — or, once it has ended, why (a day of blocks, a day on the device
    /// clock, or this wallet never confirmed at all) and that sending is
    /// refused until a server that reports its network is used.
    ///
    /// **Do not render this as "Up to date".** The honest copy is "caught up,
    /// but this server isn't reporting the network version — you can still
    /// send for about N hours / N blocks, then switch servers" while it runs,
    /// and "this server hasn't reported the network version for N blocks / for
    /// a day — switch servers" (for [GraceExpiry.clock]: "if the device's date
    /// and time are wrong, fix them first — then switch to a server that
    /// reports the network version"; the clock is a precondition, never an
    /// alternative, since a corrected clock alone re-permits nothing — UI-1,
    /// §4r U-5) once it has ended. NEVER "the network was
    /// upgraded" and NEVER "update the app" ([SyncStatus.upToDateLimited]'s
    /// copy): nothing is known to have changed and an update fixes nothing.
    /// Balance beside this is current (the server serves blocks; only its
    /// network claim is missing) — UNLESS `streakReported` is true: then the
    /// loop has judged this server misbehaving (repeated rewinds, the
    /// `endpointMisbehaving` stall this claim outranks — P2-6), and the host
    /// MUST NOT say "your balance is current" (P3-12, maintainer); the next
    /// step is the same "switch servers". `pools` is the same per-pool report
    /// [SyncStatus.upToDateDegraded] carries, so a withheld pool stays visible
    /// beside this claim. Survives a relaunch (the first pass republishes it);
    /// clears on the first pass against a server that reports its network.
    UpToDateUnverified {
        tip: u32,
        grace: UnknownBranchGrace,
        pools: Option<PoolServiceReport>,
        streak_reported: bool,
    },
    /// Typed, renderable degradation — never a dead stream.
    Stalled { reason: StallReason },
    /// No connectivity. Queued sends are NORMAL in this state, not errors;
    /// `lastSynced` is the age of what the user is looking at.
    Offline { last_synced: Option<SyncStamp> },
    /// Forward-compatibility arm: the core reported a state this binding
    /// does not know. Unreachable in a lockstep build (CI enforces arm
    /// coverage); render as a neutral "syncing" state.
    Unknown,
}

/// What the light server did for ONE shielded pool's subtree roots on the
/// last sync pass. Five different sentences, never to be rendered alike:
/// "served N", "served nothing yet", "served nothing although the network
/// has some", "does not know this pool", "served heights that cannot be
/// true".
pub enum PoolService {
    /// The server served `roots` completed subtree roots. `0` here is a zero
    /// the wallet has no proof against (every pool before its first 2^16
    /// notes) and is not, by itself, a problem — render it as information
    /// ("no completed subtrees yet"), never as an error.
    Served { roots: u32 },
    /// The server served FEWER roots — none, or only the first few — than the
    /// wallet can prove the pool already has (`proven` completed subtrees at or
    /// below the server's own reported tip, from the signed data the app ships
    /// with — a public fact about the network, never about this wallet). Funds
    /// received in the part of the pool it did not serve cannot be spent
    /// through this server. `proven` is the network's count, not the gap. NOT
    /// "the pool is empty" — switch servers.
    Withheld { proven: u32 },
    /// The server does not know this pool at all (an older lightwalletd).
    /// Funds received in this pool cannot be spent through this server. NOT
    /// "the pool is empty".
    Unsupported,
    /// The server served subtree completion heights that cannot be true, and
    /// the wallet refused to record them. NOT "empty", NOT "unknown pool":
    /// the server answered and the answer was wrong — switch servers.
    HeightViolation,
    /// Forward-compatibility arm — render as "this pool's status is unknown",
    /// never as healthy.
    Unknown,
}

/// Per-pool service on the last sync pass, for every shielded pool the wallet
/// ingests subtree roots for. Carried by [SyncStatus.upToDateDegraded].
/// Names a POOL and a bounded COUNT only — never a note, an address, a
/// height, or the server's identity.
pub struct PoolServiceReport {
    pub sapling: PoolService,
    pub orchard: PoolService,
    pub ironwood: PoolService,
}

/// Why sync is stalled.
pub enum StallReason {
    /// Endpoint unreachable.
    EndpointUnreachable,
    /// Tor is REQUIRED by policy and the runtime is unreachable —
    /// fail-closed: zero clearnet packets were (or will be) sent.
    TorUnavailable,
    /// Device storage is full — "free space and retry".
    StorageFull,
    /// Chain reorganization in progress.
    ChainReorg,
    /// A corrupt wallet store, or a local fault the wallet could not diagnose —
    /// the problem is on THIS device, not the network. The honest next step is
    /// repair / restore-from-seed, never "switch servers". A transient local
    /// fault is [StallReason.storageUnavailable] instead.
    Internal,
    /// The server ANSWERED, and the answer was wrong: data that will not
    /// decode, a block span that does not add up, a required pool it does
    /// not know, or subtree completion heights the wallet's own evidence
    /// refutes — so sync stopped rather than trust it. The connection is
    /// FINE and the server is not — the honest next step IS "switch
    /// servers": never "check your connection" (that is
    /// [StallReason.endpointUnreachable]) and never "restore from seed" (that
    /// is [StallReason.internal]). Retried automatically; clears if the server
    /// starts answering correctly. One member of the class is a served root or
    /// height that conflicts with what an EARLIER server told this wallet, and
    /// the wallet cannot tell which of the two was wrong — so if every server
    /// is refused, the last resort is a rescan (a rebuild that keeps the seed),
    /// never a restore.
    EndpointMisbehaving,
    /// This wallet is set to start from a block height the chain, as THIS
    /// SERVER reports it, has not reached yet — a birthday above both the
    /// server's tip and the newest height this app's signed data vouches for.
    /// The wallet cannot tell a server behind the chain from a birthday typed
    /// above the real chain tip, so the honest copy names BOTH next steps:
    /// check the starting height you entered when restoring, or try another
    /// server. Never "check your connection" (the server answered) and never
    /// "restore from seed" (nothing on the device is at fault). Retried
    /// automatically; clears when the server catches up or the birthday is
    /// lowered (a rescan from an earlier height).
    BirthdayInFuture,
    /// This device's wallet storage could not be used for a moment — the
    /// database was busy past its timeout, or an I/O fault (e.g. a locked iOS
    /// device's Data Protection). Transient and local: the store is intact and
    /// sync retries by itself. Never "restore from seed" (that is
    /// [StallReason.internal]), never "switch servers" or "check your
    /// connection". The background sync loop publishes it (and `internal`) only
    /// at the second local fault before a pass completes; one bounded `syncFor`
    /// pass publishes its own fault at once, and nothing retries until the host
    /// syncs again.
    StorageUnavailable,
    /// Forward-compatibility arm — render as "sync stalled (unknown
    /// reason)": still a stall, never a healthy state.
    Unknown,
}

/// Why the wallet's grace for a server that does not report which network it
/// is on has ended — carried by [WalletErrorKind.consensusGraceExpired], by
/// [ParkedSend.signingBlock] and by [SyncStatus.upToDateUnverified], so every
/// copy site names the cause and the next step. Three sentences, never
/// rendered alike, and NEVER the "update the app" copy — nothing was upgraded
/// and an update fixes nothing.
pub enum GraceExpiry {
    /// A day's worth of blocks passed since this app last confirmed, with a
    /// server that reports its network, that it can send — and this server
    /// never said. Next step: switch servers.
    Blocks,
    /// A day passed on the DEVICE clock since that confirmation, whatever this
    /// server's block height did (the one thing a server that freezes its
    /// height cannot hold still) — or the clock was set back after the app
    /// saw that day pass, which does not re-open the grace. Next step: a
    /// server that reports the network version; if the device's date and
    /// time are wrong, fix them FIRST (a wrong clock is the one benign cause,
    /// but a corrected clock alone re-opens nothing — the copy names the clock
    /// as a precondition, never as an alternative; UI-1, §4r U-5).
    Clock,
    /// This wallet has never confirmed it can send at all — every server it
    /// has met withheld the network version — so there is no grace to run.
    /// Next step: switch servers. NOT the never-synced case
    /// ([WalletErrorKind.consensusNotEvaluated] — wait for the first sync).
    NeverConfirmed,
    /// Forward-compatibility arm — render as "this server isn't reporting the
    /// network version — switch servers": still a refusal, never a healthy
    /// state.
    Unknown,
}

/// Where the grace for a server that does not report its network stands, as
/// carried by [SyncStatus.upToDateUnverified]. REMAINING counts and a reason
/// only — never a timestamp.
pub enum UnknownBranchGrace {
    /// Sending still works. `blocksLeft` on the block rule; `secsLeft` is the
    /// time left on whichever rule runs out first (the blocks converted
    /// through the network's target block spacing by the SDK, so hosts never
    /// need to know it) — null when the device clock cannot be trusted for
    /// it, in which case show the blocks and no time.
    Running {
        blocks_left: u32,
        secs_left: Option<u32>,
    },
    /// Sending is refused; `by` says why. `blocksSinceLastCurrent` is the
    /// count the [GraceExpiry.blocks] copy names ("for N blocks"); null for
    /// [GraceExpiry.neverConfirmed].
    Ended {
        by: GraceExpiry,
        blocks_since_last_current: Option<u32>,
    },
    /// Forward-compatibility arm — render as the ended state's generic copy
    /// ("this server isn't reporting the network version — switch servers"):
    /// never as a running grace.
    Unknown,
}

/// Why the wallet will NOT sign a parked send, carried per row by
/// [ParkedSend.signingBlock]. The condition is about the app or the server,
/// never this payment — every queued row carries the same value. Render each
/// distinctly; NEVER "will send when ready" and NEVER "failed" (the money is
/// untouched and the intent is intact). Keep `cancelParkedSend` available;
/// do NOT offer `retryParkedSend` while any block stands.
pub enum SigningBlock {
    /// The Zcash network runs rules this app version does not implement —
    /// "waiting for an app update; your funds are safe and nothing has been
    /// sent".
    NetworkUpgrade,
    /// This server will not say which network it is on and the grace has run
    /// out (or never began) — "waiting for a server that reports the network
    /// version — switch servers"; for [GraceExpiry.clock] the clock is named
    /// as a PRECONDITION ("if the device's date and time are wrong, fix them
    /// first — then switch servers"), never as an alternative (UI-1, §4r
    /// U-5). NEVER the app-update copy.
    GraceExpired { by: GraceExpiry },
    /// Forward-compatibility arm — render as "this payment can't be sent
    /// right now — your funds are safe and nothing has been sent": still a
    /// block, never a healthy pending row.
    Unknown,
}

/// Transaction lifecycle state. `queued` is a first-class NORMAL state
/// (offline-first), not an error.
pub enum TxStatus {
    /// Waiting for connectivity/broadcast — normal offline-first state.
    Queued,
    /// Broadcast, unmined.
    Pending,
    /// Mined at `depth` confirmations.
    Confirmed { depth: u32 },
    /// Expired unmined — the funds RETURNED to spendable; a normal history
    /// row (also how a multi-device double-spend race resolves).
    Expired,
    /// Endpoint rejected it.
    Failed,
    /// Forward-compatibility arm (see [SyncStatus.unknown]).
    Unknown,
}

/// Where a transaction THIS WALLET CREATED stands on its way to the chain — the
/// wallet's delivery obligation, readable per transaction. Once the wallet has
/// signed a payment and kept its bytes, delivering it is the wallet's job: the
/// SAME bytes go out again on every later sync and after a relaunch until an
/// endpoint takes them and the chain mines them, or they expire. Orthogonal to
/// [TxStatus]: a received one carries NO delivery state (`null`), nor does an
/// expired one — unless the wallet will send that payment again (`retryPending`).
pub enum DeliveryState {
    /// The signed bytes are kept, and the wallet is NOT going to broadcast them
    /// on its own right now (a swap deposit held past its quote's window; the
    /// narrow window between signing and recording an offline-queued send).
    /// Render it as "saved" without a promise of automatic sending.
    Persisted,
    /// The wallet owes this payment and sends it on a later sync pass and after a
    /// relaunch; on an EXPIRED attempt, it sends the payment again by itself once
    /// the expiry is past any reorg — never send such a payment again yourself.
    RetryPending,
    /// An endpoint accepted the bytes into its mempool; not yet in the chain.
    /// The wallet keeps re-offering the same bytes until it mines (a duplicate
    /// is refused harmlessly), so this is a reading, never a stop.
    Accepted,
    /// Mined — the chain owns it; [TxStatus.confirmed] carries the depth.
    Confirmed,
    /// Forward-compatibility arm (see [SyncStatus.unknown]): render as "saved"
    /// without a promise, never as "on its way".
    Unknown,
}

/// One history row.
pub struct TxSummary {
    /// Transaction id, lowercase hex in block-explorer display order.
    pub txid_hex: String,
    /// Groups the transactions minted by ONE proposal (a payment can mint
    /// several txs); lets history render "payment X partially landed".
    /// (`i64` for a plain Dart `int`; a local counter can't reach 2^63.)
    pub batch_id: Option<i64>,
    /// Mined height, if mined.
    pub mined_height: Option<u32>,
    pub status: TxStatus,
    /// SIGNED net effect on this wallet (zatoshis; negative = outgoing).
    pub net_amount_zat: i64,
    /// Fee, when known (zatoshis).
    pub fee_zat: Option<i64>,
    /// Whether a memo rides this tx (content is fetched separately).
    pub has_memo: bool,
    /// Whether the wallet sees a NON-CHANGE TRANSPARENT output on this tx — the
    /// public-payment class (a send to a transparent recipient, a payment
    /// received at our transparent address, or a ZIP-320 hop leg). Its amount +
    /// addresses are publicly visible on-chain; hosts should render such rows
    /// distinguishably (a "public" badge). Fully-shielded activity and
    /// pool-internal shields stay `false`.
    pub has_transparent_output: bool,
    /// Unix seconds, display-only.
    pub timestamp: Option<i64>,
    /// The wallet's delivery obligation for a transaction IT created
    /// ([DeliveryState]); `null` for a received or an expired one. The activity
    /// list refines its status word from this ("Retrying" while the wallet
    /// still owes the broadcast, "Saved" while it holds the bytes), and the
    /// send flow reads the same value per transaction through
    /// [WalletHandle.deliveryState].
    pub delivery: Option<DeliveryState>,
    /// The height at which the outcome of a transaction THIS WALLET CREATED
    /// resolves either way: the last block it can be mined in (its expiry).
    /// `null` for a received transaction and for one built with no expiry.
    ///
    /// **How to read it.** The row's status is judged against the wallet's OWN
    /// scan, never the chain tip. So beside a pending row (and
    /// `delivery == retryPending`), read this height against a bounded sync's
    /// `{scannedTo, tip}` (`syncFor`):
    /// - `expiryHeight <= tip` — the chain has already decided; the wallet has
    ///   not looked yet. A sync that scans to at least `expiryHeight` resolves
    ///   the row (confirmed, or expired with the funds spendable again).
    /// - `expiryHeight > tip` — unresolved on chain: wait for block
    ///   `expiryHeight`. Until then the transaction can still be mined, so do
    ///   not tell the user to send again.
    pub expiry_height: Option<u32>,
}

/// One page of history plus the OPAQUE cursor to continue. `next_cursor` is
/// `Some` when more rows may follow — the host shows a "load more" affordance and
/// passes the token back to [WalletHandle.transactions] VERBATIM (it is an opaque
/// keyset token; never parse or construct it). `None` ⇒ the last page.
pub struct HistoryPage {
    pub rows: Vec<TxSummary>,
    pub next_cursor: Option<String>,
}

/// What edge produced an [IncomingFundsEvent] (ADR-0536).
pub enum IncomingFundsEventKind {
    /// The one per-subscriber catch-up event, FIRST on every subscription:
    /// arrivals already in history above the `sinceCursor` the host passed
    /// (count 0 on a null cursor — the baseline that hands the host a cursor
    /// to persist).
    Replay,
    /// The scan edge detected arrivals in a just-committed batch. On a detect
    /// read fault the event still fires with `newTxCount` 0 and the span set —
    /// "pull to confirm", never a silent miss.
    Live,
    /// An enhancement pass changed how existing rows read — re-pull if it matters.
    ///
    /// TWO causes, and you must NOT assume either one: decrypted transaction data
    /// landed after detection (memos arrive via a later enhancement pass), so
    /// attribution may have improved; OR the chain's view of a transaction was
    /// recorded, which can flip a send out of "expired" with nothing new to decrypt.
    /// So this is NOT proof that new memo data exists — do not prefetch memos on the
    /// strength of the kind alone. Re-pulling the transaction list is correct under
    /// both causes.
    ///
    /// Counts unchanged; the cursor carries the last delivered
    /// watermark forward, so it is safe to persist from ANY event.
    MemoRefresh,
    /// Forward-compatibility arm (see [SyncStatus.unknown]) — treat as a
    /// generic "pull now" nudge.
    Unknown,
}

/// One incoming-funds stream item (ADR-0536 — the FR-1 "funds arrived" hook):
/// a MINIMAL counts+heights event, safe BY CONSTRUCTION to log or forward
/// toward a notification path — it carries NO txid, amount, memo, or address.
/// Attribution (which tx, how much, the memo) is a PULL via
/// [WalletHandle.transactions] / the memo accessor, in the main app — never in
/// a notification extension. Delivery is AT-LEAST-ONCE with latest-wins
/// coalescing: `newTxCount` and the span are advisory; `totalTxDetected` is
/// the coalesce-proof monotonic a host diffs to know edges were missed. This
/// stream is a FRESHNESS SIGNAL, not an accounting or unlock ledger — a
/// rescan/restore replays history with OLD spans, so a notification-driving
/// host MUST gate on `spanToHeight > its own persisted watermark` (and its
/// own consumed-txid ledger, if it needs exactly-once).
pub struct IncomingFundsEvent {
    pub kind: IncomingFundsEventKind,
    /// Arrivals in THIS edge (advisory under coalescing).
    pub new_tx_count: u32,
    /// Cumulative live detections since wallet-open — monotonic, never
    /// rewinds. (`i64` for a plain Dart `int`; a per-open counter can't
    /// reach 2^63.)
    pub total_tx_detected: i64,
    /// The mined-height span of this edge's detections (`null` on
    /// `memoRefresh` and on a count-0 `replay`).
    pub span_from_height: Option<u32>,
    pub span_to_height: Option<u32>,
    /// OPAQUE watermark token: persist it and pass it back as `sinceCursor` on
    /// the next subscription — never parse or construct it. Always non-empty
    /// on a delivered event. NOTE: a NOTIFICATION watermark (the "have I told
    /// the user about this span?" gate) is the max `spanToHeight` you have
    /// acknowledged, tracked on YOUR side — the cursor's only use is
    /// `sinceCursor`.
    pub cursor: String,
}

/// Per-transaction submit outcome. One proposal may create SEVERAL
/// transactions (pool-crossing); partial failure is first-class and never
/// collapsed into a single error. The list [WalletHandle.send] returns is in
/// BROADCAST order; for a ZIP-320 TEX two-step that order is LEG-significant —
/// `[0]` is the unshield to a wallet-controlled one-time address, `[1]` the
/// forward to the recipient (see [WalletHandle.send] / [SendProposal.isTwoStepTex]).
pub enum TxSubmitResult {
    /// Accepted by the endpoint.
    Success { txid_hex: String },
    /// No verdict from the endpoint (a network failure or a non-OK status) — the
    /// tx MAY still have landed, and it may still be re-broadcast; watch the
    /// history row.
    GrpcFailure { txid_hex: String },
    /// Endpoint rejected it from the mempool (`code` is the server's).
    SubmitFailure { txid_hex: String, code: i32 },
    /// An earlier tx in the set got NO verdict from the endpoint (a network
    /// failure — a mempool reject does NOT produce this arm; later txs are still
    /// attempted); this one was not attempted. It stays persisted and is
    /// re-broadcast on the next sync.
    NotAttempted { txid_hex: String },
    /// Forward-compatibility arm (see [SyncStatus.unknown]).
    Unknown,
}

/// Queryable Tor/network-privacy state — cold state, no event replay needed.
/// In `dialer` mode the SDK reports policy + flow only; torness ATTESTATION
/// belongs to the host's transport (the variant says which runtime is
/// speaking so a panel can attribute the claim honestly).
pub enum TorState {
    /// Off by configuration.
    Off,
    /// The private path is starting; `percent` when the runtime exposes
    /// progress. `transport` is the registered transport's own name, as on
    /// [TorState.unavailable].
    Bootstrapping {
        percent: Option<f32>,
        transport: Option<String>,
    },
    /// The configured path is ready and nothing has said otherwise.
    Active { runtime: TorRuntimeKind },
    /// Policy was `preferred` and Tor degraded to clearnet — VISIBLE, never
    /// silent.
    FellBack,
    /// Zero traffic: policy is `required` and the path is unreachable, nothing
    /// is registered, or the registrant declared its transport FAILED.
    /// `transport` is the HOST'S OWN name for it (the same bounded, validated,
    /// render-verbatim string as [TorRuntimeKind.hostDialer]'s `name`) —
    /// `null` when nothing is registered or the runtime has no registry, where
    /// a UI renders its own transport-neutral noun. The SDK never names a
    /// transport the host did not declare.
    Unavailable { transport: Option<String> },
    /// Ready, and nothing has come back over it for a minute while the wallet
    /// was trying — the path or the wallet server; the SDK does not say which.
    /// Same payload as [TorState.active], so render the same transport name
    /// with a different sentence. Under `preferred` it is brief: the next
    /// connection leaves for clearnet and the state reads [TorState.fellBack].
    /// Under `required` it stands until something comes back; a host that can
    /// see its own traffic carrying on the same path may sharpen the sentence
    /// to "the wallet server is not answering". Nothing was sent in the clear.
    Unanswered { runtime: TorRuntimeKind },
    /// Forward-compatibility arm. PRIVACY RULE: render as "Tor status
    /// unknown — treat as NOT protected"; never inherit a benign/protected
    /// framing for a state this binding cannot interpret.
    Unknown,
}

/// How many connections each arm served, by outcome, since this wallet
/// opened ([WalletHandle.dialCounts]). One connection is one dial — an
/// outcome, not bytes. Per wallet, in memory only, gone at close; never
/// logged by the SDK. Show it as numbers beside the transport state; it
/// carries counts by arm × outcome and nothing else.
pub struct DialCounts {
    /// Dials the host's registered transport served.
    pub private: ArmCounts,
    /// Dials the SDK's own direct dialer served: every dial under
    /// `TorPolicy.off`, and the `preferred` fallback after the minute.
    pub clearnet: ArmCounts,
}

/// One arm's tally, one field per dial outcome. (`int` on the Dart side; a
/// per-wallet-open counter can't reach 2^63.)
pub struct ArmCounts {
    pub connected: i64,
    pub not_ready: i64,
    pub retired: i64,
    pub unreachable: i64,
    pub timeout: i64,
    pub unsupported: i64,
    pub io: i64,
    pub transport_failed: i64,
}

/// Which runtime is speaking. No free text and no key material cross: the
/// one arm with a payload carries two CLOSED enums, the host transport's
/// declared kind and isolation (FR-29), so a UI renders "via your app's
/// private path (Tor)" or "connections can be linked by the proxy" from what
/// the host declared and nothing more.
pub enum TorRuntimeKind {
    /// Host-side Tor via SOCKS5 (Orbot, system Tor, a sidecar).
    ExternalSocks5,
    /// A Rust-level dialer injected by the host (not constructible from
    /// Dart; reported here so a UI can attribute the torness claim).
    Dialer,
    /// The dialer the host's native library registered
    /// (`TorRuntimeConfig.hostDialer`), with the transport it declared as of
    /// this read (ADR-0547: the SDK has no predefined transport kinds).
    /// `name` is the HOST'S OWN display name for its transport — at most 32
    /// bytes of validated UTF-8 with no control characters, rendered verbatim
    /// in the chip, never interpreted; an empty `name` is the SDK's
    /// "unattributed" rendering (never a host's), shown as "a private path".
    /// PRIVACY RULE: `exposure` says whether the path hides the device's
    /// address from the server — `exposed` renders as not private whatever
    /// the isolation, `unknown` renders with caution; `isolation` says
    /// whether the wallet's per-purpose connections are kept apart on it —
    /// `unsupported` or `unknown` renders as "connections can be linked by
    /// the proxy".
    HostDialer {
        name: String,
        isolation: IsolationSupport,
        exposure: TransportExposure,
    },
    /// Forward-compatibility arm (see [SyncStatus.unknown]).
    Unknown,
}

/// Whether the host's registered transport HIDES the device's network address
/// from the server (FR-29, ADR-0547) — the one privacy fact no transport name
/// can tell the wallet. `hidden`: the server sees the transport's exit, not
/// the device (Tor, a proxy, a tunnel). `exposed`: the server sees the
/// device's address (a plain connection behind the host's dialer, a forward
/// proxy that passes the client address) — render as NOT private, whatever
/// the isolation says. `unknown`: the host did not declare it — render with
/// caution, never the protected tone.
pub enum TransportExposure {
    /// The host did not declare it: render with caution.
    Unknown,
    Hidden,
    Exposed,
}

/// Whether the host's registered transport honours the wallet's per-purpose
/// circuit isolation (FR-29). The SDK passes its isolation key on EVERY
/// dial regardless; this reports whether the host's path keeps the
/// connections apart. `unknown` and `unsupported` BOTH render as "connections
/// can be linked by the proxy" — the state never promises what the host did
/// not declare.
pub enum IsolationSupport {
    /// The host did not declare it: render as linkable.
    Unknown,
    Supported,
    Unsupported,
}

/// The on-resume snapshot bundle: everything a UI needs to re-render cold.
pub struct WalletState {
    pub balance: BalanceSnapshot,
    /// (`sync` is a Dart keyword — named `syncStatus` on the Dart side.)
    pub sync_status: SyncStatus,
    pub tor: TorState,
    /// Current chain tip, when known.
    pub tip: Option<u32>,
    /// Balance age — always renderable.
    pub last_synced: Option<SyncStamp>,
    /// #357: `true` iff this wallet has reached chain tip at least once SINCE
    /// the last rescan (or ever, if none). Durable — survives a process death
    /// (the old session-local proved-tip latch did not), is CLEARED by a
    /// rescan (the rebuild genuinely catches up again, so the catch-up cue
    /// must re-show; post-ship #357 fix), and dies with the wallet identity.
    /// UX-only; `false` on any read fault.
    pub ever_synced: bool,
    /// #377 s357b-2: `true` iff a rescan rebuild swapped in and has not
    /// reached tip since — durable across a process death, so a UI can name
    /// the rebuild ("rebuilding after your rescan") instead of showing the
    /// generic first-run catch-up copy after a relaunch loses the in-session
    /// rescan state. Cleared at the first post-rescan reached-tip. UX-only;
    /// `false` on any read fault.
    pub rescan_rebuilding: bool,
    /// Monotonic sequence stamping the snapshot AND every stream item — a
    /// stale stream event can never beat a fresher snapshot in a UI race
    /// (compare and keep the larger). (`i64` for a plain Dart `int`; a
    /// process-local counter can't reach 2^63.)
    pub seq: i64,
}

/// FR-53 — the whole answer of one [WalletHandle.severCustody], the duress
/// force-sever. Every custody outcome is a value here, never an exception:
/// read `severed`, not the absence of an error. Nothing in it names a key, a
/// path, a namespace or a custody id.
pub struct SeverReport {
    /// What happened to the wallet's key-store custody.
    pub severed: SeverOutcome,
    /// Who held the wallet's store when the sever ran.
    pub holder: HolderSeen,
    /// Whether the wallet's files are gone, or the host must delete them.
    pub files: FilesOutcome,
}

/// What a sever did to the wallet's key-store custody. Only `severed` is
/// PROVEN; branch on the variant, never on the report's arrival.
pub enum SeverOutcome {
    /// PROVEN: the key store reported `count` (> 0) items severed: the key that
    /// unlocks the files on disk is deleted from the key store. What that
    /// establishes depends on the tier (`CustodyDisclosure.eraseAssurance`,
    /// ADR-0571): no tier proves an earlier copy of the key unusable. A live
    /// holder's memory still holds the seed and the database key until it drops.
    Severed {
        /// Items the key store severed (saturates at the largest `int` the
        /// bridge carries; always > 0).
        count: u32,
    },
    /// The deletes were issued and each answered success or not-found, but no
    /// count could be read (the phone was locked). A `wipe` after the next
    /// unlock confirms it. Never render this as a proven sever.
    SeveredUnproven { reason: UnprovenReason },
    /// There was no custody left to sever: nothing was ever custodied here, or
    /// an earlier wipe or sever already severed it.
    AlreadyGone,
    /// Nothing was severed; for any cause but `timeout`/`busy` the custody may
    /// still be live. See [NotSeveredCause].
    NotSevered { cause: NotSeveredCause },
    /// Forward-compatibility arm — an outcome this bridge does not know yet.
    /// Unreachable in a lockstep build (`bridge_enums_cover_core_variants`).
    /// Treat it as NOT severed: the custody may still be live.
    Unknown,
}

/// Why a sever could not prove its count.
pub enum UnprovenReason {
    /// The device was locked: the key store refused the reads a count needs.
    /// A `wipe` after the next unlock confirms the sever.
    CountUnreadable,
    /// Forward-compatibility arm. Treat it as unproven.
    Unknown,
}

/// Why a sever severed nothing.
pub enum NotSeveredCause {
    /// A wallet is on disk, yet the key store severed zero items under every
    /// namespace this directory names (the verify-real-sever guard). Pass the
    /// SAME `dbDir` the wallet was opened with.
    NothingSevered,
    /// The key store did not answer inside the deadline. Call `severCustody`
    /// again: no new custody write can land meanwhile.
    Timeout,
    /// The key store is still busy with an earlier call — any wallet's in
    /// this process, or the custody selftest's; the sever's calls were refused
    /// at once. Call `severCustody` again: it succeeds once that call returns.
    Busy,
    /// The deadline was spent before a key-store call could start (a zero
    /// deadline makes no key-store call).
    PastDeadline,
    /// This platform has no key store, and a wallet is on disk.
    VaultAbsent,
    /// The key store failed, or a blind delete on a locked device failed.
    KeystoreUnavailable,
    /// The SDK stopped WAITING at `deadlineMs` + `severAnswerGraceMs()`, but its
    /// own sever is still running in the background (it cannot be cancelled),
    /// and it may still be deleting files in `dbDir`. The report's `files` is
    /// `stillInUse`. Do NOT purge or re-create anything at that `dbDir`. Call
    /// `severCustody` again, with a backoff, until the cause is not
    /// `stillRunning` (each call made while it runs answers `stillRunning` at
    /// once and never reaches the store), or exit the process.
    StillRunning,
    /// Forward-compatibility arm. Treat the custody as possibly live.
    Unknown,
}

/// Who held the wallet's store when a sever ran.
pub enum HolderSeen {
    /// Nobody: the sever took the lock itself and ran the ordinary wipe.
    None,
    /// An instance in this process (a live handle, a straggler past `close`,
    /// a rescan or switch mid-rebuild, an open in flight). It is poisoned:
    /// every later call on it answers `invalidState` (`wiped`).
    ThisProcess,
    /// Something outside this process holds the wallet's lock.
    OtherProcess,
    /// Not known, for one of two reasons: a holder a newer core reports and
    /// this build does not know (forward compatibility), or the `stillRunning`
    /// answer, which carries no holder: the SDK's own sever, still running,
    /// reads it later, and that later read is not reported.
    Unknown,
}

/// What a sever did to the wallet's files.
pub enum FilesOutcome {
    /// The directory was swept (the lock was free).
    Removed,
    /// No file was deleted, and the SDK is done with the directory: the host
    /// deletes it itself. This is the ONLY value to purge on.
    LeftForHost,
    /// The SDK is still working in this directory (the `stillRunning`
    /// answer): do not purge or re-create here until a later call answers
    /// otherwise. Never purge on it.
    StillInUse,
    /// Forward-compatibility arm. Do not purge on it: call `severCustody`
    /// again, and purge only on a `leftForHost` answer (or exit the process).
    Unknown,
}
