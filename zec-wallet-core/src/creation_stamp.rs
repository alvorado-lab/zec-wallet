//! The persisted wallet-creation stamp (§3.2f, #356-F4 — the offline-create
//! birthday floor).
//!
//! One `at_unix_secs` row recording WHEN this wallet's seed was freshly
//! generated (`SeedSource::Generate`, or host-ATTESTED fresh via
//! `SeedSource::FreshRawBytes` — FR-24) on this device. Provisioning is LAZY
//! (first sync), so without this stamp an offline-created wallet's birthday
//! resolved at `tip − lag` *of the first-sync day* — a deposit received
//! between create and first sync sat below the birthday (0 balance, "Up to
//! date": the silent-fund-loss shape). The stamp lets `resolve_birthday`
//! bound the fresh-wallet default at `min(tip − lag,
//! estimate_birthday(created_at))` no matter when the first sync runs.
//!
//! **A MONEY input, but only ever DOWNWARD** (§3.2f): the stamp can only
//! LOWER the birthday below the pre-F4 defaults (`tip − lag` for fresh,
//! activation for the stamp-less reopen), so a wrong/garbage/missing stamp
//! degrades to a WIDER scan — never a higher (fund-losing) birthday. That is
//! why write-side faults fail provisioning loudly (create is user-attended)
//! while the READ is infallible — any read fault degrades to `None` (the
//! pre-F4 arms) rather than failing an `open` over a value whose absence is
//! money-safe.
//!
//! **Write-once, fresh-provision-only, earliest wins.** Written ONLY by a
//! freshly-generated create's FRESH provision (`SeedSource::Generate` or the
//! host-attested `FreshRawBytes`, via `store::provision_with` →
//! `db::provision_db`); `ON CONFLICT DO NOTHING` keeps the EARLIEST stamp —
//! the conservative one — across idempotent re-runs. NEVER written on
//! restore (`Mnemonic`/plain-`RawBytes` history may predate any stamp), NEVER
//! written by `store::repair` (a remnant's sealed seed may originate from an
//! aborted restore — the review HIGH; see the repair-site comment), and
//! NEVER updated. The ONE deletion is the DOWNGRADE UN-STAMP
//! ([`clear`], called only by a restore-shaped `store::repair` completion —
//! see its doc). Its PRESENCE therefore proves a seed was minted at ~that
//! wall-clock — on this device for Generate, or per the HOST's `FreshRawBytes`
//! attestation (the §3.2f lying-host contract carries that trust shift; a
//! mis-attested restore floors its birthday here, hiding pre-stamp funds) —
//! which is what lets an account-less REOPEN (where `freshly_generated` is
//! unknowable) still take the bounded-from-creation floor instead of the
//! activation full scan.
//! CAVEAT (`SeedPersistence::None`): the stamp binds to the STORE, not the
//! seed — nothing at rest can verify the seed a host port later supplies is
//! the one generated here. The port contract (§4.2: supply THIS wallet's
//! seed) already carries the wallet's money-safety generally; the stamp adds
//! no verification of its own. Seed-fingerprint binding is #357-scope.
//!
//! **The write-time clock-ahead cap is enforced HERE** ([`record_once`]
//! clamps to `checkpoints::newest_checkpoint_time(network)`), not at call
//! sites — structurally unbypassable for any future caller. Clamping to the
//! CREATING binary's bundle tail means no later binary's fresher bundle can
//! estimate the stamp above a height this binary already knew about (≤ the
//! tip at create ≤ any deposit's height) — the spec's argument (b).
//!
//! **Privacy:** the stamp is exact-second — finer than the checkpoint-granular
//! birthday that eventually becomes chain-inferable. It never leaves the
//! SQLCipher-encrypted `wallet.db` (no FFI surface, no tracing of the value,
//! not in the plaintext manifest); the only party who can read it already
//! holds the DB key and, with it, the full tx history.
//!
//! **Storage.** Same discipline as [`crate::sync_stamp`]: pure SQL over the
//! aux SQLCipher connection to the ONE sealed/wiped/backup-excluded
//! `wallet.db`. The table is created idempotently in `db::migrate` (the aux
//! SET's single declaration, `ensure_aux_tables`) — plus one direct
//! `ensure_table` inside `db::provision_db`, which must write the stamp
//! BEFORE `migrate` consumes the connection. Single-statement writes take
//! `RESERVED` atomically, so the intent-store deadlock-freedom invariant
//! holds.
//!
//! **Rescan PRESERVES the row** — the opposite of [`crate::sync_stamp`]'s
//! reset: when the seed came into existence is a fact a rescan cannot change,
//! and losing it would silently re-widen a future provisioning to the
//! activation floor (safe but a multi-hour regression). The row rides
//! `db::copy_aux_tables` unmodified (listed in `AUX_TABLES_PRESERVED`,
//! ADR-0534).

