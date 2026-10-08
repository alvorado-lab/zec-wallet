//! Restore helpers exposed to Dart (spec §3.6/§3.3). `estimate_birthday` is a
//! PURE, OFFLINE `#[frb(sync)]` function: no wallet handle, no network, no I/O
//! — a static-table lookup, so it is safe to run on the calling (UI) isolate.

use crate::api::config::Network;

/// Estimate the restore birthday HEIGHT from an approximate wallet-creation
/// time (UNIX SECONDS, UTC). CONSERVATIVE FLOOR over the bundled checkpoints —
/// never returns a height past the given time, so a restore can never silently
/// skip the user's notes (spec §3.6). Clamped to [Sapling activation, last
/// bundled checkpoint]; a pre-1970 (negative) input clamps to activation.
///
/// At the bridge this is the primitive (network + unix seconds → height); the
/// Dart facade (§3.3, W3) wraps it with a `DateTime` and the async surface.
#[flutter_rust_bridge::frb(sync)]
pub fn estimate_birthday(network: Network, approx_unix_secs: i64) -> u32 {
    // pre-epoch (negative) input ⇒ 0 ⇒ pre-activation ⇒ the activation floor.
    let secs = u64::try_from(approx_unix_secs).unwrap_or(0);
    zec_wallet_core::estimate_birthday(network.into(), secs).value()
}
