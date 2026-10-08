//! R12 §4.4 — a busy store is not a corrupt one, and a blind mapping does not
//! come back (`docs/plan/r12-a-busy-store-is-not-a-corrupt-one.md`).
//!
//! THE RULE, stated once. Every bare `StoreCorrupt` token in the PRODUCTION code
//! of `zec-wallet-core/src` is counted, keyed on (file, enclosing fn), and the
//! count is compared to [`ALLOWLIST`] — each entry carries its class and a
//! one-line reason. A count that RISES anywhere fails: a new `StoreCorrupt` is a
//! claim that the wallet's own bytes are damaged, and an error that came out of
//! SQLite or the engine is not that claim until it has been classified
//! (`db::map_aux_err` for a `rusqlite::Error`, `ClassifyStoreFault::into_store_fault`
//! for an engine error). A count that FALLS prints a reminder to lower the entry
//! — except the one [`FLOOR`] entry (R13 §4.1), whose fall fails.
//!
//! What a count is, and what it is not:
//!
//!   * the bare TOKEN, not the path — a `Self::StoreCorrupt`, a pattern
//!     (`store.rs`'s `resolve_custody`, the sweep and reclaim hard-fault lists)
//!     and a construction all count the same; a pattern is harmless and the
//!     count stays consistent;
//!   * read over a LEXED text: line and block comments (nested), strings, byte
//!     strings, raw strings and char literals are blanked before anything is
//!     matched, so a doc line that names the variant is not a site and a
//!     `format!("… {table} …")` brace cannot move the enclosing-fn match.
//!     A char literal is told apart from a lifetime (`'{'` is blanked, `'a`
//!     is not);
//!   * keyed on the ENCLOSING fn found by brace matching over that lexed text,
//!     so a rustfmt-wrapped `|_e| { … StoreCorrupt }` is one site of the fn it
//!     sits in, wherever the line breaks fall. Inside an `impl` the key carries
//!     the type (`Wallet::scanned_tip`, `<SqliteClientError as
//!     ClassifyStoreFault>::into_store_fault`).
//!
//! Test code is out of scope, and the exclusion is an ITEM: `#[cfg(test)]` (or
//! `#[cfg(all(test, …))]`) on a module, fn, impl or statement skips that item
//! and the scan resumes after it (`wallet.rs` carries statement-level ones
//! inside production fns). `cfg_attr(not(test), …)` and `cfg(any(test, …))` are
//! NOT test code — both compile into production. Which FILES are production is
//! answered the way rustc answers it: a walk from `lib.rs` over every `mod x;`
//! that is not itself under a `cfg(test)` item, resolved per the reference
//! (`#[path]`, a non-mod-rs declarer owning `<stem>/`, inline module components).
//!
//! Denied outright, because each makes a blind mapping the count cannot see:
//!   * a `use` that imports `StoreCorrupt` (an alias `as Bad` hides every use
//!     site) or globs `WalletError::*`;
//!   * a `macro_rules!` whose body names it;
//!   * a `const`/`static` item or a `const fn` whose body names it;
//!   * an `impl From<…> for WalletError` whose body names it;
//!   * a fn returning a bare `WalletError` that names it and is a MAPPER rather
//!     than a classifier — generic, or taking an ignored (`_`-named) argument,
//!     or deciding nothing (no `match`/`if` in its body) — so `.map_err(helper)`
//!     would carry the blind mapping to sites with no token;
//!   * a `let` binding of a blind closure (`let c = |_| … StoreCorrupt;`), for
//!     the same reason.
//!
//! What this cannot see, stated rather than smoothed (§4.4): a class-B site
//! deleted and a blind site added in the SAME fn nets zero. The key includes the
//! fn and every entry names its class so that a reviewer reading the table's
//! diff — which the commit shows — is the check for that case.
//!
//! No `syn`: the lexer below is what the evasions need, and a parser is a
//! supply-chain addition for a test.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

