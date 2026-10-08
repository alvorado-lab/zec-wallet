//! The lifecycle (`tor-plugin.md` §3.1 and §3.4): `init`, `set_bridges`,
//! `retry_bootstrap`, `clear_state`, `dispose`, the pause and resume, the
//! rebuild, and the two loops each engine generation runs — the bootstrap loop
//! and the readiness watcher.
//!
//! Every transition happens under the trampoline's one lock, and every push to
//! the wallet is made inside the same critical section as the transition it
//! announces, so a dial is either accepted before a flip, on its generation, or
//! refused after it (P6, P20). Nothing here blocks on the plugin's runtime from
//! inside it: `init` and `dispose` wait on std primitives from the host's
//! thread while the runtime's own threads do the work (never `block_on` —
//! `init` may be called from a thread of another tokio runtime).

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dialer_tor::{BridgeLines, DormantMode, Readiness, TorDialError, tor_config};
use tokio::sync::mpsc;
use zeroize::Zeroizing;

use crate::abi::{
    ZW_RC_ABI, ZW_RC_NULL_ARG, ZW_RC_OCCUPIED, ZW_RC_PANICKED, ZW_RC_POISONED, ZWT_RC_ABI_MISMATCH,
    ZWT_RC_BRIDGES_REFUSED, ZWT_RC_DESCRIPTOR_REFUSED, ZWT_RC_DISPOSED, ZWT_RC_ENGINE_SETUP,
    ZWT_RC_INVALID_DATA_DIR, ZWT_RC_NOT_INITIALIZED, ZWT_RC_NULL_ARG, ZWT_RC_OK, ZWT_RC_PANICKED,
    ZWT_RC_REGISTRY_POISONED, ZWT_RC_RESTART_REQUIRED, ZWT_RC_SLOT_OCCUPIED, ZWT_RC_STOPPING,
    ZWT_RC_WALLET_NOT_LOADED, vtable_for,
};
use crate::constants::{
    BOOTSTRAP_DEADLINE, CACHE_SUBDIR, CLASS_BOOTSTRAP_DEADLINE, CLASS_NOT_REGISTERED,
    CLEAR_PENDING_MARKER, QUIESCE_POLL, READINESS_POLL, READINESS_POLL_SLOW, READINESS_SUSPENDED,
    REBUILD_AFTER_PAUSE, REBUILD_WAIT_MAX, RETIRE_QUIESCE_MAX, SLOW_AFTER, STATE_SUBDIR,
    TOR_DIR_MARKER,
};
use crate::engine::{EngineConfig, TorEngine, Tracked};
use crate::status::{Blockage, Phase};
use crate::trampoline::{Plugin, PluginState, Registration, StatusSink};
use crate::watcher::{Health, Pushed, live_pair, readiness_percent};

/// The class `dialer-tor` gives a config arti refuses — met at `init`, before
/// registration (the security angle's MEDIUM on the spec).
fn setup_class() -> &'static str {
    TorDialError::Setup {
        kind: dialer_tor::ErrorKind::InvalidConfig,
    }
    .class()
}

/// The pair a paused or just-registered plugin pushes: starting, readiness 0.
const STARTING_AT_ZERO: Pushed = Pushed {
    health: Health::Starting,
    readiness: READINESS_SUSPENDED,
};

/// `tor_dir` as the plugin accepts it: non-empty, absolute, and with no `..`
/// component (a path that climbs out of what the host named is refused, not
/// normalised). NEVER logged.
pub fn validated_tor_dir(tor_dir: &str) -> Option<PathBuf> {
    let path = Path::new(tor_dir);
    let ok = !tor_dir.is_empty()
        && path.is_absolute()
        && !path.components().any(|c| matches!(c, Component::ParentDir));
    ok.then(|| path.to_path_buf())
}

/// Create `<tor_dir>`, its two subtrees (0700 on unix) and the marker.
fn prepare_tor_dir(tor_dir: &Path) -> std::io::Result<(PathBuf, PathBuf)> {
    let state = tor_dir.join(STATE_SUBDIR);
    let cache = tor_dir.join(CACHE_SUBDIR);
    for dir in [&state, &cache] {
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
            builder.mode(0o700);
            builder.create(dir)?;
            std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
        }
        #[cfg(not(unix))]
        builder.create(dir)?;
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(tor_dir.join(TOR_DIR_MARKER))?;
    Ok((state, cache))
}

