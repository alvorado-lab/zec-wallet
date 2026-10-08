//! Stage S8 `obligation` — a signed send the wallet could not broadcast is the
//! wallet's to deliver — the TEST AUTHOR's wallet-level rows
//! (`docs/plan/stage-8-payment-identity-and-durable-retry.md` §3.2, written
//! blind against the contract, IT-2a). A child of `wallet::tests` so it reaches
//! the fixtures there (`cfg`, `test_vault`, `raw_seed`, `FakeUtxoSource`,
//! `script_paying`, `decode_taddr`, `NOOP_PROGRESS`) and the
//! handle's private seams (`sign_proposal_upstream`, `broadcast_persisted`,
//! `resubmit_queued_sends`, `inner`); its own file so no cited line in
//! `wallet.rs` moves.
//!
//! THE TRANSACTION IS REAL. Every row signs a SHIELD through the product path —
//! `propose_shield` → `sign_proposal_upstream` (the engine's persist) →
//! `broadcast_persisted` — because that is the single-step arm the item is
//! about: `RetainedProposal::Shield` calls bare `create_signed_core` and keeps
//! NO intent row, so what the wallet holds afterwards is exactly a persisted
//! `transactions` row with `created` set and nothing in the outbox. A wallet
//! synced over `FakeChain` holds one transparent UTXO the fixture put there,
//! which is what a shield spends; the notes the engine marks spent are its own.
//!
//! THE ENDPOINT IS REAL TOO. The bytes are watched on a loopback lightwalletd
//! that speaks the little HTTP/2 a tonic client needs (`net/grpc.rs`'s
//! `AnsweringPeer` shape; that one is private to its module and its lines are
//! cited, so this file carries its own copy) and RECORDS every request body.
//! It answers one of two ways: `grpc-status: 0` with an empty `SendResponse`
//! (the endpoint ACCEPTED — `TxSubmitResult::Success`), or a hang-up once the
//! request is in (the endpoint SAW the bytes and gave no verdict — the
//! `GrpcFailure` the contract's "failing dialer" produces, with the bytes on
//! record). So "the dialer saw the bytes" is asserted on the bytes.
//!
//! THE KILL is `close()` and no further call on the handle: every write on the
//! path is committed before `send` returns, and `close()` writes nothing to the
//! money tables, so a reopened wallet holds what a killed one would. Where a
//! contract row names a point BETWEEN two calls, this file calls the first and
//! not the second — the seam is the handle's own private method boundary.
//!
//! Rows that name a SYMBOL the contract leaves to the implementer (the
//! per-transaction delivery state, the enumeration's predicate on the
//! create-committed-but-unrecorded window) are the sibling file's, in their
//! own commit, so this one compiled and ran red at the base commit. Since the
//! adjudication's repair (ruling.md §6) the accepted-send row reads the
//! implementer's `DeliveryState` too — the join is behind it.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::*;
use crate::config::TorRuntime;
use crate::constants::{BROADCAST_ISOLATION_KEY_PREFIX, WALLET_SYNC_ISOLATION_KEY};
use crate::intent_store::{NoteClaim, PROTO_SAPLING};
use crate::net::host_dialer::testing::ScriptedHostDialer;
use crate::net::host_dialer::{HostDialCode, HostDialer};
use crate::state::{ArmCounts, DeliveryState, TxStatus};
use crate::tracing_guard::{CaptureLayer, CapturedEvents, force_wallet_callsites_enabled};
use zcash_client_backend::data_api::{TransactionStatus, WalletWrite};

/// The anchor account 0 is imported at (its birthday is the block after), and
/// the tip the fixture syncs to over `FakeChain` — three batches above the
/// anchor, so a rewind has checkpoints to land on. NOT the parent module's
/// 281,000: that is a Sapling-era testnet height, where a v4 transaction
/// cannot carry the shielded self-output a shield proposes and the sign fails
/// closed. 3,000,000 is post-NU6 and pre-Ironwood on testnet (the same tip
/// the parent module's own NU6-era row syncs to). A shield signed here
/// carries `expiry_height = tip + 1 + DEFAULT_TX_EXPIRY_DELTA`, so it is
/// unexpired against this tip and expired against nothing the fixture mines.
const ANCHOR: u64 = 2_999_700;
pub(super) const SYNCED_TIP: u64 = 3_000_000;

/// The funding UTXO — above `SHIELDING_THRESHOLD_ZAT`, so `propose_shield`
/// returns a proposal.
const FUNDING_ZAT: i64 = 500_000;

// ── The loopback lightwalletd ────────────────────────────────────────────────

/// What the loopback lightwalletd does with each complete request.
#[derive(Clone, Copy)]
pub(super) enum Answer {
    /// `grpc-status: 0` and an empty message — `SendResponse { error_code: 0 }`:
    /// the endpoint ACCEPTED.
    Accept,
    /// Hang up once the request is in: the endpoint saw the bytes and gave no
    /// verdict — the transport miss `broadcast_one` maps to `GrpcFailure`.
    HangUp,
}

/// `:status: 200` (static index 8) and `content-type: application/grpc`
/// (literal, never indexed, name = static index 31).
const H2_RESPONSE_HEADERS: &[u8] = b"\x88\x0f\x10\x10application/grpc";
/// `grpc-status: 0` (literal, never indexed, new name).
const H2_OK_TRAILERS: &[u8] = b"\x00\x0bgrpc-status\x010";
/// One gRPC message: uncompressed, zero bytes long — every field default.
const EMPTY_GRPC_MESSAGE: [u8; 5] = [0; 5];

