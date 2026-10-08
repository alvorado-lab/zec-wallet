//! Stage S2 `rescan` — the test author's rows (contract
//! `docs/plan/stage-2-the-hosts-lifecycle.md` §3.2), written blind (IT-2a)
//! against `b2d30176`: a host-seed (`SeedPersistence::None`) wallet can be
//! told to look further back, the seed arriving through the port the host
//! already registered at the FIRST sync after the rebuild — never pulled at
//! the rescan and never held across it (mechanism (a); a pull at the rescan
//! is the witness of the ruled-out (b)). A child of `wallet::tests` for its
//! fixtures (`cfg`, `test_vault`, `create_none_then_reopen_with_port`,
//! `ironwood_receipt_for`, `one_payment`), in its own file so this module's
//! cited lines stay where their watches printed them (the `truth_probes`
//! precedent).
//!
//! RED AT BASE, by design: every row whose story crosses `rescan_from` on a
//! None wallet stops at the base gate (`rescan_inner` step (1):
//! `seed.is_none() ⇒ SeedRequired`) — the one behaviour change this item
//! makes. Two rows are green at base ON PURPOSE and say so at their head:
//! the no-port refusal (the base answer, kept for the right reason once the
//! gate learns "is a port registered") and the SealedKeychain control that
//! proves the receipt fixture itself scans, so the None rows can redden on
//! the gate and nothing else.

use std::sync::atomic::AtomicUsize;

use super::*;
use crate::seed::SeedSupplyError;
use rusqlite::OptionalExtension;

// ── The fixture: one immutable synthetic chain with a receipt BELOW the
// restore birthday ──────────────────────────────────────────────────────────
//
// The heights are one story: a wallet RESTORED at birthday B = 280_050 (its
// treestate anchor at 280_049), a chain whose tip is 280_080, and ONE
// Ironwood receipt paying the wallet's own account at 280_020 — thirty blocks
// BELOW the birthday, so the restore's first scan ([B, tip]) cannot see it
// and the funds sit unscanned exactly the way the contract's Behaviour
// paragraph describes. `rescan_from(280_019)` (B − k − 1, the row-6
// arithmetic) drops the scan floor below the receipt.
//
// The one wrinkle the fake's honest treestates cannot express by themselves:
// `with_ironwood_receipt` stamps the CUMULATIVE Ironwood commitment count
// (1) into every block at and above the receipt's height, so a pass that
// starts ABOVE the receipt would derive frontier size 0 against a declared
// size 1 and refuse the endpoint — the pre-rescan pass must anchor on a
// treestate that already carries that one commitment. `tree_state_override`
// serves exactly that state at the restore anchor (the B1-10
// `frontier_hex`/`DeclaredTreeSizes` knob's encoding — the same bytes
// `sync_bind_proof` proves both checkers accept), keeping ONE chain honest
// for both passes: the pre-rescan pass anchors on size 1 and scans clean
// over blocks that declare 1; the post-rescan pass anchors BELOW the receipt
// where the honest state is empty, derives +1 at the receipt's own block,
// and agrees from there.

/// The treestate anchor of the restore birthday: `chain_tree_state(280_049)`
/// ⇒ account birthday B = 280_050 (`chain_birthday`'s convention — the
/// birthday sits one above its anchor).
const STORY_ANCHOR: u64 = 280_049;
/// The chain tip both passes scan toward.
const STORY_TIP: u64 = 280_080;
/// Where the receipt mines: 30 blocks below the 280_050 birthday.
const RECEIPT_H: u64 = 280_020;
/// Where the rescan drops the floor to: the receipt's height minus one (the
/// contract's `rescan_from(B − k − 1)`). The re-provision then floors at the
/// bundle's activation row, so the post-rescan pass scans [280_001, tip] —
/// one 80-block batch.
const RESCAN_H: u32 = 280_019;
/// The receipt's exact value, zatoshis.
const RECEIPT_ZAT: u64 = 50_000;

/// The account birthday a "restored with a late birthday" wallet imports at.
fn story_birthday() -> AccountBirthday {
    account::birthday_from_treestate(sync::testing::chain_tree_state(STORY_ANCHOR))
        .expect("the empty-frontier anchor decodes")
}