/// The allowlist for the POST-SWEEP tree: (file relative to `src/`, enclosing
/// fn key, count, class, reason). Classes:
///   * `B` — a decode, range check or invariant over OUR OWN persisted bytes,
///     or a compiled-in asset: that IS corruption (§3);
///   * `B-inv` — an `ok_or(StoreCorrupt)` / `return Err(StoreCorrupt)` over a
///     value an earlier read already returned (None where the store guarantees
///     Some, a state the schema forbids) — invariant checks, class B (survey);
///   * `D` — not a store error (a poisoned mutex, endpoint data); honest kind
///     owed, ledger R15 (LOW), §4.4;
///   * `4.3` — the one A/C site the sweep leaves alone (`raw_tx_bytes`);
///   * `classifier` — the classifiers' own fail-closed defaults (§4.1);
///   * `pattern` — a match PATTERN on the variant, not a construction;
///   * `definition` — the variant's own declaration.
const ALLOWLIST: &[(&str, &str, usize, &str, &str)] = &[
    // ── account.rs ──────────────────────────────────────────────────────────
    (
        "account.rs",
        "account_default_address",
        2,
        "B-inv",
        "an id get_account_ids returned resolves; a derived account carries a UFVK",
    ),
    (
        "account.rs",
        "account_transparent_address",
        2,
        "B-inv",
        "the primary account resolves and carries a UFVK",
    ),
    (
        "account.rs",
        "account_transparent_receiver",
        2,
        "B-inv",
        "the primary account resolves and carries a UFVK",
    ),
    (
        "account.rs",
        "account_ufvk_encoding",
        2,
        "B-inv",
        "the primary account resolves and carries a UFVK",
    ),
    (
        "account.rs",
        "primary_account_default_index",
        2,
        "B-inv",
        "the primary account resolves and carries a UFVK",
    ),
    (
        "account.rs",
        "checked_add_zat",
        1,
        "B",
        "a balance sum over the stored notes past MAX_MONEY",
    ),
    (
        "account.rs",
        "to_sdk_zat",
        2,
        "B",
        "a stored amount beyond the supply",
    ),
    // ── block_cache.rs ──────────────────────────────────────────────────────
    (
        "block_cache.rs",
        "BlockCache::insert",
        1,
        "D",
        "a server block's height does not fit a u32 — endpoint data, not the store; R15",
    ),
    (
        "block_cache.rs",
        "decode_block",
        1,
        "B",
        "a cached block that does not decode",
    ),
    (
        "block_cache.rs",
        "row_block_bytes",
        1,
        "B",
        "a cached BLOB over the gRPC cap (size-cap-before-alloc)",
    ),
    // ── db.rs ───────────────────────────────────────────────────────────────
    (
        "db.rs",
        "assert_columns_match",
        1,
        "B-inv",
        "the rescan aux copy's column signatures differ",
    ),
    (
        "db.rs",
        "classify_sqlite_error",
        3,
        "classifier",
        "the SSOT: the two corruption-class IOERR codes and its fail-closed defaults",
    ),
    (
        "db.rs",
        "copy_aux_tables",
        1,
        "D",
        "a non-UTF-8 db_dir (host config, not store state) cannot form the ATTACH URI; honest kind owed (ledger R15)",
    ),
    (
        "db.rs",
        "ensure_rollback_journal",
        1,
        "B-inv",
        "the cache echoed a journal mode it cannot run",
    ),
    (
        "db.rs",
        "ensure_synchronous_full",
        1,
        "B-inv",
        "the store cannot honour synchronous=FULL",
    ),
    (
        "db.rs",
        "ensure_wal",
        1,
        "B-inv",
        "the store echoed a journal mode that is neither wal nor delete",
    ),
    (
        "db.rs",
        "classify_migration_error",
        3,
        "classifier",
        "§4.1: the migration unwrap's documented defaults (Io non-full, Other's `_`, the outer `_`), shared by the first try and the SeedRequired retry",
    ),
    (
        "db.rs",
        "open_db",
        1,
        "B",
        "the provisioning sentinel names a schema version this build does not run",
    ),
    (
        "db.rs",
        "verify_wal_folded",
        1,
        "B-inv",
        "a non-empty -wal survived the fold the rename depends on",
    ),
    // ── delivery.rs ─────────────────────────────────────────────────────────
    (
        "delivery.rs",
        "enumerate",
        1,
        "B",
        "a stored txid blob that is not 32 bytes",
    ),
    (
        "delivery.rs",
        "spenders_of",
        2,
        "B",
        "a stored txid blob / pool tag that does not decode",
    ),
    // ── derivation.rs ───────────────────────────────────────────────────────
    (
        "derivation.rs",
        "transparent_receiver_from_stored_address",
        2,
        "B",
        "a stored address that does not decode to a transparent receiver",
    ),
    // ── diversified_index.rs ────────────────────────────────────────────────
    (
        "diversified_index.rs",
        "reserve_next_index",
        1,
        "B-inv",
        "the stored counter is out of range",
    ),
    // ── error.rs ────────────────────────────────────────────────────────────
    (
        "error.rs",
        "<enum WalletError>",
        1,
        "definition",
        "the variant's declaration",
    ),
    (
        "error.rs",
        "WalletError::code",
        1,
        "pattern",
        "the RW- code arm",
    ),
    // ── history.rs ──────────────────────────────────────────────────────────
    (
        "history.rs",
        "HistoryCursor::of",
        1,
        "B",
        "a stored txid blob that is not 32 bytes",
    ),
    (
        "history.rs",
        "HistoryCursor::decode",
        6,
        "D",
        "a HOST-supplied cursor (it crosses the FFI) that does not decode — not store corruption; honest kind owed (ledger R15)",
    ),
    (
        "history.rs",
        "IncomingCursor::decode",
        4,
        "D",
        "a HOST-supplied cursor (it crosses the FFI) that does not decode — not store corruption; honest kind owed (ledger R15)",
    ),
    (
        "history.rs",
        "map_row",
        1,
        "B",
        "a stored txid/height that does not decode",
    ),
    // ── intent_store.rs ─────────────────────────────────────────────────────
    (
        "intent_store.rs",
        "IntentState::from_i64",
        1,
        "B",
        "a stored state integer outside the enum",
    ),
    (
        "intent_store.rs",
        "decode_claims",
        3,
        "B",
        "a stored claim blob that does not decode",
    ),
    (
        "intent_store.rs",
        "decode_txids",
        2,
        "B",
        "a stored txid list that does not decode",
    ),
    (
        "intent_store.rs",
        "list_in_flight",
        5,
        "B-inv",
        "a row whose columns contradict its state",
    ),
    (
        "intent_store.rs",
        "list_sent_multi_step",
        4,
        "B-inv",
        "a row whose columns contradict its state",
    ),
    (
        "intent_store.rs",
        "list_stranded_intents",
        2,
        "B-inv",
        "a stranded row without its txids",
    ),
    (
        "intent_store.rs",
        "mark_sent_multi",
        1,
        "B-inv",
        "a transition the stored state forbids",
    ),
    // ── issued_quote_store.rs ───────────────────────────────────────────────
    (
        "issued_quote_store.rs",
        "check_row",
        1,
        "B-inv",
        "a stored quote row that fails its own checks",
    ),
    (
        "issued_quote_store.rs",
        "persist",
        4,
        "B-inv",
        "a stored quote row that contradicts the one being persisted",
    ),
    // ── parked.rs ───────────────────────────────────────────────────────────
    (
        "parked.rs",
        "classify",
        1,
        "B",
        "a zip321 re-parse of our own stored URI",
    ),
    (
        "parked.rs",
        "classify_in_flight",
        1,
        "B",
        "a zip321 re-parse of our own stored URI",
    ),
    // ── provision.rs ────────────────────────────────────────────────────────
    (
        "provision.rs",
        "resolve_birthday",
        2,
        "B",
        "the compiled-in checkpoint does not decode",
    ),
    (
        "provision.rs",
        "resolve_offline_birthday",
        2,
        "B",
        "the compiled-in checkpoint does not decode",
    ),
    (
        "provision.rs",
        "resolve_watch_only_birthday",
        2,
        "B",
        "the compiled-in checkpoint does not decode",
    ),
    // ── refund_index.rs ─────────────────────────────────────────────────────
    (
        "refund_index.rs",
        "backfill_bounds",
        1,
        "B-inv",
        "the stored counter is out of range",
    ),
    (
        "refund_index.rs",
        "next_index_snapshot",
        1,
        "B-inv",
        "the stored counter is out of range",
    ),
    (
        "refund_index.rs",
        "reserve_next_index",
        1,
        "B-inv",
        "the stored counter is out of range",
    ),
    (
        "refund_index.rs",
        "set_registered_up_to",
        1,
        "B-inv",
        "the registration marker would pass the counter",
    ),
    (
        "refund_index.rs",
        "widen_for_deep_scan",
        2,
        "B-inv",
        "the stored counter is out of range",
    ),
    // ── root_bind.rs ────────────────────────────────────────────────────────
    (
        "root_bind.rs",
        "apply_boundary_correction",
        1,
        "B",
        "a stored shard index that does not fit a u64",
    ),
    (
        "root_bind.rs",
        "clear_boundary_bounds_in",
        2,
        "B",
        "a range bound that does not fit SQLite's signed integer (fail closed)",
    ),
    (
        "root_bind.rs",
        "read_block_tree_size",
        1,
        "B",
        "a stored tree size that does not fit a u64",
    ),
    (
        "root_bind.rs",
        "read_scanned_sizes",
        2,
        "B",
        "a stored tree size that does not fit, or is out of order",
    ),
    (
        "root_bind.rs",
        "read_scanned_tree_size",
        1,
        "B",
        "a stored tree size that does not fit a u64",
    ),
    (
        "root_bind.rs",
        "reconcile_scanned_boundaries",
        1,
        "B",
        "a stored shard index that does not fit a u64",
    ),
    (
        "root_bind.rs",
        "record_boundary_bounds",
        1,
        "B",
        "a shard index that does not fit a u64",
    ),
    (
        "root_bind.rs",
        "withdraw_suffix",
        1,
        "B",
        "a shard index that does not fit a u64",
    ),
    // ── send.rs ─────────────────────────────────────────────────────────────
    (
        "send.rs",
        "<() as ClassifyStoreFault>::into_store_fault",
        1,
        "classifier",
        "§4.1: the synthetic `()` backend has no SQLite store and keeps its StoreCorrupt impl",
    ),
    (
        "send.rs",
        "create_two_step_enrolled",
        2,
        "B-inv",
        "the enrolled two-step's stored row contradicts the group",
    ),
    (
        "send.rs",
        "drain_multi",
        1,
        "B-inv",
        "a group member the store just held is gone",
    ),
    (
        "send.rs",
        "proto_from_tag",
        1,
        "B",
        "a stored protocol tag byte outside the enum",
    ),
    (
        "send.rs",
        "read_raw_tx",
        1,
        "B",
        "re-serialising a decoded tx into a Vec",
    ),
    (
        "send.rs",
        "rebroadcast_group",
        1,
        "B-inv",
        "an empty stored group",
    ),
    // ── store.rs ────────────────────────────────────────────────────────────
    (
        "store.rs",
        "ManifestSeedSource::from_byte",
        1,
        "B",
        "a manifest byte outside the enum",
    ),
    (
        "store.rs",
        "PersistenceKind::from_byte",
        1,
        "B",
        "a manifest byte outside the enum",
    ),
    (
        "store.rs",
        "inspect",
        1,
        "B-inv",
        "a store state the manifest forbids",
    ),
    (
        "store.rs",
        "read_manifest",
        7,
        "B",
        "a manifest that does not decode",
    ),
    (
        "store.rs",
        "read_seal_file",
        2,
        "B",
        "a seal file that does not decode",
    ),
    (
        "store.rs",
        "repair_resolving",
        1,
        "B-inv",
        "a sealed-seed manifest without its seal",
    ),
    (
        "store.rs",
        "resolve_custody",
        1,
        "pattern",
        "an `Err(StoreCorrupt)` arm",
    ),
    // ── sync.rs ─────────────────────────────────────────────────────────────
    (
        "sync.rs",
        "<SqliteClientError as ClassifyStoreFault>::into_store_fault",
        1,
        "classifier",
        "§4.1: the fail-closed default (decoding, gap-limit, tree logic)",
    ),
    (
        "sync.rs",
        "<Error as ClassifyStoreFault>::into_store_fault",
        1,
        "classifier",
        "§4.1: the fail-closed default (CheckpointConflict, SubtreeDiscontinuity)",
    ),
    (
        "sync.rs",
        "engine_io_fault",
        1,
        "classifier",
        "§4.1: the engine's Io / Serialization decode rule — every kind but StorageFull is a decode of bytes SQLite returned",
    ),
    (
        "sync.rs",
        "map_shardtree_err",
        2,
        "B",
        "a shard-tree Insert/Query logic fault (documented there)",
    ),
    // ── sync_controller.rs ──────────────────────────────────────────────────
    (
        "sync_controller.rs",
        "stall_for",
        1,
        "pattern",
        "R10: StoreCorrupt renders Internal",
    ),
    // ── transparent.rs ──────────────────────────────────────────────────────
    (
        "transparent.rs",
        "read_stored_unspent",
        1,
        "B",
        "a stored txid blob that is not 32 bytes",
    ),
    // ── wallet.rs ───────────────────────────────────────────────────────────
    (
        "wallet.rs",
        "Wallet::backfill_swap_address_registrations",
        1,
        "B-inv",
        "the stored registration marker passes the counter",
    ),
    (
        "wallet.rs",
        "Wallet::raw_tx_bytes",
        3,
        "4.3",
        "the post-persist get_transaction read (§4.3: StoreBusy would say 'wrote nothing' over a persisted tx; R13), its miss, and the re-serialise",
    ),
    (
        "wallet.rs",
        "Wallet::reclaim_ephemeral_slots_with",
        1,
        "pattern",
        "the reclaim's hard-fault list",
    ),
    (
        "wallet.rs",
        "Wallet::rescan_inner",
        2,
        "D",
        "a poisoned mutex is not corruption — honest kind owed, ledger R15 (LOW)",
    ),
    (
        "wallet.rs",
        "Wallet::sweep_one",
        1,
        "pattern",
        "the sweep's hard-fault list",
    ),
    (
        "wallet.rs",
        "Wallet::switch_sync_server_inner",
        3,
        "D",
        "a poisoned mutex, and a failed close on the server switch — not corruption, honest kind owed, ledger R15 (LOW)",
    ),
    (
        "wallet.rs",
        "Wallet::try_sign_queued_intent",
        1,
        "B",
        "a zip321 re-parse of our own stored URI",
    ),
    (
        "wallet.rs",
        "issued_record_from_stored",
        10,
        "B",
        "range checks and decode arms over a stored quote row",
    ),
    (
        "wallet.rs",
        "persist_issued_quote_blocking",
        1,
        "B",
        "a reserved index that does not fit a u32",
    ),
];

