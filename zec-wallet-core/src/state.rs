//! Wallet state DTOs — what Dart sees (spec §2.5). All DTOs are SDK-owned;
//! librustzcash types never leak into the public API, so upstream nu-churn
//! is absorbed inside the core (ADR-0005 refinement 5).

use crate::money::{BlockHeight, TxId, ZatBalance, Zatoshis};
use crate::net::host_dialer::{HostTransportName, IsolationSupport, TransportExposure};

/// Balance breakdown. `transparent` is a REAL, privacy-relevant user state
/// (drives the host shield banner; nonzero after a failed auto-shield must
/// be visible — §2.5).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct BalanceSnapshot {
    /// Spendable now: shielded funds confirmed to the spendable confirmations
    /// policy (`account::spendable_policy` — ZIP-315: 3 trusted / 10 untrusted), the
    /// SAME policy that drives `SyncStatus::Scanning.spendable_ready`, so the two can
    /// never disagree (the SSOT). From `AccountBalance::spendable_value()`.
    pub spendable: Zatoshis,
    /// Received but not yet spendable (`value_pending_spendability()`): notes
    /// still below the confirmation depth, AND notes whose witness cannot be
    /// built until more of the chain is scanned — upstream's own definition,
    /// which does not say which. The second case is the COMMON one for a wallet
    /// tracking a moving tip (phase-1 §3a: an Ironwood note is held here
    /// whenever ANY post-activation scan range is not yet `Scanned`, thousands of
    /// confirmations notwithstanding), so this field must never be rendered as
    /// "confirming" — say "arriving" / "not yet spendable", never "once it
    /// confirms" (phase-2 P2-4). The value is upstream's, unchanged.
    pub pending_incoming: Zatoshis,
    /// Our own change in flight (`change_pending_confirmation()`).
    pub pending_change: Zatoshis,
    /// Unshielded funds (`unshielded_balance().total()`) — drives the host shield
    /// banner; a nonzero after a failed auto-shield must stay visible (§2.5).
    pub transparent: Zatoshis,
    /// The audited grand total (`AccountBalance::total()`), mapped directly — NOT a
    /// re-sum of the fields above (so a future upstream pool is never silently dropped).
    pub total: Zatoshis,
}

/// `{height, at}`: when the balances/history shown were last
/// chain-confirmed. `at` is unix seconds, DISPLAY-ONLY (internal timing is
/// monotonic, §7).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SyncStamp {
    pub height: BlockHeight,
    pub at: u64,
}

/// What `Wallet::sync_for` (FR-40, the bounded sync) reports once its one pass
/// returns. Both heights are read from the wallet's database after the pass,
/// never from the pass's own progress samples.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BoundedSync {
    /// The fully-scanned frontier: every block at or below it is scanned (the
    /// incoming-funds watermark's height, `history::fully_scanned_frontier`).
    /// `None` before any block is scanned.
    pub scanned_to: Option<BlockHeight>,
    /// The chain tip the wallet has recorded — this pass's, once it got past its
    /// tip fetch. `None` before any pass has reached a server.
    pub tip: Option<BlockHeight>,
    /// The pass reached the tip: it returned without its deadline, `stop_sync`
    /// or the stuck-sync watchdog cutting it.
    pub finished: bool,
    /// The post-pass resubmission of queued and in-flight sends ran to its end.
    /// `false` when the budget left after the pass did not cover its worst case
    /// (or the pass did not finish): the next `start_sync` pass carries it.
    pub resubmitted: bool,
}