/// `clear_state` (plan D-12, D-16; the security angle's A2): remove ONLY
/// `<tor_dir>/state` and `<tor_dir>/cache`, then the marker, then `tor_dir`
/// if it is left empty. Refused — nothing removed — for a path that is not
/// absolute or climbs with `..`, that is not a directory (a symlink included),
/// or that lacks the marker the plugin writes at `init`: a host that passes
/// the wrong directory (the wallet's `db_dir`, an app's data root whose
/// `cache` belongs to the OS) loses nothing. A subtree that is a SYMLINK is
/// unlinked, never followed. An absent `tor_dir` is success (idempotent).
pub fn remove_tor_state(tor_dir: &str) -> i32 {
    let Some(dir) = validated_tor_dir(tor_dir) else {
        return ZWT_RC_INVALID_DATA_DIR;
    };
    match std::fs::symlink_metadata(&dir) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return ZWT_RC_OK,
        Err(_) => return ZWT_RC_INVALID_DATA_DIR,
        Ok(meta) if !meta.is_dir() => return ZWT_RC_INVALID_DATA_DIR,
        Ok(_) => {}
    }
    let marker = dir.join(TOR_DIR_MARKER);
    if !std::fs::symlink_metadata(&marker).is_ok_and(|m| m.is_file()) {
        return ZWT_RC_INVALID_DATA_DIR;
    }
    for sub in [STATE_SUBDIR, CACHE_SUBDIR] {
        let path = dir.join(sub);
        let removed = match std::fs::symlink_metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
            Ok(meta) if meta.is_dir() => std::fs::remove_dir_all(&path),
            // A symlink or a file: the entry itself, never its target.
            Ok(_) => std::fs::remove_file(&path),
        };
        if removed.is_err() {
            tracing::warn!(
                rc = ZWT_RC_INVALID_DATA_DIR,
                "clear_state could not remove a tor subtree"
            );
            return ZWT_RC_INVALID_DATA_DIR;
        }
    }
    // A pending clear is done now (absent is fine).
    match std::fs::remove_file(dir.join(CLEAR_PENDING_MARKER)) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return ZWT_RC_INVALID_DATA_DIR,
    }
    if std::fs::remove_file(&marker).is_err() {
        return ZWT_RC_INVALID_DATA_DIR;
    }
    // Only if empty: a host's own files beside ours stay.
    let _ = std::fs::remove_dir(&dir);
    ZWT_RC_OK
}

/// Record that a `clear_state` could not run now (a client is stopping or
/// stuck), so the next `init` completes it ([`CLEAR_PENDING_MARKER`]).
///
/// `Ok` only once the record is on disk (the file and the directory entry
/// synced), or when `tor_dir` does not exist (nothing to reset, as
/// [`remove_tor_state`] answers). Otherwise `ZWT_RC_INVALID_DATA_DIR`, and
/// `clear_state` answers that instead of "call again later": the host is
/// never told a reset will finish that nothing recorded (the external review
/// of `962a91de7`). The preconditions are the removal's own — a real
/// directory (never a symlink) holding the plugin's marker — because a record
/// where the removal refuses would fail every later `init`. Never logs the
/// path.
pub(crate) fn mark_clear_pending(tor_dir: &str) -> Result<(), i32> {
    let Some(dir) = validated_tor_dir(tor_dir) else {
        return Err(ZWT_RC_INVALID_DATA_DIR);
    };
    match std::fs::symlink_metadata(&dir) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Ok(meta) if meta.is_dir() => {}
        _ => return Err(ZWT_RC_INVALID_DATA_DIR),
    }
    if !std::fs::symlink_metadata(dir.join(TOR_DIR_MARKER)).is_ok_and(|m| m.is_file()) {
        return Err(ZWT_RC_INVALID_DATA_DIR);
    }
    let pending = dir.join(CLEAR_PENDING_MARKER);
    // Already saved by an earlier refusal (a failed save removes its file):
    // the Dart side retries every 100 ms under this lock, and a sync each
    // time would hold every other verb behind the disk (the security review
    // of this fix).
    if std::fs::symlink_metadata(&pending).is_ok_and(|m| m.is_file()) {
        return Ok(());
    }
    let saved = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&pending)
        .and_then(|record| record.sync_all())
        .and_then(|()| sync_dir(&dir));
    if saved.is_err() {
        tracing::warn!("could not record a pending Tor state clear");
        // "Nothing pending" must hold: a record created but not synced could
        // still be found by the next `init` (best effort; a file that stays
        // finishes a reset the host did ask for).
        let _ = std::fs::remove_file(&pending);
        return Err(ZWT_RC_INVALID_DATA_DIR);
    }
    Ok(())
}

/// Make a new entry in `dir` durable (a synced file can still vanish with its
/// unsynced directory entry). Windows has no directory handle to sync; its
/// file sync is the bound there.
fn sync_dir(dir: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::fs::File::open(dir)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = dir;
        Ok(())
    }
}

/// The plugin's code for the wallet's refusal of `register`.
fn register_refusal(rc: i32) -> i32 {
    match rc {
        ZW_RC_OCCUPIED => ZWT_RC_SLOT_OCCUPIED,
        ZW_RC_ABI => ZWT_RC_ABI_MISMATCH,
        ZW_RC_POISONED => ZWT_RC_REGISTRY_POISONED,
        ZW_RC_NULL_ARG => ZWT_RC_NULL_ARG,
        ZW_RC_PANICKED => ZWT_RC_PANICKED,
        // ZW_RC_DESCRIPTOR, and any code a newer wallet invents: a build
        // defect either way (P3 pins that the descriptor is accepted).
        _ => ZWT_RC_DESCRIPTOR_REFUSED,
    }
}

/// Why `clear_state` or `init` must not touch the Tor directory now, if it
/// must not: a client stuck past its shutdown bound (only a restart ends it),
/// or one still stopping (call again shortly). The ONE rule both verbs use.
fn writers_refusal(s: &PluginState) -> Option<i32> {
    if s.clients_stuck() {
        Some(ZWT_RC_RESTART_REQUIRED)
    } else if !s.state_writers_quiet() {
        Some(ZWT_RC_STOPPING)
    } else {
        None
    }
}

/// Whether the plugin holds a registration (an engine may be starting).
fn registered(s: &PluginState) -> bool {
    matches!(
        s.phase,
        Phase::Bootstrapping | Phase::Ready | Phase::Failed | Phase::Suspended
    )
}

