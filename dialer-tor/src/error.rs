//! What went wrong, in words a user can act on.
//!
//! **A downstream consumer reads THIS FILE at compile time, from disk — so
//! an uncommitted edit here can red its build.** The consumer includes this
//! source (`include_str!`) to bind a POSITIVE ALLOWLIST mapping our variants
//! onto the wallet's frozen `ZW_DIAL_*` codes, so its gate grades OUR variant
//! set, and a rename here fails its build rather than drifting silently.
//!
//! **Do not move or rename this file, and treat [`TorDialError::class`]'s
//! strings as a published surface.** That direction is deliberate and is the
//! loud one; what it costs us is that the path, the file name and the
//! `class()` strings are now contract. A consumer that counts failures per
//! class keys its counters by these strings, so a moved label silently
//! re-denominates its counts.
//!
//! (The same holds for any consumer that parses this file's declaration
//! rather than linking the enum: the variant set is the contract it reads,
//! and a new variant must be mapped there before it can ship.)
//!
//! The mapping it binds, recorded here so a change to a `class()` string is
//! visibly a change to someone's money path: `not-bootstrapped` and
//! `bootstrap-failed` → `NOT_READY`; `tor-network-timeout` → `TIMEOUT` (the
//! maintainer's "keep it", ADR-0546 — the ONE third-party-inducible fallback);
//! everything else → `REFUSED`, which is the DEFAULT, so an unmapped or new
//! variant fails closed. `UNREACHABLE` (code 2) is reachable from NO variant
//! of ours by construction — it is the registrant's own device-offline
//! derivation — and it is one of only two codes a `Preferred` wallet follows
//! to clearnet, which is why nothing here may start producing it silently.

use arti_client::status::BlockageKind;
use arti_client::{ErrorKind, HasKind as _};

/// How far along the Tor client is.
/// Not `PartialEq`: arti's `BlockageKind` is `#[non_exhaustive]` and does not
/// implement it, and mirroring it here would be a second answer to "what is
/// arti stuck on".
#[derive(Debug, Clone)]
pub enum Readiness {
    /// [`crate::TorDialer::bootstrap`] has not finished. `progress` is arti's
    /// own 0.0..=1.0 estimate; `blockage` is arti's best guess at what is
    /// stuck, and is `Disabled` when bootstrap has simply not been asked for.
    Bootstrapping {
        progress: f32,
        blockage: Option<BlockageKind>,
    },
    /// Tor is usable; a dial will be attempted.
    Ready,
}

/// Why a target could never be dialed, whatever the network was doing.
///
/// A separate enum rather than four error variants: the recovery is the same
/// (supply a different target), but a caller and a log line still need to know
/// which of the four it was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetProblem {
    /// The host was the empty string.
    EmptyHost,
    /// Longer than the 253 characters a DNS name can have.
    HostTooLong { len: usize },
    /// Port 0 is not a destination; an exit would build a circuit and then
    /// refuse it.
    PortZero,
    /// Not a hostname or IP literal arti will accept.
    NotAHostname,
}

impl std::fmt::Display for TargetProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyHost => write!(f, "the host is empty"),
            Self::HostTooLong { len } => {
                write!(f, "the host is {len} characters; the limit is 253")
            }
            Self::PortZero => write!(f, "port 0 is not a destination"),
            Self::NotAHostname => write!(f, "that is not a hostname or IP address"),
        }
    }
}

