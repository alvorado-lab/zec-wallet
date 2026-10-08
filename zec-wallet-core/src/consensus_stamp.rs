//! The persisted consensus verdict (`ironwood-nu63-support.md` §2 / §6.2).
//!
//! One row recording the last [`ConsensusCompatibility`] this wallet evaluated,
//! when it did, and — for the §6.3 grace — the two anchors of the last verdict
//! that PERMITTED signing: the height it was judged at, bounded, and the
//! device-clock instant it was taken at. Since T0-1c-R2 (M4) the caller
//! (`Wallet::evaluate_consensus`, `consensus::grace_anchor`) clamps that height
//! into `[scanned, scanned + REORG_MAX_BLOCKS]` before it reaches [`record`]:
//! no endpoint height can put the anchor below what this wallet scanned, nor
//! more than a reorg margin above it. This module stores what it is handed.
//!
//! **The CARRIED time and the latch (GRACE-1, §4p).** `capable_tip` was always
//! carried forward across non-capable verdicts; `capable_at_unix_secs` now
//! rides with it — the instant of the last CAPABLE verdict, which is not the
//! same column as `at_unix_secs` (the latest verdict of any kind; §4p P-G1: a
//! silent server's passes kept refreshing that one, so "seconds since the last
//! capable verdict" did not exist in the store). The clock rule is measured
//! from it on EVERY read through [`observe`], which takes `now` from the
//! `WallClock` port: a send a day after the last pass sees the day even though
//! no pass ran (G-13). `clock_expired` is the G-2b latch — set by `observe` the
//! moment any reader sees a day elapsed, carried across non-capable verdicts,
//! cleared ONLY by a capable one — so a clock set back after the expiry was
//! observed cannot re-permit signing; only a server that reports its branch
//! can. The latch is durable state because the readers are (a relaunch reads
//! the same row); an in-memory high-water would re-permit after a restart.
//! Why a latch and not a monotone clock reading: a monotone reading would have
//! to be persisted on every read too, and after a clock jumped FORWARD and was
//! corrected it would hold the clock rule silent for the whole jump; the latch
//! stores one bit with one meaning ("the expiry has been observed since the
//! last capable verdict").
//!
//! **A capable time the clock cannot vouch for EXPIRES the clock rule
//! (GRACE-2, §4v — maintainer decision 2026-09-10, item 1).** A recorded capable
//! time LATER than the device clock (the clock moved back after the capable
//! pass, or was ahead during it), or a capable time of `0` (a pre-epoch clock
//! at the capable pass — `SystemClock::now_unix` reads `0` before the epoch),
//! is not a time the rule can measure from, and GRACE-1 let it ABSTAIN:
//! `elapsed` read `None`, nothing latched, and the block rule decided alone.
//! The block rule is exactly what a frozen-tip silent server defeats
//! (§4p-run review row 2), so one capable pass taken under a wrong clock left
//! the grace with no expiry on EITHER axis until real time passed the stamp —
//! and only a capable pass repairs it, which is what the adversary withholds.
//! Now [`observe`] writes the SAME latch on such a reading, fail-closed: the
//! clock rule reads expired until a capable verdict, which re-records the time
//! from the clock as it reads then. Both saturations on this path point the
//! same way ([`record`]'s `i64::MAX` is later than any real `now`, so it
//! expires too). The rule as one sentence: a clock can only SHORTEN the
//! grace; a time the wallet cannot trust ENDS it.
//!
//! **Why persist a "derived" thing at all.** The verdict itself is derived and
//! is never synced between devices (§10). But §6.2 owes the host "the last
//! verdict and its age" while offline, and a purely in-memory verdict is absent
//! on every process start — so a cold start would have to either fabricate
//! `Current` (the §0 failure: signing transactions we cannot know are valid) or
//! refuse everything including composing. One small local row buys the honest
//! third answer.
//!
//! **Rescan does NOT clear this row** (contrast [`crate::sync_stamp`]). A
//! rescan rebuilds OUR view of the chain; it says nothing about which consensus
//! rules the network is running. Clearing it would drop the grace anchor and
//! silently re-refuse a wallet whose only sin was rebuilding its balance.
//!
//! **Storage.** Same discipline as [`crate::sync_stamp`]: pure SQL over the aux
//! SQLCipher connection, table created idempotently in `db::migrate`, listed in
//! `AUX_TABLES_PRESERVED` (ADR-0534), single-statement writes. **Format bump
//! (GRACE-1): drop-and-GC, no migration** — the dev-stage rule. A row of the
//! pre-GRACE-1 shape (no `capable_at_unix_secs` column) is dropped by
//! [`ensure_table`] and the table recreated; the wallet then reads "no verdict"
//! until its next pass evaluates one — refusing to sign for that one pass, the
//! fail-closed direction, and the same state as a fresh install.

use rusqlite::{Connection, OptionalExtension};

use crate::consensus::{ConsensusCompatibility, ConsensusStatusSnapshot, GraceClock};
use crate::db::map_aux_err;
use crate::error::WalletError;
use crate::money::BlockHeight;

/// The single-row table name — referenced by `db::AUX_TABLES_PRESERVED`.
pub(crate) const TABLE: &str = "consensus_verdict";

/// The column whose presence marks the GRACE-1 shape — the drop-and-GC probe.
const SHAPE_MARKER_COLUMN: &str = "capable_at_unix_secs";

/// Discriminants as stored. Stable strings, not enum ordinals: a reordered
/// enum must never silently re-interpret an existing row as a different
/// verdict (and the difference between `unsupported` and `current` is whether
/// we sign).
const KIND_CURRENT: &str = "current";
const KIND_BEHIND: &str = "behind";
const KIND_UNSUPPORTED: &str = "unsupported";
const KIND_UNKNOWN: &str = "unknown";

