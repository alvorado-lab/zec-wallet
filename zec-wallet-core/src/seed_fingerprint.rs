//! The seed fingerprint (#357) — at-rest verification of a host-supplied seed.
//!
//! A domain-separated SHA-256 of the wallet's seed, recorded at fresh provision
//! for `SeedPersistence::None` wallets. It closes the §3.2f caveat: a
//! None-persistence store keeps NO seed at rest (the host re-supplies it each
//! `open`), so nothing could verify the supplied seed was the one this wallet
//! was created with — a wrong seed silently opened an empty-LOOKING wallet (the
//! "my funds vanished" money-scare). With the fingerprint, `open` recomputes the
//! supplied seed's fingerprint and constant-time-compares; a mismatch is a typed
//! [`WalletError::SeedMismatch`].
//!
//! **Why None-persistence ONLY.** A `SealedKeychain` wallet keeps its seed
//! sealed at rest, so `store::repair` already constant-time-compares the supplied
//! seed against the SEALED one (`store.rs`); a fingerprint there would be
//! redundant (there is nothing to compare it against at `open` but the seal
//! itself). None-persistence is the ONLY mode with no sealed seed — so the
//! fingerprint is the ONLY at-rest check, and is written there alone (minimal
//! at-rest seed-derived surface).
//!
//! **Privacy / at-rest disclosure.** The fingerprint is a PREIMAGE-RESISTANT
//! one-way hash: at rest it discloses nothing recoverable about the seed. It
//! lives ONLY inside the SQLCipher-encrypted `wallet.db` (NEVER the plaintext
//! `wallet.manifest`, NEVER the FFI surface, NEVER a `tracing`/log line — §5.4);
//! an actor who can read it already holds the DB key and, with it, the full tx
//! history — so it grants no new capability. It is derived FROM the seed but is
//! not itself a secret.
//!
//! **Domain separation.** `SHA-256(DOMAIN_TAG ‖ seed)` with a FROZEN tag, so the
//! fingerprint can never alias any other SHA-256 use of the same seed (e.g. a
//! keychain blob hash). The tag is versioned; a future scheme change bumps it —
//! it is NEVER edited in place (frozen-label crypto rule).
//!
//! **Fail-open READ, fail-CLOSED mismatch.** [`verify`] rejects only on a
//! successfully-read fingerprint that DIFFERS (`Mismatch`). An absent fingerprint
//! (pre-#357 wallet, or a read fault, or a tamper-truncated blob) degrades to
//! `Absent` — verification is SKIPPED, exactly the pre-#357 posture (the §4.2
//! port contract carries the money-safety, and a wallet that cannot open cannot
//! sync). A tamper of the blob only DISABLES a defence-in-depth check; an actor
//! who can write it already holds the DB key (game-over regardless).
//!
//! **Availability rests on SQLCipher page integrity (LOAD-BEARING).** The one
//! way `verify` could wrongly `Mismatch` a CORRECT seed and lock a legitimate
//! user out is a stored fingerprint mutated to a DIFFERENT valid-length value.
//! Benign at-rest corruption cannot do this: SQLCipher's per-page authentication
//! fails the HMAC on a flipped bit, so `read` gets a decrypt `Err` and degrades
//! to `Absent` (SKIP), NEVER a false `Mismatch`. A wrong-but-valid fingerprint
//! is only producible by a DELIBERATE write under the DB key (already game-over),
//! and recovery is restore-from-phrase (the same posture as a corrupted seal).
//!
//! **Storage / rescan.** Same discipline as [`crate::creation_stamp`]: pure SQL
//! over the aux SQLCipher connection; created idempotently in `db::migrate`;
//! rides the rescan copy UNCLEARED (`AUX_TABLES_PRESERVED`, ADR-0534 — the seed
//! a rescan keeps is unchanged, so its fingerprint must survive). Dies with the
//! wallet identity (the whole `wallet.db` is crypto-shredded at wipe).

