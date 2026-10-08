//! **T0-2 (§4z) — the arrival readback.** The device proof's independent
//! discriminator: the transaction the wallet says it detected is read back from
//! a mainnet lightwalletd and parsed HERE, so the claim "a real outside payment
//! arrived in the Ironwood bundle" does not rest on the wallet that detected it.
//!
//! Contract: `docs/plan/production-readiness-phase-1.md` §4z, and the T0-2
//! parameters in the T0-2 handoff note, §3 item 3 — *"verify
//! independently that the arriving transaction really carries `ironwood_actions`
//! — read it back from lightwalletd rather than trusting either end — because a
//! sender and a scanner that are both ours could in principle be
//! self-consistently wrong and still look green."*
//!
//! ## What it can and cannot prove
//!
//! It proves: the txid the wallet's history shows names a transaction the CHAIN
//! holds, that transaction parses as v6/NU6.3, it carries a non-empty Ironwood
//! bundle, and the parsed bytes COMMIT to that txid (so a lying endpoint cannot
//! hand back a different transaction and have it pass).
//!
//! It does not prove which action is ours — the note is the wallet's to detect,
//! and the bundle is encrypted to the recipient. The proof is a conjunction: the
//! wallet's `wallet.incoming_detect outcome="detected"` says a note arrived for
//! this wallet, and this row says the transaction it arrived in is a real mined
//! Ironwood one.
//!
//! ## No wallet fact is recorded in THIS FILE — and that is a narrower claim
//! ## than it looks
//!
//! The txid comes in through the environment, never a committed constant: it is
//! OUR wallet's receipt, unlike `staleness_proof`'s stranger-transaction
//! fixture, so it is not in this source file (§5.4).
//!
//! **But delegating it to a run record does not contain it.** A run record
//! kept in a repository is IN THE TREE, so pasting the tool's output there
//! commits the txid, the mined height and the amount — two of which §5.4's
//! never-log list names — next to the sentence saying who sent it and to which
//! device. Anyone with repo access resolves that on a block explorer and links a
//! real payment to a real wallet, a named device and a project, which is the
//! deanonymisation this product exists to prevent. Once committed, removing
//! the line does not remove it: it stays in the history.
//!
//! **So a tracked file carries only the SHAPE of the proof** — the verdict, the
//! field names, the counts, the outcome codes. The identifying values (txid,
//! amount, heights, receive address, device serial, who sent it) belong in a
//! file kept outside version control, which the run record may cite by name.
//! If you find yourself pasting a txid into a tracked document, that is the
//! mistake this paragraph exists to stop.
//!
//! The rule applies to every run of this tool, and to any record that quotes
//! its output: a screenshot, a log excerpt, a bug report. Keep the shape and
//! drop the values before it goes anywhere another person can read it.
//!
//! (Environment-only input is the first half of the rule; this is the second.)
//!
//!

