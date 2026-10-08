//! Wire → port mapping with the §4.6 funnel: every provider string is
//! length-bounded and shape-validated HERE, before anything enters a port
//! DTO; all unit conversion is exact integer math (no floats anywhere near
//! an amount).
//!
//! Unit contract: the wire speaks BASE UNITS (decimal-integer strings; the
//! provider's own 400 message spells this out), the port speaks HUMAN
//! decimal strings (spec §2.6 — what the user anchored, what the UI shows,
//! what `SwapService`'s M1 bps math compares). The token's `decimals` value
//! is the exact bridge; for ZEC, decimals = 8 means base units ARE zatoshis
//! (verified at resolution, not assumed).

use zec_wallet_core::constants::{PROVIDER_STR_MAX_BYTES, SWAP_DECIMAL_MAX_DIGITS};
use zec_wallet_core::{
    DisclosureItem, ProviderProtocolReason, QuoteRequest, SwapAmount, SwapDirection, SwapError,
    SwapFailureCode, SwapId, SwapPrivacyDisclosure, SwapQuote, SwapStatus, Zatoshis,
};

use crate::time::{format_rfc3339_utc, parse_rfc3339_utc};
use crate::wire::{
    WireQuoteRequest, WireQuoteResponse, WireStatus, WireStatusResponse, WireSwapDetails, WireToken,
};

/// One provider-swappable asset, bounds-checked — the host's token-picker
/// row and our internal `AssetId{chain,symbol}` → provider-id resolution
/// table.
#[derive(Clone, PartialEq, Debug)]
pub struct SwapAsset {
    /// Provider chain id (open set: `"zec"`, `"near"`, `"eth"`, …) — the
    /// value `AssetId::chain` must match.
    pub chain: String,
    /// Ticker symbol (`AssetId::symbol` must match).
    pub symbol: String,
    /// Base-unit scale (ZEC = 8 ⇒ base units are zatoshis).
    pub decimals: u8,
    /// The provider's exact asset id (diagnostics + dedup; e.g.
    /// `nep141:zec.omft.near`).
    pub provider_asset_id: String,
    /// USD price estimate — DISPLAY ONLY (sorting a token list), never
    /// funds math; absent when the provider sends none.
    pub price_usd: Option<f64>,
}

fn protocol(reason: ProviderProtocolReason) -> SwapError {
    SwapError::ProviderProtocol { reason }
}

/// m1: the named bound on EVERY provider string, before anything reads it.
fn bounded(s: &str) -> Result<&str, SwapError> {
    if s.len() > PROVIDER_STR_MAX_BYTES {
        return Err(protocol(ProviderProtocolReason::OversizedField));
    }
    Ok(s)
}

/// A deposit memo must RENDER exactly as it COPIES — a control byte (C0/C1, incl.
/// CR/LF) or a bidi/zero-width format char could make the displayed memo differ
/// from the bytes the user pastes into their external wallet (a money-display
/// spoof). Such a char makes the memo malformed provider data, REJECTED rather
/// than stripped: a required memo is funds-critical and must stay byte-exact, so
/// silently altering it is never safe. A legitimate tag/memo contains none of
/// these (it is a single-line printable token).
fn memo_display_unsafe(c: char) -> bool {
    c.is_control()
        || matches!(c,
            '\u{200B}'..='\u{200F}'   // zero-width chars + LRM/RLM
            | '\u{202A}'..='\u{202E}' // bidi embeddings/overrides
            | '\u{2066}'..='\u{2069}' // bidi isolates
            | '\u{FEFF}') // zero-width no-break space / BOM
}

// ── Exact base-unit arithmetic (no floats on a funds path) ──────────────────

/// Base-unit amount: ASCII digits only (no sign/dot/exponent), 1..=27
/// significant digits (`SWAP_DECIMAL_MAX_DIGITS` — same bound the service's
/// decimal math uses; 27 digits fit u128 with room).
pub(crate) fn parse_base_units(s: &str) -> Option<u128> {
    if s.is_empty() || s.len() > SWAP_DECIMAL_MAX_DIGITS {
        return None;
    }
    let mut v: u128 = 0;
    for b in s.bytes() {
        if !b.is_ascii_digit() {
            return None;
        }
        v = v * 10 + u128::from(b - b'0');
    }
    Some(v)
}

