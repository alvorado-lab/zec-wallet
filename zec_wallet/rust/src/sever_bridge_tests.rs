//! Stage S16 `bridge` (§3.3) — the duress force-sever as the host reaches it,
//! `WalletHandle.severCustody` → `SeverReport`. The TEST AUTHOR's rows, written
//! blind (IT-2a) against the names declared in the run's `contractFindings`:
//!
//!   * `api::wallet::WalletHandle::sever_custody(config: WalletConfig,
//!     deadline_ms: u32) -> Result<api::state::SeverReport, WalletApiError>`;
//!   * `api::state::SeverReport { severed, holder, files }` and its enums,
//!     each mirroring the core's variants by name plus the bridge's `Unknown`
//!     arm: `SeverOutcome { Severed { count }, SeveredUnproven { reason },
//!     AlreadyGone, NotSevered { cause }, Unknown }`, `UnprovenReason`,
//!     `NotSeveredCause`, `HolderSeen`, `FilesOutcome`;
//!   * `convert.rs`'s `From<rw::SeverReport> for api_state::SeverReport`;
//!   * `convert::sever_deadline(deadline_ms: u32) -> Duration`, the clamp —
//!     the contract names the clamp, not a function; see the finding on
//!     [`sever_deadline_crosses_in_milliseconds_and_clamps_to_the_budget`].
//!
//! Its own file, because `convert.rs` and `api/wallet.rs` are the
//! implementer's; in the crate, because the crate is a cdylib/staticlib and a
//! `tests/*.rs` cannot link it. No FRB codegen is touched: the regen is the
//! fold's.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::api::config as api_config;
// The DTO and its enums live where every bridge DTO does, `api::state`; the
// verb is `api::wallet::WalletHandle` (the join's module path).
use crate::api::state as api_state;
use crate::api::wallet::WalletHandle;
use zec_wallet_core as rw;

fn config_with_db_dir(db_dir: &str) -> api_config::WalletConfig {
    api_config::WalletConfig {
        db_dir: db_dir.to_string(),
        network: api_config::Network::Main,
        endpoint_url: "https://zec.rocks:443".to_string(),
        endpoint_auth_header: None,
        endpoint_auth_value: None,
        tor: api_config::TorPolicy::Off,
        seed_persistence: api_config::SeedPersistence::SealedKeychain,
        birthday_height: None,
        broadcast_jitter: api_config::JitterPolicy::None,
        machine_memo_prefixes: Vec::new(),
        sync_servers: None,
    }
}

// ---- the report, field by field ------------------------------------------

fn unproven_label(r: api_state::UnprovenReason) -> &'static str {
    match r {
        api_state::UnprovenReason::CountUnreadable => "countUnreadable",
        api_state::UnprovenReason::Unknown => "unknown",
    }
}

fn cause_label(c: api_state::NotSeveredCause) -> &'static str {
    match c {
        api_state::NotSeveredCause::NothingSevered => "nothingSevered",
        api_state::NotSeveredCause::Timeout => "timeout",
        api_state::NotSeveredCause::Busy => "busy",
        api_state::NotSeveredCause::PastDeadline => "pastDeadline",
        api_state::NotSeveredCause::VaultAbsent => "vaultAbsent",
        api_state::NotSeveredCause::KeystoreUnavailable => "keystoreUnavailable",
        api_state::NotSeveredCause::StillRunning => "stillRunning",
        api_state::NotSeveredCause::Unknown => "unknown",
    }
}

fn outcome_label(o: api_state::SeverOutcome) -> String {
    match o {
        api_state::SeverOutcome::Severed { count } => format!("severed({count})"),
        api_state::SeverOutcome::SeveredUnproven { reason } => {
            format!("severedUnproven({})", unproven_label(reason))
        }
        api_state::SeverOutcome::AlreadyGone => "alreadyGone".to_string(),
        api_state::SeverOutcome::NotSevered { cause } => {
            format!("notSevered({})", cause_label(cause))
        }
        api_state::SeverOutcome::Unknown => "unknown".to_string(),
    }
}

fn holder_label(h: api_state::HolderSeen) -> &'static str {
    match h {
        api_state::HolderSeen::None => "none",
        api_state::HolderSeen::ThisProcess => "thisProcess",
        api_state::HolderSeen::OtherProcess => "otherProcess",
        api_state::HolderSeen::Unknown => "unknown",
    }
}

fn files_label(f: api_state::FilesOutcome) -> &'static str {
    match f {
        api_state::FilesOutcome::Removed => "removed",
        api_state::FilesOutcome::LeftForHost => "leftForHost",
        api_state::FilesOutcome::StillInUse => "stillInUse",
        api_state::FilesOutcome::Unknown => "unknown",
    }
}

/// What the host reads off one bridged report: `(severed, holder, files)`.
fn crossed(report: rw::SeverReport) -> (String, &'static str, &'static str) {
    let dto = api_state::SeverReport::from(report);
    (
        outcome_label(dto.severed),
        holder_label(dto.holder),
        files_label(dto.files),
    )
}

