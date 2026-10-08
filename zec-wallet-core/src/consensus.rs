//! §3.2 of `ironwood-nu63-support.md` — THE single source of truth for "can
//! this binary transact on this network".
//!
//! **Why this module exists.** On 2026-07-28 Ironwood / NU6.3 activated at
//! mainnet block 3,428,143. Our pinned `zcash_protocol` knew upgrades only
//! through `Nu6_2`, so `BranchId::for_height()` returned the wrong consensus
//! branch for every current height, every transaction the SDK signed was
//! rejected by every node, and **nothing in the product said so** for six
//! weeks. The endpoint had been handing us `consensus_branch_id` on every
//! handshake and we dropped it on the floor. This module is the honest answer:
//! it cannot make Ironwood work (that is the crate upgrade, Phase B), but it
//! turns a silent outage into a status message, and it does that for the NEXT
//! upgrade too.
//!
//! **The load-bearing rule (spec §1.3).** The endpoint's claim is an input to
//! the REFUSAL predicate only. It can cause us to refuse to sign; it can never
//! cause us to sign differently. The branch we sign under is
//! `BranchId::for_height(compiled_params, h)` where `h` is a height WE
//! validated — the server's value is never a branch we adopt and never an
//! index into our own params.
//!
//! **Why the predicate takes two heights.** Judging at our own scanned tip
//! alone cannot see an upgrade we have not scanned up to yet; judging at the
//! endpoint's claimed tip alone is attacker-movable (an endpoint that
//! under-reports its tip moves the activation boundary out from under us and
//! a naive check reports `Current`). We judge at the HIGHEST height either
//! side attests, so an endpoint can only ever move the judgement toward more
//! scrutiny, never less.

use zcash_protocol::consensus::{
    BlockHeight as ConsensusHeight, BranchId, NetworkUpgrade, Parameters,
};

use crate::constants::{
    TARGET_BLOCK_SPACING_SECS, UNKNOWN_BRANCH_GRACE_BLOCKS, UNKNOWN_BRANCH_GRACE_SECS,
};
use crate::money::BlockHeight;
use crate::provision::ServerIdentity;
use crate::state::{GraceExpiry, UnknownBranchGrace};

/// The CLOCK half of the §6.3 grace (GRACE-1, §4p — maintainer decision
/// 2026-09-10), as read at one instant from the persisted stamp and the device
/// clock (`consensus_stamp::observe`). The block half is
/// `ConsensusCompatibility::Unknown::blocks_since_last_current`; the grace ends
/// when EITHER expires, and this is the dimension a server that freezes its
/// claimed tip cannot hold still.
///
/// A clock can only SHORTEN the grace; a time the wallet cannot trust ENDS it
/// (GRACE-2, §4v — the maintainer's decision 2026-09-10, item 1). A clock set
/// forward ends the grace early (fail-closed, one capable pass restores it); a
/// clock set back AFTER the expiry was observed changes nothing (the latch);
/// and a recorded capable time the clock cannot vouch for — LATER than now (a
/// clock moved back past it, or ahead when the verdict was taken), or `0` (a
/// pre-epoch clock at the pass) — is EXPIRED, latched the same way. GRACE-1
/// let that last cell abstain and hand the decision to the block rule, which
/// a frozen tip holds still (§4p-run review row 2). Within the readings this
/// rule can distinguish it moves in one direction only, and a capable verdict
/// is the one thing that resets it.
///
/// **What it does NOT cover (§4v-run review row 1; the maintainer's decision
/// 2026-09-11 — documented, not built): a clock that does not ADVANCE.** The
/// untrusted test is `capable_at == 0 || capable_at > now`, so a clock held
/// anywhere inside `[capable_at, capable_at + UNKNOWN_BRANCH_GRACE_SECS − 1]`
/// reads as trusted, `elapsed` never reaches the threshold, and nothing
/// latches. An attacker who runs both the lightwalletd and the device's
/// unauthenticated NTP source freezes the tip, withholds the branch id and
/// holds the clock still, and signing stays permitted indefinitely. Both of
/// this rule's axes are then held by the same party — which is exactly what
/// having two axes was meant to prevent, and it is why "at most one day of
/// silence on either axis" is not true in that cell. Closing it needs a third
/// axis neither party can freeze (monotonic uptime across passes, or a
/// high-water reading latched when the clock fails to advance over N passes).
/// See [`crate::ports::WallClock`] for the same statement at the port.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraceClock {
    /// Seconds the device clock has advanced past the last signing-capable
    /// verdict. `None` when there is nothing to measure — no capable time
    /// recorded at all (then `latched` is false and, with no anchor either,
    /// the block half refuses on its own: `NeverConfirmed`), OR the recorded
    /// capable time is one the clock cannot vouch for (later than now, or
    /// `0`). That second case is never "abstain" any more:
    /// `consensus_stamp::observe` LATCHES it (§4v), so `None` beside
    /// `latched: true` reads as expired by the clock — never negative, never
    /// "a full day left", never the base's blocks-and-no-time.
    pub elapsed_secs: Option<u64>,
    /// The G-2b latch: the clock rule has ALREADY been observed expired since
    /// the last capable verdict, by any reader of the stamp. Durable on the
    /// stamp; set by `consensus_stamp::observe` the moment it sees
    /// `elapsed_secs ≥ UNKNOWN_BRANCH_GRACE_SECS` OR a capable time the clock
    /// cannot vouch for (§4v); cleared ONLY by a capable verdict
    /// (`consensus_stamp::record`). While it holds, the clock rule is expired
    /// whatever the clock reads now — "the clock only tightens, never
    /// extends": a device whose time is set back after the expiry does not
    /// re-permit signing; a server that reports its branch does.
    pub latched: bool,
}

impl GraceClock {
    /// No clock evidence at all — nothing recorded (no capable verdict, so no
    /// capable time; with no anchor the block half refuses on its own,
    /// `NeverConfirmed`). Since GRACE-2 (§4v) this is the ONLY reading with
    /// `elapsed_secs: None` and the latch clear that a writer produces: a
    /// recorded time the clock cannot vouch for is latched, never `NONE`.
    pub(crate) const NONE: Self = Self {
        elapsed_secs: None,
        latched: false,
    };

    /// THE clock comparison (§4p item 6) — ONE site: has `elapsed_secs`
    /// reached `UNKNOWN_BRANCH_GRACE_SECS`? Exclusive at the threshold, like
    /// the block rule; `None` (nothing recorded) never has — an untrusted
    /// capable time is latched before this is asked (§4v).
    /// `consensus_stamp::observe` asks this when it WRITES the latch and
    /// [`Self::expired`] asks it when the rule READS as expired, so the two
    /// cannot drift by an off-by-one (§4p-run row 2's arch review: the
    /// comparison was written twice).
    pub(crate) fn past_threshold(elapsed_secs: Option<u64>) -> bool {
        elapsed_secs.is_some_and(|secs| secs >= UNKNOWN_BRANCH_GRACE_SECS)
    }

    /// Has the clock rule ended the grace? The latch, or a day on the device
    /// clock since the last capable verdict — exclusive at the threshold, like
    /// the block rule.
    pub(crate) fn expired(&self) -> bool {
        self.latched || Self::past_threshold(self.elapsed_secs)
    }

    /// Seconds of clock grace left: `Some(0)` once expired (the latch — which
    /// an untrusted capable time sets, §4v — or a day); `None` only with no
    /// capable time recorded at all ([`Self::NONE`]).
    pub(crate) fn secs_left(&self) -> Option<u64> {
        if self.expired() {
            return Some(0);
        }
        self.elapsed_secs
            .map(|secs| UNKNOWN_BRANCH_GRACE_SECS.saturating_sub(secs))
    }
}

/// What the SDK knows about the network's consensus rules relative to what THIS
/// BINARY can produce. Derived from (compiled params, our scanned tip, the
/// endpoint's claim); only the [`ConsensusStatusSnapshot`] that remembers the
/// last verdict is persisted (spec §10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConsensusCompatibility {
    /// Our compiled rules cover the judged height, the endpoint agrees, and we
    /// have scanned up to what it claims. Signing allowed.
    Current,

    /// Compatible at the judged height, but our scanned tip is below the
    /// endpoint's claimed tip — a wallet mid-restore or back from a long
    /// offline stretch. **Signing allowed** (existing spend-before-sync rules
    /// apply); it exists so the host never renders "this app needs an update"
    /// at someone who is merely catching up (spec §6.1).
    Behind {
        scanned_tip: BlockHeight,
        claimed_tip: BlockHeight,
    },

    /// The endpoint reports a consensus branch this binary does not know, or a
    /// branch that disagrees with what our params compute for the judged
    /// height. EITHER we are too old OR the endpoint is not on our chain — and
    /// the SDK CANNOT TELL THOSE APART, which is why there is one variant and
    /// one error code. Both are fail-closed for signing.
    ///
    /// `endpoint_branch_id` is the endpoint's raw claim, retained for display
    /// and support diagnostics ONLY. It is never fed to transaction
    /// construction, and never used as an index into our own params (§1.3).
    Unsupported {
        expected_branch_id: u32,
        endpoint_branch_id: Option<u32>,
        judged_at_height: BlockHeight,
    },

    /// The endpoint did not supply a usable `consensus_branch_id`, so we could
    /// not check. NOT `Current`: "we cannot check" is never rendered as "we
    /// checked and it is fine" (the honest-off discipline).
    ///
    /// Signing follows the GRACE rule (spec §6.3, maintainer decisions 2026-09-06
    /// and 2026-09-10): permitted while our own last signing-capable verdict is
    /// recent on BOTH axes — blocks the chain advanced AND seconds the device
    /// clock advanced — refused once EITHER ages out. See
    /// [`ConsensusCompatibility::permits_signing`].
    Unknown {
        judged_at_height: BlockHeight,
        /// How far the judged height has advanced since our last signing-capable
        /// verdict — the BLOCK rule's input. `None` when there has never been
        /// one for this wallet — outside the grace by construction.
        blocks_since_last_current: Option<u32>,
        /// The CLOCK rule's input (GRACE-1): seconds since that verdict on the
        /// device clock, and the latch. [`GraceClock::NONE`] when nothing was
        /// recorded.
        clock: GraceClock,
    },
}

