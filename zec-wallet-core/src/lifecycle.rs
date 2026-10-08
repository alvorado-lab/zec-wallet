//! Wallet lifecycle (spec §3.3): the handle state machine + the exclusive
//! single-writer lock. "Rejected or safe" is not a contract — REJECTED is
//! the contract: one `Wallet` per DB, in-process AND cross-process.

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::constants::WALLET_LOCK_FILE_NAME;
use crate::error::WalletError;

/// Lifecycle phase (§3.3). Public because it rides error payloads
/// (`WalletBusy`/`InvalidState`) so a host can render "what is the wallet
/// doing" instead of a bare failure.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum LifecyclePhase {
    /// create/restore in progress (two-phase provisioning, §6.3).
    Provisioning,
    /// resuming from a provisioning remnant (§6.3 repair contract).
    Repairing,
    Open,
    /// `rescan_from` is rebuilding the data DB at an earlier birthday (ADR-0534):
    /// a transient phase so a concurrent `snapshot`/`propose` gets a retryable
    /// `WalletBusy { Rescanning }`, never a torn read of a half-rebuilt store.
    Rescanning,
    /// `switch_sync_server` is swapping the session onto another server
    /// (`sync-server-picker.md` D3): the loop is stopped and joined, the
    /// choice row written, `Inner` rebuilt over the SAME database. A
    /// concurrent `snapshot`/`propose` gets the retryable
    /// `WalletBusy { SwitchingServer }`, never a torn read. Rescanning's
    /// sibling — same edges, no data-DB rebuild.
    SwitchingServer,
    /// `close()` resolves in-flight futures with this phase's error.
    Closing,
    /// Terminal: the store was crypto-shredded underneath the config.
    Wiped,
}

/// Handle state machine. Transitions are guarded — an invalid edge is a
/// typed error, never silent corruption (rust-patterns § State Machines).
///
/// Owned by `Wallet` (wallet.rs): `create`/`open` construct it in `Open`.
/// `phase`/`require_open` gate the §3.1 `&self` API methods. THREE mutators,
/// all on the one mutex (stage S16 §3.2 mechanism 4):
/// - [`Self::transition`], the guarded edge set below;
/// - [`Self::sever_poison`], the duress sever's move to `Wiped` from ANY
///   phase, `Open` included (a severed instance refuses everything after);
/// - [`Self::begin_close`], `close`'s one locked decision: `Open → Closing`,
///   or leave a poisoned `Wiped` as it is. `transition` has no `(Wiped, _)`
///   edge, so a close can never regress a poisoned handle to the retryable
///   `Closing`.
pub(crate) struct Lifecycle {
    state: Mutex<LifecyclePhase>,
}

impl Lifecycle {
    pub(crate) fn new(initial: LifecyclePhase) -> Self {
        Self {
            state: Mutex::new(initial),
        }
    }

    pub(crate) fn phase(&self) -> LifecyclePhase {
        *self.state.lock().expect("lifecycle mutex poisoned")
    }

    /// Gate for every state-requiring API call: typed `WalletBusy` for
    /// transient phases (retry when the phase completes), `InvalidState` for
    /// terminal misuse — both renderable, neither a race (§3.3). Consumed by the
    /// §3.1 `&self` methods (`current_address` now; sync/send next).
    pub(crate) fn require_open(&self) -> Result<(), WalletError> {
        let phase = self.phase();
        match phase {
            LifecyclePhase::Open => Ok(()),
            LifecyclePhase::Provisioning
            | LifecyclePhase::Repairing
            | LifecyclePhase::Rescanning
            | LifecyclePhase::SwitchingServer
            | LifecyclePhase::Closing => Err(WalletError::WalletBusy { phase }),
            LifecyclePhase::Wiped => Err(WalletError::InvalidState { phase }),
        }
    }

