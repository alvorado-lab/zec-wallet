//! # zec-wallet-core — extraction-ready ZEC wallet SDK core
//!
//! The Rust engine of the Flutter ZEC Wallet SDK (`docs/specs/wallet-sdk.md`;
//! ADR-0005 librustzcash-direct, ADR-0013 extraction-ready SDK, ADR-0014
//! SwapPort). Headline security property: **seed and spending keys never
//! leave Rust** — consumers see addresses, balances, history, sync status,
//! and swap state only.
//!
//! Two consumers, one crate (spec §3.4): a third-party Flutter app via the
//! `zec_wallet` Dart package (W3, FRB bridge crate), and a Rust host via
//! direct consumption (`zec-wallet-core-host`, W5). **No `relim-*` dependencies,
//! ever** — every dep is publicly resolvable (crates.io); enforced by the
//! `core_and_bridge_have_no_relim_deps` named test.
//!
//! Network access is pluggable by construction (spec §3.2): all traffic —
//! sync gRPC and swap REST alike — rides [`NetDialer`]/[`TorPolicy`], so the
//! SDK reuses a host app's transport infrastructure (a registered dialer, a
//! host-provided Tor transport). This crate compiles NO Tor of its own: a
//! host without a transport adds the optional plugin `zec_wallet_tor`, a
//! registrant of the same dialer contract (ADR-0548; `docs/specs/tor-plugin.md`).
//!
//! Feature gates: `swap` (default ON — kill layer 1, §3.5: off ⇒ the swap
//! surface is not compiled). There is no Tor feature (`tor-builtin` was
//! removed at FR-5 C1).

#![forbid(unsafe_code)]

pub mod constants;

