//! FR-29 — the host transport crossing: the SDK's half of
//! `include/zec_wallet_net_dialer.h` (the CONTRACT; on any disagreement with
//! prose the header wins), designed in `docs/specs/host-transport-crossing.md`
//! §3.1–§3.5 and ruled by ADR-0543…0546.
//!
//! ## Why this module exists
//! Relim consumes the SDK as a separate native library (Relim ADR-0031), so
//! the core's `TorRuntime::Dialer(Arc<dyn NetDialer>)` — a Rust trait object —
//! cannot reach it, and the messenger rides Tor while the wallet syncs from
//! the real IP. The host therefore REGISTERS a dialer across the library
//! boundary (ADR-0543: a registered dialer, no port opened by the host, the
//! FR-15 seed-port pattern), and this module adapts that C vtable to the
//! core's [`NetDialer`] + [`HostDialer`] as [`CAbiHostDialer`], a ZST over a
//! process-global registry exactly like `seed_port_cabi::CAbiSeedPort`.
//!
//! ## The shape (spec §3.2, D4)
//! Two tokio runtimes and two copies of std live in one process, so the
//! stream crosses as a COMPLETION-CALLBACK ABI: the SDK calls the host's
//! `dial`/`read`/`write`/`close`; the host completes on ITS threads through
//! the two SDK-side `extern "C"` callbacks below, which only store a result
//! and wake a waker — the SDK never enters its runtime from a host thread and
//! never re-enters the host from inside a completion.
//!
//! - **The registry** — one slot: a COPY of the host's vtable, the validated
//!   descriptor, the 32-byte auth token, and a GENERATION every replace, clear
//!   and retire bumps. First-wins + token-gated (`ZW_RC_OCCUPIED` is ONE code
//!   for occupied/unauthorized — no probe oracle), replaceable only through the
//!   token (ADR-0544 D4).
//! - **The op table** — every dial/read/write is an SDK-minted op id whose
//!   record (generation, owned buffer, waker, one-way state
//!   `Pending → Completed | Closed`) is INSERTED BEFORE the host verb is
//!   called and re-checked after it returns, so a completion that fires on the
//!   calling thread before the verb returns is never lost (the reviewer's
//!   MAJOR, T23). Buffers are owned by records and leave the table only with
//!   them — never while a host op is outstanding (T18).
//! - **`HostStream`** — one outstanding read and one outstanding write at a
//!   time over owned 16 KiB chunks; every integer the host reports is validated
//!   (`0 <= n <= cap`, a non-zero handle) before a byte is trusted; the write
//!   re-present rule is CHECKED (T24); a stale generation fails typed
//!   `Retired` without calling the host (T6).
//!
//! ## Unsafe posture
//! The bridge's second handwritten module with scoped `unsafe` (`lib.rs`):
//! calling a host-supplied function pointer and reading/writing through a raw
//! host pointer is irreducibly `unsafe`. Every such site carries a SAFETY
//! comment naming the host contract it relies on; the registry lock is never
//! held across a host call; every export and both completion callbacks run
//! behind `catch_unwind` (spec D7). The core stays `#![forbid(unsafe_code)]`.

use std::collections::HashMap;
use std::ffi::c_void;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, RwLock};
use std::task::{Context, Poll, Waker};

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use zec_wallet_core::{
    AsyncByteStream, DialError, HOST_DIALER_ABI_VERSION, HOST_TRANSPORT_NAME_MAX_BYTES,
    HostDialCode, HostDialer, HostTransportDescriptor, NetDialer, Zeroizing,
};

// ── The header's constants (T15 pins each against the parsed header) ────────

/// `ZW_NET_DIALER_AUTH_TOKEN_BYTES` — the registration auth token width.
pub const NET_DIALER_AUTH_TOKEN_BYTES: usize = 32;
/// `ZW_HOST_NAME_MAX_BYTES` — the DNS presentation-form maximum (RFC 1035).
pub const HOST_NAME_MAX_BYTES: usize = 253;
/// `ZW_ISOLATION_KEY_MAX_BYTES` — the bound the host's isolation table enforces.
pub const ISOLATION_KEY_MAX_BYTES: usize = 256;

/// The verb return codes (`ZW_RC_*`). Non-zero is FATAL to that call (FR-15b).
pub const ZW_RC_OK: i32 = 0;
pub const ZW_RC_POISONED: i32 = -1;
pub const ZW_RC_OCCUPIED: i32 = -2;
pub const ZW_RC_NULL_ARG: i32 = -3;
pub const ZW_RC_PANICKED: i32 = -4;
pub const ZW_RC_ABI: i32 = -5;
pub const ZW_RC_DESCRIPTOR: i32 = -6;

/// One read chunk. WHY 16 384: one TLS record (RFC 8446 §5.1's plaintext
/// maximum) — rustls consumes records whole, so a larger chunk buys nothing
/// and a smaller one splits records into extra round trips through the host.
pub const HOST_STREAM_READ_CHUNK_BYTES: usize = 16 * 1024;
/// One write chunk — the same record-sized bound, for the same reason.
pub const HOST_STREAM_WRITE_CHUNK_BYTES: usize = 16 * 1024;

// ── The C types (`#[repr(C)]`, mirroring the header field for field) ────────

/// `zw_transport_descriptor` (ABI v3, ADR-0547 + ADR-0549): the host's own
/// name for its transport as a fixed `uint8_t[32]` + `name_len`, then four
/// closed `uint32_t`s — validated at the crossing by [`descriptor_from_raw`].
/// 52 bytes, 4-byte aligned; T15 pins the layout against the header.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZwTransportDescriptor {
    pub name: [u8; HOST_TRANSPORT_NAME_MAX_BYTES],
    pub name_len: u32,
    pub readiness: u32,
    pub isolation: u32,
    pub exposure: u32,
    /// `ZW_HEALTH_*` (v3). A registrant that has judged its transport FAILED
    /// says so here instead of flooring `readiness` to 0.
    pub health: u32,
}

/// The ONE place a raw C descriptor becomes the core's validated one.
/// `name_len` is bounded BEFORE the name bytes are sliced (spec §4 — a
/// `name_len` past the array must never index it); the core then applies the
/// name's four rules and the four closed ranges. `None` is `ZW_RC_DESCRIPTOR`.
fn descriptor_from_raw(raw: &ZwTransportDescriptor) -> Option<HostTransportDescriptor> {
    let len = usize::try_from(raw.name_len).ok()?;
    let name = raw.name.get(..len)?;
    HostTransportDescriptor::from_raw(name, raw.readiness, raw.isolation, raw.exposure, raw.health)
}

/// `zw_dial_complete_fn` — the SDK's; the host calls it exactly once per
/// accepted dial, on any thread, possibly before `dial` returns.
pub type ZwDialCompleteFn = extern "C" fn(sdk_ctx: *mut c_void, op_id: u64, code: u32, stream: u64);
/// `zw_io_complete_fn` — the SDK's; once per accepted read/write.
pub type ZwIoCompleteFn = extern "C" fn(sdk_ctx: *mut c_void, op_id: u64, code: u32, n: usize);

/// The host's `dial` (see the header for the pointer lifetimes: `host` and
/// `isolation_key` are valid for the duration of the CALL only).
pub type ZwDialFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    host: *const u8,
    host_len: usize,
    port: u16,
    isolation_key: *const u8,
    isolation_key_len: usize,
    op_id: u64,
    sdk_ctx: *mut c_void,
    complete: ZwDialCompleteFn,
) -> u32;
/// The host's `read`: `buf` is valid for `cap` writes until the completion.
pub type ZwReadFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    stream: u64,
    buf: *mut u8,
    cap: usize,
    op_id: u64,
    sdk_ctx: *mut c_void,
    complete: ZwIoCompleteFn,
) -> u32;
/// The host's `write`: `buf` is valid for `len` reads until the completion.
pub type ZwWriteFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    stream: u64,
    buf: *const u8,
    len: usize,
    op_id: u64,
    sdk_ctx: *mut c_void,
    complete: ZwIoCompleteFn,
) -> u32;
/// The host's `close`: cancel + release; idempotent.
pub type ZwCloseFn = unsafe extern "C" fn(ctx: *mut c_void, stream: u64);

/// `zw_net_dialer_v1`. Every entry is REQUIRED — a null one is `ZW_RC_NULL_ARG`
/// (the `Option<fn>` is the FFI-safe nullable pointer).
#[repr(C)]
pub struct ZwNetDialerV1 {
    pub ctx: *mut c_void,
    pub dial: Option<ZwDialFn>,
    pub read: Option<ZwReadFn>,
    pub write: Option<ZwWriteFn>,
    pub close: Option<ZwCloseFn>,
}

// ── The registry ────────────────────────────────────────────────────────────

/// The registered vtable, COPIED out of the host's struct. `ctx` is held as a
/// `usize` so the record is `Send + Sync` without an `unsafe impl` (the host
/// owns the pointer's lifetime and thread-safety while registered) and cast
/// back only at a call site. All fields are `Copy`: every host call runs on a
/// snapshot taken out of the lock.
#[derive(Clone, Copy)]
struct Registration {
    ctx: usize,
    dial: ZwDialFn,
    read: ZwReadFn,
    write: ZwWriteFn,
    close: ZwCloseFn,
}

impl Registration {
    /// `None` when any required entry is null (`ZW_RC_NULL_ARG`).
    fn from_vtable(vt: &ZwNetDialerV1) -> Option<Self> {
        Some(Self {
            ctx: vt.ctx as usize,
            dial: vt.dial?,
            read: vt.read?,
            write: vt.write?,
            close: vt.close?,
        })
    }

    fn ctx(&self) -> *mut c_void {
        self.ctx as *mut c_void
    }
}

/// The one slot. `guard` is the host-held token gating every mutation; it
/// zeroizes on drop (an in-process capability, not key material — hygiene).
struct RegistryState {
    registration: Option<Registration>,
    descriptor: Option<HostTransportDescriptor>,
    guard: Option<Zeroizing<[u8; NET_DIALER_AUTH_TOKEN_BYTES]>>,
}

fn registry() -> &'static RwLock<RegistryState> {
    static REGISTRY: OnceLock<RwLock<RegistryState>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        RwLock::new(RegistryState {
            registration: None,
            descriptor: None,
            guard: None,
        })
    })
}

/// The registry GENERATION (spec §3.2 "Generation"): bumped — under the
/// registry write lock — by every replace, clear and retire. Every op record
/// and every [`HostStream`] carries the generation it was minted under; a
/// mismatch is `Retired`, typed, without a host call. Read lock-free on every
/// poll. Starts at 1 so 0 is never a live generation.
static GENERATION: AtomicU64 = AtomicU64::new(1);

fn current_generation() -> u64 {
    GENERATION.load(Ordering::SeqCst)
}

/// Callers hold the registry WRITE lock (the bump and the slot change are
/// one event to a reader that takes the read lock; a lock-free poll may see
/// either order, and both are honest — a stale-by-one poll re-checks next
/// time).
fn bump_generation() {
    GENERATION.fetch_add(1, Ordering::SeqCst);
}

/// Constant-time token equality (`subtle`, the seed port's barrier'd form).
fn ct_eq(a: &[u8; NET_DIALER_AUTH_TOKEN_BYTES], b: &[u8; NET_DIALER_AUTH_TOKEN_BYTES]) -> bool {
    subtle::ConstantTimeEq::ct_eq(&a[..], &b[..]).into()
}

/// Mint a fresh auth token (OS CSPRNG — the same pinned `rand_core` the core
/// draws from). The audit's one panic site (RNG failure) — it runs inside
/// `guarded`, before any lock.
fn mint_auth_token() -> Zeroizing<[u8; NET_DIALER_AUTH_TOKEN_BYTES]> {
    let mut token = Zeroizing::new([0u8; NET_DIALER_AUTH_TOKEN_BYTES]);
    rand_core::RngCore::fill_bytes(&mut rand_core::OsRng, token.as_mut());
    token
}

/// The panic boundary every export runs behind (spec D7; the
/// `seed_port_cabi::guarded` shape, same code, same two stated limits: it
/// needs `panic = "unwind"`, and the default hook has printed the message
/// before the catch — so nothing in these verbs formats a secret into a
/// panic). `AssertUnwindSafe` because the bodies capture raw host pointers;
/// nothing they touch is observable half-written — the slot commits under one
/// lock, and `auth_out` is written only after that commit.
fn guarded(body: impl FnOnce() -> i32) -> i32 {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)).unwrap_or(ZW_RC_PANICKED)
}

#[cfg(test)]
static PLANT_VERB_PANIC: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[cfg(test)]
static PLANT_COMPLETION_PANIC: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Test-only planted panic inside a verb's boundary, before any lock (T12).
#[inline]
fn planted_verb_panic_point() {
    #[cfg(test)]
    if PLANT_VERB_PANIC.load(Ordering::SeqCst) {
        panic!("planted panic inside a net-dialer verb (test)");
    }
}

/// Test-only planted panic inside a completion callback's boundary (T12).
#[inline]
fn planted_completion_panic_point() {
    #[cfg(test)]
    if PLANT_COMPLETION_PANIC.load(Ordering::SeqCst) {
        panic!("planted panic inside a net-dialer completion (test)");
    }
}

/// The §5 descriptor event — the TWO mechanical values, nothing else, and
/// NEVER the host's transport name (ADR-0547: host-chosen text stays off every
/// log line); transport-specific field names (a bare field in the crate-wide
/// §5.4 allowlist would admit a future `kind = <hostname>` — the wave review's
/// LOW). `tracing_guard`'s T16 pins this body at its source.
///
/// `transport_health` joined at C1 (ABI v3) for a diagnosis reason, not a
/// completeness one: without it a registrant that declares FAILED at its
/// measured readiness logs `transport_readiness=100` while the wallet renders
/// `Unavailable`, so the log says "ready" for exactly the state the user is
/// being told is broken — FR-30 (b)'s own shape, relocated into the log
/// stream, and unreadable on the H-6 walk where it would matter most.
///
/// WHY `isolation` AND `exposure` ARE NOT HERE, and must not come back (found
/// by Relim,; a joint change, their push line drops the same pair).
/// Keeping the host's NAME off this line did not close the disclosure it was
/// meant to close. For the one registrant that exists, `(exposure, isolation)`
/// is a BIJECTION onto its three transports — Tor (1,1), Shadowsocks (1,2),
/// Direct (2,2) — so a holder of this log recovered which circumvention
/// product carried the traffic from two integers, with no name anywhere.
/// ADR-0547's promise is about host-chosen TEXT, so this line satisfied its
/// letter and leaked the same fact.
///
/// **Re-encoding does not help, which is the part worth keeping**: any field
/// that tells a diagnostician the privacy GRADE separates those three, because
/// the grade is exactly what distinguishes them — "hides address / linkable"
/// is `(1,2)` in better clothes. So the question is not the encoding but
/// whether the grade belongs on an UNCONSENTED artefact at all. It does not.
/// This line keeps the mechanical facts a lifecycle bug needs — which write it
/// was, readiness, health — and the grade lives in the host's debug bundle,
/// where the maintainer ruled collection is broad and CONSENTED and
/// joinability is wanted.
///
/// Both fields still CROSS the ABI and must: the wallet renders "connections
/// can be linked" from `isolation` and its honest privacy line from
/// `exposure`. Nothing here is about the descriptor; it is about the log.
///
/// The guard is the FIELD-SET PIN in `tracing_guard`'s T16, not a property of
/// today's roster: a fourth backing sharing a pair would NARROW the
/// disclosure, and one with a distinct pair would widen it, so an injectivity
/// check would alarm on the safe direction and sleep through the dangerous one.
/// Pinning the field set holds whatever the roster does.
fn log_descriptor(d: &HostTransportDescriptor) {
    tracing::info!(
        target: "zec_wallet",
        transport_readiness = d.readiness,
        transport_health = ?d.health,
        "wallet.host_dialer_descriptor"
    );
}