/// Honest-degradation sync state (§2.5). Every state is renderable; a
/// silent hang is a bug by contract (§3.3: streams never die, faults surface
/// as data).
#[derive(Clone, PartialEq, Debug)]
#[non_exhaustive]
pub enum SyncStatus {
    /// Wallet open, sync not started.
    Idle,
    /// A FORWARD SEAM with no producer in core. A transport's bootstrap (the
    /// `zec_wallet_tor` plugin's arti, a host's own) does NOT arrive here: it
    /// reaches the wallet through the registered dialer's descriptor and
    /// renders as [`TorState::Bootstrapping`] (FR-5 C1). Kept because removing
    /// a public arm is a bigger change than any feature has needed.
    Connecting {
        tor_bootstrap_percent: Option<f32>,
    },
    /// Spend-before-Sync (§1.7): funds usable before 100%. `percent` is
    /// MONOTONIC scanned/total — never range position (non-linear scan order
    /// would make a naive percent regress).
    Scanning {
        /// MONOTONIC scanned-equivalent height (`wallet::scanned_equiv`), placed so
        /// `to - from == round((to - birthday) * (1 - percent))` — an honest blocks-left
        /// that AGREES with `percent` and counts DOWN as it rises (a reorg lowering
        /// `percent` raises it). NOT the raw download frontier: rendering `to - from`
        /// gives a consistent countdown, never the jumping "1 %, 289 blocks left".
        from: BlockHeight,
        /// The chain tip the pass is scanning toward.
        to: BlockHeight,
        percent: f32,
        spendable_ready: bool,
        /// THIS pass has rewound at least once (a chain reorg un-scanned a
        /// span; #317). First-class so a host header/latch reacts to the
        /// rewind explicitly instead of inferring it from a shrinking `to` —
        /// the depth stays observability-only (`wallet.reorg_rewind`, §5.4).
        /// Resets with the pass: the next pass's samples start `false`.
        rewound: bool,
    },
    UpToDate {
        tip: BlockHeight,
    },
    /// Scanned as far as the chain goes, but this build could NOT fully
    /// interpret every block it passed (`ironwood-nu63-support.md` §6.4): the
    /// network is running consensus rules it does not implement, so blocks past
    /// that upgrade may carry value in a pool it cannot model, and post-upgrade
    /// transactions cannot even be parsed for their memos.
    ///
    /// **A DISTINCT VARIANT, not a flag on [`Self::UpToDate`], on purpose.** A
    /// host that ignores a new bool renders a confident "Up to date" over a
    /// range it was just told is unreliable — which is the §0 silence wearing a
    /// different hat. A host that ignores a new VARIANT falls into its
    /// catch-all instead, which is honest by default. Balance shown alongside
    /// this is a FLOOR, not a total (§1.5).
    UpToDateLimited {
        tip: BlockHeight,
    },
    /// Scanned to the chain tip, but THIS ENDPOINT did not fully serve every
    /// shielded pool's subtree roots on the pass that reached it (T0-1b): a pool
    /// whose roots the server refuses, or serves completion heights for that
    /// cannot be true, cannot have its notes witnessed from this server — money
    /// received there stays unspendable while everything else looks synced
    /// (INC-020's own user-visible failure). The honest next step is "switch
    /// servers": never "update the app" ([`Self::UpToDateLimited`]) and never
    /// "check your connection" ([`StallReason::EndpointUnreachable`]).
    ///
    /// **A DISTINCT VARIANT, for the reason `UpToDateLimited` is one:** a host
    /// that ignores it falls into its catch-all, which is honest by default; a
    /// host that ignored a flag on `UpToDate` would render a confident "Up to
    /// date" over a pool it was just told is unserved. Balance shown alongside
    /// this is a FLOOR for the unserved pool.
    ///
    /// **Raised for a REFUSED pool, a REFUSED height sequence, or a WITHHELD
    /// pool** — and the last is CONDITIONAL (T0-1b adjudication (b); widened to
    /// short serves by T0-1d). A pool served with zero completed subtrees is
    /// every pool's honest state before its first 2^16 notes, and the wallet
    /// cannot tell an honest zero — or an honest short serve — from a withheld
    /// one on the wire; but the signed bundle it ships with can, for the heights
    /// it covers: when a bundled treestate at or below the endpoint's own tip
    /// already shows `proven` subtrees complete for that pool and the server
    /// served fewer — none, or a strict prefix — it is a server that serves less
    /// than the network has ([`PoolService::Withheld`]) — INC-020's stuck state,
    /// "switch servers", for every note above the last served subtree. A serve
    /// with no such proof against it (a pool below its first completion, a new
    /// pool's first weeks, a testnet below the bundle's coverage, or a count at
    /// or above what the bundle can prove) renders as `UpToDate`, truthfully,
    /// with the count carried in [`PoolService::Served`]. Raising on THAT serve
    /// would be a false alarm on the normal state of every honest wallet, not
    /// honesty.
    ///
    /// **Scoped to the pass that produced it.** It is a property of this
    /// endpoint's answer on this pass, held in memory on the status channel: no
    /// durable row records the pool report, a later pass on which every pool is
    /// served publishes a plain `UpToDate` again, and a new handle — the only
    /// way this SDK changes endpoints — starts at `Idle`. (The pass that
    /// produced it DOES write the durable last-synced stamp and `ever_synced`,
    /// as any at-or-above pass does: it reached the tip; the server under-serves
    /// a pool. Its sibling below differs there.) The cost is the one
    /// `consensus_stamp` argues against: a cold start shows nothing until the
    /// first pass, which is also true of every other sync state.
    ///
    /// `UpToDateLimited` takes precedence when both hold: a build that cannot
    /// read the chain cannot act on "switch servers" until it is updated, and
    /// its balance-is-a-floor claim already subsumes this one.
    ///
    /// **Its sibling, [`Self::EndpointBehind`] (T0-1c), also takes precedence
    /// when both hold** — and carries this variant's `pools` report with it, so
    /// the pool fact is not dropped when the server is behind AND under-serving
    /// a pool: one status carries both facts (D1 — never a conflation, never a
    /// silent drop). The full order in `sync_controller::emit_synced`:
    /// `UpToDateLimited` > `EndpointBehind` > `UpToDateDegraded` > `UpToDate`.
    UpToDateDegraded {
        tip: BlockHeight,
        /// What the endpoint did for EVERY subtree-root pool on this pass, so a
        /// host can name the pool and say whether it was refused, lied about,
        /// or — for the pools that are fine — how many completed subtrees it
        /// served. `extraction_policy`-clean: a pool name and a bounded count
        /// describe the SERVER's behaviour and identify no note, account,
        /// address, height or endpoint.
        pools: PoolServiceReport,
    },
    /// Scanned to THIS ENDPOINT's reported tip, and that tip is BELOW a height
    /// this wallet already holds without trusting any endpoint (T0-1c, INC-023;
    /// widened by T0-1c-R2): the newest height the signed bundle this binary
    /// ships with carries for the wallet's network, or the wallet's OWN scanned
    /// height less the reorg margin (`REORG_MAX_BLOCKS`). Either way the server
    /// is provably behind the chain — a validator still syncing, stuck or
    /// forked, or a server under-reporting its tip — and the balance shown
    /// beside this is current only as of `tip`, a block the network passed
    /// before this build shipped or before this wallet's last scan. Money
    /// received after it is not visible from here, and a send built against
    /// this server's tip carries an expiry the real chain may already be past.
    /// The honest next step is "switch servers": never "update the app"
    /// ([`Self::UpToDateLimited`]) and never "check your connection"
    /// ([`StallReason::EndpointUnreachable`]).
    ///
    /// **Or scanned to nothing (T0-1c-R3, INC-024).** When the account's
    /// birthday is above this server's `tip + 1` and the wallet has scanned
    /// nothing past that tip, the pass publishes this variant WITHOUT recording
    /// the tip or asking for a block: `Wallet::sync_once`'s guard
    /// (`sync::birthday_beyond_tip`) returns before `record_chain_tip`, because
    /// upstream's `update_chain_tip` asserts on the inverted range
    /// `birthday..tip + 1` — on the R2 join that pass panicked and the loop
    /// task died with the last status on glass, for every fresh wallet against
    /// a validator stuck below its birthday. `tip` is still the server's tip,
    /// `pools` is still this pass's root report (the roots are asked before the
    /// guard), the balance is current as of nothing this server can show, and
    /// the remedy is the same: switch servers. The pass on which a server
    /// reports a tip at or past the birthday scans as normal.
    ///
    /// **A DISTINCT VARIANT, for the reason `UpToDateLimited` is one (D1):** a
    /// host that ignores it falls into its catch-all, which is honest by default;
    /// a host that ignored a flag on `UpToDate` would render a confident "Up to
    /// date" over a height it was just told is stale — INC-023's silence with a
    /// bool bolted on. And NOT a stall: the pass completed, the pools were asked,
    /// the tip was recorded and the gap scanned (REPORT AND CONTINUE, §4k-R —
    /// the orchestrator's product decision, recorded for the maintainer to
    /// overturn; the REFUSE build sits on a private branch and was held back).
    ///
    /// **The evidence needs no endpoint trust.** `newest_known` is the reference
    /// the grade fired against (`sync::tip_standing`): the larger of
    /// `root_bind::newest_bundled_height` for the wallet's network — a PUBLIC
    /// CONSTANT of the signed binary (3,459,780 mainnet / 4,301,840 testnet
    /// today), the treestate table's newest row, a checkpoint with a hash and a
    /// time — and, since T0-1c-R2, the wallet's own scanned height less
    /// `REORG_MAX_BLOCKS` (blocks this wallet validated by scanning; no endpoint
    /// can move that number). It crosses the bridge under
    /// `tests/extraction_policy.rs`'s E10 shape row (no host, URL, index, note,
    /// account or address); as a height it narrows nothing `UpToDate { tip }`
    /// and `Scanning.scanned_to` do not already carry. So a host can say "behind
    /// by at least `newest_known − tip` blocks" — AT LEAST, because the chain
    /// has moved on since the row was compiled in or the wallet last scanned.
    ///
    /// **The residual, stated so it is not rediscovered:** an endpoint that
    /// under-reports its tip to anywhere AT OR ABOVE `newest_known` is not
    /// caught by this — it holds the wallet at a stale but self-consistent
    /// height. Before T0-1c-R2 that window was binary age (the chain's distance
    /// above the row); now it is the reorg margin below the wallet's own last
    /// scan, ~100 blocks, and it does not grow. Above the row every bundled
    /// proof is in force, so what such a server cannot do is hide a withheld
    /// pool (`sync::proven_complete_at_or_below`). Closing what is left needs a
    /// device-clock bound on the bundle's `(height, time)` rows — its own item.
    ///
    /// **Scoped to the pass that produced it**, like its sibling: held on the
    /// status channel; no durable row records the standing itself, the pass on
    /// which the endpoint reports a tip at or above the reference publishes a
    /// plain `UpToDate` again (E6), and a new handle starts `Idle`. **What a
    /// behind pass leaves behind durably: nothing** (T0-1c-R2, §4n M2). Before
    /// it, `sync_controller::emit_synced` stamped every clean pass, so a behind
    /// pass set `ever_synced` (write-once) and wrote the behind height into the
    /// last-synced stamp — after a relaunch the header read "as of block
    /// `<behind>`" with no qualification and the first-run framing was gone for
    /// the wallet's life. Now `record_synced` is gated on `AtOrAboveBundle`: a
    /// relaunch after a behind pass renders the last CURRENT stamp, or none
    /// with the first-run framing for a wallet that has never had one — the
    /// truth, since it has never been current.
    ///
    /// Precedence when several hold: `UpToDateLimited` outranks this (a build
    /// that cannot read the chain cannot act on "switch servers" until it is
    /// updated); this outranks `UpToDateDegraded` and carries its `pools` report,
    /// so nothing is hidden by the ranking.
    EndpointBehind {
        /// The endpoint's reported tip — the height this pass scanned to and the
        /// height the balance beside this is current as of.
        tip: BlockHeight,
        /// The newest height this wallet knows the chain reached (see above):
        /// the signed bundle's newest row for this network, or the wallet's own
        /// scanned height less the reorg margin, whichever is higher — the
        /// evidence, and a lower bound on how far behind the server is.
        newest_known: BlockHeight,
        /// What the endpoint did for every subtree-root pool on this pass — the
        /// SAME report [`Self::UpToDateDegraded`] carries, so a withheld or
        /// refused pool is visible beside the behind claim and not dropped by it.
        /// `None` when the pass made no pool claim at all (an engine that skipped
        /// root ingestion; never the production engine, which fills both claims
        /// from one pass) — "nothing to say", not "every pool served".
        pools: Option<PoolServiceReport>,
    },
    /// Scanned to the tip, but THIS SERVER will not say which network it is
    /// on — it omits `consensus_branch_id`, so the wallet's verdict is
    /// `Unknown` and signing rides the §6.3 grace (GRACE-1, §4p Q-G2). While
    /// the grace RUNS the user sees that the server stopped reporting the
    /// network version and how much grace remains — whichever of blocks and
    /// time expires first, the blocks converted through
    /// `TARGET_BLOCK_SPACING_SECS`; when it has ENDED, why (blocks / clock /
    /// never confirmed) and the next step: switch servers — or check the
    /// device's date and time, for the clock. Before this variant an `Unknown`
    /// wallet inside or outside its grace rendered plain `UpToDate` (§4p
    /// P-G3), and a refused send said "the network was upgraded — update the
    /// app", which for a silent server is false and the update fixes nothing.
    ///
    /// **A DISTINCT VARIANT, for the reason its three siblings are** (`D1`): a
    /// host that ignores it falls into its catch-all, which is honest by
    /// default. Balance beside it is CURRENT (the server serves blocks; only
    /// its network claim is missing) — this is not a balance-is-a-floor state,
    /// which is why it is not `UpToDateLimited` (that one is about the BUILD
    /// and says "update the app"; this one is about the SERVER and says
    /// "switch servers", the same next step as `EndpointBehind` and
    /// `UpToDateDegraded`).
    ///
    /// **What crosses (§4p G-12):** remaining counts and a reason — never the
    /// absolute capable timestamp, which is a signing input and stays inside
    /// the core.
    ///
    /// **Read from the stamp, not the pass** — `SyncEnginePort::unknown_branch_grace`,
    /// read against the device clock after every clean pass — so the reading
    /// SURVIVES A RELAUNCH: the anchors are durable, and the first pass after
    /// `open()` republishes it (a new handle starts `Idle` until then, like every
    /// status). `UpToDateLimited` cannot co-occur (one verdict) and outranks it
    /// by position; this outranks `EndpointBehind` and `UpToDateDegraded` because
    /// its consequence reaches SIGNING (a running grace is a countdown on the
    /// user's ability to send; an ended one is a refusal), and it carries the
    /// `pools` report with it so the pool fact is never dropped by the ranking
    /// (the behind claim's `newest_known` is the one fact this ranking hides in
    /// the both-at-once corner — same remedy, "switch servers"; the cost item 1
    /// prices). The full order in `sync_controller::emit_synced`:
    /// `UpToDateLimited` > this > `Stalled { EndpointMisbehaving }` (the
    /// rewinding streak's report, phase-2 P2-6 — it ranks below this so the
    /// countdown and the clock remedy are never hidden by "switch servers") >
    /// `EndpointBehind` > `UpToDateDegraded` > `UpToDate`.
    UpToDateUnverified {
        /// The height this pass scanned to.
        tip: BlockHeight,
        /// The grace as read at this pass's end: running (and how much is
        /// left) or ended (and why).
        grace: UnknownBranchGrace,
        /// The pass's pool report, carried so the ranking drops nothing;
        /// `None` when the pass made no pool claim.
        pools: Option<PoolServiceReport>,
        /// Phase-3 P3-12 (maintainer, Q2 option 2): the rewinding streak has
        /// reached its report (`Stalled { EndpointMisbehaving }` is what the
        /// loop would publish if this claim did not outrank it — P2-6). Carried
        /// so the glass can DROP "your balance is current" under a reported
        /// streak: a server that keeps serving blocks the wallet then rewinds
        /// is not one whose balance reading deserves that sentence, and the
        /// ranking otherwise hides the streak's remedy behind the grace's.
        /// The field log's `outcome = "rewinding_streak"` line is unchanged.
        streak_reported: bool,
    },
    /// Typed, renderable degradation — never a dead stream.
    Stalled {
        reason: StallReason,
    },
    /// No connectivity; queued ≠ error; the AGE of what the user is looking
    /// at is data.
    Offline {
        last_synced: Option<SyncStamp>,
    },
}

