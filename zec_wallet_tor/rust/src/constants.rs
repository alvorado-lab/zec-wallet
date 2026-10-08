//! Every named value the plugin uses, each with its WHY (`tor-plugin.md` §2
//! and §7; gate 7 — no magic numbers). The ONE home: other modules name these
//! constants and never restate a literal. `dialer-tor`'s own bounds are
//! consumed by re-export, never re-minted.

use std::time::Duration;

/// The name the plugin registers in the wallet's transport descriptor — a
/// proper noun, unlocalized by design (the descriptor's name is rendered
/// verbatim; Relim registers the same word). Within
/// `ZW_TRANSPORT_NAME_MAX_BYTES`, no control characters — the crossing's name
/// rules (P3 pins that the wallet accepts it).
pub const TRANSPORT_NAME: &str = "Tor";

/// The one readiness the SDK dials at: arti's `ready_for_traffic()` is true.
pub const READINESS_READY: u32 = 100;

/// A bootstrapping client is capped one below [`READINESS_READY`]: arti's
/// `as_frac()` can read 1.0 before `ready_for_traffic()` (the directory is
/// usable, no channel yet), and 100 is the one value the wallet renders as a
/// working private path.
pub const READINESS_CEILING_WHILE_BOOTSTRAPPING: u32 = 99;

/// What a pause pushes — 0, never "the last value", so a resumed wallet never
/// sees a stale 100 (§3.4).
pub const READINESS_SUSPENDED: u32 = 0;

/// The watcher's cadence while bootstrapping: a local read, no traffic; one
/// second is the resolution the chip's percent needs (Relim's figure).
pub const READINESS_POLL: Duration = Duration::from_secs(1);

/// The watcher's cadence past the bootstrap budget: the transport is no longer
/// "starting", but the loop must not end at ready, because the expensive miss
/// is the DROP (Relim's figure).
pub const READINESS_POLL_SLOW: Duration = Duration::from_secs(10);

/// When the watcher slows: symbolized, never a second literal — the bootstrap
/// budget, measured on the runtime's clock (only the PAUSE interval needs the
/// sleep-counting pause clock).
pub const SLOW_AFTER: Duration = BOOTSTRAP_DEADLINE;

/// How many consecutive watcher reads a DROP must persist before the wallet is
/// told its path died: arti's readiness is not monotonic. It delays the
/// ANNOUNCEMENT only — `dial` always re-reads the live value (Relim's figure).
pub const DOWNWARD_CONFIRMATIONS: u32 = 3;

/// Backoff jitter: up to one part in this many, only ever LENGTHENING the
/// wait, on the plugin's own clock — two installs never pace together
/// (Relim's figure).
pub const JITTER_SHARE: u32 = 4;

/// One bootstrap attempt's budget: Relim's figure (three relay-request
/// budgets), so both products give Tor the same patience. A censor sets the
/// attempt speed, so no value is "long enough"; past it the honest state is
/// `Failed` plus a retry, not a longer spinner.
pub const BOOTSTRAP_DEADLINE: Duration = Duration::from_secs(180);

/// The first retry after a failed bootstrap: the shortest interval after which
/// a network that was merely slow can look different.
///
/// It was DERIVED as "one SDK dial budget" when that budget was 30 s. Since
/// ADR-0552 stage 1b the SDK's budget is 25 s
/// (`ZW_NET_DIALER_DIAL_BUDGET_SECS`), and this deliberately did NOT follow it
/// down: a retry base ABOVE one dial budget keeps the derivation's intent — a
/// full attempt has finished before the next begins — while tracking the SDK's
/// number would make this value move whenever theirs does.
///
/// **Do not restate the SDK's figure here again.** This comment previously read
/// "one SDK dial budget (`DIAL_TIMEOUT_SECS`, 30 s)" and became false the day
/// that constant moved, with nothing on this side to notice: the plugin is its
/// own cargo workspace (ADR-0551), so no gate here can see it. Point at the
/// symbol, never at the number.
pub const BOOTSTRAP_RETRY_BASE: Duration = Duration::from_secs(30);

