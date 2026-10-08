//! `SwapService` — the provider-agnostic orchestration over [`SwapPort`]
//! (spec §3.2). W2 scope: the QUOTE/EXECUTE validation funnel — the M1
//! user-anchored sanity ceiling, the m1 provider-string/decimal bounds, the
//! refundTo echo check, deposit-address validation, and the quote registry
//! that makes `execute` re-verify before anything downstream. Deposit
//! signing, status polling, and persistence land with the wallet-store
//! chunk.
//!
//! Crash-restart single-flight (§6.3) — CLOSED at W-swap-3-c-3-ii. The
//! issued-quote registry was IN-MEMORY at W2, so a process kill between
//! `quote()` and `execute()` emptied it and a dropped-`execute`-then-requote
//! could DOUBLE-CHARGE (never LOSE — every deposit is gated by the durable
//! `deposit_gate`). W-swap-3-c-1/-c-2-i made the issued quote durable, and
//! W-swap-3-c-3-ii promotes the durable atomic `take` to THE single-flight,
//! keyed on the quote id rather than process memory: a post-kill execute now
//! RECONSTRUCTS the frozen deposit from the durable row, and the `take` claims
//! it exactly once, so a retry / double-tap / second restart sees `None` ⇒
//! `QuoteExpired` (#367 — "this quote is no longer valid, get a new quote";
//! no second deposit). The host may now safely retry a dropped `execute`
//! future — the durable claim makes it idempotent.
//!
//! H1 (decided at W2, manager preference confirmed): the NEAR adapter
//! (v1.x, `zec-wallet-swap-near`) will be a THIN TYPED REST CLIENT over
//! the documented 1Click API — not the OpenAPI-generated `one-click-sdk-rs`
//! (0.1.x; a funds path deserves a hand-audited client). The `SwapPort`
//! shape here is unaffected either way; the adapter rides the SAME
//! `NetDialer`/`TorPolicy` stack as sync (§3.2 m5 — no independent clearnet
//! path; `Required` fail-closes swap calls too).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use tokio::sync::watch;
use tracing::Instrument as _;

use crate::constants::{
    DEADLINE_SAFETY_MARGIN_SECS, DEPOSIT_EXECUTE_MARGIN_SECS, PROVIDER_STR_MAX_BYTES,
    SLIPPAGE_MAX_BPS, SWAP_DEADLINE_DEFAULT_SECS, SWAP_DECIMAL_MAX_DIGITS,
    SWAP_DEPOSIT_DEADLINE_OUT_OF_ZEC_SECS, SWAP_NOT_FOUND_TERMINAL_POLLS, SWAP_POLL_INITIAL_SECS,
    SWAP_POLL_MAX_SECS, SWAP_SETTLEMENT_MAX_SECS,
};
use crate::error::{DestinationInvalidReason, ProviderProtocolReason, QuoteBoundSide, SwapError};
use crate::memo::{Address, AddressKind};
use crate::money::{Network, Zatoshis};
use crate::seed::SpendBinding;

use super::SwapPort;
use super::types::{
    ExactSide, QuoteRequest, SwapAmount, SwapDirection, SwapFailureCode, SwapId, SwapKill,
    SwapQuote, SwapStatus, TokenList,
};

/// Where OutOfZec refund addresses come from: a FRESH wallet transparent
/// address per swap, single-use, never recycled (§2.6 HARD-H — the deposit
/// leaves the shielded pool, so without this the refund path doesn't
/// exist). Since #368 the mint also ENGINE-REGISTERS the address
/// (`get_address_for_index` at the same shared single-use index, byte-equality
/// cross-checked against the raw BIP44 derivation) and returns the index
/// alongside — recorded in the durable issued-quote record so execute can arm
/// the refund watch (the ADR-0527 discharge; pre-#368 this returned a bare
/// String and the address was derive-and-forget). Implemented by the wallet
/// store chunk; tests inject a stub.
///
/// **§5.4 NEVER-log:** the returned address is a never-log item; an impl MUST
/// NOT log it. Surface failures as [`SwapError::RefundAddressUnavailable`]
/// (payload-free).
#[async_trait]
pub trait RefundAddressSource: Send + Sync {
    async fn fresh_refund_address(&self) -> Result<IssuedRefund, SwapError>;
}

/// Where a fresh per-swap IntoZec DESTINATION address comes from (§3.3b D1 / ADR-0530
/// — the SIBLING of [`RefundAddressSource`]: refunds are the OutOfZec leg, destinations
/// the IntoZec leg; they share ONE single-use external counter at the adapter, but their
/// contracts differ so they stay sibling ports — refunds derive read-only, destinations
/// engine-PERSIST). The wallet mints a fresh ZEC receive address at a DISTINCT single-use
/// external index via the audited engine `get_address_for_index` — engine-persisted so the
/// §3.3b D2 scoped poll can detect the delivery (the piece refunds defer) — and returns
/// BOTH the encoded address (handed to 1Click as `recipient`) and the index (recorded in
/// the durable issued-quote record so a crash between quote and execute reconstructs the
/// detection-set entry, §3.3b L1). Distinct per call ⇒ no two swaps share a destination
/// (HARD-H unlinkability). Implemented by the wallet store chunk; tests inject a stub.
///
/// **§5.4 NEVER-log:** the returned address is a never-log item; an impl MUST NOT log it.
/// Surface failures as [`SwapError::DestinationAddressUnavailable`] (payload-free).
#[async_trait]
pub trait DestinationAddressSource: Send + Sync {
    async fn fresh_destination(&self) -> Result<IssuedDestination, SwapError>;
}

/// Where an `OutOfZec` deposit is actually SENT: the wallet's §4.4 deposit-send
/// path (the `RefundAddressSource` precedent — a wallet capability the swap port
/// depends on, not the other way round). [`SwapService::execute`] calls this
/// AFTER the provider registers intent, handing the EXACT transparent `Address`
/// that [`validate_quote`] blessed (L-1 — never re-parsed, so "validated" and
/// "sent" are provably the same value), the ZEC amount to deposit, and the
/// quote's wall-clock `expires_at`. The deadline rides into the durable deposit
/// intent so the resubmission machinery re-checks it on every pass (§4.4;
/// inc-2d-swap-a `deposit_gate`) — the monotonic gate in `execute` is only the
/// at-execute pre-flight.
///
/// The impl is OFFLINE-FIRST + crash-safe (the wallet's §6.2 outbox + §6.3
/// double-send guard): a process kill after this returns loses no deposit, and a
/// single call yields AT MOST ONE deposit (the §6.3 guard's idempotent
/// re-propose). Implemented by the wallet (`WalletDepositSender`, W-swap-3);
/// tests inject a recording double. Errors map to [`SwapError::DepositSendFailed`]
/// so no wallet-internal type leaks across the port.
///
/// **§5.4 NEVER-log:** an impl MUST NOT log `deposit` or `amount` — both are
/// NEVER-log items (a deposit address + an amount). Surface failures as
/// [`SwapError::DepositSendFailed`] (payload-free), never the values.
#[async_trait]
pub trait DepositSender: Send + Sync {
    /// Send the OutOfZec deposit NOW, inside the user-present execute window (FR-23-a /
    /// custody invariant §1.9): send `amount` ZEC to the provider's transparent `deposit`
    /// address, tagged with the quote's wall-clock `deadline_unix` (`expires_at`). The impl
    /// enqueues the durable, deadline-tagged intent AND signs it in the authorize-spend
    /// bracket this call runs within — so it works at every custody tier (a host-custody
    /// port serves the seed while the user is present). A signed-but-unbroadcast deposit
    /// (offline at execute) rides the §6.1 resubmission re-broadcast; a deposit that could
    /// not be signed this instant is left durable and money-safe. Returns once the deposit
    /// is signed (or durably enqueued), never leaking the address/amount on failure.
    /// `binding` (FR-17): the issued quote's spend-binding nonce — the impl copies it
    /// onto the durable intent row so every sign pull for this deposit presents it.
    async fn send_deposit_now(
        &self,
        deposit: Address,
        amount: Zatoshis,
        deadline_unix: u64,
        binding: Option<SpendBinding>,
    ) -> Result<(), SwapError>;
}

/// The DURABLE in-flight issued-quote backing (§3.2 W2; the W-swap-3-c-1
/// `issued_swap_quote` store). [`take`] is THE single-flight authority for execute —
/// within-process AND cross-restart (W-swap-3-c-3-ii): its atomic `IMMEDIATE`
/// SELECT-then-DELETE claims a quote exactly once, so no double-tap / replay / crash
/// recovery can ever double-deposit. Since stage S8 the durable record is ALSO the
/// sole source of every execution term (address, amount, deadline, binding, refund,
/// watch, the provider handle) on BOTH paths — the in-memory [`SwapService`]
/// registry keeps only the monotonic deadline half, which can refuse but never
/// supplies a term — so a within-process execute and a post-restart one run the
/// same record. [`peek`] reads a record WITHOUT consuming it, for the caller-DTO
/// terms compare that precedes the claim. [`persist`] records an issued quote so a
/// crash-then-requote is recoverable; [`take`] consumes it on execute so an executed
/// swap is never left looking in-flight AND a `None` is the authoritative
/// "already-claimed / never-issued" signal.
///
/// Implemented by the wallet (`WalletIssuedQuoteStore`, W-swap-3-c-2) over the sealed
/// aux-db; tests inject an in-memory double. Errors map to
/// [`SwapError::SwapStateUnavailable`] so no wallet-internal type leaks across the port.
///
/// **§5.4 NEVER-log:** an impl MUST NOT log the deposit `address`, the `amount`, or
/// the `provider_ref` (all NEVER-log items). Surface failures as the payload-free
/// `SwapStateUnavailable`.
///
/// [`persist`]: IssuedQuoteStore::persist
/// [`take`]: IssuedQuoteStore::take
/// [`peek`]: IssuedQuoteStore::peek
#[async_trait]
pub trait IssuedQuoteStore: Send + Sync {
    /// Durably record an issued quote under its SDK-minted id, returning
    /// [`IssuedQuotePersist::AtCapacity`] when the in-flight cap is hit (the consumer
    /// maps that to the same `RequestInvalid` the in-memory registry returns — one cap
    /// message). A duplicate id is an INVARIANT VIOLATION (the id is minted from the
    /// OS CSPRNG, never derived from provider data), surfaced as a typed store error —
    /// never first-wins, never an overwrite. A store failure is `SwapStateUnavailable`.
    async fn persist(&self, record: IssuedQuoteRecord) -> Result<IssuedQuotePersist, SwapError>;
    /// Read an issued quote WITHOUT consuming it — the non-claiming read the caller-DTO
    /// terms compare runs on before `take`, so a DTO whose terms differ is refused with
    /// the record intact (a later honest call still executes). `None` = never recorded
    /// or already consumed (the same signal as [`Self::take`]'s miss).
    async fn peek(&self, id: &SwapId) -> Result<Option<IssuedQuoteRecord>, SwapError>;
    /// Atomically CONSUME an issued quote — THE single-flight authority for execute
    /// (within-process AND cross-restart, W-swap-3-c-3-ii). `None` = never recorded or
    /// already consumed ⇒ the consumer refuses the execute `QuoteExpired` (no
    /// authority to proceed). Returns the durable record every execution term is
    /// read from.
    async fn take(&self, id: &SwapId) -> Result<Option<IssuedQuoteRecord>, SwapError>;
}

/// The durable in-flight swap RECORD sink (W-swap-5, #366) — the user-visibility
/// home row, written by `execute` immediately AFTER the provider registers intent
/// and BEFORE the OutOfZec deposit leg. RECORD-FIRST ordering is load-bearing: a
/// deposit can never be durably queued without its home row already committed, so
/// the "invisible armed deposit" window is structurally closed; a record failure
/// propagates and the deposit is NEVER queued (fail-safe — a
/// registered-but-unfunded order expires/refunds provider-side, the same accepted
/// bound as the §3.5 mid-execute kill abort).
///
/// Implemented by the wallet (`WalletSwapRecordSink`) over the sealed aux-db;
/// tests inject an in-memory double. Errors map to
/// [`SwapError::SwapStateUnavailable`] (payload-free port isolation).
///
/// Since stage S8 the port also carries the ONE read the status poll needs:
/// [`provider_ref`](Self::provider_ref) resolves a swap's provider handle from
/// the durable home row BEFORE the poll task is spawned — never inside it, never
/// from an in-memory cache (dead after a reopen).
///
/// **§5.4 NEVER-log:** an impl MUST NOT log `id`, `provider_ref` (the provider's
/// deposit address) or any deadline field tied to them.
#[async_trait]
pub trait SwapRecordSink: Send + Sync {
    /// Durably record "an order was registered at execute" — the honest minimum
    /// the home surface re-attaches from. A duplicate `id` is an invariant
    /// violation surfaced typed (the id is SDK-minted; the issued-quote
    /// single-flight makes a same-quote replay structurally dead).
    async fn record_started(&self, record: StartedSwap) -> Result<(), SwapError>;
    /// The provider handle recorded for `id` at execute, or `None` when no home
    /// row carries it (never recorded, dismissed, or a row written before the
    /// handle was recorded). Read-only.
    async fn provider_ref(&self, id: &SwapId) -> Result<Option<String>, SwapError>;
}

/// The port-level in-flight record (the storage row is the wallet impl's). The
/// direction is the COARSE arm only — no asset names or amounts are durably
/// stored (data minimization; the UI polls provider truth live via
/// `watch_swap_status`).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StartedSwap {
    /// The SDK-minted execution identity (see [`SwapId`]) — the home row's key.
    pub id: SwapId,
    /// The provider's own handle for this swap (1Click: the deposit address) —
    /// the value the status poll hands the adapter as data. §5.4 NEVER-log.
    pub provider_ref: String,
    /// `true` = OutOfZec (the wallet sends the deposit), `false` = IntoZec.
    pub out_of_zec: bool,
    /// When the DEPOSIT must land (OutOfZec: the clamped §4.4 tag handed to
    /// [`DepositSender::send_deposit_now`]; IntoZec: the quote's durable
    /// deposit window the USER must beat). Display-only downstream.
    pub deposit_deadline: Option<u64>,
    /// The record's own self-lapse bound: execute-time + the settlement
    /// ceiling (`SWAP_SETTLEMENT_MAX_SECS` — the same conservative bound the
    /// destination watch uses).
    pub expires_at_wall: u64,
    /// The swap's watched inbound transparent leg (#368): the IntoZec
    /// DESTINATION or the OutOfZec REFUND address, with its single-use
    /// external index — from the taken durable record. The sink arms/extends
    /// the scoped detection watch ATOMICALLY with the home row (record-first
    /// is watch-first; this REPLACES the pre-#368 `take`-time
    /// `mark_executed`, so a refused execute no longer parks a settlement
    /// watch and a refund watch never exists for a deposit that never left).
    /// `None` only for a record that predates the durable handles.
    pub watch: Option<SwapWatch>,
}

/// A watched inbound transparent leg (#368) — the address a provider was told
/// to pay us at (IntoZec: the destination UA; OutOfZec: the refund t-addr)
/// plus the single-use external index it was minted at. §5.4 NEVER-log.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SwapWatch {
    pub address: String,
    pub index: u32,
}

/// A durable issued-quote at the PORT level (the storage `StoredQuote` reconstruction is
/// the wallet impl's) — since stage S8 THE record every execution term is read from,
/// keyed by the SDK-minted `id`. `deposit` is `Some` for an OutOfZec quote (the frozen
/// §4.4 deposit the wallet commits to send), `None` for IntoZec (ZEC is RECEIVED to our
/// own address — no wallet-side deposit). `expires_at_wall` is the quote's wall-clock
/// deadline (unix seconds) — the deadline authority on both paths (a monotonic
/// `Instant` cannot survive a process exit; the within-process monotonic half stays on
/// the in-memory registry and can only refuse earlier).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IssuedQuoteRecord {
    /// The SDK-minted execution identity (see [`SwapId`]).
    pub id: SwapId,
    /// The provider's own handle for this quote — the `id` the adapter populated
    /// (1Click: the deposit address). Carried onto the home row at execute and
    /// handed back to the adapter for its status URL / deposit notification as
    /// DATA; never a key. §5.4 NEVER-log.
    pub provider_ref: String,
    /// The terms the approval showed, both directions — what the caller's DTO is
    /// compared against field-by-field before the record is consumed.
    pub terms: QuoteTerms,
    pub deposit: Option<IssuedDeposit>,
    /// The fresh per-swap IntoZec DESTINATION (§3.3b D1 / ADR-0530): `Some` for `IntoZec`
    /// (the engine-persisted ZEC receive address handed to 1Click + its single-use external
    /// index), `None` for `OutOfZec`. MUTUALLY EXCLUSIVE with `deposit` by direction —
    /// enforced in [`SwapService::quote`], where the direction is known (a quote carries a
    /// deposit OR a destination, never both). Recorded durably so the §3.3b L2 scoped poll
    /// re-derives its detection set across a crash.
    pub destination: Option<IssuedDestination>,
    /// The fresh per-swap OutOfZec REFUND (#368 / ADR-0527): `Some` for `OutOfZec`
    /// (ALONGSIDE `deposit` — the refundTo t-addr the provider holds + its single-use
    /// external index, engine-registered at mint), `None` for `IntoZec` (whose inbound leg
    /// is `destination`). Recorded durably so execute can arm the refund watch from the
    /// taken record — the durable handle the pre-#368 derive-and-forget mint never left.
    pub refund: Option<IssuedRefund>,
    pub expires_at_wall: u64,
    /// FR-17 (#396): the spend-binding nonce minted at quote issue — the SAME value
    /// surfaced on the returned [`SwapQuote`] (the host records it at its
    /// execute-authorize bracket) and copied onto the deposit's durable intent row
    /// at execute, so the sign-at-execute pull presents it. `Some` for every quote
    /// issued post-FR-17; `None` only when reconstructed from a pre-FR-17 durable row.
    pub binding: Option<SpendBinding>,
}

/// The terms of a quote as the approval showed them (stage S8, R01): the fields of
/// the [`SwapQuote`] a host renders and — for OutOfZec — reads its `WalletSpendIntent`
/// off. Recorded durably at quote issue for BOTH directions (an IntoZec quote's
/// `zec_side` / `min_amount_out` had no durable home before) and compared
/// field-by-field against the caller's DTO at execute, BEFORE the record is
/// consumed: a DTO that names a record but carries different terms executes
/// nothing and consumes nothing. The deadline the approval showed IS among them
/// (contract §3.1, row 2): `expires_at` is the DTO's actionable deadline exactly as
/// `quote` returned it — recorded verbatim, so the compare is exact by construction
/// (the record's own wall gate still enforces the real bound after the claim; a
/// record that gate would already refuse is never compared, see `execute`). `binding`
/// is compared alongside (it is a field of the record already). `deposit_address`
/// and `deposit_memo` are §5.4 NEVER-log.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct QuoteTerms {
    pub deposit_address: String,
    pub deposit_memo: Option<String>,
    pub amount_in: String,
    pub min_amount_out: String,
    pub zec_side: Zatoshis,
    pub refund_to: Option<String>,
    /// The actionable deadline the approval showed (unix seconds) — the returned
    /// DTO's `expires_at`, verbatim.
    pub expires_at: u64,
}

impl QuoteTerms {
    /// The terms a DTO carries, in the record's shape.
    pub fn of(quote: &SwapQuote) -> Self {
        Self {
            deposit_address: quote.deposit_address.clone(),
            deposit_memo: quote.deposit_memo.clone(),
            amount_in: quote.amount_in.clone(),
            min_amount_out: quote.min_amount_out.clone(),
            zec_side: quote.zec_side,
            refund_to: quote.refund_to.clone(),
            expires_at: quote.expires_at,
        }
    }
}

/// A freshly minted OutOfZec refund (#368 / ADR-0527) — the return of
/// [`RefundAddressSource::fresh_refund_address`] AND the durable
/// [`IssuedQuoteRecord::refund`]: the bare BIP44 external t-addr handed to the provider as
/// refundTo (engine-registered at mint via `get_address_for_index` at the SAME index,
/// byte-equality cross-checked), plus the single-use external `index` it was minted at —
/// the SAME shared counter destinations draw from (DRY; distinct per swap — HARD-H).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IssuedRefund {
    pub address: String,
    pub index: u32,
}

/// The frozen §4.4 deposit carried in a durable [`IssuedQuoteRecord`]: the provider's
/// deposit-address STRING (already validated transparent + right-network by
/// `validate_quote` and frozen per quote id) plus the ZEC to send. The WITHIN-PROCESS
/// execute uses the in-memory L-1 `Address` (no re-parse); the CROSS-RESTART execute
/// (W-swap-3-c-3-ii) re-parses + re-blesses this string via `reconstruct_deposit_plan`
/// (the L-1 `Address` is gone after a restart) as defense-in-depth.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IssuedDeposit {
    pub address: String,
    pub amount: Zatoshis,
}

/// A freshly minted IntoZec destination (§3.3b D1 / ADR-0530) — the return of
/// [`DestinationAddressSource::fresh_destination`] AND the durable
/// [`IssuedQuoteRecord::destination`]: the engine-persisted ZEC receive address handed to
/// 1Click as `recipient`, plus the single-use external `index` it was minted at. Both are
/// recorded durably so the §3.3b L2 scoped detection set is crash-reconstructable (the
/// post-`open()` poll re-derives it from the issued-quote store). The `index` is the SAME
/// single-use external counter refunds draw from (DRY; distinct per swap — HARD-H).
///
/// **§5.4 NEVER-log:** `address` is a never-log item (an address).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IssuedDestination {
    pub address: String,
    pub index: u32,
}

/// Whether [`IssuedQuoteStore::persist`] stored the quote or refused it at the in-flight
/// cap (the crash-loop backstop). Returned (not a typed error) so the consumer maps
/// `AtCapacity` to the SAME `RequestInvalid{"too many in-flight quotes"}` the in-memory
/// registry returns — one cap message, not a second error code.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive] // G2 policy (spec §2): SDK upgrades may add outcomes
pub enum IssuedQuotePersist {
    Persisted,
    AtCapacity,
}

/// The §4.4 deposit a quote commits the wallet to send when executed — rebuilt
/// from the durable record at execute (`reconstruct_deposit_plan`: the same
/// parse + transparent + right-network checks that blessed the string at quote,
/// re-run as defense-in-depth). `None` deposit (an IntoZec quote) means execute
/// registers intent only (ZEC is RECEIVED to our own fresh address; no
/// wallet-side send).
struct DepositPlan {
    /// The transparent deposit address [`validate_quote`] blessed (parseable,
    /// transparent, right network — §4.4). Owned, moved into `send_deposit_now`.
    address: Address,
    /// The ZEC to deposit = the quote's `zec_side` (for OutOfZec, ZEC is the
    /// INPUT side the M1 bound already pinned).
    amount: Zatoshis,
}

pub struct SwapService {
    provider: Arc<dyn SwapPort>,
    refund_source: Arc<dyn RefundAddressSource>,
    /// The wallet's IntoZec DESTINATION-mint leg (§3.3b D1 / ADR-0530 — the
    /// [`RefundAddressSource`] sibling): `quote` mints a fresh engine-persisted ZEC
    /// receive address through it for an IntoZec quote, binds it as the provider
    /// `recipient`, and records it durably for the scoped detection set.
    destination_source: Arc<dyn DestinationAddressSource>,
    /// The wallet's §4.4 deposit-send leg (the L-1 hook): `execute` drives an
    /// OutOfZec deposit through it after the provider registers intent.
    deposit_sender: Arc<dyn DepositSender>,
    /// The DURABLE single-flight authority (W-swap-3-c-3-ii): `quote` persists the row,
    /// `execute` atomically CLAIMS it via `take` (within-process AND cross-restart). A
    /// `None` claim is the authoritative "already-claimed / never-issued" ⇒ `RequestInvalid`.
    issued_store: Arc<dyn IssuedQuoteStore>,
    /// The durable in-flight RECORD sink (W-swap-5, #366): `execute` writes the
    /// user-visibility home row here — after the provider registers intent,
    /// before the deposit leg (record-first; see [`SwapRecordSink`]).
    record_sink: Arc<dyn SwapRecordSink>,
    network: Network,
    /// The monotonic half of the deadline gate, per issued quote: the receipt-anchored
    /// `Instant` past which execute refuses (already includes the direction-aware
    /// margin; §2.6 clock-skew posture — it defends against a regressed wall clock).
    /// Present only within the issuing process run; absent (after a restart) the
    /// durable wall deadline gates alone. A CACHE OF A GATE, never a source of a
    /// term: every execution term comes from the durable record on both paths.
    issued: Mutex<HashMap<SwapId, Instant>>,
    /// §3.5 layer-3/4 kill state. Starts `Live`; a host escalates it at
    /// runtime via [`set_kill`](Self::set_kill) (e.g. from the signed
    /// network manifest, W5). A `watch::Sender` (not a `Mutex`) so an in-flight
    /// [`run_swap_status_poll`] loop can OBSERVE a `Hard` escalation and stop
    /// PROMPTLY (the §3.5 "zero swap traffic from this instant" stop — interrupts
    /// the poll's between-poll sleep, so no straggler `status` call fires after a
    /// Hard kill). The `Sender` IS the authority: read with `borrow()`, escalate
    /// with `send_if_modified` — which ALWAYS updates the stored value and returns
    /// no `Err` regardless of receiver count, so no retained receiver is needed
    /// (unlike `Sender::send`, whose zero-receiver `Err` the status channel sidesteps
    /// with a keepalive). A `subscribe()` after an escalation still observes the
    /// latched value.
    kill: watch::Sender<SwapKill>,
}

impl SwapService {
    pub fn new(
        provider: Arc<dyn SwapPort>,
        refund_source: Arc<dyn RefundAddressSource>,
        destination_source: Arc<dyn DestinationAddressSource>,
        deposit_sender: Arc<dyn DepositSender>,
        issued_store: Arc<dyn IssuedQuoteStore>,
        record_sink: Arc<dyn SwapRecordSink>,
        network: Network,
    ) -> Self {
        Self {
            provider,
            refund_source,
            destination_source,
            deposit_sender,
            issued_store,
            record_sink,
            network,
            issued: Mutex::new(HashMap::new()),
            kill: watch::channel(SwapKill::Live).0,
        }
    }

    /// Escalate the swap kill severity (§3.5 layers 3/4). MONOTONIC-TOWARD-OFF
    /// (review M4): this only ever moves toward MORE-off (Live < WindDown <
    /// Hard); a directive weaker than the current state is a NO-OP. The service
    /// never resurrects swap — re-enabling is host reconstruction (layer 2), so
    /// a forged/replayed "on" flag can never revive a killed instance. Hosts
    /// pass a DEFINED severity via [`resolve_manifest_kill`](super::resolve_manifest_kill).
    pub fn set_kill(&self, severity: SwapKill) {
        // escalate-only: `SwapKill`'s derived `Ord` IS the monotonic-off ordering
        // (Live < WindDown < Hard); a weaker directive is a no-op. `send_if_modified`
        // updates the watched value AND notifies subscribers ONLY on a real change,
        // so an in-flight poll loop is woken by a genuine escalation but never by a
        // no-op weaker directive.
        self.kill.send_if_modified(|k| {
            if severity > *k {
                *k = severity;
                true
            } else {
                false
            }
        });
    }

    /// The current kill severity (a cheap `watch::borrow`). Crate-internal — the
    /// poll loop subscribes for live escalations; `ensure_live` and tests read
    /// the point-in-time value here.
    pub(crate) fn kill_severity(&self) -> SwapKill {
        *self.kill.borrow()
    }

    /// The door both `quote` and `execute` check FIRST: a killed instance
    /// refuses new work typed (`SwapDisabled`), before any provider call or
    /// registry mutation. Both severities refuse NEW work — the WindDown↔Hard
    /// divergence is on in-flight status polling (status-poll chunk).
    ///
    /// Accepted TOCTOU bound (§3.5): a `set_kill` that lands AFTER this check
    /// but before the subsequent provider `.await` lets that ONE in-flight
    /// request finish; monotonicity guarantees every SUBSEQUENT call is
    /// refused. One request completing on the heels of a near-simultaneous kill
    /// is within the honest-off contract (the deposit settles/refunds
    /// provider-side regardless) — and a hard kill's network teardown
    /// (`wipe`/`set_dormant`, §4.7) is the harder stop for traffic already in
    /// flight.
    fn ensure_live(&self) -> Result<(), SwapError> {
        if self.kill_severity().is_killed() {
            return Err(SwapError::SwapDisabled);
        }
        Ok(())
    }

    /// Validate the request, fetch a quote, and bounds-check it against the
    /// USER's own numbers (M1) — a quote that fails here is never returned,
    /// so nothing out-of-bounds can ever be accepted, displayed-then-signed,
    /// or executed.
    pub async fn quote(&self, req: QuoteRequest) -> Result<SwapQuote, SwapError> {
        // §5.4 `wallet.swap_quote` span (§3.3b L5 / ADR-0530) — the quote-time destination-MINT is
        // the highest-risk address site, so it gets the symmetric span to `wallet.swap` (execute):
        // the provider NAME + a coarse DIRECTION code (known up front from the request) + the outcome
        // CODE only; NEVER the swap id, destination/deposit address, refund address, or any amount.
        // `.instrument` (not `enter`) across the `.await`s — an entered guard held over a yield
        // mis-attributes later spans on the executor thread — and the outcome recorded at the single
        // exit so a destination-mint DB-lock vs store-unavailable vs provider failure is
        // distinguishable by its `RW-SWAP-NNN` code without any secret crossing.
        let span = tracing::info_span!(
            target: "zec_wallet_core",
            "wallet.swap_quote",
            provider = self.provider.name(),
            direction = direction_code(&req.direction),
            outcome = tracing::field::Empty,
        );
        let result = self.quote_inner(req).instrument(span.clone()).await;
        span.record("outcome", quote_outcome_code(&result));
        result
    }

