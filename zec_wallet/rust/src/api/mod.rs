//! The FRB-scanned API surface (`rust_input: crate::api`). RULES OF THIS DIR
//! (spec §3.3 + §4.1):
//!
//! - **DTO definitions and api functions ONLY.** Conversions to/from core
//!   types live in `crate::convert` — OUTSIDE the codegen scan — so nothing
//!   from `zec_wallet_core` can leak into a generated signature by accident.
//!   ONE carve-out: `wallet.rs` holds the OPAQUE `WalletHandle` (the stateful
//!   surface), whose private field is the core `zec_wallet_core::Wallet`. An
//!   opaque must name its content, but FRB never mirrors an opaque's fields,
//!   so no core type reaches a generated signature; all core LOGIC still lives
//!   in `crate::convert` (see `wallet.rs`'s module note).
//! - **No key material, ever.** No seed, mnemonic, spending/viewing key, or
//!   raw `u8` buffer appears in any type or signature here; the
//!   `ffi_surface_exposes_no_key_types` policy test (zec-wallet-core
//!   `tests/extraction_policy.rs`) scans this directory and fails CI on the
//!   first violation. The two SANCTIONED mnemonic crossings — `reveal_mnemonic`
//!   outbound and `restore` inbound (the recovery-phrase pair, spec §3.3) — both
//!   carry the words as a plain `Vec<String>` (never a key TYPE) and are
//!   method/param-name allowlisted in that test; the `SeedSource` construction
//!   stays in `crate::convert`, off this scanned surface.
//! - **OUTBOUND status enums carry an `Unknown` forward-compatibility
//!   variant** (the G2 Dart default-arm target). In a lockstep build it is
//!   unreachable — the `bridge_enums_cover_core_variants` scan keeps
//!   conversion arms complete — but it means an unknown state can never
//!   panic across FFI. INBOUND-only enums (`SwapAmount`, `ExactSide`,
//!   config enums) deliberately have no `Unknown`: Dart can only construct
//!   variants this binding generated, and a bidirectional enum's inbound
//!   `Unknown` is rejected typed as a host bug (see `convert::try_direction`).
//! - **`#[frb(sync)]` is reserved for PURE, cheap, no-I/O functions**
//!   (sub-millisecond): sync calls run ON the calling Dart isolate — the UI
//!   thread. Every wallet fn that touches storage, crypto, or network
//!   (increment 2+) MUST be async (FRB dispatches those off-thread).
//! - Doc comments here flow into the generated Dart API: write them for the
//!   Dart consumer.

#![deny(unsafe_code)]

pub mod config;
pub mod error;
pub mod meta;
pub mod payments;
pub mod restore;
pub mod selftest;
pub mod state;
pub mod swap;
pub mod wallet;