/// Every way [`crate::TorDialer::dial`] can decline.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum TorDialError {
    #[error("Tor is not ready yet ({}% bootstrapped){}", (progress * 100.0) as u8,
            match blockage { Some(b) => format!(" — {b}"), None => String::new() })]
    NotBootstrapped {
        progress: f32,
        blockage: Option<BlockageKind>,
    },

    #[error("could not connect to the Tor network{}",
            match blockage { Some(b) => format!(" — {b}"), None => String::new() })]
    BootstrapFailed {
        kind: ErrorKind,
        blockage: Option<BlockageKind>,
    },

    #[error("onion addresses are not supported by this build")]
    OnionUnsupported,

    #[error("that address is on your own machine or local network; Tor will not carry it")]
    ForbiddenTarget,

    #[error("{problem}")]
    BadTarget { problem: TargetProblem },

    #[error("no Tor exit would carry a connection to that port")]
    RefusedByExitPolicy,

    #[error("the far end refused the connection")]
    RefusedByHost,

    #[error("that hostname does not resolve")]
    HostNotFound,

    /// A circuit BUILD timed out inside the Tor network (arti
    /// `TorNetworkTimeout`; tor-circmgr's `CircTimeout` / `RequestTimeout`,
    /// tor-dirclient's timeout). The one timeout a consumer may treat as *the
    /// Tor network is not carrying right now* — which a relay ON THE PATH (the
    /// guard, a middle, an on-path censor) can cause, and to which a stalling
    /// EXIT contributes too: the last hop built is the chosen exit, though arti
    /// retries the build over fresh paths, which blunts any one exit. The
    /// DESTINATION cannot cause it: a stalled stream is
    /// [`Self::ExitTimeout`] / [`Self::RemoteNetworkTimeout`].
    #[error("the connection timed out inside the Tor network")]
    TorNetworkTimeout,

    /// The exit relay reported that it waited too long for the destination
    /// (arti `ExitTimeout`). Produced at the far end — a relay or the
    /// destination can cause it at will, so a consumer must not let it choose
    /// a clearnet fallback (the Tor plugin's design §3.3; the split's reason).
    #[error("the exit waited too long for the destination to answer")]
    ExitTimeout,

    /// Our own timeout expired waiting on the remote side of a stream (arti
    /// `RemoteNetworkTimeout`): an overloaded exit or destination, or a Tor
    /// network problem — "trying later, or on a different circuit, might
    /// help". Destination-influenceable, like [`Self::ExitTimeout`].
    #[error("the destination did not answer in time")]
    RemoteNetworkTimeout,

    /// A network connection of THIS DEVICE failed (arti `LocalNetworkError`).
    /// In arti 0.45.0 its producer is an IO error on an already-open channel
    /// to the guard or bridge (tor-proto's `ChanIoErr`, reaching a dial
    /// through tor-circmgr's `Protocol` wrap): the device's own network, OR
    /// the guard, OR an on-path party resetting the flow after the handshake
    /// — an adversary who already sees this device's address. Never the
    /// destination and never an exit. **NOT "the device has no route":** a
    /// device that is simply offline produces [`Self::TorAccessFailed`]
    /// (tor-chanmgr / tor-guardmgr), never this — so a consumer must not read
    /// this variant as the safe, local-only failure; the SDK's plugin maps it
    /// REFUSED (the Tor plugin's design §3.3) and derives "offline" elsewhere.
    #[error("this device's network connection failed")]
    LocalNetworkError,

    /// Tor could not be reached: the local network, or the chosen relay or
    /// bridge, is not working (arti `TorAccessFailed`). A guard down is
    /// transient; arti's readiness does NOT fall after a success.
    #[error(
        "this device could not reach the Tor network: the local network, or the chosen relay or bridge, is not working"
    )]
    TorAccessFailed,

    #[error("Tor could not set up: {kind}")]
    Setup { kind: ErrorKind },

    #[error("Tor reported: {kind}")]
    Tor { kind: ErrorKind },

    /// This owned client was shut down ([`crate::OwnedDialer::shutdown`]). A
    /// LOCAL, retryable fact that says nothing about any relay or
    /// destination: map it to a not-ready error, never to a refusal that marks
    /// a relay failed and never to a clearnet fallback.
    #[error("the Tor client was shut down")]
    Closed,

    /// [`crate::TorDialer::spawn_owned`] refused: an earlier client's
    /// shutdown overran its budget, so it may still write its state for the
    /// rest of the process and no new client may start; or a shutdown in
    /// flight had not settled within its bound. Local, like [`Self::Closed`].
    /// While the ledger's `stuck()` is zero a later attempt may succeed (that
    /// shutdown may yet settle `Stopped`); once it is not, only a restart ends
    /// it.
    #[error("an earlier Tor client did not finish shutting down")]
    RestartRequired,
}

