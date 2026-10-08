//! Stage S9 `bound` (FR-47) — a wedged key store answers in time. The test
//! author's rows, written BLIND (IT-2a) against `5e1ec24e`, contract
//! `docs/plan/stage-9-a-wedged-keystore-answers-in-time.md` §3.1 (revision
//! 2). A child of `wallet::tests` for its fixtures (`cfg`, `raw_seed`,
//! `shared_mk`, `keychain_namespace_for`), in its own file so no cited line
//! in `wallet.rs` moves.
//!
//! EVERY row here names something the base tree does not carry (the
//! decorator, the timeout variant, the constants, `store_wrap_key` by value),
//! so the module is declared `#[cfg(any())]` in `wallet.rs` — never built —
//! until the adjudicator joins it (the `delivery_obligation_named` shape).
//!
//! COUPLING DECLARED FOR THE JOIN — every name below is this author's
//! spelling of the contract's words; the adjudicator maps them in the
//! "declared names" section, which is the only place they are spelled:
//!
//!   * `crate::keychain::BoundedVault::with_bounds(inner: Arc<dyn
//!     KeychainPort>, call_bound: Duration, wipe_budget: Duration)` — the
//!     contract's seam, taken as returning `Self`.
//!   * `BoundedVault::worker_spawns() -> usize` — the contract's `cfg(test)`
//!     spawn counter (process-wide, never reset).
//!   * `BoundedVault::reset_severed_late()` — the contract's `cfg(test)`
//!     reset of the process-wide severed-late set (assertion 7a's "a fresh
//!     process").
//!   * `BoundedVault::payloads_held() -> usize` — assertion 12's drop-probe:
//!     how many queued jobs still hold their inputs. The contract names the
//!     probe but no symbol; see the finding on row 12.
//!   * `WalletError::KeychainTimeout { cause }` — `cause` is read through its
//!     `Debug` rendering, normalised (`Timeout` / `timeout` → "timeout",
//!     `PastDeadline` / `past_deadline` → "pastdeadline"), so its type is not
//!     named here.
//!   * `crate::keychain::{KEYCHAIN_CALL_BOUND, KEYCHAIN_WIPE_BUDGET}` as
//!     `std::time::Duration`.
//!   * `KeychainPort::store_wrap_key(&self, key: SealKey, sealed_blob: &[u8])`
//!     — the key BY VALUE (item 5).
//!
//! Every row serialises on one lock: the worker, the severed-late set and
//! the spawn counter are process-wide, so a row that wedges the worker would
//! turn a concurrent row's calls into `busy`. Any other test that drives the
//! bounded worker must take the same lock.
//!
//! No row sleeps to make its claim. The fake blocks on a gate the row holds,
//! so a call that answers while the gate is shut can only have answered
//! through the bound; `ANSWER_CEILING` exists only so a build without a bound
//! FAILS instead of hanging the suite.

#![allow(clippy::await_holding_lock)] // the serial guard is held across awaits on purpose

use super::*;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::custody::CustodyIndexEntry;
use crate::keychain::testvault::SharedKeychain;
use crate::keychain::{ResolvedVault, WrapArtifact};
use crate::seal::SealKey;

// ───────────────────────────── declared names ─────────────────────────────

// JOINED (adjudicator): the constants and the reset live in
// `keychain::bounded`. REPAIRED (orchestrator, from the CONTRACT_WRONG
// ruling): `with_bounds` now shares the process worker, and `worker_spawns` /
// `payloads_held` map to its process-wide probes.
use crate::keychain::BoundedVault;
use crate::keychain::bounded::{KEYCHAIN_CALL_BOUND, KEYCHAIN_WIPE_BUDGET};

/// The decorator over `inner`, with injected bounds (the contract's seam).
fn bounded(
    inner: Arc<dyn KeychainPort>,
    call_bound: Duration,
    budget: Duration,
) -> Arc<dyn KeychainPort> {
    // No namespace: these rows predate the S16 tombstone check.
    Arc::new(BoundedVault::with_bounds(inner, call_bound, budget, None))
}

/// How many times the process spawned the key-store worker.
fn worker_spawns() -> usize {
    BoundedVault::worker_spawns()
}

/// Forget every late sever the worker recorded — what a restart does.
fn forget_late_severs() {
    crate::keychain::bounded::reset_severed_late();
}

/// How many queued jobs still hold their inputs (row 12's drop-probe).
fn payloads_held() -> usize {
    BoundedVault::payloads_held()
}

/// The timeout's `cause`, normalised; `None` for any other error.
fn cause(e: &WalletError) -> Option<String> {
    match e {
        WalletError::KeychainTimeout { cause } => Some(
            format!("{cause:?}")
                .chars()
                .filter(char::is_ascii_alphanumeric)
                .collect::<String>()
                .to_ascii_lowercase(),
        ),
        _ => None,
    }
}

// ───────────────────────────── the clock ─────────────────────────────

/// The per-call bound the wedge rows inject (the wipe budget is larger, so a
/// wedged call inside a wipe is cut by this one).
const BOUND: Duration = Duration::from_millis(1_500);
/// The wipe budget the wedge rows inject.
const BUDGET: Duration = Duration::from_millis(2_500);
/// The bound of a caller that leaves while its job is still queued.
const SHORT: Duration = Duration::from_millis(300);
/// The bound of a caller that stays for as long as the row needs it.
const LONG: Duration = Duration::from_secs(120);
/// Anti-hang only: a build with no bound fails here instead of hanging.
const ANSWER_CEILING: Duration = Duration::from_secs(30);

/// Wrap-key "sealed blob" bytes for the direct `store_wrap_key` calls.
const BLOB: &[u8] = b"s9-bound sealed blob";

// ───────────────────────────── serialisation ─────────────────────────────

static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

// ───────────────────────────── the wedging fake ─────────────────────────────

/// A gate the fake's wedged call blocks on until the row opens it.
#[derive(Default)]
struct Gate {
    open: Mutex<bool>,
    cv: Condvar,
}

impl Gate {
    fn wait(&self) {
        let mut open = self.open.lock().unwrap_or_else(|p| p.into_inner());
        while !*open {
            open = self.cv.wait(open).unwrap_or_else(|p| p.into_inner());
        }
    }

    fn release(&self) {
        *self.open.lock().unwrap_or_else(|p| p.into_inner()) = true;
        self.cv.notify_all();
    }
}

