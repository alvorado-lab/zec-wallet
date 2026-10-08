//! Bridge SELF-TEST surface — NOT wallet API.
//!
//! Why this exists: the bridge's job is moving every DTO shape across FFI
//! intact, and that is only provable ON the bridge (the example app's
//! integration tests drive these from Dart — spec §8's W3 E2E harness; a
//! static review cannot catch a type that dies in codegen or transport).
//! These functions also serve as the codegen ROOTS for the outbound DTO
//! tree: FRB generates only types reachable from api functions (verified
//! against 2.12), so without them the §2.5/§2.6 Dart surface would not
//! exist yet.
//!
//! Properties:
//! - touches NO wallet, NO storage, NO network — pure constants;
//! - sample values are built as CORE types and pushed through the real
//!   `convert` layer, so the conversion path itself is what's exercised;
//! - stable enough to script against, but documented as diagnostic surface:
//!   hosts must not ship UX built on `selftest*` calls.

use zec_wallet_core as rw;

use crate::api::error::{SwapApiError, WalletApiError};
use crate::api::state::{SyncStatus, TorState, TxSubmitResult, TxSummary, WalletState};
use crate::api::swap::{QuoteRequest, SwapQuote, SwapRecord, SwapStatus};

/// Everything outbound, one call: a full DTO tree built Rust-side and
/// crossed in a single hop. Dart asserts shape + values; the integration
/// test is "did every field survive".
pub struct SelftestBundle {
    /// A populated snapshot (balance / scanning sync / active-Tor state).
    pub wallet_state: WalletState,
    /// Every sync-status arm this binding knows, including `unknown`.
    pub sync_statuses: Vec<SyncStatus>,
    /// Every Tor-state arm.
    pub tor_states: Vec<TorState>,
    /// History rows covering every REAL tx-status arm (the `Unknown`
    /// arm's bridge crossing is proven by the status vectors above).
    pub tx_rows: Vec<TxSummary>,
    /// Every per-tx submit outcome, including the `unknown` arm.
    pub submit_results: Vec<TxSubmitResult>,
    /// A quote with a fully-populated privacy disclosure.
    pub quote: SwapQuote,
    /// Every swap-status arm.
    pub swap_statuses: Vec<SwapStatus>,
    /// A swap history row.
    pub swap_record: SwapRecord,
}

/// Build the bundle. Pure constants; safe to call anywhere, any time.
#[flutter_rust_bridge::frb(sync)]
pub fn selftest_bundle() -> SelftestBundle {
    SelftestBundle {
        wallet_state: sample_wallet_state().into(),
        sync_statuses: sample_sync_statuses()
            .into_iter()
            .map(Into::into)
            .chain([SyncStatus::Unknown])
            .collect(),
        tor_states: sample_tor_states()
            .into_iter()
            .map(Into::into)
            .chain([TorState::Unknown])
            .collect(),
        tx_rows: sample_tx_rows().into_iter().map(Into::into).collect(),
        submit_results: sample_submit_results()
            .into_iter()
            .map(Into::into)
            .chain([TxSubmitResult::Unknown])
            .collect(),
        quote: sample_quote().into(),
        swap_statuses: sample_swap_statuses()
            .into_iter()
            .map(Into::into)
            .chain([SwapStatus::Unknown])
            .collect(),
        swap_record: sample_swap_record().into(),
    }
}

/// Always throws `RW-LIFE-001` (`walletAlreadyOpen`) — Dart asserts
/// catch-BY-TYPE on [WalletApiError] and reads the stable code.
#[flutter_rust_bridge::frb(sync)]
pub fn selftest_throw_wallet_error() -> Result<(), WalletApiError> {
    Err(rw::WalletError::WalletAlreadyOpen.into())
}

/// Always throws `RW-SWAP-003` (`quoteExpired`) — same contract for
/// [SwapApiError].
#[flutter_rust_bridge::frb(sync)]
pub fn selftest_throw_swap_error() -> Result<(), SwapApiError> {
    Err(rw::SwapError::QuoteExpired.into())
}