fn h2_frame(kind: u8, flags: u8, stream: u32, payload: &[u8]) -> Vec<u8> {
    let len = payload.len() as u32;
    let mut frame = vec![(len >> 16) as u8, (len >> 8) as u8, len as u8, kind, flags];
    frame.extend_from_slice(&stream.to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

/// A loopback lightwalletd over real TCP: an `http` endpoint an `Off` wallet
/// dials with its own direct dialer (the development configuration the door
/// admits), answering as scripted and keeping every request body it completed.
pub(super) struct LoopbackLightwalletd {
    answer: Mutex<Answer>,
    /// Every completed request's body, in arrival order — the DATA payloads of
    /// one stream, concatenated: the gRPC framing and the `RawTransaction`
    /// protobuf around the signed bytes, which sit in it whole.
    requests: Mutex<Vec<Vec<u8>>>,
}

impl LoopbackLightwalletd {
    pub(super) async fn serve(answer: Answer) -> (Arc<Self>, LightServerEndpoint) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback");
        let port = listener.local_addr().expect("addr").port();
        let peer = Arc::new(Self {
            answer: Mutex::new(answer),
            requests: Mutex::new(Vec::new()),
        });
        let serving = Arc::clone(&peer);
        tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                let peer = Arc::clone(&serving);
                tokio::spawn(async move { peer.speak_h2(socket).await });
            }
        });
        let endpoint = LightServerEndpoint::new(format!("http://127.0.0.1:{port}"))
            .expect("loopback endpoint");
        (peer, endpoint)
    }

    pub(super) fn set_answer(&self, answer: Answer) {
        *self.answer.lock().expect("answer poisoned") = answer;
    }

    pub(super) fn requests(&self) -> Vec<Vec<u8>> {
        self.requests.lock().expect("requests poisoned").clone()
    }

    /// How many completed requests carried `raw` whole.
    pub(super) fn times_seen(&self, raw: &[u8]) -> usize {
        self.requests()
            .iter()
            .filter(|body| contains(body, raw))
            .count()
    }

    /// The request index at which `raw` was FIRST seen at or after `from`.
    fn first_seen_from(&self, raw: &[u8], from: usize) -> Option<usize> {
        self.requests()
            .iter()
            .enumerate()
            .skip(from)
            .find(|(_, body)| contains(body, raw))
            .map(|(i, _)| i)
    }

    async fn speak_h2(&self, socket: tokio::net::TcpStream) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let (mut from, mut to) = socket.into_split();
        let mut preface = [0_u8; 24];
        if from.read_exact(&mut preface).await.is_err() {
            return;
        }
        if to.write_all(&h2_frame(0x4, 0, 0, &[])).await.is_err() {
            return;
        }
        let mut bodies: HashMap<u32, Vec<u8>> = HashMap::new();
        loop {
            let mut head = [0_u8; 9];
            if from.read_exact(&mut head).await.is_err() {
                return;
            }
            let len = u32::from_be_bytes([0, head[0], head[1], head[2]]) as usize;
            let (kind, flags) = (head[3], head[4]);
            let stream = u32::from_be_bytes([head[5], head[6], head[7], head[8]]) & 0x7fff_ffff;
            let mut payload = vec![0_u8; len];
            if from.read_exact(&mut payload).await.is_err() {
                return;
            }
            match kind {
                // SETTINGS and PING, unless they are themselves acknowledgements.
                0x4 if flags & 0x1 == 0 => {
                    let _ = to.write_all(&h2_frame(0x4, 0x1, 0, &[])).await;
                }
                0x6 if flags & 0x1 == 0 => {
                    let _ = to.write_all(&h2_frame(0x6, 0x1, 0, &payload)).await;
                }
                0x0 => bodies
                    .entry(stream)
                    .or_default()
                    .extend_from_slice(&payload),
                _ => {}
            }
            // DATA or HEADERS carrying END_STREAM: the request is complete.
            if matches!(kind, 0x0 | 0x1) && flags & 0x1 != 0 {
                let body = bodies.remove(&stream).unwrap_or_default();
                self.requests.lock().expect("requests poisoned").push(body);
                // Copied out: the guard must not live across the writes below.
                let answer = *self.answer.lock().expect("answer poisoned");
                match answer {
                    // The socket drops here: the endpoint gave no verdict.
                    Answer::HangUp => return,
                    Answer::Accept => {
                        for frame in [
                            h2_frame(0x1, 0x4, stream, H2_RESPONSE_HEADERS),
                            h2_frame(0x0, 0, stream, &EMPTY_GRPC_MESSAGE),
                            h2_frame(0x1, 0x5, stream, H2_OK_TRAILERS),
                        ] {
                            if to.write_all(&frame).await.is_err() {
                                return;
                            }
                        }
                        let _ = to.flush().await;
                    }
                }
            }
        }
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
}

/// An `https` loopback endpoint nothing listens on — for a `Required` wallet,
/// whose private leg rides the host dialer and never reaches the port.
async fn dead_https_loopback() -> LightServerEndpoint {
    let port = {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback");
        listener.local_addr().expect("addr").port()
    };
    LightServerEndpoint::new(format!("https://127.0.0.1:{port}")).expect("loopback endpoint")
}

/// Install the `wallet.dial` capture for this test's thread; the guard must
/// outlive every dial the test drives.
fn capture() -> (CapturedEvents, tracing::subscriber::DefaultGuard) {
    use tracing_subscriber::layer::SubscriberExt;
    force_wallet_callsites_enabled();
    let sink = CapturedEvents::default();
    let guard = tracing::subscriber::set_default(
        tracing_subscriber::registry().with(CaptureLayer::new(sink.clone())),
    );
    (sink, guard)
}

/// Every `wallet.dial` line captured so far as `(arm, class)`, in order.
fn dial_arms(sink: &CapturedEvents) -> Vec<(String, String)> {
    sink.records_of("wallet.dial")
        .into_iter()
        .map(|fields| {
            let get = |name: &str| {
                fields
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default()
            };
            (get("dial_arm"), get("dial_class"))
        })
        .collect()
}

// ── The wallet ───────────────────────────────────────────────────────────────

pub(super) fn config_over(
    dir: &std::path::Path,
    policy: TorPolicy,
    endpoint: LightServerEndpoint,
) -> WalletConfig {
    let mut c = cfg(dir, Network::Test, SeedPersistence::SealedKeychain);
    c.tor = policy;
    c.endpoint = endpoint;
    c
}