/// What a `purge_namespace` does once it runs.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Purge {
    /// Delegates: the key under the namespace is severed.
    Real,
    /// The iOS lock interleaving: the SE key query is filtered, so the purge
    /// answers `Ok(0)` and severs nothing while the key stays live.
    Locked,
}

/// The fake's policy and its observations, shared by every vault it builds.
struct Ctl {
    /// The one port method that blocks on the gate.
    wedge: Option<&'static str>,
    /// Every call takes this long (the slow keychain).
    slow: Option<Duration>,
    /// The port method that panics.
    panic_on: Option<&'static str>,
    /// The port method that fails `KeystoreUnavailable` (a locked keychain).
    locked_on: Option<&'static str>,
    purge: Purge,
    gate: Gate,
    /// Every call that reached the fake: (method, namespace).
    entered: Mutex<Vec<(&'static str, String)>>,
    entered_cv: Condvar,
    in_flight: AtomicUsize,
    max_in_flight: AtomicUsize,
}

impl Ctl {
    fn new() -> Self {
        Self {
            wedge: None,
            slow: None,
            panic_on: None,
            locked_on: None,
            purge: Purge::Real,
            gate: Gate::default(),
            entered: Mutex::new(Vec::new()),
            entered_cv: Condvar::new(),
            in_flight: AtomicUsize::new(0),
            max_in_flight: AtomicUsize::new(0),
        }
    }

    fn wedging(op: &'static str) -> Self {
        Self {
            wedge: Some(op),
            ..Self::new()
        }
    }

    fn calls(&self) -> Vec<(&'static str, String)> {
        self.entered
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    fn count(&self, op: &str) -> usize {
        self.calls().iter().filter(|(o, _)| *o == op).count()
    }

    fn count_in(&self, op: &str, ns: &str) -> usize {
        self.calls()
            .iter()
            .filter(|(o, n)| *o == op && n == ns)
            .count()
    }

    /// Block until `op` has reached the fake (the worker is RUNNING it).
    fn wait_entered(&self, op: &str) {
        let deadline = Instant::now() + ANSWER_CEILING;
        let mut seen = self.entered.lock().unwrap_or_else(|p| p.into_inner());
        while !seen.iter().any(|(o, _)| *o == op) {
            let left = deadline
                .checked_duration_since(Instant::now())
                .unwrap_or_else(|| panic!("`{op}` never reached the vault"));
            seen = self
                .entered_cv
                .wait_timeout(seen, left)
                .unwrap_or_else(|p| p.into_inner())
                .0;
        }
    }
}

/// Opens the gate when the row ends, however it ends — a failed assertion
/// must not leave the process's worker wedged for every later row.
struct ReleaseOnDrop(Arc<Ctl>);

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.0.gate.release();
    }
}

/// Decrements the in-flight count on the way out, a panic included.
struct InFlight<'a>(&'a Ctl);

impl Drop for InFlight<'_> {
    fn drop(&mut self) {
        self.0.in_flight.fetch_sub(1, Ordering::SeqCst);
    }
}

/// `WedgeVault`: a delegating vault whose policy is the row's [`Ctl`].
struct WedgeVault {
    inner: Arc<dyn KeychainPort>,
    ns: String,
    ctl: Arc<Ctl>,
}

impl WedgeVault {
    fn enter(&self, op: &'static str) -> Result<InFlight<'_>, WalletError> {
        let now = self.ctl.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.ctl.max_in_flight.fetch_max(now, Ordering::SeqCst);
        let guard = InFlight(&self.ctl);
        self.ctl
            .entered
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push((op, self.ns.clone()));
        self.ctl.entered_cv.notify_all();
        if self.ctl.panic_on == Some(op) {
            panic!("the vault call `{op}` panicked (planted)");
        }
        if let Some(d) = self.ctl.slow {
            std::thread::sleep(d);
        }
        if self.ctl.wedge == Some(op) {
            self.ctl.gate.wait();
        }
        if self.ctl.locked_on == Some(op) {
            return Err(WalletError::KeystoreUnavailable);
        }
        Ok(guard)
    }
}

impl KeychainPort for WedgeVault {
    fn probe(&self) -> Result<(), WalletError> {
        let _g = self.enter("probe")?;
        self.inner.probe()
    }

    fn tier(&self) -> Result<VaultTier, WalletError> {
        let _g = self.enter("tier")?;
        self.inner.tier()
    }

    fn store_wrap_key(
        &self,
        key: SealKey,
        sealed_blob: &[u8],
    ) -> Result<WrapArtifact, WalletError> {
        let _g = self.enter("store_wrap_key")?;
        self.inner.store_wrap_key(key, sealed_blob)
    }

    fn load_wrap_key(
        &self,
        artifact: &WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<SealKey, WalletError> {
        let _g = self.enter("load_wrap_key")?;
        self.inner.load_wrap_key(artifact, sealed_blob)
    }

    fn rotate_wrap_key(
        &self,
        artifact: &WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<WrapArtifact, WalletError> {
        let _g = self.enter("rotate_wrap_key")?;
        self.inner.rotate_wrap_key(artifact, sealed_blob)
    }

    fn finish_rotation(&self, old: &WrapArtifact, new: &WrapArtifact) -> Result<(), WalletError> {
        let _g = self.enter("finish_rotation")?;
        self.inner.finish_rotation(old, new)
    }

    fn delete_wrap_key(&self, artifact: &WrapArtifact) -> Result<(), WalletError> {
        let _g = self.enter("delete_wrap_key")?;
        self.inner.delete_wrap_key(artifact)
    }

    fn purge_namespace(&self) -> Result<usize, WalletError> {
        let _g = self.enter("purge_namespace")?;
        match self.ctl.purge {
            Purge::Real => self.inner.purge_namespace(),
            Purge::Locked => Ok(0),
        }
    }

    fn store_index(&self, entry: &CustodyIndexEntry) -> Result<(), WalletError> {
        let _g = self.enter("store_index")?;
        self.inner.store_index(entry)
    }

    fn load_index(&self) -> Result<Option<CustodyIndexEntry>, WalletError> {
        let _g = self.enter("load_index")?;
        self.inner.load_index()
    }

    fn delete_index(&self) -> Result<(), WalletError> {
        let _g = self.enter("delete_index")?;
        self.inner.delete_index()
    }
}

/// Per namespace: the shared keychain's vault, inside the fake, inside the
/// decorator — the production stack with the platform vault swapped out.
struct BoundedResolver {
    keychain: SharedKeychain,
    ctl: Arc<Ctl>,
    call_bound: Duration,
    budget: Duration,
}

impl VaultResolver for BoundedResolver {
    fn vault_for<'a>(
        &'a self,
        namespace: &KeychainNamespace,
    ) -> Result<ResolvedVault<'a>, WalletError> {
        let fake: Arc<dyn KeychainPort> = Arc::new(WedgeVault {
            inner: shared_mk(&self.keychain, namespace.as_str()),
            ns: namespace.as_str().to_owned(),
            ctl: self.ctl.clone(),
        });
        Ok(ResolvedVault::Owned(bounded(
            fake,
            self.call_bound,
            self.budget,
        )))
    }
}

fn bounded_resolver(
    keychain: &SharedKeychain,
    ctl: &Arc<Ctl>,
    call_bound: Duration,
    budget: Duration,
) -> Arc<dyn VaultResolver> {
    Arc::new(BoundedResolver {
        keychain: keychain.clone(),
        ctl: ctl.clone(),
        call_bound,
        budget,
    })
}

/// The plain per-namespace resolver over the shared keychain (no decorator):
/// how a row builds its wallet before the key store misbehaves.
struct PlainResolver(SharedKeychain);

impl VaultResolver for PlainResolver {
    fn vault_for<'a>(
        &'a self,
        namespace: &KeychainNamespace,
    ) -> Result<ResolvedVault<'a>, WalletError> {
        Ok(ResolvedVault::Owned(shared_mk(&self.0, namespace.as_str())))
    }
}