/// Base units → canonical human decimal string: `(412248474, 6)` →
/// `"412.248474"`, `(100000000, 8)` → `"1"` (trailing zeros trimmed — the
/// canonical form `parse_decimal` on the service side reads back exactly).
/// `None` = a `decimals` scale beyond u128 (>38) — TOTAL regardless of
/// caller, not reliant on the `map_tokens` ≤38 invariant (review fold: the
/// service side learned this lesson once already, spec §8 W2 note).
pub(crate) fn format_units(v: u128, decimals: u8) -> Option<String> {
    if decimals == 0 {
        return Some(v.to_string());
    }
    let div = 10u128.checked_pow(u32::from(decimals))?;
    let int = v / div;
    let frac = format!("{:0width$}", v % div, width = decimals as usize);
    let frac = frac.trim_end_matches('0');
    Some(if frac.is_empty() {
        int.to_string()
    } else {
        format!("{int}.{frac}")
    })
}

/// Human decimal string → base units, exact: `("1.5", 6)` → `1500000`.
/// `None` = malformed, more fractional digits than the asset carries
/// (rounding someone's money is not ours to do), or a scaled value beyond
/// u128. The digit cap bounds the MANTISSA only — scaling by
/// `10^(decimals − frac_len)` can overflow u128 for high-decimal assets
/// (21 integer digits × 10^18 already does), so the pow AND the multiply
/// are CHECKED: wrap-silently-in-release is the exact wrong-money bug the
/// service's bps math already fixed once (spec §8 W2 note) — fail typed,
/// never wrap, never round.
pub(crate) fn decimal_to_base_units(s: &str, decimals: u8) -> Option<u128> {
    let (int_part, frac_part) = match s.split_once('.') {
        Some((i, f)) => (i, f),
        None => (s, ""),
    };
    // "1." and ".5" rejected: both halves must be present where named
    if int_part.is_empty() || (s.contains('.') && frac_part.is_empty()) {
        return None;
    }
    if frac_part.len() > usize::from(decimals) {
        return None;
    }
    if int_part.len() + frac_part.len() > SWAP_DECIMAL_MAX_DIGITS {
        return None;
    }
    if !int_part.bytes().all(|b| b.is_ascii_digit())
        || !frac_part.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let mut v: u128 = 0;
    for b in int_part.bytes().chain(frac_part.bytes()) {
        v = v * 10 + u128::from(b - b'0');
    }
    // scale up by the fractional digits NOT provided — checked end to end
    let scale = 10u128.checked_pow(u32::from(decimals) - frac_part.len() as u32)?;
    v.checked_mul(scale)
}

fn zatoshis_from_base(v: u128) -> Result<Zatoshis, SwapError> {
    let zat = i64::try_from(v).map_err(|_| protocol(ProviderProtocolReason::MalformedAmount))?;
    Zatoshis::new(zat).map_err(|_| protocol(ProviderProtocolReason::MalformedAmount))
}

// ── Tokens ──────────────────────────────────────────────────────────────────

/// Strict whole-list mapping: one malformed entry fails the call typed —
/// silently dropping assets would be a silent failure, and the recorded
/// fixture pins that real entries are well-formed.
pub(crate) fn map_tokens(wire: Vec<WireToken>) -> Result<Vec<SwapAsset>, SwapError> {
    wire.into_iter()
        .map(|t| {
            bounded(&t.asset_id)?;
            bounded(&t.blockchain)?;
            bounded(&t.symbol)?;
            // OpenAPI `number`; only integral 0..=38 is a meaningful scale
            if !t.decimals.is_finite()
                || t.decimals.fract() != 0.0
                || !(0.0..=38.0).contains(&t.decimals)
            {
                return Err(protocol(ProviderProtocolReason::MalformedAmount));
            }
            Ok(SwapAsset {
                chain: t.blockchain,
                symbol: t.symbol,
                decimals: t.decimals as u8,
                provider_asset_id: t.asset_id,
                price_usd: t.price.filter(|p| p.is_finite()),
            })
        })
        .collect()
}

// ── Quote (OutOfZec — the only direction the port can complete today) ───────

