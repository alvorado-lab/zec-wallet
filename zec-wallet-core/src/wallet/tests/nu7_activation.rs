//! The NU7 wave's step 4, the activation check (`docs/plan/nu7-wave.md` §3 step 4;
//! ADR-0579 Decisions 4 and 6): the TEST AUTHOR's rows that compile at the base
//! commit, written blind to the implementation (IT-2a) against the contract.
//!
//! **The claim.** A transaction is signed under the branch of its target height and
//! can be mined only in blocks of that branch (ZIP 259). So a send signed before
//! the activation `H` and not mined by `H - 1` is dead from the tip `H - 1`, whatever
//! its stored expiry says, and every place the wallet decides "can this still be
//! mined" must say so from that block. Each row here runs on a chain that
//! STRADDLES the activation (`LocalNetwork { nu7: Some(H), ..ironwood_network() }`,
//! the seam the contract names), through the readers a user's screen and the
//! wallet's money decisions read, never a private helper.
//!
//! **The join points.** Four of the five readers take no consensus parameters at
//! the base, and the kernel cannot judge a branch without them, so the implementer
//! will change their signatures. Every call of a reader goes through ONE function in
//! the `join` module below, written here against the base signatures (the params
//! passed and unused); the adjudicator re-points each one at the built signature
//! and touches nothing else. The new `TxStatus` variant ("cancelled by the network
//! upgrade") is read the same way, through [`join::reading`], whose catch-all maps
//! anything it does not know to `Reading::Other`, so a join that forgets the new
//! variant stays red rather than passing.
//!
//! The rows that name a symbol the implementer ADDS (the kernel, its answers, the
//! phase, the two heights, the burial predicate) are the sibling
//! `nu7_activation_named.rs`, never built until the join.

use std::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;
use zcash_address::{ToAddress, ZcashAddress};
use zcash_client_backend::data_api::testing::TestBuilder;
use zcash_client_backend::data_api::{TransactionDataRequest, TransactionStatus, WalletWrite};
use zcash_primitives::transaction::Transaction;
use zcash_protocol::consensus::{BlockHeight, BranchId, NetworkType};
use zcash_protocol::local_consensus::LocalNetwork;
use zcash_protocol::value::Zatoshis as ProtoZat;

use crate::delivery;
use crate::enhance::{
    CheckedStatus, EnhancementStore, FetchedTransaction, TransactionFetcher, UnparseableTxids,
};
use crate::error::WalletError;
use crate::intent_store::{self, NoteClaim};
use crate::net::grpc::GrpcError;
use crate::send::Reconciled;
use crate::state::{DeliveryState, TxStatus, TxSummary};
use crate::test_support::HistoryHarness;

/// The NU7 activation on the straddling chain. The harness's first block is its
/// Sapling activation (100,000), so this leaves room for a note to mature (10
/// blocks) and a send to be built at `H - 10`.
pub(super) const H: u32 = 100_060;

/// ZIP 218's wall-clock target for an irreversible burial (ADR-0579 Decision 6):
/// 600 blocks at 25 s once every block counted is at or above `H`.
pub(super) const BURIAL_SECS: u64 = 15_000;

/// The burial depth after the activation, in blocks: 15,000 s at 25 s. Pinned
/// against the spacing function in [`the_burial_premise_is_600_blocks_after_the_activation`].
pub(super) const BURIAL_BLOCKS_AFTER_H: u32 = 600;

pub(super) const NOTE_VALUE: u64 = 500_000;
pub(super) const AMOUNT: u64 = 20_000;

/// The straddling chain: every upgrade from Sapling to NU6.3 at the harness's one
/// height, as `ironwood_spendability`'s `ironwood_network`, and NU7 at [`H`]. A send
/// targeting a height below `H` is signed under `Nu6_3`, the branch mainnet runs
/// before the activation, and `Nu6_3`'s `height_bounds` end at `H`.
pub(super) fn straddling() -> LocalNetwork {
    let below = TestBuilder::<(), ()>::DEFAULT_NETWORK.nu5;
    LocalNetwork {
        nu6: below,
        nu6_1: below,
        nu6_2: below,
        nu6_3: below,
        nu7: Some(BlockHeight::from_u32(H)),
        ..TestBuilder::<(), ()>::DEFAULT_NETWORK
    }
}

pub(super) fn recipient() -> ZcashAddress {
    ZcashAddress::from_transparent_p2pkh(NetworkType::Regtest, [0x71; 20])
}

/// The wall clock these rows run on: a fixed instant, never the system clock (the
/// injected-time rule). Every intent here is an ordinary send, whose deposit gate never
/// reads the clock, so the readers that take one (`state_of`, the history row, the
/// accepted mark, the reconcile) only need a plausible time, above
/// `CLOCK_PLAUSIBILITY_FLOOR_SECS`, and the same one on every run.
pub(super) fn now_unix() -> u64 {
    const FIXED_NOW_UNIX: u64 = 1_793_000_000;
    const _: () = assert!(FIXED_NOW_UNIX > crate::constants::CLOCK_PLAUSIBILITY_FLOOR_SECS);
    FIXED_NOW_UNIX
}

/// A wallet's persisted bytes for `txid` (INTERNAL order).
pub(super) fn raw_of(conn: &Connection, txid: &zcash_protocol::TxId) -> Vec<u8> {
    conn.query_row(
        "SELECT raw FROM transactions WHERE txid = ?1",
        rusqlite::params![txid.as_ref()],
        |r| r.get(0),
    )
    .expect("the created send has its bytes")
}

/// The consensus branch id at bytes 8 to 12 of a v5 or v6 header.
pub(super) fn header_branch_bytes(raw: &[u8]) -> u32 {
    u32::from_le_bytes(raw[8..12].try_into().expect("a whole header"))
}

