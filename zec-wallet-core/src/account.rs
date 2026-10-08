//! §3.2b — account import: the wallet's one derived account (ZIP-32 account 0).
//!
//! The DB is provisioned ACCOUNT-LESS (db.rs, inc-2b-ii-A: `init_wallet_db` with
//! `seed = None`). This module adds the account that turns a schema into a wallet
//! that can detect funds, over the audited
//! `WalletWrite::import_account_hd` (Rule Zero — we write no account/derivation
//! logic). It is OFFLINE and idempotent; the network FETCH of the birthday
//! treestate and the decision of WHEN to import are the sync engine's
//! orchestration (inc-2c-iv), not this module.
//!
//! **Keys-in-Rust (§4.1):** the seed is wrapped in `secrecy::SecretVec` ONLY
//! because the audited API demands it (a transient zeroizing copy, dropped at
//! return); the returned `UnifiedSpendingKey` is dropped immediately
//! (function-local, never stored, never logged). The only value that ever leaves
//! is the encoded default UA (public by design), read back via the account's
//! public viewing key.
//!
//! **The M2 boundary ([`birthday_from_treestate`]):** the lightwalletd treestate
//! is hostile input (§4.6). We decode its SHAPE here (typed error, never a panic);
//! whether a well-formed treestate is the REAL chain state at that height (a lying
//! endpoint that hides funds) is the continuity check escalated to the sync-engine
//! crypto-change review (inc-2c-iv).

use secrecy::SecretVec;
use std::hash::Hash;
use zcash_client_backend::data_api::locking::LockFilter;
use zcash_client_backend::data_api::wallet::ConfirmationsPolicy;
use zcash_client_backend::data_api::{
    Account, AccountBalance, AccountBirthday, AccountPurpose, CoinbaseFilter, InputSource, Ratio,
    WalletRead, WalletSummary, WalletWrite,
};
use zcash_client_backend::proto::service::TreeState;
use zcash_client_backend::wallet::TransparentAddressMetadata;
use zcash_client_sqlite::error::SqliteClientError;
use zcash_keys::keys::UnifiedFullViewingKey;
use zcash_transparent::address::TransparentAddress;
use zip32::AccountId;

use crate::constants::{SEED_MAX_BYTES, SEED_MIN_BYTES};
use crate::db::WalletConn;
use crate::derivation::default_address_from_ufvk;
use crate::error::WalletError;
use crate::money::{BlockHeight, Network, Zatoshis};
use crate::state::BalanceSnapshot;
use crate::sync::{ClassifyStoreFault, ProgressSnapshot};

/// Opaque human-readable account label stored by `import_account_hd` (the upstream
/// treats it as opaque metadata). One account per wallet (§1.6), so a fixed label
/// suffices; it is NOT key material, NOT a lookup key (the account is referenced by
/// `AccountId::ZERO` / `primary_account_id`), and never leaves the DB. Host-neutral
/// (this is a UNIVERSAL ZEC wallet SDK — no host name baked into stored data).
const ACCOUNT_NAME: &str = "zec-wallet";

/// Import the wallet's single derived account (ZIP-32 account 0) from `seed` at
/// `birthday`, IDEMPOTENTLY (§6.3 / unstable-network): if account 0 already
/// exists this is a NO-OP, so a kill mid-import, a retry after a dropped
/// connection, or a double-call all converge to exactly one account. No block
/// scanning happens here — `import_account_hd` only records the account + initial
/// scan ranges (the scan is the engine's `scan_cached_blocks`, inc-2c-iv) — so the
/// call is one rusqlite write; the caller runs it inside `spawn_blocking`.
///
/// The ZIP-32 account index is PINNED to 0 (the same index
/// `derive_default_address` uses), which is what makes the stored account UA equal
/// the offline-derived one (§3.2b). The returned `(Account, UnifiedSpendingKey)`
/// is dropped immediately (§4.1). A finer error taxonomy (a dedicated public
/// account-import variant + its FRB mirror) rides inc-2c-iv's public sync-error
/// surface; here an import failure fails closed (`StoreCorrupt`), never silent.
///
/// **The birthday is IMMUTABLE after the first import** (idempotency consequence,
/// money-lens S20): once account 0 exists, a later `ensure_account` with a
/// DIFFERENT `birthday` is a silent no-op — the FIRST birthday wins. So the
/// caller (inc-2c-iv) MUST present the EARLIEST correct birthday on the first
/// import — this seam cannot LOWER it later, and a too-high first birthday
/// silently skips notes below it (a too-high birthday is the §2.3 silent-fund-loss
/// class). A future restore-from-seed flow that re-imports at an earlier height
/// must wipe + re-provision, not call this.
pub(crate) fn ensure_account(
    wdb: &mut WalletConn,
    seed: &[u8],
    birthday: &AccountBirthday,
) -> Result<(), WalletError> {
    if !wdb
        .get_account_ids()
        .map_err(|e| e.into_store_fault())?
        .is_empty()
    {
        return Ok(());
    }
    // Defense-in-depth panic guard (§4.2(b)): a seed outside 32..=252 PANICS in
    // the `UnifiedSpendingKey::from_seed` that `import_account_hd` calls, and a
    // panic across FFI is a crash. SeedSource/SeedPayload
    // already enforce this; re-enforce so the function is panic-safe for any caller.
    if !(SEED_MIN_BYTES..=SEED_MAX_BYTES).contains(&seed.len()) {
        return Err(WalletError::InvalidSeedLength { len: seed.len() });
    }
    // The single transient copy of the seed the audited API requires (`SecretVec`
    // zeroizes on drop); released at return. The `(Account, UnifiedSpendingKey)`
    // the call returns is dropped by the trailing `;` — never stored or logged.
    let secret = SecretVec::new(seed.to_vec());
    wdb.import_account_hd(ACCOUNT_NAME, &secret, AccountId::ZERO, birthday, None)
        .map_err(|e| e.into_store_fault())?;
    Ok(())
}

/// #397 (spec §3.7 D2): the WATCH-ONLY sibling of [`ensure_account`] — import
/// the single account from a host-supplied UFVK via the audited
/// `import_account_ufvk` with [`AccountPurpose::ViewOnly`] (the engine then
/// tracks NO spend information; Rule Zero, no hand-rolled key handling).
/// Same idempotency contract: once ANY account exists this is a no-op, so a
/// repair-resume with the account already imported converges — and the SAME
/// first-birthday-wins consequence: the caller must present the earliest
/// correct birthday on the first import (a later lower birthday needs the
/// rescan rebuild, exactly like the seeded path).
pub(crate) fn ensure_account_ufvk(
    wdb: &mut WalletConn,
    ufvk: &UnifiedFullViewingKey,
    birthday: &AccountBirthday,
) -> Result<(), WalletError> {
    if !wdb
        .get_account_ids()
        .map_err(|e| e.into_store_fault())?
        .is_empty()
    {
        return Ok(());
    }
    wdb.import_account_ufvk(ACCOUNT_NAME, ufvk, birthday, AccountPurpose::ViewOnly, None)
        .map_err(|e| e.into_store_fault())?;
    Ok(())
}

/// FR-12 B1 (money-safety): verify the seed about to SIGN actually CONTROLS the
/// wallet's stored account before any transaction is built. The caller derives the
/// `UnifiedSpendingKey` from the supplied seed (held, or pulled through the
/// [`WalletSeedPort`](crate::seed::WalletSeedPort)) and passes ITS UFVK; the audited
/// `WalletRead::get_account_for_ufvk` returns the account IFF that UFVK matches a
/// stored account — no hand-rolled key compare (Rule Zero). A miss means the supplied
/// seed is WRONG (or controls no stored account) ⇒ typed [`WalletError::SeedMismatch`],
/// NEVER an opaque `SignFailed` and never a tx. Without this gate a host-custodied
/// wrong seed (`SeedPersistence::None`) would only fail deep in the engine's
/// `KeyNotRecognized` AFTER the one-shot proposal token was consumed.
///
/// Cheap (one or two indexed lookups), so the caller runs it under the db lock just
/// before consuming the token. A DB fault is classified (`into_store_fault`, R12) —
/// never `.ok()`-swallowed into a false match (rust-patterns). The match is BINARY
/// presence only — it never reveals WHICH account or anything about the seed, so it is
/// not a decoy-vs-real oracle (the decoy story is a SEPARATE `db_dir`, gated on FR-13).
///
/// **No account yet ⇒ no-op.** When the wallet has no imported account, there is
/// nothing to verify against, so this is `Ok(())` — the seed-vs-account check would
/// otherwise reject the CORRECT seed (no account ⇒ no UFVK match). This never weakens
/// a real sign: a signable proposal can only exist once an account does (`propose`
/// requires `primary_account_id`), so by the time a real token reaches signing the
/// account is always present and the check is always active. It also leaves the
/// account-less token-consume ordering (the unknown-token gate) intact. B1 catches the
/// realistic bug — the host staged a DIFFERENT wallet's seed for THIS send; a seed
/// that was wrong at BOTH import and sign is the host's identity-fingerprint
/// responsibility (the H2 hardening), not detectable here.
pub(crate) fn ensure_seed_controls_account(
    wdb: &WalletConn,
    ufvk: &UnifiedFullViewingKey,
) -> Result<(), WalletError> {
    if primary_account_id(wdb)?.is_none() {
        return Ok(());
    }
    match wdb
        .get_account_for_ufvk(ufvk)
        .map_err(|e| e.into_store_fault())?
    {
        Some(_) => Ok(()),
        None => Err(WalletError::SeedMismatch),
    }
}

/// The wallet's default receive UA, read from the STORED account's UFVK (§3.2b).
/// `Ok(None)` if no account has been imported yet (the caller falls back to the
/// offline seed-derivation shortcut, wallet.rs). Derives PURELY from the account's
/// public viewing key — no seed — which is what lets `current_address` work in
/// `None`-persistence (no seed at rest). The UA equals the seed-derived one by
/// construction (shared `default_address_from_ufvk`, §3.2b).
/// The wallet's single account id (§1.6 v1 = one account), or `Ok(None)` before the
/// account is imported. The only id this returns is the one `import_account_hd`
/// created at ZIP-32 index 0 — the opaque DB id is NOT the ZIP-32 index. ONE door for
/// "which account do we operate on" (shared by `account_default_address` and the send
/// path's `propose`); a future multi-account increment selects the account whose
/// `Zip32Derivation.account_index == 0` explicitly here, in one place. A DB fault is
/// classified (`ClassifyStoreFault`, R12), never `.ok()`-swallowed.
pub(crate) fn primary_account_id(
    wdb: &WalletConn,
) -> Result<Option<<WalletConn as WalletRead>::AccountId>, WalletError> {
    let ids = wdb
        .get_account_ids()
        .map_err(ClassifyStoreFault::into_store_fault)?;
    Ok(ids.into_iter().next())
}

/// The primary account's birthday height — the FLOOR of the `birthday→tip` window the
/// monotonic scan `percent` ([`progress_percent`]) is measured over. The engine reads
/// this ONCE per pass to render an honest, monotonic blocks-left countdown that AGREES
/// with `percent` by construction (`tip − scanned_to == round(span · (1 − percent))`,
/// `wallet::scanned_equiv`) — NEVER the per-range download `frontier`, which jumps
/// between Spend-before-Sync priority ranges (§2.5 "never range position": a host
/// rendering `tip − frontier` sees the contradictory "1 %, 289 blocks left"). `Ok(None)`
/// before an account exists (pre-provision) ⇒ the caller falls back to the frontier
/// mapping (no worse than before). Read-only; the height is §5.4-loggable (a height,
/// not money). A DB fault PROPAGATES as `Err(StoreCorrupt)` to the caller — THIS fn never
/// `.ok()`-swallows it; the engine wrapper ([`crate::wallet::Wallet::scan_window_start`]) is
/// what chooses to degrade a corrupt floor-read to the frontier fallback rather than fail the
/// pass, since the floor is a UX estimate (§2.5), not a money input (the money paths surface
/// the same corruption independently and fail-closed). The upstream `get_account_birthday`
/// returns the `zcash_client_backend` height type, so the `u32`→`money::BlockHeight` hop is a
/// real cross-type conversion, not an identity round-trip.
pub(crate) fn scan_window_start(wdb: &WalletConn) -> Result<Option<BlockHeight>, WalletError> {
    let Some(id) = primary_account_id(wdb)? else {
        return Ok(None);
    };
    let birthday = wdb
        .get_account_birthday(id)
        .map_err(|e| e.into_store_fault())?;
    Ok(Some(BlockHeight::new(u32::from(birthday))))
}

pub(crate) fn account_default_address(
    wdb: &WalletConn,
    network: Network,
) -> Result<Option<String>, WalletError> {
    let Some(account_id) = primary_account_id(wdb)? else {
        return Ok(None);
    };
    let account = wdb
        .get_account(account_id)
        .map_err(|e| e.into_store_fault())?
        // an id returned by get_account_ids must resolve — its absence is corruption
        .ok_or(WalletError::StoreCorrupt)?;
    // a derived (HD) account always carries a UFVK; its absence is corruption
    let ufvk = account.ufvk().ok_or(WalletError::StoreCorrupt)?;
    default_address_from_ufvk(network, ufvk).map(Some)
}

/// The stored account's UFVK in the standard unified encoding — the #397 D1
/// EXPORT read path (spec §3.7). Reads the ENGINE's persisted account, so it
/// needs NO seed and works at every custody tier (`SealedKeychain`,
/// `None`-persistence, watch-only alike). `Ok(None)` if no account has been
/// imported yet (the caller decides the pre-import story — mirror of
/// [`account_default_address`]). The returned string is TOTAL-HISTORY-
/// VISIBILITY material (§3.7): the ONLY caller is the sanctioned
/// `Wallet::export_ufvk`; never logged (§5.4).
pub(crate) fn account_ufvk_encoding(
    wdb: &WalletConn,
    network: Network,
) -> Result<Option<String>, WalletError> {
    let Some(account_id) = primary_account_id(wdb)? else {
        return Ok(None);
    };
    let account = wdb
        .get_account(account_id)
        .map_err(|e| e.into_store_fault())?
        // an id returned by get_account_ids must resolve — its absence is corruption
        .ok_or(WalletError::StoreCorrupt)?;
    // a stored account always carries a UFVK; its absence is corruption
    let ufvk = account.ufvk().ok_or(WalletError::StoreCorrupt)?;
    Ok(Some(crate::derivation::encode_ufvk(network, ufvk)))
}