impl Plugin {
    /// `init` (§3.1): check the directory, classify the bridges and BUILD the
    /// arti config, resolve the wallet, REGISTER at readiness 0, then mint the
    /// engine and start its loops. Idempotent while registered.
    pub fn init(
        self: &Arc<Self>,
        tor_dir: &str,
        bridges: Option<Zeroizing<String>>,
        sink: Option<StatusSink>,
    ) -> i32 {
        {
            let s = self.lock();
            if registered(&s) {
                return ZWT_RC_OK;
            }
            // A client from before a dispose may still be stopping: a new one
            // over the same `state/` would race its last write. Refused BEFORE
            // `prepare_tor_dir` writes anything (external review 2026-10-07, 1);
            // checked again under the lock that registers, below.
            if let Some(rc) = writers_refusal(&s) {
                return rc;
            }
            // A clear the host asked for while a client was stopping or stuck
            // (refused, and recorded by `mark_clear_pending`): nothing can
            // write now, so finish it BEFORE anything is created (under the
            // lock, as `clear_state` removes).
            if let Some(d) = validated_tor_dir(tor_dir)
                .filter(|d| std::fs::symlink_metadata(d.join(CLEAR_PENDING_MARKER)).is_ok())
            {
                // The record without the plugin's own marker: written only
                // beside that marker and removed before it, so something
                // outside the plugin removed it. `remove_tor_state` refuses
                // such a directory, and failing closed would refuse every
                // `init` for good; drop the record instead (the code review
                // of the 2026-10-07 folds).
                if !std::fs::symlink_metadata(d.join(TOR_DIR_MARKER)).is_ok_and(|m| m.is_file()) {
                    tracing::warn!("a pending Tor state clear lost its directory marker");
                    if std::fs::remove_file(d.join(CLEAR_PENDING_MARKER)).is_err() {
                        return ZWT_RC_INVALID_DATA_DIR;
                    }
                } else {
                    let rc = remove_tor_state(tor_dir);
                    if rc != ZWT_RC_OK {
                        // Fail closed: Tor does not start over an identity the
                        // host asked to reset. Every init refuses until the
                        // state can be removed (a permissions fault needs the
                        // user or host).
                        tracing::warn!(rc, "a pending Tor state clear could not finish");
                        return rc;
                    }
                }
            }
        }
        let Some(dir) = validated_tor_dir(tor_dir) else {
            return ZWT_RC_INVALID_DATA_DIR;
        };
        let lines = match BridgeLines::parse(bridges.as_ref().map_or("", |b| b.as_str())) {
            Ok(lines) => lines,
            Err(class) => {
                return self.refuse_before_registration(Phase::Idle, class, ZWT_RC_BRIDGES_REFUSED);
            }
        };
        drop(bridges);
        let had_bridges = !lines.is_empty();
        let Ok((state_dir, cache_dir)) = prepare_tor_dir(&dir) else {
            return ZWT_RC_INVALID_DATA_DIR;
        };
        let client = match tor_config(&state_dir, &cache_dir, lines) {
            Ok(client) => client,
            Err(_) => {
                let rc = if had_bridges {
                    ZWT_RC_BRIDGES_REFUSED
                } else {
                    ZWT_RC_ENGINE_SETUP
                };
                return self.refuse_before_registration(Phase::Idle, setup_class(), rc);
            }
        };
        let Some(link) = self.deps.resolver.resolve() else {
            return self.refuse_before_registration(
                Phase::NotRegistered,
                CLASS_NOT_REGISTERED,
                ZWT_RC_WALLET_NOT_LOADED,
            );
        };
        let cfg = Arc::new(EngineConfig {
            state_dir,
            cache_dir,
            client,
        });

        let generation = {
            let mut s = self.lock();
            if registered(&s) {
                return ZWT_RC_OK;
            }
            if let Some(rc) = writers_refusal(&s) {
                return rc;
            }
            let token = match link.register(
                &vtable_for(self),
                &crate::trampoline::descriptor(STARTING_AT_ZERO),
            ) {
                Ok(token) => token,
                Err(rc) => {
                    s.phase = Phase::NotRegistered;
                    s.failure_class = Some(CLASS_NOT_REGISTERED);
                    self.emit(&mut s);
                    tracing::warn!(
                        rc,
                        class = CLASS_NOT_REGISTERED,
                        "the wallet refused the registration"
                    );
                    return register_refusal(rc);
                }
            };
            // The host's callback is installed only once the plugin holds a
            // registration, which `dispose` then closes and flushes: a refused
            // `init` leaves no live `ctx` behind (the security angle's
            // L4; the refusal's status is read with `status`).
            if let Some(sink) = sink {
                self.open_outbox(&mut s, sink);
            }
            s.registration = Some(Registration { link, token });
            s.debounce = Default::default();
            s.debounce.record(STARTING_AT_ZERO);
            s.phase = Phase::Bootstrapping;
            s.failure_class = None;
            s.blockage = None;
            s.tor_dir = Some(dir);
            s.engine_cfg = Some(cfg.clone());
            self.emit(&mut s);
            tracing::info!(phase = "bootstrapping", "registered with the wallet");
            // The mint below writes over `state/`: counted like a rebuild's, so a
            // dispose landing during it waits (bounded), and `clear_state` and a
            // later `init` refuse until it ends (external review 2026-10-07, 1).
            s.rebuilds_in_flight += 1;
            s.generation
        };
        let in_flight = InFlight(self.clone());

        // Mint on the plugin's runtime; wait here on a std channel. The client
        // is counted live from the moment its work can exist (the factory
        // counts its own runtime; [`Tracked`] covers a client without one).
        let (tx, rx) = std::sync::mpsc::channel();
        let factory = self.deps.factory.clone();
        let live = self.lock().live_clients.clone();
        self.runtime.spawn(async move {
            let minted = factory.mint(&cfg, &live).await;
            let _ = tx.send(minted.map(|engine| Tracked::wrap(engine, &live)));
        });
        let rc = match rx.recv() {
            Ok(Ok(engine)) => match self.install(generation, engine) {
                Ok(()) => ZWT_RC_OK,
                Err(superseded) => {
                    self.drop_on_runtime(superseded);
                    ZWT_RC_DISPOSED
                }
            },
            _ => {
                let mut s = self.lock();
                if s.generation == generation {
                    self.terminal_setup_failure(&mut s);
                }
                ZWT_RC_ENGINE_SETUP
            }
        };
        // A superseded client is counted in `live_clients` until its drop ends.
        drop(in_flight);
        rc
    }