/// The restore anchor's treestate CARRYING the one Ironwood commitment that
/// mined below the birthday — the state an honest endpoint at 280_999 would
/// serve over this chain, and which `chain_tree_state`'s deliberately-empty
/// frontiers cannot (see the fixture note above). Self-checked through the
/// same upstream reader production uses, the B1-10 discipline.
fn restore_anchor_treestate() -> zcash_client_backend::proto::service::TreeState {
    let ts = zcash_client_backend::proto::service::TreeState {
        network: "test".to_owned(),
        height: STORY_ANCHOR,
        hash: sync::testing::display_order_hex(sync::testing::block_id(STORY_ANCHOR)),
        time: 0,
        sapling_tree: String::new(),
        orchard_tree: String::new(),
        ironwood_tree: sync::testing::frontier_hex(1),
    };
    // The self-check: these bytes must decode to a chain state whose Ironwood
    // frontier really holds one leaf (a wrong encoding fails HERE, loudly —
    // never as a mystery endpoint fault inside the rows below).
    let decoded = ts.to_chain_state().expect("the size-1 anchor decodes");
    assert_eq!(
        decoded.final_ironwood_tree().tree_size(),
        1,
        "the override anchor must carry exactly the receipt's one commitment"
    );
    ts
}

/// GEOMETRY CONSTRAINT (measured, see the fixture note below): every height
/// that declares the receipt's Ironwood commitment — the receipt's own block
/// and everything above it, up to the tip — must sit inside ONE scan batch.
/// `scan_cached_blocks` restarts each batch from the wallet's STORED block
/// metadata at `from − 1`, and that read hands the scanner an Ironwood tree
/// size of 0 for a block whose scan computed 1 (measured:
/// `TreeSizeMismatch { Ironwood, given: 1, computed: 0 }` at the first batch
/// boundary above the receipt — the batch rewinds, the re-anchored re-scan
/// mismatches again, and the pass dies). With the whole declared-size region
/// in one batch the in-memory prior chain carries the receipt, which is
/// exactly the shape the T0-3 rows already scan. The heights above are chosen
/// so the post-rescan pass (floor 280_001, the bundle's activation row) is a
/// single 80-block batch.
///
/// The chain both passes scan: tip 280_080, the receipt at 280_020, and —
/// only for the pass that starts ABOVE the receipt — the size-1 restore
/// anchor. The post-rescan client takes `with_restore_anchor = false`: its
/// first anchor sits below the receipt, where the honest empty state is
/// already correct.
fn receipt_chain(receipt: &IronwoodReceipt, with_restore_anchor: bool) -> sync::testing::FakeChain {
    let mut client = sync::testing::FakeChain::to_tip(STORY_TIP);
    client.ironwood_receipt = Some(receipt.clone());
    if with_restore_anchor {
        client.tree_state_override = Some((STORY_ANCHOR, restore_anchor_treestate()));
    }
    client
}

/// The #357 fingerprint row, read RAW from the aux DB — the row the contract
/// names ("the fingerprint row is the pre-rescan row"), not just its verdict.
fn seed_fingerprint_row(inner: &Inner) -> Option<Vec<u8>> {
    let aux = inner.aux_db.lock().expect("aux db mutex poisoned");
    aux.query_row(
        "SELECT fp FROM wallet_seed_fingerprint WHERE singleton = 1",
        [],
        |r| r.get(0),
    )
    .optional()
    .expect("the fingerprint row reads")
}

// ── The port double: one whose ANSWER can change between syncs ────────────
//
// The shared `StagedSeedPort` answers ONE fixed seed for its whole life, but
// §3.2 row 3 needs "the NEXT sync asks the port again" after the port has
// answered WRONG, and the staging story needs a port with nothing staged at
// the moment of the rescan. This double holds its answer behind a lock so a
// test can flip it in place — built HERE, in the test file, never in
// production code (the FR-30 doubles stay one source of truth for the shapes
// they already cover).

/// What the port answers on the next pull.
enum PortAnswer {
    /// The phrase this wallet was created from.
    Right,
    /// A different wallet's phrase — the #357 mismatch shape.
    Wrong,
    /// Nothing staged (`Unavailable`, which `acquire_seed` collapses to
    /// `SeedRequired` — the no-oracle collapse).
    Nothing,
}

/// A host seed port whose answer can CHANGE between pulls, counting every
/// pull (the confinement witness the FR-12 rows use).
struct FlippableSeedPort {
    right: Zeroizing<Vec<u8>>,
    wrong: Zeroizing<Vec<u8>>,
    answer: Mutex<PortAnswer>,
    pulls: AtomicUsize,
}

impl FlippableSeedPort {
    fn new(right: &[u8]) -> Arc<Self> {
        Arc::new(Self {
            right: Zeroizing::new(right.to_vec()),
            wrong: Zeroizing::new(FR12_WRONG_SEED.to_vec()),
            answer: Mutex::new(PortAnswer::Right),
            pulls: AtomicUsize::new(0),
        })
    }

    /// Change what the next pull answers.
    fn answer(&self, answer: PortAnswer) {
        *self.answer.lock().expect("port answer poisoned") = answer;
    }