/// A synced wallet under `config`, holding ONE spendable transparent UTXO
/// above the shielding threshold — `synced_wallet_with_transparent_funds`'s
/// shape with the transport injectable. No network is touched: the chain is
/// `FakeChain`, the UTXO poll a scripted source.
pub(super) async fn funded_wallet(vault: &Arc<dyn KeychainPort>, config: WalletConfig) -> Wallet {
    let w = Wallet::create_with_vault(config, raw_seed(), Arc::clone(vault))
        .await
        .expect("create");
    let birthday = account::birthday_from_treestate(sync::testing::chain_tree_state(ANCHOR))
        .expect("the empty-frontier anchor decodes");
    w.import_account(birthday).await.expect("import");
    w.sync_once(
        &mut sync::testing::FakeChain::to_tip(SYNCED_TIP),
        &sync::CancelToken::new(),
        NOOP_PROGRESS,
    )
    .await
    .expect("sync records the chain tip");
    // `sync_once` is the scan half; the verdict a really-synced wallet holds.
    w.seed_current_consensus_for_test();
    fund_utxo(&w, 0x21).await;
    w
}

/// Put ONE more spendable transparent UTXO of `FUNDING_ZAT` at the tip, under
/// a txid of `tag` bytes — the engine-write path a real transparent receive
/// takes, so a second shield has something to spend.
async fn fund_utxo(w: &Wallet, tag: u8) {
    let addr = w
        .current_transparent_address()
        .await
        .expect("receive t-address");
    let record = crate::transparent::TransparentUtxoRecord {
        txid: vec![tag; 32],
        index: 0,
        script: script_paying(&decode_taddr(&addr)),
        value_zat: FUNDING_ZAT,
        height: SYNCED_TIP,
    };
    let mut src = FakeUtxoSource::ok(vec![record]);
    let out = w
        .refresh_transparent_utxos(&mut src)
        .await
        .expect("put the funding transparent UTXO");
    assert_eq!(out.put, 1, "fixture: the funding UTXO is detected + put");
}

async fn reopen(config: WalletConfig, vault: &Arc<dyn KeychainPort>) -> Wallet {
    let w = Wallet::open_with_vault(config, Arc::clone(vault))
        .await
        .expect("reopen");
    w.seed_current_consensus_for_test();
    w
}

/// Propose + sign the wallet's shield — the engine's persist WITHOUT the
/// broadcast: the state a kill between `sign_proposal_upstream` and
/// `broadcast_persisted` leaves (§3.2 row 2 (i)). Returns the upstream txid.
pub(super) async fn persist_shield(w: &Wallet) -> zcash_protocol::TxId {
    let dto = w
        .propose_shield()
        .await
        .expect("propose_shield")
        .expect("funds above the threshold");
    let txids = w
        .sign_proposal_upstream(dto.proposal_id)
        .await
        .expect("sign + persist");
    assert_eq!(txids.len(), 1, "fixture: a shield is one transaction");
    txids[0]
}

/// The PRODUCT path: propose, then `send_by_id` (sign, persist, broadcast).
/// Returns the upstream txid and the per-tx result the host would see.
async fn shield_and_send(w: &Wallet) -> (zcash_protocol::TxId, TxSubmitResult) {
    let dto = w
        .propose_shield()
        .await
        .expect("propose_shield")
        .expect("funds above the threshold");
    let mut results = w.send_by_id(dto.proposal_id).await.expect("send");
    assert_eq!(results.len(), 1, "fixture: a shield is one transaction");
    let result = results.remove(0);
    let display = match &result {
        TxSubmitResult::Success { txid }
        | TxSubmitResult::GrpcFailure { txid }
        | TxSubmitResult::SubmitFailure { txid, .. }
        | TxSubmitResult::NotAttempted { txid } => txid.clone(),
    };
    // Our `TxId` is DISPLAY order; the engine keys on INTERNAL order (the
    // `From<zcash_protocol::TxId>` door reverses, so this reverses back).
    let mut internal = *display.as_bytes();
    internal.reverse();
    (zcash_protocol::TxId::from_bytes(internal), result)
}

async fn resubmit(w: &Wallet) -> ResubmitSummary {
    w.resubmit_queued_sends(&sync::CancelToken::new())
        .await
        .expect("resubmit")
}

/// The wallet-CREATED rows the engine holds (`created IS NOT NULL` — the
/// pinned upstream's own stamp on `create_proposed_transactions`).
fn wallet_created_rows(w: &Wallet) -> i64 {
    let aux = w.inner.aux_db.lock().expect("aux db");
    aux.query_row(
        "SELECT COUNT(*) FROM transactions WHERE created IS NOT NULL",
        [],
        |r| r.get::<_, i64>(0),
    )
    .expect("count")
}

/// Move `txid`'s expiry BELOW the synced tip — the row now reads
/// `expired_unmined` in the engine's own view, exactly as a wallet that
/// scanned past the height would read it.
fn expire_below_the_tip(w: &Wallet, txid: zcash_protocol::TxId) {
    let aux = w.inner.aux_db.lock().expect("aux db");
    let n = aux
        .execute(
            "UPDATE transactions SET expiry_height = ?1 WHERE txid = ?2",
            rusqlite::params![SYNCED_TIP as u32 - 10, txid.as_ref()],
        )
        .expect("expire");
    assert_eq!(n, 1, "fixture: the row exists");
}

/// Mine `txid` at `height` through the engine's own status write — the arm
/// the enhancement pass takes for a `GetStatus` answer that names a height.
pub(super) fn mark_mined(w: &Wallet, txid: zcash_protocol::TxId, height: u32) {
    let mut guard = w.inner.db.lock().expect("wallet db");
    guard
        .db
        .set_transaction_status(
            txid,
            TransactionStatus::Mined(zcash_protocol::consensus::BlockHeight::from_u32(height)),
        )
        .expect("set Mined");
}

