//! `isolation_key` -> Tor circuit isolation group.
//!
//! The contract is the `zec-wallet` SDK's `NetDialer` port: equal keys MAY
//! share a circuit, distinct keys MUST be unlinkable at the network layer.
//!
//! A group's token is also how a group LEAVES a circuit: arti 0.45 keeps
//! handing a circuit whose exit failed to the same isolation group until
//! `max_dirtiness` (600 s), so after a suspect failure the group's token is
//! replaced with a fresh one — bounded per key, with backoff (FR-54,
//! ADR-0567). A fresh token joins no other group: it is MORE isolation,
//! never less.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use arti_client::IsolationToken;

/// How many distinct isolation KEYS one dialer holds before the table is
/// cleared. Not a limit on dials, not on circuits: on live keys.
///
/// Chosen against the party who sets the input — the SDK, which mints one key
/// per transaction, so the count grows with the process lifetime rather than
/// with a burst. 512 keys is ~25 KB of table, and far more circuit groups than
/// a client should ever have open, so the table is bounded well below anything
/// that matters and the ceiling exists to stop unbounded growth, not to shape
/// behaviour.
///
/// Clearing is contract-safe in one direction only, which is why it is the
/// chosen recovery: a key seen before the clear gets a FRESH token, so it
/// stops sharing circuits with its own past. It can never come to share one
/// with a different key, because `IsolationToken::new` is a process-global
/// counter that never repeats.
pub const MAX_ISOLATION_KEYS: usize = 512;

/// The shortest wait between two rotations of one key's token after a
/// suspect dial failure (FR-54, ADR-0567): a key's first rotation is
/// immediate, the next waits this long, and each one after that waits twice
/// the previous wait, up to [`EXIT_ROTATION_INTERVAL_MAX`].
///
/// Why bounded at all: a destination that does not exist, or a hostile exit,
/// produces the same failures at will, so without a bound every failed dial
/// would cost a fresh circuit build at the caller's retry rate.
pub const EXIT_ROTATION_INTERVAL: Duration = Duration::from_secs(60);

/// The longest wait between two rotations of one key's token — arti 0.45's
/// own `max_dirtiness`, after which arti retires the circuit by itself
/// anyway. Under the default policy also the QUIET period: a key's wait falls
/// back to the base only after this long with no suspect failure on it (never
/// on a success — ADR-0567). Under [`ExitRotation::every`] with a longer base,
/// the ceiling AND the quiet period are that base.
pub const EXIT_ROTATION_INTERVAL_MAX: Duration = Duration::from_secs(600);

/// The floor [`ExitRotation::every`] clamps to: arti's own `connect_timeout`
/// (10 s). Below it the bound is gone in all but name (an interval of zero
/// leaves only the compare-and-swap, one circuit build per failed dial).
const EXIT_ROTATION_FLOOR: Duration = Duration::from_secs(10);

/// Whether, and how often, a dialer moves an isolation group off a circuit
/// whose dial failed in a way a fresh circuit can fix (FR-54, ADR-0567).
///
/// A struct with PRIVATE fields, so no host can write an interval under the
/// 10 s floor past the constructors. Set it with
/// [`crate::TorDialer::exit_rotation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExitRotation {
    /// `None` = off: arti's own behaviour, the circuit is kept until
    /// `max_dirtiness`.
    base: Option<Duration>,
    /// The backoff ceiling, and the quiet period after which the wait falls
    /// back to `base`. Never below `base`.
    cap: Duration,
}

impl Default for ExitRotation {
    /// On: [`EXIT_ROTATION_INTERVAL`] (60 s), backing off to
    /// [`EXIT_ROTATION_INTERVAL_MAX`] (600 s).
    fn default() -> Self {
        Self {
            base: Some(EXIT_ROTATION_INTERVAL),
            cap: EXIT_ROTATION_INTERVAL_MAX,
        }
    }
}

impl ExitRotation {
    /// On, with `interval` as the base wait — clamped UP to 10 s (arti's
    /// `connect_timeout`). The backoff ceiling is
    /// [`EXIT_ROTATION_INTERVAL_MAX`], or `interval` itself when that is
    /// longer — and that ceiling is also the quiet period before the wait
    /// decays. `Duration::MAX` is accepted and never overflows (every sum and
    /// product saturates); it means "once per key-generation": a key rotates
    /// once and, never going quiet that long, not again.
    pub fn every(interval: Duration) -> Self {
        let base = interval.max(EXIT_ROTATION_FLOOR);
        Self {
            base: Some(base),
            cap: EXIT_ROTATION_INTERVAL_MAX.max(base),
        }
    }

