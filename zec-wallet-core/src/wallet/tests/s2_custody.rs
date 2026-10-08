//! Stage S2 `custody` (R04 + FR-38's convergence half) — the test author's
//! rows, written BLIND (IT-2a) against `b2d30176`, contract
//! `docs/plan/stage-2-the-hosts-lifecycle.md` §3.1. A child of
//! `wallet::tests` so it uses that module's fixtures (`cfg`, `raw_seed`,
//! `shared_mk`, `keychain_namespace_for`) without adding to it — the
//! `private_path_truth` / `s8_identity` shape, so no cited line in
//! `wallet.rs` moves.
//!
//! TWO commits, by the split's rule:
//!
//! 1. THIS file's first commit — the rows that compile against the BASE tree
//!    alone and run there. Rows 2, 6 and 7's core half are expressible with
//!    today's seams (`open_with_vault`, `wipe_with_vault`, the path-derived
//!    namespace); row 2's locator pairing is RED at base (the artifact is
//!    inert bytes under the shared-keychain double, so the wrong wallet's
//!    artifact opens silently — the exact defect the contract retires), rows
//!    6 and 7 are the convergence/reach pins the contract must NOT regress
//!    when the namespace stops being path-derived. The adjudication's
//!    repair (ruling §1.1) later moved row 2's two locator-pairing opens to
//!    the per-namespace resolver — through a pre-built vault the locator
//!    selects nothing, so no conforming build could refuse; the same edit
//!    class the fold performs on the R04 probe. Those two opens now name
//!    the seam with the declared-name rows; the row's claim does not move.
//! 2. The declared-name commit — the rows that name the resolver seam and
//!    `CustodyId` (migration, crash windows, the full tamper row, the
//!    never-logged compile-time pin). They cannot compile at base; the
//!    adjudicator maps the names at the join.
//!
//! The R04 probe (`a_relocated_wallet_directory_keeps_its_custody_identity`,
//! in `wallet.rs`) is the orchestrator's — untouched here, and NOT duplicated
//! under another name.

use super::*;

// ───────────────────────── the wedged-keystore vault double ─────────────────────────

/// A `KeychainPort` wrapper over a real vault double whose EVERY purge
/// refuses — the wedged keystore of row 8 — while every other call delegates
/// unchanged. (The migration fault-injection wrapper lands with the
/// declared-name commit, where its fault slots have users.)
struct WedgedPurgeVault {
    inner: Arc<dyn KeychainPort>,
}

impl KeychainPort for WedgedPurgeVault {
    fn probe(&self) -> Result<(), WalletError> {
        self.inner.probe()
    }

    fn tier(&self) -> Result<VaultTier, WalletError> {
        self.inner.tier()
    }

    fn store_wrap_key(
        &self,
        key: crate::seal::SealKey,
        sealed_blob: &[u8],
    ) -> Result<crate::keychain::WrapArtifact, WalletError> {
        self.inner.store_wrap_key(key, sealed_blob)
    }