/// Build the registering (`dry: false`) wire request for an OutOfZec quote.
/// `origin` is the provider's ZEC asset (decimals verified == 8 by the
/// caller); `dest` is the resolved foreign asset.
pub(crate) fn build_out_of_zec_request(
    req: &QuoteRequest,
    origin: &SwapAsset,
    dest: &SwapAsset,
    now_unix: u64,
    deadline_window_secs: u64,
) -> Result<WireQuoteRequest, SwapError> {
    use zec_wallet_core::ExactSide;

    let (swap_type, amount) = match &req.exact {
        // user fixed the ZEC they spend — zatoshis ARE the base units
        ExactSide::In(SwapAmount::Zec(z)) => ("EXACT_INPUT", z.zat().to_string()),
        // user fixed the foreign amount to receive
        ExactSide::Out(SwapAmount::Foreign(s)) => (
            "EXACT_OUTPUT",
            decimal_to_base_units(s, dest.decimals)
                .ok_or(SwapError::RequestInvalid {
                    reason: "exact amount malformed or more precise than the asset's decimals",
                })?
                .to_string(),
        ),
        // service coherence makes these unreachable; refuse defensively
        _ => {
            return Err(SwapError::RequestInvalid {
                reason: "exact side names the wrong asset family for this direction",
            });
        }
    };
    let refund_to = req
        .refund_address
        .clone()
        .ok_or(SwapError::RequestInvalid {
            reason: "refund address missing — SwapService populates it for OutOfZec",
        })?;
    let recipient = req
        .destination
        .clone()
        .ok_or(SwapError::DestinationInvalid {
            reason: zec_wallet_core::DestinationInvalidReason::Missing,
        })?;
    Ok(WireQuoteRequest {
        dry: false,
        swap_type,
        slippage_tolerance: req.slippage_tolerance_bps,
        origin_asset: origin.provider_asset_id.clone(),
        deposit_type: "ORIGIN_CHAIN",
        destination_asset: dest.provider_asset_id.clone(),
        amount,
        refund_to,
        refund_type: "ORIGIN_CHAIN",
        recipient,
        recipient_type: "DESTINATION_CHAIN",
        deadline: format_rfc3339_utc(now_unix.saturating_add(deadline_window_secs)),
    })
}

// ── Quote (IntoZec — foreign → ZEC; IZ-1 / §3.3b) ───────────────────────────

/// Build the registering (`dry: false`) wire request for an IntoZec quote — the MIRROR of
/// [`build_out_of_zec_request`] with the asset roles SWAPPED: `origin` is the user's foreign
/// SOURCE asset (the coin they send), `dest` is the provider's ZEC (decimals verified == 8 by
/// the caller). `recipient` is OUR fresh engine-persisted ZEC destination (the service minted
/// it via `get_address_for_index` and bound it into `req.destination`, §3.3b D1); `refund_to`
/// is the USER's source-chain refund target (where their coin returns if the swap fails, §4.4
/// HARD-G). Both `refund_type`/`recipient_type` stay `ORIGIN_CHAIN`/`DESTINATION_CHAIN` (refund
/// on the deposit/input chain, recipient on the output chain — direction-independent in the
/// 1Click model); only WHICH asset is origin vs destination flips.
///
/// **Wire-injection (§3.3b D6):** `refund_to` (user-supplied, untrusted) reaches the request
/// ONLY as a `serde_json`-serialized field value in `lib.rs` (serde is the escaper — a `"` or
/// CRLF cannot forge a second field); NEVER via string interpolation. `recipient` is our own
/// engine-derived UA. Both inherit the §4.6 `PROVIDER_STR_MAX_BYTES` bound at the service door.
pub(crate) fn build_into_zec_request(
    req: &QuoteRequest,
    origin: &SwapAsset,
    dest: &SwapAsset,
    now_unix: u64,
    deadline_window_secs: u64,
) -> Result<WireQuoteRequest, SwapError> {
    use zec_wallet_core::ExactSide;

    let (swap_type, amount) = match &req.exact {
        // user fixed the FOREIGN they send — scale by the source asset's decimals
        ExactSide::In(SwapAmount::Foreign(s)) => (
            "EXACT_INPUT",
            decimal_to_base_units(s, origin.decimals)
                .ok_or(SwapError::RequestInvalid {
                    reason: "exact amount malformed or more precise than the asset's decimals",
                })?
                .to_string(),
        ),
        // user fixed the ZEC they receive — zatoshis ARE the base units (dest.decimals == 8)
        ExactSide::Out(SwapAmount::Zec(z)) => ("EXACT_OUTPUT", z.zat().to_string()),
        // service coherence (`validate_request`) makes these unreachable; refuse defensively
        _ => {
            return Err(SwapError::RequestInvalid {
                reason: "exact side names the wrong asset family for this direction",
            });
        }
    };
    let refund_to = req
        .refund_address
        .clone()
        .ok_or(SwapError::RequestInvalid {
            reason: "refund address missing — IntoZec requires a source-chain refund target",
        })?;
    // The service populated this with our fresh engine-persisted ZEC destination (§3.3b D1);
    // its absence is a service-wiring bug, not a user error.
    let recipient = req
        .destination
        .clone()
        .ok_or(SwapError::DestinationInvalid {
            reason: zec_wallet_core::DestinationInvalidReason::Missing,
        })?;
    Ok(WireQuoteRequest {
        dry: false,
        swap_type,
        slippage_tolerance: req.slippage_tolerance_bps,
        origin_asset: origin.provider_asset_id.clone(),
        deposit_type: "ORIGIN_CHAIN",
        destination_asset: dest.provider_asset_id.clone(),
        amount,
        refund_to,
        refund_type: "ORIGIN_CHAIN",
        recipient,
        recipient_type: "DESTINATION_CHAIN",
        deadline: format_rfc3339_utc(now_unix.saturating_add(deadline_window_secs)),
    })
}

