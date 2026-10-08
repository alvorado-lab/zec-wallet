//! The in-process FAKE HOST for the FR-29 crossing (spec
//! `docs/specs/host-transport-crossing.md` §8): a `zw_net_dialer_v1`-shaped C
//! vtable over plain loopback TCP, driven by its OWN tokio runtime, so the
//! two-runtime crossing is proven without Relim — every completion arrives on
//! a thread the SDK's test runtime does not own.
//!
//! NOT a cargo test target on purpose. Cargo autodiscovers `tests/*.rs` and
//! `tests/*/main.rs`; a `tests/<dir>/mod.rs` is a plain helper module, which
//! is what this is: `src/net_dialer_cabi.rs` includes it by `#[path]` under
//! `cfg(test)`, because the registry-touching tests need that module's
//! `cfg(test)` reset seam, and a `tests/host_dialer_fake.rs` target would be an
//! orphan under `every_sdk_test_target_is_named_by_a_live_carrier` (the
//! nightly's `supply_chain_policy`). The shipped library carries no test seam.
//!
//! The vtable and descriptor structs are declared HERE, independently of the
//! SDK's `#[repr(C)]` types, and handed across as raw pointers exactly as a C
//! host would — so a layout disagreement between the two sides fails the tests
//! instead of being papered over by sharing one Rust type.
//!
//! What the fake can do, per the tests that need it: complete INLINE before
//! the verb returns (`DialMode::ConnectInline`, `IoMode::Inline`; T23), refuse
//! synchronously with a code (`DialMode::RefuseSync`), report a chosen dial
//! code, a zero handle or an unknown code (`DialMode::Complete`,
//! `DialMode::ZeroHandle`; T17), over-report a read (`IoMode::OverReport`;
//! T17), park completions and release them later from any thread
//! (`IoMode::Hold`, `DialMode::Hold` + [`FakeHost::release_held`]; T18, T25,
//! FR-39's close), and record the isolation-key bytes it received (T8).

use std::collections::HashMap;
use std::ffi::c_void;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::Notify;

/// `ZW_DIAL_*` as the header spells them — the fake speaks the C table, not
/// the SDK's enum, so a drift between the two is visible here.
pub const ZW_DIAL_OK: u32 = 0;
pub const ZW_DIAL_UNREACHABLE: u32 = 2;
pub const ZW_DIAL_TIMEOUT: u32 = 3;
pub const ZW_DIAL_RETIRED: u32 = 5;

/// The header's `ZW_HEALTH_*` (ABI v3, ADR-0549), declared independently of
/// the SDK's copy exactly as the dial codes above are.
pub const ZW_HEALTH_STARTING: u32 = 0;
pub const ZW_HEALTH_READY: u32 = 1;
pub const ZW_HEALTH_FAILED: u32 = 2;

/// The header's completion typedefs (`zw_dial_complete_fn`, `zw_io_complete_fn`).
pub type DialCompleteFn = extern "C" fn(*mut c_void, u64, u32, u64);
pub type IoCompleteFn = extern "C" fn(*mut c_void, u64, u32, usize);

/// `zw_transport_descriptor` (ABI v3, ADR-0547 + ADR-0549), declared
/// independently of the SDK's copy: the host's own name for its transport in a
/// fixed 32-byte field with its byte length, then the four closed values.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Descriptor {
    pub name: [u8; 32],
    pub name_len: u32,
    pub readiness: u32,
    pub isolation: u32,
    pub exposure: u32,
    pub health: u32,
}

