//! Fixture KATs — the spec §8 row `one_click_wire_contract_pinned_by_fixtures`:
//! the recorded (and labeled-derived, see `UPSTREAM.md`) 1Click responses
//! are this adapter's known-answer vectors. Provider drift fails HERE,
//! never in production.

use zec_wallet_core::{ProviderProtocolReason, SwapDirection, SwapError, SwapStatus};

use crate::map::{self, SwapAsset};
use crate::wire;

macro_rules! fixture {
    ($rel:literal) => {
        include_bytes!(concat!("../tests/fixtures/", $rel)).as_slice()
    };
}

fn usdc_near() -> SwapAsset {
    SwapAsset {
        chain: "near".into(),
        symbol: "USDC".into(),
        decimals: 6,
        provider_asset_id:
            "nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1".into(),
        price_usd: Some(0.99981),
    }
}

fn zec() -> SwapAsset {
    SwapAsset {
        chain: "zec".into(),
        symbol: "ZEC".into(),
        decimals: 8,
        provider_asset_id: "nep141:zec.omft.near".into(),
        price_usd: Some(416.92),
    }
}

fn out_of_zec() -> SwapDirection {
    SwapDirection::OutOfZec {
        to: zec_wallet_core::AssetId {
            chain: "near".into(),
            symbol: "USDC".into(),
        },
    }
}

/// Decimals resolver over the two assets the fixtures use.
fn resolve(asset_id: &str) -> Option<u8> {
    [zec(), usdc_near()]
        .into_iter()
        .find(|t| t.provider_asset_id == asset_id)
        .map(|t| t.decimals)
}

fn expect_protocol(err: SwapError, want: ProviderProtocolReason) {
    match err {
        SwapError::ProviderProtocol { reason } if reason == want => {}
        other => panic!("expected ProviderProtocol({want:?}), got {other:?}"),
    }
}

