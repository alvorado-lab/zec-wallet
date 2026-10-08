//! §3.2h The send pipeline — inc-2d-1 PROPOSE + inc-2d-2 CREATE+SIGN.
//!
//! The deterministic money core: given a validated `zip321::TransactionRequest` this
//! runs the audited note-selection + fee + change computation over the wallet DB and
//! returns a [`SendProposal`] — the numbers the user signs off on. It is the
//! lowest-risk send slice by construction:
//!
//! - **No keys.** Propose reads notes and computes amounts; it derives NO spending
//!   key, builds NO proof, signs NOTHING (that is inc-2d-2 `send`).
//! - **No network.** Pure DB read; no dial, no broadcast (that is inc-2d-3).
//! - **No DB writes on OUR path.** This bullet narrowed with
//!   `zcash_client_backend` 0.24.0: `propose_transfer` now takes a `WalletWrite`
//!   bound, because it can LOCK the inputs it selects on behalf of a `LockOwner`.
//!   We pass `lock_inputs: None`, so no write happens — but the bound is real and
//!   the claim is now about what we ask for, not about what the signature forbids.
//!   Re-proposing is still idempotent.
//!
//! Rule Zero (ADR-0005): every money decision is the audited
//! `zcash_client_backend` engine's — `propose_transfer` with the `GreedyInputSelector`
//! and the standard ZIP-317 change strategy. Our code is the validation funnel
//! (`PaymentRequest` → `zip321::TransactionRequest`, one door — `payment_uri`), the
//! SSOT confirmations policy (`account::spendable_policy`), the SDK display DTO, and
//! the opaque one-shot token registry. We compute NO fee, select NO note, and decide
//! NO output pool ourselves — the DTO's per-recipient pool is the engine's own
//! `Proposal::payment_pools` verdict.
//!
//! The upstream `Proposal<FeeRule, NoteRef>` never crosses FFI (it holds note
//! references): it is retained Rust-side in the per-wallet [`ProposalRegistry`] and
//! the public `send` (inc-2d-2) consumes it by id — one-shot + TTL-bounded.

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use sapling_crypto::prover::{OutputProver, SpendProver};
use tokio::time::Instant;
use zcash_client_backend::data_api::locking::LockFilter;
use zcash_client_backend::data_api::wallet::input_selection::{GreedyInputSelector, SpendPolicy};
use zcash_client_backend::data_api::wallet::{
    SpendingKeys, create_proposed_transactions, propose_shielding, propose_transfer,
};
use zcash_client_backend::data_api::{
    CoinbaseFilter, InputSource, WalletCommitmentTrees, WalletRead, WalletWrite,
};
use zcash_client_backend::fees::{DustOutputPolicy, StandardFeeRule, standard};
use zcash_client_backend::proposal::{Proposal, StepOutput, StepOutputIndex};
use zcash_client_backend::wallet::OvkPolicy;
use zcash_client_sqlite::ReceivedNoteId;
use zcash_client_sqlite::error::SqliteClientError;
use zcash_keys::keys::UnifiedSpendingKey;
use zcash_protocol::TxId;
use zcash_protocol::consensus::Parameters;
// `ShieldedProtocol` is a deprecated alias for `ShieldedPool` as of zcash_protocol
// 0.10.5, and CI runs `clippy --all-targets -D warnings` — so the alias is a BUILD
// FAILURE, not a nit. Renaming it here rather than silencing the lint is deliberate:
// the alias's own deprecation note is what points at `ShieldedPool::Ironwood`, the
// variant every match in this file now has to answer for.
use zcash_protocol::{PoolType, ShieldedPool};
use zcash_transparent::address::TransparentAddress;
use zcash_transparent::keys::TransparentKeyScope;

use crate::account;
use crate::constants::{
    CLOCK_PLAUSIBILITY_FLOOR_SECS, DEPOSIT_CONTINUE_MARGIN_SECS, DEPOSIT_FIRST_FEED_MARGIN_SECS,
    PROPOSAL_ANCHOR_DRIFT_MAX_BLOCKS, PROPOSAL_REGISTRY_MAX_LIVE, PROPOSAL_TTL_SECS,
    SHIELDING_THRESHOLD_ZAT,
};
use crate::error::WalletError;
use crate::intent_store::{
    InFlightIntent, MAX_CLAIM_NOTES, NoteClaim, PROTO_IRONWOOD, PROTO_ORCHARD, PROTO_SAPLING,
};
use crate::memo::PaymentRequest;
use crate::money::Zatoshis;
use crate::payment_uri::zatoshis_from_protocol;
use crate::seed::SpendBinding;
use crate::state::QueuedSendId;
use crate::sync::ClassifyStoreFault;

/// A retained TRANSFER proposal for THIS wallet's DB (`WalletConn`): a `StandardFeeRule`
/// (ZIP-317) proposal over `ReceivedNoteId` (shielded-note) references. Never crosses FFI.
type TransferProposal = Proposal<StandardFeeRule, ReceivedNoteId>;

/// A retained SHIELD proposal (Recv-3 / inc-2d-shield): the inputs are transparent UTXOs,
/// so it references NO shielded notes — the upstream `propose_shielding` types the NoteRef
/// as the uninhabited [`Infallible`](std::convert::Infallible). Same fee rule, same
/// `summarize`/`create_signed_core` generics; only the `NoteRef` differs. Never crosses FFI.
type ShieldProposal = Proposal<StandardFeeRule, std::convert::Infallible>;

/// The kind of money a one-shot token retains (§3.2h decision 3). `send(id)` consumes
/// either a `Transfer` (the §3.2h send pipeline — shielded-note inputs, an external
/// recipient) or a `Shield` (Recv-3 — transparent-UTXO inputs → a shielded self-output).
/// Both summarize + sign through the SAME `NoteRef`-generic `summarize`/`create_signed_core`
/// (verified: `create_proposed_transactions`'s `N` is free of `DbT::NoteRef`); this enum
/// exists only because the two concrete `NoteRef`s differ, so the registry must name both.
/// Never crosses FFI (it holds note/UTXO references).
pub(crate) enum RetainedProposal {
    Transfer(TransferProposal),
    Shield(ShieldProposal),
}

/// The on-chain value pool a proposal output lands in — the §5.1 de-shield
/// disclosure primitive (`Transparent` ⇒ funds leave the shielded set). This is the
/// AUDITED engine's own per-output pool determination (`Proposal::payment_pools`),
/// not a guess re-derived from the recipient address.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum OutputPool {
    Transparent,
    Sapling,
    Orchard,
    /// The Ironwood pool (NU6.3). After the activation the Orchard turnstile forbids
    /// ADDING value to Orchard, so every payment to an Orchard receiver is delivered
    /// in the Ironwood bundle — which makes this the ORDINARY case for a shielded
    /// send, not an exotic one. It gets its own variant rather than folding into
    /// `Orchard` because this DTO is what the user reads on the confirm screen.
    Ironwood,
}

impl OutputPool {
    fn from_pool_type(p: PoolType) -> Self {
        match p {
            PoolType::Transparent => OutputPool::Transparent,
            PoolType::Shielded(ShieldedPool::Sapling) => OutputPool::Sapling,
            PoolType::Shielded(ShieldedPool::Orchard) => OutputPool::Orchard,
            PoolType::Shielded(ShieldedPool::Ironwood) => OutputPool::Ironwood,
        }
    }

    /// The inverse of [`from_pool_type`](Self::from_pool_type). Its only caller is
    /// `output_pool_round_trips_every_pool_type`, and that is the point: a wildcard
    /// repair of `from_pool_type` would silently label an Ironwood output as
    /// something else on the screen the user confirms a payment on, and nothing
    /// would fail.
    ///
    /// What the pairing does and does not buy, said exactly (an earlier draft
    /// overclaimed it): adding a variant to `OutputPool` breaks THIS match at
    /// compile time, because the match is over `Self`. It does NOT force
    /// `from_pool_type` to stay exhaustive — a wildcard `_ => Orchard` there still
    /// compiles. That case is caught at RUNTIME, by the round-trip assertion,
    /// which is why the test asserts identity rather than merely totality.
    #[cfg(test)]
    fn to_pool_type(self) -> PoolType {
        match self {
            OutputPool::Transparent => PoolType::Transparent,
            OutputPool::Sapling => PoolType::SAPLING,
            OutputPool::Orchard => PoolType::ORCHARD,
            OutputPool::Ironwood => PoolType::IRONWOOD,
        }
    }
}

/// The display DTO for a prepared send (§3.2h decision 3) — the numbers the user
/// confirms. Integer zatoshis only (no float ever); the opaque proposal is retained
/// Rust-side and named by [`proposal_id`](Self::proposal_id).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SendProposal {
    /// Handle to the retained one-shot proposal token (§3.2h). `send(proposal_id)`
    /// (inc-2d-2) consumes it exactly once; a second use is `ProposalAlreadyUsed`.
    pub proposal_id: u64,
    /// FR-17 spend-binding nonce — SDK-minted at propose, retained beside the
    /// one-shot token, and presented to the host seed port when THIS proposal
    /// signs. A host that records it at its authorize bracket can fail-close a
    /// seed pull for any OTHER proposal (the review↔sign confusion defense).
    /// Opaque, random, NOT key material (§5.4: never logged; `Debug` redacted).
    pub binding: SpendBinding,
    /// Total debited from the wallet = recipient amounts + fee (the headline number).
    pub total_zat: i64,
    /// The ZIP-317 fee the audited change strategy computed.
    pub fee_zat: i64,
    /// Change returned to the wallet (informational; already net out of `total`).
    pub change_zat: i64,
    /// Per-step recipients. One step for an ordinary send; TWO for a ZIP-320 TEX two-step
    /// (tx0 unshields to a wallet-controlled ephemeral t-address, tx1 forwards it to the TEX
    /// destination — see [`summarize`] and [`is_two_step_tex`](Self::is_two_step_tex)). Any
    /// OTHER multi-step shape is fail-closed in [`summarize`], so >1 step ⟺ the TEX two-step.
    pub steps: Vec<ProposalStep>,
    /// The chain height the proposal's note anchor targets. The on-chain expiry is
    /// finalized when the tx is built (inc-2d-2); the wall-clock freshness guard is
    /// [`PROPOSAL_TTL_SECS`] on the retained token.
    pub target_height: u32,
    /// §5.1 de-shield disclosure: TRUE if any output lands in the transparent pool
    /// (funds leave the shielded set — the host must surface it before the user signs).
    pub has_transparent_recipient: bool,
    /// TRUE iff this is a SHIELD proposal (Recv-3 / `propose_shield`): transparent UTXOs →
    /// a shielded self-output. Privacy-POSITIVE (the §5.1 de-shield disclosure does NOT
    /// apply — opposite direction). The host shows shield-specific confirm copy ("Shield
    /// X · fee Y") instead of a recipient/send screen; `total_zat` is the gross transparent
    /// being shielded, `change_zat` the net that lands shielded (gross − fee), and the lone
    /// `steps` recipient is the wallet's OWN shielded output (no external recipient).
    pub is_shield: bool,
    /// TRUE iff this proposal is a ZIP-320 TEX two-step (the engine-recognised ephemeral pair —
    /// computed via [`is_zip320_two_step`], the SSOT predicate, NOT re-derived from `steps.len()`).
    /// The host carries it past the confirm screen so the post-send OUTCOME can tell a partial
    /// TEX broadcast (funds in-motion on a wallet-controlled ephemeral address — money has LEFT the
    /// shielded pool but NOT reached the recipient; recoverable via the manual sweep; the user must
    /// NOT re-send) from a partial ORDINARY multi-tx send (each tx is independent + saved-for-retry).
    /// §5.4: a pure SHAPE bool — carries no address/amount/txid. Always `false` for a shield and for
    /// a single-step send. (Equivalent to `steps.len() > 1` for any proposal that survives
    /// [`summarize`]'s fail-closed multi-step check, but tied to the predicate so a future engine
    /// multi-step shape can never silently mislabel the outcome copy.)
    pub is_two_step_tex: bool,
    /// MONEY-SAFETY (§3 send): the large-amount magnitude signal — `Some` ⇒ the host shows ONE
    /// deliberate large-amount confirm before signing (the only friction step; everything else is
    /// a passive cue — no alert fatigue). `None` ⇒ an ordinary send. Computed at propose time from
    /// `total_zat` vs the SAME spendable SSOT the balance uses (never re-derived). Always `None`
    /// for a shield (a self-shield is not a "large send" to a recipient). See [`LargeSendReason`].
    pub large_send: Option<LargeSendReason>,
    /// MONEY-SAFETY (§3 send): a best-effort signal that a recipient is the wallet's OWN current
    /// address — the host shows a passive INFO note ("you're sending to yourself; the fee still
    /// applies"), never a blocker (a self-send is money-safe, just usually unintended). Best-effort
    /// by design: an exact match against the wallet's current unified address (the common
    /// pasted-my-own-address case); a diversified / sub-receiver own-address is NOT detected (a
    /// scoped v1 limitation, upgradeable to a full ownership test). Always `false` for a shield.
    pub self_send: bool,
    /// FR-46: the TOTAL paid to the send's one recipient, in zatoshis, as SIGNED, the fee and
    /// the change excluded — `Some` iff every payment of every step names the SAME address
    /// (`ZcashAddress` equality; two payments to one address are summed). Two recipients ⇒
    /// `None`, never a partial sum; a UA and one of its own receivers are two recipients (the
    /// safe direction: no figure, never a wrong one). The ZIP-320 two-step counts the forwarded
    /// amount once (its ephemeral hop is change, never a payment). Always `None` for a shield
    /// (no external recipient). A statement about what was signed, never about arrival.
    pub single_recipient_zat: Option<i64>,
}

/// Why a proposal tripped the large-amount confirm (§3 send money-safety). Drives the host's
/// confirm copy: "almost your whole balance" vs "a large amount" vs both. The triggers fire
/// independently ("whichever first") — see [`classify_magnitude`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum LargeSendReason {
    /// The total debit is ≥ `NEAR_TOTAL_SPENDABLE_BPS` of spendable — sending almost everything.
    NearTotalBalance,
    /// The total debit is ≥ `LARGE_SEND_ABSOLUTE_ZAT` — objectively large regardless of balance.
    OverAbsoluteThreshold,
    /// Both triggers fired.
    Both,
}

/// Classify a proposal's magnitude for the §3 large-amount confirm: `Some(reason)` when the total
/// debit trips the RELATIVE (≥ `NEAR_TOTAL_SPENDABLE_BPS` of the AVAILABLE balance) OR the
/// ABSOLUTE (≥ `LARGE_SEND_ABSOLUTE_ZAT`) trigger, "whichever first". Pure + integer-only (i128
/// so the `× 10_000` can't overflow a near-max `total`); no float ever touches money. `available_zat`
/// is the SHIELDED spendable balance — the ONLY pool `propose_transfer` selects from (verified
/// against zcash_client_backend 0.23.0: the `GreedyInputSelector` funds transfers via
/// `select_spendable_notes` alone; transparent funds move only through `propose_shielding`) — so
/// `total ≤ available` always and the fraction is well-formed; the `> 0` guard keeps it total even
/// on a degenerate zero-available input (the absolute still fires).
pub(crate) fn classify_magnitude(total_zat: i64, available_zat: i64) -> Option<LargeSendReason> {
    use crate::constants::{LARGE_SEND_ABSOLUTE_ZAT, NEAR_TOTAL_SPENDABLE_BPS};
    let near = available_zat > 0
        && i128::from(total_zat) * 10_000
            >= i128::from(available_zat) * i128::from(NEAR_TOTAL_SPENDABLE_BPS);
    let over = total_zat >= LARGE_SEND_ABSOLUTE_ZAT;
    match (near, over) {
        (true, true) => Some(LargeSendReason::Both),
        (true, false) => Some(LargeSendReason::NearTotalBalance),
        (false, true) => Some(LargeSendReason::OverAbsoluteThreshold),
        (false, false) => None,
    }
}

/// Best-effort §3 self-send detection: TRUE iff any recipient leg's encoded address EXACTLY
/// equals the wallet's own current address. Drives a passive info note only (never a gate), so
/// the conservative exact-match is the right scope: a false negative (a diversified own-address)
/// just omits the note, never blocks or mis-warns a real send. `own_encoded` is the wallet's
/// current unified address (`Wallet::current_address`).
pub(crate) fn is_self_send(request: &PaymentRequest, own_encoded: &str) -> bool {
    request
        .payments
        .iter()
        .any(|p| p.recipient.encoded() == own_encoded)
}

/// One step of a proposal (one transaction). >1 step ⇒ pool-crossing (§1.7).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProposalStep {
    pub recipients: Vec<ProposalRecipient>,
}

/// One recipient output within a step — its on-chain pool (§5.1 disclosure) + amount.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ProposalRecipient {
    pub pool: OutputPool,
    pub amount_zat: i64,
}

/// Run the audited note-selection + fee + change over `wdb` for an already-lowered
/// `zip321` request, returning the retained upstream [`Proposal`]. DETERMINISTIC: no
/// keys, no proofs, no network, no DB writes. Generic over the DB + network params so
/// the funded-wallet test harness (`LocalNetwork`) and the live wallet (`Network`)
/// drive the SAME code.
///
/// Wiring is the §3.2h-locked set: `GreedyInputSelector` + the standard ZIP-317
/// change strategy, the [`account::spendable_policy`] SSOT (so the notes a balance
/// calls "spendable" and the notes propose may select can never disagree), and an
/// Orchard fallback change pool (this SDK is always Orchard-enabled). The
/// `CommitmentTreeErrT` is pinned `Infallible` exactly as the upstream test harness
/// does — the SQLite store's tree reads do not surface a foreign tree error here.
///
/// The caller (`Wallet::propose`) lowers the host's [`PaymentRequest`] to the
/// `request` through the shared `payment_uri::to_transaction_request` door (DRY) and
/// enforces the amount-required-for-send rule first; this function takes the lowered
/// request so it stays network-agnostic and unit-testable on any consensus params.
///
/// CALLER CONTRACT (do NOT move into this fn): a caller passing a user-supplied request MUST
/// apply [`normalize_tex_first`] first — both production funnels do (`Wallet::propose` and
/// [`prepare_queued`]). This fn is deliberately kept RAW (no internal normalisation) so the
/// engine-ordering-quirk pin `tex_mixed_shielded_first_is_propose_failed_engine_ordering_quirk`
/// retains access to unnormalised behaviour; a future funnel that skips normalisation will
/// `ProposeFailed` on a legitimately-ordered shielded-first mixed TEX send once signing is ungated.
pub(crate) fn propose_core<DbT, P>(
    wdb: &mut DbT,
    params: &P,
    account_id: <DbT as InputSource>::AccountId,
    request: zip321::TransactionRequest,
) -> Result<Proposal<StandardFeeRule, <DbT as InputSource>::NoteRef>, WalletError>
where
    DbT: WalletRead + WalletWrite + InputSource<Error = <DbT as WalletRead>::Error>,
    <DbT as WalletRead>::Error: ClassifyStoreFault,
    <DbT as InputSource>::NoteRef: Copy + Eq + Ord,
    P: Parameters + Clone,
{
    let input_selector = GreedyInputSelector::new();
    let change_strategy = standard::SingleOutputChangeStrategy::new(
        StandardFeeRule::Zip317,
        // No change memo — we never attach one to our own change output.
        None,
        // Fallback change pool when the recipient pool can't determine it (e.g. a
        // pure de-shield to transparent): Orchard. This SDK is unconditionally
        // Orchard-enabled (zcash_keys/zcash_client_sqlite carry `orchard`), and the
        // modern Orchard-preferred posture keeps our change in the ecosystem
        // anonymity set (§2.3) rather than a distinguishing Sapling fingerprint.
        //
        // NAMING THE LEGACY POOL IS DELIBERATE, and it is not what decides where the
        // change lands (survey S8). Post-NU6.3 the Orchard turnstile forbids ADDING
        // value to Orchard, and the engine redirects change accordingly; this
        // argument is the FALLBACK the strategy consults when the recipient pool
        // cannot determine one.
        //
        // OWED, and not written here: the assertion that the change really landed
        // where consensus requires belongs on the BUILT proposal's own
        // `output_pool()` at a POST-ACTIVATION target height, never on this
        // constructor argument. The funded harness cannot reach that height yet —
        // upstream's `DEFAULT_NETWORK` has `nu6_3: None` and there is no Ironwood
        // note fixture — so the test needs a post-activation `LocalNetwork` and
        // belongs with the money step, not with the pin wave. Half of it (the
        // pre-activation direction) would pass today and prove nothing, which is
        // the reason it is owed rather than approximated.
        ShieldedPool::Orchard,
        DustOutputPolicy::default(),
    );

    let result = propose_transfer::<_, _, _, _, std::convert::Infallible>(
        wdb,
        params,
        account_id,
        &input_selector,
        &change_strategy,
        request,
        // THE spendable SSOT — the same policy the balance reader uses.
        account::spendable_policy(),
        // 0.24.0 splits "how many confirmations" from "which pools may be spent".
        // The default admits all three shielded pools and NO transparent inputs,
        // which is exactly this path's posture: the user send path never spends a
        // transparent UTXO (only the privacy-positive shielding path does, via
        // `propose_shielding`). Admitting Ironwood here is load-bearing rather than
        // permissive — after the activation new notes ARE Ironwood, so a policy that
        // excluded it would show the user a balance it then refuses to spend
        // (survey S7).
        //
        // Say the rest of it plainly, since `default()` reads as "no choice made":
        // this is upstream's MAXIMALLY PERMISSIVE shielded set — Sapling, Orchard
        // AND Ironwood, so one transaction may spend across shielded pools, and
        // upstream's own note says crossing a pool boundary reduces privacy and
        // "must be an explicit choice of the caller". It is the choice here, and it
        // preserves 0.23.0's behaviour exactly; a narrower policy would strand the
        // pool a user's balance actually sits in. Revisit against spec §2.3 if the
        // wallet ever gains a pool-preference posture.
        &SpendPolicy::default(),
        // No input locking. Locking exists to keep CONCURRENT proposals for one
        // account off the same notes; this SDK serialises propose→create→send
        // through the one-shot proposal token, so there is no second owner to race,
        // and a lock whose window expires mid-proving would re-open the very race it
        // was taken for.
        None,
        // No pinned transaction version: build at the version the TARGET HEIGHT's
        // branch implies. `Some(_)` would freeze the version, and freezing it at the
        // pre-Ironwood one is exactly how a wave like this compiles green and keeps
        // signing under the old rules (survey S15). `None` also lets upstream reject
        // an Orchard-receiver payment that could not carry an Ironwood bundle,
        // rather than us building one that fails at build time.
        None,
    );
    match result {
        Ok(proposal) => Ok(proposal),
        // The mutable borrow for `propose_transfer` is released here, so the error
        // mapper can re-read `wdb` (immutably) for the `pending_incoming` enrichment.
        Err(e) => Err(map_propose_err(wdb, e)),
    }
}

/// Map the upstream propose error to the SDK taxonomy (§3.2h decision 4 — reuse, no
/// collisions). `InsufficientFunds` carries the selector's `available`/`required`
/// plus `pending_incoming` read from the SAME spendable-policy summary as the balance
/// (the SSOT). A DB-access fault is classified (`ClassifyStoreFault`, R12); a
/// not-yet-anchorable wallet folds to `ProposalStale` (re-propose after sync);
/// a refusal the wallet's own state clears is `ProposeTransient` (INC-018 (b);
/// `propose_refusal_is_transient` is the one place the class is decided, and its
/// doc says why that class is EMPTY on the propose path at this pin); everything
/// else fails closed and payload-free as `ProposeFailed` (the selector message
/// can echo amounts — §5.4 — so it is DROPPED).
fn map_propose_err<DbT, DE: ClassifyStoreFault, CE, SE, FE, ChE, N>(
    wdb: &DbT,
    e: zcash_client_backend::data_api::error::Error<DE, CE, SE, FE, ChE, N>,
) -> WalletError
where
    DbT: WalletRead<Error: ClassifyStoreFault>,
{
    use zcash_client_backend::data_api::error::Error;
    match e {
        Error::DataSource(de) => de.into_store_fault(),
        // The wallet has no usable target/anchor height yet (not scanned far enough);
        // re-propose once sync advances — the same recovery the TTL-stale case takes.
        Error::ScanRequired => WalletError::ProposalStale,
        Error::InsufficientFunds {
            available,
            required,
        } => insufficient_funds(wdb, available, required),
        // INC-018 (b), phase-2 P2-2: the refusals the wallet's OWN state will clear —
        // a witness the scan has not completed, an anchor not yet recorded, an input
        // a concurrent proposal holds — are TRANSIENT, and the copy for them is
        // "try again in a moment", never "check the details". Told apart from the
        // enum in `propose_refusal_is_transient` (the one place); the `code` names
        // the variant so the field log says which shape it was.
        transient if propose_refusal_is_transient(&transient) => {
            // One event name for both arms (`wallet.propose_refused`, the crate's
            // `wallet.<event>` shape); `outcome` names the class, `code` the variant.
            tracing::warn!(
                target: "zec_wallet_core",
                outcome = "propose_refused_transient",
                code = propose_err_variant(&transient),
                "wallet.propose_refused"
            );
            WalletError::ProposeTransient
        }
        // change/fee fault, internal note-selection error, structurally-unsupported
        // request (e.g. multi-step pool-crossing — deferred), conversion faults …
        // — every one deterministic on this input.
        //
        // INC-018 (a): this arm used to say NOTHING. On the device proof the first
        // propose of a valid self-send took it, the user read "check the details" over
        // details that were correct, and neither the user nor the log said why the
        // money path refused. The line below names the VARIANT — a static string, no
        // payload, no selector message, no amount (§5.4) — so a refusal on the money
        // path is never silent again. INC-018 (b) then split the retryable class out
        // (the arm above); what is left here is the class whose honest copy IS
        // "check the details": retrying the same input re-fails.
        other => {
            tracing::warn!(
                target: "zec_wallet_core",
                outcome = "propose_refused_unmapped",
                code = propose_err_variant(&other),
                "wallet.propose_refused"
            );
            WalletError::ProposeFailed
        }
    }
}

/// INC-018 (b), phase-2 P2-2 (maintainer decision 4): is this upstream propose
/// refusal one the wallet's OWN state will clear without the user changing
/// anything? Classified FROM THE ENUM — `data_api::error::Error` and the
/// `ProposalError` members it wraps — not from field logs, so every variant
/// `propose_err_variant` names has a class here and the two functions are read
/// together at a pin bump. The one place the two classes are told apart; the
/// mapper's arms carry no classification of their own.
///
/// **Transient (`true`) — the next sync pass or the concurrent operation clears
/// it; the copy is "try again in a moment":**
/// - `CommitmentTree(_)`: the note commitment tree could not answer —
///   `Query(TreeIncomplete | CheckpointPruned | NotContained)` is a witness the
///   scan has not completed, and `Insert`/`Storage` are tree state the next
///   pass rewrites.
/// - `Proposal(AnchorNotFound(_))`: no anchor recorded at the target height yet —
///   the scan is behind the height the proposal reached for.
/// - `Proposal(InputsLocked(_))`: an input another in-flight proposal holds —
///   upstream's own doc: "a transient condition … retry".
///
/// **What this class REACHES on the propose path at this pin (0.24.0), read at
/// upstream by the Batch C security pass and confirmed at the code: NOTHING.**
/// `AnchorNotFound` is constructed only inside `build_proposed_transaction`
/// (the create+sign path, `data_api/wallet.rs` ~1766/1811/1864 — where
/// `map_create_err` folds it and `CommitmentTree` to `ProposalStale`);
/// `Error::CommitmentTree` arises only through `From<ShardTreeError>` on tree
/// operations, and `propose_transfer` / `propose_shielding` / the greedy
/// input selector perform none; `InputsLocked` is constructed only by
/// `lock_proposal_inputs`, which runs only under `lock_inputs: Some(_)` — this
/// crate passes `None` at both call sites. So the propose-side transient class
/// is EMPTY today: the mechanism (this predicate, the second variant, the FFI
/// arm, the Dart reason and copy) is built and guarded end to end, and the day
/// INC-018 (a)'s `code` field names the variant the device actually hit,
/// adding it here is a one-line change — but until then the refusal is
/// NOT routed to the retryable copy, and the registry row says so (OWED on
/// (b)). A witness-held note under the spendable policy is NOT this class
/// either: it is excluded from selection and surfaces as the typed
/// `InsufficientFunds` with `pending_incoming` (the P2-4 copy), never the
/// catch-all.
///
/// **Deterministic on this input (`false`) — retrying unchanged re-fails; the
/// copy is "check the details":** `NoteSelection(_)` (a balance overflow, an
/// address with no supported receiver, a TEX address without the feature),
/// `Change(_)` (dust inputs, a strategy or bundle fault — the selector already
/// turns its `InsufficientFunds` into the typed one before it reaches here),
/// every other `Proposal(_)` member (structural invariants of the request:
/// totals, double-spends, pool rules, transaction size, ephemeral shapes),
/// `ProposalNotSupported`, `AccountIdNotRecognized`, `KeyNotRecognized`,
/// `AccountCannotSpend`, `BalanceError(_)`, `Builder(_)`, `Payment(_)`,
/// `UnsupportedChangeType(_)`, `NoSupportedReceivers(_)`, `KeyNotAvailable(_)`,
/// `NoteMismatch(_)`, `Address(_)`, `AddressNotRecognized(_)`, both
/// `ExpiryHeight*` shapes. `DataSource`, `ScanRequired` and `InsufficientFunds`
/// never reach the question — the mapper types them first.
///
/// **The wildcard (`Pczt` under a feature this crate does not enable, and any
/// variant a future pin adds) is NOT transient:** "try again in a moment" for a
/// shape nobody has classified could loop a user forever, while "check the
/// details" promises nothing about time. A new variant lands here as
/// deterministic until someone reads its doc — the registry row for this
/// function is where a pin bump goes to check that.
fn propose_refusal_is_transient<DE, CE, SE, FE, ChE, N>(
    e: &zcash_client_backend::data_api::error::Error<DE, CE, SE, FE, ChE, N>,
) -> bool {
    use zcash_client_backend::data_api::error::Error;
    use zcash_client_backend::proposal::ProposalError;
    matches!(
        e,
        Error::CommitmentTree(_)
            | Error::Proposal(ProposalError::AnchorNotFound(_))
            | Error::Proposal(ProposalError::InputsLocked(_))
    )
}

/// Every string [`propose_err_variant`] can return — the CLOSED static vocabulary
/// the `code` field carries on a `wallet.propose_refused` event (phase-3 P3-2).
/// `tracing_guard::first_violation` exempts a `code` VALUE from the §5.4
/// forbidden-substring scan only when it is exactly one of these: five of them
/// (`address`, `address_not_recognized`, `key_not_recognized`,
/// `key_not_available`, `balance_error`) contain a forbidden token as a
/// substring, and without the exemption the first guard driving those arms
/// would red on a benign static label — a gate/production mismatch, not a leak
/// (the Batch C code reviewer's MEDIUM). Membership is exact-string, on the
/// `code` field only: a real address, a word with a tail, a different field
/// still trips (`tracing_guard::tests`). **The match below and this list move
/// together** — `propose_err_variant_names_are_all_in_the_static_set` drives
/// every constructible variant through the match and asserts membership, and
/// the registry row for it is where a pin bump goes to add both. It was
/// test-only while the scanner it feeds was; since an earlier revision the scanner's per-field
/// predicate ships (`tracing_guard::field_is_loggable`, enforced at runtime by
/// the bridge's device-log layer), so the set ships with it — without it a
/// shipped log would withhold `code` on exactly the five refusals above.
pub(crate) const PROPOSE_ERR_VARIANT_CODES: &[&str] = &[
    "data_source",
    "commitment_tree",
    "note_selection",
    "change",
    "proposal",
    "proposal_not_supported",
    "account_id_not_recognized",
    "key_not_recognized",
    "account_cannot_spend",
    "balance_error",
    "insufficient_funds",
    "scan_required",
    "builder",
    "payment",
    "unsupported_change_type",
    "no_supported_receivers",
    "key_not_available",
    "note_mismatch",
    "address",
    "address_not_recognized",
    "expiry_height_below_target_height",
    "expiry_height_conflicts_with_canonical_crossing",
    "other",
];

/// The NAME of an upstream propose error's variant, and nothing else — no payload,
/// so nothing an endpoint or a user supplied can reach a log line through it
/// (§5.4). Every variant the pinned crate exposes to this feature set is named;
/// the enum is `#[non_exhaustive]` upstream, so a wildcard is REQUIRED and a
/// variant a future pin adds logs as `"other"` until it is named here — the
/// registry row for this line is where a pin bump goes to check that. Every
/// return value is a member of `PROPOSE_ERR_VARIANT_CODES`.
fn propose_err_variant<DE, CE, SE, FE, ChE, N>(
    e: &zcash_client_backend::data_api::error::Error<DE, CE, SE, FE, ChE, N>,
) -> &'static str {
    use zcash_client_backend::data_api::error::Error;
    match e {
        Error::DataSource(_) => "data_source",
        Error::CommitmentTree(_) => "commitment_tree",
        Error::NoteSelection(_) => "note_selection",
        Error::Change(_) => "change",
        Error::Proposal(_) => "proposal",
        Error::ProposalNotSupported => "proposal_not_supported",
        Error::AccountIdNotRecognized => "account_id_not_recognized",
        Error::KeyNotRecognized => "key_not_recognized",
        Error::AccountCannotSpend => "account_cannot_spend",
        Error::BalanceError(_) => "balance_error",
        Error::InsufficientFunds { .. } => "insufficient_funds",
        Error::ScanRequired => "scan_required",
        Error::Builder(_) => "builder",
        Error::Payment(_) => "payment",
        Error::UnsupportedChangeType(_) => "unsupported_change_type",
        Error::NoSupportedReceivers(_) => "no_supported_receivers",
        Error::KeyNotAvailable(_) => "key_not_available",
        Error::NoteMismatch(_) => "note_mismatch",
        Error::Address(_) => "address",
        Error::AddressNotRecognized(_) => "address_not_recognized",
        Error::ExpiryHeightBelowTargetHeight { .. } => "expiry_height_below_target_height",
        Error::ExpiryHeightConflictsWithCanonicalCrossing { .. } => {
            "expiry_height_conflicts_with_canonical_crossing"
        }
        // `Error::Pczt` exists only under the backend's `pczt` feature, which this
        // crate does not enable, and the enum is `#[non_exhaustive]` upstream: the
        // wildcard is what the compiler demands, and it is the one arm that says
        // less than it could.
        _ => "other",
    }
}

/// Build the typed `InsufficientFunds` with the honest "wait for confirmations"
/// enrichment. `pending_incoming` comes from the SAME `fold_account_balances`
/// mapping (the SSOT) the balance uses — never a separately-derived number; a DB
/// fault reading it is the corruption door, not a fabricated half-error.
fn insufficient_funds<DbT: WalletRead<Error: ClassifyStoreFault>>(
    wdb: &DbT,
    available: zcash_protocol::value::Zatoshis,
    required: zcash_protocol::value::Zatoshis,
) -> WalletError {
    let available = match zatoshis_from_protocol(available) {
        Ok(z) => z,
        Err(e) => return e,
    };
    let required = match zatoshis_from_protocol(required) {
        Ok(z) => z,
        Err(e) => return e,
    };
    let pending_incoming = match read_pending_incoming(wdb) {
        Ok(z) => z,
        Err(e) => return e,
    };
    WalletError::InsufficientFunds {
        available,
        required,
        pending_incoming,
    }
}

/// The pending-incoming (detected, below spendable depth) total, read from the SAME
/// spendable-policy summary + SSOT fold as [`crate::account`]'s balance reader, so
/// the "still confirming" figure the host shows can never disagree with the balance.
fn read_pending_incoming<DbT: WalletRead<Error: ClassifyStoreFault>>(
    wdb: &DbT,
) -> Result<Zatoshis, WalletError> {
    let Some(summary) = wdb
        .get_wallet_summary(account::spendable_policy())
        .map_err(|e| e.into_store_fault())?
    else {
        return Ok(Zatoshis::ZERO);
    };
    Ok(account::fold_account_balances(summary.account_balances().values())?.pending_incoming)
}

/// The recognised shape of a ZIP-320 (TEX) two-step proposal, returned by
/// [`is_zip320_two_step`]. Carries the index — within step0's `proposed_change()` — of the
/// single ephemeral transparent output that step1 (tx1) consumes, so callers that key on the
/// ephemeral hop (the `change_zat` exclusion below; the slice-B multi-txid persistence) locate
/// it ONCE from the SSOT predicate instead of re-deriving it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Zip320Shape {
    /// Index of the lone `is_ephemeral()` change output in step0's `proposed_change()`.
    pub(crate) ephemeral_change_index: usize,
}

/// SSOT shape predicate for a ZIP-320 / TEX two-step proposal (§3.2i-2 pt 1).
///
/// With `transparent-inputs` ON (ADR-0528) `propose_transfer` AUTO-builds a two-step
/// ephemeral proposal for a TEX recipient (`zcash_client_backend` ≥ 0.13, no special flag):
/// step0 (tx0) unshields to a wallet-controlled ephemeral transparent address, step1 (tx1)
/// spends that ephemeral output to the TEX destination — the ZIP-320 MUST that stops the
/// exchange from linking the deposit to the wallet's other activity. This predicate POSITIVELY
/// asserts that exact engine output and returns `Some(Zip320Shape)`; ANY other multi-step shape
/// returns `None`, and the caller fails closed (`ProposeFailed`). A bare `steps().len() == 2`
/// check is deliberately NOT sufficient — a future engine could synthesise a different two-step
/// — so every structural invariant the money-safety + display math rely on is checked here:
///
/// - exactly two steps;
/// - **step0** spends shielded notes (`shielded_inputs().is_some()`), has NO transparent
///   inputs and NO prior-step inputs, is not a shielding step, and produces EXACTLY ONE
///   ephemeral transparent change output. It MAY also carry a shielded change (a partial-amount
///   or mixed send leaves one) — that is normal; we require exactly one *ephemeral* among them;
/// - **step1** has NO shielded inputs and NO transparent inputs (the ephemeral is a
///   `prior_step_inputs` [`StepOutput`], NOT a `transparent_input` — `input_selection.rs`), is
///   not a shielding step, its SOLE prior input is exactly step0's ephemeral **Change** output
///   (bound to `Change`, never a `Payment`, so "Σ all payments" cannot double-count the hop),
///   and every step1 payment is to the **transparent** pool (the TEX destination).
///
/// The all-transparent check is **step1-only**: a legal MIXED send (TEX + a shielded recipient)
/// keeps the shielded recipient as a step0 payment, so checking step0 would wrongly reject it.
pub(crate) fn is_zip320_two_step<FeeRuleT, N>(
    proposal: &Proposal<FeeRuleT, N>,
) -> Option<Zip320Shape> {
    let steps = proposal.steps();
    if steps.len() != 2 {
        return None;
    }
    // With exactly two steps, `first()` is step index 0 and `last()` is step index 1 — so the
    // literal `0` in the `StepOutput` reference below is step0's position (the engine binds tx1's
    // ephemeral input to `StepOutput::new(0, Change(_))`; `input_selection.rs`).
    let step0 = steps.first();
    let step1 = steps.last();

    // ── step0: the shielded → ephemeral-transparent unshield ──
    if step0.shielded_inputs().is_none()
        || !step0.transparent_inputs().is_empty()
        || !step0.prior_step_inputs().is_empty()
        || step0.is_shielding()
    {
        return None;
    }
    // Exactly one ephemeral transparent change output (the hop tx1 consumes). Any other
    // (shielded) change is allowed and ignored here.
    let mut ephemeral_change_index = None;
    for (idx, cv) in step0.balance().proposed_change().iter().enumerate() {
        if cv.is_ephemeral() {
            if ephemeral_change_index.is_some() {
                return None; // > 1 ephemeral output ⇒ not the ZIP-320 shape
            }
            ephemeral_change_index = Some(idx);
        }
    }
    let ephemeral_change_index = ephemeral_change_index?;

    // ── step1: the ephemeral → TEX forward ──
    if step1.shielded_inputs().is_some()
        || !step1.transparent_inputs().is_empty()
        || step1.is_shielding()
    {
        return None;
    }
    // The SOLE input is step0's ephemeral CHANGE output (never a Payment).
    let expected = StepOutput::new(0, StepOutputIndex::Change(ephemeral_change_index));
    match step1.prior_step_inputs() {
        [only] if *only == expected => {}
        _ => return None,
    }
    // ≥1 payment and every one is to the transparent pool (the TEX destination). A step1
    // with no payment, or any non-transparent payment, is not a TEX forward.
    let pools = step1.payment_pools();
    let payments = step1.transaction_request().payments();
    if payments.is_empty() {
        return None;
    }
    for idx in payments.keys() {
        match pools.get(idx) {
            Some(PoolType::Transparent) => {}
            _ => return None,
        }
    }

    Some(Zip320Shape {
        ephemeral_change_index,
    })
}

/// Summarize a retained proposal into the [`SendProposal`] display DTO (pure; no IO,
/// no keys, no network). Every money figure is the audited proposal's own — fee from
/// `TransactionBalance::fee_required`, change from `proposed_change`, recipient
/// amounts from the step's `zip321` request, and the per-output pool from the
/// engine's `payment_pools` (Rule Zero — we never re-derive a pool from an address).
///
/// **Multi-step (TEX / ZIP-320).** With `transparent-inputs` ON (ADR-0528) the engine
/// auto-synthesises a TWO-step ephemeral proposal for a TEX recipient (§3.2i-2): tx0
/// unshields to a wallet-controlled ephemeral t-address, tx1 forwards it to the TEX
/// destination. [`is_zip320_two_step`] is the SSOT predicate that POSITIVELY recognises that
/// exact shape; ANY other multi-step proposal is a structural surprise ⇒ fail closed
/// `ProposeFailed` (never a silently mis-totaled DTO). The display math is double-count-safe
/// BY CONSTRUCTION: the ephemeral hop lives in `proposed_change` (flagged `is_ephemeral()`),
/// NEVER in a payment, so the recipient total = Σ all-step payments is exact for both the
/// single- and the two-step shape (the total loop is UNCHANGED). `change_zat` MUST exclude the
/// ephemeral hop, else step0's in-transit output (≈ recipient + tx1 fee) misreports as "change
/// returned to you" — excluded exactly as [`summarize_shield`] does for its defensive case.
///
/// GATE-REMOVAL LANDED (2e-2b-v-5, §251): a TEX two-step now PROPOSES, SIGNS, and DRAINS in
/// production — the slice-A gates at `Wallet::propose`, [`create_signed_core`], and
/// [`prepare_queued`] are gone (their recovery preconditions — the multi-txid persistence +
/// re-broadcast machine, the leaked-ephemeral sweep, the stranded surface + cancel escape hatch —
/// all landed first, 2e-2b-i…iv + v-1…v-4). This summary sets
/// [`is_two_step_tex`](SendProposal::is_two_step_tex) from the SSOT predicate so the host's
/// post-send outcome copy is honest about a partial broadcast (funds in-motion on an ephemeral
/// address vs an independent saved-for-retry tx).
pub(crate) fn summarize<N>(
    proposal: &Proposal<StandardFeeRule, N>,
    proposal_id: u64,
    binding: SpendBinding,
) -> Result<SendProposal, WalletError> {
    // Accept a single step, OR the engine's ZIP-320 two-step (the TEX ephemeral pair). Any
    // OTHER multi-step shape is a structural surprise ⇒ fail closed rather than risk a
    // double-counted total.
    if proposal.steps().len() > 1 && is_zip320_two_step(proposal).is_none() {
        return Err(WalletError::ProposeFailed);
    }

    let mut fee = Zatoshis::ZERO;
    let mut change = Zatoshis::ZERO;
    let mut recipients_total = Zatoshis::ZERO;
    let mut has_transparent_recipient = false;
    let mut steps = Vec::with_capacity(proposal.steps().len());
    // FR-46: the one address every payment names, and whether a second one was seen.
    let mut recipient = None;
    let mut one_recipient = true;

    for step in proposal.steps() {
        let balance = step.balance();
        fee = checked_add(fee, zatoshis_from_protocol(balance.fee_required())?)?;
        for cv in balance.proposed_change() {
            // The ephemeral transparent hop (step0's tx0 output that tx1 consumes) is
            // in-transit money, NOT change returned to the user — exclude it so `change_zat`
            // never misreports it as change (mirrors `summarize_shield`). A single-step
            // proposal has no ephemeral change, so this is a no-op there.
            if cv.is_ephemeral() {
                continue;
            }
            change = checked_add(change, zatoshis_from_protocol(cv.value())?)?;
        }

        let pools = step.payment_pools();
        let mut recipients = Vec::new();
        for (idx, payment) in step.transaction_request().payments() {
            let amount = payment.amount().ok_or(WalletError::SendAmountRequired)?;
            let amount = zatoshis_from_protocol(amount)?;
            recipients_total = checked_add(recipients_total, amount)?;
            match recipient {
                None => recipient = Some(payment.recipient_address()),
                Some(seen) if seen == payment.recipient_address() => {}
                Some(_) => one_recipient = false,
            }
            // The audited engine's own per-output pool decision (never re-derived).
            // A payment index with no pool entry would be an upstream contract break
            // ⇒ fail-closed `ProposeFailed`, never a panic / silent default.
            let pool =
                OutputPool::from_pool_type(*pools.get(idx).ok_or(WalletError::ProposeFailed)?);
            if pool == OutputPool::Transparent {
                has_transparent_recipient = true;
            }
            recipients.push(ProposalRecipient {
                pool,
                amount_zat: amount.zat(),
            });
        }
        steps.push(ProposalStep { recipients });
    }

    let total = checked_add(recipients_total, fee)?;
    Ok(SendProposal {
        proposal_id,
        binding,
        total_zat: total.zat(),
        fee_zat: fee.zat(),
        change_zat: change.zat(),
        steps,
        target_height: u32::from(proposal.min_target_height()),
        has_transparent_recipient,
        // The SSOT shape signal (NOT `steps.len() > 1`): the positive ZIP-320 predicate, so a
        // future engine multi-step shape can never silently mislabel the partial-terminal copy.
        // The fail-closed guard above already rejected any non-ZIP-320 multi-step, so for a
        // surviving proposal this is exactly the two-step ⟺ `steps.len() > 1`.
        is_two_step_tex: is_zip320_two_step(proposal).is_some(),
        is_shield: false,
        // The money-safety signals need the spendable balance + the recipient address, which the
        // pure summarizer does not see; `Wallet::propose` fills them after this returns. Default to
        // the no-friction values so a missed wire is fail-safe (no false confirm, no false note).
        large_send: None,
        self_send: false,
        // FR-46: the recipient total is already exact for both shapes (the ephemeral hop is
        // change, never a payment), so with one recipient it IS that recipient's figure.
        single_recipient_zat: (one_recipient && recipient.is_some())
            .then_some(recipients_total.zat()),
    })
}

/// Checked zatoshi sum for the DTO roll-up. Overflow past max supply is impossible
/// for a real proposal (it cannot move more than the money supply) ⇒ `ProposeFailed`
/// (honest "couldn't prepare"), never a silent wrap.
fn checked_add(acc: Zatoshis, value: Zatoshis) -> Result<Zatoshis, WalletError> {
    acc.checked_add(value).ok_or(WalletError::ProposeFailed)
}

// ── inc-2d-shield (Recv-3): PROPOSE the shield of transparent funds ──────────────
//
// The companion to Recv-2: move detected transparent funds into the shielded pool. Rule
// Zero — the audited upstream `propose_shielding` selects the UTXOs, computes the ZIP-317
// fee, and builds the shielded change output; our code is the threshold gate + the display
// DTO + the same one-shot token the send path consumes. DETERMINISTIC: no keys, no proofs,
// no network, no DB writes (read-only). A shield is PRIVACY-POSITIVE (public → shielded),
// so the §5.1 de-shield disclosure is inverted (it does not apply).

/// Propose shielding ALL spendable transparent funds at `from_addrs` into the wallet's own
/// shielded pool (§3.3a Recv-3), over the audited upstream `propose_shielding`. `from_addrs` is the
/// SCOPED source set: index-0 alone for an ordinary Recv-3 shield, or `{ index-0 } ∪ { active swap
/// destinations }` to also shield an IntoZec delivery (§3.3b D3 / ADR-0530, IZ-1b — the engine
/// selects whichever of them carry spendable UTXOs; an address with none contributes nothing).
/// Returns:
/// - `Ok(Some(proposal))` when the GROSS transparent amount being shielded ≥
///   `SHIELDING_THRESHOLD_ZAT` (0.001 ZEC) — the engine's OWN gate is
///   `balance.total() >= shielding_threshold`, and for a shield `balance.total()` is the
///   GROSS (Σ transparent inputs = net-shielded + fee), NOT the net-of-fee amount;
/// - `Ok(None)` when it is below the threshold OR there is nothing spendable to shield yet
///   (the engine's `InsufficientFunds` — an honest "not worth a fee yet", NOT an error; the
///   host hides the shield action); and
/// - a typed `WalletError` for a real fault (corruption ⇒ `StoreCorrupt`, not synced far
///   enough ⇒ `ProposalStale`).
///
/// Money-decision-only, exactly like [`propose_core`]: no spending key, no proof, no
/// network, no DB write. Generic over DB + params so the funded `data_api::testing` harness
/// and the live `Network` wallet drive the SAME code. Wiring mirrors `propose_core`:
/// `GreedyInputSelector` + the standard ZIP-317 change strategy with an Orchard fallback
/// change pool (shield INTO Orchard — the modern pool keeps the new note in the larger
/// anonymity set, §2.3), the [`account::spendable_policy`] SSOT (so the UTXOs the balance
/// calls spendable and the ones shield selects can never disagree), and
/// [`CoinbaseFilter::AllTransparentOutputs`] (shield every received transparent output, not
/// only coinbase). The engine auto-excludes dust inputs.
pub(crate) fn propose_shield_core<DbT, P>(
    wdb: &mut DbT,
    params: &P,
    account_id: <DbT as InputSource>::AccountId,
    from_addrs: &[TransparentAddress],
) -> Result<Option<ShieldProposal>, WalletError>
where
    DbT: WalletRead + WalletWrite + InputSource<Error = <DbT as WalletRead>::Error>,
    <DbT as WalletRead>::Error: ClassifyStoreFault,
    <DbT as InputSource>::NoteRef: Copy + Eq + Ord,
    P: Parameters + Clone,
{
    // `SHIELDING_THRESHOLD_ZAT` is `i64` to match the codebase's money domain (`MAX_MONEY_ZAT`
    // is also `i64`; `money::Zatoshis` is i64-backed) — NOT a lone `u64`. The `as u64` is the
    // i64→u64 conversion ONLY at this upstream-API boundary (`const_from_u64`); it is total
    // here because the value is a frozen, non-negative, in-range constant (100_000 ≪ MAX_MONEY),
    // proven at compile time by the `const` (no runtime expect, no wrap — the value is positive).
    const THRESHOLD: zcash_protocol::value::Zatoshis =
        zcash_protocol::value::Zatoshis::const_from_u64(SHIELDING_THRESHOLD_ZAT as u64);
    propose_shield_with_threshold(wdb, params, account_id, from_addrs, THRESHOLD)
}

/// §3.2i-2 2e-2b-v-2 — propose a SWEEP of stranded funds on a wallet-controlled EPHEMERAL one-time
/// address into the wallet's own shielded (Orchard) pool, at the ECONOMIC floor (maintainer — recover
/// EVERYTHING sweepable) rather than [`propose_shield_core`]'s convenience 0.001-ZEC threshold. Same
/// audited `propose_shielding` path, same Orchard destination, same per-call non-resurrection (a
/// re-sweep after the prior swept tx mines finds the UTXOs SPENT ⇒ `Ok(None)`); ONLY the gate differs —
/// [`EPHEMERAL_SWEEP_THRESHOLD_ZAT`](crate::constants::EPHEMERAL_SWEEP_THRESHOLD_ZAT) (1 zat) admits any
/// ≥ 1-zat ephemeral and delegates the REAL floor to the engine's ZIP-317 fee economics (a genuinely-
/// uneconomic ephemeral reports `Change(InsufficientFunds)` ⇒ `Ok(None)`; aggregate sub-marginal-fee
/// dust that COLLECTIVELY clears the fee IS swept). `from_addrs` is the SINGLE ephemeral being swept —
/// the caller never batches several ephemerals into ONE tx (per-ephemeral isolation, so a sweep never
/// clusters the wallet's one-time-address set on-chain). Privacy-POSITIVE (transparent → shielded), so
/// the §5.1 de-shield disclosure is INVERTED — it does not apply. Money-decision-only, like
/// [`propose_shield_core`]: no spending key, no proof, no network, no DB write.
pub(crate) fn propose_sweep_core<DbT, P>(
    wdb: &mut DbT,
    params: &P,
    account_id: <DbT as InputSource>::AccountId,
    from_addrs: &[TransparentAddress],
) -> Result<Option<ShieldProposal>, WalletError>
where
    DbT: WalletRead + WalletWrite + InputSource<Error = <DbT as WalletRead>::Error>,
    <DbT as WalletRead>::Error: ClassifyStoreFault,
    <DbT as InputSource>::NoteRef: Copy + Eq + Ord,
    P: Parameters + Clone,
{
    // The ECONOMIC floor (1 zat): the explicit gate admits any ≥ 1-zat ephemeral and the engine's
    // ZIP-317 change strategy is the REAL floor (uneconomic ⇒ `Ok(None)`). `const_from_u64` is total
    // here — `EPHEMERAL_SWEEP_THRESHOLD_ZAT` is a frozen, positive, in-range constant (1 ≪ MAX_MONEY).
    const THRESHOLD: zcash_protocol::value::Zatoshis =
        zcash_protocol::value::Zatoshis::const_from_u64(
            crate::constants::EPHEMERAL_SWEEP_THRESHOLD_ZAT as u64,
        );
    propose_shield_with_threshold(wdb, params, account_id, from_addrs, THRESHOLD)
}

/// Shared body for [`propose_shield_core`] (the convenience shield) and [`propose_sweep_core`] (the
/// recovery sweep): propose shielding the spendable transparent funds at `from_addrs` into Orchard
/// over the audited `propose_shielding`, gated at `threshold` (the engine's own
/// `balance.total() >= threshold`). DRY — the two callers differ ONLY in that gate. Returns `Ok(None)`
/// below the threshold OR when nothing spendable can cover the fee; a real fault maps via
/// [`map_propose_err`]. Same `GreedyInputSelector` + ZIP-317 Orchard-change wiring +
/// [`account::spendable_policy`] SSOT + `TransparentOutputFilter::All` as `propose_core`; the engine
/// auto-excludes dust inputs.
fn propose_shield_with_threshold<DbT, P>(
    wdb: &mut DbT,
    params: &P,
    account_id: <DbT as InputSource>::AccountId,
    from_addrs: &[TransparentAddress],
    threshold: zcash_protocol::value::Zatoshis,
) -> Result<Option<ShieldProposal>, WalletError>
where
    DbT: WalletRead + WalletWrite + InputSource<Error = <DbT as WalletRead>::Error>,
    <DbT as WalletRead>::Error: ClassifyStoreFault,
    <DbT as InputSource>::NoteRef: Copy + Eq + Ord,
    P: Parameters + Clone,
{
    use zcash_client_backend::data_api::error::Error;
    use zcash_client_backend::fees::ChangeError;

    let input_selector = GreedyInputSelector::new();
    let change_strategy = standard::SingleOutputChangeStrategy::new(
        StandardFeeRule::Zip317,
        // No change memo — the shielded output is our own, never carries a memo.
        None,
        // Shield INTO Orchard (the same fallback-change posture as `propose_core`,
        // and the same S8 note applies: this is the fallback the strategy consults,
        // not the pool the engine is obliged to use post-turnstile).
        ShieldedPool::Orchard,
        DustOutputPolicy::default(),
    );
    let result = propose_shielding::<_, _, _, _, std::convert::Infallible>(
        wdb,
        params,
        &input_selector,
        &change_strategy,
        threshold,
        // The SCOPED transparent source set — for a shield: index-0 (Recv-2a) ∪ active swap
        // destinations; for a sweep: the single ephemeral. The engine selects the spendable UTXOs
        // across all of them; the GROSS total it compares to `threshold` is their union.
        from_addrs,
        account_id,
        // THE spendable SSOT — the same policy the balance reader + `propose_core` use.
        account::spendable_policy(),
        // 0.24.0's rename of `TransparentOutputFilter::All` — same meaning. Shielding
        // draws from every spendable transparent output; `NonCoinbaseOnly` would
        // silently skip coinbase funds, and `propose_shielding_coinbase` is the
        // separate path for those.
        CoinbaseFilter::AllTransparentOutputs,
        // No input locking — same reasoning as `propose_core`.
        None,
    );
    match result {
        Ok(proposal) => Ok(Some(proposal)),
        // Below the explicit §7 threshold gate (`balance.total() < threshold`) — an
        // honest no-op, not an error (the host hides the shield CTA / the sweep skips this ephemeral).
        Err(Error::InsufficientFunds { .. }) => Ok(None),
        // NO spendable transparent inputs to even cover the fee (every UTXO already spent, or
        // only dust remains): the change strategy reports `Change(InsufficientFunds)`. The SAME
        // honest "nothing worth shielding yet" no-op — and exactly the post-shield state that
        // makes the non-resurrection check return None (the UTXO is spent ⇒ no inputs left).
        Err(Error::Change(ChangeError::InsufficientFunds { .. })) => Ok(None),
        // The mutable borrow for `propose_shielding` is released here, so the mapper can
        // re-read `wdb`. Both InsufficientFunds flavors are handled above, so every OTHER
        // fault maps identically to the transfer path (DataSource ⇒ classified,
        // ScanRequired ⇒ ProposalStale, a transient shape ⇒ ProposeTransient — empty on
        // this path at this pin, see `propose_refusal_is_transient` — else ⇒
        // ProposeFailed). The shield sheet renders ProposeTransient too
        // (`classifyShieldPrepareFailure`, the Batch C arch pass).
        Err(e) => Err(map_propose_err(wdb, e)),
    }
}

/// Summarize a SHIELD proposal into the [`SendProposal`] display DTO (pure; no IO/keys/net).
/// A shield has NO external recipient (the upstream request is `TransactionRequest::empty()`):
/// its inputs are transparent UTXOs and its lone output is a shielded change note back to the
/// wallet. So the honest display numbers are the engine's OWN:
/// - `total_zat`  = GROSS transparent being shielded (Σ the step's transparent inputs),
/// - `fee_zat`    = the ZIP-317 fee (`TransactionBalance::fee_required`),
/// - `change_zat` = NET that lands shielded (Σ the non-ephemeral `proposed_change` = gross − fee),
///
/// and the single `steps` recipient is the wallet's OWN shielded output, whose pool is the
/// engine's `ChangeValue::output_pool` (never re-derived — Rule Zero). `is_shield = true`,
/// `has_transparent_recipient = false` (privacy-POSITIVE; §5.1 de-shield does not apply).
///
/// A multi-step shield cannot arise for a single t-address with no ephemeral funds (the
/// upstream builds `Proposal::single_step`); a >1-step proposal — or one with no shielded
/// output — is a structural surprise ⇒ fail-closed `ProposeFailed`, never a mis-totaled DTO.
pub(crate) fn summarize_shield<N>(
    proposal: &Proposal<StandardFeeRule, N>,
    proposal_id: u64,
    binding: SpendBinding,
) -> Result<SendProposal, WalletError> {
    if proposal.steps().len() != 1 {
        return Err(WalletError::ProposeFailed);
    }
    let step = proposal.steps().first();
    let balance = step.balance();
    let fee = zatoshis_from_protocol(balance.fee_required())?;

    // Gross = Σ the transparent UTXOs being shielded (the engine's selected inputs).
    let mut gross = Zatoshis::ZERO;
    for input in step.transparent_inputs() {
        gross = checked_add(gross, zatoshis_from_protocol(input.value())?)?;
    }

    // Net = Σ the shielded change (the self-output). Ephemeral change cannot arise for a
    // transparent→shielded shield, but exclude it defensively so `change_zat` is exactly the
    // user's shielded gain. Each non-ephemeral change carries the engine's own output pool.
    let mut net = Zatoshis::ZERO;
    let mut recipients = Vec::new();
    for cv in balance.proposed_change() {
        if cv.is_ephemeral() {
            continue;
        }
        let value = zatoshis_from_protocol(cv.value())?;
        net = checked_add(net, value)?;
        recipients.push(ProposalRecipient {
            pool: OutputPool::from_pool_type(cv.output_pool()),
            amount_zat: value.zat(),
        });
    }
    // A shield with no shielded output gains the user nothing (it could not clear the
    // threshold) — a structural surprise ⇒ fail-closed, never a zero-gain shield DTO.
    if recipients.is_empty() {
        return Err(WalletError::ProposeFailed);
    }

    Ok(SendProposal {
        proposal_id,
        binding,
        total_zat: gross.zat(),
        fee_zat: fee.zat(),
        change_zat: net.zat(),
        steps: vec![ProposalStep { recipients }],
        target_height: u32::from(proposal.min_target_height()),
        has_transparent_recipient: false,
        is_shield: true,
        // A shield is a single-step self-shield (transparent UTXOs → one shielded output), never a
        // ZIP-320 two-step — so a partial here is an ordinary saved-for-retry tx, not in-motion.
        is_two_step_tex: false,
        // A shield is a self-output, not a send to a recipient: no large-amount confirm (the user
        // isn't parting with funds) and no self-send note (the "recipient" is always the wallet).
        large_send: None,
        self_send: false,
        // FR-46: no external recipient, so no recipient amount.
        single_recipient_zat: None,
    })
}

// ── inc-2d-2: CREATE + SIGN (the first internal half of the public `send`) ──────
//
// Build, prove, sign, and PERSIST the proposed transaction(s) over the audited
// `create_proposed_transactions` (Rule Zero — we write no builder, no prover, no
// signer; the engine does proofs + signatures + the §6.3 persist-before-submit via
// `store_transactions_to_be_sent`). DETERMINISTIC inputs, money-critical. The
// production prover-SOURCE (the ~50 MB Sapling Groth16 params) is INJECTED, so the
// packaging decision (bundle vs host-asset vs verified-download — §3.2h inc-2d-2 AS
// BUILT) is a wiring choice, not a core one; only the test names a concrete prover.

/// Re-anchor guard (§3.2h re-anchor obligation — the discharged suspend tripwire).
///
/// **NOT ALWAYS 40 BLOCKS ANY MORE (`zcash_client_backend` 0.24.0).** For a
/// CANONICAL ZIP-318 CROSSING — a single payment of a `{1,2,5}×10^k` denomination
/// at a height where NU6.3 is active, which on mainnet is *now* and is the shape a
/// human typing "1 ZEC" produces — upstream replaces the builder's expiry with
/// `zip318::expiry_height(target)`: the ZIP-318 rolling expiry, **34,561 to 69,120
/// blocks (≈30–60 days)** above the target rather than 40 blocks (≈50 minutes).
/// That is deliberate and privacy-correct (every crossing in a modulus period
/// shares one expiry; a per-transaction expiry would single it out), and this
/// guard is unaffected — it compares ANCHOR drift, not expiry. What IS affected is
/// every downstream statement about how fast an unbroadcast send self-heals; see
/// `note_spendable` and `docs/plan/ironwood-phase-b.md` §3.
///
/// For every other send the builder still sets `min_target_height +
/// DEFAULT_TX_EXPIRY_DELTA` (40 blocks). Either way it NEVER consults the live tip, and
/// `create_proposed_transactions` errors only on a *pruned* anchor — never a merely
/// *stale* one. So a proposal resumed after a device suspend (the monotonic
/// `PROPOSAL_TTL_SECS` clock PAUSED while the chain advanced) could otherwise build a
/// BORN-EXPIRED tx (funds locked to a dead expiry height until it un-mines — §6.2).
/// This runs FIRST, before any key is derived: it compares the proposal's anchor
/// (`min_target_height`) against the wallet's recorded chain tip (`chain_height()`)
/// and rejects `ProposalStale` (re-`propose`) when the chain has advanced past
/// `min_target_height + PROPOSAL_ANCHOR_DRIFT_MAX_BLOCKS` — keeping a signed tx ≥ 20
/// blocks of mining headroom. A wallet with no recorded tip (`None`, not synced far
/// enough) is also `ProposalStale` (sync first). A DB fault is the corruption door.
///
/// This catches the realistic suspend case (post-resume sync recorded the new tip);
/// the resume-before-sync residual (the DB tip is itself still stale, so no drift is
/// visible here) degrades to the engine's expiry + the §6.2 resubmission/expiry
/// machinery — an honest expiry, never a fund loss.
fn assert_proposal_anchor_fresh<DbT, N>(
    wdb: &DbT,
    proposal: &Proposal<StandardFeeRule, N>,
) -> Result<(), WalletError>
where
    DbT: WalletRead<Error: ClassifyStoreFault>,
{
    let target = u32::from(proposal.min_target_height());
    let tip = wdb
        .chain_height()
        .map_err(|e| e.into_store_fault())?
        // No recorded chain tip ⇒ not synced far enough to anchor a send safely.
        .ok_or(WalletError::ProposalStale)?;
    let tip = u32::from(tip);
    if tip > target.saturating_add(PROPOSAL_ANCHOR_DRIFT_MAX_BLOCKS) {
        return Err(WalletError::ProposalStale);
    }
    Ok(())
}

/// Build + prove + sign + persist the proposed transaction(s) for `proposal`, over the
/// audited `create_proposed_transactions`. Generic over the DB + network EXACTLY like
/// [`propose_core`] (the funded `LocalNetwork` harness and the live `Network` wallet
/// run the SAME code), and over the injected Sapling spend/output provers.
///
/// Order is load-bearing: (1) the [`assert_proposal_anchor_fresh`] re-anchor guard
/// rejects a suspend-stale proposal BEFORE any key material is touched; (2) the engine
/// builds, proves, signs, and PERSISTS every tx via `store_transactions_to_be_sent`
/// BEFORE returning the ids — so a kill between this return and the broadcast (inc-2d-3)
/// loses no bookkeeping (the resubmission machinery re-submits unexpired unmined txs on
/// the next sync — §6.3). `OvkPolicy::Sender` so the sender can later decrypt its own
/// outputs for history (§3.2h decision 1).
///
/// **§4.2 USK CONFINEMENT.** `usk` is taken BY VALUE and consumed into
/// `SpendingKeys::from_unified_spending_key` here, then dropped at return — it is never
/// stored, never held across an `.await` (this whole function is synchronous; the caller
/// runs it inside `spawn_blocking`), and never placed in a `Debug`-reachable container.
/// The caller derives it transiently via [`crate::derivation::derive_spending_key`] and
/// moves it straight in. The un-zeroized residue is the documented §4.2(a) residual.
///
/// Reached in production through the public `send` (2d-3 landed); the test-only
/// `Wallet::sign_proposal` drives it directly over the harness prover (P3-7 measured
/// both, so no allow is needed).
pub(crate) fn create_signed_core<DbT, P, N, SP, OP>(
    wdb: &mut DbT,
    params: &P,
    usk: UnifiedSpendingKey,
    proposal: &Proposal<StandardFeeRule, N>,
    spend_prover: &SP,
    output_prover: &OP,
    // `ironwood-nu63-support.md` §3.2 — THE consensus-staleness gate, taken by
    // value so it cannot be forgotten. Every signature in the SDK passes
    // through this function, so requiring the permit HERE covers the
    // interactive send, the queued drain, the sweep and the reclaim-mint at
    // once, and the compiler rejects a fifth path that forgets to ask.
    // Unused at runtime by design: holding one IS the check.
    _permit: crate::consensus::SigningPermit,
) -> Result<Vec<TxId>, WalletError>
where
    DbT: WalletWrite + WalletCommitmentTrees,
    // The engine's create error is `<DbT as WalletRead>::Error`; `map_create_err` reads it for the
    // ZIP-320 ephemeral gap-limit ceiling (round-2 #7). Satisfied by the production + harness
    // `WalletDb` (`SqliteClientError: GapLimitProbe`); no other backend reaches this path.
    <DbT as WalletRead>::Error: GapLimitProbe + ClassifyStoreFault,
    P: Parameters + Clone,
    SP: SpendProver,
    OP: OutputProver,
{
    // (0) MULTI-STEP CREATE — slice-A gate REMOVED (2e-2b-v-5, §251); a recognised ZIP-320 TEX
    //     two-step now signs here (the interactive path wraps this create in
    //     `create_two_step_enrolled`, Option A; the queued path drains it through `drain_multi`,
    //     the load-bearing `mark_submitting`-before-create / `mark_sent_multi`-before-broadcast money
    //     order). All recovery preconditions are in place (re-broadcast machine, the typed
    //     `TexSendLimitReached` ceiling, the manual sweep + parked surface + cancel, 2e-2b-i…iv +
    //     v-1…v-4). The engine persists txs only on FULL success, so a sign fault here spends NO
    //     note — but NB (#315): for a two-step the engine reserves the ephemeral index BEFORE tx
    //     construction and never rolls it back, so a faulting two-step create still consumes one
    //     gap slot (why the queued drain counts attempts and caps them — `CappedParked`).
    //
    //     DEFENSIVE SHAPE GUARD (defense-in-depth, the LAST line before signing): the ONLY multi-step
    //     the recovery contract covers is the recognised ZIP-320 two-step. Both callers already
    //     fail-close a non-ZIP-320 multi-step (the interactive path via `summarize` before minting a
    //     token; the queued path via the matching guard in `prepare_queued`), but this is the final
    //     barrier — even if a future caller forgot, signing an unrecognised multi-step is structurally
    //     impossible here. A single-step send and the recognised two-step pass through; anything else
    //     fails closed `ProposeFailed` (the same structural "shape not handled" signal `summarize` uses).
    if proposal.steps().len() > 1 && is_zip320_two_step(proposal).is_none() {
        return Err(WalletError::ProposeFailed);
    }

    // (1) Re-anchor against the live tip BEFORE deriving/using any spend authority. The
    //     check is HEIGHT-based (not clock-based), and it is the ONLY freshness gate on this
    //     path — a device suspend DURING the proving below is latency-only (no clock/TTL is
    //     consulted after this point), so there is no frozen-monotonic-clock hazard here. Do
    //     NOT add a post-proving freshness re-check: it would re-introduce exactly the
    //     suspend-paused-TTL trap the §3.2h consume SAFETY-OBLIGATION warns against.
    assert_proposal_anchor_fresh(wdb, proposal)?;

    // (2) Create + prove + sign + persist. InputsErrT/ChangeErrT are pinned `Infallible`
    //     — create+sign runs neither input selection nor change selection (both already
    //     ran in propose), so the engine never constructs those error arms here.
    let result = create_proposed_transactions::<
        _,
        _,
        std::convert::Infallible,
        StandardFeeRule,
        std::convert::Infallible,
        _,
    >(
        wdb,
        params,
        spend_prover,
        output_prover,
        &SpendingKeys::from_unified_spending_key(usk),
        OvkPolicy::Sender,
        proposal,
        // No expiry override: take the builder's own expiry, derived from the
        // proposal's target height. Upstream REFUSES a caller-supplied expiry on a
        // canonical ZIP-318 crossing (`ExpiryHeightConflictsWithCanonicalCrossing`)
        // precisely because the rolling expiry is part of the shape that makes such
        // a transaction indistinguishable from its peers — a per-tx expiry would
        // single it out. The transaction VERSION is no longer passed here; it rides
        // on the proposal, chosen at propose time from the target height's branch.
        None,
    );
    // The `NonEmpty<TxId>` becomes a plain `Vec` (we never lose the txids; the broadcast
    // path — inc-2d-3 — iterates them). The USK + SpendingKeys drop here.
    Ok(result.map_err(map_create_err)?.into_iter().collect())
}

/// Probe a create/sign `DataSource` error for the ZIP-320 ephemeral-address gap-limit
/// ceiling WITHOUT pinning [`map_create_err`] to a concrete DB backend (the rest of
/// `send.rs` maps engine errors generic-over-`DE`, so the funded harness — whose
/// `WalletDb` IS the production `SqliteClientError` backend — and a synthetic unit-test
/// error both flow through one mapper). The production + harness `WalletDb` surfaces the
/// ceiling as [`SqliteClientError::ReachedGapLimit`]; any other backend (the
/// synthetic-error test's `()` DataSource) reports `false` via the default.
///
/// `pub(crate)` (not private): it appears in the `where`-bound of the `pub(crate)`
/// [`create_signed_core`] / [`prepare_queued`], so it must be at least as visible (the
/// `private_bounds` lint). It is an internal mapper detail — never part of the public API.
pub(crate) trait GapLimitProbe {
    fn reached_gap_limit(&self) -> bool {
        false
    }
}
impl GapLimitProbe for SqliteClientError {
    fn reached_gap_limit(&self) -> bool {
        // NARROWED to the `EPHEMERAL` scope (review HARDENING): the create/sign path's only
        // transparent reservation is the ZIP-320 ephemeral t-address (a TEX tx0 unshields to one
        // engine-owned ephemeral output), so the TEX in-flight ceiling is precisely a
        // `ReachedGapLimit(EPHEMERAL, _)`. A `ReachedGapLimit` on ANY OTHER transparent scope here
        // would be an unanticipated engine change, NOT a TEX ceiling — narrowing makes that fall
        // through to the conservative `StoreCorrupt` door (loud, investigated) rather than silently
        // mislabeling it "too many transfers confirming". `EPHEMERAL` is a frozen public ZIP-32
        // scope constant (`zcash_transparent::keys`), no worse to depend on than `ReachedGapLimit`
        // itself. The carried index is diagnostic only (§5.4 — dropped, never surfaced).
        matches!(
            self,
            SqliteClientError::ReachedGapLimit(scope, _) if *scope == TransparentKeyScope::EPHEMERAL
        )
    }
}
// Synthetic test backend: no gap-limit concept — the ceiling never fires (the default `false`).
impl GapLimitProbe for () {}
// Synthetic test backend has no SQLite store — a `DataSource` fault can't be a disk/IO
// condition, so it stays the fail-closed `StoreCorrupt` (#371).
impl ClassifyStoreFault for () {
    fn into_store_fault(self) -> WalletError {
        WalletError::StoreCorrupt
    }
}
// An uninhabited `DataSource` (the propose mapper's synthetic tests): no value, no fault.
impl ClassifyStoreFault for std::convert::Infallible {
    fn into_store_fault(self) -> WalletError {
        match self {}
    }
}

/// Map the upstream create+sign error to the SDK taxonomy (§3.2h decision 4 — reuse, no
/// collisions). The ZIP-320 ephemeral-address gap-limit ceiling
/// ([`SqliteClientError::ReachedGapLimit`], a `DataSource` error — round-2 #7) is detected
/// FIRST via [`GapLimitProbe`] and folds to the typed [`WalletError::TexSendLimitReached`]:
/// it is an availability ceiling (≤ the engine's ephemeral gap limit of outstanding
/// reservations — dual-natured, #315: confirming-tx0 slots free as they mine, leaked slots never
/// do), NOT corruption — mislabeling it `StoreCorrupt` would scare a user off a
/// healthy wallet, and swallowing it into a generic retry would churn a window only confirmation
/// (or the #315 reclaim) can free. Every OTHER `DataSource` access fault is the one
/// corruption door (`StoreCorrupt`). The STALE class — re-`propose` against the current tree is
/// the honest recovery — is precisely a reorg-pruned anchor (`Proposal(AnchorNotFound)`) or a
/// pruned witness/checkpoint (`CommitmentTree`, a `ShardTreeError`); those fold to
/// `ProposalStale`. (The SAME two shapes are the propose-side transient class in
/// [`propose_refusal_is_transient`] — `ProposeTransient`, "try again in a moment" — because
/// there is no proposal yet to refresh there; a pin bump reads the two together. At this pin
/// they are constructed only on THIS path, so the propose-side class is empty — said there.)
/// Every OTHER fault fails closed and payload-free as `SignFailed`:
/// proving/build faults, `KeyNotRecognized` (the USK's UFVK not matching the account —
/// see [`derive_spending_key`](crate::derivation::derive_spending_key): impossible on
/// the production seed-derived path, a defense-in-depth LOUD catch otherwise, never a
/// silent wrong-key sign), AND the remaining structural `ProposalError` variants
/// (double-spend / multi-step / transparent / ephemeral — these would signal a MALFORMED proposal,
/// not a transient fault: a recognised ZIP-320 two-step is signed normally post-gate-removal, and a
/// non-ZIP-320 multi-step never reaches this create (the shape guard above + the matching guards in
/// `prepare_queued`/`summarize` fail it closed first), so a re-`propose` would NOT fix these and
/// suggesting it would be dishonest). The builder/proposal
/// message can echo amounts AND a txid
/// (`ChainDoubleSpend`) — §5.4 — so the value is DROPPED here, never formatted.
fn map_create_err<DE: GapLimitProbe + ClassifyStoreFault, CE, SE, FE, ChE, N>(
    e: zcash_client_backend::data_api::error::Error<DE, CE, SE, FE, ChE, N>,
) -> WalletError {
    use zcash_client_backend::data_api::error::Error;
    use zcash_client_backend::proposal::ProposalError;
    match e {
        // Round-2 #7: the ephemeral gap-limit ceiling — checked BEFORE the generic
        // `DataSource` classification door (the ceiling IS a `DataSource` error).
        Error::DataSource(de) if de.reached_gap_limit() => WalletError::TexSendLimitReached,
        // #371: an out-of-disk / transient-busy / locked-device IO fault persisting the
        // created tx is honest as `DiskFull`/`StoreBusy`/`Io` (the send UI routes a
        // `DiskFull` back to the form's "free up space", not a terminal dead-end), not the
        // blanket `StoreCorrupt` seed-restore scare. The create is transactional — a fault
        // rolls back, spends no note (§6.3) — so retry-after-freeing-space is safe. Genuine
        // corruption still folds to the fail-closed `StoreCorrupt` default.
        Error::DataSource(de) => de.into_store_fault(),
        Error::Proposal(ProposalError::AnchorNotFound(_)) | Error::CommitmentTree(_) => {
            WalletError::ProposalStale
        }
        _ => WalletError::SignFailed,
    }
}

// ── inc-2d-3-b-ii-A: the double-send-guard LOGIC half ───────────────────────────
//
// The DURABLE half (the `claim`/`txid` columns + the lifecycle transitions) lives in
// `crate::intent_store`; THIS half is the logic that needs the upstream proposal +
// `WalletRead`/`InputSource` types: extract the claim from a proposal, ask the engine
// whether a claimed note is still spendable (the spend witness), and decide the recovery
// action. Together they make an intent never mint two txs and a tx never orphan its intent
// WITHOUT a cross-table transaction (the upstream constraint; spec §3.2h design-lock).
// Production callers are the resubmission hook at 3-b-ii-B (reached: P3-7 measured every
// item here live in a lib-only build); the §8 tests drive them over the funded harness.

/// Map a `ShieldedPool` to the storage-stable [`NoteClaim`] tag byte (the byte values
/// are owned by `crate::intent_store`, the storage-format SSOT).
///
/// **There is no fail-closed arm available here, and that is why this mapping is
/// spelled out rather than defaulted.** Returning an error (or a sentinel byte) for
/// the Ironwood arm would refuse EVERY send after the NU6.3 activation, because after
/// it every new shielded note the wallet receives is an Ironwood note. The only
/// correct repair is a real third tag with a matching [`proto_from_tag`] arm and a
/// matching `decode_claims` arm — anything narrower either refuses all sends or
/// writes a claim that cannot be read back.
fn proto_tag(p: ShieldedPool) -> u8 {
    match p {
        ShieldedPool::Sapling => PROTO_SAPLING,
        ShieldedPool::Orchard => PROTO_ORCHARD,
        ShieldedPool::Ironwood => PROTO_IRONWOOD,
    }
}

/// Map a stored [`NoteClaim`] tag byte back to a `ShieldedPool`. An unknown tag is
/// corruption (`decode_claims` already validates the byte, so this is defense-in-depth).
fn proto_from_tag(tag: u8) -> Result<ShieldedPool, WalletError> {
    match tag {
        PROTO_SAPLING => Ok(ShieldedPool::Sapling),
        PROTO_ORCHARD => Ok(ShieldedPool::Orchard),
        PROTO_IRONWOOD => Ok(ShieldedPool::Ironwood),
        _ => Err(WalletError::StoreCorrupt),
    }
}

/// Extract the §6.3 double-send-guard CLAIM from a proposal — the chain-stable
/// `(txid, protocol, output_index)` of every shielded input note it selected. Persisted
/// (via `intent_store::mark_submitting`) BEFORE the engine creates the tx, so crash-recovery
/// can witness whether those notes were spent. A step carrying a TRANSPARENT input (a
/// transparent UTXO spend) cannot be covered by the shielded-note witness, so it — or a proposal
/// selecting NO shielded input at all — is a structural fault (`ProposeFailed`), never a
/// silently-incomplete claim that could mis-witness a spend.
///
/// The engine's `transparent-inputs` is ON (ADR-0528), but for the RECEIVE address + scanning,
/// not a send-path transparent SPEND. A TEX two-step's step1 spends its ephemeral output via
/// `prior_step_inputs` — which is NOT a `transparent_input` — so this guard does not fire for it,
/// and step1 contributes no shielded claim (step0's claim covers the real money source). The
/// witness therefore tracks step0's shielded notes correctly with no code change here.
///
/// Generic over the proposal's `NoteRef` so the funded `LocalNetwork` harness and the live
/// `Network` wallet drive the SAME code (the note's pool + position come from the upstream
/// `ReceivedNote`, not from `NoteRef`).
pub(crate) fn claim_from_proposal<N>(
    proposal: &Proposal<StandardFeeRule, N>,
) -> Result<Vec<NoteClaim>, WalletError> {
    let mut claims = Vec::new();
    for step in proposal.steps() {
        // A transparent input cannot be covered by the shielded-note witness; recording a
        // claim that omits it would let the guard mis-read "not created". Fail closed.
        if !step.transparent_inputs().is_empty() {
            return Err(WalletError::ProposeFailed);
        }
        if let Some(shielded) = step.shielded_inputs() {
            for note in shielded.notes().iter() {
                // `TxId: AsRef<[u8; 32]>` — copy the 32-byte id straight out.
                let txid: [u8; 32] = *note.txid().as_ref();
                claims.push(NoteClaim {
                    txid,
                    // `Note::protocol()` was removed in 0.24.0 for `pool()`; the value
                    // is the same discriminant, now able to say `Ironwood`.
                    protocol: proto_tag(note.note().pool()),
                    // `output_index()` is `u16` upstream; widened (lossless) to the `u32`
                    // `get_spendable_note` expects + the on-disk 4-byte field.
                    output_index: u32::from(note.output_index()),
                });
            }
        }
    }
    // Every real send spends ≥1 note; an empty claim witnesses nothing (the guard would
    // read every note trivially spendable and re-propose blindly) ⇒ structural fault.
    if claims.is_empty() {
        return Err(WalletError::ProposeFailed);
    }
    // Cap at the PRODUCER so a claim that `decode_claims` would later reject (> the codec's
    // `MAX_CLAIM_NOTES`) can never be written in the first place — closing the write/read
    // asymmetry (review fold). A real proposal selects far fewer (ZIP-317 proof/size limits);
    // exceeding it is anomalous ⇒ fail closed rather than persist an un-reconcilable claim.
    if claims.len() > MAX_CLAIM_NOTES {
        return Err(WalletError::ProposeFailed);
    }
    Ok(claims)
}

/// The §6.3 spend WITNESS: is the claimed note still spendable, per the engine's own atomic
/// note-spend state? `false` ⟺ the note is spent by a still-live (unexpired) tx ⟺ OUR tx was
/// created — so a `false` on ANY claim note means "do not re-propose". `true` ⟺ never created,
/// OR the created tx EXPIRED (the engine frees the note then — the expiry safety net)
/// ⟺ safe to re-propose.
///
/// **HOW LONG THAT SAFETY NET TAKES CHANGED WITH `zcash_client_backend` 0.24.0, and it
/// is not 40 blocks for the modal send.** A canonical ZIP-318 crossing — a lone payment
/// of a `{1,2,5}×10^k` denomination at a height where NU6.3 is active, i.e. mainnet today
/// and the amount a person actually types — is built with the ZIP-318 ROLLING expiry:
/// 34,561–69,120 blocks, roughly **30 to 60 days**, not the ~50 minutes
/// `DEFAULT_TX_EXPIRY_DELTA` gives everything else. So for such a send, a sign that
/// succeeded followed by a broadcast that did not (offline, endpoint down, crash between
/// the two) leaves the claimed note held and the intent `Submitting` for that whole
/// window instead of an hour. Nothing here is wrong — the witness is still correct and
/// still conservative — but the SELF-HEAL BOUND this comment used to imply is off by
/// three orders of magnitude for the common case, and any UI copy or timeout reasoning
/// built on "it clears in about an hour" is now false.
/// Recorded in `docs/plan/ironwood-phase-b.md` §3; the test that would pin it needs a
/// post-activation `LocalNetwork` the funded harness cannot build yet (survey S8).
///
/// **SAFETY (the double-spend hinge):** the witness `target` is the HONEST current tip+1
/// (`get_target_and_anchor_heights` with `min_confirmations = 1`), NEVER inflated — a too-high
/// target would read a still-live spending tx as expired (the upstream `tx_unexpired_condition`
/// compares `expiry_height` against this target), surface the note as spendable, and let a
/// re-propose double-spend a tx that could still mine. Every false-positive direction (a note
/// unspendable for a non-spent reason — a deep reorg un-mine, a missing note) only yields a
/// conservative `false` ⇒ WAIT, never a re-propose.
///
/// Generic over the DB exactly like `propose_core`/`create_signed_core`. Any DB fault is
/// classified (`ClassifyStoreFault`, R12); a not-yet-synced wallet (no block data) is a
/// conservative `false`.
pub(crate) fn note_spendable<DbT>(wdb: &DbT, claim: &NoteClaim) -> Result<bool, WalletError>
where
    DbT: WalletRead<Error: ClassifyStoreFault> + InputSource<Error: ClassifyStoreFault>,
{
    let Some((target, _anchor)) = wdb
        .get_target_and_anchor_heights(NonZeroU32::MIN)
        .map_err(|e| e.into_store_fault())?
    else {
        return Ok(false);
    };
    let txid = TxId::from_bytes(claim.txid);
    let protocol = proto_from_tag(claim.protocol)?;
    let note = wdb
        .get_spendable_note(
            &txid,
            protocol,
            claim.output_index,
            target,
            // UNFILTERED, deliberately (survey S13). 0.24.0 lets a note be LOCKED by
            // a concurrent proposal, and the default `Policy` filter would hide such
            // a note. This witness asks one question — "is the note we claimed still
            // UNSPENT?" — and a locked note is unspent. Reading a merely-locked note
            // as absent would answer `false`, which this path interprets as "our
            // transaction WAS created", and the send would be dropped instead of
            // retried. Lock state is a selection concern; spentness is the question
            // here.
            LockFilter::Unfiltered,
        )
        .map_err(|e| e.into_store_fault())?;
    Ok(note.is_some())
}

/// The crash-recovery action for an in-flight intent (§6.3 reconcile), as a PURE decision over
/// the two durable facts: whether ALL its claimed notes are still spendable (the witness), and
/// whether a `txid` was recorded. Pure so the decision matrix is unit-testable with no DB.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Recovery {
    /// NO group recorded and all claim notes spendable ⇒ the tx was never created: re-propose.
    ReProposeFresh,
    /// A tx GROUP was recorded ⇒ reconcile it over the chain in [`rebroadcast_group`], WHATEVER
    /// the witness reads: the engine frees a claim note at BARE expiry, while a reorg can still
    /// revive the signed group — a fresh re-propose there could pay the recipient twice (S7 C1),
    /// so only the group reconciler's burial-gated requeue may send the payment again. Step order
    /// (tx0→tx1); non-empty by construction; a single-step send is a one-element group. §3.2i-2 pt 4.
    ReBroadcast(Vec<[u8; 32]>),
    /// A claim note is spent but NO tx group was recorded (the narrow create-committed-but
    /// -unrecorded window) ⇒ wait for the tx to mine or expire; do NOTHING this pass.
    AwaitWitness,
}

/// Decide the reconcile action (see [`Recovery`]). `all_claims_spendable` is the AND over each
/// claim note's [`note_spendable`] witness; `recorded_txids` is the intent's ordered `txids`
/// group (EMPTY ⇒ `Submitting` / the create-window; non-empty ⇒ `Sent`). A recorded group wins.
pub(crate) fn reconcile(all_claims_spendable: bool, recorded_txids: &[[u8; 32]]) -> Recovery {
    if !recorded_txids.is_empty() {
        Recovery::ReBroadcast(recorded_txids.to_vec())
    } else if all_claims_spendable {
        Recovery::ReProposeFresh
    } else {
        Recovery::AwaitWitness
    }
}

/// The §4.4 deposit-deadline decision for a queued/in-flight intent (inc-2d-swap-a). The
/// resubmission cores match on this to decide whether to (re-)propose, abandon, or wait.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DepositGate {
    /// Not a swap deposit (no deadline) OR a deposit whose quote is still live ⇒ proceed with
    /// the ordinary send / reconcile (the `None` and live-deadline cases collapse here).
    Proceed,
    /// A swap deposit whose quote deadline has lapsed ⇒ never feed it; the caller DELETES the
    /// intent (terminal — the user must re-quote; §4.4 "would feed a dead quote").
    Expired,
    /// A swap deposit the device wall-clock cannot time-check — an UNSYNCED/implausible clock
    /// (a just-booted phone before NTP, a reset clock reading near the epoch) ⇒ do NOTHING this
    /// pass: never feed a quote we cannot verify is live, never delete by an unknowable deadline.
    /// Self-heals once the clock syncs (the normal `Proceed`/`Expired` decision then applies).
    Wait,
}

/// Classify a queued/in-flight intent's §4.4 deposit deadline against the pass's wall-clock
/// `now_unix` (inc-2d-swap-a). `deadline` is the persisted `deposit_deadline` column.
///
/// - `None` ⇒ an ordinary send (never a deposit) ⇒ [`DepositGate::Proceed`].
/// - An UNSYNCED clock (`now_unix < CLOCK_PLAUSIBILITY_FLOOR_SECS`) ⇒ [`DepositGate::Wait`] for a
///   deposit (a deadline can't be time-checked by a clock that says "1970"; never fed, never
///   deleted — the mobile boot-before-NTP case). The floor is checked BEFORE the lapse math so a
///   sub-floor clock never decides expiry in EITHER direction.
/// - Otherwise the deposit is [`DepositGate::Expired`] once `now_unix` is within
///   [`DEPOSIT_FIRST_FEED_MARGIN_SECS`] of the deadline, or the deadline is corrupt
///   (`<= 0` — fail-closed, never feed an unknowable quote); else [`DepositGate::Proceed`].
///   The margin is sized in BLOCK time, not wall slack (W-swap-4-a-3 review re-price): the
///   deposit only counts if it MINES before the provider deadline, and a first broadcast in
///   the final ~75 s block interval is a refund coin-flip whose refund the wallet cannot yet
///   display (ADR-0527/#368) — so this gate demands ~3 block intervals of mining room. The
///   `SwapService` execute pre-flight uses the STRICTLY LARGER
///   [`DEPOSIT_EXECUTE_MARGIN_SECS`](crate::constants::DEPOSIT_EXECUTE_MARGIN_SECS) for
///   deposit-carrying quotes, so an execute this gate would immediately kill can never be
///   admitted. The 60 s margin gap is an EXECUTE-ADMISSION guarantee only (the sign/kick
///   inside the execute bracket start with ≥ ~1 min above this gate; W-swap-4-a-4 docs
///   truth): a DRAIN-side sign legally proceeds right up to this 240 s gate — its headroom
///   is whatever the pass timing leaves, which is exactly why the broadcast phase re-gates
///   per group (rule 3) and orders deadline-tagged groups first.
///
/// CLOCK posture (§3.2h inc-2d-swap-a): wall-clock is the ONLY restart-durable deadline a
/// resubmission pass can read (the monotonic `SwapService` dual-gate does not survive a process
/// restart — it is the strict pre-flight at inc-2d-swap-b; this is the durable net). The `>=`
/// boundary is the canonical lapse edge (inc-2d-swap-b's monotonic gate MUST match it). A
/// far-future clock fails closed-DESTRUCTIVE (deletes → re-quote, bounded, no fund loss); the
/// common reset-to-epoch case is caught by the floor (`Wait`, non-destructive).
pub(crate) fn deposit_gate(deadline: Option<i64>, now_unix: u64) -> DepositGate {
    deposit_gate_with_margin(deadline, now_unix, DEPOSIT_FIRST_FEED_MARGIN_SECS)
}

/// The CONTINUE gate (the 2026-10-05 review, F01; plan §2.2): may a LATER leg of a
/// deposit group follow a previous leg that is already on the network (mined, or
/// accepted by an endpoint — never merely `Rejected`)? `Expired` from
/// [`DEPOSIT_CONTINUE_MARGIN_SECS`] (priced there) before the deadline, not
/// [`DEPOSIT_FIRST_FEED_MARGIN_SECS`]. That larger margin is for STARTING a
/// deposit; applied to tx1 of a two-step (TEX) deposit whose tx0 is out, it
/// strands the funds on the one-time address in the window where tx1 would most
/// likely still land (the constant prices it). Same unsynced-clock `Wait` and
/// corrupt-deadline handling as [`deposit_gate`]; the start/continue choice is
/// always the CALLER's.
pub(crate) fn deposit_gate_continue(deadline: Option<i64>, now_unix: u64) -> DepositGate {
    deposit_gate_with_margin(deadline, now_unix, DEPOSIT_CONTINUE_MARGIN_SECS)
}

/// The one predicate behind [`deposit_gate`] and [`deposit_gate_continue`].
fn deposit_gate_with_margin(deadline: Option<i64>, now_unix: u64, margin_secs: u64) -> DepositGate {
    let Some(d) = deadline else {
        return DepositGate::Proceed; // ordinary send — no deadline, always proceeds
    };
    if now_unix < CLOCK_PLAUSIBILITY_FLOOR_SECS {
        return DepositGate::Wait; // unsynced clock — a deadline is untimeable; wait, never feed/delete
    }
    // The `||` short-circuits left-to-right: `d.unsigned_abs()` is evaluated only when `d > 0`, so
    // the i64→u64 widening can never sign-confuse; `saturating_add` can never wrap.
    if d <= 0 || now_unix.saturating_add(margin_secs) >= d.unsigned_abs() {
        DepositGate::Expired
    } else {
        DepositGate::Proceed
    }
}

/// The durable deposit-deadline TAG for a provider `expires_at` (§4.4 W-swap-4-a-2,
/// the arch review HIGH fold): CLAMPED to `now + SWAP_DEPOSIT_DEADLINE_OUT_OF_ZEC_SECS`
/// (15 min — a deadline tag exists ONLY for a wallet-sent OutOfZec deposit, so the
/// wallet's own requested window IS the honest ceiling) so a hostile/buggy/MITM'd
/// provider deadline can never make the ONE-deposit-in-flight guard's lockout
/// provider-controlled — the guard self-clears on OUR ceiling, not the provider's
/// imagination (the W2 fold clamps only the in-memory `act_deadline`; the durable
/// `expires_at_wall` is verbatim by design, so the wallet seam must bound its own
/// tag). W-swap-4-a-3 (security review HIGH fold): the ceiling dropped from the 24 h
/// `SWAP_DEADLINE_DEFAULT_SECS` to the 15-min window itself — an echoed deadline
/// beyond what we requested is hostile or buggy by definition, and the old ceiling
/// left the guard/zombie lockout 96× looser than the wallet's stated need. Too-SHORT
/// is the money-safe direction: past the clamp the wallet just stops serving the
/// quote (hold → expiry frees the notes → purge/gate delete → the user re-quotes) —
/// nothing is lost, availability only. Against an HONEST provider the tag never
/// meets this ceiling (the echo ≈ the request).
/// A value that does not fit `i64` (> year 2262) saturates to `0` ⇒ `deposit_gate`
/// reads `Expired` (fail-CLOSED, the pre-existing rule). A sub-plausibility `now`
/// (boot-before-NTP) yields a 1970-ish clamp: the gate `Wait`s while the clock is
/// untimeable and expires the deposit once it heals — fail-closed, never a
/// dead-quote feed.
#[cfg_attr(not(feature = "swap"), allow(dead_code))]
pub(crate) fn clamped_deposit_deadline(deadline_unix: u64, now_unix: u64) -> i64 {
    let ceiling = now_unix.saturating_add(crate::constants::SWAP_DEPOSIT_DEADLINE_OUT_OF_ZEC_SECS);
    i64::try_from(deadline_unix.min(ceiling)).unwrap_or(0)
}

/// SEEDLESS purge of every `Queued` swap deposit whose quote deadline has lapsed
/// (§4.4 W-swap-4-a-2 — the crypto audit/security BLOCKER fold). The in-gate delete
/// inside [`prepare_queued`] is unreachable exactly where it matters most: the
/// drain's Phase 2 pulls `acquire_seed` BEFORE `prepare_queued`, so a host-custody
/// (`SeedPersistence::None`) wallet — whose background pull is always
/// `SeedRequired` — leaves a sign-missed deposit `Queued` forever, and a portless
/// wallet skips Phase 2 entirely. Pre-guard that zombie was a benign dead record;
/// with the ONE-deposit-in-flight guard it would brick every future swap
/// (deposits are excluded from `cancel_queued_send` — no user remedy short of a
/// wipe). This purge needs only the deadline tag + the pass clock (no seed, no
/// wallet-db lock), so the orchestrator runs it UNCONDITIONALLY on every pass —
/// it is what makes the guard's "self-clears with the quote lifetime" claim true
/// at EVERY custody tier. It also terminates the requeued arm of the same wedge:
/// an all-legs-dead-buried SIGNED deposit is `reset_to_queued` by the reconcile
/// and lands here on the next pass. `DepositGate::Wait` (untimeable clock) leaves
/// the row untouched, exactly like the in-gate path; a LIVE deposit and every
/// ordinary send (`None` deadline ⇒ `Proceed`) pass through untouched. Returns
/// the number of rows deleted (the orchestrator counts them `deposit_expired`).
pub(crate) fn purge_lapsed_queued_deposits(
    aux: &mut rusqlite::Connection,
    now_unix: u64,
) -> Result<usize, WalletError> {
    let mut purged = 0;
    for intent in crate::intent_store::list_queued(aux)? {
        if intent.deposit_deadline.is_some()
            && matches!(
                deposit_gate(intent.deposit_deadline, now_unix),
                DepositGate::Expired
            )
            && crate::intent_store::delete(aux, intent.id)?
        {
            purged += 1;
        }
    }
    Ok(purged)
}

// ── inc-2d-3-b-ii-B: the resubmission ENGINE-SIDE cores (sync, DB-only) ──────────
//
// The §6.3 resubmission machinery, split engine-side (these — sync, generic over the DB,
// NO network) vs orchestration (`Wallet::resubmit_queued_sends` — async, drives these in
// `run_blocking` then broadcasts). The split is the SAME `broadcast_loop` lesson: the
// money LOGIC is a pure-DB generic fn so the funded `LocalNetwork` harness drives the
// EXACT production code that the live `Network` wallet runs (the two are type-incompatible
// by construction, so an inline `Wallet` method could only be E2E-tested on a device). The
// async orchestrator stays a THIN bridge over the already-tested 3-a broadcast path.

/// What the orchestrator must do after one in-flight intent is reconciled. Only the
/// [`Reconciled::Rebroadcast`] arm needs the network; the rest were finalized in the DB
/// here. Carries the disposition (not just "settled") so the resubmit event + tests can
/// count re-queues / mined-deletes / waits distinctly. `Debug` is HAND-WRITTEN (not
/// derived) to REDACT the `Rebroadcast` arm's raw signed-tx bytes (recipient scripts +
/// amounts — §5.4 NEVER-LOG), mirroring [`Prepared`]: a stray `{reconciled:?}` can only
/// ever print the group SIZE, never the money material.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Reconciled {
    /// The UNMINED txs of the recorded group → (re)broadcast these raw consensus bytes IN ORDER
    /// (best-effort; the row stays `Sent` until the FINAL tx mines — a failed send retries next
    /// pass). For a single-step send this is one tx; for a partially-broadcast TEX two-step it is
    /// the unmined tail (the skip-mined recovery, §3.2i-2 pt 4). `continues[i]` says whether
    /// `txs[i]` goes under the continue gate (it, or the recorded leg before it, is mined or
    /// carries the accepted mark — F01, plan §2.2); same length as `txs`.
    Rebroadcast {
        txs: Vec<Vec<u8>>,
        continues: Vec<bool>,
    },
    /// A multi-step send STRANDED (§3.2i-2 round-2 BLOCKER #1 + #6): an earlier tx mined AND BURIED
    /// beyond `REORG_MAX_BLOCKS` but a later tx expired un-mined, so re-broadcast can never complete
    /// it AND the irreversible step can no longer reorg out. The row was moved to the terminal
    /// `Stranded` state (it leaves the reconcile loop) and the funds — on a wallet-controlled
    /// ephemeral address — are owned by the 2e-2b detect surface. Money-safe: the funds are the
    /// user's and recoverable, just not via this outbox.
    Stranded,
    /// A `Submitting` row whose claimed notes are spendable (the tx was never created) was reset
    /// to `Queued` for a fresh re-propose. ALSO — the ONLY way a `Sent` row requeues — the
    /// group-side all-expired terminal (v-5c finding #1): a recorded group with NO tx mined and
    /// EVERY tx past its expiry with the expiry BURIED (the round-2 #6 reorg margin) is dead
    /// WHOLE, whatever the note witness reads (the engine frees the notes at BARE expiry — S7
    /// C1 — and a mid-scan kill can leave it reading "spent").
    Requeued,
    /// The recorded FINAL tx is mined AND buried beyond `REORG_MAX_BLOCKS` (round-2 #6) ⇒ the chain
    /// irreversibly owns the send; the row was deleted (the §6.3 delete-on-mined cleanup). A shallow
    /// (reorg-reversible) mine is NOT yet deleted — it waits as [`AwaitingBurial`](Self::AwaitingBurial).
    Deleted,
    /// Wait — do nothing this pass. Either (a) a claim note is spent but no `txid` was recorded
    /// (the narrow create-committed-but-unrecorded window — such a tx was never broadcast and WILL
    /// expire → notes free → a later pass `Requeued`s), (b) a swap deposit on an UNSYNCED clock
    /// (`DepositGate::Wait`) whose deadline is untimeable until the clock syncs (inc-2d-swap-a), or
    /// (c) a needed tx is momentarily missing from the store (pruned) on the re-broadcast path. A
    /// reorg BURIAL wait is the DISTINCT [`AwaitingBurial`](Self::AwaitingBurial) (round-2 #6).
    Awaiting,
    /// Wait for REORG BURIAL (round-2 #6) — an EXPECTED depth accumulation, distinct from
    /// [`Awaiting`](Self::Awaiting): the recorded final tx is mined but still shallow (delete
    /// waits for burial beyond `REORG_MAX_BLOCKS`); OR an earlier tx is mined-but-shallow while the
    /// final is un-minable (strand waits for it to bury); OR no leg mined and EVERY leg is past its
    /// expiry, not yet buried (S7 C1: no leg can enter a block on this branch, so no re-broadcast —
    /// the requeue waits). Resolves to `Deleted`/`Stranded`/`Requeued`. The orchestrator counts
    /// it on its OWN field and does NOT treat it as `did_work` (so it emits NO per-pass
    /// `wallet.resubmit` event during the ~`REORG_MAX_BLOCKS`-block wait — it is neither a fault nor
    /// a move). HOST NOTE: the user-facing finality signal is `TxStatus::Confirmed { depth >
    /// REORG_MAX_BLOCKS }` from `transactions()` — `depth = tip - mined + 1`, so `depth >
    /// REORG_MAX_BLOCKS` is EXACTLY this code's own burial/irreversibility threshold
    /// `tip - mined >= REORG_MAX_BLOCKS` (NOT `depth >= REORG_MAX_BLOCKS`, which is one block too
    /// eager) — NOT the presence/absence of this recovery row (which lingers ~`REORG_MAX_BLOCKS`
    /// past the mine before cleanup).
    AwaitingBurial,
    /// A SWAP-DEPOSIT intent whose quote deadline has lapsed AND whose notes are spendable again
    /// (never created / expired) ⇒ the row was DELETED — re-proposing would feed a dead quote
    /// (§4.4). Terminal: the user must re-quote (inc-2d-swap-a). An already-CREATED deposit tx
    /// reconciles normally through the terminal arms, but its (re)broadcast is HELD past the
    /// deadline — the distinct [`DepositBroadcastHold`](Self::DepositBroadcastHold).
    DepositExpired,
    /// A SWAP-DEPOSIT group whose (re)broadcast is HELD this pass (§4.4 W-swap-4-a-2 rule 2):
    /// its quote deadline has lapsed (`DepositGate::Expired`) or is untimeable on an unsynced
    /// clock (`DepositGate::Wait`), and post-FR-23-a the FIRST broadcast can be deferred
    /// (sign@execute, broadcast@drain) — so broadcasting here could FIRST-feed a DEAD quote
    /// (a fee-burning provider refund round-trip). The row is left untouched: the terminal
    /// arms (delete-on-buried / all-legs-dead-buried requeue → `deposit_gate` delete / strand)
    /// were checked FIRST and still resolve it, so the hold is bounded by tx expiry + burial,
    /// never a wedge. Counted on its own field and — like
    /// [`AwaitingBurial`](Self::AwaitingBurial) — excluded from `did_work` (an EXPECTED wait
    /// that repeats every pass until expiry frees the notes; per-pass events would be noise).
    DepositBroadcastHold,
}

impl std::fmt::Debug for Reconciled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // §5.4: print the group SIZE, never the signed bytes (recipient scripts + amounts).
            Self::Rebroadcast { txs, continues } => {
                write!(f, "Rebroadcast({} txs, continues {continues:?})", txs.len())
            }
            Self::Stranded => f.write_str("Stranded"),
            Self::Requeued => f.write_str("Requeued"),
            Self::Deleted => f.write_str("Deleted"),
            Self::Awaiting => f.write_str("Awaiting"),
            Self::AwaitingBurial => f.write_str("AwaitingBurial"),
            Self::DepositExpired => f.write_str("DepositExpired"),
            Self::DepositBroadcastHold => f.write_str("DepositBroadcastHold"),
        }
    }
}

/// The outcome of driving one `Queued` intent toward broadcast. `Debug` is HAND-WRITTEN (not
/// derived) to REDACT the `Broadcast` group's signed-tx bytes (recipient scripts + amounts — §5.4
/// NEVER-LOG): a stray `{prepared:?}` can only ever print the group SIZE, never the money material.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Prepared {
    /// Proposed, claimed, created+persisted, the ordered tx GROUP recorded — broadcast these raw
    /// bytes IN ORDER (tx0→tx1…). A single-step queued send is a one-element group; a multi-step
    /// (TEX / ZIP-320) queued drain is the `[tx0, tx1]` chain (§3.2i-2 pt 3-4). The orchestrator
    /// sends each group over the ordered, non-latching-per-group broadcast tail (round-2 #4/#5).
    Broadcast(Vec<Vec<u8>>),
    /// Re-propose can no longer cover the intent (`InsufficientFunds`) ⇒ `QueuedSendStale`
    /// (`RW-BCAST-001`). Left `Queued`, never silently dropped — funds may still arrive, and a
    /// future per-intent FFI status read surfaces the typed [`WalletError::QueuedSendStale`].
    Stale,
    /// A transient condition (not-synced-far-enough `ProposalStale`, a `mark_submitting` race, a
    /// create fault that left no spent note, OR a swap deposit on an UNSYNCED clock —
    /// `DepositGate::Wait`, inc-2d-swap-a) ⇒ left `Queued` to retry on the next pass.
    Retry,
    /// A queued TEX two-step that reached the ZIP-320 ephemeral-address gap-limit ceiling at create
    /// time (round-2 #7 — `create` returned [`WalletError::TexSendLimitReached`]). OTHER outstanding
    /// reservations fill the engine's window, so THIS send cannot mint a fresh ephemeral address
    /// right now — NOT a fault of this send and NOT corruption. No note was spent (the engine
    /// persists TXS only on full success — NB the refused reservation here reserved nothing, but a
    /// SUCCESSFUL reservation is committed before construction and never rolled back, see
    /// `CappedParked`); the Submitting claim is released back to `Queued`. The ceiling is
    /// DUAL-NATURED and this code cannot tell the halves apart (#315 review): window slots held by
    /// LIVE in-flight tx0s clear as those mine (a MINED tx0 advances the engine's ephemeral gap —
    /// transient, self-heals), while slots held by LEAKED reservations (tx0 expired un-mined — the
    /// engine NEVER un-reserves; `find_gap_start` advances only on a MINED first-use, NOT on
    /// expiry) never clear on their own — the slice-2 reclaim (#315 Mechanism A) is their only
    /// remedy. So surface copy must hold BOTH ("some may clear as transfers confirm; sends that
    /// can't confirm won't clear on their own"). Counted DISTINCTLY (not the generic `Retry`, which
    /// would mislabel a money-incomplete ceiling as benign progress); the row stays `Queued`
    /// (visible via `list_parked_sends`, cancellable), retried next pass. A ceiling wait consumes
    /// NO `CappedParked` budget — the typed refusal proves nothing was reserved, so the drain
    /// REFUNDS the pessimistic count (`reset_to_queued_refunding_attempt`, red-team F1) and
    /// a healthy TEX behind a transiently-full window self-heals when a live tx0 mines, exactly
    /// as before #315; an all-leaked window keeps the row honestly waiting here, surfaced by the
    /// pressure gauge + the per-pass `ceiling` telemetry, until the slice-2 reclaim reopens it.
    /// (Replaces the old gate-era `Blocked` terminal, removed at gate-removal — 2e-2b-v-5.)
    Ceiling,
    /// A queued TEX intent whose `repropose_attempts` reached `MAX_TEX_REPROPOSE_ATTEMPTS`
    /// (crate::constants) — the #315 slice-1 leak cap. Every ephemeral-reserving create attempt
    /// reserves a fresh engine index that is NEVER un-reserved (not on tx0 expiry — `find_gap_start`
    /// advances only on a MINED first-use — and not on a create fault, where the reservation is
    /// committed BEFORE construction and deliberately not rolled back), so an uncapped requeue loop
    /// lets ONE stubborn intent burn the whole `EPHEMERAL_GAP_LIMIT` window. The gate fires in
    /// `prepare_queued` AFTER the deposit gate (a capped swap deposit still self-expires at its
    /// quote deadline) and BEFORE `propose_core` (no propose/build/seed cost on a parked row), so
    /// NO note is ever touched by it. The row stays `Queued`, surfaced `paused` via
    /// `list_parked_sends` (distinguishable from a healthy pending send), cancellable, and
    /// resumable via `Wallet::retry_parked_send` (reset the counter) — an HONEST park, never an
    /// infinite silent re-propose and never an auto-drop. Availability-only severity: the cap can
    /// under-send (paused until the user acts) but can never double-send or strand.
    CappedParked,
    /// The engine lowered a queued send to a multi-step proposal that is NOT the recognised ZIP-320
    /// TEX two-step. The `[tx0, tx1]` recovery contract does not cover it, so the SDK REFUSES to sign
    /// it — the queued path's fail-closed analogue of `summarize`'s `ProposeFailed` on the interactive
    /// path. Money-SAFE (no note spent, the row left `Queued`) and a STABLE shape, so it is surfaced
    /// as a permanently-wedged `corrupt` send (counted distinctly, never a churning `Retry`): the
    /// user's queued send will never broadcast, so an operator must see it. DEFENSIVE / currently-
    /// unreachable: the engine today emits only a single step or a clean ZIP-320 two-step (multiple
    /// TEX recipients BATCH into ONE ephemeral pair — still the recognised shape), so reaching this
    /// would take a FUTURE engine multi-step shape — exactly what this guard (and `summarize`'s twin)
    /// foreclose.
    Unsupported,
    /// A SWAP-DEPOSIT intent whose quote deadline has already lapsed at re-propose time ⇒ the row
    /// was DELETED without proposing — a fresh deposit must never feed a dead quote (§4.4;
    /// inc-2d-swap-a). Terminal: the user must re-quote.
    DepositExpired,
}

impl std::fmt::Debug for Prepared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // §5.4: print the group SIZE, never the signed bytes (recipient scripts + amounts).
            Self::Broadcast(group) => write!(f, "Broadcast({} txs)", group.len()),
            Self::Stale => f.write_str("Stale"),
            Self::Retry => f.write_str("Retry"),
            Self::Ceiling => f.write_str("Ceiling"),
            Self::CappedParked => f.write_str("CappedParked"),
            Self::Unsupported => f.write_str("Unsupported"),
            Self::DepositExpired => f.write_str("DepositExpired"),
        }
    }
}

/// Read a persisted tx's consensus-serialized bytes, or `None` if it is no longer in the
/// store. UNLIKE [`Wallet::raw_tx_bytes`](crate::wallet) (the `send` path, where a miss is
/// genuine `StoreCorrupt` — "we just wrote it"), a resubmission read can legitimately MISS a
/// pruned/expired unmined tx, so a miss is `Ok(None)` (the 3-a `raw_tx_bytes` OBLIGATION),
/// never the corruption door. A DB fault is classified (R12; a decode fault is `StoreCorrupt`).
fn read_raw_tx<DbT>(wdb: &DbT, txid: [u8; 32]) -> Result<Option<Vec<u8>>, WalletError>
where
    DbT: WalletRead<Error: ClassifyStoreFault>,
{
    let Some(tx) = wdb
        .get_transaction(TxId::from_bytes(txid))
        .map_err(|e| e.into_store_fault())?
    else {
        return Ok(None);
    };
    let mut bytes = Vec::new();
    tx.write(&mut bytes)
        .map_err(|_| WalletError::StoreCorrupt)?;
    Ok(Some(bytes))
}

/// Read a persisted tx's `expiry_height` (0 ⇒ no expiry), or `None` if it is no longer in the
/// store (pruned/expired-and-collected). Like [`read_raw_tx`], a miss is `Ok(None)` on the
/// resubmission path (NOT corruption — a re-read can legitimately miss). Drives the
/// expiry→`Stranded` terminal: a recorded tx whose `expiry_height < tip` can never mine.
fn read_tx_expiry<DbT>(wdb: &DbT, txid: [u8; 32]) -> Result<Option<u32>, WalletError>
where
    DbT: WalletRead<Error: ClassifyStoreFault>,
{
    Ok(wdb
        .get_transaction(TxId::from_bytes(txid))
        .map_err(|e| e.into_store_fault())?
        .map(|tx| u32::from(tx.expiry_height())))
}

/// `true` iff a persisted tx's `expiry_height` proves it PAST its validity window at `tip`:
/// the next block (`tip + 1` = target) exceeds `expiry`, so it can never enter a block on the
/// CURRENT branch (ZIP-203 — the wallet's own `expiry < target` notion). `expiry == 0` means
/// NO expiry: such a tx can mine at ANY height and is NEVER past-expiry — the `!= 0` arm is a
/// live double-send door if ever dropped (`0 <= tip` holds at every tip). BRANCH-LOCAL only:
/// a reorg whose fork point is below `expiry` re-creates heights `<= expiry`, so an
/// IRREVERSIBLE decision must ALSO gate on burial (round-2 #6). The shared kernel of the
/// all-expired requeue terminal and the strand gate, so the two sites can never drift.
/// `pub(crate)` since stage S8: the delivery obligation (`crate::delivery`) judges a
/// wallet-created transaction's expiry by the SAME kernel, against the scanned tip.
pub(crate) fn past_expiry(expiry: u32, tip: u32) -> bool {
    expiry != 0 && expiry <= tip
}

/// Reconcile a `Sent` intent's recorded tx GROUP over the chain (§3.2i-2 pt 4 + round-2 #1). The
/// send WAS created (a group is recorded), so we are in the re-broadcast/recovery regime whatever
/// the note witness reads (S7 C1). `group` is the ordered txids (tx0→tx1…) the engine created;
/// a single-step send is a one-element group. Decides:
///
/// - **delete-on-FINAL-BURIED (round-2 #6):** the FINAL tx mined ⇒ the whole pool-crossing chain
///   landed (a later tx cannot mine before the earlier one it spends), so the chain owns the send —
///   but the row is DELETED only once that final tx is BURIED beyond `REORG_MAX_BLOCKS` (the deepest
///   reorg the wallet auto-recovers) ⇒ [`Reconciled::Deleted`]; a shallow (still-reversible) mine
///   WAITS as [`Reconciled::AwaitingBurial`] (the tx is on-chain — nothing to re-broadcast — until
///   it buries). Deleting at one confirmation would, on a deeper reorg un-mining the chain afterwards,
///   leave NO recovery row and no possible re-propose (post-tx0 is irreversible).
/// - **expiry→Stranded (round-2 BLOCKER #1 + #6 burial):** the final tx is UNMINED + past its
///   expiry (can never mine) AND an EARLIER tx already mined+BURIED (the shielded notes left
///   IRREVERSIBLY, so re-propose is impossible) ⇒ STOP re-broadcasting, move the row to terminal
///   `Stranded` (it leaves the reconcile loop) and hand the ephemeral funds to the 2e-2b detect ⇒
///   [`Reconciled::Stranded`]. While the earlier tx is mined but still SHALLOW the row WAITS
///   ([`Reconciled::AwaitingBurial`]) — a reorg could un-mine it and recover via the spendable-note
///   re-queue, so stranding then would be a false terminal. A single-step send has no earlier tx,
///   so it can NEVER strand here (an expired single-step waits for the all-expired requeue
///   below) — the explicit `earlier_mined` guard makes that structural.
/// - **all-expired→Requeued (v-5c finding #1):** NO tx of the group mined and EVERY tx is past
///   its expiry WITH the expiry BURIED (`expiry != 0 && expiry <= tip && tip - expiry >=
///   REORG_MAX_BLOCKS` — the same round-2 #6 margin every irreversible terminal gates on) ⇒
///   the created group is dead WHOLE even across any auto-recoverable reorg, and no money
///   moved: reset the row to `Queued` for a fresh re-propose ⇒ [`Reconciled::Requeued`] — the
///   ONLY exit that sends a `Sent` payment again, whatever the note witness reads (it frees
///   the notes at BARE expiry, and a mid-scan kill can leave it reading "spent"). The margin
///   is the double-spend hinge: the re-propose may select other inputs, so a reorg reviving
///   the old group would pay the recipient twice — burial puts revival beyond the wallet's
///   reorg model. A leg with an unreadable (pruned) or absent (`0`) expiry is NOT proven dead
///   (it could still mine) ⇒ no requeue — it lands `Awaiting` below.
/// - **bare-expired→AwaitingBurial (S7 C1):** no tx mined and EVERY tx past its expiry, the
///   expiry NOT yet buried ⇒ no leg can enter a block on this branch: WAIT with no network (a
///   reorg that lowers the tip makes `past_expiry` false and the re-broadcast resumes by
///   itself). A lapsed deposit reads [`Reconciled::DepositBroadcastHold`] (checked first).
/// - **skip-mined re-broadcast:** otherwise re-broadcast every UNMINED tx IN ORDER, skipping any
///   already-mined tx (a re-broadcast of a mined/already-known tx0 must not stop tx1) ⇒
///   [`Reconciled::Rebroadcast`]. A read miss (pruned) of a needed tx is a conservative
///   [`Reconciled::Awaiting`], never a re-queue (a note is spent by a live tx that may yet mine).
///
/// An UNSYNCED wallet (no chain tip yet) is a conservative `Awaiting` — never delete or strand
/// without a tip to judge mined/expiry against.
///
/// PRECONDITION (positional, money-relevant): the mined-ness reads (`get_tx_height`) are only
/// as fresh as the SCANNED chain, while `tip` tracks the RECORDED chain tip — in general the
/// recorded tip can run ahead of the scan. The sole production caller is the §6.3 resubmission
/// pass, which runs ONLY `after_synced` on a CLEAN pass (scanned == tip, scanner idle under
/// the controller's single-writer pass guard), so "unmined at `tip`" is honest there. A future
/// second caller on an unsynced view could read a mined-in-the-gap leg as unmined and requeue
/// a live group (a double-pay door) — do NOT call this outside the post-sync pass.
fn rebroadcast_group<DbT>(
    wdb: &DbT,
    aux: &mut rusqlite::Connection,
    id: QueuedSendId,
    group: &[[u8; 32]],
    deposit_deadline: Option<i64>,
    now_unix: u64,
) -> Result<Reconciled, WalletError>
where
    DbT: WalletRead<Error: ClassifyStoreFault>, // tip/mined/tx reads only, no `InputSource` (NIT)
{
    let Some((target, _anchor)) = wdb
        .get_target_and_anchor_heights(NonZeroU32::MIN)
        .map_err(|e| e.into_store_fault())?
    else {
        return Ok(Reconciled::Awaiting); // unsynced — can't judge mined/expiry; wait
    };
    let tip = u32::from(target).saturating_sub(1);
    // The mined HEIGHT of a tx (`None` ⇒ not on-chain) and its BURIAL test. A tx is buried once it
    // sits at least `REORG_MAX_BLOCKS` deep — the deepest reorg the wallet auto-recovers
    // (`constants::REORG_MAX_BLOCKS`) — so no reorg the recovery handles can un-mine it. The
    // IRREVERSIBLE terminal decisions (delete-on-final, mark-stranded) gate on burial, NOT a shallow
    // one-confirmation mine (round-2 #6): a deeper reorg AFTER an irreversible delete/strand would
    // un-mine the tx with no recovery row left to re-broadcast or re-queue from, and a post-tx0
    // re-propose is structurally impossible. `tip - mined >= REORG_MAX_BLOCKS` ⇒ ≥ that many blocks
    // were built atop the mining block, beyond auto-recoverable reorg depth.
    let mined_height = |txid: [u8; 32]| -> Result<Option<u32>, WalletError> {
        Ok(wdb
            .get_tx_height(TxId::from_bytes(txid))
            .map_err(|e| e.into_store_fault())?
            .map(u32::from))
    };
    let buried = |mined: u32| crate::stranded::buried(mined, tip);

    // A `ReBroadcast` group is non-empty by construction (`reconcile` only yields it for a
    // non-empty slice, and `list_in_flight` rejects a `Sent` row with no group). Fail closed
    // rather than panic on a money path if that invariant chain is ever weakened (review NIT).
    let final_txid = *group.last().ok_or(WalletError::StoreCorrupt)?;
    // delete-on-FINAL-BURIED (round-2 #6): the chain owns the whole group (a later tx can't mine
    // before the earlier one it spends), but only DELETE the recovery row once the FINAL tx is
    // BURIED beyond `REORG_MAX_BLOCKS`. A shallow mine is reversible — a deep reorg un-mining the
    // chain AFTER the row is gone would leave nothing to recover from. While mined-but-shallow the
    // tx is already on-chain (nothing to re-broadcast), so WAIT for burial.
    if let Some(h) = mined_height(final_txid)? {
        if buried(h) {
            crate::intent_store::delete(aux, id)?;
            return Ok(Reconciled::Deleted);
        }
        return Ok(Reconciled::AwaitingBurial);
    }

    // Final tx UNMINED. Did an EARLIER tx mine, and is it BURIED? (⇒ the irreversible pool-crossing
    // step is permanent, so a fresh re-propose can never reproduce this send.) A single-step group
    // has no earlier tx. We capture the FIRST mined earlier tx's burial: stranding is committed only
    // once it is buried (round-2 #6) — while it is shallow a reorg could un-mine it, and the group
    // then re-broadcasts (or, dead whole, requeues after burial) below; a terminal `Stranded` then
    // would be a FALSE terminal that frees the money but records a dead send.
    let mut legs_expired = false; // no leg mined and every leg past its expiry (the S7 C1 wait)
    let mut earlier_mined = false;
    let mut earlier_buried = false;
    for &txid in &group[..group.len() - 1] {
        if let Some(h) = mined_height(txid)? {
            earlier_mined = true;
            earlier_buried = buried(h);
            break;
        }
    }
    if !earlier_mined {
        // all-expired→Requeued TERMINAL (v-5c device-proof FINDING #1): NO tx of the group
        // mined and EVERY tx is past its expiry AND that expiry is BURIED (`tip - expiry >=
        // REORG_MAX_BLOCKS`, the round-2 #6 margin) ⇒ nothing in the group can ever enter a
        // block, even across the deepest reorg the wallet auto-recovers: the send is dead WHOLE,
        // no money moved — reset the row to `Queued` for a fresh re-propose (the intent store's
        // documented third `Sent` exit, and since S7 C1 the ONLY one that sends it again). WHY
        // THE MARGIN (double-spend hinge): the re-propose may select other inputs, and at bare
        // `expiry <= tip` a reorg back below the expiry could revive a retained copy of the old
        // group ALONGSIDE the new send (recipient paid twice). Between bare expiry and burial
        // the row WAITS (`legs_expired` → `AwaitingBurial`, below the deposit hold). CONSERVATIVE
        // on an unprovable leg: an unreadable (`None`, pruned) or absent (`0`) expiry does not
        // prove it dead, so fall through — `Awaiting` on the re-broadcast read, never a requeue.
        // Deadline-AGNOSTIC on purpose: a requeued lapsed swap deposit is deleted by the drain's
        // own `deposit_gate` before any propose (`prepare_queued`) — its terminal path.
        let mut all_legs_dead_buried = true;
        for &txid in group {
            match read_tx_expiry(wdb, txid)? {
                Some(expiry) if past_expiry(expiry, tip) && buried(expiry) => {}
                _ => {
                    all_legs_dead_buried = false;
                    break;
                }
            }
        }
        if all_legs_dead_buried {
            crate::intent_store::reset_to_queued(aux, id)?;
            return Ok(Reconciled::Requeued);
        }
        legs_expired = true;
        for &txid in group {
            legs_expired &= matches!(read_tx_expiry(wdb, txid)?, Some(e) if past_expiry(e, tip));
        }
    } else {
        // expiry→Stranded TERMINAL (round-2 BLOCKER #1 — "the state machine MUST terminate" — +
        // #6 burial): with an earlier tx mined the final tx can never complete the send if it
        // (a) is past its expiry (`expired` above), OR (b) is GONE from the store
        // (pruned/collected) so it can never even be re-broadcast. EITHER way the funds
        // stranded on the ephemeral address. But only commit the terminal `Stranded` once the
        // earlier (irreversible) tx is BURIED; while it is shallow, WAIT — a reorg could still
        // un-mine it into the `!earlier_mined` arm above (never busy-loop a dead tx either
        // way). A single-step group never reaches here (it is the `!earlier_mined` arm — the
        // mutual exclusion is structural). The expiry-known and pruned cases are folded so a
        // pruned final tx can never leave the row Awaiting-forever (the termination hole the
        // review caught).
        let final_unminable = match read_tx_expiry(wdb, final_txid)? {
            Some(expiry) => past_expiry(expiry, tip), // past its validity window
            None => true,                             // pruned — un-broadcastable, stuck
        };
        if final_unminable {
            if earlier_buried {
                crate::intent_store::mark_stranded(aux, id)?;
                return Ok(Reconciled::Stranded);
            }
            // Earlier tx mined but still SHALLOW — do not strand yet (a reorg could un-mine it back
            // into the re-broadcast / burial-gated requeue); wait for it to bury.
            return Ok(Reconciled::AwaitingBurial);
        }
    }

    // THE DEPOSIT DEADLINE HOLD (§4.4 W-swap-4-a-2 rule 2) — checked AFTER every terminal arm
    // above (a mined-buried deposit still deletes; an all-legs-dead-buried one still requeues
    // into the `deposit_gate` delete; a stranded shape still strands) and ONLY in place of the
    // network (re)broadcast below. Post-FR-23-a the sign (execute) and the FIRST broadcast
    // (drain) are decoupled, so this arm could otherwise FIRST-broadcast a signed deposit into
    // a DEAD quote (the provider refunds it: a fee-burning transparent round-trip). `Expired`
    // holds; `Wait` (unsynced clock — the deadline is untimeable) holds fail-safe and re-checks
    // next pass; an ordinary send (`None` deadline) is `Proceed`. Bounded, never a wedge: a held
    // START expires, its expiry buries, and the requeue→`deposit_gate` terminal resolves it; a held
    // CONTINUATION whose predecessor is mined resolves to `Stranded` once that buries.
    //
    // START vs CONTINUE, PER LEG (the 2026-10-05 review F01, plan §2.2): each unmined leg is judged
    // at its own position in the RECORDED group. It continues — the 90 s continue gate, not the
    // 240 s start gate — when it, or the leg before it, is already on the network: mined, or
    // carrying the accepted mark. A re-sent accepted tx0 answers `Rejected`, so the in-call
    // answer cannot carry this; the mark does. The hold applies the FIRST unmined leg's gate.
    // ONE RULE, THREE SITES (each over its own data): this one, `wallet::DepositCheck::passes`
    // (adds the in-call `Accepted`) and `delivery::DeliveryView::state_of` (the user-visible
    // mirror). Change what counts as "on the network" in all three together.
    let mut unmined = Vec::new();
    let mut continues = Vec::new();
    for (i, &txid) in group.iter().enumerate() {
        if mined_height(txid)?.is_some() {
            continue;
        }
        // Only a deposit has two gates; an ordinary send proceeds through both, so it never
        // reads the marks.
        let leg_continues = if deposit_deadline.is_some() {
            let on_network = |t: [u8; 32]| -> Result<bool, WalletError> {
                Ok(mined_height(t)?.is_some() || crate::delivery::is_accepted(aux, &t)?)
            };
            let prev_on_network = match i.checked_sub(1) {
                Some(p) => on_network(group[p])?,
                None => false,
            };
            prev_on_network || on_network(txid)?
        } else {
            false
        };
        continues.push(leg_continues);
        unmined.push(txid);
    }
    let first_continues = continues.first().copied().unwrap_or(false);
    let gate = if first_continues {
        deposit_gate_continue(deposit_deadline, now_unix)
    } else {
        deposit_gate(deposit_deadline, now_unix)
    };
    if !matches!(gate, DepositGate::Proceed) {
        return Ok(Reconciled::DepositBroadcastHold);
    }

    // Every leg past its expiry, the expiry not yet buried (S7 C1): no leg can enter a block on
    // this branch, so no network — the burial-gated requeue above ends the wait. A reorg that
    // lowers the tip makes `past_expiry` false again, and the re-broadcast below resumes.
    if legs_expired {
        return Ok(Reconciled::AwaitingBurial);
    }

    // Re-broadcast the UNMINED txs IN ORDER (skip-mined). A needed tx missing from the store
    // (pruned/expired) ⇒ conservative `Awaiting`, never a re-queue (a note is spent by a live tx).
    // (`unmined` already skipped every mined leg — never re-broadcast a mined/already-known tx.)
    let mut to_send = Vec::new();
    for &txid in &unmined {
        match read_raw_tx(wdb, txid)? {
            Some(raw) => to_send.push(raw),
            None => return Ok(Reconciled::Awaiting),
        }
    }
    // The final tx is proven UNMINED above, so the loop either pushed it (present) or already
    // returned `Awaiting` (its bytes missing) — `to_send` is therefore always non-empty here. The
    // `debug_assert` makes that invariant visible + caught in tests without shipping a dead live
    // branch on the money path (review NIT); an empty `Rebroadcast` would be a harmless no-op anyway.
    debug_assert!(
        !to_send.is_empty(),
        "rebroadcast_group: the unmined final tx is always pushed (or we returned Awaiting)",
    );
    Ok(Reconciled::Rebroadcast {
        txs: to_send,
        continues,
    })
}

/// Reconcile ONE in-flight intent (`Submitting`/`Sent`) on crash-recovery (§6.3). Computes the
/// spend witness over EVERY claim note (AND — a single not-spendable note means "spent", so do
/// not re-propose), decides via [`reconcile`], and EXECUTES the DB side: `reset_to_queued`
/// (re-propose fresh) in the `ReProposeFresh` arm, or hands the recorded tx GROUP to
/// [`rebroadcast_group`] (skip-mined ordered re-broadcast, delete-on-FINAL-mined, or the
/// expiry→`Stranded` terminal) in the `ReBroadcast` arm. Generic over the DB so the funded
/// harness drives the production logic.
///
/// **delete-on-FINAL-BURIED / Stranded (the §6.3 `delete` B-OBLIGATION + §3.2i-2 pt 4 / round-2
/// #1 + #6):** see [`rebroadcast_group`] — a row is deleted ONLY when its FINAL tx mined AND buried
/// beyond `REORG_MAX_BLOCKS` (a re-broadcast-but-unmined OR shallow-mined tx is NEVER deleted); a
/// partially-mined multi-step whose later tx expired moves to terminal `Stranded` once the earlier
/// tx is buried (never a busy-loop of a dead tx, never a false terminal); a group with NO tx mined
/// and EVERY tx past its BURIED expiry resets to `Queued` (v-5c finding #1 — dead whole, no money
/// moved: the third `Sent` exit and the only requeue of a `Sent` row, whatever the witness reads —
/// before burial it waits, `AwaitingBurial`, S7 C1); a recorded tx gone from the store (a
/// `read_raw_tx` miss) is left `Awaiting`, NOT re-queued (a note may be spent by a live tx, so
/// re-proposing could double-spend a tx that may still mine).
///
/// **The §4.4 deposit deadline (inc-2d-swap-a + W-swap-4-a-2):** `now_unix` is the pass's
/// wall-clock, consumed by the [`deposit_gate`] decision in BOTH arms. `ReProposeFresh`: a
/// SWAP-DEPOSIT intent whose deadline has lapsed (`DepositGate::Expired`) is DELETED there
/// (`DepositExpired`) instead of re-queued — re-proposing would feed a dead quote; one whose
/// clock is untrustworthy (`DepositGate::Wait`) is left untouched and re-checked next pass.
/// `ReBroadcast`: the terminal decisions (delete-on-buried / requeue / strand) stay
/// deadline-agnostic — an already-CREATED deposit tx already left the shielded pool, so it
/// reconciles normally — but the network (re)broadcast TAIL is gated: a non-`Proceed` gate
/// HOLDS it ([`Reconciled::DepositBroadcastHold`]), because post-FR-23-a the FIRST broadcast
/// can be deferred to this pass and must never first-feed a dead quote (the tx then expires, its
/// expiry buries → the group requeue → the drain's `deposit_gate`/purge deletes the lapsed row).
pub(crate) fn reconcile_inflight<DbT>(
    wdb: &DbT,
    aux: &mut rusqlite::Connection,
    intent: &InFlightIntent,
    now_unix: u64,
) -> Result<Reconciled, WalletError>
where
    DbT: WalletRead<Error: ClassifyStoreFault> + InputSource<Error: ClassifyStoreFault>,
{
    let mut all_spendable = true;
    for claim in &intent.claims {
        if !note_spendable(wdb, claim)? {
            all_spendable = false;
            break;
        }
    }
    match reconcile(all_spendable, &intent.txids) {
        // No group recorded and the notes spendable ⇒ the tx was never created (a `Submitting`
        // row). Re-queue for a fresh re-propose; `reset_to_queued` clears the stale claim. EXCEPT a
        // swap deposit whose §4.4 gate says otherwise: a lapsed quote would be fed by a fresh
        // re-propose, so delete it instead; an untrustworthy clock can't decide, so wait. The funds
        // are safe in every branch — these notes are spendable (no tx was ever created).
        Recovery::ReProposeFresh => match deposit_gate(intent.deposit_deadline, now_unix) {
            DepositGate::Expired => {
                crate::intent_store::delete(aux, intent.id)?;
                Ok(Reconciled::DepositExpired)
            }
            // Unsynced clock: leave the row untouched and re-check next pass (reuse `Awaiting` —
            // "do nothing this pass", here awaiting a trustworthy clock rather than mine-or-expire).
            DepositGate::Wait => Ok(Reconciled::Awaiting),
            DepositGate::Proceed => {
                crate::intent_store::reset_to_queued(aux, intent.id)?;
                Ok(Reconciled::Requeued)
            }
        },
        // A group is recorded ⇒ the send WAS created; reconcile it over the chain whatever the
        // witness reads (S7 C1: skip-mined ordered re-broadcast — deadline-held for a lapsed
        // deposit, delete-on-final-buried, the burial-gated requeue, or the expiry→Stranded
        // terminal — §3.2i-2 pt 4 + round-2 #1/#6). A single-step send is a one-element group.
        Recovery::ReBroadcast(group) => rebroadcast_group(
            wdb,
            aux,
            intent.id,
            &group,
            intent.deposit_deadline,
            now_unix,
        ),
        // Spent, no recorded txid (the create-committed-but-unrecorded window) ⇒ wait for expiry.
        Recovery::AwaitWitness => Ok(Reconciled::Awaiting),
    }
}

/// `true` iff `addr` is a TEX (ZIP-320) recipient — distinct from an ordinary transparent
/// (p2pkh/p2sh) address, which `ZcashAddress::is_transparent_only` cannot tell apart. A tiny
/// [`TryFromAddress`](zcash_address::TryFromAddress) probe whose ONLY accepting arm is
/// `try_from_tex`: every other address kind takes the trait's default `Unsupported` arm, so the
/// conversion succeeds for a TEX address and ONLY a TEX address. Used by [`normalize_tex_first`] and
/// (`pub(crate)`, 2e-2b-v-3) the parked-TEX classifier [`crate::parked::classify`] — the SSOT for
/// "this recipient makes the send a gated two-step", so the surface can never drift from the producer.
pub(crate) fn is_tex_recipient(addr: &zcash_address::ZcashAddress) -> bool {
    struct TexProbe;
    impl zcash_address::TryFromAddress for TexProbe {
        type Error = std::convert::Infallible;
        fn try_from_tex(
            _net: zcash_protocol::consensus::NetworkType,
            _data: [u8; 20],
        ) -> Result<Self, zcash_address::ConversionError<Self::Error>> {
            Ok(TexProbe)
        }
    }
    addr.clone().convert::<TexProbe>().is_ok()
}

/// Round-2 #9 — mixed-recipient ORDER NORMALISATION. The engine only proposes a mixed
/// TEX + shielded send when the TEX legs occupy the LOWEST payment indices: it keys step1's
/// `payment_pools` by the ORIGINAL request index, but `TransactionRequest::new` re-enumerates the
/// step1 payments from 0, so a shielded-FIRST mixed request trips `PaymentPoolsMismatch` →
/// fail-closed `ProposeFailed` (pinned by `tex_mixed_shielded_first…`). A legitimately-ordered
/// exchange URI silently refusing to send is a real bug once multi-step SIGNs, so re-order the
/// payments TEX-leg-first before [`propose_core`], preserving each leg's exact `(address, amount,
/// memo, label, message, other_params)` tuple by CLONE (no field reconstruction — never alter the
/// money).
///
/// IDENTITY for every NON-mixed request (all-TEX, all-shielded, all-transparent, single-leg, or no
/// TEX leg at all): only a genuine TEX⊕non-TEX mix is re-ordered, and within each group the
/// relative order is preserved (`BTreeMap` iterates by ascending index = the request order; the
/// partition is stable). A single-step send's payment order is immaterial to the engine, so this
/// is safe to apply on the shared propose input. Reconstruction of an already-validated request
/// cannot fail, but a defensive failure falls back to the original (then `propose_core` fails
/// closed — money-safe).
///
/// `pub(crate)` because BOTH send funnels apply it at their OWN call site, symmetrically: the queued
/// drain ([`prepare_queued`]) and the interactive `Wallet::propose` lowering (2e-2b-v-1).
/// [`propose_core`] itself stays RAW — it does NOT normalise — so the engine-quirk pin
/// `tex_mixed_shielded_first_is_propose_failed_engine_ordering_quirk` keeps documenting the raw
/// behaviour; the normalisation POLICY lives with the callers, never inside the chokefn.
pub(crate) fn normalize_tex_first(
    request: zip321::TransactionRequest,
) -> zip321::TransactionRequest {
    let (tex, rest): (Vec<_>, Vec<_>) = request
        .payments()
        .values()
        .cloned()
        .partition(|p| is_tex_recipient(p.recipient_address()));
    // No mix to re-order: a pure-TEX / no-TEX request keeps its exact original ordering.
    if tex.is_empty() || rest.is_empty() {
        return request;
    }
    let reordered: Vec<zip321::Payment> = tex.into_iter().chain(rest).collect();
    match zip321::TransactionRequest::new(reordered) {
        Ok(normalized) => normalized,
        // Reconstruction of an ALREADY-VALIDATED request (its legs cloned whole) cannot fail; fall
        // back to the original so `propose_core` fails closed — money-safe. A debug build trips here
        // so a contract violation surfaces in a test, never silently swallows the #9 fix in the field.
        Err(_) => {
            debug_assert!(
                false,
                "re-ordering an already-validated request must reconstruct"
            );
            request
        }
    }
}

/// Drive ONE `Queued` intent through the §6.3 guarded send: re-propose (TEX-order-normalised) →
/// [`drain_multi`] (claim → `mark_submitting` → create+persist the ordered group → `mark_sent_multi`
/// → read the bytes). A TEX two-step drains here exactly like a single-step send (its slice-A park
/// gate is gone, 2e-2b-v-5); an ephemeral gap-limit ceiling parks the row `Queued` and returns
/// [`Prepared::Ceiling`] (recoverable + cancellable, dual-natured — see the variant doc), never a
/// hard terminal; a TEX intent at its #315 attempt cap parks as [`Prepared::CappedParked`] BEFORE
/// any propose (the gate below). Generic over the DB + provers EXACTLY like
/// [`propose_core`]/[`create_signed_core`], so the funded harness drives it.
///
/// **The load-bearing ordering** (money-safety, in [`drain_multi`]): `mark_submitting` persists the
/// claim BEFORE the engine creates the tx (a crash in between recovers via the witness →
/// `ReProposeFresh`), and `mark_sent_multi` records the ordered group BEFORE the broadcast — so a
/// broadcast tx ALWAYS has a recorded txid (delete-on-mined can find it), and an un-recorded tx is
/// necessarily un-broadcast, hence expires and self-heals. A create fault leaves NO note spent (the
/// engine persists TXS only on full success — though a two-step's ephemeral RESERVATION is already
/// committed and never rolled back, which is why the attempt was counted), so the row is reset to
/// `Queued` and retried — never a stuck `Submitting`, and never an unbounded leak (the cap).
///
/// `usk` is consumed into `create_signed_core` (the §4.2 confinement is the core's); the caller
/// derives it transiently inside its `run_blocking` closure and moves it straight in.
// The arg list threads the audited propose + create inputs (DB, params, aux store, account,
// request, spend key, the two provers, intent id) — each a distinct typed dependency this fn
// genuinely needs; the signature deliberately MIRRORS `create_signed_core` (which it wraps).
// Bundling them into a struct purely to satisfy the lint would obscure the money path.
#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_queued<DbT, P, SP, OP>(
    wdb: &mut DbT,
    params: &P,
    aux: &mut rusqlite::Connection,
    account_id: <DbT as InputSource>::AccountId,
    request: zip321::TransactionRequest,
    usk: UnifiedSpendingKey,
    spend_prover: &SP,
    output_prover: &OP,
    intent_id: QueuedSendId,
    deposit_deadline: Option<i64>,
    repropose_attempts: i64,
    now_unix: u64,
    // §3.2: the drain is a SIGNING path. It is the one the offline-first flow
    // uses, so the verdict must be re-checked HERE, at drain time — a send
    // composed and queued a week ago is signed against today's network, not the
    // one that existed when the user tapped Send.
    permit: crate::consensus::SigningPermit,
) -> Result<Prepared, WalletError>
where
    DbT: WalletRead
        + InputSource<Error = <DbT as WalletRead>::Error>
        + WalletWrite
        + WalletCommitmentTrees,
    <DbT as InputSource>::NoteRef: Copy + Eq + Ord,
    // Threads the round-2 #7 gap-limit bound into the create call (see `create_signed_core`).
    <DbT as WalletRead>::Error: GapLimitProbe + ClassifyStoreFault,
    P: Parameters + Clone,
    SP: SpendProver,
    OP: OutputProver,
{
    // §4.4 deposit deadline (inc-2d-swap-a), BEFORE any propose / note selection / signing. An
    // ordinary send (`None`) proceeds. A lapsed deposit is DELETED (terminal — never feed a dead
    // quote; the user re-quotes). A deposit on an untrustworthy (unsynced) clock is left Queued to
    // retry once the clock syncs — never fed by a deadline we can't time-check, never dropped.
    match deposit_gate(deposit_deadline, now_unix) {
        DepositGate::Expired => {
            crate::intent_store::delete(aux, intent_id)?;
            return Ok(Prepared::DepositExpired);
        }
        DepositGate::Wait => return Ok(Prepared::Retry),
        DepositGate::Proceed => {}
    }
    // #315 slice-1 leak cap, AFTER the deposit gate (a capped swap deposit must still self-expire
    // at its quote deadline above — parked-invisible forever otherwise, since a deposit is excluded
    // from `list_parked_sends`) and BEFORE `propose_core` (a capped row pays no propose/build/seed
    // cost — the park is the cheap steady state). `repropose_attempts` only ever increments on an
    // ephemeral-reserving attempt (`mark_submitting_counting`, two-step only), so `attempts > 0 ⟺
    // TEX` and no shape re-check is needed here. At the cap the intent PARKS honestly
    // (`CappedParked`): still `Queued`, surfaced `paused`, cancellable, resumable via the retry
    // affordance — the drain just stops burning one engine gap slot per pass on it.
    if repropose_attempts >= crate::constants::MAX_TEX_REPROPOSE_ATTEMPTS {
        return Ok(Prepared::CappedParked);
    }
    // Re-propose FRESH against the current tip (§6.2 — never the queue-time anchor). An
    // InsufficientFunds re-propose is the QueuedSendStale signal: surfaced, the intent LEFT
    // queued (funds may still arrive). Any other propose fault is transient ⇒ retry next pass.
    // `normalize_tex_first` (round-2 #9) re-orders a mixed TEX+shielded request TEX-leg-first so a
    // legitimately-ordered exchange URI does not silently `ProposeFailed` on the engine's index
    // quirk; it is identity for every non-mixed request (the common case).
    let proposal = match propose_core(wdb, params, account_id, normalize_tex_first(request)) {
        Ok(p) => p,
        Err(WalletError::InsufficientFunds { .. }) => return Ok(Prepared::Stale),
        Err(_) => return Ok(Prepared::Retry),
    };
    // STRUCTURAL SHAPE GUARD (the SSOT, mirroring `summarize` on the interactive path): the ONLY
    // multi-step the SDK signs is the recognised ZIP-320 TEX two-step (the `[tx0, tx1]` ephemeral
    // pair the recovery machine is built for). A queued send NEVER passes through `summarize` (that
    // builds the interactive confirm DTO), so this is the queued path's OWN fail-closed check — it
    // is NOT the removed slice-A gate (which blanket-rejected EVERY `steps > 1`, INCLUDING the
    // legitimate two-step). DEFENSIVE / currently-unreachable: the engine today emits only a single
    // step or a clean ZIP-320 two-step — even MULTIPLE TEX recipients BATCH into ONE ephemeral pair
    // (step1 pays them all), still the recognised shape. A non-ZIP-320 multi-step would take a FUTURE
    // engine change; this guard (exactly like `summarize`'s) makes that change fail closed instead of
    // signing a shape the `[tx0, tx1]` recovery cannot cover. On that path: `Prepared::Unsupported`
    // (money-safe — no note spent, the row left `Queued`; a STABLE shape, so it is counted as a
    // permanently-wedged `corrupt` send, never a churning `Retry` masquerading as progress). The
    // recognised two-step flows through to `drain_multi` below exactly like a single-step send.
    if proposal.steps().len() > 1 && is_zip320_two_step(&proposal).is_none() {
        return Ok(Prepared::Unsupported);
    }
    // A recognised TEX two-step now DRAINS like any other send — the slice-A park gate is REMOVED
    // (2e-2b-v-5, §251). Its recovery preconditions are all in place (the re-broadcast machine + the
    // round-2 #7 gap-limit ceiling surfaced as `Prepared::Ceiling`, NOT a silent brick, + the manual
    // ephemeral-sweep recovery + the parked-TEX surface + the cancel escape hatch). A persistent
    // ceiling parks the row `Queued` (visible via `list_parked_sends`, cancellable), never the old
    // gate-era `Blocked` terminal (now gone).
    //
    // Record the durable claim, create+persist the tx GROUP, record it, and read the bytes — all in
    // the load-bearing money order (`mark_submitting` BEFORE create, `mark_sent_multi` BEFORE
    // broadcast). `drain_multi` handles ANY group size: a one-tx single-step send or the `[tx0, tx1]`
    // chain of a TEX two-step (`create_signed_core` signs both, behind its own defensive shape guard).
    drain_multi(wdb, aux, &proposal, intent_id, |db| {
        create_signed_core(
            db,
            params,
            usk,
            &proposal,
            spend_prover,
            output_prover,
            permit,
        )
    })
}

/// Drain an already-enrolled `Queued` intent into the §6.2 outbox: record the durable claim
/// (`mark_submitting` BEFORE create — the double-send witness), create+persist the engine's
/// ordered tx GROUP (one tx for a single-step send, `[tx0, tx1]` for a TEX two-step), record the
/// group (`mark_sent_multi` BEFORE broadcast — every broadcast tx then has a recorded txid the
/// delete-on-mined recovery can find), and read each persisted tx IN ORDER for the orchestrator.
///
/// `create` is INJECTED (it captures the DB-mutating [`create_signed_core`] call in production, a
/// gate-free engine call in the funded harness) so this ONE drain body is exercised over a REAL
/// two-step while production stays gated. The borrow is sound: `create(db)` holds `&mut DbT` only
/// for its call; once it returns the persisted-bytes read takes `&DbT`.
///
/// On a create fault NO note is spent (the engine persists TXS only on full success), so the row
/// is reset to `Queued` and retried fresh next pass — never a stuck `Submitting`. NB for a
/// two-step the same is NOT true of the EPHEMERAL RESERVATION: the engine reserves the index
/// BEFORE tx construction and deliberately never rolls it back ("not worth the complexity of
/// being able to unreserve them" — `zcash_client_backend` `create_proposed_transactions`), so a
/// faulting two-step create leaks one gap slot per attempt even though the money is untouched.
/// That is why an ephemeral-reserving attempt is COUNTED here (`mark_submitting_counting`, #315
/// slice 1) — the generic `Err(_) => Retry` arm below would otherwise leak one slot per ~20 s
/// poll pass under a persistent create fault (mobile memory pressure, full DB) and brick TEX for
/// the account in minutes, invisibly. This DIVERGES from the interactive
/// [`create_two_step_enrolled`], which DELETEs on a create fault (the synchronous user
/// re-proposes, there is no queued send to drain); a queued row's self-heal is exactly the
/// re-propose — now bounded by the `CappedParked` gate in [`prepare_queued`]. The ONE create
/// error treated distinctly is the ZIP-320 ephemeral gap-limit ceiling
/// ([`WalletError::TexSendLimitReached`], round-2 #7): the row is still reset to `Queued` (the claim
/// must release) AND the pessimistic count is REFUNDED in the same write (the typed refusal is
/// the engine's proof nothing was reserved — a ceiling wait must never consume leak budget), and
/// it returns [`Prepared::Ceiling`] rather than `Retry` so a money-incomplete availability
/// ceiling is never logged as benign progress (reachable now that gate-removal lets a TEX reach
/// this create — a single-step never reserves an ephemeral).
///
/// The two state-guard misses (`mark_submitting`/`mark_sent_multi` returning `Ok(false)` — the row
/// is no longer in the expected state) are unreachable under the single-writer post-pass contract,
/// but never trusted on the money path: each leaves the row durable and `Retry`s rather than
/// broadcasting an unrecorded tx (`mark_sent_multi`-false returns BEFORE broadcast, so the
/// persisted txs are persisted-but-UNBROADCAST → cannot mine → expire → free their notes; no
/// strand). A production group is a single tx for an ordinary send or the `[tx0, tx1]` chain of a
/// TEX two-step — both flow through this SAME body now that the slice-A park gate is gone (gate-
/// removal, 2e-2b-v-5).
fn drain_multi<DbT, N, F>(
    wdb: &mut DbT,
    aux: &mut rusqlite::Connection,
    proposal: &Proposal<StandardFeeRule, N>,
    intent_id: QueuedSendId,
    create: F,
) -> Result<Prepared, WalletError>
where
    DbT: WalletRead<Error: ClassifyStoreFault>,
    F: FnOnce(&mut DbT) -> Result<Vec<TxId>, WalletError>,
{
    let claims = claim_from_proposal(proposal)?;
    // #315 slice 1: an EPHEMERAL-RESERVING create attempt is COUNTED, atomically with the
    // Queued→Submitting transition (one IMMEDIATE UPDATE — no kill window that under-counts).
    // The shape test is `steps() > 1`: the `prepare_queued` shape guard already refused every
    // multi-step that is not the recognised ZIP-320 two-step, so here multi-step ⟺ TEX ⟺ the
    // create below will reserve a fresh ephemeral index the engine never gives back. Counting at
    // THIS funnel — not at the requeue sites — covers all three leak paths with one counter: the
    // burial-gated group requeue, the witness `ReProposeFresh` requeue, and the fast create-fault
    // `Retry` arm below (which no requeue-site counter would ever see).
    let marked = if proposal.steps().len() > 1 {
        crate::intent_store::mark_submitting_counting(aux, intent_id, &claims)?
    } else {
        crate::intent_store::mark_submitting(aux, intent_id, &claims)?
    };
    if !marked {
        return Ok(Prepared::Retry);
    }
    let txids = match create(wdb) {
        Ok(t) => t,
        Err(WalletError::TexSendLimitReached) => {
            // Round-2 #7: the ZIP-320 ephemeral-address gap-limit ceiling. OTHER outstanding
            // reservations fill the engine's window, so this send cannot mint a fresh ephemeral
            // address right now — not a fault of THIS send. No note was spent AND nothing was
            // reserved — the typed refusal IS the engine's proof this attempt failed to reserve,
            // so the pessimistic count taken above is REFUNDED in the same atomic write that
            // releases the Submitting claim (`reset_to_queued_refunding_attempt`, red-team
            // F1): a ceiling wait must never consume leak budget, else a healthy TEX behind a
            // transiently-full window pauses in ~3 poll passes (~1 min) instead of self-healing
            // when a live tx0 mines — the transient→persistent over-correction the review
            // forbade. (The generic `Err(_)` arm below stays counted: a real create fault DID
            // reserve.) PARK as a DISTINCT honest outcome — never the generic `Retry` (which
            // would mislabel a money-incomplete ceiling as benign progress). The ceiling is
            // DUAL-NATURED (#315 review): a slot held by a LIVE in-flight tx0 frees when that
            // tx0 MINES (a mined first-use advances the engine's gap — self-heals, and this row
            // then drains with its budget intact), but a slot held by a LEAKED reservation (tx0
            // expired un-mined — a censoring endpoint / offline abandonment) NEVER frees on its
            // own: the engine has no un-reserve, and — contrary to the belief this comment used
            // to record — tx0 EXPIRY does NOT free it either (`find_gap_start` advances ONLY on
            // a MINED first-use; the v-5c "engine frees on expiry" hypothesis is REFUTED at the
            // pinned source). A leaked slot's only remedy is the #315 slice-2 self-mint reclaim;
            // an all-leaked window keeps this row honestly waiting at the ceiling (visible via
            // `list_parked_sends` + the pressure gauge + the per-pass `ceiling` count — the
            // window-level stuck signal), while the CappedParked budget stays reserved for
            // attempts that actually LEAK. NB the manual sweep (v-2) does NOT clear a LEAKED
            // slot either — it recovers FUNDS from a MINED/funded ephemeral (the strand); a
            // never-mined reservation holds no funds. The row stays cancellable (v-4), never a
            // silent wait. REACHABLE now that gate-removal (2e-2b-v-5) lets a queued TEX
            // two-step reach this create: a single-step send never reserves an ephemeral.
            crate::intent_store::reset_to_queued_refunding_attempt(aux, intent_id)?;
            return Ok(Prepared::Ceiling);
        }
        Err(_) => {
            // A create fault spends NO note (the engine persists txs only on full success) — but
            // for a two-step it HAS already reserved an ephemeral index that is never rolled back
            // (counted above; the cap parks a persistently-faulting intent instead of letting this
            // arm leak one slot per poll pass forever).
            crate::intent_store::reset_to_queued(aux, intent_id)?;
            return Ok(Prepared::Retry);
        }
    };
    // The ordered group (internal txid byte-order — what the witness + delete-on-final read).
    let group: Vec<[u8; 32]> = txids.iter().map(|t| *t.as_ref()).collect();
    // `mark_sent_multi` transitions Submitting → Sent; its `false` (row not Submitting) is the
    // load-bearing single-writer invariant — the caller holds the SAME `aux` lock across this whole
    // function (from `mark_submitting` above), so nothing can move the row between the two writes.
    // If a future refactor ever NARROWS that lock, this `false` becomes reachable: it still fails
    // money-safe (Retry BEFORE broadcast ⇒ the persisted txs are unbroadcast → expire → free notes),
    // but the single-writer hold is what makes it UNREACHABLE today — do not narrow it lightly.
    if !crate::intent_store::mark_sent_multi(aux, intent_id, &group)? {
        return Ok(Prepared::Retry);
    }
    // Read each just-persisted tx (present — we wrote them this txn) IN ORDER for the orchestrator
    // to broadcast over the ordered, non-latching-per-group tail.
    let mut raws = Vec::with_capacity(group.len());
    for txid in &group {
        raws.push(read_raw_tx(wdb, *txid)?.ok_or(WalletError::StoreCorrupt)?);
    }
    Ok(Prepared::Broadcast(raws))
}

/// Enrol an INTERACTIVE TEX two-step in the §6.2 outbox AROUND the create — Option A:
/// the interactive `send` path's analogue of [`prepare_queued`]'s durable
/// claim→create→record ordering, for an ALREADY-PROPOSED two-step (no re-propose — the user
/// signs the EXACT reviewed proposal, the money-honest property [`prepare_queued`]'s background
/// re-propose deliberately trades for offline durability).
///
/// **Why a two-step MUST enrol where a single-step need not.** A single-step interactive send
/// keeps no outbox row: its one tx simply expires (~40 blocks) → the spent notes free → the
/// user re-sends, losing only immediacy. A two-step CANNOT self-heal once tx0 mines — the
/// shielded notes are then irreversibly spent into the engine's ephemeral t-address, and tx1
/// must be re-broadcast or its funds strand there. So the recovery machine
/// ([`reconcile_inflight`] / `rebroadcast_group`) needs a durable `[tx0, tx1]` row to re-drive
/// the unmined tail (the #4/#5 ordered, skip-mined, non-latching re-broadcast).
///
/// **The load-bearing ordering** (money-safety, identical to [`prepare_queued`]):
/// 1. [`claim_from_proposal`] — step0's chain-stable shielded-input claim (step1 spends its
///    ephemeral via `prior_step_inputs`, contributing no claim; step0's covers the money source).
/// 2. `enqueue` the reviewed URI (`deposit_deadline = None` — an interactive send is not a swap
///    deposit). A FULL outbox is an honest terminal HERE (`QueuedSendsFull`), BEFORE any note is
///    spent — the user retries once the queue drains, never a silent drop.
/// 3. `mark_submitting` the claim BEFORE the create — the durable double-send witness: a crash
///    before the create recovers via the spend witness (notes unspent → re-propose fresh).
/// 4. `create` (injected) builds+proves+signs+persists `[tx0, tx1]` all-or-nothing.
/// 5. `mark_sent_multi` records the ordered group BEFORE the caller broadcasts, so
///    delete-on-FINAL-mined and the skip-mined re-broadcast can key on it.
///
/// A create fault leaves NO note spent (the engine persists TXS only on FULL success — though a
/// two-step create fault may already have consumed an ephemeral gap slot, which the engine never
/// rolls back; the enrol counts the attempt via `mark_submitting_counting` so a crash-recovered
/// row carries an honest #315 attempt tally), so the freshly-enrolled row is DELETED and the
/// error surfaced VERBATIM — including the round-2 #7 gap-limit ceiling
/// [`WalletError::TexSendLimitReached`], which passes through unchanged (the ceiling is
/// dual-natured — slots held by live tx0s clear as they confirm; leaked slots need the #315
/// reclaim — so the surface copy holds both). This DIVERGES from [`prepare_queued`]
/// (which resets a pre-existing Queued intent back to Queued for the next background pass): an
/// interactive send is synchronous and the user re-proposes, and this row exists ONLY to track a
/// SENT group's re-broadcast — so a never-sent attempt must leave nothing behind (no non-draining
/// two-step Queued row, no orphan). NB this "nothing behind" guarantee holds for the SYNCHRONOUS
/// create-failure arm; a process KILL between `mark_submitting` and the create leaves a Submitting
/// row that the background reconcile then `reset_to_queued`s into a Queued two-step — which
/// `prepare_queued` now DRAINS on the next pass (gate-removal, 2e-2b-v-5), re-broadcasting the
/// `[tx0, tx1]` chain (money-safe: a fresh re-propose, notes still free if nothing was signed). So
/// a crash-recovered interactive two-step heals via the SAME queued drain as a `queue_send`-origin row.
///
/// `create` is INJECTED (production passes [`create_signed_core`]; the funded harness passes the
/// raw engine call) so the success path is exercised over a REAL two-step. Post-gate-removal this
/// branch is LIVE in production: `Wallet::propose` mints a real two-step token and
/// `create_signed_core` signs it (its own `steps() > 1` gate is gone too).
pub(crate) fn create_two_step_enrolled<C>(
    aux: &mut rusqlite::Connection,
    proposal: &TransferProposal,
    uri: &str,
    created_at: i64,
    binding: Option<SpendBinding>,
    create: C,
) -> Result<Vec<TxId>, WalletError>
where
    C: FnOnce() -> Result<Vec<TxId>, WalletError>,
{
    // (1) step0's shielded claim — extracted BEFORE any durable write, so a malformed proposal
    //     fails closed without leaving an outbox row.
    let claims = claim_from_proposal(proposal)?;
    // (2) Enrol the reviewed URI. A full outbox is an honest pre-spend terminal
    //     (`QueuedSendsFull`). `binding` (FR-17): the INTERACTIVE proposal's binding rides
    //     onto the outbox row — the sign below already happened under it, and if the notes
    //     ever free (reset_to_queued) the drain RE-SIGN presents the same value the host
    //     recorded at the original authorize.
    let intent_id = crate::intent_store::enqueue(aux, uri, created_at, None, binding)?;
    // (3) Durable claim BEFORE the create (the double-send witness). The COUNTING door (#315):
    //     this function only ever enrols a ZIP-320 two-step, and the create below reserves an
    //     ephemeral index the engine never rolls back — so the attempt is tallied here exactly
    //     like the queued drain's. On the synchronous paths the count dies with the row (success
    //     → Sent → delete-on-buried; fault → the delete below), but a process KILL between here
    //     and the create leaves a Submitting row that recovers into the QUEUED drain — which
    //     must inherit an honest attempt count of 1, not 0 (the reservation may already be spent).
    if !crate::intent_store::mark_submitting_counting(aux, intent_id, &claims)? {
        // A row enqueued THIS call is Queued, so `mark_submitting` must take it; a `false` is a
        // store anomaly (unreachable under the single-writer interactive path). Delete the orphan
        // and fail closed — no note is spent yet, so nothing is lost.
        crate::intent_store::delete(aux, intent_id)?;
        return Err(WalletError::StoreCorrupt);
    }
    // (4) Build+prove+sign+persist [tx0, tx1] (all-or-nothing). On ANY fault NO note is spent.
    let txids = match create() {
        Ok(t) => t,
        Err(e) => {
            // Remove the row this call created — the synchronous user re-proposes; there is no
            // persisted tx to recover and no queued send to drain. Fail closed with the create's
            // own error (`ProposalStale` re-anchor / `SignFailed` / `StoreCorrupt`); if the delete
            // ITSELF faults, that `StoreCorrupt` supersedes (the store is then badly wrong — still
            // fail closed, still no note spent).
            crate::intent_store::delete(aux, intent_id)?;
            return Err(e);
        }
    };
    // (5) Record the ordered group (internal txid byte-order — what the witness + delete-on-final
    //     read) BEFORE the caller broadcasts.
    let group: Vec<[u8; 32]> = txids.iter().map(|t| *t.as_ref()).collect();
    if !crate::intent_store::mark_sent_multi(aux, intent_id, &group)? {
        // Not Submitting (unreachable under the single-writer interactive path — the aux lock is
        // held across this whole function, so nothing can move the row we just `mark_submitting`d).
        // Defensive only: the txs ARE persisted, but we return the error BEFORE the caller
        // broadcasts, so they are persisted-but-UNBROADCAST — they cannot mine, will expire
        // unbroadcast, and free their notes (no strand). Surface the anomaly; do NOT broadcast an
        // unrecorded group, and do NOT delete (that could orphan a persisted tx were the contract
        // ever violated). The Submitting+claim row self-heals via the witness → expiry → re-propose.
        return Err(WalletError::StoreCorrupt);
    }
    Ok(txids)
}

/// One retained proposal — the opaque token plus the freshness clock and the intent
/// to re-propose at broadcast time (§6.2). Fields are `pub(crate)` so `send`
/// (inc-2d-2) can consume them.
pub(crate) struct Retained {
    /// The opaque token `Wallet::sign_proposal` feeds to `create_signed_core`. Held now;
    /// consumed there (the public `send` wires that chain at 2d-3).
    // read by Wallet::sign_proposal — unreachable from the public API until `send` wires it at 2d-3
    pub(crate) proposal: RetainedProposal,
    /// Monotonic creation stamp for the TTL guard. `tokio::time::Instant` so the TTL
    /// is deterministically testable under `start_paused`; in production it is plain
    /// monotonic time. (Like every monotonic clock it PAUSES during device suspend —
    /// acceptable here: the authoritative staleness check is the chain anchor at
    /// `send`-time, this TTL is the UX freshness hint.)
    created: Instant,
    /// The originating intent, retained for the §6.2 re-propose-at-broadcast path
    /// (inc-2d-3). Held now so the registry is the one place the intent↔token pair
    /// lives. `None` for a [`RetainedProposal::Shield`] — a shield has no re-proposable
    /// `PaymentRequest` (its upstream request is empty; it is synchronous-only, never
    /// queued for offline re-propose — Recv-3), so there is honestly nothing to retain.
    ///
    /// **OBLIGATION — TODO(inc-2d-3):** when the broadcast/outbox re-propose path wires
    /// this field, it MUST branch on `None` (a shield token is synchronous-only — SKIP
    /// re-enqueue, never `.unwrap()`/`.expect()`): a shield's crash-safety is the engine's
    /// spent-UTXO exclusion, not the §6.2 intent re-propose. A named test for the
    /// None-for-shield contract is owed there.
    pub(crate) request: Option<PaymentRequest>,
    /// FR-17: the spend binding minted at propose (the same value on the public
    /// `SendProposal` DTO). `sign_proposal_upstream` PEEKS it pre-pull
    /// ([`ProposalRegistry::peek_binding`]) and presents it to the seed port.
    pub(crate) binding: SpendBinding,
}

/// Per-wallet retained-proposal registry (§3.2h decision 3). One-shot + TTL-bounded:
/// `propose` inserts the opaque token, `send` (inc-2d-2) consumes it exactly once.
/// Per-wallet (lives in `Inner`) so a token minted by one wallet can never be
/// consumed against another.
pub(crate) struct ProposalRegistry {
    next_id: AtomicU64,
    live: Mutex<HashMap<u64, Retained>>,
}

impl ProposalRegistry {
    pub(crate) fn new() -> Self {
        Self {
            // Ids start at 1 so a 0 id is never a valid handle (a defensive sentinel).
            next_id: AtomicU64::new(1),
            live: Mutex::new(HashMap::new()),
        }
    }

    /// Reserve the next one-shot id WITHOUT inserting. The caller summarizes the DTO
    /// with this REAL id, then [`put`](Self::put)s the token — so the public
    /// `SendProposal` is never constructed with the invalid `0` sentinel (review fold).
    /// `Relaxed` is sufficient: uniqueness needs only the atomic increment; the `live`
    /// mutex orders the insert/consume that actually gate the spend, and nothing's
    /// visibility is published THROUGH this counter.
    pub(crate) fn reserve_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Retain a TRANSFER `proposal` (+ its re-propose intent) under a previously
    /// [`reserve_id`]ed `id` — the §3.2h send path. Thin wrapper over `put_retained`
    /// that keeps the transfer call sites (and their tests) unchanged; a shield retains
    /// via `put_shield`.
    ///
    /// [`reserve_id`]: Self::reserve_id
    pub(crate) fn put(
        &self,
        id: u64,
        proposal: TransferProposal,
        request: PaymentRequest,
        binding: SpendBinding,
    ) {
        self.put_retained(
            id,
            RetainedProposal::Transfer(proposal),
            Some(request),
            binding,
        );
    }

    /// Retain a SHIELD `proposal` under a previously [`reserve_id`]ed `id` (Recv-3).
    /// A shield has no re-proposable `PaymentRequest` (synchronous-only, never queued),
    /// so `request` is `None` — the token is consumed by the SAME `send(id)`.
    ///
    /// [`reserve_id`]: Self::reserve_id
    pub(crate) fn put_shield(&self, id: u64, proposal: ShieldProposal, binding: SpendBinding) {
        self.put_retained(id, RetainedProposal::Shield(proposal), None, binding);
    }

    /// Retain a proposal token (transfer or shield) under `id`. BOUNDS the registry on
    /// every insert (crypto+security review fold): it first drops TTL-expired entries
    /// (logically dead — a consume would reject them `ProposalStale` anyway, and each holds
    /// §5.4-sensitive memo bytes that should not outlive the token), then enforces
    /// [`PROPOSAL_REGISTRY_MAX_LIVE`] by evicting the OLDEST live entries — so a
    /// propose-without-send loop can never grow it without bound, and the newest proposal
    /// (the one on screen) always survives. Both token kinds share this ONE bounded path.
    fn put_retained(
        &self,
        id: u64,
        proposal: RetainedProposal,
        request: Option<PaymentRequest>,
        binding: SpendBinding,
    ) {
        let ttl = Duration::from_secs(PROPOSAL_TTL_SECS);
        let mut live = self.live.lock().expect("proposal registry mutex poisoned");
        // Opportunistic sweep: TTL-expired entries are already un-consumable.
        live.retain(|_, r| r.created.elapsed() < ttl);
        // Hard cap even within the TTL window: evict the oldest until there is room. The
        // "newest survives" property (the on-screen proposal is never evicted) rests on
        // distinct `created` instants — guaranteed in production because each propose does
        // a DB selection read between inserts (ms of real monotonic time apart); only a
        // virtual-clock test with zero `advance` could tie them, and even then the cap
        // (bounded length) still holds, only the tie-break is then arbitrary.
        while live.len() >= PROPOSAL_REGISTRY_MAX_LIVE {
            let Some(oldest) = live
                .iter()
                .min_by_key(|(_, r)| r.created)
                .map(|(&oid, _)| oid)
            else {
                break;
            };
            live.remove(&oldest);
        }
        live.insert(
            id,
            Retained {
                proposal,
                created: Instant::now(),
                request,
                binding,
            },
        );
    }

    /// FR-17: the binding of a LIVE proposal, WITHOUT consuming it — the pre-pull
    /// peek `sign_proposal_upstream` runs so (a) the pull presents THE
    /// id-being-signed's binding and (b) a dead id fails typed BEFORE any seed
    /// pull (a dead id must never burn the host's take-once stage). Mirrors
    /// [`consume`](Self::consume)'s error taxonomy; unlike `consume` a stale hit
    /// is NOT removed here — `consume` stays the one-shot authority that burns it.
    pub(crate) fn peek_binding(&self, id: u64) -> Result<SpendBinding, WalletError> {
        let live = self.live.lock().expect("proposal registry mutex poisoned");
        let retained = live.get(&id).ok_or(WalletError::ProposalAlreadyUsed)?;
        if retained.created.elapsed() >= Duration::from_secs(PROPOSAL_TTL_SECS) {
            return Err(WalletError::ProposalStale);
        }
        Ok(retained.binding)
    }

    /// Reserve + put a TRANSFER token in one step (the test/standalone convenience; the
    /// public `Wallet::propose` uses [`reserve_id`](Self::reserve_id) + [`put`](Self::put)
    /// so it can stamp the DTO with the real id before retaining the token).
    #[cfg(test)]
    pub(crate) fn insert(&self, proposal: TransferProposal, request: PaymentRequest) -> u64 {
        let id = self.reserve_id();
        self.put(id, proposal, request, SpendBinding::mint());
        id
    }

    /// Reserve + put a SHIELD token in one step (the test convenience mirror of [`insert`];
    /// the public `Wallet::propose_shield` uses `reserve_id` + [`put_shield`]).
    ///
    /// [`insert`]: Self::insert
    #[cfg(test)]
    pub(crate) fn insert_shield(&self, proposal: ShieldProposal) -> u64 {
        let id = self.reserve_id();
        self.put_shield(id, proposal, SpendBinding::mint());
        id
    }

    /// Consume the proposal `id` exactly once (the §3.2h one-shot + TTL contract).
    /// Removes it on EVERY outcome (so a stale token is also burned, never replayable):
    /// a missing id ⇒ `ProposalAlreadyUsed` (a double-tap on send), a live id past
    /// [`PROPOSAL_TTL_SECS`] ⇒ `ProposalStale` (re-propose for a fresh fee). Consumed
    /// by the public `send` (inc-2d-2); exercised at this seam now.
    ///
    /// **SAFETY-OBLIGATION (inc-2d-2) — a returned `Ok` is NOT a freshness guarantee.**
    /// `created` is a monotonic `tokio::time::Instant`, which PAUSES during device suspend
    /// (mobile background; desktop laptop-lid hibernate is the same suspend class). So a
    /// phone that sleeps for an hour mid-proposal can wake with a token whose TTL never
    /// elapsed, yet whose chain anchor (`proposal.min_target_height`) is now many blocks
    /// stale. The wall-clock TTL is only a cheap UX freshness hint; the AUTHORITATIVE
    /// staleness check is the chain anchor. The inc-2d-2 `send` MUST, after `consume`,
    /// re-validate the proposal's target height against the LIVE chain tip and reject
    /// `ProposalStale` if it has drifted past the anchor window — never sign against the
    /// suspend-extended TTL alone (the failure mode is a tx built to a dead anchor, burned
    /// to expiry, funds locked until the expiry height — §6.2). This obligation is
    /// height-driven, so it is untestable with virtual time (suspend FREEZES the clock,
    /// the opposite of `advance`). DISCHARGED at inc-2d-2: `create_signed_core` runs
    /// [`assert_proposal_anchor_fresh`] (the live-tip re-anchor) BEFORE any signing, and
    /// the height-fixture test `proposal_resumed_after_suspend_must_reanchor_at_send`
    /// (below) is now a live `#[test]` proving it. The obligation persists into the
    /// public `send` wiring: it MUST call the core (which re-anchors), never sign on a
    /// `consume`d token alone.
    pub(crate) fn consume(&self, id: u64) -> Result<Retained, WalletError> {
        let retained = self
            .live
            .lock()
            .expect("proposal registry mutex poisoned")
            .remove(&id)
            .ok_or(WalletError::ProposalAlreadyUsed)?;
        if retained.created.elapsed() >= Duration::from_secs(PROPOSAL_TTL_SECS) {
            return Err(WalletError::ProposalStale);
        }
        // NB: reaching here means the TTL has NOT elapsed — see the SAFETY-OBLIGATION
        // above: `send` (2d-2) must STILL re-anchor against the live tip; this is not a
        // freshness guarantee on a suspend-resumed device.
        Ok(retained)
    }
}

// ── §8 gate-8 named tests (inc-2d-1 propose) ────────────────────────────────
//
// The funded-wallet harness (spec §3.2h step 0): a `data_api::testing::TestBuilder`
// over OUR own `DataStoreFactory` (producing the real `WalletDb` the live wallet uses,
// on the harness `LocalNetwork`) + a minimal in-memory `TestCache`. We mine real
// compact sapling notes to the test account and scan them deep enough to clear the
// DEFAULT spendable confirmations policy, then drive `propose_core` — the SAME code
// the live `Wallet::propose` runs. `test-dependencies` is a DEV-ONLY feature (never in
// the shipped graph; supply-chain gate).
#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    // The non-test `buried` predicate now delegates to `crate::stranded::buried` (the SSOT), so
    // REORG_MAX_BLOCKS is referenced ONLY by these burial-boundary tests.
    use crate::constants::REORG_MAX_BLOCKS;
    use rand_core::OsRng;
    use rusqlite::Connection;
    use std::collections::BTreeMap;
    use std::convert::Infallible;
    use zcash_address::{ToAddress, ZcashAddress};
    use zcash_client_backend::data_api::chain::BlockSource;
    use zcash_client_backend::data_api::chain::error::Error as ChainError;
    use zcash_client_backend::data_api::testing::{
        AddressType, CacheInsertionResult, DataStoreFactory, TestBuilder, TestCache, TestState,
    };
    use zcash_client_backend::data_api::{Account, WalletRead, WalletWrite};
    use zcash_client_backend::proto::compact_formats::CompactBlock;
    use zcash_client_sqlite::WalletDb;
    use zcash_client_sqlite::util::SystemClock;
    use zcash_client_sqlite::wallet::init::init_wallet_db;
    use zcash_keys::keys::UnifiedSpendingKey;
    // ADR-0528: the engine deps carry `transparent-inputs` UNCONDITIONALLY (our Cargo.toml),
    // so `DataStoreFactory::new_data_store` always has the `gap_limits` parameter here — it is
    // NOT a `zec-wallet-core` feature, so no `cfg` gate. The propose tests do not exercise
    // transparent gap behavior, so we honor whatever the builder passes (None ⇒ the engine's
    // default limits) and apply it exactly as the upstream `TestDbFactory` does.
    use zcash_client_backend::data_api::anchor_retention::AnchorRetentionInterval;
    use zcash_keys::keys::transparent::gap_limits::GapLimits;
    use zcash_primitives::block::BlockHash;
    use zcash_protocol::TxId;
    use zcash_protocol::consensus::{BlockHeight, NetworkType};
    use zcash_protocol::local_consensus::LocalNetwork;
    use zcash_protocol::value::Zatoshis as ProtoZat;
    use zip32::AccountId;

    /// The harness wallet DB — the SAME `WalletDb` shape the live wallet uses, on the
    /// fake `LocalNetwork` the `data_api::testing` builder requires.
    type HarnessDb = WalletDb<Connection, LocalNetwork, SystemClock, OsRng>;
    type HarnessState = TestState<MemCache, HarnessDb, LocalNetwork>;

    // ── In-memory block cache (a `TestCache` over a height→block map) ───────────

    #[derive(Default)]
    struct MemCache {
        blocks: BTreeMap<u64, CompactBlock>,
    }

    /// We do not assert on per-insert txids in any propose test (scan reads blocks via
    /// the `BlockSource`, independent of this), so the insert result carries none.
    struct NoTxids;
    impl CacheInsertionResult for NoTxids {
        fn txids(&self) -> &[TxId] {
            &[]
        }
    }

    impl BlockSource for MemCache {
        type Error = Infallible;
        fn with_blocks<F, WalletErrT>(
            &self,
            from_height: Option<BlockHeight>,
            limit: Option<usize>,
            mut with_block: F,
        ) -> Result<(), ChainError<WalletErrT, Infallible>>
        where
            F: FnMut(CompactBlock) -> Result<(), ChainError<WalletErrT, Infallible>>,
        {
            let from = from_height.map(|h| u64::from(u32::from(h))).unwrap_or(0);
            let mut remaining = limit.unwrap_or(usize::MAX);
            for (_h, cb) in self.blocks.range(from..) {
                if remaining == 0 {
                    break;
                }
                with_block(cb.clone())?;
                remaining -= 1;
            }
            Ok(())
        }
    }

    impl TestCache for MemCache {
        type BsError = Infallible;
        type BlockSource = Self;
        type InsertResult = NoTxids;

        fn block_source(&self) -> &Self {
            self
        }
        fn insert(&mut self, cb: &CompactBlock) -> NoTxids {
            self.blocks.insert(cb.height, cb.clone());
            NoTxids
        }
        fn truncate_to_height(&mut self, height: BlockHeight) {
            let h = u64::from(u32::from(height));
            self.blocks.retain(|k, _| *k <= h);
        }
    }

    // ── Our DataStoreFactory: a real `WalletDb` on an in-memory SQLite ──────────

    #[derive(Default)]
    struct InMemDsf;

    impl DataStoreFactory for InMemDsf {
        type Error = ();
        type AccountId = <HarnessDb as WalletRead>::AccountId;
        type Account = <HarnessDb as WalletRead>::Account;
        type DsError = <HarnessDb as WalletRead>::Error;
        type DataStore = HarnessDb;

        fn new_data_store(
            &self,
            network: LocalNetwork,
            anchor_retention_interval: Option<AnchorRetentionInterval>,
            gap_limits: Option<GapLimits>,
        ) -> Result<Self::DataStore, ()> {
            // In-memory keeps the whole DB inside the owned `Connection`, so the store
            // lives exactly as long as the `WalletDb` (no temp-file lifetime to manage,
            // and no SQLCipher key — these tests exercise propose logic, not at-rest
            // encryption). The rarray vtab is the one module `WalletDb` requires.
            let conn = Connection::open_in_memory().map_err(|_| ())?;
            rusqlite::vtab::array::load_module(&conn).map_err(|_| ())?;
            let mut db = WalletDb::from_connection(conn, network, SystemClock, OsRng);
            // 0.24.0's new builder knob, applied exactly as upstream's own
            // `TestDbFactory` does: `None` ⇒ the engine default (`ZIP_318`). Honoring
            // the builder's value rather than ignoring it matters — a test that asks
            // for a short retention interval and silently gets the default would
            // exercise the wrong anchor-expiry behaviour and pass.
            if let Some(interval) = anchor_retention_interval {
                db = db.with_anchor_retention_interval(interval);
            }
            // Apply the builder's gap limits exactly as upstream `TestDbFactory` does
            // (None ⇒ the engine default); transparent gap behavior is not under test here.
            if let Some(gap_limits) = gap_limits {
                db = db.with_gap_limits(gap_limits);
            }
            init_wallet_db(&mut db, None).map_err(|_| ())?;
            Ok(db)
        }
    }

    // ── Funding + propose helpers ───────────────────────────────────────────────

    const NOTE_VALUE: u64 = 60_000;
    /// Untrusted-incoming spendable depth (the conservative arm of the SSOT policy,
    /// `constants::MIN_CONFIRMATIONS`). A mined note must reach this depth to be
    /// selectable under `ConfirmationsPolicy::default()`.
    const SPENDABLE_DEPTH: usize = 10;

    fn fresh_state() -> HarnessState {
        TestBuilder::new()
            .with_data_store_factory(InMemDsf)
            .with_block_cache(MemCache::default())
            .with_account_from_sapling_activation(BlockHash([0; 32]))
            .build()
    }

    /// Mine ONE sapling note of `value` to the test account, then advance enough empty
    /// blocks that it clears `SPENDABLE_DEPTH` confirmations under the default policy,
    /// and scan the whole range. Returns the state with one spendable note.
    fn funded(value: u64) -> HarnessState {
        let mut st = fresh_state();
        let dfvk = st.test_account_sapling().cloned().expect("sapling dfvk");
        let (note_h, _, _) = st.generate_next_block(
            &dfvk,
            AddressType::DefaultExternal,
            ProtoZat::const_from_u64(value),
        );
        let mut tip = note_h;
        for _ in 0..(SPENDABLE_DEPTH - 1) {
            let (h, _) = st.generate_empty_block();
            tip = h;
        }
        st.scan_cached_blocks(note_h, SPENDABLE_DEPTH);
        st.wallet_mut()
            .update_chain_tip(tip)
            .expect("update chain tip");
        st
    }

    /// Mine ONE more sapling note of `value` at the current tip and scan just it, so it
    /// sits SHALLOW (one confirmation — below the spendable depth).
    fn add_shallow_note(st: &mut HarnessState, value: u64) {
        let dfvk = st.test_account_sapling().cloned().expect("sapling dfvk");
        let (h, _, _) = st.generate_next_block(
            &dfvk,
            AddressType::DefaultExternal,
            ProtoZat::const_from_u64(value),
        );
        st.scan_cached_blocks(h, 1);
        st.wallet_mut()
            .update_chain_tip(h)
            .expect("update chain tip");
    }

    /// Mine N sapling notes (one per block, `values[i]` each), then bury the batch under
    /// enough empty blocks that ALL clear `SPENDABLE_DEPTH`, and scan the whole range in
    /// one pass. The realistic shape of a wallet that received several payments and now
    /// wants to spend across them (the selector must aggregate notes).
    fn funded_multi(values: &[u64]) -> HarnessState {
        assert!(!values.is_empty(), "funded_multi needs at least one note");
        let mut st = fresh_state();
        let dfvk = st.test_account_sapling().cloned().expect("sapling dfvk");
        let mut first = None;
        let mut tip = None;
        for &v in values {
            let (h, _, _) = st.generate_next_block(
                &dfvk,
                AddressType::DefaultExternal,
                ProtoZat::const_from_u64(v),
            );
            first.get_or_insert(h);
            tip = Some(h);
        }
        for _ in 0..(SPENDABLE_DEPTH - 1) {
            let (h, _) = st.generate_empty_block();
            tip = Some(h);
        }
        let first = first.expect("at least one note");
        st.scan_cached_blocks(first, values.len() + (SPENDABLE_DEPTH - 1));
        st.wallet_mut()
            .update_chain_tip(tip.expect("a tip"))
            .expect("update chain tip");
        st
    }

    /// `funded_multi` with a CUSTOM ephemeral gap limit — the round-2 #7 ceiling lever. The engine
    /// reserves one ephemeral t-address per TEX tx0, bounded by this limit; shrinking it to a small
    /// value lets a REAL gap-limit exhaustion brick after just `ephemeral_gap` outstanding unmined
    /// tx0s (vs the engine default of 10), keeping the proof-heavy ceiling test cheap. external /
    /// internal stay at the engine defaults (10 / 5) so note funding + receive derivation are
    /// unaffected — only the ephemeral (TEX) scope is constrained.
    fn funded_multi_with_ephemeral_gap(values: &[u64], ephemeral_gap: u32) -> HarnessState {
        assert!(!values.is_empty(), "needs at least one note");
        let mut st = TestBuilder::new()
            .with_data_store_factory(InMemDsf)
            .with_block_cache(MemCache::default())
            .with_gap_limits(GapLimits::new(10, 5, ephemeral_gap))
            .with_account_from_sapling_activation(BlockHash([0; 32]))
            .build();
        let dfvk = st.test_account_sapling().cloned().expect("sapling dfvk");
        let mut first = None;
        let mut tip = None;
        for &v in values {
            let (h, _, _) = st.generate_next_block(
                &dfvk,
                AddressType::DefaultExternal,
                ProtoZat::const_from_u64(v),
            );
            first.get_or_insert(h);
            tip = Some(h);
        }
        for _ in 0..(SPENDABLE_DEPTH - 1) {
            let (h, _) = st.generate_empty_block();
            tip = Some(h);
        }
        let first = first.expect("at least one note");
        st.scan_cached_blocks(first, values.len() + (SPENDABLE_DEPTH - 1));
        st.wallet_mut()
            .update_chain_tip(tip.expect("a tip"))
            .expect("update chain tip");
        st
    }

    // ── inc-2d-shield (Recv-3) funding + helpers ────────────────────────────────

    /// Build a harness with a scanned chain that has NO shielded notes for us, plus ONE
    /// spendable transparent UTXO of `value` paying the account's external receive
    /// t-address — the shield-test funding (mirrors the upstream
    /// `data_api::testing::transparent::transparent_balance_spendability` shape). The
    /// default ZIP-315 `ConfirmationsPolicy` allows ZERO-conf shielding, so a tip-height
    /// UTXO is immediately shieldable. Returns the state + the account's transparent
    /// receiver (the SAME address `propose_shield_core` shields from).
    fn funded_transparent(value: u64) -> (HarnessState, TransparentAddress) {
        funded_transparent_n(&[value])
    }

    /// Build a harness with a scanned chain that has NO shielded notes for us, plus N
    /// DISTINCT spendable transparent UTXOs (`values[i]` each) paying the account's
    /// external receive t-address — the realistic exchange-withdrawal / swap-delivery
    /// shape where several UTXOs accrue at the same address. Each gets a distinct
    /// `OutPoint` so the engine tracks them as separate inputs (the shield's input-sum
    /// loop must aggregate them). Returns the state + the account's transparent receiver.
    fn funded_transparent_n(values: &[u64]) -> (HarnessState, TransparentAddress) {
        use sapling_crypto::zip32::ExtendedSpendingKey;
        use zcash_client_backend::data_api::Account;
        use zcash_client_backend::wallet::WalletTransparentOutput;
        use zcash_keys::keys::UnifiedAddressRequest;
        use zcash_transparent::bundle::{OutPoint, TxOut};

        let mut st = fresh_state();
        let account = st.test_account().cloned().expect("account");
        let uaddr = st
            .wallet()
            .get_last_generated_address_matching(
                account.id(),
                UnifiedAddressRequest::AllAvailableKeys,
            )
            .expect("address read")
            .expect("a generated UA");
        let taddr = *uaddr
            .transparent()
            .expect("the account UA carries a transparent receiver (transparent-inputs ON)");

        // A chain whose blocks DON'T pay us, so our ONLY funds are the transparent UTXOs
        // put below (a clean shield: the proposal can select nothing else).
        let not_our_key = ExtendedSpendingKey::master(&[]).to_diversifiable_full_viewing_key();
        let not_our_value = ProtoZat::const_from_u64(10_000);
        let (start, _, _) =
            st.generate_next_block(&not_our_key, AddressType::DefaultExternal, not_our_value);
        let mut tip = start;
        for _ in 0..SPENDABLE_DEPTH {
            let (h, _) = st.generate_empty_block();
            tip = h;
        }
        st.scan_cached_blocks(start, SPENDABLE_DEPTH + 1);
        st.wallet_mut()
            .update_chain_tip(tip)
            .expect("update chain tip");

        // Each UTXO pays OUR receive address, received at the chain tip with a DISTINCT
        // outpoint. Zero-conf shielding (ZIP-315) ⇒ immediately spendable for a shield.
        let height = st
            .wallet()
            .chain_height()
            .expect("tip read")
            .expect("a tip");
        for (i, &value) in values.iter().enumerate() {
            // A distinct fake txid per UTXO so the engine records them as separate inputs.
            let txid = [0x40u8.wrapping_add(i as u8); 32];
            let txout = TxOut::new(ProtoZat::const_from_u64(value), taddr.script().into());
            // `None` for the three attribution fields 0.24.0 added, for the same
            // reason `transparent::validate_fields` passes `None`: the engine derives
            // account + key scope from its own `addresses` table by the recipient
            // address and reads none of them on the put path.
            let utxo = WalletTransparentOutput::from_parts(
                OutPoint::new(txid, 0),
                txout,
                Some(height),
                None,
                None,
                None,
            )
            .expect("valid utxo");
            st.wallet_mut()
                .put_received_transparent_utxo(&utxo)
                .expect("put the funding transparent UTXO");
        }
        (st, taddr)
    }

    /// Propose a shield over the funded harness (the SAME `propose_shield_core` the live
    /// `Wallet::propose_shield` runs), from the account's receive t-address.
    fn propose_shield_on(
        st: &mut HarnessState,
        from_addr: &TransparentAddress,
    ) -> Result<Option<ShieldProposal>, WalletError> {
        use zcash_client_backend::data_api::Account;
        let network = *st.network();
        let aid = st.test_account().expect("account").account().id();
        propose_shield_core(
            st.wallet_mut(),
            &network,
            aid,
            std::slice::from_ref(from_addr),
        )
    }

    /// Drive create+sign over a SHIELD proposal on the funded harness — the sign-side mirror
    /// of [`sign_on`], over the production bundled prover (`NoteRef = Infallible`).
    fn sign_shield_on(
        st: &mut HarnessState,
        proposal: &ShieldProposal,
    ) -> Result<Vec<TxId>, WalletError> {
        let network = *st.network();
        let usk = st.test_account().expect("account").usk().clone();
        let prover = crate::prover::tx_prover();
        create_signed_core(
            st.wallet_mut(),
            &network,
            usk,
            proposal,
            prover,
            prover,
            crate::consensus::SigningPermit::assume_current_for_test(),
        )
    }

    /// Propose a SWEEP (2e-2b-v-2 economic floor) over the funded harness — the SAME
    /// `propose_sweep_core` the live `Wallet::sweep_ephemeral_funds` runs per ephemeral. The threshold
    /// logic is amount-based (scope-agnostic), so the funded-transparent harness exercises the floor.
    fn propose_sweep_on(
        st: &mut HarnessState,
        from_addr: &TransparentAddress,
    ) -> Result<Option<ShieldProposal>, WalletError> {
        use zcash_client_backend::data_api::Account;
        let network = *st.network();
        let aid = st.test_account().expect("account").account().id();
        propose_sweep_core(
            st.wallet_mut(),
            &network,
            aid,
            std::slice::from_ref(from_addr),
        )
    }

    #[tokio::test]
    async fn shield_proposal_over_threshold_signs_and_does_not_resurrect() {
        // §8 (Recv-3 — THE shield money path, funded harness, REAL prover): a transparent
        // balance above the threshold proposes a shield whose gross = the funds and whose
        // net = gross − fee; create+sign produces exactly ONE tx; and after the spend the
        // UTXO is gone, so a fresh `propose_shield` finds nothing (non-resurrection —
        // engine-guaranteed spent-UTXO exclusion, NO double-shield).
        let (mut st, taddr) = funded_transparent(200_000);

        let proposal = propose_shield_on(&mut st, &taddr)
            .expect("propose")
            .expect("Some over threshold");
        let dto = summarize_shield(&proposal, 1, SpendBinding::mint()).expect("summarize");
        assert!(dto.is_shield && !dto.has_transparent_recipient);
        assert_eq!(
            dto.total_zat, 200_000,
            "gross = the funded transparent UTXO"
        );
        assert!(dto.fee_zat > 0, "a real ZIP-317 fee");
        assert_eq!(
            dto.change_zat,
            dto.total_zat - dto.fee_zat,
            "net shielded = gross − fee"
        );

        let txids = sign_shield_on(&mut st, &proposal).expect("create+sign the shield");
        assert_eq!(txids.len(), 1, "a single-step shield mints exactly one tx");

        // Non-resurrection: the UTXO is spent by the (unexpired) shield tx, so the engine
        // excludes it — re-proposing finds nothing above the threshold.
        let again = propose_shield_on(&mut st, &taddr).expect("re-propose");
        assert!(
            again.is_none(),
            "after the shield spends the UTXO, a fresh propose_shield finds nothing — \
             never a double-shield of already-spent funds"
        );
    }

    #[tokio::test]
    async fn sweep_recovers_below_the_shield_threshold_and_does_not_resurrect() {
        // §8 (2e-2b-v-2 — THE sweep economic-floor money proof, funded harness, REAL prover): a
        // transparent balance BELOW the 0.001-ZEC convenience shield threshold but ABOVE the ZIP-317
        // fee is REJECTED by `propose_shield_core` (the completeness-gap tail — a small late return
        // would stay stranded) yet RECOVERED by `propose_sweep_core` at the economic floor; create+sign
        // produces exactly ONE tx; and after the spend the UTXO is gone, so a fresh sweep finds nothing
        // (non-resurrection — the same engine spent-UTXO exclusion, NO double-spend).
        let below_threshold = SHIELDING_THRESHOLD_ZAT as u64 - 50_000; // 50_000 zat ≪ 0.001 ZEC
        let (mut st, taddr) = funded_transparent(below_threshold);

        // The convenience shield leaves it stranded (below threshold) — the gap the sweep closes.
        assert!(
            propose_shield_on(&mut st, &taddr)
                .expect("propose shield")
                .is_none(),
            "a sub-0.001-ZEC return is below the convenience shield threshold — propose_shield skips it",
        );

        // The sweep recovers it at the economic floor.
        let proposal = propose_sweep_on(&mut st, &taddr)
            .expect("propose sweep")
            .expect("Some at the economic floor — above the ZIP-317 fee");
        let dto = summarize_shield(&proposal, 1, SpendBinding::mint()).expect("summarize");
        assert!(
            dto.is_shield && !dto.has_transparent_recipient,
            "privacy-positive consolidation"
        );
        assert_eq!(
            dto.total_zat, below_threshold as i64,
            "gross = the stranded UTXO"
        );
        assert!(dto.fee_zat > 0, "a real ZIP-317 fee");
        assert_eq!(
            dto.change_zat,
            dto.total_zat - dto.fee_zat,
            "net recovered into the shielded pool = gross − fee",
        );

        let txids = sign_shield_on(&mut st, &proposal).expect("create+sign the sweep");
        assert_eq!(
            txids.len(),
            1,
            "a single-step sweep mints exactly one tx (passes the steps()>1 gate)"
        );

        // Non-resurrection: the swept UTXO is spent, so a re-run finds nothing — idempotent, never a
        // double-spend even if the prior sweep's broadcast outcome was unknown.
        assert!(
            propose_sweep_on(&mut st, &taddr)
                .expect("re-propose sweep")
                .is_none(),
            "after the sweep spends the UTXO, a fresh sweep finds nothing — idempotent re-run",
        );
    }

    #[test]
    fn sweep_below_the_marginal_fee_is_an_uneconomic_none() {
        // The economic floor is the ENGINE's ZIP-317 fee, not zero: a UTXO too small to cover the
        // marginal fee cannot be swept for less than it costs ⇒ honest `Ok(None)` (the change strategy
        // reports `Change(InsufficientFunds)`), never a fee-eats-everything tx. True sub-marginal-fee
        // dust stays un-spendable wallet-wide (the engine excludes EACH ≤-marginal-fee UTXO BEFORE
        // aggregation); only UTXOs each ABOVE the marginal fee aggregate below the shield threshold (the
        // next test).
        let (mut st, taddr) = funded_transparent(500); // ≪ the ~10_000-zat minimal shield fee
        assert!(
            propose_sweep_on(&mut st, &taddr)
                .expect("propose sweep")
                .is_none(),
            "a sub-marginal-fee dust UTXO is uneconomic to sweep — Ok(None), never a wasteful tx",
        );
    }

    #[tokio::test]
    async fn sweep_aggregates_multiple_sub_threshold_utxos_below_the_shield_threshold() {
        // The economic-floor sweep recovers SEVERAL UTXOs on one ephemeral whose SUM is below the
        // convenience shield threshold (each ABOVE the marginal fee, so each is engine-selectable; the
        // SUM is what `propose_shield` rejects). The engine's GreedyInputSelector aggregates them into
        // one sweep tx; gross = their sum, net = sum − one fee.
        let values = [30_000u64, 30_000, 30_000]; // sum 90_000 ≪ the 100_000 shield threshold
        let (mut st, taddr) = funded_transparent_n(&values);
        let sum: u64 = values.iter().sum();

        assert!(
            propose_shield_on(&mut st, &taddr)
                .expect("propose shield")
                .is_none(),
            "the 90_000-zat aggregate is below the convenience shield threshold",
        );
        let proposal = propose_sweep_on(&mut st, &taddr)
            .expect("propose sweep")
            .expect("Some — the aggregate clears the fee");
        let dto = summarize_shield(&proposal, 1, SpendBinding::mint()).expect("summarize");
        assert_eq!(dto.total_zat, sum as i64, "gross = Σ the aggregated UTXOs");
        assert_eq!(
            dto.change_zat,
            dto.total_zat - dto.fee_zat,
            "net = aggregate − one fee"
        );
        assert_eq!(
            sign_shield_on(&mut st, &proposal).expect("sign").len(),
            1,
            "the aggregate sweeps in a SINGLE tx (one fee, not one per UTXO)",
        );
    }

    #[test]
    fn shielding_below_threshold_returns_none_on_the_harness() {
        // §8 (Recv-3 threshold gate): a transparent balance UNDER `SHIELDING_THRESHOLD_ZAT`
        // (even gross, before fees) yields `Ok(None)` — the engine's own `InsufficientFunds`
        // mapped to "nothing to shield yet", never an error.
        let (mut st, taddr) = funded_transparent(SHIELDING_THRESHOLD_ZAT as u64 - 1);
        let out = propose_shield_on(&mut st, &taddr).expect("propose");
        assert!(
            out.is_none(),
            "below the threshold ⇒ None (the host hides the shield CTA)"
        );
    }

    #[test]
    fn shielding_exactly_at_the_threshold_is_shieldable() {
        // §8 (the constant's BOUNDARY — testing-patterns "every named constant is tested at
        // its boundary"): the engine's gate is `balance.total() >= shielding_threshold`, and
        // `balance.total()` is the GROSS, so a UTXO of EXACTLY `SHIELDING_THRESHOLD_ZAT` clears
        // the INCLUSIVE `>=` edge ⇒ `Some` (not the just-below `None`). Pins the inclusive edge
        // so a future `>`-vs-`>=` drift fails here.
        let (mut st, taddr) = funded_transparent(SHIELDING_THRESHOLD_ZAT as u64);
        let dto = propose_shield_on(&mut st, &taddr)
            .expect("propose")
            .map(|p| summarize_shield(&p, 1, SpendBinding::mint()).expect("summarize"))
            .expect("Some at exactly the threshold (the gate is inclusive)");
        assert_eq!(
            dto.total_zat, SHIELDING_THRESHOLD_ZAT,
            "gross = the threshold"
        );
        assert!(dto.change_zat < dto.total_zat, "net is gross − a real fee");
    }

    #[test]
    fn shield_token_round_trips_through_the_registry_as_a_shield() {
        // §8 (the registry one-shot contract for the new Shield variant): a shield proposal
        // retains under its own token and consumes EXACTLY once as a `Shield` (so
        // `sign_proposal` dispatches it to create+sign, not the transfer arm) — a second
        // consume is `ProposalAlreadyUsed`, like a transfer token.
        let (mut st, taddr) = funded_transparent(200_000);
        let proposal = propose_shield_on(&mut st, &taddr)
            .expect("propose")
            .expect("Some");

        let registry = ProposalRegistry::new();
        let id = registry.insert_shield(proposal);
        let retained = registry.consume(id).expect("first consume succeeds");
        assert!(
            matches!(retained.proposal, RetainedProposal::Shield(_)),
            "the retained token is a Shield variant"
        );
        assert!(
            retained.request.is_none(),
            "a shield has no re-proposable PaymentRequest"
        );
        assert!(
            matches!(registry.consume(id), Err(WalletError::ProposalAlreadyUsed)),
            "the one-shot token is burned after the first consume"
        );
    }

    #[test]
    fn shield_sums_multiple_transparent_utxos_into_one_gross() {
        // §8 (the real-world money path — exchange withdrawals + swap deliveries accrue
        // SEVERAL UTXOs at the same receive t-address): a shield over N distinct UTXOs has
        // gross = Σ ALL of them (never just one), and net = the summed gross − the single
        // fee. Exercises the `summarize_shield` input-sum loop the single-UTXO tests never
        // multi-iterate, AND proves `propose_shielding` aggregates every UTXO at the address.
        let (mut st, taddr) = funded_transparent_n(&[120_000, 80_000, 50_000]);
        let proposal = propose_shield_on(&mut st, &taddr)
            .expect("propose")
            .expect("Some over threshold");
        let dto = summarize_shield(&proposal, 1, SpendBinding::mint()).expect("summarize");
        assert_eq!(
            dto.total_zat, 250_000,
            "gross = the SUM of all three transparent UTXOs, never one of them"
        );
        assert!(dto.fee_zat > 0, "a real ZIP-317 fee");
        assert_eq!(
            dto.change_zat,
            dto.total_zat - dto.fee_zat,
            "net shielded = the summed gross − the single fee"
        );
    }

    #[test]
    fn shield_and_transfer_tokens_coexist_and_dispatch_independently() {
        // §8 (the registry under MIXED kinds): a live Transfer token and a live Shield token
        // in the SAME registry each consume to the CORRECT variant, so `sign_proposal`
        // dispatches each to its own create+sign arm — a mis-dispatch would sign the wrong
        // proposal. The two ids are distinct (one shared counter) and neither shadows the other.
        let transfer = {
            let mut st = funded(NOTE_VALUE);
            propose_on(
                &mut st,
                request(vec![(sapling_recipient(b"coexist"), 10_000)]),
            )
            .expect("transfer proposal")
        };
        let shield = {
            let (mut st, taddr) = funded_transparent(200_000);
            propose_shield_on(&mut st, &taddr)
                .expect("propose")
                .expect("shield proposal")
        };

        let registry = ProposalRegistry::new();
        let transfer_id = registry.insert(transfer, PaymentRequest { payments: vec![] });
        let shield_id = registry.insert_shield(shield);
        assert_ne!(
            transfer_id, shield_id,
            "distinct ids from the one shared counter"
        );

        // Each consumes to its OWN variant — never the other's.
        let a = registry.consume(transfer_id).expect("consume transfer");
        assert!(
            matches!(a.proposal, RetainedProposal::Transfer(_)),
            "the transfer id yields a Transfer"
        );
        assert!(
            a.request.is_some(),
            "a transfer retains its re-proposable PaymentRequest"
        );
        let b = registry.consume(shield_id).expect("consume shield");
        assert!(
            matches!(b.proposal, RetainedProposal::Shield(_)),
            "the shield id yields a Shield"
        );
        assert!(b.request.is_none(), "a shield retains no PaymentRequest");
    }

    /// A valid Regtest sapling recipient address derived from a throwaway key.
    #[test]
    fn classify_magnitude_trips_on_relative_or_absolute_whichever_first() {
        use LargeSendReason::*;
        // RELATIVE: NEAR_TOTAL_SPENDABLE_BPS = 9000 (90%). spendable = 1_000_000 zat.
        let sp = 1_000_000;
        assert_eq!(
            classify_magnitude(899_999, sp),
            None,
            "just under 90% ⇒ ordinary"
        );
        assert_eq!(
            classify_magnitude(900_000, sp),
            Some(NearTotalBalance),
            "exactly 90% of spendable ⇒ near-total (the boundary)",
        );
        assert_eq!(classify_magnitude(950_000, sp), Some(NearTotalBalance));

        // ABSOLUTE: LARGE_SEND_ABSOLUTE_ZAT = 100_000_000 (1 ZEC). Big spendable so relative is off.
        let big = 10_000_000_000; // 100 ZEC
        assert_eq!(
            classify_magnitude(99_999_999, big),
            None,
            "under 1 ZEC + a tiny % ⇒ ordinary"
        );
        assert_eq!(
            classify_magnitude(100_000_000, big),
            Some(OverAbsoluteThreshold),
            "exactly 1 ZEC (only 1% of balance) ⇒ the absolute trips, not the relative",
        );

        // BOTH: ≥ 90% AND ≥ 1 ZEC (sending the whole 1-ZEC balance).
        assert_eq!(classify_magnitude(100_000_000, 100_000_000), Some(Both));

        // FULL DRAIN of a SUB-1-ZEC wallet — the relative arm fires STANDALONE at 100% (the
        // canonical "sweep my whole small wallet": a send of `available − fee` yields
        // `total == available`). Must be NearTotalBalance — NOT Both (it's under the absolute),
        // NOT None. (`InsufficientFunds` only rejects total > available, so total == available is
        // a real, reachable, desirable trigger — this is exactly what 9000bps < 10000 buys.)
        assert_eq!(classify_magnitude(900_000, 900_000), Some(NearTotalBalance));
        assert_eq!(classify_magnitude(500_000, 500_000), Some(NearTotalBalance));
        // The NearTotal↔Both transition across the 1-ZEC line: a just-under-1-ZEC full drain is
        // relative-only; once total crosses 1 ZEC the absolute joins (the host copy must change).
        assert_eq!(
            classify_magnitude(99_999_999, 99_999_999),
            Some(NearTotalBalance)
        );
        assert_eq!(classify_magnitude(100_000_000, 100_000_001), Some(Both));

        // NEITHER: a small send from a large balance.
        assert_eq!(classify_magnitude(5_000, big), None);

        // Degenerate zero-spendable: the relative guard is off; the absolute still decides.
        assert_eq!(classify_magnitude(50_000, 0), None);
        assert_eq!(
            classify_magnitude(100_000_000, 0),
            Some(OverAbsoluteThreshold)
        );

        // Overflow safety: a max-supply total × 10_000 must use the i128 path, never panic/wrap.
        assert_eq!(
            classify_magnitude(
                crate::constants::MAX_MONEY_ZAT,
                crate::constants::MAX_MONEY_ZAT
            ),
            Some(Both),
        );
    }

    #[test]
    fn is_self_send_exact_matches_the_wallets_own_address() {
        use crate::memo::{Address, Memo, Payment};
        use crate::money::Network;
        let to = |encoded: &str| -> PaymentRequest {
            let addr = Address::parse(encoded, Network::Main).expect("valid main address");
            PaymentRequest {
                payments: vec![
                    Payment::new(
                        addr,
                        Some(Zatoshis::new(1_000).expect("amt")),
                        Memo::Empty,
                        None,
                        None,
                    )
                    .expect("valid payment"),
                ],
            }
        };
        let own = ZcashAddress::from_sapling(NetworkType::Main, [0x11; 43]).encode();
        let other = ZcashAddress::from_sapling(NetworkType::Main, [0x22; 43]).encode();
        // A two-leg split payment where ONE leg matches own (the real `.any()` boundary).
        let mixed = |a: &str, b: &str| -> PaymentRequest {
            let mut req = to(a);
            req.payments.extend(to(b).payments);
            req
        };

        assert!(
            is_self_send(&to(&own), &own),
            "recipient == own ⇒ self-send"
        );
        assert!(
            !is_self_send(&to(&other), &own),
            "a different recipient ⇒ not a self-send"
        );
        assert!(
            is_self_send(&mixed(&other, &own), &own),
            "a multi-leg request where ANY leg is own ⇒ self-send (the .any() boundary)",
        );
        assert!(
            !is_self_send(&mixed(&other, &other), &own),
            "a multi-leg request with no own leg ⇒ not a self-send",
        );
        assert!(
            !is_self_send(&PaymentRequest { payments: vec![] }, &own),
            "an empty request is vacuously not a self-send",
        );

        // SCOPED LIMITATION (documented + CI-pinned): the wallet's own address in a DIFFERENT
        // encoding — e.g. the bare sapling receiver of its own unified address — is NOT detected
        // (exact encoded-string match only). This pins the known false-negative as DELIBERATE
        // behavior: a future upgrade to a real ownership test must consciously flip this assertion
        // (and the host's self-send-note expectation), not discover it as a surprise.
        use zcash_address::unified::{Address as Ua, Encoding, Receiver};
        let sap = [0x33u8; 43];
        let own_ua = Ua::try_from_items(vec![Receiver::Sapling(sap)])
            .expect("a sapling-only UA")
            .encode(&NetworkType::Main);
        let bare_sapling = ZcashAddress::from_sapling(NetworkType::Main, sap).encode();
        assert!(
            !is_self_send(&to(&bare_sapling), &own_ua),
            "the bare-sapling form of own UA is NOT detected — the documented exact-match limitation",
        );
    }

    fn sapling_recipient(seed: &[u8]) -> ZcashAddress {
        let extsk = sapling_crypto::zip32::ExtendedSpendingKey::master(seed);
        let (_, addr) = extsk.default_address();
        ZcashAddress::from_sapling(NetworkType::Regtest, addr.to_bytes())
    }

    /// A valid Regtest transparent p2pkh recipient (any 20-byte hash is structurally
    /// valid — transparent addresses carry no curve constraint).
    fn transparent_recipient(hash: [u8; 20]) -> ZcashAddress {
        ZcashAddress::from_transparent_p2pkh(NetworkType::Regtest, hash)
    }

    /// A valid Regtest TEX (ZIP-320) recipient — a Bech32m re-encoding of a 20-byte
    /// transparent pubkey-hash (HRP `texregtest`). Paying SHIELDED funds to this forces the
    /// engine's two-step ephemeral proposal (the `Address::Tex(_)` upstream test uses the same
    /// `[0x4; 20]`-style fixed hash; any 20-byte value is structurally valid).
    fn tex_recipient(hash: [u8; 20]) -> ZcashAddress {
        ZcashAddress::from_tex(NetworkType::Regtest, hash)
    }

    /// v-5c DEVICE-PROOF TOOL, not a test (run: `ZEC_WALLET_TADDR=tm… cargo test -p
    /// zec-wallet-core --lib tool_print_tex_encoding_for_taddr -- --ignored --nocapture`).
    /// Prints the ZIP-320 TEX encoding of a transparent P2PKH address so the live two-step
    /// proof can pay a `textest1…` recipient the SAME wallet controls — tx1 then pays back
    /// into the wallet and the round-trip loses only fees. The address's own network picks
    /// the HRP (tm… → textest1…, t1… → tex1…). §5.4 does not apply: an operator-invoked dev
    /// tool echoing the operator's own input, never a production code path.
    #[test]
    #[ignore = "v-5c device-proof tool — needs ZEC_WALLET_TADDR, prints only"]
    fn tool_print_tex_encoding_for_taddr() {
        struct P2pkhProbe(NetworkType, [u8; 20]);
        impl zcash_address::TryFromAddress for P2pkhProbe {
            type Error = Infallible;
            fn try_from_transparent_p2pkh(
                net: NetworkType,
                data: [u8; 20],
            ) -> Result<Self, zcash_address::ConversionError<Self::Error>> {
                Ok(P2pkhProbe(net, data))
            }
        }
        let addr = std::env::var("ZEC_WALLET_TADDR")
            .expect("set ZEC_WALLET_TADDR to the wallet's own tm…/t1… receive address");
        let parsed = ZcashAddress::try_from_encoded(addr.trim()).expect("a valid Zcash address");
        let P2pkhProbe(net, hash) = parsed
            .convert::<P2pkhProbe>()
            .expect("a transparent P2PKH (tm…/t1…) address — not a UA/shielded/TEX");
        println!(
            "TEX (ZIP-320) encoding of {}:\n{}",
            addr.trim(),
            ZcashAddress::from_tex(net, hash).encode(),
        );
    }

    fn request(legs: Vec<(ZcashAddress, u64)>) -> zip321::TransactionRequest {
        let payments = legs
            .into_iter()
            .map(|(addr, amount)| {
                zip321::Payment::without_memo(addr, ProtoZat::const_from_u64(amount))
            })
            .collect();
        zip321::TransactionRequest::new(payments).expect("valid request")
    }

    fn propose_on(
        st: &mut HarnessState,
        req: zip321::TransactionRequest,
    ) -> Result<TransferProposal, WalletError> {
        let network = *st.network();
        let aid = st.test_account().expect("account").id();
        propose_core(st.wallet_mut(), &network, aid, req)
    }

    // ── TEX / ZIP-320 two-step (§3.2i-2 slice 2e-2a) ─────────────────────────────
    //
    // These drive the engine's REAL two-step ephemeral proposal over the funded harness
    // (`funded()` shielded notes + a TEX recipient) and assert the SSOT shape predicate +
    // the no-double-count display math against actual engine output.
    //
    // NEAR-MISS COVERAGE: these pin the exact ACCEPTED shape against real engine output, plus
    // the real-world rejections (single-step send, plain-transparent send, shield). The
    // predicate's stricter rejection branches — a step1 consuming a step0 *payment*, a step1
    // with a shielded input, >1 ephemeral change, a 3-step — guard against a FUTURE engine
    // emitting a different multi-step. The predicate is now LOAD-BEARING on BOTH money paths
    // post-gate-removal (2e-2b-v-5): `summarize` fail-closes a non-ZIP-320 multi-step on the
    // interactive path, and the matching guards in `prepare_queued` + the defensive guard in
    // `create_signed_core` fail-close it on the queued path — so only the recognised two-step ever
    // reaches signing. A mis-recognised shape is the failure these branches foreclose.

    /// The engine builds a real ZIP-320 two-step for a TEX recipient, and the predicate
    /// recognises it with the correct ephemeral-change index.
    #[test]
    fn tex_send_builds_a_real_zip320_two_step_the_predicate_recognises() {
        let mut st = funded(NOTE_VALUE);
        let to = tex_recipient([0x04; 20]);
        let proposal = propose_on(&mut st, request(vec![(to, 20_000)])).expect("TEX proposes");

        assert_eq!(
            proposal.steps().len(),
            2,
            "TEX-from-shielded is a two-step proposal",
        );
        let shape = is_zip320_two_step(&proposal).expect("the engine output IS the ZIP-320 shape");

        let step0 = proposal.steps().first();
        let step1 = proposal.steps().last();

        // The located index really points at the lone ephemeral change in step0.
        let changes = step0.balance().proposed_change();
        assert!(
            changes[shape.ephemeral_change_index].is_ephemeral(),
            "the located index is the ephemeral output",
        );
        assert_eq!(
            changes.iter().filter(|cv| cv.is_ephemeral()).count(),
            1,
            "exactly one ephemeral change in step0",
        );

        // Structural cross-check of what the predicate asserted.
        assert!(
            step0.shielded_inputs().is_some(),
            "step0 spends shielded notes"
        );
        assert!(
            step0.transparent_inputs().is_empty(),
            "step0 has no transparent inputs"
        );
        assert!(
            step1.shielded_inputs().is_none(),
            "step1 has no shielded inputs"
        );
        assert!(
            step1.transparent_inputs().is_empty(),
            "step1's ephemeral is a prior_step_input"
        );
        assert_eq!(
            step1.prior_step_inputs().len(),
            1,
            "step1 has one prior input"
        );
        assert_eq!(
            step1.prior_step_inputs()[0],
            StepOutput::new(0, StepOutputIndex::Change(shape.ephemeral_change_index)),
            "step1 consumes step0's ephemeral CHANGE (not a payment)",
        );
        let (idx, _) = step1
            .transaction_request()
            .payments()
            .iter()
            .next()
            .expect("a TEX payment");
        assert_eq!(
            step1.payment_pools().get(idx),
            Some(&PoolType::Transparent),
            "the TEX leg is paid to the transparent pool",
        );
    }

    /// A normal single-step shielded send is NOT the ZIP-320 shape.
    #[test]
    fn is_zip320_two_step_rejects_a_normal_single_step_send() {
        let mut st = funded(NOTE_VALUE);
        let to = sapling_recipient(&[0x11; 32]);
        let proposal = propose_on(&mut st, request(vec![(to, 10_000)])).expect("proposes");
        assert!(
            is_zip320_two_step(&proposal).is_none(),
            "a single-step send is not ZIP-320",
        );
    }

    /// A plain transparent (non-TEX) recipient is a SINGLE-step shielded→transparent unshield
    /// (no ephemeral hop) — also not the ZIP-320 shape. This pins that the predicate keys on
    /// the two-step ephemeral STRUCTURE, not merely on "has a transparent recipient".
    #[test]
    fn is_zip320_two_step_rejects_a_plain_transparent_send() {
        let mut st = funded(NOTE_VALUE);
        let to = transparent_recipient([0x22; 20]);
        let proposal = propose_on(&mut st, request(vec![(to, 10_000)])).expect("proposes");
        assert_eq!(
            proposal.steps().len(),
            1,
            "a plain t-addr send is single-step"
        );
        assert!(is_zip320_two_step(&proposal).is_none());
    }

    /// A shield (transparent→shielded, the OPPOSITE crossing) is single-step and carries
    /// transparent INPUTS, not the TEX ephemeral structure — also not the ZIP-320 shape.
    #[test]
    fn is_zip320_two_step_rejects_a_shield() {
        let (mut st, taddr) = funded_transparent(200_000);
        let proposal = propose_shield_on(&mut st, &taddr)
            .expect("shield proposes")
            .expect("a balance over the threshold");
        assert_eq!(proposal.steps().len(), 1, "a shield is single-step");
        assert!(is_zip320_two_step(&proposal).is_none());
    }

    /// A pure-TEX send: the total is the recipient amount + BOTH step fees, with no
    /// double-count (the ephemeral hop is change, never a payment), and the note reconciles.
    #[test]
    fn summarize_tex_pure_total_is_recipient_plus_both_fees() {
        let mut st = funded(NOTE_VALUE);
        let amount = 20_000u64;
        let to = tex_recipient([0x04; 20]);
        let proposal = propose_on(&mut st, request(vec![(to, amount)])).expect("TEX proposes");
        let dto = summarize(&proposal, 0, SpendBinding::mint()).expect("summarizes the two-step");

        assert_eq!(dto.steps.len(), 2, "the DTO carries both steps");
        assert!(
            dto.is_two_step_tex,
            "the SSOT two-step signal is set (drives the host's partial-terminal outcome copy)",
        );
        assert!(
            dto.has_transparent_recipient,
            "TEX is a transparent recipient (the §5.1 de-shield applies)",
        );
        assert!(dto.fee_zat > 0, "real ZIP-317 fees across both steps");
        assert_eq!(
            dto.total_zat,
            amount as i64 + dto.fee_zat,
            "total = the one recipient + both fees, no double-count",
        );
        assert_eq!(
            dto.change_zat,
            NOTE_VALUE as i64 - dto.total_zat,
            "the input note reconciles: input = total debited + change returned",
        );
    }

    /// FR-46 (S2 §3.5f): a send to ONE recipient reports the total paid to it, as signed, the
    /// fee and the change excluded — and two payments to the same address are that one
    /// recipient, summed (zip321 0.9.0 accepts a duplicate address; measured here, the engine
    /// proposes it).
    #[test]
    fn a_proposal_to_one_recipient_reports_its_total_excluding_the_fee() {
        let mut st = funded(NOTE_VALUE);
        let to = sapling_recipient(&[0x31; 32]);
        let proposal = propose_on(&mut st, request(vec![(to.clone(), 12_000)])).expect("proposes");
        let dto = summarize(&proposal, 0, SpendBinding::mint()).expect("summarizes");
        assert!(
            dto.fee_zat > 0 && dto.change_zat > 0,
            "a real fee and change"
        );
        assert_eq!(
            dto.single_recipient_zat,
            Some(12_000),
            "the recipient's amount alone — no fee, no change",
        );

        let proposal = propose_on(&mut st, request(vec![(to.clone(), 10_000), (to, 5_000)]))
            .expect("two payments to one address propose");
        let dto = summarize(&proposal, 0, SpendBinding::mint()).expect("summarizes");
        assert_eq!(
            dto.single_recipient_zat,
            Some(15_000),
            "two payments to the same address are one recipient, summed",
        );
    }

    /// FR-46: two recipients ⇒ `None`, never a partial sum (the host refuses rather than shows
    /// a figure that is not what that recipient was paid).
    #[test]
    fn a_proposal_paying_two_recipients_reports_no_single_recipient_amount() {
        let mut st = funded(NOTE_VALUE);
        let proposal = propose_on(
            &mut st,
            request(vec![
                (sapling_recipient(&[0x32; 32]), 10_000),
                (sapling_recipient(&[0x33; 32]), 5_000),
            ]),
        )
        .expect("two recipients propose");
        let dto = summarize(&proposal, 0, SpendBinding::mint()).expect("summarizes");
        assert_eq!(dto.single_recipient_zat, None);
    }

    /// FR-46: the ZIP-320 two-step reports the forwarded amount ONCE — the ephemeral hop is
    /// change, never a payment, so it is not counted a second time.
    #[test]
    fn a_tex_two_step_reports_the_forwarded_amount_once() {
        let mut st = funded(NOTE_VALUE);
        let proposal = propose_on(&mut st, request(vec![(tex_recipient([0x04; 20]), 20_000)]))
            .expect("TEX proposes");
        let dto = summarize(&proposal, 0, SpendBinding::mint()).expect("summarizes the two-step");
        assert!(dto.is_two_step_tex);
        assert_eq!(dto.single_recipient_zat, Some(20_000));
    }

    /// FR-46: a shield has no external recipient, so no recipient amount.
    #[test]
    fn a_shield_proposal_reports_no_recipient_amount() {
        let (mut st, taddr) = funded_transparent(200_000);
        let proposal = propose_shield_on(&mut st, &taddr)
            .expect("propose")
            .expect("Some over threshold");
        let dto = summarize_shield(&proposal, 1, SpendBinding::mint()).expect("summarize");
        assert!(dto.is_shield);
        assert_eq!(dto.single_recipient_zat, None);
    }

    /// MANDATORY mixed send (a pure-TEX vector would pass a buggy `steps().last()`-only total
    /// and HIDE the bug): a request paying BOTH a TEX recipient and a shielded recipient. The
    /// engine keeps the shielded payment in step0 and the TEX in step1, so the total MUST span
    /// ALL steps — a last-step-only sum would drop the step0 shielded leg and under-debit.
    #[test]
    fn tex_mixed_total_includes_step0_shielded_recipient() {
        let mut st = funded(NOTE_VALUE);
        let tex = tex_recipient([0x04; 20]);
        let shielded = sapling_recipient(&[0x55; 32]);
        let tex_amount = 10_000u64;
        let shielded_amount = 10_000u64;
        let proposal = propose_on(
            &mut st,
            request(vec![(tex, tex_amount), (shielded, shielded_amount)]),
        )
        .expect("mixed TEX + shielded proposes");

        assert!(
            is_zip320_two_step(&proposal).is_some(),
            "a mixed send is still the ZIP-320 shape (step1's all-transparent check is step1-only)",
        );
        let dto = summarize(&proposal, 0, SpendBinding::mint()).expect("summarizes");

        assert_eq!(
            dto.total_zat,
            (tex_amount + shielded_amount) as i64 + dto.fee_zat,
            "total spans BOTH recipients + fees (a last()-only total would drop the shielded leg)",
        );
        let all_recipients: usize = dto.steps.iter().map(|s| s.recipients.len()).sum();
        assert_eq!(
            all_recipients, 2,
            "both recipients are represented across the steps"
        );
        assert!(
            dto.has_transparent_recipient,
            "the TEX leg is a transparent recipient"
        );
        // FR-46 across the two steps: step0 pays the shielded address, step1 the
        // TEX — two addresses, so no single-recipient figure, never step1's alone.
        assert_eq!(
            dto.single_recipient_zat, None,
            "a mixed two-step pays two addresses: no single-recipient amount"
        );
    }

    /// ENGINE ORDERING QUIRK (documented, money-safe — pinned by the operational round). A mixed
    /// send only proposes when the TEX leg(s) occupy the LOWEST payment indices: the engine keys
    /// step1's `payment_pools` by ORIGINAL request index, but `TransactionRequest::new` re-enumerates
    /// the step1 payments from 0, so a shielded-FIRST mixed request trips `PaymentPoolsMismatch`
    /// inside `Step::from_parts` → fail-closed `ProposeFailed`. No money loss / no leak (fails
    /// closed), but a legitimately-ordered exchange URI can refuse to propose depending on recipient
    /// order. Pinned so it is a KNOWN constraint, not a surprise — slice B owes payment normalisation
    /// (move TEX legs to the front, or a typed "reorder recipients" error) before the multi-step path
    /// actually signs. The TEX-FIRST order proposes fine (see the mixed test above).
    #[test]
    fn tex_mixed_shielded_first_is_propose_failed_engine_ordering_quirk() {
        let mut st = funded(NOTE_VALUE);
        let shielded = sapling_recipient(&[0x55; 32]);
        let tex = tex_recipient([0x04; 20]);
        // shielded at index 0, TEX at index 1 — the engine rejects this ordering.
        let err = propose_on(&mut st, request(vec![(shielded, 10_000), (tex, 10_000)]))
            .expect_err("shielded-first mixed is rejected by the engine");
        assert!(
            matches!(err, WalletError::ProposeFailed),
            "shielded-first mixed → fail-closed ProposeFailed, got {err:?}",
        );
    }

    /// Round-2 #9 — the FIX for the quirk above: `normalize_tex_first` re-orders the TEX leg to the
    /// front, so a legitimately-ordered exchange URI proposes regardless of the order the user/host
    /// supplied it in. `prepare_queued` applies this before `propose_core` (the queued drain path).
    #[test]
    fn tex_mixed_any_order_proposes_after_normalisation() {
        let mut st = funded(NOTE_VALUE);
        let shielded = sapling_recipient(&[0x55; 32]);
        let tex = tex_recipient([0x04; 20]);
        // shielded-FIRST — the exact order the raw engine rejected above.
        let raw = request(vec![(shielded, 10_000), (tex, 10_000)]);
        let proposal =
            propose_on(&mut st, normalize_tex_first(raw)).expect("normalised mixed proposes");
        assert_eq!(
            proposal.steps().len(),
            2,
            "the normalised mixed send is a two-step proposal"
        );
        assert!(
            is_zip320_two_step(&proposal).is_some(),
            "the normalised proposal is the recognised ZIP-320 two-step",
        );
    }

    /// 2e-2b-v-1 (the interactive `Wallet::propose` path now normalises too): a mixed shielded-FIRST
    /// send — the exact order the RAW engine rejects (`tex_mixed_shielded_first…`) — proposes with
    /// FULL money fidelity once normalised, IDENTICALLY to the TEX-first order the engine accepts raw
    /// (`tex_mixed_total_includes_step0_shielded_recipient`). This is the gate-removal guarantee for
    /// the interactive entry: order-independence, no leg dropped, no double-count. `Wallet::propose`
    /// applies the SAME `normalize_tex_first` as the queued drain, so a host that supplies the
    /// shielded leg first never silently fails to send now that the multi-step gate is off (2e-2b-v-5).
    #[test]
    fn normalised_shielded_first_mix_proposes_with_full_money_fidelity() {
        let mut st = funded(NOTE_VALUE);
        let shielded = sapling_recipient(&[0x55; 32]);
        let tex = tex_recipient([0x04; 20]);
        let shielded_amount = 10_000u64;
        let tex_amount = 7_000u64;
        // shielded-FIRST — rejected raw, accepted after the normalisation both send paths apply.
        let raw = request(vec![(shielded, shielded_amount), (tex, tex_amount)]);
        let proposal = propose_on(&mut st, normalize_tex_first(raw))
            .expect("normalised shielded-first mix proposes");
        assert!(
            is_zip320_two_step(&proposal).is_some(),
            "the normalised shielded-first mix is the recognised ZIP-320 two-step",
        );
        let dto = summarize(&proposal, 0, SpendBinding::mint()).expect("summarizes");
        assert_eq!(
            dto.total_zat,
            (shielded_amount + tex_amount) as i64 + dto.fee_zat,
            "total spans BOTH legs + fees regardless of input order — no leg dropped, no double-count",
        );
        let all_recipients: usize = dto.steps.iter().map(|s| s.recipients.len()).sum();
        assert_eq!(all_recipients, 2, "both recipients survive the reorder");
        // Input-side reconciliation (the money red-team belt-and-braces): the funded note fully
        // accounts as total-debited + change-returned, order-independent — the reorder moves no value
        // in OR out, it only permutes the output legs.
        assert_eq!(
            dto.change_zat,
            NOTE_VALUE as i64 - dto.total_zat,
            "the input note reconciles after the reorder: input = total debited + change returned",
        );
        assert_eq!(
            dto.single_recipient_zat, None,
            "the reordered mix still pays two addresses: no single-recipient amount"
        );
    }

    /// `normalize_tex_first` is IDENTITY for every request that is NOT a genuine TEX⊕non-TEX mix:
    /// a single leg, a pure-shielded set, and a pure-TEX set all keep their EXACT payment set + order
    /// (the URI is byte-identical), so a normal send is never perturbed.
    #[test]
    fn normalize_tex_first_is_identity_for_non_mixed_requests() {
        let shielded_a = sapling_recipient(&[0x11; 32]);
        let shielded_b = sapling_recipient(&[0x22; 32]);
        let tex_a = tex_recipient([0x01; 20]);
        let tex_b = tex_recipient([0x02; 20]);
        for legs in [
            vec![(shielded_a.clone(), 1_000)],
            vec![(shielded_a.clone(), 1_000), (shielded_b.clone(), 2_000)],
            vec![(tex_a.clone(), 1_000), (tex_b.clone(), 2_000)],
        ] {
            let req = request(legs);
            let before = req.to_uri();
            let after = normalize_tex_first(req).to_uri();
            assert_eq!(before, after, "a non-mixed request is returned unchanged");
        }
    }

    /// The re-order itself, pinned at the REQUEST level (engine-independent): the TEX leg moves to
    /// index 0, the shielded leg follows, and each leg's FULL tuple is preserved EXACTLY (never
    /// alter the money — the legs are cloned whole, not reconstructed). The shielded leg carries a
    /// MEMO, asserted to survive the move (the clone-whole proves it, not field reconstruction).
    #[test]
    fn normalize_tex_first_moves_the_tex_leg_to_the_front_of_a_mix() {
        let shielded = sapling_recipient(&[0x33; 32]);
        let tex = tex_recipient([0x04; 20]);
        let memo = zcash_protocol::memo::MemoBytes::from_bytes(b"exchange ref 42").expect("memo");
        // shielded-FIRST, the shielded leg memo'd (a TEX leg is memo-incapable — transparent).
        let shielded_leg = zip321::Payment::new(
            shielded,
            Some(ProtoZat::const_from_u64(10_000)),
            Some(memo.clone()),
            None,
            None,
            vec![],
        )
        .expect("valid memo'd shielded payment");
        let tex_leg = zip321::Payment::without_memo(tex, ProtoZat::const_from_u64(7_000));
        let req = zip321::TransactionRequest::new(vec![shielded_leg, tex_leg]).expect("request");

        let normalized = normalize_tex_first(req);
        let payments: Vec<_> = normalized.payments().values().collect();
        assert_eq!(payments.len(), 2);
        assert!(
            is_tex_recipient(payments[0].recipient_address()),
            "the TEX leg is first"
        );
        assert!(
            !is_tex_recipient(payments[1].recipient_address()),
            "the shielded leg follows"
        );
        assert_eq!(
            u64::from(payments[0].amount().expect("tex amount")),
            7_000,
            "the TEX amount is preserved",
        );
        assert!(payments[0].memo().is_none(), "the TEX leg stays memo-less");
        assert_eq!(
            u64::from(payments[1].amount().expect("shielded amount")),
            10_000,
            "the shielded amount is preserved",
        );
        assert_eq!(
            payments[1].memo(),
            Some(&memo),
            "the shielded leg's MEMO survives the reorder (cloned whole, not reconstructed)",
        );
    }

    /// A k>1 TEX-leg mix: `[shielded, tex, shielded, tex]` normalises to `[tex, tex, shielded,
    /// shielded]` — ALL TEX legs ahead of ALL non-TEX legs, the relative order WITHIN each group
    /// preserved (the partition is stable). Request-level (engine-independent), so it pins the
    /// reorder for multiple TEX legs without needing a 4-output funded proposal.
    #[test]
    fn normalize_tex_first_orders_all_tex_legs_ahead_of_all_non_tex_legs() {
        let s0 = sapling_recipient(&[0xA0; 32]);
        let t0 = tex_recipient([0xB0; 20]);
        let s1 = sapling_recipient(&[0xA1; 32]);
        let t1 = tex_recipient([0xB1; 20]);
        // Interleaved, shielded-first: shielded(1), tex(2), shielded(3), tex(4).
        let normalized = normalize_tex_first(request(vec![
            (s0, 1_000),
            (t0, 2_000),
            (s1, 3_000),
            (t1, 4_000),
        ]));
        let payments: Vec<_> = normalized.payments().values().collect();
        assert_eq!(payments.len(), 4, "no leg is dropped or duplicated");
        // Both TEX legs first, in their original relative order (2_000 then 4_000)…
        assert!(is_tex_recipient(payments[0].recipient_address()));
        assert!(is_tex_recipient(payments[1].recipient_address()));
        assert_eq!(u64::from(payments[0].amount().expect("amt")), 2_000);
        assert_eq!(u64::from(payments[1].amount().expect("amt")), 4_000);
        // …then both shielded legs, in their original relative order (1_000 then 3_000).
        assert!(!is_tex_recipient(payments[2].recipient_address()));
        assert!(!is_tex_recipient(payments[3].recipient_address()));
        assert_eq!(u64::from(payments[2].amount().expect("amt")), 1_000);
        assert_eq!(u64::from(payments[3].amount().expect("amt")), 3_000);
    }

    /// `change_zat` excludes the ephemeral hop: it reports ONLY the shielded change returned to
    /// the user, never the in-transit ephemeral output (≈ recipient + tx1 fee). Without the
    /// `is_ephemeral()` skip, change_zat would be inflated by the hop and the note would not
    /// reconcile.
    #[test]
    fn tex_change_zat_excludes_the_ephemeral_hop() {
        let mut st = funded(NOTE_VALUE);
        let amount = 15_000u64;
        let to = tex_recipient([0x04; 20]);
        let proposal = propose_on(&mut st, request(vec![(to, amount)])).expect("TEX proposes");

        // The ephemeral output's value — what an un-excluded change_zat would wrongly add.
        let ephemeral_value: i64 = proposal
            .steps()
            .first()
            .balance()
            .proposed_change()
            .iter()
            .filter(|cv| cv.is_ephemeral())
            .map(|cv| u64::from(cv.value()) as i64)
            .sum();
        assert!(ephemeral_value > 0, "there is an ephemeral hop to exclude");

        // What `change_zat` WOULD be without the `is_ephemeral()` skip: Σ ALL proposed_change
        // across both steps, ephemeral hop included.
        let change_with_hop: i64 = proposal
            .steps()
            .iter()
            .flat_map(|s| s.balance().proposed_change())
            .map(|cv| u64::from(cv.value()) as i64)
            .sum();

        let dto = summarize(&proposal, 0, SpendBinding::mint()).expect("summarizes");
        // The DTO's change is the un-excluded sum MINUS exactly the ephemeral hop — i.e. the skip
        // removed the hop and nothing else (this fails if the skip is dropped OR over-broad).
        assert_eq!(
            change_with_hop - dto.change_zat,
            ephemeral_value,
            "change_zat excludes exactly the ephemeral hop (no more, no less)",
        );
        // And the note reconciles WITHOUT the hop: input = total debited + change returned.
        assert_eq!(
            dto.change_zat,
            NOTE_VALUE as i64 - dto.total_zat,
            "input = total + change, with the in-transit hop kept out of change",
        );
    }

    /// GATE-REMOVAL PROOF (2e-2b-v-5): `create_signed_core` now SIGNS a ZIP-320 TEX two-step — the
    /// slice-A create gate is gone. The engine persists the full `[tx0, tx1]` chain (proof that a
    /// two-step is no longer fail-closed). Signing can no longer strand tx1 with no retry: the
    /// recovery preconditions (the re-broadcast machine, the gap-limit ceiling, the manual sweep,
    /// the parked surface, the cancel escape hatch) all landed first.
    #[test]
    fn create_signed_core_signs_the_tex_two_step_after_gate_removal() {
        let mut st = funded(NOTE_VALUE);
        let to = tex_recipient([0x04; 20]);
        let proposal = propose_on(&mut st, request(vec![(to, 20_000)])).expect("TEX proposes");
        assert!(
            summarize(&proposal, 0, SpendBinding::mint()).is_ok(),
            "the DTO previews fine"
        );
        assert!(
            is_zip320_two_step(&proposal).is_some(),
            "the engine built the two-step"
        );
        let txids = sign_on(&mut st, &proposal).expect("the two-step now signs (gate removed)");
        assert_eq!(
            txids.len(),
            2,
            "a TEX two-step persists the [tx0, tx1] chain"
        );
    }

    /// Drive the inc-2d-2 create+sign CORE over the funded harness: the account's own
    /// transient USK (cloned from the test account — the §4.2 confinement is the core's),
    /// the harness `LocalNetwork`, and the PRODUCTION bundled-prover provider
    /// (`crate::prover::tx_prover` — the SAME prover the live wallet's `sign_proposal`
    /// uses; DRY, parsed once via its `OnceLock`).
    fn sign_on(
        st: &mut HarnessState,
        proposal: &TransferProposal,
    ) -> Result<Vec<TxId>, WalletError> {
        let network = *st.network();
        // Clone the account's USK and END the immutable borrow before `wallet_mut()`.
        let usk = st.test_account().expect("account").usk().clone();
        let prover = crate::prover::tx_prover();
        create_signed_core(
            st.wallet_mut(),
            &network,
            usk,
            proposal,
            prover,
            prover,
            crate::consensus::SigningPermit::assume_current_for_test(),
        )
    }

    /// Sign a MULTI-STEP proposal (the TEX two-step) over the funded harness via the RAW audited
    /// engine call (`create_proposed_transactions`) — the same call `create_signed_core` wraps,
    /// minus its re-anchor + gap-limit error mapping. Post-gate-removal (2e-2b-v-5) `create_signed_core`
    /// itself signs a two-step too (see `create_signed_core_signs_the_tex_two_step_after_gate_removal`),
    /// so this leaner helper is kept only to drive the multi-txid recovery machine over genuine
    /// ZIP-320 output without re-anchoring between setup steps. Returns the ordered group `[tx0, tx1]`.
    fn sign_two_step_on(st: &mut HarnessState, proposal: &TransferProposal) -> Vec<TxId> {
        let network = *st.network();
        let usk = st.test_account().expect("account").usk().clone();
        let prover = crate::prover::tx_prover();
        create_proposed_transactions::<_, _, Infallible, StandardFeeRule, Infallible, _>(
            st.wallet_mut(),
            &network,
            prover,
            prover,
            &SpendingKeys::from_unified_spending_key(usk),
            OvkPolicy::Sender,
            proposal,
            // No expiry override — same as production `create_signed_core`.
            None,
        )
        .expect("two-step create+sign")
        .into_iter()
        .collect()
    }

    /// Like [`sign_two_step_on`] but maps the engine error through [`map_create_err`] and RETURNS
    /// it (no panic) — the gate-free analogue of what `create_signed_core` does at gate-removal (its
    /// `steps()>1` gate is the ONLY reason production cannot run this path yet). Lets a REAL
    /// create-time fault — notably the round-2 #7 ephemeral gap-limit ceiling (`ReachedGapLimit`) —
    /// surface as the TYPED `WalletError` exactly as production will once that gate is removed LAST.
    fn try_sign_two_step_on(
        st: &mut HarnessState,
        proposal: &TransferProposal,
    ) -> Result<Vec<TxId>, WalletError> {
        let network = *st.network();
        let usk = st.test_account().expect("account").usk().clone();
        let prover = crate::prover::tx_prover();
        create_proposed_transactions::<_, _, Infallible, StandardFeeRule, Infallible, _>(
            st.wallet_mut(),
            &network,
            prover,
            prover,
            &SpendingKeys::from_unified_spending_key(usk),
            OvkPolicy::Sender,
            proposal,
            // No expiry override — same as production `create_signed_core`.
            None,
        )
        .map(|group| group.into_iter().collect())
        .map_err(map_create_err)
    }

    /// Persist a real two-step (TEX) proposal as a `Sent` outbox row — the slice-B multi-txid
    /// path the recovery tests drive: enqueue → `mark_submitting` (step0's shielded claim) →
    /// `sign_two_step_on` (both txs persisted) → `mark_sent_multi([tx0, tx1])`. Returns the aux
    /// store, the intent id, and the ordered txids so tests can mine/expire selected txs.
    fn tex_two_step_sent_row(st: &mut HarnessState) -> (Connection, QueuedSendId, Vec<[u8; 32]>) {
        let to = tex_recipient([0x04; 20]);
        let proposal = propose_on(st, request(vec![(to, 20_000)])).expect("TEX proposes");
        assert_eq!(
            proposal.steps().len(),
            2,
            "a TEX send is a two-step proposal"
        );
        let claims = claim_from_proposal(&proposal).expect("step0's shielded claim");
        let txids = sign_two_step_on(st, &proposal);
        assert_eq!(txids.len(), 2, "the engine created tx0 + tx1");
        let group: Vec<[u8; 32]> = txids.iter().map(|t| *t.as_ref()).collect();

        let mut aux = Connection::open_in_memory().expect("aux conn");
        crate::intent_store::ensure_table(&aux).expect("table");
        let id = crate::intent_store::enqueue(&mut aux, "zcash:tex-intent", 0, None, None)
            .expect("enqueue");
        assert!(crate::intent_store::mark_submitting(&mut aux, id, &claims).expect("submit"));
        assert!(crate::intent_store::mark_sent_multi(&mut aux, id, &group).expect("sent group"));
        (aux, id, group)
    }

    /// The wallet's spendable total under the SSOT policy, read the SAME way the
    /// balance reader does — used to prove propose and balance agree.
    fn spendable_zat(st: &HarnessState) -> u64 {
        let summary = st
            .wallet()
            .get_wallet_summary(account::spendable_policy())
            .expect("summary read")
            .expect("a funded wallet has a summary");
        summary
            .account_balances()
            .values()
            .map(|b| u64::from(b.spendable_value()))
            .sum()
    }

    // ── The named tests ─────────────────────────────────────────────────────────

    #[test]
    fn propose_selects_notes_fee_and_change_for_a_simple_send() {
        let mut st = funded(NOTE_VALUE);
        let to = sapling_recipient(&[0x11; 32]);
        let amount = 10_000u64;
        let proposal = propose_on(&mut st, request(vec![(to, amount)])).expect("proposes");
        let dto = summarize(&proposal, 0, SpendBinding::mint()).expect("summarizes");

        // Total = amount + fee, fee is the real ZIP-317 fee (nonzero), and since the one
        // 60k note is spent whole, change reconciles: input = total + change.
        assert!(dto.fee_zat > 0, "a real ZIP-317 fee");
        assert_eq!(
            dto.total_zat,
            amount as i64 + dto.fee_zat,
            "total = amount + fee"
        );
        assert_eq!(
            dto.change_zat,
            NOTE_VALUE as i64 - dto.total_zat,
            "input note = total debited + change",
        );
        assert_eq!(dto.steps.len(), 1, "single-step send");
        assert!(
            !dto.is_two_step_tex,
            "a single-step send is not a TEX two-step"
        );
        assert_eq!(dto.steps[0].recipients.len(), 1);
        assert_eq!(dto.steps[0].recipients[0].amount_zat, amount as i64);
        assert_eq!(dto.steps[0].recipients[0].pool, OutputPool::Sapling);
        assert!(!dto.has_transparent_recipient, "no de-shield leg");
    }

    #[test]
    fn propose_insufficient_funds_is_typed_with_available_and_required() {
        let mut st = funded(NOTE_VALUE);
        let to = sapling_recipient(&[0x22; 32]);
        // Far beyond the single 60k note.
        let err = propose_on(&mut st, request(vec![(to, 10_000_000)])).unwrap_err();
        match err {
            WalletError::InsufficientFunds {
                available,
                required,
                ..
            } => {
                assert!(available.zat() <= NOTE_VALUE as i64, "available ≤ the note");
                assert!(required.zat() >= 10_000_000, "required covers the ask");
            }
            other => panic!("expected InsufficientFunds, got {other:?}"),
        }
    }

    #[test]
    fn propose_amount_at_spendable_boundary() {
        let mut st = funded(NOTE_VALUE);
        let to = sapling_recipient(&[0x33; 32]);

        // Just under the balance is fine (room for the fee).
        let ok = propose_on(&mut st, request(vec![(to.clone(), NOTE_VALUE - 20_000)]));
        assert!(ok.is_ok(), "an amount with fee headroom proposes");

        // The WHOLE balance is over the boundary: there is no room left for the fee, so
        // the required (amount + fee) exceeds the available — the fee tips it over.
        let err = propose_on(&mut st, request(vec![(to, NOTE_VALUE)])).unwrap_err();
        match err {
            WalletError::InsufficientFunds {
                available,
                required,
                ..
            } => assert!(
                required.zat() > available.zat(),
                "sending the whole balance can't cover the fee",
            ),
            other => panic!("expected InsufficientFunds at the boundary, got {other:?}"),
        }
    }

    #[test]
    fn propose_uses_the_same_spendable_policy_as_balance() {
        // One deep (spendable) note + one shallow (not-yet-spendable) note.
        let mut st = funded(NOTE_VALUE);
        add_shallow_note(&mut st, NOTE_VALUE);

        // Balance counts only the deep note under the SSOT policy.
        assert_eq!(
            spendable_zat(&st),
            NOTE_VALUE,
            "only the deep note is spendable"
        );

        // Propose for more than the deep note but less than both: the shallow note is
        // NOT selectable (propose obeys the SAME policy), so it's InsufficientFunds and
        // `available` equals the balance's spendable — the two can never disagree.
        let to = sapling_recipient(&[0x44; 32]);
        let err = propose_on(&mut st, request(vec![(to, NOTE_VALUE + 30_000)])).unwrap_err();
        match err {
            WalletError::InsufficientFunds { available, .. } => {
                assert_eq!(
                    available.zat(),
                    spendable_zat(&st) as i64,
                    "propose's available == balance's spendable (the SSOT)",
                );
            }
            other => panic!("expected InsufficientFunds, got {other:?}"),
        }
    }

    #[test]
    fn propose_is_read_only_no_db_mutation_no_network() {
        let mut st = funded(NOTE_VALUE);
        let before = spendable_zat(&st);

        let to = sapling_recipient(&[0x55; 32]);
        let first = summarize(
            &propose_on(&mut st, request(vec![(to.clone(), 12_345)])).expect("proposes"),
            0,
            SpendBinding::mint(),
        )
        .expect("summarizes");
        // Re-propose the same request: deterministic, no DB write between them.
        let second = summarize(
            &propose_on(&mut st, request(vec![(to, 12_345)])).expect("re-proposes"),
            0,
            SpendBinding::mint(),
        )
        .expect("summarizes");

        assert_eq!(
            before,
            spendable_zat(&st),
            "propose wrote nothing to the DB"
        );
        assert_eq!(
            first.total_zat, second.total_zat,
            "re-propose is idempotent"
        );
        assert_eq!(first.fee_zat, second.fee_zat);
        assert_eq!(first.change_zat, second.change_zat);
    }

    #[test]
    fn propose_multi_recipient_and_a_transparent_leg_sets_disclosure() {
        let mut st = funded(NOTE_VALUE);
        let shielded = sapling_recipient(&[0x66; 32]);
        let transparent = transparent_recipient([0xAB; 20]);
        let proposal = propose_on(
            &mut st,
            request(vec![(shielded, 10_000), (transparent, 5_000)]),
        )
        .expect("multi-recipient proposes");
        let dto = summarize(&proposal, 0, SpendBinding::mint()).expect("summarizes");

        assert!(
            dto.has_transparent_recipient,
            "a transparent leg sets the §5.1 de-shield disclosure",
        );
        let pools: Vec<OutputPool> = dto.steps[0].recipients.iter().map(|r| r.pool).collect();
        assert!(
            pools.contains(&OutputPool::Transparent),
            "the de-shield output"
        );
        assert!(pools.contains(&OutputPool::Sapling), "the shielded output");
    }

    #[tokio::test(start_paused = true)]
    async fn propose_token_is_opaque_and_single_use() {
        let mut st = funded(NOTE_VALUE);
        let to = sapling_recipient(&[0x77; 32]);
        let registry = ProposalRegistry::new();

        // One-shot: the first consume succeeds, a second of the SAME id is the
        // double-tap guard (`ProposalAlreadyUsed`) — never a second spend of the notes.
        let proposal = propose_on(&mut st, request(vec![(to.clone(), 10_000)])).expect("proposes");
        let id = registry.insert(proposal, PaymentRequest { payments: vec![] });
        assert!(registry.consume(id).is_ok(), "first consume succeeds");
        assert!(
            matches!(registry.consume(id), Err(WalletError::ProposalAlreadyUsed)),
            "a second consume of the same token is the double-tap guard",
        );

        // TTL: a token left past PROPOSAL_TTL is stale (and still burned — one-shot).
        let proposal = propose_on(&mut st, request(vec![(to, 10_000)])).expect("re-proposes");
        let id = registry.insert(proposal, PaymentRequest { payments: vec![] });
        tokio::time::advance(Duration::from_secs(PROPOSAL_TTL_SECS + 1)).await;
        assert!(
            matches!(registry.consume(id), Err(WalletError::ProposalStale)),
            "a token past its TTL is ProposalStale",
        );
    }

    #[tokio::test(start_paused = true)]
    async fn proposal_registry_is_bounded_without_consume() {
        // Gate-7 boundary for PROPOSAL_REGISTRY_MAX_LIVE: a propose-without-send loop
        // (re-quoting a fee, a background-resumed re-propose) must NOT grow the registry
        // without bound — each retained token holds §5.4-sensitive memo bytes. We retain
        // far past the cap and assert the live set stays bounded, the OLDEST id is
        // evicted (its consume → ProposalAlreadyUsed), and the NEWEST survives.
        let mut st = funded(NOTE_VALUE);
        let to = sapling_recipient(&[0x88; 32]);
        let registry = ProposalRegistry::new();

        let mut ids = Vec::new();
        for _ in 0..(PROPOSAL_REGISTRY_MAX_LIVE * 2) {
            let proposal =
                propose_on(&mut st, request(vec![(to.clone(), 10_000)])).expect("proposes");
            ids.push(registry.insert(proposal, PaymentRequest { payments: vec![] }));
            // Advance the virtual clock a touch so each `created` is strictly later than
            // the last — exactly as real wall-clock proposes are spaced — making the
            // oldest-first eviction deterministic (and well under the TTL, so no sweep).
            tokio::time::advance(Duration::from_millis(1)).await;
        }

        assert!(
            registry.live.lock().unwrap().len() <= PROPOSAL_REGISTRY_MAX_LIVE,
            "the live set never exceeds the cap",
        );
        // The very first (oldest) token was evicted — its consume is the double-tap arm.
        assert!(
            matches!(
                registry.consume(ids[0]),
                Err(WalletError::ProposalAlreadyUsed)
            ),
            "the oldest proposal is evicted under the cap",
        );
        // The most recent token (the one a user would be looking at) survives.
        assert!(
            registry.consume(*ids.last().unwrap()).is_ok(),
            "the newest proposal always survives eviction",
        );
    }

    #[tokio::test(start_paused = true)]
    async fn proposal_registry_sweeps_expired_on_insert() {
        // The TTL sweep reclaims dead entries (their memo bytes) without waiting for a
        // consume: insert one, let it expire, insert another — the first is gone.
        let mut st = funded(NOTE_VALUE);
        let to = sapling_recipient(&[0x99; 32]);
        let registry = ProposalRegistry::new();

        let stale = registry.insert(
            propose_on(&mut st, request(vec![(to.clone(), 10_000)])).expect("proposes"),
            PaymentRequest { payments: vec![] },
        );
        tokio::time::advance(Duration::from_secs(PROPOSAL_TTL_SECS + 1)).await;
        // A fresh insert triggers the sweep that drops the now-expired `stale`.
        registry.insert(
            propose_on(&mut st, request(vec![(to, 10_000)])).expect("proposes"),
            PaymentRequest { payments: vec![] },
        );
        assert_eq!(
            registry.live.lock().unwrap().len(),
            1,
            "only the fresh token remains"
        );
        assert!(
            matches!(
                registry.consume(stale),
                Err(WalletError::ProposalAlreadyUsed)
            ),
            "the swept-expired token is gone (not even ProposalStale — it was reclaimed)",
        );
    }

    #[tokio::test(start_paused = true)]
    async fn propose_mints_distinct_bindings_per_proposal() {
        // FR-17 (#396): every retained proposal carries its OWN binding — two live
        // tokens must never share one (a shared binding would let a stage recorded
        // for proposal A satisfy a pull signing proposal B, the exact confusion the
        // nonce exists to make detectable). Pinned at the registry seam: two puts
        // under two mints peek back EXACTLY what was put (`Wallet::propose` mints
        // one value onto both the DTO and this row — Copy semantics, one mint site).
        let mut st = funded(NOTE_VALUE);
        let to = sapling_recipient(&[0xAA; 32]);
        let registry = ProposalRegistry::new();

        let binding_a = SpendBinding::mint();
        let binding_b = SpendBinding::mint();
        assert_ne!(
            binding_a.as_bytes(),
            binding_b.as_bytes(),
            "two mints are distinct (256-bit OsRng nonces — a collision is a broken RNG)",
        );

        let proposal_a =
            propose_on(&mut st, request(vec![(to.clone(), 10_000)])).expect("proposes A");
        let id_a = registry.reserve_id();
        registry.put(
            id_a,
            proposal_a,
            PaymentRequest { payments: vec![] },
            binding_a,
        );
        let proposal_b = propose_on(&mut st, request(vec![(to, 10_000)])).expect("proposes B");
        let id_b = registry.reserve_id();
        registry.put(
            id_b,
            proposal_b,
            PaymentRequest { payments: vec![] },
            binding_b,
        );

        let peeked_a = registry.peek_binding(id_a).expect("peek A");
        let peeked_b = registry.peek_binding(id_b).expect("peek B");
        assert_eq!(
            peeked_a.as_bytes(),
            binding_a.as_bytes(),
            "the registry returns byte-for-byte the binding put with A",
        );
        assert_eq!(
            peeked_b.as_bytes(),
            binding_b.as_bytes(),
            "the registry returns byte-for-byte the binding put with B",
        );
        assert_ne!(
            peeked_a.as_bytes(),
            peeked_b.as_bytes(),
            "distinct proposals present distinct bindings at the sign pull",
        );
    }

    #[tokio::test(start_paused = true)]
    async fn peek_binding_does_not_consume_and_mirrors_consume_taxonomy() {
        // FR-17 (#396): `peek_binding` is the PRE-PULL fast-fail — it must speak
        // `consume`'s exact error taxonomy (unknown ⇒ ProposalAlreadyUsed, past-TTL ⇒
        // ProposalStale) so a dead id fails typed BEFORE any seed pull, while a LIVE
        // id peeks any number of times WITHOUT burning the one-shot token (`consume`
        // stays the sole authority).
        let mut st = funded(NOTE_VALUE);
        let to = sapling_recipient(&[0xBB; 32]);
        let registry = ProposalRegistry::new();

        // Unknown id ⇒ the double-tap arm, same as a consume of a missing token.
        assert!(
            matches!(
                registry.peek_binding(999),
                Err(WalletError::ProposalAlreadyUsed)
            ),
            "peek of a never-issued id mirrors consume's ProposalAlreadyUsed",
        );

        // A LIVE id peeks repeatedly (same binding each time) and the token SURVIVES:
        // the consume afterwards still succeeds — peek burned nothing.
        let proposal = propose_on(&mut st, request(vec![(to.clone(), 10_000)])).expect("proposes");
        let id = registry.insert(proposal, PaymentRequest { payments: vec![] });
        let first = registry.peek_binding(id).expect("first peek");
        let second = registry.peek_binding(id).expect("second peek");
        assert_eq!(
            first.as_bytes(),
            second.as_bytes(),
            "peek is a read — the same retained binding every time",
        );
        assert!(
            registry.consume(id).is_ok(),
            "the one-shot token survives any number of peeks (peek never consumes)",
        );

        // TTL arm: a token left past PROPOSAL_TTL peeks ProposalStale — and is NOT
        // removed by the peek (consume still finds it and burns it as Stale itself),
        // so the peek can never become a second removal authority.
        let proposal = propose_on(&mut st, request(vec![(to, 10_000)])).expect("re-proposes");
        let id = registry.insert(proposal, PaymentRequest { payments: vec![] });
        tokio::time::advance(Duration::from_secs(PROPOSAL_TTL_SECS + 1)).await;
        assert!(
            matches!(registry.peek_binding(id), Err(WalletError::ProposalStale)),
            "peek of a past-TTL token mirrors consume's ProposalStale",
        );
        assert!(
            matches!(registry.consume(id), Err(WalletError::ProposalStale)),
            "the stale token was still PRESENT after the peek (peek removed nothing) — \
             consume remains the one place it is burned",
        );
    }

    // ── Real-world money edge cases (the maintainer's operational pass) ─────────────

    #[test]
    fn propose_selects_multiple_notes_when_one_is_too_small() {
        // Spend across several received payments: neither note alone covers 50k+fee, so
        // the selector must aggregate both. A regression in the multi-input total/change
        // roll-up would only ever surface with >1 selected note.
        let mut st = funded_multi(&[30_000, 40_000]);
        let spendable = spendable_zat(&st);
        assert_eq!(
            spendable, 70_000,
            "both notes are spendable (the SSOT total)"
        );

        let to = sapling_recipient(&[0xa1; 32]);
        let amount = 50_000u64;
        let dto = summarize(
            &propose_on(&mut st, request(vec![(to, amount)])).expect("proposes across both notes"),
            0,
            SpendBinding::mint(),
        )
        .expect("summarizes");

        let marginal = marginal_fee_zat();
        assert!(
            dto.fee_zat > 0 && dto.fee_zat % marginal == 0,
            "fee is a +ve multiple of MARGINAL_FEE"
        );
        assert!(
            dto.fee_zat >= minimum_fee_zat(),
            "fee is at least the ZIP-317 grace floor"
        );
        assert_eq!(
            dto.total_zat,
            amount as i64 + dto.fee_zat,
            "total = amount + fee"
        );
        // Conservation across the SUMMED inputs (both notes spent whole): Σin = total + change.
        assert_eq!(
            dto.total_zat + dto.change_zat,
            spendable as i64,
            "Σ inputs = total debited + change"
        );
        assert!(dto.change_zat >= 0, "change is never negative");
        assert_eq!(dto.steps.len(), 1);
        assert_eq!(dto.steps[0].recipients[0].pool, OutputPool::Sapling);
    }

    #[test]
    fn propose_max_send_spends_nearly_the_whole_balance() {
        // The "send everything" button. Probe the fee, then ask for (balance − fee). The
        // send must succeed and strand nothing material — conservation (total + change ==
        // balance) holds for the single fully-spent note, and the leftover change is at
        // most the fee delta between the change-bearing probe and the (near-)no-change max.
        let mut st = funded(NOTE_VALUE);
        let spendable = spendable_zat(&st);
        let to = sapling_recipient(&[0xa2; 32]);

        let fee = summarize(
            &propose_on(&mut st, request(vec![(to.clone(), 1_000)])).expect("probe proposes"),
            0,
            SpendBinding::mint(),
        )
        .expect("summarizes")
        .fee_zat as u64;
        assert!(
            fee > 0 && fee < spendable,
            "a sane fee strictly inside the balance"
        );

        let dto = summarize(
            &propose_on(&mut st, request(vec![(to, spendable - fee)])).expect("max-send proposes"),
            0,
            SpendBinding::mint(),
        )
        .expect("summarizes");
        // Conservation for the single fully-spent note: total + change == balance.
        assert_eq!(
            dto.total_zat + dto.change_zat,
            spendable as i64,
            "nothing is created or lost"
        );
        assert!(
            dto.change_zat < fee as i64,
            "a max-send strands at most the fee delta as change"
        );
        assert!(
            dto.total_zat >= (spendable - fee) as i64,
            "nearly the whole balance leaves the wallet"
        );
    }

    #[test]
    fn propose_after_chain_advances_uses_a_newer_or_equal_anchor() {
        // A proposal sits on screen while the chain moves under it (the mobile
        // background→resume case). Re-proposing must still succeed against the NEW state
        // and target an anchor no older than before — the monotonic-anchor invariant the
        // TTL / re-propose-at-broadcast path leans on. propose_core re-reads the live DB
        // every call (no cached anchor), so this is the regression net for a future cache.
        let mut st = funded(NOTE_VALUE);
        let to = sapling_recipient(&[0xa4; 32]);
        let req = || request(vec![(to.clone(), 10_000)]);

        let first = summarize(
            &propose_on(&mut st, req()).expect("first proposes"),
            0,
            SpendBinding::mint(),
        )
        .expect("sum");

        let scan_from = st.latest_cached_block().expect("a cached block").height() + 1;
        let mut tip = None;
        for _ in 0..5 {
            let (h, _) = st.generate_empty_block();
            tip = Some(h);
        }
        st.scan_cached_blocks(scan_from, 5);
        st.wallet_mut()
            .update_chain_tip(tip.expect("advanced"))
            .expect("update chain tip");

        let second = summarize(
            &propose_on(&mut st, req()).expect("re-proposes after advance"),
            0,
            SpendBinding::mint(),
        )
        .expect("sum");
        assert_eq!(
            second.total_zat, first.total_zat,
            "the same send costs the same"
        );
        assert_eq!(second.fee_zat, first.fee_zat);
        assert!(
            second.target_height >= first.target_height,
            "the re-proposed anchor is never older ({} >= {})",
            second.target_height,
            first.target_height,
        );
    }

    #[test]
    fn propose_mid_scan_reports_the_full_honest_triple() {
        // Mid-sync: a deep (spendable) note + a shallow (still-confirming) note. An over-ask
        // must surface the WHOLE honest triple — available == balance.spendable AND
        // pending_incoming == the still-confirming value — so the host can render "you have
        // X, Y is arriving, you need Z" without a guess. pending_incoming on the propose
        // error path is otherwise unasserted anywhere.
        let mut st = funded(NOTE_VALUE);
        add_shallow_note(&mut st, 50_000);
        assert_eq!(
            spendable_zat(&st),
            NOTE_VALUE,
            "only the deep note is spendable"
        );

        let to = sapling_recipient(&[0xa7; 32]);
        match propose_on(&mut st, request(vec![(to, NOTE_VALUE + 10_000)])).unwrap_err() {
            WalletError::InsufficientFunds {
                available,
                required,
                pending_incoming,
            } => {
                assert_eq!(
                    available.zat(),
                    NOTE_VALUE as i64,
                    "available == balance.spendable (SSOT)"
                );
                assert_eq!(
                    pending_incoming.zat(),
                    50_000,
                    "the shallow note shows as still-confirming"
                );
                assert!(
                    required.zat() >= (NOTE_VALUE + 10_000) as i64,
                    "required covers the ask"
                );
            }
            other => panic!("expected InsufficientFunds, got {other:?}"),
        }
    }

    #[test]
    fn propose_on_an_unfunded_wallet_is_typed_never_a_panic() {
        // Right after account creation: the wallet exists but nothing has been scanned and
        // no note backs a send. Propose must return a TYPED error (no anchor yet ⇒
        // ProposalStale, or no value ⇒ InsufficientFunds available==0), never a panic and
        // never the payload-free ProposeFailed mask hiding a real reason.
        let mut st = fresh_state();
        let mut tip = None;
        for _ in 0..=SPENDABLE_DEPTH {
            let (h, _) = st.generate_empty_block();
            tip = Some(h);
        }
        st.wallet_mut()
            .update_chain_tip(tip.expect("a tip"))
            .expect("update chain tip");

        let to = sapling_recipient(&[0xa5; 32]);
        match propose_on(&mut st, request(vec![(to, 5_000)])) {
            Err(WalletError::ProposalStale) => {}
            Err(WalletError::InsufficientFunds { available, .. }) => {
                assert_eq!(available.zat(), 0, "a fresh wallet has zero spendable");
            }
            Err(other) => panic!("expected a typed stale/funds error, got {other:?}"),
            Ok(_) => panic!("a wallet with no spendable note cannot propose a send"),
        }
    }

    #[test]
    fn propose_change_from_a_deshield_goes_back_shielded() {
        // A partial de-shield: send some value to a transparent address, keep the rest. The
        // recipient leg discloses, and the CHANGE must return to the shielded pool (Orchard,
        // the propose_core fallback) — never leaking back out as a second transparent
        // output. The DTO carries change_zat + disclosure; the change POOL is only on the
        // retained proposal, so we inspect both (the host-side change-pool surface is an
        // inc-2d-ffi observability follow-on, noted in the handoff).
        let mut st = funded(NOTE_VALUE);
        let transparent = transparent_recipient([0xa6; 20]);
        let dto = summarize(
            &propose_on(&mut st, request(vec![(transparent, 10_000)])).expect("de-shield proposes"),
            0,
            SpendBinding::mint(),
        )
        .expect("summarizes");
        assert!(
            dto.has_transparent_recipient,
            "a transparent recipient discloses the de-shield"
        );
        assert_eq!(dto.steps[0].recipients[0].pool, OutputPool::Transparent);
        assert!(
            dto.change_zat > 0,
            "a partial de-shield returns change to the wallet"
        );

        let proposal = propose_on(
            &mut st,
            request(vec![(transparent_recipient([0xa6; 20]), 10_000)]),
        )
        .expect("re-proposes for the change-pool inspection");
        let change_pools: Vec<OutputPool> = proposal
            .steps()
            .first()
            .balance()
            .proposed_change()
            .iter()
            .map(|cv| OutputPool::from_pool_type(cv.output_pool()))
            .collect();
        assert!(
            !change_pools.is_empty(),
            "there is a change output to inspect"
        );
        // The load-bearing claim: every change output stays SHIELDED — the change from a
        // de-shield never leaks back out as a transparent output. (A sapling-funded send
        // keeps change in Sapling, the input pool; the Orchard fallback only applies when
        // no shielded input pool is preferred — either is shielded, which is the point.)
        assert!(
            change_pools
                .iter()
                .all(|p| matches!(p, OutputPool::Sapling | OutputPool::Orchard)),
            "change returns to a shielded pool, never to transparent",
        );
    }

    // ── inc-2d-2: CREATE + SIGN (the create_signed_core money-signing path) ──────

    #[test]
    fn create_signed_core_signs_and_persists_a_simple_send() {
        // §8: the create+sign core builds, proves, signs, and PERSISTS a real tx. The
        // returned txid is readable back from the wallet DB — proving the engine's
        // store-before-return (`store_transactions_to_be_sent`; the §6.3
        // persist-before-submit, so a kill before broadcast loses no bookkeeping).
        let mut st = funded(NOTE_VALUE);
        let to = sapling_recipient(&[0xb1; 32]);
        let proposal = propose_on(&mut st, request(vec![(to, 10_000)])).expect("proposes");

        let txids = sign_on(&mut st, &proposal).expect("signs + persists");
        assert_eq!(txids.len(), 1, "a single-step send is exactly one tx");
        assert!(
            st.wallet()
                .get_transaction(txids[0])
                .expect("tx read")
                .is_some(),
            "the signed tx is persisted in the wallet DB before the txid is returned",
        );
    }

    #[test]
    fn create_signed_core_signs_across_multiple_notes() {
        // §8 (MONEY): a send that the selector must fund from SEVERAL notes still signs a
        // single valid, persisted tx (the multi-input spend path — distinct proving from
        // the single-note case). 50k needs both the 30k + 40k notes.
        let mut st = funded_multi(&[30_000, 40_000]);
        let to = sapling_recipient(&[0xb2; 32]);
        let proposal =
            propose_on(&mut st, request(vec![(to, 50_000)])).expect("proposes across both notes");

        let txids = sign_on(&mut st, &proposal).expect("signs across both notes");
        assert_eq!(txids.len(), 1);
        assert!(
            st.wallet()
                .get_transaction(txids[0])
                .expect("tx read")
                .is_some(),
            "the multi-input signed tx is persisted",
        );
    }

    #[test]
    fn create_signed_key_mismatch_is_loud_never_silent() {
        // §8 (security — never a silent wrong-key sign): a USK whose UFVK is NOT the
        // account's (a foreign seed) is rejected typed `SignFailed` (the engine's
        // `KeyNotRecognized` folds there), never a tx signed with the wrong key. The key
        // check happens BEFORE proving, so this is fast (no prover needed).
        let mut st = funded(NOTE_VALUE);
        let to = sapling_recipient(&[0xb3; 32]);
        let proposal = propose_on(&mut st, request(vec![(to, 10_000)])).expect("proposes");

        let network = *st.network();
        let foreign = UnifiedSpendingKey::from_seed(&network, &[0x42; 32], AccountId::ZERO)
            .expect("a foreign USK");
        let prover = crate::prover::tx_prover();
        let err = create_signed_core(
            st.wallet_mut(),
            &network,
            foreign,
            &proposal,
            prover,
            prover,
            crate::consensus::SigningPermit::assume_current_for_test(),
        )
        .unwrap_err();
        assert!(
            matches!(err, WalletError::SignFailed),
            "a key the account doesn't own is a loud SignFailed, never a silent wrong-key sign: {err:?}",
        );
    }

    #[test]
    fn proposal_resumed_after_suspend_must_reanchor_at_send() {
        // §8 (the DISCHARGED reserved obligation — see ProposalRegistry::consume's
        // SAFETY-OBLIGATION). The suspend bug is a CHAIN-HEIGHT delta, not a clock delta:
        // virtual time cannot model it (suspend FREEZES the monotonic TTL clock while the
        // chain advances), so this is height-driven. Fund + propose, then advance the
        // RECORDED chain tip far past the proposal's anchor window WITHOUT touching the TTL
        // clock (a suspended device whose chain moved on post-resume sync), then `send`
        // (create_signed_core) MUST reject `ProposalStale` — re-anchored against the live
        // tip — rather than sign a tx to a dead anchor (born expired → funds locked to the
        // expiry height until it un-mines, §6.2).
        let mut st = funded(NOTE_VALUE);
        let to = sapling_recipient(&[0xb4; 32]);
        let proposal = propose_on(&mut st, request(vec![(to, 10_000)])).expect("proposes");
        let target = u32::from(proposal.min_target_height());

        // Record a tip well past the drift window (no virtual-clock advance — the TTL is
        // still "fresh", which is exactly the suspend trap this guard closes).
        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(
                target + PROPOSAL_ANCHOR_DRIFT_MAX_BLOCKS + 5,
            ))
            .expect("advance the recorded chain tip");

        let err = sign_on(&mut st, &proposal).unwrap_err();
        assert!(
            matches!(err, WalletError::ProposalStale),
            "a suspend-stale proposal re-anchors to ProposalStale, never a born-expired sign: {err:?}",
        );
    }

    #[test]
    fn create_signed_reanchor_boundary_is_exact() {
        // §8 (gate 7 — the exact PROPOSAL_ANCHOR_DRIFT_MAX_BLOCKS flip): a tip EXACTLY at
        // `target + DRIFT_MAX` still signs (≥ 20 blocks of mining headroom remain — the
        // builder's expiry is target + 40); one block PAST is `ProposalStale`. Pins the
        // `>` (not `>=`) boundary so a drive-by `<`/`<=` edit fails here.
        let to = sapling_recipient(&[0xb5; 32]);

        // Exactly at the window → signs.
        let mut st = funded(NOTE_VALUE);
        let proposal = propose_on(&mut st, request(vec![(to.clone(), 10_000)])).expect("proposes");
        let target = u32::from(proposal.min_target_height());
        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(
                target + PROPOSAL_ANCHOR_DRIFT_MAX_BLOCKS,
            ))
            .expect("tip exactly at the window");
        assert!(
            sign_on(&mut st, &proposal).is_ok(),
            "a proposal exactly at the drift window still signs (headroom remains)",
        );

        // One block past the window → ProposalStale.
        let mut st = funded(NOTE_VALUE);
        let proposal = propose_on(&mut st, request(vec![(to, 10_000)])).expect("proposes");
        let target = u32::from(proposal.min_target_height());
        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(
                target + PROPOSAL_ANCHOR_DRIFT_MAX_BLOCKS + 1,
            ))
            .expect("tip one past the window");
        assert!(
            matches!(sign_on(&mut st, &proposal), Err(WalletError::ProposalStale)),
            "one block past the drift window is ProposalStale (re-propose)",
        );
    }

    #[test]
    fn create_signed_on_an_unsynced_wallet_is_proposal_stale() {
        // §8: the re-anchor guard's None branch — a wallet with no recorded chain tip
        // cannot safely anchor a send. Build a real proposal on a funded wallet, then run
        // the guard against a FRESH (unsynced, `chain_height() == None`) wallet DB: it is
        // `ProposalStale` (sync first), never a sign against an absent tip.
        let mut funded_st = funded(NOTE_VALUE);
        let to = sapling_recipient(&[0xb6; 32]);
        let proposal = propose_on(&mut funded_st, request(vec![(to, 10_000)])).expect("proposes");

        let fresh = fresh_state();
        assert!(
            fresh.wallet().chain_height().expect("read").is_none(),
            "a fresh wallet has no recorded chain tip",
        );
        assert!(
            matches!(
                assert_proposal_anchor_fresh(fresh.wallet(), &proposal),
                Err(WalletError::ProposalStale)
            ),
            "an unsynced wallet can't anchor a send → ProposalStale",
        );
    }

    // ── inc-2d-2 operational pass: real-world money edges (the maintainer's distinct round) ──

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn consume_admits_exactly_one_under_concurrent_double_tap() {
        // §8 (MONEY-CRITICAL — the double-broadcast boundary; testing-patterns mandates a
        // concurrent test for a thread-safe type): a user double-taps "send" on a laggy
        // phone → N `consume(id)` of the SAME token race on the registry's std `Mutex`. The
        // one-shot guarantee (the only thing between the user and signing the same notes
        // twice) must hold UNDER CONCURRENCY, not just sequentially. EXACTLY ONE wins; the
        // rest are `ProposalAlreadyUsed`. A TOCTOU regression (check-then-remove) would pass
        // every sequential one-shot test and ship a double-spend — this catches it.
        let mut st = funded(NOTE_VALUE);
        let to = sapling_recipient(&[0xc1; 32]);
        let proposal = propose_on(&mut st, request(vec![(to, 10_000)])).expect("proposes");
        let registry = std::sync::Arc::new(ProposalRegistry::new());
        let id = registry.insert(proposal, PaymentRequest { payments: vec![] });

        const N: usize = 16;
        let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(N));
        let mut tasks = Vec::with_capacity(N);
        for _ in 0..N {
            let reg = std::sync::Arc::clone(&registry);
            let bar = std::sync::Arc::clone(&barrier);
            // all tasks rendezvous on the barrier, THEN hit consume together (real
            // contention on the multi-thread runtime).
            tasks.push(tokio::spawn(async move {
                bar.wait().await;
                reg.consume(id).is_ok()
            }));
        }
        let mut oks = 0usize;
        for t in tasks {
            if t.await.expect("task joins") {
                oks += 1;
            }
        }
        assert_eq!(
            oks, 1,
            "exactly one of {N} concurrent double-taps consumes the token (never a second spend)",
        );
        // and the token is now gone for everyone — a late tap is the double-tap guard.
        assert!(
            matches!(registry.consume(id), Err(WalletError::ProposalAlreadyUsed)),
            "after the race the token is burned",
        );
    }

    #[test]
    fn create_signed_core_anchors_a_mineable_future_expiry() {
        // §8 (MONEY — the re-anchor's CONSTRUCTIVE half): the boundary tests prove WHEN we
        // reject a stale anchor; this proves that a tx we DO sign is actually MINEABLE — its
        // on-chain expiry is `target + DEFAULT_TX_EXPIRY_DELTA` (40) and strictly above the
        // recorded tip, never born-expired (which would lock funds to a dead height until it
        // un-mines, §6.2). Read the persisted tx back and check the real expiry, tied to the
        // upstream constant (gate 7 — never a hardcoded 40), so a builder/dep change that
        // shifted the expiry can't leave the guard green and the tx born-dead.
        use zcash_primitives::transaction::builder::DEFAULT_TX_EXPIRY_DELTA;
        let mut st = funded(NOTE_VALUE);
        let to = sapling_recipient(&[0xc2; 32]);
        let proposal = propose_on(&mut st, request(vec![(to, 10_000)])).expect("proposes");
        let target = u32::from(proposal.min_target_height());

        let txids = sign_on(&mut st, &proposal).expect("signs");
        let tx = st
            .wallet()
            .get_transaction(txids[0])
            .expect("tx read")
            .expect("a persisted tx");
        let expiry = u32::from(tx.expiry_height());
        assert_eq!(
            expiry,
            target + DEFAULT_TX_EXPIRY_DELTA,
            "the signed tx expiry is the audited builder's target + delta",
        );
        let tip = u32::from(
            st.wallet()
                .chain_height()
                .expect("tip read")
                .expect("a tip"),
        );
        assert!(
            expiry > tip,
            "the signed tx is MINEABLE, never born-expired (expiry {expiry} > tip {tip})",
        );
    }

    #[test]
    fn create_signed_core_signs_a_deshield_to_transparent() {
        // §8 (MONEY — the most common real send: off-ramping to an exchange t-address). The
        // de-shield was tested only at PROPOSE (disclosure + shielded change); this drives a
        // partial de-shield THROUGH create_signed_core — the transparent-output build/prove
        // path is structurally distinct from the all-shielded happy path. The disclosure the
        // user saw and the tx we signed are the SAME proposal object.
        let mut st = funded(NOTE_VALUE);
        let transparent = transparent_recipient([0xc3; 20]);
        let proposal =
            propose_on(&mut st, request(vec![(transparent, 10_000)])).expect("de-shield proposes");
        assert!(
            summarize(&proposal, 0, SpendBinding::mint())
                .expect("sum")
                .has_transparent_recipient,
            "the de-shield discloses to the user before signing",
        );

        let txids = sign_on(&mut st, &proposal).expect("signs the de-shield");
        assert_eq!(txids.len(), 1);
        assert!(
            st.wallet()
                .get_transaction(txids[0])
                .expect("tx read")
                .is_some(),
            "the signed de-shield tx is persisted",
        );
    }

    #[test]
    fn create_signed_core_max_send_signs_to_completion() {
        // §8 (MONEY — the "send everything" button THROUGH signing): a max-send drives the
        // change strategy to a minimal/no-change output, a distinct builder edge from the
        // comfortable mid-range sign tests (zero-change / dust-policy bugs hide here). The
        // propose-side conservation is already proven; this proves the no-change tx BUILDS,
        // proves, signs, and persists.
        let mut st = funded(NOTE_VALUE);
        let spendable = spendable_zat(&st);
        let to = sapling_recipient(&[0xc4; 32]);
        let fee = summarize(
            &propose_on(&mut st, request(vec![(to.clone(), 1_000)])).expect("fee probe"),
            0,
            SpendBinding::mint(),
        )
        .expect("sum")
        .fee_zat as u64;

        let proposal =
            propose_on(&mut st, request(vec![(to, spendable - fee)])).expect("max-send proposes");
        let txids = sign_on(&mut st, &proposal).expect("max-send signs (minimal/no change)");
        assert_eq!(txids.len(), 1);
        assert!(
            st.wallet()
                .get_transaction(txids[0])
                .expect("tx read")
                .is_some(),
            "the max-send tx persists",
        );
    }

    // §8 (gate 7 — the named-constant RELATIONSHIP, arch review fold): the re-anchor
    // drift window must leave ≥ half the upstream tx-expiry delta as mining headroom (the
    // Builder sets expiry = target + DEFAULT_TX_EXPIRY_DELTA). Tied to the REAL upstream
    // constant at COMPILE TIME — a dep bump that narrows the expiry window, OR a drive-by
    // edit that widens our drift, fails to COMPILE rather than shipping born-near-expired
    // txs (mirrors the constants.rs `const _: () = assert!(…)` SSOT proofs).
    const _: () = assert!(
        PROPOSAL_ANCHOR_DRIFT_MAX_BLOCKS
            <= zcash_primitives::transaction::builder::DEFAULT_TX_EXPIRY_DELTA / 2,
    );

    #[test]
    fn map_create_err_routes_stale_vs_signfailed() {
        // §8 (gate 6 — error-path precision; the arch review fold): ONLY a reorg-pruned
        // anchor (Proposal::AnchorNotFound) is the re-propose-able STALE class; every other
        // create+sign fault — KeyNotRecognized AND the structural ProposalError variants a
        // re-propose can't fix — fails closed as SignFailed (never a misleading "re-propose"
        // hint, never a silent swallow). Pure mapping over hand-built upstream errors.
        use zcash_client_backend::data_api::error::Error as UpstreamError;
        use zcash_client_backend::proposal::ProposalError;
        type E = UpstreamError<(), (), (), (), (), ()>;

        assert!(matches!(
            map_create_err(E::DataSource(())),
            WalletError::StoreCorrupt
        ));
        assert!(
            matches!(
                map_create_err(E::Proposal(ProposalError::AnchorNotFound(
                    BlockHeight::from_u32(100)
                ))),
                WalletError::ProposalStale
            ),
            "a reorg-pruned anchor is the re-propose-able stale class",
        );
        assert!(
            matches!(
                map_create_err(E::Proposal(ProposalError::PaymentPoolsMismatch)),
                WalletError::SignFailed
            ),
            "a structural proposal fault fails closed, never a misleading re-propose",
        );
        assert!(
            matches!(map_create_err(E::KeyNotRecognized), WalletError::SignFailed),
            "a key the account doesn't own is a loud SignFailed",
        );
    }

    #[test]
    fn map_create_err_routes_reached_gap_limit_to_the_typed_tex_ceiling() {
        // Round-2 #7: the ZIP-320 ephemeral-address gap-limit ceiling is a `DataSource` error in the
        // production + harness `SqliteClientError` backend. It MUST fold to the TYPED
        // `TexSendLimitReached` (a typed, dual-natured availability ceiling the host can act on), NOT the
        // generic `DataSource ⇒ StoreCorrupt` corruption door (which would scare a user off a healthy
        // wallet) and never a silent retry. A DIFFERENT `DataSource` fault stays `StoreCorrupt` — the
        // probe is precise, not a blanket "any DataSource ⇒ ceiling". Pure mapping over hand-built
        // errors with the REAL backend error type (`DE = SqliteClientError`), the type the engine
        // actually produces (the live exhaustion is proven separately over the funded harness).
        use zcash_client_backend::data_api::error::Error as UpstreamError;
        use zcash_transparent::keys::TransparentKeyScope;
        type E = UpstreamError<SqliteClientError, (), (), (), (), ()>;

        assert!(
            matches!(
                map_create_err(E::DataSource(SqliteClientError::ReachedGapLimit(
                    TransparentKeyScope::EPHEMERAL,
                    11,
                ))),
                WalletError::TexSendLimitReached
            ),
            "the ephemeral gap-limit ceiling is the typed TexSendLimitReached, never StoreCorrupt",
        );
        assert!(
            matches!(
                map_create_err(E::DataSource(SqliteClientError::ChainHeightUnknown)),
                WalletError::StoreCorrupt
            ),
            "a non-gap-limit DataSource fault stays the corruption door — the probe does not over-match",
        );
        // The EPHEMERAL-scope narrowing (review HARDENING): a ReachedGapLimit on a NON-ephemeral
        // transparent scope is NOT the TEX ceiling — it must fall through to the conservative
        // StoreCorrupt door, not be mislabeled "too many transfers confirming". Without this case,
        // accidentally dropping the `*scope == EPHEMERAL` guard would be invisible (the EPHEMERAL
        // case above still passes either way).
        assert!(
            matches!(
                map_create_err(E::DataSource(SqliteClientError::ReachedGapLimit(
                    TransparentKeyScope::EXTERNAL,
                    0,
                ))),
                WalletError::StoreCorrupt
            ),
            "a non-EPHEMERAL ReachedGapLimit is NOT a TEX ceiling — the narrowing must fall through",
        );
        // #371: a DataSource fault that is really an out-of-disk on the create-persist is
        // the honest DiskFull (the send UI routes it to "free up space", not a terminal
        // dead-end), never the blanket StoreCorrupt seed-restore scare. The create is
        // transactional so retry-after-freeing is safe.
        let full = SqliteClientError::DbError(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_FULL),
            None,
        ));
        assert!(
            matches!(map_create_err(E::DataSource(full)), WalletError::DiskFull),
            "a create-persist SQLITE_FULL is DiskFull, never StoreCorrupt (#371)",
        );
    }

    /// `MARGINAL_FEE` / `MINIMUM_FEE` (ZIP-317) as SDK i64 zatoshis — pinned at the
    /// constant, not hardcoded, so the fee tests stay green across a legitimate parameter
    /// bump and fail loudly on an accidental fee change (gate 7).
    fn marginal_fee_zat() -> i64 {
        zcash_primitives::transaction::fees::zip317::MARGINAL_FEE.into_u64() as i64
    }
    fn minimum_fee_zat() -> i64 {
        zcash_primitives::transaction::fees::zip317::MINIMUM_FEE.into_u64() as i64
    }

    // ── inc-2d-3-b-ii-A: the double-send-guard witness (funded harness) ──────────

    #[test]
    fn claim_witness_reads_spendable_before_create_and_spent_after() {
        // THE headline money-safety proof: a proposal's claimed input notes read SPENDABLE
        // before the tx is created, and SPENT once `create_signed_core` marks them — so the
        // §6.3 guard can tell "the tx was created" from durable wallet state alone, with NO
        // cross-table transaction. A regression that recorded the wrong notes (or queried the
        // witness at an inflated height) would flip one of these assertions.
        let mut st = funded(NOTE_VALUE);
        let proposal = propose_on(
            &mut st,
            request(vec![(sapling_recipient(b"dest"), NOTE_VALUE / 3)]),
        )
        .expect("propose");
        let claims = claim_from_proposal(&proposal).expect("claim");
        assert!(!claims.is_empty(), "a real send claims ≥1 input note");

        for c in &claims {
            assert!(
                note_spendable(st.wallet(), c).expect("witness"),
                "every claimed note is spendable BEFORE the tx is created",
            );
        }

        sign_on(&mut st, &proposal).expect("create+sign");

        for c in &claims {
            assert!(
                !note_spendable(st.wallet(), c).expect("witness"),
                "every claimed note is SPENT once the tx is created (the spend witness)",
            );
        }
    }

    #[test]
    fn expired_unbroadcast_tx_frees_the_claim_note() {
        // The EXPIRY SAFETY NET (upstream `tx_unexpired_condition`): a tx created but never
        // broadcast EXPIRES (~40 blocks) and the engine frees its notes — so the witness reads
        // them spendable AGAIN and the guard safely re-proposes. This is what un-sticks the
        // narrow create-committed-but-txid-unrecorded crash window without a double-send.
        let mut st = funded(NOTE_VALUE);
        let proposal = propose_on(
            &mut st,
            request(vec![(sapling_recipient(b"dest"), NOTE_VALUE / 3)]),
        )
        .expect("propose");
        let claims = claim_from_proposal(&proposal).expect("claim");
        sign_on(&mut st, &proposal).expect("create");
        assert!(
            !note_spendable(st.wallet(), &claims[0]).expect("witness"),
            "spent right after create",
        );

        // Advance well past DEFAULT_TX_EXPIRY_DELTA (40) so the unbroadcast tx expires.
        let mut first = None;
        let mut count = 0u32;
        for _ in 0..50 {
            let (h, _) = st.generate_empty_block();
            first.get_or_insert(h);
            count += 1;
        }
        let first = first.expect("a generated block");
        st.scan_cached_blocks(first, count as usize);
        let new_tip = BlockHeight::from_u32(u32::from(first) + count - 1);
        st.wallet_mut()
            .update_chain_tip(new_tip)
            .expect("advance tip past expiry");

        assert!(
            note_spendable(st.wallet(), &claims[0]).expect("witness"),
            "the expired (never-broadcast) tx frees its note — the witness reads spendable again",
        );
    }

    #[test]
    fn claim_spans_every_selected_note_across_multiple_inputs() {
        // A send that must aggregate several notes claims ALL of them (the guard witnesses
        // each), never just the first — a partial claim would mis-read "not created".
        let mut st = funded_multi(&[40_000, 40_000]);
        // 60k + the ZIP-317 fee forces selecting BOTH 40k notes.
        let proposal = propose_on(&mut st, request(vec![(sapling_recipient(b"dest"), 60_000)]))
            .expect("propose");
        let claims = claim_from_proposal(&proposal).expect("claim");
        assert!(
            claims.len() >= 2,
            "a multi-note send claims every selected input"
        );
        assert!(
            claims.iter().all(|c| c.protocol == PROTO_SAPLING),
            "sapling-funded notes carry the sapling tag",
        );
        let mut positions: Vec<_> = claims.iter().map(|c| (c.txid, c.output_index)).collect();
        positions.sort();
        positions.dedup();
        assert_eq!(
            positions.len(),
            claims.len(),
            "each claimed position is distinct"
        );
    }

    #[test]
    fn reconcile_decides_the_recovery_action() {
        // The pure decision matrix (no DB): no group + all-spendable ⇒ re-propose (the tx was
        // never created); ANY recorded group ⇒ reconcile the group, whatever the witness reads;
        // spent with an EMPTY group ⇒ wait (the narrow create-committed-but-unrecorded window).
        assert_eq!(reconcile(true, &[]), Recovery::ReProposeFresh);
        assert_eq!(
            reconcile(true, &[[7u8; 32]]),
            Recovery::ReBroadcast(vec![[7u8; 32]]),
            "a recorded group wins over a spendable witness (bare expiry frees the notes — S7 C1)",
        );
        assert_eq!(
            reconcile(false, &[[9u8; 32]]),
            Recovery::ReBroadcast(vec![[9u8; 32]]),
            "spent + a one-tx group ⇒ re-broadcast it",
        );
        assert_eq!(
            reconcile(false, &[[0xA0u8; 32], [0xB1u8; 32]]),
            Recovery::ReBroadcast(vec![[0xA0u8; 32], [0xB1u8; 32]]),
            "spent + a TEX two-tx group ⇒ re-broadcast the WHOLE ordered group",
        );
        assert_eq!(reconcile(false, &[]), Recovery::AwaitWitness);
    }

    #[test]
    fn protocol_tag_round_trips_and_rejects_unknown() {
        // All three variants. Stated precisely, because an earlier draft of this
        // comment claimed they were "enumerated from the type" and they are not —
        // this is a hand-written array, and a hand-written array is exactly what
        // was green here listing two of two before the wave and would have stayed
        // green listing two of three. What actually forces a fourth pool to be
        // handled is the compiler, at `proto_tag`: `ShieldedPool` is NOT
        // `#[non_exhaustive]` (`zcash_protocol-0.10.5` `src/lib.rs:38`), so adding
        // a variant upstream breaks that match. This loop's job is narrower — it
        // proves the mapping ROUND-TRIPS and that the tags are distinct.
        for pool in [
            ShieldedPool::Sapling,
            ShieldedPool::Orchard,
            ShieldedPool::Ironwood,
        ] {
            assert_eq!(
                proto_from_tag(proto_tag(pool)).expect("round trip"),
                pool,
                "{pool:?} must round-trip through its storage tag"
            );
        }
        // The tags are DISTINCT. This is the assertion that kills the tempting
        // `Ironwood => PROTO_ORCHARD` repair: that mapping still round-trips
        // "successfully" in one direction and files an Ironwood note under the
        // Orchard tag, where `get_spendable_note` can resolve it to a different,
        // still-unspent note — the witness then reads "our transaction was never
        // created" and the recovery re-sends a payment already on chain.
        let tags = [
            proto_tag(ShieldedPool::Sapling),
            proto_tag(ShieldedPool::Orchard),
            proto_tag(ShieldedPool::Ironwood),
        ];
        let distinct: std::collections::BTreeSet<u8> = tags.iter().copied().collect();
        assert_eq!(
            distinct.len(),
            tags.len(),
            "every shielded pool needs its OWN storage tag: {tags:?}"
        );
        // An unknown tag is corruption. `3`, not `2` — `2` became `PROTO_IRONWOOD`
        // in this wave, and a test that kept asserting `2` is corrupt would have
        // inverted into asserting the new pool is unreadable.
        assert!(
            matches!(proto_from_tag(3), Err(WalletError::StoreCorrupt)),
            "an unknown protocol tag is corruption, never a silent default",
        );
        assert!(
            matches!(proto_from_tag(u8::MAX), Err(WalletError::StoreCorrupt)),
            "an unknown protocol tag is corruption, never a silent default",
        );
    }

    #[test]
    fn output_pool_round_trips_every_pool_type() {
        // The confirm screen's DTO (survey S9). `from_pool_type` is what labels the
        // pool the user sees before authorising a payment, and its natural repair
        // under a new upstream variant is a wildcard arm — which would silently
        // relabel an Ironwood output as something else with nothing failing.
        //
        // Pairing the two directions is the check: `to_pool_type` must have an arm
        // for every `OutputPool`, so adding a variant to one side without the other
        // does not compile, and the round-trip below proves the pairing is an
        // identity rather than just total.
        for p in [
            PoolType::Transparent,
            PoolType::SAPLING,
            PoolType::ORCHARD,
            PoolType::IRONWOOD,
        ] {
            assert_eq!(
                OutputPool::from_pool_type(p).to_pool_type(),
                p,
                "{p} must survive the round trip through the display DTO"
            );
        }
        // And the shielded pools do not collapse onto one another — the specific
        // mistake a wildcard would make.
        assert_ne!(
            OutputPool::from_pool_type(PoolType::IRONWOOD),
            OutputPool::from_pool_type(PoolType::ORCHARD),
            "an Ironwood output must not be shown to the user as Orchard"
        );
    }

    // ── inc-2d-3-b-ii-A operational pass: real-world money/crash edges ───────────

    /// Read the persisted created tx's actual expiry height back from the wallet — the
    /// money-safe boundary the guard's expiry net pivots on (not a hardcoded delta).
    fn created_tx_expiry(st: &HarnessState, txid: TxId) -> u32 {
        use zcash_client_backend::data_api::WalletRead;
        let tx = st
            .wallet()
            .get_transaction(txid)
            .expect("read tx")
            .expect("the just-created tx is present");
        u32::from(tx.expiry_height())
    }

    #[test]
    fn queued_intent_reconciles_to_its_tx_via_note_spend_witness() {
        // THE §8-OWED integration seam (queued_intent_reconciles_to_its_tx_via_note_spend_witness):
        // a REAL proposal's claim threaded THROUGH the intent store, then witnessed — proving the
        // storage half (intent_store) and the logic half (send) agree on a real proposal
        // end-to-end. Today they are proven separately (storage uses synthetic [9;32] claims; send
        // builds real claims it never persists); nothing proves a real claim survives the
        // codec/storage round-trip AND then witnesses correctly. This is the seam 3-b-ii-B rides.
        let mut st = funded(NOTE_VALUE);
        let proposal = propose_on(
            &mut st,
            request(vec![(sapling_recipient(b"dest"), NOTE_VALUE / 3)]),
        )
        .expect("propose");
        let claims = claim_from_proposal(&proposal).expect("claim");

        // Persist the intent + its REAL claim, create the tx, record the REAL spending txid.
        let mut store = Connection::open_in_memory().expect("intent store conn");
        crate::intent_store::ensure_table(&store).expect("table");
        let id = crate::intent_store::enqueue(&mut store, "zcash:real-intent", 0, None, None)
            .expect("enqueue");
        assert!(crate::intent_store::mark_submitting(&mut store, id, &claims).expect("submit"));
        let txids = sign_on(&mut st, &proposal).expect("create+sign");
        let spending_txid: [u8; 32] = *txids[0].as_ref();
        assert!(crate::intent_store::mark_sent(&mut store, id, &spending_txid).expect("sent"));

        // Reconcile from the STORED state, exactly as the 3-b-ii-B hook will.
        let in_flight = crate::intent_store::list_in_flight(&store).expect("list");
        assert_eq!(in_flight.len(), 1);
        let row = &in_flight[0];
        assert_eq!(
            row.claims, claims,
            "the real claim round-trips encode→store→decode byte-exact"
        );
        assert_eq!(
            row.txids,
            vec![spending_txid],
            "the real spending txid round-trips as a one-element group"
        );
        let all_spendable = row
            .claims
            .iter()
            .all(|c| note_spendable(st.wallet(), c).expect("witness"));
        assert!(!all_spendable, "after create the claim notes read spent");
        assert_eq!(
            reconcile(all_spendable, &row.txids),
            Recovery::ReBroadcast(vec![spending_txid]),
            "spent + recorded group ⇒ re-broadcast the persisted tx (never re-propose)",
        );

        // The mined cleanup edge: delete removes the in-flight row.
        assert!(crate::intent_store::delete(&mut store, id).expect("delete"));
        assert!(
            crate::intent_store::list_in_flight(&store)
                .expect("empty")
                .is_empty()
        );
    }

    #[test]
    fn claim_note_stays_spent_one_block_before_expiry_and_frees_exactly_at_expiry() {
        // GATE-7 on the DOUBLE-SPEND HINGE: the witness must flip spendable EXACTLY at the
        // upstream tx_unexpired_condition boundary (target > expiry), never a block early — a
        // one-block-early flip would free a still-mineable tx's note → re-propose → double-spend.
        // The safety-net test jumps 50 blocks; this pins the exact flip against the persisted
        // expiry, so a silent upstream `>=`→`>` drift fails here.
        let mut st = funded(NOTE_VALUE);
        let proposal = propose_on(
            &mut st,
            request(vec![(sapling_recipient(b"dest"), NOTE_VALUE / 3)]),
        )
        .expect("propose");
        let claims = claim_from_proposal(&proposal).expect("claim");
        let txids = sign_on(&mut st, &proposal).expect("create"); // notes spent; tx unbroadcast
        let expiry = created_tx_expiry(&st, txids[0]);

        // tip = expiry-1 ⇒ witness target = expiry ⇒ expiry >= target ⇒ STILL live ⇒ spent.
        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(expiry - 1))
            .expect("tip one block before expiry");
        assert!(
            !note_spendable(st.wallet(), &claims[0]).expect("witness"),
            "one block before expiry the spending tx is still live — the note stays SPENT",
        );

        // tip = expiry ⇒ target = expiry+1 ⇒ expiry < target ⇒ EXPIRED ⇒ note freed.
        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(expiry))
            .expect("tip exactly at expiry");
        assert!(
            note_spendable(st.wallet(), &claims[0]).expect("witness"),
            "exactly at expiry the tx can no longer mine — the note FREES (safe to re-propose)",
        );
    }

    #[test]
    fn submitting_intent_awaits_witness_then_re_proposes_after_expiry() {
        // CRASH WINDOW (b): the engine CREATED the tx (notes spent) but the aux `txid` write was
        // killed before it landed — a Submitting row with a claim and NO txid. Recovery must NOT
        // re-propose (the tx is live → double-spend) and cannot re-broadcast (no txid) ⇒ it WAITS
        // (AwaitWitness). The ~40-block expiry then frees the never-recorded tx's notes ⇒ the next
        // reconcile re-proposes cleanly. This is the bounded self-heal of the narrowest window.
        let mut st = funded(NOTE_VALUE);
        let proposal = propose_on(
            &mut st,
            request(vec![(sapling_recipient(b"dest"), NOTE_VALUE / 3)]),
        )
        .expect("propose");
        let claims = claim_from_proposal(&proposal).expect("claim");
        let txids = sign_on(&mut st, &proposal).expect("create"); // notes spent; txid NOT recorded
        let expiry = created_tx_expiry(&st, txids[0]);

        let spendable_now = claims
            .iter()
            .all(|c| note_spendable(st.wallet(), c).expect("witness"));
        assert!(!spendable_now, "the created tx spent the notes");
        assert_eq!(
            reconcile(spendable_now, &[]),
            Recovery::AwaitWitness,
            "spent + an EMPTY group ⇒ WAIT — never a re-propose (no double-spend), never a stuck strand",
        );

        // Advance past expiry: the never-broadcast tx expires, the engine frees its notes.
        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(expiry))
            .expect("advance past expiry");
        let spendable_after = claims
            .iter()
            .all(|c| note_spendable(st.wallet(), c).expect("witness"));
        assert!(spendable_after, "expiry freed the orphaned tx's notes");
        assert_eq!(
            reconcile(spendable_after, &[]),
            Recovery::ReProposeFresh,
            "freed ⇒ the intent re-proposes cleanly — the window self-heals within the expiry bound",
        );
    }

    // ── inc-2d-3-b-ii-B: the resubmission ENGINE-SIDE cores end-to-end ───────────
    //
    // The A tests above drive the PURE decision (`reconcile`) over hand-threaded state. These
    // drive the DB-MUTATING cores the hook actually calls (`prepare_queued`/`reconcile_inflight`)
    // over the funded harness — the production money path, no live network.

    /// A fresh in-memory aux intent store (the SECOND connection the live wallet keeps).
    fn aux_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("aux conn");
        crate::intent_store::ensure_table(&conn).expect("ensure table");
        // Production's migrate creates the accepted-mark table beside the intents (`db.rs`);
        // a deposit's reconcile reads it (F01).
        crate::delivery::ensure_table(&conn).expect("accepted table");
        conn
    }

    /// Fund a wallet, enqueue an intent, and drive it `Queued`→`Sent` via [`prepare_queued`] (the
    /// production path: propose → claim → mark_submitting → create → mark_sent). Returns the
    /// harness, its aux store, the intent id, and the recorded spending txid — the shared setup for
    /// the reconcile-path tests.
    fn prepared_sent_intent() -> (HarnessState, Connection, QueuedSendId, [u8; 32]) {
        let mut st = funded(NOTE_VALUE);
        let mut aux = aux_conn();
        let id =
            crate::intent_store::enqueue(&mut aux, "zcash:queued", 1, None, None).expect("enqueue");
        let req = request(vec![(sapling_recipient(b"dest"), NOTE_VALUE / 3)]);
        let network = *st.network();
        let aid = st.test_account().expect("acct").id();
        let usk = st.test_account().expect("acct").usk().clone();
        let prover = crate::prover::tx_prover();
        let prepared = prepare_queued(
            st.wallet_mut(),
            &network,
            &mut aux,
            aid,
            req,
            usk,
            prover,
            prover,
            id,
            None,
            0,
            0,
            crate::consensus::SigningPermit::assume_current_for_test(),
        )
        .expect("prepare");
        assert!(
            matches!(prepared, Prepared::Broadcast(ref raw) if !raw.is_empty()),
            "a funded queued intent creates a tx to broadcast, got {prepared:?}",
        );
        let txid = *crate::intent_store::list_in_flight(&aux).expect("inflight")[0]
            .txids
            .last()
            .expect("txid recorded after create");
        (st, aux, id, txid)
    }

    #[test]
    fn resubmit_prepare_drives_queued_to_sent_and_flips_the_witness_spent() {
        // THE headline: `prepare_queued` advances a real Queued intent all the way to `Sent` with
        // the txid recorded BEFORE broadcast, and the guard witness flips — the claimed notes now
        // read SPENT, so a concurrent re-propose can never double-spend.
        let (st, aux, _id, _txid) = prepared_sent_intent();
        let row = crate::intent_store::list_in_flight(&aux).expect("inflight");
        assert_eq!(row.len(), 1);
        assert_eq!(
            row[0].state,
            crate::intent_store::IntentState::Sent,
            "Queued→Sent in one prepare",
        );
        assert!(
            !row[0].txids.is_empty(),
            "the txid group is durable BEFORE any broadcast (the delete-on-mined invariant)",
        );
        assert!(
            row[0]
                .claims
                .iter()
                .all(|c| !note_spendable(st.wallet(), c).expect("witness")),
            "after create the claimed notes read SPENT — the guard blocks a re-propose",
        );
        assert!(
            crate::intent_store::list_queued(&aux)
                .expect("q")
                .is_empty(),
            "the intent left the Queued list",
        );
    }

    #[test]
    fn resubmit_reconcile_rebroadcasts_an_unmined_sent_intent() {
        // A `Sent` intent whose tx has NOT yet mined → re-broadcast its persisted bytes (best-effort
        // delivery on every pass until it lands), and the row is untouched (NOT deleted until mined).
        let (st, mut aux, _id, _txid) = prepared_sent_intent();
        let row = crate::intent_store::list_in_flight(&aux).expect("inflight")[0].clone();
        let action = reconcile_inflight(st.wallet(), &mut aux, &row, 0).expect("reconcile");
        assert!(
            matches!(action, Reconciled::Rebroadcast { txs: ref raw, .. } if !raw.is_empty()),
            "unmined Sent ⇒ re-broadcast the persisted tx, got {action:?}",
        );
        assert_eq!(
            crate::intent_store::list_in_flight(&aux)
                .expect("still")
                .len(),
            1,
            "a re-broadcast does NOT remove the row (it stays Sent until mined)",
        );
    }

    #[test]
    fn resubmit_reconcile_waits_for_burial_then_deletes_a_mined_sent_intent() {
        // The §6.3 delete-on-mined cleanup, NOW gated on REORG BURIAL (round-2 #6): a tx mined but
        // still SHALLOW (≤ `REORG_MAX_BLOCKS` deep) is reversible, so the row WAITS (Awaiting); only
        // once it is buried beyond `REORG_MAX_BLOCKS` is it irreversibly owned by the chain and the
        // row deleted. Mine the created tx, scan it, then drive the tip across the burial boundary.
        let (mut st, mut aux, _id, txid) = prepared_sent_intent();
        let (mined_h, _) = st.generate_next_block_including(TxId::from_bytes(txid));
        st.scan_cached_blocks(mined_h, 1);

        // SHALLOW (tip == mined height, one confirmation): on-chain but reorg-reversible ⇒
        // AwaitingBurial, NOT deleted. The recovery row must survive a possible deep reorg.
        st.wallet_mut()
            .update_chain_tip(mined_h)
            .expect("tip at the mined block (one confirmation)");
        let row = crate::intent_store::list_in_flight(&aux).expect("inflight")[0].clone();
        let shallow = reconcile_inflight(st.wallet(), &mut aux, &row, 0).expect("reconcile");
        assert_eq!(
            shallow,
            Reconciled::AwaitingBurial,
            "a shallow (1-confirmation) mine WAITS for burial — it is not yet irreversibly deleted",
        );
        assert_eq!(
            crate::intent_store::list_in_flight(&aux)
                .expect("still")
                .len(),
            1,
            "the recovery row survives until the mine is buried",
        );

        // BURIAL BOUNDARY (tip == mined + REORG_MAX_BLOCKS - 1): exactly ONE block short of burial —
        // a depth-`REORG_MAX_BLOCKS` reorg can still reach the mining block, so STILL AwaitingBurial.
        // Pins the `>=` against an off-by-one that would delete a block too early.
        let one_short = BlockHeight::from_u32(u32::from(mined_h) + REORG_MAX_BLOCKS - 1);
        st.wallet_mut()
            .update_chain_tip(one_short)
            .expect("tip one block short of burial");
        let row = crate::intent_store::list_in_flight(&aux).expect("inflight")[0].clone();
        assert_eq!(
            reconcile_inflight(st.wallet(), &mut aux, &row, 0).expect("reconcile"),
            Reconciled::AwaitingBurial,
            "one block short of REORG_MAX_BLOCKS depth ⇒ still waiting, not yet deleted",
        );

        // BURIED (tip == mined height + REORG_MAX_BLOCKS): beyond the deepest auto-recoverable reorg
        // ⇒ the chain irreversibly owns the send, delete the row. The intervening blocks need not be
        // scanned — burial reads `get_tx_height` (the mined height) against the chain tip only.
        let buried_tip = BlockHeight::from_u32(u32::from(mined_h) + REORG_MAX_BLOCKS);
        st.wallet_mut()
            .update_chain_tip(buried_tip)
            .expect("tip a full reorg-depth past the mined block");
        let row = crate::intent_store::list_in_flight(&aux).expect("inflight")[0].clone();
        let action = reconcile_inflight(st.wallet(), &mut aux, &row, 0).expect("reconcile");
        assert_eq!(
            action,
            Reconciled::Deleted,
            "once buried beyond REORG_MAX_BLOCKS ⇒ delete (the chain irreversibly owns it)",
        );
        assert!(
            crate::intent_store::list_in_flight(&aux)
                .expect("gone")
                .is_empty(),
            "the row is removed once its tx is buried",
        );
    }

    /// **R12 §4.5 test 6 — decode is corruption (B1), at a swept door.** Garbage in
    /// `transactions.raw` for a transaction the wallet CREATED (signed, persisted,
    /// never mined) is damage to our own bytes: the engine's `get_transaction` →
    /// `parse_tx` fails with `SqliteClientError::Io`, and `read_tx_expiry` — on
    /// every reconcile pass — must return `StoreCorrupt`, never the "retrying" `Io`
    /// a plain classification of that `Io` would give. A CONTROL at the base (the
    /// blind mapping already said `StoreCorrupt`); it reds if the sweep classifies
    /// this door without the decode rule. Two garbage shapes: truncated
    /// (`UnexpectedEof`) and overwritten (`InvalidData`).
    #[test]
    fn garbage_in_a_created_transactions_raw_reads_store_corrupt_at_read_tx_expiry() {
        use crate::test_support::HistoryHarness;
        let mut h = HistoryHarness::new();
        h.mine_received(100_000);
        h.mine_empty(10);
        h.scan();
        let recipient = ZcashAddress::from_transparent_p2pkh(NetworkType::Regtest, [0x71; 20]);
        let txid: [u8; 32] = *h.create_send(recipient, 20_000).as_ref();
        assert!(
            read_tx_expiry(h.wallet(), txid)
                .expect("the control: the created tx reads before any tamper")
                .is_some(),
            "the control: the created tx is in the store"
        );

        let conn = h.read_conn();
        let raw: Vec<u8> = conn
            .query_row(
                "SELECT raw FROM transactions WHERE txid = ?1",
                rusqlite::params![txid.as_slice()],
                |r| r.get(0),
            )
            .expect("the created tx's raw bytes");
        for (shape, garbage) in [
            ("truncated", raw[..raw.len() / 2].to_vec()),
            ("overwritten", vec![0xFF; raw.len()]),
        ] {
            conn.execute(
                "UPDATE transactions SET raw = ?1 WHERE txid = ?2",
                rusqlite::params![garbage, txid.as_slice()],
            )
            .expect("tamper the stored bytes");
            match read_tx_expiry(h.wallet(), txid) {
                Err(WalletError::StoreCorrupt) => {}
                other => panic!(
                    "R12 §4.1: a {shape} `transactions.raw` is our own damaged bytes — \
                     StoreCorrupt, never the retrying Io; got {other:?}"
                ),
            }
        }
    }

    // ── §3.2i-2 slice B pt 4 + round-2 #1: the TEX two-step recovery machine ──────
    //
    // Drive a REAL ZIP-320 two-step ([tx0, tx1]) through the multi-txid recovery over the funded
    // harness (signing via `sign_two_step_on`, the raw engine create — post-gate-removal `create_signed_core`
    // signs a two-step in production too). These pin the
    // skip-mined ordered re-broadcast, the delete-on-FINAL-mined, and the expiry→Stranded terminal.

    #[test]
    fn tex_two_step_rebroadcasts_the_whole_ordered_group_when_neither_mined() {
        let mut st = funded(NOTE_VALUE);
        let (mut aux, _id, group) = tex_two_step_sent_row(&mut st);
        let row = crate::intent_store::list_in_flight(&aux).expect("inflight")[0].clone();
        assert_eq!(
            row.txids, group,
            "the Sent row carries the ordered two-tx group"
        );

        let action = reconcile_inflight(st.wallet(), &mut aux, &row, 0).expect("reconcile");
        let raws = match action {
            Reconciled::Rebroadcast { txs: raws, .. } => raws,
            other => panic!("both unmined ⇒ Rebroadcast the group, got {other:?}"),
        };
        // Both unmined ⇒ re-broadcast BOTH, tx0 BEFORE tx1 (the pool-crossing order the engine built).
        assert_eq!(raws.len(), 2, "both unmined txs re-broadcast");
        assert_eq!(
            raws[0],
            read_raw_tx(st.wallet(), group[0])
                .expect("tx0")
                .expect("present"),
            "tx0 is re-broadcast first",
        );
        assert_eq!(
            raws[1],
            read_raw_tx(st.wallet(), group[1])
                .expect("tx1")
                .expect("present"),
            "tx1 is re-broadcast second",
        );
        assert_eq!(
            crate::intent_store::list_in_flight(&aux)
                .expect("still")
                .len(),
            1,
            "a re-broadcast leaves the row Sent (not deleted until the FINAL tx mines)",
        );
    }

    // ── the INTERACTIVE two-step outbox enrolment (Option A) ──────────────────────────
    // These drive `create_two_step_enrolled` with the raw engine create (`sign_two_step_on`) — the
    // SAME shape the production path takes now that gate-removal (2e-2b-v-5) is LANDED: production
    // mints a real two-step token at `Wallet::propose` and signs it via `create_signed_core` (whose
    // own `steps() > 1` gate is gone). These tests pin the enrol→create→record sequence directly.

    #[test]
    fn interactive_two_step_enrols_a_sent_row_the_recovery_can_rebroadcast() {
        // The keystone: an interactive TEX send enrols the [tx0, tx1] group AROUND the create, and
        // the resulting Sent row plugs into the SAME recovery the queued path uses — proven by
        // driving `reconcile_inflight` over it and getting the whole ordered group back (so a
        // missed tx1 is re-broadcast next pass, the whole point of Option A).
        let mut st = funded(NOTE_VALUE);
        let to = tex_recipient([0x07; 20]);
        let proposal = propose_on(&mut st, request(vec![(to, 20_000)])).expect("TEX proposes");
        assert_eq!(
            proposal.steps().len(),
            2,
            "a TEX send is a two-step proposal"
        );

        let mut aux = Connection::open_in_memory().expect("aux conn");
        crate::intent_store::ensure_table(&aux).expect("table");

        let txids = create_two_step_enrolled(
            &mut aux,
            &proposal,
            "zcash:tex-interactive",
            0,
            None,
            || Ok(sign_two_step_on(&mut st, &proposal)),
        )
        .expect("enrol + create the two-step");
        assert_eq!(txids.len(), 2, "the engine created tx0 + tx1");
        let group: Vec<[u8; 32]> = txids.iter().map(|t| *t.as_ref()).collect();

        // The row transitioned Queued → Submitting → Sent, carrying the ordered group (the
        // witness / delete-on-final contract the recovery keys on).
        let inflight = crate::intent_store::list_in_flight(&aux).expect("inflight");
        assert_eq!(inflight.len(), 1, "exactly one enrolled in-flight row");
        assert_eq!(
            inflight[0].state,
            crate::intent_store::IntentState::Sent,
            "the row reached the Sent terminal (Queued → Submitting → Sent)",
        );
        assert_eq!(
            inflight[0].txids, group,
            "the Sent row carries the ordered [tx0, tx1] group",
        );
        assert!(
            crate::intent_store::list_queued(&aux)
                .expect("queued")
                .is_empty(),
            "no Queued residue — the row is fully Sent",
        );

        // It reconciles identically to a queued-enrolled `tex_two_step_sent_row`: neither mined ⇒
        // re-broadcast the whole ordered group, tx0 before tx1.
        let row = inflight[0].clone();
        let action = reconcile_inflight(st.wallet(), &mut aux, &row, 0).expect("reconcile");
        match action {
            Reconciled::Rebroadcast { txs: raws, .. } => {
                assert_eq!(raws.len(), 2, "both unmined txs re-broadcast, in order");
                assert_eq!(
                    raws[0],
                    read_raw_tx(st.wallet(), group[0])
                        .expect("tx0")
                        .expect("present"),
                    "tx0 is re-broadcast first",
                );
                assert_eq!(
                    raws[1],
                    read_raw_tx(st.wallet(), group[1])
                        .expect("tx1")
                        .expect("present"),
                    "tx1 is re-broadcast second",
                );
            }
            other => panic!("an enrolled, unmined two-step ⇒ Rebroadcast the group, got {other:?}"),
        }
    }

    #[test]
    fn interactive_two_step_create_failure_surfaces_the_error_verbatim_and_leaves_no_row() {
        // A create fault spends NO note (the engine persists all-or-nothing), so the freshly-
        // enrolled row is DELETED — the synchronous user re-proposes; no orphan and no
        // non-draining two-step Queued row. (Diverges from `prepare_queued`'s reset-to-Queued,
        // which serves a pre-existing background-drained intent.) Driven over THREE faults —
        // including the REALISTIC `ProposalStale` from `create_signed_core`'s live-tip re-anchor —
        // so the test proves the create's OWN error surfaces VERBATIM: an error-masking mutation
        // that collapsed the cleanup arm to a hardcoded variant would fail the cases it doesn't
        // match (`ProposalStale` and `SignFailed` carry different downstream semantics —
        // re-anchor-and-retry vs hard-fail — so masking one as the other is money-adjacent).
        let mut st = funded(NOTE_VALUE);
        let to = tex_recipient([0x08; 20]);
        let proposal = propose_on(&mut st, request(vec![(to, 20_000)])).expect("TEX proposes");

        let mut aux = Connection::open_in_memory().expect("aux conn");
        crate::intent_store::ensure_table(&aux).expect("table");

        // ProposalStale — the realistic re-anchor failure; must NOT be masked as SignFailed.
        let stale =
            create_two_step_enrolled(&mut aux, &proposal, "zcash:tex-fail", 0, None, || {
                Err(WalletError::ProposalStale)
            });
        assert!(
            matches!(stale, Err(WalletError::ProposalStale)),
            "the realistic re-anchor failure surfaces as ProposalStale, not masked",
        );
        assert!(
            crate::intent_store::list_in_flight(&aux)
                .expect("inflight")
                .is_empty()
        );
        assert!(
            crate::intent_store::list_queued(&aux)
                .expect("queued")
                .is_empty()
        );

        // SignFailed — a proving/build fault.
        let sign_failed =
            create_two_step_enrolled(&mut aux, &proposal, "zcash:tex-fail", 0, None, || {
                Err(WalletError::SignFailed)
            });
        assert!(
            matches!(sign_failed, Err(WalletError::SignFailed)),
            "a sign fault surfaces"
        );
        assert!(
            crate::intent_store::list_in_flight(&aux)
                .expect("inflight")
                .is_empty()
        );
        assert!(
            crate::intent_store::list_queued(&aux)
                .expect("queued")
                .is_empty()
        );

        // StoreCorrupt — a DB fault during create.
        let corrupt =
            create_two_step_enrolled(&mut aux, &proposal, "zcash:tex-fail", 0, None, || {
                Err(WalletError::StoreCorrupt)
            });
        assert!(
            matches!(corrupt, Err(WalletError::StoreCorrupt)),
            "a store fault surfaces"
        );
        assert!(
            crate::intent_store::list_in_flight(&aux)
                .expect("inflight")
                .is_empty(),
            "no Submitting/Sent row survives any create fault",
        );
        assert!(
            crate::intent_store::list_queued(&aux)
                .expect("queued")
                .is_empty(),
            "and no non-draining two-step Queued row is left behind by any create fault",
        );
    }

    #[test]
    fn interactive_two_step_on_a_full_outbox_is_a_pre_spend_terminal() {
        // The outbox cap is an HONEST terminal reached BEFORE any note is spent: `enqueue` fails
        // `QueuedSendsFull`, the injected create NEVER runs (so no tx0 mines → nothing can
        // strand), and the user retries once the queue drains.
        let mut st = funded(NOTE_VALUE);
        let to = tex_recipient([0x09; 20]);
        let proposal = propose_on(&mut st, request(vec![(to, 20_000)])).expect("TEX proposes");

        let mut aux = Connection::open_in_memory().expect("aux conn");
        crate::intent_store::ensure_table(&aux).expect("table");
        for _ in 0..crate::constants::QUEUED_SEND_INTENTS_MAX {
            crate::intent_store::enqueue(&mut aux, "zcash:filler", 0, None, None)
                .expect("fill the outbox");
        }

        let result =
            create_two_step_enrolled(&mut aux, &proposal, "zcash:tex-full", 0, None, || {
                panic!("the create MUST NOT run when the outbox is full — no note may be spent")
            });
        assert!(
            matches!(result, Err(WalletError::QueuedSendsFull)),
            "an honest pre-spend terminal",
        );
        assert_eq!(
            crate::intent_store::list_queued(&aux)
                .expect("queued")
                .len(),
            crate::constants::QUEUED_SEND_INTENTS_MAX,
            "no extra row was added past the cap",
        );
        assert!(
            crate::intent_store::list_in_flight(&aux)
                .expect("inflight")
                .is_empty(),
            "nothing entered the submit window",
        );
    }

    // ── §3.2i-2 slice B3-d-1: the multi-txid QUEUED DRAIN (drain_multi + #10) ────────────────────
    //
    // `drain_multi` is the ONE drain body BOTH `prepare_queued`'s single-step AND two-step paths
    // flow through in production (gate-removal, 2e-2b-v-5); these drive it over a REAL two-step to
    // prove the queued multi-txid drain. The end-to-end queued-TEX drain through `prepare_queued`
    // is pinned by `prepare_queued_drains_a_tex_two_step_after_gate_removal`.

    #[test]
    fn drain_multi_drains_a_queued_two_step_into_an_ordered_sent_group() {
        // The keystone of the queued drain: a gate-free two-step create flows through `drain_multi`,
        // recording the ordered [tx0, tx1] group BEFORE broadcast and returning both raws IN ORDER.
        // The resulting Sent row reconciles back to the whole ordered group (the SAME recovery a
        // single-step or interactively-enrolled row uses), so a missed tx1 re-broadcasts.
        let mut st = funded(NOTE_VALUE);
        let to = tex_recipient([0x0A; 20]);
        let proposal = propose_on(&mut st, request(vec![(to, 20_000)])).expect("TEX proposes");
        assert_eq!(
            proposal.steps().len(),
            2,
            "a TEX send is a two-step proposal"
        );

        let mut aux = aux_conn();
        let id = crate::intent_store::enqueue(&mut aux, "zcash:tex-queued", 0, None, None)
            .expect("enqueue");

        let network = *st.network();
        let usk = st.test_account().expect("acct").usk().clone();
        let prover = crate::prover::tx_prover();
        let prepared = drain_multi(st.wallet_mut(), &mut aux, &proposal, id, |db| {
            // The gate-free engine create `create_signed_core` wraps — the shape the production
            // single-step path runs today, and the shape `prepare_queued` runs once the slice-A
            // create gate is removed LAST (§251 sequencing).
            Ok(
                create_proposed_transactions::<_, _, Infallible, StandardFeeRule, Infallible, _>(
                    db,
                    &network,
                    prover,
                    prover,
                    &SpendingKeys::from_unified_spending_key(usk),
                    OvkPolicy::Sender,
                    &proposal,
                    // No expiry override — same as production `create_signed_core`.
                    None,
                )
                .expect("two-step create+sign")
                .into_iter()
                .collect(),
            )
        })
        .expect("drain the queued two-step");

        let raws = match prepared {
            Prepared::Broadcast(raws) => raws,
            other => panic!("a drained two-step ⇒ Broadcast the ordered group, got {other:?}"),
        };
        assert_eq!(raws.len(), 2, "the ordered [tx0, tx1] group is returned");

        let inflight = crate::intent_store::list_in_flight(&aux).expect("inflight");
        assert_eq!(inflight.len(), 1, "exactly one drained in-flight row");
        assert_eq!(
            inflight[0].state,
            crate::intent_store::IntentState::Sent,
            "the row reached Sent (Queued → Submitting → Sent)",
        );
        let group = inflight[0].txids.clone();
        assert_eq!(
            group.len(),
            2,
            "the Sent row carries the ordered [tx0, tx1] group"
        );
        assert!(
            crate::intent_store::list_queued(&aux)
                .expect("queued")
                .is_empty(),
            "no Queued residue — the row is fully Sent",
        );
        // The returned raws ARE the persisted txs, in step order (the durable-before-broadcast read).
        assert_eq!(
            raws[0],
            read_raw_tx(st.wallet(), group[0])
                .expect("tx0")
                .expect("present")
        );
        assert_eq!(
            raws[1],
            read_raw_tx(st.wallet(), group[1])
                .expect("tx1")
                .expect("present")
        );

        // It plugs into the SAME recovery: neither mined ⇒ re-broadcast the whole ordered group.
        let row = inflight[0].clone();
        match reconcile_inflight(st.wallet(), &mut aux, &row, 0).expect("reconcile") {
            Reconciled::Rebroadcast {
                txs: group_raws, ..
            } => {
                assert_eq!(
                    group_raws.len(),
                    2,
                    "both unmined txs re-broadcast, in order"
                );
                assert_eq!(group_raws[0], raws[0], "tx0 first");
                assert_eq!(group_raws[1], raws[1], "tx1 second");
            }
            other => panic!("an unmined drained two-step ⇒ Rebroadcast the group, got {other:?}"),
        }
    }

    #[test]
    fn drain_multi_create_fault_resets_the_row_to_queued() {
        // A create fault spends NO note (the engine persists all-or-nothing), so the row is RESET to
        // Queued and retried next pass — the queued path's self-heal. This DIVERGES from the
        // interactive enrol's DELETE (a queued row IS the durable intent to re-propose, where the
        // synchronous user re-proposes). No Submitting/Sent residue, no broadcast.
        let mut st = funded(NOTE_VALUE);
        let to = tex_recipient([0x0B; 20]);
        let proposal = propose_on(&mut st, request(vec![(to, 20_000)])).expect("TEX proposes");

        let mut aux = aux_conn();
        let id = crate::intent_store::enqueue(&mut aux, "zcash:tex-queued", 0, None, None)
            .expect("enqueue");

        let prepared = drain_multi(st.wallet_mut(), &mut aux, &proposal, id, |_db| {
            Err(WalletError::SignFailed)
        })
        .expect("a create fault is a recoverable Retry, not a hard error");
        assert_eq!(
            prepared,
            Prepared::Retry,
            "a create fault ⇒ Retry next pass"
        );
        assert!(
            crate::intent_store::list_in_flight(&aux)
                .expect("inflight")
                .is_empty(),
            "no Submitting/Sent row survives the create fault",
        );
        assert_eq!(
            crate::intent_store::list_queued(&aux)
                .expect("queued")
                .len(),
            1,
            "the row is reset to Queued (the durable intent to re-propose), never deleted",
        );
    }

    #[test]
    fn drain_multi_parks_a_gap_limit_ceiling_distinctly_not_a_generic_retry() {
        // Round-2 #7: when the create hits the ephemeral gap-limit ceiling (`TexSendLimitReached`),
        // the queued drain RELEASES the Submitting claim back to Queued (the row must drain later,
        // once an in-flight tx0 mines and frees a slot) but returns the DISTINCT `Prepared::Ceiling`,
        // NOT the generic `Retry` — so a transient (or stranding-induced, pt 7(b)) availability
        // ceiling is never read as benign progress. No Submitting/Sent residue, no broadcast. Reachable
        // in production post-gate-removal (2e-2b-v-5); driven here via the injected-create seam (the
        // SAME seam the create-fault test uses) to force the ceiling deterministically.
        let mut st = funded(NOTE_VALUE);
        let to = tex_recipient([0x0D; 20]);
        let proposal = propose_on(&mut st, request(vec![(to, 20_000)])).expect("TEX proposes");

        let mut aux = aux_conn();
        let id = crate::intent_store::enqueue(&mut aux, "zcash:tex-queued", 0, None, None)
            .expect("enqueue");

        let prepared = drain_multi(st.wallet_mut(), &mut aux, &proposal, id, |_db| {
            Err(WalletError::TexSendLimitReached)
        })
        .expect("the ceiling is a recoverable parked outcome, not a hard error");
        assert_eq!(
            prepared,
            Prepared::Ceiling,
            "the gap-limit ceiling ⇒ a distinct Ceiling, never a generic Retry",
        );
        assert!(
            crate::intent_store::list_in_flight(&aux)
                .expect("inflight")
                .is_empty(),
            "no Submitting/Sent row survives the ceiling — the claim was released",
        );
        assert_eq!(
            crate::intent_store::list_queued(&aux)
                .expect("queued")
                .len(),
            1,
            "the row is reset to Queued — it drains once an in-flight tx0 mines and frees a slot",
        );
    }

    // ── §3.2i-2 2e-2b-vi (#315 slice 1): the ephemeral-reserving attempt cap ─────────────────────
    //
    // Every drain attempt of a two-step reserves a fresh engine ephemeral index that is NEVER
    // un-reserved — not on tx0 expiry (`find_gap_start` advances only on a MINED first-use) and
    // not on a create fault (the reservation is committed BEFORE construction). The counter is
    // incremented at the DRAIN funnel (`mark_submitting_counting`, atomic with the transition)
    // so all three leak paths are covered by one count: the burial-gated group requeue, the
    // witness `ReProposeFresh` requeue, and the fast create-fault `Retry` arm — the one the
    // review caught uncounted (a persistent create fault would leak one slot per ~20 s poll pass
    // and brick TEX in minutes). The typed ceiling refusal REFUNDS its count (it provably
    // reserved nothing — red-team F1). The cap gate then parks the intent in `prepare_queued`.

    /// Read one row's attempt counter through the queued-row read the production drain consumes.
    fn queued_attempts(aux: &Connection, id: QueuedSendId) -> i64 {
        crate::intent_store::load(aux, id)
            .expect("load")
            .expect("row present")
            .repropose_attempts
    }

    #[test]
    fn drain_multi_counts_every_ephemeral_reserving_attempt_including_the_fault_paths() {
        // The count-at-the-drain correction: a two-step attempt is tallied when the create
        // FAULTS (the fast leak path — the engine reserved before failing) and when it SUCCEEDS
        // (the row completes and is deleted, so the tally dies with it — a healthy send never
        // accumulates toward a false cap); a CEILING refusal is counted-then-REFUNDED (the typed
        // refusal proves nothing was reserved — red-team F1: a ceiling wait must never
        // consume leak budget, else a healthy TEX behind a transiently-full window pauses in
        // ~1 min instead of self-healing when a live tx0 mines).
        let mut st = funded(NOTE_VALUE);
        let to = tex_recipient([0x0E; 20]);
        let proposal = propose_on(&mut st, request(vec![(to, 20_000)])).expect("TEX proposes");
        let mut aux = aux_conn();
        let id = crate::intent_store::enqueue(&mut aux, "zcash:tex-count", 0, None, None)
            .expect("enqueue");

        // Attempt 1: a create fault. The reservation already happened engine-side — counted.
        let prepared = drain_multi(st.wallet_mut(), &mut aux, &proposal, id, |_db| {
            Err(WalletError::SignFailed)
        })
        .expect("fault is recoverable");
        assert_eq!(prepared, Prepared::Retry);
        assert_eq!(
            queued_attempts(&aux, id),
            1,
            "the create-fault leak path is COUNTED (the S159 blind spot)"
        );

        // A ceiling refusal: counted pessimistically at the door, REFUNDED on the typed refusal —
        // net zero budget consumed, the row waits at the ceiling with its budget intact.
        let prepared = drain_multi(st.wallet_mut(), &mut aux, &proposal, id, |_db| {
            Err(WalletError::TexSendLimitReached)
        })
        .expect("ceiling is recoverable");
        assert_eq!(prepared, Prepared::Ceiling);
        assert_eq!(
            queued_attempts(&aux, id),
            1,
            "a ceiling refusal reserves nothing and consumes NO leak budget (refunded)"
        );

        // Attempt 2 (the success): the tally reaches 2 on the Sent row, then dies with the row's
        // delete-on-buried exit (a completed send never carries budget into a future intent).
        let network = *st.network();
        let usk = st.test_account().expect("acct").usk().clone();
        let prover = crate::prover::tx_prover();
        let prepared = drain_multi(st.wallet_mut(), &mut aux, &proposal, id, |db| {
            Ok(
                create_proposed_transactions::<_, _, Infallible, StandardFeeRule, Infallible, _>(
                    db,
                    &network,
                    prover,
                    prover,
                    &SpendingKeys::from_unified_spending_key(usk),
                    OvkPolicy::Sender,
                    &proposal,
                    // No expiry override — same as production `create_signed_core`.
                    None,
                )
                .expect("two-step create+sign")
                .into_iter()
                .collect(),
            )
        })
        .expect("drain");
        assert!(
            matches!(prepared, Prepared::Broadcast(ref raws) if raws.len() == 2),
            "the next attempt drains normally — the count never blocks below the cap",
        );
        assert_eq!(
            queued_attempts(&aux, id),
            2,
            "the successful attempt is counted on the (now Sent) row; the refunded ceiling is not"
        );
    }

    #[test]
    fn drain_multi_never_counts_a_single_step_attempt() {
        // The `attempts > 0 ⟺ TEX` invariant at the drain: a single-step send reserves NO
        // ephemeral, so even a persistently-faulting one must never consume attempt budget (it
        // retries forever on the generic Retry arm — pre-#315 behavior, unchanged).
        let mut st = funded(NOTE_VALUE);
        let proposal = propose_on(
            &mut st,
            request(vec![(sapling_recipient(b"plain"), 20_000)]),
        )
        .expect("single-step proposes");
        assert_eq!(proposal.steps().len(), 1, "a shielded send is single-step");
        let mut aux = aux_conn();
        let id =
            crate::intent_store::enqueue(&mut aux, "zcash:plain", 0, None, None).expect("enqueue");
        for _ in 0..3 {
            let prepared = drain_multi(st.wallet_mut(), &mut aux, &proposal, id, |_db| {
                Err(WalletError::SignFailed)
            })
            .expect("fault is recoverable");
            assert_eq!(prepared, Prepared::Retry);
        }
        assert_eq!(
            queued_attempts(&aux, id),
            0,
            "a single-step fault burns no TEX attempt budget (it leaked nothing)"
        );
    }

    #[test]
    fn prepare_queued_parks_a_capped_tex_before_proposing_and_keeps_it_actionable() {
        // The cap gate end-to-end over the REAL wiring: reach the cap through actual fault-drains
        // (not a fabricated counter), then prove (a) the next pass PARKS as `CappedParked` WITHOUT
        // proposing — on this funded harness an attempted drain would succeed, so `CappedParked`
        // instead of `Broadcast` is behavioral proof the skip fired first; (b) the parked row
        // surfaces `paused` (the honesty bit); (c) it stays cancellable; (d) the retry affordance
        // re-arms it and the SAME intent then drains to completion.
        let mut st = funded(NOTE_VALUE);
        let to = tex_recipient([0x0F; 20]);
        let proposal = propose_on(&mut st, request(vec![(to.clone(), 20_000)])).expect("proposes");
        let mut aux = aux_conn();
        let created_at = 77;
        // A REAL parseable URI (the classify step below re-parses the stored form, exactly like
        // the production surface does — a placeholder string would fail closed as StoreCorrupt).
        let stored_uri = request(vec![(to.clone(), 20_000)]).to_uri();
        let id = crate::intent_store::enqueue(&mut aux, &stored_uri, created_at, None, None)
            .expect("enqueue");

        // Burn the whole attempt budget on a persistent create fault (the offline/censored or
        // faulting-device shape — each pass leaked an engine slot; the cap stops the bleeding).
        for _ in 0..crate::constants::MAX_TEX_REPROPOSE_ATTEMPTS {
            let prepared = drain_multi(st.wallet_mut(), &mut aux, &proposal, id, |_db| {
                Err(WalletError::SignFailed)
            })
            .expect("fault is recoverable");
            assert_eq!(prepared, Prepared::Retry);
        }
        let attempts = queued_attempts(&aux, id);
        assert_eq!(attempts, crate::constants::MAX_TEX_REPROPOSE_ATTEMPTS);

        // (a) The next pass parks BEFORE propose/create — no Broadcast despite full funding.
        let network = *st.network();
        let aid = st.test_account().expect("acct").id();
        let usk = st.test_account().expect("acct").usk().clone();
        let prover = crate::prover::tx_prover();
        let prepared = prepare_queued(
            st.wallet_mut(),
            &network,
            &mut aux,
            aid,
            request(vec![(to.clone(), 20_000)]),
            usk,
            prover,
            prover,
            id,
            None,
            attempts,
            0,
            crate::consensus::SigningPermit::assume_current_for_test(),
        )
        .expect("prepare");
        assert_eq!(
            prepared,
            Prepared::CappedParked,
            "at the cap the drain parks instead of leaking a fourth slot",
        );
        let queued = crate::intent_store::list_queued(&aux).expect("q");
        assert_eq!(
            queued.len(),
            1,
            "the capped row STAYS Queued (parked, not dropped)"
        );
        assert_eq!(
            queued_attempts(&aux, id),
            attempts,
            "a parked pass consumes no attempt budget (no mark_submitting ran)"
        );

        // (b) The parked surface tells the truth: paused, distinguishable from a healthy row.
        let parked = crate::parked::classify(&queued[0], false, None)
            .expect("classifies")
            .expect("surfaces");
        assert!(parked.paused, "a capped row surfaces paused = true");

        // (c) Still cancellable: the park leaves the row exactly `Queued` (asserted above) —
        // precisely the state the v-4a `cancel_queued` guard accepts (pinned by its own tests),
        // so the cap can never trap a send the user wants gone.
        // (d) The retry affordance re-arms the SAME intent; the next pass drains it fully.
        assert!(
            crate::intent_store::reset_repropose_attempts(&mut aux, id, created_at).expect("retry"),
            "the retry affordance re-arms the capped row",
        );
        let usk2 = st.test_account().expect("acct").usk().clone();
        let rearmed_attempts = queued_attempts(&aux, id);
        assert_eq!(rearmed_attempts, 0, "retry reset the budget");
        let prepared = prepare_queued(
            st.wallet_mut(),
            &network,
            &mut aux,
            aid,
            request(vec![(to, 20_000)]),
            usk2,
            prover,
            prover,
            id,
            None,
            rearmed_attempts,
            0,
            crate::consensus::SigningPermit::assume_current_for_test(),
        )
        .expect("prepare after retry");
        assert!(
            matches!(prepared, Prepared::Broadcast(ref raws) if raws.len() == 2),
            "after retry the SAME intent drains to a real [tx0, tx1] group, got {prepared:?}",
        );
    }

    #[test]
    fn prepare_queued_deposit_gate_precedes_the_attempt_cap() {
        // Gate ORDER is load-bearing: a swap deposit is EXCLUDED from the parked
        // surface, so a capped deposit that skipped the deadline check would orphan INVISIBLY
        // forever. The deposit gate runs FIRST: a lapsed capped deposit self-expires (deleted,
        // terminal) — never `CappedParked`.
        let mut st = funded(NOTE_VALUE);
        let mut aux = aux_conn();
        let id = crate::intent_store::enqueue(&mut aux, "zcash:dep-cap", 0, Some(100), None)
            .expect("enqueue");
        let network = *st.network();
        let aid = st.test_account().expect("acct").id();
        let usk = st.test_account().expect("acct").usk().clone();
        let prover = crate::prover::tx_prover();
        let prepared = prepare_queued(
            st.wallet_mut(),
            &network,
            &mut aux,
            aid,
            request(vec![(tex_recipient([0x10; 20]), 20_000)]),
            usk,
            prover,
            prover,
            id,
            Some(100),                                    // quote deadline long past…
            crate::constants::MAX_TEX_REPROPOSE_ATTEMPTS, // …on a row that is ALSO capped
            2_000_000_000, // trustworthy (post-plausibility-floor) clock, far past the deadline,
            crate::consensus::SigningPermit::assume_current_for_test(),
        )
        .expect("prepare");
        assert_eq!(
            prepared,
            Prepared::DepositExpired,
            "the deadline wins: a lapsed capped deposit self-expires, never parks invisibly",
        );
        assert!(
            crate::intent_store::load(&aux, id).expect("load").is_none(),
            "the lapsed deposit row is deleted (terminal — the user re-quotes)",
        );
    }

    #[test]
    fn interactive_enrol_counts_the_ephemeral_attempt_for_crash_recovery() {
        // The interactive two-step enrol is ALSO an ephemeral-reserving create, so it counts via
        // the same atomic door. On the synchronous paths the tally is moot (success → the row
        // completes; fault → the row is deleted), but a process KILL between mark_submitting and
        // the create leaves a Submitting row that recovers into the QUEUED drain — which must
        // inherit attempts = 1 (the reservation may already be consumed), not a fresh 0.
        let mut st = funded(NOTE_VALUE);
        let proposal = propose_on(&mut st, request(vec![(tex_recipient([0x11; 20]), 20_000)]))
            .expect("TEX proposes");
        let mut aux = aux_conn();
        let txids =
            create_two_step_enrolled(&mut aux, &proposal, "zcash:tex-count", 7, None, || {
                Ok(vec![
                    TxId::from_bytes([0xA0; 32]),
                    TxId::from_bytes([0xB1; 32]),
                ])
            })
            .expect("enrol succeeds");
        assert_eq!(txids.len(), 2);
        let row = &crate::intent_store::list_in_flight(&aux).expect("inflight")[0];
        assert_eq!(
            queued_attempts(&aux, row.id),
            1,
            "the interactive ephemeral-reserving attempt is tallied (crash-recovery honesty)"
        );
    }

    #[test]
    fn tex_create_hits_the_typed_gap_limit_ceiling_when_the_ephemeral_window_is_full() {
        // Round-2 #7 (the LAST recovery hardening), proven over the REAL engine. A ZIP-320 two-step
        // reserves one engine-owned ephemeral t-address per tx0, bounded by the ephemeral gap limit.
        // With the gap shrunk to 1, the FIRST TEX create reserves the only slot; a SECOND TEX create
        // — while the first tx0 is still UNMINED — exceeds the gap and the engine returns
        // `ReachedGapLimit`. It MUST surface as the typed `TexSendLimitReached` (RW-SEND-007), so a
        // host renders "wait for transfers to confirm / recover stranded ones" rather than dead-ending
        // the user with a generic StoreCorrupt/SignFailed. Two SEPARATE spendable notes fund the two
        // sends (the second send's notes are independent of the first send's spend).
        //
        // It also EMPIRICALLY PINS the §3.2i-2 pt 7(a) reservation semantic: the ephemeral gap
        // advances on ON-CHAIN OBSERVATION, not at create — a run of created-but-unmined tx0s fills
        // it. Were the engine to advance at create instead, the second create would SUCCEED; this
        // test is the guard that the documented "offline tx0s brick the next TEX send" hazard is real.
        let mut st = funded_multi_with_ephemeral_gap(&[100_000, 100_000], 1);

        // First TEX send: a real two-step create reserves ephemeral index 0 (tx0 left UNMINED).
        let p0 = propose_on(&mut st, request(vec![(tex_recipient([0x21; 20]), 20_000)]))
            .expect("first TEX proposes");
        assert_eq!(p0.steps().len(), 2, "a TEX send is a two-step");
        let first = try_sign_two_step_on(&mut st, &p0).expect("first TEX create reserves the slot");
        assert_eq!(first.len(), 2, "the engine created [tx0, tx1]");

        // Second TEX send: PROPOSE still succeeds (the reservation is create-time, not propose-time —
        // pt 7(a)), but the CREATE exceeds the now-full ephemeral window.
        let p1 = propose_on(&mut st, request(vec![(tex_recipient([0x22; 20]), 20_000)])).expect(
            "second TEX still PROPOSES — the ceiling is a create-time reservation, not propose",
        );
        assert_eq!(p1.steps().len(), 2, "still a two-step");
        let err = try_sign_two_step_on(&mut st, &p1)
            .expect_err("the second create exceeds the ephemeral gap limit");
        assert!(
            matches!(err, WalletError::TexSendLimitReached),
            "a real ReachedGapLimit folds to the typed ceiling (RW-SEND-007), got {err:?}",
        );
    }

    #[test]
    fn a_gap_limit_refusal_reserves_nothing_the_ceiling_refund_coupling() {
        // review A4/H6 — COUPLING PIN for the slice-1 ceiling REFUND. On a `TexSendLimitReached`
        // refusal the drain calls `reset_to_queued_refunding_attempt`, REFUNDING the pessimistic
        // re-propose count on the belief that the refusal proves NOTHING was reserved. That holds ONLY
        // because today's engine raises `ReachedGapLimit` from the PRE-reservation gap check. Pin it
        // against the REAL engine: the reserved-ephemeral count is UNCHANGED across the failing create.
        // Were a future engine to reserve-THEN-validate (take a slot, then hit the limit), this count
        // would grow, the refund would UNDER-count the leak, and the cap would stop firing — the exact
        // silent-brick the slice-1 cap exists to prevent. If this ever fails, re-audit the refund.
        let mut st = funded_multi_with_ephemeral_gap(&[100_000, 100_000], 1);
        let aid = st.test_account().expect("acct").id();
        let reserved_count = |st: &HarnessState| {
            let tip = u32::from(st.wallet().chain_height().expect("tip read").expect("tip"));
            st.wallet()
                .get_ephemeral_transparent_receivers(aid, tip, false)
                .expect("enumerate reserved ephemerals")
                .len()
        };

        // First TEX reserves the only ephemeral slot (gap = 1); its tx0 is left unmined.
        let p0 = propose_on(&mut st, request(vec![(tex_recipient([0x41; 20]), 20_000)]))
            .expect("first TEX proposes");
        try_sign_two_step_on(&mut st, &p0).expect("first TEX reserves index 0");
        let before = reserved_count(&st);
        assert_eq!(before, 1, "the first TEX reserved exactly one ephemeral");

        // Second TEX create trips the now-full window ⇒ the typed ceiling.
        let p1 = propose_on(&mut st, request(vec![(tex_recipient([0x42; 20]), 20_000)]))
            .expect("second TEX still proposes (the ceiling is create-time)");
        assert!(
            matches!(
                try_sign_two_step_on(&mut st, &p1),
                Err(WalletError::TexSendLimitReached)
            ),
            "the second create exceeds the ephemeral gap limit",
        );

        // THE INVARIANT: the refusal reserved NOTHING — so the ceiling refund is honest.
        let after = reserved_count(&st);
        assert_eq!(
            after, before,
            "a TexSendLimitReached refusal must reserve NOTHING (the ceiling-refund coupling): the \
             engine raises ReachedGapLimit from the PRE-reservation gap check",
        );
    }

    #[test]
    fn self_mint_on_the_highest_leaked_ephemeral_reopens_the_whole_window() {
        // ══ THE #315 SLICE-2 MECHANISM-A PROOF (§3.2i-2 2e-2b-vi VECTORS-FIRST GATE) ══
        // The reclaim's load-bearing engine claim, proven over the REAL engine BEFORE any
        // money-moving reclaim code is trusted: the engine never un-reserves a leaked ephemeral
        // (pinned by the ceiling test above), but `find_gap_start` advances past ANY mined
        // first-use — including one WE create. So paying a tiny amount from the wallet's OWN
        // shielded pool to the HIGHEST leaked ephemeral address via a NORMAL single-step send
        // (no TEX, no new reservation), and MINING it, must reopen the ENTIRE reservation
        // window in ONE transaction — the lower leaks fall below `gap_start` as harmless dead
        // indices. This test is the gate: if it fails, slice 2 falls back to Mechanism C
        // (accept the ceiling; only the upstream FR + honest UX ship).
        //
        // Also guards the two review-flagged wrinkles: (a) proposing a payment to an address the
        // wallet KNOWS is its own ephemeral must lower as a plain SINGLE-step (no reservation,
        // no reuse-check trip — the leaked address never received on-chain); (b) the CREATE of
        // the self-mint alone must NOT reopen the window (the gap advances on ON-CHAIN
        // OBSERVATION, mirroring the ceiling test's create-time pin) — only the MINE+SCAN does.
        //
        // Success is the reservation PROBE (a real TEX create), NOT the pressure gauge — the
        // final pin shows the gauge still counting the dead lower leaks (design (c): keying a
        // "Recover now" CTA on the gauge would drive repeat fee burn).
        //
        // Gap shrunk to 3 (the ceiling lever) so the proof covers a MULTI-leak window cheaply:
        // 3 leaked indices {0,1,2}, mint the HIGHEST (2), window [3,6) reopens.
        let mut st = funded_multi_with_ephemeral_gap(&[100_000; 5], 3);

        // (1) LEAK the whole window: three real TEX creates whose tx0s are never broadcast nor
        //     mined (the offline/censored shape). Each reserves one ephemeral index engine-side.
        for tag in [0x31u8, 0x32, 0x33] {
            let p = propose_on(&mut st, request(vec![(tex_recipient([tag; 20]), 20_000)]))
                .expect("TEX proposes");
            try_sign_two_step_on(&mut st, &p).expect("TEX create reserves an ephemeral index");
        }

        // (2) CONTROL — the window is genuinely bricked: a fourth TEX create refuses. (Its
        //     create fault consumes NO note, so the remaining notes fund the mint + the probe.)
        let p_bricked = propose_on(&mut st, request(vec![(tex_recipient([0x34; 20]), 20_000)]))
            .expect("still PROPOSES — the ceiling is create-time");
        assert!(
            matches!(
                try_sign_two_step_on(&mut st, &p_bricked),
                Err(WalletError::TexSendLimitReached)
            ),
            "control: the window is full — TEX sends are bricked before the reclaim",
        );

        // (3) Enumerate ALL reserved ephemerals over the WIDE window (the sweep's enumeration
        //     idiom — exposure_depth = tip, NOT the bounded detect lookback) and pick the
        //     HIGHEST index: exactly what the slice-2 reclaim will do. Driven at the RAW engine
        //     API (`get_ephemeral_transparent_receivers`, the same read the production
        //     `account::all_reserved_ephemeral_addresses` wraps — the wrappers are
        //     production-`Network`-typed and the harness is `LocalNetwork` by construction).
        let aid = st.test_account().expect("acct").id();
        let tip = u32::from(
            st.wallet()
                .chain_height()
                .expect("tip read")
                .expect("synced harness has a tip"),
        );
        let reserved: Vec<(
            TransparentAddress,
            zcash_client_backend::wallet::TransparentAddressMetadata,
        )> = st
            .wallet()
            .get_ephemeral_transparent_receivers(aid, tip, false)
            .expect("enumerate reserved ephemerals")
            .into_iter()
            .collect();
        assert_eq!(reserved.len(), 3, "all three leaked reservations enumerate");
        let (highest_addr, highest_meta) = reserved
            .iter()
            .max_by_key(|(_, m)| m.address_index().map(|i| i.index()))
            .expect("non-empty");
        assert_eq!(
            highest_meta.address_index().map(|i| i.index()),
            Some(2),
            "the highest leaked index is 2 (indices 0..3 reserved in order)"
        );

        // (4) SELF-MINT: a NORMAL send from the wallet's own shielded pool to the highest
        //     leaked ephemeral address. Wrinkle (a): it must propose + sign as a plain
        //     SINGLE-step — reserving nothing and never tripping the reuse check.
        let TransparentAddress::PublicKeyHash(hash) = highest_addr else {
            panic!("an engine ephemeral is always P2PKH");
        };
        let mint_recipient = ZcashAddress::from_transparent_p2pkh(NetworkType::Regtest, *hash);
        let p_mint = propose_on(&mut st, request(vec![(mint_recipient, 20_000)]))
            .expect("a normal send to the wallet's own leaked ephemeral PROPOSES");
        assert_eq!(
            p_mint.steps().len(),
            1,
            "wrinkle (a): the self-mint lowers as a SINGLE-step (no reservation)",
        );
        let mint = try_sign_two_step_on(&mut st, &p_mint)
            .expect("the self-mint signs — the reuse check passes (never received on-chain)");
        assert_eq!(mint.len(), 1, "one plain tx");

        // Wrinkle (b): CREATING the self-mint alone reopens NOTHING — the gap advances on
        // on-chain observation, not on create/persist.
        let p_still = propose_on(&mut st, request(vec![(tex_recipient([0x35; 20]), 20_000)]))
            .expect("proposes");
        assert!(
            matches!(
                try_sign_two_step_on(&mut st, &p_still),
                Err(WalletError::TexSendLimitReached)
            ),
            "wrinkle (b): an unmined self-mint frees nothing — mining is the mechanism",
        );

        // (5) MINE + SCAN the self-mint: the on-chain first-use of index 2.
        let (mined_h, _) = st.generate_next_block_including(mint[0]);
        st.scan_cached_blocks(mined_h, 1);
        st.wallet_mut()
            .update_chain_tip(mined_h)
            .expect("update tip");

        // (6) THE PROOF — the reservation PROBE: one mined self-mint on the HIGHEST index
        //     reopens the WHOLE window (gap_start advances past indices 0..2; [3,6) is fresh).
        let p_after = propose_on(&mut st, request(vec![(tex_recipient([0x36; 20]), 20_000)]))
            .expect("proposes");
        try_sign_two_step_on(&mut st, &p_after).expect(
            "MECHANISM A: one mined self-mint on the highest leaked index reopens the window",
        );

        // (7) Design (c) pin, STRENGTHENED by a proof-round DISCOVERY: the pressure GAUGE is
        //     structurally BLIND here, not just imprecise. The `exclude_used = true` read (what
        //     `account::outstanding_ephemeral_reservations` wraps) filters on the EXISTENCE of a
        //     `transparent_received_outputs` row — and the engine writes that row for the
        //     wallet's OWN tx0 at CREATE-PERSIST time, unmined (`store_transactions_to_be_sent`,
        //     `Recipient::EphemeralTransparent` arm, pinned source wallet.rs:3486). So EVERY
        //     created-then-leaked reservation reads "used" and the gauge counts ZERO while the
        //     window is fully bricked (it counts only create-FAULT leaks, where no tx persisted).
        //     Keying ANY reclaim decision or success signal on the gauge would be doubly wrong;
        //     the reservation PROBE above is the only truth. (The gauge's own docs/FR fallout is
        //     recorded in the spec's slice-2 proof note.)
        let tip_after = u32::from(st.wallet().chain_height().expect("tip read").expect("tip"));
        let outstanding = st
            .wallet()
            .get_ephemeral_transparent_receivers(aid, tip_after, true)
            .expect("gauge read")
            .len();
        assert_eq!(
            outstanding, 0,
            "DISCOVERY pin: the exclude_used gauge is blind to created-tx0 leaks (their tro rows \
             exist from create-persist) — it reads 0 with two dead leaks + a live reservation \
             outstanding; never key reclaim success (or the ceiling warn) on it",
        );
    }

    #[test]
    fn prepare_queued_drains_a_tex_two_step_after_gate_removal() {
        // GATE-REMOVAL PROOF (2e-2b-v-5): a queued TEX (queue_send accepts a TEX URI) now DRAINS
        // through `prepare_queued` → `drain_multi` exactly like a single-step send — the slice-A park
        // gate (the old `Prepared::Blocked` terminal) is gone. The drain signs the `[tx0, tx1]` chain
        // in the load-bearing money order (`mark_submitting` BEFORE create, `mark_sent_multi` BEFORE
        // broadcast) and hands the ordered 2-tx group up for broadcast.
        let mut st = funded(NOTE_VALUE);
        let mut aux = aux_conn();
        let id = crate::intent_store::enqueue(&mut aux, "zcash:tex-queued", 0, None, None)
            .expect("enqueue");
        // The stored URI re-parses to a real TEX request (what queue_send_uri persists + drains).
        let req = request(vec![(tex_recipient([0x0C; 20]), 20_000)]);
        let network = *st.network();
        let aid = st.test_account().expect("acct").id();
        let usk = st.test_account().expect("acct").usk().clone();
        let prover = crate::prover::tx_prover();
        let prepared = prepare_queued(
            st.wallet_mut(),
            &network,
            &mut aux,
            aid,
            req,
            usk,
            prover,
            prover,
            id,
            None,
            0,
            0,
            crate::consensus::SigningPermit::assume_current_for_test(),
        )
        .expect("prepare");
        match prepared {
            Prepared::Broadcast(group) => {
                assert_eq!(
                    group.len(),
                    2,
                    "the queued TEX two-step drains as [tx0, tx1]"
                );
            }
            other => panic!("a queued TEX now drains, got {other:?}"),
        }
        // The row left Queued for the in-flight outbox: `mark_sent_multi` recorded the group BEFORE
        // broadcast, so every broadcast tx has a recorded txid the delete-on-mined recovery finds.
        assert!(
            crate::intent_store::list_queued(&aux)
                .expect("queued")
                .is_empty(),
            "the drained TEX is no longer Queued",
        );
        assert_eq!(
            crate::intent_store::list_in_flight(&aux)
                .expect("inflight")
                .len(),
            1,
            "the drained TEX is in the in-flight outbox (claimed + sent, awaiting mine)",
        );
    }

    #[test]
    fn prepare_queued_drains_a_multi_recipient_tex_two_step() {
        // GATE-REMOVAL MONEY-SAFETY (the v-5a review fix): the queued path NEVER passes through
        // `summarize`, so it carries its OWN fail-closed shape guard. That guard must reject ONLY a
        // NON-ZIP-320 multi-step (the structural surprise) and never OVER-reject a legitimate one. The
        // engine BATCHES multiple TEX recipients into ONE ZIP-320 two-step (step0 unshields to a single
        // ephemeral; step1 pays ALL the TEX destinations from it — `is_zip320_two_step` recognises it
        // because step1's payments are all transparent), so a multi-recipient TEX is the recognised
        // shape and MUST drain through the guard, not park as `Unsupported`. (A non-ZIP-320 multi-step
        // is unreachable from today's engine — the `Unsupported` arm is defensive, mirroring the twin
        // guard in `summarize`/`create_signed_core`.)
        let mut st = funded(500_000); // enough for two TEX legs + the pair's fees
        let mut aux = aux_conn();
        let two_tex = request(vec![
            (tex_recipient([0x05; 20]), 20_000),
            (tex_recipient([0x06; 20]), 20_000),
        ]);
        // Confirm the engine produced the RECOGNISED two-step (the guard's positive branch).
        let proposal = propose_on(&mut st, two_tex.clone()).expect("two-TEX proposes");
        assert_eq!(
            proposal.steps().len(),
            2,
            "two TEX recipients batch into one ephemeral pair"
        );
        assert!(
            is_zip320_two_step(&proposal).is_some(),
            "...the recognised ZIP-320 [tx0, tx1] two-step (step1 pays both TEX destinations)",
        );

        let id = crate::intent_store::enqueue(&mut aux, "zcash:two-tex", 0, None, None)
            .expect("enqueue");
        let network = *st.network();
        let aid = st.test_account().expect("acct").id();
        let usk = st.test_account().expect("acct").usk().clone();
        let prover = crate::prover::tx_prover();
        let prepared = prepare_queued(
            st.wallet_mut(),
            &network,
            &mut aux,
            aid,
            two_tex,
            usk,
            prover,
            prover,
            id,
            None,
            0,
            0,
            crate::consensus::SigningPermit::assume_current_for_test(),
        )
        .expect("prepare");
        match prepared {
            Prepared::Broadcast(group) => {
                assert_eq!(
                    group.len(),
                    2,
                    "the multi-recipient TEX drains as [tx0, tx1]"
                );
            }
            other => panic!("a recognised multi-recipient TEX must drain, not park: {other:?}"),
        }
        assert!(
            crate::intent_store::list_queued(&aux)
                .expect("queued")
                .is_empty(),
            "the drained multi-recipient TEX is no longer Queued",
        );
        assert_eq!(
            crate::intent_store::list_in_flight(&aux)
                .expect("inflight")
                .len(),
            1,
            "the drained multi-recipient TEX is in the in-flight outbox",
        );
    }

    // ── F01 (the 2026-10-05 review, plan §2.2): start vs continue in the drain ──
    // A two-step deposit, 150 s before its deadline: inside the 240 s start margin, outside the
    // 90 s continue margin. Whether tx1 goes depends on tx0 being on the network.

    /// The F01 fixtures' one deadline, shared with `wallet::tests` (the drain, the kick
    /// and the per-leg check read the same scenario).
    pub(crate) const F01_DEADLINE: i64 = 1_900_000_000;
    const F01_NOW: u64 = (F01_DEADLINE - 150) as u64;

    #[test]
    fn the_drain_sends_tx1_after_an_accepted_unmined_tx0() {
        // Pass 3's case: tx0 accepted by an endpoint (marked) but not mined, the app restarted,
        // and the drain hands up [tx0, tx1]. Both legs continue, so neither is held.
        let mut st = funded(NOTE_VALUE);
        let (mut aux, id, group) = tex_two_step_sent_row(&mut st);
        crate::delivery::ensure_table(&aux).expect("accepted table");
        crate::delivery::mark_accepted(&aux, &group[0], None, 1).expect("mark tx0");
        let action = rebroadcast_group(
            st.wallet(),
            &mut aux,
            id,
            &group,
            Some(F01_DEADLINE),
            F01_NOW,
        )
        .expect("reconcile");
        match action {
            Reconciled::Rebroadcast { txs, continues } => {
                assert_eq!(txs.len(), 2, "both unmined legs go up");
                assert_eq!(continues, vec![true, true], "tx0 marked ⇒ both continue");
            }
            other => panic!("an accepted tx0 must not hold tx1, got {other:?}"),
        }
    }

    #[test]
    fn the_drain_continues_after_a_mined_tx0() {
        let mut st = funded(NOTE_VALUE);
        let (mut aux, id, group) = tex_two_step_sent_row(&mut st);
        crate::delivery::ensure_table(&aux).expect("accepted table");
        let (mined_h, _) = st.generate_next_block_including(TxId::from_bytes(group[0]));
        st.scan_cached_blocks(mined_h, 1);
        st.wallet_mut().update_chain_tip(mined_h).expect("tip");
        let action = rebroadcast_group(
            st.wallet(),
            &mut aux,
            id,
            &group,
            Some(F01_DEADLINE),
            F01_NOW,
        )
        .expect("reconcile");
        match action {
            Reconciled::Rebroadcast { txs, continues } => {
                assert_eq!(txs.len(), 1, "only the unmined tx1");
                assert_eq!(continues, vec![true], "its predecessor is mined");
            }
            other => panic!("a mined tx0 must not hold tx1 inside the start margin, got {other:?}"),
        }
    }

    #[test]
    fn the_drain_holds_a_two_step_deposit_with_nothing_on_the_network() {
        // The control: neither leg mined nor marked ⇒ a START, and inside the start margin it holds.
        let mut st = funded(NOTE_VALUE);
        let (mut aux, id, group) = tex_two_step_sent_row(&mut st);
        crate::delivery::ensure_table(&aux).expect("accepted table");
        let action = rebroadcast_group(
            st.wallet(),
            &mut aux,
            id,
            &group,
            Some(F01_DEADLINE),
            F01_NOW,
        )
        .expect("reconcile");
        assert_eq!(action, Reconciled::DepositBroadcastHold);
    }

    #[test]
    fn tex_two_step_rebroadcasts_only_the_unmined_tx1_after_tx0_mines() {
        // The skip-mined recovery (§3.2i-2 pt 4): tx0 mined (the shielded notes left irreversibly)
        // but tx1 missed ⇒ re-broadcast ONLY tx1; never re-submit the mined tx0 (whose
        // already-known reject would, under a latching broadcaster, wrongly skip tx1). tx1 stays
        // unexpired, so no strand yet — the transient miss the recovery exists to heal.
        let mut st = funded(NOTE_VALUE);
        let (mut aux, _id, group) = tex_two_step_sent_row(&mut st);
        let (mined_h, _) = st.generate_next_block_including(TxId::from_bytes(group[0]));
        st.scan_cached_blocks(mined_h, 1);
        st.wallet_mut()
            .update_chain_tip(mined_h)
            .expect("tip past the mined tx0");

        let row = crate::intent_store::list_in_flight(&aux).expect("inflight")[0].clone();
        let action = reconcile_inflight(st.wallet(), &mut aux, &row, 0).expect("reconcile");
        let raws = match action {
            Reconciled::Rebroadcast { txs: raws, .. } => raws,
            other => panic!("tx0 mined / tx1 unmined ⇒ Rebroadcast only tx1, got {other:?}"),
        };
        assert_eq!(
            raws.len(),
            1,
            "the mined tx0 is SKIPPED — only tx1 re-broadcasts"
        );
        assert_eq!(
            raws[0],
            read_raw_tx(st.wallet(), group[1])
                .expect("tx1")
                .expect("present"),
            "and it is tx1 (the unmined tail)",
        );
        assert_eq!(
            crate::intent_store::list_in_flight(&aux)
                .expect("still")
                .len(),
            1,
            "the row stays Sent (tx1 has not landed)",
        );
    }

    // delete-on-FINAL for the TEX tx1 (both txs mined ⇒ delete) is DEVICE-GATED: the
    // `data_api::testing` harness cannot observe the FINAL tx as mined — tx1 pays an EXTERNAL TEX
    // p2pkh with no wallet-relevant output, and the harness does not index the scope-2 ephemeral
    // UTXO spend that would mark it mined, so `get_tx_height(tx1)` stays `None` here (the funded
    // testnet `textest1…` proof in the e2e checklist covers the real tx0+tx1 mine). The delete-on-
    // FINAL code path IS covered structurally: `rebroadcast_group` deletes on `group.last()` mined
    // AND buried beyond `REORG_MAX_BLOCKS` (round-2 #6), exercised by
    // `resubmit_reconcile_waits_for_burial_then_deletes_a_mined_sent_intent` (a one-tx group, final
    // == only — shallow→AwaitingBurial, buried→Deleted), and the "NEVER delete on tx0-mined" half
    // (delete keys on the FINAL, not tx0) is proven by
    // `tex_two_step_rebroadcasts_only_the_unmined_tx1_after_tx0_mines` above.

    #[test]
    fn tex_two_step_strands_when_tx0_mined_and_tx1_expired() {
        // ROUND-2 BLOCKER #1 — the recovery state machine MUST terminate. tx0 mined (the shielded
        // notes left IRREVERSIBLY) but tx1 then EXPIRED un-mined ⇒ re-broadcast can never complete
        // it (an expired tx can never enter the mempool; re-propose is structurally impossible
        // post-tx0). Without this, reconcile would re-broadcast a DEAD tx1 forever. The fix: move
        // the row to terminal `Stranded` (leave the reconcile loop) and hand the ephemeral funds to
        // the 2e-2b detect — always forward progress, never a silent busy-loop.
        let mut st = funded(NOTE_VALUE);
        let (mut aux, _id, group) = tex_two_step_sent_row(&mut st);
        let tx1_expiry = created_tx_expiry(&st, TxId::from_bytes(group[1]));
        // Mine tx0 only (the irreversible step).
        let (h0, _) = st.generate_next_block_including(TxId::from_bytes(group[0]));
        st.scan_cached_blocks(h0, 1);

        // ONE BLOCK BEFORE expiry (`tip = expiry - 1`): the next block IS `expiry`, still within
        // tx1's validity window, so tx1 can still mine ⇒ re-broadcast it, do NOT strand. This pins
        // the corrected `expiry <= tip` boundary against a premature strand (review off-by-one).
        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(tx1_expiry - 1))
            .expect("tip one block before tx1's expiry");
        let row = crate::intent_store::list_in_flight(&aux).expect("inflight")[0].clone();
        let pre = reconcile_inflight(st.wallet(), &mut aux, &row, 0).expect("reconcile");
        assert!(
            matches!(pre, Reconciled::Rebroadcast { txs: ref r, .. } if r.len() == 1),
            "one block before tx1's expiry it is still mineable ⇒ re-broadcast tx1, got {pre:?}",
        );
        assert_eq!(
            crate::intent_store::list_in_flight(&aux)
                .expect("still")
                .len(),
            1,
            "still Sent — not stranded a block early",
        );

        // tx1 DEAD but tx0 still SHALLOW (round-2 #6): at `tip = expiry` the next block `tip+1 >
        // expiry`, so tx1 can NEVER enter a block — yet tx0's mine is still reorg-reversible (its
        // standard ~40-block expiry window is well within `REORG_MAX_BLOCKS = 100`). Stranding here
        // would be a FALSE terminal: a deep reorg could un-mine tx0, returning the notes to spendable
        // for a fresh re-propose. So WAIT (Awaiting) for tx0 to bury before committing the terminal.
        assert!(
            tx1_expiry < u32::from(h0) + REORG_MAX_BLOCKS,
            "test precondition: tx1's expiry must fall within tx0's burial window so the strand WAITS",
        );
        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(tx1_expiry))
            .expect("tip exactly at tx1's expiry — tx1 dead, tx0 still shallow");
        let row = crate::intent_store::list_in_flight(&aux).expect("inflight")[0].clone();
        let pending = reconcile_inflight(st.wallet(), &mut aux, &row, 0).expect("reconcile");
        assert_eq!(
            pending,
            Reconciled::AwaitingBurial,
            "tx1 dead but tx0 not yet buried ⇒ WAIT (a reorg could still un-mine tx0 and recover)",
        );
        assert_eq!(
            crate::intent_store::list_in_flight(&aux)
                .expect("still")
                .len(),
            1,
            "the row stays Sent while tx0 buries — not a premature terminal",
        );

        // tx0 BURIED + tx1 DEAD (`tip = h0 + REORG_MAX_BLOCKS`, also well past tx1's expiry): the
        // irreversible step can no longer reorg out AND tx1 can never mine ⇒ terminal Stranded (leave
        // the reconcile loop, hand the ephemeral funds to the 2e-2b detect).
        let buried_tip = BlockHeight::from_u32(u32::from(h0) + REORG_MAX_BLOCKS);
        st.wallet_mut()
            .update_chain_tip(buried_tip)
            .expect("tip a full reorg-depth past tx0 — tx0 now buried, tx1 long dead");
        let row = crate::intent_store::list_in_flight(&aux).expect("inflight")[0].clone();
        let action = reconcile_inflight(st.wallet(), &mut aux, &row, 0).expect("reconcile");
        assert_eq!(
            action,
            Reconciled::Stranded,
            "tx0 mined+buried + tx1 dead ⇒ terminal Stranded, never a doomed re-broadcast",
        );
        assert!(
            crate::intent_store::list_in_flight(&aux)
                .expect("left loop")
                .is_empty(),
            "the stranded row LEAVES the reconcile loop (it can never busy-loop a dead tx)",
        );
        assert!(
            crate::intent_store::list_queued(&aux)
                .expect("not requeued")
                .is_empty(),
            "and does NOT re-queue (re-propose is impossible once tx0 spent the notes)",
        );
    }

    #[test]
    fn tex_two_step_strands_when_tx0_mined_and_the_final_tx_is_gone_from_the_store() {
        // TERMINATION GUARANTEE (review HARDENING): if the final tx is GONE from the store
        // (pruned/collected) while an earlier tx mined, it can never be re-broadcast AND can never
        // mine — the send is stuck. The machine must STILL terminate to `Stranded`, never loop
        // `Awaiting` forever. Simulate the pruned tx1 with a txid absent from the store, paired with
        // a REAL mined tx0 (so the step0 claim genuinely reads spent ⇒ the ReBroadcast regime).
        let mut st = funded(NOTE_VALUE);
        let to = tex_recipient([0x04; 20]);
        let proposal = propose_on(&mut st, request(vec![(to, 20_000)])).expect("TEX proposes");
        let claims = claim_from_proposal(&proposal).expect("step0 claim");
        let txids = sign_two_step_on(&mut st, &proposal);
        let tx0: [u8; 32] = *txids[0].as_ref();
        let absent_tx1 = [0xEE; 32]; // never persisted — stands in for a pruned/collected final tx
        let (h0, _) = st.generate_next_block_including(txids[0]);
        st.scan_cached_blocks(h0, 1);

        let mut aux = Connection::open_in_memory().expect("aux");
        crate::intent_store::ensure_table(&aux).expect("table");
        let id = crate::intent_store::enqueue(&mut aux, "zcash:tex-pruned", 0, None, None)
            .expect("enqueue");
        crate::intent_store::mark_submitting(&mut aux, id, &claims).expect("submit");
        crate::intent_store::mark_sent_multi(&mut aux, id, &[tx0, absent_tx1]).expect("sent group");

        // tx0 still SHALLOW (round-2 #6): even with the final tx un-broadcastable, do not strand
        // while tx0 could still reorg out and return the notes to spendable — WAIT.
        st.wallet_mut()
            .update_chain_tip(h0)
            .expect("tip at tx0 (shallow)");
        let row = crate::intent_store::list_in_flight(&aux).expect("inflight")[0].clone();
        let shallow = reconcile_inflight(st.wallet(), &mut aux, &row, 0).expect("reconcile");
        assert_eq!(
            shallow,
            Reconciled::AwaitingBurial,
            "final tx absent but tx0 shallow ⇒ WAIT for burial, not a premature terminal",
        );

        // tx0 BURIED + final tx absent ⇒ it can never be re-broadcast AND can never mine, and the
        // irreversible step is now permanent ⇒ terminal `Stranded`, never `Awaiting`-forever.
        let buried_tip = BlockHeight::from_u32(u32::from(h0) + REORG_MAX_BLOCKS);
        st.wallet_mut()
            .update_chain_tip(buried_tip)
            .expect("tip a reorg-depth past tx0");
        let row = crate::intent_store::list_in_flight(&aux).expect("inflight")[0].clone();
        let action = reconcile_inflight(st.wallet(), &mut aux, &row, 0).expect("reconcile");
        assert_eq!(
            action,
            Reconciled::Stranded,
            "tx0 mined+buried + final tx absent from the store ⇒ terminal Stranded, never Awaiting-forever",
        );
        assert!(
            crate::intent_store::list_in_flight(&aux)
                .expect("left loop")
                .is_empty(),
            "the row terminates out of the reconcile loop even with no expiry to read",
        );
    }

    #[test]
    fn past_expiry_never_declares_a_no_expiry_tx_dead() {
        // The `expiry != 0` arm is a live double-send door if ever dropped: `0` means NO
        // expiry (mineable at ANY height), and a plain `expiry <= tip` reads it dead at
        // every tip — every engine-created tx carries a real expiry, so no group test can
        // catch that rewrite. Pin the arm directly, plus the exact ZIP-203 boundary the
        // group terminals share.
        assert!(!past_expiry(0, 0), "no-expiry at tip 0");
        assert!(
            !past_expiry(0, u32::MAX),
            "no-expiry is never dead, at any tip"
        );
        assert!(
            !past_expiry(100, 99),
            "tip = expiry - 1: the next block IS the expiry height — still mineable",
        );
        assert!(
            past_expiry(100, 100),
            "tip = expiry: the next block is past the window — dead on this branch",
        );
    }

    #[test]
    fn tex_two_step_requeues_when_neither_mined_and_both_legs_expired() {
        // v-5c DEVICE-PROOF FINDING #1 — the all-offline-expired two-step. A TEX two-step was
        // created (both legs persisted, the row `Sent`) but NEVER broadcast (offline), and BOTH
        // legs then expired unmined. The mainnet device proof pinned the reachable stuck state:
        // the spendable-note witness conservatively read a claim note unspendable (a mid-scan
        // kill / unsettled anchor window), so reconcile stayed in the ReBroadcast regime and the
        // group reconciler re-broadcast two dead txs forever — the permanently-stuck "on its
        // way" cue. The group reconciler must therefore carry its OWN termination guarantee,
        // independent of the witness: this test drives `rebroadcast_group` DIRECTLY (simulating
        // "the witness chose ReBroadcast, however that happened") and pins the terminal.
        let mut st = funded(NOTE_VALUE);
        let (mut aux, id, group) = tex_two_step_sent_row(&mut st);
        let expiry0 = created_tx_expiry(&st, TxId::from_bytes(group[0]));
        let expiry1 = created_tx_expiry(&st, TxId::from_bytes(group[1]));

        // ONE BLOCK BEFORE the earliest expiry (`tip = expiry - 1`): the next block IS `expiry`,
        // still inside the validity window — both legs could yet mine ⇒ re-broadcast BOTH, do
        // NOT requeue (a premature exit is the double-spend door: a fresh re-propose racing a
        // still-mineable tx0). Pins the `expiry <= tip` boundary against an off-by-one.
        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(expiry0.min(expiry1) - 1))
            .expect("tip one block before the earliest expiry");
        let live =
            rebroadcast_group(st.wallet(), &mut aux, id, &group, None, 0).expect("reconcile");
        assert!(
            matches!(live, Reconciled::Rebroadcast { txs: ref r, .. } if r.len() == 2),
            "one block before expiry both legs are still mineable ⇒ re-broadcast, got {live:?}",
        );
        assert_eq!(
            crate::intent_store::list_in_flight(&aux)
                .expect("still")
                .len(),
            1,
            "still Sent — not requeued a block early",
        );

        // BOTH legs dead but the expiry NOT YET BURIED (`tip` one block short of `expiry +
        // REORG_MAX_BLOCKS`): an auto-recoverable reorg could still rewind below the expiry
        // and revive a retained copy of the old group — and THIS path's fresh re-propose
        // selects disjoint inputs, so a revival would pay the recipient twice. NO requeue
        // inside the reorg window (the round-2 #6 margin); the group WAITS for burial, with no
        // re-broadcast of two dead legs (S7 C1: `AwaitingBurial`).
        let dead = expiry0.max(expiry1);
        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(dead + REORG_MAX_BLOCKS - 1))
            .expect("tip one block short of the expiry burying");
        let shallow =
            rebroadcast_group(st.wallet(), &mut aux, id, &group, None, 0).expect("reconcile");
        assert!(
            matches!(shallow, Reconciled::AwaitingBurial),
            "dead but not expiry-buried ⇒ keep waiting inside the reorg window, got {shallow:?}",
        );
        assert_eq!(
            crate::intent_store::list_in_flight(&aux)
                .expect("still")
                .len(),
            1,
            "still Sent — never a requeue inside the reorg window",
        );

        // BOTH legs dead AND expiry-buried (`tip = expiry + REORG_MAX_BLOCKS`) with NEITHER
        // mined ⇒ the group can never move money, across any auto-recoverable reorg —
        // terminal Requeued.
        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(dead + REORG_MAX_BLOCKS))
            .expect("tip a full reorg-depth past the latest expiry — the group is dead whole");
        let action =
            rebroadcast_group(st.wallet(), &mut aux, id, &group, None, 0).expect("reconcile");
        assert_eq!(
            action,
            Reconciled::Requeued,
            "neither mined + both expiry-buried ⇒ reset to Queued, never a dead re-broadcast loop",
        );
        assert!(
            crate::intent_store::list_in_flight(&aux)
                .expect("left loop")
                .is_empty(),
            "the row leaves Sent — the in-flight don't-re-send cue self-clears (item E)",
        );
        assert_eq!(
            crate::intent_store::list_queued(&aux)
                .expect("requeued")
                .len(),
            1,
            "and returns to Queued — the parked, cancellable surface (no money moved)",
        );
    }

    #[test]
    fn single_step_send_requeues_when_unmined_and_expired() {
        // The ONE-ELEMENT-GROUP analogue of the all-expired terminal: a single-step send whose
        // only tx expired unmined is equally dead-whole. Since S7 C1 this is the ONLY requeue of
        // a `Sent` single-step, whatever the witness reads (it frees the note at bare expiry —
        // the tests at the end of this module pin the wait before burial). It can
        // never strand (no earlier tx) — requeue is its only honest terminal.
        let mut st = funded(NOTE_VALUE);
        let proposal = propose_on(
            &mut st,
            request(vec![(sapling_recipient(b"dest"), NOTE_VALUE / 3)]),
        )
        .expect("propose");
        let claims = claim_from_proposal(&proposal).expect("claim");
        let txids = sign_on(&mut st, &proposal).expect("create");
        assert_eq!(txids.len(), 1, "a shielded single-step is one tx");
        let group: Vec<[u8; 32]> = txids.iter().map(|t| *t.as_ref()).collect();
        let expiry = created_tx_expiry(&st, txids[0]);

        let mut aux = Connection::open_in_memory().expect("aux");
        crate::intent_store::ensure_table(&aux).expect("table");
        let id =
            crate::intent_store::enqueue(&mut aux, "zcash:single", 0, None, None).expect("enqueue");
        assert!(crate::intent_store::mark_submitting(&mut aux, id, &claims).expect("submit"));
        assert!(crate::intent_store::mark_sent_multi(&mut aux, id, &group).expect("sent"));

        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(expiry + REORG_MAX_BLOCKS))
            .expect("tip a full reorg-depth past the tx's expiry — dead whole");
        let action =
            rebroadcast_group(st.wallet(), &mut aux, id, &group, None, 0).expect("reconcile");
        assert_eq!(
            action,
            Reconciled::Requeued,
            "an expiry-buried unmined single-step is dead whole ⇒ requeue (its only honest terminal)",
        );
        assert_eq!(
            crate::intent_store::list_queued(&aux)
                .expect("requeued")
                .len(),
            1,
            "back on the queued surface for a fresh re-propose",
        );
    }

    // ── W-swap-4-a-2 rule 2: the deposit deadline HOLD on the (re)broadcast tail ────
    //
    // Post-FR-23-a the sign (execute) and the FIRST broadcast (a kick, or a later drain
    // pass) are decoupled, so the ReBroadcast arm can be a deposit's FIRST hand-off to the
    // network — it must never first-feed a DEAD quote. These pin the hold on the gate's
    // two non-Proceed arms, the live-deadline control, and the untouched terminals.

    /// Build a SIGNED deposit-tagged `Sent` row (a real created single-step tx) — the
    /// exact shape sign-at-execute leaves for the kick/drain to broadcast.
    fn deposit_sent_row(
        st: &mut HarnessState,
        deadline: i64,
    ) -> (Connection, QueuedSendId, Vec<[u8; 32]>) {
        let proposal = propose_on(
            st,
            request(vec![(sapling_recipient(b"deposit"), NOTE_VALUE / 3)]),
        )
        .expect("propose");
        let claims = claim_from_proposal(&proposal).expect("claim");
        let txids = sign_on(st, &proposal).expect("create");
        let group: Vec<[u8; 32]> = txids.iter().map(|t| *t.as_ref()).collect();
        let mut aux = Connection::open_in_memory().expect("aux");
        crate::intent_store::ensure_table(&aux).expect("table");
        // Production's migrate creates the accepted-mark table beside the intents (`db.rs`); a
        // deposit's reconcile reads it (F01).
        crate::delivery::ensure_table(&aux).expect("accepted table");
        let id = crate::intent_store::enqueue(&mut aux, "zcash:deposit", 0, Some(deadline), None)
            .expect("enqueue");
        assert!(crate::intent_store::mark_submitting(&mut aux, id, &claims).expect("submit"));
        assert!(crate::intent_store::mark_sent_multi(&mut aux, id, &group).expect("sent"));
        (aux, id, group)
    }

    #[test]
    fn clamped_deposit_deadline_bounds_a_hostile_provider() {
        // The arch review HIGH fold: the durable tag can never exceed OUR ceiling, so
        // the one-deposit-in-flight guard's lockout is never provider-controlled.
        let now = 1_800_000_000u64;
        let ceiling = (now + crate::constants::SWAP_DEPOSIT_DEADLINE_OUT_OF_ZEC_SECS) as i64;
        // A sane minutes-scale deadline passes through verbatim.
        assert_eq!(
            clamped_deposit_deadline(now + 600, now),
            (now + 600) as i64,
            "a real provider deadline is untouched",
        );
        // A hostile 10-year deadline clamps to now + the 15-min OutOfZec window
        // (W-swap-4-a-3: the window WE request is the ceiling — an echo beyond it
        // is hostile/buggy by definition).
        assert_eq!(
            clamped_deposit_deadline(now + 315_360_000, now),
            ceiling,
            "an absurd deadline is bounded by our own ceiling",
        );
        // Even u64::MAX clamps (the min picks the ceiling long before the i64 edge).
        assert_eq!(clamped_deposit_deadline(u64::MAX, now), ceiling);
        // At the clamp the gate is LIVE for the window then expires — never eternal.
        assert_eq!(deposit_gate(Some(ceiling), now), DepositGate::Proceed);
        assert_eq!(
            deposit_gate(
                Some(ceiling),
                now + crate::constants::SWAP_DEPOSIT_DEADLINE_OUT_OF_ZEC_SECS
            ),
            DepositGate::Expired,
        );
    }

    #[test]
    fn purge_deletes_only_lapsed_queued_deposits() {
        // The seedless drain purge (the review-BLOCKER fold): ONLY a `Queued` +
        // deadline-tagged + gate-`Expired` row dies. An ordinary send is untouched at any
        // clock; a LIVE deposit is untouched; an untimeable clock (`Wait`) touches nothing;
        // an in-flight (Submitting/Sent) deposit is Phase 1's business, never purged here.
        let mut aux = Connection::open_in_memory().expect("aux");
        crate::intent_store::ensure_table(&aux).expect("table");

        // A lapsed deposit (plausible deadline, well past) + an ordinary send.
        crate::intent_store::enqueue(&mut aux, "zcash:lapsed-dep", 1, Some(1_700_000_100), None)
            .expect("lapsed deposit");
        crate::intent_store::enqueue(&mut aux, "zcash:plain", 2, None, None).expect("ordinary");

        // An untimeable clock purges NOTHING (Wait — fail-safe, re-checked next pass).
        assert_eq!(
            purge_lapsed_queued_deposits(&mut aux, 0).expect("purge on a broken clock"),
            0,
            "a sub-floor clock cannot judge the deadline — nothing deleted",
        );

        // A plausible clock past the deadline purges EXACTLY the lapsed deposit.
        assert_eq!(
            purge_lapsed_queued_deposits(&mut aux, 1_900_000_000).expect("purge"),
            1,
            "the lapsed deposit dies; the guard frees",
        );
        let left = crate::intent_store::list_queued(&aux).expect("list");
        assert_eq!(left.len(), 1, "the ordinary send survives");
        assert_eq!(left[0].deposit_deadline, None);

        // The guard is genuinely freed: a LIVE deposit enqueues, and the purge leaves it.
        let live =
            crate::intent_store::enqueue(&mut aux, "zcash:live-dep", 3, Some(1_950_000_000), None)
                .expect("a new deposit enqueues once the zombie is purged");
        assert_eq!(
            purge_lapsed_queued_deposits(&mut aux, 1_900_000_000).expect("purge"),
            0,
            "a live-deadline deposit is never purged",
        );

        // An IN-FLIGHT deposit is out of scope even when lapsed (state != Queued): the
        // Phase-1 reconcile owns it (hold → expiry → requeue → THEN this purge catches it).
        assert!(
            crate::intent_store::mark_submitting(
                &mut aux,
                live,
                &[NoteClaim {
                    txid: [3; 32],
                    protocol: PROTO_SAPLING,
                    output_index: 0,
                }],
            )
            .expect("submit")
        );
        assert_eq!(
            purge_lapsed_queued_deposits(&mut aux, 2_100_000_000).expect("purge"),
            0,
            "Submitting/Sent rows are never purged here — their notes may be spent",
        );
    }

    #[test]
    fn rebroadcast_holds_a_deposit_past_its_deadline() {
        // A signed, unmined, still-VALID (inside its expiry window) deposit whose QUOTE
        // deadline has lapsed on a plausible clock: broadcasting would feed a dead quote
        // (fee-burning provider refund) ⇒ HOLD, row untouched. Control first: the SAME row
        // under a LIVE deadline broadcasts — the gate is the only differentiator.
        let mut st = funded(NOTE_VALUE);
        let deadline: i64 = 1_800_000_000;
        let (mut aux, id, group) = deposit_sent_row(&mut st, deadline);

        // LIVE (now + margin < deadline, clock plausible) ⇒ the normal re-broadcast.
        let live_now = CLOCK_PLAUSIBILITY_FLOOR_SECS + 1;
        let live = rebroadcast_group(st.wallet(), &mut aux, id, &group, Some(deadline), live_now)
            .expect("reconcile");
        assert!(
            matches!(live, Reconciled::Rebroadcast { txs: ref r, .. } if r.len() == 1),
            "a live-deadline deposit broadcasts normally, got {live:?}",
        );

        // LAPSED (now well past deadline + margin) ⇒ HOLD: no bytes for the network, the
        // row stays Sent (the notes are spent by the created tx; the all-expired requeue →
        // `deposit_gate` delete terminal resolves it once the tx expires + buries).
        let lapsed = rebroadcast_group(
            st.wallet(),
            &mut aux,
            id,
            &group,
            Some(deadline),
            1_900_000_000,
        )
        .expect("reconcile");
        assert_eq!(
            lapsed,
            Reconciled::DepositBroadcastHold,
            "a lapsed-deadline deposit is HELD — never a first feed of a dead quote",
        );
        assert_eq!(
            crate::intent_store::list_in_flight(&aux)
                .expect("still")
                .len(),
            1,
            "the hold leaves the row Sent (untouched this pass)",
        );
        assert!(
            crate::intent_store::list_queued(&aux)
                .expect("no requeue")
                .is_empty(),
            "the hold never resets state — the terminals own resolution",
        );
    }

    #[test]
    fn rebroadcast_holds_a_deposit_on_an_untrustworthy_clock() {
        // `DepositGate::Wait` (a sub-plausibility-floor clock) can't time the deadline —
        // fail-safe is NOT broadcasting (re-checked next pass once the clock syncs). An
        // ordinary send is clock-INDEPENDENT here (every pre-existing hold-free test passes
        // `None, 0`), so availability regresses for nothing but a deadline-tagged deposit
        // on a broken clock.
        let mut st = funded(NOTE_VALUE);
        let (mut aux, id, group) = deposit_sent_row(&mut st, 1_800_000_000);
        let action = rebroadcast_group(
            st.wallet(),
            &mut aux,
            id,
            &group,
            Some(1_800_000_000),
            0, // pre-epoch/unsynced clock — below CLOCK_PLAUSIBILITY_FLOOR_SECS
        )
        .expect("reconcile");
        assert_eq!(
            action,
            Reconciled::DepositBroadcastHold,
            "an untimeable deadline holds fail-safe (never broadcast on a clock that can't judge it)",
        );
        assert_eq!(
            crate::intent_store::list_in_flight(&aux)
                .expect("still")
                .len(),
            1,
            "row untouched — re-checked next pass",
        );
    }

    #[test]
    fn rebroadcast_still_deletes_a_mined_buried_deposit_past_its_deadline() {
        // TERMINALS BEFORE THE HOLD: a deposit that DID broadcast pre-deadline and mined
        // must still leave the outbox once buried, even though its deadline has since
        // lapsed — the hold gates ONLY the network tail, never the reconcile decisions
        // (a hold-first rewrite would wedge every completed lapsed deposit `Sent` forever,
        // holding the one-deposit-in-flight guard with it).
        let mut st = funded(NOTE_VALUE);
        let (mut aux, _id, group) = deposit_sent_row(&mut st, 1_800_000_000);
        let (mined_h, _) = st.generate_next_block_including(TxId::from_bytes(group[0]));
        st.scan_cached_blocks(mined_h, 1);
        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(u32::from(mined_h) + REORG_MAX_BLOCKS))
            .expect("tip a full reorg-depth past the mined block");
        let row = crate::intent_store::list_in_flight(&aux).expect("inflight")[0].clone();
        let action =
            reconcile_inflight(st.wallet(), &mut aux, &row, 1_900_000_000).expect("reconcile");
        assert_eq!(
            action,
            Reconciled::Deleted,
            "a mined+buried deposit deletes on schedule — the lapsed deadline holds only the broadcast tail",
        );
        assert!(
            crate::intent_store::list_in_flight(&aux)
                .expect("gone")
                .is_empty(),
            "the outbox row clears (and with it the one-deposit-in-flight guard)",
        );
    }

    #[test]
    fn pruned_leg_never_requeues_the_row_stays_awaiting() {
        // CONSERVATISM GUARD: the requeue terminal must NEVER fire off an unprovable leg. tx0 is
        // real and expired, but the final leg is GONE from the store (pruned/collected) — its
        // expiry cannot be read, so it is NOT proven dead (for all this code knows it was
        // broadcast once and could still mine). No requeue: fall through to the re-broadcast
        // read, which misses ⇒ conservative `Awaiting`, the row stays Sent. A requeue here is
        // exactly the double-spend door the predicate exists to keep shut.
        let mut st = funded(NOTE_VALUE);
        let to = tex_recipient([0x04; 20]);
        let proposal = propose_on(&mut st, request(vec![(to, 20_000)])).expect("TEX proposes");
        let claims = claim_from_proposal(&proposal).expect("claim");
        let txids = sign_two_step_on(&mut st, &proposal);
        let tx0: [u8; 32] = *txids[0].as_ref();
        let absent_tx1 = [0xEE; 32]; // never persisted — a pruned/collected final leg
        let expiry0 = created_tx_expiry(&st, txids[0]);

        let mut aux = Connection::open_in_memory().expect("aux");
        crate::intent_store::ensure_table(&aux).expect("table");
        let id = crate::intent_store::enqueue(&mut aux, "zcash:tex-pruned-unmined", 0, None, None)
            .expect("enqueue");
        assert!(crate::intent_store::mark_submitting(&mut aux, id, &claims).expect("submit"));
        let group = vec![tx0, absent_tx1];
        assert!(crate::intent_store::mark_sent_multi(&mut aux, id, &group).expect("sent group"));

        // Far past tx0's expiry — the only READABLE leg is dead, but the group is not PROVEN dead.
        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(expiry0 + REORG_MAX_BLOCKS))
            .expect("tip far past tx0's expiry");
        let action =
            rebroadcast_group(st.wallet(), &mut aux, id, &group, None, 0).expect("reconcile");
        assert_eq!(
            action,
            Reconciled::Awaiting,
            "an unreadable (pruned) leg is never proven dead ⇒ no requeue, conservative Awaiting",
        );
        assert_eq!(
            crate::intent_store::list_in_flight(&aux)
                .expect("still")
                .len(),
            1,
            "the row stays Sent — never a requeue off an unprovable leg",
        );
        assert!(
            crate::intent_store::list_queued(&aux)
                .expect("no requeue")
                .is_empty(),
            "nothing returned to Queued",
        );
    }

    #[test]
    fn mixed_expired_and_live_legs_never_requeue() {
        // THE ALL-LEGS CONJUNCTION (the double-spend hinge): one dead leg is NOT the group dead.
        // Fabricate a group from two REAL created-but-unmined txs with DIFFERENT expiries
        // (created a full reorg-depth apart), park the tip where the first is dead AND
        // expiry-buried but the second is still mineable, and assert the group RE-BROADCASTS
        // (both unmined) rather than requeues — a requeue while ANY leg could still mine would
        // let a fresh re-propose race a live tx.
        let mut st = funded_multi(&[40_000, 40_000]);
        let prop_a = propose_on(&mut st, request(vec![(sapling_recipient(b"a"), 10_000)]))
            .expect("propose A");
        let claims = claim_from_proposal(&prop_a).expect("claim A");
        let tx_a = sign_on(&mut st, &prop_a).expect("create A")[0];
        let expiry_a = created_tx_expiry(&st, tx_a);

        // A full reorg-depth + margin later (scanned so the wallet stays consistent), create
        // B — its expiry lands past A's burial point, so a tip can see A dead+buried while B
        // is still inside its validity window.
        let gap = REORG_MAX_BLOCKS + 20;
        let mut first = None;
        for _ in 0..gap {
            let (h, _) = st.generate_empty_block();
            first.get_or_insert(h);
        }
        st.scan_cached_blocks(first.expect("a generated block"), gap as usize);
        let prop_b = propose_on(&mut st, request(vec![(sapling_recipient(b"b"), 10_000)]))
            .expect("propose B");
        let tx_b = sign_on(&mut st, &prop_b).expect("create B")[0];
        let expiry_b = created_tx_expiry(&st, tx_b);
        assert!(
            expiry_a + REORG_MAX_BLOCKS < expiry_b,
            "test precondition: A buries while B is still live",
        );

        let group = vec![*tx_a.as_ref(), *tx_b.as_ref()];
        let mut aux = Connection::open_in_memory().expect("aux");
        crate::intent_store::ensure_table(&aux).expect("table");
        let id =
            crate::intent_store::enqueue(&mut aux, "zcash:mixed", 0, None, None).expect("enqueue");
        assert!(crate::intent_store::mark_submitting(&mut aux, id, &claims).expect("submit"));
        assert!(crate::intent_store::mark_sent_multi(&mut aux, id, &group).expect("sent group"));

        // A dead AND expiry-buried (`tip = expiry_a + REORG_MAX_BLOCKS`), B live
        // (`expiry_b > tip`).
        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(expiry_a + REORG_MAX_BLOCKS))
            .expect("tip past A's burial, inside B's validity window");
        let action =
            rebroadcast_group(st.wallet(), &mut aux, id, &group, None, 0).expect("reconcile");
        assert!(
            matches!(action, Reconciled::Rebroadcast { txs: ref r, .. } if r.len() == 2),
            "one live leg keeps the group in the re-broadcast regime, got {action:?}",
        );
        assert_eq!(
            crate::intent_store::list_in_flight(&aux)
                .expect("still")
                .len(),
            1,
            "still Sent — never a requeue while any leg could mine",
        );
    }

    #[test]
    fn requeued_dead_group_with_lapsed_deposit_is_deleted_at_the_drain() {
        // The requeue terminal is deliberately deadline-AGNOSTIC (the ReBroadcast arm's
        // documented posture) — its safety leans on the drain's own `deposit_gate` running
        // BEFORE any propose. Pin that TWO-HOP: an all-expired dead group carrying a LAPSED
        // swap-deposit deadline is Requeued by the terminal, and the very next drain pass
        // DELETES it (`DepositExpired`) — the dead quote is never fed by the fresh re-propose
        // the requeue enables.
        let now = 2_000_000_000u64;
        let lapsed = (now - 100) as i64; // past, well beyond the safety margin
        let mut st = funded(NOTE_VALUE);

        // A real created-but-unmined de-shield (deposit shape), enrolled as a Sent intent.
        let deposit = transparent_recipient([0x66; 20]);
        let proposal =
            propose_on(&mut st, request(vec![(deposit.clone(), NOTE_VALUE / 3)])).expect("propose");
        let claims = claim_from_proposal(&proposal).expect("claim");
        let txids = sign_on(&mut st, &proposal).expect("create");
        let group: Vec<[u8; 32]> = txids.iter().map(|t| *t.as_ref()).collect();
        let expiry = created_tx_expiry(&st, txids[0]);

        let mut aux = aux_conn();
        let id = crate::intent_store::enqueue(
            &mut aux,
            "zcash:deposit-dead-group",
            1,
            Some(lapsed),
            None,
        )
        .expect("enqueue");
        assert!(crate::intent_store::mark_submitting(&mut aux, id, &claims).expect("submit"));
        assert!(crate::intent_store::mark_sent_multi(&mut aux, id, &group).expect("sent"));

        // Hop 1: dead + expiry-buried ⇒ the terminal requeues, deadline unexamined.
        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(expiry + REORG_MAX_BLOCKS))
            .expect("tip a reorg-depth past the expiry");
        let action =
            rebroadcast_group(st.wallet(), &mut aux, id, &group, None, 0).expect("reconcile");
        assert_eq!(
            action,
            Reconciled::Requeued,
            "the dead group requeues (the ReBroadcast arm stays deadline-agnostic)",
        );

        // Hop 2: the drain's own `deposit_gate` catches the lapsed deadline BEFORE any propose.
        let network = *st.network();
        let aid = st.test_account().expect("acct").id();
        let usk = st.test_account().expect("acct").usk().clone();
        let prover = crate::prover::tx_prover();
        let prepared = prepare_queued(
            st.wallet_mut(),
            &network,
            &mut aux,
            aid,
            request(vec![(deposit, NOTE_VALUE / 3)]),
            usk,
            prover,
            prover,
            id,
            Some(lapsed),
            0,
            now,
            crate::consensus::SigningPermit::assume_current_for_test(),
        )
        .expect("prepare");
        assert_eq!(
            prepared,
            Prepared::DepositExpired,
            "the requeued dead quote is deleted at the drain, never fed",
        );
        assert!(
            crate::intent_store::list_queued(&aux)
                .expect("q")
                .is_empty()
                && crate::intent_store::list_in_flight(&aux)
                    .expect("f")
                    .is_empty(),
            "the row is gone — the two-hop terminal outcome the comment promises",
        );
    }

    #[test]
    fn resubmit_reconcile_requeues_a_never_created_submitting_intent() {
        // CRASH WINDOW (a) through the DB-mutating path: `mark_submitting` committed (claim durable)
        // but create never ran — the notes are still spendable, so `reconcile_inflight` must RE-QUEUE
        // for a fresh re-propose (never strand a Submitting row, never lose the queued send).
        let mut st = funded(NOTE_VALUE);
        let mut aux = aux_conn();
        let id =
            crate::intent_store::enqueue(&mut aux, "zcash:queued", 1, None, None).expect("enqueue");
        let proposal = propose_on(
            &mut st,
            request(vec![(sapling_recipient(b"dest"), NOTE_VALUE / 3)]),
        )
        .expect("propose");
        let claims = claim_from_proposal(&proposal).expect("claim");
        assert!(crate::intent_store::mark_submitting(&mut aux, id, &claims).expect("submit"));
        // No create — the notes are still spendable.
        let row = crate::intent_store::list_in_flight(&aux).expect("inflight")[0].clone();
        let action = reconcile_inflight(st.wallet(), &mut aux, &row, 0).expect("reconcile");
        assert_eq!(
            action,
            Reconciled::Requeued,
            "a never-created Submitting intent ⇒ re-queue",
        );
        assert_eq!(
            crate::intent_store::list_queued(&aux).expect("q").len(),
            1,
            "back in the FIFO queue for a fresh re-propose",
        );
        assert!(
            crate::intent_store::list_in_flight(&aux)
                .expect("none")
                .is_empty(),
            "no longer in flight",
        );
    }

    #[test]
    fn resubmit_prepare_insufficient_funds_is_stale_and_stays_queued() {
        // QueuedSendStale (RW-BCAST-001): a re-propose that can no longer cover the intent is
        // SURFACED (Stale) and the intent is LEFT Queued — never silently dropped, never advanced to
        // a half-state. (Funds may still arrive; a future per-intent FFI read returns the typed
        // error.)
        let mut st = funded(NOTE_VALUE);
        let mut aux = aux_conn();
        let id = crate::intent_store::enqueue(&mut aux, "zcash:overask", 1, None, None)
            .expect("enqueue");
        // Far beyond the single 60k note.
        let req = request(vec![(sapling_recipient(b"dest"), 10_000_000)]);
        let network = *st.network();
        let aid = st.test_account().expect("acct").id();
        let usk = st.test_account().expect("acct").usk().clone();
        let prover = crate::prover::tx_prover();
        let prepared = prepare_queued(
            st.wallet_mut(),
            &network,
            &mut aux,
            aid,
            req,
            usk,
            prover,
            prover,
            id,
            None,
            0,
            0,
            crate::consensus::SigningPermit::assume_current_for_test(),
        )
        .expect("prepare");
        assert_eq!(
            prepared,
            Prepared::Stale,
            "an unfundable re-propose is QueuedSendStale",
        );
        assert_eq!(
            crate::intent_store::list_queued(&aux).expect("q").len(),
            1,
            "a stale send STAYS queued — never silently dropped",
        );
        assert!(
            crate::intent_store::list_in_flight(&aux)
                .expect("none")
                .is_empty(),
            "never advanced to in-flight",
        );
    }

    #[test]
    fn resubmit_prepare_two_queued_intents_over_one_note_never_double_spends_in_a_pass() {
        // THE operational double-spend test the whole §6.3 guard exists for. A user who composes
        // TWO sends offline (a plane, a tunnel) ends up with two Queued intents and ONE spendable
        // note. When the network returns, the resubmission pass walks them in FIFO order. The first
        // MUST broadcast (it spends the note); the second MUST hit InsufficientFunds on its fresh
        // re-propose — because A's create flipped the only note SPENT — and surface as Stale, NOT
        // mint a second tx over the same note. This is the cross-intent no-double-spend property in
        // ONE pass; every existing resubmit test drives a single intent, so nothing pins it.
        let mut st = funded(NOTE_VALUE); // exactly ONE spendable note
        let mut aux = aux_conn();
        let network = *st.network();
        let aid = st.test_account().expect("acct").id();

        // FIFO: A enqueued first, B second. Each asks well within the single note on its own, so a
        // naive per-intent re-propose would fund BOTH — only the live spend witness stops B.
        let id_a = crate::intent_store::enqueue(&mut aux, "zcash:send-a", 1, None, None)
            .expect("enqueue A");
        let id_b = crate::intent_store::enqueue(&mut aux, "zcash:send-b", 2, None, None)
            .expect("enqueue B");
        let prover = crate::prover::tx_prover();

        // Pass over intent A: it funds, claims, creates, records the txid — Broadcast.
        let usk_a = st.test_account().expect("acct").usk().clone();
        let prepared_a = prepare_queued(
            st.wallet_mut(),
            &network,
            &mut aux,
            aid,
            request(vec![(sapling_recipient(b"to-a"), NOTE_VALUE / 4)]),
            usk_a,
            prover,
            prover,
            id_a,
            None,
            0,
            0,
            crate::consensus::SigningPermit::assume_current_for_test(),
        )
        .expect("prepare A");
        assert!(
            matches!(prepared_a, Prepared::Broadcast(ref raw) if !raw.is_empty()),
            "the first intent spends the one note and broadcasts, got {prepared_a:?}",
        );

        // SAME wallet, SAME pass: the only note is now spent. B re-proposes FRESH against the
        // current chain state and finds nothing spendable ⇒ InsufficientFunds ⇒ Stale. It is NEVER
        // advanced to in-flight (no claim recorded, no second tx created).
        let usk_b = st.test_account().expect("acct").usk().clone();
        let prepared_b = prepare_queued(
            st.wallet_mut(),
            &network,
            &mut aux,
            aid,
            request(vec![(sapling_recipient(b"to-b"), NOTE_VALUE / 4)]),
            usk_b,
            prover,
            prover,
            id_b,
            None,
            0,
            0,
            crate::consensus::SigningPermit::assume_current_for_test(),
        )
        .expect("prepare B");
        assert_eq!(
            prepared_b,
            Prepared::Stale,
            "the second intent cannot double-spend the already-spent note — it is Stale, not a tx",
        );

        // Exactly ONE in-flight row (A, Sent, with a txid); B is still Queued — both rows intact,
        // nothing dropped, nothing double-created.
        let in_flight = crate::intent_store::list_in_flight(&aux).expect("inflight");
        assert_eq!(in_flight.len(), 1, "only A advanced to in-flight");
        assert_eq!(
            in_flight[0].id, id_a,
            "and it is A (the first in FIFO order)"
        );
        assert_eq!(in_flight[0].state, crate::intent_store::IntentState::Sent);
        assert!(
            !in_flight[0].txids.is_empty(),
            "A's tx group is durable before broadcast"
        );

        let queued = crate::intent_store::list_queued(&aux).expect("queued");
        assert_eq!(queued.len(), 1, "B stays queued — never silently dropped");
        assert_eq!(queued[0].id, id_b, "and it is B (the over-committed send)");

        // The witness corroborates the no-double-spend: A's claimed notes read SPENT, so any later
        // re-propose for B keeps reading InsufficientFunds until B's funds genuinely arrive.
        assert!(
            in_flight[0]
                .claims
                .iter()
                .all(|c| !note_spendable(st.wallet(), c).expect("witness")),
            "A's note is spent — the witness that keeps B from re-spending it",
        );
    }

    #[test]
    fn resubmit_reconcile_is_idempotent_across_passes_never_a_second_create() {
        // CRASH/FLAKY-NETWORK REALITY: the sync loop runs reconcile on EVERY pass, and a broadcast
        // can silently fail (lossy mobile radio), so the same unmined Sent intent is reconciled
        // again next pass, and the next. Each pass must RE-BROADCAST the SAME persisted bytes — and
        // NEVER mint a second tx or mutate the recorded txid. The existing rebroadcast test runs
        // ONE pass; this pins the cross-pass invariant (re-broadcast is a pure replay, idempotent).
        let (st, mut aux, _id, txid) = prepared_sent_intent();

        let row1 = crate::intent_store::list_in_flight(&aux).expect("inflight")[0].clone();
        let action1 = reconcile_inflight(st.wallet(), &mut aux, &row1, 0).expect("pass 1");
        let raw1 = match action1 {
            Reconciled::Rebroadcast { txs: raw, .. } => raw,
            other => panic!("pass 1: expected Rebroadcast, got {other:?}"),
        };

        // The tx is STILL unmined (we never mined it) — a second pass over the identical row.
        let row2 = crate::intent_store::list_in_flight(&aux).expect("inflight")[0].clone();
        assert_eq!(
            row2, row1,
            "the row is unchanged between passes (no txid churn)"
        );
        let action2 = reconcile_inflight(st.wallet(), &mut aux, &row2, 0).expect("pass 2");
        let raw2 = match action2 {
            Reconciled::Rebroadcast { txs: raw, .. } => raw,
            other => panic!("pass 2: expected Rebroadcast, got {other:?}"),
        };

        assert_eq!(
            raw1, raw2,
            "every pass re-broadcasts the IDENTICAL consensus bytes — a pure replay, no new tx",
        );
        let after = crate::intent_store::list_in_flight(&aux).expect("inflight");
        assert_eq!(
            after.len(),
            1,
            "still exactly one in-flight row after two passes"
        );
        assert_eq!(
            after[0].txids,
            vec![txid],
            "the recorded txid is stable across passes — never a second create",
        );
        assert_eq!(
            after[0].state,
            crate::intent_store::IntentState::Sent,
            "the row stays Sent (a re-broadcast never advances or resets it)",
        );
    }

    #[test]
    fn resubmit_requeued_intent_self_heals_and_completes_on_the_next_pass() {
        // THE SELF-HEAL LOOP end-to-end (the property that makes "your money eventually moves"
        // true): a send that crashed mid-flight (Submitting, never created) is RE-QUEUED by phase-1
        // reconcile, then on the NEXT pass `prepare_queued` re-proposes it fresh and drives it all
        // the way to Sent. Existing tests prove each half (reconcile→Requeued, prepare→Sent)
        // SEPARATELY over independent fixtures; nothing proves the SAME intent survives the crash
        // and then completes — the loop actually closing.
        let mut st = funded(NOTE_VALUE);
        let mut aux = aux_conn();
        let id = crate::intent_store::enqueue(&mut aux, "zcash:crashed-send", 1, None, None)
            .expect("enqueue");
        let network = *st.network();
        let aid = st.test_account().expect("acct").id();
        let prover = crate::prover::tx_prover();

        // Simulate the crash window (a): mark_submitting committed the claim, but the process died
        // before create ever ran — the note is still spendable.
        let proposal = propose_on(
            &mut st,
            request(vec![(sapling_recipient(b"dest"), NOTE_VALUE / 3)]),
        )
        .expect("propose");
        let claims = claim_from_proposal(&proposal).expect("claim");
        assert!(crate::intent_store::mark_submitting(&mut aux, id, &claims).expect("submit"));

        // Pass 1 — phase-1 reconcile: the notes are spendable ⇒ re-queue (recover the crashed send).
        let row = crate::intent_store::list_in_flight(&aux).expect("inflight")[0].clone();
        assert_eq!(
            reconcile_inflight(st.wallet(), &mut aux, &row, 0).expect("reconcile"),
            Reconciled::Requeued,
            "the crashed Submitting send is recovered to Queued, never stranded",
        );
        let requeued = crate::intent_store::list_queued(&aux).expect("queued");
        assert_eq!(requeued.len(), 1, "back in the FIFO queue");
        assert_eq!(
            requeued[0].id, id,
            "the SAME intent, re-proposable from its URI"
        );

        // Pass 2 — phase-2 prepare: re-propose the re-queued intent FRESH and complete it. The note
        // freed by the never-created tx is selectable again, so the send finally goes through.
        let usk = st.test_account().expect("acct").usk().clone();
        let req = request(vec![(sapling_recipient(b"dest"), NOTE_VALUE / 3)]);
        let prepared = prepare_queued(
            st.wallet_mut(),
            &network,
            &mut aux,
            aid,
            req,
            usk,
            prover,
            prover,
            id,
            None,
            0,
            0,
            crate::consensus::SigningPermit::assume_current_for_test(),
        )
        .expect("prepare");
        assert!(
            matches!(prepared, Prepared::Broadcast(ref raw) if !raw.is_empty()),
            "the recovered intent re-proposes cleanly and completes — the loop closes, got {prepared:?}",
        );
        let in_flight = crate::intent_store::list_in_flight(&aux).expect("inflight");
        assert_eq!(in_flight.len(), 1, "exactly one in-flight row");
        assert_eq!(
            in_flight[0].id, id,
            "the SAME intent that crashed now carries the tx"
        );
        assert_eq!(in_flight[0].state, crate::intent_store::IntentState::Sent);
        assert!(
            !in_flight[0].txids.is_empty(),
            "tx group durable before broadcast"
        );
        assert!(
            crate::intent_store::list_queued(&aux)
                .expect("q")
                .is_empty(),
            "the queue is drained — the crashed send self-healed to completion",
        );
    }

    // ── inc-2d-swap-a: the §4.4 deposit-deadline exclusion ───────────────────────────

    #[test]
    fn deposit_gate_classifies_deadline_and_clock() {
        // The §4.4 decision predicate (gate 7 — pinned at the constant boundaries). On a
        // TRUSTWORTHY (post-floor) clock: an ordinary send proceeds; a deposit is Expired once
        // `now + DEPOSIT_FIRST_FEED_MARGIN_SECS` reaches the deadline (the block-time-sized
        // first-feed margin, W-swap-4-a-3 — never fed without mining room) or the deadline is
        // corrupt (≤0, fail-closed); else it proceeds.
        let now = 2_000_000_000u64; // > CLOCK_PLAUSIBILITY_FLOOR_SECS
        assert_eq!(
            deposit_gate(None, now),
            DepositGate::Proceed,
            "an ordinary send has no deadline — always proceeds",
        );
        assert_eq!(
            deposit_gate(Some((now + DEPOSIT_FIRST_FEED_MARGIN_SECS + 1) as i64), now),
            DepositGate::Proceed,
            "one second beyond the margin ⇒ still live",
        );
        assert_eq!(
            deposit_gate(Some((now + DEPOSIT_FIRST_FEED_MARGIN_SECS) as i64), now),
            DepositGate::Expired,
            "exactly at the margin boundary ⇒ expired (born-within-margin is excluded)",
        );
        assert_eq!(
            deposit_gate(Some((now - 1) as i64), now),
            DepositGate::Expired,
            "already past ⇒ expired",
        );
        assert_eq!(
            deposit_gate(Some(0), now),
            DepositGate::Expired,
            "zero ⇒ fail-closed expired"
        );
        assert_eq!(
            deposit_gate(Some(-5), now),
            DepositGate::Expired,
            "negative ⇒ fail-closed"
        );

        // On an UNSYNCED clock (below the floor) a deposit WAITS in BOTH deadline directions —
        // never fed by, never deleted by, an untimeable deadline (the boot-before-NTP case).
        let unsynced = CLOCK_PLAUSIBILITY_FLOOR_SECS - 1;
        assert_eq!(
            deposit_gate(Some(1), unsynced),
            DepositGate::Wait,
            "a long-past deadline on an unsynced clock WAITS — not deleted by a clock saying 1970",
        );
        assert_eq!(
            deposit_gate(Some((unsynced + 10_000) as i64), unsynced),
            DepositGate::Wait,
            "a far-future deadline on an unsynced clock WAITS — not fed by an untimeable quote",
        );
        assert_eq!(
            deposit_gate(None, unsynced),
            DepositGate::Proceed,
            "an ordinary send is unaffected by the clock floor — only deposits wait",
        );
        // The literal `now_unix == 0` the orchestrator produces on a pre-epoch clock
        // (`SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, …)`) is below the floor ⇒ `Wait`,
        // NEVER fail-open: a broken clock can't feed a deposit to a dead quote (security M-1).
        assert_eq!(
            deposit_gate(Some(1), 0),
            DepositGate::Wait,
            "a 0 (pre-epoch / unset) clock WAITS — the broken-clock fail-open is closed",
        );
    }

    #[test]
    fn deposit_send_rejects_expired_quote() {
        // §8 gate-1 named test (§4.4): a swap deposit whose quote deadline has lapsed is NEVER
        // proposed, signed, or broadcast — the resubmission core DELETES it (terminal; the user
        // re-quotes), so no ZEC is ever sent to a dead quote. Covers BOTH exclusion seams.
        let now = 2_000_000_000u64;
        let lapsed = (now - 100) as i64; // past, well beyond the safety margin

        // ── prepare_queued (the Queued path): a lapsed deposit is deleted WITHOUT proposing ──
        let mut st = funded(NOTE_VALUE);
        let mut aux = aux_conn();
        // A deposit is a de-shield → the recipient is transparent (the provider has no shielded
        // support, §2.6). The URI is the deposit send; the deadline is the tag.
        let deposit = transparent_recipient([0x55; 20]);
        let id = crate::intent_store::enqueue(&mut aux, "zcash:deposit", 1, Some(lapsed), None)
            .expect("enqueue");
        let req = request(vec![(deposit, NOTE_VALUE / 3)]);
        let network = *st.network();
        let aid = st.test_account().expect("acct").id();
        let usk = st.test_account().expect("acct").usk().clone();
        let prover = crate::prover::tx_prover();
        let prepared = prepare_queued(
            st.wallet_mut(),
            &network,
            &mut aux,
            aid,
            req,
            usk,
            prover,
            prover,
            id,
            Some(lapsed),
            0,
            now,
            crate::consensus::SigningPermit::assume_current_for_test(),
        )
        .expect("prepare");
        assert_eq!(
            prepared,
            Prepared::DepositExpired,
            "a lapsed-deadline deposit is excluded, never proposed",
        );
        assert!(
            crate::intent_store::list_queued(&aux)
                .expect("q")
                .is_empty()
                && crate::intent_store::list_in_flight(&aux)
                    .expect("f")
                    .is_empty(),
            "the dead-quote deposit row is DELETED — never advanced to Submitting/Sent",
        );
        // No ZEC moved: the funds are intact (a fresh propose still selects the note — nothing was
        // signed or spent against the dead quote).
        assert!(
            propose_on(
                &mut st,
                request(vec![(sapling_recipient(b"intact"), NOTE_VALUE / 3)]),
            )
            .is_ok(),
            "no spend happened — the wallet's funds are untouched",
        );

        // ── reconcile_inflight (the ReProposeFresh arm): a lapsed deposit whose notes freed ──
        // A Submitting deposit whose tx was never created (notes still spendable) + a lapsed
        // deadline must DELETE (never re-queue toward a doomed re-propose to a dead quote).
        let mut st2 = funded(NOTE_VALUE);
        let mut aux2 = aux_conn();
        let id2 = crate::intent_store::enqueue(&mut aux2, "zcash:deposit2", 1, Some(lapsed), None)
            .expect("enqueue");
        let proposal = propose_on(
            &mut st2,
            request(vec![(transparent_recipient([0x66; 20]), NOTE_VALUE / 3)]),
        )
        .expect("propose");
        let claims = claim_from_proposal(&proposal).expect("claim");
        assert!(crate::intent_store::mark_submitting(&mut aux2, id2, &claims).expect("submit"));
        let row = crate::intent_store::list_in_flight(&aux2).expect("inflight")[0].clone();
        assert_eq!(
            row.deposit_deadline,
            Some(lapsed),
            "the deadline tag rode into the in-flight row",
        );
        let action = reconcile_inflight(st2.wallet(), &mut aux2, &row, now).expect("reconcile");
        assert_eq!(
            action,
            Reconciled::DepositExpired,
            "a lapsed deposit with freed notes ⇒ delete, never re-queue to a dead quote",
        );
        assert!(
            crate::intent_store::list_queued(&aux2)
                .expect("q")
                .is_empty()
                && crate::intent_store::list_in_flight(&aux2)
                    .expect("f")
                    .is_empty(),
            "deleted, not re-queued",
        );
    }

    #[test]
    fn deposit_send_within_deadline_is_proposed_and_sent() {
        // A swap deposit whose quote is STILL LIVE drives Queued→Sent EXACTLY like an ordinary
        // send — the deadline tag excludes only a lapsed quote, never a live one. Proves the
        // deposit path reuses the whole §6.3-guarded send unchanged, and the tag is preserved.
        let now = 2_000_000_000u64;
        let live = (now + 10_000) as i64; // well beyond the safety margin
        let mut st = funded(NOTE_VALUE);
        let mut aux = aux_conn();
        let id = crate::intent_store::enqueue(&mut aux, "zcash:deposit", 1, Some(live), None)
            .expect("enqueue");
        let req = request(vec![(transparent_recipient([0x77; 20]), NOTE_VALUE / 3)]);
        let network = *st.network();
        let aid = st.test_account().expect("acct").id();
        let usk = st.test_account().expect("acct").usk().clone();
        let prover = crate::prover::tx_prover();
        let prepared = prepare_queued(
            st.wallet_mut(),
            &network,
            &mut aux,
            aid,
            req,
            usk,
            prover,
            prover,
            id,
            Some(live),
            0,
            now,
            crate::consensus::SigningPermit::assume_current_for_test(),
        )
        .expect("prepare");
        assert!(
            matches!(prepared, Prepared::Broadcast(ref raw) if !raw.is_empty()),
            "a live-quote deposit sends like a normal send, got {prepared:?}",
        );
        let row = crate::intent_store::list_in_flight(&aux).expect("inflight");
        assert_eq!(row.len(), 1);
        assert_eq!(
            row[0].state,
            crate::intent_store::IntentState::Sent,
            "Queued→Sent — a live deposit is not excluded",
        );
        assert_eq!(
            row[0].deposit_deadline,
            Some(live),
            "the deadline tag is preserved through the send (re-broadcast still respects it)",
        );
    }

    #[test]
    fn deposit_send_waits_when_the_device_clock_is_unsynced() {
        // The §4.4 mobile boot-before-NTP edge (security M-1): on an UNSYNCED wall clock a deposit
        // is NEVER fed (we can't verify the quote is live) and NEVER deleted (we won't drop the
        // user's intent by a clock that says 1970) — it WAITS, left Queued, retried once the clock
        // syncs. Drive `prepare_queued` with a sub-floor `now`; the intent must be untouched.
        let unsynced = CLOCK_PLAUSIBILITY_FLOOR_SECS - 1; // a just-booted phone, pre-NTP
        let deadline = 1_900_000_000i64; // a real future deadline — but untimeable by this clock
        let mut st = funded(NOTE_VALUE);
        let mut aux = aux_conn();
        let id = crate::intent_store::enqueue(&mut aux, "zcash:deposit", 1, Some(deadline), None)
            .expect("enqueue");
        let req = request(vec![(transparent_recipient([0x88; 20]), NOTE_VALUE / 3)]);
        let network = *st.network();
        let aid = st.test_account().expect("acct").id();
        let usk = st.test_account().expect("acct").usk().clone();
        let prover = crate::prover::tx_prover();
        let prepared = prepare_queued(
            st.wallet_mut(),
            &network,
            &mut aux,
            aid,
            req,
            usk,
            prover,
            prover,
            id,
            Some(deadline),
            0,
            unsynced,
            crate::consensus::SigningPermit::assume_current_for_test(),
        )
        .expect("prepare");
        assert_eq!(
            prepared,
            Prepared::Retry,
            "an unsynced clock ⇒ wait (Retry), never feed or delete a deposit",
        );
        let queued = crate::intent_store::list_queued(&aux).expect("q");
        assert_eq!(
            queued.len(),
            1,
            "the deposit is STILL Queued — neither sent nor deleted"
        );
        assert_eq!(queued[0].id, id);
        assert!(
            crate::intent_store::list_in_flight(&aux)
                .expect("f")
                .is_empty(),
            "nothing advanced to Submitting/Sent on an untimeable clock",
        );
    }

    #[test]
    fn deposit_send_already_sent_past_deadline_holds_broadcast_never_deleted() {
        // THE Sent-row preservation invariant, proven through the funded harness. HISTORY: this
        // test originally pinned the OPPOSITE broadcast behavior ("keep re-broadcasting — the
        // refund path"), written when sign+broadcast happened in ONE gated pass, so this arm
        // could only ever RE-push a tx that was already public while its quote was live.
        // W-swap-4-a-2 (HIGH-2) inverted that premise: post-FR-23-a the sign (execute) and
        // the FIRST broadcast (kick/drain) are decoupled and the row cannot say whether its tx
        // was ever handed to the network — so this arm could FIRST-feed a never-broadcast
        // deposit into a DEAD quote (a fee-burning provider refund round-trip). The gate now
        // HOLDS the network tail past the deadline: an already-public tx needs no re-push from
        // us to resolve (mempools/miners have it; if it mines, the mined+buried terminal below
        // still deletes on schedule — the provider refund lands regardless), and a never-public
        // one must not be fed. What this test STILL guards with its life: the hold NEVER
        // deletes/resets the Sent row (the original money fear — orphaning funds that already
        // moved), the recorded txid stays stable, and the §4.4 `DepositExpired` delete stays
        // confined to the FRESH re-propose arm.
        let now = 2_000_000_000u64;
        let live = (now + 10_000) as i64; // alive when the deposit was created…

        // Drive a LIVE deposit Queued→Sent (the tx is created, the funds left, the txid recorded).
        let mut st = funded(NOTE_VALUE);
        let mut aux = aux_conn();
        let id = crate::intent_store::enqueue(&mut aux, "zcash:deposit", 1, Some(live), None)
            .expect("enqueue");
        let req = request(vec![(transparent_recipient([0x99; 20]), NOTE_VALUE / 3)]);
        let network = *st.network();
        let aid = st.test_account().expect("acct").id();
        let usk = st.test_account().expect("acct").usk().clone();
        let prover = crate::prover::tx_prover();
        let prepared = prepare_queued(
            st.wallet_mut(),
            &network,
            &mut aux,
            aid,
            req,
            usk,
            prover,
            prover,
            id,
            Some(live),
            0,
            now,
            crate::consensus::SigningPermit::assume_current_for_test(),
        )
        .expect("prepare");
        assert!(
            matches!(prepared, Prepared::Broadcast(ref raw) if !raw.is_empty()),
            "the live deposit is created and broadcast — the ZEC has left, got {prepared:?}",
        );
        let row = crate::intent_store::list_in_flight(&aux).expect("inflight")[0].clone();
        assert_eq!(row.state, crate::intent_store::IntentState::Sent);
        let txid_before = *row.txids.last().expect("txid recorded");

        // …NOW the clock advances PAST the deadline. The deposit's tx is still unmined.
        let lapsed_now = (live + DEPOSIT_FIRST_FEED_MARGIN_SECS as i64 + 100) as u64;
        assert_eq!(
            deposit_gate(Some(live), lapsed_now),
            DepositGate::Expired,
            "by `lapsed_now` the quote deadline has indeed lapsed (precondition)",
        );

        // Reconcile the in-flight Sent deposit with the lapsed clock: the broadcast tail HOLDS
        // (W-swap-4-a-2 rule 2 — never a first feed of a dead quote), NOT delete and NOT
        // DepositExpired — the §4.4 delete is reached only in the ReProposeFresh arm, which a
        // Sent (notes-spent) row never enters.
        let action =
            reconcile_inflight(st.wallet(), &mut aux, &row, lapsed_now).expect("reconcile");
        assert_eq!(
            action,
            Reconciled::DepositBroadcastHold,
            "a Sent deposit past its deadline HOLDS its broadcast — never fed to a dead quote",
        );
        let after = crate::intent_store::list_in_flight(&aux).expect("inflight");
        assert_eq!(after.len(), 1, "the in-flight deposit row is NEVER deleted");
        assert_eq!(
            after[0].txids,
            vec![txid_before],
            "the recorded txid is stable — no re-create, no reset, the funds stay accounted for",
        );
        assert_eq!(
            after[0].state,
            crate::intent_store::IntentState::Sent,
            "the deposit stays Sent — the hold gates ONLY the network tail, never the row",
        );

        // And once that tx MINES AND BURIES (the swap or its refund lands and is reorg-final), the
        // deadline STILL never touches the reconcile decisions — the §6.3 delete-on-mined cleanup
        // (round-2 #6 burial) runs exactly as for any ordinary send.
        let (mined_h, _) = st.generate_next_block_including(TxId::from_bytes(txid_before));
        st.scan_cached_blocks(mined_h, 1);
        let buried_tip = BlockHeight::from_u32(u32::from(mined_h) + REORG_MAX_BLOCKS);
        st.wallet_mut()
            .update_chain_tip(buried_tip)
            .expect("update tip a reorg-depth past the mined block");
        let mined_row = crate::intent_store::list_in_flight(&aux).expect("inflight")[0].clone();
        assert_eq!(
            reconcile_inflight(st.wallet(), &mut aux, &mined_row, lapsed_now).expect("reconcile"),
            Reconciled::Deleted,
            "a MINED+buried deposit is delete-on-mined like any send — the lapsed deadline is irrelevant",
        );
        assert!(
            crate::intent_store::list_in_flight(&aux)
                .expect("gone")
                .is_empty(),
            "the row is cleaned up once its deposit tx mines",
        );
    }

    #[test]
    fn deposit_send_waits_on_unsynced_clock_then_self_heals_when_it_syncs() {
        // THE Wait→self-heal loop end-to-end (the property that makes "boot before NTP doesn't lose
        // your swap" true). Pass 1: an unsynced wall clock can't time-check the deadline, so the
        // deposit WAITS (Retry, still Queued — never fed, never deleted). Pass 2: NTP has synced and
        // the deadline is STILL live, so the SAME intent now drives Queued→Sent. Existing tests prove
        // each half over independent fixtures (the single-pass Wait, the single-pass live send);
        // nothing proves the SAME deposit survives the unsynced pass and then completes once the
        // clock is trustworthy. A regression that, on `Wait`, mutated the row (deleted or marked it)
        // would break the self-heal and slip CI today.
        let unsynced = CLOCK_PLAUSIBILITY_FLOOR_SECS - 1; // pass 1: a just-booted phone, pre-NTP
        let synced = 2_000_000_000u64; // pass 2: NTP arrived — a trustworthy clock
        let live = (synced + 10_000) as i64; // a deadline still well in the future once synced

        let mut st = funded(NOTE_VALUE);
        let mut aux = aux_conn();
        let id = crate::intent_store::enqueue(&mut aux, "zcash:deposit", 1, Some(live), None)
            .expect("enqueue");
        let to = transparent_recipient([0xaa; 20]);
        let network = *st.network();
        let aid = st.test_account().expect("acct").id();
        let prover = crate::prover::tx_prover();

        // Pass 1 — unsynced clock: WAIT. The intent is left exactly as it was (Queued, deadline tag
        // intact), and crucially nothing is proposed/signed/spent.
        let usk1 = st.test_account().expect("acct").usk().clone();
        let pass1 = prepare_queued(
            st.wallet_mut(),
            &network,
            &mut aux,
            aid,
            request(vec![(to.clone(), NOTE_VALUE / 3)]),
            usk1,
            prover,
            prover,
            id,
            Some(live),
            0,
            unsynced,
            crate::consensus::SigningPermit::assume_current_for_test(),
        )
        .expect("pass 1");
        assert_eq!(pass1, Prepared::Retry, "pass 1 on an unsynced clock WAITS");
        let queued = crate::intent_store::list_queued(&aux).expect("q");
        assert_eq!(queued.len(), 1, "still queued after the unsynced pass");
        assert_eq!(queued[0].id, id, "the SAME intent, untouched");
        assert_eq!(
            queued[0].deposit_deadline,
            Some(live),
            "the deadline tag is preserved across the unsynced pass — nothing was rewritten",
        );

        // Pass 2 — the clock has synced and the deadline is still live: the SAME intent completes.
        let usk2 = st.test_account().expect("acct").usk().clone();
        let pass2 = prepare_queued(
            st.wallet_mut(),
            &network,
            &mut aux,
            aid,
            request(vec![(to, NOTE_VALUE / 3)]),
            usk2,
            prover,
            prover,
            id,
            Some(live),
            0,
            synced,
            crate::consensus::SigningPermit::assume_current_for_test(),
        )
        .expect("pass 2");
        assert!(
            matches!(pass2, Prepared::Broadcast(ref raw) if !raw.is_empty()),
            "once the clock syncs the waiting deposit self-heals and sends, got {pass2:?}",
        );
        let in_flight = crate::intent_store::list_in_flight(&aux).expect("inflight");
        assert_eq!(
            in_flight.len(),
            1,
            "the deposit advanced after the clock synced"
        );
        assert_eq!(
            in_flight[0].id, id,
            "the SAME intent that waited now carries the tx"
        );
        assert_eq!(in_flight[0].state, crate::intent_store::IntentState::Sent);
        assert!(
            crate::intent_store::list_queued(&aux)
                .expect("q")
                .is_empty(),
            "the queue drained — the boot-before-NTP deposit self-healed to completion",
        );
    }

    #[test]
    fn deposit_delete_in_a_pass_leaves_an_ordinary_queued_send_untouched() {
        // ISOLATION across a single resubmission pass: a lapsed-deposit DELETE and an ordinary
        // (no-deadline) queued send coexist. Walking them in one pass, the deadline gate must
        // terminally delete ONLY the deposit and the ordinary send must still drive Queued→Sent
        // exactly as if the deposit were never there — the delete must not perturb the other
        // intent's row, claim, or spend. Every existing two-intent test uses two ORDINARY sends;
        // nothing proves a deposit-delete is surgically scoped. A regression that deleted by id-range
        // / cleared shared state, or let the deposit's gate short-circuit the whole pass, would slip
        // CI today.
        let now = 2_000_000_000u64;
        let lapsed = (now - 100) as i64; // a dead quote
        let mut st = funded(NOTE_VALUE); // ONE spendable note — the ordinary send must get it
        let mut aux = aux_conn();
        let network = *st.network();
        let aid = st.test_account().expect("acct").id();
        let prover = crate::prover::tx_prover();

        // A lapsed deposit (FIFO-first) + an ordinary queued send (no deadline) over the one note.
        let id_dep =
            crate::intent_store::enqueue(&mut aux, "zcash:dead-deposit", 1, Some(lapsed), None)
                .expect("enqueue deposit");
        let id_ord = crate::intent_store::enqueue(&mut aux, "zcash:ordinary", 2, None, None)
            .expect("enqueue ordinary");

        // Pass over the deposit FIRST: its lapsed deadline DELETES it without proposing — no note is
        // touched, so the ordinary send's note stays fully available.
        let usk_dep = st.test_account().expect("acct").usk().clone();
        let dep = prepare_queued(
            st.wallet_mut(),
            &network,
            &mut aux,
            aid,
            request(vec![(transparent_recipient([0xbb; 20]), NOTE_VALUE / 3)]),
            usk_dep,
            prover,
            prover,
            id_dep,
            Some(lapsed),
            0,
            now,
            crate::consensus::SigningPermit::assume_current_for_test(),
        )
        .expect("prepare deposit");
        assert_eq!(
            dep,
            Prepared::DepositExpired,
            "the dead-quote deposit is deleted, never proposed",
        );

        // The ordinary send in the SAME pass: no deadline ⇒ Proceed ⇒ it claims the note and sends.
        // The deposit's delete neither consumed the note nor disturbed this row.
        let usk_ord = st.test_account().expect("acct").usk().clone();
        let ord = prepare_queued(
            st.wallet_mut(),
            &network,
            &mut aux,
            aid,
            request(vec![(sapling_recipient(b"ordinary-dest"), NOTE_VALUE / 3)]),
            usk_ord,
            prover,
            prover,
            id_ord,
            None,
            0,
            now,
            crate::consensus::SigningPermit::assume_current_for_test(),
        )
        .expect("prepare ordinary");
        assert!(
            matches!(ord, Prepared::Broadcast(ref raw) if !raw.is_empty()),
            "the ordinary send is unperturbed by the deposit delete and sends, got {ord:?}",
        );

        // Exactly the ordinary send is in flight; the deposit row is gone; nothing is left queued.
        let in_flight = crate::intent_store::list_in_flight(&aux).expect("inflight");
        assert_eq!(in_flight.len(), 1, "only the ordinary send advanced");
        assert_eq!(
            in_flight[0].id, id_ord,
            "and it is the ordinary send, not the deposit"
        );
        assert_eq!(in_flight[0].state, crate::intent_store::IntentState::Sent);
        assert_eq!(
            in_flight[0].deposit_deadline, None,
            "the surviving intent is the no-deadline ordinary send",
        );
        assert!(
            crate::intent_store::list_queued(&aux)
                .expect("q")
                .is_empty(),
            "nothing left queued — the deposit deleted, the ordinary send sent",
        );
        assert!(
            crate::intent_store::load(&aux, id_dep)
                .expect("load")
                .is_none(),
            "the deposit row was surgically removed (only that id)",
        );
    }

    // REORG coverage (the witness's conservative-false direction under a rewind that un-mines a
    // claimed note) is OWED to the GA reorg e2e: the `data_api::testing` MemoryShardStore rejects
    // `truncate_to_height` even at its own reported `safe_rewind_height` (checkpoint-boundary
    // limited), so a clean funding-rewind unit test is impractical without deeper fixture work,
    // and the spec already defers reorg testing to the GA e2e (§3.2g/§8). The conservative
    // direction itself is designed-for (documented on `note_spendable`: every false-positive
    // direction yields `false` ⇒ WAIT, never a re-propose) and was verified against the upstream
    // `get_spendable_note` SQL (`t.block IS NOT NULL` on the producing tx) in the S69 money pass.
    // ORCHARD end-to-end witness coverage is likewise OWED (test review Rank 1): the harness funds
    // sapling notes, and an orchard-funded harness needs an NU5-seeded `with_initial_chain_state`
    // fixture — a bounded but real cost ticketed for 3-b-ii-B / a fixture session. The Orchard tag
    // mapping IS covered (`protocol_tag_round_trips_and_rejects_unknown`).
    //
    // `reconcile_inflight`'s `Reconciled::Awaiting` arm has TWO entry paths: (a) a `Submitting` row
    // whose txid was never recorded (spent + no txid ⇒ AwaitWitness) — covered by the pure-decision
    // `submitting_intent_awaits_witness_then_re_proposes_after_expiry`; and (b) a `ReBroadcast` whose
    // recorded tx was PRUNED from the store (`read_raw_tx` miss — a spent note whose live spending tx
    // is gone) ⇒ WAIT, never a re-queue (a re-queue could double-spend a still-live tx). Path (b) is
    // covered at the pure level (the `reconcile` matrix + the documented invariant on
    // `reconcile_inflight`) but NOT yet through the funded harness — the `data_api::testing` `WalletDb`
    // exposes no "prune a persisted tx" seam, so forcing the miss needs the same fixture work as the
    // reorg cases above; OWED alongside them (code reviewer S70). A miss here is near-impossible in
    // practice (a live unmined spending tx is not pruned), so the risk is defensive, not operational.

    /// **INC-018 (a): a refusal on the money path is never silent.** `map_propose_err`'s
    /// catch-all arm — the one the device proof took over a VALID self-send,
    /// printing "check the details" and logging nothing — now emits one payload-free
    /// event naming the upstream VARIANT. Driven with a unit variant the mapper does
    /// not name (`ProposalNotSupported`), against the funded harness's own store so the
    /// signature under test is the production one.
    ///
    /// Two clauses: the event exists with the static `outcome` and the variant's
    /// static name, and the captured fields pass `assert_5_4_clean` — the line may
    /// say WHICH arm refused and nothing else (no selector message, no amount, no
    /// address). The mapped error is unchanged (`ProposeFailed`): what the user reads
    /// is INC-018 (b)'s item, phase-2 P2-2.
    ///
    /// Mutant: delete the `tracing::warn!` at the catch-all → the first assertion
    /// finds no event. Sibling mutant: replace `propose_err_variant(&other)` with a
    /// literal → the variant clause reds.
    #[tokio::test]
    async fn an_unmapped_propose_refusal_is_logged_by_variant_and_payload_free() {
        use crate::tracing_guard::{
            CaptureLayer, CapturedEvents, assert_5_4_clean, force_wallet_callsites_enabled,
        };
        use tracing_subscriber::layer::SubscriberExt;
        use zcash_client_backend::data_api::error::Error as ProposeError;
        force_wallet_callsites_enabled();
        let sink = CapturedEvents::default();
        let subscriber = tracing_subscriber::registry().with(CaptureLayer::new(sink.clone()));
        let _guard = tracing::subscriber::set_default(subscriber);

        let st = fresh_state();
        let e: ProposeError<Infallible, Infallible, Infallible, Infallible, Infallible, ()> =
            ProposeError::ProposalNotSupported;
        let mapped = map_propose_err(st.wallet(), e);
        assert!(
            matches!(mapped, WalletError::ProposeFailed),
            "the catch-all still maps to the generic refusal — this item changes what is \
             SAID, not what is refused. Got {mapped:?}"
        );

        let fields = sink.fields();
        assert!(
            fields
                .iter()
                .any(|(n, v)| n == "outcome" && v == "propose_refused_unmapped"),
            "INC-018 (a): the catch-all must emit an event with the static outcome — \
             a refusal on the money path was silent once and it is not allowed to be \
             again. Captured: {fields:?}"
        );
        assert!(
            fields
                .iter()
                .any(|(n, v)| n == "code" && v == "proposal_not_supported"),
            "the event names the upstream VARIANT on the allowlisted `code` field (§5.4: \
             a stable code, never a payload), so the log says which arm refused. \
             Captured: {fields:?}"
        );
        assert_5_4_clean(&fields);
    }

    /// **INC-018 (b), phase-2 P2-2 (maintainer decision 4): the retryable
    /// refusal is typed apart from the user-error one, so the copy stops blaming
    /// details that are correct.** Drives one variant of each class through the
    /// production `map_propose_err` against the funded harness's store, under the
    /// capture layer:
    ///
    /// 1. `CommitmentTree(Query(CheckpointPruned))` — the witness-held shape of the
    ///    device proof — maps to `ProposeTransient` ("try again in a
    ///    moment"), and its event is `outcome = "propose_refused_transient"` with
    ///    the variant's static name on `code`, §5.4-clean.
    /// 2. `ProposalNotSupported` — deterministic on the input — still maps to
    ///    `ProposeFailed` ("check the details"), and its event is the (a) shape
    ///    `outcome = "propose_refused_unmapped"`, §5.4-clean.
    /// 3. The wildcard is NOT transient: `Proposal(TransactionTooLarge { .. })`, a
    ///    structural member the classifier does not name, maps to `ProposeFailed`.
    ///
    /// Each class has its own mutant (registry rows): M1 the transient arm
    /// collapsed (`propose_refusal_is_transient` returns `false` for
    /// `CommitmentTree`) reds clause 1 — the incident's own shape, a witness-held
    /// send told to check its details; M2 the deterministic class widened (the
    /// predicate returns `true` for `ProposalNotSupported`) reds clause 2 — a user
    /// error told to wait. The copy each class reaches is pinned on the Dart side
    /// (`send_state_test.dart`'s classify table and `form_fault_view_test.dart`).
    #[tokio::test]
    async fn a_transient_propose_refusal_is_typed_retryable_and_a_user_error_is_not() {
        use crate::tracing_guard::{
            CaptureLayer, CapturedEvents, assert_5_4_clean, force_wallet_callsites_enabled,
        };
        use shardtree::error::{QueryError, ShardTreeError};
        use tracing_subscriber::layer::SubscriberExt;
        use zcash_client_backend::data_api::error::Error as ProposeError;
        use zcash_client_backend::proposal::ProposalError;
        type E = ProposeError<Infallible, Infallible, Infallible, Infallible, Infallible, ()>;
        force_wallet_callsites_enabled();
        let sink = CapturedEvents::default();
        let subscriber = tracing_subscriber::registry().with(CaptureLayer::new(sink.clone()));
        let _guard = tracing::subscriber::set_default(subscriber);
        let st = fresh_state();
        // The LAST `wallet.propose_refused` record must carry BOTH fields — a
        // per-record check, not two existence checks over the flat field list
        // (the Batch C security pass's row 7: `has(outcome) && has(code)` over
        // `fields()` would pass if the two came from different events).
        let last_refusal = |outcome: &str, code: &str| {
            let records = sink.records_of("wallet.propose_refused");
            let last = records.last().cloned().unwrap_or_default();
            let ok = last.iter().any(|(n, v)| n == "outcome" && v == outcome)
                && last.iter().any(|(n, v)| n == "code" && v == code);
            (ok, last)
        };

        // Clause 1 — the transient class, driven with a variant the class NAMES;
        // whether the propose path can PRODUCE it at this pin is the predicate's
        // doc and the registry row, not this test (it cannot: see there).
        let witness_held: E =
            ProposeError::CommitmentTree(ShardTreeError::Query(QueryError::CheckpointPruned));
        let mapped = map_propose_err(st.wallet(), witness_held);
        assert!(
            matches!(mapped, WalletError::ProposeTransient),
            "INC-018 (b): a refusal the wallet's own state clears is the typed retryable \
             class, so the host says \"try again in a moment\" and never \"check the \
             details\". Got {mapped:?}"
        );
        let (ok, record) = last_refusal("propose_refused_transient", "commitment_tree");
        assert!(
            ok,
            "the transient arm logs ONE event carrying its own outcome AND the variant's \
             static name — a field log tells the two classes apart without a lookup. Last \
             record: {record:?}"
        );
        assert_5_4_clean(&sink.fields());

        // Clause 2 — the deterministic class, unchanged by (b).
        let user_error: E = ProposeError::ProposalNotSupported;
        let mapped = map_propose_err(st.wallet(), user_error);
        assert!(
            matches!(mapped, WalletError::ProposeFailed),
            "a request this build cannot express is deterministic on its input: the same \
             payment re-fails unchanged, so \"check the details\" stays its copy. Got {mapped:?}"
        );
        let (ok, record) = last_refusal("propose_refused_unmapped", "proposal_not_supported");
        assert!(
            ok,
            "the (a) event still names the variant on the deterministic arm, in ONE record. \
             Last record: {record:?}"
        );
        assert_5_4_clean(&sink.fields());

        // Clause 3 — the wildcard is not transient: a structural `Proposal` member
        // the classifier does not name lands as deterministic, never as "wait".
        let too_large: E = ProposeError::Proposal(ProposalError::TransactionTooLarge {
            estimated_size: 2_000_001,
            limit: 2_000_000,
            sapling_input_count: 0,
            orchard_input_count: 0,
            ironwood_input_count: 0,
        });
        let mapped = map_propose_err(st.wallet(), too_large);
        assert!(
            matches!(mapped, WalletError::ProposeFailed),
            "an unclassified structural shape fails to the deterministic class — \"try \
             again in a moment\" for a shape nobody has read could loop a user forever. \
             Got {mapped:?}"
        );
    }

    /// Phase-3 P3-2: every string `propose_err_variant` returns is a member of the
    /// closed static set `PROPOSE_ERR_VARIANT_CODES` the §5.4 scanner exempts on
    /// `code`. Driven with every variant this test can construct without an
    /// upstream payload type (the unit variants, the two address shapes, a
    /// `Proposal` member, an expiry shape); the payload-carrying arms are read,
    /// not driven, and the doc on the const says the two move together.
    /// Mutant (registry row): a member removed from the const → the membership
    /// assertion reds on that arm's name.
    #[test]
    fn propose_err_variant_names_are_all_in_the_static_set() {
        use zcash_address::ConversionError;
        use zcash_client_backend::data_api::error::Error as ProposeError;
        use zcash_client_backend::proposal::ProposalError;
        use zcash_protocol::consensus::{BlockHeight, NetworkType};
        type E = ProposeError<Infallible, Infallible, Infallible, Infallible, Infallible, ()>;
        let driven: Vec<E> = vec![
            ProposeError::ProposalNotSupported,
            ProposeError::AccountIdNotRecognized,
            ProposeError::KeyNotRecognized,
            ProposeError::AccountCannotSpend,
            ProposeError::ScanRequired,
            ProposeError::Address(ConversionError::IncorrectNetwork {
                expected: NetworkType::Main,
                actual: NetworkType::Test,
            }),
            ProposeError::Proposal(ProposalError::AnchorNotFound(BlockHeight::from_u32(1))),
            ProposeError::ExpiryHeightBelowTargetHeight {
                expiry_height: BlockHeight::from_u32(1),
                min_target_height: BlockHeight::from_u32(2),
            },
        ];
        assert!(driven.len() >= 8, "harness: the driven set is not empty");
        for e in &driven {
            let name = propose_err_variant(e);
            assert!(
                PROPOSE_ERR_VARIANT_CODES.contains(&name),
                "`propose_err_variant` returned {name:?}, which is not in \
                 PROPOSE_ERR_VARIANT_CODES — the match and the const moved apart"
            );
        }
        assert_eq!(
            propose_err_variant(&driven[5]),
            "address",
            "the address arm is the one the §5.4 exemption exists for"
        );
    }

    /// Phase-3 P3-2 (the Batch C code reviewer's MEDIUM): a propose refusal whose
    /// variant NAME carries a forbidden substring — `Error::Address(_)` logs
    /// `code = "address"` — is §5.4-clean on the production event, because the
    /// scanner exempts the closed static set on `code`. Before the exemption this
    /// test tripped on a benign static label: a gate/production mismatch, not a
    /// leak. Mutant (registry row): the `static_variant_code` exemption removed
    /// from `tracing_guard::first_violation` → `assert_5_4_clean` reds with
    /// "forbidden token \"address\" in field value: code=address".
    #[tokio::test]
    async fn an_address_shaped_variant_name_on_code_is_5_4_clean() {
        use crate::tracing_guard::{
            CaptureLayer, CapturedEvents, assert_5_4_clean, force_wallet_callsites_enabled,
        };
        use tracing_subscriber::layer::SubscriberExt;
        use zcash_address::ConversionError;
        use zcash_client_backend::data_api::error::Error as ProposeError;
        use zcash_protocol::consensus::NetworkType;
        type E = ProposeError<Infallible, Infallible, Infallible, Infallible, Infallible, ()>;
        force_wallet_callsites_enabled();
        let sink = CapturedEvents::default();
        let subscriber = tracing_subscriber::registry().with(CaptureLayer::new(sink.clone()));
        let _guard = tracing::subscriber::set_default(subscriber);
        let st = fresh_state();

        let wrong_network: E = ProposeError::Address(ConversionError::IncorrectNetwork {
            expected: NetworkType::Main,
            actual: NetworkType::Test,
        });
        let mapped = map_propose_err(st.wallet(), wrong_network);
        assert!(
            matches!(mapped, WalletError::ProposeFailed),
            "an address for the wrong network is deterministic on the input. Got {mapped:?}"
        );
        let records = sink.records_of("wallet.propose_refused");
        let last = records.last().cloned().unwrap_or_default();
        assert!(
            last.iter().any(|(n, v)| n == "code" && v == "address"),
            "the event names the variant on `code`: {last:?}"
        );
        assert_5_4_clean(&sink.fields());
    }

    // ── S7 C1: a recorded group is never re-proposed at BARE expiry ─────────────────
    //
    // The engine frees a claim note at `tip == expiry` (no burial margin), so the witness reads
    // "spendable" while the old signed group is still valid wherever it was retained — and a
    // reorg back below the expiry could mine it beside a fresh re-proposal (the recipient paid
    // twice). These drive the PRODUCTION `reconcile_inflight` (witness and all) over a `Sent`
    // row at bare expiry: the row must WAIT for burial with no network, then requeue.

    /// Record `group` (created from `claims`) on a fresh aux store as a `Sent` row.
    fn c1_sent_row(claims: &[NoteClaim], group: &[[u8; 32]]) -> (Connection, QueuedSendId) {
        let mut aux = aux_conn();
        let id =
            crate::intent_store::enqueue(&mut aux, "zcash:c1", 0, None, None).expect("enqueue");
        assert!(crate::intent_store::mark_submitting(&mut aux, id, claims).expect("submit"));
        assert!(crate::intent_store::mark_sent_multi(&mut aux, id, group).expect("sent"));
        (aux, id)
    }

    /// Reconcile the ONE in-flight row through the production entry point.
    fn c1_reconcile(st: &HarnessState, aux: &mut Connection, now_unix: u64) -> Reconciled {
        let row = crate::intent_store::list_in_flight(aux).expect("in flight")[0].clone();
        reconcile_inflight(st.wallet(), aux, &row, now_unix).expect("reconcile")
    }

    #[test]
    fn a_sent_single_step_at_bare_expiry_waits_for_burial_before_it_requeues() {
        let mut st = funded(NOTE_VALUE);
        let proposal = propose_on(
            &mut st,
            request(vec![(sapling_recipient(b"c1-single"), NOTE_VALUE / 3)]),
        )
        .expect("propose");
        let claims = claim_from_proposal(&proposal).expect("claim");
        let txids = sign_on(&mut st, &proposal).expect("create");
        let group: Vec<[u8; 32]> = txids.iter().map(|t| *t.as_ref()).collect();
        let expiry = created_tx_expiry(&st, txids[0]);
        let (mut aux, _id) = c1_sent_row(&claims, &group);

        // BARE expiry: the engine has freed the claim notes (the precondition that made the
        // old witness-first order re-propose here).
        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(expiry))
            .expect("tip at the expiry");
        assert!(
            claims
                .iter()
                .all(|c| note_spendable(st.wallet(), c).expect("witness")),
            "precondition: at bare expiry the witness reads every claim note spendable"
        );
        let bare = c1_reconcile(&st, &mut aux, 0);
        assert_eq!(
            bare,
            Reconciled::AwaitingBurial,
            "a Sent row at bare expiry waits — never a re-propose beside a revivable signed tx"
        );
        assert!(
            crate::intent_store::list_queued(&aux)
                .expect("queued")
                .is_empty(),
            "not requeued at bare expiry"
        );

        // One block short of burial: still waiting.
        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(expiry + REORG_MAX_BLOCKS - 1))
            .expect("tip one block short of burial");
        assert_eq!(c1_reconcile(&st, &mut aux, 0), Reconciled::AwaitingBurial);

        // Buried: the group is dead whole across any auto-recoverable reorg — requeue.
        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(expiry + REORG_MAX_BLOCKS))
            .expect("tip at burial");
        assert_eq!(c1_reconcile(&st, &mut aux, 0), Reconciled::Requeued);
        assert_eq!(
            crate::intent_store::list_queued(&aux)
                .expect("queued")
                .len(),
            1,
            "requeued once the expiry is buried"
        );
    }

    #[test]
    fn a_sent_tex_two_step_at_bare_expiry_waits_for_burial_before_it_requeues() {
        let mut st = funded(NOTE_VALUE);
        let (mut aux, _id, group) = tex_two_step_sent_row(&mut st);
        let claims = crate::intent_store::list_in_flight(&aux).expect("in flight")[0]
            .claims
            .clone();
        let expiry0 = created_tx_expiry(&st, TxId::from_bytes(group[0]));
        let expiry1 = created_tx_expiry(&st, TxId::from_bytes(group[1]));
        let dead = expiry0.max(expiry1);

        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(dead))
            .expect("tip at the later expiry");
        assert!(
            claims
                .iter()
                .all(|c| note_spendable(st.wallet(), c).expect("witness")),
            "precondition: both legs expired, the witness reads step0's claims spendable"
        );
        assert_eq!(
            c1_reconcile(&st, &mut aux, 0),
            Reconciled::AwaitingBurial,
            "a Sent two-step with both legs at bare expiry waits — never a re-propose"
        );

        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(dead + REORG_MAX_BLOCKS))
            .expect("tip at burial of the later expiry");
        assert_eq!(c1_reconcile(&st, &mut aux, 0), Reconciled::Requeued);
    }

    #[test]
    fn a_lapsed_sent_deposit_at_bare_expiry_holds_then_requeues_into_the_purge_at_burial() {
        let mut st = funded(NOTE_VALUE);
        let deadline: i64 = 1_800_000_000;
        let lapsed_now = 1_900_000_000u64;
        let (mut aux, _id, group) = deposit_sent_row(&mut st, deadline);
        let expiry = created_tx_expiry(&st, TxId::from_bytes(group[0]));

        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(expiry))
            .expect("tip at the expiry");
        assert_eq!(
            c1_reconcile(&st, &mut aux, lapsed_now),
            Reconciled::DepositBroadcastHold,
            "a Sent lapsed deposit at bare expiry is held — not deleted on a freed witness"
        );
        assert_eq!(
            crate::intent_store::list_in_flight(&aux)
                .expect("in flight")
                .len(),
            1,
            "the row stays Sent (the one-deposit guard holds until burial)"
        );

        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(expiry + REORG_MAX_BLOCKS))
            .expect("tip at burial");
        assert_eq!(
            c1_reconcile(&st, &mut aux, lapsed_now),
            Reconciled::Requeued
        );
        assert_eq!(
            purge_lapsed_queued_deposits(&mut aux, lapsed_now).expect("purge"),
            1,
            "the requeued lapsed deposit is purged, never re-proposed to a dead quote"
        );
    }
}
