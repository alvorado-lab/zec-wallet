//! Named constants (spec §7 + §2) — every threshold/timeout/cap is a NAMED
//! constant whose comment explains WHY this value (validation gate 7). The
//! named test `named_constants_at_boundary` pins every value below, so a
//! drive-by edit fails CI loudly and comes to review with its WHY.
//!
//! Clock discipline (spec §7): all internal durations (watchdog, backoff,
//! poll, jitter, proposal TTL) are measured on the MONOTONIC clock — a
//! wall-clock jump (NTP, timezone, manual set) must never fire a watchdog or
//! freeze a backoff. Only swap deadlines touch wall time, display-only.

// ── Seed bounds (spec §2.2 / §4.2) ───────────────────────────────────────────

/// Lower seed-length bound, enforced BEFORE any upstream call: verified
/// (2026-06-10) `UnifiedSpendingKey::from_seed` PANICS on seeds < 32 bytes,
/// and a panic across FFI is a crash, not an error.
pub const SEED_MIN_BYTES: usize = 32;

/// Upper seed-length bound = the ZIP-32 master-seed maximum (252 bytes). This is the
/// PANIC-floor door (`< 32` panics in `from_seed`; `> 252` is a typed reject). NOTE (ADR-0528):
/// with `transparent-inputs` ON, `from_seed` also derives the BIP44 transparent key (BIP32,
/// 16/32/64-byte master seeds only), so an in-range-but-non-BIP32 length (e.g. 48/100/252)
/// passes THIS door and then returns a typed `KeyDerivation` at derivation — never a panic.
/// Every production seed is 32 (raw) or 64 (BIP39), so no real wallet is affected; the bound
/// stays 252 as the panic-floor guard, with derivation enforcing the narrower {32, 64} reality.
pub const SEED_MAX_BYTES: usize = 252;

// ── Money (spec §2.1) ────────────────────────────────────────────────────────

/// Maximum zatoshis that can ever exist (21M ZEC × 10⁸). Constructor bound:
/// any amount beyond this is corrupt or hostile input, never a real balance.
/// Also < 2⁵³, so every representable value survives the FRB i64 → Dart `int`
/// mapping exactly on all shipped platforms (no web, §9).
pub const MAX_MONEY_ZAT: i64 = 2_100_000_000_000_000;

// ── Memos & payment requests (spec §2.4 / §4.6) ──────────────────────────────

/// ZIP-302 memo field is 512 bytes on the wire; a text memo's UTF-8 content
/// may fill all of it.
pub const MEMO_TEXT_MAX_BYTES: usize = 512;

/// `Memo::Arbitrary` rides behind the 0xFF tag byte, leaving 511 payload
/// bytes (ZIP-302).
pub const MEMO_ARBITRARY_MAX_BYTES: usize = 511;

/// Size cap applied BEFORE parsing a `zcash:` payment URI (hostile input —
/// QR codes top out at ~4,296 alphanumeric chars; 2× headroom covers
/// multi-payment URIs from other channels without admitting megabyte garbage).
pub const PAYMENT_URI_MAX_BYTES: usize = 8_192;

/// FR-27 — the longest machine-memo READ prefix a host may register. A prefix
/// is a magic + version marker, not a payload: 32 bytes is generous for every
/// envelope shape we have seen and still far under
/// [`MEMO_ARBITRARY_MAX_BYTES`], so a prefix can never be longer than the memo
/// it filters (which would silently match nothing).
pub const MACHINE_MEMO_PREFIX_MAX_BYTES: usize = 32;

/// FR-27 — how many read prefixes one wallet may register.
///
/// This bounds the comparisons PER MEMO, not the total work: the NUMBER of
/// memos comes from the chain and nothing here caps it. The aggregate is
/// bounded instead by consensus — a transaction's outputs must fit in a block,
/// so the worst case is a few MB of transient churn on a deliberately crafted
/// max-size transaction, not an unbounded read. Stated precisely because the
/// first version of this comment claimed the count "bounds the work a hostile
/// transaction can cause", which is the wrong bound (FR-27 security review).
///
/// A host needing more than a handful of simultaneous envelope formats has a
/// design problem, not a configuration one.
pub const MACHINE_MEMO_PREFIX_MAX_COUNT: usize = 8;

/// Size cap applied BEFORE parsing a single address (user paste, QR,
/// provider strings — §4.6): a 3-receiver ZIP-316 UA encodes to ~250 chars;
/// 2× headroom admits future receiver typecodes, refuses garbage.
pub const ADDRESS_MAX_BYTES: usize = 512;

/// Hard ceiling on one transaction-history page (FR-1 `transactions(limit, …)`).
/// `limit` is a public-FFI/host-supplied value that flows to SQL `LIMIT`; clamp it so
/// a forgotten/huge page size on a wallet with a very large history can't materialize an
/// unbounded `Vec<TxSummary>` and pressure mobile memory. 500 rows is far more than one
/// scroll page yet a bounded allocation; the host paginates with the `before` cursor.
pub const MAX_TX_PAGE_SIZE: u32 = 500;

// ── Sync engine (spec §7; constants land ahead of the engine — values marked
//    "provisional" are re-measured when `LightdSyncEngine` ships) ────────────

/// tonic per-message decode ceiling — every gRPC byte is hostile input
/// (§4.6); compact blocks are KB-scale and a full tx ≤ 2 MB by protocol, so
/// 8 MiB is a generous ceiling, not a working size. Provisional: re-measured
/// when the sync engine lands.
pub const GRPC_MAX_MESSAGE_BYTES: usize = 8 * 1024 * 1024;

/// Unary gRPC timeout. Split from streaming on purpose: upstream shipped a
/// shared timeout and long block streams starved unary calls (§1.7 regression).
pub const GRPC_UNARY_TIMEOUT_SECS: u64 = 30;

/// Streaming gRPC timeout (block download ranges legitimately run long,
/// especially over Tor).
pub const GRPC_STREAMING_TIMEOUT_SECS: u64 = 100;

/// Per-UTXO scriptPubKey size cap on a `GetAddressUtxos` reply (Recv-2b transparent
/// detection, §3.3a / §4.6). A standard transparent output script is tiny — P2PKH is
/// 25 bytes, P2SH 23 — so any longer script is not a wallet-spendable receive and is
/// rejected at the hostile-input boundary BEFORE the engine parses it (the outer
/// `GRPC_MAX_MESSAGE_BYTES` bounds the whole reply; this bounds each record).
/// Pinned-boundary tested by `validate_rejects_oversized_script`.
pub const MAX_TRANSPARENT_SCRIPT_BYTES: usize = 64;

/// Bound on EVERY dial — the built-in `DirectTcpDialer` AND the host runtime
/// dialer (applied in `PolicyDialer`) — §3.2a / ADR-0526. A blackholed connect
/// or a hung host circuit must never park the caller or wedge the channel
/// worker; the liveness guarantee must NOT rest on an unstated dialer-internal
/// timeout (the W-swap-1 review fold — the `NetDialer` contract mandates
/// isolation, not timing, so the SDK enforces the bound itself).
///
/// It is also the budget the HOST must honour, so it is not ours alone: the
/// C-ABI publishes it as `ZW_NET_DIALER_DIAL_BUDGET_SECS` in
/// `sdk/zec_wallet/rust/include/zec_wallet_net_dialer.h`, and
/// `the_header_dial_budget_matches_the_core_constant` pins the two to one
/// number (the C3a security pass's A1 — before it, the budget was PROSE in a
/// comment and a host cannot honour a contract whose clock is a comment).
///
/// 25, not 30, since an earlier revision (ADR-0552 stage 1b): it must stay STRICTLY below
/// [`GRPC_UNARY_TIMEOUT_SECS`] — see the assert below for the money argument
/// that margin carries.
pub const DIAL_TIMEOUT_SECS: u64 = 25;

/// How long a `Preferred` wallet INSISTS on the private path before it may
/// switch to a direct connection — the maintainer's minute (ADR-0552:
/// "keep waiting, but I think 1 minute is enough to switch with
/// notification"). A minute of the private path NOT WORKING, per circuit
/// family: silent since its last confirmed RPC (or since the posture's
/// creation) AND failing without interruption since the first failure of the
/// run (`net/tor_posture.rs`, phase 2) — never merely a minute of not being used.
///
/// It deliberately does NOT live in [`DIAL_TIMEOUT_SECS`]. That is a LIVENESS
/// bound shared by every dial including clearnet ones, it is asserted below
/// against the streaming budget (a 60 s value fails to compile), and doubling
/// it would slow every ordinary offline failure on every platform. Before this
/// constant existed the switch happened after ONE dial bound — 30 s — which
/// was nobody's decision: it fell out of that bound.
pub const TOR_PATIENCE_SECS: u64 = 60;

/// Happy-eyeballs stagger for the `DirectTcpDialer` race (FR-21; RFC 8305 §5
/// "Connection Attempt Delay"): attempt `i` over the interleaved address list
/// starts `i × this` after the first — earlier attempts KEEP RUNNING, the
/// first established stream wins. 250 ms is the RFC's recommended default:
/// long enough that a healthy first attempt usually wins alone (one SYN
/// round-trip on any sane path), short enough that a blackholed/unroutable
/// first family costs a quarter-second, not the whole [`DIAL_TIMEOUT_SECS`]
/// budget (the field failure: IPv4-only Wi-Fi against a dual-stack
/// endpoint, where the sequential dial never reached the A record).
pub const DIAL_ATTEMPT_STAGGER_MS: u64 = 250;

/// Cap on resolved addresses tried PER FAMILY in one `DirectTcpDialer` dial.
/// Public lightwalletd deployments publish a handful of A/AAAA records; 4 per
/// family covers them while bounding the race's socket fan-out. Truncation is
/// logged (counts only — no silent caps, principle 10).
pub const DIAL_MAX_ADDRS_PER_FAMILY: usize = 4;

// The whole start schedule (last attempt starts at `(2·cap−1)·stagger`) must
// fit in the FIRST QUARTER of the dial bound — the tail attempt gets ≥ ¾ of
// the budget to actually COMPLETE a handshake, not merely to start (security
// review: a start-only pin would let a drive-by stagger bump hollow out the
// tail attempts while CI stays green). NOTE the caller-side belt in
// `DirectTcpDialer::dial` is additionally shared with `lookup_host`, which
// this assert cannot see — the race carries its own internal bound for
// exactly that reason.
const _: () = assert!(
    (2 * DIAL_MAX_ADDRS_PER_FAMILY as u64 - 1) * DIAL_ATTEMPT_STAGGER_MS
        <= DIAL_TIMEOUT_SECS * 1000 / 4
);

// The worst-case ESTABLISHMENT chain under `Preferred` WAS three serial
// `DIAL_TIMEOUT_SECS` legs — the PolicyDialer primary belt, the self-belted
// `DirectTcpDialer` fallback, then the connector's TLS-handshake budget (also
// DIAL_TIMEOUT-sized, net/grpc.rs) — and a sync pass's lazy first dial rides
// the STREAMING bound. Since phase 2 the two clearnet legs share
// `FALLBACK_ESTABLISH_BUDGET_SECS` (below), so the chain is shorter than this
// pin allows; it stays at three legs because it is the bound that holds even
// if that budget is ever withdrawn. Pin the SUM: the
// per-leg pin alone lets a DIAL_TIMEOUT bump to 34 s pass every assert while
// pushing the 3-leg chain past the 100 s stream establishment, after which
// the visible `Preferred` clearnet fallback could never complete in time.
const _: () = assert!(3 * DIAL_TIMEOUT_SECS < GRPC_STREAMING_TIMEOUT_SECS);

// THE SWITCH MUST BE REACHABLE ON THE MONEY PATH (ADR-0552 stage 1b, the crypto
// angle's MEDIUM). `send_transaction` wraps the whole lazy connect in the unary
// budget, so if the two bounds were EQUAL the RPC could cancel the dial future
// at the instant the dial bound fired — and neither the dialer's success record
// nor its fallback arm would ever run. A `Preferred` wallet whose private path
// hangs would then fail every send inside the minute forever, having never once
// evaluated the switch the maintainer ruled for. The margin is what makes the
// dialer, not the caller, the one who observes the outcome. It is a STRICT
// inequality on purpose: equality is the defect, not the boundary of it.
//
// And the margin must be a REAL one, which strictness alone does not say: a
// 29/30 split satisfies the line above while leaving the dial finishing one
// second before its caller gives up — too tight for the dialer to reliably
// observe its own timeout, latch, and dial the fallback. Five seconds is the
// floor, and it is asserted rather than tested because it is a compile-time
// fact about two literals: a runtime test of it could never fail on a tree
// that builds (the departure is recorded in docs/plan/tor-patience-phase-1.md
// §11.4).
const _: () = assert!(DIAL_TIMEOUT_SECS < GRPC_UNARY_TIMEOUT_SECS);
const _: () = assert!(GRPC_UNARY_TIMEOUT_SECS - DIAL_TIMEOUT_SECS >= 5);

/// What a `Preferred` wallet's CLEARNET leg gets to establish itself once it
/// has switched — TCP and the TLS handshake TOGETHER, one deadline, not five
/// seconds each (ADR-0552 phase 2, `tor-patience-phase-2.md` §3 (c)).
///
/// The margin above makes the switch REACHABLE inside a caller's unary budget;
/// nothing made it USABLE. The fallback dial carried its own
/// [`DIAL_TIMEOUT_SECS`] and the connector's TLS handshake the same again, so
/// after a primary that hung for its whole bound the SYN left (the address
/// disclosed, the latch set) and the send still failed — the worst of both,
/// on the censored network the switch exists for. A clearnet dial on a working
/// network is fast; it does not need a Tor-sized bound.
///
/// HONEST RESIDUAL: a clearnet leg that needs longer than this fails that
/// attempt. The latch is already set, the outbox is durable and re-broadcast
/// is idempotent, so that is a delay and never a lost payment. What it is NOT
/// is a bound on the first RPC: that stays the caller's own budget, of which
/// this leg can use up all but nothing.
pub const FALLBACK_ESTABLISH_BUDGET_SECS: u64 = 5;

// The whole chain — a primary that uses its full bound, then the clearnet leg's
// establishment — must end inside ONE unary budget, or the switch is reachable
// and useless.
const _: () =
    assert!(DIAL_TIMEOUT_SECS + FALLBACK_ESTABLISH_BUDGET_SECS <= GRPC_UNARY_TIMEOUT_SECS);

