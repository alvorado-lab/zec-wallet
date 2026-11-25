//! The absent-answer contract's rows that name the cursor itself (`EnhancementCursor`, which the
//! implementation adds): row 7's random start, and row 9 read off the cursor while a fetch hangs.
//! They compile only against that name, so they were written apart from the rows beside them and
//! joined by this module's declaration.
//!
//! What these rows take for granted about the name, and nothing more: `EnhancementCursor::new()`
//! is a fresh cursor, `record(txid)` moves it, and `get()` reads it as an `Option<TxId>`.

use super::*;

// A txid is wallet metadata: the cursor prints nowhere (C4, §5.4). And it sits in the wallet's
// shared state, so it crosses threads.
static_assertions::assert_not_impl_any!(EnhancementCursor: std::fmt::Debug, std::fmt::Display);
static_assertions::assert_impl_all!(EnhancementCursor: Send, Sync);

/// **Row 7's last clause (C4).** A cursor is born at a RANDOM txid, so a host that runs one pass
/// per wake-up does not ask the same first fifty every time: over 64 fresh cursors, the first
/// request asked lies in both halves of a fixed list spread evenly over the txids there are.
///
/// A cursor born at a fixed txid (or at none) starts every one of them at the same request. A
/// uniform one leaves a half unreached once in 2^63 runs.
#[test]
fn a_fresh_cursor_starts_at_a_random_point_of_the_list() {
    let none = UnparseableTxids::default();
    let listed: Vec<TxId> = (0..128u8).map(|i| t(i * 2 + 1)).collect();
    let mut reached: BTreeSet<usize> = BTreeSet::new();
    for _ in 0..64 {
        let cursor = EnhancementCursor::new();
        assert!(cursor.get().is_some(), "a fresh cursor stands somewhere");
        let first = join::select(
            listed
                .iter()
                .map(|txid| TransactionDataRequest::Enhancement(*txid))
                .collect(),
            1,
            &none,
            cursor.get(),
        )[0]
        .0;
        reached.insert(
            listed
                .iter()
                .position(|txid| *txid == first)
                .expect("the first request asked is a listed one"),
        );
    }
    let half = listed.len() / 2;
    assert!(
        reached.iter().any(|i| *i < half) && reached.iter().any(|i| *i >= half),
        "both halves of the list are reached by 64 fresh cursors, got {reached:?}"
    );
}

/// **Row 9, read off the cursor (C4).** While a fetch is awaited the cursor already stands at the
/// txid being asked, and it can be read: it is written before the fetch, and no lock on it is held
/// across the await (a held one would block this read for good). Dropping the pass leaves it
/// there; a pass that runs to its end leaves it at the last txid asked.
#[tokio::test]
async fn the_cursor_stands_at_the_txid_being_asked_while_its_fetch_is_awaited() {
    let five = [t(1), t(2), t(3), t(4), t(5)];
    let store = RecordingStore::with_backlog(&five, Some(BlockHeight::from_u32(TIP)));
    let mut fetcher = ScriptedFetcher::new(Vec::new())
        .scripting(t(3), vec![Reply::Hang])
        .answering_the_rest(Reply::Status(GRPC_NOT_FOUND));
    let cursor = join::Cursor::at_the_end();
    let unparseable = UnparseableTxids::default();
    {
        let hanging = std::sync::Arc::clone(&fetcher.hanging);
        let mut pass = Box::pin(join::pass(
            &store,
            &mut fetcher,
            &MainNetwork,
            &unparseable,
            &cursor,
        ));
        tokio::select! {
            biased;
            _ = &mut pass => panic!("the fixture: a pass over a hanging fetch does not end"),
            () = hanging.notified() => {}
        }
        assert_eq!(
            cursor.get(),
            Some(t(3)),
            "written before the fetch, and readable while it is awaited"
        );
        drop(pass);
    }
    assert_eq!(cursor.get(), Some(t(3)), "the dropped pass leaves it there");

    join::pass(&store, &mut fetcher, &MainNetwork, &unparseable, &cursor)
        .await
        .expect("the next pass");
    assert_eq!(
        (cursor.get(), fetcher.calls.last().copied()),
        (Some(t(3)), Some(t(3))),
        "a pass that runs to its end leaves the cursor at the last txid it asked"
    );
}
