//! The seam under the plugin (`tor-plugin.md` §2): what the trampoline and the
//! lifecycle drive. Production is [`ArtiEngine`] over `dialer_tor::OwnedDialer`;
//! the tests drive a fake with a scripted readiness and `tokio::io::duplex`
//! streams, so every rule of §3.3/§3.4 is proved without the network (§3.5).
//!
//! It returns `dialer-tor`'s OWN error type on purpose (the arch angle's
//! MINOR, accepted in the spec): the plugin's taxonomy is the one table in
//! `dial_codes.rs`, and a plugin-owned error enum between the two would be a
//! second taxonomy with its own drift.
//!
//! The client's lifetime is `dialer-tor`'s (§12, ADR-0572): each client runs
//! on a runtime of its own, its shutdown ends every task arti started, and the
//! crate's [`ClientLedger`] counts it from that runtime's construction until
//! the shutdown has finished. The plugin keeps ONE ledger and reads it.

use std::path::PathBuf;
use std::sync::Arc;

use crate::constants::ENGINE_SHUTDOWN_MAX;

use async_trait::async_trait;
use dialer_tor::{
    AsyncByteStream, ClientLedger, DormantMode, LedgerEntry, OwnedDialer, OwnedOptions, Readiness,
    TorClientConfig, TorDialError, TorDialer,
};

/// One Tor client, as the plugin sees it. Object-safe, `Send + Sync`.
#[async_trait]
pub trait TorEngine: Send + Sync {
    /// Bootstrap. Slow, fallible, idempotent.
    async fn bootstrap(&self) -> Result<(), TorDialError>;
    /// What the client would do right now — a local read, no traffic.
    fn readiness(&self) -> Readiness;
    /// Open a stream through Tor. `None` joins the client's unkeyed group.
    async fn connect(
        &self,
        host: &str,
        port: u16,
        isolation_key: Option<&str>,
    ) -> Result<Box<dyn AsyncByteStream>, TorDialError>;
    /// Hand arti a dormant mode (battery; readiness 0 is what stops dials).
    fn set_dormant(&self, mode: DormantMode);
    /// Start this client's shutdown NOW, without waiting for its other
    /// holders: a rebuild or a `dispose` calls it before letting go, so a
    /// late reference (an op still in flight) cannot hold the old client's
    /// shutdown back. The default does nothing: a client with nothing of its
    /// own to stop.
    fn retire(&self) {}
    /// Take over the plugin's ledger entry for this client, to release it
    /// only once everything the client started has ended. `Some` hands it
    /// back: the wrapper then releases it when the client itself drops (the
    /// default, right for a client with no tasks of its own).
    fn adopt_live(&self, entry: LedgerEntry) -> Option<LedgerEntry> {
        Some(entry)
    }
}

/// A minted client, counted in the plugin's ledger until EVERYTHING it
/// started has ended (2026-10-07 external review, finding 1). The entry goes
/// to the client first ([`TorEngine::adopt_live`]): the production client is
/// already counted by `dialer-tor` from the moment its runtime exists, so it
/// drops this one; a client that hands it back (the test fakes) is covered by
/// field order: `inner` drops first, then `_live`.
pub(crate) struct Tracked {
    inner: Arc<dyn TorEngine>,
    _live: Option<LedgerEntry>,
}

impl Tracked {
    /// Wrap a freshly minted client; `live` is the plugin's ledger.
    pub(crate) fn wrap(inner: Arc<dyn TorEngine>, live: &Arc<ClientLedger>) -> Arc<dyn TorEngine> {
        let kept = inner.adopt_live(live.enter());
        Arc::new(Self { inner, _live: kept })
    }
}

