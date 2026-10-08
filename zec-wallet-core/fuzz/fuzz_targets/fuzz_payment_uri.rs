#![no_main]
//! Fuzz the WHOLE ZIP-321 funnel (wallet-sdk spec §4.6 `fuzz_payment_uri`):
//! QR codes and links are attacker-controlled, so `parse_payment_uri` must
//! never panic on any string, under either network. On the well-formed
//! subset, our own encoding must parse back to the same request (the
//! round-trip half of the §8 contract, on fuzzer-found inputs).

use libfuzzer_sys::fuzz_target;
use zec_wallet_core::{encode_payment_uri, parse_payment_uri, Network};

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };
    // never-panic on the hostile door, both networks
    let _ = parse_payment_uri(s, Network::Main);
    if let Ok(req) = parse_payment_uri(s, Network::Test) {
        // anything the funnel ACCEPTS must re-encode and parse back equal —
        // a divergence here is a validation gap, not a style issue
        let uri = encode_payment_uri(&req).expect("accepted request must encode");
        let back = parse_payment_uri(&uri, Network::Test).expect("own encoding must parse");
        assert_eq!(back, req, "funnel round-trip must be exact");
    }
});