impl ConsensusCompatibility {
    /// May the wallet produce a signature under this verdict?
    ///
    /// This is the ONE question every refusal site asks (spec §3.2). A second
    /// implementation of it is a bug. Both grace rules live on the data the
    /// verdict carries — computed ONCE, in [`Self::unknown_branch_grace`], and
    /// read from there by this predicate (`Running` ⇔ permit), by the gate's
    /// refusal ([`SigningPermit::granted`]) and by the surface — so a cold
    /// read of the stamp (the send gate, the drain, the parked-row flag, the
    /// sync surface — every one of them reads the row through
    /// `consensus_stamp::observe`, which fills `clock` from the port at THAT
    /// instant) answers exactly as `evaluate_consensus` does: a rule that ran
    /// only inside the pass would not be consulted by a send a day later
    /// (§4p G-13).
    pub(crate) fn permits_signing(&self) -> bool {
        match self {
            Self::Current | Self::Behind { .. } => true,
            Self::Unsupported { .. } => false,
            // The grace rule, both halves, EITHER ending it (§6.3 + GRACE-1):
            // an endpoint that withholds the branch can extend the §0 silence
            // by at most `UNKNOWN_BRANCH_GRACE_BLOCKS` of chain OR
            // `UNKNOWN_BRANCH_GRACE_SECS` of device clock, whichever comes
            // first — the clock is the axis a frozen tip cannot hold — and a
            // wallet that has never held a signing-capable verdict is outside
            // the grace by construction. The clock half can only tighten:
            // `GraceClock::expired` is false only while nothing is recorded,
            // and a recorded time the clock cannot vouch for reads expired
            // (§4v).
            //
            // ONE derivation (§4p-run row 2). The answer here and the reason
            // `granted` names used to be computed twice from the same two
            // fields, and a drift between the two computations panicked the
            // send gate at an `unreachable!` (M-r at the GRACE-1
            // adjudication). Now the grace is computed once and the permit
            // IS "still running".
            Self::Unknown { .. } => matches!(
                self.unknown_branch_grace(),
                Some(UnknownBranchGrace::Running { .. })
            ),
        }
    }

    /// The §6.3 grace as this verdict reads it — `None` for every verdict the
    /// grace does not govern (`Current`/`Behind` sign, `Unsupported` refuses,
    /// neither on a grace). For `Unknown`: what remains on each rule while
    /// both hold, or which rule ended it. THE derivation of the grace, from
    /// the two fields the verdict carries: [`Self::permits_signing`] reads its
    /// `Unknown` answer here (`Running` ⇔ permit) and
    /// [`SigningPermit::granted`] reads its refusal's reason here, so the
    /// surface, the gate and the parked-row flag cannot disagree — there is
    /// no second computation to disagree with (§4p-run row 2).
    ///
    /// When both rules have expired, `Blocks` is named — the block evidence
    /// alone suffices, and "check the device time" would be a wrong hint beside
    /// it (`GraceExpiry`'s doc).
    pub(crate) fn unknown_branch_grace(&self) -> Option<UnknownBranchGrace> {
        let Self::Unknown {
            blocks_since_last_current,
            clock,
            ..
        } = self
        else {
            return None;
        };
        let Some(blocks) = *blocks_since_last_current else {
            return Some(UnknownBranchGrace::Ended {
                by: GraceExpiry::NeverConfirmed,
                blocks_since_last_current: None,
            });
        };
        if blocks >= UNKNOWN_BRANCH_GRACE_BLOCKS {
            return Some(UnknownBranchGrace::Ended {
                by: GraceExpiry::Blocks,
                blocks_since_last_current: Some(blocks),
            });
        }
        if clock.expired() {
            return Some(UnknownBranchGrace::Ended {
                by: GraceExpiry::Clock,
                blocks_since_last_current: Some(blocks),
            });
        }
        let blocks_left = UNKNOWN_BRANCH_GRACE_BLOCKS - blocks;
        // "Whichever expires first" (§4p G-5), decided HERE through the ONE
        // conversion (`TARGET_BLOCK_SPACING_SECS`, item 5) so no host has to
        // know the block spacing: the time shown is the smaller of the clock
        // rule's remainder and the block rule's remainder in seconds. `None`
        // only when no capable time was recorded at all (`GraceClock::NONE`)
        // — which, beside an anchor, no writer produces (`record` writes the
        // two together); since GRACE-2 (§4v) a capable time the clock cannot
        // vouch for is latched and reaches `Ended` above, never a
        // blocks-and-no-time `Running`. The `Option` stays for the bridge's
        // shape and that one unwritten row.
        let secs_left = clock.secs_left().map(|clock_secs| {
            let block_secs = u64::from(blocks_left) * TARGET_BLOCK_SPACING_SECS;
            surface_secs(clock_secs.min(block_secs))
        });
        Some(UnknownBranchGrace::Running {
            blocks_left,
            secs_left,
        })
    }

    /// Does this verdict mean we could not fully READ the blocks we scanned?
    ///
    /// A different question from [`Self::permits_signing`], and deliberately a
    /// narrower one: an `Unknown` past its grace refuses to SIGN (we cannot
    /// confirm the rules) but is no evidence at all that we misread a block —
    /// a server omitting one protocol field says nothing about block contents.
    /// Claiming otherwise would put a false statement about the user's money in
    /// the very surface built to stop false statements.
    ///
    /// Lives here, beside its sibling, so all three readers of "is this wallet
    /// blocked" (the signing gate, the sync surface, the parked-row flag) ask
    /// `ConsensusCompatibility` rather than re-deriving it with an inline
    /// `matches!` that a new variant would silently miss (code reviewer).
    pub(crate) fn blocks_interpretation(&self) -> bool {
        matches!(self, Self::Unsupported { .. })
    }

    /// WHY the queued-send drain will not sign under this verdict, if it will
    /// not — the parked-row flag's value (GRACE-1, §4p item 2), derived HERE
    /// from the same predicate the gate asks so the row and the drain can never
    /// disagree: `None` ⇔ [`Self::permits_signing`]. `Unsupported` is the
    /// app-update block; an ended grace is the switch-servers block with its
    /// reason. (A wallet with NO verdict reads `None` at the surface — the
    /// first pass evaluates before it drains, so "will send on its own" is
    /// true there; see `SigningBlock`.)
    pub(crate) fn signing_block(&self) -> Option<crate::state::SigningBlock> {
        if self.permits_signing() {
            return None;
        }
        if self.blocks_interpretation() {
            return Some(crate::state::SigningBlock::NetworkUpgrade);
        }
        match self.unknown_branch_grace() {
            Some(UnknownBranchGrace::Ended { by, .. }) => {
                Some(crate::state::SigningBlock::GraceExpired { by })
            }
            // `permits_signing()` false ⇔ `Ended` for an `Unknown`; `Current`/
            // `Behind` permit. Fail-CLOSED on the surface if that ever drifts:
            // a refused row must never pose as a healthy pending send.
            Some(UnknownBranchGrace::Running { .. }) | None => {
                Some(crate::state::SigningBlock::GraceExpired {
                    by: GraceExpiry::NeverConfirmed,
                })
            }
        }
    }

    /// Is this a verdict that should refresh the snapshot's "last known good"
    /// anchor? Only a positive check counts — an `Unknown` inside its grace is
    /// riding the PREVIOUS anchor and must never renew it, or a silent endpoint
    /// would extend its own grace forever.
    pub(crate) fn is_signing_capable_check(&self) -> bool {
        matches!(self, Self::Current | Self::Behind { .. })
    }
}

/// The remaining seconds the SURFACE carries, narrowed to the bridge's `u32`.
/// ≤ `UNKNOWN_BRANCH_GRACE_SECS` (86,400) by construction — both operands of
/// the `min` in `unknown_branch_grace` are bounded by it — so the narrowing
/// never fails today; if the arithmetic it does not trust ever overflowed, it
/// saturates toward LESS grace (`0`, "none left"), never toward `u32::MAX`
/// ("~68,000 hours left" — the base's direction, §4v G2-6 / §4p-run review
/// row 8). Pinned with an out-of-range input by
/// `the_surface_seconds_saturate_toward_no_grace_left`.
fn surface_secs(secs: u64) -> u32 {
    u32::try_from(secs).unwrap_or(0)
}

/// Every network upgrade THIS BUILD knows about, oldest first.
///
/// Hand-maintained because upstream's own ordered list is private — and kept
/// honest by `known_upgrades_is_really_complete`, which fails the moment the
/// pinned crate learns an upgrade this list does not name. A list that goes
/// quietly stale is the whole shape of the Ironwood outage.
const KNOWN_UPGRADES: &[NetworkUpgrade] = &[
    NetworkUpgrade::Overwinter,
    NetworkUpgrade::Sapling,
    NetworkUpgrade::Blossom,
    NetworkUpgrade::Heartwood,
    NetworkUpgrade::Canopy,
    NetworkUpgrade::Nu5,
    NetworkUpgrade::Nu6,
    NetworkUpgrade::Nu6_1,
    NetworkUpgrade::Nu6_2,
    // NU6.3 / Ironwood — added with the crate wave that gave this build the rules
    // to model it (`ironwood-phase-b.md` §1 step 6). Adding it does TWO things at
    // once, and the second is the money-relevant one:
    //   1. `evaluate_consensus` stops calling a post-3,428,143 chain `Unsupported`,
    //      which is what restores signing — the outage itself.
    //   2. It RAISES THE ANCHOR CEILING from Nu6_2's 3,364,600 to Nu6_3's
    //      3,428,143. Measured effect, because the obvious guess is wrong: this
    //      does NOT make the 13 bundled rows above the Ironwood activation
    //      anchorable — `bundled_treestate` picks the newest row strictly below
    //      `ceiling + 1` and the first of those rows is 3,429,810, still above it.
    //      The ceiling stays conservative on purpose: past the newest activation we
    //      know, an upgrade we do NOT know may be live, which is as true of NU7
    //      today as it was of NU6.3 yesterday. What actually moves is the newest
    //      anchorable row, 3,362,500 -> 3,427,310 — 26 further rows and ~64,810
    //      fewer blocks of first scan — and every one of them is BELOW the
    //      activation, so an empty Ironwood frontier is correct there. Pinned by
    //      `the_anchor_ceiling_is_the_newest_upgrade_this_build_can_model`.
    NetworkUpgrade::Nu6_3,
];