    /// How many times the SDK has pulled the seed.
    fn pulls(&self) -> usize {
        self.pulls.load(Ordering::SeqCst)
    }
}

impl WalletSeedPort for FlippableSeedPort {
    fn provide_seed(
        &self,
        _binding: Option<&SpendBinding>,
    ) -> Result<Zeroizing<Vec<u8>>, SeedSupplyError> {
        self.pulls.fetch_add(1, Ordering::SeqCst);
        match *self.answer.lock().expect("port answer poisoned") {
            PortAnswer::Right => Ok(Zeroizing::new(self.right.to_vec())),
            PortAnswer::Wrong => Ok(Zeroizing::new(self.wrong.to_vec())),
            PortAnswer::Nothing => Err(SeedSupplyError::Unavailable),
        }
    }
}

// ── The rows ────────────────────────────────────────────────────────────────

/// FIXTURE CONTROL (green at base, on purpose — see the module doc): the SAME
/// chain, receipt, re-provision and scan driven through a SealedKeychain
/// twin, whose rescan the base tree already allows. It exists so that when
/// the None rows below redden, they redden on the GATE and nothing else:
/// every other link in their story — the size-1 anchor override, the
/// bundle-resolved re-provision, a real scan of a re-provisioned account, the
/// history read — is proven exercisable at the base commit by this row.
#[tokio::test]
async fn the_receipt_rescan_fixture_scans_on_a_sealed_twin() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = Wallet::create_with_vault(
        cfg(dir.path(), Network::Test, SeedPersistence::SealedKeychain),
        raw_seed(),
        Arc::clone(&vault),
    )
    .await
    .expect("create sealed twin");
    w.import_account(story_birthday()).await.expect("import");

    let receipt = ironwood_receipt_for(&w, RECEIPT_H, RECEIPT_ZAT).await;
    let mut client = receipt_chain(&receipt, true);
    w.sync_once(&mut client, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await
        .expect("the birthday-anchored pass scans clean over the receipt chain");
    assert!(
        w.transactions(10, None)
            .await
            .expect("history")
            .rows
            .is_empty(),
        "precondition: the restore's own scan starts at the birthday — the receipt below it is invisible"
    );

    let w = w
        .rescan_from_with_vault(Some(BlockHeight::new(RESCAN_H)), Arc::clone(&vault))
        .await
        .expect("the sealed twin rescans at base");
    let mut client2 = receipt_chain(&receipt, false);
    w.provision_account(&mut client2, w.inner.birthday)
        .await
        .expect("re-provision at the lower birthday");
    w.sync_once(&mut client2, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await
        .expect("the post-rescan pass scans");
    let page = w.transactions(10, None).await.expect("history");
    assert_eq!(page.rows.len(), 1, "the receipt is in history");
    assert_eq!(page.rows[0].net_amount.zat() as u64, RECEIPT_ZAT);
    w.close().await.expect("close");
}

/// §3.2 row 1 — THE ASK. A `SeedPersistence::None` wallet with a registered
/// port and a `rescan_from(h)` below its birthday: the call SUCCEEDS, the
/// rebuild resets the account (the same crash-atomic rename, seals
/// untouched), the rescan itself pulls NOTHING (the seed is asked at the NEXT
/// sync — mechanism (a); a pull here is the ruled-out (b)), and the FIRST
/// sync after it re-imports account 0 through the port — the receipt mined
/// between `h` and the old birthday is in the balance.
#[tokio::test]
async fn a_host_seed_wallet_rescans_from_a_lower_birthday_through_its_port() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let port = FlippableSeedPort::new(FR12_SENTINEL_SEED);
    let w = create_none_then_reopen_with_port(
        dir.path(),
        &vault,
        FR12_SENTINEL_SEED,
        Some(port.clone() as Arc<dyn WalletSeedPort>),
    )
    .await;
    // Restored with a LATE birthday — the account lands through the port.
    w.import_account(story_birthday())
        .await
        .expect("import via port");
    assert_eq!(port.pulls(), 1, "the import pulled the seed exactly once");

    // The wallet syncs at its own birthday: the receipt below stays unseen.
    let receipt = ironwood_receipt_for(&w, RECEIPT_H, RECEIPT_ZAT).await;
    let mut client = receipt_chain(&receipt, true);
    w.sync_once(&mut client, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await
        .expect("the birthday-anchored pass scans clean");
    assert_eq!(
        w.balance().await.expect("balance").total.zat(),
        0,
        "precondition: the funds that arrived before the restore are invisible"
    );

    let pulls_at_rescan = port.pulls();
    let w = w
        .rescan_from_with_vault(Some(BlockHeight::new(RESCAN_H)), Arc::clone(&vault))
        .await
        .expect("a registered port stands in for the seed at rest — the rescan rebuilds");
    assert_eq!(
        port.pulls(),
        pulls_at_rescan,
        "the rescan itself asks the port NOTHING — the seed is needed at the first \
         sync, not held across the rebuild (mechanism (a); a pull here is (b), ruled out)"
    );
    assert!(
        !w.account_exists().await.expect("account_exists"),
        "the rebuild reset the data DB — the old account is gone"
    );
    assert_eq!(
        w.inner.birthday,
        Some(BlockHeight::new(RESCAN_H)),
        "the new birthday is retained for the first sync"
    );

    // The FIRST sync re-imports account 0 at the lower birthday THROUGH the
    // port, then scans down to the receipt.
    let mut client2 = receipt_chain(&receipt, false);
    w.provision_account(&mut client2, w.inner.birthday)
        .await
        .expect("the first sync re-imports through the port");
    assert_eq!(
        port.pulls(),
        pulls_at_rescan + 1,
        "exactly one pull for the re-import — the seed came through the port"
    );
    let pass = w
        .sync_once(&mut client2, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await
        .expect("the post-rescan pass scans");
    assert!(pass.batches > 0, "the pass actually scanned; {pass:?}");
    assert_eq!(
        w.balance().await.expect("balance").total.zat() as u64,
        RECEIPT_ZAT,
        "the receipt between the new birthday and the old one is in the balance"
    );
    w.close().await.expect("close");
}

/// §3.2 row 2 — NO PORT, SAME ANSWER. A `None` wallet with NO port
/// registered: `SeedRequired`, before any teardown — the seals are
/// byte-identical and the wallet re-opens. GREEN AT BASE AND STAYING: the
/// base gate already refuses this wallet (`seed.is_none()`); the item only
/// changes the REASON to "no port registered", and this row pins that the
/// new gate keeps the refusal and keeps it non-destructive.
#[tokio::test]
async fn a_host_seed_wallet_with_no_port_keeps_seed_required_before_any_teardown() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = create_none_then_reopen_with_port(dir.path(), &vault, FR12_SENTINEL_SEED, None).await;
    let dbkey_before =
        std::fs::read(dir.path().join(crate::constants::DBKEY_SEAL_FILE_NAME)).expect("dbkey seal");
    let wrap_before = std::fs::read(dir.path().join(crate::constants::WRAP_ARTIFACT_FILE_NAME))
        .expect("wrap artifact");
    // The fingerprint row is present and verifies before the call.
    {
        let aux = w.inner.aux_db.lock().expect("aux db mutex poisoned");
        assert_eq!(
            crate::seed_fingerprint::verify(&aux, FR12_SENTINEL_SEED),
            crate::seed_fingerprint::Verdict::Match,
            "precondition: the create recorded this wallet's fingerprint"
        );
    }

    match w
        .rescan_from_with_vault(Some(BlockHeight::new(RESCAN_H)), Arc::clone(&vault))
        .await
    {
        Err(WalletError::SeedRequired) => {}
        Err(other) => panic!(
            "expected SeedRequired for a portless None wallet, got {}",
            other.code()
        ),
        Ok(_) => panic!("a portless None-persistence rescan must stay refused"),
    }
    // The refusal consumed the handle without touching the store.
    assert_eq!(
        std::fs::read(dir.path().join(crate::constants::DBKEY_SEAL_FILE_NAME)).expect("dbkey seal"),
        dbkey_before,
        "the refusal is BEFORE any teardown — the seals are pristine"
    );
    assert_eq!(
        std::fs::read(dir.path().join(crate::constants::WRAP_ARTIFACT_FILE_NAME))
            .expect("wrap artifact"),
        wrap_before,
        "the wrap artifact is pristine"
    );
    let w2 = Wallet::open_with_vault(
        cfg(dir.path(), Network::Test, SeedPersistence::None),
        Arc::clone(&vault),
    )
    .await
    .expect("re-open after the refused rescan (the store was untouched)");
    {
        let aux = w2.inner.aux_db.lock().expect("aux db mutex poisoned");
        assert_eq!(
            crate::seed_fingerprint::verify(&aux, FR12_SENTINEL_SEED),
            crate::seed_fingerprint::Verdict::Match,
            "the fingerprint row survived the refusal intact"
        );
    }
    w2.close().await.expect("close");
}

/// §3.2 row 3 — the WRONG seed is refused BEFORE it derives, and the wallet
/// stays usable. After the rebuild, a port answering a different phrase: the
/// #357 fingerprint refuses it (`SeedMismatch`), no account is imported, the
/// wallet reads exactly like a fresh host-seed create before its first sync,
/// and the NEXT sync asks the port again — a corrected port succeeds.
#[tokio::test]
async fn a_wrong_seed_from_the_port_is_refused_and_the_next_sync_asks_again() {
    use crate::provision::testing::FakeOracle;
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let port = FlippableSeedPort::new(FR12_SENTINEL_SEED);
    let w = create_none_then_reopen_with_port(
        dir.path(),
        &vault,
        FR12_SENTINEL_SEED,
        Some(port.clone() as Arc<dyn WalletSeedPort>),
    )
    .await;
    w.import_account(story_birthday()).await.expect("import");
    let pulls_after_import = port.pulls();

    let w = w
        .rescan_from_with_vault(Some(BlockHeight::new(RESCAN_H)), Arc::clone(&vault))
        .await
        .expect("the registered port lets the rebuild proceed");
    assert_eq!(
        port.pulls(),
        pulls_after_import,
        "the rescan asked the port nothing"
    );

    // The host's port now answers a DIFFERENT phrase.
    port.answer(PortAnswer::Wrong);
    let mut oracle = FakeOracle::honest(Network::Test, STORY_TIP);
    match w.provision_account(&mut oracle, w.inner.birthday).await {
        Err(WalletError::SeedMismatch) => {}
        other => panic!(
            "a wrong phrase must be refused by the #357 fingerprint before a single \
             note is derived, got {other:?}"
        ),
    }
    assert_eq!(
        port.pulls(),
        pulls_after_import + 1,
        "the first sync ASKED the port — the refusal is the fingerprint's, not a \
         missing pull"
    );
    assert!(
        !w.account_exists().await.expect("account_exists"),
        "no account was imported from the wrong phrase"
    );
    assert_eq!(
        w.balance().await.expect("balance").total.zat(),
        0,
        "the state is the honest no-account zero"
    );
    assert!(
        w.transactions(10, None)
            .await
            .expect("history")
            .rows
            .is_empty(),
        "no rows — exactly a fresh host-seed create before its first sync"
    );

    // The NEXT sync asks the port again — a corrected port succeeds.
    port.answer(PortAnswer::Right);
    let mut oracle2 = FakeOracle::honest(Network::Test, STORY_TIP);
    w.provision_account(&mut oracle2, w.inner.birthday)
        .await
        .expect("the corrected port re-imports the account");
    assert_eq!(
        port.pulls(),
        pulls_after_import + 2,
        "the next sync asked the port AGAIN — a wrong answer is retryable, never \
         cached as a verdict on the wallet"
    );
    assert!(
        w.account_exists().await.expect("account_exists"),
        "the account landed at the lower birthday"
    );
    w.close().await.expect("close");
}

/// §3.2 row 3's stated PRE-#357 shape, as a row: a wallet whose fingerprint
/// row is ABSENT (the pre-#357 at-rest shape) degrades to the UNCHECKED
/// import after a rescan, exactly as its first sync did — the rescan neither
/// adds nor repairs the guard. The absence is planted by deleting the row
/// from the aux DB, which is the shape a pre-#357 wallet's store holds.
#[tokio::test]
async fn an_absent_fingerprint_degrades_to_the_unchecked_import_after_a_rescan() {
    use crate::provision::testing::FakeOracle;
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let port = FlippableSeedPort::new(FR12_SENTINEL_SEED);
    let w = create_none_then_reopen_with_port(
        dir.path(),
        &vault,
        FR12_SENTINEL_SEED,
        Some(port.clone() as Arc<dyn WalletSeedPort>),
    )
    .await;
    w.import_account(story_birthday()).await.expect("import");
    // Plant the pre-#357 shape: no fingerprint row at rest.
    {
        let aux = w.inner.aux_db.lock().expect("aux db mutex poisoned");
        aux.execute(
            "DELETE FROM wallet_seed_fingerprint WHERE singleton = 1",
            [],
        )
        .expect("delete the fingerprint row");
    }
    assert!(
        seed_fingerprint_row(&w.inner).is_none(),
        "precondition: the wallet now reads Absent, the pre-#357 posture"
    );

    let w = w
        .rescan_from_with_vault(Some(BlockHeight::new(RESCAN_H)), Arc::clone(&vault))
        .await
        .expect("the rescan proceeds — absence is not a refusal");
    port.answer(PortAnswer::Wrong);
    let mut oracle = FakeOracle::honest(Network::Test, STORY_TIP);
    w.provision_account(&mut oracle, w.inner.birthday)
        .await
        .expect("an Absent fingerprint degrades to the unchecked import, as its first sync did");
    assert!(
        w.account_exists().await.expect("account_exists"),
        "the (wrong-seed) account imported unchecked — the documented degradation"
    );
    w.close().await.expect("close");
}

/// §3.2 row 4 — the FINGERPRINT and the IDENTIFIER survive the rebuild. The
/// cross-item row of §2: the fingerprint half is asserted LIVE here (the
/// `wallet_seed_fingerprint` row in the aux DB, before == after the rebuild,
/// and still verifying); the custody-identifier half is asserted WITHOUT a
/// custody symbol — the `wrap.artifact` file's bytes before the rescan ==
/// after (the base tree already pins this for a KILLED rescan in `store.rs`;
/// a byte-equal file carries a byte-equal outer header, which is where §3.1's
/// identifier lives). On the JOINED tree this row is re-read with custody's
/// own accessor.
#[tokio::test]
async fn the_seed_fingerprint_and_the_custody_identifier_survive_a_host_seed_rescan() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let port = FlippableSeedPort::new(FR12_SENTINEL_SEED);
    let w = create_none_then_reopen_with_port(
        dir.path(),
        &vault,
        FR12_SENTINEL_SEED,
        Some(port.clone() as Arc<dyn WalletSeedPort>),
    )
    .await;
    w.import_account(story_birthday()).await.expect("import");
    let fp_before = seed_fingerprint_row(&w.inner);
    assert!(
        fp_before.is_some(),
        "precondition: a None-persistence create records its fingerprint"
    );
    let wrap_before = std::fs::read(dir.path().join(crate::constants::WRAP_ARTIFACT_FILE_NAME))
        .expect("wrap artifact");

    let w = w
        .rescan_from_with_vault(Some(BlockHeight::new(RESCAN_H)), Arc::clone(&vault))
        .await
        .expect("the host-seed rescan proceeds");

    assert_eq!(
        seed_fingerprint_row(&w.inner),
        fp_before,
        "the fingerprint row is the pre-rescan row, byte for byte — the rebuilt \
         wallet's import is exactly as guarded as its first"
    );
    {
        let aux = w.inner.aux_db.lock().expect("aux db mutex poisoned");
        assert_eq!(
            crate::seed_fingerprint::verify(&aux, FR12_SENTINEL_SEED),
            crate::seed_fingerprint::Verdict::Match,
            "the right phrase still verifies after the rebuild"
        );
        assert_eq!(
            crate::seed_fingerprint::verify(&aux, FR12_WRONG_SEED),
            crate::seed_fingerprint::Verdict::Mismatch,
            "a different phrase still mismatches after the rebuild"
        );
    }
    assert_eq!(
        std::fs::read(dir.path().join(crate::constants::WRAP_ARTIFACT_FILE_NAME))
            .expect("wrap artifact"),
        wrap_before,
        "the wrap artifact is byte-identical across the rebuild — its outer header \
         (the custody identifier's home, §3.1) carried across"
    );
    w.close().await.expect("close");
}