impl Descriptor {
    /// Build a descriptor as a C host would: `name`'s bytes copied into the
    /// fixed field (at most 32 of them) and `name_len` set to `name`'s FULL
    /// byte length — so a 33-byte `name` yields the over-long `name_len` the
    /// SDK must refuse without indexing past the array (T27).
    pub const fn new(name: &[u8], readiness: u32, isolation: u32, exposure: u32) -> Self {
        // The health a registrant that has NOT failed pushes, derived from the
        // readiness so every existing row keeps its meaning; `with_health`
        // spells the axis out where a test is about it.
        let health = if readiness >= 100 {
            ZW_HEALTH_READY
        } else {
            ZW_HEALTH_STARTING
        };
        Self::with_health(name, readiness, isolation, exposure, health)
    }

    /// As [`Self::new`], with the `health` value stated (ABI v3, ADR-0549).
    pub const fn with_health(
        name: &[u8],
        readiness: u32,
        isolation: u32,
        exposure: u32,
        health: u32,
    ) -> Self {
        let mut bytes = [0u8; 32];
        let mut i = 0;
        while i < name.len() && i < 32 {
            bytes[i] = name[i];
            i += 1;
        }
        Self {
            name: bytes,
            name_len: name.len() as u32,
            readiness,
            isolation,
            exposure,
            health,
        }
    }
}

type DialFn = unsafe extern "C" fn(
    *mut c_void,
    *const u8,
    usize,
    u16,
    *const u8,
    usize,
    u64,
    *mut c_void,
    DialCompleteFn,
) -> u32;
type ReadFn =
    unsafe extern "C" fn(*mut c_void, u64, *mut u8, usize, u64, *mut c_void, IoCompleteFn) -> u32;
type WriteFn =
    unsafe extern "C" fn(*mut c_void, u64, *const u8, usize, u64, *mut c_void, IoCompleteFn) -> u32;
type CloseFn = unsafe extern "C" fn(*mut c_void, u64);

/// `zw_net_dialer_v1`, declared independently of the SDK's copy.
#[repr(C)]
pub struct VTable {
    pub ctx: *mut c_void,
    pub dial: Option<DialFn>,
    pub read: Option<ReadFn>,
    pub write: Option<WriteFn>,
    pub close: Option<CloseFn>,
}

/// How the fake answers `dial`.
#[derive(Clone, Copy, Debug)]
pub enum DialMode {
    /// Connect on the fake's runtime; complete on one of ITS threads.
    Connect,
    /// Connect synchronously and complete INLINE, before `dial` returns (T23).
    ConnectInline,
    /// Return the code synchronously; no completion follows.
    RefuseSync(u32),
    /// Accept, then complete with this (failure) code on a fake thread — an
    /// unknown code models a protocol violation (T17).
    Complete(u32),
    /// Accept, then complete `ZW_DIAL_OK` with stream handle 0 (T17).
    ZeroHandle,
    /// Connect synchronously, but PARK the `ZW_DIAL_OK` completion (with the
    /// live stream's handle) until [`FakeHost::release_held`] — a dial still
    /// in flight when the wallet closes (FR-39).
    Hold,
}

/// How the fake completes `read`/`write`.
#[derive(Clone, Copy, Debug)]
pub enum IoMode {
    /// Real I/O on the fake's runtime; completion on one of ITS threads.
    Async,
    /// Real I/O, completed INLINE before the verb returns (T23).
    Inline,
    /// Real I/O, but the completion is PARKED until [`FakeHost::release_held`]
    /// (T18, T25). A parked op is no longer "outstanding": `close` does not
    /// retire it — the release is the one completion it gets.
    Hold,
    /// Complete a read with `n = cap + 1`, whatever was actually read (T17):
    /// the SDK must trust no byte of it.
    OverReport,
}

/// What the fake saw at one `dial` call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DialRecord {
    pub host: String,
    pub port: u16,
    /// `None` when the SDK passed a null key with length 0.
    pub isolation_key: Option<Vec<u8>>,
}

/// One accepted op the fake owes exactly one completion for.
struct Op {
    op_id: u64,
    sdk_ctx: usize,
    complete: IoCompleteFn,
    done: AtomicBool,
}

struct Held {
    op: Arc<Op>,
    code: u32,
    n: usize,
}

