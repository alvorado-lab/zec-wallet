//! The plugin's two C boundaries (`tor-plugin.md` §2.1) — and the ONE module
//! of the crate that may use `unsafe`.
//!
//! **Toward the wallet** — the plugin's OWN `#[repr(C)]` mirror of
//! `zec_wallet_net_dialer.h` (ABI v4), declared independently of the wallet's
//! Rust types: the wallet is not a dependency, it is reached BY SYMBOL at
//! runtime (D8). P14 parses the wallet's header and pins every value and the
//! descriptor's layout, so a drift fails a test rather than a device.
//!
//! **Toward Dart** — the mirror of the plugin's own `include/zec_wallet_tor.h`
//! (the `ffigen` source), the conversions from the typed [`Status`] to the
//! fixed-width `zwt_status`, and the version export the Dart side reads BY
//! NAME before any other call (the wallet's FR-33 lesson, applied here).
//!
//! **The resolver** — the wallet's three verbs, found in the ALREADY-LOADED
//! wallet image (never loaded by the plugin: that would change who owns its
//! lifetime). Apple verifies CONTAINMENT (the image sits under the main
//! bundle); ELF and Windows verify IDENTITY only, so a same-named image from
//! a writable path is not excluded there. The decisions are
//! pure functions, unit-tested on the host; each platform's lookup is a thin
//! shell. Relim's `wallet_registrar.rs` is the shipped Apple/ELF precedent.

#![allow(unsafe_code)]

use std::ffi::c_void;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use zeroize::{Zeroize, Zeroizing};

use crate::bootstrap::RandomJitter;
use crate::constants::{PLUGIN_RUNTIME_WORKERS, STAGING_BUFFER_BYTES};
use crate::engine::ArtiFactory;
use crate::lifecycle::remove_tor_state;
use crate::log::plugin_dispatch;
use crate::status::{Blockage, FAILURE_CLASSES, Phase, Status};
use crate::trampoline::{
    Deps, LinkResolver, PauseClock, Plugin, ReadOutcome, StatusSink, Token, WalletLink, reentrant,
};

// ─── The wallet's contract: zec_wallet_net_dialer.h, ABI v4 ──────────────────

/// `ZW_NET_DIALER_ABI_VERSION` — the version `register` must be called with; 4 since an earlier revision (ADR-0553 as built — why the mirror follows with no change to what the plugin pushes: `tor-plugin.md` A17/P14; one line so the cited lines below stay put).
pub const ZW_NET_DIALER_ABI_VERSION: u32 = 4;
/// `ZW_NET_DIALER_AUTH_TOKEN_BYTES` — the mutation token `register` mints.
pub const ZW_NET_DIALER_AUTH_TOKEN_BYTES: usize = 32;
/// `ZW_ISOLATION_KEY_MAX_BYTES`.
pub const ZW_ISOLATION_KEY_MAX_BYTES: usize = 256;
/// `ZW_HOST_NAME_MAX_BYTES`.
pub const ZW_HOST_NAME_MAX_BYTES: usize = 253;
/// `ZW_TRANSPORT_NAME_MAX_BYTES`.
pub const ZW_TRANSPORT_NAME_MAX_BYTES: usize = 32;

/// `ZW_RC_*` — what the wallet's three verbs return.
pub const ZW_RC_OK: i32 = 0;
/// Registry lock poisoned by an earlier panic.
pub const ZW_RC_POISONED: i32 = -1;
/// `register`: the slot is taken; `update`/`notify`: a bad token or an empty slot.
pub const ZW_RC_OCCUPIED: i32 = -2;
/// A required pointer or vtable entry is NULL.
pub const ZW_RC_NULL_ARG: i32 = -3;
/// The wallet panicked inside the verb; nothing changed.
pub const ZW_RC_PANICKED: i32 = -4;
/// `abi_version` is not the wallet's.
pub const ZW_RC_ABI: i32 = -5;
/// A descriptor value out of its closed range, or the name invalid.
pub const ZW_RC_DESCRIPTOR: i32 = -6;

/// `ZW_DIAL_*` — the codes a dial or io op completes with.
pub const ZW_DIAL_OK: u32 = 0;
/// Not ready: the SDK waits (`Preferred` too).
pub const ZW_DIAL_NOT_READY: u32 = 1;
/// The device has no route — one of the two codes `Preferred` falls back on.
pub const ZW_DIAL_UNREACHABLE: u32 = 2;
/// Timed out — the other code `Preferred` falls back on.
pub const ZW_DIAL_TIMEOUT: u32 = 3;
/// This request will not be carried; no fallback.
pub const ZW_DIAL_REFUSED: u32 = 4;
/// A rebuild or a dispose intervened.
pub const ZW_DIAL_RETIRED: u32 = 5;

/// `ZW_ISOLATION_*` — whether the transport honours isolation keys.
pub const ZW_ISOLATION_UNKNOWN: u32 = 0;
/// Isolation keys are honoured (Tor: per-key circuit groups).
pub const ZW_ISOLATION_SUPPORTED: u32 = 1;
/// Isolation keys are ignored.
pub const ZW_ISOLATION_UNSUPPORTED: u32 = 2;

/// `ZW_EXPOSURE_*` — whether the transport hides the device's address.
pub const ZW_EXPOSURE_UNKNOWN: u32 = 0;
/// The server does not see the device's address (Tor).
pub const ZW_EXPOSURE_HIDDEN: u32 = 1;
/// The server sees the device's address.
pub const ZW_EXPOSURE_EXPOSED: u32 = 2;

/// `ZW_HEALTH_*` (v3) — the registrant's own verdict on its transport.
pub const ZW_HEALTH_STARTING: u32 = 0;
/// Healthy.
pub const ZW_HEALTH_READY: u32 = 1;
/// The registrant judged its transport failed; forbids a dial whatever the
/// readiness.
pub const ZW_HEALTH_FAILED: u32 = 2;

/// `zw_transport_descriptor` — 52 bytes, 4-aligned, `health` appended at 48.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZwTransportDescriptor {
    /// UTF-8; `name_len` bytes used, the rest ignored.
    pub name: [u8; ZW_TRANSPORT_NAME_MAX_BYTES],
    /// `1..=ZW_TRANSPORT_NAME_MAX_BYTES`.
    pub name_len: u32,
    /// `0..=100`.
    pub readiness: u32,
    /// `ZW_ISOLATION_*`.
    pub isolation: u32,
    /// `ZW_EXPOSURE_*`.
    pub exposure: u32,
    /// `ZW_HEALTH_*`.
    pub health: u32,
}

/// `zw_dial_complete_fn`.
pub type ZwDialCompleteFn = extern "C" fn(sdk_ctx: *mut c_void, op_id: u64, code: u32, stream: u64);
/// `zw_io_complete_fn`.
pub type ZwIoCompleteFn = extern "C" fn(sdk_ctx: *mut c_void, op_id: u64, code: u32, n: usize);

/// `zw_net_dialer_v1.dial`.
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
/// `zw_net_dialer_v1.read`.
pub type ZwReadFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    stream: u64,
    buf: *mut u8,
    cap: usize,
    op_id: u64,
    sdk_ctx: *mut c_void,
    complete: ZwIoCompleteFn,
) -> u32;
/// `zw_net_dialer_v1.write`.
pub type ZwWriteFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    stream: u64,
    buf: *const u8,
    len: usize,
    op_id: u64,
    sdk_ctx: *mut c_void,
    complete: ZwIoCompleteFn,
) -> u32;
/// `zw_net_dialer_v1.close`.
pub type ZwCloseFn = unsafe extern "C" fn(ctx: *mut c_void, stream: u64);

/// `zw_net_dialer_v1` — the vtable the plugin installs. `ctx` is the plugin's
/// process-lifetime trampoline (§3.3).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZwNetDialerV1 {
    /// Host-opaque; the plugin owns its lifetime (for the process).
    pub ctx: *mut c_void,
    /// Connect.
    pub dial: Option<ZwDialFn>,
    /// Read up to `cap` bytes.
    pub read: Option<ZwReadFn>,
    /// Write up to `len` bytes.
    pub write: Option<ZwWriteFn>,
    /// The SDK's cancel + release.
    pub close: Option<ZwCloseFn>,
}

/// `zec_wallet_register_net_dialer`.
pub type RegisterFn = unsafe extern "C" fn(
    abi_version: u32,
    dialer: *const ZwNetDialerV1,
    descriptor: *const ZwTransportDescriptor,
    auth_out: *mut u8,
) -> i32;
/// `zec_wallet_update_net_dialer`.
pub type UpdateFn = unsafe extern "C" fn(
    auth: *const u8,
    dialer: *const ZwNetDialerV1,
    descriptor: *const ZwTransportDescriptor,
) -> i32;
/// `zec_wallet_net_dialer_notify`.
pub type NotifyFn = unsafe extern "C" fn(
    auth: *const u8,
    descriptor: *const ZwTransportDescriptor,
    retire: u32,
) -> i32;

