//! What the plugin pushes to the wallet, and when (`tor-plugin.md` §3.4). Pure:
//! the lifecycle owns the loop and the lock, this module owns the numbers and
//! the push rule, so both are unit-tested without a clock.
//!
//! The rule the wallet's header states for every registrant (Relim's
//! H13v3): push on ANY change of the `(health, readiness)` PAIR, never on
//! readiness alone, and rank FAILED below every readiness — or a transport
//! judged FAILED at an unchanged readiness is never announced.

use dialer_tor::Readiness;

use crate::abi::{ZW_HEALTH_FAILED, ZW_HEALTH_READY, ZW_HEALTH_STARTING};
use crate::constants::{
    DOWNWARD_CONFIRMATIONS, READINESS_CEILING_WHILE_BOOTSTRAPPING, READINESS_READY,
};

/// The readiness arti's state maps to (P4): 100 only when arti is ready for
/// traffic; otherwise `floor(progress × 100)` capped at 99, a non-finite or
/// negative progress read as 0. Computed in `f32`, arti's own type — `0.42`
/// widened to `f64` first would floor to 41.
pub fn readiness_percent(readiness: &Readiness) -> u32 {
    match readiness {
        Readiness::Ready => READINESS_READY,
        Readiness::Bootstrapping { progress, .. } => {
            let pct = (progress * 100.0).floor();
            if pct.is_finite() && pct > 0.0 {
                (pct as u32).min(READINESS_CEILING_WHILE_BOOTSTRAPPING)
            } else {
                0
            }
        }
    }
}

/// The transport's health axis, closed (`ZW_HEALTH_*`). Declared in RANK
/// order: `Failed` ranks below every readiness, then starting, then ready.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Health {
    /// The plugin judged the path failed.
    Failed,
    /// Coming up (or paused).
    Starting,
    /// Carrying.
    Ready,
}

impl Health {
    /// The `ZW_HEALTH_*` code.
    pub fn code(self) -> u32 {
        match self {
            Self::Failed => ZW_HEALTH_FAILED,
            Self::Starting => ZW_HEALTH_STARTING,
            Self::Ready => ZW_HEALTH_READY,
        }
    }
}

/// One push: the pair the wallet's descriptor carries.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pushed {
    /// `health`.
    pub health: Health,
    /// `readiness`, 0..=100 — the value MEASURED, never floored for FAILED.
    pub readiness: u32,
}

impl Pushed {
    /// The ranking the push rule compares: FAILED below everything, then the
    /// readiness, then STARTING below READY at the same readiness.
    fn rank(self) -> (bool, u32, Health) {
        (self.health != Health::Failed, self.readiness, self.health)
    }
}

/// The pair a live (not failed, not paused) client asks to push.
pub fn live_pair(readiness: &Readiness) -> Pushed {
    let pct = readiness_percent(readiness);
    Pushed {
        health: if pct == READINESS_READY {
            Health::Ready
        } else {
            Health::Starting
        },
        readiness: pct,
    }
}

/// The watcher's push rule (P5, P5b). A rise is pushed at once (a wallet that
/// can dial must be told now); a drop only after [`DOWNWARD_CONFIRMATIONS`]
/// consecutive reads agree (arti's readiness flaps, and telling the wallet its
/// path died abandons a sync in flight); a drop TO `Failed` is exempt and
/// pushed at once — the confirmation gate damps a flapping NUMBER, while
/// FAILED is a judgement the plugin already made (the wallet stops dialling
/// the private path at once; under `Preferred` it may switch after its
/// patience minute, ADR-0553 v4), so damping it only delays that.
#[derive(Debug, Default)]
pub struct Debounce {
    last: Option<Pushed>,
    drops_seen: u32,
}

impl Debounce {
    /// The pair last pushed, if any.
    pub fn last(&self) -> Option<Pushed> {
        self.last
    }

    /// A push made OUTSIDE the rule (a pause, a failure, a rebuild — the
    /// lifecycle's own transitions): recorded, so the next read compares
    /// against what the wallet actually holds.
    pub fn record(&mut self, pushed: Pushed) {
        self.last = Some(pushed);
        self.drops_seen = 0;
    }

