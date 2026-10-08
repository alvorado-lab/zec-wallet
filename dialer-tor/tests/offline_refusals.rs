//! Every test here drives a REAL `arti_client::TorClient` and touches NO
//! network: the client is created with `BootstrapBehavior::Manual` and never
//! bootstrapped, so each dial is refused before a socket is opened. They run
//! in `just ci` like any other unit test.
//!
//! What they are for is the polarity that matters: the dialer must REFUSE when
//! it cannot be honest, and must say which refusal it is.

use dialer_tor::{TargetProblem, TorClientConfig, TorDialError, TorDialer, arti_client};
use tempfile::TempDir;

/// A dialer over private scratch directories.
///
/// `dangerously_trust_everyone` is set for the TEST ONLY: fs-mistrust
/// otherwise judges the whole path down to the temp root, which differs
/// between a developer's macOS `$TMPDIR` (0700, user-owned) and a CI Linux
/// `/tmp` (1777) — a difference that has nothing to do with what is under
/// test. Production goes through `TorDialer::unbootstrapped`, which leaves
/// arti's default permission checks on.
async fn dialer(dirs: &TempDir) -> TorDialer {
    let mut builder = arti_client::config::TorClientConfigBuilder::from_directories(
        dirs.path().join("state"),
        dirs.path().join("cache"),
    );
    builder.storage().permissions().dangerously_trust_everyone();
    let config: TorClientConfig = builder.build().expect("the test config must build");
    TorDialer::from_config(config)
        .await
        .expect("creating an unbootstrapped client must not need the network")
}

#[tokio::test]
async fn an_unbootstrapped_dialer_refuses_and_says_it_is_bootstrapping() {
    let dirs = TempDir::new().expect("tempdir");
    let d = dialer(&dirs).await;

    let err = d
        .dial("example.com", 443, None)
        .await
        .err()
        .expect("an unbootstrapped dialer must NEVER return a stream");

    match err {
        TorDialError::NotBootstrapped { progress, blockage } => {
            assert!(
                (0.0..=1.0).contains(&progress),
                "progress {progress} is not a fraction"
            );
            assert!(
                blockage.is_some(),
                "arti knows why it is stuck; the dialer dropped that reason"
            );
        }
        other => panic!(
            "expected NotBootstrapped, got {other:?} ({})",
            other.class()
        ),
    }
}

#[tokio::test]
async fn readiness_agrees_with_the_refusal() {
    let dirs = TempDir::new().expect("tempdir");
    let d = dialer(&dirs).await;
    match d.readiness() {
        dialer_tor::Readiness::Bootstrapping { progress, blockage } => {
            assert!((0.0..=1.0).contains(&progress));
            assert!(blockage.is_some());
        }
        dialer_tor::Readiness::Ready => {
            panic!("a client that was never bootstrapped reported itself Ready")
        }
    }
}

/// The anti-vacuity row for the bootstrap gate: the SAME unbootstrapped dialer
/// must answer differently for a target it can judge locally. If it answered
/// `NotBootstrapped` for everything, the test above would prove nothing.
#[tokio::test]
async fn a_bad_target_beats_the_bootstrap_gate() {
    let dirs = TempDir::new().expect("tempdir");
    let d = dialer(&dirs).await;

    for (host, port, want) in [
        ("", 443u16, TargetProblem::EmptyHost),
        ("example.com", 0, TargetProblem::PortZero),
    ] {
        match d.dial(host, port, None).await {
            Err(TorDialError::BadTarget { problem }) => assert_eq!(problem, want),
            other => panic!(
                "{host:?}:{port} gave {:?}, not BadTarget({want:?})",
                other.err().map(|e| e.class())
            ),
        }
    }
}

/// The LGPL carve-out has a user-visible consequence, and this is it.
#[tokio::test]
async fn an_onion_target_is_refused_by_name_not_by_bootstrap() {
    let dirs = TempDir::new().expect("tempdir");
    let d = dialer(&dirs).await;

    let err = d
        .dial(
            "eweiibe6tdjsdprb4px6rqrzzcsi22m4koia44kc5pcjr7nec2rlxyad.onion",
            443,
            None,
        )
        .await
        .err()
        .expect("an onion target must never return a stream");

    assert!(
        matches!(err, TorDialError::OnionUnsupported),
        "onion target gave {err:?} ({}) instead of OnionUnsupported",
        err.class()
    );
    assert!(
        err.to_string().to_lowercase().contains("onion"),
        "the message does not name the state: {err}"
    );
}

/// A loopback or private target must be refused by arti's address filter, not
/// silently dialed direct. This one deliberately does NOT go through the
/// crate's own `check_target`, so it exercises the arti arm of the mapping.
#[tokio::test]
async fn a_local_target_is_refused_as_forbidden_not_dialed() {
    let dirs = TempDir::new().expect("tempdir");
    let d = dialer(&dirs).await;

    for host in ["127.0.0.1", "localhost", "192.168.1.1", "::1"] {
        let err = d
            .dial(host, 9050, None)
            .await
            .err()
            .unwrap_or_else(|| panic!("{host} returned a STREAM; the dialer left Tor"));
        assert!(
            matches!(err, TorDialError::ForbiddenTarget),
            "{host} gave {err:?} ({}), not ForbiddenTarget",
            err.class()
        );
    }
}

/// Two dials with the same key, then with different keys, then unkeyed — none
/// of them may produce a stream while unbootstrapped, and the isolation
/// bookkeeping must not change that.
#[tokio::test]
async fn isolation_keys_do_not_open_a_side_door() {
    let dirs = TempDir::new().expect("tempdir");
    let d = dialer(&dirs).await;

    for key in [Some("tx-1"), Some("tx-1"), Some("tx-2"), None] {
        let err = d
            .dial("example.com", 443, key)
            .await
            .err()
            .unwrap_or_else(|| panic!("key {key:?} produced a stream without a bootstrap"));
        assert_eq!(err.class(), "not-bootstrapped", "key {key:?} gave {err}");
    }
}

/// H-15's first addition (the Tor plugin's design §3.4): `set_dormant` is a
/// passthrough to arti AND records the mode, so a consumer can report what it
/// asked for. Against a REAL unbootstrapped client, no network: `Soft` then
/// `Normal`, the recorded mode following each call, and readiness still
/// answering while dormant — arti's dormancy stops periodic work, not
/// queries. What this cannot see offline is arti's own reaction to the mode,
/// so the mutant this row is watched against is the RECORDING half (spec
/// P6/P8 observe the plugin's calls through a fake engine, not through here).
#[tokio::test]
async fn set_dormant_records_the_mode_and_a_dormant_client_still_answers_readiness() {
    let dirs = TempDir::new().expect("tempdir");
    let d = dialer(&dirs).await;
    assert_eq!(d.dormant_mode(), dialer_tor::DormantMode::Normal);

    d.set_dormant(dialer_tor::DormantMode::Soft);
    assert_eq!(d.dormant_mode(), dialer_tor::DormantMode::Soft);
    assert!(
        matches!(d.readiness(), dialer_tor::Readiness::Bootstrapping { .. }),
        "a dormant, never-bootstrapped client must still answer Bootstrapping"
    );

    d.set_dormant(dialer_tor::DormantMode::Normal);
    assert_eq!(d.dormant_mode(), dialer_tor::DormantMode::Normal);
}