/// The three verbs' exported names, NUL-terminated for `dlsym`.
pub const REGISTER_SYMBOL: &[u8] = b"zec_wallet_register_net_dialer\0";
/// See [`REGISTER_SYMBOL`].
pub const UPDATE_SYMBOL: &[u8] = b"zec_wallet_update_net_dialer\0";
/// See [`REGISTER_SYMBOL`].
pub const NOTIFY_SYMBOL: &[u8] = b"zec_wallet_net_dialer_notify\0";

// ─── The plugin's own contract: include/zec_wallet_tor.h ─────────────────────

/// `ZWT_ABI_VERSION` — answered by [`zec_wallet_tor_abi_version`].
pub const ZWT_ABI_VERSION: u32 = 1;

/// `ZWT_RC_*` — every export's return code.
pub const ZWT_RC_OK: i32 = 0;
/// The wallet library is not in this process: `RustLib.init()` first.
pub const ZWT_RC_WALLET_NOT_LOADED: i32 = -1;
/// The wallet answered `ZW_RC_ABI`.
pub const ZWT_RC_ABI_MISMATCH: i32 = -2;
/// The wallet answered `ZW_RC_OCCUPIED`: another registrant holds the slot.
pub const ZWT_RC_SLOT_OCCUPIED: i32 = -3;
/// The wallet answered `ZW_RC_POISONED`.
pub const ZWT_RC_REGISTRY_POISONED: i32 = -4;
/// The wallet answered `ZW_RC_DESCRIPTOR` — a build defect.
pub const ZWT_RC_DESCRIPTOR_REFUSED: i32 = -5;
/// `tor_dir` empty, relative, or not creatable/openable.
pub const ZWT_RC_INVALID_DATA_DIR: i32 = -6;
/// The bridge paste was refused before registration; the class is in the status.
pub const ZWT_RC_BRIDGES_REFUSED: i32 = -7;
/// arti could not be built after registration; the slot was retired and cleared.
pub const ZWT_RC_ENGINE_SETUP: i32 = -8;
/// No successful `init` yet (or `clear_state` while an engine runs).
pub const ZWT_RC_NOT_INITIALIZED: i32 = -9;
/// `dispose` ran.
pub const ZWT_RC_DISPOSED: i32 = -10;
/// A required pointer was NULL.
pub const ZWT_RC_NULL_ARG: i32 = -11;
/// The plugin panicked inside the export.
pub const ZWT_RC_PANICKED: i32 = -12;
/// `clear_state` or `init` while an earlier Tor client is still stopping (a
/// mint in flight, or a client `dispose` stopped waiting for at its bound):
/// nothing changed; call again shortly.
pub const ZWT_RC_STOPPING: i32 = -13;
/// `clear_state` or `init` after an earlier Tor client did not finish shutting
/// down within its bound: it may still write, for the life of this process.
/// Nothing changed; restart the app and clear (or init) before anything else.
pub const ZWT_RC_RESTART_REQUIRED: i32 = -14;

/// `ZWT_PHASE_*`.
pub const ZWT_PHASE_IDLE: u32 = 0;
/// See [`Phase::Bootstrapping`].
pub const ZWT_PHASE_BOOTSTRAPPING: u32 = 1;
/// See [`Phase::Ready`].
pub const ZWT_PHASE_READY: u32 = 2;
/// See [`Phase::Failed`].
pub const ZWT_PHASE_FAILED: u32 = 3;
/// See [`Phase::Suspended`].
pub const ZWT_PHASE_SUSPENDED: u32 = 4;
/// See [`Phase::NotRegistered`].
pub const ZWT_PHASE_NOT_REGISTERED: u32 = 5;

/// `ZWT_BLOCKAGE_NONE` — no blockage reported; `1..=7` are [`Blockage::ALL`]
/// in order.
pub const ZWT_BLOCKAGE_NONE: u32 = 0;
/// `ZWT_CLASS_NONE` — no failure class; `1..=20` are [`FAILURE_CLASSES`] in
/// order.
pub const ZWT_CLASS_NONE: u32 = 0;

/// `zwt_status` — four closed integers, 16 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ZwtStatus {
    /// `ZWT_PHASE_*`.
    pub phase: u32,
    /// `0..=100`: the readiness last pushed to the wallet.
    pub readiness: u32,
    /// `ZWT_BLOCKAGE_*`.
    pub blockage: u32,
    /// `ZWT_CLASS_*`.
    pub failure_class: u32,
}

/// `zwt_config` — two byte spans, valid for the call only. Its `Debug` is
/// hand-written and prints NOTHING of either span, lengths included: "this
/// user has bridges, N bytes of them" is the fact an examiner of a censorship
/// tool wants (the security angle's A3).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZwtConfig {
    /// UTF-8, absolute; required.
    pub tor_dir: *const u8,
    /// Length of `tor_dir`.
    pub tor_dir_len: usize,
    /// UTF-8 bridge lines; NULL with length 0 = none.
    pub bridges: *const u8,
    /// Length of `bridges`.
    pub bridges_len: usize,
}

impl std::fmt::Debug for ZwtConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ZwtConfig { <redacted> }")
    }
}

/// `zwt_status_fn`.
pub type ZwtStatusFn = extern "C" fn(ctx: *mut c_void, status: *const ZwtStatus);

/// The `ZWT_PHASE_*` code of a phase.
pub fn phase_code(phase: Phase) -> u32 {
    match phase {
        Phase::Idle => ZWT_PHASE_IDLE,
        Phase::Bootstrapping => ZWT_PHASE_BOOTSTRAPPING,
        Phase::Ready => ZWT_PHASE_READY,
        Phase::Failed => ZWT_PHASE_FAILED,
        Phase::Suspended => ZWT_PHASE_SUSPENDED,
        Phase::NotRegistered => ZWT_PHASE_NOT_REGISTERED,
    }
}

/// The `ZWT_BLOCKAGE_*` code of a blockage: its position in [`Blockage::ALL`]
/// plus one, `ZWT_BLOCKAGE_NONE` for none.
pub fn blockage_code(blockage: Option<Blockage>) -> u32 {
    blockage.map_or(ZWT_BLOCKAGE_NONE, |b| {
        let index = Blockage::ALL
            .iter()
            .position(|x| *x == b)
            .expect("Blockage::ALL lists every variant (the P14 test pins it)");
        index as u32 + 1
    })
}

/// The `ZWT_CLASS_*` code of a failure class: its position in
/// [`FAILURE_CLASSES`] plus one, `ZWT_CLASS_NONE` for none. A class outside
/// the closed table is a programming error — the status only ever names one
/// of [`FAILURE_CLASSES`] — and is reported as `ZWT_CLASS_NONE` rather than
/// a code the Dart table does not have (P14 pins that every class the plugin
/// produces is in the table).
pub fn class_code(class: Option<&str>) -> u32 {
    class
        .and_then(|c| FAILURE_CLASSES.iter().position(|x| *x == c))
        .map_or(ZWT_CLASS_NONE, |i| i as u32 + 1)
}

impl From<&Status> for ZwtStatus {
    fn from(status: &Status) -> Self {
        Self {
            phase: phase_code(status.phase),
            readiness: status.readiness,
            blockage: blockage_code(status.blockage),
            failure_class: class_code(status.failure_class),
        }
    }
}

/// `zec_wallet_tor_abi_version` — [`ZWT_ABI_VERSION`]. Pure: no argument, no
/// state, cannot fail or panic. The Dart side resolves it BY NAME before any
/// other call, so a Dart half and a native half built from different releases
/// fail loudly instead of reading a status one field off.
#[unsafe(no_mangle)]
pub extern "C" fn zec_wallet_tor_abi_version() -> u32 {
    ZWT_ABI_VERSION
}

// ─── The resolver: the wallet's three verbs, by name (D8) ────────────────────

/// The wallet's three verbs, resolved together: a wallet image that exports
/// `register` but not `update` could never be deregistered, so all three are
/// required before any is used (Relim's rule for its seed port).
#[derive(Clone, Copy)]
pub struct Verbs {
    /// `zec_wallet_register_net_dialer`.
    pub register: RegisterFn,
    /// `zec_wallet_update_net_dialer`.
    pub update: UpdateFn,
    /// `zec_wallet_net_dialer_notify`.
    pub notify: NotifyFn,
}