// A COMPLETE private attempt must always fit inside the patience window, or the
// wallet could switch to clearnet without ever having given the private path a
// full attempt — the window would then be shorter than the failure it waits for.
const _: () = assert!(TOR_PATIENCE_SECS > DIAL_TIMEOUT_SECS);
// And the switch must happen long before the wallet declares its sync wedged,
// else the honest degradation the ruling asks for arrives after the watchdog's
// verdict and the user reads "stuck" instead of "switched".
const _: () = assert!(TOR_PATIENCE_SECS < SYNC_STUCK_WATCHDOG_SECS);

/// Compact-block scan batch: balances scan throughput against
/// checkpoint/resume granularity on mobile — a killed app loses at most one
/// batch (§6.3) — AND the cooperative-cancel latency bound: one batch's
/// `scan_cached_blocks` is the ONLY uninterruptible span in the sync loop
/// (sync.rs `CancelToken` doc), so this constant IS the worst-case wait every
/// cancel-then-reacquire path (rescan step-4 quiesce, `close`, backgrounding)
/// sits behind. DEVICE-MEASURED (the v-5c rescan re-proof): at the
/// upstream-proven 1_000 a sparse batch trial-decrypted for 1–4 MINUTES on an
/// arm64 emulator (~60–240 ms/block), blowing past the 10 s rescan quiesce and
/// surfacing `WalletAlreadyOpen` to the user — the #316 defect. At 100 the
/// worst case is seconds-scale on the slowest supported hardware, safely under
/// [`crate::wallet::Wallet`]'s `QUIESCE_MAX`. COST: deep restores pay ~10× more
/// per-batch fixed overhead (one stream setup per batch, and — since SCAN-2,
/// [`TREE_STATE_RECONCILE_BATCHES`] — one `GetTreeState` RPC per that many
/// consecutive batches rather than one per batch; minutes on a 100k-block
/// restore, dwarfed by scan CPU); incremental passes (≤ 100 blocks) are unchanged.
pub const SYNC_BATCH_BLOCKS: u32 = 100;

/// Reduced batch for output-dense ranges (the upstream "sandblasting" lesson
/// §1.7: a one-size batch stalls in spam ranges). Provisional until W2+
/// measurement. Since the base [`SYNC_BATCH_BLOCKS`] equals this bound
/// (the cancel-latency fold), so the dense split is currently a no-op held for
/// the W2+ re-tune (a future throughput raise of the base must keep the dense
/// clamp — enforced below, not just documented).
pub const SYNC_BATCH_BLOCKS_DENSE: u32 = 100;
const _: () = assert!(
    SYNC_BATCH_BLOCKS_DENSE <= SYNC_BATCH_BLOCKS,
    "the dense clamp must never exceed the base batch (S148 review fold)"
);

/// Refresh the authoritative wallet summary (`get_wallet_summary` → the live
/// percent + `spendable_ready`) at most once per this many committed batches —
/// NOT every batch. RATIONALE (on-device measured): `get_wallet_summary`'s
/// scan-progress SQL (`subtree_scan_progress`) sums over the `blocks` table from
/// the start height, so its cost GROWS with the scanned set — measured at
/// `snap_ms = 6.5 s` on a tip-synced wallet, ~74 % of a batch's wall-clock.
/// Calling it every batch made a deep recovery trend O(n²) (the measured
/// 387→48 blk/s deceleration). Throttling it to every N batches restores O(n)
/// while keeping the UI honest: the per-batch `report()` still fires with the
/// carried summary and the FRESH scanned height (so "blocks left" decrements
/// every batch — only the notes-based percent lags ≤ N batches, and it moves
/// slowly anyway). A reorg (the percent can DROP — §2.5 honesty) and reaching
/// the tip BOTH force an immediate refresh, so a stale-high percent is never
/// shown. Tuned with on-device measurement; conservative default. SCALED ×10
/// with the batch-size reduction (1_000 → 100 blocks/batch) so the
/// expensive read keeps the SAME ~8_000-block cadence — throttling is
/// per-BLOCK-progress in substance, and leaving it at 8 batches would have
/// re-run the 6.5 s read every 800 blocks (the O(n²) crawl partially back).
pub const SYNC_SUMMARY_REFRESH_BATCHES: u32 = 80;

/// SCAN-2 (`docs/plan/production-readiness-phase-1.md` §4t): the DERIVED
/// per-batch scan anchor is reconciled against the endpoint's own `GetTreeState`
/// every this many consecutive batches. A batch that follows the previous one
/// (same pass, `from == previous end`, previous outcome `Scanned`) takes as its
/// anchor the previous batch's `ChainState` with that batch's note commitments
/// appended per pool (`sync::derive_chain_state` — the scanner's own leaf
/// constructors and `incrementalmerkletree`'s `Frontier::append`, no hash of
/// ours) instead of fetching it; the first batch of a pass, a batch after a
/// rewind or a scan-queue jump, and every N-th consecutive batch FETCH — the
/// last one compares the served state with the derived one whole and ends the
/// pass as the endpoint's fault (`EndpointMisbehaving`) on a disagreement.
///
/// PRICE: one `GetTreeState` round trip per N consecutive batches instead of
/// one per batch. On the Seeker (§4o-run S1-c) that RPC was p50 287 ms of an
/// 1,180 ms batch (22 %); at 20 its per-batch share is ~14 ms — ~95 % of the
/// round trips gone — against the fold's own tens of milliseconds per batch.
/// BOUGHT: a wrong derivation — a fold bug the size self-check cannot see, or
/// an endpoint whose served blocks and served tree state disagree — runs at
/// most N − 1 batches (≤ 1,900 blocks, ~40 chain-minutes at 75 s blocks) before
/// the reconcile catches it; the per-block `chain_metadata` size check inside
/// `derive_chain_state` (§4t P6) catches the size class of fault on the block
/// itself, before anything is cached. LOST, stated plainly: the batches scanned
/// on a derived anchor before a mismatch are NOT rewound by the mismatch — the
/// pass stalls, the next pass starts with a fetch and scans on from where the
/// wallet is (§4t IT-1b 1 says "re-scanned"; no mechanism does that, and this
/// constant's doc is where that is recorded). At most
/// [`SYNC_SUMMARY_REFRESH_BATCHES`] (pinned below): the reconcile runs at least
/// as often as the loop's other per-N-batches cost. `1` is the pre-SCAN-2 loop
/// (every carried anchor reconciled = every batch fetches).
pub const TREE_STATE_RECONCILE_BATCHES: u32 = 20;
const _: () = assert!(
    TREE_STATE_RECONCILE_BATCHES >= 1
        && TREE_STATE_RECONCILE_BATCHES <= SYNC_SUMMARY_REFRESH_BATCHES,
    "the anchor reconcile runs at least every batch and at most every summary refresh (§4t)"
);

/// Foreground tip-following cadence ≈ block_time/4 at 75 s blocks
/// (upstream-proven). Foreground only — this is NOT a background timer (§7).
pub const POLL_INTERVAL_SECS: u64 = 20;

/// Emit a sync-progress sample every this many blocks DURING a batch download
/// (§3.2g iv-d-3-b, the cooperative-cancel contract). Each sample re-arms the d-3
/// stuck-sync watchdog (`SYNC_STUCK_WATCHDOG_SECS`) INTRA-batch. The value is bounded
/// so that ANY link the per-message idle timeout (`GRPC_STREAMING_TIMEOUT_SECS`)
/// still considers ALIVE also re-arms the watchdog BEFORE it fires: the worst gap
/// between samples is `PROGRESS_REPORT_BLOCKS × GRPC_STREAMING_TIMEOUT_SECS` (one
/// block can take up to the idle timeout), which MUST stay below the watchdog
/// window (pinned below). So a slow-but-ADVANCING download is never falsely
/// force-restarted, and a genuinely idle link surfaces a typed `Timeout` (at the
/// idle bound, faster than the watchdog) — the false-restart-into-re-download
/// livelock band is ELIMINATED, not merely narrowed (the crypto audit fold). Also
/// strictly below the dense [`SYNC_BATCH_BLOCKS_DENSE`] so ≥ 1 sample lands strictly
/// WITHIN every batch. The per-block hot-path cost is one relaxed atomic load + (per
/// cadence) two coalescing `watch` sends — negligible.
pub const PROGRESS_REPORT_BLOCKS: u32 = 5;

// Any idle-timeout-ALIVE link must re-arm the watchdog before it fires — else a
// slow-but-advancing download is falsely cancelled + restarted into a re-download
// livelock (the crypto audit MINOR fold). Pinned at compile time, derived from the
// two timing constants so a bump to either re-checks the relationship.
const _: () =
    assert!(PROGRESS_REPORT_BLOCKS as u64 * GRPC_STREAMING_TIMEOUT_SECS < SYNC_STUCK_WATCHDOG_SECS);
// And ≥ 1 sample STRICTLY within the smallest (dense) batch (a drive-by edit cannot
// silence the intra-batch watchdog re-arm).
const _: () =
    assert!(PROGRESS_REPORT_BLOCKS > 0 && PROGRESS_REPORT_BLOCKS < SYNC_BATCH_BLOCKS_DENSE);
// The §3.3 enhancement drain (Fix A) re-arms the watchdog via a per-tx `keepalive` fired before
// each unary `GetTransaction`; the silent gap between ticks is ONE fetch, bounded by the unary
// timeout. That gap MUST stay under the watchdog window — else a slow memo backlog re-opens the
// false-`Stalled` livelock the keepalive closes. The enhancement analogue of the scan-liveness
// pin above, derived from the two constants so a bump to either re-checks the relationship.
const _: () = assert!(GRPC_UNARY_TIMEOUT_SECS < SYNC_STUCK_WATCHDOG_SECS);

/// First sync-retry backoff interval. Doubles on each consecutive fault up to
/// [`SYNC_BACKOFF_MAX_SECS`]; resets to this on a CLEAN pass only — `Ok` with no
/// rewind. A pass that rewound (`Ok` with `reorgs > 0`, or a `ChainReorg` stall)
/// climbs the ladder like a fault and sleeps `max(POLL_INTERVAL_SECS, backoff)`,
/// never below the poll interval (§4u REW-1; before it an `Ok` carrying thirty
/// rewinds reset the ladder to this and handed a patient forking endpoint a fresh
/// [`MAX_TOTAL_REORGS_PER_PASS`] budget every [`POLL_INTERVAL_SECS`]). One second
/// is the minimum observable interval — small enough that a brief blip recovers
/// near-instantly, large enough not to hot-loop a hard-down endpoint.
pub const SYNC_BACKOFF_INITIAL_SECS: u64 = 1;

/// Sync retry backoff cap. The loop retries FOREVER while started (§6.2 —
/// upstream shipped retry-capped sync, harvested "manually restart it" bug
/// reports, and reversed); only the cap is configurable by constant.
pub const SYNC_BACKOFF_MAX_SECS: u64 = 600;

/// Force-restart a sync run making no progress for this long (upstream's
/// stuck-sync watchdog value). Monotonic clock.
pub const SYNC_STUCK_WATCHDOG_SECS: u64 = 600;

/// Deepest reorg the wallet tolerates without manual intervention —
/// inherited upstream posture, never re-derived here.
pub const REORG_MAX_BLOCKS: u32 = 100;

/// Rewind distance on detected reorg — inherited upstream posture.
pub const REWIND_DISTANCE_BLOCKS: u32 = 10;

/// Hard bound on reorgs SINCE THE LAST GENUINE FORWARD PROGRESS (a new high-water
/// scanned frontier) that a single `sync_once` pass tolerates before surfacing
/// `Stalled { ChainReorg }` (§3.2g iv-d-2b-ii). Each reorg rewinds
/// [`REWIND_DISTANCE_BLOCKS`], so this many rewinds-without-progress cover
/// [`REORG_MAX_BLOCKS`] — the deepest reorg the wallet recovers from automatically.
/// The (N+1)-th means rewinding has OUTPACED real progress by more than that depth,
/// which is no longer an ordinary reorg but a misbehaving/forking endpoint: the pass
/// STOPS (honest degradation, recoverable on retry/fallback) rather than rewind-and-
/// rescan forever (money-safety: every loop terminates). The counter resets only
/// when a `Scanned` batch pushes the frontier ABOVE its prior high-water — NOT on
/// every `Scanned` — so a hostile server that interleaves a tiny scanned batch with
/// a deeper reorg (which would reset a naive *consecutive* counter and pin the pass
/// forever — the review's money-safety fold) is bounded just like a run of
/// consecutive reorgs; a long sync that legitimately weathers several SEPARATE
/// shallow reorgs, each followed by real progress, is never stalled. Derived from
/// the two upstream-posture constants so the relationship is the single source of
/// truth (a bump to either re-derives this). The time-based stuck-sync watchdog
/// (`SYNC_STUCK_WATCHDOG_SECS`) is the d-3 sibling for the no-reorg stall.
///
/// Load-bearing inside ONE pass since §4q (REQ-1, INC-025): the reorg arm re-queues
/// the rewound span under the scan lock, so a forking endpoint is met again in the
/// SAME pass rather than the next, and this bound — not the pass boundary — is what
/// ends the pass. Two bounds hold together (`sync::classify_batch` states both): a
/// server that forks WITHOUT new progress trips `reorg_storm` after this many
/// rewinds; a server that alternates a fork with a one-block advance is bounded by
/// the RECORDED tip, because the counter resets only on a batch end strictly above
/// the prior high-water and the tip is fixed for the pass — each reset buys at most
/// this many rewinds more, and there are at most `tip − first high-water` resets.
/// That second bound is a TERMINATION bound only — the tip is the endpoint's own
/// claim — and the WORK such a server can extract from one pass is capped by
/// [`MAX_TOTAL_REORGS_PER_PASS`] below (§4q-R P-RR3).
pub const MAX_SCAN_REORGS_PER_PASS: u32 = REORG_MAX_BLOCKS / REWIND_DISTANCE_BLOCKS;