/// The activation height of the newest upgrade this build knows — the highest
/// height whose consensus rules we can actually model.
///
/// **This is the ceiling for a bundled birthday anchor** (spec §1.5 / §9.2).
/// Above it, an upgrade we have never heard of may have activated, and the
/// chain state at such a height can contain a value pool we cannot represent.
/// Anchoring there bakes an implicitly-empty tree for that pool into the
/// wallet — persisted, immutable after first import, and NOT OBVIOUSLY repairable
/// in place
/// once a later build learns the pool. That is exactly what the 13 bundled
/// mainnet rows above the Ironwood activation would have done.
///
/// The ceiling is derived from what the compiled params know, not a literal —
/// but it does **NOT** raise itself at a crate bump, and an earlier version of
/// this very comment said it did (the claim spread from here into four
/// documents). `KNOWN_UPGRADES` above is HAND-MAINTAINED: the NU6.3 crate wave
/// landing did nothing to the ceiling on its own; adding the `Nu6_3` variant to
/// that list is what moved it, in the same commit, deliberately. The list is
/// kept honest by `known_upgrades_is_really_complete`, which fails the moment
/// the pinned crate learns an upgrade the list does not name — that test is the
/// forcing function, not the bump.
pub(crate) fn newest_known_activation<P: Parameters>(params: &P) -> Option<ConsensusHeight> {
    KNOWN_UPGRADES
        .iter()
        .rev()
        .find_map(|nu| params.activation_height(*nu))
}

/// The cold-read answer (spec §3.3) — what a host can render without
/// attempting a send, and the honest offline story.
///
/// `verdict: None` means this wallet has NEVER evaluated one (a fresh install
/// that has not reached a server, or a row we could not interpret). That is a
/// real state, distinct from every verdict, and it does not permit signing —
/// fabricating `Current` there is exactly the §0 failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ConsensusStatusSnapshot {
    pub verdict: Option<ConsensusCompatibility>,
    /// The height the last verdict that permitted signing was judged at,
    /// bounded into `[scanned, scanned + REORG_MAX_BLOCKS]` ([`grace_anchor`];
    /// T0-1c-R2 M4) — the anchor the §6.3 grace's BLOCK rule is measured from.
    pub grace_anchor_tip: Option<BlockHeight>,
    /// Wall-clock seconds at evaluation of the LATEST verdict (of any kind), so
    /// the host can render the age. `None` only if a stored value was out of
    /// range. NOT the clock rule's anchor — that is the capable time, which
    /// stays inside the core (§4p G-12) and reaches this snapshot only as
    /// [`Self::grace_clock`]'s elapsed reading.
    pub evaluated_at_unix: Option<u64>,
    /// The §6.3 grace's CLOCK rule as of the instant this snapshot was read
    /// (GRACE-1): seconds since the last capable verdict on the device clock,
    /// and the latch — the anchor's sibling for the second axis. Every
    /// `ConsensusCompatibility::Unknown` this wallet evaluates or reads back
    /// carries it, which is how the send gate a day after the last pass sees
    /// the day (§4p G-13).
    pub grace_clock: GraceClock,
}

impl ConsensusStatusSnapshot {
    /// How far `judged_tip` has advanced past the grace anchor, saturating at
    /// zero. `None` when there is no anchor — outside the grace by
    /// construction (§6.3). Fed the JUDGED height, `max(scanned, claimed)`
    /// (§4p item 4): a claim below what this wallet scanned itself cannot
    /// shrink the count.
    pub(crate) fn blocks_since_last_current(&self, judged_tip: BlockHeight) -> Option<u32> {
        self.grace_anchor_tip
            .map(|anchor| judged_tip.value().saturating_sub(anchor.value()))
    }
}

/// Proof that the staleness predicate was consulted and permitted signing.
///
/// **This type is the gate.** It is constructible only from a verdict that
/// permits signing ([`SigningPermit::granted`]), and [`crate::send::
/// create_signed_core`] — the ONE choke point every signature in the SDK passes
/// through — takes it by value. So the compiler enforces what a review
/// checklist could not: the interactive send, the queued drain, the sweep and
/// the reclaim-mint cannot reach a signature without having asked.
///
/// The draft spec gated `propose()` instead. Three of the four signing paths do
/// not go through `propose()`, and the queued drain — the offline-first path —
/// is one of them; gating there would have reproduced the §0 incident on the
/// exact flow users hit when connectivity is poor.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SigningPermit {
    /// Private field, so no other module can construct one structurally.
    _consulted: (),
}

impl SigningPermit {
    /// Grant a permit, or return the typed refusal the host renders — ONE of
    /// three, distinguishable BY TYPE on the bridge and in Dart (§4p G-1),
    /// because each has a different next step and rendering any two alike was
    /// the defect: "update the app" for a server that merely stopped talking
    /// is false, and an update fixes nothing.
    ///
    /// `None` — this wallet has never evaluated a verdict — REFUSES with
    /// `ConsensusNotEvaluated`. There is no honest reading of "we have never
    /// checked" that permits signing; and it is not "the network was upgraded"
    /// either — it is "not synced yet", which the first completed pass resolves
    /// (INC-004: a pass evaluates before it scans).
    ///
    /// **No panic path** (§4p-run row 2). The answer is `permits_signing()`
    /// and the reason is `unknown_branch_grace()` — the one derivation, read
    /// twice, never recomputed here. The base guarded their agreement with
    /// two `unreachable!`s, and under a drift (M-r at the GRACE-1
    /// adjudication) the send gate PANICKED on ten rows instead of refusing.
    /// A money path answers a defect by refusing with a typed error, never by
    /// crashing: every arm below that "cannot happen" is a refusal.
    pub(crate) fn granted(
        verdict: Option<&ConsensusCompatibility>,
    ) -> Result<Self, crate::error::WalletError> {
        let Some(verdict) = verdict else {
            return Err(crate::error::WalletError::ConsensusNotEvaluated);
        };
        if verdict.permits_signing() {
            return Ok(Self { _consulted: () });
        }
        if let ConsensusCompatibility::Unsupported {
            expected_branch_id,
            endpoint_branch_id,
            judged_at_height,
        } = verdict
        {
            return Err(crate::error::WalletError::NetworkUpgradeUnsupported {
                expected_branch_id: *expected_branch_id,
                endpoint_branch_id: *endpoint_branch_id,
                judged_at_height: judged_at_height.value(),
            });
        }
        // Refused, and not by the build: an `Unknown` past its grace refuses
        // with ITS OWN variant naming the rule that ended the grace — the
        // honest story is that the server would not say which network it is
        // on (§6.3), and the next step is "switch servers" (plus "check the
        // device time" for the clock), never "update the app".
        match verdict.unknown_branch_grace() {
            Some(UnknownBranchGrace::Ended {
                by,
                blocks_since_last_current,
            }) => Err(crate::error::WalletError::ConsensusGraceExpired {
                by,
                blocks_since_last_current,
            }),
            // `permits_signing()` false ⇔ `Ended` for an `Unknown`, and the
            // grace reads `None` only for a verdict that permits. Either
            // reading here is a code defect — and it REFUSES, the way
            // `signing_block` does: a refused send must never pose as a
            // permit, and a money path must never panic.
            Some(UnknownBranchGrace::Running { .. }) | None => {
                Err(crate::error::WalletError::ConsensusGraceExpired {
                    by: GraceExpiry::NeverConfirmed,
                    blocks_since_last_current: None,
                })
            }
        }
    }

    /// Test-only permit. Named to be loud in a diff: any appearance of this
    /// outside `#[cfg(test)]` is a review blocker.
    #[cfg(test)]
    pub(crate) fn assume_current_for_test() -> Self {
        Self { _consulted: () }
    }
}

/// Parse the endpoint's `consensus_branch_id` — lightwalletd reports it as a
/// hex STRING (proto tag 5). §4.6: every byte from the network is hostile, so
/// this is total and allocation-free. `None` for absent, empty, over-long or
/// non-hex input — an old or non-conforming server, never a panic.
///
/// NOTE: prost renders an ABSENT tag-5 string and a present-but-empty one
/// identically as `""`. Both are `None` here, and spec §6.3 owns what that
/// means. We cannot distinguish them and do not pretend to.
pub(crate) fn parse_branch_id(raw: &str) -> Option<u32> {
    let trimmed = raw
        .strip_prefix("0x")
        .or_else(|| raw.strip_prefix("0X"))
        .unwrap_or(raw);
    // A branch id is exactly 4 bytes. Anything longer is not a short read of a
    // valid value, it is a different thing — reject rather than truncate.
    if trimmed.is_empty() || trimmed.len() > 8 {
        return None;
    }
    u32::from_str_radix(trimmed, 16).ok()
}