use rusqlite::{Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::db::map_aux_err;
use crate::error::WalletError;

/// FROZEN domain-separation tag (crypto rule: NEVER edit; a scheme change bumps
/// the `/vN` suffix and gets its own migration).
const DOMAIN_TAG: &[u8] = b"zec-wallet/seed-fingerprint/v1";

/// SHA-256 output width — the exact stored blob length (validate-never-truncate).
const FINGERPRINT_LEN: usize = 32;

/// The single-row table name — referenced by `db::AUX_TABLES_PRESERVED`
/// (ADR-0534).
pub(crate) const TABLE: &str = "wallet_seed_fingerprint";

/// The outcome of comparing a supplied seed to the stored fingerprint.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Verdict {
    /// A fingerprint was stored and the supplied seed reproduces it.
    Match,
    /// A fingerprint was stored and the supplied seed does NOT reproduce it —
    /// the caller rejects with `SeedMismatch`.
    Mismatch,
    /// No usable fingerprint stored (pre-#357 wallet, read fault, or a
    /// tamper-truncated blob) — verification is SKIPPED (the pre-#357 posture).
    Absent,
}

/// The domain-separated fingerprint of `seed`. `seed` is borrowed and never
/// copied or stored — only this one-way digest is. NOT a secret (preimage-
/// resistant), so no `Zeroizing` return.
fn fingerprint(seed: &[u8]) -> [u8; FINGERPRINT_LEN] {
    let mut h = Sha256::new();
    h.update(DOMAIN_TAG);
    h.update(seed);
    h.finalize().into()
}

/// Create the fingerprint table (idempotent; runs in `db::migrate` on every
/// provision AND open — a pre-#357 or `SealedKeychain` wallet simply has no row,
/// which reads `Absent`).
pub(crate) fn ensure_table(conn: &Connection) -> Result<(), WalletError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS wallet_seed_fingerprint (
             singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
             fp        BLOB NOT NULL
         );",
    )
    .map_err(map_aux_err)
}

/// Record the fingerprint of `seed` ONCE (single row, single statement —
/// crash-atomic, takes `RESERVED` directly per the aux-write invariant). `ON
/// CONFLICT DO NOTHING`: an idempotent re-provision keeps the first fingerprint
/// (the seed is fixed at create time, so a re-run reproduces the same bytes
/// anyway). Written ONLY on a fresh `SeedPersistence::None` provision.
pub(crate) fn record_once(conn: &Connection, seed: &[u8]) -> Result<(), WalletError> {
    let fp = fingerprint(seed);
    conn.execute(
        "INSERT INTO wallet_seed_fingerprint (singleton, fp) VALUES (1, ?1)
         ON CONFLICT(singleton) DO NOTHING",
        rusqlite::params![&fp[..]],
    )
    .map_err(map_aux_err)?;
    Ok(())
}

/// Read the stored fingerprint, if any. Returns `None` for a wallet that never
/// stored one (pre-#357 / `SealedKeychain`), on any read fault, or on a
/// wrong-length (tampered) blob — the fail-open direction (see the module doc).
/// No value is logged (§5.4).
fn read(conn: &Connection) -> Option<[u8; FINGERPRINT_LEN]> {
    let row: Result<Option<Vec<u8>>, rusqlite::Error> = conn
        .query_row(
            "SELECT fp FROM wallet_seed_fingerprint WHERE singleton = 1",
            [],
            |r| r.get(0),
        )
        .optional();
    match row {
        Ok(Some(bytes)) if bytes.len() == FINGERPRINT_LEN => {
            let mut fp = [0u8; FINGERPRINT_LEN];
            fp.copy_from_slice(&bytes);
            Some(fp)
        }
        Ok(Some(_)) => {
            tracing::warn!(
                target: "zec_wallet_core",
                "seed fingerprint has wrong length; treating as absent (verification skipped)"
            );
            None
        }
        Ok(None) => None,
        Err(_) => {
            tracing::warn!(
                target: "zec_wallet_core",
                "seed fingerprint unreadable; treating as absent (verification skipped)"
            );
            None
        }
    }
}