/// Fns that return a bare `WalletError`, name `StoreCorrupt`, and decide nothing
/// — the mapper shape the deny list refuses — kept on purpose. (file, key, reason.)
const MAPPER_EXCEPTIONS: &[(&str, &str, &str)] = &[(
    "send.rs",
    "<() as ClassifyStoreFault>::into_store_fault",
    "§4.1: the synthetic `()` test backend has no SQLite store; it is never a production DbT",
)];

/// A floor on the files the walk reaches (anti-vacuity: a resolver that stops at
/// `lib.rs` would pass with nothing scanned). Measured at the base: 90+.
const FILE_FLOOR: usize = 60;

/// The site the scan must see in the real tree, or it is not reading what it
/// thinks it is: §4.3's one deliberate exception.
const CANARY: (&str, &str) = ("wallet.rs", "Wallet::raw_tx_bytes");

/// R13 §4.1 — the one allowlist entry that is a FLOOR as well as a ceiling: a
/// FALL here fails instead of printing. The UI's
/// `singleStepSendErrorPrecedesPersistence` (`send_state.dart`) keeps
/// `DiskFull`, `StoreBusy` and `Io` as "nothing was saved" on the shield/move
/// path ONLY because `broadcast_persisted`'s post-persist `raw_tx_bytes` read
/// stays an unclassified `StoreCorrupt`. Classifying it makes those kinds
/// reachable after persistence, so the Dart list must drop them in the same
/// commit — then lower this entry.
const FLOOR: (&str, &str) = CANARY;

fn src_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

// ── The lexer ────────────────────────────────────────────────────────────────

fn is_ident_byte(c: u8) -> bool {
    c == b'_' || c.is_ascii_alphanumeric() || c >= 0x80
}

/// The source with every comment, string, byte string, raw string and char
/// literal BLANKED to spaces (newlines kept, every byte offset preserved — so a
/// `#[path = "…"]` value can be read back from the original at the same span).
/// Delimiting quotes are kept; their contents are not. A lifetime or label
/// (`'a`) is left alone: a `'` begins a char literal only when it is followed by
/// an escape, or by exactly one character and a closing `'`.
fn blank(src: &str) -> String {
    let b = src.as_bytes();
    let mut out = b.to_vec();
    let n = b.len();
    let wipe = |out: &mut Vec<u8>, from: usize, to: usize| {
        for byte in out.iter_mut().take(to.min(n)).skip(from) {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
    };
    let mut i = 0;
    while i < n {
        let c = b[i];
        // Line comment (doc comments included).
        if c == b'/' && b.get(i + 1) == Some(&b'/') {
            let mut j = i;
            while j < n && b[j] != b'\n' {
                j += 1;
            }
            wipe(&mut out, i, j);
            i = j;
            continue;
        }
        // Block comment, nested.
        if c == b'/' && b.get(i + 1) == Some(&b'*') {
            let mut depth = 0usize;
            let mut j = i;
            while j < n {
                if b[j] == b'/' && b.get(j + 1) == Some(&b'*') {
                    depth += 1;
                    j += 2;
                } else if b[j] == b'*' && b.get(j + 1) == Some(&b'/') {
                    depth -= 1;
                    j += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    j += 1;
                }
            }
            wipe(&mut out, i, j);
            i = j;
            continue;
        }
        // Raw string: `r#*"` where the `r` begins a token (or follows a `b`/`c`
        // prefix that does). `r#ident` (a raw identifier) has no quote and is code.
        if c == b'r' {
            let prev = if i > 0 { b[i - 1] } else { b' ' };
            let starts_token = !is_ident_byte(prev)
                || ((prev == b'b' || prev == b'c') && (i < 2 || !is_ident_byte(b[i - 2])));
            if starts_token {
                let mut j = i + 1;
                let mut hashes = 0usize;
                while j < n && b[j] == b'#' {
                    hashes += 1;
                    j += 1;
                }
                if j < n && b[j] == b'"' {
                    let mut k = j + 1;
                    let end = loop {
                        if k >= n {
                            break n;
                        }
                        if b[k] == b'"'
                            && b[k + 1..]
                                .iter()
                                .take(hashes)
                                .filter(|h| **h == b'#')
                                .count()
                                == hashes
                        {
                            break k;
                        }
                        k += 1;
                    };
                    wipe(&mut out, j + 1, end);
                    i = (end + 1 + hashes).min(n);
                    continue;
                }
            }
        }
        // String (and byte/C string — the prefix letter stays, it is harmless).
        if c == b'"' {
            let mut j = i + 1;
            while j < n && b[j] != b'"' {
                if b[j] == b'\\' {
                    j += 1;
                }
                j += 1;
            }
            wipe(&mut out, i + 1, j);
            i = j + 1;
            continue;
        }
        // Char literal vs lifetime.
        if c == b'\'' {
            if b.get(i + 1) == Some(&b'\\') {
                let mut j = i + 2;
                // `'\''` — the escaped quote is the char, not the close.
                if b.get(j) == Some(&b'\'') {
                    j += 1;
                }
                while j < n && b[j] != b'\'' {
                    j += 1;
                }
                wipe(&mut out, i + 1, j);
                i = j + 1;
                continue;
            }
            if let Some(&lead) = b.get(i + 1) {
                let width = match lead {
                    0xF0..=0xFF => 4,
                    0xE0..=0xEF => 3,
                    0xC0..=0xDF => 2,
                    _ => 1,
                };
                if lead != b'\'' && b.get(i + 1 + width) == Some(&b'\'') {
                    wipe(&mut out, i + 1, i + 1 + width);
                    i += 2 + width;
                    continue;
                }
            }
            i += 1;
            continue;
        }
        i += 1;
    }
    // Every replacement wrote ASCII over whole UTF-8 sequences, so this holds.
    String::from_utf8(out).expect("blanking writes whole characters")
}

#[derive(Clone, Debug)]
struct Tok {
    text: String,
    start: usize,
    line: usize,
}

/// Tokens of a blanked text: identifiers (and numbers), `::`, `->`, `=>`, and
/// single punctuation characters. Whitespace collapses away, so a wrapped
/// expression is one token stream wherever rustfmt broke it.
fn tokens(text: &str) -> Vec<Tok> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1;
    while i < b.len() {
        let c = b[i];
        if c == b'\n' {
            line += 1;
            i += 1;
            continue;
        }
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if is_ident_byte(c) {
            let start = i;
            while i < b.len() && is_ident_byte(b[i]) {
                i += 1;
            }
            out.push(Tok {
                text: text[start..i].to_string(),
                start,
                line,
            });
            continue;
        }
        let two = &b[i..(i + 2).min(b.len())];
        let width = if two == b"::" || two == b"->" || two == b"=>" {
            2
        } else {
            1
        };
        out.push(Tok {
            text: text[i..i + width].to_string(),
            start: i,
            line,
        });
        i += width;
    }
    out
}

