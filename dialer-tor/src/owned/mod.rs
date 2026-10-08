//! The OWNED client (`tor-plugin.md` §12, ADR-0572): a [`TorDialer`] on a
//! runtime of its own, with an awaited and bounded [`OwnedDialer::shutdown`]
//! that ends every task the client started, and a [`ClientLedger`] the host
//! reads to know whether a client may still write.
//!
//! Use it where the CLIENT'S LIFETIME matters: a Tor identity reset (deleting
//! the state directory must not be undone by a late write), or an "Off" that
//! must really stop Tor traffic. Dropping a plain [`TorDialer`] does not end
//! arti's background tasks; shutting an [`OwnedDialer`] down does.
//!
//! Host requirement, unchanged from [`TorDialer`]: poll the streams inside a
//! tokio context with timers enabled (arti's per-stream sleeps use the
//! POLLING runtime).

mod core;

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

pub use self::core::{ClientLedger, LedgerEntry, Shutdown};
use self::core::{Gated, Host, Owned, OwnedRuntime, Spawner, thread_spawner};
use crate::constants::RECOMMENDED_SHUTDOWN_BUDGET;
use crate::{
    AsyncByteStream, DormantMode, ErrorKind, ExitRotation, Readiness, TorClientConfig,
    TorDialError, TorDialer, TorStream,
};

/// How an owned client is built. Start from [`OwnedOptions::default`] and set
/// the fields you need: the struct is `#[non_exhaustive]`, so a later field
/// is not a breaking change.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct OwnedOptions {
    /// The owned runtime's thread name, for the host's own logs.
    pub thread_name: &'static str,
    /// The exit-rotation policy (ADR-0567); ON by default, as for
    /// [`TorDialer`].
    pub exit_rotation: ExitRotation,
    /// The budget an UNAWAITED shutdown uses: the last drop of a client never
    /// shut down, or a [`TorDialer::spawn_owned`] cancelled half-way.
    pub drop_budget: Duration,
}

impl Default for OwnedOptions {
    fn default() -> Self {
        Self {
            thread_name: "dialer-tor-client",
            exit_rotation: ExitRotation::default(),
            drop_budget: RECOMMENDED_SHUTDOWN_BUDGET,
        }
    }
}

/// A Tor client on a runtime of its own. Every clone is the same client; the
/// last clone's drop, without a [`Self::shutdown`], shuts it down unawaited.
#[derive(Clone)]
pub struct OwnedDialer {
    host: Arc<Host<TorDialer>>,
}

impl std::fmt::Debug for OwnedDialer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OwnedDialer")
            .field("closed", &self.is_closed())
            .finish_non_exhaustive()
    }
}

impl TorDialer {
    /// Build a dialer over `config` on a runtime of its OWN, WITHOUT
    /// bootstrapping, and count it in `ledger` from the moment that runtime
    /// exists until its shutdown has finished.
    ///
    /// First waits for every shutdown already started in `ledger` to settle
    /// (`ledger.stopping()`; each settles within its own budget), then is
    /// refused with [`TorDialError::RestartRequired`] while `ledger.stuck()`
    /// is not zero: an earlier client may still write its state. So a client
    /// started after an earlier one's `start_shutdown` returned never runs
    /// beside it, awaited or not. The wait is bounded by the longest budget
    /// handed off in `ledger` plus [`crate::SHUTDOWN_WAIT_GRACE`]; a
    /// shutdown still unsettled then is answered as `RestartRequired` too
    /// (fail closed). While a shutdown is in flight the wait needs a tokio
    /// timer, as [`OwnedDialer::shutdown`] does. A host that builds with
    /// bridges composes `config` with [`crate::tor_config`]. Callable from
    /// any tokio runtime. Cancel-safe: a future dropped during the wait holds
    /// nothing, and one dropped half-way through the build shuts the
    /// half-built client down (unawaited, `drop_budget`).
    pub async fn spawn_owned(
        config: TorClientConfig,
        ledger: &Arc<ClientLedger>,
        options: OwnedOptions,
    ) -> Result<OwnedDialer, TorDialError> {
        spawn_owned_with(config, ledger, options, thread_spawner).await
    }
}

pub(crate) async fn spawn_owned_with(
    config: TorClientConfig,
    ledger: &Arc<ClientLedger>,
    options: OwnedOptions,
    spawner: Spawner,
) -> Result<OwnedDialer, TorDialError> {
    // A shutdown in flight may still overrun: wait for its verdict, so a
    // stuck client and a new one never run side by side. One that has not
    // settled within its bound is treated as stuck (fail closed).
    if !ledger.shutdowns_settled().await || ledger.stuck() > 0 {
        return Err(TorDialError::RestartRequired);
    }
    let runtime = OwnedRuntime::new(ledger, options.thread_name, options.drop_budget, spawner)
        .map_err(|_| TorDialError::Setup {
            kind: ErrorKind::LocalResourceExhausted,
        })?;
    let (owned, host) = Owned::start(runtime);
    let host = Arc::new(host);
    let rotation = options.exit_rotation;
    // On the owned runtime, so arti binds it (`PreferredRuntime::current()`);
    // the dialer is installed there and never handed back through the task.
    owned
        .mint(async move {
            TorDialer::from_config(config)
                .await
                .map(|dialer| dialer.exit_rotation(rotation))
        })
        .await?;
    Ok(OwnedDialer { host })
}