/// Inbound crossing: a [QuoteRequest] built in Dart runs through the REAL
/// bridge→core conversion (including the zatoshi bounds check) and reports
/// the typed outcome. Proves Dart→Rust DTO integrity without touching any
/// provider.
#[flutter_rust_bridge::frb(sync)]
pub fn selftest_quote_request_roundtrip(request: QuoteRequest) -> Result<(), SwapApiError> {
    rw::QuoteRequest::try_from(request)
        .map(|_| ())
        .map_err(Into::into)
}

// ─── core-side samples (kept private; the api surface above is the contract)

/// In-range constants — the constructors cannot fail on them.
fn zat(v: i64) -> rw::Zatoshis {
    rw::Zatoshis::new(v).expect("selftest constant within money bounds")
}

fn sample_wallet_state() -> rw::WalletState {
    rw::WalletState {
        balance: rw::BalanceSnapshot {
            spendable: zat(123_450_000),
            pending_incoming: zat(5_000_000),
            pending_change: zat(250_000),
            transparent: zat(100_000),
            total: zat(128_800_000),
        },
        sync: rw::SyncStatus::Scanning {
            from: rw::BlockHeight::new(2_400_000),
            to: rw::BlockHeight::new(2_500_000),
            percent: 42.5,
            // The two adjacent bools DIFFER (false, true) so the Dart parity
            // assert catches a transposed pair, not just a dropped/defaulted
            // field — same-valued neighbours are invariant under a swap.
            spendable_ready: false,
            rewound: true,
        },
        tor: rw::TorState::Active {
            runtime: rw::TorRuntimeKind::ExternalSocks5,
        },
        tip: Some(rw::BlockHeight::new(2_500_000)),
        last_synced: Some(rw::SyncStamp {
            height: rw::BlockHeight::new(2_499_900),
            at: 1_780_000_000,
        }),
        // The two adjacent bools DIFFER (true, false) — same transposed-pair
        // rationale as the Scanning pair above.
        ever_synced: true,
        rescan_rebuilding: false,
        seq: 7,
    }
}

fn sample_sync_statuses() -> Vec<rw::SyncStatus> {
    vec![
        rw::SyncStatus::Idle,
        rw::SyncStatus::Connecting {
            tor_bootstrap_percent: Some(80.0),
        },
        rw::SyncStatus::Scanning {
            from: rw::BlockHeight::new(1),
            to: rw::BlockHeight::new(100),
            percent: 1.0,
            spendable_ready: false,
            rewound: false,
        },
        rw::SyncStatus::UpToDate {
            tip: rw::BlockHeight::new(2_500_000),
        },
        // Was never added when the arm landed (`ironwood-nu63-support.md` §6.4) —
        // folded in with T0-1b so the Dart-side arm count stops under-reporting.
        rw::SyncStatus::UpToDateLimited {
            tip: rw::BlockHeight::new(2_500_000),
        },
        // T0-1b: DISTINCT per-pool values so the Dart parity check catches a
        // transposed pool, not only a dropped one.
        rw::SyncStatus::UpToDateDegraded {
            tip: rw::BlockHeight::new(2_500_000),
            pools: rw::PoolServiceReport {
                sapling: rw::PoolService::Served { roots: 17 },
                orchard: rw::PoolService::Withheld { proven: 3 },
                ironwood: rw::PoolService::Unsupported,
            },
        },
        // T0-1c: the behind arm with DISTINCT heights and a carried pools report,
        // so the Dart parity check catches a transposed height or a dropped report.
        rw::SyncStatus::EndpointBehind {
            tip: rw::BlockHeight::new(2_400_000),
            newest_known: rw::BlockHeight::new(2_500_000),
            pools: Some(rw::PoolServiceReport {
                sapling: rw::PoolService::Served { roots: 17 },
                orchard: rw::PoolService::Served { roots: 9 },
                ironwood: rw::PoolService::Withheld { proven: 1 },
            }),
        },
        // GRACE-1: the unverified arm with a RUNNING grace whose two figures are
        // distinct and a carried pools report, so the Dart parity check catches
        // a transposed figure or a dropped report. (The ended shape rides the
        // same `UnknownBranchGrace` conversion.)
        rw::SyncStatus::UpToDateUnverified {
            tip: rw::BlockHeight::new(2_500_000),
            grace: rw::UnknownBranchGrace::Running {
                blocks_left: 1_000,
                secs_left: Some(6_400),
            },
            pools: Some(rw::PoolServiceReport {
                sapling: rw::PoolService::Served { roots: 17 },
                orchard: rw::PoolService::Served { roots: 9 },
                ironwood: rw::PoolService::Served { roots: 0 },
            }),
            // P3-12: the carried streak report, `true` so the sample is the
            // non-default shape. A DROPPED field is caught by codegen and
            // `wallet-bridge-verify` (the generated Dart factory requires it),
            // not by a parity assertion — `bridge_selftest.dart` reads only the
            // arm count (review angles, LOW: the earlier comment here
            // claimed more than any assertion enforces).
            streak_reported: true,
        },
        rw::SyncStatus::Stalled {
            reason: rw::StallReason::TorUnavailable,
        },
        rw::SyncStatus::Stalled {
            reason: rw::StallReason::EndpointUnreachable,
        },
        rw::SyncStatus::Stalled {
            reason: rw::StallReason::StorageFull,
        },
        rw::SyncStatus::Stalled {
            reason: rw::StallReason::ChainReorg,
        },
        rw::SyncStatus::Stalled {
            reason: rw::StallReason::Internal,
        },
        rw::SyncStatus::Stalled {
            reason: rw::StallReason::EndpointMisbehaving,
        },
        rw::SyncStatus::Stalled {
            reason: rw::StallReason::BirthdayInFuture,
        },
        rw::SyncStatus::Stalled {
            reason: rw::StallReason::StorageUnavailable,
        },
        rw::SyncStatus::Offline { last_synced: None },
    ]
}