/// §3.3 assertion 1. Every core outcome (with its reason or cause), holder and
/// files value crosses to its OWN Dart value, never to another's and never to
/// `Unknown` (which exists for a core variant this build does not know). The
/// three fields are crossed in every combination, so a field that took its
/// value from a sibling — `holder` read off `files`, say — is caught too. The
/// pairs a host must never confuse are then asserted by name: reading
/// `severedUnproven` as `severed` claims a proof the phone could not give;
/// `timeout` as `busy` (or back) sends the host to the wrong retry;
/// `alreadyGone` as `severed` claims this call severed something; and
/// `otherProcess` as `thisProcess` claims a poison that never ran.
#[test]
fn sever_report_dto_maps_every_variant_without_transposition() {
    let outcomes = [
        (rw::SeverOutcome::Severed { count: 3 }, "severed(3)"),
        (
            rw::SeverOutcome::SeveredUnproven {
                reason: rw::UnprovenReason::CountUnreadable,
            },
            "severedUnproven(countUnreadable)",
        ),
        (rw::SeverOutcome::AlreadyGone, "alreadyGone"),
        (
            rw::SeverOutcome::NotSevered {
                cause: rw::NotSeveredCause::NothingSevered,
            },
            "notSevered(nothingSevered)",
        ),
        (
            rw::SeverOutcome::NotSevered {
                cause: rw::NotSeveredCause::Timeout,
            },
            "notSevered(timeout)",
        ),
        (
            rw::SeverOutcome::NotSevered {
                cause: rw::NotSeveredCause::Busy,
            },
            "notSevered(busy)",
        ),
        (
            rw::SeverOutcome::NotSevered {
                cause: rw::NotSeveredCause::PastDeadline,
            },
            "notSevered(pastDeadline)",
        ),
        (
            rw::SeverOutcome::NotSevered {
                cause: rw::NotSeveredCause::VaultAbsent,
            },
            "notSevered(vaultAbsent)",
        ),
        (
            rw::SeverOutcome::NotSevered {
                cause: rw::NotSeveredCause::KeystoreUnavailable,
            },
            "notSevered(keystoreUnavailable)",
        ),
    ];
    let holders = [
        (rw::HolderSeen::None, "none"),
        (rw::HolderSeen::ThisProcess, "thisProcess"),
        (rw::HolderSeen::OtherProcess, "otherProcess"),
    ];
    let files = [
        (rw::FilesOutcome::Removed, "removed"),
        (rw::FilesOutcome::LeftForHost, "leftForHost"),
    ];

    let mut seen = 0;
    for (severed, want_severed) in outcomes {
        for (holder, want_holder) in holders {
            for (files, want_files) in files {
                let report = rw::SeverReport {
                    severed,
                    holder,
                    files,
                };
                assert_eq!(
                    crossed(report),
                    (want_severed.to_string(), want_holder, want_files),
                    "{report:?} must cross field by field to its own Dart values"
                );
                seen += 1;
            }
        }
    }
    assert_eq!(seen, 9 * 3 * 2, "every combination crossed");

    // The count is the proof; it crosses as the core read it.
    let proven = |count| rw::SeverReport {
        severed: rw::SeverOutcome::Severed { count },
        holder: rw::HolderSeen::ThisProcess,
        files: rw::FilesOutcome::LeftForHost,
    };
    assert_eq!(crossed(proven(1)).0, "severed(1)");
    assert_eq!(crossed(proven(4_096)).0, "severed(4096)");

    // The named pairs, one by one.
    let with = |severed| rw::SeverReport {
        severed,
        holder: rw::HolderSeen::None,
        files: rw::FilesOutcome::Removed,
    };
    let pairs = [
        (
            rw::SeverOutcome::Severed { count: 2 },
            rw::SeverOutcome::SeveredUnproven {
                reason: rw::UnprovenReason::CountUnreadable,
            },
        ),
        (
            rw::SeverOutcome::NotSevered {
                cause: rw::NotSeveredCause::Timeout,
            },
            rw::SeverOutcome::NotSevered {
                cause: rw::NotSeveredCause::Busy,
            },
        ),
        (
            rw::SeverOutcome::Severed { count: 2 },
            rw::SeverOutcome::AlreadyGone,
        ),
    ];
    for (a, b) in pairs {
        assert_ne!(
            crossed(with(a)).0,
            crossed(with(b)).0,
            "{a:?} and {b:?} must reach the host as different values"
        );
    }
    let holder = |holder| rw::SeverReport {
        severed: rw::SeverOutcome::AlreadyGone,
        holder,
        files: rw::FilesOutcome::LeftForHost,
    };
    assert_ne!(
        crossed(holder(rw::HolderSeen::ThisProcess)).1,
        crossed(holder(rw::HolderSeen::OtherProcess)).1,
        "thisProcess and otherProcess must reach the host as different values"
    );
}

// ---- the door ------------------------------------------------------------

/// §3.3 assertion 2. A relative (or empty) `dbDir` is refused at the bridge's
/// door, typed `RW-CFG-003`, before the core is called: a relative dir would
/// resolve the sever against the process cwd. The refusal is the door's and
/// nothing else's — the core never answers `InvalidDbDir`, and the core's
/// first act on any path is to create the directory and its lock file
/// (`WalletLock::acquire`). So a real relative directory is planted under the
/// cwd with a file in it, and after every refused shape it still holds that
/// file and has no lock file: the core never touched it.
#[tokio::test]
async fn sever_custody_refuses_a_relative_db_dir_before_the_core() {
    // The sever quiesces the process device log (S5): serialise with the
    // tests that assert on it.
    let _serial = crate::device_log::test_sink::PROCESS_LOG.lock().await;
    let planted = tempfile::Builder::new()
        .prefix("zw-sever-relative-")
        .tempdir_in(".")
        .expect("a directory under the cwd");
    let name = planted
        .path()
        .file_name()
        .and_then(|n| n.to_str())
        .expect("a UTF-8 directory name")
        .to_string();
    let sentinel = planted.path().join("wallet.db");
    std::fs::write(&sentinel, b"not a wallet").expect("plant a file");

    for db_dir in [name.clone(), format!("./{name}"), String::new()] {
        assert!(
            !std::path::Path::new(&db_dir).is_absolute(),
            "the case is relative: {db_dir:?}"
        );
        let code = match WalletHandle::sever_custody(config_with_db_dir(&db_dir), 2_000).await {
            Ok(_) => panic!("severCustody must refuse the relative dbDir {db_dir:?}"),
            Err(e) => e.code,
        };
        assert_eq!(
            code, "RW-CFG-003",
            "the refusal of {db_dir:?} is the dbDir door's"
        );
    }
    assert!(
        sentinel.exists(),
        "a refused sever must leave the cwd-relative directory's files alone"
    );
    assert!(
        !planted
            .path()
            .join(rw::constants::WALLET_LOCK_FILE_NAME)
            .exists(),
        "a refused sever must never reach the core, whose first act is the lock file"
    );
}

