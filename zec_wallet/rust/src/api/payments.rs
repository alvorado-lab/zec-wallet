//! ZIP-321 payment-URI surface (spec §2.4/§3.1): parse for the QR/link flow,
//! encode for receive-request composition. Both are PURE string↔DTO codecs —
//! no storage, no network, no key material — hence `#[frb(sync)]`.
//!
//! LOSSLESS-TOKEN CONTRACT: the URI **string** is the canonical, lossless
//! form of a request. [`ParsedPayment`] is the DISPLAY projection — a
//! machine memo crosses as presence + length only (raw bytes never cross
//! this bridge; the api-surface policy bans byte buffers). An app that
//! later sends a parsed request keeps the original URI as its token.
//!
//! Composition ([`PaymentDraft`]) takes EITHER a text memo or opaque machine
//! bytes (FR-28). The two directions are deliberately asymmetric, and the
//! asymmetry is the point: the parse direction is a DISPLAY projection, so
//! machine bytes never come back out here (a display layer has no use for
//! them and they are hostile on-chain data — principle 7); the compose
//! direction takes them because the bytes are the CALLER's own, and a Dart
//! host could already put arbitrary bytes on-chain by writing its own ZIP-321
//! URI and handing it straight to `propose`. Refusing them here bought no
//! safety and only made the convenience encoder the one path a host could not
//! use.
//!
//! (This note previously said machine memos are "composed host-side in Rust,
//! never from Dart", citing ADR-0530 A6. A6 is nonce hygiene, PARSER
//! obligations and a fuzz target — read verbatim, it says nothing about where
//! composition happens. The rule it was cited for did not exist.)

use flutter_rust_bridge::frb;

use crate::api::config::Network;
use crate::api::error::WalletApiError;

/// Address classification of a parsed recipient. Drives the §5.1 privacy
/// disclosure ("this leg is transparent") and compose affordances.
///
/// SDK upgrades may ADD variants: keep a default arm when switching.
pub enum AddressKind {
    /// Unified address (may bundle several receiver types).
    Unified,
    /// Sapling shielded address.
    Sapling,
    /// Transparent address (incl. TEX): publicly visible on-chain.
    Transparent,
    /// Forward-compatibility arm — render neutrally, never crash.
    Unknown,
}

/// Memo of a parsed payment leg — the display projection (see the module
/// note: the URI string stays the lossless token).
pub enum ParsedMemo {
    /// No memo.
    Empty,
    /// Human-readable UTF-8 memo, carried in full.
    Text { text: String },
    /// Machine-to-machine bytes (ZIP-302 `0xFF` arm). Presence + length
    /// only — render as "binary memo (N bytes)"; the bytes themselves
    /// stay Rust-side.
    Arbitrary { len: u32 },
    /// ZIP-302 reserved framing. NEVER produced by `parsePaymentUri` — the
    /// core's payment-construction gate rejects reserved framing in a send
    /// intent typed; this arm exists for the incoming-transaction lane,
    /// where reserved memos are receivable on-chain reality. Presence +
    /// length only, like `arbitrary`: render as "reserved memo (N bytes)"
    /// and do NOT attempt to decode — it is a wire-format artefact, not a
    /// displayable value.
    Reserved { len: u32 },
    /// Forward-compatibility arm — render neutrally, never crash.
    Unknown,
}

/// One parsed payment leg (display projection of a ZIP-321 request).
pub struct ParsedPayment {
    /// Canonical recipient encoding — already parsed, classified and
    /// network-checked by the core funnel.
    pub recipient_address: String,
    /// Classification for disclosure/affordances.
    pub recipient_kind: AddressKind,
    /// Whether this recipient can receive a memo at all (a transparent-only
    /// unified address exists and cannot).
    pub recipient_memo_capable: bool,
    /// Requested amount in **zatoshis**, exact (max supply < 2^53). `null` =
    /// the requester left the amount to the sender (donation-QR form).
    pub amount_zat: Option<i64>,
    /// The memo, as far as a UI can render it.
    pub memo: ParsedMemo,
    /// ZIP-321 display field — never sent on-chain.
    pub label: Option<String>,
    /// ZIP-321 display field — never sent on-chain.
    pub message: Option<String>,
}