mod account;
mod block_cache;
mod checkpoints;
mod config;
// §3.2 of ironwood-nu63-support.md — the ONE staleness predicate. Its own module
// because it is cross-cutting (provisioning, the sync pass and every signing
// door consult it) and a second implementation of "are we stale" is a bug.
mod consensus;
// The persisted verdict + grace anchor (§6.2 owes the host "the last verdict and
// its age" while offline; a purely in-memory one is absent on every cold start).
mod consensus_stamp;
mod creation_stamp;
// Stage S2 `custody` (R04): the wallet's identity — a random 128-bit CustodyId
// minted at create, carried as a plaintext locator in the wrap artifact's
// header, from which the keychain namespace is derived; the path-keyed index
// item; the one-commit-point migration. A relocated container opens the SAME
// custody (ADR-0559 reverses "a moved db_dir is a new wallet").
mod custody;
mod db;
// Stage S8 `obligation` (R02): the delivery obligation — a persisted signed
// transaction IS the wallet's obligation to broadcast it. The enumeration the
// resubmission pass and the rescan fence read, the per-transaction delivery
// state, and the best-effort accepted mark. Pure SQL over the aux connection.
mod delivery;
mod derivation;
// FR-8 (#393, ADR-0537): the never-recycle diversifier-index allocator for PUBLIC
// diversified receive UAs — the refund_index sibling in the disjoint high region.
mod diversified_index;
// §3.3 tx-enhancement (memo recovery): the §4.6 hostile-input boundary (re-parse +
// txid-match an endpoint's full-tx reply) + the wire-height sentinel mapping. The audited
// decryptor + the bounded drain loop live in `wallet`, the fetch port in `sync`.
mod enhance;
// §3.2i-2 2e-2b — ephemeral-transparent DETECT (TEX/ZIP-320 stranded + exchange-return poll):
// the bounded-lookback scope predicate, the per-ephemeral isolated-circuit COUNT+SKIP loop, the
// detect isolation key, and the §5.4 COUNTS-only summary. The `Inner`-touching enumerate / probe /
// recognise parts live in `wallet`; the recognition put in `transparent`.
mod ephemeral_detect;
mod error;
// #357: the durable "reached tip at least once" flag — the `creation_stamp` /
// `sync_stamp` aux sibling that backs the reference UI's catch-up cue across a
// process death (spec FR-1b). Row cleared on rescan (the `sync_stamp`
// precedent — the cue re-shows during a rebuild), dies with the identity.
mod ever_synced;
mod history;
pub use history::{HistoryPage, TransactionMemos};
mod intent_store;
// T0-1 A6/A6b + the S3/S2 predicate rows: the spendability drive for an Ironwood
// note above a subtree the wallet never scanned. Test-only at BOTH sites — the
// attribute here like its three siblings, and `#![cfg(test)]` inside — so it
// compiles to nothing in a library build. The name is the one `sync.rs`'s A6
// comment pointed at for three sessions before the module existed.
#[cfg(test)]
mod ironwood_spendability;
mod issued_quote_store;
mod sync_stamp;
// §4.3a custody. The custody PATH is live: the wallet handle (wallet.rs)
// resolves the platform vault and store.rs drives `SealedSeedVault` store/load.
// No module allow (P3-7): the items still staged behind their own consumers —
// the wrap-KEY ROTATION methods and the Android-only wrap-artifact codec — carry
// their own attributes with the reason, so a NEW dead item in here is a warning.
pub(crate) mod keychain;
mod lifecycle;
mod memo;
mod money;
mod net;
mod parked;
mod payment_uri;
mod ports;
mod prover;
mod provision;
mod reclaim;
mod refund_index;
// T0-1a: the A11 subtree-completion-height bind — what the wallet knows about
// subtree completion heights that the endpoint cannot forge (its own recorded
// shard rows, the bundled treestate frontiers, and its own scanned tree sizes),
// and how a served sequence is held to it before it can reach the witness tree.
mod root_bind;
// #377 s357b-2: the durable "rescan rebuild in progress" breadcrumb — set at
// the rescan swap, cleared at the first post-rescan reached-tip; lets the
// catch-up cue name the rebuild after a process death instead of falling back
// to the generic first-run copy.
mod rescan_rebuilding;
mod runtime;
mod seal;
mod seed;
// #357: the domain-separated SHA-256 seed fingerprint — at-rest verification of a
// host-supplied seed for `SeedPersistence::None` wallets (the §3.2f caveat close).
mod seed_fingerprint;
// FR-53 (stage S16): the duress force-sever's report — values only, no key,
// path or namespace. The verb is `Wallet::sever_custody`.
mod sever;
// §3.2h send pipeline — inc-2d-1 PROPOSE landed (deterministic note-selection + fee +
// change → `SendProposal`, the opaque one-shot token registry). The public `Wallet::send`
// / `queue_send` (which consume the registry) land at inc-2d-2/3, so the `Retained`
// `request`/`consume` seam is exercised by tests until then.
mod send;
mod state;
mod store;
mod stranded;
#[cfg(feature = "swap")]
mod swap;
mod sweep;
// The durable SCOPED swap-destination detection set (§3.3b D2/L2, ADR-0530, IZ-1b) — the
// sibling aux-db store the transparent poll unions with the index-0 receiver.
// UNGATED since W-swap-5 (#366, ADR-0534): the SCHEMA must exist in every build so a
// non-swap build's rescan copies a swap-build's in-flight rows instead of silently
// dropping them — only the DETECT/service BEHAVIOR stays `cfg(feature = "swap")`
// (the non-swap allowance covers the poll-only functions with no caller there).
#[cfg_attr(not(feature = "swap"), allow(dead_code))]
mod swap_destination_store;
// The durable in-flight swap RECORD (W-swap-5, #366) — the user-visibility home row
// listed after process death / re-entry / session swaps. UNGATED end-to-end: local
// reads are not swap traffic (§3.5), so listing/dismissing works in every build;
// only the WRITE path (the SwapService execute leg) is feature-gated.
#[cfg(test)]
mod degraded_pool_proof;
// (§4t SCAN-2): the derived-anchor proof — the test half's rows S2-1..S2-5,
// the mainnet vector row and its `#[ignore]`d capture tool (read its module doc).
#[cfg(test)]
mod anchor_proof;
// T0-4 (§4y STALE-1): the staleness VECTOR — a real mined Ironwood transaction,
// and the `#[ignore]`d tool that captured it (read its module doc).
#[cfg(test)]
mod staleness_proof;
// T0-2 (§4z): the arrival READBACK — the `#[ignore]`d tool that re-reads a
// device-proof arrival from a mainnet lightwalletd and parses its Ironwood
// bundle here, so the proof does not rest on the wallet that detected it (read
// its module doc).
#[cfg(test)]
mod arrival_proof;
// the live scan-throughput probe (an `#[ignore]`d test; read its module doc).
#[cfg(test)]
mod live_scan_probe;
#[cfg_attr(not(feature = "swap"), allow(dead_code))]
mod swap_record_store;
mod sync;
#[cfg(test)]
mod sync_bind_proof;
// P3-13 — the sync-server picker's types, reference catalog, aux row and the
// pure choice resolution (`docs/specs/sync-server-picker.md`).
mod sync_server;
// §3.2g iv-d-3. The SyncController orchestration core (loop + retry/backoff +
// stuck-sync watchdog + live SyncStatus stream + the first tracing site) is wired
// into the handle by the real `LightdSyncEngine` (d-3-b-ii, `wallet.rs`) — `start` /
// `stop` drive it now, and `subscribe` / `status` are on the bridge. `once` (a host
// pull-to-refresh) has no production caller and is `#[cfg(test)]` (P3-7); no module allow.
mod sync_controller;
// §3.2g iv-d-3-b-iii-B-2-b: the live `TorState` derivation (policy + dialer
// fell-back flag + current SyncStatus → the cold-queryable TorState). Consumed by
// `Inner::tor_state` (wallet.rs) and assembled into `WalletState::tor` by the cold
// `snapshot()` (B-2-c) and read by the public FRB surface — every item reached, no
// allow (P3-7).
mod tor_status;
// §5.4 tracing capture-guard: the shared test harness + the ONE source of truth
// for the never-log list + field allowlist. Consumed by the controller's
// event-surface guard (`sync_controller`) AND the engine pass's per-range
// `wallet.sync` span guard (`wallet`). The capture harness inside it is
// `#[cfg(test)]` item by item; the two lists and the two predicates SHIP, since
// because the bridge's host-switched device-log layer enforces them at
// runtime (`log_policy` below).
mod tracing_guard;
/// The §5.4 log policy, for a log sink that enforces it AT RUNTIME.
///
/// Two predicates and nothing else — the lists stay private to
/// `tracing_guard`, where every capture test reads them, so a sink cannot
/// consult a copy: [`is_sdk_target`](log_policy::is_sdk_target) says whether a
/// `tracing` target is this SDK's (and therefore graded by the policy at all),
/// and [`field_is_loggable`](log_policy::field_is_loggable) says whether one
/// field may be written. The bridge's device-log layer
/// (`sdk/zec_wallet/rust/src/device_log.rs`) prints an event only when the
/// first is true and withholds every field for which the second is false.
pub mod log_policy {
    pub use crate::tracing_guard::{field_is_loggable, is_sdk_target};
}
// Shared `#[cfg(test)]` funded-wallet harness (file-backed `WalletDb` + a second aux
// connection over real scanned `v_transactions`): the deep-e2e history/reorg/concurrency
// tests that a hand-built fixture can't reach. No production weight.
#[cfg(test)]
mod test_support;
// Recv-2b transparent UTXO detection (§3.3a, ADR-0528): the §4.6 hostile-input
// boundary that turns a `GetAddressUtxos` reply into audited
// `WalletTransparentOutput`s and puts them via the engine, surfacing
// `BalanceSnapshot.transparent`. Compact-block scan is shielded-only, so this poll
// is the ONLY path that detects a transparent receive.
mod transparent;
mod wallet;

