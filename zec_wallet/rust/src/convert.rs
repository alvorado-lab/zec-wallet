//! Core ⇄ bridge DTO conversions. Lives OUTSIDE `api/` so FRB codegen never
//! sees a core type (spec §3.3: nothing from `zec_wallet_core` may appear in a
//! generated signature).
//!
//! DRIFT GUARDS (the bridge and core are version-locked, but locks need
//! teeth):
//! - **Structs convert via EXHAUSTIVE destructuring** (no `..`) — a field
//!   added to a core DTO fails compile HERE, so the Dart shape is updated in
//!   the same change, never silently dropped.
//! - **Enum matches end in a wildcard → `Unknown`** (core enums are
//!   `#[non_exhaustive]`, so a wildcard is mandatory outside their crate).
//!   The `bridge_enums_cover_core_variants` policy test keeps the named arms
//!   complete, so the wildcard is unreachable in a lockstep build; if it
//!   ever fires anyway, Dart renders an honest "unknown" instead of the
//!   bridge panicking across FFI.
//! - Error conversions capture `code()`/`Display` from the CORE before
//!   matching — there is no second code table in this crate, and a
//!   future-unknown variant still carries its real `RW-*` code.

#![deny(unsafe_code)]

use zec_wallet_core as rw;

use crate::api::config as api_config;
use crate::api::error as api_error;
use crate::api::payments as api_payments;
use crate::api::state as api_state;
use crate::api::swap as api_swap;
use crate::api::wallet as api_wallet;
use crate::frb_generated::StreamSink;

// ─── Outbound: core state → Dart DTOs (consumed by the wallet handle /
//     streams — increment 2 of W3, once WalletDb lands) ─────────────────────
//
// The `u64 as i64` narrowing comments below assume an HONEST core: a corrupt
// store feeding u64::MAX (possible after a partial write + kill, increment
// 2+) degrades to a visible-but-sane negative value Dart-side (a 1969
// timestamp, a -1 batch id) — never UB, never a panic. The store layer owns
// rejecting corrupt values BEFORE they get here (StoreCorrupt).

impl From<rw::BalanceSnapshot> for api_state::BalanceSnapshot {
    fn from(b: rw::BalanceSnapshot) -> Self {
        let rw::BalanceSnapshot {
            spendable,
            pending_incoming,
            pending_change,
            transparent,
            total,
        } = b;
        Self {
            spendable_zat: spendable.zat(),
            pending_incoming_zat: pending_incoming.zat(),
            pending_change_zat: pending_change.zat(),
            transparent_zat: transparent.zat(),
            total_zat: total.zat(),
        }
    }
}

impl From<rw::StrandedAmount> for api_state::RecoverableEphemeralFunds {
    fn from(s: rw::StrandedAmount) -> Self {
        // EXHAUSTIVE destructuring (the file-top drift-guard contract): a field ADDED to the core
        // `StrandedAmount` fails compile HERE, so the Dart shape is updated in the same change rather
        // than silently dropped.
        let rw::StrandedAmount {
            recoverable_zat,
            is_final,
        } = s;
        Self {
            // The core amount is `Balance::total()` — consensus-bounded ≤ max supply (≈ 2.1e15) <
            // i64::MAX — so `try_from` never saturates for an honest engine. Unlike the file-top
            // "visible-but-sane negative" note (right for a timestamp/batch-id), a NEGATIVE recoverable
            // AMOUNT is a money-display footgun (it is partly endpoint-influenced — a lying endpoint
            // funds the ephemeral). Saturating at i64::MAX (the over-show direction, never hide) is the
            // money-safe fallback for the unreachable overflow.
            recoverable_zat: i64::try_from(recoverable_zat).unwrap_or(i64::MAX),
            is_final,
        }
    }
}

impl From<rw::EphemeralSweepSummary> for api_state::EphemeralSweepSummary {
    fn from(s: rw::EphemeralSweepSummary) -> Self {
        // EXHAUSTIVE destructuring (the file-top drift-guard contract): a field ADDED to the core
        // `EphemeralSweepSummary` fails compile HERE, so the Dart shape is updated in the same change.
        let rw::EphemeralSweepSummary {
            scanned,
            swept,
            recovered_zat,
            failed,
            truncated,
        } = s;
        // `swept` is bounded by the per-invocation cap (`EPHEMERAL_SWEEP_MAX_ADDRS`); `scanned` /
        // `truncated` are the PRE-cap full reserved-set size (so they can exceed the cap on a heavy
        // wallet) but are still a `usize` count of the wallet's own ephemerals, decades below u32::MAX.
        // So `try_from` never saturates for any real wallet; saturating at u32::MAX is the unreachable
        // fallback (over-show, never a panic). `recovered_zat` is already the i64 money domain (an
        // aggregate of consensus-bounded nets), crossed directly.
        Self {
            scanned: u32::try_from(scanned).unwrap_or(u32::MAX),
            swept: u32::try_from(swept).unwrap_or(u32::MAX),
            recovered_zat,
            failed: u32::try_from(failed).unwrap_or(u32::MAX),
            truncated: u32::try_from(truncated).unwrap_or(u32::MAX),
        }
    }
}

impl From<rw::ReclaimOutcome> for api_state::ReclaimOutcome {
    fn from(o: rw::ReclaimOutcome) -> Self {
        // EXHAUSTIVE match (the drift-guard contract): a variant ADDED to the core `ReclaimOutcome`
        // fails compile HERE, so the Dart union is updated in the same change. `amount_zat` is
        // already the i64 money domain (a fixed consensus-bounded constant), crossed directly.
        match o {
            rw::ReclaimOutcome::NothingToReclaim => Self::NothingToReclaim,
            rw::ReclaimOutcome::Minted { amount_zat } => Self::Minted { amount_zat },
            rw::ReclaimOutcome::NotBroadcast { amount_zat } => Self::NotBroadcast { amount_zat },
            // The core enum is `#[non_exhaustive]`; a NEW variant trips the lockstep policy test
            // (`bridge_enums_cover_core_variants`) so this arm is unreachable in a lockstep build.
            _ => Self::Unknown,
        }
    }
}

impl From<rw::SwapAddressCheckReport> for api_state::SwapAddressCheckReport {
    fn from(r: rw::SwapAddressCheckReport) -> Self {
        // EXHAUSTIVE destructuring (the file-top drift-guard contract). All three fields
        // are already `u32` counts in the core (never an index value — §5.4), crossed
        // directly; no saturation needed.
        let rw::SwapAddressCheckReport {
            widened_by,
            covered_swaps,
            pending,
        } = r;
        Self {
            widened_by,
            covered_swaps,
            pending,
        }
    }
}

impl From<rw::SwapAddressCheckCoverage> for api_state::SwapAddressCoverage {
    fn from(c: rw::SwapAddressCheckCoverage) -> Self {
        // EXHAUSTIVE destructuring; both `u32` counts crossed directly (§5.4).
        let rw::SwapAddressCheckCoverage {
            covered_swaps,
            pending,
        } = c;
        Self {
            covered_swaps,
            pending,
        }
    }
}

impl From<rw::ParkedAuthorization> for api_state::ParkedAuthorization {
    fn from(a: rw::ParkedAuthorization) -> Self {
        // 1:1 map; the core enum is #[non_exhaustive] (G2), so an outcome this bridge doesn't know
        // folds to the honest `Unknown` — which the host doc pins to "treat exactly like
        // stillQueued": say NOTHING about money having moved, re-read the list. Deliberately NOT
        // folded to `Signed` (that would claim a send the wallet never made) and NOT to `NotFound`
        // (that would invite a re-send: a DOUBLE-PAY). The extraction-policy lockstep MAP makes a
        // NEW core arm trip CI so this fallback never silently absorbs one.
        match a {
            rw::ParkedAuthorization::Signed => Self::Signed,
            rw::ParkedAuthorization::StillQueued => Self::StillQueued,
            rw::ParkedAuthorization::Expired => Self::Expired,
            rw::ParkedAuthorization::NotFound => Self::NotFound,
            _ => Self::Unknown,
        }
    }
}

impl From<rw::ParkedSend> for api_state::ParkedSend {
    fn from(p: rw::ParkedSend) -> Self {
        // EXHAUSTIVE destructuring (the file-top drift-guard contract): a field ADDED to the core
        // `ParkedSend` fails compile HERE, so the Dart shape is updated in the same change.
        let rw::ParkedSend {
            id,
            kind,
            amount_zat,
            created_at,
            paused,
            sending,
            binding,
            signing_block,
        } = p;
        // The i64s are already the right domain (id = rowid, amount_zat = a consensus-bounded gross
        // total ≤ max supply, created_at = unix secs) — crossed directly, no cast. The kind maps
        // 1:1; the core enum is #[non_exhaustive] (G2), so a shape this bridge doesn't know folds
        // to the honest `Unknown` (generic "saved & pending" — still committed, still
        // cancellable). The extraction-policy lockstep MAP makes a NEW core kind trip CI so this
        // arm never silently absorbs one.
        Self {
            id,
            kind: match kind {
                rw::ParkedSendKind::TwoStep => api_state::ParkedSendKind::TwoStep,
                rw::ParkedSendKind::SingleStep => api_state::ParkedSendKind::SingleStep,
                _ => api_state::ParkedSendKind::Unknown,
            },
            amount_zat,
            created_at,
            paused,
            // #400 R2: the mid-signature bit — crossed as-is; the host renders "preparing" and
            // offers no authorize affordance.
            sending,
            // FR-17: opaque bytes; `None` for a pre-FR-17 row.
            binding: binding.map(|b| b.as_bytes().to_vec()),
            // `ironwood-nu63-support.md` §3.2 / GRACE-1 §4p item 2: the drain
            // skips every queued row while signing is refused, so the row says
            // so — and says WHY, since the two causes have different next steps —
            // rather than posing as a healthy pending send.
            signing_block: signing_block.map(Into::into),
        }
    }
}

impl From<rw::InFlightSend> for api_state::InFlightSend {
    fn from(s: rw::InFlightSend) -> Self {
        // EXHAUSTIVE destructuring (the file-top drift-guard contract): a field ADDED to the core
        // `InFlightSend` fails compile HERE, so the Dart shape is updated in the same change.
        let rw::InFlightSend {
            amount_zat,
            created_at,
        } = s;
        // Both already the i64 domain (a consensus-bounded gross total ≤ max supply; unix secs) —
        // crossed directly, no cast.
        Self {
            amount_zat,
            created_at,
        }
    }
}

impl From<rw::MintedDiversifiedAddress> for api_state::MintedDiversifiedAddress {
    fn from(m: rw::MintedDiversifiedAddress) -> Self {
        // EXHAUSTIVE destructuring (the drift-guard contract): a new core field fails compile HERE.
        let rw::MintedDiversifiedAddress {
            address,
            diversifier_index,
        } = m;
        Self {
            address,
            diversifier_index,
        }
    }
}

impl From<rw::ReservationPressure> for api_state::ReservationPressure {
    fn from(p: rw::ReservationPressure) -> Self {
        // Weld the ceiling bool from the core SSOT BEFORE destructuring, so the host can never render a
        // money-relevant availability state by skipping the comparison (the `is_final` welding pattern).
        let at_ceiling = p.at_ceiling();
        // EXHAUSTIVE destructuring (the drift-guard contract): a new core field fails compile HERE.
        let rw::ReservationPressure { outstanding, limit } = p;
        Self {
            outstanding,
            limit,
            at_ceiling,
        }
    }
}

/// The host depth-finality predicate (§3.2i-2 2e-2b-iii) — delegates to the core SSOT so the reorg
/// horizon lives in ONE place and the host never hard-codes it. Pure passthrough (no core type
/// crosses), exposed `#[frb(sync)]` from `api::state::tx_confirmation_is_final`.
pub(crate) fn tx_confirmation_is_final(confirmation_depth: u32) -> bool {
    rw::confirmation_depth_is_final(confirmation_depth)
}

impl From<rw::SyncStamp> for api_state::SyncStamp {
    fn from(s: rw::SyncStamp) -> Self {
        let rw::SyncStamp { height, at } = s;
        Self {
            height: height.value(),
            // u64 → i64: unix seconds; wraps only past year ~292e9
            at: at as i64,
        }
    }
}

impl From<rw::BoundedSync> for api_state::BoundedSync {
    fn from(b: rw::BoundedSync) -> Self {
        // Destructured so a field added to the core report is a compile error here.
        let rw::BoundedSync {
            scanned_to,
            tip,
            finished,
            resubmitted,
        } = b;
        Self {
            scanned_to: scanned_to.map(|h| h.value()),
            tip: tip.map(|h| h.value()),
            finished,
            resubmitted,
        }
    }
}

impl From<rw::SyncStatus> for api_state::SyncStatus {
    fn from(s: rw::SyncStatus) -> Self {
        match s {
            rw::SyncStatus::Idle => Self::Idle,
            rw::SyncStatus::Connecting {
                tor_bootstrap_percent,
            } => Self::Connecting {
                // the bridge HONORS its own finiteness contract (the api
                // `SyncStatus::Connecting` doc): a non-finite percent (NaN/±inf
                // from a future producer) would break the generated Dart
                // value-equality (NaN != NaN ⇒ every state update a spurious
                // rebuild AND the §3.3 dedup never settles), so it crosses as
                // `None` ("connecting, progress unknown"). Mirrors the
                // `TorState::Bootstrapping` NaN→None fold (B-2-b) — but the
                // SyncStatus path carries the RAW status from the controller
                // (`snapshot()` does NOT route it through `live_tor_state`), so
                // the bridge is the one enforcement point until/unless a future
                // producer sanitizes at source.
                tor_bootstrap_percent: tor_bootstrap_percent.filter(|p| p.is_finite()),
            },
            rw::SyncStatus::Scanning {
                from,
                to,
                percent,
                spendable_ready,
                rewound,
            } => Self::Scanning {
                from: from.value(),
                to: to.value(),
                // crosses RAW (no finiteness filter, unlike `Connecting` above)
                // because the source GUARANTEES finiteness: `account::scan_percent`
                // is div-by-zero-guarded (denom 0 → 1.0) and `clamp(0.0, 1.0)`, so a
                // `Scanning.percent` is always finite by construction. LOAD-BEARING on
                // that clamp: a future producer that skips it would ship a NaN here
                // and break the §3.3 Dart value-equality dedup (the failure the
                // `Connecting` arm filters). Keep `scan_percent`'s clamp, or add a
                // filter here.
                percent,
                spendable_ready,
                rewound,
            },
            rw::SyncStatus::UpToDate { tip } => Self::UpToDate { tip: tip.value() },
            // `ironwood-nu63-support.md` §6.4 — carried as its OWN arm, never
            // folded into `UpToDate`: the whole point of the state is that a
            // host must not render a clean "synced" over a range this build
            // could not fully read.
            rw::SyncStatus::UpToDateLimited { tip } => Self::UpToDateLimited { tip: tip.value() },
            // T0-1b — its OWN arm for the same reason `UpToDateLimited` has one: a
            // host must not render "synced" over a pool this server does not serve.
            rw::SyncStatus::UpToDateDegraded { tip, pools } => Self::UpToDateDegraded {
                tip: tip.value(),
                pools: pools.into(),
            },
            // T0-1c — its OWN arm for the reason its two siblings have one: a
            // host must not render "synced" over a height the server is behind
            // the network at. `newest_known` is a public constant of the signed
            // binary (the E10 shape row in `tests/extraction_policy.rs`);
            // `pools` is the same report `UpToDateDegraded` carries, nullable
            // when the pass made no pool claim.
            rw::SyncStatus::EndpointBehind {
                tip,
                newest_known,
                pools,
            } => Self::EndpointBehind {
                tip: tip.value(),
                newest_known: newest_known.value(),
                pools: pools.map(Into::into),
            },
            // GRACE-1 (§4p) — its OWN arm for the reason its three siblings have
            // one: a host must not render "synced" over a server that will not
            // say which network it is on. What crosses is the grace's REMAINING
            // counts and a reason (G-12) — never the capable timestamp.
            rw::SyncStatus::UpToDateUnverified {
                tip,
                grace,
                pools,
                streak_reported,
            } => Self::UpToDateUnverified {
                tip: tip.value(),
                grace: grace.into(),
                pools: pools.map(Into::into),
                streak_reported,
            },
            rw::SyncStatus::Stalled { reason } => Self::Stalled {
                reason: reason.into(),
            },
            rw::SyncStatus::Offline { last_synced } => Self::Offline {
                last_synced: last_synced.map(Into::into),
            },
            _ => Self::Unknown,
        }
    }
}

impl From<rw::PoolService> for api_state::PoolService {
    fn from(s: rw::PoolService) -> Self {
        match s {
            rw::PoolService::Served { roots } => Self::Served { roots },
            rw::PoolService::Withheld { proven } => Self::Withheld { proven },
            rw::PoolService::Unsupported => Self::Unsupported,
            rw::PoolService::HeightViolation => Self::HeightViolation,
            _ => Self::Unknown,
        }
    }
}

impl From<rw::PoolServiceReport> for api_state::PoolServiceReport {
    fn from(r: rw::PoolServiceReport) -> Self {
        // Exhaustive by-name destructure: a fourth pool in the core is a compile
        // error HERE, never a silently dropped field (the file-wide drift guard).
        let rw::PoolServiceReport {
            sapling,
            orchard,
            ironwood,
        } = r;
        Self {
            sapling: sapling.into(),
            orchard: orchard.into(),
            ironwood: ironwood.into(),
        }
    }
}

impl From<rw::StallReason> for api_state::StallReason {
    fn from(r: rw::StallReason) -> Self {
        match r {
            rw::StallReason::EndpointUnreachable => Self::EndpointUnreachable,
            rw::StallReason::TorUnavailable => Self::TorUnavailable,
            rw::StallReason::StorageFull => Self::StorageFull,
            rw::StallReason::ChainReorg => Self::ChainReorg,
            rw::StallReason::Internal => Self::Internal,
            rw::StallReason::EndpointMisbehaving => Self::EndpointMisbehaving,
            rw::StallReason::BirthdayInFuture => Self::BirthdayInFuture,
            rw::StallReason::StorageUnavailable => Self::StorageUnavailable,
            _ => Self::Unknown,
        }
    }
}

// GRACE-1 (§4p): the reason enum — the SAME value on the refusal, the parked
// row and the sync surface, converted here once. A core variant this bridge
// does not know folds to `Unknown`, which the host renders as the switch-servers
// refusal it is (never a healthy state).
impl From<rw::GraceExpiry> for api_state::GraceExpiry {
    fn from(r: rw::GraceExpiry) -> Self {
        // EXHAUSTIVE-by-name (the lockstep drift guard `bridge_enums_cover_core_variants`):
        // a NEW core reason trips CI until this arm learns it. The core enum is
        // `#[non_exhaustive]`, so the wildcard is unreachable in a lockstep build.
        match r {
            rw::GraceExpiry::Blocks => Self::Blocks,
            rw::GraceExpiry::Clock => Self::Clock,
            rw::GraceExpiry::NeverConfirmed => Self::NeverConfirmed,
            _ => Self::Unknown,
        }
    }
}

// GRACE-1 (§4p G-5/G-12): the grace reading — remaining counts and a reason; a
// core shape this bridge does not know folds to `Unknown`, rendered as the ENDED
// state's generic copy, never as a running grace (fail-closed on the surface).
impl From<rw::UnknownBranchGrace> for api_state::UnknownBranchGrace {
    fn from(g: rw::UnknownBranchGrace) -> Self {
        // EXHAUSTIVE-by-name (the lockstep drift guard `bridge_enums_cover_core_variants`):
        // a NEW core shape trips CI until this arm learns it. The core enum is
        // `#[non_exhaustive]`, so the wildcard is unreachable in a lockstep build.
        match g {
            rw::UnknownBranchGrace::Running {
                blocks_left,
                secs_left,
            } => Self::Running {
                blocks_left,
                secs_left,
            },
            rw::UnknownBranchGrace::Ended {
                by,
                blocks_since_last_current,
            } => Self::Ended {
                by: by.into(),
                blocks_since_last_current,
            },
            _ => Self::Unknown,
        }
    }
}

// GRACE-1 (§4p item 2): the parked row's block — a core kind this bridge does
// not know folds to `Unknown`, which the host renders as a block (never as a
// healthy pending row).
impl From<rw::SigningBlock> for api_state::SigningBlock {
    fn from(b: rw::SigningBlock) -> Self {
        // EXHAUSTIVE-by-name (the lockstep drift guard `bridge_enums_cover_core_variants`):
        // a NEW core kind trips CI until this arm learns it. The core enum is
        // `#[non_exhaustive]`, so the wildcard is unreachable in a lockstep build.
        match b {
            rw::SigningBlock::NetworkUpgrade => Self::NetworkUpgrade,
            rw::SigningBlock::GraceExpired { by } => Self::GraceExpired { by: by.into() },
            _ => Self::Unknown,
        }
    }
}

impl From<rw::TxStatus> for api_state::TxStatus {
    fn from(s: rw::TxStatus) -> Self {
        match s {
            rw::TxStatus::Queued => Self::Queued,
            rw::TxStatus::Pending => Self::Pending,
            rw::TxStatus::Confirmed { depth } => Self::Confirmed { depth },
            rw::TxStatus::Expired => Self::Expired,
            rw::TxStatus::Failed => Self::Failed,
            _ => Self::Unknown,
        }
    }
}

impl From<rw::DeliveryState> for api_state::DeliveryState {
    fn from(s: rw::DeliveryState) -> Self {
        // EXHAUSTIVE-by-name (the lockstep drift guard `bridge_enums_cover_core_variants`):
        // a NEW core reading trips CI until this arm learns it. The core enum is
        // `#[non_exhaustive]`, so the wildcard is unreachable in a lockstep build.
        match s {
            rw::DeliveryState::Persisted => Self::Persisted,
            rw::DeliveryState::RetryPending => Self::RetryPending,
            rw::DeliveryState::Accepted => Self::Accepted,
            rw::DeliveryState::Confirmed => Self::Confirmed,
            _ => Self::Unknown,
        }
    }
}

impl From<rw::TxSummary> for api_state::TxSummary {
    fn from(t: rw::TxSummary) -> Self {
        let rw::TxSummary {
            txid,
            batch_id,
            mined_height,
            status,
            net_amount,
            fee,
            has_memo,
            has_transparent_output,
            timestamp,
            delivery,
            expiry_height,
        } = t;
        Self {
            txid_hex: txid.to_string(),
            // u64 → i64: local proposal counter, can't reach 2^63
            batch_id: batch_id.map(|b| b.value() as i64),
            mined_height: mined_height.map(rw::BlockHeight::value),
            status: status.into(),
            net_amount_zat: net_amount.zat(),
            fee_zat: fee.map(rw::Zatoshis::zat),
            has_memo,
            has_transparent_output,
            timestamp: timestamp.map(|t| t as i64),
            delivery: delivery.map(Into::into),
            expiry_height: expiry_height.map(rw::BlockHeight::value),
        }
    }
}

/// The per-transaction delivery reading behind
/// [`WalletHandle::delivery_state`](crate::api::wallet::WalletHandle::delivery_state):
/// the txid is parsed through the ONE display-hex door
/// (`TxidInvalid` on anything but 64 hex chars), the core's `Option` crosses as
/// `null` for a transaction the wallet owes nothing for.
pub(crate) async fn delivery_state(
    wallet: &rw::Wallet,
    txid_hex: &str,
) -> Result<Option<api_state::DeliveryState>, api_error::WalletApiError> {
    let txid = rw::TxId::from_display_hex(txid_hex)?;
    Ok(wallet.delivery_state(txid).await?.map(Into::into))
}

impl From<rw::HistoryPage> for api_state::HistoryPage {
    fn from(p: rw::HistoryPage) -> Self {
        Self {
            rows: p.rows.into_iter().map(Into::into).collect(),
            next_cursor: p.next,
        }
    }
}

impl From<rw::TxSubmitResult> for api_state::TxSubmitResult {
    fn from(r: rw::TxSubmitResult) -> Self {
        match r {
            rw::TxSubmitResult::Success { txid } => Self::Success {
                txid_hex: txid.to_string(),
            },
            rw::TxSubmitResult::GrpcFailure { txid } => Self::GrpcFailure {
                txid_hex: txid.to_string(),
            },
            rw::TxSubmitResult::SubmitFailure { txid, code } => Self::SubmitFailure {
                txid_hex: txid.to_string(),
                code,
            },
            rw::TxSubmitResult::NotAttempted { txid } => Self::NotAttempted {
                txid_hex: txid.to_string(),
            },
            _ => Self::Unknown,
        }
    }
}

impl From<rw::TorState> for api_state::TorState {
    fn from(t: rw::TorState) -> Self {
        match t {
            rw::TorState::Off => Self::Off,
            rw::TorState::Bootstrapping { percent, transport } => Self::Bootstrapping {
                percent,
                transport: transport.map(|name| name.as_str().to_owned()),
            },
            rw::TorState::Active { runtime } => Self::Active {
                runtime: runtime.into(),
            },
            rw::TorState::FellBack => Self::FellBack,
            rw::TorState::Unavailable { transport } => Self::Unavailable {
                transport: transport.map(|name| name.as_str().to_owned()),
            },
            rw::TorState::Unanswered { runtime } => Self::Unanswered {
                runtime: runtime.into(),
            },
            _ => Self::Unknown,
        }
    }
}

impl From<rw::DialCounts> for api_state::DialCounts {
    fn from(c: rw::DialCounts) -> Self {
        Self {
            private: c.private.into(),
            clearnet: c.clearnet.into(),
        }
    }
}

impl From<rw::ArmCounts> for api_state::ArmCounts {
    fn from(a: rw::ArmCounts) -> Self {
        // u64 → i64 for a plain Dart int: a per-wallet-open dial counter can't
        // reach 2^63 (the `IncomingFundsEvent.total_tx_detected` precedent).
        // SATURATING, not `as`: unreachable by dialling, but the one thing an
        // `as` cast can produce here is a NEGATIVE count on a host's privacy
        // panel, and "impossibly large" is a reading a user can dismiss while
        // "-7 private connections" is one that impeaches the whole panel.
        // The field names must stay the core's, spelled the same
        // (`bridge_structs_mirror_core_fields`).
        fn dart_int(n: u64) -> i64 {
            i64::try_from(n).unwrap_or(i64::MAX)
        }
        Self {
            connected: dart_int(a.connected),
            not_ready: dart_int(a.not_ready),
            retired: dart_int(a.retired),
            unreachable: dart_int(a.unreachable),
            timeout: dart_int(a.timeout),
            unsupported: dart_int(a.unsupported),
            io: dart_int(a.io),
            transport_failed: dart_int(a.transport_failed),
        }
    }
}

impl From<rw::TorRuntimeKind> for api_state::TorRuntimeKind {
    fn from(k: rw::TorRuntimeKind) -> Self {
        match k {
            rw::TorRuntimeKind::ExternalSocks5 => Self::ExternalSocks5,
            rw::TorRuntimeKind::Dialer => Self::Dialer,
            rw::TorRuntimeKind::HostDialer {
                name,
                isolation,
                exposure,
            } => Self::HostDialer {
                name: name.as_str().to_owned(),
                isolation: isolation.into(),
                exposure: exposure.into(),
            },
            _ => Self::Unknown,
        }
    }
}

impl From<rw::TransportExposure> for api_state::TransportExposure {
    fn from(e: rw::TransportExposure) -> Self {
        match e {
            rw::TransportExposure::Unknown => Self::Unknown,
            rw::TransportExposure::Hidden => Self::Hidden,
            rw::TransportExposure::Exposed => Self::Exposed,
            // A future core value this build cannot read: caution, never a promise.
            _ => Self::Unknown,
        }
    }
}

impl From<rw::IsolationSupport> for api_state::IsolationSupport {
    fn from(i: rw::IsolationSupport) -> Self {
        match i {
            rw::IsolationSupport::Unknown => Self::Unknown,
            rw::IsolationSupport::Supported => Self::Supported,
            rw::IsolationSupport::Unsupported => Self::Unsupported,
            // A future core value the host declared and this build cannot
            // read: the cautious rendering (linkable), never a promise.
            _ => Self::Unknown,
        }
    }
}

impl From<rw::WalletState> for api_state::WalletState {
    fn from(w: rw::WalletState) -> Self {
        let rw::WalletState {
            balance,
            sync,
            tor,
            tip,
            last_synced,
            ever_synced,
            rescan_rebuilding,
            seq,
        } = w;
        Self {
            balance: balance.into(),
            sync_status: sync.into(),
            tor: tor.into(),
            tip: tip.map(rw::BlockHeight::value),
            last_synced: last_synced.map(Into::into),
            // #357 + #377 s357b-2: the durable catch-up flags, carried through
            // verbatim.
            ever_synced,
            rescan_rebuilding,
            // u64 → i64: process-local monotonic counter, can't reach 2^63
            seq: seq as i64,
        }
    }
}

// ─── Outbound: core errors → Dart exceptions ────────────────────────────────