/// A `DialMode::Hold` dial: its `ZW_DIAL_OK` completion, parked.
struct HeldDial {
    sdk_ctx: usize,
    op_id: u64,
    stream: u64,
    complete: DialCompleteFn,
}

struct StreamEntry {
    read: tokio::sync::Mutex<OwnedReadHalf>,
    write: tokio::sync::Mutex<OwnedWriteHalf>,
    closed: AtomicBool,
    notify: Notify,
    outstanding: Mutex<Vec<Arc<Op>>>,
}

struct Inner {
    runtime: Mutex<Option<tokio::runtime::Runtime>>,
    handle: tokio::runtime::Handle,
    dial_mode: Mutex<DialMode>,
    io_mode: Mutex<IoMode>,
    next_handle: AtomicU64,
    streams: Mutex<HashMap<u64, Arc<StreamEntry>>>,
    dials: Mutex<Vec<DialRecord>>,
    held: Mutex<Vec<Held>>,
    held_dials: Mutex<Vec<HeldDial>>,
    completions: AtomicUsize,
    reads: AtomicUsize,
    writes: AtomicUsize,
    closes: AtomicUsize,
}

/// A raw pointer that crosses into a fake-runtime task. The SDK's contract
/// keeps the buffer valid until the completion the fake delivers for it.
#[derive(Clone, Copy)]
struct SendPtr(usize);

/// The fake host. Its address is the vtable's `ctx`; keep it alive while it
/// is registered (the tests reset the registry before dropping one).
pub struct FakeHost {
    inner: Arc<Inner>,
}

impl Default for FakeHost {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeHost {
    pub fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("fake-host")
            .enable_io()
            .enable_time()
            .build()
            .expect("the fake host's runtime builds");
        let handle = runtime.handle().clone();
        Self {
            inner: Arc::new(Inner {
                runtime: Mutex::new(Some(runtime)),
                handle,
                dial_mode: Mutex::new(DialMode::Connect),
                io_mode: Mutex::new(IoMode::Async),
                next_handle: AtomicU64::new(1),
                streams: Mutex::new(HashMap::new()),
                dials: Mutex::new(Vec::new()),
                held: Mutex::new(Vec::new()),
                held_dials: Mutex::new(Vec::new()),
                completions: AtomicUsize::new(0),
                reads: AtomicUsize::new(0),
                writes: AtomicUsize::new(0),
                closes: AtomicUsize::new(0),
            }),
        }
    }

    /// The `zw_net_dialer_v1` a host would hand to `zec_wallet_register_net_dialer`.
    pub fn vtable(&self) -> VTable {
        VTable {
            ctx: Arc::as_ptr(&self.inner) as *mut c_void,
            dial: Some(fake_dial),
            read: Some(fake_read),
            write: Some(fake_write),
            close: Some(fake_close),
        }
    }

    pub fn set_dial_mode(&self, mode: DialMode) {
        *lock(&self.inner.dial_mode) = mode;
    }

    pub fn set_io_mode(&self, mode: IoMode) {
        *lock(&self.inner.io_mode) = mode;
    }

    /// Every `dial` the fake received, in order.
    pub fn dials(&self) -> Vec<DialRecord> {
        lock(&self.inner.dials).clone()
    }

    /// Completions the fake has delivered (dial + io, all codes).
    pub fn completions(&self) -> usize {
        self.inner.completions.load(Ordering::SeqCst)
    }

    pub fn reads(&self) -> usize {
        self.inner.reads.load(Ordering::SeqCst)
    }

    pub fn writes(&self) -> usize {
        self.inner.writes.load(Ordering::SeqCst)
    }

    pub fn closes(&self) -> usize {
        self.inner.closes.load(Ordering::SeqCst)
    }

    /// Streams the fake still holds open.
    pub fn open_streams(&self) -> usize {
        lock(&self.inner.streams).len()
    }