pub use checkpoints::estimate_birthday;
// `ironwood-nu63-support.md` §3.3 — the cold read, so a host can render an
// update prompt without attempting a send.
pub use config::{
    EndpointAuth, JitterPolicy, LightServerEndpoint, TorPolicy, TorRuntime, WalletConfig,
};
// P3-13 — the picker's public shape (`sync-server-picker.md` §2).
pub use consensus::{ConsensusCompatibility, ConsensusStatusSnapshot, GraceClock};
pub use sync_server::{
    REFERENCE_SYNC_SERVERS_MAINNET, REFERENCE_SYNC_SERVERS_TESTNET, ReferenceSyncServer,
    SyncServer, SyncServerCatalog, SyncServerChoice, SyncServerFallback, SyncServerId,
    SyncServerProbe, SyncServerStatus, validate_sync_servers,
};
pub use wallet::SwitchRefused;
// §4.3a context handoff — called once by whichever host loads the library
// (the zec_wallet plugin courier; the host FFI crate at W5). The ONLY jni type in the
// public API, android-only by construction.
#[cfg(feature = "swap")]
pub use error::{
    DestinationInvalidReason, ProviderProtocolReason, QuoteBoundSide, SwapAddressCheckRefusal,
    SwapError,
};
pub use error::{DialError, KeychainTimeoutCause, WalletError};
// FR-8 / Recv-4 (ADR-0537): the public diversified-receive mint result.
pub use diversified_index::MintedDiversifiedAddress;
#[cfg(target_os = "android")]
pub use keychain::android::init_android_vault;
// §4.3a device-E2E custody selftest + the measured-tier enum it reports
// (non-secret; selftest-scoped vault identities — see keychain/selftest.rs).
pub use keychain::VaultTier;
// The PRODUCTION custody disclosure (non-secret; the shippable per-tier honesty
// query that drives the `wipe` confirmation + an ambient custody badge —
// distinct from the diagnostic `seed_custody_selftest`).
pub use keychain::selftest::{CustodySelftestReport, seed_custody_selftest};
pub use keychain::{CustodyDisclosure, EraseAssurance};
// FR-53 (stage S16): the key-store budget of a wipe, which is also the most a
// duress sever spends — the bridge clamps a host's deadline to it.
pub use keychain::bounded::KEYCHAIN_WIPE_BUDGET;
pub use lifecycle::LifecyclePhase;
pub use memo::{
    Address, AddressKind, MachineMemoPrefix, Memo, Payment, PaymentRequest,
    validate_machine_memo_scope,
};
pub use money::{BlockHeight, Network, TxId, ZatBalance, Zatoshis};
pub use payment_uri::{encode_payment_uri, parse_payment_uri};
pub use ports::{AsyncByteStream, NetDialer};
pub use sever::{
    FilesOutcome, HolderSeen, NotSeveredCause, SeverOutcome, SeverReport, UnprovenReason,
};
// FR-29 — the host transport crossing's core-side types (the bridge's cabi
// module implements `HostDialer` over its registry; the descriptor, the frozen
// dial-code table and the ABI version are the contract's Rust half —
// `docs/specs/host-transport-crossing.md` §2, `include/zec_wallet_net_dialer.h`).
pub use net::host_dialer::{
    HOST_DIALER_ABI_VERSION, HOST_TRANSPORT_NAME_MAX_BYTES, HOST_TRANSPORT_READY, HostDialCode,
    HostDialer, HostTransportDescriptor, HostTransportName, IsolationSupport, TransportExposure,
    TransportHealth,
};
pub use seed::{
    SPEND_BINDING_BYTES, SeedPersistence, SeedSource, SeedSupplyError, SpendBinding, WalletSeedPort,
};
// Re-exported so a Rust host implementing `WalletSeedPort` (FR-12 in-workspace, or
// the FR-15 C-ABI shim in the separate-dylib build) can name the seed-supply return
// type without a separate `zeroize` dep + version-match dance.
pub use parked::{
    InFlightSend, ParkedAuthorization, ParkedSend, ParkedSendKind, ReservationPressure,
};
pub use reclaim::ReclaimOutcome;
#[cfg(feature = "swap")]
pub use refund_index::{SwapAddressCheckCoverage, SwapAddressCheckReport};
pub use send::{LargeSendReason, OutputPool, ProposalRecipient, ProposalStep, SendProposal};
pub use state::{
    ArmCounts, BalanceSnapshot, BatchId, BoundedSync, DeliveryState, DialCounts, GraceExpiry,
    IncomingFundsEvent, IncomingFundsEventKind, PoolService, PoolServiceReport, QueuedSendId,
    SigningBlock, StallReason, SyncStamp, SyncStatus, TorRuntimeKind, TorState, TxStatus,
    TxSubmitResult, TxSummary, UnknownBranchGrace, WalletState,
};
pub use stranded::{StrandedAmount, confirmation_depth_is_final};
#[cfg(feature = "swap")]
pub use swap::{
    AssetId, DepositSender, DestinationAddressSource, DisclosureItem, ExactSide, IssuedDeposit,
    IssuedDestination, IssuedQuotePersist, IssuedQuoteRecord, IssuedQuoteStore, IssuedRefund,
    QuoteRequest, QuoteTerms, RefundAddressSource, StartedSwap, SwapAmount, SwapDirection,
    SwapFailureCode, SwapId, SwapKill, SwapPort, SwapPrivacyDisclosure, SwapQuote, SwapRecordSink,
    SwapService, SwapStatus, SwapStatusSink, SwapWatch, TokenInfo, TokenList,
    resolve_manifest_kill, testing,
};
pub use swap_record_store::{InFlightSwap, SwapOutcome};
pub use sweep::EphemeralSweepSummary;
pub use wallet::{IncomingFundsSink, SyncStatusSink, TorStateSink, Wallet};
pub use zeroize::Zeroizing;

// Fuzz-only re-exports (testing-patterns §3): expose crate-internal hostile-input
// parsers to the `cargo-fuzz` harness, which cannot reach a private module.
//
// CFG-GATED, not merely `#[doc(hidden)]` (FR-27 post-fold security review).
// `doc(hidden)` hides an item from rustdoc and from nothing else: these were
// fully callable and semver-public on a crate with no `publish = false`, so a
// downstream consumer could build on a facade we intend to change freely.
// `cargo-fuzz` compiles targets with `--cfg fuzzing`, so the harness still sees
// them and an ordinary build does not. The gate covers all five — the FR-27
// addition inherited the shape from four siblings that predate it.
#[cfg(fuzzing)]
#[doc(hidden)]
pub use enhance::__fuzz_parse_and_validate;
#[cfg(fuzzing)]
#[doc(hidden)]
pub use memo::__fuzz_filter_machine_memos;
#[cfg(fuzzing)]
#[doc(hidden)]
pub use sync::__fuzz_block_is_scannable;
#[cfg(fuzzing)]
#[doc(hidden)]
pub use sync::__fuzz_parse_subtree_root;
#[cfg(fuzzing)]
#[doc(hidden)]
pub use transparent::__fuzz_validate_utxo;