/// What an endpoint did for ONE shielded pool's subtree roots on a sync pass
/// (T0-1b — the public projection of the engine's per-pool fetch outcome).
/// Every outcome the engine can record is a DISTINCT value here, on purpose:
/// "the server does not know this pool", "the server served heights that cannot
/// be true", "the server served nothing where the network provably has some",
/// "the server served nothing yet" and "the server served N roots" are five
/// different sentences, and rendering any two of them alike is the conflation
/// INC-020 was made of.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum PoolService {
    /// The endpoint streamed `roots` completed subtree roots and closed cleanly.
    /// `0` here is a zero the wallet has NO proof against — every pool's state
    /// before its first completed subtree — reported as a count and not
    /// pretended about. Bounded by the client-side per-pool cap, never a raw
    /// endpoint number.
    Served { roots: u32 },
    /// The endpoint served FEWER roots — none, or a strict prefix — than the
    /// wallet can prove the pool has completed subtrees at or below the
    /// endpoint's own reported tip (`proven`, from the signed bundled treestates
    /// it ships with — a public constant, never this wallet's data). Withheld or
    /// broken, the effect is the same: notes in the subtrees it did not serve
    /// cannot be witnessed from this server, which is INC-020's stuck state
    /// exactly — for a short serve, every note above the last served subtree.
    /// NEVER "the pool is empty" — the bundle says it is not. Next step: "switch
    /// servers".
    ///
    /// Carries the PROOF, not the gap (T0-1d, §4l decision 2): `proven` is what
    /// the network provably has; how many of them the server did serve is on the
    /// core's log line, not here. A host therefore reads this as "this server
    /// serves less of this pool than the network has", zero or short alike —
    /// the same sentence and the same next step for both, which is why the
    /// short case widened this variant rather than minting a sibling (a
    /// distinct variant would have said "short" at the price of a bridge
    /// variant, five UI sites and sixteen locales, for a fact whose remedy is
    /// identical).
    Withheld { proven: u32 },
    /// The endpoint refused the protocol outright: it does not know this pool.
    /// Nothing was learned about the pool, and the roots already held for it
    /// are untouched. NEVER "the pool is empty".
    Unsupported,
    /// The endpoint served a completion-height sequence that cannot be true and
    /// the wallet refused to record it; nothing was written for this pool.
    /// NEVER "empty" and NEVER "unknown pool": the server answered, and the
    /// answer was a lie or a bug. Which check caught it is logged (§5.4, a
    /// payload-free code) and deliberately does not cross here — the next step
    /// is "switch servers" whichever check it was.
    HeightViolation,
}

