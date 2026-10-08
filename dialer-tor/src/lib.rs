//! A standalone Tor **dialer**: give it `host:port`, get a duplex byte stream
//! that left your machine inside a Tor circuit.
//!
//! # Direction
//!
//! This crate depends on NO `relim-*` crate and knows nothing about Relim's
//! domain types. It is meant to be consumable from outside this repository —
//! notably by the `zec-wallet` SDK, whose frozen `NetDialer` port
//! (`sdk/zec-wallet-core/src/ports.rs`) has the shape [`TorDialer::dial`]
//! matches: `dial(host, port, isolation_key) -> Box<dyn AsyncByteStream>`.
//! That port is the seam and lives there; this crate does not declare a second
//! one. An adapter in the consuming product implements the port over this
//! dialer, never the other way round. The same holds for Relim's own
//! `Transport` trait, which crosses Relim domain types and returns a
//! multiplexed connection rather than a byte stream.
//!
//! # It fails closed
//!
//! There is no clearnet path in this crate. A stream can only be produced by
//! [`TorStream`], whose only constructor takes an `arti_client::DataStream`,
//! and the dialer is built with `BootstrapBehavior::Manual`: until
//! [`TorDialer::bootstrap`] has succeeded, every dial returns
//! [`TorDialError::NotBootstrapped`] rather than connecting some other way or
//! blocking on an invisible bootstrap.
//!
//! # `.onion` is not supported, on purpose
//!
//! arti's onion-service support reaches the `equix`/`hashx` crates, which are
//! LGPL-3.0-only and cannot ship inside a third-party product. The features
//! that would pull them are off (see this crate's `Cargo.toml` entry for
//! `arti-client`), so a `.onion` target is refused with
//! [`TorDialError::OnionUnsupported`].
//!
//! # Errors carry a class, and five of them decide a clearnet fallback
//!
//! Every refusal is a [`TorDialError`]; [`TorDialError::class`] is the short,
//! stable, PII-free label to log and count. The three timeout kinds and the
//! two network kinds arti distinguishes are FIVE variants here
//! ([`TorDialError::TorNetworkTimeout`], [`TorDialError::ExitTimeout`],
//! [`TorDialError::RemoteNetworkTimeout`]; [`TorDialError::LocalNetworkError`],
//! [`TorDialError::TorAccessFailed`]), never folded, so a consumer with a
//! clearnet fallback can refuse the ones a relay or the destination can
//! cause at will and follow only the ones the device or the Tor network
//! produces. Read each variant's doc before mapping it to a policy: the names
//! are arti's, and in arti 0.45.0 `LocalNetworkError` is a guard-channel
//! error, not "the device has no route". The README carries the table.
//!
//! # Dormancy, logging, features
//!
//! [`TorDialer::set_dormant`] / [`TorDialer::dormant_mode`] pass a
//! [`DormantMode`] to arti and record it (arti wakes itself on any use; `Soft`
//! also turns channel padding off — the verb's doc). Logging is `tracing`
//! only, with closed values; a refused bridge line logs its class under the
//! target `dialer_tor::censor`. The one feature, `static-sqlite`, vendors
//! sqlite for arti's directory cache on targets with none to link.
//!
//! # A lifetime you can end
//!
//! Dropping a [`TorDialer`] does not stop arti's background work. Where that
//! matters (an "Off" that must stop Tor, or deleting the state directory to
//! reset the Tor identity), build with [`TorDialer::spawn_owned`]: the client
//! runs on a runtime of its own, and [`OwnedDialer::shutdown`] ends everything
//! it started, inside a budget, reporting [`Shutdown::Stopped`] or
//! [`Shutdown::Overran`]. A [`ClientLedger`] counts the clients that may still
//! write their state.
//!
//! # Where this crate lives
//!
//! `sdk/dialer-tor` in the zec-wallet repository, its own cargo workspace
//! (ADR-0551: arti's RustCrypto line and the Zcash stack's pre-release pins
//! cannot share a lock), consumed by path until its crates.io publish
//! (ADR-0550). Copied from the first host's Tor transport crate; `CHANGELOG.md`
//! lists what a consumer of that crate must change.

#![forbid(unsafe_code)]

mod bridges;
mod constants;
mod dialer;
mod error;
mod isolation;
mod owned;
mod stream;

use tokio::io::{AsyncRead, AsyncWrite};

pub use bridges::{
    BridgeLines, CLASS_CONFIG_TOO_LONG, CLASS_LINE_TOO_LONG, CLASS_PT_UNSUPPORTED,
    CLASS_TOO_MANY_LINES, CLASS_UNUSABLE, MAX_BRIDGE_CONFIG_BYTES, MAX_BRIDGE_LINE_BYTES,
    MAX_BRIDGE_LINES, bridge_class,
};
pub use constants::{
    OWNED_MAX_BLOCKING_THREADS, OWNED_WORKER_THREADS, RECOMMENDED_SHUTDOWN_BUDGET,
    SHUTDOWN_WAIT_GRACE,
};
pub use dialer::{TorDialer, tor_config};
pub use error::{Readiness, TargetProblem, TorDialError};
pub use isolation::{
    EXIT_ROTATION_INTERVAL, EXIT_ROTATION_INTERVAL_MAX, ExitRotation, MAX_ISOLATION_KEYS,
};
pub use owned::{ClientLedger, LedgerEntry, OwnedDialer, OwnedOptions, OwnedStream, Shutdown};
pub use stream::TorStream;

/// arti's own configuration and status types, re-exported so a caller can
/// build a full [`arti_client::TorClientConfig`] (bridges, timeouts, paths)
/// without this crate mirroring arti's options.
pub use arti_client;
pub use arti_client::{DormantMode, ErrorKind, TorClientConfig, status::BlockageKind};

/// Byte-stream object the dialer returns.
///
/// Blanket-implemented over the `tokio` duplex traits, so it is the same BOUND
/// the `zec-wallet` SDK's `NetDialer` port declares — and a NOMINALLY DISTINCT
/// trait, in another cargo workspace. `Box<dyn AsyncByteStream>` from here does
/// NOT coerce to the SDK's (`E0308`: two identical traits are still two traits).
/// An adapter either re-boxes what [`TorDialer::dial`] returns, or boxes the
/// [`TorStream`] from [`TorDialer::connect`] for one allocation fewer — the
/// SDK's own blanket impl accepts either.
pub trait AsyncByteStream: AsyncRead + AsyncWrite + Send + Unpin {}

impl<T: AsyncRead + AsyncWrite + Send + Unpin> AsyncByteStream for T {}