    /// Off: a group keeps its circuit after any failure, until arti's own
    /// `max_dirtiness` retires it — arti's behaviour, unchanged.
    pub fn off() -> Self {
        Self {
            base: None,
            cap: EXIT_ROTATION_INTERVAL_MAX,
        }
    }

    /// The base wait between two rotations, or `None` when rotation is off.
    pub fn interval(&self) -> Option<Duration> {
        self.base
    }
}

/// One isolation group: its token and the state of its rotation bound.
#[derive(Debug, Clone, Copy)]
struct Group {
    token: IsolationToken,
    /// When the token was last replaced; `None` = never, or the backoff has
    /// decayed — the next rotation is then immediate.
    last_rotated: Option<Instant>,
    /// The wait before the next rotation; `None` = the policy's base.
    /// Stored as the policy-free "not backed off yet" so a group minted by
    /// [`IsolationRegistry::token_for`] needs no policy.
    interval: Option<Duration>,
    /// The last suspect failure seen on this group, rotated or not — what
    /// the quiet period is measured from.
    last_suspect: Option<Instant>,
}

impl Group {
    fn fresh() -> Self {
        Self {
            token: IsolationToken::new(),
            last_rotated: None,
            interval: None,
            last_suspect: None,
        }
    }
}

struct Groups {
    /// The group every `None` dial from THIS dialer joins. Minted per dialer,
    /// not `IsolationToken::no_isolation()`, so two dialers built for two
    /// purposes do not silently share circuits just because both passed
    /// `None`. Behind the same mutex as the keyed table (it can be rotated)
    /// but OUTSIDE it: it occupies no slot and a ceiling clear never touches
    /// it.
    unkeyed: Group,
    keyed: HashMap<String, Group>,
}

pub(crate) struct IsolationRegistry {
    groups: Mutex<Groups>,
}

impl IsolationRegistry {
    pub(crate) fn new() -> Self {
        Self {
            groups: Mutex::new(Groups {
                unkeyed: Group::fresh(),
                keyed: HashMap::new(),
            }),
        }
    }

    /// A poisoned table is recovered rather than propagated: the worst a
    /// half-written table can do is lose an entry or a rotation, and losing
    /// an entry mints a fresh token — more isolation, never less.
    fn lock(&self) -> std::sync::MutexGuard<'_, Groups> {
        self.groups.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(crate) fn token_for(&self, key: Option<&str>) -> IsolationToken {
        let mut groups = self.lock();
        let Some(key) = key else {
            return groups.unkeyed.token;
        };
        if let Some(group) = groups.keyed.get(key) {
            return group.token;
        }
        if groups.keyed.len() >= MAX_ISOLATION_KEYS {
            groups.keyed.clear();
        }
        let group = Group::fresh();
        groups.keyed.insert(key.to_owned(), group);
        group.token
    }