    /// The quote body (see [`Self::quote`] for the §5.4 span contract). Split out so the public
    /// `quote` owns the `wallet.swap_quote` span and records the terminal outcome uniformly across
    /// every `?`-propagated return.
    async fn quote_inner(&self, req: QuoteRequest) -> Result<SwapQuote, SwapError> {
        self.ensure_live()?; // §3.5: a killed instance emits ZERO swap traffic
        validate_request(&req, self.network)?;
        // OutOfZec: mint the fresh refund t-addr (engine-registered + cross-checked at the
        // source since #368) and bind it as the provider refundTo; returns the durable
        // handle recorded below so execute can arm the refund watch. IntoZec leaves `None`.
        let (mut outgoing, refund) = self.populate_refund(req).await?;
        // IntoZec: mint a fresh engine-persisted ZEC destination + bind it as `recipient`
        // (§3.3b D1 / ADR-0530). The minted destination (address + single-use external index)
        // is recorded durably below so the §3.3b L2 scoped detection set survives a crash.
        // OutOfZec leaves this `None` (the user supplied the foreign receive address;
        // `validate_quote` blesses the deposit instead). Done BEFORE `provider.quote` because
        // the quote request BINDS `recipient` — the destination must exist when we ask.
        let destination = self.populate_destination(&mut outgoing).await?;
        let mut quote = self.provider.quote(outgoing.clone()).await?;
        // The blessed transparent deposit `Address` (OutOfZec) or `None` (IntoZec).
        // Captured ONCE here and stored in the registry so execute threads the EXACT
        // validated value into the deposit send (L-1) — a second independent parse
        // could admit a wrong-network/non-transparent recipient `Payment::new` won't
        // catch (it accepts any valid recipient; only THIS check enforces §4.4).
        let blessed = validate_quote(&quote, &outgoing, self.network)?;
        // Direction-aware deadline posture (W-swap-4-a-3, the security review fold): the
        // window WE requested is the ceiling on every STORED deadline — for a
        // deposit-carrying (OutOfZec) quote the 15-min
        // `SWAP_DEPOSIT_DEADLINE_OUT_OF_ZEC_SECS` (an echo past our request is hostile
        // or buggy; honoring it kept the quote executable — and every §4.4 lockout
        // armed — 96× longer than the wallet's stated need), for IntoZec the 24 h
        // default (the user's EXTERNAL deposit genuinely owns that window). The
        // execute margin is direction-aware too: a deposit execute must leave the feed
        // gates' `DEPOSIT_FIRST_FEED_MARGIN_SECS` of mining room PLUS prove/kick
        // headroom (`DEPOSIT_EXECUTE_MARGIN_SECS`, strictly larger — an execute
        // admitted with less would enqueue a deposit already dead at its own gates).
        let (deadline_margin, window_ceiling) = if blessed.is_some() {
            (
                DEPOSIT_EXECUTE_MARGIN_SECS,
                SWAP_DEPOSIT_DEADLINE_OUT_OF_ZEC_SECS,
            )
        } else {
            (DEADLINE_SAFETY_MARGIN_SECS, SWAP_DEADLINE_DEFAULT_SECS)
        };
        // Wall clock captured BEFORE the persist so the durable and in-memory records
        // carry the SAME ceiling-clamped wall deadline (a pre-epoch clock reads
        // `u64::MAX` ⇒ the clamp is a no-op and `valid_for` below saturates to 0 —
        // the quote is born expired, unchanged posture).
        let wall_now = now_unix();
        let expires_at_wall = quote
            .expires_at
            .min(wall_now.saturating_add(window_ceiling));
        // #367: the RETURNED quote's `expires_at` is the ACTIONABLE deadline —
        // `wall_now + valid_for` = the ceiling-clamped wall expiry minus the
        // direction-aware execute margin, i.e. the exact instant `execute`
        // starts refusing `QuoteExpired`. The raw provider echo (observed live
        // at ~72–96 h against a ~10-min actionable window — the
        // hardware-photographed countdown lie) never reaches a host: a
        // countdown rendered from this field and the execute gate agree by
        // construction. `expires_at_wall` is already ceiling-clamped to OUR
        // requested window (security fold, W2 review + the W-swap-4-a-3
        // direction-aware re-price above), so the subtraction needs no second
        // `.min()`. Set BEFORE the persist (S8, row 2): the deadline the approval
        // shows is one of the recorded terms, recorded verbatim off this very DTO,
        // so the execute compare is exact by construction. Enforcement is
        // untouched — the registry/durable rows capture the un-margined wall
        // deadline, not this display value.
        let valid_for = expires_at_wall
            .saturating_sub(wall_now)
            .saturating_sub(deadline_margin);
        quote.expires_at = wall_now.saturating_add(valid_for);
        // Durably record the issued quote BEFORE returning it (the cross-restart truth,
        // W-swap-3-c-2): a quote we cannot durably record is NOT issued (fail-closed —
        // a store error surfaces typed `SwapStateUnavailable`, nothing leaves the pool).
        // FIRST-WINS + the in-flight cap live in the store; `AtCapacity` is the crash-loop
        // backstop, mapped to the SAME `RequestInvalid` the in-memory registry returns
        // (one cap message). Persist BEFORE the in-memory insert so the durable row is
        // never absent for an in-memory-live quote (the in-memory insert is infallible).
        // BOTH directions are recorded (IntoZec carries a `None` deposit) so `execute`'s
        // durable consume is uniform — the store already models the no-deposit row.
        // FR-17: mint the spend binding for THIS quote — recorded durably with the
        // issued row AND surfaced on the returned DTO below, so the value the host
        // records at its execute-authorize bracket is the value the deposit's sign
        // pull presents. SDK-minted (never the provider quote id — provider-
        // controlled, no uniqueness contract).
        let binding = SpendBinding::mint();
        // S8 (R01): the EXECUTION IDENTITY is minted here too, from the OS CSPRNG,
        // independent of anything the provider sent — the adapter's `id` (the
        // provider's own handle; 1Click: the deposit address) comes OFF the DTO
        // and is kept as `provider_ref`, data for the status poll. A provider
        // that reuses or reissues a deposit address across quotes therefore
        // yields two records under two keys, never one overwritten by the other.
        let provider_ref = quote.id.as_str().to_owned();
        quote.id = SwapId::mint();
        let persisted = self
            .issued_store
            .persist(IssuedQuoteRecord {
                id: quote.id.clone(),
                provider_ref,
                // The terms the approval shows — recorded for BOTH directions and
                // compared against the caller's DTO at execute before the claim.
                terms: QuoteTerms::of(&quote),
                deposit: blessed.as_ref().map(|_| IssuedDeposit {
                    // the raw provider deposit string `validate_quote` already blessed
                    // (transparent + right-network); frozen per id, stored, never re-parsed here
                    address: quote.deposit_address.clone(),
                    amount: quote.zec_side,
                }),
                // The IntoZec destination (mutually exclusive with `deposit` by direction —
                // `blessed` is `Some` only for OutOfZec, `destination` only for IntoZec): the
                // engine-persisted address + its single-use index, recorded so a crash between
                // quote and execute reconstructs the §3.3b L2 detection-set entry. Moved (last use).
                destination,
                // The OutOfZec refund handle (#368 — rides WITH the deposit): recorded so
                // execute can arm the refund watch from the taken record. Moved (last use).
                refund,
                // The CLAMPED wall deadline (W-swap-4-a-3): the cross-restart execute
                // gate reads this row, so a hostile far-future echo must be bounded
                // HERE, not only in the in-memory half.
                expires_at_wall,
                binding: Some(binding),
            })
            .await?;
        if matches!(persisted, IssuedQuotePersist::AtCapacity) {
            return Err(SwapError::RequestInvalid {
                reason: "too many in-flight quotes — execute or let one expire",
            });
        }
        // Provider-relative duration captured at response receipt; primary
        // enforcement is monotonic from here — a slow/regressed device clock
        // can't sign deposits to dead quotes. (The wall half of the dual gate
        // lives in `IssuedQuote::expires_at_wall` — suspend-paused monotonic.)
        let receipt = Instant::now();
        let now = Instant::now();
        let mut issued = self.issued.lock().expect("issued-quote registry poisoned");
        // cheap sweep (security fold): expired entries leave at insert time, so a
        // quote-polling host never grows the registry unboundedly. This is expiry-only
        // cleanup, NOT cap enforcement — the in-flight cap lives ONCE in the durable store
        // (the `AtCapacity` short-circuit above already returned), so there is no second
        // in-memory cap to drift from it (`MAX_ISSUED_QUOTES`, one SSOT).
        issued.retain(|_, act_deadline| *act_deadline > now);
        // Only the monotonic deadline half lives here (S8): the blessed deposit
        // `Address` is rebuilt from the durable row at execute, on both paths.
        issued.insert(quote.id.clone(), receipt + Duration::from_secs(valid_for));
        drop(issued);
        // FR-17: the returned quote carries the minted binding (the durable row above
        // recorded the same value) — the host's authorize bracket records it from the
        // DTO; adapters populate `None` (they never mint, the service is the authority).
        quote.binding = Some(binding);
        Ok(quote)
    }

    /// Register intent with the provider and, for an OutOfZec swap, drive the
    /// §4.4 deposit send. Accepts ONLY quotes this service issued (already
    /// bounds-checked), one-shot, behind the terms compare, the durable claim and
    /// the deadline gate:
    ///
    /// 1. **Terms compare** (S8, R01) — read the durable record the DTO names
    ///    WITHOUT consuming it (`peek`) and compare the DTO's terms
    ///    ([`QuoteTerms`] + the FR-17 binding) against it field-by-field. A
    ///    difference is the typed [`SwapError::QuoteTermsDiffer`]: zero provider
    ///    calls, zero deposit, and the record stays claimable — the DTO is not the
    ///    one this service issued (or was edited on the way back), and a later
    ///    honest call still executes. Defence in depth over the identity: even a DTO
    ///    that passes a host's approval with one record's numbers cannot execute
    ///    another record's.
    /// 2. **Atomic durable claim** (W-swap-3-c-3-ii) — `take` the durable issued-quote
    ///    row. This is THE single-flight: the `IMMEDIATE` SELECT-then-DELETE
    ///    claims a quote exactly once, so a second execute (a double-tap, a retry after a
    ///    perceived failure, a concurrent call, or a post-restart replay) finds it gone ⇒
    ///    `QuoteExpired` — the provider is never asked to register a second intent and no
    ///    second deposit is ever queued (the swap-layer no-double-deposit guarantee, now
    ///    un-racy across a restart; the §6.3 guard only de-dups a SINGLE intent). The cost —
    ///    a transient `provider.execute` failure means re-quote, not retry — is the fail-safe
    ///    posture (re-quoting surfaces any price move). EVERY execution term — deposit
    ///    address and amount, deadline, binding, refund/destination watch, the provider
    ///    handle — is read from the taken record on both paths; the in-memory registry
    ///    supplies only the monotonic deadline half within the issuing run.
    /// 3. **Deadline pre-flight**: reject if expired. Within-process it is the DUAL gate
    ///    (monotonic OR wall); cross-restart it is the durable WALL deadline alone (a
    ///    monotonic `Instant` can't survive a restart). This is the FAST-PATH rejection of
    ///    an ALREADY-dead quote — NOT the authoritative "never sent to a dead quote"
    ///    guarantee: a quote can lapse DURING the `provider.execute` await (a slow Tor
    ///    circuit) after this gate passed, so the deposit can be QUEUED to a now-lapsed
    ///    quote. That is money-safe — the deposit only PERSISTS here; the durable
    ///    `deposit_gate` (inc-2d-swap-a) re-checks the wall deadline on every resubmission
    ///    pass and DELETES the intent before any propose/sign/broadcast, so it is never
    ///    SENT. The durable gate is the real guarantee; this pre-flight just avoids queuing
    ///    an obviously-dead one. It runs AFTER the claim on purpose: an expired quote is
    ///    consumed, so recovery never re-drives it either.
    /// 4. **Register intent** with the provider FIRST (so it expects the deposit),
    ///    THEN queue the deposit. A kill between the two is fail-safe: a registered
    ///    quote with no deposit refunds provider-side after the deadline (nothing
    ///    left the pool); a deposit can never precede a successful `provider.execute`.
    ///
    /// IntoZec carries no `deposit` plan ⇒ step 4's send is skipped (ZEC is RECEIVED
    /// to our own fresh address). The deposit itself is offline-first + crash-safe in
    /// the [`DepositSender`] impl; this returns the swap's SDK-minted id either way —
    /// the same value the DTO carried, the home row is keyed by, and `watch_status`
    /// takes.
    pub async fn execute(&self, quote: &SwapQuote) -> Result<SwapId, SwapError> {
        // §5.4 `wallet.swap` span — the provider NAME + a coarse DIRECTION code +
        // the outcome CODE only; NEVER the swap id, deposit address, amount, or any
        // money field. Mirrors the engine's per-range `wallet.sync` batch span:
        // `.instrument` (not `enter`) across the `.await`s — an entered guard held
        // over a yield mis-attributes later spans on the executor thread — and
        // `direction`/`outcome` recorded as the path resolves. Emitted on EVERY
        // execute, including a killed / expired / not-issued rejection, so a host
        // sees the on-ramp's outcome distribution without ever seeing a secret.
        let span = tracing::info_span!(
            target: "zec_wallet_core",
            "wallet.swap",
            provider = self.provider.name(),
            direction = tracing::field::Empty,
            outcome = tracing::field::Empty,
        );
        let result = self.execute_inner(quote).instrument(span.clone()).await;
        // Coarse, secret-free outcome (`"ok"` or the stable `RW-SWAP-NNN` code) —
        // the same payload-free code the port boundaries log; recorded after the
        // instrumented body so every return path (incl. an early `?`) lands here.
        span.record("outcome", execute_outcome_code(&result));
        result
    }

    /// The execute body (see [`Self::execute`] for the step-by-step contract).
    /// Split out so the public `execute` owns the §5.4 span and records the
    /// terminal outcome uniformly across every `?`-propagated return.
    async fn execute_inner(&self, quote: &SwapQuote) -> Result<SwapId, SwapError> {
        self.ensure_live()?; // §3.5: no execute past a kill (before any consume) — zero swap traffic

        // TERMS COMPARE (S8, R01) — before the claim, on a non-consuming read. The
        // record the DTO names is the authority for every term below; the DTO is the
        // host's echo of what the user approved. Any field-by-field difference (the
        // approval's numbers under another record's id, an edited address, a
        // stripped binding) is refused typed with the record intact: nothing was
        // consumed, nothing was sent, and the honest DTO still executes. A miss here
        // is the same `QuoteExpired` a take-miss is (below).
        let peeked = self
            .issued_store
            .peek(&quote.id)
            .await?
            .ok_or(SwapError::QuoteExpired)?;
        // The wall-clock half of the deadline gate — the deadline authority on both
        // paths (a monotonic `Instant` can't survive a process exit). Computed ONCE,
        // here, so the pre-claim check below and the gate after the claim read the
        // same instant.
        let wall_now = now_unix();
        // A record the wall gate would already refuse is NOT compared: it goes to
        // the claim and the gate, which refuse it `QuoteExpired` and consume it
        // exactly as before. So a born-expired or lapsed quote is reported by its
        // true reason — "get a new quote" — never as "terms differ", whatever its
        // DTO says; and the one shape in which an honest DTO's `expires_at` cannot
        // round-trip exactly (a pre-epoch clock's `u64::MAX`, which the bridge's
        // i64 cast folds) is dead here by construction.
        // The `binding` compare is a derived (short-circuiting) equality on purpose:
        // the host — the only caller — holds the binding on the DTO it was issued,
        // and no network path lets anyone else time it (crypto pass; recorded).
        let wall_dead = wall_now.saturating_add(execute_margin(peeked.deposit.is_some()))
            >= peeked.expires_at_wall;
        if !wall_dead && (QuoteTerms::of(quote) != peeked.terms || quote.binding != peeked.binding)
        {
            return Err(SwapError::QuoteTermsDiffer);
        }
        drop(peeked);

        // ATOMIC CLAIM (W-swap-3-c-3-ii): the durable `take` is THE single-flight for
        // EVERY execute — within-process AND cross-restart. Its `IMMEDIATE`
        // SELECT-then-DELETE serializes concurrent claims, so exactly ONE execute per
        // quote id gets `Some`; a double-tap / replay / second execute (incl. one that
        // passed the compare above a moment earlier) gets `None` ⇒ `QuoteExpired`, so
        // no second provider intent and no second deposit can ever be queued (the
        // no-double-deposit guarantee, now un-racy across restart). A durable `None` is
        // LOAD-BEARING (no durable claim ⇒ no authority to execute), and a production
        // `None` ⟺ the row was pruned because it lapsed ⟺ the quote is dead anyway. A
        // store FAILURE aborts fail-safe (provider not yet called, nothing queued) as
        // typed `SwapStateUnavailable`.
        // A take MISS is typed `QuoteExpired`, not the generic `RequestInvalid`
        // (#367, the swept-row cross-note): every production path here — the
        // hygiene sweep of a pre-upgrade far-future row, a lapsed row's prune, a
        // >1 h backward clock step, an already-executed replay — ends in the same
        // user action, "this quote is no longer valid — get a new quote". The old
        // "not issued by this service" read as an integrity accusation for a
        // quote the user minted moments ago on an old build.
        let durable = self
            .issued_store
            .take(&quote.id)
            .await?
            .ok_or(SwapError::QuoteExpired)?;

        // The quote's durable wall deadline — captured BEFORE the arms below move
        // `durable.deposit`. This is the W-swap-5 record's `deposit_deadline` for
        // BOTH directions: OutOfZec it is the same tag `send_deposit_now` gates on;
        // IntoZec it is the user's external deposit window.
        let quote_expires_wall = durable.expires_at_wall;

        // FR-17: the issued row's spend binding (Copy), captured before the arms move
        // `durable.deposit` — threaded onto the deposit's durable intent row so the
        // sign-at-execute pull presents the value the host recorded from the quote DTO.
        // `None` only for a pre-FR-17 durable row (its deposit signs unbound, honest).
        let deposit_binding = durable.binding;

        // #368: the swap's watched inbound transparent leg — IntoZec: the destination
        // (whose settlement extension MOVED here from the take port, so a refused execute
        // below no longer parks a 48 h watch); OutOfZec: the refund handle (armed for the
        // first time — nothing can land at it before the deposit leaves). Captured BEFORE
        // the arms below move `durable.deposit`; threaded into `record_started` so the
        // sink arms it ATOMICALLY with the home row. A far-future-SWEPT quote never gets
        // here (`take` returned `None` above) — its quote-time destination row dies at the
        // clamped quote deadline, the documented bounded asymmetry.
        let watch = durable
            .destination
            .as_ref()
            .map(|d| SwapWatch {
                address: d.address.clone(),
                index: d.index,
            })
            .or_else(|| {
                durable.refund.as_ref().map(|r| SwapWatch {
                    address: r.address.clone(),
                    index: r.index,
                })
            });

        // The monotonic deadline half: present iff the quote was issued in THIS process
        // run (removed here — the claim above is one-shot, so nothing else will read it;
        // no `.await` under the lock). Absent ⇒ a CROSS-RESTART execute ⇒ the durable
        // wall deadline gates alone.
        let act_deadline = self
            .issued
            .lock()
            .expect("issued-quote registry poisoned")
            .remove(&quote.id);
        // The deadline gate: reject if EITHER clock says expired — monotonic defends
        // against a regressed wall clock, wall defends against a suspend-paused
        // monotonic (`Instant` PAUSES during device suspend on every shipped platform,
        // so a phone asleep between quote and execute under-counts elapsed time). The
        // wall half is `now + MARGIN >= deadline` (the same `>=` lapse edge the
        // inc-2d-swap-a `deposit_gate` uses) against the DURABLE record's wall deadline
        // on both paths. The margin is direction-aware (W-swap-4-a-3): a
        // deposit-carrying quote is gated on `DEPOSIT_EXECUTE_MARGIN_SECS` — STRICTLY
        // stricter than the feed gates' `DEPOSIT_FIRST_FEED_MARGIN_SECS`, so an admitted
        // execute can always still sign + first-broadcast with mining room to spare.
        //
        // No CLOCK-PLAUSIBILITY FLOOR here (unlike `deposit_gate`, send.rs): a backward
        // wall-clock jump could let this PRE-FLIGHT pass a lapsed quote. That is
        // money-safe by design — the deposit rides into the outbox tagged with
        // `expires_at_wall`, and the durable `deposit_gate` (the REAL guarantee)
        // re-checks it WITH the plausibility floor before any sign/broadcast: a sub-floor
        // clock ⇒ `Wait` (non-destructive), a plausible-but-lapsed clock ⇒ `Expired` ⇒
        // deleted, never signed. This pre-flight only avoids queuing an obviously-dead one.
        let margin = execute_margin(durable.deposit.is_some());
        if act_deadline.is_some_and(|d| Instant::now() >= d)
            || wall_now.saturating_add(margin) >= durable.expires_at_wall
        {
            return Err(SwapError::QuoteExpired);
        }
        // The §4.4 deposit plan, rebuilt from the durable string on BOTH paths: the
        // address is re-parsed + re-blessed transparent + right-network (exactly the
        // `validate_quote` checks that blessed it at persist), so a tampered sealed row
        // fails closed (no deposit to a garbage address) and a within-process execute
        // runs the same record a post-restart one does.
        let deposit = match durable.deposit {
            Some(d) => Some((
                reconstruct_deposit_plan(d, self.network)?,
                durable.expires_at_wall,
            )),
            None => None,
        };

        // §5.4: the swap DIRECTION is now known — OutOfZec carries a wallet-side deposit
        // plan, IntoZec does not. A coarse public code (no asset names, no amounts) onto
        // the live `wallet.swap` span, recorded HERE (past the claim + deadline gate) so
        // it reflects a real, proceeding swap; a kill / not-issued / expired rejection
        // returned earlier and leaves `direction` Empty (honest — and Empty isn't emitted).
        tracing::Span::current().record(
            "direction",
            if deposit.is_some() {
                "out_of_zec"
            } else {
                "into_zec"
            },
        );
        // Step 4: register intent FIRST. On failure the quote is already consumed —
        // no deposit was queued, the user re-quotes (fail-safe; no funds in motion).
        self.provider.execute(quote).await?;
        // The swap's identity from here on is the durable record's SDK-minted id —
        // the value the DTO carried and the home row is keyed by.
        let id = durable.id;
        // W-swap-5 (#366): the durable in-flight RECORD, written RECORD-FIRST —
        // after the provider registered intent (so the row never asserts a fiction)
        // and BEFORE the deposit leg (so a deposit can never be durably queued
        // without its home row). A record failure propagates and the deposit is
        // NEVER queued — fail-safe: the registered-but-unfunded order
        // expires/refunds provider-side, the same accepted bound as the §3.5
        // mid-execute kill abort right below. The provider handle rides the row —
        // it is what a later `watch_status` resolves the poll's request value from.
        self.record_sink
            .record_started(StartedSwap {
                id: id.clone(),
                provider_ref: durable.provider_ref,
                out_of_zec: deposit.is_some(),
                deposit_deadline: Some(quote_expires_wall),
                expires_at_wall: wall_now.saturating_add(SWAP_SETTLEMENT_MAX_SECS),
                watch,
            })
            .await?;
        if let Some((plan, deadline)) = deposit {
            // §3.5 honest-off RE-CHECK before the money-moving leg: `ensure_live`
            // ran at entry, but `provider.execute` is an `.await` — a `set_kill(Hard)`
            // landing DURING it must still stop a NEW deposit from being queued (zero
            // swap traffic from the kill instant). The provider intent is already
            // registered, but a registered-but-undeposited quote is fail-safe (it
            // refunds provider-side after the deadline — nothing left the pool). A
            // kill landing AFTER `send_deposit_now` persists is the §3.5 accepted bound
            // (the deposit is then a generic durable send; a Hard kill's network
            // teardown is the stop for traffic already in flight).
            self.ensure_live()?;
            // The OutOfZec deposit — the blessed address (L-1) + the ZEC input + the wall
            // deadline that rides into the durable intent so the §6.1 resubmission machinery
            // re-checks it (never feeds a dead quote, §4.4). SIGN-AT-EXECUTE (FR-23-a): the
            // wallet signs the deposit here, in the user-present authorize-spend bracket, so
            // swap works at every custody tier — not at a later background drain the
            // host-custody port can't serve.
            self.deposit_sender
                .send_deposit_now(plan.address, plan.amount, deadline, deposit_binding)
                .await?;
        }
        Ok(id)
    }

    /// OutOfZec: the provider's refundTo is ALWAYS a fresh wallet t-addr —
    /// the user-supplied field is ignored by design (§2.6) — and since #368
    /// the minted handle (address + single-use index) is ALSO returned so
    /// `quote` records it durably (the `populate_destination` mirror).
    /// IntoZec: the user's source-chain refund target passes through,
    /// `None` handle (its inbound leg is the destination).
    async fn populate_refund(
        &self,
        mut req: QuoteRequest,
    ) -> Result<(QuoteRequest, Option<IssuedRefund>), SwapError> {
        if matches!(req.direction, SwapDirection::OutOfZec { .. }) {
            let refund = self.refund_source.fresh_refund_address().await?;
            req.refund_address = Some(refund.address.clone());
            Ok((req, Some(refund)))
        } else {
            Ok((req, None))
        }
    }

    /// IntoZec: mint a FRESH engine-persisted ZEC destination and bind it as the provider
    /// `recipient` (§3.3b D1 / ADR-0530 — the MIRROR of `populate_refund`: refunds are the
    /// OutOfZec leg, destinations the IntoZec leg). Returns the minted destination (address +
    /// single-use index) so `quote` records it durably; `None` for OutOfZec (the user supplied
    /// the foreign receive address, `validate_request` already checked it). `validate_request`
    /// has already rejected a caller-supplied IntoZec destination, so the field is empty here —
    /// the wallet is the sole minter. A mint failure is the typed, payload-free
    /// `DestinationAddressUnavailable` (port isolation; the address is §5.4 never-log).
    async fn populate_destination(
        &self,
        req: &mut QuoteRequest,
    ) -> Result<Option<IssuedDestination>, SwapError> {
        if matches!(req.direction, SwapDirection::IntoZec { .. }) {
            let destination = self.destination_source.fresh_destination().await?;
            req.destination = Some(destination.address.clone());
            Ok(Some(destination))
        } else {
            Ok(None)
        }
    }

    /// Stream the live [`SwapStatus`] of one in-flight swap into `sink` on a
    /// DETACHED task, returning once the task is spawned (the `watch_swap_status`
    /// surface — the [`crate::Wallet::watch_sync_status`] parallel for swaps). The
    /// task polls `provider.status(id, provider_ref)` at the §7 cadence
    /// (`SWAP_POLL_INITIAL_SECS` → `SWAP_POLL_MAX_SECS` exponential), emits each
    /// status, and STOPS on a terminal status, on host cancel (`emit` → `false`),
    /// or on a `Hard` kill (§3.5). See [`run_swap_status_poll`] for the full contract.
    ///
    /// **The pre-spawn resolution (S8):** `id` is the SDK-minted execution identity
    /// — the value `execute` returned, the home row is keyed by, and a host feeds back
    /// from `list_in_flight_swaps` after a reopen. The provider's own handle (what the
    /// request must carry) is resolved from the durable home row through the record
    /// port HERE, before the task exists: never inside the task (it captures only the
    /// provider `Arc` + the kill receiver), never from an in-memory id→handle cache
    /// (dead after a reopen; a rescan keeps the swap tables). An id with no home row
    /// is a typed `RequestInvalid` — a host bug (an id it never received, or a
    /// dismissed record), never a spinning stream.
    ///
    /// The spawned task holds ONLY a clone of the provider `Arc`, the resolved handle
    /// and a kill `watch::Receiver` — NEVER `&self`/the wallet `Inner` — so it cannot
    /// pin a torn-down wallet (the [`crate::Wallet::watch_sync_status`] no-lock-pin
    /// contract). The kill `Receiver` doubles as the teardown signal: when the
    /// `SwapService` (and so its kill `watch::Sender`) drops, the loop's `changed()`
    /// errors and the task ends — see the `kill` field doc for why no keepalive
    /// receiver is retained. Spawns onto the current tokio runtime (the caller — the
    /// SDK async API or the FFI bridge — is inside one).
    pub async fn watch_status(
        &self,
        id: SwapId,
        sink: impl SwapStatusSink,
    ) -> Result<(), SwapError> {
        let provider_ref =
            self.record_sink
                .provider_ref(&id)
                .await?
                .ok_or(SwapError::RequestInvalid {
                    reason: "no in-flight swap record carries this id",
                })?;
        // Detached: the loop is self-terminating (terminal / host-cancel / Hard),
        // so there is nothing to join; dropping the `JoinHandle` does NOT cancel it.
        // INVARIANT (the type system can't enforce it — keep it true by hand): the
        // spawned future may capture ONLY the provider `Arc` clone, the resolved
        // handle + the kill `Receiver`, NEVER `self`/`Inner` — a captured `&self`
        // would pin a torn-down wallet and reintroduce the close-hang the
        // no-lock-pin contract avoids.
        tokio::spawn(run_swap_status_poll(
            Arc::clone(&self.provider),
            self.kill.subscribe(),
            id,
            provider_ref,
            sink,
        ));
        Ok(())
    }

    /// The dynamic source-asset list for the host's IntoZec picker (§3.3b D5/L6 / ADR-0530, IZ-2).
    /// Delegates to the provider's FILTERED + cached [`SwapPort::list_tokens`]: the adapter owns the
    /// `/v0/tokens` fetch, its dedicated circuit isolation (never the sync circuit, §2.3), and the
    /// serve-stale-on-fault fallback ([`TokenList::fresh`]` = false`). A killed instance emits ZERO
    /// swap traffic (§3.5) — `ensure_live` gates first. The §5.4 `wallet.swap_tokens` span records
    /// ONLY the provider label, the pickable-token COUNT, and a coarse outcome code (`"ok"` fresh /
    /// `"stale"` served-from-cache / the `RW-SWAP-NNN` fault) — NEVER a symbol, chain, id, or price.
    pub async fn list_tokens(&self) -> Result<TokenList, SwapError> {
        self.ensure_live()?; // §3.5: a killed instance lists nothing — zero swap traffic
        let result = self.provider.list_tokens().await;
        let count = result.as_ref().map(|l| l.tokens.len()).unwrap_or(0);
        let outcome = match &result {
            Ok(list) if list.fresh => "ok",
            Ok(_) => "stale", // served from the last good cache (the host's "couldn't refresh" banner)
            Err(e) => e.code(),
        };
        tracing::info!(
            target: "zec_wallet_core",
            provider = self.provider.name(),
            count,
            outcome,
            "wallet.swap_tokens",
        );
        result
    }
}

/// A consumer of one swap's live [`SwapStatus`] stream (§7; the §3.3 outbound
/// pattern). [`SwapService::watch_status`] drives one from a DETACHED poll task;
/// the FFI bridge implements it over a flutter_rust_bridge `StreamSink` (the D
/// slice), and tests implement it over a recording buffer — so the core owns the
/// poll loop and its runtime while staying ignorant of the bridge (the
/// `SyncStatusSink` precedent: the core never depends on flutter_rust_bridge, and
/// the bridge needs no async runtime of its own).
pub trait SwapStatusSink: Send + 'static {
    /// Deliver one status downstream. Return `true` to keep polling, or `false`
    /// if the downstream is closed/cancelled (the host dropped its Dart
    /// subscription on `AppLifecycleState.paused` / provider dispose — the
    /// §7 FOREGROUND-ONLY stop). The poll loop then ends PROMPTLY and drops,
    /// holding nothing.
    fn emit(&mut self, status: SwapStatus) -> bool;
}

/// Drive one swap's status into `sink`, polling `provider.status(id)` at the §7
/// cadence until a terminal outcome. The awaitable core of
/// [`SwapService::watch_status`] (a free `async fn` so tests drive it directly on
/// the virtual clock — no spawn, no wall-clock wait). Contract:
///
/// - **CURRENT-first:** the first poll fires IMMEDIATELY (no initial sleep), so a
///   re-subscribing host (a mobile resume) sees the latest status at once.
/// - **§7 cadence:** between polls, an EXPONENTIAL `SWAP_POLL_INITIAL_SECS` →
///   `SWAP_POLL_MAX_SECS` sleep (provider settle times are tens of seconds to
///   minutes). Foreground-only: this is NOT a background timer — on mobile the
///   process suspends and freezes the task; the host cancels (below) on desktop.
/// - **Stops on TERMINAL** (`SwapStatus::is_terminal`): the terminal status is
///   emitted, THEN the stream ends (the host sees the final value before EOF).
/// - **Stops on HOST CANCEL** (`emit` → `false`): the §7 foreground-only stop.
/// - **§3.5 WindDown↔Hard divergence:** a `Hard` kill (`stops_polling`) ends the
///   loop — zero `status` traffic from this instant — WITHOUT a synthetic
///   terminal (the host renders "tracking-unavailable" from its own Hard state;
///   funds settle/refund provider-side regardless, §3.5). A Hard kill landing
///   mid-sleep WAKES the sleep (the `watch` change), so no straggler poll fires.
///   `WindDown` is NOT `stops_polling`, so it keeps polling in-flight to terminal
///   (status is OBSERVATIONAL — the §3.5 default).
/// - **NEVER ends on a transient fault:** a `provider.status` error is logged-free
///   and RETRIED with backoff (the `watch_sync_status` "a stall is DATA, not a
///   dead stream" contract) — only a terminal / cancel / Hard ends the stream.
/// - **EXCEPT the provider-definitive `SwapNotFound` (#367 poll policy):**
///   `SWAP_NOT_FOUND_TERMINAL_POLLS` CONSECUTIVE not-found answers synthesize a
///   terminal `Failed(NotFound)` and end the stream (`outcome = "not_found"`).
///   Pre-#367 a 404 was retried forever — harmless while tracking was one-shot,
///   but the W-swap-5 durable home made re-attaching to a provider-GC'd order a
///   first-class flow, and that re-attach spun "checking" eternally (the
///   wedge root). Not first-404-is-terminal: a transient 404 while the provider
///   indexes a just-executed order must not false-fail a live swap — the counter
///   resets on ANY other result (a real status, a transport fault, a 5xx).
///
/// Observability: a §5.4-clean `wallet.swap_poll {provider, outcome, faults}` span
/// — NO `SwapId`, NO `provider_ref` (the deposit address, a §5.4 NEVER-log item)
/// and NO status payload (txids/amounts are NEVER-log) ever enter the span;
/// `outcome` is a fixed category code (never a provider string) and `faults` is a
/// COUNT of transient `provider.status` errors ridden over (a number, never an
/// error payload — so the host sees "the stream stalled because the PROVIDER kept
/// erroring" vs "a slow provider", without a NEVER-log value crossing).
async fn run_swap_status_poll(
    provider: Arc<dyn SwapPort>,
    mut kill_rx: watch::Receiver<SwapKill>,
    id: SwapId,
    provider_ref: String,
    mut sink: impl SwapStatusSink,
) {
    let span = tracing::info_span!(
        target: "zec_wallet_core", "wallet.swap_poll",
        provider = provider.name(),
        outcome = tracing::field::Empty,
        faults = tracing::field::Empty,
    );
    // `poll_loop` runs INSIDE `span.clone()`; recording on `span` here hits the SAME
    // underlying span (a clone shares the span id — it is one span, not two), so the
    // single exit records exactly one `outcome` + `faults` pair.
    let (outcome, faults) = poll_loop(&provider, &mut kill_rx, &id, &provider_ref, &mut sink)
        .instrument(span.clone())
        .await;
    span.record("outcome", outcome);
    span.record("faults", faults);
}

/// The §3.5/§7 poll loop body; returns the §5.4-clean `(outcome code, transient
/// fault count)` for the [`run_swap_status_poll`] span. Split out so the span
/// records exactly one outcome at exactly one exit.
async fn poll_loop(
    provider: &Arc<dyn SwapPort>,
    kill_rx: &mut watch::Receiver<SwapKill>,
    id: &SwapId,
    provider_ref: &str,
    sink: &mut impl SwapStatusSink,
) -> (&'static str, u32) {
    let mut backoff = Duration::from_secs(SWAP_POLL_INITIAL_SECS);
    let mut faults: u32 = 0;
    let mut consecutive_not_found: u32 = 0;
    loop {
        // §3.5: a Hard kill stops in-flight polling BEFORE any further traffic.
        // WindDown/Live fall through and keep polling (status is observational).
        if kill_rx.borrow_and_update().stops_polling() {
            return ("halted", faults);
        }
        match provider.status(id, provider_ref).await {
            Ok(status) => {
                consecutive_not_found = 0;
                let terminal = status.is_terminal();
                if !sink.emit(status) {
                    return ("cancelled", faults); // host dropped the subscription (foreground-only)
                }
                if terminal {
                    return ("terminal", faults);
                }
            }
            // The provider-DEFINITIVE not-found (#367 poll policy; see the
            // `run_swap_status_poll` contract): counted CONSECUTIVELY — at the
            // threshold, synthesize the terminal `Failed(NotFound)` so a
            // re-attached provider-GC'd order reaches an honest terminal instead
            // of spinning forever. Below the threshold it
            // is a fault like any other (a transient post-execute 404 while the
            // provider indexes the order must not false-fail a live swap).
            Err(SwapError::ProviderProtocol {
                reason: ProviderProtocolReason::SwapNotFound,
            }) => {
                faults = faults.saturating_add(1);
                consecutive_not_found = consecutive_not_found.saturating_add(1);
                if consecutive_not_found >= SWAP_NOT_FOUND_TERMINAL_POLLS {
                    if !sink.emit(SwapStatus::Failed {
                        code: SwapFailureCode::NotFound,
                    }) {
                        return ("cancelled", faults);
                    }
                    return ("not_found", faults);
                }
            }
            // Transient provider/transport fault — NEVER end the stream on it (a
            // stall is DATA, not a dead stream); back off and retry. The error is
            // NOT logged: a `SwapError` payload could carry a §5.4 NEVER-log value —
            // only the COUNT is observed (recorded in the exit span, §5.4-clean).
            // Resets the not-found run: only CONSECUTIVE definitive answers
            // terminate (an interleaved transport fault means we can't know).
            Err(_transient) => {
                faults = faults.saturating_add(1);
                consecutive_not_found = 0;
            }
        }
        // Between-poll sleep: a `Hard` kill landing mid-sleep wakes us via the
        // `watch` change, so the loop-top check stops BEFORE another poll fires
        // ("zero swap traffic from this instant").
        let woke = tokio::select! {
            _ = tokio::time::sleep(backoff) => Ok(()),
            res = kill_rx.changed() => res,
        };
        // `changed()` → `Err` means EVERY kill `Sender` dropped, i.e. the
        // `SwapService` (and the wallet) is gone — NOT a transient fault. End the
        // stream rather than spin re-polling on a stale value (the
        // `watch_sync_status` "Sender dropped → wallet teardown → end" contract).
        if woke.is_err() {
            return ("gone", faults);
        }
        backoff = next_poll_backoff(backoff);
    }
}

