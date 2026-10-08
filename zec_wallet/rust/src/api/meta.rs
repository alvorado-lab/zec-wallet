//! SDK metadata + the FRB init hook.

use crate::frb_generated::StreamSink;

/// SDK version (core + bridge build in lockstep — one workspace version).
/// For host support/diagnostics logging; carries no user or wallet data.
#[flutter_rust_bridge::frb(sync)]
pub fn sdk_version() -> String {
    // SCAN-1 §4o S5: the measurement build marks ITSELF, so
    // an artifact that logs is never mistaken for a release one — `+devtiming`
    // is semver build metadata, appended only under the door's cfg and never in
    // a default build (`sdk_version_carries_no_devtiming_marker_by_default`).
    let base = env!("CARGO_PKG_VERSION");
    if cfg!(zec_wallet_device_timing) {
        format!("{base}+devtiming")
    } else {
        base.to_string()
    }
}

/// How long after its `deadlineMs` a `WalletHandle.severCustody` call may take
/// to ANSWER, in whole milliseconds: the hard bound on the whole call is
/// `deadlineMs + severAnswerGraceMs()`. A host budgets its duress sequence
/// with it (a constant of this build).
#[flutter_rust_bridge::frb(sync)]
pub fn sever_answer_grace_ms() -> u32 {
    crate::convert::SEVER_ANSWER_GRACE_MS
}

/// How much of the SDK's own diagnostics the HOST lets reach the device log
/// (FR-35). The three settings are the maintainer's (2026-09-19): *"'Off' /
/// 'Errors only' / 'Detailed logging'"*. Which of them is the DEFAULT in a
/// debug build, in production, or for a given user is the HOST's decision and
/// lives in a setting its user can see; this SDK's own default is [`Off`].
///
/// There is no level above [`Detailed`]: a setting a host could raise further
/// would be a second road to DEBUG on a user's phone, and the measurement
/// build is already the developer's door for that.
///
/// [`Off`]: DeviceLogLevel::Off
/// [`Detailed`]: DeviceLogLevel::Detailed
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceLogLevel {
    /// Nothing is written. What a host that says nothing gets.
    Off,
    /// WARN and above: something went wrong and the wallet carried on — a
    /// private path that would not connect, a fall-back to clearnet, a stalled
    /// sync. Not the routine lines.
    Errors,
    /// INFO and above: `Errors`, plus the connectivity narrative — the host
    /// transport's lifecycle and one `wallet.dial` line per dial.
    Detailed,
}

/// Set how much the SDK writes to the DEVICE LOG — the HOST's switch (FR-35),
/// reflecting a choice the host's user can see. **OFF until the host calls
/// this**: a host that says nothing gets no device log. Call it right after
/// `RustLib.init()`, before anything touches the wallet, with the user's current
/// choice, and again whenever that choice changes — it takes effect for the next
/// event, with no re-init, is idempotent, and is safe from any thread.
///
/// What is written is this SDK's own `tracing` events, events only, to the
/// platform log — Android logcat, tags `zec_wallet_core` and `zec_wallet`
/// (`adb logcat -s zec_wallet_core:V zec_wallet:V`); Apple's unified log,
/// subsystem `zec_wallet` with those two as categories (`log stream --level
/// info --predicate 'subsystem == "zec_wallet"'`, or Console.app), where the
/// lines are PERSISTED to disk for days; Linux and Windows, the process's
/// stderr, one `<LEVEL> <line>` per event — with the §5.4 never-log
/// policy ENFORCED PER FIELD AT RUNTIME: a field outside the allowlist is
/// withheld and the line says so (`withheld=<n>`). It carries connectivity —
/// never wallet contents, never a server's name, and never a dependency
/// crate's events. What it DOES say, and a host's consent copy should too: that
/// a private transport is in use, how ready and healthy it reports itself, and
/// which arm — `private` or `clearnet` — performed each dial.
///
/// RETURNS the EFFECTIVE level after the call: the one asked for where
/// something here can write it, otherwise [`DeviceLogLevel::Off`] — a platform
/// with no sink (anything but Android, Apple, Linux and Windows), or a process
/// whose global `tracing` default another library already owns. Render THIS in
/// a settings row, not the setting, so the row cannot claim a log that does not
/// exist.
///
/// It does not govern a debug or measurement BUILD's own louder layer: that is a
/// developer's build-time door, never a distributed artifact.
#[flutter_rust_bridge::frb(sync)]
pub fn set_device_log(level: DeviceLogLevel) -> DeviceLogLevel {
    crate::device_log::set_level(level)
}