    /// Completions parked under [`IoMode::Hold`].
    pub fn held(&self) -> usize {
        lock(&self.inner.held).len()
    }

    /// Dial completions parked under [`DialMode::Hold`].
    pub fn held_dials(&self) -> usize {
        lock(&self.inner.held_dials).len()
    }

    /// Deliver every parked completion NOW, on the calling thread — the io
    /// ones, then the held dials.
    pub fn release_held(&self) {
        let held: Vec<Held> = lock(&self.inner.held).drain(..).collect();
        for h in held {
            h.op.complete_once(&self.inner, h.code, h.n);
        }
        let dials: Vec<HeldDial> = lock(&self.inner.held_dials).drain(..).collect();
        for d in dials {
            let ctx = Arc::as_ptr(&self.inner) as usize;
            host_completion(ctx, d.sdk_ctx, d.op_id, ZW_DIAL_OK, d.stream, d.complete);
        }
    }

    /// Spin until `pred` holds, or panic after `within`.
    pub fn wait_until(&self, within: Duration, what: &str, mut pred: impl FnMut(&Self) -> bool) {
        let deadline = Instant::now() + within;
        while !pred(self) {
            assert!(
                Instant::now() < deadline,
                "the fake host never reached: {what}"
            );
            thread::sleep(Duration::from_millis(2));
        }
    }
}

impl Drop for FakeHost {
    fn drop(&mut self) {
        // A tokio runtime must not be dropped inside an async context; the
        // background shutdown is the documented way out of one.
        if let Some(rt) = lock(&self.inner.runtime).take() {
            rt.shutdown_background();
        }
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Op {
    /// The fake's exactly-once: the first caller delivers, every later one
    /// finds `done` and delivers nothing.
    fn complete_once(&self, host: &Inner, code: u32, n: usize) -> bool {
        if self.done.swap(true, Ordering::SeqCst) {
            return false;
        }
        host.completions.fetch_add(1, Ordering::SeqCst);
        (self.complete)(self.sdk_ctx as *mut c_void, self.op_id, code, n);
        true
    }
}

impl Inner {
    fn register_stream(&self, stream: tokio::net::TcpStream) -> u64 {
        let handle = self.next_handle.fetch_add(1, Ordering::SeqCst);
        let (read, write) = stream.into_split();
        lock(&self.streams).insert(
            handle,
            Arc::new(StreamEntry {
                read: tokio::sync::Mutex::new(read),
                write: tokio::sync::Mutex::new(write),
                closed: AtomicBool::new(false),
                notify: Notify::new(),
                outstanding: Mutex::new(Vec::new()),
            }),
        );
        handle
    }

    /// Connect to the loopback peer on the calling thread and register the
    /// stream (`ConnectInline`, `Hold`).
    fn connect_now(&self, host_name: &str, port: u16) -> u64 {
        let std_stream = std::net::TcpStream::connect((host_name, port)).expect("loopback connect");
        std_stream
            .set_nonblocking(true)
            .expect("nonblocking for tokio");
        let _enter = self.handle.enter();
        let stream =
            tokio::net::TcpStream::from_std(std_stream).expect("tokio adopts the std stream");
        self.register_stream(stream)
    }

    fn stream(&self, handle: u64) -> Option<Arc<StreamEntry>> {
        lock(&self.streams).get(&handle).cloned()
    }

    /// Deliver an io result under the current [`IoMode`].
    fn deliver(&self, entry: &StreamEntry, op: Arc<Op>, code: u32, n: usize, cap: usize) {
        lock(&entry.outstanding).retain(|o| !Arc::ptr_eq(o, &op));
        let mode = *lock(&self.io_mode);
        match mode {
            IoMode::Hold => lock(&self.held).push(Held { op, code, n }),
            IoMode::OverReport => {
                op.complete_once(self, ZW_DIAL_OK, cap + 1);
            }
            IoMode::Async | IoMode::Inline => {
                op.complete_once(self, code, n);
            }
        }
    }
}

/// # Safety
/// `ctx` is the `Arc<Inner>` pointer the fake's vtable carries; the SDK passes
/// it back verbatim while the fake is registered and alive.
unsafe fn inner_of<'a>(ctx: *mut c_void) -> &'a Inner {
    // SAFETY: the vtable's `ctx` is `Arc::as_ptr(&inner)`, alive while registered.
    unsafe { &*(ctx as *const Inner) }
}

// The four `extern "C"` verbs are thin `catch_unwind` wrappers: a panic
// inside an `extern "C"` fn ABORTS the process, which leaves no verdict line
// at all (watched once, on a mutant that starved an inline read). A fake
// that fails internally must instead refuse synchronously with a code the
// SDK maps typed, so the TEST's own assertion is what reds.

unsafe extern "C" fn fake_dial(
    ctx: *mut c_void,
    host_ptr: *const u8,
    host_len: usize,
    port: u16,
    key_ptr: *const u8,
    key_len: usize,
    op_id: u64,
    sdk_ctx: *mut c_void,
    complete: DialCompleteFn,
) -> u32 {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: forwarded verbatim; the wrapper adds only the boundary.
        unsafe {
            fake_dial_impl(
                ctx, host_ptr, host_len, port, key_ptr, key_len, op_id, sdk_ctx, complete,
            )
        }
    }))
    .unwrap_or(ZW_DIAL_UNREACHABLE)
}

