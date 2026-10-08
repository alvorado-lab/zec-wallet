//! The durable "a rescan rebuild is in progress" breadcrumb (#377 s357b-2).
//!
//! One presence-only row set when a rescan swaps in its rebuilt-from-birthday
//! DB, cleared at the first post-rescan reached-tip. Its PRESENCE is the
//! durable answer to "is this wallet's emptiness a rescan rebuild?" — the one
//! distinction the #357 clear-on-rescan design could not carry across a
//! process death: after a relaunch mid-rebuild the reference UI knew only
//! "never reached tip since the last rescan" (`ever_synced` cleared) and had
//! to fall back to the GENERIC catch-up copy, indistinguishable from a
//! restore/create's first sync or a genuinely-empty offline wallet. With the
//! breadcrumb the cue can say "rebuilding after a rescan" — the user's own
//! deliberate action — which is the stronger "your funds are not lost"
//! reassurance (spec FR-1b; the review-P1-#4 panic window).
//!
//! **Lifecycle.** SET on the rescan temp DB BEFORE the atomic rename (the
//! `sync_stamp` / `ever_synced` copy-then-reset discipline, inverted): a kill
//! leaves either the intact-old DB (no breadcrumb) or the complete-new DB
//! (breadcrumb set), never a torn state. CLEARED at the same clean-pass
//! reached-tip seam that sets `ever_synced` (`record_synced`), independently
//! and fail-open — the rebuild is over exactly when the wallet proves tip.
//!
//! **UX-only, NEVER a money input.** A wrong row can only mis-name the
//! catch-up copy. Fail-open accordingly: [`read`] is INFALLIBLE — any fault
//! degrades to `false`, which falls back to the generic catch-up cue (the
//! pre-breadcrumb copy; reassurance still shows, only less specifically). A
//! stale row (clear-fault at tip) is masked by the cue's `everSynced`
//! priority and retried on every later clean pass.
//!
//! **Storage.** Same discipline as [`crate::ever_synced`]: pure SQL over the
//! aux SQLCipher connection to the ONE sealed/wiped/backup-excluded
//! `wallet.db`; created idempotently in `db::migrate` (`ensure_aux_tables`)
//! with no schema bump; the TABLE rides `db::copy_aux_tables`
//! (`AUX_TABLES_PRESERVED` == the ensure set, ADR-0534). Single-statement
//! writes take `RESERVED` atomically, so the aux-write deadlock-freedom
//! invariant holds. Dies with the identity at wipe.

use rusqlite::{Connection, OptionalExtension};

use crate::db::map_aux_err;
use crate::error::WalletError;

/// The single-row table name — referenced by `db::AUX_TABLES_PRESERVED`
/// (ADR-0534).
pub(crate) const TABLE: &str = "wallet_rescan_rebuilding";

/// Create the breadcrumb table (idempotent; runs in `db::migrate` on every
/// provision AND open, so existing wallets gain it with no schema bump — a
/// wallet mid-rebuild from a pre-breadcrumb rescan simply has no row, which
/// reads `false` = the generic catch-up copy it showed before).
pub(crate) fn ensure_table(conn: &Connection) -> Result<(), WalletError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS wallet_rescan_rebuilding (
             singleton INTEGER PRIMARY KEY CHECK (singleton = 1)
         );",
    )
    .map_err(map_aux_err)
}

/// Set the breadcrumb (single row, single statement — crash-atomic, takes
/// `RESERVED` directly per the aux-write invariant). Idempotent: a rescan over
/// a still-rebuilding wallet just keeps the row. Presence-only — the row
/// carries no clock (the rescan swap is not a display event; the cue needs
/// only the fact).
pub(crate) fn set(conn: &Connection) -> Result<(), WalletError> {
    conn.execute(
        "INSERT INTO wallet_rescan_rebuilding (singleton) VALUES (1)
         ON CONFLICT(singleton) DO NOTHING",
        [],
    )
    .map_err(map_aux_err)?;
    Ok(())
}

/// Clear the breadcrumb — the rebuild is over (reached tip). Called at the
/// `record_synced` clean-pass seam, independent and fail-open beside the
/// `ever_synced` set (a fault here is retried by every later clean pass, and
/// the cue's `everSynced` priority masks a stale row meanwhile). Single
/// statement — crash-atomic.
pub(crate) fn clear(conn: &Connection) -> Result<(), WalletError> {
    conn.execute("DELETE FROM wallet_rescan_rebuilding", [])
        .map_err(map_aux_err)?;
    Ok(())
}

/// Read the breadcrumb — `true` iff a rescan rebuild swapped in and has not
/// reached tip since. INFALLIBLE by design (UX-only; absence falls back to the
/// generic catch-up copy): a read fault degrades to `false` with a logged
/// warning rather than failing the caller's `open`. No value is logged (§5.4).
pub(crate) fn read(conn: &Connection) -> bool {
    let row: Result<Option<i64>, rusqlite::Error> = conn
        .query_row(
            "SELECT 1 FROM wallet_rescan_rebuilding WHERE singleton = 1",
            [],
            |r| r.get(0),
        )
        .optional();
    match row {
        Ok(v) => v.is_some(),
        Err(_) => {
            tracing::warn!(
                target: "zec_wallet_core",
                "rescan-rebuilding breadcrumb unreadable; falling back to generic catch-up copy (UX-safe)"
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
    fn set_is_idempotent_and_clear_resets() {
        let c = conn();
        set(&c).expect("set");
        assert!(read(&c));
        set(&c).expect("set again (rescan over a still-rebuilding wallet)");
        assert!(read(&c));
        let rows: i64 = c
            .query_row("SELECT COUNT(*) FROM wallet_rescan_rebuilding", [], |r| {
                r.get(0)
            })
            .expect("count");
        assert_eq!(rows, 1);
        clear(&c).expect("clear (reached tip)");
        assert!(!read(&c));
        clear(&c).expect("clear again is a no-op");
        assert!(!read(&c));
    }

    #[test]
    fn ensure_table_is_idempotent_and_never_clobbers_the_row() {
        let c = conn();
        set(&c).expect("set");
        ensure_table(&c).expect("re-ensure");
        assert!(read(&c));
    }

    #[test]
    fn a_missing_table_reads_false_never_an_error() {
        // The infallible read: a store missing the table (a pre-breadcrumb
        // wallet mid-rebuild across the upgrade) degrades to `false` — the
        // generic catch-up copy it showed before — never a panic or a bricked
        // open.
        let c = Connection::open_in_memory().expect("in-memory db");
        assert!(!read(&c));
    }
}
