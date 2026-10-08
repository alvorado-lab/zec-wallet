//! The persisted last-synced stamp (§2.5 `WalletState.last_synced`, #317).
//!
//! One `(height, at_unix_secs)` row recording the tip the LAST completed sync
//! pass reached and when it completed — the cross-launch "Balance (as of block
//! H, time)" source. There is NO cold timestamp in the audited engine
//! (`BlockMetadata` carries no block time; compact blocks are deleted after
//! scanning), so the SDK records its own stamp on the controller's clean-pass
//! path ([`crate::sync_controller`] → `SyncEnginePort::record_synced`) — and,
//! since T0-1c-R2, only for a pass whose tip the grade found at or above what
//! this wallet already holds: a behind server's pass (`SyncStatus::
//! EndpointBehind`) writes NO stamp, so this row never carries a height the
//! wallet knows is behind, and a relaunch renders the last CURRENT stamp or
//! none. The cost, priced at `emit_synced`: a behind pass that scanned leaves
//! the stamp staler than the balance until a current pass — under-claiming
//! freshness, the safe direction for a display-only row.
//!
//! **Display-only, NEVER a money input** (§2.5): a wrong/stale stamp can
//! mislabel a header, never move funds. Fail-open accordingly: the writer logs
//! and swallows its own faults (a stamp write must never fail a sync pass) and
//! readers degrade to `None` ("age unknown") — but a real DB fault still
//! surfaces typed from [`read`] so `snapshot()` keeps its fail-closed contract.
//!
//! **Storage.** Same discipline as [`crate::intent_store`]: pure SQL over the
//! aux SQLCipher connection (`Inner.aux_db`) to the ONE sealed/wiped/
//! backup-excluded `wallet.db`; the table is created idempotently in
//! `db::migrate` (the single provision+open chokepoint) and is listed in
//! `AUX_TABLES_PRESERVED` (ADR-0534). Single-statement writes take `RESERVED`
//! atomically, so the intent-store deadlock-freedom invariant holds.
//!
//! **Rescan resets the ROW (not the table).** A rescan rebuilds balances from
//! the birthday; a surviving stamp would pair the OLD height/time with a
//! rebuilding balance — precisely the dishonest claim `last_synced` exists to
//! avoid. The rescan rebuild ([`crate::store::reset_data_db_keep_seed`])
//! clears the row on the temp DB BEFORE the atomic rename, so the reset rides
//! the same crash-atomic replace (never a torn "new DB, old stamp" state).

use rusqlite::{Connection, OptionalExtension};

use crate::db::map_aux_err;
use crate::error::WalletError;
use crate::money::BlockHeight;
use crate::state::SyncStamp;

/// The single-row table name — referenced by `db::AUX_TABLES_PRESERVED`
/// (ADR-0534) and the rescan row-clear.
pub(crate) const TABLE: &str = "last_synced_stamp";

/// Create the stamp table (idempotent; runs in `db::migrate` on every
/// provision AND open, so existing wallets gain it with no schema bump).
pub(crate) fn ensure_table(conn: &Connection) -> Result<(), WalletError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS last_synced_stamp (
             singleton    INTEGER PRIMARY KEY CHECK (singleton = 1),
             height       INTEGER NOT NULL,
             at_unix_secs INTEGER NOT NULL
         );",
    )
    .map_err(map_aux_err)
}

/// Upsert THE stamp (single row, single statement — crash-atomic, takes
/// `RESERVED` directly per the aux-write invariant). `at_unix_secs` is
/// display-only wall-clock time supplied by the caller (the engine adapter
/// reads the system clock; tests pass fixed values).
pub(crate) fn record(
    conn: &Connection,
    height: BlockHeight,
    at_unix_secs: u64,
) -> Result<(), WalletError> {
    // Wall-clock seconds fit i64 until year ~292e9; the saturating guard keeps
    // a hostile/broken clock from panicking the cast.
    let at = i64::try_from(at_unix_secs).unwrap_or(i64::MAX);
    conn.execute(
        "INSERT INTO last_synced_stamp (singleton, height, at_unix_secs) VALUES (1, ?1, ?2)
         ON CONFLICT(singleton) DO UPDATE
             SET height = excluded.height, at_unix_secs = excluded.at_unix_secs",
        rusqlite::params![i64::from(height.value()), at],
    )
    .map_err(map_aux_err)?;
    Ok(())
}

