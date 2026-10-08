//! ZIP-321 payment-URI codec (spec §3.1 free functions; §4.6 hostile-input
//! funnel). Parsing is REUSED from the audited `zip321` crate WHOLE — this
//! module is the validation funnel only: size-cap BEFORE parse, a network
//! check on every recipient, and re-validation of every leg through
//! [`Payment::new`] so nothing that skipped our constructor can exist.
//!
//! ERROR POSTURE (§5.4): URIs carry addresses and amounts — the never-log
//! list. Every upstream error value (which may echo URI content in its
//! message) is DROPPED and collapsed to the payload-free
//! [`WalletError::PaymentUriInvalid`]; failures our own validators can
//! attribute stay precise ([`WalletError::NetworkMismatch`],
//! [`WalletError::ReservedMemoNotSendable`], …) and are payload-free too.

use crate::constants::PAYMENT_URI_MAX_BYTES;
use crate::error::WalletError;
use crate::memo::{Address, Memo, Payment, PaymentRequest};
use crate::money::{Network, Zatoshis};

/// Parse a `zcash:` payment URI (QR codes, links — HOSTILE input) into a
/// validated [`PaymentRequest`].
///
/// The bare donation form `zcash:ADDR` (no amount) is valid — the leg comes
/// back with `amount: None` (sender specifies; see [`Payment::amount`]).
/// A URI with no payment at all (`zcash:`) is rejected: it requests nothing.
pub fn parse_payment_uri(uri: &str, network: Network) -> Result<PaymentRequest, WalletError> {
    // size-cap BEFORE the parser ever sees the bytes (§4.6)
    if uri.len() > PAYMENT_URI_MAX_BYTES {
        return Err(WalletError::PaymentUriInvalid);
    }
    let parsed =
        zip321::TransactionRequest::from_uri(uri).map_err(|_| WalletError::PaymentUriInvalid)?;
    if parsed.payments().is_empty() {
        return Err(WalletError::PaymentUriInvalid);
    }
    let payments = parsed
        .payments()
        .values()
        .map(|leg| {
            // every recipient through OUR door: re-encoded canonically, then
            // network-checked (`zip321` validates structure, not network)
            let recipient = Address::parse(&leg.recipient_address().encode(), network)?;
            let amount = leg.amount().map(zatoshis_from_protocol).transpose()?;
            let memo = match leg.memo() {
                None => Memo::Empty,
                Some(mb) => Memo::from_wire(mb)?,
            };
            // unknown `req-` params were already rejected upstream (the
            // ZIP-321 MUST rule); remaining unknown params are ignorable by
            // spec and intentionally dropped here.
            Payment::new(
                recipient,
                amount,
                memo,
                leg.label().cloned(),
                leg.message().cloned(),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PaymentRequest { payments })
}

/// Lower a validated [`PaymentRequest`] to the audited `zip321::TransactionRequest`
/// — the ONE conversion shared by the URI encoder ([`encode_payment_uri`]) and the
/// send path (`send::propose`, which then drives `propose_transfer`). One door, so
/// the bytes the user signs and the bytes the URI shows can never drift (DRY).
///
/// All legs must target ONE network (`network_mismatch_rejected_everywhere` — a
/// request mixing mainnet and testnet recipients can only be a construction bug or
/// a cross-feed, never a valid intent). An empty request is rejected: it requests
/// nothing. Amount-LESS legs (the donation form) are PRESERVED here — the
/// amount-required-for-send rule belongs to the propose funnel (the one caller that
/// knows the request is a send, not a display), not to this generic lowering.
pub(crate) fn to_transaction_request(
    request: &PaymentRequest,
) -> Result<zip321::TransactionRequest, WalletError> {
    let first = request
        .payments
        .first()
        .ok_or(WalletError::PaymentUriInvalid)?;
    let network = first.recipient.network();

    let legs = request
        .payments
        .iter()
        .map(|p| {
            if p.recipient.network() != network {
                return Err(WalletError::NetworkMismatch);
            }
            // `Payment`'s fields are pub (spec §2.4 DTO shape), so a struct
            // literal can bypass `Payment::new` — re-assert the consensus
            // rule here PRECISELY (zip321's own check below would catch it
            // too, but collapsed into the generic URI error)
            if p.recipient.transparent_only() && p.amount == Some(Zatoshis::ZERO) {
                return Err(WalletError::ZeroValuedTransparentOutput);
            }
            // Address is parse-validated, so re-decoding its canonical
            // encoding cannot fail — mapped typed anyway (no expect).
            let addr = zcash_address::ZcashAddress::try_from_encoded(p.recipient.encoded())
                .map_err(|_| WalletError::AddressInvalid)?;
            let amount = p.amount.map(zatoshis_to_protocol).transpose()?;
            let memo = match &p.memo {
                Memo::Empty => None,
                m => Some(m.to_wire()?),
            };
            // our `Payment::new` mirrors zip321's construction rules, so a
            // rejection here is unreachable — collapsed typed, never a panic
            zip321::Payment::new(
                addr,
                amount,
                memo,
                p.label.clone(),
                p.message.clone(),
                vec![],
            )
            .map_err(|_| WalletError::PaymentUriInvalid)
        })
        .collect::<Result<Vec<_>, _>>()?;

    zip321::TransactionRequest::new(legs).map_err(|_| WalletError::PaymentUriInvalid)
}

/// Encode a validated [`PaymentRequest`] as a ZIP-321 `zcash:` URI. Lowers through
/// the shared [`to_transaction_request`] door, then serializes.
pub fn encode_payment_uri(request: &PaymentRequest) -> Result<String, WalletError> {
    Ok(to_transaction_request(request)?.to_uri())
}

/// `zcash_protocol` zatoshis → ours. Same `MAX_MONEY` bound on both sides,
/// so the error arm is a belt-and-braces typed reject, not a live path.
/// `pub(crate)` since inc-2d-1: the send path maps proposal fee/change/recipient
/// amounts (upstream `Zatoshis`) into the SDK DTO through this one door.
pub(crate) fn zatoshis_from_protocol(
    v: zcash_protocol::value::Zatoshis,
) -> Result<Zatoshis, WalletError> {
    let zat = i64::try_from(v.into_u64()).map_err(|_| WalletError::AmountOutOfRange)?;
    Zatoshis::new(zat)
}

/// Ours → `zcash_protocol` zatoshis (same bound; same posture as above).
fn zatoshis_to_protocol(v: Zatoshis) -> Result<zcash_protocol::value::Zatoshis, WalletError> {
    let zat = u64::try_from(v.zat()).map_err(|_| WalletError::AmountOutOfRange)?;
    zcash_protocol::value::Zatoshis::from_u64(zat).map_err(|_| WalletError::AmountOutOfRange)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use zcash_protocol::consensus::NetworkType;
    use zcash_protocol::memo::MemoBytes;

    use super::*;
    use crate::constants::MAX_MONEY_ZAT;
    use crate::memo::AddressKind;

    // ── Official ZIP-321 vectors (lifted byte-exact from the ZIP; also the
    //    zip321 crate's own KATs — ours prove the FUNNEL preserves them) ──

    const ZIP321_VALID_SIMPLE: &str = "zcash:ztestsapling10yy2ex5dcqkclhc7z7yrnjq2z6feyjad56ptwlfgmy77dmaqqrl9gyhprdx59qgmsnyfska2kez?amount=1&memo=VGhpcyBpcyBhIHNpbXBsZSBtZW1vLg&message=Thank%20you%20for%20your%20purchase";
    const ZIP321_VALID_MULTI: &str = "zcash:?address=tmEZhbWHTpdKMw5it8YDspUXSMGQyFwovpU&amount=123.456&address.1=ztestsapling10yy2ex5dcqkclhc7z7yrnjq2z6feyjad56ptwlfgmy77dmaqqrl9gyhprdx59qgmsnyfska2kez&amount.1=0.789&memo.1=VGhpcyBpcyBhIHVuaWNvZGUgbWVtbyDinKjwn6aE8J-PhvCfjok";
    const ZIP321_VALID_MAX_MONEY: &str = "zcash:ztestsapling10yy2ex5dcqkclhc7z7yrnjq2z6feyjad56ptwlfgmy77dmaqqrl9gyhprdx59qgmsnyfska2kez?amount=21000000";

    fn encoded_sapling(net: NetworkType) -> String {
        use zcash_address::ToAddress;
        zcash_address::ZcashAddress::from_sapling(net, [0xAB; 43]).encode()
    }

    fn encoded_p2pkh(net: NetworkType) -> String {
        use zcash_address::ToAddress;
        zcash_address::ZcashAddress::from_transparent_p2pkh(net, [0xCD; 20]).encode()
    }

    fn encoded_unified(net: &NetworkType) -> String {
        use zcash_address::unified::{Address as Ua, Encoding, Receiver};
        Ua::try_from_items(vec![
            Receiver::Orchard([0xEF; 43]),
            Receiver::Sapling([0xAB; 43]),
        ])
        .expect("valid receiver set")
        .encode(net)
    }

    fn memo_param(bytes: &[u8]) -> String {
        zip321::memo_to_base64(&MemoBytes::from_bytes(bytes).expect("fits the field"))
    }

    #[test]
    fn zip321_official_vectors_parse_through_the_funnel() {
        // §8 "ZIP-321 round-trip vectors" — the simple single-payment example
        let req = parse_payment_uri(ZIP321_VALID_SIMPLE, Network::Test).expect("official vector");
        assert_eq!(req.payments.len(), 1);
        let leg = &req.payments[0];
        assert_eq!(leg.recipient.kind(), AddressKind::Sapling);
        assert_eq!(leg.amount, Some(Zatoshis::new(100_000_000).expect("1 ZEC")));
        assert_eq!(
            leg.memo,
            Memo::text("This is a simple memo.").expect("valid")
        );
        assert_eq!(leg.message.as_deref(), Some("Thank you for your purchase"));
        assert_eq!(leg.label, None);

        // the multi-payment example: transparent leg + shielded leg with a
        // unicode memo; BTreeMap order = payment-index order
        let req = parse_payment_uri(ZIP321_VALID_MULTI, Network::Test).expect("official vector");
        assert_eq!(req.payments.len(), 2);
        assert_eq!(req.payments[0].recipient.kind(), AddressKind::Transparent);
        assert_eq!(
            req.payments[0].amount,
            Some(Zatoshis::new(12_345_600_000).expect("123.456 ZEC"))
        );
        assert_eq!(req.payments[0].memo, Memo::Empty);
        assert_eq!(req.payments[1].recipient.kind(), AddressKind::Sapling);
        assert_eq!(
            req.payments[1].amount,
            Some(Zatoshis::new(78_900_000).expect("0.789 ZEC"))
        );
        assert_eq!(
            req.payments[1].memo,
            Memo::text("This is a unicode memo ✨🦄🏆🎉").expect("valid")
        );

        // the exact-MAX_MONEY boundary vector parses (and pins our bound to
        // the protocol's)
        let req =
            parse_payment_uri(ZIP321_VALID_MAX_MONEY, Network::Test).expect("official vector");
        assert_eq!(
            req.payments[0].amount,
            Some(Zatoshis::new(MAX_MONEY_ZAT).expect("max supply"))
        );

        // the bare donation form: address only, amount left to the sender
        let donation = format!("zcash:{}", encoded_sapling(NetworkType::Test));
        let req = parse_payment_uri(&donation, Network::Test).expect("donation QR is valid");
        assert_eq!(req.payments.len(), 1);
        assert_eq!(req.payments[0].amount, None);
        assert_eq!(req.payments[0].memo, Memo::Empty);
    }

    #[test]
    fn zip321_parser_rejects_oversized_and_malformed() {
        // §8 named test. Oversize FIRST: same valid shape, one byte past the
        // cap — proving the cap (not the parser) is the discriminator.
        let base = format!("zcash:{}?message=", encoded_sapling(NetworkType::Test));
        let fits = format!("{base}{}", "a".repeat(PAYMENT_URI_MAX_BYTES - base.len()));
        assert!(parse_payment_uri(&fits, Network::Test).is_ok());
        let oversized = format!(
            "{base}{}",
            "a".repeat(PAYMENT_URI_MAX_BYTES - base.len() + 1)
        );
        assert!(matches!(
            parse_payment_uri(&oversized, Network::Test),
            Err(WalletError::PaymentUriInvalid)
        ));

        // the official ZIP-321 invalid examples (representative set), plus
        // non-URI garbage and the empty request forms — ALL collapse to the
        // payload-free typed error
        for invalid in [
            // missing `address=` for index 0
            "zcash:?amount=3491405.05201255&address.1=ztestsapling10yy2ex5dcqkclhc7z7yrnjq2z6feyjad56ptwlfgmy77dmaqqrl9gyhprdx59qgmsnyfska2kez&amount.1=5740296.87793245",
            // duplicate `amount=`
            "zcash:?amount=1.234&amount=2.345&address=tmEZhbWHTpdKMw5it8YDspUXSMGQyFwovpU",
            // memo on a transparent recipient (upstream TransparentMemo)
            "zcash:?address=tmEZhbWHTpdKMw5it8YDspUXSMGQyFwovpU&amount=123.456&memo=eyAia2V5IjogIlRoaXMgaXMgYSBKU09OLXN0cnVjdHVyZWQgbWVtby4iIH0",
            // amount beyond MAX_MONEY / i64-wrap / negative / trailing dot
            "zcash:ztestsapling10yy2ex5dcqkclhc7z7yrnjq2z6feyjad56ptwlfgmy77dmaqqrl9gyhprdx59qgmsnyfska2kez?amount=21000000.00000001",
            "zcash:ztestsapling10yy2ex5dcqkclhc7z7yrnjq2z6feyjad56ptwlfgmy77dmaqqrl9gyhprdx59qgmsnyfska2kez?amount=18446744073709551624",
            "zcash:ztestsapling10yy2ex5dcqkclhc7z7yrnjq2z6feyjad56ptwlfgmy77dmaqqrl9gyhprdx59qgmsnyfska2kez?amount=-1",
            "zcash:?address=tmEZhbWHTpdKMw5it8YDspUXSMGQyFwovpU&amount=123.",
            // unrecognized REQUIRED parameter (the ZIP-321 MUST-reject rule)
            "zcash:?address=tmEZhbWHTpdKMw5it8YDspUXSMGQyFwovpU&amount=123.45&req-unknown=x",
            // not ZIP-321 at all
            "",
            "garbage",
            "http://example.com",
            "bitcoin:tmEZhbWHTpdKMw5it8YDspUXSMGQyFwovpU",
            // structurally valid but requests NOTHING — useless as an intent
            "zcash:",
            "zcash:?",
        ] {
            assert!(
                matches!(
                    parse_payment_uri(invalid, Network::Test),
                    Err(WalletError::PaymentUriInvalid)
                ),
                "should reject typed: {invalid:.40}"
            );
        }
    }

    #[test]
    fn network_mismatch_on_payment_uris_is_typed() {
        // §8 network_mismatch_rejected_everywhere — the URI arm: a TESTNET
        // vector under a Main config is a NETWORK error, never 'malformed'
        assert!(matches!(
            parse_payment_uri(ZIP321_VALID_SIMPLE, Network::Main),
            Err(WalletError::NetworkMismatch)
        ));
        // and the other way around
        let mainnet = format!("zcash:{}?amount=1", encoded_sapling(NetworkType::Main));
        assert!(parse_payment_uri(&mainnet, Network::Main).is_ok());
        assert!(matches!(
            parse_payment_uri(&mainnet, Network::Test),
            Err(WalletError::NetworkMismatch)
        ));
    }

    #[test]
    fn payment_uri_memo_arms_are_funneled() {
        let shielded = encoded_sapling(NetworkType::Test);

        // a hostile URI CAN carry reserved ZIP-302 framing (zip321 accepts
        // any ≤512 memo bytes) — the funnel rejects it as a send intent
        let reserved = format!("zcash:{shielded}?amount=1&memo={}", memo_param(&[0xF5]));
        assert!(matches!(
            parse_payment_uri(&reserved, Network::Test),
            Err(WalletError::ReservedMemoNotSendable)
        ));

        // text-lead memo with invalid UTF-8: the one upstream-invalid shape
        let bad_utf8 = format!(
            "zcash:{shielded}?amount=1&memo={}",
            memo_param(&[0x41, 0xFF])
        );
        assert!(matches!(
            parse_payment_uri(&bad_utf8, Network::Test),
            Err(WalletError::MemoInvalid)
        ));

        // 0xFF arbitrary bytes ride through — padded to the full 511 (the
        // wire has no length framing; documented on `Memo::arbitrary`)
        let arb = format!(
            "zcash:{shielded}?amount=1&memo={}",
            memo_param(&[0xFF, 1, 2, 3])
        );
        let req = parse_payment_uri(&arb, Network::Test).expect("arbitrary memo is sendable");
        let Memo::Arbitrary(bytes) = &req.payments[0].memo else {
            panic!("expected the arbitrary arm");
        };
        assert_eq!(bytes.len(), 511);
        assert_eq!(&bytes[..3], &[1, 2, 3]);
    }

    /// FR-28 — the COMPOSE side of a machine memo, asserted BYTE-FOR-BYTE
    /// through the audited encoder and back through the audited parser.
    ///
    /// The plan for this feature was explicit that a length assertion proves
    /// nothing (`ParsedMemo` already gives a length, and it is the same length
    /// for every 0xFF memo ever written — the field is padded). So this
    /// compares the actual bytes, and pins the padding rule with them: the
    /// host's bytes lead, zeros follow, and the field is always 511. A host
    /// that needs its own length frames it INSIDE the bytes; the SDK does not
    /// and must not invent framing on its behalf.
    #[test]
    fn arbitrary_memo_survives_encode_then_parse_byte_for_byte() {
        let shielded =
            Address::parse(&encoded_sapling(NetworkType::Test), Network::Test).expect("parses");
        // A host envelope shape: magic, version, a length byte, payload —
        // meaningless to us, which is the point.
        let host_bytes: Vec<u8> = vec![0x52, 0x4C, 0x01, 0x04, 0xDE, 0xAD, 0xBE, 0xEF];

        let request = PaymentRequest {
            payments: vec![
                Payment::new(
                    shielded,
                    Some(Zatoshis::new(150_000).expect("amount")),
                    Memo::arbitrary(host_bytes.clone()).expect("within the 0xFF bound"),
                    None,
                    None,
                )
                .expect("a machine memo to a SHIELDED recipient is constructible"),
            ],
        };

        let uri = encode_payment_uri(&request).expect("encodes");
        let parsed = parse_payment_uri(&uri, Network::Test).expect("round-trips");
        let Memo::Arbitrary(back) = &parsed.payments[0].memo else {
            panic!("the 0xFF arm must survive the round trip, not degrade to text/empty");
        };

        assert_eq!(
            &back[..host_bytes.len()],
            &host_bytes[..],
            "the host's bytes must come back EXACTLY — a length-only assertion \
             would pass over corrupted content"
        );
        assert_eq!(back.len(), 511, "the 0xFF field is fixed-width on the wire");
        assert!(
            back[host_bytes.len()..].iter().all(|b| *b == 0),
            "everything past the host's bytes is zero PADDING, not data — a \
             host reading this back must use its own length frame"
        );
    }

    /// The negative polarity: a machine memo to a TRANSPARENT recipient is
    /// refused typed at construction, exactly as a text memo is. The bytes are
    /// never silently dropped to let the payment through — a host reference the
    /// recipient may need must not vanish (spec §2.4).
    #[test]
    fn arbitrary_memo_to_a_transparent_recipient_is_refused() {
        let transparent =
            Address::parse(&encoded_p2pkh(NetworkType::Test), Network::Test).expect("parses");
        assert!(matches!(
            Payment::new(
                transparent,
                Some(Zatoshis::new(150_000).expect("amount")),
                Memo::arbitrary(vec![1, 2, 3]).expect("within bound"),
                None,
                None,
            ),
            Err(WalletError::MemoRequiresShieldedRecipient)
        ));
    }

    #[test]
    fn encode_payment_uri_roundtrips_and_guards() {
        let shielded =
            Address::parse(&encoded_sapling(NetworkType::Test), Network::Test).expect("parses");
        let transparent =
            Address::parse(&encoded_p2pkh(NetworkType::Test), Network::Test).expect("parses");
        let unified =
            Address::parse(&encoded_unified(&NetworkType::Test), Network::Test).expect("parses");

        // multi-leg round-trip: every field survives encode → parse
        let original = PaymentRequest {
            payments: vec![
                Payment::new(
                    shielded.clone(),
                    Some(Zatoshis::new(78_900_000).expect("valid")),
                    Memo::text("round trip ✨").expect("valid"),
                    Some("label".into()),
                    Some("a message with spaces & symbols".into()),
                )
                .expect("valid leg"),
                Payment::new(
                    transparent.clone(),
                    Some(Zatoshis::new(1).expect("valid")),
                    Memo::Empty,
                    None,
                    None,
                )
                .expect("valid leg"),
                // donation leg: amount left to the sender
                Payment::new(unified, None, Memo::Empty, None, None).expect("valid leg"),
            ],
        };
        let uri = encode_payment_uri(&original).expect("encodes");
        let parsed = parse_payment_uri(&uri, Network::Test).expect("parses back");
        assert_eq!(parsed, original);

        // empty request: `zcash:` encodes "nothing", not a payment
        let empty = PaymentRequest { payments: vec![] };
        assert!(matches!(
            encode_payment_uri(&empty),
            Err(WalletError::PaymentUriInvalid)
        ));

        // mixed networks can only be a construction bug or a cross-feed
        let mainnet =
            Address::parse(&encoded_sapling(NetworkType::Main), Network::Main).expect("parses");
        let mixed = PaymentRequest {
            payments: vec![
                Payment::new(
                    shielded,
                    Some(Zatoshis::new(1).expect("valid")),
                    Memo::Empty,
                    None,
                    None,
                )
                .expect("valid leg"),
                Payment::new(mainnet, None, Memo::Empty, None, None).expect("valid leg"),
            ],
        };
        assert!(matches!(
            encode_payment_uri(&mixed),
            Err(WalletError::NetworkMismatch)
        ));

        // pub-field struct literals can bypass `Payment::new` — the encoder
        // backstop still refuses reserved framing (defense in depth)
        let bypassed = PaymentRequest {
            payments: vec![Payment {
                recipient: transparent.clone(),
                amount: Some(Zatoshis::new(1).expect("valid")),
                memo: Memo::Reserved(vec![0xF5]),
                label: None,
                message: None,
            }],
        };
        assert!(matches!(
            encode_payment_uri(&bypassed),
            Err(WalletError::ReservedMemoNotSendable)
        ));

        // …and the consensus zero-valued-transparent rule, PRECISELY typed
        // (not collapsed into the generic URI error by the zip321 backstop)
        let bypassed_zero = PaymentRequest {
            payments: vec![Payment {
                recipient: transparent,
                amount: Some(Zatoshis::ZERO),
                memo: Memo::Empty,
                label: None,
                message: None,
            }],
        };
        assert!(matches!(
            encode_payment_uri(&bypassed_zero),
            Err(WalletError::ZeroValuedTransparentOutput)
        ));
    }

    #[test]
    fn empty_text_memo_canonicalizes_to_empty_on_the_wire() {
        // upstream ZIP-302 rule: a zero-length text memo serializes as the
        // canonical "no memo" encoding — Text("") does not survive the wire
        // as a distinct state, BY DESIGN (librustzcash semantics, A4)
        let shielded =
            Address::parse(&encoded_sapling(NetworkType::Test), Network::Test).expect("parses");
        let req = PaymentRequest {
            payments: vec![
                Payment::new(
                    shielded,
                    Some(Zatoshis::new(1).expect("valid")),
                    Memo::text("").expect("empty text is constructible"),
                    None,
                    None,
                )
                .expect("valid leg"),
            ],
        };
        let uri = encode_payment_uri(&req).expect("encodes");
        let parsed = parse_payment_uri(&uri, Network::Test).expect("parses back");
        assert_eq!(parsed.payments[0].memo, Memo::Empty);
    }

    // ── Property tests (§8: round-trip + never-panic on the hostile door) ──

    fn arb_recipient() -> impl Strategy<Value = Address> {
        use zcash_address::ToAddress;
        prop_oneof![
            any::<[u8; 43]>().prop_map(|d| {
                zcash_address::ZcashAddress::from_sapling(NetworkType::Test, d).encode()
            }),
            any::<[u8; 20]>().prop_map(|d| {
                zcash_address::ZcashAddress::from_transparent_p2pkh(NetworkType::Test, d).encode()
            }),
            (any::<[u8; 43]>(), any::<[u8; 43]>()).prop_map(|(o, s)| {
                use zcash_address::unified::{Address as Ua, Encoding, Receiver};
                Ua::try_from_items(vec![Receiver::Orchard(o), Receiver::Sapling(s)])
                    .expect("valid receiver set")
                    .encode(&NetworkType::Test)
            }),
        ]
        .prop_map(|s| Address::parse(&s, Network::Test).expect("self-encoded address parses"))
    }

    /// Memo strategy in WIRE-CANONICAL forms only, so round-trip equality is
    /// exact: nonempty text (Text("") canonicalizes to Empty — own unit
    /// test) and exactly-511-byte arbitrary memos (shorter ones come back
    /// zero-padded — own unit test). Reserved is unconstructible for send.
    fn arb_canonical_memo() -> impl Strategy<Value = Memo> {
        prop_oneof![
            3 => Just(Memo::Empty),
            3 => "[a-zA-Z0-9 ✨🦄]{1,80}"
                .prop_map(|s| Memo::text(s).expect("bounded by the strategy")),
            1 => proptest::collection::vec(any::<u8>(), 511)
                .prop_map(|b| Memo::arbitrary(b).expect("exactly the wire size")),
        ]
    }

    fn arb_leg() -> impl Strategy<Value = Payment> {
        (
            arb_recipient(),
            proptest::option::of(0i64..=MAX_MONEY_ZAT),
            arb_canonical_memo(),
            proptest::option::of("[ -~]{0,60}"),
            proptest::option::of("[ -~]{0,60}"),
        )
            .prop_filter_map(
                "construction-invalid legs (zero→transparent, memo→transparent)",
                |(recipient, amount, memo, label, message)| {
                    let amount = amount.map(|z| Zatoshis::new(z).expect("in range"));
                    Payment::new(recipient, amount, memo, label, message).ok()
                },
            )
    }

    proptest! {
        #[test]
        fn payment_uri_roundtrip_preserves_validated_requests(
            legs in proptest::collection::vec(arb_leg(), 1..4)
        ) {
            let original = PaymentRequest { payments: legs };
            let uri = encode_payment_uri(&original).expect("validated request encodes");
            prop_assert!(uri.len() <= PAYMENT_URI_MAX_BYTES, "encoder stays under our own cap");
            let parsed = parse_payment_uri(&uri, Network::Test).expect("own encoding parses");
            prop_assert_eq!(parsed, original);
        }

        #[test]
        fn parse_payment_uri_never_panics(s in ".{0,600}", inflate in 1usize..32) {
            // typed Ok/Err only — the hostile door has no panic path.
            // `inflate` repeats the input so the domain CROSSES the 8192-byte
            // size cap (review fold: the cap branch must be inside the
            // property's reach, not only the hand-built oversize vector).
            let s = s.repeat(inflate);
            let _ = parse_payment_uri(&s, Network::Test);
            let _ = parse_payment_uri(&s, Network::Main);
        }

        #[test]
        fn parse_payment_uri_never_panics_on_zcash_shaped_input(s in "zcash:[ -~]{0,400}") {
            let _ = parse_payment_uri(&s, Network::Test);
        }
    }
}