/// Honest per-direction disclosure (§2.6 — the DTO the UI renders from).
/// A direction this adapter does not know cannot have its privacy shape
/// described honestly — typed refusal, never a guessed disclosure.
pub(crate) fn disclosure_for(
    direction: &SwapDirection,
) -> Result<SwapPrivacyDisclosure, SwapError> {
    Ok(match direction {
        SwapDirection::OutOfZec { .. } => SwapPrivacyDisclosure {
            ends_shielded: false,
            deshields: true,
            provider_legs_transparent: true,
            provider_sees: vec![
                DisclosureItem::CrossAssetLink,
                DisclosureItem::Amounts,
                DisclosureItem::DestinationAddress,
                DisclosureItem::IpUnlessTor,
            ],
        },
        SwapDirection::IntoZec { .. } => SwapPrivacyDisclosure {
            ends_shielded: true,
            deshields: false,
            provider_legs_transparent: true,
            provider_sees: vec![
                DisclosureItem::CrossAssetLink,
                DisclosureItem::Amounts,
                DisclosureItem::SourceAddress,
                DisclosureItem::IpUnlessTor,
            ],
        },
        // SwapDirection is #[non_exhaustive]
        _ => {
            return Err(SwapError::RequestInvalid {
                reason: "swap direction unknown to this adapter",
            });
        }
    })
}

/// The fields a quote response maps to identically in BOTH directions — the §4.6
/// funnel applied once. `amount_in_units`/`min_out_units` are the RAW base-unit
/// integers retained so each direction can pick the ZEC side (`amount_in` for
/// OutOfZec, `min_out` for IntoZec) without re-parsing; the `*_str` are the human
/// decimal strings the port speaks.
struct CommonQuote {
    deposit_address: String,
    expires_at: u64,
    amount_in_str: String,
    min_amount_out_str: String,
    amount_in_units: u128,
    min_out_units: u128,
    refund_to: Option<String>,
}

