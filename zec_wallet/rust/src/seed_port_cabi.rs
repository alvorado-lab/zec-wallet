//! FR-15 — the C-ABI host-seed seam (cross-dylib seed supply, keys-in-host),
//! hardened by FR-17 (proposal-bound supply) + FR-18 (guarded registration), #396.
//!
//! ## Why this module exists
//! Some hosts consume this SDK as a SEPARATE native library, not an in-workspace
//! Rust crate (Relim ADR-0031: a pre-release-vs-release RustCrypto conflict forces
//! the wallet into its own cargo workspace + its own `.so`/framework). That puts the
//! SDK's [`WalletSeedPort`] (FR-12, a *Rust trait*) on the far side of a dylib
//! boundary from the host code that derives the wallet sub-seed. The seed MUST cross
//! **native-to-native, never through a managed runtime (Dart/JVM)** — a `Uint8List`
//! cannot be zeroized, so a seed that reaches Dart is a leak by construction.
//!
//! This module is a thin **C-ABI transport** over FR-12: the host registers a
//! callback (an opaque `ctx` pointer + a length-returning `extern "C"` function) and
//! [`CAbiSeedPort`] adapts it to the Rust [`WalletSeedPort`] the core pulls during
//! its blocking proving/import section. The FR-12 staging discipline — the host
//! stages narrowly and clears (take-once for a send, scoped for a provision) — is
//! UNCHANGED and stays entirely host-side; this module only moves already-staged
//! bytes across the `.so` boundary.
//!
//! ## FR-17 — the bound supplier
//! [`SeedSupplyBoundFn`] additionally receives the [`SpendBinding`] of the exact
//! proposal/row the SDK is signing (null/0 for an unbound pull — import, sweep,
//! reclaim, refund derivation, or a legacy pre-FR-17 queued row). A host that
//! records the binding at its authorize bracket compares with STRICT `Option`
//! equality (unbound stage matches ONLY an unbound pull; constant-time bytes
//! compare) and returns a negative code on mismatch — fail-closing a seed pull for
//! any proposal it did not authorize. The legacy [`SeedSupplyFn`] keeps working but
//! RECEIVES no binding — it cannot enforce FR-17; registering the bound form is the
//! opt-in.
//!
//! ## FR-18 — the guarded registry
//! The registry is **first-wins + token-gated** (it was idempotent last-wins, and
//! the null-supply clear was itself the substitution primitive: any in-process
//! caller could deregister the host then register itself). First registration
//! stands; [`zec_wallet_register_seed_port_bound`] mints a random 32-byte
//! **registration auth token** into the host's out-pointer, and only
//! [`zec_wallet_update_seed_port`] presenting that token (constant-time compare)
//! can replace or clear the supplier (identity switch / shutdown). The v1
//! [`zec_wallet_register_seed_port`] keeps its exact ABI but is now
//! set-once-permanent (no token to mutate with). No return code distinguishes
//! "wrong token" from "occupied" (`-2` for both — no probe oracle).
//!
//! ## Unsafe posture (the one carve-out)
//! Every other handwritten module in this bridge `#![deny(unsafe_code)]`. This one
//! cannot: calling a host-supplied function pointer and writing through raw
//! pointers is irreducibly `unsafe`. The `unsafe` is confined to the
//! [`CAbiSeedPort::provide_seed`] call site plus the two out-pointer/token reads in
//! the registration verbs, with the host's safety contract documented on each. The
//! core stays `#![forbid(unsafe_code)]`; the unsafe lives here, in the cdylib, by
//! design.
//!
//! ## Reversal note (Relim ADR-0031)
//! When the wallet collapses back to an in-workspace crate + static link, the host
//! reverts to the direct Rust [`WalletSeedPort`] (FR-12) and this shim is dropped.
//! It is a thin bridge for the separate-dylib window, harmless to keep otherwise —
//! any host that embeds the SDK as a standalone native package and keeps key
//! derivation in its own native code needs exactly this cross-boundary seam.

use std::ffi::c_void;
use std::sync::{Arc, OnceLock, RwLock};

use zec_wallet_core::constants::{SEED_MAX_BYTES, SEED_MIN_BYTES};
use zec_wallet_core::{
    SPEND_BINDING_BYTES, SeedSupplyError, SpendBinding, WalletSeedPort, Zeroizing,
};

/// The FR-18 registration auth token length (bytes). Fixed so the host can embed
/// it in a fixed slot; the same 32-byte width as a [`SpendBinding`].
pub const SEED_PORT_AUTH_TOKEN_BYTES: usize = 32;

/// The host's LEGACY seed-supply callback (FR-15 v1). One call writes the staged
/// sub-seed into `out` and returns the number of bytes written, or a NEGATIVE value
/// to signal "no seed available" (the FR-12 `Unavailable`/`Denied` cases — collapsed
/// to one outcome here, so there is no refusal-vs-absent oracle). Receives NO
/// spend binding — a v1 supplier cannot enforce FR-17 (documented; register the
/// bound form to opt in).
///
/// # Safety (the host's contract)
/// - `out` is valid for writes of `out_cap` bytes for the duration of the call.
/// - The function writes AT MOST `out_cap` bytes and returns exactly that count
///   (`0 <= n <= out_cap`), or a negative code; it never reports more than it wrote.
/// - It is callable from an arbitrary thread (the SDK pulls on a blocking pool
///   thread) and is non-blocking + non-re-entrant: it returns already-staged bytes
///   and MUST NOT call back into the wallet or into any registration verb.
/// - It does NOT unwind across the boundary (a panic across `extern "C"` is
///   undefined behaviour — the host catches its own panics).
/// - `ctx` is the host-opaque pointer it registered; the host owns its lifetime and
///   guarantees it is valid + safe to share across threads while registered.
pub type SeedSupplyFn =
    unsafe extern "C" fn(ctx: *mut c_void, out: *mut u8, out_cap: usize) -> isize;