    fn load_wrap_key(
        &self,
        artifact: &crate::keychain::WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<crate::seal::SealKey, WalletError> {
        self.inner.load_wrap_key(artifact, sealed_blob)
    }

    fn rotate_wrap_key(
        &self,
        artifact: &crate::keychain::WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<crate::keychain::WrapArtifact, WalletError> {
        self.inner.rotate_wrap_key(artifact, sealed_blob)
    }

    fn finish_rotation(
        &self,
        old: &crate::keychain::WrapArtifact,
        new: &crate::keychain::WrapArtifact,
    ) -> Result<(), WalletError> {
        self.inner.finish_rotation(old, new)
    }

    fn delete_wrap_key(&self, artifact: &crate::keychain::WrapArtifact) -> Result<(), WalletError> {
        self.inner.delete_wrap_key(artifact)
    }

    fn purge_namespace(&self) -> Result<usize, WalletError> {
        Err(WalletError::KeystoreUnavailable)
    }

    // Join: the trait grew the three S2 index methods — every call but the
    // wedged PURGE delegates unchanged (the declared policy of this double).
    fn store_index(&self, entry: &crate::custody::CustodyIndexEntry) -> Result<(), WalletError> {
        self.inner.store_index(entry)
    }

    fn load_index(&self) -> Result<Option<crate::custody::CustodyIndexEntry>, WalletError> {
        self.inner.load_index()
    }

    fn delete_index(&self) -> Result<(), WalletError> {
        self.inner.delete_index()
    }
}

// ───────────────────────── row 2 — two containers, two wallets ─────────────────────────

/// Which typed refusals the custody layer may answer a cross-wallet pairing
/// with (contract §3.1 row 3 names both: `WrapArtifactInvalid` where the wrap
/// key does not authenticate the artifact, the `KeystoreInconsistent`
/// keysMissing class where the locator names a namespace with no key).
fn assert_custody_refusal(result: &Result<Wallet, WalletError>, what: &str) {
    match result {
        Err(WalletError::WrapArtifactInvalid) | Err(WalletError::KeystoreInconsistent { .. }) => {}
        // `Wallet` is pinned non-Debug, so print only the error side.
        Ok(_) => panic!(
            "{what}: the open must be refused typed, never a silent open of \
             the wrong wallet; it OPENED"
        ),
        Err(other) => panic!(
            "{what}: the open must be refused typed as a custody error \
             (WrapArtifactInvalid / KeystoreInconsistent); got {other:?}"
        ),
    }
}

/// §3.1 row 2 (NEW — red first). Two wallets created in two directories are
/// two custody identities, and NO cross-pairing of one wallet's files with
/// the other's opens: the other wallet's wrap artifact (the LOCATOR file —
/// post-stage it names the other wallet's namespace, so its wrap key must
/// fail the AEAD over THIS wallet's sealed blob), the other wallet's seals
/// (its blob hash is not the one this namespace's custody item was bound
/// to), the other wallet's `wallet.db` (encrypted under the other wallet's
/// DB key — fails at the SQLCipher key, already bound by encryption).
///
/// The artifact pairing is the RED half at base: the shared-keychain
/// double's artifact is a 1-byte marker that carries nothing, so swapping it
/// is a no-op and the base tree opens the wrong wallet's artifact SILENTLY —
/// the defect the locator retires (the red record is the first commit's,
/// which opened the pairing through a pre-built vault; the adjudication's
/// repair, ruling §1.1, moved the two pairing opens to the per-namespace
/// resolver — the row's own second-commit shape — because through a
/// pre-built vault the locator selects nothing). The seals/DB pairings
/// already refuse at base (the AAD blob-binding) and pin that the locator
/// must not WEAKEN them. `# provenance: contract §3.1 row 2`.
#[tokio::test]
async fn two_containers_are_two_wallets_and_neither_opens_under_the_others_files() {
    let parent = tempfile::tempdir().expect("parent dir");
    let a = parent.path().join("container-a/wallet");
    let b = parent.path().join("container-b/wallet");
    std::fs::create_dir_all(&a).expect("container a");
    std::fs::create_dir_all(&b).expect("container b");
    let keychain = SharedKeychainVault::shared();
    let ns_a = keychain_namespace_for(&a);
    let ns_b = keychain_namespace_for(&b);
    let mk = |ns: &KeychainNamespace| shared_mk(&keychain, ns.as_str());

    let wa = Wallet::create_with_vault(
        cfg(&a, Network::Test, SeedPersistence::SealedKeychain),
        raw_seed(),
        mk(&ns_a),
    )
    .await
    .expect("create wallet a");
    wa.close().await.expect("close a");
    let wb = Wallet::create_with_vault(
        cfg(&b, Network::Test, SeedPersistence::SealedKeychain),
        raw_seed(),
        mk(&ns_b),
    )
    .await
    .expect("create wallet b");
    wb.close().await.expect("close b");

    // Two containers minted two custody identities: the shared keychain
    // holds TWO distinct namespaces, one per wallet.
    {
        let namespaces: Vec<String> = keychain.lock().expect("kc").keys().cloned().collect();
        assert_eq!(namespaces.len(), 2, "two wallets, two namespaces");
        assert_ne!(namespaces[0], namespaces[1]);
        assert!(namespaces.contains(&ns_a.as_str().to_owned()));
        assert!(namespaces.contains(&ns_b.as_str().to_owned()));
    }

    // Snapshot A's own files so every pairing restores before the next.
    let artifact_a =
        std::fs::read(a.join(crate::constants::WRAP_ARTIFACT_FILE_NAME)).expect("a artifact");
    let artifact_b =
        std::fs::read(b.join(crate::constants::WRAP_ARTIFACT_FILE_NAME)).expect("b artifact");
    let seed_seal_a =
        std::fs::read(a.join(crate::constants::SEED_SEAL_FILE_NAME)).expect("a seed seal");
    let dbkey_seal_a =
        std::fs::read(a.join(crate::constants::DBKEY_SEAL_FILE_NAME)).expect("a dbkey seal");
    let db_a = std::fs::read(a.join(crate::constants::WALLET_DB_FILE_NAME)).expect("a wallet.db");
    let restore_a = || {
        std::fs::write(
            a.join(crate::constants::WRAP_ARTIFACT_FILE_NAME),
            &artifact_a,
        )
        .expect("restore artifact");
        std::fs::write(a.join(crate::constants::SEED_SEAL_FILE_NAME), &seed_seal_a)
            .expect("restore seed seal");
        std::fs::write(
            a.join(crate::constants::DBKEY_SEAL_FILE_NAME),
            &dbkey_seal_a,
        )
        .expect("restore dbkey seal");
        std::fs::write(a.join(crate::constants::WALLET_DB_FILE_NAME), &db_a)
            .expect("restore wallet.db");
    };

    // Pairing 1 — the other wallet's SEALS under this container's artifact:
    // the custody item's blob-hash binding refuses (already load-bearing at
    // base; the locator must not weaken it).
    let seed_seal_b =
        std::fs::read(b.join(crate::constants::SEED_SEAL_FILE_NAME)).expect("b seed seal");
    let dbkey_seal_b =
        std::fs::read(b.join(crate::constants::DBKEY_SEAL_FILE_NAME)).expect("b dbkey seal");
    std::fs::write(a.join(crate::constants::SEED_SEAL_FILE_NAME), &seed_seal_b)
        .expect("plant b seed seal");
    std::fs::write(
        a.join(crate::constants::DBKEY_SEAL_FILE_NAME),
        &dbkey_seal_b,
    )
    .expect("plant b dbkey seal");
    let opened = Wallet::open_with_vault(
        cfg(&a, Network::Test, SeedPersistence::SealedKeychain),
        mk(&ns_a),
    )
    .await;
    assert_custody_refusal(&opened, "a's artifact under b's seals");
    if let Ok(w) = opened {
        w.close().await.expect("close refused-pairing control");
    }
    restore_a();

    // Pairing 2 — the other wallet's wallet.db under this container's seals:
    // custody unwraps (these ARE a's seals), the SQLCipher key then fails on
    // b's DB — "already bound by encryption" (contract row 3's closing
    // clause). Any typed refusal; a silent open of the wrong wallet is the
    // failure this row exists to make impossible.
    let db_b = std::fs::read(b.join(crate::constants::WALLET_DB_FILE_NAME)).expect("b wallet.db");
    std::fs::write(a.join(crate::constants::WALLET_DB_FILE_NAME), &db_b)
        .expect("plant b wallet.db");
    let opened = Wallet::open_with_vault(
        cfg(&a, Network::Test, SeedPersistence::SealedKeychain),
        mk(&ns_a),
    )
    .await;
    assert!(
        opened.is_err(),
        "a's seals under b's wallet.db must be refused typed, never a silent \
         open of the wrong wallet; got {:?}",
        opened.map(|_| "()")
    );
    restore_a();

    // Pairing 3 — the OTHER wallet's wrap artifact under this container.
    // Post-stage the artifact is the LOCATOR: it names wallet b's namespace,
    // whose wrap key was bound to b's sealed blob, so it must fail the AEAD
    // over a's blobs — `WrapArtifactInvalid`, loud, never a wrong open. The
    // open resolves PER NAMESPACE (ruling §1.1's repair): a pre-built vault
    // hands every namespace this wallet's item, so through `mk` the locator
    // selects nothing and the wrong wallet opens silently — a refusal no
    // conforming build could deliver; the honest resolver makes the
    // locator's own namespace the vault that answers.
    std::fs::write(
        a.join(crate::constants::WRAP_ARTIFACT_FILE_NAME),
        &artifact_b,
    )
    .expect("plant b's artifact");
    let opened = Wallet::open_resolving(
        cfg(&a, Network::Test, SeedPersistence::SealedKeychain),
        fn_resolver(recording_resolver(
            keychain.clone(),
            Arc::new(Mutex::new(Vec::new())),
            ns_a.as_str(),
            None,
        )),
        None,
        None,
    )
    .await;
    assert_custody_refusal(&opened, "a's files under b's wrap artifact");
    if let Ok(w) = opened {
        w.close().await.expect("close refused-pairing control");
    }
    // …and the same pairing the other way round ("neither opens under the
    // other's files" is symmetric).
    std::fs::write(
        b.join(crate::constants::WRAP_ARTIFACT_FILE_NAME),
        &artifact_a,
    )
    .expect("plant a's artifact");
    let opened = Wallet::open_resolving(
        cfg(&b, Network::Test, SeedPersistence::SealedKeychain),
        fn_resolver(recording_resolver(
            keychain.clone(),
            Arc::new(Mutex::new(Vec::new())),
            ns_b.as_str(),
            None,
        )),
        None,
        None,
    )
    .await;
    assert_custody_refusal(&opened, "b's files under a's wrap artifact");
    if let Ok(w) = opened {
        w.close().await.expect("close refused-pairing control");
    }
    restore_a();
    std::fs::write(
        b.join(crate::constants::WRAP_ARTIFACT_FILE_NAME),
        &artifact_b,
    )
    .expect("restore b artifact");

    // Positive control — every refusal above was the pairing's, not a broken
    // fixture: A under its OWN files opens.
    let w = Wallet::open_with_vault(
        cfg(&a, Network::Test, SeedPersistence::SealedKeychain),
        mk(&ns_a),
    )
    .await
    .expect("positive control: a under its own files opens");
    w.close().await.expect("close control");
}

// ───────────────────────── row 6 — the wipe converges from a bare config ─────────────────────────

/// §3.1 row 6 (NEW). The host's own native wipe deletes the wallet directory
/// FIRST (Relim's phase 2.5), then calls our wipe with a bare config and no
/// files under it: the wipe still finds and purges the namespace, converges,
/// and a second call is a no-op success.
///
/// At BASE this converges only because the namespace is path-derived (§3.0:
/// "today that works only because the namespace is derivable from the path
/// alone") — so this row runs GREEN at base and is the PIN the stage must not
/// regress when the namespace becomes identifier-derived and the resolution
/// has to go through the index instead. The index-resolving half (and the
/// both-namespaces purge under a pending migration) is the declared-name
/// commit's. `# provenance: contract §3.1 row 6`.
#[tokio::test]
async fn a_wipe_converges_from_a_bare_config_after_the_host_deleted_the_files() {
    let keychain = SharedKeychainVault::shared();
    let parent = tempfile::tempdir().expect("parent dir");
    let dir = parent.path().join("container-a/wallet");
    std::fs::create_dir_all(&dir).expect("container dir");
    let ns = keychain_namespace_for(&dir);

    let w = Wallet::create_with_vault(
        cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
        raw_seed(),
        shared_mk(&keychain, ns.as_str()),
    )
    .await
    .expect("create");
    w.close().await.expect("close");
    assert_eq!(keychain.lock().expect("kc").len(), 1);

    // The host's native wipe removed the whole directory before our wipe ran.
    std::fs::remove_dir_all(&dir).expect("host deleted the files");
    assert!(!dir.exists());

    Wallet::wipe_with_vault(&dir, Some(shared_mk(&keychain, ns.as_str())), false)
        .await
        .expect("wipe converges from a bare config with no files under it");
    assert_eq!(
        keychain.lock().expect("kc").len(),
        0,
        "nothing of this wallet left in the keychain"
    );
    assert!(!dir.exists(), "the wipe leaves no resurrected directory");

    // The re-run (a crash or retry after the first pass) is a no-op success.
    Wallet::wipe_with_vault(&dir, Some(shared_mk(&keychain, ns.as_str())), false)
        .await
        .expect("a second wipe is a no-op success");
    assert_eq!(keychain.lock().expect("kc").len(), 0);
}

// ───────────────────────── row 7 — the wipe's reach ─────────────────────────

/// §3.1 row 7 (NEW), the CORE half. A wipe of one wallet reaches nothing
/// outside that wallet: a second wallet in the same process — its files, its
/// keychain namespace, its OPEN handle — is untouched. The seed-port and
/// dialer registrations and the device-log level are BRIDGE-side process
/// state the core wipe structurally cannot see (no reference exists from
/// `store::destroy` to them); that half is reported as a contract finding,
/// not papered over here.
///
/// Runs green at base (FR-13's namespace isolation already holds it) — the
/// pin the stage must not regress when one wipe must purge TWO namespaces
/// (the identifier's and, while pending, the legacy one) and still touch
/// neither of the neighbour's. `# provenance: contract §3.1 row 7`.
#[tokio::test]
async fn a_wipe_reaches_nothing_outside_its_wallet() {
    let keychain = SharedKeychainVault::shared();
    let parent = tempfile::tempdir().expect("parent dir");
    let a = parent.path().join("wallet-a");
    let b = parent.path().join("wallet-b");
    std::fs::create_dir_all(&a).expect("dir a");
    std::fs::create_dir_all(&b).expect("dir b");
    let ns_a = keychain_namespace_for(&a);
    let ns_b = keychain_namespace_for(&b);

    let wa = Wallet::create_with_vault(
        cfg(&a, Network::Test, SeedPersistence::SealedKeychain),
        raw_seed(),
        shared_mk(&keychain, ns_a.as_str()),
    )
    .await
    .expect("create a");
    wa.close().await.expect("close a");
    // B stays OPEN through A's wipe — its handle must keep answering.
    let wb = Wallet::create_with_vault(
        cfg(&b, Network::Test, SeedPersistence::SealedKeychain),
        raw_seed(),
        shared_mk(&keychain, ns_b.as_str()),
    )
    .await
    .expect("create b");

    Wallet::wipe_with_vault(&a, Some(shared_mk(&keychain, ns_a.as_str())), false)
        .await
        .expect("wipe a");

    assert!(
        !a.join(crate::constants::WALLET_DB_FILE_NAME).exists(),
        "a's files are gone"
    );
    assert_eq!(
        keychain.lock().expect("kc").len(),
        1,
        "only b's namespace remains — the wipe reached no other custody"
    );
    assert!(
        keychain.lock().expect("kc").contains_key(ns_b.as_str()),
        "b's namespace survived a's wipe"
    );
    assert!(
        b.join(crate::constants::WALLET_DB_FILE_NAME).exists(),
        "b's files are untouched"
    );
    // The open handle still answers.
    wb.current_address()
        .await
        .expect("b's open handle still answers after a's wipe");
    wb.close().await.expect("close b");

    // And b reopens from its untouched files.
    let w = Wallet::open_with_vault(
        cfg(&b, Network::Test, SeedPersistence::SealedKeychain),
        shared_mk(&keychain, ns_b.as_str()),
    )
    .await
    .expect("b reopens untouched");
    w.close().await.expect("close reopened b");
}

// ───────────────────────── row 8 — a wedged keystore ─────────────────────────

/// §3.1 row 8 (NEW), the half that exists today. A keystore whose purge
/// cannot complete (wedged — the host's Dart `.timeout` cannot cancel a
/// native call) makes the wipe return that failure TYPED with the wallet's
/// files and custody preserved (keychain-first: no partial shred, the re-run
/// re-purges), and the re-run converges once the keystore answers.
///
/// THE BOUND ITSELF IS NOT ASSERTED HERE — honestly: "the SDK's existing
/// per-call bound" does not exist anywhere in the tree at the base commit
/// (no such constant in `keychain/` or the bridge), so there is no number to
/// assert against; the wall clock of the wedged refusal is PRINTED, not
/// asserted. Reported as a contract finding — the builder names the constant
/// and the adjudicator arms the ceiling against it. Runs green at base (typed
/// propagation is already the behavior); the pin the bound must not replace.
/// `# provenance: contract §3.1 row 8`.
#[tokio::test]
async fn a_wedged_keystore_returns_typed_within_the_sdk_bound() {
    let keychain = SharedKeychainVault::shared();
    let parent = tempfile::tempdir().expect("parent dir");
    let dir = parent.path().join("wallet");
    std::fs::create_dir_all(&dir).expect("dir");
    let ns = keychain_namespace_for(&dir);

    let w = Wallet::create_with_vault(
        cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
        raw_seed(),
        shared_mk(&keychain, ns.as_str()),
    )
    .await
    .expect("create");
    w.close().await.expect("close");

    // The wedged keystore: every purge call refuses, nothing else changes.
    let wedged: Arc<dyn KeychainPort> = Arc::new(WedgedPurgeVault {
        inner: shared_mk(&keychain, ns.as_str()),
    });
    let started = std::time::Instant::now();
    match Wallet::wipe_with_vault(&dir, Some(wedged), false).await {
        Err(WalletError::KeystoreUnavailable) => {}
        other => panic!("a wedged keystore must return its failure typed; got {other:?}"),
    }
    println!(
        "wedged-keystore wipe refused after {:?} (the SDK per-call bound \
         does not exist at base — printed, not asserted; see the test doc)",
        started.elapsed()
    );
    assert!(
        dir.join(crate::constants::WALLET_DB_FILE_NAME).exists(),
        "files preserved — no partial shred under a wedged keystore"
    );
    assert_eq!(
        keychain.lock().expect("kc").len(),
        1,
        "custody still live — nothing was silently severed"
    );

    // The re-run, keystore answering, converges.
    Wallet::wipe_with_vault(&dir, Some(shared_mk(&keychain, ns.as_str())), false)
        .await
        .expect("the re-run converges once the keystore answers");
    assert_eq!(keychain.lock().expect("kc").len(), 0);
    assert!(!dir.exists());
}
// ───────────── the declared-name rows (the second commit) ─────────────
//
// COUPLING DECLARED FOR THE JOIN (the `s8_identity` pattern): every symbol
// this half names that the base tree does not carry, with the shape the rows
// assume. The contract fixes the TYPE names (`CustodyId` / `custody_id`); the
// five shapes below are this author's spelling of the contract's words, and
// the adjudicator maps them at the join:
//
//   * `Wallet::create_with_resolver(cfg, seed, resolver)` /
//     `Wallet::open_with_resolver(cfg, resolver)` — §3.1's "the SDK's own
//     resolver": a namespace → vault factory,
//     `Arc<dyn Fn(&KeychainNamespace) -> Arc<dyn KeychainPort> + Send +
//     Sync>`, taking the place of `*_with_vault`'s PRE-BUILT vault (the
//     wallet reads the header FIRST, then constructs the vault for the
//     namespace the header names — the factory is where a test observes and
//     faults each namespace).
//   * `crate::keychain::CustodyId` — `Clone + PartialEq + Eq`, NO `Debug`
//     assumed anywhere below (the contract pins no `Display`; a blind row
//     must not require `Debug` either). `CustodyId::random()` mints one
//     (row 3's "to a random id").
//   * `WrapArtifact::custody_id() -> Option<CustodyId>` — read the header
//     locator (`None` = a pre-stage artifact).
//   * `WrapArtifact::with_custody_id(Option<CustodyId>) -> WrapArtifact` —
//     the same backend bytes under an edited locator: the tamper row's
//     instrument, and (with `None`) how this half BUILDS a pre-stage wallet.
//   * `WrapArtifact::to_bytes() -> Vec<u8>` — the serialized file form that
//     round-trips through the store's one door `from_bytes` (the base
//     `as_bytes` may become the inner form once an outer frame exists).
//
// JOIN MAPPING APPLIED (the adjudicator): the implementer's committed names
// are `Wallet::create_resolving` / `Wallet::open_resolving(cfg, resolver,
// None, None)`, `crate::custody::CustodyId::generate()`,
// `WrapArtifact::file_bytes()` / `with_custody_id(&CustodyId)`, and the
// `VaultResolver` TRAIT (not a bare closure) — mapped mechanically below:
// `FnResolver` adapts this half's closure factory onto the trait,
// `from_freshly_wrapped(as_bytes())` is the strip-locator (`None`) edit, and
// the two vault doubles forward the three index methods the trait grew.
//
// Two fixture couplings, both inside the test double this module already
// uses: the `SharedKeychain` map is keyed by the namespace string (the
// base `HashMap<String, ([u8; 32], [u8; 32])>` shape), and `store.rs`'s
// atomic write stages its temp file as `<name>.tmp` beside the final name.

use std::path::{Path, PathBuf};

use crate::keychain::testvault::SharedKeychain;

/// Read the wallet's wrap artifact — the LOCATOR file, the one file read
/// before any key — through the store's single door (`from_bytes`), so the
/// rows stay frame-agnostic whichever outer header the implementation adds.
fn read_locator(dir: &Path) -> crate::keychain::WrapArtifact {
    let bytes = std::fs::read(dir.join(crate::constants::WRAP_ARTIFACT_FILE_NAME))
        .expect("wrap.artifact present");
    crate::keychain::WrapArtifact::from_bytes(bytes).expect("wrap artifact parses")
}

/// Write the wrap artifact back (an edited locator), as the store's commit
/// would — the final name, not a temp.
fn write_locator(dir: &Path, artifact: &crate::keychain::WrapArtifact) {
    std::fs::write(
        dir.join(crate::constants::WRAP_ARTIFACT_FILE_NAME),
        artifact.file_bytes(),
    )
    .expect("write wrap.artifact");
}

/// The distinct namespaces a run asked the resolver for, in first-seen order
/// — the one observable of "which namespace did the wallet take its key
/// from" a blind test has.
fn distinct_ns(seen: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for ns in seen.lock().expect("seen").iter() {
        if !out.contains(ns) {
            out.push(ns.clone());
        }
    }
    out
}

/// The declared closure-factory shape this half's rows were authored against
/// (the JOIN MAPPING note above) — aliased because clippy's type-complexity
/// gate prices the full spelling per site.
type VaultFactory = Arc<dyn Fn(&KeychainNamespace) -> Arc<dyn KeychainPort> + Send + Sync>;

/// Join adapter (see the JOIN MAPPING note above): the SDK's
/// `VaultResolver` trait over this half's declared closure factory — every
/// namespace resolves by calling the closure, exactly the shape the rows
/// were authored against.
struct FnResolver(VaultFactory);

impl VaultResolver for FnResolver {
    fn vault_for<'a>(
        &'a self,
        namespace: &KeychainNamespace,
    ) -> Result<crate::keychain::ResolvedVault<'a>, WalletError> {
        let make: &dyn Fn(&KeychainNamespace) -> Arc<dyn KeychainPort> = self.0.as_ref();
        Ok(crate::keychain::ResolvedVault::Owned(make(namespace)))
    }
}