fn plain(keychain: &SharedKeychain) -> Arc<dyn VaultResolver> {
    Arc::new(PlainResolver(keychain.clone()))
}

// ───────────────────────────── row helpers ─────────────────────────────

/// A post-S2 wallet, created and closed.
struct Fixture {
    _parent: tempfile::TempDir,
    dir: PathBuf,
    keychain: SharedKeychain,
    path_ns: String,
    id_ns: String,
}

async fn created(label: &str) -> Fixture {
    let keychain = SharedKeychainVault::shared();
    let parent = tempfile::tempdir().expect("parent dir");
    let dir = parent.path().join(label).join("wallet");
    std::fs::create_dir_all(&dir).expect("wallet dir");
    Wallet::create_resolving(
        cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
        raw_seed(),
        plain(&keychain),
    )
    .await
    .expect("create")
    .close()
    .await
    .expect("close the create");
    let id = locator(&dir)
        .custody_id()
        .expect("a post-S2 create frames its identifier");
    let id_ns = crate::custody::namespace_for(&id).as_str().to_owned();
    let path_ns = keychain_namespace_for(&dir).as_str().to_owned();
    assert!(
        keychain.lock().expect("kc").contains_key(&id_ns),
        "the wrap key lives under the identifier's namespace"
    );
    Fixture {
        _parent: parent,
        dir,
        keychain,
        path_ns,
        id_ns,
    }
}

/// A wallet created before S2: its key under the PATH namespace, no
/// identifier in its header (the create's minted id stripped — the S2 rows'
/// own pre-stage edit).
async fn pre_stage(label: &str) -> Fixture {
    let keychain = SharedKeychainVault::shared();
    let parent = tempfile::tempdir().expect("parent dir");
    let dir = parent.path().join(label).join("wallet");
    std::fs::create_dir_all(&dir).expect("wallet dir");
    let path_ns = keychain_namespace_for(&dir).as_str().to_owned();
    Wallet::create_with_vault(
        cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
        raw_seed(),
        shared_mk(&keychain, &path_ns),
    )
    .await
    .expect("create the to-be-pre-stage wallet")
    .close()
    .await
    .expect("close the create");
    let stripped = WrapArtifact::from_freshly_wrapped(locator(&dir).as_bytes().to_vec());
    std::fs::write(
        dir.join(crate::constants::WRAP_ARTIFACT_FILE_NAME),
        stripped.file_bytes(),
    )
    .expect("write the stripped artifact");
    assert!(locator(&dir).custody_id().is_none(), "the pre-stage shape");
    assert!(
        keychain.lock().expect("kc").contains_key(&path_ns),
        "the wrap key lives under the path namespace"
    );
    Fixture {
        _parent: parent,
        dir,
        keychain,
        id_ns: String::new(),
        path_ns,
    }
}

fn locator(dir: &Path) -> WrapArtifact {
    let bytes = std::fs::read(dir.join(crate::constants::WRAP_ARTIFACT_FILE_NAME))
        .expect("wrap.artifact present");
    WrapArtifact::from_bytes(bytes).expect("wrap artifact parses")
}

fn index_at(keychain: &SharedKeychain, path_ns: &str) -> Option<CustodyIndexEntry> {
    shared_mk(keychain, path_ns)
        .load_index()
        .expect("read the index item directly")
}

fn key_live(keychain: &SharedKeychain, ns: &str) -> bool {
    keychain.lock().expect("kc").contains_key(ns)
}

/// Files, header and index item all where the wallet left them.
fn untouched(f: &Fixture, what: &str) {
    assert!(
        f.dir.join(crate::constants::WALLET_DB_FILE_NAME).exists(),
        "{what}: wallet.db still there"
    );
    assert!(
        f.dir
            .join(crate::constants::WRAP_ARTIFACT_FILE_NAME)
            .exists(),
        "{what}: the header (wrap.artifact) still there"
    );
    assert!(
        index_at(&f.keychain, &f.path_ns).is_some(),
        "{what}: the index item still there"
    );
}

/// Run a (possibly wedging) direct port call off the row's thread, and fail
/// — never hang — when it does not answer.
fn answers<T: Send + 'static>(what: &str, f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(ANSWER_CEILING).unwrap_or_else(|_| {
        panic!("{what}: no answer within {ANSWER_CEILING:?} — the call is not bounded")
    })
}

async fn wipe(
    dir: &Path,
    resolver: Arc<dyn VaultResolver>,
    force: bool,
) -> Result<(), WalletError> {
    tokio::time::timeout(
        ANSWER_CEILING,
        Wallet::wipe_resolving(dir, Some(resolver), force),
    )
    .await
    .unwrap_or_else(|_| {
        panic!("the wipe did not answer within {ANSWER_CEILING:?} — the call is not bounded")
    })
}

/// Wait until the worker takes new calls again (a late call finished, or
/// nothing was ever in flight): a probe through the decorator over a plain
/// vault, retried while it answers `busy`.
fn settle() {
    let v = bounded(
        Arc::new(TestVault::new(VaultTier::Tee)),
        Duration::from_secs(10),
        Duration::from_secs(10),
    );
    let started = Instant::now();
    loop {
        match v.probe() {
            Ok(()) => return,
            Err(e)
                if cause(&e).as_deref() == Some("busy") && started.elapsed() < ANSWER_CEILING =>
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => panic!("the worker never came back after the late call: {e:?}"),
        }
    }
}