impl From<rw::LifecyclePhase> for api_error::LifecyclePhase {
    fn from(p: rw::LifecyclePhase) -> Self {
        match p {
            rw::LifecyclePhase::Provisioning => Self::Provisioning,
            rw::LifecyclePhase::Repairing => Self::Repairing,
            rw::LifecyclePhase::Open => Self::Open,
            // The transient `Rescanning` phase (ADR-0534) gets its OWN bridge variant
            // (NOT folded to `Unknown`): the `bridge_enums_cover_core_variants` policy
            // (extraction_policy.rs) requires every core `LifecyclePhase` variant to be
            // mirrored on the bridge so a host can honestly render "rescanning" — the
            // Dart bindings are regenerated in the same change.
            rw::LifecyclePhase::Rescanning => Self::Rescanning,
            rw::LifecyclePhase::SwitchingServer => Self::SwitchingServer,
            rw::LifecyclePhase::Closing => Self::Closing,
            rw::LifecyclePhase::Wiped => Self::Wiped,
            _ => Self::Unknown,
        }
    }
}

impl From<rw::WalletError> for api_error::WalletApiError {
    fn from(e: rw::WalletError) -> Self {
        // capture the stable code + static display BEFORE consuming the
        // variant — the single source of truth for both is the core
        let code = e.code().to_string();
        let message = e.to_string();
        use api_error::WalletErrorKind as K;
        let kind = match e {
            rw::WalletError::InvalidSeedLength { len } => K::InvalidSeedLength { len: len as u64 },
            rw::WalletError::InvalidMnemonic { word_index } => K::InvalidMnemonic { word_index },
            rw::WalletError::NoMnemonic => K::NoMnemonic,
            rw::WalletError::SeedRequired => K::SeedRequired,
            rw::WalletError::SeedMismatch => K::SeedMismatch,
            rw::WalletError::KeyDerivation => K::KeyDerivation,
            rw::WalletError::WatchOnly => K::WatchOnly,
            rw::WalletError::InvalidViewingKey => K::InvalidViewingKey,
            rw::WalletError::InvalidEndpoint { reason } => K::InvalidEndpoint {
                reason: reason.to_string(),
            },
            rw::WalletError::BirthdayInFuture => K::BirthdayInFuture,
            rw::WalletError::InvalidDbDir { reason } => K::InvalidDbDir {
                reason: reason.to_string(),
            },
            rw::WalletError::InvalidEndpointAuth { reason } => K::InvalidEndpointAuth {
                reason: reason.to_string(),
            },
            rw::WalletError::BroadcastJitterTooLong { max_ms, ceiling_ms } => {
                K::BroadcastJitterTooLong { max_ms, ceiling_ms }
            }
            rw::WalletError::AmountOutOfRange => K::AmountOutOfRange,
            rw::WalletError::MemoTooLong { len, max } => K::MemoTooLong {
                len: len as u64,
                max: max as u64,
            },
            rw::WalletError::ReservedMemoNotSendable => K::ReservedMemoNotSendable,
            rw::WalletError::MemoRequiresShieldedRecipient => K::MemoRequiresShieldedRecipient,
            rw::WalletError::AddressInvalid => K::AddressInvalid,
            rw::WalletError::MemoInvalid => K::MemoInvalid,
            rw::WalletError::MemoConflict => K::MemoConflict,
            rw::WalletError::ZeroValuedTransparentOutput => K::ZeroValuedTransparentOutput,
            rw::WalletError::PaymentUriInvalid => K::PaymentUriInvalid,
            rw::WalletError::TxidInvalid => K::TxidInvalid,
            rw::WalletError::MachineMemoScopeInvalid { reason } => K::MachineMemoScopeInvalid {
                // `reason` is a core-authored `&'static str` — never host input,
                // never a memo byte (§5.4).
                reason: reason.to_owned(),
            },
            rw::WalletError::KeystoreUnavailable => K::KeystoreUnavailable,
            // FR-47: a key store that did not answer inside the SDK's bound
            // rides the EXISTING kind (ABI unchanged); `code` (RW-KEY-008) is
            // what tells it from a locked one.
            rw::WalletError::KeychainTimeout { .. } => K::KeystoreUnavailable,
            rw::WalletError::KeystoreInconsistent {
                permanently_invalidated,
            } => K::KeystoreInconsistent {
                permanently_invalidated,
            },
            rw::WalletError::SealVersionUnsupported { found } => {
                K::SealVersionUnsupported { found }
            }
            rw::WalletError::SealInvalid => K::SealInvalid,
            rw::WalletError::VaultAbsent => K::VaultAbsent,
            rw::WalletError::WrapArtifactInvalid => K::WrapArtifactInvalid,
            rw::WalletError::WrapVersionUnsupported { found } => {
                K::WrapVersionUnsupported { found }
            }
            rw::WalletError::NotFound => K::NotFound,
            rw::WalletError::NetworkMismatch => K::NetworkMismatch,
            rw::WalletError::ProvisioningIncomplete => K::ProvisioningIncomplete,
            rw::WalletError::WalletAlreadyExists => K::WalletAlreadyExists,
            rw::WalletError::StoreCorrupt => K::StoreCorrupt,
            rw::WalletError::DiskFull => K::DiskFull,
            rw::WalletError::StoreBusy => K::StoreBusy,
            rw::WalletError::WalletAlreadyOpen => K::WalletAlreadyOpen,
            rw::WalletError::WalletBusy { phase } => K::WalletBusy {
                phase: phase.into(),
            },
            rw::WalletError::InvalidState { phase } => K::InvalidState {
                phase: phase.into(),
            },
            rw::WalletError::WalletOpen => K::WalletOpen,
            rw::WalletError::WipeWithPendingSwap { count } => K::WipeWithPendingSwap { count },
            rw::WalletError::RescanWithInFlightSend => K::RescanWithInFlightSend,
            rw::WalletError::SyncServerNotOffered => K::SyncServerNotOffered,
            rw::WalletError::SyncServerUnreachable => K::SyncServerUnreachable,
            // #390 — the "Check older swap addresses" typed refusal; the reason enum
            // rides its own lockstep-guarded conversion below.
            rw::WalletError::SwapAddressCheckRefused { reason } => K::SwapAddressCheckRefused {
                reason: reason.into(),
            },
            rw::WalletError::Sync { stall } => K::Sync {
                stall: stall.into(),
            },
            // `ironwood-nu63-support.md` §2 / ADR-0013: the host is told in
            // TYPES what the SDK can no longer do. The branch ids are public
            // protocol constants (never user data), carried for a diagnostics
            // surface; the main path renders the §6.1 copy off the kind alone.
            rw::WalletError::NetworkUpgradeUnsupported {
                expected_branch_id,
                endpoint_branch_id,
                judged_at_height,
            } => K::NetworkUpgradeUnsupported {
                expected_branch_id,
                endpoint_branch_id,
                judged_at_height,
            },
            // GRACE-1 (§4p G-1): the grace refusal is ITS OWN kind, distinguishable
            // by type from the upgrade one — the host must never render "update
            // the app" for a server that merely stopped talking. What crosses is a
            // reason and a block COUNT (a difference of two heights the sync
            // surface already carries), never the capable timestamp (G-12).
            rw::WalletError::ConsensusGraceExpired {
                by,
                blocks_since_last_current,
            } => K::ConsensusGraceExpired {
                by: by.into(),
                blocks_since_last_current,
            },
            // And "never checked" is its own kind too: the not-synced-yet story,
            // not an update prompt (§4p item 2).
            rw::WalletError::ConsensusNotEvaluated => K::ConsensusNotEvaluated,
            // FR-40: the bounded sync's refusal while another pass runs.
            rw::WalletError::SyncRunning => K::SyncRunning,
            // §3.2h send taxonomy. InsufficientFunds carries the amounts the host
            // renders (zatoshis → Dart int); they are typed payload, never logged
            // (§5.4 — the static `message`/`code` is what reaches a log line).
            rw::WalletError::InsufficientFunds {
                available,
                required,
                pending_incoming,
            } => K::InsufficientFunds {
                available_zat: available.zat(),
                required_zat: required.zat(),
                pending_incoming_zat: pending_incoming.zat(),
            },
            rw::WalletError::SendAmountRequired => K::SendAmountRequired,
            rw::WalletError::ProposalAlreadyUsed => K::ProposalAlreadyUsed,
            rw::WalletError::ProposalStale => K::ProposalStale,
            rw::WalletError::ProposeFailed => K::ProposeFailed,
            // INC-018 (b), phase-2 P2-2: the retryable class crosses TYPED. This match
            // ends in `_ => K::Unknown`, so a core variant left out here would reach
            // Dart as `unknown` → the generic "check the details" — the incident's
            // own copy, silently; the arm is pinned by
            // `propose_transient_crosses_the_ffi_typed_and_apart_from_propose_failed`.
            rw::WalletError::ProposeTransient => K::ProposeTransient,
            rw::WalletError::SignFailed => K::SignFailed,
            // §3.2i-2 2e-2b-iii: the typed TEX/ZIP-320 gap-limit ceiling (RW-SEND-007). Typed in iii so
            // the FFI taxonomy + bindings were complete ahead of gate-removal (regenerated ONCE, not
            // again in v). REACHABLE now that gate-removal (2e-2b-v-5) removed the slice-A `steps()>1`
            // signing gates: a TEX two-step create that exceeds the ephemeral gap-limit window returns
            // this, and the core maps it here. UX (dual-natured since #315): render ORANGE
            // ("some may free up as transfers confirm, but this may not clear on its own; your funds
            // are safe" — never a bare "wait, then retry": leaked slots don't clear by waiting),
            // NEVER the RED `SendSignFailed`/`couldNotPrepare`
            // dead-end — a ceiling moved NO money, so a red "send failed" risks a double-pay or panic;
            // any "recover" affordance scopes to OTHER prior transfers' funds, never implying THIS send
            // stranded (it spent nothing). The Dart `classifySendFailure` arm landed 2e-2b-iv.
            rw::WalletError::TexSendLimitReached => K::TexSendLimitReached,
            // §6.2 broadcast/outbox taxonomy (inc-2d-3-b-ii). Payload-free.
            rw::WalletError::QueuedSendStale => K::QueuedSendStale,
            rw::WalletError::QueuedSendsFull => K::QueuedSendsFull,
            // payload (the io::Error) deliberately dropped: classification
            // already happened core-side (ENOSPC → DiskFull); the rest is Io
            rw::WalletError::Io(_) => K::Io,
            _ => K::Unknown,
        };
        Self {
            code,
            message,
            kind,
        }
    }
}

impl From<rw::SwapError> for api_error::SwapApiError {
    fn from(e: rw::SwapError) -> Self {
        let code = e.code().to_string();
        let message = e.to_string();
        use api_error::SwapErrorKind as K;
        let kind = match e {
            rw::SwapError::SlippageToleranceTooHigh {
                requested_bps,
                max_bps,
            } => K::SlippageToleranceTooHigh {
                requested_bps,
                max_bps,
            },
            rw::SwapError::QuoteOutOfBounds { side } => K::QuoteOutOfBounds { side: side.into() },
            rw::SwapError::QuoteExpired => K::QuoteExpired,
            rw::SwapError::DestinationInvalid { reason } => K::DestinationInvalid {
                reason: reason.into(),
            },
            rw::SwapError::RequestInvalid { reason } => K::RequestInvalid {
                reason: reason.to_string(),
            },
            rw::SwapError::ProviderUnavailable => K::ProviderUnavailable,
            rw::SwapError::ProviderProtocol { reason } => K::ProviderProtocol {
                reason: reason.into(),
            },
            rw::SwapError::SwapDisabled => K::SwapDisabled,
            rw::SwapError::DepositSendFailed => K::DepositSendFailed,
            rw::SwapError::RefundAddressUnavailable => K::RefundAddressUnavailable,
            rw::SwapError::DestinationAddressUnavailable => K::DestinationAddressUnavailable,
            rw::SwapError::SwapStateUnavailable => K::SwapStateUnavailable,
            rw::SwapError::SwapStateBusy => K::SwapStateBusy,
            rw::SwapError::SwapAlreadyInFlight => K::SwapAlreadyInFlight,
            // #397 §3.7 D3: structural watch-only — typed through, never the
            // retryable ProviderUnavailable mask (RW-SWAP-015).
            rw::SwapError::WatchOnly => K::WatchOnly,
            // S8 (R01): the caller-DTO terms compare's refusal (RW-SWAP-016).
            rw::SwapError::QuoteTermsDiffer => K::QuoteTermsDiffer,
            _ => K::Unknown,
        };
        Self {
            code,
            message,
            kind,
        }
    }
}

impl From<rw::QuoteBoundSide> for api_error::QuoteBoundSide {
    fn from(s: rw::QuoteBoundSide) -> Self {
        match s {
            rw::QuoteBoundSide::Zec => Self::Zec,
            rw::QuoteBoundSide::Foreign => Self::Foreign,
            _ => Self::Unknown,
        }
    }
}

impl From<rw::DestinationInvalidReason> for api_error::DestinationInvalidReason {
    fn from(r: rw::DestinationInvalidReason) -> Self {
        match r {
            rw::DestinationInvalidReason::Missing => Self::Missing,
            rw::DestinationInvalidReason::NotAllowedForDirection => Self::NotAllowedForDirection,
            rw::DestinationInvalidReason::ZcashAddressNotAllowed => Self::ZcashAddressNotAllowed,
            rw::DestinationInvalidReason::Oversized => Self::Oversized,
            _ => Self::Unknown,
        }
    }
}

impl From<rw::SwapAddressCheckRefusal> for api_error::SwapAddressCheckRefusal {
    fn from(r: rw::SwapAddressCheckRefusal) -> Self {
        // EXHAUSTIVE-by-name (the lockstep drift guard `bridge_enums_cover_core_variants`):
        // a NEW core refusal reason trips CI until this arm learns it. The core enum is
        // `#[non_exhaustive]`, so the wildcard is unreachable in a lockstep build.
        match r {
            rw::SwapAddressCheckRefusal::SwapDisabled => Self::SwapDisabled,
            rw::SwapAddressCheckRefusal::CheckOutstanding => Self::CheckOutstanding,
            _ => Self::Unknown,
        }
    }
}

impl From<rw::ProviderProtocolReason> for api_error::ProviderProtocolReason {
    fn from(r: rw::ProviderProtocolReason) -> Self {
        match r {
            rw::ProviderProtocolReason::RefundAddressMismatch => Self::RefundAddressMismatch,
            rw::ProviderProtocolReason::OversizedField => Self::OversizedField,
            rw::ProviderProtocolReason::MalformedAmount => Self::MalformedAmount,
            rw::ProviderProtocolReason::UnknownStatus => Self::UnknownStatus,
            rw::ProviderProtocolReason::DepositAddressInvalid => Self::DepositAddressInvalid,
            rw::ProviderProtocolReason::MissingField => Self::MissingField,
            rw::ProviderProtocolReason::UnexpectedHttpStatus => Self::UnexpectedHttpStatus,
            rw::ProviderProtocolReason::OversizedBody => Self::OversizedBody,
            rw::ProviderProtocolReason::UndecodableResponse => Self::UndecodableResponse,
            rw::ProviderProtocolReason::DepositMemoUnsupported => Self::DepositMemoUnsupported,
            rw::ProviderProtocolReason::SwapNotFound => Self::SwapNotFound,
            rw::ProviderProtocolReason::ReservedId => Self::ReservedId,
            rw::ProviderProtocolReason::RequestEchoMismatch => Self::RequestEchoMismatch,
            _ => Self::Unknown,
        }
    }
}

// ─── Outbound: core swap DTOs → Dart ────────────────────────────────────────

impl From<rw::SwapDirection> for api_swap::SwapDirection {
    fn from(d: rw::SwapDirection) -> Self {
        match d {
            rw::SwapDirection::IntoZec { from } => Self::IntoZec { from: from.into() },
            rw::SwapDirection::OutOfZec { to } => Self::OutOfZec { to: to.into() },
            _ => Self::Unknown,
        }
    }
}

impl From<rw::AssetId> for api_swap::AssetId {
    fn from(a: rw::AssetId) -> Self {
        let rw::AssetId { chain, symbol } = a;
        Self { chain, symbol }
    }
}

impl From<rw::TokenInfo> for api_swap::SwapToken {
    fn from(t: rw::TokenInfo) -> Self {
        // Exhaustive destructure: a new core field is a compile error here, never a silently
        // dropped picker field (the drift guard the other swap DTOs use).
        let rw::TokenInfo {
            chain,
            symbol,
            decimals,
            provider_asset_id,
            price_usd,
        } = t;
        Self {
            chain,
            symbol,
            decimals,
            provider_asset_id,
            price_usd,
        }
    }
}

impl From<rw::TokenList> for api_swap::SwapTokenList {
    fn from(l: rw::TokenList) -> Self {
        let rw::TokenList { tokens, fresh } = l;
        Self {
            tokens: tokens.into_iter().map(Into::into).collect(),
            fresh,
        }
    }
}

impl From<rw::SwapQuote> for api_swap::SwapQuote {
    fn from(q: rw::SwapQuote) -> Self {
        let rw::SwapQuote {
            id,
            deposit_address,
            deposit_memo,
            expires_at,
            amount_in,
            min_amount_out,
            zec_side,
            refund_to,
            disclosure,
            binding,
        } = q;
        Self {
            id: id.as_str().to_string(),
            deposit_address,
            deposit_memo,
            // u64 → i64: unix seconds (see SyncStamp note)
            expires_at: expires_at as i64,
            amount_in,
            min_amount_out,
            zec_side_zat: zec_side.zat(),
            refund_to,
            disclosure: disclosure.into(),
            // FR-17: opaque bytes (service-minted; always Some on a served quote).
            binding: binding.map(|b| b.as_bytes().to_vec()),
        }
    }
}

