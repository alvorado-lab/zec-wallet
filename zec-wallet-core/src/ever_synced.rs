//! The durable "this wallet has reached chain tip at least once" flag (#357).
//!
//! One write-once row recording WHEN this wallet's sync first caught up to the
//! chain tip. Its PRESENCE is the durable answer to "has the user ever seen this
//! wallet fully synced?" — the fact the reference UI's catch-up cue needs to stop
//! over-explaining ("history is filling in…") after a process death.
//!
//! **Why a durable flag and not the in-memory latch (#357, spec FR-1b).** The
//! reference UI derived "first-run catch-up" from the durable pair (`last_synced
//! == null` + below-tip) BACKSTOPPED by a SESSION-LOCAL proved-tip latch. The
//! latch is lost on every process death (app update / LMK kill), so a relaunch
//! mid-catch-up re-showed the first-time reassurance for a wallet the user has
//! synced many times. This flag makes the latch durable: set once at tip, read
//! at `open`, surfaced to the host — the framing is suppressed across relaunches
//! WITHOUT suppressing honest below-tip sync progress.
//!
//! **UX-only, NEVER a money input.** A wrong flag can only mis-frame the sync
//! cue, never move funds. Fail-open accordingly: [`read`] is INFALLIBLE — any
//! fault (missing table, garbage cell, SQL error) degrades to `false` (the
//! pre-#357 "show the first-run framing" arm), the SAFE direction (a wallet that
//! HAS synced briefly re-explains; the converse — suppressing the framing for a
//! genuinely-first-syncing wallet — is the worse, more-confusing miss). The
//! writer swallows its own faults (a flag write must never fail a sync pass).
//!
//! **Write-once, earliest wins.** [`set_once`] is `INSERT … ON CONFLICT DO
//! NOTHING`: the FIRST tip-reaching pass stamps it and every later pass no-ops.
//! Set at the same clean-pass seam as [`crate::sync_stamp`] (the controller's
//! `record_synced`), gated on the pass having reached tip — which, since
//! T0-1c-R2, means a tip the grade found at or above what this wallet already
//! holds (`sync_controller::emit_synced` skips the call on a pass whose tip
//! graded behind the bundle's row or the wallet's own scanned height): a
//! behind server's pass has not "reached chain tip", and a wallet whose only
//! passes were behind stays `false`, with the first-run framing, honestly.
//!
//! **Storage.** Same discipline as [`crate::creation_stamp`] / [`crate::
//! sync_stamp`]: pure SQL over the aux SQLCipher connection to the ONE
//! sealed/wiped/backup-excluded `wallet.db`; created idempotently in
//! `db::migrate` (`ensure_aux_tables`) with no schema bump. Single-statement
//! writes take `RESERVED` atomically, so the aux-write deadlock-freedom
//! invariant holds.
//!
//! **Rescan CLEARS the row** (the `sync_stamp` precedent, NOT `creation_stamp`'s
//! preserve). This flag is the DURABLE equivalent of the reference UI's
//! session-local proved-tip latch — and that latch RESETS on a rescan's session
//! swap. A rescan rebuilds balances from the birthday, so the wallet is genuinely
//! catching up AGAIN (balance reads low until the first post-rescan reached-tip);
//! the catch-up cue MUST re-show during that rebuild — the "did I lose funds?"
//! reassurance the whole feature exists for. If the flag rode UNCLEARED it would
//! make `everSynced == true && lastSynced == null` ambiguous (settled-with-stale-
//! stamp vs rebuilding), and a process death + offline relaunch mid-rebuild would
//! SUPPRESS the reassurance over a rebuilding balance (the post-ship
//! reliability/UX HIGH). So the rescan rebuild CLEARS the row on the temp DB
//! BEFORE the atomic rename (the `sync_stamp` copy-then-clear discipline), and the
//! first post-rescan clean pass re-sets it. The TABLE still rides
//! `db::copy_aux_tables` (keeping `AUX_TABLES_PRESERVED` == the ensure set,
//! ADR-0534); only its ROW is cleared. It dies with the identity — the whole
//! `wallet.db` is crypto-shredded at wipe.

use rusqlite::{Connection, OptionalExtension};

use crate::db::map_aux_err;
use crate::error::WalletError;

/// The single-row table name — referenced by `db::AUX_TABLES_PRESERVED`
/// (ADR-0534).
pub(crate) const TABLE: &str = "wallet_ever_synced";

/// Create the flag table (idempotent; runs in `db::migrate` on every provision
/// AND open, so existing wallets gain it with no schema bump — a pre-#357 wallet
/// simply has no row, which reads `false` = the pre-#357 "show first-run framing"
/// arm).
pub(crate) fn ensure_table(conn: &Connection) -> Result<(), WalletError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS wallet_ever_synced (
             singleton    INTEGER PRIMARY KEY CHECK (singleton = 1),
             at_unix_secs INTEGER NOT NULL
         );",
    )
    .map_err(map_aux_err)
}