/// A RECEIVED row: bytes the wallet did not create — `created IS NULL`, no
/// `expiry_height`, the shape `decrypt_and_store` leaves. `raw` is a real
/// serialized transaction so any reader that parses rows can parse this one.
fn insert_received_row(w: &Wallet, raw: &[u8]) -> zcash_protocol::TxId {
    let txid = zcash_protocol::TxId::from_bytes([0x5a; 32]);
    let aux = w.inner.aux_db.lock().expect("aux db");
    aux.execute(
        "INSERT INTO transactions (txid, raw, expiry_height, min_observed_height, created) \
         VALUES (?1, ?2, NULL, ?3, NULL)",
        rusqlite::params![txid.as_ref(), raw, SYNCED_TIP as u32],
    )
    .expect("insert a received row");
    txid
}

/// An intent row that OWNS `group` — `Sent` with the ordered txids recorded,
/// its claim a note the witness reads as spent (so reconcile lands on the
/// `ReBroadcast` arm, as it does for a real created send), tagged with
/// `deadline` when the row is a swap deposit. Shared with the sibling file: its
/// four-states row reaches `Persisted` through this HELD shape.
pub(super) fn enrol_sent_row(
    w: &Wallet,
    group: &[[u8; 32]],
    deadline: Option<i64>,
) -> QueuedSendId {
    let mut aux = w.inner.aux_db.lock().expect("aux db");
    let id = crate::intent_store::enqueue(&mut aux, "zcash:t1owned", 0, deadline, None)
        .expect("enqueue");
    assert!(
        crate::intent_store::mark_submitting(
            &mut aux,
            id,
            &[NoteClaim {
                txid: [7; 32],
                protocol: PROTO_SAPLING,
                output_index: 0,
            }],
        )
        .expect("submitting")
    );
    assert!(crate::intent_store::mark_sent_multi(&mut aux, id, group).expect("sent"));
    id
}

fn in_flight_rows(w: &Wallet) -> usize {
    let aux = w.inner.aux_db.lock().expect("aux db");
    crate::intent_store::list_in_flight(&aux)
        .expect("list_in_flight")
        .len()
}

pub(super) fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

// ── §3.2 row 2: death at each point ──────────────────────────────────────────

/// Row 2 (i): the process dies after the engine's persist and before
/// `broadcast_persisted` — nothing ever reached an endpoint. The reopened
/// wallet's next pass attempts exactly that transaction, and the endpoint
/// receives the bytes the engine stored, whole. Row 1's byte-equality clause
/// is asserted here on the bytes, not the count.
#[tokio::test]
async fn a_send_that_died_before_broadcast_goes_out_on_the_next_sync() {
    let (peer, endpoint) = LoopbackLightwalletd::serve(Answer::HangUp).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let config = || config_over(dir.path(), TorPolicy::Off, endpoint.clone());

    let w = funded_wallet(&vault, config()).await;
    let txid = persist_shield(&w).await;
    let raw = w
        .raw_tx_bytes(txid)
        .await
        .expect("the persisted bytes read back");
    assert!(
        peer.requests().is_empty(),
        "fixture: nothing reached the endpoint before the kill"
    );
    assert_eq!(wallet_created_rows(&w), 1);
    w.close().await.expect("close");

    let w = reopen(config(), &vault).await;
    assert_eq!(
        w.raw_tx_bytes(txid)
            .await
            .expect("the bytes survived the reopen"),
        raw
    );
    let retry = resubmit(&w).await;
    assert_eq!(
        retry.attempted, 1,
        "a persisted signed transaction is the wallet's to deliver: one attempt on the next sync"
    );
    assert_eq!(
        peer.times_seen(&raw),
        1,
        "the endpoint received the SIGNED bytes — the row's bytes, whole"
    );
    assert_eq!(
        w.dial_counts().clearnet.connected,
        1,
        "one connection carried it (an Off wallet's direct dialer)"
    );
    assert_eq!(
        wallet_created_rows(&w),
        1,
        "the same transaction went out — never a fresh payment"
    );
    w.close().await.expect("close");
}

/// Row 2 (ii): the broadcast ran and the endpoint gave no verdict
/// (`GrpcFailure` — it may or may not have landed), then the process died.
/// Next pass after the reopen: the SAME bytes again, once.
#[tokio::test]
async fn a_send_that_died_after_a_failed_broadcast_goes_out_on_the_next_sync() {
    let (peer, endpoint) = LoopbackLightwalletd::serve(Answer::HangUp).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let config = || config_over(dir.path(), TorPolicy::Off, endpoint.clone());

    let w = funded_wallet(&vault, config()).await;
    let (txid, result) = shield_and_send(&w).await;
    assert!(
        matches!(result, TxSubmitResult::GrpcFailure { .. }),
        "fixture: the endpoint hung up without a verdict, got {result:?}"
    );
    let raw = w.raw_tx_bytes(txid).await.expect("bytes");
    assert_eq!(
        peer.times_seen(&raw),
        1,
        "fixture: the first attempt carried the bytes"
    );
    w.close().await.expect("close");

    let w = reopen(config(), &vault).await;
    let retry = resubmit(&w).await;
    assert_eq!(
        retry.attempted, 1,
        "the failed send is retried after the reopen"
    );
    assert_eq!(
        peer.times_seen(&raw),
        2,
        "the retry carried the IDENTICAL bytes the first attempt did"
    );
    assert_eq!(wallet_created_rows(&w), 1, "never a fresh payment");
    w.close().await.expect("close");
}

