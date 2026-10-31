//! The NU7 wave's step 2b, the generator seam: the test author's rows that
//! compile at the contract commit (`docs/plan/nu7-wave.md` §3 "Step 2b",
//! ADR-0579 Decision 3). Written blind against the contract: the author has
//! not read the implementer's half.
//!
//! **What is at stake.** `create_proposed_transactions` takes "the randomness
//! used to construct, prove, and sign each transaction" from its caller. The
//! RedDSA signing nonce is hashed from that generator's bytes
//! (`reddsa-0.6.1/src/signing_key.rs:160-169`), and the spend's key
//! randomizer `alpha` is drawn from the same generator
//! (`sapling-crypto-0.9.0/src/builder.rs:259`,
//! `orchard-0.16.0/src/builder.rs:449`). A stream an observer can predict
//! hands them the nonce, then the randomized key, then the spend authorizing
//! key. So the property under test is not "signatures are not repeatable":
//! it is that two signatures over the SAME spend never share randomness.
//!
//! **How it is driven.** Two harness wallets from one seed over one chain:
//! `TestBuilder` seeds both its chain generator and the account seed with
//! constants, so two builds are the same wallet holding the same note. Each
//! proposes the same payment and signs it through the production
//! `send::create_signed_core`, which draws its generator from the seam. A
//! transparent spend would prove nothing (its signature is deterministic
//! under RFC 6979), so every row here spends a SHIELDED note.
//!
//! **What is compared.** The txid does not commit to proofs or signatures,
//! so the authorizing data is compared separately through
//! `auth_commitment()` (`zcash_primitives-0.31.0/src/transaction/mod.rs:1317`),
//! and the randomized keys `rk` are compared directly, since a repeated `rk`
//! over the same note is a repeated `alpha`.
//!
//! **The control.** The last row feeds both wallets one fixed stream, the
//! contract's mutant, through the raw engine call, and asserts the two
//! transactions are then byte-identical. Without it, "the two transactions
//! differ" could hold for a reason other than the generator (a clock in the
//! transaction, a hidden draw from the operating system inside the engine),
//! and the rows above would pass with a fixed seam.

use getrandom::rand_core::{TryCryptoRng, TryRng};
use zcash_address::ZcashAddress;
use zcash_client_backend::data_api::testing::{AddressType, TestBuilder};
use zcash_client_backend::data_api::wallet::{SpendingKeys, create_proposed_transactions};
use zcash_client_backend::data_api::{Account, WalletRead, WalletWrite};
use zcash_client_backend::fees::StandardFeeRule;
use zcash_client_backend::proposal::Proposal;
use zcash_client_backend::wallet::OvkPolicy;
use zcash_client_sqlite::ReceivedNoteId;
use zcash_client_sqlite::util::SystemClock;
use zcash_primitives::block::BlockHash;
use zcash_primitives::transaction::Transaction;
use zcash_protocol::TxId;
use zcash_protocol::consensus::NetworkType;
use zcash_protocol::value::Zatoshis as ProtoZat;

use crate::error::WalletError;
use crate::test_support::{FileBackedDsf, HarnessState, MemCache};

/// The note each wallet holds, and the payment each signs.
const NOTE_VALUE: u64 = 60_000;
const PAYMENT: u64 = 10_000;

/// Untrusted-incoming spendable depth under the default policy (the same
/// constant `send`'s own harness mines to).
const SPENDABLE_DEPTH: usize = 10;

/// A harness wallet over a temp FILE. The `TempDir` is returned with the
/// state and must outlive it.
fn harness() -> (HarnessState, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let state = TestBuilder::new()
        .with_data_store_factory(FileBackedDsf {
            path: dir.path().join("wallet.db"),
        })
        .with_block_cache(MemCache::default())
        .with_account_from_sapling_activation(BlockHash([0; 32]))
        .build();
    (state, dir)
}

/// Which pool the one funding note is mined into.
#[derive(Clone, Copy)]
enum Pool {
    Sapling,
    Orchard,
}

/// A wallet holding ONE spendable note of `NOTE_VALUE` in `pool`: the note,
/// then empty blocks to the spendable depth, all scanned, the tip published.
fn funded(pool: Pool) -> (HarnessState, tempfile::TempDir) {
    let (mut st, dir) = harness();
    let value = ProtoZat::const_from_u64(NOTE_VALUE);
    let note_h = match pool {
        Pool::Sapling => {
            let fvk = st.test_account_sapling().cloned().expect("sapling fvk");
            st.generate_next_block(&fvk, AddressType::DefaultExternal, value)
                .0
        }
        Pool::Orchard => {
            let fvk = st.test_account_orchard().cloned().expect("orchard fvk");
            st.generate_next_block(&fvk, AddressType::DefaultExternal, value)
                .0
        }
    };
    let mut tip = note_h;
    for _ in 0..(SPENDABLE_DEPTH - 1) {
        let (h, _) = st.generate_empty_block();
        tip = h;
    }
    st.scan_cached_blocks(note_h, SPENDABLE_DEPTH);
    st.wallet_mut()
        .update_chain_tip(tip)
        .expect("update chain tip");
    (st, dir)
}