/// One payment leg for COMPOSING a request URI.
pub struct PaymentDraft {
    /// Recipient address; validated (incl. network check) by the core
    /// before anything is encoded.
    pub recipient_address: String,
    /// Amount in **zatoshis**; `null` = let the sender choose (donation
    /// form).
    pub amount_zat: Option<i64>,
    /// Optional human-readable memo. The recipient must be able to receive
    /// memos (typed error otherwise, before any URI exists).
    ///
    /// Mutually exclusive with [`Self::memo_bytes`] — a leg carries ONE
    /// memo, and setting both is a caller bug, refused typed rather than
    /// silently preferring one.
    pub memo_text: Option<String>,
    /// FR-28 — OPAQUE machine-memo bytes (the ZIP-302 `0xFF` arm), for a
    /// host that attaches its own typed envelope to a payment (an invoice
    /// reference, an order id, a chat correlator).
    ///
    /// **The SDK never interprets these bytes.** Their layout is the host's
    /// and stays the host's; nothing here parses, validates or renders
    /// them beyond the ZIP-302 length bound.
    ///
    /// **WIRE REALITY, and a host that ignores it will read garbage back.**
    /// The `0xFF` field carries no length framing: fewer than 511 bytes are
    /// ZERO-PADDED on the wire and come back as the full 511. A host that
    /// needs its exact length must frame it INSIDE the bytes. "Byte-for-byte"
    /// means the leading bytes are exactly what was supplied — not that the
    /// on-chain field is the same length.
    ///
    /// Same recipient rule as [`Self::memo_text`]: a memo needs a shielded
    /// receiver, so a transparent recipient is a typed refusal before any URI
    /// exists.
    pub memo_bytes: Option<Vec<u8>>,
    /// ZIP-321 display field — never sent on-chain.
    pub label: Option<String>,
    /// ZIP-321 display field — never sent on-chain.
    pub message: Option<String>,
}

/// The on-chain value pool a proposal output lands in — the §5.1 de-shield
/// disclosure primitive: `transparent` ⇒ funds LEAVE the shielded set (the host
/// must surface it before the user signs). This is the AUDITED engine's own
/// per-output verdict (`Proposal::payment_pools`), not a guess re-derived from
/// the recipient address.
///
/// SDK upgrades may ADD variants: keep a default arm when switching.
pub enum OutputPool {
    /// Publicly visible on-chain — a de-shield.
    Transparent,
    /// Sapling shielded pool.
    Sapling,
    /// Orchard shielded pool.
    Orchard,
    /// Ironwood shielded pool (NU6.3). After the activation the Orchard turnstile
    /// forbids ADDING value to Orchard, so every payment to an Orchard receiver is
    /// delivered in the Ironwood bundle — this is the ORDINARY pool for a shielded
    /// send, not an exotic one. Shielded, so it is never a de-shield disclosure.
    Ironwood,
    /// Forward-compatibility arm — render neutrally, never crash.
    Unknown,
}

/// Why a proposal tripped the large-amount confirm (the §3 money-safety check). The host
/// shows ONE deliberate confirm before signing when [SendProposal.largeSend] is set, with
/// copy keyed off this reason: "almost your whole balance" / "a large amount" / both.
///
/// SDK upgrades may ADD variants: keep a default arm when switching.
pub enum LargeSendReason {
    /// The total debit is a high fraction of the available balance — sending almost everything.
    NearTotalBalance,
    /// The total debit is objectively large regardless of balance (a fixed threshold).
    OverAbsoluteThreshold,
    /// Both triggers fired.
    Both,
    /// Forward-compatibility arm — treat as "large, confirm", never crash.
    Unknown,
}

/// One recipient output within a proposal step — its on-chain pool (the §5.1
/// disclosure) + amount in **zatoshis**.
pub struct ProposalRecipient {
    pub pool: OutputPool,
    /// Amount in zatoshis (exact; max supply < 2^53).
    pub amount_zat: i64,
}

/// One step of a proposal (one transaction). One step for an ordinary send; TWO for a
/// ZIP-320 TEX two-step (tx0 unshields to a wallet-controlled ephemeral address, tx1
/// forwards it to the TEX destination — see [SendProposal.isTwoStepTex]).
pub struct ProposalStep {
    pub recipients: Vec<ProposalRecipient>,
}