/// How the verbs are found. Production: [`ImageResolver`]; tests: a recording
/// fake (C3b) or the wallet's real library loaded by the test harness (P19).
pub trait VerbResolver: Send + Sync {
    /// `None` = the wallet image is not loaded in this process, or it is
    /// ambiguous (more than one image matches) — `walletNotLoaded`.
    fn resolve(&self) -> Option<Verbs>;
}

/// The production resolver: the already-loaded wallet image, per platform.
pub struct ImageResolver;

impl VerbResolver for ImageResolver {
    fn resolve(&self) -> Option<Verbs> {
        // ONE image chosen, all three read from it (the security angle's
        // A7): a walk per symbol could pick two different images.
        let [register, update, notify] =
            platform::resolve_symbols([REGISTER_SYMBOL, UPDATE_SYMBOL, NOTIFY_SYMBOL])?;
        // SAFETY: each address is an export of the image-verified wallet
        // library under the name the wallet's header declares; the types they
        // are cast to are this module's mirrors of those declarations, which
        // P14 pins against the header itself.
        unsafe {
            Some(Verbs {
                register: std::mem::transmute::<*const c_void, RegisterFn>(register),
                update: std::mem::transmute::<*const c_void, UpdateFn>(update),
                notify: std::mem::transmute::<*const c_void, NotifyFn>(notify),
            })
        }
    }
}

/// The ELF wallet library's file name (Android, desktop Linux).
pub const WALLET_ELF_NAME: &str = "libzec_wallet.so";
/// The Apple framework's binary name and directory component.
pub const WALLET_FRAMEWORK_BINARY: &str = "zec_wallet";
/// See [`WALLET_FRAMEWORK_BINARY`].
pub const WALLET_FRAMEWORK_COMPONENT: &str = "/zec_wallet.framework/";
/// The Windows wallet library.
pub const WALLET_DLL_NAME: &str = "zec_wallet.dll";

/// ELF: the resolved symbol's backing image is the wallet library — an exact
/// BASENAME match, never a substring (a decoy `/x/zec_wallet_evil.so` would
/// pass a substring test). Pure.
pub fn elf_image_is_wallet(fname: &str) -> bool {
    std::path::Path::new(fname)
        .file_name()
        .and_then(|n| n.to_str())
        == Some(WALLET_ELF_NAME)
}

/// Apple: the `.app` bundle root of an executable path — the prefix up to and
/// including the INNERMOST `.app/` (Relim's `bundle_root_of`: the tightest
/// residency region). `None` outside a bundle — callers fail closed. Pure.
pub fn bundle_root_of(exec_path: &str) -> Option<&str> {
    const APP_DIR: &str = ".app/";
    exec_path
        .rfind(APP_DIR)
        .map(|i| &exec_path[..i + APP_DIR.len()])
}

/// Apple, the FRAMEWORK case (tried first; the SDK's own example and Relim):
/// an image whose basename is the wallet binary, under a
/// `/zec_wallet.framework/` component, inside the main app bundle. A
/// free-floating framework a `DYLD_INSERT_LIBRARIES` decoy could plant fails
/// the bundle prefix though it matches the shape. Pure.
pub fn apple_image_is_wallet_framework(image: &str, main_executable: &str) -> bool {
    let basename_is_binary = std::path::Path::new(image)
        .file_name()
        .and_then(|n| n.to_str())
        == Some(WALLET_FRAMEWORK_BINARY);
    // dyld's paths are not canonicalised: `…/Runner.app/Frameworks/../../../
    // tmp/zec_wallet.framework/zec_wallet` passes a raw prefix test (the
    // security angle's A5), so a climbing component refuses outright.
    let climbs = image.split('/').any(|c| c == "..");
    basename_is_binary
        && !climbs
        && image.contains(WALLET_FRAMEWORK_COMPONENT)
        && bundle_root_of(main_executable).is_some_and(|root| image.starts_with(root))
}

/// Apple: which image to use, given the candidates. The FRAMEWORK case wins
/// when exactly one in-bundle framework image matches; the STATIC case (the
/// wallet linked into the main executable — `use_frameworks! :linkage =>
/// :static`, or no `use_frameworks!`) applies when none does; BOTH present, or
/// two frameworks, is ambiguous and fails closed (Relim's rule — never guess
/// which image receives the wallet's registration). Pure.
pub fn apple_choose(frameworks: &[String], static_symbol_in_main: bool) -> AppleChoice {
    match (frameworks, static_symbol_in_main) {
        ([one], false) => AppleChoice::Framework(one.clone()),
        ([], true) => AppleChoice::MainExecutable,
        ([], false) => AppleChoice::Absent,
        _ => AppleChoice::Ambiguous,
    }
}

/// [`apple_choose`]'s answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppleChoice {
    /// The in-bundle `zec_wallet.framework` image at this path.
    Framework(String),
    /// The main executable (the static case).
    MainExecutable,
    /// No wallet image in this process.
    Absent,
    /// More than one candidate — refused.
    Ambiguous,
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod platform {
    //! Apple: a walk of the loaded mach-o images for the FRAMEWORK case, then
    //! `RTLD_MAIN_ONLY` for the STATIC case; every resolved address is checked
    //! with `dladdr` against the chosen image.
    use std::ffi::{CStr, c_void};

    use super::{AppleChoice, apple_choose, apple_image_is_wallet_framework};

    // dyld's image-introspection API (`<mach-o/dyld.h>`, in libSystem — always
    // linked on Apple), declared locally as Relim does rather than pulling a
    // crate into this module.
    unsafe extern "C" {
        fn _dyld_image_count() -> u32;
        fn _dyld_get_image_name(image_index: u32) -> *const std::os::raw::c_char;
    }

    /// The loader-owned path of image `i`, or `None`.
    fn image_name(i: u32) -> Option<String> {
        // SAFETY: returns a loader-owned, NUL-terminated, process-lifetime C
        // string or null (guarded). The image list is read at `init`, a
        // quiescent point; the API is not synchronized against a concurrent
        // `dlclose`, and no image of interest unloads here (Relim's note).
        let name = unsafe { _dyld_get_image_name(i) };
        if name.is_null() {
            return None;
        }
        // SAFETY: non-null and NUL-terminated per the dyld contract.
        Some(
            unsafe { CStr::from_ptr(name) }
                .to_string_lossy()
                .into_owned(),
        )
    }

    /// The image `addr` belongs to, by `dladdr`.
    fn backing_image(addr: *const c_void) -> Option<String> {
        // SAFETY: all-zero is a valid `Dl_info`; `dladdr` reads `addr` only as
        // an opaque code address and fills `info` with loader-owned strings.
        let mut info: libc::Dl_info = unsafe { std::mem::zeroed() };
        let ok = unsafe { libc::dladdr(addr, &mut info) };
        if ok == 0 || info.dli_fname.is_null() {
            return None;
        }
        // SAFETY: non-null, NUL-terminated, loader-owned.
        Some(
            unsafe { CStr::from_ptr(info.dli_fname) }
                .to_string_lossy()
                .into_owned(),
        )
    }

    /// `symbol` in the main executable, verified by `dladdr` to live there.
    fn in_main(symbol: &[u8], main: &str) -> Option<*const c_void> {
        // SAFETY: `RTLD_MAIN_ONLY` searches the main executable only;
        // `symbol` is NUL-terminated (the module's constants).
        let addr = unsafe { libc::dlsym(libc::RTLD_MAIN_ONLY, symbol.as_ptr().cast()) };
        (!addr.is_null() && backing_image(addr).as_deref() == Some(main))
            .then_some(addr.cast_const())
    }

    pub(super) fn resolve_symbols(symbols: [&[u8]; 3]) -> Option<[*const c_void; 3]> {
        let main = image_name(0)?;
        // SAFETY: argless dyld query.
        let count = unsafe { _dyld_image_count() };
        let frameworks: Vec<String> = (0..count)
            .filter_map(image_name)
            .filter(|path| apple_image_is_wallet_framework(path, &main))
            .collect();

        match apple_choose(&frameworks, in_main(symbols[0], &main).is_some()) {
            AppleChoice::MainExecutable => Some([
                in_main(symbols[0], &main)?,
                in_main(symbols[1], &main)?,
                in_main(symbols[2], &main)?,
            ]),
            AppleChoice::Framework(path) => {
                use libloading::os::unix::Library;
                // SAFETY: reopening an ALREADY-MAPPED, code-signed embedded
                // framework by the path dyld reported, with `RTLD_NOLOAD` — no
                // new image is loaded and no initializer runs. A miss is None.
                let lib = unsafe {
                    Library::open(Some(path.as_str()), libc::RTLD_NOW | libc::RTLD_NOLOAD)
                }
                .ok()?;
                let mut out = [std::ptr::null(); 3];
                for (slot, symbol) in out.iter_mut().zip(symbols) {
                    // SAFETY: reads the export's ADDRESS only; nothing is
                    // called.
                    let addr = *unsafe { lib.get::<*const c_void>(symbol) }.ok()?;
                    if backing_image(addr).as_deref() != Some(path.as_str()) {
                        return None;
                    }
                    *slot = addr;
                }
                // The wallet library never unloads: forgetting the handle leaks
                // one refcount, which is correct (Relim's shape).
                std::mem::forget(lib);
                Some(out)
            }
            AppleChoice::Absent | AppleChoice::Ambiguous => None,
        }
    }
}