impl From<rw::SwapPrivacyDisclosure> for api_swap::SwapPrivacyDisclosure {
    fn from(d: rw::SwapPrivacyDisclosure) -> Self {
        let rw::SwapPrivacyDisclosure {
            ends_shielded,
            deshields,
            provider_legs_transparent,
            provider_sees,
        } = d;
        Self {
            ends_shielded,
            deshields,
            provider_legs_transparent,
            provider_sees: provider_sees.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<rw::DisclosureItem> for api_swap::DisclosureItem {
    fn from(i: rw::DisclosureItem) -> Self {
        match i {
            rw::DisclosureItem::CrossAssetLink => Self::CrossAssetLink,
            rw::DisclosureItem::Amounts => Self::Amounts,
            rw::DisclosureItem::DestinationAddress => Self::DestinationAddress,
            rw::DisclosureItem::SourceAddress => Self::SourceAddress,
            rw::DisclosureItem::IpUnlessTor => Self::IpUnlessTor,
            _ => Self::Unknown,
        }
    }
}

impl From<rw::SwapStatus> for api_swap::SwapStatus {
    fn from(s: rw::SwapStatus) -> Self {
        match s {
            rw::SwapStatus::PendingDeposit { expires_at } => Self::PendingDeposit {
                expires_at: expires_at as i64,
            },
            rw::SwapStatus::UnderDeposited {
                received,
                missing,
                deadline,
            } => Self::UnderDeposited {
                received,
                missing,
                deadline: deadline as i64,
            },
            rw::SwapStatus::DepositDetected => Self::DepositDetected,
            rw::SwapStatus::Processing => Self::Processing,
            rw::SwapStatus::Success {
                out_txid,
                realized_slippage_bps,
            } => Self::Success {
                out_txid,
                realized_slippage_bps,
            },
            rw::SwapStatus::Refunded { refund_txid } => Self::Refunded { refund_txid },
            rw::SwapStatus::Failed { code } => Self::Failed { code: code.into() },
            _ => Self::Unknown,
        }
    }
}

impl From<rw::SwapFailureCode> for api_swap::SwapFailureCode {
    fn from(c: rw::SwapFailureCode) -> Self {
        match c {
            rw::SwapFailureCode::ProviderFailure => Self::ProviderFailure,
            rw::SwapFailureCode::ProviderProtocol => Self::ProviderProtocol,
            rw::SwapFailureCode::Expired => Self::Expired,
            rw::SwapFailureCode::NotFound => Self::NotFound,
            _ => Self::Unknown,
        }
    }
}

impl From<rw::SwapOutcome> for api_swap::SwapOutcome {
    fn from(o: rw::SwapOutcome) -> Self {
        // Exhaustive BOTH ways (no Unknown arm): the core enum is closed, and
        // core-side already maps an unrecognized STORED value to `None` before
        // this conversion ever runs — a new core variant fails compile HERE.
        match o {
            rw::SwapOutcome::Success => Self::Success,
            rw::SwapOutcome::Refunded => Self::Refunded,
            rw::SwapOutcome::Failed => Self::Failed,
        }
    }
}

impl From<api_swap::SwapOutcome> for rw::SwapOutcome {
    fn from(o: api_swap::SwapOutcome) -> Self {
        // The inbound half `recordSwapOutcome` lowers through — exhaustive.
        match o {
            api_swap::SwapOutcome::Success => Self::Success,
            api_swap::SwapOutcome::Refunded => Self::Refunded,
            api_swap::SwapOutcome::Failed => Self::Failed,
        }
    }
}

impl From<rw::InFlightSwap> for api_swap::SwapRecord {
    fn from(r: rw::InFlightSwap) -> Self {
        // Exhaustive destructure: a core field added to the durable row fails
        // compile HERE, so the Dart shape is updated in the same change.
        let rw::InFlightSwap {
            swap_id,
            out_of_zec,
            created_at,
            deposit_deadline,
            expires_at_wall,
            outcome,
        } = r;
        Self {
            id: swap_id,
            direction: if out_of_zec {
                api_swap::SwapRecordDirection::OutOfZec
            } else {
                api_swap::SwapRecordDirection::IntoZec
            },
            created_at,
            deposit_deadline,
            expires_at: expires_at_wall,
            outcome: outcome.map(Into::into),
        }
    }
}

// ─── Inbound: Dart config/request → core (consumed by the wallet handle —
//     increment 2; conversions land WITH the shapes so drift is caught from
//     day one) ───────────────────────────────────────────────────────────────

impl From<api_config::Network> for rw::Network {
    fn from(n: api_config::Network) -> Self {
        match n {
            api_config::Network::Main => Self::Main,
            api_config::Network::Test => Self::Test,
        }
    }
}

// ─── P3-13: the sync-server picker (`sync-server-picker.md` §3.3) ────────────

/// An offered entry whose key is ALREADY `Zeroizing` (crypto audit H5; the
/// security review of the built diff): the config door wraps every entry's
/// value the moment it destructures the config, so none of its earlier early
/// returns — nor a failure on an earlier entry — drops a plain `String` key.
struct WrappedSyncServer {
    id: String,
    label: String,
    url: String,
    auth_header: Option<String>,
    auth_value: Option<rw::Zeroizing<String>>,
}

impl From<api_config::SyncServer> for WrappedSyncServer {
    fn from(s: api_config::SyncServer) -> Self {
        let api_config::SyncServer {
            id,
            label,
            url,
            auth_header,
            auth_value,
        } = s;
        Self {
            id,
            label,
            url,
            auth_header,
            // A move into the wrapper — no copy of the key.
            auth_value: auth_value.map(rw::Zeroizing::new),
        }
    }
}

impl TryFrom<api_config::SyncServer> for rw::SyncServer {
    type Error = rw::WalletError;

    fn try_from(s: api_config::SyncServer) -> Result<Self, Self::Error> {
        WrappedSyncServer::from(s).try_into()
    }
}

impl TryFrom<WrappedSyncServer> for rw::SyncServer {
    type Error = rw::WalletError;

    fn try_from(s: WrappedSyncServer) -> Result<Self, Self::Error> {
        let WrappedSyncServer {
            id,
            label,
            url,
            auth_header,
            auth_value,
        } = s;
        // Size-cap BEFORE the validator sees the bytes (§4.6), like the
        // custom choice below; a host DTO is hostile-shaped input too.
        if url.len() > rw::constants::SYNC_SERVER_URL_MAX_BYTES {
            return Err(rw::WalletError::InvalidEndpoint {
                reason: "sync server url is longer than SYNC_SERVER_URL_MAX_BYTES",
            });
        }
        // BOTH-OR-NEITHER, the `endpoint_auth` rule applied to the list: a
        // gated entry without its value would be offered unauthenticated and
        // fail on the network as an opaque refusal.
        let auth = match (auth_header, auth_value) {
            (None, None) => None,
            (Some(h), Some(v)) => Some(rw::EndpointAuth::from_zeroizing(h, v)?),
            _ => {
                return Err(rw::WalletError::InvalidEndpointAuth {
                    reason: "sync server auth needs BOTH a header name and a value",
                });
            }
        };
        rw::SyncServer::new(
            rw::SyncServerId::new(id)?,
            label,
            rw::LightServerEndpoint::new(url)?,
            auth,
        )
    }
}

impl From<&rw::SyncServer> for api_config::SyncServer {
    /// The HANDLE's view of an offered entry: never the key (`auth_value` is
    /// `None`); `auth_header` says whether the entry is gated.
    fn from(s: &rw::SyncServer) -> Self {
        Self {
            id: s.id().as_str().to_owned(),
            label: s.label().to_owned(),
            url: s.endpoint().as_str().to_owned(),
            auth_header: s.auth_header().map(str::to_owned),
            auth_value: None,
        }
    }
}

impl TryFrom<api_config::SyncServerChoice> for rw::SyncServerChoice {
    type Error = rw::WalletError;

    fn try_from(c: api_config::SyncServerChoice) -> Result<Self, Self::Error> {
        match c {
            api_config::SyncServerChoice::Predefined { id } => {
                Ok(Self::Predefined(rw::SyncServerId::new(id)?))
            }
            api_config::SyncServerChoice::Custom { url, key } => {
                // The user's key (ADR-0568) is wrapped FIRST (crypto audit
                // H5), so no early return drops a plain copy; then every input
                // is bounded before a validator sees it (§4.6).
                let key = key.map(|api_config::SyncServerKey { header, value }| {
                    (header, rw::Zeroizing::new(value))
                });
                if url.len() > rw::constants::SYNC_SERVER_URL_MAX_BYTES {
                    return Err(rw::WalletError::InvalidEndpoint {
                        reason: "custom sync server url is longer than SYNC_SERVER_URL_MAX_BYTES",
                    });
                }
                let endpoint = rw::LightServerEndpoint::new(url)?;
                let key = match key {
                    None => None,
                    // `from_zeroizing` bounds the header and the value before
                    // anything else reads them.
                    Some((header, value)) => Some(rw::EndpointAuth::from_zeroizing(header, value)?),
                };
                Ok(Self::Custom { endpoint, key })
            }
            api_config::SyncServerChoice::Default => Ok(Self::Default),
            api_config::SyncServerChoice::Unknown => Err(rw::WalletError::InvalidEndpoint {
                reason: "unknown sync server choice",
            }),
        }
    }
}

impl From<rw::SyncServerChoice> for api_config::SyncServerChoice {
    fn from(c: rw::SyncServerChoice) -> Self {
        match c {
            rw::SyncServerChoice::Predefined(id) => Self::Predefined {
                id: id.as_str().to_owned(),
            },
            // A RETURNED choice never carries the user's key (ADR-0568): the
            // header says one is saved; the value is always the empty string.
            rw::SyncServerChoice::Custom { endpoint, key } => Self::Custom {
                url: endpoint.as_str().to_owned(),
                key: key.map(|k| api_config::SyncServerKey {
                    header: k.header_name().to_owned(),
                    value: String::new(),
                }),
            },
            rw::SyncServerChoice::Default => Self::Default,
            // `#[non_exhaustive]` core enum; a NEW variant trips the lockstep
            // policy test, so this arm is unreachable in a lockstep build.
            _ => Self::Unknown,
        }
    }
}

impl From<rw::SyncServerFallback> for api_config::SyncServerFallback {
    fn from(f: rw::SyncServerFallback) -> Self {
        match f {
            rw::SyncServerFallback::ChoiceNotOffered { id } => Self::ChoiceNotOffered {
                id: id.as_str().to_owned(),
            },
            rw::SyncServerFallback::ChoiceUnreadable => Self::ChoiceUnreadable,
            rw::SyncServerFallback::ChoiceRefusedByTransport => Self::ChoiceRefusedByTransport,
            _ => Self::Unknown,
        }
    }
}

impl From<rw::SyncServerStatus> for api_config::SyncServerStatus {
    fn from(s: rw::SyncServerStatus) -> Self {
        // EXHAUSTIVE destructuring (the drift-guard contract).
        let rw::SyncServerStatus {
            effective,
            default,
            choice,
            fallback,
        } = s;
        Self {
            effective_url: effective.as_str().to_owned(),
            default_url: default.as_str().to_owned(),
            choice: choice.map(Into::into),
            fallback: fallback.map(Into::into),
        }
    }
}

impl From<rw::SyncServerProbe> for api_config::SyncServerProbe {
    fn from(p: rw::SyncServerProbe) -> Self {
        let rw::SyncServerProbe { tip } = p;
        Self { tip: tip.value() }
    }
}

/// [`api_config::reference_sync_servers`]: the catalog's entries as DTOs the
/// host passes into `WalletConfig.syncServers` (and appends its own to).
/// MEMBERSHIP has one producer — the core's own `SyncServerCatalog::reference`;
/// every entry is public, so no DTO carries a key (ADR-0568).
pub(crate) fn reference_sync_servers(
    network: api_config::Network,
) -> Result<Vec<api_config::SyncServer>, api_error::WalletApiError> {
    let network: rw::Network = network.into();
    let validated = rw::SyncServerCatalog::reference(network)?;
    Ok(validated.iter().map(api_config::SyncServer::from).collect())
}

/// [`WalletHandle::sync_servers`](crate::api::wallet::WalletHandle::sync_servers):
/// the offered list as DTOs, never with the key.
pub(crate) fn sync_servers_of(wallet: &rw::Wallet) -> Vec<api_config::SyncServer> {
    wallet.sync_servers().iter().map(Into::into).collect()
}

/// [`WalletHandle::sync_server_status`](crate::api::wallet::WalletHandle::sync_server_status).
pub(crate) fn sync_server_status_of(wallet: &rw::Wallet) -> api_config::SyncServerStatus {
    crate::status_spelling::as_offered(wallet.sync_server_status(), wallet.sync_servers())
}

/// [`WalletHandle::probe_sync_server`](crate::api::wallet::WalletHandle::probe_sync_server):
/// the choice through its door, the probe, the answer as a DTO.
pub(crate) async fn probe_sync_server(
    wallet: &rw::Wallet,
    choice: api_config::SyncServerChoice,
) -> Result<api_config::SyncServerProbe, api_error::WalletApiError> {
    let choice: rw::SyncServerChoice = choice.try_into()?;
    wallet
        .probe_sync_server(&choice)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

/// [`WalletHandle::switch_sync_server`](crate::api::wallet::WalletHandle::switch_sync_server):
/// the core's consuming switch, its refusal folded into "the handle to keep
/// (if any) plus the typed error". A choice the door refuses hands the
/// wallet straight back — nothing was attempted.
pub(crate) async fn switch_sync_server(
    wallet: rw::Wallet,
    choice: api_config::SyncServerChoice,
) -> Result<rw::Wallet, (Option<rw::Wallet>, api_error::WalletApiError)> {
    let choice: rw::SyncServerChoice = match choice.try_into() {
        Ok(c) => c,
        Err(e) => return Err((Some(wallet), api_error::WalletApiError::from(e))),
    };
    wallet
        .switch_sync_server(choice)
        .await
        .map_err(|rw::SwitchRefused { wallet, error }| {
            (wallet, api_error::WalletApiError::from(error))
        })
}

impl From<api_config::SeedPersistence> for rw::SeedPersistence {
    fn from(p: api_config::SeedPersistence) -> Self {
        match p {
            api_config::SeedPersistence::SealedKeychain => Self::SealedKeychain,
            api_config::SeedPersistence::None => Self::None,
        }
    }
}

impl From<api_config::JitterPolicy> for rw::JitterPolicy {
    fn from(j: api_config::JitterPolicy) -> Self {
        match j {
            api_config::JitterPolicy::None => Self::None,
            api_config::JitterPolicy::Uniform { max_ms } => Self::Uniform {
                max_ms: u64::from(max_ms),
            },
        }
    }
}

impl TryFrom<api_config::TorPolicy> for rw::TorPolicy {
    type Error = rw::WalletError;

    fn try_from(t: api_config::TorPolicy) -> Result<Self, rw::WalletError> {
        Ok(match t {
            api_config::TorPolicy::Off => Self::Off,
            api_config::TorPolicy::Preferred { runtime } => Self::Preferred {
                runtime: runtime.try_into()?,
            },
            api_config::TorPolicy::Required { runtime } => Self::Required {
                runtime: runtime.try_into()?,
            },
        })
    }
}

impl TryFrom<api_config::TorRuntimeConfig> for rw::TorRuntime {
    type Error = rw::WalletError;

    fn try_from(r: api_config::TorRuntimeConfig) -> Result<Self, rw::WalletError> {
        match r {
            api_config::TorRuntimeConfig::ExternalSocks5 { addr } => {
                // host:port tops out far below this; an unbounded string from
                // a buggy host is refused at the door (W3 unstable-env fold)
                if addr.len() > rw::constants::SOCKS_ADDR_MAX_BYTES {
                    return Err(rw::WalletError::InvalidEndpoint {
                        reason: "socks5 address too long",
                    });
                }
                // T0-6: REFUSED AT THE CONFIG DOOR, honestly and early.
                //
                // `ExternalSocks5` is the ONLY runtime a Dart host can express,
                // and no dialer is built for it — `net::dialer::runtime_dialer`
                // returns `UnsupportedRuntime` for it, and `resolve_dialer`
                // propagates that with a `?`, so `Preferred { ExternalSocks5 }`
                // never even reaches the clearnet fallback its own name
                // promises. A host that asked for Tor therefore got, before this
                // change, a wallet that appeared to configure fine and then
                // failed at the first dial with a runtime-shaped error — the
                // worst place to learn that a privacy setting is not wired.
                //
                // Refusing here means `validate_wallet_config` — the call a host
                // makes BEFORE opening anything — says so, with a reason a
                // developer can act on. The consequence, stated plainly: a Dart
                // host has no Tor option in this build at all. That is the
                // truth; the previous behaviour dressed it as a transient
                // network fault. A Rust host can still supply
                // `TorRuntime::Dialer` (ADR-0526), which is wired and which this
                // door never sees.
                //
                // `InvalidEndpoint` (RW-CFG-001) rather than a new kind: the
                // field IS an endpoint, the length check one line above already
                // rejects it that way, and a new error variant would mean a
                // bridge-lockstep regeneration for a message.
                let _ = addr;
                Err(rw::WalletError::InvalidEndpoint {
                    reason: "external socks5 tor runtime is not built in this SDK — \
                             use TorPolicy.off, or supply a host dialer from Rust",
                })
            }
            // FR-29 spec §2.1: the dialer the host's native library registered
            // through the C contract, read from the cabi module's registry. A
            // `None` (nothing registered — the host validated before it
            // registered, or cleared) is the door's E1 refusal, EARLIER than
            // the core's own check would make it (`config::validate_transport`
            // asks the same question of a `HostDialer` runtime): the same
            // `RW-CFG-001`, never a first-dial failure, never clearnet.
            api_config::TorRuntimeConfig::HostDialer => {
                crate::net_dialer_cabi::registered_host_dialer()
                    .map(rw::TorRuntime::HostDialer)
                    .ok_or(rw::WalletError::InvalidEndpoint {
                        reason: "no host dialer registered — register before validating the config",
                    })
            }
        }
    }
}

/// The `validate_wallet_config` api fn's engine (api/ files never reference
/// the core crate): full conversion through the core's validating
/// constructors, success result discarded.
pub(crate) fn validate_wallet_config(
    c: api_config::WalletConfig,
) -> Result<(), api_error::WalletApiError> {
    rw::WalletConfig::try_from(c)
        .map(|_| ())
        .map_err(Into::into)
}

impl TryFrom<api_config::WalletConfig> for rw::WalletConfig {
    type Error = rw::WalletError;

    fn try_from(c: api_config::WalletConfig) -> Result<Self, Self::Error> {
        let api_config::WalletConfig {
            db_dir,
            network,
            endpoint_url,
            endpoint_auth_header,
            endpoint_auth_value,
            tor,
            seed_persistence,
            birthday_height,
            broadcast_jitter,
            machine_memo_prefixes,
            sync_servers,
        } = c;
        // The host's key is wrapped FIRST (crypto audit H5): the door's early
        // returns below drop a `Zeroizing`, never a plain `String`.
        let endpoint_auth_value = endpoint_auth_value.map(rw::Zeroizing::new);
        // …and every offered entry's key, for the same reason.
        let sync_servers: Option<Vec<WrappedSyncServer>> =
            sync_servers.map(|list| list.into_iter().map(WrappedSyncServer::from).collect());
        // The db_dir shape gate (RW-CFG-003, #324): an empty/relative dir
        // would `create_dir_all` against the process CWD downstream — mobile
        // fails closed (unwritable cwd), but desktop silently plants the
        // wallet under the launch directory and the next launch from another
        // cwd offers a fresh CREATE over a funded wallet. Shared with the
        // dbDir-only entries (see [`validate_db_dir`]); `Path::is_absolute`
        // is platform-correct (POSIX `/…`; Windows drive/UNC prefixes).
        validate_db_dir(&db_dir)?;
        let cfg = Self {
            db_dir: db_dir.into(),
            network: network.into(),
            // the ONE validation door for host-supplied endpoint URLs
            endpoint: rw::LightServerEndpoint::new(endpoint_url)?,
            // BOTH-OR-NEITHER, refused at the door. A host that sets only one
            // half means to authenticate and would otherwise get a wallet that
            // silently talks to a gated endpoint with no credential — which
            // fails later, on the network, as an opaque refusal. The core's
            // constructor is the ONE validation door for the pair (a valid
            // header name, and a value with no CR/LF/NUL — the header-injection
            // check), exactly as `LightServerEndpoint::new` is for the URL.
            endpoint_auth: match (endpoint_auth_header, endpoint_auth_value) {
                (None, None) => None,
                (Some(h), Some(v)) => Some(rw::EndpointAuth::from_zeroizing(h, v)?),
                _ => {
                    return Err(rw::WalletError::InvalidEndpointAuth {
                        reason: "endpoint auth needs BOTH a header name and a value",
                    });
                }
            },
            tor: tor.try_into()?,
            seed_persistence: seed_persistence.into(),
            birthday: birthday_height.map(rw::BlockHeight::new),
            broadcast_jitter: broadcast_jitter.into(),
            // FR-27: every registered prefix through the core's validating
            // constructor, so an unusable read scope (an EMPTY prefix, which
            // would match every memo ever written) fails HERE — at
            // create/open/validate — and never as a silent always-empty read.
            //
            // The COUNT bound runs here TOO, not only at the core's door into
            // `Inner`. It used to run only there, which made
            // `validateWalletConfig` — the settings-form pre-flight whose whole
            // job is "would this config work?" — answer Ok for a scope that
            // `create` then refuses. A pre-flight door that disagrees with the
            // real one is worse than no pre-flight (FR-27 review, found by two
            // angles). Both doors now share the same predicate.
            machine_memo_prefixes: {
                // COUNT FIRST, before materializing anything (§4.6
                // size-cap-before-alloc): a host passing a million prefixes
                // would otherwise allocate every validated one and only then
                // learn there are too many. The core validator checks count
                // first for the same reason; this door has to match it.
                if machine_memo_prefixes.len() > rw::constants::MACHINE_MEMO_PREFIX_MAX_COUNT {
                    return Err(rw::WalletError::MachineMemoScopeInvalid {
                        reason: "more prefixes registered than the maximum",
                    });
                }
                let prefixes = machine_memo_prefixes
                    .into_iter()
                    .map(rw::MachineMemoPrefix::new)
                    .collect::<Result<Vec<_>, _>>()?;
                // Belt-and-braces, and the SSOT for the rule: if the bound ever
                // moves, it moves in one place and this door inherits it.
                rw::validate_machine_memo_scope(&prefixes)?;
                prefixes
            },
            // P3-13: the offered list, every entry through the core's
            // constructors (the ONE validation door per field) and the list
            // through `validate_sync_servers` — HERE as well as at the core's
            // constructors, so `validateWalletConfig` (the settings pre-flight)
            // cannot answer Ok for a list `create`/`open` then refuses (the
            // FR-27 lesson above). `null` and empty mean the same: nothing
            // offered beyond the default and a custom entry.
            sync_servers: {
                let offered = sync_servers.unwrap_or_default();
                // COUNT FIRST, before converting anything (§4.6 size-cap
                // before alloc — the machine-memo door's own rule above; the
                // security review's INFO): the list predicate re-checks it.
                if offered.len() > rw::constants::SYNC_SERVERS_MAX {
                    return Err(rw::WalletError::InvalidEndpoint {
                        reason: "more sync servers offered than SYNC_SERVERS_MAX",
                    });
                }
                let list = offered
                    .into_iter()
                    .map(rw::SyncServer::try_from)
                    .collect::<Result<Vec<_>, _>>()?;
                rw::validate_sync_servers(&list)?;
                list
            },
        };
        // The core's cross-field rules — the SAME list create, watch-only
        // create and open run (`WalletConfig::validate`), so the
        // `validateWalletConfig` pre-flight can never accept what a
        // constructor refuses. Twice a new rule reached the constructors and
        // not this door (the jitter ceiling, N02; the plaintext-transport
        // rule, found by N02's security review); the shared list ends that.
        cfg.validate()?;
        Ok(cfg)
    }
}

/// The §4.3a measured-tier label — the ONE place a [`rw::VaultTier`] becomes a
/// stable Dart string, shared by the production `custody_disclosure` and the
/// diagnostic `selftest_seed_custody` (DRY: one mapping, so the two surfaces can
/// never drift). `VaultTier` is `#[non_exhaustive]`, so the wildcard is
/// mandatory — a newer core renders an honest `"unknown"` (the predicates
/// `eraseAssurance`/`degraded` are the load-bearing signals;
/// the label is diagnostic). `None` (no platform key vault — headless desktop)
/// is rendered `"none"` by the caller, never routed through this map.
pub(crate) fn vault_tier_label(tier: rw::VaultTier) -> &'static str {
    match tier {
        rw::VaultTier::StrongBox => "strongbox",
        rw::VaultTier::Tee => "tee",
        rw::VaultTier::SoftwareKeystore => "software_keystore",
        rw::VaultTier::AppleKeychain => "apple_keychain",
        // FR-14 chunk 2 — the hardware-held Apple tier (distinct from the
        // best-effort raw `apple_keychain`; ADR-0571).
        rw::VaultTier::AppleSecureEnclave => "apple_secure_enclave",
        _ => "unknown",
    }
}

// ─── Wallet handle (api/wallet.rs) — the FIRST stateful FFI surface ──────────
//
// These thin async helpers own EVERY core-crate interaction the opaque handle
// needs, so `api/wallet.rs` references only the opaque `rw::Wallet` TYPE (an
// opaque must name its content) and never the core config/seed/error
// constructors. In particular `SeedSource` is constructed HERE — never in
// `api/` (the `ffi_surface_exposes_no_key_types` gate denies the token, and
// seed material has no business on the FRB-scanned surface). Conversions go
// through the `From`/`TryFrom` impls above (the single code/DTO table).

/// Create a brand-new wallet (fresh seed generated in Rust via `OsRng`) at
/// `config.dbDir`, then open it. The seed is generated, sealed and used
/// ENTIRELY Rust-side — nothing key-bearing crosses the bridge here (restore
/// from an existing phrase is the sibling [`restore_wallet`] below).
pub(crate) async fn create_generated_wallet(
    config: api_config::WalletConfig,
) -> Result<rw::Wallet, api_error::WalletApiError> {
    let cfg = rw::WalletConfig::try_from(config)?;
    refuse_while_severing(&cfg.db_dir, rw::WalletError::WalletAlreadyOpen).await?;
    rw::Wallet::create(cfg, rw::SeedSource::Generate)
        .await
        .map_err(Into::into)
}

/// Restore a wallet at `config.dbDir` from a BIP39 recovery phrase — the engine
/// behind `WalletHandle::restore`, the ONE sanctioned *inbound* key-material
/// crossing (spec §3.3, §10), the symmetric counterpart to the outbound
/// `reveal_mnemonic`. The words arrive as a `Vec<String>` (a word LIST, never a
/// single Dart string); they are joined into the space-separated phrase the
/// audited `bip39` parser normalizes and moved into a `SeedSource::Mnemonic`,
/// which owns them (and the passphrase) in zeroizing buffers from construction.
/// The join + `SeedSource` construction live HERE, NOT in `api/`, so no key TYPE
/// (`SeedSource`/`Zeroizing`) ever names the FRB-scanned surface
/// (`ffi_surface_exposes_no_key_types`).
///
/// EVERY money-safety property is inherited from the audited core
/// `Wallet::create` path this delegates to — NONE is re-implemented here: BIP39
/// checksum/word validation (errors carry the word INDEX only, never the word,
/// §6.1); the non-clobber `WalletAlreadyExists` on an already-completed store
/// (`create_or_repair`); the `SeedMismatch` verify of a supplied phrase against
/// an interrupted-create remnant; and the no-birthday⇒Sapling-activation floor
/// (a full money-safe scan, NEVER ~tip — `resolve_birthday`, §3.2f). No logging
/// (§5.4 — the words / passphrase are never traced/printed on this path).
///
/// Residue (§10): Dart's own copy of the words can't be zeroized. On the Rust
/// side the inbound words and passphrase are wrapped in `Zeroizing` at entry
/// (S7 B1), so every early exit wipes them; the joined phrase is built at its
/// exact length (no reallocation) and moves straight into `SeedSource`, which
/// keeps it in `Zeroizing`. FRB can't marshal `Zeroizing`, hence the wrap here.
pub(crate) async fn restore_wallet(
    config: api_config::WalletConfig,
    mnemonic_words: Vec<String>,
    passphrase: Option<String>,
) -> Result<rw::Wallet, api_error::WalletApiError> {
    // S7 B1: wiped on every exit below, the early ones included.
    let mnemonic_words = rw::Zeroizing::new(mnemonic_words);
    let mut passphrase = rw::Zeroizing::new(passphrase);
    // Validate the host config at the SAME door (a bad endpoint can't slip a
    // restore through) BEFORE constructing any seed material.
    let cfg = rw::WalletConfig::try_from(config)?;
    refuse_while_severing(&cfg.db_dir, rw::WalletError::WalletAlreadyOpen).await?;
    // SIZE-CAP-BEFORE-ALLOC (§4.6 discipline, mobile robustness): bound the inbound
    // phrase BEFORE the `.join` allocates. A valid BIP39 phrase is ≤24 words and
    // sits far under the seal cap; anything larger can NEVER seal into a wallet
    // (`SeedPayload::new` rejects it post-parse), so reject it HERE — typed, no
    // alloc — rather than let a pathological clipboard paste on a low-memory device
    // transiently allocate a huge join. `joined_len = Σ word bytes + the (n-1)
    // separators` is the exact post-join length, summed without allocating. Reuses
    // the core's `SeedPayload` cap (DRY) so the inbound and at-rest bounds agree.
    let joined_len = mnemonic_words.iter().map(String::len).sum::<usize>()
        + mnemonic_words.len().saturating_sub(1);
    if joined_len > rw::constants::SEAL_MNEMONIC_MAX_BYTES {
        // a `None` index — the phrase is malformed as a WHOLE (too large), not a
        // single bad word; same typed kind/code as any other BIP39 reject.
        return Err(rw::WalletError::InvalidMnemonic { word_index: None }.into());
    }
    // Join into a space-separated phrase (a blank/extra word is a typed error,
    // never a panic). The audited parser does NOT case-fold: an upper-case word is
    // a typed `InvalidMnemonic` with its index, so the HOST UI MUST lowercase each
    // word (we keep the audited bip39 path WHOLE rather than pre-fold key material).
    // `join` sizes its buffer exactly; both secrets move into `Zeroizing` there.
    let phrase = mnemonic_words.join(" ");
    let seed = rw::SeedSource::mnemonic(phrase, passphrase.take());
    rw::Wallet::create(cfg, seed).await.map_err(Into::into)
}

/// Open an EXISTING wallet at `config.dbDir` (no key material needed — the
/// account UFVK is already provisioned in the keyed store).
pub(crate) async fn open_wallet(
    config: api_config::WalletConfig,
) -> Result<rw::Wallet, api_error::WalletApiError> {
    let cfg = rw::WalletConfig::try_from(config)?;
    refuse_while_severing(&cfg.db_dir, rw::WalletError::WalletAlreadyOpen).await?;
    rw::Wallet::open(cfg).await.map_err(Into::into)
}

/// #397 (spec §3.7 D2): create a WATCH-ONLY wallet from an exported UFVK
/// string. Config validated at the SAME door as create/open; the string is
/// decoded INSIDE the core (network-bound, typed `InvalidViewingKey`, never
/// echoed — §5.4) before any store side effect. No key material is
/// constructed here — the UFVK is viewing capability, and it stays a plain
/// String until the audited core decoder consumes it.
pub(crate) async fn create_watch_only_wallet(
    config: api_config::WalletConfig,
    ufvk: String,
    birthday_height: u32,
) -> Result<rw::Wallet, api_error::WalletApiError> {
    let cfg = rw::WalletConfig::try_from(config)?;
    refuse_while_severing(&cfg.db_dir, rw::WalletError::WalletAlreadyOpen).await?;
    rw::Wallet::create_watch_only(cfg, &ufvk, rw::BlockHeight::new(birthday_height))
        .await
        .map_err(Into::into)
}

/// Provision a NEW host-custodied-seed (`SeedPersistence::None`) wallet at
/// `config.dbDir`, pulling the seed from the C-ABI seed port the host registered via
/// `zec_wallet_register_seed_port` (FR-15). The seed crosses native→native from the
/// host's own library into this one and NEVER through Dart — there is no mnemonic and
/// no Dart-side seed (the keys-in-host integration: Relim ADR-0031 / ADR-0013).
///
/// The core `create` path attaches no seed port (its documented `SeedPersistence::None`
/// footgun; qualified by FR-24 — with `freshly_generated` or a configured birthday
/// at/below the bundle's tree-state tail the account imports AT CREATE, otherwise
/// import stays a first-sync job). So this follows the prescribed pattern: pull the
/// seed once for the create-time provisioning (validated `32..=252` by
/// `SeedSource::raw_bytes`/`raw_bytes_fresh`), then
/// `close` and reopen WITH the port attached so signing (and account import on the
/// lazy arms) pull it on demand. The pulled bytes ride a `Zeroizing` buffer and are
/// MOVED into `SeedSource` (no lingering copy; `SeedSource` zeroizes) — never logged,
/// never reaching Dart. A missing port or an unavailable supply is the honest
/// `SeedRequired` (fail-closed, never a silent half-provisioned wallet).
///
/// `freshly_generated` (FR-24, §3.2f — the T2 fresh flow): `true` marks the pulled
/// bytes `SeedSource::FreshRawBytes` — the host's ATTESTATION that the seed was
/// minted for the first time immediately before this call. The create then stamps
/// #356-F4, imports account 0 eagerly offline, and the host may clear its staged
/// seed the moment this returns (the create→first-sync staging window never opens;
/// the two narrow lazy exceptions — an above-tail configured birthday, a retry
/// over a pre-stamp crash remnant — are documented on the FFI door).
/// ⚠️ LYING-HOST CONTRACT: attesting fresh for a seed with prior chain history pins
/// the scan floor at ~now — pre-existing funds/refunds become invisible (recovery is
/// `rescan_from` with an EXPLICIT lower height through the registered port — FR-43 —
/// or wipe + re-create with `false`; a `from = None` rescan re-floors at the
/// stamp). The
/// full contract lives on the core variant + §3.2f; `false` is byte-identically
/// today's restore-shaped create.
pub(crate) async fn create_wallet_with_host_seed(
    mut config: api_config::WalletConfig,
    freshly_generated: bool,
) -> Result<rw::Wallet, api_error::WalletApiError> {
    // A host-custodied-seed wallet is DEFINITIONALLY `SeedPersistence::None`: the seed
    // is host-derived and supplied per-op, never a wallet-local mnemonic to seal. PIN it
    // here so a host that mis-set `SealedKeychain` can NEVER make `Wallet::create` seal
    // the supplied sub-seed at rest — which would defeat the whole keys-in-host model
    // (arch review fold, FR-15). There is no coherent SealedKeychain interpretation of a
    // host-custodied seed, so this enforces the verb's invariant rather than trusting the
    // field.
    config.seed_persistence = api_config::SeedPersistence::None;

    // Validate the config up front (the same door create/open use); build it TWICE so
    // the reopen leg has its own (core `WalletConfig` is not `Clone` — the api DTO is).
    let create_cfg = rw::WalletConfig::try_from(config.clone())?;
    let open_cfg = rw::WalletConfig::try_from(config)?;
    refuse_while_severing(&create_cfg.db_dir, rw::WalletError::WalletAlreadyOpen).await?;
    let port = crate::seed_port_cabi::registered_seed_port()
        .ok_or_else(|| api_error::WalletApiError::from(rw::WalletError::SeedRequired))?;

    // Pull once for create-time provisioning; `mem::take` moves the bytes into
    // `SeedSource` without a lingering copy (the source `Zeroizing` is left holding an
    // empty Vec and drops harmlessly). Both constructors ride the SAME 32..=252
    // bounds door and zeroize on reject; the fresh fork changes create-time
    // behavior only (stamp/eager/posture — §3.2f), never validation or derivation.
    let mut staged = port
        .provide_seed(None)
        .map_err(|_| api_error::WalletApiError::from(rw::WalletError::SeedRequired))?;
    let seed = staged_seed_source(std::mem::take(&mut *staged), freshly_generated)?;

    // create → close → open_with_seed_port (the None-mode contract): provision the
    // seedless store, then reopen WITH the port so key-deriving ops can pull on demand.
    rw::Wallet::create(create_cfg, seed).await?.close().await?;
    rw::Wallet::open_with_seed_port(open_cfg, port)
        .await
        .map_err(Into::into)
}

/// The ONE seed-source fork the host-seed create rides (FR-24) — PURE so
/// the money-semantics inversion is unit-pinned without a port or a store: a
/// flipped condition here would silently give a plain restore FRESH semantics
/// (hidden pre-stamp funds behind the stamp floor) or a fresh create RESTORE
/// semantics (the T2 offline-receive gap re-opens), and nothing downstream
/// red-flags either direction. Both arms ride the same 32..=252 bounds door.
fn staged_seed_source(
    bytes: Vec<u8>,
    freshly_generated: bool,
) -> Result<rw::SeedSource, rw::WalletError> {
    if freshly_generated {
        rw::SeedSource::raw_bytes_fresh(bytes)
    } else {
        rw::SeedSource::raw_bytes(bytes)
    }
}

/// Open an EXISTING host-custodied-seed (`SeedPersistence::None`) wallet at
/// `config.dbDir`, attaching the registered C-ABI seed port (FR-15) so signing +
/// fresh-refund-address derivation can pull the seed on demand. For an
/// already-imported account (the steady state) balance, history, and the receive
/// address need NO seed and never call the port. EXCEPTION — a wallet reopened
/// BEFORE its account import (a restore-shaped create whose first sync hasn't run,
/// or the failed-fresh-create recovery): first sync imports the account THROUGH the
/// port, so the FR-12 staging contract must hold until it completes and
/// `current_address` is typed `SeedRequired` until then. A missing port is the
/// honest `SeedRequired` (a host wiring fault surfaced, never silent).
pub(crate) async fn open_wallet_with_host_seed(
    mut config: api_config::WalletConfig,
) -> Result<rw::Wallet, api_error::WalletApiError> {
    // Host-custodied = `SeedPersistence::None`. On open the on-disk manifest is the
    // authority on persistence, but pin it here too so the host contract is unambiguous
    // and symmetric with `create_wallet_with_host_seed` (FR-15).
    config.seed_persistence = api_config::SeedPersistence::None;
    let cfg = rw::WalletConfig::try_from(config)?;
    refuse_while_severing(&cfg.db_dir, rw::WalletError::WalletAlreadyOpen).await?;
    let port = crate::seed_port_cabi::registered_seed_port()
        .ok_or_else(|| api_error::WalletApiError::from(rw::WalletError::SeedRequired))?;
    rw::Wallet::open_with_seed_port(cfg, port)
        .await
        .map_err(Into::into)
}

/// Rebuild the wallet's data DB at an earlier `from_height` (or `None` = full
/// history → Sapling activation; a freshly-generated wallet's durable creation
/// stamp re-floors `None` at ~creation — §3.2f) — the ADR-0534 post-restore
/// recovery. NO key material crosses this bridge: the only input is a bare block
/// height. A SEALED-KEYCHAIN wallet re-imports from the seed that stays sealed in
/// the keychain, re-read entirely Rust-side. A `SeedPersistence::None` wallet
/// (FR-43) re-imports through the seed port `open_wallet_with_host_seed` attached —
/// the core carries the port HANDLE across the rebuild and pulls at the first sync
/// after it, native-to-native, never here; a `None` wallet whose handle carries no
/// port is rejected typed `SeedRequired` UP FRONT (nothing can serve the import),
/// and per the take-and-replace contract below the handle is then closed, so the
/// host must reopen (`openWalletWithHostSeed`). CONSUMES the handle's wallet and
/// yields the rebuilt one; the caller (the FFI take-and-replace) puts it back on
/// success. `from_height` is the plain ZIP-32 birthday height the host picked (e.g.
/// via `estimateBirthday`); it is resolved + clamped at the next sync through the
/// audited `resolve_birthday` (a height above tip ⇒ `BirthdayInFuture` there,
/// never a silent scan-past).
pub(crate) async fn rescan_wallet(
    wallet: rw::Wallet,
    from_height: Option<u32>,
) -> Result<rw::Wallet, api_error::WalletApiError> {
    let from = from_height.map(rw::BlockHeight::new);
    wallet.rescan_from(from).await.map_err(Into::into)
}

/// The db_dir SHAPE gate (RW-CFG-003, #324) — shared by the full config door
/// (`WalletConfig::try_from`) AND the three dbDir-only entries
/// (`wallet_exists` / `wipe_wallet` / `custody_disclosure`). Those entries
/// deliberately skip TRANSPORT validation (endpoint/Tor — see their docs),
/// but the dbDir shape is a PATH question, exactly what they consume: an
/// unchecked relative dir would probe (or WIPE) against the process cwd —
/// `exists` would answer a misleading `false` and a kill-switch wipe could
/// shred a cwd-relative accident. Empty/relative is a host programming
/// error; typed here on every path that touches the directory.
fn validate_db_dir(db_dir: &str) -> Result<(), rw::WalletError> {
    if db_dir.is_empty() {
        return Err(rw::WalletError::InvalidDbDir { reason: "empty" });
    }
    if !std::path::Path::new(db_dir).is_absolute() {
        return Err(rw::WalletError::InvalidDbDir {
            reason: "must be an absolute path",
        });
    }
    Ok(())
}

/// Cheap, lock-free probe — is a wallet provisioned at `config.dbDir`? (spec
/// §3.1 boot fork.) Existence is a PATH question, so unlike `create`/`open`
/// this looks at ONLY `db_dir`: it deliberately skips the endpoint/Tor
/// validation `WalletConfig::try_from` runs, so a host can ask "is there a
/// wallet here?" before (or independent of) finalizing its transport policy,
/// and a misconfigured endpoint never masquerades as "no wallet". The dbDir
/// SHAPE is still gated ([`validate_db_dir`]) — a relative dir would stat
/// the cwd and answer a misleading `false`. The lock-free / no-SQLCipher-open
/// / `StoreCorrupt`-propagating contract lives in `rw::Wallet::exists`; here
/// we only move `db_dir` across and map the error.
pub(crate) async fn wallet_exists(
    config: api_config::WalletConfig,
) -> Result<bool, api_error::WalletApiError> {
    let db_dir = db_dir_only(config);
    validate_db_dir(&db_dir)?;
    rw::Wallet::exists(db_dir.into()).await.map_err(Into::into)
}

/// Drop a picker choice the bridge will not convert (a closed handle) with its
/// user key moved into `Zeroizing` first (ADR-0568; the security review of the
/// compiled diff).
pub(crate) fn discard_choice(choice: api_config::SyncServerChoice) {
    if let api_config::SyncServerChoice::Custom { key: Some(key), .. } = choice {
        let api_config::SyncServerKey { value, .. } = key;
        drop(rw::Zeroizing::new(value));
    }
}

/// The `dbDir` of a config an entry point uses for nothing else (`exists`,
/// `wipe`, the sever, the custody disclosure) — and every key it carries, the
/// host's and each offered entry's, moved into `Zeroizing` and dropped on the
/// way (the security review of the compiled SRV-KEY diff: these doors dropped
/// them as plain `String`s). No early return, so a caller that must act
/// before anything can fail (the sever's log quiesce) keeps its order.
fn db_dir_only(config: api_config::WalletConfig) -> String {
    let api_config::WalletConfig {
        db_dir,
        endpoint_auth_value,
        sync_servers,
        ..
    } = config;
    drop(endpoint_auth_value.map(rw::Zeroizing::new));
    drop(sync_servers.map(|list| {
        list.into_iter()
            .map(WrappedSyncServer::from)
            .collect::<Vec<_>>()
    }));
    db_dir
}

/// Crypto-shred the wallet at `config.dbDir` (FR-14) — the host's panic-wipe /
/// "delete wallet" primitive. Delegates to the audited core `wipe`/`wipe_force`
/// WHOLE (keychain-first wrap-key sever → file removal; refuses with `WalletOpen`
/// while an instance holds the lock; the fail-closed verify-real-sever guard).
/// `force = true` selects `wipe_force` — the escape hatch that SKIPS the guard
/// for an already-gone custody item; the host must be certain `config.dbDir`
/// matches the wallet's create/open `dbDir` (and, on Apple SE custody, run with
/// the device UNLOCKED — see the core `wipe_force` contract). NO key material
/// crosses this bridge.
///
/// Consumes ONLY `config.dbDir` (like `wallet_exists`) — a destructive
/// crypto-shred / panic-wipe must NEVER be blockable by an unrelated
/// transport-config validation (a host kill-switch may assemble a minimal config,
/// or a stored endpoint may have drifted invalid); so this deliberately SKIPS the
/// `WalletConfig::try_from` endpoint/Tor door create/open use. The fail-closed
/// money guard is the core's verify-real-sever (a `dbDir` that mis-derives the
/// keychain namespace leaves the files untouched), NOT transport validation.
pub(crate) async fn wipe_wallet(
    config: api_config::WalletConfig,
    force: bool,
) -> Result<(), api_error::WalletApiError> {
    // FIRST, before any validation (S5 §3.1, the duress seam): the device log
    // goes Off and the host's log stream closes. This door cannot tell a duress
    // wipe from a user's Delete, so nothing may be logged or streamed on it —
    // a malformed `dbDir` included. An early return added above this line
    // would reopen that window. The host re-arms after a housekeeping Delete.
    crate::device_log::quiesce_for_sever();
    // The dbDir SHAPE gate still applies (unlike the transport door): a
    // relative dir would resolve the SHRED against the process cwd — a
    // destructive primitive must never act on a cwd-relative accident.
    let db_dir = db_dir_only(config);
    validate_db_dir(&db_dir)?;
    let db_dir: std::path::PathBuf = db_dir.into();
    refuse_while_severing(&db_dir, rw::WalletError::WalletOpen).await?;
    if force {
        rw::Wallet::wipe_force(&db_dir).await.map_err(Into::into)
    } else {
        rw::Wallet::wipe(&db_dir).await.map_err(Into::into)
    }
}

/// The duress force-sever (FR-53, stage S16 §3.3) — the door to the audited
/// core `Wallet::sever_custody`, which answers within the deadline and reports
/// every custody outcome as a value. The bridge adds no authority: it quiesces
/// the device log, gates the `dbDir` shape exactly as [`wipe_wallet`] does,
/// clamps the deadline ([`sever_deadline`]) and maps the report field by field.
/// Only an I/O fault, or a bad `dbDir`, is an `Err`. Consumes ONLY
/// `config.dbDir`, for `wipe_wallet`'s reason: a duress call must never be
/// blockable by transport validation.
pub(crate) async fn sever_custody(
    config: api_config::WalletConfig,
    deadline_ms: u32,
) -> Result<api_state::SeverReport, api_error::WalletApiError> {
    // FIRST, before any validation — the same seam as `wipe_wallet` (S5 §3.1):
    // the device log goes Off and the host's log stream closes, so nothing is
    // logged or streamed once a duress call reaches the SDK. An early return
    // added above this line would reopen that window.
    crate::device_log::quiesce_for_sever();
    // A relative dir would resolve the sever against the process cwd.
    let db_dir = db_dir_only(config);
    validate_db_dir(&db_dir)?;
    let db_dir: std::path::PathBuf = db_dir.into();
    let deadline = sever_deadline(deadline_ms);
    let core_dir = db_dir.clone();
    answer_by(db_dir, deadline, async move {
        rw::Wallet::sever_custody(&core_dir, deadline).await
    })
    .await
}

/// How long past its deadline a sever may take to ANSWER, in whole
/// milliseconds (crossed to Dart as `severAnswerGraceMs()`): the core bounds
/// every key-store call by the deadline, and this is the slack for the rest of
/// the call (the lock try, the file reads, the thread hand-off).
pub(crate) const SEVER_ANSWER_GRACE_MS: u32 = 250;
pub(crate) const SEVER_ANSWER_GRACE: std::time::Duration =
    std::time::Duration::from_millis(SEVER_ANSWER_GRACE_MS as u64);

/// How long a door (open, create, restore, wipe…) may spend computing its
/// real-path key while a sever is in flight. Past it, the door REFUSES
/// (fail-closed, ADR-0566): it never hangs on a stalled filesystem, and it
/// never lets a spelling it could not resolve through.
pub(crate) const DOOR_KEY_BOUND: std::time::Duration = SEVER_ANSWER_GRACE;

/// A function from a `dbDir` to its real-path key: [`sever_key`] in
/// production, a stalling or counting stand-in in the rows.
pub(crate) type KeyOf =
    std::sync::Arc<dyn Fn(&std::path::Path) -> std::path::PathBuf + Send + Sync>;

/// What is in flight: every key a running sever registered (counted, since its
/// lexical and real-path keys may be the same path), and how many of those
/// severs could not resolve their real-path key in time.
#[derive(Default)]
struct InFlightState {
    keys: std::collections::HashMap<std::path::PathBuf, usize>,
    unresolved: usize,
}

/// The `dbDir`s whose sever is still running in this process, answered or not
/// (ADR-0566). An entry lives exactly as long as its core call. The rule is
/// FAIL-CLOSED:
/// - a sever registers FIRST: its LEXICAL key and an UNRESOLVED mark, under one
///   lock, before any I/O. It then adds its REAL-PATH key and clears the mark
///   when that key resolves inside its bound; otherwise it stays UNRESOLVED;
/// - a door with nothing in flight does no key work at all;
/// - otherwise, while ANY sever is unresolved, every door refuses whatever its
///   spelling; a door refuses on its lexical key, then computes its real-path
///   key under [`DOOR_KEY_BOUND`] and, in ONE snapshot after that await,
///   refuses if any sever is now unresolved or that key is in flight; a door
///   whose real-path key stalls refuses too. It is a check, not a hold: a sever
///   that starts after a door passed runs beside it, under the core's lock and
///   tombstone.
///
/// The UNRESOLVED gate is PROCESS-WIDE, not per directory: while one sever is
/// still resolving its key, a door at an UNRELATED `dbDir` — another sever
/// included — refuses too. That is the price of failing closed on a key nobody
/// can yet compare. It lasts until that sever's key resolves, or, if the key
/// never does, until its core call ends (ADR-0566 §5, "The cost").
pub(crate) struct SeverRegistry {
    state: std::sync::Mutex<InFlightState>,
    key_of: KeyOf,
}

impl SeverRegistry {
    pub(crate) fn new(key_of: KeyOf) -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            state: std::sync::Mutex::new(InFlightState::default()),
            key_of,
        })
    }

