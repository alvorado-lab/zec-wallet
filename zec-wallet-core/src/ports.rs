//! Outbound network seam (spec §3.2).
//!
//! Transport pluggability is the same philosophy as the host messenger's
//! driver/transport split: the engine builds its channels over WHATEVER this
//! dials — built-in arti is one impl of the seam, not a special case. That is
//! what keeps the SDK usable over a host app's existing network
//! infrastructure (Orbot SOCKS, a shared arti instance, a test loopback)
//! without the SDK knowing any of them by name.
//!
//! (`SyncEnginePort` — the Tachyon seam — lands with the sync engine; it is
//! crate-internal and has no consumer in the W2 skeleton.)

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::error::DialError;

/// Byte-stream object the dialer returns. Blanket-implemented: any
/// `AsyncRead + AsyncWrite` duplex qualifies, so host dialers compose from
/// plain tokio types.
pub trait AsyncByteStream: AsyncRead + AsyncWrite + Send + Unpin {}

impl<T: AsyncRead + AsyncWrite + Send + Unpin> AsyncByteStream for T {}

/// Network runtime seam (maintainer 2026-06-10 — "the SDK may reuse the host
/// app's infrastructure"). All SDK traffic — sync gRPC AND swap REST — rides
/// a dialer; there is no independent clearnet path (§3.2 m5).
#[async_trait]
pub trait NetDialer: Send + Sync {
    /// Dial a TCP-like byte stream to `host:port`.
    ///
    /// **WHAT `Ok` ASSERTS (stage S1 `truth`).** The byte stream is
    /// ESTABLISHED END TO END: a proxy has completed its CONNECT reply, a
    /// circuit is built, and bytes written to the returned stream are already
    /// on their way to `host:port`. An `Ok` that merely handed back a socket
    /// or a duplex pipe while the real connection was still being made — a
    /// SOCKS front end that answers before the proxy replies, a plugin that
    /// builds its circuit lazily behind a pipe — is a LIE the SDK cannot
    /// detect, and it does not merely delay: the dial's `Ok` is the SDK's only
    /// evidence that the private path did its part, so a `Required` wallet
    /// would then report `Active` and "the server is not answering" for a path
    /// that is dead — the private path taking credit for traffic it never
    /// carried, which is the one direction the transport state exists to
    /// prevent (`net/grpc.rs::transport_stall`, FR-36). A transport that
    /// cannot promise this MUST return [`DialError::Unreachable`] or
    /// [`DialError::Timeout`] instead of an optimistic `Ok`; being slow is
    /// always safe, and the caller bounds the dial itself
    /// (`DIAL_TIMEOUT_SECS`).
    ///
    /// `isolation_key` contract (§2.3): equal keys MAY share a circuit/path;
    /// distinct keys MUST be unlinkable at the network layer (per-txid keys
    /// make actions on one tx unlinkable from everything else — the upstream
    /// ServiceMode pattern). The SDK's OWN dialers and built-in arti that
    /// cannot honor isolation MUST return `DialError::Unsupported` rather
    /// than silently linking (ADR-0526). That clause is RETIRED for a
    /// HOST-PROVIDED dialer (`TorRuntime::HostDialer`; ADR-0545, FR-29 spec
    /// §0 A4): the host is trusted, the key is passed on every dial, a
    /// non-isolating host transport ignores it and never refuses for it, and
    /// the host's descriptor (`HostDialer::descriptor`) says whether the key
    /// was honoured — the state renders that honestly instead of refusing.
    async fn dial(
        &self,
        host: &str,
        port: u16,
        isolation_key: Option<&str>,
    ) -> Result<Box<dyn AsyncByteStream>, DialError>;
}

