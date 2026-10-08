//! Bundled-checkpoint birthday estimation (spec §3.6). OFFLINE: turns "around
//! when did you create this wallet?" into a restore start height with NO
//! network call, via the `(height, block_time)` anchor table in `data.rs` (the
//! `checkmate`-generated set the ECC mobile SDKs ship — `CHECKPOINTS.md`
//! records the reproducible provenance).
//!
//! CONSERVATIVE FLOOR — the silent-fund-loss guard (§3.6): the estimate is the
//! height of the LATEST bundled checkpoint whose block-time ≤ the given time,
//! never a height ABOVE it. A birthday BEFORE true creation only costs a longer
//! scan; a birthday AFTER silently loses notes — so we never overshoot, and the
//! worst-case over-scan is bounded by the LARGEST inter-anchor gap in the
//! bundle (mainnet 80,800 blocks across the sparsely-sampled pre-2019 history;
//! 2,500 in the modern tail — `estimate_birthday_floors_..._never_overshoots`
//! exercises every gap).
//!
//! The tree-state slice of the same checkmate bundle (sapling/orchard frontiers)
//! is AS BUILT in W3-inc-2c-iv-c ([`bundled_treestate`], §3.2f): the
//! crypto audit showed the backend does NOT self-verify a birthday frontier, so
//! the bundled frontier in the signed binary is the TRUSTED anchor provisioning
//! uses INSTEAD of a live `get_tree_state` (no live-frontier trust). It is
//! ADDITIVE from the SAME pinned source rows (`MAINNET_TREESTATES` /
//! `TESTNET_TREESTATES`) — the `(height, time)` slice + `estimate_birthday` are
//! untouched.

mod data;

use zcash_client_backend::proto::service::TreeState;

use crate::money::{BlockHeight, Network};

/// Estimate the restore birthday height from an approximate creation time
/// (unix seconds), §3.6. Output is CLAMPED to `[activation, last bundled
/// checkpoint]`: a pre-activation time ⇒ activation; a future/too-recent time ⇒
/// the last checkpoint (we never extrapolate past what we know offline — §2.3
/// catches an above-tip birthday at sync as `BirthdayInFuture`).
///
/// Infallible by design (§3.1 signature). FAIL-CLOSED-TO-SAFE (§3.6 M2): a
/// bundle that fails its monotonic self-check (a corrupt/poisoned build
/// artifact) degrades to the activation height — a full scan, never a too-high
/// birthday — with a logged error, never a silent bad estimate. The shipped
/// bundle's monotonicity + activation anchoring is pinned by
/// `checkpoint_bundle_monotonic_or_load_fails`. The self-check runs on each
/// call — it is a ~N-comparison sweep over a static table and restore is not a
/// hot path, so the simplicity is worth more than caching.
pub fn estimate_birthday(network: Network, approx_unix_secs: u64) -> BlockHeight {
    let table = checkpoints(network);
    if !is_monotonic(table) {
        // SAFE degrade: an empty/corrupt bundle ⇒ the first known height (or 0
        // if somehow empty) ⇒ a full(er) scan, never a too-high (fund-losing)
        // birthday. `.first()` is panic-free even on the empty-table case that
        // `is_monotonic` also rejects. No PII in the line (§5.4): network + a
        // static message only.
        tracing::error!(
            target: "zec_wallet_core",
            ?network,
            "checkpoint bundle failed its monotonic self-check; using earliest known height (fuller scan)"
        );
        return BlockHeight::new(table.first().map_or(0, |&(h, _)| h));
    }
    let (first_h, first_t) = table[0];
    let (last_h, last_t) = table[table.len() - 1];
    if approx_unix_secs <= first_t {
        return BlockHeight::new(first_h); // pre-activation ⇒ activation
    }
    if approx_unix_secs >= last_t {
        return BlockHeight::new(last_h); // future/too-recent ⇒ last (no extrapolation)
    }
    // Floor: the latest checkpoint with block-time ≤ `approx_unix_secs`. The
    // table is time-sorted ascending, so the `t <= secs` rows are a contiguous
    // prefix; `partition_point` is its length (≥ 1 here — `first_t < secs`), and
    // index `len-1` is the floor.
    let idx = table.partition_point(|&(_, t)| t <= approx_unix_secs) - 1;
    BlockHeight::new(table[idx].0)
}

fn checkpoints(network: Network) -> &'static [(u32, u64)] {
    match network {
        Network::Main => data::MAINNET,
        Network::Test => data::TESTNET,
    }
}

/// The newest bundled checkpoint's BLOCK TIME (unix secs) — the creation
/// stamp's write-time clock-ahead cap (§3.2f #356-F4). The stamp is clamped to
/// THIS BINARY's bundle tail when written, so a device clock running ahead can
/// never stamp a time later than a block this binary already knew about — and,
/// load-bearingly, a FUTURE binary's fresher bundle can then never lift the
/// cap either (`estimate_birthday(stamp)` under any later bundle still floors
/// at ≤ this binary's tail height ≤ the tip at create ≤ any deposit's height).
/// Mirrors [`estimate_birthday`]'s fail-closed-to-safe degrade: a corrupt/
/// empty bundle returns 0, which clamps the stamp to 0 ⇒ the estimate floors
/// to activation ⇒ a full scan — never a too-high birthday.
pub(crate) fn newest_checkpoint_time(network: Network) -> u64 {
    let table = checkpoints(network);
    if !is_monotonic(table) {
        tracing::error!(
            target: "zec_wallet_core",
            ?network,
            "checkpoint bundle failed its monotonic self-check; creation stamp clamps to 0 (fuller scan)"
        );
        return 0;
    }
    table.last().map_or(0, |&(_, t)| t)
}

/// §3.6 self-check: a non-empty table whose heights AND block-times are BOTH
/// STRICTLY increasing. This makes the floor search well-defined and rejects a
/// bundle corrupted by REORDERING, DUPLICATION, or a REGRESSED height/time.
///
/// It does NOT — and cannot — detect a STILL-MONOTONIC value tamper: an
/// attacker who LOWERS one interior anchor's time while keeping the row above
/// its neighbour passes this check yet shifts the floor later for a band of
/// dates (an overshoot → silent fund loss). That class is closed not here but
/// by PROVENANCE INTEGRITY — the pinned content hash
/// (`checkpoint_data_matches_pinned_provenance_hash`) makes any change to
/// `data.rs` a CI failure that forces re-derivation from the `CHECKPOINTS.md`
/// source. The activation-anchor half (first row == Sapling activation) is
/// asserted by `checkpoint_bundle_monotonic_or_load_fails`.
fn is_monotonic(table: &[(u32, u64)]) -> bool {
    !table.is_empty() && table.windows(2).all(|w| w[1].0 > w[0].0 && w[1].1 > w[0].1)
}

// ── The tree-state slice (W3-inc-2c-iv-c, §3.2f) ─────────────────────────────

/// The tree-state slice for `network`: `(height, hash_hex, sapling_hex,
/// orchard_hex, ironwood_hex)`, ascending by height, row-aligned with the
/// `(height, time)` slice (`treestate_and_time_slices_are_row_aligned`).
///
/// The Ironwood column entered with the NU6.3 pin wave (Phase B step 4). It is
/// EMPTY below the NU6.3 activation and populated at/above it, on 13 mainnet
/// and 25 testnet rows — a distinction that has to survive into the DTO,
/// because `TreeState::ironwood_tree()` decodes an empty field to an empty tree
/// with no error.
pub(crate) fn treestate_table(
    network: Network,
) -> &'static [(u32, &'static str, &'static str, &'static str, &'static str)] {
    match network {
        Network::Main => data::MAINNET_TREESTATES,
        Network::Test => data::TESTNET_TREESTATES,
    }
}

/// The lightwalletd `chain_name` for `network` (the §3.2f network-match label).
pub(crate) fn chain_name(network: Network) -> &'static str {
    match network {
        Network::Main => "main",
        Network::Test => "test",
    }
}

/// Sapling activation height — the bundle's first-row anchor
/// (`checkpoint_bundle_monotonic_or_load_fails` pins it). The birthday FLOOR
/// (no shielded notes exist below) and the §3.2f network-match discriminant
/// (compared against the endpoint's `sapling_activation_height`).
pub(crate) fn activation_height(network: Network) -> u32 {
    checkpoints(network).first().map_or(0, |&(h, _)| h)
}

