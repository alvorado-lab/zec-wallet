#![no_main]
//! Fuzz the hostile-input **full-transaction** parse boundary (wallet-sdk spec §3.3 tx-enhancement,
//! §4.6). The `GetTransaction` reply for memo recovery is a complete consensus-encoded transaction
//! — the deepest, most branched decoder the SDK feeds untrusted bytes to (sapling + orchard
//! bundles, signatures, the v4/v5 split). The parse gate (behind the `__fuzz_parse_and_validate`
//! hook, which wraps `Transaction::read` in `catch_unwind`) must NEVER panic on any bytes: it
//! returns a clean accept/reject, never a crash that would unwind the sync worker (`run_blocking`
//! re-raises panics — an uncaught parser panic is a remote DoS). The
//! `parse_and_validate_never_panics_on_arbitrary_bytes` proptest samples this space at ≤4 KiB;
//! this target explores it coverage-guided + persistently, with no length cap (a real tx runs to
//! the 8 MiB `GRPC_MAX_MESSAGE_BYTES` decode bound).

use libfuzzer_sys::fuzz_target;
use zec_wallet_core::{__fuzz_header_branch, __fuzz_parse_and_validate};

fuzz_target!(|data: &[u8]| {
    // The hook carves the first 32 bytes as the expected txid and the rest as the tx body, so the
    // fuzzer drives BOTH the parse path and (rarely) the txid-match path. The only assertion is
    // reaching here without a crash on ANY input.
    let _ = __fuzz_parse_and_validate(data);
    // The expiry kernel and the pair-checked header reader it sits on (NU7 wave step 4) read the
    // same hostile bytes: the first four are the stored expiry, the rest the header. Neither may
    // panic, and the kernel must agree with the hook's own reading of the header layout: bytes
    // that are not a whole v5 or v6 header never shorten the stored expiry, and a header whose
    // branch ends at H gives min(stored, H - 1).
    assert!(
        __fuzz_header_branch(data),
        "the expiry kernel disagreed with an independent reading of the header"
    );
});
