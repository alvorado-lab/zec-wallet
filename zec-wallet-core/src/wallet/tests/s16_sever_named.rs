//! Stage S16 `sever` + `poison` (FR-53) — the test author's rows against the
//! names the contract says the implementer ADDS. Written BLIND (IT-2a) against
//! `2a8223f4`, contract `docs/plan/stage-16-the-duress-force-sever.md` §3.1 and
//! §3.2 (revision 5). A child of `wallet::tests` for its fixtures, in its own
//! file so no cited line in `wallet.rs` moves; declared `#[cfg(any())]` there
//! (never built) until the adjudicator joins it (the `s9_bound` shape).
//!
//! COUPLING DECLARED FOR THE JOIN — every name in the "declared names" section
//! below is this author's spelling of the contract's words, and that section is
//! the ONLY place they are spelled:
//!
//!   * `Wallet::sever_custody_resolving(db_dir: &Path, resolver:
//!     Option<Arc<dyn VaultResolver>>, deadline: Duration) ->
//!     Result<SeverReport, WalletError>` — seam (ii), mirroring
//!     `wipe_resolving` (`None` = the vault-absent platform).
//!   * `SeverReport { severed: SeverOutcome, holder: HolderSeen, files:
//!     FilesOutcome }`; `SeverOutcome::{Severed { count }, SeveredUnproven {
//!     reason }, AlreadyGone, NotSevered { cause }}`; `HolderSeen::{None,
//!     ThisProcess, OtherProcess}`; `FilesOutcome::{Removed, LeftForHost}` —
//!     imported from the crate root (`crate::{…}`, re-exported from
//!     `sever.rs`). `reason` and `cause` are read through their `Debug`
//!     rendering, normalised (`CountUnreadable` → "countunreadable",
//!     `KeystoreUnavailable` → "keystoreunavailable"), so their types are not
//!     named here; `count` is a `usize`, read as it is.
//!   * `BoundedVault::with_bounds(inner, call_bound, wipe_budget, namespace:
//!     Option<KeychainNamespace>)` — seam (iii)'s new LAST argument.
//!   * `crate::keychain::bounded::wipe_scope_until(end: Instant)` — §3.1
//!     item 5, returning a drop guard.
//!   * `crate::keychain::bounded::reset_paths()` — seam (iii)'s reset.
//!   * `crate::keychain::bounded::path_entry(&KeychainNamespace) ->
//!     Option<(live, opening, tombstoned)>` — live `Weak`s, openers, the
//!     tombstone: §3.2's "the table's cfg(test) count accessor", read here
//!     into [`Entry`].
//!   * `crate::wallet::sever_seams::{arm, Seam}` — §3.2's pauses and broadcast
//!     hook, as ONE process-wide table keyed by (seam, `db_dir`) whose hook
//!     fires ONCE. [`Armed::new`]`(point, dir, f)` re-arms after each firing
//!     for as long as the value lives, so a row's hook runs every time the
//!     point is passed at `dir`. This file's points map onto the built ones,
//!     and each sits:
//!       - `Seam::OpenAfterAcquire` (`AfterAcquireOpening`) — an open, right
//!         after `acquire_opening`, before the tombstone check that refuses
//!         `Wiped` early (§3.2 assertion 6(a));
//!       - `Seam::OpenBeforeBirthGate` (`BeforeBirthGate`) — inside
//!         `from_open_carrying`, after `Arc::new_cyclic`, before the birth
//!         gate takes `PATHS` (6(b));
//!       - `Seam::RebuildBeforeAssemble` (`AfterUnwrap`) — the switch right
//!         after its drain handed it the sole `Inner`; the rescan AFTER its
//!         rebuild has re-opened the store (the store open fails first on
//!         purged custody, so the pause cannot sit before it), and before
//!         their `from_open_carrying` (assertion 4's "(ii) after its unwrap");
//!       - `Seam::BroadcastTop` / `Seam::BroadcastBeforeSend`
//!         (`BroadcastBeforeFirstCheck` / `BroadcastBeforeSendCheck`) —
//!         immediately BEFORE `broadcast_one`'s top check and its check
//!         between the connect and `send_transaction` (assertion 7).
//!   * The migration hook (seam i) and the new `ProvisionStep` (seam iv) are
//!     NOT used: rows 6 and 7 pause on the CALLER's thread, in a wrapper
//!     OUTSIDE the bounded decorator (`Pausing`), right after the custody call
//!     that opens the window returns — no store seam is needed, and the worker
//!     still runs every call the production way.
//!
//! Every row serialises on one lock: the key-store worker, the path table and
//! the hook table are process-wide. A row that wedges the worker waits for it
//! to take calls again (`settle`) before it ends. No row sleeps to make its
//! claim; `ANSWER_CEILING` exists only so a build without a bound FAILS
//! instead of hanging the suite.

#![allow(clippy::await_holding_lock)] // the serial guard is held across awaits on purpose

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use super::delivery_obligation::{
    Answer, LoopbackLightwalletd, config_over, funded_wallet, persist_shield,
};
use super::s16_sever::fund_another_utxo;
use super::*;
use crate::custody::{CustodyId, CustodyIndexEntry};
use crate::keychain::testvault::SharedKeychain;
use crate::keychain::{FixedVault, ResolvedVault, WrapArtifact};
use crate::seal::SealKey;
use crate::tracing_guard::{CaptureLayer, CapturedEvents, force_wallet_callsites_enabled};

// ───────────────────────────── declared names ─────────────────────────────

use crate::keychain::BoundedVault;
use crate::keychain::bounded::KEYCHAIN_CALL_BOUND;
// The report types live at the crate root (`sever.rs`, re-exported).
use crate::{FilesOutcome, HolderSeen, SeverOutcome, SeverReport};

/// The decorator over `inner`, with injected bounds, checking `ns`'s tombstone.
fn bounded(
    inner: Arc<dyn KeychainPort>,
    call_bound: Duration,
    budget: Duration,
    ns: &KeychainNamespace,
) -> Arc<dyn KeychainPort> {
    Arc::new(BoundedVault::with_bounds(
        inner,
        call_bound,
        budget,
        Some(ns.clone()),
    ))
}

/// The decorator with no namespace to check (the `settle` probe).
fn unchecked(inner: Arc<dyn KeychainPort>) -> Arc<dyn KeychainPort> {
    Arc::new(BoundedVault::with_bounds(
        inner,
        Duration::from_secs(10),
        Duration::from_secs(10),
        None,
    ))
}

async fn sever_core(
    dir: &Path,
    resolver: Option<Arc<dyn VaultResolver>>,
    deadline: Duration,
) -> Result<SeverReport, WalletError> {
    Wallet::sever_custody_resolving(dir, resolver, deadline).await
}

/// What the report says, in one comparable value.
#[derive(Debug, PartialEq, Eq)]
enum Got {
    Severed(usize),
    Unproven(String),
    AlreadyGone,
    NotSevered(String),
}

fn got(r: &SeverReport) -> Got {
    match &r.severed {
        SeverOutcome::Severed { count } => Got::Severed(*count),
        SeverOutcome::SeveredUnproven { reason } => Got::Unproven(norm(reason)),
        SeverOutcome::AlreadyGone => Got::AlreadyGone,
        SeverOutcome::NotSevered { cause } => Got::NotSevered(norm(cause)),
    }
}

fn this_process(r: &SeverReport) -> bool {
    matches!(r.holder, HolderSeen::ThisProcess)
}

fn other_process(r: &SeverReport) -> bool {
    matches!(r.holder, HolderSeen::OtherProcess)
}

fn no_holder(r: &SeverReport) -> bool {
    matches!(r.holder, HolderSeen::None)
}

fn left_for_host(r: &SeverReport) -> bool {
    matches!(r.files, FilesOutcome::LeftForHost)
}

fn removed(r: &SeverReport) -> bool {
    matches!(r.files, FilesOutcome::Removed)
}

fn wipe_scope_until(end: Instant) -> impl Drop {
    crate::keychain::bounded::wipe_scope_until(end)
}

fn reset_paths() {
    crate::keychain::bounded::reset_paths();
}

/// One path-table entry, as the row reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Entry {
    live: usize,
    opening: usize,
    tombstoned: bool,
}

/// The path table's `cfg(test)` accessor, `path_entry(ns) -> Option<(live,
/// opening, tombstoned)>`, read as one [`Entry`].
fn entry(ns: &KeychainNamespace) -> Option<Entry> {
    crate::keychain::bounded::path_entry(ns).map(|(live, opening, tombstoned)| Entry {
        live,
        opening,
        tombstoned,
    })
}

/// This file's names for the built pause points (the hook table is keyed by
/// (seam, `db_dir`) and fires a hook ONCE; [`Armed`] re-arms it after each
/// firing while the value lives, and takes the directory at the call site).
#[derive(Clone, Copy, Debug)]
enum Seam {
    OpenAfterAcquire,
    OpenBeforeBirthGate,
    RebuildBeforeAssemble,
    BroadcastTop,
    BroadcastBeforeSend,
}

impl Seam {
    fn built(self) -> sever_seams::Seam {
        match self {
            Seam::OpenAfterAcquire => sever_seams::Seam::AfterAcquireOpening,
            Seam::OpenBeforeBirthGate => sever_seams::Seam::BeforeBirthGate,
            Seam::RebuildBeforeAssemble => sever_seams::Seam::AfterUnwrap,
            Seam::BroadcastTop => sever_seams::Seam::BroadcastBeforeFirstCheck,
            Seam::BroadcastBeforeSend => sever_seams::Seam::BroadcastBeforeSendCheck,
        }
    }
}

type RepeatingHook = Arc<Mutex<Box<dyn FnMut() + Send>>>;

fn arm_repeating(point: Seam, dir: PathBuf, f: RepeatingHook, live: Arc<AtomicBool>) {
    let at = dir.clone();
    sever_seams::arm(point.built(), &at, move || {
        if live.load(Ordering::SeqCst) {
            (f.lock().expect("hook"))();
            arm_repeating(point, dir, f, live);
        }
    });
}

/// A hook armed for the life of the value — disarmed however the row ends.
struct Armed(Arc<AtomicBool>);

impl Armed {
    fn new(point: Seam, dir: &Path, f: impl FnMut() + Send + 'static) -> Self {
        let live = Arc::new(AtomicBool::new(true));
        arm_repeating(
            point,
            dir.to_path_buf(),
            Arc::new(Mutex::new(Box::new(f))),
            Arc::clone(&live),
        );
        Self(live)
    }
}

impl Drop for Armed {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

// ───────────────────────────── the clock ─────────────────────────────

/// Anti-hang only: a build with no bound fails here instead of hanging.
const ANSWER_CEILING: Duration = Duration::from_secs(30);
/// The duress deadline the exit gate names.
const DEADLINE: Duration = Duration::from_secs(2);
/// A caller that stays for as long as the row needs it.
const LONG: Duration = Duration::from_secs(120);
/// A caller that leaves while its call is still in the key store.
const SHORT: Duration = Duration::from_millis(300);
/// How long an operation on an UNRELATED wallet may take while the key store
/// is wedged (§3.1 assertion 16 says < 100 ms; see the finding — a debug-build
/// open does not fit 100 ms, and the witness is that it finishes at all while
/// the wedge is held, well before the 2 s sever answers).
const UNRELATED_OP: Duration = Duration::from_secs(1);

const BLOB: &[u8] = b"s16 sealed blob";

// ───────────────────────────── serialisation ─────────────────────────────

static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|p| p.into_inner())
}

fn norm(v: &impl std::fmt::Debug) -> String {
    format!("{v:?}")
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_ascii_lowercase()
}

fn timeout_cause(e: &WalletError) -> Option<String> {
    match e {
        WalletError::KeychainTimeout { cause } => Some(norm(cause)),
        _ => None,
    }
}

fn is_wiped(e: &WalletError) -> bool {
    matches!(
        e,
        WalletError::InvalidState {
            phase: LifecyclePhase::Wiped
        }
    )
}

// ───────────────────────────── gates ─────────────────────────────

#[derive(Default)]
struct Gate {
    open: Mutex<bool>,
    cv: Condvar,
}

impl Gate {
    fn wait(&self) {
        let deadline = Instant::now() + ANSWER_CEILING * 4;
        let mut open = self.open.lock().unwrap_or_else(|p| p.into_inner());
        while !*open {
            let left = deadline
                .checked_duration_since(Instant::now())
                .unwrap_or_else(|| panic!("a gate was never opened"));
            open = self
                .cv
                .wait_timeout(open, left)
                .unwrap_or_else(|p| p.into_inner())
                .0;
        }
    }