/// The EFFECTIVE device-log level right now — what [`set_device_log`] last
/// asked for, where it can actually be written; otherwise
/// [`DeviceLogLevel::Off`]. Changes nothing.
///
/// `Off` means THIS switch writes nothing. It does not describe a debug or
/// measurement BUILD's own louder layer, which logs whatever this reports — a
/// developer's build-time door, never a distributed artifact.
#[flutter_rust_bridge::frb(sync)]
pub fn device_log_level() -> DeviceLogLevel {
    crate::device_log::effective_level()
}

/// How serious one device-log line is — the event's own level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceLogSeverity {
    /// An ERROR event.
    Error,
    /// A WARN event: something went wrong and the wallet carried on.
    Warn,
    /// An INFO event: the connectivity narrative (`Detailed` only).
    Info,
}

/// One line of the SDK's device log, as [`watch_device_log`] delivers it.
///
/// `text` is exactly the line the platform log is handed for the same event,
/// before any per-platform prefix: printable ASCII, at most 512 bytes (then
/// `...`), with every field outside the §5.4 allowlist withheld and counted
/// (`withheld=<n>`). No timestamp: stamp it on receipt.
///
/// `text` and `tag` are CONTROL-safe, not DELIMITER-safe: they hold no newline
/// or other control character, so they cannot forge a second line in a
/// line-oriented log, but they can hold `"`, `\`, `,`, `|`, `=` and spaces. A
/// host that puts them into JSON, CSV or any other structured format must
/// escape them for that format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceLogLine {
    /// Counts every line the log produced while it was on, delivered or not,
    /// so within one subscription a gap is a line this stream did not get (the
    /// host budget, or no stream registered at that moment). Restarts at 0 when
    /// a wipe begins. It cannot show lines posted to an isolate that died
    /// before reading them.
    pub seq: u64,
    pub severity: DeviceLogSeverity,
    /// `zec_wallet_core` or `zec_wallet` — which half of the SDK emitted it.
    pub tag: String,
    pub text: String,
}

/// Stream the SDK's device-log lines to the host: the same lines, under the
/// same [`set_device_log`] switch, that the platform log gets — nothing while
/// it is `Off`, WARN and above at `Errors`, INFO and above at `Detailed`. Call
/// it after `RustLib.init()`, beside `setDeviceLog`, and KEEP DRAINING the
/// stream: the SDK keeps no queue of its own, and sends at most 60 lines a
/// minute to the host (a line over that still reaches the platform log, and
/// leaves a gap in `seq`).
///
/// ONE subscriber per process: a second call replaces the first, whose stream
/// closes. That is how a hot restart or a re-subscribe after unlock takes over;
/// a host with two consumers fans out on its own side.
///
/// Any `wipe`, `wipeForce` or `severCustody` call — a failed one included —
/// closes this stream and sets the device log `Off` before it does anything
/// else, and no line is sent after that. Nothing re-opens it on its own: the
/// host re-arms with `setDeviceLog` and `watchDeviceLog`, except on a duress
/// path, where it must not.
///
/// The stream never errors, even when nothing can feed it. So after
/// `setDeviceLog`, read `deviceLogLevel()`. If it answers `Off` although you
/// asked for more, no layer is installed (no platform sink, or another
/// library took the process's `tracing` default first), and this stream will
/// stay silent.
pub fn watch_device_log(sink: StreamSink<DeviceLogLine>) {
    crate::device_log::watch_process(sink);
}

