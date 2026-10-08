//! The bootstrap retry schedule (`tor-plugin.md` §3.4): `BOOTSTRAP_RETRY_BASE`
//! × 2ⁿ, capped at `BOOTSTRAP_RETRY_CAP`, jittered UP on the plugin's own
//! clock, and a success HALVES n rather than zeroing it — the decay rule, so a
//! network that flaps between working and censored does not reset to the
//! shortest wait every time it briefly works. Pure; the lifecycle owns the
//! loop.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::constants::{BOOTSTRAP_RETRY_BASE, BOOTSTRAP_RETRY_CAP, JITTER_SHARE};

/// The doubling exponent past which the cap always wins (30 s × 2⁵ > 10 min),
/// so the arithmetic never overflows however long a wallet stays censored.
const MAX_EXPONENT: u32 = 16;

/// The retry schedule.
#[derive(Debug, Default)]
pub struct Backoff {
    n: u32,
}

impl Backoff {
    /// The wait before the next attempt, un-jittered.
    pub fn delay(&self) -> Duration {
        BOOTSTRAP_RETRY_BASE
            .saturating_mul(1 << self.n.min(MAX_EXPONENT))
            .min(BOOTSTRAP_RETRY_CAP)
    }

    /// An attempt failed: the next wait doubles.
    pub fn on_failure(&mut self) {
        self.n = (self.n + 1).min(MAX_EXPONENT);
    }

    /// An attempt succeeded: n halves (never zeroes in one step).
    pub fn on_success(&mut self) {
        self.n /= 2;
    }
}

/// How a wait is lengthened. Production draws from the std hasher's random
/// keys (no new crate); tests pass [`NoJitter`].
pub trait Jitter: Send + Sync {
    /// `d` plus at most `d / JITTER_SHARE` — never shorter.
    fn stretch(&self, d: Duration) -> Duration;
}

/// The production jitter: each install and each call draws differently, so two
/// installs never pace together.
#[derive(Default)]
pub struct RandomJitter {
    keys: RandomState,
    calls: AtomicU64,
}

impl Jitter for RandomJitter {
    fn stretch(&self, d: Duration) -> Duration {
        let mut h = self.keys.build_hasher();
        h.write_u64(self.calls.fetch_add(1, Ordering::Relaxed));
        let per_mille = u32::try_from(h.finish() % 1000).unwrap_or(0);
        d + (d / JITTER_SHARE) * per_mille / 1000
    }
}

/// No jitter (tests).
pub struct NoJitter;

impl Jitter for NoJitter {
    fn stretch(&self, d: Duration) -> Duration {
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-5 spec §8 **P10** (the schedule half; the deadline and the loop are
    /// the lifecycle's test): `BASE × 2ⁿ` up to the cap; a success halves n.
    #[test]
    fn the_backoff_doubles_to_its_cap_and_a_success_halves_it() {
        let mut b = Backoff::default();
        assert_eq!(b.delay(), BOOTSTRAP_RETRY_BASE);
        b.on_failure();
        assert_eq!(b.delay(), BOOTSTRAP_RETRY_BASE * 2);
        b.on_failure();
        b.on_failure();
        assert_eq!(b.delay(), BOOTSTRAP_RETRY_BASE * 8);
        b.on_success();
        assert_eq!(
            b.delay(),
            BOOTSTRAP_RETRY_BASE * 2,
            "a success halves the exponent, it does not reset it"
        );
        for _ in 0..40 {
            b.on_failure();
        }
        assert_eq!(b.delay(), BOOTSTRAP_RETRY_CAP);
    }

    /// Jitter only ever lengthens, and by at most one part in `JITTER_SHARE`.
    #[test]
    fn jitter_only_lengthens_and_stays_within_its_share() {
        let j = RandomJitter::default();
        let d = Duration::from_secs(40);
        for _ in 0..200 {
            let s = j.stretch(d);
            assert!(s >= d && s <= d + d / JITTER_SHARE, "{s:?}");
        }
    }
}