/// The host's BOUND seed-supply callback (FR-17). Same contract as
/// [`SeedSupplyFn`], plus the SDK passes the [`SpendBinding`] of the exact
/// proposal/row being signed:
/// - `binding` / `binding_len`: the 32-byte nonce for a bound (spend-signing) pull,
///   or `NULL`/`0` for an unbound pull (import, sweep, reclaim, refund derivation,
///   legacy row). `binding` is valid for reads of `binding_len` bytes for the
///   duration of the call ONLY — the host copies if it needs it longer.
/// - The host compares against the binding it recorded at its authorize bracket
///   with STRICT `Option` equality (unbound stage ⟺ unbound pull; bytes equal via a
///   constant-time compare) and returns a negative code on ANY mismatch — that
///   fail-close is the FR-17 property; the SDK presents honestly but cannot verify
///   for the host.
pub type SeedSupplyBoundFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    binding: *const u8,
    binding_len: usize,
    out: *mut u8,
    out_cap: usize,
) -> isize;

/// Which callback shape the host registered. Both are bare fn pointers (`Copy`),
/// so a [`Registration`] snapshot never holds the lock across the foreign call.
#[derive(Clone, Copy)]
enum SupplierKind {
    /// FR-15 v1 — receives no binding (cannot enforce FR-17).
    Plain(SeedSupplyFn),
    /// FR-17 — receives the spend binding (or null/0 for an unbound pull).
    Bound(SeedSupplyBoundFn),
}

/// A registered host seed supplier. `ctx` is held as a `usize` so the record is
/// `Send + Sync` WITHOUT an `unsafe impl` — the host owns the pointer's lifetime and
/// thread-safety (see [`SeedSupplyFn`]), and it is cast back to `*mut c_void` only at
/// the call site. All fields are `Copy`, so the registration is snapshotted out of
/// the lock before the foreign call (the lock never wraps the host callback).
#[derive(Clone, Copy)]
struct Registration {
    ctx: usize,
    supply: SupplierKind,
}

/// The guarded registry slot (FR-18). `guard` is the host-held auth token that
/// gates every mutation of an occupied slot; `None` while occupied means the
/// registration is PERMANENT (v1, or a bound register with a null `auth_out`).
/// The token zeroizes on drop (it is an in-process capability, not key material —
/// hygiene, not secrecy-critical).
struct RegistryState {
    registration: Option<Registration>,
    guard: Option<Zeroizing<[u8; SEED_PORT_AUTH_TOKEN_BYTES]>>,
}

/// Process-global registry. One host library registers once; the wallet library
/// pulls through it. `RwLock` because reads (every pull) vastly outnumber writes
/// (register/update), and the foreign call runs OUTSIDE the lock.
fn registry() -> &'static RwLock<RegistryState> {
    static REGISTRY: OnceLock<RwLock<RegistryState>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        RwLock::new(RegistryState {
            registration: None,
            guard: None,
        })
    })
}

/// Constant-time 32-byte equality via `subtle::ConstantTimeEq` (crypto audit
/// fold: the hand XOR-fold was data-independent but barrier-less — compiler
/// goodwill; `subtle` is already workspace-pinned, so the barrier'd form costs no
/// new crate). The compared values live in the same process, so the timing oracle
/// is marginal either way — but the compare sits on the FR-18 trust boundary and
/// doing it right is one line.
fn ct_eq(a: &[u8; SEED_PORT_AUTH_TOKEN_BYTES], b: &[u8; SEED_PORT_AUTH_TOKEN_BYTES]) -> bool {
    subtle::ConstantTimeEq::ct_eq(&a[..], &b[..]).into()
}

/// Mint a fresh registration auth token (OS CSPRNG — the same pinned `rand_core`
/// the core draws from).
fn mint_auth_token() -> Zeroizing<[u8; SEED_PORT_AUTH_TOKEN_BYTES]> {
    let mut token = Zeroizing::new([0u8; SEED_PORT_AUTH_TOKEN_BYTES]);
    rand_core::RngCore::fill_bytes(&mut rand_core::OsRng, token.as_mut());
    token
}

/// Return code of every registration verb when the SDK PANICKED inside it
/// (FR-29 stage 0 (iii); the design audit's HIGH). A panic that reaches an
/// `extern "C"` boundary ABORTS the process (defined behaviour since Rust
/// 1.81, undefined before) — the HOST's process, here — so each verb runs its
/// body behind [`guarded`] and reports a caught panic as this code instead.
/// The registry is unchanged by a panicked verb: the one panic site the audit
/// named (`mint_auth_token`'s CSPRNG fill, which panics on RNG failure) runs
/// BEFORE the registry write, and a panic INSIDE a write's lock scope poisons
/// the lock, after which every verb reads `-1` — honest, never silent, never
/// unwinding. Host-facing: `docs/specs/wallet-sdk.md` §3.5's FR-15 code list.
pub const SEED_PORT_RC_PANICKED: i32 = -4;

/// The panic boundary every `extern "C"` verb runs behind: a caught unwind
/// becomes [`SEED_PORT_RC_PANICKED`]. `AssertUnwindSafe` because the bodies
/// capture raw host pointers, which the type system cannot judge; nothing the
/// bodies touch is observable half-written — the registry commits under one
/// lock, and the host's `auth_out` is written only after that commit.
///
/// Two limits, stated so they are not read as guarantees: (1) the boundary
/// needs `panic = "unwind"` — `sdk/Cargo.toml` sets no `panic = "abort"`
/// profile, but a host that builds the library with abort in its RUSTFLAGS
/// gets exactly the abort this guard exists to prevent; (2) the default panic
/// hook has already printed the panic message (and a backtrace under
/// `RUST_BACKTRACE`) to stderr by the time the unwind is caught — so nothing
/// inside these verbs may format a secret into a panic message.
fn guarded(body: impl FnOnce() -> i32) -> i32 {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)).unwrap_or(SEED_PORT_RC_PANICKED)
}

/// Test-only planted panic. The audit's site (`OsRng` failing inside
/// `mint_auth_token`) cannot be provoked on a test host, so the named test
/// plants a panic HERE — inside the boundary, before any lock — in every verb.
#[cfg(test)]
static PLANT_PANIC: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[inline]
fn planted_panic_point() {
    #[cfg(test)]
    if PLANT_PANIC.load(std::sync::atomic::Ordering::SeqCst) {
        panic!("planted panic inside a seed-port verb (test)");
    }
}