/// Mine empty blocks to `tip` and scan them, so the scanned tip (what history and
/// delivery read) and the chain tip (what the resubmission pass reads) agree.
pub(super) fn advance_to(harness: &mut HistoryHarness, tip: u32) {
    let now = harness.current_tip();
    assert!(tip >= now, "advance_to moves forward only ({now} to {tip})");
    harness.mine_empty((tip - now) as usize);
    harness.scan();
    assert_eq!(harness.current_tip(), tip);
}

/// Move only the CHAIN tip (what `rebroadcast_group` judges against), the way the
/// existing burial rows do: burial reads the mined or expiry height against the tip,
/// so the blocks between need not be scanned.
pub(super) fn chain_tip_to(harness: &mut HistoryHarness, tip: u32) {
    harness
        .state_mut()
        .wallet_mut()
        .update_chain_tip(BlockHeight::from_u32(tip))
        .expect("update the chain tip");
}

/// A funded wallet on the straddling chain with ONE real single-step send signed
/// over the product path at target `H - 10` (the tip `H - 11`), so its stored
/// expiry is `H + 30` and its branch is `Nu6_3`; plus the second connection the
/// aux readers ride, with the intent and accepted-mark tables on it.
///
/// The tip `H - 11` is inside the pre-upgrade pause's window (ADR-0580), so this SDK
/// refuses that send today; the fixture builds it with the pause suspended
/// (`send::upgrade_pause_testing`). It stands for the transaction the readers must
/// still judge: one signed inside the window by an older build, or by another wallet
/// on the same seed, still pending at the activation.
pub(super) struct Killed {
    pub(super) harness: HistoryHarness,
    pub(super) network: LocalNetwork,
    pub(super) send: zcash_protocol::TxId,
    pub(super) claims: Vec<NoteClaim>,
    pub(super) conn: Connection,
}

pub(super) fn a_send_built_ten_blocks_below_the_activation() -> Killed {
    let network = straddling();
    let mut harness = HistoryHarness::on_network(network);
    harness.mine_received(NOTE_VALUE);
    advance_to(&mut harness, H - 11);
    let (send, claims) = {
        let _inside_the_window = crate::send::upgrade_pause_testing::suspend();
        harness.create_send_with_claims(recipient(), AMOUNT)
    };
    let conn = harness.read_conn();
    intent_store::ensure_table(&conn).expect("the intent table on the same file");
    delivery::ensure_table(&conn).expect("the accepted-mark table on the same file");

    // The premises the contract states, read off the transaction, never assumed.
    assert_eq!(
        harness.expiry_height_of(send),
        H + 30,
        "premise: a send targeting H - 10 carries the builder's pre-NU7 expiry, H + 30"
    );
    let raw = raw_of(&conn, &send);
    assert_eq!(
        header_branch_bytes(&raw),
        u32::from(BranchId::Nu6_3),
        "premise: it is signed under Nu6_3, the branch the activation ends"
    );
    let parsed = Transaction::read(&raw[..], BranchId::Nu6_3).expect("our own bytes parse");
    assert_eq!(
        parsed.consensus_branch_id(),
        BranchId::Nu6_3,
        "premise: the parsed transaction agrees with its header bytes"
    );
    assert_eq!(
        BranchId::Nu6_3
            .height_bounds(&network)
            .and_then(|(_, upper)| upper),
        Some(BlockHeight::from_u32(H)),
        "premise: the crate's own bounds end Nu6_3 at H on this chain"
    );
    Killed {
        harness,
        network,
        send,
        claims,
        conn,
    }
}

/// What a history row reads, mapped into this file's own words so a row can be
/// asserted before the new variant exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Reading {
    Pending,
    Expired,
    Confirmed,
    /// The contract's "cancelled by the network upgrade" reading.
    CancelledByUpgrade,
    /// Anything else, including a variant [`join::reading`] does not map yet.
    Other,
}

/// THE JOIN POINTS: every call of a reader whose signature the step changes, and
/// the one read of the new `TxStatus` variant. Written against the base; the
/// adjudicator re-points each at the built name and signature.
pub(super) mod join {
    use super::*;

    /// Reader 1, `send::rebroadcast_group`, through its one caller
    /// `send::reconcile_inflight` (the resubmission pass's reconcile of one row).
    pub(in crate::wallet::tests) fn reconcile(
        harness: &HistoryHarness,
        network: &LocalNetwork,
        conn: &mut Connection,
        now: u64,
    ) -> Reconciled {
        let row = intent_store::list_in_flight(conn).expect("in flight")[0].clone();
        crate::send::reconcile_inflight(harness.wallet(), network, conn, &row, now)
            .expect("reconcile")
    }

    /// Reader 2, `delivery::enumerate`, against the scanned tip as the drain reads it.
    pub(in crate::wallet::tests) fn enumerated(
        conn: &Connection,
        network: &LocalNetwork,
    ) -> Vec<[u8; 32]> {
        let tip = crate::history::scanned_tip(conn).expect("tip");
        delivery::enumerate(conn, network, tip)
            .expect("enumerate")
            .into_iter()
            .map(|o| o.txid)
            .collect()
    }

    /// Reader 3, `DeliveryView::state_of`, as `Wallet::delivery_state` reads it.
    pub(in crate::wallet::tests) fn state_of(
        conn: &Connection,
        network: &LocalNetwork,
        now: u64,
        txid: &zcash_protocol::TxId,
    ) -> Option<DeliveryState> {
        let tip = crate::history::scanned_tip(conn).expect("tip");
        delivery::DeliveryView::load(conn)
            .expect("view")
            .state_of(conn, network, tip, now, txid.as_ref())
            .expect("state_of")
    }