/// The ceiling of the doubling retry: a wallet left open on a censored network
/// retries at most six times an hour; longer would leave a user who fixed
/// their network waiting.
pub const BOOTSTRAP_RETRY_CAP: Duration = Duration::from_secs(10 * 60);

/// A pause longer than this rebuilds the client on resume. BELOW iOS's sourced
/// five seconds for `applicationDidEnterBackground` (the process can be
/// suspended "shortly after" and then has no live sockets), because the costs
/// are asymmetric: a needless rebuild is one warm bootstrap; a missed one is a
/// `Preferred` fallback the OS caused. Two seconds keeps a quick app switch on
/// the short branch. Measured on the sleep-counting pause clock (§7
/// `PAUSE_CLOCK`), never `Instant`.
pub const REBUILD_AFTER_PAUSE: Duration = Duration::from_secs(2);

/// The header's "bounded delay" for a superseded backing: an arti stream that
/// has not completed five seconds after a retire is a dead socket the OS will
/// reap.
pub const RETIRE_QUIESCE_MAX: Duration = Duration::from_secs(5);

/// How long a retired client's own runtime may take to shut down (plan §5):
/// the shutdown ends every task arti started for that client, so its last
/// state write runs inside this window. Overrun, and the client stays counted
/// live for the life of the process (fail closed: `clear_state` and `init`
/// refuse, and the reset is a clear on the next start). Off the host's thread
/// (a dedicated one), so it does not bound any call the host makes. The budget
/// the plugin passes to `dialer-tor`'s owned client (its
/// `RECOMMENDED_SHUTDOWN_BUDGET`, chosen here first); a rebuild's own wait is
/// [`REBUILD_WAIT_MAX`] (`tor-plugin.md` §12).
pub const ENGINE_SHUTDOWN_MAX: Duration = Duration::from_secs(10);

/// The most a rebuild waits for EVERY earlier client to stop before it mints
/// (`tor-plugin.md` §12): the old client's shutdown and, at most, one
/// superseded half-built client's, each bounded by [`ENGINE_SHUTDOWN_MAX`] on
/// a thread that may start late. Past it, or as soon as a client is stuck,
/// the rebuild mints nothing and takes the terminal setup path: never two
/// clients on one `state/`.
pub const REBUILD_WAIT_MAX: Duration = Duration::from_secs(2 * ENGINE_SHUTDOWN_MAX.as_secs());

/// How often a rebuild checks whether the superseded engine's last user let
/// go, within [`RETIRE_QUIESCE_MAX`]: a local count, no traffic; fine enough
/// that a rebuild waits no longer than it must.
pub const QUIESCE_POLL: Duration = Duration::from_millis(50);

/// The plugin's own bound on one `dial`: the trampoline completes every
/// accepted dial within it, with a code IT chose (the crypto angle's
/// HIGH). It sits strictly BELOW the wallet's `ZW_NET_DIALER_DIAL_BUDGET_SECS`
/// (read from the wallet's header by
/// `the_dial_deadline_sits_below_the_wallets_budget`, never restated here),
/// so the wallet never ends a dial on its own clock — that elapse is a
/// reachability TIMEOUT a `Preferred` wallet follows to clearnet after its
/// patience window. Five seconds under today's budget: room for the
/// completion to cross threads on a loaded phone.
///
/// An elapse completes `REFUSED`, never `TIMEOUT`: the dial code stays off the
/// fallback codes. It DOES count toward [`LIVENESS_FAILED_DIALS`]: arti bounds
/// a stream's BEGIN at its own 10 s `connect_timeout` and reports that as an
/// exit/remote kind, so an elapse at 20 s means circuit acquisition — Tor-side
/// time the destination cannot hold (the crypto angle).
pub const DIAL_DEADLINE: Duration = Duration::from_secs(20);

