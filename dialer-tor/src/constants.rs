//! The owned client's named values (`tor-plugin.md` §12.2; no magic numbers),
//! each with its WHY. The other bounds of this crate live beside the code they
//! bound and are re-exported from the crate root.

use std::time::Duration;

/// The owned runtime's worker threads. It only drives arti's own tasks and its
/// I/O; the host's runtime runs the host. One worker keeps a client's cost to
/// one thread while its I/O driver runs independently of the host's runtime.
pub const OWNED_WORKER_THREADS: usize = 1;

/// The owned runtime's blocking-pool cap. arti asks for no blocking threads;
/// a small cap bounds what a STUCK client (one whose shutdown overran) can
/// hold for the rest of the process.
pub const OWNED_MAX_BLOCKING_THREADS: usize = 2;

/// The shutdown budget a host is advised to pass. A normal shutdown is a drop
/// path of milliseconds (arti runs nothing on a blocking pool), so an overrun
/// means a worker is stuck in a synchronous filesystem call inside a task (the
/// directory manager's SQLite store, or the drop-time state write itself).
/// Ten seconds is a wide margin over that on a loaded phone, and short enough
/// not to hold a wipe. Not yet measured on a device.
pub const RECOMMENDED_SHUTDOWN_BUDGET: Duration = Duration::from_secs(10);

/// How much longer than its budget a caller of `OwnedDialer::shutdown` waits
/// for the outcome before reading `Overran`. The shutdown thread's own clock
/// starts only once that thread runs (thread creation on a loaded phone), and
/// the client's drop runs before the runtime shutdown with no bound of its
/// own; two seconds covers a slow thread start without letting a blocked
/// drop hold the caller indefinitely.
pub const SHUTDOWN_WAIT_GRACE: Duration = Duration::from_secs(2);

/// The longest budget a shutdown runs with; a larger one is cut to this. No
/// shutdown needs an hour (a normal one takes milliseconds), and the cap
/// keeps every deadline built from a budget (`now + budget + grace`) far from
/// overflow, so a host passing `Duration::MAX` cannot panic a shutdown.
pub(crate) const MAX_SHUTDOWN_BUDGET: Duration = Duration::from_secs(60 * 60);
