//! The dialer itself.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use arti_client::config::TorClientConfigBuilder;
use arti_client::{
    BootstrapBehavior, DormantMode, ErrorKind, HasKind as _, IsolationToken, StreamPrefs,
    TorClient, TorClientConfig,
};
use tor_rtcompat::PreferredRuntime;
use tracing::{debug, warn};

use crate::AsyncByteStream;
use crate::bridges::BridgeLines;
use crate::error::{Readiness, TargetProblem, TorDialError, circuit_is_suspect, classify_connect};
use crate::isolation::{ExitRotation, IsolationRegistry};
use crate::stream::TorStream;

/// The longest a DNS name can be, in characters. Frozen by RFC 1035/1123, and
/// the same number arti's own validator uses — so this bound decides which
/// error a caller SEES, not whether an over-long host is refused.
const MAX_HOST_LEN: usize = 253;

const ONION_SUFFIX: &[u8] = b".onion";

/// The arti configuration a bridged dialer runs on, exposed rather than kept
/// inside [`TorDialer::with_bridges`].
///
/// A caller — and this crate's own tests — must be able to assert that a pasted
/// bridge line reached a BUILT configuration and not merely a successful parse:
/// a config that holds nothing is the first falsifier
/// the first consumer's design states, and a parse assertion cannot see it.
/// [`TorDialer::with_bridges`] is this plus [`TorDialer::from_config`], so there
/// is one config-building path and the no-bridge case cannot drift from the
/// bridged one.
///
/// # Errors
///
/// [`TorDialError::Setup`] when arti refuses the assembled configuration.
pub fn tor_config(
    state_dir: impl AsRef<Path>,
    cache_dir: impl AsRef<Path>,
    bridges: BridgeLines,
) -> Result<TorClientConfig, TorDialError> {
    let mut builder = TorClientConfigBuilder::from_directories(state_dir, cache_dir);
    for bridge in bridges.into_parsed() {
        builder.bridges().bridges().push(bridge);
    }
    builder.build().map_err(|_e| {
        // The KIND, never the error's `Display`: this is the path a pasted
        // bridge line travels, and arti's config-build error is the one
        // `Display` in this crate that could carry pasted text (today its
        // arms interpolate only literals and field names; the arms that would
        // echo a setting are behind `pt-client`, off here — the security
        // angle on FR-5 C0). `bridges.rs` keeps the line out of every
        // refusal class; this exit keeps it out of the log.
        warn!(kind = %ErrorKind::InvalidConfig, "tor client configuration is not usable");
        TorDialError::Setup {
            kind: ErrorKind::InvalidConfig,
        }
    })
}

/// Dials `host:port` through Tor, and has no other way to reach the network.
pub struct TorDialer {
    client: Arc<TorClient<PreferredRuntime>>,
    isolation: IsolationRegistry,
    /// Whether a suspect dial failure moves its isolation group off the
    /// circuit ([`Self::exit_rotation`]); on by default.
    exit_rotation: ExitRotation,
    /// The mode last handed to [`Self::set_dormant`] — a QUERY for a consumer
    /// that must report what it asked for (arti keeps no getter), never a
    /// substitute for arti's own state. A `std` mutex: never held across an
    /// `.await`, and the critical section is one `Copy`.
    dormant: Mutex<DormantMode>,
}

impl TorDialer {
    /// Build a dialer over a state and a cache directory, WITHOUT bootstrapping.
    ///
    /// No network traffic happens here. Until [`Self::bootstrap`] succeeds,
    /// every dial returns [`TorDialError::NotBootstrapped`].
    // reachability-check: owed(#762)
    pub async fn unbootstrapped(
        state_dir: impl AsRef<Path>,
        cache_dir: impl AsRef<Path>,
    ) -> Result<Self, TorDialError> {
        Self::with_bridges(state_dir, cache_dir, BridgeLines::default()).await
    }