/// Read the host's 32-byte token into a zeroizing local.
///
/// # Safety
/// `auth` is non-null and valid for [`NET_DIALER_AUTH_TOKEN_BYTES`] reads.
unsafe fn read_token(auth: *const u8) -> Zeroizing<[u8; NET_DIALER_AUTH_TOKEN_BYTES]> {
    let mut buf = Zeroizing::new([0u8; NET_DIALER_AUTH_TOKEN_BYTES]);
    // SAFETY: the caller's contract above; a fresh local cannot overlap `auth`.
    unsafe {
        std::ptr::copy_nonoverlapping(auth, buf.as_mut_ptr(), NET_DIALER_AUTH_TOKEN_BYTES);
    }
    buf
}

/// Copy + validate a vtable and a descriptor the host passed by pointer.
/// `Err(code)` is the verb's return.
///
/// # Safety
/// Both pointers are non-null and valid for one read of their struct.
unsafe fn read_vtable_and_descriptor(
    dialer: *const ZwNetDialerV1,
    descriptor: *const ZwTransportDescriptor,
) -> Result<(Registration, HostTransportDescriptor), i32> {
    // SAFETY: the caller's contract — both non-null, each valid for one read
    // of a `#[repr(C)]` struct the header lays out; the host may free them
    // after the call (we COPY).
    let (vt, raw) = unsafe { (std::ptr::read(dialer), std::ptr::read(descriptor)) };
    let reg = Registration::from_vtable(&vt).ok_or(ZW_RC_NULL_ARG)?;
    let desc = descriptor_from_raw(&raw).ok_or(ZW_RC_DESCRIPTOR)?;
    Ok((reg, desc))
}

/// `zec_wallet_register_net_dialer` — first-wins into an empty slot; mints
/// the auth token into `auth_out` (non-null REQUIRED: there is no permanent
/// variant, a dialer that cannot be replaced breaks carrier switching, spec
/// §1.2 D2). `dialer` and `descriptor` are COPIED.
///
/// Returns `0` · `-1` lock poisoned · `-2` occupied (the first stands,
/// `auth_out` untouched) · `-3` a null argument or a null required vtable
/// entry · `-4` the SDK panicked (nothing changed) · `-5` ABI mismatch ·
/// `-6` a descriptor value out of its closed range.
///
/// # Safety
/// `dialer` and `descriptor` are valid for one read each; `auth_out` is valid
/// for [`NET_DIALER_AUTH_TOKEN_BYTES`] writes and unaliased with any concurrent
/// registration attempt. The vtable's `ctx` and functions stay valid while
/// registered.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn zec_wallet_register_net_dialer(
    abi_version: u32,
    dialer: *const ZwNetDialerV1,
    descriptor: *const ZwTransportDescriptor,
    auth_out: *mut u8,
) -> i32 {
    guarded(|| {
        planted_verb_panic_point();
        // The ABI check comes FIRST: a host built against another version may
        // lay its structs out differently, so no pointer is read before it.
        if abi_version != HOST_DIALER_ABI_VERSION {
            return ZW_RC_ABI;
        }
        if dialer.is_null() || descriptor.is_null() || auth_out.is_null() {
            return ZW_RC_NULL_ARG;
        }
        // SAFETY: both non-null (checked above); the caller's contract.
        let (reg, desc) = match unsafe { read_vtable_and_descriptor(dialer, descriptor) } {
            Ok(pair) => pair,
            Err(code) => return code,
        };
        // Minted into a LOCAL before the lock (the seed port's H-1 fold): the
        // host's pointer is written only after the slot commits and the lock
        // is released, so the `-1`/`-2` paths never touch `auth_out`.
        let minted = mint_auth_token();
        {
            let Ok(mut slot) = registry().write() else {
                return ZW_RC_POISONED;
            };
            if slot.registration.is_some() {
                return ZW_RC_OCCUPIED;
            }
            slot.registration = Some(reg);
            slot.descriptor = Some(desc);
            slot.guard = Some(minted.clone()); // the Zeroizing type stays at the call site (the wave review's LOW; the inner Copy is the same temporary either way — the code reviewer's correction)
        } // write lock RELEASED before any foreign write
        // SAFETY: caller contract — `auth_out` is non-null (checked), valid for
        // NET_DIALER_AUTH_TOKEN_BYTES writes and unaliased; the source is a
        // fresh local. Reached only on the success path, after the commit.
        unsafe {
            std::ptr::copy_nonoverlapping(minted.as_ptr(), auth_out, NET_DIALER_AUTH_TOKEN_BYTES);
        }
        tracing::info!(target: "zec_wallet", "wallet.host_dialer_registered");
        log_descriptor(&desc);
        ZW_RC_OK
    })
}

/// `zec_wallet_update_net_dialer` — token-gated REPLACE (both non-null) or
/// CLEAR (both null). Either bumps the generation: every in-flight op and
/// every open stream of the superseded backing fails typed `Retired` on its
/// next poll. A replace keeps the token; a clear invalidates it.
///
/// Returns `0` · `-1` · `-2` unauthorized (empty slot or wrong token — ONE
/// code, no oracle) · `-3` null `auth`, one of the pair null, or a null
/// required vtable entry · `-4` · `-6`.
///
/// # Safety
/// `auth` is valid for [`NET_DIALER_AUTH_TOKEN_BYTES`] reads; a non-null
/// `dialer`/`descriptor` is valid for one read each.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn zec_wallet_update_net_dialer(
    auth: *const u8,
    dialer: *const ZwNetDialerV1,
    descriptor: *const ZwTransportDescriptor,
) -> i32 {
    guarded(|| {
        planted_verb_panic_point();
        if auth.is_null() {
            return ZW_RC_NULL_ARG;
        }
        // SAFETY: non-null (checked); the caller's contract.
        let presented = unsafe { read_token(auth) };
        let replacement = match (dialer.is_null(), descriptor.is_null()) {
            (true, true) => None,
            // SAFETY: both non-null; the caller's contract.
            (false, false) => match unsafe { read_vtable_and_descriptor(dialer, descriptor) } {
                Ok(pair) => Some(pair),
                Err(code) => return code,
            },
            _ => return ZW_RC_NULL_ARG,
        };
        let replaced = {
            let Ok(mut slot) = registry().write() else {
                return ZW_RC_POISONED;
            };
            let authorized = match (&slot.registration, &slot.guard) {
                (Some(_), Some(expected)) => ct_eq(expected, &presented),
                _ => false,
            };
            if !authorized {
                return ZW_RC_OCCUPIED;
            }
            match replacement {
                Some((reg, desc)) => {
                    slot.registration = Some(reg);
                    slot.descriptor = Some(desc);
                    bump_generation();
                    Some(desc)
                }
                None => {
                    slot.registration = None;
                    slot.descriptor = None;
                    slot.guard = None;
                    bump_generation();
                    None
                }
            }
        };
        match replaced {
            Some(desc) => {
                tracing::info!(target: "zec_wallet", "wallet.host_dialer_replaced");
                log_descriptor(&desc);
            }
            None => tracing::info!(target: "zec_wallet", "wallet.host_dialer_cleared"),
        }
        ZW_RC_OK
    })
}

/// `zec_wallet_net_dialer_notify` — the readiness/retire PUSH. Stores the
/// descriptor; `retire != 0` additionally bumps the generation exactly as a
/// replace does, WITHOUT changing the registration (the host tore its backing
/// down and will re-arm behind the same trampoline).
///
/// Returns `0` · `-1` · `-2` unauthorized · `-3` null · `-4` · `-6`.
///
/// # Safety
/// `auth` is valid for [`NET_DIALER_AUTH_TOKEN_BYTES`] reads; `descriptor` for
/// one read.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn zec_wallet_net_dialer_notify(
    auth: *const u8,
    descriptor: *const ZwTransportDescriptor,
    retire: u32,
) -> i32 {
    guarded(|| {
        planted_verb_panic_point();
        if auth.is_null() || descriptor.is_null() {
            return ZW_RC_NULL_ARG;
        }
        // SAFETY: both non-null (checked); the caller's contract.
        let (presented, raw) = unsafe { (read_token(auth), std::ptr::read(descriptor)) };
        let Some(desc) = descriptor_from_raw(&raw) else {
            return ZW_RC_DESCRIPTOR;
        };
        {
            let Ok(mut slot) = registry().write() else {
                return ZW_RC_POISONED;
            };
            let authorized = match (&slot.registration, &slot.guard) {
                (Some(_), Some(expected)) => ct_eq(expected, &presented),
                _ => false,
            };
            if !authorized {
                return ZW_RC_OCCUPIED;
            }
            slot.descriptor = Some(desc);
            if retire != 0 {
                bump_generation();
            }
        }
        if retire != 0 {
            tracing::info!(target: "zec_wallet", "wallet.host_dialer_retired");
        }
        log_descriptor(&desc);
        ZW_RC_OK
    })
}

// ── The op table ────────────────────────────────────────────────────────────

/// What a completion delivered. `Host` carries the host's code and its value
/// (`n` for io, the stream handle for a dial); `SdkPanicked` is the boundary's
/// own record of a panic inside the callback (the op fails `Io`).
#[derive(Clone, Copy)]
enum OpOutcome {
    Host { code: u32, value: u64 },
    SdkPanicked,
}

/// The one-way state machine (spec §3.2): the first transition wins, the
/// second is a no-op — a `close` racing a completion on another thread is
/// exactly-once by construction (T25).
#[derive(Clone, Copy)]
enum OpState {
    Pending,
    Completed(OpOutcome),
    Closed,
}

#[derive(Clone, Copy)]
enum OpKind {
    /// A dial; the registration that accepted it, so a stream delivered to a
    /// closed record can be released to the SAME backing.
    Dial(Registration),
    Io,
}

struct OpRecord {
    generation: u64,
    /// OWNED here: the host writes into it (read) or reads from it (write)
    /// until the completion; it leaves the table only with the record.
    buffer: Vec<u8>,
    waker: Option<Waker>,
    state: OpState,
    kind: OpKind,
}

fn op_table() -> &'static Mutex<HashMap<u64, OpRecord>> {
    static OPS: OnceLock<Mutex<HashMap<u64, OpRecord>>> = OnceLock::new();
    OPS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The op table lock. A poisoned table (a panic inside the lock scope) is
/// recovered: every state is one-way and every buffer is owned by a record,
/// so nothing inside is observable half-written.
fn lock_ops() -> MutexGuard<'static, HashMap<u64, OpRecord>> {
    op_table().lock().unwrap_or_else(|e| e.into_inner())
}

/// A PROCESS counter, never reset: it counts the ops of every wallet this
/// process has opened, so an op id says nothing about which wallet minted it
/// and a closed wallet's ids are never reissued to the next one.
static NEXT_OP_ID: AtomicU64 = AtomicU64::new(1);

fn mint_op_id() -> u64 {
    NEXT_OP_ID.fetch_add(1, Ordering::SeqCst)
}

/// Insert the record BEFORE the host verb is called (spec §3.2 registration
/// order): a completion invoked inline finds it.
fn insert_op(op_id: u64, generation: u64, buffer: Vec<u8>, waker: Option<Waker>, kind: OpKind) {
    lock_ops().insert(
        op_id,
        OpRecord {
            generation,
            buffer,
            waker,
            state: OpState::Pending,
            kind,
        },
    );
}

/// A stream handle the SDK owes a `close` for, found on a record no future
/// was waiting on (spec §3.2 "Cancel": a delivered stream is closed at once —
/// no orphan). A completion callback must not re-enter the host, so the
/// callback parks it here and the next SDK-side host interaction sweeps it.
struct Orphan {
    reg: Registration,
    generation: u64,
    handle: u64,
}

fn orphans() -> &'static Mutex<Vec<Orphan>> {
    static ORPHANS: OnceLock<Mutex<Vec<Orphan>>> = OnceLock::new();
    ORPHANS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Release a delivered-but-unwanted stream to the backing that minted it —
/// only while that backing is still the live generation (a retired backing
/// tore its streams down itself; spec §3.2 "Generation").
fn release_handle(reg: Registration, generation: u64, handle: u64) {
    if generation != current_generation() {
        return;
    }
    // SAFETY: `reg` is a snapshot of a registration that was live under
    // `generation`, which is still current: the host keeps `ctx` and the
    // functions valid while registered. `close` is idempotent per the header.
    unsafe { (reg.close)(reg.ctx(), handle) }
}

/// Close every parked orphan. Called from SDK threads only (never inside a
/// completion).
fn sweep_orphans() {
    let parked: Vec<Orphan> = {
        let mut list = orphans().lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut *list)
    };
    for o in parked {
        release_handle(o.reg, o.generation, o.handle);
    }
}