    /// The watcher read `wanted`: `Some` is the pair to push now.
    pub fn observe(&mut self, wanted: Pushed) -> Option<Pushed> {
        let Some(last) = self.last else {
            self.record(wanted);
            return Some(wanted);
        };
        if wanted == last {
            self.drops_seen = 0;
            return None;
        }
        let push = if wanted.rank() > last.rank() || wanted.health == Health::Failed {
            true
        } else {
            self.drops_seen += 1;
            self.drops_seen >= DOWNWARD_CONFIRMATIONS
        };
        if push {
            self.record(wanted);
            Some(wanted)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boot(progress: f32) -> Readiness {
        Readiness::Bootstrapping {
            progress,
            blockage: None,
        }
    }

    /// FR-5 spec §8 **P4**: 100 only when arti says ready for traffic; a
    /// bootstrapping client that reads 1.0 is 99; 0.42 is 42 (the f32 rule);
    /// NaN and a negative progress are 0.
    #[test]
    fn readiness_is_100_only_when_arti_is_ready_for_traffic() {
        assert_eq!(readiness_percent(&Readiness::Ready), 100);
        assert_eq!(readiness_percent(&boot(1.0)), 99);
        assert_eq!(readiness_percent(&boot(0.42)), 42);
        assert_eq!(readiness_percent(&boot(f32::NAN)), 0);
        assert_eq!(readiness_percent(&boot(-0.5)), 0);
        assert_eq!(readiness_percent(&boot(f32::INFINITY)), 0);
        assert_eq!(live_pair(&boot(1.0)).health, Health::Starting);
        assert_eq!(live_pair(&Readiness::Ready).health, Health::Ready);
    }

    fn pair(health: Health, readiness: u32) -> Pushed {
        Pushed { health, readiness }
    }

    /// FR-5 spec §8 **P5** (the rule half; the loop's cadence is in the
    /// lifecycle tests): a rise pushes on the first read, a drop only on the
    /// third agreeing read, and an interrupted drop starts counting again.
    #[test]
    fn a_readiness_rise_is_pushed_at_once_and_a_drop_needs_confirmations() {
        let mut d = Debounce::default();
        assert_eq!(
            d.observe(pair(Health::Starting, 40)),
            Some(pair(Health::Starting, 40))
        );
        assert_eq!(
            d.observe(pair(Health::Starting, 70)),
            Some(pair(Health::Starting, 70))
        );
        assert_eq!(
            d.observe(pair(Health::Ready, 100)),
            Some(pair(Health::Ready, 100))
        );
        // A drop: two reads say 60, then a read says 100 again — nothing.
        assert_eq!(d.observe(pair(Health::Starting, 60)), None);
        assert_eq!(d.observe(pair(Health::Starting, 60)), None);
        assert_eq!(d.observe(pair(Health::Ready, 100)), None);
        // Three agreeing reads push.
        assert_eq!(d.observe(pair(Health::Starting, 60)), None);
        assert_eq!(d.observe(pair(Health::Starting, 60)), None);
        assert_eq!(
            d.observe(pair(Health::Starting, 60)),
            Some(pair(Health::Starting, 60))
        );
        assert_eq!(d.last(), Some(pair(Health::Starting, 60)));
    }

    /// FR-5 plan §2 C3b **P5b** (Relim's H13v3; the quality pass): a
    /// FAILED health at an UNCHANGED readiness is pushed, at once (exempt from
    /// the confirmations); an identical pair is not pushed; and the way back
    /// up from FAILED is a rise, pushed at once.
    #[test]
    fn a_failed_health_at_an_unchanged_readiness_is_pushed_and_an_identical_pair_is_not() {
        let mut d = Debounce::default();
        d.record(pair(Health::Ready, 100));
        assert_eq!(d.observe(pair(Health::Ready, 100)), None);
        assert_eq!(
            d.observe(pair(Health::Failed, 100)),
            Some(pair(Health::Failed, 100)),
            "FAILED at a measured 100 must be announced, on the first read"
        );
        assert_eq!(d.observe(pair(Health::Failed, 100)), None);
        assert_eq!(
            d.observe(pair(Health::Ready, 100)),
            Some(pair(Health::Ready, 100))
        );
        // FAILED ranks below every readiness, 0 included.
        assert!(pair(Health::Failed, 100).rank() < pair(Health::Starting, 0).rank());
    }
}