/// The quote door's echo check: the provider's echo of the request must state
/// EXACTLY the terms we sent — who receives, which assets, how much, which side
/// is exact, the slippage, and where a refund goes. A field the echo leaves out
/// is `MissingField`; a different refund address is `RefundAddressMismatch` (the
/// reason `SwapService`'s own refund check gives); any other difference is
/// `RequestEchoMismatch`. Either way the quote is refused before it can reach a
/// review, so the payout address the user verifies there is one the provider
/// has confirmed, not only the one we asked for (2026-10-07 external review,
/// finding 3).
///
/// What it does NOT cover, said plainly: the deadline (the provider may round
/// it, and `SwapService` gates on the quote's own deadline), and terms the
/// provider ADDS (`appFees`, `depositMode`; their effect lands in the checked
/// amounts and deposit memo). It catches a buggy or crossed-up provider. It
/// cannot catch a malicious one, which can echo our terms faithfully and still
/// control the deposit address it hands back.
pub(crate) fn check_request_echo(
    sent: &WireQuoteRequest,
    echo: &crate::wire::WireRequestEcho,
) -> Result<(), SwapError> {
    use serde_json::{Value, json};
    fn same(echoed: Option<&Value>, sent: Value) -> Result<(), SwapError> {
        match echoed {
            None => Err(protocol(ProviderProtocolReason::MissingField)),
            Some(v) if *v == sent => Ok(()),
            Some(_) => Err(protocol(ProviderProtocolReason::RequestEchoMismatch)),
        }
    }
    match echo.refund_to.as_deref() {
        None => return Err(protocol(ProviderProtocolReason::MissingField)),
        Some(r) if r != sent.refund_to => {
            return Err(protocol(ProviderProtocolReason::RefundAddressMismatch));
        }
        Some(_) => {}
    }
    match echo.origin_asset.as_deref() {
        None => return Err(protocol(ProviderProtocolReason::MissingField)),
        Some(a) if a != sent.origin_asset => {
            return Err(protocol(ProviderProtocolReason::RequestEchoMismatch));
        }
        Some(_) => {}
    }
    same(echo.dry.as_ref(), json!(sent.dry))?;
    same(echo.swap_type.as_ref(), json!(sent.swap_type))?;
    same(
        echo.slippage_tolerance.as_ref(),
        json!(sent.slippage_tolerance),
    )?;
    same(echo.deposit_type.as_ref(), json!(sent.deposit_type))?;
    same(
        echo.destination_asset.as_ref(),
        json!(sent.destination_asset),
    )?;
    same(echo.amount.as_ref(), json!(sent.amount))?;
    same(echo.refund_type.as_ref(), json!(sent.refund_type))?;
    same(echo.recipient.as_ref(), json!(sent.recipient))?;
    same(echo.recipient_type.as_ref(), json!(sent.recipient_type))
}

/// Map the direction-independent fields of a quote response (DRY — the deposit
/// address, deadline, both amounts via the token `decimals`, and the echoed
/// refund). Each direction adds the parts that genuinely differ: which side is
/// ZEC, the deposit-memo posture, and the disclosure. `amount_in` is formatted
/// with `origin.decimals`, `min_amount_out` with `dest.decimals` — the SAME in
/// both directions (only the ROLE of origin/dest, foreign-vs-ZEC, differs).
fn map_common_quote(
    wire: WireQuoteResponse,
    origin: &SwapAsset,
    dest: &SwapAsset,
) -> Result<CommonQuote, SwapError> {
    let q = wire.quote;
    let deposit_address = match q.deposit_address.as_deref() {
        Some(a) if !a.is_empty() => bounded(a)?.to_string(),
        _ => return Err(protocol(ProviderProtocolReason::MissingField)),
    };
    let deadline = q
        .deadline
        .as_deref()
        .ok_or(protocol(ProviderProtocolReason::MissingField))?;
    let expires_at = parse_rfc3339_utc(bounded(deadline)?)
        .ok_or(protocol(ProviderProtocolReason::UndecodableResponse))?;
    let amount_in_units = parse_base_units(bounded(&q.amount_in)?)
        .ok_or(protocol(ProviderProtocolReason::MalformedAmount))?;
    let min_out_units = parse_base_units(bounded(&q.min_amount_out)?)
        .ok_or(protocol(ProviderProtocolReason::MalformedAmount))?;
    let refund_to = match wire.quote_request.refund_to {
        Some(r) => {
            bounded(&r)?;
            Some(r)
        }
        None => None,
    };
    Ok(CommonQuote {
        deposit_address,
        expires_at,
        // port amounts are HUMAN decimal strings (module note)
        amount_in_str: format_units(amount_in_units, origin.decimals)
            .ok_or(protocol(ProviderProtocolReason::MalformedAmount))?,
        min_amount_out_str: format_units(min_out_units, dest.decimals)
            .ok_or(protocol(ProviderProtocolReason::MalformedAmount))?,
        amount_in_units,
        min_out_units,
        refund_to,
    })
}