/// The display DTO for a prepared send (spec §3.2h) — the numbers the user
/// confirms before signing. Integer **zatoshis** only (no float ever). The
/// upstream proposal stays an OPAQUE, Rust-retained one-shot token; this DTO
/// carries its handle in [`proposalId`](Self::proposal_id) and the display data.
///
/// To execute: pass `proposalId` to `send` (the opaque token is consumed exactly
/// once — a re-send is [WalletErrorKind.proposalAlreadyUsed]; a stale anchor past
/// the short minutes-scale TTL — MONOTONIC, so device suspend counts — is
/// [WalletErrorKind.proposalStale], on which the host re-`propose`s for fresh
/// numbers and re-presents them, never retrying the same `proposalId`).
pub struct SendProposal {
    /// Handle to the retained one-shot proposal token. `i64` (not `u64`) keeps the
    /// Dart side a plain `int`: the registry id is a process-local monotonic
    /// counter that cannot reach 2^63, so the round-trip to `send` is lossless
    /// (the same i64-counter argument as `TxSummary.batchId` / `WalletState.seq`).
    pub proposal_id: i64,
    /// Total debited from the wallet = recipient amounts + fee (the headline).
    pub total_zat: i64,
    /// The ZIP-317 fee the audited change strategy computed.
    pub fee_zat: i64,
    /// Change returned to the wallet (informational; already net out of `total`).
    pub change_zat: i64,
    /// Per-step recipients. One step for an ordinary send; TWO for a ZIP-320 TEX two-step
    /// (see `isTwoStepTex`). Any other multi-step shape is fail-closed in the SDK.
    pub steps: Vec<ProposalStep>,
    /// The chain height the proposal's note anchor targets.
    pub target_height: u32,
    /// §5.1 de-shield disclosure: TRUE if any output lands in the transparent
    /// pool (funds leave the shielded set — surface it before the user signs).
    pub has_transparent_recipient: bool,
    /// TRUE iff this is a SHIELD proposal (from `proposeShield`): transparent funds
    /// → the wallet's own shielded pool. Privacy-POSITIVE — the `hasTransparentRecipient`
    /// de-shield disclosure does NOT apply (opposite direction). The host shows
    /// shield-specific confirm copy ("Shield X · fee Y · net Z shielded") instead of a
    /// recipient screen: `totalZat` is the GROSS transparent being shielded, `changeZat`
    /// the NET that lands shielded (gross − fee), and the lone `steps` output is the
    /// wallet's OWN shielded note (no external recipient). `send` consumes it unchanged.
    pub is_shield: bool,
    /// TRUE iff this is a ZIP-320 TEX two-step (the engine-recognised ephemeral pair). The host
    /// carries it past the confirm screen so the post-`send` OUTCOME copy is HONEST about a partial
    /// broadcast: a partial TEX two-step leaves funds in motion on a wallet-controlled ephemeral
    /// address — money has LEFT the shielded pool but is NOT confirmed at the recipient, recoverable
    /// via the manual sweep, and the user must NOT re-send — which is DIFFERENT from a partial ordinary
    /// multi-tx send (each tx is independent + saved-for-retry by the resubmission machinery). A pure
    /// SHAPE flag: it carries no address/amount/txid (§5.4). Always `false` for a shield and for a
    /// single-step send.
    pub is_two_step_tex: bool,
    /// MONEY-SAFETY (§3): the large-amount signal. Non-null ⇒ show ONE deliberate large-amount
    /// confirm before signing (the only friction step — everything else is a passive cue, no
    /// alert fatigue); `null` ⇒ an ordinary send. Computed from `totalZat` vs the wallet's
    /// AVAILABLE balance (shielded spendable + transparent, the audited SSOT). Always `null` for
    /// a shield. UX note: surface the confirm only once the spendable balance is settled (the
    /// denominator excludes in-flight change, so a mid-scan small send can transiently over-fire).
    pub large_send: Option<LargeSendReason>,
    /// MONEY-SAFETY (§3): best-effort — `true` iff a recipient is the wallet's OWN current
    /// unified address. Show a passive INFO note ("sending to yourself; the fee still applies"),
    /// never a blocker (a self-send is money-safe, just usually unintended). A diversified /
    /// sub-receiver own-address (e.g. the bare sapling form of the UA, or the wallet's transparent
    /// receive address) is NOT detected — a scoped v1 limitation.
    pub self_send: bool,
    /// FR-46: the TOTAL paid to this send's one recipient, in zatoshis, as SIGNED — the fee
    /// and the change excluded. Non-null iff every payment names the same address (two
    /// payments to one address are summed); two recipients ⇒ `null`, never a partial sum (a
    /// unified address and one of its own receivers count as two). A ZIP-320 two-step counts
    /// the forwarded amount once. Always `null` for a shield. The send flow carries it into a
    /// TAGGED `WalletSendTransactionCreated` as `recipientAmountZat`.
    pub single_recipient_zat: Option<i64>,
    /// FR-17 (#396): the SDK-minted spend-binding nonce for THIS proposal (32 opaque
    /// bytes, random, NOT key material). A host with external seed custody records it
    /// at its authorize bracket; the native seed pull for `send` presents the same
    /// value, so the host's supplier fail-closes a sign for any OTHER proposal (the
    /// review↔sign confusion defense). Hosts without a native seed port can ignore
    /// it. Do not log it (it correlates a proposal with its sign moment).
    ///
    /// FR-17 ENFORCEMENT PREREQUISITE: recording this value only protects you if the
    /// host registered the BOUND C-ABI seed supplier (`zec_wallet_register_seed_port_bound`)
    /// and its supply callback actually compares (strict `Option` equality). A legacy
    /// v1 `zec_wallet_register_seed_port` supplier RECEIVES no binding and cannot
    /// fail-close — recording the token without the bound registration is a silent no-op.
    pub binding: Vec<u8>,
}