/// THE staleness predicate (spec §3.2). Pure — no IO, no store, and no clock
/// READ: the device clock enters as an INPUT (`clock`, read by the caller
/// through the `WallClock` port), so every upgrade boundary and every grace
/// geometry is unit-testable without a server or a real clock.
///
/// `blocks_since_last_current` and `clock` come from the persisted snapshot as
/// observed now and are only consulted for the `Unknown` arm (the §6.3 grace,
/// both rules).
pub(crate) fn consensus_compatibility<P: Parameters>(
    params: &P,
    scanned_tip: BlockHeight,
    claimed_tip: BlockHeight,
    endpoint: &ServerIdentity,
    blocks_since_last_current: Option<u32>,
    clock: GraceClock,
) -> ConsensusCompatibility {
    // Judge at the highest height either side attests. An endpoint that
    // under-reports its tip cannot pull the judgement below blocks we have
    // already scanned, so lying about the height only ever costs it.
    let judged_at = BlockHeight::new(scanned_tip.value().max(claimed_tip.value()));
    let expected = BranchId::for_height(params, ConsensusHeight::from_u32(judged_at.value()));
    let expected_raw = u32::from(expected);

    let Some(claimed) = endpoint.consensus_branch_id else {
        return ConsensusCompatibility::Unknown {
            judged_at_height: judged_at,
            blocks_since_last_current,
            clock,
        };
    };

    // Two ways to fail, one verdict (§2): a branch this build has never heard
    // of (we are old), or a known branch that disagrees with our rules at the
    // judged height (we are on different chains). The SDK cannot tell them
    // apart from inside, so it does not invent a distinction.
    if BranchId::try_from(claimed).is_err() || claimed != expected_raw {
        return ConsensusCompatibility::Unsupported {
            expected_branch_id: expected_raw,
            endpoint_branch_id: Some(claimed),
            judged_at_height: judged_at,
        };
    }

    if scanned_tip.value() < claimed_tip.value() {
        ConsensusCompatibility::Behind {
            scanned_tip,
            claimed_tip,
        }
    } else {
        ConsensusCompatibility::Current
    }
}

/// THE grace anchor (spec §6.3), pure: the height a signing-capable verdict is
/// recorded at, bounded into `[scanned_tip, scanned_tip + REORG_MAX_BLOCKS]`.
///
/// Two bounds, from two incidents. The UPPER bound is the clamp (INC-008):
/// unclamped, an endpoint claiming `block_height = u64::MAX` earns `Behind`
/// (its branch matches ours at any height past the last activation) and pins
/// the anchor at `u32::MAX` for the life of the row, after which every
/// `Unknown` computes a grace of zero and signs forever. The LOWER bound is
/// T0-1c-R2 M4 (§4n Q-G3, the wrap's crypto #9): the anchor was
/// `min(claimed, scanned + margin)` with no floor, so a validator stuck between
/// the last activation and the bundle's row — reporting the correct branch and
/// a height 22 k below what this wallet had scanned — earned `Current` and
/// LOWERED the persisted anchor by 22 k; the next server that omitted
/// `consensus_branch_id` then computed a grace above `UNKNOWN_BRANCH_GRACE_BLOCKS`
/// and signing was refused with "update the app" — fail-closed, one capable
/// pass restores it, and still an untrusted number moving a durable
/// money-path gate. Anchoring on the JUDGED height closes it: the verdict is
/// taken at `max(scanned, claimed)` (the predicate above), so the anchor is
/// that height, capped by the reorg margin — `clamp(claimed, scanned, scanned +
/// margin)`, which is `min(max(scanned, claimed), scanned + margin)`. The
/// alternative the contract priced — a MONOTONE anchor, `max(previous, …)` —
/// was not taken: it would hold the anchor above the wallet's own height after
/// a real rewind or a rescan (a persisted number the wallet no longer holds
/// evidence for, by up to a margin plus the rewind depth — the fail-OPEN
/// direction, even if bounded), and it needs the previous row to compute. This
/// shape needs only the pass's two heights and keeps the invariant exact.
///
/// The rewind case, stated: a real rewind lowers `scanned_tip`; the next
/// capable pass records an anchor lower than the previous by at most the
/// rewind depth (bounded by `REORG_MAX_BLOCKS` per pass — the storm guard), so
/// the grace a later `Unknown` computes grows by that much: fail-CLOSED, and a
/// capable pass repairs it. A rescan drops `scanned_tip` to the birthday, so
/// the first capable pass after it records `birthday + margin` and the anchor
/// climbs with the scan — the pre-R2 behaviour, unchanged.
///
/// Saturating, so a wallet scanned to within a margin of `u32::MAX` cannot
/// panic the clamp (`scanned ≤ scanned.saturating_add(margin)` always holds).
pub(crate) fn grace_anchor(scanned_tip: BlockHeight, claimed_tip: BlockHeight) -> BlockHeight {
    let floor = scanned_tip.value();
    let ceiling = floor.saturating_add(crate::constants::REORG_MAX_BLOCKS);
    BlockHeight::new(claimed_tip.value().clamp(floor, ceiling))
}

#[cfg(test)]
mod tests {
    use super::*;
    use zcash_protocol::consensus::MAIN_NETWORK;

    /// T0-1c-R2 (§4n G4, M4) — the anchor at its bounds, pure: a claim below
    /// what the wallet scanned anchors AT the scanned height (the lying-low
    /// validator cannot lower it); a claim inside the margin anchors at the
    /// claim (the honest case — the judged height); a claim above the margin
    /// anchors at `scanned + margin` (INC-008, the clamp); the extremes
    /// saturate. Mutants: the floor removed (`min(claimed, scanned + margin)`,
    /// the base — the first assertion red); the ceiling removed (the third and
    /// fourth red); the margin doubled (the third red).
    #[test]
    fn the_grace_anchor_is_the_judged_height_bounded_by_the_reorg_margin() {
        use crate::constants::REORG_MAX_BLOCKS;
        let scanned = BlockHeight::new(3_477_824);
        let h = |v: u32| BlockHeight::new(v);
        assert_eq!(
            grace_anchor(scanned, h(3_455_000)),
            scanned,
            "a claim 22k below our own height anchors at our own height, not the claim"
        );
        assert_eq!(
            grace_anchor(scanned, h(3_477_824 + 40)),
            h(3_477_824 + 40),
            "a claim inside the margin anchors at the judged height"
        );
        assert_eq!(
            grace_anchor(scanned, h(3_477_824 + REORG_MAX_BLOCKS + 1)),
            h(3_477_824 + REORG_MAX_BLOCKS),
            "a claim past the margin is capped at scanned + margin (INC-008)"
        );
        assert_eq!(
            grace_anchor(scanned, h(u32::MAX)),
            h(3_477_824 + REORG_MAX_BLOCKS),
            "u32::MAX cannot pin the anchor"
        );
        assert_eq!(
            grace_anchor(h(0), h(3_477_824)),
            h(REORG_MAX_BLOCKS),
            "a wallet that scanned nothing anchors at most a margin above zero"
        );
        assert_eq!(
            grace_anchor(h(u32::MAX - 10), h(u32::MAX)),
            h(u32::MAX),
            "the ceiling saturates rather than panicking the clamp"
        );
    }

    /// Mainnet Ironwood / NU6.3 activation, READ FROM THE CRATE (Phase B step 7).
    ///
    /// It was a literal until the NU6.3 wave, because the pinned `zcash_protocol`
    /// had no `Nu6_3` to read it from — and that absence WAS the outage. Now that
    /// the crate knows it, a literal here would be a second source of truth for a
    /// consensus constant, which is the shape the outage came in. The value is
    /// pinned against the number the ADR, the spec and the bundle all use by
    /// `checkpoint_activation_heights_match_zcash_protocol` below.
    fn ironwood_mainnet() -> u32 {
        u32::from(
            MAIN_NETWORK
                .activation_height(NetworkUpgrade::Nu6_3)
                .expect("the pinned params know Nu6_3 — that is what this wave bought"),
        )
    }

    /// `BranchId::Nu6_3`, likewise read from the crate rather than written down.
    fn nu6_3_branch() -> u32 {
        u32::from(BranchId::for_height(
            &MAIN_NETWORK,
            ConsensusHeight::from_u32(ironwood_mainnet()),
        ))
    }

    /// The branch of the upgrade BEFORE Ironwood, derived rather than pinned, so
    /// these fixtures keep meaning "one epoch back" at the next wave too.
    fn previous_branch() -> u32 {
        u32::from(BranchId::for_height(
            &MAIN_NETWORK,
            ConsensusHeight::from_u32(ironwood_mainnet() - 1),
        ))
    }

    /// A branch id this build does NOT implement — the role `NU6_3_BRANCH` played
    /// before the wave, now filled by a value chosen to have no upgrade behind it.
    ///
    /// The outage tests below are about a branch we cannot model, NOT about
    /// Ironwood specifically; writing them against Ironwood is what made every one
    /// of them invert the moment we learned it. Expressed this way they survive
    /// NU7 and everything after. `unknown_branch_fixture_is_really_unknown` is the
    /// non-vacuity guard: if a future crate ever learns this value, it fails and
    /// says to pick another.
    const UNKNOWN_FUTURE_BRANCH: u32 = 0xdead_beef;

    fn endpoint(branch: Option<u32>) -> ServerIdentity {
        ServerIdentity {
            chain_name: "main".to_owned(),
            sapling_activation_height: 419_200,
            consensus_branch_id: branch,
            block_height: 0,
        }
    }

    /// The block-rule fixtures: the clock ABSTAINS (`GraceClock::NONE`), so
    /// every row below exercises the block half alone — the shape the base's
    /// tests had, unchanged in meaning now that the clock is a second input.
    fn verdict(
        scanned: u32,
        claimed: u32,
        branch: Option<u32>,
        since: Option<u32>,
    ) -> ConsensusCompatibility {
        verdict_at(scanned, claimed, branch, since, GraceClock::NONE)
    }