/// Map a quote response to the port DTO (OutOfZec). The in-side is ZEC
/// (`origin.decimals == 8`), so `zec_side` is `amount_in`.
pub(crate) fn map_out_of_zec_quote(
    wire: WireQuoteResponse,
    direction: &SwapDirection,
    origin: &SwapAsset,
    dest: &SwapAsset,
) -> Result<SwapQuote, SwapError> {
    // funds-loss guard FIRST: the WALLET sends this deposit (a plain
    // shielded→transparent ZEC send, §4.4) and CANNOT attach a source-chain
    // memo — a memo-requiring deposit is unusable. PERMANENT typed refusal
    // (unlike IntoZec, where the USER attaches it). ZEC-chain deposits carry no
    // memo, so this never fires in practice.
    if wire.quote.deposit_memo.is_some() {
        return Err(protocol(ProviderProtocolReason::DepositMemoUnsupported));
    }
    let c = map_common_quote(wire, origin, dest)?;
    Ok(SwapQuote {
        // The PROVIDER's handle (the 1Click deposit address) — `SwapService::quote`
        // swaps in the SDK-minted execution identity and keeps this as `provider_ref`.
        id: SwapId::new(c.deposit_address.clone()),
        deposit_address: c.deposit_address,
        deposit_memo: None,
        expires_at: c.expires_at,
        amount_in: c.amount_in_str,
        min_amount_out: c.min_amount_out_str,
        zec_side: zatoshis_from_base(c.amount_in_units)?,
        refund_to: c.refund_to,
        disclosure: disclosure_for(direction)?,
        // FR-17: adapters never mint the spend binding — the SwapService is the
        // authority and sets it after validation (types.rs field doc).
        binding: None,
    })
}

/// Map a quote response to the port DTO (IntoZec) — the MIRROR of
/// [`map_out_of_zec_quote`]: `origin` is the foreign SOURCE asset, `dest` is ZEC
/// (`decimals == 8`). The IN-side amount is FOREIGN, the OUT-side is ZEC; `zec_side`
/// is the OUTPUT (`min_amount_out` as zatoshis), which the M1 user-anchored bound
/// (`validate_quote`) checks against the user's requested ZEC when they fixed the
/// output side.
pub(crate) fn map_into_zec_quote(
    wire: WireQuoteResponse,
    direction: &SwapDirection,
    origin: &SwapAsset,
    dest: &SwapAsset,
) -> Result<SwapQuote, SwapError> {
    // A memo-REQUIRING deposit IS honored for IntoZec: the USER sends the deposit
    // externally (§4.4), so a required destination tag / memo is bounded here and
    // SHOWN on the D7 deposit screen — without it the deposit would be lost, so it
    // is never silently dropped. Untrusted provider input → length-bounded at the
    // §4.6 funnel; an empty memo is no memo (normalized to None so the UI renders
    // nothing rather than an empty "required" field).
    let deposit_memo = match wire.quote.deposit_memo.as_deref() {
        Some(m) if !m.is_empty() => {
            let m = bounded(m)?;
            // reject (never strip) a memo whose glyphs could differ from its bytes
            // — see `memo_display_unsafe`. A memo must reach the user byte-exact.
            if m.chars().any(memo_display_unsafe) {
                return Err(protocol(ProviderProtocolReason::UndecodableResponse));
            }
            Some(m.to_string())
        }
        _ => None,
    };
    let c = map_common_quote(wire, origin, dest)?;
    Ok(SwapQuote {
        // The PROVIDER's handle: the 1Click deposit address. For IntoZec that address is the
        // provider's SOURCE-chain deposit address (where the USER sends their foreign coin, §4.4)
        // — NOT a wallet ZEC address. Nothing makes it unique per quote (the provider may reuse
        // or reissue it), which is why `SwapService::quote` replaces it with the SDK-minted
        // execution identity and keeps this only as `provider_ref` (S8). §5.4 NEVER-log holds
        // (a deposit address is a never-log item).
        id: SwapId::new(c.deposit_address.clone()),
        deposit_address: c.deposit_address,
        deposit_memo,
        expires_at: c.expires_at,
        amount_in: c.amount_in_str,
        min_amount_out: c.min_amount_out_str,
        // the ZEC side of an IntoZec quote is the OUTPUT (min_amount_out), in zatoshis (ZEC
        // base units == zatoshis at decimals 8) — NOT amount_in (which is the foreign input)
        zec_side: zatoshis_from_base(c.min_out_units)?,
        refund_to: c.refund_to,
        disclosure: disclosure_for(direction)?,
        // FR-17: adapters never mint the spend binding — the SwapService is the
        // authority and sets it after validation (types.rs field doc).
        binding: None,
    })
}

// ── Status (the §3.2 fixed mapping; closed-world) ───────────────────────────