/// The unlisted case (IT-1 +A; the brief's "the door calls
/// `quiesce_for_sever()` FIRST, before validating the dbDir", which §3.3's
/// assertion list does not name). The duress verb must leave nothing logged
/// or streamed, INCLUDING when the door refuses its `dbDir`: a host under
/// duress that passes a malformed path must still find the device log `Off`
/// and its host stream closed. The absolute case runs with a zero deadline,
/// so it makes no key-store call on the test host, and whatever it answers,
/// the log's state is the assertion.
///
/// Watched against: the `quiesce_for_sever` call moved after the dbDir gate,
/// or left out of the sever door.
#[test]
fn sever_custody_quiesces_the_device_log_first_even_when_the_door_refuses() {
    use crate::api::meta::DeviceLogLevel::{Detailed, Off};
    let _serial = crate::device_log::test_sink::PROCESS_LOG.blocking_lock();
    let gate = crate::device_log::process_gate();
    let temp = tempfile::tempdir().expect("tempdir");
    let absolute = temp.path().to_str().expect("a UTF-8 temp path").to_string();
    for (db_dir, refused) in [
        ("relative/sever".to_string(), true),
        (String::new(), true),
        (absolute, false),
    ] {
        let closed = Arc::new(AtomicBool::new(false));
        crate::device_log::process_host_slot().watch(Box::new(ClosedFlag(closed.clone())));
        gate.set(Detailed);
        let answer = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a current-thread runtime")
            .block_on(WalletHandle::sever_custody(config_with_db_dir(&db_dir), 0));
        if refused {
            match answer {
                Ok(_) => panic!("the door must refuse the dbDir {db_dir:?}"),
                Err(e) => assert_eq!(e.code, "RW-CFG-003", "the refusal is the door's"),
            }
        }
        assert_eq!(
            gate.level(),
            Off,
            "a sever at {db_dir:?} must leave the device log Off until the host re-arms"
        );
        assert!(
            closed.load(Ordering::SeqCst),
            "a sever at {db_dir:?} closes the host stream"
        );
        crate::device_log::quiesce_for_sever();
    }
}

/// A host subscription that records only that the slot dropped it — which is
/// what closes a Dart stream.
struct ClosedFlag(Arc<AtomicBool>);

impl crate::device_log::HostLineSink for ClosedFlag {
    fn send(&self, _line: crate::api::meta::DeviceLogLine) -> bool {
        true
    }
}