/// The named §8 row, in one sweep: every fixture decodes (or refuses) to
/// EXACTLY the pinned outcome.
#[test]
fn one_click_wire_contract_pinned_by_fixtures() {
    // ── recorded: the full live token list decodes, maps, and contains the
    // pinned ZEC listing (decimals 8 = base units are zatoshis)
    let tokens =
        wire::decode_tokens_response(fixture!("recorded/tokens.json")).expect("tokens decode");
    assert_eq!(tokens.len(), 188, "recorded token count");
    let assets = map::map_tokens(tokens).expect("tokens map");
    let zec_listing = assets
        .iter()
        .find(|a| a.chain == "zec" && a.symbol == "ZEC")
        .expect("ZEC listed");
    assert_eq!(zec_listing.decimals, 8);
    assert_eq!(zec_listing.provider_asset_id, "nep141:zec.omft.near");

    // ── recorded: both dry quotes decode; a DRY response can never become a
    // port quote (no depositAddress ⇒ typed MissingField — the non-dry
    // requirement bites)
    for f in [
        fixture!("recorded/quote_dry_out_of_zec.json"),
        fixture!("recorded/quote_dry_into_zec.json"),
    ] {
        let wire_resp = wire::decode_quote_response(f).expect("dry quote decodes");
        assert!(wire_resp.quote.deposit_address.is_none());
        let err = map::map_out_of_zec_quote(wire_resp, &out_of_zec(), &zec(), &usdc_near())
            .expect_err("dry quote must not map");
        expect_protocol(err, ProviderProtocolReason::MissingField);
    }

    // ── derived: the registering (non-dry) quote maps to the exact port DTO
    let wire_resp = wire::decode_quote_response(fixture!("derived/quote_live_out_of_zec.json"))
        .expect("live quote decodes");
    let quote = map::map_out_of_zec_quote(wire_resp, &out_of_zec(), &zec(), &usdc_near())
        .expect("live quote maps");
    assert_eq!(quote.id.as_str(), "t1ZXKD285AZdKs56b9ypo9FT4LMHKuJRw5N");
    assert_eq!(quote.deposit_address, "t1ZXKD285AZdKs56b9ypo9FT4LMHKuJRw5N");
    assert_eq!(quote.expires_at, 1_781_438_400); // 2026-06-14T12:00:00Z
    assert_eq!(quote.amount_in, "1"); // 100000000 zatoshis, human form
    assert_eq!(quote.min_amount_out, "412.248474"); // 412248474 @ 6 decimals
    assert_eq!(quote.zec_side.zat(), 100_000_000);
    assert_eq!(
        quote.refund_to.as_deref(),
        Some("t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs"),
        "the echo the service verifies"
    );
    assert!(quote.disclosure.deshields && !quote.disclosure.ends_shielded);

    // ── derived: a memo-requiring deposit address is REFUSED (depositing
    // without the memo loses funds — never silently dropped)
    let wire_resp = wire::decode_quote_response(fixture!("derived/quote_deposit_memo.json"))
        .expect("memo quote decodes");
    let err = map::map_out_of_zec_quote(wire_resp, &out_of_zec(), &zec(), &usdc_near())
        .expect_err("memo quote must be refused");
    expect_protocol(err, ProviderProtocolReason::DepositMemoUnsupported);

    // ── derived: the status family maps per the §3.2 fixed table
    let status = |f: &[u8]| {
        map::map_status(
            wire::decode_status_response(f).expect("status decodes"),
            resolve,
        )
        .expect("status maps")
    };
    assert_eq!(
        status(fixture!("derived/status_pending_deposit.json")),
        SwapStatus::PendingDeposit {
            expires_at: 1_781_438_400
        }
    );
    assert_eq!(
        status(fixture!("derived/status_incomplete_deposit.json")),
        SwapStatus::UnderDeposited {
            received: "0.4".into(), // 40000000 of the
            missing: "0.6".into(),  // expected 100000000, ZEC decimals
            deadline: 1_781_438_400,
        }
    );
    // money-honesty edge (money-lens fold): an OVER-deposit reported as
    // INCOMPLETE is a provider contradiction — `missing` must SATURATE to "0",
    // never wrap to a huge "you still owe" figure. 120000000 deposited vs
    // 100000000 expected ⇒ received "1.2", missing "0".
    assert_eq!(
        status(fixture!("derived/status_over_deposit.json")),
        SwapStatus::UnderDeposited {
            received: "1.2".into(),
            missing: "0".into(),
            deadline: 1_781_438_400,
        },
        "over-deposit: missing saturates to 0, never wraps"
    );
    assert_eq!(
        status(fixture!("derived/status_processing.json")),
        SwapStatus::Processing
    );
    assert_eq!(
        status(fixture!("derived/status_success.json")),
        SwapStatus::Success {
            out_txid: Some("8GnQCAghnYaLYFLDTbDVHDvVfPGuLrbMBuJha7nrqs2r".into()),
            realized_slippage_bps: Some(12),
        }
    );
    assert_eq!(
        status(fixture!("derived/status_refunded.json")),
        SwapStatus::Refunded {
            // the LAST origin-chain tx (the first is the user's own deposit)
            refund_txid: Some(
                "6f7cf9580f1c2dfb3c4d5e6a7b8091a2b3c4d5e6f708192a3b4c5d6e7f809102".into()
            ),
        }
    );
    assert_eq!(
        status(fixture!("derived/status_failed.json")),
        SwapStatus::Failed {
            code: zec_wallet_core::SwapFailureCode::ProviderFailure
        }
    );
    // the closed-enum rule: an unknown lifecycle word is a typed terminal
    // STATUS (tracking broke), never a decode error and never a guess
    assert_eq!(
        status(fixture!("derived/status_unknown_variant.json")),
        SwapStatus::Failed {
            code: zec_wallet_core::SwapFailureCode::ProviderProtocol
        }
    );

    // ── recorded error envelopes: NEVER decodable as success shapes — even
    // a status-mapping bug cannot misread an error body (and the bodies are
    // never logged: the 404 message echoes the queried address, §5.4)
    for f in [
        fixture!("recorded/quote_bad_request_400.json"),
        fixture!("recorded/status_not_found_404.json"),
    ] {
        expect_protocol(
            wire::decode_quote_response(f).expect_err("error body is not a quote"),
            ProviderProtocolReason::UndecodableResponse,
        );
        expect_protocol(
            wire::decode_status_response(f).expect_err("error body is not a status"),
            ProviderProtocolReason::UndecodableResponse,
        );
    }
}

