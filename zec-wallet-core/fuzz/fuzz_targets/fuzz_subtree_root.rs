#![no_main]
//! Fuzz the hostile-input note-commitment **subtree-root** leaf-hash parsers
//! (wallet-sdk spec §4.6, §3.2g `fuzz_subtree_root`). A `SubtreeRoot`'s `root_hash`
//! bytes and `completing_block_height` arrive from an UNTRUSTED lightwalletd
//! endpoint, so the decode (`parse_{sapling,orchard}_root` behind the
//! `__fuzz_parse_subtree_root` hook) must NEVER panic on any length / any bytes /
//! any height — it returns a typed reject or a decoded root, never a crash
//! (the FFI no-panic contract; principle 7).

use libfuzzer_sys::fuzz_target;
use zec_wallet_core::__fuzz_parse_subtree_root;

fuzz_target!(|data: &[u8]| {
    // Split the fuzz input: the first 8 bytes seed the completing height (so the
    // u32-range guard is exercised across the full u64 space), the remainder is the
    // root_hash (so the 32-byte length guard + the canonical-field decode see every
    // length, including the exact-32 boundary).
    let (height, root_hash) = if data.len() >= 8 {
        let mut h = [0u8; 8];
        h.copy_from_slice(&data[..8]);
        (u64::from_le_bytes(h), data[8..].to_vec())
    } else {
        (0u64, data.to_vec())
    };
    // never panics on any input — the only assertion is reaching here without a crash
    let _ = __fuzz_parse_subtree_root(root_hash, height);
});
