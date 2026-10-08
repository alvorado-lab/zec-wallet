//! FR-29 — the readiness gate in front of a registered host dialer (the
//! wave review's MEDIUM, found by all three angles; spec §6.1 E3, T26).
//!
//! The descriptor is AUTHORITATIVE (ADR-0545 D3), so a host that reports
//! "not ready" is never ASKED to dial: a host that would park the dial while
//! its transport bootstraps — instead of refusing `NOT_READY` synchronously —
//! would otherwise run the SDK into `DIAL_TIMEOUT_SECS`, whose elapse is a
//! reachability `Timeout` and, under `Preferred`, the clearnet fallback at
//! every app start — the leak ADR-0546 exists to prevent, arriving by a
//! non-code path. A cleared registry (`None`) fails `Retired`, the arm the
//! bridge's `CAbiHostDialer::dial` itself reports for an empty slot: through
//! `resolve_dialer` this gate answers first and the bridge's arm serves its
//! own direct callers and tests; both say `Retired`, and a change to one must
//! change the other. Both errors land in `PolicyDialer`'s surface-as-is arm;
//! neither writes the fell-back latch.
//!
//! **`is_ready()` carries the HEALTH axis too (ABI v3, ADR-0549).** It is the
//! ONE predicate this gate and `live_tor_state` share, and since C1 it reads
//! `readiness >= 100 && !is_failed()` — so a registrant that declares its
//! transport FAILED is refused at ANY readiness, its own 100 included, and
//! the host is never asked to dial it. Nor does it leave the user on a
//! spinner — the state renders `Unavailable` with the transport's name and a
//! next step, which is the whole of FR-30 (b).
//!
//! **A declared failure is refused as `TransportFailed`, not `NotReady`
//! (ADR-0552 phase 2 §2a).** The two used to share `NotReady`, which left
//! ADR-0553 unbuildable: a bootstrapping transport refuses continuously, so
//! letting `NotReady` spend the patience window would leak clearnet at every
//! slow start, and never letting it would strand a wallet whose transport is
//! not going to start. The health axis is the fact that separates them. This
//! gate only REPORTS which one it saw — `PolicyDialer` decides what each is
//! worth: `NotReady` never reaches clearnet however long it lasts;
//! `TransportFailed` is the private path failing, and a minute of it may.
//!
//! Its own module (not the tail of `net/dialer.rs`): clippy forbids items
//! after a test module, and `dialer.rs`'s test module carries registered
//! mutant rows whose cited lines must stay where their watches printed them.

use std::sync::Arc;

use async_trait::async_trait;

use crate::error::DialError;
use crate::net::host_dialer::HostDialer;
use crate::ports::{AsyncByteStream, NetDialer};

/// The gate: a `NetDialer` over a registered [`HostDialer`] that consults the
/// descriptor before every dial.
pub(crate) struct ReadinessGated(pub(crate) Arc<dyn HostDialer>);

/// The gated dialer for `runtime_dialer`'s `HostDialer` arm (a constructor so
/// that arm stays one line — its file's cited test lines must not move).
pub(crate) fn gated(host: &Arc<dyn HostDialer>) -> Arc<dyn NetDialer> {
    Arc::new(ReadinessGated(Arc::clone(host)))
}

#[async_trait]
impl NetDialer for ReadinessGated {
    async fn dial(
        &self,
        host: &str,
        port: u16,
        isolation_key: Option<&str>,
    ) -> Result<Box<dyn AsyncByteStream>, DialError> {
        match self.0.descriptor() {
            None => Err(DialError::Retired),
            // Refused either way, the host never asked. The gate only CARRIES
            // the health verdict out; what a declared failure is worth is
            // `PolicyDialer`'s to decide, the one place that does (ADR-0553).
            Some(descriptor) if descriptor.is_failed() => Err(DialError::TransportFailed),
            Some(descriptor) if !descriptor.is_ready() => Err(DialError::NotReady),
            Some(_) => self.0.dial(host, port, isolation_key).await,
        }
    }
}