pub(crate) fn ensure_table(conn: &Connection) -> Result<(), WalletError> {
    // Drop-and-GC (module doc): a table of the pre-GRACE-1 shape is dropped
    // whole; the CREATE below then makes the current one. Probed by column
    // name, never by a version number the old shape did not carry.
    if table_exists(conn)? && !has_column(conn, SHAPE_MARKER_COLUMN)? {
        conn.execute_batch("DROP TABLE consensus_verdict;")
            .map_err(map_aux_err)?;
    }
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS consensus_verdict (
             singleton            INTEGER PRIMARY KEY CHECK (singleton = 1),
             kind                 TEXT    NOT NULL,
             expected_branch      INTEGER,
             endpoint_branch      INTEGER,
             judged_height        INTEGER,
             scanned_tip          INTEGER,
             claimed_tip          INTEGER,
             capable_tip          INTEGER,
             capable_at_unix_secs INTEGER,
             clock_expired        INTEGER NOT NULL DEFAULT 0,
             at_unix_secs         INTEGER NOT NULL
         );",
    )
    .map_err(map_aux_err)
}

fn table_exists(conn: &Connection) -> Result<bool, WalletError> {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [TABLE],
        |_| Ok(()),
    )
    .optional()
    .map(|found| found.is_some())
    .map_err(map_aux_err)
}

/// Is `column` on the table? (`PRAGMA table_info`, whose row column 1 is the
/// name — the `intent_store` idiom.) Surfaces any query error.
fn has_column(conn: &Connection, column: &str) -> Result<bool, WalletError> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(consensus_verdict)")
        .map_err(map_aux_err)?;
    let names = stmt
        .query_map([], |r| r.get::<_, String>(1))
        .map_err(map_aux_err)?;
    for name in names {
        if name.map_err(map_aux_err)? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

/// What the row carries forward across non-capable verdicts: the two grace
/// anchors and the latch. Read raw (no clock) by [`record`].
#[derive(Clone, Copy, Default)]
struct Carried {
    capable_tip: Option<BlockHeight>,
    capable_at_unix_secs: Option<u64>,
    clock_expired: bool,
}

/// Upsert THE verdict.
///
/// `capable_tip` and `capable_at_unix_secs` are carried forward, not
/// recomputed: an `Unknown` riding its grace must never renew the anchors it
/// rides, or a silent endpoint would extend its own grace forever (§6.3) — on
/// EITHER axis (§4p G-10's "the time not carried forward" mutant). On a capable
/// verdict both are REPLACED — the tip by `claimed_tip` as handed in, which,
/// since T0-1c-R2 (M4), is the bounded judged height `consensus::grace_anchor`
/// computes, never the endpoint's raw claim (a capable pass can move the anchor
/// within `[scanned, scanned + REORG_MAX_BLOCKS]` in either direction; a rewind
/// lowers `scanned`, and the next capable pass's anchor with it — fail-closed
/// by at most the rewind depth; a lying-low identity height cannot move the
/// anchor, and since §4p item 4 cannot move the grace's numerator either) and
/// the time by `at_unix_secs`, the port's reading for this pass — and the
/// G-2b latch is CLEARED: a capable verdict is the one thing that re-permits
/// after a clock expiry. `at_unix_secs` itself is written for every verdict
/// (the latest verdict's time, the host's "age" display).
pub(crate) fn record(
    conn: &Connection,
    verdict: &ConsensusCompatibility,
    claimed_tip: BlockHeight,
    at_unix_secs: u64,
) -> Result<(), WalletError> {
    let previous = read_row(conn)?.map_or(Carried::default(), |r| r.carried);
    let Carried {
        capable_tip,
        capable_at_unix_secs,
        clock_expired,
    } = if verdict.is_signing_capable_check() {
        Carried {
            capable_tip: Some(claimed_tip),
            capable_at_unix_secs: Some(at_unix_secs),
            clock_expired: false,
        }
    } else {
        previous
    };

    let (kind, expected, endpoint, judged, scanned, claimed) = match verdict {
        ConsensusCompatibility::Current => (KIND_CURRENT, None, None, None, None, None),
        ConsensusCompatibility::Behind {
            scanned_tip,
            claimed_tip,
        } => (
            KIND_BEHIND,
            None,
            None,
            None,
            Some(scanned_tip.value()),
            Some(claimed_tip.value()),
        ),
        ConsensusCompatibility::Unsupported {
            expected_branch_id,
            endpoint_branch_id,
            judged_at_height,
        } => (
            KIND_UNSUPPORTED,
            Some(*expected_branch_id),
            *endpoint_branch_id,
            Some(judged_at_height.value()),
            None,
            None,
        ),
        ConsensusCompatibility::Unknown {
            judged_at_height, ..
        } => (
            KIND_UNKNOWN,
            None,
            None,
            Some(judged_at_height.value()),
            None,
            None,
        ),
    };

    // Wall-clock seconds fit i64 until year ~292e9; the saturating guard keeps
    // a hostile/broken clock from panicking the cast (the sync_stamp precedent).
    // The DIRECTION of the capable time's saturation matters (§4v G2-6, review
    // row 8): `i64::MAX` reads back as a capable time later than any real
    // `now`, which `observe` treats as an EXPIRED clock rule (§4v, fail-closed)
    // — so a clock past the year 292e9 buys no grace; the round trip is pinned
    // by `a_saturated_capable_time_round_trips_as_expired_by_the_clock`.
    let at = i64::try_from(at_unix_secs).unwrap_or(i64::MAX);
    let capable_at = capable_at_unix_secs.map(|s| i64::try_from(s).unwrap_or(i64::MAX));
    conn.execute(
        "INSERT INTO consensus_verdict
             (singleton, kind, expected_branch, endpoint_branch, judged_height,
              scanned_tip, claimed_tip, capable_tip, capable_at_unix_secs,
              clock_expired, at_unix_secs)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(singleton) DO UPDATE SET
             kind = excluded.kind,
             expected_branch = excluded.expected_branch,
             endpoint_branch = excluded.endpoint_branch,
             judged_height = excluded.judged_height,
             scanned_tip = excluded.scanned_tip,
             claimed_tip = excluded.claimed_tip,
             capable_tip = excluded.capable_tip,
             capable_at_unix_secs = excluded.capable_at_unix_secs,
             clock_expired = excluded.clock_expired,
             at_unix_secs = excluded.at_unix_secs",
        rusqlite::params![
            kind,
            expected.map(i64::from),
            endpoint.map(i64::from),
            judged.map(i64::from),
            scanned.map(i64::from),
            claimed.map(i64::from),
            capable_tip.map(|t| i64::from(t.value())),
            capable_at,
            i64::from(clock_expired),
            at,
        ],
    )
    .map_err(map_aux_err)?;
    Ok(())
}

/// The stored row, exactly as SQLite hands it back: `(kind, expected_branch,
/// endpoint_branch, judged_height, scanned_tip, claimed_tip, capable_tip,
/// capable_at_unix_secs, clock_expired, at_unix_secs)`. Named so the query
/// below reads as one thing rather than a ten-slot tuple.
type StoredRow = (
    String,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    i64,
    i64,
);

/// The row decoded but NOT yet read against a clock: the verdict as stored
/// (an `Unknown` whose clock half is still [`GraceClock::NONE`] — unfilled, not
/// a reading), plus what [`record`] carries
/// forward. Private on purpose — every consumer goes through [`observe`], so
/// no reader can take the verdict without the clock rule (§4p G-13; G-10's
/// "consulted only at evaluate" mutant is a caller of this instead of that).
struct RawRow {
    verdict: Option<ConsensusCompatibility>,
    carried: Carried,
    at_unix_secs: Option<u64>,
}

/// Decode THE row; `None` before this wallet has ever evaluated a verdict.
///
/// A row we cannot interpret (unknown `kind`, out-of-range integer) reads as
/// **no verdict**, never as a fabricated `Current` — the fail-closed direction,
/// because the consequence of a wrong `Current` is signing a transaction the
/// network will reject.
fn read_row(conn: &Connection) -> Result<Option<RawRow>, WalletError> {
    let row: Option<StoredRow> = conn
        .query_row(
            "SELECT kind, expected_branch, endpoint_branch, judged_height,
                    scanned_tip, claimed_tip, capable_tip, capable_at_unix_secs,
                    clock_expired, at_unix_secs
               FROM consensus_verdict WHERE singleton = 1",
            [],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                    r.get(8)?,
                    r.get(9)?,
                ))
            },
        )
        .optional()
        .map_err(map_aux_err)?;

    let Some((
        kind,
        expected,
        endpoint,
        judged,
        scanned,
        claimed,
        capable,
        capable_at,
        clock_expired,
        at,
    )) = row
    else {
        return Ok(None);
    };

    let height = |v: Option<i64>| -> Option<BlockHeight> {
        v.and_then(|h| u32::try_from(h).ok()).map(BlockHeight::new)
    };
    let branch = |v: Option<i64>| -> Option<u32> { v.and_then(|b| u32::try_from(b).ok()) };

    let verdict = match kind.as_str() {
        KIND_CURRENT => Some(ConsensusCompatibility::Current),
        KIND_BEHIND => match (height(scanned), height(claimed)) {
            (Some(scanned_tip), Some(claimed_tip)) => Some(ConsensusCompatibility::Behind {
                scanned_tip,
                claimed_tip,
            }),
            _ => None,
        },
        KIND_UNSUPPORTED => match (branch(expected), height(judged)) {
            (Some(expected_branch_id), Some(judged_at_height)) => {
                Some(ConsensusCompatibility::Unsupported {
                    expected_branch_id,
                    endpoint_branch_id: branch(endpoint),
                    judged_at_height,
                })
            }
            _ => None,
        },
        // The grace distance is REBUILT here, from the two heights the row
        // already carries. It must not be left `None`: `None` means "there has
        // never been a signing-capable verdict", which `permits_signing`
        // treats as outside the grace by construction — so leaving it unset
        // turned the maintainer's "degrade with age" decision into the blanket
        // refusal that decision explicitly rejected, for every honest server
        // that omits the field (security review). An earlier comment here
        // claimed the caller recomputed it live; no caller did.
        //
        // Measured at `judged_height` (the tip when the verdict was taken)
        // rather than a live tip, because a cold read may be offline and the
        // honest answer then is "as of the last evaluation". The CLOCK half is
        // the opposite: it is filled by `observe` against NOW, because a day
        // passes whether or not a pass ran — that is the whole point of it.
        KIND_UNKNOWN => height(judged).map(|judged_at_height| ConsensusCompatibility::Unknown {
            judged_at_height,
            blocks_since_last_current: height(capable)
                .map(|anchor| judged_at_height.value().saturating_sub(anchor.value())),
            clock: GraceClock::NONE,
        }),
        _ => None,
    };

    Ok(Some(RawRow {
        verdict,
        carried: Carried {
            capable_tip: height(capable),
            capable_at_unix_secs: capable_at.and_then(|s| u64::try_from(s).ok()),
            clock_expired: clock_expired != 0,
        },
        at_unix_secs: u64::try_from(at).ok(),
    }))
}