    /// The two-rule fixture (GRACE-1): a verdict with the clock half supplied.
    fn verdict_at(
        scanned: u32,
        claimed: u32,
        branch: Option<u32>,
        since: Option<u32>,
        clock: GraceClock,
    ) -> ConsensusCompatibility {
        consensus_compatibility(
            &MAIN_NETWORK,
            BlockHeight::new(scanned),
            BlockHeight::new(claimed),
            &endpoint(branch),
            since,
            clock,
        )
    }

    fn elapsed(secs: u64) -> GraceClock {
        GraceClock {
            elapsed_secs: Some(secs),
            latched: false,
        }
    }

    /// The day is the block grace in seconds, from ONE constant (§4p item 5) —
    /// a literal "86_400" or a second "75" anywhere in the grace fails here.
    #[test]
    fn the_clock_grace_is_the_block_grace_through_the_one_spacing_constant() {
        assert_eq!(
            UNKNOWN_BRANCH_GRACE_SECS,
            u64::from(UNKNOWN_BRANCH_GRACE_BLOCKS) * TARGET_BLOCK_SPACING_SECS
        );
        assert_eq!(
            UNKNOWN_BRANCH_GRACE_SECS, 86_400,
            "1,152 × 75 s is one day exactly"
        );
    }

    /// GRACE-1 (§4p Q-G1, G-10's "either made both"): the grace ends when EITHER
    /// rule expires. Blocks inside + clock outside → refused, by the clock;
    /// blocks outside + clock inside → refused, by blocks; both inside →
    /// permitted, `Running`. Mutants (in `unknown_branch_grace`, the one
    /// derivation `permits_signing` reads since §4p-run row 2): the
    /// `clock.expired()` check dropped (the first refusal green — the base);
    /// the two expiry checks fused into one "both must expire" — the first
    /// two permit; the block check dropped — the second permits.
    #[test]
    fn the_grace_ends_when_either_rule_expires() {
        let inside_blocks = Some(UNKNOWN_BRANCH_GRACE_BLOCKS - 1);
        let outside_blocks = Some(UNKNOWN_BRANCH_GRACE_BLOCKS);

        let by_clock = verdict_at(
            3_300_000,
            3_300_000,
            None,
            inside_blocks,
            elapsed(UNKNOWN_BRANCH_GRACE_SECS),
        );
        assert!(
            !by_clock.permits_signing(),
            "a day on the device clock ends the grace even while the server admits no blocks"
        );
        assert_eq!(
            by_clock.unknown_branch_grace(),
            Some(UnknownBranchGrace::Ended {
                by: GraceExpiry::Clock,
                blocks_since_last_current: inside_blocks,
            })
        );

        let by_blocks = verdict_at(
            3_300_000,
            3_300_000,
            None,
            outside_blocks,
            elapsed(UNKNOWN_BRANCH_GRACE_SECS - 1),
        );
        assert!(
            !by_blocks.permits_signing(),
            "a day of blocks ends the grace even while the clock says less"
        );
        assert_eq!(
            by_blocks.unknown_branch_grace(),
            Some(UnknownBranchGrace::Ended {
                by: GraceExpiry::Blocks,
                blocks_since_last_current: outside_blocks,
            })
        );

        let inside = verdict_at(
            3_300_000,
            3_300_000,
            None,
            inside_blocks,
            elapsed(UNKNOWN_BRANCH_GRACE_SECS - 1),
        );
        assert!(inside.permits_signing(), "inside both rules signs");
        assert_eq!(
            inside.unknown_branch_grace(),
            Some(UnknownBranchGrace::Running {
                blocks_left: 1,
                secs_left: Some(1),
            })
        );
        // Exclusive at the threshold on the clock too, like the block rule.
        assert!(
            !verdict_at(
                3_300_000,
                3_300_000,
                None,
                Some(0),
                elapsed(UNKNOWN_BRANCH_GRACE_SECS)
            )
            .permits_signing(),
            "the clock grace is exclusive at the threshold"
        );
    }

    /// GRACE-2 (§4v G2-4's pure half): the reading `consensus_stamp::observe`
    /// hands the predicate for a capable time the clock cannot vouch for —
    /// later than now, or `0` — is `{ elapsed_secs: None, latched: true }`,
    /// and the predicate reads it as the grace ENDED `by: Clock` ten blocks
    /// in, the surface's time `Some(0)` — never the base's `Running {
    /// secs_left: None }` (blocks-and-no-time); past the block threshold too,
    /// `Blocks` is named (both expired). The one reading left with
    /// `elapsed_secs: None` and the latch clear is `NONE` — no capable time
    /// recorded: with no anchor it is `NeverConfirmed`; with an anchor (a row
    /// no writer produces — `record` writes the two together) the block rule
    /// decides alone, which is the `Option` on `secs_left`'s one remaining
    /// meaning, pinned here so the residual is named rather than assumed.
    /// Mutant: `latched ||` dropped from `GraceClock::expired` — the first
    /// cell permits with blocks-and-no-time.
    #[test]
    fn an_untrusted_capable_time_reads_as_ended_by_the_clock_never_as_blocks_and_no_time() {
        let untrusted = GraceClock {
            elapsed_secs: None,
            latched: true,
        };
        let ten_in = verdict_at(3_300_000, 3_300_000, None, Some(10), untrusted);
        assert!(
            !ten_in.permits_signing(),
            "an untrusted capable time ends the grace: {ten_in:?}"
        );
        assert_eq!(
            ten_in.unknown_branch_grace(),
            Some(UnknownBranchGrace::Ended {
                by: GraceExpiry::Clock,
                blocks_since_last_current: Some(10),
            }),
            "ended by the clock — never `Running {{ secs_left: None }}`"
        );
        assert_eq!(
            untrusted.secs_left(),
            Some(0),
            "the surface's time is \"none left\", not \"no time\""
        );
        let past_blocks = verdict_at(
            3_300_000,
            3_300_000,
            None,
            Some(UNKNOWN_BRANCH_GRACE_BLOCKS + 5),
            untrusted,
        );
        assert_eq!(
            past_blocks.unknown_branch_grace(),
            Some(UnknownBranchGrace::Ended {
                by: GraceExpiry::Blocks,
                blocks_since_last_current: Some(UNKNOWN_BRANCH_GRACE_BLOCKS + 5),
            }),
            "both expired: blocks named"
        );
        // The residual `None`-and-clear reading: nothing recorded.
        assert_eq!(GraceClock::NONE.secs_left(), None);
        assert_eq!(
            verdict_at(3_300_000, 3_300_000, None, None, GraceClock::NONE).unknown_branch_grace(),
            Some(UnknownBranchGrace::Ended {
                by: GraceExpiry::NeverConfirmed,
                blocks_since_last_current: None,
            }),
            "no anchor and no time: never confirmed"
        );
        assert_eq!(
            verdict_at(3_300_000, 3_300_000, None, Some(10), GraceClock::NONE)
                .unknown_branch_grace(),
            Some(UnknownBranchGrace::Running {
                blocks_left: UNKNOWN_BRANCH_GRACE_BLOCKS - 10,
                secs_left: None,
            }),
            "an anchor with no readable time (no writer produces it): blocks alone — \
             the Option's one remaining meaning"
        );
    }

    /// GRACE-2 (§4v G2-6, §4p-run review row 8): the surface's remaining
    /// seconds narrow to the bridge's `u32` saturating toward LESS grace — an
    /// out-of-range input reads `0`, never `u32::MAX` ("~68,000 hours left").
    /// No consumer can produce that input (both operands of the `min` in
    /// `unknown_branch_grace` are bounded by `UNKNOWN_BRANCH_GRACE_SECS`, and
    /// that bound is pinned here too), so the helper is pinned directly.
    /// Mutant: `unwrap_or(0)` reverted to `unwrap_or(u32::MAX)`.
    #[test]
    fn the_surface_seconds_saturate_toward_no_grace_left() {
        assert_eq!(
            surface_secs(u64::from(u32::MAX) + 1),
            0,
            "an overflow reads as none left, never ~68,000 hours"
        );
        assert_eq!(surface_secs(u64::MAX), 0);
        assert_eq!(
            surface_secs(UNKNOWN_BRANCH_GRACE_SECS),
            86_400,
            "in range passes through"
        );
        assert!(
            UNKNOWN_BRANCH_GRACE_SECS <= u64::from(u32::MAX),
            "why no consumer reaches the saturation: both operands of the min are bounded by the day"
        );
    }

    /// GRACE-1 (§4p G-2b's pure half, item 6): the latch refuses whatever the
    /// clock reads NOW — a clock set back inside the day after the expiry was
    /// observed does not re-permit. Mutant: `latched ||` dropped from
    /// `GraceClock::expired` — permits.
    #[test]
    fn the_clock_latch_refuses_whatever_the_clock_reads_now() {
        let set_back = GraceClock {
            elapsed_secs: Some(60),
            latched: true,
        };
        let v = verdict_at(3_300_000, 3_300_000, None, Some(0), set_back);
        assert!(
            !v.permits_signing(),
            "the clock only tightens, never extends"
        );
        assert_eq!(
            v.unknown_branch_grace(),
            Some(UnknownBranchGrace::Ended {
                by: GraceExpiry::Clock,
                blocks_since_last_current: Some(0),
            })
        );
        let even_before = GraceClock {
            elapsed_secs: None,
            latched: true,
        };
        assert!(
            !verdict_at(3_300_000, 3_300_000, None, Some(0), even_before).permits_signing(),
            "nor does a clock set back past the capable time"
        );
    }