/// Register the process-global host seed supplier (FR-15 v1 ABI, FR-18 semantics).
///
/// **SEMANTICS CHANGED (#396, pre-publication):** was "idempotent / last-wins,
/// null-supply clears". Now **first-wins + permanent**: succeeds ONLY into an empty
/// slot, and a v1 registration can never be mutated again in this process (the v1
/// signature has no token out-parameter). A second register — or a null-supply
/// clear — against an occupied slot returns `-2` and the first registration stands.
/// The old null-clear path was itself the substitution primitive FR-18 exists to
/// kill. A host that needs replace/clear (identity switch, shutdown) registers via
/// [`zec_wallet_register_seed_port_bound`] and mutates via
/// [`zec_wallet_update_seed_port`].
///
/// Returns `0` ok · `-1` registry lock poisoned · `-2` slot occupied (first
/// registration stands) · `-3` null `supply` into an empty slot (nothing to clear;
/// a clear needs the token-gated update verb) · `-4` the SDK panicked inside the
/// verb ([`SEED_PORT_RC_PANICKED`]; nothing registered, never unwinds into the host).
#[unsafe(no_mangle)]
pub extern "C" fn zec_wallet_register_seed_port(
    ctx: *mut c_void,
    supply: Option<SeedSupplyFn>,
) -> i32 {
    guarded(|| {
        planted_panic_point();
        let Ok(mut guard) = registry().write() else {
            // Poisoned lock (a prior panic while holding it): refuse rather than
            // risk unwinding into the host — the host sees the -1 and can act.
            return -1;
        };
        if guard.registration.is_some() {
            return -2; // occupied — first-wins, and v1 has no token to present
        }
        let Some(supply) = supply else {
            return -3; // null-clear of an EMPTY slot: nothing to do, surfaced loud
        };
        guard.registration = Some(Registration {
            ctx: ctx as usize,
            supply: SupplierKind::Plain(supply),
        });
        guard.guard = None; // v1: permanent (no mutation capability exists)
        0
    })
}

/// Register the process-global host seed supplier in its FR-17 BOUND form, minting
/// the FR-18 registration auth token.
///
/// First-wins like the v1 verb. On success, if `auth_out` is non-null the verb
/// mints a random 32-byte auth token, stores it as the slot's mutation guard, and
/// writes it to `auth_out` — the host keeps it beside its supplier state and
/// presents it to [`zec_wallet_update_seed_port`] to replace/clear. A null
/// `auth_out` makes the registration PERMANENT (deliberate opt-out, v1-style).
///
/// **A non-zero return is FATAL to the registration attempt** (security
/// fold): `-2` means another in-process registrant won the slot first — the
/// host's supplier is NOT installed and `auth_out` is left UNTOUCHED. Proceeding
/// as if registered would hand every later seed pull to the OTHER registrant's
/// supplier. Treat it like a failed trusted-init invariant, not a retryable
/// condition. (`auth_out` is written ONLY on `0` — so a benign in-process
/// re-init that gets `-2` cannot clobber the token the host already holds from
/// its first, winning registration.)
///
/// Returns `0` ok · `-1` lock poisoned · `-2` slot occupied · `-3` null `supply`
/// · `-4` the SDK panicked inside the verb ([`SEED_PORT_RC_PANICKED`]; the slot
/// and `auth_out` untouched, never unwinds into the host).
///
/// # Safety
/// `auth_out`, when non-null, must be valid for writes of
/// [`SEED_PORT_AUTH_TOKEN_BYTES`] bytes for the duration of the call, and MUST
/// NOT be aliased with any concurrent registration attempt (two racers writing
/// the same buffer is a data race — the loser would leave the winner holding the
/// wrong bytes). It is written at most once, only on success.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn zec_wallet_register_seed_port_bound(
    ctx: *mut c_void,
    supply: Option<SeedSupplyBoundFn>,
    auth_out: *mut u8,
) -> i32 {
    guarded(|| {
        planted_panic_point();
        let Some(supply) = supply else {
            return -3;
        };
        // Mint the token into a LOCAL before the lock (no foreign write yet — just
        // a stack fill, review fold): a null `auth_out` opts out (permanent,
        // no token). Nothing is written to the host's pointer until AFTER the slot
        // commits AND the lock is released — so (a) the foreign-memory write never
        // runs under the registry write lock (a stalling `auth_out` page can't
        // wedge seed pulls; the same discipline `provide_seed` applies to the
        // callback), and (b) the `-2`/`-1` failure paths never touch `auth_out`,
        // so a benign re-register can't destroy a live token (H-1). The mint is
        // the audit's panic site (RNG failure) — it runs inside `guarded`, before
        // the lock, so a panic here changes nothing and poisons nothing.
        let minted: Option<Zeroizing<[u8; SEED_PORT_AUTH_TOKEN_BYTES]>> =
            (!auth_out.is_null()).then(mint_auth_token);
        {
            let Ok(mut guard) = registry().write() else {
                return -1; // minted drops (zeroizes) here; auth_out untouched
            };
            if guard.registration.is_some() {
                return -2; // occupied — minted drops here; auth_out untouched (H-1)
            }
            guard.registration = Some(Registration {
                ctx: ctx as usize,
                supply: SupplierKind::Bound(supply),
            });
            // Store the slot's mutation guard (a clone; the local is copied out below).
            guard.guard = minted.as_ref().map(|t| Zeroizing::new(**t));
        } // write lock RELEASED here, before any foreign write
        if let Some(token) = &minted {
            // SAFETY: caller contract — non-null `auth_out` is valid for
            // SEED_PORT_AUTH_TOKEN_BYTES writes and unaliased; `copy_nonoverlapping`
            // from a fresh local can't overlap. Reached ONLY on the success path,
            // so the registration is already committed and the delivered token
            // matches the slot's guard. If the host's `auth_out` stalls/faults here
            // the call blocks the HOST's own thread (not the registry lock); a
            // death mid-copy leaves a committed-but-tokenless registration = the
            // same permanent-registration outcome as a null `auth_out` (honest,
            // the host's own bug).
            unsafe {
                std::ptr::copy_nonoverlapping(token.as_ptr(), auth_out, SEED_PORT_AUTH_TOKEN_BYTES);
            }
        }
        0
    })
}