/// The device wall clock, as a port (GRACE-1, `production-readiness-phase-1.md`
/// §4p P-G2 — the seam the contract names so both blind halves build against
/// it).
///
/// **What it is for.** The §6.3 unknown-branch grace has a second expiry
/// dimension the endpoint cannot freeze: seconds on the DEVICE clock since the
/// last signing-capable verdict (`consensus_stamp`). That rule is a SIGNING
/// input, so its "now" must be injectable — a proof drives the clock forward a
/// day, backward past the recorded time, two years wrong — through the same
/// call the shipped build makes. The `Wallet` holds one of these on `Inner`,
/// injected through `open_with_vault_and_seed_port` (`None` = [`SystemClock`]
/// in every `pub` constructor), and every reader of the consensus stamp asks
/// it. It is NOT the SDK's monotonic timing (that stays `tokio::time`), and it
/// is not the display-only `SystemTime::now()` the sync stamp takes inline —
/// those never gate a signature.
///
/// **Trust.** The value is UNTRUSTED, and the rule it feeds can move in one
/// direction only: a clock can only SHORTEN the grace, and a time the wallet
/// cannot trust ENDS it (GRACE-2, §4v — the maintainer's decision 2026-09-10,
/// item 1, on §4p-run review row 2). A reading AFTER the recorded capable
/// time counts elapsed seconds, and the first reader to see a day writes a
/// durable latch (`consensus_stamp::observe`); a reading BEFORE the recorded
/// capable time — the maintainer's "a recorded timestamp in the future is
/// untrusted" — writes the SAME latch, fail-closed. GRACE-1 read that cell as
/// "no time passed" and let the block rule decide alone, which a frozen-tip
/// silent server defeats: one capable pass under a wrong clock then left the
/// grace with no expiry on either axis. Only a capable verdict clears the
/// latch, re-recording the time from this port as it reads then.
///
/// **The residual, and it is a real one (§4v-run review row 1; the maintainer's
/// decision 2026-09-11 — documented, not built).** Every rule above is about a
/// clock that MOVES. A clock that does not ADVANCE defeats them: `untrusted`
/// fires on `capable_at == 0 || capable_at > now`, so a clock held anywhere
/// inside `[capable_at, capable_at + UNKNOWN_BRANCH_GRACE_SECS − 1]` is
/// trusted, `elapsed` never reaches the threshold, and the latch is never
/// written. An attacker who runs the lightwalletd AND the device's
/// unauthenticated NTP source can freeze the claimed tip, withhold
/// `consensus_branch_id` and hold the clock still, and `permits_signing` stays
/// true indefinitely — the §4n-review row 1 attack, surviving in the
/// stopped-clock cell. Closing it needs a THIRD axis neither the endpoint nor
/// the wall clock can freeze (monotonic uptime accumulated across passes, or a
/// high-water reading latched when the clock fails to advance over N passes),
/// and that is NOT built. The sentence this paragraph replaced is the warning:
/// it said "nothing the clock does can make the wallet refuse LATER than the
/// rules would", which is false for a clock that does nothing at all.
pub(crate) trait WallClock: Send + Sync {
    /// Seconds since the Unix epoch, as this device believes them. A pre-epoch
    /// clock reads as `0` (the `sync_stamp` precedent) — and a capable verdict
    /// recorded at `0` is a time the rule cannot measure from: it reads as
    /// EXPIRED by the clock at every later read (§4v), never as "a day left".
    fn now_unix(&self) -> u64;
}

/// The shipped [`WallClock`]: the operating system's wall clock.
pub(crate) struct SystemClock;

impl WallClock for SystemClock {
    fn now_unix(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
    }
}

/// Shared crate-test doubles for the port (one source of truth — duplicates are bugs).
#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    /// A zero-behavior [`NetDialer`] that never produces a stream — for tests that only
    /// need SOME `Arc<dyn NetDialer>` to construct a `TorRuntime::Dialer`/policy and never
    /// actually dial (e.g. the `tor_status` mapping + the `wallet` `tor_state` wiring). A
    /// dial would be a test bug, so it returns `Unreachable` rather than panicking.
    pub(crate) struct StubDialer;

    #[async_trait]
    impl NetDialer for StubDialer {
        async fn dial(
            &self,
            _host: &str,
            _port: u16,
            _isolation_key: Option<&str>,
        ) -> Result<Box<dyn AsyncByteStream>, DialError> {
            Err(DialError::Unreachable)
        }
    }

    /// A host runtime that is DOWN — every dial returns `Unreachable`. For end-to-end
    /// tests of a degraded network: a `Required` wallet fail-closes (zero clearnet) and
    /// a `Preferred` wallet falls back to its clearnet `DirectTcpDialer`. (Distinct from
    /// [`StubDialer`], which models a runtime that's never actually dialed.)
    pub(crate) struct FailingDialer;

    #[async_trait]
    impl NetDialer for FailingDialer {
        async fn dial(
            &self,
            _host: &str,
            _port: u16,
            _isolation_key: Option<&str>,
        ) -> Result<Box<dyn AsyncByteStream>, DialError> {
            Err(DialError::Unreachable)
        }
    }
}