/// Row 2 (iii) and row 6's idempotence. Two sends: one the endpoint ACCEPTED,
/// one it gave no verdict on; the process dies; the next pass attempts BOTH —
/// acceptance does not end delivery, confirmation or expiry does (§1: the
/// bytes go out again "until they confirm or expire"; row 6: a second
/// broadcast of accepted bytes is the endpoint's Success, never a second
/// payment). The accepted mark SURVIVED the reopen (a close is a kill for the
/// money tables: every write is committed before `send` returns) and is
/// asserted where it lives — on the per-transaction reading, `Accepted`,
/// taken after the reopen and BEFORE the pass that would re-write it — never
/// through a suppressed broadcast. Its loss mode, named: harmless — no
/// broadcast decision reads it; a lost mark costs only the reading, which says
/// retry-pending until the next pass's Success re-writes it. The rebroadcast
/// of the accepted bytes is what the pass shows: Success again, no new row.
/// (Repaired at the adjudication, ruling.md §6: as first written the row
/// asserted a stop the exit gate forbids.)
#[tokio::test]
async fn an_accepted_send_is_not_broadcast_again_or_only_idempotently() {
    let (peer, endpoint) = LoopbackLightwalletd::serve(Answer::Accept).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let config = || config_over(dir.path(), TorPolicy::Off, endpoint.clone());

    let w = funded_wallet(&vault, config()).await;
    let (accepted, result) = shield_and_send(&w).await;
    assert!(
        matches!(result, TxSubmitResult::Success { .. }),
        "fixture: the loopback accepted, got {result:?}"
    );
    let raw_accepted = w.raw_tx_bytes(accepted).await.expect("bytes");
    assert_eq!(peer.times_seen(&raw_accepted), 1);

    fund_utxo(&w, 0x22).await;
    peer.set_answer(Answer::HangUp);
    let (missed, result) = shield_and_send(&w).await;
    assert!(
        matches!(result, TxSubmitResult::GrpcFailure { .. }),
        "fixture: the second send got no verdict, got {result:?}"
    );
    let raw_missed = w.raw_tx_bytes(missed).await.expect("bytes");
    assert_eq!(peer.times_seen(&raw_missed), 1);
    assert_eq!(wallet_created_rows(&w), 2);
    w.close().await.expect("close");

    peer.set_answer(Answer::Accept);
    let w = reopen(config(), &vault).await;
    assert_eq!(
        w.transaction(TxId::from(accepted))
            .await
            .expect("history read")
            .expect("the accepted shield is a history row")
            .delivery,
        Some(DeliveryState::Accepted),
        "the accepted mark survived the reopen and says so on the reading"
    );
    let retry = resubmit(&w).await;
    assert_eq!(
        retry.attempted, 2,
        "both unmined sends are attempted — acceptance does not end delivery"
    );
    assert_eq!(
        retry.accepted, 2,
        "the endpoint's Success for both: the accepted bytes, idempotently"
    );
    assert_eq!(
        peer.times_seen(&raw_accepted),
        2,
        "the ACCEPTED bytes are re-offered, harmlessly — the mempool knows them"
    );
    assert_eq!(peer.times_seen(&raw_missed), 2, "the missed send went out");
    assert_eq!(wallet_created_rows(&w), 2, "idempotent: no second payment");
    w.close().await.expect("close");
}

// ── §3.2 row 3: the intent machinery keeps what it owns ─────────────────────

/// Row 3, the cross-item row, both arms of the hold. The HELD shape: the
/// intent row alive and `Sent` with the transaction's txid, the transaction
/// unexpired, and the deposit gate reading `Expired` (a deadline an hour past)
/// or `Wait` (the broadcast phase's fresh clock below the plausibility floor).
/// The pass before the row is enrolled attempts the transaction — the
/// control; with the row alive nothing does, and the row is untouched.
#[tokio::test]
async fn a_held_deposit_is_not_broadcast_by_the_generic_phase() {
    let (peer, endpoint) = LoopbackLightwalletd::serve(Answer::HangUp).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();

    let w = funded_wallet(&vault, config_over(dir.path(), TorPolicy::Off, endpoint)).await;
    let (txid, result) = shield_and_send(&w).await;
    assert!(matches!(result, TxSubmitResult::GrpcFailure { .. }));
    let raw = w.raw_tx_bytes(txid).await.expect("bytes");

    // Control: unowned, the transaction is retried.
    let retry = resubmit(&w).await;
    assert_eq!(
        retry.attempted, 1,
        "control: an unowned signed transaction is retried"
    );
    assert_eq!(peer.times_seen(&raw), 2);

    // The Expired arm: a signed deposit whose quote lapsed, tx still valid.
    let lapsed = enrol_sent_row(&w, &[*txid.as_ref()], Some(now_unix() - 3_600));
    let held = resubmit(&w).await;
    assert_eq!(
        held.attempted, 0,
        "a held deposit is never broadcast — by either phase"
    );
    assert_eq!(held.deposit_held, 1, "the reconcile arm held it (Expired)");
    assert_eq!(peer.times_seen(&raw), 2, "no bytes reached the endpoint");
    assert_eq!(
        in_flight_rows(&w),
        1,
        "the row is untouched: the terminals own it"
    );

    // The Wait arm: the quote is live, the broadcast phase's clock is not.
    {
        let mut aux = w.inner.aux_db.lock().expect("aux db");
        assert!(crate::intent_store::delete(&mut aux, lapsed).expect("delete"));
    }
    enrol_sent_row(&w, &[*txid.as_ref()], Some(now_unix() + 3_600));
    let held = w
        .resubmit_queued_sends_with_broadcast_clock(&sync::CancelToken::new(), Some(10))
        .await
        .expect("resubmit");
    assert_eq!(held.attempted, 0, "an untimeable clock holds fail-safe");
    assert_eq!(held.deposit_held, 1, "the broadcast re-gate held it (Wait)");
    assert_eq!(peer.times_seen(&raw), 2, "no bytes reached the endpoint");
    assert_eq!(in_flight_rows(&w), 1);
    w.close().await.expect("close");
}

