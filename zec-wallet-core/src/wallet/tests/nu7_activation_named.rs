//! The NU7 wave's step 4, the activation check (`docs/plan/nu7-wave.md` §3 step 4;
//! ADR-0579 Decisions 4 and 6): the TEST AUTHOR's rows that name a SYMBOL the
//! contract leaves to the implementer. They cannot compile at the base commit, so
//! `wallet.rs` declares this module under `#[cfg(any())]` (never true) until the
//! join, as steps 2b and 4s did. Written blind (IT-2a); each name below was
//! validated against a throwaway definition of the name alone, never committed.
//!
//! **Every name the implementer adds is read in ONE place, the `named` module
//! below**, and mapped into this file's own words (`Answer`, `Phase`,
//! `Cancelled`), so the join re-points that module and touches no assertion. The
//! names this file assumes, with their signatures:
//!
//! - the kernel's answer, `crate::send::Mineable { Never, Through(BlockHeight),
//!   Nowhere }` (the contract names the three variants, S-N1);
//! - the kernel, `crate::send::mineable_through<P: Parameters>(params: &P, raw:
//!   &[u8], stored_expiry: BlockHeight) -> Mineable`, ONE pure function over the
//!   header bytes, the stored expiry and the params (ADR-0579 Decision 4);
//! - its inner function, `crate::send::mineable_given_next_activation(
//!   next_activation: BlockHeight, stored_expiry: BlockHeight) -> Mineable`, which
//!   takes `H` directly so the `Nowhere` rows can reach it;
//! - `Mineable::is_past(self, tip: BlockHeight) -> bool`, today's `past_expiry`
//!   edge (`Through(h)` is past when `h <= tip`);
//! - the pair-checked header reader, `crate::enhance::header_branch(raw: &[u8])
//!   -> Option<u32>` (the contract names it, C-L6);
//! - the burial predicate, `crate::stranded::irreversibly_buried<P: Parameters>(
//!   params: &P, mined: BlockHeight, tip: BlockHeight) -> bool` (the contract
//!   names it, ADR-0579 Decision 6);
//! - the reading, `crate::state::TxStatus::CancelledByUpgrade { stopped_at:
//!   BlockHeight, released_at: Option<BlockHeight>, phase: CancelPhase }` (the
//!   crate's own `money::BlockHeight`, as `TxSummary` carries; the names decided by
//!   the orchestrator on 2026-10-30: `stopped_at` is `H`, the activation, and
//!   `released_at` the stored expiry, `None` for a stored 0) and
//!   `crate::state::CancelPhase { NotFinal, FinalNotReleased, Released }`, the
//!   three phases the core computes (S-M3).
//!
//! The readers themselves are called through the sibling file's `join` module
//! (`nu7_activation.rs`), whose signatures the join also re-points.

use proptest::prelude::*;
use rusqlite::Connection;
use zcash_client_backend::data_api::WalletWrite;
use zcash_client_backend::data_api::testing::TestBuilder;
use zcash_primitives::transaction::{Authorized, TransactionData, TxVersion};
use zcash_protocol::consensus::{
    BlockHeight, BranchId, MAIN_NETWORK, Network, NetworkUpgrade, Parameters, TEST_NETWORK,
};
use zcash_protocol::local_consensus::LocalNetwork;

use super::nu7_activation::{
    BURIAL_BLOCKS_AFTER_H, H, Killed, Reading, a_send_built_ten_blocks_below_the_activation,
    a_send_whose_notes_are_held_past_the_burial, advance_to, chain_status_writes, enrol_sent, join,
    now_unix, raw_of, straddling,
};
use crate::intent_store;
use crate::state::{DeliveryState, TxStatus, TxSummary};
use crate::test_support::HistoryHarness;

/// The kernel's answer, in this file's words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Answer {
    Never,
    Through(u32),
    Nowhere,
}

/// The phase a cancelled send's reading carries, in this file's words (S-M3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// Before the irreversible burial of `H - 1`.
    NotFinal,
    /// Buried, but the engine still holds the notes until the stored expiry.
    FinalNotReleased,
    /// Both buried and released: the only phase in which the amount is back.
    Released,
}

/// A cancelled send's reading: the height the network stopped accepting it, the
/// height its funds are released (`None` for a send with no stored expiry), and
/// its phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Cancelled {
    stopped_at: u32,
    released_at: Option<u32>,
    phase: Phase,
}

/// THE JOIN POINT for every name the implementer adds. The assertions below never
/// name an implementer's symbol.
mod named {
    use super::*;
    use crate::expiry::EffectiveExpiry as Mineable;

    fn answer(m: Mineable) -> Answer {
        match m {
            Mineable::Never => Answer::Never,
            Mineable::Through(h) => Answer::Through(h),
            Mineable::Nowhere => Answer::Nowhere,
        }
    }

    /// The kernel over a transaction's header bytes, its stored expiry and the params.
    pub(super) fn kernel<P: Parameters>(params: &P, raw: &[u8], stored_expiry: u32) -> Answer {
        answer(crate::expiry::effective_expiry(params, raw, stored_expiry))
    }

    /// The kernel's inner function, given `H` directly.
    pub(super) fn inner(next_activation: u32, stored_expiry: u32) -> Answer {
        answer(crate::expiry::effective_expiry_before(
            Some(BlockHeight::from_u32(next_activation)),
            stored_expiry,
        ))
    }

    /// Whether the kernel's answer for this transaction is past at `tip`.
    pub(super) fn dead_at<P: Parameters>(
        params: &P,
        raw: &[u8],
        stored_expiry: u32,
        tip: u32,
    ) -> bool {
        crate::expiry::effective_expiry(params, raw, stored_expiry).is_past(tip)
    }

    /// The pair-checked header reader.
    pub(super) fn header_branch(raw: &[u8]) -> Option<u32> {
        crate::enhance::header_branch(raw)
    }

    /// The burial predicate.
    pub(super) fn buried<P: Parameters>(params: &P, mined: u32, tip: u32) -> bool {
        crate::stranded::irreversibly_buried(params, mined, tip)
    }

    /// The fuzz hook's oracle over `stored expiry (4 bytes, little-endian) ++ header`:
    /// `true` when the kernel agrees with the hook's own reading of the header.
    pub(super) fn kernel_agrees_with_an_independent_reading(data: &[u8]) -> bool {
        crate::enhance::kernel_agrees_with_an_independent_header_reading(data)
    }

    /// The cancelled reading a history row carries, if it carries one.
    pub(super) fn cancelled(status: TxStatus) -> Option<Cancelled> {
        match status {
            TxStatus::CancelledByUpgrade {
                stopped_at,
                released_at,
                phase,
            } => Some(Cancelled {
                stopped_at: stopped_at.value(),
                released_at: released_at.map(|h| h.value()),
                phase: match phase {
                    crate::state::CancelPhase::NotFinal => Phase::NotFinal,
                    crate::state::CancelPhase::FinalNotReleased => Phase::FinalNotReleased,
                    crate::state::CancelPhase::Released => Phase::Released,
                },
            }),
            _ => None,
        }
    }
}

// ── Fixtures ──────────────────────────────────────────────────────────────────

/// A whole header: `fOverwintered | version`, the version group id, the branch id,
/// then lock time and expiry. The constants come from the crate, never literals.
fn header(version: u32, group: u32, branch: u32) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&(0x8000_0000u32 | version).to_le_bytes());
    b.extend_from_slice(&group.to_le_bytes());
    b.extend_from_slice(&branch.to_le_bytes());
    b.extend_from_slice(&[0u8; 8]);
    b
}