/// Read THE stamp; `None` before the first completed pass (or after a rescan
/// reset). A negative/oversized stored value reads as `None` rather than a
/// fabricated stamp (display-only fail-open; the row is ours alone, so this
/// arm is defensive, not expected).
pub(crate) fn read(conn: &Connection) -> Result<Option<SyncStamp>, WalletError> {
    let row: Option<(i64, i64)> = conn
        .query_row(
            "SELECT height, at_unix_secs FROM last_synced_stamp WHERE singleton = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(map_aux_err)?;
    Ok(row.and_then(|(h, at)| {
        let height = u32::try_from(h).ok()?;
        let at = u64::try_from(at).ok()?;
        Some(SyncStamp {
            height: BlockHeight::new(height),
            at,
        })
    }))
}

/// Delete THE stamp row — the rescan reset (see the module doc for why a
/// rescan must not carry the old stamp across).
pub(crate) fn clear(conn: &Connection) -> Result<(), WalletError> {
    conn.execute("DELETE FROM last_synced_stamp", [])
        .map_err(map_aux_err)?;
    Ok(())
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
    fn read_is_none_before_the_first_record() {
        let c = conn();
        assert_eq!(read(&c).expect("read"), None);
    }

    #[test]
    fn record_then_read_round_trips_and_overwrites_the_single_row() {
        let c = conn();
        record(&c, BlockHeight::new(3_400_000), 1_751_700_000).expect("record");
        assert_eq!(
            read(&c).expect("read"),
            Some(SyncStamp {
                height: BlockHeight::new(3_400_000),
                at: 1_751_700_000,
            })
        );
        // A newer pass REPLACES the row (upsert, never a second row).
        record(&c, BlockHeight::new(3_400_100), 1_751_700_600).expect("record 2");
        assert_eq!(
            read(&c).expect("read 2"),
            Some(SyncStamp {
                height: BlockHeight::new(3_400_100),
                at: 1_751_700_600,
            })
        );
        let rows: i64 = c
            .query_row("SELECT COUNT(*) FROM last_synced_stamp", [], |r| r.get(0))
            .expect("count");
        assert_eq!(rows, 1);
    }

    #[test]
    fn clear_resets_to_none_and_a_second_clear_is_a_no_op() {
        let c = conn();
        record(&c, BlockHeight::new(42), 7).expect("record");
        clear(&c).expect("clear");
        assert_eq!(read(&c).expect("read"), None);
        clear(&c).expect("clear again");
        assert_eq!(read(&c).expect("read"), None);
    }

    #[test]
    fn a_corrupted_stored_value_reads_as_none_not_a_fabricated_stamp() {
        let c = conn();
        // Bypass `record` to plant an out-of-range height (display-only
        // fail-open: degrade to "age unknown", never invent a stamp).
        c.execute(
            "INSERT INTO last_synced_stamp (singleton, height, at_unix_secs) VALUES (1, ?1, ?2)",
            rusqlite::params![i64::from(u32::MAX) + 1, -5_i64],
        )
        .expect("plant");
        assert_eq!(read(&c).expect("read"), None);
    }

    #[test]
    fn a_hostile_clock_saturates_instead_of_panicking() {
        let c = conn();
        record(&c, BlockHeight::new(1), u64::MAX).expect("record");
        // i64::MAX at_unix_secs round-trips (u64::MAX saturated on write).
        assert_eq!(
            read(&c).expect("read"),
            Some(SyncStamp {
                height: BlockHeight::new(1),
                at: i64::MAX as u64,
            })
        );
    }
}