/// Map a status/submit response. `resolve_decimals` looks the echoed
/// `originAsset` id up in the token table (needed only for the
/// INCOMPLETE_DEPOSIT arm's amount rendering).
pub(crate) fn map_status(
    wire: WireStatusResponse,
    resolve_decimals: impl Fn(&str) -> Option<u8>,
) -> Result<SwapStatus, SwapError> {
    let echo_quote = &wire.quote_response.quote;
    let deadline_unix = || -> Result<u64, SwapError> {
        let d = echo_quote
            .deadline
            .as_deref()
            .ok_or(protocol(ProviderProtocolReason::MissingField))?;
        parse_rfc3339_utc(bounded(d)?).ok_or(protocol(ProviderProtocolReason::UndecodableResponse))
    };
    let details = wire.swap_details.unwrap_or_default();
    // last hash: bounded; the arrays are chronological, the final entry is
    // the settlement/refund tx (the first origin-chain entry is the user's
    // own deposit) — display-only either way
    let last_hash = |v: &[crate::wire::WireTxDetails]| -> Result<Option<String>, SwapError> {
        match v.last() {
            Some(t) => Ok(Some(bounded(&t.hash)?.to_string())),
            None => Ok(None),
        }
    };
    Ok(match wire.status {
        WireStatus::PendingDeposit => SwapStatus::PendingDeposit {
            expires_at: deadline_unix()?,
        },
        WireStatus::KnownDepositTx => SwapStatus::DepositDetected,
        // extracted for length (the busiest arm) — see `map_incomplete_deposit`
        WireStatus::IncompleteDeposit => map_incomplete_deposit(
            &wire.quote_response,
            &details,
            &resolve_decimals,
            deadline_unix()?,
        )?,
        WireStatus::Processing => SwapStatus::Processing,
        WireStatus::Success => SwapStatus::Success {
            out_txid: last_hash(&details.destination_chain_tx_hashes)?,
            // display-only: a malformed slippage value must not fail a
            // SUCCESS — quarantine the FIELD (None), not the status
            realized_slippage_bps: details
                .slippage
                .filter(|s| s.is_finite() && (-10_000.0..=10_000.0).contains(s))
                .map(|s| s.round() as i32),
        },
        WireStatus::Refunded => SwapStatus::Refunded {
            refund_txid: last_hash(&details.origin_chain_tx_hashes)?,
        },
        WireStatus::Failed => SwapStatus::Failed {
            code: SwapFailureCode::ProviderFailure,
        },
        // the closed-enum rule (§3.2): unknown lifecycle words surface as a
        // typed terminal STATUS, never a decode error and never a guess
        WireStatus::Unknown => SwapStatus::Failed {
            code: SwapFailureCode::ProviderProtocol,
        },
    })
}