    fn refuse_before_registration(&self, phase: Phase, class: &'static str, rc: i32) -> i32 {
        let mut s = self.lock();
        s.phase = phase;
        s.failure_class = Some(class);
        self.emit(&mut s);
        tracing::warn!(rc, class, "init refused before registration");
        rc
    }

    /// A new outbox to the host's callback, drained by one task on the
    /// plugin's runtime (never under the lock). The previous one, if any, is
    /// closed: its task delivers what it holds and ends.
    fn open_outbox(&self, s: &mut PluginState, sink: StatusSink) {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
        self.runtime.spawn(async move {
            while let Some(status) = rx.recv().await {
                sink(&status);
            }
            drop(done_tx);
        });
        s.outbox = Some(tx);
        s.outbox_done = Some(done_rx);
        s.last_status = None;
    }

    /// Install a minted engine if its generation is still current, and start
    /// the two loops. `Err` hands a superseded engine back for the CALLER to
    /// drop — the rebuild task drops it inside its own in-flight window, so a
    /// `dispose` waiting on that window also waits for this drop's state write.
    pub(crate) fn install(
        self: &Arc<Self>,
        generation: u64,
        engine: Arc<dyn TorEngine>,
    ) -> Result<(), Arc<dyn TorEngine>> {
        let mut s = self.lock();
        if s.generation != generation || s.registration.is_none() {
            return Err(engine);
        }
        s.engine = Some(engine.clone());
        s.liveness_failures = 0;
        // Minted while the app is paused (a rebuild the host asked for from
        // the background): the new client starts dormant too.
        if s.phase == Phase::Suspended {
            engine.set_dormant(DormantMode::Soft);
        }
        let plugin = self.clone();
        let boot_engine = engine.clone();
        self.runtime
            .spawn(async move { plugin.bootstrap_loop(generation, boot_engine).await });
        let plugin = self.clone();
        self.runtime
            .spawn(async move { plugin.watch(generation, engine).await });
        Ok(())
    }

    /// Drop a superseded engine off the caller's thread (`init` runs on the
    /// host's). The client then shuts its own runtime down on a dedicated
    /// thread and stays counted in `live_clients` until that shutdown has
    /// finished (plan §5).
    fn drop_on_runtime(&self, engine: Arc<dyn TorEngine>) {
        self.runtime.spawn(async move { drop(engine) });
    }

    /// A mint failed after registration (§3.4 "A terminal engine failure"):
    /// retire, CLEAR the slot, `NotRegistered { setup }` — the wallet's door
    /// then refuses honestly instead of reading 0% for the life of the process.
    fn terminal_setup_failure(&self, s: &mut PluginState) {
        if let Some(reg) = s.registration.take() {
            reg.link.notify(
                &reg.token,
                &crate::trampoline::descriptor(STARTING_AT_ZERO),
                true,
            );
            reg.link.clear(&reg.token);
        }
        s.engine = None;
        s.phase = Phase::NotRegistered;
        s.failure_class = Some(setup_class());
        self.emit(s);
        // With no registration, `dispose` has nothing to close: close the
        // host's callback here, after this last status (the fold
        // review's L4 — the refused-init claim covers this path too).
        s.outbox = None;
        tracing::warn!(
            class = setup_class(),
            "the tor engine could not be built; the slot is cleared"
        );
    }

    /// Resolves when the generation moves past `generation`.
    async fn superseded(&self, generation: u64) {
        let mut rx = self.generation_tx.subscribe();
        let _ = rx.wait_for(|g| *g != generation).await;
    }

    /// Wait until the plugin is not suspended. `false` = the generation moved.
    async fn until_not_suspended(&self, generation: u64) -> bool {
        loop {
            let resumed = self.resumed.notified();
            tokio::pin!(resumed);
            resumed.as_mut().enable();
            {
                let s = self.lock();
                if s.generation != generation {
                    return false;
                }
                if s.phase != Phase::Suspended {
                    return true;
                }
            }
            tokio::select! {
                () = resumed => {}
                () = self.superseded(generation) => return false,
            }
        }
    }