/// Per-pool subtree-root service for every pool the engine ingests, in one
/// value — carried by [`SyncStatus::UpToDateDegraded`] and, beside the behind
/// claim, by [`SyncStatus::EndpointBehind`]. Named fields rather than
/// a list, so a host reads `pools.ironwood` and a fourth pool is a new field —
/// a compile error in the bridge's by-name conversion, never a silently dropped
/// entry.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PoolServiceReport {
    pub sapling: PoolService,
    pub orchard: PoolService,
    pub ironwood: PoolService,
}

/// Why the §6.3 unknown-branch grace does NOT permit signing (GRACE-1, §4p) —
/// the reason enum that crosses the bridge on the refusal
/// (`WalletError::ConsensusGraceExpired`), on the parked-row flag and on the
/// sync surface, so every copy site can name the cause and the next step
/// without re-deriving it. Three sentences, never rendered alike:
///
/// - [`Self::Blocks`]: the chain advanced a day's worth of blocks
///   (`UNKNOWN_BRANCH_GRACE_BLOCKS`) past the last verdict that confirmed this
///   build can transact, and this server never said which network it is on
///   since — "switch servers".
/// - [`Self::Clock`]: the DEVICE clock advanced a day
///   (`UNKNOWN_BRANCH_GRACE_SECS`) past it, whatever the server's tip did — the
///   dimension a server that freezes its tip cannot hold still — or the clock
///   was set back after that was observed (the latch). "Switch servers, or
///   check the device's date and time": a wrong clock is the one benign cause,
///   and a corrected clock does not re-permit on its own (a capable pass does).
/// - [`Self::NeverConfirmed`]: this wallet has never held a signing-capable
///   verdict at all — every server it has met withheld the branch — so there
///   is nothing to measure a grace from (outside it by construction, §6.3).
///   "Switch servers": waiting changes nothing, and this is NOT the
///   never-evaluated case (`WalletError::ConsensusNotEvaluated` — a wallet that
///   has not completed a pass yet, which the first pass resolves).
///
/// Never an "untrusted clock" member: a recorded capable time later than now
/// makes the clock rule ABSTAIN and the block rule decide alone — it is never
/// itself a reason to refuse (§4p item 2).
///
/// When BOTH rules have expired, `Blocks` is named: the block evidence alone
/// suffices, and "check the device time" would be a wrong hint beside it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum GraceExpiry {
    Blocks,
    Clock,
    NeverConfirmed,
}

/// The §6.3 unknown-branch grace as read at ONE instant (GRACE-1, §4p) — what
/// the sync surface carries while a server withholds `consensus_branch_id`
/// (`SyncStatus::UpToDateUnverified`), computed by
/// `ConsensusCompatibility::unknown_branch_grace` from the persisted stamp and
/// the device clock. REMAINING counts and a reason only (§4p G-12): the absolute
/// capable timestamp is a signing input and never leaves the core.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum UnknownBranchGrace {
    /// Inside BOTH rules — signing still permitted, and this much remains:
    /// `blocks_left` on the block rule (`UNKNOWN_BRANCH_GRACE_BLOCKS` less the
    /// blocks since the last capable verdict) and `secs_left` on the clock rule
    /// (`UNKNOWN_BRANCH_GRACE_SECS` less the device-clock seconds since it).
    /// `secs_left` is `None` only when NOTHING was recorded to measure from —
    /// a row no writer produces beside an anchor, since `consensus_stamp::record`
    /// writes the two together. A recorded capable time the clock cannot vouch
    /// for (later than now, or pre-epoch) does NOT arrive here: since GRACE-2
    /// (`production-readiness-phase-1.md` §4v) it is latched EXPIRED and the
    /// grace reads `Ended { by: Clock }`, never this variant with no time — the
    /// GRACE-1 "blocks and no time" reading (§4p G-4) is gone. Whichever is
    /// smaller once blocks are converted through `TARGET_BLOCK_SPACING_SECS`
    /// expires first; the host says that one.
    Running {
        blocks_left: u32,
        secs_left: Option<u32>,
    },
    /// Signing refused — `by` names the rule that ended it, or that it never
    /// began. `blocks_since_last_current` is the count the `Blocks` copy names
    /// ("…for N blocks"), carried for every reason so the host can show it
    /// beside the clock reason too; `None` for `NeverConfirmed`.
    Ended {
        by: GraceExpiry,
        blocks_since_last_current: Option<u32>,
    },
}