/// The ABSOLUTE number of rewinds ONE `sync_once` pass may perform before it stops
/// with `Stalled { ChainReorg }` — counted on the pass's own `reorgs`, never reset,
/// regardless of the high-water (§4q-R P-RR3; the REQ-1 fold review's row 2).
/// [`MAX_SCAN_REORGS_PER_PASS`] above guarantees TERMINATION and not a bound on
/// WORK: its counter resets on every batch end above the prior high-water, and the
/// tip that limits those resets is the endpoint's own claim. So a server that
/// forks, serves a run of fabricated blocks past the high-water, and forks again —
/// the fork-advance-fork geometry — resets that epoch at will and, since the
/// in-pass re-queue (§4q), keeps ONE pass alive for up to `(tip − high_water) ×
/// (MAX_SCAN_REORGS_PER_PASS + 1)` rewinds, each a DB truncate, a scan-queue
/// rewrite, a ledger note and a re-download + re-scan from the landed height — an
/// amplification whose size the endpoint chooses. This constant is the number it
/// cannot choose: three epochs' worth (the orchestrator's proposal at §4q-R; the
/// maintainer may move it), room for a genuine deep reorg met more than once in a
/// long pass, and a wall for the rest. Enforced in `sync::classify_batch` through
/// `sync::reorg_cap` beside the epoch counter, on every reorg; a stalled pass is
/// recoverable on the next one (its rewinds are durable, its stamp never written).
///
/// **The byte ceiling this cap states (§4u RW-6 — the REQ-1-R fold review's row 1
/// found no byte bound stated anywhere).** A rewind makes the pass download twice:
/// the batch that forked (downloaded whole before the scan can see the fork, then
/// discarded unscanned — at most [`SYNC_BATCH_BLOCKS`] blocks) and the span the
/// truncate un-scanned (`SyncPass::rewound_blocks` for that rewind), which the arm
/// re-queues through `sync::record_chain_tip` as `landed + 1 ..= tip` (upstream's
/// `update_chain_tip`: `Verify`/`ChainTip` ranges from the new frontier, a
/// `VERIFY_LOOKAHEAD`-wide `Verify` range first when the frontier is near the tip).
/// The batch that forked is ONE batch, and one batch is at most
/// [`DOWNLOAD_BATCH_MAX_BYTES`] — 128 MiB, the tighter of the two byte bounds on a
/// batch (`SYNC_BATCH_BLOCKS × GRPC_MAX_MESSAGE_BYTES` = 800 MiB is the other);
/// [`CACHE_MAX_BYTES`] bounds what is RESIDENT, not what is downloaded. For a fork
/// met where forks are met — in the batch above the frontier — the truncate lands
/// [`REWIND_DISTANCE_BLOCKS`] below the erroring block and then on the nearest
/// checkpointed `blocks` row (`select_truncation_height`; checkpoints sit at
/// scanned-batch ends, so at most `SYNC_BATCH_BLOCKS` further — the REQ-1 fold
/// measured 92 on one rewind), so the un-scanned SPAN is under
/// `REWIND_DISTANCE_BLOCKS + SYNC_BATCH_BLOCKS` blocks.
///
/// **What bounds that span is the span, not a batch count** (§4u-run review row 5,
/// which found the first derivation wrong; the arithmetic below is the
/// review's correction of the correction). The re-queue puts a `Verify` range
/// first and a `ChainTip` range after, so 110 blocks arrive as three ranges or
/// more, and `DOWNLOAD_BATCH_MAX_BYTES` is a PER-BATCH allowance, so nothing
/// bounds the batch count.
///
/// A ceiling is a SUPREMUM OVER SPLITS, not the bound for one split. For a span
/// of `S` blocks cut into batches of sizes `b_i`, the metered cost is
/// `Σ min(b_i × GRPC_MAX_MESSAGE_BYTES, DOWNLOAD_BATCH_MAX_BYTES)`. The per-batch
/// allowance binds only for `b_i > 16` (16 × 8 MiB = 128 MiB), so ANY split into
/// batches of at most 16 blocks reaches `S × GRPC_MAX_MESSAGE_BYTES`, and no
/// split exceeds it — that maximum is the ceiling. (The first correction said
/// "`batches ≤ S` makes the per-block term the smaller", which is false in the
/// direction that matters: two batches of 55 blocks cost 256 MiB, far LESS than
/// 880 MiB. The number was right for the wrong reason.)
///
/// So the re-download is at most
/// `(REWIND_DISTANCE_BLOCKS + SYNC_BATCH_BLOCKS) × GRPC_MAX_MESSAGE_BYTES` =
/// 110 × 8 MiB = 880 MiB. The forked batch IS one batch, but the byte cap is
/// checked AFTER a block is added to the tally
/// (`download_range_folding`), so the block that trips it has already crossed
/// the wire: its METERED cost is `DOWNLOAD_BATCH_MAX_BYTES + GRPC_MAX_MESSAGE_BYTES`
/// = 136 MiB, not 128. That is 1,016 MiB per rewind, and one pass's rewinds at
/// most `MAX_TOTAL_REORGS_PER_PASS × 1,016 MiB` = **29.77 GiB** of metered
/// download above the forward span — against the 11.25 GiB the original
/// derivation stated. Pinned with its derivation in
/// `extraction_policy::named_constants_at_boundary`. Still not an upper bound on
/// the WIRE: `encoded_len()` excludes gRPC framing and TLS overhead.
/// What the ceiling does NOT bound, said plainly: a fork met in a range scanned
/// UNDER the frontier (a gap upstream's shard-priority jump leaves and scans later)
/// lands the truncate below that gap and un-scans everything above it — the cap
/// still bounds the COUNT of such rewinds, and `wallet.reorg_rewind`'s `depth`
/// measures each one. Across passes the cap bounds nothing: the backoff ladder
/// bounds the RATE and [`MAX_CONSECUTIVE_REWINDING_PASSES`] the run before the
/// user is told (§4u).
pub const MAX_TOTAL_REORGS_PER_PASS: u32 = 3 * MAX_SCAN_REORGS_PER_PASS;

/// The rewinding-streak report threshold (§4u REW-1): after this many CONSECUTIVE
/// passes that each rewound — an `Ok` pass with `reorgs > 0`, or a pass stalled
/// `ChainReorg` by the cap, the storm bound or the exit belt — the controller
/// publishes `Stalled { EndpointMisbehaving }` in place of the pass's terminal
/// status, and keeps doing so after every further rewinding pass until a CLEAN
/// pass (`Ok`, no rewind) decays the streak by one — never clears it in one (the
/// maintainer's ruling). Why a count of PASSES: the per-pass
/// cap above bounds one pass's work and the backoff ladder (which a rewinding pass
/// climbs since §4u) bounds the RATE across passes, but neither tells the user
/// anything; an honest chain reorgs once, rarely twice in a row, so a run this
/// long is a server serving forks (or one behind our scan, pass after pass) and
/// the honest next step is the class's: switch servers. Six is the orchestrator's
/// proposal (§4u; **the maintainer may move it**): at the ladder's top that is an
/// hour of rewinding passes before the badge changes. Retry-forever is unchanged —
/// a `Stalled` is retried, and a server that heals is accepted on its first clean
/// pass. Counted in the controller's shared state (`sync_controller::Shared::
/// rewinding_streak`, one writer — the loop; phase-2 P2-5): born zero with the
/// controller so a relaunch forgives it, NOT zeroed by a loop start (phase-2
/// P2-6 — a `stop()`/`start()` restarts the ladder and nothing else, so the
/// host's reconnect kick and "Try now" no longer hand the endpoint a fresh
/// budget), decayed by one per clean loop pass, read by `once()` so a
/// pull-to-refresh carries the loop's judgement.
pub const MAX_CONSECUTIVE_REWINDING_PASSES: u32 = 6;

/// How many transactions the §3.3 tx-enhancement loop downloads + decrypts per sync pass
/// (memo recovery — compact blocks omit memos, so each scanned receive needs ONE
/// `GetTransaction` RPC + a decrypt). BOUNDED so a freshly-restored wallet with thousands of
/// historic receives does not fire thousands of RPCs in one burst (mobile data/battery, and a
/// long synchronous tail that would block the scan loop): the pending requests PERSIST across
/// passes, so a fixed batch per pass converges over a handful of passes while keeping each
/// pass cheap and promptly cancellable. 50 ≈ a few seconds of fetch+decrypt; a typical active
/// wallet drains its small backlog in one pass. Pinned by `named_constants_at_boundary`.
pub const MAX_ENHANCEMENTS_PER_PASS: usize = 50;

/// The UNTRUSTED-incoming confirmation depth — the conservative arm of the ZIP-315
/// `ConfirmationsPolicy` that is the spendable SSOT (`account::spendable_policy`;
/// trusted/wallet-internal change clears at 3, untrusted incoming at this depth;
/// maintainer 2026-06-15). NOT itself the spendability predicate: the single source of
/// truth for "is this note spendable" is that ONE policy, read once per
/// `get_wallet_summary()` and shared by the balance reader's `spendable` field and the
/// `spendable_ready` sync hint. A LOWER untrusted depth is a double-spend exposure, not
/// a UX win. This constant documents the depth posture; the live decision lives in
/// `account::spendable_policy`.
pub const MIN_CONFIRMATIONS: u32 = 10;

/// Circuit-isolation key for the long-lived SYNC client (§2.3): one stable key ⇒
/// one channel ⇒ one circuit for all sync RPCs (the LightdSyncEngine reuses a single
/// client). Package-name-neutral (§1.3 — nothing hard-codes the working name).
/// Broadcast uses a FRESH per-txid key (its own circuit) — never this one (inc-2d).
pub const WALLET_SYNC_ISOLATION_KEY: &str = "wallet-sync";

/// Circuit-isolation key PREFIX for a BROADCAST (§2.3 / inc-2d-3-a): each
/// `send_transaction` rides a FRESH `LightwalletdClient` whose key is
/// `{prefix}-{random}` — its own channel ⇒ its own circuit, so a broadcast is
/// unlinkable from the sync circuit AND from every other broadcast (the ServiceMode
/// per-tx-group posture, §1.7). The unique suffix is a fresh `OsRng` token, NOT the
/// txid: the txid is §5.4-sensitive and must never ride the transport credential —
/// uniqueness (a distinct circuit) is all §2.3 needs, and a random token gives it
/// without leaking which tx is on the wire. Distinct from
/// [`WALLET_SYNC_ISOLATION_KEY`] by construction.
pub const BROADCAST_ISOLATION_KEY_PREFIX: &str = "wallet-send";

/// Circuit-isolation key PREFIX for the §3.2i-2 2e-2b EPHEMERAL-DETECT poll (the TEX/ZIP-320
/// stranded/return detection). Each per-ephemeral `GetAddressUtxos` query rides its OWN fresh
/// `LightwalletdClient` whose key is `{prefix}-{random}` — its own circuit, so the endpoint
/// cannot cluster N co-timed single-address queries as ONE wallet's TEX ephemeral set (the §5
/// privacy MUST: never the sync circuit, never a shared batch — distinct from
/// [`BROADCAST_ISOLATION_KEY_PREFIX`] so a detect circuit is never confused with a send).
/// Tor-OFF caveat (§3.2i-2): on clearnet the key is a no-op — the query reaches the unlinkable
/// ephemeral from the wallet's REAL IP, so "private/unlinkable" UX claims gate on Tor-ON.
pub const EPHEMERAL_DETECT_ISOLATION_KEY_PREFIX: &str = "wallet-ephemeral-detect";

/// New-wallet birthday lag below the live tip (§3.2f) — a fresh wallet has no
/// history below ~now, and lagging the tip by the reorg depth keeps the birthday
/// off a block a reorg could later drop (the Zashi tip−100 posture; equals
/// [`REORG_MAX_BLOCKS`] by construction). The actual birthday frontier is then
/// the newest BUNDLED checkpoint ≤ this height — never a live frontier.
pub const NEW_WALLET_BIRTHDAY_LAG_BLOCKS: u32 = REORG_MAX_BLOCKS;

/// How far the judged chain height may advance past our last signing-capable
/// consensus verdict before an endpoint that withholds `consensus_branch_id`
/// stops being allowed to sign (`ironwood-nu63-support.md` §6.3, maintainer
/// decision 2026-09-06 — "degrade with age"). ONE of the grace's TWO rules;
/// the other is [`UNKNOWN_BRANCH_GRACE_SECS`], and the grace ends when EITHER
/// expires.
///
/// WHY BLOCKS: the risk being bounded is "a network upgrade may have activated
/// since we last checked", and upgrades activate at HEIGHTS — so the primary
/// rule counts them. WHY NOT BLOCKS ALONE (GRACE-1, §4p, maintainer decision
/// 2026-09-10): the blocks counted are the ones the SERVER admits. A server
/// that omits the branch AND freezes its claimed tip advances the count by
/// zero on every pass, and — with the anchor floored at our own scanned height
/// (M4) and the wallet unable to scan past what the server serves — holds the
/// grace at zero for as long as it likes: the INC-001 safeguard disarmed in
/// exactly the adversarial case. The device clock is the dimension the
/// endpoint cannot freeze, hence the second rule. (An earlier version of this
/// comment rejected a wall clock because it "would drift against the thing it
/// is protecting"; that is true of a clock INSTEAD of blocks and irrelevant to
/// a clock BESIDE them — the clock can only ever end the grace sooner.)
///
/// WHY 1,152: ~1 day at the post-Blossom 75-second target block time
/// ([`TARGET_BLOCK_SPACING_SECS`]). Short enough that an endpoint withholding
/// the field can extend a silent consensus outage by at most a day (the
/// Ironwood one ran six weeks), long enough that an honest older server that
/// does not populate the field does not break a user mid-session.
///
/// WHAT THE ENDPOINT'S HEIGHT CAN DO TO IT: the numerator is the JUDGED height,
/// `max(scanned, claimed)` (the same height the verdict is taken at), so a
/// claim below what this wallet scanned itself cannot shrink the count (§4p
/// item 4 — taken); a claim above it advances the count and ages the endpoint
/// out faster. An earlier version of this comment said under-reporting "only
/// shortens its own grace" — backwards: a low claim LENGTHENED it by holding
/// the count near zero, which is the freeze above and the reason the clock
/// rule exists.
pub const UNKNOWN_BRANCH_GRACE_BLOCKS: u32 = 1_152;