    /// Reader 4, the history page, as `Wallet::transactions` assembles it: the page,
    /// then the delivery annotation on the same connection.
    pub(in crate::wallet::tests) fn history_row(
        conn: &Connection,
        network: &LocalNetwork,
        now: u64,
        txid: &zcash_protocol::TxId,
    ) -> TxSummary {
        let mut page = crate::history::list_transactions(conn, network, 100, None).expect("page");
        // `wallet::annotate_delivery` is built over the concrete `WalletParams`, which a
        // straddling `LocalNetwork` is not, so its body runs here as written: one view, and
        // each row's `state_of` against the scanned tip, on the page's connection.
        let tip = crate::history::scanned_tip(conn).expect("tip");
        let view = delivery::DeliveryView::load(conn).expect("view");
        for row in &mut page.rows {
            row.delivery = view
                .state_of(conn, network, tip, now, row.txid.to_internal().as_ref())
                .expect("annotate");
        }
        let display: crate::TxId = (*txid).into();
        page.rows
            .into_iter()
            .find(|r| r.txid == display)
            .expect("the send's history row")
    }

    /// The new `TxStatus` variant, read. At the join, map the built variant (with its
    /// two heights and phase, whatever it carries) to `Reading::CancelledByUpgrade`.
    pub(in crate::wallet::tests) fn reading(status: TxStatus) -> Reading {
        match status {
            TxStatus::Pending => Reading::Pending,
            TxStatus::Expired => Reading::Expired,
            TxStatus::Confirmed { .. } => Reading::Confirmed,
            TxStatus::CancelledByUpgrade { .. } => Reading::CancelledByUpgrade,
            _ => Reading::Other,
        }
    }
}

/// Enrol a live `Sent` intent row owning `send`, the shape the resubmission pass
/// reconciles, and return its id.
pub(super) fn enrol_sent(
    conn: &mut Connection,
    claims: &[NoteClaim],
    send: &zcash_protocol::TxId,
) -> crate::state::QueuedSendId {
    let id = intent_store::enqueue(conn, "zcash:owner", 0, None, None).expect("enqueue");
    assert!(intent_store::mark_submitting(conn, id, claims).expect("submitting"));
    assert!(intent_store::mark_sent_multi(conn, id, &[*send.as_ref()]).expect("sent"));
    id
}