/// Why the queued-send drain will NOT sign a parked row (GRACE-1, §4p item 2 —
/// "the parked flag's FOUR states, each with its own copy": the contract's own
/// heading, ruled inexact at adjudication —
/// the GRACE-1 ruling, item 2: the flag was a bool; the
/// four were the GATE's count, which this enum plus `None` now carries).
/// Carried per row by
/// `ParkedSend::signing_block`, wallet-level in truth (every queued row shares
/// it; the condition is about the app or the server, never this payment).
/// Derived by `ConsensusCompatibility::signing_block` — the same predicate the
/// gate asks, never re-derived at the surface.
///
/// - `None` (no block): the row will sign on its own on the next pass that
///   permits it. This is ALSO the never-evaluated wallet's reading (§4p item
///   2): the first completed pass evaluates before it drains (INC-004), so
///   "will send on its own" is true there and "waiting for an app update" was
///   the lie.
/// - [`Self::NetworkUpgrade`]: the network runs rules this build does not
///   implement (`Unsupported`) — "waiting for an app update; your funds are
///   safe and nothing has been sent". Retry is useless; cancel stays.
/// - [`Self::GraceExpired`]: this server will not say which network it is on
///   and the grace has run out or never began — "waiting for a server that
///   reports the network version — switch servers" (plus "check the device's
///   date and time" for [`GraceExpiry::Clock`]). NEVER the update copy.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum SigningBlock {
    NetworkUpgrade,
    GraceExpired { by: GraceExpiry },
}

/// Why sync is stalled (§2.5).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum StallReason {
    EndpointUnreachable,
    /// `TorPolicy::Required` and the runtime is unreachable — fail-closed,
    /// ZERO clearnet packets (§2.3).
    TorUnavailable,
    StorageFull,
    ChainReorg,
    /// A corrupt wallet store (`WalletError::StoreCorrupt`), or a local fault the
    /// wallet could not diagnose (the scan fallback, `ChainHeightUnknown`) — the
    /// problem is on THIS device, not the network. The honest next step is
    /// repair / restore-from-seed, never "switch servers" (§2.5 — errors show
    /// next steps). Distinct from `StorageFull` ("free space and retry"), from
    /// [`Self::StorageUnavailable`] (a transient local fault that retries by
    /// itself — R10 narrowed this reason to the one class a restore can answer)
    /// and the network reasons (transient, auto-retried): a corrupt store will not
    /// heal by retrying, so it surfaces as a steady, truthful `Stalled { Internal }`
    /// rather than masquerading as `EndpointUnreachable` (the resolved d-3-b
    /// known-debt). The loop publishes it only at the SECOND local fault since a
    /// completed pass (R10 §4.2, `sync_controller::run_loop`).
    Internal,
    /// The endpoint ANSWERED, and the answer was wrong (T0-1b D9, widened to the
    /// whole content tier by the adjudication): usable transport, unusable DATA —
    /// a subtree-completion sequence the wallet's own evidence refutes, a root
    /// or block that will not decode, a block span that is short, gapped or
    /// overlong, a tree state anchoring the wrong height, a flooding stream, a
    /// required pool the server does not know, a server answering for the wrong
    /// CHAIN (a testnet lightwalletd under a mainnet wallet —
    /// `WalletError::NetworkMismatch`, mapped by `sync_controller::stall_for`;
    /// T0-1c). NOT a consensus `Unsupported` verdict: there the server is fine
    /// and the BUILD is stale, which is [`SyncStatus::UpToDateLimited`] ("update
    /// the app"), published rather than stalled. The link is FINE and the server
    /// is not — the mirror image of [`Self::Internal`] ("the problem is on this
    /// device"): the problem is on THIS SERVER, and the honest next step IS
    /// "switch servers". Distinct from [`Self::EndpointUnreachable`] (the
    /// transport tier: a dead link says "check your connection", and no
    /// reconnection fixes a server that misbehaves) and from `Internal` ("restore
    /// from seed" is a destructive remedy for a fault that is not on the device).
    /// Retried like the network reasons (the next pass re-fetches, so a server
    /// that heals is accepted without user action). Carries nothing (D11): no
    /// host, URL, index, height, note or address — which check caught it is
    /// logged (§5.4) and does not cross the bridge. Named for the CLASS, not
    /// for one of its members: "inconsistent" described the height lie only.
    ///
    /// **Two members of the class are conflicts with THIS WALLET'S OWN RECORD,
    /// and there the copy's remedy has a second step (T0-1d, §4l (a)).** A served
    /// subtree root that differs from the one already recorded at that index
    /// (`sync::map_shardtree_err`'s `Insert` arm), and a served completion
    /// height that differs from the recorded one (`root_bind::check_recorded_heights`),
    /// are each a disagreement between the server serving now and the server
    /// whose answer this wallet recorded on an earlier pass — a record written
    /// from a server, not knowledge of this wallet's own. The conflict alone
    /// cannot say which of the two lied: a first-contact server that recorded a
    /// wrong root at index 0 makes every honest server thereafter conflict at
    /// that address, and "switch servers" then never ends. So the honest copy
    /// reads: switch servers; and if EVERY server is refused, the record is the
    /// poison and the exit is a rescan (`Wallet::rescan_from` — a rebuild that
    /// keeps the seed and drops the shard rows; `wallet::tests::
    /// a_first_contact_root_conflict_is_cleared_by_the_rescan_rebuild`). Still
    /// never "restore from the recovery phrase": the seed is not involved and
    /// the wallet is intact. The class keeps ONE variant for both the members
    /// where the wallet knows the server is wrong and the two where it cannot —
    /// property bought: one class, one grep, one remedy that is honest for both
    /// record dimensions; property lost: the surface cannot say "this server is
    /// certainly wrong" where it does know, so the copy hedges for all.
    ///
    /// **A member that is a judgement over a RUN of passes, not one answer (§4u
    /// REW-1).** `crate::constants::MAX_CONSECUTIVE_REWINDING_PASSES` consecutive
    /// passes that each rewound — an `Ok` pass with `reorgs > 0`, or a pass
    /// stalled `ChainReorg` by the cap, the storm bound or the exit belt — make
    /// `sync_controller::run_loop` publish THIS reason in place of the pass's own
    /// terminal status, and keep publishing it after every further rewinding pass
    /// until clean passes (`Ok`, no rewind) have DECAYED the streak — one per
    /// clean pass, never cleared in one (the maintainer's ruling). An honest chain
    /// reorgs once, rarely twice in a row; a run that long is a server serving
    /// forks pass after pass, or one behind our scan on every pass, and the
    /// class's remedy is the honest one: switch servers. The pass's own facts
    /// stand under the judgement — an `Ok` rewinding pass that reached the tip
    /// still writes the last-synced stamp (a fact about the DB; this reason is a
    /// judgement about the server) — and the surface cannot flicker under it:
    /// only clean loop passes decay it below the report, a clean pass IS the
    /// evidence the server behaved, and a pull-to-refresh (`once()`) publishes
    /// the SAME judgement rather than a zero of its own (phase-2 P2-5: the count
    /// is the controller's shared state, `sync_controller::Shared::rewinding_streak`,
    /// one writer — the loop). Retried like the rest of the class. Not durable:
    /// the count is born zero with the controller, so a relaunch forgives it —
    /// but a `stop()`/`start()` does NOT (phase-2 P2-6, maintainer decision 6):
    /// the host's reconnect kick answers this reason with exactly that pair, and
    /// a report a reachability tick or "Try now" could erase was no report.
    /// Where it RANKS (decision 5): below `UpToDateLimited` and below
    /// `UpToDateUnverified` — a stale build is told "update the app" first, and
    /// a grace's countdown or clock remedy is never hidden by "switch servers" —
    /// and above the behind and pool claims (`UpToDateUnverified`'s doc has the
    /// full order).
    ///
    /// **The record-conflict escape hatch above does NOT apply to this member,
    /// and the copy must not offer it (phase-1 §4u-run row 7; phase-2 P2-7).**
    /// A streak is a judgement about what the server SERVED, never about a row
    /// this wallet holds: a rewind that LANDS drops the very `blocks` rows it
    /// rewound (`sync::rewind_wallet_to` → upstream's `truncate_to_height`,
    /// `DELETE FROM blocks WHERE height > landed`; the re-queued span is then
    /// re-written from the serving server), so a stored record — a block hash a
    /// first-contact server lied about, say — costs an honest server at most
    /// ONE rewinding pass before the row is gone and the streak decays; and a
    /// rewind upstream REFUSES (`sync::truncate_at_or_below`, both heights
    /// `RequestedRewindInvalid`) never reaches the streak at all — it is
    /// `sync::endpoint_unusable()`, the record-conflict member two paragraphs
    /// up, which carries the hatch. A streak that persists across EVERY server
    /// is therefore the servers' doing, and a rescan removes nothing that
    /// causes it: "switch servers" is the whole remedy here, and "if every
    /// server is refused, rescan" would send the user to a rebuild that
    /// changes nothing.
    EndpointMisbehaving,
    /// This wallet is set to start from a block height the chain, as THIS
    /// SERVER reports it, has not reached (T0-1c-R2, §4m #13 / §4k-run owed
    /// 2): `WalletError::BirthdayInFuture` on a provisioning pass — a
    /// configured birthday above `max(the server's tip, the bundle's newest
    /// row)` (`provision::resolve_birthday`). Its own reading because neither
    /// neighbour is honest for it: not `EndpointUnreachable` (the link worked
    /// and the server answered — the `_ =>` fallback it used to take said
    /// "check your connection", forever, for a number the server cannot
    /// change), not `Internal` (nothing on the device is at fault; "restore
    /// from seed" would destroy the wallet for a config value), and not
    /// `EndpointMisbehaving` alone (the server may be right: a birthday typed
    /// above the real chain tip is the HOST's error, and "switch servers" never
    /// fixes it). The wallet cannot tell a server behind the chain from a
    /// birthday ahead of it — the birthday is above the one height the bundle
    /// can vouch for — so the copy names BOTH remedies: check the starting
    /// height you entered, or try another server. Retried like the network
    /// reasons: a server that catches up, or a host that lowers the birthday
    /// (`Wallet::rescan_from`), clears it. Carries nothing (D11).
    ///
    /// **Two producers since T0-1c-R3, and where neither fires.**
    /// `provision::resolve_birthday` on the provisioning pass (a configured
    /// birthday above `max(the server's tip, the row)`), and `Wallet::sync_once`'s
    /// guard on ANY pass (`sync::birthday_beyond_tip`: the account's birthday
    /// above `tip + 1`, nothing scanned past the tip, and the tip at or above
    /// the grade's reference — the birthday is above a height no oracle of this
    /// wallet disputes). Below the reference NEITHER fires: a birthday the chain
    /// provably reached is not in the future whatever a behind server says — M3
    /// clamps a provisioning tip up to the row, and the guard returns the pass
    /// as `SyncStatus::EndpointBehind` with nothing scanned instead of handing
    /// upstream the inverted range `birthday..tip + 1` that `update_chain_tip`
    /// asserts on (INC-024: on the R2 join that pass panicked and the loop
    /// died with the last status on glass). The sync-pass producer is
    /// unreachable from any shipped resolver today — every birthday written is
    /// at most the anchor row + 1, below the row on both networks — and exists
    /// because the contract names the cell and the cost is one arm.
    BirthdayInFuture,
    /// This device's wallet storage could not be used for a moment (R10):
    /// `WalletError::StoreBusy` (a lock held past `busy_timeout` — a WAL race
    /// with another writer) or `WalletError::Io` (an I/O fault, e.g. a locked
    /// iOS device's Data Protection closing the file). Both are the classes #371
    /// documents as transient and non-corrupt, so the store is intact and the
    /// next pass retries by itself. The honest copy is "sync paused on this
    /// device, retrying" — NEVER "restore from the recovery phrase" (that is
    /// [`Self::Internal`]'s, for a store that is actually corrupt) and never
    /// "switch servers" or "check your connection" (the network is not
    /// involved). Like `Internal` it is a LOCAL reason: `run_loop` holds back
    /// the first local fault since a completed pass and publishes the second
    /// (R10 §4.2); `once()` publishes at once. Appended last (the variants are
    /// append-only; the bridge mirror moves to ABI 8 with it).
    StorageUnavailable,
}

