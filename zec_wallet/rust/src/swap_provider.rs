//! NEAR Intents swap provider seam (ADR-0525; spec §3.2/§3.5). Compiled ONLY
//! under the `swap-near` feature — this module's mere existence is the
//! artifact-level link of the adapter crate (kill layer 1: feature off ⇒ not
//! compiled ⇒ not one byte of the adapter or its HTTP/TLS subtree in the
//! binary; proven by `tests/feature_policy.rs`).
//!
//! W-swap-3-c-2-ii attach point (AS BUILT): the real provider is constructed
//! here, feeding `zec_wallet_swap_near::NearIntentsProvider` the injected
//! `NetDialer` resolved from the wallet's OWN transport policy
//! ([`Wallet::swap_dialer`]) — so swap rides the SAME host-provided transport
//! stack as sync, with NO independent clearnet path (under `TorPolicy::Required`
//! the swap path fail-closes identically). The HOST owns that transport
//! (ADR-0526: `TorRuntime::Dialer` is the host's own Tor/xray/VLESS; the SDK's
//! built-in arti is an optional, not-yet-compiled follow-up — internal Tor is
//! never required), so this composition never names a transport — it only lowers
//! the wallet's configured policy into the dialer the provider rides.
//!
//! WHY HERE AND NOT IN CORE: the core `zec-wallet-core` crate must NEVER depend on a
//! concrete adapter (the pluggability invariant — drivers are peer crates that
//! never import each other; only the composition root knows them all). The bridge
//! IS that composition root, so `NearIntentsProvider` construction lives here, not
//! in `Wallet`. The core exposes only the provider-agnostic `Wallet::enable_swap` /
//! `Wallet::swap_dialer` seams; this module is the ONE place the concrete NEAR
//! adapter is named and wired.
//!
//! KILL-DOOR CONTRACT (security review fold, W-swap-2): the constructed
//! `Arc<dyn SwapPort>` MUST be consumed directly by `SwapService` and NEVER escape
//! — the kill switch (§3.5) lives on `SwapService`, so any retained provider
//! handle called directly would emit swap traffic past a `Hard` kill.
//! [`enable_near_swap`] therefore hands its provider straight into
//! `Wallet::enable_swap` (→ `SwapService::new`) and returns `()` — it never yields
//! a bare `Arc<dyn SwapPort>` to a caller — then immediately lowers the host's
//! signed-manifest flag through `resolve_manifest_kill` into `set_kill`. Gated by
//! the §8 row `no_swap_traffic_path_bypasses_the_kill_door` (`tests/swap_kill_door.rs`).
//!
//! WIRED at D-1: `enable_near_swap` is now called by the FFI `WalletHandle::enable_near_swap`
//! method (via `convert::enable_near_swap`) — the on-switch behind the
//! `swap_quote`/`swap_execute`/`watch_swap_status` Dart surface. (The host swap SCREEN is
//! the next D slice; the source gate + composition smoke test still cover it too.)
#![deny(unsafe_code)]

use std::sync::Arc;

use zec_wallet_core::{SwapError, SwapKill, SwapPort, Wallet, resolve_manifest_kill};
use zec_wallet_swap_near::{NearIntentsProvider, SwapProviderConfig};