fn expect_cause(result: Result<(), WalletError>, want: &str, what: &str) -> WalletError {
    match result {
        Err(e) if cause(&e).as_deref() == Some(want) => e,
        Err(e) => panic!("{what}: want KeychainTimeout {{ cause: {want} }}; got {e:?}"),
        Ok(()) => panic!("{what}: want KeychainTimeout {{ cause: {want} }}; got Ok"),
    }
}

// ───────────────────────────── the rows ─────────────────────────────

/// Assertion 1. A wipe whose purge wedges answers `KeychainTimeout { cause:
/// timeout }` while the wedge is held — code `RW-KEY-008` — and leaves the
/// files, the header and the index item where they were (the bridge half,
/// kind `keystoreUnavailable`, is the conversion row in the bridge crate's
/// `convert.rs`).
#[tokio::test]
async fn a_wedged_purge_answers_the_wipe_typed_inside_the_budget() {
    let _serial = serial();
    settle();
    let f = created("row1").await;
    let ctl = Arc::new(Ctl::wedging("purge_namespace"));
    let _release = ReleaseOnDrop(ctl.clone());

    let started = Instant::now();
    let r = wipe(
        &f.dir,
        bounded_resolver(&f.keychain, &ctl, BOUND, BUDGET),
        false,
    )
    .await;
    let waited = started.elapsed();
    let e = expect_cause(r, "timeout", "a wipe whose purge is wedged");
    println!(
        "wedged purge: the wipe answered after {waited:?} (bound {BOUND:?}, budget {BUDGET:?})"
    );
    assert_eq!(e.code(), "RW-KEY-008", "the timeout's own code");
    assert_eq!(
        ctl.count("purge_namespace"),
        1,
        "the purge reached the vault and wedged there"
    );

    untouched(&f, "after the timed-out wipe");
    assert!(
        key_live(&f.keychain, &f.id_ns),
        "the wedged purge has severed nothing yet"
    );
    assert!(
        locator(&f.dir).custody_id().is_some(),
        "the header still names the wallet's identifier"
    );

    ctl.gate.release();
    settle();
}

/// Assertion 2. While the first wipe's purge is still wedged, a second wipe
/// answers `cause: busy` at once and never reaches the vault; the fake never
/// holds two calls.
#[tokio::test]
async fn a_call_while_one_is_wedged_answers_at_once_and_reaches_no_vault() {
    let _serial = serial();
    settle();
    let f = created("row2").await;
    let ctl = Arc::new(Ctl::wedging("purge_namespace"));
    let _release = ReleaseOnDrop(ctl.clone());
    let resolver = bounded_resolver(&f.keychain, &ctl, BOUND, BUDGET);

    let first = wipe(&f.dir, resolver.clone(), false).await;
    expect_cause(first, "timeout", "the first wipe");
    let before = ctl.calls().len();

    let started = Instant::now();
    let second = wipe(&f.dir, resolver.clone(), false).await;
    println!(
        "second wipe while wedged answered after {:?}",
        started.elapsed()
    );
    expect_cause(
        second,
        "busy",
        "the second wipe while the purge is still wedged",
    );
    assert_eq!(
        ctl.calls().len(),
        before,
        "the busy answer reached no vault: {:?}",
        ctl.calls()
    );
    assert_eq!(
        ctl.max_in_flight.load(Ordering::SeqCst),
        1,
        "never two native calls at once"
    );
    untouched(&f, "after the busy wipe");

    ctl.gate.release();
    settle();
    assert_eq!(
        ctl.max_in_flight.load(Ordering::SeqCst),
        1,
        "never two native calls at once"
    );
}

/// Assertion 3. A job still QUEUED when its caller's bound expires is never
/// run: the caller answers `timeout`, and after the wedge in front of it
/// clears, the vault never sees that call.
#[test]
fn a_queued_call_whose_caller_left_never_runs() {
    let _serial = serial();
    settle();
    let ctl = Arc::new(Ctl::wedging("load_index"));
    let _release = ReleaseOnDrop(ctl.clone());
    let kc = SharedKeychainVault::shared();
    let fake = |ns: &str| -> Arc<dyn KeychainPort> {
        Arc::new(WedgeVault {
            inner: shared_mk(&kc, ns),
            ns: ns.to_owned(),
            ctl: ctl.clone(),
        })
    };
    let ns = "0123456789abcdef0123456789abcdef";

    // In front: a caller that stays, its call running and wedged (NOT
    // abandoned, so the next submission queues instead of answering busy).
    let stayer = bounded(fake(ns), LONG, LONG);
    let running = std::thread::spawn(move || stayer.load_index().map(|_| ()));
    ctl.wait_entered("load_index");

    // Behind it: a caller that leaves.
    let leaver = bounded(fake(ns), SHORT, SHORT);
    let left = answers("the queued caller", move || leaver.tier().map(|_| ()));
    expect_cause(
        left,
        "timeout",
        "a caller whose bound expired while its job was queued",
    );

    ctl.gate.release();
    running
        .join()
        .expect("the stayer's thread")
        .expect("the stayer's call completes once the wedge clears");
    settle();
    assert_eq!(
        ctl.count("tier"),
        0,
        "the abandoned queued job never ran: {:?}",
        ctl.calls()
    );
}

/// Assertion 4. The wedged purge is released and severs AFTER its wipe was
/// told `timeout`; the re-run wipe converges (the directory and the index item
/// gone), and a third wipe is the no-op `Ok`.
#[tokio::test]
async fn a_purge_that_lands_after_its_timeout_lets_the_rerun_converge() {
    let _serial = serial();
    settle();
    let f = created("row4").await;
    let ctl = Arc::new(Ctl::wedging("purge_namespace"));
    let _release = ReleaseOnDrop(ctl.clone());
    let resolver = bounded_resolver(&f.keychain, &ctl, BOUND, BUDGET);

    expect_cause(
        wipe(&f.dir, resolver.clone(), false).await,
        "timeout",
        "the wedged wipe",
    );
    ctl.gate.release();
    settle();
    assert!(
        !key_live(&f.keychain, &f.id_ns),
        "the late purge landed and severed the wallet's key"
    );
    untouched(&f, "the late purge deletes wrap material only");

    wipe(&f.dir, resolver.clone(), false)
        .await
        .expect("the re-run converges on the late sever, without wipe_force");
    assert!(!f.dir.exists(), "the directory is gone");
    assert!(
        index_at(&f.keychain, &f.path_ns).is_none(),
        "the index item is gone"
    );
    wipe(&f.dir, resolver, false)
        .await
        .expect("a third wipe is the no-op success");
}