/// The wallet's transparent RECEIVE address, read from the STORED account's UFVK
/// (Recv-2 / spec §3.3a; ADR-0528) — the account's canonical external-scope
/// (`m/44'/coin'/0'/0/0`) P2PKH receiver. `Ok(None)` if no account has been imported yet
/// (the caller falls back to the offline seed-derivation shortcut, wallet.rs), exactly like
/// [`account_default_address`]. Derives PURELY from the account's transparent public viewing
/// key — no seed — so `current_transparent_address` works in `None`-persistence (no seed at
/// rest). The address is PUBLIC (the spend key never leaves Rust, §4.1); §5.4 NEVER-LOG holds.
///
/// Distinct from the shielded default UA: that one Omits transparent (G7); THIS exposes the
/// transparent receiver as a bare t-address, by explicit user opt-in (§3.3a — default shielded).
pub(crate) fn account_transparent_address(
    wdb: &WalletConn,
    network: Network,
) -> Result<Option<String>, WalletError> {
    let Some(account_id) = primary_account_id(wdb)? else {
        return Ok(None);
    };
    let account = wdb
        .get_account(account_id)
        .map_err(|e| e.into_store_fault())?
        .ok_or(WalletError::StoreCorrupt)?;
    let ufvk = account.ufvk().ok_or(WalletError::StoreCorrupt)?;
    crate::derivation::transparent_receive_address_from_ufvk(network, ufvk).map(Some)
}

/// The account's transparent receive RECEIVER (typed), for the Recv-2b UTXO-detection match
/// ([`crate::transparent`]). `None` when no account is provisioned yet (a fresh wallet's first
/// sync pass, before `provision_account`) — the caller no-ops the poll. Reads the stored account
/// UFVK; no seed (None-persistence-safe, exactly like [`account_transparent_address`]). The
/// typed sibling of that function — same UFVK, same derivation — so the address detection matches
/// against is byte-identical to the address the UI displays.
pub(crate) fn account_transparent_receiver(
    wdb: &WalletConn,
) -> Result<Option<TransparentAddress>, WalletError> {
    let Some(account_id) = primary_account_id(wdb)? else {
        return Ok(None);
    };
    let account = wdb
        .get_account(account_id)
        .map_err(|e| e.into_store_fault())?
        .ok_or(WalletError::StoreCorrupt)?;
    let ufvk = account.ufvk().ok_or(WalletError::StoreCorrupt)?;
    crate::derivation::transparent_receiver_from_account_ufvk(ufvk).map(Some)
}

/// Enumerate the account's engine-owned EPHEMERAL transparent receivers within the §3.2i-2 2e-2b
/// detect's bounded-lookback window, each with its [`TransparentAddressMetadata`] (the reservation
/// `Exposure` height + the engine `next_check_time` deferral the
/// [`crate::ephemeral_detect::in_detect_scope`] predicate uses). Uses the PRODUCTION `WalletRead`
/// read (`get_ephemeral_transparent_receivers`, NOT the test-only `WalletTest::
/// get_known_ephemeral_addresses`): the engine itself applies the bounded-lookback window —
/// `exposed_at_height > chain_tip − EPHEMERAL_DETECT_LOOKBACK_BLOCKS` — and excludes never-exposed
/// (gap-advanced) addresses, so the unbounded-fingerprint footgun never even leaves the DB.
/// `exclude_used = false` so a STRANDED / RETURNED ephemeral (one that has received funds) is
/// INCLUDED — the whole point of the detect. Empty before any account exists AND when no ephemeral was
/// ever reserved (a wallet that has made no TEX send ⇒ `reserve_next_n_ephemeral_addresses` is never
/// called; post-gate-removal a TEX two-step DOES reserve one). An UNSYNCED wallet (no chain height ⇒ no
/// window) yields an empty set, NOT a corruption error. Any other DB fault is the
/// [`StoreCorrupt`](WalletError::StoreCorrupt) door. §5.4: the returned addresses are
/// wallet-controlled t-addresses — the CALLER must never log them (only the detect COUNTS are
/// observable).
pub(crate) fn known_ephemeral_addresses(
    wdb: &WalletConn,
) -> Result<Vec<(TransparentAddress, TransparentAddressMetadata)>, WalletError> {
    known_ephemeral_addresses_within(wdb, crate::constants::EPHEMERAL_DETECT_LOOKBACK_BLOCKS)
}

/// The §3.2i-2 2e-2b-v-2 MANUAL ephemeral-SWEEP enumeration: ALL reserved ephemerals, NOT just the
/// ~48 h automatic-detect window. The completeness gap the sweep closes is a LATE return (one arriving
/// long AFTER the detect window keyed on reservation height), so the manual recovery MUST reach every
/// reserved ephemeral. We pass `exposure_depth = tip` so the engine's window floor
/// (`exposed_at_height > chain_tip.saturating_sub(exposure_depth)`) collapses to ~0 — every exposed
/// ephemeral. `tip` is the wallet's recorded `chain_height` read under the SAME lock as this call, so
/// `chain_tip − tip` is `0` exactly. (A `u32::MAX` depth would ALSO work — the engine SATURATES the
/// subtraction to 0 [money-red-team / reliability round, engine-verified against
/// `zcash_client_sqlite`] — but passing the read `tip` is the self-documenting "all reserved" idiom and
/// never relies on that saturation.) `exclude_used = false` (inherited) so a USED ephemeral that later
/// received a return is still reached. §5.4: the returned t-addresses are wallet-controlled — never log.
pub(crate) fn all_reserved_ephemeral_addresses(
    wdb: &WalletConn,
    tip: u32,
) -> Result<Vec<(TransparentAddress, TransparentAddressMetadata)>, WalletError> {
    known_ephemeral_addresses_within(wdb, tip)
}

/// Shared enumeration of the engine's reserved EPHEMERAL receivers at a given `exposure_depth` (the
/// engine applies the SQL window `exposed_at_height > chain_tip − exposure_depth`). The automatic
/// detect passes [`EPHEMERAL_DETECT_LOOKBACK_BLOCKS`](crate::constants::EPHEMERAL_DETECT_LOOKBACK_BLOCKS);
/// the manual sweep passes the live tip ([`all_reserved_ephemeral_addresses`]).
fn known_ephemeral_addresses_within(
    wdb: &WalletConn,
    exposure_depth: u32,
) -> Result<Vec<(TransparentAddress, TransparentAddressMetadata)>, WalletError> {
    let Some(account_id) = primary_account_id(wdb)? else {
        return Ok(Vec::new());
    };
    match wdb.get_ephemeral_transparent_receivers(account_id, exposure_depth, false) {
        Ok(map) => Ok(map.into_iter().collect()),
        // An unsynced wallet has no chain height yet ⇒ the engine cannot compute the window ⇒
        // nothing to enumerate (NOT corruption — the next synced pass runs the detect).
        Err(SqliteClientError::ChainHeightUnknown) => Ok(Vec::new()),
        Err(e) => Err(e.into_store_fault()),
    }
}

/// §3.2i-2 2e-2b-v-3 — the OUTSTANDING ephemeral-reservation count (the leaked-ephemeral
/// reservation-PRESSURE numerator). Read via
/// `get_ephemeral_transparent_receivers(account, exposure_depth = tip, exclude_used = TRUE)`.
///
/// ⚠ KNOWN UNDER-COUNT (#315 slice-2 proof discovery, engine-verified at the pinned source): this
/// count is BLIND to created-tx0 leaks. `exclude_used = true` keys on the EXISTENCE of a
/// `transparent_received_outputs` row — and the engine writes that row for the wallet's OWN tx0 at
/// CREATE-PERSIST time, UNMINED (`store_transactions_to_be_sent`, `Recipient::EphemeralTransparent`
/// arm; this doc used to claim "a mined tx0", which is WRONG). So every created-then-expired leak —
/// the common censored/offline shape — reads "used" and is NOT counted: the gauge can read 0 while
/// the reservation window is fully bricked. What it DOES count: reservations whose create FAULTED
/// after reserving (no tx persisted, no output row — the #315 create-fault leak path). Honest
/// remediation is upstream (the #315 FR asks for gap-window occupancy / per-address mined-use; a
/// Rule-Zero-clean rebuild does not exist — raw SQL against engine tables is forbidden). Consumers
/// must treat this as a LOWER BOUND indicator: never a money gate (the typed `TexSendLimitReached`
/// at create is the real gate), never a reclaim success signal (the reservation probe is —
/// `send.rs::self_mint_on_the_highest_leaked_ephemeral_reopens_the_whole_window` pins the blindness).
///
/// `exposure_depth = tip` (the live `chain_height`, read under the SAME lock) collapses the engine
/// window floor to ~0 so every QUALIFYING reservation is counted, not just recent ones. The count is
/// bounded by the gap limit (the engine refuses to reserve beyond it —
/// [`EPHEMERAL_GAP_LIMIT`](crate::constants::EPHEMERAL_GAP_LIMIT)), so the `usize → u32` is never near
/// saturation (the `unwrap_or` is a never-taken belt). Empty (`0`) when no TEX two-step is in flight (a
/// wallet that has reserved no ephemeral) and on an unsynced wallet (no window). A DB
/// fault is the [`StoreCorrupt`](WalletError::StoreCorrupt) door. §5.4: a COUNT (loggable) — no address
/// leaves this function.
pub(crate) fn outstanding_ephemeral_reservations(
    wdb: &WalletConn,
    tip: u32,
) -> Result<u32, WalletError> {
    let Some(account_id) = primary_account_id(wdb)? else {
        return Ok(0);
    };
    match wdb.get_ephemeral_transparent_receivers(account_id, tip, true) {
        Ok(map) => Ok(u32::try_from(map.len()).unwrap_or(u32::MAX)),
        // An unsynced wallet has no chain height ⇒ no window ⇒ no outstanding count (NOT corruption).
        Err(SqliteClientError::ChainHeightUnknown) => Ok(0),
        Err(e) => Err(e.into_store_fault()),
    }
}

/// The engine's current chain-tip height (the §3.2i-2 2e-2b detect's bounded-lookback reference),
/// as a bare `u32`. `Ok(None)` before the first scan records a tip (an UNSYNCED wallet — the detect
/// caller no-ops, since a window cannot be computed). A height is §5.4-loggable (public chain data).
/// A DB fault is the corruption door.
pub(crate) fn chain_tip_height(wdb: &WalletConn) -> Result<Option<u32>, WalletError> {
    let tip = wdb.chain_height().map_err(|e| e.into_store_fault())?;
    Ok(tip.map(u32::from))
}

/// The mined height of a transaction by txid (`None` ⇒ not on-chain yet), for the §3.2i-2 2e-2b-ii
/// reap's burial check ([`crate::stranded::should_reap`] on tx0). A height is §5.4-loggable (public
/// chain data); a DB fault is the [`StoreCorrupt`](WalletError::StoreCorrupt) door.
pub(crate) fn tx_mined_height(
    wdb: &WalletConn,
    txid: [u8; 32],
) -> Result<Option<u32>, WalletError> {
    let height = wdb
        .get_tx_height(zcash_protocol::TxId::from_bytes(txid))
        .map_err(|e| e.into_store_fault())?;
    Ok(height.map(u32::from))
}

/// The §3.2i-2 2e-2b-ii stranded surface read: the account's RECOGNISED recoverable balance on each
/// EPHEMERAL transparent address, via the PRODUCTION `WalletRead::get_transparent_balances` (ADR-0535
/// Decision 1). The `zcash_client_sqlite` impl applies the `excluding_wallet_internal_ephemeral_outputs`
/// 3-way OR, so the EPHEMERAL-scope entries it returns are EXACTLY the recoverable cases — a STRAND
/// (observed unspent past tx1's expiry) or an exchange RETURN (a funding tx with no wallet inputs);
/// the normal in-flight ephemeral is excluded. We filter to `TransparentKeyOrigin::Derived { scope:
/// EPHEMERAL }` as SDK-side belt-and-braces (the external/internal receive/change balance is Recv-2b's
/// concern, never folded in here).
///
/// Each entry carries `recoverable_zat = Balance::total()` — the ECONOMICALLY-recoverable amount.
/// `total` sums `spendable_value + change_pending_confirmation + value_pending_spendability` but
/// EXCLUDES `uneconomic_value`: the engine routes a transparent UTXO ≤ the
/// ZIP-317 marginal fee into `uneconomic_value` (it cannot be swept for less than it costs), so dust is
/// omitted BY DESIGN — surfacing un-sweepable dust as "recoverable" would mislead. A dust-only ephemeral
/// therefore collapses to a zero total and is skipped. (The 2e-2b-v raw-UTXO manual sweep enumerates
/// dust too, so it must reconcile against this intentionally-economic surface.) Amounts are carried as
/// `u64` (the engine `Zatoshis` is a DIFFERENT type from `crate::money::Zatoshis`; `u64::from` is the
/// lossless cross). §5.4: the returned addresses are wallet-controlled t-addresses the CALLER must never
/// log/render (only the COUNT is observable).
///
/// FINALITY is NOT computed here (it is 2e-2b-iii's `is_final` deliverable). An real-engine
/// finding pinned WHY: `get_transparent_balances` does NOT split a transparent balance by confirmation
/// depth — a 1-confirmation ephemeral output appears fully in `Balance::spendable_value` (the
/// confirmations policy governs SHIELDING eligibility, not a depth bucketing of the read). So a
/// reorg-finality split cannot come from this call; iii derives `is_final` from per-tx/per-output depth
/// (the host's `transactions()` view or a per-output read) against the `REORG_MAX_BLOCKS` SSOT.
///
/// `Ok(Vec::new())` before any account exists AND when no ephemeral was ever reserved (a wallet that has
/// made no TEX send). An unsynced wallet (no chain state for the read) yields empty, NOT
/// corruption. Any other DB fault is the [`StoreCorrupt`](WalletError::StoreCorrupt) door.
pub(crate) fn ephemeral_recoverable_balances(
    wdb: &WalletConn,
    tip: u32,
) -> Result<Vec<crate::stranded::EphemeralRecoverable>, WalletError> {
    // The §1.6 single-account wallet: the PRIMARY account is the only one (mirrors the i detect's
    // `known_ephemeral_addresses`). A future multi-account wallet would fold every account here, as the
    // main-balance read does — recorded so a strand on a non-primary account can't go silently unsurfaced.
    let Some(account_id) = primary_account_id(wdb)? else {
        return Ok(Vec::new());
    };
    // TargetHeight = the NEXT chain tip (`tip + 1`, the standard confirmation reference). The policy
    // is the SDK spendability SSOT ([`spendable_policy`]); since we read `total()` (all recognised
    // value regardless of bucket), the choice does not affect the surfaced amount — it only satisfies
    // the API. Saturates at u32 max.
    let target = zcash_client_backend::data_api::wallet::TargetHeight::from(tip.saturating_add(1));
    let balances = match wdb.get_transparent_balances(account_id, target, spendable_policy()) {
        Ok(map) => map,
        // No chain state for the read yet (an unsynced wallet) ⇒ nothing recoverable to surface (NOT
        // corruption — the next synced pass runs it).
        Err(SqliteClientError::ChainHeightUnknown) => return Ok(Vec::new()),
        Err(e) => return Err(e.into_store_fault()),
    };
    let mut out = Vec::new();
    for (address, (origin, balance)) in balances {
        // ONLY the EPHEMERAL (ZIP-320 one-time) scope is a stranded/returned surface; an external/
        // internal receiver is the ordinary receive/change balance and is never a "stranded" amount.
        let is_ephemeral = matches!(
            origin,
            zcash_client_backend::data_api::TransparentKeyOrigin::Derived {
                scope: zcash_transparent::keys::TransparentKeyScope::EPHEMERAL,
            }
        );
        if !is_ephemeral {
            continue;
        }
        let recoverable_zat = u64::from(balance.total());
        // Skip a zero-total ephemeral (spent / never-funded / dust-only — `total` excludes
        // uneconomic dust) — nothing economically recoverable to surface.
        if recoverable_zat == 0 {
            continue;
        }
        out.push(crate::stranded::EphemeralRecoverable {
            address,
            recoverable_zat,
        });
    }
    Ok(out)
}