    /// The bootstrap loop (§3.4): attempt under `BOOTSTRAP_DEADLINE`; on a
    /// failure `Failed` with its class, FAILED pushed with the readiness
    /// measured, then the backoff (cut by `retry_bootstrap`); no attempt while
    /// suspended; ends at the first success or with its generation.
    async fn bootstrap_loop(self: Arc<Self>, generation: u64, engine: Arc<dyn TorEngine>) {
        loop {
            if !self.until_not_suspended(generation).await {
                return;
            }
            {
                // A new attempt is a bootstrap again: out of `Failed` (after
                // the backoff, a retry, or a resume that restored it).
                let mut s = self.lock();
                if s.generation != generation {
                    return;
                }
                if s.phase == Phase::Failed {
                    s.phase = Phase::Bootstrapping;
                    let pair = live_pair(&engine.readiness());
                    self.push(&mut s, pair, false);
                }
            }
            let attempt = tokio::time::timeout(BOOTSTRAP_DEADLINE, engine.bootstrap());
            let outcome = tokio::select! {
                r = attempt => r,
                () = self.superseded(generation) => return,
            };
            let delay = {
                let mut s = self.lock();
                if s.generation != generation {
                    return;
                }
                let (class, blockage) = match outcome {
                    Ok(Ok(())) => {
                        s.backoff.on_success();
                        s.failure_class = None;
                        s.blockage = None;
                        if matches!(s.phase, Phase::Bootstrapping | Phase::Ready) {
                            self.observe(&mut s, &engine);
                        }
                        tracing::info!(phase = "bootstrapped", "tor bootstrap complete");
                        return;
                    }
                    Ok(Err(error)) => {
                        let blockage = match &error {
                            TorDialError::BootstrapFailed { blockage, .. } => {
                                blockage.as_ref().map(Blockage::from_arti)
                            }
                            _ => None,
                        };
                        (error.class(), blockage)
                    }
                    Err(_elapsed) => (CLASS_BOOTSTRAP_DEADLINE, blockage_of(&engine.readiness())),
                };
                s.failure_class = Some(class);
                s.blockage = blockage;
                let measured = readiness_percent(&engine.readiness())
                    .min(crate::constants::READINESS_CEILING_WHILE_BOOTSTRAPPING);
                if s.phase == Phase::Suspended {
                    s.phase_before_pause = Phase::Failed;
                    self.emit(&mut s);
                } else {
                    s.phase = Phase::Failed;
                    self.push(
                        &mut s,
                        Pushed {
                            health: Health::Failed,
                            readiness: measured,
                        },
                        false,
                    );
                }
                tracing::warn!(class, phase = "failed", "tor bootstrap attempt failed");
                let delay = self.deps.jitter.stretch(s.backoff.delay());
                s.backoff.on_failure();
                delay
            };
            tokio::select! {
                () = tokio::time::sleep(delay) => {}
                () = self.retry.notified() => {}
                () = self.superseded(generation) => return,
            }
        }
    }

    /// The readiness watcher (§3.4): one per generation; every
    /// `READINESS_POLL` until `SLOW_AFTER`, then every `READINESS_POLL_SLOW`,
    /// jittered up; the push rule is `watcher::Debounce`'s.
    async fn watch(self: Arc<Self>, generation: u64, engine: Arc<dyn TorEngine>) {
        let started = tokio::time::Instant::now();
        loop {
            let every = if started.elapsed() < SLOW_AFTER {
                READINESS_POLL
            } else {
                READINESS_POLL_SLOW
            };
            tokio::select! {
                () = tokio::time::sleep(self.deps.jitter.stretch(every)) => {}
                () = self.superseded(generation) => return,
            }
            let mut s = self.lock();
            if s.generation != generation {
                return;
            }
            if matches!(s.phase, Phase::Bootstrapping | Phase::Ready) {
                self.observe(&mut s, &engine);
            }
        }
    }

    /// One watcher read: the blockage, and a push if the rule says so. The
    /// phase follows what was PUSHED (Ready at a pushed 100).
    fn observe(&self, s: &mut PluginState, engine: &Arc<dyn TorEngine>) {
        let readiness = engine.readiness();
        s.blockage = blockage_of(&readiness);
        if let Some(pushed) = s.debounce.observe(live_pair(&readiness)) {
            self.send(s, pushed, false);
            s.phase = if pushed.health == Health::Ready {
                Phase::Ready
            } else {
                Phase::Bootstrapping
            };
        }
        self.emit(s);
    }

    /// After the liveness rule tripped: wait `delay` (one jittered base retry
    /// interval, cut by a retry), never while suspended, then rebuild.
    pub(crate) async fn recover(self: Arc<Self>, generation: u64, delay: Duration) {
        tokio::select! {
            () = tokio::time::sleep(delay) => {}
            () = self.retry.notified() => {}
            () = self.superseded(generation) => return,
        }
        if !self.until_not_suspended(generation).await {
            return;
        }
        let still_failed = {
            let s = self.lock();
            s.generation == generation && s.phase == Phase::Failed
        };
        if still_failed {
            self.rebuild();
        }
    }

    /// REBUILD (§3.4): under the lock, bump the generation, close every op,
    /// `Bootstrapping`, and `notify(retire = 1)` (decided a rebuild is
    /// exactly the header's meaning of `retire`); then let the old engine
    /// quiesce and drop BEFORE the new mint — arti opens a state directory
    /// whose lock another client holds read-only (`client.rs:2356-2366`).
    pub(crate) fn rebuild(self: &Arc<Self>) {
        self.rebuild_leaving(false);
    }