/// The post-Blossom target block spacing, in seconds — the ONE conversion
/// between the grace's two dimensions (`ironwood-nu63-support.md` §6.3; §4p
/// item 5): it derives [`UNKNOWN_BRANCH_GRACE_SECS`] from
/// [`UNKNOWN_BRANCH_GRACE_BLOCKS`], and it is what the sync surface uses to say
/// which of the two rules expires first (`SyncStatus::UpToDateUnverified`). A
/// second "75" anywhere in the grace is a second source of truth for "a day".
pub const TARGET_BLOCK_SPACING_SECS: u64 = 75;

/// The grace's CLOCK rule (GRACE-1, §4p — maintainer decision 2026-09-10): how
/// many seconds the device clock may advance past the last signing-capable
/// verdict before an endpoint that withholds `consensus_branch_id` stops being
/// allowed to sign, whatever its claimed tip did. Derived, not written down:
/// the block grace's own wall-clock equivalent (1,152 × 75 s = 86,400 s = one
/// day exactly), so "a day" has one source and the two rules describe the same
/// window on two axes. The grace ends when EITHER expires (never "both"). A
/// clock can only SHORTEN the grace, and a time the wallet cannot trust ENDS
/// it (GRACE-2, §4v): a capable time later than "now", or one of `0`, is an
/// EXPIRED clock rule — latched, fail-closed — not a reading the block rule
/// decides alone (the GRACE-1 cell a frozen-tip silent server defeated); and
/// a clock set BACK after the rule has been observed expired does not
/// re-permit until a capable verdict (`consensus_stamp::observe`'s latch).
/// Exclusive at the threshold, like the block rule.
///
/// **The stopped clock is the residual, and it is not closed** (§4v-run review
/// row 1; the maintainer's decision 2026-09-11 — documented, not built). This
/// rule measures a clock that MOVES: a clock held anywhere inside
/// `[capable_at, capable_at + UNKNOWN_BRANCH_GRACE_SECS − 1]` reads as trusted
/// and never reaches the threshold, so an attacker who runs both the
/// lightwalletd and the device's unauthenticated NTP source freezes the
/// claimed tip AND the clock and keeps signing permitted indefinitely — both
/// axes held by one party, which is what having two axes was meant to prevent.
/// Closing it needs a THIRD axis neither can freeze (monotonic uptime
/// accumulated across passes, or a high-water reading latched when the clock
/// fails to advance over N passes). No skew band is built either: the rule
/// latches on a one-second forward skew (§4v-run review row 2), which is
/// deliberate and its own owed row.
pub const UNKNOWN_BRANCH_GRACE_SECS: u64 =
    UNKNOWN_BRANCH_GRACE_BLOCKS as u64 * TARGET_BLOCK_SPACING_SECS;

/// Hard bound on how far the compact-block FS cache may run ahead of
/// scanning: disk is a shared resource on mobile and an unbounded read-ahead
/// on a full disk turns `DiskFull` into a spin (§6.1). The cache is
/// disposable by design (deleted after scan, §1.7).
pub const CACHE_MAX_BYTES: u64 = 256 * 1024 * 1024;

/// Hard ceiling on the TOTAL bytes a single block-download batch may buffer from
/// an (untrusted) endpoint before it is written to the cache (§3.2g, §4.6). The
/// byte-DIMENSION sibling of [`MAX_SUBTREE_ROOTS_PER_POOL`]'s count cap: a download
/// batch is count-bounded (the scanner-suggested `[start, end)` width, ≤
/// [`SYNC_BATCH_BLOCKS`]), but each `CompactBlock` is itself only bounded by the
/// per-message wire cap (`GRPC_MAX_MESSAGE_BYTES`, 8 MiB), so a hostile server that
/// pads block bodies could force `width × 8 MiB` (gigabytes) of resident memory and
/// cache disk from one batch — the byte dimension of "size-cap before allocating"
/// (principle 7), which the count bound alone does NOT cover (a block is ~10^5×
/// larger than a root). At this ceiling the endpoint is provably abnormal → typed
/// `Sync`, never an OOM. Set below [`CACHE_MAX_BYTES`] (a batch always fits the
/// disposable cache with margin) and well above an honest batch (≤
/// [`SYNC_BATCH_BLOCKS`] compact blocks at tens-of-KB each ≈ a few MiB; the dense
/// clamp [`SYNC_BATCH_BLOCKS_DENSE`] is currently unwired — review). Peak RAM ≈ this value (the
/// scan runs in the MAIN app, not the iOS NSE); a streaming sub-chunk insert that
/// would decouple RAM from the total is a deferred optimization.
pub const DOWNLOAD_BATCH_MAX_BYTES: u64 = 128 * 1024 * 1024;

/// Hard ceiling on the number of note-commitment **subtree roots** accepted from
/// an (untrusted) endpoint per shielded pool, per sync (§3.2g, §4.6). The
/// note-commitment trees are consensus-fixed at depth 32 with shard height 16, so
/// at most `2^(32 − 16) = 65 536` subtrees can EVER complete — any count beyond
/// this is a provable server lie. Without the cap, `get_subtree_roots`' per-message
/// idle timeout (no TOTAL cap, by design — §3.2d) lets a hostile endpoint stream
/// well-formed roots forever and OOM the sync task on mobile (the count dimension
/// of "size-cap before allocating", principle 7). Used SOLELY as the client-side
/// reject ceiling now (`collect_roots`): the request itself sends `max_entries = 0`
/// ("all entries", the proto sentinel), because passing this value (`1<<16`) as the
/// `max_entries` hint made lightwalletd's `z_getsubtreesbyindex` reject the call —
/// `limit == 2^16` is one past zcashd's accepted maximum (see `sync.rs`). The hint
/// was never the defense anyway (a malicious server ignores it); the client check
/// always was. Generous vs the current ~few-thousand mainnet reality.
pub const MAX_SUBTREE_ROOTS_PER_POOL: u32 = 1 << 16;

/// S15-F1 (ADR-0569): after this many INCREMENTAL subtree-root passes in one
/// session, a pool's next pass fetches every root from index 0 again — the full
/// verification that re-runs the height binds, the cap `Conflict` and C6 at every
/// interior index the incremental passes no longer re-serve (the plan's ledger
/// row 7). 180 passes is an hour at [`POLL_INTERVAL_SECS`] (20 s): long enough
/// that the saving stays (one full fetch costs ~4–12 s over Tor), short enough
/// that an interior inconsistency is caught well inside one session. It bounds a
/// session whose passes run back to back (a catch-up, a retry ladder at its first
/// rungs) where the time bound [`SUBTREE_ROOTS_FULL_VERIFY_SECS`] alone would let
/// hundreds of incremental passes go by; whichever comes first forces the full
/// fetch.
pub const SUBTREE_ROOTS_FULL_VERIFY_PASSES: u32 = 180;

/// S15-F1 (ADR-0569): the wall-clock bound beside
/// [`SUBTREE_ROOTS_FULL_VERIFY_PASSES`] — at least this long after a pool's last
/// full fetch (on `Inner.clock`), the next pass fetches from index 0 again. One
/// hour bounds how long an interior-index inconsistency can wait for detection on
/// a wallet that syncs rarely (a backgrounded phone polls far less often than
/// every [`POLL_INTERVAL_SECS`] = 20 s). A clock that reads EARLIER than the last
/// full fetch is untrusted and forces the full fetch too, as everywhere in this
/// crate.
pub const SUBTREE_ROOTS_FULL_VERIFY_SECS: u64 = 3_600;

/// S15-F1 (ADR-0569, amended by ADR-0570): this many `roots_start_dropped` IN A
/// ROW for one pool turn that pool's incremental fetch off for the session. A
/// drop COUNTS only when the same pass's retry from 0 was SERVED: a transport
/// fault on the `start > 0` stream next to a from-0 stream that drained whole is
/// evidence the server resets non-zero starts, while a retry that fails too says
/// only that the path is down, which is not held against incremental fetching.
/// One counted drop is a reset — over Tor, possibly one circuit — and is not held
/// against the server; a second, with no incremental write between them, would
/// otherwise make every pass of the session cost two streams. The count resets
/// only when a pass WRITES the pool from a `start > 0` serve — not when such a
/// serve is merely drained whole and then refused, or the pass fails.
pub const SUBTREE_ROOTS_DROPS_OFF: u32 = 2;

/// S15-F1 (ADR-0569): an incremental subtree-root request's `start_index` is
/// rounded DOWN to a multiple of this. In steady state the start is
/// `count − 1`, the same for every synced wallet on the chain; the terms that can
/// lower it (the reorg window, a bracket, a newly scanned completing block) say
/// roughly how far this wallet has scanned, and the rounding hides that fine
/// signal on the wire (`docs/specs/wallet-sdk.md` §5). Rounding only ever DOWN, so
/// it re-serves more, never less. 64 roots is ~5 KB on the wire — noise beside one
/// round trip over Tor.
pub const SUBTREE_ROOTS_START_GRANULARITY: u32 = 64;

// ── Send path (spec §3.1 / §5.3 / §7) ───────────────────────────────────────

/// Send proposals expire after this long (monotonic): fee and note anchors go
/// stale as the chain moves (~8 blocks at 75 s) — a resumed-from-background
/// stale proposal re-proposes with a fresh fee shown, never a stale anchor.
pub const PROPOSAL_TTL_SECS: u64 = 600;

/// Max blocks the chain may advance PAST a proposal's `min_target_height` before
/// `send` (create+sign, inc-2d-2) refuses to sign it as stale (§3.2h re-anchor
/// obligation). The audited `Builder` sets a tx's expiry to `min_target_height +
/// DEFAULT_TX_EXPIRY_DELTA` (40 blocks, `zcash_primitives::transaction::builder`)
/// and never consults the live tip — so a proposal whose chain advanced (the
/// device-suspend case: the monotonic [`PROPOSAL_TTL_SECS`] clock PAUSED while the
/// chain moved) would build a BORN-EXPIRED tx that upstream does NOT reject (it
/// errors only on a *pruned* anchor, not a merely-*stale* one). `send` therefore
/// re-anchors against `chain_height()` BEFORE signing and rejects `ProposalStale`
/// past this drift. Set to HALF the 40-block expiry window so a signed tx always
/// retains ≥ 20 blocks (~25 min at 75 s) of mining headroom — the authoritative
/// staleness gate the suspend-paused wall-clock TTL cannot provide. Boundary-tested
/// by `create_signed_reanchor_boundary_is_exact`.
pub const PROPOSAL_ANCHOR_DRIFT_MAX_BLOCKS: u32 = 20;

/// Max live (un-consumed) retained proposals per wallet (§3.2h registry bound).
/// `propose` retains an opaque token holding a `Proposal` + the originating
/// `PaymentRequest` (which carries §5.4-sensitive memo bytes); the ONLY removal is a
/// `send`-time consume, so a propose-without-send loop (a confirm screen re-quoting
/// the fee on every edit, a background-resumed re-propose) would otherwise grow
/// unbounded. The registry sweeps TTL-expired entries on every insert AND caps the
/// live set at this many, evicting the OLDEST first (the newest — the one on screen —
/// always survives). Generous for a single-user wallet (one live proposal is the
/// norm); a value an adversarial host loop cannot exceed (crypto+security review
/// fold, §3.2h). Boundary-tested by `proposal_registry_is_bounded_without_consume`.
pub const PROPOSAL_REGISTRY_MAX_LIVE: usize = 32;

/// Default broadcast jitter ceiling (uniform 0..=10 s before tx broadcast):
/// decorrelates the user's send action from the endpoint-observable
/// broadcast timing (§5.3). Host-tunable; `JitterPolicy::None` is honest
/// opt-out.
pub const BROADCAST_JITTER_MAX_MS_DEFAULT: u64 = 10_000;

/// The longest broadcast jitter window a host may configure (the 2026-10-05
/// review, F01). `create`, `create_watch_only` and `open` refuse a larger
/// `JitterPolicy::Uniform { max_ms }` with `BroadcastJitterTooLong`: the window
/// was unbounded (the bridge takes a `u32`, ~49 days), and a wait that long
/// outlasts a transaction's validity. A deadline-tagged swap deposit never waits
/// longer than [`BROADCAST_JITTER_MAX_MS_DEFAULT`] whatever the policy.
pub const BROADCAST_JITTER_MAX_MS_CEILING: u64 = 30_000;
const _: () = assert!(BROADCAST_JITTER_MAX_MS_DEFAULT <= BROADCAST_JITTER_MAX_MS_CEILING);

/// `propose_shield` returns `None` below this transparent balance
/// (0.001 ZEC): don't burn a ZIP-317 fee shielding dust. The
/// upstream-converged value on BOTH Zodl platforms (§1.7); sanity-checked
/// against current marginal fees at W2+.
pub const SHIELDING_THRESHOLD_ZAT: i64 = 100_000;

/// The 2e-2b-v-2 manual ephemeral-SWEEP recovery floor (maintainer — recover EVERYTHING sweepable,
/// NOT the convenience `SHIELDING_THRESHOLD_ZAT`). A minimal 1 zat: the explicit `propose_shielding`
/// gate (`balance.total() >= threshold`) then admits any ephemeral with ≥ 1 zat recognised, and the
/// REAL floor is delegated to the engine's ZIP-317 fee economics (a genuinely-uneconomic ephemeral
/// reports `Change(InsufficientFunds)` ⇒ `Ok(None)` — the change strategy cannot cover the marginal
/// fee). This is what lets the sweep recover a sub-0.001-ZEC late return (above the ZIP-317 marginal
/// fee) the convenience shield threshold would re-strand. NOTE (money-red-team, engine-verified):
/// the engine excludes EACH UTXO ≤ the marginal fee from input selection BEFORE aggregation, so true
/// sub-marginal-fee dust is NOT recoverable here (it is un-spendable wallet-wide, never a wasteful tx);
/// the aggregation benefit is UTXOs EACH > the marginal fee summing below the shield threshold. The
/// sweep's real edge over the automatic surface is RAW enumeration (it recovers a never-recognised late
/// return / a 2nd deposit `list_stranded` cannot see), NOT dust (§3.2i-2 2e-2b-v-2).
pub const EPHEMERAL_SWEEP_THRESHOLD_ZAT: i64 = 1;

/// History is keyset-paged (stable cursor); the page cap keeps FRB stream
/// payloads bounded — never an unbounded list across the bridge (§7).
pub const HISTORY_PAGE_MAX: u32 = 100;