/// Token-gated REPLACE (non-null `supply`) or CLEAR (null `supply`) of the
/// registered supplier (FR-18) — the identity-switch / shutdown verb.
///
/// `auth` must present the 32 bytes minted by the registration
/// ([`zec_wallet_register_seed_port_bound`] with a non-null `auth_out`); the
/// compare is constant-time. A replace keeps the existing token; a successful
/// clear empties the slot AND invalidates the token (the next registration mints
/// fresh). Only the bound supplier shape can be installed here — the v1 shape has
/// no reason to arrive via the guarded path.
///
/// **QUIESCE CONTRACT (review fold — the shutdown ordering the host owns):**
/// a seed pull that snapshotted the registration before this verb returns may
/// still be EXECUTING the OLD supplier after it returns (the pull deliberately
/// invokes the callback outside the registry lock, so an update can never
/// deadlock behind a pull — the trade is this window). After a successful clear
/// or replace the host MUST keep the old `ctx` and the old callback's code alive
/// until no pull can still be in flight — pulls are contractually O(1) and
/// non-blocking (see [`SeedSupplyFn`]), so any wallet-side quiesce (close the
/// wallet handles, or simply a bounded delay) suffices; freeing `ctx` or
/// unloading the library on the clear's return edge is a use-after-free race.
///
/// Returns `0` ok · `-1` lock poisoned · `-2` unauthorized (no registration, a
/// permanent registration, or a wrong token — deliberately ONE code, no probe
/// oracle) · `-3` null `auth` · `-4` the SDK panicked inside the verb
/// ([`SEED_PORT_RC_PANICKED`]; the slot unchanged, never unwinds into the host).
///
/// # Safety
/// `auth` must be valid for reads of [`SEED_PORT_AUTH_TOKEN_BYTES`] bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn zec_wallet_update_seed_port(
    auth: *const u8,
    ctx: *mut c_void,
    supply: Option<SeedSupplyBoundFn>,
) -> i32 {
    guarded(|| {
        planted_panic_point();
        if auth.is_null() {
            return -3;
        }
        // SAFETY: caller contract — `auth` is valid for SEED_PORT_AUTH_TOKEN_BYTES
        // reads. Copied to a local (zeroized on drop) before taking the lock.
        let presented: Zeroizing<[u8; SEED_PORT_AUTH_TOKEN_BYTES]> = {
            let mut buf = Zeroizing::new([0u8; SEED_PORT_AUTH_TOKEN_BYTES]);
            unsafe {
                std::ptr::copy_nonoverlapping(auth, buf.as_mut_ptr(), SEED_PORT_AUTH_TOKEN_BYTES);
            }
            buf
        };
        let Ok(mut guard) = registry().write() else {
            return -1;
        };
        // One `-2` for every refusal shape: empty slot, permanent registration,
        // wrong token. Distinguishing them would hand an in-process prober a
        // registry oracle.
        let authorized = match (&guard.registration, &guard.guard) {
            (Some(_), Some(expected)) => ct_eq(expected, &presented),
            _ => false,
        };
        if !authorized {
            return -2;
        }
        match supply {
            Some(supply) => {
                // REPLACE: same token continues to guard the slot.
                guard.registration = Some(Registration {
                    ctx: ctx as usize,
                    supply: SupplierKind::Bound(supply),
                });
            }
            None => {
                // CLEAR: empty the slot AND invalidate the token — a later register
                // mints fresh; the cleared token authorizes nothing.
                guard.registration = None;
                guard.guard = None;
            }
        }
        0
    })
}

/// The [`WalletSeedPort`] the bridge attaches to a wallet (a ZST — the state lives in
/// the process-global [`registry`]). Each [`provide_seed`](WalletSeedPort::provide_seed)
/// snapshots the current registration, invokes the host callback into a fixed-size
/// zeroizing scratch buffer, validates the reported length against the
/// [`SeedSource`](zec_wallet_core::SeedSource) `32..=252` bound, and returns exactly
/// those bytes (the scratch zeroizes on drop, including every error path). A
/// [`SupplierKind::Bound`] callback additionally receives the FR-17 binding
/// (null/0 for an unbound pull); a [`SupplierKind::Plain`] one drops it (v1 —
/// cannot enforce FR-17, documented).
pub(crate) struct CAbiSeedPort;