/// Lift a declared closure factory onto the SDK's resolver trait (the join's
/// name mapping — one call site per seam use).
fn fn_resolver(f: VaultFactory) -> Arc<dyn VaultResolver> {
    Arc::new(FnResolver(f))
}

/// The one resolver this half resolves through: every namespace the wallet
/// asks for is RECORDED, and the fault (when present) wraps either the
/// legacy (path) namespace's vault or every other namespace's — the id
/// namespace, whichever it turns out to be. The faults are the tripwires:
/// the store refusal lands a migration after the index write (row 5's first
/// window), the load refusal is row 4's "never consults the legacy
/// namespace", the purge recorder is "purged exactly once" (rows 4–5).
fn recording_resolver(
    keychain: SharedKeychain,
    seen: Arc<Mutex<Vec<String>>>,
    path_ns: &str,
    fault: Option<Fault>,
) -> VaultFactory {
    let path_ns = path_ns.to_owned();
    Arc::new(move |ns: &KeychainNamespace| {
        seen.lock().expect("seen").push(ns.as_str().to_owned());
        let vault: Arc<dyn KeychainPort> = shared_mk(&keychain, ns.as_str());
        match fault {
            Some(ref f) if f.on_path == (ns.as_str() == path_ns) => Arc::new(FaultyVault {
                inner: vault,
                refuse_store: f.refuse_store,
                refuse_load: f.refuse_load,
                purges: f.purges.clone(),
            }),
            _ => vault,
        }
    })
}

/// Which vault the fault wraps and what it does (see `recording_resolver`).
struct Fault {
    /// Wrap the legacy (path) namespace's vault, else every other's.
    on_path: bool,
    /// Fail `store_wrap_key` — the fail-stop of row 5's first window.
    refuse_store: bool,
    /// Fail `load_wrap_key` — row 4's legacy-consult tripwire (the index
    /// write and the deferred purge still answer: store/delete directions).
    refuse_load: bool,
    /// Record every `purge_namespace` result — "exactly once" observable.
    purges: Option<Arc<Mutex<Vec<usize>>>>,
}

/// The fault carrier: one delegating wrapper, three policies. Every call but
/// the faulted direction passes through unchanged, so a faulted run differs
/// from an honest one in exactly one place.
struct FaultyVault {
    inner: Arc<dyn KeychainPort>,
    refuse_store: bool,
    refuse_load: bool,
    purges: Option<Arc<Mutex<Vec<usize>>>>,
}

impl KeychainPort for FaultyVault {
    fn probe(&self) -> Result<(), WalletError> {
        self.inner.probe()
    }

    fn tier(&self) -> Result<VaultTier, WalletError> {
        self.inner.tier()
    }

    fn store_wrap_key(
        &self,
        key: crate::seal::SealKey,
        sealed_blob: &[u8],
    ) -> Result<crate::keychain::WrapArtifact, WalletError> {
        if self.refuse_store {
            return Err(WalletError::KeystoreUnavailable);
        }
        self.inner.store_wrap_key(key, sealed_blob)
    }