/// Assertion 5. A TIMED-OUT index read is never read as "no index". At the
/// wipe with the directory present — `force` false and `force` true — the
/// wipe answers `KeychainTimeout`, no purge runs under the PATH namespace,
/// and the index item survives the forced call. The same at the wipe's
/// `header_unseals` and at the open's `read_index` (a pre-stage wallet's
/// migration mints nothing). At `settle_index_after_open` a timed-out read
/// leaves a `pending` index `pending`. A LOCKED keychain (a plain
/// `KeystoreUnavailable`, not a timeout) still fails open exactly as at
/// base: the wipe reads the index as absent, the header proves itself, the
/// wallet is wiped.
///
/// Also the arm the contract's item 6 names beyond the floor: `force` with
/// the directory GONE (security MEDIUM-3) — an absorbed timeout there would
/// end in `delete_index` erasing the only record of the namespace.
#[tokio::test]
async fn a_keychain_timeout_is_never_read_as_an_absent_index() {
    let _serial = serial();
    settle();

    // ── the wipe's index read, directory present, force false then true ──
    for force in [false, true] {
        let f = created(if force { "row5-forced" } else { "row5" }).await;
        let ctl = Arc::new(Ctl::wedging("load_index"));
        let _release = ReleaseOnDrop(ctl.clone());
        let r = wipe(
            &f.dir,
            bounded_resolver(&f.keychain, &ctl, BOUND, BUDGET),
            force,
        )
        .await;
        expect_cause(
            r,
            "timeout",
            &format!("a wedged index read at the wipe (force={force})"),
        );
        ctl.gate.release();
        settle();
        assert_eq!(
            ctl.count_in("purge_namespace", &f.path_ns),
            0,
            "force={force}: no purge under the PATH namespace: {:?}",
            ctl.calls()
        );
        assert_eq!(
            ctl.count("delete_index"),
            0,
            "force={force}: the index was not deleted"
        );
        untouched(&f, &format!("force={force}"));
        assert!(
            key_live(&f.keychain, &f.id_ns),
            "force={force}: nothing severed"
        );
    }

    // ── force with the directory gone ──
    {
        let f = created("row5-bare-forced").await;
        std::fs::remove_dir_all(&f.dir).expect("the host deleted the files");
        let ctl = Arc::new(Ctl::wedging("load_index"));
        let _release = ReleaseOnDrop(ctl.clone());
        let r = wipe(
            &f.dir,
            bounded_resolver(&f.keychain, &ctl, BOUND, BUDGET),
            true,
        )
        .await;
        expect_cause(
            r,
            "timeout",
            "a wedged index read at a forced bare-config wipe",
        );
        ctl.gate.release();
        settle();
        assert_eq!(
            ctl.count("delete_index"),
            0,
            "the only record of the namespace survives"
        );
        assert!(
            index_at(&f.keychain, &f.path_ns).is_some(),
            "the index item is still there"
        );
        assert!(key_live(&f.keychain, &f.id_ns), "nothing severed");
    }

    // ── the wipe's header_unseals: the index does not corroborate the
    // header (it is absent), so the header must prove itself by unsealing —
    // and that unwrap wedges ──
    {
        let f = created("row5-header").await;
        shared_mk(&f.keychain, &f.path_ns)
            .delete_index()
            .expect("drop the index item so the header must prove itself");
        let ctl = Arc::new(Ctl::wedging("load_wrap_key"));
        let _release = ReleaseOnDrop(ctl.clone());
        let r = wipe(
            &f.dir,
            bounded_resolver(&f.keychain, &ctl, BOUND, BUDGET),
            false,
        )
        .await;
        expect_cause(r, "timeout", "a wedged unwrap at the wipe's header proof");
        ctl.gate.release();
        settle();
        assert_eq!(
            ctl.count("purge_namespace"),
            0,
            "no purge after a timed-out header proof: {:?}",
            ctl.calls()
        );
        assert!(
            f.dir.join(crate::constants::WALLET_DB_FILE_NAME).exists(),
            "files untouched"
        );
        assert!(key_live(&f.keychain, &f.id_ns), "nothing severed");
    }

    // ── the open's read_index: a pre-stage wallet's migration ──
    {
        let f = pre_stage("row5-open").await;
        let before = index_at(&f.keychain, &f.path_ns);
        let ctl = Arc::new(Ctl::wedging("load_index"));
        let _release = ReleaseOnDrop(ctl.clone());
        let opened = tokio::time::timeout(
            ANSWER_CEILING,
            Wallet::open_resolving(
                cfg(&f.dir, Network::Test, SeedPersistence::SealedKeychain),
                bounded_resolver(&f.keychain, &ctl, BOUND, BUDGET),
                None,
                None,
            ),
        )
        .await
        .expect("the open did not answer — the call is not bounded");
        match opened {
            Err(e) if cause(&e).as_deref() == Some("timeout") => {}
            Err(e) => {
                panic!("a wedged index read at the migration: want KeychainTimeout; got {e:?}")
            }
            Ok(w) => {
                let _ = w.close().await;
                panic!("a wedged index read at the migration must refuse the open, not mint on it");
            }
        }
        ctl.gate.release();
        settle();
        assert_eq!(
            ctl.count("store_index"),
            0,
            "no index was written: {:?}",
            ctl.calls()
        );
        assert_eq!(ctl.count("store_wrap_key"), 0, "no key was re-custodied");
        assert!(locator(&f.dir).custody_id().is_none(), "nothing committed");
        assert!(
            index_at(&f.keychain, &f.path_ns) == before,
            "the index item is exactly as before the open"
        );
        assert_eq!(
            f.keychain.lock().expect("kc").len(),
            1,
            "no identifier namespace was minted"
        );
    }

    // ── settle_index_after_open: a pending index stays pending ──
    {
        let f = pre_stage("row5-settle").await;
        Wallet::open_resolving(
            cfg(&f.dir, Network::Test, SeedPersistence::SealedKeychain),
            plain(&f.keychain),
            None,
            None,
        )
        .await
        .expect("the committing open")
        .close()
        .await
        .expect("close the committing open");
        assert!(
            index_at(&f.keychain, &f.path_ns).is_some_and(|ix| ix.is_pending()),
            "the migration committed and its index is pending"
        );
        let ctl = Arc::new(Ctl::wedging("load_index"));
        let _release = ReleaseOnDrop(ctl.clone());
        // The load already proved the wallet opens; settle's bookkeeping is
        // best-effort and never refuses a committed wallet's open.
        tokio::time::timeout(
            ANSWER_CEILING,
            Wallet::open_resolving(
                cfg(&f.dir, Network::Test, SeedPersistence::SealedKeychain),
                bounded_resolver(&f.keychain, &ctl, BOUND, BUDGET),
                None,
                None,
            ),
        )
        .await
        .expect("the open did not answer — the call is not bounded")
        .expect("a timed-out read at settle does not refuse the open")
        .close()
        .await
        .expect("close");
        ctl.gate.release();
        settle();
        assert!(
            index_at(&f.keychain, &f.path_ns).is_some_and(|ix| ix.is_pending()),
            "a timed-out read at settle leaves the pending index pending"
        );
        assert!(
            key_live(&f.keychain, &f.path_ns),
            "and the legacy key is not stranded behind a `done` index"
        );
    }

    // ── the pin: a LOCKED keychain fails open exactly as at base ──
    {
        let f = created("row5-locked").await;
        let ctl = Arc::new(Ctl {
            locked_on: Some("load_index"),
            ..Ctl::new()
        });
        wipe(
            &f.dir,
            bounded_resolver(&f.keychain, &ctl, BOUND, BUDGET),
            false,
        )
        .await
        .expect(
            "a locked index read still fails open: the header proves itself and the wipe completes",
        );
        assert!(
            !key_live(&f.keychain, &f.id_ns),
            "the wallet's key was severed"
        );
        assert!(!f.dir.exists(), "the files are gone");
    }
}