fn v5_header(branch: u32) -> Vec<u8> {
    header(
        zcash_protocol::constants::V5_TX_VERSION,
        zcash_protocol::constants::V5_VERSION_GROUP_ID,
        branch,
    )
}

fn v6_header(branch: u32) -> Vec<u8> {
    header(
        zcash_protocol::constants::V6_TX_VERSION,
        zcash_protocol::constants::V6_VERSION_GROUP_ID,
        branch,
    )
}

/// A real, parseable, empty transaction under `branch` with `expiry`, varied by
/// `seed` (its lock time), at the version the branch takes. No prover needed.
fn empty_tx(branch: BranchId, seed: u32, expiry: u32) -> (Vec<u8>, zcash_protocol::TxId) {
    let data: TransactionData<Authorized> = TransactionData::from_parts(
        TxVersion::suggested_for_branch(branch),
        branch,
        seed,
        BlockHeight::from_u32(expiry),
        None,
        None,
        None,
        None,
    );
    let tx = data.freeze().expect("freeze an empty transaction");
    let mut bytes = Vec::new();
    tx.write(&mut bytes).expect("serialize");
    (bytes, tx.txid())
}

/// `network` with no NU7 height: the real network as the pre-release crates saw it.
fn without_nu7(network: Network) -> LocalNetwork {
    let at = |nu| network.activation_height(nu);
    let mut p = TestBuilder::<(), ()>::DEFAULT_NETWORK;
    p.overwinter = at(NetworkUpgrade::Overwinter);
    p.sapling = at(NetworkUpgrade::Sapling);
    p.blossom = at(NetworkUpgrade::Blossom);
    p.heartwood = at(NetworkUpgrade::Heartwood);
    p.canopy = at(NetworkUpgrade::Canopy);
    p.nu5 = at(NetworkUpgrade::Nu5);
    p.nu6 = at(NetworkUpgrade::Nu6);
    p.nu6_1 = at(NetworkUpgrade::Nu6_1);
    p.nu6_2 = at(NetworkUpgrade::Nu6_2);
    p.nu6_3 = at(NetworkUpgrade::Nu6_3);
    p.nu7 = None;
    p
}

/// The straddling chain with NU7 at `h` instead of [`H`].
fn straddling_at(h: u32) -> LocalNetwork {
    LocalNetwork {
        nu7: Some(BlockHeight::from_u32(h)),
        ..straddling()
    }
}

fn nu7_of<P: Parameters>(params: &P) -> u32 {
    u32::from(
        params
            .activation_height(NetworkUpgrade::Nu7)
            .expect("these params carry an NU7 height"),
    )
}

// ── The kernel ────────────────────────────────────────────────────────────────

/// **`the_kernel_returns_the_block_before_the_next_activation_or_the_stored_expiry`**
/// (ADR-0579 Decision 4; S-N1). Given the transaction's own branch and `H`, the end
/// of that branch from the crate's `height_bounds`: a stored expiry below `H - 1`
/// stands; at or above it, the answer is `Through(H - 1)`; a stored 0 (no expiry)
/// with an `H` is `Through(H - 1)`; with no `H` (`height_bounds` `None`, or no upper
/// bound) the stored expiry stands, and so does it for an unreadable header. The
/// `H = 1` and `H = 0` rows (`Nowhere`) drive the inner function, since
/// `height_bounds` cannot produce them. On the crate's real activations (mainnet
/// 3,543,405, testnet 4,465,026) and on the straddling chain.
///
/// Mutants, each red: `min` for `max` (the `H + 5` row); drop the zero row (the
/// stored-0 row reads `Never`); `H` for `H - 1` (every capped row); floor `H - 1` at
/// 0 (the `H = 1` row reads `Through(0)`, which reads as "never expires").
#[test]
fn the_kernel_returns_the_block_before_the_next_activation_or_the_stored_expiry() {
    type KernelOn = Box<dyn Fn(&[u8], u32) -> Answer>;
    let networks: [(&str, KernelOn, u32); 3] = [
        (
            "MainNetwork",
            Box::new(|raw: &[u8], e: u32| named::kernel(&MAIN_NETWORK, raw, e)),
            nu7_of(&MAIN_NETWORK),
        ),
        (
            "TestNetwork",
            Box::new(|raw: &[u8], e: u32| named::kernel(&TEST_NETWORK, raw, e)),
            nu7_of(&TEST_NETWORK),
        ),
        (
            "straddling LocalNetwork",
            Box::new(|raw: &[u8], e: u32| named::kernel(&straddling(), raw, e)),
            H,
        ),
    ];
    let nu6_3 = u32::from(BranchId::Nu6_3);
    for (name, kernel, h) in &networks {
        let h = *h;
        let raw = v6_header(nu6_3);
        for (stored, want, row) in [
            (h - 2, Answer::Through(h - 2), "below H - 1: unchanged"),
            (h - 1, Answer::Through(h - 1), "at H - 1: H - 1"),
            (h, Answer::Through(h - 1), "at H: H - 1"),
            (
                h + 30,
                Answer::Through(h - 1),
                "the builder's H - 10 + 40: H - 1",
            ),
            (
                h + 69_120,
                Answer::Through(h - 1),
                "a crossing's rolling expiry: H - 1",
            ),
            (
                0,
                Answer::Through(h - 1),
                "stored 0 with an H: H - 1, never Never",
            ),
        ] {
            assert_eq!(kernel(&raw, stored), want, "{name}, a Nu6_3 send, {row}");
        }
        // No H: a transaction under the newest branch has no upper bound.
        let nu7 = v6_header(u32::from(BranchId::Nu7));
        assert_eq!(
            kernel(&nu7, h + 30),
            Answer::Through(h + 30),
            "{name}, an NU7 send: no next activation, the stored expiry stands"
        );
        assert_eq!(
            kernel(&nu7, 0),
            Answer::Never,
            "{name}, an NU7 send with no expiry and no next activation: Never"
        );
        // An unreadable header never shortens an expiry.
        for (bad, why) in [
            (Vec::new(), "no bytes"),
            (raw[..11].to_vec(), "a truncated header"),
            (
                header(4, 0x892F_2085, nu6_3),
                "a v4 header (bytes 8 to 12 are not a branch there)",
            ),
            (
                header(
                    zcash_protocol::constants::V5_TX_VERSION,
                    zcash_protocol::constants::V6_VERSION_GROUP_ID,
                    nu6_3,
                ),
                "a v5 version under v6's group id",
            ),
            (
                v6_header(0xDEAD_BEEF),
                "a branch id the crate does not know",
            ),
        ] {
            assert_eq!(
                kernel(&bad, h + 30),
                Answer::Through(h + 30),
                "{name}, {why}: the stored expiry stands"
            );
        }
    }
    // The inner function: the activations height_bounds cannot produce.
    assert_eq!(
        named::inner(1, 500),
        Answer::Nowhere,
        "H = 1: H - 1 would be 0"
    );
    assert_eq!(named::inner(1, 0), Answer::Nowhere, "H = 1, stored 0");
    assert_eq!(named::inner(0, 500), Answer::Nowhere, "H = 0");
    assert_eq!(
        named::inner(2, 500),
        Answer::Through(1),
        "H = 2: the block before it"
    );
    assert_eq!(named::inner(2, 0), Answer::Through(1), "H = 2, stored 0");
    assert_eq!(
        named::inner(100, 7),
        Answer::Through(7),
        "a stored expiry below H - 1"
    );
}