/// The §3.2i-2 2e-2b-iii stranded surface WITH the reorg-finality gate welded in (ADR-0535 Decision 6).
/// For each recoverable ephemeral ([`ephemeral_recoverable_balances`] — the economic amount), reads
/// `InputSource::get_spendable_transparent_outputs` to split that amount by per-output BURIAL and
/// returns the welded [`StrandedAmount`](crate::stranded::StrandedAmount) (`recoverable_zat` + `is_final`).
///
/// WHY a SECOND read: the real-engine finding pinned that `get_transparent_balances` does NOT
/// bucket a transparent balance by confirmation depth (a 1-confirmation output is fully
/// `spendable_value`), so reorg-finality cannot come from the amount read. `get_spendable_transparent_outputs`
/// applies the SAME `excluding_wallet_internal_ephemeral_outputs` predicate AND the SAME dust floor
/// (`value > MARGINAL_FEE`) as the balance read, so its per-UTXO values reconcile with `recoverable_zat`
/// (the buried sum is `<= recoverable_zat`, the invariant `StrandedAmount::welded` relies on). We keep
/// only UTXOs [`buried`](crate::stranded::buried) beyond `REORG_MAX_BLOCKS`; `is_final` is true only when
/// that buried sum equals the whole economic amount — CONSERVATIVE (any shallow/pending portion ⇒ not
/// final). The reads are LOCAL sqlite (no RPC); the recoverable-ephemeral count is tiny (≤ the gap
/// limit, usually 0–1), so the per-ephemeral query is cheap.
///
/// §5.4: returns AMOUNTS + a finality bool only (no address). `Ok(Vec::new())` on the no-ephemeral /
/// unsynced path (inherited from [`ephemeral_recoverable_balances`]); any DB fault is the
/// [`StoreCorrupt`](WalletError::StoreCorrupt) door.
pub(crate) fn ephemeral_recoverable_finality(
    wdb: &WalletConn,
    tip: u32,
) -> Result<Vec<crate::stranded::StrandedAmount>, WalletError> {
    let recoverable = ephemeral_recoverable_balances(wdb, tip)?;
    // SAME confirmation reference as the amount read (`tip + 1`), so the two engine reads agree.
    let target = zcash_client_backend::data_api::wallet::TargetHeight::from(tip.saturating_add(1));
    let mut out = Vec::with_capacity(recoverable.len());
    for e in &recoverable {
        let utxos = wdb
            .get_spendable_transparent_outputs(
                &e.address,
                target,
                spendable_policy(),
                // 0.24.0 renamed `TransparentOutputFilter::All`; same meaning — every
                // spendable transparent output, coinbase or not.
                CoinbaseFilter::AllTransparentOutputs,
                // 0.24.0 (S13) adds note/UTXO locking. This read SUMS buried on-chain
                // value for a finality verdict — it must expose wallet contents
                // regardless of locks, which is what upstream's own `locking.rs` puts
                // on `Unfiltered`. A `Policy` filter here would make a merely-RESERVED
                // output read as gone and UNDERSTATE the recoverable balance.
                LockFilter::Unfiltered,
            )
            .map_err(|e| e.into_store_fault())?;
        // Sum the value of the UTXOs already buried beyond the reorg horizon. An unmined output
        // (`mined_height = None`) is never buried; a shallow one fails the `buried` depth test — both
        // are excluded, so `is_final` stays false until the WHOLE amount is irreversible.
        let buried_zat: u64 = utxos
            .iter()
            .filter(|u| {
                u.mined_height()
                    .is_some_and(|h| crate::stranded::buried(u32::from(h), tip))
            })
            .map(|u| u64::from(u.value()))
            .sum();
        out.push(crate::stranded::StrandedAmount::welded(
            e.recoverable_zat,
            buried_zat,
        ));
    }
    Ok(out)
}

/// Mint + engine-persist a fresh UA DESTINATION at external `index` via the audited
/// `get_address_for_index` (§3.3b D1 / ADR-0530; IZ-1 — the IntoZec on-ramp keystone). The
/// engine PERSISTS the derived address in its `addresses` table (engine-tracked) WITHOUT
/// advancing its gap allocator (the property `get_next_available_address` lacks — it would
/// burn the gap on abandoned quotes, ADR-0530), so `put_received_transparent_utxo` accepts a
/// UTXO at the address's transparent receiver — the §3.3b D2 scoped-poll detection property.
///
/// Returns `Ok(Some(encoded_ua))` on success; `Ok(None)` if the chosen diversifier index
/// cannot conform to `request` (the caller advances to the next single-use index — unreachable
/// for a real key: orchard has no invalid diversifiers and a transparent receiver exists at
/// every non-hardened index, so the §3.3b D1 request conforms at the first reserved index). NO
/// account provisioned yet (a fresh wallet that has not synced — the engine has no UFVK to
/// derive from) ⇒ `KeyDerivation`. An engine/store fault ⇒ `StoreCorrupt`. The address is
/// PUBLIC (handed to 1Click); §5.4 NEVER-LOG holds — it is returned/sent, never logged, and is
/// absent from `Ok(None)`/`Err`.
///
/// **Caller obligation (lock order):** the caller holds the engine `db` lock for this call. In
/// `fresh_destination_address_blocking` it ALSO holds the `aux_db` lock (acquired db→aux, the one
/// documented order) across this call — LOAD-BEARING: `get_address_for_index` is a DEFERRED-
/// then-upgrade engine WRITE that DEADLOCKS against a concurrent aux `BEGIN IMMEDIATE` write on
/// the shared file (immediate `SQLITE_BUSY`, unresolvable by `busy_timeout`), so the engine write
/// must be mutually exclusive with all aux writers (which serialize on `aux_db`).
#[cfg(feature = "swap")]
pub(crate) fn mint_swap_destination(
    wdb: &mut WalletConn,
    network: Network,
    index: u32,
    request: zcash_keys::keys::UnifiedAddressRequest,
) -> Result<Option<String>, WalletError> {
    ua_at_index(wdb, zip32::DiversifierIndex::from(index), request)
        .map(|ua| ua.map(|ua| crate::derivation::encode_unified_address(network, &ua)))
}

/// S7 C2: whether `index` is the default UA's diversifier index (`d0`). The two
/// ALLOCATION paths (`wallet::fresh_destination_address_blocking` and
/// `wallet::fresh_refund_address_blocking`) burn such an index: the destination request
/// takes Orchard and Sapling, so a mint at `d0` would hand the provider the published
/// address's receivers. [`mint_swap_destination`] itself does NOT skip it — the owed-refund
/// backfill must still re-register an index handed out at `d0` before this rule, or that
/// refund's UTXOs would go undetected.
#[cfg(feature = "swap")]
pub(crate) fn is_default_address_index(wdb: &WalletConn, index: u32) -> Result<bool, WalletError> {
    Ok(u32::try_from(primary_account_default_index(wdb)?) == Ok(index))
}

/// The stored account's default-UA diversifier index (`d0`), read from its UFVK through
/// [`crate::derivation::default_address_index`]. No account ⇒ `KeyDerivation` (the mint
/// below would answer the same); a missing account row or UFVK ⇒ `StoreCorrupt`.
#[cfg(feature = "swap")]
fn primary_account_default_index(wdb: &WalletConn) -> Result<zip32::DiversifierIndex, WalletError> {
    let Some(account_id) = primary_account_id(wdb)? else {
        return Err(WalletError::KeyDerivation);
    };
    let account = wdb
        .get_account(account_id)
        .map_err(|e| e.into_store_fault())?
        .ok_or(WalletError::StoreCorrupt)?;
    let ufvk = account.ufvk().ok_or(WalletError::StoreCorrupt)?;
    crate::derivation::default_address_index(ufvk)
}

/// The shared engine-mint core both minting surfaces route through (DRY): derive +
/// engine-persist the UA at a CHOSEN diversifier index via the audited
/// `get_address_for_index` (ADR-0530 — persists in the engine `addresses` table
/// WITHOUT advancing its gap allocator). Self-contained contract (the swap wrapper
/// above is `cfg(swap)`, so this doc must stand alone): `Ok(None)` = the chosen
/// index cannot conform to `request` (the engine maps its per-index diversifier
/// misses silently); NO provisioned account ⇒ typed `KeyDerivation`; an
/// engine/store fault ⇒ fail-closed `StoreCorrupt`. **Caller lock obligation
/// (both wrappers):** the engine `db` lock is held across the call, and db→aux
/// when an aux writer participates — this deferred-then-upgrade engine WRITE
/// deadlocks a concurrent aux `BEGIN IMMEDIATE` on the shared file otherwise.
/// Returns the typed [`UnifiedAddress`](zcash_keys::address::UnifiedAddress) so
/// the FR-8 wrapper can run its receiver-set post-condition BEFORE encoding.
fn ua_at_index(
    wdb: &mut WalletConn,
    diversifier_index: zip32::DiversifierIndex,
    request: zcash_keys::keys::UnifiedAddressRequest,
) -> Result<Option<zcash_keys::address::UnifiedAddress>, WalletError> {
    let Some(account_id) = primary_account_id(wdb)? else {
        return Err(WalletError::KeyDerivation);
    };
    match wdb.get_address_for_index(account_id, diversifier_index, request) {
        Ok(Some(ua)) => Ok(Some(ua)),
        Ok(None) => Ok(None),
        Err(e) => Err(e.into_store_fault()),
    }
}

/// Mint + engine-persist a PUBLIC diversified receive UA at diversifier `index`
/// (FR-8 / Recv-4, ADR-0537) — the [`mint_swap_destination`] sibling for the
/// disjoint high region (`index ≥ DIVERSIFIED_INDEX_BASE = 2^40`, `u64` because the
/// region sits above the BIP44 bound; a G7 UA has no transparent receiver, so the
/// ZIP-32 88-bit diversifier space is the only constraint). The receiver set is the
/// shared G7 builder ([`crate::derivation::g7_receive_request`]) — the SAME
/// compatibility posture as the advertised default UA. With `Require` sapling at a
/// FIXED index, ~half of indices are honest per-index misses (`Ok(None)`): the
/// caller's allocator burns the single-use index and advances (never a silent
/// orchard-only degrade — the W3-inc-2 lesson).
///
/// Enforced post-condition: the minted UA is re-CHECKED against G7
/// ([`crate::derivation::ua_satisfies_g7`]) — a conforming-looking engine reply that
/// dropped a receiver is a typed `KeyDerivation`, never a degraded public address.
/// Same caller lock obligation as [`mint_swap_destination`] (db held, and db→aux
/// when an aux writer participates). §5.4: the address/index are returned, never
/// logged, absent from `Ok(None)`/`Err`.
pub(crate) fn mint_diversified_receive(
    wdb: &mut WalletConn,
    network: Network,
    index: u64,
) -> Result<Option<String>, WalletError> {
    let request = crate::derivation::g7_receive_request()?;
    match ua_at_index(wdb, zip32::DiversifierIndex::from(index), request)? {
        Some(ua) => {
            if !crate::derivation::ua_satisfies_g7(&ua) {
                return Err(WalletError::KeyDerivation);
            }
            Ok(Some(crate::derivation::encode_unified_address(
                network, &ua,
            )))
        }
        None => Ok(None),
    }
}

/// Build an [`AccountBirthday`] from a lightwalletd `GetTreeState` DTO (§3.2b — the
/// M2 hostile-input boundary). `to_chain_state` validates the SHAPE (hex hash → 32
/// bytes, frontier encodings); a malformed/oversized treestate is
/// [`TreeStateInvalid`] here, NEVER a panic. It does NOT — and cannot — verify the
/// treestate is the REAL chain state at that height; that continuity check against
/// a trusted anchor is escalated to the sync-engine crypto-change review (inc-2c-iv,
/// `poisoned_checkpoint_or_tree_state_does_not_silently_hide_funds`, the live arm).
///
/// **HEIGHT CONTRACT (money-lens S20 — read before wiring the fetch):** the
/// returned birthday height is `treestate.height + 1` (the treestate anchors the
/// frontier as of the block PRIOR to the birthday). So to set birthday `H`, the
/// caller (inc-2c-iv) MUST fetch `GetTreeState` at **`H - 1`**, not at `H`.
/// Fetching one block too high records birthday `H + 1` and silently skips block
/// `H`'s notes — the §2.3 too-high-birthday silent-fund-loss class. The deferred
/// continuity check must ALSO clamp the birthday from BELOW (a maliciously-LOW
/// height is a battery/bandwidth scan-DoS), not only reject too-high.
//
// Consumed by `provision::resolve_birthday` (inc-2c-iv-c) — which decodes the
// TRUSTED bundled frontier (§3.2f), NOT a live treestate — and by the §8 tests.
pub(crate) fn birthday_from_treestate(
    treestate: TreeState,
) -> Result<AccountBirthday, TreeStateInvalid> {
    AccountBirthday::from_treestate(treestate, None).map_err(|_| TreeStateInvalid)
}

