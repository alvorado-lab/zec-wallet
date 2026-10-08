//! **STALE-1 (T0-4, §4y) — the vector.** The staleness detector is proven on a
//! REAL mined Ironwood transaction, not on a hand-built header.
//!
//! Contract: `docs/plan/production-readiness-phase-1.md` §4y, row ST-2. The
//! hand-built rows (ST-1, ST-3, ST-4) live beside the function they exercise in
//! `enhance.rs`; this file exists for the one row that needs bytes nobody in
//! this repository wrote, and for the `#[ignore]`d tool that captured them.
//!
//! ## Why a real transaction is the row that matters
//!
//! INC-015 is a test that confirmed its own assumption. `unknown_branch_evidence`
//! matched the v5 header exactly, every row that exercised it hand-built a v5
//! header, and so the suite was green for two months while the detector matched
//! nothing on the live chain — every post-Ironwood transaction is v6. A
//! hand-built v6 header (ST-1) proves the new arm reads what this session
//! believes a v6 header to be; it cannot prove that belief. These bytes can:
//! they were mined, they are what a real endpoint serves for a real txid, and
//! the row asserts the detector's answer on them BOTH ways — unmodified they are
//! not evidence (Ironwood is a branch this build implements), and with the
//! branch field overwritten by an id this build provably lacks they are.
//!
//! ## The fixture carries no wallet fact
//!
//! It is one stranger's mainnet transaction: public chain data every observer of
//! the chain already holds, with no relation to any wallet of ours (the capture
//! takes it from a public block). The row's doc names its height and txid, the
//! same class of datum — a public transaction's identifier, not a wallet's. The
//! crate is `publish = false`, so it never ships.

use std::path::PathBuf;

use zcash_protocol::consensus::BranchId;
use zcash_protocol::constants::{V6_TX_VERSION, V6_VERSION_GROUP_ID};

use crate::enhance::unknown_branch_evidence;

/// The fixture's on-disk format, one file: this magic, then `u64` LE the mined
/// height, then the 32-byte txid in INTERNAL byte order (`TxId::as_ref()`), then
/// `u32` LE the transaction's byte length, then the raw consensus-encoded
/// transaction. Nothing else — the reader asserts the file ends there.
const FIXTURE_MAGIC: &[u8; 7] = b"ZWV6TX1";

/// Where the committed fixture lives (the capture tool writes it, the row reads
/// it).
fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mainnet-v6-transaction.bin")
}

/// A transaction larger than this is not a good fixture for a header test: the
/// row reads twelve bytes, and the repository pays for the rest.
const FIXTURE_MAX_BYTES: usize = 64 * 1024;

/// The mined height of the committed transaction, pinned here so the row's doc
/// and the fixture's bytes cannot drift — the row asserts the file's header
/// against this.
const FIXTURE_HEIGHT: u64 = 3_478_152;

/// The txid of the committed transaction, in DISPLAY order (the order a block
/// explorer shows), pinned for the same reason. Public chain data.
const FIXTURE_TXID_DISPLAY: &str =
    "295b67c63d99ec65a54803b0498510ae0648cd9ebb12cf1766d5f273059aa91f";

/// An unknown branch id, ASSERTED unknown by the row that uses it — the same
/// discipline as `enhance`'s fixture, and for the same reason: the id that used
/// to be "unknown" here (`Nu6_3`) became known at a crate bump.
const UNKNOWN_BRANCH: u32 = 0x0bad_0bad;

struct Captured {
    height: u64,
    txid: [u8; 32],
    tx: Vec<u8>,
}

fn decode_fixture(bytes: &[u8]) -> Captured {
    assert!(
        bytes.starts_with(FIXTURE_MAGIC),
        "the fixture must carry its magic; got {:?}",
        &bytes[..bytes.len().min(FIXTURE_MAGIC.len())]
    );
    let mut at = FIXTURE_MAGIC.len();
    let height = u64::from_le_bytes(
        bytes[at..at + 8]
            .try_into()
            .expect("the fixture carries a height"),
    );
    at += 8;
    let txid: [u8; 32] = bytes[at..at + 32]
        .try_into()
        .expect("the fixture carries a txid");
    at += 32;
    let len = u32::from_le_bytes(
        bytes[at..at + 4]
            .try_into()
            .expect("the fixture carries a length"),
    ) as usize;
    at += 4;
    assert_eq!(
        bytes.len(),
        at + len,
        "the fixture ends after the transaction it declares ({len} bytes)"
    );
    Captured {
        height,
        txid,
        tx: bytes[at..].to_vec(),
    }
}