use rusqlite::{Connection, OptionalExtension};

use crate::checkpoints;
use crate::db::map_aux_err;
use crate::error::WalletError;
use crate::money::Network;

/// The single-row table name — referenced by `db::AUX_TABLES_PRESERVED`
/// (ADR-0534).
pub(crate) const TABLE: &str = "wallet_created_at";

/// Create the stamp table (idempotent; runs in `db::migrate` on every
/// provision AND open, so existing wallets gain it with no schema bump — a
/// pre-F4 wallet simply has no row, which reads `None` = the pre-F4 arms).
pub(crate) fn ensure_table(conn: &Connection) -> Result<(), WalletError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS wallet_created_at (
             singleton    INTEGER PRIMARY KEY CHECK (singleton = 1),
             at_unix_secs INTEGER NOT NULL
         );",
    )
    .map_err(map_aux_err)
}

/// Record the creation stamp ONCE (single row, single statement —
/// crash-atomic, takes `RESERVED` directly per the aux-write invariant).
/// `ON CONFLICT DO NOTHING`: an idempotent create re-run keeps the EARLIEST
/// (most conservative) value. `at_unix_secs` is the caller's raw wall clock;
/// the write-time clock-ahead cap — clamping to `network`'s bundled-checkpoint
/// tail time — is applied HERE (the module-doc invariant), so no caller can
/// store a time the creating binary's own bundle can't vouch for. A pre-epoch/
/// behind clock stores low ⇒ a wider scan (safe); the residual i64 saturation
/// is belt (the clamp already bounds well below it).
pub(crate) fn record_once(
    conn: &Connection,
    network: Network,
    at_unix_secs: u64,
) -> Result<(), WalletError> {
    let clamped = at_unix_secs.min(checkpoints::newest_checkpoint_time(network));
    let at = i64::try_from(clamped).unwrap_or(i64::MAX);
    conn.execute(
        "INSERT INTO wallet_created_at (singleton, at_unix_secs) VALUES (1, ?1)
         ON CONFLICT(singleton) DO NOTHING",
        rusqlite::params![at],
    )
    .map_err(map_aux_err)?;
    Ok(())
}

/// Clear the stamp — the DOWNGRADE UN-STAMP (crypto audit MED-1), called
/// ONLY by `store::repair` when completing a remnant under a RESTORE-shaped
/// intent (`freshly_generated_at: None`) while a stamp from an aborted FRESH
/// attempt is durable. Without this, the documented lying-host recovery
/// ("when in doubt, retry the create with `freshlyGenerated: false`") would
/// silently inherit the aborted attempt's stamp: the wallet the host believes
/// is restore-shaped scans from ~stamp instead of the activation floor AND
/// skips the restore-pessimistic refund sweep — invisible pre-stamp money.
/// Clearing is strictly the CONSERVATIVE direction (a wider scan + the
/// restore sweep; never a higher birthday). Single statement — crash-atomic;
/// a kill before the marker leaves a stampless remnant, which every retry
/// shape handles (fresh ⇒ the stamp-less belt arms; restore ⇒ this no-ops).
pub(crate) fn clear(conn: &Connection) -> Result<(), WalletError> {
    conn.execute("DELETE FROM wallet_created_at", [])
        .map_err(map_aux_err)?;
    Ok(())
}