    /// Build a dialer that bootstraps THROUGH the given bridges, WITHOUT
    /// bootstrapping. Same refusal contract as [`Self::unbootstrapped`], which
    /// is this verb with no bridges — one config-building path, so the
    /// no-bridge case cannot drift from the bridged one.
    ///
    /// An EMPTY [`BridgeLines`] leaves arti's own directory bootstrap exactly as
    /// it is: `bridges.enabled` stays `Auto`, which upstream reads as *use
    /// bridges iff any are configured*.
    pub async fn with_bridges(
        state_dir: impl AsRef<Path>,
        cache_dir: impl AsRef<Path>,
        bridges: BridgeLines,
    ) -> Result<Self, TorDialError> {
        Self::from_config(tor_config(state_dir, cache_dir, bridges)?).await
    }

    /// Build a dialer over a caller-supplied arti configuration, WITHOUT
    /// bootstrapping. Same refusal contract as [`Self::unbootstrapped`].
    pub async fn from_config(config: TorClientConfig) -> Result<Self, TorDialError> {
        ensure_crypto_provider();
        let client = TorClient::builder()
            .config(config)
            .bootstrap_behavior(BootstrapBehavior::Manual)
            .create_unbootstrapped_async()
            .await
            .map_err(|e| {
                warn!(kind = %e.kind(), "could not create the Tor client");
                TorDialError::Setup { kind: e.kind() }
            })?;
        Ok(Self {
            client,
            isolation: IsolationRegistry::new(),
            exit_rotation: ExitRotation::default(),
            dormant: Mutex::new(DormantMode::Normal),
        })
    }

    /// Set whether, and how often, an isolation group leaves a circuit whose
    /// dial failed at or near the exit (FR-54, ADR-0567). Consuming, builder
    /// style: `TorDialer::from_config(c).await?.exit_rotation(ExitRotation::off())`.
    ///
    /// The default ([`ExitRotation::default`]) is ON: after a suspect failure
    /// the key's next dial gets a fresh isolation token — at most once per
    /// [`crate::EXIT_ROTATION_INTERVAL`] per key, backing off to
    /// [`crate::EXIT_ROTATION_INTERVAL_MAX`] — so it cannot join the circuit
    /// that failed. [`ExitRotation::off`] keeps arti's own behaviour: the
    /// circuit is reused until arti's `max_dirtiness` (600 s).
    pub fn exit_rotation(mut self, rotation: ExitRotation) -> Self {
        self.exit_rotation = rotation;
        self
    }

    /// Hand arti a dormant mode — a passthrough to `TorClient::set_dormant`
    /// (H-15; the Tor plugin's design, §3.6).
    ///
    /// `Soft` suspends the client's periodic background work (directory
    /// refreshes, guard maintenance; arti: "attempts to use the client will
    /// wake it back up again"); `Normal` resumes it. What it does NOT do is
    /// close channels or sockets, so a consumer that needs *no dials while
    /// paused* gates that itself (the SDK's Tor plugin pushes readiness 0
    /// on pause — spec §3.4): dormancy is the battery half, not the
    /// correctness half. The mode is also RECORDED so [`Self::dormant_mode`]
    /// can answer; arti exposes no getter. The record lock is held AROUND the
    /// passthrough (both are synchronous; no await), so two callers cannot
    /// interleave into "arti `Normal`, the record `Soft`" — an indicator that
    /// says stopped while directory refreshes continue (the security angle on
    /// FR-5 C0).
    ///
    /// Two facts about arti 0.45.0 a consumer must carry (the crypto angle on
    /// FR-5 C0): (1) the record is what was ASKED, not arti's live state —
    /// arti flips `Soft` back to `Normal` by itself on ANY use of the client
    /// (`arti-client/src/client.rs:2414-2424`), so a consumer that dials, or
    /// even queries readiness, while "dormant" has silently woken it; "no
    /// background traffic from a backgrounded wallet" holds only while nothing
    /// touches the client. (2) `Soft` also switches channel PADDING off
    /// (`tor-chanmgr/src/mgr/state.rs:808-811`), which the guard can observe —
    /// dormancy is visible at the network edge, not only on the battery.
    pub fn set_dormant(&self, mode: DormantMode) {
        let mut recorded = self.dormant.lock().unwrap_or_else(|e| e.into_inner());
        self.client.set_dormant(mode);
        *recorded = mode;
        debug!(?mode, "tor client dormancy set");
    }