/// The quote door's echo check passes a FAITHFUL echo in the provider's own
/// JSON types, read from the two RECORDED (real, captured) quote responses,
/// one per direction: an integer `slippageTolerance`, a string `amount`, the
/// rest strings and a bool. Every test that refuses an echo is built by
/// editing these shapes, so this is what shows a real quote is not refused for
/// a type difference (the final code review of the 2026-10-07 fixes).
#[test]
fn the_recorded_real_echoes_pass_the_quote_door_check() {
    use crate::wire::WireQuoteRequest;
    let usdc = "nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1";
    let zec = "nep141:zec.omft.near";
    let sent = |origin: &str, dest: &str, refund: &str, recipient: &str| WireQuoteRequest {
        dry: true,
        swap_type: "EXACT_INPUT",
        slippage_tolerance: 100,
        origin_asset: origin.into(),
        deposit_type: "ORIGIN_CHAIN",
        destination_asset: dest.into(),
        amount: "100000000".into(),
        refund_to: refund.into(),
        refund_type: "ORIGIN_CHAIN",
        recipient: recipient.into(),
        recipient_type: "DESTINATION_CHAIN",
        deadline: "2026-06-14T12:00:00.000Z".into(),
    };
    let t = "t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs";
    for (fixture, request) in [
        (
            fixture!("recorded/quote_dry_out_of_zec.json"),
            sent(zec, usdc, t, "example.near"),
        ),
        (
            fixture!("recorded/quote_dry_into_zec.json"),
            sent(usdc, zec, "example.near", t),
        ),
    ] {
        let response = wire::decode_quote_response(fixture).expect("recorded quote decodes");
        map::check_request_echo(&request, &response.quote_request)
            .expect("a faithful real echo passes the check");
    }
}

/// The echo fields added for the quote door's check (2026-10-07 review,
/// finding 3) must not make a STATUS decode stricter: a swap being polled may
/// already hold the user's deposit. Odd types in every new field still decode.
#[test]
fn a_status_body_with_odd_echo_types_still_decodes() {
    let mut v: serde_json::Value =
        serde_json::from_slice(fixture!("derived/status_success.json")).expect("fixture");
    let echo = v["quoteResponse"]["quoteRequest"]
        .as_object_mut()
        .expect("the status embeds the request echo");
    for key in [
        "dry",
        "swapType",
        "slippageTolerance",
        "depositType",
        "destinationAsset",
        "amount",
        "refundType",
        "recipient",
        "recipientType",
    ] {
        echo.insert(key.to_owned(), serde_json::json!({"odd": [1, "x", null]}));
    }
    let body = serde_json::to_vec(&v).expect("re-encode");
    assert!(
        wire::decode_status_response(&body).is_ok(),
        "the status decodes whatever the new echo fields hold"
    );
}

/// Decode funnels never panic on hostile bytes (the fuzz target's
/// property-test twin; full funnel incl. mapping).
#[test]
fn decode_funnels_never_panic_on_mutations() {
    use proptest::prelude::*;
    let mut runner = proptest::test_runner::TestRunner::deterministic();
    let base = fixture!("derived/status_success.json");
    runner
        .run(
            &(
                proptest::collection::vec(any::<u8>(), 0..512),
                0usize..base.len(),
            ),
            |(junk, at)| {
                let mut mutated = base.to_vec();
                mutated.splice(at..at, junk);
                if let Ok(resp) = wire::decode_status_response(&mutated) {
                    let _ = map::map_status(resp, resolve);
                }
                Ok(())
            },
        )
        .expect("no panics");
}