    fn load_wrap_key(
        &self,
        artifact: &crate::keychain::WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<crate::seal::SealKey, WalletError> {
        if self.refuse_load {
            return Err(WalletError::KeystoreUnavailable);
        }
        self.inner.load_wrap_key(artifact, sealed_blob)
    }

    fn rotate_wrap_key(
        &self,
        artifact: &crate::keychain::WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<crate::keychain::WrapArtifact, WalletError> {
        self.inner.rotate_wrap_key(artifact, sealed_blob)
    }

    fn finish_rotation(
        &self,
        old: &crate::keychain::WrapArtifact,
        new: &crate::keychain::WrapArtifact,
    ) -> Result<(), WalletError> {
        self.inner.finish_rotation(old, new)
    }

    fn delete_wrap_key(&self, artifact: &crate::keychain::WrapArtifact) -> Result<(), WalletError> {
        self.inner.delete_wrap_key(artifact)
    }

    fn purge_namespace(&self) -> Result<usize, WalletError> {
        let severed = self.inner.purge_namespace()?;
        if let Some(p) = &self.purges {
            p.lock().expect("purges").push(severed);
        }
        Ok(severed)
    }

    // Join: the trait grew the three S2 index methods — this double's policy
    // is "every call but the faulted direction passes through unchanged".
    fn store_index(&self, entry: &crate::custody::CustodyIndexEntry) -> Result<(), WalletError> {
        self.inner.store_index(entry)
    }

    fn load_index(&self) -> Result<Option<crate::custody::CustodyIndexEntry>, WalletError> {
        self.inner.load_index()
    }

    fn delete_index(&self) -> Result<(), WalletError> {
        self.inner.delete_index()
    }
}

/// The pre-stage fixture: a wallet "created before this stage" — its wrap
/// key under the LEGACY path namespace, no identifier in its artifact
/// header. Needs no keychain-double surgery: the create resolves EVERY
/// namespace to the legacy vault (so the store's wrap-key custody lands
/// where a base-tree create would have put it), and the header's minted id
/// is then stripped — the contract's own row-3 "none" edit, applied to a
/// wallet whose key really lives at the path namespace.
struct PreStage {
    /// Keeps the tempdir alive for the wallet's life.
    _parent: tempfile::TempDir,
    dir: PathBuf,
    path_ns: String,
    keychain: SharedKeychain,
}

async fn pre_stage_wallet(label: &str) -> PreStage {
    let keychain = SharedKeychainVault::shared();
    let parent = tempfile::tempdir().expect("parent dir");
    let dir = parent.path().join(label).join("wallet");
    std::fs::create_dir_all(&dir).expect("wallet dir");
    let path_ns = keychain_namespace_for(&dir).as_str().to_owned();
    assert!(
        keychain.lock().expect("kc").is_empty(),
        "a fresh shared keychain for a fresh fixture"
    );

    // Every namespace resolves to the LEGACY vault: the base-tree shape.
    let redirect = {
        let keychain = keychain.clone();
        let path_ns = path_ns.clone();
        Arc::new(move |_ns: &KeychainNamespace| shared_mk(&keychain, &path_ns))
    };
    let w = Wallet::create_resolving(
        cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
        raw_seed(),
        fn_resolver(redirect),
    )
    .await
    .expect("create the to-be-pre-stage wallet");
    w.close().await.expect("close the create");

    let artifact = read_locator(&dir);
    assert!(
        artifact.custody_id().is_some(),
        "the create minted an identifier (stripped next)"
    );
    // The strip-locator edit (join mapping): the same backend bytes, no
    // locator — `from_freshly_wrapped(as_bytes())` is the committed shape of
    // this half's `with_custody_id(None)`.
    write_locator(
        &dir,
        &crate::keychain::WrapArtifact::from_freshly_wrapped(artifact.as_bytes().to_vec()),
    );
    assert!(
        read_locator(&dir).custody_id().is_none(),
        "the header now carries no identifier — the pre-stage shape"
    );
    assert!(
        keychain.lock().expect("kc").contains_key(&path_ns),
        "the wrap key lives under the legacy path namespace, the base-tree shape"
    );

    PreStage {
        _parent: parent,
        dir,
        path_ns,
        keychain,
    }
}

/// The exactly-one id namespace a run touched besides the legacy one (a
/// conforming run touches exactly two; a fresh mint would add a third).
fn the_id_ns(seen: &Arc<Mutex<Vec<String>>>, path_ns: &str) -> String {
    let mut ns: Vec<String> = distinct_ns(seen)
        .into_iter()
        .filter(|ns| ns != path_ns)
        .collect();
    assert_eq!(
        ns.len(),
        1,
        "exactly one non-legacy namespace — the identifier's; got {ns:?}"
    );
    ns.remove(0)
}

// ───────────────────────── row 3 — the tamper row ─────────────────────────

/// §3.1 row 3 (NEW — the full TAMPER row). The wrap artifact's header id
/// edited to another wallet's id, to a random id, or to none: every edit can
/// only REFUSE — typed, never a silent open of the wrong wallet. Integrity
/// is the AEAD's, not the locator's: the wrong id names a namespace whose
/// wrap key either does not authenticate this wallet's artifact
/// (`WrapArtifactInvalid`) or does not exist (the keysMissing class), and
/// "none" takes the legacy path, which for a wallet minted under an
/// identifier holds no key either. Row 4's mid-migration state (no id, a
/// PENDING index) is NOT tamper and OPENS — the two are distinguishable by
/// the index, and that half is row 5's.
///
/// UNLISTED CASE this row plants (IT-1 +A): a REFUSED tamper must not EAT
/// the wallet — after every refusal, restoring the true artifact reopens,
/// and the other wallet never noticed. A tamper handler that deleted or
/// rewrote the mismatched artifact would pass every refusal assertion and
/// still leave an unrecoverable wallet. `# provenance: contract §3.1 row 3`.
#[tokio::test]
async fn a_tampered_custody_locator_can_only_refuse_never_open_another_wallet() {
    let keychain = SharedKeychainVault::shared();
    let parent = tempfile::tempdir().expect("parent dir");
    let a = parent.path().join("container-a/wallet");
    let b = parent.path().join("container-b/wallet");
    std::fs::create_dir_all(&a).expect("container a");
    std::fs::create_dir_all(&b).expect("container b");
    let honest = |keychain: SharedKeychain, path_ns: String| -> VaultFactory {
        recording_resolver(keychain, Arc::new(Mutex::new(Vec::new())), &path_ns, None)
    };
    // The resolvers observe nothing here, but every vault must still come
    // from the header's own namespace — the tamper rows ride that.
    let ns_a = keychain_namespace_for(&a).as_str().to_owned();
    let ns_b = keychain_namespace_for(&b).as_str().to_owned();

    let wa = Wallet::create_resolving(
        cfg(&a, Network::Test, SeedPersistence::SealedKeychain),
        raw_seed(),
        fn_resolver(honest(keychain.clone(), ns_a.clone())),
    )
    .await
    .expect("create wallet a");
    wa.close().await.expect("close a");
    let wb = Wallet::create_resolving(
        cfg(&b, Network::Test, SeedPersistence::SealedKeychain),
        raw_seed(),
        fn_resolver(honest(keychain.clone(), ns_b.clone())),
    )
    .await
    .expect("create wallet b");
    wb.close().await.expect("close b");

    let artifact_a = read_locator(&a);
    let artifact_b = read_locator(&b);
    let id_a = artifact_a
        .custody_id()
        .expect("a's header carries its identifier");
    let id_b = artifact_b
        .custody_id()
        .expect("b's header carries its identifier");
    assert!(id_a != id_b, "two containers mint two identifiers");

    // (1) Edited to the OTHER wallet's id — names b's namespace, whose wrap
    // key does not authenticate a's artifact. (Rebuilt from the backend bytes
    // so `artifact_a` stays reusable for the later edits and the restore.)
    write_locator(
        &a,
        &crate::keychain::WrapArtifact::from_freshly_wrapped(artifact_a.as_bytes().to_vec())
            .with_custody_id(&id_b),
    );
    let opened = Wallet::open_resolving(
        cfg(&a, Network::Test, SeedPersistence::SealedKeychain),
        fn_resolver(honest(keychain.clone(), ns_a.clone())),
        None,
        None,
    )
    .await;
    assert_custody_refusal(&opened, "a's files under b's identifier");
    if let Ok(w) = opened {
        w.close().await.expect("close refused-pairing control");
    }

    // (2) Edited to a RANDOM id — a namespace that was never provisioned.
    write_locator(
        &a,
        &crate::keychain::WrapArtifact::from_freshly_wrapped(artifact_a.as_bytes().to_vec())
            .with_custody_id(&crate::custody::CustodyId::generate()),
    );
    let opened = Wallet::open_resolving(
        cfg(&a, Network::Test, SeedPersistence::SealedKeychain),
        fn_resolver(honest(keychain.clone(), ns_a.clone())),
        None,
        None,
    )
    .await;
    assert_custody_refusal(&opened, "a's files under a random identifier");
    if let Ok(w) = opened {
        w.close().await.expect("close refused-pairing control");
    }

    // (3) Edited to NONE — the legacy path, which for an identifier-minted
    // wallet holds no key (row 3's closing clause; on a MIGRATED wallet the
    // same edit fails the same way, its legacy namespace purged).
    write_locator(
        &a,
        &crate::keychain::WrapArtifact::from_freshly_wrapped(artifact_a.as_bytes().to_vec()),
    );
    let opened = Wallet::open_resolving(
        cfg(&a, Network::Test, SeedPersistence::SealedKeychain),
        fn_resolver(honest(keychain.clone(), ns_a.clone())),
        None,
        None,
    )
    .await;
    assert_custody_refusal(&opened, "a's files under no identifier");
    if let Ok(w) = opened {
        w.close().await.expect("close refused-pairing control");
    }

    // The unlisted case: every refusal above ate nothing — the true artifact
    // reopens, and b (untouched by any of it) still opens under its own.
    write_locator(&a, &artifact_a);
    let w = Wallet::open_resolving(
        cfg(&a, Network::Test, SeedPersistence::SealedKeychain),
        fn_resolver(honest(keychain.clone(), ns_a.clone())),
        None,
        None,
    )
    .await
    .expect("a reopens under its restored artifact — the tamper ate nothing");
    w.close().await.expect("close restored a");
    let w = Wallet::open_resolving(
        cfg(&b, Network::Test, SeedPersistence::SealedKeychain),
        fn_resolver(honest(keychain.clone(), ns_b.clone())),
        None,
        None,
    )
    .await
    .expect("b never noticed a's tamper");
    w.close().await.expect("close b");
}

// ───────────── row 4 — one commit point, no legacy consultation ─────────────

/// §3.1 row 4 (NEW). MIGRATION HAS ONE COMMIT POINT: a wallet created before
/// this stage opens once under the legacy namespace, migrates, and every
/// later open goes STRAIGHT to the id namespace — a mutant that consults the
/// legacy derivation on a migrated wallet reds the load tripwire of the
/// later opens below (the legacy vault refuses every LOAD, so an open that
/// took its wrap key from the legacy namespace could not succeed, while the
/// index write and the deferred purge — store and delete directions — still
/// answer). `# provenance: contract §3.1 row 4`.
#[tokio::test]
async fn a_migrated_wallet_opens_under_its_identifier_and_never_consults_the_legacy_namespace() {
    let PreStage {
        _parent,
        dir,
        path_ns,
        keychain,
    } = pre_stage_wallet("row-4").await;

    // Open 1 — the MIGRATING open, under the legacy namespace: the header
    // has no id, so the wallet takes the legacy path, opens, migrates,
    // commits.
    let seen1 = Arc::new(Mutex::new(Vec::new()));
    let w = Wallet::open_resolving(
        cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
        fn_resolver(recording_resolver(
            keychain.clone(),
            seen1.clone(),
            &path_ns,
            None,
        )),
        None,
        None,
    )
    .await
    .expect("the pre-stage wallet opens under the legacy namespace and migrates");
    w.close().await.expect("close the migrating open");

    let migrated = read_locator(&dir);
    let id = migrated
        .custody_id()
        .expect("the migration committed an identifier into the header");
    let id_ns = the_id_ns(&seen1, &path_ns);
    assert!(
        distinct_ns(&seen1).len() == 2 && distinct_ns(&seen1).contains(&path_ns),
        "the migrating open used the legacy and the id namespace only: {:?}",
        distinct_ns(&seen1)
    );

    // Open 2 and 3 — later opens go straight to the id namespace: the legacy
    // vault refuses every LOAD, and both opens still succeed. The same
    // identifier answers every time (a re-mint would ask for a third
    // namespace).
    for i in 0..2 {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let w = Wallet::open_resolving(
            cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
            fn_resolver(recording_resolver(
                keychain.clone(),
                seen.clone(),
                &path_ns,
                Some(Fault {
                    on_path: true,
                    refuse_store: false,
                    refuse_load: true,
                    purges: None,
                }),
            )),
            None,
            None,
        )
        .await
        .unwrap_or_else(|e| panic!("later open {i} goes straight to the id namespace; got {e:?}"));
        w.close()
            .await
            .unwrap_or_else(|e| panic!("close later open {i}: {e:?}"));

        assert_eq!(
            the_id_ns(&seen, &path_ns),
            id_ns,
            "the same identifier's namespace answers every open (no re-mint)"
        );
        let now = read_locator(&dir)
            .custody_id()
            .expect("the header keeps its identifier");
        assert!(now == id, "the identifier is stable across opens");
    }
}

// ───────────── row 5 — a crash in every window converges ─────────────

/// §3.1 row 5 (NEW). One variant per kill window — after the index write,
/// after the new keychain item, after the commit but before the deferred
/// purge — each re-run opens the wallet and finishes the migration with the
/// SAME id, and the legacy namespace is purged exactly once. "The same id"
/// is observed where a blind test can see it: the NAMESPACE the resolver is
/// asked for (`hex(SHA256(CUSTODY_DOMAIN ‖ id))[..16]`) — a fresh mint would
/// ask for a THIRD namespace, and a fresh mint is exactly what would orphan
/// one). `# provenance: contract §3.1 row 5`.
#[tokio::test]
async fn a_crash_mid_migration_leaves_a_wallet_that_opens_and_migrates_again() {
    crash_after_the_index_write_converges().await;
    crash_after_the_new_keychain_item_converges().await;
    crash_after_the_commit_before_the_deferred_purge_converges().await;
}

/// The re-run half every window shares: the honest resolver (with the purge
/// recorder riding the legacy vault) opens the wallet, finishes the
/// migration with the SAME id namespace the faulted run minted, one further
/// open settles the deferred purge, and no third namespace ever appears.
/// (`id_ns_faulted` is the id namespace the FAULTED run asked its resolver
/// for — every window asserts it exists before this runs.)
async fn the_re_run_converges(
    dir: &Path,
    path_ns: &str,
    keychain: &SharedKeychain,
    id_ns_faulted: String,
) {
    let purges = Arc::new(Mutex::new(Vec::new()));
    let seen2 = Arc::new(Mutex::new(Vec::new()));
    let w = Wallet::open_resolving(
        cfg(dir, Network::Test, SeedPersistence::SealedKeychain),
        fn_resolver(recording_resolver(
            keychain.clone(),
            seen2.clone(),
            path_ns,
            Some(Fault {
                on_path: true,
                refuse_store: false,
                refuse_load: false,
                purges: Some(purges.clone()),
            }),
        )),
        None,
        None,
    )
    .await
    .expect("the re-run opens the wallet and finishes the migration");
    w.close().await.expect("close the re-run");

    let id_ns = the_id_ns(&seen2, path_ns);
    assert_eq!(
        id_ns, id_ns_faulted,
        "the re-run reused the faulted run's identifier — a fresh mint would \
         ask for a third namespace"
    );
    assert!(
        read_locator(dir).custody_id().is_some(),
        "the migration committed an identifier into the header"
    );

    // One further open: the deferred purge settles here at the latest.
    let seen3 = Arc::new(Mutex::new(Vec::new()));
    let w = Wallet::open_resolving(
        cfg(dir, Network::Test, SeedPersistence::SealedKeychain),
        fn_resolver(recording_resolver(
            keychain.clone(),
            seen3.clone(),
            path_ns,
            Some(Fault {
                on_path: true,
                refuse_store: false,
                refuse_load: false,
                purges: Some(purges.clone()),
            }),
        )),
        None,
        None,
    )
    .await
    .expect("the next open settles the deferred purge");
    w.close().await.expect("close the settling open");
    assert_eq!(
        the_id_ns(&seen3, path_ns),
        id_ns,
        "still the same identifier's namespace"
    );

    let severing = purges
        .lock()
        .expect("purges")
        .iter()
        .filter(|severed| **severed > 0)
        .count();
    assert_eq!(
        severing,
        1,
        "the legacy namespace was purged exactly once across the re-run and \
         the settling open (calls returned {:?})",
        purges.lock().expect("purges")
    );

    // No orphan: across every run, only the legacy and the one id namespace
    // were ever touched.
    let mut all = vec![path_ns.to_owned(), id_ns_faulted];
    all.extend(distinct_ns(&seen2));
    all.extend(distinct_ns(&seen3));
    all.sort();
    all.dedup();
    assert_eq!(
        all.len(),
        2,
        "no third namespace was ever minted across the crash and re-run: {all:?}"
    );
}

/// Window 1 — after the index write, before the id-namespace item: the
/// id-namespace vault refuses its `store_wrap_key`, aborting the migrating
/// open AFTER the index item landed (the index write is store-direction
/// through the legacy vault, which stays honest).
async fn crash_after_the_index_write_converges() {
    let PreStage {
        _parent,
        dir,
        path_ns,
        keychain,
    } = pre_stage_wallet("row5-w1").await;

    let seen1 = Arc::new(Mutex::new(Vec::new()));
    let aborted = Wallet::open_resolving(
        cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
        fn_resolver(recording_resolver(
            keychain.clone(),
            seen1.clone(),
            &path_ns,
            Some(Fault {
                on_path: false,
                refuse_store: true,
                refuse_load: false,
                purges: None,
            }),
        )),
        None,
        None,
    )
    .await;
    assert!(
        aborted.is_err(),
        "the faulted migrating open refuses — the crash lands inside the window"
    );
    if let Ok(w) = aborted {
        w.close().await.expect("close aborted control");
    }
    assert!(
        read_locator(&dir).custody_id().is_none(),
        "the commit never ran — the header still carries no identifier"
    );
    // The fault is ON the id-namespace store, so the vault was constructed —
    // the resolver was asked for the id namespace — before the abort.
    let id_ns = the_id_ns(&seen1, &path_ns);

    the_re_run_converges(&dir, &path_ns, &keychain, id_ns).await;
}

/// Window 2 — after the new keychain item, before the commit: the commit is
/// the single `write_atomic` of the wrap artifact, which stages its temp
/// file as `wrap.artifact.tmp` beside the final name — so a DIRECTORY
/// planted at that temp path fails exactly the commit (the artifact file
/// itself stays readable, and the keychain writes — index, id-namespace
/// item — have already landed).
async fn crash_after_the_new_keychain_item_converges() {
    let PreStage {
        _parent,
        dir,
        path_ns,
        keychain,
    } = pre_stage_wallet("row5-w2").await;

    let tmp = dir.join(format!("{}.tmp", crate::constants::WRAP_ARTIFACT_FILE_NAME));
    std::fs::create_dir(&tmp).expect("plant a directory at the commit's temp path");

    let seen1 = Arc::new(Mutex::new(Vec::new()));
    let aborted = Wallet::open_resolving(
        cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
        fn_resolver(recording_resolver(
            keychain.clone(),
            seen1.clone(),
            &path_ns,
            None,
        )),
        None,
        None,
    )
    .await;
    // Remove the plant BEFORE any assertion can panic, so the tempdir drops.
    std::fs::remove_dir(&tmp).expect("unplant the temp path");
    assert!(
        aborted.is_err(),
        "the commit's write_atomic refuses — the crash lands inside the window"
    );
    if let Ok(w) = aborted {
        w.close().await.expect("close aborted control");
    }
    assert!(
        read_locator(&dir).custody_id().is_none(),
        "the commit never ran"
    );
    let id_ns = the_id_ns(&seen1, &path_ns);
    assert!(
        keychain.lock().expect("kc").contains_key(&id_ns),
        "the id-namespace item was stored before the commit failed — the \
         window is AFTER the new keychain item"
    );

    the_re_run_converges(&dir, &path_ns, &keychain, id_ns).await;
}

/// Window 3 — after the commit, before the deferred purge: the purge is BY
/// DESIGN deferred to the next open ("on the next successful open under the
/// id namespace"), so the committing open leaves exactly this window — the
/// header carries the id, the legacy namespace's key is still live.
async fn crash_after_the_commit_before_the_deferred_purge_converges() {
    let PreStage {
        _parent,
        dir,
        path_ns,
        keychain,
    } = pre_stage_wallet("row5-w3").await;

    let seen1 = Arc::new(Mutex::new(Vec::new()));
    let w = Wallet::open_resolving(
        cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
        fn_resolver(recording_resolver(
            keychain.clone(),
            seen1.clone(),
            &path_ns,
            None,
        )),
        None,
        None,
    )
    .await
    .expect("the migrating open commits");
    w.close().await.expect("close the committing open");
    let id_ns = the_id_ns(&seen1, &path_ns);
    assert!(read_locator(&dir).custody_id().is_some(), "the commit ran");
    assert!(
        keychain.lock().expect("kc").contains_key(&path_ns),
        "the legacy key is still live — the deferred-purge window"
    );

    the_re_run_converges(&dir, &path_ns, &keychain, id_ns).await;
}

// ───────────── row 9 — never logged, never across the FFI bare ─────────────

/// §3.1 row 9 (NEW). Mirroring `seed_fingerprint.rs:20-26`: the type-level
/// refusal (`CustodyId` implements no `Display` — no log formatter, no
/// `to_string`) pinned the same way the wallet handle's non-`Debug` is
/// (`assert_not_impl_any!`), plus the two surface scans the sentence names —
/// no `tracing` line in the core carries the identifier, and no bridge/DTO
/// source under `sdk/zec_wallet` names it (build A has no regen; the token
/// baseline at the base commit is ZERO occurrences on that side, so ANY
/// appearance is a new crossing). The runtime capture layer was probed by
/// this item's first commit and sees no fields in this flow, so the static
/// pins are the instrument. `# provenance: contract §3.1 row 9`.
#[test]
fn the_custody_identifier_is_never_logged_and_never_crosses_the_ffi() {
    // (1) The type refuses Display AND Debug (`{:?}` in a tracing field).
    static_assertions::assert_not_impl_any!(crate::custody::CustodyId: std::fmt::Display, std::fmt::Debug);

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));