/// A Sapling recipient that is not the wallet's own.
fn sapling_recipient() -> ZcashAddress {
    use zcash_address::ToAddress;
    let extsk = sapling_crypto::zip32::ExtendedSpendingKey::master(&[0x5b; 32])
        .expect("a Sapling master key");
    let (_, addr) = extsk.default_address();
    ZcashAddress::from_sapling(NetworkType::Regtest, addr.to_bytes())
}

/// An Orchard-only unified address that is not the wallet's own, derived
/// from a real key so the builder can encrypt to it.
fn orchard_recipient() -> ZcashAddress {
    use zcash_address::ToAddress;
    use zcash_address::unified::{Address as Ua, Encoding, Receiver, Revision, Uitem};
    let sk = Option::<orchard::keys::SpendingKey>::from(orchard::keys::SpendingKey::from_bytes(
        [0x0c; 32],
    ))
    .expect("a valid Orchard spending key");
    let fvk = orchard::keys::FullViewingKey::from(&sk);
    let raw = fvk
        .address_at(0u32, orchard::keys::Scope::External)
        .to_raw_address_bytes();
    let ua = Ua::try_from_items(Revision::R0, vec![Uitem::Data(Receiver::Orchard(raw))])
        .expect("an Orchard-only UA");
    ZcashAddress::from_unified(NetworkType::Regtest, ua)
}

/// The production propose over the harness wallet.
fn propose(
    st: &mut HarnessState,
    to: ZcashAddress,
) -> Result<Proposal<StandardFeeRule, ReceivedNoteId>, WalletError> {
    let network = *st.network();
    let aid = st.test_account().expect("account").id();
    let request = zip321::TransactionRequest::new(vec![zip321::Payment::without_memo(
        to,
        ProtoZat::const_from_u64(PAYMENT),
    )])
    .expect("valid request");
    crate::send::propose_core(st.wallet_mut(), &network, aid, request)
}

/// Sign through the production create seam, `send::create_signed_core`, with
/// the production bundled prover. The generator is the seam's, drawn inside.
fn sign(
    st: &mut HarnessState,
    proposal: &Proposal<StandardFeeRule, ReceivedNoteId>,
) -> Result<Vec<TxId>, WalletError> {
    let network = *st.network();
    let usk = st.test_account().expect("account").usk().clone();
    let prover = crate::prover::tx_prover();
    crate::send::create_signed_core(
        st.wallet_mut(),
        &network,
        usk,
        proposal,
        prover,
        prover,
        crate::consensus::SigningPermit::assume_current_for_test(),
    )
}

/// The one transaction a single-step sign persisted, read back from the
/// wallet's own store.
fn the_signed_transaction(st: &HarnessState, txids: &[TxId]) -> Transaction {
    assert_eq!(
        txids.len(),
        1,
        "a single-step payment signs one transaction"
    );
    st.wallet()
        .get_transaction(txids[0])
        .expect("the store reads")
        .expect("the signed transaction is persisted")
}

/// The Sapling spends of `tx`, as (nullifier, randomized key) bytes.
fn sapling_spends(tx: &Transaction) -> Vec<([u8; 32], [u8; 32])> {
    tx.sapling_bundle()
        .map(|b| {
            b.shielded_spends()
                .iter()
                .map(|s| (s.nullifier().0, <[u8; 32]>::from(*s.rk())))
                .collect()
        })
        .unwrap_or_default()
}

/// The Orchard actions of `tx`, as (nullifier, randomized key) bytes.
fn orchard_actions(tx: &Transaction) -> Vec<([u8; 32], [u8; 32])> {
    tx.orchard_bundle()
        .map(|b| {
            b.actions()
                .iter()
                .map(|a| (a.nullifier().to_bytes(), <[u8; 32]>::from(a.rk())))
                .collect()
        })
        .unwrap_or_default()
}

/// The two wallets are the same wallet: one account from one seed, one tip.
fn assert_same_wallet(a: &HarnessState, b: &HarnessState) {
    let ufvk = |st: &HarnessState| {
        st.test_account()
            .expect("account")
            .usk()
            .to_unified_full_viewing_key()
            .encode(st.network())
    };
    assert_eq!(
        ufvk(a),
        ufvk(b),
        "fixture: both wallets hold one account from one seed"
    );
    assert_eq!(
        a.wallet().chain_height().expect("tip"),
        b.wallet().chain_height().expect("tip"),
        "fixture: both wallets stand on one chain"
    );
}