/// Construct the live NEAR Intents provider over the wallet's OWN transport and
/// attach it (with the §3.5 kill door applied) — the live `swap_provider`
/// composition (W-swap-3-c-2-ii).
///
/// 1. Resolve the swap transport from the wallet's configured policy
///    ([`Wallet::swap_dialer`]) — the same host-provided dialer + `tor_fell_back`
///    latch the sync path rides; a transport-resolution failure (an unsupported
///    runtime) means the on-ramp has no usable network ⇒ `ProviderUnavailable`
///    (the same root the sync path would surface).
/// 2. Build [`NearIntentsProvider`] over that dialer (TLS unconditionally; the
///    `endpoint` is validated NOW, not at first dial) — the provider never sees a
///    transport it didn't get from the wallet.
/// 3. Hand it straight to [`Wallet::enable_swap`] (set-once; first-wins) so it is
///    owned by `SwapService` and can never escape the kill door.
/// 4. Lower the host's signed-manifest swap state through [`resolve_manifest_kill`]
///    into [`set_kill`](zec_wallet_core::SwapService::set_kill): `swap_enabled=false`
///    or a `declared` severity escalates the service toward-off, monotonically — a
///    forged/replayed "on" can only ever be MORE-off.
///
/// Returns the core [`SwapError`] (the D-slice FFI method maps it to `SwapApiError`
/// via the existing `From` impl). On any failure NO provider is attached — the
/// wallet stays at kill layer 2 (`Wallet::swap() == None`), zero swap surface.
#[cfg(feature = "swap-near")]
pub(crate) fn enable_near_swap(
    wallet: &Wallet,
    config: SwapProviderConfig,
    swap_enabled: bool,
    declared_kill: Option<SwapKill>,
) -> Result<(), SwapError> {
    // #397 §3.7 D3 — REFUSAL PRECEDENCE (security MINOR-1): the
    // STRUCTURAL watch-only refusal is checked FIRST, ahead of any
    // config-shaped failure (unsupported runtime → ProviderUnavailable, bad
    // endpoint → RequestInvalid), so a watch-only wallet with ALSO-broken
    // config still gets the honest "this wallet can never swap" — never a
    // "fix your endpoint / try again" a host could chase forever (the
    // sweep_ephemeral WatchOnly-first precedent, spec §3.7 D3 matrix). A
    // lock-free immutable read, side-effect-free; the core `enable_swap`
    // gate below stays the authoritative refusal — this only orders it
    // ahead of the config legs.
    if wallet.is_watch_only() {
        return Err(SwapError::WatchOnly);
    }
    // Swap rides the wallet's own transport — no separate egress. A resolution
    // failure (unsupported runtime) ⇒ the provider has no network to ride. We do NOT
    // emit a `wallet.swap_port` boundary log here (unlike the core's three swap ports):
    // the bridge is a pure FFI adapter with NO tracing surface by design — all §5.4
    // observability lives in core. And the cause is not lost to the operator: a
    // `swap_dialer` failure is an `UnsupportedRuntime` config error, which the SYNC path
    // (riding the SAME `resolve_dialer`) independently surfaces as a `TorUnavailable`
    // stall in `tor_state()`. The host sees the typed `ProviderUnavailable`.
    let dialer = wallet
        .swap_dialer()
        .map_err(|_| SwapError::ProviderUnavailable)?;
    // The constructed provider is bound to a local and immediately moved INTO
    // `enable_swap`; it is never returned, stored, or otherwise reachable past the
    // kill door (the KILL-DOOR CONTRACT above; gated by `swap_kill_door.rs`).
    let provider: Arc<dyn SwapPort> = Arc::new(NearIntentsProvider::new(config, dialer)?);
    // #397 §3.7 D3: `enable_swap` re-refuses a WATCH-ONLY wallet typed at
    // the constructor — the AUTHORITATIVE core gate behind the precedence
    // check above (belt: if the two ever disagree, the core wins and still
    // crosses TYPED). `map_enable_swap_err` keeps the structural
    // `SwapError::WatchOnly` (RW-SWAP-015) from masquerading as the
    // retryable `ProviderUnavailable` (a third-party host consuming the
    // api/swap layer directly would otherwise render "try again" for a
    // refusal no retry can clear). Unreachable from the reference UI, whose
    // watch-only chrome hides the whole swap surface (D5) and whose
    // activation provider degrades any enable failure to swap-off; the
    // typed code still lands in the host's degrade log.
    let service = wallet.enable_swap(provider).map_err(map_enable_swap_err)?;
    service.set_kill(resolve_manifest_kill(swap_enabled, declared_kill));
    Ok(())
}

/// Map an [`Wallet::enable_swap`] refusal into the swap taxonomy. Today the
/// constructor's ONLY typed refusal is `WatchOnly` (`require_spend_capable`,
/// #397 §3.7 D3) → the structural [`SwapError::WatchOnly`]; any FUTURE
/// `WalletError` variant an SDK upgrade might surface here (the enum is
/// `#[non_exhaustive]`) falls back to the retryable `ProviderUnavailable` —
/// fail-safe: an unknown refusal degrades to "swap stayed off, retryable",
/// never a wrong specific.
#[cfg(feature = "swap-near")]
fn map_enable_swap_err(e: zec_wallet_core::WalletError) -> SwapError {
    match e {
        zec_wallet_core::WalletError::WatchOnly => SwapError::WatchOnly,
        _ => SwapError::ProviderUnavailable,
    }
}

#[cfg(all(test, feature = "swap-near"))]
mod tests {
    use super::*;
    use zec_wallet_core::WalletError;

    /// #397 §3.7 D3: the watch-only constructor refusal crosses the api/swap
    /// layer TYPED — never as the retryable provider outage.
    #[test]
    fn watch_only_enable_refusal_maps_typed_not_provider_unavailable() {
        let mapped = map_enable_swap_err(WalletError::WatchOnly);
        assert!(matches!(mapped, SwapError::WatchOnly));
        assert_eq!(mapped.code(), "RW-SWAP-015");
    }

    /// Any other (unexpected-future) `enable_swap` refusal degrades to the
    /// retryable generic — fail-safe, never a wrong specific.
    #[test]
    fn unknown_enable_refusal_degrades_to_provider_unavailable() {
        let mapped = map_enable_swap_err(WalletError::SeedRequired);
        assert!(matches!(mapped, SwapError::ProviderUnavailable));
    }
}