unsafe extern "C" fn fake_read(
    ctx: *mut c_void,
    stream: u64,
    buf: *mut u8,
    cap: usize,
    op_id: u64,
    sdk_ctx: *mut c_void,
    complete: IoCompleteFn,
) -> u32 {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: forwarded verbatim; the wrapper adds only the boundary.
        unsafe { fake_read_impl(ctx, stream, buf, cap, op_id, sdk_ctx, complete) }
    }))
    .unwrap_or(ZW_DIAL_UNREACHABLE)
}

unsafe extern "C" fn fake_write(
    ctx: *mut c_void,
    stream: u64,
    buf: *const u8,
    len: usize,
    op_id: u64,
    sdk_ctx: *mut c_void,
    complete: IoCompleteFn,
) -> u32 {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: forwarded verbatim; the wrapper adds only the boundary.
        unsafe { fake_write_impl(ctx, stream, buf, len, op_id, sdk_ctx, complete) }
    }))
    .unwrap_or(ZW_DIAL_UNREACHABLE)
}

unsafe extern "C" fn fake_close(ctx: *mut c_void, stream: u64) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: forwarded verbatim; the wrapper adds only the boundary.
        unsafe { fake_close_impl(ctx, stream) }
    }));
}

// The header's `dial` shape, forwarded argument for argument.
#[allow(clippy::too_many_arguments)]
unsafe fn fake_dial_impl(
    ctx: *mut c_void,
    host_ptr: *const u8,
    host_len: usize,
    port: u16,
    key_ptr: *const u8,
    key_len: usize,
    op_id: u64,
    sdk_ctx: *mut c_void,
    complete: DialCompleteFn,
) -> u32 {
    // SAFETY: the SDK's vtable contract — `ctx` is ours; `host` is valid for
    // `host_len` reads and `isolation_key` for `key_len` reads for the
    // duration of THIS call (we copy both before returning).
    let (host, host_name, key) = unsafe {
        let h = inner_of(ctx);
        let name = std::slice::from_raw_parts(host_ptr, host_len).to_vec();
        let key = if key_ptr.is_null() {
            assert_eq!(key_len, 0, "a null key rides length 0");
            None
        } else {
            Some(std::slice::from_raw_parts(key_ptr, key_len).to_vec())
        };
        (h, name, key)
    };
    let host_name = String::from_utf8(host_name).expect("the SDK passes a UTF-8 host name");
    lock(&host.dials).push(DialRecord {
        host: host_name.clone(),
        port,
        isolation_key: key,
    });
    let mode = *lock(&host.dial_mode);
    let sdk_ctx = sdk_ctx as usize;
    let ctx_addr = ctx as usize;
    match mode {
        DialMode::RefuseSync(code) => code,
        DialMode::Complete(code) => {
            host.handle.spawn(async move {
                host_completion(ctx_addr, sdk_ctx, op_id, code, 0, complete);
            });
            ZW_DIAL_OK
        }
        DialMode::ZeroHandle => {
            host.handle.spawn(async move {
                host_completion(ctx_addr, sdk_ctx, op_id, ZW_DIAL_OK, 0, complete);
            });
            ZW_DIAL_OK
        }
        DialMode::ConnectInline => {
            let handle = host.connect_now(&host_name, port);
            host_completion(ctx_addr, sdk_ctx, op_id, ZW_DIAL_OK, handle, complete);
            ZW_DIAL_OK
        }
        DialMode::Hold => {
            let stream = host.connect_now(&host_name, port);
            lock(&host.held_dials).push(HeldDial {
                sdk_ctx,
                op_id,
                stream,
                complete,
            });
            ZW_DIAL_OK
        }
        DialMode::Connect => {
            host.handle.spawn(async move {
                // SAFETY: `ctx_addr` is the fake's `Arc<Inner>` pointer, alive
                // while the fake is registered (the tests keep it so).
                let h = unsafe { inner_of(ctx_addr as *mut c_void) };
                match tokio::net::TcpStream::connect((host_name.as_str(), port)).await {
                    Ok(stream) => {
                        let handle = h.register_stream(stream);
                        host_completion(ctx_addr, sdk_ctx, op_id, ZW_DIAL_OK, handle, complete);
                    }
                    Err(_) => {
                        host_completion(ctx_addr, sdk_ctx, op_id, ZW_DIAL_UNREACHABLE, 0, complete);
                    }
                }
            });
            ZW_DIAL_OK
        }
    }
}