/// **`a_pre_release_mainnet_nu6_3_send_keeps_its_stored_expiry`** (S-N3, restricted
/// as the review found; M2). Parametrised on whether the params carry an NU7 height.
/// Without one (mainnet as the pre-release crates saw it): a `Nu6_3` transaction has
/// no next activation and keeps its stored expiry, while a `Nu6_2` one gets `Nu6_3`'s
/// activation (3,428,143) as its `H`. With one (the final crates, pinned since step
/// 1): the same `Nu6_3` transaction gets the crate's height, 3,543,405.
///
/// Mutant: a default `H` (any `H` the crate's bounds do not give). Red.
#[test]
fn a_pre_release_mainnet_nu6_3_send_keeps_its_stored_expiry() {
    let pre = without_nu7(Network::MainNetwork);
    let nu6_3_at = u32::from(
        MAIN_NETWORK
            .activation_height(NetworkUpgrade::Nu6_3)
            .expect("mainnet NU6.3"),
    );
    let stored = 3_600_000;
    let nu6_3 = v6_header(u32::from(BranchId::Nu6_3));
    let nu6_2 = v5_header(u32::from(BranchId::Nu6_2));
    assert_eq!(
        named::kernel(&pre, &nu6_3, stored),
        Answer::Through(stored),
        "no NU7 height: a Nu6_3 send has no next activation; its stored expiry stands"
    );
    assert_eq!(
        named::kernel(&pre, &nu6_3, 0),
        Answer::Never,
        "no NU7 height, stored 0: Never"
    );
    assert_eq!(
        named::kernel(&pre, &nu6_2, stored),
        Answer::Through(nu6_3_at - 1),
        "a Nu6_2 send gets Nu6_3's activation as its H"
    );
    assert_eq!(
        nu7_of(&MAIN_NETWORK),
        3_543_405,
        "premise: the finals' mainnet height"
    );
    assert_eq!(
        named::kernel(&MAIN_NETWORK, &nu6_3, stored),
        Answer::Through(3_543_404),
        "the final crates: a Nu6_3 send gets the crate's NU7 height"
    );
}

/// The kernel's answer from the contract's own formula, the oracle the property
/// test and the readers are held to: `H - 1` for a stored 0, `min(stored, H - 1)`
/// otherwise, for a `Nu6_3` transaction on a chain with NU7 at `h >= 2`.
fn oracle(stored: u32, h: u32) -> u32 {
    if stored == 0 {
        h - 1
    } else {
        stored.min(h - 1)
    }
}

/// A fresh real wallet DB on the straddling chain at `h` holding ONE synthetic
/// wallet-created, unmined `Nu6_3` transaction with `stored` expiry (its bytes, its
/// `created` stamp, its `expiry_height` column), a block row at `tip` and the chain
/// tip at `tip`: the shape the delivery readers read, without a prover. Not a synced
/// view: no block was scanned or checkpointed, so the resubmission pass cannot judge
/// it (see the property below).
struct Sample {
    /// Owns the wallet file `conn` reads.
    _harness: HistoryHarness,
    network: LocalNetwork,
    raw: Vec<u8>,
    txid: zcash_protocol::TxId,
    conn: Connection,
}