#[cfg(all(unix, not(any(target_os = "macos", target_os = "ios"))))]
mod platform {
    //! Android and desktop Linux: the wallet is `libzec_wallet.so`, already
    //! loaded by the Dart side; `RTLD_NOLOAD` opens THAT image or nothing.
    use std::ffi::{CStr, c_void};

    use super::{WALLET_ELF_NAME, elf_image_is_wallet};

    pub(super) fn resolve_symbols(symbols: [&[u8]; 3]) -> Option<[*const c_void; 3]> {
        use libloading::os::unix::Library;
        // SAFETY: already-loaded handle only (`RTLD_NOLOAD`) — never a second
        // copy, never a global/interposed lookup; no initializer runs.
        let lib =
            unsafe { Library::open(Some(WALLET_ELF_NAME), libc::RTLD_NOW | libc::RTLD_NOLOAD) }
                .ok()?;
        let mut out = [std::ptr::null(); 3];
        for (slot, symbol) in out.iter_mut().zip(symbols) {
            // SAFETY: reads the export's ADDRESS only; nothing is called.
            let addr = *unsafe { lib.get::<*const c_void>(symbol) }.ok()?;
            // Anti-interposition: the address must live in the wallet's own
            // image. SAFETY: all-zero is a valid `Dl_info`; `dladdr` fills
            // loader-owned strings.
            let mut info: libc::Dl_info = unsafe { std::mem::zeroed() };
            if unsafe { libc::dladdr(addr, &mut info) } == 0 || info.dli_fname.is_null() {
                return None;
            }
            // SAFETY: non-null, NUL-terminated, loader-owned.
            let fname = unsafe { CStr::from_ptr(info.dli_fname) }.to_string_lossy();
            if !elf_image_is_wallet(&fname) {
                return None;
            }
            *slot = addr;
        }
        std::mem::forget(lib);
        Some(out)
    }
}

#[cfg(windows)]
mod platform {
    //! Windows: the already-loaded `zec_wallet.dll` by module handle — never
    //! `LoadLibrary`, which would load a copy.
    use std::ffi::c_void;

    use super::WALLET_DLL_NAME;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleW(name: *const u16) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
    }

    pub(super) fn resolve_symbols(symbols: [&[u8]; 3]) -> Option<[*const c_void; 3]> {
        let wide: Vec<u16> = WALLET_DLL_NAME
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: a NUL-terminated wide string; returns the handle of an
        // ALREADY-LOADED module or null, and takes no reference.
        let module = unsafe { GetModuleHandleW(wide.as_ptr()) };
        if module.is_null() {
            return None;
        }
        let mut out = [std::ptr::null(); 3];
        for (slot, symbol) in out.iter_mut().zip(symbols) {
            // SAFETY: `symbol` is NUL-terminated; returns the export's
            // address or null.
            let addr = unsafe { GetProcAddress(module, symbol.as_ptr()) };
            if addr.is_null() {
                return None;
            }
            *slot = addr.cast_const();
        }
        Some(out)
    }
}

#[cfg(not(any(unix, windows)))]
mod platform {
    //! No supported loader on this target: the wallet is never found.
    use std::ffi::c_void;

    pub(super) fn resolve_symbols(_symbols: [&[u8]; 3]) -> Option<[*const c_void; 3]> {
        None
    }
}

// ─── The wallet link over the resolved verbs ─────────────────────────────────

/// The production [`WalletLink`]: the three C verbs the resolver found.
pub struct CWalletLink(Verbs);

impl WalletLink for CWalletLink {
    fn register(
        &self,
        dialer: &ZwNetDialerV1,
        descriptor: &ZwTransportDescriptor,
    ) -> Result<Token, i32> {
        let mut token: Token = Zeroizing::new([0u8; ZW_NET_DIALER_AUTH_TOKEN_BYTES]);
        // SAFETY: the verb is the wallet's `register`, resolved by name from
        // the verified image and typed by this module's P14-pinned mirror; the
        // two structs are valid for the call (the wallet COPIES them) and
        // `auth_out` has the header's width.
        let rc = unsafe {
            (self.0.register)(
                ZW_NET_DIALER_ABI_VERSION,
                dialer,
                descriptor,
                token.as_mut_ptr(),
            )
        };
        if rc == ZW_RC_OK { Ok(token) } else { Err(rc) }
    }

    fn notify(&self, token: &Token, descriptor: &ZwTransportDescriptor, retire: bool) -> i32 {
        // SAFETY: as `register`; the token is the one it minted, full width.
        unsafe { (self.0.notify)(token.as_ptr(), descriptor, u32::from(retire)) }
    }

    fn clear(&self, token: &Token) -> i32 {
        // SAFETY: as `register`; NULL dialer and descriptor is the header's
        // CLEAR form.
        unsafe { (self.0.update)(token.as_ptr(), std::ptr::null(), std::ptr::null()) }
    }
}

/// The production [`LinkResolver`]: [`ImageResolver`] each time `init` asks
/// (an `init` after a failed one resolves afresh).
pub struct ImageLinkResolver;

impl LinkResolver for ImageLinkResolver {
    fn resolve(&self) -> Option<Arc<dyn WalletLink>> {
        ImageResolver
            .resolve()
            .map(|verbs| Arc::new(CWalletLink(verbs)) as Arc<dyn WalletLink>)
    }
}

// ─── The vtable the wallet calls ─────────────────────────────────────────────

/// A raw pointer the SDK owns, moved to the plugin's threads. The header's
/// contract is what makes that sound: `sdk_ctx` is valid until its op's
/// completion, and every completion is called from any host thread by design.
#[derive(Clone, Copy)]
struct SdkPtr(*mut c_void);
// SAFETY: see the type's doc — the pointer is only passed back to the SDK's
// own completion function, which the header allows on any thread. The same
// holds for the host's status `ctx` (the plugin header: the callback is
// called "from ANY plugin thread"); the plugin never dereferences either.
unsafe impl Send for SdkPtr {}
// SAFETY: as `Send`: shared references only copy the pointer value out.
unsafe impl Sync for SdkPtr {}

/// The SDK's read buffer, written only by the op's completion, under the
/// plugin's lock, while the op is `Pending` — the header keeps it valid until
/// that completion, and `close` (the same lock) ends every later write to it.
struct SdkReadBuf {
    ptr: *mut u8,
    cap: usize,
}
// SAFETY: see the type's doc; the plugin writes it from exactly one thread at
// a time (the lock) and never after its completion.
unsafe impl Send for SdkReadBuf {}

impl SdkReadBuf {
    fn fill(&self, data: &[u8]) -> usize {
        let n = data.len().min(self.cap);
        if n > 0 {
            // SAFETY: `ptr` is non-null with `cap` writable bytes until this
            // op's completion (the header's buffer rule, checked non-null at
            // the edge); `n <= cap`; the plugin's staging buffer and the SDK's
            // cannot overlap.
            unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), self.ptr, n) };
        }
        n
    }
}

/// `(ptr, len)` as a slice, never `from_raw_parts(NULL, …)`: `None` for a
/// NULL pointer with a non-zero length.
///
/// # Safety
/// A non-null `ptr` must point at `len` readable bytes for the returned
/// lifetime (the header's rule for every span the SDK passes).
unsafe fn span<'a>(ptr: *const u8, len: usize) -> Option<&'a [u8]> {
    if len == 0 {
        Some(&[])
    } else if ptr.is_null() {
        None
    } else {
        // SAFETY: the caller's contract.
        Some(unsafe { std::slice::from_raw_parts(ptr, len) })
    }
}

/// The vtable the plugin installs: `ctx` is the process-lifetime plugin (a
/// `static` in production; the tests keep theirs alive), never freed — a verb
/// still in flight after a clear must not find freed memory (§3.3).
pub fn vtable_for(plugin: &Arc<Plugin>) -> ZwNetDialerV1 {
    ZwNetDialerV1 {
        ctx: Arc::as_ptr(plugin).cast_mut().cast(),
        dial: Some(vt_dial),
        read: Some(vt_read),
        write: Some(vt_write),
        close: Some(vt_close),
    }
}