/// Set the flag ONCE (single row, single statement — crash-atomic, takes
/// `RESERVED` directly per the aux-write invariant). `ON CONFLICT DO NOTHING`
/// keeps the EARLIEST tip-time across idempotent re-runs. `at_unix_secs` is the
/// caller's wall clock, kept for observability only (never read as a money
/// input); a hostile/broken clock saturates the i64 cast rather than panicking.
pub(crate) fn set_once(conn: &Connection, at_unix_secs: u64) -> Result<(), WalletError> {
    let at = i64::try_from(at_unix_secs).unwrap_or(i64::MAX);
    conn.execute(
        "INSERT INTO wallet_ever_synced (singleton, at_unix_secs) VALUES (1, ?1)
         ON CONFLICT(singleton) DO NOTHING",
        rusqlite::params![at],
    )
    .map_err(map_aux_err)?;
    Ok(())
}

/// Clear the flag — the rescan reset (see the module doc). Called on the rescan
/// temp DB BEFORE the atomic rename, so a rebuilding wallet re-shows the catch-up
/// cue until its first post-rescan reached-tip re-sets the flag. Single statement
/// — crash-atomic; a kill leaves either the intact-old DB (flag set) or the
/// complete-new DB (flag cleared), never a torn state.
pub(crate) fn clear(conn: &Connection) -> Result<(), WalletError> {
    conn.execute("DELETE FROM wallet_ever_synced", [])
        .map_err(map_aux_err)?;
    Ok(())
}

/// Read the flag — `true` iff this wallet has reached tip at least once SINCE the
/// last rescan (or ever, if no rescan). INFALLIBLE by design (UX-only; absence is
/// the safe "show first-run framing" direction): a read fault degrades to `false`
/// with a logged warning rather than failing the caller's `open`. No value is
/// logged (§5.4).
pub(crate) fn read(conn: &Connection) -> bool {
    let row: Result<Option<i64>, rusqlite::Error> = conn
        .query_row(
            "SELECT 1 FROM wallet_ever_synced WHERE singleton = 1",
            [],
            |r| r.get(0),
        )
        .optional();
    match row {
        Ok(v) => v.is_some(),
        Err(_) => {
            tracing::warn!(
                target: "zec_wallet_core",
                "ever-synced flag unreadable; treating as never-synced (first-run framing, UX-safe)"
            );
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn() -> Connection {
        let c = Connection::open_in_memory().expect("in-memory db");
        ensure_table(&c).expect("ensure");
        c
    }

    #[test]
    fn read_is_false_before_any_set() {
        let c = conn();
        assert!(!read(&c));
    }

    #[test]
    fn set_once_flips_the_flag_and_is_write_once() {
        let c = conn();
        set_once(&c, 1_751_000_000).expect("set");
        assert!(read(&c));
        // A later pass must not add a second row nor move the stamp.
        set_once(&c, 1_751_999_999).expect("set again");
        assert!(read(&c));
        let rows: i64 = c
            .query_row("SELECT COUNT(*) FROM wallet_ever_synced", [], |r| r.get(0))
            .expect("count");
        assert_eq!(rows, 1);
        let at: i64 = c
            .query_row(
                "SELECT at_unix_secs FROM wallet_ever_synced WHERE singleton = 1",
                [],
                |r| r.get(0),
            )
            .expect("at");
        assert_eq!(at, 1_751_000_000, "earliest tip-time is kept");
    }

    #[test]
    fn ensure_table_is_idempotent_and_never_clobbers_the_row() {
        let c = conn();
        set_once(&c, 42).expect("set");
        ensure_table(&c).expect("re-ensure");
        assert!(read(&c));
    }

    #[test]
    fn clear_resets_to_false_and_a_second_clear_is_a_no_op() {
        // The rescan reset: a rebuilding wallet must re-show the catch-up cue.
        let c = conn();
        set_once(&c, 1_751_000_000).expect("set");
        assert!(read(&c));
        clear(&c).expect("clear");
        assert!(
            !read(&c),
            "rescan clears the flag → cue re-shows during rebuild"
        );
        clear(&c).expect("clear again");
        assert!(!read(&c));
        // …and a later reached-tip re-sets it (the first post-rescan clean pass).
        set_once(&c, 1_752_000_000).expect("re-set");
        assert!(read(&c));
    }

    #[test]
    fn a_missing_table_reads_false_never_an_error() {
        // The infallible read: a store missing the table (impossible in
        // production — migrate creates it — but the fail-open contract must hold)
        // degrades to `false`, never a panic or a bricked open.
        let c = Connection::open_in_memory().expect("in-memory db");
        assert!(!read(&c));
    }

    #[test]
    fn a_hostile_clock_saturates_instead_of_panicking() {
        let c = conn();
        set_once(&c, u64::MAX).expect("set");
        assert!(read(&c));
    }
}