    /// Guarded transition; the edge set is the §3.3 state machine. An
    /// invalid edge returns `InvalidState` with the CURRENT phase and leaves
    /// the state untouched.
    pub(crate) fn transition(&self, to: LifecyclePhase) -> Result<(), WalletError> {
        let mut state = self.state.lock().expect("lifecycle mutex poisoned");
        let valid = matches!(
            (*state, to),
            (LifecyclePhase::Provisioning, LifecyclePhase::Open)
                | (LifecyclePhase::Repairing, LifecyclePhase::Open)
                | (LifecyclePhase::Open, LifecyclePhase::Closing)
                // rescan: Open → Rescanning → Open (ADR-0534). The rebuild holds
                // the single-writer lock throughout; a concurrent caller observing
                // `Rescanning` gets the retryable `WalletBusy`. The Rescanning→Open
                // edge keeps the state machine total (the happy path assembles a
                // FRESH `Open` handle, so this edge is the in-place completion form).
                | (LifecyclePhase::Open, LifecyclePhase::Rescanning)
                | (LifecyclePhase::Rescanning, LifecyclePhase::Open)
                // server switch: Open → SwitchingServer → Open (P3-13, the
                // rescan's edges — the happy path assembles a FRESH `Open`
                // handle too; the completion edge keeps the machine total).
                | (LifecyclePhase::Open, LifecyclePhase::SwitchingServer)
                | (LifecyclePhase::SwitchingServer, LifecyclePhase::Open)
                // wipe wins from any LOCK-FREE non-terminal phase (§6.3:
                // idempotent, strongest gesture). Open is deliberately NOT an
                // edge: an open instance holds the advisory lock and wipe()
                // refuses with `WalletOpen` before any transition (§3.1). The
                // duress sever reaches `Wiped` from `Open` too, but through
                // `sever_poison`, never through this edge set.
                | (LifecyclePhase::Provisioning, LifecyclePhase::Wiped)
                | (LifecyclePhase::Repairing, LifecyclePhase::Wiped)
                | (LifecyclePhase::Rescanning, LifecyclePhase::Wiped)
                | (LifecyclePhase::SwitchingServer, LifecyclePhase::Wiped)
                | (LifecyclePhase::Closing, LifecyclePhase::Wiped)
        );
        if !valid {
            return Err(WalletError::InvalidState { phase: *state });
        }
        *state = to;
        Ok(())
    }

    /// The duress sever's poison (stage S16 §3.2): move ANY phase to the
    /// terminal `Wiped`, so every later `require_open` refuses
    /// `InvalidState { Wiped }`. Idempotent.
    pub(crate) fn sever_poison(&self) {
        *self.state.lock().expect("lifecycle mutex poisoned") = LifecyclePhase::Wiped;
    }

    /// `close`'s one locked decision: `Open → Closing` (`Ok(false)`); a
    /// poisoned `Wiped` stays `Wiped` (`Ok(true)` — the close still drains and
    /// frees the lock); any other phase answers `InvalidState` as
    /// [`Self::transition`] does.
    pub(crate) fn begin_close(&self) -> Result<bool, WalletError> {
        let mut state = self.state.lock().expect("lifecycle mutex poisoned");
        match *state {
            LifecyclePhase::Open => {
                *state = LifecyclePhase::Closing;
                Ok(false)
            }
            LifecyclePhase::Wiped => Ok(true),
            phase => Err(WalletError::InvalidState { phase }),
        }
    }
}

/// An opener's mark in the process-wide path table (stage S16 §3.2 mechanism
/// 1): while it lives, a sever of this `db_dir` reads the holder as THIS
/// process. Constructing it IS the bump (one table section); its own `Drop`
/// is the release, on every path — an `acquire_opening` that fails, or the
/// lock it was moved into dropping. It takes the path table, so it must never
/// drop while that table's guard is held (§3.1 item 8).
pub(crate) struct OpeningToken {
    namespace: crate::keychain::KeychainNamespace,
}

impl OpeningToken {
    fn new(db_dir: &Path) -> Self {
        let namespace = crate::wallet::keychain_namespace_for(db_dir);
        crate::keychain::bounded::opening_bump(&namespace);
        Self { namespace }
    }
}

impl Drop for OpeningToken {
    fn drop(&mut self) {
        crate::keychain::bounded::opening_release(&self.namespace);
    }
}