/// Row 3's ordered-group clause: a `Sent` row owning `[tx0, tx1]`, driven
/// through one pass, puts tx0 on the wire before tx1 and each exactly once —
/// the generic phase adds nothing to what the intent machinery sends.
#[tokio::test]
async fn a_two_step_driven_through_both_phases_broadcasts_in_order_exactly_once() {
    let (peer, endpoint) = LoopbackLightwalletd::serve(Answer::HangUp).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();

    let w = funded_wallet(&vault, config_over(dir.path(), TorPolicy::Off, endpoint)).await;
    let (tx0, result) = shield_and_send(&w).await;
    assert!(matches!(result, TxSubmitResult::GrpcFailure { .. }));
    let raw0 = w.raw_tx_bytes(tx0).await.expect("bytes");
    fund_utxo(&w, 0x22).await;
    let tx1 = persist_shield(&w).await;
    let raw1 = w.raw_tx_bytes(tx1).await.expect("bytes");
    let already = peer.requests().len();
    assert_eq!(already, 1, "fixture: one failed attempt on record");

    enrol_sent_row(&w, &[*tx0.as_ref(), *tx1.as_ref()], None);
    peer.set_answer(Answer::Accept);
    let pass = resubmit(&w).await;
    assert_eq!(pass.attempted, 2, "the group's two transactions, once each");
    assert_eq!(pass.accepted, 2);
    let at0 = peer
        .first_seen_from(&raw0, already)
        .expect("tx0 went out this pass");
    let at1 = peer
        .first_seen_from(&raw1, already)
        .expect("tx1 went out this pass");
    assert!(at0 < at1, "tx0 before tx1: request {at0} then {at1}");
    assert_eq!(
        peer.requests().len(),
        already + 2,
        "exactly two requests this pass — nothing was sent twice"
    );
    assert_eq!(peer.times_seen(&raw0), 2);
    assert_eq!(peer.times_seen(&raw1), 1);
    assert_eq!(
        in_flight_rows(&w),
        1,
        "the row stays Sent until its group buries"
    );
    w.close().await.expect("close");
}

// ── §3.2 row 4: identical bytes, never a fresh payment ──────────────────────

/// Row 4: after a failed broadcast the notes stay spent — the engine's own
/// spent-marks refuse them to a fresh propose (asserted, not assumed) — and
/// the retry after a reopen carries the identical bytes; the wallet never
/// holds a second created transaction for the same funds.
#[tokio::test]
async fn a_failed_broadcast_never_becomes_a_fresh_payment_while_the_first_is_unexpired() {
    let (peer, endpoint) = LoopbackLightwalletd::serve(Answer::HangUp).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let config = || config_over(dir.path(), TorPolicy::Off, endpoint.clone());

    let w = funded_wallet(&vault, config()).await;
    let (txid, result) = shield_and_send(&w).await;
    assert!(matches!(result, TxSubmitResult::GrpcFailure { .. }));
    let raw = w.raw_tx_bytes(txid).await.expect("bytes");
    assert!(
        w.propose_shield().await.expect("propose_shield").is_none(),
        "the notes the unmined transaction spends are refused to a fresh propose"
    );
    w.close().await.expect("close");

    let w = reopen(config(), &vault).await;
    let retry = resubmit(&w).await;
    assert_eq!(retry.attempted, 1);
    assert_eq!(
        peer.times_seen(&raw),
        2,
        "the retry is the same bytes — a rebroadcast, never a re-sign"
    );
    assert_eq!(
        wallet_created_rows(&w),
        1,
        "one created transaction, before and after"
    );
    assert!(
        w.propose_shield().await.expect("propose_shield").is_none(),
        "and still refused after the retry: the notes are spent until it mines or expires"
    );
    w.close().await.expect("close");
}

// ── §3.2 rows 5 and 6: expiry and confirmation end it ───────────────────────

/// Row 5: past `expiry_height` the transaction leaves the obligation — no
/// attempt, no bytes — the notes free (a fresh shield PROPOSES again, which is
/// the user's new payment to initiate, not the wallet's), and no automatic
/// re-send exists afterwards. With nothing owed, a rescan proceeds.
#[tokio::test]
async fn an_expired_transaction_leaves_the_obligation_and_is_never_rebroadcast() {
    let (peer, endpoint) = LoopbackLightwalletd::serve(Answer::HangUp).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();

    let w = funded_wallet(&vault, config_over(dir.path(), TorPolicy::Off, endpoint)).await;
    let (txid, result) = shield_and_send(&w).await;
    assert!(matches!(result, TxSubmitResult::GrpcFailure { .. }));
    let raw = w.raw_tx_bytes(txid).await.expect("bytes");
    let retry = resubmit(&w).await;
    assert_eq!(retry.attempted, 1, "control: unexpired, it is retried");
    assert_eq!(peer.times_seen(&raw), 2);

    expire_below_the_tip(&w, txid);
    let after = resubmit(&w).await;
    assert_eq!(after.attempted, 0, "expired: never rebroadcast");
    assert_eq!(peer.times_seen(&raw), 2, "no bytes reached the endpoint");
    assert_eq!(
        wallet_created_rows(&w),
        1,
        "no automatic re-send: a new payment is the user's to initiate"
    );
    assert!(
        w.propose_shield().await.expect("propose_shield").is_some(),
        "the notes freed: a fresh payment CAN be proposed — by the user"
    );
    assert_eq!(wallet_created_rows(&w), 1, "proposing signs nothing");
    let w = w
        .rescan_from_with_vault(None, Arc::clone(&vault))
        .await
        .expect("nothing owed: the rescan proceeds");
    w.close().await.expect("close");
}

/// Row 6: a mined transaction leaves the obligation — no attempt, no bytes —
/// reads `Confirmed` on the per-transaction status, and with nothing owed a
/// rescan proceeds.
#[tokio::test]
async fn a_mined_transaction_leaves_the_obligation() {
    let (peer, endpoint) = LoopbackLightwalletd::serve(Answer::HangUp).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();

    let w = funded_wallet(&vault, config_over(dir.path(), TorPolicy::Off, endpoint)).await;
    let (txid, result) = shield_and_send(&w).await;
    assert!(matches!(result, TxSubmitResult::GrpcFailure { .. }));
    let raw = w.raw_tx_bytes(txid).await.expect("bytes");
    let retry = resubmit(&w).await;
    assert_eq!(retry.attempted, 1, "control: unmined, it is retried");
    assert_eq!(peer.times_seen(&raw), 2);

    mark_mined(&w, txid, SYNCED_TIP as u32);
    let after = resubmit(&w).await;
    assert_eq!(after.attempted, 0, "mined: never rebroadcast");
    assert_eq!(peer.times_seen(&raw), 2, "no bytes reached the endpoint");
    let row = w
        .transaction(TxId::from(txid))
        .await
        .expect("history read")
        .expect("the shield is a history row");
    assert!(
        matches!(row.status, TxStatus::Confirmed { .. }),
        "the per-transaction status reads confirmed, got {:?}",
        row.status
    );
    let w = w
        .rescan_from_with_vault(None, Arc::clone(&vault))
        .await
        .expect("nothing owed: the rescan proceeds");
    w.close().await.expect("close");
}