    fn is_open(&self) -> bool {
        *self.open.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn release(&self) {
        *self.open.lock().unwrap_or_else(|p| p.into_inner()) = true;
        self.cv.notify_all();
    }
}

/// Wait, without blocking the runtime, until `gate` opens.
async fn reached(gate: &Gate, what: &str) {
    let started = Instant::now();
    while !gate.is_open() {
        assert!(started.elapsed() < ANSWER_CEILING, "{what}: never reached");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

// ───────────────────────────── the fake key store ─────────────────────────────

/// Every `KeychainPort` method, classified (assertion 15's exhaustive list).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Method {
    Probe,
    Tier,
    StoreWrapKey,
    LoadWrapKey,
    RotateWrapKey,
    FinishRotation,
    DeleteWrapKey,
    PurgeNamespace,
    StoreIndex,
    LoadIndex,
    DeleteIndex,
}

#[derive(Debug, PartialEq, Eq)]
enum Class {
    Create,
    Delete,
    Read,
}

impl Method {
    fn all() -> Vec<Method> {
        let next = |m: Method| match m {
            Method::Probe => Some(Method::Tier),
            Method::Tier => Some(Method::StoreWrapKey),
            Method::StoreWrapKey => Some(Method::LoadWrapKey),
            Method::LoadWrapKey => Some(Method::RotateWrapKey),
            Method::RotateWrapKey => Some(Method::FinishRotation),
            Method::FinishRotation => Some(Method::DeleteWrapKey),
            Method::DeleteWrapKey => Some(Method::PurgeNamespace),
            Method::PurgeNamespace => Some(Method::StoreIndex),
            Method::StoreIndex => Some(Method::LoadIndex),
            Method::LoadIndex => Some(Method::DeleteIndex),
            Method::DeleteIndex => None,
        };
        std::iter::successors(Some(Method::Probe), |m| next(*m)).collect()
    }
    /// A new trait method → a `Fake` impl → a `Method`: no compile till `all()` and this name it.
    fn class(self) -> Class {
        match self {
            Method::StoreWrapKey | Method::StoreIndex | Method::RotateWrapKey => Class::Create,
            Method::PurgeNamespace
            | Method::DeleteIndex
            | Method::DeleteWrapKey
            | Method::FinishRotation => Class::Delete,
            Method::Probe | Method::Tier | Method::LoadWrapKey | Method::LoadIndex => Class::Read,
        }
    }
}

/// The fake's policy and what it saw, shared by every vault a row builds.
#[derive(Default)]
struct Ctl {
    /// The one method that blocks on `gate` INSIDE the (bounded) call.
    wedge: Mutex<Option<Method>>,
    gate: Gate,
    /// A purge that answers `Ok(0)` and leaves the key (the locked-iOS filter).
    zero_purge: AtomicBool,
    /// Methods that answer `KeystoreUnavailable` (a locked keychain).
    locked: Mutex<Vec<Method>>,
    /// A purge under this namespace fails `KeystoreUnavailable`.
    failing_purge: Mutex<Option<String>>,
    /// Every call that reached the fake: (method, namespace).
    calls: Mutex<Vec<(Method, String)>>,
    /// Every purge count the fake answered.
    purge_counts: Mutex<Vec<usize>>,
}

impl Ctl {
    fn wedging(m: Method) -> Arc<Self> {
        let c = Self::default();
        *c.wedge.lock().expect("wedge") = Some(m);
        Arc::new(c)
    }

    fn calls(&self) -> Vec<(Method, String)> {
        self.calls.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    fn count(&self, m: Method) -> usize {
        self.calls().iter().filter(|(o, _)| *o == m).count()
    }

    fn wait_entered(&self, m: Method) {
        let started = Instant::now();
        while self.count(m) == 0 {
            assert!(
                started.elapsed() < ANSWER_CEILING,
                "`{m:?}` never reached the vault"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn lock_up(&self, m: Method) {
        self.locked.lock().expect("locked").push(m);
    }

    fn nonzero_purges(&self) -> Vec<usize> {
        self.purge_counts
            .lock()
            .expect("counts")
            .iter()
            .copied()
            .filter(|n| *n > 0)
            .collect()
    }
}

/// Opens the gate when the row ends, however it ends.
struct ReleaseOnDrop(Arc<Ctl>);

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.0.gate.release();
    }
}

/// A delegating vault whose policy is the row's [`Ctl`].
struct Fake {
    inner: Arc<dyn KeychainPort>,
    ns: String,
    ctl: Arc<Ctl>,
}

impl Fake {
    fn enter(&self, m: Method) -> Result<(), WalletError> {
        self.ctl
            .calls
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push((m, self.ns.clone()));
        let wedged = *self.ctl.wedge.lock().unwrap_or_else(|p| p.into_inner()) == Some(m);
        if wedged {
            self.ctl.gate.wait();
        }
        let locked = self
            .ctl
            .locked
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .contains(&m);
        if locked {
            return Err(WalletError::KeystoreUnavailable);
        }
        Ok(())
    }
}

impl KeychainPort for Fake {
    fn probe(&self) -> Result<(), WalletError> {
        self.enter(Method::Probe)?;
        self.inner.probe()
    }
    fn tier(&self) -> Result<VaultTier, WalletError> {
        self.enter(Method::Tier)?;
        self.inner.tier()
    }
    fn store_wrap_key(&self, key: SealKey, blob: &[u8]) -> Result<WrapArtifact, WalletError> {
        self.enter(Method::StoreWrapKey)?;
        self.inner.store_wrap_key(key, blob)
    }
    fn load_wrap_key(&self, a: &WrapArtifact, blob: &[u8]) -> Result<SealKey, WalletError> {
        self.enter(Method::LoadWrapKey)?;
        self.inner.load_wrap_key(a, blob)
    }
    fn rotate_wrap_key(&self, a: &WrapArtifact, blob: &[u8]) -> Result<WrapArtifact, WalletError> {
        self.enter(Method::RotateWrapKey)?;
        self.inner.rotate_wrap_key(a, blob)
    }
    fn finish_rotation(&self, old: &WrapArtifact, new: &WrapArtifact) -> Result<(), WalletError> {
        self.enter(Method::FinishRotation)?;
        self.inner.finish_rotation(old, new)
    }
    fn delete_wrap_key(&self, a: &WrapArtifact) -> Result<(), WalletError> {
        self.enter(Method::DeleteWrapKey)?;
        self.inner.delete_wrap_key(a)
    }
    fn purge_namespace(&self) -> Result<usize, WalletError> {
        self.enter(Method::PurgeNamespace)?;
        let failing = self.ctl.failing_purge.lock().expect("failing").clone();
        if failing.as_deref() == Some(self.ns.as_str()) {
            return Err(WalletError::KeystoreUnavailable);
        }
        let n = if self.ctl.zero_purge.load(Ordering::SeqCst) {
            0
        } else {
            self.inner.purge_namespace()?
        };
        self.ctl.purge_counts.lock().expect("counts").push(n);
        Ok(n)
    }
    fn store_index(&self, entry: &CustodyIndexEntry) -> Result<(), WalletError> {
        self.enter(Method::StoreIndex)?;
        self.inner.store_index(entry)
    }
    fn load_index(&self) -> Result<Option<CustodyIndexEntry>, WalletError> {
        self.enter(Method::LoadIndex)?;
        self.inner.load_index()
    }
    fn delete_index(&self) -> Result<(), WalletError> {
        self.enter(Method::DeleteIndex)?;
        self.inner.delete_index()
    }
}

/// What a [`Pause`] waits for: these methods, under this namespace (any when
/// `None`), the `nth` (1-based) matching call.
type PauseAt = (Vec<Method>, Option<String>, usize);

/// A pause on the CALLER's thread, right after one armed call returns — the
/// window between two custody calls, outside the worker.
#[derive(Default)]
struct Pause {
    /// Pause after the `nth` (1-based) call of one of these methods under
    /// this namespace (any namespace when `None`). One-shot.
    armed: Mutex<Option<PauseAt>>,
    reached: Gate,
    release: Gate,
}

impl Pause {
    fn after(methods: &[Method], ns: Option<&KeychainNamespace>) -> Arc<Self> {
        Self::after_nth(methods, ns, 1)
    }

    fn after_nth(methods: &[Method], ns: Option<&KeychainNamespace>, nth: usize) -> Arc<Self> {
        let p = Self::default();
        *p.armed.lock().expect("armed") =
            Some((methods.to_vec(), ns.map(|n| n.as_str().to_owned()), nth));
        Arc::new(p)
    }

    fn hit(&self, m: Method, ns: &str) -> bool {
        let mut armed = self.armed.lock().unwrap_or_else(|p| p.into_inner());
        let Some((ms, want, nth)) = armed.as_mut() else {
            return false;
        };
        if !ms.contains(&m) || want.as_deref().is_some_and(|w| w != ns) {
            return false;
        }
        *nth -= 1;
        if *nth > 0 {
            return false;
        }
        *armed = None;
        true
    }
}

/// Holds the pause released when the row ends, however it ends.
struct UnpauseOnDrop(Arc<Pause>);

impl Drop for UnpauseOnDrop {
    fn drop(&mut self) {
        self.0.release.release();
    }
}

struct Pausing {
    inner: Arc<dyn KeychainPort>,
    ns: String,
    pause: Arc<Pause>,
}

impl Pausing {
    fn after<T>(&self, m: Method, out: T) -> T {
        if self.pause.hit(m, &self.ns) {
            self.pause.reached.release();
            self.pause.release.wait();
        }
        out
    }
}

impl KeychainPort for Pausing {
    fn probe(&self) -> Result<(), WalletError> {
        self.after(Method::Probe, self.inner.probe())
    }
    fn tier(&self) -> Result<VaultTier, WalletError> {
        self.after(Method::Tier, self.inner.tier())
    }
    fn store_wrap_key(&self, key: SealKey, blob: &[u8]) -> Result<WrapArtifact, WalletError> {
        self.after(Method::StoreWrapKey, self.inner.store_wrap_key(key, blob))
    }
    fn load_wrap_key(&self, a: &WrapArtifact, blob: &[u8]) -> Result<SealKey, WalletError> {
        self.after(Method::LoadWrapKey, self.inner.load_wrap_key(a, blob))
    }
    fn rotate_wrap_key(&self, a: &WrapArtifact, blob: &[u8]) -> Result<WrapArtifact, WalletError> {
        self.after(Method::RotateWrapKey, self.inner.rotate_wrap_key(a, blob))
    }
    fn finish_rotation(&self, old: &WrapArtifact, new: &WrapArtifact) -> Result<(), WalletError> {
        self.after(Method::FinishRotation, self.inner.finish_rotation(old, new))
    }
    fn delete_wrap_key(&self, a: &WrapArtifact) -> Result<(), WalletError> {
        self.after(Method::DeleteWrapKey, self.inner.delete_wrap_key(a))
    }
    fn purge_namespace(&self) -> Result<usize, WalletError> {
        self.after(Method::PurgeNamespace, self.inner.purge_namespace())
    }
    fn store_index(&self, entry: &CustodyIndexEntry) -> Result<(), WalletError> {
        self.after(Method::StoreIndex, self.inner.store_index(entry))
    }
    fn load_index(&self) -> Result<Option<CustodyIndexEntry>, WalletError> {
        self.after(Method::LoadIndex, self.inner.load_index())
    }
    fn delete_index(&self) -> Result<(), WalletError> {
        self.after(Method::DeleteIndex, self.inner.delete_index())
    }
}

/// Per namespace: the shared keychain's vault, inside the fake, optionally
/// inside the decorator (checking THAT namespace's tombstone), optionally
/// inside a caller-thread pause.
#[derive(Clone)]
struct Stack {
    keychain: SharedKeychain,
    ctl: Arc<Ctl>,
    bounds: Option<(Duration, Duration)>,
    pause: Option<Arc<Pause>>,
}

impl VaultResolver for Stack {
    fn vault_for<'a>(
        &'a self,
        namespace: &KeychainNamespace,
    ) -> Result<ResolvedVault<'a>, WalletError> {
        let fake: Arc<dyn KeychainPort> = Arc::new(Fake {
            inner: shared_mk(&self.keychain, namespace.as_str()),
            ns: namespace.as_str().to_owned(),
            ctl: Arc::clone(&self.ctl),
        });
        let v = match self.bounds {
            Some((call, budget)) => bounded(fake, call, budget, namespace),
            None => fake,
        };
        let v = match &self.pause {
            Some(p) => Arc::new(Pausing {
                inner: v,
                ns: namespace.as_str().to_owned(),
                pause: Arc::clone(p),
            }) as Arc<dyn KeychainPort>,
            None => v,
        };
        Ok(ResolvedVault::Owned(v))
    }
}

/// The per-namespace resolver over the shared keychain, observed by `ctl`.
fn observed(keychain: &SharedKeychain, ctl: &Arc<Ctl>) -> Arc<dyn VaultResolver> {
    Arc::new(Stack {
        keychain: keychain.clone(),
        ctl: Arc::clone(ctl),
        bounds: None,
        pause: None,
    })
}

/// The plain per-namespace resolver over the shared keychain.
fn plain(keychain: &SharedKeychain) -> Arc<dyn VaultResolver> {
    observed(keychain, &Arc::new(Ctl::default()))
}

/// The production stack: every namespace behind the decorator.
fn production(
    keychain: &SharedKeychain,
    ctl: &Arc<Ctl>,
    call: Duration,
    budget: Duration,
) -> Stack {
    Stack {
        keychain: keychain.clone(),
        ctl: Arc::clone(ctl),
        bounds: Some((call, budget)),
        pause: None,
    }
}

// ───────────────────────────── row helpers ─────────────────────────────

/// A wallet at a directory of its own, over its own shared keychain.
struct Fixture {
    _parent: tempfile::TempDir,
    dir: PathBuf,
    keychain: SharedKeychain,
    path_ns: KeychainNamespace,
    id_ns: KeychainNamespace,
}

impl Fixture {
    fn config(&self) -> WalletConfig {
        cfg(&self.dir, Network::Test, SeedPersistence::SealedKeychain)
    }

    async fn open(&self) -> Wallet {
        Wallet::open_resolving(self.config(), plain(&self.keychain), None, None)
            .await
            .expect("open")
    }

    fn key_live(&self, ns: &KeychainNamespace) -> bool {
        self.keychain.lock().expect("kc").contains_key(ns.as_str())
    }

    fn no_key(&self) -> bool {
        self.keychain.lock().expect("kc").is_empty()
    }

    fn index(&self) -> Option<CustodyIndexEntry> {
        shared_mk(&self.keychain, self.path_ns.as_str())
            .load_index()
            .expect("read the index directly")
    }

    fn breadcrumb(&self) -> bool {
        self.dir
            .join(crate::constants::WIPE_COMMITTED_FILE_NAME)
            .exists()
    }

    fn files(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.dir)
            .map(|rd| {
                rd.filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }
}

/// A post-S2 wallet, created and closed.
async fn created(label: &str) -> Fixture {
    let keychain = SharedKeychainVault::shared();
    let parent = tempfile::tempdir().expect("parent dir");
    let dir = parent.path().join(label).join("wallet");
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
    let id = header_id(&dir).expect("a post-S2 create frames its identifier");
    let fixture = Fixture {
        _parent: parent,
        path_ns: keychain_namespace_for(&dir),
        id_ns: crate::custody::namespace_for(&id),
        dir,
        keychain,
    };
    assert!(fixture.key_live(&fixture.id_ns), "fixture: the key is live");
    fixture
}

/// A wallet created before S2: its key under the PATH namespace, no
/// identifier in its header (the s9 rows' own pre-stage edit), and NO index
/// item — S2 introduced the index, so a pre-S2 wallet never wrote one. Today's
/// create writes `done(id)` under the path namespace; left in place it would
/// point resolution at an id namespace that holds nothing, which is not a
/// pre-stage wallet (the adjudicator's probe: with the index the sever severs
/// 0, without it 1).
async fn pre_stage(label: &str) -> Fixture {
    let keychain = SharedKeychainVault::shared();
    let parent = tempfile::tempdir().expect("parent dir");
    let dir = parent.path().join(label).join("wallet");
    std::fs::create_dir_all(&dir).expect("wallet dir");
    let path_ns = keychain_namespace_for(&dir);
    Wallet::create_with_vault(
        cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
        raw_seed(),
        shared_mk(&keychain, path_ns.as_str()),
    )
    .await
    .expect("create the to-be-pre-stage wallet")
    .close()
    .await
    .expect("close the create");
    let stripped = WrapArtifact::from_freshly_wrapped(artifact(&dir).as_bytes().to_vec());
    std::fs::write(
        dir.join(crate::constants::WRAP_ARTIFACT_FILE_NAME),
        stripped.file_bytes(),
    )
    .expect("write the stripped artifact");
    shared_mk(&keychain, path_ns.as_str())
        .delete_index()
        .expect("delete the index a pre-S2 wallet never had");
    assert!(header_id(&dir).is_none(), "the pre-stage shape: no locator");
    assert!(
        shared_mk(&keychain, path_ns.as_str())
            .load_index()
            .expect("read the index")
            .is_none(),
        "the pre-stage shape: no index"
    );
    assert!(
        keychain.lock().expect("kc").contains_key(path_ns.as_str()),
        "the pre-stage shape: the key under the path namespace"
    );
    Fixture {
        _parent: parent,
        dir,
        keychain,
        id_ns: path_ns.clone(),
        path_ns,
    }
}

fn artifact(dir: &Path) -> WrapArtifact {
    WrapArtifact::from_bytes(
        std::fs::read(dir.join(crate::constants::WRAP_ARTIFACT_FILE_NAME)).expect("header"),
    )
    .expect("header parses")
}

fn header_id(dir: &Path) -> Option<CustodyId> {
    artifact(dir).custody_id()
}

async fn sever(
    dir: &Path,
    resolver: Option<Arc<dyn VaultResolver>>,
    deadline: Duration,
) -> SeverReport {
    tokio::time::timeout(ANSWER_CEILING, sever_core(dir, resolver, deadline))
        .await
        .unwrap_or_else(|_| panic!("the sever did not answer within {ANSWER_CEILING:?}"))
        .expect("a custody outcome is a report, never an error")
}

/// The sever, run to completion on its own thread and runtime — what a hook
/// armed inside the wallet calls, so the wallet's own runtime is not needed.
fn sever_now(
    dir: PathBuf,
    resolver: Option<Arc<dyn VaultResolver>>,
    deadline: Duration,
) -> SeverReport {
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(sever(&dir, resolver, deadline))
    })
    .join()
    .expect("the sever's thread")
}

/// A sever whose report the row reads later, run from inside a hook.
type Slot = Arc<Mutex<Option<SeverReport>>>;

/// A hook that severs `dir` the `on_call`-th time its point is passed.
fn severing_hook(
    dir: &Path,
    resolver: Arc<dyn VaultResolver>,
    slot: &Slot,
    on_call: usize,
) -> impl FnMut() + Send + 'static {
    let dir = dir.to_path_buf();
    let slot = Arc::clone(slot);
    let mut calls = 0_usize;
    move || {
        calls += 1;
        if calls == on_call {
            let r = sever_now(dir.clone(), Some(Arc::clone(&resolver)), DEADLINE);
            *slot.lock().expect("slot") = Some(r);
        }
    }
}