/// Exclusive single-writer lock over a wallet `db_dir`.
///
/// Mechanism: an OS advisory lock (`flock(LOCK_EX|LOCK_NB)` — via
/// `rustix::fs::flock` on Unix, `std::fs::File::try_lock` on Windows; see
/// [`try_lock_exclusive`]) on a lock file inside `db_dir` — NOT an O_EXCL marker
/// file. The distinction is the §6.3 crash contract: an advisory lock is released
/// by the OS on process death, so a kill can never wedge the wallet behind a
/// stale lock; a marker file would. Held for the life of the value; `Drop`
/// releases.
///
/// In-process AND cross-process: each `acquire` does its OWN `open()`, giving a
/// fresh open file description, so a second acquire from the SAME process
/// conflicts exactly like one from another process (the unit test exercises the
/// real mechanism). Note the `flock(2)` sharing rule: a `dup`/`fork`-inherited
/// COPY of an existing lock fd SHARES the lock rather than conflicting — mutual
/// exclusion holds precisely because every opener calls `open()` itself (we
/// never `dup` or inherit the lock fd). The guarantee also assumes a LOCAL
/// filesystem: over NFS/SMB `flock` can be emulated or a local-only no-op, so a
/// network-mounted `db_dir` is unsupported (on mobile `db_dir` is always
/// app-private local storage; a future desktop "wallet on a share" is not).
///
/// **Witness token (§6.3 HARDENING):** the store's provisioning/open entry
/// points (`store::create_or_repair`, `store::open`) take `&WalletLock` and
/// derive their `db_dir` from [`WalletLock::dir`] — so a caller CANNOT reach a
/// mutation/open path without first holding the exclusive lock, and the lock's
/// own directory is the single source of truth for where it operates (no
/// lock-on-dir-A / write-to-dir-B mismatch is expressible). Held for the life
/// of the `Wallet` handle; `Drop` releases.
pub(crate) struct WalletLock {
    /// RAII guard: held only so `Drop` releases the OS advisory lock when the
    /// `Wallet` handle closes. Never read — the lock IS the file's open
    /// descriptor, not its contents.
    #[allow(dead_code)] // held for `Drop` only — see above
    file: File,
    /// The `db_dir` this lock is held over — the witnessed location. Carried so
    /// the store can derive `db_dir` from the lock itself (single source of
    /// truth), never from a separately-passed path that could disagree.
    dir: PathBuf,
    /// An opener's [`OpeningToken`] (`acquire_opening` only; `None` for a
    /// plain `acquire` — a wipe, the sever's own try, a test). Declared AFTER
    /// `file`, so the flock is released before the token is: a sever in
    /// between finds the lock free, never a held lock with no holder named.
    #[allow(dead_code)] // held for `Drop` only
    token: Option<OpeningToken>,
}

impl WalletLock {
    /// `WalletAlreadyOpen` when any other holder (this process or another)
    /// has the lock — rejected IS the contract (§3.3).
    ///
    /// The lock file rides inside `db_dir`, so the host's §4.3 obligation
    /// (backup-exclude the whole dir: iOS `isExcludedFromBackup`, Android
    /// `no_backup/`) covers it too. It holds no secret and no state — a
    /// stale copy restored onto a new device is inert (the LOCK is kernel
    /// state, never the file's existence).
    ///
    /// Acquire the cross-process single-writer lock for `db_dir`. Creates the
    /// directory and an advisory lock file, then takes an EXCLUSIVE, non-blocking
    /// lock on it (see [`try_lock_exclusive`]). Returns `WalletAlreadyOpen` if
    /// another live process holds it, or `Io` on a real fault. The lock
    /// auto-releases when this [`WalletLock`]'s file handle closes (drop or
    /// process death).
    pub(crate) fn acquire(db_dir: &Path) -> Result<Self, WalletError> {
        std::fs::create_dir_all(db_dir).map_err(WalletError::from_io)?;
        let path = db_dir.join(WALLET_LOCK_FILE_NAME);
        // truncate(false): the file's CONTENT is irrelevant (the OS lock is
        // the mechanism) — never truncate under a holder's feet.
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(WalletError::from_io)?;
        match try_lock_exclusive(&file) {
            Ok(true) => Ok(Self {
                file,
                dir: db_dir.to_path_buf(),
                token: None,
            }),
            // Another live holder owns the advisory lock → one-Wallet-per-DB.
            Ok(false) => Err(WalletError::WalletAlreadyOpen),
            Err(e) => Err(WalletError::from_io(e)),
        }
    }

