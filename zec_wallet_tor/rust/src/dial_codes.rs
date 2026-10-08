//! The error table — `dialer_tor::TorDialError` → `ZW_DIAL_*` (`tor-plugin.md`
//! §3.3, D11). ONE `match`, the one home of the mapping; the trampoline (C3b)
//! calls it and restates nothing.
//!
//! The rule behind it: codes 2 (`UNREACHABLE`) and 3 (`TIMEOUT`) are the only
//! two a `Preferred` wallet falls back to clearnet on, so they are reserved for
//! failures the DEVICE, the path to Tor or the Tor network itself produces — a
//! failure the DESTINATION or an EXIT can produce at will never reaches them.
//! In arti 0.45.0 terms (read):
//!
//! * NO arti error kind produces code 2. `LocalNetworkError`'s producer is an
//!   IO error on an already-open GUARD channel — the adversary Tor is deployed
//!   against — while a device with no route produces `TorAccessFailed`.
//!   Code 2 has one source: the plugin's own device-offline derivation (C3b).
//! * `TorNetworkTimeout` — a circuit BUILD timed out inside the Tor network —
//!   is the ONLY `TorDialError` that reaches code 3. A relay on the path can
//!   cause it; the destination cannot. Kept at ADR-0546's letter by the
//!   maintainer's ruling ("keep it"): `Preferred` means connectivity first.
//! * Everything an exit or the far end controls, and everything the plugin
//!   cannot name, is `REFUSED` (4) — fail-closed, no fallback.
//!
//! The wildcard arm is the one `#[non_exhaustive]` forces on a downstream
//! crate, so the compiler cannot police the set here; P9 does instead, by
//! reading `TorDialError`'s DECLARATION from the `dialer-tor` source this
//! crate builds against — an upstream variant the table does not name is a red
//! test, never a silent `REFUSED`.

use dialer_tor::TorDialError;

use crate::abi::{ZW_DIAL_NOT_READY, ZW_DIAL_REFUSED, ZW_DIAL_TIMEOUT};

/// The `ZW_DIAL_*` code a failed dial completes with.
pub fn dial_code(error: &TorDialError) -> u32 {
    match error {
        // A bootstrap in progress, or one the plugin's own loop is retrying:
        // the SDK waits. Never UNREACHABLE — a censored bootstrap under
        // `Preferred` must render, not leak.
        TorDialError::NotBootstrapped { .. } | TorDialError::BootstrapFailed { .. } => {
            ZW_DIAL_NOT_READY
        }
        // The client was shut down under this dial (a rebuild or a dispose
        // retired it): a LOCAL fact that says nothing about the network or
        // any relay, and the next client answers the retry (§12).
        TorDialError::Closed => ZW_DIAL_NOT_READY,
        // The one third-party-inducible fallback left (the maintainer's
        // "keep it").
        TorDialError::TorNetworkTimeout => ZW_DIAL_TIMEOUT,
        // A guard-channel IO error (was 2 before an earlier revision — the guard, or a censor
        // resetting the flow, could induce it), a relay or bridge not working,
        // the exit or the far end stalling or refusing, and targets Tor will
        // not carry: no fallback.
        TorDialError::LocalNetworkError
        | TorDialError::TorAccessFailed
        | TorDialError::ExitTimeout
        | TorDialError::RemoteNetworkTimeout
        | TorDialError::RefusedByExitPolicy
        | TorDialError::RefusedByHost
        | TorDialError::HostNotFound
        | TorDialError::OnionUnsupported
        | TorDialError::ForbiddenTarget
        | TorDialError::BadTarget { .. }
        | TorDialError::Setup { .. }
        | TorDialError::Tor { .. }
        // Only a mint answers it, never a dial; named so the table is whole.
        // Not transient: no new client starts until a restart (§12).
        | TorDialError::RestartRequired => ZW_DIAL_REFUSED,
        // The `#[non_exhaustive]` tail: fail-closed until the table names it.
        _ => ZW_DIAL_REFUSED,
    }
}

#[cfg(test)]
mod tests {
    use dialer_tor::{ErrorKind, TargetProblem, TorDialError};

    use super::*;
    use crate::abi::{ZW_DIAL_OK, ZW_DIAL_RETIRED, ZW_DIAL_UNREACHABLE};