/// Wrap `hook`: once it has fired (the sever ran), remember how many fields the
/// sink held, so the row can assert that nothing was logged after.
fn marking(
    sink: &CapturedEvents,
    mark: &Arc<AtomicUsize>,
    mut hook: impl FnMut() + Send + 'static,
) -> impl FnMut() + Send + 'static {
    let sink = sink.clone();
    let mark = Arc::clone(mark);
    move || {
        hook();
        if mark.load(Ordering::SeqCst) == usize::MAX {
            mark.store(sink.fields().len(), Ordering::SeqCst);
        }
    }
}

/// Nothing reached the sink after `mark` was taken.
fn assert_silent_after(sink: &CapturedEvents, mark: &AtomicUsize, row: &str) {
    let mark = mark.load(Ordering::SeqCst);
    assert_ne!(mark, usize::MAX, "{row}: the sever hook never fired");
    let fields = sink.fields();
    assert!(
        fields.len() <= mark,
        "{row}: logged after the sever began: {:?}",
        &fields[mark..]
    );
}

fn taken(slot: &Slot) -> SeverReport {
    slot.lock()
        .expect("slot")
        .take()
        .expect("the hook fired and the sever ran")
}

/// Wait until the process worker takes new calls again.
fn settle() {
    let v = unchecked(Arc::new(TestVault::new(VaultTier::Tee)));
    let started = Instant::now();
    loop {
        match v.probe() {
            Ok(()) => return,
            Err(e)
                if timeout_cause(&e).as_deref() == Some("busy")
                    && started.elapsed() < ANSWER_CEILING =>
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => panic!("the worker never came back: {e:?}"),
        }
    }
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

// ═════════════════════════════ §3.1 `sever` ═════════════════════════════

/// Assertion 1 + the exit gate. With a second holder alive in this process —
/// an open handle AND a straggler clone of its `Inner` — the sever answers
/// inside its 2 s deadline, far under `QUIESCE_MAX`: `severed` (proven, a
/// non-zero count), `holder: thisProcess`, `files: leftForHost`. The wrap key
/// under the id namespace is gone, the index item is gone, no file in the
/// directory was removed, and the breadcrumb was written.
#[tokio::test]
async fn sever_with_a_live_holder_severs_custody_and_deletes_no_file() {
    let _serial = serial();
    let f = created("a1").await;
    let w = f.open().await;
    let straggler = Arc::clone(&w.inner);
    let before = f.files();

    let started = Instant::now();
    let r = sever(&f.dir, Some(plain(&f.keychain)), DEADLINE).await;
    let took = started.elapsed();
    println!("the sever beside a live holder answered in {took:?}");

    assert!(took < DEADLINE, "answered within its deadline: {took:?}");
    assert!(
        matches!(got(&r), Got::Severed(n) if n > 0),
        "proven: {:?}",
        got(&r)
    );
    assert!(this_process(&r), "the holder is this process");
    assert!(left_for_host(&r), "no file is deleted under a held lock");
    assert!(!f.key_live(&f.id_ns), "the wrap key is gone");
    assert!(f.index().is_none(), "the index item is gone");
    let after = f.files();
    for name in &before {
        assert!(
            after.contains(name),
            "`{name}` was removed under the live holder: before {before:?}, after {after:?}"
        );
    }
    assert!(f.breadcrumb(), "a proven sever writes its breadcrumb");

    drop(straggler);
    drop(w);
}

/// Assertion 2. With the lock free the sever IS the plain wipe: `files:
/// removed`, `holder: none`, the directory and the index gone, and a later
/// open finds no wallet — the same assertions as
/// `wipe_severs_keychain_custody_then_open_is_not_found`.
#[tokio::test]
async fn sever_with_a_free_lock_is_the_plain_wipe_and_says_files_removed() {
    let _serial = serial();
    let f = created("a2").await;

    let r = sever(&f.dir, Some(plain(&f.keychain)), DEADLINE).await;

    assert!(matches!(got(&r), Got::Severed(n) if n > 0), "{:?}", got(&r));
    assert!(no_holder(&r), "the lock was free");
    assert!(removed(&r), "the files are the sever's to remove");
    assert!(f.no_key(), "custody severed");
    assert!(!f.dir.exists(), "the directory is gone");
    assert!(f.index().is_none(), "the index item is gone");
    match Wallet::open_resolving(f.config(), plain(&f.keychain), None, None).await {
        Err(WalletError::NotFound) => {}
        Err(other) => panic!("open after the sever must be NotFound, got {other:?}"),
        Ok(_) => panic!("open after the sever must not succeed"),
    }
}

/// Assertion 3. After a sever beside a live holder, the holder goes, then a
/// FRESH process (no late-sever evidence, no path table): a plain `wipe`
/// converges with no `wipe_force`, and a second `wipe` is the no-op `Ok`.
/// On the way, §3.3 obligation 4's two answers for an open at the severed,
/// unpurged path: `wiped` in this process, the custody error after a restart.
#[tokio::test]
async fn a_plain_wipe_after_a_sever_converges_without_wipe_force() {
    let _serial = serial();
    let f = created("a3").await;
    let w = f.open().await;
    let straggler = Arc::clone(&w.inner);
    let r = sever(&f.dir, Some(plain(&f.keychain)), DEADLINE).await;
    assert!(matches!(got(&r), Got::Severed(_)), "{:?}", got(&r));
    drop(straggler);
    w.close().await.expect("a poisoned handle still closes Ok");
    // §3.3 obligation 4: in this process an open at the severed, unpurged
    // path answers `wiped` (the tombstone, read before the store is touched).
    match Wallet::open_resolving(f.config(), plain(&f.keychain), None, None).await {
        Err(e) => assert!(is_wiped(&e), "in-process open at a severed path: {e:?}"),
        Ok(_) => panic!("an open at a severed path must not succeed"),
    }

    crate::keychain::bounded::reset_severed_late();
    reset_paths();

    // After a restart the tombstone is gone: the open fails on the custody
    // the sever removed — the store's own error, not `wiped`.
    match Wallet::open_resolving(f.config(), plain(&f.keychain), None, None).await {
        Err(e) => {
            println!("after a restart, an open at the severed path answers {e:?}");
            assert!(!is_wiped(&e), "no tombstone after a restart: {e:?}");
        }
        Ok(_) => panic!("an open at a severed path must not succeed"),
    }
    assert!(f.breadcrumb(), "the refused open left the breadcrumb");

    Wallet::wipe_resolving(&f.dir, Some(plain(&f.keychain)), false)
        .await
        .expect("the breadcrumb lets the plain wipe converge, without wipe_force");
    assert!(!f.dir.exists(), "the directory is gone");
    Wallet::wipe_resolving(&f.dir, Some(plain(&f.keychain)), false)
        .await
        .expect("a second wipe is the no-op success");
}

/// Assertion 4. No keychain item survives under ANY namespace the wallet
/// used: a post-S2 wallet (the id namespace + the index), a wallet
/// mid-migration (a `pending` index, a key under the fresh id namespace AND
/// the legacy path key still live), and a pre-stage wallet (everything under
/// the path namespace). Each is severed while another holder has the lock.
#[tokio::test]
async fn sever_leaves_no_keychain_item_under_any_namespace_the_wallet_used() {
    let _serial = serial();

    // post-S2
    let f = created("a4-post").await;
    let lock = WalletLock::acquire(&f.dir).expect("another holder");
    let r = sever(&f.dir, Some(plain(&f.keychain)), DEADLINE).await;
    assert!(matches!(got(&r), Got::Severed(_)), "post-S2: {:?}", got(&r));
    assert!(f.no_key(), "post-S2: no key");
    assert!(f.index().is_none(), "post-S2: no index");
    drop(lock);

    // mid-migration: the index says `pending`, the id key was re-custodied,
    // the commit (the header's locator) never landed
    let f = pre_stage("a4-mid").await;
    let id = CustodyId::generate();
    let id_ns = crate::custody::namespace_for(&id);
    shared_mk(&f.keychain, f.path_ns.as_str())
        .store_index(&CustodyIndexEntry::pending(&id, &f.path_ns))
        .expect("the pending index");
    shared_mk(&f.keychain, id_ns.as_str())
        .store_wrap_key(SealKey::generate(), BLOB)
        .expect("the re-custodied key");
    assert!(f.key_live(&f.path_ns) && f.key_live(&id_ns), "fixture");
    let lock = WalletLock::acquire(&f.dir).expect("another holder");
    let r = sever(&f.dir, Some(plain(&f.keychain)), DEADLINE).await;
    assert!(
        matches!(got(&r), Got::Severed(n) if n >= 2),
        "mid-migration: both keys: {:?}",
        got(&r)
    );
    assert!(!f.key_live(&id_ns), "mid-migration: the id key is gone");
    assert!(
        !f.key_live(&f.path_ns),
        "mid-migration: the legacy key is gone"
    );
    assert!(f.index().is_none(), "mid-migration: the index is gone");
    drop(lock);

    // pre-stage
    let f = pre_stage("a4-pre").await;
    let lock = WalletLock::acquire(&f.dir).expect("another holder");
    let r = sever(&f.dir, Some(plain(&f.keychain)), DEADLINE).await;
    assert!(
        matches!(got(&r), Got::Severed(_)),
        "pre-stage: {:?}",
        got(&r)
    );
    assert!(f.no_key(), "pre-stage: no key");
    assert!(f.index().is_none(), "pre-stage: no index");
    drop(lock);
}

/// Assertion 5. Never "severed" on a readable zero. A non-Empty store whose
/// artifact is present and whose purge severs 0 — (a) the B1 mismatch shape
/// (the key is not where the header says; lock free), (b) a vault answering
/// `Ok(0)` with the key still there (lock held) — reports
/// `notSevered(nothingSevered)`, writes no breadcrumb, deletes no index and no
/// file.
#[tokio::test]
async fn sever_never_reports_severed_on_a_zero_count() {
    let _serial = serial();

    // (a) the mismatch: the item is gone from under the header's namespace
    let f = created("a5-mismatch").await;
    f.keychain.lock().expect("kc").remove(f.id_ns.as_str());
    let before = f.files();
    let r = sever(&f.dir, Some(plain(&f.keychain)), DEADLINE).await;
    assert_eq!(got(&r), Got::NotSevered("nothingsevered".into()), "(a)");
    assert!(!f.breadcrumb(), "(a) no breadcrumb");
    assert!(f.index().is_some(), "(a) the index survives");
    assert_eq!(f.files(), before, "(a) no file deleted");

    // (b) the vault answers Ok(0), the key stays
    let f = created("a5-zero").await;
    let w = f.open().await;
    let ctl = Arc::new(Ctl::default());
    ctl.zero_purge.store(true, Ordering::SeqCst);
    let before = f.files();
    let r = sever(&f.dir, Some(observed(&f.keychain, &ctl)), DEADLINE).await;
    assert_eq!(got(&r), Got::NotSevered("nothingsevered".into()), "(b)");
    assert!(!f.breadcrumb(), "(b) no breadcrumb");
    assert!(f.index().is_some(), "(b) the index survives");
    assert_eq!(f.files(), before, "(b) no file deleted");
    assert!(ctl.count(Method::PurgeNamespace) >= 1, "(b) the purge ran");
    drop(w);
}

/// Assertion 6. A pre-stage wallet's migration, paused between its
/// `load_wrap_key` (legacy) and its `store_wrap_key` (the fresh id), with the
/// sever run in that window. When the migration resumes, its
/// `store_wrap_key` job refuses `Wiped` inside the worker, the open fails,
/// and no item exists under the fresh id namespace.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_custody_write_racing_the_sever_cannot_resurrect_a_key() {
    let _serial = serial();
    settle();
    let f = pre_stage("a6").await;
    let ctl = Arc::new(Ctl::default());
    // The legacy key is loaded twice under the path namespace: once to prove
    // the pre-stage custody is alive (`load_custody_resolving`), then by the
    // migration itself, right before its `store_wrap_key` — the second.
    let pause = Pause::after_nth(&[Method::LoadWrapKey], Some(&f.path_ns), 2);
    let _unpause = UnpauseOnDrop(Arc::clone(&pause));
    let mut stack = production(&f.keychain, &ctl, LONG, LONG);
    stack.pause = Some(Arc::clone(&pause));

    let config = f.config();
    let opener =
        tokio::spawn(
            async move { Wallet::open_resolving(config, Arc::new(stack), None, None).await },
        );
    reached(&pause.reached, "the migration's legacy load").await;
    let pending = f
        .index()
        .expect("the migration wrote its pending index first");
    assert!(pending.is_pending(), "fixture: mid-migration");
    let id_ns = crate::custody::namespace_for(&pending.id);

    let r = sever(
        &f.dir,
        Some(Arc::new(production(&f.keychain, &ctl, LONG, LONG))),
        DEADLINE,
    )
    .await;
    assert!(this_process(&r), "the opener holds the lock");
    pause.release.release();

    let opened = tokio::time::timeout(ANSWER_CEILING, opener)
        .await
        .expect("the open answers")
        .expect("the open's task");
    match opened {
        Err(e) => assert!(is_wiped(&e), "the resumed write refuses Wiped: {e:?}"),
        Ok(_) => panic!("a migration racing a sever must not open"),
    }
    assert!(
        !f.key_live(&id_ns),
        "no key was resurrected under the fresh id namespace"
    );
    assert!(!f.key_live(&f.path_ns), "the legacy key was purged");
    settle();
}