#[async_trait]
impl TorEngine for Tracked {
    async fn bootstrap(&self) -> Result<(), TorDialError> {
        self.inner.bootstrap().await
    }
    fn readiness(&self) -> Readiness {
        self.inner.readiness()
    }
    async fn connect(
        &self,
        host: &str,
        port: u16,
        isolation_key: Option<&str>,
    ) -> Result<Box<dyn AsyncByteStream>, TorDialError> {
        self.inner.connect(host, port, isolation_key).await
    }
    fn set_dormant(&self, mode: DormantMode) {
        self.inner.set_dormant(mode);
    }
    fn retire(&self) {
        self.inner.retire();
    }
}

/// What a mint needs: the two directories (§2.3) and the arti configuration
/// BUILT at `init` or `set_bridges` — built once, before registration, so a
/// paste arti refuses is refused typed there and never here. No `Debug`:
/// arti's configuration derives it and would print the bridges.
pub struct EngineConfig {
    /// `<tor_dir>/state`.
    pub state_dir: PathBuf,
    /// `<tor_dir>/cache`.
    pub cache_dir: PathBuf,
    /// The built client configuration. Reused by every rebuild (`BridgeLines`
    /// is not `Clone`, so a rebuild cannot re-parse; it clones this instead).
    pub client: TorClientConfig,
}

/// How an engine is minted — the seam P1/P2 count calls on ("arti was never
/// constructed").
#[async_trait]
pub trait EngineFactory: Send + Sync {
    /// A new client over `cfg`, not bootstrapped. `live` is the plugin's
    /// ledger: a client that starts work of its own (the production one's
    /// runtime) counts itself in it from the moment that work can exist, so a
    /// mint that fails half-way is still counted until its work has ended,
    /// and while a client is stuck no new one is built (`RestartRequired`).
    async fn mint(
        &self,
        cfg: &EngineConfig,
        live: &Arc<ClientLedger>,
    ) -> Result<Arc<dyn TorEngine>, TorDialError>;
}

/// Production: arti, through the shared crate's OWNED client.
pub struct ArtiEngine {
    dialer: OwnedDialer,
}

#[async_trait]
impl TorEngine for ArtiEngine {
    async fn bootstrap(&self) -> Result<(), TorDialError> {
        self.dialer.bootstrap().await
    }

    /// A retired client reads as not started: its generation has ended, and
    /// nothing may dial it.
    fn readiness(&self) -> Readiness {
        self.dialer.readiness().unwrap_or(Readiness::Bootstrapping {
            progress: 0.0,
            blockage: None,
        })
    }

    async fn connect(
        &self,
        host: &str,
        port: u16,
        isolation_key: Option<&str>,
    ) -> Result<Box<dyn AsyncByteStream>, TorDialError> {
        self.dialer.dial(host, port, isolation_key).await
    }

    /// A retired client has nothing to set; `Closed` is ignored.
    fn set_dormant(&self, mode: DormantMode) {
        let _ = self.dialer.set_dormant(mode);
    }

    fn retire(&self) {
        self.dialer.start_shutdown(ENGINE_SHUTDOWN_MAX);
    }

    /// This client is counted by `dialer-tor` from its runtime's construction
    /// until that runtime has shut down; the wrapper's entry would only add a
    /// second one released too early, so it is dropped here.
    fn adopt_live(&self, entry: LedgerEntry) -> Option<LedgerEntry> {
        drop(entry);
        None
    }
}

/// Production factory: an owned client over the config built at `init` —
/// never `with_bridges`, which would need the parsed lines again. Its
/// unawaited shutdown (a client dropped without `retire`) uses the plugin's
/// own bound.
pub struct ArtiFactory;

#[async_trait]
impl EngineFactory for ArtiFactory {
    async fn mint(
        &self,
        cfg: &EngineConfig,
        live: &Arc<ClientLedger>,
    ) -> Result<Arc<dyn TorEngine>, TorDialError> {
        let mut options = OwnedOptions::default();
        options.thread_name = "zec-wallet-tor-client";
        options.drop_budget = ENGINE_SHUTDOWN_MAX;
        let dialer = TorDialer::spawn_owned(cfg.client.clone(), live, options).await?;
        Ok(Arc::new(ArtiEngine { dialer }))
    }
}