/// The record's side of a completion: `Pending → Completed` and wake; a
/// `Closed` or stale record is removed (its buffer freed — the host is done
/// with it) and a delivered stream on it is parked and released at once
/// from a short-lived SDK thread.
fn record_completion(op_id: u64, outcome: OpOutcome) {
    let (waker, orphan) = {
        let mut ops = lock_ops();
        let Some(rec) = ops.get_mut(&op_id) else {
            return; // no record: a synchronous refusal already removed it — dropped
        };
        let stale = rec.generation != current_generation();
        let state = rec.state;
        match state {
            OpState::Pending if !stale => {
                rec.state = OpState::Completed(outcome);
                (rec.waker.take(), None)
            }
            OpState::Pending | OpState::Closed => {
                let Some(mut rec) = ops.remove(&op_id) else {
                    return;
                };
                let orphan = match (rec.kind, outcome) {
                    (
                        OpKind::Dial(reg),
                        OpOutcome::Host {
                            code: 0,
                            value: handle,
                        },
                    ) if handle != 0 => Some(Orphan {
                        reg,
                        generation: rec.generation,
                        handle,
                    }),
                    _ => None,
                };
                (rec.waker.take(), orphan)
            }
            // A second completion for one op: a host protocol violation —
            // dropped; the first transition stands.
            OpState::Completed(_) => (None, None),
        }
    };
    if let Some(o) = orphan {
        orphans().lock().unwrap_or_else(|e| e.into_inner()).push(o);
        // This runs on the HOST's thread, inside its completion, where
        // calling back into its `close` is not allowed — and a dial closed by
        // a wallet close may never see another SDK sweep (the process's last
        // wallet closed, nothing dials again). So one short-lived SDK thread
        // releases it now. A failed spawn leaves it parked for the next dial
        // or stream close, as before; the take in `sweep_orphans` makes the
        // release exactly-once whichever sweep gets there first.
        let _ = std::thread::Builder::new()
            .name("zw-orphan-release".into())
            .spawn(sweep_orphans);
    }
    if let Some(w) = waker {
        w.wake();
    }
}

/// The boundary both completion callbacks run behind (spec D7): a panic
/// inside records `Io` on the op — through the same one-way transition —
/// and wakes it; nothing unwinds into the host.
fn completion_boundary(op_id: u64, outcome: OpOutcome) {
    let first = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        planted_completion_panic_point();
        record_completion(op_id, outcome);
    }));
    if first.is_err() {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            record_completion(op_id, OpOutcome::SdkPanicked);
        }));
    }
}

/// The SDK's `zw_dial_complete_fn`. `sdk_ctx` is unused: the op id is the key
/// (no pointer is dereferenced on a host thread).
extern "C" fn dial_complete(_sdk_ctx: *mut c_void, op_id: u64, code: u32, stream: u64) {
    completion_boundary(
        op_id,
        OpOutcome::Host {
            code,
            value: stream,
        },
    );
}

/// The SDK's `zw_io_complete_fn`.
extern "C" fn io_complete(_sdk_ctx: *mut c_void, op_id: u64, code: u32, n: usize) {
    let value = u64::try_from(n).unwrap_or(u64::MAX);
    completion_boundary(op_id, OpOutcome::Host { code, value });
}

/// What a poll of an op record found.
enum Polled {
    Pending,
    Done {
        code: u32,
        value: u64,
        buffer: Vec<u8>,
    },
    /// The record is gone, closed, or the completion panicked: `Io`.
    Fault(&'static str),
}

/// Poll one record: `Pending` parks the waker; `Completed` is CONSUMED (the
/// record leaves the table with its buffer). The lock is released before the
/// caller acts, so a host call never runs under it.
fn poll_op(op_id: u64, cx: &Context<'_>) -> Polled {
    let mut ops = lock_ops();
    let Some(rec) = ops.get_mut(&op_id) else {
        return Polled::Fault("host dialer op record vanished before its completion was consumed");
    };
    match rec.state {
        OpState::Pending => {
            rec.waker = Some(cx.waker().clone());
            Polled::Pending
        }
        OpState::Closed => Polled::Fault("host dialer op polled after close"),
        OpState::Completed(OpOutcome::SdkPanicked) => {
            ops.remove(&op_id);
            Polled::Fault("the SDK panicked inside a host dialer completion")
        }
        OpState::Completed(OpOutcome::Host { code, value }) => match ops.remove(&op_id) {
            Some(rec) => Polled::Done {
                code,
                value,
                buffer: rec.buffer,
            },
            None => Polled::Fault("host dialer op record vanished mid-poll"),
        },
    }
}

/// The SDK's cancel of one op: `Pending → Closed` (the buffer stays until
/// the host completes); an already-`Completed` record is removed now (the
/// host is done with it). Returns a delivered stream handle the caller owes
/// a release for.
fn close_op(op_id: u64) -> Option<(Registration, u64, u64)> {
    let mut ops = lock_ops();
    let rec = ops.get_mut(&op_id)?;
    let state = rec.state;
    match state {
        OpState::Pending => {
            rec.state = OpState::Closed;
            rec.waker = None;
            None
        }
        OpState::Closed => None,
        OpState::Completed(outcome) => {
            let rec = ops.remove(&op_id)?;
            match (rec.kind, outcome) {
                (
                    OpKind::Dial(reg),
                    OpOutcome::Host {
                        code: 0,
                        value: handle,
                    },
                ) if handle != 0 => Some((reg, rec.generation, handle)),
                _ => None,
            }
        }
    }
}

/// The wallet close's side of the op table (FR-39, S2 §3.5a): every
/// `Pending` record goes `Closed` — the transition [`close_op`] makes — and
/// the parked orphans are released. NOTHING is freed here: a `Closed` record
/// keeps its buffer until the host's completion removes it (the host holds a
/// raw pointer into it from the verb to the completion), and a stream the
/// host delivers to a `Closed` dial afterwards is released by the completion's
/// own release thread (it cannot wait for a next sweep: after the last
/// wallet closes there may be none). The waker is taken and woken, so a straggling future polls,
/// finds `Closed` and fails typed instead of waiting on a wake that the
/// completion of a `Closed` record never sends.
///
/// PROCESS-wide, by construction: [`OpRecord`] carries no wallet key, so this
/// closes every outstanding op in the process, not the closing wallet's only.
/// With one wallet open per process (the supported shape) the two sets are
/// the same; a host that overlaps two wallets fails the survivor's in-flight
/// ops typed (`Io`), and its next dial proceeds. A wallet-scoped close is
/// owed (FR-39, partial).
pub(crate) fn close_outstanding_ops() {
    let wakers: Vec<Waker> = {
        let mut ops = lock_ops();
        ops.values_mut()
            .filter_map(|rec| match rec.state {
                OpState::Pending => {
                    rec.state = OpState::Closed;
                    rec.waker.take()
                }
                OpState::Completed(_) | OpState::Closed => None,
            })
            .collect()
    };
    for w in wakers {
        w.wake();
    }
    sweep_orphans();
}

// ── The dial future ─────────────────────────────────────────────────────────

/// Awaits one dial op. Dropping it (the `DIAL_TIMEOUT_SECS` bound in
/// `PolicyDialer` fired) never frees anything the host may still touch: the
/// record goes `Closed`, and a stream delivered afterwards is released.
struct DialOp {
    op_id: u64,
    reg: Registration,
    generation: u64,
    done: bool,
}

impl Future for DialOp {
    type Output = Result<HostStream, DialError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.generation != current_generation() {
            // Retired mid-dial: the record stays for the host's completion
            // (which frees it); the backing is gone, so nothing is released.
            self.done = true;
            close_op(self.op_id);
            return Poll::Ready(Err(DialError::Retired));
        }
        match poll_op(self.op_id, cx) {
            Polled::Pending => Poll::Pending,
            Polled::Fault(msg) => {
                self.done = true;
                Poll::Ready(Err(DialError::Io(std::io::Error::other(msg))))
            }
            Polled::Done { code, value, .. } => {
                self.done = true;
                if code != HostDialCode::Ok.as_raw() {
                    // THE ONE mapping (core's) — never a code→error table here.
                    return Poll::Ready(Err(HostDialCode::failure_from_raw(code)));
                }
                if value == 0 {
                    return Poll::Ready(Err(DialError::Io(std::io::Error::other(
                        "host dialer protocol violation: zero stream handle on a successful dial",
                    ))));
                }
                Poll::Ready(Ok(HostStream::new(value, self.reg, self.generation)))
            }
        }
    }
}

impl Drop for DialOp {
    fn drop(&mut self) {
        if self.done {
            return;
        }
        if let Some((reg, generation, handle)) = close_op(self.op_id) {
            release_handle(reg, generation, handle);
        }
    }
}

// ── The stream ──────────────────────────────────────────────────────────────

/// A terminal stream fault, re-issued on every later poll. `Code` is mapped
/// through the core's ONE table at render time.
#[derive(Clone, Copy)]
enum StreamFault {
    Retired,
    Code(u32),
    Protocol(&'static str),
    Shutdown,
}

impl StreamFault {
    fn to_io(self) -> std::io::Error {
        match self {
            Self::Retired => {
                std::io::Error::new(std::io::ErrorKind::ConnectionAborted, DialError::Retired)
            }
            Self::Code(code) => std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                HostDialCode::failure_from_raw(code),
            ),
            Self::Protocol(msg) => std::io::Error::new(std::io::ErrorKind::InvalidData, msg),
            Self::Shutdown => std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "host stream used after shutdown",
            ),
        }
    }
}

/// The `Box<dyn AsyncByteStream>` a host dial returns (spec §3.2): one
/// outstanding read and one outstanding write at a time, each over an owned
/// chunk the op record holds until the host completes it.
pub(crate) struct HostStream {
    handle: u64,
    reg: Registration,
    generation: u64,
    read_op: Option<u64>,
    write_op: Option<u64>,
    /// Bytes the host delivered that the caller has not drained yet
    /// (`poll_read` never advances past them).
    delivered: Option<(Vec<u8>, usize)>,
    eof: bool,
    fault: Option<StreamFault>,
    closed: bool,
}

impl HostStream {
    fn new(handle: u64, reg: Registration, generation: u64) -> Self {
        Self {
            handle,
            reg,
            generation,
            read_op: None,
            write_op: None,
            delivered: None,
            eof: false,
            fault: None,
            closed: false,
        }
    }

    /// A terminal fault: recorded once (the first wins) and the stream is
    /// closed at the host — the op fails typed and the stream closes (E7).
    fn fail(&mut self, fault: StreamFault) {
        if self.fault.is_none() {
            self.fault = Some(fault);
        }
        self.close_host();
    }

    /// The generation check every poll makes first: a superseded backing
    /// fails typed `Retired` WITHOUT a host call (T6). The outstanding
    /// records go `Closed` so the retiring host's completions free them.
    fn check_generation(&mut self) -> bool {
        if self.generation == current_generation() {
            return true;
        }
        if self.fault.is_none() {
            self.fault = Some(StreamFault::Retired);
        }
        self.closed = true;
        for op in [self.read_op.take(), self.write_op.take()]
            .into_iter()
            .flatten()
        {
            close_op(op);
        }
        false
    }

    /// The SDK's `close`: exactly once from this side; the outstanding ops go
    /// `Closed` (buffers kept until the host completes them), then the host's
    /// idempotent `close` — outside every lock — and the orphan sweep.
    fn close_host(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        for op in [self.read_op.take(), self.write_op.take()]
            .into_iter()
            .flatten()
        {
            close_op(op);
        }
        release_handle(self.reg, self.generation, self.handle);
        sweep_orphans();
    }

    fn ready_fault(&self) -> Option<std::io::Error> {
        self.fault.map(StreamFault::to_io)
    }
}

impl AsyncRead for HostStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        dst: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = &mut *self;
        loop {
            if let Some(err) = this.ready_fault() {
                return Poll::Ready(Err(err));
            }
            if !this.check_generation() {
                continue;
            }
            if let Some((bytes, pos)) = this.delivered.as_mut() {
                let take = (bytes.len() - *pos).min(dst.remaining());
                dst.put_slice(&bytes[*pos..*pos + take]);
                *pos += take;
                if *pos == bytes.len() {
                    this.delivered = None;
                }
                return Poll::Ready(Ok(()));
            }
            if this.eof {
                return Poll::Ready(Ok(()));
            }
            if this.closed {
                return Poll::Ready(Err(StreamFault::Shutdown.to_io()));
            }
            match this.read_op {
                Some(op_id) => match poll_op(op_id, cx) {
                    Polled::Pending => return Poll::Pending,
                    Polled::Fault(msg) => {
                        this.read_op = None;
                        this.fail(StreamFault::Protocol(msg));
                    }
                    Polled::Done {
                        code,
                        value,
                        mut buffer,
                    } => {
                        this.read_op = None;
                        if code != HostDialCode::Ok.as_raw() {
                            this.fail(StreamFault::Code(code));
                            continue;
                        }
                        // `0 <= n <= cap` is VALIDATED before a byte is trusted
                        // (the audit's read-length UB row); a violation fails
                        // the op typed and closes the stream (E7).
                        let n = match usize::try_from(value) {
                            Ok(n) if n <= buffer.len() => n,
                            _ => {
                                this.fail(StreamFault::Protocol(
                                    "host dialer protocol violation: read completion past cap",
                                ));
                                continue;
                            }
                        };
                        if n == 0 {
                            this.eof = true;
                            continue;
                        }
                        buffer.truncate(n);
                        this.delivered = Some((buffer, 0));
                    }
                },
                None => {
                    let op_id = mint_op_id();
                    let mut buffer = vec![0u8; HOST_STREAM_READ_CHUNK_BYTES];
                    let ptr = buffer.as_mut_ptr();
                    let cap = buffer.len();
                    // The record — with the buffer and the waker — is in the
                    // table BEFORE the host verb runs (spec §3.2).
                    insert_op(
                        op_id,
                        this.generation,
                        buffer,
                        Some(cx.waker().clone()),
                        OpKind::Io,
                    );
                    // SAFETY: `ptr`/`cap` name the heap allocation the record
                    // now owns (a `Vec` move does not move its heap block); the
                    // header's buffer rule keeps it valid until the completion
                    // and the record frees it only then. `reg` is a live
                    // registration under this stream's generation (checked
                    // above); `ctx` is the host's own pointer.
                    let rc = unsafe {
                        (this.reg.read)(
                            this.reg.ctx(),
                            this.handle,
                            ptr,
                            cap,
                            op_id,
                            std::ptr::null_mut(),
                            io_complete,
                        )
                    };
                    if rc != HostDialCode::Ok.as_raw() {
                        // Refused synchronously: NO completion follows; the
                        // record (and its buffer) leave the table now.
                        lock_ops().remove(&op_id);
                        this.fail(StreamFault::Code(rc));
                        continue;
                    }
                    this.read_op = Some(op_id);
                    // Re-check at once: an inline completion is consumed now,
                    // never left `Pending` over a completed op (T23).
                }
            }
        }
    }
}