/// Assertion 7 (the review's BLOCKER). A create paused between its two
/// custody writes — whichever comes first — with the sever run in that
/// window. The index (written first) names the id, so the key write refuses
/// `Wiped` and NO key exists under any namespace.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_create_racing_the_sever_leaves_no_key_because_its_index_comes_first() {
    let _serial = serial();
    settle();
    let keychain = SharedKeychainVault::shared();
    let parent = tempfile::tempdir().expect("parent");
    let dir = parent.path().join("a7").join("wallet");
    let path_ns = keychain_namespace_for(&dir);
    let ctl = Arc::new(Ctl::default());
    let pause = Pause::after(&[Method::StoreIndex, Method::StoreWrapKey], None);
    let _unpause = UnpauseOnDrop(Arc::clone(&pause));
    let mut stack = production(&keychain, &ctl, LONG, LONG);
    stack.pause = Some(Arc::clone(&pause));

    let config = cfg(&dir, Network::Test, SeedPersistence::SealedKeychain);
    let creator =
        tokio::spawn(
            async move { Wallet::create_resolving(config, raw_seed(), Arc::new(stack)).await },
        );
    reached(&pause.reached, "the create's first custody write").await;

    let r = sever(
        &dir,
        Some(Arc::new(production(&keychain, &ctl, LONG, LONG))),
        DEADLINE,
    )
    .await;
    assert!(this_process(&r), "the creator holds the lock");
    pause.release.release();

    let made = tokio::time::timeout(ANSWER_CEILING, creator)
        .await
        .expect("the create answers")
        .expect("the create's task");
    match made {
        Err(e) => assert!(is_wiped(&e), "the key write refuses Wiped: {e:?}"),
        Ok(_) => panic!("a create racing a sever must not produce a wallet"),
    }
    let left: Vec<String> = keychain.lock().expect("kc").keys().cloned().collect();
    assert!(left.is_empty(), "no key under any namespace: {left:?}");
    assert!(
        shared_mk(&keychain, path_ns.as_str())
            .load_index()
            .expect("index")
            .is_none(),
        "no index either"
    );
    settle();
}

/// Assertion 8. A custody write already RUNNING in the key store (it passed
/// its tombstone check, then wedged inside the vault) when the sever begins:
/// the sever's jobs queue behind it and the 200 ms sever answers
/// `notSevered(timeout)` — never `severed` — well under `KEYCHAIN_CALL_BOUND`.
/// The wedge is released and THAT write lands; a second sever purges it and
/// reports `severed`. A write submitted while the second sever's purge is
/// running queues BEHIND the purge and still refuses `Wiped` (the resolved id
/// was tombstoned before the purge was submitted); a third write after it
/// all refuses `Wiped`, and no item survives.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_sever_behind_an_in_flight_write_reports_not_severed_and_a_second_sever_finishes() {
    let _serial = serial();
    settle();
    let f = created("a8").await;
    let _held = WalletLock::acquire(&f.dir).expect("another holder");

    // the in-flight write, wedged inside the vault
    let ctl = Ctl::wedging(Method::StoreWrapKey);
    let _release = ReleaseOnDrop(Arc::clone(&ctl));
    let writer = {
        let v = bounded(
            Arc::new(Fake {
                inner: shared_mk(&f.keychain, f.id_ns.as_str()),
                ns: f.id_ns.as_str().to_owned(),
                ctl: Arc::clone(&ctl),
            }),
            LONG,
            LONG,
            &f.id_ns,
        );
        std::thread::spawn(move || v.store_wrap_key(SealKey::generate(), BLOB).map(|_| ()))
    };
    ctl.wait_entered(Method::StoreWrapKey);

    let quiet = Arc::new(Ctl::default());
    let started = Instant::now();
    let r = sever(
        &f.dir,
        Some(Arc::new(production(&f.keychain, &quiet, LONG, LONG))),
        Duration::from_millis(200),
    )
    .await;
    let took = started.elapsed();
    assert_eq!(
        got(&r),
        Got::NotSevered("timeout".into()),
        "queued behind a running write: never severed"
    );
    assert!(
        took < KEYCHAIN_CALL_BOUND,
        "answered by its deadline: {took:?}"
    );

    ctl.gate.release();
    writer
        .join()
        .expect("the writer's thread")
        .expect("the write that was already running lands");
    settle();
    assert!(f.key_live(&f.id_ns), "the in-flight write landed");

    // the second sever, its purge wedged, a write queued behind the purge
    let purge_wedge = Ctl::wedging(Method::PurgeNamespace);
    let _release2 = ReleaseOnDrop(Arc::clone(&purge_wedge));
    let second = {
        let dir = f.dir.clone();
        let resolver: Arc<dyn VaultResolver> =
            Arc::new(production(&f.keychain, &purge_wedge, LONG, LONG));
        tokio::spawn(async move { sever(&dir, Some(resolver), Duration::from_secs(10)).await })
    };
    purge_wedge.wait_entered(Method::PurgeNamespace);
    let queued = {
        let v = bounded(
            shared_mk(&f.keychain, f.id_ns.as_str()),
            LONG,
            LONG,
            &f.id_ns,
        );
        std::thread::spawn(move || v.store_wrap_key(SealKey::generate(), BLOB).map(|_| ()))
    };
    let started = Instant::now();
    while BoundedVault::payloads_held() == 0 {
        assert!(started.elapsed() < ANSWER_CEILING, "the write never queued");
        std::thread::sleep(Duration::from_millis(2));
    }
    purge_wedge.gate.release();
    let r = second.await.expect("the second sever's task");
    assert!(
        matches!(got(&r), Got::Severed(n) if n > 0),
        "the second sever finishes: {:?}",
        got(&r)
    );
    match queued.join().expect("the queued writer's thread") {
        Err(e) => assert!(
            is_wiped(&e),
            "a write behind the purge refuses Wiped: {e:?}"
        ),
        Ok(()) => panic!("a write queued behind the purge landed after it"),
    }
    assert!(!f.key_live(&f.id_ns), "no item survives the second sever");

    // a third write, long after
    let late = bounded(
        shared_mk(&f.keychain, f.id_ns.as_str()),
        LONG,
        LONG,
        &f.id_ns,
    );
    match late.store_wrap_key(SealKey::generate(), BLOB) {
        Err(e) => assert!(is_wiped(&e), "a write after the tombstone: {e:?}"),
        Ok(_) => panic!("a write after the tombstone landed"),
    }
    assert!(!f.key_live(&f.id_ns), "still no item");
    settle();
}

/// Assertion 9. An open that had already unsealed before the sever reaches
/// its breadcrumb sweep after it: the breadcrumb the proven sever wrote is
/// still there when the open is done (the post-sweep rewrite), and the open
/// itself is refused `Wiped` at the birth gate.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_sever_breadcrumb_survives_a_racing_open() {
    let _serial = serial();
    let f = created("a9").await;
    // The open's LAST key-store call before its sweep is the index read of
    // `settle_index_after_open` — after the unseal.
    let pause = Pause::after(&[Method::LoadIndex], Some(&f.path_ns));
    let _unpause = UnpauseOnDrop(Arc::clone(&pause));
    let stack = Stack {
        keychain: f.keychain.clone(),
        ctl: Arc::new(Ctl::default()),
        bounds: None,
        pause: Some(Arc::clone(&pause)),
    };
    let config = f.config();
    let opener =
        tokio::spawn(
            async move { Wallet::open_resolving(config, Arc::new(stack), None, None).await },
        );
    reached(&pause.reached, "the open, past its unseal").await;

    let r = sever(&f.dir, Some(plain(&f.keychain)), DEADLINE).await;
    assert!(matches!(got(&r), Got::Severed(_)), "{:?}", got(&r));
    assert!(f.breadcrumb(), "the proven sever wrote its breadcrumb");
    pause.release.release();

    let opened = tokio::time::timeout(ANSWER_CEILING, opener)
        .await
        .expect("the open answers")
        .expect("the open's task");
    match opened {
        Err(e) => assert!(is_wiped(&e), "the open is refused at the birth gate: {e:?}"),
        Ok(_) => panic!("an open racing a sever must not come back working"),
    }
    assert!(
        f.breadcrumb(),
        "the open's sweep must not leave a proven sever without its breadcrumb"
    );
}