fn sample(h: u32, stored: u32, tip: u32, seed: u32) -> Sample {
    let network = straddling_at(h);
    let mut harness = HistoryHarness::on_network(network);
    let (raw, txid) = empty_tx(BranchId::Nu6_3, seed, stored);
    let conn = harness.read_conn();
    intent_store::ensure_table(&conn).expect("intent table");
    crate::delivery::ensure_table(&conn).expect("accepted table");
    conn.execute(
        "INSERT INTO transactions (txid, created, expiry_height, raw, min_observed_height) \
         VALUES (?1, '2026-10-29T00:00:00Z', ?2, ?3, ?4)",
        rusqlite::params![txid.as_ref(), stored, raw, tip.min(h)],
    )
    .expect("a wallet-created row");
    conn.execute(
        "INSERT INTO blocks (height, hash, time, sapling_tree) VALUES (?1, ?2, 0, x'00')",
        rusqlite::params![tip, vec![0u8; 32]],
    )
    .expect("the scanned tip");
    harness
        .state_mut()
        .wallet_mut()
        .update_chain_tip(BlockHeight::from_u32(tip))
        .expect("the chain tip");
    Sample {
        _harness: harness,
        network,
        raw,
        txid,
        conn,
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

    /// **`the_kernel_never_extends_an_expiry_and_its_readers_agree`** (R-2; L-7).
    /// Over the stored expiry, `H` and the tip: the kernel's answer is the oracle's,
    /// so it is never later than the stored expiry and always `Through(H - 1)` for
    /// a stored 0; and on every sample the readers that can run per sample agree
    /// with it: `enumerate` lists the transaction exactly while it is live, the
    /// generic `state_of` reads it owed exactly while it is live and `None` once it
    /// is dead, and `chain_status` accepts a claim at the tip exactly when the tip is
    /// a height it can be mined at.
    ///
    /// Two readers agree at the boundary rows of `nu7_activation.rs`, on a real
    /// chain, not per sample. The history page needs a scanned note per row. The
    /// resubmission pass judges nothing until the wallet has a synced view (a
    /// scanned, checkpointed chain): before it, `rebroadcast_group` waits without
    /// reading any expiry, and upstream cannot even read back an unmined
    /// transaction whose stored expiry is 0. A sample builds neither, and building
    /// a scanned chain hundreds of blocks long per case is not a property test.
    ///
    /// Mutant (L-7): `Through(H)` for a stored 0. Red.
    #[test]
    fn the_kernel_never_extends_an_expiry_and_its_readers_agree(
        // Above the harness birthday (100,000) by more than the tip's reach below
        // `h`, so no claim is refused by the birthday bound instead of the expiry.
        h in 100_400u32..100_800,
        stored_offset in prop_oneof![Just(None), (0u32..800).prop_map(Some)],
        tip_offset in 0u32..900,
        seed in any::<u32>(),
    ) {
        let stored = stored_offset.map_or(0, |o| h - 200 + o);
        let tip = h - 300 + tip_offset;
        let s = sample(h, stored, tip, seed);
        let eff = oracle(stored, h);

        let answer = named::kernel(&s.network, &s.raw, stored);
        prop_assert_eq!(answer, Answer::Through(eff), "the kernel is the formula");
        if stored != 0 {
            prop_assert!(eff <= stored, "never later than the stored expiry");
        } else {
            prop_assert_eq!(eff, h - 1, "a stored 0 is H - 1");
        }
        let dead = eff <= tip;
        prop_assert_eq!(named::dead_at(&s.network, &s.raw, stored, tip), dead);

        let now = now_unix();
        prop_assert_eq!(
            join::enumerated(&s.conn, &s.network).contains(s.txid.as_ref()),
            !dead,
            "enumerate"
        );
        prop_assert_eq!(
            join::state_of(&s.conn, &s.network, now, &s.txid),
            (!dead).then_some(DeliveryState::RetryPending),
            "state_of, generic obligation"
        );
        prop_assert_eq!(
            chain_status_writes(&s.network, &s.raw, s.txid, tip, tip).is_some(),
            tip <= eff,
            "chain_status, a claim at the tip"
        );
    }
}

/// **`the_header_branch_reader_reads_only_a_whole_v5_or_v6_header`** (C-L6; INC-007's
/// rule). Bytes 8 to 12 are the consensus branch id only in a v5 or v6 header, so
/// the reader takes the version and version group id TOGETHER as one of the two
/// known pairs, and anything else is `None`. Where a parsed transaction is in hand,
/// the reader agrees with `Transaction::consensus_branch_id()`. The evidence check
/// (`unknown_branch_evidence`) is layered on it and still matches only a branch the
/// crate does not know.
///
/// Mutant: read bytes 8 to 12 without the pair check. Red (the v4 row).
#[test]
fn the_header_branch_reader_reads_only_a_whole_v5_or_v6_header() {
    let nu6_3 = u32::from(BranchId::Nu6_3);
    assert_eq!(named::header_branch(&v6_header(nu6_3)), Some(nu6_3));
    assert_eq!(
        named::header_branch(&v5_header(u32::from(BranchId::Nu6_2))),
        Some(u32::from(BranchId::Nu6_2))
    );
    assert_eq!(
        named::header_branch(&v6_header(0xDEAD_BEEF)),
        Some(0xDEAD_BEEF),
        "a whole header naming an unknown branch is still read: it is a header"
    );
    for (bad, why) in [
        (
            header(4, 0x892F_2085, nu6_3),
            "a v4 header: bytes 8 to 12 are its inputs",
        ),
        (header(3, 0x03C4_8270, nu6_3), "a v3 header"),
        (
            header(
                zcash_protocol::constants::V5_TX_VERSION,
                zcash_protocol::constants::V6_VERSION_GROUP_ID,
                nu6_3,
            ),
            "a v5 version under v6's group id",
        ),
        (
            header(
                zcash_protocol::constants::V6_TX_VERSION,
                zcash_protocol::constants::V5_VERSION_GROUP_ID,
                nu6_3,
            ),
            "a v6 version under v5's group id",
        ),
        (v6_header(nu6_3)[..11].to_vec(), "eleven bytes"),
        (Vec::new(), "no bytes"),
    ] {
        assert_eq!(named::header_branch(&bad), None, "{why}");
    }
    for branch in [
        BranchId::Nu5,
        BranchId::Nu6_2,
        BranchId::Nu6_3,
        BranchId::Nu7,
    ] {
        let (raw, _) = empty_tx(branch, 7, 0);
        let parsed = zcash_primitives::transaction::Transaction::read(&raw[..], branch)
            .expect("our own bytes parse");
        assert_eq!(
            named::header_branch(&raw),
            Some(u32::from(parsed.consensus_branch_id())),
            "the header reader agrees with the parsed transaction under {branch:?}"
        );
    }
    // The evidence check, layered on the reader, is unchanged.
    assert_eq!(
        crate::enhance::unknown_branch_evidence(&v6_header(0xDEAD_BEEF)),
        Some(0xDEAD_BEEF)
    );
    assert_eq!(
        crate::enhance::unknown_branch_evidence(&v6_header(nu6_3)),
        None
    );
}

proptest! {
    /// The header reader is total: arbitrary bytes never panic it, and whatever it
    /// returns came from a whole v5 or v6 header (the existing evidence proptest,
    /// `enhance.rs`'s `unknown_branch_evidence_never_panics_on_arbitrary_bytes`,
    /// gains the reader at the join).
    #[test]
    fn the_header_branch_reader_never_panics_and_reads_only_whole_headers(
        data in proptest::collection::vec(any::<u8>(), 0..64),
    ) {
        if let Some(branch) = named::header_branch(&data) {
            prop_assert!(data.len() >= 12);
            prop_assert_eq!(branch, u32::from_le_bytes(data[8..12].try_into().unwrap()));
            let version = u32::from_le_bytes(data[0..4].try_into().unwrap()) & 0x7FFF_FFFF;
            let group = u32::from_le_bytes(data[4..8].try_into().unwrap());
            prop_assert!(
                (version == zcash_protocol::constants::V5_TX_VERSION
                    && group == zcash_protocol::constants::V5_VERSION_GROUP_ID)
                    || (version == zcash_protocol::constants::V6_TX_VERSION
                        && group == zcash_protocol::constants::V6_VERSION_GROUP_ID)
            );
            prop_assert!(data[3] & 0x80 != 0, "fOverwintered");
        }
    }
}

/// Every branch id the crate knows, read off the enum, never spelled.
fn known_branch_ids() -> Vec<u32> {
    [
        BranchId::Sprout,
        BranchId::Overwinter,
        BranchId::Sapling,
        BranchId::Blossom,
        BranchId::Heartwood,
        BranchId::Canopy,
        BranchId::Nu5,
        BranchId::Nu6,
        BranchId::Nu6_1,
        BranchId::Nu6_2,
        BranchId::Nu6_3,
        BranchId::Nu7,
    ]
    .into_iter()
    .map(u32::from)
    .collect()
}

/// Header bytes biased toward the shapes the pair check separates: the two whole
/// headers; a v3 or v4 header, or the version after v6 (`fOverwintered` set, and
/// bytes 8 to 12 are not a branch this build reads there); one version under the
/// other's group id; a version without `fOverwintered`; any word at all. Then a
/// known branch id or any, a tail, and a cut anywhere in the first twelve bytes or
/// none. One case in five is plain arbitrary bytes.
fn header_bytes() -> impl Strategy<Value = Vec<u8>> {
    use zcash_protocol::constants::{
        V3_TX_VERSION, V3_VERSION_GROUP_ID, V4_TX_VERSION, V4_VERSION_GROUP_ID, V5_TX_VERSION,
        V5_VERSION_GROUP_ID, V6_TX_VERSION, V6_VERSION_GROUP_ID,
    };
    let overwintered = 0x8000_0000u32;
    let version = prop_oneof![
        Just(overwintered | V5_TX_VERSION),
        Just(overwintered | V6_TX_VERSION),
        Just(overwintered | V4_TX_VERSION),
        Just(overwintered | V3_TX_VERSION),
        Just(overwintered | (V6_TX_VERSION + 1)),
        Just(V5_TX_VERSION),
        Just(V6_TX_VERSION),
        any::<u32>(),
    ];
    let group = prop_oneof![
        Just(V5_VERSION_GROUP_ID),
        Just(V6_VERSION_GROUP_ID),
        Just(V4_VERSION_GROUP_ID),
        Just(V3_VERSION_GROUP_ID),
        any::<u32>(),
    ];
    let branch = prop_oneof![proptest::sample::select(known_branch_ids()), any::<u32>()];
    let structured = (
        version,
        group,
        branch,
        proptest::collection::vec(any::<u8>(), 0..16),
        proptest::option::of(0usize..12),
    )
        .prop_map(|(version, group, branch, tail, cut)| {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&version.to_le_bytes());
            bytes.extend_from_slice(&group.to_le_bytes());
            bytes.extend_from_slice(&branch.to_le_bytes());
            bytes.extend_from_slice(&tail);
            if let Some(n) = cut {
                bytes.truncate(n);
            }
            bytes
        });
    prop_oneof![
        4 => structured,
        1 => proptest::collection::vec(any::<u8>(), 0..64),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 1024, ..ProptestConfig::default() })]

    /// **The fuzz oracle's twin** (`fuzz_enhance_parse`; the security review's finding
    /// that the old oracle was circular). The fuzz hook used the header reader's own
    /// verdict to decide what a whole header is, so a reader that dropped its pair
    /// check stayed green, and it cut the stored expiry from the header's own first
    /// four bytes, so a whole header always met the same stored value. The hook now
    /// takes the stored expiry from a prefix of its own and decides "a whole v5 or v6
    /// header" by its own reading of the layout; this drives it from CI, on bytes
    /// biased toward the shapes the pair check separates, with stored expiries at 0,
    /// below and above every activation:
    /// - a header the oracle cannot read, or whose branch has no next activation,
    ///   leaves the stored expiry standing: the kernel never shortens it;
    /// - a whole header whose branch ends at `H` gives `min(stored, H - 1)`, and
    ///   `H - 1` for a stored 0.
    ///
    /// Mutants, each red: the header reader reads bytes 8 to 12 behind any
    /// `fOverwintered` version (the pair check dropped); the same with only the group
    /// half dropped.
    #[test]
    fn the_kernel_agrees_with_an_independent_reading_of_any_header_bytes(
        stored in prop_oneof![Just(0u32), 0u32..5_000_000, any::<u32>()],
        header in header_bytes(),
    ) {
        let mut data = stored.to_le_bytes().to_vec();
        data.extend_from_slice(&header);
        prop_assert!(
            named::kernel_agrees_with_an_independent_reading(&data),
            "the expiry kernel disagreed with an independent reading of the header \
             (stored {stored}, header {header:02x?})"
        );
    }
}