    /// After a SUSPECT dial failure (`crate::error::circuit_is_suspect`),
    /// replace the group's token with a fresh one so the next dial on `key`
    /// cannot join the circuit that failed. Returns whether it rotated.
    ///
    /// Rotates only when ALL hold:
    /// - `policy` is not [`ExitRotation::off`];
    /// - the group EXISTS — a key cleared at the ceiling between the dial and
    ///   its failure returns `false` and is NOT re-inserted (an insert here
    ///   would bypass the ceiling);
    /// - the stored token is still `used`, the token the failed dial ran on
    ///   (compare-and-swap: N dials failing on one circuit rotate once, and a
    ///   late failure from an old circuit never discards a fresh one);
    /// - there is no previous rotation, or `now.saturating_duration_since(last)`
    ///   has reached the group's wait — never `last + interval`, which
    ///   overflows `Instant` for a large interval.
    ///
    /// On a rotation after the first, the wait doubles, capped. The wait falls
    /// back to the base only after the policy's cap passes with no suspect
    /// failure on the group — a success does not reset it: on a group that
    /// carries several destinations a dead one and a live one alternate, and
    /// a success on the fresh circuit says the failure was the destination's.
    ///
    /// `now` is injected; the registry has no clock of its own.
    pub(crate) fn rotate_after_exit_failure(
        &self,
        key: Option<&str>,
        used: IsolationToken,
        now: Instant,
        policy: ExitRotation,
    ) -> bool {
        let Some(base) = policy.base else {
            return false;
        };
        let mut groups = self.lock();
        let group = match key {
            None => &mut groups.unkeyed,
            Some(key) => match groups.keyed.get_mut(key) {
                Some(group) => group,
                None => return false,
            },
        };

        // The quiet-period decay, measured from the last suspect failure
        // BEFORE this one is recorded.
        if let Some(last) = group.last_suspect
            && now.saturating_duration_since(last) >= policy.cap
        {
            group.interval = None;
            group.last_rotated = None;
        }
        group.last_suspect = Some(group.last_suspect.map_or(now, |last| last.max(now)));

        if group.token != used {
            return false;
        }
        // Clamped into THIS policy's [base, cap], so a wait stored under an
        // earlier policy can neither undercut the floor nor exceed the cap.
        let wait = group.interval.unwrap_or(base).max(base).min(policy.cap);
        if let Some(last) = group.last_rotated
            && now.saturating_duration_since(last) < wait
        {
            return false;
        }

        group.token = IsolationToken::new();
        group.interval = Some(match group.last_rotated {
            None => base,
            Some(_) => wait.saturating_mul(2).min(policy.cap),
        });
        group.last_rotated = Some(now);
        true
    }

    #[cfg(test)]
    pub(crate) fn live_keys(&self) -> usize {
        self.lock().keyed.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_keys_share_a_group_and_distinct_keys_do_not() {
        let reg = IsolationRegistry::new();
        let a1 = reg.token_for(Some("tx-a"));
        let a2 = reg.token_for(Some("tx-a"));
        let b = reg.token_for(Some("tx-b"));
        assert_eq!(a1, a2, "the same key must reach the same circuit group");
        assert_ne!(
            a1, b,
            "two keys landed in ONE group; distinct keys must be unlinkable"
        );
        assert_eq!(reg.live_keys(), 2);
    }

    #[test]
    fn unkeyed_dials_share_one_group_that_no_key_can_join() {
        let reg = IsolationRegistry::new();
        let n1 = reg.token_for(None);
        let n2 = reg.token_for(None);
        assert_eq!(n1, n2, "two unkeyed dials must reach the same group");
        assert_ne!(
            n1,
            reg.token_for(Some("tx-a")),
            "a keyed dial joined the unkeyed group"
        );
        assert_eq!(
            reg.live_keys(),
            1,
            "the unkeyed group must not occupy a slot in the keyed table"
        );
    }

    #[test]
    fn two_dialers_do_not_share_an_unkeyed_group() {
        let a = IsolationRegistry::new();
        let b = IsolationRegistry::new();
        assert_ne!(
            a.token_for(None),
            b.token_for(None),
            "two dialers shared one unkeyed circuit group"
        );
    }

    #[test]
    fn the_table_is_cleared_at_the_ceiling_and_never_merges_two_keys() {
        let reg = IsolationRegistry::new();
        let first = reg.token_for(Some("k0"));
        for i in 1..MAX_ISOLATION_KEYS {
            reg.token_for(Some(&format!("k{i}")));
        }
        assert_eq!(
            reg.live_keys(),
            MAX_ISOLATION_KEYS,
            "the ceiling fired early: the table must hold MAX keys before clearing"
        );

        let overflow = reg.token_for(Some("k-overflow"));
        assert_eq!(
            reg.live_keys(),
            1,
            "at the ceiling the table must be cleared, leaving only the new key"
        );

        let first_again = reg.token_for(Some("k0"));
        assert_ne!(
            first, first_again,
            "a cleared key kept its old token; the clear did nothing"
        );
        assert_ne!(
            first_again, overflow,
            "two distinct keys were merged into one group by the clear"
        );
        assert_ne!(
            first, overflow,
            "a re-minted token collided with a live one"
        );
    }

    #[test]
    fn a_long_run_of_distinct_keys_never_repeats_a_token() {
        let reg = IsolationRegistry::new();
        let mut seen = std::collections::BTreeSet::new();
        for i in 0..(MAX_ISOLATION_KEYS * 2 + 3) {
            let t = reg.token_for(Some(&format!("k{i}")));
            assert!(
                seen.insert(t),
                "token repeated at key {i}: two unrelated dials would share a circuit"
            );
        }
        assert!(reg.live_keys() <= MAX_ISOLATION_KEYS);
    }
}

/// FR-54 §4.4 #1 — the swap, its bound, its backoff and the policy switch,
/// driven with an INJECTED clock (`now`), so no row sleeps. Written to the
/// design's contract (`docs/plan/fr54-a-bad-exit-does-not-hold-the-private-path.md`
/// §4.1, §4.3) before the implementation existed.
///
/// The schedule these rows pin, read from §4.1 + §4.4 #1: the FIRST rotation of
/// a key is immediate; the gap after the n-th rotation is
/// `EXIT_ROTATION_INTERVAL * 2^(n-1)`, capped at `EXIT_ROTATION_INTERVAL_MAX`
/// (60, 120, 240, 480, 600, 600 …); the gap falls back to 60 s only when
/// `EXIT_ROTATION_INTERVAL_MAX` has passed with NO suspect failure on the key
/// (a refused failure counts as a failure — it is one).
#[cfg(test)]
mod exit_rotation_tests {
    use std::time::{Duration, Instant};