    /// The variant names of `pub enum TorDialError` in the `dialer-tor` source
    /// this crate builds against (a path dependency — the same file cargo
    /// compiles). Reads the enum body only; attributes and doc lines are not
    /// variants.
    fn declared_variants() -> Vec<String> {
        let src = include_str!("../../../dialer-tor/src/error.rs");
        let start = src
            .find("pub enum TorDialError {")
            .expect("the enum is declared");
        let body = &src[start..];
        let end = body.find("\n}\n").expect("the enum closes");
        body[..end]
            .lines()
            .skip(1)
            .filter_map(|line| {
                // A variant starts at exactly four spaces of indent with an
                // upper-case identifier (the crate's rustfmt layout).
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
            .collect()
    }

    /// FR-5 spec §8 **P9**: the §3.3 table row by row, and the variant list
    /// read from `TorDialError`'s DECLARATION — so an upstream variant this
    /// table does not name is a red test. `TorNetworkTimeout` is the ONLY
    /// variant reaching `TIMEOUT`; NO variant reaches `UNREACHABLE` (that code
    /// is the plugin's own device-offline derivation, C3b); nothing maps to
    /// `OK` or `RETIRED` (a stale op is RETIRED by generation, not by error).
    ///
    /// The spec's "interim" rows (the folded `Timeout`/`NoNetwork` before the
    /// §3.6 split) are MOOT: the split landed with `dialer-tor` itself at FR-5
    /// C0, and the declaration read below proves neither folded name exists.
    #[test]
    fn every_tor_error_maps_to_one_frozen_dial_code_and_only_device_or_tor_failures_reach_the_fallback_codes()
     {
        let table: Vec<(&str, TorDialError, u32)> = vec![
            (
                "NotBootstrapped",
                TorDialError::NotBootstrapped {
                    progress: 0.5,
                    blockage: None,
                },
                ZW_DIAL_NOT_READY,
            ),
            (
                "BootstrapFailed",
                TorDialError::BootstrapFailed {
                    kind: ErrorKind::Other,
                    blockage: None,
                },
                ZW_DIAL_NOT_READY,
            ),
            (
                "OnionUnsupported",
                TorDialError::OnionUnsupported,
                ZW_DIAL_REFUSED,
            ),
            (
                "ForbiddenTarget",
                TorDialError::ForbiddenTarget,
                ZW_DIAL_REFUSED,
            ),
            (
                "BadTarget",
                TorDialError::BadTarget {
                    problem: TargetProblem::PortZero,
                },
                ZW_DIAL_REFUSED,
            ),
            (
                "RefusedByExitPolicy",
                TorDialError::RefusedByExitPolicy,
                ZW_DIAL_REFUSED,
            ),
            (
                "RefusedByHost",
                TorDialError::RefusedByHost,
                ZW_DIAL_REFUSED,
            ),
            ("HostNotFound", TorDialError::HostNotFound, ZW_DIAL_REFUSED),
            (
                "TorNetworkTimeout",
                TorDialError::TorNetworkTimeout,
                ZW_DIAL_TIMEOUT,
            ),
            ("ExitTimeout", TorDialError::ExitTimeout, ZW_DIAL_REFUSED),
            (
                "RemoteNetworkTimeout",
                TorDialError::RemoteNetworkTimeout,
                ZW_DIAL_REFUSED,
            ),
            (
                "LocalNetworkError",
                TorDialError::LocalNetworkError,
                ZW_DIAL_REFUSED,
            ),
            (
                "TorAccessFailed",
                TorDialError::TorAccessFailed,
                ZW_DIAL_REFUSED,
            ),
            (
                "Setup",
                TorDialError::Setup {
                    kind: ErrorKind::Other,
                },
                ZW_DIAL_REFUSED,
            ),
            (
                "Tor",
                TorDialError::Tor {
                    kind: ErrorKind::Other,
                },
                ZW_DIAL_REFUSED,
            ),
            ("Closed", TorDialError::Closed, ZW_DIAL_NOT_READY),
            (
                "RestartRequired",
                TorDialError::RestartRequired,
                ZW_DIAL_REFUSED,
            ),
        ];

        let declared = declared_variants();
        assert!(
            declared.len() >= 10,
            "P9 anti-vacuity: read {} variants from dialer-tor's error.rs ({declared:?}) — \
             the reader is not reading the enum it thinks it is",
            declared.len()
        );
        let named: Vec<&str> = table.iter().map(|(name, _, _)| *name).collect();
        let unnamed: Vec<&String> = declared
            .iter()
            .filter(|d| !named.contains(&d.as_str()))
            .collect();
        assert!(
            unnamed.is_empty(),
            "P9: dialer-tor declares TorDialError variant(s) {unnamed:?} that the §3.3 table does \
             not name — the runtime would map them REFUSED by the wildcard, silently. Decide \
             their code (codes 2 and 3 are only for failures the device or the Tor network \
             produces) and add them to `dial_code` and this table."
        );
        let stale: Vec<&&str> = named
            .iter()
            .filter(|n| !declared.iter().any(|d| d == *n))
            .collect();
        assert!(
            stale.is_empty(),
            "P9: the table names variant(s) {stale:?} dialer-tor no longer declares"
        );
        for gone in ["Timeout", "NoNetwork"] {
            assert!(
                !declared.iter().any(|d| d == gone),
                "the folded `{gone}` is back — the §3.6 split has been undone"
            );
        }

        for (name, error, want) in &table {
            assert_eq!(
                dial_code(error),
                *want,
                "{name} maps to the wrong ZW_DIAL_* code"
            );
        }
        let timeout: Vec<&str> = table
            .iter()
            .filter(|(_, e, _)| dial_code(e) == ZW_DIAL_TIMEOUT)
            .map(|t| t.0)
            .collect();
        assert_eq!(
            timeout,
            ["TorNetworkTimeout"],
            "only a Tor-network timeout may reach code 3"
        );
        for (name, error, _) in &table {
            let code = dial_code(error);
            assert_ne!(
                code, ZW_DIAL_UNREACHABLE,
                "{name}: no arti error may produce code 2"
            );
            assert_ne!(code, ZW_DIAL_OK, "{name}: an error never completes OK");
            assert_ne!(
                code, ZW_DIAL_RETIRED,
                "{name}: RETIRED comes from the generation"
            );
        }
    }
}
