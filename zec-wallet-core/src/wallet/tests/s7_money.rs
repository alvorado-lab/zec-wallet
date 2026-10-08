//! Stage S7 `angles` — the S1 fix's rows: the scoped UTXO poll's put cap
//! (`docs/plan/stage-7-review/fix-design.md` S1, Revision 2, and the S7 fold).
//! A child of `wallet::tests` for its fixtures (`synced_wallet_with_taddr`,
//! `intozec_quoted_wallet`, `FakeUtxoSource`, `script_paying`, `decode_taddr`,
//! `swap_dest_count`), in its own file so no cited line of `wallet.rs` moves.
//!
//! Every returned record is validated; at most `SCOPED_UTXO_POLL_MAX_PUTS` are
//! put — NEW outputs first (swap destinations before index-0), the ones the
//! engine already stores unspent last — and a pass whose NEW outputs exceed the
//! cap is `truncated`, counted apart from `rejected`.

use super::*;
use crate::constants::SCOPED_UTXO_POLL_MAX_PUTS;
use crate::transparent::{ExpectedReceiver, TransparentUtxoRecord};

/// `n` DISTINCT valid records (distinct txids) paying `to`.
fn records_paying(
    to: &zcash_transparent::address::TransparentAddress,
    n: usize,
    tag: u8,
) -> Vec<TransparentUtxoRecord> {
    (0..n)
        .map(|i| {
            let mut txid = vec![tag; 32];
            txid[..4].copy_from_slice(&u32::try_from(i).expect("small").to_le_bytes());
            TransparentUtxoRecord {
                txid,
                index: 0,
                script: script_paying(to),
                value_zat: 10_000,
                height: 281_000,
            }
        })
        .collect()
}

/// The engine's stored-unspent outputs, read the way the production poll reads them.
fn stored_outputs(w: &Wallet) -> crate::transparent::StoredOutputs {
    let aux = w.inner.aux_db.lock().expect("aux db");
    let read = crate::transparent::stored_unspent_outputs(&aux);
    assert!(!read.unread, "the stored read succeeds on a healthy wallet");
    read
}

/// The stored-unspent outpoints alone.
fn stored(w: &Wallet) -> std::collections::HashSet<crate::transparent::StoredOutpoint> {
    stored_outputs(w).observed.keys().copied().collect()
}

/// One pass through the PRODUCTION door (`refresh_transparent_utxos`), the
/// endpoint answering `records`.
async fn refresh(w: &Wallet, records: Vec<TransparentUtxoRecord>) {
    let mut src = FakeUtxoSource::ok(records);
    w.refresh_transparent_utxos(&mut src)
        .await
        .expect("the production poll");
}

/// The outpoint a record names, in the engine's terms.
fn outpoint_of(record: &TransparentUtxoRecord) -> crate::transparent::StoredOutpoint {
    (
        record.txid.as_slice().try_into().expect("32 bytes"),
        u32::try_from(record.index).expect("index"),
    )
}

/// One scoped poll through the production door: the stored set read on the aux
/// connection, then validate + put under the db lock.
async fn poll(
    w: &Wallet,
    expecteds: Vec<ExpectedReceiver>,
    records: Vec<TransparentUtxoRecord>,
) -> crate::transparent::ScopedRefreshOutcome {
    let known = stored_outputs(w);
    let inner = Arc::clone(&w.inner);
    run_blocking(move || {
        let mut guard = inner.db.lock().expect("wallet db mutex poisoned");
        crate::transparent::refresh_account_transparent_scoped_known(
            &mut guard.db,
            &expecteds,
            &records,
            &known,
        )
    })
    .await
    .expect("the scoped poll")
}

fn index0(receiver: zcash_transparent::address::TransparentAddress) -> Vec<ExpectedReceiver> {
    vec![ExpectedReceiver {
        swap_id: None,
        receiver,
    }]
}

/// The cap at its boundary: exactly the cap of NEW outputs is put whole and not
/// truncated; one NEW output over it puts the cap and says `truncated` — the
/// excess is not a reject.
#[tokio::test]
async fn a_scoped_poll_over_the_put_cap_puts_the_cap_and_reports_truncated() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, _addr, taddr) = synced_wallet_with_taddr(dir.path(), &vault).await;
    let at_cap = records_paying(&taddr, SCOPED_UTXO_POLL_MAX_PUTS, 0x5a);
    let out = poll(&w, index0(taddr), at_cap).await;
    assert_eq!(out.put, SCOPED_UTXO_POLL_MAX_PUTS, "at the cap: all put");
    assert!(!out.truncated, "at the cap: not truncated");

    let over = records_paying(&taddr, SCOPED_UTXO_POLL_MAX_PUTS + 1, 0x5b);
    let over = poll(&w, index0(taddr), over).await;
    assert_eq!(
        over.put, SCOPED_UTXO_POLL_MAX_PUTS,
        "one NEW output over the cap: EXACTLY the cap is put"
    );
    assert!(over.truncated, "one NEW output over the cap: truncated");
    assert_eq!(
        over.rejected, 0,
        "an honest high-UTXO address is not a lying server — nothing rejected"
    );
    w.close().await.expect("close");
}