/// The §7 status-poll backoff step: DOUBLE the interval, capped at
/// `SWAP_POLL_MAX_SECS` (the start is `SWAP_POLL_INITIAL_SECS`). Pure + total
/// (saturating, so an overflow can't panic) so the cadence is boundary-testable
/// without a clock — the `sync_controller::next_backoff` precedent.
fn next_poll_backoff(current: Duration) -> Duration {
    current
        .saturating_mul(2)
        .min(Duration::from_secs(SWAP_POLL_MAX_SECS))
}

/// The §5.4 `wallet.swap` outcome code: `"ok"` on success, else the SwapError's
/// stable, payload-free `code()` (RW-SWAP-NNN). Deliberately a CODE, not an error
/// string — a `"deposit_*"`/`"address"` outcome would trip the §5.4 substring
/// guard, and the code already de-identifies the cause (the durable-store failure
/// is `RW-SWAP-011`, the deposit-leg failure `RW-SWAP-009`).
fn execute_outcome_code(result: &Result<SwapId, SwapError>) -> &'static str {
    match result {
        Ok(_) => "ok",
        Err(e) => e.code(),
    }
}

/// The §5.4-clean outcome code for the `wallet.swap_quote` span — `"ok"` or the stable
/// `RW-SWAP-NNN` error code (the SAME payload-free taxonomy the port boundaries log). The
/// [`execute_outcome_code`] sibling for the quote path.
fn quote_outcome_code(result: &Result<SwapQuote, SwapError>) -> &'static str {
    match result {
        Ok(_) => "ok",
        Err(e) => e.code(),
    }
}

/// The coarse, secret-free DIRECTION code for the `wallet.swap_quote` span — known up front from
/// the request (unlike execute, which resolves it from the deposit presence post-claim). NEVER an
/// asset name or amount (§5.4) — just the on-ramp vs off-ramp category.
fn direction_code(direction: &SwapDirection) -> &'static str {
    match direction {
        SwapDirection::OutOfZec { .. } => "out_of_zec",
        SwapDirection::IntoZec { .. } => "into_zec",
    }
}

/// Current unix time in seconds, saturating (a pre-epoch clock ⇒ `u64::MAX` ⇒ the
/// wall deadline gate fails CLOSED). The within-crate clock form mirrored by every
/// swap wall-deadline read.
fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(u64::MAX)
}

/// The execute-time margin a quote's wall deadline is gated by, by direction
/// (W-swap-4-a-3): a deposit-carrying (OutOfZec) quote must leave
/// `DEPOSIT_EXECUTE_MARGIN_SECS` of mining room plus prove/kick headroom — strictly
/// more than the feed gates' `DEPOSIT_FIRST_FEED_MARGIN_SECS`; IntoZec (the user's
/// external deposit) `DEADLINE_SAFETY_MARGIN_SECS`. ONE source for the pre-claim
/// wall check and the post-claim gate in `execute`, which must agree: otherwise a
/// live quote could be skipped as dead, or a dead one compared as live.
fn execute_margin(has_deposit: bool) -> u64 {
    if has_deposit {
        DEPOSIT_EXECUTE_MARGIN_SECS
    } else {
        DEADLINE_SAFETY_MARGIN_SECS
    }
}

/// Rebuild the in-memory [`DepositPlan`] from a durable [`IssuedDeposit`] on the
/// CROSS-RESTART execute path (W-swap-3-c-3-ii): the L-1 in-memory `Address` is gone
/// after a restart, so the deposit-address STRING is re-parsed against the wallet's
/// OWN network and re-blessed transparent — exactly the `validate_quote` checks that
/// blessed it at persist time, re-run as defense-in-depth against a tampered sealed
/// DB. A parse / kind / wrong-network failure is a corrupt durable record ⇒
/// fail-closed `SwapStateUnavailable` (the payload-free durable-state door): the
/// execute aborts BEFORE `provider.execute`, so no deposit is ever sent to a garbage
/// address; the user re-quotes.
fn reconstruct_deposit_plan(d: IssuedDeposit, network: Network) -> Result<DepositPlan, SwapError> {
    match Address::parse(&d.address, network) {
        Ok(address) if address.kind() == AddressKind::Transparent => Ok(DepositPlan {
            address,
            amount: d.amount,
        }),
        _ => Err(SwapError::SwapStateUnavailable),
    }
}

// ── Request validation (user side) ──────────────────────────────────────────

fn validate_request(req: &QuoteRequest, network: Network) -> Result<(), SwapError> {
    if req.slippage_tolerance_bps > SLIPPAGE_MAX_BPS {
        return Err(SwapError::SlippageToleranceTooHigh {
            requested_bps: req.slippage_tolerance_bps,
            max_bps: SLIPPAGE_MAX_BPS,
        });
    }
    // S7 W1: out-of-ZEC fixing the foreign amount would sign a deposit (`zec_side`)
    // no number the user typed bounds (spec §2.6). No host sends it (the UI, the
    // example and Relim all send `In`), so it is refused, not bounded.
    if let (SwapDirection::OutOfZec { .. }, ExactSide::Out(SwapAmount::Foreign(_))) =
        (&req.direction, &req.exact)
    {
        return Err(SwapError::RequestInvalid {
            reason: "an out-of-ZEC swap takes the ZEC amount you pay",
        });
    }
    // direction ↔ exact-side coherence: the user's number must name the
    // asset family the direction puts on that side
    let coherent = matches!(
        (&req.direction, &req.exact),
        (
            SwapDirection::OutOfZec { .. },
            ExactSide::In(SwapAmount::Zec(_))
        ) | (
            SwapDirection::IntoZec { .. },
            ExactSide::In(SwapAmount::Foreign(_))
        ) | (
            SwapDirection::IntoZec { .. },
            ExactSide::Out(SwapAmount::Zec(_))
        )
    );
    if !coherent {
        return Err(SwapError::RequestInvalid {
            reason: "exact side names the wrong asset family for this direction",
        });
    }
    if let ExactSide::In(SwapAmount::Foreign(s)) | ExactSide::Out(SwapAmount::Foreign(s)) =
        &req.exact
    {
        // the USER's own number — a caller error, not a provider violation
        parse_decimal(s).ok_or(SwapError::RequestInvalid {
            reason: "exact amount is not a valid decimal",
        })?;
    }
    match &req.direction {
        SwapDirection::OutOfZec { .. } => {
            let dest = req
                .destination
                .as_deref()
                .ok_or(SwapError::DestinationInvalid {
                    reason: DestinationInvalidReason::Missing,
                })?;
            if dest.len() > PROVIDER_STR_MAX_BYTES {
                return Err(SwapError::DestinationInvalid {
                    reason: DestinationInvalidReason::Oversized,
                });
            }
            // a ZEC→ZEC "swap" is a fee-burning provider round-trip (§1.7):
            // ANY parseable Zcash address (either network) is refused
            if zcash_address::ZcashAddress::try_from_encoded(dest).is_ok() {
                return Err(SwapError::DestinationInvalid {
                    reason: DestinationInvalidReason::ZcashAddressNotAllowed,
                });
            }
        }
        SwapDirection::IntoZec { .. } => {
            if req.destination.is_some() {
                // we receive to our OWN fresh address — a caller-supplied
                // destination here is a confused (or hostile) integration
                return Err(SwapError::DestinationInvalid {
                    reason: DestinationInvalidReason::NotAllowedForDirection,
                });
            }
            // IntoZec REQUIRES the user's source-chain refund target (where their coin
            // returns if the swap fails, §4.4 HARD-G / §3.3b D6) — enforced HERE at the
            // service door so a malformed request fails BEFORE any mint or network work,
            // not deep inside the adapter's `build_into_zec_request`.
            let refund = req
                .refund_address
                .as_deref()
                .ok_or(SwapError::RequestInvalid {
                    reason: "IntoZec requires a source-chain refund address",
                })?;
            if refund.len() > PROVIDER_STR_MAX_BYTES {
                // DestinationInvalid reused on purpose for the refund target
                // (both are "an address you gave us is unusable"); a separate
                // RefundInvalid variant buys nothing at this surface
                return Err(SwapError::DestinationInvalid {
                    reason: DestinationInvalidReason::Oversized,
                });
            }
        }
    }
    let _ = network; // request-side checks are network-independent today
    Ok(())
}

// ── Quote validation (provider side — every byte hostile, §4.6) ─────────────

/// Returns the blessed transparent deposit `Address` for an OutOfZec quote
/// (`None` for IntoZec) so the caller threads the EXACT validated value into the
/// deposit send (L-1), never a second parse.
fn validate_quote(
    quote: &SwapQuote,
    sent: &QuoteRequest,
    network: Network,
) -> Result<Option<Address>, SwapError> {
    // m1: named bounds on EVERY provider string, before anything reads them
    // (incl. the inbound FFI path `TryFrom<api::SwapQuote>` → execute, which
    // bypasses the adapter funnel — a Dart-supplied quote is bounded HERE).
    let oversized = quote.id.as_str().len() > PROVIDER_STR_MAX_BYTES
        || quote.deposit_address.len() > PROVIDER_STR_MAX_BYTES
        || quote.amount_in.len() > PROVIDER_STR_MAX_BYTES
        || quote.min_amount_out.len() > PROVIDER_STR_MAX_BYTES
        || quote
            .deposit_memo
            .as_deref()
            .is_some_and(|m| m.len() > PROVIDER_STR_MAX_BYTES)
        || quote
            .refund_to
            .as_deref()
            .is_some_and(|r| r.len() > PROVIDER_STR_MAX_BYTES);
    if oversized {
        return Err(SwapError::ProviderProtocol {
            reason: ProviderProtocolReason::OversizedField,
        });
    }
    // #368: refuse an id inside the wallet-RESERVED synthetic-key namespace (the
    // ADR-0527 backfill rows) — a hostile id there could pre-claim/extend a synthetic
    // watch row and silently suppress refund detection.
    // No real provider id shape (base58/bech32 handles) contains `:`.
    if quote
        .id
        .as_str()
        .starts_with(crate::swap_destination_store::BACKFILL_ID_PREFIX)
    {
        return Err(SwapError::ProviderProtocol {
            reason: ProviderProtocolReason::ReservedId,
        });
    }
    let amount_in = parse_decimal(&quote.amount_in).ok_or(SwapError::ProviderProtocol {
        reason: ProviderProtocolReason::MalformedAmount,
    })?;
    let min_out = parse_decimal(&quote.min_amount_out).ok_or(SwapError::ProviderProtocol {
        reason: ProviderProtocolReason::MalformedAmount,
    })?;

    // the echoed refundTo must be EXACTLY what we sent (mismatch ⇒ abort
    // before any deposit path exists — §2.6)
    if quote.refund_to != sent.refund_address {
        return Err(SwapError::ProviderProtocol {
            reason: ProviderProtocolReason::RefundAddressMismatch,
        });
    }

    // §4.4: an OutOfZec deposit address must be a parseable TRANSPARENT
    // address on OUR network — checked long before any signing path. RETURN the
    // parsed `Address` so execute sends to the EXACT value validated here (L-1).
    let blessed = if matches!(sent.direction, SwapDirection::OutOfZec { .. }) {
        match Address::parse(&quote.deposit_address, network) {
            Ok(addr) if addr.kind() == AddressKind::Transparent => Some(addr),
            _ => {
                return Err(SwapError::ProviderProtocol {
                    reason: ProviderProtocolReason::DepositAddressInvalid,
                });
            }
        }
    } else {
        None
    };

    // M1 — THE user-anchored sanity ceiling: the side the user fixed must lie
    // within [request ± tolerance]; an out-of-ZEC deposit must BE it (S7 W1).
    // A deposit is never signed anchored to provider numbers alone. (The
    // unanchored side is fixed by quote acceptance: execute takes only these.)
    let tol = sent.slippage_tolerance_bps;
    match &sent.exact {
        // `In(Zec)` is out-of-ZEC only (the request's coherence door): the deposit is
        // `zec_side`, and EXACT_INPUT `amount = z` echoes `z` — never more or less.
        ExactSide::In(SwapAmount::Zec(z)) => {
            if quote.zec_side != *z {
                return Err(SwapError::QuoteOutOfBounds {
                    side: QuoteBoundSide::Zec,
                });
            }
        }
        // into-ZEC: the wallet signs nothing, so the two-sided bound stands.
        ExactSide::Out(SwapAmount::Zec(z)) => {
            if !zat_within_bps(quote.zec_side, *z, tol) {
                return Err(SwapError::QuoteOutOfBounds {
                    side: QuoteBoundSide::Zec,
                });
            }
        }
        ExactSide::In(SwapAmount::Foreign(f)) => {
            let anchor = parse_decimal(f).ok_or(SwapError::ProviderProtocol {
                reason: ProviderProtocolReason::MalformedAmount,
            })?;
            if !decimal_within_bps(amount_in, anchor, tol) {
                return Err(SwapError::QuoteOutOfBounds {
                    side: QuoteBoundSide::Foreign,
                });
            }
        }
        ExactSide::Out(SwapAmount::Foreign(f)) => {
            let anchor = parse_decimal(f).ok_or(SwapError::ProviderProtocol {
                reason: ProviderProtocolReason::MalformedAmount,
            })?;
            if !decimal_within_bps(min_out, anchor, tol) {
                return Err(SwapError::QuoteOutOfBounds {
                    side: QuoteBoundSide::Foreign,
                });
            }
        }
    }
    Ok(blessed)
}

// ── Exact decimal math (no floats on a funds path) ──────────────────────────

/// A non-negative decimal as (mantissa, scale): `"12.34"` → `(1234, 2)`.
type Decimal = (i128, u32);

/// Strict decimal parse: digits with at most one dot; no sign, no exponent,
/// no empties; ≤ `SWAP_DECIMAL_MAX_DIGITS` significant digits (keeps the bps
/// math overflow-free). `None` = malformed.
fn parse_decimal(s: &str) -> Option<Decimal> {
    let (int_part, frac_part) = match s.split_once('.') {
        Some((i, f)) => (i, f),
        None => (s, ""),
    };
    if int_part.is_empty() && frac_part.is_empty() {
        return None;
    }
    let digits = int_part.len() + frac_part.len();
    if digits > SWAP_DECIMAL_MAX_DIGITS
        || !int_part.bytes().all(|b| b.is_ascii_digit())
        || !frac_part.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let mut mantissa: i128 = 0;
    for b in int_part.bytes().chain(frac_part.bytes()) {
        mantissa = mantissa * 10 + i128::from(b - b'0'); // bounded by digit cap
    }
    Some((mantissa, frac_part.len() as u32))
}

/// `|value − anchor| × 10_000 ≤ anchor × bps`, exact, at a common scale.
///
/// EVERY arithmetic step is checked; overflow anywhere ⇒ `false` = the quote
/// is rejected (fail closed — a quote we cannot bound-check is never trusted).
/// W2 review MAJOR closed here: the digit cap bounds the SOURCE mantissa, but
/// rescaling to a common scale can lift a 27-digit value near i128::MAX,
/// where an unchecked `× 10_000` wraps — in release that wrap read an
/// absurdly out-of-bounds quote as IN-bounds (the M1 drain vector itself).
fn decimal_within_bps(value: Decimal, anchor: Decimal, bps: u16) -> bool {
    let scale = value.1.max(anchor.1);
    let Some(v) = rescale(value, scale) else {
        return false;
    };
    let Some(a) = rescale(anchor, scale) else {
        return false;
    };
    let diff = (v - a).abs(); // both non-negative < i128::MAX — cannot wrap
    let Some(lhs) = diff.checked_mul(10_000) else {
        return false;
    };
    let Some(rhs) = a.checked_mul(i128::from(bps)) else {
        return false;
    };
    lhs <= rhs
}

fn rescale((mantissa, scale): Decimal, to: u32) -> Option<i128> {
    let mut m = mantissa;
    for _ in scale..to {
        m = m.checked_mul(10)?;
    }
    Some(m)
}