/// Assertion 10 + the fixtures priced against `KEYCHAIN_WIPE_BUDGET`, through
/// seam (ii) with a resolver that hands out the decorator. (i) Budget 300 ms,
/// deadline 2 s, a wedged purge: `notSevered(timeout)` in ≥ 300 ms and < 2 s
/// — the budget wins. (ii) Budget 2 s, deadline 150 ms: the answer arrives
/// long before the 2 s budget — the deadline wins — on both branches.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sever_answers_by_the_earlier_of_its_deadline_and_the_wipe_budget() {
    let _serial = serial();
    settle();

    // (i) the budget wins
    let f = created("a10-i").await;
    let ctl = Ctl::wedging(Method::PurgeNamespace);
    let _release = ReleaseOnDrop(Arc::clone(&ctl));
    let started = Instant::now();
    let r = sever(
        &f.dir,
        Some(Arc::new(production(
            &f.keychain,
            &ctl,
            LONG,
            Duration::from_millis(300),
        ))),
        Duration::from_secs(2),
    )
    .await;
    let took = started.elapsed();
    println!("(i) budget 300 ms, deadline 2 s: answered in {took:?}");
    assert_eq!(got(&r), Got::NotSevered("timeout".into()), "(i)");
    assert!(
        took >= Duration::from_millis(300),
        "(i) not before the budget: {took:?}"
    );
    assert!(
        took < Duration::from_secs(2),
        "(i) the budget wins: {took:?}"
    );
    ctl.gate.release();
    settle();

    // (ii) the deadline wins — lock free, then lock held
    for held in [false, true] {
        let f = created(if held { "a10-ii-held" } else { "a10-ii-free" }).await;
        let _lock = held.then(|| WalletLock::acquire(&f.dir).expect("another holder"));
        let ctl = Ctl::wedging(Method::PurgeNamespace);
        let _release = ReleaseOnDrop(Arc::clone(&ctl));
        let started = Instant::now();
        let r = sever(
            &f.dir,
            Some(Arc::new(production(
                &f.keychain,
                &ctl,
                LONG,
                Duration::from_secs(2),
            ))),
            Duration::from_millis(150),
        )
        .await;
        let took = started.elapsed();
        println!("(ii) held={held} budget 2 s, deadline 150 ms: answered in {took:?}");
        assert!(
            matches!(got(&r), Got::NotSevered(ref c) if c == "timeout" || c == "pastdeadline"),
            "(ii) held={held}: {:?}",
            got(&r)
        );
        assert!(
            took < Duration::from_secs(1),
            "(ii) held={held}: the deadline wins, not the 2 s budget: {took:?}"
        );
        ctl.gate.release();
        settle();
    }
}

/// "Fixtures priced": a deadline of `Duration::ZERO` makes NO vault call — the
/// production-shaped decorator refuses every call before it reaches the key
/// store, so the fake counts zero — and answers `notSevered(pastDeadline)`,
/// nothing severed and no file touched, on both branches. With the lock held
/// the path is still tombstoned and the holder still poisoned (steps (i) and
/// (ii) cost no key-store call). A directory that never held a wallet, with no
/// vault at all, answers `alreadyGone` at zero too: nothing needs the key store.
#[tokio::test]
async fn a_zero_deadline_makes_no_vault_call() {
    let _serial = serial();
    settle();
    for held in [false, true] {
        let f = created(if held { "a0-held" } else { "a0-free" }).await;
        let w = if held { Some(f.open().await) } else { None };
        let ctl = Arc::new(Ctl::default());
        let before = f.files();
        let r = sever(
            &f.dir,
            Some(Arc::new(production(&f.keychain, &ctl, LONG, LONG))),
            Duration::ZERO,
        )
        .await;
        assert_eq!(
            ctl.calls(),
            Vec::new(),
            "held={held}: a zero deadline reached the key store"
        );
        assert_eq!(
            got(&r),
            Got::NotSevered("pastdeadline".into()),
            "held={held}"
        );
        assert!(left_for_host(&r), "held={held}: no file removed");
        assert_eq!(f.files(), before, "held={held}: files untouched");
        assert!(f.key_live(&f.id_ns), "held={held}: nothing severed");
        if let Some(w) = w {
            assert!(this_process(&r), "the open handle holds the lock");
            assert!(
                entry(&f.path_ns).is_some_and(|e| e.tombstoned),
                "the path is tombstoned at zero"
            );
            match w.current_address().await {
                Err(e) => assert!(is_wiped(&e), "the holder is poisoned at zero: {e:?}"),
                Ok(_) => panic!("the holder is poisoned at zero"),
            }
            w.close().await.expect("the poisoned handle closes");
            Wallet::wipe_resolving(&f.dir, Some(plain(&f.keychain)), false)
                .await
                .expect("the plain wipe clears the tombstone");
        } else {
            assert!(no_holder(&r), "the lock was free");
        }
    }

    // nothing on disk, no vault: nothing needs the key store
    let parent = tempfile::tempdir().expect("parent");
    let dir = parent.path().join("a0-never").join("wallet");
    std::fs::create_dir_all(&dir).expect("an empty directory");
    let r = sever(&dir, None, Duration::ZERO).await;
    assert_eq!(got(&r), Got::AlreadyGone, "an Empty store at zero");
}

/// §3.1 "Log": once the sever begins, its own work logs NOTHING. The fixture
/// is a zero deadline through the production-shaped decorator, whose every
/// refusal is a `wallet.vault_call` line raised on the CALLER's thread — here
/// the test's, where the capture listens. Unsilenced, the body logs (the
/// anti-vacuity half); through `sever_silenced`, nothing is captured.
#[tokio::test]
async fn the_sever_logs_nothing_once_it_begins() {
    let _serial = serial();
    settle();
    let f = created("log0").await;
    let ctl = Arc::new(Ctl::default());
    let stack = production(&f.keychain, &ctl, LONG, LONG);
    let resolver: &dyn VaultResolver = &stack;

    let (sink, guard) = capture();
    let loud = Wallet::sever_blocking(&f.dir, Some(resolver), Instant::now(), Duration::ZERO)
        .expect("a report");
    drop(guard);
    assert_eq!(got(&loud), Got::NotSevered("pastdeadline".into()));
    assert!(
        !sink.records_of("wallet.vault_call").is_empty(),
        "fixture: the unsilenced body logs its refusals"
    );

    let (sink, _guard) = capture();
    let quiet = Wallet::sever_silenced(&f.dir, Some(resolver), Instant::now(), Duration::ZERO)
        .expect("a report");
    assert_eq!(got(&quiet), Got::NotSevered("pastdeadline".into()));
    assert_eq!(
        sink.fields(),
        Vec::<(String, String)>::new(),
        "the sever logged after it began"
    );
}

/// §3.1 item 5: a nested scope keeps the EARLIER end, whichever order the two
/// scopes are opened in.
#[test]
fn a_nested_wipe_scope_keeps_the_earlier_end() {
    let _serial = serial();
    settle();
    let kc = SharedKeychainVault::shared();
    let ns = keychain_namespace_for(Path::new("/s16/nested"));
    let wedged = |ctl: &Arc<Ctl>| {
        bounded(
            Arc::new(Fake {
                inner: shared_mk(&kc, ns.as_str()),
                ns: ns.as_str().to_owned(),
                ctl: Arc::clone(ctl),
            }),
            // Bounds far above the scopes' ends and below the anti-hang
            // ceiling: a scope that is ignored answers here, not at 150 ms.
            Duration::from_secs(10),
            Duration::from_secs(10),
            &ns,
        )
    };

    // early outside, late inside
    {
        let ctl = Ctl::wedging(Method::LoadIndex);
        let _release = ReleaseOnDrop(Arc::clone(&ctl));
        let v = wedged(&ctl);
        let started = Instant::now();
        let _outer = wipe_scope_until(started + Duration::from_millis(150));
        let _inner = wipe_scope_until(started + Duration::from_secs(5));
        let e = v.load_index().err().expect("the call is wedged");
        let took = started.elapsed();
        assert!(timeout_cause(&e).is_some(), "{e:?}");
        assert!(
            took < Duration::from_secs(2),
            "an inner scope must not extend the outer end: {took:?}"
        );
        ctl.gate.release();
    }
    settle();

    // late outside, early inside
    {
        let ctl = Ctl::wedging(Method::LoadIndex);
        let _release = ReleaseOnDrop(Arc::clone(&ctl));
        let v = wedged(&ctl);
        let started = Instant::now();
        let _outer = wipe_scope_until(started + Duration::from_secs(60));
        let _inner = wipe_scope_until(started + Duration::from_millis(150));
        let e = v.load_index().err().expect("the call is wedged");
        let took = started.elapsed();
        assert!(timeout_cause(&e).is_some(), "{e:?}");
        assert!(
            took < Duration::from_secs(2),
            "the inner end holds: {took:?}"
        );
        ctl.gate.release();
    }
    settle();
}

/// Assertion 11. A real wallet with no vault reachable:
/// `notSevered(vaultAbsent)`, files untouched — on both branches.
#[tokio::test]
async fn sever_with_a_real_wallet_and_no_vault_reports_not_severed() {
    let _serial = serial();
    for held in [false, true] {
        let f = created(if held { "a11-held" } else { "a11-free" }).await;
        let _lock = held.then(|| WalletLock::acquire(&f.dir).expect("another holder"));
        let before = f.files();
        let r = sever(&f.dir, None, DEADLINE).await;
        assert_eq!(
            got(&r),
            Got::NotSevered("vaultabsent".into()),
            "held={held}"
        );
        assert_eq!(f.files(), before, "held={held}: files untouched");
        assert!(f.key_live(&f.id_ns), "held={held}: nothing severed");
    }
}

/// Assertion 12. After a sever, the host purges the directory (`leftForHost`)
/// and a `create` at the same path in this process refuses `Wiped`, custodying
/// nothing; a plain `wipe` of the path clears the tombstone and `create` then
/// succeeds.
#[tokio::test]
async fn create_at_a_severed_path_refuses_until_a_wipe_completes() {
    let _serial = serial();
    settle();
    let f = created("a12").await;
    let w = f.open().await;
    let r = sever(&f.dir, Some(plain(&f.keychain)), DEADLINE).await;
    assert!(left_for_host(&r));
    w.close().await.expect("the poisoned handle closes");
    std::fs::remove_dir_all(&f.dir).expect("the host's own purge");

    let ctl = Arc::new(Ctl::default());
    let resolver: Arc<dyn VaultResolver> = Arc::new(production(&f.keychain, &ctl, LONG, LONG));
    match Wallet::create_resolving(f.config(), raw_seed(), Arc::clone(&resolver)).await {
        Err(e) => assert!(is_wiped(&e), "create at a tombstoned path: {e:?}"),
        Ok(_) => panic!("create at a tombstoned path must refuse"),
    }
    assert!(f.no_key(), "the refused create custodied nothing");

    Wallet::wipe_resolving(&f.dir, Some(Arc::clone(&resolver)), false)
        .await
        .expect("the plain wipe of the path");
    Wallet::create_resolving(f.config(), raw_seed(), resolver)
        .await
        .expect("after the wipe, create succeeds")
        .close()
        .await
        .expect("close");
    settle();
}

/// Assertion 13. Idempotence on BOTH branches: a second sever on a severed
/// directory reports `alreadyGone` — lock held (the breadcrumb) or free (the
/// directory Empty) — and no vault call reports a non-zero count in it.
#[tokio::test]
async fn a_second_sever_reports_already_gone() {
    let _serial = serial();

    // lock held
    let f = created("a13-held").await;
    let w = f.open().await;
    let first = sever(&f.dir, Some(plain(&f.keychain)), DEADLINE).await;
    assert!(matches!(got(&first), Got::Severed(_)), "{:?}", got(&first));
    let ctl = Arc::new(Ctl::default());
    let second = sever(&f.dir, Some(observed(&f.keychain, &ctl)), DEADLINE).await;
    assert_eq!(got(&second), Got::AlreadyGone, "held: second sever");
    assert!(
        ctl.nonzero_purges().is_empty(),
        "held: nothing left to sever"
    );
    drop(w);

    // lock free
    let f = created("a13-free").await;
    let first = sever(&f.dir, Some(plain(&f.keychain)), DEADLINE).await;
    assert!(matches!(got(&first), Got::Severed(_)), "{:?}", got(&first));
    let ctl = Arc::new(Ctl::default());
    let second = sever(&f.dir, Some(observed(&f.keychain, &ctl)), DEADLINE).await;
    assert_eq!(got(&second), Got::AlreadyGone, "free: second sever");
    assert!(
        ctl.nonzero_purges().is_empty(),
        "free: nothing left to sever"
    );
}