/// The plugin behind a vtable `ctx`, as the `Arc` the verbs spawn with.
///
/// # Safety
/// `ctx` came from [`vtable_for`] over a plugin that is still alive.
unsafe fn plugin_of(ctx: *mut c_void) -> Arc<Plugin> {
    let ptr = ctx.cast_const().cast::<Plugin>();
    // SAFETY: the caller's contract; the count is raised before the borrow
    // becomes an owned `Arc`, so the plugin's own reference is untouched.
    unsafe {
        Arc::increment_strong_count(ptr);
        Arc::from_raw(ptr)
    }
}

/// A vtable verb's edge: a panic answers REFUSED (the op it may have recorded
/// was removed without a completion — `OpGuard`), never an unwind into C.
fn guarded_code(f: impl FnOnce() -> u32) -> u32 {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or(ZW_DIAL_REFUSED)
}

/// `zw_net_dialer_v1.dial` (§3.3): bounds and UTF-8 first — a violation is
/// the wallet's protocol error, REFUSED synchronously, never a fallback
/// (P18); `key_len == 0` is NO key and its pointer is never read.
///
/// # Safety
/// The header's contract for `dial`.
unsafe extern "C" fn vt_dial(
    ctx: *mut c_void,
    host: *const u8,
    host_len: usize,
    port: u16,
    isolation_key: *const u8,
    isolation_key_len: usize,
    op_id: u64,
    sdk_ctx: *mut c_void,
    complete: ZwDialCompleteFn,
) -> u32 {
    guarded_code(|| {
        if reentrant() || ctx.is_null() {
            return ZW_DIAL_REFUSED;
        }
        if host_len == 0 || host_len > ZW_HOST_NAME_MAX_BYTES {
            return ZW_DIAL_REFUSED;
        }
        if isolation_key_len > ZW_ISOLATION_KEY_MAX_BYTES {
            return ZW_DIAL_REFUSED;
        }
        // SAFETY: the header: `host` has `host_len` bytes for the call.
        let Some(host) = (unsafe { span(host, host_len) }) else {
            return ZW_DIAL_REFUSED;
        };
        let Ok(host) = std::str::from_utf8(host) else {
            return ZW_DIAL_REFUSED;
        };
        let key = if isolation_key_len == 0 {
            None
        } else {
            // SAFETY: the header: the key has `isolation_key_len` bytes.
            let Some(bytes) = (unsafe { span(isolation_key, isolation_key_len) }) else {
                return ZW_DIAL_REFUSED;
            };
            let Ok(key) = std::str::from_utf8(bytes) else {
                return ZW_DIAL_REFUSED;
            };
            Some(key.to_owned())
        };
        let sdk = SdkPtr(sdk_ctx);
        // SAFETY: `ctx` came from `vtable_for` over the live plugin.
        let plugin = unsafe { plugin_of(ctx) };
        plugin.dial(
            host.to_owned(),
            port,
            key,
            Box::new(move |code, stream| {
                let sdk = sdk;
                complete(sdk.0, op_id, code, stream);
            }),
        )
    })
}

/// `zw_net_dialer_v1.read`: the bytes land in a plugin-owned buffer and are
/// copied into the SDK's under the lock, only while the op is live.
///
/// # Safety
/// The header's contract for `read`.
unsafe extern "C" fn vt_read(
    ctx: *mut c_void,
    stream: u64,
    buf: *mut u8,
    cap: usize,
    op_id: u64,
    sdk_ctx: *mut c_void,
    complete: ZwIoCompleteFn,
) -> u32 {
    guarded_code(|| {
        if reentrant() || ctx.is_null() || buf.is_null() || cap == 0 {
            return ZW_DIAL_REFUSED;
        }
        let sdk = SdkPtr(sdk_ctx);
        let target = SdkReadBuf { ptr: buf, cap };
        // SAFETY: `ctx` came from `vtable_for` over the live plugin.
        let plugin = unsafe { plugin_of(ctx) };
        plugin.read(
            stream,
            cap,
            Box::new(move |outcome| {
                let sdk = sdk;
                let target = target;
                match outcome {
                    ReadOutcome::Data(bytes) => {
                        let n = target.fill(bytes);
                        complete(sdk.0, op_id, ZW_DIAL_OK, n);
                    }
                    ReadOutcome::Code(code) => complete(sdk.0, op_id, code, 0),
                }
            }),
        )
    })
}

/// `zw_net_dialer_v1.write`: the SDK's bytes are COPIED here, at accept, and
/// the SDK's buffer is never touched again.
///
/// # Safety
/// The header's contract for `write`.
unsafe extern "C" fn vt_write(
    ctx: *mut c_void,
    stream: u64,
    buf: *const u8,
    len: usize,
    op_id: u64,
    sdk_ctx: *mut c_void,
    complete: ZwIoCompleteFn,
) -> u32 {
    guarded_code(|| {
        if reentrant() || ctx.is_null() {
            return ZW_DIAL_REFUSED;
        }
        // SAFETY: the header: `buf` has `len` bytes until the completion; we
        // copy at most one staging buffer's worth now.
        let Some(bytes) = (unsafe { span(buf, len.min(STAGING_BUFFER_BYTES)) }) else {
            return ZW_DIAL_REFUSED;
        };
        let data = bytes.to_vec();
        let sdk = SdkPtr(sdk_ctx);
        // SAFETY: `ctx` came from `vtable_for` over the live plugin.
        let plugin = unsafe { plugin_of(ctx) };
        plugin.write(
            stream,
            data,
            Box::new(move |code, n| {
                let sdk = sdk;
                complete(sdk.0, op_id, code, n);
            }),
        )
    })
}

/// `zw_net_dialer_v1.close`: idempotent; never unwinds.
///
/// # Safety
/// The header's contract for `close`.
unsafe extern "C" fn vt_close(ctx: *mut c_void, stream: u64) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if ctx.is_null() {
            return;
        }
        // SAFETY: `ctx` came from `vtable_for` over the live plugin.
        let plugin = unsafe { plugin_of(ctx) };
        // Under the plugin's log dispatcher, as every export is (`guarded_rc`):
        // closing drops an arti stream on the wallet core's thread, and its
        // logs must not reach the host's global subscriber (security review
        // H4 of plan §5).
        let dispatch = plugin_dispatch(tracing::dispatcher::get_default(Clone::clone));
        tracing::dispatcher::with_default(&dispatch, || plugin.close(stream));
    }));
}

// ─── The pause clock ─────────────────────────────────────────────────────────

/// The production [`PauseClock`]: a clock that COUNTS device sleep —
/// `CLOCK_BOOTTIME` on Android/Linux, `mach_continuous_time` on Apple,
/// `GetTickCount64` on Windows; anywhere else `None`, so every resume
/// rebuilds (§3.4 "The clock": `Instant` stops while the device sleeps).
pub struct BootClock;

impl PauseClock for BootClock {
    fn now(&self) -> Option<Duration> {
        clock::now()
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
mod clock {
    use std::time::Duration;

    pub(super) fn now() -> Option<Duration> {
        let mut ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: a valid out-pointer to a zeroed timespec.
        let rc = unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut ts) };
        (rc == 0).then(|| {
            Duration::new(
                u64::try_from(ts.tv_sec).unwrap_or(0),
                u32::try_from(ts.tv_nsec).unwrap_or(0),
            )
        })
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod clock {
    use std::time::Duration;

    #[repr(C)]
    struct Timebase {
        numer: u32,
        denom: u32,
    }

    unsafe extern "C" {
        fn mach_continuous_time() -> u64;
        fn mach_timebase_info(info: *mut Timebase) -> i32;
    }

    pub(super) fn now() -> Option<Duration> {
        let mut tb = Timebase { numer: 0, denom: 0 };
        // SAFETY: a valid out-pointer; argless read of the continuous clock.
        let (rc, ticks) = unsafe { (mach_timebase_info(&mut tb), mach_continuous_time()) };
        (rc == 0 && tb.denom != 0).then(|| {
            let nanos = u128::from(ticks) * u128::from(tb.numer) / u128::from(tb.denom);
            Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
        })
    }
}

#[cfg(windows)]
mod clock {
    use std::time::Duration;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetTickCount64() -> u64;
    }

    pub(super) fn now() -> Option<Duration> {
        // SAFETY: argless; counts milliseconds including sleep.
        Some(Duration::from_millis(unsafe { GetTickCount64() }))
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios",
    windows
)))]
mod clock {
    pub(super) fn now() -> Option<std::time::Duration> {
        None
    }
}