// ── Send money-safety (the §3 large-amount confirm — user-error hardening atop the
//    already-audited validation; the host shows ONE deliberate confirm when a proposal
//    trips either threshold, "whichever fires first" — maintainer decision) ──────────
/// The RELATIVE large-send trigger: a proposal whose total debit (amount + fee) is at
/// least this fraction of the AVAILABLE balance (shielded spendable + transparent — the
/// pools a send draws from) is "sending almost everything" — the fat-fingered-amount /
/// drain-the-wallet case, meaningful at any wallet size. Basis points (9000 = 90%);
/// must stay STRICTLY below 10_000 (100%) so it can fire before a full drain (which the
/// `InsufficientFunds` gate already catches). Integer math only; see `classify_magnitude`.
pub const NEAR_TOTAL_SPENDABLE_BPS: u16 = 9_000;
/// The ABSOLUTE large-send trigger: a proposal whose total debit is at least this many
/// zatoshis is "objectively large" regardless of balance (the maintainer's "> ~1 ZEC" example;
/// 1 ZEC = 100_000_000 zat). A product-tunable policy backstop to the relative trigger —
/// raise it if it proves to fire on routine sends (alert-fatigue is the explicit constraint).
pub const LARGE_SEND_ABSOLUTE_ZAT: i64 = 100_000_000;

// ── Swap (spec §2.6 / §7 — compiled even when the `swap` feature is off:
//    constants are inert data; the kill layer removes CODE and traffic) ──────

/// Default slippage tolerance, 2% — the upstream ecosystem default (§1.7).
pub const SLIPPAGE_DEFAULT_BPS: u16 = 200;

/// Hard slippage ceiling (10%): a quote deviating more than this from the
/// user's own request is a drain, not a market move — the M1 user-anchored
/// sanity bound rejects it BEFORE SIGNING regardless of requested tolerance.
pub const SLIPPAGE_MAX_BPS: u16 = 1_000;

// the default must sit under the hard ceiling — pinned at compile time so a
// drive-by edit cannot invert the M1 relationship (mobile review fold)
const _: () = assert!(SLIPPAGE_DEFAULT_BPS < SLIPPAGE_MAX_BPS);

/// Cap on concurrent unexpired entries in the issued-quote registry: a
/// quote-looping integration bug (or hostile UI loop) must hit a typed wall,
/// not grow memory. 16 simultaneous live quotes is far beyond any real
/// comparison UX.
pub const MAX_ISSUED_QUOTES: usize = 16;

/// Cap on rows in the durable swap-record store (W-swap-5, #366) — bound
/// hygiene for the home surface, enforced at insert by evicting the OLDEST
/// rows. Display-only records: eviction can never lose money, only a stale
/// home row. Real concurrency is bounded far below this by the §4.4
/// one-deposit-in-flight guard (OutOfZec) and [`MAX_ISSUED_QUOTES`]
/// (IntoZec); the headroom absorbs never-dismissed lapsed rows between the
/// insert-time prunes.
pub const MAX_SWAP_RECORDS: usize = 64;

/// Bound on the OutOfZec fresh-refund-address allocation loop (§2.6 HARD-H,
/// W-swap-3-b): how many monotonic HD indices the `WalletRefundSource` will
/// burn through before giving up, when a derivation MISSES at a specific index
/// (the cryptographically vanishing ~2^-127 BIP32 invalid-child case — never an
/// exotic seed, which is rejected up front without burning any index). One miss
/// is already astronomically unlikely; 8 is "could not happen twice in the age
/// of the universe" headroom, while still bounding the loop so a (truly
/// impossible) pathological run fails closed instead of spinning the allocator.
pub const REFUND_DERIVE_MAX_ATTEMPTS: u32 = 8;

/// First swap-status poll delay; provider settle times are tens of seconds
/// to minutes — 5 s catches fast settles.
pub const SWAP_POLL_INITIAL_SECS: u64 = 5;

/// Swap-status poll backoff cap; 60 s bounds idle cost. Polling is
/// foreground-only and stops on terminal status (§7).
pub const SWAP_POLL_MAX_SECS: u64 = 60;

/// CONSECUTIVE provider-definitive `SwapNotFound` status answers before the
/// poll loop synthesizes the terminal `Failed(NotFound)` (#367 poll policy —
/// the wedge root: a re-attached provider-GC'd order otherwise polls
/// forever). 5 consecutive answers span ~75 s from attach on the §7 cadence
/// (0 s, 5 s, 15 s, 35 s, 75 s with the doubling backoff) — generous headroom
/// over a transient post-execute 404 while the provider indexes a fresh order
/// (observed sub-second in the wire fixtures; whole seconds at worst), yet
/// bounded so the honest terminal renders within ~a minute of re-attaching to
/// a dead order. Any NON-not-found result resets the run (only consecutive
/// definitive answers terminate).
pub const SWAP_NOT_FOUND_TERMINAL_POLLS: u32 = 5;

/// Default deposit deadline REQUESTED from the provider (integrator-supplied
/// by API design, §2.6 — not a platform constant; 24 h matches the
/// ecosystem-converged deposit-screen UX). Since W-swap-4-a-3 this is the
/// INTOZEC window only — an EXTERNAL user deposit from another wallet or an
/// exchange withdrawal genuinely needs a day — plus the independent
/// hostile-provider ceiling `send::clamped_deposit_deadline` fences the
/// durable §4.4 deposit tag with. An OutOfZec quote requests
/// [`SWAP_DEPOSIT_DEADLINE_OUT_OF_ZEC_SECS`] instead.
pub const SWAP_DEADLINE_DEFAULT_SECS: u64 = 86_400;

/// Deposit deadline REQUESTED from the provider for an OUTOFZEC quote (§4.4
/// W-swap-4-a-3 — the re-pricing fix). The WALLET sends this deposit
/// itself: signed inside the execute bracket, kicked to the network within
/// seconds, the drain as fallback — so the window only needs to cover
/// execute + sign + broadcast + mining (~minutes with margin; 15 min ≈ 12
/// blocks), never a human fetching an external wallet. This bound is what
/// every §4.4 deposit-tag lockout self-clears on against an honest provider
/// (the ONE-deposit-in-flight guard, a host-custody sign-miss zombie, an
/// under-funded `Stale`) — requesting the 24 h default here silently
/// re-priced all of those from minutes to a day. A miss is money-safe: the
/// hold/purge end the swap, notes recover via tx expiry, funds stay in the
/// wallet (the tracking copy says exactly that).
pub const SWAP_DEPOSIT_DEADLINE_OUT_OF_ZEC_SECS: u64 = 900;

/// IntoZec destination DETECTION window after a swap EXECUTES (§3.3b L3 / ADR-0530, IZ-1b). Pre-
/// execute a destination is watched until the QUOTE deadline (an abandoned/comparison quote retires
/// fast); at execute the swap is live and delivery follows the user's external deposit (up to the
/// `SWAP_DEADLINE_DEFAULT_SECS` deposit window) plus provider settlement, so the scoped poll keeps
/// watching this long past execute. 48 h comfortably covers a late (≈24 h) deposit + settlement; a
/// delivery that lands AFTER this window is still on-chain + engine-tracked and surfaces on the next
/// user-initiated full sync (the same honest fallback as the §3.5 Hard-kill drop) — so the bound
/// trades a vanishing tail of auto-detection for a scoped set that always shrinks back to index-0 at
/// idle. UNGATED since W-swap-5 (#366): the ungated `swap_record_store` hygiene also anchors its
/// far-future sweep to this ceiling (a record's self-lapse bound is execute-time + this window, so
/// anything beyond now + this + skew cannot have been written by a sane clock).
pub const SWAP_SETTLEMENT_MAX_SECS: u64 = 172_800;

/// Per-pass ceiling on the #368 / ADR-0527 backfill batch (`get_address_for_index`
/// registrations of allocated-but-unregistered single-use external indices, run on the
/// transparent-refresh pass). Bounds how long the engine `db` lock is held by one
/// backfill sweep — a heavy swapper's full history registers across a few passes, the
/// durable marker carrying the progress. 256 covers any realistic wallet in ONE pass
/// (one engine address insert per lifetime quote).
pub const REFUND_BACKFILL_MAX_PER_PASS: u32 = 256;

/// Safety margin subtracted from a quote's deadline before signing/broadcast:
/// enforcement is provider-relative duration + MONOTONIC elapsed (§2.6
/// clock-skew posture), and a deposit landing within the last minute is a
/// refund coin-flip, not a swap. Since W-swap-4-a-3 this generic minute is
/// only the NON-deposit margin (IntoZec execute pre-flight — no wallet-side
/// send rides it); the deposit-feeding gates use the block-time-sized
/// [`DEPOSIT_FIRST_FEED_MARGIN_SECS`] / [`DEPOSIT_EXECUTE_MARGIN_SECS`] pair
/// below (the review re-price of exactly that coin-flip).
pub const DEADLINE_SAFETY_MARGIN_SECS: u64 = 60;

/// First-feed margin for a WALLET-SENT swap deposit (§4.4 W-swap-4-a-3, the
/// review re-price): [`deposit_gate`](crate::send) refuses to sign or
/// FIRST-broadcast a deposit within this margin of its deadline. Sized in
/// BLOCK time, not wall slack: the deposit only counts when it MINES before
/// the provider deadline, and a first broadcast inside the final ~75 s block
/// interval is a refund coin-flip (~45% miss odds at the old 60 s margin) —
/// and while a provider refund has been DETECTABLE since #368 (refund-index
/// registration + the scoped watch), it still costs the user the provider's
/// refund fee and days of limbo, so the coin-flip stays worth pricing out.
/// 240 s ≈ 3.2 block intervals keeps the miss tail at the percent level
/// while spending ~a quarter of the 15-min OutOfZec window. A gate-caught
/// miss is money-safe by construction: the row stays (hold) or is purged,
/// notes recover via tx expiry, and the tracking copy's "the swap simply
/// ends and your funds stay in your wallet" fallback is then literally true.
pub const DEPOSIT_FIRST_FEED_MARGIN_SECS: u64 = 240;

/// Continue margin for a leg of a deposit group that is itself, or whose
/// predecessor is, already on the network (mined, or accepted by an endpoint) —
/// the 2026-10-05 review F01, plan §2.2. Holding it after a MINED predecessor
/// strands that output on the wallet's one-time address (the swap fails, the funds
/// stay the user's, one sweep fee); sending late costs a provider refund fee and
/// parks the funds with the provider for days; sending in time completes the
/// swap. A JUDGMENT, stated to be tuned: a late deposit is taken to cost about
/// twice a strand, which puts the break-even at even odds, and the upside of a
/// completed swap (which would lower the margin) is left out. A leg sent with `m` s
/// of effective time mines in time with probability about `1 − e^(−m/75)` (blocks
/// ~75 s apart); even odds is `75·ln 2 ≈ 52 s`, and up to
/// [`GRPC_UNARY_TIMEOUT_SECS`] of the call can follow the check: hold below about
/// 82 s, rounded up to 90 s.
pub const DEPOSIT_CONTINUE_MARGIN_SECS: u64 = 90;
const _: () = assert!(DEPOSIT_CONTINUE_MARGIN_SECS < DEPOSIT_FIRST_FEED_MARGIN_SECS);
const _: () = assert!(GRPC_UNARY_TIMEOUT_SECS < DEPOSIT_CONTINUE_MARGIN_SECS);

/// Execute-time pre-flight margin for a DEPOSIT-carrying (OutOfZec) quote
/// (§4.4 W-swap-4-a-3): the `SwapService` refuses `execute` unless the quote
/// has at least this long to live — [`DEPOSIT_FIRST_FEED_MARGIN_SECS`] of
/// mining room the feed gates will demand, plus ~one minute of ZK-prove +
/// kick headroom. STRICTLY LARGER than the feed margin on purpose: an
/// execute admitted with less would durably enqueue a deposit whose own sign
/// gate is already (or imminently) Expired — a swap dead on arrival behind a
/// live tracking screen. IntoZec keeps [`DEADLINE_SAFETY_MARGIN_SECS`] (no
/// wallet-side deposit; the user's external transfer owns the window).
pub const DEPOSIT_EXECUTE_MARGIN_SECS: u64 = 300;

/// Wall-clock plausibility floor (unix seconds) below which the device clock is
/// treated as UNSYNCED and a swap-deposit deadline cannot be time-checked
/// (inc-2d-swap-a / §4.4). A just-booted phone before NTP, or a reset clock, reads
/// near the epoch — feeding a deposit to a possibly-dead quote by a clock that says
/// "1970" is wrong, and DELETING it by such a clock is equally wrong. So a deposit on
/// a sub-floor clock WAITS (never fed, never deleted) until the clock syncs; the
/// resubmission net then resumes the normal live/lapsed decision. This is the durable
/// net's honest degradation; the strict pre-flight is the monotonic `SwapService`
/// dual-gate (inc-2d-swap-b). 2023-11-14 — comfortably before any real swap could
/// have been quoted, after every reset/epoch value.
pub const CLOCK_PLAUSIBILITY_FLOOR_SECS: u64 = 1_700_000_000;

/// §3.2i-2 2e-2b ephemeral-detect bounded-lookback window, in BLOCKS. An engine-owned ephemeral
/// transparent address (a TEX/ZIP-320 tx0 unshield target) is polled by the detect only if it was
/// RESERVED (exposed) within this many blocks of the chain tip — the "recent return-window" bound
/// (the -review scope correction) that keeps the per-pass circuit count + endpoint-visible
/// fingerprint BOUNDED instead of growing one-per-send for the wallet's life. Passed to the engine
/// `get_ephemeral_transparent_receivers` as `exposure_depth` (the engine applies the SQL window:
/// `exposed_at_height > chain_tip − this`). Sized to the same ~48 h exchange settlement/return
/// horizon as a swap delivery (cf. `SWAP_SETTLEMENT_MAX_SECS`, which is swap-feature-gated so it is
/// deliberately NOT referenced here) over a CONSERVATIVE 60 s/block floor (real Zcash spacing ≈ 75 s,
/// so the window over-covers): 172_800 / 60 = 2_880 blocks ≈ 2.5 d of real blocks. COMPLETENESS-vs-
/// FINGERPRINT TRADE (explicit, §3.2i-2): a return arriving to a zero-recognised-balance ephemeral
/// AFTER this window is not auto-detected until the funded-re-inclusion of 2e-2b-ii (which adds the
/// engine recognised-balance reader the re-inclusion and the skip-funded battery rule share);
/// widening the window discloses more of the wallet's historical ephemeral set to the endpoint.
pub const EPHEMERAL_DETECT_LOOKBACK_BLOCKS: u32 = 2_880;