/// Assertion 13's last sentence: a directory that never held a wallet reports
/// `alreadyGone`, never `severed`.
#[tokio::test]
async fn a_free_lock_sever_of_nothing_reports_already_gone() {
    let _serial = serial();
    let keychain = SharedKeychainVault::shared();
    let parent = tempfile::tempdir().expect("parent");
    let dir = parent.path().join("never").join("wallet");
    std::fs::create_dir_all(&dir).expect("an empty directory");
    let ctl = Arc::new(Ctl::default());
    let r = sever(&dir, Some(observed(&keychain, &ctl)), DEADLINE).await;
    assert_eq!(got(&r), Got::AlreadyGone);
    assert!(no_holder(&r));
    assert!(
        ctl.nonzero_purges().is_empty(),
        "no vault call reports a non-zero count"
    );
}

/// Assertion 14. A locked keychain (the index read answers
/// `KeystoreUnavailable`; every purge answers `Ok(0)` — the key is filtered,
/// not gone): the sever still issues its deletes, ONLY under the path
/// namespace and the namespace the header names, and reports
/// `severedUnproven(countUnreadable)` with no breadcrumb and no file removed —
/// with the lock held, and with it free. On both, an open in this process then
/// refuses `Wiped`: the hidden key is not recoverable here. One delete that
/// fails → `notSevered(keystoreUnavailable)`.
#[tokio::test]
async fn a_locked_keychain_gets_a_blind_scoped_delete_reported_unproven() {
    let _serial = serial();
    for held in [true, false] {
        let f = created(if held { "a14-held" } else { "a14-free" }).await;
        let w = if held { Some(f.open().await) } else { None };
        let ctl = Arc::new(Ctl::default());
        ctl.lock_up(Method::LoadIndex);
        ctl.zero_purge.store(true, Ordering::SeqCst);
        let before = f.files();
        let r = sever(&f.dir, Some(observed(&f.keychain, &ctl)), DEADLINE).await;
        assert_eq!(
            got(&r),
            Got::Unproven("countunreadable".into()),
            "held={held}"
        );
        assert!(
            left_for_host(&r),
            "held={held}: no file removed on a blind sever"
        );
        assert!(
            !f.breadcrumb(),
            "held={held}: no breadcrumb on a blind sever"
        );
        assert_eq!(f.files(), before, "held={held}: files untouched");
        let deletes: Vec<(Method, String)> = ctl
            .calls()
            .into_iter()
            .filter(|(m, _)| m.class() == Class::Delete)
            .collect();
        assert!(!deletes.is_empty(), "held={held}: the deletes were issued");
        for (m, ns) in &deletes {
            assert!(
                ns == f.path_ns.as_str() || ns == f.id_ns.as_str(),
                "held={held}: {m:?} under a namespace that is not this wallet's: {ns}"
            );
        }
        assert!(
            deletes
                .iter()
                .any(|(m, ns)| *m == Method::PurgeNamespace && ns == f.id_ns.as_str()),
            "held={held}: the header's namespace was purged: {deletes:?}"
        );
        drop(w);
        // The key the lock hid survived the blind delete (the purge answered
        // 0). In this process an open must not recover it, on EITHER branch:
        // the blind path tombstones the path itself (the S16 diff review's
        // CRITICAL — the free-lock branch has no step (i) before it).
        assert!(f.key_live(&f.id_ns), "held={held}: fixture: the hidden key");
        match Wallet::open_resolving(f.config(), plain(&f.keychain), None, None).await {
            Err(e) => assert!(
                is_wiped(&e),
                "held={held}: an open after an unproven sever: {e:?}"
            ),
            Ok(_) => panic!("held={held}: an open after an unproven sever recovered the wallet"),
        }
    }

    // one delete fails
    let f = created("a14-fail").await;
    let w = f.open().await;
    let ctl = Arc::new(Ctl::default());
    ctl.lock_up(Method::LoadIndex);
    ctl.zero_purge.store(true, Ordering::SeqCst);
    *ctl.failing_purge.lock().expect("failing") = Some(f.id_ns.as_str().to_owned());
    let r = sever(&f.dir, Some(observed(&f.keychain, &ctl)), DEADLINE).await;
    assert_eq!(got(&r), Got::NotSevered("keystoreunavailable".into()));
    assert!(!f.breadcrumb());
    drop(w);
}

/// Assertion 15. On a tombstoned namespace, through the decorator: every
/// DELETE and every READ reaches the vault; every CREATE answers `Wiped`
/// without reaching it. The list is `Method::all()` (11), classified exhaustively.
/// A namespace nobody severed (the selftest's shape) still takes creates.
#[tokio::test]
async fn a_tombstone_stops_creates_at_the_worker_and_lets_deletes_through() {
    let _serial = serial();
    settle();
    let f = created("a15").await;
    let a = artifact(&f.dir);
    let _held = WalletLock::acquire(&f.dir).expect("another holder");
    let r = sever(&f.dir, Some(plain(&f.keychain)), DEADLINE).await;
    assert!(matches!(got(&r), Got::Severed(_)), "fixture: {:?}", got(&r));

    for ns in [&f.path_ns, &f.id_ns] {
        for m in Method::all() {
            let ctl = Arc::new(Ctl::default());
            let v = bounded(
                Arc::new(Fake {
                    inner: Arc::new(TestVault::new(VaultTier::Tee)),
                    ns: ns.as_str().to_owned(),
                    ctl: Arc::clone(&ctl),
                }),
                LONG,
                LONG,
                ns,
            );
            let out: Result<(), WalletError> = match m {
                Method::Probe => v.probe(),
                Method::Tier => v.tier().map(|_| ()),
                Method::StoreWrapKey => v.store_wrap_key(SealKey::generate(), BLOB).map(|_| ()),
                Method::LoadWrapKey => v.load_wrap_key(&a, BLOB).map(|_| ()),
                Method::RotateWrapKey => v.rotate_wrap_key(&a, BLOB).map(|_| ()),
                Method::FinishRotation => v.finish_rotation(&a, &a),
                Method::DeleteWrapKey => v.delete_wrap_key(&a),
                Method::PurgeNamespace => v.purge_namespace().map(|_| ()),
                Method::StoreIndex => {
                    v.store_index(&CustodyIndexEntry::done(&CustodyId::generate()))
                }
                Method::LoadIndex => v.load_index().map(|_| ()),
                Method::DeleteIndex => v.delete_index(),
            };
            match m.class() {
                Class::Create => {
                    match &out {
                        Err(e) => assert!(is_wiped(e), "{m:?} on a tombstone: {e:?}"),
                        Ok(()) => panic!("{m:?} on a tombstone must answer Wiped"),
                    }
                    assert_eq!(ctl.count(m), 0, "{m:?} never reached the vault");
                }
                Class::Delete | Class::Read => {
                    if let Err(e) = &out {
                        assert!(!is_wiped(e), "{m:?} is not a create: {e:?}");
                    }
                    assert_eq!(ctl.count(m), 1, "{m:?} reached the vault");
                }
            }
        }
    }

    let fresh = crate::custody::namespace_for(&CustodyId::generate());
    let v = bounded(Arc::new(TestVault::new(VaultTier::Tee)), LONG, LONG, &fresh);
    v.store_wrap_key(SealKey::generate(), BLOB)
        .expect("a namespace nobody severed is not refused");
    settle();
}

/// B's close, a fresh open of B (the birth gate and the token) and its
/// `current_address`, each bounded by `UNRELATED_OP`.
async fn b_is_usable(
    b: Wallet,
    config: WalletConfig,
    vault: &Arc<dyn KeychainPort>,
    what: &str,
) -> Wallet {
    let started = Instant::now();
    tokio::time::timeout(UNRELATED_OP, b.close())
        .await
        .unwrap_or_else(|_| panic!("{what}: B's close waited on the path table"))
        .expect("close b");
    let b = tokio::time::timeout(
        UNRELATED_OP,
        Wallet::open_with_vault(config, Arc::clone(vault)),
    )
    .await
    .unwrap_or_else(|_| panic!("{what}: B's open waited on the path table"))
    .expect("open b");
    tokio::time::timeout(UNRELATED_OP, b.current_address())
        .await
        .unwrap_or_else(|_| panic!("{what}: B's current_address waited"))
        .expect("address");
    println!(
        "{what}: B closed, reopened and answered in {:?}",
        started.elapsed()
    );
    b
}

/// Assertion 16. `PATHS` is never held across a key-store call. Wallet B, at
/// another directory over a `*_with_vault` `TestVault` (it never needs the
/// worker), stays usable — its `close`, a fresh open and `current_address`
/// each finish — while (a) wallet A's sever sits wedged inside its purge, and
/// (b) a gated `store_index` job is wedged inside its inner call.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_path_table_is_never_held_across_a_keychain_call() {
    let _serial = serial();
    settle();
    let dir_b = tempfile::tempdir().expect("b");
    let vault_b = test_vault();
    let config_b = || cfg(dir_b.path(), Network::Test, SeedPersistence::SealedKeychain);
    let b = Wallet::create_with_vault(config_b(), raw_seed(), Arc::clone(&vault_b))
        .await
        .expect("create b");

    // (a) A's sever, wedged inside its purge
    let f = created("a16").await;
    let _held = WalletLock::acquire(&f.dir).expect("another holder");
    let ctl = Ctl::wedging(Method::PurgeNamespace);
    let _release = ReleaseOnDrop(Arc::clone(&ctl));
    let severing = {
        let dir = f.dir.clone();
        let resolver: Arc<dyn VaultResolver> = Arc::new(production(&f.keychain, &ctl, LONG, LONG));
        tokio::spawn(async move { sever(&dir, Some(resolver), DEADLINE).await })
    };
    ctl.wait_entered(Method::PurgeNamespace);
    let b = b_is_usable(b, config_b(), &vault_b, "(a)").await;
    ctl.gate.release();
    let _ = severing.await.expect("the sever's task");
    settle();

    // (b) a gated store_index job, wedged inside the vault
    let ctl = Ctl::wedging(Method::StoreIndex);
    let _release = ReleaseOnDrop(Arc::clone(&ctl));
    let other = keychain_namespace_for(Path::new("/s16/a16-other"));
    let writer = {
        let v = bounded(
            Arc::new(Fake {
                inner: Arc::new(TestVault::new(VaultTier::Tee)),
                ns: other.as_str().to_owned(),
                ctl: Arc::clone(&ctl),
            }),
            LONG,
            LONG,
            &other,
        );
        std::thread::spawn(move || v.store_index(&CustodyIndexEntry::done(&CustodyId::generate())))
    };
    ctl.wait_entered(Method::StoreIndex);
    let b = b_is_usable(b, config_b(), &vault_b, "(b)").await;
    ctl.gate.release();
    let _ = writer.join().expect("the writer's thread");
    settle();
    b.close().await.expect("close b");
}

/// Assertion 17 (§3.1 item 9). The table does not grow: 1 000 distinct paths
/// opened (each an Empty directory, so the open is refused `NotFound` after
/// the token was bumped) plus 20 real wallets created and closed leave no
/// entry for any of them; 100 open/close cycles on one path leave at most one
/// entry holding at most one `Weak`, and none once closed. A severed path's
/// entry survives until a plain `wipe` of it, then goes.
#[tokio::test]
async fn the_path_table_does_not_grow_across_distinct_wallets() {
    let _serial = serial();
    let keychain = SharedKeychainVault::shared();
    let parent = tempfile::tempdir().expect("parent");

    let mut spaces = Vec::new();
    for i in 0..1_000 {
        let dir = parent.path().join(format!("empty-{i}"));
        match Wallet::open_resolving(
            cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
            plain(&keychain),
            None,
            None,
        )
        .await
        {
            Err(WalletError::NotFound) => {}
            Err(e) => panic!("fixture: an Empty directory opens NotFound, got {e:?}"),
            Ok(_) => panic!("fixture: an Empty directory must not open"),
        }
        spaces.push(keychain_namespace_for(&dir));
    }
    for i in 0..20 {
        let dir = parent.path().join(format!("real-{i}"));
        Wallet::create_resolving(
            cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
            raw_seed(),
            plain(&keychain),
        )
        .await
        .expect("create")
        .close()
        .await
        .expect("close");
        spaces.push(keychain_namespace_for(&dir));
    }
    for ns in &spaces {
        assert_eq!(entry(ns), None, "an entry outlived its wallet");
    }

    let f = created("a17-cycles").await;
    for _ in 0..100 {
        let w = f.open().await;
        let e = entry(&f.path_ns).expect("an open wallet is registered");
        assert!(e.live <= 1, "at most one Weak per open wallet: {e:?}");
        w.close().await.expect("close");
    }
    assert_eq!(entry(&f.path_ns), None, "no entry once closed");

    let held = WalletLock::acquire(&f.dir).expect("another holder");
    let r = sever(&f.dir, Some(plain(&f.keychain)), DEADLINE).await;
    assert!(matches!(got(&r), Got::Severed(_)));
    drop(held);
    assert!(
        entry(&f.path_ns).is_some_and(|e| e.tombstoned),
        "a severed path's tombstone stands"
    );
    Wallet::wipe_resolving(&f.dir, Some(plain(&f.keychain)), false)
        .await
        .expect("the plain wipe");
    assert_eq!(entry(&f.path_ns), None, "the wipe clears it");
    assert_eq!(entry(&f.id_ns), None, "and the id's tombstone with it");
}

