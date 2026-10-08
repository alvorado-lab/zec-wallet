//! 1Click wire DTOs — the exact JSON shapes of the four endpoints, pinned to
//! OpenAPI document 0.1.10 (see `UPSTREAM.md`; recorded fixtures are the
//! KATs in `src/wire_contract_tests.rs`).
//!
//! Hostile-surface posture (§4.6): response structs declare ONLY the fields
//! we read (serde ignores the rest — forward-compatible AND a smaller attack
//! surface); every body is size-capped BEFORE these types exist (`http.rs`);
//! every string is length-bounded BEFORE it enters a port DTO (`map.rs`).
//! The ONE deliberately closed enum is [`WireStatus`] — the spec's
//! closed-enum rule (§3.2) maps anything unknown to
//! `SwapStatus::Failed(ProviderProtocol)`, never a parse failure: a provider
//! adding status variants must not brick polling, it must surface typed.
//! (Live proof this matters: the API already speaks a `depositType` value —
//! `CONFIDENTIAL_INTENTS` — that the official 0.1.10 SDK does not know.)

use serde::{Deserialize, Serialize};
use zec_wallet_core::{ProviderProtocolReason, SwapError};

// ── Request bodies (serialize only) ─────────────────────────────────────────

/// `POST /v0/quote` request. Field set and casing are the documented wire
/// contract; everything here is OUR data (already validated by `SwapService`
/// + `map.rs`) — never echo provider strings into it.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WireQuoteRequest {
    /// `false` = the REGISTERING quote: the response's `depositAddress` is
    /// the live swap handle (there is no separate "create swap" endpoint).
    pub dry: bool,
    /// `EXACT_INPUT` | `EXACT_OUTPUT` (closed set, ours to choose).
    pub swap_type: &'static str,
    /// Basis points (provider-validated 0..=10000; ours ≤ SLIPPAGE_MAX_BPS).
    pub slippage_tolerance: u16,
    pub origin_asset: String,
    /// Always `ORIGIN_CHAIN`: deposits come from a real chain address —
    /// the INTENTS/CONFIDENTIAL_INTENTS account models are not our flow.
    pub deposit_type: &'static str,
    pub destination_asset: String,
    /// BASE UNITS as a decimal-integer string (the 400 fixture states this
    /// verbatim); conversion from user decimals is exact integer math.
    pub amount: String,
    pub refund_to: String,
    pub refund_type: &'static str,
    pub recipient: String,
    pub recipient_type: &'static str,
    /// RFC 3339 UTC (`time::format_rfc3339_utc`).
    pub deadline: String,
}

/// `POST /v0/deposit/submit` request (wired into the port at the
/// send-pipeline chunk; the endpoint ships typed + fixture-pinned now).
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WireSubmitRequest {
    pub tx_hash: String,
    pub deposit_address: String,
}

// ── Response bodies (deserialize; unknown fields ignored) ────────────────────

/// `POST /v0/quote` 201 response (also embedded in status responses).
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WireQuoteResponse {
    pub quote: WireQuote,
    /// The provider's echo of OUR request — `refundTo` feeds the §2.6
    /// echo-verification in `SwapService`; `originAsset` keys the decimals
    /// lookup when mapping status amounts.
    pub quote_request: WireRequestEcho,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WireQuote {
    /// Present only on non-dry quotes — REQUIRED for our flow (missing ⇒
    /// typed `MissingField`); doubles as the swap id (`/v0/status` keys off
    /// it).
    #[serde(default)]
    pub deposit_address: Option<String>,
    /// A deposit address that REQUIRES a memo. The wallet cannot honor one
    /// (and depositing without it LOSES FUNDS per the provider docs), so
    /// presence is a typed refusal — never silently dropped (`map.rs`).
    #[serde(default)]
    pub deposit_memo: Option<String>,
    /// Origin-asset base units (decimal-integer string).
    pub amount_in: String,
    /// Destination-asset base units, slippage floor — the GUARANTEE the
    /// port DTO carries (the wire's pre-slippage `amountOut` estimate is
    /// deliberately not read: `SwapQuote` has no field for it, §2.6).
    pub min_amount_out: String,
    /// RFC 3339; the deposit deadline — REQUIRED on non-dry quotes (feeds
    /// the dual deadline gate in `SwapService`).
    #[serde(default)]
    pub deadline: Option<String>,
}

/// The provider's echo of OUR quote request. Every field we send that names a
/// term of the swap is kept, so the quote door can compare it with the request
/// before accepting the quote ([`crate::map::check_request_echo`]).
///
/// A STATUS response embeds the same shape, for a swap that may already hold
/// the user's deposit, so decoding must not get stricter than it was: the two
/// fields read before this check keep their types, and every field added for
/// it is an untyped JSON value. An odd value there fails only the quote door's
/// comparison, never a status decode (the security review of the fix). Absent
/// fields decode as `None`; the quote door refuses them.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WireRequestEcho {
    #[serde(default)]
    pub refund_to: Option<String>,
    #[serde(default)]
    pub origin_asset: Option<String>,
    #[serde(default)]
    pub dry: Option<serde_json::Value>,
    #[serde(default)]
    pub swap_type: Option<serde_json::Value>,
    #[serde(default)]
    pub slippage_tolerance: Option<serde_json::Value>,
    #[serde(default)]
    pub deposit_type: Option<serde_json::Value>,
    #[serde(default)]
    pub destination_asset: Option<serde_json::Value>,
    #[serde(default)]
    pub amount: Option<serde_json::Value>,
    #[serde(default)]
    pub refund_type: Option<serde_json::Value>,
    #[serde(default)]
    pub recipient: Option<serde_json::Value>,
    #[serde(default)]
    pub recipient_type: Option<serde_json::Value>,
}