/// The passive-DETECT per-address probe-backoff FLOOR (#334, §3.2i-2 2e-2b-vi contract item 2 — the
/// #294/#301 deferral): the first re-probe delay applied to an in-window ephemeral a pass queried and
/// found still COLD (empty `Ok(vec![])` or a transport fault). Doubles on each consecutive cold probe
/// up to [`EPHEMERAL_DETECT_BACKOFF_MAX_SECS`]. 3× [`POLL_INTERVAL_SECS`]: barely touches a
/// freshly-reserved ephemeral in an ACTIVE two-step (it funds + is skip-funded within a block or two),
/// while quickly decaying the wasted per-pass query on a leaked / never-funded / censored one. Applied
/// to a MONOTONIC in-memory battery timer (not the engine's persisted wall-clock `next_check_time`).
pub const EPHEMERAL_DETECT_BACKOFF_INITIAL_SECS: u64 = 60;

/// The passive-DETECT per-address probe-backoff CAP (#334): the steady-state re-probe cadence for a
/// persistently-cold in-window ephemeral (≈ 1 probe / 10 min vs 1 / 20 s ≈ 30× fewer queries + wakeups).
/// A deliberate PRODUCT surfacing-latency budget for the PASSIVE stranded-recovery surface — the
/// worst-case ADDED latency before a cold ephemeral that later receives funds is detected — acceptable
/// because (a) the detect is a recovery/reap surface, not the spendable-balance path; (b) once funded,
/// skip-funded takes over the instant the balance is recognised; (c) the ACTIVE manual sweep
/// (`sweep_ephemeral_funds`) bypasses this backoff and always probes fresh over a WIDE window; (d) each
/// probe rides a fresh isolated circuit, so a transient censor is retried next window. The value
/// COINCIDES with [`SYNC_BACKOFF_MAX_SECS`] but is chosen INDEPENDENTLY (a money-surface latency budget,
/// not a network-retry ceiling) — do NOT alias them. Measured in monotonic ACTIVE time: on a frequently
/// suspended phone the wall-clock span can exceed 10 min (the clock freezes in deep sleep), still
/// money-safe (funds durable on-chain; the manual sweep is the always-fresh escape hatch).
pub const EPHEMERAL_DETECT_BACKOFF_MAX_SECS: u64 = 600;

// The floor must not exceed the cap (a drive-by edit inverting them would clamp every first backoff
// straight to the cap). Pinned at compile time — the crate's self-checking-constant convention.
const _: () = assert!(EPHEMERAL_DETECT_BACKOFF_INITIAL_SECS <= EPHEMERAL_DETECT_BACKOFF_MAX_SECS);
// And the cap must stay FAR below the point where `jittered_delay`'s `interval.as_nanos()/2` (u128)
// would truncate on its `as u64` cast (~1170 years). A backoff ceiling anywhere near a year is itself
// a bug, so pin it under one year — astronomical margin over the real 10-minute value, and it forecloses
// the truncation entirely.
const _: () = assert!(EPHEMERAL_DETECT_BACKOFF_MAX_SECS <= 60 * 60 * 24 * 365);

/// The 2e-2b-v-2 manual ephemeral-SWEEP per-invocation address CEILING. Unlike the automatic detect
/// (windowed to `EPHEMERAL_DETECT_LOOKBACK_BLOCKS`), the MANUAL sweep enumerates ALL reserved
/// ephemerals — the completeness gap it closes is a LATE return long past the detect window — so it
/// would otherwise fan out one circuit-isolated query + sweep tx per ephemeral the wallet ever
/// reserved. This caps the per-invocation work (with the `RESUBMIT_BROADCAST_BUDGET_SECS` deadline) so
/// a heavy wallet does not open an unbounded burst; the summary's `truncated` count honestly tells the
/// user to re-run (the §"no silent caps" rule). Sized FAR above the realistic stranded count (0–1) and
/// the ZIP-320 in-flight gap-limit ceiling (10), so a normal recovery is never truncated.
pub const EPHEMERAL_SWEEP_MAX_ADDRS: usize = 64;

/// Defense-in-depth (§4.6) per-ephemeral UTXO-record cap for `recognise_ephemeral_outputs` (SHARED by
/// the automatic detect AND the 2e-2b-v-2 manual sweep). A hostile/buggy endpoint can return a flood of
/// (phantom) records paying our queried ephemeral — up to the 8 MiB gRPC frame, ~tens of thousands —
/// to amplify the put-loop (held under the db lock), a later sweep's prove, and permanent DB bloat
/// (no fund loss — the destination is always our own pool — but a multi-minute UI freeze + bloat). A
/// LEGITIMATE one-time address holds at most a handful of UTXOs (a return + a rare 2nd deposit), FAR
/// below this, so an honest reply is never truncated; the excess is a §4.6 boundary refusal (counted
/// rejected, §5.4 counts-only). With `EPHEMERAL_SWEEP_MAX_ADDRS` this bounds a sweep at ≤ 64×64 inputs.
pub const EPHEMERAL_RECOGNISE_MAX_UTXOS: usize = 64;

/// Defense-in-depth (§4.6) cap on the engine PUTS one scoped UTXO poll makes
/// (`transparent::refresh_account_transparent_scoped_known` — index-0 plus the active swap
/// destinations, S7 S1). Every returned record is still VALIDATED (CPU work, done under the wallet
/// db lock the poll already holds — cheap next to a put) and every validated match still
/// attributes its swap destination; only the puts — each a write under that lock, each a
/// permanent row — stop at the cap. NEW outputs are put first (swap destinations before index-0);
/// outputs the engine already stores unspent come last, so stored dust can never starve a new
/// receive. A pass whose NEW outputs exceed the cap is reported `truncated` (apart from
/// `rejected`: an honest high-UTXO address is not a lying server); the rest land next pass. Sized
/// well above any real shape: a personal receive address does not collect over a thousand new
/// UTXOs between two polls, so the bound caps the per-pass lock time and bloat rate a flooding
/// endpoint can buy without truncating an honest wallet.
pub const SCOPED_UTXO_POLL_MAX_PUTS: usize = 1024;

/// The engine's ZIP-320 EPHEMERAL-address gap limit (§3.2i-2 2e-2b-v-3): the maximum number of
/// reserved-but-not-yet-on-chain-used ephemeral indices the engine permits OUTSTANDING before
/// `reserve_next_n_ephemeral_addresses` refuses with `ReachedGapLimit` (the round-2 #7 ceiling,
/// folded to [`WalletError::TexSendLimitReached`](crate::WalletError)). This MIRRORS the engine
/// default (`zcash_keys::keys::transparent::gap_limits` — the ephemeral gap is 10) and is the
/// `limit` denominator the [`ephemeral_reservation_pressure`](crate::Wallet::ephemeral_reservation_pressure)
/// gauge reports so the host can warn "N of 10 one-time-address slots in use" BEFORE a TEX send
/// parks. It is an OBSERVABILITY denominator only — never a money gate (the engine remains the SSOT
/// that actually enforces the ceiling at reservation time); if a future engine changes the default,
/// the worst case is a cosmetically-off ratio, never a fund-safety regression. §5.4: a count, loggable.
pub const EPHEMERAL_GAP_LIMIT: u32 = 10;

/// §3.2i-2 2e-2b-vi (#315 slice 2) — the PROVABLE-DEADNESS margin for the Mechanism-A reclaim: a
/// reserved ephemeral index is a safe reclaim target ONLY once `tip − exposed_at_height` reaches
/// this many blocks. The engine-exact arithmetic (crypto+security review, both derivations
/// independently landing here; verified at the pinned source):
/// - a reservation stamps `exposed_at_height = the reserve-time CHAIN TIP` (`H`), NOT the tx0's
///   target height (`zcash_client_sqlite` `transparent.rs` `SET exposed_at_height = chain_tip`);
/// - the tx0's target is `chain_tip + 1 = H+1` (`zcash_client_sqlite` `wallet.rs`), and the audited
///   `Builder` sets its expiry to `target + DEFAULT_TX_EXPIRY_DELTA` (40 blocks) — so the LAST block
///   the tx0 can mine is `H + 41`, one block ABOVE `exposed + 40`.
///
/// "Past-expiry AND buried beyond the deepest reorg the wallet auto-recovers ([`REORG_MAX_BLOCKS`])"
/// therefore requires `tip ≥ (H + 41) + REORG_MAX_BLOCKS`, i.e. `tip − exposed ≥ 41 +
/// REORG_MAX_BLOCKS`. So the margin is `(DEFAULT_TX_EXPIRY_DELTA + 1) + REORG_MAX_BLOCKS` = 141
/// blocks (~3 h at 75 s) — a one-time cost per stuck window. (The earlier `40 + REORG_MAX_BLOCKS`
/// = 140 was off by one: at the boundary a maximal `REORG_MAX_BLOCKS` reorg re-enabled the tx0 for
/// exactly its expiry block, delivering only `REORG_MAX_BLOCKS − 1` of burial.) Below this margin an
/// index might still hold a LIVE, mineable tx0 (a legitimate exchange deposit in flight), and
/// self-minting on it could race + cancel that deposit — so the reclaim NEVER targets an index
/// younger than this (the deadness gate; the double-deposit-free property holds ONLY under it).
///
/// The `40` is NOT hard-coded: this DERIVES from the audited `Builder`'s own
/// [`DEFAULT_TX_EXPIRY_DELTA`](zcash_primitives::transaction::builder::DEFAULT_TX_EXPIRY_DELTA), so an
/// upstream change to the expiry delta auto-recomputes the margin to the still-correct value (the
/// `(delta + 1) + reorg` derivation is general for any delta) — and trips the `== 141` boundary pin in
/// `named_constants_at_boundary` + the scenario pin in `reclaim::tests`, forcing conscious re-review.
pub const EPHEMERAL_RECLAIM_DEADNESS_BLOCKS: u32 =
    (zcash_primitives::transaction::builder::DEFAULT_TX_EXPIRY_DELTA + 1) + REORG_MAX_BLOCKS;

/// §3.2i-2 2e-2b-vi (#315 slice 2) — the self-mint amount the Mechanism-A reclaim pays from the
/// wallet's OWN shielded pool to the highest provably-dead ephemeral address. It must clear the
/// ephemeral SWEEP's economic floor (the sweep drops any UTXO ≤ `MARGINAL_FEE` = 5 000 zat, plus
/// the sweep-back tx's own fee) so the minted output is later recoverable, never stranded as
/// unsweepable dust — so this is deliberately NOT "dust". Set to 50 000 zat (0.0005 ZEC), ~10×
/// the marginal fee: comfortably economic, small enough that the round-trip's honest cost is just
/// the two txs' fees (~4 marginal fees), and the principal returns to the wallet via the sweep.
pub const EPHEMERAL_RECLAIM_MINT_ZAT: u64 = 50_000;

/// §3.2i-2 2e-2b-vi (#315 slice 1) — the per-intent cap on EPHEMERAL-RESERVING create attempts a
/// queued TEX (ZIP-320) intent may make before the drain PARKS it (`Prepared::CappedParked`) instead
/// of re-proposing. Every drain attempt of a two-step reserves a FRESH ephemeral index inside the
/// engine's `create_proposed_transactions`, and the engine NEVER un-reserves one — not on tx0
/// expiry (`find_gap_start` advances only on a MINED first-use) and not on a create fault (the
/// reservation is committed BEFORE tx construction and deliberately not rolled back). So an
/// unbounded re-propose loop (the #310 requeue exits, or a persistent create fault retried every
/// poll pass) lets ONE stubborn intent leak the whole [`EPHEMERAL_GAP_LIMIT`] window by itself and
/// brick TEX sends for the account. The cap bounds a single intent's worst-case leak to this many
/// slots (≪ the gap limit), preserving window headroom for other sends and for the slice-2 reclaim.
///
/// PER-INTENT and counted only on ephemeral-reserving attempts, so it can never pre-deny a fresh
/// legitimate send (a new intent starts at 0) and never touches a single-step send (which reserves
/// nothing). A capped intent is PARKED, honest, and user-recoverable (visible `paused` via
/// `list_parked_sends`, cancellable, and resumable via `retry_parked_send`) — never auto-dropped.
/// Availability-only: the cap fires BEFORE propose/create, so no note is ever spent by it.
pub const MAX_TEX_REPROPOSE_ATTEMPTS: i64 = 3;

/// Named max length for EVERY provider-supplied string (ids, addresses,
/// symbols, txids) — bounds-checked before DTO entry (review m1); covers all
/// real chain address formats with slack, refuses payload smuggling.
pub const PROVIDER_STR_MAX_BYTES: usize = 256;

/// Max significant digits accepted in a provider decimal-amount string
/// (review m1: finite/in-range BEFORE DTO entry). 27 digits cover every
/// real asset supply at full precision; the bps check itself uses CHECKED
/// arithmetic throughout (rescaling to a common scale can lift values near
/// i128::MAX — overflow ⇒ fail-closed reject, see `decimal_within_bps`).
/// No floats on a funds path, ever.
pub const SWAP_DECIMAL_MAX_DIGITS: usize = 27;

// ── Seed seal (spec §4.2a) ───────────────────────────────────────────────────

/// Validation cap on the sealed mnemonic phrase: 24 BIP39 words — including
/// multibyte wordlists (Japanese ≈ 3 bytes/char) plus separators — sit well
/// under 1 KiB; anything larger inside a seal blob is corruption, and the cap
/// bounds the plaintext parse before it allocates (§4.6 discipline).
pub const SEAL_MNEMONIC_MAX_BYTES: usize = 1_024;

// ── Config bounds (spec §2.3) ────────────────────────────────────────────────

/// Cap on a host-supplied SOCKS5 proxy address string (`host:port`): a DNS
/// name maxes at 253 bytes + `:65535` — 256 leaves headroom while refusing
/// an unbounded string from a buggy host at the config door (§4.6
/// size-cap-before-use discipline at the FFI boundary).
pub const SOCKS_ADDR_MAX_BYTES: usize = 256;