/// Reader 5, `enhance::chain_status`, driven through the production enhancement
/// pass (`run_enhancement_pass`, where the evidence is built from the parsed
/// transaction), never the pure function with hand-built evidence: a `GetStatus`
/// request for `txid`, an endpoint that answers with the transaction's real bytes
/// and claims it was mined at `claimed`, and a known tip of `known_tip`. Returns
/// the status the pass wrote, `None` when it refused the claim.
pub(super) fn chain_status_writes(
    network: &LocalNetwork,
    raw: &[u8],
    txid: zcash_protocol::TxId,
    claimed: u32,
    known_tip: u32,
) -> Option<TransactionStatus> {
    struct OneStatus {
        txid: zcash_protocol::TxId,
        tip: BlockHeight,
        written: Mutex<Vec<TransactionStatus>>,
    }
    #[async_trait]
    impl EnhancementStore for OneStatus {
        async fn pending_requests(&self) -> Result<Vec<TransactionDataRequest>, WalletError> {
            Ok(vec![TransactionDataRequest::GetStatus(self.txid)])
        }
        async fn chain_tip(&self) -> Result<Option<BlockHeight>, WalletError> {
            Ok(Some(self.tip))
        }
        async fn set_status(
            &self,
            _txid: zcash_protocol::TxId,
            status: CheckedStatus,
        ) -> Result<(), WalletError> {
            self.written.lock().expect("written").push(status.get());
            Ok(())
        }
        async fn mined_height_of(
            &self,
            _txid: zcash_protocol::TxId,
        ) -> Result<Option<BlockHeight>, WalletError> {
            // The wallet holds no height of its own: the case a status request asks about.
            Ok(None)
        }
        async fn wallet_birthday(&self) -> Result<Option<BlockHeight>, WalletError> {
            Ok(TestBuilder::<(), ()>::DEFAULT_NETWORK.sapling)
        }
        async fn store_decrypted(
            &self,
            _tx: Transaction,
            _mined_height: Option<BlockHeight>,
        ) -> Result<(), WalletError> {
            panic!("a status request is answered with a status, never a decrypt")
        }
    }
    struct Claims {
        data: Vec<u8>,
        height: u64,
    }
    #[async_trait]
    impl TransactionFetcher for Claims {
        async fn fetch_transaction(
            &mut self,
            _txid: zcash_protocol::TxId,
        ) -> Result<Option<FetchedTransaction>, GrpcError> {
            Ok(Some(FetchedTransaction {
                data: self.data.clone(),
                height: self.height,
            }))
        }
    }
    let store = OneStatus {
        txid,
        tip: BlockHeight::from_u32(known_tip),
        written: Mutex::new(Vec::new()),
    };
    let mut fetcher = Claims {
        data: raw.to_vec(),
        height: u64::from(claimed),
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("runtime");
    let outcome = runtime
        .block_on(crate::enhance::run_enhancement_pass(
            &store,
            &mut fetcher,
            network,
            50,
            &|| {},
            &UnparseableTxids::default(),
        ))
        .expect("the pass runs");
    assert_eq!(
        outcome.requested, 1,
        "premise: the one status request was asked"
    );
    let written = store.written.into_inner().expect("written");
    assert!(written.len() <= 1, "one request, at most one status");
    written.into_iter().next()
}

/// Every reading at one tip, the scanned tip and the chain tip agreeing. The
/// generic readers run with NO intent row (an ordinary send the delivery
/// obligation owns); the intent-owned ones with a live `Sent` row owning the send
/// and an accepted mark on it, which is the fixture where the kernel and a
/// stored-expiry reader disagree for `state_of` (the contract's M-4: a generic
/// obligation reads `None` dead and `RetryPending` live; an accepted intent-owned
/// one reads `RetryPending` dead and `Accepted` live).
#[derive(Debug, PartialEq, Eq)]
struct Readings {
    enumerated: bool,
    generic_state: Option<DeliveryState>,
    history: Reading,
    owned_state: Option<DeliveryState>,
    rebroadcasts: bool,
    reconciled: String,
}

fn readings_at(k: &mut Killed, tip: u32) -> Readings {
    advance_to(&mut k.harness, tip);
    let now = now_unix();
    let enumerated = join::enumerated(&k.conn, &k.network).contains(k.send.as_ref());
    let generic_state = join::state_of(&k.conn, &k.network, now, &k.send);
    let history = join::reading(join::history_row(&k.conn, &k.network, now, &k.send).status);

    let id = enrol_sent(&mut k.conn, &k.claims, &k.send);
    delivery::mark_accepted(&k.conn, k.send.as_ref(), Some(tip), now).expect("mark accepted");
    let owned_state = join::state_of(&k.conn, &k.network, now, &k.send);
    let reconciled = join::reconcile(&k.harness, &k.network, &mut k.conn, now);
    // Leave the file as it was: no row, no mark, for the next tip's generic readers.
    assert!(intent_store::delete(&mut k.conn, id).expect("delete"));
    k.conn
        .execute(
            "DELETE FROM tx_delivery_accepted WHERE txid = ?1",
            rusqlite::params![k.send.as_ref()],
        )
        .expect("clear the mark");
    Readings {
        enumerated,
        generic_state,
        history,
        owned_state,
        rebroadcasts: matches!(reconciled, Reconciled::Rebroadcast { .. }),
        reconciled: format!("{reconciled:?}"),
    }
}

/// **`every_expiry_reader_agrees_a_send_the_upgrade_killed_is_dead_from_the_block_before_it`**
/// (`docs/plan/nu7-wave.md` §3 step 4; ADR-0579 Decision 4, A-F3, M-4).
///
/// On the straddling chain, a send built at target `H - 10` (stored expiry `H + 30`):
/// - at the tip `H - 2` every reader reads it as live: `enumerate` lists it,
///   `state_of` reads it owed (the generic obligation `RetryPending`, the accepted
///   intent-owned one `Accepted`), `rebroadcast_group` re-broadcasts, and history
///   reads `Pending`;
/// - at the tip `H - 1` every reader reads it past its expiry: `enumerate` drops it,
///   the generic `state_of` reads `None` and the intent-owned one `RetryPending`
///   (the live intent will send the payment again), `rebroadcast_group` stops
///   re-broadcasting and waits for burial, and history reads "cancelled by the
///   network upgrade";
/// - `chain_status`, with a known tip at `H + 30` so its tip bound refuses nothing
///   (it refuses `h > known_tip` before any expiry), accepts "mined at `H - 1`" and
///   refuses "mined at `H`" and "mined at `H + 30`": a server cannot confirm a
///   payment in a block where the payment cannot exist.
///
/// Mutants, one per reader, each keeping the stored expiry: each red, at the `H - 1`
/// row of its own reader (or `chain_status`'s `H` claim).
#[test]
fn every_expiry_reader_agrees_a_send_the_upgrade_killed_is_dead_from_the_block_before_it() {
    let mut k = a_send_built_ten_blocks_below_the_activation();

    let live = readings_at(&mut k, H - 2);
    assert_eq!(
        live,
        Readings {
            enumerated: true,
            generic_state: Some(DeliveryState::RetryPending),
            history: Reading::Pending,
            owned_state: Some(DeliveryState::Accepted),
            rebroadcasts: true,
            reconciled: live.reconciled.clone(),
        },
        "at the tip H - 2 the next block is H - 1, still Nu6_3: the send can be mined, \
         and every reader says so"
    );

    let dead = readings_at(&mut k, H - 1);
    assert!(
        !dead.enumerated,
        "enumerate: at the tip H - 1 the next block is under NU7 and the send can never \
         be mined; the delivery obligation must not broadcast it again ({dead:?})"
    );
    assert_eq!(
        dead.generic_state, None,
        "state_of, generic obligation: a dead send with no intent behind it is owed no \
         broadcast ({dead:?})"
    );
    assert_eq!(
        dead.owned_state,
        Some(DeliveryState::RetryPending),
        "state_of, accepted intent-owned: this attempt is dead and the live intent will \
         send the payment again, which is what retry-pending says; `Accepted` would \
         say it is on its way ({dead:?})"
    );
    assert!(
        !dead.rebroadcasts,
        "rebroadcast_group: from the tip H - 1 the send is rebuilt, never re-broadcast \
         ({dead:?})"
    );
    assert_eq!(
        dead.reconciled,
        format!("{:?}", Reconciled::AwaitingBurial),
        "rebroadcast_group: dead but not buried, so it waits for burial before the \
         requeue ({dead:?})"
    );
    assert_eq!(
        dead.history,
        Reading::CancelledByUpgrade,
        "history: the row reads cancelled by the network upgrade from the tip H - 1, \
         while upstream's expired_unmined is still false ({dead:?})"
    );

    // Reader 5. A known tip at H + 30, the stored expiry, so the tip bound refuses no
    // claim here and only the expiry bound can.
    let raw = raw_of(&k.conn, &k.send);
    assert_eq!(
        chain_status_writes(&k.network, &raw, k.send, H - 1, H + 30),
        Some(TransactionStatus::Mined(BlockHeight::from_u32(H - 1))),
        "chain_status: mined at H - 1, the last Nu6_3 block, is possible and accepted"
    );
    for claimed in [H, H + 30] {
        assert_eq!(
            chain_status_writes(&k.network, &raw, k.send, claimed, H + 30),
            None,
            "chain_status: a claim that the send was mined at {claimed} (H + {}) is \
             refused: a Nu6_3 transaction cannot be in a block under NU7, and accepting \
             it would read the cancelled payment as confirmed and delete its intent",
            claimed - H
        );
    }
}

/// **The premise the burial rows rest on**, pinned against the spacing function
/// step 4s landed (`crate::chain_time`), so a row's `600` is the contract's number
/// and the spacing function's at once: from the block `H - 1`, every block counted
/// is at or above `H` (25 s each), and 15,000 s is reached at exactly 600 blocks.
#[test]
fn the_burial_premise_is_600_blocks_after_the_activation() {
    let network = straddling();
    let from = BlockHeight::from_u32(H - 1);
    assert_eq!(
        crate::chain_time::chain_time_between(&network, from, from + BURIAL_BLOCKS_AFTER_H),
        BURIAL_SECS
    );
    assert_eq!(
        crate::chain_time::chain_time_between(&network, from, from + (BURIAL_BLOCKS_AFTER_H - 1)),
        BURIAL_SECS - 25
    );
}

/// **`the_requeue_after_an_activation_waits_for_irreversible_burial`** (ADR-0579
/// Decision 6; the plan's consumer table: requeue follows the predicate). The send
/// the upgrade killed is dead from the tip `H - 1` (its effective expiry), and the
/// requeue (which sends the payment again, possibly from other notes) waits until
/// that block is irreversibly buried: 15,000 s of chain time, 600 blocks at 25 s, so
/// the tip `H - 1 + 600`. One block short of it the row still waits.
///
/// Mutants, each red: requeue at the bare effective expiry (red at the tip `H - 1`);
/// measure from the stored expiry `H + 30` (red at `H + 599`); read
/// `REORG_MAX_BLOCKS` in place of the predicate (red at `H + 99`).
#[test]
fn the_requeue_after_an_activation_waits_for_irreversible_burial() {
    let Killed {
        mut harness,
        network,
        send,
        claims,
        mut conn,
    } = a_send_built_ten_blocks_below_the_activation();
    enrol_sent(&mut conn, &claims, &send);
    let now = now_unix();
    let buried_at = H - 1 + BURIAL_BLOCKS_AFTER_H;
    for (tip, why) in [
        (H - 1, "the bare effective expiry: dead, not buried"),
        (
            H + 99,
            "the 100 blocks REORG_MAX_BLOCKS counts: not the predicate's depth",
        ),
        (
            H + 30 + 100,
            "the stored expiry's 100-block burial: not this send's expiry",
        ),
        (buried_at - 1, "one block short of 15,000 s after H - 1"),
    ] {
        chain_tip_to(&mut harness, tip);
        assert_eq!(
            join::reconcile(&harness, &network, &mut conn, now),
            Reconciled::AwaitingBurial,
            "at the tip {tip} ({why}) the dead send waits; a requeue now could pay twice \
             if a reorg revived it"
        );
    }
    chain_tip_to(&mut harness, buried_at);
    assert_eq!(
        join::reconcile(&harness, &network, &mut conn, now),
        Reconciled::Requeued,
        "at the tip H - 1 + 600 the block before the activation is irreversibly buried: \
         the payment is sent again"
    );
}

/// **`a_reorg_below_the_activation_resumes_the_rebroadcast_and_never_requeues`**
/// (`docs/plan/nu7-wave.md` §3 step 4). "Dead from `H - 1`" is a fact about the
/// branch the next block belongs to, so it holds only while the tip stays at or
/// above `H - 1`. A reorg back to `H - 2` makes the send mineable again, and every
/// reader must say so: the resubmission pass re-broadcasts it, the delivery
/// obligation lists it and reads it owed, history reads it `Pending`. Nothing may
/// remember "cancelled".
///
/// Every reader is read while the send is dead (at `H + 5`) BEFORE the reorg, so a
/// latch set by that reading is in place when the reorg happens.
///
/// Mutant: a durable "cancelled" latch in any reader (`enumerate`, `state_of`, the
/// history page, the resubmission pass). Red.
#[test]
fn a_reorg_below_the_activation_resumes_the_rebroadcast_and_never_requeues() {
    let Killed {
        mut harness,
        network,
        send,
        claims,
        mut conn,
    } = a_send_built_ten_blocks_below_the_activation();
    let now = now_unix();
    advance_to(&mut harness, H + 5);
    assert!(
        !join::enumerated(&conn, &network).contains(send.as_ref()),
        "premise: above the activation the delivery obligation does not list the dead send"
    );
    assert_eq!(
        join::state_of(&conn, &network, now, &send),
        None,
        "premise: above the activation a dead send with no intent behind it is owed nothing"
    );
    assert_eq!(
        join::reading(join::history_row(&conn, &network, now, &send).status),
        Reading::CancelledByUpgrade,
        "premise: above the activation the send reads cancelled"
    );
    let id = enrol_sent(&mut conn, &claims, &send);
    assert_eq!(
        join::reconcile(&harness, &network, &mut conn, now),
        Reconciled::AwaitingBurial,
        "premise: above the activation the pass waits for burial"
    );
    assert!(intent_store::delete(&mut conn, id).expect("delete"));

    harness.reorg_to_height(H - 2);
    assert!(
        join::enumerated(&conn, &network).contains(send.as_ref()),
        "after a reorg to H - 2 the delivery obligation owes the broadcast again"
    );
    assert_eq!(
        join::state_of(&conn, &network, now, &send),
        Some(DeliveryState::RetryPending),
        "after a reorg to H - 2 the send reads owed again"
    );
    assert_eq!(
        join::reading(join::history_row(&conn, &network, now, &send).status),
        Reading::Pending,
        "after a reorg to H - 2 history reads the send as on its way again"
    );
    enrol_sent(&mut conn, &claims, &send);
    assert!(
        matches!(
            join::reconcile(&harness, &network, &mut conn, now),
            Reconciled::Rebroadcast { .. }
        ),
        "after a reorg to H - 2 the resubmission pass re-broadcasts the send"
    );
    // And it never requeues on the strength of the earlier reading: far past where a
    // burial counted from H - 1 would have ended, a tip at H - 2 is still live.
    assert_eq!(
        intent_store::list_in_flight(&conn)
            .expect("in flight")
            .len(),
        1,
        "the row is still Sent: nothing was requeued"
    );
}

/// **`history_says_cancelled_by_upgrade_even_after_upstream_says_expired`**
/// (ADR-0579 Decision 4, reader 4; A-F3). Upstream's view column
/// `expired_unmined` flips when the scanned tip reaches the STORED expiry (`H + 30`;
/// `expiry_height BETWEEN 1 AND max_height`). The row reads cancelled by the upgrade
/// on both sides of that flip: the cancelled reading is the truer one (the network
/// stopped accepting it at `H`, not at `H + 30`), and it wins over plain `Expired`.
///
/// Mutant: let `expired_unmined` win. Red at `H + 30`.
#[test]
fn history_says_cancelled_by_upgrade_even_after_upstream_says_expired() {
    let Killed {
        mut harness,
        network,
        send,
        conn,
        ..
    } = a_send_built_ten_blocks_below_the_activation();
    let now = now_unix();
    for tip in [H + 29, H + 30, H + 31] {
        advance_to(&mut harness, tip);
        let upstream_expired: bool = conn
            .query_row(
                "SELECT IFNULL(expired_unmined, 0) FROM v_transactions WHERE txid = ?1",
                rusqlite::params![send.as_ref()],
                |r| r.get(0),
            )
            .expect("the view's row");
        assert_eq!(
            upstream_expired,
            tip >= H + 30,
            "premise: upstream's column flips at the stored expiry, H + 30"
        );
        assert_eq!(
            join::reading(join::history_row(&conn, &network, now, &send).status),
            Reading::CancelledByUpgrade,
            "at the scanned tip {tip} the row reads cancelled by the network upgrade, \
             whatever upstream's expired_unmined says ({upstream_expired})"
        );
    }
}

/// **`a_live_submitting_claim_reads_retry_pending_after_the_activation`** (S-M3 (c)).
/// A `Submitting` row whose claim owns the send (the create-committed-but-unrecorded
/// window: the transaction exists, no row recorded its txid) is a live intent: it
/// re-proposes once the stored expiry frees its notes. So once the upgrade kills the
/// attempt, the honest reading is `RetryPending` (the wallet WILL send the payment
/// again; the user must not), never `None`, which a host renders as "cancelled, the
/// amount is yours" and which invites a second payment by hand.
///
/// Mutant: `None`. Red.
#[test]
fn a_live_submitting_claim_reads_retry_pending_after_the_activation() {
    let Killed {
        mut harness,
        network,
        send,
        claims,
        mut conn,
    } = a_send_built_ten_blocks_below_the_activation();
    let id = intent_store::enqueue(&mut conn, "zcash:owner", 0, None, None).expect("enqueue");
    assert!(intent_store::mark_submitting(&mut conn, id, &claims).expect("submitting"));
    let now = now_unix();

    advance_to(&mut harness, H - 2);
    assert_eq!(
        join::state_of(&conn, &network, now, &send),
        Some(DeliveryState::Persisted),
        "control at H - 2: the unrecorded window, kept and not retried by this phase"
    );
    for tip in [H - 1, H + 30] {
        advance_to(&mut harness, tip);
        assert_eq!(
            join::state_of(&conn, &network, now, &send),
            Some(DeliveryState::RetryPending),
            "at the tip {tip} the attempt is dead and the live Submitting row will send \
             the payment again: retry-pending, never None"
        );
    }
}

/// **`a_proposal_that_straddles_the_activation_is_stale_at_signing`** (S-B1; the
/// signing-time half of ADR-0579 Decision 4, which ADR-0580 keeps). Upstream builds at
/// the proposal's target height, so a proposal targeted below `H` and signed once the
/// tip has reached `H - 1` would be signed under `Nu6_3` for a chain whose next block
/// is under NU7: dead on arrival, its notes reserved until its stored expiry. The guard
/// refuses it with `ProposalStale`, and a fresh proposal is signed under NU7 (a v6
/// transaction, branch `0x77190AD9`).
///
/// The pre-upgrade pause (ADR-0580) refuses every send from the tip `H - 21` to
/// `H - 2`, so the proposal the branch clause still has to catch is the one made at
/// the last tip the pause allows, `H - 22` (target `H - 21`), and signed at `H - 1`.
/// There the drift is exactly 20, which the drift bound lets through, and the next 21
/// blocks are all NU7's, which the pause lets through: only the branch clause refuses.
/// The other side, a case the contract does not name: the same proposal signed at
/// once, at `H - 22`, signs under `Nu6_3`.
///
/// Mutant: drop the branch clause. Red.
#[test]
fn a_proposal_that_straddles_the_activation_is_stale_at_signing() {
    use crate::consensus::SigningPermit;
    use zcash_client_backend::data_api::Account;
    use zcash_protocol::constants::{V6_TX_VERSION, V6_VERSION_GROUP_ID};

    fn funded_at(tip: u32) -> HistoryHarness {
        let mut harness = HistoryHarness::on_network(straddling());
        harness.mine_received(NOTE_VALUE);
        advance_to(&mut harness, tip);
        harness
    }
    type Proposal = zcash_client_backend::proposal::Proposal<
        zcash_client_backend::fees::StandardFeeRule,
        zcash_client_sqlite::ReceivedNoteId,
    >;
    fn propose(harness: &mut HistoryHarness) -> Proposal {
        let network = straddling();
        let st = harness.state_mut();
        let aid = st.test_account().expect("account").id();
        let request = zip321::TransactionRequest::new(vec![zip321::Payment::without_memo(
            recipient(),
            ProtoZat::const_from_u64(AMOUNT),
        )])
        .expect("a valid request");
        crate::send::propose_core(st.wallet_mut(), &network, aid, request).expect("propose")
    }
    fn sign(
        harness: &mut HistoryHarness,
        proposal: &Proposal,
    ) -> Result<Vec<zcash_protocol::TxId>, WalletError> {
        let network = straddling();
        let st = harness.state_mut();
        let usk = st.test_account().expect("account").usk().clone();
        let prover = crate::prover::tx_prover();
        crate::send::create_signed_core(
            st.wallet_mut(),
            &network,
            usk,
            proposal,
            prover,
            prover,
            SigningPermit::assume_current_for_test(),
        )
    }

    // The side that signs: proposed and signed at H - 22, the last tip before the pause.
    let mut early = funded_at(H - 22);
    let proposal = propose(&mut early);
    assert_eq!(
        u32::from(proposal.min_target_height()),
        H - 21,
        "premise: a proposal at the tip H - 22 targets H - 21"
    );
    let txids = sign(&mut early, &proposal)
        .expect("at the tip H - 22 the next 21 blocks are all Nu6_3: the proposal signs");
    let raw = raw_of(&early.read_conn(), &txids[0]);
    assert_eq!(header_branch_bytes(&raw), u32::from(BranchId::Nu6_3));

    // The side that is stale: proposed at H - 22, signed at H - 1, the drift exactly 20.
    let mut late = funded_at(H - 22);
    let proposal = propose(&mut late);
    advance_to(&mut late, H - 1);
    assert!(
        matches!(sign(&mut late, &proposal), Err(WalletError::ProposalStale)),
        "a proposal targeted below H, signed once the next block is H, would be signed \
         under the dead branch: ProposalStale (re-propose)"
    );

    // Re-proposed at the tip H - 1, it targets H and is signed under NU7.
    let fresh = propose(&mut late);
    assert_eq!(u32::from(fresh.min_target_height()), H);
    let txids = sign(&mut late, &fresh).expect("the fresh proposal signs");
    let raw = raw_of(&late.read_conn(), &txids[0]);
    assert_eq!(
        (
            u32::from_le_bytes(raw[0..4].try_into().expect("header")),
            u32::from_le_bytes(raw[4..8].try_into().expect("group")),
            header_branch_bytes(&raw),
        ),
        (
            0x8000_0000 | V6_TX_VERSION,
            V6_VERSION_GROUP_ID,
            0x7719_0AD9
        ),
        "the re-proposed send is a v6 transaction under NU7's branch id"
    );
}

/// **A case the contract does not name: a send signed under the NEW branch is never
/// cancelled by it.** The kernel's `H` is the end of the transaction's OWN branch,
/// and `Nu7` has no next activation on this chain, so a send built at target
/// `H + 1` keeps its stored expiry (`H + 1 + 120`, the NU7 default delta): it is
/// live at the tip `H + 1 + 119` and dead at its stored expiry, as before. A kernel
/// that keyed on the tip's side of the activation instead of the transaction's
/// branch (every send dead from `H - 1`) passes every listed row above and fails
/// here.
#[test]
fn a_send_signed_under_the_new_branch_is_never_cancelled_by_it() {
    let network = straddling();
    let mut harness = HistoryHarness::on_network(network);
    harness.mine_received(NOTE_VALUE);
    advance_to(&mut harness, H);
    let (send, claims) = harness.create_send_with_claims(recipient(), AMOUNT);
    let mut conn = harness.read_conn();
    intent_store::ensure_table(&conn).expect("intent table");
    delivery::ensure_table(&conn).expect("accepted table");
    let raw = raw_of(&conn, &send);
    assert_eq!(
        header_branch_bytes(&raw),
        0x7719_0AD9,
        "premise: signed under NU7"
    );
    let stored = harness.expiry_height_of(send);
    assert_eq!(
        stored,
        H + 1 + 120,
        "premise: the NU7 default delta, 120 blocks"
    );
    let now = now_unix();

    advance_to(&mut harness, stored - 1);
    assert!(join::enumerated(&conn, &network).contains(send.as_ref()));
    assert_eq!(
        join::state_of(&conn, &network, now, &send),
        Some(DeliveryState::RetryPending)
    );
    assert_eq!(
        join::reading(join::history_row(&conn, &network, now, &send).status),
        Reading::Pending,
        "one block before its own stored expiry an NU7 send is still on its way"
    );
    enrol_sent(&mut conn, &claims, &send);
    assert!(matches!(
        join::reconcile(&harness, &network, &mut conn, now),
        Reconciled::Rebroadcast { .. }
    ));
    assert_eq!(
        chain_status_writes(&network, &raw, send, stored, stored),
        Some(TransactionStatus::Mined(BlockHeight::from_u32(stored))),
        "a claim at its own stored expiry is possible and accepted"
    );

    advance_to(&mut harness, stored);
    assert_eq!(
        join::reading(join::history_row(&conn, &network, now, &send).status),
        Reading::Expired,
        "at its stored expiry it is plain Expired: no upgrade cancelled it"
    );
}

/// A send built at target `H - 10` whose STORED expiry is `expiry` instead of the
/// builder's, through upstream's own `create_proposed_transactions` with the
/// override our product path never passes: the shape of a canonical crossing,
/// whose ZIP-318 rolling expiry sits weeks above its target (S4), so its notes are
/// held long past the burial. A stand-in for the contract's
/// `crossing_straddling_nu7(h)` in the one property these rows read (the stored
/// expiry), never a ZIP-318 crossing: no fixture of that shape exists, and its
/// canonical shape (one Orchard input, an anchor on the ZIP-318 grid, a canonical
/// denomination into Ironwood) is not what the activation check reads. Built at the
/// tip `H - 11` with the pre-upgrade pause suspended, for the reason
/// [`a_send_built_ten_blocks_below_the_activation`] states.
pub(super) fn a_send_whose_notes_are_held_past_the_burial(expiry: u32) -> Killed {
    use zcash_client_backend::data_api::Account;
    use zcash_client_backend::data_api::wallet::{SpendingKeys, create_proposed_transactions};
    use zcash_client_backend::wallet::OvkPolicy;

    let network = straddling();
    let mut harness = HistoryHarness::on_network(network);
    harness.mine_received(NOTE_VALUE);
    advance_to(&mut harness, H - 11);
    let _inside_the_window = crate::send::upgrade_pause_testing::suspend();
    let st = harness.state_mut();
    let aid = st.test_account().expect("account").id();
    let request = zip321::TransactionRequest::new(vec![zip321::Payment::without_memo(
        recipient(),
        ProtoZat::const_from_u64(AMOUNT),
    )])
    .expect("a valid request");
    let proposal =
        crate::send::propose_core(st.wallet_mut(), &network, aid, request).expect("propose");
    let claims = crate::send::claim_from_proposal(&proposal).expect("claims");
    let usk = st.test_account().expect("account").usk().clone();
    let prover = crate::prover::tx_prover();
    let txids = create_proposed_transactions::<
        _,
        _,
        std::convert::Infallible,
        _,
        std::convert::Infallible,
        _,
    >(
        st.wallet_mut(),
        &network,
        &zcash_client_sqlite::util::SystemClock,
        &mut crate::rng::wallet_rng(),
        prover,
        prover,
        &SpendingKeys::from_unified_spending_key(usk),
        OvkPolicy::Sender,
        &proposal,
        Some(BlockHeight::from_u32(expiry)),
    )
    .expect("create with a far expiry");
    let send = *txids.first();
    let conn = harness.read_conn();
    intent_store::ensure_table(&conn).expect("intent table");
    delivery::ensure_table(&conn).expect("accepted table");
    assert_eq!(
        harness.expiry_height_of(send),
        expiry,
        "premise: the stored expiry"
    );
    assert_eq!(
        header_branch_bytes(&raw_of(&conn, &send)),
        u32::from(BranchId::Nu6_3),
        "premise: signed under Nu6_3"
    );
    Killed {
        harness,
        network,
        send,
        claims,
        conn,
    }
}

/// **The core half of `a_requeued_crossing_with_reserved_notes_names_its_cause_and_date`**
/// (S-M3 (d)). A send the upgrade killed whose notes the engine holds past the
/// burial (a crossing's shape: stored expiry `H + 700`) is requeued at the burial of
/// `H - 1` (the tip `H + 599`), and its re-proposal then meets `InsufficientFunds`,
/// because the one note is still reserved, so the drain lands in `Prepared::Stale`
/// and leaves the row `Queued`; one block before the release it is still stale, and
/// at the release (`H + 700`, the stored expiry) it is re-proposed and signed under
/// NU7. The other half, "its copy names the cause and the release date", has no
/// core surface to assert against: `Prepared::Stale` reaches the host only as a
/// count (`summary.stale`), and no reading carries a stale row's cause or release
/// height (reported as a missing seam).
#[test]
fn a_requeued_crossing_with_reserved_notes_is_stale_until_its_release() {
    use crate::consensus::SigningPermit;
    use crate::send::Prepared;
    use zcash_client_backend::data_api::Account;

    let Killed {
        mut harness,
        network,
        send,
        claims,
        mut conn,
    } = a_send_whose_notes_are_held_past_the_burial(H + 700);
    let id = enrol_sent(&mut conn, &claims, &send);
    let now = now_unix();

    advance_to(&mut harness, H + 598);
    assert_eq!(
        join::reconcile(&harness, &network, &mut conn, now),
        Reconciled::AwaitingBurial,
        "one block before the burial of H - 1 the dead send waits"
    );
    advance_to(&mut harness, H + 599);
    assert_eq!(
        join::reconcile(&harness, &network, &mut conn, now),
        Reconciled::Requeued,
        "at the burial of H - 1 it is requeued, though its notes are held to H + 700"
    );

    let drain = |harness: &mut HistoryHarness, conn: &mut Connection| -> Prepared {
        let st = harness.state_mut();
        let aid = st.test_account().expect("account").id();
        let usk = st.test_account().expect("account").usk().clone();
        let request = zip321::TransactionRequest::new(vec![zip321::Payment::without_memo(
            recipient(),
            ProtoZat::const_from_u64(AMOUNT),
        )])
        .expect("a valid request");
        let prover = crate::prover::tx_prover();
        crate::send::prepare_queued(
            st.wallet_mut(),
            &network,
            conn,
            aid,
            request,
            usk,
            prover,
            prover,
            id,
            None,
            0,
            now,
            SigningPermit::assume_current_for_test(),
        )
        .expect("prepare")
    };
    for tip in [H + 599, H + 699] {
        advance_to(&mut harness, tip);
        assert!(
            matches!(drain(&mut harness, &mut conn), Prepared::Stale),
            "at the tip {tip} the one note is still reserved by the dead send: the \
             re-proposal is short of funds and the row stays queued"
        );
        assert_eq!(
            intent_store::list_queued(&conn).expect("queued").len(),
            1,
            "the requeued row is still queued"
        );
    }
    advance_to(&mut harness, H + 700);
    assert!(
        matches!(drain(&mut harness, &mut conn), Prepared::Broadcast(ref txs) if txs.len() == 1),
        "at the release (the stored expiry) the note is free: the payment is sent again"
    );
}