/// The INCOMPLETE_DEPOSIT arm of [`map_status`], extracted for length: render
/// the partial-deposit shape (received / still-missing / top-up deadline).
/// Needs the token table (`resolve_decimals`) to turn the echoed BASE-UNIT
/// amounts into the port's decimal strings — a provider actively swapping an
/// asset it no longer describes in `/v0/tokens` cannot have its amounts
/// rendered honestly, so that is a typed `UndecodableResponse`, never a guess.
fn map_incomplete_deposit(
    quote_response: &WireQuoteResponse,
    details: &WireSwapDetails,
    resolve_decimals: &impl Fn(&str) -> Option<u8>,
    deadline: u64,
) -> Result<SwapStatus, SwapError> {
    let origin_id = quote_response
        .quote_request
        .origin_asset
        .as_deref()
        .ok_or(protocol(ProviderProtocolReason::MissingField))?;
    let decimals = resolve_decimals(bounded(origin_id)?)
        .ok_or(protocol(ProviderProtocolReason::UndecodableResponse))?;
    let expected = parse_base_units(bounded(&quote_response.quote.amount_in)?)
        .ok_or(protocol(ProviderProtocolReason::MalformedAmount))?;
    let received = details
        .deposited_amount
        .as_deref()
        .ok_or(protocol(ProviderProtocolReason::MissingField))
        .and_then(|s| {
            parse_base_units(bounded(s)?).ok_or(protocol(ProviderProtocolReason::MalformedAmount))
        })?;
    Ok(SwapStatus::UnderDeposited {
        received: format_units(received, decimals)
            .ok_or(protocol(ProviderProtocolReason::MalformedAmount))?,
        // saturating: an over-deposit reported as INCOMPLETE is the provider's
        // contradiction — "0 missing" is the honest floor
        missing: format_units(expected.saturating_sub(received), decimals)
            .ok_or(protocol(ProviderProtocolReason::MalformedAmount))?,
        deadline,
    })
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn unit_conversion_known_answers() {
        // the recorded OutOfZec fixture's numbers
        assert_eq!(format_units(100_000_000, 8).as_deref(), Some("1"));
        assert_eq!(format_units(412_248_474, 6).as_deref(), Some("412.248474"));
        assert_eq!(format_units(0, 8).as_deref(), Some("0"));
        assert_eq!(format_units(1, 8).as_deref(), Some("0.00000001"));
        assert_eq!(format_units(23_540_040, 8).as_deref(), Some("0.2354004"));
        assert_eq!(format_units(5, 0).as_deref(), Some("5"));
        // total beyond the token-validated range: 10^39 > u128 ⇒ None, not panic
        assert_eq!(format_units(1, 39), None);

        assert_eq!(decimal_to_base_units("1", 8), Some(100_000_000));
        assert_eq!(decimal_to_base_units("1.5", 6), Some(1_500_000));
        assert_eq!(decimal_to_base_units("0.00000001", 8), Some(1));
        assert_eq!(decimal_to_base_units("412.248474", 6), Some(412_248_474));
        // more precision than the asset carries: rejected, never rounded
        assert_eq!(decimal_to_base_units("0.0000001", 6), None);
        for bad in ["", ".", "1.", ".5", "-1", "1e3", "1.2.3", "１", "0x10"] {
            assert_eq!(decimal_to_base_units(bad, 8), None, "must reject {bad:?}");
        }
        // THE overflow boundary (review BLOCKER fold): the digit cap bounds
        // the mantissa, not value×10^decimals — the scaled product must fail
        // typed at u128::MAX, never wrap (release builds have no overflow
        // checks; a wrapped amount would be silent wrong money on the wire).
        // u128::MAX = 340282366920938463463374607431768211455:
        assert_eq!(
            decimal_to_base_units("340282366920938463463", 18),
            Some(340_282_366_920_938_463_463_000_000_000_000_000_000),
            "exactly-fits stays exact"
        );
        assert_eq!(
            decimal_to_base_units("340282366920938463464", 18),
            None,
            "one past the fit must be None, not a wrapped number"
        );
        assert_eq!(decimal_to_base_units("400000000000000000000", 18), None);
        assert_eq!(decimal_to_base_units("4", 38), None, "4×10^38 > u128::MAX");

        assert_eq!(parse_base_units("100000000"), Some(100_000_000));
        for bad in ["", "1.0", "-1", " 1", "999999999999999999999999999999"] {
            assert_eq!(parse_base_units(bad), None, "must reject {bad:?}");
        }
        // SWAP_DECIMAL_MAX_DIGITS = 27, at its exact boundary (gate 7)
        assert_eq!(
            parse_base_units(&"9".repeat(27)),
            Some(999_999_999_999_999_999_999_999_999)
        );
        assert_eq!(parse_base_units(&"1".repeat(28)), None, "28 digits > cap");

        // PROVIDER_STR_MAX_BYTES = 256, at its exact boundary (gate 7)
        assert!(bounded(&"x".repeat(PROVIDER_STR_MAX_BYTES)).is_ok());
        assert!(bounded(&"x".repeat(PROVIDER_STR_MAX_BYTES + 1)).is_err());
    }

    proptest! {
        /// format∘parse identity: any base-unit value within the digit cap
        /// renders to a decimal string that converts back exactly.
        #[test]
        fn units_roundtrip(v in 0u128..1_000_000_000_000_000_000_000_000_000, decimals in 0u8..=18) {
            let s = format_units(v, decimals).expect("≤38 decimals always formats");
            prop_assert_eq!(decimal_to_base_units(&s, decimals), Some(v));
        }

        #[test]
        fn decimal_to_base_units_never_panics(s in "\\PC{0,40}", d in 0u8..=38) {
            let _ = decimal_to_base_units(&s, d);
        }

        /// Directed at the overflow region the generic never-panics generator
        /// can't reach (review fold): pure digit strings up to the 27-digit
        /// cap × the full token-validated decimals range. The expected value
        /// is recomputed with independent checked arithmetic — the function
        /// must agree exactly, returning None precisely where u128 ends.
        #[test]
        fn scaled_conversion_is_exact_or_none_never_wrapped(
            s in "[0-9]{1,27}",
            d in 0u8..=38,
        ) {
            let expected = s
                .parse::<u128>()
                .ok()
                .and_then(|v| v.checked_mul(10u128.checked_pow(u32::from(d))?));
            prop_assert_eq!(decimal_to_base_units(&s, d), expected);
        }
    }
}