/// Cap on a host-supplied light-server auth header VALUE (§2.3
/// `WalletConfig::endpoint_auth`). Real credentials are short — a JWT runs a few
/// hundred bytes, an opaque API key a few dozen — and 4096 is comfortably above
/// any of them while refusing the mis-paste that would otherwise be accepted at
/// the config door and then huffman-encoded into h2 buffers nothing zeroizes, on
/// every request, past the server's `SETTINGS_MAX_HEADER_LIST_SIZE`. Same
/// §4.6 size-cap-before-use discipline as `SOCKS_ADDR_MAX_BYTES` above.
pub const ENDPOINT_AUTH_VALUE_MAX_BYTES: usize = 4096;

/// Cap on a light-server auth header NAME (ADR-0568): since a user may type
/// one for their own server, the name is user input like the value. Real
/// gating headers are under 32 bytes (`x-api-key`, `authorization`); 64 leaves
/// room while capping the bridge input and the aux cell before the validator
/// sees them — the same §4.6 size-cap-before-use discipline.
pub const SYNC_SERVER_AUTH_HEADER_MAX_BYTES: usize = 64;

// ── Lifecycle (spec §3.3) ────────────────────────────────────────────────────

/// Lock-file name inside `db_dir` for the exclusive single-writer lock.
/// Deliberately package-name-neutral (§1.3: nothing hard-codes the working
/// name in storage artifacts). The file itself is ephemeral — the LOCK is an
/// OS advisory lock released on process death, never a stale-able marker.
pub const WALLET_LOCK_FILE_NAME: &str = ".wallet.lock";

// ── Two-phase provisioning store (spec §6.3) ─────────────────────────────────
//
// On-disk artifacts inside `db_dir`. All package-name-neutral (§1.3). The
// SOURCE OF TRUTH for "is there a usable wallet here" is the COMPLETION MARKER,
// never any file's mere existence (the §6.3 create-vs-open guard: a 0-byte or
// truncated `wallet.db` must never masquerade as a fresh wallet). The marker is
// written LAST, atomically (`create_new` ⇒ CAS, never clobbers an existing
// wallet); the manifest's presence marks a recoverable provisioning REMNANT.

/// The SQLCipher wallet database (encrypted at rest under the sealed DB key).
pub const WALLET_DB_FILE_NAME: &str = "wallet.db";

/// The disposable compact-block cache (§3.2e, W3-inc-2c-iv-b) — a SEPARATE
/// SQLCipher DB under the SAME `db_key`, NOT part of the §6.3 "usable wallet"
/// contract (a missing/corrupt cache is re-created, never a wallet failure).
/// Package-name-neutral (§1.3), like the other `db_dir` artifacts.
pub const BLOCK_CACHE_DB_FILE_NAME: &str = "block-cache.db";

/// The temp path the rescan rebuild (ADR-0534) writes the fresh, lower-birthday
/// data DB to BEFORE the single atomic rename over `wallet.db`. A stale copy left
/// by an interrupted rescan is INERT — same-ciphertext, carries no completion
/// marker, is never opened — and is cleared on the next rescan. SEAM NOTE (like
/// the `write_atomic` `*.tmp` files): the §3.3 WIPE sweep must include this so a
/// seized device can't yield a partially-rebuilt wallet DB.
pub const RESCAN_TMP_DB_FILE_NAME: &str = "wallet.db.rescan-tmp";

/// Sealed seed blob (§4.2a) — present only in `SealedKeychain` mode.
pub const SEED_SEAL_FILE_NAME: &str = "seed.seal";

/// Sealed DB-key blob (§4.3) — present in BOTH persistence modes (the DB is
/// unconditionally encrypted; the key is random, never seed-derived, so it
/// MUST be sealed at rest regardless of seed persistence).
pub const DBKEY_SEAL_FILE_NAME: &str = "dbkey.seal";

/// Wrap-key locator (§4.3a) — the vault custody artifact for the ONE `SealKey`
/// that seals both blobs above.
pub const WRAP_ARTIFACT_FILE_NAME: &str = "wrap.artifact";

/// Provisioning manifest: `[ver:1][network:1][persistence:1]`. Its presence
/// marks a recoverable REMNANT; it is written durably BEFORE the DB so a
/// remnant always implies the seal artifacts are on disk (§6.3 repair contract).
pub const MANIFEST_FILE_NAME: &str = "wallet.manifest";

/// Completion marker — created `create_new` (CAS) as the LAST provisioning
/// step. Its presence (with the manifest) is the ONLY thing that means
/// "openable wallet" (§6.3). Dotfile so it sorts with the lock.
pub const COMPLETE_MARKER_FILE_NAME: &str = ".provisioned";

/// FR-14 wipe breadcrumb — written into `db_dir` by `store::destroy` AFTER a
/// genuine keychain sever (`severed > 0`) and BEFORE the file deletes. Its
/// presence on a re-run means THIS namespace's custody was ALREADY severed, so a
/// now-empty purge is expected (not a namespace mismatch) and the verify-real-sever
/// guard is skipped — letting an interrupted wipe AUTO-converge. Swept by the
/// `remove_dir_all` it precedes; a dotfile so it sorts with the lock + marker.
pub const WIPE_COMMITTED_FILE_NAME: &str = ".wipe-committed";

/// Manifest format v1 — `[ver][network][persistence]`, the pre-#357 layout. Still
/// READ (a wallet provisioned before #357 keeps it); its ABSENT seed-source byte
/// reads as `ManifestSeedSource::Unknown` — the conservative "repair never stamps"
/// arm, byte-identical to pre-#357 behaviour.
pub const MANIFEST_VERSION_V1: u8 = 0x01;

/// Manifest format v2 (#357) — `[ver][network][persistence][seed_source]`. Adds
/// the seed-source provenance byte so `store::repair` can stamp a
/// Generate-over-Generate remnant (§3.2f). Append-only: a new version gets a new
/// byte and its own exact length; an unknown version is a typed reject, never
/// guessed at (§4.6 validate-before-use).
pub const MANIFEST_VERSION_V2: u8 = 0x02;

/// Manifest format v3 (#397, spec §3.7 D4) — SAME 4-byte layout as v2, written
/// ONLY by a WATCH-ONLY provision (`persistence = WatchOnly (0x02)`,
/// `seed_source = WatchOnly (0x03)`; any other byte pair under this version is
/// corruption — no mixed states). The version byte doubles as a CAPABILITY
/// GATE: a pre-#397 binary reading a watch-only store fails its version
/// dispatch TYPED (`StoreCorrupt` — honest "this build cannot open this
/// wallet"), never a half-open that later hunts a seed that does not exist.
/// Spending wallets KEEP writing v2.
pub const MANIFEST_VERSION_V3: u8 = 0x03;

/// The manifest version THIS build writes at a SPENDING-wallet provision
/// (watch-only provisions write [`MANIFEST_VERSION_V3`] — the capability gate).
pub const PROVISION_MANIFEST_VERSION: u8 = MANIFEST_VERSION_V2;

/// Exact v1 manifest length (`[ver][network][persistence]`).
pub const MANIFEST_LEN_V1: usize = 3;

/// Exact v2 manifest length (`[ver][network][persistence][seed_source]`).
/// `read_manifest` dispatches on the version byte, then demands the EXACT length
/// for that version — a length matching neither is corruption, rejected before
/// parse (§4.6 size-cap-before-use).
pub const MANIFEST_LEN_V2: usize = 4;

/// Exact v3 manifest length — same shape as v2 (the version byte, not the
/// length, is what gates capability).
pub const MANIFEST_LEN_V3: usize = 4;

/// #397 (§4.6 SIZE-CAP-BEFORE-ALLOC; review security M3): the inbound UFVK
/// string bound, enforced INSIDE `derivation::decode_ufvk` before the upstream
/// parser's length-proportional collect. A real `uview…` encoding is ~300-600
/// chars; 4 KiB leaves generous headroom for any future receiver-set growth
/// while a pathological clipboard paste (the unbounded vector — QR is
/// physics-capped) rejects typed with zero allocation. The same discipline as
/// `SEAL_MNEMONIC_MAX_BYTES` / `PAYMENT_URI_MAX_BYTES`.
pub const UFVK_MAX_BYTES: usize = 4_096;

/// SDK-owned provisioning schema version, recorded in the `wallet_provisioning`
/// sentinel row INSIDE the encrypted DB (distinct from zcash_client_sqlite's
/// own `schemer_migrations`). Its presence on open proves the DB was actually
/// provisioned — a truncated / 0-byte `wallet.db` (even WITH the completion
/// marker present) lacks the sentinel and fails typed `StoreCorrupt`, never a
/// silent fresh wallet (§6.3 create-vs-open guard, the truncation arm).
/// Append-only: a bump comes with a migration of our own tables.
pub const WALLET_SCHEMA_VERSION: i64 = 1;

/// How long a keyed connection waits for a busy write lock before erroring
/// (`SQLITE_BUSY`). The wallet DB has TWO connections to the same file — the
/// `zcash_client_sqlite` engine (sync/propose/sign) and OUR aux connection (the
/// queued-send intent store, inc-2d-3-b-i) — in WAL mode (W-swap-4-a-4; the
/// rollback-journal era let ANY engine read window longer than this timeout fail
/// an aux COMMIT `SQLITE_BUSY` — the device blocker: a single engine
/// `progress_snapshot` measured ~6.5 s). Under WAL a reader never blocks the
/// writer, so this wait covers only WRITER-vs-WRITER contention: a tiny, rare
/// aux write must WAIT out an in-flight scan-batch commit rather than spuriously
/// fail. 5 s comfortably exceeds a single batch commit yet never hangs the
/// (blocking-pool) caller indefinitely. Aux writes use `BEGIN IMMEDIATE`, so the
/// wait is a simple lock-acquire retry, never a read-then-upgrade deadlock; a
/// wait that still expires surfaces as the RETRYABLE `StoreBusy` (never
/// `StoreCorrupt`), and the user-facing aux seams retry the whole txn
/// ([`AUX_BUSY_RETRY_ATTEMPTS`]).
///
/// The symmetry is NOT total: the ENGINE's writes ride
/// rusqlite-default DEFERRED transactions, and under WAL a deferred read→write
/// UPGRADE that races an aux commit fails `SQLITE_BUSY_SNAPSHOT` IMMEDIATELY —
/// the busy handler is not consulted, so this timeout does not cover that
/// ms-scale window, and the engine seams still fold it to their generic error
/// kinds (the #371 taxonomy follow-up). The engine's plain lock-acquire waits
/// DO honor this timeout.
///
/// This is a BLOCKING wait: while a connection waits here it still holds whatever
/// Rust-level lock the caller took — the ENGINE waiter holds `Inner::db`, the AUX
/// waiter (`queue_send`) holds the SEPARATE `Inner::aux_db` (so an aux wait never
/// parks a concurrent `balance()`/`current_address()`, which take `Inner::db`). The
/// value MUST stay an order of magnitude below the stuck-sync watchdog
/// (`SYNC_STUCK_WATCHDOG_SECS = 600`) and the gRPC timeouts — a stalled UI read is
/// acceptable, a watchdog trip is not. 5 s satisfies that with wide margin.
pub const SQLITE_BUSY_TIMEOUT_MS: u64 = 5_000;

/// Total attempts a USER-FACING aux-store operation makes when a whole
/// `IMMEDIATE` txn comes back [`StoreBusy`](crate::error::WalletError::StoreBusy)
/// (`db::with_aux_busy_retry`, W-swap-4-a-4). Each attempt already waits out
/// [`SQLITE_BUSY_TIMEOUT_MS`] inside SQLite, so 3 attempts ride out roughly
/// three engine scan-batch commits back-to-back — beyond that the store is
/// genuinely contended and the typed retryable error is the honest answer
/// (the host may retry; nothing was written — a failed COMMIT rolls back).
/// Background writers (drain marks, purge, watch sweeps) do NOT use this
/// retry: they are retry-by-design on the next resubmission pass.
pub const AUX_BUSY_RETRY_ATTEMPTS: u32 = 3;

/// Pause between [`AUX_BUSY_RETRY_ATTEMPTS`] (a blocking `thread::sleep` on the
/// spawn_blocking pool — never the async runtime). Short on purpose: the real
/// wait is SQLite's own [`SQLITE_BUSY_TIMEOUT_MS`] inside each attempt; this
/// gap only de-phases the retry from the writer it just lost to. Worst-case
/// stall per wrapped txn = `ATTEMPTS × busy_timeout + (ATTEMPTS-1) × backoff`
/// ≈ 15.5 s — and the two COMPOSITE seams (issued-quote `persist`+watch-upsert
/// and `take`+watch-mark, each two individually-wrapped txns under ONE held
/// aux guard) can stall ≈ 2× that (review fold: the pin below models the
/// ×2 so a future `ATTEMPTS` bump cannot silently breach the watchdog via the
/// composites).
pub const AUX_BUSY_RETRY_BACKOFF_MS: u64 = 250;

// A fully-exhausted user-facing aux retry must stay well inside the stuck-sync
// watchdog window — a contended store read as a stalled UI action is acceptable;
// tripping the watchdog is not. Modeled on the WORST seam: the two-txn composite
// (2× the single-txn envelope, review fold).
const _: () = assert!(
    2 * (AUX_BUSY_RETRY_ATTEMPTS as u64 * SQLITE_BUSY_TIMEOUT_MS
        + (AUX_BUSY_RETRY_ATTEMPTS as u64 - 1) * AUX_BUSY_RETRY_BACKOFF_MS)
        / 1000
        < SYNC_STUCK_WATCHDOG_SECS / 10
);

/// Clock-skew allowance on the issued-quote FAR-FUTURE legitimacy cutoff
/// (`issued_quote_store::far_future_cutoff`, W-swap-4-a-4 review fold —
/// the 3-review converged finding). The live provider ignores our requested
/// window (UPSTREAM.md), so every live IntoZec row is ceiling-clamped to
/// EXACTLY `persist_now + SWAP_DEADLINE_DEFAULT_SECS` — with a zero-margin
/// cutoff, ANY backward wall-clock step (a routine NTP correction, a user
/// fixing a fast clock) between quote and any read arm would make the row
/// read as far-future and refuse/delete a live quote. One hour absorbs real
/// clock corrections while a genuine pre-clamp ~72 h echo still sits ~47 h
/// past the cutoff — the sweep loses none of its teeth. A regress LARGER
/// than this still sweeps (money-safe: re-quote, never funds).
pub const FAR_FUTURE_SKEW_ALLOWANCE_SECS: u64 = 3_600;

