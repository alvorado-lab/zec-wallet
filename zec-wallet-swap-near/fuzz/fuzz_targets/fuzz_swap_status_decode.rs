#![no_main]
//! Fuzz the WHOLE 1Click response funnel (wallet-sdk spec §8
//! `fuzz_swap_status_decode`): provider bytes are hostile (§4.6), so
//! decode → bounds → port mapping must never panic for any of the three
//! response families (status, quote, tokens). The byte caps live in the
//! HTTP layer; the funnel itself must hold without them.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    zec_wallet_swap_near::fuzzing::decode_and_map_all(data);
});