impl OwnedDialer {
    fn owned(&self) -> &Arc<Owned<TorDialer>> {
        self.host.owned()
    }

    /// [`TorDialer::bootstrap`], run on the owned runtime. Dropping the
    /// future aborts the bootstrap.
    pub async fn bootstrap(&self) -> Result<(), TorDialError> {
        self.owned()
            .call(|dialer| async move { dialer.bootstrap().await })
            .await
    }

    /// [`TorDialer::connect`], run on the owned runtime; the stream comes back
    /// behind the poll gate. Dropping the future aborts the dial, which then
    /// rotates nothing, as for [`TorDialer`].
    pub async fn connect(
        &self,
        host: &str,
        port: u16,
        isolation_key: Option<&str>,
    ) -> Result<OwnedStream, TorDialError> {
        let host = host.to_owned();
        let key = isolation_key.map(str::to_owned);
        let stream = self
            .owned()
            .call(move |dialer| async move { dialer.connect(&host, port, key.as_deref()).await })
            .await?;
        Ok(OwnedStream(Gated::new(stream, self.owned().gate())))
    }

    /// [`Self::connect`] in the boxed shape of the wallet SDK's `NetDialer`
    /// port, as [`TorDialer::dial`].
    pub async fn dial(
        &self,
        host: &str,
        port: u16,
        isolation_key: Option<&str>,
    ) -> Result<Box<dyn AsyncByteStream>, TorDialError> {
        Ok(Box::new(self.connect(host, port, isolation_key).await?))
    }

    /// [`TorDialer::readiness`]; `Closed` after a shutdown.
    pub fn readiness(&self) -> Result<Readiness, TorDialError> {
        self.owned().with(TorDialer::readiness)
    }

    /// [`TorDialer::set_dormant`]; `Closed` after a shutdown.
    pub fn set_dormant(&self, mode: DormantMode) -> Result<(), TorDialError> {
        self.owned().with(|dialer| dialer.set_dormant(mode))
    }

    /// [`TorDialer::dormant_mode`]; `Closed` after a shutdown.
    pub fn dormant_mode(&self) -> Result<DormantMode, TorDialError> {
        self.owned().with(TorDialer::dormant_mode)
    }

    /// Hand the shutdown off NOW, synchronously, and return: for a caller
    /// that cannot await (a synchronous transition, a wipe's teardown). The
    /// first call on any clone does it, with its `budget`; later calls do
    /// nothing.
    pub fn start_shutdown(&self, budget: Duration) {
        self.owned().start_shutdown(budget);
    }

    /// Start the shutdown now, then await its one outcome: [`Shutdown::Stopped`]
    /// once every task the client started has ended inside `budget`, else
    /// [`Shutdown::Overran`] (the client stays counted and stuck in the
    /// ledger for the process). Every caller, on any clone, gets the same
    /// outcome; dropping the future loses nothing. The shutdown runs with the
    /// budget of the FIRST call ([`Self::start_shutdown`] included), and
    /// every caller waits by that budget, not its own. The wait is bounded:
    /// past that budget plus [`crate::SHUTDOWN_WAIT_GRACE`] the shutdown is
    /// settled `Overran`, unless it has already settled, and every caller
    /// gets the outcome that settled.
    ///
    /// Await it inside a tokio runtime with timers enabled (the bound is a
    /// tokio timer; without one the future panics). Code that cannot await,
    /// or has no timer, calls [`Self::start_shutdown`] and reads the ledger.
    /// Never await it from a task on the client's own runtime: that starts
    /// the shutdown, logs an error and answers `Overran` at once, stranding
    /// the client for the process.
    pub fn shutdown(&self, budget: Duration) -> impl Future<Output = Shutdown> + Send + 'static {
        self.owned().shutdown(budget)
    }

    /// Whether a shutdown has started.
    pub fn is_closed(&self) -> bool {
        self.owned().is_closed()
    }
}

/// A stream from an [`OwnedDialer`]: a [`TorStream`] behind the poll gate.
/// After a shutdown every poll answers `NotConnected` without reaching arti,
/// and a poll parked before it is woken to do so.
pub struct OwnedStream(Gated<TorStream>);

impl std::fmt::Debug for OwnedStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OwnedStream").finish_non_exhaustive()
    }
}

impl AsyncRead for OwnedStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}

impl AsyncWrite for OwnedStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.0).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_shutdown(cx)
    }
}