/// Defensive hard cap on the durable queued-send intent table (§6.2 / inc-2d-3-b),
/// enforced at the `queue_send` enqueue door. The offline-first promise lets a user
/// queue sends with zero connectivity (compose on a plane), so this is generous
/// headroom for legitimate offline batching — a real person queues a handful, never
/// 128 — while BOUNDING the table so a buggy or hostile host cannot grow it without
/// limit. Each queued intent costs a full propose+create+broadcast on the §1.7
/// resubmission pass (inc-2d-3-b-ii), so an unbounded queue would be both a storage
/// and a per-pass compute DoS; the cap bounds the storage, and the resubmission hook
/// throttles its own per-pass work independently. Distinct from
/// [`PROPOSAL_REGISTRY_MAX_LIVE`] (the in-memory, TTL-bounded propose-token cap): this
/// bounds DURABLE money intents that survive a kill. Tested at its boundary (the
/// 128th enqueue succeeds, the 129th is typed `QueuedSendsFull`).
pub const QUEUED_SEND_INTENTS_MAX: usize = 128;

/// Total wall-clock budget for the BROADCAST phase of ONE resubmission pass
/// (inc-2d-3-b-ii-B). The resubmission hook runs under the sync controller's
/// single-writer pass guard (scanner idle), so its broadcast phase delays the NEXT
/// scan pass for as long as it holds. Each `broadcast_one` can stall up to
/// [`GRPC_UNARY_TIMEOUT_SECS`] on a black-hole endpoint, and a full
/// [`QUEUED_SEND_INTENTS_MAX`] queue would otherwise hold the guard for
/// `128 × (timeout + jitter)` ≈ tens of minutes on a bad link — stalling the user's
/// balance display (the operational-review starvation finding). This bounds it: the
/// loop stops STARTING new broadcasts once the budget is spent, leaving the rest for
/// the next pass (idempotent — a `Sent` tx just re-broadcasts, a `Queued` intent is
/// still durable). > [`GRPC_UNARY_TIMEOUT_SECS`] so at least one broadcast always fits
/// in a window (forward progress — the queue can never wedge); ≪ the watchdog window.
/// The two relationships below are pinned at compile time; the constant itself is pinned
/// in `extraction_policy::named_constants_at_boundary`.
pub const RESUBMIT_BROADCAST_BUDGET_SECS: u64 = 60;

// Forward progress: at least one broadcast (worst-case one unary timeout) must fit
// inside a budget window, else a full queue could wedge making zero progress per pass.
const _: () = assert!(RESUBMIT_BROADCAST_BUDGET_SECS > GRPC_UNARY_TIMEOUT_SECS);
// And the broadcast phase must yield the pass guard well before the stuck-sync
// watchdog window, so a budgeted resubmission never looks like a wedged scan.
const _: () = assert!(RESUBMIT_BROADCAST_BUDGET_SECS < SYNC_STUCK_WATCHDOG_SECS);
// A `Preferred` wallet's patience window (ADR-0552) must not outlast one pass's
// broadcast budget. Since phase 2 the window runs from the FIRST FAILURE of a
// run, and a pass's first failure is at the earliest one dial or RPC bound into
// it — so the pass that starts a run never reaches the switch itself. The run
// lives in the wallet's posture, not in the pass, so the NEXT pass's first dial
// finds it spent: ADR-0552's "a queued send may switch in a LATER pass". The
// queue is durable and re-broadcast is idempotent, so that is a delay, never a
// lost payment. A window LONGER than the budget would push the switch out by a
// further pass for nothing.
const _: () = assert!(TOR_PATIENCE_SECS <= RESUBMIT_BROADCAST_BUDGET_SECS);
// The budget gate is PER-GROUP (§3.2i-2 round-2 #4/#5): a group started just under the
// deadline runs to completion, so the WORST-CASE phase duration is the budget PLUS one full
// group of unary timeouts. A TEX two-step is the largest group (≤2 txs), so the overrun is
// ≤ 2·GRPC_UNARY_TIMEOUT_SECS — pin that the overrun still clears the watchdog, so a future
// timeout/group-size bump can't silently let a budgeted resubmission masquerade as a wedged scan.
const _: () = assert!(
    RESUBMIT_BROADCAST_BUDGET_SECS + 2 * GRPC_UNARY_TIMEOUT_SECS < SYNC_STUCK_WATCHDOG_SECS
);

// §3.2i-2 2e-2b: `after_synced` runs the TWO budgeted NETWORK phases back-to-back — `resubmit_queued_sends`
// then the ephemeral DETECT (`detect_ephemeral_utxos`), EACH with its OWN fresh `RESUBMIT_BROADCAST_BUDGET_SECS`
// window + a per-phase overrun for the ONE item started just under its deadline (resubmit's ≤ a 2-tx TEX
// group, the detect's ≤ one ephemeral query — each item ≤ jitter + 1·unary; this assert counts the unary
// terms; the runtime jitter is bounded by `BROADCAST_JITTER_MAX_MS_CEILING`, 30 s, and adding it per item
// still clears the window — 2·60 + 3·30 + 3·30 = 300 < 600). A THIRD phase,
// the 2e-2b-ii stranded-row REAP (`reap_stranded_rows`), runs AFTER these but is LOCAL-ONLY (a `get_tx_height`
// SQLite read per row, NO network I/O) so it is unbudgeted + omitted from this bound — an endpoint cannot
// stall it. The watchdog is torn down in `after_synced` (the hook-placement rationale), so this is a
// CONSERVATIVE documentation bound — pin that even the combined unary worst-case of the two NETWORK phases
// clears the watchdog window with room for the jitter, so a future budget/timeout bump can't silently let
// the tail masquerade as a stall.
const _: () = assert!(
    2 * RESUBMIT_BROADCAST_BUDGET_SECS + 3 * GRPC_UNARY_TIMEOUT_SECS < SYNC_STUCK_WATCHDOG_SECS
);

/// How many times the DETACHED prompt kick (`Wallet::spawn_broadcast_kick` — the FR-23-a
/// deposit kick and the FR-23-b parked authorization) may (re)broadcast one just-signed
/// group before it gives the row back to the §6.1 drain.
///
/// WHY A RETRY EXISTS AT ALL (#400 R1, the reliability HIGH). The kick fires ONCE and a
/// transport miss is swallowed to a tracing event; the durable fallback (`ReBroadcast` inside
/// `resubmit_queued_sends`) is reachable only from `after_synced`, i.e. from a CLEAN SYNC PASS.
/// A host that turns its sync policy OFF (`walletSyncPolicyProvider = false` — a data-saver /
/// org-policy toggle) issues no passes at all, so for it the fallback NEVER runs: one dropped
/// packet on the kick left a signed, note-spending transaction that no code path would ever
/// re-send. This bounded retry makes the ordinary transient miss self-heal WITHOUT a pass. It
/// is NOT a substitute for the drain — a process death still hands the row back to §6.1 — and
/// the honest copy the parked surface shows under a sync-off policy carries that residual.
///
/// 3 = one immediate try plus two retries: enough to ride a link renegotiation / a captive-portal
/// blip, few enough that a genuinely offline device stops quickly instead of holding a task
/// (and its clone of the raw group) alive for minutes.
pub const KICK_BROADCAST_MAX_ATTEMPTS: u32 = 3;

/// Base backoff between the kick's broadcast attempts, DOUBLED per retry (5 s, then 10 s).
/// Rides on top of the §5.3 per-tx jitter that `broadcast_group` already applies, so the
/// endpoint never sees a tight loop. The whole worst-case kick — every attempt stalling on a
/// black-hole endpoint for a full group — is bounded by the assert below and stays FAR inside
/// the ~50-minute tx-expiry headroom, so a retrying kick can never push a tx past expiry.
pub const KICK_BROADCAST_RETRY_BASE_SECS: u64 = 5;

// Sanity bound so the geometric term below cannot shift-overflow, and so "bounded" stays
// meaningful: a kick is a burst, never a background sender.
const _: () = assert!(KICK_BROADCAST_MAX_ATTEMPTS >= 1 && KICK_BROADCAST_MAX_ATTEMPTS <= 16);

/// Ceiling for the kick's worst-case EXECUTED wall time, asserted below. A DESIGN BOUND, not a
/// runtime deadline: nothing reads it at run time, and no round is ever suppressed by it.
///
/// #403 R7 ADDED A RUNTIME WALL-CLOCK BUDGET AGAINST THIS NUMBER, AND #407 R1 REMOVED IT. The
/// mistake is worth keeping written down, because the shape recurs: a LOOSE CEILING that a
/// compile-time assert may safely over-approximate was reused as a LIVE DEADLINE, where the same
/// number has to be *tight against a different quantity* — the transaction's remaining validity.
///
/// The numbers, which are what settles it. The audited `Builder` sets expiry to
/// `min_target_height + DEFAULT_TX_EXPIRY_DELTA` (40 blocks ≈ 50 min at 75 s), and `send`
/// re-anchors within [`PROPOSAL_ANCHOR_DRIFT_MAX_BLOCKS`], so a freshly signed tx carries a
/// GUARANTEED ≥ 20 blocks — ≥ 20 min even at the conservative 60 s/block floor this file uses
/// elsewhere, ~25 min at real spacing. A 600 s budget is 20–40 % of that, so the round it
/// suppressed was, in the overwhelming majority of cases, a round on a STILL-VALID transaction.
///
/// And it could only ever suppress on ONE path. With a `Some` deadline the `deposit_gate` after
/// the backoff re-checks the durable tag against a FRESH clock — that IS the doze case, so the
/// budget was redundant there. With `None` ([`KickOrigin::ParkedAuthorization`](crate::wallet))
/// the gate proceeds unconditionally, so the budget bit exactly where there is no §6.1 drain
/// fallback: at a sync-off or failed-start host, nothing else ever re-sends that transaction.
///
/// The trade it was making, once both sides are priced: a late round costs ONE endpoint rejection
/// (the kick writes no durable state — its tally is logged, never latched, so a rejection cannot
/// mark the row terminal). An abandoned round costs the user's payment. Firing late is cheap;
/// not firing is not.
pub const KICK_BROADCAST_EXECUTED_CEILING_SECS: u64 = 600;

// The kick's worst-case EXECUTED wall time: the GEOMETRIC backoff sum plus, per attempt, a full
// TEX two-step group of jittered unary timeouts. Pinned ≪ the expiry headroom the §6.1 drain
// assumes, so a future attempt/backoff bump cannot silently turn the kick into a tx-expiry risk.
//
// The backoff term is `BASE × (2^(MAX-1) − 1)` — the sum of `BASE << (attempt-1)` — NOT the
// `3 × BASE` this assert first used. Those agree only at `MAX == 3`, which made the guard a
// coincidence rather than a guard: at `MAX = 7` the linear form computed 575 (passing) while the
// real worst case was ~875. The whole point of this assert is to survive exactly that bump.
// The per-tx jitter term is the host-configurable CEILING (F01: an ordinary parked-authorization
// kick draws from the policy as configured, up to 30 s; a deposit's is cut to the 10 s default).
const _: () = assert!(
    (KICK_BROADCAST_MAX_ATTEMPTS as u64)
        * 2
        * (GRPC_UNARY_TIMEOUT_SECS + BROADCAST_JITTER_MAX_MS_CEILING / 1000)
        + KICK_BROADCAST_RETRY_BASE_SECS * ((1u64 << (KICK_BROADCAST_MAX_ATTEMPTS - 1)) - 1)
        < KICK_BROADCAST_EXECUTED_CEILING_SECS
);

/// Hard cap on a seal-artifact file read from `db_dir` (§4.6 hostile input —
/// these files are attacker-substitutable on a seized device). The largest is
/// the seed seal (`ENVELOPE_OVERHEAD + SEED_PT_MAX` ≈ 1.3 KiB); 8 KiB leaves
/// headroom for a future envelope without admitting megabyte garbage. The
/// envelope's own size window (`open_envelope`) is the exact check; this is the
/// pre-allocation bound.
pub const SEAL_FILE_MAX_BYTES: usize = 8 * 1024;

// ── The sync-server picker (`docs/specs/sync-server-picker.md` §2) ──────────

/// Most servers a host may OFFER in `WalletConfig.sync_servers`. A picker is a
/// short list; eight is more than any host has asked for (the reference
/// catalog has two) and bounds the config's allocation from a hostile-looking
/// host DTO before anything is validated.
pub const SYNC_SERVERS_MAX: usize = 8;

/// Longest `SyncServerId` slug. Ids are slugs, never display strings; 32 holds
/// `example-gated-mainnet-fallback` with room, and bounds the aux cell a
/// `predefined` row stores.
pub const SYNC_SERVER_ID_MAX_BYTES: usize = 32;

/// Longest `SyncServer` label. A label fits one picker row on a 320 dp screen
/// at 2× text scale below this; longer is a paragraph, not a name.
pub const SYNC_SERVER_LABEL_MAX_BYTES: usize = 64;

/// Longest custom endpoint URL accepted from a user (and the bound on a
/// `custom` aux cell read back). The classic browser URL bound; a hostname is
/// at most 253 bytes, so anything near this is not a server — and it caps what
/// a text field can hand `LightServerEndpoint::new` (§4.6 size-cap first).
pub const SYNC_SERVER_URL_MAX_BYTES: usize = 2048;

/// The reachability probe's whole budget (`GetLightdInfo` through the wallet's
/// own TorPolicy). Above the measured cold TLS+h2 connect to both reference
/// servers (≤ 0.35 s, `docs/plan/probes/lightwalletd-responsiveness.*`) by two
/// orders, below the point a user gives up on a spinner. Distinct from
/// `GRPC_UNARY_TIMEOUT_SECS`, which is a scan budget, not a user's wait.
pub const SYNC_SERVER_PROBE_TIMEOUT_SECS: u64 = 15;

/// Isolation-key prefix for the reachability probe's client. A probe is ONE
/// `GetLightdInfo` to a server the user is considering; its circuit must not
/// be the sync circuit (an exit that saw the probe must not be able to tie it
/// to this wallet's sync session) nor a broadcast circuit. Random suffix per
/// probe, like `BROADCAST_ISOLATION_KEY_PREFIX`.
pub const SYNC_SERVER_PROBE_ISOLATION_KEY_PREFIX: &str = "sync-server-probe";