    /// [`Self::acquire`] for an OPENER (create / open): the [`OpeningToken`]
    /// is built FIRST — no fallible step sits between the bump and the flock —
    /// and moved into the lock only on success, so "the lock is held by an
    /// opener in this process" and "the token exists" have no gap. On any
    /// failure (`create_dir_all`, `open`, the flock's I/O arm, or
    /// `WalletAlreadyOpen`) the token's own `Drop` releases it.
    pub(crate) fn acquire_opening(db_dir: &Path) -> Result<Self, WalletError> {
        let token = OpeningToken::new(db_dir);
        let mut lock = Self::acquire(db_dir)?;
        lock.token = Some(token);
        Ok(lock)
    }

    /// The `db_dir` this lock is held over (the witnessed location). The store
    /// derives its `db_dir` from here so holding the lock and naming the target
    /// are one and the same — see the type doc.
    pub(crate) fn dir(&self) -> &Path {
        &self.dir
    }
}

/// Take an EXCLUSIVE, non-blocking advisory lock on `file`.
/// `Ok(true)` = acquired; `Ok(false)` = another live holder (would block);
/// `Err` = a real IO fault.
///
/// UNIX uses rustix's `flock` syscall wrapper: `std::fs::File::try_lock` (Rust
/// 1.89) returns `ErrorKind::Unsupported` on ANDROID, which made every wallet
/// create/restore/open fail at lock acquisition on-device (§6.3). rustix's
/// `flock` works uniformly on macOS/iOS/Linux/Android. WINDOWS keeps std
/// `try_lock` (a §9 v2 target; rustix has no Windows `flock`). Both are the SAME
/// advisory `flock(LOCK_EX|LOCK_NB)` semantics — released when the file's last
/// descriptor closes (drop / process death), so a crashed holder never wedges
/// the next open.
#[cfg(unix)]
fn try_lock_exclusive(file: &File) -> Result<bool, std::io::Error> {
    use rustix::fs::{FlockOperation, flock};
    match flock(file, FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => Ok(true),
        // EWOULDBLOCK (== EAGAIN on Linux) is the "another holder" signal.
        Err(e) if e == rustix::io::Errno::WOULDBLOCK || e == rustix::io::Errno::AGAIN => Ok(false),
        Err(e) => Err(std::io::Error::from(e)),
    }
}