/// **The readback tool (§4z).** `#[ignore]`d: it talks to a live mainnet
/// lightwalletd and needs a txid from a device run.
///
/// ```text
/// cd sdk && ZEC_WALLET_ARRIVAL_TXID=<display-order hex from the app's tx detail sheet> \
///   cargo test -p zec-wallet-core --lib \
///   'arrival_proof::the_arrival' -- --ignored --nocapture
/// ```
///
/// `ZEC_WALLET_LWD` overrides the endpoint (default `https://zec.rocks:443`);
/// the endpoint is REFUSED unless its own `chain_name` is `main` — the
/// lesson from `staleness_proof`'s capture tool, where a height was trusted to
/// name a network and could not (testnet heights run above mainnet's).
#[tokio::test]
#[ignore = "live network: reads back a T0-2 device-proof arrival; needs ZEC_WALLET_ARRIVAL_TXID"]
async fn the_arrival_transaction_carries_a_real_ironwood_bundle() {
    use std::sync::Arc;

    use zcash_primitives::transaction::Transaction;
    use zcash_protocol::TxId;
    use zcash_protocol::consensus::BranchId;
    use zcash_protocol::constants::{V6_TX_VERSION, V6_VERSION_GROUP_ID};

    use crate::config::{EndpointAuth, LightServerEndpoint, TorPolicy};
    use crate::net::grpc::LightwalletdClient;

    let display = std::env::var("ZEC_WALLET_ARRIVAL_TXID").expect(
        "ZEC_WALLET_ARRIVAL_TXID: the txid the app's tx detail sheet shows (display order)",
    );
    let display = display.trim().to_ascii_lowercase();
    // ASCII-hex FIRST, then the length: the loop below slices by BYTE index, so a
    // 64-byte non-ASCII string would panic on a char boundary rather than on the
    // hex parse — an unhelpful failure for a mistyped argument.
    assert!(
        display.len() == 64 && display.bytes().all(|b| b.is_ascii_hexdigit()),
        "ZEC_WALLET_ARRIVAL_TXID must be 64 hex characters; got {} characters: {display:?}",
        display.len()
    );
    let mut internal = [0u8; 32];
    for (i, byte) in internal.iter_mut().enumerate() {
        let hi = 62 - i * 2;
        *byte = u8::from_str_radix(&display[hi..hi + 2], 16).expect("hex txid");
    }
    // Display order is the byte-REVERSAL of internal order, so read the display
    // string back to front — the same convention `staleness_proof::display_txid`
    // writes with.
    let txid = TxId::from_bytes(internal);

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

    // THE NETWORK REFUSAL, first: a testnet endpoint would answer a 64-hex txid
    // with `None` and the row would read as "the chain does not hold it" — a
    // false negative about the wrong chain. Name the network before asking it
    // anything.
    let info = client
        .get_lightd_info()
        .await
        .expect("get_lightd_info names the chain");
    assert_eq!(
        info.chain_name, "main",
        "REFUSING TO READ: {url} serves {:?}, not mainnet. The T0-2 arrival is a MAINNET \
         payment; a testnet endpoint would answer 'no such transaction' and that absence \
         would say nothing about the arrival. Point ZEC_WALLET_LWD at a mainnet lightwalletd",
        info.chain_name
    );

    let fetched = client
        .get_transaction(txid)
        .await
        .expect("get_transaction")
        .unwrap_or_else(|| {
            panic!(
                "{url} holds no transaction {display} — wrong txid, or the \
             endpoint is behind the block that mined it"
            )
        });
    println!(
        "readback: {url} chain={} txid={display} height={} bytes={}",
        info.chain_name,
        fetched.height,
        fetched.data.len()
    );

    // MINED, asserted rather than printed. lightwalletd answers an unconfirmed
    // transaction with `height = 0`, so without this the module doc's "a real
    // MINED Ironwood one" rested on a field that was displayed twice and checked
    // never — the arrival could have been read straight out of the mempool.
    assert!(
        fetched.height > 0,
        "{url} returned {display} with height 0 — that is lightwalletd's answer for an \
         UNCONFIRMED transaction. The proof is about a mined arrival; wait for a confirmation"
    );
    assert!(
        fetched.data.len() >= 12,
        "a transaction shorter than its own 12-byte header fragment is not a transaction; \
         {url} returned {} bytes for {display}",
        fetched.data.len()
    );
    let header = u32::from_le_bytes(fetched.data[0..4].try_into().expect("four bytes"));
    let group = u32::from_le_bytes(fetched.data[4..8].try_into().expect("four bytes"));
    let branch = u32::from_le_bytes(fetched.data[8..12].try_into().expect("four bytes"));
    println!("  header={header:#010x} group={group:#010x} branch={branch:#010x}");
    // ASSERTED, not merely printed, and the sibling tool is the precedent
    // (`staleness_proof`'s capture checks the same triple before it will write a
    // fixture). `Transaction::read`'s `BranchId` argument does NOT pin this: it is
    // passed to `read_v4` only, and V5/V6 discard it and take the branch from the
    // transaction's own header fragment. So "parsed as v6/NU6.3" is a claim the
    // parse call cannot make and these three lines have to.
    assert_eq!(
        (header, group),
        (0x8000_0000 | V6_TX_VERSION, V6_VERSION_GROUP_ID),
        "not a v6 transaction: header {header:#010x} / group {group:#010x}"
    );
    assert_eq!(
        BranchId::try_from(branch),
        Ok(BranchId::Nu6_3),
        "the transaction does not name NU6.3 as its consensus branch: {branch:#010x}"
    );

    let parsed = Transaction::read(&fetched.data[..], BranchId::Nu6_3)
        .expect("the endpoint's bytes parse as a transaction");
    // THE BINDING (staleness_proof's lesson): an endpoint that hands back a
    // DIFFERENT transaction must not pass. The parsed bytes have to commit to
    // the txid we asked for.
    assert_eq!(
        parsed.txid(),
        txid,
        "the transaction {url} returned does not commit to the txid it was asked for — a \
         lying or confused endpoint, not a proof"
    );

    let ironwood = parsed.ironwood_bundle().expect(
        "a post-NU6.3 shielded payment rides the IRONWOOD bundle; this transaction \
             carries none, so the arrival proves the detect gate but NOT the Ironwood path",
    );
    let actions = ironwood.actions().len();
    println!(
        "  VERDICT: v6 Ironwood transaction, mined at {}, ironwood_actions={actions}, \
         sapling={}, transparent_out={}",
        fetched.height,
        parsed
            .sapling_bundle()
            .map_or(0, |b| b.shielded_outputs().len()),
        parsed.transparent_bundle().map_or(0, |b| b.vout.len())
    );
    assert!(
        actions > 0,
        "an Ironwood bundle with no actions cannot be the payment"
    );
}