/// The G-2b latch, written — ONE statement for both reasons [`observe`] writes
/// it (a day elapsed on the device clock; a capable time the clock cannot vouch
/// for, §4v), so the two cannot drift into two bits.
fn write_latch(conn: &Connection) -> Result<(), WalletError> {
    conn.execute(
        "UPDATE consensus_verdict SET clock_expired = 1 WHERE singleton = 1",
        [],
    )
    .map_err(map_aux_err)?;
    Ok(())
}

/// Read THE verdict AS OF `now_unix` (the `WallClock` port's reading) — the ONE
/// reader every consumer of the stamp uses (GRACE-1, §4p G-13): the send gate
/// and the drain (`signing_permit`), the parked-row flag, the sync surface's
/// reader, the cold read (`consensus_status`) and the pass itself
/// (`evaluate_consensus`, for the previous row). `None` before this wallet has
/// ever evaluated a verdict.
///
/// Fills the clock half of the grace from the carried capable time:
/// `elapsed = now − capable_at` — and WRITES the latch (one `UPDATE` under
/// the aux lock the reader already holds) the moment any reader, with the
/// latch clear, sees EITHER of two things:
///
/// - `elapsed ≥ UNKNOWN_BRANCH_GRACE_SECS` (`GraceClock::past_threshold` — the
///   ONE comparison, the same one `GraceClock::expired` reads): a day passed.
/// - a capable time the clock cannot vouch for — LATER than `now`, or `0`
///   (GRACE-2, §4v; the module doc says why): `elapsed` reads `None` and the
///   rule is EXPIRED, fail-closed, with its own duration-free warn line.
///
/// From then on the clock rule is expired whatever the clock reads, until a
/// capable verdict clears it ([`record`]). A read that writes, on purpose —
/// the alternative (latch only at the pass) re-permitted a send the moment the
/// device clock was set back with no pass in between, which is the case G-2b
/// names. `elapsed_secs: None` with the latch CLEAR now means only "no capable
/// time recorded" (`GraceClock::NONE`) — and with no capable verdict the block
/// half refuses on its own (`NeverConfirmed`).
pub(crate) fn observe(
    conn: &Connection,
    now_unix: u64,
) -> Result<Option<ConsensusStatusSnapshot>, WalletError> {
    let Some(RawRow {
        verdict,
        carried,
        at_unix_secs,
    }) = read_row(conn)?
    else {
        return Ok(None);
    };
    // §4v (GRACE-2): the cell GRACE-1 let abstain. A capable time later than
    // the device clock, or a capable time of 0, is EXPIRED — the base's filter
    // (`capable_at <= now` → elapsed, else `None` and nothing latched) handed
    // a frozen-tip silent server a grace with no expiry on either axis.
    let untrusted = carried
        .capable_at_unix_secs
        .is_some_and(|capable_at| capable_at == 0 || capable_at > now_unix);
    let elapsed_secs = if untrusted {
        None
    } else {
        carried
            .capable_at_unix_secs
            .map(|capable_at| now_unix - capable_at)
    };
    let mut latched = carried.clock_expired;
    if !latched && untrusted {
        write_latch(conn)?;
        latched = true;
        // The same durable bit the day writes, and the same two-kind story
        // below: on an `Unknown` row the grace has ended; on a capable row the
        // wallet still signs and a silent pass recorded later inherits the
        // latch. One sentence is true of both kinds here — it is about the
        // time, not the grace — and it carries no duration and no absolute
        // time (§5.4; `outcome` is a static code, `tracing_guard::ALLOWLIST`).
        tracing::warn!(
            target: "wallet.consensus",
            outcome = "capable_time_untrusted",
            "the recorded capable time is later than the device clock, or pre-epoch: the clock rule is expired until a capable verdict"
        );
    }
    if !latched && GraceClock::past_threshold(elapsed_secs) {
        write_latch(conn)?;
        latched = true;
        // The latch is an observation of REAL TIME passing since the last
        // capable verdict, written whatever the row's verdict kind — two
        // cases, both wanted (§4p-run row 2; the second found by G-7b):
        //   - the row is `Unknown`: the grace has ENDED by the clock, and the
        //     send gate refuses from this read on.
        //   - the row is capable (`Current`/`Behind` — a pass long ago, no
        //     pass since): the wallet is on NO grace and still signs, but the
        //     day has passed, and a silent pass recorded later INHERITS the
        //     latch (`record` carries `clock_expired` across non-capable
        //     verdicts), so its grace starts already ended — "the clock only
        //     tightens".
        // The write is the same in both; only the sentence differs, and
        // "grace expired" would be false on a capable row. §5.4: a duration,
        // never the absolute time (`tracing_guard::ALLOWLIST`).
        let grace_secs = elapsed_secs.unwrap_or(0);
        if matches!(verdict, Some(ConsensusCompatibility::Unknown { .. })) {
            tracing::warn!(
                target: "wallet.consensus",
                grace_secs,
                "consensus grace expired by clock"
            );
        } else {
            tracing::debug!(
                target: "wallet.consensus",
                grace_secs,
                "a day passed since the last capable verdict, no grace running: clock latched"
            );
        }
    }
    let grace_clock = GraceClock {
        elapsed_secs,
        latched,
    };
    let verdict = verdict.map(|v| match v {
        ConsensusCompatibility::Unknown {
            judged_at_height,
            blocks_since_last_current,
            clock: _,
        } => ConsensusCompatibility::Unknown {
            judged_at_height,
            blocks_since_last_current,
            clock: grace_clock,
        },
        other => other,
    });
    Ok(Some(ConsensusStatusSnapshot {
        verdict,
        grace_anchor_tip: carried.capable_tip,
        evaluated_at_unix: at_unix_secs,
        grace_clock,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{UNKNOWN_BRANCH_GRACE_BLOCKS, UNKNOWN_BRANCH_GRACE_SECS};

    fn conn() -> Connection {
        let c = Connection::open_in_memory().expect("in-memory db");
        ensure_table(&c).expect("ensure");
        c
    }

    /// The instant the fixtures below record their capable verdicts at, and
    /// read back at unless a row is ABOUT the clock — so the clock half reads
    /// "no time passed" and the block-rule rows stay about blocks.
    const T0: u64 = 1_757_000_000;

    fn at_t0(secs: u64) -> GraceClock {
        GraceClock {
            elapsed_secs: Some(secs),
            latched: false,
        }
    }

    #[test]
    fn read_is_none_before_the_first_evaluation() {
        assert_eq!(observe(&conn(), T0).expect("read"), None);
    }

    #[test]
    fn every_verdict_kind_round_trips() {
        let c = conn();
        let cases = [
            ConsensusCompatibility::Current,
            ConsensusCompatibility::Behind {
                scanned_tip: BlockHeight::new(1_200_000),
                claimed_tip: BlockHeight::new(3_400_000),
            },
            ConsensusCompatibility::Unsupported {
                expected_branch_id: 0x5437_f330,
                endpoint_branch_id: Some(0x37a5_165b),
                judged_at_height: BlockHeight::new(3_473_803),
            },
            ConsensusCompatibility::Unknown {
                judged_at_height: BlockHeight::new(3_400_000),
                // `Current` runs FIRST in this list and sets the anchor at the
                // same claimed tip, so the rebuilt distance here is 0 — not
                // `None`. The rebuild is the point: leaving it unset is what
                // made the §6.3 grace inert in production.
                blocks_since_last_current: Some(0),
                // And the clock half is filled the same way, from the carried
                // capable time against the instant of the read (T0 − T0).
                clock: at_t0(0),
            },
        ];
        for verdict in cases {
            record(&c, &verdict, BlockHeight::new(3_400_000), T0).expect("record");
            let back = observe(&c, T0)
                .expect("read")
                .expect("a row")
                .verdict
                .expect("a verdict");
            assert_eq!(back, verdict, "{verdict:?} must round-trip");
        }
    }

    /// With NO prior signing-capable verdict there is no anchor, so an
    /// `Unknown` reads back with `None` — outside the grace by construction,
    /// the fail-closed direction. (The sibling above covers the anchored case.)
    #[test]
    fn an_unknown_with_no_anchor_reads_back_outside_the_grace() {
        let c = conn();
        record(
            &c,
            &ConsensusCompatibility::Unknown {
                judged_at_height: BlockHeight::new(3_400_000),
                blocks_since_last_current: None,
                clock: GraceClock::NONE,
            },
            BlockHeight::new(3_400_000),
            T0,
        )
        .expect("record");
        let back = observe(&c, T0)
            .expect("read")
            .expect("row")
            .verdict
            .expect("verdict");
        assert_eq!(
            back,
            ConsensusCompatibility::Unknown {
                judged_at_height: BlockHeight::new(3_400_000),
                blocks_since_last_current: None,
                // No capable time either: nothing to measure from (`NONE`).
                clock: GraceClock::NONE,
            }
        );
        assert!(!back.permits_signing(), "no anchor ⇒ no grace ⇒ no signing");
    }

    /// THE STORE ROUND TRIP for the §6.3 grace — the gap that made the maintainer's
    /// "degrade with age" decision inert (security review).
    ///
    /// `permits_signing` is exercised elsewhere against the PREDICATE, which is
    /// handed a live grace distance. Production does not: it reads the verdict
    /// back from THIS table, so if the distance is not rebuilt here, every
    /// `Unknown` refuses and the grace never applies to anything. That is the
    /// option the maintainer rejected, arrived at by omission.
    #[test]
    fn an_unknown_read_back_from_the_store_still_honours_its_grace() {
        let c = conn();
        // A real check at 3,400,000 sets the anchor.
        record(
            &c,
            &ConsensusCompatibility::Current,
            BlockHeight::new(3_400_000),
            T0,
        )
        .expect("record current");

        // The endpoint goes quiet 100 blocks later — well inside the grace.
        record(
            &c,
            &ConsensusCompatibility::Unknown {
                judged_at_height: BlockHeight::new(3_400_100),
                blocks_since_last_current: Some(100),
                clock: at_t0(10_000),
            },
            BlockHeight::new(3_400_100),
            T0 + 10_000,
        )
        .expect("record unknown");
        let inside = observe(&c, T0 + 10_000)
            .expect("read")
            .expect("row")
            .verdict
            .expect("verdict");
        assert!(
            inside.permits_signing(),
            "an Unknown INSIDE its grace must still sign after a store round trip, \
             or the grace rule exists only in the predicate and never in production"
        );

        // Far past the grace IN BLOCKS: the same round trip must refuse — read
        // back inside the day, so this row stays about the block rule.
        record(
            &c,
            &ConsensusCompatibility::Unknown {
                judged_at_height: BlockHeight::new(3_400_000 + UNKNOWN_BRANCH_GRACE_BLOCKS + 1),
                blocks_since_last_current: Some(UNKNOWN_BRANCH_GRACE_BLOCKS + 1),
                clock: at_t0(10_100),
            },
            BlockHeight::new(3_400_000 + UNKNOWN_BRANCH_GRACE_BLOCKS + 1),
            T0 + 10_100,
        )
        .expect("record unknown outside");
        let outside = observe(&c, T0 + 10_100)
            .expect("read")
            .expect("row")
            .verdict
            .expect("verdict");
        assert!(
            !outside.permits_signing(),
            "and past the grace it must refuse"
        );
    }

    /// §6.3's anti-extension rule, made mechanical: an `Unknown` riding the
    /// grace must not move the anchor it rides.
    #[test]
    fn an_unknown_verdict_does_not_move_the_grace_anchor() {
        let c = conn();
        record(
            &c,
            &ConsensusCompatibility::Current,
            BlockHeight::new(3_400_000),
            T0,
        )
        .expect("record current");
        assert_eq!(
            observe(&c, T0)
                .expect("read")
                .expect("row")
                .grace_anchor_tip,
            Some(BlockHeight::new(3_400_000))
        );

        // Ten thousand blocks later the endpoint stops reporting its branch.
        record(
            &c,
            &ConsensusCompatibility::Unknown {
                judged_at_height: BlockHeight::new(3_410_000),
                blocks_since_last_current: Some(10_000),
                clock: GraceClock::NONE,
            },
            BlockHeight::new(3_410_000),
            T0 + 900_000,
        )
        .expect("record unknown");
        assert_eq!(
            observe(&c, T0)
                .expect("read")
                .expect("row")
                .grace_anchor_tip,
            Some(BlockHeight::new(3_400_000)),
            "the anchor must stay where the last REAL check was, or the grace never expires"
        );
    }

    /// GRACE-1 (§4p P-G1, G-10's "the time not carried forward" mutant): the
    /// CAPABLE time rides across `Unknown` verdicts exactly as `capable_tip`
    /// does — a silent server's passes refresh `at_unix_secs` (the latest
    /// verdict's time) and NOT the instant the clock rule measures from; a
    /// capable verdict renews it. Read through `observe`: the elapsed reading is
    /// the proof the column is the capable one. Mutant: `record` writing
    /// `Some(at_unix_secs)` on every verdict — the second elapsed reads 0.
    #[test]
    fn the_capable_time_is_carried_across_unknown_verdicts_and_renewed_by_a_capable_one() {
        let c = conn();
        record(
            &c,
            &ConsensusCompatibility::Current,
            BlockHeight::new(3_400_000),
            T0,
        )
        .expect("capable at T0");
        // Two silent passes, hours apart, each stamping its own `at`.
        for hours in [1_u64, 2] {
            record(
                &c,
                &ConsensusCompatibility::Unknown {
                    judged_at_height: BlockHeight::new(3_400_000),
                    blocks_since_last_current: Some(0),
                    clock: GraceClock::NONE,
                },
                BlockHeight::new(3_400_000),
                T0 + hours * 3_600,
            )
            .expect("silent pass");
        }
        let snap = observe(&c, T0 + 3 * 3_600).expect("read").expect("row");
        assert_eq!(
            snap.evaluated_at_unix,
            Some(T0 + 2 * 3_600),
            "`at_unix_secs` is the LATEST verdict's time"
        );
        assert_eq!(
            snap.grace_clock.elapsed_secs,
            Some(3 * 3_600),
            "but the clock rule measures from the CAPABLE time, carried across the silent passes"
        );
        // A capable verdict renews it.
        record(
            &c,
            &ConsensusCompatibility::Current,
            BlockHeight::new(3_400_000),
            T0 + 4 * 3_600,
        )
        .expect("capable again");
        assert_eq!(
            observe(&c, T0 + 4 * 3_600 + 5)
                .expect("read")
                .expect("row")
                .grace_clock
                .elapsed_secs,
            Some(5)
        );
    }

    /// GRACE-1 (§4p G-1's cold-read clause, G-2b, item 6): a day on the device
    /// clock ends the grace ON A COLD READ — no pass in between — and the
    /// expiry LATCHES: the clock set back inside the day still refuses, on
    /// every later read, until one capable verdict clears it. Mutants: the
    /// `UPDATE` in `observe` removed (the set-back read permits); the latch
    /// not cleared on a capable verdict (the last assertion refuses); the clock
    /// consulted only at the pass (the first refusal never happens — there is
    /// no pass here).
    #[test]
    fn a_day_on_the_clock_ends_the_grace_on_a_cold_read_and_the_latch_survives_a_set_back_clock() {
        let c = conn();
        record(
            &c,
            &ConsensusCompatibility::Current,
            BlockHeight::new(3_400_000),
            T0,
        )
        .expect("capable at T0");
        record(
            &c,
            &ConsensusCompatibility::Unknown {
                judged_at_height: BlockHeight::new(3_400_000),
                blocks_since_last_current: Some(0),
                clock: at_t0(60),
            },
            BlockHeight::new(3_400_000),
            T0 + 60,
        )
        .expect("silent pass, frozen tip");
        let verdict_at = |now: u64| {
            observe(&c, now)
                .expect("read")
                .expect("row")
                .verdict
                .expect("verdict")
        };
        assert!(
            verdict_at(T0 + 3_600).permits_signing(),
            "an hour in: inside both rules"
        );
        let expired = verdict_at(T0 + UNKNOWN_BRANCH_GRACE_SECS);
        assert!(
            !expired.permits_signing(),
            "a day later, with NO pass and the tip frozen at zero blocks: refused by the clock"
        );
        assert!(
            matches!(
                expired,
                ConsensusCompatibility::Unknown {
                    clock: GraceClock { latched: true, .. },
                    ..
                }
            ),
            "and the expiry is latched on the row: {expired:?}"
        );
        // The clock is set BACK inside the day (G-2b) — even to before the
        // capable time.
        for set_back_to in [T0 + 3_600, T0 - 3_600] {
            let v = verdict_at(set_back_to);
            assert!(
                !v.permits_signing(),
                "a clock set back to {set_back_to} does not re-permit: {v:?}"
            );
        }
        // Only a capable verdict clears the latch — and renews the time.
        record(
            &c,
            &ConsensusCompatibility::Current,
            BlockHeight::new(3_400_000),
            T0 + 3_600,
        )
        .expect("capable pass on a server that reports its branch");
        record(
            &c,
            &ConsensusCompatibility::Unknown {
                judged_at_height: BlockHeight::new(3_400_000),
                blocks_since_last_current: Some(0),
                clock: at_t0(0),
            },
            BlockHeight::new(3_400_000),
            T0 + 3_600,
        )
        .expect("silent again");
        let restored = verdict_at(T0 + 2 * 3_600);
        assert!(
            restored.permits_signing(),
            "one capable pass restores the grace: {restored:?}"
        );
    }

    /// GRACE-2 (§4v G2-1, the store seam; the test author's blind twin through
    /// the shipped path is G2-2): a capable time LATER than now is a time the
    /// clock cannot vouch for, and the clock rule is EXPIRED — `observe` writes
    /// the latch, the reading is `{ elapsed_secs: None, latched: true }`, the
    /// grace ends `by: Clock` ten blocks in, and the latch is DURABLE: a re-read
    /// at a later `now` still below the capable time, and at a `now` above it
    /// (the latch, not the reading — `elapsed` is then a minute), both refuse;
    /// one capable `record` clears it and renews the time from the clock. At
    /// the base: `elapsed None`, not latched, not expired — blocks decided, and
    /// a frozen tip held the grace open (§4p-run review row 2). Mutant: the
    /// future-time branch removed (the base's `capable_at <= now` filter
    /// restored) — the first assertion reads `latched: false`.
    #[test]
    fn a_capable_time_later_than_now_is_an_expired_clock_rule() {
        let c = conn();
        record(
            &c,
            &ConsensusCompatibility::Current,
            BlockHeight::new(3_400_000),
            T0 + 7 * 86_400,
        )
        .expect("capable, a week ahead");
        record(
            &c,
            &ConsensusCompatibility::Unknown {
                judged_at_height: BlockHeight::new(3_400_010),
                blocks_since_last_current: Some(10),
                clock: GraceClock::NONE,
            },
            BlockHeight::new(3_400_010),
            T0,
        )
        .expect("silent");
        let snap = observe(&c, T0).expect("read").expect("row");
        assert_eq!(
            snap.grace_clock,
            GraceClock {
                elapsed_secs: None,
                latched: true,
            },
            "a capable time later than now is latched expired, not abstained"
        );
        let v = snap.verdict.expect("verdict");
        assert!(
            !v.permits_signing(),
            "…and the grace is ended by the clock: {v:?}"
        );
        assert_eq!(
            v.unknown_branch_grace(),
            Some(crate::state::UnknownBranchGrace::Ended {
                by: crate::state::GraceExpiry::Clock,
                blocks_since_last_current: Some(10),
            }),
            "ended by the clock, ten blocks in — never blocks-and-no-time"
        );
        // Durable: a later `now` still below the capable time, and a `now`
        // above it (a minute past — the latch refuses, not the reading).
        for later in [T0 + 86_400, T0 + 7 * 86_400 + 60] {
            let v = observe(&c, later)
                .expect("read")
                .expect("row")
                .verdict
                .expect("verdict");
            assert!(
                matches!(
                    v,
                    ConsensusCompatibility::Unknown {
                        clock: GraceClock { latched: true, .. },
                        ..
                    }
                ),
                "the latch holds at {later}: {v:?}"
            );
            assert!(!v.permits_signing(), "…and refuses at {later}: {v:?}");
        }
        // One capable verdict clears it and re-records the time from the clock.
        record(
            &c,
            &ConsensusCompatibility::Current,
            BlockHeight::new(3_400_010),
            T0 + 7 * 86_400 + 60,
        )
        .expect("capable pass on a server that reports its branch");
        assert_eq!(
            observe(&c, T0 + 7 * 86_400 + 60)
                .expect("read")
                .expect("row")
                .grace_clock,
            GraceClock {
                elapsed_secs: Some(0),
                latched: false,
            },
            "cleared, and the capable time is the clock's reading now"
        );
    }

    /// GRACE-2 (§4v G2-3's store half; §4p-run review row 7's cell): a capable
    /// time of ZERO — a pre-epoch device clock at the capable pass — is a time
    /// the clock cannot vouch for, and the clock rule is EXPIRED at any `now`:
    /// at a clock of 0, where the base's filter passed, `elapsed` read
    /// `Some(0)` and the surface promised "about 24 more hours"; and at a real
    /// clock, where the base latched by the DAY on a nonsense elapsed of
    /// ~1.7e9 s (the `elapsed_secs` assertion tells the two apart). Mutant:
    /// the `== 0` cell dropped from `observe` — the clock-of-0 read permits
    /// with a day left, and the real-clock read shows `elapsed_secs: Some(T0)`.
    #[test]
    fn a_capable_time_of_zero_is_an_expired_clock_rule_not_a_day_of_grace() {
        for now in [0, T0] {
            let c = conn();
            record(
                &c,
                &ConsensusCompatibility::Current,
                BlockHeight::new(3_400_000),
                0,
            )
            .expect("capable, at a pre-epoch clock");
            record(
                &c,
                &ConsensusCompatibility::Unknown {
                    judged_at_height: BlockHeight::new(3_400_000),
                    blocks_since_last_current: Some(0),
                    clock: GraceClock::NONE,
                },
                BlockHeight::new(3_400_000),
                now,
            )
            .expect("silent, frozen tip");
            let snap = observe(&c, now).expect("read").expect("row");
            assert_eq!(
                snap.grace_clock,
                GraceClock {
                    elapsed_secs: None,
                    latched: true,
                },
                "a capable time of 0 is untrusted at now={now}: latched, no elapsed reading"
            );
            let v = snap.verdict.expect("verdict");
            assert!(!v.permits_signing(), "refused at now={now}: {v:?}");
            assert_eq!(
                v.unknown_branch_grace(),
                Some(crate::state::UnknownBranchGrace::Ended {
                    by: crate::state::GraceExpiry::Clock,
                    blocks_since_last_current: Some(0),
                }),
                "never \"about 24 more hours\" at now={now}"
            );
        }
    }

    /// GRACE-2 (§4v G2-6, §4p-run review row 8): `record`'s saturation of the
    /// capable time to `i64::MAX` points toward LESS grace — the round trip
    /// reads back as a capable time later than any real `now`, which is the
    /// §4v cell: latched, expired by the clock. Mutant: the future-time branch
    /// removed (the base abstained here: `elapsed None`, not latched, blocks
    /// deciding — the saturation then pointed toward MORE grace).
    #[test]
    fn a_saturated_capable_time_round_trips_as_expired_by_the_clock() {
        let c = conn();
        record(
            &c,
            &ConsensusCompatibility::Current,
            BlockHeight::new(3_400_000),
            u64::MAX,
        )
        .expect("capable, at a clock past what i64 holds");
        record(
            &c,
            &ConsensusCompatibility::Unknown {
                judged_at_height: BlockHeight::new(3_400_000),
                blocks_since_last_current: Some(0),
                clock: GraceClock::NONE,
            },
            BlockHeight::new(3_400_000),
            T0,
        )
        .expect("silent, frozen tip");
        let snap = observe(&c, T0).expect("read").expect("row");
        assert_eq!(
            snap.grace_clock,
            GraceClock {
                elapsed_secs: None,
                latched: true,
            },
            "i64::MAX reads back as later than now: expired by the rule, never abstained"
        );
        assert!(
            !snap.verdict.expect("verdict").permits_signing(),
            "a saturated capable time buys no grace"
        );
    }

    /// GRACE-1 (§4p item 6; §4p-run row 2's arch review): the clock threshold
    /// is ONE comparison (`GraceClock::past_threshold`) asked at both sites —
    /// where `observe` WRITES the latch and where `GraceClock::expired` READS
    /// the rule for `permits_signing` — so this row pins the boundary through
    /// both at once: one second short of `UNKNOWN_BRANCH_GRACE_SECS` nothing is
    /// latched and the grace runs with one second left; at exactly
    /// `UNKNOWN_BRANCH_GRACE_SECS` the latch is written and the grace is ended
    /// by the clock. Mutants: `>` in place of `>=` (the day exactly permits and
    /// nothing is latched); the threshold off by one early (the second short
    /// latches and refuses).
    #[test]
    fn the_clock_threshold_is_exclusive_at_the_write_and_the_read_alike() {
        let c = conn();
        record(
            &c,
            &ConsensusCompatibility::Current,
            BlockHeight::new(3_400_000),
            T0,
        )
        .expect("capable at T0");
        record(
            &c,
            &ConsensusCompatibility::Unknown {
                judged_at_height: BlockHeight::new(3_400_000),
                blocks_since_last_current: Some(0),
                clock: at_t0(0),
            },
            BlockHeight::new(3_400_000),
            T0,
        )
        .expect("silent, frozen tip");
        let verdict_at = |now: u64| {
            observe(&c, now)
                .expect("read")
                .expect("row")
                .verdict
                .expect("verdict")
        };

        let one_short = verdict_at(T0 + UNKNOWN_BRANCH_GRACE_SECS - 1);
        assert!(
            matches!(
                one_short,
                ConsensusCompatibility::Unknown {
                    clock: GraceClock {
                        elapsed_secs: Some(secs),
                        latched: false,
                    },
                    ..
                } if secs == UNKNOWN_BRANCH_GRACE_SECS - 1
            ),
            "one second short of the day: read as elapsed, NOT latched: {one_short:?}"
        );
        assert!(
            one_short.permits_signing(),
            "…and the grace still runs: {one_short:?}"
        );
        assert_eq!(
            one_short.unknown_branch_grace(),
            Some(crate::state::UnknownBranchGrace::Running {
                blocks_left: UNKNOWN_BRANCH_GRACE_BLOCKS,
                secs_left: Some(1),
            }),
            "with one second left"
        );

        let at_day = verdict_at(T0 + UNKNOWN_BRANCH_GRACE_SECS);
        assert!(
            matches!(
                at_day,
                ConsensusCompatibility::Unknown {
                    clock: GraceClock { latched: true, .. },
                    ..
                }
            ),
            "the day exactly: the latch is WRITTEN: {at_day:?}"
        );
        assert!(
            !at_day.permits_signing(),
            "…and the rule READS as expired: {at_day:?}"
        );
        assert_eq!(
            at_day.unknown_branch_grace(),
            Some(crate::state::UnknownBranchGrace::Ended {
                by: crate::state::GraceExpiry::Clock,
                blocks_since_last_current: Some(0),
            }),
            "ended by the clock, zero blocks in"
        );
    }

    /// GRACE-1 (§4p-run row 2; found by the test author's G-7b): the latch is
    /// an observation of real time, written on a CAPABLE row too — a capable
    /// pass, then a cold read a day later with NO pass between. The wallet is
    /// on no grace (it still signs; there is nothing to show), yet the latch
    /// is written, so a silent pass recorded afterwards INHERITS it: its grace
    /// starts already ended, even with the clock then set back inside the day
    /// — the maintainer's "the clock only tightens". Mutant: the write skipped
    /// on non-`Unknown` rows (the capable read-back shows `latched: false`,
    /// and the set-back read after the silent pass permits).
    #[test]
    fn a_day_with_no_pass_latches_the_clock_on_a_capable_row_too() {
        let c = conn();
        record(
            &c,
            &ConsensusCompatibility::Current,
            BlockHeight::new(3_400_000),
            T0,
        )
        .expect("capable at T0");
        let a_day_later = observe(&c, T0 + UNKNOWN_BRANCH_GRACE_SECS)
            .expect("read")
            .expect("row");
        assert_eq!(
            a_day_later.verdict,
            Some(ConsensusCompatibility::Current),
            "a capable row stays capable: no grace, still signs"
        );
        assert!(a_day_later.verdict.expect("verdict").permits_signing());
        assert_eq!(
            a_day_later.grace_clock,
            GraceClock {
                elapsed_secs: Some(UNKNOWN_BRANCH_GRACE_SECS),
                latched: true,
            },
            "…and the day is latched on the row all the same"
        );
        // A silent pass recorded now inherits the latch — and the clock set
        // BACK inside the day afterwards (G-2b's shape) does not un-inherit it.
        record(
            &c,
            &ConsensusCompatibility::Unknown {
                judged_at_height: BlockHeight::new(3_400_000),
                blocks_since_last_current: Some(0),
                clock: GraceClock::NONE,
            },
            BlockHeight::new(3_400_000),
            T0 + UNKNOWN_BRANCH_GRACE_SECS,
        )
        .expect("silent pass, a day later");
        let set_back = observe(&c, T0 + 3_600)
            .expect("read")
            .expect("row")
            .verdict
            .expect("verdict");
        assert!(
            !set_back.permits_signing(),
            "the silent pass starts its grace already ended, whatever the clock reads: {set_back:?}"
        );
        assert_eq!(
            set_back.unknown_branch_grace(),
            Some(crate::state::UnknownBranchGrace::Ended {
                by: crate::state::GraceExpiry::Clock,
                blocks_since_last_current: Some(0),
            })
        );
    }

    /// The dev-stage format bump (module doc): a table of the pre-GRACE-1 shape
    /// is dropped whole and recreated — its row reads as "no verdict" (the
    /// fail-closed direction), and the new columns exist. Mutant: the probe
    /// removed — the SELECT in `read_row` fails on the missing columns.
    #[test]
    fn an_old_shape_table_is_dropped_and_recreated() {
        let c = Connection::open_in_memory().expect("in-memory db");
        c.execute_batch(
            "CREATE TABLE consensus_verdict (
                 singleton       INTEGER PRIMARY KEY CHECK (singleton = 1),
                 kind            TEXT    NOT NULL,
                 expected_branch INTEGER,
                 endpoint_branch INTEGER,
                 judged_height   INTEGER,
                 scanned_tip     INTEGER,
                 claimed_tip     INTEGER,
                 capable_tip     INTEGER,
                 at_unix_secs    INTEGER NOT NULL
             );
             INSERT INTO consensus_verdict (singleton, kind, capable_tip, at_unix_secs)
                 VALUES (1, 'current', 3400000, 1);",
        )
        .expect("the pre-GRACE-1 shape, with a row");
        ensure_table(&c).expect("ensure");
        assert!(has_column(&c, SHAPE_MARKER_COLUMN).expect("pragma"));
        assert!(has_column(&c, "clock_expired").expect("pragma"));
        assert_eq!(
            observe(&c, T0).expect("read"),
            None,
            "the old row is gone: no verdict until the next pass"
        );
        // And idempotent on the new shape: a second ensure keeps the row.
        record(
            &c,
            &ConsensusCompatibility::Current,
            BlockHeight::new(1),
            T0,
        )
        .expect("record");
        ensure_table(&c).expect("ensure again");
        assert!(observe(&c, T0).expect("read").is_some());
    }

    /// An uninterpretable row is "no verdict", never a fabricated `Current` —
    /// the fail-closed direction.
    #[test]
    fn an_unreadable_row_is_no_verdict_not_current() {
        let c = conn();
        c.execute(
            "INSERT INTO consensus_verdict (singleton, kind, at_unix_secs) VALUES (1, 'martian', 1)",
            [],
        )
        .expect("insert");
        let snapshot = observe(&c, T0).expect("read").expect("a row exists");
        assert_eq!(snapshot.verdict, None);
    }
}
