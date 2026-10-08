//! Stage S16 `sever` + `poison` (FR-53) — the test author's rows that COMPILE
//! at the base commit, written BLIND (IT-2a) against `2a8223f4`, contract
//! `docs/plan/stage-16-the-duress-force-sever.md` §3.1 and §3.2 (revision 5).
//! A child of `wallet::tests` for its fixtures (`cfg`, `raw_seed`, `shared_mk`,
//! `keychain_namespace_for`, `FakeUtxoSource`, `script_paying`,
//! `decode_taddr`) and of `delivery_obligation`'s loopback lightwalletd and
//! funded wallet; its own file so no cited line in `wallet.rs` moves.
//!
//! Every row that names something the implementer ADDS (the verb, its report,
//! the path table, the seams) is in the sibling `s16_sever_named.rs`, declared
//! but never built until the join. The rows here use only names the base tree
//! carries:
//!
//! - `a_create_writes_its_custody_index_before_its_wrap_key` — §3.1 item 4's
//!   index-first provisioning, observed at the key store. RED at base (the base
//!   tree writes the key first, `store.rs:695` before `:698-700`).
//! - `a_transport_miss_on_an_open_wallet_is_unchanged` — §3.2 assertion 8, the
//!   byte-identical promise for a wallet nobody severed. GREEN at base by
//!   design: it is the guard that the broadcast refusal changes nothing else.
//! - `relims_duress_sequence_fits_its_ten_second_budget` — §3.1 "Fixtures
//!   priced", a constant assertion over the host's two numbers. GREEN at base
//!   by design (the constant exists since S9).

use std::sync::Mutex;

use super::delivery_obligation::{
    Answer, LoopbackLightwalletd, SYNCED_TIP, config_over, funded_wallet, persist_shield,
};
use super::*;
use crate::custody::CustodyIndexEntry;
use crate::keychain::testvault::SharedKeychain;
use crate::keychain::{ResolvedVault, WrapArtifact};
use crate::seal::SealKey;
use crate::tracing_guard::{CaptureLayer, CapturedEvents, force_wallet_callsites_enabled};

// ───────────────────────────── index-first provisioning ─────────────────────

/// Records every custody WRITE that reaches the shared keychain, in order, as
/// `(op, namespace)`; every call delegates.
struct Recording {
    inner: Arc<dyn KeychainPort>,
    ns: String,
    writes: Arc<Mutex<Vec<(&'static str, String)>>>,
}

impl Recording {
    fn note(&self, op: &'static str) {
        self.writes
            .lock()
            .expect("writes")
            .push((op, self.ns.clone()));
    }
}

impl KeychainPort for Recording {
    fn probe(&self) -> Result<(), WalletError> {
        self.inner.probe()
    }
    fn tier(&self) -> Result<VaultTier, WalletError> {
        self.inner.tier()
    }
    fn store_wrap_key(&self, key: SealKey, blob: &[u8]) -> Result<WrapArtifact, WalletError> {
        self.note("store_wrap_key");
        self.inner.store_wrap_key(key, blob)
    }
    fn load_wrap_key(&self, a: &WrapArtifact, blob: &[u8]) -> Result<SealKey, WalletError> {
        self.inner.load_wrap_key(a, blob)
    }
    fn rotate_wrap_key(&self, a: &WrapArtifact, blob: &[u8]) -> Result<WrapArtifact, WalletError> {
        self.note("rotate_wrap_key");
        self.inner.rotate_wrap_key(a, blob)
    }
    fn finish_rotation(&self, old: &WrapArtifact, new: &WrapArtifact) -> Result<(), WalletError> {
        self.inner.finish_rotation(old, new)
    }
    fn delete_wrap_key(&self, a: &WrapArtifact) -> Result<(), WalletError> {
        self.inner.delete_wrap_key(a)
    }
    fn purge_namespace(&self) -> Result<usize, WalletError> {
        self.inner.purge_namespace()
    }
    fn store_index(&self, entry: &CustodyIndexEntry) -> Result<(), WalletError> {
        self.note("store_index");
        self.inner.store_index(entry)
    }
    fn load_index(&self) -> Result<Option<CustodyIndexEntry>, WalletError> {
        self.inner.load_index()
    }
    fn delete_index(&self) -> Result<(), WalletError> {
        self.inner.delete_index()
    }
}

struct RecordingResolver {
    keychain: SharedKeychain,
    writes: Arc<Mutex<Vec<(&'static str, String)>>>,
}

impl VaultResolver for RecordingResolver {
    fn vault_for<'a>(
        &'a self,
        namespace: &KeychainNamespace,
    ) -> Result<ResolvedVault<'a>, WalletError> {
        Ok(ResolvedVault::Owned(Arc::new(Recording {
            inner: shared_mk(&self.keychain, namespace.as_str()),
            ns: namespace.as_str().to_owned(),
            writes: Arc::clone(&self.writes),
        })))
    }
}