/// A txid as a block explorer shows it: the reverse of the internal order the
/// wire and `TxId::as_ref()` use.
fn display_txid(internal: &[u8; 32]) -> String {
    let mut b = internal.to_vec();
    b.reverse();
    hex::encode(b)
}

/// **§4y ST-2 — the VECTOR, and the DEFECT row this item exists for.** A real
/// mined Ironwood transaction is recognised as v6, and is not evidence.
///
/// The fixture is one post-activation **mainnet** transaction, captured whole by
/// the `#[ignore]`d tool below from a live lightwalletd's `GetTransaction`.
/// Height 3,478,152, 9,166 bytes; txid (display order)
/// `295b67c63d99ec65a54803b0498510ae0648cd9ebb12cf1766d5f273059aa91f`. Public
/// chain data belonging to a stranger; no wallet of ours is testnet-adjacent to
/// it.
///
/// Three clauses:
/// 1. **the fixture's own soundness**, checked before the detector is blamed for
///    anything — the bytes carry the v6 header pair
///    (`fOverwintered | V6_TX_VERSION`, `V6_VERSION_GROUP_ID`), name
///    `BranchId::Nu6_3`, sit above MAINNET's Ironwood activation read from
///    `zcash_protocol` (which is also what rules out a testnet recapture:
///    testnet activates Nu6_3 far higher, so no testnet transaction can carry
///    that branch at this height), and — the binding that makes them REAL —
///    PARSE as a v6 transaction whose own txid equals the one the fixture
///    declares. Without that last clause a hand-made file matching five
///    constants plus arbitrary padding would pass every other assertion, while
///    this module's whole premise is "bytes nobody in this repository wrote".
///    (The parse and the activation check were added by the crypto pass,
///    which found this list claiming a height check that did not exist.);
/// 2. **the control**: unmodified, the detector says `None` — Nu6_3 is a branch
///    this build implements, so a real Ironwood transaction is not evidence of
///    staleness. A detector that fired here would brand every healthy endpoint;
/// 3. **the defect clause**: the SAME bytes with bytes 8..12 overwritten by an
///    id this build provably lacks give `Some(that id)` — the v6 arm exercised
///    by real bytes rather than by this session's idea of a v6 header. **At the
///    base (v5-only matching) this clause is `None`**, which is INC-015: the
///    detector saw nothing on the live chain from 2026-07-28 onward.
#[test]
fn a_real_mined_ironwood_transaction_is_recognised_as_v6_and_is_not_evidence() {
    let path = fixture_path();
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "the committed v6 fixture is missing at {}: {e}. Re-capture it with \
             `cargo test -p zec-wallet-core --lib 'staleness_proof::capture' --ignored --nocapture`",
            path.display()
        )
    });
    let captured = decode_fixture(&bytes);

    // 1. the fixture's own soundness.
    assert_eq!(
        captured.height, FIXTURE_HEIGHT,
        "the fixture's declared height must match the one this row's doc names"
    );
    assert_eq!(
        display_txid(&captured.txid),
        FIXTURE_TXID_DISPLAY,
        "the fixture's txid must match the one this row's doc names"
    );
    assert!(
        captured.tx.len() >= 12 && captured.tx.len() <= FIXTURE_MAX_BYTES,
        "the fixture is a whole transaction under the size cap; it is {} bytes",
        captured.tx.len()
    );
    let header = u32::from_le_bytes(captured.tx[0..4].try_into().expect("four bytes"));
    let group = u32::from_le_bytes(captured.tx[4..8].try_into().expect("four bytes"));
    let branch = u32::from_le_bytes(captured.tx[8..12].try_into().expect("four bytes"));
    assert_eq!(
        (header, group),
        (0x8000_0000 | V6_TX_VERSION, V6_VERSION_GROUP_ID),
        "the fixture must be a v6 transaction — that is the whole point of it. A v5 fixture \
         here would pass clause 3 through the OLD arm and prove nothing about v6"
    );
    assert_eq!(
        BranchId::try_from(branch),
        Ok(BranchId::Nu6_3),
        "the fixture must name the Ironwood branch (raw {branch:#010x}); a transaction from \
         another branch is not the post-activation mainnet shape this row claims to hold"
    );
    // The height is above MAINNET's Ironwood activation — which is also what
    // rules out a testnet recapture, since testnet's Nu6_3 activates far higher
    // (`zcash_protocol-0.10.5 consensus.rs`), so no testnet transaction can
    // carry Nu6_3 at a height in this range. This row's doc claimed the check;
    // the crypto pass found it did not exist.
    {
        use zcash_protocol::consensus::{NetworkUpgrade, Parameters};
        let activation = zcash_protocol::consensus::Network::MainNetwork
            .activation_height(NetworkUpgrade::Nu6_3)
            .expect("mainnet activates Nu6_3");
        assert!(
            FIXTURE_HEIGHT > u64::from(u32::from(activation)),
            "the fixture must sit above MAINNET's Ironwood activation ({activation:?})"
        );
    }
    // THE BINDING: parse the bytes and check the txid they COMMIT to is the one
    // the fixture declares. Without this a hand-made file matching five
    // constants plus arbitrary padding passes every assertion above — while
    // this module's whole premise is "bytes nobody in this repository wrote".
    // Found by the crypto pass.
    let parsed =
        zcash_primitives::transaction::Transaction::read(&captured.tx[..], BranchId::Nu6_3)
            .expect("the fixture is a well-formed v6 transaction");
    assert_eq!(
        parsed.txid().as_ref(),
        &captured.txid,
        "the parsed transaction must COMMIT to the declared txid — this is what makes the \
         fixture real bytes rather than a shape that happens to match five constants"
    );

    // 2. the control: a real Ironwood transaction is NOT staleness evidence.
    assert_eq!(
        unknown_branch_evidence(&captured.tx),
        None,
        "a mined transaction naming a branch this build IMPLEMENTS is not evidence of \
         anything — a detector that fired here would brand every healthy endpoint stale"
    );

    // 3. the defect clause: the same real bytes, one branch id this build lacks.
    assert!(
        BranchId::try_from(UNKNOWN_BRANCH).is_err(),
        "the overwritten id must be one this build genuinely lacks"
    );
    let mut stale = captured.tx.clone();
    stale[8..12].copy_from_slice(&UNKNOWN_BRANCH.to_le_bytes());
    assert_eq!(
        unknown_branch_evidence(&stale),
        Some(UNKNOWN_BRANCH),
        "a REAL v6 transaction naming a branch this build lacks is local, unforgeable proof \
         that the chain has moved past our params. At the base this is None — the detector \
         matched the v5 header only, and every post-Ironwood transaction is v6 (INC-015)"
    );
}