// ─── The process plugin and the nine exports ─────────────────────────────────

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
static PLUGIN: OnceLock<Arc<Plugin>> = OnceLock::new();

/// The plugin's own runtime (§3.3 "Threading"): built on the first `init`,
/// held for the process, every thread under the plugin's log dispatcher.
fn runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(PLUGIN_RUNTIME_WORKERS)
            .thread_name("zec-wallet-tor")
            .enable_all()
            .on_thread_start(|| {
                let inner = tracing::dispatcher::get_default(Clone::clone);
                // Held for the thread's life: the guard is leaked on purpose.
                std::mem::forget(tracing::dispatcher::set_default(&plugin_dispatch(inner)));
            })
            .build()
            .expect("the plugin's runtime builds (threads and a timer)")
    })
}

/// The process plugin, built on first use.
fn process_plugin() -> &'static Arc<Plugin> {
    PLUGIN.get_or_init(|| {
        Plugin::new(
            Deps {
                factory: Arc::new(ArtiFactory),
                resolver: Box::new(ImageLinkResolver),
                clock: Box::new(BootClock),
                jitter: Box::new(RandomJitter::default()),
            },
            runtime().handle().clone(),
        )
    })
}

/// An export's edge: the plugin's log dispatcher around the call, and a
/// panic answers `ZWT_RC_PANICKED` — never an unwind into the host.
fn guarded_rc(f: impl FnOnce() -> i32) -> i32 {
    let dispatch = plugin_dispatch(tracing::dispatcher::get_default(Clone::clone));
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tracing::dispatcher::with_default(&dispatch, f)
    }))
    .unwrap_or(ZWT_RC_PANICKED)
}

/// A pasted bridge list from its span, as the ONE owned copy — zeroized on
/// drop, and on a refusal too (the security angle's A9). `Err(())` =
/// not UTF-8.
///
/// # Safety
/// As [`span`].
unsafe fn bridges_from_span(ptr: *const u8, len: usize) -> Result<Option<Zeroizing<String>>, ()> {
    if len == 0 {
        return Ok(None);
    }
    // SAFETY: the caller's contract.
    let bytes = unsafe { span(ptr, len) }.ok_or(())?;
    match String::from_utf8(bytes.to_vec()) {
        Ok(text) => Ok(Some(Zeroizing::new(text))),
        Err(e) => {
            let mut raw = e.into_bytes();
            raw.zeroize();
            Err(())
        }
    }
}

/// The host's status callback as a [`StatusSink`].
fn status_sink(on_status: ZwtStatusFn, ctx: *mut c_void) -> StatusSink {
    let ctx = SdkPtr(ctx);
    Box::new(move |status| {
        let ctx = ctx;
        let raw = ZwtStatus::from(status);
        on_status(ctx.0, &raw);
    })
}

/// `zec_wallet_tor_init` — see the header.
///
/// # Safety
/// `config` is NULL or a valid `zwt_config` whose spans are readable for the
/// call; `ctx` is passed back to `on_status` untouched.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn zec_wallet_tor_init(
    config: *const ZwtConfig,
    on_status: Option<ZwtStatusFn>,
    ctx: *mut c_void,
) -> i32 {
    guarded_rc(|| {
        // SAFETY: NULL-checked; the caller's contract.
        let Some(config) = (unsafe { config.as_ref() }) else {
            return ZWT_RC_NULL_ARG;
        };
        // SAFETY: the header: `tor_dir` spans `tor_dir_len` bytes.
        let Some(dir) = (unsafe { span(config.tor_dir, config.tor_dir_len) }) else {
            return ZWT_RC_NULL_ARG;
        };
        let Ok(dir) = std::str::from_utf8(dir) else {
            return ZWT_RC_INVALID_DATA_DIR;
        };
        // SAFETY: the header: `bridges` spans `bridges_len` bytes.
        let Ok(bridges) = (unsafe { bridges_from_span(config.bridges, config.bridges_len) }) else {
            return ZWT_RC_BRIDGES_REFUSED;
        };
        let sink = on_status.map(|f| status_sink(f, ctx));
        process_plugin().init(dir, bridges, sink)
    })
}

/// `zec_wallet_tor_status` — see the header.
///
/// # Safety
/// `out` is NULL or valid for one `zwt_status` write.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn zec_wallet_tor_status(out: *mut ZwtStatus) -> i32 {
    guarded_rc(|| {
        if out.is_null() {
            return ZWT_RC_NULL_ARG;
        }
        let Some(plugin) = PLUGIN.get() else {
            return ZWT_RC_NOT_INITIALIZED;
        };
        let status = ZwtStatus::from(&plugin.status());
        // SAFETY: non-null, and the caller's contract.
        unsafe { out.write(status) };
        ZWT_RC_OK
    })
}

/// `zec_wallet_tor_set_bridges` — see the header.
///
/// # Safety
/// `(bridges, bridges_len)` is a readable span, or `(NULL, 0)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn zec_wallet_tor_set_bridges(bridges: *const u8, bridges_len: usize) -> i32 {
    guarded_rc(|| {
        let Some(plugin) = PLUGIN.get() else {
            return ZWT_RC_NOT_INITIALIZED;
        };
        // SAFETY: the caller's contract.
        let Ok(bridges) = (unsafe { bridges_from_span(bridges, bridges_len) }) else {
            return ZWT_RC_BRIDGES_REFUSED;
        };
        plugin.set_bridges(bridges)
    })
}

/// `zec_wallet_tor_retry_bootstrap` — see the header.
#[unsafe(no_mangle)]
pub extern "C" fn zec_wallet_tor_retry_bootstrap() -> i32 {
    guarded_rc(|| {
        PLUGIN
            .get()
            .map_or(ZWT_RC_NOT_INITIALIZED, |p| p.retry_bootstrap())
    })
}

/// `zec_wallet_tor_clear_state` — see the header. Works with no `init` in the
/// process (after a restart), which is why it takes the directory.
///
/// # Safety
/// `(tor_dir, tor_dir_len)` is a readable span.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn zec_wallet_tor_clear_state(tor_dir: *const u8, tor_dir_len: usize) -> i32 {
    guarded_rc(|| {
        // SAFETY: the caller's contract.
        let Some(dir) = (unsafe { span(tor_dir, tor_dir_len) }) else {
            return ZWT_RC_NULL_ARG;
        };
        let Ok(dir) = std::str::from_utf8(dir) else {
            return ZWT_RC_INVALID_DATA_DIR;
        };
        match PLUGIN.get() {
            Some(plugin) => plugin.clear_state(dir),
            None => remove_tor_state(dir),
        }
    })
}

/// `zec_wallet_tor_dispose` — see the header.
#[unsafe(no_mangle)]
pub extern "C" fn zec_wallet_tor_dispose() -> i32 {
    guarded_rc(|| PLUGIN.get().map_or(ZWT_RC_NOT_INITIALIZED, |p| p.dispose()))
}

/// `zec_wallet_tor_on_paused` — see the header.
#[unsafe(no_mangle)]
pub extern "C" fn zec_wallet_tor_on_paused() -> i32 {
    guarded_rc(|| {
        PLUGIN
            .get()
            .map_or(ZWT_RC_NOT_INITIALIZED, |p| p.on_paused())
    })
}