/// Assertion 6. The budget bounds a SLOW keychain, not only a wedged one:
/// every call takes 60% of the per-call bound (so no call alone times out)
/// and the wipe still answers `KeychainTimeout` by the injected budget.
///
/// Counted calls on the wipe path this row drives (a post-S2 wallet whose
/// index corroborates its header, directory present): `load_index`,
/// `purge_namespace`, `delete_index` — three. At 600 ms each against a
/// 900 ms budget the purge is the call the budget cuts (600 + 600 > 900);
/// with no budget the wipe would answer `Ok` after ~1.8 s.
#[tokio::test]
async fn the_wipe_budget_bounds_a_slow_keychain_not_only_a_wedged_one() {
    let _serial = serial();
    settle();
    const CALL: Duration = Duration::from_millis(1_000);
    const SLOW: Duration = Duration::from_millis(600);
    const SLOW_BUDGET: Duration = Duration::from_millis(900);
    assert!(
        SLOW < CALL && SLOW * 2 > SLOW_BUDGET && SLOW < SLOW_BUDGET,
        "the fixture's pricing"
    );

    let f = created("row6").await;
    let ctl = Arc::new(Ctl {
        slow: Some(SLOW),
        ..Ctl::new()
    });
    let started = Instant::now();
    let r = wipe(
        &f.dir,
        bounded_resolver(&f.keychain, &ctl, CALL, SLOW_BUDGET),
        false,
    )
    .await;
    let waited = started.elapsed();
    println!(
        "slow keychain: the wipe answered after {waited:?}; calls reached: {:?}",
        ctl.calls().iter().map(|(op, _)| *op).collect::<Vec<_>>()
    );
    match r {
        Err(ref e) if cause(e).is_some() => {}
        other => panic!("a slow keychain must still exhaust the wipe's budget; got {other:?}"),
    }
    assert!(
        f.dir.join(crate::constants::WALLET_DB_FILE_NAME).exists(),
        "files untouched"
    );
    settle();
}

/// Assertion 7. A late purge converges on EVIDENCE, never on a zero count.
/// (a) The shape of row 4 with the severed-late set cleared (a restart):
/// the re-run answers `KeystoreInconsistent`, files untouched.
/// (b) The index read succeeds, then a "locked" purge answers `Ok(0)`: the
/// wipe answers `KeystoreInconsistent` and deletes nothing.
/// (c) A late `Ok(0)` puts nothing in the set: the re-run still refuses.
#[tokio::test]
async fn a_late_purge_converges_on_evidence_never_on_a_zero_count() {
    let _serial = serial();
    settle();

    // (a)
    {
        let f = created("row7a").await;
        let ctl = Arc::new(Ctl::wedging("purge_namespace"));
        let _release = ReleaseOnDrop(ctl.clone());
        let resolver = bounded_resolver(&f.keychain, &ctl, BOUND, BUDGET);
        expect_cause(
            wipe(&f.dir, resolver.clone(), false).await,
            "timeout",
            "(a) the wedged wipe",
        );
        ctl.gate.release();
        settle();
        assert!(
            !key_live(&f.keychain, &f.id_ns),
            "(a) the late purge severed"
        );
        forget_late_severs();
        match wipe(&f.dir, resolver, false).await {
            Err(WalletError::KeystoreInconsistent { .. }) => {}
            other => panic!("(a) with the evidence gone the re-run must refuse; got {other:?}"),
        }
        untouched(&f, "(a) the refused re-run");
    }

    // (b)
    {
        let f = created("row7b").await;
        let ctl = Arc::new(Ctl {
            purge: Purge::Locked,
            ..Ctl::new()
        });
        match wipe(
            &f.dir,
            bounded_resolver(&f.keychain, &ctl, BOUND, BUDGET),
            false,
        )
        .await
        {
            Err(WalletError::KeystoreInconsistent { .. }) => {}
            other => panic!(
                "(b) a purge that severed 0 while the key is live must refuse; got {other:?}"
            ),
        }
        assert_eq!(
            ctl.count("purge_namespace"),
            1,
            "(b) the purge ran and answered Ok(0)"
        );
        untouched(&f, "(b)");
        assert!(key_live(&f.keychain, &f.id_ns), "(b) the key is still live");
    }

    // (c)
    {
        let f = created("row7c").await;
        let ctl = Arc::new(Ctl {
            wedge: Some("purge_namespace"),
            purge: Purge::Locked,
            ..Ctl::new()
        });
        let _release = ReleaseOnDrop(ctl.clone());
        let resolver = bounded_resolver(&f.keychain, &ctl, BOUND, BUDGET);
        expect_cause(
            wipe(&f.dir, resolver.clone(), false).await,
            "timeout",
            "(c) the wedged wipe",
        );
        ctl.gate.release();
        settle();
        assert!(
            key_live(&f.keychain, &f.id_ns),
            "(c) the late purge answered Ok(0) and severed nothing"
        );
        match wipe(&f.dir, resolver, false).await {
            Err(WalletError::KeystoreInconsistent { .. }) => {}
            other => {
                panic!("(c) a late Ok(0) is no evidence; the re-run must refuse; got {other:?}")
            }
        }
        untouched(&f, "(c)");
        assert!(key_live(&f.keychain, &f.id_ns), "(c) the key is still live");
    }
}