/// The SINGLE confirmations policy that governs spendability across the WHOLE SDK
/// (maintainer 2026-06-15 — the spendable-confirmations SSOT): the audited ZIP-315
/// `ConfirmationsPolicy::default()` — 3 confirmations for trusted/wallet-internal
/// outputs (our own change), 10 for untrusted incoming. BOTH the live
/// `spendable_ready` sync hint ([`progress_snapshot`]) AND the
/// [`BalanceSnapshot::spendable`] field ([`state_snapshot`], the production balance
/// reader; `summary_snapshot` is the test-only combiner over the same two, P3-7) read
/// it from the SAME `get_wallet_summary()` call, so they can NEVER disagree on what is
/// spendable. The
/// send path (inc-2d) re-validates the actual spend against the live DB under this
/// SAME policy — these are honest UX hints, never the final send-eligibility predicate
/// (§1.7).
///
/// **Why ZIP-315 `default()` over a symmetric depth:** upstream explicitly recommends
/// it, and Zashi + the mobile SDK ship it — so this wallet's on-chain spend timing blends
/// into the ecosystem anonymity set (a bespoke depth would itself be a distinguishing
/// fingerprint, §2.3). `constants::MIN_CONFIRMATIONS = 10` is the UNTRUSTED-incoming
/// depth this policy applies (its conservative arm); a future change to the depth
/// posture changes THIS one function — nothing else.
///
/// `pub(crate)` since inc-2d-1: the send path (`send::propose`) passes THIS SAME
/// policy to `propose_transfer`, so the notes a balance calls "spendable" and the
/// notes propose is allowed to select can never disagree (the SSOT — one policy,
/// no second confirmations constant).
pub(crate) fn spendable_policy() -> ConfirmationsPolicy {
    ConfirmationsPolicy::default()
}

/// ONE audited `get_wallet_summary()` read under [`spendable_policy`] with the
/// fail-closed error posture EVERY reader shares (one source for the read + the
/// error mapping): a DB fault is classified (R12) — NEVER `.ok()`-swallowed
/// (rust-patterns: a transient SQLite error must not masquerade as "no summary").
/// `Ok(None)` ONLY for a genuinely summary-less wallet (no account / pre-first-scan).
/// Each reader below applies its own pure projection to the result.
fn fetch_summary(
    wdb: &WalletConn,
) -> Result<Option<WalletSummary<<WalletConn as WalletRead>::AccountId>>, WalletError> {
    wdb.get_wallet_summary(spendable_policy())
        .map_err(|e| e.into_store_fault())
}

/// Read the live scan-progress + spendability from the audited
/// `WalletRead::get_wallet_summary()` (§3.2g iv-d-3-b-iii-A) — the AUTHORITATIVE source
/// for the `Scanning` UX, replacing the d-3-b-ii coarse `frontier/tip` interim. Rule
/// Zero: we compute NO progress ourselves; `Progress::scan()` (the upstream
/// scanned/total-notes ratio) and `AccountBalance::spendable_value()` ARE the numbers
/// the audited engine derives. Read-only over `WalletRead` — no key material, no
/// secrets logged (the percent/heights are §5.4-loggable; the balance VALUE never is).
///
/// This is the HOT scan-loop path (called per committed batch), so it derives ONLY the
/// progress half — the [`BalanceSnapshot`] (the per-batch sum work) is read on demand
/// by [`state_snapshot`] (production; `summary_snapshot` is the test-only combiner).
/// Both go through [`spendable_policy`], so the
/// `spendable_ready` hint here and the balance reader's `spendable` field agree by
/// construction.
///
/// `Ok(None)` when the wallet has no summary yet (no account, or pre-first-scan) — the
/// caller (`sync_once`) then carries the prior snapshot forward. A DB fault is typed
/// `StoreCorrupt` (never `.ok()`-swallowed — rust-patterns: a transient SQLite error
/// must not masquerade as "no progress").
pub(crate) fn progress_snapshot(wdb: &WalletConn) -> Result<Option<ProgressSnapshot>, WalletError> {
    let Some(summary) = fetch_summary(wdb)? else {
        return Ok(None);
    };
    Ok(Some(progress_from(&summary)))
}

/// The SINGLE combined read of the audited wallet summary → BOTH SDK DTOs (§3.2g
/// iv-d-3-b-iii-B-1, the balance reader): ONE `get_wallet_summary()` call yields the
/// [`ProgressSnapshot`] (scan percent + spendable hint) AND the [`BalanceSnapshot`]
/// (the §2.5 balance breakdown). Reading both from ONE summary means they can never
/// reflect different chain states or different confirmation policies — the SSOT (both
/// derive from the same summary under [`spendable_policy`]). The caller holds the db
/// lock for the single read (one acquire). Read-only over `WalletRead`; the balance
/// VALUES are §5.4 never-log (no amount is traced here).
///
/// `Ok(None)` when the wallet has no summary yet (no account / pre-first-scan) — the
/// caller renders an all-zero balance (`BalanceSnapshot::default()`), never a stale or
/// fabricated number. A DB fault is typed `StoreCorrupt` (fail-closed, never
/// `.ok()`-swallowed); a balance that somehow exceeds the SDK money bound is treated as
/// summary corruption (`StoreCorrupt`), never a silent clamp.
#[cfg(test)]
pub(crate) fn summary_snapshot(
    wdb: &WalletConn,
) -> Result<Option<(ProgressSnapshot, BalanceSnapshot)>, WalletError> {
    let Some(summary) = fetch_summary(wdb)? else {
        return Ok(None);
    };
    Ok(Some((progress_from(&summary), balance_from(&summary)?)))
}

/// The cold state read for `snapshot()` (§3.2g iv-d-3-b-iii-B-2-c, the on-resume bundle):
/// ONE `get_wallet_summary()` → the [`BalanceSnapshot`] AND the chain tip that summary
/// saw, so the bundle's balance and `tip` reflect ONE chain state (never a torn read
/// across two queries). The chain tip is the UNTRUSTED endpoint's best height as the
/// audited engine recorded it — §5.4-loggable (a height, not money). `Ok(None)` when
/// there is no summary yet (no account / pre-first-scan) ⇒ the caller renders an all-zero
/// balance + a `None` tip. Same fail-closed `StoreCorrupt` posture as `summary_snapshot`.
pub(crate) fn state_snapshot(
    wdb: &WalletConn,
) -> Result<Option<(BalanceSnapshot, BlockHeight)>, WalletError> {
    let Some(summary) = fetch_summary(wdb)? else {
        return Ok(None);
    };
    // `chain_tip_height()` NOT `fully_scanned_height()`: `WalletState.tip` is the
    // endpoint's best-known tip, not the scan watermark — the two diverge by up to a
    // batch during active scanning, and the host renders "synced to X of TIP" from this.
    let tip = BlockHeight::new(u32::from(summary.chain_tip_height()));
    Ok(Some((balance_from(&summary)?, tip)))
}

/// Map the audited wallet summary → the SDK [`ProgressSnapshot`] (pure; no IO).
/// `percent` is the upstream MONOTONIC scanned/total-notes fraction across BOTH the
/// recovery and scan windows ([`progress_percent`], checked division); `spendable_ready`
/// is "any account has a positive spendable balance" under [`spendable_policy`] (v1 = one
/// account). `spendable_value()` already applies the policy the summary was read with, so
/// this hint and [`BalanceSnapshot::spendable`] agree by construction.
fn progress_from<A: Eq + Hash>(summary: &WalletSummary<A>) -> ProgressSnapshot {
    let progress = summary.progress();
    let percent = progress_percent(progress.scan(), progress.recovery());
    let spendable_ready = summary
        .account_balances()
        .values()
        .any(|b| !b.spendable_value().is_zero());
    ProgressSnapshot {
        percent,
        spendable_ready,
    }
}

/// Map the audited per-account `AccountBalance`s of a summary → the SDK
/// [`BalanceSnapshot`] (§2.5). Thin wrapper over [`fold_account_balances`] so the
/// money-critical field mapping is unit-testable with hand-built balances (no scanned
/// notes required).
fn balance_from<A: Eq + Hash>(summary: &WalletSummary<A>) -> Result<BalanceSnapshot, WalletError> {
    fold_account_balances(summary.account_balances().values())
}

/// Fold the audited per-account `AccountBalance`s → the SDK [`BalanceSnapshot`],
/// SUMMED across accounts (v1 = one; the sum future-proofs multi-account without
/// leaking an upstream type). The mapping is the audited engine's OWN breakdown — we
/// COMPUTE no balances:
/// - `spendable` ← `spendable_value()` (shielded, confirmed to [`spendable_policy`])
/// - `pending_incoming` ← `value_pending_spendability()` (received, not yet spendable:
///   below depth OR witness-held until more of the chain is scanned — `state.rs` says why
///   the second is the common case)
/// - `pending_change` ← `change_pending_confirmation()` (our own change in flight)
/// - `transparent` ← `unshielded_balance().total()` (unshielded UTXOs — a real,
///   privacy-relevant state that drives the host shield banner, §2.5)
/// - `total` ← `total()` (the audited grand total; mapped DIRECTLY, NOT re-summed from
///   the fields above, so a future upstream pool can never be silently dropped)
///
/// Every value crosses `into_u64()` → the SDK [`Zatoshis`] via [`to_sdk_zat`] (typed
/// reject on the impossible out-of-range, never a clamp); cross-account sums are
/// CHECKED ([`checked_add_zat`]; overflow ⇒ `StoreCorrupt`, never a wrap).
///
/// `pub(crate)` since inc-2d-1: the send path reads `pending_incoming` for the
/// honest `InsufficientFunds` next-step ("wait for confirmations") from the SAME
/// mapping the balance uses, so the two can never report a different pending value.
pub(crate) fn fold_account_balances<'a>(
    balances: impl Iterator<Item = &'a AccountBalance>,
) -> Result<BalanceSnapshot, WalletError> {
    let mut snap = BalanceSnapshot::default();
    for b in balances {
        snap.spendable =
            checked_add_zat(snap.spendable, to_sdk_zat(b.spendable_value().into_u64())?)?;
        snap.pending_incoming = checked_add_zat(
            snap.pending_incoming,
            to_sdk_zat(b.value_pending_spendability().into_u64())?,
        )?;
        snap.pending_change = checked_add_zat(
            snap.pending_change,
            to_sdk_zat(b.change_pending_confirmation().into_u64())?,
        )?;
        snap.transparent = checked_add_zat(
            snap.transparent,
            to_sdk_zat(b.unshielded_balance().total().into_u64())?,
        )?;
        snap.total = checked_add_zat(snap.total, to_sdk_zat(b.total().into_u64())?)?;
    }
    Ok(snap)
}

/// An upstream summary zatoshi count → the SDK [`Zatoshis`]. The audited summary value
/// is itself ≤ MAX_MONEY, so this never narrows in practice; a value that somehow
/// exceeds the SDK bound is summary CORRUPTION ⇒ fail-closed `StoreCorrupt`, never a
/// silent clamp (rust-patterns: never default-on-failure / never-truncate in storage
/// code).
fn to_sdk_zat(raw: u64) -> Result<Zatoshis, WalletError> {
    let signed = i64::try_from(raw).map_err(|_| WalletError::StoreCorrupt)?;
    Zatoshis::new(signed).map_err(|_| WalletError::StoreCorrupt)
}

/// Checked cross-account sum. Overflow past MAX_MONEY is impossible for a real wallet
/// (it cannot hold more than the money supply) ⇒ corruption, typed `StoreCorrupt`.
fn checked_add_zat(acc: Zatoshis, value: Zatoshis) -> Result<Zatoshis, WalletError> {
    acc.checked_add(value).ok_or(WalletError::StoreCorrupt)
}

/// The scan fraction in `[0, 1]` from the upstream `Progress::scan()` ratio (notes
/// scanned / total notes in the birthday→tip window). CHECKED division: a zero
/// denominator means there are NO shielded notes to scan in the window, i.e. nothing
/// to do ⇒ fully progressed (`1.0`) — the upstream `Progress` contract (callers must
/// use checked division), never a NaN/divide-by-zero.
///
/// MONOTONIC absent a reorg: the numerator only grows for a fixed tip, so it never
/// regresses while scanning forward (§2.5 — never the range-position jiggle that
/// `frontier/tip` would show). A REORG rewind un-scans notes and lowers the numerator,
/// so the fraction can legitimately drop to the honest post-rewind truth (the engine
/// refreshes the snapshot on the rewind arm) — a far smaller, truthful move than the
/// `frontier/tip` it replaces, never a value to hide.
fn scan_percent(scan: Ratio<u64>) -> f32 {
    let denom = *scan.denominator();
    if denom == 0 {
        1.0
    } else {
        (*scan.numerator() as f32 / denom as f32).clamp(0.0, 1.0)
    }
}

/// Combine the librustzcash recovery + scan windows into ONE birthday→tip display
/// fraction. `Progress` divides the work into the RECOVERY window (wallet birthday →
/// recovery height) and the SCAN window (recovery height → chain tip). During a deep
/// FIRST sync the active work is the RECOVERY window, so a `scan()`-only percent sits at
/// ~0 for the whole recovery phase even though real progress is being made — the maintainer
/// report: "Scanning 0%" that never moves. Both windows are reported as "notes scanned /
/// notes in window" (upstream doc), so summing the numerators and the denominators yields
/// the overall notes-scanned/total fraction across the entire birthday→tip window — the
/// honest figure to display. `recovery() == None` (no recovery height set) ⇒ the scan
/// window already spans birthday→tip, so it IS the whole fraction. Saturating adds (the
/// sums can't overflow real note counts, but stay honest if they ever did) + the SAME
/// CHECKED division as [`scan_percent`] (a zero-note window ⇒ denominator 0 ⇒ "done",
/// never a NaN). MONOTONIC absent a reorg for the same reason `scan_percent` is: both
/// numerators only grow for a fixed tip; a reorg rewind lowers them to the honest truth.
fn progress_percent(scan: Ratio<u64>, recovery: Option<Ratio<u64>>) -> f32 {
    let combined = match recovery {
        Some(rec) => Ratio::new(
            scan.numerator().saturating_add(*rec.numerator()),
            scan.denominator().saturating_add(*rec.denominator()),
        ),
        None => scan,
    };
    scan_percent(combined)
}

