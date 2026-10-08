//! `zec_wallet_tor` — the optional Tor plugin for the `zec_wallet` SDK (FR-5,
//! ADR-0548; spec `docs/specs/tor-plugin.md`, plan `docs/plan/fr5-phase-1.md`).
//!
//! A host with no transport of its own registers Tor with the wallet through
//! the wallet's own contract (`zec_wallet_net_dialer.h`, ABI v4) — the same
//! door any host transport uses. The plugin owns arti through the shared crate
//! `dialer-tor`; the wallet compiles no arti at all (P15).
//!
//! The wallet is reached BY SYMBOL at runtime, never as a dependency (D8): it
//! is a separate native library the host has already loaded. This crate is its
//! OWN cargo workspace, excluded from `sdk/`'s, because arti's lock cannot
//! share one with the wallet's ZEC stack (the plan's §5 D-7).
//!
//! The modules: `abi` (both C boundaries, the resolver, the exports — the one
//! `unsafe` module), `trampoline` (the vtable verbs and THE lock), `lifecycle`
//! (init, pause/resume, rebuild, dispose, the bootstrap loop and the readiness
//! watcher), `engine` (the seam over arti), `watcher` and `bootstrap` (the
//! pure push rule and retry schedule), `log` (what the plugin's threads may
//! log), `dial_codes` (the error table), `status` and `constants`.
//!
//! Unsafe posture: denied crate-wide; `abi.rs` is the one module allowed it.

#![deny(unsafe_code)]

pub mod abi;
pub mod bootstrap;
pub mod constants;
pub mod dial_codes;
pub mod engine;
pub mod lifecycle;
pub mod log;
pub mod status;
pub mod trampoline;
pub mod watcher;

#[cfg(test)]
mod tests;