/// `GET /v0/status` / `POST /v0/deposit/submit` response.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WireStatusResponse {
    pub status: WireStatus,
    pub quote_response: WireQuoteResponse,
    #[serde(default)]
    pub swap_details: Option<WireSwapDetails>,
}

/// The provider lifecycle — THE closed enum (§3.2 fixed mapping). Unknown
/// strings land on `Unknown` via `#[serde(other)]` and map to
/// `SwapStatus::Failed(ProviderProtocol)` in `map.rs`, never a decode error.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum WireStatus {
    #[serde(rename = "PENDING_DEPOSIT")]
    PendingDeposit,
    #[serde(rename = "KNOWN_DEPOSIT_TX")]
    KnownDepositTx,
    #[serde(rename = "INCOMPLETE_DEPOSIT")]
    IncompleteDeposit,
    #[serde(rename = "PROCESSING")]
    Processing,
    #[serde(rename = "SUCCESS")]
    Success,
    #[serde(rename = "REFUNDED")]
    Refunded,
    #[serde(rename = "FAILED")]
    Failed,
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WireSwapDetails {
    /// Realized slippage (display-only; rounded to bps if sane, else None —
    /// a malformed display field must not fail a SUCCESS status).
    #[serde(default)]
    pub slippage: Option<f64>,
    #[serde(default)]
    pub origin_chain_tx_hashes: Vec<WireTxDetails>,
    #[serde(default)]
    pub destination_chain_tx_hashes: Vec<WireTxDetails>,
    /// Base units of the origin asset actually seen on-chain.
    #[serde(default)]
    pub deposited_amount: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WireTxDetails {
    pub hash: String,
}

/// `GET /v0/tokens` entry. `blockchain` is deliberately an OPEN string (the
/// generated SDK's closed enum is already behind the live API) — we only
/// ever match it against caller-supplied chain ids.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WireToken {
    pub asset_id: String,
    /// OpenAPI `number`; validated integral 0..=38 before use.
    pub decimals: f64,
    pub blockchain: String,
    pub symbol: String,
    /// USD estimate — DISPLAY ONLY, never funds math.
    #[serde(default)]
    pub price: Option<f64>,
}

// ── Decode funnels (body bytes already size-capped by http.rs) ───────────────

fn undecodable() -> SwapError {
    SwapError::ProviderProtocol {
        reason: ProviderProtocolReason::UndecodableResponse,
    }
}

pub(crate) fn decode_quote_response(body: &[u8]) -> Result<WireQuoteResponse, SwapError> {
    serde_json::from_slice(body).map_err(|_| undecodable())
}

pub(crate) fn decode_status_response(body: &[u8]) -> Result<WireStatusResponse, SwapError> {
    serde_json::from_slice(body).map_err(|_| undecodable())
}

pub(crate) fn decode_tokens_response(body: &[u8]) -> Result<Vec<WireToken>, SwapError> {
    serde_json::from_slice(body).map_err(|_| undecodable())
}