/// FRB runtime init. Runs once from `RustLib.init()` before any other call.
///
/// EVERY BUILD (maintainer ruling 2026-09-19 — *"sdk definitely should include
/// logging layer so you will be able to debug connectivity first on the android.
/// and then on iphone"*): installs the device-log layer (`crate::device_log`)
/// with its gate CLOSED — nothing is written until the host calls
/// [`set_device_log`] (FR-35: the host switches it; the SDK never decides on its
/// own to write to a user's device log). It is safe to carry in every build
/// because it ENFORCES the §5.4 allowlist at runtime: a field the capture guard
/// would fail a test over is withheld from the line (`withheld=<n>`), on every
/// path, driven by a test or not. Until it existed a connectivity report from a
/// tester's phone was unfalsifiable — the only layer was the debug one below,
/// and `debug_assertions` can never be true in a build anyone ships. Four
/// platforms have a sink: Android (logcat) and Apple (the unified log — iOS and
/// macOS), in the maintainer's order, and Linux and Windows (stderr, FR-42).
/// Elsewhere nothing is installed and the host's verb answers `Off`.
///
/// What it still does NOT do: mutate the host environment — specifically NOT
/// FRB's `setup_default_user_utils()` (a TRACE-level platform-console sink +
/// process-wide `RUST_BACKTRACE=1`, a standing §5.4 never-log risk). Rust panics
/// still surface as Dart exceptions through FRB's built-in panic capture. And it
/// never routes a DEPENDENCY's events: the crates that log addresses and txids
/// are refused at the callsite (`device_log::prints`).
///
/// DEBUG (D-2b-2, maintainer-requested on-device debugging): a debug build ALSO
/// routes the core's `tracing` events to Android logcat at DEBUG with span
/// closes (tag `zec_wallet_core`) so an on-device failure — e.g. a sync transport
/// error otherwise reduced to a coarse stall — is visible in full.
/// `cfg!(debug_assertions)` is compile-time and can NEVER be true in a
/// profile/release build, so only a developer's local debug build carries that
/// louder, UNENFORCED fmt layer (the measurement build is not a distributed
/// artifact — refused as a release configuration, self-marked `+devtiming`).
/// Beside it the device-log layer leaves the core's module target alone, so no
/// INFO event prints twice. A double-init (hot restart) is a no-op: the global
/// default can be set once and the second attempt's error is discarded.
///
/// MEASUREMENT (SCAN-1, §4o S5): the SAME layer, in a `release` `.so`, behind a
/// compile-time door that is OFF in every default build —
/// `cfg(zec_wallet_device_timing)`, emitted by this crate's `build.rs` only when
/// `ZEC_WALLET_DEVICE_TIMING=1` is in the build's environment AND the build is
/// not a release configuration (`CARGOKIT_CONFIGURATION`, or cargo's `PROFILE`
/// outside cargokit — a release build with the variable set FAILS to build;
/// `just wallet-device-timing-apk` is the profile build that passes). Door (b)
/// of the contract: per build, crate-local, nothing committed that arms a
/// consumer (door (a) — a `cargokit.yaml` `extra_flags` feature — ships in the
/// pub.dev package and would arm every consumer's profile build). Both doors
/// call ONE install function; neither is a copy of the other. The cfg is never
/// true in a build nobody asked for it in: `tests/extraction_policy.rs::
/// the_device_timing_door_is_closed_in_every_default_build` pins the door's
/// shape, and `just wallet-device-timing-negative-witness` proves a door-closed
/// profile `.so` carries no span-close layer.
#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    #[cfg(any(
        target_os = "android",
        target_vendor = "apple",
        target_os = "linux",
        target_os = "windows"
    ))]
    install_device_log();
}

/// Install the process's ONE `tracing` subscriber: the host-switched device-log
/// layer, gate CLOSED, and — only behind the debug/measurement door — the loud
/// fmt layer beside it. One registry, because a process has one global default
/// and the second `set_global_default` loses; installed at init rather than at
/// the host's first `set_device_log(true)` for the same reason — a door-open
/// build has already taken the one slot by then.
///
/// `set_global_default`, not `try_init`: `try_init` is that call AND a
/// `LogTracer` install whenever feature unification turns `tracing-log` on, and
/// one `Result` then covers two installs of which the second must fail on a
/// host that owns the `log` facade. This names the one thing it does.
#[cfg(target_os = "android")]
fn install_device_log() {
    use crate::device_log::{
        FanOut, LogcatSink, Scope, device_log_layer, mark_installed, process_gate,
        process_host_slot,
    };
    use tracing_subscriber::layer::SubscriberExt;

    let gate = process_gate().clone();
    // The scope is the PLATFORM arm's: the layer's filter takes every SDK
    // target in both arms, so the host stream gets the core's module target
    // even where logcat leaves it to the loud layer.
    #[cfg(any(debug_assertions, zec_wallet_device_timing))]
    let subscriber = tracing_subscriber::registry()
        .with(loud_logcat_layer())
        .with(device_log_layer(
            FanOut {
                platform: LogcatSink::new(),
                platform_scope: Scope::BesideTheLoudLayer,
                host: process_host_slot().clone(),
            },
            gate,
        ));
    #[cfg(not(any(debug_assertions, zec_wallet_device_timing)))]
    let subscriber = tracing_subscriber::registry().with(device_log_layer(
        FanOut {
            platform: LogcatSink::new(),
            platform_scope: Scope::AllSdkTargets,
            host: process_host_slot().clone(),
        },
        gate,
    ));
    // `Ok` is the only proof a line can be written, so it alone lets
    // `device_log_level` report anything but `Off`. `Err` has two causes and they
    // differ: a second init (hot restart) finds OUR subscriber already in place —
    // still marked from the first — while a process whose default another
    // library took first never gets marked, and the host is told the truth: `Off`.
    if tracing::subscriber::set_global_default(subscriber).is_ok() {
        mark_installed();
    }
}