/// Transaction lifecycle state (§2.5). `Queued` is a first-class normal
/// state (offline-first, §6.2), not an error.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum TxStatus {
    Queued,
    Pending,
    Confirmed {
        depth: u32,
    },
    /// Expired unmined — funds RETURN to spendable (§6.1); a normal history
    /// row, also how the multi-device double-spend race resolves (§10).
    Expired,
    Failed,
}

/// Where a WALLET-CREATED transaction stands on its way to the chain — the
/// delivery obligation's four readings (stage S8 `obligation`; the mechanism is
/// [`crate::delivery`]). Orthogonal to [`TxStatus`]: a received transaction and
/// an expired one have none (`None` on [`TxSummary::delivery`]), because the
/// wallet owes them no broadcast and `TxStatus` already says what they are —
/// except an expired one a live queued send will REPLACE, which reads
/// [`RetryPending`](Self::RetryPending) beside its `Expired` status.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum DeliveryState {
    /// The signed bytes are kept, and the wallet is NOT going to broadcast them on
    /// its own right now: a swap deposit held past its quote's window (the deposit
    /// hold, both arms), or the narrow create-committed-but-unrecorded window the
    /// intent path waits out.
    Persisted,
    /// The wallet owes a broadcast and attempts it on the next sync pass and after
    /// a reopen — the same bytes every time, until it is accepted, mined or expired.
    /// On an EXPIRED transaction it means the attempt is dead but the wallet WILL
    /// send the payment again by itself (a fresh transaction, once the expiry is
    /// buried beyond reorg reach): the user must not send it again by hand.
    RetryPending,
    /// An endpoint accepted the bytes into its mempool; not yet in the chain. The
    /// wallet keeps rebroadcasting until it mines (a duplicate is rejected
    /// harmlessly), so this is a reading, never a stop.
    Accepted,
    /// Mined — the chain owns it.
    Confirmed,
}

/// Groups the transactions minted by ONE proposal — after a kill, history
/// can reconstruct "payment X half-landed" (§6.3); also tags shield and
/// swap-deposit txs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BatchId(u64);

impl BatchId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn value(self) -> u64 {
        self.0
    }
}