// ── §3.2 row 7: the rescan fence ─────────────────────────────────────────────

/// Row 7: a user-driven rescan under a broadcast-but-unmined single-step send
/// is refused, typed `RescanWithInFlightSend`. The refusal consumes the handle
/// like every rescan fault and nothing else: the reopened wallet still owes
/// the send, and pays it on the next pass.
#[tokio::test]
async fn a_rescan_under_an_unmined_single_step_send_is_refused() {
    let (peer, endpoint) = LoopbackLightwalletd::serve(Answer::HangUp).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let config = || config_over(dir.path(), TorPolicy::Off, endpoint.clone());

    let w = funded_wallet(&vault, config()).await;
    let (txid, result) = shield_and_send(&w).await;
    assert!(matches!(result, TxSubmitResult::GrpcFailure { .. }));
    let raw = w.raw_tx_bytes(txid).await.expect("bytes");
    assert_eq!(
        in_flight_rows(&w),
        0,
        "fixture: a single-step send keeps no intent row"
    );

    let err = match w.rescan_from_with_vault(None, Arc::clone(&vault)).await {
        Err(e) => e,
        Ok(_) => panic!("a rescan must refuse while a signed send is unmined and unexpired"),
    };
    assert!(
        matches!(err, WalletError::RescanWithInFlightSend),
        "typed fence over the persisted send, got {err:?}"
    );

    let w = reopen(config(), &vault).await;
    assert_eq!(
        w.raw_tx_bytes(txid)
            .await
            .expect("the refusal left the wallet intact"),
        raw
    );
    let retry = resubmit(&w).await;
    assert_eq!(
        retry.attempted, 1,
        "the refusal did not cost the obligation"
    );
    assert_eq!(peer.times_seen(&raw), 2);
    w.close().await.expect("close");
}

/// NOT IN THE CONTRACT'S LIST (IT-1 +A). Row 7's fence and row 9's exclusion
/// must be the ONE predicate: a received, unmined row (`created IS NULL`) is
/// not an obligation, so it never blocks a rescan — a fence over "any unmined
/// row" would refuse every wallet with a payment awaiting confirmation. The
/// received row is present throughout: the fence fires on the created send and
/// releases once it mines, with the received row still unmined beside it.
#[tokio::test]
async fn a_received_transaction_never_blocks_a_rescan() {
    let (_peer, endpoint) = LoopbackLightwalletd::serve(Answer::HangUp).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let config = || config_over(dir.path(), TorPolicy::Off, endpoint.clone());

    let w = funded_wallet(&vault, config()).await;
    let (txid, result) = shield_and_send(&w).await;
    assert!(matches!(result, TxSubmitResult::GrpcFailure { .. }));
    let raw = w.raw_tx_bytes(txid).await.expect("bytes");
    insert_received_row(&w, &raw);

    let err = match w.rescan_from_with_vault(None, Arc::clone(&vault)).await {
        Err(e) => e,
        Ok(_) => panic!("the created send is owed: the rescan must refuse"),
    };
    assert!(
        matches!(err, WalletError::RescanWithInFlightSend),
        "got {err:?}"
    );

    let w = reopen(config(), &vault).await;
    mark_mined(&w, txid, SYNCED_TIP as u32);
    let w = w
        .rescan_from_with_vault(None, Arc::clone(&vault))
        .await
        .expect("the created send mined; the received row alone blocks nothing");
    w.close().await.expect("close");
}

// ── §3.2 row 8: reorg ────────────────────────────────────────────────────────

/// Row 8: a transaction un-mined by a rewind — `sync::rewind_wallet_to`, the
/// ONE body behind both reorg callers — re-enters the obligation and is
/// attempted on the next pass, with the same bytes.
#[tokio::test]
async fn a_reorged_out_transaction_re_enters_the_obligation() {
    let (peer, endpoint) = LoopbackLightwalletd::serve(Answer::HangUp).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();

    let w = funded_wallet(&vault, config_over(dir.path(), TorPolicy::Off, endpoint)).await;
    let (txid, result) = shield_and_send(&w).await;
    assert!(matches!(result, TxSubmitResult::GrpcFailure { .. }));
    let raw = w.raw_tx_bytes(txid).await.expect("bytes");
    mark_mined(&w, txid, SYNCED_TIP as u32);
    let mined = resubmit(&w).await;
    assert_eq!(mined.attempted, 0, "fixture: mined, nothing owed");
    assert_eq!(peer.times_seen(&raw), 1);

    let landing = {
        let mut guard = w.inner.db.lock().expect("wallet db");
        let aux = w.inner.aux_db.lock().expect("aux db");
        sync::rewind_wallet_to(
            &mut guard.db,
            &aux,
            zcash_protocol::consensus::BlockHeight::from_u32(SYNCED_TIP as u32 - 1),
            None,
        )
        .expect("the reorg body rewinds")
    };
    assert!(
        u64::from(u32::from(landing.landed)) < SYNCED_TIP,
        "fixture: the rewind landed below the mining block"
    );
    let reorged = resubmit(&w).await;
    assert_eq!(
        reorged.attempted, 1,
        "un-mined by the reorg, the transaction is owed again"
    );
    assert_eq!(peer.times_seen(&raw), 2, "the same bytes, once more");
    assert_eq!(wallet_created_rows(&w), 1);
    w.close().await.expect("close");
}

// ── §3.2 row 9: a received transaction is never enumerated ──────────────────