/// [`install_device_log`] for APPLE targets (iOS and macOS): the same layer, the
/// same process gate, the same "installed only if the default was really taken"
/// — over the unified-log sink. There is no loud layer here (the debug and
/// measurement doors are Android's), so the device log covers every SDK target
/// in every Apple build, debug included, at whatever level the HOST sets.
///
/// If the OS hands out no log handle the install is skipped whole: the wallet
/// starts, and `device_log_level` says `Off` — a diagnostic must never cost the
/// wallet its init (`OsLogSink::new` fences the shim's assertion).
#[cfg(target_vendor = "apple")]
fn install_device_log() {
    use crate::device_log::{
        FanOut, OsLogSink, Scope, device_log_layer, mark_installed, process_gate, process_host_slot,
    };
    use tracing_subscriber::layer::SubscriberExt;

    let Some(sink) = OsLogSink::new() else {
        return;
    };
    let gate = process_gate().clone();
    let subscriber = tracing_subscriber::registry().with(device_log_layer(
        FanOut {
            platform: sink,
            platform_scope: Scope::AllSdkTargets,
            host: process_host_slot().clone(),
        },
        gate,
    ));
    if tracing::subscriber::set_global_default(subscriber).is_ok() {
        mark_installed();
    }
}

/// [`install_device_log`] for LINUX and WINDOWS (S5 `stderr`, FR-42): the Apple
/// shape over stderr — the same layer, process gate and host slot, every SDK
/// target, no loud layer. A desktop host that loads this library as its own
/// `.so`/`.dll` (the cargokit build) gives it its own `tracing-core`, so the
/// global default taken here does not meet a host's `fmt::init()`. A host that
/// statically links the `staticlib` output beside another `tracing-core` user
/// can share one, and then the first install wins. That is why the `is_ok()`
/// check below is load-bearing: the loser answers an honest `Off`.
///
/// It is NOT folded into one helper with Apple's install, although the tails
/// match. `extraction_policy.rs::the_device_log_is_installed_closed_in_every_build`
/// reads each arm's own `FanOut` literal, process slot and mark-on-success.
/// A shared helper would move those out of the arms and need that pin
/// rewritten, to save about ten lines. Android's two arms could not use it
/// anyway, because of the loud layer.
///
/// The cost, stated: the layer's filter admits every SDK INFO-and-above
/// callsite, so a desktop build now evaluates those events' fields even while
/// the gate is `Off` — as Android and Apple have since an earlier revision.
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn install_device_log() {
    use crate::device_log::{
        FanOut, Scope, StderrSink, device_log_layer, mark_installed, process_gate,
        process_host_slot,
    };
    use tracing_subscriber::layer::SubscriberExt;

    let gate = process_gate().clone();
    let subscriber = tracing_subscriber::registry().with(device_log_layer(
        FanOut {
            platform: StderrSink::new(),
            platform_scope: Scope::AllSdkTargets,
            host: process_host_slot().clone(),
        },
        gate,
    ));
    if tracing::subscriber::set_global_default(subscriber).is_ok() {
        mark_installed();
    }
}

