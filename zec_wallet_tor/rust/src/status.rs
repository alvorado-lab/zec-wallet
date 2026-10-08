//! The plugin's typed status and configuration (`tor-plugin.md` §2) — every
//! field a closed value or a small integer. No path, no bridge line, no key and
//! no destination ever appears here: the status crosses to Dart as the
//! fixed-width `zwt_status` (`abi.rs`), and the Dart side owns the code → name
//! tables.

use zeroize::Zeroizing;

use crate::constants::{CLASS_BOOTSTRAP_DEADLINE, CLASS_CIRCUITS_FAILING, CLASS_NOT_REGISTERED};

/// What the plugin is doing, as the host's Dart reads it. A CLOSED set; no
/// free text beside it. Crosses as `ZWT_PHASE_*`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    /// `init` has not run, or it failed before registration.
    Idle,
    /// Registered; arti is bootstrapping (readiness < 100).
    Bootstrapping,
    /// Registered; arti is ready for traffic (readiness 100).
    Ready,
    /// Registered; the bootstrap attempt ended without readiness — the deadline
    /// elapsed or arti reported failure — and the plugin waits out its retry
    /// backoff (or a manual `retryBootstrap`). Readiness < 100.
    Failed,
    /// The app is paused: readiness pushed to SUSPENDED, arti dormant. Left on
    /// `resumed`.
    Suspended,
    /// `init` was refused at the crossing (slot occupied, ABI mismatch, the
    /// wallet image not loaded) or `dispose` ran. Arti was never started or has
    /// been torn down; a later `init` registers again.
    NotRegistered,
}

/// What blocks the bootstrap, in arti's own terms: a 1:1 CLOSED mirror of
/// `arti_client::status::BlockageKind` at the pinned version, plus `Unknown`
/// for its `#[non_exhaustive]` tail. P24 pins the list against the locked
/// arti source. Rendered by the host; never interpreted by the plugin beyond
/// "not ready". Crosses as `ZWT_BLOCKAGE_*`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Blockage {
    /// Bootstrap not yet asked for (`BootstrapBehavior::Manual`).
    Disabled,
    /// The device appears offline.
    Offline,
    /// "Our connections seem to be filtered."
    Filtering,
    /// Tor itself cannot be reached.
    CantReachTor,
    /// The device clock is skewed.
    ClockSkewed,
    /// A directory problem.
    CantBootstrap,
    /// A kind this plugin's pinned arti did not have — the `#[non_exhaustive]`
    /// tail. Never a panic, never a guess.
    Unknown,
}

impl Blockage {
    /// Every variant, in the order the C ABI numbers them (P24 and P14 read
    /// this list; a variant added here without a code is a red test).
    pub const ALL: [Self; 7] = [
        Self::Disabled,
        Self::Offline,
        Self::Filtering,
        Self::CantReachTor,
        Self::ClockSkewed,
        Self::CantBootstrap,
        Self::Unknown,
    ];

    /// arti's variant NAME this one mirrors, or `None` for [`Self::Unknown`].
    /// P24 compares this against the declaration in the locked arti source.
    pub fn arti_name(self) -> Option<&'static str> {
        match self {
            Self::Disabled => Some("Disabled"),
            Self::Offline => Some("Offline"),
            Self::Filtering => Some("Filtering"),
            Self::CantReachTor => Some("CantReachTor"),
            Self::ClockSkewed => Some("ClockSkewed"),
            Self::CantBootstrap => Some("CantBootstrap"),
            Self::Unknown => None,
        }
    }

    /// The closed mirror of arti's value. The wildcard arm is the one
    /// `#[non_exhaustive]` forces on a downstream crate; P24 is what turns a
    /// new upstream variant into a red test rather than a silent `Unknown`.
    pub fn from_arti(kind: &dialer_tor::BlockageKind) -> Self {
        use dialer_tor::BlockageKind as B;
        match kind {
            B::Disabled => Self::Disabled,
            B::Offline => Self::Offline,
            B::Filtering => Self::Filtering,
            B::CantReachTor => Self::CantReachTor,
            B::ClockSkewed => Self::ClockSkewed,
            B::CantBootstrap => Self::CantBootstrap,
            _ => Self::Unknown,
        }
    }
}