// ── Test-code exclusion (item-level `cfg(test)`) ─────────────────────────────

/// The index just past the attribute that opens at `i` (`#[` or `#![`).
fn attr_end(t: &[Tok], i: usize) -> usize {
    let mut j = i + 1;
    if t.get(j).is_some_and(|x| x.text == "!") {
        j += 1;
    }
    let mut depth = 0i32;
    while j < t.len() {
        match t[j].text.as_str() {
            "[" => depth += 1,
            "]" => {
                depth -= 1;
                if depth == 0 {
                    return j + 1;
                }
            }
            _ => {}
        }
        j += 1;
    }
    j
}

/// Is the attribute `t[i..end]` one that compiles its item ONLY under test?
/// `cfg(test)` and `cfg(all(…, test, …))` are; `cfg(any(test, …))`,
/// `cfg(not(test))` and every `cfg_attr` are not.
fn is_test_only_cfg(t: &[Tok], i: usize, end: usize) -> bool {
    let words: Vec<&str> = t[i..end]
        .iter()
        .map(|x| x.text.as_str())
        .filter(|w| *w != "#" && *w != "!")
        .collect();
    // `[ cfg ( … ) ]`
    if words.len() < 5 || words[1] != "cfg" || words[2] != "(" {
        return false;
    }
    let inner = &words[3..words.len() - 2];
    if inner == ["test"] {
        return true;
    }
    if inner.first() == Some(&"all") && inner.get(1) == Some(&"(") {
        // Top-level arguments of `all( … )`.
        let mut depth = 0i32;
        let mut arg: Vec<&str> = Vec::new();
        for w in &inner[2..inner.len().saturating_sub(1)] {
            match *w {
                "(" => depth += 1,
                ")" => depth -= 1,
                "," if depth == 0 => {
                    if arg == ["test"] {
                        return true;
                    }
                    arg.clear();
                    continue;
                }
                _ => {}
            }
            arg.push(w);
        }
        return arg == ["test"];
    }
    false
}

/// Items — and block-like expression statements (`#[cfg(test)] if … { … }`,
/// which `net/tor_posture.rs` carries) — that end at their closing brace.
const BRACE_ITEMS: &[&str] = &[
    "fn",
    "mod",
    "impl",
    "trait",
    "struct",
    "enum",
    "union",
    "macro_rules",
    "if",
    "match",
    "loop",
    "while",
    "for",
];
const ITEM_PREFIX: &[&str] = &[
    "pub", "crate", "super", "in", "self", "unsafe", "async", "const", "extern", "default", "(",
    ")", "\"",
];

/// The index just past the item that begins at `i` (after its attribute). A
/// brace item (fn, mod, impl, …) ends at its closing `}` (or a `;` for `mod x;`,
/// a bodiless fn, a tuple struct); a statement, field, variant or arm ends at
/// the first `;` or `,` at its own depth; nothing ever runs past the close of
/// the block that encloses it.
fn item_end(t: &[Tok], mut i: usize) -> usize {
    while t.get(i).is_some_and(|x| x.text == "#") {
        i = attr_end(t, i);
    }
    let first = t[i..]
        .iter()
        .map(|x| x.text.as_str())
        .find(|w| !ITEM_PREFIX.contains(w))
        .unwrap_or("");
    let brace_item = BRACE_ITEMS.contains(&first) || t.get(i).is_some_and(|x| x.text == "{");
    let mut depth = 0i32;
    let mut j = i;
    while j < t.len() {
        match t[j].text.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => {
                depth -= 1;
                if depth < 0 {
                    return j;
                }
                // An `if … { } else { }` chain runs on through its `else`.
                if depth == 0
                    && t[j].text == "}"
                    && brace_item
                    && t.get(j + 1).is_none_or(|x| x.text != "else")
                {
                    return j + 1;
                }
            }
            ";" if depth == 0 => return j + 1,
            "," if depth == 0 && !brace_item => return j + 1,
            _ => {}
        }
        j += 1;
    }
    j
}

/// The index of the `}` that closes the block enclosing `i`.
fn block_rest(t: &[Tok], i: usize) -> usize {
    let mut depth = 0i32;
    for (j, x) in t.iter().enumerate().skip(i) {
        match x.text.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => {
                depth -= 1;
                if depth < 0 {
                    return j;
                }
            }
            _ => {}
        }
    }
    t.len()
}