fn host_completion(
    ctx: usize,
    sdk_ctx: usize,
    op_id: u64,
    code: u32,
    stream: u64,
    complete: DialCompleteFn,
) {
    // SAFETY: `ctx` is the fake's `Arc<Inner>` pointer, alive while registered.
    let h = unsafe { inner_of(ctx as *mut c_void) };
    h.completions.fetch_add(1, Ordering::SeqCst);
    complete(sdk_ctx as *mut c_void, op_id, code, stream);
}

unsafe fn fake_read_impl(
    ctx: *mut c_void,
    stream: u64,
    buf: *mut u8,
    cap: usize,
    op_id: u64,
    sdk_ctx: *mut c_void,
    complete: IoCompleteFn,
) -> u32 {
    // SAFETY: `ctx` is ours (vtable contract).
    let h = unsafe { inner_of(ctx) };
    h.reads.fetch_add(1, Ordering::SeqCst);
    let Some(entry) = h.stream(stream) else {
        return ZW_DIAL_RETIRED;
    };
    let op = Arc::new(Op {
        op_id,
        sdk_ctx: sdk_ctx as usize,
        complete,
        done: AtomicBool::new(false),
    });
    lock(&entry.outstanding).push(Arc::clone(&op));
    let mode = *lock(&h.io_mode);
    let ptr = SendPtr(buf as usize);
    if matches!(mode, IoMode::Inline) {
        // Complete INLINE: spin on the readable half until the peer's bytes
        // are there (loopback: milliseconds), then complete before returning.
        let half = entry
            .read
            .try_lock()
            .expect("one outstanding read at a time");
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            // SAFETY: the SDK's `buf` is valid for `cap` writes until the
            // completion we deliver below, on this very thread.
            let slice = unsafe { std::slice::from_raw_parts_mut(ptr.0 as *mut u8, cap) };
            match half.try_read(slice) {
                Ok(n) => {
                    drop(half);
                    h.deliver(&entry, op, ZW_DIAL_OK, n, cap);
                    return ZW_DIAL_OK;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        // No bytes within 5 s: complete TIMEOUT (typed), never
                        // panic inside an `extern "C"` frame.
                        drop(half);
                        h.deliver(&entry, op, ZW_DIAL_TIMEOUT, 0, cap);
                        return ZW_DIAL_OK;
                    }
                    thread::sleep(Duration::from_millis(1));
                }
                Err(_) => {
                    drop(half);
                    h.deliver(&entry, op, ZW_DIAL_UNREACHABLE, 0, cap);
                    return ZW_DIAL_OK;
                }
            }
        }
    }
    let ctx_addr = ctx as usize;
    h.handle.spawn(async move {
        // SAFETY: `ctx_addr` is the fake's `Arc<Inner>` pointer, alive while registered.
        let h = unsafe { inner_of(ctx_addr as *mut c_void) };
        let mut half = entry.read.lock().await;
        // Register the waiter BEFORE checking the flag so a `close` between
        // the check and the wait cannot be missed (the `Notify` pattern).
        let notified = entry.notify.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        if entry.closed.load(Ordering::SeqCst) {
            return; // `close` retires every outstanding op itself
        }
        // SAFETY: the SDK's `buf` stays valid for `cap` writes until the
        // completion for `op_id` — even after `close` (the header's buffer
        // rule) — and this task delivers at most one.
        let slice = unsafe { std::slice::from_raw_parts_mut(ptr.0 as *mut u8, cap) };
        let outcome = tokio::select! {
            r = half.read(slice) => Some(r),
            _ = &mut notified => None,
        };
        drop(half);
        match outcome {
            None => {}
            Some(Ok(n)) => h.deliver(&entry, op, ZW_DIAL_OK, n, cap),
            Some(Err(_)) => h.deliver(&entry, op, ZW_DIAL_UNREACHABLE, 0, cap),
        }
    });
    ZW_DIAL_OK
}