    /// GRACE-1: when BOTH rules have expired the reason is `Blocks` — the block
    /// evidence suffices, and "check the device time" would be a wrong hint
    /// beside it. And a wallet that never held a capable verdict is
    /// `NeverConfirmed`, not an expiry of either rule.
    #[test]
    fn the_expiry_names_blocks_when_both_rules_expired_and_never_when_nothing_was_confirmed() {
        let both = verdict_at(
            3_300_000,
            3_300_000,
            None,
            Some(UNKNOWN_BRANCH_GRACE_BLOCKS * 2),
            elapsed(UNKNOWN_BRANCH_GRACE_SECS * 2),
        );
        assert_eq!(
            both.unknown_branch_grace(),
            Some(UnknownBranchGrace::Ended {
                by: GraceExpiry::Blocks,
                blocks_since_last_current: Some(UNKNOWN_BRANCH_GRACE_BLOCKS * 2),
            })
        );
        let never = verdict_at(3_300_000, 3_300_000, None, None, elapsed(0));
        assert!(!never.permits_signing());
        assert_eq!(
            never.unknown_branch_grace(),
            Some(UnknownBranchGrace::Ended {
                by: GraceExpiry::NeverConfirmed,
                blocks_since_last_current: None,
            })
        );
        // Off the grace entirely: the verdicts the rule does not govern.
        assert_eq!(ConsensusCompatibility::Current.unknown_branch_grace(), None);
        assert_eq!(
            verdict(3_473_803, 3_473_803, Some(UNKNOWN_FUTURE_BRANCH), Some(0))
                .unknown_branch_grace(),
            None,
            "Unsupported is a refusal, not a grace"
        );
    }

    /// GRACE-1 (§4p G-5's conversion): the time the surface shows is whichever
    /// rule expires FIRST, the block rule converted through the ONE spacing
    /// constant — 100 blocks left with a fresh clock reads 7,500 s (blocks
    /// first); 1,000 blocks left with 6,400 s on the clock reads 6,400 s (the
    /// clock first). Mutant: the `min` dropped (the first reads 86,400); a
    /// second literal spacing (the first reads something other than 7,500).
    #[test]
    fn the_time_shown_is_whichever_rule_expires_first_through_the_one_constant() {
        let blocks_first = verdict_at(
            3_300_000,
            3_300_000,
            None,
            Some(UNKNOWN_BRANCH_GRACE_BLOCKS - 100),
            elapsed(0),
        );
        assert_eq!(
            blocks_first.unknown_branch_grace(),
            Some(UnknownBranchGrace::Running {
                blocks_left: 100,
                secs_left: Some(u32::try_from(100 * TARGET_BLOCK_SPACING_SECS).expect("fits")),
            })
        );
        let clock_first = verdict_at(
            3_300_000,
            3_300_000,
            None,
            Some(UNKNOWN_BRANCH_GRACE_BLOCKS - 1_000),
            elapsed(UNKNOWN_BRANCH_GRACE_SECS - 6_400),
        );
        assert_eq!(
            clock_first.unknown_branch_grace(),
            Some(UnknownBranchGrace::Running {
                blocks_left: 1_000,
                secs_left: Some(6_400),
            })
        );
    }

    /// GRACE-1 (§4p G-1's typed clause, item 2): the three refusals are three
    /// TYPES — `Unsupported` keeps `NetworkUpgradeUnsupported`; an expired grace
    /// is `ConsensusGraceExpired { by, .. }`; never evaluated is
    /// `ConsensusNotEvaluated` — and the codes differ. Mutant: the grace arm
    /// folded back into `NetworkUpgradeUnsupported` (the base).
    #[test]
    fn each_refusal_is_its_own_type() {
        use crate::error::WalletError;
        let expired = verdict_at(
            3_300_000,
            3_300_000,
            None,
            Some(0),
            elapsed(UNKNOWN_BRANCH_GRACE_SECS),
        );
        let refusal = SigningPermit::granted(Some(&expired)).expect_err("refused");
        assert!(
            matches!(
                refusal,
                WalletError::ConsensusGraceExpired {
                    by: GraceExpiry::Clock,
                    blocks_since_last_current: Some(0)
                }
            ),
            "got {refusal:?}"
        );
        let unsupported = verdict(3_473_803, 3_473_803, Some(UNKNOWN_FUTURE_BRANCH), Some(0));
        assert!(matches!(
            SigningPermit::granted(Some(&unsupported)).expect_err("refused"),
            WalletError::NetworkUpgradeUnsupported { .. }
        ));
        let never = SigningPermit::granted(None).expect_err("refused");
        assert!(matches!(never, WalletError::ConsensusNotEvaluated));
        let codes = [
            refusal.code(),
            SigningPermit::granted(Some(&unsupported))
                .expect_err("refused")
                .code(),
            never.code(),
        ];
        assert_eq!(codes.len(), 3);
        assert!(
            codes[0] != codes[1] && codes[1] != codes[2] && codes[0] != codes[2],
            "three refusals, three codes: {codes:?}"
        );
        assert!(SigningPermit::granted(Some(&ConsensusCompatibility::Current)).is_ok());
    }

    /// §4p-run row 2 (the GRACE-1 ruling on `SigningPermit::granted`): the
    /// answer and the reason are ONE derivation. For each grace an `Unknown`
    /// can read — `Ended { Blocks }`, `Ended { Clock }`,
    /// `Ended { NeverConfirmed }`, `Running` — the gate refuses with exactly
    /// the rule `unknown_branch_grace()` names (and grants for `Running`), and
    /// `permits_signing()` is `granted(..).is_ok()`; the three verdicts the
    /// grace does not govern obey the same equation with no grace. Non-vacuity:
    /// the four fixtures are asserted to read four DIFFERENT graces. Mutants:
    /// `permits_signing`'s `Unknown` arm → `true` (an ended grace is granted —
    /// "got Ok"); → `false` (M-r: the running grace is refused, and it is the
    /// "got refused" assertion that reds, never a panic).
    #[test]
    fn the_gate_refuses_with_the_rule_the_grace_names_and_grants_exactly_while_it_runs() {
        use crate::error::WalletError;
        let rows = [
            verdict_at(
                3_300_000,
                3_300_000,
                None,
                Some(UNKNOWN_BRANCH_GRACE_BLOCKS),
                elapsed(0),
            ),
            verdict_at(
                3_300_000,
                3_300_000,
                None,
                Some(7),
                elapsed(UNKNOWN_BRANCH_GRACE_SECS),
            ),
            verdict_at(3_300_000, 3_300_000, None, None, elapsed(0)),
            verdict_at(3_300_000, 3_300_000, None, Some(7), elapsed(60)),
        ];
        let mut graces = Vec::with_capacity(rows.len());
        for v in &rows {
            let grace = v
                .unknown_branch_grace()
                .expect("an Unknown always reads a grace");
            let permits = v.permits_signing();
            let gate = SigningPermit::granted(Some(v));
            assert_eq!(
                permits,
                gate.is_ok(),
                "the answer IS the gate: {grace:?} → permits {permits} / gate {gate:?}"
            );
            match grace {
                UnknownBranchGrace::Running { .. } => assert!(
                    gate.is_ok(),
                    "a running grace is granted, got refused: {gate:?}"
                ),
                UnknownBranchGrace::Ended {
                    by,
                    blocks_since_last_current,
                } => assert!(
                    matches!(
                        &gate,
                        Err(WalletError::ConsensusGraceExpired {
                            by: got,
                            blocks_since_last_current: got_blocks,
                        }) if *got == by && *got_blocks == blocks_since_last_current
                    ),
                    "an ended grace refuses with the rule that ended it \
                     ({by:?}, {blocks_since_last_current:?}), got {gate:?}"
                ),
            }
            graces.push(grace);
        }
        // Anti-vacuity: four rows, four different readings — three expiries
        // and a run.
        assert!(
            matches!(
                graces[0],
                UnknownBranchGrace::Ended {
                    by: GraceExpiry::Blocks,
                    ..
                }
            ),
            "{:?}",
            graces[0]
        );
        assert!(
            matches!(
                graces[1],
                UnknownBranchGrace::Ended {
                    by: GraceExpiry::Clock,
                    ..
                }
            ),
            "{:?}",
            graces[1]
        );
        assert!(
            matches!(
                graces[2],
                UnknownBranchGrace::Ended {
                    by: GraceExpiry::NeverConfirmed,
                    ..
                }
            ),
            "{:?}",
            graces[2]
        );
        assert!(
            matches!(graces[3], UnknownBranchGrace::Running { .. }),
            "{:?}",
            graces[3]
        );
        // The verdicts the grace does not govern: the same equation, no grace.
        let behind = verdict(1_200_000, 3_400_000, Some(previous_branch()), None);
        assert!(matches!(behind, ConsensusCompatibility::Behind { .. }));
        let unsupported = verdict(3_473_803, 3_473_803, Some(UNKNOWN_FUTURE_BRANCH), Some(0));
        assert!(matches!(
            unsupported,
            ConsensusCompatibility::Unsupported { .. }
        ));
        for v in [ConsensusCompatibility::Current, behind, unsupported] {
            assert_eq!(v.unknown_branch_grace(), None, "{v:?}");
            assert_eq!(
                v.permits_signing(),
                SigningPermit::granted(Some(&v)).is_ok(),
                "{v:?}"
            );
        }
    }