/// Row 9: a row stored the way the decrypt path stores one (`created IS NULL`)
/// is never broadcast, beside a created one that is. The received row carries
/// the SAME serialized bytes under another txid, so a phase that enumerated
/// it would show as a third carriage of those bytes — the count is exact.
#[tokio::test]
async fn a_received_transaction_is_never_enumerated_for_broadcast() {
    let (peer, endpoint) = LoopbackLightwalletd::serve(Answer::HangUp).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();

    let w = funded_wallet(&vault, config_over(dir.path(), TorPolicy::Off, endpoint)).await;
    let (txid, result) = shield_and_send(&w).await;
    assert!(matches!(result, TxSubmitResult::GrpcFailure { .. }));
    let raw = w.raw_tx_bytes(txid).await.expect("bytes");
    insert_received_row(&w, &raw);
    assert_eq!(
        wallet_created_rows(&w),
        1,
        "fixture: one created row, one received"
    );

    let pass = resubmit(&w).await;
    assert_eq!(
        pass.attempted, 1,
        "exactly the created transaction is attempted"
    );
    assert_eq!(
        peer.times_seen(&raw),
        2,
        "one carriage this pass: the received copy was not broadcast (it would be a third)"
    );
    w.close().await.expect("close");
}

// ── §3.2 row 11: the private path, on the host's dialer ─────────────────────

/// Row 11: a rebroadcast rides the dial policy of the first broadcast, on the
/// dialer the host REGISTERED — `TorRuntime::HostDialer`, the variant the
/// bridge's `registered_host_dialer()` produces for `TorRuntimeConfig::HostDialer`
/// (`net_dialer_cabi.rs`, FR-29), driven here through the core's scripted
/// double. Under `Required`: zero clearnet dials, ever; every dial line private
/// and of the broadcast class; a FRESH isolation key per attempt, prefixed as
/// a broadcast key and never the sync key.
#[tokio::test]
async fn a_rebroadcast_under_required_makes_no_clearnet_dial_on_the_host_dialer() {
    let (sink, _guard) = capture();
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let endpoint = dead_https_loopback().await;
    let host = Arc::new(ScriptedHostDialer::with(
        Some(ScriptedHostDialer::tor_ready()),
        Err(HostDialCode::Unreachable),
    ));
    let config = || {
        config_over(
            dir.path(),
            TorPolicy::Required {
                runtime: TorRuntime::HostDialer(Arc::clone(&host) as Arc<dyn HostDialer>),
            },
            endpoint.clone(),
        )
    };

    let w = funded_wallet(&vault, config()).await;
    let (txid, result) = shield_and_send(&w).await;
    assert!(
        matches!(result, TxSubmitResult::GrpcFailure { .. }),
        "fixture: the host's path is unreachable, got {result:?}"
    );
    let first_keys = host.keys_seen();
    assert_eq!(first_keys.len(), 1, "fixture: one dial, on the host dialer");
    let first = first_keys[0]
        .clone()
        .expect("a broadcast dial carries its isolation key");
    assert!(first.starts_with(BROADCAST_ISOLATION_KEY_PREFIX));
    w.close().await.expect("close");

    let w = reopen(config(), &vault).await;
    let retry = resubmit(&w).await;
    assert_eq!(retry.attempted, 1, "the send is retried after the reopen");
    let keys = host.keys_seen();
    assert_eq!(keys.len(), 2, "the retry dialed the HOST dialer, once");
    let second = keys[1].clone().expect("the retry carries an isolation key");
    assert_ne!(second, first, "a fresh circuit per attempt");
    assert!(second.starts_with(BROADCAST_ISOLATION_KEY_PREFIX));
    assert_ne!(second, WALLET_SYNC_ISOLATION_KEY, "never the sync circuit");
    assert_eq!(
        w.dial_counts().clearnet,
        ArmCounts::default(),
        "Required: zero clearnet dials on the reopened wallet"
    );
    assert_eq!(w.dial_counts().private.unreachable, 1);
    let lines = dial_arms(&sink);
    assert_eq!(
        lines.len(),
        2,
        "two dial lines across both wallets: {lines:?}"
    );
    assert!(
        lines
            .iter()
            .all(|(arm, class)| arm == "host" && class == "broadcast"),
        "every dial private and of the broadcast class: {lines:?}"
    );
    assert!(
        !w.raw_tx_bytes(txid).await.expect("bytes").is_empty(),
        "the bytes stayed persisted through the refused dials"
    );
    w.close().await.expect("close");
}

// ── NOT IN THE CONTRACT'S LIST (IT-1 +A) ────────────────────────────────────

/// The obligation is every persisted send, not the last one: two signed
/// shields — one that got no verdict, one never broadcast — are BOTH attempted
/// on the pass after a reopen, each with its own bytes.
#[tokio::test]
async fn every_persisted_send_is_retried_after_a_reopen_not_only_the_last() {
    let (peer, endpoint) = LoopbackLightwalletd::serve(Answer::HangUp).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let config = || config_over(dir.path(), TorPolicy::Off, endpoint.clone());

    let w = funded_wallet(&vault, config()).await;
    let (missed, result) = shield_and_send(&w).await;
    assert!(matches!(result, TxSubmitResult::GrpcFailure { .. }));
    let raw_missed = w.raw_tx_bytes(missed).await.expect("bytes");
    fund_utxo(&w, 0x22).await;
    let never = persist_shield(&w).await;
    let raw_never = w.raw_tx_bytes(never).await.expect("bytes");
    assert_eq!(wallet_created_rows(&w), 2);
    w.close().await.expect("close");

    let w = reopen(config(), &vault).await;
    let retry = resubmit(&w).await;
    assert_eq!(retry.attempted, 2, "both persisted sends are attempted");
    assert_eq!(peer.times_seen(&raw_missed), 2);
    assert_eq!(peer.times_seen(&raw_never), 1);
    assert_eq!(wallet_created_rows(&w), 2, "no fresh payment for either");
    w.close().await.expect("close");
}
