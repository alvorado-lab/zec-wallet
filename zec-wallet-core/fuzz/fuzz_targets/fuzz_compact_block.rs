#![no_main]
//! Fuzz the hostile-block SHAPE GUARD (`block_is_scannable`, wallet-sdk spec §4.6,
//! §3.2g). A `CompactBlock` arrives from an UNTRUSTED lightwalletd endpoint and is
//! handed to the consumed-whole `scan_cached_blocks`, which reads several
//! fixed-width fields (`height`/`hash`/`prev_hash`/`txid`/`cmu`) through accessors
//! that PANIC on a wrong length — an FFI crash / remote DoS. `download_range`
//! pre-validates each block; this target asserts the GUARD IS SOUND: for any bytes
//! that decode to a block, if the guard ACCEPTS it, those panicking accessors do
//! NOT panic (the `__fuzz_block_is_scannable` hook runs them). A guard gap surfaces
//! as a libFuzzer crash (the FFI no-panic contract; principle 7).

use libfuzzer_sys::fuzz_target;
use zec_wallet_core::__fuzz_block_is_scannable;

fuzz_target!(|data: &[u8]| {
    // The raw bytes are decoded as a protobuf CompactBlock (any field length); the
    // only assertion is reaching here without a crash for any ACCEPTED block.
    let _ = __fuzz_block_is_scannable(data.to_vec());
});