impl AsyncWrite for HostStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        src: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = &mut *self;
        loop {
            if let Some(err) = this.ready_fault() {
                return Poll::Ready(Err(err));
            }
            if !this.check_generation() {
                continue;
            }
            if this.closed {
                return Poll::Ready(Err(StreamFault::Shutdown.to_io()));
            }
            match this.write_op {
                Some(op_id) => match poll_op(op_id, cx) {
                    Polled::Pending => return Poll::Pending,
                    Polled::Fault(msg) => {
                        this.write_op = None;
                        this.fail(StreamFault::Protocol(msg));
                    }
                    Polled::Done {
                        code,
                        value,
                        buffer,
                    } => {
                        this.write_op = None;
                        if code != HostDialCode::Ok.as_raw() {
                            this.fail(StreamFault::Code(code));
                            continue;
                        }
                        let n = match usize::try_from(value) {
                            Ok(n) if n <= buffer.len() => n,
                            _ => {
                                this.fail(StreamFault::Protocol(
                                    "host dialer protocol violation: write completion past len",
                                ));
                                continue;
                            }
                        };
                        // The write re-present rule, CHECKED (T24): the bytes
                        // the host consumed must be the bytes the caller now
                        // claims it wrote — a wrong byte is never silently
                        // committed to the wire.
                        if src.len() < n || src[..n] != buffer[..n] {
                            this.fail(StreamFault::Protocol("write re-present violation"));
                            continue;
                        }
                        return Poll::Ready(Ok(n));
                    }
                },
                None => {
                    if src.is_empty() {
                        return Poll::Ready(Ok(0));
                    }
                    let op_id = mint_op_id();
                    let take = src.len().min(HOST_STREAM_WRITE_CHUNK_BYTES);
                    let buffer = src[..take].to_vec();
                    let ptr = buffer.as_ptr();
                    insert_op(
                        op_id,
                        this.generation,
                        buffer,
                        Some(cx.waker().clone()),
                        OpKind::Io,
                    );
                    // SAFETY: as in `poll_read` — the record owns the heap
                    // block `ptr`/`take` name until the completion; `reg` is
                    // live under this generation.
                    let rc = unsafe {
                        (this.reg.write)(
                            this.reg.ctx(),
                            this.handle,
                            ptr,
                            take,
                            op_id,
                            std::ptr::null_mut(),
                            io_complete,
                        )
                    };
                    if rc != HostDialCode::Ok.as_raw() {
                        lock_ops().remove(&op_id);
                        this.fail(StreamFault::Code(rc));
                        continue;
                    }
                    this.write_op = Some(op_id);
                }
            }
        }
    }

    /// No-op by contract — the header's "WRITE COMPLETION MEANS SENT" rule.
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.ready_fault() {
            Some(err) => Poll::Ready(Err(err)),
            None => Poll::Ready(Ok(())),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let this = &mut *self;
        if this.check_generation() {
            this.close_host();
        }
        Poll::Ready(Ok(()))
    }
}

impl Drop for HostStream {
    fn drop(&mut self) {
        if self.check_generation() {
            self.close_host();
        }
    }
}

// ── The dialer the core consumes ────────────────────────────────────────────

/// The [`HostDialer`] the bridge attaches under `TorRuntime::HostDialer`
/// (a ZST — the state lives in the process-global registry, the
/// `CAbiSeedPort` shape).
pub(crate) struct CAbiHostDialer;

#[async_trait]
impl NetDialer for CAbiHostDialer {
    async fn dial(
        &self,
        host: &str,
        port: u16,
        isolation_key: Option<&str>,
    ) -> Result<Box<dyn AsyncByteStream>, DialError> {
        sweep_orphans();
        // Snapshot the registration AND its generation under one read lock,
        // so a replace cannot interleave between the two; the host call runs
        // outside it.
        let (reg, generation) = {
            let slot = registry().read().map_err(|_| {
                DialError::Io(std::io::Error::other("host dialer registry poisoned"))
            })?;
            match slot.registration {
                Some(reg) => (reg, current_generation()),
                // The door refuses a `HostDialer` policy with nothing
                // registered (spec E1), so at dial time this is the
                // cleared-registry case: typed, never a clearnet fallback.
                None => return Err(DialError::Retired),
            }
        };
        if host.len() > HOST_NAME_MAX_BYTES {
            return Err(DialError::Unsupported);
        }
        let key = isolation_key.unwrap_or("");
        if key.len() > ISOLATION_KEY_MAX_BYTES {
            return Err(DialError::Unsupported);
        }
        let op_id = mint_op_id();
        insert_op(op_id, generation, Vec::new(), None, OpKind::Dial(reg));
        let (key_ptr, key_len) = if key.is_empty() {
            (std::ptr::null(), 0)
        } else {
            (key.as_ptr(), key.len())
        };
        // SAFETY: `host` and the key are live `&str`s for the duration of this
        // call, which is exactly the lifetime the header grants the host for
        // them (it copies what it keeps). `reg` is a live registration under
        // `generation`; `ctx` is the host's own pointer; `dial_complete` is the
        // SDK's callback, valid for the process life.
        let rc = unsafe {
            (reg.dial)(
                reg.ctx(),
                host.as_ptr(),
                host.len(),
                port,
                key_ptr,
                key_len,
                op_id,
                std::ptr::null_mut(),
                dial_complete,
            )
        };
        if rc != HostDialCode::Ok.as_raw() {
            // Refused synchronously: NO completion follows, so the record
            // leaves the table now. A host that completed anyway (a violation)
            // may have delivered a stream — released, never orphaned.
            if let Some((reg, generation, handle)) = close_op(op_id) {
                release_handle(reg, generation, handle);
            }
            lock_ops().remove(&op_id);
            return Err(HostDialCode::failure_from_raw(rc));
        }
        let stream = DialOp {
            op_id,
            reg,
            generation,
            done: false,
        }
        .await?;
        Ok(Box::new(stream))
    }
}

impl HostDialer for CAbiHostDialer {
    fn descriptor(&self) -> Option<HostTransportDescriptor> {
        let slot = registry().read().ok()?;
        slot.registration.as_ref()?;
        slot.descriptor
    }
}

/// The registered host dialer as a `HostDialer`, or `None` when nothing is
/// registered — the bridge's `convert.rs` maps `TorRuntimeConfig::HostDialer`
/// through this (chunk C3bc, spec §2.1); the door then refuses a `None`.
/// `convert.rs` is that caller (`TryFrom<TorRuntimeConfig> for TorRuntime`).
pub(crate) fn registered_host_dialer() -> Option<Arc<dyn HostDialer>> {
    let slot = registry().read().ok()?;
    slot.registration
        .is_some()
        .then(|| Arc::new(CAbiHostDialer) as Arc<dyn HostDialer>)
}

// Compile-time pins: the stream rides the core's runtime and the registration
// sits in a `static`, so both must be `Send + Sync`/`Unpin` — assert it here so
// a future field that breaks it fails with this message.
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    const fn assert_send_unpin<T: Send + Unpin>() {}
    assert_send_sync::<CAbiHostDialer>();
    assert_send_sync::<Registration>();
    assert_send_unpin::<HostStream>();
};

/// Test-only reset of the process-global state: the slot, the op table and
/// the orphans; the generation is bumped so anything a previous test minted
/// is stale. `cfg(test)` by construction — the shipped library has no seam.
#[cfg(test)]
pub(crate) fn reset_registry_for_test() {
    {
        let mut slot = registry().write().unwrap_or_else(|e| e.into_inner());
        slot.registration = None;
        slot.descriptor = None;
        slot.guard = None;
        bump_generation();
    }
    lock_ops().clear();
    orphans().lock().unwrap_or_else(|e| e.into_inner()).clear();
}

/// Test-only: how many op records the table holds (T17/T18/T25 assert the
/// buffer's lifetime through it).
#[cfg(test)]
pub(crate) fn op_records_for_test() -> usize {
    lock_ops().len()
}

// The in-process fake host (spec §8) — a helper module, not a cargo target
// (see its docs for why it is `tests/host_dialer_fake/mod.rs`).
#[cfg(test)]
#[path = "../tests/host_dialer_fake/mod.rs"]
mod host_dialer_fake;

#[cfg(test)]
// The `TEST_LOCK` guard is held across the awaits of every async test ON
// PURPOSE: it serialises whole test FUNCTIONS (each on its own
// `current_thread` runtime with one task) over the process-global registry,
// and no other task on that runtime ever wants it — the lint's deadlock
// premise (another task on the same runtime blocking on the mutex) does not
// apply.
#[allow(clippy::await_holding_lock)]
mod tests {
    use super::host_dialer_fake::{
        Descriptor, DialMode, DialRecord, FakeHost, IoMode, LoopbackServer, Peer, ZW_HEALTH_FAILED,
        ZW_HEALTH_READY, ZW_HEALTH_STARTING,
    };
    use super::*;
    use std::io::ErrorKind;
    use std::ptr;
    use std::sync::atomic::AtomicUsize;
    use std::task::Wake;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use zec_wallet_core::{
        HostTransportName, IsolationSupport, TransportExposure, TransportHealth,
    };