/// The S7 fold's MEDIUM: over a thousand UTXOs the wallet ALREADY stores (dust
/// sent to index-0) must never starve a later NEW receive. 1025 stored + 1 new
/// at the END of the reply: the new one is put, and the pass is not truncated.
/// Driven through the PRODUCTION door (`refresh_transparent_utxos`), so the aux
/// stored read itself is under test, not only the ranking it feeds.
#[tokio::test]
async fn stored_dust_over_the_cap_never_starves_a_new_receive() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, _addr, taddr) = synced_wallet_with_taddr(dir.path(), &vault).await;
    let dust = records_paying(&taddr, SCOPED_UTXO_POLL_MAX_PUTS + 1, 0x6c);
    let (head, tail) = dust.split_at(SCOPED_UTXO_POLL_MAX_PUTS); // two disjoint passes store all
    refresh(&w, head.to_vec()).await;
    refresh(&w, tail.to_vec()).await;
    let known = stored(&w);
    assert!(
        dust.iter().all(|r| known.contains(&outpoint_of(r))),
        "precondition: every dust UTXO is stored unspent"
    );

    let fresh = records_paying(&taddr, 1, 0x6d).remove(0);
    let mut reply = dust.clone();
    reply.push(fresh.clone());
    refresh(&w, reply).await;
    assert!(
        stored(&w).contains(&outpoint_of(&fresh)),
        "the NEW receive is put even behind 1025 stored dust UTXOs"
    );

    // The same shape at the ranking door reads not-truncated (one NEW output).
    let fresh2 = records_paying(&taddr, 1, 0x6e).remove(0);
    let mut reply = dust;
    reply.push(fresh2);
    let out = poll(&w, index0(taddr), reply).await;
    assert!(
        !out.truncated,
        "one NEW output is under the cap: not truncated"
    );
    w.close().await.expect("close");
}

/// The S7 fold, LOW: a stored output with a recorded SPEND is not "stored
/// unspent" — the read's `WHERE s.transaction_id IS NULL` is what says so.
#[tokio::test]
async fn a_spent_stored_output_is_not_read_as_stored_unspent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, _addr, taddr) = synced_wallet_with_taddr(dir.path(), &vault).await;
    let utxo = records_paying(&taddr, 1, 0x7a).remove(0);
    refresh(&w, vec![utxo.clone()]).await;
    assert!(
        stored(&w).contains(&outpoint_of(&utxo)),
        "precondition: the put output reads stored unspent"
    );
    // Record a spend of it the way the engine's junction does (the spending tx is
    // immaterial to the read; its own row stands in).
    {
        let aux = w.inner.aux_db.lock().expect("aux db");
        let n = aux
            .execute(
                "INSERT INTO transparent_received_output_spends \
                 (transparent_received_output_id, transaction_id) \
                 SELECT id, transaction_id FROM transparent_received_outputs",
                [],
            )
            .expect("record the spend");
        assert_eq!(n, 1, "precondition: one spend recorded");
    }
    assert!(
        !stored(&w).contains(&outpoint_of(&utxo)),
        "a spent output is not stored-unspent"
    );
    w.close().await.expect("close");
}

/// The S7 fold, (b): a NEW swap-destination output is put before NEW index-0
/// outputs, so a flood at index-0 cannot delay a delivery the wallet watches.
#[tokio::test]
#[cfg(feature = "swap")]
async fn a_new_destination_output_is_put_before_new_index0_outputs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, index0_addr, dest) = intozec_quoted_wallet(dir.path(), &vault).await;
    let dest_receiver =
        crate::derivation::transparent_receiver_from_stored_address(Network::Test, &dest)
            .expect("decode");
    let index0_receiver = decode_taddr(&index0_addr);
    let expecteds = vec![
        ExpectedReceiver {
            swap_id: None,
            receiver: index0_receiver,
        },
        ExpectedReceiver {
            swap_id: Some("dest".to_string()),
            receiver: dest_receiver,
        },
    ];
    let delivery = records_paying(&dest_receiver, 1, 0x71).remove(0);
    let mut reply = records_paying(&index0_receiver, SCOPED_UTXO_POLL_MAX_PUTS, 0x72);
    reply.push(delivery.clone());
    let out = poll(&w, expecteds, reply).await;
    assert!(
        out.truncated,
        "precondition: 1025 NEW outputs, cut at the cap"
    );
    assert!(
        stored(&w).contains(&outpoint_of(&delivery)),
        "the destination's NEW output is put first, even last in the reply"
    );
    w.close().await.expect("close");
}