/// arti's readiness never drops once it has connected (`tor-chanmgr`
/// `event.rs:249`, upstream's own TODO; the crypto angle's MEDIUM), so
/// the plugin derives liveness itself: this many CONSECUTIVE failed dials
/// while `Ready` — a [`DIAL_DEADLINE`] elapse, or a kind the device, the path
/// to Tor or the Tor network produces (`TorAccessFailed`, `LocalNetworkError`,
/// `TorNetworkTimeout`; never an exit- or destination-side kind, so a stalling
/// server cannot declare the path dead) — move the plugin to `Failed`, pushed
/// at once as health FAILED, and the recovery is a REBUILD (a fresh client is
/// the only way to reset arti's latch). Three: one failure is a guard switch
/// arti handles by itself; three in a row with no success between is a dead
/// path. Worst case before the wallet learns: three dials, each bounded by
/// [`DIAL_DEADLINE`].
///
/// What FAILED means for the wallet, stated because two reviews found it
/// unstated: the counted kinds INCLUDE what a guard, a bridge operator
/// or an on-path censor can cause. Under `Required` FAILED is fail-closed.
/// Under `Preferred` it is switch-eligible after the wallet's patience minute
/// (ADR-0553 v4) — the maintainer's ruling for a declared failure, the same one
/// a censor already reaches by failing the bootstrap. A host that cannot
/// accept that runs `Required` (the README says so).
pub const LIVENESS_FAILED_DIALS: u32 = 3;

/// The marker the plugin writes in `tor_dir` at `init`; `clear_state` refuses
/// a directory without it, so a mistaken path (the wallet's `db_dir`, an app's
/// data root whose `cache` belongs to the OS) loses nothing (the security
/// angle's A2).
pub const TOR_DIR_MARKER: &str = ".zec_wallet_tor";

/// Written beside [`TOR_DIR_MARKER`] (never inside `state/`) when `clear_state`
/// is refused because a client is stuck: the next `init` after a restart (in
/// this process `init` answers "restart required" first) removes the state
/// before it creates anything, so an
/// identity reset the host asked for completes even if the app is killed and
/// the host never asks again (the crypto audit of plan §5).
pub const CLEAR_PENDING_MARKER: &str = ".zec_wallet_tor_clear_pending";

/// The two subtrees the plugin creates under `tor_dir`, and the only ones
/// `clear_state` removes (§2.3; plan D-16).
pub const STATE_SUBDIR: &str = "state";
/// See [`STATE_SUBDIR`].
pub const CACHE_SUBDIR: &str = "cache";

/// The plugin's own tokio runtime: arti's work is I/O-bound; one worker would
/// serialize a bootstrap behind stream I/O, and more than two buys nothing on
/// a phone while costing idle threads the OS must schedule.
pub const PLUGIN_RUNTIME_WORKERS: usize = 2;

/// The plugin-owned staging buffer per direction per stream (§3.3: the plugin
/// never hands an SDK pointer to arti). The SDK's own read/write chunk.
pub const STAGING_BUFFER_BYTES: usize = 16 * 1024;

/// `dialer-tor`'s bridge bounds and isolation-key cap, consumed by re-export
/// with their WHYs at their one home.
pub use dialer_tor::MAX_ISOLATION_KEYS;
pub use dialer_tor::{MAX_BRIDGE_CONFIG_BYTES, MAX_BRIDGE_LINE_BYTES, MAX_BRIDGE_LINES};

/// The plugin's own two failure classes, beside `dialer-tor`'s (consumed by
/// re-export, never re-minted). PII-free, frozen: they cross the C ABI as the
/// closed `ZWT_CLASS_*` codes and the Dart side owns the code → name table.
pub const CLASS_BOOTSTRAP_DEADLINE: &str = "bootstrap-deadline";
/// `init` was refused at the crossing, or `dispose` ran.
pub const CLASS_NOT_REGISTERED: &str = "not-registered";
/// A ready client whose dials kept failing ([`LIVENESS_FAILED_DIALS`] in a
/// row): the path died after it came up. Code 21, appended at C3b.
pub const CLASS_CIRCUITS_FAILING: &str = "circuits-failing";
