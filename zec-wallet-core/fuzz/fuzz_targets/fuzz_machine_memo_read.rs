#![no_main]
//! Fuzz the FR-27 machine-memo READ path (wallet-sdk spec §4.6 / §2.4
//! amendment): the ONE inbound crossing that hands raw on-chain bytes to a
//! host. Every byte here is attacker-chosen — anyone can pay this wallet one
//! zatoshi with a memo containing anything, including a forged copy of the
//! host's registered prefix — so the classifier + scope filter must be TOTAL,
//! and the filter must never widen its own scope.
//!
//! Two doors in one target, because they are the two the host's bytes touch:
//! the ZIP-302 classifier (`Memo::from_wire`, via the fuzz facade) and the
//! scope filter. `TxId::from_display_hex` rides here too — it is the other new
//! hostile parser on this verb, and it is cheap to drive from the same corpus.

use libfuzzer_sys::fuzz_target;
use zec_wallet_core::constants::{MACHINE_MEMO_PREFIX_MAX_BYTES, MEMO_ARBITRARY_MAX_BYTES};
use zec_wallet_core::{TxId, __fuzz_filter_machine_memos};

fuzz_target!(|data: &[u8]| {
    // Split the input: a prefix the "host registered" and the memo's wire
    // bytes. The modulus is the PREFIX BOUND, not the input length — with
    // `% data.len()` any input over 256 bytes gave `split = data[0]`, i.e.
    // 0..=255, so ~87% of runs produced an over-long prefix that the facade
    // rejects before the filter runs at all. The fuzzer was mostly exercising
    // an early return (FR-27 post-fold review).
    if data.is_empty() {
        return;
    }
    let split = (data[0] as usize) % (MACHINE_MEMO_PREFIX_MAX_BYTES + 1);
    let body = &data[1..];
    let (prefix, memo_wire) = body.split_at(split.min(body.len()));

    // Never panics, whatever the wire bytes classify as (Empty / Text /
    // Arbitrary / Reserved / invalid).
    let out = __fuzz_filter_machine_memos(memo_wire, prefix);

    // The scope invariant, on fuzzer-found inputs: nothing comes back that does
    // not LEAD with the registered prefix, and nothing over the ZIP-302 bound.
    // A `contains`-shaped regression, a widened bound, or a `Reserved` leak all
    // fail here rather than shipping.
    for bytes in &out {
        assert!(
            bytes.starts_with(prefix),
            "only prefix-matched bytes may leave the wallet"
        );
        assert!(
            bytes.len() <= MEMO_ARBITRARY_MAX_BYTES,
            "ZIP-302 arbitrary payload bound"
        );
    }
    // NOT asserted here: "an unregistrable prefix yields nothing". The facade
    // returns early on a rejected prefix, so such an assertion grades the
    // HARNESS rather than the filter — it cannot fail for any input. The real
    // guard for that rule is `machine_memo_filter_with_no_prefixes_returns_
    // nothing` in memo.rs, which calls the filter directly (FR-27 post-fold
    // review; the same tautology shape as the sign-escape row in money.rs).

    // The txid hex door: total over any string, and an accepted one round-trips
    // to the exact input (lowercased) — a reversal or a truncation fails here.
    if let Ok(s) = std::str::from_utf8(data) {
        if let Ok(txid) = TxId::from_display_hex(s) {
            assert_eq!(
                txid.to_string(),
                s.to_lowercase(),
                "an accepted txid must render back as itself"
            );
        }
    }
});