/// Compare `seed` to the stored fingerprint in constant time. `Match`/`Mismatch`
/// when a fingerprint is stored; `Absent` when none is (⇒ the caller skips the
/// check, the pre-#357 posture). Constant-time so a mistyped/hostile seed cannot
/// become a timing oracle for the stored bytes.
pub(crate) fn verify(conn: &Connection, seed: &[u8]) -> Verdict {
    match read(conn) {
        None => Verdict::Absent,
        Some(stored) => {
            let computed = fingerprint(seed);
            if bool::from(computed.ct_eq(&stored)) {
                Verdict::Match
            } else {
                Verdict::Mismatch
            }
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

    // ── KAT vectors (computed offline: `SHA-256(DOMAIN_TAG ‖ seed)`) ──
    // seed = 0x00..0x1f
    const KAT_SEED_32: [u8; 32] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
        0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d,
        0x1e, 0x1f,
    ];
    const KAT_FP_32: [u8; 32] = [
        0xbe, 0x65, 0xae, 0x3b, 0x6c, 0xe6, 0xc6, 0xf7, 0xd2, 0xbb, 0xf5, 0xc3, 0xda, 0x31, 0x94,
        0x32, 0x42, 0x9c, 0x70, 0xfb, 0x39, 0x0f, 0xcb, 0x6e, 0x0d, 0xc6, 0x3e, 0x73, 0x6d, 0x78,
        0x13, 0xf2,
    ];

    #[test]
    fn fingerprint_matches_the_known_answer_vector() {
        // Pins the exact domain-separated construction against an offline
        // reference — a silent change to DOMAIN_TAG or the hash input order
        // fails HERE, never silently re-keys existing wallets.
        assert_eq!(fingerprint(&KAT_SEED_32), KAT_FP_32);
    }

    #[test]
    fn a_64_byte_bip39_shaped_seed_matches_its_known_answer() {
        let seed64: Vec<u8> = (0u8..64).collect();
        let expected = [
            0x19, 0xee, 0xf1, 0xb8, 0x8f, 0x29, 0x1e, 0x68, 0x83, 0x69, 0x5f, 0x06, 0x1c, 0x85,
            0x71, 0x20, 0xcd, 0x78, 0xc7, 0xab, 0x38, 0xf4, 0xa8, 0x8c, 0x43, 0xa0, 0xfd, 0x1a,
            0x6d, 0x18, 0x44, 0x7d,
        ];
        assert_eq!(fingerprint(&seed64), expected);
    }

    #[test]
    fn the_domain_tag_actually_separates_from_a_plain_sha256() {
        // The fingerprint MUST differ from an undomained SHA-256 of the same
        // seed — otherwise the separation is cosmetic.
        let plain: [u8; 32] = {
            let mut h = Sha256::new();
            h.update(KAT_SEED_32);
            h.finalize().into()
        };
        assert_ne!(fingerprint(&KAT_SEED_32), plain);
    }

    #[test]
    fn distinct_seeds_fingerprint_distinctly() {
        assert_ne!(fingerprint(&KAT_SEED_32), fingerprint(&[0xffu8; 32]));
    }

    #[test]
    fn record_then_verify_matches_the_same_seed() {
        let c = conn();
        record_once(&c, &KAT_SEED_32).expect("record");
        assert_eq!(verify(&c, &KAT_SEED_32), Verdict::Match);
    }

    #[test]
    fn verify_rejects_a_wrong_seed() {
        let c = conn();
        record_once(&c, &KAT_SEED_32).expect("record");
        assert_eq!(verify(&c, &[0xffu8; 32]), Verdict::Mismatch);
    }

    #[test]
    fn verify_is_absent_before_any_record() {
        let c = conn();
        assert_eq!(verify(&c, &KAT_SEED_32), Verdict::Absent);
    }

    #[test]
    fn record_is_write_once_and_keeps_the_first_fingerprint() {
        let c = conn();
        record_once(&c, &KAT_SEED_32).expect("record");
        // A second record with a DIFFERENT seed must NOT overwrite (write-once);
        // in production the seed is fixed, so this only guards the invariant.
        record_once(&c, &[0xffu8; 32]).expect("record again");
        assert_eq!(verify(&c, &KAT_SEED_32), Verdict::Match);
        let rows: i64 = c
            .query_row("SELECT COUNT(*) FROM wallet_seed_fingerprint", [], |r| {
                r.get(0)
            })
            .expect("count");
        assert_eq!(rows, 1);
    }

    #[test]
    fn a_tamper_truncated_blob_reads_absent_never_a_false_match() {
        let c = conn();
        // Plant a wrong-length blob directly: verification is SKIPPED (Absent),
        // never a spurious Match/Mismatch on a partial compare.
        c.execute(
            "INSERT INTO wallet_seed_fingerprint (singleton, fp) VALUES (1, ?1)",
            rusqlite::params![&[0u8; 16][..]],
        )
        .expect("plant");
        assert_eq!(verify(&c, &KAT_SEED_32), Verdict::Absent);
    }

    #[test]
    fn a_missing_table_verifies_absent_never_an_error() {
        let c = Connection::open_in_memory().expect("in-memory db");
        assert_eq!(verify(&c, &KAT_SEED_32), Verdict::Absent);
    }
}