/// Assertion 18. Another wallet's wedge on the SHARED worker: B's call is
/// wedged in the key store and its caller has left, so the worker is `busy`.
/// A's sever (2 s deadline) answers `notSevered(busy)` well inside the
/// deadline, making no call on A's vault; a retry while the wedge holds says
/// `busy` again; once it is released, a retry reports `severed`.
#[tokio::test]
async fn another_wallets_wedge_on_the_shared_worker_makes_the_sever_answer_busy_in_time() {
    let _serial = serial();
    settle();
    let f = created("a18").await;
    let _held = WalletLock::acquire(&f.dir).expect("another holder");

    let wedge = Ctl::wedging(Method::LoadIndex);
    let _release = ReleaseOnDrop(Arc::clone(&wedge));
    let ns_b = keychain_namespace_for(Path::new("/s16/a18-b"));
    let b = bounded(
        Arc::new(Fake {
            inner: Arc::new(TestVault::new(VaultTier::Tee)),
            ns: ns_b.as_str().to_owned(),
            ctl: Arc::clone(&wedge),
        }),
        SHORT,
        SHORT,
        &ns_b,
    );
    let left = std::thread::spawn(move || b.load_index().map(|_| ()))
        .join()
        .expect("b's thread");
    assert_eq!(
        left.as_ref().err().and_then(timeout_cause).as_deref(),
        Some("timeout"),
        "fixture: B's caller left"
    );

    let a_ctl = Arc::new(Ctl::default());
    let resolver =
        || -> Arc<dyn VaultResolver> { Arc::new(production(&f.keychain, &a_ctl, LONG, LONG)) };
    let started = Instant::now();
    let r = sever(&f.dir, Some(resolver()), DEADLINE).await;
    let took = started.elapsed();
    assert_eq!(got(&r), Got::NotSevered("busy".into()), "behind B's wedge");
    assert!(
        took < Duration::from_secs(1),
        "at once, not at the deadline: {took:?}"
    );
    assert!(
        a_ctl.calls().is_empty(),
        "no call reached A's vault: {:?}",
        a_ctl.calls()
    );
    let r = sever(&f.dir, Some(resolver()), DEADLINE).await;
    assert_eq!(
        got(&r),
        Got::NotSevered("busy".into()),
        "a retry while it holds"
    );

    wedge.gate.release();
    settle();
    let r = sever(&f.dir, Some(resolver()), DEADLINE).await;
    assert!(
        matches!(got(&r), Got::Severed(n) if n > 0),
        "once B's call returns: {:?}",
        got(&r)
    );
    assert!(!f.key_live(&f.id_ns));
}

// ═════════════════════════════ §3.2 `poison` ═════════════════════════════

/// Assertion 1. A live `Open` handle in this process: after the sever every
/// call on it answers `InvalidState { Wiped }` — `current_address`, a
/// `propose`, the transparent address — and the report says `thisProcess`.
/// Its `close` still returns `Ok` and frees the lock.
#[tokio::test]
async fn a_severed_live_handle_refuses_every_operation_as_wiped() {
    let _serial = serial();
    let f = created("p1").await;
    let w = f.open().await;
    let r = sever(&f.dir, Some(plain(&f.keychain)), DEADLINE).await;
    assert!(this_process(&r));

    for (what, e) in [
        ("current_address", w.current_address().await.err()),
        ("propose_shield", w.propose_shield().await.err()),
        (
            "current_transparent_address",
            w.current_transparent_address().await.err(),
        ),
    ] {
        match e {
            Some(e) => assert!(is_wiped(&e), "{what}: {e:?}"),
            None => panic!("{what} answered on a severed wallet"),
        }
    }
    w.close().await.expect("a poisoned handle closes Ok");
    WalletLock::acquire(&f.dir).expect("the lock is free after the close");
}

/// Assertion 2. A straggler past `close` (phase `Closing`: the close's drain
/// ran out while the clone held on): the sever finds it, and a `from_shared`
/// wrapper's next call answers `Wiped`.
#[tokio::test(start_paused = true)]
async fn a_severed_straggler_refuses_its_next_operation() {
    let _serial = serial();
    let f = created("p2").await;
    let w = f.open().await;
    let straggler = Arc::clone(&w.inner);
    w.close().await.expect("a wedged ref does not fail close");
    assert_eq!(
        straggler.lifecycle.phase(),
        LifecyclePhase::Closing,
        "fixture"
    );

    let r = sever(&f.dir, Some(plain(&f.keychain)), DEADLINE).await;
    assert!(this_process(&r), "the straggler is this process's");
    match Wallet::from_shared(Arc::clone(&straggler))
        .current_address()
        .await
    {
        Err(e) => assert!(is_wiped(&e), "{e:?}"),
        Ok(_) => panic!("a severed straggler answered"),
    }
    drop(straggler);
}

/// Assertion 3. The lock held by a bare `WalletLock::acquire` — no registered
/// `Inner`, no opening token, the other-process shape — reads `otherProcess`,
/// while the same sever beside an open handle reads `thisProcess`; the
/// keychain rows hold either way.
#[tokio::test]
async fn sever_tells_another_process_holder_from_this_one() {
    let _serial = serial();
    let f = created("p3-other").await;
    let lock = WalletLock::acquire(&f.dir).expect("the bare holder");
    let r = sever(&f.dir, Some(plain(&f.keychain)), DEADLINE).await;
    assert!(
        other_process(&r),
        "a bare lock is not this process's wallet"
    );
    assert!(matches!(got(&r), Got::Severed(_)), "{:?}", got(&r));
    assert!(left_for_host(&r));
    assert!(!f.key_live(&f.id_ns));
    drop(lock);

    let f = created("p3-this").await;
    let w = f.open().await;
    let r = sever(&f.dir, Some(plain(&f.keychain)), DEADLINE).await;
    assert!(this_process(&r), "an open handle is this process's");
    assert!(matches!(got(&r), Got::Severed(_)));
    assert!(!f.key_live(&f.id_ns));
    drop(w);
}

/// Poll until `inner`'s phase is `phase` (the rebuild entered its drain).
async fn phase_reaches(inner: &Arc<Inner>, phase: LifecyclePhase) {
    let started = Instant::now();
    while inner.lifecycle.phase() != phase {
        assert!(
            started.elapsed() < ANSWER_CEILING,
            "never reached {phase:?}"
        );
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
}

/// Assertion 4 (i), the rescan: severed DURING its drain (a straggler holds
/// the `Inner`, so the drain is waiting). No post-drain re-check (removed at the
/// S16 fold): the rebuild's re-open fails on the severed custody and the ONE pin of
/// `rescan_from`'s any-error mapping answers `Wiped`; no handle, no key appears.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_rescan_severed_during_its_drain_yields_no_live_handle() {
    let _serial = serial();
    let f = created("p4-rescan-drain").await;
    let w = f.open().await;
    let straggler = Arc::clone(&w.inner);
    let resolver = plain(&f.keychain);
    let rescan = tokio::spawn(async move { w.rescan_from_with_resolver(None, resolver).await });
    phase_reaches(&straggler, LifecyclePhase::Rescanning).await;

    let r = sever(&f.dir, Some(plain(&f.keychain)), DEADLINE).await;
    assert!(this_process(&r));
    drop(straggler);

    match tokio::time::timeout(ANSWER_CEILING, rescan)
        .await
        .expect("the rescan answers")
        .expect("the rescan's task")
    {
        Err(e) => assert!(is_wiped(&e), "{e:?}"),
        Ok(_) => panic!("a rescan severed during its drain came back working"),
    }
    assert!(f.no_key(), "no key appears");
    assert!(f.index().is_none(), "no index appears");
}

/// Assertion 4 (ii), the rescan: severed after its unwrap, right before it
/// assembles the rebuilt handle. The birth gate refuses it: `Wiped`, no
/// working handle, `thisProcess` (the rebuild's lock carries the token).
#[tokio::test]
async fn a_rescan_severed_after_its_unwrap_yields_no_live_handle() {
    let _serial = serial();
    let f = created("p4-rescan-after").await;
    let w = f.open().await;
    let slot: Slot = Arc::default();
    let _armed = Armed::new(
        Seam::RebuildBeforeAssemble,
        &f.dir,
        severing_hook(&f.dir, plain(&f.keychain), &slot, 1),
    );

    match w.rescan_from_with_resolver(None, plain(&f.keychain)).await {
        Err(e) => assert!(is_wiped(&e), "{e:?}"),
        Ok(_) => panic!("a rescan severed after its unwrap came back working"),
    }
    let r = taken(&slot);
    assert!(this_process(&r), "the rebuild's lock is this process's");
    assert!(f.no_key(), "no key appears");
    assert!(f.index().is_none(), "no index appears");
}

/// Assertion 4 (i), the server switch: severed during its drain.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_switch_severed_during_its_drain_yields_no_live_handle() {
    let _serial = serial();
    let f = created("p4-switch-drain").await;
    let w = f.open().await;
    let straggler = Arc::clone(&w.inner);
    let switch = tokio::spawn(async move {
        let mut oracle = provision::testing::FakeOracle::honest(Network::Test, 3_000_000);
        w.switch_sync_server_inner(custom("https://mine.example:443"), Some(&mut oracle))
            .await
    });
    phase_reaches(&straggler, LifecyclePhase::SwitchingServer).await;

    let r = sever(&f.dir, Some(plain(&f.keychain)), DEADLINE).await;
    assert!(this_process(&r));
    drop(straggler);

    match tokio::time::timeout(ANSWER_CEILING, switch)
        .await
        .expect("the switch answers")
        .expect("the switch's task")
    {
        Err(SwitchRefused { wallet, error }) => {
            assert!(is_wiped(&error), "{error:?}");
            assert!(wallet.is_none(), "no handle comes back");
        }
        Ok(_) => panic!("a switch severed during its drain came back working"),
    }
    assert!(f.no_key(), "no key appears");
}

/// Assertion 4 (ii), the server switch: severed after its unwrap. The switch
/// never touches the key store, so the birth gate is the only thing that can
/// refuse it.
#[tokio::test]
async fn a_switch_severed_after_its_unwrap_yields_no_live_handle() {
    let _serial = serial();
    let f = created("p4-switch-after").await;
    let w = f.open().await;
    let slot: Slot = Arc::default();
    let _armed = Armed::new(
        Seam::RebuildBeforeAssemble,
        &f.dir,
        severing_hook(&f.dir, plain(&f.keychain), &slot, 1),
    );
    let mut oracle = provision::testing::FakeOracle::honest(Network::Test, 3_000_000);
    match w
        .switch_sync_server_inner(custom("https://mine.example:443"), Some(&mut oracle))
        .await
    {
        Err(SwitchRefused { wallet, error }) => {
            assert!(is_wiped(&error), "{error:?}");
            assert!(wallet.is_none(), "no handle comes back");
        }
        Ok(_) => panic!("a switch severed after its unwrap came back working"),
    }
    assert!(this_process(&taken(&slot)));
    assert!(f.no_key(), "no key appears");
}

/// Assertion 5, first order: poison, then close. `close` returns `Ok`, the
/// phase still reads `Wiped` afterwards (never the retryable `Closing`), and
/// the lock frees once the row drops its last clone.
#[tokio::test(start_paused = true)]
async fn close_after_poison_keeps_wiped_drains_and_frees_the_lock() {
    let _serial = serial();
    let f = created("p5-poison-close").await;
    let w = f.open().await;
    let clone = Arc::clone(&w.inner);
    let r = sever(&f.dir, Some(plain(&f.keychain)), DEADLINE).await;
    assert!(this_process(&r));
    assert_eq!(clone.lifecycle.phase(), LifecyclePhase::Wiped, "poisoned");

    w.close().await.expect("close on a poisoned handle is Ok");
    assert_eq!(
        clone.lifecycle.phase(),
        LifecyclePhase::Wiped,
        "close must not regress a poisoned handle to Closing"
    );
    assert!(
        matches!(
            WalletLock::acquire(&f.dir),
            Err(WalletError::WalletAlreadyOpen)
        ),
        "the clone still holds the lock"
    );
    drop(clone);
    WalletLock::acquire(&f.dir).expect("the lock frees with the last clone");
}

/// Assertion 5, second order: close, then poison (the `(Closing, Wiped)`
/// edge). A straggler past the close gets `Wiped`.
#[tokio::test(start_paused = true)]
async fn poison_after_close_refuses_stragglers_as_wiped() {
    let _serial = serial();
    let f = created("p5-close-poison").await;
    let w = f.open().await;
    let straggler = Arc::clone(&w.inner);
    w.close().await.expect("close");
    assert_eq!(
        straggler.lifecycle.phase(),
        LifecyclePhase::Closing,
        "fixture"
    );
    let r = sever(&f.dir, Some(plain(&f.keychain)), DEADLINE).await;
    assert!(this_process(&r));
    assert_eq!(straggler.lifecycle.phase(), LifecyclePhase::Wiped);
    match Wallet::from_shared(Arc::clone(&straggler))
        .propose_shield()
        .await
    {
        Err(e) => assert!(is_wiped(&e), "{e:?}"),
        Ok(_) => panic!("a straggler past close answered after the sever"),
    }
    drop(straggler);
    WalletLock::acquire(&f.dir).expect("the lock frees");
}