    fn state(&self) -> std::sync::MutexGuard<'_, InFlightState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// `dir`'s real-path key, on the blocking pool, or `None` when it did not
    /// resolve within `bound` (a stall, or a panic in the key function).
    async fn real_key(
        &self,
        dir: &std::path::Path,
        bound: std::time::Duration,
    ) -> Option<std::path::PathBuf> {
        let key_of = std::sync::Arc::clone(&self.key_of);
        let owned = dir.to_path_buf();
        match tokio::time::timeout(bound, tokio::task::spawn_blocking(move || key_of(&owned))).await
        {
            Ok(Ok(key)) => Some(key),
            // The key function panicked, or it stalled past `bound`.
            Ok(Err(_)) | Err(_) => None,
        }
    }

    /// REGISTER FIRST: a sever of `dir` inserts its lexical key and marks itself
    /// UNRESOLVED under one lock, before any key I/O, so every door refuses from
    /// the instant the sever starts until it resolves. `None` when a sever is
    /// already in flight at that key, or any sever is unresolved.
    fn register_unresolved(
        self: &std::sync::Arc<Self>,
        lexical: std::path::PathBuf,
    ) -> Option<InFlight> {
        let mut s = self.state();
        if s.unresolved > 0 || s.keys.contains_key(&lexical) {
            return None;
        }
        *s.keys.entry(lexical.clone()).or_default() += 1;
        s.unresolved += 1;
        Some(InFlight {
            registry: std::sync::Arc::clone(self),
            keys: vec![lexical],
            unresolved: true,
        })
    }

    /// The sever's real-path key resolved: add it and clear its own unresolved
    /// mark, under the lock. `false` when another sever is already in flight at
    /// that real path (a differently spelled sever of the same directory); the
    /// caller then drops `in_flight`, which removes what it registered.
    fn resolve(&self, in_flight: &mut InFlight, real: std::path::PathBuf) -> bool {
        let mut s = self.state();
        if real != in_flight.keys[0] && s.keys.contains_key(&real) {
            return false;
        }
        *s.keys.entry(real.clone()).or_default() += 1;
        in_flight.keys.push(real);
        s.unresolved = s.unresolved.saturating_sub(1);
        in_flight.unresolved = false;
        true
    }

    /// THE guard for a door (see the type's rule). `Err(busy)` refuses.
    pub(crate) async fn refuse(
        &self,
        dir: &std::path::Path,
        busy: rw::WalletError,
    ) -> Result<(), rw::WalletError> {
        {
            let s = self.state();
            if s.keys.is_empty() && s.unresolved == 0 {
                return Ok(());
            }
            if s.unresolved > 0 || s.keys.contains_key(&lexical_key(dir)) {
                return Err(busy);
            }
        }
        let Some(real) = self.real_key(dir, DOOR_KEY_BOUND).await else {
            // It could not resolve in time: refuse, never hang.
            return Err(busy);
        };
        // ONE snapshot after the await: a sever that registered (unresolved)
        // while this door computed its key refuses the door too.
        let s = self.state();
        if s.unresolved > 0 || s.keys.contains_key(&real) {
            return Err(busy);
        }
        Ok(())
    }

    /// How many severs are registered but not yet resolved. (The rows' view.)
    #[cfg(test)]
    pub(crate) fn unresolved(&self) -> usize {
        self.state().unresolved
    }

    /// Is a sever of `dir` in flight, by either of its keys? (The rows' view.)
    #[cfg(test)]
    pub(crate) fn holds(&self, dir: &std::path::Path) -> bool {
        let real = (self.key_of)(dir);
        let s = self.state();
        s.keys.contains_key(&lexical_key(dir)) || s.keys.contains_key(&real)
    }
}

/// The process's registry.
static SEVERS_IN_FLIGHT: std::sync::LazyLock<std::sync::Arc<SeverRegistry>> =
    std::sync::LazyLock::new(|| SeverRegistry::new(std::sync::Arc::new(sever_key)));