    use arti_client::ErrorKind;

    use super::*;
    use crate::{EXIT_ROTATION_INTERVAL, EXIT_ROTATION_INTERVAL_MAX, ExitRotation};

    const EPS: Duration = Duration::from_millis(1);

    fn secs(s: u64) -> Duration {
        Duration::from_secs(s)
    }

    /// A suspect failure on `key`, reported with the token the key holds NOW
    /// (the dial that failed used the current token). Uses the registry's
    /// default schedule.
    fn fail(reg: &IsolationRegistry, key: Option<&str>, now: Instant) -> bool {
        let used = reg.token_for(key);
        reg.rotate_after_exit_failure(key, used, now, ExitRotation::default())
    }

    /// The same failure through `connect`'s failure arm, under a policy.
    fn fail_under(
        reg: &IsolationRegistry,
        key: Option<&str>,
        now: Instant,
        policy: ExitRotation,
    ) -> bool {
        let used = reg.token_for(key);
        crate::dialer::after_dial_failure(
            reg,
            key,
            used,
            ErrorKind::RemoteHostResolutionFailed,
            now,
            policy,
        )
    }

    /// Fill the keyed table until the ceiling clear fires, leaving exactly
    /// one live key (the one whose insert triggered the clear).
    fn force_ceiling_clear(reg: &IsolationRegistry) {
        for i in 0..=MAX_ISOLATION_KEYS {
            reg.token_for(Some(&format!("filler-{i}")));
        }
        assert!(
            reg.live_keys() < MAX_ISOLATION_KEYS,
            "the fill did not reach the ceiling clear; the row below proves nothing"
        );
    }

    #[test]
    fn the_rotation_constants_are_one_and_ten_minutes() {
        assert_eq!(EXIT_ROTATION_INTERVAL, secs(60));
        assert_eq!(
            EXIT_ROTATION_INTERVAL_MAX,
            secs(600),
            "the cap is arti's max_dirtiness; past it arti retires the circuit itself"
        );
    }

    #[test]
    fn a_first_exit_failure_rotates_the_keys_token() {
        let reg = IsolationRegistry::new();
        let before = reg.token_for(Some("k"));
        assert!(
            fail(&reg, Some("k"), Instant::now()),
            "the first failure must rotate"
        );
        assert_ne!(
            reg.token_for(Some("k")),
            before,
            "rotate reported true but the key still holds the failed circuit's token"
        );
    }

    #[test]
    fn the_second_rotation_waits_the_interval_to_its_boundary() {
        let reg = IsolationRegistry::new();
        let t0 = Instant::now();
        assert!(fail(&reg, Some("k"), t0));
        let held = reg.token_for(Some("k"));

        assert!(
            !fail(&reg, Some("k"), t0 + EXIT_ROTATION_INTERVAL - EPS),
            "rotated before the interval elapsed — one circuit per failed dial"
        );
        assert_eq!(
            reg.token_for(Some("k")),
            held,
            "a refused rotation moved the token"
        );

        assert!(
            fail(&reg, Some("k"), t0 + EXIT_ROTATION_INTERVAL),
            "the bound is `>=`: at exactly the interval the key must rotate"
        );
        assert_ne!(reg.token_for(Some("k")), held);
    }