unsafe fn fake_write_impl(
    ctx: *mut c_void,
    stream: u64,
    buf: *const u8,
    len: usize,
    op_id: u64,
    sdk_ctx: *mut c_void,
    complete: IoCompleteFn,
) -> u32 {
    // SAFETY: `ctx` is ours (vtable contract).
    let h = unsafe { inner_of(ctx) };
    h.writes.fetch_add(1, Ordering::SeqCst);
    let Some(entry) = h.stream(stream) else {
        return ZW_DIAL_RETIRED;
    };
    let op = Arc::new(Op {
        op_id,
        sdk_ctx: sdk_ctx as usize,
        complete,
        done: AtomicBool::new(false),
    });
    lock(&entry.outstanding).push(Arc::clone(&op));
    let mode = *lock(&h.io_mode);
    let ptr = SendPtr(buf as usize);
    if matches!(mode, IoMode::Inline) {
        // Complete INLINE, mirroring the read path above: tokio's `try_write`
        // answers from CACHED readiness, which only the fake's own reactor
        // updates after `ConnectInline` adopted the socket a moment ago — so a
        // single `try_write` is a race against that driver thread (lost under
        // parallel test load). Poll it with the read path's bounded
        // deadline; the completion is still delivered before the verb returns.
        let half = entry
            .write
            .try_lock()
            .expect("one outstanding write at a time");
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            // SAFETY: the SDK's `buf` is valid for `len` reads until the
            // completion delivered below on this thread.
            let slice = unsafe { std::slice::from_raw_parts(ptr.0 as *const u8, len) };
            match half.try_write(slice) {
                Ok(n) => {
                    drop(half);
                    h.deliver(&entry, op, ZW_DIAL_OK, n, len);
                    return ZW_DIAL_OK;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        drop(half);
                        h.deliver(&entry, op, ZW_DIAL_TIMEOUT, 0, len);
                        return ZW_DIAL_OK;
                    }
                    thread::sleep(Duration::from_millis(1));
                }
                Err(_) => {
                    drop(half);
                    h.deliver(&entry, op, ZW_DIAL_UNREACHABLE, 0, len);
                    return ZW_DIAL_OK;
                }
            }
        }
    }
    let ctx_addr = ctx as usize;
    h.handle.spawn(async move {
        // SAFETY: `ctx_addr` is the fake's `Arc<Inner>` pointer, alive while registered.
        let h = unsafe { inner_of(ctx_addr as *mut c_void) };
        let mut half = entry.write.lock().await;
        let notified = entry.notify.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        if entry.closed.load(Ordering::SeqCst) {
            return;
        }
        // SAFETY: the SDK's `buf` stays valid for `len` reads until this op's
        // completion (the header's buffer rule).
        let slice = unsafe { std::slice::from_raw_parts(ptr.0 as *const u8, len) };
        let outcome = tokio::select! {
            r = half.write(slice) => Some(r),
            _ = &mut notified => None,
        };
        drop(half);
        match outcome {
            None => {}
            Some(Ok(n)) => h.deliver(&entry, op, ZW_DIAL_OK, n, len),
            Some(Err(_)) => h.deliver(&entry, op, ZW_DIAL_UNREACHABLE, 0, len),
        }
    });
    ZW_DIAL_OK
}