/// `zec_wallet_tor_on_resumed` — see the header.
#[unsafe(no_mangle)]
pub extern "C" fn zec_wallet_tor_on_resumed() -> i32 {
    guarded_rc(|| {
        PLUGIN
            .get()
            .map_or(ZWT_RC_NOT_INITIALIZED, |p| p.on_resumed())
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::mem::{align_of, offset_of, size_of};

    use super::*;

    /// Every `#define NAME value` with an integer value in a header, the `u`
    /// suffix stripped — T15's reader, the wallet bridge's shape.
    fn defines(header: &str) -> HashMap<String, i64> {
        let mut out = HashMap::new();
        for line in header.lines() {
            let Some(rest) = line.trim().strip_prefix("#define ") else {
                continue;
            };
            let mut parts = rest.split_whitespace();
            let (Some(name), Some(raw)) = (parts.next(), parts.next()) else {
                continue;
            };
            if let Ok(v) = raw.trim_end_matches('u').parse::<i64>() {
                out.insert(name.to_string(), v);
            }
        }
        out
    }

    fn get(h: &HashMap<String, i64>, name: &str) -> i64 {
        *h.get(name)
            .unwrap_or_else(|| panic!("the header defines {name} with a value"))
    }

    fn count(h: &HashMap<String, i64>, prefix: &str) -> usize {
        h.keys().filter(|k| k.starts_with(prefix)).count()
    }

    /// FR-5 spec §8 **P14, the wallet half**: the plugin's independent mirror
    /// of `zec_wallet_net_dialer.h` equals the header — the version (3), the
    /// token width, the four bounds, the seven `ZW_RC_*`, the six `ZW_DIAL_*`,
    /// the three value sets (isolation, exposure, health), each set CLOSED on
    /// the header side (nothing the plugin does not know), and the
    /// descriptor's and the vtable's layouts. Parsed from the wallet's own
    /// header by path, so a drift fails here rather than on a device.
    #[test]
    fn the_plugins_abi_mirror_agrees_with_the_wallet_header() {
        let h = defines(include_str!(
            "../../../zec_wallet/rust/include/zec_wallet_net_dialer.h"
        ));
        assert_eq!(
            get(&h, "ZW_NET_DIALER_ABI_VERSION"),
            i64::from(ZW_NET_DIALER_ABI_VERSION)
        );
        assert_eq!(
            get(&h, "ZW_NET_DIALER_AUTH_TOKEN_BYTES"),
            ZW_NET_DIALER_AUTH_TOKEN_BYTES as i64
        );
        assert_eq!(
            get(&h, "ZW_ISOLATION_KEY_MAX_BYTES"),
            ZW_ISOLATION_KEY_MAX_BYTES as i64
        );
        assert_eq!(
            get(&h, "ZW_HOST_NAME_MAX_BYTES"),
            ZW_HOST_NAME_MAX_BYTES as i64
        );
        assert_eq!(
            get(&h, "ZW_TRANSPORT_NAME_MAX_BYTES"),
            ZW_TRANSPORT_NAME_MAX_BYTES as i64
        );

        let sets: [(&str, &[(&str, i64)]); 5] = [
            (
                "ZW_RC_",
                &[
                    ("ZW_RC_OK", i64::from(ZW_RC_OK)),
                    ("ZW_RC_POISONED", i64::from(ZW_RC_POISONED)),
                    ("ZW_RC_OCCUPIED", i64::from(ZW_RC_OCCUPIED)),
                    ("ZW_RC_NULL_ARG", i64::from(ZW_RC_NULL_ARG)),
                    ("ZW_RC_PANICKED", i64::from(ZW_RC_PANICKED)),
                    ("ZW_RC_ABI", i64::from(ZW_RC_ABI)),
                    ("ZW_RC_DESCRIPTOR", i64::from(ZW_RC_DESCRIPTOR)),
                ],
            ),
            (
                "ZW_DIAL_",
                &[
                    ("ZW_DIAL_OK", i64::from(ZW_DIAL_OK)),
                    ("ZW_DIAL_NOT_READY", i64::from(ZW_DIAL_NOT_READY)),
                    ("ZW_DIAL_UNREACHABLE", i64::from(ZW_DIAL_UNREACHABLE)),
                    ("ZW_DIAL_TIMEOUT", i64::from(ZW_DIAL_TIMEOUT)),
                    ("ZW_DIAL_REFUSED", i64::from(ZW_DIAL_REFUSED)),
                    ("ZW_DIAL_RETIRED", i64::from(ZW_DIAL_RETIRED)),
                ],
            ),
            (
                "ZW_ISOLATION_",
                &[
                    ("ZW_ISOLATION_UNKNOWN", i64::from(ZW_ISOLATION_UNKNOWN)),
                    ("ZW_ISOLATION_SUPPORTED", i64::from(ZW_ISOLATION_SUPPORTED)),
                    (
                        "ZW_ISOLATION_UNSUPPORTED",
                        i64::from(ZW_ISOLATION_UNSUPPORTED),
                    ),
                    // the bound shares the prefix; it is pinned above
                    (
                        "ZW_ISOLATION_KEY_MAX_BYTES",
                        ZW_ISOLATION_KEY_MAX_BYTES as i64,
                    ),
                ],
            ),
            (
                "ZW_EXPOSURE_",
                &[
                    ("ZW_EXPOSURE_UNKNOWN", i64::from(ZW_EXPOSURE_UNKNOWN)),
                    ("ZW_EXPOSURE_HIDDEN", i64::from(ZW_EXPOSURE_HIDDEN)),
                    ("ZW_EXPOSURE_EXPOSED", i64::from(ZW_EXPOSURE_EXPOSED)),
                ],
            ),
            (
                "ZW_HEALTH_",
                &[
                    ("ZW_HEALTH_STARTING", i64::from(ZW_HEALTH_STARTING)),
                    ("ZW_HEALTH_READY", i64::from(ZW_HEALTH_READY)),
                    ("ZW_HEALTH_FAILED", i64::from(ZW_HEALTH_FAILED)),
                ],
            ),
        ];
        for (prefix, members) in sets {
            for (name, value) in members {
                assert_eq!(get(&h, name), *value, "{name}");
            }
            assert_eq!(
                count(&h, prefix),
                members.len(),
                "the wallet header's {prefix}* set has a member the plugin's mirror does not"
            );
        }

        // The descriptor: 32 name bytes then five u32s, 52 bytes, 4-aligned,
        // `health` APPENDED at 48 — the same numbers the wallet's T15 pins.
        assert_eq!(size_of::<ZwTransportDescriptor>(), 52);
        assert_eq!(align_of::<ZwTransportDescriptor>(), 4);
        assert_eq!(offset_of!(ZwTransportDescriptor, name), 0);
        assert_eq!(offset_of!(ZwTransportDescriptor, name_len), 32);
        assert_eq!(offset_of!(ZwTransportDescriptor, readiness), 36);
        assert_eq!(offset_of!(ZwTransportDescriptor, isolation), 40);
        assert_eq!(offset_of!(ZwTransportDescriptor, exposure), 44);
        assert_eq!(offset_of!(ZwTransportDescriptor, health), 48);
        // The vtable: `ctx` then four function pointers, pointer-aligned.
        let p = size_of::<*const c_void>();
        assert_eq!(size_of::<ZwNetDialerV1>(), 5 * p);
        assert_eq!(offset_of!(ZwNetDialerV1, dial), p);
        assert_eq!(offset_of!(ZwNetDialerV1, close), 4 * p);
        // The three verb names are the header's.
        let header = include_str!("../../../zec_wallet/rust/include/zec_wallet_net_dialer.h");
        for symbol in [REGISTER_SYMBOL, UPDATE_SYMBOL, NOTIFY_SYMBOL] {
            let name = std::str::from_utf8(&symbol[..symbol.len() - 1]).expect("ASCII");
            assert!(
                header.contains(&format!("{name}(")),
                "the header declares {name}"
            );
        }
    }

    /// FR-5 spec §8 **P14, the plugin half**: `include/zec_wallet_tor.h` and
    /// this module agree — every `ZWT_*` value, each set closed, `zwt_status`'s
    /// and `zwt_config`'s layouts — and the codes the typed status maps to are
    /// exactly the header's sets (every phase, blockage and class has a
    /// distinct code; nothing maps outside them).
    #[test]
    fn the_plugins_own_header_agrees_with_its_rust() {
        let h = defines(include_str!("../include/zec_wallet_tor.h"));
        assert_eq!(get(&h, "ZWT_ABI_VERSION"), i64::from(ZWT_ABI_VERSION));
        assert_eq!(zec_wallet_tor_abi_version(), ZWT_ABI_VERSION);

        let rc = [
            ("ZWT_RC_OK", ZWT_RC_OK),
            ("ZWT_RC_WALLET_NOT_LOADED", ZWT_RC_WALLET_NOT_LOADED),
            ("ZWT_RC_ABI_MISMATCH", ZWT_RC_ABI_MISMATCH),
            ("ZWT_RC_SLOT_OCCUPIED", ZWT_RC_SLOT_OCCUPIED),
            ("ZWT_RC_REGISTRY_POISONED", ZWT_RC_REGISTRY_POISONED),
            ("ZWT_RC_DESCRIPTOR_REFUSED", ZWT_RC_DESCRIPTOR_REFUSED),
            ("ZWT_RC_INVALID_DATA_DIR", ZWT_RC_INVALID_DATA_DIR),
            ("ZWT_RC_BRIDGES_REFUSED", ZWT_RC_BRIDGES_REFUSED),
            ("ZWT_RC_ENGINE_SETUP", ZWT_RC_ENGINE_SETUP),
            ("ZWT_RC_NOT_INITIALIZED", ZWT_RC_NOT_INITIALIZED),
            ("ZWT_RC_DISPOSED", ZWT_RC_DISPOSED),
            ("ZWT_RC_NULL_ARG", ZWT_RC_NULL_ARG),
            ("ZWT_RC_PANICKED", ZWT_RC_PANICKED),
            ("ZWT_RC_STOPPING", ZWT_RC_STOPPING),
            ("ZWT_RC_RESTART_REQUIRED", ZWT_RC_RESTART_REQUIRED),
        ];
        for (name, value) in rc {
            assert_eq!(get(&h, name), i64::from(value), "{name}");
        }
        assert_eq!(count(&h, "ZWT_RC_"), rc.len(), "ZWT_RC_* is closed");

        let phases = [
            (Phase::Idle, "ZWT_PHASE_IDLE"),
            (Phase::Bootstrapping, "ZWT_PHASE_BOOTSTRAPPING"),
            (Phase::Ready, "ZWT_PHASE_READY"),
            (Phase::Failed, "ZWT_PHASE_FAILED"),
            (Phase::Suspended, "ZWT_PHASE_SUSPENDED"),
            (Phase::NotRegistered, "ZWT_PHASE_NOT_REGISTERED"),
        ];
        for (phase, name) in phases {
            assert_eq!(get(&h, name), i64::from(phase_code(phase)), "{name}");
        }
        assert_eq!(
            count(&h, "ZWT_PHASE_"),
            phases.len(),
            "ZWT_PHASE_* is closed"
        );

        let blockage_names = [
            "ZWT_BLOCKAGE_DISABLED",
            "ZWT_BLOCKAGE_OFFLINE",
            "ZWT_BLOCKAGE_FILTERING",
            "ZWT_BLOCKAGE_CANT_REACH_TOR",
            "ZWT_BLOCKAGE_CLOCK_SKEWED",
            "ZWT_BLOCKAGE_CANT_BOOTSTRAP",
            "ZWT_BLOCKAGE_UNKNOWN",
        ];
        assert_eq!(get(&h, "ZWT_BLOCKAGE_NONE"), i64::from(ZWT_BLOCKAGE_NONE));
        assert_eq!(blockage_code(None), ZWT_BLOCKAGE_NONE);
        assert_eq!(blockage_names.len(), Blockage::ALL.len());
        for (b, name) in Blockage::ALL.iter().zip(blockage_names) {
            assert_eq!(get(&h, name), i64::from(blockage_code(Some(*b))), "{name}");
        }
        assert_eq!(
            count(&h, "ZWT_BLOCKAGE_"),
            Blockage::ALL.len() + 1,
            "ZWT_BLOCKAGE_* is closed"
        );

        let class_names = [
            "ZWT_CLASS_BOOTSTRAP_DEADLINE",
            "ZWT_CLASS_NOT_REGISTERED",
            "ZWT_CLASS_BOOTSTRAP_FAILED",
            "ZWT_CLASS_SETUP",
            "ZWT_CLASS_NOT_BOOTSTRAPPED",
            "ZWT_CLASS_BRIDGE_CONFIG_TOO_LONG",
            "ZWT_CLASS_BRIDGE_TOO_MANY_LINES",
            "ZWT_CLASS_BRIDGE_LINE_TOO_LONG",
            "ZWT_CLASS_BRIDGE_PT_UNSUPPORTED",
            "ZWT_CLASS_BRIDGE_UNUSABLE",
            "ZWT_CLASS_BRIDGE_LINE_EMPTY",
            "ZWT_CLASS_BRIDGE_INVALID_TRANSPORT_OR_ADDRESS",
            "ZWT_CLASS_BRIDGE_INVALID_ADDRESS",
            "ZWT_CLASS_BRIDGE_INVALID_IDENTITY",
            "ZWT_CLASS_BRIDGE_DUPLICATE_IDENTITY",
            "ZWT_CLASS_BRIDGE_UNSUPPORTED_IDENTITY_TYPE",
            "ZWT_CLASS_BRIDGE_UNSUPPORTED_CHANNEL_METHOD",
            "ZWT_CLASS_BRIDGE_DIRECT_PARAMETERS_NOT_ALLOWED",
            "ZWT_CLASS_BRIDGE_NO_RSA_IDENTITY",
            "ZWT_CLASS_BRIDGE_SUPPORT_DISABLED",
            "ZWT_CLASS_CIRCUITS_FAILING",
        ];
        assert_eq!(get(&h, "ZWT_CLASS_NONE"), i64::from(ZWT_CLASS_NONE));
        assert_eq!(class_code(None), ZWT_CLASS_NONE);
        assert_eq!(class_names.len(), FAILURE_CLASSES.len());
        for (class, name) in FAILURE_CLASSES.iter().zip(class_names) {
            assert_eq!(get(&h, name), i64::from(class_code(Some(class))), "{name}");
        }
        assert_eq!(
            count(&h, "ZWT_CLASS_"),
            FAILURE_CLASSES.len() + 1,
            "ZWT_CLASS_* is closed"
        );
        assert_eq!(class_code(Some("not-a-class")), ZWT_CLASS_NONE);

        // `zwt_status`: four u32s, 16 bytes; `zwt_config`: two (ptr, len)
        // spans.
        assert_eq!(size_of::<ZwtStatus>(), 16);
        assert_eq!(offset_of!(ZwtStatus, phase), 0);
        assert_eq!(offset_of!(ZwtStatus, readiness), 4);
        assert_eq!(offset_of!(ZwtStatus, blockage), 8);
        assert_eq!(offset_of!(ZwtStatus, failure_class), 12);
        let p = size_of::<usize>();
        assert_eq!(size_of::<ZwtConfig>(), 4 * p);
        assert_eq!(offset_of!(ZwtConfig, tor_dir_len), p);
        assert_eq!(offset_of!(ZwtConfig, bridges), 2 * p);
        assert_eq!(offset_of!(ZwtConfig, bridges_len), 3 * p);
    }

    /// The Apple choice is the spec's rule (§3.2): one in-bundle framework, or
    /// the main executable, never both and never two frameworks.
    #[test]
    fn the_apple_resolver_takes_one_wallet_image_and_refuses_ambiguity() {
        let main = "/var/containers/Bundle/Application/U/Runner.app/Runner";
        let fw = "/var/containers/Bundle/Application/U/Runner.app/Frameworks/zec_wallet.framework/zec_wallet";
        assert!(apple_image_is_wallet_framework(fw, main));
        // A decoy of the right SHAPE outside the bundle is not the wallet.
        assert!(!apple_image_is_wallet_framework(
            "/tmp/x/zec_wallet.framework/zec_wallet",
            main
        ));
        // Wrong basename, and a component look-alike.
        assert!(!apple_image_is_wallet_framework(
            "/var/containers/Bundle/Application/U/Runner.app/Frameworks/zec_wallet.framework/zec_wallet_evil",
            main
        ));
        assert!(!apple_image_is_wallet_framework(
            "/var/containers/Bundle/Application/U/Runner.app/Frameworks/zec_wallet_tor.framework/zec_wallet",
            main
        ));
        // Outside any `.app`, nothing is in-bundle: fail closed.
        assert!(!apple_image_is_wallet_framework(
            fw,
            "/usr/local/bin/test-binary"
        ));
        // macOS: the innermost `.app/` is the residency region.
        assert_eq!(
            bundle_root_of("/Applications/X.app/Contents/MacOS/X"),
            Some("/Applications/X.app/")
        );

        let one = vec![fw.to_string()];
        let two = vec![fw.to_string(), fw.to_string()];
        assert_eq!(
            apple_choose(&one, false),
            AppleChoice::Framework(fw.to_string())
        );
        assert_eq!(apple_choose(&[], true), AppleChoice::MainExecutable);
        assert_eq!(apple_choose(&[], false), AppleChoice::Absent);
        assert_eq!(apple_choose(&one, true), AppleChoice::Ambiguous);
        assert_eq!(apple_choose(&two, false), AppleChoice::Ambiguous);
    }

    /// ELF: an exact basename, never a substring.
    #[test]
    fn the_elf_resolver_accepts_only_the_wallets_own_basename() {
        assert!(elf_image_is_wallet(
            "/data/app/x/lib/arm64/libzec_wallet.so"
        ));
        assert!(!elf_image_is_wallet(
            "/data/app/x/lib/arm64/libzec_wallet_tor.so"
        ));
        assert!(!elf_image_is_wallet("/x/libzec_wallet.so.evil"));
        // A prefix decoy: an `ends_with` check would take it.
        assert!(!elf_image_is_wallet("/x/notlibzec_wallet.so"));
        assert!(!elf_image_is_wallet("/libzec_wallet.so/other.so"));
    }

    /// The resolver answers `None` in a process with no wallet image — the
    /// `walletNotLoaded` path, which is also what a host test binary is.
    #[test]
    fn without_the_wallet_image_the_verbs_do_not_resolve() {
        assert!(ImageResolver.resolve().is_none());
    }
}