    #[test]
    fn a_stale_used_token_never_rotates() {
        let reg = IsolationRegistry::new();
        let t0 = Instant::now();
        // Two dials raced on token T; the first failure rotates T away.
        let t = reg.token_for(Some("k"));
        assert!(reg.rotate_after_exit_failure(Some("k"), t, t0, ExitRotation::default()));
        let fresh = reg.token_for(Some("k"));
        // The second failure, still carrying T, arrives long after the bound:
        // the CAS alone must refuse it.
        assert!(
            !reg.rotate_after_exit_failure(Some("k"), t, t0 + secs(3600), ExitRotation::default()),
            "a stale token rotated the key — the compare-and-swap is gone"
        );
        assert_eq!(
            reg.token_for(Some("k")),
            fresh,
            "a stale failure moved the token"
        );
    }

    #[test]
    fn a_stale_used_token_never_rotates_the_unkeyed_group() {
        let reg = IsolationRegistry::new();
        let t0 = Instant::now();
        let t = reg.token_for(None);
        assert!(reg.rotate_after_exit_failure(None, t, t0, ExitRotation::default()));
        let fresh = reg.token_for(None);
        assert!(!reg.rotate_after_exit_failure(None, t, t0 + secs(3600), ExitRotation::default()));
        assert_eq!(reg.token_for(None), fresh);
    }

    #[test]
    fn an_absent_key_is_not_rotated_and_not_inserted() {
        let reg = IsolationRegistry::new();
        assert!(
            !reg.rotate_after_exit_failure(
                Some("ghost"),
                IsolationToken::new(),
                Instant::now(),
                ExitRotation::default()
            ),
            "a key never dialed was rotated"
        );
        assert_eq!(
            reg.live_keys(),
            0,
            "rotate inserted a key — an insert that bypasses the ceiling (m3)"
        );
    }

    #[test]
    fn a_key_cleared_between_the_dial_and_its_failure_is_not_re_inserted() {
        let reg = IsolationRegistry::new();
        let used = reg.token_for(Some("k"));
        force_ceiling_clear(&reg);
        let live = reg.live_keys();
        assert!(
            !reg.rotate_after_exit_failure(
                Some("k"),
                used,
                Instant::now(),
                ExitRotation::default()
            ),
            "a key the ceiling cleared was rotated"
        );
        assert_eq!(
            reg.live_keys(),
            live,
            "rotate re-inserted a cleared key (m3)"
        );
    }

    #[test]
    fn the_unkeyed_group_rotates_and_never_lands_on_a_keyed_token() {
        let reg = IsolationRegistry::new();
        let a = reg.token_for(Some("a"));
        let b = reg.token_for(Some("b"));
        let before = reg.token_for(None);
        assert!(
            fail(&reg, None, Instant::now()),
            "the unkeyed group must rotate"
        );
        let after = reg.token_for(None);
        assert_ne!(after, before);
        assert_eq!(
            reg.token_for(None),
            after,
            "the rotated unkeyed token is not stable"
        );
        for keyed in [a, b, reg.token_for(Some("c"))] {
            assert_ne!(after, keyed, "the unkeyed group joined a keyed one");
        }
        assert_eq!(
            reg.live_keys(),
            3,
            "the unkeyed rotation occupies a keyed slot; it must live outside the map"
        );
    }

    #[test]
    fn the_unkeyed_rotation_and_its_bound_survive_a_ceiling_clear() {
        let reg = IsolationRegistry::new();
        let t0 = Instant::now();
        assert!(fail(&reg, None, t0));
        let rotated = reg.token_for(None);
        force_ceiling_clear(&reg);
        assert_eq!(
            reg.token_for(None),
            rotated,
            "a ceiling clear reset the unkeyed group"
        );
        assert!(
            !fail(&reg, None, t0 + secs(1)),
            "a ceiling clear dropped the unkeyed group's rotation state"
        );
    }

    #[test]
    fn rotating_one_key_leaves_every_other_group_alone() {
        let reg = IsolationRegistry::new();
        let other = reg.token_for(Some("other"));
        let unkeyed = reg.token_for(None);
        assert!(fail(&reg, Some("k"), Instant::now()));
        assert_eq!(
            reg.token_for(Some("other")),
            other,
            "a neighbouring key moved"
        );
        assert_eq!(reg.token_for(None), unkeyed, "the unkeyed group moved");
        // …and the neighbour's own first failure is still immediate: the bound
        // is per key, not per dialer.
        assert!(fail(&reg, Some("other"), Instant::now()));
    }