impl TorDialError {
    /// A short, stable, PII-free label for logs and metrics.
    ///
    /// The `Display` strings above are for humans and may name the target;
    /// device and CI logs are public, so log THIS.
    pub fn class(&self) -> &'static str {
        match self {
            Self::NotBootstrapped { .. } => "not-bootstrapped",
            Self::BootstrapFailed { .. } => "bootstrap-failed",
            Self::OnionUnsupported => "onion-unsupported",
            Self::ForbiddenTarget => "forbidden-target",
            Self::BadTarget { .. } => "bad-target",
            Self::RefusedByExitPolicy => "refused-by-exit-policy",
            Self::RefusedByHost => "refused-by-host",
            Self::HostNotFound => "host-not-found",
            Self::TorNetworkTimeout => "tor-network-timeout",
            Self::ExitTimeout => "exit-timeout",
            Self::RemoteNetworkTimeout => "remote-network-timeout",
            Self::LocalNetworkError => "local-network-error",
            Self::TorAccessFailed => "tor-access-failed",
            Self::Setup { .. } => "setup",
            Self::Tor { .. } => "tor",
            Self::Closed => "closed",
            Self::RestartRequired => "restart-required",
        }
    }
}

/// Map an arti connect failure onto a state a caller can act on.
///
/// The classification itself is [`classify_kind`], a pure function of the
/// `ErrorKind`, so a test can drive every arm without minting an
/// `arti_client::Error` (whose constructors are arti's own).
pub(crate) fn classify_connect(err: &arti_client::Error, readiness: Readiness) -> TorDialError {
    classify_kind(err.kind(), readiness)
}

/// A POSITIVE allowlist over `ErrorKind`: an arm is written only where the
/// recovery differs. Anything unlisted becomes `Tor { kind }`, which still
/// carries arti's own discriminant, so a new upstream kind is reported rather
/// than merged into a neighbour's meaning.
///
/// `readiness` is supplied by the caller so the `BootstrapRequired` arm can
/// say how far along Tor actually is.
///
/// THE SPLIT (H-15; the Tor plugin's design §3.3): the three timeouts and the
/// two network failures are FIVE variants, never two. A consumer that follows
/// a code to a clearnet fallback (the wallet's `Preferred` policy) must
/// reserve it for what only the TOR NETWORK's own silence produces —
/// [`TorDialError::TorNetworkTimeout`] — and refuse every variant a relay,
/// the DESTINATION or an on-path censor can induce: the two remote timeouts
/// [`TorDialError::ExitTimeout`] / [`TorDialError::RemoteNetworkTimeout`],
/// [`TorDialError::TorAccessFailed`] (arti reports it for a plain offline
/// device too) and [`TorDialError::LocalNetworkError`] (the HIGH: a censor
/// dropping relay SYNs produces it, so it is NOT the safe local-only signal).
pub(crate) fn classify_kind(kind: ErrorKind, readiness: Readiness) -> TorDialError {
    match kind {
        ErrorKind::BootstrapRequired => match readiness {
            Readiness::Bootstrapping { progress, blockage } => {
                TorDialError::NotBootstrapped { progress, blockage }
            }
            Readiness::Ready => TorDialError::NotBootstrapped {
                progress: 1.0,
                blockage: None,
            },
        },
        ErrorKind::FeatureDisabled => TorDialError::OnionUnsupported,
        ErrorKind::ForbiddenStreamTarget => TorDialError::ForbiddenTarget,
        ErrorKind::InvalidStreamTarget => TorDialError::BadTarget {
            problem: TargetProblem::NotAHostname,
        },
        ErrorKind::ExitPolicyRejected | ErrorKind::NoExit => TorDialError::RefusedByExitPolicy,
        ErrorKind::RemoteConnectionRefused | ErrorKind::RemoteStreamReset => {
            TorDialError::RefusedByHost
        }
        ErrorKind::RemoteHostNotFound | ErrorKind::RemoteHostResolutionFailed => {
            TorDialError::HostNotFound
        }
        ErrorKind::TorNetworkTimeout => TorDialError::TorNetworkTimeout,
        ErrorKind::ExitTimeout => TorDialError::ExitTimeout,
        ErrorKind::RemoteNetworkTimeout => TorDialError::RemoteNetworkTimeout,
        ErrorKind::LocalNetworkError => TorDialError::LocalNetworkError,
        ErrorKind::TorAccessFailed => TorDialError::TorAccessFailed,
        other => TorDialError::Tor { kind: other },
    }
}