/// Read the stamp; `None` for a restore-created or pre-F4 wallet (⇒ the
/// pre-F4 resolution arms). INFALLIBLE by design: the stamp is a money input
/// whose ABSENCE is the safe direction (a wider scan, never a higher
/// birthday), so a read fault — value garbage, a wrong-typed cell, an SQL
/// error — degrades to `None` with a logged warning instead of failing the
/// caller's `open` (a wallet that cannot open cannot sync OR rescan; bricking
/// it over an optional downward bound would invert the posture). No value is
/// ever logged (§5.4).
pub(crate) fn read(conn: &Connection) -> Option<u64> {
    let row: Result<Option<i64>, rusqlite::Error> = conn
        .query_row(
            "SELECT at_unix_secs FROM wallet_created_at WHERE singleton = 1",
            [],
            |r| r.get(0),
        )
        .optional();
    match row {
        Ok(v) => v.and_then(|at| u64::try_from(at).ok()),
        Err(_) => {
            tracing::warn!(
                target: "zec_wallet_core",
                "creation stamp unreadable; treating as absent (wider scan, money-safe)"
            );
            None
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
    fn read_is_none_before_any_record() {
        let c = conn();
        assert_eq!(read(&c), None);
    }

    #[test]
    fn record_once_round_trips_and_the_earliest_stamp_wins() {
        let c = conn();
        record_once(&c, Network::Main, 1_751_000_000).expect("record");
        assert_eq!(read(&c), Some(1_751_000_000));
        // An idempotent re-run stamping LATER must NOT move the stamp forward —
        // the earliest (widest-scan, most conservative) value is kept.
        record_once(&c, Network::Main, 1_751_999_999).expect("record again");
        assert_eq!(read(&c), Some(1_751_000_000));
        let rows: i64 = c
            .query_row("SELECT COUNT(*) FROM wallet_created_at", [], |r| r.get(0))
            .expect("count");
        assert_eq!(rows, 1);
    }

    #[test]
    fn ensure_table_is_idempotent_and_never_clobbers_the_row() {
        let c = conn();
        record_once(&c, Network::Main, 42).expect("record");
        ensure_table(&c).expect("re-ensure");
        assert_eq!(read(&c), Some(42));
    }

    #[test]
    fn the_clock_ahead_cap_is_enforced_at_write_for_both_networks() {
        // The module-doc invariant, now structural: NO caller can store a time
        // above the creating binary's bundle-tail time — a hostile/broken
        // clock (u64::MAX) stores exactly the cap.
        for net in [Network::Main, Network::Test] {
            let c = conn();
            record_once(&c, net, u64::MAX).expect("record");
            assert_eq!(
                read(&c),
                Some(checkpoints::newest_checkpoint_time(net)),
                "{net:?}: the stored stamp is clamped to the bundle tail"
            );
        }
    }

    #[test]
    fn a_corrupted_negative_value_reads_as_none_not_a_fabricated_stamp() {
        let c = conn();
        // Bypass `record_once` to plant garbage: degrade to None (the pre-F4
        // arms — a WIDER scan), never a fabricated stamp.
        c.execute(
            "INSERT INTO wallet_created_at (singleton, at_unix_secs) VALUES (1, -5)",
            [],
        )
        .expect("plant");
        assert_eq!(read(&c), None);
    }

    #[test]
    fn a_wrong_typed_cell_reads_as_none_never_an_error() {
        // SQLite dynamic typing admits a TEXT cell despite INTEGER NOT NULL;
        // pre-fold this surfaced as StoreCorrupt and BRICKED open (
        // security M2). The infallible read degrades it to the pre-F4 arms.
        let c = conn();
        c.execute(
            "INSERT INTO wallet_created_at (singleton, at_unix_secs) VALUES (1, 'garbage')",
            [],
        )
        .expect("plant");
        assert_eq!(read(&c), None);
    }
}
