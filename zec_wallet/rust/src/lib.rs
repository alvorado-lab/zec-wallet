//! FRB bridge crate for the `zec_wallet` Flutter package (wallet-sdk spec
//! §1.2/§3.3). The bridge is a pure TRANSLATION layer: it owns the Dart-facing
//! DTO shapes (`api/`) and the conversions from the core's types
//! (`convert.rs`) — no wallet logic, no state, and BY CONSTRUCTION no key
//! material (the §8 `ffi_surface_exposes_no_key_types` policy test scans
//! `api/`; key types never appear in any signature here).
//!
//! Unsafe posture (SPEC-DELTA vs wallet-sdk.md §4.1, recorded in §11): the
//! spec asks for crate-level `#![forbid(unsafe_code)]`; `frb_generated.rs`
//! is GENERATED code owned by flutter_rust_bridge and necessarily contains
//! the FFI unsafe, so the crate-level forbid is unavailable. The enforceable
//! equivalent is applied instead: every HANDWRITTEN module denies unsafe
//! (inner attributes in `api/mod.rs` and `convert.rs`). The TWO handwritten
//! exceptions are the C-ABI host seams, `seed_port_cabi.rs` (FR-15, the
//! host-seed supplier) and `net_dialer_cabi.rs` (FR-29, the host transport
//! crossing): calling a host-supplied function pointer and reading or writing
//! through a raw host pointer is irreducibly `unsafe`, confined to documented
//! call sites with the host's contract named on each — `zec-wallet-core` itself
//! stays `#![forbid(unsafe_code)]`.

pub mod api;
// FR-33 — the FRB boundary's own wire-contract version, a C export Dart reads
// BY NAME before the first bridge call (see the module docs). Its only
// unsafe-lint allowance is the `no_mangle` attribute on that one export.
mod bridge_abi;
// §4.3a context-handoff shim — android-only; the one JNI export this cdylib
// carries (deny(unsafe_code) inside, like every handwritten module).
#[cfg(target_os = "android")]
mod android;
mod convert;
// The HOST-SWITCHED device-log layer `init_app` installs, OFF (FR-35; see
// the module docs): this SDK's events, at the level the host sets, with the
// §5.4 allowlist enforced at RUNTIME. deny(unsafe_code) inside — the platform
// sinks are safe wrappers.
mod device_log;
mod frb_generated;
// FR-15 — the C-ABI host-seed seam (see the module docs). The one handwritten
// module with scoped `unsafe`; platform-agnostic (the host that registers the
// supplier may be any native lib, not only Android/iOS).
mod seed_port_cabi;
// FR-29 — the C-ABI host transport crossing (`include/zec_wallet_net_dialer.h`;
// see the module docs). The second handwritten module with scoped `unsafe`;
// platform-agnostic like its sibling.
mod net_dialer_cabi;
// — the sync-server status spells its URLs as the offered entry they name.
mod status_spelling;
// NEAR swap provider seam — present ONLY under `swap-near` (spec §3.5 layer 1
// at the artifact level; see the module docs + `tests/feature_policy.rs`).
#[cfg(feature = "swap-near")]
mod swap_provider;
// Stage S8 `obligation` — the test author's bridge row against names the
// contract says the implementer ADDS (`api::state::DeliveryState`,
// `TxSummary::delivery`, their `convert.rs` crossing). It cannot compile at the
// base commit, so the module is declared but never BUILT: `#[cfg(any())]` is
// never true. The adjudicator joins it by replacing that attribute with
// `#[cfg(test)]` once the names exist (each name is declared in the test
// author's `contractFindings`). Validated against a throwaway definition of
// each name, never committed. JOINED (adjudicator); it compiles once the
// fold's regen carries `TxSummary.delivery` in `frb_generated.rs`.
#[cfg(test)]
mod delivery_state_bridge_tests;
// Stage S16 `bridge` — the test author's rows for `WalletHandle.severCustody`
// and its `SeverReport`, against the names the contract says the implementer
// ADDS (declared in the test author's `contractFindings`). Declared but never
// BUILT until those names exist: the adjudicator joins it by replacing
// `#[cfg(any())]` with `#[cfg(test)]`. JOINED (adjudicator).
#[cfg(test)]
mod sever_bridge_tests;