    /// The registry, the op table and the generation are process-global, so
    /// the tests that touch them serialise here (cargo runs test fns in
    /// parallel) and reset through the `cfg(test)` seam.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn serial() -> MutexGuard<'static, ()> {
        let g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_registry_for_test();
        g
    }

    /// A ready Tor descriptor in the FAKE's own declaration (the host's name
    /// "Tor", readiness 100, isolation supported, exposure hidden) — it
    /// crosses as a raw pointer cast to the SDK's type, exactly as a C host's
    /// struct would.
    const TOR_READY: Descriptor = Descriptor::new(b"Tor", 100, 1, 1);

    fn dptr(d: &Descriptor) -> *const ZwTransportDescriptor {
        ptr::from_ref(d).cast::<ZwTransportDescriptor>()
    }

    /// Register `fake` as a C host would (its own struct declarations, cast
    /// to the SDK's pointer types) and hand back the minted token.
    fn register(fake: &FakeHost, desc: Descriptor) -> [u8; NET_DIALER_AUTH_TOKEN_BYTES] {
        let vt = fake.vtable();
        let mut token = [0u8; NET_DIALER_AUTH_TOKEN_BYTES];
        let rc = unsafe {
            zec_wallet_register_net_dialer(
                HOST_DIALER_ABI_VERSION,
                ptr::addr_of!(vt).cast::<ZwNetDialerV1>(),
                dptr(&desc),
                token.as_mut_ptr(),
            )
        };
        assert_eq!(rc, ZW_RC_OK, "registration into an empty slot succeeds");
        token
    }

    fn replace(
        fake: &FakeHost,
        token: &[u8; NET_DIALER_AUTH_TOKEN_BYTES],
        desc: Descriptor,
    ) -> i32 {
        let vt = fake.vtable();
        unsafe {
            zec_wallet_update_net_dialer(
                token.as_ptr(),
                ptr::addr_of!(vt).cast::<ZwNetDialerV1>(),
                dptr(&desc),
            )
        }
    }

    fn notify(token: &[u8; NET_DIALER_AUTH_TOKEN_BYTES], desc: Descriptor, retire: u32) -> i32 {
        unsafe { zec_wallet_net_dialer_notify(token.as_ptr(), dptr(&desc), retire) }
    }

    async fn dial(
        server: &LoopbackServer,
        key: Option<&str>,
    ) -> Result<Box<dyn AsyncByteStream>, DialError> {
        tokio::time::timeout(
            Duration::from_secs(5),
            CAbiHostDialer.dial(&server.host(), server.port(), key),
        )
        .await
        .expect("a fake dial resolves within 5 s")
    }

    /// A read that must RESOLVE (ok or err) at once — a stale stream that
    /// hangs instead of failing typed is the defect T5/T6 exist to catch.
    async fn read_now(
        stream: &mut (impl AsyncReadExt + Unpin),
        buf: &mut [u8],
    ) -> std::io::Result<usize> {
        tokio::time::timeout(Duration::from_secs(5), stream.read(buf))
            .await
            .expect("a poll on a retired stream resolves at once, it never hangs")
    }

    async fn write_now(
        stream: &mut (impl AsyncWriteExt + Unpin),
        bytes: &[u8],
    ) -> std::io::Result<usize> {
        tokio::time::timeout(Duration::from_secs(5), stream.write(bytes))
            .await
            .expect("a poll on a retired stream resolves at once, it never hangs")
    }

    async fn dial_stream(server: &LoopbackServer) -> HostStream {
        let (reg, generation) = {
            let slot = registry().read().expect("registry");
            (slot.registration.expect("registered"), current_generation())
        };
        let op_id = mint_op_id();
        insert_op(op_id, generation, Vec::new(), None, OpKind::Dial(reg));
        let host = server.host();
        let rc = unsafe {
            (reg.dial)(
                reg.ctx(),
                host.as_ptr(),
                host.len(),
                server.port(),
                ptr::null(),
                0,
                op_id,
                ptr::null_mut(),
                dial_complete,
            )
        };
        assert_eq!(rc, 0);
        tokio::time::timeout(
            Duration::from_secs(5),
            DialOp {
                op_id,
                reg,
                generation,
                done: false,
            },
        )
        .await
        .expect("a fake dial resolves within 5 s")
        .expect("the fake connects")
    }

    /// A waker that counts its wakes (T12b, T25).
    struct CountingWaker(AtomicUsize);

    impl Wake for CountingWaker {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn counting_waker() -> (Arc<CountingWaker>, Waker) {
        let counter = Arc::new(CountingWaker(AtomicUsize::new(0)));
        let waker = Waker::from(Arc::clone(&counter));
        (counter, waker)
    }

    /// The error of a dial that must fail (`Box<dyn AsyncByteStream>` has no
    /// `Debug`, so `expect_err` cannot be used on it).
    fn dial_err(outcome: Result<Box<dyn AsyncByteStream>, DialError>) -> DialError {
        match outcome {
            Ok(_) => panic!("the dial was expected to fail"),
            Err(e) => e,
        }
    }

    fn dial_error_of(err: &std::io::Error) -> Option<&DialError> {
        err.get_ref().and_then(|e| e.downcast_ref::<DialError>())
    }

    fn spin_until(what: &str, mut pred: impl FnMut() -> bool) {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !pred() {
            assert!(
                std::time::Instant::now() < deadline,
                "never reached: {what}"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    // ── T4 ───────────────────────────────────────────────────────────────

    /// Spec §3.1 / T4: the slot is first-wins. A second registration — from
    /// another in-process registrant — returns `ZW_RC_OCCUPIED`, leaves its
    /// `auth_out` byte-identical (the seed port's H-1 fold), and the FIRST
    /// registration keeps serving: a dial reaches the first fake, not the
    /// second. The other refusal codes are pinned beside it: the ABI check
    /// runs before any pointer is read (`-5`), a null argument or a null
    /// vtable entry is `-3`, a descriptor out of range is `-6`.
    #[tokio::test]
    async fn a_second_registration_is_refused() {
        let _g = serial();
        let first = FakeHost::new();
        let second = FakeHost::new();
        let live_token = register(&first, TOR_READY);

        let vt2 = second.vtable();
        let mut buf = [0xAAu8; NET_DIALER_AUTH_TOKEN_BYTES];
        let rc = unsafe {
            zec_wallet_register_net_dialer(
                HOST_DIALER_ABI_VERSION,
                ptr::addr_of!(vt2).cast::<ZwNetDialerV1>(),
                dptr(&TOR_READY),
                buf.as_mut_ptr(),
            )
        };
        assert_eq!(
            rc, ZW_RC_OCCUPIED,
            "the occupied slot refuses a second registrant"
        );
        assert_eq!(
            buf, [0xAAu8; NET_DIALER_AUTH_TOKEN_BYTES],
            "the -2 path leaves auth_out untouched"
        );

        // The FIRST registration serves: the dial reaches `first`, never `second`.
        let server = LoopbackServer::spawn(Peer::Echo);
        let stream = dial(&server, Some("wallet-sync"))
            .await
            .expect("first dials");
        drop(stream);
        assert_eq!(
            first.dials().len(),
            1,
            "the first registrant carried the dial"
        );
        assert!(
            second.dials().is_empty(),
            "the refused registrant saw nothing"
        );
        // …and the live token still authorizes the first registrant's host.
        assert_eq!(notify(&live_token, TOR_READY, 0), ZW_RC_OK);

        // The sibling refusals, on a fresh slot.
        reset_registry_for_test();
        let vt = first.vtable();
        let mut token = [0u8; NET_DIALER_AUTH_TOKEN_BYTES];
        let rc = unsafe {
            zec_wallet_register_net_dialer(
                HOST_DIALER_ABI_VERSION + 1,
                ptr::addr_of!(vt).cast::<ZwNetDialerV1>(),
                dptr(&TOR_READY),
                token.as_mut_ptr(),
            )
        };
        assert_eq!(
            rc, ZW_RC_ABI,
            "an ABI mismatch is refused before any pointer is read"
        );
        // …and with EVERY pointer null (FR-5 C2; the security angle on C1,
        // INFO). Valid pointers cannot tell "refused before any read" from
        // "refused after one"; null ones can. A null check ahead of the version
        // check answers `-3`, a read ahead of it faults — only the version
        // check FIRST answers `-5` here.
        let rc = unsafe {
            zec_wallet_register_net_dialer(
                HOST_DIALER_ABI_VERSION + 1,
                ptr::null(),
                ptr::null(),
                ptr::null_mut(),
            )
        };
        assert_eq!(
            rc, ZW_RC_ABI,
            "with every pointer null, the version check still answers first"
        );
        let rc = unsafe {
            zec_wallet_register_net_dialer(
                HOST_DIALER_ABI_VERSION,
                ptr::addr_of!(vt).cast::<ZwNetDialerV1>(),
                dptr(&TOR_READY),
                ptr::null_mut(),
            )
        };
        assert_eq!(
            rc, ZW_RC_NULL_ARG,
            "auth_out is required: there is no permanent variant"
        );
        let mut no_close = first.vtable();
        no_close.close = None;
        let rc = unsafe {
            zec_wallet_register_net_dialer(
                HOST_DIALER_ABI_VERSION,
                ptr::addr_of!(no_close).cast::<ZwNetDialerV1>(),
                dptr(&TOR_READY),
                token.as_mut_ptr(),
            )
        };
        assert_eq!(
            rc, ZW_RC_NULL_ARG,
            "a null required vtable entry is refused"
        );
        let bad = Descriptor::new(b"Tor", 101, 1, 1);
        let rc = unsafe {
            zec_wallet_register_net_dialer(
                HOST_DIALER_ABI_VERSION,
                ptr::addr_of!(vt).cast::<ZwNetDialerV1>(),
                dptr(&bad),
                token.as_mut_ptr(),
            )
        };
        assert_eq!(rc, ZW_RC_DESCRIPTOR, "readiness above 100 is out of range");
        assert!(
            registered_host_dialer().is_none(),
            "none of the refused calls registered anything"
        );
    }

    // ── T5 (this chunk's half) ───────────────────────────────────────────

    /// Spec §3.1 / T5: a token-gated REPLACE and a `notify(retire = 1)` each
    /// bump the generation and fail an OPEN stream's next poll with a typed
    /// `Retired` — without a host call on the stale stream (its handle is
    /// the retired backing's; `close` is not sent to it either). The
    /// `TorState` half is C3bc's. A wrong token mutates nothing (`-2`).
    #[tokio::test]
    async fn host_retire_fails_inflight_streams_typed() {
        let _g = serial();
        let fake = FakeHost::new();
        let token = register(&fake, TOR_READY);
        let server = LoopbackServer::spawn(Peer::Echo);

        // A live stream round-trips first, so the failure below is the retire's.
        let mut stream = dial(&server, None).await.expect("dial");
        stream.write_all(b"live").await.expect("write");
        let mut echoed = [0u8; 4];
        read_exact_now(&mut stream, &mut echoed).await;
        assert_eq!(&echoed, b"live");

        // REPLACE bumps the generation.
        let before = current_generation();
        let mut wrong = token;
        wrong[3] ^= 0x80;
        assert_eq!(
            replace(&fake, &wrong, TOR_READY),
            ZW_RC_OCCUPIED,
            "a wrong token cannot replace"
        );
        assert_eq!(
            current_generation(),
            before,
            "a refused replace bumps nothing"
        );
        assert_eq!(replace(&fake, &token, TOR_READY), ZW_RC_OK);
        assert_eq!(
            current_generation(),
            before + 1,
            "a replace bumps the generation"
        );
        let reads_before = fake.reads();
        let closes_before = fake.closes();
        let err = read_now(&mut stream, &mut echoed)
            .await
            .expect_err("the stale stream fails");
        assert!(
            matches!(dial_error_of(&err), Some(DialError::Retired)),
            "the next poll after a replace is a typed Retired, got {err:?}"
        );
        assert_eq!(
            fake.reads(),
            reads_before,
            "no host call on the stale stream"
        );
        drop(stream);
        assert_eq!(
            fake.closes(),
            closes_before,
            "no close is sent to a retired backing"
        );

        // RETIRE (the push) bumps the generation without changing the registration.
        let mut stream = dial(&server, None)
            .await
            .expect("dial under the new generation");
        stream.write_all(b"live").await.expect("write");
        read_exact_now(&mut stream, &mut echoed).await;
        let before = current_generation();
        assert_eq!(notify(&token, TOR_READY, 1), ZW_RC_OK);
        assert_eq!(
            current_generation(),
            before + 1,
            "a retire bumps the generation"
        );
        let err = write_now(&mut stream, b"x")
            .await
            .expect_err("the retired stream fails");
        assert!(matches!(dial_error_of(&err), Some(DialError::Retired)));
        assert!(
            registered_host_dialer().is_some(),
            "a retire keeps the registration (the host re-arms behind it)"
        );
        // A plain readiness push (retire = 0) bumps nothing.
        let before = current_generation();
        assert_eq!(notify(&token, TOR_READY, 0), ZW_RC_OK);
        assert_eq!(
            current_generation(),
            before,
            "a readiness push is not a retire"
        );
        drop(stream);
    }

    // ── T6 ───────────────────────────────────────────────────────────────

    /// Spec §3.2 "Generation" / T6: a stream minted under generation N that
    /// polls after a replace returns `Retired` WITHOUT calling the host —
    /// the read count at the fake does not move — and a CLEAR makes the next
    /// dial fail `Retired` too (never a clearnet path; the cleared token
    /// authorizes nothing).
    #[tokio::test]
    async fn a_stale_generation_handle_is_refused() {
        let _g = serial();
        let fake = FakeHost::new();
        let token = register(&fake, TOR_READY);
        let server = LoopbackServer::spawn(Peer::Echo);
        let mut stream = dial(&server, None).await.expect("dial");
        let reads_before = fake.reads();
        assert_eq!(replace(&fake, &token, TOR_READY), ZW_RC_OK);
        let mut buf = [0u8; 8];
        let err = read_now(&mut stream, &mut buf).await.expect_err("stale");
        assert!(matches!(dial_error_of(&err), Some(DialError::Retired)));
        assert_eq!(
            fake.reads(),
            reads_before,
            "a stale-generation stream never reaches the host"
        );
        // Every later poll stays typed the same way.
        let err = write_now(&mut stream, b"z").await.expect_err("still stale");
        assert!(matches!(dial_error_of(&err), Some(DialError::Retired)));
        drop(stream);

        // CLEAR: the slot empties, the token dies, a dial fails typed.
        assert_eq!(
            unsafe { zec_wallet_update_net_dialer(token.as_ptr(), ptr::null(), ptr::null()) },
            ZW_RC_OK
        );
        assert!(registered_host_dialer().is_none(), "the slot is empty");
        assert!(CAbiHostDialer.descriptor().is_none());
        let err = dial_err(dial(&server, None).await);
        assert!(matches!(err, DialError::Retired), "got {err:?}");
        assert_eq!(
            notify(&token, TOR_READY, 0),
            ZW_RC_OCCUPIED,
            "a cleared token is dead"
        );
    }

    // ── T8 ───────────────────────────────────────────────────────────────

    /// Spec §5 / T8: the isolation key crosses VERBATIM — the fake records
    /// the exact bytes for `wallet-sync` and a `wallet-send-<rand>` key, a
    /// `None` key crosses as null/0, and the host name and port arrive
    /// unchanged. The two bounds refuse typed (`Unsupported`) before any
    /// host call.
    #[tokio::test]
    async fn the_isolation_key_survives_the_crossing() {
        let _g = serial();
        let fake = FakeHost::new();
        register(&fake, TOR_READY);
        let server = LoopbackServer::spawn(Peer::Echo);
        let send_key = "wallet-send-3f9a1c0e";
        for key in [Some("wallet-sync"), Some(send_key), None] {
            let stream = dial(&server, key).await.expect("dial");
            drop(stream);
        }
        let dials = fake.dials();
        assert_eq!(
            dials,
            vec![
                DialRecord {
                    host: server.host(),
                    port: server.port(),
                    isolation_key: Some(b"wallet-sync".to_vec()),
                },
                DialRecord {
                    host: server.host(),
                    port: server.port(),
                    isolation_key: Some(send_key.as_bytes().to_vec()),
                },
                DialRecord {
                    host: server.host(),
                    port: server.port(),
                    isolation_key: None,
                },
            ],
            "the key bytes, the host and the port cross the C-ABI verbatim"
        );
        let long_key = "k".repeat(ISOLATION_KEY_MAX_BYTES + 1);
        let err = dial_err(
            CAbiHostDialer
                .dial(&server.host(), server.port(), Some(&long_key))
                .await,
        );
        assert!(matches!(err, DialError::Unsupported));
        let long_host = "h".repeat(HOST_NAME_MAX_BYTES + 1);
        let err = dial_err(CAbiHostDialer.dial(&long_host, server.port(), None).await);
        assert!(matches!(err, DialError::Unsupported));
        assert_eq!(
            fake.dials().len(),
            3,
            "an out-of-bound dial never reaches the host"
        );
    }

    // ── T12 ──────────────────────────────────────────────────────────────

    /// Spec D7 / T12: a panic inside any of the three exports is caught at
    /// the boundary and reported as `ZW_RC_PANICKED`; nothing changes (an
    /// empty slot stays empty, an occupied one keeps its host, `auth_out` is
    /// untouched) and the lock is not poisoned — the next verb works.
    #[test]
    fn a_planted_panic_at_every_export_returns_a_code() {
        let _g = serial();
        let fake = FakeHost::new();
        let vt = fake.vtable();
        let _disarm = Plant::arm(&PLANT_VERB_PANIC);
        let mut auth_out = [0x55u8; NET_DIALER_AUTH_TOKEN_BYTES];
        let rc_register = unsafe {
            zec_wallet_register_net_dialer(
                HOST_DIALER_ABI_VERSION,
                ptr::addr_of!(vt).cast::<ZwNetDialerV1>(),
                dptr(&TOR_READY),
                auth_out.as_mut_ptr(),
            )
        };
        let guess = [9u8; NET_DIALER_AUTH_TOKEN_BYTES];
        let rc_update = replace(&fake, &guess, TOR_READY);
        let rc_notify = notify(&guess, TOR_READY, 1);
        PLANT_VERB_PANIC.store(false, Ordering::SeqCst);
        assert_eq!(
            [rc_register, rc_update, rc_notify],
            [ZW_RC_PANICKED; 3],
            "every export reports a caught panic as -4"
        );
        assert_eq!(
            auth_out, [0x55u8; NET_DIALER_AUTH_TOKEN_BYTES],
            "auth_out untouched"
        );
        assert!(
            registered_host_dialer().is_none(),
            "a panicked register registers nothing"
        );

        // Not poisoned: a real registration works, and an occupied slot
        // survives a panicked clear and a panicked retire.
        let token = register(&fake, TOR_READY);
        let generation = current_generation();
        let _disarm = Plant::arm(&PLANT_VERB_PANIC);
        let rc_clear =
            unsafe { zec_wallet_update_net_dialer(token.as_ptr(), ptr::null(), ptr::null()) };
        let rc_retire = notify(&token, TOR_READY, 1);
        PLANT_VERB_PANIC.store(false, Ordering::SeqCst);
        assert_eq!([rc_clear, rc_retire], [ZW_RC_PANICKED; 2]);
        assert!(
            registered_host_dialer().is_some(),
            "a panicked clear clears nothing"
        );
        assert_eq!(
            current_generation(),
            generation,
            "a panicked retire bumps nothing"
        );
    }

    /// Spec D7 / T12: a panic inside a completion callback never unwinds into
    /// the host thread; the boundary records `Io` on the op and WAKES it, so
    /// the waiting poll returns the fault instead of hanging.
    #[tokio::test]
    async fn a_planted_panic_in_a_completion_records_io_and_wakes() {
        let _g = serial();
        let fake = FakeHost::new();
        register(&fake, TOR_READY);
        let server = LoopbackServer::spawn(Peer::GreetThenEcho(b"hello"));
        let mut stream = dial_stream(&server).await;
        let (counter, waker) = counting_waker();
        let mut cx = Context::from_waker(&waker);
        let mut raw = [0u8; 16];
        let mut dst = ReadBuf::new(&mut raw);
        let _disarm = Plant::arm(&PLANT_COMPLETION_PANIC);
        // The fake PARKS the completion until the first poll has returned:
        // the greeting is already on the wire, so an `Async` read completes on
        // the fake's thread at once and could land before `poll_read` returns
        // (Ready, not Pending — watched on a Linux runner).
        fake.set_io_mode(IoMode::Hold);
        assert!(
            Pin::new(&mut stream)
                .poll_read(&mut cx, &mut dst)
                .is_pending()
        );
        fake.wait_until(
            Duration::from_secs(5),
            "the fake parked the completion",
            |f| f.held() == 1,
        );
        std::thread::scope(|s| {
            s.spawn(|| fake.release_held());
        });
        spin_until("the completion woke the task", || {
            counter.0.load(Ordering::SeqCst) >= 1
        });
        PLANT_COMPLETION_PANIC.store(false, Ordering::SeqCst);
        let polled = Pin::new(&mut stream).poll_read(&mut cx, &mut dst);
        let Poll::Ready(Err(err)) = polled else {
            panic!("a panicked completion must surface as a ready error, got pending/ok");
        };
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert!(
            err.to_string()
                .contains("panicked inside a host dialer completion"),
            "the fault names the boundary: {err}"
        );
        assert_eq!(counter.0.load(Ordering::SeqCst), 1, "exactly one wake");
        assert_eq!(
            dst.filled().len(),
            0,
            "no byte is trusted from a panicked completion"
        );
        assert_eq!(op_records_for_test(), 0, "the record was consumed");
    }

    // ── T15 ──────────────────────────────────────────────────────────────

    fn header_defines() -> HashMap<String, i64> {
        let src = include_str!("../include/zec_wallet_net_dialer.h");
        let mut out = HashMap::new();
        for line in src.lines() {
            let Some(rest) = line.trim().strip_prefix("#define ") else {
                continue;
            };
            let mut parts = rest.split_whitespace();
            let (Some(name), Some(raw)) = (parts.next(), parts.next()) else {
                continue;
            };
            let raw = raw.trim_end_matches('u');
            let value = match raw.strip_prefix("0x") {
                Some(hex) => i64::from_str_radix(hex, 16),
                None => raw.parse::<i64>(),
            };
            if let Ok(v) = value {
                out.insert(name.to_string(), v);
            }
        }
        out
    }

    /// Spec §3.3 / §3.5 / T15: the header and the Rust table are ONE table.
    /// Every constant the header defines with a value — the ABI version (3),
    /// the token width, the three bounds (host name, isolation key, transport
    /// name), the seven verb codes, the six dial codes, the three isolation
    /// values, the three exposure values and the three health values
    /// (ADR-0549) — equals its Rust counterpart, each closed set has exactly
    /// that many members on the header side (nothing the Rust side does not
    /// know), the header defines NO transport kind (ADR-0547), and the
    /// descriptor struct's layout matches field for field.
    #[test]
    fn the_header_and_the_rust_table_agree() {
        let h = header_defines();
        let get = |name: &str| -> i64 {
            *h.get(name)
                .unwrap_or_else(|| panic!("the header defines {name} with a value"))
        };
        assert_eq!(
            get("ZW_NET_DIALER_ABI_VERSION"),
            i64::from(HOST_DIALER_ABI_VERSION)
        );
        assert_eq!(
            get("ZW_NET_DIALER_AUTH_TOKEN_BYTES"),
            NET_DIALER_AUTH_TOKEN_BYTES as i64
        );
        assert_eq!(
            get("ZW_ISOLATION_KEY_MAX_BYTES"),
            ISOLATION_KEY_MAX_BYTES as i64
        );
        assert_eq!(get("ZW_HOST_NAME_MAX_BYTES"), HOST_NAME_MAX_BYTES as i64);

        let rc = [
            ("ZW_RC_OK", ZW_RC_OK),
            ("ZW_RC_POISONED", ZW_RC_POISONED),
            ("ZW_RC_OCCUPIED", ZW_RC_OCCUPIED),
            ("ZW_RC_NULL_ARG", ZW_RC_NULL_ARG),
            ("ZW_RC_PANICKED", ZW_RC_PANICKED),
            ("ZW_RC_ABI", ZW_RC_ABI),
            ("ZW_RC_DESCRIPTOR", ZW_RC_DESCRIPTOR),
        ];
        for (name, value) in rc {
            assert_eq!(get(name), i64::from(value), "{name}");
        }
        assert_eq!(
            h.keys().filter(|k| k.starts_with("ZW_RC_")).count(),
            rc.len()
        );

        let dial = [
            ("ZW_DIAL_OK", HostDialCode::Ok),
            ("ZW_DIAL_NOT_READY", HostDialCode::NotReady),
            ("ZW_DIAL_UNREACHABLE", HostDialCode::Unreachable),
            ("ZW_DIAL_TIMEOUT", HostDialCode::Timeout),
            ("ZW_DIAL_REFUSED", HostDialCode::Refused),
            ("ZW_DIAL_RETIRED", HostDialCode::Retired),
        ];
        for (name, code) in dial {
            assert_eq!(get(name), i64::from(code.as_raw()), "{name}");
            assert_eq!(HostDialCode::from_raw(code.as_raw()), Some(code));
        }
        assert_eq!(
            h.keys().filter(|k| k.starts_with("ZW_DIAL_")).count(),
            dial.len()
        );

        // ADR-0547: the SDK carries NO predefined transport kinds — the header
        // defines none, and names the bound on the host's own name instead.
        assert_eq!(
            h.keys()
                .filter(|k| k.starts_with("ZW_TRANSPORT_KIND_"))
                .count(),
            0,
            "the v1 kind set is retired (ADR-0547)"
        );
        assert_eq!(
            get("ZW_TRANSPORT_NAME_MAX_BYTES"),
            HOST_TRANSPORT_NAME_MAX_BYTES as i64
        );

        let exposure = [
            ("ZW_EXPOSURE_UNKNOWN", TransportExposure::Unknown),
            ("ZW_EXPOSURE_HIDDEN", TransportExposure::Hidden),
            ("ZW_EXPOSURE_EXPOSED", TransportExposure::Exposed),
        ];
        for (name, value) in exposure {
            assert_eq!(get(name), i64::from(value.as_raw()), "{name}");
        }
        assert_eq!(
            h.keys().filter(|k| k.starts_with("ZW_EXPOSURE_")).count(),
            exposure.len()
        );

        let health = [
            ("ZW_HEALTH_STARTING", TransportHealth::Starting),
            ("ZW_HEALTH_READY", TransportHealth::Ready),
            ("ZW_HEALTH_FAILED", TransportHealth::Failed),
        ];
        for (name, value) in health {
            assert_eq!(get(name), i64::from(value.as_raw()), "{name}");
        }
        assert_eq!(
            h.keys().filter(|k| k.starts_with("ZW_HEALTH_")).count(),
            health.len()
        );

        // The struct's layout IS the ABI: 32 name bytes then five u32s, 52
        // bytes at 4-byte alignment, exactly as the header lays it out. The
        // v3 field is APPENDED — every v2 offset is unchanged, which is what
        // makes the version bump the only thing a v2 host trips over.
        assert_eq!(std::mem::size_of::<ZwTransportDescriptor>(), 52);
        assert_eq!(std::mem::align_of::<ZwTransportDescriptor>(), 4);
        assert_eq!(std::mem::offset_of!(ZwTransportDescriptor, name), 0);
        assert_eq!(std::mem::offset_of!(ZwTransportDescriptor, name_len), 32);
        assert_eq!(std::mem::offset_of!(ZwTransportDescriptor, readiness), 36);
        assert_eq!(std::mem::offset_of!(ZwTransportDescriptor, isolation), 40);
        assert_eq!(std::mem::offset_of!(ZwTransportDescriptor, exposure), 44);
        assert_eq!(std::mem::offset_of!(ZwTransportDescriptor, health), 48);

        // …and the FAKE host's independent copy has that layout too (FR-5 C2;
        // the arch angle on C1, MINOR). The fake declares its own `Descriptor`
        // on purpose, so a disagreement fails a test instead of being hidden by
        // one shared type — but nothing bound the fake's LAYOUT. At a future v4
        // an un-updated fake hands the SDK a pointer to a smaller object than
        // the `ptr::read` of the larger struct reads: UB that usually passes.
        {
            use std::mem::{align_of, offset_of, size_of};
            type Sdk = ZwTransportDescriptor;
            assert_eq!(size_of::<Descriptor>(), size_of::<Sdk>(), "the fake's size");
            assert_eq!(
                align_of::<Descriptor>(),
                align_of::<Sdk>(),
                "the fake's alignment"
            );
            assert_eq!(offset_of!(Descriptor, name), offset_of!(Sdk, name));
            assert_eq!(offset_of!(Descriptor, name_len), offset_of!(Sdk, name_len));
            assert_eq!(
                offset_of!(Descriptor, readiness),
                offset_of!(Sdk, readiness)
            );
            assert_eq!(
                offset_of!(Descriptor, isolation),
                offset_of!(Sdk, isolation)
            );
            assert_eq!(offset_of!(Descriptor, exposure), offset_of!(Sdk, exposure));
            assert_eq!(offset_of!(Descriptor, health), offset_of!(Sdk, health));
        }

        let iso = [
            ("ZW_ISOLATION_UNKNOWN", IsolationSupport::Unknown),
            ("ZW_ISOLATION_SUPPORTED", IsolationSupport::Supported),
            ("ZW_ISOLATION_UNSUPPORTED", IsolationSupport::Unsupported),
        ];
        for (name, value) in iso {
            assert_eq!(get(name), i64::from(value.as_raw()), "{name}");
        }
        assert_eq!(
            h.keys()
                .filter(|k| k.starts_with("ZW_ISOLATION_") && !k.ends_with("_MAX_BYTES"))
                .count(),
            iso.len()
        );
    }

    // ── T17 ──────────────────────────────────────────────────────────────

    /// Spec §4 / E7 / T17: every integer from the host is validated. A read
    /// completion with `n > cap` fails the op `Io`, closes the stream (one
    /// host `close`), frees the record and advances NOTHING into the caller's
    /// buffer; an unknown dial code and a zero handle on a successful dial
    /// fail the dial `Io` through the core's one mapping.
    #[tokio::test]
    async fn an_out_of_range_completion_fails_the_op_typed() {
        let _g = serial();
        let fake = FakeHost::new();
        register(&fake, TOR_READY);
        let server = LoopbackServer::spawn(Peer::GreetThenEcho(b"greeting"));

        // (a) n > cap on a read.
        let mut stream = dial(&server, None).await.expect("dial");
        fake.set_io_mode(IoMode::OverReport);
        let closes_before = fake.closes();
        let mut buf = [0u8; 32];
        let err = stream.read(&mut buf).await.expect_err("past cap");
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert!(err.to_string().contains("past cap"), "{err}");
        assert_eq!(
            buf, [0u8; 32],
            "no byte is advanced from an over-reported read"
        );
        assert_eq!(
            fake.closes(),
            closes_before + 1,
            "the stream is closed at the host"
        );
        assert_eq!(
            op_records_for_test(),
            0,
            "the record and its buffer are freed"
        );
        let err = stream.read(&mut buf).await.expect_err("terminal");
        assert_eq!(
            err.kind(),
            ErrorKind::InvalidData,
            "every later poll re-issues the fault"
        );
        drop(stream);
        fake.set_io_mode(IoMode::Async);

        // (b) an unknown dial code.
        fake.set_dial_mode(DialMode::Complete(99));
        let err = dial_err(dial(&server, None).await);
        assert!(matches!(err, DialError::Io(_)), "got {err:?}");
        // (c) a zero handle on OK.
        fake.set_dial_mode(DialMode::ZeroHandle);
        let err = dial_err(dial(&server, None).await);
        assert!(
            matches!(&err, DialError::Io(e) if e.to_string().contains("zero stream handle")),
            "got {err:?}"
        );
        // (d) a known failure code maps through the ONE table.
        fake.set_dial_mode(DialMode::Complete(HostDialCode::NotReady.as_raw()));
        let err = dial_err(dial(&server, None).await);
        assert!(matches!(err, DialError::NotReady));
        // (e) a synchronous refusal: no completion follows, no record stays.
        fake.set_dial_mode(DialMode::RefuseSync(HostDialCode::Refused.as_raw()));
        let completions = fake.completions();
        let err = dial_err(dial(&server, None).await);
        assert!(matches!(err, DialError::Unsupported));
        assert_eq!(fake.completions(), completions);
        assert_eq!(op_records_for_test(), 0);
    }

    // ── T18 ──────────────────────────────────────────────────────────────

    /// Spec §3.2 "Cancel" / T18: the SDK cancels a read (the stream is
    /// dropped → `close`) while the host still holds the buffer. The record
    /// — and the buffer it owns — STAYS in the table until the host's late
    /// completion arrives; that completion is discarded (nothing to wake)
    /// and only then is the buffer freed. No use-after-free by construction.
    #[tokio::test]
    async fn a_cancelled_op_keeps_its_buffer_until_the_host_completes() {
        let _g = serial();
        let fake = FakeHost::new();
        register(&fake, TOR_READY);
        let server = LoopbackServer::spawn(Peer::GreetThenEcho(b"late bytes"));
        let mut stream = dial(&server, None).await.expect("dial");
        fake.set_io_mode(IoMode::Hold);
        let mut buf = [0u8; 16];
        // The read parks (the fake holds its completion); the caller's future
        // is dropped by the timeout — the op is still outstanding at the host.
        let timed_out =
            tokio::time::timeout(Duration::from_millis(200), stream.read(&mut buf)).await;
        assert!(timed_out.is_err(), "the held read does not complete");
        fake.wait_until(
            Duration::from_secs(5),
            "the fake parked the completion",
            |f| f.held() == 1,
        );
        assert_eq!(op_records_for_test(), 1, "one outstanding record");
        // Cancel: drop the stream (close). The record stays — the host still
        // holds the buffer.
        drop(stream);
        assert_eq!(op_records_for_test(), 1, "the buffer outlives the cancel");
        // The host completes LATE, from a thread that is not the SDK's.
        let completions = fake.completions();
        std::thread::scope(|s| {
            s.spawn(|| fake.release_held());
        });
        assert_eq!(
            fake.completions(),
            completions + 1,
            "the late completion was delivered"
        );
        assert_eq!(
            op_records_for_test(),
            0,
            "…and only then is the record (buffer) freed"
        );
    }

    // ── FR-39 (S2 §3.5a) ─────────────────────────────────────────────────

    /// The wallet close's side of the op table, with two wallets in turn in
    /// one process: A's dial is still in flight at the host when A closes (a
    /// straggling future keeps it alive), B opens and dials, then the host
    /// completes A's dial LATE with a live stream. Nothing of A's was freed
    /// before that completion, the stream it delivered goes back to the
    /// host's `close` (the orphan-release path, run by B's close), B's own
    /// stream is untouched, and A's future gets no stream — only a typed
    /// fault. `close_outstanding_ops` is what `WalletHandle::close` runs; a
    /// bridge unit cannot open a real wallet (no platform vault), so each
    /// wallet here is the dials it made.
    #[tokio::test]
    async fn closing_wallet_a_leaves_nothing_of_it_reachable_by_wallet_b() {
        let _g = serial();
        let fake = FakeHost::new();
        register(&fake, TOR_READY);
        let server = LoopbackServer::spawn(Peer::Echo);

        // Wallet A: a dial the host HOLDS.
        fake.set_dial_mode(DialMode::Hold);
        let host = server.host();
        let mut a_dial = CAbiHostDialer.dial(&host, server.port(), None);
        let (a_wakes, waker) = counting_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(
            a_dial.as_mut().poll(&mut cx).is_pending(),
            "A's dial is in flight at the host"
        );
        assert_eq!(fake.held_dials(), 1, "the host holds A's completion");
        assert_eq!(op_records_for_test(), 1, "one outstanding record");

        // Wallet A closes before the host completes.
        close_outstanding_ops();
        let a_woken_at_close = a_wakes.0.load(Ordering::SeqCst);
        assert_eq!(
            op_records_for_test(),
            1,
            "nothing is freed before the host's completion"
        );

        // Wallet B opens and dials: its own op, its own stream.
        fake.set_dial_mode(DialMode::Connect);
        let mut b_stream = dial(&server, None).await.expect("B's dial proceeds");
        b_stream.write_all(b"wallet-b").await.expect("B writes");
        let mut buf = [0u8; 8];
        tokio::time::timeout(Duration::from_secs(5), b_stream.read_exact(&mut buf))
            .await
            .expect("B's read never hangs")
            .expect("B reads");
        assert_eq!(&buf, b"wallet-b", "B's stream is B's");
        assert_eq!(fake.open_streams(), 2, "A's held stream and B's");

        // The host completes A's dial LATE, from a thread that is not the
        // SDK's, then wallet B closes.
        let closes = fake.closes();
        std::thread::scope(|s| {
            s.spawn(|| fake.release_held());
        });
        close_outstanding_ops();
        // The release may come from the completion's own release thread or
        // from this sweep — exactly once either way; wait for it, then pin
        // that it did not happen twice.
        fake.wait_until(
            Duration::from_secs(5),
            "the stream the host delivered to A's closed dial is released to the host",
            |f| f.closes() > closes,
        );
        assert_eq!(
            fake.closes(),
            closes + 1,
            "the stream the host delivered to A's closed dial is released to the host"
        );
        assert_eq!(fake.open_streams(), 1, "B's own stream is untouched");
        assert_eq!(op_records_for_test(), 0, "the completion freed A's record");

        // A's straggler gets no stream — the host's or B's — only a fault.
        match a_dial.as_mut().poll(&mut cx) {
            Poll::Ready(Err(DialError::Io(_))) => {}
            Poll::Ready(Ok(_)) => panic!("A's closed dial must not yield a stream"),
            Poll::Ready(Err(e)) => panic!("A's closed dial fails Io, got {e}"),
            Poll::Pending => panic!("A's closed dial must not stay pending"),
        }
        assert_eq!(
            a_woken_at_close, 1,
            "the close woke A's straggler so it sees the close"
        );
        drop(b_stream);
    }

    /// The S2 diff review's race (FR-39): the host completes a dial the
    /// wallet close has already closed, AFTER the close's own sweep ran, and
    /// the process never dials or closes again (the last wallet closed). The
    /// stream it delivered still goes back to the host's `close` — the
    /// completion cannot release it on the host's own thread, so it is not
    /// allowed to wait for a sweep that may never come.
    #[tokio::test]
    async fn a_stream_delivered_after_the_wallet_closed_is_released_without_a_later_dial() {
        let _g = serial();
        let fake = FakeHost::new();
        register(&fake, TOR_READY);
        let server = LoopbackServer::spawn(Peer::Echo);

        fake.set_dial_mode(DialMode::Hold);
        let host = server.host();
        let mut a_dial = CAbiHostDialer.dial(&host, server.port(), None);
        let (_wakes, waker) = counting_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(a_dial.as_mut().poll(&mut cx).is_pending());
        assert_eq!(fake.held_dials(), 1, "the host holds the dial's completion");

        // The wallet closes; its sweep runs now, with nothing parked yet.
        close_outstanding_ops();
        let closes = fake.closes();

        // The host completes LATE, from its own thread. Nothing else runs.
        std::thread::scope(|s| {
            s.spawn(|| fake.release_held());
        });
        fake.wait_until(
            Duration::from_secs(5),
            "the late stream is released to the host with no later dial or close",
            |f| f.closes() == closes + 1,
        );
        assert_eq!(op_records_for_test(), 0, "the completion freed the record");
        drop(a_dial);
    }

    // ── T23 ──────────────────────────────────────────────────────────────

    /// Spec §3.2 "Registration order" / T23: the fake completes INSIDE
    /// `dial`, `write` and `read` — on the calling thread, before the verb
    /// returns. Because the record is in the table before the verb runs, the
    /// completion finds it, and the op resolves on the next poll: nothing
    /// hangs, no result is lost.
    #[tokio::test]
    async fn a_completion_that_fires_before_the_verb_returns_is_not_lost() {
        let _g = serial();
        let fake = FakeHost::new();
        register(&fake, TOR_READY);
        fake.set_dial_mode(DialMode::ConnectInline);
        fake.set_io_mode(IoMode::Inline);
        let server = LoopbackServer::spawn(Peer::Echo);
        let mut stream = dial(&server, Some("wallet-sync"))
            .await
            .expect("inline dial resolves");
        assert_eq!(fake.completions(), 1, "the dial completed inline");
        let n = tokio::time::timeout(Duration::from_secs(5), stream.write(b"inline"))
            .await
            .expect("an inline write never hangs")
            .expect("write");
        assert_eq!(n, 6);
        assert_eq!(fake.completions(), 2, "the write completed inline");
        let mut buf = [0u8; 6];
        tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut buf))
            .await
            .expect("an inline read never hangs")
            .expect("read");
        assert_eq!(&buf, b"inline");
        assert_eq!(
            op_records_for_test(),
            0,
            "every inline completion was consumed"
        );
    }

    // ── T24 ──────────────────────────────────────────────────────────────

    /// Spec §3.2 "The write re-present rule" / T24: after `Pending`, a caller
    /// that re-polls with a CHANGED prefix gets `Io("write re-present
    /// violation")`, the stream closes and nothing more reaches the wire; the
    /// honest re-poll with the same bytes returns `Ready(n)`.
    #[tokio::test]
    async fn a_write_re_presented_with_different_bytes_fails_the_stream() {
        let _g = serial();
        let fake = FakeHost::new();
        register(&fake, TOR_READY);
        let server = LoopbackServer::spawn(Peer::Echo);
        // The fake PARKS each write's completion until the test has seen the
        // first poll return Pending, then releases it from another thread. An
        // `Async` write completes on the fake's own thread and can land before
        // `poll_write` returns (Ready, not Pending — watched on a Linux runner).
        fake.set_io_mode(IoMode::Hold);
        let release_the_parked_write = || {
            fake.wait_until(
                Duration::from_secs(5),
                "the fake parked the write's completion",
                |f| f.held() == 1,
            );
            std::thread::scope(|s| {
                s.spawn(|| fake.release_held());
            });
        };

        // The dishonest re-present.
        let mut stream = dial_stream(&server).await;
        let (counter, waker) = counting_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(
            Pin::new(&mut stream)
                .poll_write(&mut cx, b"hello")
                .is_pending()
        );
        release_the_parked_write();
        spin_until("the write completed", || {
            counter.0.load(Ordering::SeqCst) >= 1
        });
        let writes = fake.writes();
        let polled = Pin::new(&mut stream).poll_write(&mut cx, b"jello");
        let Poll::Ready(Err(err)) = polled else {
            panic!("a changed prefix must fail, got pending/ok");
        };
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert!(
            err.to_string().contains("write re-present violation"),
            "{err}"
        );
        assert!(stream.closed, "the stream is closed");
        let polled = Pin::new(&mut stream).poll_write(&mut cx, b"more");
        assert!(
            matches!(polled, Poll::Ready(Err(_))),
            "the fault is terminal"
        );
        assert_eq!(fake.writes(), writes, "nothing more reached the wire");
        drop(stream);

        // The honest re-present.
        let mut stream = dial_stream(&server).await;
        let (counter, waker) = counting_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(
            Pin::new(&mut stream)
                .poll_write(&mut cx, b"hello")
                .is_pending()
        );
        release_the_parked_write();
        spin_until("the write completed", || {
            counter.0.load(Ordering::SeqCst) >= 1
        });
        let polled = Pin::new(&mut stream).poll_write(&mut cx, b"hello");
        assert!(
            matches!(polled, Poll::Ready(Ok(5))),
            "the same bytes are Ready(5)"
        );
        drop(stream);
    }

    // ── T25 ──────────────────────────────────────────────────────────────

    /// Which side of the T25 race goes first in a round.
    #[derive(Clone, Copy, Debug)]
    enum Order {
        /// The host's completion lands, THEN the SDK closes.
        CompletionThenClose,
        /// The SDK closes, THEN the host's late completion lands.
        CloseThenCompletion,
        /// Both at once, from two threads.
        Race,
    }

    /// Spec §3.2 (the one-way state machine) / T25: the fake completes a
    /// read from a SECOND thread while the SDK closes the stream. Whichever
    /// transition wins, the record leaves the table exactly once, the host
    /// delivered exactly one completion, the SDK issued exactly one `close`,
    /// and the task was woken at most once. Both deterministic orders are
    /// pinned first (so a defect in either arm reds every run), then the
    /// race is run twenty times.
    #[tokio::test]
    async fn close_during_an_executing_completion_is_exactly_once() {
        let _g = serial();
        let fake = FakeHost::new();
        register(&fake, TOR_READY);
        let server = LoopbackServer::spawn(Peer::GreetThenEcho(b"racing"));
        fake.set_io_mode(IoMode::Hold);
        let rounds = [Order::CompletionThenClose, Order::CloseThenCompletion]
            .into_iter()
            .chain(std::iter::repeat_n(Order::Race, 20));
        for (round, order) in rounds.enumerate() {
            let mut stream = dial_stream(&server).await;
            let (counter, waker) = counting_waker();
            let mut cx = Context::from_waker(&waker);
            let mut raw = [0u8; 16];
            let mut dst = ReadBuf::new(&mut raw);
            assert!(
                Pin::new(&mut stream)
                    .poll_read(&mut cx, &mut dst)
                    .is_pending()
            );
            fake.wait_until(
                Duration::from_secs(5),
                "the fake parked the completion",
                |f| f.held() == 1,
            );
            let completions = fake.completions();
            let closes = fake.closes();
            match order {
                Order::CompletionThenClose => {
                    std::thread::scope(|s| {
                        s.spawn(|| fake.release_held());
                    });
                    drop(stream);
                }
                Order::CloseThenCompletion => {
                    drop(stream);
                    std::thread::scope(|s| {
                        s.spawn(|| fake.release_held());
                    });
                }
                Order::Race => {
                    let go = std::sync::atomic::AtomicBool::new(false);
                    std::thread::scope(|s| {
                        s.spawn(|| {
                            while !go.load(Ordering::SeqCst) {
                                std::hint::spin_loop();
                            }
                            fake.release_held();
                        });
                        go.store(true, Ordering::SeqCst);
                        drop(stream); // the SDK's close, racing the release
                    });
                }
            }
            let round = format!("{round} ({order:?})");
            assert_eq!(
                fake.completions(),
                completions + 1,
                "round {round}: one host completion"
            );
            assert_eq!(fake.closes(), closes + 1, "round {round}: one SDK close");
            assert_eq!(
                op_records_for_test(),
                0,
                "round {round}: the record left the table once"
            );
            assert!(
                counter.0.load(Ordering::SeqCst) <= 1,
                "round {round}: at most one wake"
            );
            assert_eq!(fake.held(), 0);
        }
    }

    // ── The round trip ───────────────────────────────────────────────────

    /// The crossing end to end through the fake: dial to a loopback peer,
    /// write, read the echo back, and read EOF once the peer closes; the
    /// descriptor the core reads is the registered one; the SDK's `close`
    /// reaches the host once.
    #[tokio::test]
    async fn bytes_round_trip_through_the_fake_host_and_eof_arrives_on_close() {
        let _g = serial();
        let fake = FakeHost::new();
        register(&fake, TOR_READY);
        assert_eq!(
            CAbiHostDialer.descriptor(),
            Some(HostTransportDescriptor {
                name: HostTransportName::new(b"Tor").expect("valid"),
                readiness: 100,
                isolation: IsolationSupport::Supported,
                exposure: TransportExposure::Hidden,
                health: TransportHealth::Ready,
            })
        );
        let server = LoopbackServer::spawn(Peer::EchoOnceThenClose);
        let mut stream = dial(&server, Some("wallet-sync")).await.expect("dial");
        let payload: Vec<u8> = (0..40_000u32).map(|i| (i % 251) as u8).collect();
        // Larger than one chunk: the write is split into record-sized pieces.
        stream.write_all(&payload[..1000]).await.expect("write");
        let mut echoed = vec![0u8; 1000];
        read_exact_now(&mut stream, &mut echoed).await;
        assert_eq!(
            echoed,
            payload[..1000],
            "the bytes round-trip byte-identical"
        );
        let mut tail = [0u8; 8];
        let n = stream.read(&mut tail).await.expect("eof");
        assert_eq!(n, 0, "the peer's close arrives as EOF (n == 0)");
        stream.shutdown().await.expect("shutdown");
        assert_eq!(fake.closes(), 1, "shutdown closed the stream at the host");
        drop(stream);
        assert_eq!(fake.closes(), 1, "drop after shutdown does not close twice");
        assert_eq!(op_records_for_test(), 0);
        assert_eq!(fake.open_streams(), 0);
    }

    /// An armed plant that DISARMS on drop — on a passing test and on a
    /// failing one alike. A plant left armed by a failed assertion would
    /// panic every later completion in the process (watched during the T12
    /// mutant: one red cascaded into eleven hangs), which is a defect of the
    /// test, not of the boundary.
    struct Plant(&'static std::sync::atomic::AtomicBool);

    impl Plant {
        fn arm(flag: &'static std::sync::atomic::AtomicBool) -> Self {
            flag.store(true, Ordering::SeqCst);
            Self(flag)
        }
    }

    impl Drop for Plant {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }

    /// `read_exact` that must finish within 5 s: a stream that delivers
    /// fewer bytes than the host wrote would otherwise wait forever here
    /// (watched during the round-trip mutant: a hang, not a red).
    async fn read_exact_now(stream: &mut (impl AsyncReadExt + Unpin), buf: &mut [u8]) {
        tokio::time::timeout(Duration::from_secs(5), stream.read_exact(buf))
            .await
            .expect("the echo arrives within 5 s")
            .expect("echo");
    }

    // ── the health axis across the crossing (ABI v3, ADR-0549) ───────────

    /// ADR-0549 / FR-30 (b) at the C BOUNDARY: `health` survives the crossing
    /// as itself and its set is CLOSED there, not merely in the core. A
    /// registrant that declares `ZW_HEALTH_FAILED` is read back `Failed` and
    /// is NOT ready — at the readiness it measured, and at 100 too, because
    /// health never permits a dial — while `STARTING` at 100 is the ordinary
    /// ready descriptor. An integer outside the set is `ZW_RC_DESCRIPTOR` at
    /// every verb, with nothing stored: the `from_raw` refusal is what keeps a
    /// MIS-BUILT v3 host's garbage sixth word from being read as a health. (It
    /// is NOT what protects against a v2 host: that is the version check at
    /// `register`, which returns `ZW_RC_ABI` before any pointer is read, and a
    /// v2 caller can never reach `notify`/`update` because those need a token
    /// only a successful v3 `register` mints. Stated precisely because the
    /// looser version of this sentence claimed the closed set was the v2
    /// guard, and a reader could then weaken the version check believing the
    /// range check still covered it.)
    #[test]
    fn a_declared_health_crosses_the_c_boundary_and_its_set_is_closed() {
        let _g = serial();
        let fake = FakeHost::new();
        let vt = fake.vtable();

        // FAILED at the readiness the registrant MEASURED (never floored).
        let failed = Descriptor::with_health(b"Shadowsocks", 40, 2, 1, ZW_HEALTH_FAILED);
        let token = register(&fake, failed);
        let read = CAbiHostDialer.descriptor().expect("registered");
        assert_eq!(read.health, TransportHealth::Failed);
        assert_eq!(read.readiness, 40, "the measured readiness crosses intact");
        assert!(read.is_failed() && !read.is_ready());

        // FAILED at 100: health forbids, it never permits.
        assert_eq!(
            notify(
                &token,
                Descriptor::with_health(b"Shadowsocks", 100, 2, 1, ZW_HEALTH_FAILED),
                0
            ),
            ZW_RC_OK
        );
        let read = CAbiHostDialer.descriptor().expect("registered");
        assert!(
            !read.is_ready(),
            "a FAILED transport is not ready at readiness 100"
        );

        // THE POLARITY, so this test cannot pass by reading every descriptor as
        // failed — and it is asserted with STARTING, not READY, because ADR-0549
        // D1's rule is that `health` never PERMITS a dial: READINESS ALONE opens
        // the gate, and only FAILED closes it. So STARTING at 100 must be READY
        // in the SDK's sense. (Written first with `ZW_HEALTH_READY` here, which
        // made the comment a claim the code did not test — the security angle
        // caught it. A registrant that cannot carry a dial says so with
        // readiness below 100, or refuses synchronously; the header says that
        // now too.)
        assert_eq!(
            notify(
                &token,
                Descriptor::with_health(b"Shadowsocks", 100, 2, 1, ZW_HEALTH_STARTING),
                0
            ),
            ZW_RC_OK
        );
        let read = CAbiHostDialer.descriptor().expect("registered");
        assert!(
            read.is_ready() && !read.is_failed(),
            "STARTING at readiness 100 is ready — health forbids, it never permits"
        );
        assert_eq!(read.health, TransportHealth::Starting);

        // ...and READY at 100 likewise, so both non-failed values agree.
        assert_eq!(
            notify(
                &token,
                Descriptor::with_health(b"Shadowsocks", 100, 2, 1, ZW_HEALTH_READY),
                0
            ),
            ZW_RC_OK
        );
        let read = CAbiHostDialer.descriptor().expect("registered");
        assert!(read.is_ready() && !read.is_failed());
        assert_eq!(read.health, TransportHealth::Ready);

        // The set is CLOSED at the boundary: 3 and u32::MAX are refused and
        // NOTHING is stored — the ready descriptor above still stands.
        for bad in [3u32, u32::MAX] {
            let out_of_set = Descriptor::with_health(b"Shadowsocks", 100, 2, 1, bad);
            assert_eq!(
                notify(&token, out_of_set, 0),
                ZW_RC_DESCRIPTOR,
                "health {bad} is outside the closed set"
            );
            assert_eq!(
                notify(&token, out_of_set, 1),
                ZW_RC_DESCRIPTOR,
                "health {bad} is refused with retire = 1 too"
            );
            let unchanged = CAbiHostDialer.descriptor().expect("still registered");
            assert_eq!(
                unchanged.health,
                TransportHealth::Ready,
                "a refused descriptor stores nothing"
            );
            assert_eq!(unchanged.readiness, 100);
        }

        // And at `register`, before anything is stored.
        reset_registry_for_test();
        let mut token_out = [0xAAu8; NET_DIALER_AUTH_TOKEN_BYTES];
        let rc = unsafe {
            zec_wallet_register_net_dialer(
                HOST_DIALER_ABI_VERSION,
                ptr::addr_of!(vt).cast::<ZwNetDialerV1>(),
                dptr(&Descriptor::with_health(b"Tor", 100, 1, 1, 3)),
                token_out.as_mut_ptr(),
            )
        };
        assert_eq!(rc, ZW_RC_DESCRIPTOR);
        assert!(
            CAbiHostDialer.descriptor().is_none(),
            "the slot stays empty"
        );
        assert_eq!(token_out, [0xAAu8; NET_DIALER_AUTH_TOKEN_BYTES]);
    }

    // ── T27 (ADR-0547) ───────────────────────────────────────────────────

    /// Spec §6.1 E13 / T27: the host's transport NAME is validated at every
    /// verb under exactly four rules — non-empty, at most 32 bytes, UTF-8, no
    /// control character — and `name_len` is bounded BEFORE the bytes are
    /// sliced (a `name_len` past the array never indexes it). Each bad name
    /// returns `-6` from `register` (the slot stays empty, `auth_out`
    /// untouched), from `update` (the registration, the generation and the
    /// stored descriptor unchanged) and from `notify` (the same, even with
    /// `retire = 1`); a 32-byte name of four-byte characters is accepted and
    /// reads back verbatim with its exposure.
    #[test]
    fn a_bad_transport_name_is_refused_at_every_verb() {
        let _g = serial();
        let fake = FakeHost::new();
        let vt = fake.vtable();
        let mut past_the_array = Descriptor::new(b"Tor", 100, 1, 1);
        past_the_array.name_len = 33;
        let mut far_past_the_array = Descriptor::new(b"Tor", 100, 1, 1);
        far_past_the_array.name_len = u32::MAX;
        let bad_names = [
            ("an empty name", Descriptor::new(b"", 100, 1, 1)),
            ("a blank name", Descriptor::new(b"   ", 100, 1, 1)),
            // 32 VALID bytes in the array with name_len 33: bounding refuses
            // it; a truncating implementation would accept a shortened name.
            (
                "32 valid bytes with name_len 33",
                Descriptor::new(&[b'a'; 33], 100, 1, 1),
            ),
            (
                "a bidi override",
                Descriptor::new("Tor\u{202E}".as_bytes(), 100, 1, 1),
            ),
            (
                "a zero-width space",
                Descriptor::new("To\u{200B}r".as_bytes(), 100, 1, 1),
            ),
            ("name_len past the array", past_the_array),
            ("name_len far past the array", far_past_the_array),
            (
                "a NUL inside the counted bytes",
                Descriptor::new(b"Tor\0", 100, 1, 1),
            ),
            (
                "an invalid UTF-8 byte",
                Descriptor::new(&[0xFF, b'T', b'o', b'r'], 100, 1, 1),
            ),
            (
                "a C0 control character",
                Descriptor::new(b"Tor\n", 100, 1, 1),
            ),
            (
                "a C1 control character",
                Descriptor::new("Tor\u{85}".as_bytes(), 100, 1, 1),
            ),
        ];
        // register: -6, the slot stays empty, auth_out untouched.
        for (what, bad) in &bad_names {
            let mut token = [0xAAu8; NET_DIALER_AUTH_TOKEN_BYTES];
            let rc = unsafe {
                zec_wallet_register_net_dialer(
                    HOST_DIALER_ABI_VERSION,
                    ptr::addr_of!(vt).cast::<ZwNetDialerV1>(),
                    dptr(bad),
                    token.as_mut_ptr(),
                )
            };
            assert_eq!(rc, ZW_RC_DESCRIPTOR, "register refuses {what}");
            assert!(
                registered_host_dialer().is_none(),
                "{what}: nothing was registered"
            );
            assert_eq!(
                token, [0xAAu8; NET_DIALER_AUTH_TOKEN_BYTES],
                "{what}: auth_out untouched"
            );
        }
        // A valid registration, then every bad name at update and notify:
        // -6 and NOTHING changes — not the registration, not the generation,
        // not the stored descriptor.
        let token = register(&fake, TOR_READY);
        let before = CAbiHostDialer.descriptor().expect("registered");
        let generation = current_generation();
        for (what, bad) in &bad_names {
            assert_eq!(
                replace(&fake, &token, *bad),
                ZW_RC_DESCRIPTOR,
                "update refuses {what}"
            );
            assert_eq!(
                notify(&token, *bad, 1),
                ZW_RC_DESCRIPTOR,
                "notify refuses {what}"
            );
            assert_eq!(
                CAbiHostDialer.descriptor(),
                Some(before),
                "{what}: the stored descriptor stands"
            );
            assert_eq!(
                current_generation(),
                generation,
                "{what}: the generation did not move"
            );
        }
        assert_eq!(
            notify(&token, TOR_READY, 0),
            ZW_RC_OK,
            "the token still works after the refusals"
        );
        // The bound is inclusive: 32 bytes of four-byte characters cross
        // verbatim, and the exposure beside them.
        let wide = "😀😀😀😀😀😀😀😀";
        assert_eq!(wide.len(), HOST_TRANSPORT_NAME_MAX_BYTES);
        assert_eq!(
            notify(&token, Descriptor::new(wide.as_bytes(), 100, 1, 2), 0),
            ZW_RC_OK
        );
        let stored = CAbiHostDialer.descriptor().expect("registered");
        assert_eq!(
            stored.name.as_str(),
            wide,
            "the host's name crosses verbatim"
        );
        assert_eq!(stored.exposure, TransportExposure::Exposed);
    }

    /// A1 (the C3a security pass, HIGH) — the dial budget the host must honour
    /// and the bound the SDK actually applies are ONE number.
    ///
    /// Before this, the budget was a sentence in the header's READINESS
    /// paragraph while `DIAL_TIMEOUT_SECS` was the thing that really fired, and
    /// the Tor plugin restated the figure as its own literal. Nothing compared
    /// the three, so a change to the constant would have left every host
    /// reading the old number — and a host that parks a dial past the real
    /// budget turns a wait into a `Preferred` wallet's clearnet fallback.
    ///
    /// The header side is parsed from the shipped text rather than mirrored in
    /// Rust, so a hand-edit of either file fails HERE rather than on a host's
    /// device.
    #[test]
    fn the_header_dial_budget_matches_the_core_constant() {
        let budget = *header_defines()
            .get("ZW_NET_DIALER_DIAL_BUDGET_SECS")
            .expect("the header defines ZW_NET_DIALER_DIAL_BUDGET_SECS with a value");
        assert_eq!(
            budget,
            zec_wallet_core::constants::DIAL_TIMEOUT_SECS as i64,
            "the header's published dial budget and the SDK's own bound must be \
             the same number — a host cannot honour a clock it reads wrong"
        );
    }

    /// (`on-device-log-layer-phase-1.md` §3): the FIVE events this module
    /// emits, fired by the real exports and read through the REAL device-log
    /// layer — the one that ships — with its §5.4 enforcement on. Every one
    /// reaches the sink, on the bridge's tag, and NOTHING is withheld: what a
    /// connectivity diagnosis needs (did a descriptor arrive, did it go ready,
    /// did it go FAILED, did a retire land) is exactly what a device will show.
    ///
    /// This closes the seam from the producer's side. Until now these
    /// events were on a target neither the capture guard nor any log layer
    /// matched, and the only check on `log_descriptor` was a string scan of its
    /// source from another crate; this drives the function itself.
    ///
    /// Watched against: a `transport_exposure = ?d.exposure` field added to
    /// `log_descriptor` (the line gains `withheld=1` — the disclosure,
    /// refused at runtime now as well as by the source pin); the
    /// `wallet.host_dialer_retired` event moved to target `zec_wallet_net` (its
    /// line vanishes).
    #[test]
    fn every_bridge_event_reaches_the_sink_with_nothing_withheld() {
        use crate::device_log::{BRIDGE_TAG, Scope, test_sink::installed};
        let _g = serial();
        let fake = FakeHost::new();
        let lines = installed(Scope::AllSdkTargets, || {
            let starting = Descriptor::with_health(b"Tor", 37, 1, 1, ZW_HEALTH_STARTING);
            let token = register(&fake, starting);
            assert_eq!(replace(&fake, &token, TOR_READY), ZW_RC_OK);
            let failed = Descriptor::with_health(b"Tor", 100, 1, 1, ZW_HEALTH_FAILED);
            assert_eq!(notify(&token, failed, 0), ZW_RC_OK);
            assert_eq!(notify(&token, starting, 1), ZW_RC_OK);
            assert_eq!(
                unsafe { zec_wallet_update_net_dialer(token.as_ptr(), ptr::null(), ptr::null()) },
                ZW_RC_OK
            );
        });
        for (level, tag, line) in &lines {
            assert_eq!(*level, tracing::Level::INFO);
            assert_eq!(*tag, BRIDGE_TAG, "{line}");
            assert!(
                !line.contains("withheld"),
                "a bridge field was refused: {line}"
            );
            assert!(
                !line.contains("Tor"),
                "the host's transport name never reaches a log: {line}"
            );
        }
        let text: Vec<&str> = lines.iter().map(|(_, _, s)| s.as_str()).collect();
        assert_eq!(
            text,
            [
                "wallet.host_dialer_registered",
                "wallet.host_dialer_descriptor transport_readiness=37 transport_health=Starting",
                "wallet.host_dialer_replaced",
                "wallet.host_dialer_descriptor transport_readiness=100 transport_health=Ready",
                "wallet.host_dialer_descriptor transport_readiness=100 transport_health=Failed",
                "wallet.host_dialer_retired",
                "wallet.host_dialer_descriptor transport_readiness=37 transport_health=Starting",
                "wallet.host_dialer_cleared",
            ]
        );
    }
}