    /// The mode last handed to [`Self::set_dormant`]; `Normal` at creation.
    pub fn dormant_mode(&self) -> DormantMode {
        *self.dormant.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Bootstrap the Tor client. Slow, fallible, and idempotent.
    ///
    /// On failure the dialer stays refusing — there is no degraded mode in
    /// which it connects some other way.
    // reachability-check: owed(#762)
    pub async fn bootstrap(&self) -> Result<(), TorDialError> {
        match self.client.bootstrap().await {
            Ok(()) => {
                debug!("tor bootstrap complete");
                Ok(())
            }
            Err(e) => {
                let err = TorDialError::BootstrapFailed {
                    kind: e.kind(),
                    blockage: self.client.bootstrap_status().blocked().map(|b| b.kind()),
                };
                warn!(class = err.class(), kind = %e.kind(), "tor bootstrap failed");
                Err(err)
            }
        }
    }

    /// What the dialer would do right now, in arti's own terms.
    pub fn readiness(&self) -> Readiness {
        let status = self.client.bootstrap_status();
        if status.ready_for_traffic() {
            Readiness::Ready
        } else {
            Readiness::Bootstrapping {
                progress: status.as_frac(),
                blockage: status.blocked().map(|b| b.kind()),
            }
        }
    }

    /// Open a Tor stream to `host:port`.
    ///
    /// `isolation_key`: equal keys may share a circuit; distinct keys are put
    /// in distinct circuit isolation groups. `None` joins this dialer's own
    /// unkeyed group, which no key can join.
    ///
    /// When the dial fails at or near the exit (`circuit_is_suspect` in
    /// `error.rs`: the exit could not resolve or reach the destination, timed
    /// out, refused, was busy), the key's group — the unkeyed one included —
    /// gets a fresh isolation token, so the NEXT dial on that key cannot
    /// reuse the circuit that failed; at most once per key per
    /// [`crate::EXIT_ROTATION_INTERVAL`], backing off to
    /// [`crate::EXIT_ROTATION_INTERVAL_MAX`] (ADR-0567; [`Self::exit_rotation`]
    /// turns it off). The failed dial itself is NOT retried, and a dial that
    /// is cancelled or dropped before it fails rotates nothing.
    #[tracing::instrument(skip_all, fields(isolated = isolation_key.is_some()))]
    pub async fn connect(
        &self,
        host: &str,
        port: u16,
        isolation_key: Option<&str>,
    ) -> Result<TorStream, TorDialError> {
        if let Err(err) = check_target(host, port) {
            warn!(
                class = err.class(),
                "tor dial refused before it left this device"
            );
            return Err(err);
        }

        // Kept: the failure arm rotates only if the group still holds THIS
        // token (compare-and-swap).
        let used = self.isolation.token_for(isolation_key);
        let mut prefs = StreamPrefs::new();
        prefs.set_isolation(used);

        match self.client.connect_with_prefs((host, port), &prefs).await {
            Ok(stream) => {
                debug!("tor stream open");
                Ok(TorStream::new(stream))
            }
            Err(e) => {
                let err = classify_connect(&e, self.readiness());
                warn!(class = err.class(), kind = %e.kind(), "tor dial failed");
                after_dial_failure(
                    &self.isolation,
                    isolation_key,
                    used,
                    e.kind(),
                    Instant::now(),
                    self.exit_rotation,
                );
                Err(err)
            }
        }
    }

    /// [`Self::connect`] in the shape the `zec-wallet` SDK's `NetDialer` port
    /// declares. The box always holds a [`TorStream`].
    pub async fn dial(
        &self,
        host: &str,
        port: u16,
        isolation_key: Option<&str>,
    ) -> Result<Box<dyn AsyncByteStream>, TorDialError> {
        Ok(Box::new(self.connect(host, port, isolation_key).await?))
    }
}

/// [`TorDialer::connect`]'s failure arm, pure so a test can drive it without a
/// bootstrapped client (an unbootstrapped one only ever answers
/// `NotBootstrapped`): if `kind` says the circuit is suspect
/// ([`circuit_is_suspect`]), move `key`'s group off the circuit `used` ran on,
/// within the registry's bound. Returns whether it rotated.
///
/// One `debug!` per rotation naming the KIND only — never the key (the
/// README's logging policy: a wallet's isolation key can carry a token).
pub(crate) fn after_dial_failure(
    registry: &IsolationRegistry,
    key: Option<&str>,
    used: IsolationToken,
    kind: ErrorKind,
    now: Instant,
    policy: ExitRotation,
) -> bool {
    if !circuit_is_suspect(kind) {
        return false;
    }
    let rotated = registry.rotate_after_exit_failure(key, used, now, policy);
    if rotated {
        debug!(%kind, "isolation group left a suspect circuit");
    }
    rotated
}

/// Refuse targets that can never be a Tor stream, before a circuit is spent on
/// them.
fn check_target(host: &str, port: u16) -> Result<(), TorDialError> {
    if host.is_empty() {
        return Err(TorDialError::BadTarget {
            problem: TargetProblem::EmptyHost,
        });
    }
    if host.len() > MAX_HOST_LEN {
        return Err(TorDialError::BadTarget {
            problem: TargetProblem::HostTooLong { len: host.len() },
        });
    }
    if port == 0 {
        return Err(TorDialError::BadTarget {
            problem: TargetProblem::PortZero,
        });
    }
    if is_onion(host) {
        return Err(TorDialError::OnionUnsupported);
    }
    Ok(())
}

/// Byte-wise so a non-ASCII host cannot panic a slice, and case-insensitive
/// because `Example.ONION` is the same address.
fn is_onion(host: &str) -> bool {
    let host = host.as_bytes();
    host.len() >= ONION_SUFFIX.len()
        && host[host.len() - ONION_SUFFIX.len()..].eq_ignore_ascii_case(ONION_SUFFIX)
}

/// Install `ring` as the process-wide TLS crypto provider IF NONE IS INSTALLED
/// YET, before arti's rustls backend asks for one.
///
/// Not redundant, and not reachable from this workspace's tests: rustls
/// auto-installs from its own crate features, but only when exactly one of
/// `ring`/`aws_lc_rs` is enabled — with both it panics. deny.toml keeps
/// aws-lc-rs out of THIS graph; a third-party host graph is not bound by that.
///
/// What this does NOT do (the crypto angle on FR-5 C0): own the choice.
/// arti reads the process-global provider at use (`tor-rtcompat`'s rustls
/// impl), so a host that installed a provider before this ran owns arti's TLS
/// to guards. The consumer that must not depend on the global — the wallet's
/// TLS to lightwalletd — passes an explicit provider of its own and is
/// unaffected either way.
fn ensure_crypto_provider() {
    use rustls::crypto::CryptoProvider;
    if CryptoProvider::get_default().is_some() {
        return;
    }
    if rustls::crypto::ring::default_provider()
        .install_default()
        .is_err()
    {
        debug!("a rustls CryptoProvider was already installed; using the host's");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `BridgeConfig` is not re-exported by `arti-client` and `tor-guardmgr` is
    /// not a dependency of this crate, so the element type is named through the
    /// public `BridgeList` alias rather than imported.
    type Bridge = <arti_client::config::BridgeList as IntoIterator>::Item;

    /// How many bridges a BUILT configuration actually holds. `AsRef` is arti's
    /// own accessor and it answers `&[]` when `bridge-client` is off, which is
    /// the pre-feature shape this row must go red against.
    fn bridges_held(config: &TorClientConfig) -> usize {
        let held: &[Bridge] = config.as_ref();
        held.len()
    }

    fn problem(host: &str, port: u16) -> Option<TargetProblem> {
        match check_target(host, port) {
            Err(TorDialError::BadTarget { problem }) => Some(problem),
            _ => None,
        }
    }

    /// D13's first falsifier: a bridge that parses and then reaches a config
    /// holding nothing. Asserted against the BUILT config, never against the
    /// parse — the parse is what the pre-feature build also passes.
    #[test]
    fn a_pasted_bridge_line_reaches_a_built_config_that_holds_it() {
        let dirs = tempfile::TempDir::new().expect("tempdir");
        let lines = BridgeLines::parse(
            "192.0.2.55:38114 316E643333645F6D79216558614D3931657A5F5F\n\
             192.0.2.56:38115 316E643333645F6D79216558614D3931657A5F5E",
        )
        .expect("two well-formed vanilla bridge lines");
        assert_eq!(lines.len(), 2);

        let config = tor_config(dirs.path().join("state"), dirs.path().join("cache"), lines)
            .expect("arti accepts a configuration carrying two vanilla bridges");
        assert_eq!(
            bridges_held(&config),
            2,
            "the bridge lines parsed and then died at the config builder — the \
             exact defect D13's falsifier names"
        );
    }

    /// The other polarity, and it is the one that would break every user who
    /// supplies nothing: *carries a bridge* must not become *requires a bridge*.
    #[test]
    fn no_bridge_supplied_builds_a_config_that_holds_none() {
        let dirs = tempfile::TempDir::new().expect("tempdir");
        let config = tor_config(
            dirs.path().join("state"),
            dirs.path().join("cache"),
            BridgeLines::default(),
        )
        .expect("the ordinary directory-bootstrap configuration must still build");
        assert_eq!(bridges_held(&config), 0);
    }

    #[test]
    fn a_reachable_target_passes_every_check() {
        check_target("example.com", 443).expect("an ordinary target must not be refused");
        check_target(&"a".repeat(MAX_HOST_LEN), 1)
            .expect("a host exactly at the limit must be accepted");
    }

    #[test]
    fn an_empty_or_over_long_host_is_named_as_such() {
        assert_eq!(problem("", 443), Some(TargetProblem::EmptyHost));
        let too_long = "a".repeat(MAX_HOST_LEN + 1);
        assert_eq!(
            problem(&too_long, 443),
            Some(TargetProblem::HostTooLong {
                len: MAX_HOST_LEN + 1
            })
        );
    }

    #[test]
    fn port_zero_is_refused_here_and_not_at_an_exit() {
        assert_eq!(problem("example.com", 0), Some(TargetProblem::PortZero));
    }

    #[test]
    fn onion_targets_are_refused_in_every_spelling() {
        for host in [
            "eweiibe6tdjsdprb4px6rqrzzcsi22m4koia44kc5pcjr7nec2rlxyad.onion",
            "EWEIIBE6TDJSDPRB4PX6RQRZZCSI22M4KOIA44KC5PCJR7NEC2RLXYAD.ONION",
            "sub.example.OnIoN",
            ".onion",
        ] {
            assert!(
                matches!(check_target(host, 443), Err(TorDialError::OnionUnsupported)),
                "{host:?} was not refused as an onion address"
            );
        }
    }

    #[test]
    fn a_host_merely_containing_onion_is_not_an_onion_address() {
        check_target("onion.example.com", 443).expect("a clearnet host was refused as onion");
        check_target("myonion", 443).expect("a clearnet host was refused as onion");
        check_target("onion", 443).expect("a clearnet host was refused as onion");
    }

    #[test]
    fn a_non_ascii_host_is_classified_without_panicking() {
        let host = "ü".repeat(10);
        assert!(check_target(&host, 443).is_ok());
        assert!(!is_onion("ünion"));
    }
}

/// FR-54 §4.4 #3 — `connect`'s failure arm, factored into the pure
/// [`after_dial_failure`] because the offline harness can never reach it (an
/// unbootstrapped client always answers `BootstrapRequired`). Each row drives
/// the arm with one arti `ErrorKind` and reads the registry's token: the arm
/// must rotate exactly on `circuit_is_suspect`'s set and leave the token alone
/// on everything else. Written to the design's contract before the code.
#[cfg(test)]
mod exit_rotation_arm_tests {
    use std::time::{Duration, Instant};

    use super::*;
    use crate::ExitRotation;

    /// The CALL SITE, which the pure-fn rows below cannot see (built-diff
    /// security review): `connect`'s `Err` arm must hand the arm the token it
    /// DIALED with (`used`, not a fresh read — that would defeat the
    /// compare-and-swap), arti's own kind, and the dialer's OWN policy (not a
    /// default — that would ignore `exit_rotation(off())`). The 600-byte
    /// window is sliced on byte offsets, so the source it covers must stay
    /// ASCII (a non-ASCII character there would panic the test, not fail it).
    #[test]
    fn connect_hands_the_arm_the_dialed_token_the_kind_and_its_own_policy() {
        let src = include_str!("dialer.rs");
        let start = src
            .find("Err(e) => {\n                let err = classify_connect")
            .expect("connect's Err arm moved — re-anchor this test on it");
        let arm = &src[start..start + 600];
        let call = &arm[arm
            .find("after_dial_failure(")
            .expect("connect's Err arm no longer calls after_dial_failure")..];
        let call = &call[..call.find(");").expect("the call is not closed")];
        for needed in [
            "&self.isolation",
            "isolation_key",
            "used",
            "e.kind()",
            "self.exit_rotation",
        ] {
            assert!(
                call.contains(needed),
                "connect's failure arm does not pass `{needed}`: {call}"
            );
        }
        assert!(
            !call.contains("token_for"),
            "the arm re-reads the token instead of passing `used`"
        );
        assert!(
            !call.contains("ExitRotation::default"),
            "the arm ignores the dialer's policy"
        );
    }

    /// §4.1's rotate-set, by name.
    const ROTATE: [ErrorKind; 11] = [
        ErrorKind::RemoteHostResolutionFailed,
        ErrorKind::RemoteHostNotFound,
        ErrorKind::ExitTimeout,
        ErrorKind::RemoteNetworkTimeout,
        ErrorKind::RemoteNetworkFailed,
        ErrorKind::RemoteStreamError,
        ErrorKind::RemoteConnectionRefused,
        ErrorKind::RemoteStreamReset,
        ErrorKind::ExitPolicyRejected,
        ErrorKind::RelayTooBusy,
        ErrorKind::RemoteStreamClosed,
    ];

    /// §4.1's named don't-rotate kinds, plus the bootstrap / directory /
    /// local / target kinds a failed dial can carry.
    const KEEP: [ErrorKind; 14] = [
        ErrorKind::NoExit,
        ErrorKind::CircuitCollapse,
        ErrorKind::TorProtocolViolation,
        ErrorKind::BootstrapRequired,
        ErrorKind::TorNetworkTimeout,
        ErrorKind::TorAccessFailed,
        ErrorKind::LocalNetworkError,
        ErrorKind::DirectoryExpired,
        ErrorKind::TorDirectoryUnusable,
        ErrorKind::ForbiddenStreamTarget,
        ErrorKind::InvalidStreamTarget,
        ErrorKind::FeatureDisabled,
        ErrorKind::ClockSkew,
        ErrorKind::Other,
    ];

    fn arm(
        reg: &IsolationRegistry,
        key: Option<&str>,
        kind: ErrorKind,
        now: Instant,
        policy: ExitRotation,
    ) -> bool {
        let used = reg.token_for(key);
        after_dial_failure(reg, key, used, kind, now, policy)
    }

    #[test]
    fn every_suspect_kind_through_the_arm_replaces_the_dialed_token() {
        for kind in ROTATE {
            for key in [None, Some("k")] {
                let reg = IsolationRegistry::new();
                let used = reg.token_for(key);
                assert!(
                    arm(&reg, key, kind, Instant::now(), ExitRotation::default()),
                    "{kind:?} did not rotate (keyed: {})",
                    key.is_some()
                );
                assert_ne!(
                    reg.token_for(key),
                    used,
                    "{kind:?}: the arm said it rotated but the next dial reuses the circuit"
                );
            }
        }
    }

    #[test]
    fn no_other_kind_through_the_arm_moves_the_token() {
        for kind in KEEP {
            for key in [None, Some("k")] {
                let reg = IsolationRegistry::new();
                let used = reg.token_for(key);
                assert!(
                    !arm(&reg, key, kind, Instant::now(), ExitRotation::default()),
                    "{kind:?} rotated; a new circuit cannot help it (keyed: {})",
                    key.is_some()
                );
                assert_eq!(reg.token_for(key), used, "{kind:?} moved the token");
            }
        }
    }

    /// A non-suspect failure must not consume the key's rotation either: the
    /// next suspect failure is still the immediate first one.
    #[test]
    fn a_non_suspect_failure_leaves_the_first_rotation_immediate() {
        let reg = IsolationRegistry::new();
        let t0 = Instant::now();
        assert!(!arm(
            &reg,
            Some("k"),
            ErrorKind::CircuitCollapse,
            t0,
            ExitRotation::default()
        ));
        assert!(arm(
            &reg,
            Some("k"),
            ErrorKind::RemoteHostResolutionFailed,
            t0 + Duration::from_millis(1),
            ExitRotation::default()
        ));
    }

    #[test]
    fn the_arm_carries_the_bound_and_the_compare_and_swap() {
        let reg = IsolationRegistry::new();
        let t0 = Instant::now();
        let used = reg.token_for(None);
        assert!(after_dial_failure(
            &reg,
            None,
            used,
            ErrorKind::ExitTimeout,
            t0,
            ExitRotation::default()
        ));
        // A current-token failure inside the bound: refused. (Time only moves
        // forward here: the decay is applied by any later suspect failure, so a
        // check after the 3600 s one below would see a key that went quiet.)
        assert!(!arm(
            &reg,
            None,
            ErrorKind::ExitTimeout,
            t0 + Duration::from_secs(1),
            ExitRotation::default()
        ));
        // The racing dial that also used `used` fails much later: refused.
        assert!(!after_dial_failure(
            &reg,
            None,
            used,
            ErrorKind::ExitTimeout,
            t0 + Duration::from_secs(3600),
            ExitRotation::default()
        ));
    }

    #[test]
    fn the_arm_under_off_never_rotates_any_kind() {
        for kind in ROTATE {
            let reg = IsolationRegistry::new();
            let used = reg.token_for(None);
            assert!(
                !arm(&reg, None, kind, Instant::now(), ExitRotation::off()),
                "{kind:?} rotated with rotation off"
            );
            assert_eq!(reg.token_for(None), used);
        }
    }

    #[test]
    fn the_arm_rotates_only_the_key_it_was_given() {
        let reg = IsolationRegistry::new();
        let other = reg.token_for(Some("other"));
        let unkeyed = reg.token_for(None);
        assert!(arm(
            &reg,
            Some("k"),
            ErrorKind::RemoteNetworkFailed,
            Instant::now(),
            ExitRotation::default()
        ));
        assert_eq!(reg.token_for(Some("other")), other);
        assert_eq!(reg.token_for(None), unkeyed);
    }

    /// The two tables above are the arm's view; `circuit_is_suspect` is the
    /// predicate's. They must agree, or the arm grew its own list.
    #[test]
    fn the_arm_agrees_with_the_predicate_on_every_listed_kind() {
        for kind in ROTATE.into_iter().chain(KEEP) {
            let reg = IsolationRegistry::new();
            assert_eq!(
                arm(
                    &reg,
                    Some("k"),
                    kind,
                    Instant::now(),
                    ExitRotation::default()
                ),
                circuit_is_suspect(kind),
                "{kind:?}: the failure arm and circuit_is_suspect disagree"
            );
        }
    }

    /// The builder is additive and consuming (§4.3); a dialer built either way
    /// still refuses honestly before bootstrap. Compiles the public shape.
    #[tokio::test]
    async fn the_rotation_switch_is_a_consuming_builder() {
        let dirs = tempfile::TempDir::new().expect("tempdir");
        let mut builder = TorClientConfigBuilder::from_directories(
            dirs.path().join("state"),
            dirs.path().join("cache"),
        );
        builder.storage().permissions().dangerously_trust_everyone();
        let config = builder.build().expect("config");
        let dialer: TorDialer = TorDialer::from_config(config)
            .await
            .expect("unbootstrapped")
            .exit_rotation(ExitRotation::off());
        assert!(matches!(
            dialer.connect("example.com", 443, None).await,
            Err(TorDialError::NotBootstrapped { .. })
        ));
    }
}