/// Assertion 12. An abandoned queued job holds no key: a `store_wrap_key`
/// queued behind a wedge gives its inputs up at abandonment, while the wedge
/// is still held. Positive control first: a `store_wrap_key` queued by a
/// caller that STAYS is seen holding its payload, so the probe is not
/// vacuously zero.
#[test]
fn an_abandoned_queued_job_drops_its_key_at_abandonment() {
    let _serial = serial();
    settle();
    let ctl = Arc::new(Ctl::wedging("load_index"));
    let _release = ReleaseOnDrop(ctl.clone());
    let kc = SharedKeychainVault::shared();
    let fake = |ns: &str| -> Arc<dyn KeychainPort> {
        Arc::new(WedgeVault {
            inner: shared_mk(&kc, ns),
            ns: ns.to_owned(),
            ctl: ctl.clone(),
        })
    };
    let ns = "fedcba9876543210fedcba9876543210";
    assert_eq!(payloads_held(), 0, "an idle worker holds no payload");

    let stayer = bounded(fake(ns), LONG, LONG);
    let running = std::thread::spawn(move || stayer.load_index().map(|_| ()));
    ctl.wait_entered("load_index");

    let queued = bounded(fake(ns), LONG, LONG);
    let staying_wrap =
        std::thread::spawn(move || queued.store_wrap_key(SealKey::generate(), BLOB).map(|_| ()));
    let started = Instant::now();
    while payloads_held() < 1 {
        assert!(
            started.elapsed() < ANSWER_CEILING,
            "the queued wrap never showed its payload"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        payloads_held(),
        1,
        "positive control: one queued wrap, holding its key"
    );

    let leaver = bounded(fake(ns), SHORT, SHORT);
    let left = answers("the abandoned wrap", move || {
        leaver.store_wrap_key(SealKey::generate(), BLOB).map(|_| ())
    });
    expect_cause(
        left,
        "timeout",
        "a wrap whose caller left while it was queued",
    );
    assert_eq!(
        payloads_held(),
        1,
        "the abandoned wrap's key is gone at abandonment, with the wedge still held \
         (only the staying caller's payload remains)"
    );
    assert_eq!(ctl.count("store_wrap_key"), 0, "no wrap has run yet");

    ctl.gate.release();
    running
        .join()
        .expect("the stayer")
        .expect("the stayer's read completes");
    staying_wrap
        .join()
        .expect("the staying wrap")
        .expect("the staying caller's wrap runs once the wedge clears");
    settle();
    assert_eq!(
        ctl.count("store_wrap_key"),
        1,
        "only the staying caller's wrap ever ran: {:?}",
        ctl.calls()
    );
    assert_eq!(payloads_held(), 0, "nothing left holding a payload");
}

/// Assertion 8. However many wipes, one wedged native call and one worker:
/// 50 wipes against a permanently wedged key store answer typed, the first
/// by its bound and the rest at once, while the fake sees ONE call and the
/// process spawned ONE worker.
#[tokio::test]
async fn a_permanently_wedged_keychain_holds_one_call_and_one_thread() {
    let _serial = serial();
    settle();
    let f = created("row8").await;
    let ctl = Arc::new(Ctl::wedging("load_index"));
    let _release = ReleaseOnDrop(ctl.clone());
    let resolver = bounded_resolver(&f.keychain, &ctl, BOUND, BUDGET);

    let started = Instant::now();
    for attempt in 0..50 {
        let r = wipe(&f.dir, resolver.clone(), false).await;
        let want = if attempt == 0 { "timeout" } else { "busy" };
        expect_cause(r, want, &format!("wipe #{}", attempt + 1));
    }
    println!(
        "50 wipes against a wedged key store answered in {:?}",
        started.elapsed()
    );
    assert_eq!(
        ctl.calls().len(),
        1,
        "one native call ever reached the vault: {:?}",
        ctl.calls()
    );
    assert_eq!(worker_spawns(), 1, "one worker thread for the process");
    assert_eq!(ctl.max_in_flight.load(Ordering::SeqCst), 1);

    ctl.gate.release();
    settle();
    assert_eq!(
        worker_spawns(),
        1,
        "still one worker after the wedge clears"
    );
    wipe(&f.dir, resolver, false)
        .await
        .expect("once the key store answers, the wipe converges");
    assert!(!f.dir.exists());
}

/// Assertion 9. A `store_wrap_key` that wedges past the bound fails the
/// create `KeychainTimeout`; it is released and lands (a wrap key under the
/// first attempt's fresh identifier — the stated orphan); a retried create
/// makes a wallet that opens, and a wipe of it converges.
#[tokio::test]
async fn a_wrap_that_lands_after_create_timed_out_leaves_a_retry_that_opens() {
    let _serial = serial();
    settle();
    let keychain = SharedKeychainVault::shared();
    let parent = tempfile::tempdir().expect("parent dir");
    let dir = parent.path().join("row9").join("wallet");
    std::fs::create_dir_all(&dir).expect("wallet dir");
    let ctl = Arc::new(Ctl::wedging("store_wrap_key"));
    let _release = ReleaseOnDrop(ctl.clone());
    let resolver = bounded_resolver(&keychain, &ctl, BOUND, BUDGET);

    let first = tokio::time::timeout(
        ANSWER_CEILING,
        Wallet::create_resolving(
            cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
            raw_seed(),
            resolver.clone(),
        ),
    )
    .await
    .expect("the create did not answer — the call is not bounded");
    match first {
        Err(e) if cause(&e).as_deref() == Some("timeout") => {}
        Err(e) => panic!("a wedged wrap must fail the create KeychainTimeout; got {e:?}"),
        Ok(w) => {
            let _ = w.close().await;
            panic!("a wedged wrap must fail the create");
        }
    }
    assert!(
        !dir.join(crate::constants::SEED_SEAL_FILE_NAME).exists(),
        "the failed create wrote no sealed blob"
    );

    ctl.gate.release();
    settle();
    println!(
        "the late wrap landed: {} key item(s) in the keychain before the retry",
        keychain.lock().expect("kc").len()
    );

    Wallet::create_resolving(
        cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
        raw_seed(),
        resolver.clone(),
    )
    .await
    .expect("the retried create")
    .close()
    .await
    .expect("close the retry");
    Wallet::open_resolving(
        cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
        resolver.clone(),
        None,
        None,
    )
    .await
    .expect("the retried create's wallet opens")
    .close()
    .await
    .expect("close the open");
    let id_ns = crate::custody::namespace_for(
        &locator(&dir)
            .custody_id()
            .expect("the retry framed its identifier"),
    )
    .as_str()
    .to_owned();

    wipe(&dir, resolver, false)
        .await
        .expect("a wipe of it converges");
    assert!(!dir.exists());
    assert!(
        !key_live(&keychain, &id_ns),
        "the live wallet's key is severed"
    );
}

/// Assertion 10. A panic inside a vault call answers `KeystoreUnavailable`
/// (a failure, not a timeout), the next call runs, and the worker was not
/// replaced.
#[test]
fn a_panicking_vault_call_does_not_kill_the_worker() {
    let _serial = serial();
    settle();
    let ctl = Arc::new(Ctl {
        panic_on: Some("tier"),
        ..Ctl::new()
    });
    let kc = SharedKeychainVault::shared();
    let ns = "00112233445566778899aabbccddeeff";
    let v = bounded(
        Arc::new(WedgeVault {
            inner: shared_mk(&kc, ns),
            ns: ns.to_owned(),
            ctl: ctl.clone(),
        }),
        BOUND,
        BUDGET,
    );
    let spawns = worker_spawns();

    let v1 = v.clone();
    match answers("the panicking call", move || v1.tier().map(|_| ())) {
        Err(WalletError::KeystoreUnavailable) => {}
        other => panic!("a panicking vault call answers KeystoreUnavailable; got {other:?}"),
    }
    let v2 = v.clone();
    answers("the call after the panic", move || v2.probe()).expect("the next call runs");
    assert_eq!(ctl.count("probe"), 1, "the next call reached the vault");
    assert_eq!(
        worker_spawns(),
        spawns,
        "the worker survived; none was spawned in its place"
    );
    assert_eq!(worker_spawns(), 1);
}

/// Assertion 11. The constants at their boundary: two wipe attempts and a
/// second of file sweep fit the HOST's 10 s (Relim's panic-wipe budget, D14
/// clause 7 — their number, cited), and the per-call bound stays under the
/// SDK UI's own Dart belt, `FrbWalletProvisioner.defaultLocalIoBound =
/// Duration(seconds: 30)` (`sdk/zec_wallet_ui/lib/features/wallet/onboarding/
/// frb_wallet_provisioner.dart`, quoted at `5e1ec24e`; the honest
/// cross-language check is the deferred item 4).
#[test]
fn the_wipe_budget_fits_two_attempts_in_the_hosts_ten_seconds() {
    const HOST_PANIC_WIPE_BUDGET: Duration = Duration::from_secs(10);
    const DART_LOCAL_IO_BOUND: Duration = Duration::from_secs(30);
    assert!(
        KEYCHAIN_WIPE_BUDGET * 2 + Duration::from_secs(1) <= HOST_PANIC_WIPE_BUDGET,
        "two wipe attempts plus the sweep fit the host's budget: {KEYCHAIN_WIPE_BUDGET:?}"
    );
    assert!(
        KEYCHAIN_CALL_BOUND < DART_LOCAL_IO_BOUND,
        "the per-call bound answers before the Dart belt: {KEYCHAIN_CALL_BOUND:?}"
    );
    assert!(KEYCHAIN_WIPE_BUDGET > Duration::ZERO && KEYCHAIN_CALL_BOUND > Duration::ZERO);
}

/// UNLISTED (IT-1 +A) — the contract's LOG clause, which its assertion list
/// does not name: a timed-out call and a busy one each leave one
/// `wallet.vault_call` line carrying `outcome` and `op` (revision 3's closed
/// vocabulary), and neither line carries the namespace (§5.4). A bound
/// that answered in time but said nothing would leave the exit walk's
/// "the wipe's measured key-store time in the device log" with nothing to
/// read.
#[test]
fn a_bounded_call_that_times_out_says_so_in_one_payload_free_line() {
    use crate::tracing_guard::{CaptureLayer, CapturedEvents, assert_5_4_clean};
    use tracing_subscriber::prelude::*;

    let _serial = serial();
    settle();
    let ctl = Arc::new(Ctl::wedging("load_index"));
    let _release = ReleaseOnDrop(ctl.clone());
    let kc = SharedKeychainVault::shared();
    let ns = "aabbccddeeff00112233445566778899";
    let v = bounded(
        Arc::new(WedgeVault {
            inner: shared_mk(&kc, ns),
            ns: ns.to_owned(),
            ctl: ctl.clone(),
        }),
        SHORT,
        SHORT,
    );

    crate::tracing_guard::force_wallet_callsites_enabled();
    let sink = CapturedEvents::default();
    let (first, second) = {
        let sink = sink.clone();
        answers("the logged calls", move || {
            let subscriber = tracing_subscriber::registry().with(CaptureLayer::new(sink));
            let _guard = tracing::subscriber::set_default(subscriber);
            let first = v.load_index().map(|_| ());
            let second = v.load_index().map(|_| ());
            (first, second)
        })
    };
    expect_cause(first, "timeout", "the wedged read");
    expect_cause(second, "busy", "the read behind it");
    ctl.gate.release();
    settle();

    let lines = sink.records_of("wallet.vault_call");
    let field = |line: &Vec<(String, String)>, name: &str| {
        line.iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.trim_matches('"').to_owned())
    };
    for outcome in ["timeout", "busy"] {
        let n = lines
            .iter()
            .filter(|l| {
                field(l, "outcome").as_deref() == Some(outcome)
                    && field(l, "op").as_deref() == Some("load_index")
            })
            .count();
        assert_eq!(
            n, 1,
            "one `{outcome}` line for `load_index`; captured: {lines:?}"
        );
    }
    let fields = sink.fields();
    assert!(
        fields.iter().all(|(_, v)| !v.contains(ns)),
        "no line carries the namespace: {fields:?}"
    );
    assert_5_4_clean(&fields);
}