/// The ONE loud logcat layer — built for the debug door and for the measurement
/// door (SCAN-1 S5: one function, called under one `cfg(any(..))`; never a copy
/// per door, so the two builds cannot drift in scope or level).
///
/// SCOPED to the `zec_wallet_core` target ONLY. Without its target filter
/// the registry would capture EVERY crate at EVERY level — including the
/// `zcash_client_backend`/`zcash_client_sqlite`/`h2`/`tonic` deps, which log
/// ADDRESSES, TXIDS, and note commitments at debug/trace/info (a §5.4 leak to
/// logcat even in a debug build — a debug build is still a real wallet on a real
/// device). Our own `zec_wallet_core`-targeted sites are §5.4-disciplined (the
/// `tracing_guard` allowlist), so capturing only that target — and dropping all
/// dependency-crate events — keeps the log useful AND never-log-safe. (This also
/// drops the noisy h2 frame trace.) The measurement build keeps exactly this
/// scope and `LevelFilter::DEBUG` (§4o S8). The target is matched on a `::`
/// boundary (`device_log::is_core_module`), so a future `zec_wallet_core_*`
/// target does NOT inherit this layer by borrowing the prefix — until an earlier revision it
/// would have: the filter was `Targets`, which matches by bare `starts_with`.
///
/// `.with_span_events(FmtSpan::CLOSE)` (SCAN-1, §4o IT-1b item 1): a plain fmt
/// layer prints EVENTS only, and an Advance batch fires none — the per-batch
/// timings ride the `wallet.sync` SPAN — so every `zec_wallet_core` span's close
/// is printed as one line carrying its recorded fields. For `wallet.sync` that is
/// `from`/`to`/`blocks`/`outcome`/`anchor_ms`/`dl_ms`/`scan_ms`/`snap_ms`/
/// `chain_outputs`: one INFO line per batch (~2,074 for six months). It is EVERY
/// span, not only that one — today `wallet.rescan` (`from`, `outcome`),
/// `wallet.swap_quote` and `wallet.swap` (`provider`, `direction`, `outcome`) and
/// `wallet.swap_poll` (`provider`, `outcome`, `faults`), each field allowlisted
/// in `tracing_guard` — plus `time.busy` / `time.idle`, which the fmt layer
/// SYNTHESIZES at close: they are not tracing fields, the capture guard cannot
/// see them and no allowlist entry names them; they are durations. Scoping the
/// close lines to `wallet.sync` alone would take a per-span filter on this layer,
/// which would also drop those spans' fields from the context of the events
/// inside them — the debug log's reason to exist — so the doc enumerates instead.
#[cfg(all(target_os = "android", any(debug_assertions, zec_wallet_device_timing)))]
fn loud_logcat_layer<S>() -> impl tracing_subscriber::Layer<S>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    use tracing_subscriber::Layer; // brings `.with_filter` into scope
    use tracing_subscriber::filter::{LevelFilter, filter_fn};
    use tracing_subscriber::fmt::format::FmtSpan;
    // The SAME target predicate the device log EXCLUDES beside this layer
    // (`device_log::is_core_module`), so no target is printed by both or by
    // neither. It replaced `Targets::new().with_target("zec_wallet_core", DEBUG)`
    // `Targets` matches by bare PREFIX, so a future
    // `zec_wallet_core_x` would have reached this UNENFORCED layer while the
    // `::`-boundary predicate next door left it alone. Same scope for every
    // target that exists today, same level, spans as well as events.
    paranoid_android::layer("zec_wallet_core")
        .with_span_events(FmtSpan::CLOSE)
        .with_filter(
            filter_fn(|meta| {
                *meta.level() <= tracing::Level::DEBUG
                    && crate::device_log::is_core_module(meta.target())
            })
            .with_max_level_hint(LevelFilter::DEBUG),
        )
}

#[cfg(test)]
mod tests {
    /// FR-35, the HONEST half of the host's verb: on a platform where nothing
    /// is installed to write through (this host — the test binary never runs
    /// `init_app`'s Android install), asking for ON is recorded and answered
    /// truthfully: NOT active. A settings row rendering the return cannot claim
    /// a log that does not exist. Since S5 every wipe writes the process gate
    /// too (it quiesces the log), so this holds the process log's test lock; it
    /// leaves the gate as it found it — closed.
    ///
    /// Watched against: `set_level` returning its argument.
    #[test]
    fn asking_for_a_log_nothing_can_write_is_answered_off() {
        use super::DeviceLogLevel::{Detailed, Errors, Off};
        let _serial = crate::device_log::test_sink::PROCESS_LOG.blocking_lock();
        let gate = crate::device_log::process_gate();
        assert_eq!(
            super::device_log_level(),
            Off,
            "OFF until the host says otherwise"
        );
        for asked in [Detailed, Errors] {
            assert_eq!(
                super::set_device_log(asked),
                Off,
                "asked for {asked:?}, but no subscriber of ours is installed in this process"
            );
            assert_eq!(
                gate.level(),
                asked,
                "the host's choice is still recorded — a sink installed later reads this gate"
            );
            assert_eq!(super::device_log_level(), Off);
        }
        assert_eq!(super::set_device_log(Off), Off);
        assert_eq!(gate.level(), Off);
    }