    // (2) No tracing line in the core crate carries it.
    let mut lines_scanned = 0usize;
    let mut token_seen = false;
    let mut carriers: Vec<String> = Vec::new();
    scan_sources(&manifest.join("src"), &mut |path, line| {
        lines_scanned += 1;
        let names_id = line.contains("CustodyId") || line.contains("custody_id");
        token_seen |= names_id;
        // A CARRIER is code: comment lines (a doc sentence about this very
        // rule can name both tokens) are skipped, which is why the bridge
        // scan below — whose baseline is ZERO appearances of the token —
        // deliberately does NOT skip them.
        if line.trim_start().starts_with("//") {
            return;
        }
        let is_event = line.contains("tracing::")
            || line.contains("info!(")
            || line.contains("warn!(")
            || line.contains("error!(")
            || line.contains("debug!(")
            || line.contains("trace!(")
            || line.contains("event!(");
        if names_id && is_event {
            carriers.push(format!("{}: {}", path.display(), line.trim()));
        }
    });
    assert!(
        token_seen,
        "the scan reads real sources — the identifier's own definition is in them"
    );
    assert!(
        lines_scanned > 1_000,
        "the core scan covered the crate ({lines_scanned} lines)"
    );
    assert!(
        carriers.is_empty(),
        "no tracing line may carry the custody identifier: {carriers:?}"
    );