    /// [`Self::rebuild`]; `leaving_suspended` is the long-pause resume, which
    /// leaves `Suspended` in the SAME critical section as the generation bump
    /// (a phase flip released before it let the old generation's watcher push
    /// a stale READY for an instant — the fold review's L7).
    fn rebuild_leaving(self: &Arc<Self>, leaving_suspended: bool) {
        let (generation, cfg, old) = {
            let mut s = self.lock();
            if s.registration.is_none() {
                return;
            }
            let Some(cfg) = s.engine_cfg.clone() else {
                return;
            };
            // A client stuck past its shutdown bound means arti hangs on this
            // device; another rebuild would strand one more runtime and its
            // threads for the life of the process (security review H3). Keep
            // the current client; a restart is the way out, as `clear_state`
            // already says. The callers that must not silently lose a rebuild
            // check first: `set_bridges` refuses, `on_resumed` wakes instead;
            // here it is the backstop (a liveness recovery stays `Failed`,
            // which is what it reports).
            if s.clients_stuck() {
                tracing::warn!("rebuild skipped: an earlier Tor client is stuck");
                // A rebuild that was to END a pause must still end it: a client
                // stuck after `on_resumed` checked would otherwise leave the
                // plugin `Suspended` with nothing coming (the confirmation
                // review). Wake the current client instead.
                if leaving_suspended && s.phase == Phase::Suspended {
                    if let Some(engine) = &s.engine {
                        engine.set_dormant(DormantMode::Normal);
                    }
                    s.phase = if s.phase_before_pause == Phase::Failed {
                        Phase::Failed
                    } else {
                        Phase::Bootstrapping
                    };
                    self.emit(&mut s);
                    drop(s);
                    self.resumed.notify_waiters();
                }
                return;
            }
            s.generation += 1;
            self.generation_tx.send_replace(s.generation);
            Plugin::close_all(&mut s);
            let old = s.engine.take();
            // A rebuild asked for while the app is paused (`set_bridges` from
            // the background, a recovery racing a pause) stays paused: the new
            // client is minted dormant (`install`) and bootstraps on resume —
            // no Tor traffic from a backgrounded wallet (the arch and
            // crypto angles).
            if s.phase == Phase::Suspended && !leaving_suspended {
                s.phase_before_pause = Phase::Bootstrapping;
            } else {
                s.phase = Phase::Bootstrapping;
            }
            s.rebuilds_in_flight += 1;
            s.failure_class = None;
            s.blockage = None;
            s.liveness_failures = 0;
            self.push(&mut s, STARTING_AT_ZERO, true);
            tracing::info!(
                rebuilt = true,
                generation = s.generation,
                "tor client rebuilding"
            );
            (s.generation, cfg, old)
        };
        self.resumed.notify_waiters();
        let guard = InFlight(self.clone());
        self.runtime.spawn(async move {
            let plugin = guard.0.clone();
            if let Some(old) = old {
                let until = tokio::time::Instant::now() + RETIRE_QUIESCE_MAX;
                while Arc::strong_count(&old) > 1 && tokio::time::Instant::now() < until {
                    tokio::time::sleep(QUIESCE_POLL).await;
                }
                // Start its shutdown even if a late holder remains.
                old.retire();
                drop(old);
            }
            // EVERY rebuild — one that found no engine too (an earlier
            // rebuild's retired client, or its half-built one, may still hold
            // `state/`) — waits until NO client is counted and no other mint
            // is in flight before it mints:
            // two clients never share `state/` (`tor-plugin.md` §12; the
            // overlap the audit plan's §6 deferred). It never mints while one
            // is still counted: a stuck client, or one past
            // `REBUILD_WAIT_MAX`, takes the terminal setup path instead (the
            // reviews of the built diff).
            let until = tokio::time::Instant::now() + REBUILD_WAIT_MAX;
            let quiet = loop {
                let (live, others_minting, stuck, superseded) = {
                    let s = plugin.lock();
                    (
                        s.live_clients.live(),
                        // This rebuild counts itself; any other is a mint
                        // (an `init`'s or an older rebuild's) that may not
                        // have counted its client yet. Waiting it out makes
                        // the check and the mint one step: a later rebuild
                        // waits on THIS one's count, which lasts until its
                        // task ends (the code review of the folds).
                        s.rebuilds_in_flight > 1,
                        s.live_clients.stuck(),
                        s.generation != generation,
                    )
                };
                if superseded {
                    // A dispose or another rebuild took over: mint nothing.
                    return;
                }
                if live == 0 && !others_minting {
                    break true;
                }
                if stuck > 0 || tokio::time::Instant::now() >= until {
                    break false;
                }
                tokio::time::sleep(QUIESCE_POLL).await;
            };
            if !quiet {
                let mut s = plugin.lock();
                if s.generation == generation {
                    tracing::warn!("rebuild: an earlier Tor client is still running; not minting");
                    plugin.terminal_setup_failure(&mut s);
                }
                return;
            }
            let live = plugin.lock().live_clients.clone();
            match plugin.deps.factory.mint(&cfg, &live).await {
                Ok(engine) => {
                    let engine = Tracked::wrap(engine, &live);
                    if let Err(superseded) = plugin.install(generation, engine) {
                        drop(superseded);
                    }
                }
                Err(_) => {
                    let mut s = plugin.lock();
                    if s.generation == generation {
                        plugin.terminal_setup_failure(&mut s);
                    }
                }
            }
            drop(guard);
        });
    }