fn sample_tor_states() -> Vec<rw::TorState> {
    vec![
        rw::TorState::Off,
        rw::TorState::Bootstrapping {
            percent: Some(25.0),
            transport: None,
        },
        rw::TorState::Bootstrapping {
            percent: Some(40.0),
            transport: rw::HostTransportName::new(b"Shadowsocks"),
        },
        rw::TorState::Active {
            runtime: rw::TorRuntimeKind::ExternalSocks5,
        },
        rw::TorState::Active {
            runtime: rw::TorRuntimeKind::Dialer,
        },
        rw::TorState::FellBack,
        rw::TorState::Unavailable { transport: None },
        rw::TorState::Unavailable {
            transport: rw::HostTransportName::new(b"Tor"),
        },
        rw::TorState::Unanswered {
            runtime: rw::TorRuntimeKind::Dialer,
        },
    ]
}

fn sample_txid(fill: u8) -> rw::TxId {
    rw::TxId::from_display_order([fill; 32])
}

fn sample_tx_rows() -> Vec<rw::TxSummary> {
    let statuses = [
        rw::TxStatus::Queued,
        rw::TxStatus::Pending,
        rw::TxStatus::Confirmed { depth: 10 },
        rw::TxStatus::Expired,
        rw::TxStatus::Failed,
    ];
    statuses
        .into_iter()
        .enumerate()
        .map(|(i, status)| rw::TxSummary {
            txid: sample_txid(i as u8),
            batch_id: Some(rw::BatchId::new(1)),
            mined_height: Some(rw::BlockHeight::new(2_400_001)),
            status,
            net_amount: rw::ZatBalance::new(-1_500_000)
                .expect("selftest constant within balance bounds"),
            fee: Some(zat(10_000)),
            has_memo: i % 2 == 0,
            // OPPOSITE parity from has_memo so a transposed-field bridge bug can't
            // cancel out (row 0: memo=true/transparent=false; row 1 the reverse).
            has_transparent_output: i % 2 == 1,
            timestamp: Some(1_780_000_000),
            // The delivery reading (stage S8 `obligation`) crosses on the bridge's
            // `TxSummary` from the fold's regen on; the selftest's rows carry
            // none until then.
            delivery: None,
            // The outcome-resolution height (stage S2 `outcome`) likewise: none.
            expiry_height: None,
        })
        .collect()
}

fn sample_submit_results() -> Vec<rw::TxSubmitResult> {
    vec![
        rw::TxSubmitResult::Success {
            txid: sample_txid(0xaa),
        },
        rw::TxSubmitResult::GrpcFailure {
            txid: sample_txid(0xbb),
        },
        rw::TxSubmitResult::SubmitFailure {
            txid: sample_txid(0xcc),
            code: -25,
        },
        rw::TxSubmitResult::NotAttempted {
            txid: sample_txid(0xdd),
        },
    ]
}