// ── The burial predicate ─────────────────────────────────────────────────────

/// **The burial predicate's boundary rows** (ADR-0579 Decision 6; the contract moves
/// `stranded.rs`'s `buried` rows and `send.rs`'s burial boundaries here). Irreversible
/// burial is 15,000 s of chain time through the spacing function: 200 blocks wholly
/// before `H`, 600 wholly after it, summed across a straddle, one block each side of
/// each boundary, on both real networks and on the straddling chain; with no NU7
/// height, 200 at every height. A mined height above the tip is never buried.
///
/// **"Never below 100" has no row**, and no mutant can be watched on it: at 75 s and
/// 25 s the predicate's smallest depth is 200, so the floor binds only for a spacing
/// above 150 s, which no network the crate models has after Blossom.
///
/// The straddling chain's rows reach `H - 1_000`, below its Blossom height (100,000),
/// where the spacing is the crate's 150 s since step 7m; so these rows read it with
/// Blossom moved below their lowest height, and every "at 75 s" row counts 75 s blocks.
#[test]
fn irreversible_burial_is_15000_seconds_of_chain_time_each_side_of_the_activation() {
    fn rows<P: Parameters>(name: &str, params: &P, h: u32) {
        // `(mined, tip]` is what is counted: each block at its own spacing.
        for (mined, depth, why) in [
            (h - 1_000, 200, "wholly before H: 200 blocks at 75 s"),
            (h - 1, 600, "from H - 1: 600 blocks at 25 s"),
            (h + 5_000, 600, "wholly after H: 600 blocks at 25 s"),
            (h - 101, 400, "straddling: 100 at 75 s, then 300 at 25 s"),
            (
                h - 201,
                200,
                "the last span wholly before H: 200 at 75 s, ending at H - 1",
            ),
            (h - 200, 202, "one block later: 199 at 75 s, then 3 at 25 s"),
            (
                h - 199,
                204,
                "two blocks later: 198 at 75 s, then 6 at 25 s",
            ),
        ] {
            assert!(
                named::buried(params, mined, mined + depth),
                "{name}: mined at {mined} ({why}) is buried at depth {depth}"
            );
            assert!(
                !named::buried(params, mined, mined + depth - 1),
                "{name}: mined at {mined} ({why}) is NOT buried at depth {}",
                depth - 1
            );
        }
        assert!(
            !named::buried(params, h + 10, h),
            "{name}: a mined height above the tip"
        );
    }
    rows("MainNetwork", &MAIN_NETWORK, nu7_of(&MAIN_NETWORK));
    rows("TestNetwork", &TEST_NETWORK, nu7_of(&TEST_NETWORK));
    let straddling_above_blossom = LocalNetwork {
        blossom: Some(BlockHeight::from_u32(H - 1_000)),
        ..straddling()
    };
    rows("straddling LocalNetwork", &straddling_above_blossom, H);
    for (name, params) in [
        ("MainNetwork without NU7", without_nu7(Network::MainNetwork)),
        (
            "LocalNetwork without NU7",
            LocalNetwork {
                nu7: None,
                ..straddling()
            },
        ),
    ] {
        for mined in [1_000_000u32, 3_543_300, 3_543_404, 3_543_405] {
            assert!(
                named::buried(&params, mined, mined + 200),
                "{name}: {mined}"
            );
            assert!(
                !named::buried(&params, mined, mined + 199),
                "{name}: {mined}"
            );
        }
    }
}

// ── What a user sees ─────────────────────────────────────────────────────────

fn reading_at(k: &mut Killed, tip: u32) -> TxSummary {
    advance_to(&mut k.harness, tip);
    join::history_row(&k.conn, &k.network, now_unix(), &k.send)
}

/// **`a_send_the_upgrade_cancelled_carries_both_heights_and_its_phase`** (S-M3 (a)
/// and (b); M-3). The history row of a send the upgrade killed carries the height the
/// network stopped accepting it (the activation, `H`) and the height its funds are
/// released (its stored expiry, which `TxSummary.expiry_height` keeps), and a phase
/// the core computes, so the host renders and computes nothing:
/// - an ordinary send (stored expiry `H + 30`): the engine frees its notes at
///   `H + 30`, BEFORE the burial of `H - 1` at `H + 599`, and the phase still waits
///   for the burial: not final through `H + 598`, released at `H + 599`;
/// - a send whose notes are held past the burial (stored expiry `H + 700`): not
///   final through `H + 598`, final but not released from `H + 599` through
///   `H + 699`, released at `H + 700`.
///
/// One block each side of every phase change.
///
/// Mutants, each red: `expiry_height` set to `H - 1`; the phase reads released at
/// release alone (the ordinary send at `H + 30`). The third, the "still yours to
/// spend" copy before the released phase, is the Dart row's.
#[test]
fn a_send_the_upgrade_cancelled_carries_both_heights_and_its_phase() {
    let mut ordinary = a_send_built_ten_blocks_below_the_activation();
    for (tip, phase, why) in [
        (H - 1, Phase::NotFinal, "the first tip it is dead at"),
        (
            H + 29,
            Phase::NotFinal,
            "one block before the engine frees its notes",
        ),
        (
            H + 30,
            Phase::NotFinal,
            "the engine frees the notes; burial is still ahead",
        ),
        (
            H + 598,
            Phase::NotFinal,
            "one block before the burial of H - 1",
        ),
        (H + 599, Phase::Released, "buried and released"),
    ] {
        let row = reading_at(&mut ordinary, tip);
        assert_eq!(
            named::cancelled(row.status),
            Some(Cancelled {
                stopped_at: H,
                released_at: Some(H + 30),
                phase,
            }),
            "an ordinary send at the tip {tip} ({why})"
        );
        assert_eq!(
            row.expiry_height.map(|h| h.value()),
            Some(H + 30),
            "TxSummary.expiry_height stays the release height, never H - 1"
        );
    }

    let mut held = a_send_whose_notes_are_held_past_the_burial(H + 700);
    for (tip, phase) in [
        (H - 1, Phase::NotFinal),
        (H + 598, Phase::NotFinal),
        (H + 599, Phase::FinalNotReleased),
        (H + 699, Phase::FinalNotReleased),
        (H + 700, Phase::Released),
    ] {
        let row = reading_at(&mut held, tip);
        assert_eq!(
            named::cancelled(row.status),
            Some(Cancelled {
                stopped_at: H,
                released_at: Some(H + 700),
                phase,
            }),
            "a send whose notes are held to H + 700, at the tip {tip}"
        );
    }
}