    /// THE EXACT INCIDENT (spec §8 gate 3). A wallet provisioned before an
    /// upgrade must detect it after, WITHOUT re-provisioning — the property the
    /// shipped provisioning-only guard structurally cannot have.
    ///
    /// REWRITTEN in Phase B step 7, not retuned. It used to phrase the incident
    /// as "the endpoint reports Ironwood and we refuse", which was true only
    /// while we did not know Ironwood — the very condition this wave ends. Phrased
    /// against Ironwood it would now assert the OPPOSITE of what it was built to
    /// protect. The property was never about Ironwood: it is that a branch this
    /// build cannot model flips the verdict on an already-provisioned wallet. Both
    /// halves are asserted below, and the second half is what the wave bought.
    #[test]
    fn wallet_provisioned_before_an_upgrade_detects_it_after() {
        // Pre-activation: a height one epoch back, endpoint agreeing. Signing fine.
        let before = verdict(
            ironwood_mainnet() - 1,
            ironwood_mainnet() - 1,
            Some(previous_branch()),
            None,
        );
        assert_eq!(before, ConsensusCompatibility::Current);
        assert!(before.permits_signing());

        // The tip crosses an activation and the endpoint reports a branch this
        // build has never heard of. Same wallet, same endpoint, no re-provisioning
        // — the verdict must flip. THIS is the incident, and it is now expressed
        // against an unmodellable branch rather than a named one, so the next
        // upgrade cannot invert it the way Ironwood inverted the first version.
        let after = verdict(3_473_803, 3_473_803, Some(UNKNOWN_FUTURE_BRANCH), None);
        assert!(
            matches!(after, ConsensusCompatibility::Unsupported { .. }),
            "a tip on an unmodellable branch must be Unsupported, got {after:?}"
        );
        assert!(
            !after.permits_signing(),
            "the six-week outage is exactly this returning true"
        );

        // AND THE OTHER DIRECTION, which is what Phase B restores and no earlier
        // version of this test could assert: a post-Ironwood tip on the Ironwood
        // branch is now MODELLABLE, so it signs. If this ever reverts to
        // `Unsupported`, sending is broken again for the same reason it was for
        // six weeks — which is the failure this row exists to catch from now on.
        let ironwood = verdict(3_473_803, 3_473_803, Some(nu6_3_branch()), None);
        assert_eq!(
            ironwood,
            ConsensusCompatibility::Current,
            "post-wave, an honest Ironwood endpoint must be Current — the outage inverted"
        );
        assert!(ironwood.permits_signing(), "sending must work again");
    }

    /// Pins spec §1.2: the two fields the shipped guard compares are immutable
    /// historical constants, so an endpoint that gets BOTH right and still
    /// reports a new branch must be caught. Guards against a future
    /// "simplification" back to the old predicate.
    #[test]
    fn chain_name_and_sapling_activation_alone_cannot_detect_an_upgrade() {
        // Phase B step 7: the "new branch" is now an UNMODELLABLE one rather than
        // Ironwood, for the reason spelled out on
        // `wallet_provisioned_before_an_upgrade_detects_it_after`. The property is
        // unchanged — the two fields the shipped guard compared are immutable, so
        // they cannot detect any upgrade — and it is now stated in a way the next
        // upgrade cannot invert.
        let id = endpoint(Some(UNKNOWN_FUTURE_BRANCH));
        // The old guard's two inputs are correct...
        assert_eq!(id.chain_name, "main");
        assert_eq!(id.sapling_activation_height, 419_200);
        // ...and the wallet is still refused, because they cannot move.
        let v = consensus_compatibility(
            &MAIN_NETWORK,
            BlockHeight::new(ironwood_mainnet()),
            BlockHeight::new(ironwood_mainnet()),
            &id,
            None,
            GraceClock::NONE,
        );
        assert!(!v.permits_signing());
    }

    /// §1.3 / gate 1: a hostile claim can only ever STOP signing. There is no
    /// input to this function that turns an unknown branch into permission.
    #[test]
    fn hostile_endpoint_branch_id_is_refusal_only() {
        // Phase B step 7: Ironwood's branch LEFT this list, because at a
        // post-activation tip it is now the HONEST answer, not a hostile one —
        // keeping it here would have asserted that a correct endpoint is refused,
        // i.e. the outage. What stays is every claim that is genuinely not the
        // branch for the judged height. `previous_branch()` is added: an endpoint
        // reporting the PRE-Ironwood branch at a post-Ironwood tip is exactly the
        // downgrade shape, and it must not sign either.
        for claim in [
            0xdead_beef_u32,
            0,
            1,
            u32::MAX,
            UNKNOWN_FUTURE_BRANCH,
            previous_branch(),
        ] {
            let v = verdict(3_473_803, 3_473_803, Some(claim), Some(0));
            assert!(
                !v.permits_signing(),
                "claim {claim:#x} must not permit signing at a post-Ironwood tip"
            );
        }
        // Anti-vacuity: this loop must not be rejecting EVERYTHING. The one claim
        // that is honest at that height signs.
        let honest = verdict(3_473_803, 3_473_803, Some(nu6_3_branch()), Some(0));
        assert!(
            honest.permits_signing(),
            "the honest branch at the judged height must still sign — otherwise \
             this test proves nothing about hostility"
        );
    }

    /// §1.3's other half, and the finding that refuted the draft: the branch we
    /// sign under is `for_height(params, h)`, so an endpoint that UNDER-REPORTS
    /// its tip moves `h` — and with it the branch — unless the judgement is
    /// pinned to the highest height either side attests.
    ///
    /// The attack, made concrete with today's pin: the endpoint claims a tip
    /// deep in the Canopy epoch and the branch that is HONEST at that height.
    /// Judged at the claim alone the pair is self-consistent and answers
    /// `Current`; judged at the max it disagrees with blocks we have already
    /// scanned, and is refused.
    #[test]
    fn under_reported_tip_cannot_buy_a_current_verdict() {
        let claimed_tip = 1_000_000; // Canopy epoch
        let honest_at_claim = u32::from(BranchId::for_height(
            &MAIN_NETWORK,
            ConsensusHeight::from_u32(claimed_tip),
        ));
        // Sanity: the claim really is internally consistent, so this test is
        // about the HEIGHT rule and not about an obviously bogus branch.
        assert_ne!(
            honest_at_claim,
            previous_branch(),
            "the fixture must straddle a real upgrade boundary"
        );

        let scanned_tip = 3_400_000; // we have scanned into the Nu6_2 epoch
        let v = verdict(scanned_tip, claimed_tip, Some(honest_at_claim), None);
        assert!(
            matches!(v, ConsensusCompatibility::Unsupported { .. }),
            "a self-consistent under-reporting endpoint must still be refused, got {v:?}"
        );
        assert!(!v.permits_signing());

        // The anti-vacuity half: the SAME branch claim at the SAME height it is
        // honest for is accepted, so the test is detecting the height rule and
        // not merely rejecting everything.
        let honest = verdict(claimed_tip, claimed_tip, Some(honest_at_claim), None);
        assert_eq!(honest, ConsensusCompatibility::Current);
    }

    /// The limit of what this predicate can do, stated as a test so nobody
    /// mistakes it for a defence it is not: a STALE build whose endpoint lies
    /// in the direction of our own staleness cannot detect the lie from these
    /// inputs alone. That is why §4 promotes the local unforgeable evidence —
    /// a mined transaction naming a branch we do not implement
    /// (`enhance::unknown_branch_evidence`) — to a staleness input.
    #[test]
    fn a_lying_endpoint_can_hide_our_own_staleness_from_this_predicate() {
        // Phase B step 7. The limit is real and unchanged; only the fixture moved,
        // and it had to. The old version used the pre-Ironwood branch at a
        // post-Ironwood tip — which was indistinguishable-from-honest ONLY while
        // this build did not know Ironwood. It does now, so that exact pair is
        // caught (see `hostile_endpoint_branch_id_is_refusal_only`), and asserting
        // `Current` for it would have been asserting the outage.
        //
        // The limit needs a chain that has moved PAST what this build models — so
        // it is expressed at a height beyond every activation we know, where the
        // endpoint reports the newest branch we DO know. A future upgrade could be
        // live there and this predicate cannot tell.
        let far_future = ironwood_mainnet() + 5_000_000;
        let v = verdict(far_future, far_future, Some(nu6_3_branch()), None);
        assert_eq!(
            v,
            ConsensusCompatibility::Current,
            "documented limit: the branch check alone cannot catch a server that \
             lies in the direction of our staleness"
        );
        // And the reason it is a LIMIT rather than a hole: the local unforgeable
        // evidence in §4 (`enhance::unknown_branch_evidence`) is the input that
        // closes it — a mined transaction naming a branch we do not implement.
        // That path is OWED for v6 (INC-015) and this row is why it matters.
    }

    /// A CURRENT binary mid-restore must not be refused (the spend-before-sync
    /// property). Scanned tip deep in history, endpoint honest at the tip.
    #[test]
    fn a_restoring_wallet_is_behind_not_unsupported() {
        // Our params' newest upgrade is Nu6_2, so an honest endpoint at a
        // post-Nu6_2 / pre-Ironwood tip reports Nu6_2 and we agree.
        let v = verdict(1_200_000, 3_400_000, Some(previous_branch()), None);
        assert!(
            matches!(v, ConsensusCompatibility::Behind { .. }),
            "a lagging scan is Behind, never Unsupported, got {v:?}"
        );
        assert!(
            v.permits_signing(),
            "refusing here would kill spend-before-sync for every restore"
        );
    }

    /// §6.3: absent/blank is `Unknown`, never `Current`.
    #[test]
    fn absent_branch_id_is_unknown_not_current() {
        let v = verdict(3_300_000, 3_300_000, None, Some(0));
        assert!(matches!(v, ConsensusCompatibility::Unknown { .. }));
        assert_ne!(v, ConsensusCompatibility::Current);
    }

    /// §6.3's boundary, BOTH sides. The maintainer decision is "degrade with age",
    /// and a mutant that ignores the grace in either direction fails here.
    #[test]
    fn unknown_branch_signs_inside_the_grace_and_refuses_outside_it() {
        let inside = verdict(
            3_300_000,
            3_300_000,
            None,
            Some(UNKNOWN_BRANCH_GRACE_BLOCKS - 1),
        );
        assert!(
            inside.permits_signing(),
            "an honest quiet server must not break a user mid-session"
        );

        let at_threshold = verdict(
            3_300_000,
            3_300_000,
            None,
            Some(UNKNOWN_BRANCH_GRACE_BLOCKS),
        );
        assert!(
            !at_threshold.permits_signing(),
            "the grace is exclusive at the threshold"
        );

        let outside = verdict(
            3_300_000,
            3_300_000,
            None,
            Some(UNKNOWN_BRANCH_GRACE_BLOCKS * 10),
        );
        assert!(!outside.permits_signing());
    }

