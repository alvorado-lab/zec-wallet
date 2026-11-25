//! The absent-answer contract's wallet-level row that names the cursor itself: row 8. It reads
//! `Inner::enhancement_cursor`, which the implementation adds, so it was written apart from the
//! rows beside it and joined by this module's declaration. [`cursor_of`] is its one join point.

use super::*;

/// Where the handle's cursor stands.
fn cursor_of(w: &Wallet) -> Option<EngineTxId> {
    w.inner.enhancement_cursor.get()
}

/// **Row 8 (C4).** The cursor is not carried over a server switch: after a switch to another
/// server by the honest-oracle path, and after one that only changes the key the wallet sends (a
/// rebuild of the session, like a new server), it is a fresh one. A choice that resolves to the
/// server already in use rebuilds nothing, and the cursor stays.
///
/// The positive control is the first assertion: a pass over the handle is what moves this cursor,
/// so "it moved" and "it stayed" are both claims about the one the passes use. The queue is the
/// engine's and survives each switch, so a pass after one asks again, and starts somewhere of its
/// own. A cursor carried in `Carried` reads the same before and after a switch; one born at a
/// fixed txid reads the same after both.
#[tokio::test]
async fn a_server_switch_restarts_the_cursor() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_at(dir.path(), &vault, SIGNED_AT).await;
    let sixty = plant_parents(&w, 60);
    let mut fetcher = CountingFetcher::default();
    let last_asked =
        |fetcher: &CountingFetcher| Some(*fetcher.calls.last().expect("something was asked"));

    pass(&w, &mut fetcher).await;
    assert_eq!(
        cursor_of(&w),
        last_asked(&fetcher),
        "control: a pass leaves the handle's cursor at the last txid it asked"
    );

    // The server already in use: the choice is remembered, and the session is the same one.
    let w = switched_via_honest_oracle(w, custom("https://zec.rocks:443"), Network::Test).await;
    assert_eq!(
        cursor_of(&w),
        last_asked(&fetcher),
        "a choice of the server in use keeps the cursor"
    );
    pass(&w, &mut fetcher).await;
    assert_walks(
        &fetcher.calls,
        &sixty,
        "and the next pass goes on from where the last one stopped",
    );

    // Another server.
    let before = cursor_of(&w);
    let w = switched_via_honest_oracle(w, custom("https://mine.example:443"), Network::Test).await;
    let on_another_server = cursor_of(&w);
    assert!(
        on_another_server.is_some() && on_another_server != before,
        "another server starts over, at a fresh point"
    );
    let out = pass(&w, &mut fetcher).await;
    assert_eq!(
        out.requested, MAX_ENHANCEMENTS_PER_PASS,
        "control: the queue survives the switch"
    );
    assert_eq!(cursor_of(&w), last_asked(&fetcher));

    // The same server with a new key: a rebuild, so it starts over too.
    let before = cursor_of(&w);
    let w = switched_via_honest_oracle(
        w,
        keyed("https://mine.example:443", "a-new-value"),
        Network::Test,
    )
    .await;
    let on_a_new_key = cursor_of(&w);
    assert!(
        on_a_new_key.is_some() && on_a_new_key != before,
        "a new key starts over, at a fresh point"
    );
    assert_ne!(
        on_a_new_key, on_another_server,
        "and each fresh point is its own: a cursor is not born at a fixed txid"
    );
    w.close().await.expect("close");
}