/// Opaque handle to a durably-queued send intent (§3.1 `queue_send`; §3.2h
/// inc-2d-3-b-i). Wraps the `queued_send_intent` row id (the SQLite `rowid`,
/// hence `i64`). Returned to the host so a later UI can reference / cancel the
/// queued send; it carries NO money data (the amount/recipient live only in the
/// encrypted intent row). Crosses FFI as a string at inc-2d-ffi.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct QueuedSendId(i64);

impl QueuedSendId {
    pub fn new(id: i64) -> Self {
        Self(id)
    }

    pub fn value(self) -> i64 {
        self.0
    }
}

/// One history row (§2.5).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TxSummary {
    pub txid: TxId,
    pub batch_id: Option<BatchId>,
    pub mined_height: Option<BlockHeight>,
    pub status: TxStatus,
    /// Signed effect on this wallet.
    pub net_amount: ZatBalance,
    pub fee: Option<Zatoshis>,
    pub has_memo: bool,
    /// Whether the wallet sees a NON-CHANGE TRANSPARENT output on this tx — the
    /// §5.1 public-payment class (a send to a transparent recipient, a payment
    /// received at our transparent address, or a ZIP-320 hop leg): its amount +
    /// addresses are publicly visible on-chain, so history must be able to
    /// DISTINGUISH it (§3.2i-3 pool clarity). Shielded activity and
    /// pool-internal shields (whose outputs are shielded) stay `false`.
    pub has_transparent_output: bool,
    /// Unix seconds, display-only.
    pub timestamp: Option<u64>,
    /// The delivery obligation's reading for a wallet-created transaction
    /// ([`DeliveryState`]); `None` for a received or expired one.
    pub delivery: Option<DeliveryState>,
    /// The last height at which a transaction THIS WALLET CREATED can be mined
    /// (its ZIP-203 expiry) — the height at which an unknown send outcome
    /// resolves either way. `None` for a received transaction and for one built
    /// with no expiry (a stored `0`).
    ///
    /// **The reading rule:** status is judged against the wallet's OWN scan
    /// (`v_transactions.expired_unmined` compares against `MAX(blocks.height)`),
    /// never the chain tip. So beside a `Pending` row, read this height against
    /// the chain tip: at or below it, the chain has already decided and a sync
    /// that scans to at least this height resolves the row (`Confirmed` or
    /// `Expired`); above it, the outcome is unresolved on chain until block N.
    pub expiry_height: Option<BlockHeight>,
}

/// What edge produced an [`IncomingFundsEvent`] (ADR-0536 #392).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum IncomingFundsEventKind {
    /// The one per-subscriber catch-up event emitted first on subscribe: arrivals
    /// already in history above the host's `since_cursor` (count 0 on a `None`
    /// cursor — the baseline that hands the host a cursor to persist).
    Replay,
    /// The scan edge detected arrivals in a just-committed batch (or, on a detect
    /// read fault, MAY have — `new_tx_count` 0 with a span set is the
    /// conservative-fire "pull to confirm" degradation, never a silent miss).
    Live,
    /// An enhancement pass changed how existing rows read — re-pull if it matters. Counts
    /// unchanged; the cursor carries the last delivered watermark forward (the
    /// pump substitutes it, so a DELIVERED event's cursor is never empty
    /// M2; only the channel-internal value can be pre-stamp empty).
    ///
    /// TWO causes, not one (widened, INC-009 — ADR-0536 Decision 4 named only the first;
    /// **ADR-0539** amends it and is the current contract):
    /// 1. decrypted tx data was stored (memos arrive AFTER scan detection), so attribution
    ///    may have improved; or
    /// 2. a `GetStatus` request was answered with a chain status, so a send may have flipped
    ///    out of "expired" — no new data to decrypt, but a materially different rendering.
    ///
    /// So this kind does NOT imply new attribution data exists. A host that prefetches memos
    /// on the strength of the kind alone will find nothing roughly half the time; the honest
    /// reaction is the one the reference host already takes — re-pull the activity list.
    MemoRefresh,
}

/// Incoming-funds stream item (§2.5, reshaped by ADR-0536): the MINIMAL
/// counts+heights "funds arrived" event — §5.4-safe BY CONSTRUCTION (no txid,
/// amount, memo, or address; a host can log it or forward it toward a
/// notification path with no sanitizing layer). Attribution stays a PULL via
/// `transactions()` / `transaction_memos()` in the main app (FR-11: never the
/// NSE). Delivery is AT-LEAST-ONCE over a latest-wins `watch` channel:
/// `new_tx_count` and the span are advisory under coalescing;
/// `total_tx_detected` is the coalesce-proof monotonic a host diffs to know it
/// missed edges. The stream is a FRESHNESS SIGNAL, not an accounting or unlock
/// ledger — a rescan/restore replays history with OLD spans, so a
/// notification-driving host MUST gate on `span_to_height > its own persisted
/// watermark` (and its FR-10 consumed-txid ledger).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IncomingFundsEvent {
    pub kind: IncomingFundsEventKind,
    /// Arrivals in THIS edge (advisory; see the struct doc).
    pub new_tx_count: u32,
    /// Cumulative `Live` detections since wallet-open (monotonic; never rewinds,
    /// even across a reorg — at-least-once counts re-detections).
    pub total_tx_detected: u64,
    /// The mined-height span of this edge's detections (`None` on `MemoRefresh`
    /// and on a count-0 `Replay`).
    pub span_from_height: Option<u32>,
    pub span_to_height: Option<u32>,
    /// Opaque versioned watermark to persist and pass back as `since_cursor`.
    /// Host treats it as OPAQUE — never parse it: a NOTIFICATION watermark (the
    /// "have I already told the user about this span?" gate) is the max
    /// `span_to_height` the host has acknowledged, tracked host-side; the
    /// cursor's only use is `since_cursor`. Every DELIVERED event carries a
    /// non-empty cursor (the pump carries the last one forward across a
    /// pre-stamp `MemoRefresh` — M2).
    pub cursor: String,
}

/// Per-transaction submit outcome (§2.5). A proposal may create SEVERAL txs
/// (pool-crossing, §1.7); every created tx is persisted BEFORE submit and
/// partial failure is first-class, never collapsed into one error.
#[derive(Clone, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum TxSubmitResult {
    Success {
        txid: TxId,
    },
    /// No endpoint verdict (a transport/timeout fault or a non-OK gRPC status —
    /// the tx MAY still have landed; the lost-ack safe direction).
    GrpcFailure {
        txid: TxId,
    },
    /// Endpoint rejected it from the mempool.
    SubmitFailure {
        txid: TxId,
        code: i32,
    },
    /// An earlier tx in the set got NO endpoint verdict (a transport/timeout fault
    /// or a non-OK gRPC status — the transport-scoped latch, #307; a mempool reject
    /// does not produce this arm); this one was not attempted. It stays persisted;
    /// the §1.7 resubmission re-drives it.
    NotAttempted {
        txid: TxId,
    },
}