fn into_zec() -> SwapDirection {
    SwapDirection::IntoZec {
        from: zec_wallet_core::AssetId {
            chain: "near".into(),
            symbol: "USDC".into(),
        },
    }
}

/// IntoZec NOW honors a required deposit memo: the USER attaches it to their
/// EXTERNAL deposit (§4.4), so it is bounded + carried to the port DTO and shown
/// on the D7 screen — never dropped (omitting a required memo loses funds). The
/// asymmetry is the money invariant: OutOfZec, where the WALLET sends its own
/// deposit and cannot attach a source-chain memo, STILL refuses the same quote.
#[test]
fn into_zec_carries_a_required_deposit_memo_while_out_of_zec_refuses() {
    let decode = || {
        wire::decode_quote_response(fixture!("derived/quote_deposit_memo.json"))
            .expect("memo quote decodes")
    };

    // IntoZec (origin = foreign USDC, dest = ZEC): the memo is carried verbatim
    let quote = map::map_into_zec_quote(decode(), &into_zec(), &usdc_near(), &zec())
        .expect("IntoZec memo quote maps");
    assert_eq!(
        quote.deposit_memo.as_deref(),
        Some("100345677"),
        "the required memo reaches the DTO (shown on D7, never silently dropped)"
    );
    assert_eq!(quote.deposit_address, "t1ZXKD285AZdKs56b9ypo9FT4LMHKuJRw5N");
    // IntoZec zec_side = the OUTPUT (min_amount_out as zatoshis; dest = ZEC, 8 dp)
    assert_eq!(quote.zec_side.zat(), 412_248_474);
    assert!(quote.disclosure.ends_shielded && !quote.disclosure.deshields);

    // OutOfZec: the SAME memo is a permanent typed refusal (funds-loss guard)
    let err = map::map_out_of_zec_quote(decode(), &out_of_zec(), &zec(), &usdc_near())
        .expect_err("OutOfZec memo quote is refused");
    expect_protocol(err, ProviderProtocolReason::DepositMemoUnsupported);
}

/// The deposit memo is untrusted provider input (§4.6): an empty memo is no memo
/// (normalized to None so the UI renders nothing rather than an empty "required"
/// field), and an oversized one is refused at the funnel BEFORE it can reach the
/// port DTO / cross the FFI.
#[test]
fn into_zec_deposit_memo_is_bounded_and_empty_normalizes_to_none() {
    use zec_wallet_core::constants::PROVIDER_STR_MAX_BYTES;

    let mut resp =
        wire::decode_quote_response(fixture!("derived/quote_deposit_memo.json")).expect("decodes");
    resp.quote.deposit_memo = Some(String::new());
    let quote = map::map_into_zec_quote(resp, &into_zec(), &usdc_near(), &zec())
        .expect("empty-memo quote still maps");
    assert_eq!(quote.deposit_memo, None, "an empty memo is no memo");

    let mut resp =
        wire::decode_quote_response(fixture!("derived/quote_deposit_memo.json")).expect("decodes");
    resp.quote.deposit_memo = Some("x".repeat(PROVIDER_STR_MAX_BYTES + 1));
    let err = map::map_into_zec_quote(resp, &into_zec(), &usdc_near(), &zec())
        .expect_err("oversized memo is refused at the funnel");
    expect_protocol(err, ProviderProtocolReason::OversizedField);
}