/// **The capture tool (§4y ST-2).** `#[ignore]`d: it talks to a live mainnet
/// lightwalletd. Run it once, on purpose, when the fixture must be re-made:
///
/// ```text
/// cd sdk && cargo test -p zec-wallet-core --lib \
///   'staleness_proof::capture' --ignored --nocapture
/// ```
///
/// `ZEC_WALLET_LWD` overrides the endpoint (default `https://zec.rocks:443`);
/// `ZEC_WALLET_LWD_KEY_HEADER` + `ZEC_WALLET_LWD_KEY` add an auth header if the
/// endpoint wants one (both or neither).
///
/// It walks DOWN from a point well below the tip — past Zcash's pruning depth,
/// so the block is final — reading compact blocks until it finds one with a
/// shielded transaction, fetches that transaction WHOLE, and refuses to write
/// anything that is not a v6 transaction naming `Nu6_3` under the size cap. The
/// refusals are the point: a capture tool that writes whatever it got is how a
/// fixture ends up proving something other than what its row claims.
#[tokio::test]
#[ignore = "live network: captures the ST-2 mainnet v6 fixture; run once, on purpose"]
async fn capture_the_mainnet_v6_transaction() {
    use std::sync::Arc;

    use zcash_protocol::TxId;

    use crate::config::{EndpointAuth, LightServerEndpoint, TorPolicy};
    use crate::net::grpc::LightwalletdClient;
    // `block_range` / `latest_block_height` are the `ScanClient` port's, which the
    // real client implements — the same door the shipped pass uses.
    use crate::sync::ScanClient;

    /// Far past the 100-block pruning depth, so the block cannot be reorged out.
    const SEARCH_BELOW_TIP: u64 = 1_000;
    const SEARCH_ATTEMPTS: u64 = 200;

    let url = std::env::var("ZEC_WALLET_LWD").unwrap_or_else(|_| "https://zec.rocks:443".into());
    let auth = match (
        std::env::var("ZEC_WALLET_LWD_KEY_HEADER"),
        std::env::var("ZEC_WALLET_LWD_KEY"),
    ) {
        (Ok(h), Ok(v)) => Some(EndpointAuth::new(h, v).expect("valid auth pair")),
        (Err(_), Err(_)) => None,
        _ => panic!("ZEC_WALLET_LWD_KEY_HEADER and ZEC_WALLET_LWD_KEY: both or neither"),
    };
    let endpoint = LightServerEndpoint::new(url.clone()).expect("valid endpoint");
    let fell_back = Arc::new(crate::net::tor_posture::TorPosture::new());
    let mut client = LightwalletdClient::connect(
        &endpoint,
        &TorPolicy::Off,
        None,
        &fell_back,
        &Default::default(),
        auth.as_ref(),
    )
    .expect("connect builds lazily");
    let tip = client
        .latest_block_height()
        .await
        .expect("latest_block_height");
    println!(
        "capture: {url} tip={tip}, searching down from {}",
        tip - SEARCH_BELOW_TIP
    );

    let mut chosen: Option<(u64, TxId, Vec<u8>)> = None;
    for attempt in 0..SEARCH_ATTEMPTS {
        let h = tip - SEARCH_BELOW_TIP - attempt;
        let mut stream = client.block_range(h, h).await.expect("block_range");
        let Some(block) = stream.next_block().await.expect("a streamed block") else {
            continue;
        };
        let Some(vtx) = block.vtx.first() else {
            continue;
        };
        let raw: [u8; 32] = match vtx.txid.as_slice().try_into() {
            Ok(b) => b,
            Err(_) => continue,
        };
        let txid = TxId::from_bytes(raw);
        let Some(fetched) = client.get_transaction(txid).await.expect("get_transaction") else {
            continue;
        };
        if fetched.data.len() < 12 || fetched.data.len() > FIXTURE_MAX_BYTES {
            println!("  {h}: {} bytes, outside the cap", fetched.data.len());
            continue;
        }
        let header = u32::from_le_bytes(fetched.data[0..4].try_into().expect("four bytes"));
        let group = u32::from_le_bytes(fetched.data[4..8].try_into().expect("four bytes"));
        let branch = u32::from_le_bytes(fetched.data[8..12].try_into().expect("four bytes"));
        println!(
            "  {h}: txid={} bytes={} header={header:#010x} group={group:#010x} branch={branch:#010x}",
            display_txid(&raw),
            fetched.data.len()
        );
        if (header, group) != (0x8000_0000 | V6_TX_VERSION, V6_VERSION_GROUP_ID) {
            continue;
        }
        if BranchId::try_from(branch) != Ok(BranchId::Nu6_3) {
            continue;
        }
        chosen = Some((h, txid, fetched.data));
        break;
    }
    let (height, txid, tx) =
        chosen.expect("a v6 mainnet transaction naming Nu6_3 under the size cap");

    // THE NETWORK REFUSAL, and it exists because this tool ate its own fixture
    // TWICE in one session (both measured). Pointed at testnet via
    // `ZEC_WALLET_LWD` it found a real v6 transaction — testnet IS past Nu6.3,
    // tip 4,338,946 against an activation of 4,134,000 — and wrote it over the
    // committed MAINNET fixture, under a path named `mainnet-…`.
    //
    // The first guard written for this checked the HEIGHT against mainnet's
    // activation, and it was useless in the direction that matters: testnet
    // heights run ABOVE mainnet's, so 4,338,946 > 3,428,143 passed and the
    // clobber happened again. A height cannot name a network. The endpoint's own
    // `chain_name` can, and it is the same field `provision` already guards on.
    //
    // This function's doc says "a capture tool that writes whatever it got is how
    // a fixture ends up proving something other than what its row claims" — the
    // height and branch refusals above implement that, and the NETWORK axis did
    // not.
    let info = client
        .get_lightd_info()
        .await
        .expect("get_lightd_info names the chain");
    assert_eq!(
        info.chain_name, "main",
        "REFUSING TO WRITE: this endpoint serves {:?}, not mainnet — writing would put a \
         foreign-network transaction under a path the row reads as mainnet, and the row's \
         pinned height would then fail for a reason that has nothing to do with the detector. \
         Point ZEC_WALLET_LWD at a mainnet lightwalletd",
        info.chain_name
    );

    let mut out = Vec::new();
    out.extend_from_slice(FIXTURE_MAGIC);
    out.extend_from_slice(&height.to_le_bytes());
    out.extend_from_slice(txid.as_ref());
    out.extend_from_slice(&(tx.len() as u32).to_le_bytes());
    out.extend_from_slice(&tx);
    let path = fixture_path();
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("fixture dir");
    std::fs::write(&path, &out).expect("write the fixture");
    println!(
        "capture: wrote {} ({} bytes)\n  FIXTURE_HEIGHT = {height}\n  FIXTURE_TXID_DISPLAY = \"{}\"",
        path.display(),
        out.len(),
        display_txid(txid.as_ref())
    );
}