/// The history reading, through the base file's mapping, agrees with the named
/// reading: the join of [`join::reading`] maps the built variant, and only it.
#[test]
fn the_base_rows_mapping_names_the_cancelled_reading() {
    let mut k = a_send_built_ten_blocks_below_the_activation();
    let row = reading_at(&mut k, H - 1);
    assert!(named::cancelled(row.status).is_some());
    assert_eq!(join::reading(row.status), Reading::CancelledByUpgrade);
}

/// **A cancelled send whose stored expiry is 0** (no expiry; the step 4 review's owed
/// row: no row covered a stored-0 cancelled send or `released_at: None`). Its
/// effective expiry is `H - 1` all the same, so from the tip `H - 1` it reads
/// cancelled by the upgrade, with `stopped_at` the activation `H`. But the engine
/// never frees the notes of a transaction stored with no expiry, so it has no release
/// height: `released_at` is `None` and so is `TxSummary.expiry_height`, never
/// `Some(0)`, which a host would read as "released at height 0". Its phase is not
/// final until the burial of `H - 1` (the tip `H + 599`), then final but not
/// released, and it never reads released, however far the tip goes.
///
/// Mutants, each red: a stored 0 counts as released (`Released` at `H + 599`);
/// `released_at` carries the stored 0 as a height (`Some(0)`).
#[test]
fn a_cancelled_send_with_no_stored_expiry_has_no_release_height_and_is_never_released() {
    let mut k = a_send_whose_notes_are_held_past_the_burial(0);
    let live = reading_at(&mut k, H - 2);
    assert_eq!(
        join::reading(live.status),
        Reading::Pending,
        "at the tip H - 2 the next block is still Nu6_3: on its way"
    );
    assert_eq!(
        live.expiry_height, None,
        "a send stored with no expiry names no expiry height"
    );
    for (tip, phase, why) in [
        (H - 1, Phase::NotFinal, "the first tip it is dead at"),
        (
            H + 598,
            Phase::NotFinal,
            "one block before the burial of H - 1",
        ),
        (
            H + 599,
            Phase::FinalNotReleased,
            "buried, and the engine never frees the notes of a stored 0",
        ),
        (
            H + 1_000,
            Phase::FinalNotReleased,
            "far past the burial: still never released",
        ),
    ] {
        let row = reading_at(&mut k, tip);
        assert_eq!(
            named::cancelled(row.status),
            Some(Cancelled {
                stopped_at: H,
                released_at: None,
                phase,
            }),
            "a send stored with no expiry, at the tip {tip} ({why})"
        );
        assert_eq!(
            row.expiry_height, None,
            "at the tip {tip}: no release height, never Some(0)"
        );
    }
}

// ── One send over a random tip walk ──────────────────────────────────────────

/// The walk's wall clock. The readers it drives read `now` only for a swap
/// deposit's deadline, and none of these sends is a deposit.
const WALK_NOW: u64 = 1_790_000_000;

/// Where every walk starts: the tip each fixture signed its send at.
const WALK_BASE: u32 = H - 11;

/// What the resubmission pass decided for a live `Sent` row owning the send.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pass {
    Rebroadcast,
    AwaitingBurial,
    Requeued,
    /// Anything else (`Stranded`, `Deleted`, `Awaiting`, a hold): never right for an
    /// unmined single-step send with its bytes in the store.
    Other,
}

/// Every reader's answer at one tip, the chain tip and the scanned tip agreeing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Seen {
    /// `delivery::enumerate` lists it (no intent row).
    enumerated: bool,
    /// `state_of`, the generic obligation (no intent row).
    generic: Option<DeliveryState>,
    /// `state_of` with a live `Sent` row owning it and an accepted mark on it.
    owned: Option<DeliveryState>,
    /// `rebroadcast_group`, through the resubmission pass's reconcile of that row.
    pass: Pass,
    /// The history row's status, and its cancelled reading if it carries one.
    history: Reading,
    cancelled: Option<Cancelled>,
    /// `TxSummary.expiry_height`.
    expiry_height: Option<u32>,
    /// `chain_status` accepts an endpoint's "mined at the tip" with the tip known.
    accepts_a_claim_at_the_tip: bool,
}

impl Seen {
    /// How far the send has settled: live, dead but not final, final but held,
    /// done. While the tip only rises this never falls.
    fn settled(&self) -> u8 {
        match (self.pass, self.cancelled.map(|c| c.phase)) {
            (Pass::Rebroadcast, None) => 0,
            (Pass::AwaitingBurial, None | Some(Phase::NotFinal)) => 1,
            (Pass::Requeued, Some(Phase::FinalNotReleased)) => 2,
            (Pass::Requeued, None | Some(Phase::Released)) => 3,
            _ => u8::MAX,
        }
    }
}

/// One send under the walk, on its own wallet. The walk drives the chain through
/// upstream's own test state: empty blocks mined and scanned, and a reorg as a real
/// truncation of the wallet AND the block cache, so the next block up is a new fork.
struct Walked {
    name: &'static str,
    k: Killed,
    /// The stored expiry, `0` for none.
    stored: u32,
    /// The contract's effective expiry ([`oracle`]): the last height it can be mined at.
    effective: u32,
    /// The first tip at which [`Self::effective`] is irreversibly buried.
    buried_from: u32,
    tip: u32,
}

impl Walked {
    fn new(name: &'static str, k: Killed) -> Self {
        let stored = k.harness.expiry_height_of(k.send);
        let effective = oracle(stored, H);
        let buried_from = (effective..)
            .find(|&t| named::buried(&k.network, effective, t))
            .expect("a burial height");
        Self {
            name,
            k,
            stored,
            effective,
            buried_from,
            tip: WALK_BASE,
        }
    }

    /// Move the chain to `to`: up by mining and scanning empty blocks, down by a
    /// reorg whose fork point is `to`. Upstream rewinds the wallet to its nearest
    /// checkpoint at or below `to` and the blocks from there to `to`, which the fork
    /// keeps, are scanned again, as a sync after a reorg does. The harness's own
    /// block count is not used after this.
    fn walk_to(&mut self, to: u32) {
        let from = self.tip;
        let st = self.k.harness.state_mut();
        if to > from {
            for _ in from..to {
                st.generate_empty_block();
            }
            st.scan_cached_blocks(BlockHeight::from_u32(from + 1), (to - from) as usize);
        } else if to < from {
            st.truncate_to_height(BlockHeight::from_u32(to));
            let rewound = crate::history::scanned_tip(&self.k.conn)
                .expect("scanned tip")
                .expect("a scanned block survives the rewind");
            if rewound < to {
                st.scan_cached_blocks(BlockHeight::from_u32(rewound + 1), (to - rewound) as usize);
            }
        }
        st.wallet_mut()
            .update_chain_tip(BlockHeight::from_u32(to))
            .expect("the chain tip");
        self.tip = to;
        assert_eq!(
            crate::history::scanned_tip(&self.k.conn).expect("scanned tip"),
            Some(to),
            "premise: the scanned tip is the walk's ({from} to {to})"
        );
    }