    // (3) The bridge/DTO surface names it nowhere — Rust or Dart.
    let bridge_roots = [
        manifest.join("../zec_wallet/rust/src"),
        manifest.join("../zec_wallet/lib"),
    ];
    let mut bridge_lines = 0usize;
    let mut crossings: Vec<String> = Vec::new();
    for root in &bridge_roots {
        scan_sources(root, &mut |path, line| {
            bridge_lines += 1;
            if line.contains("CustodyId")
                || line.contains("custody_id")
                || line.contains("custodyId")
            {
                crossings.push(format!("{}: {}", path.display(), line.trim()));
            }
        });
    }
    assert!(
        bridge_lines > 0,
        "the bridge scan found the bridge sources ({} under {:?})",
        bridge_lines,
        bridge_roots
    );
    assert!(
        crossings.is_empty(),
        "the identifier never crosses to the bridge/DTO surface: {crossings:?}"
    );
}

/// Walk `dir` recursively (skipping `target`), passing every line of every
/// `.rs`/`.dart` file to `visit` with its path. Unreadable files contribute
/// nothing — a scan that cannot read a file cannot clear it either, so the
/// non-vacuity asserts in the caller are what keep this honest.
fn scan_sources(dir: &Path, visit: &mut dyn FnMut(&Path, &str)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if file_type.is_dir() {
            if path.file_name().map(|n| n == "target").unwrap_or(false) {
                continue;
            }
            scan_sources(&path, visit);
        } else if path.extension().is_some_and(|e| e == "rs" || e == "dart")
            && let Ok(text) = std::fs::read_to_string(&path)
        {
            for line in text.lines() {
                visit(&path, line);
            }
        }
    }
}

// ───────── the fold's rows — the orchestrator's review of the join ─────────
//
// Four defects the adjudicated join still carried (stage plan §5, review),
// each row red against the joined tree before its fix, plus one pin the wipe
// fix must not break. Appended below everything the halves wrote, so no cited
// line above moves.

/// A pass-through vault for the PATH namespace that counts index writes and,
/// on request, refuses the namespace purge — the two things the post-commit
/// bookkeeping must survive without refusing an open.
struct IndexWatchVault {
    inner: Arc<dyn KeychainPort>,
    index_writes: Arc<std::sync::atomic::AtomicUsize>,
    refuse_purge: bool,
}

impl KeychainPort for IndexWatchVault {
    fn probe(&self) -> Result<(), WalletError> {
        self.inner.probe()
    }

    fn tier(&self) -> Result<VaultTier, WalletError> {
        self.inner.tier()
    }

    fn store_wrap_key(
        &self,
        key: crate::seal::SealKey,
        sealed_blob: &[u8],
    ) -> Result<crate::keychain::WrapArtifact, WalletError> {
        self.inner.store_wrap_key(key, sealed_blob)
    }