impl WalletSeedPort for CAbiSeedPort {
    fn provide_seed(
        &self,
        binding: Option<&SpendBinding>,
    ) -> Result<Zeroizing<Vec<u8>>, SeedSupplyError> {
        // Snapshot the registration OUT of the lock so the host callback never runs
        // while we hold it (defensive against a re-entrant register/update; the lock
        // is only ever held for the O(1) copy here).
        let reg = {
            let guard = registry()
                .read()
                .map_err(|_| SeedSupplyError::Unavailable)?;
            match guard.registration.as_ref() {
                Some(reg) => *reg, // Registration: Copy
                None => return Err(SeedSupplyError::Unavailable),
            }
        };

        // Fixed scratch sized to the SeedSource upper bound; zeroizes on drop
        // (including every early return below).
        let mut scratch: Zeroizing<Vec<u8>> = Zeroizing::new(vec![0u8; SEED_MAX_BYTES]);

        // SAFETY: `scratch` is a live allocation of `scratch.len()` (== SEED_MAX_BYTES)
        // bytes; we pass its pointer and its true capacity. Per the callback contracts
        // the host writes at most `out_cap` bytes, returns the count written (or a
        // negative code), is callable on this thread, and does not unwind across the
        // boundary. `reg.ctx` is the host-opaque pointer it registered. For the bound
        // shape, the binding pointer/len name a live 32-byte array borrowed for
        // exactly this call (or null/0 — the documented unbound pull).
        let written = unsafe {
            match reg.supply {
                SupplierKind::Plain(supply) => {
                    // v1: the binding is DROPPED (the callback has no parameter for
                    // it) — a plain supplier cannot enforce FR-17.
                    supply(reg.ctx as *mut c_void, scratch.as_mut_ptr(), scratch.len())
                }
                SupplierKind::Bound(supply) => {
                    let (ptr, len) = match binding {
                        Some(b) => (b.as_bytes().as_ptr(), SPEND_BINDING_BYTES),
                        None => (std::ptr::null(), 0),
                    };
                    supply(
                        reg.ctx as *mut c_void,
                        ptr,
                        len,
                        scratch.as_mut_ptr(),
                        scratch.len(),
                    )
                }
            }
        };

        if written < 0 {
            // Unavailable / Denied / FR-17 binding mismatch collapse to one outcome
            // — no oracle (FR-12).
            return Err(SeedSupplyError::Unavailable);
        }
        // `written >= 0` ⇒ the cast is exact. A count outside the SeedSource bound is
        // rejected fail-closed BEFORE the `scratch[..written]` slice below — which would
        // otherwise PANIC if a buggy/hostile host over-reports (the scratch is fully
        // zero-initialized, so an in-bound over-report reads only zeros, never foreign or
        // uninitialized memory).
        let written = written as usize;
        if !(SEED_MIN_BYTES..=SEED_MAX_BYTES).contains(&written) {
            return Err(SeedSupplyError::Unavailable);
        }

        // Exactly the written bytes; the full-capacity `scratch` zeroizes on drop.
        Ok(Zeroizing::new(scratch[..written].to_vec()))
    }
}

/// The registered host seed port as a `WalletSeedPort`, or `None` when the host has
/// not registered one. The bridge's host-seed create/open verbs attach this; a `None`
/// lets them fail EARLY + honestly (mapped to `SeedRequired`) rather than build a
/// wallet that could never import its account or sign.
pub(crate) fn registered_seed_port() -> Option<Arc<dyn WalletSeedPort>> {
    let guard = registry().read().ok()?;
    guard
        .registration
        .is_some()
        .then(|| Arc::new(CAbiSeedPort) as Arc<dyn WalletSeedPort>)
}