    /// Every reader, at the current tip. The intent-owned readings run on a `Sent`
    /// row enrolled for them and removed after, so the generic ones see no row.
    fn seen(&mut self) -> Seen {
        let k = &mut self.k;
        let enumerated = join::enumerated(&k.conn, &k.network).contains(k.send.as_ref());
        let generic = join::state_of(&k.conn, &k.network, WALK_NOW, &k.send);
        let row = join::history_row(&k.conn, &k.network, WALK_NOW, &k.send);

        let id = enrol_sent(&mut k.conn, &k.claims, &k.send);
        crate::delivery::mark_accepted(&k.conn, k.send.as_ref(), Some(self.tip), WALK_NOW)
            .expect("mark accepted");
        let owned = join::state_of(&k.conn, &k.network, WALK_NOW, &k.send);
        let pass = match join::reconcile(&k.harness, &k.network, &mut k.conn, WALK_NOW) {
            crate::send::Reconciled::Rebroadcast { .. } => Pass::Rebroadcast,
            crate::send::Reconciled::AwaitingBurial => Pass::AwaitingBurial,
            crate::send::Reconciled::Requeued => Pass::Requeued,
            _ => Pass::Other,
        };
        assert!(
            intent_store::delete(&mut k.conn, id).expect("delete"),
            "the enrolled row, Sent or requeued, is removed"
        );
        k.conn
            .execute(
                "DELETE FROM tx_delivery_accepted WHERE txid = ?1",
                rusqlite::params![k.send.as_ref()],
            )
            .expect("clear the mark");

        let raw = raw_of(&k.conn, &k.send);
        let accepts_a_claim_at_the_tip =
            chain_status_writes(&k.network, &raw, k.send, self.tip, self.tip).is_some();
        Seen {
            enumerated,
            generic,
            owned,
            pass,
            history: join::reading(row.status),
            cancelled: named::cancelled(row.status),
            expiry_height: row.expiry_height.map(|h| h.value()),
            accepts_a_claim_at_the_tip,
        }
    }

    /// What the contract says every reader must answer at the current tip, from
    /// the tip alone: nothing a reader saw at an earlier tip may change it.
    fn expected(&self) -> Seen {
        let t = self.tip;
        let live = t < self.effective;
        let buried = t >= self.buried_from;
        let by_upgrade = self.stored == 0 || self.effective < self.stored;
        let released_at = (self.stored != 0).then_some(self.stored);
        let phase = if !buried {
            Phase::NotFinal
        } else if released_at.is_some_and(|r| t >= r) {
            Phase::Released
        } else {
            Phase::FinalNotReleased
        };
        Seen {
            enumerated: live,
            generic: live.then_some(DeliveryState::RetryPending),
            owned: Some(if live {
                DeliveryState::Accepted
            } else {
                DeliveryState::RetryPending
            }),
            pass: if live {
                Pass::Rebroadcast
            } else if buried {
                Pass::Requeued
            } else {
                Pass::AwaitingBurial
            },
            history: if live {
                Reading::Pending
            } else if by_upgrade {
                Reading::CancelledByUpgrade
            } else {
                Reading::Expired
            },
            cancelled: (!live && by_upgrade).then_some(Cancelled {
                stopped_at: H,
                released_at,
                phase,
            }),
            expiry_height: released_at,
            accepts_a_claim_at_the_tip: t <= self.effective,
        }
    }
}

/// A height the walk moves the tip to, resolved against the send it walks.
#[derive(Clone, Copy, Debug)]
enum Mark {
    Base,
    /// `H - 1 - k`: the last heights below the activation, `H - 1` itself first.
    BelowActivation(u32),
    /// One block either side of the effective expiry, or on it.
    Effective(i32),
    /// One block either side of the activation, or on it.
    Activation(i32),
    /// One block either side of the burial of the effective expiry, or on it.
    Buried(i32),
    /// One block either side of the stored expiry, or on it; for a stored 0, far
    /// past the burial.
    Stored(i32),
    /// `WALK_BASE + n`.
    Anywhere(u32),
}

impl Walked {
    fn resolve(&self, mark: Mark, capped: bool) -> u32 {
        let near = |at: u32, d: i32| at.saturating_add_signed(d);
        let t = match mark {
            Mark::Base => WALK_BASE,
            Mark::BelowActivation(k) => H - 1 - k,
            Mark::Effective(d) => near(self.effective, d),
            Mark::Activation(d) => near(H, d),
            Mark::Buried(d) => near(self.buried_from, d),
            Mark::Stored(d) if self.stored == 0 => near(self.buried_from + 400, d),
            Mark::Stored(d) => near(self.stored, d),
            Mark::Anywhere(n) => WALK_BASE + n,
        }
        .max(WALK_BASE);
        // A server whose tip never reaches the activation.
        if capped { t.min(H - 1) } else { t }
    }
}

/// A walk: which send, whether the server's tip never reaches `H`, and the moves.
fn tip_walk() -> impl Strategy<Value = (usize, bool, Vec<Mark>)> {
    let mark = prop_oneof![
        1 => Just(Mark::Base),
        2 => (0u32..10).prop_map(Mark::BelowActivation),
        3 => (-1i32..=1).prop_map(Mark::Effective),
        2 => (-1i32..=1).prop_map(Mark::Activation),
        3 => (-1i32..=1).prop_map(Mark::Buried),
        2 => (-1i32..=1).prop_map(Mark::Stored),
        2 => (0u32..800).prop_map(Mark::Anywhere),
    ];
    (
        0usize..4,
        proptest::bool::weighted(0.25),
        proptest::collection::vec(mark, 1..8),
    )
}