    fn load_wrap_key(
        &self,
        artifact: &crate::keychain::WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<crate::seal::SealKey, WalletError> {
        self.inner.load_wrap_key(artifact, sealed_blob)
    }

    fn rotate_wrap_key(
        &self,
        artifact: &crate::keychain::WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<crate::keychain::WrapArtifact, WalletError> {
        self.inner.rotate_wrap_key(artifact, sealed_blob)
    }

    fn finish_rotation(
        &self,
        old: &crate::keychain::WrapArtifact,
        new: &crate::keychain::WrapArtifact,
    ) -> Result<(), WalletError> {
        self.inner.finish_rotation(old, new)
    }

    fn delete_wrap_key(&self, artifact: &crate::keychain::WrapArtifact) -> Result<(), WalletError> {
        self.inner.delete_wrap_key(artifact)
    }

    fn purge_namespace(&self) -> Result<usize, WalletError> {
        if self.refuse_purge {
            return Err(WalletError::KeystoreUnavailable);
        }
        self.inner.purge_namespace()
    }

    fn store_index(&self, entry: &crate::custody::CustodyIndexEntry) -> Result<(), WalletError> {
        self.index_writes.fetch_add(1, Ordering::SeqCst);
        self.inner.store_index(entry)
    }

    fn load_index(&self) -> Result<Option<crate::custody::CustodyIndexEntry>, WalletError> {
        self.inner.load_index()
    }

    fn delete_index(&self) -> Result<(), WalletError> {
        self.inner.delete_index()
    }
}

/// The per-namespace resolver over the shared keychain, with the PATH
/// namespace's vault wrapped in an [`IndexWatchVault`].
fn watched_resolver(
    keychain: &SharedKeychain,
    path_ns: &str,
    index_writes: Arc<std::sync::atomic::AtomicUsize>,
    refuse_purge: bool,
) -> Arc<dyn VaultResolver> {
    let keychain = keychain.clone();
    let path_ns = path_ns.to_owned();
    fn_resolver(Arc::new(
        move |ns: &KeychainNamespace| -> Arc<dyn KeychainPort> {
            let vault = shared_mk(&keychain, ns.as_str());
            if ns.as_str() == path_ns {
                Arc::new(IndexWatchVault {
                    inner: vault,
                    index_writes: index_writes.clone(),
                    refuse_purge,
                })
            } else {
                vault
            }
        },
    ))
}

/// The plain per-namespace resolver over the shared keychain.
fn per_namespace(keychain: &SharedKeychain) -> Arc<dyn VaultResolver> {
    let keychain = keychain.clone();
    fn_resolver(Arc::new(move |ns: &KeychainNamespace| {
        shared_mk(&keychain, ns.as_str())
    }))
}

/// A fresh post-S2 wallet created at `dir` over the per-namespace resolver,
/// closed; returns its custody identifier.
async fn created_at(keychain: &SharedKeychain, dir: &Path) -> crate::custody::CustodyId {
    std::fs::create_dir_all(dir).expect("wallet dir");
    let w = Wallet::create_resolving(
        cfg(dir, Network::Test, SeedPersistence::SealedKeychain),
        raw_seed(),
        per_namespace(keychain),
    )
    .await
    .expect("create");
    w.close().await.expect("close the create");
    read_locator(dir)
        .custody_id()
        .expect("a post-S2 create frames its identifier")
}

/// §3.1 mechanism step (2), exactly: "a re-run REUSES this id" — the id of
/// THIS path's own unfinished (`pending`) migration, nothing else. A `done`
/// entry at the path belongs to a wallet that moved away or was deleted
/// without a wipe; borrowing its identifier would custody this wallet under
/// ANOTHER wallet's namespace (on Apple's Secure Enclave `store_wrap_key`
/// begins by deleting everything under that namespace) and would skip the
/// `pending` record the deferred purge reads — the legacy key would then
/// never be purged (the root cause of row 5's red on the repaired join).
#[tokio::test]
async fn a_migration_never_borrows_an_identifier_from_a_finished_index_entry() {
    let PreStage {
        _parent,
        dir,
        path_ns,
        keychain,
    } = pre_stage_wallet("fold-l1").await;
    // Another wallet, alive elsewhere on the same keychain; its identifier is
    // planted as a finished index entry at the pre-stage wallet's path.
    let other_dir = _parent.path().join("elsewhere/wallet");
    let other_id = created_at(&keychain, &other_dir).await;
    let other_ns = crate::custody::namespace_for(&other_id);
    shared_mk(&keychain, &path_ns)
        .store_index(&crate::custody::CustodyIndexEntry::done(&other_id))
        .expect("plant the finished foreign entry");

    let seen = Arc::new(Mutex::new(Vec::new()));
    let w = Wallet::open_resolving(
        cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
        fn_resolver(recording_resolver(
            keychain.clone(),
            seen.clone(),
            &path_ns,
            None,
        )),
        None,
        None,
    )
    .await
    .expect("the pre-stage wallet opens and migrates");
    w.close().await.expect("close the migrating open");

    let committed = read_locator(&dir)
        .custody_id()
        .expect("the migration committed an identifier");
    assert!(
        committed != other_id,
        "the migration minted its own identifier — it never borrows another wallet's"
    );
    assert!(
        !distinct_ns(&seen).contains(&other_ns.as_str().to_owned()),
        "the migrating open never asked for the other wallet's namespace"
    );
    let index = shared_mk(&keychain, &path_ns)
        .load_index()
        .expect("read the index")
        .expect("the migration recorded itself");
    assert!(
        index.is_pending() && index.id == committed,
        "the migration's own pending record, naming the committed identifier"
    );
    // The other wallet is untouched.
    Wallet::open_resolving(
        cfg(&other_dir, Network::Test, SeedPersistence::SealedKeychain),
        per_namespace(&keychain),
        None,
        None,
    )
    .await
    .expect("the other wallet still opens")
    .close()
    .await
    .expect("close the other wallet");
}

/// The deferral's whole point (the s2-custody ruling §1.2): a legacy purge
/// that cannot complete after the commit never refuses an open of the
/// committed wallet. It stays `pending` — recorded, so a wipe purges both
/// namespaces — and the next healthy open finishes it.
#[tokio::test]
async fn a_legacy_purge_that_fails_after_the_commit_never_refuses_an_open() {
    let PreStage {
        _parent,
        dir,
        path_ns,
        keychain,
    } = pre_stage_wallet("fold-h1").await;
    let open = |resolver: Arc<dyn VaultResolver>| {
        Wallet::open_resolving(
            cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
            resolver,
            None,
            None,
        )
    };
    open(per_namespace(&keychain))
        .await
        .expect("the committing open")
        .close()
        .await
        .expect("close the committing open");
    assert!(read_locator(&dir).custody_id().is_some(), "committed");

    let writes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    open(watched_resolver(&keychain, &path_ns, writes, true))
        .await
        .expect("a wedged legacy purge must not refuse the committed wallet's open")
        .close()
        .await
        .expect("close the wedged open");
    assert!(
        keychain.lock().expect("kc").contains_key(&path_ns),
        "the legacy key is still live — the purge is still owed"
    );
    assert!(
        shared_mk(&keychain, &path_ns)
            .load_index()
            .expect("read the index")
            .is_some_and(|e| e.is_pending()),
        "the owed purge stays recorded as pending"
    );

    open(per_namespace(&keychain))
        .await
        .expect("the next healthy open")
        .close()
        .await
        .expect("close the healthy open");
    assert!(
        !keychain.lock().expect("kc").contains_key(&path_ns),
        "the next healthy open finished the purge"
    );
    assert!(
        shared_mk(&keychain, &path_ns)
            .load_index()
            .expect("read the index")
            .is_some_and(|e| !e.is_pending()),
        "and recorded it done"
    );
}

/// A steady-state open leaves the index alone: on Android an index write
/// deletes the prior alias and GENERATES a new Keystore key (StrongBox
/// first), so a write per open is a per-open key generation and a window
/// with no index at all. The index is written only when it is missing or
/// wrong — at the first open after a relocation, for one.
#[tokio::test]
async fn a_steady_open_leaves_the_index_alone() {
    let keychain = SharedKeychainVault::shared();
    let parent = tempfile::tempdir().expect("parent dir");
    let dir = parent.path().join("container-a/wallet");
    created_at(&keychain, &dir).await;
    let path_ns = keychain_namespace_for(&dir).as_str().to_owned();

    let writes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    for _ in 0..2 {
        Wallet::open_resolving(
            cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
            watched_resolver(&keychain, &path_ns, writes.clone(), false),
            None,
            None,
        )
        .await
        .expect("open")
        .close()
        .await
        .expect("close");
    }
    assert_eq!(
        writes.load(Ordering::SeqCst),
        0,
        "two steady opens rewrote the index"
    );

    // A relocation: the new path has no index yet — the first open writes it,
    // the second leaves it.
    let moved = parent.path().join("container-b/wallet");
    std::fs::create_dir_all(moved.parent().expect("parent")).expect("container b");
    std::fs::rename(&dir, &moved).expect("relocate");
    let moved_ns = keychain_namespace_for(&moved).as_str().to_owned();
    let writes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    for _ in 0..2 {
        Wallet::open_resolving(
            cfg(&moved, Network::Test, SeedPersistence::SealedKeychain),
            watched_resolver(&keychain, &moved_ns, writes.clone(), false),
            None,
            None,
        )
        .await
        .expect("open the relocated wallet")
        .close()
        .await
        .expect("close");
    }
    assert_eq!(
        writes.load(Ordering::SeqCst),
        1,
        "exactly one index write at the new path"
    );
}

/// §3.1 row 7 ("a wipe never reaches anything that is not this wallet's")
/// against a TAMPERED header. An open refuses a locator whose key does not
/// authenticate the files (row 3); a wipe must not sever it either — the
/// header is a file in the sandbox, the index a keychain item. A header the
/// index does not corroborate is trusted only if it unseals THIS wallet's
/// files; otherwise the wipe goes by the index.
#[tokio::test]
async fn a_wipe_never_severs_a_namespace_its_header_cannot_prove() {
    let keychain = SharedKeychainVault::shared();
    let parent = tempfile::tempdir().expect("parent dir");
    let a = parent.path().join("container-a/wallet");
    let b = parent.path().join("container-b/wallet");
    let id_a = created_at(&keychain, &a).await;
    let id_b = created_at(&keychain, &b).await;

    // A's header now names B's identifier.
    write_locator(&a, &read_locator(&a).with_custody_id(&id_b));

    Wallet::wipe_resolving(&a, Some(per_namespace(&keychain)), false)
        .await
        .expect("A's wipe completes through its index");
    assert!(
        !keychain
            .lock()
            .expect("kc")
            .contains_key(crate::custody::namespace_for(&id_a).as_str()),
        "A's own key is severed"
    );
    Wallet::open_resolving(
        cfg(&b, Network::Test, SeedPersistence::SealedKeychain),
        per_namespace(&keychain),
        None,
        None,
    )
    .await
    .expect("B's custody survived A's wipe")
    .close()
    .await
    .expect("close B");
}

/// The pin the header proof must not break: a wallet that moved and was never
/// opened at its new path has NO index there — its header is the only
/// locator, and it proves itself by unsealing the files, so the wipe still
/// severs the moved wallet's key.
#[tokio::test]
async fn a_wipe_at_a_new_path_before_any_open_there_still_severs_the_moved_wallet() {
    let keychain = SharedKeychainVault::shared();
    let parent = tempfile::tempdir().expect("parent dir");
    let a = parent.path().join("container-a/wallet");
    let id = created_at(&keychain, &a).await;
    let moved = parent.path().join("container-b/wallet");
    std::fs::create_dir_all(moved.parent().expect("parent")).expect("container b");
    std::fs::rename(&a, &moved).expect("relocate");

    Wallet::wipe_resolving(&moved, Some(per_namespace(&keychain)), false)
        .await
        .expect("the wipe at the new path completes");
    assert!(
        !keychain
            .lock()
            .expect("kc")
            .contains_key(crate::custody::namespace_for(&id).as_str()),
        "the moved wallet's key is severed"
    );
    assert!(!moved.exists(), "and its files are gone");
}

/// §3.1 row 6 through a PER-NAMESPACE resolver (the fold). The row-6
/// test above runs over a pre-built vault, which answers every namespace
/// with the same item — it passes whether or not the wipe found the
/// identifier's namespace through the index. Here the key lives ONLY under
/// the identifier's namespace: with the host's files already gone, the index
/// is the one thing that can name it (Relim's panic wipe, by their spec).
#[tokio::test]
async fn a_wipe_after_the_host_deleted_the_files_finds_the_identifier_through_the_index() {
    let keychain = SharedKeychainVault::shared();
    let parent = tempfile::tempdir().expect("parent dir");
    let dir = parent.path().join("container-a/wallet");
    let id = created_at(&keychain, &dir).await;
    let id_ns = crate::custody::namespace_for(&id).as_str().to_owned();
    let path_ns = keychain_namespace_for(&dir).as_str().to_owned();
    assert!(
        keychain.lock().expect("kc").contains_key(&id_ns),
        "the wrap key lives under the identifier's namespace, not the path's"
    );

    std::fs::remove_dir_all(&dir).expect("the host deleted the files");
    Wallet::wipe_resolving(&dir, Some(per_namespace(&keychain)), false)
        .await
        .expect("the wipe converges from a bare config");
    assert!(
        !keychain.lock().expect("kc").contains_key(&id_ns),
        "the identifier's namespace was found through the index and severed"
    );
    assert!(
        shared_mk(&keychain, &path_ns)
            .load_index()
            .expect("read the index")
            .is_none(),
        "the index breadcrumb is deleted last"
    );
    Wallet::wipe_resolving(&dir, Some(per_namespace(&keychain)), false)
        .await
        .expect("a second wipe is a no-op success");
}

/// §3.1 row 6's "both namespaces under a pending migration", through a
/// per-namespace resolver: a pre-stage wallet whose migration committed but
/// whose deferred purge has not run yet holds a LIVE key under BOTH the
/// legacy (path) and the identifier's namespace; a wipe with the files
/// already gone severs both.
#[tokio::test]
async fn a_wipe_during_a_pending_migration_severs_both_namespaces() {
    let PreStage {
        _parent,
        dir,
        path_ns,
        keychain,
    } = pre_stage_wallet("fold-row6-pending").await;
    Wallet::open_resolving(
        cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
        per_namespace(&keychain),
        None,
        None,
    )
    .await
    .expect("the committing open")
    .close()
    .await
    .expect("close the committing open");
    let id_ns = crate::custody::namespace_for(
        &read_locator(&dir)
            .custody_id()
            .expect("the migration committed an identifier"),
    )
    .as_str()
    .to_owned();
    {
        let kc = keychain.lock().expect("kc");
        assert!(
            kc.contains_key(&path_ns) && kc.contains_key(&id_ns),
            "pending: the legacy key and the identifier's key are both live"
        );
    }

    std::fs::remove_dir_all(&dir).expect("the host deleted the files");
    Wallet::wipe_resolving(&dir, Some(per_namespace(&keychain)), false)
        .await
        .expect("the wipe converges from a bare config");
    let kc = keychain.lock().expect("kc");
    assert!(!kc.contains_key(&id_ns), "the identifier's key is severed");
    assert!(!kc.contains_key(&path_ns), "and the legacy key with it");
}

// ───────── the diff review's rows — red first against e144fc7e ─────────

/// A pass-through vault for the PATH namespace whose INDEX calls can be made
/// to fail: a refused read (the keychain answers, but not for this item), a
/// refused write (a kill between the legacy purge and the index write, as the
/// store sees it). Every other call passes through unchanged.
struct IndexFaultVault {
    inner: Arc<dyn KeychainPort>,
    refuse_index_load: bool,
    refuse_index_store: bool,
}

impl KeychainPort for IndexFaultVault {
    fn probe(&self) -> Result<(), WalletError> {
        self.inner.probe()
    }

    fn tier(&self) -> Result<VaultTier, WalletError> {
        self.inner.tier()
    }

    fn store_wrap_key(
        &self,
        key: crate::seal::SealKey,
        sealed_blob: &[u8],
    ) -> Result<crate::keychain::WrapArtifact, WalletError> {
        self.inner.store_wrap_key(key, sealed_blob)
    }

    fn load_wrap_key(
        &self,
        artifact: &crate::keychain::WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<crate::seal::SealKey, WalletError> {
        self.inner.load_wrap_key(artifact, sealed_blob)
    }

    fn rotate_wrap_key(
        &self,
        artifact: &crate::keychain::WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<crate::keychain::WrapArtifact, WalletError> {
        self.inner.rotate_wrap_key(artifact, sealed_blob)
    }

    fn finish_rotation(
        &self,
        old: &crate::keychain::WrapArtifact,
        new: &crate::keychain::WrapArtifact,
    ) -> Result<(), WalletError> {
        self.inner.finish_rotation(old, new)
    }

    fn delete_wrap_key(&self, artifact: &crate::keychain::WrapArtifact) -> Result<(), WalletError> {
        self.inner.delete_wrap_key(artifact)
    }

    fn purge_namespace(&self) -> Result<usize, WalletError> {
        self.inner.purge_namespace()
    }

    fn store_index(&self, entry: &crate::custody::CustodyIndexEntry) -> Result<(), WalletError> {
        if self.refuse_index_store {
            return Err(WalletError::KeystoreUnavailable);
        }
        self.inner.store_index(entry)
    }

    fn load_index(&self) -> Result<Option<crate::custody::CustodyIndexEntry>, WalletError> {
        if self.refuse_index_load {
            return Err(WalletError::KeystoreUnavailable);
        }
        self.inner.load_index()
    }

    fn delete_index(&self) -> Result<(), WalletError> {
        self.inner.delete_index()
    }
}

/// The per-namespace resolver with the PATH namespace's vault wrapped in an
/// [`IndexFaultVault`].
fn index_fault_resolver(
    keychain: &SharedKeychain,
    path_ns: &str,
    refuse_index_load: bool,
    refuse_index_store: bool,
) -> Arc<dyn VaultResolver> {
    let keychain = keychain.clone();
    let path_ns = path_ns.to_owned();
    fn_resolver(Arc::new(
        move |ns: &KeychainNamespace| -> Arc<dyn KeychainPort> {
            let vault = shared_mk(&keychain, ns.as_str());
            if ns.as_str() == path_ns {
                Arc::new(IndexFaultVault {
                    inner: vault,
                    refuse_index_load,
                    refuse_index_store,
                })
            } else {
                vault
            }
        },
    ))
}

/// The review's arch CRITICAL: with the files already gone, the index is the
/// ONLY thing that can name the namespace — an index the wipe cannot READ must
/// not end in `Ok` with the wallet's key still live. It fails typed, nothing
/// severed, and a retry against a healthy keychain converges.
#[tokio::test]
async fn a_bare_config_wipe_that_cannot_read_the_index_fails_typed_and_leaves_nothing_behind() {
    let keychain = SharedKeychainVault::shared();
    let parent = tempfile::tempdir().expect("parent dir");
    let dir = parent.path().join("container-a/wallet");
    let id = created_at(&keychain, &dir).await;
    let id_ns = crate::custody::namespace_for(&id).as_str().to_owned();
    let path_ns = keychain_namespace_for(&dir).as_str().to_owned();

    std::fs::remove_dir_all(&dir).expect("the host deleted the files");
    let unreadable = Wallet::wipe_resolving(
        &dir,
        Some(index_fault_resolver(&keychain, &path_ns, true, false)),
        false,
    )
    .await;
    assert!(
        unreadable.is_err(),
        "a bare-config wipe that cannot read the index must not report success"
    );
    assert!(
        keychain.lock().expect("kc").contains_key(&id_ns),
        "nothing was severed on the refused wipe — the retry has what it needs"
    );

    Wallet::wipe_resolving(&dir, Some(per_namespace(&keychain)), false)
        .await
        .expect("the retry converges");
    assert!(
        !keychain.lock().expect("kc").contains_key(&id_ns),
        "and the retry severed the identifier's key"
    );
}

/// The review's security finding: a `pending` record's legacy namespace is
/// purged only when it is this path's own. A record naming another wallet's
/// namespace — a stale or planted entry — never makes a wipe of THIS wallet
/// sever it.
#[tokio::test]
async fn a_wipe_never_purges_a_legacy_namespace_its_pending_record_does_not_own() {
    let keychain = SharedKeychainVault::shared();
    let parent = tempfile::tempdir().expect("parent dir");
    let a = parent.path().join("container-a/wallet");
    let b = parent.path().join("container-b/wallet");
    let id_a = created_at(&keychain, &a).await;
    let id_b = created_at(&keychain, &b).await;
    let a_path_ns = keychain_namespace_for(&a).as_str().to_owned();

    // A's index now claims a pending migration whose legacy namespace is B's.
    shared_mk(&keychain, &a_path_ns)
        .store_index(&crate::custody::CustodyIndexEntry::pending(
            &id_a,
            &crate::custody::namespace_for(&id_b),
        ))
        .expect("plant the foreign pending record");

    Wallet::wipe_resolving(&a, Some(per_namespace(&keychain)), false)
        .await
        .expect("A's wipe completes");
    Wallet::open_resolving(
        cfg(&b, Network::Test, SeedPersistence::SealedKeychain),
        per_namespace(&keychain),
        None,
        None,
    )
    .await
    .expect("B's custody survived A's wipe")
    .close()
    .await
    .expect("close B");
}

/// The review's crypto CRITICAL: the deferred legacy purge runs on the
/// index's own namespace. If the purge also swept the index, a kill between
/// the purge and the `done` write would leave NO index — and the host's
/// delete-then-wipe would "succeed" with the key alive. The purge is wrap
/// material only; the record survives it, so the wipe still finds the key.
#[tokio::test]
async fn a_kill_between_the_legacy_purge_and_the_index_write_still_leaves_a_wipe_that_finds_the_key()
 {
    let PreStage {
        _parent,
        dir,
        path_ns,
        keychain,
    } = pre_stage_wallet("review-crypto-critical").await;
    Wallet::open_resolving(
        cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
        per_namespace(&keychain),
        None,
        None,
    )
    .await
    .expect("the committing open")
    .close()
    .await
    .expect("close the committing open");
    let id_ns = crate::custody::namespace_for(
        &read_locator(&dir)
            .custody_id()
            .expect("the migration committed an identifier"),
    )
    .as_str()
    .to_owned();

    // The settling open: the legacy purge runs, then the `done` write is lost.
    Wallet::open_resolving(
        cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
        index_fault_resolver(&keychain, &path_ns, false, true),
        None,
        None,
    )
    .await
    .expect("the settling open still opens")
    .close()
    .await
    .expect("close the settling open");
    assert!(
        !keychain.lock().expect("kc").contains_key(&path_ns),
        "the legacy key was purged before the lost index write"
    );

    std::fs::remove_dir_all(&dir).expect("the host deleted the files");
    Wallet::wipe_resolving(&dir, Some(per_namespace(&keychain)), false)
        .await
        .expect("the wipe converges");
    assert!(
        !keychain.lock().expect("kc").contains_key(&id_ns),
        "the index survived the purge, so the wipe found and severed the identifier's key"
    );
}