// Compile-time pins: the wrapper + the registration are `Send + Sync` so the `Arc`
// rides the blocking pool and the registry can be a `static`. (Both are also enforced
// structurally — the `WalletSeedPort` supertrait and the `static RwLock` — but assert
// it explicitly so a future field that breaks it fails HERE, with this message.)
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<CAbiSeedPort>();
    assert_send_sync::<Registration>();
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};

    // The registry is process-global, so serialise the tests that touch it (cargo
    // runs test fns in parallel). Each test resets the slot via the test-only
    // reset below (FR-18 removed the unauthenticated clear the old tests used).
    static TEST_LOCK: Mutex<()> = Mutex::new(());
    // Cross-talk between a test and its `extern "C"` supply (closures can't be
    // `extern "C"` fns, so the behaviour rides module statics).
    static PULLS: AtomicUsize = AtomicUsize::new(0);
    static RECEIVED_CTX: AtomicUsize = AtomicUsize::new(0);
    // `SENTINEL` = "no override, write a real seed"; any other value is returned
    // verbatim by the suppliers (to model a negative code or an out-of-bound count).
    const SENTINEL: isize = isize::MIN;
    static OVERRIDE_CODE: AtomicIsize = AtomicIsize::new(SENTINEL);
    // How many bytes the suppliers write when not overridden (default the 32-byte
    // floor; a test bumps it to exercise the 252-byte ceiling).
    static WRITE_LEN: AtomicUsize = AtomicUsize::new(32);
    // What the BOUND supplier observed for (binding_ptr_null, binding_len) plus the
    // first 4 binding bytes (enough to pin verbatim delivery without a Vec static).
    static SAW_BINDING_LEN: AtomicUsize = AtomicUsize::new(usize::MAX);
    static SAW_BINDING_PREFIX: AtomicUsize = AtomicUsize::new(0);

    /// Test-only registry reset: FR-18 deliberately removed every unauthenticated
    /// mutation path, so tests (which share the process-global slot) need this
    /// seam to isolate from each other. `#[cfg(test)]`-only by construction.
    fn reset_registry_for_test() {
        let mut guard = registry().write().unwrap_or_else(|e| e.into_inner());
        guard.registration = None;
        guard.guard = None;
    }

    /// Write `len` deterministic bytes `[1, 2, …]` into `out` (bounded by `cap`).
    ///
    /// # Safety
    /// `out` must be valid for writes of `cap` bytes (the wrapper passes its scratch).
    unsafe fn write_seed(out: *mut u8, cap: usize, len: usize) -> isize {
        let n = len.min(cap);
        for i in 0..n {
            // SAFETY: `i < n <= cap` and `out` is valid for `cap` bytes.
            unsafe { *out.add(i) = (i as u8).wrapping_add(1) };
        }
        n as isize
    }

    /// v1 supplier: always writes a `WRITE_LEN`-byte seed (default 32), unless an
    /// `OVERRIDE_CODE` is set (then returns it verbatim). Records `ctx`.
    unsafe extern "C" fn supply_always(ctx: *mut c_void, out: *mut u8, cap: usize) -> isize {
        RECEIVED_CTX.store(ctx as usize, Ordering::SeqCst);
        PULLS.fetch_add(1, Ordering::SeqCst);
        let code = OVERRIDE_CODE.load(Ordering::SeqCst);
        if code != SENTINEL {
            return code;
        }
        // SAFETY: the SDK wrapper passes a valid `out` for `cap` bytes.
        unsafe { write_seed(out, cap, WRITE_LEN.load(Ordering::SeqCst)) }
    }

    /// v1 supplier: 32-byte seed on the FIRST pull, then `-1` (models the host's
    /// send-scope take-once: one credential confirmation ⇒ exactly one signature).
    unsafe extern "C" fn supply_take_once(ctx: *mut c_void, out: *mut u8, cap: usize) -> isize {
        RECEIVED_CTX.store(ctx as usize, Ordering::SeqCst);
        let prior = PULLS.fetch_add(1, Ordering::SeqCst);
        if prior > 0 {
            return -1; // already taken — no second sign
        }
        // SAFETY: the SDK wrapper passes a valid `out` for `cap` bytes.
        unsafe { write_seed(out, cap, 32) }
    }

    /// BOUND supplier: records what binding it was shown, then behaves like
    /// `supply_always`.
    unsafe extern "C" fn supply_bound_recording(
        ctx: *mut c_void,
        binding: *const u8,
        binding_len: usize,
        out: *mut u8,
        cap: usize,
    ) -> isize {
        RECEIVED_CTX.store(ctx as usize, Ordering::SeqCst);
        PULLS.fetch_add(1, Ordering::SeqCst);
        SAW_BINDING_LEN.store(binding_len, Ordering::SeqCst);
        if !binding.is_null() && binding_len >= 4 {
            // SAFETY: the wrapper passes a pointer valid for `binding_len` reads.
            let prefix = unsafe {
                u32::from_le_bytes([*binding, *binding.add(1), *binding.add(2), *binding.add(3)])
            };
            SAW_BINDING_PREFIX.store(prefix as usize, Ordering::SeqCst);
        } else {
            SAW_BINDING_PREFIX.store(0, Ordering::SeqCst);
        }
        let code = OVERRIDE_CODE.load(Ordering::SeqCst);
        if code != SENTINEL {
            return code;
        }
        // SAFETY: the SDK wrapper passes a valid `out` for `cap` bytes.
        unsafe { write_seed(out, cap, WRITE_LEN.load(Ordering::SeqCst)) }
    }

    /// A distinct bound supplier for the FR-18 replace test (a different fn address).
    unsafe extern "C" fn supply_bound_second(
        ctx: *mut c_void,
        _binding: *const u8,
        _binding_len: usize,
        out: *mut u8,
        cap: usize,
    ) -> isize {
        RECEIVED_CTX.store(ctx as usize, Ordering::SeqCst);
        PULLS.fetch_add(1, Ordering::SeqCst);
        // SAFETY: the SDK wrapper passes a valid `out` for `cap` bytes.
        unsafe { write_seed(out, cap, 64) } // 64 bytes ⇒ distinguishable from the first
    }

    fn reset() {
        PULLS.store(0, Ordering::SeqCst);
        OVERRIDE_CODE.store(SENTINEL, Ordering::SeqCst);
        RECEIVED_CTX.store(0, Ordering::SeqCst);
        WRITE_LEN.store(32, Ordering::SeqCst);
        SAW_BINDING_LEN.store(usize::MAX, Ordering::SeqCst);
        SAW_BINDING_PREFIX.store(0, Ordering::SeqCst);
        reset_registry_for_test();
    }

    /// Register the recording bound supplier and hand back its auth token.
    fn register_bound_with_token() -> [u8; SEED_PORT_AUTH_TOKEN_BYTES] {
        let mut token = [0u8; SEED_PORT_AUTH_TOKEN_BYTES];
        let rc = unsafe {
            zec_wallet_register_seed_port_bound(
                ptr::null_mut(),
                Some(supply_bound_recording),
                token.as_mut_ptr(),
            )
        };
        assert_eq!(rc, 0, "bound registration into an empty slot succeeds");
        token
    }

    /// FR-29 stage 0 (iii) — the design audit's HIGH: a panic inside a
    /// registration verb must NEVER unwind into the host (undefined behaviour; in
    /// practice an abort of the host process). The audit's site is
    /// `mint_auth_token`'s CSPRNG fill (`OsRng` panics on failure — not
    /// reproducible on a test host), so a panic is PLANTED at the verbs' shared
    /// plant point, inside the boundary and before any lock: every verb returns
    /// `SEED_PORT_RC_PANICKED`, the registry is untouched (empty stays empty; an
    /// occupied slot keeps its supplier), the host's `auth_out` is untouched, and
    /// the lock is not poisoned (the next verb works).
    #[test]
    fn a_planted_panic_inside_every_seed_port_verb_returns_a_code_and_never_unwinds() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        PLANT_PANIC.store(true, Ordering::SeqCst);
        let mut auth_out = [0xAAu8; SEED_PORT_AUTH_TOKEN_BYTES];
        let rc_v1 = zec_wallet_register_seed_port(ptr::null_mut(), Some(supply_always));
        let rc_bound = unsafe {
            zec_wallet_register_seed_port_bound(
                ptr::null_mut(),
                Some(supply_bound_recording),
                auth_out.as_mut_ptr(),
            )
        };
        let token = [7u8; SEED_PORT_AUTH_TOKEN_BYTES];
        let rc_update = unsafe {
            zec_wallet_update_seed_port(token.as_ptr(), ptr::null_mut(), Some(supply_bound_second))
        };
        PLANT_PANIC.store(false, Ordering::SeqCst);
        assert_eq!(
            [rc_v1, rc_bound, rc_update],
            [SEED_PORT_RC_PANICKED; 3],
            "every verb reports a caught panic as -4"
        );
        assert_eq!(
            auth_out, [0xAAu8; SEED_PORT_AUTH_TOKEN_BYTES],
            "the host's auth_out is untouched by a panicked registration"
        );
        assert!(
            registered_seed_port().is_none(),
            "a panicked verb registers nothing"
        );
        // The lock is not poisoned and the slot is empty: a real registration
        // succeeds and pulls through it.
        let real = register_bound_with_token();
        assert!(registered_seed_port().is_some());
        assert!(CAbiSeedPort.provide_seed(None).is_ok());
        // And an occupied slot survives a panicked clear: the supplier stays.
        PLANT_PANIC.store(true, Ordering::SeqCst);
        let rc = unsafe { zec_wallet_update_seed_port(real.as_ptr(), ptr::null_mut(), None) };
        PLANT_PANIC.store(false, Ordering::SeqCst);
        assert_eq!(rc, SEED_PORT_RC_PANICKED);
        assert!(
            registered_seed_port().is_some(),
            "a panicked clear clears nothing"
        );
    }

    #[test]
    fn round_trips_a_max_length_seed_inclusive_ceiling() {
        // The SeedSource bound is 32..=252 INCLUSIVE; pin the upper boundary so a future
        // off-by-one in the range check or the slice is caught. A 252-byte seed must
        // round-trip byte-identical, exactly like the 32-byte floor.
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        WRITE_LEN.store(SEED_MAX_BYTES, Ordering::SeqCst);
        assert_eq!(
            zec_wallet_register_seed_port(ptr::null_mut(), Some(supply_always)),
            0
        );
        let seed = CAbiSeedPort
            .provide_seed(None)
            .expect("max-length staged seed");
        let expected: Vec<u8> = (0..SEED_MAX_BYTES)
            .map(|i| (i as u8).wrapping_add(1))
            .collect();
        assert_eq!(
            seed.len(),
            SEED_MAX_BYTES,
            "the inclusive 252-byte ceiling is accepted"
        );
        assert_eq!(&seed[..], &expected[..], "all 252 bytes round-trip exactly");
    }

    #[test]
    fn round_trips_the_exact_staged_seed_and_threads_ctx() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        let ctx = 0xC0FFEE_usize as *mut c_void;
        assert_eq!(zec_wallet_register_seed_port(ctx, Some(supply_always)), 0);

        let seed = CAbiSeedPort.provide_seed(None).expect("staged seed");
        let expected: Vec<u8> = (1..=32u8).collect();
        assert_eq!(
            &seed[..],
            &expected[..],
            "the C-ABI delivers the exact bytes the host wrote — fidelity is non-negotiable for a seed"
        );
        assert_eq!(
            RECEIVED_CTX.load(Ordering::SeqCst),
            0xC0FFEE,
            "the opaque ctx threads through to the host callback unchanged"
        );
    }

    #[test]
    fn unavailable_when_no_port_registered() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        assert!(registered_seed_port().is_none());
        assert_eq!(
            CAbiSeedPort.provide_seed(None).unwrap_err(),
            SeedSupplyError::Unavailable
        );
    }

    #[test]
    fn negative_return_is_unavailable_no_oracle() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        OVERRIDE_CODE.store(-1, Ordering::SeqCst);
        assert_eq!(
            zec_wallet_register_seed_port(ptr::null_mut(), Some(supply_always)),
            0
        );
        // Denied / Unavailable / an FR-17 refusal are indistinguishable downstream.
        assert_eq!(
            CAbiSeedPort.provide_seed(None).unwrap_err(),
            SeedSupplyError::Unavailable
        );
    }

    #[test]
    fn out_of_bound_lengths_are_rejected_fail_closed() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // 0 and 31 are under the floor; 253 and 4096 are over the ceiling. None may
        // produce a "seed" (and over-report must never trigger an OOB read).
        for bad in [
            0isize,
            (SEED_MIN_BYTES as isize) - 1,
            (SEED_MAX_BYTES as isize) + 1,
            4096,
        ] {
            reset();
            OVERRIDE_CODE.store(bad, Ordering::SeqCst);
            assert_eq!(
                zec_wallet_register_seed_port(ptr::null_mut(), Some(supply_always)),
                0
            );
            assert_eq!(
                CAbiSeedPort.provide_seed(None).unwrap_err(),
                SeedSupplyError::Unavailable,
                "a reported length of {bad} must be rejected fail-closed"
            );
        }
    }

    #[test]
    fn send_scope_is_take_once() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        assert_eq!(
            zec_wallet_register_seed_port(ptr::null_mut(), Some(supply_take_once)),
            0
        );
        // The one signature succeeds…
        let first = CAbiSeedPort
            .provide_seed(None)
            .expect("first pull (the signature)");
        assert_eq!(first.len(), 32);
        // …and a second pull against the same staged send is Unavailable.
        assert_eq!(
            CAbiSeedPort.provide_seed(None).unwrap_err(),
            SeedSupplyError::Unavailable,
            "a second pull against a take-once send-stage must not yield a second sign"
        );
        assert_eq!(
            PULLS.load(Ordering::SeqCst),
            2,
            "the wrapper pulled exactly twice"
        );
    }

    // ── FR-17: the bound supplier sees the binding verbatim ──────────────────

    #[test]
    fn bound_supplier_receives_the_binding_bytes_verbatim() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        register_bound_with_token();
        let binding = SpendBinding::mint();
        let expected_prefix = u32::from_le_bytes([
            binding.as_bytes()[0],
            binding.as_bytes()[1],
            binding.as_bytes()[2],
            binding.as_bytes()[3],
        ]) as usize;
        let seed = CAbiSeedPort
            .provide_seed(Some(&binding))
            .expect("bound pull with a matching supplier");
        assert_eq!(seed.len(), 32);
        assert_eq!(
            SAW_BINDING_LEN.load(Ordering::SeqCst),
            SPEND_BINDING_BYTES,
            "the callback saw the full fixed-width binding"
        );
        assert_eq!(
            SAW_BINDING_PREFIX.load(Ordering::SeqCst),
            expected_prefix,
            "the binding bytes cross the C-ABI verbatim — the host compares EXACTLY \
             what the SDK is signing"
        );
    }

    #[test]
    fn unbound_pull_presents_null_and_zero_to_a_bound_supplier() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        register_bound_with_token();
        CAbiSeedPort
            .provide_seed(None)
            .expect("unbound pull (import/sweep/reclaim/refund-derive)");
        assert_eq!(
            SAW_BINDING_LEN.load(Ordering::SeqCst),
            0,
            "an unbound pull is null/0 — never a fabricated token"
        );
    }

    #[test]
    fn v1_supplier_still_signs_a_bound_pull_binding_dropped() {
        // A plain v1 supplier has no binding parameter — a bound pull works but the
        // binding is dropped (the documented no-enforcement posture; the host opts
        // into FR-17 by registering the bound shape).
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        assert_eq!(
            zec_wallet_register_seed_port(ptr::null_mut(), Some(supply_always)),
            0
        );
        let binding = SpendBinding::mint();
        let seed = CAbiSeedPort
            .provide_seed(Some(&binding))
            .expect("v1 supplier serves a bound pull (no enforcement)");
        assert_eq!(seed.len(), 32);
    }

    // ── FR-18: the guarded registry ──────────────────────────────────────────

    #[test]
    fn second_registration_is_rejected_and_the_first_stands() {
        // THE FR-18 acceptance: a later in-process registrant cannot replace the
        // supplier — by re-register OR by the old null-clear primitive.
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        let ctx_first = 0xA11CE_usize as *mut c_void;
        assert_eq!(
            zec_wallet_register_seed_port(ctx_first, Some(supply_always)),
            0
        );
        // A second v1 register (a would-be substituting library) is refused…
        assert_eq!(
            zec_wallet_register_seed_port(0xBAD_usize as *mut c_void, Some(supply_take_once)),
            -2
        );
        // …the null-supply CLEAR (the old substitution primitive) is refused…
        assert_eq!(zec_wallet_register_seed_port(ptr::null_mut(), None), -2);
        // …a bound register against the occupied slot is refused too…
        let rc = unsafe {
            zec_wallet_register_seed_port_bound(
                ptr::null_mut(),
                Some(supply_bound_recording),
                ptr::null_mut(),
            )
        };
        assert_eq!(rc, -2);
        // …and the FIRST registration still serves, with ITS ctx.
        let seed = CAbiSeedPort.provide_seed(None).expect("first still stands");
        assert_eq!(seed.len(), 32);
        assert_eq!(RECEIVED_CTX.load(Ordering::SeqCst), 0xA11CE);
    }

    #[test]
    fn occupied_bound_register_leaves_auth_out_untouched() {
        // review fold (H-1, the fold's own regression): a benign in-process
        // re-register (activity recreate / hot-restart) that gets `-2` must NOT
        // scribble the host's `auth_out` — else the SECOND call overwrites the
        // token the host is still holding from its FIRST, winning registration,
        // and every later token-gated update/clear is then dead.
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        let live_token = register_bound_with_token(); // the first (winning) registration
        // The host re-inits into the SAME buffer where it kept its live token.
        let mut buf = live_token;
        let rc = unsafe {
            zec_wallet_register_seed_port_bound(
                ptr::null_mut(),
                Some(supply_bound_second),
                buf.as_mut_ptr(),
            )
        };
        assert_eq!(rc, -2, "the occupied slot rejects the re-register");
        assert_eq!(
            buf, live_token,
            "the -2 path must leave auth_out byte-identical — the host's live token survives"
        );
        // And the ORIGINAL token still authorizes a clear (it was never clobbered).
        let rc = unsafe { zec_wallet_update_seed_port(live_token.as_ptr(), ptr::null_mut(), None) };
        assert_eq!(
            rc, 0,
            "the un-clobbered original token still gates the update verb"
        );
    }

    #[test]
    fn token_gated_update_replaces_and_wrong_token_is_rejected() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        let token = register_bound_with_token();
        // A wrong token cannot replace (constant-time compare; one shared -2).
        let mut wrong = token;
        wrong[0] ^= 0xFF;
        let rc = unsafe {
            zec_wallet_update_seed_port(wrong.as_ptr(), ptr::null_mut(), Some(supply_bound_second))
        };
        assert_eq!(rc, -2, "a wrong token must not mutate the slot");
        // The right token replaces; the new supplier (64-byte writer) serves.
        let rc = unsafe {
            zec_wallet_update_seed_port(token.as_ptr(), ptr::null_mut(), Some(supply_bound_second))
        };
        assert_eq!(rc, 0);
        let seed = CAbiSeedPort.provide_seed(None).expect("replaced supplier");
        assert_eq!(
            seed.len(),
            64,
            "the REPLACEMENT supplier is the one serving"
        );
    }

    #[test]
    fn token_gated_clear_empties_the_slot_and_invalidates_the_token() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        let token = register_bound_with_token();
        // CLEAR (identity switch / shutdown).
        let rc = unsafe { zec_wallet_update_seed_port(token.as_ptr(), ptr::null_mut(), None) };
        assert_eq!(rc, 0);
        assert!(registered_seed_port().is_none(), "the slot is empty");
        assert_eq!(
            CAbiSeedPort.provide_seed(None).unwrap_err(),
            SeedSupplyError::Unavailable
        );
        // The OLD token authorizes nothing anymore (a fresh register mints anew).
        let rc = unsafe {
            zec_wallet_update_seed_port(
                token.as_ptr(),
                ptr::null_mut(),
                Some(supply_bound_recording),
            )
        };
        assert_eq!(rc, -2, "a cleared token is dead — no resurrection path");
    }

    #[test]
    fn null_auth_out_registration_is_permanent() {
        // Opting out of the token (null auth_out) = v1-style permanence: no token
        // exists, so NO update can ever mutate the slot.
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        let rc = unsafe {
            zec_wallet_register_seed_port_bound(
                ptr::null_mut(),
                Some(supply_bound_recording),
                ptr::null_mut(),
            )
        };
        assert_eq!(rc, 0);
        let guess = [0u8; SEED_PORT_AUTH_TOKEN_BYTES];
        let rc = unsafe { zec_wallet_update_seed_port(guess.as_ptr(), ptr::null_mut(), None) };
        assert_eq!(
            rc, -2,
            "a permanent registration has no mutation capability"
        );
        assert!(registered_seed_port().is_some(), "the registration stands");
    }

    #[test]
    fn distinct_registrations_mint_distinct_tokens() {
        // The token is a random capability, not a derived constant — two lives of
        // the slot must not share one (a captured old token must not open a new
        // registration; the collision probability is 2^-256, so equality ⇒ broken RNG).
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        let first = register_bound_with_token();
        let rc = unsafe { zec_wallet_update_seed_port(first.as_ptr(), ptr::null_mut(), None) };
        assert_eq!(rc, 0);
        let second = register_bound_with_token();
        assert_ne!(first, second, "fresh registration ⇒ fresh token");
    }
}