/// Parse a `zcash:` payment URI (QR code, link — HOSTILE input) into
/// validated display legs.
///
/// `network` is the app's own network — a URI from the other network fails
/// with [`WalletErrorKind.networkMismatch`] ("this is a testnet address"),
/// distinct from a malformed URI. Size-capped, parser-validated, and every
/// leg re-validated by the core before anything is returned.
#[frb(sync)]
pub fn parse_payment_uri(
    uri: String,
    network: Network,
) -> Result<Vec<ParsedPayment>, WalletApiError> {
    crate::convert::parse_payment_uri(&uri, network)
}

/// Encode validated payment drafts as a ZIP-321 `zcash:` URI (e.g. to render
/// a receive QR).
///
/// All drafts must target `network` (cross-feeds fail typed) and at least
/// one draft is required — `zcash:` alone encodes "nothing".
#[frb(sync)]
pub fn encode_payment_uri(
    payments: Vec<PaymentDraft>,
    network: Network,
) -> Result<String, WalletApiError> {
    crate::convert::encode_payment_uri(payments, network)
}

/// The result of `validateAddress`: whether a recipient address can receive a
/// memo. Memo capability requires a shielded receiver, so this single fact is
/// also the SHIELDED-vs-TRANSPARENT (private-vs-public) axis the send form
/// surfaces — a transparent / transparent-only recipient is
/// `memo_capable == false`. No address string is echoed back (§5.4 never-log;
/// the caller already holds the input).
pub struct ValidatedAddress {
    /// `true` ⇒ a shielded recipient: private, and able to receive a ZIP-302
    /// memo. `false` ⇒ transparent / transparent-only: PUBLIC on-chain and
    /// memo-incapable — the host disables + explains the memo field and shows
    /// the §5.1 de-shield disclosure.
    pub memo_capable: bool,
}

/// Validate ONE recipient address for `network` and report its memo capability
/// (the shielded/transparent axis) — WITHOUT composing a URI or proposing a
/// send. Drives the send form's live recipient feedback + memo gating.
///
/// The SAME audited gate the send path lowers through (`Address::parse`), so a
/// live check can never disagree with what `propose` will accept: a malformed
/// address is [`WalletErrorKind.addressInvalid`]; an other-network address is
/// [`WalletErrorKind.networkMismatch`] (distinct, renderable as "this is for a
/// different network"), not collapsed into "malformed". Pure, local and SYNC —
/// no storage, no network, no key material — so a host can validate on every
/// keystroke and even fully offline.
#[frb(sync)]
pub fn validate_address(
    address: String,
    network: Network,
) -> Result<ValidatedAddress, WalletApiError> {
    crate::convert::validate_address(&address, network)
}