/// Build the TRUSTED birthday treestate (§3.2f): the signed-binary commitment-tree
/// frontier at the NEWEST bundled checkpoint with `height < requested`, so the
/// resulting birthday (`height + 1`) is `≤ requested` and NEVER skips the user's
/// funds at `requested`. The frontier comes from the SIGNED BINARY, never a live
/// `get_tree_state` — the crypto audit (§3.2f) proved a live frontier is
/// cryptographically unverifiable, so we provision from the bundle only.
///
/// When `requested` is at/below the activation row, the activation row is used
/// (the floor — no shielded notes exist below; the documented activation-floor
/// edge gives birthday `activation + 1`). Returns the synthesized upstream
/// [`TreeState`] DTO (decoded by `account::birthday_from_treestate`, the same M2
/// boundary inc-2c-iii built — but here the input is TRUSTED signed-binary data,
/// so a decode failure is a corrupt-binary bug, fail-closed). `None` only if the
/// bundle is empty (a corrupt binary — every shipped row is non-empty +
/// decodable, `treestate_bundle_every_row_decodes`).
pub(crate) fn bundled_treestate(network: Network, requested: BlockHeight) -> Option<TreeState> {
    let table = treestate_table(network);
    // `ironwood-nu63-support.md` §1.5 / §9.2 — THE ANCHOR CEILING. Never anchor
    // above the newest activation this build knows: past it, an upgrade we have
    // never heard of may have activated, and the chain state there can contain
    // a value pool we cannot represent. Anchoring on such a row bakes an
    // implicitly-empty tree for that pool into the wallet — persisted,
    // immutable after first import, and NOT OBVIOUSLY repairable in place once a later
    // build learns the pool. (Concretely: the bundle ships 13 mainnet rows
    // above the Ironwood activation, and a fresh create floors onto the newest
    // row, so without this every wallet created on a pre-Ironwood build would
    // be born wrong.)
    //
    // Cost is a longer first scan, which is the direction the birthday logic
    // already prefers — a low anchor over-scans, a high one silently skips.
    // The ceiling is derived from the compiled params, never a literal — but it
    // does NOT rise by itself at a crate bump, which an earlier version of this
    // comment claimed and three documents then copied.
    // `newest_known_activation` reads `consensus::KNOWN_UPGRADES`, a
    // HAND-MAINTAINED list. `NetworkUpgrade::Nu6_3` IS on that list, added
    // deliberately in the same commit as the crate wave rather than by it, so the
    // mainnet ceiling is Nu6_3's 3,428,143 — NOT the Nu6_2 height this comment
    // used to name, which `consensus.rs` (`KNOWN_UPGRADES`, and
    // `the_anchor_ceiling_is_the_newest_upgrade_this_build_can_model`) has
    // contradicted since. Raising it further means editing that list again, which
    // `known_upgrades_is_really_complete` forces.
    //
    // ADJUDICATOR (T0-1 fold, IT-9): the superseded height is named by UPGRADE and
    // not spelled out, deliberately. `the_anchor_ceiling_is_narrated_with_the_
    // height_it_actually_has` — the test author's unlisted guard, landed in the
    // same fold — refuses any pre-Nu6_3 activation height appearing in this file's
    // production prose. It cannot tell a stale CLAIM from a sentence correcting
    // one, so the correction printed here in the implementer's commit red-lined it.
    // Neither half was wrong against the contract; the two collided only once
    // folded, which is the class IT-9 exists for. Rewording rather than relaxing
    // the guard is the better side of the trade anyway: the next reader greps this
    // file for a number, and a file whose job is to narrate the live ceiling should
    // not print a dead one beside it.
    //
    // The measured consequence of it having moved, because the obvious guess is
    // wrong: the 13 bundled mainnet rows above the Ironwood activation are STILL
    // unreachable — `bundled_treestate` takes the newest row strictly below
    // `ceiling + 1` and the first of those rows is 3,429,810, above it. What moved
    // is the newest anchorable row, 3,362,500 -> 3,427,310, and every one of those
    // is BELOW the activation, where an empty Ironwood frontier is correct.
    let ceiling = crate::consensus::newest_known_activation(&network.consensus())
        .map_or(u32::MAX, |h| u32::from(h).saturating_add(1));
    let req = requested.value().min(ceiling);
    // newest row with height < req; floor to the activation row (idx 0) when none.
    let idx = table.partition_point(|&(h, ..)| h < req).saturating_sub(1);
    let &(height, hash, sapling, orchard, ironwood) = table.get(idx)?;
    // Defense-in-depth money post-condition (parity with `estimate_birthday`'s
    // `is_monotonic` fail-closed): the selected row MUST be below `req` unless it
    // is the activation floor (idx 0). A mis-sorted table — which the pinned hash
    // + `treestate_and_time_slices_are_row_aligned` already prevent — would
    // otherwise let `partition_point` pick a row ≥ `req` ⇒ birthday > requested ⇒
    // a SILENT fund skip. Fail closed instead (`None` ⇒ `StoreCorrupt`).
    if idx != 0 && height >= req {
        return None;
    }
    Some(TreeState {
        network: chain_name(network).to_owned(),
        height: u64::from(height),
        // `time` is unused by `to_chain_state` (it parses hash + frontiers); the
        // `(height, time)` slice owns block-time for `estimate_birthday`.
        time: 0,
        hash: hash.to_owned(),
        sapling_tree: sapling.to_owned(),
        orchard_tree: orchard.to_owned(),
        // The bundled Ironwood frontier (Phase B step 4). Empty below the NU6.3
        // activation, which is CORRECT there — the pool does not exist yet — and
        // populated above it. Passing the bundled value rather than `String::new()`
        // is the whole point of the re-extraction: `ironwood_tree()` maps an empty
        // field to an empty tree with NO error, so a hard-coded empty here would
        // anchor a post-activation wallet against a pool the chain already has,
        // compile, run, and be wrong. The anchor ceiling makes those rows
        // unreachable today; it is a ceiling, and this is the data under it.
        ironwood_tree: ironwood.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Sapling activation heights — the bundle's first-row anchor contract.
    const MAINNET_ACTIVATION: u32 = 419_200;
    const TESTNET_ACTIVATION: u32 = 280_000;

    #[test]
    fn checkpoint_bundle_monotonic_or_load_fails() {
        // §8 named test. The SHIPPED bundles pass the load-time self-check ...
        assert!(
            is_monotonic(data::MAINNET),
            "mainnet bundle must be monotonic"
        );
        assert!(
            is_monotonic(data::TESTNET),
            "testnet bundle must be monotonic"
        );
        // ... and are Sapling-activation-anchored (first row).
        assert_eq!(data::MAINNET[0].0, MAINNET_ACTIVATION);
        assert_eq!(data::TESTNET[0].0, TESTNET_ACTIVATION);

        // a corrupt/poisoned table is REJECTED (never trusted → never a wrong
        // height): every way the strict-increasing invariant can break.
        assert!(!is_monotonic(&[(10, 100), (9, 200)]), "height regresses");
        assert!(!is_monotonic(&[(10, 200), (11, 100)]), "time regresses");
        assert!(!is_monotonic(&[(10, 100), (10, 200)]), "height not STRICT");
        assert!(!is_monotonic(&[(10, 100), (11, 100)]), "time not STRICT");
        assert!(!is_monotonic(&[]), "empty");
    }

    #[test]
    fn checkpoint_data_matches_pinned_provenance_hash() {
        // §4.6 M2 — the REAL defense the monotonic self-check CANNOT provide: a
        // still-monotonic value tamper (a lowered interior time) shifts the
        // floor later → overshoot. Pinning the canonical content hash turns ANY
        // change to `data.rs` into a CI failure that forces re-derivation from
        // the `CHECKPOINTS.md`-pinned source. Canonical form: MAINNET then
        // TESTNET, each row = u32-LE height ++ u64-LE time.
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        for table in [data::MAINNET, data::TESTNET] {
            for &(height, time) in table {
                h.update(height.to_le_bytes());
                h.update(time.to_le_bytes());
            }
        }
        assert_eq!(
            hex::encode(h.finalize()),
            "7aedb445336f451d09b87d58a150cf819b46f9770680f4883080485d5877369b",
            "checkpoint data changed — re-derive from the CHECKPOINTS.md source + update the pin"
        );
    }

    #[test]
    fn birthday_out_of_range_arms() {
        // §8 named test (the OFFLINE clamp arms; the above-tip-birthday ⇒
        // `BirthdayInFuture` arm is config validation on the gated wallet
        // handle, §2.3 — not reachable from this estimator).
        for (net, table) in [
            (Network::Main, data::MAINNET),
            (Network::Test, data::TESTNET),
        ] {
            let (first_h, first_t) = table[0];
            let (last_h, last_t) = table[table.len() - 1];
            assert_eq!(
                estimate_birthday(net, 0).value(),
                first_h,
                "{net:?}: epoch ⇒ activation"
            );
            assert_eq!(
                estimate_birthday(net, first_t - 1).value(),
                first_h,
                "{net:?}: pre-activation ⇒ activation"
            );
            assert_eq!(
                estimate_birthday(net, first_t).value(),
                first_h,
                "{net:?}: exactly activation"
            );
            assert_eq!(
                estimate_birthday(net, last_t).value(),
                last_h,
                "{net:?}: exactly last"
            );
            assert_eq!(
                estimate_birthday(net, last_t + 1).value(),
                last_h,
                "{net:?}: just-future ⇒ last"
            );
            assert_eq!(
                estimate_birthday(net, u64::MAX).value(),
                last_h,
                "{net:?}: far-future ⇒ last, never extrapolated"
            );
        }
    }

    #[test]
    fn newest_checkpoint_time_is_the_bundle_tails_time() {
        // §3.2f #356-F4: the creation stamp's write-time clock-ahead cap IS the
        // last bundled row's block time — a stamp clamped to it estimates to
        // exactly the tail height under THIS bundle, and (the load-bearing
        // upgrade property) can never exceed the tail's TIME, so no future
        // bundle can estimate it above this binary's tail HEIGHT either.
        for net in [Network::Main, Network::Test] {
            let table = checkpoints(net);
            let (last_h, last_t) = table[table.len() - 1];
            let cap = newest_checkpoint_time(net);
            assert_eq!(cap, last_t, "{net:?}: the cap is the tail's block time");
            assert_eq!(
                estimate_birthday(net, cap).value(),
                last_h,
                "{net:?}: a cap-clamped stamp estimates to the tail height"
            );
        }
    }

    #[test]
    fn estimate_birthday_floors_to_bracketing_checkpoint_never_overshoots() {
        // §8 named test. FLOOR + NEVER-OVERSHOOT over EVERY bracketing interval
        // of the REAL bundles: query the midpoint of each consecutive pair; the
        // estimate is the LOWER checkpoint, whose time ≤ query (height never
        // ahead of the date) — exercising every gap, incl. the 80,800-block one.
        for (net, table) in [
            (Network::Main, data::MAINNET),
            (Network::Test, data::TESTNET),
        ] {
            for w in table.windows(2) {
                let (h0, t0) = w[0];
                let (_h1, t1) = w[1];
                if t1 - t0 < 2 {
                    continue; // no room for a strict interior midpoint
                }
                let mid = t0 + (t1 - t0) / 2;
                assert_eq!(
                    estimate_birthday(net, mid).value(),
                    h0,
                    "{net:?}: floor to the lower bracket at t={mid}"
                );
                assert!(
                    t0 <= mid,
                    "{net:?}: chosen checkpoint never dated after the query"
                );
            }
        }
    }

    #[test]
    fn poisoned_checkpoint_or_tree_state_does_not_silently_hide_funds() {
        // §4.6 M2 (bundle slice). The dangerous poison is a STILL-MONOTONIC time
        // tamper — a lowered interior time passes `is_monotonic` yet shifts the
        // floor later (overshoot). This proves the TWO-LAYER defense:
        let honest = [(100u32, 1000u64), (200, 2000), (300, 3000)];
        let poisoned = [(100u32, 1000u64), (200, 1001), (300, 3000)]; // #200 time lowered

        // (1) the monotonic self-check ALONE does NOT catch it ...
        assert!(is_monotonic(&honest));
        assert!(
            is_monotonic(&poisoned),
            "a still-monotonic time tamper slips the self-check"
        );
        // ... and WOULD overshoot: a t=1500 query floors to 100 honestly, 200 poisoned.
        let floor = |t: &[(u32, u64)], q: u64| {
            t.partition_point(|&(_, x)| x <= q)
                .checked_sub(1)
                .map(|i| t[i].0)
        };
        assert_eq!(floor(&honest, 1500), Some(100));
        assert_eq!(
            floor(&poisoned, 1500),
            Some(200),
            "the overshoot the self-check can't prevent"
        );

        // (2) the HASH GATE does catch it — poisoned data hashes differently, so
        // it can never silently ship (the shipped bundle's hash is pinned by
        // `checkpoint_data_matches_pinned_provenance_hash`).
        use sha2::{Digest, Sha256};
        let hash = |rows: &[(u32, u64)]| {
            let mut h = Sha256::new();
            for &(a, b) in rows {
                h.update(a.to_le_bytes());
                h.update(b.to_le_bytes());
            }
            hex::encode(h.finalize())
        };
        assert_ne!(
            hash(&honest),
            hash(&poisoned),
            "the hash gate distinguishes the poison"
        );
        // The LIVE tree-state path is RESOLVED (§3.2f): we provision the birthday
        // frontier from the BUNDLED tree-state slice (below), never a live fetch.
    }

    // ── The tree-state slice (W3-inc-2c-iv-c, §3.2f) ─────────────────────────

    #[test]
    fn treestate_and_time_slices_are_row_aligned() {
        // §8: the tree-state slice and the (height,time) slice are ONE source —
        // same rows, same order, same heights. A drift between them is a
        // generation bug (the two pinned hashes guard the contents separately).
        for (ts_table, t_table) in [
            (data::MAINNET_TREESTATES, data::MAINNET),
            (data::TESTNET_TREESTATES, data::TESTNET),
        ] {
            assert_eq!(ts_table.len(), t_table.len(), "same row count");
            for (a, b) in ts_table.iter().zip(t_table.iter()) {
                assert_eq!(a.0, b.0, "same heights, same order");
            }
        }
        // anchored at activation (the floor + network-match discriminant)
        assert_eq!(activation_height(Network::Main), MAINNET_ACTIVATION);
        assert_eq!(activation_height(Network::Test), TESTNET_ACTIVATION);
    }

    #[test]
    fn treestate_bundle_matches_pinned_provenance_hash() {
        // §4.6 M2 over the tree-state slice (sibling of
        // `checkpoint_data_matches_pinned_provenance_hash`): any change to a
        // bundled frontier fails CI and forces re-derivation from the
        // CHECKPOINTS.md-pinned source. Canonical form: MAINNET_TREESTATES then
        // TESTNET_TREESTATES, each row = u32-LE height ++ for each of
        // (hash, sapling, orchard, ironwood): u32-LE byte-len ++ the ASCII hex
        // bytes.
        //
        // The canonical form GAINED the Ironwood column in Phase B step 4, so
        // this pin moved over an unchanged upstream commit
        // (`3819943…`): the shape changed, not the source. Proven, not
        // asserted — the re-extraction was run against the previous pinned
        // commit first and reproduced the old `data.rs` byte-for-byte, then run
        // again with the field carried, and every pre-existing column of all
        // 1,242 rows is byte-identical. The `(height, time)` slice's pin below
        // is UNCHANGED, which is the cheap check that says the same thing.
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        for table in [data::MAINNET_TREESTATES, data::TESTNET_TREESTATES] {
            for &(height, hash, sapling, orchard, ironwood) in table {
                h.update(height.to_le_bytes());
                for s in [hash, sapling, orchard, ironwood] {
                    h.update((s.len() as u32).to_le_bytes());
                    h.update(s.as_bytes());
                }
            }
        }
        assert_eq!(
            hex::encode(h.finalize()),
            "f405eb5f714b88d831df4b6363eae0cb0f62c00f5e34e885a87f4eb9d7970605",
            "treestate data changed — re-derive from the CHECKPOINTS.md source + update the pin"
        );
    }

    #[test]
    fn treestate_bundle_every_row_decodes() {
        // §8 (vectors-first): EVERY bundled row (counts live in CHECKPOINTS.md,
        // the one home for them — they change on each refresh) synthesizes a
        // TreeState that decodes to a valid AccountBirthday via the inc-2c-iii
        // boundary — no runtime decode surprise on the trusted signed-binary
        // anchor. Covers the activation row (empty "000000" sapling frontier) and
        // pre-NU5 rows (empty orchard frontier). Birthday == row height + 1.
        for (net, table) in [
            (Network::Main, data::MAINNET_TREESTATES),
            (Network::Test, data::TESTNET_TREESTATES),
        ] {
            for &(height, hash, sapling, orchard, ironwood) in table {
                let ts = TreeState {
                    network: chain_name(net).to_owned(),
                    height: u64::from(height),
                    time: 0,
                    hash: hash.to_owned(),
                    sapling_tree: sapling.to_owned(),
                    orchard_tree: orchard.to_owned(),
                    ironwood_tree: ironwood.to_owned(),
                };
                let b = crate::account::birthday_from_treestate(ts)
                    .unwrap_or_else(|_| panic!("{net:?} bundled row {height} must decode"));
                assert_eq!(
                    u64::from(b.height()),
                    u64::from(height) + 1,
                    "{net:?} row {height}: birthday = height + 1"
                );
            }
        }
    }

    /// NU5 activation heights (`zcash_protocol::consensus`, verified against
    /// zcash_protocol-0.9.0 consensus.rs:492/526). NOT 1680000/1840000 — those
    /// are merely the last bundled rows BELOW the activation.
    const MAINNET_NU5: u32 = 1_687_104;
    const TESTNET_NU5: u32 = 1_842_420;

    fn nu5_activation(net: Network) -> u32 {
        match net {
            Network::Main => MAINNET_NU5,
            Network::Test => TESTNET_NU5,
        }
    }

    #[test]
    fn treestate_bundle_frontiers_grow_and_orchard_activates_at_nu5() {
        // §4.6 M2 — the assertion `treestate_bundle_every_row_decodes` CANNOT
        // make. That test's only non-trivial check is `birthday == height + 1`,
        // and the birthday is derived from the height WE supplied, so it is
        // tautological with respect to the frontier: no frontier value can fail
        // it. A blanked orchard frontier on a post-NU5 row therefore passed
        // every gate we had — including the pinned provenance hash, because a
        // refresh updates the pin in the SAME commit as the data (the pin
        // catches LATER drift, never the refresh itself).
        //
        // The frontier is what positions every note in the tree. Blanking one
        // puts notes at position ~0 instead of ~50.4M, so witnesses are built
        // against the wrong subtree, anchors never match chain, and funds are
        // silently unspendable from import. Closing that needs a check on the
        // DECODED tree, which is what this is.
        for (net, table) in [
            (Network::Main, data::MAINNET_TREESTATES),
            (Network::Test, data::TESTNET_TREESTATES),
        ] {
            let nu5 = nu5_activation(net);
            let mut prev: Option<(u32, u64, u64)> = None;
            let mut rows_checked = 0usize;
            for &(height, hash, sapling, orchard, ironwood) in table {
                // Absent below NU5, PRESENT from the activation onward — where
                // "present" starts as the empty tree "000000" (size 0), which is
                // a different thing from absent.
                if height < nu5 {
                    assert_eq!(
                        orchard, "",
                        "{net:?} row {height}: orchard frontier must be ABSENT below NU5 {nu5}"
                    );
                } else {
                    assert_ne!(
                        orchard, "",
                        "{net:?} row {height}: orchard frontier must be PRESENT at/after NU5 {nu5}"
                    );
                }

                let ts = TreeState {
                    network: chain_name(net).to_owned(),
                    height: u64::from(height),
                    time: 0,
                    hash: hash.to_owned(),
                    sapling_tree: sapling.to_owned(),
                    orchard_tree: orchard.to_owned(),
                    ironwood_tree: ironwood.to_owned(),
                };
                let b = crate::account::birthday_from_treestate(ts)
                    .unwrap_or_else(|_| panic!("{net:?} bundled row {height} must decode"));
                let cs = b.prior_chain_state();
                let (sap, orch) = (
                    cs.final_sapling_tree().tree_size(),
                    cs.final_orchard_tree().tree_size(),
                );

                // Note commitment trees are APPEND-ONLY: a later checkpoint can
                // never describe a smaller tree. This is the assertion that kills
                // the blanked-frontier mutant — size drops to 0 against a
                // predecessor in the tens of millions.
                if let Some((ph, psap, porch)) = prev {
                    assert!(
                        sap >= psap,
                        "{net:?} sapling tree SHRANK {ph} ({psap}) -> {height} ({sap})"
                    );
                    assert!(
                        orch >= porch,
                        "{net:?} orchard tree SHRANK {ph} ({porch}) -> {height} ({orch})"
                    );
                }
                prev = Some((height, sap, orch));
                rows_checked += 1;
            }
            // Anti-vacuity: the loop ran over a real bundle, and the tail is a
            // genuinely large tree — not a table of zeroes that trivially
            // satisfies "non-decreasing".
            assert!(
                rows_checked > 300,
                "{net:?}: only {rows_checked} rows checked — bundle looks empty"
            );
            let (tail_h, tail_sap, tail_orch) = prev.expect("non-empty table");
            assert!(
                tail_sap > 100_000 && tail_orch > 100_000,
                "{net:?} tail row {tail_h}: frontiers look empty (sapling {tail_sap}, orchard {tail_orch})"
            );
        }
    }

    /// The lowest bundled tree-state row at or above the Ironwood activation —
    /// the re-import threshold (`ironwood-nu63-support.md` §9.2 /
    /// `ironwood-phase-b.md` §2 / ADR-0540 Decision 6).
    const REIMPORT_THRESHOLD_MAINNET: u32 = 3_429_810;
    const REIMPORT_THRESHOLD_TESTNET: u32 = 4_134_750;

    /// Ironwood / NU6.3 activation, READ FROM THE CRATE (Phase B step 7).
    ///
    /// These were `IRONWOOD_MAINNET` / `IRONWOOD_TESTNET` literals, and their own
    /// comment called the pair "a known duplicate" of `consensus.rs`'s, to be
    /// swept in step 7. Both are gone now: the pinned `zcash_protocol` knows
    /// `Nu6_3`, so a literal here would be a second source of truth for a
    /// consensus constant — the exact shape that produced the outage. The value is
    /// pinned to the number the ADR, the spec and the emitter state by
    /// `consensus::tests::checkpoint_activation_heights_match_zcash_protocol`.
    ///
    /// This also RETIRES a limit the gap test below documented at length: it used
    /// to note that any upward drift of the activation literal rode along
    /// unguarded, "by construction, not by oversight", because a test cannot
    /// self-anchor a constant it compares against. That is no longer the shape —
    /// the activation now comes from the compiled params, so moving it means
    /// moving the pin, which is a reviewed event.
    fn ironwood_activation(net: Network) -> u32 {
        use zcash_protocol::consensus::{NetworkUpgrade, Parameters};
        u32::from(
            net.consensus()
                .activation_height(NetworkUpgrade::Nu6_3)
                .expect("the pinned params know Nu6_3 — Phase B step 6 added it"),
        )
    }

    fn reimport_threshold(net: Network) -> u32 {
        match net {
            Network::Main => REIMPORT_THRESHOLD_MAINNET,
            Network::Test => REIMPORT_THRESHOLD_TESTNET,
        }
    }

    #[test]
    fn treestate_bundle_samples_no_row_between_ironwood_activation_and_the_reimport_threshold() {
        // `ironwood-nu63-support.md` §9.2 / `ironwood-phase-b.md` §2 / ADR-0540
        // Decision 6 — the gate that lets the re-import threshold NARROW below
        // the activation height, and the only thing holding the narrowing up.
        //
        // A wallet provisioned before the Ironwood pool was modelled carries an
        // implicitly-EMPTY third frontier in its birthday `ChainState`. The
        // conservative threshold is the activation height itself: re-import
        // every account born at or above 3,428,143. Narrowing it to the first
        // bundled row above the activation (3,429,810, 1,667 blocks later)
        // rests on ONE claim — that no bundled row falls between them, so no
        // wallet can be ANCHORED there. `bundled_treestate` floors to a bundled
        // row and `birthday_from_treestate` yields `row + 1`, so a birthday
        // anywhere in that window still anchors on the newest row BELOW the
        // activation, where an empty Ironwood frontier is correct by consensus.
        //
        // The claim is worth a gate rather than a comment because the obvious
        // alternative justification is FALSE: this is not an empty-tree region.
        // Measured against zec.rocks / testnet.zec.rocks (ECC LightWalletD
        // v0.5.3 / v0.5.4) on 2026-09-07, `TreeState` field 7 is ABSENT below
        // activation, present-and-empty ("000000") at exactly the activation
        // height, and NON-EMPTY from 3,428,144 on mainnet and 4,134,683 on
        // testnet — so an empty frontier is correct for exactly one height
        // inside the window on mainnet and 683 on testnet, in neither case
        // throughout it. The window is safe ONLY while the bundle samples no
        // row inside it, and this is what says so.
        //
        // The frontier half of the §8 contract — every row at/after the
        // threshold carries a decoded, non-decreasing Ironwood `tree_size()`,
        // and the 13 / 25 row counts — needs the 5-ary table the Phase B
        // re-extraction produces and stays OWED as
        // `treestate_bundle_carries_an_ironwood_frontier_after_activation`.
        for (net, table) in [
            (Network::Main, data::MAINNET_TREESTATES),
            (Network::Test, data::TESTNET_TREESTATES),
        ] {
            let activation = ironwood_activation(net);
            let threshold = reimport_threshold(net);

            // THE DIRECTION GUARD. Everything else here is relative — it
            // compares the two constants against each other and against the
            // table — so a JOINT upward drift of both would pass while silently
            // excluding wallets that are already anchored above the old
            // threshold and need remediation. Over-including costs a rescan;
            // under-including costs wrong witnesses for real funds, so the
            // threshold may only ever move DOWN toward the activation.
            //
            // The ceiling is expressed as a DISTANCE from the activation, not as
            // the threshold's own value. A ceiling spelled `3_429_810` would be
            // the same token as `REIMPORT_THRESHOLD_MAINNET`, so a value-based
            // search-and-replace — the way the rest of this constant already
            // gets edited — would move both in lockstep and the guard would
            // degenerate to `X <= X`. Neither `1_667` nor `750` appears
            // anywhere else in this crate as a height, so nothing that edits a
            // height can move them by accident. (`750` does appear in
            // `relim-core`'s transport as a millisecond duration — a different
            // crate and a different unit, so no height edit reaches it.) They
            // are frozen at the distances ADR-0540 Decision 6 records.
            //
            // WHAT THIS DID NOT COVER, and no longer has to. Every assertion
            // here is anchored to `activation`, so while that was a LITERAL in
            // this file, any upward drift of it rode along: every assertion
            // passed for an activation raised by anything from +1 up to the full
            // window width (+1,667 mainnet; +1,668 was the first value the
            // inversion assert and the table pin caught). The comment called that
            // a limit rather than a hole, on the grounds that a test cannot
            // self-anchor a consensus constant — any literal it compares against
            // is one more literal in the same file.
            //
            // **Phase B step 7 removed the premise.** `ironwood_activation` now
            // reads `NetworkUpgrade::Nu6_3` from the compiled params, so the
            // activation is not a literal here at all; drifting it means moving
            // the crate pin, which is a reviewed event, and
            // `consensus::tests::checkpoint_activation_heights_match_zcash_protocol`
            // pins the crate's value to the number the ADR, the spec and the
            // emitter state. What stays literal is the frozen DISTANCE below, and
            // deliberately: it is the direction guard, and it must not be
            // derivable from the two constants it polices.
            let frozen_max_distance: u32 = match net {
                Network::Main => 1_667,
                Network::Test => 750,
            };
            let frozen_ceiling = activation + frozen_max_distance;
            assert!(
                threshold <= frozen_ceiling,
                "{net:?}: re-import threshold {threshold} is more than {frozen_max_distance} \
                 blocks above the Ironwood activation {activation}. A threshold may only \
                 narrow toward the activation, never rise: raising it drops already-anchored \
                 wallets out of remediation. If a bundle refresh removed the row this was \
                 pinned to, WIDEN the threshold to {activation} — do not follow the table \
                 upward."
            );
            assert!(
                activation <= threshold,
                "{net:?}: activation {activation} is above the threshold {threshold} — the \
                 window is inverted and the narrowing argument does not apply"
            );

            let below: Vec<u32> = table
                .iter()
                .map(|&(h, ..)| h)
                .filter(|&h| h < activation)
                .collect();
            let at_or_above: Vec<u32> = table
                .iter()
                .map(|&(h, ..)| h)
                .filter(|&h| h >= activation)
                .collect();

            // Anti-vacuity: the activation must fall INSIDE the bundle's range,
            // or "no row in the window" is trivially true — as it would be for a
            // bundle that stops before Ironwood entirely, or one that starts
            // after it.
            assert!(
                !below.is_empty(),
                "{net:?}: no bundled row below the Ironwood activation {activation} — \
                 the window check is vacuous"
            );
            assert!(
                !at_or_above.is_empty(),
                "{net:?}: no bundled row at/above the Ironwood activation {activation} — \
                 the window check is vacuous, and the anchor ceiling has nothing to exclude"
            );

            // The load-bearing claim. (Implied by the pin below, since
            // `at_or_above` is ascending — kept because its failure message is
            // the one a bundle refresh needs to read.)
            let in_window: Vec<u32> = at_or_above
                .iter()
                .copied()
                .filter(|&h| h < threshold)
                .collect();
            assert!(
                in_window.is_empty(),
                "{net:?}: bundled row(s) {in_window:?} fall in [{activation}, {threshold}) — a \
                 wallet anchored there would bake an empty Ironwood frontier that the chain \
                 does not have. The re-import threshold must WIDEN back to {activation}; do \
                 not retune this test."
            );

            // Pin the threshold constant to the data, so the number in the ADR,
            // the spec and the remediation cannot drift from the bundle.
            assert_eq!(
                at_or_above[0], threshold,
                "{net:?}: the first bundled row at/above the Ironwood activation \
                 {activation} is not the height the re-import threshold is stated as"
            );
        }
    }

    #[test]
    fn treestate_bundle_carries_an_ironwood_frontier_after_activation() {
        // §8 named test, OWED since the 2026-09-06 bundle refresh first carried
        // `ironwoodTree` and payable only now that the table is 5-ary
        // (`ironwood-phase-b.md` §3, the "bump without re-extract" row).
        //
        // This is the gate on the hazard the whole re-extraction exists for, and
        // it is a hazard precisely because NOTHING ELSE FAILS:
        // `TreeState::ironwood_tree()` maps an EMPTY field to `CommitmentTree::
        // empty()` and returns `Ok`, so a bundle that dropped the column — or an
        // emitter that regressed to `String::new()` — decodes cleanly, anchors
        // every post-activation wallet against an empty third pool, reports 100%
        // synced, and loses the funds silently. `treestate_bundle_every_row_
        // decodes` cannot see it (its only real assertion is `birthday ==
        // height + 1`, derived from the height WE supplied); the pinned
        // provenance hash cannot see it (a refresh updates the pin in the same
        // commit as the data). The check has to be on the DECODED tree, which is
        // what this is — the exact sibling of
        // `..._frontiers_grow_and_orchard_activates_at_nu5` one pool later.
        for (net, table) in [
            (Network::Main, data::MAINNET_TREESTATES),
            (Network::Test, data::TESTNET_TREESTATES),
        ] {
            let activation = ironwood_activation(net);
            let mut prev: Option<(u32, u64)> = None;
            let mut carried = 0usize;
            for &(height, hash, sapling, orchard, ironwood) in table {
                // Absent below the activation, PRESENT from it onward. Below the
                // activation an absent frontier is CORRECT — the pool does not
                // exist — which is exactly why "absent" cannot be treated as an
                // error anywhere downstream, and why the boundary has to be
                // asserted here instead.
                if height < activation {
                    assert_eq!(
                        ironwood, "",
                        "{net:?} row {height}: ironwood frontier must be ABSENT below the \
                         NU6.3 activation {activation}"
                    );
                    continue;
                }
                assert_ne!(
                    ironwood, "",
                    "{net:?} row {height}: ironwood frontier must be PRESENT at/after the \
                     NU6.3 activation {activation} — an absent field decodes to an EMPTY \
                     tree with no error, so this is the only place it can be caught"
                );

                let ts = TreeState {
                    network: chain_name(net).to_owned(),
                    height: u64::from(height),
                    time: 0,
                    hash: hash.to_owned(),
                    sapling_tree: sapling.to_owned(),
                    orchard_tree: orchard.to_owned(),
                    ironwood_tree: ironwood.to_owned(),
                };
                let b = crate::account::birthday_from_treestate(ts)
                    .unwrap_or_else(|_| panic!("{net:?} bundled row {height} must decode"));
                let size = b.prior_chain_state().final_ironwood_tree().tree_size();

                // A decoded, NON-EMPTY tree — EXCEPT where the chain itself is
                // legitimately empty. `"000000"` is the PRESENT-BUT-EMPTY encoding,
                // and it is the honest chain value between the activation and the
                // pool's first note: measured against zec.rocks 2026-09-07
                // (`docs/plan/probes/treestate-field7-probe.output.txt`), mainnet
                // field 7 is `"000000"` at exactly 3,428,143 and non-empty from
                // 3,428,144; testnet is `"000000"` through 4,134,682 and non-empty
                // from 4,134,683 — 683 legitimate heights.
                //
                // No BUNDLED row sits in either window today (that is what
                // `..._samples_no_row_between_ironwood_activation_and_the_reimport_threshold`
                // asserts), so this arm is not exercised. It is here because the
                // alternative is a test that FALSE-FAILS a correct bundle refresh
                // with the message "decoded to an EMPTY tree" — reading as data
                // loss when the data is right. The assertion that a blanked or
                // dropped column trips is the `assert_ne!(ironwood, "")` above; this
                // one catches a frontier that is present, non-empty as text, and
                // decodes to nothing.
                const EMPTY_TREE_ENCODING: &str = "000000";
                assert!(
                    size > 0 || ironwood == EMPTY_TREE_ENCODING,
                    "{net:?} row {height}: ironwood frontier decoded to an EMPTY tree \
                     from non-empty hex — the encoding says data, the tree says none"
                );
                // Note commitment trees are APPEND-ONLY — a later checkpoint can
                // never describe a smaller tree.
                if let Some((ph, psize)) = prev {
                    assert!(
                        size >= psize,
                        "{net:?} ironwood tree SHRANK {ph} ({psize}) -> {height} ({size})"
                    );
                }
                prev = Some((height, size));
                carried += 1;
            }

            // Anti-vacuity with a MEASURED count, not a lower bound that any
            // non-empty run satisfies: the upstream bundle at commit
            // `3819943…` carries the field on exactly 13 mainnet rows
            // (3429810…3459780) and 25 testnet rows (4134750…4301840). A refresh
            // that adds rows moves these numbers, and moving them is a decision
            // — CHECKPOINTS.md records the counts for the same reason.
            let expected = match net {
                Network::Main => 13,
                Network::Test => 25,
            };
            assert_eq!(
                carried, expected,
                "{net:?}: {carried} rows carry an ironwood frontier, expected {expected} — \
                 if a bundle refresh really added rows, update this count and CHECKPOINTS.md \
                 together; do not relax the assertion"
            );
        }
    }

    #[test]
    fn treestate_bundle_frontiers_chain_to_their_predecessors() {
        // The STRONGEST purely-offline check available, and the one that binds a
        // refresh's NEW rows to the data we already audited: a completed subtree
        // root is immutable once its leaf range is full, so wherever two
        // frontiers share a completed subtree, the stored hash MUST be identical.
        // A forged row cannot satisfy this without a Sinsemilla/BLAKE2s collision.
        //
        // Frontier layout: for a leaf at position `p`, an ommer exists at level
        // `k` exactly when bit `k` of `p` is set, and it is the root of the
        // complete level-`k` subtree with index `(p >> k) - 1`. Two rows share
        // that subtree when the level AND the index agree.
        //
        // HONEST BOUND, measured rather than asserted. Both mutants were planted
        // and run:
        //   CAUGHT — a nibble flipped in the HIGHEST-level ommer of an interior
        //     row (mainnet 2660000): the shared completed subtree stops matching
        //     its neighbours on both sides.
        //   NOT CAUGHT — the same flip in a LOW-level ommer of the TAIL row
        //     (3459780): the tail has no successor, and its low levels are not
        //     shared with a predecessor thousands of notes back.
        // So this constrains completed subtrees that two bundled rows actually
        // share, which is most of the interior and progressively less of each
        // frontier's newest end. It closes the careless and accidental classes;
        // the trust root remains provenance (CHECKPOINTS.md), and the sibling
        // `..._frontiers_grow_and_orchard_activates_at_nu5` covers the blanked
        // and shrunken-tree classes this one is blind to.
        // `(position, ommers)` is all this check needs, and taking it as plain
        // data keeps the helpers off the `incrementalmerkletree` type names —
        // that crate is a transitive dep we do not name directly anywhere else.
        fn ommers_by_level<H: Clone>(f: Option<(u64, Vec<H>)>) -> Vec<(u8, u64, H)> {
            let Some((p, ommers)) = f else {
                return Vec::new();
            };
            let mut out = Vec::new();
            let mut ommers = ommers.into_iter();
            let mut k = 0u8;
            while (p >> k) != 0 {
                if ((p >> k) & 1) == 1 {
                    match ommers.next() {
                        Some(h) => out.push((k, (p >> k) - 1, h)),
                        None => break,
                    }
                }
                k += 1;
            }
            out
        }

        fn compare<H: Clone + PartialEq + core::fmt::Debug>(
            pool: &str,
            net: Network,
            prev_h: u32,
            cur_h: u32,
            a: Option<(u64, Vec<H>)>,
            b: Option<(u64, Vec<H>)>,
        ) -> usize {
            let (pa, pb) = (ommers_by_level(a), ommers_by_level(b));
            let mut shared = 0;
            for (ka, ia, ha) in &pa {
                for (kb, ib, hb) in &pb {
                    if ka == kb && ia == ib {
                        assert_eq!(
                            ha, hb,
                            "{net:?} {pool}: completed subtree (level {ka}, index {ia}) \
                             differs between rows {prev_h} and {cur_h} — a completed \
                             subtree root is immutable, so one of these rows is wrong"
                        );
                        shared += 1;
                    }
                }
            }
            shared
        }

        let mut total_shared = 0usize;
        // PER-NETWORK, not global. A single counter summed over both networks has
        // 40 shared roots in it, of which mainnet alone contributes enough to clear
        // any global floor — so a testnet-only column regression would slip through
        // while the comment claimed the counter "goes to zero".
        let mut ironwood_shared: Vec<(Network, usize)> = Vec::new();
        for (net, table) in [
            (Network::Main, data::MAINNET_TREESTATES),
            (Network::Test, data::TESTNET_TREESTATES),
        ] {
            let mut net_ironwood_shared = 0usize;
            let decode = |height: u32, hash: &str, sapling: &str, orchard: &str, iw: &str| {
                crate::account::birthday_from_treestate(TreeState {
                    network: chain_name(net).to_owned(),
                    height: u64::from(height),
                    time: 0,
                    hash: hash.to_owned(),
                    sapling_tree: sapling.to_owned(),
                    orchard_tree: orchard.to_owned(),
                    ironwood_tree: iw.to_owned(),
                })
                .unwrap_or_else(|_| panic!("{net:?} bundled row {height} must decode"))
            };
            for pair in table.windows(2) {
                let (ph, phash, psap, porch, piw) = pair[0];
                let (ch, chash, csap, corch, ciw) = pair[1];
                let (pb, cb) = (
                    decode(ph, phash, psap, porch, piw),
                    decode(ch, chash, csap, corch, ciw),
                );
                let (pcs, ccs) = (pb.prior_chain_state(), cb.prior_chain_state());
                let sap = |cs: &zcash_client_backend::data_api::chain::ChainState| {
                    cs.final_sapling_tree()
                        .value()
                        .map(|n| (u64::from(n.position()), n.ommers().to_vec()))
                };
                let orch = |cs: &zcash_client_backend::data_api::chain::ChainState| {
                    cs.final_orchard_tree()
                        .value()
                        .map(|n| (u64::from(n.position()), n.ommers().to_vec()))
                };
                let iron = |cs: &zcash_client_backend::data_api::chain::ChainState| {
                    cs.final_ironwood_tree()
                        .value()
                        .map(|n| (u64::from(n.position()), n.ommers().to_vec()))
                };
                total_shared += compare("sapling", net, ph, ch, sap(pcs), sap(ccs));
                total_shared += compare("orchard", net, ph, ch, orch(pcs), orch(ccs));
                // SEPARATE counter, deliberately. Ironwood is empty on all but
                // 13 mainnet / 25 testnet rows, and an empty frontier decodes to
                // `None`, which `ommers_by_level` turns into an empty vector —
                // so the nested match never fires and the arm passes comparing
                // nothing to nothing. Folded into `total_shared` (already in the
                // tens of thousands from the two older pools) that vacuity would
                // be invisible: the new column could be entirely absent and this
                // test would still be green.
                net_ironwood_shared += compare("ironwood", net, ph, ch, iron(pcs), iron(ccs));
            }
            ironwood_shared.push((net, net_ironwood_shared));
        }
        // Anti-vacuity, and the whole point: this test is worthless if the
        // nested match never fires. A pass MUST mean thousands of real hash
        // comparisons happened.
        assert!(
            total_shared > 10_000,
            "only {total_shared} shared subtree roots compared — the check is not engaging"
        );
        // The Ironwood pool is young, so its bundled frontiers share far fewer
        // completed subtrees than sapling/orchard — but "far fewer" is not
        // "none". Measured on this bundle by raising the floor to 99,999 and
        // reading the failure: **mainnet 10, testnet 30**, 40 together. The floor
        // is asserted PER NETWORK, and the split is why: a single global floor of
        // 20 is cleared by TESTNET ALONE, so a MAINNET-only column regression —
        // the money network — would have sat green under it. 5 is half of the
        // smaller measurement, so both sides keep real headroom.
        for (net, shared) in &ironwood_shared {
            assert!(
                *shared >= 5,
                "{net:?}: only {shared} shared IRONWOOD subtree roots compared — the \
                 Ironwood column is missing for this network, or its rows stopped chaining"
            );
        }
    }

    #[test]
    fn bundled_treestate_picks_newest_row_below_birthday() {
        // §8 (the never-skip-funds money property): the chosen row is the NEWEST
        // with height < requested, so birthday (= height+1) ≤ requested — we start
        // at or below the requested height, never above it.
        //
        // AMENDED 2026-09-06 (`ironwood-nu63-support.md` §1.5): "newest below
        // requested" is now "newest below min(requested, ANCHOR CEILING)". A row
        // above the newest activation this build knows is never selected, because
        // the chain state there can contain a pool we cannot model and the decoded
        // state is immutable after first import. The money property is UNCHANGED —
        // a lower anchor over-scans, which is the safe direction; only the
        // "newest" clause is now bounded.
        for net in [Network::Main, Network::Test] {
            let table = treestate_table(net);
            let ceiling = crate::consensus::newest_known_activation(&net.consensus())
                .map_or(u32::MAX, u32::from);
            // a requested height strictly inside the modern tail (between rows)
            let probe_idx = table.len() - 3;
            let requested = table[probe_idx].0 + 1; // just above row `probe_idx`
            let ts = bundled_treestate(net, BlockHeight::new(requested)).expect("a row");
            let picked = u32::try_from(ts.height).expect("height fits u32");
            assert!(picked < requested, "{net:?}: picked height < requested");
            assert!(
                picked <= ceiling,
                "{net:?}: picked {picked} is above the anchor ceiling {ceiling} — a build \
                 must never anchor where it cannot model the consensus rules"
            );
            // it is the NEWEST such row, BOUNDED by the ceiling: the next row is
            // either at/above `requested` or above the ceiling.
            let idx = table
                .iter()
                .position(|&(h, ..)| h == picked)
                .expect("picked is a real row");
            if idx + 1 < table.len() {
                let next = table[idx + 1].0;
                assert!(
                    next >= requested || next > ceiling,
                    "{net:?}: row {next} was skipped and it is neither above the request \
                     nor above the ceiling"
                );
            }
            // and the decoded birthday never exceeds requested (the money property)
            let b = crate::account::birthday_from_treestate(ts).expect("decode");
            assert!(
                u64::from(b.height()) <= u64::from(requested),
                "{net:?}: birthday never above the requested height"
            );
        }
    }

    #[test]
    fn bundled_treestate_floors_to_activation() {
        // §8 (the floor edge): a requested height at/below activation resolves to
        // the activation row, never below (no shielded notes exist below).
        for net in [Network::Main, Network::Test] {
            let act = activation_height(net);
            for requested in [act, act - 1] {
                let ts = bundled_treestate(net, BlockHeight::new(requested)).expect("a row");
                assert_eq!(
                    u32::try_from(ts.height).expect("height fits u32"),
                    act,
                    "{net:?}: requested {requested} floors to the activation row"
                );
            }
        }
    }

    // ── T0-1 · the anchor block (B1, B2, B4) ─────────────────────────────────
    //
    // These are the CI-time ceiling invariant the maintainer decided on
    // (`production-readiness-phase-1.md` §4, B-BLOCK STATUS) — NOT a runtime
    // refusal, which the same section supersedes.
    //
    // THE PREDICATE, NAMED (B2 requires it be named): an Ironwood frontier is
    // **PRESENT** iff the raw `ironwood_tree` field is a non-empty string.
    // `"000000"` is PRESENT (it is the honest present-but-empty chain value for
    // the heights between the activation and the pool's first note — measured,
    // `probes/treestate-field7-probe.output.txt`); `""` is ABSENT.
    //
    // It is deliberately NOT "decodes to a tree with `tree_size() > 0`". That
    // predicate calls `"000000"` empty, and `"000000"` is exactly what a correct
    // server serves at the activation height — the one height the anchor ceiling
    // makes reachable. A decoded-emptiness guard would therefore false-fail a
    // correct bundle refresh at the only height it can currently fire on, which
    // is the "my remedy was worse than the bug" shape this whole B block was
    // re-decided to avoid. The hazard B1 exists for is a DROPPED/blanked column,
    // and a dropped column is `""`.
    const IRONWOOD_FRONTIER_EMPTY_TREE_ENCODING: &str = "000000";

    /// B1's predicate, as a function, so B2 can drive the SAME code path the
    /// guard uses rather than a restatement of it (IT-10: a negative case that is
    /// refused by something other than the mechanism it names cannot see that
    /// mechanism break).
    fn ironwood_frontier_is_present(raw: &str) -> bool {
        !raw.is_empty()
    }

    /// Every requested height worth driving `bundled_treestate` with, for one
    /// network: every bundled row and its immediate neighbours, the Ironwood
    /// activation and its neighbours, and `u32::MAX` (the fresh-create case that
    /// floors onto the newest anchorable row).
    fn anchor_probe_heights(net: Network) -> Vec<u32> {
        let mut hs: Vec<u32> = Vec::new();
        for &(h, ..) in treestate_table(net) {
            hs.extend([h.saturating_sub(1), h, h.saturating_add(1)]);
        }
        let act = ironwood_activation(net);
        hs.extend([act.saturating_sub(1), act, act.saturating_add(1), u32::MAX]);
        hs.sort_unstable();
        hs.dedup();
        hs
    }

    #[test]
    fn bundled_treestate_never_anchors_an_empty_ironwood_frontier_above_activation() {
        // T0-1 B1 + B4. THE CEILING INVARIANT, asserted over the WHOLE bundle on
        // BOTH networks — not a sample, and driven through the real
        // `bundled_treestate` rather than over the table, because the table is
        // already covered by
        // `treestate_bundle_carries_an_ironwood_frontier_after_activation` and
        // that test CANNOT see the regression this one is aimed at: a
        // `bundled_treestate` whose constructor hard-codes `ironwood_tree:
        // String::new()` (the exact regression `bundled_treestate`'s own comment
        // at :220-227 warns about) reads the same 5-ary table and would leave
        // that test green.
        //
        // TWO ARMS, and the second is why this is not a tautology.
        //
        //   ARM 1 — the invariant. For every height a caller can ask for, if the
        //   selected row is ABOVE the Ironwood activation then its frontier must
        //   be PRESENT. This is the assertion that starts biting the moment the
        //   anchor ceiling rises.
        //
        //   ARM 2 — the reachability record. Today the ceiling
        //   (`newest_known_activation + 1` = Nu6_3's 3,428,143 + 1) is at or
        //   below the activation, so NO above-activation row is reachable and arm
        //   1 is vacuously true. That is a fact about the ceiling, not about the
        //   data, so it is asserted as a MEASURED count rather than left implicit
        //   — the same discipline `..._carries_an_ironwood_frontier_after_
        //   activation` uses for its 13/25. When somebody raises the ceiling past
        //   3,429,810 this count changes and THIS TEST FIRES, in CI, which is the
        //   hazard the B block exists for. It is a decision point, not a
        //   false failure: the message says what to do.
        //
        //   ARM 3 — pre-clearance of the rows the ceiling currently excludes, so
        //   that a bundle refresh which blanks a future-reachable row is caught
        //   NOW rather than on the day the ceiling moves. Without this, arms 1+2
        //   are a pair of true statements that never touch the data.
        for net in [Network::Main, Network::Test] {
            let activation = ironwood_activation(net);
            let ceiling = crate::consensus::newest_known_activation(&net.consensus())
                .map_or(u32::MAX, |h| u32::from(h).saturating_add(1));

            // ARM 1
            let mut reachable_above_activation = 0usize;
            for requested in anchor_probe_heights(net) {
                let Some(ts) = bundled_treestate(net, BlockHeight::new(requested)) else {
                    // `None` is the fail-closed post-condition (a mis-sorted
                    // table) — it anchors nothing, so it cannot anchor wrongly.
                    continue;
                };
                let picked = u32::try_from(ts.height).expect("a bundled height fits u32");
                if picked <= activation {
                    continue;
                }
                reachable_above_activation += 1;
                assert!(
                    ironwood_frontier_is_present(&ts.ironwood_tree),
                    "{net:?}: requesting {requested} anchors on row {picked}, which is ABOVE \
                     the Ironwood activation {activation}, with an ABSENT ironwood frontier. \
                     `TreeState::ironwood_tree()` decodes an absent field to an EMPTY tree \
                     with no error, so a wallet born here would carry a third frontier the \
                     chain does not have — wrong witnesses for real funds, silently."
                );
            }

            // ARM 2 — the measured reachability, and the ceiling-raise tripwire.
            assert_eq!(
                reachable_above_activation, 0,
                "{net:?}: {reachable_above_activation} anchorable row(s) now sit ABOVE the \
                 Ironwood activation {activation} (ceiling {ceiling}). Nothing is necessarily \
                 wrong — arm 1 above is now doing real work instead of being vacuous — but \
                 this is the ceiling raise B1 was written to fire on. Re-verify the bundle's \
                 above-activation rows, then update this expected count deliberately."
            );
            assert!(
                ceiling <= activation.saturating_add(1),
                "{net:?}: the anchor ceiling {ceiling} is above the Ironwood activation \
                 {activation} + 1, so the zero above-activation count asserted just now is \
                 not explained by the ceiling. Something else is excluding those rows and \
                 this test no longer knows what."
            );

            // ARM 3 — the excluded rows, pre-cleared.
            let mut excluded_above = 0usize;
            for &(h, _, _, _, ironwood) in treestate_table(net) {
                if h <= activation {
                    continue;
                }
                excluded_above += 1;
                assert!(
                    ironwood_frontier_is_present(ironwood),
                    "{net:?} row {h}: above the Ironwood activation {activation} with an \
                     ABSENT frontier. The anchor ceiling makes it unreachable TODAY; raising \
                     the ceiling would make this row anchorable and wrong."
                );
            }
            assert!(
                excluded_above > 0,
                "{net:?}: the bundle carries no row above the Ironwood activation \
                 {activation}, so arm 3 asserted nothing"
            );
        }
    }

    #[test]
    fn an_empty_ironwood_frontier_at_or_below_the_activation_is_legitimate() {
        // T0-1 B2 + B4. THE NEGATIVE CASE, and the reason the B block was
        // re-decided: the accept path must stay an accept. Both encodings the
        // field-7 probe recorded are covered — `""` (ABSENT, what every
        // pre-activation row carries) and `"000000"` (PRESENT-but-decodes-empty,
        // what a correct server serves at exactly the activation height and, on
        // testnet, for 683 heights above it).
        //
        // IT-10 — WHICH MECHANISM REFUSES THIS? Two, and both are exercised
        // rather than restated:
        //   (a) `ironwood_frontier_is_present`, the predicate B1's guard calls.
        //       If it were tightened to decoded-emptiness, `"000000"` would flip
        //       to ABSENT and the sub-assertion below fails. That is the whole
        //       point of covering both strings: the two candidate predicates
        //       DISAGREE on `"000000"` and agree on `""`.
        //   (b) `bundled_treestate` itself + `birthday_from_treestate`: a real
        //       at/below-activation request must still return a usable birthday.
        //       If a future version of this check became a refusal, this is where
        //       it would show.
        for net in [Network::Main, Network::Test] {
            let activation = ironwood_activation(net);

            // (a) the predicate, on both encodings, directly.
            assert!(
                !ironwood_frontier_is_present(""),
                "an absent field is ABSENT — this is the encoding a dropped column produces"
            );
            assert!(
                ironwood_frontier_is_present(IRONWOOD_FRONTIER_EMPTY_TREE_ENCODING),
                "`{IRONWOOD_FRONTIER_EMPTY_TREE_ENCODING}` is PRESENT: it is the honest \
                 chain value between the Ironwood activation and the pool's first note. A \
                 predicate that calls it absent flags a correct bundle."
            );
            // and the two encodings really do decode to the SAME empty tree, which
            // is what makes raw-emptiness and decoded-emptiness disagree.
            for raw in ["", IRONWOOD_FRONTIER_EMPTY_TREE_ENCODING] {
                let ts = TreeState {
                    network: chain_name(net).to_owned(),
                    height: u64::from(activation),
                    hash: "00".repeat(32),
                    time: 0,
                    sapling_tree: String::new(),
                    orchard_tree: String::new(),
                    ironwood_tree: raw.to_owned(),
                };
                let b = crate::account::birthday_from_treestate(ts)
                    .expect("an at-activation treestate with an empty ironwood pool decodes");
                assert_eq!(
                    b.prior_chain_state().final_ironwood_tree().tree_size(),
                    0,
                    "{net:?}: `{raw}` must decode to an EMPTY ironwood tree — if this ever \
                     stops holding, the two predicates no longer disagree and B2's fixture \
                     has stopped testing anything"
                );
            }

            // (b) the real function, at and below the activation, still anchors.
            let mut checked = 0usize;
            for requested in anchor_probe_heights(net) {
                if requested > activation {
                    continue;
                }
                let ts = bundled_treestate(net, BlockHeight::new(requested))
                    .unwrap_or_else(|| panic!("{net:?}: requesting {requested} must anchor"));
                let picked = u32::try_from(ts.height).expect("a bundled height fits u32");
                assert!(
                    picked <= activation,
                    "{net:?}: requesting {requested} (at/below the activation) picked \
                     {picked}, above it"
                );
                // The accept: an empty frontier here must NOT be treated as a fault.
                crate::account::birthday_from_treestate(ts).unwrap_or_else(|_| {
                    panic!(
                        "{net:?}: row {picked} is at/below the Ironwood activation \
                         {activation}, where an empty ironwood frontier is CORRECT — it must \
                         still produce a usable birthday, not a refusal"
                    )
                });
                checked += 1;
            }
            assert!(
                checked > 0,
                "{net:?}: no at/below-activation request was exercised — B2's accept arm is \
                 vacuous"
            );
        }
    }

    /// `n` rendered the way this file's prose renders heights (`3,428,143`).
    /// Built at RUNTIME on purpose: writing the comma form as a literal would put
    /// the needle into the very file the scan reads, and the check would fire on
    /// its own artifact.
    fn grouped(n: u32) -> String {
        let s = n.to_string();
        let mut out = String::new();
        for (i, c) in s.chars().enumerate() {
            // ADJUDICATOR: `% 3 == 0` → `is_multiple_of(3)`; the tree builds with
            // `clippy -D warnings` and `manual_is_multiple_of` is on. Mechanical.
            if i > 0 && (s.len() - i).is_multiple_of(3) {
                out.push(',');
            }
            out.push(c);
        }
        out
    }

    #[test]
    fn the_anchor_ceiling_is_narrated_with_the_height_it_actually_has() {
        // UNLISTED CASE — not an A/B assertion row. `production-readiness-phase-1.md`
        // §4 records it as prose ("Owed, in the same change as whatever is
        // chosen"): this file's ceiling narration still names Nu6_2's height,
        // which `consensus.rs`'s `KNOWN_UPGRADES` note corrects to Nu6_3's. The
        // whole B-block argument rests on that number and it is stated two ways
        // in two files. Owed prose goes stale again; this binds the sentence to
        // the computed value.
        //
        // SCOPED TO THE PRODUCTION PREFIX (everything above the first
        // `#[cfg(test)]`), for two reasons. It is the half that a reader of the
        // shipped crate sees, and — the reason that actually bites — this test's
        // own comment is allowed to discuss the stale height without the check
        // accusing itself.
        let params = Network::Main.consensus();
        let ceiling = crate::consensus::newest_known_activation(&params)
            .map(u32::from)
            .expect("mainnet params know a newest upgrade");
        let src = include_str!("mod.rs");
        let marker = "\n#[cfg(test)]\n";
        let cut = src
            .find(marker)
            .expect("this file has a #[cfg(test)] module");
        let production = &src[..cut];
        // Anti-vacuity: prove the prefix really is the production half and not an
        // early truncation that would make any scan over it pass by construction.
        assert!(
            production.contains("pub(crate) fn bundled_treestate"),
            "the production prefix stops before `bundled_treestate` — this scan is \
             reading the wrong region and cannot see the narration at all"
        );

        // Every network upgrade this build knows about, rendered as prose. Any of
        // them that is NOT the ceiling and appears in the production prose is a
        // ceiling claim that has gone stale — the exact drift the contract owes.
        let ceiling_text = grouped(ceiling);
        for nu in [
            zcash_protocol::consensus::NetworkUpgrade::Nu6,
            zcash_protocol::consensus::NetworkUpgrade::Nu6_1,
            zcash_protocol::consensus::NetworkUpgrade::Nu6_2,
        ] {
            use zcash_protocol::consensus::Parameters;
            let Some(h) = params.activation_height(nu) else {
                continue;
            };
            let h = u32::from(h);
            if h == ceiling {
                continue;
            }
            let stale = grouped(h);
            assert!(
                !production.contains(&stale),
                "the ceiling narration in this file still names {stale} ({nu:?}), but the \
                 anchor ceiling this build computes is {ceiling_text}. The B-block argument \
                 rests on that number and it is currently stated two ways in two files. Fix \
                 the sentence; do not relax this test."
            );
        }
    }

    proptest! {
        /// The fund-safety property over arbitrary times: the returned height is
        /// always a REAL checkpoint height, and its block-time is ≤ the query
        /// (or it is the activation clamp) — NEVER a checkpoint dated after the
        /// asked-for time, so the birthday can never overshoot true creation.
        #[test]
        fn estimate_never_returns_a_checkpoint_after_the_query(secs in any::<u64>()) {
            for (net, table) in [
                (Network::Main, data::MAINNET),
                (Network::Test, data::TESTNET),
            ] {
                let h = estimate_birthday(net, secs).value();
                let (_, t) = *table
                    .iter()
                    .find(|&&(ch, _)| ch == h)
                    .expect("estimate returns a real checkpoint height");
                prop_assert!(t <= secs || h == table[0].0);
            }
        }
    }
}