    #[test]
    fn a_ceiling_clear_drops_a_keys_rotation_state_for_at_most_one_extra_circuit() {
        let reg = IsolationRegistry::new();
        let t0 = Instant::now();
        assert!(fail(&reg, Some("k"), t0));
        force_ceiling_clear(&reg);
        assert!(
            fail(&reg, Some("k"), t0 + secs(1)),
            "the cleared key kept its rotation state (m2 says the clear drops it)"
        );
        assert!(
            !fail(&reg, Some("k"), t0 + secs(2)),
            "after the one extra rotation the key must be bounded again"
        );
    }

    #[test]
    fn the_interval_doubles_per_rotation_and_caps_at_the_maximum() {
        let reg = IsolationRegistry::new();
        let mut last = Instant::now();
        assert!(fail(&reg, Some("k"), last));
        for gap in [60, 120, 240, 480, 600, 600, 600] {
            let gap = secs(gap);
            assert!(
                !fail(&reg, Some("k"), last + gap - EPS),
                "rotated {:?} early on the {gap:?} step of the backoff",
                EPS
            );
            assert!(
                fail(&reg, Some("k"), last + gap),
                "did not rotate after the {gap:?} step of the backoff elapsed"
            );
            last += gap;
        }
    }

    #[test]
    fn the_interval_falls_back_after_a_quiet_maximum() {
        let reg = IsolationRegistry::new();
        let t0 = Instant::now();
        assert!(fail(&reg, Some("k"), t0));
        assert!(fail(&reg, Some("k"), t0 + secs(60))); // next gap would be 120
        let t1 = t0 + secs(60) + EXIT_ROTATION_INTERVAL_MAX;
        assert!(fail(&reg, Some("k"), t1));
        assert!(
            fail(&reg, Some("k"), t1 + EXIT_ROTATION_INTERVAL),
            "after a quiet EXIT_ROTATION_INTERVAL_MAX the gap must be back to 60 s"
        );
    }

    #[test]
    fn the_interval_does_not_fall_back_one_tick_before_the_quiet_maximum() {
        let reg = IsolationRegistry::new();
        let t0 = Instant::now();
        assert!(fail(&reg, Some("k"), t0));
        assert!(fail(&reg, Some("k"), t0 + secs(60))); // next gap 120
        let t1 = t0 + secs(60) + EXIT_ROTATION_INTERVAL_MAX - EPS;
        assert!(fail(&reg, Some("k"), t1)); // third rotation: next gap 240
        assert!(
            !fail(&reg, Some("k"), t1 + EXIT_ROTATION_INTERVAL),
            "the backoff decayed before EXIT_ROTATION_INTERVAL_MAX of quiet"
        );
    }

    #[test]
    fn a_refused_suspect_failure_restarts_the_quiet_period() {
        let reg = IsolationRegistry::new();
        let t0 = Instant::now();
        assert!(fail(&reg, Some("k"), t0));
        assert!(fail(&reg, Some("k"), t0 + secs(60))); // next gap 120
        // A failure inside the bound: refused, but it IS a suspect failure.
        assert!(!fail(&reg, Some("k"), t0 + secs(100)));
        // 600 s after the last ROTATION, but only 560 s after the last FAILURE.
        let t1 = t0 + secs(660);
        assert!(fail(&reg, Some("k"), t1)); // third rotation: next gap 240
        assert!(
            !fail(&reg, Some("k"), t1 + EXIT_ROTATION_INTERVAL),
            "the quiet period was measured from the last rotation, not the last \
             suspect failure"
        );
    }

    /// The re-check's row: on Relim's unkeyed lane a dead destination and a
    /// live one alternate, so successes sit between the failures. A success
    /// is a dial that took the key's token and returned a stream — the
    /// registry sees only `token_for` — and it must not reset the backoff.
    #[test]
    fn successes_alternating_with_failures_still_grow_the_backoff() {
        let reg = IsolationRegistry::new();
        let t0 = Instant::now();
        let success = |reg: &IsolationRegistry| {
            reg.token_for(None);
        };
        assert!(fail(&reg, None, t0));
        success(&reg);
        assert!(fail(&reg, None, t0 + secs(60)));
        success(&reg);
        assert!(
            !fail(&reg, None, t0 + secs(120)),
            "a success between failures reset the backoff to 60 s"
        );
        success(&reg);
        assert!(fail(&reg, None, t0 + secs(180)));
        success(&reg);
        assert!(
            !fail(&reg, None, t0 + secs(180) + secs(239)),
            "the third gap must be 240 s despite the successes"
        );
        assert!(fail(&reg, None, t0 + secs(420)));
    }