impl Drop for ClosedFlag {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

// ---- the deadline --------------------------------------------------------

/// §3.3 assertion 3. The deadline crosses as whole milliseconds and is clamped
/// on the Rust side to `[0, KEYCHAIN_WIPE_BUDGET]`: above the budget answers
/// the budget, `0` crosses as `0`, and everything between crosses unchanged.
///
/// FINDING (declared in `contractFindings`): through the VERB this clamp cannot
/// be seen, because the core already spends the EARLIER of the deadline and
/// `KEYCHAIN_WIPE_BUDGET` (`sever_blocking`'s `wipe_scope_until`), so a bridge
/// that did not clamp answers identically. The test therefore pins the
/// crossing function. The contract names the clamp, not the function:
/// `convert::sever_deadline` is this author's name for it. The budget is read
/// from the core's own re-exported `KEYCHAIN_WIPE_BUDGET` (one source for the
/// 4 s), and the rows are placed around it.
#[test]
fn sever_deadline_crosses_in_milliseconds_and_clamps_to_the_budget() {
    let budget = rw::KEYCHAIN_WIPE_BUDGET;
    let budget_ms = u32::try_from(budget.as_millis()).expect("the budget fits a u32 of ms");
    assert!(
        budget_ms > 2_000,
        "fixture: the 2 s row sits below the budget"
    );
    for (ms, want) in [
        (0, Duration::ZERO),
        (1, Duration::from_millis(1)),
        (2_000, Duration::from_millis(2_000)),
        (
            budget_ms - 1,
            Duration::from_millis(u64::from(budget_ms - 1)),
        ),
        (budget_ms, budget),
        (budget_ms + 1, budget),
        (60_000, budget),
        (u32::MAX, budget),
    ] {
        assert_eq!(
            crate::convert::sever_deadline(ms),
            want,
            "a {ms} ms deadline crosses as {want:?}"
        );
    }
}

// ---- Relim's four asks (sync point 1, relim-18) ----------------------------

/// A fresh absolute directory that never held a wallet.
fn never_a_wallet() -> (tempfile::TempDir, String) {
    let parent = tempfile::tempdir().expect("tempdir");
    let dir = parent.path().join("never-a-wallet");
    std::fs::create_dir_all(&dir).expect("the directory");
    let dir = dir.to_str().expect("a UTF-8 path").to_string();
    (parent, dir)
}

/// Wait until the process key-store worker takes calls again. A sever with a
/// tiny deadline abandons its native key-store call (the test host's is real),
/// and until that call returns every sever answers `notSevered(busy)` — the
/// documented shared-worker residual (ADR-0565). Probed on a scratch
/// directory, so the row's own calls stay first calls.
async fn settle_worker() {
    let (_parent, scratch) = never_a_wallet();
    let started = std::time::Instant::now();
    loop {
        let r = WalletHandle::sever_custody(config_with_db_dir(&scratch), 2_000)
            .await
            .unwrap_or_else(|e| panic!("the settle probe errs: {}", e.code));
        let label = outcome_label(r.severed);
        if label != "notSevered(busy)" && label != "notSevered(timeout)" {
            return;
        }
        assert!(
            started.elapsed() < Duration::from_secs(120), // a real Keychain call, under load: RC run took >30 s
            "the key-store worker never came back: {label}"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Relim's ask (a), the backstop: the deadline is a HARD wall-clock bound on
/// the WHOLE call. A core that stalls outside its key-store budget (here, one
/// that never answers in time) still gets an answer at
/// `deadline + SEVER_ANSWER_GRACE`: `notSevered(stillRunning)`,
/// `holder: unknown`, `files: stillInUse`, never a claimed sever. A core that
/// answers in time is passed through unchanged.
#[tokio::test]
async fn a_sever_answers_by_its_deadline_whatever_stalls() {
    let deadline = Duration::from_millis(20);
    let stalled = async {
        tokio::time::sleep(Duration::from_secs(30)).await;
        Ok(rw::SeverReport {
            severed: rw::SeverOutcome::Severed { count: 1 },
            holder: rw::HolderSeen::None,
            files: rw::FilesOutcome::Removed,
        })
    };
    let started = std::time::Instant::now();
    let answer = tokio::time::timeout(
        Duration::from_secs(5),
        answer_by(key("stalls"), deadline, stalled),
    )
    .await
    .expect("the backstop answered: no hard bound on the whole call")
    .unwrap_or_else(|e| panic!("an overrun is a report, never an error: {}", e.code));
    let took = started.elapsed();
    assert!(
        took < deadline + crate::convert::SEVER_ANSWER_GRACE + Duration::from_millis(200),
        "answered at {took:?}, past deadline + grace"
    );
    assert_eq!(
        (
            outcome_label(answer.severed),
            holder_label(answer.holder),
            files_label(answer.files)
        ),
        (
            "notSevered(stillRunning)".to_string(),
            "unknown",
            "stillInUse"
        ),
        "an overrun never claims a sever"
    );

    let on_time = async {
        Ok(rw::SeverReport {
            severed: rw::SeverOutcome::AlreadyGone,
            holder: rw::HolderSeen::None,
            files: rw::FilesOutcome::Removed,
        })
    };
    let answer = answer_by(key("on-time"), deadline, on_time)
        .await
        .unwrap_or_else(|e| panic!("a report: {}", e.code));
    assert_eq!(outcome_label(answer.severed), "alreadyGone");
}

/// A registry key of this row's own: `answer_by` keys its in-flight set by
/// the `dbDir`, and no row may share one with another.
fn key(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("/s16-bridge-rows/{name}"))
}

/// A core call that takes `for_` and then reports `report`, and records that
/// it was reached.
async fn slow_core(
    for_: Duration,
    report: rw::SeverReport,
    reached: Arc<AtomicBool>,
) -> Result<rw::SeverReport, rw::WalletError> {
    reached.store(true, Ordering::SeqCst);
    tokio::time::sleep(for_).await;
    Ok(report)
}

fn severed_report() -> rw::SeverReport {
    rw::SeverReport {
        severed: rw::SeverOutcome::Severed { count: 1 },
        holder: rw::HolderSeen::None,
        files: rw::FilesOutcome::Removed,
    }
}

/// The overrun is a DISTINCT answer (the S16 bridge diff review): its cause is
/// `stillRunning`, never the core's `timeout` (a key store that did not
/// answer), and its `files` is `stillInUse`, never `leftForHost`, the one
/// value a host purges on. The SDK's own sever may still be deleting in that
/// directory.
#[tokio::test]
async fn an_overrun_is_still_running_and_still_in_use_never_timeout_or_left_for_host() {
    let reached = Arc::new(AtomicBool::new(false));
    let answer = answer_by(
        key("overrun"),
        Duration::from_millis(10),
        slow_core(Duration::from_secs(2), severed_report(), reached),
    )
    .await
    .unwrap_or_else(|e| panic!("a report: {}", e.code));
    let cause = outcome_label(answer.severed);
    let files = files_label(answer.files);
    assert_ne!(
        cause, "notSevered(timeout)",
        "an overrun is not the core's timeout"
    );
    assert_ne!(
        files, "leftForHost",
        "never purge while the SDK still works here"
    );
    assert_eq!(
        (cause.as_str(), files),
        ("notSevered(stillRunning)", "stillInUse")
    );
}

/// A call at the same `dbDir` while the first sever still runs answers
/// `stillRunning` AT ONCE and never reaches the store. Reaching it would read
/// the orphan's held lock as `otherProcess` and answer `leftForHost` while the
/// orphan still sweeps.
#[tokio::test]
async fn a_call_during_a_still_running_sever_answers_still_running_without_reaching_the_store() {
    let dir = key("during");
    let first = answer_by(
        dir.clone(),
        Duration::from_millis(10),
        slow_core(
            Duration::from_secs(2),
            severed_report(),
            Arc::new(AtomicBool::new(false)),
        ),
    )
    .await
    .unwrap_or_else(|e| panic!("a report: {}", e.code));
    assert_eq!(outcome_label(first.severed), "notSevered(stillRunning)");

    let reached = Arc::new(AtomicBool::new(false));
    let started = std::time::Instant::now();
    let second = answer_by(
        dir,
        Duration::from_secs(2),
        slow_core(Duration::ZERO, severed_report(), reached.clone()),
    )
    .await
    .unwrap_or_else(|e| panic!("a report: {}", e.code));
    assert!(
        !reached.load(Ordering::SeqCst),
        "the second call reached the store"
    );
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "answered at once"
    );
    assert_eq!(
        (outcome_label(second.severed), files_label(second.files)),
        ("notSevered(stillRunning)".to_string(), "stillInUse")
    );
}

/// Once the SDK's own sever ends, the directory leaves the in-flight set, and
/// the next call reaches the store and answers the TRUE outcome.
#[tokio::test]
async fn once_the_sever_ends_the_next_call_answers_the_true_outcome() {
    let dir = key("after");
    let first = answer_by(
        dir.clone(),
        Duration::from_millis(10),
        slow_core(
            Duration::from_millis(400),
            severed_report(),
            Arc::new(AtomicBool::new(false)),
        ),
    )
    .await
    .unwrap_or_else(|e| panic!("a report: {}", e.code));
    assert_eq!(outcome_label(first.severed), "notSevered(stillRunning)");
    let started = std::time::Instant::now();
    while sever_in_flight(&dir) {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the core call ended but its dbDir stayed in flight"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let reached = Arc::new(AtomicBool::new(false));
    let next = answer_by(
        dir,
        Duration::from_secs(2),
        slow_core(
            Duration::ZERO,
            rw::SeverReport {
                severed: rw::SeverOutcome::AlreadyGone,
                holder: rw::HolderSeen::None,
                files: rw::FilesOutcome::Removed,
            },
            reached.clone(),
        ),
    )
    .await
    .unwrap_or_else(|e| panic!("a report: {}", e.code));
    assert!(
        reached.load(Ordering::SeqCst),
        "the next call reached the store"
    );
    assert_eq!(outcome_label(next.severed), "alreadyGone");
}

/// The grace crosses to Dart as the same number the bound uses.
#[test]
fn the_answer_grace_crosses_to_dart_as_the_bound_uses_it() {
    assert_eq!(
        u128::from(crate::api::meta::sever_answer_grace_ms()),
        crate::convert::SEVER_ANSWER_GRACE.as_millis()
    );
}

/// Relim's ask (a), through the verb: the lower clamp is 0 (well under the
/// asked ~1 s), and a tiny deadline answers within it plus the grace.
#[tokio::test]
async fn a_tiny_deadline_answers_within_it() {
    let _serial = crate::device_log::test_sink::PROCESS_LOG.lock().await;
    assert_eq!(crate::convert::sever_deadline(1), Duration::from_millis(1));
    let (_parent, dir) = never_a_wallet();
    for ms in [0_u32, 1, 50] {
        let started = std::time::Instant::now();
        WalletHandle::sever_custody(config_with_db_dir(&dir), ms)
            .await
            .unwrap_or_else(|e| panic!("a custody outcome is a report: {}", e.code));
        let took = started.elapsed();
        assert!(
            took <= Duration::from_millis(u64::from(ms)) + crate::convert::SEVER_ANSWER_GRACE,
            "a {ms} ms deadline answered at {took:?}"
        );
    }
    // Leave the shared worker free for the next row (see `settle_worker`).
    settle_worker().await;
}

/// Relim's ask (b): the report carries no path, key id or free string. None
/// of its types owns heap data (a `String`, a `Vec`, a `Box` would all need a
/// drop), and `NotSeveredCause` is a closed set of unit variants whose Dart
/// names the README lists, every one of them.
#[test]
fn sever_report_carries_no_path_key_or_free_string() {
    use std::mem::needs_drop;
    assert!(
        !needs_drop::<api_state::SeverReport>(),
        "SeverReport owns no heap data"
    );
    assert!(!needs_drop::<api_state::SeverOutcome>());
    assert!(!needs_drop::<api_state::UnprovenReason>());
    assert!(!needs_drop::<api_state::NotSeveredCause>());
    assert!(!needs_drop::<api_state::HolderSeen>());
    assert!(!needs_drop::<api_state::FilesOutcome>());

    let readme = include_str!("../../README.md");
    let listed = readme
        .lines()
        .find(|l| l.contains("`NotSeveredCause` is closed"))
        .expect("the README lists the causes");
    for cause in [
        api_state::NotSeveredCause::NothingSevered,
        api_state::NotSeveredCause::Timeout,
        api_state::NotSeveredCause::Busy,
        api_state::NotSeveredCause::PastDeadline,
        api_state::NotSeveredCause::VaultAbsent,
        api_state::NotSeveredCause::KeystoreUnavailable,
        api_state::NotSeveredCause::StillRunning,
        api_state::NotSeveredCause::Unknown,
    ] {
        let name = cause_label(cause);
        assert!(
            listed.contains(&format!("`{name}`")),
            "the README's cause list names `{name}`: {listed}"
        );
    }
}

/// Relim's ask (c): idempotence through the verb. A `dbDir` that never held a
/// wallet answers `alreadyGone`, with NO error, and so does the call after it.
/// (After a PROVEN sever, the second call's `alreadyGone` is the core's
/// `a_second_sever_reports_already_gone`, on both branches, over an injected
/// vault. The bridge verb reaches the host's REAL key store, and these rows
/// never custody a wallet into it.)
#[tokio::test]
async fn a_dir_that_never_held_a_wallet_answers_already_gone_every_time() {
    let _serial = crate::device_log::test_sink::PROCESS_LOG.lock().await;
    settle_worker().await;
    let (_parent, dir) = never_a_wallet();
    for call in 1..=2 {
        let r = WalletHandle::sever_custody(config_with_db_dir(&dir), 2_000)
            .await
            .unwrap_or_else(|e| {
                panic!("call {call}: no error for a dir with no wallet: {}", e.code)
            });
        assert_eq!(
            outcome_label(r.severed),
            "alreadyGone",
            "call {call}: nothing to sever is alreadyGone"
        );
    }
}

/// Relim's ask (d): a STATIC entry point that needs only the dir and a
/// deadline — no open handle, and nothing else in the config. Every other
/// field here is one the create/open door refuses (a malformed endpoint, half
/// an auth pair — the fixture checks the door does refuse it), and the sever
/// answers anyway: it consumes ONLY `dbDir`.
#[tokio::test]
async fn sever_custody_needs_only_the_dir_and_a_deadline() {
    let _serial = crate::device_log::test_sink::PROCESS_LOG.lock().await;
    settle_worker().await;
    let (_parent, dir) = never_a_wallet();
    let mut config = config_with_db_dir(&dir);
    config.endpoint_url = "not a url".to_string();
    config.endpoint_auth_header = Some("x-only-half".to_string());
    config.endpoint_auth_value = None;
    assert!(
        rw::WalletConfig::try_from(config.clone()).is_err(),
        "fixture: the create/open door refuses this config"
    );
    let r = WalletHandle::sever_custody(config, 2_000)
        .await
        .unwrap_or_else(|e| panic!("the sever must not validate beyond dbDir: {}", e.code));
    assert_eq!(outcome_label(r.severed), "alreadyGone");
}

// ---- the in-flight set's key and its doors (ADR-0566) -----------------------

/// Start a sever of `dir` that stays in flight for `for_`, and return once the
/// bridge has answered it `stillRunning`.
async fn in_flight_for(dir: &std::path::Path, for_: Duration) {
    let first = answer_by(
        dir.to_path_buf(),
        Duration::from_millis(10),
        slow_core(for_, severed_report(), Arc::new(AtomicBool::new(false))),
    )
    .await
    .unwrap_or_else(|e| panic!("a report: {}", e.code));
    assert_eq!(outcome_label(first.severed), "notSevered(stillRunning)");
}

/// The set is keyed by the directory's REAL path: a symlinked alias (the iOS
/// `/var` → `/private/var` shape), a `..` spelling and a trailing `.` of the
/// same directory all meet the running sever's entry and answer `stillRunning`
/// without reaching the store.
#[cfg(unix)]
#[tokio::test]
async fn a_symlinked_or_dotted_alias_meets_the_same_in_flight_entry() {
    let parent = tempfile::tempdir().expect("tempdir");
    let real = parent.path().join("real").join("wallet");
    std::fs::create_dir_all(&real).expect("the real dir");
    let alias = parent.path().join("alias");
    std::os::unix::fs::symlink(parent.path().join("real"), &alias).expect("a symlink");
    in_flight_for(&real, Duration::from_secs(3)).await;
    for spelled in [
        alias.join("wallet"),
        parent
            .path()
            .join("real")
            .join("..")
            .join("real")
            .join("wallet"),
        real.join("."),
    ] {
        let reached = Arc::new(AtomicBool::new(false));
        let r = answer_by(
            spelled.clone(),
            Duration::from_secs(2),
            slow_core(Duration::ZERO, severed_report(), reached.clone()),
        )
        .await
        .unwrap_or_else(|e| panic!("a report: {}", e.code));
        assert!(
            !reached.load(Ordering::SeqCst),
            "{spelled:?} reached the store while the sever of its real path ran"
        );
        assert_eq!(outcome_label(r.severed), "notSevered(stillRunning)");
    }
}

/// ONE guard for every other `dbDir` door: while a sever of the directory is
/// still in flight, open, create, restore, watch-only and the host-seed pair
/// answer `walletAlreadyOpen` (RW-LIFE-001), and wipe and wipeForce answer
/// `walletOpen` (RW-LIFE-004), typed and before any work. The directory is
/// the child of a regular FILE, so a door the guard missed fails in its own
/// lock try with an I/O error, never touching the host's real key store.
#[tokio::test]
async fn every_door_refuses_a_dir_whose_sever_still_runs() {
    let _serial = crate::device_log::test_sink::PROCESS_LOG.lock().await;
    let parent = tempfile::tempdir().expect("tempdir");
    let file = parent.path().join("a-regular-file");
    std::fs::write(&file, b"not a directory").expect("the file");
    let dir = file.join("wallet");
    in_flight_in_the_process(&dir, Duration::from_secs(20)).await;
    let config = || config_with_db_dir(dir.to_str().expect("a UTF-8 path"));
    fn code<T>(r: Result<T, crate::api::error::WalletApiError>) -> String {
        match r {
            Ok(_) => "ok".to_string(),
            Err(e) => e.code,
        }
    }
    let answers = [
        ("open", code(WalletHandle::open(config()).await)),
        (
            "createGenerated",
            code(WalletHandle::create_generated(config()).await),
        ),
        (
            "restore",
            code(WalletHandle::restore(config(), vec!["abandon".to_string(); 12], None).await),
        ),
        (
            "createWatchOnly",
            code(WalletHandle::create_watch_only(config(), "uview".to_string(), 0).await),
        ),
        (
            "createWithHostSeed",
            code(WalletHandle::create_with_host_seed(config(), None).await),
        ),
        (
            "openWithHostSeed",
            code(WalletHandle::open_with_host_seed(config()).await),
        ),
        ("wipe", code(WalletHandle::wipe(config()).await)),
        ("wipeForce", code(WalletHandle::wipe_force(config()).await)),
    ];
    for (door, answered) in answers {
        let want = if door.starts_with("wipe") {
            "RW-LIFE-004"
        } else {
            "RW-LIFE-001"
        };
        assert_eq!(answered, want, "{door} while a sever still runs");
    }
    crate::device_log::quiesce_for_sever();
}

/// The in-flight entry is removed by a guard the SPAWNED task holds, so a core
/// call that PANICS still clears it (the task's future unwinds), and the next
/// call reaches the store.
#[tokio::test]
async fn a_panicking_sever_task_still_clears_its_entry() {
    let dir = key("panics");
    let joined = tokio::spawn(answer_by(dir.clone(), Duration::from_secs(2), async {
        if std::hint::black_box(true) {
            panic!("the seam: the core call panics");
        }
        Ok(severed_report())
    }))
    .await;
    assert!(
        joined.is_err_and(|e| e.is_panic()),
        "the core call's panic is re-raised"
    );
    assert!(
        !sever_in_flight(&dir),
        "a panicking sever left its dbDir in flight"
    );
}

// ---- the fail-closed rule, over PRIVATE registries (ADR-0566) ----------------

/// A key function the row controls: it counts its calls, and stalls (blocks
/// its blocking-pool thread) for `stall` on every call numbered in `stalls`.
fn key_fn(
    calls: Arc<std::sync::atomic::AtomicUsize>,
    stalls: &'static [usize],
    stall: Duration,
) -> crate::convert::KeyOf {
    Arc::new(move |dir: &std::path::Path| {
        let n = calls.fetch_add(1, Ordering::SeqCst) + 1;
        if stalls.contains(&n) {
            std::thread::sleep(stall);
        }
        dir.to_path_buf()
    })
}

/// A sever whose real-path key stalls past its bound registers UNRESOLVED,
/// and then EVERY door refuses, whatever its spelling, until it ends. Without
/// that, a door spelling the same directory differently would be let through.
#[tokio::test]
async fn a_registration_stall_makes_every_door_refuse() {
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let registry =
        crate::convert::SeverRegistry::new(key_fn(calls.clone(), &[1], Duration::from_secs(3)));
    let first = crate::convert::answer_by_in(
        &registry,
        key("stall-a"),
        Duration::from_millis(10),
        slow_core(
            Duration::from_secs(5),
            severed_report(),
            Arc::new(AtomicBool::new(false)),
        ),
    )
    .await
    .unwrap_or_else(|e| panic!("a report: {}", e.code));
    assert_eq!(outcome_label(first.severed), "notSevered(stillRunning)");
    let started = std::time::Instant::now();
    let door = registry
        .refuse(&key("another-spelling"), rw::WalletError::WalletAlreadyOpen)
        .await;
    assert!(
        matches!(door, Err(rw::WalletError::WalletAlreadyOpen)),
        "a door must refuse while a sever is unresolved"
    );
    assert!(
        started.elapsed() < Duration::from_millis(100),
        "refused at once"
    );
}

/// A door whose OWN real-path key stalls refuses within [`DOOR_KEY_BOUND`]:
/// it fails closed, and never hangs on a stalled filesystem.
#[tokio::test]
async fn a_door_side_stall_refuses_within_the_bound() {
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    // Call 1 is the sever's registration (resolves); call 2 is the door's.
    let registry =
        crate::convert::SeverRegistry::new(key_fn(calls.clone(), &[2], Duration::from_secs(5)));
    crate::convert::answer_by_in(
        &registry,
        key("door-stall-a"),
        Duration::from_millis(10),
        slow_core(
            Duration::from_secs(8),
            severed_report(),
            Arc::new(AtomicBool::new(false)),
        ),
    )
    .await
    .unwrap_or_else(|e| panic!("a report: {}", e.code));
    let started = std::time::Instant::now();
    let door = tokio::time::timeout(
        Duration::from_secs(4),
        registry.refuse(&key("door-stall-b"), rw::WalletError::WalletOpen),
    )
    .await
    .expect("the door hung on its stalled key");
    assert!(
        matches!(door, Err(rw::WalletError::WalletOpen)),
        "a door that cannot resolve its key refuses"
    );
    assert!(
        started.elapsed() < crate::convert::DOOR_KEY_BOUND + Duration::from_millis(200),
        "refused within the door's bound: {:?}",
        started.elapsed()
    );
}

/// With nothing in flight a door does NO key work: no blocking-pool hop, no
/// filesystem call. Once a sever is in flight, a door does.
#[tokio::test]
async fn an_empty_set_does_no_key_work() {
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let registry = crate::convert::SeverRegistry::new(key_fn(calls.clone(), &[], Duration::ZERO));
    for _ in 0..3 {
        registry
            .refuse(&key("empty-door"), rw::WalletError::WalletAlreadyOpen)
            .await
            .unwrap_or_else(|_| panic!("nothing in flight: the door passes"));
    }
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "no key work on an empty set"
    );

    crate::convert::answer_by_in(
        &registry,
        key("empty-sever"),
        Duration::from_millis(10),
        slow_core(
            Duration::from_secs(3),
            severed_report(),
            Arc::new(AtomicBool::new(false)),
        ),
    )
    .await
    .unwrap_or_else(|e| panic!("a report: {}", e.code));
    let before = calls.load(Ordering::SeqCst);
    let _ = registry
        .refuse(&key("empty-door"), rw::WalletError::WalletAlreadyOpen)
        .await;
    assert!(
        calls.load(Ordering::SeqCst) > before,
        "fixture: with a sever in flight the door does compute its key"
    );
}

// ---- interleavings, forced with latches, not sleeps ------------------------

/// A one-way latch a blocking-pool thread can wait on.
#[derive(Default)]
struct Latch {
    open: std::sync::Mutex<bool>,
    cv: std::sync::Condvar,
}

impl Latch {
    fn open(&self) {
        *self.open.lock().unwrap_or_else(|p| p.into_inner()) = true;
        self.cv.notify_all();
    }
    fn is_open(&self) -> bool {
        *self.open.lock().unwrap_or_else(|p| p.into_inner())
    }
    fn wait(&self, at_most: Duration) {
        let guard = self.open.lock().unwrap_or_else(|p| p.into_inner());
        let _ = self
            .cv
            .wait_timeout_while(guard, at_most, |open| !*open)
            .unwrap_or_else(|p| p.into_inner());
    }
}

/// Until `cond` holds (bounded), without blocking the runtime.
async fn until(what: &str, cond: impl Fn() -> bool) {
    let started = std::time::Instant::now();
    while !cond() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "{what}: never happened"
        );
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

/// Interleaving (a): a door is INSIDE its real-path key computation when a
/// second, differently spelled sever registers (unresolved: its own key
/// stalls). When the door's key comes back, the door refuses. It re-reads the
/// registry after its await, in one snapshot, rather than trusting the
/// registry it saw before.
#[tokio::test]
async fn a_door_mid_key_refuses_when_a_sever_registers_unresolved_meanwhile() {
    let door_dir = key("mid-door");
    let late_dir = key("mid-late-sever");
    let door_in = Arc::new(Latch::default());
    let door_go = Arc::new(Latch::default());
    let key_of: crate::convert::KeyOf = {
        let (door_dir, late_dir) = (door_dir.clone(), late_dir.clone());
        let (door_in, door_go) = (door_in.clone(), door_go.clone());
        Arc::new(move |p: &std::path::Path| {
            if p == door_dir {
                door_in.open();
                door_go.wait(Duration::from_secs(2));
            } else if p == late_dir {
                std::thread::sleep(Duration::from_secs(2));
            }
            p.to_path_buf()
        })
    };
    let registry = crate::convert::SeverRegistry::new(key_of);
    // A resolved sever in flight, so the door has to compute its key.
    crate::convert::answer_by_in(
        &registry,
        key("mid-first-sever"),
        Duration::from_millis(10),
        slow_core(
            Duration::from_secs(5),
            severed_report(),
            Arc::new(AtomicBool::new(false)),
        ),
    )
    .await
    .unwrap_or_else(|e| panic!("a report: {}", e.code));
    assert_eq!(
        registry.unresolved(),
        0,
        "fixture: the first sever resolved"
    );

    let door = {
        let (registry, door_dir) = (registry.clone(), door_dir.clone());
        tokio::spawn(async move {
            registry
                .refuse(&door_dir, rw::WalletError::WalletAlreadyOpen)
                .await
        })
    };
    until("the door is inside its key", || door_in.is_open()).await;
    let late = {
        let registry = registry.clone();
        tokio::spawn(async move {
            crate::convert::answer_by_in(
                &registry,
                late_dir,
                Duration::from_millis(10),
                slow_core(
                    Duration::from_secs(5),
                    severed_report(),
                    Arc::new(AtomicBool::new(false)),
                ),
            )
            .await
        })
    };
    until("the late sever registered unresolved", || {
        registry.unresolved() > 0
    })
    .await;
    door_go.open();
    let answered = door.await.expect("the door's task");
    assert!(
        matches!(answered, Err(rw::WalletError::WalletAlreadyOpen)),
        "a sever registered while the door computed its key: the door must refuse"
    );
    drop(late);
}

/// Interleaving (b): a door arrives while a sever is between its registration
/// and resolving its real-path key. The sever registered FIRST (lexical key,
/// unresolved), so the door refuses at once. It never sees an empty set.
#[tokio::test]
async fn a_door_during_a_severs_key_work_refuses() {
    let sever_dir = key("resolving-sever");
    let sever_in = Arc::new(Latch::default());
    let sever_go = Arc::new(Latch::default());
    let key_of: crate::convert::KeyOf = {
        let sever_dir = sever_dir.clone();
        let (sever_in, sever_go) = (sever_in.clone(), sever_go.clone());
        Arc::new(move |p: &std::path::Path| {
            if p == sever_dir {
                sever_in.open();
                sever_go.wait(Duration::from_secs(2));
            }
            p.to_path_buf()
        })
    };
    let registry = crate::convert::SeverRegistry::new(key_of);
    let sever = {
        let registry = registry.clone();
        tokio::spawn(async move {
            crate::convert::answer_by_in(
                &registry,
                sever_dir,
                Duration::from_secs(3),
                slow_core(
                    Duration::from_secs(5),
                    severed_report(),
                    Arc::new(AtomicBool::new(false)),
                ),
            )
            .await
        })
    };
    until("the sever is inside its key", || sever_in.is_open()).await;
    let answered = registry
        .refuse(&key("resolving-door"), rw::WalletError::WalletOpen)
        .await;
    sever_go.open();
    assert!(
        matches!(answered, Err(rw::WalletError::WalletOpen)),
        "a door must refuse while a sever resolves its key"
    );
    drop(sever);
}

// ---- the rows' registries -------------------------------------------------

thread_local! {
    /// Each row's OWN registry. libtest runs every row on a thread of its own,
    /// and the rows' `#[tokio::test]` runtimes are current-thread. Private
    /// registries keep the rows from reading each other's severs; the
    /// process-wide one refuses every door while ANY sever registers.
    static ROW_REGISTRY: std::sync::Arc<crate::convert::SeverRegistry> =
        crate::convert::SeverRegistry::new(Arc::new(crate::convert::sever_key));
}

/// `convert::answer_by`, over this row's own registry.
fn answer_by<F>(
    dir: std::path::PathBuf,
    deadline: Duration,
    core: F,
) -> impl std::future::Future<Output = Result<api_state::SeverReport, crate::api::error::WalletApiError>>
+ Send
+ 'static
where
    F: std::future::Future<Output = Result<rw::SeverReport, rw::WalletError>> + Send + 'static,
{
    let registry = ROW_REGISTRY.with(Arc::clone);
    async move { crate::convert::answer_by_in(&registry, dir, deadline, core).await }
}

/// Is a sever of `dir` in flight in this row's own registry?
fn sever_in_flight(dir: &std::path::Path) -> bool {
    ROW_REGISTRY.with(|r| r.holds(dir))
}

/// [`in_flight_for`] in the PROCESS-WIDE registry, which the doors consult.
async fn in_flight_in_the_process(dir: &std::path::Path, for_: Duration) {
    let first = crate::convert::answer_by(
        dir.to_path_buf(),
        Duration::from_millis(10),
        slow_core(for_, severed_report(), Arc::new(AtomicBool::new(false))),
    )
    .await
    .unwrap_or_else(|e| panic!("a report: {}", e.code));
    assert_eq!(outcome_label(first.severed), "notSevered(stillRunning)");
}