/// A treestate failed to decode (§3.2b). Crate-internal. For the bundled frontier
/// (§3.2f) this is the fail-closed corrupt-binary case (`provision` maps it to
/// `StoreCorrupt`); the decode boundary is shared with the inc-2c-iii M2 path.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TreeStateInvalid;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::derivation::derive_default_address;
    use crate::seal::WalletDbKey;
    use tempfile::tempdir;

    // testnet Sapling activation: an empty tree is genuinely correct at/below it.
    const TESTNET_SAPLING_ACTIVATION: u64 = 280_000;
    const TEST_SEED: &[u8] = b"relim-wallet-kat-seed-0123456789"; // 32 bytes

    /// An empty-tree treestate fixture (no network): `sapling_tree`/`orchard_tree`/
    /// `ironwood_tree` empty ⇒ empty frontiers (upstream `to_chain_state`); a 32-byte
    /// all-zero block hash. Birthday height is `height + 1`.
    ///
    /// Callers use it at or below the testnet Sapling activation, where all three
    /// empty frontiers are genuinely correct. It is NOT a general fixture: above the
    /// NU6.3 activation an empty `ironwood_tree` decodes silently to an empty tree
    /// and would anchor against a pool that exists on chain.
    fn empty_treestate(height: u64) -> TreeState {
        TreeState {
            network: "test".to_owned(),
            height,
            hash: "00".repeat(32), // 64 hex chars = 32 bytes
            time: 0,
            sapling_tree: String::new(),
            orchard_tree: String::new(),
            ironwood_tree: String::new(),
        }
    }

    fn test_birthday() -> AccountBirthday {
        // birthday at testnet Sapling activation (prior block = activation - 1)
        birthday_from_treestate(empty_treestate(TESTNET_SAPLING_ACTIVATION - 1))
            .expect("empty treestate decodes")
    }

    /// A fresh, provisioned, account-less testnet `WalletConn` on a temp keyed DB.
    fn fresh_conn(dir: &std::path::Path) -> (WalletConn, WalletDbKey) {
        let path = dir.join("wallet.db");
        let key = WalletDbKey::generate();
        crate::db::provision_db(&path, &key, Network::Test, None).expect("provision");
        let conn = crate::db::open_db_migrated(&path, &key, Network::Test, None).expect("open");
        (conn, key)
    }

    #[test]
    fn birthday_from_treestate_decodes_and_rejects_malformed() {
        // §8: an empty-tree treestate decodes to a birthday one block above the
        // treestate height; a malformed hash / tree is a typed decode error, never
        // a panic (the M2 SHAPE boundary).
        let b = birthday_from_treestate(empty_treestate(279_999)).expect("decodes");
        assert_eq!(
            u64::from(b.height()),
            280_000,
            "birthday = treestate height + 1"
        );

        // non-hex block hash
        let mut bad = empty_treestate(279_999);
        bad.hash = "nothex!!".to_owned();
        assert_eq!(birthday_from_treestate(bad), Err(TreeStateInvalid));

        // wrong-length hash (16 bytes, not 32)
        let mut short = empty_treestate(279_999);
        short.hash = "00".repeat(16);
        assert_eq!(birthday_from_treestate(short), Err(TreeStateInvalid));

        // garbage sapling frontier bytes (valid hex, invalid frontier encoding)
        let mut badtree = empty_treestate(279_999);
        badtree.sapling_tree = "deadbeef".to_owned();
        assert_eq!(birthday_from_treestate(badtree), Err(TreeStateInvalid));
    }

    #[test]
    fn import_account_is_idempotent() {
        // §8: one account, ever. A second import is a no-op (get_account_ids guard).
        let dir = tempdir().expect("tempdir");
        let (mut conn, _key) = fresh_conn(dir.path());
        let birthday = test_birthday();

        assert!(
            account_default_address(&conn, Network::Test)
                .expect("read")
                .is_none(),
            "no account before import"
        );
        ensure_account(&mut conn, TEST_SEED, &birthday).expect("first import");
        ensure_account(&mut conn, TEST_SEED, &birthday).expect("second import is a no-op");
        let ids = conn.get_account_ids().expect("ids");
        assert_eq!(ids.len(), 1, "exactly one account after a double import");
    }

    #[test]
    fn ensure_seed_controls_account_skips_when_account_less_matches_right_seed_rejects_wrong() {
        // §8 (FR-12 B1): the seed-vs-account gate, all three branches without a funded prove.
        // (a) account-less ⇒ Ok (nothing to verify — a real signable proposal can't exist
        //     before an account does, so the skip never weakens a real sign).
        // (b) MONEY-AVAILABILITY: the CORRECT seed's UFVK controls the imported account ⇒ Ok.
        //     This pins the import↔derive UFVK round-trip — if it ever broke, B1 would return
        //     a false SeedMismatch and BRICK every None-mode spend, and only this test (not the
        //     E2E-deferred funded sign) would catch it.
        // (c) MONEY-SAFETY: a DIFFERENT seed's UFVK does not ⇒ typed SeedMismatch.
        use crate::derivation::derive_spending_key;
        const OTHER_SEED: &[u8] = b"relim-wallet-OTHER-seed-87654321"; // 32 bytes, ≠ TEST_SEED

        let dir = tempdir().expect("tempdir");
        let (mut conn, _key) = fresh_conn(dir.path());
        let ufvk_of = |seed: &[u8]| {
            derive_spending_key(Network::Test, seed)
                .expect("derive usk")
                .to_unified_full_viewing_key()
        };

        // (a) no account yet
        ensure_seed_controls_account(&conn, &ufvk_of(TEST_SEED)).expect("account-less is a no-op");

        ensure_account(&mut conn, TEST_SEED, &test_birthday()).expect("import");

        // (b) the right seed controls the account
        ensure_seed_controls_account(&conn, &ufvk_of(TEST_SEED))
            .expect("the import seed's UFVK controls the stored account");

        // (c) a wrong seed does not
        match ensure_seed_controls_account(&conn, &ufvk_of(OTHER_SEED)) {
            Err(WalletError::SeedMismatch) => {}
            other => panic!("expected SeedMismatch for a wrong seed, got {other:?}"),
        }
    }

    #[test]
    fn none_mode_reads_account_ua_without_seed() {
        // §8: after import, the default UA is readable from the DB account UFVK with
        // NO seed (the None-persistence property), and it byte-equals the offline
        // seed-derivation — equality by construction (§3.2b).
        let dir = tempdir().expect("tempdir");
        let (mut conn, _key) = fresh_conn(dir.path());
        ensure_account(&mut conn, TEST_SEED, &test_birthday()).expect("import");

        let from_db = account_default_address(&conn, Network::Test)
            .expect("read")
            .expect("an account exists");
        let from_seed = derive_default_address(Network::Test, TEST_SEED).expect("derive");
        assert_eq!(
            from_db, from_seed,
            "DB account UA == offline seed derivation"
        );
        assert!(from_db.starts_with("utest1"), "testnet-HRP UA");
    }

    /// The first `[b; 32]` testnet seed whose default UA sits past index 0, and that
    /// index (`d0`). Found by a deterministic search and pinned, so the fixture is
    /// the same on every run. `d0 = 1` is the refund counter's floor: the very first
    /// index a fresh wallet hands out.
    #[cfg(feature = "swap")]
    const D0_SEED_BYTE: u8 = 0x00;
    #[cfg(feature = "swap")]
    const D0_OF_THAT_SEED: u32 = 1;

    /// S7 C2 (KAT): the allocators' predicate names the default UA's diversifier index.
    /// On a seed whose `d0 ≥ 1`, the engine WOULD mint a conforming destination there
    /// carrying the published address's Orchard and Sapling receivers (the hazard is
    /// real); [`is_default_address_index`] is true at `d0` and false at `d0 + 1`, and the
    /// destination at `d0 + 1` shares no receiver with the default UA. (The allocation
    /// paths themselves: `wallet::tests::a_quote_never_allocates_the_default_address_index`.)
    #[cfg(feature = "swap")]
    #[test]
    fn a_swap_destination_never_takes_the_default_address_index() {
        use crate::derivation::{default_address_index, derive_spending_key};
        let found = (0u8..=u8::MAX).find_map(|b| {
            let ufvk = derive_spending_key(Network::Test, &[b; 32])
                .expect("derive usk")
                .to_unified_full_viewing_key();
            let d0 = u32::try_from(default_address_index(&ufvk).expect("d0")).expect("u32");
            (d0 >= 1).then_some((b, d0))
        });
        assert_eq!(
            found,
            Some((D0_SEED_BYTE, D0_OF_THAT_SEED)),
            "the pinned d0 ≥ 1 fixture"
        );
        let seed = [D0_SEED_BYTE; 32];
        let d0 = D0_OF_THAT_SEED;
        let dir = tempdir().expect("tempdir");
        let (mut conn, _key) = fresh_conn(dir.path());
        ensure_account(&mut conn, &seed, &test_birthday()).expect("import");
        let request = crate::derivation::swap_destination_address_request().expect("request");
        let published = match zcash_keys::address::Address::decode(
            &zcash_protocol::consensus::TEST_NETWORK,
            &account_default_address(&conn, Network::Test)
                .expect("read")
                .expect("an account exists"),
        ) {
            Some(zcash_keys::address::Address::Unified(ua)) => ua,
            other => panic!("the default address is a UA, got {other:?}"),
        };

        // the allocators' predicate: d0 is burned, the next index is not
        assert!(
            is_default_address_index(&conn, d0).expect("predicate"),
            "the default UA's index is burned, never allocated"
        );
        assert!(!is_default_address_index(&conn, d0 + 1).expect("predicate"));
        let minted = mint_swap_destination(&mut conn, Network::Test, d0 + 1, request)
            .expect("mint")
            .expect("d0 + 1 conforms");
        let minted = match zcash_keys::address::Address::decode(
            &zcash_protocol::consensus::TEST_NETWORK,
            &minted,
        ) {
            Some(zcash_keys::address::Address::Unified(ua)) => ua,
            other => panic!("the destination is a UA, got {other:?}"),
        };
        assert_ne!(
            minted.orchard(),
            published.orchard(),
            "no shared Orchard receiver"
        );
        assert_ne!(
            minted.sapling(),
            published.sapling(),
            "no shared Sapling receiver"
        );

        // the skipped index IS the published address's: the raw engine mint there
        // carries the default UA's receivers
        let raw = ua_at_index(&mut conn, zip32::DiversifierIndex::from(d0), request)
            .expect("engine mint")
            .expect("d0 conforms to the destination request");
        assert_eq!(
            raw.orchard(),
            published.orchard(),
            "d0 is the default UA's index"
        );
        assert_eq!(
            raw.sapling(),
            published.sapling(),
            "d0 is the default UA's index"
        );
    }

    /// S7 C2: the skip lives in the ALLOCATION paths only. The owed-refund backfill
    /// re-registers indices already handed out through [`mint_swap_destination`]; a
    /// refund issued at `d0` before the rule must still be registered there, or its
    /// UTXOs would go undetected. So the mint primitive still answers at `d0`.
    #[cfg(feature = "swap")]
    #[test]
    fn the_backfill_mint_still_registers_the_default_address_index() {
        let dir = tempdir().expect("tempdir");
        let (mut conn, _key) = fresh_conn(dir.path());
        ensure_account(&mut conn, &[D0_SEED_BYTE; 32], &test_birthday()).expect("import");
        let request = crate::derivation::swap_destination_address_request().expect("request");
        let registered = mint_swap_destination(&mut conn, Network::Test, D0_OF_THAT_SEED, request)
            .expect("mint");
        assert!(
            registered.is_some(),
            "a refund issued at d0 before the rule is still registered by the backfill"
        );
    }

    #[test]
    fn none_mode_reads_transparent_address_without_seed() {
        // §8 (Recv-2, ADR-0528): after import, the transparent RECEIVE address is readable
        // from the DB account's transparent UFVK component with NO seed (the None-persistence
        // property), it byte-equals the offline seed-derivation (equality by construction —
        // the shared encoder), and it is the KAT-pinned external-scope address (TEST_SEED is
        // KAT_SEED_32). This is the stored-account path `current_transparent_address` uses.
        let dir = tempdir().expect("tempdir");
        let (mut conn, _key) = fresh_conn(dir.path());
        ensure_account(&mut conn, TEST_SEED, &test_birthday()).expect("import");

        let from_db = account_transparent_address(&conn, Network::Test)
            .expect("read")
            .expect("an account exists");
        let from_seed =
            crate::derivation::derive_transparent_receive_address(Network::Test, TEST_SEED)
                .expect("derive");
        assert_eq!(
            from_db, from_seed,
            "DB account transparent address == offline seed derivation"
        );
        // tied to the derivation KAT (TEST_SEED == KAT_SEED_32) — the external-scope golden
        assert_eq!(
            from_db, "tmVR3wVRZRtGH37nGdQmcWSkb3XJ1CxAnXF",
            "stored-account transparent address must equal the KAT-pinned external-scope receiver"
        );
        assert!(from_db.starts_with("tm"), "testnet P2PKH HRP");
        // it is NOT the shielded UA (a distinct, transparent-only address)
        let ua = account_default_address(&conn, Network::Test)
            .expect("ua")
            .expect("account");
        assert_ne!(
            from_db, ua,
            "transparent receive address must differ from the shielded UA"
        );
    }

    #[test]
    fn scan_percent_is_checked_and_clamped_at_the_boundaries() {
        // §8 (gate 7 — the pure monotonic-fraction helper, fixture-free): the upstream
        // `Progress::scan()` ratio → a `[0,1]` percent with CHECKED division.
        // denominator 0 ⇒ no notes in the window ⇒ nothing to scan ⇒ 1.0 (NOT a NaN).
        assert_eq!(
            scan_percent(Ratio::new(0, 0)),
            1.0,
            "no-notes window ⇒ done"
        );
        assert_eq!(
            scan_percent(Ratio::new(7, 0)),
            1.0,
            "div-0 is 1.0, never NaN"
        );
        // ordinary fractions
        assert_eq!(scan_percent(Ratio::new(0, 4)), 0.0, "nothing scanned yet");
        assert!((scan_percent(Ratio::new(1, 2)) - 0.5).abs() < 1e-6);
        assert!((scan_percent(Ratio::new(3, 4)) - 0.75).abs() < 1e-6);
        assert_eq!(scan_percent(Ratio::new(4, 4)), 1.0, "fully scanned");
        // numerator past denominator (shouldn't happen upstream) clamps, never > 1.0
        assert_eq!(
            scan_percent(Ratio::new(9, 4)),
            1.0,
            "an over-unity ratio clamps to 1.0"
        );
        // the fraction is non-NaN + finite for every case above (no divide-by-zero)
        for (n, d) in [(0u64, 0u64), (5, 0), (1, 3), (10, 10), (1_000_000, 999)] {
            let p = scan_percent(Ratio::new(n, d));
            assert!(p.is_finite() && (0.0..=1.0).contains(&p), "{n}/{d} ⇒ {p}");
        }
    }

    #[test]
    fn progress_percent_combines_recovery_and_scan_windows() {
        // §8 (the maintainer report fix): during a deep FIRST sync the work is in the
        // RECOVERY window while the SCAN window sits at ~0 — a scan-only percent would
        // read "0%" the whole time. The combined fraction (notes scanned / total notes
        // across BOTH windows) advances as recovery progresses.

        // No recovery height ⇒ the scan window spans the whole range ⇒ scan-only.
        assert!((progress_percent(Ratio::new(1, 2), None) - 0.5).abs() < 1e-6);
        assert_eq!(
            progress_percent(Ratio::new(0, 0), None),
            1.0,
            "no notes ⇒ done"
        );

        // THE BUG CASE: the recent scan window is untouched (0/100) but recovery is half
        // done (500/1000). A scan-only percent is 0.0 (the stuck "Scanning 0%"); the
        // combined percent is 500/1100 ≈ 0.45 — a moving figure, never a frozen 0.
        let combined = progress_percent(Ratio::new(0, 100), Some(Ratio::new(500, 1000)));
        assert!(
            combined > 0.0,
            "recovery progress MUST surface, not read 0%"
        );
        assert!(
            (combined - (500.0 / 1100.0)).abs() < 1e-6,
            "numerators + denominators summed across both windows"
        );

        // Both windows fully scanned ⇒ done; both empty ⇒ done (never NaN).
        assert_eq!(
            progress_percent(Ratio::new(100, 100), Some(Ratio::new(1000, 1000))),
            1.0,
            "all notes scanned ⇒ 1.0",
        );
        assert_eq!(
            progress_percent(Ratio::new(0, 0), Some(Ratio::new(0, 0))),
            1.0,
            "no notes in either window ⇒ done, not NaN",
        );

        // Finite + clamped, and MONOTONIC as recovery then scan fill in (fixed tip).
        let seq: Vec<f32> = [(0u64, 0u64), (250, 0), (1000, 0), (1000, 50), (1000, 100)]
            .iter()
            .map(|&(rec, sca)| progress_percent(Ratio::new(sca, 100), Some(Ratio::new(rec, 1000))))
            .collect();
        for p in &seq {
            assert!(
                p.is_finite() && (0.0..=1.0).contains(p),
                "finite+clamped: {p}"
            );
        }
        for w in seq.windows(2) {
            assert!(w[1] >= w[0], "combined progress is monotonic: {seq:?}");
        }
    }

    #[test]
    fn out_of_bounds_seed_import_is_typed_reject_not_panic() {
        // Defense-in-depth: a seed outside 32..=252 would PANIC inside
        // import_account_hd; the guard returns a typed error instead (panic-safe
        // across FFI). BOTH boundaries of SEED_MIN_BYTES/SEED_MAX_BYTES are pinned
        // here (a second enforcement path, distinct from seed.rs/derivation.rs).
        let dir = tempdir().expect("tempdir");
        let (mut conn, _key) = fresh_conn(dir.path());
        let birthday = test_birthday();
        for (bad, len) in [(vec![0u8; 31], 31usize), (vec![0u8; 253], 253usize)] {
            match ensure_account(&mut conn, &bad, &birthday) {
                Err(WalletError::InvalidSeedLength { len: got }) if got == len => {}
                other => panic!("expected typed InvalidSeedLength{{{len}}}, got {other:?}"),
            }
            // and no account was created by the rejected import
            assert!(
                conn.get_account_ids().expect("ids").is_empty(),
                "a rejected import (len {len}) must not create an account"
            );
        }
    }

    // ── iii-B-1: the BalanceSnapshot reader ──────────────────────────────────
    use crate::constants::MAX_MONEY_ZAT;
    use zcash_client_backend::data_api::Balance;
    use zcash_protocol::value::{BalanceError, Zatoshis as UpstreamZat};

    /// Build an upstream `AccountBalance` with DISTINCT per-pool/per-field values so a
    /// field-mapping swap is caught (every field [`fold_account_balances`] reads is set
    /// to a different number). `sapling`/`orchard` are `(spendable, change_pending,
    /// value_pending)`; `transparent` is the unshielded spendable (its change/pending are
    /// always zero upstream).
    fn account_balance(
        sapling: (u64, u64, u64),
        orchard: (u64, u64, u64),
        transparent: u64,
    ) -> AccountBalance {
        account_balance_with_ironwood(sapling, orchard, (0, 0, 0), transparent)
    }

    /// As [`account_balance`], plus the Ironwood pool (NU6.3). Separate rather than a
    /// fourth parameter on the older helper so the fourteen existing call sites keep
    /// their meaning; the pool is exercised by
    /// `fold_account_balances_counts_the_ironwood_pool`, which is the only place that
    /// needs a non-zero value for it.
    fn account_balance_with_ironwood(
        sapling: (u64, u64, u64),
        orchard: (u64, u64, u64),
        ironwood: (u64, u64, u64),
        transparent: u64,
    ) -> AccountBalance {
        let mut ab = AccountBalance::ZERO;
        let set = |b: &mut Balance, (s, c, p): (u64, u64, u64)| -> Result<(), BalanceError> {
            b.add_spendable_value(UpstreamZat::const_from_u64(s))?;
            b.add_pending_change_value(UpstreamZat::const_from_u64(c))?;
            b.add_pending_spendable_value(UpstreamZat::const_from_u64(p))?;
            Ok(())
        };
        ab.with_sapling_balance_mut::<(), BalanceError>(|b| set(b, sapling))
            .expect("sapling");
        ab.with_orchard_balance_mut::<(), BalanceError>(|b| set(b, orchard))
            .expect("orchard");
        ab.with_ironwood_balance_mut::<(), BalanceError>(|b| set(b, ironwood))
            .expect("ironwood");
        // 0.24.0 split the unshielded pool into REGULAR and COINBASE halves.
        // `with_unshielded_balance_mut` is gone; `..._regular_balance_mut` is the
        // half this fixture has always meant — an ordinary received transparent
        // output, not mining income (coinbase carries its own maturity rules and
        // must be shielded through a different path).
        ab.with_unshielded_regular_balance_mut::<(), BalanceError>(|b| {
            b.add_spendable_value(UpstreamZat::const_from_u64(transparent))
        })
        .expect("unshielded");
        ab
    }

    fn zat(v: i64) -> Zatoshis {
        Zatoshis::new(v).expect("in-range test zatoshis")
    }

    #[test]
    fn fold_account_balances_counts_the_ironwood_pool() {
        // §8 — the pool the NU6.3 wave added, and the reason the sibling test above
        // cannot speak for it: `fold_account_balances` reads `spendable_value()`,
        // `value_pending_spendability()`, `change_pending_confirmation()` and
        // `total()`, all of which upstream now sums over THREE shielded pools. Every
        // existing fixture leaves Ironwood at zero, so the fold would produce
        // identical numbers whether upstream counted the pool or silently dropped it
        // — and after the activation Ironwood is where a shielded balance actually
        // lives (every payment to an Orchard receiver is delivered there), so a
        // dropped pool means the user is shown less money than they have.
        //
        // DISTINCT values per field, same discipline as the mapping test: a swapped
        // or missing Ironwood field changes a specific number, not just a total.
        let a = account_balance_with_ironwood((100, 20, 3), (200, 40, 5), (400, 80, 9), 7);
        let snap = fold_account_balances([&a].into_iter()).expect("fold");
        assert_eq!(
            snap.spendable,
            zat(700),
            "spendable sums sapling+orchard+IRONWOOD (100+200+400)"
        );
        assert_eq!(
            snap.pending_change,
            zat(140),
            "pending_change sums all three shielded pools (20+40+80)"
        );
        assert_eq!(
            snap.pending_incoming,
            zat(17),
            "pending_incoming sums all three shielded pools (3+5+9)"
        );
        assert_eq!(
            snap.transparent,
            zat(7),
            "the Ironwood pool is shielded — it must not leak into transparent"
        );
        // total = 123 (sapling) + 245 (orchard) + 489 (ironwood) + 7 = 864
        assert_eq!(snap.total, zat(864), "total counts the Ironwood pool");
        // And the buckets still reconcile with a third pool present — the assertion
        // that catches an upstream pool `total()` counts but no snapshot field maps.
        assert_eq!(
            snap.spendable
                .checked_add(snap.pending_change)
                .and_then(|x| x.checked_add(snap.pending_incoming))
                .and_then(|x| x.checked_add(snap.transparent)),
            Some(snap.total),
            "the mapped fields reconcile to the audited total with Ironwood funded"
        );
        // The reconciliation above holds ONLY while `locked_value` is zero, and
        // survey S7 named that explicitly. 0.24.0 added a fifth component to
        // `Balance::total()` — `spendable + LOCKED + change_pending + pending_spendable`
        // — and `fold_account_balances` maps four of them. It is zero today by
        // construction: every propose passes `lock_inputs: None` and nothing calls
        // `lock_outputs`. Assert it rather than rely on it, because the failure is
        // silent and one-directional: a future path that locks inputs would make
        // `total` exceed the sum of the parts the user can see, and the reconciliation
        // above would be the thing that broke, with no hint why.
        assert_eq!(
            a.sapling_balance().locked_value(),
            UpstreamZat::ZERO,
            "sapling locked_value must stay 0 — we never lock inputs"
        );
        assert_eq!(
            a.orchard_balance().locked_value(),
            UpstreamZat::ZERO,
            "orchard locked_value must stay 0 — we never lock inputs"
        );
        assert_eq!(
            a.ironwood_balance().locked_value(),
            UpstreamZat::ZERO,
            "ironwood locked_value must stay 0 — we never lock inputs"
        );
        // Anti-vacuity: the same fixture with Ironwood zeroed must differ. Without
        // this, a fold that dropped the pool AND a `total()` that dropped it would
        // agree with each other and every assertion above would still pass.
        let without = account_balance_with_ironwood((100, 20, 3), (200, 40, 5), (0, 0, 0), 7);
        let snap_without = fold_account_balances([&without].into_iter()).expect("fold");
        assert_ne!(
            snap.total, snap_without.total,
            "funding Ironwood must change the folded total — otherwise this test is vacuous"
        );
        assert_ne!(
            snap.spendable, snap_without.spendable,
            "funding Ironwood must change the folded spendable"
        );
    }

    #[test]
    fn fold_account_balances_pins_the_mapping_and_sums_across_accounts() {
        // §8 (gate 7, MONEY-critical): the AccountBalance → BalanceSnapshot field mapping
        // is pinned with DISTINCT values (a swapped field fails here), and a second
        // account proves the cross-account SUM. (v1 is one account; the sum future-proofs
        // the multi-account path without leaking an upstream type.)
        let a = account_balance((100, 20, 3), (200, 40, 5), 7);
        let one = fold_account_balances([&a].into_iter()).expect("fold one");
        assert_eq!(
            one.spendable,
            zat(300),
            "spendable = sapling+orchard spendable_value"
        );
        assert_eq!(
            one.pending_change,
            zat(60),
            "pending_change = change_pending_confirmation"
        );
        assert_eq!(
            one.pending_incoming,
            zat(8),
            "pending_incoming = value_pending_spendability"
        );
        assert_eq!(one.transparent, zat(7), "transparent = unshielded total");
        // total = (100+20+3) + (200+40+5) + 7 = 123 + 245 + 7 = 375
        assert_eq!(
            one.total,
            zat(375),
            "total = AccountBalance::total (direct)"
        );
        // The four buckets reconcile to total for the CURRENT pool set (Sapling +
        // Orchard + transparent) — no value invented or dropped at v1 scope; upstream
        // `total()` excludes only the uneconomic dust bucket, which we also omit. This is
        // intentionally fragile: a future upstream pool that `total()` counts but no
        // `BalanceSnapshot` field maps will BREAK this assertion (loudly, by design),
        // flagging the unmapped pool rather than letting `total` silently drift.
        assert_eq!(
            one.spendable
                .checked_add(one.pending_change)
                .and_then(|x| x.checked_add(one.pending_incoming))
                .and_then(|x| x.checked_add(one.transparent)),
            Some(one.total),
            "the mapped fields reconcile to the audited total"
        );

        let b = account_balance((1, 2, 3), (4, 5, 6), 8);
        let two = fold_account_balances([&a, &b].into_iter()).expect("fold two");
        assert_eq!(
            two.spendable,
            zat(300 + 5),
            "spendable sums across accounts"
        );
        assert_eq!(
            two.total,
            zat(375 + (1 + 2 + 3 + 4 + 5 + 6 + 8)),
            "totals sum across accounts"
        );
    }

    #[test]
    fn fold_account_balances_empty_is_all_zero() {
        // §8: no accounts ⇒ the all-zero balance (the `Ok(None)` analog at the fold
        // level; a fresh wallet shows honest zeros, never a fabricated number).
        let none: [&AccountBalance; 0] = [];
        assert_eq!(
            fold_account_balances(none.into_iter()).expect("fold empty"),
            BalanceSnapshot::default()
        );
    }

    #[test]
    fn fold_account_balances_keeps_transparent_pending_out_of_shielded_change() {
        // §8 (mapping CONTRACT): `pending_change`/`pending_incoming` are SHIELDED-only —
        // `AccountBalance::change_pending_confirmation()`/`value_pending_spendability()`
        // sum SAPLING+ORCHARD, NOT the unshielded pool — while `transparent` is the
        // unshielded pool's FULL total. Pin it with an unshielded pool carrying change +
        // pending (an upstream "always-zero in practice" state, constructed here to
        // DEFEND the assumption): those zats must land entirely in `transparent`, never
        // double-counted into the shielded pending fields. If a future upstream version
        // folds transparent pending into the shielded accessors, this fails loudly.
        let mut ab = AccountBalance::ZERO;
        // 0.24.0: `with_unshielded_balance_mut` → `with_unshielded_regular_balance_mut`
        // (the pool split into regular + coinbase halves). `snap.transparent` still
        // reads `unshielded_balance().total()`, which spans BOTH halves, so this
        // fixture's regular-only zats are the same test it always was.
        ab.with_unshielded_regular_balance_mut::<(), BalanceError>(|b| {
            b.add_spendable_value(UpstreamZat::const_from_u64(10))?;
            b.add_pending_change_value(UpstreamZat::const_from_u64(5))?;
            b.add_pending_spendable_value(UpstreamZat::const_from_u64(3))?;
            Ok(())
        })
        .expect("unshielded");
        let snap = fold_account_balances([&ab].into_iter()).expect("fold");
        assert_eq!(snap.spendable, Zatoshis::ZERO, "no shielded spendable");
        assert_eq!(
            snap.pending_change,
            Zatoshis::ZERO,
            "transparent pending is NOT shielded change"
        );
        assert_eq!(
            snap.pending_incoming,
            Zatoshis::ZERO,
            "transparent pending is NOT shielded incoming"
        );
        assert_eq!(
            snap.transparent,
            zat(18),
            "transparent = unshielded total (10+5+3)"
        );
        assert_eq!(
            snap.total,
            zat(18),
            "total counts the unshielded pool exactly once"
        );
    }

    #[test]
    fn fold_account_balances_overflow_is_typed_store_corrupt() {
        // §8 (money-safety): a cross-account sum that exceeds MAX_MONEY is impossible for
        // a real wallet ⇒ corruption, a typed StoreCorrupt — never a silent i64 wrap.
        let max = (MAX_MONEY_ZAT as u64, 0, 0);
        let a = account_balance(max, (0, 0, 0), 0);
        let b = account_balance(max, (0, 0, 0), 0);
        assert!(
            matches!(
                fold_account_balances([&a, &b].into_iter()),
                Err(WalletError::StoreCorrupt)
            ),
            "summing two max-supply accounts overflows ⇒ StoreCorrupt"
        );
    }

    #[test]
    fn to_sdk_zat_rejects_beyond_supply_as_corruption() {
        // §8 (gate 7, fail-closed boundary): the upstream→SDK conversion rejects a value
        // beyond max supply as corruption (StoreCorrupt), never a clamp; an in-range
        // value round-trips exactly.
        assert_eq!(to_sdk_zat(0).expect("zero"), Zatoshis::ZERO);
        assert_eq!(
            to_sdk_zat(MAX_MONEY_ZAT as u64).expect("max"),
            zat(MAX_MONEY_ZAT)
        );
        assert!(
            matches!(
                to_sdk_zat(MAX_MONEY_ZAT as u64 + 1),
                Err(WalletError::StoreCorrupt)
            ),
            "a beyond-supply value is corruption, not a clamp"
        );
        assert!(
            matches!(to_sdk_zat(u64::MAX), Err(WalletError::StoreCorrupt)),
            "u64::MAX (> i64::MAX) is rejected, never wrapped"
        );
    }

    #[test]
    fn summary_snapshot_agrees_with_progress_snapshot_on_a_fresh_wallet() {
        // §8: cross-read CONSISTENCY — the combined read and the progress-only read agree
        // on presence AND on the progress half over a fresh, unscanned, note-free wallet.
        // (The structural SSOT — that `summary_snapshot` derives BOTH DTOs from ONE
        // `get_wallet_summary()` result under one `spendable_policy` — is guaranteed by the
        // code, not this runtime test; this test guards against the readers drifting apart.)
        let dir = tempdir().expect("tempdir");
        let (mut conn, _key) = fresh_conn(dir.path());
        ensure_account(&mut conn, TEST_SEED, &test_birthday()).expect("import");

        let progress_only = progress_snapshot(&conn).expect("progress read");
        let combined = summary_snapshot(&conn).expect("combined read");
        // The PRODUCTION balance reader too (P3-7 made `summary_snapshot` test-only;
        // the security review asked that the pair production actually uses —
        // `progress_snapshot` + `state_snapshot` — is the one this row stands in for).
        let production = state_snapshot(&conn).expect("production balance read");
        assert_eq!(
            production.is_some(),
            progress_only.is_some(),
            "state_snapshot and progress_snapshot agree on summary presence"
        );
        match (progress_only, combined) {
            (None, None) => {} // pre-scan (no chain tip yet): both agree no summary
            (Some(p), Some((cp, bal))) => {
                assert_eq!(p, cp, "combined progress == progress-only read");
                assert!(!cp.spendable_ready, "no notes ⇒ not spendable");
                assert_eq!(
                    bal,
                    BalanceSnapshot::default(),
                    "no notes ⇒ all-zero balance"
                );
                assert_eq!(
                    production.map(|(b, _)| b),
                    Some(bal),
                    "the production reader's balance == the test combiner's"
                );
            }
            // Presence-only in the message (never Debug-format a balance / amount, §5.4).
            (a, b) => panic!(
                "readers disagree on summary presence: {} vs {}",
                a.is_some(),
                b.is_some()
            ),
        }
    }

    #[test]
    fn state_snapshot_is_none_without_an_account() {
        // §8 (B-2-c): the `Ok(None)` path in isolation — an account-less conn has no
        // summary ⇒ `state_snapshot` is `None`, so `snapshot()` renders an all-zero
        // balance + a `None` tip, never a fabricated number. (The `Some(balance, tip)`
        // path + the exact post-sync chain tip ride the wallet.rs integration test
        // `snapshot_after_a_sync_reflects_the_tip_consistently_with_balance`.)
        let dir = tempdir().expect("tempdir");
        let (conn, _key) = fresh_conn(dir.path());
        assert!(
            state_snapshot(&conn).expect("state read").is_none(),
            "no account ⇒ no summary ⇒ None",
        );
    }

    #[test]
    fn mappers_pin_the_spendable_ssot_true_side_and_a_fractional_percent() {
        // operational 3-lens (MONEY — the maintainer's distinct adversarial pass on the
        // committed a042522): the pre-commit suite only exercised the all-zero balance /
        // constant-1.0 percent sides — a note-free `FakeChain` mints nothing AND always
        // reads denominator-0 ⇒ 1.0. A SYNTHETIC summary (built directly via
        // `WalletSummary::new`, no scanned notes / no GA fixtures) pins the three sides
        // the constant fixtures never reached, at the mapper boundary:
        //   (a) the TRUE side of the spendable SSOT — a spendable-bearing account makes
        //       BOTH `progress_from.spendable_ready` AND `balance_from.spendable > 0`,
        //       from the SAME summary (they cannot disagree by construction);
        //   (b) a FRACTIONAL scan ratio maps to 0.75, NOT clamped to 1.0;
        //   (c) the summary→HashMap→fold path across MULTIPLE accounts (the production
        //       `account_balances().values()` iteration, vs the slice the fold tests use),
        //       and a non-spendable account's pending must NOT falsely set spendable_ready.
        use std::collections::HashMap;
        use zcash_client_backend::data_api::{Progress, WalletSummary};
        use zcash_protocol::consensus::BlockHeight as UpstreamHeight;

        let mut balances: HashMap<u32, AccountBalance> = HashMap::new();
        balances.insert(0, account_balance((500, 0, 0), (0, 0, 0), 0)); // 500 shielded spendable
        balances.insert(1, account_balance((0, 0, 7), (0, 0, 3), 0)); // pending-incoming only (10)

        let summary = WalletSummary::new(
            balances,
            UpstreamHeight::from_u32(281_000), // chain tip
            UpstreamHeight::from_u32(280_750), // fully scanned
            Progress::new(Ratio::new(3, 4), None),
            // DISTINCT indices, not three zeros. 0.24.0 appends a third subtree index
            // (`next_ironwood_subtree_index`) to a positional constructor whose last
            // three arguments now share a type; all-zeros makes a mis-ordered or
            // mis-counted argument list invisible. The three values are read back
            // below, which is what turns them into a check rather than decoration.
            11, // next_sapling_subtree_index
            22, // next_orchard_subtree_index (orchard feature ON)
            33, // next_ironwood_subtree_index (NU6.3, same feature gate)
        );
        assert_eq!(summary.next_sapling_subtree_index(), 11);
        assert_eq!(summary.next_orchard_subtree_index(), 22);
        assert_eq!(
            summary.next_ironwood_subtree_index(),
            33,
            "the Ironwood subtree index is its own argument, not a copy of Orchard's"
        );

        let progress = progress_from(&summary);
        let balance = balance_from(&summary).expect("balance maps");

        // (a) + (c): the SSOT true side, derived across the multi-account HashMap.
        assert!(
            progress.spendable_ready,
            "a summary with one spendable-bearing account is spendable_ready"
        );
        assert_eq!(
            balance.spendable,
            zat(500),
            "500 shielded spendable maps through"
        );
        assert_eq!(
            balance.pending_incoming,
            zat(10),
            "the second account's pending sums in (7+3)"
        );
        assert_eq!(
            progress.spendable_ready,
            balance.spendable != Zatoshis::ZERO,
            "spendable_ready ⟺ balance.spendable > 0 (the SSOT, true side)"
        );
        // (b): a fractional scan ratio is NOT clamped to 1.0 (the non-denominator-0 path).
        assert!(
            (progress.percent - 0.75).abs() < 1e-6,
            "a 3/4 scan ratio maps to 0.75, not the denominator-0 1.0: {}",
            progress.percent
        );
    }

    // ── iii-B-1 real-world money edge cases (unstable-net/mobile/money review fold) ──

    /// A synthetic single-snapshot `WalletSummary<u32>` over the given account balances
    /// at the given scan ratio — the no-DB / no-scanned-notes path for exercising the
    /// mappers end to end (the same constructor `mappers_pin_…` uses, factored for reuse).
    fn summary_of(accounts: Vec<AccountBalance>, scan: Ratio<u64>) -> WalletSummary<u32> {
        use std::collections::HashMap;
        use zcash_client_backend::data_api::Progress;
        use zcash_protocol::consensus::BlockHeight as UpstreamHeight;
        let balances: HashMap<u32, AccountBalance> = accounts
            .into_iter()
            .enumerate()
            .map(|(i, b)| (i as u32, b))
            .collect();
        WalletSummary::new(
            balances,
            UpstreamHeight::from_u32(281_000),
            UpstreamHeight::from_u32(280_750),
            Progress::new(scan, None),
            0, // next_sapling_subtree_index
            0, // next_orchard_subtree_index
            0, // next_ironwood_subtree_index (0.24.0)
        )
    }

    #[test]
    fn pending_only_wallet_is_detected_but_never_spendable() {
        // §8 (MONEY / §1.7 Spend-before-Sync honesty — the FALSE side): funds DETECTED
        // but not yet confirmed to the spendable policy (pending_incoming > 0, spendable
        // == 0) MUST read `spendable_ready == false`. Showing "spendable" for unconfirmed
        // funds is the one UX lie the ZIP-315 SSOT exists to prevent — a user would build
        // a tx that fails/expires. A mid-scan (3/4) summary with a large pending balance.
        let summary = summary_of(
            vec![account_balance((0, 0, 4_000_000), (0, 0, 0), 0)],
            Ratio::new(3, 4),
        );
        let progress = progress_from(&summary);
        let balance = balance_from(&summary).expect("maps");
        assert!(
            !progress.spendable_ready,
            "detected-but-unconfirmed is NOT spendable_ready"
        );
        assert_eq!(balance.spendable, Zatoshis::ZERO, "nothing spendable yet");
        assert_eq!(
            balance.pending_incoming,
            zat(4_000_000),
            "the detected funds are pending_incoming"
        );
        assert_eq!(
            progress.spendable_ready,
            balance.spendable != Zatoshis::ZERO,
            "the SSOT identity holds on the false side too"
        );
    }

    #[test]
    fn orchard_only_spendable_is_recognized() {
        // §8 (MONEY — the v1 orchard-on asymmetric pool): spendable funds in ORCHARD with
        // ZERO sapling must still read spendable (AccountBalance::spendable_value sums
        // both pools). Guards a refactor that read only sapling_balance().spendable_value.
        let summary = summary_of(
            vec![account_balance((0, 0, 0), (300, 0, 0), 0)],
            Ratio::new(1, 1),
        );
        let progress = progress_from(&summary);
        let balance = balance_from(&summary).expect("maps");
        assert!(
            progress.spendable_ready,
            "orchard-only spendable IS spendable_ready"
        );
        assert_eq!(
            balance.spendable,
            zat(300),
            "orchard spendable maps through"
        );
        assert_eq!(balance.total, zat(300));
    }

    #[test]
    fn transparent_only_wallet_is_real_but_not_shielded_spendable() {
        // §8 (MONEY — the exchange-withdrawal real-world state): a user receives to a
        // transparent address. The funds are REAL (counted in `total`, light the §2.5
        // shield banner via transparent>0) but NOT shielded-spendable (spend stays
        // disabled until shielded). Guards folding transparent into `spendable` (would
        // offer an impossible shielded spend) or dropping it from `total` (funds vanish).
        let summary = summary_of(
            vec![account_balance((0, 0, 0), (0, 0, 0), 5_000_000)],
            Ratio::new(1, 1),
        );
        let progress = progress_from(&summary);
        let balance = balance_from(&summary).expect("maps");
        assert!(
            !progress.spendable_ready,
            "transparent funds are not shielded-spendable"
        );
        assert_eq!(balance.spendable, Zatoshis::ZERO);
        assert_eq!(
            balance.transparent,
            zat(5_000_000),
            "real transparent funds are visible"
        );
        assert_eq!(balance.total, zat(5_000_000), "and counted in total");
    }

    #[test]
    fn uneconomic_dust_is_invisible_in_every_field() {
        // §8 (MONEY contract — dust exclusion): upstream `Balance::total()` EXCLUDES
        // `uneconomic_value` (sub-marginal-fee dust), and no SDK field maps it, so an
        // account holding ONLY dust reads all-zero (honest-invisible, matches Zashi /
        // §2.5). Guards an upstream change that ever surfaced dust into total()/an
        // accessor (would inflate the visible balance). Built inline — `account_balance`
        // does not expose uneconomic.
        let mut ab = AccountBalance::ZERO;
        ab.with_sapling_balance_mut::<(), BalanceError>(|b| {
            b.add_uneconomic_value(UpstreamZat::const_from_u64(50))
        })
        .expect("uneconomic set");
        let snap = fold_account_balances([&ab].into_iter()).expect("fold");
        assert_eq!(
            snap,
            BalanceSnapshot::default(),
            "dust-only ⇒ every field zero"
        );
    }

    #[test]
    fn fold_at_exactly_max_money_is_ok_inclusive_not_off_by_one() {
        // §8 (gate-7 boundary, MONEY): a wallet holding EXACTLY the money supply maps
        // cleanly (the bound is inclusive). Complements the overflow reject (MAX+MAX →
        // StoreCorrupt): pins the exact flip so a `<` vs `<=` regression — which would
        // false-StoreCorrupt a full-supply wallet (an unreadable balance) — is caught.
        let max = MAX_MONEY_ZAT as u64;
        let one = fold_account_balances([&account_balance((max, 0, 0), (0, 0, 0), 0)].into_iter())
            .expect("a single full-supply account maps (inclusive bound)");
        assert_eq!(one.spendable, zat(MAX_MONEY_ZAT));
        assert_eq!(one.total, zat(MAX_MONEY_ZAT));
        // two accounts summing to EXACTLY MAX (the cap, not over)
        let a = account_balance((max - 1, 0, 0), (0, 0, 0), 0);
        let b = account_balance((1, 0, 0), (0, 0, 0), 0);
        let two = fold_account_balances([&a, &b].into_iter())
            .expect("two accounts summing to exactly MAX map (the cap is inclusive)");
        assert_eq!(
            two.total,
            zat(MAX_MONEY_ZAT),
            "exact-cap sum is Ok, not an off-by-one reject"
        );
    }

    #[test]
    fn to_sdk_zat_boundary_table_pins_the_exact_flip() {
        // §8 (gate-7 — the exact in/out boundary; proptest rarely lands on MAX). 1 and
        // MAX-1 are the just-inside cases the original test (0/MAX/MAX+1/u64::MAX) skipped.
        for ok in [0u64, 1, (MAX_MONEY_ZAT as u64) - 1, MAX_MONEY_ZAT as u64] {
            assert_eq!(
                to_sdk_zat(ok).expect("in-range"),
                zat(ok as i64),
                "{ok} is in range"
            );
        }
        for bad in [
            (MAX_MONEY_ZAT as u64) + 1,
            i64::MAX as u64,
            (i64::MAX as u64) + 1,
            u64::MAX,
        ] {
            assert!(
                matches!(to_sdk_zat(bad), Err(WalletError::StoreCorrupt)),
                "{bad} is corruption, never a clamp/wrap"
            );
        }
    }

    // ── R12 §4.5: the pass's every-pass account doors, under a REAL held lock ──
    //
    // `fetch_summary` (every scan batch) and `primary_account_id` (every pass) are
    // two of the fourteen doors that reach `stall_for` (R12 §3). The lock is the
    // `test_support` fixture: an EXCLUSIVE-mode write transaction on the WAL file,
    // which refuses every other connection's first read with SQLITE_BUSY. The
    // door's own connection is opened only after the lock is held and has read
    // nothing, so it is the door's first statement that meets the lock.

    /// A provisioned testnet wallet with its account imported, every connection to
    /// it CLOSED (a WAL reader keeps its SHARED lock while open), and its path + key.
    fn closed_wallet_with_account(dir: &std::path::Path) -> (std::path::PathBuf, WalletDbKey) {
        let (mut conn, key) = fresh_conn(dir);
        ensure_account(&mut conn, TEST_SEED, &test_birthday()).expect("import");
        drop(conn);
        (dir.join("wallet.db"), key)
    }

    /// R12 §4.5 test 1 (a). A held lock at the every-batch summary read is the
    /// transient `StoreBusy` — never `StoreCorrupt`, which two passes in a row turn
    /// into "restore from your recovery phrase" (R10). And it is transient: the SAME
    /// connection reads once the lock is gone.
    #[test]
    fn fetch_summary_under_a_held_lock_is_store_busy_not_corruption() {
        let dir = tempdir().expect("tempdir");
        let (path, key) = closed_wallet_with_account(dir.path());
        let blocker = crate::test_support::hold_the_wallet_file_locked(&path, &key);
        let wdb = crate::test_support::wallet_conn_that_has_not_read(&path, &key, Network::Test);
        match fetch_summary(&wdb) {
            Err(WalletError::StoreBusy) => {}
            Err(other) => panic!(
                "R12: a held lock at `fetch_summary` must reach the pass as StoreBusy, never the \
                 blind StoreCorrupt; got {other:?}"
            ),
            Ok(_) => panic!("fixture: a read through a held lock cannot succeed"),
        }
        drop(blocker);
        fetch_summary(&wdb).expect(
            "once the lock is gone the same connection reads — the condition was transient and \
             nothing about the wallet was damaged by meeting it",
        );
    }

    /// R12 §4.5 test 1 (b). The pass's account-id read under a held lock is
    /// `StoreBusy`, and the same connection reads the account once it is released.
    #[test]
    fn primary_account_id_under_a_held_lock_is_store_busy_not_corruption() {
        let dir = tempdir().expect("tempdir");
        let (path, key) = closed_wallet_with_account(dir.path());
        let blocker = crate::test_support::hold_the_wallet_file_locked(&path, &key);
        let wdb = crate::test_support::wallet_conn_that_has_not_read(&path, &key, Network::Test);
        match primary_account_id(&wdb) {
            Err(WalletError::StoreBusy) => {}
            other => panic!(
                "R12: a held lock at `primary_account_id` must reach the pass as StoreBusy, never \
                 the blind StoreCorrupt; got {other:?}"
            ),
        }
        drop(blocker);
        assert!(
            primary_account_id(&wdb)
                .expect("reads once released")
                .is_some(),
            "the imported account is there — the lock damaged nothing"
        );
    }

    /// R12 §4.5 test 7 — the open-path invariant AFTER open: a byte flipped on the
    /// pages that hold the `accounts` table and its indexes (never the first page),
    /// read through a swept engine door after a SUCCESSFUL open, is `StoreCorrupt`.
    /// SQLCipher's page HMAC refuses the page with a code that is neither BUSY, FULL
    /// nor IOERR (§4.1), so classifying the door must not turn real damage into
    /// "retrying". The open-time pins (`db.rs`) cover only a header that does not
    /// authenticate.
    #[test]
    fn a_page_flipped_after_open_reads_store_corrupt_through_a_swept_door() {
        let dir = tempdir().expect("tempdir");
        let (path, key) = closed_wallet_with_account(dir.path());
        // Every b-tree of the table (the table and its indexes — the engine reads
        // `uuid` through a covering index, so the table's own root alone is not
        // what the door touches), and the page size.
        let (roots, page_size) = {
            let c = crate::db::open_existing_keyed_connection(&path, &key).expect("open");
            let mut stmt = c
                .prepare("SELECT rootpage FROM sqlite_master WHERE tbl_name = 'accounts'")
                .expect("prepare");
            let roots: Vec<i64> = stmt
                .query_map([], |r| r.get(0))
                .expect("query")
                .collect::<Result<_, _>>()
                .expect("the accounts b-trees' root pages");
            // SQLCipher answers `PRAGMA page_size` as TEXT (`cipher_page_size`).
            let page_size: i64 = c
                .query_row("PRAGMA page_size", [], |r| r.get::<_, String>(0))
                .expect("page size")
                .parse()
                .expect("a numeric page size");
            (roots, page_size)
        };
        assert!(
            !roots.is_empty() && roots.iter().all(|r| *r > 1),
            "fixture: the accounts b-trees sit on pages other than the first ({roots:?})"
        );
        crate::db::verify_wal_folded(&path)
            .expect("fixture: every commit is in the main file once the last connection closed");

        let mut bytes = std::fs::read(&path).expect("read the wallet file");
        for root in &roots {
            let offset = usize::try_from((root - 1) * page_size + page_size / 2).expect("offset");
            assert!(offset < bytes.len(), "fixture: the page is inside the file");
            bytes[offset] ^= 0x01;
        }
        std::fs::write(&path, bytes).expect("write the tampered file");
        // The fixture proves itself: the engine's own query now fails on a raw
        // connection, so what the door returns below is the door's classification.
        {
            let raw = crate::db::open_existing_keyed_connection(&path, &key)
                .expect("fixture: the header still authenticates");
            let read = raw.query_row("SELECT uuid FROM accounts", [], |r| {
                r.get::<_, rusqlite::types::Value>(0)
            });
            // And the code it refuses with is the §4.1 premise: not BUSY, FULL or
            // IOERR (verified against the pinned SQLCipher by the security review) —
            // checked here, so a SQLCipher bump that changes it reds HERE, by name.
            match &read {
                Err(rusqlite::Error::SqliteFailure(e, _)) => {
                    println!(
                        "the damaged page reads as {:?} (extended {})",
                        e.code, e.extended_code
                    );
                    assert!(
                        !matches!(
                            e.code,
                            rusqlite::ErrorCode::DatabaseBusy
                                | rusqlite::ErrorCode::DatabaseLocked
                                | rusqlite::ErrorCode::DiskFull
                                | rusqlite::ErrorCode::SystemIoFailure
                        ),
                        "§4.1 premise: an HMAC failure is not a transient code, got {e:?}"
                    );
                }
                other => panic!(
                    "fixture: the damaged pages must refuse a read inside SQLite, got {other:?}"
                ),
            }
        }

        // The production open runs the migrator, and `init_wallet_db` itself reads
        // the accounts index — so on this file the open is where the damage surfaces,
        // and it must surface as StoreCorrupt there too (the migrate unwrap's default).
        assert!(
            matches!(
                crate::db::open_db_migrated(&path, &key, Network::Test, None).err(),
                Some(WalletError::StoreCorrupt)
            ),
            "a damaged page the migrator reads fails the open as StoreCorrupt"
        );
        // AFTER a successful open: the open's guard layers (`db::open_db` — no-CREATE,
        // the key, the forced probe, the provisioning sentinel) all pass on this file;
        // the engine is wrapped over that connection without the migrator, and the
        // swept door reads the damaged pages.
        let conn = crate::db::open_db(&path, &key).expect(
            "fixture: the open succeeds — the header, the schema and the sentinel are intact; \
             only the accounts pages are damaged",
        );
        let wdb = zcash_client_sqlite::WalletDb::from_connection(
            conn,
            Network::Test.consensus(),
            zcash_client_sqlite::util::SystemClock,
            rand_core::OsRng,
        );
        match primary_account_id(&wdb) {
            Err(WalletError::StoreCorrupt) => {}
            other => panic!(
                "R12 §4.1 open-path invariant, after open: a page that fails its HMAC is damage \
                 and must stay StoreCorrupt through a classified door; got {other:?}"
            ),
        }
    }

    proptest::proptest! {
        // §8 (testing-patterns #2 — property coverage over the money space; every fold
        // invariant is otherwise pinned at a single hand-picked point). Per-field values
        // are bounded so neither a per-account total nor the 4-account cross-sum can
        // exceed MAX_MONEY (≤ 7 fields/account × 4 × MAX/32 < MAX) — staying on the
        // in-supply Ok path; the overflow path is the explicit StoreCorrupt tests.

        /// fold_account_balances is a faithful, total-preserving, non-negative aggregation.
        #[test]
        fn fold_balance_properties_hold_over_the_money_space(
            accts in proptest::collection::vec(
                (
                    (
                        0u64..=(MAX_MONEY_ZAT as u64 / 32),
                        0u64..=(MAX_MONEY_ZAT as u64 / 32),
                        0u64..=(MAX_MONEY_ZAT as u64 / 32),
                    ),
                    (
                        0u64..=(MAX_MONEY_ZAT as u64 / 32),
                        0u64..=(MAX_MONEY_ZAT as u64 / 32),
                        0u64..=(MAX_MONEY_ZAT as u64 / 32),
                    ),
                    0u64..=(MAX_MONEY_ZAT as u64 / 32),
                ),
                1..=4usize,
            ),
        ) {
            let balances: Vec<AccountBalance> = accts
                .iter()
                .map(|&(s, o, t)| account_balance(s, o, t))
                .collect();
            let snap = fold_account_balances(balances.iter()).expect("in-supply fold is Ok");
            // non-negativity (structurally guaranteed by Zatoshis; a regression net).
            for v in [
                snap.spendable,
                snap.pending_incoming,
                snap.pending_change,
                snap.transparent,
                snap.total,
            ] {
                proptest::prop_assert!(v.zat() >= 0);
            }
            // the universal reconciliation identity (upstream total() == the three pool
            // sub-states summed; uneconomic dust excluded on both sides).
            let summed = snap
                .spendable
                .checked_add(snap.pending_change)
                .and_then(|x| x.checked_add(snap.pending_incoming))
                .and_then(|x| x.checked_add(snap.transparent));
            proptest::prop_assert_eq!(summed, Some(snap.total));
            // monotonic-sum: folding the full set never shrinks a bucket vs the first alone.
            if let Some(first) = balances.first() {
                let one = fold_account_balances([first].into_iter()).expect("one");
                proptest::prop_assert!(snap.spendable.zat() >= one.spendable.zat());
                proptest::prop_assert!(snap.total.zat() >= one.total.zat());
            }
        }

        /// The spendable SSOT: spendable_ready ⟺ balance.spendable > 0, over arbitrary
        /// multi-account summaries (random key placement ⇒ order-independent too).
        #[test]
        fn spendable_ready_iff_spendable_positive_over_arbitrary_summaries(
            accts in proptest::collection::vec(
                (
                    0u64..=(MAX_MONEY_ZAT as u64 / 32), // shielded spendable (sapling)
                    0u64..=(MAX_MONEY_ZAT as u64 / 32), // shielded pending  (sapling value_pending)
                    0u64..=(MAX_MONEY_ZAT as u64 / 32), // transparent
                ),
                1..=4usize,
            ),
        ) {
            let balances: Vec<AccountBalance> = accts
                .iter()
                .map(|&(sp, pend, t)| account_balance((sp, 0, pend), (0, 0, 0), t))
                .collect();
            let summary = summary_of(balances, Ratio::new(1, 2));
            let progress = progress_from(&summary);
            let balance = balance_from(&summary).expect("maps");
            proptest::prop_assert_eq!(
                progress.spendable_ready,
                balance.spendable != Zatoshis::ZERO
            );
        }

        /// to_sdk_zat is total over u64 and fail-closed at the supply bound.
        #[test]
        fn to_sdk_zat_is_total_and_fail_closed_over_u64(raw in proptest::prelude::any::<u64>()) {
            match to_sdk_zat(raw) {
                Ok(z) => {
                    proptest::prop_assert!(raw <= MAX_MONEY_ZAT as u64);
                    proptest::prop_assert_eq!(z.zat() as u64, raw); // exact round-trip
                }
                Err(_) => proptest::prop_assert!(raw > MAX_MONEY_ZAT as u64),
            }
        }
    }
}