    #[test]
    fn sdk_version_carries_no_devtiming_marker_by_default() {
        // §4o S5: a default build — no ZEC_WALLET_DEVICE_TIMING
        // in its environment — reports the package version, nothing appended.
        // Mutant: the marker made unconditional → red. (With the variable exported
        // this is not "the default" and fails by design; build.rs refuses that
        // for a release build.)
        let v = super::sdk_version();
        assert_eq!(v, env!("CARGO_PKG_VERSION"));
        assert!(!v.contains("+devtiming"));
    }

    /// The env var that turns [`linux_desktop_detailed_is_answered_and_reaches_stderr`]
    /// into its CHILD role. Only the parent sets it, and only on the child.
    #[cfg(target_os = "linux")]
    const LINUX_RUN_CHILD: &str = "ZEC_WALLET_LINUX_RUN_CHILD";

    /// S5's last exit-gate clause, RUN rather than compiled: on Linux desktop,
    /// `setDeviceLog(detailed)` answers `detailed` and the SDK's lines appear on
    /// stderr. The child calls the real `init_app` — the Linux arm's install over
    /// the real `StderrSink` — asks for `Detailed`, and emits the `wallet.dial`
    /// event exactly as `net/dialer.rs` does; the parent reads the child's real
    /// stderr for the `INFO zec_wallet_core wallet.dial` line.
    ///
    /// Why a child process: the install takes the process's ONE global `tracing`
    /// default, for good. Done in this test binary it would make
    /// [`asking_for_a_log_nothing_can_write_is_answered_off`] — which needs
    /// nothing installed — wrong for whichever test ran after it. The child is
    /// this same binary re-run on this one test (`--exact`), so the install
    /// never reaches the parent's process.
    ///
    /// Watched against: `init_app`'s cfg without `target_os = "linux"`
    /// (`s5-linux-install-cfg-dropped`).
    #[cfg(target_os = "linux")]
    #[test]
    fn linux_desktop_detailed_is_answered_and_reaches_stderr() {
        use super::DeviceLogLevel::Detailed;
        if std::env::var_os(LINUX_RUN_CHILD).is_some() {
            super::init_app();
            assert_eq!(
                super::set_device_log(Detailed),
                Detailed,
                "on Linux the install took the default, so Detailed is answered"
            );
            assert_eq!(super::device_log_level(), Detailed);
            // The production emit (`net/dialer.rs`), fields allowlisted.
            let (dial_arm, dial_class, outcome) = ("host", "sync", "connected");
            tracing::info!(target: "zec_wallet_core", dial_arm, dial_class, outcome, "wallet.dial");
            return;
        }
        let out = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "api::meta::tests::linux_desktop_detailed_is_answered_and_reaches_stderr",
                "--nocapture",
            ])
            .env(LINUX_RUN_CHILD, "1")
            .output()
            .expect("the child runs");
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success(),
            "the child failed: {:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
            out.status
        );
        assert!(
            stdout.contains("1 passed"),
            "the child ran exactly this test: {stdout}"
        );
        assert!(
            stderr
                .lines()
                .any(|l| l.starts_with("INFO zec_wallet_core wallet.dial")),
            "the SDK's line is on the child's stderr:\n{stderr}"
        );
    }

    /// `sdkVersion()` names the release a host logs for support, so it is the
    /// `zec_wallet` package's own version (the Rust workspace version once
    /// stood at 0.1.0 while every package said 0.0.1). The pubspec is beside
    /// `rust/` in the repository and in the staged package alike.
    #[test]
    fn sdk_version_is_the_packages_version() {
        let pubspec = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../pubspec.yaml"),
        )
        .expect("the zec_wallet pubspec is beside rust/");
        let published = pubspec
            .lines()
            .find_map(|l| l.strip_prefix("version:"))
            .map(str::trim)
            .expect("the pubspec declares a version");
        assert_eq!(env!("CARGO_PKG_VERSION"), published);
    }
}
