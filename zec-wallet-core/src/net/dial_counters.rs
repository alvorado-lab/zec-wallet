//! The wallet's dial tallies by arm × outcome (FR-37; stage S1 `truth`) — the
//! numbers behind `Wallet::dial_counts`, so a host can show how many
//! connections went which way.
//!
//! ONE writer, and it is the attribution line itself: `dialer::log_dial` takes
//! this object and counts what it prints, so the number a host reads and the
//! `wallet.dial` line a device log shows can never disagree — an increment
//! beside the call would be a second site. Per wallet (`Inner.dial_counters`),
//! shared into every dialer the wallet builds the way the posture is; NOT the
//! posture, because a future policy epoch (`set_tor_policy`) must reset the
//! posture's windows while these run "since the wallet opened"; and NOT a
//! process global (FR-39: a second wallet in the same process starts at
//! zero). In memory only, gone at close.
//!
//! PRIVACY, priced (contract §3.2): an always-readable usage signal where the
//! device log is OFF by default. It crosses the bridge only on the host's
//! call, is never logged, and carries counts by arm × outcome and nothing
//! else — no class axis, no host, no port, no key.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::DialError;
use crate::net::tor_posture::DialArm;
use crate::state::{ArmCounts, DialCounts};

/// One arm's eight tallies. `Relaxed` throughout: independent counters with
/// no happens-before to publish; a snapshot is a set of individually-exact
/// counts, not an atomic cut across them.
#[derive(Default)]
struct ArmTally {
    connected: AtomicU64,
    not_ready: AtomicU64,
    retired: AtomicU64,
    unreachable: AtomicU64,
    timeout: AtomicU64,
    unsupported: AtomicU64,
    io: AtomicU64,
    transport_failed: AtomicU64,
}

impl ArmTally {
    /// Exhaustive over `DialError` on purpose (like `log_dial`'s own match): a
    /// new outcome is a compile error here, never a dial that goes uncounted
    /// while its line prints.
    fn record<T>(&self, outcome: &Result<T, DialError>) {
        let slot = match outcome {
            Ok(_) => &self.connected,
            Err(DialError::NotReady) => &self.not_ready,
            Err(DialError::Retired) => &self.retired,
            Err(DialError::Unreachable) => &self.unreachable,
            Err(DialError::Timeout) => &self.timeout,
            Err(DialError::Unsupported) => &self.unsupported,
            Err(DialError::Io(_)) => &self.io,
            Err(DialError::TransportFailed) => &self.transport_failed,
        };
        slot.fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot(&self) -> ArmCounts {
        ArmCounts {
            connected: self.connected.load(Ordering::Relaxed),
            not_ready: self.not_ready.load(Ordering::Relaxed),
            retired: self.retired.load(Ordering::Relaxed),
            unreachable: self.unreachable.load(Ordering::Relaxed),
            timeout: self.timeout.load(Ordering::Relaxed),
            unsupported: self.unsupported.load(Ordering::Relaxed),
            io: self.io.load(Ordering::Relaxed),
            transport_failed: self.transport_failed.load(Ordering::Relaxed),
        }
    }
}

/// The per-wallet counter object — see the module header for who owns it and
/// who writes it.
#[derive(Default)]
pub(crate) struct DialCounters {
    private: ArmTally,
    clearnet: ArmTally,
}

impl DialCounters {
    /// Count one dial outcome against the arm that performed it. Called by
    /// `log_dial` and nobody else.
    pub(crate) fn record<T>(&self, arm: DialArm, outcome: &Result<T, DialError>) {
        match arm {
            DialArm::Private => self.private.record(outcome),
            DialArm::Clearnet => self.clearnet.record(outcome),
        }
    }

    /// The counts as they stand — the DTO `Wallet::dial_counts` hands out.
    pub(crate) fn snapshot(&self) -> DialCounts {
        DialCounts {
            private: self.private.snapshot(),
            clearnet: self.clearnet.snapshot(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every outcome lands in its own field on its own arm, and nowhere else:
    /// the eight `wallet.dial` outcome values are the eight fields, one to one.
    #[test]
    fn each_outcome_counts_once_on_its_own_arm() {
        let counts = DialCounters::default();
        let ok: Result<(), DialError> = Ok(());
        counts.record(DialArm::Private, &ok);
        counts.record(DialArm::Private, &ok);
        counts.record(DialArm::Clearnet, &ok);
        for (arm, err) in [
            (DialArm::Private, DialError::NotReady),
            (DialArm::Private, DialError::Retired),
            (DialArm::Private, DialError::Unreachable),
            (DialArm::Private, DialError::Timeout),
            (DialArm::Private, DialError::Unsupported),
            (DialArm::Private, DialError::TransportFailed),
            (
                DialArm::Clearnet,
                DialError::Io(std::io::Error::other("refused")),
            ),
            (DialArm::Clearnet, DialError::Timeout),
        ] {
            let outcome: Result<(), DialError> = Err(err);
            counts.record(arm, &outcome);
        }
        let snap = counts.snapshot();
        assert_eq!(
            snap.private,
            ArmCounts {
                connected: 2,
                not_ready: 1,
                retired: 1,
                unreachable: 1,
                timeout: 1,
                unsupported: 1,
                io: 0,
                transport_failed: 1,
            }
        );
        assert_eq!(
            snap.clearnet,
            ArmCounts {
                connected: 1,
                not_ready: 0,
                retired: 0,
                unreachable: 0,
                timeout: 1,
                unsupported: 0,
                io: 1,
                transport_failed: 0,
            }
        );
    }

    /// A fresh object reads all zeros — what a second wallet in the same
    /// process sees (FR-39), and what `Default` promises the snapshot DTO.
    #[test]
    fn a_fresh_tally_is_all_zeros() {
        assert_eq!(DialCounters::default().snapshot(), DialCounts::default());
    }
}