/// The S7 fold, (c): with the truncated gate gone, a funded destination whose
/// stored UTXO the cap did NOT re-put this pass is still funded (attribution is
/// from every validated match), so the retire debounce never counts it empty.
#[tokio::test]
#[cfg(feature = "swap")]
async fn a_funded_destination_the_cap_did_not_reput_is_not_retired() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, index0_addr, dest) = intozec_quoted_wallet(dir.path(), &vault).await;
    let dest_receiver =
        crate::derivation::transparent_receiver_from_stored_address(Network::Test, &dest)
            .expect("decode");
    let delivery = records_paying(&dest_receiver, 1, 0x33);

    // Pass 1: the provider delivered — the destination is funded and its UTXO stored.
    let mut src1 = FakeUtxoSource::ok(delivery.clone());
    let out1 = w
        .refresh_transparent_utxos(&mut src1)
        .await
        .expect("poll 1");
    assert_eq!(out1.put, 1, "the delivery is detected + put");

    // RETIRE_AFTER_EMPTY_PASSES passes of 1025 NEW index-0 outputs plus the (stored)
    // delivery: the cap is spent on the new ones, the delivery is not re-put.
    let index0_receiver = decode_taddr(&index0_addr);
    for pass in 1..=crate::swap_destination_store::RETIRE_AFTER_EMPTY_PASSES {
        let tag = 0x40 + u8::try_from(pass).expect("small");
        let mut reply = records_paying(&index0_receiver, SCOPED_UTXO_POLL_MAX_PUTS + 1, tag);
        reply.extend(delivery.clone());
        let mut src = FakeUtxoSource::ok(reply);
        let out = w
            .refresh_transparent_utxos(&mut src)
            .await
            .expect("capped poll");
        assert_eq!(
            out.put, SCOPED_UTXO_POLL_MAX_PUTS,
            "precondition: pass {pass} was cut at the cap"
        );
        assert_eq!(
            swap_dest_count(&w.inner),
            1,
            "pass {pass}: the delivery was validated, so the destination is funded — not retired"
        );
    }
    w.close().await.expect("close");
}

/// The S7 second fold, LOW: a failing stored read never aborts the money-seeing
/// poll — an aux fault (no such table) or a malformed row (a short txid) yields
/// the EMPTY set with `unread`, so every output ranks NEW (money-safe) and the
/// span's `outcome` says so.
#[test]
fn a_failing_stored_read_falls_back_to_the_empty_set() {
    use crate::transparent::{ScopedRefreshOutcome, stored_unspent_outputs};
    let absent = rusqlite::Connection::open_in_memory().expect("conn");
    let read = stored_unspent_outputs(&absent);
    assert!(read.unread, "an aux fault is reported unread");
    assert!(read.observed.is_empty(), "and the set is empty");

    let bad = rusqlite::Connection::open_in_memory().expect("conn");
    bad.execute_batch(
        "CREATE TABLE transactions (id_tx INTEGER PRIMARY KEY, txid BLOB);
         CREATE TABLE transparent_received_outputs (id INTEGER PRIMARY KEY,
             transaction_id INTEGER, output_index INTEGER, max_observed_unspent_height INTEGER);
         CREATE TABLE transparent_received_output_spends (
             transparent_received_output_id INTEGER, transaction_id INTEGER);
         INSERT INTO transactions VALUES (1, x'0102');
         INSERT INTO transparent_received_outputs VALUES (1, 1, 0, NULL);",
    )
    .expect("schema");
    let read = stored_unspent_outputs(&bad);
    assert!(read.unread, "a malformed txid is reported unread");
    assert!(read.observed.is_empty(), "and the set is empty");
    let outcome = ScopedRefreshOutcome {
        put: 0,
        rejected: 0,
        truncated: true,
        stored_unread: read.unread,
        funded_swap_ids: std::collections::HashSet::new(),
    };
    assert_eq!(outcome.outcome(), "stored_unread", "the span names it");
}