/// Both wallets propose the same payment and sign it through the seam;
/// returns the two signed transactions.
fn sign_the_same_payment_twice(pool: Pool, to: fn() -> ZcashAddress) -> (Transaction, Transaction) {
    let (mut a, _dir_a) = funded(pool);
    let (mut b, _dir_b) = funded(pool);
    assert_same_wallet(&a, &b);

    let proposal_a = propose(&mut a, to()).expect("wallet A proposes");
    let proposal_b = propose(&mut b, to()).expect("wallet B proposes");
    assert!(
        proposal_a == proposal_b,
        "fixture: the two wallets build the SAME proposal (same inputs, outputs, fee, \
         target and anchor), so only the generator can tell their transactions apart"
    );
    let step = &proposal_a.steps()[0];
    assert!(
        step.transparent_inputs().is_empty(),
        "fixture: the proposal spends no transparent input"
    );
    assert!(
        step.shielded_inputs().is_some(),
        "fixture: the proposal spends a SHIELDED note (a transparent spend signs \
         deterministically and proves nothing here)"
    );

    let txids_a = sign(&mut a, &proposal_a).expect("wallet A signs");
    let txids_b = sign(&mut b, &proposal_b).expect("wallet B signs");
    (
        the_signed_transaction(&a, &txids_a),
        the_signed_transaction(&b, &txids_b),
    )
}

/// Contract step 2b, named test 1 (C-L3, R-8): the same SHIELDED payment,
/// signed twice from one seed over one chain, never yields the same
/// transaction, and never the same authorizing data.
///
/// Mutant (the contract's): the seam hands every create a generator seeded
/// with one fixed seed. Red: both transactions are then byte-identical, which
/// the control at the end of this file measures directly.
#[test]
fn the_same_shielded_payment_signed_twice_never_yields_the_same_transaction() {
    let (tx_a, tx_b) = sign_the_same_payment_twice(Pool::Sapling, sapling_recipient);

    let spends_a = sapling_spends(&tx_a);
    let spends_b = sapling_spends(&tx_b);
    assert!(
        !spends_a.is_empty() && !spends_b.is_empty(),
        "fixture: both transactions carry a Sapling spend, the signature under test"
    );
    // The premise: both spend the SAME note. A nullifier is a function of the
    // note and the key alone, never of the generator.
    let nullifiers = |s: &[([u8; 32], [u8; 32])]| s.iter().map(|(nf, _)| *nf).collect::<Vec<_>>();
    assert_eq!(
        nullifiers(&spends_a),
        nullifiers(&spends_b),
        "fixture: both transactions spend the same note (equal nullifiers)"
    );

    assert_ne!(
        tx_a.txid(),
        tx_b.txid(),
        "the same payment signed twice produced the same txid: the two creates drew the \
         same randomness"
    );
    assert_ne!(
        tx_a.auth_commitment(),
        tx_b.auth_commitment(),
        "the same payment signed twice produced the same proofs and signatures: the \
         signing nonce and the key randomizer are predictable"
    );
    for (_, rk_a) in &spends_a {
        assert!(
            spends_b.iter().all(|(_, rk_b)| rk_b != rk_a),
            "a randomized spend key repeated across two signs of the same note: `alpha` \
             was drawn twice from the same stream"
        );
    }
}

/// A case the contract does not name: the same property over an ORCHARD
/// spend. The contract's risk names Orchard's key randomizer
/// (`orchard-0.16.0/src/builder.rs:449`) and RedPallas beside Sapling's, but
/// its named row funds the harness's default Sapling note, so Orchard's half
/// of the signing path would otherwise go unmeasured.
///
/// Mutant: as the Sapling row's. Red the same way.
#[test]
fn the_same_orchard_payment_signed_twice_never_yields_the_same_transaction() {
    let (tx_a, tx_b) = sign_the_same_payment_twice(Pool::Orchard, orchard_recipient);

    let actions_a = orchard_actions(&tx_a);
    let actions_b = orchard_actions(&tx_b);
    assert!(
        !actions_a.is_empty() && !actions_b.is_empty(),
        "fixture: both transactions carry Orchard actions, the signatures under test"
    );
    assert!(
        sapling_spends(&tx_a).is_empty() && sapling_spends(&tx_b).is_empty(),
        "fixture: the Orchard note funds the payment alone (no Sapling spend)"
    );
    // The premise: the real spend's nullifier appears in both (the note is the
    // same). The dummy spends' nullifiers come from the generator, so they are
    // part of the property, asserted below.
    let shared = actions_a
        .iter()
        .filter(|(nf, _)| actions_b.iter().any(|(other, _)| other == nf))
        .count();
    assert!(
        shared >= 1,
        "fixture: the funding note's nullifier is common to both transactions"
    );

    assert_ne!(
        tx_a.txid(),
        tx_b.txid(),
        "the same Orchard payment signed twice produced the same txid"
    );
    assert_ne!(
        tx_a.auth_commitment(),
        tx_b.auth_commitment(),
        "the same Orchard payment signed twice produced the same proofs and signatures"
    );
    assert_eq!(
        shared, 1,
        "a dummy spend's nullifier repeated across two signs: the dummy notes were drawn \
         from the same stream"
    );
    for (_, rk_a) in &actions_a {
        assert!(
            actions_b.iter().all(|(_, rk_b)| rk_b != rk_a),
            "a randomized Orchard spend key repeated across two signs of the same note"
        );
    }
}