/// **One send's readings over a random tip walk agree, at every step, with what the
/// contract says from the tip alone** (`docs/plan/nu7-wave.md` §3 step 4; ADR-0579
/// Decisions 4 and 6; the review's state-machine ask). Four sends on the straddling
/// chain, each signed at `H - 11` under `Nu6_3`: an ordinary one (stored expiry
/// `H + 30`), one whose notes are held past the burial (`H + 700`), one stored with no
/// expiry (`0`), and one whose own stored expiry ends it first (`H - 5`). Each walk
/// moves one of them through forward steps and reorgs (real truncations, below and
/// above `H - 1`), and one walk in four is a server whose tip never reaches `H`. Two
/// fixed walks per send run before the random ones, so the shapes named here are
/// covered on every run.
///
/// At every step, across all readers (`enumerate`, `state_of` generic and
/// intent-owned, the resubmission pass, the history row with its phase and heights,
/// `chain_status`):
/// - never re-broadcast, listed or read owed at a tip at or above the effective
///   expiry, and a claim of "mined at the tip" accepted exactly while the tip is a
///   height it can be mined at;
/// - never `Released` before the effective expiry is buried AND the stored expiry is
///   reached (never, for a stored 0), and `released_at` is the stored expiry or `None`;
/// - the resubmission pass requeues only once the effective expiry is buried;
/// - the reading is a function of the tip alone, so a reorg below `H - 1` returns the
///   send to a live reading with nothing latched, and while the tip only rises the
///   settlement (live, not final, final but held, released) never falls.
///
/// Mutants, each red: a durable "cancelled" latch in the history reading; the
/// resubmission pass requeues at the bare effective expiry; a stored 0 counts as
/// released.
#[test]
fn one_sends_readings_over_a_random_tip_walk_agree_with_the_tip_alone() {
    use proptest::test_runner::{Config, TestCaseResult, TestRunner};
    use std::cell::RefCell;

    let sends = RefCell::new([
        Walked::new(
            "ordinary, stored H + 30",
            a_send_built_ten_blocks_below_the_activation(),
        ),
        Walked::new(
            "notes held past the burial, stored H + 700",
            a_send_whose_notes_are_held_past_the_burial(H + 700),
        ),
        Walked::new(
            "no expiry, stored 0",
            a_send_whose_notes_are_held_past_the_burial(0),
        ),
        Walked::new(
            "its own expiry first, stored H - 5",
            a_send_whose_notes_are_held_past_the_burial(H - 5),
        ),
    ]);
    // The premises the oracle's burial rests on, pinned to the contract's numbers
    // rather than to the predicate it reads: 15,000 s from `H - 1` is 600 blocks at
    // 25 s; from `H - 5`, 4 blocks at 75 s and 588 at 25 s.
    {
        let sends = sends.borrow();
        for w in sends.iter().take(3) {
            assert_eq!(
                (w.effective, w.buried_from),
                (H - 1, H - 1 + BURIAL_BLOCKS_AFTER_H),
                "premise: {}",
                w.name
            );
        }
        assert_eq!(
            (sends[3].effective, sends[3].buried_from),
            (H - 5, H - 5 + 4 + 588),
            "premise: {}",
            sends[3].name
        );
    }

    let walk = |(which, capped, moves): (usize, bool, Vec<Mark>)| -> TestCaseResult {
        let mut sends = sends.borrow_mut();
        let w = &mut sends[which];
        w.walk_to(WALK_BASE);
        let mut trail = vec![WALK_BASE];
        let mut before = w.seen();
        prop_assert_eq!(before, w.expected(), "{} at the base tip", w.name);
        for mark in moves {
            let from = w.tip;
            let to = w.resolve(mark, capped);
            w.walk_to(to);
            trail.push(to);
            let seen = w.seen();
            prop_assert_eq!(
                seen,
                w.expected(),
                "{}: at the tip {} (H{:+}) after the walk {:?}",
                w.name,
                to,
                i64::from(to) - i64::from(H),
                trail
            );
            if to >= from {
                prop_assert!(
                    seen.settled() >= before.settled(),
                    "{}: the tip rose from {} to {} and the reading went back ({:?} to {:?})",
                    w.name,
                    from,
                    to,
                    before,
                    seen
                );
            }
            if capped {
                prop_assert!(
                    seen.pass != Pass::Requeued
                        && seen.cancelled.is_none_or(|c| c.phase == Phase::NotFinal),
                    "{}: a server tip that never reaches H never buries the block before it ({:?})",
                    w.name,
                    seen
                );
            }
            before = seen;
        }
        Ok(())
    };

    // Two walks every run takes for each send, so the shapes the contract names are
    // never left to chance: up through every phase, a reorg above `H - 1` from past
    // the burial, a reorg below it, and up again; and a server stuck at `H - 1`.
    for which in 0..4 {
        for (capped, moves) in [
            (
                false,
                vec![
                    Mark::Effective(-1),
                    Mark::Effective(0),
                    Mark::Buried(-1),
                    Mark::Buried(0),
                    Mark::Stored(0),
                    Mark::Stored(1),
                    Mark::BelowActivation(1),
                    Mark::Buried(1),
                ],
            ),
            (
                true,
                vec![
                    Mark::BelowActivation(0),
                    Mark::Anywhere(799),
                    Mark::BelowActivation(3),
                    Mark::Activation(0),
                ],
            ),
        ] {
            if let Err(e) = walk((which, capped, moves.clone())) {
                panic!("the fixed walk {moves:?} (capped {capped}) failed: {e}");
            }
        }
    }

    let mut runner = TestRunner::new(Config {
        cases: 32,
        max_shrink_iters: 64,
        failure_persistence: None,
        ..Config::default()
    });
    if let Err(e) = runner.run(&tip_walk(), walk) {
        panic!("{e}");
    }
}

/// **`a_cancelled_by_upgrade_reading_logs_branches_and_heights_only`**
/// (`docs/specs/wallet-sdk.md` §5.4; the contract's surface list names a log line).
/// The readers of a send the upgrade killed, run under the capture layer at the tip
/// `H - 1`, log the cancellation with branch ids and heights, and nothing that
/// identifies the payment: no txid, in either byte order. Every field passes the
/// §5.4 guard.
///
/// The contract names no field, so the branch and the heights are found by VALUE
/// among everything captured: the branch the send was signed under (`Nu6_3`, in
/// hex or decimal), the activation `H` and the send's stored expiry `H + 30`, each
/// as a whole number.
///
/// Mutant: log the txid. Red.
#[test]
fn a_cancelled_by_upgrade_reading_logs_branches_and_heights_only() {
    use crate::tracing_guard::{
        CaptureLayer, CapturedEvents, assert_5_4_clean, force_wallet_callsites_enabled,
    };
    use tracing_subscriber::layer::SubscriberExt;

    let mut k = a_send_built_ten_blocks_below_the_activation();
    advance_to(&mut k.harness, H - 1);
    force_wallet_callsites_enabled();
    let sink = CapturedEvents::default();
    let subscriber = tracing_subscriber::registry().with(CaptureLayer::new(sink.clone()));
    {
        let _guard = tracing::subscriber::set_default(subscriber);
        let now = now_unix();
        let _ = join::enumerated(&k.conn, &k.network);
        let _ = join::state_of(&k.conn, &k.network, now, &k.send);
        let _ = join::history_row(&k.conn, &k.network, now, &k.send);
        enrol_sent(&mut k.conn, &k.claims, &k.send);
        let _ = join::reconcile(&k.harness, &k.network, &mut k.conn, now);
    }
    let fields = sink.fields();
    // A whole decimal number in a rendered value (`100060`, `BlockHeight(100060)`),
    // never a run of digits inside a longer one.
    let carries = |value: &str, n: u32| {
        value
            .split(|c: char| !c.is_ascii_digit())
            .any(|run| run == n.to_string())
    };
    let nu6_3 = u32::from(BranchId::Nu6_3);
    assert!(
        fields.iter().any(|(_, v)| {
            let v = v.to_lowercase();
            v.contains(&format!("{nu6_3:08x}")) || carries(&v, nu6_3)
        }),
        "the cancellation is logged with the branch it was signed under (Nu6_3, \
         {nu6_3:#010x}): {fields:?}"
    );
    for (height, what) in [
        (H, "the activation H"),
        (H + 30, "the stored expiry H + 30"),
    ] {
        assert!(
            fields.iter().any(|(_, v)| carries(v, height)),
            "and the heights that decided it: {what} ({height}) is among the values: {fields:?}"
        );
    }
    let internal = hex::encode(k.send.as_ref());
    let mut display_bytes = *k.send.as_ref();
    display_bytes.reverse();
    let display = hex::encode(display_bytes);
    for (name, value) in &fields {
        let v = value.to_lowercase();
        assert!(
            !v.contains(&internal) && !v.contains(&display),
            "field `{name}` carries the txid: {value}"
        );
    }
    assert_5_4_clean(&fields);
}