    #[test]
    fn rotation_off_never_rotates() {
        let reg = IsolationRegistry::new();
        let t0 = Instant::now();
        for key in [None, Some("k")] {
            let held = reg.token_for(key);
            for later in [secs(0), secs(60), secs(600), secs(86_400)] {
                assert!(
                    !fail_under(&reg, key, t0 + later, ExitRotation::off()),
                    "ExitRotation::off() rotated (key {:?}, +{later:?})",
                    key.is_some()
                );
            }
            assert_eq!(reg.token_for(key), held, "off() moved a token");
        }
    }

    #[test]
    fn the_default_policy_is_the_sixty_second_bound() {
        let reg = IsolationRegistry::new();
        let t0 = Instant::now();
        let p = ExitRotation::default();
        assert!(fail_under(&reg, Some("k"), t0, p));
        assert!(!fail_under(
            &reg,
            Some("k"),
            t0 + EXIT_ROTATION_INTERVAL - EPS,
            p
        ));
        assert!(fail_under(&reg, Some("k"), t0 + EXIT_ROTATION_INTERVAL, p));
    }

    /// Security M2: no public constructor yields an interval under arti's
    /// 10 s `connect_timeout` — `every(ZERO)` would leave only the CAS, one
    /// circuit build per failed dial at the caller's retry rate.
    #[test]
    fn every_clamps_any_interval_under_ten_seconds_to_ten() {
        const FLOOR: Duration = Duration::from_secs(10);
        for asked in [
            Duration::ZERO,
            Duration::from_nanos(1),
            Duration::from_millis(1),
            FLOOR - EPS,
        ] {
            let reg = IsolationRegistry::new();
            let p = ExitRotation::every(asked);
            let t0 = Instant::now();
            assert!(fail_under(&reg, Some("k"), t0, p));
            assert!(
                !fail_under(&reg, Some("k"), t0 + FLOOR - EPS, p),
                "every({asked:?}) rotated under the 10 s floor"
            );
            assert!(
                fail_under(&reg, Some("k"), t0 + FLOOR, p),
                "every({asked:?}) did not rotate at the 10 s floor"
            );
        }
    }

    #[test]
    fn every_honours_an_interval_above_the_floor() {
        let reg = IsolationRegistry::new();
        let p = ExitRotation::every(secs(30));
        let t0 = Instant::now();
        assert!(fail_under(&reg, Some("k"), t0, p));
        assert!(!fail_under(&reg, Some("k"), t0 + secs(30) - EPS, p));
        assert!(fail_under(&reg, Some("k"), t0 + secs(30), p));
    }

    /// Security M2: `last + interval` overflows `Instant`; the contract is
    /// `now.saturating_duration_since(last) >= interval`.
    #[test]
    fn every_duration_max_never_panics() {
        let reg = IsolationRegistry::new();
        let p = ExitRotation::every(Duration::MAX);
        let t0 = Instant::now();
        assert!(
            fail_under(&reg, Some("k"), t0, p),
            "the first failure still rotates"
        );
        // Inside the quiet window (each failure < EXIT_ROTATION_INTERVAL_MAX
        // after the last) the huge interval holds.
        assert!(!fail_under(&reg, Some("k"), t0 + secs(1), p));
        assert!(!fail_under(&reg, Some("k"), t0 + secs(599), p));
        // A `now` EARLIER than the last failure (a clock the caller got
        // wrong) must not panic either: saturating, never subtracting.
        assert!(!fail_under(&reg, Some("k"), t0, p));
        // Far later: under `every(Duration::MAX)` the quiet period is also
        // `Duration::MAX`, so the key never decays and never rotates again;
        // the row only demands that nothing overflows on the way.
        for later in [secs(86_400), secs(10 * 365 * 86_400)] {
            if let Some(now) = t0.checked_add(later) {
                let _ = fail_under(&reg, Some("k"), now, p);
            }
        }
        let reg = IsolationRegistry::new();
        assert!(fail(&reg, Some("k"), t0 + secs(5)));
        assert!(!fail(&reg, Some("k"), t0));
    }
}