/// Queryable Tor/network-privacy state (review G1 — a network-transparency UI
/// needs "is Tor active for the wallet" COLD,
/// without replaying event history). In `Dialer` mode the SDK reports
/// policy + flow only — torness ATTESTATION belongs to the host's transport;
/// the variant says which runtime is speaking so a panel can attribute the
/// claim honestly (§2.3 trust boundary).
#[derive(Clone, PartialEq, Debug)]
#[non_exhaustive]
pub enum TorState {
    /// By configuration.
    Off,
    /// The private path is coming up. `transport` is the registered
    /// descriptor's name (FR-30 (a)): `Some` only under a `HostDialer` runtime
    /// with a live registration; `None` for every other runtime — the UI then
    /// renders its transport-neutral noun, never a transport the host did not
    /// name. Redacting `Debug`, never logged (the name's own rule).
    Bootstrapping {
        percent: Option<f32>,
        transport: Option<HostTransportName>,
    },
    Active {
        runtime: TorRuntimeKind,
    },
    /// `Preferred` degraded to clearnet — visible, never silent.
    FellBack,
    /// Zero traffic: `Required` + the path unreachable, nothing registered, or
    /// the registrant declared its transport FAILED (`health`, ADR-0549).
    /// `transport` as on [`Self::Bootstrapping`].
    Unavailable {
        transport: Option<HostTransportName>,
    },
    /// Ready, and nothing has come back over it for the maintainer's minute while
    /// the wallet was trying — the path or the server (stage S1 `truth`,
    /// FR-36). The SDK reports what its own RPCs saw over connections whose
    /// arm it knows and never says WHY a ready path is not carrying: a
    /// blackholed transport and a wedged server look the same from here, and
    /// the host may be able to tell them apart from its side. `runtime` is
    /// the same payload as [`Self::Active`], so a host renders the same
    /// transport name with a different sentence. Under `Preferred` it is brief
    /// by construction — the next dial leaves for clearnet and the state reads
    /// [`Self::FellBack`]; under `Required` it stands until the path carries
    /// again. Derived from the two classes whose RPCs can confirm (sync and
    /// broadcast), never from a bare connect.
    Unanswered {
        runtime: TorRuntimeKind,
    },
}

/// How many connections each ARM served, by outcome, since this wallet opened
/// (FR-37; stage S1 `truth`). One "connection" is one `wallet.dial` line — a
/// dial outcome, not bytes (bytes by arm are not counted: the SDK cannot count
/// them honestly above a host's stream). Per wallet, in memory only, gone at
/// close; never logged; crosses the bridge only on the host's call. It carries
/// counts by arm × outcome and nothing else — no class, no host, no port.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[non_exhaustive]
pub struct DialCounts {
    /// Dials the host's transport served (a `PolicyDialer`'s primary).
    pub private: ArmCounts,
    /// Dials the SDK's own direct dialer served: every dial under
    /// `TorPolicy::Off`, and the `Preferred` fallback after the minute.
    pub clearnet: ArmCounts,
}

/// One arm's tally, one field per `outcome` value the `wallet.dial` line
/// prints, spelled the same so the number and the line cannot disagree. An
/// outcome added later is a new field and an ABI bump, which is the policy
/// already.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[non_exhaustive]
pub struct ArmCounts {
    pub connected: u64,
    pub not_ready: u64,
    pub retired: u64,
    pub unreachable: u64,
    pub timeout: u64,
    pub unsupported: u64,
    pub io: u64,
    pub transport_failed: u64,
}

/// Which §2.3 `TorRuntime` is speaking — DTO-safe for Dart: the arms carry
/// no key material and no unbounded text; the host dialer arm carries the
/// host's OWN bounded, validated display name for its transport plus two
/// CLOSED enums (FR-29 spec §0 A12/A14, ADR-0547: the SDK has no predefined
/// transport kinds — the UI renders "via your app's private path (<the
/// host's name>)", "connections can be linked by the proxy" from
/// `isolation`, and "not private" from `exposure`, without the SDK claiming
/// torness the host did not declare). The bridge mirrors the two enums and
/// carries the name as a `String`; `bridge_enums_cover_core_variants` is
/// the arbiter.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum TorRuntimeKind {
    ExternalSocks5,
    Dialer,
    /// The REGISTERED cross-library dialer (`TorRuntime::HostDialer`), with
    /// the descriptor's name, isolation and exposure as of this derivation
    /// (§3.4).
    HostDialer {
        name: HostTransportName,
        isolation: IsolationSupport,
        exposure: TransportExposure,
    },
}

/// The on-resume snapshot bundle (§3.1 `snapshot()`).
#[derive(Clone, PartialEq, Debug)]
pub struct WalletState {
    pub balance: BalanceSnapshot,
    pub sync: SyncStatus,
    /// G1 — cold-queryable, frozen pre-W3.
    pub tor: TorState,
    pub tip: Option<BlockHeight>,
    /// Balance age — always renderable. **B-2-c ships this as always-`None`** (no cold
    /// timestamp source: `BlockMetadata` carries no block time and compact blocks are
    /// deleted post-scan). It MUST be wired from a PERSISTED last-synced stamp (a DB-meta
    /// write on each completed pass) BEFORE the B-2-d FRB surface exposes it — surfacing a
    /// permanent `null` "balance age" to Dart would be an API lie. Wire it from the SAME
    /// source as [`SyncStatus::Offline`]'s `last_synced` so the two can never diverge.
    pub last_synced: Option<SyncStamp>,
    /// #357: `true` iff this wallet has reached chain tip at least once SINCE
    /// the last rescan (or ever, if none) — the DURABLE backing for the
    /// reference UI's catch-up cue (spec FR-1b). Read from the
    /// `wallet_ever_synced` aux flag, it survives a process death (the
    /// in-memory proved-tip latch did not); a rescan CLEARS it (the
    /// `sync_stamp` precedent — the rebuild is genuinely catching up again, so
    /// the cue must re-show; post-ship #357 fix), and the first post-rescan
    /// reached-tip re-sets it. Dies only with the wallet identity. UX-only,
    /// NEVER a money input: a read fault degrades to `false` (show the
    /// catch-up framing — the safe direction).
    pub ever_synced: bool,
    /// #377 s357b-2: `true` iff a rescan rebuild swapped in and has not reached
    /// tip since — the durable breadcrumb that lets the catch-up cue NAME the
    /// rebuild ("rebuilding after your rescan") across a process death, instead
    /// of the generic first-run copy. Set atomically with the rescan's DB swap,
    /// cleared at the first post-rescan reached-tip. UX-only, NEVER a money
    /// input; a read fault degrades to `false` (the generic catch-up copy).
    pub rescan_rebuilding: bool,
    /// Monotonic; stamps the snapshot AND every stream item (B-2-d). A stale stream
    /// event can never beat a fresher snapshot in the UI race (§3.3) for the DB-derived
    /// fields (`balance`, `tip`) — `snapshot()` allocates `seq` under the db lock at the
    /// instant it reads them. `tor` and `sync` ride their own independent live streams and
    /// are not ordered by `seq`.
    pub seq: u64,
}
