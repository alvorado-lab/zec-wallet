//! Outbound network seam (spec §3.2a, ADR-0526). The wallet rides the HOST
//! app's network infrastructure — Tor today, xray/VLESS tomorrow —
//! through the protocol-agnostic [`crate::ports::NetDialer`]; this module turns
//! a [`crate::config::TorPolicy`] into a concrete dialer ([`dialer`]) and builds
//! the lightwalletd gRPC channel over it (`grpc`, lands next).

// FR-37 — the wallet's dial tallies by arm × outcome, written by `log_dial`
// and read by `Wallet::dial_counts`. Its own module for the reason
// `readiness_gate` gives below, and because it is neither the posture nor a
// process global (stage S1 `truth`).
pub(crate) mod dial_counters;
pub(crate) mod dialer;
pub(crate) mod grpc;
// FR-29 — the host transport crossing's core-side types (the descriptor,
// the frozen dial-code table and its ONE mapping, the `HostDialer` accessor
// trait the bridge's cabi module implements). Safe Rust; the `unsafe` lives
// in the bridge. `docs/specs/host-transport-crossing.md` §2.
pub(crate) mod host_dialer;
// FR-29 — the readiness gate in front of a registered host dialer (spec §6.1
// E3): its own module because `dialer.rs`'s tail is a test module with cited
// mutant rows, and clippy forbids items after a test module.
pub(crate) mod readiness_gate;
// The wallet's ONE Tor posture — the sticky fell-back latch plus the patience
// clock of ADR-0552 (the maintainer's minute). Its own module for the same reason
// as `readiness_gate` above, and because the latch and the clock are read and
// written together at a single site.
pub(crate) mod tor_posture;

use crate::state::TorRuntimeKind;

/// Crate-internal connect/resolve failure (§3.2a). Kept INTERNAL on purpose: the
/// public `WalletError` mapping + its FRB bridge mirror + Dart codegen land when
/// the sync engine first surfaces these to the public API (inc-2c-iv), so this
/// seam does not prematurely widen the FFI error surface (the
/// `bridge_enums_cover_core_variants` gate enforces that pairing).
#[derive(Debug)]
pub(crate) enum NetError {
    /// The configured `TorRuntime` has no dialer yet (`ExternalSocks5` / built-in
    /// arti are the §3.2a / ADR-0526 follow-ups). Surfaced typed at resolution —
    /// NEVER a silent clearnet fall-through. `Dialer` is always available.
    UnsupportedRuntime {
        #[allow(dead_code)] // read by the `Debug` rendering only (P3-7)
        runtime: TorRuntimeKind,
    },
    /// Endpoint shape rejected — DEFENSIVE: the §2.3 validator already gates the
    /// endpoint at `WalletConfig` construction, so this is unreachable for a
    /// validated endpoint (kept total for any future caller).
    InvalidEndpoint { reason: &'static str },
}