/// A memo whose displayed glyphs could differ from its bytes (a CR/LF split, a
/// bidi-override reversing digits, a zero-width hidden char, a NUL) is a money-
/// display SPOOF: REJECTED, never silently stripped — a required memo must reach
/// the user byte-exact. Mirrors the IZ-4 refund-address control-byte hardening.
#[test]
fn into_zec_rejects_a_display_unsafe_deposit_memo() {
    for hostile in [
        "100\n345677",       // CR/LF: a smuggled "second line"
        "100\u{202E}345677", // RTL override: reverses the displayed digits
        "100\u{200B}345677", // zero-width space: a hidden char
        "tag\u{0000}",       // NUL
    ] {
        let mut resp = wire::decode_quote_response(fixture!("derived/quote_deposit_memo.json"))
            .expect("decodes");
        resp.quote.deposit_memo = Some(hostile.to_string());
        let err = map::map_into_zec_quote(resp, &into_zec(), &usdc_near(), &zec())
            .expect_err("a display-unsafe memo is refused");
        expect_protocol(err, ProviderProtocolReason::UndecodableResponse);
    }
}

/// The memo bound is on BYTES (`bounded`), not chars: a multi-byte UTF-8 memo is
/// accepted up to 256 BYTES and carried byte-exact; one byte over rejects. And a
/// legitimate non-ASCII / numeric / alphanumeric memo is NOT a display-spoof —
/// `memo_display_unsafe` must not over-reject printable Unicode (else real memo
/// chains become un-offerable, a money-availability regression).
#[test]
fn into_zec_deposit_memo_byte_bound_and_legit_unicode() {
    use zec_wallet_core::constants::PROVIDER_STR_MAX_BYTES;

    let map_memo = |memo: &str| {
        let mut resp = wire::decode_quote_response(fixture!("derived/quote_deposit_memo.json"))
            .expect("decodes");
        resp.quote.deposit_memo = Some(memo.to_string());
        map::map_into_zec_quote(resp, &into_zec(), &usdc_near(), &zec())
    };

    // EXACTLY at the byte bound (ASCII): accepted + carried byte-exact (gate 7)
    let at_bound = "x".repeat(PROVIDER_STR_MAX_BYTES);
    assert_eq!(
        map_memo(&at_bound)
            .expect("at-bound memo maps")
            .deposit_memo,
        Some(at_bound.clone())
    );

    // multi-byte UTF-8: 255 bytes (85 × 3-byte 'あ') accepted + carried...
    let utf8_under = "\u{3042}".repeat(85);
    assert_eq!(utf8_under.len(), 255);
    assert_eq!(
        map_memo(&utf8_under)
            .expect("255-byte memo maps")
            .deposit_memo,
        Some(utf8_under)
    );
    // ...258 bytes (86 × 3) rejects OversizedField (a BYTE bound, not a char count)
    expect_protocol(
        map_memo(&"\u{3042}".repeat(86)).expect_err("258-byte memo refused"),
        ProviderProtocolReason::OversizedField,
    );

    // a legitimate numeric XRP tag, an alphanumeric memo, accented Latin, and CJK
    // all carry VERBATIM — none is display-unsafe
    for legit in [
        "100345677",
        "order-2024",
        "Caf\u{e9}-\u{00dc}ber",
        "\u{3042}\u{3044}",
    ] {
        assert_eq!(
            map_memo(legit)
                .expect("legit memo maps")
                .deposit_memo
                .as_deref(),
            Some(legit),
            "a legitimate memo must carry verbatim"
        );
    }
}

/// A whitespace-only memo is carried VERBATIM, never trimmed — a deposit memo is
/// a funds-critical value we never silently alter (pins the decision so a future
/// "trim" is a conscious change, not silent drift). Distinct from "" → None.
#[test]
fn into_zec_whitespace_only_memo_is_carried_verbatim() {
    let mut resp =
        wire::decode_quote_response(fixture!("derived/quote_deposit_memo.json")).expect("decodes");
    resp.quote.deposit_memo = Some("   ".to_string());
    let quote = map::map_into_zec_quote(resp, &into_zec(), &usdc_near(), &zec())
        .expect("whitespace memo maps");
    assert_eq!(quote.deposit_memo.as_deref(), Some("   "), "never trimmed");
}