#[cfg(not(unix))]
fn try_lock_exclusive(file: &File) -> Result<bool, std::io::Error> {
    use std::fs::TryLockError;
    match file.try_lock() {
        Ok(()) => Ok(true),
        Err(TryLockError::WouldBlock) => Ok(false),
        Err(TryLockError::Error(e)) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_opener_gets_wallet_already_open() {
        // §8 named test. Two acquires on the same dir conflict; the lock is
        // per open-file-description, so this exercises the SAME mechanism
        // that rejects a second process.
        let dir = tempfile::tempdir().expect("tempdir");
        let first = WalletLock::acquire(dir.path()).expect("first opener acquires");
        match WalletLock::acquire(dir.path()) {
            Err(WalletError::WalletAlreadyOpen) => {}
            Ok(_) => panic!("second opener must be REJECTED while the first holds the lock"),
            Err(other) => panic!("expected WalletAlreadyOpen, got {}", other.code()),
        }
        // crash-release semantics (approximated by drop): a dead holder
        // never wedges the next opener
        drop(first);
        WalletLock::acquire(dir.path()).expect("lock is free after the holder dies");
    }

    #[test]
    fn api_calls_in_wrong_lifecycle_state_get_typed_errors() {
        // §8 named test: the full phase table, each wrong-state call typed.
        // transient phases → WalletBusy (retryable)
        for phase in [
            LifecyclePhase::Provisioning,
            LifecyclePhase::Repairing,
            LifecyclePhase::Rescanning,
            LifecyclePhase::Closing,
        ] {
            let err = Lifecycle::new(phase)
                .require_open()
                .expect_err("busy phase must reject");
            assert!(
                matches!(err, WalletError::WalletBusy { .. }),
                "phase {phase:?} produced wrong error {}",
                err.code()
            );
        }
        // terminal phase → InvalidState
        let err = Lifecycle::new(LifecyclePhase::Wiped)
            .require_open()
            .expect_err("wiped wallet must reject");
        assert!(matches!(
            err,
            WalletError::InvalidState {
                phase: LifecyclePhase::Wiped
            }
        ));
        assert!(Lifecycle::new(LifecyclePhase::Open).require_open().is_ok());
    }

    #[test]
    fn lifecycle_transitions_are_guarded() {
        // valid: provisioning completes to open, open closes
        let lc = Lifecycle::new(LifecyclePhase::Provisioning);
        lc.transition(LifecyclePhase::Open)
            .expect("provisioning -> open is the happy path");
        lc.transition(LifecyclePhase::Closing)
            .expect("open -> closing");
        // invalid: closing cannot reopen — typed, state untouched
        let err = lc
            .transition(LifecyclePhase::Open)
            .expect_err("closing -> open is invalid");
        assert!(matches!(
            err,
            WalletError::InvalidState {
                phase: LifecyclePhase::Closing
            }
        ));
        // wipe wins from a non-terminal phase
        lc.transition(LifecyclePhase::Wiped)
            .expect("closing -> wiped");
        // terminal is terminal
        assert!(lc.transition(LifecyclePhase::Open).is_err());
    }

    #[test]
    fn a_concurrent_snapshot_during_a_switch_is_wallet_busy_switching_server() {
        // P3-13 (`sync-server-picker.md` §6): the switch's transient phase is
        // the rescan's sibling — a concurrent state-requiring call gets the
        // retryable `WalletBusy { SwitchingServer }`, the completion edge
        // returns to Open, a panic-wipe may race it, and nothing else may
        // enter or leave it.
        let lc = Lifecycle::new(LifecyclePhase::Open);
        lc.transition(LifecyclePhase::SwitchingServer)
            .expect("open -> switching is the switch entry");
        assert!(matches!(
            lc.require_open(),
            Err(WalletError::WalletBusy {
                phase: LifecyclePhase::SwitchingServer
            })
        ));
        assert!(matches!(
            lc.transition(LifecyclePhase::Rescanning),
            Err(WalletError::InvalidState {
                phase: LifecyclePhase::SwitchingServer
            })
        ));
        lc.transition(LifecyclePhase::Open)
            .expect("switching -> open is the completion edge");
        assert!(lc.require_open().is_ok());
        assert!(matches!(
            lc.transition(LifecyclePhase::SwitchingServer),
            Ok(())
        ));
        lc.transition(LifecyclePhase::Wiped)
            .expect("a panic-wipe may race a switch");
        assert!(matches!(
            Lifecycle::new(LifecyclePhase::Provisioning)
                .transition(LifecyclePhase::SwitchingServer),
            Err(WalletError::InvalidState { .. })
        ));
    }

    #[test]
    fn rescan_phase_edges_are_guarded() {
        // ADR-0534: Open → Rescanning → Open is the rescan cycle; Rescanning → Wiped
        // (a panic-wipe racing a rescan) is allowed; every other edge into/out of
        // Rescanning is a typed reject, state untouched.
        let lc = Lifecycle::new(LifecyclePhase::Open);
        lc.transition(LifecyclePhase::Rescanning)
            .expect("open -> rescanning is the rescan entry");
        // a concurrent require_open during the rebuild is retryable WalletBusy
        assert!(matches!(
            lc.require_open(),
            Err(WalletError::WalletBusy {
                phase: LifecyclePhase::Rescanning
            })
        ));
        lc.transition(LifecyclePhase::Open)
            .expect("rescanning -> open completes the cycle");

        // Rescanning -> Wiped (panic-wipe wins) is valid
        let lc2 = Lifecycle::new(LifecyclePhase::Open);
        lc2.transition(LifecyclePhase::Rescanning)
            .expect("-> rescanning");
        lc2.transition(LifecyclePhase::Wiped)
            .expect("rescanning -> wiped");
        // terminal is terminal
        assert!(lc2.transition(LifecyclePhase::Open).is_err());

        // an invalid entry (Closing -> Rescanning) is typed, state untouched
        let lc3 = Lifecycle::new(LifecyclePhase::Open);
        lc3.transition(LifecyclePhase::Closing).expect("-> closing");
        assert!(matches!(
            lc3.transition(LifecyclePhase::Rescanning),
            Err(WalletError::InvalidState {
                phase: LifecyclePhase::Closing
            })
        ));
    }
}
