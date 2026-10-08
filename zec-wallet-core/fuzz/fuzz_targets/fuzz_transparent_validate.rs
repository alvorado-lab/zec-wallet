#![no_main]
//! Fuzz the hostile-input **transparent UTXO** validator (wallet-sdk spec §3.3a Recv-2b, §4.6).
//! Every field of a `GetAddressUtxos` reply item — the txid bytes, the signed output index, the
//! raw `script_pubkey`, the signed value, the height — arrives from an UNTRUSTED lightwalletd
//! endpoint, so the validate gate (behind the `__fuzz_validate_utxo` hook) must NEVER panic on
//! any length / any bytes / any number: it returns a typed accept/reject, never a crash (the FFI
//! no-panic contract; principle 7). The `validate_never_panics_on_arbitrary_input` proptest
//! samples this space; this target explores it coverage-guided + persistently.

use libfuzzer_sys::fuzz_target;
use zec_wallet_core::__fuzz_validate_utxo;

fuzz_target!(|data: &[u8]| {
    // Carve the fuzz input into the five wire fields. The first bytes seed the fixed-width
    // numeric fields (so the i32/i64/u64 range guards see the full signed/large space + the
    // exact bounds), and the REMAINDER splits between the txid and the script — so both the
    // 32-byte txid length guard AND the script size-cap / parser see every length, including
    // their exact boundaries.
    let take_i32 = |b: &[u8]| -> i32 {
        let mut x = [0u8; 4];
        let n = b.len().min(4);
        x[..n].copy_from_slice(&b[..n]);
        i32::from_le_bytes(x)
    };
    let take_i64 = |b: &[u8]| -> i64 {
        let mut x = [0u8; 8];
        let n = b.len().min(8);
        x[..n].copy_from_slice(&b[..n]);
        i64::from_le_bytes(x)
    };
    let take_u64 = |b: &[u8]| -> u64 {
        let mut x = [0u8; 8];
        let n = b.len().min(8);
        x[..n].copy_from_slice(&b[..n]);
        u64::from_le_bytes(x)
    };

    let index = take_i32(data.get(0..4).unwrap_or(&[]));
    let value_zat = take_i64(data.get(4..12).unwrap_or(&[]));
    let height = take_u64(data.get(12..20).unwrap_or(&[]));
    let rest = data.get(20..).unwrap_or(&[]);
    // Split the remainder so the txid length varies around the exact-32 boundary.
    let split = rest.len() / 2;
    let txid = rest[..split].to_vec();
    let script = rest[split..].to_vec();

    // The only assertion is reaching here without a crash on ANY input.
    let _ = __fuzz_validate_utxo(txid, index, script, value_zat, height);
});