/// Assertion 6. An open in flight at the sever, paused (a) after its blocking
/// section returned and before `from_open_carrying`, (b) between
/// `Arc::new_cyclic` and the birth gate (the TOCTOU row: an entry-placed check
/// would pass here). In both the open returns `Wiped`, the refused `Inner` is
/// dropped without a deadlock (a 5 s watchdog), and the report says
/// `thisProcess`. The watch-only early return releases its token too.
#[tokio::test]
async fn an_open_in_flight_at_the_sever_is_refused_and_reported_this_process() {
    let _serial = serial();
    for point in [Seam::OpenAfterAcquire, Seam::OpenBeforeBirthGate] {
        let f = created(&format!("p6-{point:?}")).await;
        let slot: Slot = Arc::default();
        let armed = Armed::new(
            point,
            &f.dir,
            severing_hook(&f.dir, plain(&f.keychain), &slot, 1),
        );
        let opened = tokio::time::timeout(
            Duration::from_secs(5),
            Wallet::open_resolving(f.config(), plain(&f.keychain), None, None),
        )
        .await
        .unwrap_or_else(|_| panic!("{point:?}: the refused open deadlocked"));
        drop(armed);
        match opened {
            Err(e) => assert!(is_wiped(&e), "{point:?}: {e:?}"),
            Ok(_) => panic!("{point:?}: an open in flight at the sever came back working"),
        }
        let r = taken(&slot);
        assert!(this_process(&r), "{point:?}: the opener is this process's");
        assert!(!f.key_live(&f.id_ns), "{point:?}: severed");
        assert!(
            entry(&f.path_ns).is_none_or(|e| e.opening == 0 && e.live == 0),
            "{point:?}: the refused open released its token: {:?}",
            entry(&f.path_ns)
        );
    }

    // the watch-only early return
    let (vault, dir) = (test_vault(), tempfile::tempdir().expect("dir"));
    watch_only_at(dir.path(), &vault).await;
    let ns = keychain_namespace_for(dir.path());
    match Wallet::open_resolving(
        cfg(dir.path(), Network::Main, SeedPersistence::SealedKeychain),
        Arc::new(FixedVault(Arc::clone(&vault))),
        Some(Arc::new(crate::seed::testing::UnavailableSeedPort)),
        None,
    )
    .await
    {
        Err(WalletError::WatchOnly) => {}
        Err(e) => panic!("fixture: the watch-only early return, got {e:?}"),
        Ok(_) => panic!("fixture: a seed port on a watch-only store is refused"),
    }
    assert!(
        entry(&ns).is_none_or(|e| e.opening == 0 && e.live == 0),
        "the watch-only early return released its token: {:?}",
        entry(&ns)
    );
}

/// A watch-only wallet at `dir` (the twin of a spending wallet), closed.
async fn watch_only_at(dir: &Path, vault: &Arc<dyn KeychainPort>) {
    let spend_dir = tempfile::tempdir().expect("spend dir");
    let a = Wallet::create_with_vault(
        cfg(
            spend_dir.path(),
            Network::Main,
            SeedPersistence::SealedKeychain,
        ),
        SeedSource::raw_bytes(vec![0x42u8; 32]).expect("seed"),
        Arc::clone(vault),
    )
    .await
    .expect("create the spending wallet");
    let ufvk = a.export_ufvk().await.expect("export");
    a.close().await.expect("close");
    Wallet::create_watch_only_with_vault(
        cfg(dir, Network::Main, SeedPersistence::SealedKeychain),
        &ufvk,
        BlockHeight::new(2_000_000),
        Arc::clone(vault),
    )
    .await
    .expect("create the watch-only twin")
    .close()
    .await
    .expect("close");
}

fn kind(r: &TxSubmitResult) -> &'static str {
    match r {
        TxSubmitResult::Success { .. } => "success",
        TxSubmitResult::GrpcFailure { .. } => "grpc_failure",
        TxSubmitResult::SubmitFailure { .. } => "submit_failure",
        TxSubmitResult::NotAttempted { .. } => "not_attempted",
    }
}

/// Assertion 7. A severed wallet broadcasts nothing. The sever runs from the
/// broadcast hook: (a) just before `broadcast_one`'s first check, on a
/// two-tx chain — neither tx reaches the endpoint; (b) between the connect and
/// `send_transaction` — the one tx never reaches it. In both the send answers
/// `InvalidState { Wiped }`, NOT `Ok` with `GrpcFailure` rows, and the send
/// logs NOTHING once the sever has run (Relim's ring must never record a wallet
/// line after its panic wipe; the refusal is the answer, not a log line — the
/// `wallet.broadcast_refused` event the contract first named is withdrawn).
/// (c) tx0 accepted, THEN the sever,
/// then tx1: `Ok(results)` with tx0 `Success` and tx1 `GrpcFailure` — the
/// accepted broadcast is never discarded.
#[tokio::test]
async fn a_severed_wallet_broadcasts_nothing() {
    let _serial = serial();

    // (a) before the first check, a two-tx chain
    {
        let (peer, endpoint) = LoopbackLightwalletd::serve(Answer::Accept).await;
        let dir = tempfile::tempdir().expect("dir");
        let vault = test_vault();
        let w = funded_wallet(&vault, config_over(dir.path(), TorPolicy::Off, endpoint)).await;
        let tx0 = persist_shield(&w).await;
        fund_another_utxo(&w, 0x41).await;
        let tx1 = persist_shield(&w).await;
        let raw0 = w.raw_tx_bytes(tx0).await.expect("raw0");
        let raw1 = w.raw_tx_bytes(tx1).await.expect("raw1");
        let slot: Slot = Arc::default();
        let (sink, _guard) = capture();
        let mark = Arc::new(AtomicUsize::new(usize::MAX));
        let _armed = Armed::new(
            Seam::BroadcastTop,
            dir.path(),
            marking(
                &sink,
                &mark,
                severing_hook(
                    dir.path(),
                    Arc::new(FixedVault(Arc::clone(&vault))),
                    &slot,
                    1,
                ),
            ),
        );
        match w.broadcast_persisted(vec![tx0, tx1]).await {
            Err(e) => assert!(is_wiped(&e), "(a) {e:?}"),
            Ok(results) => panic!("(a) a severed send must answer Wiped, got {results:?}"),
        }
        assert!(this_process(&taken(&slot)));
        assert_eq!(
            peer.times_seen(&raw0),
            0,
            "(a) tx0 never reached the endpoint"
        );
        assert_eq!(
            peer.times_seen(&raw1),
            0,
            "(a) tx1 never reached the endpoint"
        );
        assert_silent_after(&sink, &mark, "(a)");
    }

    // (b) between the connect and send_transaction
    {
        let (peer, endpoint) = LoopbackLightwalletd::serve(Answer::Accept).await;
        let dir = tempfile::tempdir().expect("dir");
        let vault = test_vault();
        let w = funded_wallet(&vault, config_over(dir.path(), TorPolicy::Off, endpoint)).await;
        let tx0 = persist_shield(&w).await;
        let raw0 = w.raw_tx_bytes(tx0).await.expect("raw0");
        let slot: Slot = Arc::default();
        let (sink, _guard) = capture();
        let mark = Arc::new(AtomicUsize::new(usize::MAX));
        let _armed = Armed::new(
            Seam::BroadcastBeforeSend,
            dir.path(),
            marking(
                &sink,
                &mark,
                severing_hook(
                    dir.path(),
                    Arc::new(FixedVault(Arc::clone(&vault))),
                    &slot,
                    1,
                ),
            ),
        );
        match w.broadcast_persisted(vec![tx0]).await {
            Err(e) => assert!(is_wiped(&e), "(b) {e:?}"),
            Ok(results) => panic!("(b) a severed send must answer Wiped, got {results:?}"),
        }
        assert!(this_process(&taken(&slot)));
        assert_eq!(
            peer.times_seen(&raw0),
            0,
            "(b) the tx never reached the endpoint"
        );
        assert_silent_after(&sink, &mark, "(b)");
    }

    // (c) tx0 accepted, then the sever, then tx1
    {
        let (peer, endpoint) = LoopbackLightwalletd::serve(Answer::Accept).await;
        let dir = tempfile::tempdir().expect("dir");
        let vault = test_vault();
        let w = funded_wallet(&vault, config_over(dir.path(), TorPolicy::Off, endpoint)).await;
        let tx0 = persist_shield(&w).await;
        fund_another_utxo(&w, 0x42).await;
        let tx1 = persist_shield(&w).await;
        let raw0 = w.raw_tx_bytes(tx0).await.expect("raw0");
        let raw1 = w.raw_tx_bytes(tx1).await.expect("raw1");
        let slot: Slot = Arc::default();
        let _armed = Armed::new(
            Seam::BroadcastTop,
            dir.path(),
            severing_hook(
                dir.path(),
                Arc::new(FixedVault(Arc::clone(&vault))),
                &slot,
                2,
            ),
        );
        let results = w
            .broadcast_persisted(vec![tx0, tx1])
            .await
            .expect("(c) an accepted broadcast is never discarded");
        assert_eq!(
            results.iter().map(kind).collect::<Vec<_>>(),
            ["success", "grpc_failure"],
            "(c) {results:?}"
        );
        assert!(this_process(&taken(&slot)));
        assert_eq!(peer.times_seen(&raw0), 1, "(c) tx0 went out once");
        assert_eq!(peer.times_seen(&raw1), 0, "(c) tx1 never went out");
    }
}

/// Assertion 9. The table leaks nothing and keeps nothing alive: 100 cycles
/// on one path — opens that close, and failed opens (the wrong network) —
/// and watch-only early returns on another leave no live `Weak` and
/// `opening == 0`; the lock is free after every close (a `Weak` never keeps
/// `Inner` alive).
#[tokio::test]
async fn the_path_table_holds_no_instance_alive() {
    let _serial = serial();
    let f = created("p9").await;
    for i in 0..100 {
        if i % 10 == 9 {
            match Wallet::open_resolving(
                cfg(&f.dir, Network::Main, SeedPersistence::SealedKeychain),
                plain(&f.keychain),
                None,
                None,
            )
            .await
            {
                Err(WalletError::NetworkMismatch) => {}
                Err(e) => panic!("fixture: the wrong network, got {e:?}"),
                Ok(_) => panic!("fixture: the wrong network must not open"),
            }
        } else {
            let w = f.open().await;
            assert!(
                entry(&f.path_ns).is_some_and(|e| e.live == 1),
                "the open wallet is registered: {:?}",
                entry(&f.path_ns)
            );
            w.close().await.expect("close");
            WalletLock::acquire(&f.dir).expect("a Weak never keeps the lock held");
        }
    }
    assert!(
        entry(&f.path_ns).is_none_or(|e| e.live == 0 && e.opening == 0),
        "{:?}",
        entry(&f.path_ns)
    );

    let (vault, dir) = (test_vault(), tempfile::tempdir().expect("dir"));
    watch_only_at(dir.path(), &vault).await;
    for _ in 0..5 {
        let refused = Wallet::open_resolving(
            cfg(dir.path(), Network::Main, SeedPersistence::SealedKeychain),
            Arc::new(FixedVault(Arc::clone(&vault))),
            Some(Arc::new(crate::seed::testing::UnavailableSeedPort)),
            None,
        )
        .await;
        assert!(matches!(refused, Err(WalletError::WatchOnly)), "fixture");
    }
    let ns = keychain_namespace_for(dir.path());
    assert!(
        entry(&ns).is_none_or(|e| e.live == 0 && e.opening == 0),
        "{:?}",
        entry(&ns)
    );
}

/// Assertion 9's I/O fault: a `db_dir` whose parent is a regular file makes
/// `acquire_opening`'s `create_dir_all` fail; the open answers typed `Io` and
/// the path's opening count is back to zero.
#[tokio::test]
async fn an_io_fault_in_acquire_opening_leaves_no_opening_count() {
    let _serial = serial();
    let parent = tempfile::tempdir().expect("parent");
    let file = parent.path().join("a-regular-file");
    std::fs::write(&file, b"not a directory").expect("the file");
    let dir = file.join("wallet");
    let keychain = SharedKeychainVault::shared();
    match Wallet::open_resolving(
        cfg(&dir, Network::Test, SeedPersistence::SealedKeychain),
        plain(&keychain),
        None,
        None,
    )
    .await
    {
        Err(WalletError::Io(_)) => {}
        Err(e) => panic!("an ENOTDIR fault answers typed Io, got {e:?}"),
        Ok(_) => panic!("fixture: the path cannot hold a wallet"),
    }
    let ns = keychain_namespace_for(&dir);
    assert!(
        entry(&ns).is_none_or(|e| e.opening == 0),
        "the token's own Drop released it: {:?}",
        entry(&ns)
    );
}