/// Whether a dial that failed with `kind` says the CIRCUIT it ran on should
/// not be reused — the trigger for an isolation group to leave it (FR-54,
/// ADR-0567; the rotation and its bound are `isolation.rs`).
///
/// Read from arti's own [`ErrorKind`], never from [`TorDialError`]: the
/// classified variant has already lost the line this needs
/// ([`TorDialError::RefusedByExitPolicy`] folds the exit's own
/// `ExitPolicyRejected` with the consensus-level `NoExit`, and `RelayTooBusy`
/// lands in the catch-all `Tor { kind }`). A POSITIVE list, so a kind a
/// future arti adds does not rotate until it is placed here.
///
/// Named "suspect", not "exit-attributable": `RemoteNetworkTimeout` is arti's
/// own connect timeout around the stream request, so a stall at the guard or
/// a middle relay produces it too — leaving a stalled circuit is still right.
///
/// **Rotate** — the circuit's EXIT answered (or failed to answer) in a way a
/// different exit may not:
/// - `RemoteHostResolutionFailed` (END `RESOLVEFAILED`), `RemoteHostNotFound`
///   — the exit could not resolve the name (the FR-54 outage's own class);
/// - `ExitTimeout` (END `TIMEOUT`) — the exit gave up waiting on the
///   destination;
/// - `RemoteNetworkTimeout` — our own timeout around the stream request (the
///   outage's other class);
/// - `RemoteNetworkFailed` (END `NOROUTE`) — the exit has no route;
/// - `RemoteStreamError` (END `MISC`, and every END reason tor-cell does not
///   name) — an unexplained refusal at the exit;
/// - `RemoteConnectionRefused` (END `CONNECTREFUSED`), `RemoteStreamReset`
///   (END `CONNRESET`) — reported BY the exit, which may be lying or broken;
/// - `ExitPolicyRejected` (END `EXITPOLICY`) — this exit's own policy; arti
///   keeps handing the group the same circuit regardless;
/// - `RelayTooBusy` (END `RESOURCELIMIT` / `HIBERNATING`) — the exit is
///   overloaded or going to sleep;
/// - `RemoteStreamClosed` (END `DONE` before CONNECTED) — an exit answering a
///   BEGIN that way is misbehaving.
///
/// **Not** (everything else, and by name):
/// - `NoExit` — no exit in the consensus allows the port; a new circuit
///   cannot help;
/// - `CircuitCollapse` (END `DESTROY`) — the circuit is already dead; there
///   is nothing to leave;
/// - `TorProtocolViolation` (END `INTERNAL` / `TORPROTOCOL` / `NOTDIRECTORY`)
///   — may be any hop, so it says nothing about the exit;
/// - every bootstrap, directory, configuration and local kind — not a
///   circuit's fault.
pub(crate) fn circuit_is_suspect(kind: ErrorKind) -> bool {
    matches!(
        kind,
        ErrorKind::RemoteHostResolutionFailed
            | ErrorKind::RemoteHostNotFound
            | ErrorKind::ExitTimeout
            | ErrorKind::RemoteNetworkTimeout
            | ErrorKind::RemoteNetworkFailed
            | ErrorKind::RemoteStreamError
            | ErrorKind::RemoteConnectionRefused
            | ErrorKind::RemoteStreamReset
            | ErrorKind::ExitPolicyRejected
            | ErrorKind::RelayTooBusy
            | ErrorKind::RemoteStreamClosed
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The class each variant must carry, MIRRORED rather than called through:
    /// a call-through passes vacuously, and this match is what makes the
    /// compiler police the set. `#[non_exhaustive]` binds only DOWNSTREAM
    /// crates, so in here a variant added to the enum is a COMPILE error.
    fn expected_class(e: &TorDialError) -> &'static str {
        match e {
            TorDialError::NotBootstrapped { .. } => "not-bootstrapped",
            TorDialError::BootstrapFailed { .. } => "bootstrap-failed",
            TorDialError::OnionUnsupported => "onion-unsupported",
            TorDialError::ForbiddenTarget => "forbidden-target",
            TorDialError::BadTarget { .. } => "bad-target",
            TorDialError::RefusedByExitPolicy => "refused-by-exit-policy",
            TorDialError::RefusedByHost => "refused-by-host",
            TorDialError::HostNotFound => "host-not-found",
            TorDialError::TorNetworkTimeout => "tor-network-timeout",
            TorDialError::ExitTimeout => "exit-timeout",
            TorDialError::RemoteNetworkTimeout => "remote-network-timeout",
            TorDialError::LocalNetworkError => "local-network-error",
            TorDialError::TorAccessFailed => "tor-access-failed",
            TorDialError::Setup { .. } => "setup",
            TorDialError::Tor { .. } => "tor",
            TorDialError::Closed => "closed",
            TorDialError::RestartRequired => "restart-required",
        }
    }

    /// A downstream consumer uses this test as the denominator of the Tor
    /// half of its per-class failure counters, and these STRINGS are the keys
    /// those counters are written under — since the split that set gained five
    /// keys and lost two (`timeout`, `no-network`); the CHANGELOG says so.
    /// (Its own register names this test.)
    ///
    /// It used to end at `assert_eq!(seen.len(), all.len())` — the list
    /// compared with ITSELF, green for a thirteenth variant it never had
    /// and green for a RENAMED label, which silently
    /// re-denominates the counter. [`expected_class`] closes both: an added
    /// variant fails to compile, and a moved label is a red. A variant added
    /// to `expected_class` but not to the list below is caught one crate over
    /// by a consumer's test that reads this enum's DECLARATION (the wallet
    /// SDK's Tor plugin does the same), so the variant set is checked against
    /// the declaration, never against a list's own row count.
    #[test]
    fn every_variant_has_its_own_class() {
        let all = [
            TorDialError::NotBootstrapped {
                progress: 0.0,
                blockage: None,
            },
            TorDialError::BootstrapFailed {
                kind: ErrorKind::Other,
                blockage: None,
            },
            TorDialError::OnionUnsupported,
            TorDialError::ForbiddenTarget,
            TorDialError::BadTarget {
                problem: TargetProblem::PortZero,
            },
            TorDialError::RefusedByExitPolicy,
            TorDialError::RefusedByHost,
            TorDialError::HostNotFound,
            TorDialError::TorNetworkTimeout,
            TorDialError::ExitTimeout,
            TorDialError::RemoteNetworkTimeout,
            TorDialError::LocalNetworkError,
            TorDialError::TorAccessFailed,
            TorDialError::Setup {
                kind: ErrorKind::Other,
            },
            TorDialError::Tor {
                kind: ErrorKind::Other,
            },
            TorDialError::Closed,
            TorDialError::RestartRequired,
        ];
        let mut seen = std::collections::HashSet::new();
        for e in &all {
            assert_eq!(
                e.class(),
                expected_class(e),
                "this label moved, so every counter keyed on it changed denomination \
                 without anything going red"
            );
            assert!(
                seen.insert(e.class()),
                "two variants share the log class {:?}; a log line could not tell them apart",
                e.class()
            );
            assert!(
                !e.to_string().is_empty(),
                "{:?} renders an empty message",
                e.class()
            );
        }
        assert_eq!(seen.len(), all.len());
    }

    /// The owned client's two local classes are published keys too
    /// (`tor-plugin.md` §12.2): a host maps them to its not-ready error by
    /// these strings' variants, and counts them under these strings.
    #[test]
    fn closed_and_restart_required_have_frozen_classes() {
        assert_eq!(TorDialError::Closed.class(), "closed");
        assert_eq!(TorDialError::RestartRequired.class(), "restart-required");
    }

    /// The H-15 split (the Tor plugin's design §3.3 — its crypto angle's HIGH):
    /// each of arti's three timeout kinds and two network kinds reaches ITS OWN
    /// variant, never a shared one. The pairs that must never fold are named
    /// explicitly, because that fold is the whole finding: a consumer reserving
    /// its clearnet-fallback codes for device- or Tor-network failures must be
    /// able to tell `TorNetworkTimeout` from the two the far end can cause, and
    /// `LocalNetworkError` from a failing guard. Driven through
    /// [`classify_kind`] with a `Ready` readiness so the bootstrap arm is not
    /// in play. The variant NAMES are arti's — a rename upstream is a compile
    /// error here, not a silent remap.
    #[test]
    fn every_split_kind_reaches_its_own_variant_and_no_pair_folds() {
        let split = [
            (ErrorKind::TorNetworkTimeout, "tor-network-timeout"),
            (ErrorKind::ExitTimeout, "exit-timeout"),
            (ErrorKind::RemoteNetworkTimeout, "remote-network-timeout"),
            (ErrorKind::LocalNetworkError, "local-network-error"),
            (ErrorKind::TorAccessFailed, "tor-access-failed"),
        ];
        let mut seen = std::collections::HashSet::new();
        for (kind, class) in split {
            let e = classify_kind(kind, Readiness::Ready);
            assert_eq!(e.class(), class, "{kind:?} reached the wrong variant");
            assert!(
                !matches!(e, TorDialError::Tor { .. }),
                "{kind:?} fell through to the unlisted tail"
            );
            assert!(
                seen.insert(e.class()),
                "{kind:?} shares a variant with another split kind"
            );
        }
        // The two folds that must never come back, stated as themselves.
        let tor = classify_kind(ErrorKind::TorNetworkTimeout, Readiness::Ready).class();
        assert_ne!(
            classify_kind(ErrorKind::ExitTimeout, Readiness::Ready).class(),
            tor
        );
        assert_ne!(
            classify_kind(ErrorKind::RemoteNetworkTimeout, Readiness::Ready).class(),
            tor
        );
        assert_ne!(
            classify_kind(ErrorKind::TorAccessFailed, Readiness::Ready).class(),
            classify_kind(ErrorKind::LocalNetworkError, Readiness::Ready).class()
        );
    }

    #[test]
    fn bootstrapping_message_names_the_progress_and_the_blockage() {
        let e = TorDialError::NotBootstrapped {
            progress: 0.42,
            blockage: Some(BlockageKind::Offline),
        };
        let s = e.to_string();
        assert!(s.contains("42%"), "progress missing from {s:?}");
        assert!(
            s.contains(&BlockageKind::Offline.to_string()),
            "blockage missing from {s:?}"
        );
    }

    /// FR-54 §4.4 #2 — the rotation trigger's structural guard (security M1 +
    /// arch MAJOR). Every END reason a relay can send is walked through
    /// tor-cell's OWN `EndReason -> ErrorKind` mapping (the exact-pinned
    /// dev-dependency is the tor-cell arti links), so the table pins arti's
    /// mapping rather than mirroring ours. Each kind the walk produces must sit
    /// on EXACTLY ONE of two explicit lists, and `circuit_is_suspect` must
    /// agree with the list it sits on: a kind a future arti starts producing
    /// fails here until someone places it.
    ///
    /// At 0.45.0 the walk yields eleven kinds (measured at base):
    /// RemoteStreamError, RemoteHostResolutionFailed, RemoteConnectionRefused,
    /// ExitPolicyRejected, CircuitCollapse, RemoteStreamClosed, ExitTimeout,
    /// RemoteNetworkFailed, RelayTooBusy, TorProtocolViolation,
    /// RemoteStreamReset.
    const END_ROTATE: [ErrorKind; 9] = [
        ErrorKind::RemoteStreamError,          // MISC and every unknown code
        ErrorKind::RemoteHostResolutionFailed, // RESOLVEFAILED
        ErrorKind::RemoteConnectionRefused,    // CONNECTREFUSED
        ErrorKind::ExitPolicyRejected,         // EXITPOLICY
        ErrorKind::RemoteStreamClosed,         // DONE before CONNECTED
        ErrorKind::ExitTimeout,                // TIMEOUT
        ErrorKind::RemoteNetworkFailed,        // NOROUTE
        ErrorKind::RelayTooBusy,               // RESOURCELIMIT, HIBERNATING
        ErrorKind::RemoteStreamReset,          // CONNRESET
    ];
    const END_KEEP: [ErrorKind; 2] = [
        ErrorKind::CircuitCollapse,      // DESTROY: the circuit is already dead
        ErrorKind::TorProtocolViolation, // INTERNAL, TORPROTOCOL, NOTDIRECTORY: any hop
    ];

    #[test]
    fn every_end_reason_kind_is_placed_on_exactly_one_rotation_list() {
        use tor_cell::relaycell::msg::EndReason;

        let mut walked: Vec<ErrorKind> = Vec::new();
        for code in 0..=u8::MAX {
            let kind = EndReason::from(code).kind();
            if !walked.contains(&kind) {
                walked.push(kind);
            }
        }
        assert_eq!(
            walked.len(),
            11,
            "tor-cell's END-reason mapping changed shape ({walked:?}); re-place every kind"
        );
        for kind in &walked {
            let on_rotate = END_ROTATE.contains(kind);
            let on_keep = END_KEEP.contains(kind);
            assert!(
                on_rotate ^ on_keep,
                "END kind {kind:?} is on {} rotation lists; it must be on exactly one",
                if on_rotate { "both" } else { "neither of the" }
            );
            assert_eq!(
                circuit_is_suspect(*kind),
                on_rotate,
                "circuit_is_suspect({kind:?}) disagrees with the list it is placed on"
            );
        }
        // No dead rows: every listed END kind is one the walk produced.
        for kind in END_ROTATE.iter().chain(END_KEEP.iter()) {
            assert!(
                walked.contains(kind),
                "{kind:?} is listed as an END kind but no END reason maps to it"
            );
        }
    }

    /// The kinds a failed dial can carry that NO END reason produces, by name.
    #[test]
    fn the_non_end_kinds_are_placed_by_name() {
        for kind in [
            ErrorKind::RemoteNetworkTimeout,
            ErrorKind::RemoteHostNotFound,
        ] {
            assert!(circuit_is_suspect(kind), "{kind:?} must rotate");
        }
        for kind in [
            ErrorKind::NoExit,
            ErrorKind::CircuitCollapse,
            ErrorKind::TorProtocolViolation,
            // bootstrap / directory
            ErrorKind::BootstrapRequired,
            ErrorKind::TorNetworkTimeout,
            ErrorKind::TorAccessFailed,
            ErrorKind::DirectoryExpired,
            ErrorKind::TorDirectoryError,
            ErrorKind::TorDirectoryUnusable,
            ErrorKind::TorDocumentRejected,
            ErrorKind::ClockSkew,
            ErrorKind::CircuitRefused,
            ErrorKind::NoPath,
            ErrorKind::TransientFailure,
            // local
            ErrorKind::LocalNetworkError,
            ErrorKind::LocalProtocolViolation,
            ErrorKind::LocalResourceExhausted,
            ErrorKind::ForbiddenStreamTarget,
            ErrorKind::InvalidStreamTarget,
            ErrorKind::FeatureDisabled,
            ErrorKind::ReactorShuttingDown,
            ErrorKind::ArtiShuttingDown,
            ErrorKind::InvalidConfig,
            ErrorKind::BadApiUsage,
            ErrorKind::Internal,
            ErrorKind::Other,
        ] {
            assert!(
                !circuit_is_suspect(kind),
                "{kind:?} rotated: a new circuit cannot help it, and a rotation costs one"
            );
        }
    }

    #[test]
    fn target_problems_render_distinctly() {
        let msgs = [
            TargetProblem::EmptyHost.to_string(),
            TargetProblem::HostTooLong { len: 400 }.to_string(),
            TargetProblem::PortZero.to_string(),
            TargetProblem::NotAHostname.to_string(),
        ];
        let uniq: std::collections::HashSet<_> = msgs.iter().collect();
        assert_eq!(uniq.len(), msgs.len(), "two target problems read alike");
        assert!(
            msgs[1].contains("400") && msgs[1].contains("253"),
            "the too-long message names neither the length nor the limit: {:?}",
            msgs[1]
        );
    }
}