fn sample_quote() -> rw::SwapQuote {
    rw::SwapQuote {
        id: rw::SwapId::new("selftest-quote-1"),
        binding: None,
        deposit_address: "t1SelftestDepositAddressDoNotUse00000".to_string(),
        deposit_memo: None,
        expires_at: 1_780_086_400,
        amount_in: "100.25".to_string(),
        min_amount_out: "0.98".to_string(),
        zec_side: zat(98_000_000),
        refund_to: Some("t1SelftestRefundAddressDoNotUse000000".to_string()),
        disclosure: rw::SwapPrivacyDisclosure {
            ends_shielded: false,
            deshields: true,
            provider_legs_transparent: true,
            provider_sees: vec![
                rw::DisclosureItem::CrossAssetLink,
                rw::DisclosureItem::Amounts,
                rw::DisclosureItem::DestinationAddress,
                rw::DisclosureItem::SourceAddress,
                rw::DisclosureItem::IpUnlessTor,
            ],
        },
    }
}

fn sample_swap_statuses() -> Vec<rw::SwapStatus> {
    vec![
        rw::SwapStatus::PendingDeposit {
            expires_at: 1_780_086_400,
        },
        rw::SwapStatus::UnderDeposited {
            received: "50.0".to_string(),
            missing: "50.25".to_string(),
            deadline: 1_780_086_400,
        },
        rw::SwapStatus::DepositDetected,
        rw::SwapStatus::Processing,
        rw::SwapStatus::Success {
            out_txid: Some("0xselftest".to_string()),
            realized_slippage_bps: Some(12),
        },
        rw::SwapStatus::Refunded { refund_txid: None },
        rw::SwapStatus::Failed {
            code: rw::SwapFailureCode::Expired,
        },
    ]
}

fn sample_swap_record() -> rw::InFlightSwap {
    // The W-swap-5 durable home row — the honest minimum the store actually
    // persists (no status snapshot, no txid; coarse direction only; the #367
    // outcome pin unset, as at execute).
    rw::InFlightSwap {
        swap_id: "selftest-record-1".to_string(),
        out_of_zec: true,
        created_at: 1_780_000_000,
        deposit_deadline: Some(1_780_000_900),
        expires_at_wall: 1_780_172_800,
        outcome: None,
    }
}

/// §4.3a device-E2E custody report (the FRB face of
/// `zec_wallet_core::CustodySelftestReport`). Non-secret by construction:
/// booleans + the measured tier label.
pub struct CustodyReport {
    /// Measured vault tier: "strongbox" | "tee" | "software_keystore" |
    /// "apple_keychain" | "apple_secure_enclave" | "unknown" (forward arm — a
    /// newer core may add tiers; the LABEL is diagnostic, the `degraded` bool is
    /// the load-bearing predicate). `apple_secure_enclave` is the FR-14 chunk 2
    /// hardware-key tier; `apple_keychain` is the best-effort raw fallback.
    pub tier: String,
    /// §4.3a degraded-custody predicate, surfaced as data.
    pub degraded: bool,
    pub roundtrip_ok: bool,
    pub binding_rejects_tampered_blob: bool,
    pub wipe_severs: bool,
}

/// Run the §4.3a custody round-trip against THIS device's real vault
/// (selftest-scoped identities; production custody untouched). Async —
/// real Keystore/keychain I/O rides the FRB worker pool, not the Dart
/// thread. Diagnostic surface like everything in this module: hosts must
/// not ship UX on it — and must NOT call it concurrently (the selftest
/// alias scan races; one caller reports a false FAIL; see the core fn).
pub async fn selftest_seed_custody() -> Result<CustodyReport, WalletApiError> {
    let report = rw::seed_custody_selftest().map_err(WalletApiError::from)?;
    // Shared with the PRODUCTION `custody_disclosure` (DRY — one tier→label map,
    // so the diagnostic and shippable surfaces can never drift).
    let tier = crate::convert::vault_tier_label(report.tier).to_string();
    Ok(CustodyReport {
        tier,
        degraded: report.degraded,
        roundtrip_ok: report.roundtrip_ok,
        binding_rejects_tampered_blob: report.binding_rejects_tampered_blob,
        wipe_severs: report.wipe_severs,
    })
}