    /// `set_bridges`: classify and build first — a refused paste leaves the
    /// running client untouched, its class in the status — then rebuild.
    pub fn set_bridges(self: &Arc<Self>, bridges: Option<Zeroizing<String>>) -> i32 {
        {
            let mut s = self.lock();
            if s.registration.is_none() {
                return ZWT_RC_NOT_INITIALIZED;
            }
            let Some(cfg) = s.engine_cfg.clone() else {
                return ZWT_RC_NOT_INITIALIZED;
            };
            // New bridges need a new client, and none may start while one is
            // stuck: refuse BEFORE storing anything, so the host never believes
            // bridges are in use that are not (the final code review of plan §5).
            if s.clients_stuck() {
                return ZWT_RC_RESTART_REQUIRED;
            }
            let lines = match BridgeLines::parse(bridges.as_ref().map_or("", |b| b.as_str())) {
                Ok(lines) => lines,
                Err(class) => {
                    s.failure_class = Some(class);
                    self.emit(&mut s);
                    return ZWT_RC_BRIDGES_REFUSED;
                }
            };
            let Ok(client) = tor_config(&cfg.state_dir, &cfg.cache_dir, lines) else {
                s.failure_class = Some(setup_class());
                self.emit(&mut s);
                return ZWT_RC_BRIDGES_REFUSED;
            };
            s.engine_cfg = Some(Arc::new(EngineConfig {
                state_dir: cfg.state_dir.clone(),
                cache_dir: cfg.cache_dir.clone(),
                client,
            }));
        }
        self.rebuild();
        ZWT_RC_OK
    }

    /// `retry_bootstrap`: cut the backoff wait. A no-op unless `Failed`.
    pub fn retry_bootstrap(&self) -> i32 {
        let s = self.lock();
        if s.registration.is_none() {
            return ZWT_RC_NOT_INITIALIZED;
        }
        if s.phase == Phase::Failed {
            self.retry.notify_one();
        }
        ZWT_RC_OK
    }

    /// `clear_state`: refused while an engine runs, and while an earlier client
    /// is still STOPPING — a mint in flight, or a client `dispose` stopped
    /// waiting for at its bound, would write `state/` again after the removal
    /// (external review 2026-10-07, finding 1: unregistered is not stopped),
    /// and "restart required" while a client is stuck past its shutdown bound.
    /// Otherwise [`remove_tor_state`], under the lock, so no `init` can start a
    /// new writer between the check and the removal.
    pub fn clear_state(&self, tor_dir: &str) -> i32 {
        let s = self.lock();
        if registered(&s) {
            return ZWT_RC_NOT_INITIALIZED;
        }
        if let Some(rc) = writers_refusal(&s) {
            // The host asked for the reset; the next init finishes it even if
            // the host never asks again — after a `stopping` too, which a host
            // may meet and then lose to a force-quit (the security review of
            // the 2026-10-07 range). "Later" is answered only once the record
            // is saved; a record that failed answers its own refusal.
            return match mark_clear_pending(tor_dir) {
                Ok(()) => rc,
                Err(not_recorded) => not_recorded,
            };
        }
        remove_tor_state(tor_dir)
    }

    /// `on_paused` (§3.4 "Pause"): under the lock, `Suspended`, readiness 0
    /// pushed with `retire = 0`, arti dormant, the instant recorded on the
    /// pause clock. NO generation bump: ops in flight finish on their merits.
    pub fn on_paused(&self) -> i32 {
        let mut s = self.lock();
        if !matches!(s.phase, Phase::Bootstrapping | Phase::Ready | Phase::Failed) {
            return ZWT_RC_OK;
        }
        s.phase_before_pause = s.phase;
        s.phase = Phase::Suspended;
        s.liveness_failures = 0;
        self.push(&mut s, STARTING_AT_ZERO, false);
        if let Some(engine) = &s.engine {
            engine.set_dormant(DormantMode::Soft);
        }
        s.paused_at = self.deps.clock.now();
        tracing::info!(phase = "suspended", "paused");
        ZWT_RC_OK
    }

    /// `on_resumed` (§3.4 "Resume"): paused at least `REBUILD_AFTER_PAUSE` on
    /// the sleep-counting clock (or no such clock) → rebuild; else wake arti
    /// and re-push the LIVE state.
    pub fn on_resumed(self: &Arc<Self>) -> i32 {
        let long = {
            let mut s = self.lock();
            if s.phase != Phase::Suspended {
                return ZWT_RC_OK;
            }
            let long = match (s.paused_at, self.deps.clock.now()) {
                (Some(at), Some(now)) => now.saturating_sub(at) >= REBUILD_AFTER_PAUSE,
                _ => true,
            };
            // While a client is stuck no rebuild may start (it would strand
            // another runtime), so a long pause resumes like a short one: wake
            // the current client. Never leave the plugin `Suspended` with no
            // rebuild coming (the final code review of plan §5).
            let long = long && !s.clients_stuck();
            if !long {
                let Some(engine) = s.engine.clone() else {
                    // Paused while the first mint was still running.
                    s.phase = Phase::Bootstrapping;
                    self.emit(&mut s);
                    drop(s);
                    self.resumed.notify_waiters();
                    return ZWT_RC_OK;
                };
                engine.set_dormant(DormantMode::Normal);
                let readiness = engine.readiness();
                if s.phase_before_pause == Phase::Failed {
                    s.phase = Phase::Failed;
                    let measured = readiness_percent(&readiness)
                        .min(crate::constants::READINESS_CEILING_WHILE_BOOTSTRAPPING);
                    self.push(
                        &mut s,
                        Pushed {
                            health: Health::Failed,
                            readiness: measured,
                        },
                        false,
                    );
                } else {
                    let pair = live_pair(&readiness);
                    s.phase = if pair.health == Health::Ready {
                        Phase::Ready
                    } else {
                        Phase::Bootstrapping
                    };
                    self.push(&mut s, pair, false);
                }
                tracing::info!(rebuilt = false, "resumed");
            }
            long
        };
        if long {
            // Leaves `Suspended` inside the rebuild's own critical section.
            self.rebuild_leaving(true);
        } else {
            self.resumed.notify_waiters();
        }
        ZWT_RC_OK
    }

