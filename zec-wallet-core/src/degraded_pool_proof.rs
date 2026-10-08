//! **T0-1b — the independent PROOF half.** A degraded Ironwood pool is
//! invisible in a shipped build.
//!
//! Contract: `docs/plan/production-readiness-phase-1.md` §4c, item T0-1b —
//! floor D1–D8 plus the tail "The Sapling/Orchard question, DECIDED"
//! (D9–D11). Written blind under §2.1 IT-1/IT-2a/IT-3/IT-6 of
//! `docs/plan/production-readiness.md`: the author of this file has not read
//! the implementation and may not edit it.
//!
//! ## What is observed (seam S1)
//!
//! Every row observes ONE value: the [`SyncStatus`] a [`SyncController`]
//! publishes on `subscribe()` after a pass — the value `WalletState.sync`
//! mirrors and the FFI streams. Never `SubtreeRoots::outcome()`, never
//! `SyncPass.root_outcomes`, never a log (D2, D3). Those are the seams where
//! the distinction is already known, and a test that reads them proves
//! nothing about visibility.
//!
//! ## What is driven (seam S2)
//!
//! `Wallet::controller_over(client)` — the production engine's pass with the
//! endpoint injected, returning the controller whose status is the surface.
//! It does not exist at the commit this file was written against
//! (`a02a33e6`), so this file does not compile at HEAD; the red-first
//! artifacts were recorded through a temporary, uncommitted shim in
//! `wallet.rs` that wraps `Wallet::sync_once` in a `SyncEnginePort` exactly as
//! S2 describes (the shim's text is in the test author's report).
//!
//! ## Distinguishability, never a name (S3, S4)
//!
//! The shape of the degraded datum is the implementer's call, and the
//! Sapling/Orchard datum is a `StallReason` variant whose NAME this file does
//! not know. So no row here names a variant. A row asserts that a status
//! **says degraded** — it is not a healthy terminal (`UpToDate` /
//! `UpToDateLimited`), not a transient (`Idle` / `Connecting` / `Scanning` /
//! `Offline`), and not one of the five `StallReason`s that exist today, each
//! of which would be a specific lie (dead link / Tor / disk / reorg / local
//! fault) — and that two degraded statuses are UNEQUAL where the contract
//! requires two sentences. The adjudicator refines these to the variant's
//! name at the join.
//!
//! ## The wallet every row drives, and why it is shaped this way (IT-10)
//!
//! The shipped pass is `fetch_tip → fetch roots → bind → put → record tip →
//! scan whatever the queue holds → terminal status`. Two shapes for the
//! fixture wallet were tried and measured wrong before this one:
//!
//! 1. **No account.** Assumed to scan nothing; false. `record_chain_tip`
//!    queues a `ChainTip` range from the tip shard's start — the Sapling
//!    root's own completing height, 558,822 — to the tip, and nothing bounds
//!    it (the pass asked for `558822..=558921`, the first of ~29,000
//!    batches). Upstream's lower bound is `max(shard_start, wallet_birthday)`
//!    (`zcash_client_sqlite-0.22.0/src/wallet/scanning.rs:649`).
//! 2. **An account 100 blocks below the tip, over blocks DECLARING the tree
//!    sizes the honest roots imply.** The scanner accepts that (it seeds the
//!    first block's size from the block's own declaration when no prior block
//!    is stored), and the roots are accepted (`Ok([Served{1}, Served{1},
//!    Served{4}])`), but the store refuses the first scanned batch over an
//!    EMPTY-frontier birthday — the block's checkpoint position is derived
//!    from the declared size, which an empty tree cannot hold — and that
//!    fault surfaces as `Sync { EndpointUnreachable }` (measured: one range
//!    `(3475399, 3475498)` downloaded, then `Err`). Scanning BELOW the roots,
//!    the precedent's escape (`sync_bind_proof::scan_below_the_fixtures`), is
//!    unreachable here because of (1).
//!
//! So every wallet here imports account 0 at a birthday of **`TIP + 1`**:
//! upstream's own lower bound then yields an empty range, the pass ingests
//! the roots, records the tip, finds nothing to scan, and publishes its
//! terminal status — a wallet created at the tip of a chain that has not
//! advanced, which is an ordinary state and the one INC-020's first-launch
//! user is in. The scanned-tree-size and completing-hash arms of the bind
//! ABSTAIN by construction on such a wallet (nothing scanned), which is the
//! arm set the precedent's non-scanning rows already live under; the arms in
//! play are the gap floor, the recorded heights and the bundled frontiers,
//! and every refusal fixture below names which of those refuses it. Every
//! row that reaches a healthy status asserts through the double's receipt
//! that no block was ever requested, so a fixture drift that starts a scan
//! is a named failure rather than a silent swap of mechanisms.
//!
//! Also held constant: **one tip for every fixture** ([`TIP`]), so two
//! statuses that differ never differ in the tip; **`NoSubscriber` scoped
//! over every pass** (D3); and **a receipt** of every stream the double
//! opened, refused or served, so each negative case states that the pool it
//! is named after was actually asked for — "the pool was never fetched" is the
//! cheapest wrong reason for a degraded-pool row to pass.
//!
//! ## The numbers
//!
//! Mainnet, the measured values `sync_bind_proof.rs` already pins (its module
//! doc carries the provenance): Sapling's first completion at 558,822,
//! Orchard's at 1,707,429, the Ironwood sequence `{3451206, 3463000, 3467168,
//! 3475149}`, endpoint tip 3,475,499. Two refusal fixtures are reused with
//! their arm measured at HEAD this session: `[3_465_000]` at Ironwood index 0
//! is refused by `bundled_frontier` and by nothing else
//! (`a_bundled_frontier_violation_is_refused_by_the_bundled_arm`), and
//! Sapling `[419_200]` after an honest `[558_822]` is refused by
//! `recorded_height` (`the_same_compression_is_refused_for_sapling_and_orchard`
//! plus `root_bind::check_recorded_heights`, equality-when-recorded).
//!
//! ## T0-1c — the rows added (§4k, E1–E6 plus two plants)
//!
//! Contract: `docs/plan/production-readiness-phase-1.md` §4k, item T0-1c — an
//! endpoint behind the binary's own data reads as "Up to date". Written blind
//! under the same rules as the rows above; the implementation half was not
//! read. The T0-1b rows above are byte-identical; what changed in the harness
//! is mechanical and named in the commit.
//!
//! **One tip per fixture is no longer true.** The floor's geometry is a tip
//! BELOW the bundle's newest row (3,459,780 on mainnet, read from
//! `root_bind::bundled_counts` in every row that relies on it rather than
//! hard-coded), so these rows drive tips of 3,455,000 (E1/E6), 3,451,205
//! (INC-023's own, E2), the newest row and the row below it (the two plants),
//! and `u32::MAX + 1` (E5) beside the file-wide [`TIP`]. Each wallet is
//! anchored at ITS endpoint's tip + 1, for the reason the module doc gives
//! above — and for one more, measured before a row was written: upstream's
//! `update_chain_tip` builds `wallet_birthday..chain_end` and
//! `ScanRange::from_parts` asserts `end >= start`, so a wallet born at
//! `TIP + 1` driven at a lower tip panics inside upstream, which would be a
//! harness artifact, not the contract's mechanism.
//!
//! **E6 scans, deliberately.** Its second pass is the SAME wallet at [`TIP`],
//! 20,499 blocks above its birthday, so the pass ingests the roots, records the
//! tip and scans the gap over [`fixture_block`]s that declare zero commitments
//! — the honest shape of "a behind endpoint that catches up", and the one pass
//! in this file whose receipt shows block requests. Its bind runs BEFORE that
//! scan (roots first), so the scanned-tree-size arm still abstains where it
//! matters; the row asserts the ranges it scanned lie inside the gap.
//!
//! **The two plants pin the boundary the floor does not name.** An endpoint
//! exactly AT the newest row is not behind the binary's data (CONTROL, green
//! at the contract commit — it is the row that fails a `<=`); one block BELOW
//! it is (DEFECT — it is the row that fails a floor with any slack).
//!
//! ## T0-1d — the rows added (§4l part (c): F5, F6, two plants, the generator)
//!
//! Contract: `docs/plan/production-readiness-phase-1.md` §4l — a strict prefix
//! above a proven boundary reads `UpToDate`. Written blind under the same rules
//! as everything above; the implementation half was not read.
//!
//! **The honest fixture is no longer short.** Until this section every honest
//! serve in this file was ONE Sapling root and ONE Orchard root where the signed
//! bundle proves 1128 and 769 complete at [`TIP`] (§4i (i)) — the fixture-driven
//! floor T0-1b's zero-only rule was built on. Under (c) that serve is a strict
//! prefix and must not read healthy, so every row asserting a healthy terminal
//! over it would go red at the join for a reason it is not named for.
//! [`bind_consistent_sequence`] derives the FULL sequence per pool from
//! `root_bind::bundled_counts` alone: subtree `i` is placed inside
//! `(max{H : complete(H) ≤ i}, min{H : complete(H) > i}]`, at least
//! [`COMPLETION_GAP_FLOOR`] blocks from its neighbours, packed from the bottom
//! of each window; a window that cannot hold the completions the bundle pins
//! into it is REPORTED (§4l P8), never padded. [`ProofEndpoint::honest`] serves
//! it at [`TIP`]; [`ProofEndpoint::honest_at`] serves the tip-filtered prefix at
//! a lower tip, so each T0-1c row keeps exactly one thing wrong (the tip, or the
//! Ironwood zero) and its mutant still has one thing to break. Every regenerated
//! row is named in the report with its old → new served counts; no Ironwood
//! script changed.
//!
//! **The root-hash scheme is re-keyed to `(pool, index)`** — byte 0 the pool
//! tag, bytes 1–2 the index little-endian — because `tag_base + index` in one
//! byte overflows at index 240 and the honest Sapling serve has 1128 roots.
//! Index 0 of every pool is byte-identical to the old scheme (the only index any
//! refusal fixture in this file serves); no row asserts a hash byte.
//!
//! **Two rows read the store.** F6's cross-wire clause ("every root written and
//! read back per pool") and F5's IT-10 receipt ("the prefix was accepted and
//! WRITTEN, so a degraded status is about shortness and not a refusal") read
//! `{pool}_tree_shards` through a second keyed connection —
//! `sync_bind_proof.rs`'s observer pattern, copied rather than shared because the
//! two proof files are blind to each other's edits; the fold may unify them.
//!
//! ## T0-1c-R2 — the rows added (§4n: G1–G6 plus four plants)
//!
//! Contract: `docs/plan/production-readiness-phase-1.md` §4n — the floor reads
//! the wallet's own height, the stamp waits for a current tip, the birthday and
//! the signing anchor stop trusting an ungraded height. Written blind under the
//! same rules as everything above; the implementation half was not read. Every
//! row above is byte-identical; the harness gains what this section names and
//! nothing else.
//!
//! **Some wallets here SCAN, on purpose** (G1, G5, G4, the plants). M1's grade
//! reads the wallet's OWN scanned height, so the wallet must hold one: it is
//! anchored [`R2_SCAN_DEPTH`] blocks below [`TIP`] (an empty-frontier anchor,
//! E6's shape) and driven once by an honest endpoint at [`TIP`], which scans the
//! gap in `SYNC_BATCH_BLOCKS` batches over [`fixture_block`]s
//! ([`scanned_wallet_at_tip`] asserts the receipt: ≥ 1 range asked, every range
//! inside the gap, `UpToDate { TIP }`, `scanned_tip() == TIP`). G1's `k` is
//! [`R2_PIN_BELOW`]: above one rewind step AND above `REORG_MAX_BLOCKS`, so
//! whichever margin the implementer prices (§4n decision 1) the pinned tip sits
//! below it, and `TIP − k` stays at or above the newest bundled row so the T0-1c
//! floor alone cannot be what reds the row (both asserted in the row).
//!
//! **What a scanned wallet can be served afterwards — the harness's limit,
//! measured, not argued.** This file's chain declares ZERO commitments in every
//! block (the only shape an empty-frontier anchor can scan; module doc, shape
//! 2), and the bind's scanned-tree-size arm reads the NEWEST scanned block's
//! declared size (`root_bind::read_scanned_tree_size`) — so after the scan
//! every served Sapling and Orchard root is a completion the wallet's own
//! blocks say cannot exist, a required-pool refusal, and the pass stalls
//! `EndpointMisbehaving` (measured at the contract commit: the first build of
//! these rows served the honest sequence on the post-scan passes and every one
//! stalled there). A chain whose blocks declare the roots' sizes needs a
//! per-batch anchor frontier of that size whose ommers agree with the shard
//! roots — the non-empty-frontier fixture the T0-1 ruling (§5.1, OWED #3) and
//! the T0-1a ruling both record as owed, unbuildable here without
//! `incrementalmerkletree`, which `sync.rs` records this crate declined as a
//! dev-dependency (a supply-chain decision). So every post-scan pass serves
//! [`ProofEndpoint::withholding_at`] — NO roots, the one serve that arm admits
//! — which T0-1d reads as `Withheld { proven }` on every pool: DEGRADED, never
//! refused, so the pass COMPLETES, `record_synced` runs, and the terminal is
//! `UpToDateDegraded` at the tip (or `EndpointBehind` carrying the withheld
//! report). The rows therefore assert by NAME — `EndpointBehind` at the
//! endpoint's tip against [`is_at_or_above_terminal_at`] — never by
//! `says_degraded`, which a withheld report satisfies for the wrong reason;
//! the receipt shows each pool WAS asked. The cost, stated: G5's "→ `UpToDate`"
//! reads "→ the at-or-above terminal" here, and every post-scan record line
//! carries a withheld report that is the harness's, not the mechanism's.
//!
//! **Two endpoint heights the double now reports separately.** Until this
//! section `latest_block_height`, `tip_height` and the identity's `block_height`
//! were one number (P1). M4 is about the identity height ALONE moving a
//! persisted signing input, so [`ProofEndpoint::with_identity_height`] lets the
//! identity claim a height the tip ports do not, and
//! [`ProofEndpoint::omitting_branch_id`] is the old server §6.3's grace exists
//! for. Both identities are synthesised inside `testing` — the
//! `no_second_branch_comparison` policy row reads that module the way it reads
//! `sync::testing`.
//!
//! **The stamped, account-less wallet (G3).** M3's arm —
//! `min(tip − lag, estimate(created_at))` — is reached only by a wallet that
//! carries the creation stamp and has no account when a pass runs. Through
//! `Wallet::create_with_vault` that never happens today: a freshly-generated
//! create imports account 0 EAGERLY (FR-24) before the handle exists, and a
//! restore-shaped create floors to activation whatever the tip says (so on this
//! file's `unprovisioned_wallet` the birthday IS at activation by design — G3's
//! "never at activation" would be red there for the wrong reason, which the
//! report records). The shape that reaches the arm is the one
//! `resolve_birthday`'s own doc names — "a stamped account-less REOPEN": the
//! create's order is provision (stamp, marker) → open → `ensure_account`, so a
//! kill between the marker and the import, or a wallet created by a pre-FR-24
//! binary, leaves exactly it. [`stamped_account_less_remnant`] builds it through
//! the same store functions the create calls, with the import omitted; the drive
//! is still `controller_over`.
//!
//! **Distinguishability for the durable stamp** (G2, the relaunch plant). §4n
//! decision 2 leaves the shape to the implementer, and a row cannot name a field
//! that does not exist at the contract commit. So "the stamp carries a standing
//! the render can qualify" is asserted as a SHAPE: the relaunch snapshot renders
//! a field the contract commit's `WalletState` / `SyncStamp` do not have, or a
//! `sync` that is not `Idle` ([`relaunch_reads_qualified`]). The adjudicator
//! refines to the field's name.
//!
//! ## T0-1c-R3 — the repair round (§4n-R: R3-2 new, R3-3 re-aimed)
//!
//! Contract: `docs/plan/production-readiness-phase-1.md` §4n-R — a birthday
//! above the endpoint's tip never reaches upstream. Written blind, by the same
//! author as the T0-1c-R2 rows, against the T0-1c-R2 join and its ruling
//! (the T0-1c-R2 ruling, a private design record); the implementation half was
//! not read. Two edits and nothing else: a NEW row for INC-024's second
//! geometry (the ruling's probe P-1 — a configured birthday imported EAGERLY,
//! then a server far below it: upstream's `Historic` arm) beside the born-above
//! plant (the `ChainTip` arm, UNEDITED); and G3 RE-AIMED by its author — the
//! ruling found "≥ row − lag" unbuildable on either network (the anchor
//! ceiling), so the clamp arm now asks for the birthday a CONTROL remnant
//! resolves against a current server at the row, and asserts the pass
//! CONTINUES past the write. Every other row is byte-identical; the harness
//! gains one record helper ([`record_r3`]) and one cell reader ([`q_r5_cell`]).
//!
//! ## GRACE-1 — the rows added (§4p: G-1..G-8 plus two plants)
//!
//! Contract: `docs/plan/production-readiness-phase-1.md` §4p — the
//! unknown-branch grace expires on the device clock too, and the user sees the
//! grace, its expiry and the next step. Written blind under the same rules as
//! everything above; the implementation half was not read. Every row above is
//! byte-identical; the harness gains what this section names and nothing else.
//!
//! **The clock enters through the seam P-G2 NAMES, called blind:**
//! `Wallet::open_with_vault_and_seed_port(cfg, vault, None,
//! Some(Arc::new(sync::testing::ManualClock::at(secs))))`. That call does not
//! exist at the contract commit, so the rows that need it live in the
//! [`clock_seam`] module at the END of this file, committed separately and
//! non-compiling there; the rows outside it compile and were run red-first at
//! the contract commit. `ManualClock::at` is the ONE constructor the contract
//! names, so this file never advances or rewinds a clock in place: every clock
//! change is a `close()` and a re-open through the seam with a new reading —
//! which makes each clock row ALSO a relaunch, proving the carried capable time
//! (and the latch, if built as one) is read cold at every gate G-13 names.
//!
//! **Every row drives the shipped path.** The verdict through `controller_over`
//! (`pass_over`); the refusal through the real send gate — `propose_uri`, which
//! asks `signing_permit` → `granted` BEFORE it parses the URI, so the refusal
//! under test is the gate's and the permitted control is the parse stopping at
//! the funds; the parked-row flag through `list_parked_sends` over a row
//! `queue_send_uri` queued (composing and queueing are ungated by design); the
//! cold read through `consensus_status`. Never `permits_signing` itself — its
//! signature is the implementer's to change (G-13: "it takes `now`, or the
//! snapshot carries `capable_at`").
//!
//! **Distinguishability, never a name (the §4n precedent).** The grace refusal
//! is "a `WalletError` variant the contract commit does not have"
//! ([`WALLET_ERROR_VARIANTS_AT_THE_CONTRACT_COMMIT`]) and never
//! `NetworkUpgradeUnsupported` — the type the bridge renders as "the network
//! was upgraded, update the app". The grace SURFACE is "not the contract
//! commit's at-or-above shape" ([`shows_grace`]): on this file's scanned wallet
//! that shape is `UpToDateDegraded { tip, pools }` with a withheld report (the
//! harness's limit, above), not plain `UpToDate` — so a row that asserted "not
//! `UpToDate`" would be green at the contract commit for the withheld report's
//! reason, and none does. The parked row's blockage is read as the flag plus
//! every field the contract commit's `ParkedSend` lacks ([`parked_blockage`]),
//! and rows compare readings across states rather than naming one. The
//! adjudicator refines each to the minted name.
//!
//! **Geometry.** The anchor is TIP after [`scanned_wallet_anchored_at_tip`]
//! (the honest pass is judged before its scan and anchors at `TIP − 1,100`; the
//! G4 row's pass 1 is repeated to bring it to TIP). The claim moves through
//! `with_identity_height` — the number `blocks_since_last_current` is fed
//! (`wallet.rs:6461`) — with the tip ports held at TIP so nothing scans; a
//! frozen claim is `advance = 0`. Blocks: `500` inside (with the reorg margin a
//! capable pass can add, still inside) and `2,000` past (by more than that
//! margin). Clock: an arbitrary fixed epoch `G1_T0`, then `+12 h`, `+23 h`
//! (inside), `+34 h` (past a day, but only 11 h after the previous Unknown pass
//! — a capable time RENEWED by an Unknown pass would still read inside), `+48 h`
//! (the contract's number), `−1 h` (a capable time in the future), `±2 years`
//! (G-7). 24 h exactly and 1,152 × 75 s are both 86,400, so every step sits on
//! the same side of "a day" whichever item-5 constant is built.

use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use async_trait::async_trait;
use tracing::instrument::WithSubscriber;
use tracing::subscriber::NoSubscriber;
use zcash_client_backend::proto::compact_formats::{ChainMetadata, CompactBlock};
use zcash_client_backend::proto::service::{ShieldedProtocol, SubtreeRoot, TreeState};

use crate::config::{JitterPolicy, LightServerEndpoint, TorPolicy, WalletConfig};
use crate::constants::{NEW_WALLET_BIRTHDAY_LAG_BLOCKS, REORG_MAX_BLOCKS, REWIND_DISTANCE_BLOCKS};
use crate::enhance::{FetchedTransaction, TransactionFetcher};
use crate::error::WalletError;
use crate::keychain::testvault::TestVault;
use crate::keychain::{KeychainPort, VaultTier};
use crate::money::{BlockHeight, Network};
use crate::net::grpc::testing::scripted_stream;
use crate::net::grpc::{BlockStream, GrpcError, SubtreeRootStream};
use crate::provision::{ChainOracle, ServerIdentity};
use crate::seal::WalletDbKey;
use crate::seed::{SeedPersistence, SeedSource};
use crate::state::{PoolService, StallReason, SyncStatus, WalletState};
use crate::sync::testing::display_order_hex;
use crate::sync::{self, ScanClient, SubtreeRootSource, TransparentUtxoSource};
use crate::transparent::TransparentUtxoRecord;
use crate::wallet::{PassClient, Wallet};

// ── The fixtures ────────────────────────────────────────────────────────────

/// The endpoint's reported tip, the SAME for every fixture in this file.
const TIP: u64 = 3_475_499;
/// The empty-frontier anchor the imported account's birthday is derived from
/// (birthday = anchor + 1 = `TIP + 1`, so the scan queue is empty — module doc).
const BIRTHDAY_ANCHOR: u64 = TIP;
/// The measured first completions of Sapling and Orchard. **Since T0-1d these
/// are PREMISES, not the honest serve**: a one-root serve is a strict prefix of
/// what the bundle proves and reads degraded under §4l (c). The honest serve is
/// [`full_sequence`]; these constants anchor the T0-1c tip premises ("above
/// every pool's first completion") and nothing else.
const SAPLING_TRUTHFUL: &[u64] = &[558_822];
const ORCHARD_TRUTHFUL: &[u64] = &[1_707_429];
/// The honest Ironwood sequence.
const IRONWOOD_TRUTHFUL: &[u64] = &[3_451_206, 3_463_000, 3_467_168, 3_475_149];
/// One Ironwood root refused by the BUNDLED arm and by nothing else on a
/// wallet that has scanned nothing yet (`sync_bind_proof::BUNDLED_ONLY_VIOLATION`,
/// measured at HEAD).
const IRONWOOD_BUNDLED_VIOLATION: &[u64] = &[3_465_000];
/// Sapling "activation + 0" — the measured compression at k = 0. On a wallet
/// that recorded [`SAPLING_TRUTHFUL`] it is refused by `recorded_height`.
/// Every A11 clause passes it: one root (ordering is trivial), at the
/// activation (not below it — asserted from the compiled params in the row),
/// below [`TIP`]. So the refuser is `apply_height_bind`'s required-pool arm.
const SAPLING_COMPRESSED: &[u64] = &[419_200];
/// The PLANTED required-pool violation: a Sapling completion ABOVE the
/// endpoint's own tip. Refused by `validate_root_sequence`'s ceiling clause
/// BEFORE the bind is consulted — a different mechanism from D9's, and one
/// the contract's D9 text does not name.
const SAPLING_ABOVE_TIP: &[u64] = &[TIP + 1];

// ── T0-1c fixtures (§4k) ────────────────────────────────────────────────────

/// E1/E6's tip: below the bundle's newest mainnet row (3,459,780 — asserted
/// from `bundled_counts` in the row, never assumed) and above every pool's
/// first completion. The contract names it.
const BEHIND_TIP: u64 = 3_455_000;
/// INC-023's own tip: one below Ironwood's first completion (3,451,206) and
/// below the first bundled row that proves an Ironwood subtree complete
/// (3,452,280) — so at the contract commit `proven` reads 0 there.
const INC_023_TIP: u64 = 3_451_205;
/// The honest Ironwood sequence for ANY tip in `3_451_206..3_463_000`: one
/// root, the first completion. The bundle row `(3_452_280, 1)` agrees, and the
/// row that relies on that asserts it.
const IRONWOOD_HONEST_BELOW_BUNDLE: &[u64] = &[3_451_206];
/// A tip no consensus height can be (E5): one past `u32`.
const TIP_BEYOND_U32: u64 = u32::MAX as u64 + 1;

// ── T0-1c-R2 fixtures (§4n) ─────────────────────────────────────────────────

/// How far below [`TIP`] the scanning wallets of the R2 rows are anchored, so
/// the honest first pass scans `R2_SCAN_DEPTH` blocks (module doc: "some wallets
/// here SCAN") and the wallet holds a height of its own for M1's grade to read.
/// Above [`R2_PIN_BELOW`], so G1's pinned tip stays above the birthday and
/// upstream's range builder is never handed an inverted range (asserted).
const R2_SCAN_DEPTH: u64 = 1_200;
/// G1's `k`: how far below the wallet's own scanned height the pinned endpoint
/// reports. Above one rewind step (`REWIND_DISTANCE_BLOCKS`, 10) AND above
/// `REORG_MAX_BLOCKS` (100) — the two margins §4n decision 1 offers — by an
/// order of magnitude, so the margin the implementer prices cannot be what makes
/// the row pass; and small enough that `TIP − k` stays above the bundle's newest
/// mainnet row, so the T0-1c floor cannot be what makes it fail. Both asserted
/// in the row from the compiled constants and the bundle, never assumed.
const R2_PIN_BELOW: u64 = 1_000;
/// G4: how far below the wallet's scanned height the behind identity claims —
/// the contract's `T − 20,000` (§4n M4: "a height 22 k below the wallet's
/// scanned height"). Above `REORG_MAX_BLOCKS` by two orders of magnitude, so
/// the clamp (`min(claimed, scanned + REORG_MAX_BLOCKS)`) takes the CLAIM
/// and the anchor moves at the base.
const R2_IDENTITY_BEHIND: u64 = 20_000;
/// G4's control and G5's "above both references": the chain advances by this
/// much (one `SYNC_BATCH_BLOCKS` batch) — the contract's `T + 100`.
const R2_RAISE: u64 = 100;
/// The over-claim plant: an identity height this far ABOVE the scanned height,
/// which the clamp must still hold to `scanned + REORG_MAX_BLOCKS`.
const R2_IDENTITY_OVERCLAIM: u64 = 10_000;
/// G3's "far below": the lying provisioning tip sits at least this far below
/// the bundle's newest row (the contract's "~3 M-block scan" is the extreme; a
/// row derived from the treestate table half a million blocks down already
/// buys a scan the wallet cannot undo by switching servers).
const R2_FAR_BELOW: u64 = 500_000;

/// Per-pool tags: distinct across pools, for the two reasons
/// `sync_bind_proof::pool_script` gives (Orchard and Ironwood are the same Rust
/// type; same-hash roots make `put_shard_roots` fail — INC-021). The index is
/// carried in bytes 1–2 since T0-1d (module doc), so distinctness across
/// indices no longer depends on this byte.
const SAPLING_TAG: u8 = 0x10;
const ORCHARD_TAG: u8 = 0x30;
const IRONWOOD_TAG: u8 = 0x50;

/// Two endpoints for the D4 endpoint clause. Neither is ever dialed — the
/// client is injected — and that is part of what the D6 rows check: the host
/// string must not appear in the published status.
const HOST_A: &str = "https://a.example:443";
const HOST_B: &str = "https://b.example:443";

/// A root at `height` whose hash is keyed by `(pool tag, index)`: byte 0 the
/// tag, bytes 1–2 the index little-endian, the rest zero — a small integer, so
/// canonical for both `jubjub::Base` and `pallas::Base`. Index 0 is byte-identical
/// to the pre-T0-1d `tag + index` scheme (module doc).
fn root_at(height: u64, tag: u8, index: usize) -> SubtreeRoot {
    let mut root_hash = vec![0u8; 32];
    root_hash[0] = tag;
    let index = u16::try_from(index).expect("no pool has 65 536 subtrees below a bundled row");
    root_hash[1..3].copy_from_slice(&index.to_le_bytes());
    SubtreeRoot {
        root_hash,
        // Empty on purpose: the completing-hash arm ABSTAINS on anything that
        // is not exactly 32 bytes, and every served completion height sits
        // below the scan window anyway, so there is no scanned block for it to
        // compare against. An arm that cannot fire cannot be the wrong refuser.
        completing_block_hash: Vec::new(),
        completing_block_height: height,
    }
}

/// A whole pool's script: `heights[i]` served at subtree index `i`, hash keyed
/// by `(tag, i)` — so a re-serve of index `i` carries a byte-identical hash and
/// the shardtree's `Conflict` path can never be what refuses a height case.
fn pool_script(heights: &[u64], tag: u8) -> Vec<SubtreeRoot> {
    heights
        .iter()
        .enumerate()
        .map(|(i, h)| root_at(*h, tag, i))
        .collect()
}

// ── T0-1d (§4l): the bind-consistent sequence generator ─────────────────────

/// The block-size floor between consecutive completions the bind's
/// `completion_gap` arm enforces — `root_bind::MIN_COMPLETION_GAP_BLOCKS` itself,
/// read through the seam. Written blind as a COPY (`= 2`) because the constant
/// was private at the contract commit and a missing seam is reported, not
/// invented (report §5); the implementer's half made it `pub(crate)` for its own
/// generator in `sync::testing`, and the T0-1d adjudication unified the two here
/// (§4l decision 3 — a harness repair, charged in the RULING), so the honest
/// control is derived from the bind's own floor rather than from a number that
/// could drift from it. The generator spaces every consecutive pair by at least
/// this and asserts it.
const COMPLETION_GAP_FLOOR: u64 = crate::root_bind::MIN_COMPLETION_GAP_BLOCKS as u64;

/// One window the signed bundle pins a run of completions into: every subtree
/// in `first_index..first_index + count` completed strictly above `lo` and at
/// or below `hi` (§4l P8's `(max{H : complete(H) ≤ i}, min{H : complete(H) > i}]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Window {
    lo: u64,
    hi: u64,
    first_index: usize,
    count: usize,
}

/// A pool's bind-consistent synthetic completion sequence at one tip — the
/// generator's receipt (§4l F6).
struct Generated {
    pool: ShieldedProtocol,
    tip: u64,
    /// The largest `complete` among bundled rows at or below `tip` — what
    /// `proven_complete_at_or_below` reads, derived here independently.
    proven: usize,
    /// `heights[i]` is subtree `i`'s synthesised completing height. Shorter than
    /// `proven` iff `unfilled` is non-empty.
    heights: Vec<u64>,
    /// Every window the admitted rows pin, in index order.
    windows: Vec<Window>,
    /// P8: windows that cannot hold the completions the bundle pins into them
    /// at [`COMPLETION_GAP_FLOOR`] spacing. Reported, never padded.
    unfilled: Vec<Window>,
}

impl Generated {
    /// One line per pool for the red-first / post-fix artifact (`--nocapture`).
    fn receipt(&self) -> String {
        let widths: Vec<u64> = self.windows.iter().map(|w| w.hi - w.lo).collect();
        let busiest = self.windows.iter().map(|w| w.count).max().unwrap_or(0);
        let min_gap = self
            .heights
            .windows(2)
            .map(|p| p[1] - p[0])
            .min()
            .unwrap_or(0);
        format!(
            "{} at tip {}: proven {} — {} windows (width {}..={} blocks, busiest holds {} \
             completions), {} heights placed (min gap {}), first {:?}, last {:?}, unfilled {:?}",
            self.pool.as_str_name(),
            self.tip,
            self.proven,
            self.windows.len(),
            widths.iter().min().copied().unwrap_or(0),
            widths.iter().max().copied().unwrap_or(0),
            busiest,
            self.heights.len(),
            min_gap,
            self.heights.first(),
            self.heights.last(),
            self.unfilled
        )
    }

    /// The generator's own assertions (§4l F6): every window filled, every
    /// height inside its window, ≥ the gap floor from its neighbours, within
    /// `[activation, tip]`, strictly increasing.
    fn assert_bind_consistent(&self) {
        assert!(
            self.unfilled.is_empty(),
            "P8: the bundle pins more completions into a window than {} blocks apart can hold — \
             a finding about the bundle's spacing, not about the item: {:?}",
            COMPLETION_GAP_FLOOR,
            self.unfilled
        );
        assert_eq!(
            self.heights.len(),
            self.proven,
            "the full sequence has exactly `proven` roots ({})",
            self.receipt()
        );
        let activation = pool_activation(self.pool);
        for w in &self.windows {
            for i in w.first_index..w.first_index + w.count {
                let h = self.heights[i];
                assert!(
                    h > w.lo && h <= w.hi,
                    "{} index {i} at {h} is outside its window ({}, {}]",
                    self.pool.as_str_name(),
                    w.lo,
                    w.hi
                );
                assert!(
                    h >= activation && h <= self.tip,
                    "{h} outside [activation {activation}, tip {}]",
                    self.tip
                );
            }
        }
        for (i, pair) in self.heights.windows(2).enumerate() {
            assert!(
                pair[1] >= pair[0] + COMPLETION_GAP_FLOOR,
                "{} indices {i}/{}: {} then {} — closer than the gap floor {}",
                self.pool.as_str_name(),
                i + 1,
                pair[0],
                pair[1],
                COMPLETION_GAP_FLOOR
            );
        }
    }
}

/// The A11 activation floor per pool, from the compiled params.
fn pool_activation(pool: ShieldedProtocol) -> u64 {
    use zcash_protocol::consensus::{NetworkUpgrade, Parameters};
    let nu = match pool {
        ShieldedProtocol::Sapling => NetworkUpgrade::Sapling,
        ShieldedProtocol::Orchard => NetworkUpgrade::Nu5,
        ShieldedProtocol::Ironwood => NetworkUpgrade::Nu6_3,
    };
    u64::from(u32::from(
        Network::Main
            .consensus()
            .activation_height(nu)
            .expect("the pinned params know every pool's activation on mainnet"),
    ))
}

/// **The generator** (§4l (c), §4i (i)): a sequence honest by the signed
/// bundle's own arithmetic, derived from `root_bind::bundled_counts` and
/// nothing else. Rows above `tip` are ignored — the same tip-bounding
/// `proven_complete_at_or_below` applies — so the sequence at a lower tip is a
/// prefix of the sequence at a higher one (a later row can only raise `hi`
/// for indices it newly proves, never move a window an earlier row fixed).
///
/// Each window is packed from its bottom (`lo + 1`, then `+ gap` per index),
/// never touching `hi` unless full; a window that cannot hold its run is left
/// EMPTY and reported, so a caller that serves the result serves a sequence
/// with a hole and fails loudly rather than a padded one that lies.
fn bind_consistent_sequence(pool: ShieldedProtocol, tip: u64) -> Generated {
    let rows: Vec<(u64, usize)> = crate::root_bind::bundled_counts(Network::Main, pool)
        .iter()
        .map(|&(h, c)| {
            (
                u64::from(h),
                usize::try_from(c).expect("a subtree count fits usize"),
            )
        })
        .filter(|&(h, _)| h <= tip)
        .collect();
    for pair in rows.windows(2) {
        assert!(
            pair[0].0 < pair[1].0,
            "premise: bundled rows ascend by height ({:?} then {:?})",
            pair[0],
            pair[1]
        );
        assert!(
            pair[0].1 <= pair[1].1,
            "premise: a later row proves at least as many subtrees complete ({:?} then {:?})",
            pair[0],
            pair[1]
        );
    }
    let proven = rows.iter().map(|r| r.1).max().unwrap_or(0);
    let activation = pool_activation(pool);
    let window_of = |i: usize| -> (u64, u64) {
        let lo = rows
            .iter()
            .filter(|r| r.1 <= i)
            .map(|r| r.0)
            .max()
            .unwrap_or(activation.saturating_sub(1));
        let hi = rows
            .iter()
            .filter(|r| r.1 > i)
            .map(|r| r.0)
            .min()
            .expect("i < proven, so some admitted row proves subtree i complete");
        (lo, hi)
    };
    let mut windows: Vec<Window> = Vec::new();
    for i in 0..proven {
        let (lo, hi) = window_of(i);
        match windows.last_mut() {
            Some(w) if w.lo == lo && w.hi == hi => w.count += 1,
            _ => windows.push(Window {
                lo,
                hi,
                first_index: i,
                count: 1,
            }),
        }
    }
    let mut heights: Vec<u64> = Vec::with_capacity(proven);
    let mut unfilled = Vec::new();
    for w in &windows {
        let start = heights.last().map_or(w.lo + 1, |&prev| {
            (w.lo + 1).max(prev + COMPLETION_GAP_FLOOR)
        });
        let last = start + (w.count as u64 - 1) * COMPLETION_GAP_FLOOR;
        if last > w.hi {
            unfilled.push(*w);
            continue;
        }
        heights.extend((0..w.count as u64).map(|t| start + t * COMPLETION_GAP_FLOOR));
    }
    Generated {
        pool,
        tip,
        proven,
        heights,
        windows,
        unfilled,
    }
}

/// The honest sequence for `pool` at `tip`, or a loud P8 failure — never a
/// sequence with a hole served as if it were whole.
fn honest_sequence_at(pool: ShieldedProtocol, tip: u64) -> Vec<u64> {
    let g = bind_consistent_sequence(pool, tip);
    assert!(
        g.unfilled.is_empty(),
        "P8: the honest control is unbuildable for {} at tip {tip} — {}",
        pool.as_str_name(),
        g.receipt()
    );
    g.heights
}

/// The honest sequence at the file-wide [`TIP`], generated once per pool.
fn full_sequence(pool: ShieldedProtocol) -> &'static [u64] {
    static SAPLING: OnceLock<Vec<u64>> = OnceLock::new();
    static ORCHARD: OnceLock<Vec<u64>> = OnceLock::new();
    static IRONWOOD: OnceLock<Vec<u64>> = OnceLock::new();
    let cell = match pool {
        ShieldedProtocol::Sapling => &SAPLING,
        ShieldedProtocol::Orchard => &ORCHARD,
        ShieldedProtocol::Ironwood => &IRONWOOD,
    };
    cell.get_or_init(|| honest_sequence_at(pool, TIP))
}

/// A deterministic 32-byte block id for `height`, height-encoded so blocks
/// chain (`prev_hash` of `h` = id of `h-1`) and so the anchor tree state's
/// hash is the block the first scanned block links to.
fn block_id(height: u64) -> Vec<u8> {
    let mut v = vec![0u8; 32];
    v[..8].copy_from_slice(&height.to_le_bytes());
    v
}

/// A well-formed, empty, chained block (the precedent's `compact_block_on`
/// shape). Never requested by a row in this file — see the module doc and
/// [`assert_never_scanned`] — but served honestly if one ever is, so a
/// fixture drift fails on the receipt and not on a decode.
fn fixture_block(height: u64) -> CompactBlock {
    CompactBlock {
        height,
        hash: block_id(height),
        prev_hash: block_id(height.wrapping_sub(1)),
        chain_metadata: Some(ChainMetadata {
            sapling_commitment_tree_size: 0,
            orchard_commitment_tree_size: 0,
            ironwood_commitment_tree_size: 0,
        }),
        ..Default::default()
    }
}

/// An EMPTY-frontier tree state at `height` whose hash is [`block_id`]`(height)`
/// — the birthday anchor (and the per-batch scan anchor, if a scan were ever
/// requested), the precedent's `tree_state_on` shape.
fn anchor_tree_state(height: u64) -> TreeState {
    TreeState {
        network: "main".to_owned(),
        height,
        // DISPLAY order, which is the reverse of the internal order a
        // `CompactBlock.hash` carries: `TreeState::to_chain_state` reverses the hex
        // it decodes (`zcash_client_backend-0.24.0/src/proto.rs:456-457`), so
        // hex-encoding the raw id served a hash that was the byte-reverse of the
        // same block's own. Nothing read it until SCAN-2 — the anchor reconcile
        // compares the whole `ChainState`, and a derived anchor carries the block's
        // own hash — and then it red two rows of THIS file that are in no diff
        // (`a_behind_endpoint_that_catches_up_clears_the_indicator`,
        // `a_lying_low_provisioning_tip_cannot_floor_the_birthday_below_the_bundle_row`).
        // The same defect and the same line as `sync::testing::chain_tree_state`,
        // in a second fixture nobody ran: measured green at `a2feaf5e` and red at
        // `b7548601` (the SCAN-2 fold).
        hash: display_order_hex(block_id(height)),
        time: 0,
        sapling_tree: String::new(),
        orchard_tree: String::new(),
        ironwood_tree: String::new(),
    }
}

// ── The double, and its receipt ─────────────────────────────────────────────

/// What one root call to the double did, in the order the pass made them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Call {
    protocol: i32,
    /// `Some(n)`: streamed `n` roots. `None`: refused the protocol at the open
    /// door.
    served: Option<usize>,
}

/// A handle onto the double's log that survives the double being moved into
/// the seam.
#[derive(Clone, Default)]
struct Receipt {
    calls: Arc<Mutex<Vec<Call>>>,
    /// Every `block_range(start, end_inclusive)` the pass requested.
    ranges: Arc<Mutex<Vec<(u64, u64)>>>,
}

impl Receipt {
    fn calls(&self, protocol: ShieldedProtocol) -> Vec<Call> {
        self.calls
            .lock()
            .expect("receipt")
            .iter()
            .copied()
            .filter(|c| c.protocol == protocol as i32)
            .collect()
    }
    /// How many times the pass asked for this pool's roots.
    fn opened(&self, protocol: ShieldedProtocol) -> usize {
        self.calls(protocol).len()
    }
    /// How many roots this double actually streamed for the pool, summed.
    fn streamed(&self, protocol: ShieldedProtocol) -> usize {
        self.calls(protocol).iter().filter_map(|c| c.served).sum()
    }
    /// How many of those asks were refused at the open door.
    fn refused(&self, protocol: ShieldedProtocol) -> usize {
        self.calls(protocol)
            .iter()
            .filter(|c| c.served.is_none())
            .count()
    }
    fn scanned_ranges(&self) -> Vec<(u64, u64)> {
        self.ranges.lock().expect("receipt").clone()
    }
}

/// This file's own `ScanClient + SubtreeRootSource`. It clones its root script
/// on every call (a pool can be served on any number of passes), serves the
/// fixture chain for any range, and records what it did.
///
/// **Adjudication repair (charged to the contract's seam S2).** As
/// written, this double was NOT a `ChainOracle`, because S2's bound named only
/// the scan and root ports. The production pass body drives five ports on one
/// client (`wallet::PassClient`), and `run_engine_pass` calls
/// `ChainOracle::server_identity` BEFORE `sync_once`'s tip fetch on every
/// pass. The three impls below are the adjudicator's, not the test author's:
/// the oracle answers through this double's own `latest_block_height` (so the
/// `dead()` control still fails at the pass's first RPC, which on the shipped
/// path is the identity fetch) with a mainnet identity whose branch id the
/// compiled params compute at the tip — the honest answer, and the one that
/// keeps the consensus verdict at `Behind` rather than `Unsupported`, so no
/// row here can be masked behind `UpToDateLimited`. The transparent poll and
/// the enhancement fetch ABSTAIN (`Ok(empty)` / `Ok(None)`); neither is
/// reached on a pass that scanned nothing, and each is the truthful answer of
/// an endpoint holding no such data if it ever is.
struct ProofEndpoint {
    tip: u64,
    sapling: Vec<SubtreeRoot>,
    orchard: Vec<SubtreeRoot>,
    ironwood: Vec<SubtreeRoot>,
    /// A protocol refusal at the Ironwood open door (v0.5.4's `InvalidArgument`
    /// after classification — the value `an_ironwood_refusal_leaves_the_other_two_pools_working`
    /// injects). Consumed on use.
    ironwood_open_err: Option<GrpcError>,
    /// A transport fault on the tip fetch — the dead-link control. Consumed.
    tip_err: Option<GrpcError>,
    /// Which chain this endpoint's `server_identity` claims to serve (T0-1c
    /// E4). Every fixture is a mainnet wallet; `Network::Test` here is the
    /// wrong-chain server, honest about being one.
    identity_network: Network,
    /// T0-1c-R2 (M4): the height the identity claims when it is NOT the tip
    /// ports' number. `None` = one number per fixture, as P1 has it — every
    /// row above this section. `Some` only where a row is about the identity
    /// height ALONE moving a persisted signing input.
    identity_height: Option<u64>,
    /// T0-1c-R2 (G4's reading of the §6.3 grace): the identity carries NO
    /// branch id — the old or non-conforming server the grace exists for.
    identity_branch_omitted: bool,
    /// GRACE-1 (G-6's control): the identity claims a branch id our params do
    /// NOT compute for its height — the stale-build (or wrong-chain) server,
    /// the one refusal "update the app" is TRUE for.
    identity_branch_bogus: bool,
    receipt: Receipt,
}

impl ProofEndpoint {
    fn serving(sapling: &[u64], orchard: &[u64], ironwood: &[u64]) -> Self {
        Self {
            tip: TIP,
            sapling: pool_script(sapling, SAPLING_TAG),
            orchard: pool_script(orchard, ORCHARD_TAG),
            ironwood: pool_script(ironwood, IRONWOOD_TAG),
            ironwood_open_err: None,
            tip_err: None,
            identity_network: Network::Main,
            identity_height: None,
            identity_branch_omitted: false,
            identity_branch_bogus: false,
            receipt: Receipt::default(),
        }
    }
    /// GRACE-1 (G-6's control): the identity reports a branch id that is not
    /// the one our params compute at its height, so the verdict is
    /// `Unsupported` — the state `NetworkUpgradeUnsupported` and its "update
    /// the app" copy are honest for. Everything else about it is honest.
    fn claiming_a_bogus_branch(mut self) -> Self {
        self.identity_branch_bogus = true;
        self
    }
    /// T0-1c: the same endpoint reporting a different tip. Both ports that
    /// read the tip (`latest_block_height`, `tip_height`) and the identity's
    /// `block_height` follow it — one number per fixture, as P1 has it.
    fn at_tip(mut self, tip: u64) -> Self {
        self.tip = tip;
        self
    }
    /// T0-1c-R2 (M4): the identity claims `height` while both tip ports keep
    /// reporting `tip` — the one endpoint number the grade never sees, moved
    /// on its own. The branch it reports is the one the compiled params compute
    /// AT `height` (honest for the claim), so a verdict here is about the
    /// height, never about a branch mismatch.
    fn with_identity_height(mut self, height: u64) -> Self {
        self.identity_height = Some(height);
        self
    }
    /// T0-1c-R2 (G4): the identity omits its branch id — the server §6.3's
    /// grace was written for. Everything else about it is honest.
    fn omitting_branch_id(mut self) -> Self {
        self.identity_branch_omitted = true;
        self
    }
    /// T0-1c E4: a server on the OTHER chain — its identity is testnet's,
    /// honestly computed for its tip; the wallet is mainnet's.
    fn on_the_wrong_chain(mut self) -> Self {
        self.identity_network = Network::Test;
        self
    }
    /// An honest endpoint: every pool served truthfully. Since T0-1d, Sapling
    /// and Orchard serve the generator's FULL sequence at [`TIP`] (1128 / 769
    /// roots — the counts the bundle proves there, asserted in F6), Ironwood
    /// the measured four.
    fn honest() -> Self {
        Self::serving(
            full_sequence(ShieldedProtocol::Sapling),
            full_sequence(ShieldedProtocol::Orchard),
            IRONWOOD_TRUTHFUL,
        )
    }
    /// T0-1d: an endpoint honest FOR `tip` — Sapling and Orchard served the
    /// generator's tip-filtered sequence, Ironwood the given script — reporting
    /// `tip`. The T0-1c rows use it so that the only thing wrong on a behind
    /// pass is the tip (or the Ironwood zero), not also a short serve.
    fn honest_at(tip: u64, ironwood: &[u64]) -> Self {
        Self::serving(
            &honest_sequence_at(ShieldedProtocol::Sapling, tip),
            &honest_sequence_at(ShieldedProtocol::Orchard, tip),
            ironwood,
        )
        .at_tip(tip)
    }
    /// T0-1c-R2: an endpoint reporting `tip` that answers every pool with ZERO
    /// roots — the one serve the scanned-tree-size arm admits over this file's
    /// chain once the wallet has scanned (module doc, "the harness's limit").
    /// T0-1d reads it `Withheld { proven }` on every pool: degraded, never
    /// refused, so the pass completes and `record_synced` runs. Every row that
    /// uses it says so and asserts by name, never by `says_degraded`.
    fn withholding_at(tip: u64) -> Self {
        Self::serving(&[], &[], &[]).at_tip(tip)
    }
    /// Every completing height this endpoint would serve, for the D6/D11
    /// value check (`assert_carries_no_fixture_datum`).
    fn served_heights(&self) -> Vec<u64> {
        self.sapling
            .iter()
            .chain(&self.orchard)
            .chain(&self.ironwood)
            .map(|r| r.completing_block_height)
            .collect()
    }
    /// The A9 refusal: the server does not know the Ironwood protocol.
    fn refusing_ironwood(mut self) -> Self {
        self.ironwood_open_err = Some(GrpcError::ShieldedProtocolUnknown { code: 3 });
        self
    }
    /// The A10 shape: Ironwood answered, with zero roots; the other two pools
    /// full (T0-1d), so the zero is the only thing wrong.
    fn serving_no_ironwood_roots() -> Self {
        Self::serving(
            full_sequence(ShieldedProtocol::Sapling),
            full_sequence(ShieldedProtocol::Orchard),
            &[],
        )
    }
    /// A link that is genuinely down: the very first RPC of the pass fails.
    fn dead() -> Self {
        let mut ep = Self::honest();
        ep.tip_err = Some(GrpcError::Transport {
            stall: StallReason::EndpointUnreachable,
        });
        ep
    }
    fn receipt(&self) -> Receipt {
        self.receipt.clone()
    }
}

#[async_trait]
impl SubtreeRootSource for ProofEndpoint {
    async fn subtree_roots(
        &mut self,
        protocol: ShieldedProtocol,
        start_index: u32,
    ) -> Result<SubtreeRootStream, GrpcError> {
        // Exhaustive, no wildcard: a fourth upstream pool breaks this double's
        // build instead of letting it answer for a pool it has no script for.
        let (script, open_err) = match protocol {
            ShieldedProtocol::Sapling => (&self.sapling, None),
            ShieldedProtocol::Orchard => (&self.orchard, None),
            ShieldedProtocol::Ironwood => (&self.ironwood, self.ironwood_open_err.take()),
        };
        if let Some(e) = open_err {
            self.receipt.calls.lock().expect("receipt").push(Call {
                protocol: protocol as i32,
                served: None,
            });
            return Err(e);
        }
        // The script is the pool from index 0; a stream opened at `start_index`
        // serves from that index (S15-F1), and the receipt counts what was streamed.
        let items: Vec<Result<Option<SubtreeRoot>, tonic::Status>> = script
            .iter()
            .skip(start_index as usize)
            .cloned()
            .map(|r| Ok(Some(r)))
            .collect();
        self.receipt.calls.lock().expect("receipt").push(Call {
            protocol: protocol as i32,
            served: Some(items.len()),
        });
        Ok(scripted_stream(items))
    }
}

#[async_trait]
impl ScanClient for ProofEndpoint {
    async fn block_range(
        &mut self,
        start: u64,
        end_inclusive: u64,
    ) -> Result<BlockStream, GrpcError> {
        self.receipt
            .ranges
            .lock()
            .expect("receipt")
            .push((start, end_inclusive));
        let blocks: Vec<_> = (start..=end_inclusive)
            .map(|h| Ok(Some(fixture_block(h))))
            .collect();
        Ok(scripted_stream(blocks))
    }
    async fn tree_state(&mut self, height: u64) -> Result<TreeState, GrpcError> {
        Ok(anchor_tree_state(height))
    }
    async fn latest_block_height(&mut self) -> Result<u64, GrpcError> {
        match self.tip_err.take() {
            Some(e) => Err(e),
            None => Ok(self.tip),
        }
    }
}

// ── Adjudication repair: the three ports S2's bound omitted (see the struct doc)

#[async_trait]
impl ChainOracle for ProofEndpoint {
    async fn tip_height(&mut self) -> Result<u64, GrpcError> {
        // The same tip, and the same injected fault, as the scan port's read.
        self.latest_block_height().await
    }
    async fn server_identity(&mut self) -> Result<ServerIdentity, GrpcError> {
        // A dead link is dead for its first RPC, whichever port makes it.
        let tip = self.latest_block_height().await?;
        // T0-1c-R2: the identity's height is the tip unless a row moved it on
        // its own (`with_identity_height`); the branch is computed for the
        // height the identity CLAIMS, so it is honest for that claim.
        let claimed = self.identity_height.unwrap_or(tip);
        // GRACE-1 (G-6's control): the branch our params do not compute for
        // the claimed height — `Unsupported`, the one state the stale-build
        // copy is true for. Synthesised in `testing`, like the other three.
        if self.identity_branch_bogus {
            return Ok(testing::mainnet_identity_with_a_bogus_branch(claimed));
        }
        Ok(
            match (self.identity_network, self.identity_branch_omitted) {
                (Network::Main, false) => testing::mainnet_identity(claimed),
                (Network::Main, true) => testing::mainnet_identity_without_branch(claimed),
                (Network::Test, _) => testing::testnet_identity(claimed),
            },
        )
    }
}

#[async_trait]
impl TransparentUtxoSource for ProofEndpoint {
    async fn address_utxos(
        &mut self,
        _addresses: Vec<String>,
        _start_height: u64,
    ) -> Result<Vec<TransparentUtxoRecord>, GrpcError> {
        // Abstains: this endpoint knows no unspent transparent output. Not
        // reached on a pass with zero batches (`should_refresh_transparent`).
        Ok(Vec::new())
    }
}

#[async_trait]
impl TransactionFetcher for ProofEndpoint {
    async fn fetch_transaction(
        &mut self,
        _txid: zcash_protocol::TxId,
    ) -> Result<Option<FetchedTransaction>, GrpcError> {
        // Abstains: the endpoint has no such transaction. Not reached on a
        // wallet that has scanned nothing (an empty enhancement backlog makes
        // no RPC).
        Ok(None)
    }
}

// ── Wallet harness ──────────────────────────────────────────────────────────

fn test_vault() -> Arc<dyn KeychainPort> {
    Arc::new(TestVault::new(VaultTier::Tee))
}

fn cfg(db_dir: &Path, endpoint: &str) -> WalletConfig {
    WalletConfig {
        db_dir: db_dir.to_path_buf(),
        network: Network::Main,
        endpoint: LightServerEndpoint::new(endpoint).expect("endpoint"),
        endpoint_auth: None,
        tor: TorPolicy::Off,
        seed_persistence: SeedPersistence::SealedKeychain,
        birthday: None,
        broadcast_jitter: JitterPolicy::None,
        machine_memo_prefixes: Vec::new(),
        sync_servers: Vec::new(),
    }
}

fn raw_seed() -> SeedSource {
    SeedSource::raw_bytes(vec![0x2b; 32]).expect("valid seed")
}

/// A new wallet with account 0 imported at birthday `TIP + 1` (module doc):
/// nothing to scan, nothing served yet.
async fn anchored_wallet(dir: &Path, vault: &Arc<dyn KeychainPort>, endpoint: &str) -> Wallet {
    wallet_anchored_at(dir, vault, endpoint, BIRTHDAY_ANCHOR).await
}

/// T0-1c: the same wallet shape anchored at an arbitrary height — account 0
/// imported at birthday `anchor + 1`, so a pass at tip `anchor` scans nothing
/// (module doc, "One tip per fixture is no longer true").
async fn wallet_anchored_at(
    dir: &Path,
    vault: &Arc<dyn KeychainPort>,
    endpoint: &str,
    anchor: u64,
) -> Wallet {
    let w = unprovisioned_wallet(dir, vault, endpoint).await;
    let birthday = crate::account::birthday_from_treestate(anchor_tree_state(anchor))
        .expect("the empty-frontier anchor decodes");
    w.import_account(birthday).await.expect("import account");
    w
}

/// T0-1c E5: a wallet with NO account. `SeedSource::raw_bytes` is
/// restore-shaped (`is_freshly_generated` is false), and with no configured
/// birthday `resolve_offline_birthday` stays lazy, so `create_with_vault`
/// imports nothing and the first pass through `run_engine_pass` takes the
/// provisioning branch. The row asserts that precondition rather than
/// assuming it.
async fn unprovisioned_wallet(dir: &Path, vault: &Arc<dyn KeychainPort>, endpoint: &str) -> Wallet {
    Wallet::create_with_vault(cfg(dir, endpoint), raw_seed(), Arc::clone(vault))
        .await
        .expect("create")
}

// ── T0-1d: the observer connection (sync_bind_proof.rs's pattern, copied) ───

/// [`wallet_anchored_at`] that also captures the wallet's SQLCipher key, so a
/// row can read `{pool}_tree_shards` through a second keyed connection while
/// the handle stays open. The key is reachable only through `store::open`,
/// which needs the single-writer lock the live handle holds — hence the
/// close/capture/re-open before the account is imported; nothing here can be
/// mistaken for the behaviour under test.
async fn anchored_wallet_and_key(
    dir: &Path,
    vault: &Arc<dyn KeychainPort>,
    endpoint: &str,
    anchor: u64,
) -> (Wallet, WalletDbKey) {
    let w = unprovisioned_wallet(dir, vault, endpoint).await;
    w.close().await.expect("close");
    let key = {
        let lock = crate::lifecycle::WalletLock::acquire(dir).expect("acquire the wallet lock");
        let opened = crate::store::open(&lock, &**vault, Network::Main).expect("store open");
        let crate::store::OpenWallet {
            db_key, db, aux_db, ..
        } = opened;
        drop(db);
        drop(aux_db);
        db_key
    };
    let w = Wallet::open_with_vault(cfg(dir, endpoint), Arc::clone(vault))
        .await
        .expect("re-open after capturing the db key");
    let birthday = crate::account::birthday_from_treestate(anchor_tree_state(anchor))
        .expect("the empty-frontier anchor decodes");
    w.import_account(birthday).await.expect("import account");
    (w, key)
}

/// A second keyed connection onto the same wallet file.
fn observe(dir: &Path, key: &WalletDbKey) -> rusqlite::Connection {
    crate::db::open_existing_keyed_connection(&dir.join("wallet.db"), key)
        .expect("a second keyed connection onto the wallet db")
}

/// Every recorded `subtree_end_height` for a pool, in index order — upstream's
/// own table, the column only `put_shard_roots` fills.
fn recorded_heights(conn: &rusqlite::Connection, pool: ShieldedProtocol) -> Vec<u64> {
    let table = match pool {
        ShieldedProtocol::Sapling => "sapling_tree_shards",
        ShieldedProtocol::Orchard => "orchard_tree_shards",
        ShieldedProtocol::Ironwood => "ironwood_tree_shards",
    };
    let mut stmt = conn
        .prepare(&format!(
            "SELECT subtree_end_height FROM {table} ORDER BY shard_index"
        ))
        .expect("the shard table exists");
    stmt.query_map([], |r| r.get::<_, Option<i64>>(0))
        .expect("query shard rows")
        .map(|h| {
            u64::try_from(
                h.expect("shard rows decode")
                    .expect("a served root records a height"),
            )
            .expect("a height is non-negative")
        })
        .collect()
}

/// What one pass over the seam produced: the published status (the surface)
/// and the pass result (for the message only — never asserted as the surface).
struct Passed {
    status: SyncStatus,
    result: Result<sync::SyncPass, WalletError>,
}

fn describe(p: &Passed) -> String {
    let pass = match &p.result {
        Ok(pass) => format!("Ok({pass:?})"),
        Err(e) => format!("Err({e:?} / code {})", e.code()),
    };
    format!("published {:?}; pass {pass}", p.status)
}

/// ONE pass through the seam, under the no-op dispatcher (D3), reading the
/// status the controller published for it.
async fn pass_over<C>(w: &Wallet, client: C) -> Passed
where
    // Adjudication repair: S2 said `ScanClient + SubtreeRootSource + Send`; the
    // seam as built is bounded by every port the production pass drives.
    C: PassClient + 'static,
{
    let ctl = w.controller_over(client);
    let rx = ctl.subscribe();
    let result = ctl.once().with_subscriber(NoSubscriber::default()).await;
    let status = rx.borrow().clone();
    drop(ctl);
    Passed { status, result }
}

/// The harness precondition every row that reaches a healthy status states
/// (the sibling of `sync_bind_proof::scan_below_the_fixtures`' assertion, with
/// the opposite geometry): the pass requested NO block, so the scanned-tree-size
/// and completing-hash arms abstained by construction and the arms in play
/// are the gap floor, the recorded heights and the bundled frontiers — the
/// ones every refusal fixture in this file is attributed to. A fixture drift
/// that starts a scan would silently swap one mechanism for another (module
/// doc, shape 2); this makes it a named failure instead.
fn assert_never_scanned(receipt: &Receipt, row: &str) {
    let ranges = receipt.scanned_ranges();
    assert!(
        ranges.is_empty(),
        "{row} — precondition: the pass must scan nothing (birthday TIP + 1), or \
         the scanned-tree-size arm is armed against a chain that declares zero \
         commitments and refuses every honest re-serve for a reason no row here \
         is named after; got ranges {ranges:?}"
    );
}

// ── The predicates (S3/S4: distinguishability, never a name) ────────────────

fn is_terminal_healthy(s: &SyncStatus) -> bool {
    matches!(
        s,
        SyncStatus::UpToDate { .. } | SyncStatus::UpToDateLimited { .. }
    )
}

fn is_transient(s: &SyncStatus) -> bool {
    matches!(
        s,
        SyncStatus::Idle
            | SyncStatus::Connecting { .. }
            | SyncStatus::Scanning { .. }
            | SyncStatus::Offline { .. }
    )
}

/// A stall for one of the five reasons that exist today — each of which, over
/// a pass whose link worked and whose store is fine, is a specific lie.
fn is_known_stall(s: &SyncStatus) -> bool {
    matches!(
        s,
        SyncStatus::Stalled {
            reason: StallReason::EndpointUnreachable
                | StallReason::TorUnavailable
                | StallReason::StorageFull
                | StallReason::ChainReorg
                | StallReason::Internal
        }
    )
}

/// The status says something a healthy, transient, or already-known-stalled
/// wallet cannot say. The adjudicator refines this to the datum's shape.
fn says_degraded(s: &SyncStatus) -> bool {
    !is_terminal_healthy(s) && !is_transient(s) && !is_known_stall(s)
}

/// A stall whose reason is none of the five that exist today.
fn is_new_stall_reason(s: &SyncStatus) -> bool {
    matches!(s, SyncStatus::Stalled { .. }) && !is_known_stall(s)
}

fn stall_reason(s: &SyncStatus) -> Option<StallReason> {
    match s {
        SyncStatus::Stalled { reason } => Some(*reason),
        _ => None,
    }
}

/// `UpToDate` at exactly the fixture tip — the one healthy sentence.
fn is_healthy_at_tip(s: &SyncStatus) -> bool {
    matches!(s, SyncStatus::UpToDate { tip } if u64::from(tip.value()) == TIP)
}

/// T0-1c: the same sentence at a tip that is not [`TIP`].
fn is_up_to_date_at(s: &SyncStatus, at: u64) -> bool {
    matches!(s, SyncStatus::UpToDate { tip } if u64::from(tip.value()) == at)
}

/// T0-1c-R2: the "at or above both references" terminal at `at` — plain
/// `UpToDate`, or `UpToDateDegraded` carrying a pools report (the terminal a
/// [`ProofEndpoint::withholding_at`] serve reaches on a scanned wallet; module
/// doc). Named, not distinguished: both variants exist at the contract commit.
/// What it excludes is the point: `EndpointBehind`, every stall, every
/// transient, `UpToDateLimited`.
fn is_at_or_above_terminal_at(s: &SyncStatus, at: u64) -> bool {
    matches!(
        s,
        SyncStatus::UpToDate { tip } | SyncStatus::UpToDateDegraded { tip, .. }
            if u64::from(tip.value()) == at
    )
}

/// T0-1c: the newest bundled row for a mainnet pool, `(height, complete)`,
/// read from the signed bundle the binary ships with — the floor's reference
/// height per §4k's one-sided fact, never a literal in a row.
fn newest_bundled_row(pool: ShieldedProtocol) -> (u32, u64) {
    *crate::root_bind::bundled_counts(Network::Main, pool)
        .last()
        .expect("the mainnet bundle carries rows for every pool")
}

/// T0-1c: what the receipt says about WHERE the pass stopped — §4k E1/E2
/// require the row to state which it observed, because the disposition
/// (refuse before the roots, or report after them) is the implementer's.
fn where_it_stopped(receipt: &Receipt) -> String {
    let asked: usize = [
        ShieldedProtocol::Sapling,
        ShieldedProtocol::Orchard,
        ShieldedProtocol::Ironwood,
    ]
    .into_iter()
    .map(|p| receipt.opened(p))
    .sum();
    if asked == 0 {
        "the pass stopped before any pool was asked".to_owned()
    } else {
        format!(
            "the pools were asked (sapling {} / orchard {} / ironwood {} streams opened)",
            receipt.opened(ShieldedProtocol::Sapling),
            receipt.opened(ShieldedProtocol::Orchard),
            receipt.opened(ShieldedProtocol::Ironwood)
        )
    }
}

/// T0-1c: the red-first / post-fix record line for a row (IT-3 +A) — what the
/// row OBSERVED, printed whether or not it then passes, so a green row's
/// status is in the artifact too (`--nocapture`).
fn record(row: &str, p: &Passed, receipt: &Receipt) {
    println!(
        "[T0-1c {row}] {} — {}",
        describe(p),
        where_it_stopped(receipt)
    );
}

/// D6/D11 on the VALUE: nothing the fixture served, and no endpoint identity,
/// appears in the published status. The tip is allowed — `UpToDate { tip }`
/// already carries it and it is the endpoint's public claim, not this
/// wallet's. Served completion heights are not: a completion height narrows
/// a wallet's scan range (`PoolFetch::HeightViolation`'s own doc).
fn assert_carries_no_fixture_datum(status: &SyncStatus, served: &[u64], host: &str, row: &str) {
    let rendered = format!("{status:?}");
    let host_only = host
        .trim_start_matches("https://")
        .split(':')
        .next()
        .expect("host");
    assert!(
        !rendered.contains(host_only) && !rendered.contains("http"),
        "{row} — D6: the published status names the endpoint ({host_only}): {rendered}"
    );
    for h in served {
        assert!(
            !rendered.contains(&h.to_string()),
            "{row} — D6: the published status carries a served completion height {h}: {rendered}"
        );
    }
}

fn sapling_activation_from_params() -> u64 {
    use zcash_protocol::consensus::{NetworkUpgrade, Parameters};
    u64::from(u32::from(
        Network::Main
            .consensus()
            .activation_height(NetworkUpgrade::Sapling)
            .expect("the pinned params know Sapling on mainnet"),
    ))
}

// ── T0-1c-R2 harness (§4n) ───────────────────────────────────────────────────

/// T0-1c-R2: the record line for the R2 rows (`--nocapture`), [`record`]'s
/// shape under its own tag so the red-first artifact reads by item.
fn record_r2(row: &str, p: &Passed, receipt: &Receipt) {
    println!(
        "[T0-1c-R2 {row}] {} — {}",
        describe(p),
        where_it_stopped(receipt)
    );
}

/// A `BlockHeight` as the `u64` every fixture in this file is written in.
fn height_of(h: BlockHeight) -> u64 {
    u64::from(h.value())
}

/// A `u64` fixture height as the crate's `BlockHeight` (every fixture here is
/// far below `u32::MAX`; a fixture that is not is E5's business).
fn block(h: u64) -> BlockHeight {
    BlockHeight::new(u32::try_from(h).expect("a fixture height fits u32"))
}

/// The newest height the signed mainnet bundle carries — the T0-1c grade's
/// reference, read through the seam the grade itself reads
/// (`root_bind::newest_bundled_height`), never a literal.
fn newest_bundled_mainnet() -> u64 {
    u64::from(crate::root_bind::newest_bundled_height(Network::Main))
}

/// T0-1c-R2: a wallet that has SCANNED — anchored [`R2_SCAN_DEPTH`] below
/// [`TIP`] and driven once by an honest endpoint at [`TIP`], so it holds a
/// height of its own for M1's grade to read (module doc). Returns the wallet,
/// the honest pass and every height that pass served (for the D6 value check),
/// having asserted the precondition every row over it relies on: the pass
/// asked for ≥ 1 block range, every range lies inside the gap, the surface is
/// `UpToDate { TIP }`, and `scanned_tip()` reads `TIP`.
async fn scanned_wallet_at_tip(
    dir: &Path,
    vault: &Arc<dyn KeychainPort>,
    row: &str,
) -> (Wallet, Passed, Vec<u64>) {
    let anchor = TIP - R2_SCAN_DEPTH;
    let w = wallet_anchored_at(dir, vault, HOST_A, anchor).await;
    let honest = ProofEndpoint::honest();
    let receipt = honest.receipt();
    let served = honest.served_heights();
    let p = pass_over(&w, honest).await;
    record_r2(&format!("{row} honest pass at TIP"), &p, &receipt);
    let ranges = receipt.scanned_ranges();
    assert!(
        !ranges.is_empty(),
        "{row} — precondition: the honest pass SCANNED (≥ 1 block range asked); got none, \
         so the wallet holds no height of its own for the grade to read"
    );
    for (start, end) in &ranges {
        assert!(
            *start > anchor && *end <= TIP,
            "{row} — the honest pass scans only the gap ({anchor}, {TIP}]; got {start}..={end}"
        );
    }
    assert!(
        is_healthy_at_tip(&p.status),
        "{row} — precondition: an honest pass at TIP over a wallet {R2_SCAN_DEPTH} blocks \
         below it is UpToDate at TIP; got {}",
        describe(&p)
    );
    let scanned = w.scanned_tip().await.expect("scanned_tip").map(height_of);
    assert_eq!(
        scanned,
        Some(TIP),
        "{row} — precondition: the wallet's own scanned height reads TIP after the honest pass"
    );
    (w, p, served)
}

/// T0-1c-R2 (§4n decision 2, by distinguishability): does a relaunch snapshot
/// carry something the contract commit's unqualified stamp does not — a field
/// on `WalletState` or `SyncStamp` that does not exist at the contract commit,
/// or a `sync` that is not `Idle`? A row cannot name a field the implementer
/// has not minted, so this reads the SHAPE of the `Debug` rendering against the
/// field lists the contract commit has (both public, so naming them leaks
/// nothing); the adjudicator refines to the field's name. `true` never means
/// "correct" — it means "not the base's rendering", which is all a blind row
/// can say about a shape the contract left open.
fn relaunch_reads_qualified(snap: &WalletState) -> bool {
    const STATE_FIELDS_AT_THE_CONTRACT_COMMIT: &[&str] = &[
        "balance",
        "sync",
        "tor",
        "tip",
        "last_synced",
        "ever_synced",
        "rescan_rebuilding",
        "seq",
    ];
    const STAMP_FIELDS_AT_THE_CONTRACT_COMMIT: &[&str] = &["height", "at"];
    let state_fields = debug_fields_at_depth(&format!("{snap:?}"), 1);
    let stamp_fields = debug_fields_at_depth(&format!("{:?}", snap.last_synced), 2);
    let new_state_field = state_fields
        .iter()
        .any(|f| !STATE_FIELDS_AT_THE_CONTRACT_COMMIT.contains(&f.as_str()));
    let new_stamp_field = stamp_fields
        .iter()
        .any(|f| !STAMP_FIELDS_AT_THE_CONTRACT_COMMIT.contains(&f.as_str()));
    let sync_not_idle = !matches!(snap.sync, SyncStatus::Idle);
    new_state_field || new_stamp_field || sync_not_idle
}

/// The field names (`ident: `) that appear at exactly `depth` levels of
/// brackets inside a one-line `Debug` rendering — `WalletState { a: .., b: .. }`
/// has its fields at depth 1, `Some(SyncStamp { height: .., at: .. })` at 2.
/// Enum variants and type names are followed by `,`, ` {` or `(`, never `: `,
/// so they are not fields.
fn debug_fields_at_depth(rendered: &str, depth: usize) -> Vec<String> {
    let chars: Vec<char> = rendered.chars().collect();
    let mut level = 0usize;
    let mut fields = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '{' | '(' | '[' => {
                level += 1;
                i += 1;
            }
            '}' | ')' | ']' => {
                level = level.saturating_sub(1);
                i += 1;
            }
            c if level == depth && (c.is_ascii_alphabetic() || c == '_') => {
                let s = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                if chars.get(i) == Some(&':') && chars.get(i + 1) == Some(&' ') {
                    fields.push(chars[s..i].iter().collect());
                }
            }
            _ => i += 1,
        }
    }
    fields
}

/// T0-1c-R2 (G3): the stamped, account-less wallet — the ONE shape on which
/// `resolve_birthday`'s `min(tip − lag, estimate(created_at))` arm runs (module
/// doc). Built the way the crash window leaves it: the same `store` calls
/// `Wallet::create_with_vault` makes for a freshly-generated seed — provision
/// (the creation stamp and the completion marker land here) then open — with
/// the FR-24 eager `ensure_account` that follows them omitted. Asserts the
/// shape rather than assuming it: the stamp is present, no account exists.
/// The drive stays `controller_over`; nothing here touches the mechanism under
/// test.
async fn stamped_account_less_remnant(
    dir: &Path,
    vault: &Arc<dyn KeychainPort>,
    endpoint: &str,
) -> Wallet {
    let payload = crate::derivation::resolve_seed(
        SeedSource::raw_bytes_fresh(vec![0x2b; 32]).expect("valid fresh seed bytes"),
    )
    .expect("resolve the seed");
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    {
        let lock = crate::lifecycle::WalletLock::acquire(dir).expect("acquire the wallet lock");
        let intent = crate::store::ProvisionIntent {
            network: Network::Main,
            persistence: crate::store::PersistenceKind::SealedKeychain,
            seed: Some(&payload),
            repair_mode: crate::store::RepairMode::VerifySupplied,
            freshly_generated_at: Some(at),
        };
        crate::store::create_or_repair(&lock, &**vault, &intent)
            .expect("provision the store: stamp + marker");
        let opened =
            crate::store::open(&lock, &**vault, Network::Main).expect("the completed store opens");
        assert!(
            opened.created_at.is_some(),
            "the remnant carries the creation stamp (a freshly-generated provision writes it)"
        );
        let crate::store::OpenWallet { db, aux_db, .. } = opened;
        drop(db);
        drop(aux_db);
    }
    let w = Wallet::open_with_vault(cfg(dir, endpoint), Arc::clone(vault))
        .await
        .expect("open the remnant");
    assert!(
        !w.account_exists().await.expect("account_exists"),
        "the remnant has no account (the eager import never ran)"
    );
    w
}

// ════════════════════════════════════════════════════════════════════════════
// The named tests. Names are the contract's (§4c "Named tests", D9); a rename
// is a finding, not a liberty.
// ════════════════════════════════════════════════════════════════════════════

/// **D1 (refusal) / D2 / D3 / D5 / D6.** An endpoint that does not know the
/// Ironwood protocol produces a published status that says so — through the
/// shipped controller path, with no `tracing` subscriber, and with nothing
/// about the endpoint in it.
///
/// **Which mechanism, and the wrong reason (IT-10).** The receipt pins that the
/// pass ASKED for Ironwood and was refused at the open door (`refused == 1`),
/// while Sapling and Orchard were served (`streamed == 1` each) — so a status
/// that says degraded here is saying it about a refusal the pass actually
/// met, not about a pool it never fetched, and not about a transport fault on
/// a required pool (which would be `Stalled { EndpointUnreachable }` and is
/// excluded by `says_degraded`). The wrong-reason pass this row cannot see: a
/// surface that reports degraded for EVERY pass — that is
/// `a_healthy_sync_raises_no_degraded_pool_indicator`'s job.
///
/// **D5, and its limit.** The other surface checked is `Wallet::snapshot()`'s
/// `sync` field: it must not claim a healthy terminal while the controller
/// says degraded. Under seam S2 `snapshot()` reads `Inner.sync`, which the
/// injected controller may or may not share a channel with — so this half is
/// weak on purpose and says so; the strong half is the S1 surface.
#[tokio::test]
async fn a_refused_ironwood_pool_is_visible_without_a_tracing_subscriber() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = anchored_wallet(dir.path(), &vault, HOST_A).await;

    let client = ProofEndpoint::honest().refusing_ironwood();
    let receipt = client.receipt();
    let served = client.served_heights();
    let p = pass_over(&w, client).await;

    assert_eq!(
        receipt.refused(ShieldedProtocol::Ironwood),
        1,
        "IT-10: the pass asked for Ironwood roots and was refused at the open door"
    );
    assert_eq!(
        receipt.streamed(ShieldedProtocol::Sapling),
        full_sequence(ShieldedProtocol::Sapling).len(),
        "IT-10: Sapling served in full (T0-1d: a short serve would be a second reason)"
    );
    assert_eq!(
        receipt.streamed(ShieldedProtocol::Orchard),
        full_sequence(ShieldedProtocol::Orchard).len(),
        "IT-10: Orchard served in full"
    );

    assert!(
        says_degraded(&p.status),
        "D1/D2/D3: a refused Ironwood pool must be visible in the status the \
         controller publishes, with no subscriber installed. A healthy terminal \
         here is the shipped build claiming a full sync over a pool this server \
         will not serve — INC-020's own user-visible failure. Got {}",
        describe(&p)
    );
    let snap = w.snapshot().await.expect("snapshot");
    assert!(
        !is_terminal_healthy(&snap.sync),
        "D5 (surface checked: Wallet::snapshot().sync): the wallet must not report \
         a fully-healthy sync through the snapshot while the pass degraded a pool; \
         got snapshot.sync = {:?} against {}",
        snap.sync,
        describe(&p)
    );
    assert_carries_no_fixture_datum(&p.status, &served, HOST_A, "refused pool");
    w.close().await.expect("close");
}

/// **D1 (zero-root vs refusal).** "The server does not know this pool" and
/// "the server served nothing for it" are two sentences; the surface must not
/// collapse them (the A10 `CONTRACT_WRONG` conflation, one door over).
///
/// **IT-10.** Both wallets are new; both passes serve the same Sapling and
/// Orchard roots at the same tip; the only difference is Ironwood's answer —
/// refused (`refused == 1, streamed == 0`) versus opened-and-empty
/// (`refused == 0, opened == 1, streamed == 0`). The zero-root fixture sits
/// well above the NU6.3 activation on a chain whose bundled frontier already
/// shows a completed Ironwood subtree, so it is the stuck state, not the
/// honest pre-activation empty (that case is a contract finding — see the
/// report — not a row, because the datum carries no height for a surface to
/// discriminate on). The wrong-reason pass: two statuses unequal because of
/// the TIP — excluded by construction, one [`TIP`] for both.
#[tokio::test]
async fn a_zero_root_pool_is_distinguishable_from_a_refused_pool_at_the_surface() {
    let vault = test_vault();

    let dir_a = tempfile::tempdir().expect("tempdir");
    let w_a = anchored_wallet(dir_a.path(), &vault, HOST_A).await;
    let refused = ProofEndpoint::honest().refusing_ironwood();
    let r_refused = refused.receipt();
    let refused = pass_over(&w_a, refused).await;

    let dir_b = tempfile::tempdir().expect("tempdir");
    let w_b = anchored_wallet(dir_b.path(), &vault, HOST_A).await;
    let zero = ProofEndpoint::serving_no_ironwood_roots();
    let r_zero = zero.receipt();
    let zero = pass_over(&w_b, zero).await;

    assert_eq!(
        r_refused.refused(ShieldedProtocol::Ironwood),
        1,
        "IT-10: refused"
    );
    assert_eq!(
        r_zero.refused(ShieldedProtocol::Ironwood),
        0,
        "IT-10: not refused"
    );
    assert_eq!(r_zero.opened(ShieldedProtocol::Ironwood), 1, "IT-10: asked");
    assert_eq!(
        r_zero.streamed(ShieldedProtocol::Ironwood),
        0,
        "IT-10: served nothing"
    );

    assert!(
        says_degraded(&refused.status),
        "a refused pool must say degraded; got {}",
        describe(&refused)
    );
    assert!(
        says_degraded(&zero.status),
        "a zero-root serve above the activation, on a chain that has completed a \
         subtree, is the A10 stuck state and must say degraded; got {}",
        describe(&zero)
    );
    assert_ne!(
        refused.status,
        zero.status,
        "D1: 'the server does not know this pool' and 'the server served nothing \
         for it' must be two different published values — refused: {} / zero: {}",
        describe(&refused),
        describe(&zero)
    );
    w_a.close().await.expect("close");
    w_b.close().await.expect("close");
}

/// **D1 — the whole matrix.** Every outcome `PoolFetch` can express today —
/// healthy, refused, zero-root, height violation — is a distinct published
/// value, and the three degraded ones are none of them a healthy terminal.
/// Four new wallets, one tip.
///
/// **IT-10 for the violation.** The Ironwood fixture is one root at index 0
/// claiming 3,465,000, on a wallet that has scanned nothing yet: the gap
/// floor has no pair, the recorded arm and the scanned arm and the hash arm
/// all abstain (nothing recorded, nothing scanned), and the bundled frontier
/// at 3,452,280 already shows subtree 0 complete — so `bundled_frontier` is
/// the only arm that can refuse it, and
/// `sync_bind_proof::a_bundled_frontier_violation_is_refused_by_the_bundled_arm`
/// pins that code for this exact fixture at HEAD. The receipt shows the root
/// WAS streamed (`streamed == 1`), so "the double served nothing" is not what
/// distinguishes it from the zero-root case.
#[tokio::test]
async fn a_height_violation_is_distinguishable_from_a_protocol_refusal_at_the_surface() {
    let vault = test_vault();

    let dir_h = tempfile::tempdir().expect("tempdir");
    let w_h = anchored_wallet(dir_h.path(), &vault, HOST_A).await;
    let healthy = pass_over(&w_h, ProofEndpoint::honest()).await;

    let dir_r = tempfile::tempdir().expect("tempdir");
    let w_r = anchored_wallet(dir_r.path(), &vault, HOST_A).await;
    let refused = pass_over(&w_r, ProofEndpoint::honest().refusing_ironwood()).await;

    let dir_z = tempfile::tempdir().expect("tempdir");
    let w_z = anchored_wallet(dir_z.path(), &vault, HOST_A).await;
    let zero = pass_over(&w_z, ProofEndpoint::serving_no_ironwood_roots()).await;

    let dir_v = tempfile::tempdir().expect("tempdir");
    let w_v = anchored_wallet(dir_v.path(), &vault, HOST_A).await;
    let lying = ProofEndpoint::serving(
        full_sequence(ShieldedProtocol::Sapling),
        full_sequence(ShieldedProtocol::Orchard),
        IRONWOOD_BUNDLED_VIOLATION,
    );
    let r_lying = lying.receipt();
    let served_lying = lying.served_heights();
    let violation = pass_over(&w_v, lying).await;

    assert_eq!(
        r_lying.streamed(ShieldedProtocol::Ironwood),
        1,
        "IT-10: the violating root was streamed, so this is not the zero-root case"
    );
    assert_eq!(
        r_lying.refused(ShieldedProtocol::Ironwood),
        0,
        "IT-10: not a refusal"
    );

    assert!(
        is_healthy_at_tip(&healthy.status),
        "control: the honest endpoint is UpToDate at the fixture tip; got {}",
        describe(&healthy)
    );
    assert!(
        says_degraded(&violation.status),
        "D1: a pool whose served heights cannot be true must say degraded — \
         'the endpoint lied about heights' is the fourth sentence; got {}",
        describe(&violation)
    );
    assert_ne!(
        violation.status,
        refused.status,
        "D1: a height violation and a protocol refusal must be two published \
         values — violation: {} / refused: {}",
        describe(&violation),
        describe(&refused)
    );
    assert_ne!(
        violation.status,
        zero.status,
        "D1: a height violation and a zero-root serve must be two published \
         values — violation: {} / zero: {}",
        describe(&violation),
        describe(&zero)
    );
    assert_ne!(
        refused.status, zero.status,
        "D1: refused and zero-root must be two published values"
    );
    for (name, p) in [
        ("refused", &refused),
        ("zero", &zero),
        ("violation", &violation),
    ] {
        assert_ne!(
            p.status,
            healthy.status,
            "D1: the {name} status must not equal the healthy one — {}",
            describe(p)
        );
    }
    assert_carries_no_fixture_datum(&violation.status, &served_lying, HOST_A, "height violation");
    for w in [w_h, w_r, w_z, w_v] {
        w.close().await.expect("close");
    }
}

/// **D4 (first clause) / D7.** An honest endpoint raises nothing: the
/// published status is exactly `UpToDate` at the tip, on the first pass and
/// again on the next. This is the row that fails a surface that reports
/// degraded unconditionally — the mutation D7 names in reverse — and it is
/// EXPECTED GREEN at HEAD: it guards the correct half, and HEAD is correct on
/// this half.
///
/// It also carries the harness precondition (module doc): the pass requested
/// no block, so the healthy status is about ingested roots on a wallet with
/// nothing to scan, and the second pass's acceptance is the recorded and
/// bundled arms' answer.
#[tokio::test]
async fn a_healthy_sync_raises_no_degraded_pool_indicator() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = anchored_wallet(dir.path(), &vault, HOST_A).await;

    let first_client = ProofEndpoint::honest();
    let receipt = first_client.receipt();
    let first = pass_over(&w, first_client).await;
    let second = pass_over(&w, ProofEndpoint::honest()).await;

    for pool in [
        ShieldedProtocol::Sapling,
        ShieldedProtocol::Orchard,
        ShieldedProtocol::Ironwood,
    ] {
        assert!(
            receipt.streamed(pool) > 0,
            "IT-10: {} was served, so a healthy status is about served pools, not \
             about pools the pass never asked for",
            pool.as_str_name()
        );
    }
    assert!(
        is_healthy_at_tip(&first.status),
        "D4: an honest endpoint publishes UpToDate at the tip, nothing more; got {}",
        describe(&first)
    );
    assert_never_scanned(&receipt, "healthy");
    assert_eq!(
        first.status,
        second.status,
        "a second honest pass says the same thing — {} vs {}",
        describe(&first),
        describe(&second)
    );
    w.close().await.expect("close");
}

/// **S6 `watchdog` (plan §3.2).** A pass through `pass_over` that scans
/// nothing — the shape of most rows in this file — reports no progress until
/// it returns, so a stuck-sync watchdog running on real time could cancel it on
/// a slow enough machine (the 2026-09-28 nightly: four rows `cancelled: true`).
/// These rows assert the indicator, not the watchdog, so `controller_over`
/// builds with no watchdog (`Wallet::CONTROLLER_OVER_WINDOW`); the watchdog's
/// own rows run it on the paused clock. The healthy row's fixture: the verdict
/// is the pass's, it was not cancelled, and it is the healthy status.
///
/// Mutant `s6-watchdog-window-tiny`: the window set to 1 ms — red on
/// `cancelled`.
#[tokio::test]
async fn a_degraded_pool_verdict_is_not_the_stuck_watchdogs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = anchored_wallet(dir.path(), &vault, HOST_A).await;

    let client = ProofEndpoint::honest();
    let receipt = client.receipt();
    let passed = pass_over(&w, client).await;

    assert_never_scanned(&receipt, "watchdog");
    let pass = passed
        .result
        .as_ref()
        .unwrap_or_else(|e| panic!("the pass returns Ok; got {e:?}"));
    assert!(
        !pass.cancelled,
        "no watchdog cancels a controller_over pass, however slow the machine; got {}",
        describe(&passed)
    );
    assert!(
        is_healthy_at_tip(&passed.status),
        "the healthy fixture publishes UpToDate at the tip; got {}",
        describe(&passed)
    );
    w.close().await.expect("close");
}

/// **D4 (second clause).** A pool degraded on one pass and served honestly on
/// the next CLEARS: the published status is the healthy one again, exactly. A
/// sticky indicator is a new lie.
///
/// **IT-10.** Pass 2 is a NEW client (no `mem::take` double can serve a pool
/// twice; this one clones), and its receipt shows Ironwood streamed 4 roots,
/// so the clear is attributable to an honest serve and not to a pass that
/// skipped the pool. Red at HEAD on the precondition (HEAD never degrades);
/// the clearing half is what the row is named for.
#[tokio::test]
async fn a_degraded_pool_that_recovers_clears_the_indicator() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = anchored_wallet(dir.path(), &vault, HOST_A).await;

    let refusing = ProofEndpoint::honest().refusing_ironwood();
    let r_refusing = refusing.receipt();
    let degraded = pass_over(&w, refusing).await;
    let healer = ProofEndpoint::honest();
    let receipt = healer.receipt();
    let recovered = pass_over(&w, healer).await;

    assert_never_scanned(&r_refusing, "recovers (pass 1)");
    assert_eq!(
        receipt.streamed(ShieldedProtocol::Ironwood),
        IRONWOOD_TRUTHFUL.len(),
        "IT-10: the recovering pass really served the pool"
    );
    assert!(
        says_degraded(&degraded.status),
        "precondition: the refusing pass says degraded; got {}",
        describe(&degraded)
    );
    assert!(
        is_healthy_at_tip(&recovered.status),
        "D4: a pool that recovers clears the indicator — the next honest pass \
         publishes plain UpToDate at the tip; got {} (after {})",
        describe(&recovered),
        describe(&degraded)
    );
    w.close().await.expect("close");
}

/// **D4 (third clause) — and a FINDING, stated in the row.** "Switching
/// servers must not carry the old server's verdict forward." No endpoint
/// setter exists on `Wallet` at `a02a33e6` (`Inner.endpoint` is documented
/// lock-free immutable after open; no `set_endpoint`/`reconnect` anywhere in
/// the SDK, Rust or Dart), so the only switch a host can perform is
/// `close()` + `open()` with a new `WalletConfig.endpoint`. That is the shape
/// driven here.
///
/// What it can and cannot see: the fresh handle's `snapshot().sync` starts at
/// `Idle` and its first pass over an honest endpoint must be plain `UpToDate`.
/// At HEAD nothing persists, so the switch clause is GREEN BY CONSTRUCTION
/// (the row goes red at HEAD only on its degrade precondition). Post-join it
/// guards exactly one design: a persisted carrier (contract sub-question 2,
/// the `consensus_stamp` branch) that is not keyed to the endpoint. A carrier
/// that lives in memory can never fail it. Reported as a finding, not claimed
/// as a proof.
#[tokio::test]
async fn switching_endpoints_does_not_carry_the_old_servers_verdict_forward() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = anchored_wallet(dir.path(), &vault, HOST_A).await;
    let degraded = pass_over(&w, ProofEndpoint::honest().refusing_ironwood()).await;
    assert!(
        says_degraded(&degraded.status),
        "precondition: server A degrades the pool; got {}",
        describe(&degraded)
    );
    w.close().await.expect("close");

    // The switch: the same wallet, reopened against server B.
    let w = Wallet::open_with_vault(cfg(dir.path(), HOST_B), Arc::clone(&vault))
        .await
        .expect("reopen against the new endpoint");
    let before_any_pass = w.snapshot().await.expect("snapshot").sync;
    assert!(
        !says_degraded(&before_any_pass),
        "D4: server A's verdict must not be carried into a handle opened against \
         server B before B has answered anything; got snapshot.sync = {before_any_pass:?}"
    );
    let over_b = pass_over(&w, ProofEndpoint::honest()).await;
    assert!(
        is_healthy_at_tip(&over_b.status),
        "D4: an honest server B publishes plain UpToDate — no residue of A; got {}",
        describe(&over_b)
    );
    w.close().await.expect("close");
}

/// **D9.** A Sapling height violation — a REQUIRED pool, fatal to the pass —
/// is published as something a dead link cannot say.
///
/// **The positive control.** A genuinely dead link (the tip fetch fails with a
/// transport fault) is driven through the same seam first and pinned to
/// `Stalled { EndpointUnreachable }`: that is the sentence a dead link says
/// here, so the instrument can see it.
///
/// **Which mechanism refuses the violation (IT-10).** Pass 1 serves Sapling
/// `[558_822]` honestly and is asserted `UpToDate` (the record exists and the
/// index is writable). Pass 2 serves `[419_200]` at the same index. The A11
/// clauses cannot refuse it — one root, at the Sapling activation (asserted
/// from the compiled params, `h < floor` is false at equality), below the tip
/// — so the refuser is `apply_height_bind`'s required-pool arm
/// (`recorded_height`, equality-when-recorded, consulted before the bundled
/// and scanned arms), the arm whose `Err(endpoint_unusable())` D9 exists to
/// fix. The receipt shows the root was streamed. Orchard is honest and
/// identical on both passes, so it cannot be the refuser either.
///
/// **The wrong-reason pass.** A surface that renamed `endpoint_unusable()`
/// wholesale would pass this row AND the planted row below; a surface that
/// wired only the bind arm passes this row and fails the plant. Either way
/// this row cannot be passed by a pass that never reached the bind: a
/// transport fault is `EndpointUnreachable` and is excluded, and a local fault
/// is `Internal` and is excluded.
#[tokio::test]
async fn a_required_pool_height_violation_is_not_reported_as_an_unreachable_endpoint() {
    let vault = test_vault();

    let dir_dead = tempfile::tempdir().expect("tempdir");
    let w_dead = anchored_wallet(dir_dead.path(), &vault, HOST_A).await;
    let dead = pass_over(&w_dead, ProofEndpoint::dead()).await;
    assert_eq!(
        dead.status,
        SyncStatus::Stalled {
            reason: StallReason::EndpointUnreachable
        },
        "control: a dead link is Stalled {{ EndpointUnreachable }} on this seam; got {}",
        describe(&dead)
    );

    assert_eq!(
        sapling_activation_from_params(),
        SAPLING_COMPRESSED[0],
        "the compressed fixture sits AT the Sapling activation, so the A11 floor \
         clause (strictly below) cannot be the refuser"
    );
    assert!(
        SAPLING_COMPRESSED[0] < TIP,
        "and below the tip, so the ceiling cannot be"
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let w = anchored_wallet(dir.path(), &vault, HOST_A).await;
    let honest = pass_over(&w, ProofEndpoint::honest()).await;
    assert!(
        is_healthy_at_tip(&honest.status),
        "control: the truthful Sapling index 0 is accepted and recorded; got {}",
        describe(&honest)
    );
    let hostile = ProofEndpoint::serving(
        SAPLING_COMPRESSED,
        full_sequence(ShieldedProtocol::Orchard),
        IRONWOOD_TRUTHFUL,
    );
    let receipt = hostile.receipt();
    let served = hostile.served_heights();
    let violation = pass_over(&w, hostile).await;
    assert_eq!(
        receipt.streamed(ShieldedProtocol::Sapling),
        1,
        "IT-10: the compressed Sapling root was streamed"
    );

    assert!(
        matches!(violation.status, SyncStatus::Stalled { .. }),
        "a required-pool violation is fatal to the pass and is published as a \
         stall; got {}",
        describe(&violation)
    );
    assert_ne!(
        stall_reason(&violation.status),
        Some(StallReason::EndpointUnreachable),
        "D9: the link is fine and the server is not — rendering this as \
         EndpointUnreachable tells the user to check a connection that works \
         while their wallet has stopped syncing for a reason no reconnection \
         fixes; got {}",
        describe(&violation)
    );
    assert_ne!(
        violation.status, dead.status,
        "D9: the violation and a dead link must be two published values"
    );
    assert!(
        is_new_stall_reason(&violation.status),
        "D9 (S3): none of the five reasons that exist today says 'this server \
         lied about heights' — the adjudicator refines this to the variant's \
         name; got {}",
        describe(&violation)
    );
    assert_carries_no_fixture_datum(
        &violation.status,
        &[served.as_slice(), full_sequence(ShieldedProtocol::Sapling)].concat(),
        HOST_A,
        "required-pool violation",
    );
    w_dead.close().await.expect("close");
    w.close().await.expect("close");
}

/// **D10.** The same violation is distinguishable from a LOCAL fault — and
/// the remedy the correct reason implies, "switch servers", actually works.
///
/// `StallReason::Internal` means "the problem is on this device: repair or
/// restore from seed". A surface that collapsed a server's lie into it would
/// send a user to their seed phrase because a server lied. The row asserts
/// the published reason is not `Internal` (and not any of the five that exist,
/// so the whole sentence "this server lied" is one the enum can say), then
/// drives the thing a local fault could never do: the NEXT pass, over an
/// honest endpoint, is plain `UpToDate` again. A corrupt store does not heal
/// by changing servers; this must.
///
/// IT-10 is the D9 row's (same fixture, same arm, same receipt). Red at HEAD
/// through the "none of the five" clause; the `Internal` clause on its own is
/// green at HEAD by construction and guards the wrong remedy, which is what
/// D10 is for.
#[tokio::test]
async fn a_required_pool_height_violation_is_not_reported_as_a_local_fault() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = anchored_wallet(dir.path(), &vault, HOST_A).await;

    let honest = pass_over(&w, ProofEndpoint::honest()).await;
    assert!(
        is_healthy_at_tip(&honest.status),
        "control; got {}",
        describe(&honest)
    );

    let hostile = ProofEndpoint::serving(
        SAPLING_COMPRESSED,
        full_sequence(ShieldedProtocol::Orchard),
        IRONWOOD_TRUTHFUL,
    );
    let receipt = hostile.receipt();
    let violation = pass_over(&w, hostile).await;
    assert_eq!(
        receipt.streamed(ShieldedProtocol::Sapling),
        1,
        "IT-10: streamed"
    );

    assert_ne!(
        stall_reason(&violation.status),
        Some(StallReason::Internal),
        "D10: a server's lie must never render as a LOCAL fault — 'restore from \
         seed' is the wrong next step; got {}",
        describe(&violation)
    );
    assert!(
        is_new_stall_reason(&violation.status),
        "D10 (S3): the reason must be one that means 'this server', which none \
         of the five existing reasons does; got {}",
        describe(&violation)
    );

    // The remedy the reason implies, driven: a different (honest) server heals.
    let switched = pass_over(&w, ProofEndpoint::honest()).await;
    assert!(
        is_healthy_at_tip(&switched.status),
        "D10: 'switch servers' is the honest next step precisely because it \
         WORKS — an honest endpoint after the lying one publishes plain \
         UpToDate, which no local fault would; got {}",
        describe(&switched)
    );
    w.close().await.expect("close");
}

// ════════════════════════════════════════════════════════════════════════════
// PLANTED cases (IT-1 +A) — not named by the contract.
// ════════════════════════════════════════════════════════════════════════════

/// **PLANTED — D9's boundary, one mechanism over.** A Sapling completion
/// height ABOVE the endpoint's own reported tip is "a height that cannot be
/// true" in exactly D9's sense — the link is fine, the server is not, the
/// honest next step is a different server — and today it reaches the surface
/// as `Stalled { EndpointUnreachable }` through the SAME `endpoint_unusable()`
/// D9 names. But it is refused by `validate_root_sequence`'s ceiling clause
/// inside `fetch_subtree_roots`, BEFORE `apply_height_bind` is consulted, and
/// the contract's D9 text names only the bind's required-pool arm.
///
/// **PREDICTION:** the implementer wires the arm the contract names and
/// leaves `validate_root_sequence`'s `?` returning `endpoint_unusable()`, so
/// this row stays RED after the join while D9 goes green. If the adjudicator
/// rules the A11 clauses out of D9's scope, the ruling has to say why a
/// server claiming a subtree completed in a block it has not seen is a
/// connection problem — which is the sentence D9 was written to stop.
///
/// **IT-10.** One root (ordering trivial), far above the activation (floor
/// cannot fire), `TIP + 1` (only the ceiling can). The receipt shows Sapling
/// streamed 1 and Orchard opened 0: the pass aborted inside the Sapling arm,
/// before Orchard was asked — so the bind, which runs after all three pools
/// are collected, was never reached. A dead-link pass would show Sapling
/// opened 0.
#[tokio::test]
async fn a_required_pool_root_above_the_endpoints_own_tip_is_not_reported_as_an_unreachable_endpoint()
 {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = anchored_wallet(dir.path(), &vault, HOST_A).await;

    let above = ProofEndpoint::serving(
        SAPLING_ABOVE_TIP,
        full_sequence(ShieldedProtocol::Orchard),
        IRONWOOD_TRUTHFUL,
    );
    let receipt = above.receipt();
    let served = above.served_heights();
    let p = pass_over(&w, above).await;

    assert_eq!(
        receipt.streamed(ShieldedProtocol::Sapling),
        1,
        "IT-10: streamed"
    );
    assert_eq!(
        receipt.opened(ShieldedProtocol::Orchard),
        0,
        "IT-10: the pass aborted in the Sapling arm, before the bind (which needs \
         all three pools) could run — the refuser is the A11 ceiling clause"
    );
    assert!(
        matches!(p.status, SyncStatus::Stalled { .. }),
        "fatal to the pass; got {}",
        describe(&p)
    );
    assert_ne!(
        stall_reason(&p.status),
        Some(StallReason::EndpointUnreachable),
        "PLANT: a server that claims a subtree completed in a block above its own \
         tip has said something that cannot be true — the link is fine, the \
         server is not; got {}",
        describe(&p)
    );
    assert!(
        is_new_stall_reason(&p.status),
        "PLANT (S3): and the reason is one that means 'this server'; got {}",
        describe(&p)
    );
    assert_carries_no_fixture_datum(&p.status, &served, HOST_A, "above-tip violation");
    w.close().await.expect("close");
}

/// **PLANTED — the indicator must RE-RAISE, not only clear.** Degrade, recover,
/// degrade again: the third pass must say degraded, and say the same thing
/// the first did.
///
/// **PREDICTION:** the two carriers the contract's own precedent list offers
/// are a write-once flag (`ever_synced::set_once`) and a breadcrumb that is
/// set by one event and cleared by another (`rescan_rebuilding`). An
/// implementer who builds D4's "clears on recovery" over either shape gets a
/// latch that clears once and never re-arms — D4 green, this row red. The
/// datum has to be re-derived from EVERY pass, which is what the
/// `emit_synced` candidate does for free and a persisted flag does not.
#[tokio::test]
async fn a_pool_that_degrades_again_after_recovering_is_visible_again() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = anchored_wallet(dir.path(), &vault, HOST_A).await;

    let first = pass_over(&w, ProofEndpoint::honest().refusing_ironwood()).await;
    let recovered = pass_over(&w, ProofEndpoint::honest()).await;
    let again = ProofEndpoint::honest().refusing_ironwood();
    let receipt = again.receipt();
    let again = pass_over(&w, again).await;

    assert_eq!(
        receipt.refused(ShieldedProtocol::Ironwood),
        1,
        "IT-10: refused again"
    );
    assert!(
        says_degraded(&first.status),
        "precondition: the first refusal says degraded; got {}",
        describe(&first)
    );
    assert!(
        is_healthy_at_tip(&recovered.status),
        "precondition: recovery clears; got {}",
        describe(&recovered)
    );
    assert!(
        says_degraded(&again.status),
        "PLANT: a pool refused AGAIN after a recovery must be visible again — a \
         latch that clears once and never re-arms is D4 satisfied and the \
         indicator gone; got {}",
        describe(&again)
    );
    assert_eq!(
        again.status,
        first.status,
        "and it says the same thing it said the first time — {} vs {}",
        describe(&again),
        describe(&first)
    );
    w.close().await.expect("close");
}

// ════════════════════════════════════════════════════════════════════════════
// T0-1c (§4k). Names are the contract's; a rename is a finding, not a liberty.
// DEFECT rows are RED at the contract commit for the reason §4k states;
// CONTROL rows are GREEN there and name the behaviour a careless repair would
// break. Every row runs `assert_carries_no_fixture_datum` (E10).
// ════════════════════════════════════════════════════════════════════════════

/// **E1 — DEFECT.** A tip below the bundle's newest row is not rendered
/// "Up to date".
///
/// The endpoint reports 3,455,000 — below the newest bundled mainnet row and
/// above every pool's first completion (both asserted from the bundle and the
/// measured constants, not assumed) — and serves every pool a sequence honest
/// FOR THAT TIP: the generator's tip-filtered Sapling and Orchard sequences
/// (T0-1d; before it, one root each — a short serve, which under §4l (c) would
/// be a second reason for "degraded" and would leave this row's mutant nothing
/// to break), Ironwood `[3_451_206]`
/// (the bundle row `(3_452_280, 1)` agrees, asserted). Every A11 clause and
/// every bind arm passes, so nothing here refuses the data: the only thing
/// wrong is the tip, and the only evidence against it is the signed bundle.
///
/// **Asserted by DISTINGUISHABILITY** (S3): the published status is not a
/// healthy terminal, not a transient, and not one of the five stalls that
/// would each be a specific lie. The shape is the implementer's; the
/// adjudicator refines to the name. Whether the pools were asked or the pass
/// stopped first is the disposition's call — the row RECORDS which it saw
/// (`where_it_stopped`) rather than asserting either.
///
/// **IT-10.** Nothing scans (birthday = tip + 1), so a status that says
/// degraded here cannot be the scanned-tree-size arm's answer. At the
/// contract commit: `UpToDate { tip: 3455000 }` — `proven_complete_at_or_below`
/// reads 1 (row 3,452,280 ≤ tip), Ironwood served 1, no pool degraded, and
/// nothing compares the tip to anything the endpoint did not supply.
#[tokio::test]
async fn a_tip_below_the_bundles_newest_row_is_not_rendered_up_to_date() {
    let (newest, _) = newest_bundled_row(ShieldedProtocol::Ironwood);
    assert!(
        BEHIND_TIP < u64::from(newest),
        "premise: E1's tip sits below the bundle's newest mainnet row (got newest {newest})"
    );
    for (pool, first) in [
        ("sapling", SAPLING_TRUTHFUL[0]),
        ("orchard", ORCHARD_TRUTHFUL[0]),
        ("ironwood", IRONWOOD_HONEST_BELOW_BUNDLE[0]),
    ] {
        assert!(
            BEHIND_TIP > first,
            "premise: E1's tip is above {pool}'s first completion ({first})"
        );
    }
    assert!(
        crate::root_bind::bundled_counts(Network::Main, ShieldedProtocol::Ironwood)
            .contains(&(3_452_280, 1)),
        "premise: the bundle row (3_452_280, 1) agrees that one Ironwood subtree is \
         complete below E1's tip"
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_anchored_at(dir.path(), &vault, HOST_A, BEHIND_TIP).await;

    let behind = ProofEndpoint::honest_at(BEHIND_TIP, IRONWOOD_HONEST_BELOW_BUNDLE);
    let receipt = behind.receipt();
    let served = behind.served_heights();
    let p = pass_over(&w, behind).await;
    record("E1", &p, &receipt);

    assert_never_scanned(&receipt, "E1");
    assert!(
        says_degraded(&p.status),
        "E1: an endpoint whose reported tip is below the newest row of the signed \
         bundle this binary ships with is provably behind the chain, and the wallet \
         must not render a plain \"Up to date\" over it — that is INC-020's silence \
         at a lower price than the refusal T0-1b closed, holding every pool ~24k \
         blocks behind at once. Got {}; {}",
        describe(&p),
        where_it_stopped(&receipt)
    );
    assert_carries_no_fixture_datum(&p.status, &served, HOST_A, "E1");
    w.close().await.expect("close");
}

/// **E2 — DEFECT.** An under-reported tip does not hide a withheld pool —
/// INC-023's own geometry, verbatim (§4j row 6 (ii)).
///
/// Tip 3,451,205: one below Ironwood's first completion and below the first
/// bundled row that proves an Ironwood subtree complete, so at the contract
/// commit the tip filter switches the discriminator off (`proven` reads 0), a
/// zero Ironwood serve stays `Served { roots: 0 }`, and the wallet publishes
/// `UpToDate { tip: 3451205 }` — the attack. Sapling and Orchard are served
/// honestly, so nothing is refused and nothing else can be the reason.
///
/// Asserted by distinguishability: not a healthy terminal, not a transient,
/// not a dead link, not local. The receipt records whether Ironwood was asked
/// (report-and-continue) or the pass stopped first (refuse); the row does not
/// choose.
#[tokio::test]
async fn an_under_reported_tip_does_not_hide_a_withheld_pool() {
    let (newest, _) = newest_bundled_row(ShieldedProtocol::Ironwood);
    assert!(
        INC_023_TIP < u64::from(newest),
        "premise: INC-023's tip is below the newest bundled row ({newest})"
    );
    assert!(
        INC_023_TIP < IRONWOOD_HONEST_BELOW_BUNDLE[0] && INC_023_TIP < 3_452_280,
        "premise: INC-023's tip is below Ironwood's first completion and below the \
         first bundled row that proves one complete"
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_anchored_at(dir.path(), &vault, HOST_A, INC_023_TIP).await;

    let attacker = ProofEndpoint::honest_at(INC_023_TIP, &[]);
    let receipt = attacker.receipt();
    let served = attacker.served_heights();
    let p = pass_over(&w, attacker).await;
    record("E2", &p, &receipt);

    assert_never_scanned(&receipt, "E2");
    assert!(
        says_degraded(&p.status),
        "E2 (INC-023): an endpoint that under-reports its tip to just below the \
         first bundled proof serves zero Ironwood roots and must not be rendered a \
         healthy terminal — the tip filter inside the Withheld discriminator lets \
         the endpoint switch the badge off with a number only it supplied. Got {}; {}",
        describe(&p),
        where_it_stopped(&receipt)
    );
    assert_carries_no_fixture_datum(&p.status, &served, HOST_A, "E2");
    w.close().await.expect("close");
}

/// **E3 — CONTROL.** A tip above the bundle with a zero serve is `Withheld`
/// (§4j row 6 (i)). GREEN at the contract commit (T0-1b folded) and it must
/// stay green: it is the row that keeps a floor from being built by deleting
/// the badge, and the row that turns red if the tip filter's `<=` becomes `>=`
/// (mutant (c)) — at a tip above every bundled row the flipped filter admits
/// no row, `proven` reads 0, and the zero serve degrades to `Served { 0 }`.
///
/// Named, not distinguished: `PoolService::Withheld { proven }` exists at
/// HEAD, so naming it leaks nothing. The row asserts the premise the contract
/// states — `TIP` is above the newest bundled height, read from
/// `bundled_counts` — so the control cannot silently drift into E2's geometry
/// if the bundle is re-derived.
#[tokio::test]
async fn a_tip_above_the_bundle_with_a_zero_serve_is_withheld() {
    let (newest, newest_complete) = newest_bundled_row(ShieldedProtocol::Ironwood);
    assert!(
        TIP > u64::from(newest),
        "premise: the file-wide TIP ({TIP}) sits above the newest bundled row ({newest})"
    );
    assert!(
        newest_complete >= 1,
        "premise: the newest bundled row proves at least one Ironwood subtree complete"
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = anchored_wallet(dir.path(), &vault, HOST_A).await;

    let zero = ProofEndpoint::serving_no_ironwood_roots();
    let receipt = zero.receipt();
    let served = zero.served_heights();
    let p = pass_over(&w, zero).await;
    record("E3", &p, &receipt);

    assert_eq!(
        receipt.opened(ShieldedProtocol::Ironwood),
        1,
        "IT-10: asked"
    );
    assert_eq!(
        receipt.streamed(ShieldedProtocol::Ironwood),
        0,
        "IT-10: served nothing"
    );
    assert_never_scanned(&receipt, "E3");
    match &p.status {
        SyncStatus::UpToDateDegraded { tip, pools } => {
            assert_eq!(u64::from(tip.value()), TIP, "E3: at the fixture tip");
            assert!(
                matches!(pools.ironwood, PoolService::Withheld { proven } if proven >= 1),
                "E3: a zero Ironwood serve at a tip above the bundle, where the bundle \
                 proves >= 1 subtree complete, is `Withheld {{ proven >= 1 }}`; got \
                 pools = {pools:?}"
            );
        }
        other => panic!(
            "E3: a zero serve above the bundle publishes UpToDateDegraded with \
             ironwood Withheld — the T0-1b badge, which a tip floor must not delete; \
             got {other:?} ({})",
            describe(&p)
        ),
    }
    assert_carries_no_fixture_datum(&p.status, &served, HOST_A, "E3");
    w.close().await.expect("close");
}

/// **E4 — DEFECT.** A wrong-chain server says "switch servers", not "check
/// your connection" (§4j row 2).
///
/// `server_identity` answers with a TESTNET identity — chain name and Sapling
/// activation both testnet's, the branch id honestly computed for testnet at
/// the tip — to a mainnet wallet. `provision::endpoint_identity`'s chain guard
/// returns `WalletError::NetworkMismatch` from `evaluate_consensus`, the first
/// thing `run_engine_pass` does, so the receipt shows NO pool stream was
/// opened and nothing scanned: the identity check precedes the roots.
///
/// At the contract commit `stall_for`'s `_ =>` arm renders it
/// `Stalled { EndpointUnreachable }` — "check your connection", every pass,
/// forever, for the one fault where switching servers is the only remedy.
/// Named: `StallReason::EndpointMisbehaving` landed with D9.
#[tokio::test]
async fn a_wrong_chain_server_says_switch_servers_not_check_your_connection() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = anchored_wallet(dir.path(), &vault, HOST_A).await;

    let testnet = ProofEndpoint::honest().on_the_wrong_chain();
    let receipt = testnet.receipt();
    let served = testnet.served_heights();
    let p = pass_over(&w, testnet).await;
    record("E4", &p, &receipt);

    for pool in [
        ShieldedProtocol::Sapling,
        ShieldedProtocol::Orchard,
        ShieldedProtocol::Ironwood,
    ] {
        assert_eq!(
            receipt.opened(pool),
            0,
            "IT-10: the identity check precedes the roots — no {} stream may be opened \
             for a server on the wrong chain",
            pool.as_str_name()
        );
    }
    assert_never_scanned(&receipt, "E4");
    assert!(
        matches!(p.status, SyncStatus::Stalled { .. }),
        "a wrong-chain server is fatal to the pass; got {}",
        describe(&p)
    );
    assert_ne!(
        stall_reason(&p.status),
        Some(StallReason::EndpointUnreachable),
        "E4: the link is fine and the server is on another chain — rendering this as \
         EndpointUnreachable tells the user to check a connection that works, on \
         every pass, for the one fault only a different server fixes; got {}",
        describe(&p)
    );
    assert_ne!(
        stall_reason(&p.status),
        Some(StallReason::Internal),
        "E4: not a local fault either — 'restore from seed' is the wrong next step; got {}",
        describe(&p)
    );
    assert_eq!(
        p.status,
        SyncStatus::Stalled {
            reason: StallReason::EndpointMisbehaving
        },
        "E4: the honest sentence is 'this server' — switch servers; got {}",
        describe(&p)
    );
    assert_carries_no_fixture_datum(&p.status, &served, HOST_A, "E4");
    w.close().await.expect("close");
}

/// **E5 — DEFECT.** The provisioning tip's `u32` collapse says the same thing
/// `fetch_tip` says.
///
/// **The positive control, driven first.** A PROVISIONED wallet whose endpoint
/// reports a tip past `u32` is refused by `sync::fetch_tip` (`endpoint_unusable`)
/// and publishes `Stalled { EndpointMisbehaving }` at the contract commit —
/// that is the sentence the contract says both sites must say.
///
/// **The row.** An UNPROVISIONED wallet (no account — asserted, not assumed:
/// `SeedSource::raw_bytes` is restore-shaped, so create imports nothing and
/// `run_engine_pass` takes the provisioning branch) whose endpoint answers the
/// same tip. `evaluate_consensus` saturates the identity's `block_height` and
/// passes; `provision_account → resolve_birthday` then reads `tip_height`,
/// fails the `u32` conversion, and at the contract commit constructs
/// `Sync { EndpointUnreachable }` directly — the comment beside it says there
/// is no "endpoint misbehaved" reason, which has been false since D9. Two
/// answers to one question; the row asserts they are the same answer.
///
/// IT-10: the receipt shows no pool asked and nothing scanned on either
/// wallet — both passes stop at the tip.
#[tokio::test]
async fn a_tip_beyond_u32_at_provisioning_says_switch_servers() {
    let vault = test_vault();

    let dir_p = tempfile::tempdir().expect("tempdir");
    let w_p = anchored_wallet(dir_p.path(), &vault, HOST_A).await;
    let lying = ProofEndpoint::honest().at_tip(TIP_BEYOND_U32);
    let r_p = lying.receipt();
    let provisioned = pass_over(&w_p, lying).await;
    record("E5 control (provisioned)", &provisioned, &r_p);
    assert_never_scanned(&r_p, "E5 control");
    assert_eq!(
        provisioned.status,
        SyncStatus::Stalled {
            reason: StallReason::EndpointMisbehaving
        },
        "control: on a provisioned wallet a tip past u32 is refused by fetch_tip as \
         EndpointMisbehaving (the sentence both sites must say); got {}",
        describe(&provisioned)
    );

    let dir_u = tempfile::tempdir().expect("tempdir");
    let w_u = unprovisioned_wallet(dir_u.path(), &vault, HOST_A).await;
    assert!(
        !w_u.account_exists().await.expect("account_exists"),
        "precondition: the wallet has no account, so the pass takes the provisioning branch"
    );
    let lying = ProofEndpoint::honest().at_tip(TIP_BEYOND_U32);
    let r_u = lying.receipt();
    let served = lying.served_heights();
    let p = pass_over(&w_u, lying).await;
    record("E5", &p, &r_u);

    for pool in [
        ShieldedProtocol::Sapling,
        ShieldedProtocol::Orchard,
        ShieldedProtocol::Ironwood,
    ] {
        assert_eq!(
            r_u.opened(pool),
            0,
            "IT-10: provisioning fails at the tip, before any root stream"
        );
    }
    assert_never_scanned(&r_u, "E5");
    assert!(
        !w_u.account_exists().await.expect("account_exists"),
        "IT-10: nothing was provisioned — the pass stopped at the tip"
    );
    assert!(
        matches!(p.status, SyncStatus::Stalled { .. }),
        "fatal to the pass; got {}",
        describe(&p)
    );
    assert_ne!(
        stall_reason(&p.status),
        Some(StallReason::EndpointUnreachable),
        "E5: a tip beyond u32 is a server that answered with garbage, not a dead \
         link — the provisioning arm must not collapse it to 'check your \
         connection' while fetch_tip says 'switch servers'; got {}",
        describe(&p)
    );
    assert_eq!(
        p.status,
        SyncStatus::Stalled {
            reason: StallReason::EndpointMisbehaving
        },
        "E5: the same lie says the same thing at provisioning as on a sync pass; got {}",
        describe(&p)
    );
    assert_eq!(
        p.status,
        provisioned.status,
        "E5: one question, one answer — unprovisioned {} vs provisioned {}",
        describe(&p),
        describe(&provisioned)
    );
    assert_carries_no_fixture_datum(&p.status, &served, HOST_A, "E5");
    w_p.close().await.expect("close");
    w_u.close().await.expect("close");
}

/// **E6 — DEFECT.** A behind endpoint that catches up clears the indicator
/// (D4's class).
///
/// Pass 1 is E1's (tip 3,455,000, honest for that tip). Pass 2, the SAME
/// wallet, is an honest endpoint at [`TIP`] serving all four Ironwood roots:
/// the published status is plain `UpToDate { tip: TIP }`, and the receipt
/// shows pass 2 asked for every pool (streamed > 0 each) — so the clear is
/// attributable to an endpoint that caught up, not to a pass that skipped
/// the roots. Held nowhere durable (the `UpToDateDegraded` precedent) or the
/// implementer states the durable cost.
///
/// **This pass scans** (module doc): the wallet is 20,499 blocks below the
/// new tip, so pass 2 ingests the roots, records the tip and scans the gap
/// over empty fixture blocks. The row asserts every range it requested lies
/// inside `(BEHIND_TIP, TIP]` — the gap, and nothing below the birthday.
///
/// **The control, driven first on its own wallet:** the same two passes with
/// pass 1 at the same tip — GREEN at the contract commit on both (`UpToDate`
/// at 3,455,000 then at `TIP`), so a red on the row's second clause after the
/// join is the indicator failing to clear, never the scan failing. Red at the
/// contract commit on its precondition only (pass 1 says nothing).
#[tokio::test]
async fn a_behind_endpoint_that_catches_up_clears_the_indicator() {
    let vault = test_vault();

    // The control: the geometry works at HEAD independent of any floor.
    let dir_c = tempfile::tempdir().expect("tempdir");
    let w_c = wallet_anchored_at(dir_c.path(), &vault, HOST_A, BEHIND_TIP).await;
    let c1 = pass_over(
        &w_c,
        ProofEndpoint::honest_at(BEHIND_TIP, IRONWOOD_HONEST_BELOW_BUNDLE),
    )
    .await;
    let healer_c = ProofEndpoint::honest();
    let r_c2 = healer_c.receipt();
    let c2 = pass_over(&w_c, healer_c).await;
    record("E6 control pass 2", &c2, &r_c2);
    assert!(
        is_healthy_at_tip(&c2.status),
        "control: the same wallet driven at BEHIND_TIP then at TIP reaches UpToDate at \
         TIP when no floor is in play (pass 1 {}); got {} — if this is red the scan \
         geometry broke, not the indicator",
        describe(&c1),
        describe(&c2)
    );

    // The row.
    let dir = tempfile::tempdir().expect("tempdir");
    let w = wallet_anchored_at(dir.path(), &vault, HOST_A, BEHIND_TIP).await;
    let behind = ProofEndpoint::honest_at(BEHIND_TIP, IRONWOOD_HONEST_BELOW_BUNDLE);
    let r1 = behind.receipt();
    let served_behind = behind.served_heights();
    let first = pass_over(&w, behind).await;
    record("E6 pass 1", &first, &r1);
    let healer = ProofEndpoint::honest();
    let r2 = healer.receipt();
    let served_healer = healer.served_heights();
    let caught_up = pass_over(&w, healer).await;
    record("E6 pass 2", &caught_up, &r2);

    assert_never_scanned(&r1, "E6 (pass 1)");
    for pool in [
        ShieldedProtocol::Sapling,
        ShieldedProtocol::Orchard,
        ShieldedProtocol::Ironwood,
    ] {
        assert!(
            r2.streamed(pool) > 0,
            "IT-10: pass 2 asked for {} and was served, so the clear is about an \
             endpoint that caught up, not a pass that skipped the pool",
            pool.as_str_name()
        );
    }
    for (start, end) in r2.scanned_ranges() {
        assert!(
            start > BEHIND_TIP && end <= TIP,
            "E6 geometry: pass 2 scans only the gap the chain advanced over; got a \
             request for {start}..={end}"
        );
    }
    assert!(
        says_degraded(&first.status),
        "precondition (E1): the behind pass says something a healthy wallet cannot; got {}",
        describe(&first)
    );
    assert!(
        is_healthy_at_tip(&caught_up.status),
        "E6: once the endpoint's tip is above the bundle and every pool is served, the \
         indicator clears — plain UpToDate at TIP, nothing sticky; got {} (after {})",
        describe(&caught_up),
        describe(&first)
    );
    assert_carries_no_fixture_datum(
        &caught_up.status,
        &[served_behind, served_healer].concat(),
        HOST_A,
        "E6",
    );
    w_c.close().await.expect("close");
    w.close().await.expect("close");
}

// ════════════════════════════════════════════════════════════════════════════
// T0-1c PLANTED cases (IT-1 +A) — the boundary the floor does not name.
// ════════════════════════════════════════════════════════════════════════════

/// **PLANTED — CONTROL.** An endpoint exactly AT the bundle's newest row is
/// not behind the binary's data, and its honest pass is plain `UpToDate`.
///
/// **PREDICTION:** the floor is written `tip <= newest` instead of
/// `tip < newest` — the off-by-one every comparison invites — and on the day
/// a release ships, every honest, current endpoint whose tip equals the row
/// the binary was built from is stalled or badged as behind. E1 (4,780 below)
/// and E2 cannot see it; P3's "fires on no honest, current endpoint today" is
/// broken exactly where P3's margin is zero. GREEN at the contract commit
/// (`UpToDate { tip: 3459780 }`) and must stay green.
///
/// The tip is READ from the bundle, so the row follows a re-derived bundle
/// rather than drifting off it. One Ironwood root is honest here (the newest
/// row proves exactly one complete, asserted, and the second completion
/// 3,463,000 is above it).
#[tokio::test]
async fn an_endpoint_exactly_at_the_bundles_newest_row_is_not_behind() {
    let (newest, complete) = newest_bundled_row(ShieldedProtocol::Ironwood);
    let at_newest = u64::from(newest);
    assert_eq!(
        complete, 1,
        "premise: the newest mainnet row proves exactly one Ironwood subtree complete, \
         so a one-root serve is honest at it"
    );
    assert!(
        at_newest < IRONWOOD_TRUTHFUL[1],
        "premise: the second Ironwood completion ({}) is above the newest row",
        IRONWOOD_TRUTHFUL[1]
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_anchored_at(dir.path(), &vault, HOST_A, at_newest).await;

    let current = ProofEndpoint::honest_at(at_newest, IRONWOOD_HONEST_BELOW_BUNDLE);
    let receipt = current.receipt();
    let served = current.served_heights();
    let p = pass_over(&w, current).await;
    record("PLANT at-newest", &p, &receipt);

    for pool in [
        ShieldedProtocol::Sapling,
        ShieldedProtocol::Orchard,
        ShieldedProtocol::Ironwood,
    ] {
        assert!(
            receipt.streamed(pool) > 0,
            "IT-10: {} served — the healthy status is about served pools",
            pool.as_str_name()
        );
    }
    assert_never_scanned(&receipt, "PLANT at-newest");
    assert!(
        is_up_to_date_at(&p.status, at_newest),
        "PLANT: an endpoint whose tip EQUALS the newest bundled row is not behind the \
         data this binary shipped with — a floor that fires at equality stalls every \
         honest endpoint on release day; got {}",
        describe(&p)
    );
    assert_carries_no_fixture_datum(&p.status, &served, HOST_A, "PLANT at-newest");
    w.close().await.expect("close");
}

/// **PLANTED — DEFECT.** An endpoint ONE block below the bundle's newest row
/// is behind the binary's data, and says so.
///
/// **PREDICTION:** the implementer adds slack to the floor — a reorg depth
/// (`REORG_MAX_BLOCKS`, 100), "a day", any tolerance meant to avoid flapping
/// — so a tip within the slack of the newest row passes. E1 sits 4,780 blocks
/// below and stays green; this row, 1 below, goes red. The contract's floor
/// has no slack and P3 says the looseness it already has GROWS with binary
/// age, so a tolerance buys nothing an honest endpoint needs and gives an
/// under-reporter the exact width of the tolerance for free. RED at the
/// contract commit: `UpToDate { tip: 3459779 }`, the same mechanism as E1.
#[tokio::test]
async fn an_endpoint_one_block_below_the_bundles_newest_row_is_behind() {
    let (newest, complete) = newest_bundled_row(ShieldedProtocol::Ironwood);
    let one_below = u64::from(newest) - 1;
    assert_eq!(
        complete, 1,
        "premise: one Ironwood root is honest at the newest row"
    );
    assert!(
        one_below > IRONWOOD_HONEST_BELOW_BUNDLE[0] && one_below < IRONWOOD_TRUTHFUL[1],
        "premise: one Ironwood root is honest one block below the newest row too"
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_anchored_at(dir.path(), &vault, HOST_A, one_below).await;

    let just_behind = ProofEndpoint::honest_at(one_below, IRONWOOD_HONEST_BELOW_BUNDLE);
    let receipt = just_behind.receipt();
    let served = just_behind.served_heights();
    let p = pass_over(&w, just_behind).await;
    record("PLANT one-below", &p, &receipt);

    assert_never_scanned(&receipt, "PLANT one-below");
    assert!(
        says_degraded(&p.status),
        "PLANT: a tip one block below the newest bundled row is below the newest \
         bundled row — the floor has no slack, because any slack is exactly the \
         width an under-reporter gets for free; got {}; {}",
        describe(&p),
        where_it_stopped(&receipt)
    );
    assert_carries_no_fixture_datum(&p.status, &served, HOST_A, "PLANT one-below");
    w.close().await.expect("close");
}

// ════════════════════════════════════════════════════════════════════════════
// T0-1d (§4l part (c)). Names are the contract's; a rename is a finding, not a
// liberty. DEFECT rows are RED at the contract commit for the reason §4l
// states; CONTROL rows are GREEN there. Every row runs the D6 value check.
// ════════════════════════════════════════════════════════════════════════════

/// **F6 — CONTROL, and the generator's receipt.** Every pool served the
/// generator's FULL sequence at [`TIP`] — `m` roots where the bundle proves `m`
/// there — publishes plain `UpToDate` at the tip, and every root is written and
/// read back per pool through the observer connection (the cross-wire clause:
/// Orchard and Ironwood are the same Rust type, so a swap between them is
/// invisible to the compiler and visible only in the wrong table).
///
/// **Why it exists** (§4l F6): so (c) cannot be built by grading every serve as
/// short. GREEN at the contract commit — a full honest sequence is healthy
/// today too — and the row that goes red under `served <= proven` in place of
/// `served < proven`, or under a rule graded against the whole bundle at a tip
/// that admits every row (here they coincide; the behind-tip plant below is
/// where they part).
///
/// **The generator's own receipt.** Before the wallet is opened the row prints
/// one line per pool (proven, windows, widths, the busiest window, the min gap)
/// and runs [`Generated::assert_bind_consistent`]: every window filled (P8 —
/// an unfillable window is REPORTED here, never padded), every height inside
/// its window, at least [`COMPLETION_GAP_FLOOR`] from its neighbours, within
/// `[activation, tip]`. `proven` is also checked against the bundle's newest
/// row, which at this tip is every row admitted (P9).
///
/// **IT-10.** Nothing scans; the receipt shows every pool streamed exactly its
/// sequence; the store shows every height at its index. A healthy status here
/// is about `proven` roots served and written, not about a pass that skipped a
/// pool.
#[tokio::test]
async fn a_full_bind_consistent_sequence_from_the_bundle_is_healthy() {
    let pools = [
        ShieldedProtocol::Sapling,
        ShieldedProtocol::Orchard,
        ShieldedProtocol::Ironwood,
    ];
    let generated: Vec<Generated> = pools
        .iter()
        .map(|&p| bind_consistent_sequence(p, TIP))
        .collect();
    for g in &generated {
        println!("[T0-1d F6 generator] {}", g.receipt());
        g.assert_bind_consistent();
        let (newest, complete) = newest_bundled_row(g.pool);
        assert!(
            TIP > u64::from(newest),
            "premise (P9): TIP is above the newest bundled row, so every row is admitted"
        );
        assert_eq!(
            g.proven,
            usize::try_from(complete).expect("fits"),
            "at a tip above the newest row `proven` is the newest row's count for {}",
            g.pool.as_str_name()
        );
        assert!(
            g.proven >= 1,
            "premise: the bundle proves at least one {} subtree complete",
            g.pool.as_str_name()
        );
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = anchored_wallet_and_key(dir.path(), &vault, HOST_A, BIRTHDAY_ANCHOR).await;
    let obs = observe(dir.path(), &key);

    let full = ProofEndpoint::serving(
        &generated[0].heights,
        &generated[1].heights,
        &generated[2].heights,
    );
    let receipt = full.receipt();
    let served = full.served_heights();
    let p = pass_over(&w, full).await;
    record("F6", &p, &receipt);

    assert_never_scanned(&receipt, "F6");
    for g in &generated {
        assert_eq!(
            receipt.streamed(g.pool),
            g.heights.len(),
            "IT-10: {} streamed its full sequence",
            g.pool.as_str_name()
        );
    }
    assert!(
        is_healthy_at_tip(&p.status),
        "F6: every pool served exactly the number of roots the signed bundle proves \
         complete at this tip is the honest, current endpoint, and it publishes plain \
         UpToDate — a short rule that grades this serve degraded is a rule that stalls \
         every honest server on the day it ships. Got {}",
        describe(&p)
    );
    for g in &generated {
        assert_eq!(
            recorded_heights(&obs, g.pool),
            g.heights,
            "F6 cross-wire: the {} shard table holds exactly the {} sequence, at its indices",
            g.pool.as_str_name(),
            g.pool.as_str_name()
        );
    }
    assert_carries_no_fixture_datum(&p.status, &served, HOST_A, "F6");

    // Idempotence at this size: the same full serve again says the same thing.
    let again = pass_over(
        &w,
        ProofEndpoint::serving(
            &generated[0].heights,
            &generated[1].heights,
            &generated[2].heights,
        ),
    )
    .await;
    assert_eq!(
        again.status,
        p.status,
        "a second full re-serve is accepted and says the same thing — {} vs {}",
        describe(&again),
        describe(&p)
    );
    w.close().await.expect("close");
}

/// **F5 — DEFECT.** A strict prefix above a proven boundary is not `UpToDate`.
///
/// Sapling is served ONE root — the generator's index 0, the shape every honest
/// fixture in this file served until T0-1d — where the bundle proves `m` (1128)
/// complete at [`TIP`]; Orchard and Ironwood are served in full. The prefix is
/// honest as far as it goes (inside its window, accepted by every arm, and
/// WRITTEN — asserted through the observer, so a degraded status here is about
/// shortness and not about a refusal). At the contract commit `withheld` fires
/// on `Served { roots: 0 }` only, so the pass publishes `UpToDate { tip }`
/// with subtrees `1..1128` — the ommers every later Sapling note's witness needs
/// — silently unserved: INC-020's stuck state for a wallet whose notes sit
/// above subtree 0, with a healthy badge (§4l (c)).
///
/// **Asserted by distinguishability** (S3): not a healthy terminal, not a
/// transient, not one of the five stalls that would each be a specific lie —
/// and not `EndpointBehind`, because the tip is above the newest row and
/// "behind" would be a different lie. The shape of "short" is the
/// implementer's (§4l decision 2); if the status carries a pools report the
/// Sapling line is not a clean `Served` while the two full pools' lines are.
#[tokio::test]
async fn a_strict_prefix_above_a_proven_boundary_is_not_up_to_date() {
    let full = full_sequence(ShieldedProtocol::Sapling);
    let prefix = &full[..1];
    assert!(
        full.len() > prefix.len(),
        "premise: the bundle proves more Sapling subtrees complete at TIP ({}) than the \
         prefix serves ({})",
        full.len(),
        prefix.len()
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = anchored_wallet_and_key(dir.path(), &vault, HOST_A, BIRTHDAY_ANCHOR).await;
    let obs = observe(dir.path(), &key);

    let short = ProofEndpoint::serving(
        prefix,
        full_sequence(ShieldedProtocol::Orchard),
        IRONWOOD_TRUTHFUL,
    );
    let receipt = short.receipt();
    let served = short.served_heights();
    let p = pass_over(&w, short).await;
    record("F5", &p, &receipt);

    assert_never_scanned(&receipt, "F5");
    assert_eq!(
        receipt.streamed(ShieldedProtocol::Sapling),
        prefix.len(),
        "IT-10: the prefix was streamed"
    );
    assert_eq!(
        recorded_heights(&obs, ShieldedProtocol::Sapling),
        prefix,
        "IT-10: the prefix was ACCEPTED and WRITTEN — so whatever the surface says below \
         is about a SHORT serve, not a refused one"
    );
    assert_eq!(
        recorded_heights(&obs, ShieldedProtocol::Orchard),
        full_sequence(ShieldedProtocol::Orchard),
        "IT-10: Orchard was served in full and written"
    );
    assert_eq!(
        recorded_heights(&obs, ShieldedProtocol::Ironwood),
        IRONWOOD_TRUTHFUL,
        "IT-10: Ironwood was served in full and written"
    );
    assert!(
        !matches!(p.status, SyncStatus::EndpointBehind { .. }),
        "F5: the tip is above the bundle's newest row; 'behind' is the wrong sentence \
         for a short serve. Got {}",
        describe(&p)
    );
    assert!(
        says_degraded(&p.status),
        "F5: Sapling served {} root where the signed bundle proves {} subtrees complete \
         at this tip — every subtree from 1 up is unserved, and those roots are the \
         ommers every later note's witness needs. Rendering this UpToDate is INC-020's \
         silence with a healthy badge. Got {}",
        prefix.len(),
        full.len(),
        describe(&p)
    );
    if let SyncStatus::UpToDateDegraded { tip, pools } = &p.status {
        assert_eq!(u64::from(tip.value()), TIP, "F5: at the fixture tip");
        assert!(
            !matches!(pools.sapling, PoolService::Served { .. }),
            "F5: the Sapling pool line must not be a clean serve; got pools = {pools:?}"
        );
        assert!(
            matches!(pools.orchard, PoolService::Served { roots }
                if usize::try_from(roots).expect("fits") == full_sequence(ShieldedProtocol::Orchard).len()),
            "F5: Orchard, served in full, is a clean serve; got pools = {pools:?}"
        );
        assert!(
            matches!(pools.ironwood, PoolService::Served { roots }
                if usize::try_from(roots).expect("fits") == IRONWOOD_TRUTHFUL.len()),
            "F5: Ironwood, served in full, is a clean serve; got pools = {pools:?}"
        );
    } else {
        println!(
            "[T0-1d F5] the surface carried no pool line: {:?}",
            p.status
        );
    }
    assert_carries_no_fixture_datum(&p.status, &served, HOST_A, "F5");
    w.close().await.expect("close");
}

// ════════════════════════════════════════════════════════════════════════════
// T0-1d PLANTED cases (IT-1 +A) — the cells the floor does not name.
// ════════════════════════════════════════════════════════════════════════════

/// **PLANTED — DEFECT.** A serve ONE root short of the proven count is short.
///
/// **PREDICTION:** the comparison is written with slack — a tolerance of one
/// shard ("the newest subtree may still be completing"), a fraction, or
/// `proven - served > 1` — so F5's one-of-1128 goes red-to-green while the
/// smallest short serve stays healthy. The T0-1c one-below plant's class: any
/// slack is exactly the width a withholding endpoint gets for free, and the
/// bundle's `proven` is a count of subtrees COMPLETE at a row at or below the
/// endpoint's own tip, so there is no honest reason for the newest one to be
/// missing. F6 pins the other edge (exactly `proven` is not short); this row
/// pins the first one below it. RED at the contract commit for F5's reason.
///
/// IT-10 as F5's: the prefix is accepted and written (observer), streamed
/// (receipt), nothing scanned.
#[tokio::test]
async fn a_serve_one_root_short_of_the_proven_count_is_not_up_to_date() {
    let full = full_sequence(ShieldedProtocol::Sapling);
    assert!(
        full.len() >= 2,
        "premise: there is a prefix one short of proven"
    );
    let prefix = &full[..full.len() - 1];

    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = anchored_wallet_and_key(dir.path(), &vault, HOST_A, BIRTHDAY_ANCHOR).await;
    let obs = observe(dir.path(), &key);

    let short = ProofEndpoint::serving(
        prefix,
        full_sequence(ShieldedProtocol::Orchard),
        IRONWOOD_TRUTHFUL,
    );
    let receipt = short.receipt();
    let served = short.served_heights();
    let p = pass_over(&w, short).await;
    record("PLANT one-short", &p, &receipt);

    assert_never_scanned(&receipt, "PLANT one-short");
    assert_eq!(
        receipt.streamed(ShieldedProtocol::Sapling),
        prefix.len(),
        "IT-10: streamed"
    );
    assert_eq!(
        recorded_heights(&obs, ShieldedProtocol::Sapling),
        prefix,
        "IT-10: the prefix was accepted and written"
    );
    assert!(
        !matches!(p.status, SyncStatus::EndpointBehind { .. }),
        "PLANT: not 'behind' at a tip above the bundle; got {}",
        describe(&p)
    );
    assert!(
        says_degraded(&p.status),
        "PLANT: {} of {} Sapling roots served is a strict prefix — one subtree short is \
         one subtree's worth of witnesses missing, and a rule with any slack hands an \
         endpoint that width for free. Got {}",
        prefix.len(),
        full.len(),
        describe(&p)
    );
    if let SyncStatus::UpToDateDegraded { pools, .. } = &p.status {
        assert!(
            !matches!(pools.sapling, PoolService::Served { .. }),
            "PLANT: the Sapling line is not a clean serve; got pools = {pools:?}"
        );
    }
    assert_carries_no_fixture_datum(&p.status, &served, HOST_A, "PLANT one-short");
    w.close().await.expect("close");
}

/// **PLANTED — DEFECT.** A short serve at a BEHIND tip is carried on the
/// behind variant — RE4's class ("both facts at once") for the short rule.
///
/// The endpoint reports [`BEHIND_TIP`] (below the bundle's newest row, so the
/// T0-1c-R surface is `EndpointBehind` whatever the pools say) and serves
/// Sapling ONE root where the rows AT OR BELOW that tip prove many more,
/// Orchard and Ironwood honest for the tip. The tip-bounding is the point:
/// `proven` is read against rows the endpoint's own tip admits, so this serve is
/// short by the endpoint's own account, and the T0-1c-R carrier exists so a
/// degraded pool is not dropped when the endpoint is also behind.
///
/// **PREDICTION:** the short comparison is wired where F5 observes it — the
/// `UpToDateDegraded` arm, or `degraded_pools`' `Some` gate — and the behind
/// arm's `pools` keeps the report T0-1b built (withheld-only), so after the
/// join F5 is green and this row is still red with `sapling: Served { 1 }` on
/// the behind variant. The rule has to live in the one place both arms read
/// (`bind_pool` / `pool_report`), or it is two rules. RED at the contract
/// commit: `EndpointBehind { pools: Some(.. sapling: Served { roots: 1 } ..) }`.
///
/// IT-10: nothing scans (birthday = tip + 1); the prefix is written; the
/// receipt shows every pool was asked, so `pools: None` is not an honest
/// answer here.
#[tokio::test]
async fn a_short_serve_at_a_behind_tip_is_carried_on_the_behind_variant() {
    let (newest, _) = newest_bundled_row(ShieldedProtocol::Ironwood);
    assert!(
        BEHIND_TIP < u64::from(newest),
        "premise: the tip is below the bundle's newest row ({newest})"
    );
    let sapling_for_tip = honest_sequence_at(ShieldedProtocol::Sapling, BEHIND_TIP);
    assert!(
        sapling_for_tip.len() > 1,
        "premise: the rows at or below {BEHIND_TIP} prove more than one Sapling subtree \
         complete (got {})",
        sapling_for_tip.len()
    );
    let prefix = &sapling_for_tip[..1];

    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = anchored_wallet_and_key(dir.path(), &vault, HOST_A, BEHIND_TIP).await;
    let obs = observe(dir.path(), &key);

    let short_and_behind = ProofEndpoint::serving(
        prefix,
        &honest_sequence_at(ShieldedProtocol::Orchard, BEHIND_TIP),
        IRONWOOD_HONEST_BELOW_BUNDLE,
    )
    .at_tip(BEHIND_TIP);
    let receipt = short_and_behind.receipt();
    let served = short_and_behind.served_heights();
    let p = pass_over(&w, short_and_behind).await;
    record("PLANT short-behind", &p, &receipt);

    assert_never_scanned(&receipt, "PLANT short-behind");
    for pool in [
        ShieldedProtocol::Sapling,
        ShieldedProtocol::Orchard,
        ShieldedProtocol::Ironwood,
    ] {
        assert!(
            receipt.opened(pool) >= 1,
            "IT-10: {} was asked, so the pass made a pool claim",
            pool.as_str_name()
        );
    }
    assert_eq!(
        recorded_heights(&obs, ShieldedProtocol::Sapling),
        prefix,
        "IT-10: the prefix was accepted and written"
    );
    match &p.status {
        SyncStatus::EndpointBehind {
            tip,
            newest_known,
            pools,
        } => {
            assert_eq!(u64::from(tip.value()), BEHIND_TIP, "at the endpoint's tip");
            assert_eq!(
                u64::from(newest_known.value()),
                u64::from(newest),
                "the newest bundled row is the reference"
            );
            let pools = pools.as_ref().unwrap_or_else(|| {
                panic!(
                    "PLANT: the production engine asked every pool ({}), so the behind \
                     variant must carry its report, not None",
                    where_it_stopped(&receipt)
                )
            });
            assert!(
                !matches!(pools.sapling, PoolService::Served { .. }),
                "PLANT: Sapling served {} root where the rows at or below the endpoint's own \
                 tip prove {} complete — a short serve — and the behind variant renders it \
                 as a clean serve. A short pool hidden by a behind tip is the RE4 hide one \
                 rule over. Got pools = {pools:?}",
                prefix.len(),
                sapling_for_tip.len()
            );
            assert!(
                matches!(pools.orchard, PoolService::Served { .. }),
                "PLANT: Orchard, honest for the tip, is a clean serve; got pools = {pools:?}"
            );
            assert!(
                matches!(pools.ironwood, PoolService::Served { .. }),
                "PLANT: Ironwood, honest for the tip, is a clean serve; got pools = {pools:?}"
            );
        }
        other => panic!(
            "PLANT: below the bundle's newest row the T0-1c-R surface is EndpointBehind \
             (with the pools report riding it); got {other:?} ({})",
            describe(&p)
        ),
    }
    assert_carries_no_fixture_datum(&p.status, &served, HOST_A, "PLANT short-behind");
    w.close().await.expect("close");
}

// ════════════════════════════════════════════════════════════════════════════
// T0-1c-R2 (§4n). Names are the contract's; a rename is a finding, not a
// liberty. DEFECT rows are RED at the contract commit for the reason §4n
// states; CONTROL rows are GREEN there and name the behaviour a careless repair
// would break. Every row runs `assert_carries_no_fixture_datum` (G10).
// ════════════════════════════════════════════════════════════════════════════

/// **G1 — DEFECT.** A tip pinned below the wallet's own scanned height is
/// behind.
///
/// The wallet is driven to [`TIP`] by an honest pass that really scans
/// ([`scanned_wallet_at_tip`]: ≥ 1 range asked, `scanned_tip() == TIP`, the
/// stamp at TIP). A second endpoint then reports `TIP − k` with
/// `k = `[`R2_PIN_BELOW`] — above one rewind step and above `REORG_MAX_BLOCKS`
/// (so no margin the implementer prices can make this pass for the wrong
/// reason), with `TIP − k` at or above the bundle's newest row (so the T0-1c
/// floor cannot be what fails it). Every premise is asserted from the compiled
/// constants and the bundle. The pinned endpoint is a
/// [`ProofEndpoint::withholding_at`] serve — the only one a scanned wallet on
/// this file's chain accepts (module doc, "the harness's limit"): every pool is
/// asked (receipt) and reads `Withheld`, degraded but complete, so
/// `record_synced` runs exactly as it does for an honest serve. The contract's
/// "every pool honest for that tip" is the harness's cost here, stated.
///
/// **What the row observes — by NAME, not by `says_degraded`** (a withheld
/// report satisfies that predicate for the wrong reason): the variant that
/// exists for "this endpoint is behind", `EndpointBehind` at the endpoint's own
/// tip; the durable stamp read back through `snapshot()` still TIP; the receipt
/// showing no block range below TIP was asked; `scanned_tip()` still TIP, so
/// the dip is the endpoint's and not a reorg's (IT-10 — the row is not passing
/// because a rewind happened).
///
/// **At the contract commit** (M1): the grade compares against the bundle row
/// only, `TIP − k` passes it as `AtOrAboveBundle`, upstream's `update_chain_tip`
/// returns early on `new_tip < max_scanned` (a clean 0-batch pass), the wallet
/// publishes the at-or-above terminal at `TIP − k` (`UpToDateDegraded` here;
/// plain `UpToDate` on a chain that could serve the roots) and `record_synced`
/// rewrites the stamp DOWN — while the wallet's own stronger oracle sat unread
/// in the same pass.
#[tokio::test]
async fn a_tip_pinned_below_the_wallets_own_scanned_height_is_behind() {
    let newest = newest_bundled_mainnet();
    let pinned = TIP - R2_PIN_BELOW;
    assert!(
        R2_PIN_BELOW > u64::from(REWIND_DISTANCE_BLOCKS)
            && R2_PIN_BELOW > u64::from(REORG_MAX_BLOCKS),
        "premise: k ({R2_PIN_BELOW}) is above one rewind step ({REWIND_DISTANCE_BLOCKS}) and \
         above REORG_MAX_BLOCKS ({REORG_MAX_BLOCKS}) — whichever margin the implementer \
         prices, the pin sits below it"
    );
    assert!(
        pinned >= newest,
        "premise: TIP − k ({pinned}) is at or above the newest bundled row ({newest}), so the \
         T0-1c floor alone cannot be what reds this row"
    );
    assert!(
        pinned > TIP - R2_SCAN_DEPTH,
        "premise: the pinned tip is above the wallet's anchor ({}), so upstream's range \
         builder is never handed an inverted range",
        TIP - R2_SCAN_DEPTH
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, _honest, served_honest) = scanned_wallet_at_tip(dir.path(), &vault, "G1").await;
    let before = w.snapshot().await.expect("snapshot");
    assert_eq!(
        before.last_synced.map(|s| height_of(s.height)),
        Some(TIP),
        "precondition: the honest pass stamped TIP"
    );
    assert!(
        before.ever_synced,
        "precondition: the honest pass set ever_synced"
    );

    let pinned_ep = ProofEndpoint::withholding_at(pinned);
    let receipt = pinned_ep.receipt();
    let served = pinned_ep.served_heights();
    let p = pass_over(&w, pinned_ep).await;
    record_r2("G1 pinned pass", &p, &receipt);
    let after = w.snapshot().await.expect("snapshot");
    let scanned_after = w.scanned_tip().await.expect("scanned_tip").map(height_of);
    println!(
        "[T0-1c-R2 G1] k = {R2_PIN_BELOW} (pinned tip {pinned}, newest bundled {newest}); stamp \
         before {:?} after {:?}; ever_synced after {}; scanned_tip after {scanned_after:?}; \
         ranges asked on the pinned pass {:?}",
        before.last_synced,
        after.last_synced,
        after.ever_synced,
        receipt.scanned_ranges()
    );

    for pool in [
        ShieldedProtocol::Sapling,
        ShieldedProtocol::Orchard,
        ShieldedProtocol::Ironwood,
    ] {
        assert!(
            receipt.opened(pool) >= 1,
            "IT-10: {} was asked on the pinned pass — the pass reached the roots and made a \
             pool claim, so the surface below is a completed pass's, not a stall's",
            pool.as_str_name()
        );
    }
    assert!(
        p.result.is_ok(),
        "IT-10: the pinned pass COMPLETED (a withheld serve is degraded, never refused), so \
         `record_synced` ran and the stamp clause below is about that write; got {}",
        describe(&p)
    );
    for (start, end) in receipt.scanned_ranges() {
        assert!(
            start > TIP,
            "G1: the pinned pass asked for a block range below the wallet's own height \
             ({start}..={end}) — a rewind the row did not ask for"
        );
    }
    assert_eq!(
        scanned_after,
        Some(TIP),
        "IT-10: the wallet's own scanned height is untouched by the pinned pass (no rewind), \
         so the dip is the endpoint's and not a reorg's"
    );
    assert!(
        matches!(p.status, SyncStatus::EndpointBehind { tip, .. } if height_of(tip) == pinned),
        "G1: an endpoint reporting a tip {R2_PIN_BELOW} blocks below the height this wallet has \
         itself scanned and validated is behind the chain by the wallet's own evidence — a \
         monotone height no endpoint can move — and the surface must say so: EndpointBehind \
         at the endpoint's own tip ({pinned}), the variant that exists for it. The bundle \
         row is a floor for a wallet that has never scanned; once it has, its own height is \
         the stronger oracle, read on every pass already (M1). Got {}; {}",
        describe(&p),
        where_it_stopped(&receipt)
    );
    assert_eq!(
        after.last_synced.map(|s| height_of(s.height)),
        Some(TIP),
        "G1: the durable stamp read back is still TIP — a behind pass must not rewrite the \
         wallet's \"as of block\" DOWN to a number only the endpoint supplied; got {:?} \
         (before the pinned pass: {:?})",
        after.last_synced,
        before.last_synced
    );
    assert!(
        after.ever_synced,
        "G1: ever_synced, set by the honest pass, is not un-set by a behind one"
    );
    assert_carries_no_fixture_datum(&p.status, &[served_honest, served].concat(), HOST_A, "G1");
    w.close().await.expect("close");
}

/// **G5 — CONTROL.** At or above the row AND the scanned height is not behind;
/// a rewind-sized dip is not behind either.
///
/// The same scanning wallet as G1; every post-scan endpoint is a
/// [`ProofEndpoint::withholding_at`] serve (module doc, "the harness's limit"),
/// so the healthy terminal reads `UpToDateDegraded` at the tip here — the
/// contract's "→ `UpToDate`" is asserted as "→ the at-or-above terminal"
/// ([`is_at_or_above_terminal_at`]), which excludes exactly what the row is
/// for: `EndpointBehind`. Clause (a): the endpoint at TIP again (at both
/// references) → the terminal at TIP; then at `TIP + 100` (above both, one
/// batch scanned) → the terminal at `TIP + 100`. Clause (b): the endpoint then
/// reports `REWIND_DISTANCE_BLOCKS` below the new scanned height — the size of
/// one rewind step, P10's "a REORG, not a behind server", and the contract's
/// "≤ the margin" at the smaller of the two margins §4n decision 1 offers — and
/// the surface is the terminal at that tip, not `EndpointBehind`; the wallet's
/// own height is not rewound. GREEN at the contract commit (no scanned-height
/// grade exists there) and it must stay green: it is the row a margin-less
/// grade (`tip < scanned`) reds — every honest endpoint one block behind our
/// own view of a shallow fork would be badged behind.
#[tokio::test]
async fn at_or_above_the_row_and_the_scanned_height_is_not_behind() {
    let newest = newest_bundled_mainnet();
    assert!(
        TIP > newest,
        "premise: TIP ({TIP}) is above the newest bundled row ({newest})"
    );
    let raised = TIP + R2_RAISE;
    let dip = raised - u64::from(REWIND_DISTANCE_BLOCKS);
    assert!(
        dip > TIP,
        "premise: the rewind-sized dip ({dip}) still sits above TIP, so it is above both \
         references by any reading and a rewind of exactly one step below the new height"
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, _honest, served_honest) = scanned_wallet_at_tip(dir.path(), &vault, "G5").await;

    // (a) at both references: the endpoint at TIP again.
    let same = ProofEndpoint::withholding_at(TIP);
    let r_same = same.receipt();
    let at_both = pass_over(&w, same).await;
    record_r2("G5 at both references", &at_both, &r_same);
    assert!(
        !matches!(at_both.status, SyncStatus::EndpointBehind { .. }),
        "G5: an endpoint whose tip equals the wallet's own scanned height and sits above the \
         row is not behind; got {}",
        describe(&at_both)
    );
    assert!(
        is_at_or_above_terminal_at(&at_both.status, TIP),
        "G5: at both references the pass reaches the at-or-above terminal at TIP; got {}",
        describe(&at_both)
    );

    // (a) above both references: the chain advanced by one batch.
    let ahead = ProofEndpoint::withholding_at(raised);
    let r_ahead = ahead.receipt();
    let served_ahead = ahead.served_heights();
    let above = pass_over(&w, ahead).await;
    record_r2("G5 above both references", &above, &r_ahead);
    for (start, end) in r_ahead.scanned_ranges() {
        assert!(
            start > TIP && end <= raised,
            "G5 geometry: the pass above both references scans only the new blocks; got \
             {start}..={end}"
        );
    }
    assert!(
        is_at_or_above_terminal_at(&above.status, raised),
        "G5: an endpoint above both references reaches the at-or-above terminal at its tip \
         ({raised}); got {}",
        describe(&above)
    );
    assert_eq!(
        w.scanned_tip().await.expect("scanned_tip").map(height_of),
        Some(raised),
        "precondition: the wallet followed the chain to {raised}"
    );

    // (b) the rewind-sized dip.
    let dipped = ProofEndpoint::withholding_at(dip);
    let r_dip = dipped.receipt();
    let served_dip = dipped.served_heights();
    let after_dip = pass_over(&w, dipped).await;
    record_r2("G5 rewind-sized dip", &after_dip, &r_dip);
    println!(
        "[T0-1c-R2 G5] dip = {} below the scanned height {raised} (one rewind step); \
         scanned_tip after the dip {:?}",
        u64::from(REWIND_DISTANCE_BLOCKS),
        w.scanned_tip().await.expect("scanned_tip").map(height_of)
    );
    for pool in [
        ShieldedProtocol::Sapling,
        ShieldedProtocol::Orchard,
        ShieldedProtocol::Ironwood,
    ] {
        assert!(
            r_dip.opened(pool) >= 1,
            "IT-10: {} asked on the dip pass — a completed pass, not a stall",
            pool.as_str_name()
        );
    }
    assert!(
        !matches!(after_dip.status, SyncStatus::EndpointBehind { .. }),
        "G5: a tip one rewind step below the wallet's own height is a reorg's legitimate \
         downward move, not a behind server (§4n P10) — the floor must not fire on it; got {}",
        describe(&after_dip)
    );
    assert!(
        is_at_or_above_terminal_at(&after_dip.status, dip),
        "G5: the dip pass reaches the at-or-above terminal at the endpoint's tip ({dip}); got {}",
        describe(&after_dip)
    );
    assert_carries_no_fixture_datum(&at_both.status, &served_honest, HOST_A, "G5 (a)");
    assert_carries_no_fixture_datum(&above.status, &served_ahead, HOST_A, "G5 (a, above)");
    assert_carries_no_fixture_datum(&after_dip.status, &served_dip, HOST_A, "G5 (b)");
    w.close().await.expect("close");
}

/// **G2 — DEFECT.** A behind pass does not stamp the wallet as ever synced.
///
/// E1's wallet and E1's endpoint (below the bundle's newest row, honest for
/// its tip): the FIRST completed pass of a fresh wallet is `EndpointBehind`
/// (by name — the variant exists at the contract commit; asserted as the
/// precondition, with `result.is_ok()` so a stall cannot pass this row by
/// writing nothing). Then `close()`, `open()`, and `snapshot()` BEFORE any
/// pass: either the durable pair is clear — `ever_synced` false and
/// `last_synced` absent — or the stamp carries a standing the render can
/// qualify, asserted by distinguishability from the contract commit's
/// unqualified stamp ([`relaunch_reads_qualified`]; §4n decision 2 is the
/// implementer's, the adjudicator refines to the field).
///
/// **At the contract commit** (M2): `emit_synced` calls `record_synced(tip)`
/// BEFORE it ranks the claims, so the behind pass durably sets `ever_synced`
/// (write-once) and stamps the behind height; after the relaunch the snapshot
/// reads `ever_synced: true`, `last_synced: Some({ height: 3455000, .. })`,
/// `sync: Idle` — a host renders "Balance as of block 3,455,000" with no
/// behind qualification and suppresses the first-run catch-up framing for
/// the wallet's life. "Held nowhere durable" is false for this variant.
#[tokio::test]
async fn a_behind_pass_does_not_stamp_the_wallet_as_ever_synced() {
    let newest = newest_bundled_mainnet();
    assert!(
        BEHIND_TIP < newest,
        "premise: the endpoint's tip ({BEHIND_TIP}) is below the newest bundled row ({newest})"
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_anchored_at(dir.path(), &vault, HOST_A, BEHIND_TIP).await;
    let fresh = w.snapshot().await.expect("snapshot");
    assert!(
        !fresh.ever_synced && fresh.last_synced.is_none(),
        "precondition: a fresh wallet has never synced (ever_synced {}, last_synced {:?})",
        fresh.ever_synced,
        fresh.last_synced
    );

    let behind = ProofEndpoint::honest_at(BEHIND_TIP, IRONWOOD_HONEST_BELOW_BUNDLE);
    let receipt = behind.receipt();
    let served = behind.served_heights();
    let p = pass_over(&w, behind).await;
    record_r2("G2 behind pass", &p, &receipt);
    assert_never_scanned(&receipt, "G2");
    assert!(
        p.result.is_ok(),
        "precondition: the behind pass COMPLETED — a stalled pass writes no stamp and would \
         prove nothing here; got {}",
        describe(&p)
    );
    assert!(
        matches!(p.status, SyncStatus::EndpointBehind { tip, .. } if height_of(tip) == BEHIND_TIP),
        "precondition (E1, by name): the first completed pass says EndpointBehind at \
         {BEHIND_TIP}; got {}",
        describe(&p)
    );
    w.close().await.expect("close");

    // The relaunch: the same wallet, reopened; nothing has run yet.
    let w = Wallet::open_with_vault(cfg(dir.path(), HOST_A), Arc::clone(&vault))
        .await
        .expect("relaunch");
    let snap = w.snapshot().await.expect("snapshot");
    let cleared = !snap.ever_synced && snap.last_synced.is_none();
    let qualified = relaunch_reads_qualified(&snap);
    println!(
        "[T0-1c-R2 G2] after relaunch, before any pass: ever_synced {}, last_synced {:?}, \
         sync {:?} — cleared {cleared}, qualified-by-shape {qualified}",
        snap.ever_synced, snap.last_synced, snap.sync
    );
    assert!(
        cleared || qualified,
        "G2: a wallet whose only completed pass was against a BEHIND server relaunches \
         reading ever_synced {} and last_synced {:?} with nothing to qualify them — a host \
         renders \"Balance as of block {BEHIND_TIP}\" as if that were the chain's height and \
         suppresses the first-run catch-up framing for the wallet's life. Either the durable \
         pair stays clear until a pass at or above the row, or the stamp carries a standing \
         the render can qualify (§4n decision 2). Whole snapshot: {snap:?}",
        snap.ever_synced,
        snap.last_synced
    );
    assert_carries_no_fixture_datum(&p.status, &served, HOST_A, "G2");
    w.close().await.expect("close");
}

/// **G3 — DEFECT.** A lying-low provisioning tip cannot floor the birthday
/// below the bundle row.
///
/// The wallet is the stamped, account-less remnant
/// ([`stamped_account_less_remnant`] — the one shape on which the
/// `min(tip − lag, estimate(created_at))` arm runs; module doc). The endpoint
/// reports a tip derived from the treestate table itself: the newest bundled
/// row at least [`R2_FAR_BELOW`] blocks below the newest, plus the lag, plus
/// one — so at the contract commit the birthday resolves to exactly one row
/// below the lie (asserted as the observed value in the message) and the scan
/// the lie buys is the smallest the geometry allows; Sapling and Orchard are
/// served honestly for that tip, Ironwood is honestly empty (pre-activation).
///
/// **Two dispositions, one row** (§4n decision 3; RE-AIMED at T0-1c-R3, §4n-R
/// R3-3, by this row's author). REFUSE: no account is provisioned, the surface
/// is a typed stall that is neither `EndpointUnreachable` nor `Internal`, and
/// the receipt shows nothing was asked past the tip. CLAMP: the account's
/// birthday read back through `birthday_height()` EQUALS the birthday a
/// CURRENT server at the row gives — a CONTROL remnant of the same shape,
/// driven first through the same seam against `tip = newest bundled row` with
/// the same fixture, its birthday read back the same way — and is never at (or
/// one above) activation. The number this clause first asked for, `≥ row −
/// lag`, was unbuildable on either network: `bundled_treestate`'s anchor
/// ceiling flattens every request above the newest activation this build
/// knows onto one row (the T0-1c-R2 ruling, VERDICT row 2 (b)) — so the row
/// now asks the mechanism's own question: does a lying-low tip provision the
/// SAME birthday a truthful one does? **A clamped birthday sits ABOVE this
/// endpoint's tip**, so the clamp arm also asserts that the pass CONTINUES — a
/// typed status that is one of §4n-R Q-R5's two cells ([`q_r5_cell`]:
/// `EndpointBehind` below the row, or `Stalled { BirthdayInFuture }`), never a
/// healthy terminal, never a transient, never `EndpointUnreachable` or
/// `Internal`; the row prints which — and that a SECOND pass over the same liar
/// runs and publishes: the loop is alive. At the T0-1c-R2 join the clamp arm
/// handed upstream's `update_chain_tip` the inverted range `3427311..2957602`
/// and the pass PANICKED (`IMPL_WRONG`, M3); a panic here is a finding about
/// the arm, not a harness artifact.
///
/// **At the contract commit** (M3): `resolve_birthday` reads `tip_height` —
/// the third endpoint height a pass reads and the only one that writes durable
/// state before the grade — takes `tip − lag`, anchors on the bundled row
/// below it and imports the account THERE; the same pass's `fetch_tip` then
/// grades the tip behind, but the import is permanent and survives switching
/// servers.
#[tokio::test]
async fn a_lying_low_provisioning_tip_cannot_floor_the_birthday_below_the_bundle_row() {
    let newest = newest_bundled_mainnet();
    let lag = u64::from(NEW_WALLET_BIRTHDAY_LAG_BLOCKS);
    let activation = sapling_activation_from_params();
    let far_row = crate::checkpoints::treestate_table(Network::Main)
        .iter()
        .map(|row| u64::from(row.0))
        .filter(|h| h + R2_FAR_BELOW <= newest)
        .max()
        .expect("a bundled row at least R2_FAR_BELOW blocks below the newest");
    let lying_tip = far_row + lag + 1;
    assert!(
        lying_tip + R2_FAR_BELOW - lag - 1 <= newest,
        "premise: the lying tip ({lying_tip}) is far below the newest bundled row ({newest})"
    );
    assert!(
        lying_tip > activation + lag,
        "premise: the lying tip is well above activation ({activation}), so a birthday at \
         activation would be the arm's floor and not the tip's arithmetic"
    );

    let vault = test_vault();

    // The CONTROL (§4n-R R3-3, re-aimed): the same remnant shape against a
    // CURRENT server exactly at the row — the birthday a truthful tip
    // provisions, read back the same way. Driven FIRST, so its record line is
    // in the artifact even when the liar's pass panics (the red-first shape on
    // the T0-1c-R2 join). It scans the gap the anchor ceiling leaves below the
    // row over [`fixture_block`]s (E6's shape); the roots are bound BEFORE that
    // scan, so the post-scan harness limit (module doc) is not in play.
    let dir_control = tempfile::tempdir().expect("tempdir");
    let w_control = stamped_account_less_remnant(dir_control.path(), &vault, HOST_A).await;
    let current = ProofEndpoint::honest_at(newest, IRONWOOD_HONEST_BELOW_BUNDLE);
    let receipt_control = current.receipt();
    let control = pass_over(&w_control, current).await;
    record_r2("G3 control at the row", &control, &receipt_control);
    let control_birthday = w_control
        .birthday_height()
        .await
        .expect("birthday_height")
        .map(height_of);
    println!(
        "[T0-1c-R2 G3] control: current tip {newest} (the row); birthday read back \
         {control_birthday:?}; ranges asked {}",
        receipt_control.scanned_ranges().len()
    );
    let control_birthday = control_birthday.expect(
        "control: a current server at the row provisions the remnant, so a birthday exists",
    );
    assert!(
        control_birthday > activation + 1,
        "control: a current server at the row never provisions at activation ({activation}); \
         got {control_birthday}"
    );
    assert!(
        is_up_to_date_at(&control.status, newest),
        "control: the remnant against a current server at the row completes its pass \
         UpToDate at the row — the reference the clamp is measured against is a HEALTHY \
         pass; got {}",
        describe(&control)
    );
    w_control.close().await.expect("close");

    let dir = tempfile::tempdir().expect("tempdir");
    let w = stamped_account_less_remnant(dir.path(), &vault, HOST_A).await;
    assert!(
        w.birthday_height()
            .await
            .expect("birthday_height")
            .is_none(),
        "precondition: no account, so no birthday yet"
    );

    let liar = ProofEndpoint::honest_at(lying_tip, &[]);
    let receipt = liar.receipt();
    let served = liar.served_heights();
    let p = pass_over(&w, liar).await;
    record_r2("G3", &p, &receipt);
    let provisioned = w.account_exists().await.expect("account_exists");
    let birthday = w
        .birthday_height()
        .await
        .expect("birthday_height")
        .map(height_of);
    println!(
        "[T0-1c-R2 G3] lying tip {lying_tip} (newest bundled {newest}, control birthday \
         {control_birthday}); provisioned {provisioned}; birthday read back {birthday:?}; \
         ranges asked {:?}",
        receipt.scanned_ranges()
    );

    match birthday {
        None => {
            assert!(
                !provisioned,
                "IT-10: no birthday means no account — the refusal preceded the import"
            );
            assert!(
                matches!(p.status, SyncStatus::Stalled { .. }),
                "G3 (refuse): a refused provisioning is a typed stall; got {}",
                describe(&p)
            );
            assert_ne!(
                stall_reason(&p.status),
                Some(StallReason::EndpointUnreachable),
                "G3 (refuse): the link answered — 'check your connection' is the wrong next \
                 step; got {}",
                describe(&p)
            );
            assert_ne!(
                stall_reason(&p.status),
                Some(StallReason::Internal),
                "G3 (refuse): not a local fault — 'restore from seed' is the wrong next step; \
                 got {}",
                describe(&p)
            );
            assert!(
                receipt.scanned_ranges().is_empty(),
                "G3 (refuse): nothing was scanned — the refusal precedes the import and the pass; \
                 got {:?}",
                receipt.scanned_ranges()
            );
        }
        Some(b) => {
            assert!(
                b > activation + 1,
                "G3: never a birthday at activation ({activation}); got {b} — the lying tip \
                 bought a full-chain scan"
            );
            assert_eq!(
                b,
                control_birthday,
                "G3 (re-aimed, §4n-R R3-3): the account's birthday read back ({b}) is not the \
                 birthday a CURRENT server at the row gives the same remnant ({control_birthday}): \
                 a first tip the grade never sees moved a PERMANENT write by {} blocks — a scan \
                 the wallet cannot undo by switching servers (M3; the T0-1c-R2 contract \
                 commit's `resolve_birthday` took tip − lag from an ungraded number). Got {}; {}",
                control_birthday.abs_diff(b),
                describe(&p),
                where_it_stopped(&receipt)
            );
            // The clamp arm CONTINUES the pass (§4n-R R3-3): a birthday clamped
            // toward the row sits above this tip by construction, and at the
            // T0-1c-R2 join the pass handed upstream `3427311..2957602` and
            // panicked. A typed status here — one of Q-R5's two cells, never a
            // healthy terminal (the tip is E1's geometry, far below the row),
            // never a transient, never a specific lie — is what "continues" means.
            assert_ne!(
                stall_reason(&p.status),
                Some(StallReason::EndpointUnreachable),
                "G3 (clamp): the link answered — 'check your connection' is the wrong next \
                 step; got {}",
                describe(&p)
            );
            assert_ne!(
                stall_reason(&p.status),
                Some(StallReason::Internal),
                "G3 (clamp): not a local fault — 'restore from seed' is the wrong next step; \
                 got {}",
                describe(&p)
            );
            let cell = q_r5_cell(&p.status).unwrap_or_else(|| {
                panic!(
                    "G3 (clamp): the pass continues past the import to a typed status — \
                     `EndpointBehind` below the row, or `Stalled {{ BirthdayInFuture }}` — \
                     never a healthy terminal over a tip {} blocks below the row; got {}; {}",
                    newest - lying_tip,
                    describe(&p),
                    where_it_stopped(&receipt)
                )
            });
            println!("[T0-1c-R2 G3] clamp arm: pass 1 landed in the cell: {cell}");
            // The loop is alive: a second pass over the same liar runs and publishes.
            let liar_again = ProofEndpoint::honest_at(lying_tip, &[]);
            let receipt_again = liar_again.receipt();
            let again = pass_over(&w, liar_again).await;
            record_r2("G3 clamp arm, pass 2", &again, &receipt_again);
            let cell_again = q_r5_cell(&again.status).unwrap_or_else(|| {
                panic!(
                    "G3 (clamp): the SECOND pass over the same liar publishes a typed cell too \
                     — the loop is alive; got {}; {}",
                    describe(&again),
                    where_it_stopped(&receipt_again)
                )
            });
            println!("[T0-1c-R2 G3] clamp arm: pass 2 landed in the cell: {cell_again}");
            assert_eq!(
                w.birthday_height()
                    .await
                    .expect("birthday_height")
                    .map(height_of),
                Some(b),
                "G3 (clamp): the second pass did not move the durable birthday"
            );
        }
    }
    assert_carries_no_fixture_datum(&p.status, &served, HOST_A, "G3");
    w.close().await.expect("close");
}

/// **G4 — DEFECT.** A behind identity height does not lower the signing
/// anchor.
///
/// The scanning wallet (its honest pass judges at `scanned = 0`, so the
/// clamp holds that first anchor at `REORG_MAX_BLOCKS`); pass 1, at TIP again,
/// judges at `scanned = TIP` and records `capable_tip = TIP` (asserted through
/// `consensus_status()`, the crate's cold read of the stamp). Every post-scan
/// endpoint is a [`ProofEndpoint::withholding_at`] serve (module doc, "the
/// harness's limit") — `evaluate_consensus`, where the anchor is written, runs
/// before the roots are asked, so the pools' withheld report is beside the
/// point and the pass still completes to the at-or-above terminal. Pass 2: the
/// same endpoint whose IDENTITY claims `TIP −`[`R2_IDENTITY_BEHIND`] with the
/// correct branch for that height — the tip ports still say TIP, the pass
/// completes at TIP and the verdict is `Current` (asserted: the verdict was
/// CAPABLE, so the anchor was eligible to move, and an unchanged anchor is
/// monotonicity rather than a refused verdict). `capable_tip` read back is
/// still TIP and the grace measured at TIP is 0.
/// Pass 3, the money consequence: an endpoint that OMITS its branch id at TIP
/// is judged `Unknown` inside the §6.3 grace and `permits_signing()` — the gate
/// every signature passes through. Pass 4, the control: an honest endpoint at
/// `TIP + 100` RAISES `capable_tip` to it — monotone means "never lowered by a
/// claim", not "frozen".
///
/// Every pass runs and prints its record before any assertion, so the
/// red-first artifact carries the whole chain (the anchor after pass 2, the
/// gate after pass 3) and not only the first red clause.
///
/// **At the contract commit** (M4): `evaluate_consensus` builds
/// `anchor_tip = min(claimed, scanned + REORG_MAX_BLOCKS)` from the identity's
/// height, and `consensus_stamp::record` overwrites `capable_tip` on any
/// capable verdict — so pass 2 drops it to `TIP − 20,000`, and pass 3 computes
/// a grace of 20,000 > 1,152 and refuses to sign: "update the app", for a
/// server that once reported a low height. Decision 5's sentence enumerates
/// only the verdict.
#[tokio::test]
async fn a_behind_identity_height_does_not_lower_the_signing_anchor() {
    let behind_claim = TIP - R2_IDENTITY_BEHIND;
    let raised = TIP + R2_RAISE;
    assert!(
        R2_IDENTITY_BEHIND > u64::from(REORG_MAX_BLOCKS),
        "premise: the identity's under-claim ({R2_IDENTITY_BEHIND}) exceeds REORG_MAX_BLOCKS \
         ({REORG_MAX_BLOCKS}), so the S253 clamp takes the CLAIM and the anchor can move"
    );
    assert!(
        behind_claim > newest_bundled_mainnet() - R2_IDENTITY_BEHIND,
        "premise: the claim sits within the bundle's era (no activation the params know lies \
         between it and TIP is asserted by the Current verdict below)"
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, _honest, served_honest) = scanned_wallet_at_tip(dir.path(), &vault, "G4").await;
    let s0 = w.consensus_status().await.expect("consensus_status");

    // Pass 1: at TIP again on a wallet scanned to TIP → capable_tip = TIP.
    let again = ProofEndpoint::withholding_at(TIP);
    let r1 = again.receipt();
    let p1 = pass_over(&w, again).await;
    record_r2("G4 pass 1 (TIP again)", &p1, &r1);
    let s1 = w.consensus_status().await.expect("consensus_status");

    // Pass 2: the identity under-claims; the tip ports still say TIP.
    let liar = ProofEndpoint::withholding_at(TIP).with_identity_height(behind_claim);
    let r2 = liar.receipt();
    let served2 = liar.served_heights();
    let p2 = pass_over(&w, liar).await;
    record_r2("G4 pass 2 (identity below the scanned height)", &p2, &r2);
    let s2 = w.consensus_status().await.expect("consensus_status");

    // Pass 3: an endpoint that does not say its branch, at TIP.
    let mute = ProofEndpoint::withholding_at(TIP).omitting_branch_id();
    let r3 = mute.receipt();
    let served3 = mute.served_heights();
    let p3 = pass_over(&w, mute).await;
    record_r2("G4 pass 3 (branch omitted)", &p3, &r3);
    let s3 = w.consensus_status().await.expect("consensus_status");

    // Pass 4: the control — the chain advanced, the endpoint follows.
    let ahead = ProofEndpoint::withholding_at(raised);
    let r4 = ahead.receipt();
    let served4 = ahead.served_heights();
    let p4 = pass_over(&w, ahead).await;
    record_r2("G4 control (TIP + 100)", &p4, &r4);
    let s4 = w.consensus_status().await.expect("consensus_status");

    println!(
        "[T0-1c-R2 G4] capable_tip: after the honest scan {:?}; after pass 1 {:?}; after pass 2 \
         (identity {behind_claim}) {:?} verdict {:?}; after pass 3 (branch omitted) {:?} \
         verdict {:?} permits_signing {:?}; after the control at {raised} {:?}",
        s0.grace_anchor_tip,
        s1.grace_anchor_tip,
        s2.grace_anchor_tip,
        s2.verdict,
        s3.grace_anchor_tip,
        s3.verdict,
        s3.verdict.map(|v| v.permits_signing()),
        s4.grace_anchor_tip
    );

    assert!(
        is_at_or_above_terminal_at(&p1.status, TIP),
        "precondition: pass 1 completes to the at-or-above terminal at TIP; got {}",
        describe(&p1)
    );
    assert_eq!(
        s1.grace_anchor_tip.map(height_of),
        Some(TIP),
        "precondition: after a capable pass on a wallet scanned to TIP, capable_tip is TIP \
         (min(claimed, scanned + REORG_MAX_BLOCKS)); got {s1:?}"
    );
    assert_eq!(
        s1.blocks_since_last_current(block(TIP)),
        Some(0),
        "precondition: the grace measured at TIP is 0 after pass 1"
    );
    for pool in [
        ShieldedProtocol::Sapling,
        ShieldedProtocol::Orchard,
        ShieldedProtocol::Ironwood,
    ] {
        assert!(
            r2.opened(pool) >= 1,
            "IT-10: {} asked on pass 2 — the pass ran past the consensus evaluation into the \
             roots, so the anchor read below was written by this pass's evaluation",
            pool.as_str_name()
        );
    }
    assert!(
        is_at_or_above_terminal_at(&p2.status, TIP),
        "IT-10: pass 2 itself completes at TIP (the tip ports say TIP) — the identity height \
         is the only thing that moved; got {}",
        describe(&p2)
    );
    // The verdict is ASKED, never matched (`extraction_policy::
    // consensus_compatibility_is_the_only_staleness_predicate`): capable means
    // `is_signing_capable_check()`, the enum's own word for "this verdict may
    // refresh the anchor".
    assert!(
        s2.verdict.is_some_and(|v| v.is_signing_capable_check()),
        "IT-10: the under-claiming identity with the correct branch earns a CAPABLE verdict \
         (scanned TIP ≥ claimed {behind_claim}), so the anchor was ELIGIBLE to move on this \
         pass — an unchanged anchor below is monotonicity, not a refused verdict; got {:?}",
        s2.verdict
    );
    assert_eq!(
        s2.grace_anchor_tip.map(height_of),
        Some(TIP),
        "G4: capable_tip read back through the stamp is unchanged at TIP after a pass whose \
         identity claimed {behind_claim} — an UNTRUSTED number must not move a durable \
         money-path gate downward (M4); got {:?}",
        s2.grace_anchor_tip
    );
    assert_eq!(
        s2.blocks_since_last_current(block(TIP)),
        Some(0),
        "G4: the grace measured at TIP is unchanged (0) after the behind identity; got {:?}",
        s2.blocks_since_last_current(block(TIP))
    );
    // Unknown, asked of the type: neither a capable check nor a verdict that
    // blocks interpretation — the one remaining arm.
    let v3 = s3.verdict.unwrap_or_else(|| {
        panic!(
            "IT-10: pass 3 recorded a verdict; got none ({})",
            describe(&p3)
        )
    });
    assert!(
        !v3.is_signing_capable_check() && !v3.blocks_interpretation(),
        "IT-10: an identity without a branch id is judged Unknown (neither capable nor \
         blocking); got {v3:?} ({})",
        describe(&p3)
    );
    assert!(
        v3.permits_signing(),
        "G4 (the gate): an endpoint that omits its branch id at TIP is judged Unknown INSIDE \
         the §6.3 grace when the anchor is TIP, and signing is permitted; got {v3:?} with \
         anchor {:?} — \"update the app\" for a server that once reported a low height is \
         the incident's shape",
        s3.grace_anchor_tip
    );
    assert_eq!(
        s3.grace_anchor_tip.map(height_of),
        Some(TIP),
        "G4: an Unknown rides the previous anchor and does not renew it; got {:?}",
        s3.grace_anchor_tip
    );
    assert!(
        is_at_or_above_terminal_at(&p4.status, raised),
        "control: the chain advanced and the pass followed to {raised}; got {}",
        describe(&p4)
    );
    assert!(
        s4.verdict.is_some_and(|v| v.is_signing_capable_check()),
        "control: the pass at {raised} is a capable verdict; got {:?}",
        s4.verdict
    );
    assert_eq!(
        s4.grace_anchor_tip.map(height_of),
        Some(raised),
        "G4 control: a capable pass at TIP + {R2_RAISE} RAISES capable_tip to it — monotone \
         means never lowered by a claim, not frozen; got {:?}",
        s4.grace_anchor_tip
    );
    assert_carries_no_fixture_datum(
        &p2.status,
        &[served_honest, served2].concat(),
        HOST_A,
        "G4 pass 2",
    );
    assert_carries_no_fixture_datum(&p3.status, &served3, HOST_A, "G4 pass 3");
    assert_carries_no_fixture_datum(&p4.status, &served4, HOST_A, "G4 control");
    w.close().await.expect("close");
}

/// **G6 — DEFECT.** A configured birthday above a stuck validator's tip is not
/// a dead link.
///
/// A restore-shaped create with `config.birthday` ONE above the bundle's
/// tree-state tail — above the tail, so the create stays lazy and the first
/// pass provisions (asserted: no account). The endpoint is E1's stuck validator
/// (tip [`BEHIND_TIP`], below the row and below the configured birthday):
/// `resolve_birthday`'s clamp returns `BirthdayInFuture` (asserted through the
/// pass result — the IT-10 receipt that the pass stopped at the clamp and not
/// elsewhere; nothing provisioned, no root stream opened, nothing scanned).
/// The published status is a stall that is neither `EndpointUnreachable` nor
/// `Internal` (the floor's sentence; the reason is the implementer's — §4n
/// scope: "its own reading").
///
/// **At the contract commit** (crypto #13, §4k-run owed 2): `stall_for`'s
/// `_ =>` arm renders `BirthdayInFuture` as `EndpointUnreachable` — "check your
/// connection", for a link that answered — and because provisioning runs
/// before `sync_once`, the behind grade never runs: on exactly T0-1c's
/// population the dead-link sentence PREEMPTS the behind report, every pass.
#[tokio::test]
async fn a_configured_birthday_above_a_stuck_validators_tip_is_not_a_dead_link() {
    let tail = crate::checkpoints::treestate_table(Network::Main)
        .last()
        .map(|row| u64::from(row.0))
        .expect("the mainnet bundle has a tree-state tail");
    assert_eq!(
        tail,
        newest_bundled_mainnet(),
        "premise: the tree-state tail IS the newest bundled row the grade reads"
    );
    let configured = tail + 1;
    assert!(
        BEHIND_TIP < configured,
        "premise: the stuck validator's tip ({BEHIND_TIP}) is below the configured birthday \
         ({configured})"
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let mut config = cfg(dir.path(), HOST_A);
    config.birthday = Some(block(configured));
    let w = Wallet::create_with_vault(config, raw_seed(), Arc::clone(&vault))
        .await
        .expect("create");
    assert!(
        !w.account_exists().await.expect("account_exists"),
        "precondition: a configured birthday above the tail keeps the create lazy — no account \
         until the first pass provisions"
    );

    let stuck = ProofEndpoint::honest_at(BEHIND_TIP, IRONWOOD_HONEST_BELOW_BUNDLE);
    let receipt = stuck.receipt();
    let served = stuck.served_heights();
    let p = pass_over(&w, stuck).await;
    record_r2("G6", &p, &receipt);

    assert!(
        !w.account_exists().await.expect("account_exists"),
        "IT-10: nothing was provisioned — the pass stopped at the birthday clamp"
    );
    for pool in [
        ShieldedProtocol::Sapling,
        ShieldedProtocol::Orchard,
        ShieldedProtocol::Ironwood,
    ] {
        assert_eq!(
            receipt.opened(pool),
            0,
            "IT-10: provisioning stops before any root stream ({})",
            pool.as_str_name()
        );
    }
    assert_never_scanned(&receipt, "G6");
    assert!(
        matches!(&p.result, Err(WalletError::BirthdayInFuture)),
        "IT-10: the pass stopped at the birthday clamp (BirthdayInFuture) and nowhere else; \
         got {}",
        describe(&p)
    );
    assert!(
        matches!(p.status, SyncStatus::Stalled { .. }),
        "G6: fatal to the pass — a typed stall; got {}",
        describe(&p)
    );
    assert_ne!(
        stall_reason(&p.status),
        Some(StallReason::EndpointUnreachable),
        "G6: the link answered and the server is behind the birthday this wallet was told to \
         start from — rendering that as EndpointUnreachable tells the user to check a \
         connection that works, on every pass, and pre-empts the behind report on exactly \
         T0-1c's population; got {}",
        describe(&p)
    );
    assert_ne!(
        stall_reason(&p.status),
        Some(StallReason::Internal),
        "G6: not a local fault either — 'restore from seed' is the wrong next step; got {}",
        describe(&p)
    );
    assert_carries_no_fixture_datum(&p.status, &served, HOST_A, "G6");
    w.close().await.expect("close");
}

// ════════════════════════════════════════════════════════════════════════════
// T0-1c-R2 PLANTED cases (IT-1 +A) — the cells the floor does not name.
// ════════════════════════════════════════════════════════════════════════════

/// **PLANTED — DEFECT.** A behind pass does not rewrite the stamp of a wallet
/// that HAS reached tip.
///
/// G2 names the fresh wallet ("does not stamp the wallet as ever synced"); this
/// is the wallet that synced honestly to TIP first (the scanning wallet: stamp
/// TIP, `ever_synced` true), then meets E1's stuck validator — a tip below the
/// bundle row AND below its own scanned height. After `close()`/`open()` the
/// stamp still reads TIP, or the snapshot is qualified by shape
/// ([`relaunch_reads_qualified`]).
///
/// **PREDICTION:** the gate is written around G2's name — `record_synced`
/// skipped while `ever_synced` is unset, or the whole write gated on "the first
/// pass" — so a fresh wallet is protected, G2 goes green, and a wallet that
/// has already reached tip still has its "as of block" rewritten DOWN to the
/// behind height on every behind pass and renders it unqualified after a
/// relaunch. §4n decision 2's second option ("gate only `ever_synced`") is
/// exactly this miss with a name; the row is red under it, as G2 is. RED at
/// the contract commit: the stamp reads [`BEHIND_TIP`] after the relaunch.
#[tokio::test]
async fn a_behind_pass_does_not_rewrite_the_stamp_of_a_wallet_that_has_reached_tip() {
    let newest = newest_bundled_mainnet();
    assert!(
        BEHIND_TIP < newest && BEHIND_TIP < TIP - R2_SCAN_DEPTH,
        "premise: the stuck validator's tip ({BEHIND_TIP}) is below the row ({newest}) and \
         below the scanning wallet's anchor ({})",
        TIP - R2_SCAN_DEPTH
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, _honest, served_honest) =
        scanned_wallet_at_tip(dir.path(), &vault, "PLANT relaunch").await;
    let before = w.snapshot().await.expect("snapshot");
    assert_eq!(
        before.last_synced.map(|s| height_of(s.height)),
        Some(TIP),
        "precondition: the honest pass stamped TIP"
    );
    assert!(before.ever_synced, "precondition: ever_synced is set");

    // A withholding serve (module doc, "the harness's limit"): behind, complete.
    let behind = ProofEndpoint::withholding_at(BEHIND_TIP);
    let receipt = behind.receipt();
    let served = behind.served_heights();
    let p = pass_over(&w, behind).await;
    record_r2("PLANT relaunch: behind pass", &p, &receipt);
    for (start, end) in receipt.scanned_ranges() {
        assert!(
            start > TIP,
            "PLANT: the behind pass asked for a range below the wallet's own height \
             ({start}..={end})"
        );
    }
    assert_eq!(
        w.scanned_tip().await.expect("scanned_tip").map(height_of),
        Some(TIP),
        "IT-10: the wallet's own height is untouched — no rewind"
    );
    assert!(
        p.result.is_ok(),
        "precondition: the behind pass completed; got {}",
        describe(&p)
    );
    assert!(
        matches!(p.status, SyncStatus::EndpointBehind { tip, .. } if height_of(tip) == BEHIND_TIP),
        "precondition (by name): EndpointBehind at {BEHIND_TIP}; got {}",
        describe(&p)
    );
    w.close().await.expect("close");

    let w = Wallet::open_with_vault(cfg(dir.path(), HOST_A), Arc::clone(&vault))
        .await
        .expect("relaunch");
    let snap = w.snapshot().await.expect("snapshot");
    let kept = snap.last_synced.map(|s| height_of(s.height)) == Some(TIP);
    let qualified = relaunch_reads_qualified(&snap);
    println!(
        "[T0-1c-R2 PLANT relaunch] after relaunch: ever_synced {}, last_synced {:?}, sync {:?} — \
         kept-at-TIP {kept}, qualified-by-shape {qualified}",
        snap.ever_synced, snap.last_synced, snap.sync
    );
    assert!(
        kept || qualified,
        "PLANT: a wallet that reached TIP honestly and then met a behind server relaunches \
         with last_synced {:?} — its \"as of block\" rewritten DOWN by a number only the \
         behind server supplied, unqualified. A gate that protects only the never-synced \
         wallet (G2's name) leaves every synced wallet's header at the mercy of one stuck \
         validator. Whole snapshot: {snap:?}",
        snap.last_synced
    );
    assert_carries_no_fixture_datum(
        &p.status,
        &[served_honest, served].concat(),
        HOST_A,
        "PLANT relaunch",
    );
    w.close().await.expect("close");
}

/// **PLANTED — CONTROL.** A high identity height still cannot raise the
/// signing anchor past the scanned margin.
///
/// The clamp — `capable_tip = min(claimed, scanned + REORG_MAX_BLOCKS)` —
/// exists because an endpoint reporting `block_height = u64::MAX` once pinned
/// the anchor at `u32::MAX` and every later `Unknown` signed forever. G4 makes
/// the anchor monotone; this row pins that the clamp SURVIVES the change: the
/// scanning wallet at TIP meets an endpoint whose identity claims
/// `TIP +`[`R2_IDENTITY_OVERCLAIM`] with the correct branch (the verdict is
/// `Behind`, capable — asserted), and `capable_tip` read back is exactly
/// `TIP + REORG_MAX_BLOCKS`, not the claim.
///
/// **PREDICTION:** §4n decision 4's second option — "anchor on `judged_at`" —
/// taken as written re-opens `judged_at` is `max(scanned, claimed)` and
/// is UNCLAMPED, so the over-claim becomes the anchor; so does a monotone rule
/// written `max(previous, claimed)` with the `min` dropped. GREEN at the
/// contract commit (the clamp is there) and it must stay green.
#[tokio::test]
async fn a_high_identity_height_still_cannot_raise_the_signing_anchor_past_the_scanned_margin() {
    let over_claim = TIP + R2_IDENTITY_OVERCLAIM;
    let ceiling = TIP + u64::from(REORG_MAX_BLOCKS);
    assert!(
        R2_IDENTITY_OVERCLAIM > u64::from(REORG_MAX_BLOCKS),
        "premise: the over-claim exceeds the margin, so the clamp is the only thing that can \
         hold the anchor at {ceiling}"
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, _honest, served_honest) =
        scanned_wallet_at_tip(dir.path(), &vault, "PLANT over-claim").await;
    // Post-scan serves withhold (module doc, "the harness's limit"); the anchor
    // is written in `evaluate_consensus`, before the roots are asked.
    let again = ProofEndpoint::withholding_at(TIP);
    let r1 = again.receipt();
    let p1 = pass_over(&w, again).await;
    record_r2("PLANT over-claim: pass 1 (TIP again)", &p1, &r1);
    let s1 = w.consensus_status().await.expect("consensus_status");
    assert!(
        is_at_or_above_terminal_at(&p1.status, TIP),
        "precondition: pass 1 completes to the at-or-above terminal at TIP; got {}",
        describe(&p1)
    );
    assert_eq!(
        s1.grace_anchor_tip.map(height_of),
        Some(TIP),
        "precondition: capable_tip is TIP after a capable pass on a wallet scanned to TIP"
    );

    let braggart = ProofEndpoint::withholding_at(TIP).with_identity_height(over_claim);
    let receipt = braggart.receipt();
    let served = braggart.served_heights();
    let p2 = pass_over(&w, braggart).await;
    record_r2("PLANT over-claim: pass 2", &p2, &receipt);
    let s2 = w.consensus_status().await.expect("consensus_status");
    println!(
        "[T0-1c-R2 PLANT over-claim] identity {over_claim} over scanned {TIP}: capable_tip {:?} \
         (ceiling {ceiling}); verdict {:?}",
        s2.grace_anchor_tip, s2.verdict
    );
    assert!(
        is_at_or_above_terminal_at(&p2.status, TIP),
        "IT-10: the pass completes at TIP — only the identity height moved; got {}",
        describe(&p2)
    );
    assert!(
        s2.verdict.is_some_and(|v| v.is_signing_capable_check()),
        "IT-10: an identity above the scanned height with the correct branch is judged capable \
         (Behind, asked of the type), so the anchor was eligible to move; got {:?}",
        s2.verdict
    );
    assert_eq!(
        s2.grace_anchor_tip.map(height_of),
        Some(ceiling),
        "PLANT: capable_tip = min(claimed, scanned + REORG_MAX_BLOCKS) — the S253 clamp — must \
         survive the monotone rule: an identity claiming {over_claim} raises the anchor to at \
         most {ceiling} (scanned {TIP} + {REORG_MAX_BLOCKS}); got {:?}. An anchor on the \
         unclamped judged_at (max(scanned, claimed)), or a monotone rule written \
         max(previous, claimed), hands a lying-high endpoint its own grace",
        s2.grace_anchor_tip
    );
    assert_carries_no_fixture_datum(
        &p2.status,
        &[served_honest, served].concat(),
        HOST_A,
        "PLANT over-claim",
    );
    w.close().await.expect("close");
}

/// **PLANTED — DEFECT.** `BirthdayInFuture` on a provisioning pass is not the
/// wrong-chain server's sentence either.
///
/// G6 forbids two readings (`EndpointUnreachable`, `Internal`). The cheapest
/// repair that satisfies G6 is the third existing reason — `EndpointMisbehaving`,
/// the content tier's "switch servers … if every server is refused, rescan"
/// (E4's wrong-chain sentence). For a stuck validator that copy is right; for
/// the cell the clamp was BUILT for — a birthday typed above the real chain
/// (the config-typo guard, `provision.rs`'s own words) — no server switch and
/// no rescan ever helps, and the SDK cannot tell the two apart from the error.
/// §4n's scope asks for "its own reading"; this row asserts it by
/// distinguishability: the status G6's geometry publishes is not equal to the
/// status E4's geometry publishes on a sibling wallet (same seam, same pass
/// shape, the identity check failing instead of the birthday clamp).
///
/// **PREDICTION:** the implementer adds the `BirthdayInFuture` arm to
/// `stall_for` as `EndpointMisbehaving` — G6 green, this row red, and a user
/// with a mistyped birthday is told to switch servers forever. RED at the
/// contract commit for G6's reason (`EndpointUnreachable`).
#[tokio::test]
async fn a_configured_birthday_above_a_stuck_validators_tip_is_not_the_wrong_chain_sentence() {
    let tail = newest_bundled_mainnet();
    let configured = tail + 1;
    assert!(
        BEHIND_TIP < configured,
        "premise: the tip is below the configured birthday"
    );

    let vault = test_vault();

    // The sibling: E4's wrong-chain server, on its own wallet — the content
    // tier's sentence, driven here so the instrument can see it.
    let dir_e4 = tempfile::tempdir().expect("tempdir");
    let w_e4 = anchored_wallet(dir_e4.path(), &vault, HOST_A).await;
    let wrong_chain = pass_over(&w_e4, ProofEndpoint::honest().on_the_wrong_chain()).await;
    assert_eq!(
        wrong_chain.status,
        SyncStatus::Stalled {
            reason: StallReason::EndpointMisbehaving
        },
        "control (E4): a wrong-chain server says EndpointMisbehaving on this seam; got {}",
        describe(&wrong_chain)
    );

    // The row: G6's geometry.
    let dir = tempfile::tempdir().expect("tempdir");
    let mut config = cfg(dir.path(), HOST_A);
    config.birthday = Some(block(configured));
    let w = Wallet::create_with_vault(config, raw_seed(), Arc::clone(&vault))
        .await
        .expect("create");
    assert!(
        !w.account_exists().await.expect("account_exists"),
        "precondition: lazy"
    );
    let stuck = ProofEndpoint::honest_at(BEHIND_TIP, IRONWOOD_HONEST_BELOW_BUNDLE);
    let receipt = stuck.receipt();
    let served = stuck.served_heights();
    let p = pass_over(&w, stuck).await;
    record_r2("PLANT birthday-in-future vs wrong-chain", &p, &receipt);
    assert!(
        matches!(&p.result, Err(WalletError::BirthdayInFuture)),
        "IT-10: the pass stopped at the birthday clamp; got {}",
        describe(&p)
    );
    assert!(
        matches!(p.status, SyncStatus::Stalled { .. }),
        "fatal to the pass; got {}",
        describe(&p)
    );
    assert_ne!(
        stall_reason(&p.status),
        Some(StallReason::EndpointUnreachable),
        "PLANT (G6's clause): not a dead link; got {}",
        describe(&p)
    );
    assert_ne!(
        p.status,
        wrong_chain.status,
        "PLANT: a birthday above the server's tip and a server on the wrong chain are two \
         sentences — the first may be a stuck validator OR a mistyped birthday, and \"switch \
         servers, then rescan\" is wrong for the second; the reason must be its own. Got {} \
         vs the wrong-chain sibling {}",
        describe(&p),
        describe(&wrong_chain)
    );
    assert_carries_no_fixture_datum(&p.status, &served, HOST_A, "PLANT birthday-in-future");
    w_e4.close().await.expect("close");
    w.close().await.expect("close");
}

/// **PLANTED — DEFECT (outside the floor's list; §4n's own population).** A
/// wallet born above a behind server's tip does not crash the pass.
///
/// Crypto #13's four-cell sentence says the behind cell is unreachable on a
/// PROVISIONING pass that fails the birthday clamp (G6). One pass later the
/// same geometry has no clamp at all: a fresh wallet is imported EAGERLY at
/// the bundle's tree-state tail (FR-24 — `estimate_birthday(created_at)` on a
/// current binary IS the tail), so its birthday is `tail + 1`, above every
/// stuck validator below the row. This row anchors a wallet exactly there and
/// drives E1's stuck validator through the shipped pass: `evaluate_consensus`
/// passes, the account exists, `sync_once` grades the tip behind, serves the
/// roots, and then hands upstream's `update_chain_tip` a birthday ABOVE the
/// new tip. `ScanRange::from_parts` asserts `end >= start`, and `run_blocking`
/// re-raises a panic by contract — so at the contract commit the pass PANICS
/// (the test is red with upstream's message, not with an assertion of its
/// own), where a shipped build's sync loop dies with it. The row asserts what
/// the surface must say instead: not `Internal` ("restore from seed"), not
/// `EndpointUnreachable`, and — the behind cell being reachable on this pass —
/// something a healthy wallet cannot say.
///
/// **PREDICTION:** the implementer writes the four-cell matrix sentence for
/// the provisioning pass and the `BirthdayInFuture` reason (G6), and this
/// non-provisioning sibling — every fresh wallet's SECOND pass against P4's
/// population — stays a crash. Measured, not argued: the record line prints
/// whatever the pass returned; if it does not panic at the contract commit,
/// that is the report's finding and this row is a control.
#[tokio::test]
async fn a_wallet_born_above_a_behind_servers_tip_does_not_crash_the_pass() {
    let tail = newest_bundled_mainnet();
    assert!(
        BEHIND_TIP < tail,
        "premise: the stuck validator's tip ({BEHIND_TIP}) is below the tail ({tail})"
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    // Born at the tail: birthday tail + 1 — the eager fresh create's own height.
    let w = wallet_anchored_at(dir.path(), &vault, HOST_A, tail).await;
    assert_eq!(
        w.birthday_height()
            .await
            .expect("birthday_height")
            .map(height_of),
        Some(tail + 1),
        "precondition: the wallet's birthday is tail + 1, above the stuck validator's tip"
    );

    let stuck = ProofEndpoint::honest_at(BEHIND_TIP, IRONWOOD_HONEST_BELOW_BUNDLE);
    let receipt = stuck.receipt();
    let served = stuck.served_heights();
    let p = pass_over(&w, stuck).await;
    record_r2("PLANT born-above-tip", &p, &receipt);

    assert_ne!(
        stall_reason(&p.status),
        Some(StallReason::Internal),
        "PLANT: a server behind this wallet's birthday is not a local fault — 'restore from \
         seed' is the wrong next step; got {}",
        describe(&p)
    );
    assert_ne!(
        stall_reason(&p.status),
        Some(StallReason::EndpointUnreachable),
        "PLANT: the link answered — not 'check your connection'; got {}",
        describe(&p)
    );
    assert!(
        says_degraded(&p.status),
        "PLANT: the endpoint is below the row and below this wallet's birthday — the surface \
         says something a healthy wallet cannot; got {}; {}",
        describe(&p),
        where_it_stopped(&receipt)
    );
    assert_carries_no_fixture_datum(&p.status, &served, HOST_A, "PLANT born-above-tip");
    w.close().await.expect("close");
}

// ════════════════════════════════════════════════════════════════════════════
// T0-1c-R3 (§4n-R) — the repair round's new row. The plant above is R3-1,
// UNEDITED; G3 above is R3-3, re-aimed in place.
// ════════════════════════════════════════════════════════════════════════════

/// T0-1c-R3: the record line for the R3 rows (`--nocapture`), [`record_r2`]'s
/// shape under its own tag so the red-first artifact reads by item.
fn record_r3(row: &str, p: &Passed, receipt: &Receipt) {
    println!(
        "[T0-1c-R3 {row}] {} — {}",
        describe(p),
        where_it_stopped(receipt)
    );
}

/// T0-1c-R3 (§4n-R Q-R5): which of the two cells the contract names a status
/// landed in, by NAME — both variants exist at the contract commit (T0-1c's
/// `EndpointBehind`; `StallReason::BirthdayInFuture`, minted for G6 — P-R3).
/// `None` is every other status: a healthy terminal, a transient, one of the
/// stalls that would be a specific lie here, or a status this file cannot
/// name. Which cell a geometry lands in is the implementer's (§4n-R "does NOT
/// decide" 2); the rows print it.
fn q_r5_cell(s: &SyncStatus) -> Option<&'static str> {
    match s {
        SyncStatus::EndpointBehind { .. } => Some("EndpointBehind (the cell below the row)"),
        SyncStatus::Stalled {
            reason: StallReason::BirthdayInFuture,
        } => Some("Stalled { BirthdayInFuture } (the cell below the birthday)"),
        _ => None,
    }
}

/// **R3-2 — DEFECT (§4n-R; INC-024's second geometry — the T0-1c-R2 ruling's
/// probe P-1).** A configured birthday above a behind server's tip does not
/// crash the pass.
///
/// The born-above plant above stands for the fresh eager create, anchored
/// SYNTHETICALLY at the tail (birthday = tail + 1 — a number no shipped
/// resolver writes: FR-24's `estimate_birthday(created_at)` goes through
/// `bundled_treestate`'s anchor ceiling and is born at 3,427,311 on mainnet,
/// measured through `create_with_vault` at the T0-1c-R3 adjudication; the
/// plant reproduces the CLASS, this row the shipped configured arm). This is
/// the OTHER eager arm of the shipped create: `config.birthday` AT the bundle's
/// tree-state tail — a configured birthday at or below the tail is imported
/// OFFLINE by `provision::resolve_offline_birthday`, so the account exists
/// before any pass, and G6's `BirthdayInFuture` clamp (which needs a live tip
/// and runs only on a PROVISIONING pass) never sees this wallet. The birthday
/// read back is whatever `bundled_treestate`'s anchor ceiling makes of the
/// request (the ruling's 3,427,311 on mainnet — asserted as the observed value
/// against the tip, never assumed). The server is then FAR below it: a tip one
/// under Sapling's first subtree completion — at or above activation, so
/// upstream's `update_chain_tip` does not early-return; below every pool's
/// first completion, so no shard row exists at it and upstream takes its
/// `Historic` arm, `ScanRange::from_parts(birthday..tip + 1, Historic)` — the
/// arm P-1 recorded as `4125171..280101` on testnet, where the plant above hits
/// the `ChainTip` arm (`3459781..3455001`). Both arms assert `end >= start`; a
/// guard that sits before `record_chain_tip` closes both, one keyed on the
/// shard rows closes one. The wallet has scanned nothing, so the bind abstains
/// and the endpoint's empty serve is honest for that tip.
///
/// **Asserted by DISTINGUISHABILITY between the two cells §4n-R names** (Q-R5,
/// [`q_r5_cell`]): the published status is `EndpointBehind` (the tip is below
/// the row — "this server is behind; switch servers") or `Stalled {
/// BirthdayInFuture }` (G6's reading) — never `Internal`, never
/// `EndpointUnreachable`, never a healthy terminal, never a transient; the row
/// PRINTS which cell it landed in. Then a SECOND pass over the same geometry
/// runs and publishes a typed status in the same set — the loop is alive.
/// Nothing is scanned: the tip is below the birthday, so there is no block the
/// wallet could want.
///
/// **PLANTED clause (IT-1 +A), not in the floor:** the durable birthday reads
/// the same after both passes. A guard that "repairs" the inversion by moving
/// the birthday DOWN to the behind server's tip is M3's permanent write with a
/// new door — a lying-low tip buying a scan the wallet cannot undo by switching
/// servers — and one that moves it UP is the silent-fund-skip class; the
/// contract names neither.
///
/// **At the contract commit** (the T0-1c-R2 join): the pass PANICS in upstream
/// `scanning.rs:51` — recorded verbatim in the red-first artifact — and
/// `run_blocking` re-raises it, so a shipped build's `run_loop` task dies with
/// the last status frozen on glass, on every pass (INC-024).
#[tokio::test]
async fn a_configured_birthday_above_a_behind_servers_tip_does_not_crash_the_pass() {
    let tail = crate::checkpoints::treestate_table(Network::Main)
        .last()
        .map(|row| u64::from(row.0))
        .expect("the mainnet bundle has a tree-state tail");
    assert_eq!(
        tail,
        newest_bundled_mainnet(),
        "premise: the tree-state tail IS the newest bundled row the grade reads"
    );
    let activation = sapling_activation_from_params();
    // The Historic-arm tip: at or above activation, below every pool's first
    // completion (both asserted from the params and the measured constants).
    let far_tip = SAPLING_TRUTHFUL[0] - 1;
    assert!(
        far_tip >= activation,
        "premise: the behind tip ({far_tip}) is at or above Sapling activation ({activation}), \
         so upstream's `update_chain_tip` does not return early on it"
    );
    for (pool, first) in [
        ("sapling", SAPLING_TRUTHFUL[0]),
        ("orchard", ORCHARD_TRUTHFUL[0]),
        ("ironwood", IRONWOOD_TRUTHFUL[0]),
    ] {
        assert!(
            far_tip < first,
            "premise: the behind tip ({far_tip}) is below {pool}'s first completion ({first}), \
             so no shard row exists at it and upstream takes the `Historic` arm"
        );
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let mut config = cfg(dir.path(), HOST_A);
    config.birthday = Some(block(tail));
    let w = Wallet::create_with_vault(config, raw_seed(), Arc::clone(&vault))
        .await
        .expect("create");
    assert!(
        w.account_exists().await.expect("account_exists"),
        "precondition: a configured birthday AT the tail is imported EAGERLY (FR-24) — the \
         account exists before any pass, so no provisioning clamp can see this wallet"
    );
    let birthday = w
        .birthday_height()
        .await
        .expect("birthday_height")
        .map(height_of)
        .expect("precondition: the eager import wrote a birthday");
    assert!(
        birthday <= tail + 1,
        "precondition: the birthday read back ({birthday}) is at or below the request + 1 \
         ({tail} + 1) — an import never starts above what the host asked for"
    );
    assert!(
        birthday > far_tip + 1,
        "premise: the birthday ({birthday}) is above the behind tip + 1 ({far_tip} + 1) — the \
         inverted range upstream's `from_parts` asserts on"
    );
    println!(
        "[T0-1c-R3 R3-2] configured {tail} (the tail, eager); birthday read back {birthday}; \
         behind tip {far_tip} (activation {activation})"
    );

    // Pass 1.
    let stuck = ProofEndpoint::honest_at(far_tip, &[]);
    let receipt = stuck.receipt();
    let served = stuck.served_heights();
    let p = pass_over(&w, stuck).await;
    record_r3("R3-2 pass 1", &p, &receipt);
    assert_ne!(
        stall_reason(&p.status),
        Some(StallReason::Internal),
        "R3-2: a server behind this wallet's birthday is not a local fault — 'restore from \
         seed' is the wrong next step; got {}",
        describe(&p)
    );
    assert_ne!(
        stall_reason(&p.status),
        Some(StallReason::EndpointUnreachable),
        "R3-2: the link answered — not 'check your connection'; got {}",
        describe(&p)
    );
    let cell = q_r5_cell(&p.status).unwrap_or_else(|| {
        panic!(
            "R3-2: a birthday above the endpoint's tip publishes one of the two cells §4n-R \
             names — `EndpointBehind` below the row, or `Stalled {{ BirthdayInFuture }}` — \
             instead of handing upstream an inverted range; got {}; {}",
            describe(&p),
            where_it_stopped(&receipt)
        )
    });
    println!("[T0-1c-R3 R3-2] pass 1 landed in the cell: {cell}");
    assert!(
        receipt.scanned_ranges().is_empty(),
        "R3-2: the tip is below the birthday — there is no block this wallet could want, and \
         none was asked; got {:?}",
        receipt.scanned_ranges()
    );
    assert_carries_no_fixture_datum(&p.status, &served, HOST_A, "R3-2 pass 1");

    // Pass 2 — the loop is alive: the same geometry runs again and publishes.
    let stuck_again = ProofEndpoint::honest_at(far_tip, &[]);
    let receipt_again = stuck_again.receipt();
    let served_again = stuck_again.served_heights();
    let again = pass_over(&w, stuck_again).await;
    record_r3("R3-2 pass 2", &again, &receipt_again);
    let cell_again = q_r5_cell(&again.status).unwrap_or_else(|| {
        panic!(
            "R3-2: the SECOND pass over the same behind server publishes a typed cell too — \
             the loop is alive, not a one-shot; got {}; {}",
            describe(&again),
            where_it_stopped(&receipt_again)
        )
    });
    println!("[T0-1c-R3 R3-2] pass 2 landed in the cell: {cell_again}");
    assert!(
        receipt_again.scanned_ranges().is_empty(),
        "R3-2: nothing to scan on the second pass either; got {:?}",
        receipt_again.scanned_ranges()
    );
    assert_carries_no_fixture_datum(&again.status, &served_again, HOST_A, "R3-2 pass 2");

    // The planted clause: the durable birthday did not move.
    let after = w
        .birthday_height()
        .await
        .expect("birthday_height")
        .map(height_of);
    assert_eq!(
        after,
        Some(birthday),
        "R3-2 (planted): two passes against a server below the birthday moved the durable \
         birthday — down is M3's permanent over-scan through a new door, up is a silent fund \
         skip; neither is a guard's business"
    );
    w.close().await.expect("close");
}

// ── GRACE-1 harness (§4p) ────────────────────────────────────────────────────

/// GRACE-1: the block grace as the mechanism reads it (`constants.rs`), in this
/// file's `u64`; never a literal in a row.
const GRACE_BLOCKS: u64 = crate::constants::UNKNOWN_BRANCH_GRACE_BLOCKS as u64;
/// GRACE-1: the identity's claim advanced INSIDE the block rule — with room for
/// the margin a capable pass can move the anchor by (`grace_anchor`'s ceiling,
/// `REORG_MAX_BLOCKS`) still inside; asserted in every row that relies on it.
const G1_BLOCKS_INSIDE: u64 = 500;
/// GRACE-1: …and PAST it by more than that margin, so no anchor movement a
/// capable pass can make brings it back inside; asserted where relied on.
const G1_BLOCKS_PAST: u64 = 2_000;
/// GRACE-1: the one payment URI every send-gate probe here proposes — a mainnet
/// P2PKH address `derivation.rs`'s own vectors pin (single-step: never a TEX,
/// so the parked row's `paused` stays false, `intent_store.rs:312`). The gate
/// refuses BEFORE `propose_uri` parses it; only the permitted control reaches
/// the parse, and stops at the funds.
const G1_SEND_URI: &str = "zcash:t1hD4Xt5goM4Y9cMBCFbQLQNER68Zh1LDjc?amount=0.001";

/// Every `WalletError` variant at the contract commit (`error.rs`, read there).
/// The grace refusal — §4p item 2, DECIDED by G-1: a NEW variant with its own
/// kind and copy — is whichever name is not on this list; no row names it.
const WALLET_ERROR_VARIANTS_AT_THE_CONTRACT_COMMIT: &[&str] = &[
    "InvalidSeedLength",
    "InvalidMnemonic",
    "NoMnemonic",
    "SeedRequired",
    "SeedMismatch",
    "KeyDerivation",
    "WatchOnly",
    "InvalidViewingKey",
    "InvalidEndpoint",
    "BirthdayInFuture",
    "InvalidDbDir",
    "AmountOutOfRange",
    "MemoTooLong",
    "ReservedMemoNotSendable",
    "MemoRequiresShieldedRecipient",
    "AddressInvalid",
    "MemoInvalid",
    "MemoConflict",
    "ZeroValuedTransparentOutput",
    "PaymentUriInvalid",
    "TxidInvalid",
    "MachineMemoScopeInvalid",
    "KeystoreUnavailable",
    "KeystoreInconsistent",
    "SealVersionUnsupported",
    "SealInvalid",
    "VaultAbsent",
    "WrapArtifactInvalid",
    "WrapVersionUnsupported",
    "NotFound",
    "NetworkMismatch",
    "ProvisioningIncomplete",
    "StoreCorrupt",
    "DiskFull",
    "WalletAlreadyExists",
    "StoreBusy",
    "WalletAlreadyOpen",
    "WalletBusy",
    "InvalidState",
    "WalletOpen",
    "WipeWithPendingSwap",
    "RescanWithInFlightSend",
    "SwapAddressCheckRefused",
    "Sync",
    "NetworkUpgradeUnsupported",
    "InsufficientFunds",
    "SendAmountRequired",
    "ProposalAlreadyUsed",
    "ProposalStale",
    "ProposeFailed",
    "SignFailed",
    "TexSendLimitReached",
    "QueuedSendStale",
    "QueuedSendsFull",
    "SwapDepositInFlight",
    "Io",
];
/// Every `SyncStatus` variant at the contract commit (`state.rs`, read there).
const SYNC_STATUS_VARIANTS_AT_THE_CONTRACT_COMMIT: &[&str] = &[
    "Idle",
    "Connecting",
    "Scanning",
    "UpToDate",
    "UpToDateLimited",
    "UpToDateDegraded",
    "EndpointBehind",
    "Stalled",
    "Offline",
];
/// Every `StallReason` variant at the contract commit — the seven, not the
/// five `is_known_stall` predates: `EndpointMisbehaving` is this file's own
/// harness failure mode (the scanned-tree-size arm) and must never be read as
/// a minted expiry reading.
const STALL_REASONS_AT_THE_CONTRACT_COMMIT: &[&str] = &[
    "EndpointUnreachable",
    "TorUnavailable",
    "StorageFull",
    "ChainReorg",
    "Internal",
    "EndpointMisbehaving",
    "BirthdayInFuture",
];
/// The `ParkedSend` fields at the contract commit (`parked.rs`'s `Debug`).
const PARKED_FIELDS_AT_THE_CONTRACT_COMMIT: &[&str] = &[
    "id",
    "kind",
    "amount_zat",
    "created_at",
    "paused",
    "sending",
    "blocked_by_network_upgrade",
    "binding",
];
/// The `ConsensusStatusSnapshot` fields at the contract commit.
const CONSENSUS_STATUS_FIELDS_AT_THE_CONTRACT_COMMIT: &[&str] =
    &["verdict", "grace_anchor_tip", "evaluated_at_unix"];

/// GRACE-1: the record line for these rows (`--nocapture`), [`record`]'s shape
/// under its own tag so the red-first artifact reads by item.
fn record_g1(row: &str, p: &Passed, receipt: &Receipt) {
    println!(
        "[GRACE-1 {row}] {} — {}",
        describe(p),
        where_it_stopped(receipt)
    );
}

/// The variant name a one-line `Debug` rendering starts with.
fn debug_variant(rendered: &str) -> String {
    rendered
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

/// The `name: value` pairs at exactly `depth` levels of brackets inside a
/// one-line `Debug` rendering — [`debug_fields_at_depth`] with the values
/// kept (a value runs to the next `,` at its own level, or the closing
/// bracket). Nested values keep their brackets balanced, so
/// `binding: Some("SpendBinding(..)")` is one pair.
fn debug_pairs_at_depth(rendered: &str, depth: usize) -> Vec<(String, String)> {
    let chars: Vec<char> = rendered.chars().collect();
    let mut level = 0usize;
    let mut pairs = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '{' | '(' | '[' => {
                level += 1;
                i += 1;
            }
            '}' | ')' | ']' => {
                level = level.saturating_sub(1);
                i += 1;
            }
            c if level == depth && (c.is_ascii_alphabetic() || c == '_') => {
                let s = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                if chars.get(i) == Some(&':') && chars.get(i + 1) == Some(&' ') {
                    let name: String = chars[s..i].iter().collect();
                    i += 2;
                    let vs = i;
                    let mut inner = 0usize;
                    while i < chars.len() {
                        match chars[i] {
                            '{' | '(' | '[' => inner += 1,
                            '}' | ')' | ']' => {
                                if inner == 0 {
                                    break;
                                }
                                inner -= 1;
                            }
                            ',' if inner == 0 => break,
                            _ => {}
                        }
                        i += 1;
                    }
                    let value: String = chars[vs..i].iter().collect();
                    pairs.push((name, value.trim().to_owned()));
                }
            }
            _ => i += 1,
        }
    }
    pairs
}

/// A `WalletError` whose variant did not exist at the contract commit — the
/// grace refusal, by distinguishability (§4p item 2, decided by G-1).
fn is_minted_error(e: &WalletError) -> bool {
    !WALLET_ERROR_VARIANTS_AT_THE_CONTRACT_COMMIT
        .contains(&debug_variant(&format!("{e:?}")).as_str())
}

/// The stale-build refusal, by name — it exists, and it is what an expired
/// grace must NOT be.
fn is_upgrade_refusal(e: &WalletError) -> bool {
    matches!(e, WalletError::NetworkUpgradeUnsupported { .. })
}

/// What the send gate answered. `Permitted`: the propose got PAST the consensus
/// gate — into the parse and the funds, where this unfunded wallet stops with
/// the variant named in [`send_gate`]. `Refused`: the gate's own refusal — the
/// contract commit's carrier or a minted one. Anything else is a harness fault
/// and panics with its value (IT-10: the refuser must be the gate).
enum Gate {
    Permitted(String),
    Refused(WalletError),
}

/// Drive the real send gate: `propose_uri` asks `signing_permit` → `granted`
/// BEFORE it parses the URI (`wallet.rs:2053-2072` at `bea7960c`), so a
/// refusal here is the gate's and nothing after it has run.
async fn send_gate(w: &Wallet, row: &str) -> Gate {
    match w.propose_uri(G1_SEND_URI).await {
        Ok(_) => Gate::Permitted("Ok(proposal)".to_owned()),
        // provenance: measured at the contract commit on the G-8 control —
        // the unfunded scanned wallet's propose gets past the gate and stops
        // at the audited selector's verdict.
        Err(e @ WalletError::InsufficientFunds { .. }) => {
            Gate::Permitted(format!("{e:?} / code {}", e.code()))
        }
        Err(e) if is_upgrade_refusal(&e) || is_minted_error(&e) => Gate::Refused(e),
        Err(other) => panic!(
            "{row} — harness: propose_uri answered neither the gate nor the funds: {other:?} / \
             code {}",
            other.code()
        ),
    }
}

fn refused(g: &Gate) -> Option<&WalletError> {
    match g {
        Gate::Refused(e) => Some(e),
        Gate::Permitted(_) => None,
    }
}

fn describe_gate(g: &Gate) -> String {
    match g {
        Gate::Permitted(s) => format!("permitted (past the gate: {s})"),
        Gate::Refused(e) => format!("refused: {e:?} / code {}", e.code()),
    }
}

/// The parked row's BLOCKAGE reading, in the contract commit's vocabulary:
/// `blocked_by_network_upgrade: <bool>` plus every `name: value` the contract
/// commit's `ParkedSend` does not have — item 2 leaves the four states' shape
/// to the implementer, and this reads it without naming it. Sorted, so two
/// readings compare as sets. Exactly one queued row is expected (the row the
/// test queued).
///
/// **Adjudication repair (GRACE-1; charged to the test half).** As
/// written this reader required the base's bool to SURVIVE by name ("it
/// exists"), which its own doc disclaims: item 2 left the flag's shape open,
/// and the implementer replaced the bool with `signing_block:
/// Option<SigningBlock>` (a bool cannot carry four states — G-6's own
/// sentence). Five rows redded on `got ["signing_block: None"]`. The reader now
/// PROJECTS that field onto the reading the twelve assertion sites expect,
/// with the base's meaning kept exactly: `None` (the row will sign on its own)
/// → `blocked_by_network_upgrade: false`; `Some(NetworkUpgrade)` (the
/// app-update block, `Unsupported`) → `blocked_by_network_upgrade: true`; any
/// other `Some(..)` (an ended grace) → `blocked_by_network_upgrade: false`
/// PLUS the field line itself, so the reading is not healthy, differs from
/// the `Unsupported` one, and names its reason. A surviving bool is read as
/// before; every other new field is kept verbatim. No assertion changed.
async fn parked_blockage(w: &Wallet, row: &str) -> Vec<String> {
    let rows = w.list_parked_sends().await.expect("list_parked_sends");
    assert_eq!(
        rows.len(),
        1,
        "{row} — harness: exactly one queued row is parked; got {rows:?}"
    );
    let rendered = format!("{:?}", rows[0]);
    let mut reading: Vec<String> = Vec::new();
    for (n, v) in debug_pairs_at_depth(&rendered, 1) {
        if n == "signing_block" {
            if v == "None" {
                reading.push("blocked_by_network_upgrade: false".to_owned());
            } else if v.starts_with("Some(NetworkUpgrade") {
                reading.push("blocked_by_network_upgrade: true".to_owned());
            } else {
                reading.push("blocked_by_network_upgrade: false".to_owned());
                reading.push(format!("{n}: {v}"));
            }
        } else if n == "blocked_by_network_upgrade"
            || !PARKED_FIELDS_AT_THE_CONTRACT_COMMIT.contains(&n.as_str())
        {
            reading.push(format!("{n}: {v}"));
        }
    }
    reading.sort();
    reading.dedup();
    reading
}

/// The row renders as an ordinary pending send: the flag false and nothing the
/// contract commit's `ParkedSend` lacks.
fn parked_reads_healthy(reading: &[String]) -> bool {
    reading == ["blocked_by_network_upgrade: false"]
}

/// The contract commit's rendering of an at-or-above terminal: `UpToDate {
/// tip }` or `UpToDateDegraded { tip, pools }` with no field either lacks
/// there. Its negation on one of those two variants is "the implementer widened
/// the terminal".
fn is_base_at_or_above_shape(s: &SyncStatus) -> bool {
    let fields = debug_fields_at_depth(&format!("{s:?}"), 1);
    match s {
        SyncStatus::UpToDate { .. } => fields.iter().all(|f| f == "tip"),
        SyncStatus::UpToDateDegraded { .. } => fields.iter().all(|f| f == "tip" || f == "pools"),
        _ => false,
    }
}

/// G-5 by distinguishability: the surface carries something the contract
/// commit's cannot say about an `Unknown` inside its grace — a variant not on
/// the list, or a field the at-or-above terminals do not have. NOT a stall, a
/// transient, `UpToDateLimited` or `EndpointBehind`: each of those is a specific
/// lie about a server that merely went quiet.
fn shows_grace(s: &SyncStatus) -> bool {
    let minted = !SYNC_STATUS_VARIANTS_AT_THE_CONTRACT_COMMIT
        .contains(&debug_variant(&format!("{s:?}")).as_str());
    let widened = matches!(
        s,
        SyncStatus::UpToDate { .. } | SyncStatus::UpToDateDegraded { .. }
    ) && !is_base_at_or_above_shape(s);
    minted || widened
}

/// A stall whose reason did not exist at the contract commit.
fn is_minted_stall_reason(s: &SyncStatus) -> bool {
    stall_reason(s).is_some_and(|r| {
        !STALL_REASONS_AT_THE_CONTRACT_COMMIT.contains(&debug_variant(&format!("{r:?}")).as_str())
    })
}

/// The EXPIRED reading may be [`shows_grace`]'s shape or a stall with a minted
/// reason; never the contract commit's terminal and never one of its seven
/// stalls (`EndpointMisbehaving` is this harness's own failure mode).
fn says_expired(s: &SyncStatus) -> bool {
    shows_grace(s) || is_minted_stall_reason(s)
}

/// G-5's relaunch clause, by shape (§4p item 1 leaves what a relaunch shows to
/// the implementer): the snapshot is qualified the way the R2 plant reads it
/// ([`relaunch_reads_qualified`]), or the cold read carries a field the contract
/// commit's `ConsensusStatusSnapshot` does not. The adjudicator refines to the
/// field's name.
fn relaunch_carries_the_grace(
    snap: &WalletState,
    cold: &crate::consensus::ConsensusStatusSnapshot,
) -> bool {
    let cold_fields = debug_fields_at_depth(&format!("{cold:?}"), 1);
    relaunch_reads_qualified(snap)
        || cold_fields
            .iter()
            .any(|f| !CONSENSUS_STATUS_FIELDS_AT_THE_CONTRACT_COMMIT.contains(&f.as_str()))
}

/// GRACE-1: one capable pass at [`TIP`] over the scanned wallet — the G4 row's
/// pass 1 — so the anchor reads TIP (the honest pass is judged BEFORE its scan
/// and anchors at `TIP − 1,100`). Asserts the anchor and returns the served
/// heights extended by this pass's.
async fn anchor_at_tip(w: &Wallet, mut served: Vec<u64>, row: &str) -> Vec<u64> {
    let again = ProofEndpoint::withholding_at(TIP);
    let receipt = again.receipt();
    served.extend(again.served_heights());
    let p = pass_over(w, again).await;
    record_g1(&format!("{row} capable pass at TIP"), &p, &receipt);
    assert!(
        is_at_or_above_terminal_at(&p.status, TIP),
        "{row} — precondition: the capable pass completes to the at-or-above terminal at TIP; \
         got {}",
        describe(&p)
    );
    let s = w.consensus_status().await.expect("consensus_status");
    assert!(
        s.verdict.is_some_and(|v| v.is_signing_capable_check()),
        "{row} — precondition: a capable verdict; got {:?}",
        s.verdict
    );
    assert_eq!(
        s.grace_anchor_tip.map(height_of),
        Some(TIP),
        "{row} — precondition: the anchor is TIP after a capable pass at TIP on a wallet \
         scanned to TIP (the G4 row's pass 1); got {:?}",
        s.grace_anchor_tip
    );
    served
}

/// GRACE-1: the scanned wallet with its anchor at TIP, under the system clock
/// — [`scanned_wallet_at_tip`] then [`anchor_at_tip`].
async fn scanned_wallet_anchored_at_tip(
    dir: &Path,
    vault: &Arc<dyn KeychainPort>,
    row: &str,
) -> (Wallet, Vec<u64>) {
    let (w, _honest, served) = scanned_wallet_at_tip(dir, vault, row).await;
    let served = anchor_at_tip(&w, served, row).await;
    (w, served)
}

/// GRACE-1: one pass over an endpoint at [`TIP`] that OMITS its branch id, its
/// identity claiming `TIP + advance` (§6.3's silent server; a withholding serve
/// — module doc, "the harness's limit"; `advance = 0` is the frozen claim).
/// Asserts the pass completed and the pools were asked (so the verdict, which
/// runs before them, was THIS pass's), and that the verdict recorded is
/// `Unknown` — neither capable nor blocking, asked of the type. Returns the
/// pass and the heights served.
async fn unknown_pass(w: &Wallet, advance: u64, row: &str) -> (Passed, Vec<u64>) {
    let mut ep = ProofEndpoint::withholding_at(TIP).omitting_branch_id();
    if advance > 0 {
        ep = ep.with_identity_height(TIP + advance);
    }
    let receipt = ep.receipt();
    let served = ep.served_heights();
    let p = pass_over(w, ep).await;
    record_g1(row, &p, &receipt);
    assert!(
        p.result.is_ok(),
        "{row} — precondition: the Unknown pass completes; got {}",
        describe(&p)
    );
    assert!(
        receipt.opened(ShieldedProtocol::Sapling) >= 1,
        "{row} — IT-10: the pools were asked, so the verdict read below was written by this \
         pass's evaluation"
    );
    let v = w
        .consensus_status()
        .await
        .expect("consensus_status")
        .verdict
        .unwrap_or_else(|| panic!("{row} — precondition: a verdict was recorded"));
    assert!(
        !v.is_signing_capable_check() && !v.blocks_interpretation(),
        "{row} — an identity without a branch id is judged Unknown (neither capable nor \
         blocking); got {v:?}"
    );
    (p, served)
}

/// GRACE-1: one pass over an endpoint at [`TIP`] that REPORTS its branch, the
/// identity claiming `TIP + advance` with the branch honest for that height —
/// a capable verdict (`Current` at TIP, `Behind` above it), asserted.
async fn capable_pass(w: &Wallet, advance: u64, row: &str) -> (Passed, Vec<u64>) {
    let mut ep = ProofEndpoint::withholding_at(TIP);
    if advance > 0 {
        ep = ep.with_identity_height(TIP + advance);
    }
    let receipt = ep.receipt();
    let served = ep.served_heights();
    let p = pass_over(w, ep).await;
    record_g1(row, &p, &receipt);
    assert!(
        p.result.is_ok(),
        "{row} — precondition: the capable pass completes; got {}",
        describe(&p)
    );
    let v = w
        .consensus_status()
        .await
        .expect("consensus_status")
        .verdict
        .unwrap_or_else(|| panic!("{row} — precondition: a verdict was recorded"));
    assert!(
        v.is_signing_capable_check(),
        "{row} — an identity reporting the branch our params compute earns a capable \
         verdict; got {v:?}"
    );
    (p, served)
}

/// GRACE-1 (G-6's control): one pass over an endpoint at [`TIP`] whose identity
/// claims a branch our params do not compute — `Unsupported`, asserted through
/// `blocks_interpretation()`, the type's own word for it.
async fn unsupported_pass(w: &Wallet, row: &str) -> (Passed, Vec<u64>) {
    let ep = ProofEndpoint::withholding_at(TIP).claiming_a_bogus_branch();
    let receipt = ep.receipt();
    let served = ep.served_heights();
    let p = pass_over(w, ep).await;
    record_g1(row, &p, &receipt);
    assert!(
        p.result.is_ok(),
        "{row} — precondition: the Unsupported pass completes (the verdict is recorded, the \
         pass goes on); got {}",
        describe(&p)
    );
    let v = w
        .consensus_status()
        .await
        .expect("consensus_status")
        .verdict
        .unwrap_or_else(|| panic!("{row} — precondition: a verdict was recorded"));
    assert!(
        v.blocks_interpretation(),
        "{row} — a branch id our params do not compute is judged Unsupported; got {v:?}"
    );
    (p, served)
}

// ════════════════════════════════════════════════════════════════════════════
// GRACE-1 — the rows that compile at the contract commit (§4p G-5, G-6, G-8
// and a planted control). The clock rows are in `clock_seam`, at the end.
// ════════════════════════════════════════════════════════════════════════════

/// **G-8 — CONTROL.** An honest old server inside both rules is permitted and
/// shown.
///
/// The scanned wallet anchored at TIP; a queued send (ungated); one pass over
/// an endpoint that omits its branch id and claims `TIP + 500` — 500 blocks
/// into a 1,152-block grace, and seconds into a day. The send gate PERMITS
/// (the propose gets past the gate and stops at the funds — by name), the
/// parked row is not blocked, the surface is neither `UpToDateLimited` (that
/// says "update the app") nor `EndpointBehind` nor a stall nor a transient.
///
/// The "grace surface shown" half of G-8 is G-5's DEFECT mechanism, asserted
/// there; asserting it here would make this control red at the contract commit
/// for G-5's reason. GREEN at the contract commit by the existing block rule
/// and by the absence of any clock rule; it must stay green — it is the row
/// that fails an "either" built as "both", and a clock rule that refuses on
/// its own first reading.
#[tokio::test]
async fn an_honest_old_server_inside_both_rules_is_permitted_and_shown() {
    assert!(
        G1_BLOCKS_INSIDE + u64::from(REORG_MAX_BLOCKS) < GRACE_BLOCKS,
        "premise: {G1_BLOCKS_INSIDE} blocks (plus the margin a capable pass can move the \
         anchor by) is inside the {GRACE_BLOCKS}-block grace"
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, served) = scanned_wallet_anchored_at_tip(dir.path(), &vault, "G-8").await;
    w.queue_send_uri(G1_SEND_URI)
        .await
        .expect("queueing is ungated — composing works with no verdict at all");

    let (p, served2) = unknown_pass(
        &w,
        G1_BLOCKS_INSIDE,
        &format!("G-8 Unknown pass, claim TIP + {G1_BLOCKS_INSIDE}"),
    )
    .await;
    let gate = send_gate(&w, "G-8").await;
    let parked = parked_blockage(&w, "G-8").await;
    println!(
        "[GRACE-1 G-8] gate: {}; parked: {parked:?}; surface: {:?}",
        describe_gate(&gate),
        p.status
    );

    assert!(
        matches!(gate, Gate::Permitted(_)),
        "G-8: {G1_BLOCKS_INSIDE} blocks into the grace and seconds into the day, signing is \
         PERMITTED — the propose gets past the gate; got {}",
        describe_gate(&gate)
    );
    assert!(
        parked.contains(&"blocked_by_network_upgrade: false".to_owned()),
        "G-8: the parked row is not blocked while the grace runs; got {parked:?}"
    );
    assert!(
        !matches!(p.status, SyncStatus::UpToDateLimited { .. }),
        "G-8: UpToDateLimited NOT shown — that reading says 'update the app', false for a \
         server that merely went quiet; got {}",
        describe(&p)
    );
    assert!(
        !matches!(
            p.status,
            SyncStatus::EndpointBehind { .. } | SyncStatus::Stalled { .. }
        ) && !is_transient(&p.status),
        "G-8: the surface is the at-or-above terminal or the grace reading (G-5 owns the \
         DEFECT half), never a behind/stall/transient sentence; got {}",
        describe(&p)
    );
    assert_carries_no_fixture_datum(&p.status, &[served, served2].concat(), HOST_A, "G-8");
    w.close().await.expect("close");
}

/// **G-5 — DEFECT (UI).** The grace is visible while it runs.
///
/// Two `Unknown` passes inside both rules over the scanned wallet anchored at
/// TIP — the claim frozen at TIP, then `TIP + 500` — and a relaunch. Each
/// surface must NOT be the contract commit's at-or-above shape
/// ([`shows_grace`]: a variant the commit lacks, or a field its two terminals
/// lack); the two must DIFFER, because the reading says how much grace remains
/// and the second claim has spent 500 blocks of it; and after `close()` /
/// `open()` the reading survives ([`relaunch_carries_the_grace`]: a qualified
/// snapshot or a cold-read field the commit lacks).
///
/// **At the contract commit:** both passes publish the same
/// `UpToDateDegraded { TIP, pools: Withheld × 3 }` — this file's post-scan
/// terminal (the harness's limit; a real old server serving full roots reads
/// plain `UpToDate`, P-G3) — and the relaunch snapshot is the R2 plant's
/// unqualified one. An `Unknown` inside its grace is indistinguishable from a
/// capable pass at the surface.
#[tokio::test]
async fn the_grace_is_visible_while_it_runs() {
    assert!(
        G1_BLOCKS_INSIDE + u64::from(REORG_MAX_BLOCKS) < GRACE_BLOCKS,
        "premise: {G1_BLOCKS_INSIDE} blocks is inside the {GRACE_BLOCKS}-block grace"
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, served) = scanned_wallet_anchored_at_tip(dir.path(), &vault, "G-5").await;

    let (pa, served_a) = unknown_pass(&w, 0, "G-5 pass A, claim frozen at TIP").await;
    let (pb, served_b) = unknown_pass(
        &w,
        G1_BLOCKS_INSIDE,
        &format!("G-5 pass B, claim TIP + {G1_BLOCKS_INSIDE}"),
    )
    .await;
    let ra = format!("{:?}", pa.status);
    let rb = format!("{:?}", pb.status);
    w.close().await.expect("close");
    let w = Wallet::open_with_vault(cfg(dir.path(), HOST_A), Arc::clone(&vault))
        .await
        .expect("relaunch");
    let snap = w.snapshot().await.expect("snapshot");
    let cold = w.consensus_status().await.expect("consensus_status");
    println!(
        "[GRACE-1 G-5] surface A: {ra}; surface B: {rb}; after relaunch: snapshot {snap:?}; cold \
         read {cold:?}"
    );

    assert!(
        shows_grace(&pa.status),
        "G-5: under Unknown inside both rules the sync surface is NOT the contract commit's \
         at-or-above terminal (plain UpToDate on a real old server; on this file's scanned \
         wallet the withheld-pools UpToDateDegraded) — a distinct reading that the server \
         stopped reporting the network version and the grace is running; got {ra}"
    );
    assert!(
        shows_grace(&pb.status),
        "G-5: …and still is {G1_BLOCKS_INSIDE} blocks in; got {rb}"
    );
    assert_ne!(
        ra, rb,
        "G-5: the reading says HOW MUCH grace remains (whichever of blocks and time expires \
         first), so a claim {G1_BLOCKS_INSIDE} blocks further along reads differently — at \
         the contract commit both render the same terminal"
    );
    assert!(
        relaunch_carries_the_grace(&snap, &cold),
        "G-5: the reading survives a relaunch — the snapshot is qualified, or the cold read \
         carries a field the contract commit's ConsensusStatusSnapshot does not; got \
         snapshot {snap:?}, cold read {cold:?}"
    );
    assert_carries_no_fixture_datum(&pa.status, &[served, served_a].concat(), HOST_A, "G-5 A");
    assert_carries_no_fixture_datum(&pb.status, &served_b, HOST_A, "G-5 B");
    w.close().await.expect("close");
}

/// **G-6 — DEFECT (UI), the core half.** An expired grace says why and what to
/// do. (The copy half — the arb strings never say "upgraded" / "update the app"
/// — is `extraction_policy::the_grace_copy_never_says_upgraded_or_update_the_app`.)
///
/// Three states on one scanned wallet with one queued row, in this order so no
/// state's reading is carried into the next by anything but the mechanism:
/// (1) `Unsupported` — a branch our params do not compute — the control: the
/// send refuses `NetworkUpgradeUnsupported` BY NAME, the surface is
/// `UpToDateLimited { TIP }`, the parked row reads `blocked_by_network_upgrade:
/// true` — every one of those sentences ("the network was upgraded", "update
/// the app", "waiting for an app update") is TRUE here. (2) A capable pass —
/// permitted again. (3) `Unknown` with the claim `TIP + 2,000`, past the block
/// grace by more than the reorg margin: the send REFUSES with a carrier that is
/// NOT `NetworkUpgradeUnsupported` — a variant the contract commit does not
/// have (item 2, decided by G-1) — that names its reason (BLOCKS, not the
/// clock); the parked row is not healthy AND its blockage reading differs from
/// state (1)'s (`walletParkedBlockedByNetworkUpgrade` renders ONLY for
/// `Unsupported` and never-evaluated, and a bool cannot carry that); the
/// surface says the grace expired ([`says_expired`]).
///
/// **At the contract commit:** (3) refuses `NetworkUpgradeUnsupported {
/// expected_branch_id: 0, endpoint_branch_id: None, judged_at_height: TIP +
/// 2000 }` — the bridge renders it as "The Zcash network was upgraded and this
/// app needs an update before it can send", the parked row as "Waiting for an
/// app update" — for a server that merely went silent, nothing was upgraded
/// and an update fixes nothing; the spec's own §6.3 sentence ("this server
/// will not say what network it is on, so try another one") was never
/// delivered. The surface is the same `UpToDateDegraded` as a capable pass.
#[tokio::test]
async fn an_expired_grace_says_why_and_what_to_do() {
    assert!(
        G1_BLOCKS_PAST >= GRACE_BLOCKS + u64::from(REORG_MAX_BLOCKS),
        "premise: {G1_BLOCKS_PAST} blocks is past the {GRACE_BLOCKS}-block grace by more than \
         the margin a capable pass can move the anchor by"
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, served) = scanned_wallet_anchored_at_tip(dir.path(), &vault, "G-6").await;
    w.queue_send_uri(G1_SEND_URI)
        .await
        .expect("queueing is ungated");

    // (1) the stale-build refusal — the control every grace sentence must differ from
    let (pu, served_u) = unsupported_pass(&w, "G-6 (1) Unsupported pass at TIP").await;
    let gate_u = send_gate(&w, "G-6 (1) under Unsupported").await;
    let parked_u = parked_blockage(&w, "G-6 (1) under Unsupported").await;
    // (2) capable again
    let (_pc, served_c) = capable_pass(&w, 0, "G-6 (2) capable pass at TIP").await;
    let gate_c = send_gate(&w, "G-6 (2) after the capable pass").await;
    // (3) expired by blocks
    let (pe, served_e) = unknown_pass(
        &w,
        G1_BLOCKS_PAST,
        &format!("G-6 (3) Unknown pass, claim TIP + {G1_BLOCKS_PAST}"),
    )
    .await;
    let gate_e = send_gate(&w, "G-6 (3) expired by blocks").await;
    let parked_e = parked_blockage(&w, "G-6 (3) expired by blocks").await;
    println!(
        "[GRACE-1 G-6] (1) Unsupported: gate {}; parked {parked_u:?}; surface {:?}\n[GRACE-1 \
         G-6] (2) capable: gate {}\n[GRACE-1 G-6] (3) expired by blocks: gate {}; parked \
         {parked_e:?}; surface {:?}",
        describe_gate(&gate_u),
        pu.status,
        describe_gate(&gate_c),
        describe_gate(&gate_e),
        pe.status
    );

    // the controls (green at the contract commit)
    assert!(
        matches!(pu.status, SyncStatus::UpToDateLimited { tip } if height_of(tip) == TIP),
        "control: Unsupported renders UpToDateLimited at TIP (P-G3); got {}",
        describe(&pu)
    );
    assert!(
        refused(&gate_u).is_some_and(is_upgrade_refusal),
        "control: the stale-build refusal is NetworkUpgradeUnsupported, by name — 'update the \
         app' is TRUE here; got {}",
        describe_gate(&gate_u)
    );
    assert!(
        parked_u.contains(&"blocked_by_network_upgrade: true".to_owned()),
        "control: under Unsupported the parked row is blocked-by-network-upgrade — 'Waiting \
         for an app update' is TRUE here; got {parked_u:?}"
    );
    assert!(
        matches!(gate_c, Gate::Permitted(_)),
        "control: a capable pass permits again; got {}",
        describe_gate(&gate_c)
    );
    // the DEFECT
    let e = refused(&gate_e).unwrap_or_else(|| {
        panic!(
            "G-6: a grace expired by blocks ({G1_BLOCKS_PAST} ≥ {GRACE_BLOCKS}) REFUSES; got {}",
            describe_gate(&gate_e)
        )
    });
    assert!(
        !is_upgrade_refusal(e),
        "G-6: the refusal is NOT NetworkUpgradeUnsupported — that carrier crosses the bridge \
         as WalletErrorKind.networkUpgradeUnsupported and renders 'The Zcash network was \
         upgraded and this app needs an update', which is FALSE for a server that merely \
         stopped reporting the network version; got {e:?}"
    );
    assert!(
        is_minted_error(e),
        "G-6: …it is a WalletError variant the contract commit does not have (§4p item 2, \
         decided by G-1), distinguishable BY TYPE on the bridge and in Dart; got {e:?}"
    );
    let er = format!("{e:?}").to_ascii_lowercase();
    assert!(
        er.contains("block"),
        "G-6: the carrier names its reason — expired by BLOCKS ('…for N blocks — switch \
         servers'); got {e:?}"
    );
    assert!(
        !er.contains("clock"),
        "G-6: …and not the clock's reason (the clock read seconds); got {e:?}"
    );
    assert!(
        !parked_reads_healthy(&parked_e),
        "G-6: the parked row does not render as a healthy pending send while the drain skips \
         it; got {parked_e:?}"
    );
    assert_ne!(
        parked_e, parked_u,
        "G-6: the parked row under an expired grace is distinguishable from the row under \
         Unsupported — walletParkedBlockedByNetworkUpgrade ('Waiting for an app update') \
         renders ONLY for Unsupported / never-evaluated, and one bool cannot carry that"
    );
    assert!(
        says_expired(&pe.status),
        "G-6: the sync surface says the grace EXPIRED (by blocks) — not the contract commit's \
         at-or-above terminal, and none of its seven stalls; got {}",
        describe(&pe)
    );
    assert_carries_no_fixture_datum(
        &pe.status,
        &[served, served_u, served_c, served_e].concat(),
        HOST_A,
        "G-6",
    );
    w.close().await.expect("close");
}

/// **PLANTED — CONTROL.** A wallet that never evaluated a verdict is not
/// refused as an expired grace.
///
/// The floor names four refusal states (Unsupported, expired by blocks, expired
/// by clock, never evaluated) and item 2 leaves the never-evaluated case's
/// copy open — `NetworkUpgradeUnsupported`-shaped, or its own. What it may NOT
/// become is the GRACE refusal: "this server hasn't reported the network
/// version for N blocks / a day — switch servers" is false for a wallet that
/// has never met a server. A fresh wallet, no pass, no verdict (asserted), a
/// send: refused (a wallet that never checked does not sign — the incident's
/// own rule), and the refusal is `NetworkUpgradeUnsupported` by name, or a
/// minted variant that names neither blocks nor the clock. The parked reading
/// of the never-evaluated state is RECORDED, not asserted (item 2 leaves it to
/// the implementer to state).
///
/// **PREDICTION:** an implementer who reshapes `granted`'s `None` arm
/// (`consensus.rs:260`) into the new carrier because both cases "have no
/// branch" makes a fresh install say "switch servers" before it has met one.
/// GREEN at the contract commit and it must stay green.
#[tokio::test]
async fn a_wallet_that_never_evaluated_a_verdict_is_not_refused_as_an_expired_grace() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = unprovisioned_wallet(dir.path(), &vault, HOST_A).await;
    assert!(
        w.consensus_status()
            .await
            .expect("consensus_status")
            .verdict
            .is_none(),
        "precondition: no pass has run, so no verdict exists"
    );
    w.queue_send_uri(G1_SEND_URI)
        .await
        .expect("queueing is ungated — even with no verdict at all");
    let gate = send_gate(&w, "PLANT never-evaluated").await;
    let parked = parked_blockage(&w, "PLANT never-evaluated").await;
    println!(
        "[GRACE-1 PLANT never-evaluated] gate: {}; parked (recorded, not asserted — item 2): \
         {parked:?}",
        describe_gate(&gate)
    );
    let e = refused(&gate).unwrap_or_else(|| {
        panic!(
            "PLANT: a wallet that has never evaluated a verdict does not sign; got {}",
            describe_gate(&gate)
        )
    });
    let er = format!("{e:?}").to_ascii_lowercase();
    assert!(
        is_upgrade_refusal(e)
            || (is_minted_error(e) && !er.contains("clock") && !er.contains("block")),
        "PLANT: the never-evaluated refusal keeps a shape of its own — NetworkUpgradeUnsupported \
         (as today) or a variant that names neither blocks nor the clock — and is never the \
         GRACE refusal, whose sentence is false for a wallet that has not met a server; got \
         {e:?}"
    );
    w.close().await.expect("close");
}

// ── The one place this file synthesises an endpoint identity ────────────────
//
// Adjudication repair. In its own `testing` module so that
// `extraction_policy::no_second_branch_comparison` reads it the way it reads
// `sync::testing::FakeChain`'s: a fake that must synthesise a `ServerIdentity`
// to be an honest endpoint at all, not a second staleness comparison. Nothing
// above this line names the branch field.
mod testing {
    use super::*;

    /// An honest MAINNET endpoint's identity at `tip`: the branch our compiled
    /// params compute there, so the per-pass consensus verdict is `Behind`,
    /// never `Unsupported` — a stale-build verdict would outrank every status
    /// this file measures (`UpToDateLimited` takes precedence).
    pub(super) fn mainnet_identity(tip: u64) -> ServerIdentity {
        identity_on(Network::Main, tip)
    }

    /// T0-1c E4: an honest TESTNET endpoint's identity at `tip` — the
    /// wrong-chain server, computed the same way for the other chain. Lives
    /// here for the same reason `mainnet_identity` does.
    pub(super) fn testnet_identity(tip: u64) -> ServerIdentity {
        identity_on(Network::Test, tip)
    }

    /// T0-1c-R2 (G4): an honest MAINNET endpoint that does not SAY its branch
    /// — the old or non-conforming server §6.3's grace exists for. Everything
    /// else is [`mainnet_identity`]'s. Lives here for the same reason the
    /// others do: this module is the one place the file names the field.
    pub(super) fn mainnet_identity_without_branch(tip: u64) -> ServerIdentity {
        ServerIdentity {
            consensus_branch_id: None,
            ..identity_on(Network::Main, tip)
        }
    }

    /// GRACE-1 (G-6's control): a MAINNET identity whose branch id is the
    /// bitwise complement of the one our params compute at `tip` — never equal
    /// to it, so the verdict is `Unsupported` for the branch's own reason and
    /// nothing else (the chain name and activation height stay honest, so the
    /// wrong-CHAIN guard does not fire first). Lives here for the reason the
    /// others do.
    pub(super) fn mainnet_identity_with_a_bogus_branch(tip: u64) -> ServerIdentity {
        let honest = identity_on(Network::Main, tip);
        ServerIdentity {
            consensus_branch_id: honest.consensus_branch_id.map(|b| !b),
            ..honest
        }
    }

    fn identity_on(network: Network, tip: u64) -> ServerIdentity {
        use zcash_protocol::consensus::{BlockHeight as ConsensusHeight, BranchId};
        ServerIdentity {
            chain_name: crate::checkpoints::chain_name(network).to_owned(),
            sapling_activation_height: u64::from(crate::checkpoints::activation_height(network)),
            consensus_branch_id: Some(u32::from(BranchId::for_height(
                &network.consensus(),
                ConsensusHeight::from_u32(u32::try_from(tip).unwrap_or(u32::MAX)),
            ))),
            block_height: tip,
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════
// GRACE-1 — the rows that need the P-G2 seam (§4p G-1, G-2, G-2b, G-3, G-4,
// G-7 and a planted case).
//
// Committed SEPARATELY from the rows above and NON-COMPILING at the contract
// commit: `reopen_with_clock` below is the ONE call site of the seam the
// contract names — `open_with_vault_and_seed_port`'s fourth argument and
// `sync::testing::ManualClock::at` — and neither exists there. Recorded as
// RED-BY-ABSENCE with the exact call; the adjudicator re-runs red-first with
// only the seam commit overlaid. Everything else in this module was
// compile-checked at the contract commit through a stub of that one function
// (uncommitted, in this file only — never a shim in the implementation).
//
// ONE row here was NOT written blind: G-7b
// (`a_capable_verdict_two_years_old_still_permits_without_a_new_pass`, at the
// end, beside the G-7 whose blindness it repairs) was added against the
// FOLDED implementation, out of the GRACE-1 adjudication
// (the GRACE-1 ruling §6 row 2, §4p-run owed row 3). It
// is the only row in this file that may name what the implementer built; its
// doc says which reads those are and why a blind reading could not have caught
// what it catches.
// ════════════════════════════════════════════════════════════════════════════
mod clock_seam {
    use super::*;

    /// The injected clock's reading at the capable pass every row starts from:
    /// an arbitrary fixed epoch (2027-01-15T08:00:00Z), so every offset below
    /// is a constant the artifact can quote and no row reads `SystemTime`.
    const G1_T0: u64 = 1_800_000_000;
    const HOUR: u64 = 3_600;
    /// "A day" as THIS MODULE prices its margins — and only its margins. Item
    /// 5 leaves the constant to the implementer: 24 h exactly and 1,152 × 75 s
    /// are BOTH 86,400, so every step below sits on the same side of it
    /// whichever is built, and `>` versus `≥` at the boundary cannot matter to
    /// a step an hour from it. Never handed to the mechanism.
    const G1_DAY: u64 = 24 * HOUR;
    /// Inside the day, twice — 12 h, then 23 h: each below a day, and so is the
    /// gap between them.
    const G1_INSIDE_EARLY: u64 = 12 * HOUR;
    const G1_INSIDE_LATE: u64 = 23 * HOUR;
    /// Past the day at 34 h — but only 11 h after [`G1_INSIDE_LATE`]: a capable
    /// time RENEWED by the Unknown pass at 23 h would still read inside here
    /// (G-10's "renewed on every Unknown pass" mutant is what this margin is
    /// for).
    const G1_PAST: u64 = 34 * HOUR;
    /// The contract's own number for "well past".
    const G1_FAR_PAST: u64 = 48 * HOUR;
    /// G-7: a clock two years wrong, either way.
    const G1_TWO_YEARS: u64 = 2 * 365 * G1_DAY;
    /// G-3: how far the honest old server's claim advances per pass.
    const G1_STEP: u64 = 100;

    /// THE seam (§4p P-G2), called by name and blind: re-open the wallet at
    /// `dir` with the crate's wall clock replaced by
    /// `sync::testing::ManualClock::at(secs)` — the fourth argument of
    /// `open_with_vault_and_seed_port`, `None` being the system clock in every
    /// `pub` constructor. Every clock change in this module is a `close()` and
    /// this call with a new reading (`at` is the one constructor the contract
    /// names), so each row is also a relaunch: the carried capable time — and
    /// the latch, if built as one — must be read cold at every gate G-13
    /// names, and a reading that lived only in a process would pass a
    /// per-process mutant. The ONE function that does not compile at the
    /// contract commit.
    async fn reopen_with_clock(dir: &Path, vault: &Arc<dyn KeychainPort>, secs: u64) -> Wallet {
        Wallet::open_with_vault_and_seed_port(
            cfg(dir, HOST_A),
            Arc::clone(vault),
            None,
            Some(Arc::new(sync::testing::ManualClock::at(secs))),
        )
        .await
        .expect("re-open through the P-G2 clock seam")
    }

    /// G-12: no absolute clock reading crosses in a published status — the
    /// capable timestamp is a SIGNING input and stays inside; only a REMAINING
    /// count or duration and a reason may cross.
    fn assert_carries_no_clock_reading(status: &SyncStatus, readings: &[u64], row: &str) {
        let rendered = format!("{status:?}");
        for r in readings {
            assert!(
                !rendered.contains(&r.to_string()),
                "{row} — G-12: the published status carries an absolute clock reading {r}: \
                 {rendered}"
            );
        }
    }

    /// G-7b: `SyncEnginePort::unknown_branch_grace` read off a snapshot the row
    /// already holds — the grace the SYNC SURFACE would carry, without running a
    /// pass to publish one.
    ///
    /// The production engine's method is `consensus_stamp::observe(conn,
    /// clock.now_unix())` followed by
    /// `verdict.and_then(ConsensusCompatibility::unknown_branch_grace)`, and
    /// `Wallet::consensus_status` is that SAME `observe` call on the SAME
    /// injected clock — so this is the value `SyncStatus::UpToDateUnverified`
    /// would be built from at this instant, and `None` is "no grace to show:
    /// the plain terminal". A row that runs no pass has no published status to
    /// read, which is the whole point of it; this is the closest reachable
    /// surface (the engine itself is private to `wallet.rs`).
    fn published_grace(
        cold: &crate::consensus::ConsensusStatusSnapshot,
    ) -> Option<crate::state::UnknownBranchGrace> {
        cold.verdict
            .as_ref()
            .and_then(crate::consensus::ConsensusCompatibility::unknown_branch_grace)
    }

    // ── GRACE-2 harness (§4v) ────────────────────────────────────────────────

    /// GRACE-2: the clock grace as the mechanism reads it (`constants.rs`), in
    /// this file's `u64` — the distance the §4v geometry moves the clock BACK
    /// by, so the cell is "a capable time a day later than the device clock".
    /// Read here, never asserted against: under the rule §4v asks for ANY
    /// capable time later than now expires, and the day is only what makes
    /// this the abstain × frozen-tip × a-day cell §4p-run review row 2 named.
    const GRACE_SECS: u64 = crate::constants::UNKNOWN_BRANCH_GRACE_SECS;

    /// THE seam (§4p P-G2) on a handle the row KEEPS. `ManualClock` is one
    /// shared `Arc<AtomicU64>` (`sync::testing`, read there), so after this
    /// call `clock.set(..)` moves the clock the OPEN wallet reads — backward
    /// too — with no relaunch, which is the §4v geometry (the device clock
    /// corrected downward after a capable pass, the wallet still running); and
    /// `close()` + this call again on the SAME handle is a relaunch on the
    /// same clock. [`reopen_with_clock`] mints a fresh clock per relaunch and
    /// can express neither.
    async fn reopen_on_clock(
        dir: &Path,
        vault: &Arc<dyn KeychainPort>,
        clock: &sync::testing::ManualClock,
    ) -> Wallet {
        Wallet::open_with_vault_and_seed_port(
            cfg(dir, HOST_A),
            Arc::clone(vault),
            None,
            Some(clock.port()),
        )
        .await
        .expect("re-open through the P-G2 clock seam on a shared clock")
    }

    /// The grace the SYNC SURFACE published with a pass:
    /// `SyncStatus::UpToDateUnverified`'s reading, `None` for any other status
    /// (no grace shown). Named, not distinguished — the variant has been in the
    /// tree since the GRACE-1 fold, and a §4v row is about WHICH reading it
    /// carries.
    fn surface_grace(s: &SyncStatus) -> Option<crate::state::UnknownBranchGrace> {
        match s {
            SyncStatus::UpToDateUnverified { grace, .. } => Some(*grace),
            _ => None,
        }
    }

    /// §4v's reading, exactly: the grace ENDED and the CLOCK ended it —
    /// `Ended { by: Clock, .. }`. Not `Running` (a countdown: the base's
    /// blocks-and-no-time), not `Blocks`, not `NeverConfirmed`.
    fn is_the_clock_ending(g: Option<crate::state::UnknownBranchGrace>) -> bool {
        matches!(
            g,
            Some(crate::state::UnknownBranchGrace::Ended {
                by: crate::state::GraceExpiry::Clock,
                ..
            })
        )
    }

    /// The send gate's refusal, if it is §4v's: `ConsensusGraceExpired { by:
    /// Clock, .. }` — the carrier by TYPE and the reason by name. The update
    /// refusal, a blocks or never-confirmed ending, and a permit are all
    /// `None`.
    fn refused_by_the_clock(g: &Gate) -> Option<&WalletError> {
        refused(g).filter(|e| {
            matches!(
                e,
                WalletError::ConsensusGraceExpired {
                    by: crate::state::GraceExpiry::Clock,
                    ..
                }
            )
        })
    }

    /// The parked row AGREES with the gate (§4v G2-2; G-13's third path): not
    /// a healthy pending send, and its `signing_block` line names the clock
    /// ending — [`parked_blockage`] renders that line as `signing_block:
    /// Some(GraceExpired { by: Clock })`.
    fn parked_names_the_clock_ending(reading: &[String]) -> bool {
        !parked_reads_healthy(reading)
            && reading
                .iter()
                .any(|f| f.starts_with("signing_block: Some(GraceExpired") && f.contains("Clock"))
    }

    /// §4v G2-3's "never 'about 24 more hours'": a rendering that carries a
    /// full day (or a day and an hour) as a remaining figure, in seconds or in
    /// minutes — what `elapsed = Some(0)` on a capable time of 0 renders
    /// (`secs_left: Some(86400)`), and what un-saturated arithmetic on a
    /// future time renders. G-4's own figures, kept.
    fn assert_carries_no_full_day(rendered: &str, row: &str) {
        for figure in [G1_DAY, G1_DAY + HOUR, G1_DAY / 60, (G1_DAY + HOUR) / 60] {
            assert!(
                !rendered.contains(&figure.to_string()),
                "{row} — the reading carries {figure}: a full day (or a day and an hour) of \
                 time remaining — 'about 24 more hours' on a time the wallet cannot trust: \
                 {rendered}"
            );
        }
    }

    /// The scanned wallet with its anchor at TIP whose last CAPABLE verdict was
    /// taken with the injected clock reading `secs`. Created under the system
    /// clock (`create_with_vault` takes no clock), closed, re-opened through
    /// the seam, then the account, the honest scan and the capable pass at TIP
    /// — [`scanned_wallet_at_tip`]'s preconditions asserted the same way — so
    /// the capable time the stamp carries is `secs` exactly.
    async fn scanned_wallet_capable_at(
        dir: &Path,
        vault: &Arc<dyn KeychainPort>,
        row: &str,
        secs: u64,
    ) -> (Wallet, Vec<u64>) {
        let anchor = TIP - R2_SCAN_DEPTH;
        let w = unprovisioned_wallet(dir, vault, HOST_A).await;
        w.close().await.expect("close");
        let w = reopen_with_clock(dir, vault, secs).await;
        let birthday = crate::account::birthday_from_treestate(anchor_tree_state(anchor))
            .expect("the empty-frontier anchor decodes");
        w.import_account(birthday).await.expect("import account");
        let honest = ProofEndpoint::honest();
        let receipt = honest.receipt();
        let served = honest.served_heights();
        let p = pass_over(&w, honest).await;
        record_g1(
            &format!("{row} honest pass at TIP (clock {secs})"),
            &p,
            &receipt,
        );
        let ranges = receipt.scanned_ranges();
        assert!(
            !ranges.is_empty(),
            "{row} — precondition: the honest pass SCANNED (≥ 1 block range asked)"
        );
        for (start, end) in &ranges {
            assert!(
                *start > anchor && *end <= TIP,
                "{row} — the honest pass scans only the gap ({anchor}, {TIP}]; got {start}..={end}"
            );
        }
        assert!(
            is_healthy_at_tip(&p.status),
            "{row} — precondition: an honest pass at TIP is UpToDate at TIP; got {}",
            describe(&p)
        );
        assert_eq!(
            w.scanned_tip().await.expect("scanned_tip").map(height_of),
            Some(TIP),
            "{row} — precondition: the wallet's own scanned height reads TIP"
        );
        let served = anchor_at_tip(&w, served, row).await;
        (w, served)
    }

    /// **G-1 — DEFECT.** A server that freezes its tip cannot hold the grace
    /// open.
    ///
    /// A capable pass at TIP with the clock at `T0` (branch reported); a queued
    /// send; then, each through a relaunch with the clock further on and the
    /// claim FROZEN at TIP with the branch omitted: `+12 h` permitted, `+23 h`
    /// permitted, `+34 h` REFUSED, `+48 h` refused. The refusal is a
    /// `WalletError` variant the contract commit does not have — never
    /// `NetworkUpgradeUnsupported` — and names its reason, the CLOCK; the
    /// parked row stops reading as a healthy pending send; the surface says the
    /// grace expired. The `+34 h` step is 11 h after `+23 h`: a capable time
    /// renewed by an Unknown pass (G-10's "not carried forward" mutant) would
    /// still read inside there, and the row reds.
    ///
    /// **At the contract commit (with the seam):** permitted at every step,
    /// forever — the block rule reads `claimed − anchor = 0` on a frozen claim
    /// (§4n-review row 1) and no clock is consulted; the parked row reads
    /// healthy; the surface is the same terminal as a capable pass.
    #[tokio::test]
    async fn a_server_that_freezes_its_tip_cannot_hold_the_grace_open() {
        const {
            assert!(
                G1_INSIDE_EARLY < G1_INSIDE_LATE && G1_INSIDE_LATE < G1_DAY,
                "premise: the inside steps are below a day"
            );
            assert!(
                G1_PAST > G1_DAY && G1_PAST - G1_INSIDE_LATE < G1_DAY && G1_FAR_PAST > G1_PAST,
                "premise: 34 h is past a day but only 11 h after the last inside step"
            );
        }
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = test_vault();
        let (w, served) = scanned_wallet_capable_at(dir.path(), &vault, "G-1", G1_T0).await;
        w.queue_send_uri(G1_SEND_URI)
            .await
            .expect("queueing is ungated");
        w.close().await.expect("close");
        let readings = [
            G1_T0,
            G1_T0 + G1_INSIDE_EARLY,
            G1_T0 + G1_INSIDE_LATE,
            G1_T0 + G1_PAST,
            G1_T0 + G1_FAR_PAST,
        ];

        let w = reopen_with_clock(dir.path(), &vault, G1_T0 + G1_INSIDE_EARLY).await;
        let (p12, s12) = unknown_pass(&w, 0, "G-1 clock +12 h, claim frozen at TIP").await;
        let g12 = send_gate(&w, "G-1 +12 h").await;
        w.close().await.expect("close");

        let w = reopen_with_clock(dir.path(), &vault, G1_T0 + G1_INSIDE_LATE).await;
        let (p23, s23) = unknown_pass(&w, 0, "G-1 clock +23 h, claim frozen at TIP").await;
        let g23 = send_gate(&w, "G-1 +23 h").await;
        w.close().await.expect("close");

        let w = reopen_with_clock(dir.path(), &vault, G1_T0 + G1_PAST).await;
        let (p34, s34) = unknown_pass(&w, 0, "G-1 clock +34 h, claim frozen at TIP").await;
        let g34 = send_gate(&w, "G-1 +34 h").await;
        let parked34 = parked_blockage(&w, "G-1 +34 h").await;
        w.close().await.expect("close");

        let w = reopen_with_clock(dir.path(), &vault, G1_T0 + G1_FAR_PAST).await;
        let (p48, s48) = unknown_pass(&w, 0, "G-1 clock +48 h, claim frozen at TIP").await;
        let g48 = send_gate(&w, "G-1 +48 h").await;
        let parked48 = parked_blockage(&w, "G-1 +48 h").await;
        println!(
            "[GRACE-1 G-1] +12 h: {} | +23 h: {} | +34 h: {}; parked {parked34:?}; surface {:?} \
             | +48 h: {}; parked {parked48:?}; surface {:?}",
            describe_gate(&g12),
            describe_gate(&g23),
            describe_gate(&g34),
            p34.status,
            describe_gate(&g48),
            p48.status
        );

        assert!(
            matches!(g12, Gate::Permitted(_)),
            "control inside: 12 h after the capable verdict, blocks frozen at 0 → permitted; got {}",
            describe_gate(&g12)
        );
        assert!(
            matches!(g23, Gate::Permitted(_)),
            "control inside: 23 h → still permitted (below a day under either constant); got {}",
            describe_gate(&g23)
        );
        let e34 = refused(&g34).unwrap_or_else(|| {
            panic!(
                "G-1: 34 h after the last CAPABLE verdict with the claim frozen at TIP, the send \
                 is REFUSED — the block rule reads 0 forever on a frozen claim (§4n-review row \
                 1), so the clock is the only thing that can end this grace, and the time it \
                 measures from is the last CAPABLE verdict's, not the last pass's (11 h ago). \
                 At the contract commit: permitted forever. got {}",
                describe_gate(&g34)
            )
        });
        assert!(
            is_minted_error(e34) && !is_upgrade_refusal(e34),
            "G-1: the refusal is distinguishable BY TYPE from NetworkUpgradeUnsupported — a \
             WalletError variant the contract commit lacks (§4p item 2): the old carrier \
             renders 'the network was upgraded, update the app', false here; got {e34:?}"
        );
        assert!(
            format!("{e34:?}").to_ascii_lowercase().contains("clock"),
            "G-1: the carrier names its reason — expired by CLOCK; got {e34:?}"
        );
        assert!(
            refused(&g48).is_some_and(|e| is_minted_error(e) && !is_upgrade_refusal(e)),
            "G-1: still refused at 48 h; got {}",
            describe_gate(&g48)
        );
        assert!(
            !parked_reads_healthy(&parked34) && !parked_reads_healthy(&parked48),
            "G-1: the parked row does not render as a healthy pending send while the drain \
             skips it; got {parked34:?} / {parked48:?}"
        );
        assert!(
            says_expired(&p34.status),
            "G-1: the sync surface says the grace expired — not the contract commit's \
             terminal; got {}",
            describe(&p34)
        );
        let all_served = [served, s12, s23, s34, s48].concat();
        for (label, st) in [
            ("+12 h", &p12.status),
            ("+23 h", &p23.status),
            ("+34 h", &p34.status),
            ("+48 h", &p48.status),
        ] {
            assert_carries_no_fixture_datum(st, &all_served, HOST_A, &format!("G-1 {label}"));
            assert_carries_no_clock_reading(st, &readings, &format!("G-1 {label}"));
        }
        w.close().await.expect("close");
    }

    /// **G-2 — CONTROL.** A clock set backward cannot extend the grace past the
    /// block rule. **Its first half was REWRITTEN at the GRACE-2 join
    /// (§4v); the second half is untouched and is what the row is kept for.**
    ///
    /// A capable pass at `T0`; a relaunch with the clock reading `T0 − 1 h` —
    /// BEFORE the capable time. First the claim `TIP + 500`, inside the block
    /// grace: **REFUSED, by CLOCK.** A capable time later than the device clock
    /// is a time the wallet cannot vouch for, and GRACE-2 makes that an EXPIRED
    /// clock rule rather than an abstention (the maintainer's decision, 2026-09-10):
    /// with a frozen-tip server the abstention left the grace with no expiry on
    /// either axis. Then the claim `TIP + 2,000`, past: still REFUSED, and the
    /// carrier still names **BLOCKS** — so this row now also pins the precedence
    /// GRACE-1's G-6 states, that a verdict with BOTH rules expired reports the
    /// block reason. The inside pass runs first so no latch question arises.
    ///
    /// **What it used to assert, and why the change is not a relaxation.** The
    /// first half read *"permitted — an untrusted timestamp abstains and the
    /// block rule decides"*, and was GREEN at the contract commit. The §4v
    /// contract listed this row as an untouched control, which was wrong: its
    /// permit half is labelled *G-4's permit half* in its own source and sits on
    /// exactly the cell GRACE-2 moves. The rewrite makes it refuse EARLIER than
    /// before, never later — the row still cannot pass by a clock buying grace,
    /// which is what its name promises. Found by the GRACE-2 implementer against
    /// its own contract, measured at the join, repaired here rather than filed
    /// (the contract's error, not a half's).
    #[tokio::test]
    async fn a_clock_set_backward_cannot_extend_the_grace_past_the_block_rule() {
        assert!(
            G1_BLOCKS_INSIDE + u64::from(REORG_MAX_BLOCKS) < GRACE_BLOCKS
                && G1_BLOCKS_PAST >= GRACE_BLOCKS + u64::from(REORG_MAX_BLOCKS),
            "premise: {G1_BLOCKS_INSIDE} is inside and {G1_BLOCKS_PAST} is past the \
             {GRACE_BLOCKS}-block grace by more than the reorg margin"
        );
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = test_vault();
        let (w, served) = scanned_wallet_capable_at(dir.path(), &vault, "G-2", G1_T0).await;
        w.queue_send_uri(G1_SEND_URI)
            .await
            .expect("queueing is ungated");
        w.close().await.expect("close");

        let w = reopen_with_clock(dir.path(), &vault, G1_T0 - HOUR).await;
        let (pi, si) = unknown_pass(
            &w,
            G1_BLOCKS_INSIDE,
            &format!("G-2 clock −1 h (before the capable time), claim TIP + {G1_BLOCKS_INSIDE}"),
        )
        .await;
        let gi = send_gate(&w, "G-2 inside").await;
        let (pp, sp) = unknown_pass(
            &w,
            G1_BLOCKS_PAST,
            &format!("G-2 clock −1 h, claim TIP + {G1_BLOCKS_PAST}"),
        )
        .await;
        let gp = send_gate(&w, "G-2 past").await;
        let parked = parked_blockage(&w, "G-2 past").await;
        println!(
            "[GRACE-1 G-2] inside: {} | past: {}; parked {parked:?}; surface {:?}",
            describe_gate(&gi),
            describe_gate(&gp),
            pp.status
        );

        let ei = refused(&gi).unwrap_or_else(|| {
            panic!(
                "G-2 (§4v): the clock reads BEFORE the capable time, so the capable time is one \
                 the wallet cannot vouch for → refused by the CLOCK even with the claim only \
                 {G1_BLOCKS_INSIDE} inside the block grace; got {}",
                describe_gate(&gi)
            )
        });
        assert!(
            format!("{ei:?}").contains("Clock"),
            "G-2 (§4v): an untrusted capable time ends the grace ON THE CLOCK AXIS — the block \
             rule has {G1_BLOCKS_INSIDE} of {GRACE_BLOCKS} still to run, so a blocks reason here \
             would mean the refusal came from somewhere this row cannot see; got {ei:?}"
        );
        let e = refused(&gp).unwrap_or_else(|| {
            panic!(
                "G-2 CONTROL: a claim {G1_BLOCKS_PAST} past the anchor is refused by BLOCKS \
                 whatever the clock says — 'either', not 'both'; got {}",
                describe_gate(&gp)
            )
        });
        assert!(
            !format!("{e:?}").to_ascii_lowercase().contains("clock"),
            "G-2: refused by blocks, never by a clock that reads before the capable time; got \
             {e:?}"
        );
        assert!(
            !parked_reads_healthy(&parked),
            "G-2: the parked row is not healthy under a blocks expiry; got {parked:?}"
        );
        let all_served = [served, si, sp].concat();
        for (label, st) in [("inside", &pi.status), ("past", &pp.status)] {
            assert_carries_no_fixture_datum(st, &all_served, HOST_A, &format!("G-2 {label}"));
            assert_carries_no_clock_reading(st, &[G1_T0, G1_T0 - HOUR], &format!("G-2 {label}"));
        }
        w.close().await.expect("close");
    }

    /// **G-2b — DEFECT.** A clock expiry is not undone by setting the clock
    /// back.
    ///
    /// G-1's geometry to `+34 h` (refused — asserted as the precondition, or
    /// the row cannot see what it is named for); then the clock set BACK to
    /// `+1 h`, inside the day, with the claim still frozen: STILL refused, on
    /// two passes (a per-read recomputation reads 1 h since the capable time
    /// and re-permits — the mutant this row exists for; the maintainer's "the
    /// clock only tightens, never extends"). One capable pass at `+1 h` clears
    /// it: permitted. Then, with the clock still at `+1 h`, the claim `TIP +
    /// 2,000`: refused by BLOCKS — a carrier that renders differently from the
    /// clock refusal and names blocks, not the clock (the two expiry reasons
    /// are distinguishable, which G-6 cannot show without the seam).
    ///
    /// The latch — or the monotone reading — is read across a relaunch here, by
    /// construction of every clock change in this module: G-13 says it is read
    /// on every path the stamp is read, and the stamp is read cold.
    ///
    /// **At the contract commit (with the seam):** the `+34 h` pass is
    /// permitted (no clock rule), so the precondition itself is what reds.
    #[tokio::test]
    async fn a_clock_expiry_is_not_undone_by_setting_the_clock_back() {
        const {
            assert!(
                HOUR < G1_DAY && G1_PAST > G1_DAY,
                "premise: +1 h is inside, +34 h is past"
            );
        }
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = test_vault();
        let (w, served) = scanned_wallet_capable_at(dir.path(), &vault, "G-2b", G1_T0).await;
        w.queue_send_uri(G1_SEND_URI)
            .await
            .expect("queueing is ungated");
        w.close().await.expect("close");

        let w = reopen_with_clock(dir.path(), &vault, G1_T0 + G1_PAST).await;
        let (p34, s34) = unknown_pass(&w, 0, "G-2b clock +34 h, claim frozen at TIP").await;
        let g34 = send_gate(&w, "G-2b +34 h").await;
        w.close().await.expect("close");

        let w = reopen_with_clock(dir.path(), &vault, G1_T0 + HOUR).await;
        let (p1, s1) = unknown_pass(&w, 0, "G-2b clock set BACK to +1 h, claim frozen").await;
        let g1 = send_gate(&w, "G-2b +1 h (set back)").await;
        let parked1 = parked_blockage(&w, "G-2b +1 h (set back)").await;
        let (p1b, s1b) = unknown_pass(&w, 0, "G-2b +1 h again, claim frozen").await;
        let g1b = send_gate(&w, "G-2b +1 h again").await;
        let (pc, sc) = capable_pass(&w, 0, "G-2b capable pass at +1 h").await;
        let gc = send_gate(&w, "G-2b after the capable pass").await;
        let parked_c = parked_blockage(&w, "G-2b after the capable pass").await;
        let (pb, sb) = unknown_pass(
            &w,
            G1_BLOCKS_PAST,
            &format!("G-2b clock +1 h, claim TIP + {G1_BLOCKS_PAST}"),
        )
        .await;
        let gb = send_gate(&w, "G-2b expired by blocks").await;
        println!(
            "[GRACE-1 G-2b] +34 h: {} | set back to +1 h: {}; parked {parked1:?}; surface {:?} \
             | again: {} | capable: {}; parked {parked_c:?} | blocks past: {}; surface {:?}",
            describe_gate(&g34),
            describe_gate(&g1),
            p1.status,
            describe_gate(&g1b),
            describe_gate(&gc),
            describe_gate(&gb),
            pb.status
        );

        let e34 = refused(&g34).unwrap_or_else(|| {
            panic!(
                "G-2b precondition (G-1's clause): expired by clock at +34 h; got {}",
                describe_gate(&g34)
            )
        });
        let e1 = refused(&g1).unwrap_or_else(|| {
            panic!(
                "G-2b: expired by clock at +34 h, then the clock set BACK to +1 h with the claim \
                 still frozen → STILL refused. The clock only tightens, never extends (the \
                 founder, 2026-09-10): a per-read recomputation reads 1 h since the capable time \
                 and re-permits — the mutant this row is for. got {}",
                describe_gate(&g1)
            )
        });
        assert!(
            is_minted_error(e1) && !is_upgrade_refusal(e1),
            "G-2b: …with the grace carrier, not the stale-build one; got {e1:?}"
        );
        assert!(
            refused(&g1b).is_some(),
            "G-2b: still refused on a second pass at the set-back reading; got {}",
            describe_gate(&g1b)
        );
        assert!(
            !parked_reads_healthy(&parked1),
            "G-2b: the parked row stays not-healthy at the set-back reading; got {parked1:?}"
        );
        assert!(
            says_expired(&p1.status) && says_expired(&p1b.status),
            "G-2b: the surface keeps saying expired at the set-back reading; got {} / {}",
            describe(&p1),
            describe(&p1b)
        );
        assert!(
            matches!(gc, Gate::Permitted(_)),
            "G-2b: one capable pass clears it — the latch is cleared only by a capable \
             verdict; got {}",
            describe_gate(&gc)
        );
        assert!(
            parked_c.contains(&"blocked_by_network_upgrade: false".to_owned()),
            "G-2b: the parked row is unblocked again after the capable pass; got {parked_c:?}"
        );
        assert!(
            !shows_grace(&pc.status) && is_at_or_above_terminal_at(&pc.status, TIP),
            "G-2b: a capable pass shows no grace; got {}",
            describe(&pc)
        );
        let eb = refused(&gb).unwrap_or_else(|| {
            panic!(
                "G-2b: after the clearing, a claim {G1_BLOCKS_PAST} past the anchor is refused \
                 by BLOCKS; got {}",
                describe_gate(&gb)
            )
        });
        assert!(
            is_minted_error(eb) && !is_upgrade_refusal(eb),
            "G-2b: the blocks refusal is the grace carrier too; got {eb:?}"
        );
        let (r34, rb) = (
            format!("{e34:?}").to_ascii_lowercase(),
            format!("{eb:?}").to_ascii_lowercase(),
        );
        assert!(
            r34 != rb && r34.contains("clock") && rb.contains("block") && !rb.contains("clock"),
            "G-2b: the two expiry reasons are distinguishable — the clock's names the clock, \
             the blocks' names blocks; got {e34:?} vs {eb:?}"
        );
        let all_served = [served, s34, s1, s1b, sc, sb].concat();
        let readings = [G1_T0, G1_T0 + G1_PAST, G1_T0 + HOUR];
        for (label, st) in [
            ("+34 h", &p34.status),
            ("set back", &p1.status),
            ("again", &p1b.status),
            ("capable", &pc.status),
            ("blocks", &pb.status),
        ] {
            assert_carries_no_fixture_datum(st, &all_served, HOST_A, &format!("G-2b {label}"));
            assert_carries_no_clock_reading(st, &readings, &format!("G-2b {label}"));
        }
        w.close().await.expect("close");
    }

    /// **G-3 — DEFECT.** A clock set forward ends the grace early, and a
    /// capable pass restores it.
    ///
    /// An honest old server — branch omitted, claim advancing 100 blocks a
    /// pass, far inside the block rule. `+1 h`, claim `TIP + 100`: permitted.
    /// The clock jumped to `+48 h`, claim `TIP + 200`: REFUSED (fail-closed)
    /// with the CLOCK reason — the next steps ("switch servers / check the
    /// device time") are the copy row's; the parked row not healthy; the
    /// surface says expired. One capable pass at `+48 h`, claim `TIP + 300`:
    /// permitted again, and the anchor moves to the clamp's ceiling `TIP +
    /// 100` (`grace_anchor`; asserted, it is what keeps the later claims
    /// inside the block rule). The recorded time is RENEWED: 23 h after the
    /// capable pass (`+71 h`), claim `TIP + 400`, permitted — a time not
    /// renewed would read 71 h; 34 h after it (`+82 h`), claim `TIP + 500`,
    /// refused by clock again — a renewal, not a permanent clearing.
    ///
    /// **At the contract commit (with the seam):** the `+48 h` pass is
    /// permitted (blocks 200 < 1,152, no clock rule).
    #[tokio::test]
    async fn a_clock_set_forward_ends_the_grace_early_and_a_capable_pass_restores_it() {
        assert!(
            5 * G1_STEP + u64::from(REORG_MAX_BLOCKS) < GRACE_BLOCKS,
            "premise: five steps of {G1_STEP} stay inside the {GRACE_BLOCKS}-block grace"
        );
        const {
            assert!(
                G1_INSIDE_LATE < G1_DAY && G1_PAST > G1_DAY,
                "premise: 23 h inside, 34 h past"
            );
        }
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = test_vault();
        let (w, served) = scanned_wallet_capable_at(dir.path(), &vault, "G-3", G1_T0).await;
        w.queue_send_uri(G1_SEND_URI)
            .await
            .expect("queueing is ungated");
        w.close().await.expect("close");
        let t_jump = G1_T0 + G1_FAR_PAST;
        let t_renewed_inside = t_jump + G1_INSIDE_LATE;
        let t_renewed_past = t_jump + G1_PAST;

        let w = reopen_with_clock(dir.path(), &vault, G1_T0 + HOUR).await;
        let (p1, s1) = unknown_pass(&w, G1_STEP, "G-3 clock +1 h, claim TIP + 100").await;
        let g1 = send_gate(&w, "G-3 +1 h").await;
        w.close().await.expect("close");

        let w = reopen_with_clock(dir.path(), &vault, t_jump).await;
        let (p2, s2) = unknown_pass(
            &w,
            2 * G1_STEP,
            "G-3 clock jumped to +48 h, claim TIP + 200",
        )
        .await;
        let g2 = send_gate(&w, "G-3 +48 h").await;
        let parked2 = parked_blockage(&w, "G-3 +48 h").await;
        let (pc, sc) = capable_pass(
            &w,
            3 * G1_STEP,
            "G-3 capable pass at +48 h, claim TIP + 300",
        )
        .await;
        let gc = send_gate(&w, "G-3 after the capable pass").await;
        let anchor = w
            .consensus_status()
            .await
            .expect("consensus_status")
            .grace_anchor_tip
            .map(height_of);
        w.close().await.expect("close");

        let w = reopen_with_clock(dir.path(), &vault, t_renewed_inside).await;
        let (p3, s3) = unknown_pass(
            &w,
            4 * G1_STEP,
            "G-3 clock +71 h (23 h after the renewal), claim TIP + 400",
        )
        .await;
        let g3 = send_gate(&w, "G-3 +71 h").await;
        w.close().await.expect("close");

        let w = reopen_with_clock(dir.path(), &vault, t_renewed_past).await;
        let (p4, s4) = unknown_pass(
            &w,
            5 * G1_STEP,
            "G-3 clock +82 h (34 h after the renewal), claim TIP + 500",
        )
        .await;
        let g4 = send_gate(&w, "G-3 +82 h").await;
        println!(
            "[GRACE-1 G-3] +1 h: {} | +48 h: {}; parked {parked2:?}; surface {:?} | capable: \
             {}; anchor {anchor:?} | +71 h: {} | +82 h: {}",
            describe_gate(&g1),
            describe_gate(&g2),
            p2.status,
            describe_gate(&gc),
            describe_gate(&g3),
            describe_gate(&g4)
        );

        assert!(
            matches!(g1, Gate::Permitted(_)),
            "control inside: 1 h and 100 blocks in → permitted; got {}",
            describe_gate(&g1)
        );
        let e2 = refused(&g2).unwrap_or_else(|| {
            panic!(
                "G-3: an honest old server (branch omitted, claim advancing) with the clock \
                 jumped 48 h forward → REFUSED, fail-closed, by the clock — 200 blocks is far \
                 inside the block rule, so at the contract commit this is permitted; got {}",
                describe_gate(&g2)
            )
        });
        assert!(
            is_minted_error(e2)
                && !is_upgrade_refusal(e2)
                && format!("{e2:?}").to_ascii_lowercase().contains("clock"),
            "G-3: the grace carrier, naming the CLOCK reason (the next steps 'switch servers / \
             check the device time' are the copy row's); got {e2:?}"
        );
        assert!(
            !parked_reads_healthy(&parked2),
            "G-3: the parked row is not healthy under the clock expiry; got {parked2:?}"
        );
        assert!(
            says_expired(&p2.status),
            "G-3: the surface says the grace expired; got {}",
            describe(&p2)
        );
        assert!(
            matches!(gc, Gate::Permitted(_)),
            "G-3: one capable pass → permitted again; got {}",
            describe_gate(&gc)
        );
        assert!(
            !shows_grace(&pc.status),
            "G-3: a capable pass shows no grace; got {}",
            describe(&pc)
        );
        assert_eq!(
            anchor,
            Some(TIP + u64::from(REORG_MAX_BLOCKS)),
            "G-3 precondition for the block arithmetic below: the capable pass at claim TIP + \
             {} anchors at the clamp's ceiling TIP + REORG_MAX_BLOCKS (`grace_anchor`)",
            3 * G1_STEP
        );
        assert!(
            matches!(g3, Gate::Permitted(_)),
            "G-3: the recorded time was RENEWED by the capable pass — 23 h after it, inside (a \
             time not renewed would read 71 h); got {}",
            describe_gate(&g3)
        );
        assert!(
            refused(&g4).is_some_and(|e| {
                is_minted_error(e) && format!("{e:?}").to_ascii_lowercase().contains("clock")
            }),
            "G-3: 34 h after the renewal → refused by clock again — a renewal, not a permanent \
             clearing; got {}",
            describe_gate(&g4)
        );
        let all_served = [served, s1, s2, sc, s3, s4].concat();
        let readings = [
            G1_T0,
            G1_T0 + HOUR,
            t_jump,
            t_renewed_inside,
            t_renewed_past,
        ];
        for (label, st) in [
            ("+1 h", &p1.status),
            ("+48 h", &p2.status),
            ("capable", &pc.status),
            ("+71 h", &p3.status),
            ("+82 h", &p4.status),
        ] {
            assert_carries_no_fixture_datum(st, &all_served, HOST_A, &format!("G-3 {label}"));
            assert_carries_no_clock_reading(st, &readings, &format!("G-3 {label}"));
        }
        w.close().await.expect("close");
    }

    /// **G2-4 — DEFECT, the G-4 rewrite (§4v).** A capable time in the future
    /// shows the clock ending.
    ///
    /// G-4's geometry under the rule §4v asks for. A capable pass at `T0`. The
    /// INSIDE case first, as the control: a relaunch at `T0 + 23 h`, claim `TIP
    /// + 500` — permitted, the surface `Running` with 652 blocks left and the
    /// clock's 3,600 s (the clock expires first; item 5's one constant). Then
    /// the FUTURE case: a relaunch at `T0 − 1 h`, the same claim — the recorded
    /// capable time is LATER than the device clock, a time the wallet cannot
    /// trust. GRACE-1 built that as an abstention (the clock rule stepped
    /// aside and 500 blocks inside the block rule decided: permitted, blocks
    /// and no time — G-4's reading); §4p-run review row 2 showed the
    /// abstention reopens the freeze (a frozen tip holds the block rule at zero,
    /// so a capable time taken on a clock that was ahead — RTC garbage at boot,
    /// a date later corrected — left the grace with NO expiry on either axis),
    /// and the maintainer ruled it an EXPIRED clock rule, fail-closed (2026-09-10,
    /// item 1). So: the send gate REFUSES `ConsensusGraceExpired { by: Clock
    /// }`, the surface and the cold read both carry `Ended { by: Clock,
    /// blocks_since_last_current: Some(500) }` — no blocks-left / time-left
    /// pair at all — and the copy is the clock ending's own ("switch servers,
    /// or check the device's date and time"): no new string. The two readings
    /// differ, as before; the future one carries no full day (G-4's figures,
    /// kept); no absolute reading crosses (G-12).
    ///
    /// The inside case runs FIRST because under the new rule the future case
    /// LATCHES (cleared only by a capable pass), and G-4 ran them the other way
    /// round — the one change to the geometry, so the control still reads
    /// inside.
    ///
    /// **At the base (`753e1b45`, the OLD rule):** the future case is permitted
    /// with `Running { blocks_left: 652, secs_left: None }` — G-4's own green —
    /// so the refusal clause reds on the permit.
    #[tokio::test]
    async fn a_capable_time_in_the_future_shows_the_clock_ending() {
        assert!(
            G1_BLOCKS_INSIDE + u64::from(REORG_MAX_BLOCKS) < GRACE_BLOCKS
                && (GRACE_BLOCKS - G1_BLOCKS_INSIDE) * 75 > G1_DAY - G1_INSIDE_LATE,
            "premise: {G1_BLOCKS_INSIDE} blocks is inside, and at +23 h the clock expires \
             before the blocks do"
        );
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = test_vault();
        let (w, served) = scanned_wallet_capable_at(dir.path(), &vault, "G2-4", G1_T0).await;
        w.close().await.expect("close");

        let w = reopen_with_clock(dir.path(), &vault, G1_T0 + G1_INSIDE_LATE).await;
        let (pi, si) = unknown_pass(
            &w,
            G1_BLOCKS_INSIDE,
            &format!("G2-4 clock +23 h, claim TIP + {G1_BLOCKS_INSIDE}"),
        )
        .await;
        let gi = send_gate(&w, "G2-4 inside").await;
        w.close().await.expect("close");

        let w = reopen_with_clock(dir.path(), &vault, G1_T0 - HOUR).await;
        let (pf, sf) = unknown_pass(
            &w,
            G1_BLOCKS_INSIDE,
            &format!("G2-4 clock 1 h BEFORE the capable time, claim TIP + {G1_BLOCKS_INSIDE}"),
        )
        .await;
        let gf = send_gate(&w, "G2-4 future").await;
        let cold_f = w.consensus_status().await.expect("consensus_status");
        let rf = format!("{:?}", pf.status);
        let ri = format!("{:?}", pi.status);
        println!(
            "[GRACE-2 G2-4] inside: {}; surface {ri} | future: {}; surface {rf}; cold {:?}",
            describe_gate(&gi),
            describe_gate(&gf),
            published_grace(&cold_f)
        );

        assert!(
            matches!(gi, Gate::Permitted(_)),
            "G2-4 control (inside): 23 h after the capable verdict and 500 blocks in → \
             permitted; got {}",
            describe_gate(&gi)
        );
        let blocks_left = u32::try_from(GRACE_BLOCKS - G1_BLOCKS_INSIDE).expect("fits");
        let clock_left = u32::try_from(GRACE_SECS - G1_INSIDE_LATE).expect("fits");
        assert_eq!(
            surface_grace(&pi.status),
            Some(crate::state::UnknownBranchGrace::Running {
                blocks_left,
                secs_left: Some(clock_left),
            }),
            "G2-4 control (inside): a countdown — {blocks_left} blocks left and the clock's \
             {clock_left} s, the rule that expires first; got {ri}"
        );
        let e_f = refused_by_the_clock(&gf).unwrap_or_else(|| {
            panic!(
                "G2-4: the recorded capable time is 1 h LATER than the device clock and the \
                 claim is 500 blocks inside the block rule → the send gate REFUSES \
                 `ConsensusGraceExpired {{ by: Clock }}`: a capable time later than now is a \
                 time the wallet cannot trust, and §4v (the founder, 2026-09-10) makes it an \
                 EXPIRED clock rule — fail-closed — not an abstention that lets a frozen tip \
                 hold the block rule at zero forever. At the base (the OLD rule): permitted, \
                 `Running {{ blocks_left: 652, secs_left: None }}` — G-4's own green. got {}",
                describe_gate(&gf)
            )
        });
        assert!(
            is_the_clock_ending(surface_grace(&pf.status)),
            "G2-4: the surface shows the CLOCK ENDING — `UpToDateUnverified` with `Ended {{ \
             by: Clock, .. }}`, no blocks-left / time-left pair; got {rf}"
        );
        assert_eq!(
            surface_grace(&pf.status),
            Some(crate::state::UnknownBranchGrace::Ended {
                by: crate::state::GraceExpiry::Clock,
                blocks_since_last_current: Some(u32::try_from(G1_BLOCKS_INSIDE).expect("fits")),
            }),
            "G2-4: …carrying the {G1_BLOCKS_INSIDE} blocks the judged height advanced (the \
             count the copy names), and nothing else; got {rf}"
        );
        assert!(
            is_the_clock_ending(published_grace(&cold_f)),
            "G2-4: the cold read a host renders agrees with the surface — one reader \
             (`observe`), one clock; got {cold_f:?} beside the gate's {e_f:?}"
        );
        assert_ne!(
            rf, ri,
            "G2-4: the same 500 blocks read differently — 23 h in the past a countdown, 1 h in \
             the future the clock ending"
        );
        assert_carries_no_full_day(&rf, "G2-4 future");
        let all_served = [served, si, sf].concat();
        let readings = [G1_T0, G1_T0 - HOUR, G1_T0 + G1_INSIDE_LATE];
        for (label, st) in [("inside", &pi.status), ("future", &pf.status)] {
            assert_carries_no_fixture_datum(st, &all_served, HOST_A, &format!("G2-4 {label}"));
            assert_carries_no_clock_reading(st, &readings, &format!("G2-4 {label}"));
        }
        w.close().await.expect("close");
    }

    /// **G-7 — CONTROL.** A server that reports the branch never consults the
    /// clock.
    ///
    /// A capable pass at `T0`; a queued send; a relaunch with the clock two
    /// years AHEAD and a capable pass (branch reported): permitted, the surface
    /// the at-or-above terminal with no grace reading, the parked row unblocked
    /// and nothing in its reading naming the clock or an expiry. Then two years
    /// BEHIND the (now renewed) capable time — a timestamp in the future by two
    /// years — the same. GREEN at the contract commit with the seam; it must
    /// stay green.
    ///
    /// **What this row does NOT see (corrected; measured at the GRACE-1
    /// adjudication, the GRACE-1 ruling's VERDICT row 6,
    /// §4p-run owed row 3).** This doc used to end "a clock rule that fires on a
    /// capable verdict, or on an untrusted timestamp, reds here". The FIRST half
    /// was false, and the mechanism is the renewal: every step above runs a
    /// capable PASS at the reading it then gates on, and a capable verdict
    /// RENEWS the recorded capable time at that very clock — so `now −
    /// capable_at` is 0 at each gate, and a clock rule mis-scoped onto a capable
    /// verdict has nothing to fire on. M-t (`consensus_stamp::observe` turning a
    /// `Current`/`Behind` row into an expired `Unknown` once `now − capable_at ≥
    /// UNKNOWN_BRANCH_GRACE_SECS`) left this row — and, as the bed then stood,
    /// every other row in it — green: 50 passed / 0 failed, measured at the
    /// adjudication on `9ab88dec`, before the row below existed. The geometry
    /// that sees it is a capable pass, then a relaunch far ahead on the clock
    /// with NO pass, then the gate:
    /// [`a_capable_verdict_two_years_old_still_permits_without_a_new_pass`]
    /// (G-7b), below.
    ///
    /// The SECOND half stands, and is this row's own: the two-years-BEHIND leg
    /// is the untrusted timestamp (a recorded capable time later than now), and
    /// a rule that refused on it would red here. Otherwise this is a CONTROL
    /// with no killer of its own — as the mutant register records it.
    #[tokio::test]
    async fn a_server_that_reports_the_branch_never_consults_the_clock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = test_vault();
        let (w, served) = scanned_wallet_capable_at(dir.path(), &vault, "G-7", G1_T0).await;
        w.queue_send_uri(G1_SEND_URI)
            .await
            .expect("queueing is ungated");
        w.close().await.expect("close");
        let ahead = G1_T0 + G1_TWO_YEARS;
        let behind = G1_T0 - G1_TWO_YEARS;

        let w = reopen_with_clock(dir.path(), &vault, ahead).await;
        let (pa, sa) = capable_pass(&w, 0, "G-7 clock two years AHEAD, branch reported").await;
        let ga = send_gate(&w, "G-7 ahead").await;
        let parked_a = parked_blockage(&w, "G-7 ahead").await;
        w.close().await.expect("close");

        let w = reopen_with_clock(dir.path(), &vault, behind).await;
        let (pb, sb) = capable_pass(&w, 0, "G-7 clock two years BEHIND, branch reported").await;
        let gb = send_gate(&w, "G-7 behind").await;
        let parked_b = parked_blockage(&w, "G-7 behind").await;
        println!(
            "[GRACE-1 G-7] ahead: {}; parked {parked_a:?}; surface {:?} | behind: {}; parked \
             {parked_b:?}; surface {:?}",
            describe_gate(&ga),
            pa.status,
            describe_gate(&gb),
            pb.status
        );

        for (label, g, p, parked) in [
            ("ahead", &ga, &pa, &parked_a),
            ("behind", &gb, &pb, &parked_b),
        ] {
            assert!(
                matches!(g, Gate::Permitted(_)),
                "G-7 {label}: a capable verdict permits whatever the clock says; got {}",
                describe_gate(g)
            );
            assert!(
                !shows_grace(&p.status) && is_at_or_above_terminal_at(&p.status, TIP),
                "G-7 {label}: no grace surface on a capable pass; got {}",
                describe(p)
            );
            assert!(
                parked.contains(&"blocked_by_network_upgrade: false".to_owned())
                    && !parked.iter().any(|f| {
                        let l = f.to_ascii_lowercase();
                        l.contains("clock") || l.contains("expired")
                    }),
                "G-7 {label}: the parked row is unblocked and names no clock reason; got {parked:?}"
            );
        }
        let all_served = [served, sa, sb].concat();
        for (label, st) in [("ahead", &pa.status), ("behind", &pb.status)] {
            assert_carries_no_fixture_datum(st, &all_served, HOST_A, &format!("G-7 {label}"));
            assert_carries_no_clock_reading(st, &[G1_T0, ahead, behind], &format!("G-7 {label}"));
        }
        w.close().await.expect("close");
    }

    /// **G-7b — CONTROL, and the row G-7 could not be.** A capable verdict two
    /// years old still permits, with no new pass.
    ///
    /// Owed out of the GRACE-1 adjudication (§4p-run owed row 3;
    /// the GRACE-1 ruling §6 row 2). Its sibling
    /// [`a_server_that_reports_the_branch_never_consults_the_clock`] runs a
    /// capable PASS at each clock reading before it gates, and a capable verdict
    /// RENEWS the recorded capable time at that very clock — so the elapsed time
    /// its gate consults is 0, and a clock rule mis-scoped onto a capable
    /// verdict is invisible to it. **The renewal is exactly what this row
    /// removes.**
    ///
    /// 1. A capable pass at `T0` (branch reported) and a queued send: the gate
    ///    permits and there is no grace — the preconditions, asserted.
    /// 2. `close()`, then a relaunch with the clock at `T0 + 2 years` and **no
    ///    pass at all**: no `controller_over`, no `once()`, nothing that could
    ///    renew the capable time. Only the gate, the cold read and the parked
    ///    list are asked. The elapsed reading is asserted to be the full two
    ///    years — the ANTI-VACUITY clause this row exists to carry, because
    ///    without it the row passes on a clock that never moved, which is
    ///    precisely how G-7 went blind.
    /// 3. The send gate PERMITS, through the shipped propose path
    ///    ([`send_gate`] → `signing_permit` → `granted`), and the verdict read
    ///    back is still the capable one. A capable verdict never consults the
    ///    clock, at ANY elapsed time.
    /// 4. The surface shows NO grace: [`published_grace`] — the engine port's
    ///    own body over the same cold `observe` read — is `None`, and the queued
    ///    row still reads as an ordinary pending send. The three paths §4p G-13
    ///    names (gate, surface, parked flag), all read cold.
    /// 5. One more capable pass at `T0 + 2 years`: still permitted, still the
    ///    at-or-above terminal with no grace reading, the parked row still
    ///    healthy.
    ///
    /// **The killer it exists for:** M-t — `consensus_stamp::observe` turning a
    /// `Current`/`Behind` row into an expired `Unknown` once `now − capable_at ≥
    /// UNKNOWN_BRANCH_GRACE_SECS`. It SURVIVED the entire bed for the
    /// renewal reason above (50 passed / 0 failed, on `9ab88dec`). Measured
    /// again with this row present, `cargo test -p zec-wallet-core --lib
    /// -- 'degraded_pool_proof::clock_seam::'`: **7 passed / 1 failed, the one
    /// being this row** — the gate clause below, `refused:
    /// ConsensusGraceExpired { by: Clock, blocks_since_last_current: Some(0) } /
    /// code RW-SYNC-003`. Steps 3, 4 and 5 each red under it; G-7 stays green,
    /// which is the finding recorded. **M-a** (the clock rule deleted from
    /// `permits_signing`) leaves this row GREEN — measured 4 passed / 4
    /// failed on the same leg, this row among the four green with G-2, G-4 and
    /// G-7. It must: a control does not move when the rule it is named after is
    /// removed — that is its shape, not a gap in it.
    ///
    /// **Not blind.** Unlike every row above, this one was written against the
    /// folded implementation, so it names two built things: the cold-read
    /// snapshot's `grace_clock.elapsed_secs` (step 2 — there is no other way to
    /// prove the geometry is real) and `unknown_branch_grace` (step 4). Both are
    /// readings a host may take; neither is a signing input.
    #[tokio::test]
    async fn a_capable_verdict_two_years_old_still_permits_without_a_new_pass() {
        const {
            assert!(
                G1_TWO_YEARS > G1_DAY && G1_TWO_YEARS > G1_FAR_PAST,
                "premise: two years is past a day — past any constant the clock rule could be \
                 built on, so this row does not depend on which one was"
            );
        }
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = test_vault();
        let (w, served) = scanned_wallet_capable_at(dir.path(), &vault, "G-7b", G1_T0).await;
        w.queue_send_uri(G1_SEND_URI)
            .await
            .expect("queueing is ungated");
        let g0 = send_gate(&w, "G-7b T0").await;
        let cold0 = w.consensus_status().await.expect("consensus_status");
        w.close().await.expect("close");
        let far = G1_T0 + G1_TWO_YEARS;

        // NO PASS from here on, and none is ever run at `far` before the gate is
        // asked: the capable time recorded at T0 is two years stale and nothing
        // renews it. That is the whole geometry — G-7's capable pass at the far
        // reading is what hides the defect.
        let w = reopen_with_clock(dir.path(), &vault, far).await;
        let cold = w.consensus_status().await.expect("consensus_status");
        let g_far = send_gate(&w, "G-7b +2 y, no pass").await;
        let parked_far = parked_blockage(&w, "G-7b +2 y, no pass").await;

        // …and only NOW a capable pass, at the same two-years-on reading.
        let (pc, sc) = capable_pass(&w, 0, "G-7b capable pass at +2 y").await;
        let g_after = send_gate(&w, "G-7b after the capable pass at +2 y").await;
        let parked_after = parked_blockage(&w, "G-7b after the capable pass at +2 y").await;
        let cold_after = w.consensus_status().await.expect("consensus_status");
        println!(
            "[GRACE-1 G-7b] T0: {}; grace {:?} | +2 y NO pass: {}; grace {:?}; parked \
             {parked_far:?}; cold {cold:?} | capable at +2 y: {}; grace {:?}; parked \
             {parked_after:?}; surface {:?}",
            describe_gate(&g0),
            published_grace(&cold0),
            describe_gate(&g_far),
            published_grace(&cold),
            describe_gate(&g_after),
            published_grace(&cold_after),
            pc.status
        );

        assert!(
            matches!(g0, Gate::Permitted(_)),
            "G-7b precondition: the capable verdict at T0 permits; got {}",
            describe_gate(&g0)
        );
        assert_eq!(
            published_grace(&cold0),
            None,
            "G-7b precondition: a capable verdict is not on a grace at all; got {cold0:?}"
        );
        assert!(
            cold.grace_clock
                .elapsed_secs
                .is_some_and(|s| s >= G1_TWO_YEARS),
            "G-7b ANTI-VACUITY: the elapsed time the gate below consults is the full two years \
             since the capable verdict — no pass has renewed it. A row whose clock never moved \
             cannot see a rule mis-scoped onto a capable verdict; that is the S264 finding this \
             row exists for. got {:?}",
            cold.grace_clock
        );
        // The money path first: this is the claim the row is FOR.
        assert!(
            matches!(g_far, Gate::Permitted(_)),
            "G-7b: a capable verdict NEVER consults the clock, at any elapsed time — two years \
             after it, with no pass since, the send gate PERMITS. M-t (`observe` turning a \
             Current/Behind row into an expired Unknown at a day's elapsed) refuses here and is \
             invisible to every other row in this file, G-7 included, because each of them \
             renews the capable time at the clock it then reads. got {}",
            describe_gate(&g_far)
        );
        assert!(
            cold.verdict.is_some_and(|v| v.is_signing_capable_check()),
            "G-7b: two years on, the verdict read back out of the stamp is still the CAPABLE \
             one — a clock rule scoped onto a capable verdict rewrites it on the way out; got \
             {:?}",
            cold.verdict
        );
        assert_eq!(
            published_grace(&cold),
            None,
            "G-7b: …and there is no grace to SHOW either — not a countdown, not an expiry. The \
             §6.3 grace governs `Unknown` only; a server that reports its branch is on no \
             grace however old the verdict is (the age is the sync surface's business, not the \
             grace's). got {cold:?}"
        );
        assert!(
            parked_reads_healthy(&parked_far),
            "G-7b: the queued row still reads as an ordinary pending send — the third path the \
             stamp is read on (G-13), asked cold with no pass; got {parked_far:?}"
        );
        assert!(
            matches!(g_after, Gate::Permitted(_)),
            "G-7b: one more capable pass at the same two-years-on reading changes nothing — \
             still permitted; got {}",
            describe_gate(&g_after)
        );
        assert!(
            !shows_grace(&pc.status) && is_at_or_above_terminal_at(&pc.status, TIP),
            "G-7b: that pass publishes the at-or-above terminal with no grace reading; got {}",
            describe(&pc)
        );
        assert!(
            parked_reads_healthy(&parked_after),
            "G-7b: and the parked row is still an ordinary pending send; got {parked_after:?}"
        );
        assert_eq!(
            published_grace(&cold_after),
            None,
            "G-7b: and still no grace at the cold surface; got {cold_after:?}"
        );
        let all_served = [served, sc].concat();
        assert_carries_no_fixture_datum(&pc.status, &all_served, HOST_A, "G-7b capable at +2 y");
        assert_carries_no_clock_reading(&pc.status, &[G1_T0, far], "G-7b capable at +2 y");
        w.close().await.expect("close");
    }

    /// **PLANTED — DEFECT.** A send a day after the last pass is refused
    /// without a new pass.
    ///
    /// The floor's clock rows (G-1, G-2b, G-3) each run a PASS at the expired
    /// reading before asking the gate, so a clock rule that runs only inside
    /// `evaluate_consensus` and persists an "expired" bit on the stamp passes
    /// every one of them. §4p's mechanism section names the case it misses —
    /// "a send that happens a day after the last pass" — and G-13 says where
    /// the clock must be read; this row is that case. A capable pass at `T0`;
    /// a queued send; a relaunch at `+1 h` with one Unknown pass, frozen claim
    /// — permitted, and the stamp as written says so; a relaunch at `+23 h`
    /// with NO pass — the send is permitted (control); a relaunch at `+48 h`
    /// with NO pass — no controller is built, no `once()` runs — the send is
    /// REFUSED with the clock reason, and the parked-row flag reads the clock
    /// too.
    ///
    /// **PREDICTION:** the evaluate-only implementation reds HERE and nowhere
    /// else in this file. Also red at the contract commit (with the seam), for
    /// G-1's reason.
    #[tokio::test]
    async fn a_send_a_day_after_the_last_pass_is_refused_without_a_new_pass() {
        const {
            assert!(
                HOUR < G1_DAY && G1_INSIDE_LATE < G1_DAY && G1_FAR_PAST > G1_DAY,
                "premise: +1 h and +23 h inside, +48 h past"
            );
        }
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = test_vault();
        let (w, _served) =
            scanned_wallet_capable_at(dir.path(), &vault, "PLANT day-after", G1_T0).await;
        w.queue_send_uri(G1_SEND_URI)
            .await
            .expect("queueing is ungated");
        w.close().await.expect("close");

        let w = reopen_with_clock(dir.path(), &vault, G1_T0 + HOUR).await;
        let (_p1, _s1) = unknown_pass(
            &w,
            0,
            "PLANT day-after: clock +1 h, claim frozen (the LAST pass)",
        )
        .await;
        let g1 = send_gate(&w, "PLANT day-after +1 h").await;
        w.close().await.expect("close");

        // No pass from here on: only the gate and the parked list are asked.
        let w = reopen_with_clock(dir.path(), &vault, G1_T0 + G1_INSIDE_LATE).await;
        let g23 = send_gate(&w, "PLANT day-after +23 h, no pass").await;
        let parked23 = parked_blockage(&w, "PLANT day-after +23 h").await;
        w.close().await.expect("close");

        let w = reopen_with_clock(dir.path(), &vault, G1_T0 + G1_FAR_PAST).await;
        let g48 = send_gate(&w, "PLANT day-after +48 h, no pass").await;
        let parked48 = parked_blockage(&w, "PLANT day-after +48 h").await;
        println!(
            "[GRACE-1 PLANT day-after] +1 h (after the last pass): {} | +23 h, no pass: {}; \
             parked {parked23:?} | +48 h, no pass: {}; parked {parked48:?}",
            describe_gate(&g1),
            describe_gate(&g23),
            describe_gate(&g48)
        );

        assert!(
            matches!(g1, Gate::Permitted(_)),
            "precondition: the last pass left a stamp that permits; got {}",
            describe_gate(&g1)
        );
        assert!(
            matches!(g23, Gate::Permitted(_))
                && parked23.contains(&"blocked_by_network_upgrade: false".to_owned()),
            "control: 23 h after the capable verdict, with no pass, still permitted and the \
             parked row unblocked; got {} / {parked23:?}",
            describe_gate(&g23)
        );
        let e = refused(&g48).unwrap_or_else(|| {
            panic!(
                "PLANT: 48 h after the capable verdict — 47 h after the LAST PASS, which left a \
                 stamp that said 'permitted' — a send with NO new pass is REFUSED: the send gate \
                 reads the CLOCK at send time (G-13), not a bit the last pass wrote. PREDICTION: \
                 a clock rule that runs only inside evaluate_consensus passes G-1, G-2b and G-3 \
                 and misses here. got {}",
                describe_gate(&g48)
            )
        });
        assert!(
            is_minted_error(e)
                && !is_upgrade_refusal(e)
                && format!("{e:?}").to_ascii_lowercase().contains("clock"),
            "PLANT: the grace carrier, naming the clock; got {e:?}"
        );
        assert!(
            !parked_reads_healthy(&parked48),
            "PLANT: the parked-row flag reads the clock too (G-13's third path), with no pass; \
             got {parked48:?}"
        );
        w.close().await.expect("close");
    }

    // ── GRACE-2 rows (§4v) ───────────────────────────────────────────────────

    /// **G2-2 — DEFECT (§4v), the blind twin of the implementer's G2-1 through
    /// the shipped path.** A capable time in the future refuses until a
    /// capable pass.
    ///
    /// §4p-run review row 2's cell, built as a row: a capable pass at `T0`
    /// (G-1's first step) and a queued send; then — the wallet still OPEN, on a
    /// shared clock — the device clock moved BACK a day, to `T0 −
    /// UNKNOWN_BRANCH_GRACE_SECS`, and two passes over a server that OMITS the
    /// branch with its tip FROZEN at TIP. The block rule reads zero (a frozen
    /// claim, §4n-review row 1); the recorded capable time is now LATER than
    /// the device clock. GRACE-1 read that as "untrusted — the clock rule
    /// abstains, blocks decide", and blocks never decide on a frozen tip: the
    /// grace had NO expiry on either axis, forever, until the one thing the
    /// adversary withholds (a capable pass). §4v: a capable time later than the
    /// device clock is an EXPIRED clock rule, fail-closed.
    ///
    /// 1. On a cold read after those passes — `signing_permit` through the real
    ///    propose gate — the send is REFUSED `ConsensusGraceExpired { by: Clock
    ///    }`, by type; the parked row agrees (`signing_block` names the clock
    ///    ending — G-13's third path); each pass published
    ///    `UpToDateUnverified` with `Ended { by: Clock }`; the cold read's grace
    ///    is the same and its `elapsed_secs` is `None` (no trustworthy elapsed
    ///    — never a number the clock cannot vouch for).
    /// 2. A relaunch — `close()`, then `open()` on the SAME clock, still a day
    ///    behind — with NO pass: still refused, the parked row still blocked,
    ///    the cold read still the clock ending. What this clause adds: the
    ///    refusal is read from the DURABLE row against the clock at the read
    ///    (`observe`, G-13), not from a bit a process computed once and kept —
    ///    a per-process reading, or one taken only inside the pass, re-permits
    ///    here.
    /// 3. ONE capable pass (branch reported) at that same clock: permitted; the
    ///    recorded time re-taken from the clock as it reads now
    ///    (`grace_clock.elapsed_secs == Some(0)`); the parked row healthy; the
    ///    at-or-above terminal with no grace.
    ///
    /// Preconditions asserted: the capable verdict at `T0` permits and reads
    /// `elapsed_secs == Some(0)` on that clock (the port, not the system
    /// clock); a relaunch at `T0` still permits.
    ///
    /// **At the base (`753e1b45`, the OLD rule):** step 1 is PERMITTED with
    /// `Running { blocks_left: 1152, secs_left: None }` — G-4's permit half,
    /// blocks and no time — so the refusal clause reds on the permit.
    ///
    /// **Self-mutant SM-1 (§4v anti-vacuity, measured at the base):** step
    /// 2's relaunch dropped — no `close()`, no re-open on the same clock, the
    /// still-open wallet answering the reads step 2 takes cold. The row still
    /// reds, on the SAME money-path clause and with the same printed permit,
    /// because at the base nothing ever reaches step 2. So the relaunch clause
    /// buys nothing HERE and everything against a built rule: it is the only
    /// clause in this row that separates a refusal read from the durable row
    /// against the clock at the read (`observe`, G-13) from a bit one process
    /// computed inside a pass and kept. It is graded against the
    /// implementation, never against the base.
    #[tokio::test]
    async fn controller_over_the_production_engine_a_capable_time_in_the_future_refuses_until_a_capable_pass()
     {
        const {
            assert!(
                G1_T0 > GRACE_SECS,
                "premise: the moved-back clock is still after the epoch — this row is not \
                 G2-3's pre-epoch cell"
            );
        }
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = test_vault();
        let (w, served) = scanned_wallet_capable_at(dir.path(), &vault, "G2-2", G1_T0).await;
        w.queue_send_uri(G1_SEND_URI)
            .await
            .expect("queueing is ungated");
        let g0 = send_gate(&w, "G2-2 at T0").await;
        let cold0 = w.consensus_status().await.expect("consensus_status");
        w.close().await.expect("close");

        let clock = sync::testing::ManualClock::at(G1_T0);
        let w = reopen_on_clock(dir.path(), &vault, &clock).await;
        let g_relaunch0 = send_gate(&w, "G2-2 relaunch at T0, no pass").await;
        // The device clock corrected DOWNWARD by a day with the wallet open:
        // the capable time recorded at T0 is now a day in the future.
        clock.set(G1_T0 - GRACE_SECS);
        let (p1, s1) = unknown_pass(&w, 0, "G2-2 clock T0 − a day, claim frozen at TIP").await;
        let (p2, s2) = unknown_pass(&w, 0, "G2-2 clock T0 − a day again, claim frozen").await;
        let g_cold = send_gate(&w, "G2-2 cold read after the frozen passes").await;
        let parked_cold = parked_blockage(&w, "G2-2 cold read after the frozen passes").await;
        let cold = w.consensus_status().await.expect("consensus_status");
        w.close().await.expect("close");

        // The relaunch: the same clock, still a day behind, and NO pass.
        let w = reopen_on_clock(dir.path(), &vault, &clock).await;
        let g_relaunch = send_gate(&w, "G2-2 relaunch a day behind, no pass").await;
        let parked_relaunch = parked_blockage(&w, "G2-2 relaunch a day behind, no pass").await;
        let cold_relaunch = w.consensus_status().await.expect("consensus_status");

        let (pc, sc) = capable_pass(&w, 0, "G2-2 one capable pass, clock still a day behind").await;
        let g_after = send_gate(&w, "G2-2 after the capable pass").await;
        let parked_after = parked_blockage(&w, "G2-2 after the capable pass").await;
        let cold_after = w.consensus_status().await.expect("consensus_status");
        println!(
            "[GRACE-2 G2-2] T0: {}; clock {:?} | relaunch at T0: {} | a day behind, frozen: {}; \
             parked {parked_cold:?}; surface {:?} / {:?}; cold {:?} / clock {:?} | relaunch, no \
             pass: {}; parked {parked_relaunch:?}; cold {:?} | capable: {}; parked \
             {parked_after:?}; surface {:?}; clock {:?}",
            describe_gate(&g0),
            cold0.grace_clock,
            describe_gate(&g_relaunch0),
            describe_gate(&g_cold),
            p1.status,
            p2.status,
            published_grace(&cold),
            cold.grace_clock,
            describe_gate(&g_relaunch),
            published_grace(&cold_relaunch),
            describe_gate(&g_after),
            pc.status,
            cold_after.grace_clock
        );

        assert!(
            matches!(g0, Gate::Permitted(_)) && matches!(g_relaunch0, Gate::Permitted(_)),
            "G2-2 precondition: the capable verdict at T0 permits, and so does a relaunch on \
             the same clock; got {} / {}",
            describe_gate(&g0),
            describe_gate(&g_relaunch0)
        );
        assert_eq!(
            cold0.grace_clock.elapsed_secs,
            Some(0),
            "G2-2 precondition: the capable time is the injected clock's reading at the pass \
             — the port, not the system clock; got {:?}",
            cold0.grace_clock
        );
        // The money path first: this is the claim the row is FOR.
        let e_cold = refused_by_the_clock(&g_cold).unwrap_or_else(|| {
            panic!(
                "G2-2: the recorded capable time is a DAY LATER than the device clock and the \
                 server froze its tip at TIP with the branch omitted → the send gate REFUSES \
                 `ConsensusGraceExpired {{ by: Clock }}` on a cold read. A capable time later \
                 than now is a time the wallet cannot trust; §4v (the founder, 2026-09-10, \
                 item 1) makes it an EXPIRED clock rule, fail-closed — because the block rule \
                 reads ZERO on a frozen claim forever (§4n-review row 1), and a clock rule that \
                 ABSTAINS here leaves the grace with no expiry on either axis until the one \
                 thing this server withholds. At the base (the OLD rule): permitted, `Running \
                 {{ blocks_left: 1152, secs_left: None }}` — G-4's permit half, blocks and no \
                 time. got {}",
                describe_gate(&g_cold)
            )
        });
        assert!(
            parked_names_the_clock_ending(&parked_cold),
            "G2-2: the parked row AGREES with the gate — not a healthy pending send, and its \
             `signing_block` names the clock ending (G-13's third path); got {parked_cold:?} \
             beside {e_cold:?}"
        );
        assert!(
            is_the_clock_ending(surface_grace(&p1.status))
                && is_the_clock_ending(surface_grace(&p2.status)),
            "G2-2: each frozen pass published `UpToDateUnverified` with `Ended {{ by: Clock \
             }}` — the existing clock ending, no new string; got {:?} / {:?}",
            p1.status,
            p2.status
        );
        assert!(
            is_the_clock_ending(published_grace(&cold)) && cold.grace_clock.elapsed_secs.is_none(),
            "G2-2: the cold read carries the clock ending and NO elapsed reading — there is no \
             trustworthy elapsed time on a capable time later than now, and the reading never \
             invents one; got {cold:?}"
        );
        assert!(
            refused_by_the_clock(&g_relaunch).is_some(),
            "G2-2: a relaunch on the SAME clock with NO pass is still refused by the clock — \
             the refusal is read from the durable row against the clock at the read (G-13), \
             not from a bit a process computed once; got {}",
            describe_gate(&g_relaunch)
        );
        assert!(
            parked_names_the_clock_ending(&parked_relaunch)
                && is_the_clock_ending(published_grace(&cold_relaunch)),
            "G2-2: …and the parked row and the cold read say so across the relaunch too; got \
             {parked_relaunch:?} / {cold_relaunch:?}"
        );
        assert!(
            matches!(g_after, Gate::Permitted(_)),
            "G2-2: ONE capable pass (branch reported) restores signing; got {}",
            describe_gate(&g_after)
        );
        assert_eq!(
            cold_after.grace_clock.elapsed_secs,
            Some(0),
            "G2-2: …and re-records the capable time from the clock AS IT READS NOW — a day \
             behind T0 — so the elapsed reading is 0 on this clock; got {:?}",
            cold_after.grace_clock
        );
        assert!(
            parked_reads_healthy(&parked_after)
                && !shows_grace(&pc.status)
                && is_at_or_above_terminal_at(&pc.status, TIP),
            "G2-2: after the capable pass the parked row is an ordinary pending send and the \
             surface shows no grace; got {parked_after:?} / {}",
            describe(&pc)
        );
        let all_served = [served, s1, s2, sc].concat();
        let readings = [G1_T0, G1_T0 - GRACE_SECS];
        for (label, st) in [
            ("frozen", &p1.status),
            ("frozen again", &p2.status),
            ("capable", &pc.status),
        ] {
            assert_carries_no_fixture_datum(st, &all_served, HOST_A, &format!("G2-2 {label}"));
            assert_carries_no_clock_reading(st, &readings, &format!("G2-2 {label}"));
        }
        w.close().await.expect("close");
    }

    /// **G2-3 — DEFECT (§4v).** A pre-epoch clock reads as expired by clock.
    ///
    /// `SystemClock::now_unix` answers 0 on a clock before the epoch
    /// (`ports.rs`), and both halves of that cell fell through GRACE-1's
    /// abstention (§4v's mechanism section; the fold review's row 7 found the
    /// `elapsed = Some(0)` half):
    ///
    /// (a) A capable pass at `T0 > 0`; a queued send; a relaunch with
    /// `ManualClock::at(0)` (the contract's own constructor) and one pass over
    /// a server that omits the branch with its tip frozen — the capable time
    /// is later than the clock. The gate REFUSES `ConsensusGraceExpired { by:
    /// Clock }`, the parked row agrees, the surface and the cold read carry
    /// `Ended { by: Clock }`, and the cold read's `elapsed_secs` is `None`.
    /// At the base: an abstention — permitted, `Running { 1152, None }`.
    ///
    /// (b) The capable pass itself taken AT clock 0 — the recorded capable
    /// time IS 0, asserted through the snapshot's `evaluated_at_unix` (the
    /// latest verdict's time): the `== 0` cell, not a clock at 1 — and the
    /// same frozen pass read at clock 0. `0 ≤ 0` is not a time the wallet can
    /// trust, it is a clock that never read: refused by the clock, the same
    /// three readings — and the surface never says "about 24 more hours" (no
    /// 86,400 / 1,440 anywhere in it). At the base: `elapsed = Some(0)`,
    /// permitted, `Running { blocks_left: 1152, secs_left: Some(86400) }` — a
    /// full day of grace measured from a time that never existed.
    ///
    /// The two permits are asserted TOGETHER so the one printed line carries
    /// both cells' readings. Two wallets, two vaults; nothing in (a) reaches
    /// (b).
    ///
    /// **Self-mutant SM-2 (§4v anti-vacuity, measured at the base):** (b)'s
    /// capable pass taken at clock 1 instead of 0. The row's REASON moves — it
    /// no longer reds on the permit but on the `== 0` precondition below
    /// (`left: Some(1)`, `right: Some(0)`), and the printed reading changes with
    /// it (`evaluated_at Some(1)`, `GraceClock { elapsed_secs: Some(0) }`: at
    /// clock 1 the capable time is not later than now and not zero either, so
    /// (b) is no longer a cell §4v names — it is (a)'s cell over again, one
    /// second up). The precondition is what keeps this row on the `== 0` cell
    /// rather than on whichever cell the fixture happens to land in.
    #[tokio::test]
    async fn a_pre_epoch_clock_reads_as_expired_by_clock() {
        // (a) a capable time > 0 against a clock at 0.
        let dir_a = tempfile::tempdir().expect("tempdir");
        let vault_a = test_vault();
        let (w, served_a) =
            scanned_wallet_capable_at(dir_a.path(), &vault_a, "G2-3(a)", G1_T0).await;
        w.queue_send_uri(G1_SEND_URI)
            .await
            .expect("queueing is ungated");
        w.close().await.expect("close");
        let w = reopen_with_clock(dir_a.path(), &vault_a, 0).await;
        let (pa, sa) = unknown_pass(
            &w,
            0,
            "G2-3(a) clock 0, capable time T0, claim frozen at TIP",
        )
        .await;
        let ga = send_gate(&w, "G2-3(a) clock 0").await;
        let parked_a = parked_blockage(&w, "G2-3(a) clock 0").await;
        let cold_a = w.consensus_status().await.expect("consensus_status");
        w.close().await.expect("close");

        // (b) a capable time of 0 against a clock at 0.
        let dir_b = tempfile::tempdir().expect("tempdir");
        let vault_b = test_vault();
        let (w, served_b) = scanned_wallet_capable_at(dir_b.path(), &vault_b, "G2-3(b)", 0).await;
        let cold_cap = w.consensus_status().await.expect("consensus_status");
        w.queue_send_uri(G1_SEND_URI)
            .await
            .expect("queueing is ungated");
        let (pb, sb) = unknown_pass(
            &w,
            0,
            "G2-3(b) clock 0, capable time 0, claim frozen at TIP",
        )
        .await;
        let gb = send_gate(&w, "G2-3(b) clock 0").await;
        let parked_b = parked_blockage(&w, "G2-3(b) clock 0").await;
        let cold_b = w.consensus_status().await.expect("consensus_status");
        let ra = format!("{:?}", pa.status);
        let rb = format!("{:?}", pb.status);
        println!(
            "[GRACE-2 G2-3] (a) capable at T0, clock 0: {}; parked {parked_a:?}; surface {ra}; \
             cold {:?} / clock {:?} | (b) capable at 0, clock 0: evaluated_at {:?}; {}; parked \
             {parked_b:?}; surface {rb}; cold {:?} / clock {:?}",
            describe_gate(&ga),
            published_grace(&cold_a),
            cold_a.grace_clock,
            cold_cap.evaluated_at_unix,
            describe_gate(&gb),
            published_grace(&cold_b),
            cold_b.grace_clock
        );

        assert_eq!(
            cold_cap.evaluated_at_unix,
            Some(0),
            "G2-3(b) precondition: the capable verdict was taken AT clock 0 — the recorded \
             capable time is 0, the `== 0` cell (a clock at 1 is not this cell); got {:?}",
            cold_cap.evaluated_at_unix
        );
        // The money path first, both cells at once: this is the claim the row is FOR.
        let (e_a, e_b) = match (refused_by_the_clock(&ga), refused_by_the_clock(&gb)) {
            (Some(e_a), Some(e_b)) => (e_a, e_b),
            _ => panic!(
                "G2-3: a pre-epoch clock reads as EXPIRED by the clock — the send gate refuses \
                 `ConsensusGraceExpired {{ by: Clock }}` in both cells (§4v), on a frozen tip \
                 the only rule that can end this grace. (a) a capable time of T0 read on a \
                 clock at 0 (what `SystemClock::now_unix` answers before the epoch): a capable \
                 time later than the device clock; at the base an abstention — permitted, \
                 `Running {{ blocks_left: 1152, secs_left: None }}`. (b) a capable time of 0 — \
                 the pass itself taken on a pre-epoch clock — read on a clock at 0: `0 ≤ 0` is \
                 not a time the wallet can trust; at the base `elapsed = Some(0)` — permitted, \
                 `Running {{ blocks_left: 1152, secs_left: Some(86400) }}`, 'about 24 more \
                 hours' measured from a time that never existed. got (a) {}; (b) {}",
                describe_gate(&ga),
                describe_gate(&gb)
            ),
        };
        assert!(
            parked_names_the_clock_ending(&parked_a)
                && is_the_clock_ending(surface_grace(&pa.status))
                && is_the_clock_ending(published_grace(&cold_a))
                && cold_a.grace_clock.elapsed_secs.is_none(),
            "G2-3(a): the parked row, the surface and the cold read all carry the clock \
             ending, and no elapsed reading is invented; got {parked_a:?} / {ra} / {cold_a:?} \
             beside {e_a:?}"
        );
        assert!(
            parked_names_the_clock_ending(&parked_b)
                && is_the_clock_ending(surface_grace(&pb.status))
                && is_the_clock_ending(published_grace(&cold_b))
                && cold_b.grace_clock.elapsed_secs.is_none(),
            "G2-3(b): the parked row, the surface and the cold read all carry the clock \
             ending, and no elapsed reading is invented; got {parked_b:?} / {rb} / {cold_b:?} \
             beside {e_b:?}"
        );
        assert_carries_no_full_day(&rb, "G2-3(b) surface");
        assert_carries_no_full_day(&format!("{:?}", published_grace(&cold_b)), "G2-3(b) cold");
        assert_carries_no_fixture_datum(&pa.status, &[served_a, sa].concat(), HOST_A, "G2-3(a)");
        assert_carries_no_clock_reading(&pa.status, &[G1_T0], "G2-3(a)");
        assert_carries_no_fixture_datum(&pb.status, &[served_b, sb].concat(), HOST_A, "G2-3(b)");
        w.close().await.expect("close");
    }
}