/// Every failure class the status may carry, in the order the C ABI numbers
/// them (`ZWT_CLASS_*`, 0 = none; append-only — a code once shipped never
/// changes meaning). The plugin's own two; `dialer-tor`'s bootstrap/setup
/// classes; then EVERY bridge class `dialer-tor` can refuse a paste with — its
/// five exported constants and the ten `bridge_class()` names it mints 1:1 from
/// upstream's parse errors. All fifteen, not a folded few: the censorship
/// spec's rule is one sentence per refusal, and a class folded to "unusable"
/// here would take that sentence away from the host (the C3b plan's finding,
/// The ten without constants are bound to `dialer-tor`'s own source by
/// `every_class_dialer_tor_can_refuse_with_has_a_code`. The Dart side owns the
/// code → name table. Appended at C3b: the liveness rule's own class (21).
pub const FAILURE_CLASSES: [&str; 21] = [
    CLASS_BOOTSTRAP_DEADLINE,
    CLASS_NOT_REGISTERED,
    "bootstrap-failed",
    "setup",
    "not-bootstrapped",
    dialer_tor::CLASS_CONFIG_TOO_LONG,
    dialer_tor::CLASS_TOO_MANY_LINES,
    dialer_tor::CLASS_LINE_TOO_LONG,
    dialer_tor::CLASS_PT_UNSUPPORTED,
    dialer_tor::CLASS_UNUSABLE,
    "bridge-line-empty",
    "bridge-invalid-transport-or-address",
    "bridge-invalid-address",
    "bridge-invalid-identity",
    "bridge-duplicate-identity",
    "bridge-unsupported-identity-type",
    "bridge-unsupported-channel-method",
    "bridge-direct-parameters-not-allowed",
    "bridge-no-rsa-identity",
    "bridge-support-disabled",
    CLASS_CIRCUITS_FAILING,
];

/// The typed status. No path, no bridge, no key, no destination.
#[derive(Clone, PartialEq, Debug)]
pub struct Status {
    /// What the plugin is doing.
    pub phase: Phase,
    /// The readiness LAST PUSHED to the wallet (0..=100) — what the chip shows.
    pub readiness: u32,
    /// What blocks the bootstrap, when arti says.
    pub blockage: Option<Blockage>,
    /// One of [`FAILURE_CLASSES`] after a failure or a refusal: a bootstrap
    /// that failed or timed out, a registration the wallet refused, or a
    /// bridge paste refused — the last even while the phase is unchanged
    /// (`setBridges` leaves a running client as it was; the header's
    /// `ZWT_RC_BRIDGES_REFUSED` says "the class is in the status"). `None`
    /// otherwise.
    pub failure_class: Option<&'static str>,
}

/// The plugin's configuration as `init` receives it.
pub struct Config {
    /// A directory of the PLUGIN'S OWN — a SIBLING of the wallet's `db_dir`,
    /// never `db_dir` itself and never inside it (the wallet's wipe sweeps
    /// `db_dir` whole, §2.3). Absolute; relative or empty is refused typed.
    /// NEVER logged (a device path is PII).
    pub tor_dir: String,
    /// Bridge lines as pasted, `None` = arti's directory bootstrap. Kept out of
    /// logs and UI; OUR copy zeroizes, arti's plaintext ones do not.
    pub bridges: Option<Zeroizing<String>>,
}

impl std::fmt::Debug for Config {
    /// Neither field is printable: the directory is a device path, and a bridge
    /// line names a bridge.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("tor_dir", &"<redacted>")
            .field("bridges", &self.bridges.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use dialer_tor::{ErrorKind, TorDialError};

    use super::*;

    /// The `dialer-tor` classes the status table carries are `dialer-tor`'s
    /// OWN strings — consumed, never re-minted. The five bridge classes are
    /// its constants (compared by value in `FAILURE_CLASSES` itself); the
    /// three it only exposes through `TorDialError::class()` are bound here to
    /// that function's answer on constructed variants.
    #[test]
    fn the_failure_classes_are_dialer_tors_own_strings() {
        let constructed = [
            TorDialError::BootstrapFailed {
                kind: ErrorKind::Other,
                blockage: None,
            }
            .class(),
            TorDialError::Setup {
                kind: ErrorKind::Other,
            }
            .class(),
            TorDialError::NotBootstrapped {
                progress: 0.0,
                blockage: None,
            }
            .class(),
        ];
        assert_eq!(&FAILURE_CLASSES[2..5], &constructed);
        let mut sorted = FAILURE_CLASSES.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            FAILURE_CLASSES.len(),
            "the class table has a duplicate"
        );
    }