    /// `dispose` (§3.4 "Dispose"): under the lock, bump the generation, close
    /// every op, `NotRegistered`; `notify(retire = 1)`; wait (bounded) for
    /// every op to complete RETIRED from its own task; CLEAR the slot; drop the
    /// engine; flush the status outbox so no callback follows the return. The
    /// trampoline itself stays: a verb the wallet still calls answers RETIRED.
    /// Every wait shares ONE bound (it runs on the host's UI thread), so `OK`
    /// means UNREGISTERED, not stopped: a mint or a client can outlive the
    /// bound. `clear_state` and `init` answer `ZWT_RC_STOPPING` until they end.
    pub fn dispose(&self) -> i32 {
        let until = Instant::now() + RETIRE_QUIESCE_MAX;
        let (outbox_done, engine) = {
            let mut s = self.lock();
            if s.registration.is_none() {
                return if s.phase == Phase::Idle {
                    ZWT_RC_NOT_INITIALIZED
                } else {
                    ZWT_RC_OK
                };
            }
            s.generation += 1;
            self.generation_tx.send_replace(s.generation);
            Plugin::close_all(&mut s);
            s.phase = Phase::NotRegistered;
            s.failure_class = Some(CLASS_NOT_REGISTERED);
            self.send(&s, STARTING_AT_ZERO, true);
            s.debounce.record(STARTING_AT_ZERO);
            // ONE deadline for every wait dispose makes (the ops' drain, a
            // rebuild still running, the client's drop, the outbox's flush):
            // dispose runs on the host's UI thread, so its worst case is one
            // `RETIRE_QUIESCE_MAX`, not the sum of several (the fold
            // review's L3).
            while Plugin::outstanding(&s) > 0 || s.rebuilds_in_flight > 0 {
                let left = until.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    tracing::warn!(
                        rc = ZWT_RC_DISPOSED,
                        "dispose: ops or a rebuild still outstanding at the bound"
                    );
                    break;
                }
                s = self
                    .drained
                    .wait_timeout(s, left)
                    .unwrap_or_else(|e| e.into_inner())
                    .0;
            }
            if let Some(reg) = s.registration.take() {
                reg.link.clear(&reg.token);
            }
            let engine = s.engine.take();
            self.emit(&mut s);
            s.outbox = None;
            tracing::info!(phase = "not-registered", "disposed");
            (s.outbox_done.take(), engine)
        };
        self.resumed.notify_waiters();
        // The client is DROPPED before dispose returns (bounded): arti writes
        // its guard and timeout state when it drops (`tor-circmgr`
        // `lib.rs:1139-1146`), so a host's `clear_state` right after dispose
        // — the Tor-identity reset — must not race that write (the
        // crypto angle's LOW). The loops holding the last clones end on the
        // generation bump above.
        if let Some(engine) = engine {
            let (tx, rx) = std::sync::mpsc::channel::<()>();
            let left = until.saturating_duration_since(Instant::now());
            self.runtime.spawn(async move {
                let stop = tokio::time::Instant::now() + left;
                while Arc::strong_count(&engine) > 1 && tokio::time::Instant::now() < stop {
                    tokio::time::sleep(QUIESCE_POLL).await;
                }
                // Start the shutdown even if a late holder remains, so it is
                // not held back by that reference (`tor-plugin.md` §12).
                engine.retire();
                drop(engine);
                drop(tx);
            });
            let _ = rx.recv_timeout(left + QUIESCE_POLL);
        }
        if let Some(done) = outbox_done {
            let _ = done.recv_timeout(until.saturating_duration_since(Instant::now()));
        }
        ZWT_RC_OK
    }
}

/// A rebuild task's presence, counted under the lock: `dispose` waits
/// (bounded) until none is running, so neither the old client's drop nor a
/// mint over `state/` can follow a `clear_state` (the fold review's
/// MEDIUM). Dropped at the end of the task, a panic included.
struct InFlight(Arc<Plugin>);

impl Drop for InFlight {
    fn drop(&mut self) {
        let mut s = self.0.lock();
        s.rebuilds_in_flight = s.rebuilds_in_flight.saturating_sub(1);
        self.0.drained.notify_all();
    }
}

/// The blockage arti reports while bootstrapping.
fn blockage_of(readiness: &Readiness) -> Option<Blockage> {
    match readiness {
        Readiness::Bootstrapping {
            blockage: Some(kind),
            ..
        } => Some(Blockage::from_arti(kind)),
        _ => None,
    }
}