/// `dir` with `.` dropped and `..` collapsed, no filesystem I/O (never above
/// the root).
fn lexical_key(dir: &std::path::Path) -> std::path::PathBuf {
    use std::path::Component;
    let mut out = std::path::PathBuf::new();
    for c in dir.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if out.parent().is_some() {
                    out.pop();
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// The ONE key the in-flight set is kept by (ADR-0566): the directory's REAL
/// path, so a symlinked alias (the iOS `/var` → `/private/var` case), a `..`
/// spelling or a trailing `.` meets the same entry. The deepest ancestor that
/// exists is canonicalized and the rest re-attached lexically, so a directory
/// the orphan's sweep already removed still keys like the one it replaced.
/// Only the KEY is canonical; the core always gets the host's own `dbDir`,
/// from which it derives the custody namespace. Blocking filesystem I/O: the
/// registry runs it on the blocking pool under a bound ([`SeverRegistry`]).
/// **Residual:** a directory created between two calls under different
/// spellings when BOTH keys resolved can still give two keys, and so can a case
/// variant `canonicalize` does not fold. A host passes the `dbDir` it opened
/// with.
pub(crate) fn sever_key(dir: &std::path::Path) -> std::path::PathBuf {
    let lexical = lexical_key(dir);
    let mut head = lexical.as_path();
    let mut tail: Vec<&std::ffi::OsStr> = Vec::new();
    loop {
        if let Ok(real) = std::fs::canonicalize(head) {
            let mut key = real;
            for part in tail.iter().rev() {
                key.push(part);
            }
            return key;
        }
        match (head.file_name(), head.parent()) {
            (Some(name), Some(parent)) => {
                tail.push(name);
                head = parent;
            }
            _ => return lexical,
        }
    }
}

/// THE guard every other `dbDir` door takes (open, create, restore, watch-only,
/// host-seed, wipe and wipeForce): while a sever of that directory is still in
/// flight in this process, refuse with the door's own typed "something holds
/// this wallet" answer (`busy`), before any work, by the fail-closed rule of
/// [`SeverRegistry`]. The sever door itself answers `stillRunning` instead (it
/// registers, in [`answer_by`]).
pub(crate) async fn refuse_while_severing(
    dir: &std::path::Path,
    busy: rw::WalletError,
) -> Result<(), rw::WalletError> {
    SEVERS_IN_FLIGHT.refuse(dir, busy).await
}

/// Removes a sever's keys (and its unresolved mark) from the registry when the
/// core call it rides with ends.
pub(crate) struct InFlight {
    registry: std::sync::Arc<SeverRegistry>,
    keys: Vec<std::path::PathBuf>,
    unresolved: bool,
}

impl Drop for InFlight {
    fn drop(&mut self) {
        let mut s = self.registry.state();
        for k in &self.keys {
            let gone = match s.keys.get_mut(k) {
                Some(n) => {
                    *n -= 1;
                    *n == 0
                }
                None => false,
            };
            if gone {
                s.keys.remove(k);
            }
        }
        if self.unresolved {
            s.unresolved = s.unresolved.saturating_sub(1);
        }
    }
}

/// The answer given while the SDK's own sever of that `dbDir` still runs.
fn still_running() -> api_state::SeverReport {
    api_state::SeverReport {
        severed: api_state::SeverOutcome::NotSevered {
            cause: api_state::NotSeveredCause::StillRunning,
        },
        holder: api_state::HolderSeen::Unknown,
        files: api_state::FilesOutcome::StillInUse,
    }
}

/// The HARD wall-clock bound on the WHOLE call (Relim's ask (a); ADR-0566, which
/// amends ADR-0565). The core already bounds its key-store work by `deadline`, but not a
/// stall outside it (a slow filesystem, a saturated blocking pool). So the
/// answer is taken at `deadline + SEVER_ANSWER_GRACE` at the latest.
///
/// The core call cannot be cancelled: it runs to its own end on the blocking
/// pool, possibly still sweeping files in `dbDir`. So an overrun answers the
/// DISTINCT `notSevered(stillRunning)` with `files: stillInUse` (never
/// `leftForHost`, which a host purges on) and `holder: unknown`. The call is
/// spawned as its own task, so it runs to its end. While it runs, `dbDir`
/// stays in [`SEVERS_IN_FLIGHT`], and a second call at that `dbDir` answers
/// `stillRunning` AT ONCE without reaching the store. Letting it through would
/// be wrong: during the orphan's file sweep the key store is free and the
/// orphan holds the lock, so the core would answer `otherProcess` and
/// `leftForHost` while the sweep runs. Once the core call ends, the entry is
/// gone, and the next call reads the true state. Each overrun parks one
/// uncancellable blocking task, so a host retries with a backoff, not in a
/// tight loop.
pub(crate) async fn answer_by<F>(
    db_dir: std::path::PathBuf,
    deadline: std::time::Duration,
    core: F,
) -> Result<api_state::SeverReport, api_error::WalletApiError>
where
    F: std::future::Future<Output = Result<rw::SeverReport, rw::WalletError>> + Send + 'static,
{
    answer_by_in(&SEVERS_IN_FLIGHT, db_dir, deadline, core).await
}

/// [`answer_by`] over a given registry (the rows use private ones).
pub(crate) async fn answer_by_in<F>(
    registry: &std::sync::Arc<SeverRegistry>,
    db_dir: std::path::PathBuf,
    deadline: std::time::Duration,
    core: F,
) -> Result<api_state::SeverReport, api_error::WalletApiError>
where
    F: std::future::Future<Output = Result<rw::SeverReport, rw::WalletError>> + Send + 'static,
{
    let bound = deadline + SEVER_ANSWER_GRACE;
    let started = std::time::Instant::now();
    // REGISTER FIRST (lexical key + UNRESOLVED, one lock, no I/O): from here
    // every door refuses. Then the real-path key, filesystem I/O inside the
    // bound; resolved, it joins the entry and clears the mark. Unresolved, the
    // sever stays UNRESOLVED and every door refuses until it ends (ADR-0566).
    let Some(mut in_flight) = registry.register_unresolved(lexical_key(&db_dir)) else {
        return Ok(still_running());
    };
    if let Some(real) = registry.real_key(&db_dir, bound).await
        && !registry.resolve(&mut in_flight, real)
    {
        // A differently spelled sever of the same directory is in flight.
        return Ok(still_running());
    }
    // Held BY the spawned task: dropped (and the entry removed) when the core
    // call returns, when it panics (the task's future unwinds), and when the
    // runtime shuts down and cancels the task. That last case removes the entry
    // while the blocking-pool work may run on; the runtime lives as long as the
    // process, so it only happens at exit.
    let task = tokio::spawn(async move {
        let _in_flight = in_flight;
        core.await
    });
    match tokio::time::timeout(bound.saturating_sub(started.elapsed()), task).await {
        Ok(Ok(answered)) => answered.map(Into::into).map_err(Into::into),
        // The core call panicked: re-raise it, as an un-spawned call would.
        Ok(Err(joined)) if joined.is_panic() => std::panic::resume_unwind(joined.into_panic()),
        // Cancelled only when the runtime shuts down under it.
        Ok(Err(_cancelled)) => Ok(still_running()),
        Err(_overran) => Ok(still_running()),
    }
}

/// A host's sever deadline, crossed as whole milliseconds, clamped to
/// `[0, KEYCHAIN_WIPE_BUDGET]`: a host cannot ask for more than the SDK would
/// spend on the key store, and `0` stays `0` (the core then makes no key-store
/// call and answers `notSevered(pastDeadline)`).
pub(crate) fn sever_deadline(deadline_ms: u32) -> std::time::Duration {
    std::time::Duration::from_millis(u64::from(deadline_ms)).min(rw::KEYCHAIN_WIPE_BUDGET)
}

impl From<rw::SeverReport> for api_state::SeverReport {
    fn from(r: rw::SeverReport) -> Self {
        // EXHAUSTIVE destructuring (the file-top drift-guard contract): a field
        // added to the core report fails compile here.
        let rw::SeverReport {
            severed,
            holder,
            files,
        } = r;
        Self {
            severed: severed.into(),
            holder: holder.into(),
            files: files.into(),
        }
    }
}

impl From<rw::SeverOutcome> for api_state::SeverOutcome {
    fn from(o: rw::SeverOutcome) -> Self {
        // EXHAUSTIVE-by-name (the lockstep drift guard `bridge_enums_cover_core_variants`).
        // The core enum is `#[non_exhaustive]`, so the wildcard is required and
        // unreachable in a lockstep build; Dart reads `unknown` as NOT severed.
        match o {
            rw::SeverOutcome::Severed { count } => Self::Severed {
                // A count is never an index; saturate rather than wrap.
                count: u32::try_from(count).unwrap_or(u32::MAX),
            },
            rw::SeverOutcome::SeveredUnproven { reason } => Self::SeveredUnproven {
                reason: reason.into(),
            },
            rw::SeverOutcome::AlreadyGone => Self::AlreadyGone,
            rw::SeverOutcome::NotSevered { cause } => Self::NotSevered {
                cause: cause.into(),
            },
            _ => Self::Unknown,
        }
    }
}

impl From<rw::UnprovenReason> for api_state::UnprovenReason {
    fn from(r: rw::UnprovenReason) -> Self {
        match r {
            rw::UnprovenReason::CountUnreadable => Self::CountUnreadable,
            _ => Self::Unknown,
        }
    }
}

impl From<rw::NotSeveredCause> for api_state::NotSeveredCause {
    fn from(c: rw::NotSeveredCause) -> Self {
        match c {
            rw::NotSeveredCause::NothingSevered => Self::NothingSevered,
            rw::NotSeveredCause::Timeout => Self::Timeout,
            rw::NotSeveredCause::Busy => Self::Busy,
            rw::NotSeveredCause::PastDeadline => Self::PastDeadline,
            rw::NotSeveredCause::VaultAbsent => Self::VaultAbsent,
            rw::NotSeveredCause::KeystoreUnavailable => Self::KeystoreUnavailable,
            _ => Self::Unknown,
        }
    }
}

impl From<rw::HolderSeen> for api_state::HolderSeen {
    fn from(h: rw::HolderSeen) -> Self {
        match h {
            rw::HolderSeen::None => Self::None,
            rw::HolderSeen::ThisProcess => Self::ThisProcess,
            rw::HolderSeen::OtherProcess => Self::OtherProcess,
            _ => Self::Unknown,
        }
    }
}

impl From<rw::FilesOutcome> for api_state::FilesOutcome {
    fn from(f: rw::FilesOutcome) -> Self {
        match f {
            rw::FilesOutcome::Removed => Self::Removed,
            rw::FilesOutcome::LeftForHost => Self::LeftForHost,
            _ => Self::Unknown,
        }
    }
}

/// The production custody disclosure (FR-14 H1) for the wallet at `config.dbDir`
/// — the host renders an honest pre-wipe confirmation (a hardware-held key
/// deleted vs best-effort removal) and an ambient custody badge from it. Reads
/// ONLY the measured vault tier (an ACTIVE probe; NO seed, NO unseal, NO key
/// material). `VaultAbsent` (headless desktop, no platform keystore) ⇒ tier
/// `"none"`, `eraseAssurance: bestEffort`. Distinct from the diagnostic
/// `selftest_seed_custody` (which provisions selftest identities and is not
/// shippable host UX).
///
/// Like `wipe`, consumes ONLY `config.dbDir` (the tier is a function of the
/// platform + the `dbDir`-derived namespace) — so a pre-wipe honesty probe is
/// never gated on transport validation.
pub(crate) async fn custody_disclosure(
    config: api_config::WalletConfig,
) -> Result<api_wallet::CustodyDisclosure, api_error::WalletApiError> {
    // Same dbDir shape gate as `wallet_exists`/`wipe_wallet`: the disclosure
    // is derived from the dbDir-derived namespace, so a relative dir would
    // probe the wrong (cwd-derived) custody and mis-render the wipe honesty.
    let db_dir = db_dir_only(config);
    validate_db_dir(&db_dir)?;
    let db_dir: std::path::PathBuf = db_dir.into();
    Ok(custody_disclosure_dto(
        rw::Wallet::custody_disclosure(&db_dir).await?,
    ))
}

/// The pure core→DTO map for the custody disclosure (extracted so the
/// field-for-field honesty — a wrong `eraseAssurance` would mis-render the wipe
/// confirmation — is unit-testable without a device keychain probe). `None` tier
/// (no platform vault) is the only non-tier label; every real tier goes through
/// the shared `vault_tier_label` map (DRY with selftest).
fn custody_disclosure_dto(d: rw::CustodyDisclosure) -> api_wallet::CustodyDisclosure {
    api_wallet::CustodyDisclosure {
        tier: d.tier.map_or("none", vault_tier_label).to_string(),
        erase_assurance: erase_assurance_dto(d.erase_assurance),
        degraded: d.degraded,
    }
}

/// A value a newer core adds crosses as `Unknown`, which a host renders as
/// best-effort: under-claiming custody is the safe direction (ADR-0571).
fn erase_assurance_dto(a: rw::EraseAssurance) -> api_wallet::EraseAssurance {
    match a {
        rw::EraseAssurance::HardwareKeyDeleted => api_wallet::EraseAssurance::HardwareKeyDeleted,
        rw::EraseAssurance::BestEffort => api_wallet::EraseAssurance::BestEffort,
        _ => api_wallet::EraseAssurance::Unknown,
    }
}

/// Reveal the wallet's BIP39 recovery words for the third-party backup flow
/// (spec §3.3) — the ONE sanctioned *outbound* key-material crossing. The core
/// returns each word in a zeroizing buffer; THIS is the boundary where they
/// become plain `String`s for FRB marshalling. Two distinct lifetimes meet
/// here: the SOURCE `words: Vec<Zeroizing<String>>` wipes each word when it
/// drops at end of scope, but the RETURNED `Vec<String>` is intentionally NOT
/// zeroizing — it (and its FRB-marshalled Dart copy) is the inherent, documented
/// §10 exposure (FRB can't marshal `Zeroizing`; Dart memory can't be zeroized),
/// so it lives un-wiped until dropped. This is unavoidable for any reveal: the
/// backup screen has to render the words. Kept in `convert.rs`, NOT `api/`, so
/// the `Zeroizing` type never names the FRB-scanned surface (the
/// `ffi_surface_exposes_no_key_types` gate). `NoMnemonic` for a RawBytes /
/// None-persistence wallet (a host-custodied-seed integration can never reveal —
/// it holds no wallet-local phrase). No logging (§5.4 — the words are never
/// traced/printed on this path).
pub(crate) fn reveal_mnemonic(
    wallet: &rw::Wallet,
) -> Result<Vec<String>, api_error::WalletApiError> {
    let words = wallet
        .reveal_mnemonic()
        .map_err(api_error::WalletApiError::from)?;
    Ok(words.iter().map(|w| w.to_string()).collect())
}

/// The ONE bridge-layer error code (NOT in the core's `code()` table): the
/// `RW-FFI-*` sub-namespace is for conditions that exist only at the FRB
/// boundary. `RW-FFI-CLOSED` has no `rw::WalletError` counterpart — the core
/// models "closed" by move semantics, only the bridge needs an `Option`-based
/// closed check — so it cannot route through `From<rw::WalletError>`. A
/// support search for `RW-FFI` lands HERE; any future bridge-originated code
/// joins this const + comment.
pub(crate) const FFI_CLOSED_CODE: &str = "RW-FFI-CLOSED";

/// A call on a handle whose wallet was already moved out by `close()`. Carries
/// the bridge-owned [`FFI_CLOSED_CODE`] but maps onto the EXISTING
/// `InvalidState { Closing }` taxonomy (the same `WalletApiError` shape +
/// "wallet is closing" Dart UX path as a racing core `close()`), so it adds a
/// code but not a parallel `kind` taxonomy.
pub(crate) fn handle_closed() -> api_error::WalletApiError {
    api_error::WalletApiError {
        code: FFI_CLOSED_CODE.to_string(),
        message: "wallet handle is closed".to_string(),
        kind: api_error::WalletErrorKind::InvalidState {
            phase: api_error::LifecyclePhase::Closing,
        },
    }
}

/// Adapts an FRB `StreamSink` to the core's [`rw::SyncStatusSink`], converting each
/// core status to its Dart DTO at the boundary. The core owns the detached pump + the
/// watch receiver + the async runtime (so the bridge needs NO runtime of its own and
/// stays a thin codec); this is only the sink + conversion glue.
struct FrbStatusSink(StreamSink<api_state::SyncStatus>);

impl rw::SyncStatusSink for FrbStatusSink {
    fn emit(&mut self, status: rw::SyncStatus) -> bool {
        // `add` returns Err once Dart cancels/closes the stream (the
        // `AppLifecycleState.paused` / provider-dispose path) ⇒ report closed so the
        // core pump stops promptly. The DTO conversion HONORS the §3.3 finiteness
        // contract (the `From` impl filters non-finite percents).
        self.0.add(status.into()).is_ok()
    }
}

/// Stream the wallet's live `SyncStatus` into an FRB `StreamSink` (spec §3.3). The
/// CORE drives the pump on a detached task and returns at once, so the bridge method
/// holds the opaque handle's read lock only briefly — NOT for the stream's whole life
/// (which would block `close()`'s write lock: a close-hang on every mobile background
/// / dispose). The pump emits the current status first, coalesces latest-wins, and
/// ends — dropping the sink, which closes the Dart stream — when the host cancels
/// (`add` errors) or the wallet tears down; the full contract lives on
/// [`rw::Wallet::watch_sync_status`] / `spawn_status_pump`.
pub(crate) fn stream_sync_status(wallet: &rw::Wallet, sink: StreamSink<api_state::SyncStatus>) {
    wallet.watch_sync_status(FrbStatusSink(sink));
}

/// Adapts an FRB `StreamSink` to the core's [`rw::TorStateSink`] — the
/// [`FrbStatusSink`] parallel for the transport state (stage S1 `truth`). The
/// payload is §5.4-safe by construction: a state and, on the ready arms, the
/// host's own bounded transport name.
struct FrbTorStateSink(StreamSink<api_state::TorState>);

impl rw::TorStateSink for FrbTorStateSink {
    fn emit(&mut self, state: rw::TorState) -> bool {
        // `add` errors once Dart cancels/closes the stream (paused / provider
        // dispose) ⇒ report closed so the core pump stops promptly.
        self.0.add(state.into()).is_ok()
    }
}

/// Stream the wallet's live `TorState` into an FRB `StreamSink` — the
/// [`stream_sync_status`] shape exactly: the core spawns the detached pump and
/// returns at once, the current state first, then every transition coalesced
/// latest-wins, ending only when the host cancels or the wallet tears down.
/// The full contract lives on [`rw::Wallet::watch_tor_state`].
pub(crate) fn stream_tor_state(wallet: &rw::Wallet, sink: StreamSink<api_state::TorState>) {
    wallet.watch_tor_state(FrbTorStateSink(sink));
}

impl From<rw::IncomingFundsEventKind> for api_state::IncomingFundsEventKind {
    fn from(k: rw::IncomingFundsEventKind) -> Self {
        match k {
            rw::IncomingFundsEventKind::Replay => Self::Replay,
            rw::IncomingFundsEventKind::Live => Self::Live,
            rw::IncomingFundsEventKind::MemoRefresh => Self::MemoRefresh,
            _ => Self::Unknown,
        }
    }
}

impl From<rw::IncomingFundsEvent> for api_state::IncomingFundsEvent {
    fn from(e: rw::IncomingFundsEvent) -> Self {
        Self {
            kind: e.kind.into(),
            new_tx_count: e.new_tx_count,
            // u64 → i64 for a plain Dart int: a per-wallet-open detection
            // counter can't reach 2^63 (the `TxSummary.batch_id` precedent).
            total_tx_detected: e.total_tx_detected as i64,
            span_from_height: e.span_from_height,
            span_to_height: e.span_to_height,
            cursor: e.cursor,
        }
    }
}

/// Adapts an FRB `StreamSink` to the core's [`rw::IncomingFundsSink`] (the
/// [`FrbStatusSink`] parallel — one sink struct per live stream). The payload is
/// §5.4-safe BY CONSTRUCTION (counts + heights + an opaque watermark; ADR-0536),
/// so this boundary carries no NEVER-log value at all.
struct FrbIncomingFundsSink(StreamSink<api_state::IncomingFundsEvent>);

impl rw::IncomingFundsSink for FrbIncomingFundsSink {
    fn emit(&mut self, event: rw::IncomingFundsEvent) -> bool {
        // `add` errors once Dart cancels/closes the stream (paused / provider
        // dispose) ⇒ report closed so the core pump stops promptly.
        self.0.add(event.into()).is_ok()
    }
}

/// Stream the wallet's incoming-funds events into an FRB `StreamSink` (spec §3.3 /
/// ADR-0536). ASYNC + fallible, unlike [`stream_sync_status`]: the core validates
/// the host-supplied `since_cursor` (a malformed or foreign token rejects TYPED,
/// never a silent wrong replay window) and runs the per-subscriber replay read
/// before spawning the detached pump — which then follows the same no-lock-pin
/// contract (the bridge method's read lock releases at return; a live stream can
/// never block `close`).
pub(crate) async fn stream_incoming_funds(
    wallet: &rw::Wallet,
    since_cursor: Option<String>,
    sink: StreamSink<api_state::IncomingFundsEvent>,
) -> Result<(), api_error::WalletApiError> {
    wallet
        .watch_incoming_funds(since_cursor.as_deref(), FrbIncomingFundsSink(sink))
        .await
        .map_err(Into::into)
}

/// Adapts an FRB `StreamSink` to the core's [`rw::SwapStatusSink`] (the
/// [`FrbStatusSink`] sync-status parallel — one sink struct per live stream),
/// converting each core [`rw::SwapStatus`] to its Dart DTO at the boundary.
struct FrbSwapStatusSink(StreamSink<api_swap::SwapStatus>);

impl rw::SwapStatusSink for FrbSwapStatusSink {
    fn emit(&mut self, status: rw::SwapStatus) -> bool {
        // `add` errors once Dart cancels/closes the subscription (the §7
        // FOREGROUND-ONLY stop: `AppLifecycleState.paused` / provider dispose) ⇒
        // report closed so the core poll loop ends PROMPTLY (no more `provider.status`
        // traffic). The §5.4 NEVER-log values inside a status (out_txid, amounts) cross
        // ONLY as the Dart DTO the host renders — they are never logged on this path.
        self.0.add(status.into()).is_ok()
    }
}

/// Stream one swap's live [`rw::SwapStatus`] into an FRB `StreamSink` (spec §3.3/§7).
/// The CORE owns the detached §7 poll pump ([`rw::SwapService::watch_status`]) and
/// returns at once, so the bridge method holds the opaque handle's read lock only
/// briefly — NOT for the stream's whole life (which would block `close()`'s write lock:
/// a close-hang on every mobile background / dispose). The pump holds only a provider
/// `Arc` + a kill `watch::Receiver` (never the wallet `Inner`), polls the
/// `SWAP_POLL_INITIAL_SECS`→`SWAP_POLL_MAX_SECS` cadence, and ends — dropping the sink,
/// closing the Dart stream — on a terminal status, a host cancel (`add` errors), a
/// `Hard` kill (§3.5), or wallet teardown. The bridge keeps no async runtime of its own.
///
/// The `swap_id` (the SDK-minted handle, ADR-0555; §5.4 render-never-log) is SIZE-CAPPED
/// here at [`PROVIDER_STR_MAX_BYTES`](rw::constants::PROVIDER_STR_MAX_BYTES) BEFORE any
/// poll spawns: the poll loop rides every `provider.status` error as a transient fault and
/// retries FOREVER, so a garbage/oversized id (which the adapter rejects every poll) would
/// otherwise spin a never-terminating stream. Empty/over-bound ⇒ a typed `RequestInvalid`
/// returned to the caller, NOT a spinning stream (validate-at-the-boundary, principle 7 —
/// the host passed the id back, so an over-bound one is a host bug). The id is NEVER logged.
pub(crate) async fn stream_swap_status(
    service: &rw::SwapService,
    swap_id: String,
    sink: StreamSink<api_swap::SwapStatus>,
) -> Result<(), api_error::SwapApiError> {
    // The cap is factored into a free predicate so the security-load-bearing reject path
    // is UNIT-TESTABLE without a `StreamSink` (which needs a Dart port); the accept path
    // (the spawned pump) still rides the on-device e2e.
    check_swap_id_bound(&swap_id)?;
    // S8: the core resolves the provider handle from the durable home row BEFORE it
    // spawns (an id with no row is a typed `RequestInvalid`, never a spinning stream).
    service
        .watch_status(rw::SwapId::new(swap_id), FrbSwapStatusSink(sink))
        .await
        .map_err(api_error::SwapApiError::from)
}

/// The `watch_swap_status` boundary guard: a swap id (the SDK-minted handle, 32 hex) must
/// be non-empty and within [`PROVIDER_STR_MAX_BYTES`](rw::constants::PROVIDER_STR_MAX_BYTES)
/// — else the §7 poll loop, which rides every `provider.status` error as a transient fault
/// FOREVER, would spin a never-terminating stream on a garbage id. Empty/over-bound ⇒ typed
/// `RequestInvalid` (a host bug — the host passed back an id it never received). A free fn
/// (not inlined) so the reject path is testable at the named-constant boundary.
fn check_swap_id_bound(swap_id: &str) -> Result<(), api_error::SwapApiError> {
    if swap_id.is_empty() || swap_id.len() > rw::constants::PROVIDER_STR_MAX_BYTES {
        return Err(rw::SwapError::RequestInvalid {
            reason: "swap id is empty or exceeds the provider string bound",
        }
        .into());
    }
    Ok(())
}

/// The swap-vocabulary "this wallet handle is closed" error — the [`handle_closed`]
/// parallel for the swap surface. A closed/torn-down handle has no swap state, so the
/// honest swap-namespaced outcome is `SwapStateUnavailable` ("swap unavailable, retry");
/// built THROUGH the core `From` so it carries the real stable `RW-SWAP-*` code + the
/// payload-free message, never a hand-typed pair.
pub(crate) fn swap_handle_closed() -> api_error::SwapApiError {
    rw::SwapError::SwapStateUnavailable.into()
}

/// The "no swap provider attached" error (kill layer 2; §3.5): `Wallet::swap() == None`
/// ⟺ Dart `swap == null` ⟺ swap was never enabled / was killed at construction. The host
/// renders "swap unavailable" and hides the surface — it is not an error to retry.
pub(crate) fn swap_disabled() -> api_error::SwapApiError {
    rw::SwapError::SwapDisabled.into()
}

impl TryFrom<api_swap::QuoteRequest> for rw::QuoteRequest {
    type Error = rw::SwapError;

    fn try_from(r: api_swap::QuoteRequest) -> Result<Self, Self::Error> {
        let api_swap::QuoteRequest {
            direction,
            exact,
            slippage_tolerance_bps,
            destination,
            refund_address,
        } = r;
        Ok(Self {
            direction: try_direction(direction)?,
            exact: try_exact(exact)?,
            slippage_tolerance_bps,
            destination,
            refund_address,
        })
    }
}

/// Inbound `Unknown` is a HOST BUG (Dart can only construct variants this
/// binding generated) — rejected typed, mirroring the core's request
/// validation posture.
fn try_direction(d: api_swap::SwapDirection) -> Result<rw::SwapDirection, rw::SwapError> {
    match d {
        api_swap::SwapDirection::IntoZec { from } => Ok(rw::SwapDirection::IntoZec {
            from: asset_in(from),
        }),
        api_swap::SwapDirection::OutOfZec { to } => {
            Ok(rw::SwapDirection::OutOfZec { to: asset_in(to) })
        }
        api_swap::SwapDirection::Unknown => Err(rw::SwapError::RequestInvalid {
            reason: "unknown swap direction",
        }),
    }
}

fn try_exact(e: api_swap::ExactSide) -> Result<rw::ExactSide, rw::SwapError> {
    match e {
        api_swap::ExactSide::In { amount } => Ok(rw::ExactSide::In(try_amount(amount)?)),
        api_swap::ExactSide::Out { amount } => Ok(rw::ExactSide::Out(try_amount(amount)?)),
    }
}

fn try_amount(a: api_swap::SwapAmount) -> Result<rw::SwapAmount, rw::SwapError> {
    match a {
        api_swap::SwapAmount::Zec { zat } => Ok(rw::SwapAmount::Zec(
            // bounds-checked at the door; out-of-range is a request error in
            // swap vocabulary (the core's validate_request posture)
            rw::Zatoshis::new(zat).map_err(|_| rw::SwapError::RequestInvalid {
                reason: "zec amount out of range",
            })?,
        )),
        api_swap::SwapAmount::Foreign { amount } => Ok(rw::SwapAmount::Foreign(amount)),
    }
}

fn asset_in(a: api_swap::AssetId) -> rw::AssetId {
    let api_swap::AssetId { chain, symbol } = a;
    rw::AssetId { chain, symbol }
}

/// Reconstruct the core [`rw::SwapQuote`] from the Dart DTO the host passes back to
/// `swapExecute` (the spec-literal `execute(SwapQuote)`). LEAST-AUTHORITY-equivalent to
/// the `send(proposalId)` token despite carrying the whole DTO: core `execute` claims the
/// durable single-flight `issued_store.take(&quote.id)` FIRST (W-swap-3-c-3-ii), so a
/// forged `id` has NO durable claim ⇒ typed `RequestInvalid` BEFORE the provider is ever
/// called, and the §4.4 deposit send uses the durable RE-BLESSED row, never this DTO ⇒ a
/// forged `deposit_address`/amount cannot redirect funds. Every field below other than
/// `id` is thus NON-AUTHORITATIVE on the execute path; they are converted FAITHFULLY (not
/// fabricated) so the reconstructed value stays honest if a future caller reads them.
impl TryFrom<api_swap::SwapQuote> for rw::SwapQuote {
    type Error = rw::SwapError;

    fn try_from(q: api_swap::SwapQuote) -> Result<Self, Self::Error> {
        // exhaustive destructuring: a new DTO field fails compile here (forces this
        // reconstruction to stay in lockstep with the outbound `From<rw::SwapQuote>`).
        let api_swap::SwapQuote {
            id,
            deposit_address,
            deposit_memo,
            expires_at,
            amount_in,
            min_amount_out,
            zec_side_zat,
            refund_to,
            disclosure,
            binding,
        } = q;
        Ok(Self {
            id: rw::SwapId::new(id),
            deposit_address,
            deposit_memo,
            // i64→u64 unix seconds: a negative (pre-epoch) display value clamps to 0 (a
            // dead-past deadline). Non-authoritative anyway — the durable WALL deadline
            // gates execute — but kept honest rather than wrapped huge.
            expires_at: expires_at.max(0) as u64,
            amount_in,
            min_amount_out,
            // out-of-range is a request error in swap vocabulary (the door posture in
            // `try_amount`); the M1 bound is re-checked core-side regardless.
            zec_side: rw::Zatoshis::new(zec_side_zat).map_err(|_| {
                rw::SwapError::RequestInvalid {
                    reason: "zec amount out of range",
                }
            })?,
            refund_to,
            disclosure: disclosure_in(disclosure),
            // FR-17 inbound. The durable issued row is the authority the deposit
            // signs from — execute never signs with this echo — but since stage S8
            // (R01) execute DOES read it, once: the terms compare refuses a DTO whose
            // binding differs from the record's (`QuoteTermsDiffer`, a stripped or
            // foreign binding is a DTO the approval cannot have been about; asserted
            // by `a_caller_dto_whose_terms_differ_from_its_record_executes_nothing`).
            // So a wrong-length echo folds to `None` here (validate-never-truncate)
            // and is refused TYPED by that compare, never by a panic at this door.
            // (The diff review: the previous sentence here said execute never
            // reads the echo — true before S8, false after.)
            binding: binding
                .as_deref()
                .and_then(zec_wallet_core::SpendBinding::from_slice),
        })
    }
}

/// Inbound disclosure (Dart → core), the mirror of `From<rw::SwapPrivacyDisclosure>`.
/// Pure DISPLAY data — never read on the execute path; converted faithfully so a future
/// reader sees what the host echoed back, not a fabricated value.
fn disclosure_in(d: api_swap::SwapPrivacyDisclosure) -> rw::SwapPrivacyDisclosure {
    let api_swap::SwapPrivacyDisclosure {
        ends_shielded,
        deshields,
        provider_legs_transparent,
        provider_sees,
    } = d;
    rw::SwapPrivacyDisclosure {
        ends_shielded,
        deshields,
        provider_legs_transparent,
        // the core enum is CLOSED (5 named items, no `Unknown`); a host-echoed `Unknown`
        // has no core counterpart and is display-only, never read by execute — DROP it
        // rather than invent a meaning (the disclosure stays as faithful as the core models).
        provider_sees: provider_sees
            .into_iter()
            .filter_map(disclosure_item_in)
            .collect(),
    }
}

fn disclosure_item_in(i: api_swap::DisclosureItem) -> Option<rw::DisclosureItem> {
    Some(match i {
        api_swap::DisclosureItem::CrossAssetLink => rw::DisclosureItem::CrossAssetLink,
        api_swap::DisclosureItem::Amounts => rw::DisclosureItem::Amounts,
        api_swap::DisclosureItem::DestinationAddress => rw::DisclosureItem::DestinationAddress,
        api_swap::DisclosureItem::SourceAddress => rw::DisclosureItem::SourceAddress,
        api_swap::DisclosureItem::IpUnlessTor => rw::DisclosureItem::IpUnlessTor,
        api_swap::DisclosureItem::Unknown => return None,
    })
}

/// Lower the Dart swap-provider config + manifest-kill state and construct/attach the
/// NEAR Intents provider (the `enableNearSwap` bridge method; §3.5). Thin glue over
/// [`crate::swap_provider::enable_near_swap`] — the kill-door composition (transport
/// resolution off the wallet's OWN dialer, set-once attach, monotonic-off `set_kill`)
/// lives there. The `jwt` secret is wrapped in its `Zeroizing` buffer by the adapter's
/// own `SwapProviderConfig::from_parts` constructor (the crate that owns the secret
/// type), so the bridge never names `zeroize`; it is never logged. The core `SwapError`
/// maps to `SwapApiError` via the existing `From`.
#[cfg(feature = "swap-near")]
pub(crate) fn enable_near_swap(
    wallet: &rw::Wallet,
    config: api_swap::SwapProviderConfig,
    swap_enabled: bool,
    declared_kill: Option<api_swap::SwapKill>,
) -> Result<(), api_error::SwapApiError> {
    let api_swap::SwapProviderConfig { endpoint, jwt } = config;
    // `?` because the JWT is now validated at CONSTRUCTION:
    // a malformed secret is refused here, at `enableNearSwap`, where the host can
    // act on it — not once per request on the network path, where the reason is
    // unreadable because the value is redacted everywhere.
    let config = zec_wallet_swap_near::SwapProviderConfig::from_parts(endpoint, jwt)?;
    crate::swap_provider::enable_near_swap(
        wallet,
        config,
        swap_enabled,
        declared_kill.map(swap_kill_in),
    )
    .map_err(Into::into)
}

/// Inbound [`api_swap::SwapKill`] → core [`rw::SwapKill`]. The §3.5 monotonic-off
/// doctrine: an UNRECOGNIZED declared kill (a host bug — Dart can only build variants
/// this binding generated) is treated as the STRONGEST kill understood (`Hard`), so an
/// unknown directive can only ever turn swap MORE off, never resurrect it.
#[cfg(feature = "swap-near")]
fn swap_kill_in(k: api_swap::SwapKill) -> rw::SwapKill {
    match k {
        api_swap::SwapKill::Live => rw::SwapKill::Live,
        api_swap::SwapKill::WindDown => rw::SwapKill::WindDown,
        api_swap::SwapKill::Hard => rw::SwapKill::Hard,
        api_swap::SwapKill::Unknown => rw::SwapKill::Hard,
    }
}

// ─── Payments: the ZIP-321 codec surface (api/payments.rs) ──────────────────
//
// LOSSLESS-TOKEN CONTRACT (api/payments.rs module note): the parse direction
// is a DISPLAY projection — an arbitrary (0xFF) memo crosses as length only,
// raw bytes never cross; the URI string itself stays the canonical form. The
// compose direction ALSO takes opaque bytes (FR-28), and the asymmetry is
// deliberate: outbound bytes are the caller's own, inbound ones are hostile
// on-chain data (principle 7).
//
// The earlier form of this note said the compose direction was text-only "by
// design (machine memos are composed host-side in Rust, A6)". ADR-0530 A6 is
// nonce hygiene, PARSER obligations and a fuzz target — verified verbatim in
// the first host's pay-to-contact privacy record — so it never
// carried that rule. Same defect the FR-27 ruling found on `memo_to_parsed`'s
// §5.4 citation, at a second site: a citation nobody re-read became a rule.

pub(crate) fn parse_payment_uri(
    uri: &str,
    network: api_config::Network,
) -> Result<Vec<api_payments::ParsedPayment>, api_error::WalletApiError> {
    let req = rw::parse_payment_uri(uri, network.into())?;
    Ok(req.payments.into_iter().map(parsed_payment_out).collect())
}

pub(crate) fn encode_payment_uri(
    payments: Vec<api_payments::PaymentDraft>,
    network: api_config::Network,
) -> Result<String, api_error::WalletApiError> {
    let network: rw::Network = network.into();
    let legs = payments
        .into_iter()
        .map(|d| {
            // exhaustive destructuring: a new draft field fails compile here
            let api_payments::PaymentDraft {
                recipient_address,
                amount_zat,
                memo_text,
                memo_bytes,
                label,
                message,
            } = d;
            // every draft through the core's validation doors — nothing
            // that skipped `Address::parse`/`Payment::new` can be encoded
            let recipient = rw::Address::parse(&recipient_address, network)?;
            let amount = amount_zat.map(rw::Zatoshis::new).transpose()?;
            // ONE memo per leg. Both set is a caller bug, and the honest
            // answer is a typed refusal — silently preferring either one
            // would put a memo on a permanent public ledger that the caller
            // did not choose.
            let memo = match (memo_text, memo_bytes) {
                (Some(_), Some(_)) => {
                    return Err(api_error::WalletApiError::from(
                        rw::WalletError::MemoConflict,
                    ));
                }
                (None, None) => rw::Memo::Empty,
                (Some(s), None) => rw::Memo::text(s)?,
                // EMPTY bytes are nothing to attach, and this is the ONE place
                // that rule can be enforced for every caller. `Memo::arbitrary`
                // accepts a zero-length vec (0 <= 511), which would put a full
                // 511-byte ALL-ZERO 0xFF memo on-chain — a fee-bearing no-op and
                // a distinguishable wallet fingerprint — and would also make
                // `Payment::new` refuse a TRANSPARENT recipient for a memo that
                // does not exist. The Dart layer drops empty to null too, but a
                // host calling `encodePaymentUri` with its own `PaymentDraft` —
                // the crossing this API exists for — reaches only this one
                // (crypto audit).
                (None, Some(bytes)) if bytes.is_empty() => rw::Memo::Empty,
                // FR-28: opaque, length-bounded, never inspected. The wire
                // zero-pads below 511 and returns the full field — the API
                // doc states it; the host frames its own length.
                (None, Some(bytes)) => rw::Memo::arbitrary(bytes)?,
            };
            Ok(rw::Payment::new(recipient, amount, memo, label, message)?)
        })
        .collect::<Result<Vec<_>, api_error::WalletApiError>>()?;
    Ok(rw::encode_payment_uri(&rw::PaymentRequest {
        payments: legs,
    })?)
}

/// FR-27 — the `machineMemos` verb's engine (api/ files hold no core types):
/// the host's txid string through the ONE hex door, then the core's
/// prefix-scoped read.
///
/// The hex parse lives HERE rather than in the verb because it is the hostile
/// half: a `txid_hex` arrives from Dart and, through it, from anywhere. It is
/// length-checked before any allocation and refused typed
/// ([`rw::WalletError::TxidInvalid`]) with the offending string never echoed
/// back (§5.4 — txids are never-log material).
///
/// Nothing here inspects the returned bytes. They are opaque on the way out,
/// exactly as they were opaque on the way in.
pub(crate) async fn machine_memos(
    wallet: &rw::Wallet,
    txid_hex: &str,
) -> Result<Vec<Vec<u8>>, api_error::WalletApiError> {
    let txid = rw::TxId::from_display_hex(txid_hex)?;
    Ok(wallet.machine_memos(txid).await?)
}

pub(crate) fn validate_address(
    address: &str,
    network: api_config::Network,
) -> Result<api_payments::ValidatedAddress, api_error::WalletApiError> {
    // The audited classification gate — the SAME `Address::parse` the send path
    // lowers through (DRY: never a second, hand-rolled address check that could
    // drift from what `propose` accepts). The size-cap + network-check live
    // inside it; a malformed / other-network address returns the typed error the
    // bridge surfaces unchanged. The address is NEVER logged (§5.4).
    let addr = rw::Address::parse(address, network.into())?;
    Ok(api_payments::ValidatedAddress {
        // memo-capable ⟺ a shielded receiver is present ⟺ private (the
        // shielded/transparent axis the host renders); the audited predicate,
        // never re-derived from the address bytes by hand.
        memo_capable: addr.memo_capable(),
    })
}

/// THE single `Memo` → `ParsedMemo` display projection — the SSOT for the bridge memo crossing
/// (the payment-URI parse path AND the incoming-memo / `transactionMemos` read both lower
/// through here). MACHINE memos (`Arbitrary`/`Reserved`) cross as PRESENCE + LENGTH only: the raw
/// bytes (a host envelope, or hostile/opaque on-chain data) never cross ON THIS PATH — only `Text`
/// memos surface their content (§2.4). A future core arm renders neutrally, never a panic.
///
/// WHY, corrected (FR-27 ruling): this comment used to cite **§5.4** for the never-cross rule.
/// §5.4 (`wallet-sdk.md`) is the **NEVER-LOG** list — addresses, amounts, memo bytes, txids — and
/// says nothing about crossing the FFI. Nothing in the spec ever forbade the crossing; what
/// justifies the projection is the comment's OTHER half, which was the real reason all along:
/// these are hostile, opaque bytes off a public chain (principle 7), and a DISPLAY read should not
/// hand them to a host that only asked what to render.
///
/// THE ONE EXCEPTION, named here so the rule and its exception live together (the ruling's own
/// condition — otherwise the next reader re-derives a rule the code no longer follows):
/// [`crate::api::wallet::WalletHandle::machine_memos`] returns `Arbitrary` bytes VERBATIM, for a
/// host that registered a prefix scope on its `WalletConfig`. That is a DIFFERENT verb with a
/// different contract (opt-in, prefix-scoped, `Arbitrary` only, never `Reserved`, and its API doc
/// tells the caller the bytes are attacker-controllable). It does NOT widen this projection:
/// `ParsedMemo` stays length-only, and `inbound_parsed_memo_never_carries_raw_bytes` fails the
/// build if it ever grows a byte field.
pub(crate) fn memo_to_parsed(memo: rw::Memo) -> api_payments::ParsedMemo {
    match memo {
        rw::Memo::Empty => api_payments::ParsedMemo::Empty,
        rw::Memo::Text(text) => api_payments::ParsedMemo::Text { text },
        // ≤ 511 by construction — the cast cannot truncate
        rw::Memo::Arbitrary(bytes) => api_payments::ParsedMemo::Arbitrary {
            len: bytes.len() as u32,
        },
        // reserved framing is receivable on-chain reality (the incoming-memo lane); a send intent
        // rejects it, so it is unreachable from the parse funnel.
        rw::Memo::Reserved(bytes) => api_payments::ParsedMemo::Reserved {
            len: bytes.len() as u32,
        },
        _ => api_payments::ParsedMemo::Unknown,
    }
}

fn parsed_payment_out(p: rw::Payment) -> api_payments::ParsedPayment {
    // capability/classification read BEFORE the move-outs below
    let recipient_kind = match p.recipient.kind() {
        rw::AddressKind::Unified => api_payments::AddressKind::Unified,
        rw::AddressKind::Sapling => api_payments::AddressKind::Sapling,
        rw::AddressKind::Transparent => api_payments::AddressKind::Transparent,
        _ => api_payments::AddressKind::Unknown,
    };
    let recipient_memo_capable = p.recipient.memo_capable();
    let rw::Payment {
        recipient,
        amount,
        memo,
        label,
        message,
    } = p;
    let memo = memo_to_parsed(memo);
    api_payments::ParsedPayment {
        recipient_address: recipient.encoded().to_owned(),
        recipient_kind,
        recipient_memo_capable,
        // amounts ≤ max supply < 2^53 — exact in Dart (spec §2.1)
        amount_zat: amount.map(rw::Zatoshis::zat),
        memo,
        label,
        message,
    }
}

// ── SendProposal: the prepared-send DISPLAY DTO (api/payments.rs) ────────────
//
// The `propose` output (inc-2d-ffi). Integer zatoshis only; the upstream
// `Proposal` never crosses — only its retained one-shot token id. `OutputPool`
// is the engine's own per-output de-shield verdict (§5.1), carried so the host
// can disclose a transparent leg before the user signs.

impl From<rw::OutputPool> for api_payments::OutputPool {
    fn from(p: rw::OutputPool) -> Self {
        match p {
            rw::OutputPool::Transparent => Self::Transparent,
            rw::OutputPool::Sapling => Self::Sapling,
            rw::OutputPool::Orchard => Self::Orchard,
            rw::OutputPool::Ironwood => Self::Ironwood,
            // a future core pool renders neutrally — never a panic
            _ => Self::Unknown,
        }
    }
}

impl From<rw::ProposalRecipient> for api_payments::ProposalRecipient {
    fn from(r: rw::ProposalRecipient) -> Self {
        // exhaustive destructure (the house drift guard): a new core field fails
        // compile here rather than silently not reaching Dart.
        let rw::ProposalRecipient { pool, amount_zat } = r;
        Self {
            pool: pool.into(),
            amount_zat,
        }
    }
}

impl From<rw::ProposalStep> for api_payments::ProposalStep {
    fn from(s: rw::ProposalStep) -> Self {
        let rw::ProposalStep { recipients } = s;
        Self {
            recipients: recipients.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<rw::LargeSendReason> for api_payments::LargeSendReason {
    fn from(r: rw::LargeSendReason) -> Self {
        match r {
            rw::LargeSendReason::NearTotalBalance => Self::NearTotalBalance,
            rw::LargeSendReason::OverAbsoluteThreshold => Self::OverAbsoluteThreshold,
            rw::LargeSendReason::Both => Self::Both,
            // a future core reason still triggers the confirm — never a silent skip
            _ => Self::Unknown,
        }
    }
}

impl From<rw::SendProposal> for api_payments::SendProposal {
    fn from(p: rw::SendProposal) -> Self {
        // exhaustive destructure: a new core SendProposal field (e.g. a second
        // disclosure signal) fails compile here instead of silently not crossing.
        let rw::SendProposal {
            proposal_id,
            binding,
            total_zat,
            fee_zat,
            change_zat,
            steps,
            target_height,
            has_transparent_recipient,
            is_shield,
            is_two_step_tex,
            large_send,
            self_send,
            single_recipient_zat,
        } = p;
        Self {
            // The registry id is a process-local monotonic counter that cannot
            // reach 2^63 (it starts low and increments per propose), so the
            // u64→i64 cast is lossless and keeps the Dart side a plain `int`. Even
            // the impossible overflow is money-safe: a wrapped id round-tripped to
            // `send` is just a registry miss (typed `ProposalAlreadyUsed`), never a
            // wrong-token sign.
            proposal_id: proposal_id as i64,
            // FR-17: the binding nonce crosses as opaque bytes (not key material —
            // its only power is to make a review↔sign mismatch detectable).
            binding: binding.as_bytes().to_vec(),
            total_zat,
            fee_zat,
            change_zat,
            steps: steps.into_iter().map(Into::into).collect(),
            target_height,
            has_transparent_recipient,
            is_shield,
            is_two_step_tex,
            large_send: large_send.map(Into::into),
            self_send,
            single_recipient_zat,
        }
    }
}

#[cfg(test)]
mod tests {
    //! The MONEY-DISPLAY conversion the wallet handle's `snapshot()` returns
    //! (`From<rw::WalletState>`) — the path that, if a field were swapped or a
    //! cast were wrong, would show the user a WRONG BALANCE. Pinned field-for-
    //! field with DISTINCT values so a transposition fails loudly. (The enum
    //! arms are covered by the lockstep gate + their own `From` impls; this is
    //! the numeric money path.)
    use super::*;

    fn zat(v: i64) -> rw::Zatoshis {
        rw::Zatoshis::new(v).expect("test value in range")
    }

    /// FR-37's `u64` → Dart `int` crossing: a count past `i64::MAX` SATURATES.
    /// Unreachable by dialling — a per-open counter cannot pass 2^63 — so this
    /// is the robustness rule, not a live defect: an `as` cast would hand the
    /// host's privacy panel a NEGATIVE number of connections, a rendering that
    /// impeaches every other number beside it. "Impossibly large" is a reading
    /// a user can dismiss; "-1 private connections" is not.
    ///
    /// Watched against: `i64::try_from(n).unwrap_or(i64::MAX)` reverted to
    /// `n as i64`.
    #[test]
    // Field-by-field assignment, not a literal: the core structs are
    // `#[non_exhaustive]`, so this crate cannot name all their fields at once
    // (which is also why the bridge's mirror is guarded by
    // `bridge_structs_mirror_core_fields` and not by a compile error here) —
    // the lint's suggested struct literal does not compile across the crate
    // boundary.
    #[allow(clippy::field_reassign_with_default)]
    fn an_out_of_range_dial_count_saturates_instead_of_going_negative() {
        let mut private = rw::ArmCounts::default();
        private.connected = u64::MAX;
        private.timeout = i64::MAX as u64 + 1; // the first value that wraps
        private.io = 7; // the in-range control: saturation is not a floor
        let mut counts = rw::DialCounts::default();
        counts.private = private;

        let crossed: api_state::DialCounts = counts.into();
        assert_eq!(
            (
                crossed.private.connected,
                crossed.private.timeout,
                crossed.private.io
            ),
            (i64::MAX, i64::MAX, 7),
            "a count past the Dart int range crosses as the largest int, never \
             as a negative connection count"
        );
    }

    #[test]
    fn a_sync_server_naming_a_header_without_a_value_is_refused_at_the_door() {
        // P3-13 D6, the `endpoint_auth` both-or-neither rule applied to the
        // offered list: a gated entry without its value would be offered
        // UNAUTHENTICATED and fail on the network as an opaque refusal — so
        // the door refuses it, and a value without a header likewise.
        let entry = |header: Option<&str>, value: Option<&str>| api_config::SyncServer {
            id: "gated".to_owned(),
            label: "Gated".to_owned(),
            url: "https://gated.example:443".to_owned(),
            auth_header: header.map(str::to_owned),
            auth_value: value.map(str::to_owned),
        };
        assert!(matches!(
            rw::SyncServer::try_from(entry(Some("x-zcash-rpc-key"), None)),
            Err(rw::WalletError::InvalidEndpointAuth { reason }) if reason.contains("BOTH")
        ));
        assert!(matches!(
            rw::SyncServer::try_from(entry(None, Some("k"))),
            Err(rw::WalletError::InvalidEndpointAuth { reason }) if reason.contains("BOTH")
        ));
        let both = rw::SyncServer::try_from(entry(Some("x-zcash-rpc-key"), Some("k")))
            .expect("both halves pass");
        assert!(both.is_gated());
        let neither = rw::SyncServer::try_from(entry(None, None)).expect("a public entry");
        assert!(!neither.is_gated());
        // The config door runs the list predicate too (the pre-flight must
        // agree with create/open): two entries sharing an id are refused.
        let cfg = |servers: Option<Vec<api_config::SyncServer>>| api_config::WalletConfig {
            db_dir: "/tmp/zec-wallet-door-test".to_owned(),
            network: api_config::Network::Test,
            endpoint_url: "https://zec.rocks:443".to_owned(),
            endpoint_auth_header: None,
            endpoint_auth_value: None,
            tor: api_config::TorPolicy::Off,
            seed_persistence: api_config::SeedPersistence::None,
            birthday_height: None,
            broadcast_jitter: api_config::JitterPolicy::None,
            machine_memo_prefixes: Vec::new(),
            sync_servers: servers,
        };
        assert!(matches!(
            rw::WalletConfig::try_from(cfg(Some(vec![entry(None, None), entry(None, None)]))),
            Err(rw::WalletError::InvalidEndpoint { reason }) if reason.contains("share an id")
        ));
        assert!(rw::WalletConfig::try_from(cfg(None)).is_ok());
        assert!(rw::WalletConfig::try_from(cfg(Some(vec![entry(None, None)]))).is_ok());
    }

    #[test]
    fn sync_server_bridge_dtos_never_carry_the_key() {
        // Gate 1: the HANDLE's view of an offered entry (`From<&rw::SyncServer>`)
        // carries the header name and never the value. (The catalog half of
        // this row went with the gated catalog entry, ADR-0568: every
        // reference entry is public — pinned below.)
        let core = rw::SyncServer::new(
            rw::SyncServerId::new("gated").expect("id"),
            "Gated",
            rw::LightServerEndpoint::new("https://gated.example:443").expect("endpoint"),
            Some(rw::EndpointAuth::new("x-zcash-rpc-key", "the-key-value").expect("auth")),
        )
        .expect("server");
        let dto = api_config::SyncServer::from(&core);
        assert_eq!(dto.auth_header.as_deref(), Some("x-zcash-rpc-key"));
        assert_eq!(dto.auth_value, None, "the handle never returns the key");
        // `WalletApiError` is deliberately not `Debug` (its message is for
        // logs, never for a panic string); grade by the code.
        let catalog = reference_sync_servers(api_config::Network::Main)
            .map_err(|e| e.code)
            .expect("catalog");
        assert!(
            catalog
                .iter()
                .all(|s| s.auth_header.is_none() && s.auth_value.is_none()),
            "the reference catalog carries no key"
        );
    }

    #[test]
    fn the_status_never_returns_a_custom_key_value() {
        // ADR-0568: a returned keyed custom choice names its header (so the
        // picker can say a key is saved) and NEVER the value — the empty
        // string, always.
        let core = rw::SyncServerChoice::Custom {
            endpoint: rw::LightServerEndpoint::new("https://mine.example:443").expect("endpoint"),
            key: Some(rw::EndpointAuth::new("x-api-key", "users-own-key-7c1e").expect("auth")),
        };
        match api_config::SyncServerChoice::from(core) {
            api_config::SyncServerChoice::Custom {
                url,
                key: Some(key),
            } => {
                assert_eq!(url, "https://mine.example:443");
                assert_eq!(key.header, "x-api-key");
                assert_eq!(key.value, "", "the value never leaves Rust");
            }
            _ => panic!("a keyed custom choice must cross as a keyed custom choice"),
        }
        let keyless = rw::SyncServerChoice::Custom {
            endpoint: rw::LightServerEndpoint::new("https://mine.example:443").expect("endpoint"),
            key: None,
        };
        assert!(matches!(
            api_config::SyncServerChoice::from(keyless),
            api_config::SyncServerChoice::Custom { key: None, .. }
        ));
    }

    #[test]
    fn custom_choice_key_is_bounded_and_typed() {
        // ADR-0568: a user's key crosses the bridge through the ONE door
        // (`EndpointAuth::from_zeroizing`): every refusal is the auth KIND,
        // never the URL's, so the sheet tells a bad key from a bad address.
        let choice = |header: &str, value: &str, url: &str| api_config::SyncServerChoice::Custom {
            url: url.to_owned(),
            key: Some(api_config::SyncServerKey {
                header: header.to_owned(),
                value: value.to_owned(),
            }),
        };
        let ok =
            rw::SyncServerChoice::try_from(choice("x-api-key", "k-1", "https://m.example:443"))
                .expect("a well-formed key");
        assert!(matches!(
            ok,
            rw::SyncServerChoice::Custom { key: Some(_), .. }
        ));
        let over_header = "x".repeat(rw::constants::SYNC_SERVER_AUTH_HEADER_MAX_BYTES + 1);
        let over_value = "k".repeat(rw::constants::ENDPOINT_AUTH_VALUE_MAX_BYTES + 1);
        for (what, header, value) in [
            ("an empty header", "", "k"),
            ("an empty value", "x-api-key", ""),
            ("an oversized header", over_header.as_str(), "k"),
            ("an oversized value", "x-api-key", over_value.as_str()),
            ("a grpc- header", "grpc-key", "k"),
            ("an injected value", "x-api-key", "k\r\nx-injected: 1"),
        ] {
            assert!(
                matches!(
                    rw::SyncServerChoice::try_from(choice(header, value, "https://m.example:443")),
                    Err(rw::WalletError::InvalidEndpointAuth { .. })
                ),
                "{what} must be the auth kind"
            );
        }
        // A bad URL with a good key is still the URL's kind.
        assert!(matches!(
            rw::SyncServerChoice::try_from(choice("x-api-key", "k", "ftp://m.example")),
            Err(rw::WalletError::InvalidEndpoint { .. })
        ));
        // And the kind crosses typed.
        let crossed: api_error::WalletApiError = rw::WalletError::InvalidEndpointAuth {
            reason: "auth header value is empty",
        }
        .into();
        assert!(matches!(
            crossed.kind,
            api_error::WalletErrorKind::InvalidEndpointAuth { .. }
        ));
        assert_eq!(crossed.code, "RW-CFG-004");
    }

    #[test]
    fn staged_seed_source_fork_maps_the_flag_faithfully_both_ways() {
        // FR-24 (arch MED-3): the ONE place the FFI `freshlyGenerated`
        // flag becomes money semantics. A flipped condition inverts the fresh
        // contract in BOTH directions with no downstream red flag — pin both
        // arms, plus the shared bounds door on each.
        let fresh = staged_seed_source(vec![0x11; 32], true).expect("valid");
        assert!(fresh.is_freshly_generated(), "true must mark fresh");
        let plain = staged_seed_source(vec![0x11; 32], false).expect("valid");
        assert!(
            !plain.is_freshly_generated(),
            "false must stay restore-shaped"
        );
        for flag in [true, false] {
            assert!(
                staged_seed_source(vec![0x11; 8], flag).is_err(),
                "both arms ride the 32..=252 bounds door"
            );
        }
    }

    #[test]
    fn vault_tier_label_maps_every_named_tier() {
        // FR-14 H1: the ONE tier→label map shared by `custody_disclosure` and
        // `selftest_seed_custody` (DRY). Pin every named arm so a re-label /
        // dropped arm fails here, not silently on-device. (The `_ => "unknown"`
        // forward arm is unconstructable from a non-exhaustive enum in test, but
        // is the documented fallback for a newer core; the booleans, not the
        // label, are the load-bearing predicates.)
        assert_eq!(vault_tier_label(rw::VaultTier::StrongBox), "strongbox");
        assert_eq!(vault_tier_label(rw::VaultTier::Tee), "tee");
        assert_eq!(
            vault_tier_label(rw::VaultTier::SoftwareKeystore),
            "software_keystore"
        );
        assert_eq!(
            vault_tier_label(rw::VaultTier::AppleKeychain),
            "apple_keychain"
        );
        assert_eq!(
            vault_tier_label(rw::VaultTier::AppleSecureEnclave),
            "apple_secure_enclave"
        );
    }

    /// #397 §3.7 D3 (review M3/H5 closure): the STRUCTURAL watch-only swap
    /// refusal crosses the FFI typed — kind `WatchOnly`, stable code
    /// RW-SWAP-015 — never folded into the RETRYABLE `ProviderUnavailable`
    /// (whose remedy, "try again", is exactly wrong here).
    #[test]
    fn watch_only_swap_error_crosses_the_ffi_typed_not_retryable() {
        let api: api_error::SwapApiError = rw::SwapError::WatchOnly.into();
        assert!(matches!(api.kind, api_error::SwapErrorKind::WatchOnly));
        assert_eq!(api.code, "RW-SWAP-015");
        // The adjacent guard: the genuinely-retryable outage keeps its own
        // kind + code — the two remedies never cross.
        let outage: api_error::SwapApiError = rw::SwapError::ProviderUnavailable.into();
        assert!(matches!(
            outage.kind,
            api_error::SwapErrorKind::ProviderUnavailable
        ));
        assert_eq!(outage.code, "RW-SWAP-006");
    }

    /// INC-018 (b), phase-2 P2-2: the retryable propose refusal crosses the FFI
    /// TYPED and apart from the deterministic one — the `WalletError → kind`
    /// match ends in `_ => K::Unknown`, so an arm left out would reach Dart as
    /// `unknown`, which the classifier maps to the generic "check the details":
    /// the incident's own copy, silently. Mutant: delete the `ProposeTransient`
    /// arm in `From<rw::WalletError>` → `kind` reads `Unknown` and the first
    /// assertion reds. The adjacent guard pins that the two classes keep their
    /// own kind + code, so the remedies never cross.
    #[test]
    fn propose_transient_crosses_the_ffi_typed_and_apart_from_propose_failed() {
        let transient: api_error::WalletApiError = rw::WalletError::ProposeTransient.into();
        assert!(
            matches!(transient.kind, api_error::WalletErrorKind::ProposeTransient),
            "the retryable class must cross typed, never as Unknown (code {})",
            transient.code
        );
        assert_eq!(transient.code, "RW-SEND-008");
        let deterministic: api_error::WalletApiError = rw::WalletError::ProposeFailed.into();
        assert!(matches!(
            deterministic.kind,
            api_error::WalletErrorKind::ProposeFailed
        ));
        assert_eq!(deterministic.code, "RW-SEND-005");
    }

    #[test]
    fn custody_disclosure_dto_maps_fields_without_transposition() {
        // FR-14 H1 (money/security honesty): the pre-wipe disclosure DTO must
        // carry the tier label, the erase assurance and `degraded` field-for-field
        // — a wrong assurance would tell the user a best-effort wipe deleted a
        // hardware-held key (or vice-versa). DISTINCT values per field so a swap
        // fails loudly.
        // A hardware tier: HardwareKeyDeleted, degraded FALSE.
        let se = custody_disclosure_dto(rw::CustodyDisclosure {
            tier: Some(rw::VaultTier::AppleSecureEnclave),
            erase_assurance: rw::EraseAssurance::HardwareKeyDeleted,
            degraded: false,
        });
        assert_eq!(se.tier, "apple_secure_enclave");
        assert!(
            matches!(
                se.erase_assurance,
                api_wallet::EraseAssurance::HardwareKeyDeleted
            ),
            "an SE key is hardware-held"
        );
        assert!(!se.degraded, "SE is hardware-backed, not degraded");

        // A software keystore: BestEffort, degraded TRUE — the OPPOSITE values,
        // so a transposition can't pass both this and the SE case.
        let sw = custody_disclosure_dto(rw::CustodyDisclosure {
            tier: Some(rw::VaultTier::SoftwareKeystore),
            erase_assurance: rw::EraseAssurance::BestEffort,
            degraded: true,
        });
        assert_eq!(sw.tier, "software_keystore");
        assert!(
            matches!(sw.erase_assurance, api_wallet::EraseAssurance::BestEffort),
            "software keystore is best-effort"
        );
        assert!(sw.degraded, "software keystore is degraded custody");

        // No platform vault (headless desktop): label "none", BestEffort, not degraded.
        let none = custody_disclosure_dto(rw::CustodyDisclosure {
            tier: None,
            erase_assurance: rw::EraseAssurance::BestEffort,
            degraded: false,
        });
        assert_eq!(
            none.tier, "none",
            "absent tier renders 'none', never 'unknown'"
        );
        assert!(matches!(
            none.erase_assurance,
            api_wallet::EraseAssurance::BestEffort
        ));
        assert!(!none.degraded);
    }

    #[test]
    fn validate_address_reports_memo_capability_and_typed_errors() {
        // The live send-form recipient gate (`validateAddress`): pins the DTO
        // mapping (`memo_capable`) AND the typed-error crossing for the new
        // surface. The classification itself is exhaustively covered by
        // zec-wallet-core's `Address::parse` tests; THIS guards the bridge glue —
        // a future refactor that inverted the flag or mapped the wrong error
        // kind would fail here, not silently mis-gate the memo field on-device.
        use zcash_address::ToAddress;
        use zcash_protocol::consensus::NetworkType;
        // valid-by-construction mainnet vectors — the SAME audited encoder the
        // parser uses (payload bytes arbitrary; the encoding/checksum is real).
        let sapling =
            zcash_address::ZcashAddress::from_sapling(NetworkType::Main, [0xAB; 43]).encode();
        let p2pkh =
            zcash_address::ZcashAddress::from_transparent_p2pkh(NetworkType::Main, [0xCD; 20])
                .encode();

        // shielded ⇒ memo-capable (private)
        assert!(
            matches!(
                validate_address(&sapling, api_config::Network::Main),
                Ok(api_payments::ValidatedAddress { memo_capable: true })
            ),
            "a sapling recipient can receive a memo",
        );

        // transparent ⇒ NOT memo-capable (public)
        assert!(
            matches!(
                validate_address(&p2pkh, api_config::Network::Main),
                Ok(api_payments::ValidatedAddress {
                    memo_capable: false
                })
            ),
            "a transparent recipient cannot receive a memo",
        );

        // garbage ⇒ typed AddressInvalid (the address is never echoed back)
        assert!(matches!(
            validate_address("u1notanaddress", api_config::Network::Main),
            Err(e) if matches!(e.kind, api_error::WalletErrorKind::AddressInvalid)
        ));

        // a MAINNET address under a Test config ⇒ typed NetworkMismatch, the
        // distinct "this is for a different network" arm (never "malformed")
        assert!(matches!(
            validate_address(&sapling, api_config::Network::Test),
            Err(e) if matches!(e.kind, api_error::WalletErrorKind::NetworkMismatch)
        ));

        // a UNIFIED address bundling a shielded receiver — THE realistic Zcash
        // recipient — is memo-capable (private). Pins the common case, not only
        // the bare-Sapling / bare-p2pkh extremes.
        use zcash_address::unified::{Address as Ua, Encoding, Receiver};
        let ua = Ua::try_from_items(vec![
            Receiver::Orchard([0xEF; 43]),
            Receiver::Sapling([0xAB; 43]),
        ])
        .expect("valid receiver set")
        .encode(&NetworkType::Main);
        assert!(
            matches!(
                validate_address(&ua, api_config::Network::Main),
                Ok(api_payments::ValidatedAddress { memo_capable: true })
            ),
            "a unified address with a shielded receiver is memo-capable (private)",
        );
    }

    #[test]
    fn validate_address_rejects_oversized_input_without_panic() {
        // A megabyte-scale clipboard paste (invariant 7 — every byte from the
        // outside is hostile): size-capped BEFORE the parse, returned as a typed
        // AddressInvalid, never an OOM or a panic crossing the sync FFI.
        let huge = "x".repeat(1_000_000);
        assert!(matches!(
            validate_address(&huge, api_config::Network::Main),
            Err(e) if matches!(e.kind, api_error::WalletErrorKind::AddressInvalid)
        ));
    }

    proptest::proptest! {
        /// `validate_address` is the byte-level gate the host calls on every
        /// keystroke (the Dart 512 cap is UX, not the security boundary). Over
        /// ARBITRARY input it must NEVER panic across the FFI and must return only
        /// the two typed arms (`AddressInvalid` / `NetworkMismatch`) — never an
        /// unexpected variant, never an Ok for a string that didn't parse.
        #[test]
        fn validate_address_never_panics_and_is_typed(s in ".*") {
            match validate_address(&s, api_config::Network::Main) {
                // a generated string that happens to be a valid mainnet address
                Ok(_) => {}
                Err(e) => proptest::prop_assert!(matches!(
                    e.kind,
                    api_error::WalletErrorKind::AddressInvalid
                        | api_error::WalletErrorKind::NetworkMismatch
                )),
            }
        }
    }

    #[test]
    fn wallet_state_maps_balances_tip_and_seq_field_for_field() {
        // distinct per-field values: a swapped field cannot pass
        let core = rw::WalletState {
            balance: rw::BalanceSnapshot {
                spendable: zat(11),
                pending_incoming: zat(22),
                pending_change: zat(33),
                transparent: zat(44),
                total: zat(110),
            },
            sync: rw::SyncStatus::UpToDate {
                tip: rw::BlockHeight::new(2_500_000),
            },
            tor: rw::TorState::Off,
            tip: Some(rw::BlockHeight::new(2_500_000)),
            // the handle's CURRENT contract: always-None balance age (the
            // persisted stamp is deferred) — a permanent null, never fabricated
            last_synced: None,
            ever_synced: true,
            rescan_rebuilding: false,
            seq: 42,
        };

        let dto = api_state::WalletState::from(core);

        assert_eq!(dto.balance.spendable_zat, 11);
        assert_eq!(dto.balance.pending_incoming_zat, 22);
        assert_eq!(dto.balance.pending_change_zat, 33);
        assert_eq!(dto.balance.transparent_zat, 44);
        assert_eq!(dto.balance.total_zat, 110);
        assert_eq!(dto.tip, Some(2_500_000));
        assert_eq!(dto.seq, 42);
        // the always-None contract crosses as a real null (an honest "age
        // unknown"), never a 0/epoch stamp — the host renders staleness from
        // tip + syncStatus
        assert!(dto.last_synced.is_none());
        assert!(matches!(
            dto.sync_status,
            api_state::SyncStatus::UpToDate { tip: 2_500_000 }
        ));
        assert!(matches!(dto.tor, api_state::TorState::Off));
    }

    #[test]
    fn sync_stamp_maps_height_and_unix_seconds_when_present() {
        // when the persisted stamp lands, the conversion must carry height +
        // unix-seconds faithfully (display-only `at`) — pin it now so the
        // wiring slice only flips `None`→`Some` at the source, not the mapping
        let core = rw::WalletState {
            balance: rw::BalanceSnapshot::default(),
            sync: rw::SyncStatus::Idle,
            tor: rw::TorState::Off,
            tip: None,
            last_synced: Some(rw::SyncStamp {
                height: rw::BlockHeight::new(2_499_900),
                at: 1_780_000_000,
            }),
            ever_synced: false,
            // s357b-2: a mid-rebuild snapshot — pins the field crossing true
            rescan_rebuilding: true,
            seq: 0,
        };

        let dto = api_state::WalletState::from(core);

        assert!(dto.rescan_rebuilding, "breadcrumb carried verbatim");
        let stamp = dto.last_synced.expect("stamp present");
        assert_eq!(stamp.height, 2_499_900);
        assert_eq!(stamp.at, 1_780_000_000);
        // tip absent stays absent — never coerced to 0
        assert_eq!(dto.tip, None);
    }

    /// An otherwise-valid config with the given `db_dir` — the config-door
    /// tests vary ONLY the field under test.
    fn config_with_db_dir(db_dir: &str) -> api_config::WalletConfig {
        api_config::WalletConfig {
            db_dir: db_dir.to_string(),
            network: api_config::Network::Main,
            endpoint_url: "https://zec.rocks:443".to_string(),
            endpoint_auth_header: None,
            endpoint_auth_value: None,
            tor: api_config::TorPolicy::Off,
            seed_persistence: api_config::SeedPersistence::SealedKeychain,
            birthday_height: None,
            broadcast_jitter: api_config::JitterPolicy::None,
            // FR-27 is opt-in: no read scope unless a row registers one.
            machine_memo_prefixes: Vec::new(),
            sync_servers: None,
        }
    }

    #[test]
    fn config_door_refuses_half_an_endpoint_auth_pair() {
        // a host that sets ONE half means to authenticate. Letting it
        // through would build a wallet that talks to a gated endpoint with no
        // credential — and that fails LATER, on the network, as an opaque
        // refusal a user cannot act on. Both half-shapes are a typed refusal at
        // the door, and the complete pair still crosses.
        let mut only_header = config_with_db_dir("/tmp/zw-auth");
        only_header.endpoint_auth_header = Some("x-zcash-rpc-key".to_string());
        assert!(
            rw::WalletConfig::try_from(only_header).is_err(),
            "a header name with no value must be refused at the door"
        );

        let mut only_value = config_with_db_dir("/tmp/zw-auth");
        only_value.endpoint_auth_value = Some("abc123".to_string());
        assert!(
            rw::WalletConfig::try_from(only_value).is_err(),
            "a value with no header name must be refused at the door"
        );

        let mut both = config_with_db_dir("/tmp/zw-auth");
        both.endpoint_auth_header = Some("x-zcash-rpc-key".to_string());
        both.endpoint_auth_value = Some("abc123".to_string());
        assert!(
            rw::WalletConfig::try_from(both).is_ok(),
            "the complete pair crosses — or the two refusals above prove nothing"
        );

        // And the core's validation is reached THROUGH this door, not bypassed:
        // a value carrying CRLF (header injection) is refused here too.
        let mut injected = config_with_db_dir("/tmp/zw-auth");
        injected.endpoint_auth_header = Some("x-zcash-rpc-key".to_string());
        injected.endpoint_auth_value = Some("abc\r\nx-injected: 1".to_string());
        assert!(
            rw::WalletConfig::try_from(injected).is_err(),
            "the door must not bypass EndpointAuth::new's injection check"
        );
    }

    #[test]
    fn config_door_rejects_empty_and_relative_db_dir_typed() {
        // #324: an empty/relative db_dir used to
        // cross UNVALIDATED and `create_dir_all` against the process cwd —
        // desktop silently plants the wallet under the launch directory, and
        // the next launch from another cwd offers a fresh CREATE over a
        // funded wallet. Both shapes must be a typed RW-CFG-003 at the door.
        for bad in ["", "relative/wallet", "./wallet", "../wallet"] {
            // NB not expect_err: the core WalletConfig has no Debug on purpose
            // (it can carry an injected dialer).
            let err = match rw::WalletConfig::try_from(config_with_db_dir(bad)) {
                Ok(_) => panic!("db_dir {bad:?} must be rejected"),
                Err(e) => e,
            };
            assert!(
                matches!(err, rw::WalletError::InvalidDbDir { .. }),
                "db_dir {bad:?} must reject as InvalidDbDir, got {err:?}"
            );
            assert_eq!(err.code(), "RW-CFG-003");
        }
    }

    /// FR-29 spec §2.1 / §6.1 E1 at the BRIDGE door (T3's Dart-side half): a
    /// `hostDialer` runtime maps to the core's `TorRuntime::HostDialer` exactly
    /// when the cabi registry holds a registration, and is refused with the
    /// door's reason (`RW-CFG-001`, "no host dialer registered") when it does
    /// not — the same refusal the core's `validate_transport` makes, one step
    /// earlier, so a host that validates before it registers learns it here
    /// and never at a first dial. The registry is process-global (C3a's
    /// module) and its own tests register and reset it in PARALLEL threads of
    /// this binary behind a lock private to that module, so this test never
    /// compares two reads: each mapping is graded on its own outcome — the
    /// `Ok` arm is `HostDialer` and nothing else, the `Err` arm is exactly
    /// the door's refusal. "Refused exactly when the slot is empty" is the
    /// cabi module's own row (`registered_host_dialer()` is `None` after a
    /// reset and `Some` after a register, under its lock); the mapping here
    /// is a pure `map`/`ok_or` over that one call. Under the `convert::tests::`
    /// filter (the watch) the slot is deterministically empty.
    #[test]
    fn config_door_refuses_host_dialer_when_nothing_is_registered() {
        match rw::TorRuntime::try_from(api_config::TorRuntimeConfig::HostDialer) {
            Ok(rw::TorRuntime::HostDialer(_)) => {}
            Ok(_) => panic!("hostDialer maps to TorRuntime::HostDialer and nothing else"),
            Err(rw::WalletError::InvalidEndpoint { reason }) => {
                assert!(
                    reason.contains("no host dialer registered"),
                    "the reason names the missing registration, not a network fault: {reason}"
                );
            }
            Err(other) => panic!("the door's refusal is InvalidEndpoint, got {other:?}"),
        }
        // Through the whole config: the same refusal, the door's code.
        let mut c = config_with_db_dir("/tmp/zw-host-dialer");
        c.tor = api_config::TorPolicy::Required {
            runtime: api_config::TorRuntimeConfig::HostDialer,
        };
        match rw::WalletConfig::try_from(c) {
            Ok(built) => assert!(
                matches!(
                    built.tor,
                    rw::TorPolicy::Required {
                        runtime: rw::TorRuntime::HostDialer(_)
                    }
                ),
                "a registered slot maps the whole policy through, runtime intact"
            ),
            Err(e) => {
                assert_eq!(e.code(), "RW-CFG-001", "the door's code, not a network one");
                assert!(
                    matches!(&e, rw::WalletError::InvalidEndpoint { reason }
                        if reason.contains("no host dialer registered")),
                    "the whole-config refusal carries the same reason: {e:?}"
                );
            }
        }
    }

    /// **T0-6.** The only Tor runtime a Dart host can name is refused at the
    /// config door, with a reason, rather than at the first dial.
    ///
    /// `TorRuntimeConfig`'s SOCKS variant, `ExternalSocks5`, has no dialer
    /// built for it: `net::dialer::runtime_dialer` returns `UnsupportedRuntime`
    /// and `resolve_dialer` propagates that with a `?`, so
    /// `Preferred { ExternalSocks5 }` never reaches the clearnet fallback its
    /// own name promises. Before T0-6 a host could configure Tor, see the config
    /// validate, open a wallet, and only then meet a network-shaped failure — the
    /// worst place to learn a privacy setting is not wired. Both policies that
    /// carry a runtime must refuse, and `Off` must still cross (or the two
    /// refusals prove nothing but that the config is broken).
    #[test]
    fn config_door_refuses_the_unbuilt_external_socks5_tor_runtime() {
        let socks = || api_config::TorRuntimeConfig::ExternalSocks5 {
            addr: "127.0.0.1:9050".to_string(),
        };
        for (name, policy) in [
            (
                "Preferred",
                api_config::TorPolicy::Preferred { runtime: socks() },
            ),
            (
                "Required",
                api_config::TorPolicy::Required { runtime: socks() },
            ),
        ] {
            let mut c = config_with_db_dir("/tmp/zw-tor");
            c.tor = policy;
            let err = match rw::WalletConfig::try_from(c) {
                Ok(_) => panic!(
                    "{name} {{ ExternalSocks5 }} must be refused at the door — this build has no \
                     dialer for it, so letting it cross only moves the failure to the first dial"
                ),
                Err(e) => e,
            };
            assert!(
                matches!(err, rw::WalletError::InvalidEndpoint { .. }),
                "{name}: the refusal is the config-door kind, got {err:?}"
            );
            assert_eq!(err.code(), "RW-CFG-001");
            assert!(
                err.to_string().contains("not built"),
                "{name}: the reason must say the runtime is not built, so a developer knows it \
                 is not a transient network condition; got {err}"
            );
        }

        let mut off = config_with_db_dir("/tmp/zw-tor");
        off.tor = api_config::TorPolicy::Off;
        assert!(
            rw::WalletConfig::try_from(off).is_ok(),
            "TorPolicy::Off must still cross — or the refusals above are about the fixture, not \
             about the runtime"
        );
    }

    #[test]
    fn config_door_accepts_a_platform_absolute_db_dir() {
        // `Path::is_absolute` is the platform-correct predicate: POSIX `/…`;
        // Windows needs the drive prefix (`C:\…` — a bare `\dir` is only
        // drive-relative there, and a POSIX-style `/dir` is not absolute).
        let abs = if cfg!(windows) {
            r"C:\wallet\data"
        } else {
            "/wallet/data"
        };
        rw::WalletConfig::try_from(config_with_db_dir(abs))
            .expect("an absolute db_dir passes the door");
    }

    #[tokio::test]
    async fn db_dir_only_entries_reject_a_relative_dir_typed_too() {
        // security review H2: `wallet_exists`/`wipe_wallet`/
        // `custody_disclosure` skip the TRANSPORT door by design, but the
        // dbDir SHAPE gate must still hold — a relative dir would stat (or
        // SHRED) against the process cwd. All three must reject RW-CFG-003
        // BEFORE touching the filesystem.
        // A wipe quiesces the process device log (S5): serialise with the tests
        // that assert on it.
        let _serial = crate::device_log::test_sink::PROCESS_LOG.lock().await;
        let e = wallet_exists(config_with_db_dir("relative/probe"))
            .await
            .expect_err("exists must reject a relative dir");
        assert_eq!(e.code, "RW-CFG-003");

        let e = wipe_wallet(config_with_db_dir("relative/shred"), false)
            .await
            .expect_err("wipe must reject a relative dir");
        assert_eq!(e.code, "RW-CFG-003");
        let e = wipe_wallet(config_with_db_dir(""), true)
            .await
            .expect_err("wipe_force must reject an empty dir");
        assert_eq!(e.code, "RW-CFG-003");

        // NB not expect_err: the disclosure DTO has no Debug.
        let e = match custody_disclosure(config_with_db_dir("./custody")).await {
            Ok(_) => panic!("custody_disclosure must reject a relative dir"),
            Err(e) => e,
        };
        assert_eq!(e.code, "RW-CFG-003");
    }

    #[test]
    fn config_door_db_dir_error_surfaces_typed_across_the_api() {
        // The full FFI shape the settings-form validator sees: kind carries
        // the reason, code rides the stable RW-CFG-003.
        let core_err = match rw::WalletConfig::try_from(config_with_db_dir("wallets")) {
            Ok(_) => panic!("relative db_dir must be rejected"),
            Err(e) => e,
        };
        let api_err: api_error::WalletApiError = core_err.into();
        assert_eq!(api_err.code, "RW-CFG-003");
        assert!(matches!(
            api_err.kind,
            api_error::WalletErrorKind::InvalidDbDir { .. }
        ));
    }

    #[test]
    fn wallet_state_balance_max_supply_crosses_bridge_exact_no_clamp() {
        // a whale wallet at the full money supply must show the EXACT balance —
        // max supply (2.1e15) < 2^53 so it is exact across the Dart `int`. The
        // store layer pins the value-PRODUCTION flip (`to_sdk_zat`); this pins
        // the BRIDGE crossing (`Zatoshis -> i64`) at the boundary, so a future
        // "improvement" to the cast (a clamp, a u64-backed field, a narrowed
        // i64) fails here instead of silently mis-displaying a large balance.
        let max = rw::constants::MAX_MONEY_ZAT; // 2_100_000_000_000_000
        let core = rw::WalletState {
            balance: rw::BalanceSnapshot {
                spendable: zat(max),
                pending_incoming: zat(max),
                pending_change: zat(max),
                transparent: zat(max),
                total: zat(max),
            },
            sync: rw::SyncStatus::UpToDate {
                tip: rw::BlockHeight::new(2_500_000),
            },
            tor: rw::TorState::Off,
            tip: Some(rw::BlockHeight::new(2_500_000)),
            last_synced: None,
            ever_synced: true,
            rescan_rebuilding: false,
            seq: 0,
        };

        let dto = api_state::WalletState::from(core);

        assert_eq!(dto.balance.spendable_zat, max);
        assert_eq!(dto.balance.pending_incoming_zat, max);
        assert_eq!(dto.balance.pending_change_zat, max);
        assert_eq!(dto.balance.transparent_zat, max);
        // total crosses independently (mapped directly, never re-summed — a
        // re-sum of five max-supply fields would overflow), exact at the bound
        assert_eq!(dto.balance.total_zat, max);
        assert_eq!(dto.balance.total_zat, 2_100_000_000_000_000);
    }

    #[test]
    fn connecting_percent_is_finite_or_none_across_the_bridge() {
        // the api `SyncStatus::Connecting` doc CONTRACT: every percent crossing
        // the bridge is finite (a NaN breaks the generated Dart value-equality —
        // NaN != NaN — turning every state update into a spurious rebuild and
        // never settling the §3.3 dedup). `snapshot()` carries the RAW
        // SyncStatus (NOT routed through `live_tor_state`), so the bridge is the
        // enforcement point. A non-finite percent must cross as `None`.
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let dto = api_state::SyncStatus::from(rw::SyncStatus::Connecting {
                tor_bootstrap_percent: Some(bad),
            });
            match dto {
                api_state::SyncStatus::Connecting {
                    tor_bootstrap_percent,
                } => assert!(
                    tor_bootstrap_percent.is_none(),
                    "a non-finite percent ({bad}) must not cross the bridge"
                ),
                _ => panic!("Connecting must map to Connecting"),
            }
        }
        // a finite percent passes through verbatim
        match api_state::SyncStatus::from(rw::SyncStatus::Connecting {
            tor_bootstrap_percent: Some(0.5),
        }) {
            api_state::SyncStatus::Connecting {
                tor_bootstrap_percent: Some(p),
            } => assert!((p - 0.5).abs() < f32::EPSILON),
            _ => panic!("a finite Connecting percent must cross verbatim"),
        }
    }

    #[test]
    fn sync_status_arms_map_to_their_named_targets() {
        // the lockstep gate proves every core variant has AN arm; it does NOT
        // prove the arm maps to the RIGHT api variant (a swapped arm — Idle =>
        // Scanning — passes the gate). Pin the VALUE mapping for the arms a cold
        // `snapshot()` can actually emit, with distinct payloads so a transposed
        // arm fails loudly. (The other enums' From impls predate this slice — a
        // full per-arm value-pinning pass over convert.rs is tracked debt.)
        use api_state::SyncStatus as A;
        assert!(matches!(A::from(rw::SyncStatus::Idle), A::Idle));
        assert!(matches!(
            A::from(rw::SyncStatus::Scanning {
                from: rw::BlockHeight::new(100),
                to: rw::BlockHeight::new(200),
                percent: 0.5,
                // The bools DIFFER (false, true): a dropped `rewound` mapping
                // (defaulting false) AND a transposed pair both fail.
                spendable_ready: false,
                rewound: true,
            }),
            A::Scanning {
                from: 100,
                to: 200,
                spendable_ready: false,
                rewound: true,
                ..
            }
        ));
        assert!(matches!(
            A::from(rw::SyncStatus::UpToDate {
                tip: rw::BlockHeight::new(2_500_000),
            }),
            A::UpToDate { tip: 2_500_000 }
        ));
        // the honest-local stall reason carries through to the named target
        assert!(matches!(
            A::from(rw::SyncStatus::Stalled {
                reason: rw::StallReason::Internal,
            }),
            A::Stalled {
                reason: api_state::StallReason::Internal
            }
        ));
        // Offline carries its (reserved) last_synced Option
        assert!(matches!(
            A::from(rw::SyncStatus::Offline { last_synced: None }),
            A::Offline { last_synced: None }
        ));
    }

    #[test]
    fn degraded_pool_status_crosses_the_bridge_pool_for_pool() {
        // T0-1b: the arm-coverage gate proves the arm EXISTS; this pins that the
        // per-pool report crosses field-for-field with DISTINCT values, so a swapped
        // pool (Ironwood's refusal shown against Sapling) or a dropped count fails.
        use api_state::PoolService as P;
        use api_state::SyncStatus as A;
        let crossed = A::from(rw::SyncStatus::UpToDateDegraded {
            tip: rw::BlockHeight::new(3_500_000),
            pools: rw::PoolServiceReport {
                sapling: rw::PoolService::Served { roots: 11 },
                orchard: rw::PoolService::HeightViolation,
                ironwood: rw::PoolService::Unsupported,
            },
        });
        let A::UpToDateDegraded { tip, pools } = crossed else {
            panic!("UpToDateDegraded must cross as its own arm, never Unknown");
        };
        assert_eq!(tip, 3_500_000);
        assert!(matches!(pools.sapling, P::Served { roots: 11 }));
        assert!(matches!(pools.orchard, P::HeightViolation));
        assert!(matches!(pools.ironwood, P::Unsupported));
        // A zero-root serve crosses as a served ZERO — never as unsupported.
        assert!(matches!(
            api_state::PoolService::from(rw::PoolService::Served { roots: 0 }),
            P::Served { roots: 0 }
        ));
        // A withheld pool crosses with the bundle's proof, never as a served zero.
        assert!(matches!(
            api_state::PoolService::from(rw::PoolService::Withheld { proven: 4 }),
            P::Withheld { proven: 4 }
        ));
        // D9/D10: the server-misbehaved stall is its own named arm, not Unknown and
        // not folded into either neighbour.
        assert!(matches!(
            api_state::StallReason::from(rw::StallReason::EndpointMisbehaving),
            api_state::StallReason::EndpointMisbehaving
        ));
        // T0-1c-R2 (§4m #13): a birthday above the chain this server reports is
        // its own named arm too — not Unknown, not folded into a neighbour.
        assert!(matches!(
            api_state::StallReason::from(rw::StallReason::BirthdayInFuture),
            api_state::StallReason::BirthdayInFuture
        ));
        // R10: a busy or I/O-faulted store is its own named arm — folded into
        // `Internal` it would name the restore remedy, into Unknown the generic one.
        assert!(matches!(
            api_state::StallReason::from(rw::StallReason::StorageUnavailable),
            api_state::StallReason::StorageUnavailable
        ));
    }

    #[test]
    fn endpoint_behind_status_crosses_the_bridge_with_both_heights_and_the_pools_report() {
        // T0-1c: the arm-coverage gate proves the arm EXISTS; this pins that BOTH
        // heights cross un-transposed (distinct values — a swapped `tip` /
        // `newest_known` would tell a host the server is AHEAD) and that the pools
        // report rides with the behind claim field-for-field, `None` staying `None`
        // (never a fabricated report).
        use api_state::PoolService as P;
        use api_state::SyncStatus as A;
        let crossed = A::from(rw::SyncStatus::EndpointBehind {
            tip: rw::BlockHeight::new(3_400_000),
            newest_known: rw::BlockHeight::new(3_500_000),
            pools: Some(rw::PoolServiceReport {
                sapling: rw::PoolService::Served { roots: 11 },
                orchard: rw::PoolService::Withheld { proven: 2 },
                ironwood: rw::PoolService::Unsupported,
            }),
        });
        let A::EndpointBehind {
            tip,
            newest_known,
            pools,
        } = crossed
        else {
            panic!("EndpointBehind must cross as its own arm, never Unknown");
        };
        assert_eq!(tip, 3_400_000);
        assert_eq!(newest_known, 3_500_000);
        let pools = pools.expect("the pools report crosses with the behind claim");
        assert!(matches!(pools.sapling, P::Served { roots: 11 }));
        assert!(matches!(pools.orchard, P::Withheld { proven: 2 }));
        assert!(matches!(pools.ironwood, P::Unsupported));
        assert!(matches!(
            A::from(rw::SyncStatus::EndpointBehind {
                tip: rw::BlockHeight::new(1),
                newest_known: rw::BlockHeight::new(2),
                pools: None,
            }),
            A::EndpointBehind {
                tip: 1,
                newest_known: 2,
                pools: None
            }
        ));
    }

    #[test]
    fn send_proposal_maps_money_and_steps_field_for_field() {
        // THE prepared-send confirm screen the user reads BEFORE signing: if a
        // field were transposed (fee shown as change, a recipient amount swapped
        // for the total) or the de-shield disclosure inverted, the user would
        // approve numbers that are NOT what gets signed. Pin it field-for-field
        // with DISTINCT values + a two-recipient step (different pools), mirroring
        // `wallet_state_maps_balances_tip_and_seq_field_for_field` — hand-built, no
        // funded wallet (`propose` needs notes; the From conversion is pure).
        let core = rw::SendProposal {
            proposal_id: 7,
            binding: rw::SpendBinding::mint(),
            total_zat: 150_500, // = recipients (130_000 + 20_000) + fee (500)
            fee_zat: 500,
            change_zat: 9_499_500,
            steps: vec![rw::ProposalStep {
                recipients: vec![
                    rw::ProposalRecipient {
                        pool: rw::OutputPool::Orchard,
                        amount_zat: 130_000,
                    },
                    rw::ProposalRecipient {
                        pool: rw::OutputPool::Transparent,
                        amount_zat: 20_000,
                    },
                ],
            }],
            target_height: 2_500_123,
            // a transparent recipient is present above ⇒ the de-shield disclosure
            // is TRUE; an inverted bool here would hide a public leg from the user
            has_transparent_recipient: true,
            is_shield: false,       // an ordinary send, not a shield
            is_two_step_tex: false, // a single-step send
            large_send: None, // §3 money-safety signals are deferred at the bridge (see the From impl)
            self_send: false,
            single_recipient_zat: None, // two recipients ⇒ no single-recipient figure
        };

        let dto = api_payments::SendProposal::from(core);

        // the headline money path, each field distinct so a swap fails loudly
        assert_eq!(dto.proposal_id, 7, "the retained one-shot token id");
        assert_eq!(
            dto.total_zat, 150_500,
            "total = recipients + fee, the headline"
        );
        assert_eq!(dto.fee_zat, 500, "the ZIP-317 fee, not the change");
        assert_eq!(dto.change_zat, 9_499_500, "change back to the wallet");
        assert_eq!(
            dto.target_height, 2_500_123,
            "the anchor height carries u32"
        );
        assert!(
            dto.has_transparent_recipient,
            "a transparent leg present ⇒ the de-shield disclosure must be TRUE"
        );

        // the per-step recipients cross in ORDER, each with its own pool + amount
        assert_eq!(dto.steps.len(), 1, "this build mints a single step");
        let recipients = &dto.steps[0].recipients;
        assert_eq!(recipients.len(), 2, "both recipient outputs cross");
        assert_eq!(
            recipients[0].amount_zat, 130_000,
            "recipient 0 amount verbatim"
        );
        assert!(
            matches!(recipients[0].pool, api_payments::OutputPool::Orchard),
            "recipient 0 pool maps to Orchard (no Debug on the DTO, hence match)"
        );
        assert_eq!(
            recipients[1].amount_zat, 20_000,
            "recipient 1 amount verbatim"
        );
        assert!(
            matches!(recipients[1].pool, api_payments::OutputPool::Transparent),
            "recipient 1 pool maps to Transparent — the de-shield leg"
        );
    }

    #[test]
    fn send_proposal_with_no_transparent_recipient_carries_a_false_disclosure() {
        // The OTHER half of the §5.1 disclosure: a fully-shielded proposal must
        // cross with `has_transparent_recipient == false`. `send_proposal_maps_..`
        // pins the TRUE branch; a refactor that hard-coded the disclosure (or read
        // the wrong field) would still pass that one but flip this. The user must
        // see "no funds leave the shielded set" honestly, never a spurious warning.
        let core = rw::SendProposal {
            proposal_id: 1,
            binding: rw::SpendBinding::mint(),
            total_zat: 50_000,
            fee_zat: 0,
            change_zat: 0,
            steps: vec![rw::ProposalStep {
                recipients: vec![rw::ProposalRecipient {
                    pool: rw::OutputPool::Sapling,
                    amount_zat: 50_000,
                }],
            }],
            target_height: 2_500_000,
            has_transparent_recipient: false,
            is_shield: false,
            is_two_step_tex: false,
            large_send: None,
            self_send: false,
            single_recipient_zat: Some(50_000),
        };

        let dto = api_payments::SendProposal::from(core);

        assert_eq!(
            dto.single_recipient_zat,
            Some(50_000),
            "FR-46: the one recipient's amount crosses verbatim"
        );
        assert!(
            !dto.has_transparent_recipient,
            "a fully-shielded send discloses NO de-shield"
        );
        assert!(
            matches!(
                dto.steps[0].recipients[0].pool,
                api_payments::OutputPool::Sapling
            ),
            "the single shielded recipient maps to Sapling"
        );
        assert!(!dto.is_shield, "an ordinary send is NOT tagged as a shield");
    }

    #[test]
    fn shield_proposal_crosses_as_a_shield_with_no_deshield_disclosure() {
        // Recv-3: a SHIELD proposal must cross with `is_shield == true` AND
        // `has_transparent_recipient == false` — the host shows the shield confirm copy,
        // not a send/recipient screen, and never a spurious de-shield warning (a shield is
        // privacy-POSITIVE). A refactor that dropped the field (or inverted it) flips here.
        // The lone output is the wallet's OWN shielded note (gross 200_000, net = gross−fee).
        let core = rw::SendProposal {
            proposal_id: 9,
            binding: rw::SpendBinding::mint(),
            total_zat: 200_000, // gross transparent being shielded
            fee_zat: 15_000,
            change_zat: 185_000, // net shielded = gross − fee
            steps: vec![rw::ProposalStep {
                recipients: vec![rw::ProposalRecipient {
                    pool: rw::OutputPool::Orchard,
                    amount_zat: 185_000,
                }],
            }],
            target_height: 2_500_321,
            has_transparent_recipient: false,
            is_shield: true,
            is_two_step_tex: false, // a shield is single-step
            large_send: None,
            self_send: false,
            single_recipient_zat: None, // a shield has no external recipient
        };

        let dto = api_payments::SendProposal::from(core);

        assert!(
            dto.is_shield,
            "a shield proposal crosses tagged as a shield"
        );
        assert!(
            !dto.has_transparent_recipient,
            "a shield is privacy-positive — NO de-shield disclosure"
        );
        assert_eq!(dto.total_zat, 200_000, "gross crosses exact");
        assert_eq!(dto.change_zat, 185_000, "net shielded crosses exact");
        assert!(
            matches!(
                dto.steps[0].recipients[0].pool,
                api_payments::OutputPool::Orchard
            ),
            "the self-output lands in the shielded Orchard pool"
        );
    }

    #[test]
    fn send_proposal_near_max_money_crosses_exact_no_clamp() {
        // A whale-scale send (a near-max-supply total) must show the EXACT integer
        // zatoshis — max supply (2.1e15) < 2^53 so it is exact across the Dart
        // `int`. Mirrors `wallet_state_balance_max_supply_crosses_bridge_exact_no_clamp`
        // for the SEND DTO: a future cast change (a clamp, a u64-backed field, a
        // narrowed i64) fails here instead of silently mis-displaying the amount a
        // user is about to sign. Integer zatoshis throughout — never a float.
        let max = rw::constants::MAX_MONEY_ZAT; // 2_100_000_000_000_000
        let core = rw::SendProposal {
            proposal_id: 2,
            binding: rw::SpendBinding::mint(),
            total_zat: max,
            fee_zat: 1_000,
            change_zat: max - 1_000,
            steps: vec![rw::ProposalStep {
                recipients: vec![rw::ProposalRecipient {
                    pool: rw::OutputPool::Orchard,
                    amount_zat: max,
                }],
            }],
            target_height: u32::MAX,
            has_transparent_recipient: false,
            is_shield: false,
            is_two_step_tex: false,
            large_send: None,
            self_send: false,
            single_recipient_zat: Some(max),
        };

        let dto = api_payments::SendProposal::from(core);

        assert_eq!(dto.total_zat, max, "the full-supply total crosses exact");
        assert_eq!(dto.total_zat, 2_100_000_000_000_000);
        assert_eq!(dto.change_zat, max - 1_000, "change near max crosses exact");
        assert_eq!(
            dto.steps[0].recipients[0].amount_zat, max,
            "the recipient amount is exact"
        );
        assert_eq!(
            dto.single_recipient_zat,
            Some(max),
            "FR-46: the single-recipient figure crosses exact"
        );
        // the anchor height is a full u32 — no narrowing on the crossing
        assert_eq!(dto.target_height, u32::MAX);
    }

    #[test]
    fn send_proposal_money_safety_signals_cross_the_bridge() {
        // §3 money-safety (#226): `large_send` (a reason) + `self_send` reach Dart. They were
        // bridge-DEFERRED while the host hadn't consumed them; this pins that they now CROSS and
        // that the reason maps arm-for-arm — a dropped/swapped signal would mis-key the host's
        // large-amount confirm copy or omit the self-send note.
        let core = rw::SendProposal {
            proposal_id: 5,
            binding: rw::SpendBinding::mint(),
            total_zat: 9_000_000,
            fee_zat: 1_000,
            change_zat: 0,
            steps: vec![rw::ProposalStep {
                recipients: vec![rw::ProposalRecipient {
                    pool: rw::OutputPool::Orchard,
                    amount_zat: 8_999_000,
                }],
            }],
            target_height: 2_500_000,
            has_transparent_recipient: false,
            is_shield: false,
            is_two_step_tex: false,
            large_send: Some(rw::LargeSendReason::NearTotalBalance),
            self_send: true,
            single_recipient_zat: Some(8_999_000),
        };
        let dto = api_payments::SendProposal::from(core);
        assert!(
            matches!(
                dto.large_send,
                Some(api_payments::LargeSendReason::NearTotalBalance)
            ),
            "the large-amount reason crosses arm-for-arm",
        );
        assert!(dto.self_send, "the self-send note crosses");

        // The reason mapping per variant (a swap would mis-key the host confirm copy).
        assert!(matches!(
            api_payments::LargeSendReason::from(rw::LargeSendReason::OverAbsoluteThreshold),
            api_payments::LargeSendReason::OverAbsoluteThreshold
        ));
        assert!(matches!(
            api_payments::LargeSendReason::from(rw::LargeSendReason::Both),
            api_payments::LargeSendReason::Both
        ));
    }

    #[test]
    fn output_pool_discriminants_are_wire_stable() {
        // `OutputPool` crosses the FFI as its DISCRIMINANT — flutter_rust_bridge
        // encodes it as an integer index and Dart decodes with
        // `OutputPool.values[raw]`. Adding `Ironwood` before `Unknown` moved
        // `Unknown` from 3 to 4, which is safe only because the Dart package and
        // the native library always ship as a matched pair. `zec_wallet` publishes
        // to pub.dev, so that pairing is a convention, not a mechanism: a host
        // resolving an older Dart package against a newer built library would
        // render an `Unknown` pool as `Ironwood` on the payment-confirm screen.
        //
        // Pin the numbers so the next variant is an explicit decision. THE RULE:
        // append after `Unknown`, never before it — inserting in the middle
        // renumbers every variant above the insertion point.
        assert_eq!(api_payments::OutputPool::Transparent as i32, 0);
        assert_eq!(api_payments::OutputPool::Sapling as i32, 1);
        assert_eq!(api_payments::OutputPool::Orchard as i32, 2);
        assert_eq!(api_payments::OutputPool::Ironwood as i32, 3);
        assert_eq!(
            api_payments::OutputPool::Unknown as i32,
            4,
            "a variant was inserted before `Unknown`, renumbering the wire encoding — \
             append after it instead, and regenerate the bindings"
        );
    }

    #[test]
    fn output_pool_maps_every_named_variant_to_its_target() {
        // The lockstep gate proves every core `OutputPool` variant has AN arm; it
        // does NOT prove the arm maps to the RIGHT api variant (a swapped
        // Sapling⇒Orchard passes the gate but would mislabel the pool a recipient's
        // funds land in — and a Transparent mislabeled shielded would HIDE a
        // de-shield). Pin the value mapping per variant. (`OutputPool` is the §5.1
        // disclosure primitive; a wrong pool is a privacy-disclosure defect.) No
        // Debug on the api enum (§5.4 posture), hence `matches!`.
        use api_payments::OutputPool as A;
        assert!(
            matches!(A::from(rw::OutputPool::Transparent), A::Transparent),
            "Transparent ⇒ Transparent (the de-shield pool — never collapse to shielded)"
        );
        assert!(matches!(A::from(rw::OutputPool::Sapling), A::Sapling));
        assert!(matches!(A::from(rw::OutputPool::Orchard), A::Orchard));
        // Ironwood (NU6.3) gets its OWN api variant, not the forward-compatibility
        // arm. That was not the first plan: the pin wave intended to leave it on
        // `Unknown` and owe the FFI surface change, and the project's own lockstep
        // gate (`extraction_policy::bridge_enums_cover_core_variants`) refused —
        // correctly, since after the activation Ironwood is the ORDINARY pool for a
        // shielded payment, so "render neutrally" would be the common case.
        assert!(matches!(A::from(rw::OutputPool::Ironwood), A::Ironwood));
        assert!(
            !matches!(A::from(rw::OutputPool::Ironwood), A::Transparent),
            "a shielded pool must never be disclosed as a de-shield (§5.1)"
        );
    }

    #[test]
    fn offline_with_last_synced_some_carries_the_stamp_across_the_bridge() {
        // The `Offline` arm has TWO reachable shapes on the LIVE stream: `None`
        // (connectivity just dropped, no prior successful sync) and `Some` (synced
        // before — the balance AGE the host renders on the offline indicator).
        // `sync_status_arms_map_to_their_named_targets` pins the `None` shape, and
        // `sync_stamp_maps_height_and_unix_seconds_when_present` pins the `SyncStamp`
        // conversion via `WalletState` — but NEITHER proves the `Some` branch of the
        // `Offline` arm's own `last_synced.map(Into::into)`, so a refactor that
        // collapsed it to `None` would ship a wrong/absent age to Dart and pass CI.
        // Pin it (money-UX: the offline staleness a user reads before deciding to
        // wait or retry). The api DTOs don't derive Debug (§5.4), hence `match`.
        let core = rw::SyncStatus::Offline {
            last_synced: Some(rw::SyncStamp {
                height: rw::BlockHeight::new(2_499_000),
                at: 1_780_000_000,
            }),
        };
        match api_state::SyncStatus::from(core) {
            api_state::SyncStatus::Offline {
                last_synced: Some(stamp),
            } => {
                assert_eq!(stamp.height, 2_499_000, "height crosses verbatim");
                assert_eq!(stamp.at, 1_780_000_000_i64, "unix seconds cross verbatim");
            }
            _ => panic!(
                "Offline{{Some(stamp)}} must map to Offline{{Some(stamp)}}, never collapse to None"
            ),
        }
    }

    // ─── Inbound swap conversions (the D-1 facade's hostile-boundary edges) ──────
    //
    // These are the Dart→core directions the four swap FFI methods drive. Every one is a
    // pure function (no vault, no network), so the real-world edges — the boundary cap that
    // stops a forever-spinning poll, the out-of-range/clamp/drop reconstruction arms, the
    // fail-safe kill mapping, the host-bug Unknown rejections — are unit-testable HERE; the
    // `StreamSink` accept path + the kill-layer-2 `SwapDisabled` path are owed to the
    // on-device e2e (they need a Dart port / a real open wallet — see api/wallet.rs).

    /// A well-formed IntoZec quote DTO (the shape Dart passes back to `swapExecute`); the
    /// edge tests mutate one field at a time so a single arm is exercised in isolation.
    fn dto_quote_into_zec() -> api_swap::SwapQuote {
        api_swap::SwapQuote {
            id: "dep-addr".to_string(),
            binding: None,
            deposit_address: "dep-addr".to_string(),
            deposit_memo: None,
            expires_at: 2_000_000_000,
            amount_in: "100".to_string(),
            min_amount_out: "0.9".to_string(),
            zec_side_zat: 100_000_000,
            refund_to: None,
            disclosure: api_swap::SwapPrivacyDisclosure {
                ends_shielded: true,
                deshields: false,
                provider_legs_transparent: true,
                provider_sees: vec![api_swap::DisclosureItem::Amounts],
            },
        }
    }

    #[test]
    fn swap_id_bound_rejects_empty_and_over_bound_accepts_at_the_boundary() {
        // the security-load-bearing guard: an over-bound/garbage id must NOT spawn a poll
        // (the loop retries provider faults forever ⇒ a never-terminating stream).
        let max = rw::constants::PROVIDER_STR_MAX_BYTES;
        assert!(check_swap_id_bound("").is_err(), "empty id rejected");
        assert!(
            check_swap_id_bound("dep-addr").is_ok(),
            "a normal id is accepted"
        );
        assert!(
            check_swap_id_bound(&"a".repeat(max)).is_ok(),
            "an id EXACTLY at the bound is accepted (the `>` boundary)"
        );
        assert!(
            check_swap_id_bound(&"a".repeat(max + 1)).is_err(),
            "one byte over the bound is rejected"
        );
    }

    #[test]
    fn swap_quote_inbound_rejects_out_of_range_zec_side() {
        let mut dto = dto_quote_into_zec();
        dto.zec_side_zat = -1; // Zatoshis::new rejects a negative
        match rw::SwapQuote::try_from(dto) {
            Err(rw::SwapError::RequestInvalid { .. }) => {}
            _ => panic!("an out-of-range zec side is a typed RequestInvalid"),
        }
    }

    #[test]
    fn swap_quote_deposit_memo_round_trips_both_directions() {
        // Dart → core carries a required deposit memo verbatim (the IntoZec
        // memo-chain case: XRP tag / Cosmos memo shown on D7).
        let mut dto = dto_quote_into_zec();
        dto.deposit_memo = Some("100345677".to_string());
        let core = rw::SwapQuote::try_from(dto).expect("inbound");
        assert_eq!(core.deposit_memo.as_deref(), Some("100345677"));
        // core → Dart carries it back (the outbound `From` arm).
        let back = api_swap::SwapQuote::from(core);
        assert_eq!(back.deposit_memo.as_deref(), Some("100345677"));
        // a None memo (every chain without the concept, incl. all ZEC deposits)
        // stays None across both arms.
        let none_core = rw::SwapQuote::try_from(dto_quote_into_zec()).expect("inbound none");
        assert_eq!(none_core.deposit_memo, None);
        assert_eq!(api_swap::SwapQuote::from(none_core).deposit_memo, None);
    }

    #[test]
    fn swap_quote_inbound_clamps_pre_epoch_expiry_and_drops_unknown_disclosure() {
        let mut dto = dto_quote_into_zec();
        dto.expires_at = -5; // a pre-epoch display value
        dto.disclosure.provider_sees = vec![
            api_swap::DisclosureItem::Amounts,
            api_swap::DisclosureItem::Unknown, // no core counterpart — dropped
            api_swap::DisclosureItem::IpUnlessTor,
        ];
        let core = rw::SwapQuote::try_from(dto).expect("a well-formed quote reconstructs");
        // a negative expiry clamps to 0 (a dead-past deadline), never a wrapped huge u64
        assert_eq!(core.expires_at, 0, "pre-epoch expiry clamps to 0");
        // the Unknown disclosure item is dropped; the two known items survive, in order
        assert_eq!(core.disclosure.provider_sees.len(), 2, "Unknown dropped");
        assert!(matches!(
            core.disclosure.provider_sees[0],
            rw::DisclosureItem::Amounts
        ));
        assert!(matches!(
            core.disclosure.provider_sees[1],
            rw::DisclosureItem::IpUnlessTor
        ));
    }

    #[test]
    fn quote_request_inbound_rejects_unknown_direction() {
        // a host-bug Unknown direction is rejected typed (Dart can only build variants this
        // binding generated, so an Unknown is never a legitimate request).
        match try_direction(api_swap::SwapDirection::Unknown) {
            Err(rw::SwapError::RequestInvalid { .. }) => {}
            _ => panic!("an Unknown direction is a typed RequestInvalid"),
        }
    }

    #[cfg(feature = "swap-near")]
    #[test]
    fn swap_kill_in_maps_each_severity_and_unknown_to_hard_fail_safe() {
        assert!(matches!(
            swap_kill_in(api_swap::SwapKill::Live),
            rw::SwapKill::Live
        ));
        assert!(matches!(
            swap_kill_in(api_swap::SwapKill::WindDown),
            rw::SwapKill::WindDown
        ));
        assert!(matches!(
            swap_kill_in(api_swap::SwapKill::Hard),
            rw::SwapKill::Hard
        ));
        // FAIL-SAFE (§3.5 monotonic-off): an unrecognized declared kill is treated as the
        // STRONGEST kill understood — an unknown directive can never RESURRECT swap.
        assert!(matches!(
            swap_kill_in(api_swap::SwapKill::Unknown),
            rw::SwapKill::Hard
        ));
    }

    #[test]
    fn list_tokens_bridge_round_trips() {
        // §3.3b D5 (IZ-2): the picker list crosses the bridge field-for-field — a transposed field
        // (e.g. decimals shown as price, the freshness flag inverted to hide a stale banner) would
        // mislead the user's source-asset pick. Pin it with DISTINCT values + a `None`-price row +
        // the stale flag so any swap/inversion fails loudly. (The `From` is pure — no live provider.)
        let core = rw::TokenList {
            tokens: vec![
                rw::TokenInfo {
                    chain: "near".into(),
                    symbol: "USDC".into(),
                    decimals: 6,
                    provider_asset_id: "nep141:usdc.near".into(),
                    price_usd: Some(0.99981),
                },
                rw::TokenInfo {
                    chain: "eth".into(),
                    symbol: "WETH".into(),
                    decimals: 18,
                    provider_asset_id: "nep141:weth.eth".into(),
                    price_usd: None, // a priceless row still crosses faithfully (the host may hide it)
                },
            ],
            fresh: false, // served-from-cache ⇒ the host's "couldn't refresh" banner
        };

        let dto = api_swap::SwapTokenList::from(core);

        assert!(!dto.fresh, "the freshness flag crosses (stale stays stale)");
        assert_eq!(dto.tokens.len(), 2);
        let usdc = &dto.tokens[0];
        assert_eq!(usdc.chain, "near");
        assert_eq!(usdc.symbol, "USDC");
        assert_eq!(usdc.decimals, 6);
        assert_eq!(usdc.provider_asset_id, "nep141:usdc.near");
        assert_eq!(usdc.price_usd, Some(0.99981));
        let weth = &dto.tokens[1];
        assert_eq!(weth.chain, "eth");
        assert_eq!(weth.symbol, "WETH");
        assert_eq!(
            weth.decimals, 18,
            "high-decimal asset crosses without truncation"
        );
        assert_eq!(weth.provider_asset_id, "nep141:weth.eth");
        assert_eq!(weth.price_usd, None, "a None price stays None, never 0");
    }

    #[test]
    fn empty_but_fresh_token_list_crosses_the_bridge_intact() {
        // §3.3b L6 (IZ-2): the honest "no source assets available right now" empty-picker state must
        // cross the bridge as an EMPTY list that is STILL `fresh = true` — distinct from a stale
        // served-from-cache list (`fresh = false`). If the `From` ever collapsed an empty list to a
        // stale flag (or vice-versa), the host would render the wrong banner: "couldn't refresh,
        // showing cached" over a genuinely empty provider, or "nothing available" over a stale cache.
        // Pin both ends of that distinction across the boundary.
        let empty_fresh = api_swap::SwapTokenList::from(rw::TokenList {
            tokens: vec![],
            fresh: true,
        });
        assert!(
            empty_fresh.tokens.is_empty(),
            "an empty core list crosses as an empty DTO list"
        );
        assert!(
            empty_fresh.fresh,
            "empty-but-FRESH stays fresh — the genuine empty-picker state, never mislabeled stale"
        );

        // the contrasting empty-STALE state (a fault with no cache producing nothing) stays stale.
        let empty_stale = api_swap::SwapTokenList::from(rw::TokenList {
            tokens: vec![],
            fresh: false,
        });
        assert!(empty_stale.tokens.is_empty());
        assert!(
            !empty_stale.fresh,
            "empty-and-stale stays stale — the freshness flag is independent of emptiness"
        );
    }

    /// A shielded mainnet recipient, valid by construction through the audited
    /// encoder (payload bytes arbitrary; the encoding/checksum is real). Shared
    /// by the two `encode_payment_uri` memo rows below — a memo needs a
    /// shielded receiver, so a transparent one would fail for the wrong reason.
    fn sapling_main() -> String {
        use zcash_address::ToAddress;
        use zcash_protocol::consensus::NetworkType;
        zcash_address::ZcashAddress::from_sapling(NetworkType::Main, [0xAB; 43]).encode()
    }

    fn draft_with_memos(
        memo_text: Option<String>,
        memo_bytes: Option<Vec<u8>>,
    ) -> api_payments::PaymentDraft {
        api_payments::PaymentDraft {
            recipient_address: sapling_main(),
            amount_zat: Some(100_000),
            memo_text,
            memo_bytes,
            label: None,
            message: None,
        }
    }

    #[test]
    fn encode_payment_uri_refuses_both_memos() {
        // FR-28: the pins live in zec-wallet-core and
        // call `Memo::arbitrary` / build a `PaymentRequest` literal, so NOTHING
        // reached the bridge function FR-28 actually changed. This is that
        // coverage. ONE memo per leg: both set is a caller bug, and the honest
        // answer is a typed refusal — silently preferring either one would put a
        // memo the caller did not choose on a permanent public ledger.
        let err = encode_payment_uri(
            vec![draft_with_memos(
                Some("dinner".into()),
                Some(vec![0x01, 0x02, 0x03]),
            )],
            api_config::Network::Main,
        )
        .expect_err("text + bytes on one leg is refused, never silently resolved");
        assert!(
            matches!(err.kind, api_error::WalletErrorKind::MemoConflict),
            "the refusal crosses as its OWN kind — a caller bug, never the \
             user-facing MemoInvalid (which means corrupt memo DATA)",
        );

        // Both single-memo polarities still encode — the refusal is about the
        // CONFLICT, not about either field. A build that refused every memo
        // would pass the row above and fail here.
        assert!(
            encode_payment_uri(
                vec![draft_with_memos(Some("dinner".into()), None)],
                api_config::Network::Main,
            )
            .is_ok(),
            "text alone still encodes",
        );
        assert!(
            encode_payment_uri(
                vec![draft_with_memos(None, Some(vec![0x01, 0x02, 0x03]))],
                api_config::Network::Main,
            )
            .is_ok(),
            "bytes alone still encode",
        );
    }

    #[test]
    fn machine_memo_scope_errors_cross_the_ffi_typed_and_payload_free() {
        // FR-27's two new kinds, at the boundary. A host branches on `kind` and
        // renders from `code`, never by matching text — so both must arrive
        // typed, and neither may carry host input (§5.4: a txid is never-log
        // material and memo bytes are content).
        let txid: api_error::WalletApiError = rw::WalletError::TxidInvalid.into();
        assert!(matches!(txid.kind, api_error::WalletErrorKind::TxidInvalid));
        assert_eq!(txid.code, "RW-PAY-009");

        let scope: api_error::WalletApiError = rw::WalletError::MachineMemoScopeInvalid {
            reason: "no machine-memo prefix registered",
        }
        .into();
        let api_error::WalletErrorKind::MachineMemoScopeInvalid { reason } = &scope.kind else {
            panic!("expected the machine-memo scope kind");
        };
        assert_eq!(reason, "no machine-memo prefix registered");
        assert_eq!(scope.code, "RW-PAY-010");
    }

    #[test]
    fn machine_memo_prefixes_lower_through_the_core_validating_constructor() {
        // The config door, at the crossing: an EMPTY prefix would match every
        // memo ever written, so a host registering one must learn at
        // create/open/validate — not by reading everything on the chain that a
        // stranger addressed to someone else.
        let mut cfg = config_with_db_dir("/tmp/zec-fr27");
        cfg.machine_memo_prefixes = vec![b"RLM\x01".to_vec()];
        assert!(
            validate_wallet_config(cfg).is_ok(),
            "a real prefix registers"
        );

        let mut empty_prefix = config_with_db_dir("/tmp/zec-fr27");
        empty_prefix.machine_memo_prefixes = vec![Vec::new()];
        assert!(
            matches!(
                validate_wallet_config(empty_prefix),
                Err(e) if matches!(e.kind, api_error::WalletErrorKind::MachineMemoScopeInvalid { .. })
            ),
            "an empty prefix is refused at the door, typed",
        );

        // …and the default (no scope at all) is a VALID config: FR-27 is opt-in,
        // and a host that never wants it must not be forced to configure one.
        assert!(validate_wallet_config(config_with_db_dir("/tmp/zec-fr27")).is_ok());

        // The COUNT bound runs in the PRE-FLIGHT validator too, not only at
        // create/open. It did not, and the FR entry claimed it did: a settings
        // form would have answered "this config is fine" for a scope that
        // `create` then refuses. A pre-flight door that disagrees with the real
        // one is worse than none (FR-27 review, two angles).
        let mut over_count = config_with_db_dir("/tmp/zec-fr27");
        over_count.machine_memo_prefixes = (0..=rw::constants::MACHINE_MEMO_PREFIX_MAX_COUNT)
            .map(|i| vec![i as u8])
            .collect();
        assert!(
            matches!(
                validate_wallet_config(over_count),
                Err(e) if matches!(e.kind, api_error::WalletErrorKind::MachineMemoScopeInvalid { .. })
            ),
            "the pre-flight validator enforces the same count bound create does",
        );
    }

    #[test]
    fn the_preflight_validator_refuses_a_jitter_above_the_ceiling() {
        // The 2026-10-06 follow-up review, N02: create/open refuse a jitter window above
        // `BROADCAST_JITTER_MAX_MS_CEILING`; the pre-flight validator must decide the same.
        let ceiling = u32::try_from(rw::constants::BROADCAST_JITTER_MAX_MS_CEILING).expect("fits");
        let mut at = config_with_db_dir("/tmp/zec-n02");
        at.broadcast_jitter = api_config::JitterPolicy::Uniform { max_ms: ceiling };
        assert!(
            validate_wallet_config(at).is_ok(),
            "exactly the ceiling is valid"
        );

        let mut over = config_with_db_dir("/tmp/zec-n02");
        over.broadcast_jitter = api_config::JitterPolicy::Uniform {
            max_ms: ceiling + 1,
        };
        let r = validate_wallet_config(over);
        assert!(
            matches!(
                &r,
                Err(e) if matches!(e.kind, api_error::WalletErrorKind::BroadcastJitterTooLong { .. })
            ),
            "the pre-flight validator must refuse what create/open refuse, got {:?}",
            r.as_ref().map_err(|e| e.code.clone())
        );
    }

    /// `WalletApiError` deliberately implements neither `Debug` nor `Display`
    /// (§5.4: an error must not become printable by accident), so the rows below
    /// cannot use `.expect()` on an `Ok`. Unwrap by pattern and report the
    /// payload-free stable `code` — never the kind's payload.
    fn expect_uri(r: Result<String, api_error::WalletApiError>, why: &str) -> String {
        match r {
            Ok(uri) => uri,
            Err(e) => panic!("{why} — refused as {}", e.code),
        }
    }

    fn expect_parsed(
        r: Result<Vec<api_payments::ParsedPayment>, api_error::WalletApiError>,
        why: &str,
    ) -> Vec<api_payments::ParsedPayment> {
        match r {
            Ok(legs) => legs,
            Err(e) => panic!("{why} — refused as {}", e.code),
        }
    }

    #[test]
    fn encode_payment_uri_drops_empty_machine_memo() {
        // FR-28 crypto audit fold, previously unpinned: `Memo::arbitrary`
        // accepts a ZERO-LENGTH vec (0 <= 511), which would write a full 511-byte
        // ALL-ZERO 0xFF memo on-chain — a fee-bearing no-op and a distinguishable
        // wallet fingerprint. `Some(vec![])` must therefore mean "no memo", the
        // same as `None`. The Dart layer drops empty to null too, but a host
        // calling `encodePaymentUri` with its own `PaymentDraft` — the crossing
        // this API exists for — reaches only this arm.
        let uri = expect_uri(
            encode_payment_uri(
                vec![draft_with_memos(None, Some(Vec::new()))],
                api_config::Network::Main,
            ),
            "empty bytes are nothing to attach, not an error",
        );

        // Read the outcome through the audited parser rather than by string
        // match on the URI: the assertion is about the MEMO, not the encoding.
        let parsed = expect_parsed(
            parse_payment_uri(&uri, api_config::Network::Main),
            "the encoder's own output parses",
        );
        assert_eq!(parsed.len(), 1);
        assert!(
            matches!(parsed[0].memo, api_payments::ParsedMemo::Empty),
            "empty bytes carry NO memo — never a 511-byte zero-filled Arbitrary",
        );

        // The contrasting polarity, so this row cannot pass by dropping every
        // machine memo: one non-empty byte IS attached.
        let uri = expect_uri(
            encode_payment_uri(
                vec![draft_with_memos(None, Some(vec![0x7A]))],
                api_config::Network::Main,
            ),
            "a one-byte machine memo encodes",
        );
        let parsed = expect_parsed(
            parse_payment_uri(&uri, api_config::Network::Main),
            "the encoder's own output parses",
        );
        assert!(
            matches!(parsed[0].memo, api_payments::ParsedMemo::Arbitrary { .. }),
            "a non-empty machine memo still rides the 0xFF arm",
        );
    }

    /// Stage S9 `bound`, assertion 1's bridge half (contract
    /// `docs/plan/stage-9-a-wedged-keystore-answers-in-time.md` §3.1 item 2):
    /// a key store that did not answer in time crosses as the EXISTING kind
    /// `keystoreUnavailable` — no new kind, the ABI stays 5 — and the code
    /// `RW-KEY-008` is what tells a host it was a wedge and not a lock. Every
    /// cause (`timeout`, `busy`, `past_deadline`) crosses the same way; the
    /// locked keychain keeps `RW-KEY-001`, so the two stay distinguishable.
    ///
    /// Names the core variant and its cause type, which the base tree does not
    /// carry (declared for the join: `rw::KeychainTimeoutCause`), so the row is
    /// never built until the adjudicator deletes the `#[cfg(any())]`.
    /// JOINED (adjudicator): the attribute is gone.
    #[test]
    fn a_keychain_timeout_crosses_as_keystore_unavailable_with_its_own_code() {
        for cause in [
            rw::KeychainTimeoutCause::Timeout,
            rw::KeychainTimeoutCause::Busy,
            rw::KeychainTimeoutCause::PastDeadline,
        ] {
            let crossed =
                api_error::WalletApiError::from(rw::WalletError::KeychainTimeout { cause });
            assert_eq!(crossed.code, "RW-KEY-008", "the timeout's own code crosses");
            assert!(
                matches!(
                    crossed.kind,
                    api_error::WalletErrorKind::KeystoreUnavailable
                ),
                "a timeout crosses as the existing keystoreUnavailable kind (code {})",
                crossed.code,
            );
        }
        let locked = api_error::WalletApiError::from(rw::WalletError::KeystoreUnavailable);
        assert_eq!(
            locked.code, "RW-KEY-001",
            "a locked keychain keeps its own code"
        );
        assert!(matches!(
            locked.kind,
            api_error::WalletErrorKind::KeystoreUnavailable
        ));
    }

    /// FR-48: `zec_wallet` exports the total-supply bound as a Dart `const`,
    /// which cannot cross the bridge, so this reads the Dart source and binds
    /// it to `MAX_MONEY_ZAT`. A host bounds a peer-claimed amount by the Dart
    /// value; if the two drifted, the host would accept an amount the core
    /// rejects, or refuse one it accepts. The barrel's export line is bound
    /// too, so the file read here is the one hosts import.
    #[test]
    fn the_dart_max_money_constant_equals_the_rust_one() {
        let lib = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../lib");
        let barrel =
            std::fs::read_to_string(lib.join("zec_wallet.dart")).expect("lib/zec_wallet.dart");
        let exports: Vec<&str> = barrel
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with("export ") && line.contains("maxMoneyZat"))
            .collect();
        assert_eq!(
            exports,
            ["export 'src/money.dart' show maxMoneyZat;"],
            "package:zec_wallet must export maxMoneyZat from lib/src/money.dart, the file this \
             test reads"
        );
        let dart = std::fs::read_to_string(lib.join("src/money.dart"))
            .expect("lib/src/money.dart is readable");
        let declared: Vec<&str> = dart
            .lines()
            .filter_map(|line| line.trim().strip_prefix("const int maxMoneyZat = "))
            .collect();
        assert_eq!(
            declared.len(),
            1,
            "lib/src/money.dart must declare `const int maxMoneyZat = N;` exactly once \
             (found {declared:?})"
        );
        let dart_value: i64 = declared[0]
            .trim_end_matches(';')
            .trim()
            .parse()
            .expect("maxMoneyZat is an integer literal");
        assert_eq!(
            dart_value,
            rw::constants::MAX_MONEY_ZAT,
            "the Dart maxMoneyZat differs from the Rust MAX_MONEY_ZAT: move them together"
        );
    }

    /// The process device log's test lock, for a sync test (outside any
    /// runtime): a sibling that opened the gate between this test's wipe and
    /// its assertion would read as a wipe that left the log on.
    fn process_log_lock() -> tokio::sync::MutexGuard<'static, ()> {
        crate::device_log::test_sink::PROCESS_LOG.blocking_lock()
    }

    /// A wipe driven to completion on its own runtime, so the process lock is
    /// never held across an `.await`.
    fn wipe_now(db_dir: &str, force: bool) -> Result<(), api_error::WalletApiError> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a current-thread runtime")
            .block_on(wipe_wallet(config_with_db_dir(db_dir), force))
    }

    /// A host subscription on the PROCESS slot: the `seq` of every line it
    /// received, and whether the slot has dropped it — which is what closes a
    /// Dart stream.
    #[derive(Clone, Default)]
    struct ProcessTap {
        seqs: std::sync::Arc<std::sync::Mutex<Vec<u64>>>,
        closed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }

    impl ProcessTap {
        fn watch() -> Self {
            let tap = Self::default();
            crate::device_log::process_host_slot().watch(Box::new(ProcessTapSink(tap.clone())));
            tap
        }

        fn seqs(&self) -> Vec<u64> {
            self.seqs
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone()
        }

        fn is_closed(&self) -> bool {
            self.closed.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    struct ProcessTapSink(ProcessTap);

    impl crate::device_log::HostLineSink for ProcessTapSink {
        fn send(&self, line: crate::api::meta::DeviceLogLine) -> bool {
            self.0
                .seqs
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(line.seq);
            true
        }
    }

    impl Drop for ProcessTapSink {
        fn drop(&mut self) {
            self.0
                .closed
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    /// `n` INFO lines through the PROCESS gate and slot, as the installed
    /// layer carries them.
    fn through_the_process_log(n: u32) {
        use crate::device_log::test_sink::MemorySink;
        use crate::device_log::{FanOut, Scope, device_log_layer, process_gate, process_host_slot};
        use tracing_subscriber::layer::SubscriberExt;
        let subscriber = tracing_subscriber::registry().with(device_log_layer(
            FanOut {
                platform: MemorySink::default(),
                platform_scope: Scope::AllSdkTargets,
                host: process_host_slot().clone(),
            },
            process_gate().clone(),
        ));
        tracing::subscriber::with_default(subscriber, || {
            for count in 0..n {
                tracing::info!(target: "zec_wallet_core", count, outcome = "ok", "wallet.sync");
            }
        });
    }

    /// S5 §3.1, the duress seam: a wipe closes the device log. A host that had
    /// turned the log on and subscribed finds, after either wipe verb, the log
    /// `Off`, its stream closed, and — once it re-arms — a stream whose `seq`
    /// starts again from 0. That holds whatever the wipe itself answered: on an
    /// empty temporary `dbDir` it is a no-op success or a keychain refusal. The
    /// SDK cannot tell a duress wipe from a user's Delete, so every wipe does it.
    ///
    /// Watched against: the `quiesce_for_sever` call removed from `wipe_wallet`.
    #[test]
    fn wipe_quiesces_the_log_first() {
        use crate::api::meta::DeviceLogLevel::{Detailed, Off};
        let _serial = process_log_lock();
        let gate = crate::device_log::process_gate();
        for force in [false, true] {
            let dir = tempfile::tempdir().expect("tempdir");
            let db_dir = dir.path().to_str().expect("a UTF-8 temp path");
            let tap = ProcessTap::watch();
            gate.set(Detailed);
            through_the_process_log(2);
            assert_eq!(tap.seqs().len(), 2, "the control: the stream was live");
            // Ok or a typed refusal: the log's state is the assertion, not this.
            let _ = wipe_now(db_dir, force);
            assert_eq!(
                gate.level(),
                Off,
                "a wipe (force = {force}) must leave the device log Off until the host re-arms"
            );
            assert!(
                tap.is_closed(),
                "a wipe (force = {force}) closes the host stream"
            );

            let rearmed = ProcessTap::watch();
            gate.set(Detailed);
            through_the_process_log(1);
            assert_eq!(
                rearmed.seqs(),
                [0],
                "a stream after a wipe carries no count from before it"
            );
            crate::device_log::quiesce_for_sever();
        }
    }

    /// S5 §3.1: the quiesce is the FIRST statement of `wipe_wallet`, ahead of
    /// the `dbDir` shape gate — fail-closed. A duress wipe whose `dbDir` is
    /// malformed must still not log, so a wipe the door refuses (a relative
    /// path, an empty one) closes the log exactly as a wipe that ran does.
    ///
    /// Watched against: the call moved after `validate_db_dir`.
    #[test]
    fn an_invalid_db_dir_still_quiesces() {
        use crate::api::meta::DeviceLogLevel::{Errors, Off};
        let _serial = process_log_lock();
        let gate = crate::device_log::process_gate();
        for (db_dir, force) in [("relative/shred", false), ("", true)] {
            let tap = ProcessTap::watch();
            gate.set(Errors);
            let code = match wipe_now(db_dir, force) {
                Ok(()) => panic!("the door must refuse the dbDir {db_dir:?}"),
                Err(e) => e.code,
            };
            assert_eq!(code, "RW-CFG-003", "the refusal is the dbDir door's");
            assert_eq!(
                gate.level(),
                Off,
                "a wipe the door refused ({db_dir:?}, force = {force}) must still close the log"
            );
            assert!(
                tap.is_closed(),
                "a wipe the door refused ({db_dir:?}) still closes the host stream"
            );
        }
    }
}