/// Per-token "is test code" flags: every `cfg(test)` ITEM's span, and the whole
/// file for an inner `#![cfg(test)]` at file scope.
fn test_mask(t: &[Tok]) -> Vec<bool> {
    let mut mask = vec![false; t.len()];
    let mut i = 0;
    let mut depth = 0i32;
    while i < t.len() {
        match t[i].text.as_str() {
            "{" => depth += 1,
            "}" => depth -= 1,
            "#" if t.get(i + 1).is_some_and(|x| x.text == "[" || x.text == "!") => {
                let end = attr_end(t, i);
                if is_test_only_cfg(t, i, end) {
                    let inner = t[i + 1].text == "!";
                    let span_end = if inner && depth == 0 {
                        t.len()
                    } else if inner {
                        // `#![cfg(test)]` inside a block: the rest of that block.
                        block_rest(t, end)
                    } else {
                        item_end(t, end)
                    };
                    for m in mask.iter_mut().take(span_end).skip(i) {
                        *m = true;
                    }
                    i = span_end;
                    continue;
                }
                i = end;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    mask
}

// ── The structural walk ──────────────────────────────────────────────────────

fn is_ident(w: &str) -> bool {
    w.as_bytes()
        .first()
        .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
}

/// Is the token before `i` one an ITEM can follow (so `impl`/`mod`/`use` at `i`
/// is an item, not `-> impl Trait` or a path segment)?
fn at_item_position(t: &[Tok], i: usize) -> bool {
    match i.checked_sub(1).map(|p| t[p].text.as_str()) {
        None => true,
        Some(p) => matches!(
            p,
            "}" | ";" | "{" | "]" | "pub" | ")" | "unsafe" | "default"
        ),
    }
}

/// `impl<…> Trait<…> for Type<…> where …` → `<Type as Trait>`; `impl Type` → `Type`.
fn impl_name(header: &[&str]) -> String {
    let mut words: Vec<&str> = Vec::new();
    let mut depth = 0i32;
    for w in header {
        match *w {
            "<" => depth += 1,
            ">" => depth = (depth - 1).max(0),
            "where" if depth == 0 => break,
            _ if depth == 0 => words.push(w),
            _ => {}
        }
    }
    let last = |xs: &[&str]| -> String {
        let ids: Vec<&&str> = xs
            .iter()
            .filter(|w| is_ident(w) && !matches!(**w, "dyn" | "impl" | "unsafe" | "mut" | "const"))
            .collect();
        match ids.last() {
            Some(w) => (**w).to_string(),
            None => xs.concat(),
        }
    };
    match words.iter().position(|w| *w == "for") {
        Some(k) => format!("<{} as {}>", last(&words[k + 1..]), last(&words[..k])),
        None => last(&words),
    }
}

#[derive(Clone, Debug)]
enum FrameKind {
    Fn(FnInfo),
    Impl {
        name: String,
        from_for_wallet_error: bool,
    },
    Named(String),
    Other,
}

#[derive(Clone, Debug)]
struct FnInfo {
    name: String,
    is_const: bool,
    generic: bool,
    ignored_arg: bool,
    returns_bare_wallet_error: bool,
}

#[derive(Clone, Debug)]
struct Frame {
    kind: FrameKind,
    body_start: usize,
}

/// What one file holds: production `StoreCorrupt` counts by enclosing-fn key
/// (with the lines, for the failure message), the deny-list violations, and the
/// `mod x;` declarations the module walk follows.
#[derive(Default, Debug)]
struct FileScan {
    counts: BTreeMap<String, Vec<usize>>,
    denied: Vec<String>,
    /// (declared module name or `#[path]` value, is a `#[path]`, inline module components)
    mods: Vec<(String, bool, Vec<String>)>,
}

const SITE: &str = "StoreCorrupt";

fn key_of(stack: &[Frame]) -> String {
    let fns: Vec<&str> = stack
        .iter()
        .filter_map(|f| match &f.kind {
            FrameKind::Fn(info) => Some(info.name.as_str()),
            _ => None,
        })
        .collect();
    let owner = stack.iter().rev().find_map(|f| match &f.kind {
        FrameKind::Impl { name, .. } => Some(name.clone()),
        FrameKind::Named(name) => Some(name.clone()),
        _ => None,
    });
    if fns.is_empty() {
        return format!("<{}>", owner.unwrap_or_else(|| "file scope".into()));
    }
    // The impl/trait that owns the OUTERMOST fn names the type.
    let first_fn = stack
        .iter()
        .position(|f| matches!(f.kind, FrameKind::Fn(_)))
        .unwrap_or(0);
    let owner = stack[..first_fn].iter().rev().find_map(|f| match &f.kind {
        FrameKind::Impl { name, .. } => Some(name.clone()),
        FrameKind::Named(name) if name.starts_with("trait ") => {
            Some(name.trim_start_matches("trait ").to_string())
        }
        _ => None,
    });
    match owner {
        Some(o) => format!("{o}::{}", fns.join("::")),
        None => fns.join("::"),
    }
}

/// The tokens of the group that opens at `t[i]` (`(`, `[`, `{`), and the index
/// just past its close.
fn group(t: &[Tok], i: usize) -> (Vec<&str>, usize) {
    let mut depth = 0i32;
    let mut j = i;
    let mut words = Vec::new();
    while j < t.len() {
        let w = t[j].text.as_str();
        match w {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth -= 1,
            _ => {}
        }
        words.push(w);
        j += 1;
        if depth == 0 {
            break;
        }
    }
    (words, j)
}

/// Tokens from `i` to the `;` that ends the statement at its own depth (inclusive).
fn statement(t: &[Tok], i: usize) -> (Vec<&str>, usize) {
    let mut depth = 0i32;
    let mut j = i;
    let mut words = Vec::new();
    while j < t.len() {
        let w = t[j].text.as_str();
        match w {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => {
                depth -= 1;
                if depth < 0 {
                    break;
                }
            }
            ";" if depth == 0 => {
                words.push(w);
                j += 1;
                break;
            }
            _ => {}
        }
        words.push(w);
        j += 1;
    }
    (words, j)
}

fn uses_glob_of_wallet_error(words: &[&str]) -> bool {
    words.windows(3).enumerate().any(|(k, w)| {
        if w[0] != "WalletError" || w[1] != "::" {
            return false;
        }
        if w[2] == "*" {
            return true;
        }
        if w[2] == "{" {
            // A `*` at the top level of the braced list.
            let mut depth = 0i32;
            for x in &words[k + 2..] {
                match *x {
                    "{" => depth += 1,
                    "}" => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    "*" if depth == 1 => return true,
                    _ => {}
                }
            }
        }
        false
    })
}

fn scan_text(src: &str, file: &str) -> FileScan {
    let blanked = blank(src);
    let t = tokens(&blanked);
    let mask = test_mask(&t);
    let mut out = FileScan::default();
    let mut stack: Vec<Frame> = Vec::new();
    let mut pending: Option<FrameKind> = None;
    let mut paren = 0i32;
    let mut path_attr: Option<String> = None;
    let mut i = 0;
    while i < t.len() {
        let w = t[i].text.as_str();
        let prod = !mask[i];
        // `#[path = "…"]` — read the value back from the ORIGINAL at the blanked span.
        if w == "#" && t.get(i + 1).is_some_and(|x| x.text == "[") {
            let end = attr_end(&t, i);
            if t.get(i + 2).is_some_and(|x| x.text == "path") {
                let quotes: Vec<&Tok> = t[i..end].iter().filter(|x| x.text == "\"").collect();
                if let [open, close] = quotes.as_slice() {
                    path_attr = Some(src[open.start + 1..close.start].to_string());
                }
            }
            i = end;
            continue;
        }
        match w {
            "fn" if t.get(i + 1).is_some_and(|x| is_ident(&x.text)) => {
                let name = t[i + 1].text.clone();
                let is_const =
                    (i > 0 && t[i - 1].text == "const") || (i > 1 && t[i - 2].text == "const");
                let generic = t.get(i + 2).is_some_and(|x| x.text == "<");
                // The parameter list and the return type, up to the body.
                let mut j = i + 2;
                if generic {
                    let mut depth = 0i32;
                    while j < t.len() {
                        match t[j].text.as_str() {
                            "<" => depth += 1,
                            ">" => {
                                depth -= 1;
                                if depth == 0 {
                                    j += 1;
                                    break;
                                }
                            }
                            _ => {}
                        }
                        j += 1;
                    }
                }
                let (params, after) = if t.get(j).is_some_and(|x| x.text == "(") {
                    group(&t, j)
                } else {
                    (Vec::new(), j)
                };
                // An ignored argument: a parameter pattern that is `_` or `_name`.
                let ignored_arg = params
                    .windows(2)
                    .any(|p| (p[0] == "(" || p[0] == ",") && p[1].starts_with('_'))
                    && params
                        .windows(3)
                        .any(|p| p[1].starts_with('_') && p[2] == ":");
                let mut ret: Vec<&str> = Vec::new();
                if t.get(after).is_some_and(|x| x.text == "->") {
                    let mut k = after + 1;
                    while k < t.len() && !matches!(t[k].text.as_str(), "{" | ";" | "where") {
                        ret.push(t[k].text.as_str());
                        k += 1;
                    }
                }
                let in_wallet_error_impl = stack.iter().rev().any(|f| match &f.kind {
                    FrameKind::Impl { name, .. } => {
                        name == "WalletError" || name.starts_with("<WalletError as")
                    }
                    _ => false,
                });
                let returns_bare_wallet_error = (ret.last() == Some(&"WalletError")
                    && ret
                        .iter()
                        .all(|x| matches!(*x, "WalletError" | "::" | "crate" | "error" | "super")))
                    || (ret == ["Self"] && in_wallet_error_impl);
                pending = Some(FrameKind::Fn(FnInfo {
                    name,
                    is_const,
                    generic,
                    ignored_arg,
                    returns_bare_wallet_error,
                }));
                paren = 0;
            }
            "impl" if at_item_position(&t, i) => {
                let mut header: Vec<&str> = Vec::new();
                let mut depth = 0i32;
                let mut j = i + 1;
                while j < t.len() {
                    let x = t[j].text.as_str();
                    match x {
                        "(" | "[" => depth += 1,
                        ")" | "]" => depth -= 1,
                        "{" | ";" if depth == 0 => break,
                        _ => {}
                    }
                    header.push(x);
                    j += 1;
                }
                let name = impl_name(&header);
                let from_for_wallet_error =
                    header.contains(&"From") && name.starts_with("<WalletError as");
                pending = Some(FrameKind::Impl {
                    name,
                    from_for_wallet_error,
                });
                paren = 0;
            }
            "mod" | "trait" | "enum" | "struct" | "union"
                if at_item_position(&t, i) && t.get(i + 1).is_some_and(|x| is_ident(&x.text)) =>
            {
                let name = t[i + 1].text.clone();
                // A `#[path]` belongs to the declaration it precedes, test or not —
                // taken here so a gated one can never leak onto the next `mod`.
                let declared_path = if w == "mod" { path_attr.take() } else { None };
                if w == "mod" && t.get(i + 2).is_some_and(|x| x.text == ";") && prod {
                    let components = stack
                        .iter()
                        .filter_map(|f| match &f.kind {
                            FrameKind::Named(n) => n.strip_prefix("mod ").map(str::to_string),
                            _ => None,
                        })
                        .collect();
                    match declared_path {
                        Some(p) => out.mods.push((p, true, components)),
                        None => out.mods.push((name.clone(), false, components)),
                    }
                }
                pending = Some(FrameKind::Named(format!("{w} {name}")));
                paren = 0;
            }
            "use" if prod && at_item_position(&t, i) => {
                let (words, _) = statement(&t, i);
                if words.contains(&SITE) {
                    out.denied.push(format!(
                        "{file}:{}: a `use` imports `StoreCorrupt` — an alias or bare import hides \
                         every site the count is keyed on; write `WalletError::StoreCorrupt`",
                        t[i].line
                    ));
                }
                if uses_glob_of_wallet_error(&words) {
                    out.denied.push(format!(
                        "{file}:{}: a `use` globs `WalletError::*` — name the variants you use",
                        t[i].line
                    ));
                }
            }
            "macro_rules" if prod && t.get(i + 1).is_some_and(|x| x.text == "!") => {
                let open =
                    (i + 2..t.len()).find(|k| matches!(t[*k].text.as_str(), "{" | "(" | "["));
                if let Some(open) = open {
                    let (body, end) = group(&t, open);
                    if body.contains(&SITE) {
                        out.denied.push(format!(
                            "{file}:{}: a `macro_rules!` expands to `StoreCorrupt` — its call sites \
                             carry no token for the count to see",
                            t[i].line
                        ));
                    }
                    // A macro body is not code of the enclosing fn; skip it whole.
                    i = end;
                    continue;
                }
            }
            "const" | "static"
                if prod
                    && at_item_position(&t, i)
                    && t.get(i + 1)
                        .is_some_and(|x| x.text != "fn" && x.text != "unsafe") =>
            {
                let (words, _) = statement(&t, i);
                if words.contains(&SITE) {
                    out.denied.push(format!(
                        "{file}:{}: a `{w}` item holds `StoreCorrupt` — its uses carry no token \
                         for the count to see",
                        t[i].line
                    ));
                }
            }
            "let" if prod => {
                // `let NAME = [move] |_…| … StoreCorrupt …;` — a blind closure kept to pass around.
                let (words, _) = statement(&t, i);
                if let Some(bar) = words.iter().position(|x| *x == "|") {
                    let before_ok = words[..bar].contains(&"=")
                        && words[..bar]
                            .iter()
                            .skip_while(|x| **x != "=")
                            .skip(1)
                            .all(|x| *x == "move");
                    // Each parameter's PATTERN (the tokens before its `:` type).
                    let params: Vec<&str> = words[bar + 1..]
                        .iter()
                        .take_while(|x| **x != "|")
                        .copied()
                        .collect();
                    let patterns: Vec<Vec<&str>> = params
                        .split(|x| *x == ",")
                        .map(|p| p.iter().take_while(|x| **x != ":").copied().collect())
                        .collect();
                    let blind = !params.is_empty()
                        && patterns
                            .iter()
                            .all(|p| p.iter().filter(|x| is_ident(x)).all(|x| x.starts_with('_')));
                    if before_ok && blind && words.contains(&SITE) {
                        out.denied.push(format!(
                            "{file}:{}: a `let`-bound closure ignores its error and yields \
                             `StoreCorrupt` — every `.map_err(it)` is a blind site the count \
                             cannot see",
                            t[i].line
                        ));
                    }
                }
            }
            "(" | "[" => paren += 1,
            ")" | "]" => paren -= 1,
            ";" if paren <= 0 => pending = None,
            "{" => {
                let kind = match pending.take() {
                    Some(k) if paren <= 0 => k,
                    _ => FrameKind::Other,
                };
                stack.push(Frame {
                    kind,
                    body_start: i,
                });
            }
            "}" => {
                if let Some(frame) = stack.pop() {
                    let body = &t[frame.body_start..=i];
                    let names_site = body
                        .iter()
                        .zip(&mask[frame.body_start..=i])
                        .any(|(x, m)| x.text == SITE && !m);
                    match &frame.kind {
                        FrameKind::Fn(info) if names_site && !mask[frame.body_start] => {
                            let decides = body
                                .iter()
                                .any(|x| matches!(x.text.as_str(), "match" | "if" | "matches"));
                            let mut keyed = stack.clone();
                            keyed.push(frame.clone());
                            let key = key_of(&keyed);
                            if info.is_const {
                                out.denied.push(format!(
                                    "{file}:{}: `const fn {}` returns `StoreCorrupt`",
                                    t[frame.body_start].line, info.name
                                ));
                            }
                            let mapper = info.generic || info.ignored_arg || !decides;
                            let excepted = MAPPER_EXCEPTIONS
                                .iter()
                                .any(|(f, k, _)| *f == file && *k == key);
                            if info.returns_bare_wallet_error && mapper && !excepted {
                                out.denied.push(format!(
                                    "{file}:{}: `{key}` returns a bare WalletError, names \
                                     `StoreCorrupt` and classifies nothing (generic, an ignored \
                                     argument, or no decision in its body) — `.map_err({})` would \
                                     be a blind site with no token; classify the error instead",
                                    t[frame.body_start].line, info.name
                                ));
                            }
                        }
                        FrameKind::Impl {
                            name,
                            from_for_wallet_error: true,
                        } if names_site && !mask[frame.body_start] => {
                            out.denied.push(format!(
                                "{file}:{}: `impl {name}` maps a foreign error to `StoreCorrupt` — \
                                 every `?` over that error becomes a blind site",
                                t[frame.body_start].line
                            ));
                        }
                        _ => {}
                    }
                }
            }
            SITE if prod => {
                let key = key_of(&stack);
                out.counts.entry(key).or_default().push(t[i].line);
            }
            _ => {}
        }
        i += 1;
    }
    out
}

// ── The module walk ──────────────────────────────────────────────────────────

/// Every production `.rs` file of the crate: `lib.rs`, then each `mod x;`
/// declared OUTSIDE a `cfg(test)` item, resolved as rustc resolves it. A file
/// reached only through a test-gated declaration is test code and is not read.
fn production_files(src: &Path) -> Vec<PathBuf> {
    let mut seen: BTreeSet<PathBuf> = BTreeSet::new();
    let mut queue: VecDeque<PathBuf> = VecDeque::from([src.join("lib.rs")]);
    let mut out = Vec::new();
    while let Some(file) = queue.pop_front() {
        let Ok(canonical) = file.canonicalize() else {
            continue;
        };
        if !seen.insert(canonical) {
            continue;
        }
        let text =
            fs::read_to_string(&file).unwrap_or_else(|e| panic!("read {}: {e}", file.display()));
        let dir = file.parent().expect("a file has a directory").to_path_buf();
        let mod_rs = file
            .file_name()
            .is_some_and(|n| n == "lib.rs" || n == "mod.rs" || n == "main.rs");
        let owned = if mod_rs {
            dir.clone()
        } else {
            dir.join(file.file_stem().expect("a stem"))
        };
        for (name, is_path, components) in scan_text(&text, "").mods {
            let base = components.iter().fold(owned.clone(), |b, c| b.join(c));
            if is_path {
                assert!(
                    !name.split(['/', '\\']).any(|p| p == ".."),
                    "{}: a `#[path]` with `..` — refused, the walk will not follow it out of the tree",
                    file.display()
                );
                let from = if components.is_empty() {
                    dir.clone()
                } else {
                    base
                };
                queue.push_back(from.join(name));
            } else {
                let flat = base.join(format!("{name}.rs"));
                let nested = base.join(&name).join("mod.rs");
                queue.push_back(if flat.exists() { flat } else { nested });
            }
        }
        out.push(file);
    }
    out
}

fn rel(src: &Path, file: &Path) -> String {
    file.strip_prefix(src)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/")
}

/// (file, enclosing fn) → the lines of each `StoreCorrupt` token there.
type SiteCounts = BTreeMap<(String, String), Vec<usize>>;

/// The whole crate's production counts, keyed (file, fn), and every denial.
fn scan_crate() -> (SiteCounts, Vec<String>, usize) {
    let src = src_root();
    let files = production_files(&src);
    let mut counts = BTreeMap::new();
    let mut denied = Vec::new();
    for file in &files {
        let text =
            fs::read_to_string(file).unwrap_or_else(|e| panic!("read {}: {e}", file.display()));
        let name = rel(&src, file);
        let scan = scan_text(&text, &name);
        for (key, lines) in scan.counts {
            counts.insert((name.clone(), key), lines);
        }
        denied.extend(scan.denied);
    }
    (counts, denied, files.len())
}

// ── The guard ────────────────────────────────────────────────────────────────

/// R12 §4.4 — the guard. A RISE fails with the file, the fn and the lines; a
/// FALL prints a reminder (a fall of [`FLOOR`] fails); a denial fails outright.
#[test]
fn a_blind_store_corrupt_mapping_does_not_come_back() {
    let (counts, denied, files) = scan_crate();
    assert!(
        files >= FILE_FLOOR,
        "the module walk reached {files} files (floor {FILE_FLOOR}) — the resolver stopped early"
    );
    assert!(
        counts.contains_key(&(CANARY.0.to_string(), CANARY.1.to_string())),
        "the scan did not see `{}` in {} — §4.3's kept site; the scanner is not reading the \
         tree it thinks it is",
        CANARY.1,
        CANARY.0
    );
    let mut allowed: BTreeMap<(String, String), usize> = BTreeMap::new();
    for (file, key, count, class, reason) in ALLOWLIST {
        assert!(
            !class.is_empty() && !reason.is_empty(),
            "{file} {key}: every entry names its class and reason"
        );
        let dup = allowed.insert((file.to_string(), key.to_string()), *count);
        assert!(dup.is_none(), "{file} {key}: listed twice");
    }

    let mut rises = Vec::new();
    for ((file, key), lines) in &counts {
        let allow = allowed
            .get(&(file.clone(), key.clone()))
            .copied()
            .unwrap_or(0);
        if lines.len() > allow {
            rises.push(format!(
                "  {file} — {key}: {} found (lines {lines:?}), {allow} allowed",
                lines.len()
            ));
        }
    }
    let mut falls = Vec::new();
    let mut floor_fell = None;
    for ((file, key), allow) in &allowed {
        let found = counts.get(&(file.clone(), key.clone())).map_or(0, Vec::len);
        if found < *allow {
            if (file.as_str(), key.as_str()) == FLOOR {
                floor_fell = Some(format!("  {file} — {key}: {found} found, {allow} allowed"));
            } else {
                falls.push(format!("  {file} — {key}: {found} found, {allow} allowed"));
            }
        }
    }
    if !falls.is_empty() {
        println!(
            "R12 §4.4 reminder — these counts FELL; lower the allowlist entries so the table \
             stays a ceiling:\n{}",
            falls.join("\n")
        );
    }
    let total: usize = counts.values().map(Vec::len).sum();
    println!("the scan read {files} production files and counted {total} StoreCorrupt tokens");
    // Both halves in ONE failure, so a single run shows everything to fix.
    let mut failures = Vec::new();
    if !denied.is_empty() {
        failures.push(format!(
            "R12 §4.4: denied shapes — each makes a blind StoreCorrupt mapping the count cannot \
             see:\n{}",
            denied.join("\n")
        ));
    }
    if !rises.is_empty() {
        failures.push(format!(
            "R12 §4.4: a StoreCorrupt count ROSE. An error from SQLite or the engine is not \
             corruption until it is classified — classify it (`db::map_aux_err` for a \
             rusqlite::Error, `ClassifyStoreFault::into_store_fault` for an engine error), or \
             allowlist it with a reason:\n{}",
            rises.join("\n")
        ));
    }
    if let Some(fell) = floor_fell {
        failures.push(format!(
            "R13 §4.1: the `raw_tx_bytes` count FELL, and that entry is a floor. A classified \
             post-persist read makes DiskFull/StoreBusy/Io reachable AFTER the shield/move \
             transaction is saved — drop them from `singleStepSendErrorPrecedesPersistence` \
             (zec_wallet_ui send_state.dart) in the same commit, then lower this entry:\n{fell}"
        ));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

// ── The instrument's own honesty ─────────────────────────────────────────────

/// The lexer and the walk over every shape §4.4 names: strings with braces,
/// `format!` with `{x}`, raw strings, char literals told from lifetimes, line,
/// doc and nested block comments, a rustfmt-wrapped `|_e| { … }`, item-level
/// `cfg(test)` (module, fn, statement) and the two cfgs that are NOT test code.
#[test]
fn the_lexer_blanks_literals_and_comments_and_the_walk_keys_each_site_on_its_fn() {
    let src = r####"
/// A doc comment that names StoreCorrupt { is not a site.
fn braces_in_literals<'a>(x: &'a str) -> Result<(), WalletError> {
    let _s = "{ StoreCorrupt }}";
    let _q = format!("SELECT {notes} FROM {table} WHERE x = '{'");
    let _r = r#"{ "StoreCorrupt" }"#;
    let _rr = r##"}}"#}"##;
    let _b = b"{";
    let _open = '{';
    let _close = '}';
    let _quote = '"';
    let _esc = '\'';
    let _lt: &'a str = x;
    /* a block } with /* a nested { */ StoreCorrupt */
    Err(WalletError::StoreCorrupt) // StoreCorrupt in a trailing comment
}

fn wrapped(conn: &Connection) -> Result<(), WalletError> {
    conn.execute("x", [])
        .map_err(|_e| {
            WalletError::StoreCorrupt
        })?;
    Ok(())
}

impl ClassifyStoreFault for SqliteClientError {
    fn into_store_fault(self) -> WalletError {
        match self {
            _ => WalletError::StoreCorrupt,
        }
    }
}

impl Wallet {
    pub(crate) async fn scanned_tip(&self) -> Result<(), WalletError> {
        run_blocking(move || {
            #[cfg(test)]
            let _skipped = WalletError::StoreCorrupt;
            Err(WalletError::StoreCorrupt)
        })
    }
}

#[cfg(test)]
fn test_helper() -> WalletError {
    WalletError::StoreCorrupt
}

#[cfg_attr(not(test), allow(dead_code))]
fn kept_by_cfg_attr() -> Result<(), WalletError> {
    Err(WalletError::StoreCorrupt)
}

#[cfg(any(test, target_os = "ios"))]
fn kept_by_any() -> Result<(), WalletError> {
    Err(WalletError::StoreCorrupt)
}

#[cfg(all(test, target_os = "macos"))]
fn dropped_by_all() -> Result<(), WalletError> {
    Err(WalletError::StoreCorrupt)
}

#[cfg(test)]
mod tests {
    fn t() { let _ = WalletError::StoreCorrupt; }
}

fn after_the_test_module() -> Result<(), WalletError> {
    Err(WalletError::StoreCorrupt)
}
"####;
    let scan = scan_text(src, "fixture.rs");
    let got: BTreeMap<&str, usize> = scan
        .counts
        .iter()
        .map(|(k, v)| (k.as_str(), v.len()))
        .collect();
    let want: BTreeMap<&str, usize> = [
        ("braces_in_literals", 1),
        ("wrapped", 1),
        (
            "<SqliteClientError as ClassifyStoreFault>::into_store_fault",
            1,
        ),
        ("Wallet::scanned_tip", 1),
        ("kept_by_cfg_attr", 1),
        ("kept_by_any", 1),
        ("after_the_test_module", 1),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        got, want,
        "each production site keyed on its own fn: literals and comments blanked (a brace in a \
         string, a `format!` placeholder, a raw string, a char literal never moves the match; a \
         lifetime is not a char), the wrapped closure is one site of `wrapped`, the impl names \
         its type, the cfg(test) statement/fn/module and cfg(all(test, …)) are skipped and the \
         scan resumes after them, and cfg_attr(not(test)) / cfg(any(test, …)) are production"
    );
    assert!(
        scan.denied.is_empty(),
        "no evasion in this fixture: {:?}",
        scan.denied
    );
}

/// Every evasion §4.4 denies, each on its own, and the classifier shape it must
/// NOT deny.
#[test]
fn the_guard_denies_each_evasion_that_hides_a_blind_mapping_from_the_count() {
    let cases: &[(&str, &str)] = &[
        (
            "an alias",
            "use crate::error::WalletError::StoreCorrupt as Bad;\nfn f() {}",
        ),
        (
            "a bare import",
            "use crate::error::WalletError::{DiskFull, StoreCorrupt};\nfn f() {}",
        ),
        ("a glob", "use crate::error::WalletError::*;\nfn f() {}"),
        (
            "a glob in a list",
            "use crate::error::WalletError::{self, *};\nfn f() {}",
        ),
        (
            "a macro",
            "macro_rules! corrupt {\n    () => { WalletError::StoreCorrupt };\n}\nfn f() {}",
        ),
        (
            "a const",
            "const CORRUPT: WalletError = WalletError::StoreCorrupt;",
        ),
        (
            "a static",
            "static CORRUPT: WalletError = WalletError::StoreCorrupt;",
        ),
        (
            "a const fn",
            "const fn corrupt() -> Result<(), WalletError> {\n    Err(WalletError::StoreCorrupt)\n}",
        ),
        (
            "a generic mapper",
            "fn blind<E>(e: E) -> WalletError {\n    drop(e);\n    if true { WalletError::StoreCorrupt } else { WalletError::StoreCorrupt }\n}",
        ),
        (
            "a mapper that ignores its argument",
            "fn blind(_e: rusqlite::Error) -> WalletError {\n    if true { WalletError::StoreCorrupt } else { WalletError::DiskFull }\n}",
        ),
        (
            "a mapper that decides nothing",
            "fn blind(e: rusqlite::Error) -> WalletError {\n    let _ = e;\n    WalletError::StoreCorrupt\n}",
        ),
        (
            "a From impl",
            "impl From<std::io::Error> for WalletError {\n    fn from(e: std::io::Error) -> Self {\n        match e.kind() { _ => WalletError::StoreCorrupt }\n    }\n}",
        ),
        (
            "a let-bound blind closure",
            "fn f(c: &Connection) -> Result<(), WalletError> {\n    let corrupt = |_| WalletError::StoreCorrupt;\n    c.execute(\"x\", []).map_err(corrupt)?;\n    Ok(())\n}",
        ),
        (
            "a let-bound blind move closure",
            "fn f() {\n    let corrupt = move |_e: rusqlite::Error| {\n        WalletError::StoreCorrupt\n    };\n}",
        ),
    ];
    for (what, src) in cases {
        let scan = scan_text(src, "fixture.rs");
        assert!(
            !scan.denied.is_empty(),
            "{what}: the guard must deny this shape; it hides a blind mapping from the count:\n{src}"
        );
    }

    // The shapes it must NOT deny: a classifier (it decides, and names a corruption
    // class), a `use` of the enum itself, and the same shapes under `cfg(test)`.
    let allowed = r#"
use crate::error::WalletError;
use crate::error::WalletError as E;
pub(crate) fn classify(e: &rusqlite::Error) -> WalletError {
    match e {
        rusqlite::Error::SqliteFailure(err, _) => match err.code {
            rusqlite::ErrorCode::DatabaseBusy => WalletError::StoreBusy,
            _ => WalletError::StoreCorrupt,
        },
        _ => WalletError::StoreCorrupt,
    }
}
#[cfg(test)]
mod tests {
    use crate::error::WalletError::*;
    const C: WalletError = WalletError::StoreCorrupt;
    fn blind<E>(_e: E) -> WalletError { WalletError::StoreCorrupt }
}
"#;
    let scan = scan_text(allowed, "fixture.rs");
    assert!(
        scan.denied.is_empty(),
        "a classifier, a `use` of the enum, and test code are not evasions: {:?}",
        scan.denied
    );
    assert_eq!(
        scan.counts.get("classify").map(Vec::len),
        Some(2),
        "the classifier's defaults are counted, not denied"
    );
}

/// The walk against the real tree: a `cfg(test)` module file is not production,
/// a nested test file under `wallet/tests/` is not, and the production files that
/// sit beside them are.
#[test]
fn the_module_walk_reads_production_files_and_skips_test_gated_ones() {
    let src = src_root();
    let files: BTreeSet<String> = production_files(&src)
        .iter()
        .map(|f| rel(&src, f))
        .collect();
    for production in [
        "lib.rs",
        "wallet.rs",
        "db.rs",
        "account.rs",
        "block_cache.rs",
        "sync.rs",
    ] {
        assert!(
            files.contains(production),
            "{production} is production code and is scanned"
        );
    }
    for test_only in ["test_support.rs", "sync_bind_proof.rs", "anchor_proof.rs"] {
        assert!(
            !files.contains(test_only),
            "{test_only} is declared under `#[cfg(test)]` in lib.rs and is never production"
        );
    }
    assert!(
        !files.iter().any(|f| f.starts_with("wallet/tests/")),
        "the files declared inside wallet.rs's `#[cfg(test)] mod tests` are test code: {files:?}"
    );
}