/// A deterministic stream: the model of the contract's mutant, "one
/// fixed-seed generator". Test code only, and only ever handed to the raw
/// engine call below; it never reaches a production path.
struct FixedStream(u64);

impl FixedStream {
    fn new() -> Self {
        Self(0x2b2b_2b2b_2b2b_2b2b)
    }

    /// SplitMix64: well spread, so no draw lands on a degenerate value.
    fn step(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

impl TryRng for FixedStream {
    type Error = std::convert::Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
        Ok(self.step() as u32)
    }

    fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
        Ok(self.step())
    }

    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
        for chunk in dst.chunks_mut(8) {
            let bytes = self.step().to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
        }
        Ok(())
    }
}

impl TryCryptoRng for FixedStream {}

/// The raw engine create, fed a fresh `FixedStream` (the mutant's shape: each
/// create gets a generator from the same fixed seed).
fn sign_with_a_fixed_stream(
    st: &mut HarnessState,
    proposal: &Proposal<StandardFeeRule, ReceivedNoteId>,
) -> Vec<TxId> {
    let network = *st.network();
    let usk = st.test_account().expect("account").usk().clone();
    let prover = crate::prover::tx_prover();
    create_proposed_transactions::<
        _,
        _,
        std::convert::Infallible,
        StandardFeeRule,
        std::convert::Infallible,
        _,
    >(
        st.wallet_mut(),
        &network,
        &SystemClock,
        &mut FixedStream::new(),
        prover,
        prover,
        &SpendingKeys::from_unified_spending_key(usk),
        OvkPolicy::Sender,
        proposal,
        None,
    )
    .expect("the fixed-stream create signs")
    .into_iter()
    .collect()
}

/// The control for the two rows above (IT-10: which mechanism makes them
/// pass?). Two identical wallets fed ONE fixed stream sign byte-identical
/// transactions: same txid, same authorizing data, same serialized bytes. So
/// the generator is the only input that tells the two signs apart, the rows
/// above go red under the contract's fixed-seed mutant, and they pass only
/// because the seam reads a live source.
///
/// If this row ever fails, the engine gained an entropy source of its own
/// (or a clock reached the transaction): the rows above would then stay green
/// under a fixed seam, and their mutant would survive.
#[test]
fn one_fixed_stream_fed_to_two_identical_wallets_signs_byte_identical_transactions() {
    let (mut a, _dir_a) = funded(Pool::Sapling);
    let (mut b, _dir_b) = funded(Pool::Sapling);
    assert_same_wallet(&a, &b);
    let proposal_a = propose(&mut a, sapling_recipient()).expect("wallet A proposes");
    let proposal_b = propose(&mut b, sapling_recipient()).expect("wallet B proposes");
    assert!(proposal_a == proposal_b, "fixture: the same proposal");

    let txids_a = sign_with_a_fixed_stream(&mut a, &proposal_a);
    let txids_b = sign_with_a_fixed_stream(&mut b, &proposal_b);
    let tx_a = the_signed_transaction(&a, &txids_a);
    let tx_b = the_signed_transaction(&b, &txids_b);

    assert!(
        !sapling_spends(&tx_a).is_empty(),
        "fixture: the control signs a Sapling spend, the same shape as the row it controls"
    );
    assert_eq!(
        tx_a.txid(),
        tx_b.txid(),
        "one fixed stream gave two different txids: something other than the generator \
         varies between two signs of the same payment"
    );
    assert_eq!(
        tx_a.auth_commitment(),
        tx_b.auth_commitment(),
        "one fixed stream gave two different authorizing commitments"
    );
    let bytes = |tx: &Transaction| {
        let mut out = Vec::new();
        tx.write(&mut out).expect("serialize");
        out
    };
    assert_eq!(
        bytes(&tx_a),
        bytes(&tx_b),
        "one fixed stream gave two different transactions"
    );
}