/// §3.2 row 5 — the window between the rebuild and the first sync is HONEST.
/// Before the first sync re-imports the account: balance reads zero WITH no
/// account behind it (the same state a fresh host-seed create shows — read
/// side by side below), history is empty, the parked surface reads from the
/// PRESERVED aux tables (a send parked before the rescan is still parked),
/// and a SEND is refused typed (no account) — never "insufficient funds"
/// over money that is merely unscanned.
#[tokio::test]
async fn a_rebuilt_host_seed_wallet_is_honest_before_its_first_sync() {
    // (a) What a FRESH host-seed create shows before its first sync — the
    // "same state" the row names, measured on the base tree's own shape.
    let fresh_dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let fresh = create_none_then_reopen_with_port(
        fresh_dir.path(),
        &vault,
        FR12_SENTINEL_SEED,
        Some(FlippableSeedPort::new(FR12_SENTINEL_SEED) as Arc<dyn WalletSeedPort>),
    )
    .await;
    assert!(
        !fresh.account_exists().await.expect("account_exists"),
        "the fresh control has no account"
    );
    let fresh_snap = fresh.snapshot().await.expect("snapshot");
    assert_eq!(fresh_snap.balance, BalanceSnapshot::default());
    assert_eq!(fresh_snap.tip, None);
    assert_eq!(fresh_snap.last_synced, None);
    assert!(
        fresh
            .transactions(10, None)
            .await
            .expect("history")
            .rows
            .is_empty(),
        "the fresh control's history is empty"
    );
    match fresh.propose(one_payment(Some(100_000))).await {
        Err(WalletError::ProposeFailed) => {}
        other => panic!("the fresh control refuses a send typed (no account), got {other:?}"),
    }
    fresh.close().await.expect("close");

    // (b) A synced None wallet with money state parked on the aux tables.
    let dir = tempfile::tempdir().expect("tempdir");
    let port = FlippableSeedPort::new(FR12_SENTINEL_SEED);
    let w = create_none_then_reopen_with_port(
        dir.path(),
        &vault,
        FR12_SENTINEL_SEED,
        Some(port.clone() as Arc<dyn WalletSeedPort>),
    )
    .await;
    w.import_account(story_birthday()).await.expect("import");
    // Park ONE queued send BEFORE the rescan: after the rebuild the parked
    // surface must still read it — from the preserved aux tables, not from a
    // scan that has not happened yet.
    let parked_id = w
        .queue_send(one_payment(Some(100_000)))
        .await
        .expect("queue")
        .value();
    let mut client = sync::testing::FakeChain::to_tip(STORY_TIP);
    w.sync_once(&mut client, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await
        .expect("the wallet has synced at its birthday");
    // Give the rebuild a stamp and an everSynced flag to clear, so the
    // window's "no stale as-of over a rebuilding balance" is non-vacuous.
    {
        let aux = w.inner.aux_db.lock().expect("aux db mutex poisoned");
        crate::sync_stamp::record(
            &aux,
            BlockHeight::new(u32::try_from(STORY_TIP).expect("tip fits u32")),
            1_751_000_000,
        )
        .expect("record the stamp");
        crate::ever_synced::set_once(&aux, 1_751_000_000).expect("set everSynced");
    }

    // THE WINDOW. No provision, no sync — the state between the rebuild and
    // the first re-import.
    let w = w
        .rescan_from_with_vault(Some(BlockHeight::new(RESCAN_H)), Arc::clone(&vault))
        .await
        .expect("the host-seed rescan rebuilds");
    assert!(
        !w.account_exists().await.expect("account_exists"),
        "no account yet — the honest 'not provisioned' state behind the zero balance"
    );
    let snap = w.snapshot().await.expect("snapshot");
    assert_eq!(
        snap.sync, fresh_snap.sync,
        "the sync status reads the same as a fresh host-seed create — no account \
         behind it, nothing claimed about the chain"
    );
    assert_eq!(
        snap.balance, fresh_snap.balance,
        "balance reads the same honest zero a fresh host-seed create shows"
    );
    assert_eq!(
        snap.tip, None,
        "no chain tip — the rebuilt wallet claims nothing about the chain"
    );
    assert_eq!(
        snap.last_synced, None,
        "the rebuild cleared the stamp — no stale 'as of block H' over a \
         rebuilding balance"
    );
    assert!(
        !snap.ever_synced,
        "the catch-up cue re-shows during the rebuild"
    );
    assert!(
        snap.rescan_rebuilding,
        "the rebuilding breadcrumb is SET — the one deliberate difference from a \
         fresh create, and the honest answer to WHY the balance is zero"
    );
    assert!(
        w.transactions(10, None)
            .await
            .expect("history")
            .rows
            .is_empty(),
        "history is empty — the old scan is gone with the data DB"
    );
    let parked = w.list_parked_sends().await.expect("parked surface");
    assert_eq!(
        parked.len(),
        1,
        "the parked surface reads the PRESERVED aux tables — the queued send \
         survived the rebuild"
    );
    assert_eq!(parked[0].id, parked_id, "it is the same row");
    assert!(
        w.list_in_flight_sends()
            .await
            .expect("in flight")
            .is_empty(),
        "nothing is in flight (the pre-rescan fence guarantees that shape)"
    );
    // A send is refused TYPED (no account) — never "insufficient funds" over
    // money that is merely unscanned.
    match w.propose(one_payment(Some(100_000))).await {
        Err(WalletError::ProposeFailed) => {}
        Err(WalletError::InsufficientFunds { .. }) => panic!(
            "an 'insufficient funds' refusal over unscanned money is the dishonest \
             window this row exists to catch"
        ),
        other => panic!("expected the typed no-account send refusal, got {other:?}"),
    }
    w.close().await.expect("close");
}

/// §3.2 row 6 — the deeper history is REAL. A wallet created at height B with
/// a receipt at B−k: before the rescan, balance 0 and no row; after
/// `rescan_from(B−k−1)` and ONE sync (the re-import through the port plus
/// the scan), the receipt's row and amount. The sealed-twin control above
/// proves every link of this fixture green at base; this row is the same
/// story on the host-seed wallet, red at base on the gate alone.
#[tokio::test]
async fn a_receipt_before_the_restore_appears_after_the_rescan_and_one_sync() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let port = FlippableSeedPort::new(FR12_SENTINEL_SEED);
    let w = create_none_then_reopen_with_port(
        dir.path(),
        &vault,
        FR12_SENTINEL_SEED,
        Some(port.clone() as Arc<dyn WalletSeedPort>),
    )
    .await;
    w.import_account(story_birthday()).await.expect("import");

    // BEFORE: synced at the restore birthday, the receipt below it is
    // invisible — balance 0 and no row.
    let receipt = ironwood_receipt_for(&w, RECEIPT_H, RECEIPT_ZAT).await;
    let mut client = receipt_chain(&receipt, true);
    w.sync_once(&mut client, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await
        .expect("the birthday-anchored pass scans clean");
    assert_eq!(
        w.balance().await.expect("balance").total.zat(),
        0,
        "before the rescan the balance is zero — the funds are unscanned, not gone"
    );
    assert!(
        w.transactions(10, None)
            .await
            .expect("history")
            .rows
            .is_empty(),
        "before the rescan there is no row"
    );

    // AFTER: `rescan_from(B−k−1)` and ONE sync — the first sync re-imports
    // through the port and the scan reaches down to the receipt.
    let w = w
        .rescan_from_with_vault(Some(BlockHeight::new(RESCAN_H)), Arc::clone(&vault))
        .await
        .expect("the host-seed rescan rebuilds at the lower birthday");
    let mut client2 = receipt_chain(&receipt, false);
    w.provision_account(&mut client2, w.inner.birthday)
        .await
        .expect("the one sync re-imports account 0 through the port");
    w.sync_once(&mut client2, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await
        .expect("the one sync scans to the tip");
    let page = w.transactions(10, None).await.expect("history");
    assert_eq!(page.rows.len(), 1, "exactly the receipt's row");
    assert_eq!(
        page.rows[0].net_amount.zat() as u64,
        RECEIPT_ZAT,
        "the receipt's exact amount"
    );
    assert_eq!(
        page.rows[0].mined_height.map(|h| h.value()),
        Some(u32::try_from(RECEIPT_H).expect("the receipt height fits u32")),
        "mined at the receipt's own height below the old birthday"
    );
    assert_eq!(
        w.balance().await.expect("balance").total.zat() as u64,
        RECEIPT_ZAT,
        "and it is in the balance"
    );
    w.close().await.expect("close");
}

/// UNLISTED CASE (IT-1 +A) — the contract's rows name a port that is
/// ABSENT (row 2: refused) and a port that answers WRONG (row 3: mismatch),
/// but never a port that is REGISTERED and has NOTHING STAGED at the moment
/// of the rescan — Relim's exact staging window (the priced "lost" of
/// mechanism (a): a port that answers only inside a host window must be
/// staged before the first sync). The gate asks "is a port REGISTERED", not
/// "will it answer": the rescan itself succeeds with nothing staged, no pull
/// happens, and the first sync's `SeedRequired` (the no-oracle collapse of
/// `Unavailable`) leaves the wallet in the honest window — retryable, and a
/// later staged seed completes the re-import on the NEXT sync.
#[tokio::test]
async fn a_rescan_succeeds_with_a_registered_port_that_has_nothing_staged_yet() {
    use crate::provision::testing::FakeOracle;
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let port = FlippableSeedPort::new(FR12_SENTINEL_SEED);
    let w = create_none_then_reopen_with_port(
        dir.path(),
        &vault,
        FR12_SENTINEL_SEED,
        Some(port.clone() as Arc<dyn WalletSeedPort>),
    )
    .await;
    w.import_account(story_birthday()).await.expect("import");
    let pulls_after_import = port.pulls();

    // The host's staging window closes BEFORE the rescan: the port stays
    // registered but has nothing to give.
    port.answer(PortAnswer::Nothing);
    let w = w
        .rescan_from_with_vault(Some(BlockHeight::new(RESCAN_H)), Arc::clone(&vault))
        .await
        .expect("registration is what the gate asks — an unstaged port does not refuse the rescan");
    assert_eq!(
        port.pulls(),
        pulls_after_import,
        "the rescan pulled nothing — it never needed an answer, only the registration"
    );

    // The first sync honestly refuses: nothing to import from, no account.
    let mut oracle = FakeOracle::honest(Network::Test, STORY_TIP);
    match w.provision_account(&mut oracle, w.inner.birthday).await {
        Err(WalletError::SeedRequired) => {}
        other => panic!(
            "an unstaged port surfaces SeedRequired at the sync (the no-oracle \
             collapse), got {other:?}"
        ),
    }
    assert!(
        !w.account_exists().await.expect("account_exists"),
        "the wallet sits in the honest window — no account, nothing derived"
    );

    // The host stages the seed; the NEXT sync completes the re-import.
    port.answer(PortAnswer::Right);
    let mut oracle2 = FakeOracle::honest(Network::Test, STORY_TIP);
    w.provision_account(&mut oracle2, w.inner.birthday)
        .await
        .expect("a later staged seed completes the re-import");
    assert!(
        w.account_exists().await.expect("account_exists"),
        "the wait was retryable, not a verdict"
    );
    w.close().await.expect("close");
}