fn zat_within_bps(value: Zatoshis, anchor: Zatoshis, bps: u16) -> bool {
    let v = i128::from(value.zat());
    let a = i128::from(anchor.zat());
    // Unchecked is safe here (unlike `decimal_within_bps`, whose rescale can lift a
    // value near i128::MAX): both args are bounded by `MAX_MONEY_ZAT` (~2.1e15), so
    // the worst-case LHS is ~2.1e15 × 10_000 = 2.1e19 and RHS ~2.1e15 × 10_000 =
    // 2.1e19 — both far below i128::MAX (~1.7e38). Cannot wrap for any `Zatoshis`.
    (v - a).abs() * 10_000 <= a * i128::from(bps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::MAX_ISSUED_QUOTES;
    use crate::swap::testing::MockSwapProvider;
    use crate::swap::types::{AssetId, SwapPrivacyDisclosure};

    /// An in-memory [`SwapRecordSink`] double (W-swap-5): RECORDS every
    /// `record_started` call so a test can assert the exact home row the execute
    /// leg committed (id, coarse direction, both deadlines), and with `fail` set
    /// returns `SwapStateUnavailable` — the record-first abort path (the deposit
    /// must then NEVER be attempted).
    #[derive(Default)]
    struct MemSwapRecords {
        started: Mutex<Vec<StartedSwap>>,
        fail: bool,
    }
    impl MemSwapRecords {
        fn rows(&self) -> Vec<StartedSwap> {
            self.started.lock().expect("records poisoned").clone()
        }
    }
    #[async_trait]
    impl SwapRecordSink for MemSwapRecords {
        async fn record_started(&self, record: StartedSwap) -> Result<(), SwapError> {
            if self.fail {
                return Err(SwapError::SwapStateUnavailable);
            }
            self.started.lock().expect("records poisoned").push(record);
            Ok(())
        }
        async fn provider_ref(&self, id: &SwapId) -> Result<Option<String>, SwapError> {
            if self.fail {
                return Err(SwapError::SwapStateUnavailable);
            }
            Ok(self
                .started
                .lock()
                .expect("records poisoned")
                .iter()
                .find(|r| &r.id == id)
                .map(|r| r.provider_ref.clone()))
        }
    }

    /// An in-memory [`IssuedQuoteStore`] double — the durable store's behavioural
    /// contract (FIRST-WINS persist, the `MAX_ISSUED_QUOTES` cap, atomic single-shot
    /// `take`) without SQLCipher. The real durable store + its cross-restart teeth are
    /// pinned on a sealed wallet in `wallet.rs` (W-swap-3-c-2 integration tests); here it
    /// lets the service-orchestration tests exercise the persist/consume wiring. `fail_*`
    /// model a tearing-down / errored store (⇒ `SwapStateUnavailable`).
    #[derive(Default)]
    struct MemIssuedQuoteStore {
        rows: Mutex<HashMap<SwapId, IssuedQuoteRecord>>,
        fail_persist: bool,
        fail_take: bool,
    }
    impl MemIssuedQuoteStore {
        fn count(&self) -> usize {
            self.rows.lock().expect("rows poisoned").len()
        }
        fn contains(&self, id: &SwapId) -> bool {
            self.rows.lock().expect("rows poisoned").contains_key(id)
        }
    }
    #[async_trait]
    impl IssuedQuoteStore for MemIssuedQuoteStore {
        async fn persist(
            &self,
            record: IssuedQuoteRecord,
        ) -> Result<IssuedQuotePersist, SwapError> {
            if self.fail_persist {
                return Err(SwapError::SwapStateUnavailable);
            }
            let mut rows = self.rows.lock().expect("rows poisoned");
            if rows.contains_key(&record.id) {
                return Ok(IssuedQuotePersist::Persisted); // first-wins: never overwrite
            }
            if rows.len() >= MAX_ISSUED_QUOTES {
                return Ok(IssuedQuotePersist::AtCapacity);
            }
            rows.insert(record.id.clone(), record);
            Ok(IssuedQuotePersist::Persisted)
        }
        async fn peek(&self, id: &SwapId) -> Result<Option<IssuedQuoteRecord>, SwapError> {
            if self.fail_take {
                return Err(SwapError::SwapStateUnavailable);
            }
            Ok(self.rows.lock().expect("rows poisoned").get(id).cloned())
        }
        async fn take(&self, id: &SwapId) -> Result<Option<IssuedQuoteRecord>, SwapError> {
            if self.fail_take {
                return Err(SwapError::SwapStateUnavailable);
            }
            Ok(self.rows.lock().expect("rows poisoned").remove(id))
        }
    }

    struct StubRefunds;
    #[async_trait]
    impl RefundAddressSource for StubRefunds {
        async fn fresh_refund_address(&self) -> Result<IssuedRefund, SwapError> {
            // a checksum-valid mainnet t-addr (test-constructed; §4.6) at a fixed
            // single-use index (#368 — the port now returns the durable handle)
            use zcash_address::ToAddress;
            use zcash_protocol::consensus::NetworkType;
            Ok(IssuedRefund {
                address: zcash_address::ZcashAddress::from_transparent_p2pkh(
                    NetworkType::Main,
                    [0x11; 20],
                )
                .encode(),
                index: 1,
            })
        }
    }

    /// A `DestinationAddressSource` double (IZ-1): hands out DISTINCT `u1dest{index}`
    /// destinations at monotonically increasing single-use indices from index 1 (never the
    /// index-0 receive address — the real counter's `REFUND_INDEX_FLOOR` posture), RECORDING
    /// each mint so a test can assert the service threaded it into the provider `recipient` AND
    /// the durable record. With `fail` set it returns the typed, payload-free
    /// `DestinationAddressUnavailable` (the mint-failure leg). The distinctness-per-call mirrors
    /// the real shared counter (HARD-H); the address strings are opaque to the service.
    #[derive(Default)]
    struct CountingDestinations {
        minted: Mutex<Vec<IssuedDestination>>,
        fail: bool,
    }
    #[async_trait]
    impl DestinationAddressSource for CountingDestinations {
        async fn fresh_destination(&self) -> Result<IssuedDestination, SwapError> {
            if self.fail {
                return Err(SwapError::DestinationAddressUnavailable);
            }
            let mut minted = self.minted.lock().expect("minted poisoned");
            // distinct, monotonic, floored above the index-0 receive address (the
            // `refund_index::REFUND_INDEX_FLOOR` rule the real adapter enforces).
            let index = 1 + minted.len() as u32;
            let dest = IssuedDestination {
                address: format!("u1dest{index}"),
                index,
            };
            minted.push(dest.clone());
            Ok(dest)
        }
    }

    /// A `DepositSender` double that RECORDS every `send_deposit_now` call (so a test
    /// can assert the exact address/amount/deadline threaded — the L-1 proof) and,
    /// when `fail` is set, returns `DepositSendFailed` (the error-path leg). Records
    /// the attempt BEFORE failing, so "attempted exactly once AND errored" is testable.
    /// The FR-17 binding is captured in its own parallel spy (one entry per call, in
    /// order) so the quote→issued-row→deposit thread is assertable.
    #[derive(Default)]
    struct RecordingDeposit {
        deposits: Mutex<Vec<(Address, Zatoshis, u64)>>,
        /// FR-17 spy: the binding each `send_deposit_now` presented, call-ordered.
        bindings: Mutex<Vec<Option<SpendBinding>>>,
        fail: bool,
    }
    #[async_trait]
    impl DepositSender for RecordingDeposit {
        async fn send_deposit_now(
            &self,
            deposit: Address,
            amount: Zatoshis,
            deadline_unix: u64,
            binding: Option<SpendBinding>,
        ) -> Result<(), SwapError> {
            self.deposits
                .lock()
                .expect("deposits poisoned")
                .push((deposit, amount, deadline_unix));
            self.bindings
                .lock()
                .expect("bindings poisoned")
                .push(binding);
            if self.fail {
                Err(SwapError::DepositSendFailed)
            } else {
                Ok(())
            }
        }
    }

    fn usdc() -> AssetId {
        AssetId {
            chain: "near".into(),
            symbol: "USDC".into(),
        }
    }

    fn out_of_zec_request(zat: i64) -> QuoteRequest {
        QuoteRequest {
            direction: SwapDirection::OutOfZec { to: usdc() },
            exact: ExactSide::In(SwapAmount::Zec(Zatoshis::new(zat).expect("valid"))),
            slippage_tolerance_bps: 200,
            destination: Some("0xabcdef0123456789".into()),
            refund_address: None,
        }
    }

    fn disclosure() -> SwapPrivacyDisclosure {
        SwapPrivacyDisclosure {
            ends_shielded: false,
            deshields: true,
            provider_legs_transparent: true,
            provider_sees: vec![],
        }
    }

    /// An honest provider echo for `req`: in-bounds zec_side, echoed
    /// refundTo, valid transparent deposit address, sane decimals.
    fn honest_quote(req: &QuoteRequest, zec_side: i64, expires_in: u64) -> SwapQuote {
        use zcash_address::ToAddress;
        use zcash_protocol::consensus::NetworkType;
        let deposit =
            zcash_address::ZcashAddress::from_transparent_p2pkh(NetworkType::Main, [0x22; 20])
                .encode();
        let wall_now = now_unix(); // the one crate clock form (also a wall-deadline read)
        SwapQuote {
            id: SwapId::new("q-1"),
            deposit_address: deposit,
            deposit_memo: None,
            expires_at: wall_now + expires_in,
            amount_in: "1.0".into(),
            min_amount_out: "41.5".into(),
            zec_side: Zatoshis::new(zec_side).expect("valid"),
            refund_to: req.refund_address.clone(),
            disclosure: disclosure(),
            binding: None,
        }
    }

    fn service(mock: Arc<MockSwapProvider>) -> SwapService {
        SwapService::new(
            mock,
            Arc::new(StubRefunds),
            Arc::new(CountingDestinations::default()),
            Arc::new(RecordingDeposit::default()),
            Arc::new(MemIssuedQuoteStore::default()),
            Arc::new(MemSwapRecords::default()),
            Network::Main,
        )
    }

    /// Like [`service`] but returns the deposit double so the deposit-send leg
    /// (the inc-2d-swap-b hook) can be inspected.
    fn service_recording(mock: Arc<MockSwapProvider>) -> (SwapService, Arc<RecordingDeposit>) {
        let deposits = Arc::new(RecordingDeposit::default());
        let svc = SwapService::new(
            mock,
            Arc::new(StubRefunds),
            Arc::new(CountingDestinations::default()),
            deposits.clone(),
            Arc::new(MemIssuedQuoteStore::default()),
            Arc::new(MemSwapRecords::default()),
            Network::Main,
        );
        (svc, deposits)
    }

    /// Like [`service_recording`] but the deposit leg always FAILS (the error
    /// path). Returns the recording double so the attempt count is testable.
    fn service_failing_deposit(
        mock: Arc<MockSwapProvider>,
    ) -> (SwapService, Arc<RecordingDeposit>) {
        let deposits = Arc::new(RecordingDeposit {
            fail: true,
            ..Default::default()
        });
        let svc = SwapService::new(
            mock,
            Arc::new(StubRefunds),
            Arc::new(CountingDestinations::default()),
            deposits.clone(),
            Arc::new(MemIssuedQuoteStore::default()),
            Arc::new(MemSwapRecords::default()),
            Network::Main,
        );
        (svc, deposits)
    }

    /// Like [`service_recording`] but also threads a caller-provided durable
    /// [`IssuedQuoteStore`] double, so the W-swap-3-c-2 persist/consume wiring is
    /// inspectable (`store.count()`/`contains`) and a FAILING store (the
    /// `SwapStateUnavailable` path) can be injected.
    fn service_with_store(
        mock: Arc<MockSwapProvider>,
        store: Arc<MemIssuedQuoteStore>,
    ) -> (SwapService, Arc<RecordingDeposit>) {
        let deposits = Arc::new(RecordingDeposit::default());
        let svc = SwapService::new(
            mock,
            Arc::new(StubRefunds),
            Arc::new(CountingDestinations::default()),
            deposits.clone(),
            store,
            Arc::new(MemSwapRecords::default()),
            Network::Main,
        );
        (svc, deposits)
    }

    /// Like [`service_recording`] but also threads a caller-provided
    /// [`SwapRecordSink`] double, so the W-swap-5 home-row wiring is inspectable
    /// (`records.rows()`) and a FAILING sink (the record-first abort path) can be
    /// injected.
    fn service_with_records(
        mock: Arc<MockSwapProvider>,
        records: Arc<MemSwapRecords>,
    ) -> (SwapService, Arc<RecordingDeposit>) {
        let deposits = Arc::new(RecordingDeposit::default());
        let svc = SwapService::new(
            mock,
            Arc::new(StubRefunds),
            Arc::new(CountingDestinations::default()),
            deposits.clone(),
            Arc::new(MemIssuedQuoteStore::default()),
            records,
            Network::Main,
        );
        (svc, deposits)
    }

    /// An IntoZec request (we RECEIVE ZEC to our own address ⇒ no wallet-side
    /// deposit). The user-supplied refund target passes through (§2.6).
    fn into_zec_request() -> QuoteRequest {
        QuoteRequest {
            direction: SwapDirection::IntoZec { from: usdc() },
            exact: ExactSide::In(SwapAmount::Foreign("1.0".into())),
            slippage_tolerance_bps: 200,
            destination: None,
            refund_address: Some("user-supplied-refund".into()),
        }
    }

    /// Mock wired so the quote it returns echoes the request the SERVICE
    /// sent (incl. the fresh refundTo it populates).
    fn echoing_mock(zec_side: i64, expires_in: u64) -> Arc<MockSwapProvider> {
        Arc::new(MockSwapProvider::with_quote_fn(move |req| {
            Ok(honest_quote(req, zec_side, expires_in))
        }))
    }

    #[tokio::test]
    async fn into_zec_quote_mints_records_and_binds_the_destination() {
        // IZ-1 / §3.3b D1: an IntoZec quote mints a fresh destination through the port, BINDS it as
        // the provider `recipient`, and RECORDS it in the durable issued-quote record (so the §3.3b
        // L2 scoped poll reconstructs the detection set). Mutually exclusive with a deposit by
        // direction — an IntoZec record carries a destination, no deposit.
        let dests = Arc::new(CountingDestinations::default());
        let store = Arc::new(MemIssuedQuoteStore::default());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen_w = Arc::clone(&seen);
        let provider = Arc::new(MockSwapProvider::with_quote_fn(move |req| {
            seen_w.lock().expect("seen poisoned").push(req.clone());
            Ok(honest_quote(req, 100_000, 3_600))
        }));
        let svc = SwapService::new(
            provider,
            Arc::new(StubRefunds),
            dests.clone(),
            Arc::new(RecordingDeposit::default()),
            store.clone(),
            Arc::new(MemSwapRecords::default()),
            Network::Main,
        );
        let q = svc.quote(into_zec_request()).await.expect("intozec quote");
        let minted = dests.minted.lock().expect("minted").clone();
        assert_eq!(
            minted.len(),
            1,
            "exactly one destination minted per IntoZec quote"
        );
        assert_eq!(
            seen.lock().expect("seen")[0].destination.as_deref(),
            Some(minted[0].address.as_str()),
            "the minted destination is bound as the provider recipient",
        );
        let rec = store
            .rows
            .lock()
            .expect("rows")
            .get(&q.id)
            .cloned()
            .expect("the issued quote was recorded");
        assert_eq!(
            rec.destination,
            Some(minted[0].clone()),
            "the destination (address + index) is recorded durably",
        );
        assert!(
            rec.deposit.is_none(),
            "an IntoZec record carries a destination, not a deposit (direction exclusivity)",
        );
    }

    #[tokio::test]
    async fn into_zec_quote_destination_mint_failure_is_typed_and_persists_nothing() {
        // §3.3b D1 fail-closed: if the wallet cannot mint a destination, the quote returns the
        // payload-free `DestinationAddressUnavailable` BEFORE the provider is called — nothing is
        // bound to a destination we cannot track, nothing is persisted (port isolation; §5.4).
        let dests = Arc::new(CountingDestinations {
            fail: true,
            ..Default::default()
        });
        let store = Arc::new(MemIssuedQuoteStore::default());
        let svc = SwapService::new(
            echoing_mock(100_000, 3_600),
            Arc::new(StubRefunds),
            dests,
            Arc::new(RecordingDeposit::default()),
            store.clone(),
            Arc::new(MemSwapRecords::default()),
            Network::Main,
        );
        assert!(matches!(
            svc.quote(into_zec_request()).await,
            Err(SwapError::DestinationAddressUnavailable)
        ));
        assert_eq!(
            store.count(),
            0,
            "a failed destination mint persists nothing (fail-closed before the provider call)",
        );
    }

    #[tokio::test]
    async fn into_zec_quote_without_a_refund_address_is_rejected() {
        // §3.3b D6: IntoZec requires the user's source-chain refund target — a missing one is a
        // typed RequestInvalid at the SERVICE door (before any mint/network), not a deep adapter
        // error.
        let svc = service(echoing_mock(100_000, 3_600));
        let mut req = into_zec_request();
        req.refund_address = None;
        assert!(matches!(
            svc.quote(req).await,
            Err(SwapError::RequestInvalid { .. })
        ));
    }

    #[tokio::test]
    async fn quote_zec_side_outside_request_tolerance_rejected_before_signing() {
        // §8 M1 named test: user asks 100_000 zat in; provider quotes 2× —
        // typed reject, and NOTHING downstream ever runs (the execute
        // counter on the mock stays zero; the signing path, when it lands,
        // sits behind execute).
        let mock = echoing_mock(200_000, 3_600);
        let svc = service(mock.clone());
        let err = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect_err("must reject");
        assert!(
            matches!(
                err,
                SwapError::QuoteOutOfBounds {
                    side: QuoteBoundSide::Zec
                }
            ),
            "got {}",
            err.code()
        );
        assert!(
            mock.executed.lock().expect("mock").is_empty(),
            "nothing ran past the gate"
        );

        // boundary honesty (S7 W1): the deposit side must BE the user's number —
        // +2% (inside the tolerance) no longer passes
        let mock = echoing_mock(102_000, 3_600);
        assert!(matches!(
            service(mock).quote(out_of_zec_request(100_000)).await,
            Err(SwapError::QuoteOutOfBounds { .. })
        ));
    }

    #[tokio::test]
    async fn slippage_beyond_tolerance_rejected() {
        // §8 named test: above the hard ceiling ⇒ typed reject BEFORE any
        // provider call (the mock never sees a request).
        let mock = Arc::new(MockSwapProvider::default());
        let svc = service(mock.clone());
        let mut req = out_of_zec_request(100_000);
        req.slippage_tolerance_bps = SLIPPAGE_MAX_BPS + 1;
        let err = svc.quote(req).await.expect_err("must reject");
        assert!(matches!(
            err,
            SwapError::SlippageToleranceTooHigh { requested_bps, max_bps }
                if requested_bps == SLIPPAGE_MAX_BPS + 1 && max_bps == SLIPPAGE_MAX_BPS
        ));
        assert_eq!(
            *mock.quote_calls.lock().expect("mock"),
            0,
            "provider never consulted"
        );
    }

    #[tokio::test]
    async fn swap_destination_rejects_zcash_address() {
        // §8 named test: a ZEC→ZEC "swap" is a fee-burning provider
        // round-trip — ANY parseable Zcash address (any kind, any network)
        // is refused as an OutOfZec destination.
        use zcash_address::ToAddress;
        use zcash_protocol::consensus::NetworkType;
        let unified = {
            use zcash_address::unified::{Address as Ua, Encoding, Receiver};
            Ua::try_from_items(vec![Receiver::Orchard([9; 43]), Receiver::Sapling([9; 43])])
                .expect("valid receiver set")
                .encode(&NetworkType::Main)
        };
        let zec_destinations = [
            zcash_address::ZcashAddress::from_transparent_p2pkh(NetworkType::Main, [9; 20])
                .encode(),
            zcash_address::ZcashAddress::from_sapling(NetworkType::Main, [9; 43]).encode(),
            zcash_address::ZcashAddress::from_sapling(NetworkType::Test, [9; 43]).encode(),
            unified,
        ];
        for dest in zec_destinations {
            let svc = service(Arc::new(MockSwapProvider::default()));
            let mut req = out_of_zec_request(100_000);
            req.destination = Some(dest);
            assert!(matches!(
                svc.quote(req).await,
                Err(SwapError::DestinationInvalid {
                    reason: DestinationInvalidReason::ZcashAddressNotAllowed
                })
            ));
        }
        // and IntoZec must not carry a destination at all
        let svc = service(Arc::new(MockSwapProvider::default()));
        let req = QuoteRequest {
            direction: SwapDirection::IntoZec { from: usdc() },
            exact: ExactSide::In(SwapAmount::Foreign("41.5".into())),
            slippage_tolerance_bps: 200,
            destination: Some("0xabc".into()),
            refund_address: None,
        };
        assert!(matches!(
            svc.quote(req).await,
            Err(SwapError::DestinationInvalid {
                reason: DestinationInvalidReason::NotAllowedForDirection
            })
        ));
    }

    #[tokio::test]
    async fn in_bounds_quote_flows_to_execute_and_unknown_quotes_do_not() {
        let mock = echoing_mock(100_000, 3_600);
        let svc = service(mock.clone());
        let quote = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("in-bounds quote");
        // OutOfZec refundTo was populated by the SERVICE (fresh t-addr),
        // echoed by the provider, and verified equal
        assert!(
            quote.refund_to.is_some(),
            "service must populate OutOfZec refundTo"
        );
        let id = svc.execute(&quote).await.expect("execute issued quote");
        assert_eq!(mock.executed.lock().expect("mock").as_slice(), &[id]);

        // a hand-built quote (never issued by this service) is refused —
        // bounds can't be bypassed by constructing the DTO. Typed `QuoteExpired`
        // since #367 (every take-miss reads "no longer valid — get a new quote").
        let foreign = SwapQuote {
            id: SwapId::new("forged"),
            ..quote
        };
        assert!(matches!(
            svc.execute(&foreign).await,
            Err(SwapError::QuoteExpired)
        ));
    }

    #[tokio::test]
    async fn expired_quote_rejected_at_execute_monotonic() {
        // expires_at within the safety margin ⇒ act-deadline already past —
        // deterministic, no sleeps (monotonic posture, §2.6)
        let mock = echoing_mock(100_000, DEADLINE_SAFETY_MARGIN_SECS / 2);
        let svc = service(mock);
        let quote = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("quote issues");
        assert!(matches!(
            svc.execute(&quote).await,
            Err(SwapError::QuoteExpired)
        ));
    }

    #[tokio::test]
    async fn suspended_device_cannot_revive_wall_dead_quote() {
        // unstable-env review fold (MAJOR): Instant PAUSES during device
        // suspend on every shipped platform, so a phone asleep between
        // quote and execute under-counts elapsed time — the monotonic half
        // alone would pass a provider-window-dead quote. Simulate "slept
        // past the window" by rewinding the DURABLE row's wall deadline (S8:
        // the record is the wall authority on both paths; the registry keeps
        // only the monotonic half); the dual gate must reject on the wall
        // clock alone.
        let mock = echoing_mock(100_000, 3_600);
        let store = Arc::new(MemIssuedQuoteStore::default());
        let (svc, _deposits) = service_with_store(mock, store.clone());
        let quote = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("quote issues");
        {
            let mut rows = store.rows.lock().expect("rows");
            rows.get_mut(&quote.id).expect("present").expires_at_wall = 0;
        }
        assert!(matches!(
            svc.execute(&quote).await,
            Err(SwapError::QuoteExpired)
        ));
    }

    // ── inc-2d-swap-b: the execute → DepositSender hook (§4.4) ────────────────

    #[tokio::test]
    async fn execute_outofzec_drives_the_deposit_send_with_the_blessed_address() {
        // §8 (inc-2d-swap-b): a live OutOfZec quote, executed, drives the wallet
        // deposit leg with the EXACT value `validate_quote` blessed — the
        // transparent deposit address (L-1: byte-equal to a fresh parse of the
        // provider string), the ZEC INPUT (`zec_side`), and the CLAMPED wall
        // deadline (W-swap-4-a-3: the stored wall deadline is ceiling-bounded to
        // the 15-min OutOfZec window we request, so THAT — not the mock's 1 h
        // echo — rides into the durable intent, §4.4).
        let mock = echoing_mock(100_000, 3_600);
        let (svc, deposits) = service_recording(mock.clone());
        let before = now_unix();
        let quote = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("in-bounds quote");
        let after = now_unix();
        let id = svc.execute(&quote).await.expect("execute issued quote");
        // provider intent registered exactly once
        assert_eq!(mock.executed.lock().expect("mock").as_slice(), &[id]);
        // and the deposit was queued with the blessed values
        let recorded = deposits.deposits.lock().expect("deposits");
        assert_eq!(recorded.len(), 1, "exactly one deposit queued");
        let (addr, amount, deadline) = &recorded[0];
        assert_eq!(
            addr.kind(),
            AddressKind::Transparent,
            "deposit recipient is transparent (§4.4)"
        );
        assert_eq!(
            addr,
            &Address::parse(&quote.deposit_address, Network::Main).expect("re-parse"),
            "L-1: the deposit went to the EXACT blessed address"
        );
        assert_eq!(
            *amount, quote.zec_side,
            "deposit amount = the ZEC input side"
        );
        let win = SWAP_DEPOSIT_DEADLINE_OUT_OF_ZEC_SECS;
        assert!(
            *deadline >= before + win && *deadline <= after + win,
            "the CLAMPED wall deadline rides into the deposit intent"
        );
        // #367: the returned quote's expires_at is the ACTIONABLE deadline —
        // the intent's tag (the full clamped wall deadline) sits exactly one
        // execute margin past it; the provider's 1 h echo rides into neither.
        assert_eq!(
            *deadline,
            quote.expires_at + DEPOSIT_EXECUTE_MARGIN_SECS,
            "intent tag = actionable display deadline + the execute margin"
        );
    }

    #[tokio::test]
    async fn quote_sets_the_binding_and_execute_threads_it_to_the_deposit_sender() {
        // FR-17 (#396): `quote` mints ONE spend binding — recorded on the durable
        // issued row AND surfaced on the returned `SwapQuote` DTO (the host records it
        // at its execute-authorize bracket) — and `execute` copies THAT value into the
        // deposit leg, so the sign-at-execute pull presents exactly the binding the
        // host reviewed. SDK-minted, never derived from the provider quote id (the
        // ack-integrity lesson: provider ids carry no uniqueness contract).
        let mock = echoing_mock(100_000, 3_600);
        let (svc, deposits) = service_recording(mock);
        let quote = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("in-bounds quote");
        assert!(
            quote.binding.is_some(),
            "quote mints a binding onto the DTO (the host's authorize-bracket record)"
        );
        // The provider echo's own DTO carried `binding: None` (`honest_quote`), so a
        // Some here is necessarily SERVICE-minted, never provider data passed through.
        svc.execute(&quote).await.expect("execute issued quote");
        let recorded = deposits.bindings.lock().expect("bindings");
        assert_eq!(recorded.len(), 1, "exactly one deposit send");
        assert!(
            recorded[0].is_some(),
            "the deposit pull is BOUND, never None"
        );
        assert_eq!(
            recorded[0].as_ref().map(SpendBinding::as_bytes),
            quote.binding.as_ref().map(SpendBinding::as_bytes),
            "the deposit sender receives the SAME binding the quote DTO surfaced \
             (the quote→issued-row→deposit copy chain, byte-equal)",
        );
    }

    #[tokio::test]
    async fn execute_outofzec_foreign_exact_is_refused_before_the_provider() {
        // S7 W1: when the user fixes the FOREIGN OUTPUT of an out-of-ZEC swap, the
        // ZEC deposit (`quote.zec_side`) is bounded by nothing the user typed — the
        // shape is refused at the request door, before any provider call.
        let req = QuoteRequest {
            exact: ExactSide::Out(SwapAmount::Foreign("41.5".into())),
            ..out_of_zec_request(100_000)
        };
        let mock = echoing_mock(100_000, 3_600);
        let err = service(mock.clone()).quote(req).await.expect_err("refused");
        assert!(
            matches!(err, SwapError::RequestInvalid { reason } if reason.contains("ZEC amount")),
            "the out-of-ZEC foreign-exact refusal, got {}",
            err.code()
        );
        // refused at the request door: the provider was never asked
        assert_eq!(*mock.quote_calls.lock().expect("mock"), 0, "no call");
    }

    #[tokio::test]
    async fn execute_intozec_registers_intent_without_a_deposit_send() {
        // IntoZec RECEIVES ZEC to our own fresh address — there is no wallet-side
        // deposit. Execute registers provider intent and drives NO deposit.
        let mock = Arc::new(MockSwapProvider::with_quote_fn(|req| {
            Ok(honest_quote(req, 100_000, 3_600))
        }));
        let (svc, deposits) = service_recording(mock.clone());
        let quote = svc.quote(into_zec_request()).await.expect("intozec quote");
        let id = svc.execute(&quote).await.expect("execute");
        assert_eq!(mock.executed.lock().expect("mock").as_slice(), &[id]);
        assert!(
            deposits.deposits.lock().expect("deposits").is_empty(),
            "IntoZec drives no wallet-side deposit"
        );
    }

    // ── W-swap-5 (#366): the execute → SwapRecordSink home row ────────────────

    #[tokio::test]
    async fn execute_outofzec_records_the_home_row_with_the_clamped_deadline() {
        // The durable home row commits at execute with the honest minimum: the
        // provider handle, the COARSE OutOfZec arm, the CLAMPED durable deposit
        // window (the same tag the deposit intent gates on — NOT the mock's 1 h
        // echo), and the settlement-ceiling self-lapse bound.
        let mock = echoing_mock(100_000, 3_600);
        let records = Arc::new(MemSwapRecords::default());
        let (svc, deposits) = service_with_records(mock, records.clone());
        let before = now_unix();
        let quote = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("quote issues");
        let after_quote = now_unix();
        let id = svc.execute(&quote).await.expect("execute");
        let after = now_unix();
        let rows = records.rows();
        assert_eq!(rows.len(), 1, "exactly one home row");
        assert_eq!(rows[0].id, id, "the row carries the provider handle");
        assert!(rows[0].out_of_zec, "the coarse direction arm");
        let win = SWAP_DEPOSIT_DEADLINE_OUT_OF_ZEC_SECS;
        let deadline = rows[0].deposit_deadline.expect("deadline recorded");
        assert!(
            deadline >= before + win && deadline <= after_quote + win,
            "the row's deposit window is the CLAMPED durable tag"
        );
        assert_eq!(
            deadline,
            deposits.deposits.lock().expect("deposits")[0].2,
            "row deadline == the deposit intent's tag (one SSOT)"
        );
        assert!(
            rows[0].expires_at_wall >= before + SWAP_SETTLEMENT_MAX_SECS
                && rows[0].expires_at_wall <= after + SWAP_SETTLEMENT_MAX_SECS,
            "self-lapse = execute-time + the settlement ceiling"
        );
        // #368: the OutOfZec home row carries its REFUND watch (address + index from the
        // durable record) so the sink arms the detection row atomically with the record —
        // and the Refunded re-arm has a durable source after the issued row is consumed.
        let watch = rows[0]
            .watch
            .as_ref()
            .expect("the refund watch rides the record");
        assert_eq!(
            Some(watch.address.as_str()),
            quote.refund_to.as_deref(),
            "the watch IS the refundTo the provider holds"
        );
        assert_eq!(watch.index, 1, "the stub refund's single-use index");
    }

    #[tokio::test]
    async fn execute_intozec_records_the_home_row_with_the_deposit_window() {
        // IntoZec: the home row commits too (the user's external deposit window as
        // the deadline), with NO wallet-side deposit.
        let mock = Arc::new(MockSwapProvider::with_quote_fn(|req| {
            Ok(honest_quote(req, 100_000, 3_600))
        }));
        let records = Arc::new(MemSwapRecords::default());
        let (svc, deposits) = service_with_records(mock, records.clone());
        let quote = svc.quote(into_zec_request()).await.expect("intozec quote");
        let id = svc.execute(&quote).await.expect("execute");
        let rows = records.rows();
        assert_eq!(rows.len(), 1, "exactly one home row");
        assert_eq!(rows[0].id, id);
        assert!(!rows[0].out_of_zec, "the coarse IntoZec arm");
        assert!(
            rows[0].deposit_deadline.is_some(),
            "the user's external deposit window rides the row"
        );
        assert!(deposits.deposits.lock().expect("deposits").is_empty());
        // #368: the IntoZec home row carries its DESTINATION watch — the settlement
        // extension the sink applies atomically (moved off the take port).
        let watch = rows[0]
            .watch
            .as_ref()
            .expect("the destination watch rides the record");
        assert!(
            watch.address.starts_with("u1dest"),
            "the watch IS the minted destination"
        );
        assert_eq!(watch.index, 1, "the counting stub's first single-use index");
    }

    #[tokio::test]
    async fn record_failure_aborts_the_execute_and_never_queues_the_deposit() {
        // RECORD-FIRST is load-bearing: a failing sink propagates the typed error
        // and the deposit leg NEVER runs — no durably-queued deposit can exist
        // without its home row (the invisible-armed-deposit window, closed
        // structurally). Fail-safe: the provider order is registered but unfunded;
        // it expires/refunds provider-side.
        let mock = echoing_mock(100_000, 3_600);
        let records = Arc::new(MemSwapRecords {
            fail: true,
            ..Default::default()
        });
        let (svc, deposits) = service_with_records(mock, records);
        let quote = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("quote issues");
        assert!(matches!(
            svc.execute(&quote).await,
            Err(SwapError::SwapStateUnavailable)
        ));
        assert!(
            deposits.deposits.lock().expect("deposits").is_empty(),
            "the deposit was NEVER attempted after the record write failed"
        );
    }

    #[test]
    fn execute_outcome_code_is_ok_or_the_swap_error_code() {
        // The §5.4 `wallet.swap` outcome vocabulary: `"ok"` on success, else the
        // SwapError's stable RW-SWAP-NNN code. Pinned as the pure fn it is so a
        // future variant gets a conscious mapping AND — the security property — so
        // NO outcome value can ever be a forbidden-token string. The deposit-leg
        // failure is `RW-SWAP-009`, NOT a `"deposit_*"` string the §5.4 substring
        // guard would (correctly) trip on.
        let ok: Result<SwapId, SwapError> = Ok(SwapId::new("swap-1"));
        assert_eq!(execute_outcome_code(&ok), "ok");
        for e in [
            SwapError::SwapDisabled,
            SwapError::QuoteExpired,
            SwapError::DepositSendFailed,
            SwapError::SwapStateUnavailable,
            SwapError::ProviderUnavailable,
            SwapError::RequestInvalid { reason: "x" },
        ] {
            // `code()` borrows `e` and returns a `&'static str`, so capture it before
            // `e` moves into the `Err` (SwapError is not `Clone`).
            let expected = e.code();
            let result: Result<SwapId, SwapError> = Err(e);
            let code = execute_outcome_code(&result);
            assert_eq!(code, expected, "outcome must be the stable error code");
            // Defense-in-depth: the value must carry no §5.4 forbidden token (the
            // guard's own scan is the SSOT; this pins it at the emit site too).
            let lc = code.to_lowercase();
            for bad in ["deposit", "address", "amount", "memo", "txid"] {
                assert!(
                    !lc.contains(bad),
                    "outcome {code:?} leaks forbidden token {bad:?}"
                );
            }
        }
    }

    #[tokio::test]
    async fn execute_emits_5_4_clean_wallet_swap_spans() {
        // §5.4: every `execute` emits a `wallet.swap` span carrying ONLY the provider
        // NAME, a coarse direction code, and the outcome code — NEVER a swap id, deposit
        // address, or amount. ONE capture test drives the shared `wallet.swap` callsite
        // (the codebase convention — the `wallet.sync` capture test is likewise the sole
        // driver of its callsite; multiple per-thread `set_default` capture tests racing
        // one callsite flake on tracing's interest cache). It exercises every arm
        // sequentially under one subscriber: (1) OutOfZec success, (2) IntoZec success,
        // (3) a deposit-leg FAILURE — the headline §5.4 dodge (the failed-execute outcome
        // is the stable `RW-SWAP-009` code, NEVER a `"deposit_*"` string the `deposit`
        // forbidden token would trip) — and the EARLY-REJECT paths that return BEFORE the
        // `direction` record (W-swap-3-c-2-ii real-world-edge): (4) a killed service
        // (`SwapDisabled`/`RW-SWAP-008`), (5) a never-issued/forged quote
        // (`RequestInvalid`/`RW-SWAP-005`), (6) an expired quote (`QuoteExpired`/
        // `RW-SWAP-003`). Each early reject MUST still emit a §5.4-clean span whose
        // `outcome` is the stable code and whose `direction` stays Empty (an un-issued
        // execute has no known direction; Empty fields aren't emitted), so the host's
        // outcome distribution is honest without ever leaking a secret. The whole
        // accumulated field set must be §5.4-clean.
        use crate::tracing_guard::{
            CaptureLayer, CapturedEvents, assert_5_4_clean, force_wallet_callsites_enabled,
        };
        use tracing_subscriber::prelude::*;
        force_wallet_callsites_enabled(); // keep callsites enabled under parallel capture
        let sink = CapturedEvents::default();
        let subscriber = tracing_subscriber::registry().with(CaptureLayer::new(sink.clone()));
        let _guard = tracing::subscriber::set_default(subscriber);

        // (1) OutOfZec success ⇒ provider=mock, direction=out_of_zec, outcome=ok.
        let (svc, _deposits) = service_recording(echoing_mock(100_000, 3_600));
        let quote = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("in-bounds quote");
        svc.execute(&quote).await.expect("execute");

        // (2) IntoZec success ⇒ direction=into_zec (no wallet-side deposit).
        let (svc2, _d2) = service_recording(echoing_mock(100_000, 3_600));
        let quote2 = svc2.quote(into_zec_request()).await.expect("intozec quote");
        svc2.execute(&quote2).await.expect("execute");

        // (3) Deposit-leg FAILURE ⇒ outcome=RW-SWAP-009 (the code, not a `deposit_*` string).
        let (svc3, _d3) = service_failing_deposit(echoing_mock(100_000, 3_600));
        let quote3 = svc3
            .quote(out_of_zec_request(100_000))
            .await
            .expect("in-bounds quote");
        assert!(matches!(
            svc3.execute(&quote3).await,
            Err(SwapError::DepositSendFailed)
        ));

        // (4) EARLY REJECT — a Hard-killed service refuses execute at the entry door
        // (`ensure_live` → `SwapDisabled`), BEFORE the issued-record gate ⇒ `direction`
        // is never recorded. The span still fires, `outcome=RW-SWAP-008`. Money-pin: the
        // §5.4 wrapper is pure observability — it returns the SAME decision the
        // un-instrumented body would (the typed `SwapDisabled`), never altering it.
        let (svc4, _d4) = service_recording(echoing_mock(100_000, 3_600));
        let quote4 = svc4
            .quote(out_of_zec_request(100_000))
            .await
            .expect("quote issues while live");
        svc4.set_kill(SwapKill::Hard);
        assert!(
            matches!(svc4.execute(&quote4).await, Err(SwapError::SwapDisabled)),
            "a killed service refuses execute typed — the span never changes the decision"
        );

        // (5) EARLY REJECT — a forged/never-issued quote. Typed `QuoteExpired`
        // since #367 (every take-miss reads "no longer valid — get a new quote").
        // Returns at the one-shot consume `ok_or`, before the direction record.
        let (svc5, _d5) = service_recording(echoing_mock(100_000, 3_600));
        let issued5 = svc5
            .quote(out_of_zec_request(100_000))
            .await
            .expect("in-bounds quote");
        let forged = SwapQuote {
            id: SwapId::new("never-issued-by-this-service"),
            ..issued5
        };
        assert!(
            matches!(svc5.execute(&forged).await, Err(SwapError::QuoteExpired)),
            "a never-issued quote is refused — span fires, outcome is the code"
        );

        // (6) EARLY REJECT — an expired quote (`QuoteExpired`/RW-SWAP-003), deterministic:
        // the quote is issued within the safety margin so the dual deadline gate trips at
        // execute (no sleep). Returns before the direction record.
        let (svc6, _d6) = service_recording(echoing_mock(100_000, DEADLINE_SAFETY_MARGIN_SECS / 2));
        let quote6 = svc6
            .quote(out_of_zec_request(100_000))
            .await
            .expect("quote issues (already within the margin)");
        assert!(
            matches!(svc6.execute(&quote6).await, Err(SwapError::QuoteExpired)),
            "an expired quote is refused — span fires, outcome is the code"
        );

        let fields = sink.fields();
        assert_5_4_clean(&fields); // no forbidden token, every name allowlisted
        // The span NAME lives in metadata (not a captured field); its presence is proven
        // by the recorded `provider` field set at span creation.
        let has = |n: &str, v: &str| fields.iter().any(|(fn_, fv)| fn_ == n && fv == v);
        assert!(
            has("provider", "mock"),
            "the span fired (provider name recorded)"
        );
        assert!(
            has("direction", "out_of_zec"),
            "OutOfZec direction recorded"
        );
        assert!(has("direction", "into_zec"), "IntoZec direction recorded");
        assert!(has("outcome", "ok"), "success outcome recorded");
        assert!(
            has("outcome", "RW-SWAP-009"),
            "the deposit-fail outcome is the stable error code, not a forbidden-token string"
        );
        // The early-reject outcomes are recorded as stable codes too — the span is honest
        // on the rejection paths a host most needs to see (kill / not-issued / expired).
        // Since #367 the never-issued take-miss ALSO records RW-SWAP-003 (QuoteExpired)
        // — one code for "this quote is no longer executable", however it got there.
        assert!(
            has("outcome", "RW-SWAP-008"),
            "the killed-service early reject records SwapDisabled's stable code"
        );
        assert!(
            has("outcome", "RW-SWAP-003"),
            "the never-issued + expired early rejects record QuoteExpired's stable code"
        );
        // §5.4 HONESTY of the Empty field: the early rejects return BEFORE the direction
        // record, so the ONLY direction values ever emitted are the two real ones from the
        // issued successes — never a fabricated direction for an un-issued execute, and
        // never an empty-string artifact.
        assert!(
            !has("direction", ""),
            "an un-issued execute leaves `direction` Empty (unrecorded), never a blank value"
        );
        assert_eq!(
            fields
                .iter()
                .filter(|(n, _)| n == "direction")
                .map(|(_, v)| v.as_str())
                .collect::<std::collections::BTreeSet<_>>(),
            std::collections::BTreeSet::from(["into_zec", "out_of_zec"]),
            "direction is recorded ONLY for issued executes — the two real directions, nothing else"
        );
    }

    #[tokio::test]
    async fn into_zec_quote_span_is_5_4_clean() {
        // §3.3b L5 / ADR-0530 (IZ-1b): every `quote` emits a `wallet.swap_quote` span carrying ONLY
        // the provider NAME, a coarse DIRECTION code (known up front from the request, so it is
        // recorded on EVERY path incl. a mint failure), and the outcome CODE — NEVER the minted
        // destination/refund address or any amount (the quote-time destination MINT is the
        // highest-risk address site). Drives the shared `wallet.swap_quote` callsite once over:
        // (1) OutOfZec success, (2) IntoZec success (a destination is minted — the §5.4 risk), and
        // (3) an IntoZec destination-MINT FAILURE whose outcome is the stable `RW-SWAP-012`, never a
        // leaked address.
        use crate::tracing_guard::{
            CaptureLayer, CapturedEvents, assert_5_4_clean, force_wallet_callsites_enabled,
        };
        use tracing_subscriber::prelude::*;
        force_wallet_callsites_enabled();
        let sink = CapturedEvents::default();
        let subscriber = tracing_subscriber::registry().with(CaptureLayer::new(sink.clone()));
        let _guard = tracing::subscriber::set_default(subscriber);

        // (1) OutOfZec quote ⇒ provider=mock, direction=out_of_zec, outcome=ok.
        service(echoing_mock(100_000, 3_600))
            .quote(out_of_zec_request(100_000))
            .await
            .expect("out_of_zec quote");

        // (2) IntoZec quote ⇒ direction=into_zec, outcome=ok — the destination is MINTED here; the
        // span must carry NO part of it.
        service(echoing_mock(100_000, 3_600))
            .quote(into_zec_request())
            .await
            .expect("into_zec quote");

        // (3) IntoZec quote with a FAILING destination mint ⇒ direction=into_zec (recorded up front),
        // outcome=RW-SWAP-012 (the stable code), and the address is never minted/leaked.
        let svc_fail = SwapService::new(
            echoing_mock(100_000, 3_600),
            Arc::new(StubRefunds),
            Arc::new(CountingDestinations {
                fail: true,
                ..Default::default()
            }),
            Arc::new(RecordingDeposit::default()),
            Arc::new(MemIssuedQuoteStore::default()),
            Arc::new(MemSwapRecords::default()),
            Network::Main,
        );
        assert!(matches!(
            svc_fail.quote(into_zec_request()).await,
            Err(SwapError::DestinationAddressUnavailable)
        ));

        let fields = sink.fields();
        assert_5_4_clean(&fields); // no forbidden token, every name allowlisted
        let has = |n: &str, v: &str| fields.iter().any(|(fn_, fv)| fn_ == n && fv == v);
        assert!(has("provider", "mock"), "the swap_quote span fired");
        assert!(
            has("direction", "out_of_zec"),
            "OutOfZec direction recorded"
        );
        assert!(has("direction", "into_zec"), "IntoZec direction recorded");
        assert!(has("outcome", "ok"), "success outcome recorded");
        assert!(
            has("outcome", "RW-SWAP-012"),
            "the destination-mint failure records DestinationAddressUnavailable's stable code, not an address"
        );
    }

    fn one_token(fresh: bool) -> TokenList {
        TokenList {
            tokens: vec![crate::swap::TokenInfo {
                chain: "near".into(),
                symbol: "USDC".into(),
                decimals: 6,
                provider_asset_id: "nep141:usdc".into(),
                price_usd: Some(1.0),
            }],
            fresh,
        }
    }

    #[tokio::test]
    async fn list_tokens_delegates_and_emits_a_5_4_clean_wallet_swap_tokens_span() {
        // §3.3b D5/L6 + §5.4: the service passes the provider's filtered list through UNCHANGED and
        // emits `wallet.swap_tokens` carrying ONLY the provider label, the pickable COUNT, and a
        // coarse outcome (`"ok"` fresh / `"stale"` served-from-cache) — NEVER a symbol/chain/price.
        use crate::tracing_guard::{
            CaptureLayer, CapturedEvents, assert_5_4_clean, force_wallet_callsites_enabled,
        };
        use tracing_subscriber::prelude::*;
        force_wallet_callsites_enabled();
        let sink = CapturedEvents::default();
        let subscriber = tracing_subscriber::registry().with(CaptureLayer::new(sink.clone()));
        let _guard = tracing::subscriber::set_default(subscriber);

        // (1) a FRESH list passes through + records outcome=ok.
        let m1 = Arc::new(MockSwapProvider::default());
        m1.set_tokens(one_token(true));
        let fresh = service(m1).list_tokens().await.expect("fresh list");
        assert_eq!(fresh.tokens.len(), 1);
        assert!(fresh.fresh);
        assert_eq!(fresh.tokens[0].symbol, "USDC", "passed through unchanged");

        // (2) a STALE (served-from-cache) list records outcome=stale.
        let m2 = Arc::new(MockSwapProvider::default());
        m2.set_tokens(one_token(false));
        let stale = service(m2).list_tokens().await.expect("stale list");
        assert!(!stale.fresh);

        let fields = sink.fields();
        assert_5_4_clean(&fields); // no token field leaked; every name allowlisted
        let has = |n: &str, v: &str| fields.iter().any(|(fn_, fv)| fn_ == n && fv == v);
        assert!(has("provider", "mock"), "the swap_tokens span fired");
        assert!(has("count", "1"), "the pickable-token count is recorded");
        assert!(has("outcome", "ok"), "a fresh fetch records ok");
        assert!(
            has("outcome", "stale"),
            "a served-from-cache fetch records stale"
        );
    }

    #[tokio::test]
    async fn list_tokens_on_a_killed_service_is_swap_disabled() {
        // §3.5: a killed instance lists NOTHING — `ensure_live` gates before the provider is
        // touched (a mock with no scripted list would return ProviderUnavailable if reached; the
        // SwapDisabled proves the gate fired first — zero swap traffic).
        let svc = service(Arc::new(MockSwapProvider::default()));
        svc.set_kill(SwapKill::Hard);
        assert!(matches!(
            svc.list_tokens().await,
            Err(SwapError::SwapDisabled)
        ));
    }

    #[tokio::test]
    async fn execute_is_one_shot_no_double_deposit() {
        // The swap-layer no-double-deposit guarantee: a quote executes AT MOST
        // once. A second execute (double-tap / perceived-failure retry) finds the
        // issued record consumed ⇒ `QuoteExpired` (#367 — "no longer valid, get a
        // new quote") — so the provider is never asked to register a second intent
        // and no second deposit is ever queued. (The §6.3 guard de-dups a SINGLE
        // intent; only this one-shot consume stops two DISTINCT deposits.)
        let mock = echoing_mock(100_000, 3_600);
        let (svc, deposits) = service_recording(mock.clone());
        let quote = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("quote issues");
        svc.execute(&quote).await.expect("first execute");
        assert!(
            matches!(svc.execute(&quote).await, Err(SwapError::QuoteExpired)),
            "a second execute of the same quote is refused (consumed)"
        );
        assert_eq!(
            deposits.deposits.lock().expect("deposits").len(),
            1,
            "exactly one deposit, never two"
        );
        assert_eq!(
            mock.executed.lock().expect("mock").len(),
            1,
            "provider intent registered exactly once"
        );
    }

    #[tokio::test]
    async fn execute_concurrent_double_tap_yields_one_deposit() {
        // The no-double-deposit guarantee under CONCURRENCY (the real-world
        // double-tap: two rapid taps fire two execute() calls that race). Both hit
        // the durable `take` FIRST (W-swap-3-c-3-ii's single-flight): the store
        // serializes the atomic claim, so exactly one gets `Some` and proceeds while
        // the other gets `None` ⇒ `QuoteExpired` (#367) BEFORE any provider intent or
        // deposit. Stronger than the sequential test above: it pins that the atomic
        // CLAIM (not the call ordering) is what stops two distinct deposits. `join!`
        // on the current-thread test runtime interleaves the two futures at the first
        // await (the durable `take`), so it is deterministic.
        let mock = echoing_mock(100_000, 3_600);
        let (svc, deposits) = service_recording(mock.clone());
        let svc = Arc::new(svc);
        let quote = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("quote issues");
        let (a, b) = tokio::join!(svc.execute(&quote), svc.execute(&quote));
        // exactly one Ok, one QuoteExpired (in either order)
        let oks = [&a, &b].iter().filter(|r| r.is_ok()).count();
        assert_eq!(oks, 1, "exactly one of two concurrent executes succeeds");
        assert!(
            [a, b]
                .into_iter()
                .any(|r| matches!(r, Err(SwapError::QuoteExpired))),
            "the loser of the race is refused (consumed)"
        );
        assert_eq!(
            deposits.deposits.lock().expect("deposits").len(),
            1,
            "exactly one deposit under a concurrent double-tap, never two"
        );
        assert_eq!(
            mock.executed.lock().expect("mock").len(),
            1,
            "provider intent registered exactly once"
        );
    }

    #[tokio::test]
    async fn deposit_tx_never_resubmitted_past_deadline() {
        // §8 (monotonic-anchored, inc-2d-swap-b): the at-execute dual deadline gate
        // is the strict PRE-FLIGHT — an expired quote NEVER queues a deposit (so
        // none can ever be resubmitted). Together with inc-2d-swap-a's wall-clock
        // resubmission exclusion (which stops an ALREADY-queued deposit being
        // re-proposed to a now-dead quote), a deposit tx is never (re)submitted past
        // its deadline. Both clocks, deterministically (no sleeps, §testing).

        // monotonic-dead: expires within the safety margin ⇒ act_deadline already past
        let mock = echoing_mock(100_000, DEADLINE_SAFETY_MARGIN_SECS / 2);
        let (svc, deposits) = service_recording(mock.clone());
        let quote = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("quote issues");
        assert!(matches!(
            svc.execute(&quote).await,
            Err(SwapError::QuoteExpired)
        ));
        assert!(
            deposits.deposits.lock().expect("deposits").is_empty(),
            "no deposit queued past the monotonic deadline"
        );
        assert!(
            mock.executed.lock().expect("mock").is_empty(),
            "provider.execute never reached past the gate"
        );

        // wall-dead: rewind the durable row's wall half (suspend-paused monotonic
        // posture; S8 — the record is the wall authority)
        let mock = echoing_mock(100_000, 3_600);
        let store = Arc::new(MemIssuedQuoteStore::default());
        let (svc, deposits) = service_with_store(mock.clone(), store.clone());
        let quote = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("quote issues");
        {
            let mut rows = store.rows.lock().expect("rows");
            rows.get_mut(&quote.id).expect("present").expires_at_wall = 0;
        }
        assert!(matches!(
            svc.execute(&quote).await,
            Err(SwapError::QuoteExpired)
        ));
        assert!(
            deposits.deposits.lock().expect("deposits").is_empty(),
            "no deposit queued past the wall deadline"
        );
        assert!(
            mock.executed.lock().expect("mock").is_empty(),
            "provider.execute never reached"
        );
    }

    #[tokio::test]
    async fn deposit_send_failure_propagates_typed() {
        // Error path: the provider registers intent, but the wallet deposit leg
        // fails (teardown / store error) ⇒ typed `DepositSendFailed`, never a
        // silent success. Fail-safe: nothing left the pool, the user re-quotes;
        // the provider refunds the un-deposited quote after its deadline. Order is
        // load-bearing — intent is registered (provider.executed == 1) BEFORE the
        // deposit is attempted, so a registered-but-undeposited quote is the only
        // reachable state, never a deposit with no registered intent.
        let mock = echoing_mock(100_000, 3_600);
        let (svc, deposits) = service_failing_deposit(mock.clone());
        let quote = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("quote issues");
        let err = svc.execute(&quote).await.expect_err("deposit leg fails");
        assert!(
            matches!(err, SwapError::DepositSendFailed),
            "got {}",
            err.code()
        );
        assert_eq!(
            mock.executed.lock().expect("mock").len(),
            1,
            "provider intent WAS registered before the deposit attempt"
        );
        assert_eq!(
            deposits.deposits.lock().expect("deposits").len(),
            1,
            "the deposit was ATTEMPTED exactly once (records-then-fails)"
        );
    }

    /// A `SwapPort` double that echoes quotes like [`echoing_mock`] but whose
    /// `execute()` FAILS `ProviderUnavailable` (the flaky-mobile-link case: the
    /// register-intent call dies mid-`.await`). Records nothing on execute — the
    /// quote is consumed by the service before the provider is reached, so the
    /// asserts read the SERVICE's registry + the deposit double, not this.
    struct FailingExecuteProvider {
        zec_side: i64,
        expires_in: u64,
    }
    #[async_trait]
    impl SwapPort for FailingExecuteProvider {
        fn name(&self) -> &'static str {
            "failing-execute"
        }
        async fn quote(&self, req: QuoteRequest) -> Result<SwapQuote, SwapError> {
            Ok(honest_quote(&req, self.zec_side, self.expires_in))
        }
        async fn execute(&self, _quote: &SwapQuote) -> Result<(), SwapError> {
            Err(SwapError::ProviderUnavailable)
        }
        async fn status(&self, _id: &SwapId, _provider_ref: &str) -> Result<SwapStatus, SwapError> {
            Ok(SwapStatus::Processing)
        }

        async fn list_tokens(&self) -> Result<TokenList, SwapError> {
            Err(SwapError::ProviderUnavailable) // not this double's concern (IZ-2)
        }
    }

    /// A `SwapPort` double that echoes quotes AND, on `execute()`, flips the
    /// service's kill to `Hard` BEFORE returning `Ok` — modelling a `set_kill`
    /// that lands in the TOCTOU window between the provider `.await` and the
    /// money-moving `send_deposit_now` (§3.5 honest-off re-check, service.rs:324).
    /// The back-reference is injected after the `Arc<SwapService>` exists
    /// (chicken-and-egg: the service owns the provider). `execute()` is `&self`
    /// and the deposit re-check is a synchronous `ensure_live`, so flipping here
    /// is deterministic — no clocks, no races.
    struct KillOnExecuteProvider {
        zec_side: i64,
        expires_in: u64,
        /// Set post-construction; the service flips itself to `Hard` the instant
        /// the provider registers intent.
        service: Mutex<Option<Arc<SwapService>>>,
    }
    impl KillOnExecuteProvider {
        fn new(zec_side: i64, expires_in: u64) -> Self {
            Self {
                zec_side,
                expires_in,
                service: Mutex::new(None),
            }
        }
        fn bind(&self, svc: Arc<SwapService>) {
            *self.service.lock().expect("bind") = Some(svc);
        }
    }
    #[async_trait]
    impl SwapPort for KillOnExecuteProvider {
        fn name(&self) -> &'static str {
            "kill-on-execute"
        }
        async fn quote(&self, req: QuoteRequest) -> Result<SwapQuote, SwapError> {
            Ok(honest_quote(&req, self.zec_side, self.expires_in))
        }
        async fn execute(&self, _quote: &SwapQuote) -> Result<(), SwapError> {
            // The kill lands AFTER provider.execute's entry but BEFORE the
            // service queues the deposit — exactly the §3.5 TOCTOU bound.
            if let Some(svc) = self.service.lock().expect("svc").as_ref() {
                svc.set_kill(SwapKill::Hard);
            }
            Ok(())
        }
        async fn status(&self, _id: &SwapId, _provider_ref: &str) -> Result<SwapStatus, SwapError> {
            Ok(SwapStatus::Processing)
        }

        async fn list_tokens(&self) -> Result<TokenList, SwapError> {
            Err(SwapError::ProviderUnavailable) // not this double's concern (IZ-2)
        }
    }

    #[tokio::test]
    async fn kill_between_provider_execute_and_deposit_stops_the_deposit() {
        // HEADLINE GAP (§3.5 honest-off re-check, service.rs:324): a set_kill(Hard)
        // that lands AFTER the entry ensure_live and AFTER provider.execute, but
        // BEFORE send_deposit_now, must STOP the money-moving leg. The provider intent
        // IS registered (a registered-but-undeposited quote refunds provider-side
        // after the deadline — fail-safe, nothing left the pool), but ZERO deposit
        // is queued and execute returns the typed SwapDisabled. Deterministic: the
        // provider double flips the kill synchronously inside execute().
        let provider = Arc::new(KillOnExecuteProvider::new(100_000, 3_600));
        let deposits = Arc::new(RecordingDeposit::default());
        let svc = Arc::new(SwapService::new(
            provider.clone(),
            Arc::new(StubRefunds),
            Arc::new(CountingDestinations::default()),
            deposits.clone(),
            Arc::new(MemIssuedQuoteStore::default()),
            Arc::new(MemSwapRecords::default()),
            Network::Main,
        ));
        provider.bind(svc.clone());
        let quote = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("quote issues while live");
        let err = svc
            .execute(&quote)
            .await
            .expect_err("kill stops the deposit");
        assert!(
            matches!(err, SwapError::SwapDisabled),
            "the post-execute re-check refuses typed, got {}",
            err.code()
        );
        // provider intent WAS registered (the kill landed after it) ...
        assert_eq!(
            provider
                .service
                .lock()
                .expect("svc")
                .as_ref()
                .map(|s| s.kill_severity().is_killed()),
            Some(true),
            "the kill did land Hard inside execute",
        );
        // ... but NO deposit was queued — the headline guarantee.
        assert!(
            deposits.deposits.lock().expect("deposits").is_empty(),
            "a kill in the execute→deposit window queues ZERO deposits (zero swap traffic)",
        );
        // and a re-execute is refused too: the instance is now Hard-killed, so the
        // ENTRY ensure_live (door #1, service.rs:280) shadows the one-shot-consume
        // check — the re-execute returns SwapDisabled (not RequestInvalid). Either
        // terminal outcome is fail-safe; pinning the actual ordering (door before
        // registry) is the point. Still ZERO deposits.
        assert!(
            matches!(svc.execute(&quote).await, Err(SwapError::SwapDisabled)),
            "a Hard-killed instance refuses re-execute at the entry door",
        );
        assert!(
            deposits.deposits.lock().expect("deposits").is_empty(),
            "still zero deposits after the re-execute attempt",
        );
    }

    #[tokio::test]
    async fn provider_execute_failure_consumes_quote_and_queues_no_deposit() {
        // FLAKY-MOBILE-LINK (service.rs:311-313): register-intent dies mid-await
        // (network drop). The quote is already consumed (one-shot), NO deposit is
        // queued (the send sits after a successful provider.execute), and execute
        // surfaces the PROVIDER's typed error. Recovery is re-quote: the registry
        // is now empty, so a re-execute of the same DTO is RequestInvalid (the
        // W2 in-memory fail-safe — price moves surface on re-quote).
        let provider = Arc::new(FailingExecuteProvider {
            zec_side: 100_000,
            expires_in: 3_600,
        });
        let deposits = Arc::new(RecordingDeposit::default());
        let svc = SwapService::new(
            provider,
            Arc::new(StubRefunds),
            Arc::new(CountingDestinations::default()),
            deposits.clone(),
            Arc::new(MemIssuedQuoteStore::default()),
            Arc::new(MemSwapRecords::default()),
            Network::Main,
        );
        let quote = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("quote issues");
        let err = svc
            .execute(&quote)
            .await
            .expect_err("provider.execute fails");
        assert!(
            matches!(err, SwapError::ProviderUnavailable),
            "the PROVIDER error surfaces typed (distinct from DepositSendFailed), got {}",
            err.code()
        );
        assert!(
            deposits.deposits.lock().expect("deposits").is_empty(),
            "no deposit is ever queued when provider.execute fails — nothing left the pool",
        );
        // the quote was consumed up front: re-execute is terminal (re-quote, not
        // retry) — typed `QuoteExpired` since #367
        assert!(
            matches!(svc.execute(&quote).await, Err(SwapError::QuoteExpired)),
            "the consumed quote can't be retried — the W2 fail-safe is re-quote",
        );
    }

    #[tokio::test]
    async fn intozec_provider_execute_failure_surfaces_typed_no_side_effect() {
        // IntoZec carries no deposit plan, so a provider.execute failure can only
        // surface the typed provider error — there is no deposit leg to confuse it
        // with. Pins that the IntoZec branch propagates the same way the OutOfZec
        // branch does (the failure is upstream of the deposit `if let`).
        let provider = Arc::new(FailingExecuteProvider {
            zec_side: 100_000,
            expires_in: 3_600,
        });
        let deposits = Arc::new(RecordingDeposit::default());
        let svc = SwapService::new(
            provider,
            Arc::new(StubRefunds),
            Arc::new(CountingDestinations::default()),
            deposits.clone(),
            Arc::new(MemIssuedQuoteStore::default()),
            Arc::new(MemSwapRecords::default()),
            Network::Main,
        );
        let quote = svc.quote(into_zec_request()).await.expect("intozec quote");
        let err = svc
            .execute(&quote)
            .await
            .expect_err("provider.execute fails");
        assert!(
            matches!(err, SwapError::ProviderUnavailable),
            "got {}",
            err.code()
        );
        assert!(
            deposits.deposits.lock().expect("deposits").is_empty(),
            "IntoZec never queues a wallet-side deposit regardless",
        );
    }

    #[tokio::test]
    async fn kill_between_quote_and_execute_queues_no_deposit() {
        // W2 in-memory-registry window (candidate 3): set_kill AFTER quote() but
        // BEFORE execute() ⇒ execute trips ensure_live#1 (SwapDisabled) before the
        // provider is consulted AND before any deposit. The existing
        // `manifest_kill_severity_reaches_swap_service` proves the SwapDisabled +
        // provider-untouched halves; this adds the missing assertion that ZERO
        // deposit was queued (it used the non-recording `service()`).
        let mock = echoing_mock(100_000, 3_600);
        let (svc, deposits) = service_recording(mock.clone());
        let quote = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("issues while live");
        svc.set_kill(SwapKill::WindDown);
        assert!(matches!(
            svc.execute(&quote).await,
            Err(SwapError::SwapDisabled)
        ));
        assert!(
            mock.executed.lock().expect("mock").is_empty(),
            "provider.execute never reached past the entry kill door",
        );
        assert!(
            deposits.deposits.lock().expect("deposits").is_empty(),
            "a kill between quote and execute queues ZERO deposits",
        );
    }

    #[tokio::test]
    async fn issued_quote_registry_is_capped() {
        // unstable-env review fold: a quote-looping integration bug hits a
        // typed wall at MAX_ISSUED_QUOTES, never unbounded memory.
        use std::sync::atomic::{AtomicU32, Ordering};
        let n = Arc::new(AtomicU32::new(0));
        let mock = Arc::new(MockSwapProvider::with_quote_fn({
            let n = n.clone();
            move |req| {
                let mut q = honest_quote(req, 100_000, 3_600);
                q.id = SwapId::new(format!("q-{}", n.fetch_add(1, Ordering::Relaxed)));
                Ok(q)
            }
        }));
        let svc = service(mock);
        for _ in 0..MAX_ISSUED_QUOTES {
            svc.quote(out_of_zec_request(100_000))
                .await
                .expect("under the cap");
        }
        assert!(matches!(
            svc.quote(out_of_zec_request(100_000)).await,
            Err(SwapError::RequestInvalid { .. })
        ));
    }

    #[tokio::test]
    async fn manifest_kill_severity_reaches_swap_service() {
        use crate::swap::resolve_manifest_kill;

        // ordering pin (M4): the derived `Ord` IS the monotonic-off ordering
        // `set_kill` relies on — a future variant inserted out of order would
        // break escalation and fail HERE.
        assert!(SwapKill::Live < SwapKill::WindDown && SwapKill::WindDown < SwapKill::Hard);

        // §8 named test (§3.5 layer 3 composition gate). PART 1 — the resolver
        // is TYPED and DEFINED, default WindDown, NEVER left unresolved:
        assert_eq!(resolve_manifest_kill(true, None), SwapKill::Live);
        assert_eq!(
            resolve_manifest_kill(true, Some(SwapKill::Hard)),
            SwapKill::Live,
            "enabled ⇒ Live regardless of a stale severity field"
        );
        assert_eq!(
            resolve_manifest_kill(false, None),
            SwapKill::WindDown,
            "a partial/unresolved off-signal defaults to the SAFE WindDown"
        );
        assert_eq!(
            resolve_manifest_kill(false, Some(SwapKill::WindDown)),
            SwapKill::WindDown
        );
        assert_eq!(
            resolve_manifest_kill(false, Some(SwapKill::Hard)),
            SwapKill::Hard
        );
        assert_eq!(
            resolve_manifest_kill(false, Some(SwapKill::Live)),
            SwapKill::WindDown,
            "a contradictory disabled+Live collapses to WindDown — disabled is at LEAST wound down"
        );

        // PART 2 — the severity REACHES the service: a killed instance refuses
        // new quotes (BOTH severities), typed, before the provider is consulted.
        for sev in [SwapKill::WindDown, SwapKill::Hard] {
            let mock = echoing_mock(100_000, 3_600);
            let svc = service(mock.clone());
            svc.set_kill(sev);
            assert!(
                matches!(
                    svc.quote(out_of_zec_request(100_000)).await,
                    Err(SwapError::SwapDisabled)
                ),
                "{sev:?}: quote must be refused"
            );
            assert_eq!(
                *mock.quote_calls.lock().expect("mock"),
                0,
                "{sev:?}: provider never consulted once killed"
            );
        }

        // execute is gated too: issue a quote LIVE, then kill, then execute is
        // refused before the provider's execute runs.
        let mock = echoing_mock(100_000, 3_600);
        let svc = service(mock.clone());
        let quote = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("issues while live");
        svc.set_kill(SwapKill::WindDown);
        assert!(matches!(
            svc.execute(&quote).await,
            Err(SwapError::SwapDisabled)
        ));
        assert!(
            mock.executed.lock().expect("mock").is_empty(),
            "nothing executed after a kill"
        );

        // PART 3 — MONOTONIC-TOWARD-OFF (review M4): a kill never resurrects.
        let svc = service(echoing_mock(100_000, 3_600));
        svc.set_kill(SwapKill::WindDown);
        svc.set_kill(SwapKill::Live); // weaker than current ⇒ no-op
        assert!(
            matches!(
                svc.quote(out_of_zec_request(100_000)).await,
                Err(SwapError::SwapDisabled)
            ),
            "set_kill(Live) must NOT revive a wound-down service"
        );
        // and a weaker directive can't walk Hard back to WindDown either.
        let svc = service(echoing_mock(100_000, 3_600));
        svc.set_kill(SwapKill::Hard);
        svc.set_kill(SwapKill::WindDown); // weaker ⇒ no-op, stays Hard
        assert!(matches!(
            svc.quote(out_of_zec_request(100_000)).await,
            Err(SwapError::SwapDisabled)
        ));
    }

    #[tokio::test]
    async fn hostile_provider_responses_rejected_typed() {
        // refundTo mismatch (the echo check)
        let mock = Arc::new(MockSwapProvider::with_quote_fn(|req| {
            let mut q = honest_quote(req, 100_000, 3_600);
            q.refund_to = Some("attacker-controlled".into());
            Ok(q)
        }));
        assert!(matches!(
            service(mock).quote(out_of_zec_request(100_000)).await,
            Err(SwapError::ProviderProtocol {
                reason: ProviderProtocolReason::RefundAddressMismatch
            })
        ));

        // oversized provider string (m1)
        let mock = Arc::new(MockSwapProvider::with_quote_fn(|req| {
            let mut q = honest_quote(req, 100_000, 3_600);
            q.deposit_address = "x".repeat(PROVIDER_STR_MAX_BYTES + 1);
            Ok(q)
        }));
        assert!(matches!(
            service(mock).quote(out_of_zec_request(100_000)).await,
            Err(SwapError::ProviderProtocol {
                reason: ProviderProtocolReason::OversizedField
            })
        ));

        // oversized deposit_memo (m1 — the field bounded by the validate_quote
        // fold; the inbound-FFI path bypasses the adapter funnel, so this door
        // must bound it too)
        let mock = Arc::new(MockSwapProvider::with_quote_fn(|req| {
            let mut q = honest_quote(req, 100_000, 3_600);
            q.deposit_memo = Some("x".repeat(PROVIDER_STR_MAX_BYTES + 1));
            Ok(q)
        }));
        assert!(matches!(
            service(mock).quote(out_of_zec_request(100_000)).await,
            Err(SwapError::ProviderProtocol {
                reason: ProviderProtocolReason::OversizedField
            })
        ));

        // a swap id inside the wallet-RESERVED `backfill:` namespace (#368 — a hostile
        // id there could pre-claim/extend a synthetic ADR-0527 watch row and silently
        // suppress refund detection; refused at the quote door, security fold)
        let mock = Arc::new(MockSwapProvider::with_quote_fn(|req| {
            let mut q = honest_quote(req, 100_000, 3_600);
            q.id = SwapId::new("backfill:7".to_owned());
            Ok(q)
        }));
        assert!(matches!(
            service(mock).quote(out_of_zec_request(100_000)).await,
            Err(SwapError::ProviderProtocol {
                reason: ProviderProtocolReason::ReservedId
            })
        ));

        // malformed decimal amount (m1)
        let mock = Arc::new(MockSwapProvider::with_quote_fn(|req| {
            let mut q = honest_quote(req, 100_000, 3_600);
            q.min_amount_out = "12.3.4".into();
            Ok(q)
        }));
        assert!(matches!(
            service(mock).quote(out_of_zec_request(100_000)).await,
            Err(SwapError::ProviderProtocol {
                reason: ProviderProtocolReason::MalformedAmount
            })
        ));

        // OutOfZec deposit address that is not a transparent addr on OUR
        // network (§4.4)
        let mock = Arc::new(MockSwapProvider::with_quote_fn(|req| {
            let mut q = honest_quote(req, 100_000, 3_600);
            q.deposit_address = "not-a-zcash-address".into();
            Ok(q)
        }));
        assert!(matches!(
            service(mock).quote(out_of_zec_request(100_000)).await,
            Err(SwapError::ProviderProtocol {
                reason: ProviderProtocolReason::DepositAddressInvalid
            })
        ));
    }

    #[test]
    fn decimal_parse_and_bps_math_exact() {
        // strict parse
        assert_eq!(parse_decimal("12.34"), Some((1234, 2)));
        assert_eq!(parse_decimal("0.001"), Some((1, 3)));
        assert_eq!(parse_decimal("41"), Some((41, 0)));
        assert_eq!(parse_decimal(".5"), Some((5, 1)));
        for bad in [
            "", ".", "1e9", "-1", "+1", "1.2.3", "12,34", "0x10", " 1", "1 ",
        ] {
            assert_eq!(parse_decimal(bad), None, "{bad:?} must not parse");
        }
        // digit cap (overflow guard)
        assert!(parse_decimal(&"9".repeat(SWAP_DECIMAL_MAX_DIGITS)).is_some());
        assert!(parse_decimal(&"9".repeat(SWAP_DECIMAL_MAX_DIGITS + 1)).is_none());
        // bps boundary at mixed scales: anchor 41.5, 2% = 0.83
        let anchor = parse_decimal("41.5").expect("ok");
        assert!(decimal_within_bps(
            parse_decimal("42.33").expect("ok"),
            anchor,
            200
        ));
        assert!(!decimal_within_bps(
            parse_decimal("42.331").expect("ok"),
            anchor,
            200
        ));
        assert!(decimal_within_bps(
            parse_decimal("40.67").expect("ok"),
            anchor,
            200
        ));
        assert!(!decimal_within_bps(
            parse_decimal("40.669").expect("ok"),
            anchor,
            200
        ));
    }

    #[test]
    fn decimal_bps_overflow_rejects_not_wraps() {
        // W2 review MAJOR regression vector: a 26-digit scale-0 anchor vs a
        // scale-12 value — the rescale lift used to overflow the unchecked
        // ×10_000 (panic in debug, silent in-bounds verdict in release).
        // Checked math must REJECT, deterministically, no panic.
        let anchor = parse_decimal(&"9".repeat(26)).expect("in-cap");
        let value = parse_decimal("0.111111111111").expect("in-cap");
        assert!(!decimal_within_bps(value, anchor, 200));
        assert!(!decimal_within_bps(anchor, value, 200));
        // and the 27-digit/scale-8 shape from the review PoC
        let anchor = parse_decimal(&format!("{}.{}", "9".repeat(19), "9".repeat(8))).expect("ok");
        let value = parse_decimal(&"9".repeat(27)).expect("ok");
        assert!(!decimal_within_bps(value, anchor, SLIPPAGE_MAX_BPS));
    }

    // ── W-swap-3-c-2 + -c-3-ii: the durable issued-quote claim ────────────────────
    // The durable `take` is THE single-flight (within-process AND cross-restart,
    // -c-3-ii); these pin persist-on-quote / claim-on-execute + the fail-closed money
    // posture. The prior within-process tests still pass because a same-run execute finds
    // BOTH the durable row (the claim) AND the in-memory record (the L-1 enrichment).

    #[tokio::test]
    async fn quote_persists_the_issued_quote_to_the_durable_store() {
        // Every issued quote is durably recorded (BOTH directions — IntoZec with a
        // `None` deposit) so a crash-then-requote is observable to recovery (W-swap-3-c-3).
        let store = Arc::new(MemIssuedQuoteStore::default());
        let (svc, _deposits) = service_with_store(echoing_mock(100_000, 3_600), store.clone());
        let before = now_unix();
        let q = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("OutOfZec quote issues");
        let after = now_unix();
        assert!(
            store.contains(&q.id),
            "the OutOfZec quote is durably recorded"
        );
        {
            let rows = store.rows.lock().expect("rows");
            let rec = rows.get(&q.id).expect("present");
            assert!(rec.deposit.is_some(), "OutOfZec carries the frozen deposit");
            assert_eq!(
                rec.deposit.as_ref().map(|d| d.amount),
                Some(q.zec_side),
                "the durable deposit amount is the quoted ZEC side"
            );
            // W-swap-4-a-3: the recorded wall deadline is CEILING-CLAMPED to the
            // window we request for a deposit-carrying quote (15 min) — the mock's
            // 1 h echo must NOT survive into the record the cross-restart execute
            // gate reads.
            let win = SWAP_DEPOSIT_DEADLINE_OUT_OF_ZEC_SECS;
            assert!(
                rec.expires_at_wall >= before + win && rec.expires_at_wall <= after + win,
                "the wall deadline is recorded clamped to the OutOfZec window"
            );
            // #367: the RETURNED quote's expires_at is the ACTIONABLE deadline —
            // exactly the durable wall deadline minus the deposit execute margin
            // (display and enforcement share one number by construction).
            assert_eq!(
                rec.expires_at_wall,
                q.expires_at + DEPOSIT_EXECUTE_MARGIN_SECS,
                "returned expires_at = the durable wall deadline − the execute margin"
            );
        }

        // IntoZec persists too, with NO deposit (a uniform consume path on execute).
        let store2 = Arc::new(MemIssuedQuoteStore::default());
        let mock2 = Arc::new(MockSwapProvider::with_quote_fn(|req| {
            Ok(honest_quote(req, 100_000, 3_600))
        }));
        let (svc2, _d2) = service_with_store(mock2, store2.clone());
        let qin = svc2.quote(into_zec_request()).await.expect("IntoZec quote");
        let rows = store2.rows.lock().expect("rows");
        assert!(
            rows.get(&qin.id).expect("present").deposit.is_none(),
            "IntoZec records a deposit-less durable row"
        );
        // #367: the IntoZec actionable deadline sits one SAFETY margin (not the
        // deposit execute margin — direction-aware) before the recorded wall bound.
        assert_eq!(
            rows.get(&qin.id).expect("present").expires_at_wall,
            qin.expires_at + DEADLINE_SAFETY_MARGIN_SECS,
            "IntoZec returned expires_at = wall deadline − the safety margin"
        );
    }

    #[tokio::test]
    async fn execute_consumes_the_durable_issued_quote() {
        // The atomic single-flight consume: after a live execute the durable row is GONE,
        // so recovery never sees an executed swap still looking in-flight.
        let store = Arc::new(MemIssuedQuoteStore::default());
        let (svc, deposits) = service_with_store(echoing_mock(100_000, 3_600), store.clone());
        let q = svc.quote(out_of_zec_request(100_000)).await.expect("quote");
        assert_eq!(store.count(), 1, "recorded at quote");
        svc.execute(&q).await.expect("execute");
        assert_eq!(store.count(), 0, "consumed at execute");
        assert!(!store.contains(&q.id));
        assert_eq!(
            deposits.deposits.lock().expect("deposits").len(),
            1,
            "and the deposit was queued — the durable claim succeeded, in-memory gave the L-1 address"
        );
    }

    #[tokio::test]
    async fn persist_at_capacity_maps_to_request_invalid() {
        // The durable store is now the SINGLE in-flight cap authority (the in-memory cap
        // check is gone). At capacity, a fetched-and-validated quote is refused with the
        // SAME `RequestInvalid` message the in-memory cap used to return (one cap message).
        let store = Arc::new(MemIssuedQuoteStore::default());
        for i in 0..MAX_ISSUED_QUOTES {
            store
                .persist(IssuedQuoteRecord {
                    refund: None,
                    id: SwapId::new(format!("fill-{i}")),
                    provider_ref: format!("t1fill{i}"),
                    terms: QuoteTerms {
                        deposit_address: format!("t1fill{i}"),
                        deposit_memo: None,
                        amount_in: "1.0".into(),
                        min_amount_out: "41.5".into(),
                        zec_side: Zatoshis::new(1).expect("valid"),
                        refund_to: None,
                        expires_at: 2_000_000_000,
                    },
                    deposit: None,
                    destination: None,
                    expires_at_wall: 2_000_000_000,
                    binding: None,
                })
                .await
                .expect("fill");
        }
        let (svc, _deposits) = service_with_store(echoing_mock(100_000, 3_600), store.clone());
        let err = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect_err("at capacity ⇒ refused");
        assert!(
            matches!(err, SwapError::RequestInvalid { reason } if reason.contains("in-flight")),
            "AtCapacity maps to the in-flight RequestInvalid, got {}",
            err.code()
        );
        assert_eq!(
            store.count(),
            MAX_ISSUED_QUOTES,
            "no row was added past the cap"
        );
    }

    #[tokio::test]
    async fn persist_failure_fails_the_quote_typed_and_issues_nothing() {
        // A tearing-down / errored durable store fails the quote CLOSED: a quote we cannot
        // durably record is NOT returned (typed `SwapStateUnavailable`), the provider was
        // consulted but nothing is issued, and a later execute of a hand-built DTO is refused
        // (the in-memory registry is empty — nothing leaked).
        let store = Arc::new(MemIssuedQuoteStore {
            fail_persist: true,
            ..Default::default()
        });
        let mock = echoing_mock(100_000, 3_600);
        let (svc, deposits) = service_with_store(mock.clone(), store.clone());
        let err = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect_err("persist failure fails the quote");
        assert!(
            matches!(err, SwapError::SwapStateUnavailable),
            "got {}",
            err.code()
        );
        assert_eq!(store.count(), 0, "nothing durably recorded");
        // a hand-built quote with the same shape is not executable — no in-memory entry exists
        // (the take-miss is `QuoteExpired` since #367; a store-fault take stays unavailable)
        let forged = honest_quote(&out_of_zec_request(100_000), 100_000, 3_600);
        assert!(matches!(
            svc.execute(&forged).await,
            Err(SwapError::QuoteExpired) | Err(SwapError::SwapStateUnavailable)
        ));
        assert!(
            deposits.deposits.lock().expect("deposits").is_empty(),
            "no deposit ever queued"
        );
    }

    #[tokio::test]
    async fn take_failure_aborts_execute_before_provider_intent_and_deposit() {
        // THE money-safety headline for the consume leg: a durable `take` that fails (the
        // store is tearing down) aborts execute FAIL-SAFE — typed `SwapStateUnavailable`,
        // the provider's execute is NEVER reached (no intent registered), and ZERO deposit
        // is queued. Nothing left the pool; the user re-quotes.
        let store = Arc::new(MemIssuedQuoteStore {
            fail_take: true,
            ..Default::default()
        });
        let mock = echoing_mock(100_000, 3_600);
        let (svc, deposits) = service_with_store(mock.clone(), store.clone());
        let q = svc.quote(out_of_zec_request(100_000)).await.expect("quote");
        let err = svc
            .execute(&q)
            .await
            .expect_err("take failure aborts execute");
        assert!(
            matches!(err, SwapError::SwapStateUnavailable),
            "got {}",
            err.code()
        );
        assert!(
            mock.executed.lock().expect("mock").is_empty(),
            "provider.execute is NEVER reached when the durable consume fails",
        );
        assert!(
            deposits.deposits.lock().expect("deposits").is_empty(),
            "and ZERO deposit is queued — nothing left the pool",
        );
    }

    #[tokio::test]
    async fn wall_expired_quote_consumes_the_durable_claim_but_never_reaches_signing() {
        // W-swap-3-c-3-ii contract change: the durable `take` is now the single-flight, so
        // it runs FIRST — an expired execute CONSUMES (claims) the durable row, THEN the dual
        // deadline gate rejects ⇒ `QuoteExpired`, provider never asked, NO deposit. Consuming
        // the expired row is money-safe (an expired quote must never be executed by recovery
        // either — `take`-first claims it so a later cross-restart execute also sees `None`).
        let store = Arc::new(MemIssuedQuoteStore::default());
        let (svc, deposits) = service_with_store(echoing_mock(100_000, 3_600), store.clone());
        let q = svc.quote(out_of_zec_request(100_000)).await.expect("quote");
        {
            // S8: the durable row is the wall authority — rewind it, not the registry.
            let mut rows = store.rows.lock().expect("rows");
            rows.get_mut(&q.id).expect("present").expires_at_wall = 0;
        }
        assert!(matches!(
            svc.execute(&q).await,
            Err(SwapError::QuoteExpired)
        ));
        assert!(
            deposits.deposits.lock().expect("deposits").is_empty(),
            "an expired quote queues no deposit (never reaches signing)",
        );
        assert!(
            !store.contains(&q.id),
            "the durable claim was consumed up front — recovery never re-drives a dead quote",
        );
    }

    #[tokio::test]
    async fn cross_restart_execute_reconstructs_the_deposit_from_the_durable_row_and_executes() {
        // THE W-swap-3-c-3-ii headline: after a restart (the in-memory registry is gone) an
        // execute RECONSTRUCTS the deposit from the durable row — re-parses + re-blesses the
        // deposit address, gates on the durable WALL deadline — registers intent, and queues
        // the SAME frozen deposit. The durable `take` is the single-flight, so the row is
        // consumed.
        let store = Arc::new(MemIssuedQuoteStore::default());
        let (svc, deposits) = service_with_store(echoing_mock(100_000, 3_600), store.clone());
        let before = now_unix();
        let q = svc.quote(out_of_zec_request(100_000)).await.expect("quote");
        let after = now_unix();
        let expected_addr = Address::parse(&q.deposit_address, Network::Main).expect("blessed");
        svc.issued.lock().expect("registry").clear(); // simulate a process restart (in-memory gone)
        let id = svc
            .execute(&q)
            .await
            .expect("a cross-restart execute reconstructs and proceeds");
        assert_eq!(id, q.id, "the reconstructed execute registered intent");
        let deposits = deposits.deposits.lock().expect("deposits");
        assert_eq!(
            deposits.len(),
            1,
            "exactly one deposit queued from the reconstruction"
        );
        assert_eq!(
            deposits[0].0, expected_addr,
            "the SAME frozen deposit address (re-blessed)"
        );
        assert_eq!(
            deposits[0].1,
            Zatoshis::new(100_000).expect("amt"),
            "the frozen ZEC amount"
        );
        let win = SWAP_DEPOSIT_DEADLINE_OUT_OF_ZEC_SECS;
        assert!(
            deposits[0].2 >= before + win && deposits[0].2 <= after + win,
            "tagged with the durable (ceiling-clamped) wall deadline"
        );
        assert!(
            !store.contains(&q.id),
            "the durable claim was consumed — a second recovery execute can't double it",
        );
    }

    #[tokio::test]
    async fn cross_restart_double_execute_recovery_claims_the_durable_row_once_no_double_deposit() {
        // The owed `take`-None double-execute recovery test: TWO cross-restart executes of the
        // SAME quote (e.g. a crash mid-execute then a retry, or a double-tap after a restart)
        // must queue EXACTLY ONE deposit. The atomic durable `take` claims the row on the first;
        // the second finds `None` ⇒ `QuoteExpired` (#367), zero second deposit.
        let store = Arc::new(MemIssuedQuoteStore::default());
        let (svc, deposits) = service_with_store(echoing_mock(100_000, 3_600), store.clone());
        let q = svc.quote(out_of_zec_request(100_000)).await.expect("quote");
        svc.issued.lock().expect("registry").clear(); // restart: in-memory gone, durable survives
        svc.execute(&q)
            .await
            .expect("first cross-restart execute reconstructs + proceeds");
        // A second execute (another restart / a retry) finds the durable row already claimed.
        assert!(
            matches!(svc.execute(&q).await, Err(SwapError::QuoteExpired)),
            "the durable single-flight refuses the second execute — no double-deposit",
        );
        assert_eq!(
            deposits.deposits.lock().expect("deposits").len(),
            1,
            "EXACTLY ONE deposit across two recovery executes — the money invariant",
        );
    }

    #[tokio::test]
    async fn execute_without_a_durable_claim_is_quote_expired() {
        // W-swap-3-c-3-ii: the durable `take` is now the authority, so a `None` is LOAD-BEARING
        // — no durable claim ⇒ no authority to execute ⇒ `QuoteExpired` (#367: every miss reads
        // "no longer valid — get a new quote"), even if a stale in-memory record lingers. (In
        // production a durable `None` for a live in-memory record can't happen — a row is only
        // pruned once it has lapsed, which the in-memory wall gate would also reject — but the
        // durable claim is the un-bypassable guard regardless.)
        let store = Arc::new(MemIssuedQuoteStore::default());
        let (svc, deposits) = service_with_store(echoing_mock(100_000, 3_600), store.clone());
        let q = svc.quote(out_of_zec_request(100_000)).await.expect("quote");
        store.rows.lock().expect("rows").remove(&q.id); // the durable row is gone
        assert!(
            matches!(svc.execute(&q).await, Err(SwapError::QuoteExpired)),
            "no durable claim ⇒ QuoteExpired (the durable take is the single-flight authority)",
        );
        assert!(
            deposits.deposits.lock().expect("deposits").is_empty(),
            "zero deposit without a durable claim",
        );
    }

    #[tokio::test]
    async fn cross_restart_execute_into_zec_reconstructs_register_only_no_deposit() {
        // The IntoZec cross-restart arm: the durable row carries a `None` deposit (ZEC is
        // RECEIVED to our own address — no wallet-side send), so a reconstructed execute
        // registers provider intent ONLY and queues ZERO deposit. Closes the reconstruction
        // matrix (OutOfZec cross-restart is covered; this is IntoZec).
        let store = Arc::new(MemIssuedQuoteStore::default());
        let (svc, deposits) = service_with_store(echoing_mock(100_000, 3_600), store.clone());
        let q = svc.quote(into_zec_request()).await.expect("intozec quote");
        svc.issued.lock().expect("registry").clear(); // restart: in-memory gone
        svc.execute(&q)
            .await
            .expect("intozec reconstructs (register-only)");
        assert!(
            !store.contains(&q.id),
            "the durable claim was consumed (register-only is still single-flight)",
        );
        assert!(
            deposits.deposits.lock().expect("deposits").is_empty(),
            "IntoZec queues NO wallet-side deposit — ZEC is received, not sent",
        );
    }

    #[tokio::test]
    async fn cross_restart_execute_with_a_tampered_durable_deposit_address_fails_closed() {
        // The tampered-sealed-DB redirect vector (security re-review): on the cross-restart
        // path the deposit address is re-parsed from the durable string — so a tampered row
        // (here a mainnet→TESTNET address swap, valid transparent but wrong network for this
        // Main wallet) MUST fail closed BEFORE `provider.execute`, never sending a deposit to
        // the attacker's address. `reconstruct_deposit_plan` re-blesses transparent +
        // right-network ⇒ a mismatch is `SwapStateUnavailable`.
        use zcash_address::ToAddress;
        use zcash_protocol::consensus::NetworkType;
        let store = Arc::new(MemIssuedQuoteStore::default());
        let mock = echoing_mock(100_000, 3_600);
        let (svc, deposits) = service_with_store(Arc::clone(&mock), store.clone());
        let q = svc.quote(out_of_zec_request(100_000)).await.expect("quote");
        // Tamper the DURABLE deposit address to a wrong-network (testnet) t-addr.
        let tampered =
            zcash_address::ZcashAddress::from_transparent_p2pkh(NetworkType::Test, [0x44; 20])
                .encode();
        {
            let mut rows = store.rows.lock().expect("rows");
            let row = rows.get_mut(&q.id).expect("row");
            row.deposit.as_mut().expect("out-of-zec deposit").address = tampered.clone();
        }
        svc.issued.lock().expect("registry").clear(); // restart: forces the durable re-parse path
        let err = svc
            .execute(&q)
            .await
            .expect_err("a tampered durable deposit address fails closed (no redirect)");
        assert!(matches!(err, SwapError::SwapStateUnavailable));
        assert_eq!(
            *mock.executed.lock().expect("executed"),
            Vec::<SwapId>::new(),
            "the provider was NEVER reached — fail-closed precedes register-intent",
        );
        assert!(
            deposits.deposits.lock().expect("deposits").is_empty(),
            "ZERO deposit — nothing went to the tampered address",
        );
        // §5.4 + mobile crash-log: the fail-closed error is payload-free — it leaks NEITHER
        // the tampered address NOR the amount into a `Display`/code a host might log.
        let rendered = format!("{err} {}", err.code());
        assert!(
            !rendered.contains(&tampered) && !rendered.contains("100000"),
            "the reconstruction-failure error must not surface the address or amount",
        );
    }

    #[tokio::test]
    async fn cross_restart_execute_of_a_wall_expired_durable_row_is_quote_expired() {
        // The cross-restart arm's wall-only deadline gate (the recovery deadline floor): a
        // durable row whose wall deadline has lapsed must NOT be reconstructed-and-executed —
        // recovery never re-drives a dead quote. The claim is consumed (take-first), the gate
        // returns `QuoteExpired`, and ZERO deposit is queued. Pins the `None`-arm wall gate
        // (the within-process arm is covered by `wall_expired_quote_consumes_the_durable_claim…`).
        let store = Arc::new(MemIssuedQuoteStore::default());
        let mock = echoing_mock(100_000, 3_600);
        let (svc, deposits) = service_with_store(Arc::clone(&mock), store.clone());
        let q = svc.quote(out_of_zec_request(100_000)).await.expect("quote");
        store
            .rows
            .lock()
            .expect("rows")
            .get_mut(&q.id)
            .expect("row")
            .expires_at_wall = 0; // the durable wall deadline has lapsed
        svc.issued.lock().expect("registry").clear(); // restart: forces the wall-only gate
        assert!(matches!(
            svc.execute(&q).await,
            Err(SwapError::QuoteExpired)
        ));
        assert_eq!(
            *mock.executed.lock().expect("executed"),
            Vec::<SwapId>::new(),
            "an expired cross-restart quote never reaches the provider",
        );
        assert!(
            deposits.deposits.lock().expect("deposits").is_empty(),
            "zero deposit for an expired reconstructed quote",
        );
        assert!(
            !store.contains(&q.id),
            "the durable claim was consumed — a later execute also sees None (never re-driven)",
        );
    }

    #[tokio::test]
    async fn kill_during_a_reconstructed_execute_stops_the_deposit_and_leaves_the_row_consumed() {
        // §3.5 honest-off on the CROSS-RESTART path: a Hard kill landing during a RECONSTRUCTED
        // execute (after the durable claim, as `provider.execute` registers intent) must STILL
        // stop the deposit — the post-`provider.execute` `ensure_live` re-check fires identically
        // whether the deposit came from the in-memory record or from reconstruction (both arms
        // converge into the one `deposit` binding). And because `take` ran first, the durable row
        // is already consumed ⇒ recovery never re-drives it.
        let provider = Arc::new(KillOnExecuteProvider::new(100_000, 3_600));
        let deposits = Arc::new(RecordingDeposit::default());
        let store = Arc::new(MemIssuedQuoteStore::default());
        let svc = Arc::new(SwapService::new(
            provider.clone(),
            Arc::new(StubRefunds),
            Arc::new(CountingDestinations::default()),
            deposits.clone(),
            store.clone(),
            Arc::new(MemSwapRecords::default()),
            Network::Main,
        ));
        provider.bind(svc.clone());
        let quote = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("quote issues while live");
        svc.issued.lock().expect("registry").clear(); // restart: force the reconstruction arm
        assert!(
            matches!(svc.execute(&quote).await, Err(SwapError::SwapDisabled)),
            "the post-execute re-check stops a reconstructed deposit too",
        );
        assert!(
            deposits.deposits.lock().expect("deposits").is_empty(),
            "ZERO deposit — a kill on the reconstructed path stops the money leg",
        );
        assert_eq!(
            store.count(),
            0,
            "the durable claim was consumed (take-first) — recovery never re-drives it",
        );
    }

    // ── re-review (S78): the FAILURE-path durable post-state. `take` runs BEFORE
    // `provider.execute` AND before the deposit, so EVERY execute that consumed the row leaves
    // it gone — recovery must never re-drive a swap whose durable row a failed execute already
    // consumed. The existing provider-fail / deposit-fail / kill-in-window tests used throwaway
    // stores and asserted this only on the happy path; these pin the durable row on each leg. ──

    #[tokio::test]
    async fn provider_execute_failure_still_consumes_the_durable_row() {
        // A flaky-link `provider.execute` failure: the durable row is consumed (take precedes
        // provider.execute), the swap did NOT execute, and no deposit was queued — fail-safe AND
        // recovery-clean (no stale in-flight row for a never-registered swap).
        let store = Arc::new(MemIssuedQuoteStore::default());
        let deposits = Arc::new(RecordingDeposit::default());
        let svc = SwapService::new(
            Arc::new(FailingExecuteProvider {
                zec_side: 100_000,
                expires_in: 3_600,
            }),
            Arc::new(StubRefunds),
            Arc::new(CountingDestinations::default()),
            deposits.clone(),
            store.clone(),
            Arc::new(MemSwapRecords::default()),
            Network::Main,
        );
        let q = svc.quote(out_of_zec_request(100_000)).await.expect("quote");
        assert_eq!(store.count(), 1, "recorded at quote");
        assert!(matches!(
            svc.execute(&q).await,
            Err(SwapError::ProviderUnavailable)
        ));
        assert_eq!(
            store.count(),
            0,
            "the durable row is consumed even when provider.execute fails — recovery-clean",
        );
        assert!(
            deposits.deposits.lock().expect("deposits").is_empty(),
            "no deposit queued on a provider failure",
        );
    }

    #[tokio::test]
    async fn deposit_send_failure_still_consumes_the_durable_row() {
        // The registered-but-undeposited fail-safe state carries NO stale durable in-flight row:
        // the deposit leg fails AFTER the durable take, so the row is already consumed. (The
        // existing `deposit_send_failure_propagates_typed` used a throwaway store.)
        let store = Arc::new(MemIssuedQuoteStore::default());
        let deposits = Arc::new(RecordingDeposit {
            fail: true,
            ..Default::default()
        });
        let svc = SwapService::new(
            echoing_mock(100_000, 3_600),
            Arc::new(StubRefunds),
            Arc::new(CountingDestinations::default()),
            deposits.clone(),
            store.clone(),
            Arc::new(MemSwapRecords::default()),
            Network::Main,
        );
        let q = svc.quote(out_of_zec_request(100_000)).await.expect("quote");
        assert!(matches!(
            svc.execute(&q).await,
            Err(SwapError::DepositSendFailed)
        ));
        assert_eq!(
            store.count(),
            0,
            "the durable row is consumed even when the deposit leg fails — recovery-clean",
        );
        assert_eq!(
            deposits.deposits.lock().expect("deposits").len(),
            1,
            "the deposit was attempted exactly once",
        );
    }

    #[tokio::test]
    async fn kill_in_the_execute_deposit_window_still_consumes_the_durable_row() {
        // A `Hard` kill landing inside `provider.execute` (the §3.5 TOCTOU window) stops the
        // deposit — and because the durable take ran BEFORE provider.execute, the row is already
        // consumed: zero deposit AND recovery-clean. (Extends `kill_between_provider_execute_and_
        // deposit_stops_the_deposit`, which used a throwaway store.)
        let provider = Arc::new(KillOnExecuteProvider::new(100_000, 3_600));
        let deposits = Arc::new(RecordingDeposit::default());
        let store = Arc::new(MemIssuedQuoteStore::default());
        let svc = Arc::new(SwapService::new(
            provider.clone(),
            Arc::new(StubRefunds),
            Arc::new(CountingDestinations::default()),
            deposits.clone(),
            store.clone(),
            Arc::new(MemSwapRecords::default()),
            Network::Main,
        ));
        provider.bind(svc.clone());
        let q = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("quote while live");
        assert!(matches!(
            svc.execute(&q).await,
            Err(SwapError::SwapDisabled)
        ));
        assert!(
            deposits.deposits.lock().expect("deposits").is_empty(),
            "zero deposit on a kill in the window",
        );
        assert_eq!(
            store.count(),
            0,
            "the durable row was consumed before the kill landed — recovery-clean",
        );
    }

    // ── W-swap-3-c-3-i — status polling + the live `watch_swap_status` stream ─────

    /// A scriptable provider that serves `status()` results FIFO — UNLIKE the
    /// public [`MockSwapProvider`] it can script a transient FAULT (`Err`), so the
    /// "a fault NEVER ends the stream" contract is testable. Empty queue ⇒
    /// `Ok(Processing)` (an in-flight swap that never settles). Counts `status`
    /// calls so a test can assert "no straggler poll after a stop".
    struct PollProvider {
        results: Mutex<std::collections::VecDeque<Result<SwapStatus, SwapError>>>,
        polls: Mutex<u32>,
    }
    impl PollProvider {
        fn new(results: Vec<Result<SwapStatus, SwapError>>) -> Self {
            Self {
                results: Mutex::new(results.into()),
                polls: Mutex::new(0),
            }
        }
        fn poll_count(&self) -> u32 {
            *self.polls.lock().expect("polls poisoned")
        }
    }
    #[async_trait]
    impl SwapPort for PollProvider {
        fn name(&self) -> &'static str {
            "poll-mock"
        }

        async fn list_tokens(&self) -> Result<TokenList, SwapError> {
            Err(SwapError::ProviderUnavailable) // not this double's concern (IZ-2)
        }
        async fn quote(&self, _req: QuoteRequest) -> Result<SwapQuote, SwapError> {
            Err(SwapError::ProviderUnavailable) // the poll tests never quote
        }
        async fn execute(&self, _quote: &SwapQuote) -> Result<(), SwapError> {
            Ok(())
        }
        async fn status(&self, _id: &SwapId, _provider_ref: &str) -> Result<SwapStatus, SwapError> {
            *self.polls.lock().expect("polls poisoned") += 1;
            self.results
                .lock()
                .expect("results poisoned")
                .pop_front()
                .unwrap_or(Ok(SwapStatus::Processing))
        }
    }

    /// A [`SwapStatusSink`] that RECORDS every emitted status. `stop_after`
    /// models the host dropping its subscription (`AppLifecycleState.paused` /
    /// dispose): once that many statuses have arrived, `emit` returns `false`
    /// (the §7 foreground-only stop). `None` ⇒ never host-cancels.
    struct RecordingSink {
        got: Arc<Mutex<Vec<SwapStatus>>>,
        stop_after: Option<usize>,
    }
    impl SwapStatusSink for RecordingSink {
        fn emit(&mut self, status: SwapStatus) -> bool {
            let mut got = self.got.lock().expect("got poisoned");
            got.push(status);
            self.stop_after.is_none_or(|n| got.len() < n)
        }
    }

    /// A sink that, on the `at`-th emit (0-based), escalates the SERVICE kill to
    /// `severity` — the runtime manifest flip landing WHILE a poll stream is live
    /// (§3.5). It always returns `true` (it never host-cancels), so the loop ends
    /// only via the kill divergence or a terminal status.
    struct KillAtSink {
        got: Arc<Mutex<Vec<SwapStatus>>>,
        svc: Arc<SwapService>,
        at: usize,
        severity: SwapKill,
    }
    impl SwapStatusSink for KillAtSink {
        fn emit(&mut self, status: SwapStatus) -> bool {
            let idx = {
                let mut got = self.got.lock().expect("got poisoned");
                got.push(status);
                got.len() - 1
            };
            if idx == self.at {
                self.svc.set_kill(self.severity);
            }
            true
        }
    }

    fn poll_service(provider: Arc<dyn SwapPort>) -> SwapService {
        SwapService::new(
            provider,
            Arc::new(StubRefunds),
            Arc::new(CountingDestinations::default()),
            Arc::new(RecordingDeposit::default()),
            Arc::new(MemIssuedQuoteStore::default()),
            Arc::new(MemSwapRecords::default()),
            Network::Main,
        )
    }

    #[test]
    fn swap_status_is_terminal_classifies_every_variant() {
        use crate::swap::types::SwapFailureCode;
        // Terminal — polling STOPS (§7).
        assert!(
            SwapStatus::Success {
                out_txid: None,
                realized_slippage_bps: None
            }
            .is_terminal()
        );
        assert!(SwapStatus::Refunded { refund_txid: None }.is_terminal());
        assert!(
            SwapStatus::Failed {
                code: SwapFailureCode::ProviderFailure
            }
            .is_terminal()
        );
        // In flight — polling CONTINUES.
        assert!(!SwapStatus::PendingDeposit { expires_at: 0 }.is_terminal());
        assert!(
            !SwapStatus::UnderDeposited {
                received: "1".into(),
                missing: "2".into(),
                deadline: 0
            }
            .is_terminal()
        );
        assert!(!SwapStatus::DepositDetected.is_terminal());
        assert!(!SwapStatus::Processing.is_terminal());
    }

    #[test]
    fn next_poll_backoff_doubles_then_caps_at_max() {
        // §7 cadence boundary (gate 7): 5 → 10 → 20 → 40 → 60 (capped at
        // SWAP_POLL_MAX_SECS), and the cap is a FIXED POINT (the
        // `next_backoff_doubles_then_caps_at_max` sync precedent).
        let secs = |d: Duration| d.as_secs();
        let mut b = Duration::from_secs(SWAP_POLL_INITIAL_SECS);
        assert_eq!(secs(b), 5, "starts at SWAP_POLL_INITIAL_SECS");
        for expected in [10, 20, 40] {
            b = next_poll_backoff(b);
            assert_eq!(secs(b), expected, "doubles below the cap");
        }
        b = next_poll_backoff(b); // 40*2 = 80 → capped to 60
        assert_eq!(
            secs(b),
            SWAP_POLL_MAX_SECS,
            "doubling past the cap clamps to it"
        );
        assert_eq!(secs(b), 60);
        assert_eq!(
            secs(next_poll_backoff(b)),
            SWAP_POLL_MAX_SECS,
            "the cap is a fixed point — never grows past SWAP_POLL_MAX_SECS",
        );
    }

    #[test]
    fn swap_kill_stops_polling_only_at_hard_or_more_off() {
        // The §3.5 WindDown↔Hard divergence as the one predicate the poll loop reads:
        // WindDown keeps in-flight polling (observational); Hard stops it.
        assert!(!SwapKill::Live.stops_polling());
        assert!(
            !SwapKill::WindDown.stops_polling(),
            "WindDown polls to terminal"
        );
        assert!(
            SwapKill::Hard.stops_polling(),
            "Hard halts in-flight polling"
        );
        // and it is the SAME monotonic-off ordering `set_kill` escalates along.
        assert!(SwapKill::Live < SwapKill::WindDown && SwapKill::WindDown < SwapKill::Hard);
    }

    #[tokio::test(start_paused = true)]
    async fn swap_status_poll_emits_each_status_until_terminal_then_ends() {
        // The headline: every status flows through, polling STOPS on the terminal,
        // and NO poll fires past it (the §7 "stops on terminal status").
        let provider = Arc::new(PollProvider::new(vec![
            Ok(SwapStatus::PendingDeposit { expires_at: 9 }),
            Ok(SwapStatus::DepositDetected),
            Ok(SwapStatus::Processing),
            Ok(SwapStatus::Success {
                out_txid: Some("abc".into()),
                realized_slippage_bps: Some(12),
            }),
        ]));
        let svc = Arc::new(poll_service(Arc::clone(&provider) as Arc<dyn SwapPort>));
        let got = Arc::new(Mutex::new(Vec::new()));
        run_swap_status_poll(
            Arc::clone(&svc.provider),
            svc.kill.subscribe(),
            SwapId::new("q-1"),
            "t1poll".to_owned(),
            RecordingSink {
                got: Arc::clone(&got),
                stop_after: None,
            },
        )
        .await;
        let got = got.lock().expect("got").clone();
        assert_eq!(got.len(), 4, "every status emitted, terminal last");
        assert!(
            got[3].is_terminal(),
            "the last emitted status is the terminal"
        );
        assert_eq!(
            provider.poll_count(),
            4,
            "exactly four polls — NONE past the terminal",
        );
    }

    #[tokio::test(start_paused = true)]
    async fn swap_status_poll_survives_a_transient_provider_fault() {
        // A `provider.status` error is NOT terminal — the stream rides it (backs off
        // and retries), never closing on a transient fault (the watch_sync_status
        // "a stall is DATA, not a dead stream" contract). The fault emits NOTHING.
        let provider = Arc::new(PollProvider::new(vec![
            Err(SwapError::ProviderUnavailable),
            Ok(SwapStatus::Processing),
            Ok(SwapStatus::Success {
                out_txid: None,
                realized_slippage_bps: None,
            }),
        ]));
        let svc = Arc::new(poll_service(Arc::clone(&provider) as Arc<dyn SwapPort>));
        let got = Arc::new(Mutex::new(Vec::new()));
        run_swap_status_poll(
            Arc::clone(&svc.provider),
            svc.kill.subscribe(),
            SwapId::new("q-1"),
            "t1poll".to_owned(),
            RecordingSink {
                got: Arc::clone(&got),
                stop_after: None,
            },
        )
        .await;
        let got = got.lock().expect("got").clone();
        assert_eq!(
            got.len(),
            2,
            "the fault emitted nothing; the two Ok statuses did"
        );
        assert!(matches!(got[0], SwapStatus::Processing));
        assert!(got[1].is_terminal());
        assert_eq!(provider.poll_count(), 3, "the fault was retried, not fatal");
    }

    #[tokio::test(start_paused = true)]
    async fn swap_status_poll_not_found_terminates_after_consecutive_threshold() {
        // #367 poll policy: a provider-definitive
        // SwapNotFound answered SWAP_NOT_FOUND_TERMINAL_POLLS consecutive polls
        // synthesizes the terminal Failed(NotFound) and ENDS the stream — a
        // re-attached provider-GC'd order reaches an honest terminal instead of
        // spinning forever.
        let not_found = || {
            Err(SwapError::ProviderProtocol {
                reason: ProviderProtocolReason::SwapNotFound,
            })
        };
        let provider = Arc::new(PollProvider::new(
            (0..SWAP_NOT_FOUND_TERMINAL_POLLS)
                .map(|_| not_found())
                .collect(),
        ));
        let svc = Arc::new(poll_service(Arc::clone(&provider) as Arc<dyn SwapPort>));
        let got = Arc::new(Mutex::new(Vec::new()));
        run_swap_status_poll(
            Arc::clone(&svc.provider),
            svc.kill.subscribe(),
            SwapId::new("q-gone"),
            "t1poll".to_owned(),
            RecordingSink {
                got: Arc::clone(&got),
                stop_after: None,
            },
        )
        .await;
        let got = got.lock().expect("got").clone();
        assert_eq!(got.len(), 1, "exactly the synthetic terminal emitted");
        assert!(
            matches!(
                got[0],
                SwapStatus::Failed {
                    code: SwapFailureCode::NotFound
                }
            ),
            "the synthetic terminal is Failed(NotFound)"
        );
        assert_eq!(
            provider.poll_count(),
            SWAP_NOT_FOUND_TERMINAL_POLLS,
            "the loop stopped AT the threshold — no poll past it",
        );
    }

    #[tokio::test(start_paused = true)]
    async fn swap_status_poll_not_found_run_resets_on_any_other_result() {
        // #367: only CONSECUTIVE definitive not-founds terminate — an interleaved
        // transport fault (or a real status) resets the run, so a flaky path can
        // never accumulate into a false Failed(NotFound) on a live swap.
        let not_found = || {
            Err(SwapError::ProviderProtocol {
                reason: ProviderProtocolReason::SwapNotFound,
            })
        };
        let mut script: Vec<Result<SwapStatus, SwapError>> = Vec::new();
        // one-below-threshold not-founds, then a TRANSPORT fault (resets the run)…
        script.extend((0..SWAP_NOT_FOUND_TERMINAL_POLLS - 1).map(|_| not_found()));
        script.push(Err(SwapError::ProviderUnavailable));
        // …then one-below-threshold again, then a REAL status (resets), then terminal.
        script.extend((0..SWAP_NOT_FOUND_TERMINAL_POLLS - 1).map(|_| not_found()));
        script.push(Ok(SwapStatus::Processing));
        script.push(Ok(SwapStatus::Success {
            out_txid: None,
            realized_slippage_bps: None,
        }));
        let expected_polls = script.len() as u32;
        let provider = Arc::new(PollProvider::new(script));
        let svc = Arc::new(poll_service(Arc::clone(&provider) as Arc<dyn SwapPort>));
        let got = Arc::new(Mutex::new(Vec::new()));
        run_swap_status_poll(
            Arc::clone(&svc.provider),
            svc.kill.subscribe(),
            SwapId::new("q-flaky"),
            "t1poll".to_owned(),
            RecordingSink {
                got: Arc::clone(&got),
                stop_after: None,
            },
        )
        .await;
        let got = got.lock().expect("got").clone();
        assert_eq!(
            got.len(),
            2,
            "no synthetic NotFound ever emitted — only the two real statuses"
        );
        assert!(matches!(got[0], SwapStatus::Processing));
        assert!(matches!(got[1], SwapStatus::Success { .. }));
        assert_eq!(
            provider.poll_count(),
            expected_polls,
            "the loop rode every fault to the REAL terminal",
        );
    }

    #[tokio::test(start_paused = true)]
    async fn swap_status_poll_stops_promptly_on_host_cancel() {
        // The §7 foreground-only stop: the host drops the subscription (sink → false)
        // and the loop ENDS — never an unbounded poll against a never-terminal swap.
        let provider = Arc::new(PollProvider::new(vec![])); // Processing forever
        let svc = Arc::new(poll_service(Arc::clone(&provider) as Arc<dyn SwapPort>));
        let got = Arc::new(Mutex::new(Vec::new()));
        run_swap_status_poll(
            Arc::clone(&svc.provider),
            svc.kill.subscribe(),
            SwapId::new("q-1"),
            "t1poll".to_owned(),
            RecordingSink {
                got: Arc::clone(&got),
                stop_after: Some(2), // cancel after the 2nd status
            },
        )
        .await;
        assert_eq!(got.lock().expect("got").len(), 2);
        assert_eq!(
            provider.poll_count(),
            2,
            "the loop stopped at host-cancel — no poll past it",
        );
    }

    #[tokio::test(start_paused = true)]
    async fn swap_status_poll_hard_kill_stops_in_flight_polling() {
        // §3.5: a Hard kill landing mid-stream STOPS in-flight polling — zero further
        // `status` traffic, and NO synthetic terminal (the host renders
        // tracking-unavailable from its own Hard state; funds settle/refund
        // provider-side regardless).
        let provider = Arc::new(PollProvider::new(vec![
            Ok(SwapStatus::Processing),
            // a 2nd status the loop must NEVER reach once Hard-killed
            Ok(SwapStatus::Success {
                out_txid: None,
                realized_slippage_bps: None,
            }),
        ]));
        let svc = Arc::new(poll_service(Arc::clone(&provider) as Arc<dyn SwapPort>));
        let got = Arc::new(Mutex::new(Vec::new()));
        let sink = KillAtSink {
            got: Arc::clone(&got),
            svc: Arc::clone(&svc),
            at: 0, // Hard-kill the service on the first emit
            severity: SwapKill::Hard,
        };
        run_swap_status_poll(
            Arc::clone(&svc.provider),
            svc.kill.subscribe(),
            SwapId::new("q-1"),
            "t1poll".to_owned(),
            sink,
        )
        .await;
        let got = got.lock().expect("got").clone();
        assert_eq!(
            got.len(),
            1,
            "only the pre-kill status emitted; no terminal"
        );
        assert!(!got[0].is_terminal());
        assert_eq!(
            provider.poll_count(),
            1,
            "the Hard kill stopped polling BEFORE any straggler `status` call",
        );
    }

    #[tokio::test(start_paused = true)]
    async fn swap_status_poll_winddown_keeps_polling_to_terminal() {
        // §3.5 the OTHER side of the divergence: WindDown refuses NEW work but keeps
        // polling in-flight swaps to terminal (status is observational) — so the same
        // mid-stream flip that Hard halts, WindDown rides to Success.
        let provider = Arc::new(PollProvider::new(vec![
            Ok(SwapStatus::Processing),
            Ok(SwapStatus::Success {
                out_txid: None,
                realized_slippage_bps: None,
            }),
        ]));
        let svc = Arc::new(poll_service(Arc::clone(&provider) as Arc<dyn SwapPort>));
        let got = Arc::new(Mutex::new(Vec::new()));
        let sink = KillAtSink {
            got: Arc::clone(&got),
            svc: Arc::clone(&svc),
            at: 0,
            severity: SwapKill::WindDown,
        };
        run_swap_status_poll(
            Arc::clone(&svc.provider),
            svc.kill.subscribe(),
            SwapId::new("q-1"),
            "t1poll".to_owned(),
            sink,
        )
        .await;
        let got = got.lock().expect("got").clone();
        assert_eq!(got.len(), 2, "WindDown did NOT stop the poll");
        assert!(
            got[1].is_terminal(),
            "it rode the in-flight swap to terminal"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn swap_status_poll_ends_when_the_wallet_tears_down() {
        // The kill `Sender` lives on the `SwapService`; when it (and the wallet) drop,
        // the loop's `changed()` returns `Err` — the watch_sync_status "Sender dropped
        // ⇒ teardown ⇒ end the stream" contract — NOT a spin re-polling a stale value.
        let (kill_tx, kill_rx) = watch::channel(SwapKill::Live);
        let provider = Arc::new(PollProvider::new(vec![Ok(SwapStatus::Processing)]));
        let got = Arc::new(Mutex::new(Vec::new()));
        let handle = tokio::spawn(run_swap_status_poll(
            Arc::clone(&provider) as Arc<dyn SwapPort>,
            kill_rx,
            SwapId::new("q-1"),
            "t1poll".to_owned(),
            RecordingSink {
                got: Arc::clone(&got),
                stop_after: None,
            },
        ));
        // Drop the only `Sender` BEFORE the spawned loop runs (the current task has not
        // yielded yet): its first poll emits `Processing`, then the between-poll
        // `changed()` finds the channel closed and ends — no second poll.
        drop(kill_tx);
        handle
            .await
            .expect("the poll task ends cleanly on teardown");
        assert_eq!(got.lock().expect("got").len(), 1);
        assert_eq!(
            provider.poll_count(),
            1,
            "exactly one poll, then teardown ended the stream — no spin re-poll",
        );
    }

    /// A [`SwapStatusSink`] over an mpsc channel — drives the `watch_status` spawn
    /// path end-to-end (the host receives each status off the channel; the sender
    /// drops when the spawned poll task ends).
    struct ChannelSink {
        tx: tokio::sync::mpsc::UnboundedSender<SwapStatus>,
    }
    impl SwapStatusSink for ChannelSink {
        fn emit(&mut self, status: SwapStatus) -> bool {
            self.tx.send(status).is_ok() // a closed channel (host gone) stops the poll
        }
    }

    #[tokio::test(start_paused = true)]
    async fn watch_status_spawns_a_detached_poll_that_streams_to_a_terminal() {
        // The PUBLIC surface (`SwapService::watch_status`) — not just the free
        // `run_swap_status_poll` — actually spawns a detached poll task that streams
        // through `provider.status` to terminal over a real channel sink.
        let provider = Arc::new(PollProvider::new(vec![
            Ok(SwapStatus::Processing),
            Ok(SwapStatus::Success {
                out_txid: None,
                realized_slippage_bps: None,
            }),
        ]));
        // S8: the pre-spawn resolution reads the provider handle off the home row the
        // execute leg wrote, so the surface needs one recorded before it will spawn.
        let records = Arc::new(MemSwapRecords::default());
        let svc = SwapService::new(
            Arc::clone(&provider) as Arc<dyn SwapPort>,
            Arc::new(StubRefunds),
            Arc::new(CountingDestinations::default()),
            Arc::new(RecordingDeposit::default()),
            Arc::new(MemIssuedQuoteStore::default()),
            records.clone(),
            Network::Main,
        );
        records
            .record_started(StartedSwap {
                id: SwapId::new("q-1"),
                provider_ref: "t1poll".into(),
                out_of_zec: true,
                deposit_deadline: None,
                expires_at_wall: 0,
                watch: None,
            })
            .await
            .expect("home row");
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        svc.watch_status(SwapId::new("q-1"), ChannelSink { tx })
            .await
            .expect("the handle resolved from the home row and the poll spawned");
        // Awaiting `recv()` lets the spawned task run (the poll sleeps auto-advance on
        // the paused clock); the sender drops at task end, closing the channel.
        let mut got = Vec::new();
        while let Some(s) = rx.recv().await {
            got.push(s);
        }
        assert_eq!(got.len(), 2, "the spawned poll streamed both statuses");
        assert!(got[1].is_terminal(), "and ran to the terminal");
        assert_eq!(
            provider.poll_count(),
            2,
            "exactly two polls via the spawn path"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn swap_poll_emits_a_5_4_clean_wallet_swap_poll_span() {
        // §5.4: the `wallet.swap_poll` span carries ONLY the provider NAME, a coarse
        // outcome code, and a transient-fault COUNT — NEVER the SwapId (it is the deposit
        // address), and NEVER the status payload (a Success's out_txid is a NEVER-log
        // txid). The script DELIBERATELY mixes a transient fault (→ faults=1) and a
        // terminal Success carrying a txid + slippage: the secret must reach the SINK but
        // never a tracing field, and the fault is observed as a COUNT, not a payload. ONE
        // capture test per callsite (the wallet.swap convention); `force_wallet_callsites_
        // enabled` keeps the callsite enabled under parallel capture.
        use crate::tracing_guard::{
            CaptureLayer, CapturedEvents, assert_5_4_clean, force_wallet_callsites_enabled,
        };
        use tracing_subscriber::prelude::*;
        force_wallet_callsites_enabled();
        let sink = CapturedEvents::default();
        let subscriber = tracing_subscriber::registry().with(CaptureLayer::new(sink.clone()));
        let _guard = tracing::subscriber::set_default(subscriber);

        let provider: Arc<dyn SwapPort> = Arc::new(PollProvider::new(vec![
            Err(SwapError::ProviderUnavailable), // a transient fault → faults=1
            Ok(SwapStatus::Success {
                out_txid: Some("deadbeeftxid".into()),
                realized_slippage_bps: Some(7),
            }),
        ]));
        let (_kill_tx, kill_rx) = watch::channel(SwapKill::Live);
        let got = Arc::new(Mutex::new(Vec::new()));
        run_swap_status_poll(
            provider,
            kill_rx,
            SwapId::new("q-1"),
            // S8: the provider handle is the never-log deposit address now
            "zs1depositaddressisthesecretid".to_owned(),
            RecordingSink {
                got: Arc::clone(&got),
                stop_after: None,
            },
        )
        .await;
        // The secret-bearing status reached the SINK (the host sees it) ...
        assert!(matches!(
            got.lock().expect("got").first(),
            Some(SwapStatus::Success { .. })
        ));
        // ... but the tracing surface is §5.4-clean: no txid, no id, no amount.
        let fields = sink.fields();
        assert_5_4_clean(&fields);
        let has = |n: &str, v: &str| fields.iter().any(|(fn_, fv)| fn_ == n && fv == v);
        assert!(
            has("provider", "poll-mock"),
            "the span fired (provider recorded)"
        );
        assert!(
            has("outcome", "terminal"),
            "the terminal outcome is a coarse code"
        );
        assert!(
            has("faults", "1"),
            "the transient fault is observed as a §5.4-clean count, not a payload",
        );
    }

    // ── W-swap-3-c-3-i real-world-edge round (building-bricks ∥ ZODL ∥ money/mobile) ──

    /// A provider that escalates the SERVICE kill to `Hard` on its FIRST `status`
    /// call — modelling a `set_kill(Hard)` landing WHILE a `provider.status()`
    /// network call is in flight (NOT after it, the `KillAtSink` case). The result
    /// of that in-flight call still resolves and emits (one straggler EMIT), but the
    /// next loop-top `stops_polling()` halts BEFORE any further `status` call.
    struct KillDuringStatusProvider {
        kill: watch::Sender<SwapKill>,
        results: Mutex<std::collections::VecDeque<Result<SwapStatus, SwapError>>>,
        polls: Mutex<u32>,
    }
    #[async_trait]
    impl SwapPort for KillDuringStatusProvider {
        fn name(&self) -> &'static str {
            "kill-during-status"
        }

        async fn list_tokens(&self) -> Result<TokenList, SwapError> {
            Err(SwapError::ProviderUnavailable) // not this double's concern (IZ-2)
        }
        async fn quote(&self, _req: QuoteRequest) -> Result<SwapQuote, SwapError> {
            Err(SwapError::ProviderUnavailable)
        }
        async fn execute(&self, _quote: &SwapQuote) -> Result<(), SwapError> {
            Ok(())
        }
        async fn status(&self, _id: &SwapId, _provider_ref: &str) -> Result<SwapStatus, SwapError> {
            let n = {
                let mut p = self.polls.lock().expect("polls poisoned");
                *p += 1;
                *p
            };
            if n == 1 {
                // the kill lands DURING this in-flight call
                self.kill.send_if_modified(|k| {
                    if SwapKill::Hard > *k {
                        *k = SwapKill::Hard;
                        true
                    } else {
                        false
                    }
                });
            }
            self.results
                .lock()
                .expect("results poisoned")
                .pop_front()
                .unwrap_or(Ok(SwapStatus::Processing))
        }
    }

    #[tokio::test(start_paused = true)]
    async fn hard_kill_during_an_in_flight_status_call_emits_then_polls_no_more() {
        // §3.5 "zero swap traffic from this instant" at its tightest boundary: a Hard
        // kill arriving WHILE `provider.status()` is awaiting. The in-flight result is
        // still delivered (one straggler emit — it already left the provider), but NO
        // second `status` call fires — the loop-top check halts first.
        let (kill_tx, kill_rx) = watch::channel(SwapKill::Live);
        let provider = Arc::new(KillDuringStatusProvider {
            kill: kill_tx,
            results: Mutex::new(vec![Ok(SwapStatus::Processing)].into()),
            polls: Mutex::new(0),
        });
        let got = Arc::new(Mutex::new(Vec::new()));
        run_swap_status_poll(
            Arc::clone(&provider) as Arc<dyn SwapPort>,
            kill_rx,
            SwapId::new("q-1"),
            "t1poll".to_owned(),
            RecordingSink {
                got: Arc::clone(&got),
                stop_after: None,
            },
        )
        .await;
        let got = got.lock().expect("got").clone();
        assert_eq!(
            got.len(),
            1,
            "the in-flight result emitted (one straggler emit)"
        );
        assert!(
            !got[0].is_terminal(),
            "and it was non-terminal — the Hard halted us"
        );
        assert_eq!(
            *provider.polls.lock().expect("polls"),
            1,
            "NO straggler poll — the Hard that landed mid-call stopped the next one",
        );
    }

    /// A provider that records the VIRTUAL time of each `status` call, so a test can
    /// assert the inter-poll cadence is the §7 `next_poll_backoff` sequence — proving
    /// it is WIRED into the loop's `select!`, not merely correct in isolation.
    #[derive(Default)]
    struct TimingProvider {
        at: Mutex<Vec<tokio::time::Instant>>,
    }
    #[async_trait]
    impl SwapPort for TimingProvider {
        fn name(&self) -> &'static str {
            "timing"
        }

        async fn list_tokens(&self) -> Result<TokenList, SwapError> {
            Err(SwapError::ProviderUnavailable) // not this double's concern (IZ-2)
        }
        async fn quote(&self, _req: QuoteRequest) -> Result<SwapQuote, SwapError> {
            Err(SwapError::ProviderUnavailable)
        }
        async fn execute(&self, _quote: &SwapQuote) -> Result<(), SwapError> {
            Ok(())
        }
        async fn status(&self, _id: &SwapId, _provider_ref: &str) -> Result<SwapStatus, SwapError> {
            self.at
                .lock()
                .expect("at poisoned")
                .push(tokio::time::Instant::now());
            Ok(SwapStatus::Processing) // never terminal — the host cancels after N polls
        }
    }

    #[tokio::test(start_paused = true)]
    async fn poll_cadence_follows_the_5_to_60_backoff_inside_the_loop() {
        // The §7 cadence as the LOOP runs it (not just the pure `next_poll_backoff`):
        // first poll immediate, then 5 → 10 → 20 → 40 → 60s gaps. Drives a non-terminal
        // `Processing` run on the paused clock (auto-advanced through each sleep) and
        // host-cancels after 6 polls.
        let provider = Arc::new(TimingProvider::default());
        let svc = Arc::new(poll_service(Arc::clone(&provider) as Arc<dyn SwapPort>));
        let got = Arc::new(Mutex::new(Vec::new()));
        run_swap_status_poll(
            Arc::clone(&svc.provider),
            svc.kill.subscribe(),
            SwapId::new("q-1"),
            "t1poll".to_owned(),
            RecordingSink {
                got: Arc::clone(&got),
                stop_after: Some(6),
            },
        )
        .await;
        let at = provider.at.lock().expect("at");
        assert_eq!(at.len(), 6, "six polls before host-cancel");
        let gaps: Vec<u64> = at.windows(2).map(|w| (w[1] - w[0]).as_secs()).collect();
        assert_eq!(
            gaps,
            vec![5, 10, 20, 40, 60],
            "the loop wires next_poll_backoff into the select — doubling, capped at 60",
        );
    }

    #[tokio::test(start_paused = true)]
    async fn underdeposited_keeps_the_stream_alive_until_a_terminal_refund() {
        // The money path that MOTIVATED making `UnderDeposited` non-terminal (§1.7): a
        // partial deposit must keep tracking through the top-up-or-refund window, not
        // EOF early. Here two `UnderDeposited` frames ride to a terminal `Refunded` (a
        // NAMED refund — terminal, not a loss).
        let provider = Arc::new(PollProvider::new(vec![
            Ok(SwapStatus::UnderDeposited {
                received: "0.4".into(),
                missing: "0.6".into(),
                deadline: 9,
            }),
            Ok(SwapStatus::UnderDeposited {
                received: "0.7".into(),
                missing: "0.3".into(),
                deadline: 9,
            }),
            Ok(SwapStatus::Refunded {
                refund_txid: Some("rfnd".into()),
            }),
        ]));
        let svc = Arc::new(poll_service(Arc::clone(&provider) as Arc<dyn SwapPort>));
        let got = Arc::new(Mutex::new(Vec::new()));
        run_swap_status_poll(
            Arc::clone(&svc.provider),
            svc.kill.subscribe(),
            SwapId::new("q-1"),
            "t1poll".to_owned(),
            RecordingSink {
                got: Arc::clone(&got),
                stop_after: None,
            },
        )
        .await;
        let got = got.lock().expect("got").clone();
        assert_eq!(got.len(), 3, "both partials AND the refund flowed");
        assert!(
            matches!(got[0], SwapStatus::UnderDeposited { .. }),
            "UnderDeposited did NOT end the stream",
        );
        assert!(got[2].is_terminal(), "only the Refunded terminal ended it");
        assert_eq!(provider.poll_count(), 3, "polled through the top-up window");
    }

    #[tokio::test(start_paused = true)]
    async fn a_provider_reported_failed_is_terminal_and_ends_the_stream() {
        // Posture pin (security re-review): a provider-reported `Failed` IS terminal,
        // INCLUDING the adapter's unknown/malformed-frame → `ProviderProtocol` mapping.
        // So one such frame mid-`Processing` ends the stream. This is the CURRENT money
        // posture, locked by a named test. OWED to the adapter slice (an ADR note): the
        // 1Click adapter should distinguish an EXPLICIT provider failure from a single
        // unparseable frame and ride the latter as transient — the loop is the wrong
        // layer for that call (it correctly honors `is_terminal`).
        let provider = Arc::new(PollProvider::new(vec![
            Ok(SwapStatus::Processing),
            Ok(SwapStatus::Failed {
                code: SwapFailureCode::ProviderProtocol,
            }),
        ]));
        let svc = Arc::new(poll_service(Arc::clone(&provider) as Arc<dyn SwapPort>));
        let got = Arc::new(Mutex::new(Vec::new()));
        run_swap_status_poll(
            Arc::clone(&svc.provider),
            svc.kill.subscribe(),
            SwapId::new("q-1"),
            "t1poll".to_owned(),
            RecordingSink {
                got: Arc::clone(&got),
                stop_after: None,
            },
        )
        .await;
        let got = got.lock().expect("got").clone();
        assert_eq!(got.len(), 2, "Processing then the terminal Failed");
        assert!(
            got[1].is_terminal(),
            "a provider-reported Failed is terminal"
        );
        assert_eq!(provider.poll_count(), 2, "no poll past the terminal Failed");
    }

    proptest::proptest! {
        #[test]
        fn decimal_parser_never_panics(s in ".{0,64}") {
            let _ = parse_decimal(&s);
        }

        #[test]
        fn decimal_bps_check_never_panics(
            a in "[0-9]{0,27}(\\.[0-9]{0,16})?",
            b in "[0-9]{0,27}(\\.[0-9]{0,16})?",
            bps in 0u16..=10_000,
        ) {
            // the full hostile cross-product of in-cap decimals at mixed
            // scales: bound-checking must never panic (W2 review fold —
            // the old proptest covered only the parser)
            if let (Some(x), Some(y)) = (parse_decimal(&a), parse_decimal(&b)) {
                let _ = decimal_within_bps(x, y, bps);
            }
        }
    }

    // ── RED BY RULING — the 2026-09-20 production-readiness review ────
    // R01, `docs/plan/audit-2026-09-20-remediation.md` §2a/§2b: the body is the
    // review's probe (`docs/reviews/2026-09-20/probes/swap_service.rs`),
    // verbatim; the name is this project's; `evals/standing_reds.txt` carries
    // it. Appended at the end of the module so no cited line above moves.

    /// R01 — `SwapId::new(c.deposit_address)`: the execution identity is the
    /// PROVIDER's deposit address, so a provider that reuses it across two
    /// quotes makes `execute(&first)` pay the SECOND quote's amount (900,000
    /// zat under a 100,000 approval, re-run). Green only with an
    /// SDK-minted immutable execution identity that the approval binds to
    /// (§2b R01) — duplicate-ID rejection alone is not the guard.
    #[tokio::test]
    async fn a_provider_reusing_a_quote_id_cannot_change_the_amount_the_user_accepted() {
        let mock = MockSwapProvider::with_quote_fn(|request| {
            let ExactSide::In(SwapAmount::Zec(amount)) = &request.exact else {
                unreachable!()
            };
            Ok(honest_quote(request, amount.zat(), 3600))
        });
        let (service, spy) = service_recording(Arc::new(mock));
        let first = service.quote(out_of_zec_request(100_000)).await.unwrap();
        let second = service.quote(out_of_zec_request(900_000)).await.unwrap();
        assert_eq!(
            first.deposit_address, second.deposit_address,
            "provider reused its deposit address"
        );
        service.execute(&first).await.unwrap();
        let deposits = spy.deposits.lock().unwrap();
        assert_eq!(deposits.len(), 1, "one real deposit-port invocation");
        assert_eq!(
            deposits[0].1, first.zec_side,
            "the deposit must equal the earlier quote the user accepted"
        );
    }

    // ── S8 `identity` (R01) — the test author's rows, appended after the probe ────
    //
    // Contract: `docs/plan/stage-8-payment-identity-and-durable-retry.md` §3.1, written
    // blind (IT-2a) against `dc57cb2d`; every name below is the contract's. The premise
    // under test: the execution identity of a swap is minted by the SDK when the quote is
    // issued and is the ONE key the record is stored, approved, consumed, homed and polled
    // by; the provider's deposit address is DATA inside that record. The fixtures choose
    // their provider addresses PER TEST and the provider double RECORDS what it is polled
    // with — the seam `honest_quote`'s hardcoded `q-1` + fixed address hid from every row
    // above. Appended here so no cited line above moves.

    /// A checksum-valid Main transparent address from one seed byte — `0x22` is
    /// `honest_quote`'s; two seeds are two provider addresses.
    fn t_addr(seed: u8) -> String {
        use zcash_address::ToAddress;
        use zcash_protocol::consensus::NetworkType;
        zcash_address::ZcashAddress::from_transparent_p2pkh(NetworkType::Main, [seed; 20]).encode()
    }

    /// The ZEC side rendered the way a provider quotes it (8 decimals) — so two quotes
    /// with different ZEC terms also differ on the foreign-side string an approval shows.
    fn zec_decimal(zat: i64) -> String {
        format!("{}.{:08}", zat / 100_000_000, zat % 100_000_000)
    }

    /// An honest provider echo whose deposit ADDRESS the test chooses. The provider-side
    /// `id` is the address itself — what the shipped 1Click adapter mints (`map.rs:437`),
    /// so a reused address IS a reused provider id (the R01 shape).
    fn quote_at(req: &QuoteRequest, zec_side: i64, expires_in: u64, seed: u8) -> SwapQuote {
        let deposit = t_addr(seed);
        SwapQuote {
            id: SwapId::new(deposit.clone()),
            deposit_address: deposit,
            deposit_memo: None,
            expires_at: now_unix() + expires_in,
            amount_in: "1.0".into(),
            min_amount_out: zec_decimal(zec_side),
            zec_side: Zatoshis::new(zec_side).expect("valid"),
            refund_to: req.refund_address.clone(),
            disclosure: disclosure(),
            binding: None,
        }
    }

    /// The provider double the S8 rows need: quotes answered from a per-test SCRIPT of
    /// `(zec_side, address seed)` — one address can serve two quotes, or two quotes two —
    /// with the request's refund echoed (the `validate_quote` echo check); every DTO an
    /// execute presented is recorded; and every status poll RECORDS THE ARGUMENT IT
    /// CARRIED (`polled`) — the observation row 4 needs and no existing double keeps.
    /// Statuses are served FIFO (empty ⇒ `Processing`, a swap still draining).
    ///
    /// COUPLING DECLARED FOR THE JOIN (the port shape is the adjudicator's ruling): at the
    /// base commit `status` has one argument, the `SwapId`, and `polled` records it. Once
    /// the adapter is handed the provider address as DATA, `polled` records THAT argument
    /// — the value the request URL is built from — whatever the port's spelling.
    struct RecordingProvider {
        script: Mutex<std::collections::VecDeque<(i64, u8)>>,
        executed: Mutex<Vec<SwapQuote>>,
        polled: Mutex<Vec<String>>,
        statuses: Mutex<std::collections::VecDeque<Result<SwapStatus, SwapError>>>,
    }
    impl RecordingProvider {
        fn scripted(script: &[(i64, u8)]) -> Arc<Self> {
            Arc::new(Self {
                script: Mutex::new(script.iter().copied().collect()),
                executed: Mutex::new(Vec::new()),
                polled: Mutex::new(Vec::new()),
                statuses: Mutex::new(std::collections::VecDeque::new()),
            })
        }
        fn push_status(&self, status: Result<SwapStatus, SwapError>) {
            self.statuses
                .lock()
                .expect("statuses poisoned")
                .push_back(status);
        }
        fn executed(&self) -> Vec<SwapQuote> {
            self.executed.lock().expect("executed poisoned").clone()
        }
        fn polled(&self) -> Vec<String> {
            self.polled.lock().expect("polled poisoned").clone()
        }
    }
    #[async_trait]
    impl SwapPort for RecordingProvider {
        fn name(&self) -> &'static str {
            "recording"
        }
        async fn quote(&self, req: QuoteRequest) -> Result<SwapQuote, SwapError> {
            let (zec_side, seed) = self
                .script
                .lock()
                .expect("script poisoned")
                .pop_front()
                .expect("recording provider: more quotes asked than scripted");
            Ok(quote_at(&req, zec_side, 3_600, seed))
        }
        async fn execute(&self, quote: &SwapQuote) -> Result<(), SwapError> {
            self.executed
                .lock()
                .expect("executed poisoned")
                .push(quote.clone());
            Ok(())
        }
        async fn status(&self, _id: &SwapId, provider_ref: &str) -> Result<SwapStatus, SwapError> {
            // the ruled port shape (both values): `polled` records the PROVIDER-ADDRESS
            // argument — the value the request URL is built from
            self.polled
                .lock()
                .expect("polled poisoned")
                .push(provider_ref.to_owned());
            self.statuses
                .lock()
                .expect("statuses poisoned")
                .pop_front()
                .unwrap_or(Ok(SwapStatus::Processing))
        }
        async fn list_tokens(&self) -> Result<TokenList, SwapError> {
            Err(SwapError::ProviderUnavailable) // not this double's concern
        }
    }

    /// A refund source minting a DISTINCT t-addr at an increasing single-use index per
    /// call — `StubRefunds` hands out ONE fixed address, which cannot show two refund
    /// watches for two swaps (row 4). Seeds from `0x41` so no refund collides with a
    /// deposit address (`0x22`/`0x33`).
    #[derive(Default)]
    struct CountingRefunds {
        minted: Mutex<u32>,
    }
    #[async_trait]
    impl RefundAddressSource for CountingRefunds {
        async fn fresh_refund_address(&self) -> Result<IssuedRefund, SwapError> {
            let mut n = self.minted.lock().expect("minted poisoned");
            *n += 1;
            let index = *n;
            Ok(IssuedRefund {
                address: t_addr(0x40 + u8::try_from(index).expect("small index")),
                index,
            })
        }
    }

    /// The doubles an S8 row inspects, held apart from the service so a "reopen" can
    /// rebuild the service over the SAME durable doubles (the issued store, the home rows)
    /// and the SAME single-use counters, with a FRESH deposit spy and an EMPTY in-memory
    /// registry — a process restart at the service seam.
    struct IdentityBed {
        provider: Arc<RecordingProvider>,
        store: Arc<MemIssuedQuoteStore>,
        records: Arc<MemSwapRecords>,
        refunds: Arc<CountingRefunds>,
        destinations: Arc<CountingDestinations>,
    }
    impl IdentityBed {
        fn new(provider: Arc<RecordingProvider>) -> Self {
            Self {
                provider,
                store: Arc::new(MemIssuedQuoteStore::default()),
                records: Arc::new(MemSwapRecords::default()),
                refunds: Arc::new(CountingRefunds::default()),
                destinations: Arc::new(CountingDestinations::default()),
            }
        }
        /// A service over the bed — call it again after dropping the first for
        /// "quote, die, reopen".
        fn open(&self) -> (SwapService, Arc<RecordingDeposit>) {
            let deposits = Arc::new(RecordingDeposit::default());
            let svc = SwapService::new(
                Arc::clone(&self.provider) as Arc<dyn SwapPort>,
                Arc::clone(&self.refunds) as Arc<dyn RefundAddressSource>,
                Arc::clone(&self.destinations) as Arc<dyn DestinationAddressSource>,
                Arc::clone(&deposits) as Arc<dyn DepositSender>,
                Arc::clone(&self.store) as Arc<dyn IssuedQuoteStore>,
                Arc::clone(&self.records) as Arc<dyn SwapRecordSink>,
                Network::Main,
            );
            (svc, deposits)
        }
    }

    /// The deposit port's record as `(address, zat, deadline)` — asserted on the address
    /// AND the amount every time (row 1: never on the count alone).
    fn deposit_terms(deposits: &RecordingDeposit) -> Vec<(Address, i64, u64)> {
        deposits
            .deposits
            .lock()
            .expect("deposits poisoned")
            .iter()
            .map(|(a, z, d)| (a.clone(), z.zat(), *d))
            .collect()
    }

    fn parsed(addr: &str) -> Address {
        Address::parse(addr, Network::Main).expect("a Main transparent address")
    }

    /// True for every `SwapError` variant that exists at the base commit. A "terms differ"
    /// refusal is a NEW typed variant (contract §3.1, host-visible surface — its copy
    /// reaches Relim users), so a refusal that matches here came from the wrong mechanism
    /// (an expiry, a store fault, a request-shape check), not from the terms compare.
    fn is_a_pre_s8_variant(e: &SwapError) -> bool {
        matches!(
            e,
            SwapError::SlippageToleranceTooHigh { .. }
                | SwapError::QuoteOutOfBounds { .. }
                | SwapError::QuoteExpired
                | SwapError::DestinationInvalid { .. }
                | SwapError::RequestInvalid { .. }
                | SwapError::ProviderUnavailable
                | SwapError::ProviderProtocol { .. }
                | SwapError::SwapDisabled
                | SwapError::DepositSendFailed
                | SwapError::RefundAddressUnavailable
                | SwapError::DestinationAddressUnavailable
                | SwapError::SwapStateUnavailable
                | SwapError::SwapStateBusy
                | SwapError::SwapAlreadyInFlight
                | SwapError::WatchOnly
        )
    }

    /// Row 2's refusal shape, asserted whole: a typed error that is none of the pre-S8
    /// variants, and NOTHING happened — no deposit-port call and no provider execute
    /// beyond the counts before the call.
    fn assert_refused_before_any_side_effect(
        result: Result<SwapId, SwapError>,
        deposits: &RecordingDeposit,
        provider: &RecordingProvider,
        deposits_before: usize,
        executes_before: usize,
        what: &str,
    ) {
        let err = result.expect_err(&format!("{what}: a DTO whose terms differ is refused"));
        assert!(
            !is_a_pre_s8_variant(&err),
            "{what}: refused by a pre-S8 variant ({}) — not the terms compare",
            err.code()
        );
        assert_eq!(
            deposit_terms(deposits).len(),
            deposits_before,
            "{what}: refused BEFORE any deposit-port call"
        );
        assert_eq!(
            provider.executed().len(),
            executes_before,
            "{what}: the provider was never asked to register intent"
        );
    }

    /// Row 1 — THE PROBE, BOTH WAYS. A provider answering two quotes (100,000 then
    /// 900,000 zat) under ONE deposit address: `execute(&first)` deposits 100,000 and
    /// `execute(&second)` deposits 900,000 — each DTO pays its OWN terms, asserted on
    /// (address, amount) at the deposit port. Reddening mutation: key the record by the
    /// provider address (the base commit — the second execute finds its claim gone, and
    /// the first pays the second's amount).
    #[tokio::test]
    async fn a_provider_reusing_a_deposit_address_pays_each_dto_its_own_terms_both_ways() {
        let bed = IdentityBed::new(RecordingProvider::scripted(&[
            (100_000, 0x22),
            (900_000, 0x22),
        ]));
        let (svc, deposits) = bed.open();
        let first = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("first quote");
        let second = svc
            .quote(out_of_zec_request(900_000))
            .await
            .expect("second quote");
        assert_eq!(
            first.deposit_address, second.deposit_address,
            "the provider reused its deposit address"
        );
        assert_ne!(
            first.id, second.id,
            "two quotes under one provider address are two SDK identities"
        );
        svc.execute(&first)
            .await
            .expect("the first DTO executes its own terms");
        svc.execute(&second)
            .await
            .expect("the second DTO executes its own terms");
        let terms = deposit_terms(&deposits);
        assert_eq!(terms.len(), 2, "one deposit per DTO");
        assert_eq!(
            (&terms[0].0, terms[0].1),
            (&parsed(&first.deposit_address), 100_000),
            "the first deposit is the first approval's (address, amount)"
        );
        assert_eq!(
            (&terms[1].0, terms[1].1),
            (&parsed(&second.deposit_address), 900_000),
            "the second deposit is the second approval's (address, amount)"
        );
    }

    /// Row 2 — CHANGED TERMS REFUSED. A caller's DTO whose amount, deposit address or
    /// refund differ from the record it names is refused BEFORE any deposit-port call with
    /// a typed error, and the refusal does NOT consume the record: the honest DTO still
    /// executes afterwards, paying the RECORD's terms. Two unlisted cases are planted at
    /// the end (IT-1 +A): a DTO carrying ANOTHER quote's spend binding, and one carrying a
    /// `None` binding — the record compares field by field, and the binding is a field.
    /// Reddening mutation: skip the compare (the base commit pays the record's terms
    /// under a DTO that shows the user something else).
    #[tokio::test]
    async fn a_caller_dto_whose_terms_differ_from_its_record_executes_nothing() {
        let bed = IdentityBed::new(RecordingProvider::scripted(&[
            (100_000, 0x22),
            (100_000, 0x23),
            (100_000, 0x24),
            (100_000, 0x25),
            (100_000, 0x26),
        ]));
        let (svc, deposits) = bed.open();
        let provider = &bed.provider;
        let mut paid = 0usize;
        type Alter = Box<dyn Fn(&SwapQuote) -> SwapQuote>;
        let cases: [(&str, Alter); 3] = [
            (
                "amount",
                Box::new(|q| SwapQuote {
                    zec_side: Zatoshis::new(q.zec_side.zat() + 1).expect("valid"),
                    ..q.clone()
                }),
            ),
            (
                "deposit address",
                Box::new(|q| SwapQuote {
                    deposit_address: t_addr(0x99),
                    ..q.clone()
                }),
            ),
            (
                "refund",
                Box::new(|q| SwapQuote {
                    refund_to: Some(t_addr(0x98)),
                    ..q.clone()
                }),
            ),
        ];
        for (what, alter) in cases {
            let honest = svc
                .quote(out_of_zec_request(100_000))
                .await
                .expect("quote issues");
            let altered = alter(&honest);
            assert_refused_before_any_side_effect(
                svc.execute(&altered).await,
                &deposits,
                provider,
                paid,
                paid,
                what,
            );
            svc.execute(&honest)
                .await
                .unwrap_or_else(|e| panic!("{what}: the refusal did not consume the record — the honest DTO still executes, got {}", e.code()));
            paid += 1;
            let terms = deposit_terms(&deposits);
            assert_eq!(
                terms.len(),
                paid,
                "{what}: exactly one deposit for the honest DTO"
            );
            assert_eq!(
                (&terms[paid - 1].0, terms[paid - 1].1),
                (&parsed(&honest.deposit_address), 100_000),
                "{what}: the honest execute pays the record's (address, amount)"
            );
        }
        // UNLISTED (IT-1 +A): the binding is a term too. The host records the DTO's binding
        // at its authorize bracket; a DTO carrying a binding the record never issued (or none)
        // is a DTO the approval cannot have been about. The miss this predicts: a compare
        // that stops at amount / address / refund / deadline.
        let other = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("another quote, for its binding");
        let honest = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("quote issues");
        assert_refused_before_any_side_effect(
            svc.execute(&SwapQuote {
                binding: other.binding,
                ..honest.clone()
            })
            .await,
            &deposits,
            provider,
            paid,
            paid,
            "another quote's binding",
        );
        assert_refused_before_any_side_effect(
            svc.execute(&SwapQuote {
                binding: None,
                ..honest.clone()
            })
            .await,
            &deposits,
            provider,
            paid,
            paid,
            "a missing binding",
        );
        svc.execute(&honest)
            .await
            .expect("the honest DTO still executes after both binding refusals");
        assert_eq!(deposit_terms(&deposits).len(), paid + 1);
    }

    /// Row 2's deadline clause — THE DEADLINE IS COMPARED, as
    /// `record.expires_at_wall − margin(direction)` against the DTO's `expires_at` (the
    /// row says which: compared, because the deadline shown at approval is one of the
    /// terms the Behaviour paragraph lists). Beside it, the assertion that prices the
    /// mutant a literal equality would be: an HONEST execute passes end to end in BOTH
    /// directions, and the deadline the wallet then acts on is the DTO's plus exactly the
    /// direction's margin (`DEPOSIT_EXECUTE_MARGIN_SECS` out of ZEC,
    /// `DEADLINE_SAFETY_MARGIN_SECS` into ZEC). An off-by-one DTO is refused with nothing
    /// done. Reddening mutations: `dto.expires_at == record.expires_at_wall` literally (the
    /// fail-closed outage — every honest execute refused); no compare at all (the base
    /// commit executes the off-by-one DTO).
    #[tokio::test]
    async fn an_honest_execute_passes_the_deadline_compare_by_its_margin() {
        let bed = IdentityBed::new(RecordingProvider::scripted(&[
            (100_000, 0x22),
            (100_000, 0x23),
            (100_000, 0x33),
            (100_000, 0x34),
        ]));
        let (svc, deposits) = bed.open();
        let provider = &bed.provider;

        // Out of ZEC: honest passes; the deposit rides the DTO deadline + the deposit margin.
        let out = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("out-of-zec quote");
        svc.execute(&out)
            .await
            .expect("an honest out-of-zec execute passes the deadline compare");
        let terms = deposit_terms(&deposits);
        assert_eq!(terms.len(), 1);
        assert_eq!(
            terms[0].2,
            out.expires_at + DEPOSIT_EXECUTE_MARGIN_SECS,
            "the deposit deadline is the approval's deadline plus exactly the deposit margin"
        );
        // Out of ZEC: one second off in either direction is refused, nothing done; the
        // record survives the refusal (the honest DTO executes afterwards).
        let mut paid = 1usize;
        for (what, delta) in [
            ("out-of-zec deadline +1", 1),
            ("out-of-zec deadline −1", -1),
        ] {
            let honest = svc
                .quote(out_of_zec_request(100_000))
                .await
                .expect("quote issues");
            let altered = SwapQuote {
                expires_at: honest
                    .expires_at
                    .checked_add_signed(delta)
                    .expect("in range"),
                ..honest.clone()
            };
            assert_refused_before_any_side_effect(
                svc.execute(&altered).await,
                &deposits,
                provider,
                paid,
                paid,
                what,
            );
            svc.execute(&honest)
                .await
                .expect("the honest out-of-zec DTO executes after the refusal");
            paid += 1;
            assert_eq!(deposit_terms(&deposits).len(), paid);
        }

        // Into ZEC: honest passes; the home row's deposit window is the DTO deadline + the
        // safety margin (no wallet-side deposit).
        let bed2 = IdentityBed::new(RecordingProvider::scripted(&[
            (100_000, 0x33),
            (100_000, 0x34),
        ]));
        let (svc2, deposits2) = bed2.open();
        let into = svc2
            .quote(into_zec_request())
            .await
            .expect("into-zec quote");
        svc2.execute(&into)
            .await
            .expect("an honest into-zec execute passes the deadline compare");
        let rows = bed2.records.rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].deposit_deadline,
            Some(into.expires_at + DEADLINE_SAFETY_MARGIN_SECS),
            "the into-zec deposit window is the approval's deadline plus exactly the safety margin"
        );
        let honest = svc2.quote(into_zec_request()).await.expect("quote issues");
        assert_refused_before_any_side_effect(
            svc2.execute(&SwapQuote {
                expires_at: honest.expires_at + 1,
                ..honest.clone()
            })
            .await,
            &deposits2,
            &bed2.provider,
            0,
            1,
            "into-zec deadline +1",
        );
        svc2.execute(&honest)
            .await
            .expect("the honest into-zec DTO executes after the refusal");
        assert_eq!(bed2.records.rows().len(), 2);
    }

    /// Row 3 — INTO ZEC TOO. A provider reusing ONE source-chain deposit address across
    /// two IntoZec quotes (100,000 then 900,000 zat to receive) cannot make the wallet
    /// present quote #1's terms for quote #2's order: the two are two records, the
    /// second's order is registered with the second DTO and homed with the SECOND
    /// destination's watch, and — after a reopen, where only the durable record can
    /// answer — a DTO carrying #2's ZEC side or #2's foreign side under #1's identity is
    /// refused while the honest #1 executes with #1's watch. Reddening mutations: the base
    /// keying (the second execute homes #1's destination); an IntoZec record that keeps no
    /// terms (the altered DTO executes after the reopen).
    #[tokio::test]
    async fn an_into_zec_quote_records_the_terms_the_approval_showed() {
        let bed = IdentityBed::new(RecordingProvider::scripted(&[
            (100_000, 0x33),
            (900_000, 0x33),
        ]));
        let (svc, deposits) = bed.open();
        let first = svc
            .quote(into_zec_request())
            .await
            .expect("first into-zec quote");
        let second = svc
            .quote(into_zec_request())
            .await
            .expect("second into-zec quote");
        assert_eq!(first.deposit_address, second.deposit_address);
        assert_ne!(
            first.id, second.id,
            "two IntoZec quotes under one address are two records"
        );
        assert_ne!(first.zec_side, second.zec_side);
        assert_ne!(first.min_amount_out, second.min_amount_out);
        svc.execute(&second)
            .await
            .expect("the second order executes");
        assert_eq!(
            bed.provider.executed(),
            vec![second.clone()],
            "the provider registered the SECOND DTO, terms and all"
        );
        let rows = bed.records.rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, second.id, "homed under the second SDK identity");
        assert!(!rows[0].out_of_zec);
        assert_eq!(
            rows[0]
                .watch
                .as_ref()
                .map(|w| (w.address.as_str(), w.index)),
            Some(("u1dest2", 2)),
            "the SECOND quote's destination is the watched leg, not the first's"
        );
        assert!(
            deposit_terms(&deposits).is_empty(),
            "no wallet-side deposit into ZEC"
        );
        drop(svc);

        // Reopen: the in-memory registry is gone; only the durable record can say what #1's
        // approval showed. #2's ZEC side or foreign side under #1's identity executes nothing.
        let (svc, deposits) = bed.open();
        assert_refused_before_any_side_effect(
            svc.execute(&SwapQuote {
                zec_side: second.zec_side,
                ..first.clone()
            })
            .await,
            &deposits,
            &bed.provider,
            0,
            1,
            "another quote's ZEC side under this identity",
        );
        assert_refused_before_any_side_effect(
            svc.execute(&SwapQuote {
                min_amount_out: second.min_amount_out.clone(),
                ..first.clone()
            })
            .await,
            &deposits,
            &bed.provider,
            0,
            1,
            "another quote's foreign side under this identity",
        );
        svc.execute(&first)
            .await
            .expect("the honest first DTO executes after the reopen");
        let rows = bed.records.rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].id, first.id);
        assert_eq!(
            rows[1]
                .watch
                .as_ref()
                .map(|w| (w.address.as_str(), w.index)),
            Some(("u1dest1", 1)),
            "the FIRST quote's destination is the watched leg of the first order"
        );
        assert_eq!(bed.provider.executed()[1], first);
    }

    /// Row 4 — TWO SWAPS UNDER ONE ADDRESS ARE TWO SWAPS. `MAX_ISSUED_QUOTES` counts
    /// RECORDS (sixteen quotes under one provider address all issue; the seventeenth is
    /// the typed cap refusal); the first two execute their own terms; two SDK ids ⇒ two
    /// home rows and two ARMED REFUND WATCHES at two single-use addresses. (The two
    /// DISTINCT isolation keys for one address are the adapter's — pinned in
    /// `zec-wallet-swap-near`'s `provider_tests`, against the adapter's port shape.)
    /// Reddening mutation: the base keying (the cap counts one, the second execute finds
    /// no claim, one home row).
    #[tokio::test]
    async fn two_swaps_under_one_provider_address_are_two_records_two_keys_two_watches() {
        let script = vec![(100_000, 0x22); MAX_ISSUED_QUOTES + 1];
        let bed = IdentityBed::new(RecordingProvider::scripted(&script));
        let (svc, deposits) = bed.open();
        let mut quotes = Vec::new();
        for i in 0..MAX_ISSUED_QUOTES {
            quotes.push(
                svc.quote(out_of_zec_request(100_000))
                    .await
                    .unwrap_or_else(|e| panic!("quote {i} under the cap issues, got {}", e.code())),
            );
        }
        assert_eq!(
            bed.store.count(),
            MAX_ISSUED_QUOTES,
            "the cap counts RECORDS — sixteen quotes under one address are sixteen"
        );
        let ids: std::collections::BTreeSet<&str> = quotes.iter().map(|q| q.id.as_str()).collect();
        assert_eq!(
            ids.len(),
            MAX_ISSUED_QUOTES,
            "sixteen distinct SDK identities"
        );
        assert!(
            !ids.contains(quotes[0].deposit_address.as_str()),
            "no SDK identity is the provider address"
        );
        let over = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect_err("the (cap + 1)-th quote is refused");
        assert!(
            matches!(over, SwapError::RequestInvalid { .. }),
            "the cap refusal is the typed cap message, got {}",
            over.code()
        );

        svc.execute(&quotes[0]).await.expect("first executes");
        svc.execute(&quotes[1]).await.expect("second executes");
        let terms = deposit_terms(&deposits);
        assert_eq!(terms.len(), 2);
        for t in &terms {
            assert_eq!((&t.0, t.1), (&parsed(&quotes[0].deposit_address), 100_000));
        }
        let rows = bed.records.rows();
        assert_eq!(rows.len(), 2, "two home rows");
        assert_eq!(
            (rows[0].id.clone(), rows[1].id.clone()),
            (quotes[0].id.clone(), quotes[1].id.clone()),
            "each homed under its own SDK identity"
        );
        let watches: Vec<(String, u32)> = rows
            .iter()
            .map(|r| {
                let w = r
                    .watch
                    .as_ref()
                    .expect("an out-of-zec row arms its refund watch");
                (w.address.clone(), w.index)
            })
            .collect();
        assert_eq!(
            watches,
            vec![(t_addr(0x41), 1), (t_addr(0x42), 2)],
            "two armed refund watches at two single-use refund addresses"
        );
    }

    /// Row 4's request leg — THE STATUS REQUEST STILL CARRIES THE PROVIDER ADDRESS. The
    /// handle `execute` returns is the quote's SDK identity and is NOT the provider
    /// address; polling by that handle reaches the provider with the ADDRESS (the value
    /// its request URL is built from), never with the SDK handle in the address position.
    /// Asserted on a provider double that records the argument it was polled with — the
    /// adapter-tier URL test is fed its own id and cannot see the service pass the wrong
    /// value. Reddening mutations: the base keying (the handle IS the address); passing
    /// the SDK id in the address position.
    #[tokio::test(start_paused = true)]
    async fn the_status_request_carries_the_provider_address_not_the_sdk_handle() {
        let provider = RecordingProvider::scripted(&[(100_000, 0x22)]);
        provider.push_status(Ok(SwapStatus::Processing));
        provider.push_status(Ok(SwapStatus::Success {
            out_txid: None,
            realized_slippage_bps: None,
        }));
        let bed = IdentityBed::new(Arc::clone(&provider));
        let (svc, _deposits) = bed.open();
        let q = svc.quote(out_of_zec_request(100_000)).await.expect("quote");
        let id = svc.execute(&q).await.expect("execute");
        assert_eq!(id, q.id, "execute returns the quote's SDK identity");
        assert_ne!(
            id.as_str(),
            q.deposit_address,
            "the SDK handle is not the provider address"
        );
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        svc.watch_status(id.clone(), ChannelSink { tx })
            .await
            .expect("the handle resolved from the home row and the poll spawned");
        let mut got = Vec::new();
        while let Some(s) = rx.recv().await {
            got.push(s);
        }
        assert_eq!(got.len(), 2, "the stream ran to the terminal");
        assert!(got[1].is_terminal());
        let polled = provider.polled();
        assert_eq!(
            polled,
            vec![q.deposit_address.clone(); 2],
            "every status poll carries the PROVIDER address"
        );
        assert!(
            polled.iter().all(|p| p != id.as_str()),
            "and never the SDK handle in the address position"
        );
    }

    /// Row 5 — CONSUMED. After `execute(&first)` a second `execute(&first)` is
    /// `QuoteExpired` with zero deposits — and stays so when the provider has meanwhile
    /// REISSUED the same address for a second quote (900,000): the stale DTO cannot claim
    /// the new record, and the new record then executes its own terms. Reddening
    /// mutation: the base keying (the replayed stale DTO takes the reissued record and
    /// pays 900,000 under a 100,000 approval).
    #[tokio::test]
    async fn a_consumed_quote_cannot_be_executed_again() {
        let bed = IdentityBed::new(RecordingProvider::scripted(&[
            (100_000, 0x22),
            (900_000, 0x22),
        ]));
        let (svc, deposits) = bed.open();
        let first = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("first quote");
        svc.execute(&first).await.expect("first executes");
        let second = svc
            .quote(out_of_zec_request(900_000))
            .await
            .expect("the provider reissues the address for a second quote");
        assert_eq!(first.deposit_address, second.deposit_address);
        let again = svc
            .execute(&first)
            .await
            .expect_err("a consumed quote does not execute again");
        assert!(
            matches!(again, SwapError::QuoteExpired),
            "the consumed replay is the typed QuoteExpired, got {}",
            again.code()
        );
        assert_eq!(
            deposit_terms(&deposits).len(),
            1,
            "no second deposit for the replay"
        );
        svc.execute(&second)
            .await
            .expect("the reissued record executes its own terms");
        let terms = deposit_terms(&deposits);
        assert_eq!(terms.len(), 2);
        assert_eq!(terms[0].1, 100_000);
        assert_eq!(terms[1].1, 900_000);
    }

    /// Row 5 — REISSUED AFTER PRUNING. Quote #1 (100,000) is pruned from the durable store
    /// (lapsed, swept); the provider reissues the SAME address for quote #2 (900,000). The
    /// stale DTO that once carried that address executes NOTHING — `QuoteExpired`, zero
    /// deposits, the provider never asked — and #2 then executes its own terms. Reddening
    /// mutation: the base keying (the stale DTO finds #2's record under the address and
    /// pays 900,000).
    #[tokio::test]
    async fn a_provider_address_reissued_after_pruning_cannot_be_claimed_by_the_stale_dto() {
        let bed = IdentityBed::new(RecordingProvider::scripted(&[
            (100_000, 0x22),
            (900_000, 0x22),
        ]));
        let (svc, deposits) = bed.open();
        let stale = svc
            .quote(out_of_zec_request(100_000))
            .await
            .expect("first quote");
        // the prune: the durable row lapses and is swept (the store double has no clock,
        // so the sweep is done by hand — the same end state `persist`'s inline prune leaves)
        assert!(
            bed.store
                .rows
                .lock()
                .expect("rows poisoned")
                .remove(&stale.id)
                .is_some(),
            "the first record was durably present before the prune"
        );
        let reissued = svc
            .quote(out_of_zec_request(900_000))
            .await
            .expect("the provider reissues the pruned address");
        assert_eq!(stale.deposit_address, reissued.deposit_address);
        let err = svc
            .execute(&stale)
            .await
            .expect_err("the stale DTO cannot claim the reissued record");
        assert!(
            matches!(err, SwapError::QuoteExpired),
            "a pruned quote reads as expired, got {}",
            err.code()
        );
        assert!(
            deposit_terms(&deposits).is_empty(),
            "nothing was deposited for the stale DTO"
        );
        assert!(
            bed.provider.executed().is_empty(),
            "the provider was never asked"
        );
        svc.execute(&reissued)
            .await
            .expect("the reissued quote executes its own terms");
        let terms = deposit_terms(&deposits);
        assert_eq!(terms.len(), 1);
        assert_eq!(
            (&terms[0].0, terms[0].1),
            (&parsed(&reissued.deposit_address), 900_000)
        );
    }

    /// The observable outcome of running the probe's scenario — one entry per execute,
    /// each term expressed RELATIVE to its own DTO so two runs with different clocks
    /// compare byte for byte: (address matches the DTO, amount, deadline − DTO deadline,
    /// binding matches the DTO, the home row's watch).
    type Outcome = Vec<(bool, i64, u64, bool, Option<(String, u32)>)>;

    async fn run_probe_scenario(
        bed: &IdentityBed,
        svc: &SwapService,
        deposits: &RecordingDeposit,
        quotes: &[SwapQuote],
    ) -> Outcome {
        let mut out = Vec::new();
        for (i, q) in quotes.iter().enumerate() {
            svc.execute(q)
                .await
                .unwrap_or_else(|e| panic!("execute {i} passes, got {}", e.code()));
            let terms = deposit_terms(deposits);
            let (addr, zat, deadline) = terms.last().expect("a deposit per execute").clone();
            let binding = deposits.bindings.lock().expect("bindings poisoned")[i];
            let row = &bed.records.rows()[i];
            out.push((
                addr == parsed(&q.deposit_address),
                zat,
                deadline - q.expires_at,
                binding == q.binding,
                row.watch.as_ref().map(|w| (w.address.clone(), w.index)),
            ));
        }
        out
    }

    /// Row 6 — CLOSE / REOPEN. Every execution term — address, amount, deadline, binding,
    /// refund watch — comes from the ONE durable record after a reopen: the probe's
    /// scenario (two quotes, one address, 100,000 then 900,000) run with a reopen between
    /// quote and execute has the SAME outcome, term for term, as the in-process run; the
    /// in-memory registry never decides a term. UNLISTED (IT-1 +A): the terms compare
    /// runs on the cross-restart path too — after the reopen a DTO carrying #2's amount
    /// under #1's identity is refused (the miss this predicts: a compare that reads the
    /// in-memory registry and lets the reconstruction path through unchecked).
    /// Reddening mutations: the base keying (either run pays the wrong amount or refuses
    /// the second execute); a compare that consults only the in-memory record.
    #[tokio::test]
    async fn every_execution_term_comes_from_the_durable_record_after_reopen() {
        let script = [(100_000, 0x22), (900_000, 0x22)];

        // The in-process run.
        let live = IdentityBed::new(RecordingProvider::scripted(&script));
        let (svc, deposits) = live.open();
        let q1 = svc.quote(out_of_zec_request(100_000)).await.expect("q1");
        let q2 = svc.quote(out_of_zec_request(900_000)).await.expect("q2");
        let in_process = run_probe_scenario(&live, &svc, &deposits, &[q1, q2]).await;

        // The reopen run: quote, drop the service (the in-memory registry dies with it),
        // open again over the same durable doubles, THEN execute.
        let reopened = IdentityBed::new(RecordingProvider::scripted(&script));
        let (svc, _quoting_deposits) = reopened.open();
        let q1 = svc.quote(out_of_zec_request(100_000)).await.expect("q1");
        let q2 = svc.quote(out_of_zec_request(900_000)).await.expect("q2");
        drop(svc);
        let (svc, deposits) = reopened.open();
        assert_refused_before_any_side_effect(
            svc.execute(&SwapQuote {
                zec_side: q2.zec_side,
                ..q1.clone()
            })
            .await,
            &deposits,
            &reopened.provider,
            0,
            0,
            "after the reopen, #2's amount under #1's identity",
        );
        let after_reopen = run_probe_scenario(&reopened, &svc, &deposits, &[q1, q2]).await;

        assert_eq!(
            after_reopen, in_process,
            "the reopen run's terms are byte-identical to the in-process run's"
        );
        assert_eq!(
            after_reopen,
            vec![
                (
                    true,
                    100_000,
                    DEPOSIT_EXECUTE_MARGIN_SECS,
                    true,
                    Some((t_addr(0x41), 1))
                ),
                (
                    true,
                    900_000,
                    DEPOSIT_EXECUTE_MARGIN_SECS,
                    true,
                    Some((t_addr(0x42), 2))
                ),
            ],
            "and they are each DTO's own terms: its address, its amount, its deadline + margin, its binding, its refund watch"
        );
    }

    /// Row 6's poll leg at the service seam — a `watch_status` opened on a service built
    /// AFTER the executing one died (the in-memory registry empty, the durable doubles
    /// intact) with the re-attach handle polls a REAL provider status: the stream reaches
    /// the provider's own terminal (never a synthesized `Failed(NotFound)`), and every poll
    /// carried the provider ADDRESS resolved from the durable record — before the poll
    /// task was spawned, not from a cache that died with the first service. The wallet-
    /// level namesake (`wallet::tests::s8_identity`) does the same over a real keyed
    /// reopen. Reddening mutations: the base keying; resolving the address from an
    /// in-memory SDK-id→address map.
    #[tokio::test(start_paused = true)]
    async fn a_reopened_service_resolves_the_provider_address_from_the_record_port_before_polling()
    {
        let provider = RecordingProvider::scripted(&[(100_000, 0x22)]);
        let bed = IdentityBed::new(Arc::clone(&provider));
        let (svc, _deposits) = bed.open();
        let q = svc.quote(out_of_zec_request(100_000)).await.expect("quote");
        let id = svc.execute(&q).await.expect("execute");
        assert_ne!(
            id.as_str(),
            q.deposit_address,
            "the handle is not the address"
        );
        drop(svc);

        provider.push_status(Ok(SwapStatus::Processing));
        provider.push_status(Ok(SwapStatus::Success {
            out_txid: None,
            realized_slippage_bps: None,
        }));
        let (svc, _deposits) = bed.open();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        svc.watch_status(id.clone(), ChannelSink { tx })
            .await
            .expect("the handle resolved from the home row and the poll spawned");
        let mut got = Vec::new();
        while let Some(s) = rx.recv().await {
            got.push(s);
        }
        assert_eq!(
            got,
            vec![
                SwapStatus::Processing,
                SwapStatus::Success {
                    out_txid: None,
                    realized_slippage_bps: None,
                }
            ],
            "a real provider status, to the provider's own terminal"
        );
        assert_eq!(
            provider.polled(),
            vec![q.deposit_address.clone(); 2],
            "every poll after the reopen carried the provider address from the durable record"
        );
    }

    /// Row 7 — THE APPROVAL BINDS. What the host's authorizer was shown and what the SDK
    /// executes are compared against the SAME record before it is consumed: a DTO that
    /// passed the host's approval with #1's numbers but names #2's record executes
    /// nothing, and the converse too; each honest DTO then executes its own terms.
    /// Reddening mutation: the base keying (the crossed DTO is indistinguishable from
    /// #1 and pays #2's amount).
    #[tokio::test]
    async fn the_approval_and_the_execution_read_the_same_record() {
        let bed = IdentityBed::new(RecordingProvider::scripted(&[
            (100_000, 0x22),
            (900_000, 0x22),
        ]));
        let (svc, deposits) = bed.open();
        let first = svc.quote(out_of_zec_request(100_000)).await.expect("first");
        let second = svc
            .quote(out_of_zec_request(900_000))
            .await
            .expect("second");
        // approved as #1 (100,000 shown to the user), naming #2's record
        assert_refused_before_any_side_effect(
            svc.execute(&SwapQuote {
                id: second.id.clone(),
                ..first.clone()
            })
            .await,
            &deposits,
            &bed.provider,
            0,
            0,
            "#1's approved numbers under #2's identity",
        );
        // approved as #2 (900,000 shown), naming #1's record
        assert_refused_before_any_side_effect(
            svc.execute(&SwapQuote {
                id: first.id.clone(),
                ..second.clone()
            })
            .await,
            &deposits,
            &bed.provider,
            0,
            0,
            "#2's approved numbers under #1's identity",
        );
        svc.execute(&first)
            .await
            .expect("the honest first executes");
        svc.execute(&second)
            .await
            .expect("the honest second executes");
        let terms = deposit_terms(&deposits);
        assert_eq!(
            terms.iter().map(|t| t.1).collect::<Vec<_>>(),
            vec![100_000, 900_000],
            "each approval paid exactly its own amount"
        );
    }

    /// Row 8 — MINTED, NOT DERIVED. Eight quotes whose provider responses are
    /// byte-identical (one address, one amount, one foreign side) get eight distinct SDK
    /// identities; none is the provider address, none carries any six-character window of
    /// it, and each is long enough to hold 128 bits in any printable alphabet. Reddening
    /// mutations: a content-hash id (identical quotes collide); the base keying (all
    /// eight are the address); a short counter.
    #[tokio::test]
    async fn the_sdk_id_is_minted_from_os_rng_and_two_identical_quotes_get_two_ids() {
        let n = 8;
        let script = vec![(100_000, 0x22); n];
        let bed = IdentityBed::new(RecordingProvider::scripted(&script));
        let (svc, _deposits) = bed.open();
        let mut quotes = Vec::new();
        for _ in 0..n {
            quotes.push(svc.quote(out_of_zec_request(100_000)).await.expect("quote"));
        }
        let address = quotes[0].deposit_address.clone();
        for q in &quotes {
            assert_eq!(
                (
                    &q.deposit_address,
                    q.zec_side,
                    &q.amount_in,
                    &q.min_amount_out
                ),
                (
                    &address,
                    quotes[0].zec_side,
                    &quotes[0].amount_in,
                    &quotes[0].min_amount_out
                ),
                "the provider's answers were identical in every term"
            );
        }
        let ids: std::collections::BTreeSet<&str> = quotes.iter().map(|q| q.id.as_str()).collect();
        assert_eq!(ids.len(), n, "identical quotes get distinct identities");
        for id in &ids {
            assert_ne!(
                *id,
                address.as_str(),
                "an identity is never the provider address"
            );
            assert!(
                id.len() >= 22,
                "an identity holds at least 128 bits ({} chars is too short for any printable encoding)",
                id.len()
            );
            for start in 0..=address.len() - 6 {
                let window = &address[start..start + 6];
                assert!(
                    !id.contains(window),
                    "an identity carries no window of the provider address ({window} in {id})"
                );
            }
        }
    }

    /// Recursively collect every `.rs` file under `dir`.
    fn rust_sources(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("readable source dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                rust_sources(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    /// Row 9 — THE NEVER-LOG MARKER MOVES WITH THE VALUE. (a) A source scan: every struct
    /// field named `provider_ref` in this crate carries the §5.4 NEVER-log marker in the
    /// doc comment directly above it — and at least one such field EXISTS (at the base
    /// commit none does; the marker sits on `SwapId` because the id IS the address).
    /// (b) A capture: a quote, its execute and a poll to terminal emit no tracing field
    /// whose VALUE contains the provider address, the SDK identity, the refund address or
    /// the destination, on top of the §5.4 name/token guard. Reddening mutations: a
    /// `provider_ref` field without the marker; a span recording `provider_ref`.
    #[tokio::test]
    async fn no_tracing_field_carries_the_provider_address() {
        // (a) the marker.
        let src_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        rust_sources(&src_root, &mut files);
        let mut declared = Vec::new();
        let mut unmarked = Vec::new();
        for path in &files {
            let src = std::fs::read_to_string(path).expect("readable source");
            let lines: Vec<&str> = src.lines().collect();
            let mut in_struct: Option<usize> = None; // the struct line's indentation
            for (i, line) in lines.iter().enumerate() {
                let indent = line.len() - line.trim_start().len();
                let body = line.trim();
                if let Some(depth) = in_struct {
                    if indent == depth && body == "}" {
                        in_struct = None;
                        continue;
                    }
                    let field = body
                        .strip_prefix("pub(crate) ")
                        .or_else(|| body.strip_prefix("pub(super) "))
                        .or_else(|| body.strip_prefix("pub "))
                        .unwrap_or(body);
                    if indent == depth + 4 && field.starts_with("provider_ref:") {
                        let at = format!("{}:{}", path.display(), i + 1);
                        let mut doc = String::new();
                        let mut j = i;
                        while j > 0 && lines[j - 1].trim().starts_with("///") {
                            j -= 1;
                            doc.push_str(lines[j]);
                        }
                        if !doc.to_ascii_lowercase().contains("never-log") {
                            unmarked.push(at.clone());
                        }
                        declared.push(at);
                    }
                } else if body.contains("struct ") && body.ends_with('{') && !body.starts_with("//")
                {
                    in_struct = Some(indent);
                }
            }
        }
        assert!(
            !declared.is_empty(),
            "no `provider_ref` struct field is declared in this crate — the provider address has not been demoted to data inside the record (contract §3.1 rows 8–9)"
        );
        assert!(
            unmarked.is_empty(),
            "`provider_ref` fields without the §5.4 NEVER-log marker in their doc: {unmarked:?}"
        );

        // (b) the capture.
        use crate::tracing_guard::{
            CaptureLayer, CapturedEvents, assert_5_4_clean, force_wallet_callsites_enabled,
        };
        use tracing_subscriber::prelude::*;
        force_wallet_callsites_enabled();
        let sink = CapturedEvents::default();
        let subscriber = tracing_subscriber::registry().with(CaptureLayer::new(sink.clone()));
        let _guard = tracing::subscriber::set_default(subscriber);

        let provider = RecordingProvider::scripted(&[(100_000, 0x22), (100_000, 0x33)]);
        provider.push_status(Ok(SwapStatus::Success {
            out_txid: None,
            realized_slippage_bps: None,
        }));
        let bed = IdentityBed::new(Arc::clone(&provider));
        let (svc, _deposits) = bed.open();
        let out = svc.quote(out_of_zec_request(100_000)).await.expect("quote");
        let id = svc.execute(&out).await.expect("execute");
        let into = svc.quote(into_zec_request()).await.expect("into-zec quote");
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        svc.watch_status(id.clone(), ChannelSink { tx })
            .await
            .expect("the handle resolved from the home row and the poll spawned");
        while rx.recv().await.is_some() {}

        let fields = sink.fields();
        assert_5_4_clean(&fields);
        assert!(!fields.is_empty(), "the swap spans fired under capture");
        let secrets = [
            ("the provider address", out.deposit_address.as_str()),
            ("the SDK identity", id.as_str()),
            ("the refund address", &t_addr(0x41)),
            (
                "the into-zec provider address",
                into.deposit_address.as_str(),
            ),
            ("the into-zec identity", into.id.as_str()),
            ("the destination", "u1dest1"),
        ];
        for (what, secret) in secrets {
            let carrying: Vec<&(String, String)> =
                fields.iter().filter(|(_, v)| v.contains(secret)).collect();
            assert!(
                carrying.is_empty(),
                "a tracing field carries {what}: {carrying:?}"
            );
        }
    }

    // ── S8 `identity` — rows against names the contract says the implementer ADDS ──
    //
    // These cannot compile at the base commit (the names do not exist there), so they
    // are PARSED but never BUILT: `#[cfg(any())]` is never true. The adjudicator joins
    // them by replacing that attribute with `#[cfg(test)]` once the names exist, and
    // adjusts a spelling to the implementer's where it differs (each name is declared
    // in the test author's `contractFindings`). Validated against a throwaway
    // definition of each name, never committed.
    #[cfg(test)]
    mod s8_declared {
        use super::*;

        /// The "terms differ" refusal is the NEW typed variant
        /// `SwapError::QuoteTermsDiffer` — payload-free (the differing terms are §5.4
        /// never-log values) — with the next append-only code `RW-SWAP-016`, so the
        /// bridge's `SwapErrorKind` mirror and the 16-locale copy can name it. The
        /// refusal consumes nothing: the honest DTO executes afterwards.
        #[tokio::test]
        async fn a_terms_differ_refusal_is_the_typed_quote_terms_differ_variant() {
            let bed = IdentityBed::new(RecordingProvider::scripted(&[(100_000, 0x22)]));
            let (svc, deposits) = bed.open();
            let honest = svc.quote(out_of_zec_request(100_000)).await.expect("quote");
            let err = svc
                .execute(&SwapQuote {
                    zec_side: Zatoshis::new(honest.zec_side.zat() + 1).expect("valid"),
                    ..honest.clone()
                })
                .await
                .expect_err("a DTO whose amount differs is refused");
            assert!(
                matches!(err, SwapError::QuoteTermsDiffer),
                "the typed terms-differ variant, got {}",
                err.code()
            );
            assert_eq!(err.code(), "RW-SWAP-016", "the next append-only code");
            assert!(deposit_terms(&deposits).is_empty(), "nothing deposited");
            svc.execute(&honest)
                .await
                .expect("the refusal consumed nothing — the honest DTO executes");
        }
    }

    // ── S7 W1: the out-of-ZEC deposit is bounded by the user's own number ──

    /// An out-of-ZEC swap fixing the ZEC paid (`In(Zec(z))`) signs a deposit of
    /// `quote.zec_side`, so that side must BE `z`: one zatoshi more or less is
    /// `QuoteOutOfBounds`, even inside the slippage tolerance. The into-ZEC
    /// `Out(Zec(z))` side (the wallet signs nothing) keeps its two-sided bound.
    #[tokio::test]
    async fn an_out_of_zec_deposit_is_exactly_the_zec_the_user_typed() {
        let quote_with = |zec_side: i64, req: QuoteRequest| async move {
            service(echoing_mock(zec_side, 3_600)).quote(req).await
        };
        quote_with(100_000, out_of_zec_request(100_000))
            .await
            .expect("an honest echo of the user's number passes");
        for zec_side in [100_001, 99_999] {
            let err = quote_with(zec_side, out_of_zec_request(100_000))
                .await
                .expect_err("a deposit that is not the user's number is refused");
            assert!(
                matches!(
                    err,
                    SwapError::QuoteOutOfBounds {
                        side: QuoteBoundSide::Zec
                    }
                ),
                "zec_side {zec_side}: got {}",
                err.code()
            );
        }
        let into_exact_out = QuoteRequest {
            exact: ExactSide::Out(SwapAmount::Zec(Zatoshis::new(100_000).expect("valid"))),
            ..into_zec_request()
        };
        quote_with(100_001, into_exact_out)
            .await
            .expect("into-ZEC exact-out stays within its tolerance");
    }
}