    /// The `None` arm of the grace: a fresh install that has only ever met a
    /// silent endpoint has never confirmed it can transact.
    #[test]
    fn unknown_branch_with_no_prior_current_verdict_refuses() {
        let v = verdict(3_300_000, 3_300_000, None, None);
        assert!(matches!(v, ConsensusCompatibility::Unknown { .. }));
        assert!(!v.permits_signing());
    }

    /// An `Unknown` riding its grace must NEVER renew the anchor it rides, or a
    /// silent endpoint extends its own grace forever.
    #[test]
    fn an_unknown_inside_grace_does_not_renew_the_anchor() {
        let inside = verdict(3_300_000, 3_300_000, None, Some(0));
        assert!(inside.permits_signing());
        assert!(
            !inside.is_signing_capable_check(),
            "riding the grace must not refresh it"
        );
        assert!(ConsensusCompatibility::Current.is_signing_capable_check());
    }

    /// §4.6 boundary: arbitrary bytes in the hex field → `None`, never a panic.
    #[test]
    fn unparseable_branch_id_does_not_panic() {
        for raw in [
            "",
            "zz",
            "0x",
            "  ",
            "-1",
            "37a5165bb",
            "0x37a5165bb",
            "ffffffffff",
            "٣٧",
            "37a5165g",
        ] {
            assert_eq!(parse_branch_id(raw), None, "{raw:?} must not parse");
        }
        assert_eq!(parse_branch_id("37a5165b"), Some(nu6_3_branch()));
        assert_eq!(parse_branch_id("0x37a5165b"), Some(nu6_3_branch()));
        assert_eq!(parse_branch_id("5437f330"), Some(previous_branch()));
    }

    /// The anti-staleness guard on `KNOWN_UPGRADES`: if the pinned crate learns
    /// an upgrade our list does not name, the branch it computes far past every
    /// activation stops matching the branch of our newest known upgrade — and
    /// the anchor ceiling would silently sit one upgrade too low. Fails LOUDLY
    /// on the next crate bump, which is the point.
    #[test]
    fn known_upgrades_is_really_complete() {
        let newest = newest_known_activation(&MAIN_NETWORK).expect("mainnet has activations");
        assert_eq!(
            BranchId::for_height(&MAIN_NETWORK, ConsensusHeight::from_u32(u32::MAX)),
            BranchId::for_height(&MAIN_NETWORK, newest),
            "zcash_protocol knows an upgrade KNOWN_UPGRADES does not list — add it, \
             and re-check the bundled-anchor ceiling it moves"
        );
    }

    /// The ceiling is the thing the bundled anchor rests on. **REWRITTEN in Phase
    /// B step 6, not retuned** — the plan says so explicitly, and the reason is
    /// that the old assertions (`ceiling == 3_364_600` AND
    /// `ceiling < IRONWOOD_MAINNET`) are BOTH inverted by adding `Nu6_3`, so
    /// swapping their literals would read as a Phase-B regression while asserting
    /// nothing. The literal was never the property.
    ///
    /// The property that survives every upgrade: the ceiling is exactly the newest
    /// activation this build can model, so a bundled row is selectable if and only
    /// if this build knows the rules in force there.
    #[test]
    fn the_anchor_ceiling_is_the_newest_upgrade_this_build_can_model() {
        let ceiling = u32::from(newest_known_activation(&MAIN_NETWORK).expect("a ceiling"));

        // 1. Structural, not a literal: the branch far past every activation is the
        //    branch at the ceiling. If the crate knew an upgrade above it, these
        //    would differ — the same invariant `known_upgrades_is_really_complete`
        //    states from the other side.
        assert_eq!(
            BranchId::for_height(&MAIN_NETWORK, ConsensusHeight::from_u32(u32::MAX)),
            BranchId::for_height(&MAIN_NETWORK, ConsensusHeight::from_u32(ceiling)),
            "the ceiling is not the newest modellable height"
        );

        // 2. It MOVED, and moving it is the point of step 6. Before this commit it
        //    was Nu6_2's 3,364,600 and the 13 bundled mainnet rows above the
        //    Ironwood activation were unreachable as anchors. Asserting it is now
        //    at/above the activation is the direction check: the ceiling may only
        //    ever rise as this build learns MORE, never fall.
        assert!(
            ceiling >= ironwood_mainnet(),
            "the ceiling ({ceiling}) fell below the Ironwood activation ({}) — a \
             build that models NU6.3 must be able to anchor there",
            ironwood_mainnet()
        );

        // 3. THE SAFETY PROPERTY, and it is not the one I first assumed. Raising
        //    the ceiling to the Ironwood activation does NOT make the 13 bundled
        //    rows ABOVE that activation anchorable — `bundled_treestate` selects
        //    the newest row strictly below `ceiling + 1`, and the first of those
        //    rows is 3,429,810. The ceiling stays deliberately conservative: above
        //    the newest activation we know, an upgrade we do NOT know may be live,
        //    and that is true of NU7 today exactly as it was of NU6.3 yesterday.
        //
        //    What it does move — measured, not assumed — is the newest anchorable
        //    row from 3,362,500 to 3,427,310, i.e. 26 further rows and about 64,810
        //    fewer blocks of first scan. EVERY one of them is below the activation,
        //    so an empty Ironwood frontier is CORRECT there, which is what makes
        //    the raise safe. Assert exactly that, since it is the property a future
        //    ceiling change could break.
        let table = crate::checkpoints::treestate_table(crate::money::Network::Main);
        let selectable: Vec<u32> = table
            .iter()
            .map(|&(h, ..)| h)
            .filter(|&h| h < ceiling.saturating_add(1))
            .collect();
        assert!(
            !selectable.is_empty(),
            "no bundled row is selectable at all — the ceiling has excluded the whole bundle"
        );
        let newest_selectable = *selectable.last().expect("non-empty");
        for &(h, _hash, _sap, _orch, ironwood) in table {
            if h < ceiling.saturating_add(1) && h >= ironwood_mainnet() {
                assert_ne!(
                    ironwood, "",
                    "row {h} is SELECTABLE as a birthday anchor and sits at/above the \
                     Ironwood activation with no Ironwood frontier — anchoring there \
                     bakes an empty third pool into every wallet born at that height"
                );
            }
        }
        // Non-vacuity for the loop above is honest rather than asserted: NO row
        // satisfies its antecedent today, because the ceiling sits below the first
        // row above the activation. It is an implication kept live for the next
        // ceiling raise, and the row-carries-a-frontier property IS asserted
        // non-vacuously by
        // `checkpoints::treestate_bundle_carries_an_ironwood_frontier_after_activation`.
        // What this test asserts non-vacuously is the boundary itself:
        assert!(
            newest_selectable < ironwood_mainnet(),
            "the newest selectable anchor ({newest_selectable}) is at/above the Ironwood \
             activation ({}) — if that is intended, the loop above becomes the live \
             guard and this assertion is what must be rewritten, deliberately",
            ironwood_mainnet()
        );
    }

    /// The inverse of the row this replaces. It used to assert the pinned params
    /// did NOT know Ironwood — the condition that WAS the outage, kept as
    /// non-vacuity for the fixtures above and as the tripwire for this wave. The
    /// wave fired it. What it becomes (spec §8 gate 7, `checkpoint_activation_
    /// heights_match_zcash_protocol`) is the check that every Ironwood height this
    /// codebase states agrees with the crate — one source of truth for a consensus
    /// constant, which is what the outage cost us.
    #[test]
    fn checkpoint_activation_heights_match_zcash_protocol() {
        // The crate knows it. If this ever fails again the wave has been reverted.
        assert!(
            BranchId::try_from(nu6_3_branch()).is_ok(),
            "the pinned zcash_protocol must know the Ironwood branch"
        );
        assert_eq!(
            u32::from(BranchId::for_height(
                &MAIN_NETWORK,
                ConsensusHeight::from_u32(ironwood_mainnet())
            )),
            nu6_3_branch(),
            "at the activation height the branch IS Ironwood — the outage was this \
             returning the previous branch at a live height"
        );

        // The numbers the ADR, the spec, the checkpoint emitter and the bundle gap
        // test all state, checked against the crate ONCE, here. A literal in any of
        // those places that drifts from the crate is the outage's own shape.
        assert_eq!(ironwood_mainnet(), 3_428_143, "mainnet NU6.3 activation");
        assert_eq!(
            u32::from(
                zcash_protocol::consensus::TEST_NETWORK
                    .activation_height(NetworkUpgrade::Nu6_3)
                    .expect("testnet knows Nu6_3")
            ),
            4_134_000,
            "testnet NU6.3 activation"
        );
        // And the height one below is the PREVIOUS branch, so the boundary is
        // exactly where every other file says it is — not one block either side.
        assert_ne!(
            previous_branch(),
            nu6_3_branch(),
            "the activation boundary must be a real branch change"
        );
    }

    /// Non-vacuity for `UNKNOWN_FUTURE_BRANCH`. The outage fixtures above are
    /// meaningless if the branch they call unmodellable is one the crate knows.
    /// This is the row that fires on the NEXT wave the way
    /// `the_pinned_params_really_do_not_know_ironwood` fired on this one.
    #[test]
    fn unknown_branch_fixture_is_really_unknown() {
        assert!(
            BranchId::try_from(UNKNOWN_FUTURE_BRANCH).is_err(),
            "the pinned zcash_protocol now knows {UNKNOWN_FUTURE_BRANCH:#x} — pick a \
             value it does not, or the outage fixtures assert nothing"
        );
    }
}