unsafe fn fake_close_impl(ctx: *mut c_void, stream: u64) {
    // SAFETY: `ctx` is ours (vtable contract).
    let h = unsafe { inner_of(ctx) };
    h.closes.fetch_add(1, Ordering::SeqCst);
    let Some(entry) = lock(&h.streams).remove(&stream) else {
        return; // idempotent
    };
    entry.closed.store(true, Ordering::SeqCst);
    entry.notify.notify_waiters();
    // The header: after `close` returns, every outstanding op on the stream
    // is completed exactly once with RETIRED. Parked (`Hold`) ops left the
    // outstanding list already — their release is their one completion.
    let outstanding: Vec<Arc<Op>> = lock(&entry.outstanding).drain(..).collect();
    for op in outstanding {
        op.complete_once(h, ZW_DIAL_RETIRED, 0);
    }
}

/// A loopback peer on a plain std thread, owned by the test.
pub struct LoopbackServer {
    pub addr: SocketAddr,
}

/// What the peer does with each accepted connection.
#[derive(Clone, Copy, Debug)]
pub enum Peer {
    /// Echo everything back until EOF.
    Echo,
    /// Send `greeting` immediately, then echo until EOF.
    GreetThenEcho(&'static [u8]),
    /// Echo the first chunk, then close the connection (the EOF proof).
    EchoOnceThenClose,
}

impl LoopbackServer {
    pub fn spawn(peer: Peer) -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().expect("local addr");
        thread::spawn(move || {
            for conn in listener.incoming() {
                let Ok(mut conn) = conn else { break };
                thread::spawn(move || {
                    let mut buf = [0u8; 4096];
                    if let Peer::GreetThenEcho(greeting) = peer
                        && conn.write_all(greeting).is_err()
                    {
                        return;
                    }
                    loop {
                        let n = match conn.read(&mut buf) {
                            Ok(0) | Err(_) => return,
                            Ok(n) => n,
                        };
                        if conn.write_all(&buf[..n]).is_err() {
                            return;
                        }
                        if matches!(peer, Peer::EchoOnceThenClose) {
                            return; // drop closes the socket → the SDK reads EOF
                        }
                    }
                });
            }
        });
        Self { addr }
    }

    pub fn host(&self) -> String {
        self.addr.ip().to_string()
    }

    pub fn port(&self) -> u16 {
        self.addr.port()
    }
}