/// §3.1 item 4, "Provisioning's order is reversed to index-first": a fresh
/// create writes the path-keyed index item — already `done(id)`, never
/// `pending` — BEFORE the wrap key exists under the id's namespace, so a key
/// that lands before a sever's tombstone is always one the sever can resolve.
/// Observed at the key store, in the order the writes arrived.
#[tokio::test]
async fn a_create_writes_its_custody_index_before_its_wrap_key() {
    let keychain = SharedKeychainVault::shared();
    let writes: Arc<Mutex<Vec<(&'static str, String)>>> = Arc::default();
    let parent = tempfile::tempdir().expect("parent dir");
    let dir = parent.path().join("index-first").join("wallet");
    let path_ns = keychain_namespace_for(&dir).as_str().to_owned();

    Wallet::create_resolving(
        cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
        raw_seed(),
        Arc::new(RecordingResolver {
            keychain: keychain.clone(),
            writes: Arc::clone(&writes),
        }),
    )
    .await
    .expect("create")
    .close()
    .await
    .expect("close the create");

    let artifact = WrapArtifact::from_bytes(
        std::fs::read(dir.join(crate::constants::WRAP_ARTIFACT_FILE_NAME)).expect("header"),
    )
    .expect("header parses");
    let id = artifact
        .custody_id()
        .expect("a fresh create frames its identifier");
    let id_ns = crate::custody::namespace_for(&id).as_str().to_owned();

    let writes = writes.lock().expect("writes").clone();
    let index_at = writes
        .iter()
        .position(|(op, ns)| *op == "store_index" && *ns == path_ns)
        .unwrap_or_else(|| panic!("the create wrote no index under the path: {writes:?}"));
    let key_at = writes
        .iter()
        .position(|(op, ns)| *op == "store_wrap_key" && *ns == id_ns)
        .unwrap_or_else(|| panic!("the create wrote no key under its id: {writes:?}"));
    assert!(
        index_at < key_at,
        "the index must name the id BEFORE the key exists under it (index-first); \
         the writes arrived as {writes:?}"
    );

    let index = shared_mk(&keychain, &path_ns)
        .load_index()
        .expect("read the index")
        .expect("the index item is there");
    assert!(
        index == CustodyIndexEntry::done(&id),
        "the index a fresh create writes first is done(id), never pending"
    );
    assert!(
        !index.is_pending(),
        "a fresh wallet has no legacy namespace"
    );
    assert!(keychain.lock().expect("kc").contains_key(&id_ns));
}

// ───────────────────────────── the transport miss ────────────────────────────

/// The funding UTXO — above `SHIELDING_THRESHOLD_ZAT`.
const FUNDING_ZAT: i64 = 500_000;

/// One more spendable transparent UTXO at the tip, so a second shield has
/// something to spend (`delivery_obligation`'s private `fund_utxo`, copied).
pub(super) async fn fund_another_utxo(w: &Wallet, tag: u8) {
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

fn capture() -> (CapturedEvents, tracing::subscriber::DefaultGuard) {
    use tracing_subscriber::layer::SubscriberExt;
    force_wallet_callsites_enabled();
    let sink = CapturedEvents::default();
    let guard = tracing::subscriber::set_default(
        tracing_subscriber::registry().with(CaptureLayer::new(sink.clone())),
    );
    (sink, guard)
}

fn field(record: &[(String, String)], name: &str) -> String {
    record
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

/// The per-tx result's kind, without its txid.
fn kind(r: &TxSubmitResult) -> &'static str {
    match r {
        TxSubmitResult::Success { .. } => "success",
        TxSubmitResult::GrpcFailure { .. } => "grpc_failure",
        TxSubmitResult::SubmitFailure { .. } => "submit_failure",
        TxSubmitResult::NotAttempted { .. } => "not_attempted",
    }
}

/// §3.2 assertion 8: with the phase `Open`, an endpoint that never answers
/// gives exactly what it gives today — `Ok(results)`, tx0 `GrpcFailure` and
/// tx1 `NotAttempted` in that order, one `wallet.send` line with the same
/// counts, NO `wallet.broadcast_refused`, and both transactions still
/// persisted for the §1.7 resubmission, which attempts them.
#[tokio::test]
async fn a_transport_miss_on_an_open_wallet_is_unchanged() {
    let (peer, endpoint) = LoopbackLightwalletd::serve(Answer::HangUp).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = funded_wallet(&vault, config_over(dir.path(), TorPolicy::Off, endpoint)).await;
    let tx0 = persist_shield(&w).await;
    fund_another_utxo(&w, 0x31).await;
    let tx1 = persist_shield(&w).await;
    let raw0 = w.raw_tx_bytes(tx0).await.expect("tx0 bytes");

    let (sink, _guard) = capture();
    let results = w
        .broadcast_persisted(vec![tx0, tx1])
        .await
        .expect("a transport miss is per-tx DATA, never an error");
    assert_eq!(
        results.iter().map(kind).collect::<Vec<_>>(),
        ["grpc_failure", "not_attempted"],
        "the latch: tx0 missed, tx1 never attempted — {results:?}"
    );
    assert_eq!(
        peer.times_seen(&raw0),
        1,
        "tx0 went out once and got no verdict"
    );
    let sends = sink.records_of("wallet.send");
    assert_eq!(sends.len(), 1, "one wallet.send line: {sends:?}");
    assert_eq!(field(&sends[0], "accepted"), "0");
    assert_eq!(field(&sends[0], "rejected"), "0");
    assert_eq!(field(&sends[0], "outcome"), "partial");
    assert!(
        sink.records_of("wallet.broadcast_refused").is_empty(),
        "an open wallet's miss is never a refusal"
    );
    assert!(w.raw_tx_bytes(tx0).await.is_ok(), "tx0 is still persisted");
    assert!(w.raw_tx_bytes(tx1).await.is_ok(), "tx1 is still persisted");

    let retry = w
        .resubmit_queued_sends(&sync::CancelToken::new())
        .await
        .expect("resubmit");
    assert_eq!(
        retry.attempted, 2,
        "both persisted sends stay the wallet's to deliver"
    );
    w.close().await.expect("close");
}

// ───────────────────────────── the host's budget ─────────────────────────────

/// §3.1 "Fixtures priced": Relim's duress sequence — its OWN close sub-bound
/// (3 s, Relim's number), then the sever inside the SDK's wipe budget, plus
/// Relim's 1 s of slack (its number) — fits the 10 s panic-wipe budget
/// (`censorship.md` §1 D14, Relim's number). A floor, not a proof of cancel
/// safety (§1's cancel note).
#[test]
fn relims_duress_sequence_fits_its_ten_second_budget() {
    use crate::keychain::bounded::KEYCHAIN_WIPE_BUDGET;
    const RELIM_CLOSE_SUB_BOUND: std::time::Duration = std::time::Duration::from_secs(3);
    const RELIM_SLACK: std::time::Duration = std::time::Duration::from_secs(1);
    const RELIM_PANIC_WIPE_BUDGET: std::time::Duration = std::time::Duration::from_secs(10);
    assert!(
        RELIM_CLOSE_SUB_BOUND + KEYCHAIN_WIPE_BUDGET + RELIM_SLACK <= RELIM_PANIC_WIPE_BUDGET,
        "close {RELIM_CLOSE_SUB_BOUND:?} + sever {KEYCHAIN_WIPE_BUDGET:?} + slack \
         {RELIM_SLACK:?} must fit {RELIM_PANIC_WIPE_BUDGET:?}"
    );
}