    /// Every bridge class `dialer-tor` can refuse a paste with has a code here
    /// — read from `dialer-tor`'s own `bridges.rs` (the path dependency cargo
    /// compiles), every `"bridge-…"` string literal: its exported constants and
    /// the names `bridge_class()` mints per upstream parse error. A class
    /// `dialer-tor` adds tomorrow is a red test here, never a refusal the host
    /// receives with no class (the C3b plan's finding).
    #[test]
    fn every_class_dialer_tor_can_refuse_with_has_a_code() {
        let src = include_str!("../../../dialer-tor/src/bridges.rs");
        // Only where a class is EMITTED: the `pub const CLASS_*` declarations
        // and the arms of `bridge_class()`. A whole-file scan read a cargo
        // feature name in a doc comment (`"bridge-client"`) as a class — its
        // first run, watched.
        fn quoted(line: &str) -> Option<&str> {
            let at = line.find('"')?;
            let rest = &line[at + 1..];
            Some(&rest[..rest.find('"')?])
        }
        let consts = src
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with("pub const CLASS_") && line.contains("&str ="))
            .filter_map(quoted);
        let start = src
            .find("pub fn bridge_class(")
            .expect("dialer-tor declares bridge_class");
        let body = &src[start..];
        let body = &body[..body.find("\n}\n").expect("bridge_class closes")];
        let arms = body
            .lines()
            .filter_map(|line| line.split_once("=>").map(|(_, rhs)| rhs))
            .filter_map(quoted);
        let mut emitted: Vec<&str> = consts.chain(arms).collect();
        emitted.sort_unstable();
        emitted.dedup();
        assert!(
            emitted.len() >= 15,
            "anti-vacuity: read {} bridge classes from dialer-tor's bridges.rs ({emitted:?}) — \
             not the file it thinks it is",
            emitted.len()
        );
        let missing: Vec<&&str> = emitted
            .iter()
            .filter(|c| !FAILURE_CLASSES.contains(c))
            .collect();
        assert!(
            missing.is_empty(),
            "dialer-tor can refuse a bridge paste with class(es) {missing:?} that have no \
             ZWT_CLASS_* code — the host would receive the refusal with no reason. Append them to \
             FAILURE_CLASSES and to include/zec_wallet_tor.h (append-only)."
        );
    }

    /// The `arti-client` version this crate's lock pins.
    fn locked_arti_version() -> String {
        let lock =
            std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock"))
                .expect("the plugin's Cargo.lock is readable (it is committed)");
        let at = lock
            .find("name = \"arti-client\"\nversion = \"")
            .expect("arti-client is locked");
        let rest = &lock[at + "name = \"arti-client\"\nversion = \"".len()..];
        rest[..rest.find('"').expect("the version closes")].to_string()
    }

    /// The pinned arti-client's `src/status.rs`, from the registry source cargo
    /// compiled (`$CARGO_HOME/registry/src/<index>/arti-client-<locked>/`).
    fn locked_arti_status_rs() -> (PathBuf, String) {
        let version = locked_arti_version();
        let home = std::env::var_os("CARGO_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("HOME").expect("HOME is set")).join(".cargo")
            });
        let src = home.join("registry").join("src");
        let dirs = std::fs::read_dir(&src).unwrap_or_else(|e| panic!("{}: {e}", src.display()));
        for index in dirs.flatten() {
            let candidate = index
                .path()
                .join(format!("arti-client-{version}/src/status.rs"));
            if let Ok(text) = std::fs::read_to_string(&candidate) {
                return (candidate, text);
            }
        }
        panic!(
            "arti-client-{version}'s source is not under {} — build the crate first",
            src.display()
        );
    }

    /// FR-5 spec §8 **P24**: the plugin's `Blockage` mirrors
    /// `arti_client::status::BlockageKind` ONE TO ONE, read from the
    /// declaration in the LOCKED arti source (Relim's "the table is the
    /// upstream declaration" pattern) — so an arti upgrade that adds a kind is
    /// a red test here, not a silent `Unknown` on a device.
    #[test]
    fn blockage_kinds_are_mirrored_one_to_one_from_the_pinned_arti_source() {
        let (path, text) = locked_arti_status_rs();
        let start = text
            .find("pub enum BlockageKind {")
            .unwrap_or_else(|| panic!("{} declares no BlockageKind", path.display()));
        let body = &text[start..];
        let body = &body[..body.find("\n}\n").expect("the enum closes")];
        let declared: Vec<String> = body
            .lines()
            .skip(1)
            .filter_map(|line| {
                let rest = line.strip_prefix("    ")?;
                if rest.starts_with(' ') || rest.starts_with('#') || rest.starts_with("//") {
                    return None;
                }
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric())
                    .collect();
                name.chars()
                    .next()
                    .filter(char::is_ascii_uppercase)
                    .map(|_| name)
            })
            .collect();
        assert!(
            declared.len() >= 5,
            "P24 anti-vacuity: read {declared:?} from {} — not the enum it thinks it is",
            path.display()
        );
        let mirrored: Vec<&str> = Blockage::ALL.iter().filter_map(|b| b.arti_name()).collect();
        assert_eq!(
            mirrored,
            declared.iter().map(String::as_str).collect::<Vec<_>>(),
            "P24: the plugin's Blockage does not mirror arti's BlockageKind at {} one to one (in \
             declaration order) — add the new kind, its ZWT_BLOCKAGE_* code and its Dart name",
            path.display()
        );
        assert_eq!(
            Blockage::ALL.last(),
            Some(&Blockage::Unknown),
            "Unknown is the tail"
        );
    }
}
