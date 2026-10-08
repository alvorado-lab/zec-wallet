//! **T0-1a — the A11 height bind.** What the wallet knows about subtree
//! completion heights that the endpoint cannot forge, and how it holds a served
//! sequence to it.
//!
//! # The gap this closes
//!
//! `put_shard_roots` lands `{prefix}_tree_shards.subtree_end_height` through an
//! unconditional `ON CONFLICT (shard_index) DO UPDATE SET subtree_end_height =
//! :subtree_end_height` (`zcash_client_sqlite-0.22.0/src/wallet/commitment_tree.rs:1296-1301`),
//! and the shardtree's own scan write-back (`put_shard`, `:570-586`) omits that
//! column from BOTH its INSERT and its `DO UPDATE`. **The endpoint is therefore
//! the sole writer of that number, in all three pools.** `sync::validate_root_sequence`
//! bounds only its SHAPE — strictly increasing, at or above the pool activation, at
//! or below the endpoint's own reported tip — and every one of those bounds is
//! either a public constant the attacker already knows or a number the same
//! endpoint supplied on the same pass. Measured, not argued:
//! `docs/plan/probes/ironwood-a11-bypass.{py,output.txt}` drives a compressed
//! sequence through the pinned crate's own SQL and lands `witness_stabilized = 1`
//! on a note whose witness can never be constructed.
//!
//! # What this module does NOT do, deliberately
//!
//! **No per-index `root_hash` comparison.** That dimension is upstream's and it
//! works: `put_shard_roots` inserts every root into `{prefix}_tree_cap` with
//! `Retention::Reference` (`commitment_tree.rs:1272-1281`), which is non-prunable,
//! so a differing leaf at a recorded address is a shardtree `Conflict` that
//! `sync::map_shardtree_err` already surfaces — and this tree has a passing named
//! test for it (`sync::tests::mutated_root_reput_is_insert_conflict_not_silent_overwrite`).
//! A second, weaker copy in SDK glue is a Principle-2 duplication. **The gap is
//! the height column only**, and that is all this module touches.
//!
//! # Where the bind's memory lives (T0-1a question 1)
//!
//! **A composition of three sources, none of which is a new schema of ours.** The
//! handoff proposed a shadow aux table on the ground that `WalletDb.conn` is
//! private; that is true of `WalletDb` and does not settle the question, because
//! this SDK already holds a SECOND SQLCipher-keyed connection to the SAME file
//! (`store::OpenWallet::aux_db`, `db::open_existing_keyed_connection`), and two of
//! the three sources below do not need a connection at all.
//!
//! | | source | binds | price paid |
//! |---|---|---|---|
//! | (b) | upstream's own `{prefix}_tree_shards.subtree_end_height`, read raw through `aux_db` | a height already recorded at that index | reads an UNDOCUMENTED internal table (unlike the `v_transactions` view `history.rs` reads on the same connection). Mitigated by the exact pin `zcash_client_sqlite = "=0.22.0"` (`sdk/Cargo.toml:76`) and by [`read_recorded_heights`] failing CLOSED on a missing table or column |
//! | (c) | the bundled treestates in the signed binary (`checkpoints::treestate_table`) | how many subtrees were complete at 13 mainnet / 25 testnet known heights — offline, no RPC, no scanned blocks | binds only up to the newest bundled row; composes with (b)/(d) above it |
//! | (d) | `blocks.{prefix}_commitment_tree_size`, a number this wallet COUNTED while scanning | the same, at the newest block this wallet actually scanned | abstains on a wallet that has scanned nothing, and on pre-Ironwood rows where the column is NULL |
//!
//! **What the composition BUYS (IT-1b).** No second copy of anything, so there is
//! no staleness, no split-atomicity between two connections, and — the question the
//! shadow-table design could not answer — **nothing new to clear.** See
//! [`self`]'s "clearing" section below.
//!
//! **What it LOSES.** (b) is silent exactly where INC-020's own population sits: a
//! wallet mid-heal has Ironwood shard ROWS (written by `put_shard` during scanning)
//! with a NULL `subtree_end_height`, so there is nothing recorded to compare
//! against on the very pass that matters most. That is not a flaw in (b), it is the
//! reason (c) and (d) are load-bearing rather than decoration, and it is why this
//! module refuses to describe (b) as "the" bind.
//!
//! # The rewind gate — what makes (b), (d) and the hash check answerable at all
//!
//! **T0-1a-R, §4e owed row 1.** Three of the four oracles above are the wallet's own
//! MEMORY of a chain, and a reorg is the event that makes a memory wrong. The first
//! build of this module answered that with a ±`REORG_MAX_BLOCKS` tolerance on the
//! recorded heights, which was wrong twice over: it was UNCONDITIONAL (an endpoint
//! got the window on every pass, forever, with no rewind required, which turns
//! "equality-when-recorded" into "drift-when-recorded") and it answered by the SIZE
//! of the move when the question is whether a rewind happened at all. It also was
//! not enough — the proof's own C15 measured a legitimate post-rewind move of 5,149
//! blocks.
//!
//! **So the wallet keeps one number of its own** ([`REWIND_TABLE`]): how many chain
//! rewinds it has performed, and what that count stood at when each pool's heights
//! were last recorded. [`note_rewind`] increments it from `sync::scan_batch`'s
//! continuity arm and, since an earlier revision, `sync_once`'s anchor reconcile — both through the
//! one `sync::rewind_wallet_to` body that runs `truncate_to_height`, so the evidence
//! is written by the thing that creates the staleness and by nothing else —
//! and [`unnote_rewind`] withdraws that increment when the truncate returns `Err`, so
//! the count is of rewinds that HAPPENED and not of ones attempted (T0-1a-R2, the
//! §4g table below). [`note_roots_recorded`] consumes it, per pool, immediately after
//! that pool's roots are actually WRITTEN.
//!
//! **THE DOOR THIS FLOOR DOES NOT CLOSE, found while building it and reported rather
//! than left for a later reader.** The direction floor defends an index that CARRIES a
//! recorded height. On a wallet with none — Ironwood mid-heal, the INC-020 population
//! this whole item exists for, whose shard rows hold NULL until `put_shard_roots`
//! first runs — [`check_recorded_heights`] abstains at every index by its own explicit
//! branch, and the only arms left are [`check_completion_gap`] and the bundled
//! frontiers. **Both admit the adjudicator's tail inflation**, and that is asserted
//! rather than argued in
//! `tests::a_rewind_does_not_license_the_tail_inflation_the_adjudicator_drove`:
//! `[3451206, 3475299, 3475399, 3475499]` is spaced 100 blocks apart, so the gap floor
//! has nothing to say, and it places index 0 INSIDE the window the newest bundled row
//! `(3_459_780, 1)` allows, so the counting arm passes it too. So the same sequence
//! that now needs an induced rewind on a wallet with a record is accepted **on the
//! first pass, with no rewind at all**, on a wallet without one.
//! `an_inflated_height_sequence_is_refused_on_first_contact` does not see this: its
//! fixture inflates index 0 as well, which is what the bundled arm catches.
//! This is §4b owed row 7 — *nothing binds above the newest bundled row* — arriving
//! through the inflation door rather than the index-shift one.
//!
//! **RE-SCOPED BY BIND-1 (§4x), and the sentence that stood here is now too large.**
//! It read: *"it is OPEN. Closing it needs an oracle that reaches above 3,459,780,
//! which this build does not have."* The first half is still true AT INGEST — every
//! oracle above binds a sequence the endpoint is OFFERING, and above the newest
//! bundled row none of them reaches an index with no record. What is no longer true
//! is the second half: this build DOES have an oracle above that height, it is the
//! wallet's own scan, and it arrives LATER — [`reconcile_scanned_boundaries`] holds
//! the RECORDED row to the per-block counts the wallet made itself once the scan
//! crosses the blocks that decide it, corrects the row and undoes the stabilization
//! the wrong row licensed. So the honest statement is: **nothing binds above the
//! newest bundled row at INGEST, until the scan arrives** — an accepted lie above
//! that height is a WINDOW, not a permanent state.
//!
//! **The window's width is the residual, and it is real** (§4w (d)). Between the
//! write and the scan reaching that boundary, the lie stands: the next
//! `update_subtree_roots` equality-binds the endpoint to it (an honest re-serve
//! refused until the scan corrects it), a `ResetToSubtreeRoots` truncation would use
//! it (its consequence UNVERIFIED), and a lie at an index the scan has not reached —
//! above the frontier, which on a mid-heal wallet is most of them — is not examined
//! at all. An upward lie on a first-contact wallet is caught at the TRUE boundary,
//! which is after a truncation could already have used it.
//!
//! **Is the ledger still needed now that the relaxation is a DIRECTION floor rather
//! than an abstention? Asked, and answered by running rather than by arguing.** The
//! tempting simplification is to drop the gate and apply `served <= rec` on every
//! pass, which would delete the table, the `AUX_TABLES_PRESERVED` entry and the
//! rescan clear. Measured with that mutation in place: `sync_bind_proof::` goes from
//! **19 passed / 2 failed to 15 passed / 6 failed**, and the four new reds are exactly
//! the rows that say why — `a_recorded_height_cannot_be_walked_down_without_a_rewind`
//! (the walk is downward, so an ungated floor licenses every step of it),
//! `a_rewind_rebinds_rather_than_disabling_the_recorded_bind`,
//! `a_poisoned_bind_is_cleared_by_the_operation_the_contract_names` and
//! `a_height_violation_writes_nothing_for_that_pool`. **The ledger stays.** The
//! direction floor decides what a rewind may license; the ledger decides whether a
//! rewind happened at all, and without it every compression is licensed for free.
//!
//! ## The five calls §4e left to the implementer, and why each went the way it did
//!
//! | | decision | why, and what it cost |
//! |---|---|---|
//! | 1 · where the evidence lives, and what writes it | **A new aux table, [`REWIND_TABLE`], one integer per scope; written by [`note_rewind`] from `sync::scan_batch`'s continuity arm** | The pointer offered was "the SDK already counts reorgs and already watches `blocks` shrink". It does — `SyncPass.reorgs`, in memory, for the duration of ONE pass, and `blocks` shrinking is observable only WHILE it happens. **Neither survives to the next `update_subtree_roots`, and a DERIVED answer is impossible in principle: the rewind destroys its own evidence.** `blocks` above the target are deleted and the shard rows stay, so after the fact nothing on disk separates "rewound since recording" from "never scanned that far" — which is the ordinary state of every mid-heal wallet. An in-memory latch would also be lost across a process restart, and on mobile the gap between a rewind and the next root fetch routinely spans one. The price: this item, which had proudly added no schema, now adds one — with all of ADR-0534's obligations, discharged below |
//! | 2 · per-pool or wallet-wide | **RECORDED wallet-wide, CONSUMED per pool** | A rewind is one chain event and one `truncate_to_height`, so counting it per pool would be three copies of one fact. What is genuinely per-pool is the correction: an endpoint may serve Ironwood and not Sapling on the same pass, and the pool it did not serve still holds a stale record. Cost: four rows instead of one, and a reviewer has to hold two scopes in mind |
//! | 3 · abstain, or a window derived from `blocks.{prefix}_commitment_tree_size` | **NEITHER, and the question itself was wrong — §4e-R2 (c): a floor on DIRECTION.** ~~ABSTAIN~~ | **The first answer here was "abstain", and it was refuted by driving it.** The reasoning against the derived window still stands (it needs the tree size on BOTH sides of the rewind, the truncate deletes the pre-rewind side, and it is vacuous on exactly the mid-heal Ironwood wallet this item targets) — but §4e's premise that abstention was therefore *equally sufficient* was false: it assumed the counting oracles could carry the pass on a pool where they do not reach. `bundled_counts(Main, Ironwood)` stops at 3,459,780 and the scanned arm abstains on the low-frontier wallet the item is about, so abstaining left the recorded height — Ironwood's only oracle up there — saying nothing, and the adjudicator drove the tail-inflation attack through the gap. The floor now built needs **no new state**: `check_recorded_heights` already holds both numbers. See its own doc for the direction argument and the cost |
//! | 4 · is `REORG_MAX_BLOCKS` still referenced | **No — the `use` is gone and no code here reads it** (it survives only in the prose that records why it left). It was answering the wrong question — how far the height moved — and it was the mechanism the adjudication ruled unsound. It stays the SSOT for reorg depth everywhere else in the crate; it just has no business in a check about whether a reorg happened | |
//! | 5 · what clears the evidence | **The next ACCEPTED, NON-EMPTY write of that pool's roots** — because that write IS the corrected record — consumed PER POOL, immediately after that pool's own put lands (§4g 0b, decision 2 in the next table) — **and, wholesale, a rescan rebuild** ([`clear_rewind_watch`]) | This is what bounds the relaxation to one pass per rewind (R4). Deliberately NOT cleared for a pool that was served nothing, was refused, or whose own put failed — in each of those its stale record still stands uncorrected, so the relaxation the honest re-serve needs must survive. A pool whose put LANDED is consumed even when a later pool's put then fails: its record was corrected, and holding its relaxation open on another pool's fault was §4f owed row 0b |
//!
//! ## The three calls §4g left to the implementer (T0-1a-R2), and the matrix under them
//!
//! §4f owed rows 0a and 0b — the two wrap-crypto HIGHs — were both ORDERING defects
//! in the ledger's two write pairs, and both pairs straddle the two connections
//! (`db` for the truncate and for the puts, `aux_db` for the ledger), so neither
//! pair can ever be one transaction (§4g P3). What follows records the shape taken
//! for each and what every intermediate state resolves to.
//!
//! | | decision | why, and what it cost |
//! |---|---|---|
//! | 1 · 0a's shape — how the ledger says a rewind HAPPENED rather than was attempted | **Compensate: note before the truncate, UN-NOTE on the truncate's `Err`** — [`note_rewind`] → `db.truncate_to_height` → [`unnote_rewind`] on `Err`, in `sync::rewind_wallet_to` — the one body, reached from `scan_batch`'s continuity arm and (since an earlier revision) from `sync_once`'s anchor reconcile | The premise was read, not assumed: `WalletDb::truncate_to_height` is `self.transactionally(..)` (`zcash_client_sqlite-0.22.0/src/lib.rs:1881`), so an `Err` return is a rolled-back transaction and a wallet that did NOT rewind — withdrawing the note makes the ledger true again with **no new state** (R19). *Truncate first, note after* is refused by R13: a kill between them is rewound-but-unrecorded, the deadlock strand round 1 ordered against. *Intent, then resolve* was attacked and rejected — below the table. Cost: a THIRD write appears on the `Err` path, and its own fault is the double-fault row of the matrix |
//! | 2 · 0b's shape — when a pool's relaxation is consumed | **Per pool, IMMEDIATELY after that pool's own successful put**, in `sync::put_subtree_roots`'s pool order (Sapling → Orchard → Ironwood), through the `recorded` hook `Wallet::update_subtree_roots` supplies | The consume loop used to run after all three puts and only if all three returned `Ok`, so a later pool's failed put — endpoint-inducible: a mutated `root_hash` at a recorded index is a shardtree `Conflict` — left an earlier pool WRITTEN and still ARMED, and one rewind bought unbounded downward steps on every pool but the last. **No new lock discipline was needed**: the hook takes `aux_db` while `db` is held, the documented `db → aux_db` order and the order the bind takes, and takes it per pool rather than once, so `aux_db` is never held across a linear shard write (the S30 lock-hold caveat). §4g's premise that *both guards are already held for the closure's whole body* was not the code: `aux_db` was scoped to the bind and to the consume, and still is |
//! | 3 · the two-connection residuals | **Named tests, both:** `tests::a_fault_in_the_compensation_leaves_the_relaxation_armed_for_one_pass` and `tests::a_consume_that_fails_after_a_put_leaves_that_pool_armed_for_one_pass` | Each residual is ONE relaxed pass for the affected pool(s), consumed by the next accepted write, and — with 0a repaired — not re-armable by the endpoint except through a real rewind, which it could already induce. The tests inject the fault with `PRAGMA query_only`, which fails every write on the connection with `SQLITE_READONLY`, so the faulting write is the ledger's own and not a fixture's stand-in |
//!
//! **The kill/fault matrix for 0a — one row per cell §4g names, plus the one the
//! shape adds.** "Ledger" is `chain` relative to its pre-arm value `N`; "truth" is
//! whether the wallet rewound. A state is TRUE when the two agree.
//!
//! | cell | ledger | truth | lands on | why it is the safer side |
//! |---|---|---|---|---|
//! | kill AFTER the note, BEFORE the truncate | `N+1` | not rewound | **armed, no rewind — the direction this shape LOSES** | one relaxed pass on the next sync: (b) admits a downward move at a recorded index, (d) and (C6) abstain, the gap floor and the bundle still bind, and that pass's accepted write consumes it. The scan then re-detects the same continuity error and performs the real rewind, so the state also corrects itself. The other order would land this kill on rewound-but-unrecorded, whose only exit is a rescan (pass-fatal for Sapling/Orchard until then). One bounded relaxed pass against an unbounded deadlock. `tests::a_kill_between_the_note_and_the_truncate_lands_on_armed_and_is_bounded` |
//! | kill AFTER the truncate | `N+1` | rewound | **TRUE** | exactly what note-before buys, and why the naive reorder was refused |
//! | `Err` from the truncate — 0a's HIGH | `N+1`, then un-noted to `N` | not rewound | **TRUE** | upstream's transaction rolled back; the pass fails with the truncate's own classified fault (`ClassifyStoreFault`: `DiskFull` / `Io` / `StoreBusy`, else `StoreCorrupt` — `RequestedRewindInvalid` takes that default); the next bind reads the BINDING ledger. Before this repair the ledger stayed at `N+1` and was re-armed on every pass. `tests::a_truncate_that_fails_is_un_noted_and_the_ledger_binds_again` |
//! | fault in the second write | — | — | **this cell IS the row above**: under this shape the second write is the truncate, and its `Err` is what the compensation answers | — |
//! | fault in the THIRD write — the un-note itself, a double fault | `N+1` | not rewound | **armed, no rewind — bounded** | the truncate's fault is what the pass returns (the original error wins over the compensation's, the `db::copy_aux_tables` rollback convention) and the un-note's is logged at `warn`. The realistic double fault is `SQLITE_FULL`: a rolled-back truncate leaves its WAL frames on disk, so a disk the truncate just filled can refuse the one-page un-note too. The endpoint cannot cause it; what it costs is the kill cell's one relaxed pass. `tests::a_fault_in_the_compensation_leaves_the_relaxation_armed_for_one_pass` |
//!
//! **Intent-then-resolve, attacked as §4g asked, and why it was not taken.** Its
//! load-bearing sentence — *`update_subtree_roots` runs before any scan in a pass, so
//! at the next bind the rewind has not yet destroyed its own evidence* — is TRUE:
//! `Wallet::sync_once` awaits `update_subtree_roots` before its first `scan_batch`,
//! `sync::scan_batch` is the only production caller of `scan_cached_blocks`, and a
//! rescan clears the ledger outright. It is not where the shape fails. The shape
//! fails on the RESOLUTION predicate, which has to read the truncate's effect off
//! `blocks`: `select_truncation_height` (`zcash_client_sqlite-0.22.0/src/wallet.rs:4091`)
//! snaps to `MAX(blocks.height) <= requested` under per-pool tolerances, so the
//! height truncated to is not the target the intent would store; and when the
//! continuity error is detected [`crate::constants::REWIND_DISTANCE_BLOCKS`] or more
//! blocks ABOVE the scan frontier — an intra-batch `prev_hash` break, which the SDK's
//! downloader does not check and upstream's scan does, block by block — the
//! requested target sits at or above the last scanned block and the truncate is an
//! `Ok` **no-op that deletes no `blocks` row** (upstream's own contract: *"if the
//! requested height is greater than or equal to the height of the last scanned
//! block, this function does nothing"*). `blocks` then cannot separate "the truncate
//! ran" from "it did not" — the one question the intent exists to answer — in
//! exactly the geometry where the two differ. Add the ledger-shape change it costs
//! (R19: `AUX_TABLES_PRESERVED`, the rescan clear, ADR-0534) and a proof set whose
//! rows read the ledger, and a design that resolves the kill window correctly in one
//! geometry and not another is a worse trade than a compensated window that loses
//! one bounded relaxed pass in every geometry.
//!
//! **A consequence of that same `Ok` no-op, stated so nobody rediscovers it as a
//! defect of this repair.** The EVENT the ledger records is "`truncate_to_height`
//! returned `Ok`", and upstream defines that to include doing nothing. An endpoint
//! that serves an intra-batch break ten or more blocks above the frontier therefore
//! arms one relaxed pass without a single block being deleted. It gains nothing by it:
//! the same endpoint can serve a fork block AT the frontier and induce a real rewind
//! (the cost paragraph above, and the proof set's own `induce_rewind`), which arms
//! the same pass. It is the priced door, reached through a different frame, and not
//! a new one.
//!
//! **What is relaxed while a rewind stands unconsumed, and what is not.**
//!
//! | oracle | after an observed rewind | why |
//! |---|---|---|
//! | [`check_completion_gap`] | **still binds** | pure arithmetic over the served sequence; it reads no wallet state, so no reorg can make it stale |
//! | (c) [`bundled_counts`] | **still binds** | it lives in the signed binary and describes the chain, not this wallet's view of it |
//! | (b) [`check_recorded_heights`] | **relaxes in ONE DIRECTION — it does not abstain** (§4e-R2 (c)): a served height at or below the record is accepted, one above it is refused | `TreeTruncation::Unaffected` leaves the shard row in place across a rewind, so the recorded height is the PRE-reorg chain's answer and an honest endpoint now gives a different one — but only a DOWNWARD correction is consistent with a reorg the wallet must tolerate, and upward motion is the cap-erasure chain's necessary condition. The first repair abstained here, and the adjudicator drove the tail-inflation attack straight through it |
//! | (d) [`read_scanned_tree_size`] | abstains | `blocks` rows above the rewind target are deleted, but the rewind target is a heuristic ([`crate::constants::REWIND_DISTANCE_BLOCKS`] below the DETECTION height), so retained rows can still be from the abandoned chain — their declared tree size then describes a chain the wallet has left |
//! | (C6) [`check_completing_hashes`] | abstains | same table, same staleness: a retained `blocks.hash` above the true fork point is the abandoned chain's block |
//!
//! **Why all three and not only the one the contract named.** They are three doors
//! into ONE deadlock. `update_subtree_roots` runs FIRST in every sync pass, and a
//! Sapling/Orchard refusal there is fatal to the pass — so a wallet that refuses on
//! stale memory never reaches the scan that would rewind further and clear it. Gating
//! two of the three doors leaves the deadlock reachable through the third.
//!
//! **Which legs are MEASURED and which are argued, because the difference matters.**
//! (b) and (d) are measured: with the (d) leg alone switched off, T0-1a's C15 is red
//! again on the same bytes and with the same diff, so the scanned-count oracle really
//! does refuse the honest post-rewind sequence — its scanned blocks all declare
//! `ironwood_commitment_tree_size = 0` while every served height sits below the scan
//! tip. (The T0-1a adjudication inferred that column was NULL for Ironwood, which
//! would have made (d) abstain; **it is a real `0`** — `put_block` takes it as `u32`
//! under the `orchard` feature this SDK enables,
//! `zcash_client_sqlite-0.22.0/src/wallet.rs:4991`, `:5057` — and a scan of one fixture
//! block was driven to read the row back rather than reasoned about.) **The (C6) leg
//! is argued, not forced**: with it alone switched off the proof set is unchanged at
//! 15/1, so no named test in that file covers it and its only cover is
//! `the_rewind_gate_relaxes_the_memory_oracles_and_nothing_else` below.
//!
//! **And the consequence for the proof set, which is owed back to it.** Because (d)
//! refuses every root whose completing block sits below the scan tip on that fixture
//! chain, `an_honest_endpoint_is_accepted_after_a_reorg_rewind` (C7) was passing on the
//! join **while the pool it names was being refused**: it re-serves the sequence the
//! wallet already holds, so "accepted and rewrote the same heights" and "refused and
//! left them alone" produce byte-identical rows, and its `is_ok` cannot separate them
//! either (an Ironwood violation is `Ok([.., HeightViolation, ..])` by design).
//! Measured, not deduced: with the (d) relaxation switched off — the state in which
//! the post-rewind re-serve IS refused — C7 is still green. It stays green after this
//! repair for the same reason. That row needs an assertion on the pass outcome, and
//! it is a finding for the test half, not something this file can fix.
//!
//! **THE SENTENCE THAT USED TO STAND HERE, AND WHY IT IS CORRECTED (§4e-R2 (e)).**
//! It read: *"The bound this buys: ONE relaxed pass per observed rewind, per pool."*
//! Every word of that is true and **it is not a bound**, which is the only thing a
//! reader was going to take from it. A fork does not produce one rewind; it produces
//! one rewind **per sync pass**, indefinitely, at
//! [`crate::constants::REWIND_DISTANCE_BLOCKS`] each, because
//! `update_subtree_roots` runs before the scan and the scan re-detects the same
//! continuity error the next time round. **Six consecutive relaxed passes were driven;
//! C15's own fork depth works out at roughly 549.** So the honest statement is: the
//! relaxation is consumed by each accepted write, and an endpoint that keeps forking
//! keeps re-arming it. Nothing here counts rewinds against a budget, and the reorg
//! STORM guard (`sync::reorg_storm`) bounds them only WITHIN one pass — it resets with
//! the pass. **OWED, and NOT closed by the direction floor**, which changes what a
//! relaxed pass may do, not how many of them there are.
//!
//! **The cost, stated (IT-1b), and it is not small.** A rewind is endpoint-INDUCIBLE:
//! serving blocks from a fork produces the continuity error that runs the rewind, so
//! an endpoint can buy itself a relaxed pass whenever it likes, and — per the
//! correction above — as many as it likes. What it can do with them is bounded by what
//! still binds: the gap floor, the bundled frontiers, and now the direction floor.
//! Since §4e-R2 that is a real bound in the direction that arms the cap erasure, and
//! **still no bound at all on downward motion at INGEST** above the newest bundled
//! Ironwood row (3,459,780) — §4b owed row 7, which the plant row is red about, and
//! which BIND-1 narrows from a permanent state to a window: the move is accepted at
//! ingest and REFUTED when the wallet's own scan reaches the blocks that decide it
//! ([`reconcile_scanned_boundaries`]), with the row corrected and the latch it
//! licensed undone. The plant stays red because it asserts at INGEST, on a wallet
//! that has scanned none of the served boundaries, where nothing has changed.
//! Two things make the trade the right one anyway: the pre-repair code gave the same
//! freedom on EVERY pass with no rewind at all and within ±100 blocks per pass
//! compounding without limit, and the alternative — refusing an honest endpoint after
//! a real reorg — is a wallet permanently stuck on a server telling the truth, which
//! for Sapling or Orchard stops the whole sync. **Tightening it further is a real,
//! named piece of work**: derive the boundary shift from
//! `blocks.{prefix}_commitment_tree_size` on both sides of the rewind and relax by
//! that much rather than by direction alone. See [`check_recorded_heights`] for why
//! the obvious cheap narrowing (relax only above the rewind target) is NOT sound as
//! this wallet computes that target — it was built and driven, and it fails on the
//! very geometry the item targets.
//!
//! # The scan is the oracle above the bundled row — BIND-1 (§4x)
//!
//! Every check above answers the question *may this served height be WRITTEN*, and
//! answers it before the write. [`reconcile_scanned_boundaries`] answers a different
//! one — *is the height we already hold still consistent with what we have since
//! COUNTED* — and answers it after, from `sync::scan_batch`'s `Ok` arm, once per
//! batch, under the db lock on the post-commit state.
//!
//! | | at ingest | at scan (BIND-1) |
//! |---|---|---|
//! | judges | the sequence the endpoint is OFFERING | the row already RECORDED |
//! | evidence | (b) the record, (c) the bundle, (d) the newest scanned count, (C6) a block hash | the per-block tree SIZES of THIS batch, anchored at the batch's own `from_state` |
//! | reaches above 3,459,780 | only where a record already exists | **yes**, at every boundary the scan crosses |
//! | remedy | refuse — nothing is written | record the BRACKET, undo the latch, correct or withdraw the row where that is safe, report |
//!
//! **And since BIND-1-R the two columns are one loop, not two independent oracles.**
//! What the scan refutes is written to [`BOUNDARY_BOUND_TABLE`] and READ BACK by the
//! ingest bind on every later pass ([`check_refuted_heights`]), because the evidence
//! itself does not survive: `read_scanned_sizes` reads only THIS batch's blocks, so
//! by the next pass the batch that caught the lie is behind the frontier and the
//! reconcile is blind to it. The first build corrected the row and kept nothing, and
//! the lie was re-served and re-accepted the moment the row went NULL — the finding
//! all three angles of the fold review reached independently. What is stored is
//! a two-sided BRACKET rather than a remembered value — the true completion lies in
//! `(floor, ceiling]` — so what it closes is the whole window on both sides of the
//! truth and not the one height that was re-served. A one-sided floor was the first
//! build of this repair, and it was INERT on every geometry where the record is too
//! HIGH: there is no incomplete observation to mint a floor from, so nothing was
//! persisted and nothing was refused.
//!
//! **Where it does NOT reach, which is the division of labour and not an oversight.**
//! Below a pool's first commitment its tree is empty, and an empty tree is what a
//! stream carrying nothing for that pool looks like too — so an absence there is not
//! evidence and this oracle abstains ([`pool_is_live`]). That region is exactly the
//! one the SIGNED BUNDLE binds offline, on 821 rows, before any write. The two
//! oracles meet at the newest bundled row and neither covers the other's ground.
//!
//! **Why a correction and not only a refusal.** By scan time the number is already in
//! the table and has already done its work: `mark_stabilized_notes` runs inside
//! `put_blocks`' own transaction (`wallet/scanning.rs:502`, `:527-572`), and
//! `SYNC_BATCH_BLOCKS` equals `PRUNING_DEPTH`, so the very batch that reveals a
//! lowered height can be the batch that latched `witness_stabilized = 1` on the notes
//! it covers — a ONE-WAY latch with no `= 0` writer anywhere upstream. Refusing
//! without correcting would leave the number that caused it in place. The price is
//! the one §4w (d) names and this module would rather not pay: **two writes into
//! upstream's own tables** — `{prefix}_tree_shards.subtree_end_height` (whose sole
//! other writer is `put_shard_roots`) and `{prefix}_received_notes.witness_stabilized`
//! (whose sole other writer only ever sets it to 1). Both are pinned by
//! `zcash_client_sqlite = "=0.22.0"`, neither is guarded by a trigger, and the write
//! that matters is a WITHDRAWAL: this module never writes a height the endpoint chose.
//!
//! # How a poisoned or stale bind is CLEARED (T0-1a question 4)
//!
//! **By the operations that already exist, because the memory is not ours.**
//!
//! * (b) is upstream's own shard table. `truncate_shards` deletes `shard_index >= ?`
//!   on a rewind (`commitment_tree.rs:613-623`); `TreeTruncation::ToCheckpoint`
//!   re-puts through `put_shard`, whose SQL does not re-set `subtree_end_height`,
//!   so the boundary row returns to NULL; `TreeTruncation::ResetToSubtreeRoots`
//!   deletes everything (`:786-793`). Our one reorg caller is
//!   `db.truncate_to_height(rewind)` (`sync.rs`, the scan loop), which a user
//!   reaches simply by syncing. A rescan rebuild replaces the wallet file entirely
//!   (ADR-0534), which clears it outright.
//! * (c) lives in the signed binary and changes only when a new build ships.
//! * (d) is `blocks`, cleared by the same rewind and the same rescan.
//! * The boundary brackets ([`BOUNDARY_BOUND_TABLE`], BIND-1-R) are ours and are
//!   cleared by four things, in ascending order of bluntness: a batch whose own
//!   counts CONTRADICT the stored bracket closes it to nothing and the row is dropped
//!   ([`record_boundary_bounds`]); the next ACCEPTED write for the pool withdraws the
//!   brackets at the indices it covered ([`clear_boundary_bounds_in`]); an
//!   outstanding rewind makes them ABSTAIN for a pass, like the three oracles above;
//!   and a rescan rebuild drops every row ([`clear_boundary_bounds`]).
//!
//! **The two things of ours a rescan must be told about are the rewind ledger**
//! ([`REWIND_TABLE`]) **and the boundary brackets** ([`BOUNDARY_BOUND_TABLE`]) —
//! the first added by T0-1a-R, the second by BIND-1-R, and the answer for both is the
//! `sync_stamp` posture, not `creation_stamp`'s. Both TABLES ride the ADR-0534 copy
//! (so the list stays equal to the ensure set); both sets of ROWS are CLEARED on the
//! rescan temp DB ([`clear_rewind_watch`], [`clear_boundary_bounds`],
//! `store::reset_data_db_keep_seed`). A rebuilt wallet has no shard rows and no
//! scanned blocks, so it has no stale memory that a relaxation could repair and no
//! recorded height for a floor to refute — and the blocks a floor was counted from
//! are gone with the rest, so carrying one across would refuse an honest server a
//! height this wallet can no longer prove wrong. This is the question a shadow-record
//! design could not answer well; here it costs one `DELETE` each in a window that is
//! already crash-atomic with the rebuild.
//!
//! # Per-index or over the sequence (T0-1a question 3)
//!
//! **Both, and they catch different things.** [`check_recorded_heights`] is
//! per-index — it is the only one that sees a single index moved while the
//! aggregate count stays right. [`bundled_counts`] and
//! [`read_scanned_tree_size`] are stated per index but derive from a COUNT
//! (`tree_size >> 16` = how many subtrees were complete at that height), which is
//! what reaches the index dimension at all: an omitted root mid-stream shifts every
//! later shard down one, which no ordering bind can see. [`check_completion_gap`]
//! is over consecutive pairs. The measured attack compresses every index at once,
//! so a per-index check alone would be enough for it — but inflation
//! (`{tip-3, tip-2, tip-1, tip}`) has nothing recorded to compare against, which is
//! why the counting checks are not optional.
//!
//! # First contact (T0-1a question 2)
//!
//! On a wallet that has recorded nothing and scanned nothing, (b) and (d) abstain
//! and (c) does not. The bundled treestates ship an Ironwood frontier on 13 mainnet
//! rows and 25 testnet rows, and a frontier fixes the tree size, hence exactly how
//! many subtrees are complete at that height. So a fresh wallet CAN bind a
//! first-contact endpoint: the measured compressed sequence claims four Ironwood
//! subtrees complete by 3,428,146, and the lowest bundled mainnet row above that
//! height shows fewer, offline. [`check_completion_gap`] also holds at first
//! contact and needs no data at all. What is honestly NOT available at first
//! contact is the `completing_block_hash` cross-check ([`check_completing_hashes`]),
//! which needs a block this wallet has scanned; it abstains explicitly and says so.
//!
//! # No endpoint-attested number enters the bind
//!
//! Not the reported tip, not anything derived from it. `sync::fetch_tip`
//! range-checks `u32` and, since T0-1c, grades the tip against the bundle's newest
//! row ([`newest_bundled_height`]) — a standing the pass REPORTS
//! (`SyncStatus::EndpointBehind`), never a refusal and never a bind: above that
//! row the tip is still a number the endpoint alone asserts (no bound against the
//! previous tip, the wall clock or a later checkpoint), and the moment such a
//! number can move the bind, the endpoint owns the bind. [`gather`] carries the
//! full argument, including why bounding the oracles by the tip is not needed to
//! keep an honest-but-BEHIND server working.

use std::collections::HashMap;

use rusqlite::Connection;
use zcash_client_backend::data_api::chain::CommitmentTreeRoot;
use zcash_client_backend::proto::service::{ShieldedProtocol, TreeState};

use crate::checkpoints;
use crate::db::map_aux_err;
use crate::error::WalletError;
use crate::money::Network;

/// The note-commitment shard height, consensus-fixed at 16 for all three pools
/// (`zcash_client_sqlite-0.22.0/src/lib.rs`: `SaplingShardStore`/`OrchardShardStore`/
/// `IronwoodShardStore` are all instantiated at their pool's `*_SHARD_HEIGHT`, and
/// the Ironwood tree is declared Orchard-shaped — `:3132-3155`). One subtree is
/// therefore `2^16` note commitments. Named here rather than spelled inline so the
/// `>> 16` in [`complete_subtrees`] is not a magic number.
const SHARD_HEIGHT: u32 = 16;

/// Note commitments in one completed subtree: `2^16 = 65_536`.
const SUBTREE_LEAVES: u64 = 1 << SHARD_HEIGHT;

/// The smallest block gap that can separate two consecutive subtree completions.
///
/// **Derived, not chosen.** Zcash's consensus block-size limit is 2,000,000 bytes.
/// A note commitment is a 32-byte field element, so a block consisting of NOTHING
/// but bare note commitments — no headers, no transactions, no ciphertexts, which
/// is already impossible — holds at most `2_000_000 / 32 = 62_500` of them, and a
/// whole subtree is 65,536. `65_536 * 32 = 2_097_152 > 2_000_000`: **one block
/// cannot carry a subtree's worth of commitments**, so subtree *i+1* cannot
/// complete in the block immediately after subtree *i*'s, and consecutive
/// completion heights differ by at least 2.
///
/// This is the arithmetic `sync::validate_root_sequence`'s docstring already
/// gestures at ("two subtrees cannot complete in the same block") carried one step
/// further, from *strictly increasing* to *at least two apart*. It is the only
/// check in this module that needs no data of any kind, and it refuses both shapes
/// the contract names — the measured compression `{h, h+1, h+2, h+3}` and the
/// inflation `{tip-3, tip-2, tip-1, tip}` — on a wallet that has never synced.
///
/// **Its residual, stated rather than hidden:** an attacker who spaces a compressed
/// sequence out by two or more blocks walks straight past it. This is the cheap
/// unconditional floor, not the binding check; the binding checks are the counting
/// ones below. The bound is deliberately the provable minimum and not a tighter
/// realistic estimate: a real Orchard action is ~820 bytes, which would give ~27
/// blocks, but that number depends on a per-pool encoding this build has no
/// compiled source for, and a bound that is too LARGE refuses honest chains.
///
/// `pub(crate)` since T0-1d: `sync::testing::bind_consistent_heights` spaces its
/// synthetic completions by exactly this floor, so a fixture the bind must accept
/// is derived from the bind's own constant rather than from a copy of it.
pub(crate) const MIN_COMPLETION_GAP_BLOCKS: u32 = 2;

/// Which check refused, and the numbers it refused on.
///
/// Kept as data rather than a formatted string so the caller decides what a user
/// may see (§5.4: heights narrow a wallet; a pool name and a reason code do not).
/// Every variant means the same thing to the caller — *the endpoint said something
/// that cannot be true* — and they differ only in which oracle caught it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HeightBindRefusal {
    /// Two completions closer together than the block-size limit allows
    /// ([`MIN_COMPLETION_GAP_BLOCKS`]).
    CompletionGap {
        index: usize,
        previous: u32,
        height: u32,
    },
    /// The served height contradicts the one already recorded at this index — it
    /// DIFFERS from it with no rewind observed since it was written, or it is ABOVE it
    /// with one observed (see [`check_recorded_heights`]: a rewind relaxes this check
    /// downward, never upward, and never to silence).
    RecordedHeight {
        index: usize,
        recorded: u32,
        served: u32,
    },
    /// The signed-binary frontier at `at_height` says `complete` subtrees were
    /// complete there; the served sequence disagrees at `index`.
    BundledFrontier {
        index: usize,
        served: u32,
        at_height: u32,
        complete: u64,
    },
    /// The wallet's own scanned `{prefix}_commitment_tree_size` at `at_height`
    /// says `complete` subtrees were complete there; the served sequence claims
    /// more.
    ScannedTreeSize {
        index: usize,
        served: u32,
        at_height: u32,
        complete: u64,
    },
    /// The endpoint named a completing block whose hash contradicts the block this
    /// wallet scanned at that height.
    CompletingBlockHash { index: usize, height: u32 },
    /// **BIND-1, and the only variant raised AFTER a write rather than before
    /// one.** The wallet's own scan reached the blocks that decide subtree
    /// `index`, and the height already RECORDED for it (`recorded`) cannot be
    /// true — see [`reconcile_boundaries`]. `truth` is the height the scan
    /// counted the boundary at when the crossing was inside the scanned span, and
    /// `None` when the record is refuted with no replacement in hand (the row
    /// then goes back to NULL).
    ///
    /// Every other variant of this enum refuses a height the endpoint is OFFERING;
    /// this one refutes a height the endpoint already got written. That is the
    /// whole point of §4b owed row 7: above the newest bundled row nothing bound
    /// the offer, so the bind that reaches it is the one that arrives later.
    ScannedBoundary {
        index: usize,
        recorded: u32,
        truth: Option<u32>,
    },
    /// **BIND-1-R — the refutation, re-used at INGEST on a later pass.** The
    /// wallet's own scan has already counted the blocks and bracketed subtree
    /// `index`'s true completion inside the half-open interval `(floor, ceiling]`
    /// ([`BOUNDARY_BOUND_TABLE`]); the endpoint is now offering a height outside it.
    ///
    /// **TWO-SIDED, and the second side is not decoration.** A floor alone refuses
    /// only a claim that is too LOW, and that is one of the two ways a recorded
    /// height is wrong: the other is a record whose own predecessor block already
    /// held a whole subtree, where no observation in the run shows the subtree
    /// incomplete and there is no floor to mint. On that geometry a floor-only build
    /// mints nothing, refuses nothing, and is INERT — measured at the join on a
    /// run declaring `2^16 + 1` leaves at every block:
    /// `incomplete_through(idx 0) = None`,
    /// `first_complete_at(idx 0) = Some((3450000, 65537))`. The ceiling is the half
    /// that reaches it.
    ///
    /// `floor` and `ceiling` are each `None` where this wallet's counts never
    /// bounded that side — an absent bound refuses nothing.
    ///
    /// This is the variant that makes [`ScannedBoundary`](Self::ScannedBoundary)
    /// worth raising. Every other member of this enum is re-derived from evidence
    /// the wallet still holds when the check runs; the scan-time refutation is
    /// derived from ONE batch's blocks, and by the next pass that batch is behind
    /// the frontier and [`reconcile_boundaries`] is blind to it
    /// (`reconcile_boundaries`' own "what it cannot say"). Without a durable
    /// interval the same wrong height is re-served and re-accepted forever, which is
    /// exactly what the fold review measured.
    RefutedHeight {
        index: usize,
        served: u32,
        floor: Option<u32>,
        ceiling: Option<u32>,
    },
}

impl HeightBindRefusal {
    /// A stable, secret-free code for the `outcome` tracing field (§5.4 allowlist:
    /// codes only — never a height, which narrows a wallet's scan range).
    /// Exhaustive on purpose: a new variant fails to compile until it has a code.
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::CompletionGap { .. } => "completion_gap",
            Self::RecordedHeight { .. } => "recorded_height",
            Self::BundledFrontier { .. } => "bundled_frontier",
            Self::ScannedTreeSize { .. } => "scanned_tree_size",
            Self::CompletingBlockHash { .. } => "completing_block_hash",
            Self::ScannedBoundary { .. } => "scanned_boundary",
            Self::RefutedHeight { .. } => "refuted_height",
        }
    }
}

/// Everything the bind knows about one pool, gathered ONCE per pass under the db
/// lock so the checks themselves touch no IO.
///
/// The three vectors are independent oracles and any of them may be empty; an
/// empty oracle ABSTAINS, and every abstention below is an explicit branch with a
/// name, never an implicit fallthrough. A guard that silently abstains on the whole
/// fleet is the vacuity class this programme exists to stop.
///
/// `rewind` is NOT a fourth oracle — it says nothing about the served heights. It is
/// the gate that decides whether the three oracles which are memories of a chain get
/// to speak at all this pass (the module's rewind-gate section).
#[derive(Debug, Default, Clone)]
pub(crate) struct PoolEvidence {
    /// `{prefix}_tree_shards.subtree_end_height` by shard index, `None` where the
    /// row exists with a NULL height (the mid-heal state) or does not exist.
    pub(crate) recorded: Vec<Option<u32>>,
    /// `(height, complete subtrees at that height)` from the signed binary,
    /// ascending by height — EVERY row the bundle carries for this network and
    /// pool, UNFILTERED by the endpoint's reported tip, on purpose ([`gather`]
    /// carries the argument: the bind must never let an endpoint-attested number
    /// move a refusal). Tip-bounding lives in exactly one consumer,
    /// `sync::proven_complete_at_or_below` — the `Withheld` discriminator, a
    /// REPORT — and nowhere in this module; and since T0-1c a tip below this
    /// list's newest row ([`newest_bundled_height`]) is graded in
    /// `sync::fetch_tip` and REPORTED by the pass (`SyncStatus::EndpointBehind`),
    /// so the price of that consumer's bound — rows above the tip switched off —
    /// is charged on glass instead of nowhere. An earlier version of this comment
    /// described the rows as tip-filtered; a reader who reconciles the code to
    /// that sentence hands the endpoint the bind (INC-020's own mechanism), which
    /// is why it is corrected here rather than left (§4j row 5).
    pub(crate) bundled: Vec<(u32, u64)>,
    /// `(height, complete subtrees at that height)` at the NEWEST block this
    /// wallet scanned that carries a non-NULL tree size — NOT bounded by the
    /// endpoint's reported tip, deliberately ([`read_scanned_tree_size`]'s own doc
    /// says why; an earlier version of this comment said otherwise). `None` ⇒
    /// abstain.
    pub(crate) scanned: Option<(u32, u64)>,
    /// `blocks.hash` for exactly those served completing heights this wallet has
    /// scanned. A height absent from the map is one the wallet has not scanned, and
    /// the hash check abstains for it.
    pub(crate) scanned_hashes: HashMap<u32, Vec<u8>>,
    /// **BIND-1-R (b2): what the wallet's own scan has already BRACKETED, by shard
    /// index.** `bounds[i] = (floor, ceiling)` ⇒ this wallet counted the blocks and
    /// subtree `i`'s true completion lies in `(floor, ceiling]`, so no height
    /// outside that interval can be true at that index. Either end may be absent.
    /// Written by [`reconcile_scanned_boundaries`], read here, withdrawn by the next
    /// accepted write for the pool ([`clear_boundary_bounds_in`]).
    ///
    /// It is a FOURTH memory of a chain, so it carries the same rewind gate the
    /// other three do — see [`check_refuted_heights`].
    pub(crate) bounds: HashMap<usize, BoundaryBounds>,
    /// The wallet's own rewind ledger for this pool — the gate on the three oracles
    /// above that are memories of a chain. See the module's rewind-gate section.
    pub(crate) rewind: RewindWatch,
}

/// **The one number in this module the wallet writes for itself:** how many chain
/// rewinds it has performed, against how many had happened when this pool's shard
/// heights were last recorded.
///
/// `Default` is `(0, 0)`, i.e. **no rewind observed, so the bind is in force**. That
/// direction is deliberate and it is the D8 lesson from this same programme: a
/// defaulted seam must default to the GUARDING reading, never to the permissive one,
/// or every fake and every future caller silently switches the control off.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RewindWatch {
    /// `subtree_root_rewind` scope `chain` — rewinds this wallet has performed, ever.
    pub(crate) observed: i64,
    /// The value `observed` had when this pool's heights were last WRITTEN.
    pub(crate) recorded_at: i64,
}

impl RewindWatch {
    /// `true` ⇒ the wallet has rewound since it recorded this pool's heights, so its
    /// own memory of the chain is under repair and the three oracles that read it
    /// abstain for this pass.
    pub(crate) fn rewound_since_record(self) -> bool {
        self.observed > self.recorded_at
    }
}

/// How many subtrees are complete in a tree of `leaves` note commitments.
fn complete_subtrees(leaves: u64) -> u64 {
    leaves / SUBTREE_LEAVES
}

/// Hold one pool's served height sequence to everything the wallet knows — a serve
/// that starts at shard index 0 (a full fetch). [`check_pool_from`] is the same
/// bind for a serve that starts above 0; both run [`check_heights`].
///
/// `served` and `completing_hashes` are positionally aligned and both start at
/// shard index 0 — our own request (`start_index = 0`) and our own write
/// (`put_*_subtree_roots(0, ..)`). That is what lets every count check below be
/// stated per index without also assuming the endpoint served a COMPLETE list: a
/// server that stops early is short, never mis-indexed.
///
/// `completing_hashes` may be shorter than `served` (or hold empty entries), which
/// abstains for those indices.
#[cfg_attr(not(test), allow(dead_code))] // production binds through `check_pool_from`
pub(crate) fn check_pool<H>(
    served: &[CommitmentTreeRoot<H>],
    completing_hashes: &[Vec<u8>],
    ev: &PoolEvidence,
) -> Result<(), HeightBindRefusal> {
    let heights: Vec<u32> = served
        .iter()
        .map(|r| u32::from(r.subtree_end_height()))
        .collect();
    check_heights(&heights, completing_hashes, ev)
}

/// The C6 indices a bound serve covered (S15-F1, ADR-0569): shard index →
/// (completing height, this wallet's SCANNED hash of that block), for every served
/// index whose completing block this wallet has scanned — C6 either compared it,
/// or abstained because the endpoint served no (or a wrong-length) hash, which a
/// server can always do. `Wallet::update_subtree_roots` keeps it in the session's
/// memo, and the next plan re-serves any scanned completing index that is not in
/// it with its current scanned hash.
pub(crate) type C6Seen = std::collections::BTreeMap<u64, (u32, [u8; 32])>;

/// **The bind for a serve that starts at shard index `start`** (S15-F1, ADR-0569).
///
/// An incremental fetch serves only `start..`; the indices below it are the
/// wallet's own stored run, untouched by this pass. Every check in
/// [`check_heights`] is positional from index 0 (`check_against_count` above all),
/// so it runs over the VIRTUAL sequence `recorded[..start] ++ served` — the heights
/// the tree will hold once this serve is written — never over the suffix alone.
/// The completing hashes are `[empty; start] ++ served`, so C6 abstains on the
/// prefix (this pass served nothing there) and judges the served part exactly as
/// a full fetch would. With no rewind outstanding, the prefix equals the recorded
/// heights, so equality, the brackets and the counting oracles return the verdict
/// a full honest re-serve of the same prefix would.
///
/// Returns, on an accepted serve, the [`C6Seen`] set of indices `>= start` — empty
/// when a rewind relaxed the bind (C6 abstains outright there, so nothing was
/// checked; the caller leaves the pool unverified).
///
/// The OUTER `Result` is OURS (`Sync { Internal }`): a stored prefix shorter than
/// `start` (a `None` where the plan's stored run promised a height — never a
/// slicing panic), or a rewind outstanding on a serve that starts above 0, which
/// the plan forces to a full fetch and the caller's snapshot re-check guards, so
/// reaching here is a fault in this wallet, not in the endpoint. The INNER one is
/// the endpoint's, as for [`check_pool`].
pub(crate) fn check_pool_from<H>(
    start: u64,
    served: &[CommitmentTreeRoot<H>],
    completing_hashes: &[Vec<u8>],
    ev: &PoolEvidence,
) -> Result<Result<C6Seen, HeightBindRefusal>, WalletError> {
    let rewound = ev.rewind.rewound_since_record();
    if start > 0 && rewound {
        return Err(internal_fault());
    }
    let heights = virtual_heights(start, served, &ev.recorded)?;
    let prefix_len = heights.len() - served.len();
    let mut hashes: Vec<Vec<u8>> = vec![Vec::new(); prefix_len];
    hashes.extend(completing_hashes.iter().take(served.len()).cloned());
    if let Err(refusal) = check_heights(&heights, &hashes, ev) {
        return Ok(Err(refusal));
    }
    let mut seen = C6Seen::new();
    if rewound {
        return Ok(Ok(seen));
    }
    for (offset, root) in served.iter().enumerate() {
        let height = u32::from(root.subtree_end_height());
        let Some(ours) = ev.scanned_hashes.get(&height) else {
            continue;
        };
        // A stored block hash that is not 32 bytes can match nothing; leaving it
        // out keeps the index re-served, which is the conservative side.
        let Ok(ours) = <[u8; 32]>::try_from(ours.as_slice()) else {
            continue;
        };
        seen.insert(start + offset as u64, (height, ours));
    }
    Ok(Ok(seen))
}

/// `Sync { Internal }` — a fault on this device, never the endpoint's.
fn internal_fault() -> WalletError {
    WalletError::Sync {
        stall: crate::state::StallReason::Internal,
    }
}

/// **The virtual sequence** `recorded[..start] ++ served` (S15-F1): the completion
/// heights the pool's tree will hold from index 0 once a serve starting at `start`
/// is written. Read with `get(..start)`, never by slicing: a stored prefix shorter
/// than `start`, or a `None` inside it, is `Sync { Internal }` — the plan's stored
/// run promised a height at every index below `start`.
pub(crate) fn virtual_heights<H>(
    start: u64,
    served: &[CommitmentTreeRoot<H>],
    recorded: &[Option<u32>],
) -> Result<Vec<u32>, WalletError> {
    let prefix_len = usize::try_from(start).map_err(|_| internal_fault())?;
    let prefix = recorded.get(..prefix_len).ok_or_else(internal_fault)?;
    let mut heights: Vec<u32> = Vec::with_capacity(prefix_len + served.len());
    for h in prefix {
        heights.push(h.ok_or_else(internal_fault)?);
    }
    heights.extend(served.iter().map(|r| u32::from(r.subtree_end_height())));
    Ok(heights)
}

/// The body of [`check_pool`] over a height sequence that starts at shard index 0
/// — the served sequence for a full fetch, the virtual one for
/// [`check_pool_from`].
fn check_heights(
    heights: &[u32],
    completing_hashes: &[Vec<u8>],
    ev: &PoolEvidence,
) -> Result<(), HeightBindRefusal> {
    // The rewind gate, read ONCE so the three memory oracles below cannot disagree
    // about it (the "one source of truth for every cross-cutting predicate" rule).
    let rewound = ev.rewind.rewound_since_record();

    check_completion_gap(heights)?;
    // BIND-1-R: the scan's own refutation, asked BEFORE the recorded row it
    // refutes. A re-served lie must be attributed to the oracle that actually
    // knows it is a lie — `refuted_height` — and not to whichever later clause
    // happens to fire, which is §4w (f3)'s lesson applied to the repair itself.
    // The brackets, minus any the NEWEST scanned block already disproves — see
    // [`usable_bounds`]. Filtered once, here, so the two consumers below cannot
    // disagree about which brackets are live.
    let bounds = usable_bounds(&ev.bounds, ev.scanned);
    check_refuted_heights(heights, &bounds, rewound)?;
    check_recorded_heights(heights, &ev.recorded, &bounds, rewound)?;
    for &(at_height, complete) in &ev.bundled {
        check_against_count(heights, at_height, complete, CountOracle::Bundled)?;
    }
    if let Some((at_height, complete)) = ev.scanned {
        if rewound {
            // ABSTAIN, explicitly (rewind): `blocks` rows above the rewind target
            // were deleted, but that target is `REWIND_DISTANCE_BLOCKS` below the
            // DETECTION height rather than below the true fork, so a retained row
            // can still be the abandoned chain's — and its declared tree size then
            // describes a chain this wallet has left. Cleared by the next accepted
            // write for this pool.
        } else {
            check_against_count(heights, at_height, complete, CountOracle::Scanned)?;
        }
    }
    check_completing_hashes(heights, completing_hashes, &ev.scanned_hashes, rewound)?;
    Ok(())
}

/// Consecutive completions at least [`MIN_COMPLETION_GAP_BLOCKS`] apart. Needs no
/// data; holds at first contact.
fn check_completion_gap(heights: &[u32]) -> Result<(), HeightBindRefusal> {
    for (index, window) in heights.windows(2).enumerate() {
        let (previous, height) = (window[0], window[1]);
        // `validate_root_sequence` has already refused a non-increasing sequence,
        // so a saturating difference here can only under-report on data that was
        // already refused; it never over-reports and so never invents a refusal.
        if height.saturating_sub(previous) < MIN_COMPLETION_GAP_BLOCKS {
            return Err(HeightBindRefusal::CompletionGap {
                index: index + 1,
                previous,
                height,
            });
        }
    }
    Ok(())
}

/// **Equality-when-recorded, with a DIRECTION floor after a rewind.** Two floors, one
/// gate: with no rewind observed since this pool's heights were written, a served
/// height must EQUAL the recorded one; with a rewind observed and not yet consumed, it
/// must be at or BELOW it. The check never abstains on a recorded index.
///
/// The invariant is NOT monotonicity. A completed subtree's height is immutable for
/// the chain the wallet is on, so a served height that differs from a recorded one
/// is either a lie or a reorg — and monotonicity admits inflation, which is the
/// trigger for the cap-erasure chain (`truncate_tree_to_subtree_roots` drops
/// `{prefix}_tree_cap` when no shard survives the truncation height,
/// `commitment_tree.rs:785-791`), so a `>=` bind would be worse than none.
///
/// **Why no tolerance.** `TreeTruncation::Unaffected`
/// (`zcash_client_sqlite-0.22.0/src/wallet.rs:4249`) is consumed as a literal no-op
/// by all three per-pool arms (`:4413`, `:4461`, `:4511`), so a pool with no
/// checkpoint at or above the truncation height keeps its shard rows through a
/// rewind — and an Ironwood pool mid-heal has no checkpoints at all, so that is its
/// NORMAL path. After a rewind an honest endpoint therefore serves a DIFFERENT height
/// at an index whose row survived, and the wallet's record and the endpoint's answer
/// honestly disagree. **That is a question about WHETHER a rewind happened, not about
/// HOW FAR the height moved**, and the first build of this check answered the wrong
/// one: it granted ±`REORG_MAX_BLOCKS` on every pass with no rewind required. Two
/// separate defects — an endpoint could walk a recorded height 100 blocks per sync
/// forever, and the window was still too small for a real reorg (the proof's C15
/// measures a legitimate 5,149-block move). The rewind gate replaces both;
/// no code in this module reads `REORG_MAX_BLOCKS` any more.
///
/// # The floor UNDER the relaxation, and it is on DIRECTION (§4e-R2 (c))
///
/// **After an observed rewind, at an index carrying a recorded height: accept a
/// served height AT OR BELOW the record; refuse one ABOVE it.** `served <= rec`
/// relaxes; `served > rec` is a [`HeightBindRefusal::RecordedHeight`].
///
/// **Why there has to be a floor at all.** The first repair let the rewind gate
/// abstain outright, on §4e's word that *"the counting oracles carry the pass"*. They
/// do not, on the one pool this item is about. `bundled_counts(Main, Ironwood)` stops
/// at **3,459,780** and the scanned arm abstains on exactly the low-frontier mid-heal
/// wallet the item targets, so above that height the recorded height is Ironwood's
/// ONLY oracle — and abstaining removed it. Driven by the adjudicator on a wallet
/// holding a correct record: a tail moved to within 200 blocks of the tip is refused
/// while the record stands, and **accepted one induced rewind later**, writing
/// `[3451206, 3475299, 3475399, 3475499]`. That is the inflation shape the item
/// exists to refuse, bought back by a rewind the endpoint can induce at will.
///
/// **Why the floor is on DIRECTION and not on distance.** Verified at the SQL rather
/// than argued: `truncate_tree_to_subtree_roots` collects the shards with
/// `subtree_end_height IS NOT NULL AND subtree_end_height <= :truncation_height`,
/// stopping at the first index gap, then executes `DELETE FROM
/// {prefix}_tree_checkpoints; DELETE FROM {prefix}_tree_shards; DELETE FROM
/// {prefix}_tree_cap;` and re-puts the survivors
/// (`zcash_client_sqlite-0.22.0/src/wallet/commitment_tree.rs:730-793`). The cap goes
/// only when the survivor set is EMPTY, which needs every shard — index 0 first,
/// since collection stops at the first gap — to sit ABOVE the truncation height.
/// **So among the moves an ENDPOINT can ask for, upward motion is the erasure's
/// necessary condition, and a lowered height can only move a shard INTO the surviving
/// prefix.** Refusing the upward half is aimed at that precondition.
///
/// **CORRECTED (BIND-1-R): upward motion is not the only condition, because a served
/// height is no longer the only way that column changes.** A WITHDRAWAL — a NULL
/// written over a recorded height — empties the survivor set just as effectively and
/// without moving anything upward: collection stops at the first index gap, so a NULL
/// at index 0 makes the set empty at every truncation height. The fold built a
/// writer that could do exactly that, and the sentence above, which the direction floor
/// and §4c both rest on, was false while it stood. It is true again in the shape that
/// matters: [`reconcile_scanned_boundaries`] withdraws only whole SUFFIXES
/// ([`plan_withdrawal`]), so it can never punch the index gap the survivor collection
/// breaks at, and the cap-erasure path itself is unreachable in this build (the
/// reachability argument, and the residual it accepts, are on `plan_withdrawal`). A
/// distance bound would instead be a guess at how far a reorg may
/// move a boundary, and this module has already been wrong once by guessing exactly
/// that (the ±`REORG_MAX_BLOCKS` window). It is the same asymmetry this doc gives
/// above for refusing a `>=` bind, carried into the post-rewind case.
///
/// **THE COST, and it is an honest user on an honest server (IT-1b).** A reorg that
/// genuinely RAISES a completion height — the replacement chain sparser after the
/// fork, so the boundary lands later — is now REFUSED at that index. For Ironwood that
/// is a degraded pool; **for Sapling and Orchard it is fatal to the sync pass.** It is
/// not hypothetical: any reorg whose replacement blocks are thinner than the ones they
/// orphan produces it. This is the price of refusing the erasure chain, and it is paid
/// by honest users rather than by attackers.
///
/// **And the exit is narrower than §4e-R2's own wording suggests, so it is stated
/// exactly here.** §4e-R2 (c) says the refusal stands *"until the pool's roots are
/// written or a rescan runs"*. For this case the first of those is not an exit at all:
/// a refusal is whole-pool, so nothing is written, and an honest endpoint on the new
/// chain keeps serving the same raised height on every later pass. There are no
/// admissible heights for it to serve. **A rescan is the only exit**, which is C17's
/// operation and is reachable from a shipped build — but it is a full re-scan, not a
/// retry.
///
/// **This partly undoes the deadlock argument the rewind gate was widened on, and that
/// is worth saying out loud.** The module's rewind-gate section gates all three memory
/// oracles because `update_subtree_roots` runs FIRST in a sync pass, so a pass-fatal
/// refusal there blocks the very scan that would rewind further and clear the stale
/// state. The upward half of this floor re-opens exactly that shape for Sapling and
/// Orchard: refused at the root ingest, no scan, no further rewind, stuck until a
/// rescan. The trade is deliberate — the deadlock needs an honest reorg that RAISES a
/// boundary, while the hole it closes was an endpoint-inducible inflation on any
/// wallet with a record — but a reviewer should weigh it as a trade and not as a
/// strict improvement.
///
/// **What the floor does NOT reach, and a named test stays RED saying so.** It accepts
/// every DOWNWARD move, and a downward move is the ORIGINAL measured attack:
/// compression collapses each shard's join window and is what reaches
/// `witness_stabilized` (INC-020, `ironwood-a11-bypass.py` row C). Above the newest
/// bundled row nothing absolute bounds it.
/// `sync_bind_proof::a_rewind_does_not_license_a_move_no_reorg_could_produce` asserts
/// a 7,749-block downward move is refused and is **RED under this floor,
/// deliberately**: it pins §4b owed row 7, already ruled OPEN by §4d's Q6, rather than
/// finding a new defect. It may not be relaxed to pass, and this floor may not be
/// widened into a distance bound to satisfy it.
///
/// **THE RESIDUAL — the narrowing that looks obvious and is NOT sound here.** The
/// tempting tightening is to relax only ABOVE the rewind target (a rewind to `R`
/// cannot change a subtree that completed at or below `R`, since that stretch of chain
/// is untouched). It is unsound twice over. `rewind_target` is
/// [`crate::constants::REWIND_DISTANCE_BLOCKS`] below the height where the continuity
/// error was DETECTED, not below the fork, so it is no evidence at all that the chain
/// below it is canonical — a fork deeper than 10 blocks moves records that sit below
/// the target. And it was BUILT AND DRIVEN before this floor was chosen (§4e-R2 (b)):
/// it refuses the attack on C15's high-scan geometry and **leaves it accepted on the
/// low-frontier geometry this item targets**, where every recorded height sits above a
/// truncation point near 3,430,091 and so every index is relaxed. A floor whose verdict
/// is decided by where the wallet's own scan frontier sits, rather than by anything the
/// endpoint did, is not a floor. Bounding this properly still needs the boundary shift
/// derived from `blocks.{prefix}_commitment_tree_size` on both sides of the rewind,
/// snapshotted BEFORE the truncate deletes the pre-rewind side — which this item does
/// not build.
///
/// **A second residual, from the consumption rule.** The relaxation is consumed by
/// the next accepted NON-EMPTY write for the pool, whatever its length. If a reorg
/// un-completes a subtree the honest endpoint then serves a SHORTER list, the write
/// consumes the relaxation, and the surplus recorded rows above the served prefix keep
/// their pre-reorg heights un-corrected — so a later honest re-serve of those indices
/// is refused until another rewind or a rescan. The alternative (consume only when the
/// serve covers every recorded index) trades that for an endpoint holding the
/// relaxation open indefinitely by always serving short, which has no exit at all;
/// this one exits through C17's rescan.
///
/// **A third, on the UPGRADE path, because it is invisible from inside this file.**
/// The ledger starts empty on an existing wallet, so `(0, 0)` — the binding reading —
/// is what a wallet that rewound BEFORE this build shipped comes back with, while its
/// shard rows may already hold a pre-reorg height. The old ±`REORG_MAX_BLOCKS` window
/// would have swallowed a shift under 100 blocks; literal equality does not. Ironwood
/// is mostly immune (its rows are NULL until `put_shard_roots` runs, and NULL
/// abstains), Sapling and Orchard are not, and there the refusal is pass-fatal with a
/// rescan as its only exit. Judged acceptable because no build carrying the T0-1
/// subtree-root write has shipped to the fleet, so the population is empty today —
/// which is exactly the kind of premise that stops being true quietly, so it is
/// written down rather than assumed.
fn check_recorded_heights(
    heights: &[u32],
    recorded: &[Option<u32>],
    bounds: &HashMap<usize, BoundaryBounds>,
    rewound_since_record: bool,
) -> Result<(), HeightBindRefusal> {
    for (index, &served) in heights.iter().enumerate() {
        // ABSTAIN, explicitly: no row at this index, or a row whose height was
        // never written (the mid-heal state `put_shard` leaves behind).
        let Some(Some(rec)) = recorded.get(index).copied() else {
            continue;
        };
        // ABSTAIN, explicitly (BIND-1-R): the ROW ITSELF is refuted. The wallet's
        // own scan bracketed subtree `index`'s completion and `rec` falls outside
        // that bracket — too low, or too high — so this row is a number the endpoint
        // got written and the scan has since contradicted. Binding an honest server
        // to it would refuse the truth and admit only the lie, which is the deadlock
        // inverted. What binds this index instead is [`check_refuted_heights`],
        // which has already run and has refused everything outside the bracket. The
        // two are exclusive by construction: a row the bracket refutes cannot equal
        // a serve the bracket admits.
        //
        // The abstention ENDS the moment an accepted write replaces the row — the
        // bracket is withdrawn with it ([`clear_boundary_bounds_in`]) and equality
        // re-engages on the next pass — so it is not a standing relaxation.
        //
        // **THE RESIDUAL, and it is the one interaction in this repair that LOSES
        // something.** This abstention does NOT carry the rewind gate, while
        // [`check_refuted_heights`] does. So at a refuted index, on the one pass an
        // outstanding rewind buys, BOTH are silent — and the direction floor's upward
        // half, which the head of this function calls the cap-erasure's necessary
        // condition, is not applied at that index either. Before BIND-1-R the row
        // there was NULL, which abstained identically, so nothing is worse than the
        // code this repairs; but it IS weaker than gating this abstention too, and
        // that was considered and refused. Gating it means the direction floor
        // refuses `served > rec` at an index whose `rec` is a number the wallet has
        // PROVED wrong, and a post-reorg honest height is above the old lie as often
        // as it is below it — which turns the reorg exit for a wrongly-minted floor
        // into a coin flip. R3 calls that deadlock worse than the bug. What bounds
        // the exposure: it reaches only indices at which this wallet has ALREADY
        // caught this endpoint lying, only while a rewind is unconsumed, the bundled
        // frontiers still bind below their newest row, and — since BIND-1-R — no
        // withdrawal of ours can punch the index gap that made an upward move able to
        // empty the survivor set at more than index 0 ([`plan_withdrawal`]).
        if bounds.get(&index).is_some_and(|b| b.refute(rec)) {
            continue;
        }
        // TWO FLOORS, and the rewind gate decides which one applies at this index.
        // Neither is an abstention: the check answers on every pass, and what the
        // rewind buys is a RELAXATION in one direction, never silence.
        let refused = if rewound_since_record {
            // RELAXED (rewind observed, not yet consumed): the record is the
            // PRE-reorg chain's answer and an honest endpoint's is the post-reorg
            // one, so a correction DOWNWARD is admitted. Upward is not — that is the
            // cap-erasure chain's necessary condition, and the reason is at the head
            // of this function.
            served > rec
        } else {
            // BOUND: equality-when-recorded. A completed subtree's height is
            // immutable for the chain the wallet is on, and no rewind has happened
            // since this row was written, so any difference is the endpoint's.
            served != rec
        };
        if refused {
            return Err(HeightBindRefusal::RecordedHeight {
                index,
                recorded: rec,
                served,
            });
        }
    }
    Ok(())
}

/// **Drop a bracket the NEWEST scanned block already disproves** — otherwise the two
/// oracles together admit nothing at that index and every height is refused.
///
/// Oracle (d) reads one `(at_height, complete)` pair from the newest scanned block
/// and applies [`check_against_count`]'s two-sided rule: at an index below `complete`
/// a served height must be at or below `at_height`; at or above it, strictly over. A
/// stored bracket can contradict either half — a ceiling at or below `at_height` for
/// an index (d) says completed ABOVE it, or a floor at or above `at_height` for an
/// index (d) says completed at or below it — and when it does, the admissible set is
/// EMPTY and the wallet refuses an honest server forever with a rescan as its only
/// exit. That is the permanent-brick shape this repair keeps having to design around.
///
/// **Fail OPEN, deliberately**, and it is the same call [`record_boundary_bounds`]
/// makes for an interval that closes to nothing: two oracles over the same column
/// disagreeing means this wallet does not know where the boundary is, and the honest
/// answer is to stop claiming rather than to refuse everything. The bracket is
/// dropped for this check only — the stored row stays, and the batch that can
/// contradict it properly will empty it.
fn usable_bounds(
    bounds: &HashMap<usize, BoundaryBounds>,
    scanned: Option<(u32, u64)>,
) -> HashMap<usize, BoundaryBounds> {
    let Some((at_height, complete)) = scanned else {
        return bounds.clone();
    };
    bounds
        .iter()
        .filter(|(index, b)| {
            let below = (**index as u64) < complete;
            // (d) says "at or below `at_height`" here, so a floor at or above it
            // leaves nothing; and "strictly above" there, so a ceiling at or below it
            // leaves nothing.
            let floor_conflicts = below && b.floor.is_some_and(|f| f >= at_height);
            let ceiling_conflicts = !below && b.ceiling.is_some_and(|c| c <= at_height);
            !(floor_conflicts || ceiling_conflicts)
        })
        .map(|(index, b)| (*index, *b))
        .collect()
}

/// **The durable half of the scan's refutation** (BIND-1-R, R2/R3 — the repair the
/// fold review's headline finding asked for).
///
/// # What the bracket IS, and why BOTH ends
///
/// `bounds[i] = (floor, ceiling)` is two sentences, each a count this wallet made
/// itself: *subtree `i` had not completed by `floor`*, and *it had completed by
/// `ceiling`*. So its true completion lies in the half-open interval
/// `(floor, ceiling]`, every height outside that interval is refused at `i`, and
/// every height inside it is still admissible. That is the whole of R3: the same row
/// that refuses the re-served wrong height ACCEPTS the honest answer, because the
/// honest answer is inside the bracket by construction.
///
/// **A floor alone is inert on half the geometries, which is how the first build of
/// this repair shipped a guard that never fired.** A floor refuses a claim that is
/// too LOW. A recorded height can be too HIGH just as easily — the record names a
/// block whose predecessor already held a whole subtree — and on that geometry NO
/// observation in the run shows the subtree incomplete, so there is no floor to mint
/// at all. Measured at the join on a run declaring `2^16 + 1` leaves at every
/// block, which is exactly that shape: `incomplete_through(idx 0) = None`,
/// `first_complete_at(idx 0) = Some((3450000, 65537))`. Floor-only: nothing minted,
/// nothing refused, the re-serve accepted exactly as at the base.
///
/// **And a value blacklist would be weaker than either.** Refusing the one height
/// that was refuted closes P3's re-serve and nothing else — the endpoint moves the
/// claim one block and walks straight back into the 15,367-block window the
/// measurement found. A bound closes the window; two bounds close both of them.
///
/// # Why this oracle has to exist at all — and why it is not just oracle (d)
///
/// The two counting oracles are two-sided but both read a COUNT AT A HEIGHT, and
/// above a boundary that reads as *"at least `i + 1` subtrees were complete by `H`"*
/// — which refuses INFLATION at index `i` ABOVE `H` (`served > H`) and says nothing
/// at all about compression, nor about an inflation that stays below the newest
/// scanned block. Oracle (d) reads the NEWEST scanned block, so that half re-fires on
/// every later batch and is durable by accident; everything else has no such
/// evidence, and the batch that had it is behind the frontier by the next pass
/// ([`reconcile_boundaries`]' "what it cannot say"). The bracket is the missing part,
/// made durable on purpose rather than by accident.
///
/// # The rewind gate — the same predicate, for the same reason
///
/// A bracket is a memory of a chain, and a reorg is the event that makes a memory
/// wrong: the blocks it was counted from are the ones `truncate_to_height` deletes.
/// So it abstains on exactly the predicate oracles (b), (d) and (C6) abstain on
/// ([`RewindWatch::rewound_since_record`]), read ONCE in [`check_pool`] so the four
/// cannot disagree. **What that costs:** an endpoint that can induce a rewind — any
/// endpoint, one non-chaining block — buys one pass in which this oracle is silent.
/// That is the identical window §4b owed row 7 already leaves open for the direction
/// floor, which `sync_bind_proof::a_rewind_does_not_license_a_move_no_reorg_could_produce`
/// is red about; this repair does not widen it and does not close it.
/// **What it buys:** a reorg that genuinely MOVES a boundary — in either direction,
/// out of a bracket this wallet recorded honestly before the fork — does not lock an
/// honest server out forever. Without the gate that lockout has no exit but a rescan,
/// and R3 calls that deadlock worse than the bug. Two-sidedness makes this gate
/// matter MORE, not less: a bracket can be escaped in both directions, so there are
/// twice as many honest reorgs it could otherwise wrongly refuse.
fn check_refuted_heights(
    heights: &[u32],
    bounds: &HashMap<usize, BoundaryBounds>,
    rewound_since_record: bool,
) -> Result<(), HeightBindRefusal> {
    // ABSTAIN, explicitly (rewind) — the fourth door into the same question; the
    // head of this function says what it costs and what it buys.
    if rewound_since_record {
        return Ok(());
    }
    for (index, &served) in heights.iter().enumerate() {
        // ABSTAIN, explicitly: the scan has proved nothing about this index. Most
        // indices on most wallets, and every index on a wallet whose scan has not
        // reached a boundary.
        let Some(&bracket) = bounds.get(&index) else {
            continue;
        };
        if bracket.refute(served) {
            return Err(HeightBindRefusal::RefutedHeight {
                index,
                served,
                floor: bracket.floor,
                ceiling: bracket.ceiling,
            });
        }
    }
    Ok(())
}

/// Which counting oracle produced a bound — the two differ only in the refusal
/// they raise, so the comparison itself is written once.
#[derive(Clone, Copy)]
enum CountOracle {
    Bundled,
    Scanned,
}

/// **The counting bind.** At `at_height` the chain's tree held exactly `complete`
/// finished subtrees. Since index *i* of the checked sequence IS subtree *i* (a full
/// fetch asks from index 0 and writes at index 0; an incremental one is checked over
/// the virtual sequence `recorded[..start] ++ served`, [`check_pool_from`]), that is
/// a two-sided statement about every height in it:
///
/// * every index below `complete` completed at or below `at_height`; and
/// * every index at or above `complete` completed strictly above it.
///
/// Both halves are sound WITHOUT assuming the endpoint served a complete list —
/// which matters, because a server that serves a short prefix is harmless (fewer
/// shards recorded ⇒ fewer notes stabilized) and refusing one would be the "remedy
/// worse than the bug" the whole item exists to avoid.
///
/// The first half is what refuses INFLATION on first contact: `{tip-3, tip-2,
/// tip-1, tip}` puts index 0 far above a bundled row that already showed a
/// completed subtree. The second half is what refuses the measured COMPRESSION:
/// index 0 at 3,428,143 sits at or below a bundled row where no Ironwood subtree
/// was complete yet.
fn check_against_count(
    heights: &[u32],
    at_height: u32,
    complete: u64,
    oracle: CountOracle,
) -> Result<(), HeightBindRefusal> {
    let refusal = |index: usize, served: u32| match oracle {
        CountOracle::Bundled => HeightBindRefusal::BundledFrontier {
            index,
            served,
            at_height,
            complete,
        },
        CountOracle::Scanned => HeightBindRefusal::ScannedTreeSize {
            index,
            served,
            at_height,
            complete,
        },
    };
    for (index, &served) in heights.iter().enumerate() {
        let below = (index as u64) < complete;
        if below && served > at_height {
            return Err(refusal(index, served));
        }
        if !below && served <= at_height {
            return Err(refusal(index, served));
        }
    }
    Ok(())
}

/// **The `completing_block_hash` half (C6).** `SubtreeRoot.completing_block_hash`
/// is served by real lightwalletd (it is in this repo's own probe capture,
/// `docs/plan/probes/ironwood-subtree-roots-probe.output.txt`) and was parsed and
/// discarded — nothing in this SDK read it and `zcash_client_sqlite` never reads it
/// either.
///
/// **What it binds, and what it cannot.** It ties an endpoint to its own earlier
/// statements across RPCs and across time, and where `blocks.hash` was written
/// under a different server it ties server B to server A. It cannot attribute the
/// disagreement to either side, so it refuses and never blames. It is NOT the
/// first-contact answer: `blocks` holds only blocks this wallet has SCANNED, so on
/// a fresh wallet it abstains at exactly the moment the recorded bind does. Compact
/// blocks carry no header, so scanning checks `prev_hash` chaining and never that a
/// hash is a real block hash — an endpoint lying consistently across
/// `GetSubtreeRoots` and the block stream breaks nothing else the wallet checks.
/// Keep it: it is cheap and it is the only cross-endpoint handle.
///
/// **Byte order, and why both are accepted.** `blocks.hash` holds a `BlockHash`,
/// which is the INTERNAL (little-endian) serialization — `CompactBlock::hash()`
/// does not reverse, while `TreeState::to_chain_state` explicitly does, with the
/// comment "Zcashd hex strings for block hashes are byte-reversed". The wire
/// `completing_block_hash` is DISPLAY order: the probe capture hexes the raw bytes
/// and gets `00000000008f9ac0…` for mainnet 558,822, and mainnet block hashes carry
/// their zero bytes at the display-leading end. So the canonical comparison is
/// reverse-then-compare. The un-reversed form is accepted TOO, deliberately: the
/// property being measured is *does the endpoint name the same block*, and a server
/// that answers in the other convention has told the truth about which block it
/// was. The cost is that this check cannot flag a byte-order BUG in a server — a
/// bug, not an attack — and the benefit is that a convention this module inferred
/// from a probe rather than from a specification cannot brick an honest fleet.
fn check_completing_hashes(
    heights: &[u32],
    completing_hashes: &[Vec<u8>],
    scanned: &HashMap<u32, Vec<u8>>,
    rewound_since_record: bool,
) -> Result<(), HeightBindRefusal> {
    // ABSTAIN, explicitly (rewind) — the third door into the same deadlock. `blocks`
    // is truncated to a HEURISTIC target, so after a rewind a retained row can be the
    // abandoned chain's block at that height; holding an honest endpoint to it would
    // refuse the pool, and for Sapling/Orchard a refusal here is fatal to the pass —
    // which is the pass that would have scanned forward and rewound further. Gating
    // only the two height oracles would leave that deadlock reachable through this
    // check.
    if rewound_since_record {
        return Ok(());
    }
    for (index, &height) in heights.iter().enumerate() {
        // ABSTAIN, explicitly (1): the endpoint served no hash for this root, or
        // served one that is not a 32-byte block id. A wrong-length field is a
        // malformed response, but the field is optional on the wire and refusing
        // the pool over it would make an old or minimal server unusable for a
        // reason that has nothing to do with the heights this module guards.
        let Some(served_hash) = completing_hashes.get(index).filter(|h| h.len() == 32) else {
            continue;
        };
        // ABSTAIN, explicitly (2): this wallet has not scanned the completing
        // block, so it has nothing to compare against. This is the fresh-wallet
        // case and the below-birthday case, and it is the majority case on any
        // wallet whose birthday is above a pool's early subtrees.
        let Some(ours) = scanned.get(&height) else {
            continue;
        };
        let reversed: Vec<u8> = served_hash.iter().rev().copied().collect();
        if ours.as_slice() != reversed.as_slice() && ours.as_slice() != served_hash.as_slice() {
            return Err(HeightBindRefusal::CompletingBlockHash { index, height });
        }
    }
    Ok(())
}

// ── Reading the evidence ─────────────────────────────────────────────────────

/// The `{prefix}_tree_shards` table name for a pool. The three prefixes are
/// `zcash_client_sqlite`'s own (`SAPLING_TABLES_PREFIX` / `ORCHARD_TABLES_PREFIX` /
/// `IRONWOOD_TABLES_PREFIX`, `lib.rs:188-190`), which are `pub(crate)` there, so
/// they are named here rather than imported. They are interpolated into SQL, which
/// is why they come from this exhaustive match over a compile-time enum and never
/// from a caller-supplied string.
fn table_prefix(pool: ShieldedProtocol) -> &'static str {
    match pool {
        ShieldedProtocol::Sapling => "sapling",
        ShieldedProtocol::Orchard => "orchard",
        ShieldedProtocol::Ironwood => "ironwood",
    }
}

// ── The rewind ledger (T0-1a-R) ──────────────────────────────────────────────
//
// The ONLY state this item adds. It holds no height, no hash, no count of anything
// the user owns — one integer per scope, so §5.4 has nothing to say about it and a
// rescan's copy carries no money state.

/// The rewind ledger's table — referenced by `db::AUX_TABLES_PRESERVED` (ADR-0534)
/// and by the rescan row-clear in `store::reset_data_db_keep_seed`.
pub(crate) const REWIND_TABLE: &str = "subtree_root_rewind";

/// The ledger's wallet-wide scope: rewinds this wallet has performed, ever.
///
/// **Wallet-wide and not per-pool, deliberately.** A rewind is a CHAIN event — one
/// `truncate_to_height` call, one fork, all three pools affected at once — so
/// counting it three times would be three copies of one fact, and the per-pool
/// `TreeTruncation` classification (`Unaffected` / `ToCheckpoint` /
/// `ResetToSubtreeRoots`) is upstream's decision about what to DELETE, not about
/// whether the chain moved. What IS per-pool is the CONSUMPTION: each pool's scope
/// row records the count standing when that pool's heights were last written, because
/// an endpoint may serve one pool and not another on the same pass.
const CHAIN_SCOPE: &str = "chain";

/// Create the rewind ledger (idempotent; runs from `db::migrate` on every provision
/// AND open, so an existing wallet gains it with no schema-version bump).
pub(crate) fn ensure_rewind_table(conn: &Connection) -> Result<(), WalletError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS subtree_root_rewind (
             scope   TEXT PRIMARY KEY,
             rewinds INTEGER NOT NULL
         );",
    )
    .map_err(map_aux_err)
}

/// Drop every ledger row — the rescan rebuild's clear, run on the temp DB before the
/// atomic rename (`store::reset_data_db_keep_seed`, the `sync_stamp` precedent).
///
/// A rebuilt wallet has no shard rows and no scanned blocks, so it has no stale
/// memory a relaxation could repair; carrying one across the rebuild would only hand
/// the endpoint a free relaxed pass on the first post-rescan sync.
pub(crate) fn clear_rewind_watch(conn: &Connection) -> Result<(), WalletError> {
    conn.execute("DELETE FROM subtree_root_rewind", [])
        .map_err(map_aux_err)?;
    Ok(())
}

/// **Record that this wallet is about to rewind — the first half of a compensated
/// pair** (T0-1a-R2, §4g 0a). Called from `sync::rewind_wallet_to` — the one BODY in
/// the SDK that runs `truncate_to_height` — BEFORE the truncate, so the evidence is
/// written by the operation that creates the staleness; [`unnote_rewind`] is its
/// compensation when the truncate returns `Err`.
///
/// **Two callers since an earlier revision, both real rewinds.** `scan_batch`'s continuity arm (a
/// chain reorg) and `sync_once`'s anchor reconcile (SCAN-2 — the endpoint's served
/// tree state contradicts the anchor its own blocks derived, so the derived streak's
/// work is truncated away). The ledger's meaning is unchanged and correct for both:
/// it records that a `truncate_to_height` happened, and the three memory oracles go
/// stale identically whichever event caused it. The doc used to say "the continuity
/// arm … and nothing else"; the structure moved and this sentence moves with it.
///
/// **The window, stated where the code is (R20).** This commits on `aux_db`; the
/// truncate commits on `db` — two connections, two transactions, never one. A kill
/// between them leaves the ledger one ahead of the truth ("armed, no rewind"): the
/// next sync gets ONE relaxed pass — (b) admits a downward move at a recorded index,
/// (d) and (C6) abstain, the gap floor and the bundle still bind — and that pass's
/// accepted write consumes it; the scan then re-detects the same continuity error and
/// performs the rewind for real. That is the direction this order chooses to lose,
/// because the other order loses rewound-but-unrecorded, whose only exit is a rescan.
/// A kill AFTER the truncate lands on a TRUE ledger, which is what note-before buys.
/// The truncate's `Err` is answered by [`unnote_rewind`], because upstream's truncate
/// is one transaction and its `Err` is a rollback (the module doc's §4g matrix).
///
/// One statement, so it takes `RESERVED` atomically (the aux-write invariant) and
/// cannot tear. Its faults are **propagated, not swallowed**: unlike `sync_stamp`
/// (display-only, fail-open) this number gates a money-path control, and a rewind the
/// wallet performed but failed to record would leave it bound to a stale row with no
/// exit but a rescan. Failing the pass is recoverable; forgetting is not — and because
/// this write comes FIRST, a ledger that cannot be written means no truncate runs.
pub(crate) fn note_rewind(conn: &Connection) -> Result<(), WalletError> {
    conn.execute(
        "INSERT INTO subtree_root_rewind (scope, rewinds) VALUES (?1, 1)
         ON CONFLICT(scope) DO UPDATE SET rewinds = rewinds + 1",
        [CHAIN_SCOPE],
    )
    .map_err(map_aux_err)?;
    Ok(())
}

/// **The compensation: the truncate returned `Err`, so the rewind [`note_rewind`]
/// announced did not happen** (T0-1a-R2, §4g 0a). Takes back exactly the one the
/// paired note added. Sound for two reasons, both read rather than assumed: the pair
/// runs back to back under the db lock — every engine writer needs that guard, so no
/// consume can land between the two and a decrement is a restore — and upstream's
/// `truncate_to_height` is `transactionally(..)`
/// (`zcash_client_sqlite-0.22.0/src/lib.rs:1881`), so its `Err` is a rolled-back
/// transaction, never a partial truncate. The note's success is this call's
/// precondition, enforced by the arm's order (a note that fails returns before the
/// truncate), which is why there is no guard here for a row that does not exist.
///
/// Before this existed, the `Err` path — endpoint-inducible through
/// `RequestedRewindInvalid` (a continuity error detected within
/// [`crate::constants::REWIND_DISTANCE_BLOCKS`] of the wallet's own scan floor puts
/// the target below every scanned block), or `SQLITE_FULL` on the WAL growth — left
/// the ledger armed on a wallet that had not rewound, and re-armed it on every pass:
/// one truncate that never ran bought unbounded downward steps. §4f owed row 0a.
///
/// **Its own fault is the double-fault residual.** The caller returns the truncate's
/// classified fault (the original error wins over the compensation's, the
/// `db::copy_aux_tables` rollback convention) and logs this one; the ledger is then
/// one ahead of the truth — the same bounded state a kill before the truncate leaves,
/// consumed by the next accepted write. The realistic way to reach it is
/// `SQLITE_FULL`: a rolled-back truncate leaves its WAL frames on disk, so a disk the
/// truncate just filled can refuse this one-page write too. Pinned by
/// `tests::a_fault_in_the_compensation_leaves_the_relaxation_armed_for_one_pass`.
pub(crate) fn unnote_rewind(conn: &Connection) -> Result<(), WalletError> {
    conn.execute(
        "UPDATE subtree_root_rewind SET rewinds = rewinds - 1 WHERE scope = ?1",
        [CHAIN_SCOPE],
    )
    .map_err(map_aux_err)?;
    Ok(())
}

/// **Consume the relaxation for one pool**, by recording the rewind count that stood
/// when the bind read it. Called from the per-pool hook `sync::put_subtree_roots`
/// runs immediately after THIS pool's put returned `Ok` and before the next pool's put
/// begins (T0-1a-R2, §4g 0b) — never for a pool that served nothing, was refused, or
/// whose put failed.
///
/// **The window, stated where the code is (R20).** The pool's put commits on `db`;
/// this commits on `aux_db`. Two connections, two transactions — there is NO
/// "consumed by the next accepted write" as one atomic step, and the wording in this
/// module that once implied one was wrong (phase-0 §4l-run row 11). The interval
/// between the put's commit and this one is the width of one aux lock acquisition and
/// one statement. A kill inside it, or an `Err` from this write, leaves that pool
/// WRITTEN and still ARMED: the next sync gets one relaxed pass for that pool — its
/// record is already the corrected one, so what the pass admits is a further downward
/// move at a recorded index, with (d) and (C6) abstaining — and that pass's write
/// consumes it. The endpoint cannot re-arm it without a real rewind. That is the side
/// this order chooses: the other order (consume, then put) would land the same kill
/// on consumed-but-unwritten, which is the stale-row deadlock the gate exists to
/// prevent. Pinned at the ledger's level by
/// `tests::a_consume_that_fails_after_a_put_leaves_that_pool_armed_for_one_pass`.
///
/// `observed` is the value read during the BIND, never a fresh read here: the bind
/// and the puts run inside one db-lock critical section so no rewind can interleave,
/// but carrying the bind's own value keeps that a fact about this function rather
/// than a dependence on the caller's locking. If a rewind ever did land in between,
/// the stored value is one behind and the next pass relaxes once more — after a
/// rewind that really happened.
pub(crate) fn note_roots_recorded(
    conn: &Connection,
    pool: ShieldedProtocol,
    observed: i64,
) -> Result<(), WalletError> {
    conn.execute(
        "INSERT INTO subtree_root_rewind (scope, rewinds) VALUES (?1, ?2)
         ON CONFLICT(scope) DO UPDATE SET rewinds = excluded.rewinds",
        rusqlite::params![table_prefix(pool), observed],
    )
    .map_err(map_aux_err)?;
    Ok(())
}

/// One scope's counter; an absent row is 0 (a wallet that has never rewound, and a
/// pool whose heights have never been written).
fn read_scope(conn: &Connection, scope: &str) -> Result<i64, WalletError> {
    use rusqlite::OptionalExtension;
    let n: Option<i64> = conn
        .query_row(
            "SELECT rewinds FROM subtree_root_rewind WHERE scope = ?1",
            [scope],
            |row| row.get(0),
        )
        .optional()
        .map_err(map_aux_err)?;
    Ok(n.unwrap_or(0))
}

/// Read the rewind gate for one pool.
///
/// **Fails CLOSED, like [`read_recorded_heights`]:** a missing table is a `rusqlite`
/// error and is returned, not swallowed into "no rewind" (which binds) or "rewound"
/// (which relaxes). Either default would be a silent answer to a question the wallet
/// could not actually answer.
pub(crate) fn read_rewind_watch(
    conn: &Connection,
    pool: ShieldedProtocol,
) -> Result<RewindWatch, WalletError> {
    Ok(RewindWatch {
        observed: read_scope(conn, CHAIN_SCOPE)?,
        recorded_at: read_scope(conn, table_prefix(pool))?,
    })
}

// ── The boundary-bound ledger (BIND-1-R) ─────────────────────────────────────
//
// The second, and last, piece of state this track adds. Like [`REWIND_TABLE`] it
// holds no money: a pool name, a shard index, and the two block heights between
// which the wallet's own scan proved a subtree's completion must lie. §5.4 treats a
// completion height as wallet-narrowing, which is why it lives in the encrypted aux
// DB beside the rest of the wallet's state and never on a log line.
//
// **Why a table of ours and not a column of upstream's** (§4x-R "does NOT decide" 1).
// The refutation is a statement ABOUT `{prefix}_tree_shards.subtree_end_height`, so it
// cannot be stored in that column: writing it there is the CRITICAL this item repairs
// (a NULL, or any value, at a wrong index re-enters `truncate_tree_to_subtree_roots`'s
// survivor query). It is also a claim with a different lifetime from the row — it
// outlives the row's value on purpose, which is the entire point.
//
// **TWO-SIDED since the join, and the second side is what made the first one
// reach.** The first build stored only a FLOOR — "subtree `i` had not completed by
// `f`" — which refuses a claim that is too LOW and nothing else. A recorded height
// can be wrong in the other direction just as easily: the record names a block whose
// PREDECESSOR already held a whole subtree, so no observation in the run shows the
// subtree incomplete, no floor is mintable, and a floor-only ledger is INERT.
// Measured at the join on `(3_450_000..=3_450_100)` declaring `2^16 + 1` leaves at
// every block: `incomplete_through(idx 0) = None`, `first_complete_at(idx 0) =
// Some((3450000, 65537))`. The true completion lies in the half-open interval
// `(floor, ceiling]`, and BOTH ends were already being computed by
// [`reconcile_boundaries`]; only the ceiling was being thrown away.

/// The boundary-bound ledger's table — referenced by `db::AUX_TABLES_PRESERVED`
/// (ADR-0534) and by the rescan row-clear in `store::reset_data_db_keep_seed`.
///
/// Renamed from `subtree_boundary_floor` when the ceiling joined it: the table is new
/// in this track, nothing outside it depends on the name, and a rename is how a
/// `CREATE TABLE IF NOT EXISTS` gains a column without a migration — the old name
/// simply stops being read.
pub(crate) const BOUNDARY_BOUND_TABLE: &str = "subtree_boundary_bound";

/// One index's bracket. Either end may be absent — the wallet's counts bounded that
/// side or they did not — and an absent end refuses nothing.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BoundaryBounds {
    /// The highest block at which this wallet's counts proved the subtree was still
    /// INCOMPLETE, so its true completion is strictly above this.
    pub(crate) floor: Option<u32>,
    /// The lowest block at which this wallet's counts proved the subtree was already
    /// COMPLETE, so its true completion is at or below this.
    pub(crate) ceiling: Option<u32>,
}

impl BoundaryBounds {
    /// Does this bracket refuse `height`? `false` for a bracket with no ends, which
    /// is the guarding default: a row that bounds nothing may not refuse anything.
    pub(crate) fn refute(self, height: u32) -> bool {
        self.floor.is_some_and(|f| height <= f) || self.ceiling.is_some_and(|c| height > c)
    }
}

/// Create the boundary-bound ledger (idempotent; runs from `db::migrate` on every
/// provision AND open, so an existing wallet gains it with no schema-version bump —
/// [`ensure_rewind_table`]'s posture exactly).
pub(crate) fn ensure_boundary_bound_table(conn: &Connection) -> Result<(), WalletError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS subtree_boundary_bound (
             pool           TEXT    NOT NULL,
             shard_index    INTEGER NOT NULL,
             floor_height   INTEGER,
             ceiling_height INTEGER,
             PRIMARY KEY (pool, shard_index)
         );",
    )
    .map_err(map_aux_err)
}

/// Drop every bracket — the rescan rebuild's clear, run on the temp DB before the
/// atomic rename, next to [`clear_rewind_watch`] and for the same reason.
///
/// A rebuilt wallet has no shard rows and no scanned blocks: there is no recorded
/// height left for a bracket to refute, and the blocks it was counted from are gone.
/// Carrying one across would refuse an honest server a height this wallet can no
/// longer prove wrong.
pub(crate) fn clear_boundary_bounds(conn: &Connection) -> Result<(), WalletError> {
    conn.execute("DELETE FROM subtree_boundary_bound", [])
        .map_err(map_aux_err)?;
    Ok(())
}

/// Read every bracket recorded for `pool`, by shard index.
///
/// **Fails CLOSED**, like [`read_recorded_heights`] and [`read_rewind_watch`]: a
/// missing table is a `rusqlite` error and is returned, never swallowed into "no
/// bounds" — which is the reading that switches the guard off.
pub(crate) fn read_boundary_bounds(
    conn: &Connection,
    pool: ShieldedProtocol,
) -> Result<HashMap<usize, BoundaryBounds>, WalletError> {
    let mut stmt = conn
        .prepare_cached(
            "SELECT shard_index, floor_height, ceiling_height
             FROM subtree_boundary_bound WHERE pool = ?1",
        )
        .map_err(map_aux_err)?;
    let rows = stmt
        .query_map([table_prefix(pool)], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<u32>>(1)?,
                row.get::<_, Option<u32>>(2)?,
            ))
        })
        .map_err(map_aux_err)?;
    let mut out = HashMap::new();
    for row in rows {
        let (shard_index, floor, ceiling) = row.map_err(map_aux_err)?;
        // A row whose index does not fit a `usize` cannot correspond to any served
        // index on this machine, so it can refute nothing; dropped rather than
        // saturated, which would refute the WRONG index.
        let Ok(idx) = usize::try_from(shard_index) else {
            continue;
        };
        out.insert(idx, BoundaryBounds { floor, ceiling });
    }
    Ok(out)
}

/// Tighten one index's bracket — the floor only ever RISES, the ceiling only ever
/// FALLS, and a bracket that closes to nothing is DELETED.
///
/// **Monotone in both directions, and that is load-bearing twice over.** A later
/// batch may observe the same boundary from further away (a re-scan of an earlier
/// range, a batch whose span starts under the last one's), and an end that moved
/// outward would silently un-refute heights an earlier batch had already proved
/// impossible — a guard that weakens itself as it sees more evidence. Monotonicity is
/// also what makes the ledger's meaning independent of the order the batches arrived
/// in: the row is the strongest thing this wallet has ever proved about that boundary.
///
/// **The emptiness sweep is the anti-deadlock guard, and it is the one new failure
/// mode two-sidedness introduces.** A floor at or above the ceiling is an interval
/// with no admissible height in it — every serve at that index refused, forever, with
/// no accepted write to withdraw it and therefore no exit but a rescan. It cannot
/// arise inside ONE run (`partition_point` puts the floor strictly below the ceiling
/// by construction), so reaching it means two batches contradicted each other: a
/// reorg, or an anchor one of them was lied to about. In that state the wallet does
/// not know where the boundary is, and the honest thing is to say so by forgetting —
/// the row is dropped and the index goes back to being bound by the ingest oracles
/// alone. Fail-OPEN here, deliberately, because the failure it prevents is a wallet
/// that can never sync again.
///
/// The only other thing that widens a bracket is withdrawing it outright
/// ([`clear_boundary_bounds_in`]), when an accepted serve replaces the row it was
/// refuting.
fn record_boundary_bounds(
    conn: &Connection,
    pool: ShieldedProtocol,
    index: usize,
    bounds: BoundaryBounds,
) -> Result<(), WalletError> {
    if bounds == BoundaryBounds::default() {
        // Nothing observed bounded either side: there is no claim to persist, and an
        // all-NULL row would be a bracket that refuses nothing while looking like one.
        return Ok(());
    }
    let shard = u64::try_from(index).map_err(|_| WalletError::StoreCorrupt)?;
    // `IFNULL` on both arms so an absent end on EITHER side keeps the other's value
    // rather than folding the pair to NULL: `MAX(NULL, 5)` is NULL in SQLite, and a
    // batch that bounds only one side must not erase the other side's evidence.
    // `prepare_cached`, not `execute`, on BOTH — the code reviewer MAJOR
    // (§4x-R-run owed row 1). This pair runs once per minted index, and the minted
    // set is `0..=reach` unioned with every index carrying a recorded row: on a deep
    // restore of a required pool that is ~1,000-1,900 indices per batch, so ~2,000-3,800
    // statement PREPARES per pool per batch where the base wrote nothing at all. The
    // SQL text is fixed (only the bound parameters vary), so the statement cache is the
    // whole fix and it changes no behaviour. `conn` is a `&Connection` reached through
    // a `Transaction`'s deref — the cache lives on the connection, so it survives the
    // transaction and is warm from the second index onward.
    conn.prepare_cached(
        "INSERT INTO subtree_boundary_bound (pool, shard_index, floor_height, ceiling_height)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(pool, shard_index) DO UPDATE SET
           floor_height = MAX(
               IFNULL(floor_height, excluded.floor_height),
               IFNULL(excluded.floor_height, floor_height)),
           ceiling_height = MIN(
               IFNULL(ceiling_height, excluded.ceiling_height),
               IFNULL(excluded.ceiling_height, ceiling_height))",
    )
    .map_err(map_aux_err)?
    .execute(rusqlite::params![
        table_prefix(pool),
        shard,
        bounds.floor,
        bounds.ceiling
    ])
    .map_err(map_aux_err)?;
    conn.prepare_cached(
        "DELETE FROM subtree_boundary_bound
         WHERE pool = ?1 AND shard_index = ?2
           AND floor_height IS NOT NULL AND ceiling_height IS NOT NULL
           AND floor_height >= ceiling_height",
    )
    .map_err(map_aux_err)?
    .execute(rusqlite::params![table_prefix(pool), shard])
    .map_err(map_aux_err)?;
    Ok(())
}

/// **Withdraw the brackets an accepted serve supersedes** — every index in
/// `[start, end)`, i.e. exactly the indices the write that just landed covered. A
/// full fetch writes `[0, len)`; an incremental one (S15-F1, ADR-0569) writes
/// `[start, start + len)` and leaves the stored prefix untouched, so the brackets
/// below `start` are not superseded by it and stay (the plan re-serves from below
/// the lowest bracket, so a bracketed index is always inside a written range by
/// the time its write is accepted).
///
/// Called from the same per-pool hook [`note_roots_recorded`] uses, immediately
/// after that pool's `put_shard_roots` returned `Ok`, and never for a pool that was
/// refused or served nothing.
///
/// # Why an accepted write is the right expiry, and what the choice costs (§4x-R "does NOT decide" 2)
///
/// **The bracket is not on a clock and does not expire on its own.** It is withdrawn
/// by the one event that makes it moot: a serve that landed INSIDE it and was
/// written. A serve outside it is refused, so nothing is written and nothing is
/// withdrawn — which is why the bracket survives exactly as long as the endpoint
/// keeps re-serving a height the wallet's own counts refute. That is R2.
///
/// **Bought:** the ledger is bounded (one row per refuted index, cleared on the first
/// honest pass), and a server that starts telling the truth is never permanently
/// locked out — the truth is inside the bracket by construction, it is accepted, and
/// the row it writes then equality-binds every later serve, so the endpoint cannot
/// return to the refuted height without a real rewind.
///
/// **Lost:** the wallet forgets the specific lie. An endpoint that gets ANY
/// admissible height written at that index clears the bracket with it, and the memory
/// of the refuted one goes. It gains nothing immediately — the newly written row
/// equality-binds it — but the ledger is not a permanent record of what a server once
/// claimed, and it is not meant to be: the permanent record of a boundary is the
/// recorded height itself once it is right. **Scoped by the written range and not
/// by pool**, so neither a short serve nor an incremental one can clear a bracket
/// at an index it did not cover.
pub(crate) fn clear_boundary_bounds_in(
    conn: &Connection,
    pool: ShieldedProtocol,
    start: u64,
    end: u64,
) -> Result<(), WalletError> {
    // FAIL CLOSED, like every other conversion this repair touches: a bound that
    // does not fit SQLite's signed integer would compare as something else, and a
    // wrong range is the permissive direction on the one statement that withdraws
    // a guard.
    let start = i64::try_from(start).map_err(|_| WalletError::StoreCorrupt)?;
    let end = i64::try_from(end).map_err(|_| WalletError::StoreCorrupt)?;
    if end <= start {
        return Ok(());
    }
    conn.execute(
        "DELETE FROM subtree_boundary_bound
         WHERE pool = ?1 AND shard_index >= ?2 AND shard_index < ?3",
        rusqlite::params![table_prefix(pool), start, end],
    )
    .map_err(map_aux_err)?;
    Ok(())
}

/// **The stored run** (S15-F1, ADR-0569): how many shard indices, from 0, this
/// wallet holds a COMPLETE subtree root for — the leading run of
/// `{prefix}_tree_shards` rows with `subtree_end_height IS NOT NULL AND root_hash
/// IS NOT NULL`, dense from index 0. Upstream's own rebuild rule
/// (`truncate_tree_to_subtree_roots`, `zcash_client_sqlite-0.22.0
/// wallet/commitment_tree.rs:730-793`) collects exactly that run and stops at the
/// first gap, so it is the prefix upstream itself would keep.
///
/// Both columns, not the height alone: a scan can leave a row whose height is
/// recorded but whose root hash is NULL (`put_shard` writes the shard without the
/// column), and an index like that has never had a served root written whole. It
/// ends the run, so the incremental plan re-serves from there and the put restores
/// the root. A withdrawn height (BIND-1's NULL) ends it the same way.
///
/// **Fails CLOSED**, like [`read_recorded_heights`]: a missing table or column is
/// returned as an error, never read as "nothing stored" — though that reading
/// would only force a full fetch, the read is the same class as its siblings.
pub(crate) fn read_stored_run(
    conn: &Connection,
    pool: ShieldedProtocol,
) -> Result<u64, WalletError> {
    let prefix = table_prefix(pool);
    let mut stmt = conn
        .prepare_cached(&format!(
            "SELECT shard_index,
                    subtree_end_height IS NOT NULL AND root_hash IS NOT NULL
             FROM {prefix}_tree_shards ORDER BY shard_index"
        ))
        .map_err(map_aux_err)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, bool>(1)?))
        })
        .map_err(map_aux_err)?;
    let mut run: u64 = 0;
    for row in rows {
        let (shard_index, whole) = row.map_err(map_aux_err)?;
        // Dense from 0: the first index that is not the next one in the run, or a
        // row missing either column, ends it.
        if u64::try_from(shard_index).ok() != Some(run) || !whole {
            break;
        }
        run += 1;
    }
    Ok(run)
}

/// Read (b): the heights already recorded for `pool`, by shard index.
///
/// **Fails CLOSED.** A missing table or a missing `subtree_end_height` column comes
/// back as a `rusqlite` error and is returned, not swallowed — the design review's
/// condition on choosing (b). `zcash_client_sqlite` is pinned `"=0.22.0"` exactly,
/// so no point release arrives without a reviewed bump; if a future bump renames
/// the column, this read turns the guard OFF loudly instead of silently.
pub(crate) fn read_recorded_heights(
    conn: &Connection,
    pool: ShieldedProtocol,
) -> Result<Vec<Option<u32>>, WalletError> {
    let prefix = table_prefix(pool);
    // `prepare_cached`, like [`read_scanned_hashes`] and [`read_scanned_sizes`]:
    // three fixed statements, and since BIND-1 this runs once per pool per BATCH
    // rather than once per pool per pass, so the parse is paid ~34,000 times on a
    // restore if it is not cached (§4x-R Q3 — SCAN-1 exists because batch timing IS
    // the four-hour restore).
    let mut stmt = conn
        .prepare_cached(&format!(
            "SELECT shard_index, subtree_end_height FROM {prefix}_tree_shards ORDER BY shard_index"
        ))
        .map_err(map_aux_err)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Option<u32>>(1)?))
        })
        .map_err(map_aux_err)?;
    let mut out: Vec<Option<u32>> = Vec::new();
    for row in rows {
        let (shard_index, height) = row.map_err(map_aux_err)?;
        // Shard indices are dense from 0 in a consistent tree, but a gap is not
        // ours to diagnose here: pad so `out[i]` is always shard `i`, and let a
        // hole abstain rather than shift every later comparison by one — which is
        // the exact defect (an index shift) this module is guarding against.
        let Ok(idx) = usize::try_from(shard_index) else {
            continue;
        };
        if idx >= out.len() {
            out.resize(idx + 1, None);
        }
        out[idx] = height;
    }
    Ok(out)
}

/// Read (d): the newest block this wallet SCANNED at or below `tip` whose
/// `{prefix}_commitment_tree_size` is non-NULL, and how many subtrees were complete
/// there.
///
/// **Why this number is not the endpoint's.** It is written per scanned block for
/// all three pools (`zcash_client_sqlite-0.22.0/src/wallet.rs:4991`, `:5026`,
/// `:5038`, `:5048`) and, unlike `completing_block_height`, it is checked during
/// scanning against the wallet's own count of the block's outputs — a mismatch is
/// `ScanError::TreeSizeMismatch` (`zcash_client_backend-0.24.0/src/scanning/compact.rs:757-786`,
/// with the Ironwood arm at `:777-785`).
///
/// **The honest limit on that claim, which the contract states more strongly than
/// the code supports.** The per-block counting chains from a `ChainState` anchor
/// that `sync::fetch_chain_state` fetches from the endpoint (`GetTreeState`). So
/// this is a CROSS-RPC bind — it holds `GetSubtreeRoots` to what the same server
/// said through `GetTreeState` and its own compact blocks — not an
/// endpoint-independent one. The bundled frontiers (c) are the endpoint-independent
/// oracle; (d) extends the bind ABOVE the newest bundled row, which is its job.
///
/// **Not bounded by the endpoint's reported tip, deliberately** — see
/// [`self`]'s note on why no endpoint-attested number enters this bind.
pub(crate) fn read_scanned_tree_size(
    conn: &Connection,
    pool: ShieldedProtocol,
) -> Result<Option<(u32, u64)>, WalletError> {
    let prefix = table_prefix(pool);
    let mut stmt = conn
        .prepare(&format!(
            "SELECT height, {prefix}_commitment_tree_size FROM blocks
             WHERE {prefix}_commitment_tree_size IS NOT NULL
             ORDER BY height DESC LIMIT 1"
        ))
        .map_err(map_aux_err)?;
    let row = stmt
        .query_row([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, i64>(1)?)))
        .map(Some)
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(map_aux_err(other)),
        })?;
    // FAIL CLOSED on a size that is not a tree size. `.unwrap_or(0)` stood here and
    // was wrong twice: the project's Rust rules bans a defaulted read of a DB
    // value, and the default it chose — zero — is the one value this module treats
    // as "the pool is absent" ([`pool_is_live`]), so a corrupt row would have
    // switched the oracle off silently in a module whose every sibling read is
    // documented as failing closed.
    row.map(|(height, size)| {
        let leaves = u64::try_from(size).map_err(|_| WalletError::StoreCorrupt)?;
        Ok((height, complete_subtrees(leaves)))
    })
    .transpose()
}

/// Read the `blocks.hash` rows for exactly those `heights` this wallet has scanned.
///
/// Point queries, and only for heights inside the wallet's own scanned span — a
/// full mainnet Sapling serve is ~1,100 roots and almost all of them sit below any
/// modern birthday, so the span filter is what keeps this off the lock budget.
/// Called twice per pool per pass since S15-F1: by [`gather`] for the SERVED
/// heights, and by the incremental plan for the RECORDED completing heights below
/// the stored run (`Wallet::update_subtree_roots`), whose C6 term re-serves any
/// scanned completing index the session has not yet hash-checked.
pub(crate) fn read_scanned_hashes(
    conn: &Connection,
    heights: &[u32],
) -> Result<HashMap<u32, Vec<u8>>, WalletError> {
    let mut out = HashMap::new();
    if heights.is_empty() {
        return Ok(out);
    }
    let span: Option<(u32, u32)> = conn
        .query_row("SELECT MIN(height), MAX(height) FROM blocks", [], |row| {
            Ok((row.get::<_, Option<u32>>(0)?, row.get::<_, Option<u32>>(1)?))
        })
        .map(|(lo, hi)| lo.zip(hi))
        .map_err(map_aux_err)?;
    let Some((lo, hi)) = span else {
        // Nothing scanned at all — every hash check abstains.
        return Ok(out);
    };
    let mut stmt = conn
        .prepare_cached("SELECT hash FROM blocks WHERE height = ?1")
        .map_err(map_aux_err)?;
    for &h in heights {
        if h < lo || h > hi {
            continue;
        }
        match stmt.query_row([h], |row| row.get::<_, Vec<u8>>(0)) {
            Ok(hash) => {
                out.insert(h, hash);
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {}
            Err(other) => return Err(map_aux_err(other)),
        }
    }
    Ok(out)
}

/// Read (c): `(height, complete subtrees)` for every bundled treestate row at or
/// below `tip` whose frontier for `pool` decodes.
///
/// **Read the TABLE, not `checkpoints::bundled_treestate`.** That function clamps
/// its answer to the anchor ceiling (`checkpoints/mod.rs:218-223`) and would hand
/// back a different row than the one asked for — which for provisioning is exactly
/// right and here would silently drop every row above the newest activation, i.e.
/// all 13 mainnet Ironwood rows.
///
/// A row whose frontier is EMPTY is kept, not skipped: an empty Ironwood frontier
/// below the NU6.3 activation is the correct treestate there, it decodes to a tree
/// of size 0, and "zero subtrees were complete at this height" is the single most
/// useful thing the bundle says about the measured attack.
///
/// A row that fails to DECODE is skipped rather than fatal. The shipped bundle is
/// pinned by a content hash and `treestate_bundle_every_row_decodes` covers it, so
/// this branch is unreachable in a correct build; making a corrupt binary refuse
/// every endpoint would turn a build fault into "your wallet cannot sync", which is
/// the wrong direction for a check whose whole purpose is to be additive.
///
/// **Decoded ONCE per process, per network and pool.** The mainnet bundle is 821
/// rows and this runs on every pool of every sync pass under the db lock, so
/// re-parsing 2,463 frontiers per pass would be a lock-budget cost for a table that
/// is compiled into the binary and cannot change while the process lives.
pub(crate) fn bundled_counts(network: Network, pool: ShieldedProtocol) -> &'static [(u32, u64)] {
    cached_bundled_counts(network, pool)
}

/// The newest height the signed bundle carries for `network` — the reference
/// height of the T0-1c tip standing (`sync::tip_standing`, read by
/// `sync::fetch_tip`), defined ONCE, here, beside the trust root it comes from.
/// Read from the treestate table itself (one height per network, shared by all
/// three pools), NOT from the last element of [`bundled_counts`]: that list is per
/// pool and skips a row whose frontier will not decode, so the two agree only
/// while every row decodes (they do today;
/// `tests::newest_bundled_height_is_the_tables_last_row_and_bounds_every_pools_counts`
/// measures it rather than assuming it). The standing wants the ROW — a
/// checkpoint with a hash and a time that the chain provably reached before this
/// binary shipped — not any pool's frontier at it. Reads no wallet state and no
/// endpoint number. `0` on an empty table, which DISARMS the standing (every tip
/// reads at-or-above) rather than badging every server: a corrupt bundle already
/// fails closed at provisioning (`checkpoints::bundled_treestate` →
/// `StoreCorrupt`), and a report is additive.
///
/// The NEWEST row is the table's maximum height, not its last element
/// (T0-1c-R2, the wrap's crypto #11): CI pins the table ascending three ways,
/// and both sibling readers of the bundle (`checkpoints::estimate_birthday`,
/// `checkpoints::newest_checkpoint_time`) still run `is_monotonic` at call
/// time and degrade loudly; this one read `.last()` with no runtime check at
/// all. `max` makes the answer independent of the table's order — defence in
/// depth, at the cost of one pass over ~800 rows once per pass.
pub(crate) fn newest_bundled_height(network: Network) -> u32 {
    newest_row_height(checkpoints::treestate_table(network))
}

/// The maximum height in a treestate table, `0` when empty — pure, so a
/// reordered table can be driven without editing the compiled one
/// (`tests::the_newest_row_is_the_tables_maximum_height_not_its_last_element`).
pub(crate) fn newest_row_height(
    table: &[(u32, &'static str, &'static str, &'static str, &'static str)],
) -> u32 {
    table.iter().map(|&(height, ..)| height).max().unwrap_or(0)
}

/// The per-`(network, pool)` decode cache behind [`bundled_counts`]. Six slots,
/// indexed by an exhaustive match so a fourth pool is a compile error here too.
fn cached_bundled_counts(network: Network, pool: ShieldedProtocol) -> &'static [(u32, u64)] {
    use std::sync::OnceLock;
    static CELLS: [OnceLock<Vec<(u32, u64)>>; 6] = [const { OnceLock::new() }; 6];
    let net_slot = match network {
        Network::Main => 0,
        Network::Test => 1,
    };
    let pool_slot = match pool {
        ShieldedProtocol::Sapling => 0,
        ShieldedProtocol::Orchard => 1,
        ShieldedProtocol::Ironwood => 2,
    };
    CELLS[net_slot * 3 + pool_slot].get_or_init(|| decode_bundled_counts(network, pool))
}

fn decode_bundled_counts(network: Network, pool: ShieldedProtocol) -> Vec<(u32, u64)> {
    let mut out = Vec::new();
    for &(height, hash, sapling, orchard, ironwood) in checkpoints::treestate_table(network) {
        let ts = TreeState {
            network: checkpoints::chain_name(network).to_owned(),
            height: u64::from(height),
            time: 0,
            hash: hash.to_owned(),
            sapling_tree: sapling.to_owned(),
            orchard_tree: orchard.to_owned(),
            ironwood_tree: ironwood.to_owned(),
        };
        let leaves = match pool {
            ShieldedProtocol::Sapling => ts.sapling_tree().map(|t| t.size()),
            ShieldedProtocol::Orchard => ts.orchard_tree().map(|t| t.size()),
            ShieldedProtocol::Ironwood => ts.ironwood_tree().map(|t| t.size()),
        };
        let Ok(leaves) = leaves else { continue };
        out.push((height, complete_subtrees(leaves as u64)));
    }
    out
}

/// Gather every oracle for one pool. Called under the db lock with the aux
/// connection, once per pool per pass.
///
/// **No endpoint-attested number enters this, and that is a decision.** The
/// obvious shape was to bound each oracle by the endpoint's reported tip, so a
/// server that is merely BEHIND is never held to a height it has not seen. It is
/// unnecessary and it is harmful:
///
/// * *Unnecessary*, because the two-sided count check cannot fire on an
///   honest-but-behind server. For an oracle row `(H, m)` above such a server's
///   tip `T`, every served height is `<= T < H`, so the "index below `m` completed
///   at or below `H`" half passes outright; the other half needs the server to
///   have served MORE than `m` roots, i.e. to claim more completed subtrees by `T`
///   than the chain had by the later `H`, which no honest chain produces.
/// * *Harmful*, because the tip is a number the endpoint alone attests — no bound
///   against the previous tip, the wall clock, or a later checkpoint. Since T0-1c
///   `sync::fetch_tip` grades it against the bundle's newest row
///   ([`newest_bundled_height`]) and the pass REPORTS a tip below it
///   (`SyncStatus::EndpointBehind`) — which changes nothing here: the standing
///   is a report, and a report may lean on the tip where a bind may not. Gating
///   the oracles on the tip would let an endpoint UNDER-report it — to anywhere,
///   badged or not — to switch the bundled rows above the lie off and then serve
///   a compressed sequence underneath them. That is the C13 hazard in miniature:
///   the moment an endpoint-supplied value can move the bind, the endpoint owns
///   the bind.
///
/// `served` is what this pass's stream carried — the whole sequence on a full
/// fetch, the suffix from `start` on an incremental one — and the scanned hashes
/// are read for those heights only: the stored prefix's completing hashes are
/// `[empty; start]` in [`check_pool_from`], so C6 has nothing to compare there.
pub(crate) fn gather<H>(
    conn: &Connection,
    network: Network,
    pool: ShieldedProtocol,
    served: &[CommitmentTreeRoot<H>],
) -> Result<PoolEvidence, WalletError> {
    let heights: Vec<u32> = served
        .iter()
        .map(|r| u32::from(r.subtree_end_height()))
        .collect();
    Ok(PoolEvidence {
        recorded: read_recorded_heights(conn, pool)?,
        bundled: bundled_counts(network, pool).to_vec(),
        scanned: read_scanned_tree_size(conn, pool)?,
        scanned_hashes: read_scanned_hashes(conn, &heights)?,
        bounds: read_boundary_bounds(conn, pool)?,
        rewind: read_rewind_watch(conn, pool)?,
    })
}

// ── BIND-1: the scan is the oracle above the newest bundled row ──────────────
//
// Everything above this line runs at INGEST, before a root is written, and holds
// the SERVED sequence to what the wallet already knows. What follows runs at SCAN
// time and holds the RECORDED row to what the wallet has since COUNTED — the only
// oracle in this module that arrives after the write it judges, and the only one
// that reaches above the newest bundled row on a wallet whose scan has got there.

/// One recorded completion height the wallet's own scan contradicts, and what to
/// put in its place.
///
/// Nothing here ever writes a height the endpoint supplied. What a correction does
/// to the ROW is one of three things — rewrite it to the height this wallet counted
/// the boundary at, withdraw it to NULL, or leave it exactly as it is — and which of
/// the three is NOT a property of this struct.
///
/// **The sentence that stood here said a withdrawal is "always available", and what
/// was wrong with it was the SHAPE, not the availability** (§4x-R P1, verified at
/// `zcash_client_sqlite-0.22.0 wallet/commitment_tree.rs:730-793`).
/// `truncate_tree_to_subtree_roots` collects `WHERE subtree_end_height IS NOT NULL
/// AND … ORDER BY shard_index` and BREAKS at the first index gap, then
/// unconditionally `DELETE`s the shards and the cap and re-puts only what it
/// collected. So a withdrawal of ONE row punches a hole that silently drops every
/// shard above it; a withdrawal of a whole SUFFIX never does, at any starting index.
/// [`plan_withdrawal`] decides it and [`reconcile_scanned_boundaries`] obeys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BoundaryCorrection {
    /// The shard index whose recorded height the scan refutes.
    pub(crate) index: usize,
    /// The number being refuted — what `{prefix}_tree_shards.subtree_end_height`
    /// held for `index` when this correction was derived.
    pub(crate) recorded: u32,
    /// `Some(h)`: the scan CROSSED this boundary inside the observed span, so `h`
    /// is the block in which subtree `index` actually completed and the row is
    /// rewritten to it (once `h` has passed the signed bundle — R4). `None`: the
    /// record is refuted but the true boundary lies OUTSIDE the span, so there is no
    /// truth to write and the row is withdrawn where withdrawing it is safe.
    pub(crate) truth: Option<u32>,
    /// **The durable half, lower end** (BIND-1-R). `Some(f)` ⇒ this batch's counts
    /// prove subtree `index` had not completed by height `f`, so every completion
    /// height at or below `f` is refuted at this index from now on
    /// ([`check_refuted_heights`]). `None` ⇒ nothing this batch counted bounded that
    /// side — most often because the run never observed the subtree incomplete at
    /// all, which is exactly the geometry a floor-only ledger is inert on.
    pub(crate) floor: Option<u32>,
    /// **The durable half, upper end.** `Some(c)` ⇒ this batch's counts prove
    /// subtree `index` had ALREADY completed by height `c`, so every completion
    /// height above `c` is refuted at this index from now on. `None` ⇒ no
    /// observation in the run reached the subtree.
    ///
    /// Needs no liveness gate, unlike [`BoundaryCorrection::floor`]: it is positive
    /// evidence — a count of commitments the wallet added up — and a chain declaring
    /// a tree size of zero produces no ceiling at all, because
    /// `complete_subtrees(0)` never exceeds any index.
    pub(crate) ceiling: Option<u32>,
}

/// **The reconcile, PURE** (§4x Q1). `recorded[i]` is the height
/// `{prefix}_tree_shards` holds for shard *i* (`None` ⇒ nothing recorded, which
/// abstains); `observed` is `(height, the pool's tree size in LEAVES at the end of
/// that block)` for an ASCENDING run of heights this wallet has counted itself —
/// the batch's anchor followed by each block of the batch. The raw size, not the
/// subtree count, because the difference between *zero leaves* and *fewer than
/// 65,536 of them* is the whole of [`pool_is_live`]; [`complete_subtrees`] does the
/// one conversion, in one place.
///
/// # Three clauses, each resting on a block this wallet actually counted
///
/// | §4w (d) | clause | the evidence, in one sentence |
/// |---|---|---|
/// | (1) a crossed boundary `(i, h)` with `rec[i] ≠ h` | **(a) the crossing** | the span contains the block where the count first exceeds `i`, and its predecessor, so `h` is known exactly — `rec ≠ h` is refuted AND corrected to `h` |
/// | (3) inflation | **(b) already complete** | a block the wallet counted shows subtree `i` ALREADY complete at `H`, and the record sits above `H` |
/// | (2) the compression lie | **(c) at the claim** | the wallet scanned the very block the record NAMES, that pool has commitments in this run at all ([`pool_is_live`]), and the tree at the end of that block had not reached `i + 1` subtrees |
///
/// # The clause that is NOT here, why it was built first, and what it cost
///
/// The obvious rule is [`check_against_count`]'s two-sided one applied to every
/// observation: *any* block whose count is at or below `i` refutes a record at or
/// below THAT block's height. It was built that way, and it is wrong in a way worth
/// writing down. The two halves of that rule do not rest on the same kind of
/// evidence. "At `H` the chain HAD `i + 1` subtrees" is a positive count of note
/// commitments this wallet added up. "At `H` it had FEWER" is an ABSENCE, and an
/// absence at a distance says nothing about a boundary somewhere else: a block a
/// hundred thousand blocks above the record, in a pool whose commitments this
/// stream simply did not carry, was being read as proof that the record could not
/// be true.
///
/// **Measured, not reasoned:** with the at-a-distance form in place the crate ran
/// **1545 passed / 29 failed** against a 2-failure baseline — 27 rows whose
/// scripted chain declares `{prefix}_commitment_tree_size = 0` on every block
/// (§4w (f4)) while serving the real mainnet completion heights `bundled_counts`
/// derives. Those two facts contradict each other by construction, so the rule
/// refuted all 1,128 Sapling rows and failed the pass. §4x's own B1-10 says every
/// existing caller of the fixture keeps today's zeros — so a production rule that
/// reads a zero as evidence is incompatible with the contract that asks for it, and
/// the rule is the half that had to change. Clause (c) is that half, restricted
/// TWICE: to the one block whose size can speak about this record (the block the
/// record names), and to a pool this run shows commitments for at all
/// ([`pool_is_live`] — the second restriction, which the same measurement forced
/// once the first one left the Ironwood rows red).
///
/// **And the vacuity the zeros would have bought, which is the reason this is not
/// merely a fixture inconvenience.** On a chain where every block declares an empty
/// tree, EVERY recorded height the scan passes is refuted — the truthful ones and
/// the lie alike, at every index. A row driven on such a fixture would go green
/// reporting index 0 while the lowered index it names was never the one that fired:
/// INC-015's shape, and §4w (f3)'s, in the guard this contract is adding. The gate
/// is what makes a row built on this oracle have to carry a real per-block tree
/// size (B1-10) before it can claim anything.
///
/// # What it cannot say, stated rather than left to be discovered
///
/// It is silent at every index no observation reaches — every index whose true
/// boundary sits above the scan frontier (the ordinary state of a mid-heal wallet),
/// every record BELOW this batch's span that the span does not contain, and every
/// record in a pool this batch saw no commitments for. A record the wallet scanned
/// in an EARLIER batch is examined by the batch that contained it and not again. It
/// abstains, it does not accept: an index this function returns nothing for has
/// been JUDGED BY NOTHING here, and the ingest oracles are still its only bind.
/// `observed` empty ⇒ no corrections at all.
///
/// # The shape of `observed`, which two of the three clauses rely on
///
/// Ascending in height and NON-DECREASING in count — a note commitment tree only
/// grows, and [`read_scanned_sizes`] returns the rows in height order. That is
/// what lets clauses (a) and (b) find their block by binary search instead of
/// walking the run once per recorded index (a mainnet Sapling wallet carries ~1,100
/// of them, and this runs under the db lock on every batch). A run that violated it
/// — a corrupt `blocks` table — yields a different observation, never a panic and
/// never an out-of-bounds.
pub(crate) fn reconcile_boundaries(
    recorded: &[Option<u32>],
    observed: &[(u32, u64)],
) -> Vec<BoundaryCorrection> {
    let mut out = Vec::new();
    for (index, rec) in recorded.iter().enumerate() {
        // ABSTAIN, explicitly: no row at this index, or a row whose height was
        // never written. There is nothing recorded to contradict.
        let Some(recorded) = *rec else { continue };
        let i = index as u64;
        // THE BRACKET, computed ONCE per index and attached to whichever clause
        // fires. Both ends come from the same partition point over the same run, so
        // the floor is strictly below the ceiling whenever both exist and they can
        // never disagree with each other within one batch. Attaching them uniformly
        // is also what stopped the first build of this repair from being inert: it
        // minted the floor per clause, and the clause that needs the CEILING was
        // written as the one that mints nothing.
        let floor = incomplete_through(observed, i);
        let ceiling = complete_by(observed, i);
        // (a) THE CROSSING — the strongest evidence there is, so it is asked first
        // and it ENDS the question for this index: the wallet counted the block in
        // which subtree `i` completed, so either the record is that block or it is
        // wrong, and either way nothing a later clause could add would change the
        // verdict or the replacement. Its bracket closes to a single height —
        // `(truth - 1, truth]` — which is the sharpest this module ever gets.
        if let Some(truth) = crossing_height(observed, i) {
            if recorded != truth {
                out.push(BoundaryCorrection {
                    index,
                    recorded,
                    truth: Some(truth),
                    floor,
                    ceiling,
                });
            }
            continue;
        }
        let refuted = match first_complete_at(observed, i) {
            // (b) ALREADY COMPLETE — a counted block shows subtree `i` finished at
            // `at_height` while the record puts it above that. Positive evidence:
            // the wallet added up commitments and there were enough of them, so this
            // clause needs no liveness gate. **This is the geometry a floor-only
            // ledger was inert on**: no observation shows the subtree incomplete, so
            // `floor` is `None` here and the CEILING is the whole of the refusal.
            Some((at_height, _)) if recorded > at_height => true,
            // (c) AT THE CLAIM — the wallet scanned the very block the record names
            // and counted fewer than `i + 1` subtrees complete at the end of it. The
            // ONLY form in which an ABSENCE refutes anything here, so it carries the
            // liveness gate: see [`pool_is_live`].
            _ => {
                pool_is_live(observed)
                    && size_at(observed, recorded).is_some_and(|size| complete_subtrees(size) <= i)
            }
        };
        if refuted {
            out.push(BoundaryCorrection {
                index,
                recorded,
                // No crossing was observed (clause (a) would have taken the branch
                // above), so there is no truth in hand.
                truth: None,
                floor,
                ceiling,
            });
        }
    }
    out
}

/// **Every bracket this batch's counts support, by shard index** — minted
/// INDEPENDENTLY of whether any recorded row was refuted (BIND-1-R, the join's
/// third HIGH).
///
/// # Why this cannot hang off a correction
///
/// [`reconcile_boundaries`] produces a correction only for an index whose recorded
/// row is non-NULL, and a WITHDRAWAL sets that row to NULL in the same transaction
/// that mints the bracket. So a bracket minted from a withdrawal could never be
/// tightened, contradicted or emptied by any later batch: the index no longer has a
/// row, so no later correction is derived for it, so
/// [`record_boundary_bounds`]' emptiness sweep is unreachable for exactly the geometry
/// this item targets. A wrong bracket there is permanent, and on a required pool
/// permanent means every honest server refused with a full rescan as the only exit.
/// Minting from the COUNTS rather than from the refutation is what gives every
/// bracket a later batch that can correct it.
///
/// It also closes the second half of the same finding: [`withdraw_suffix`] NULLs the
/// whole tail, and before this every index above the refuted one was left NULL with
/// NO bracket — both oracles abstaining, nothing above the newest bundled row binding
/// at all, so one refutation at a low index bought the endpoint a free re-write of
/// the entire tail. Every index the withdrawal covers now carries whatever the run
/// proves about it, which may be one end or neither; an absent end refuses nothing,
/// which is the honest weakest claim rather than silence.
///
/// # Which indices
///
/// The union of two sets, and both are bounded. Every index up to the number of
/// subtrees the run shows complete at its end — those are the indices the run has a
/// CEILING for, or a floor at the boundary itself — and every index carrying a
/// recorded row, which is what lets a record above the scan frontier still be
/// bracketed from below. An index the run says nothing about yields
/// `BoundaryBounds::default()`, which [`record_boundary_bounds`] drops without a
/// write.
pub(crate) fn observed_bounds(
    recorded: &[Option<u32>],
    observed: &[(u32, u64)],
) -> Vec<(usize, BoundaryBounds)> {
    let complete_at_end = observed
        .last()
        .map_or(0, |&(_, size)| complete_subtrees(size));
    // `usize` because these index `recorded`; a tree size that does not fit is a
    // corrupt row and yields no bounds rather than a saturated index.
    let Ok(reach) = usize::try_from(complete_at_end) else {
        return Vec::new();
    };
    let mut indices: Vec<usize> = (0..=reach).collect();
    indices.extend(
        recorded
            .iter()
            .enumerate()
            .filter(|(_, r)| r.is_some())
            .map(|(index, _)| index),
    );
    indices.sort_unstable();
    indices.dedup();
    indices
        .into_iter()
        .map(|index| {
            let i = index as u64;
            (
                index,
                BoundaryBounds {
                    floor: incomplete_through(observed, i),
                    ceiling: complete_by(observed, i),
                },
            )
        })
        .filter(|(_, bounds)| *bounds != BoundaryBounds::default())
        .collect()
}

/// The LOWEST observed height at which subtree `index` was already complete — the
/// ceiling [`check_refuted_heights`] then holds every later serve to. `None` when no
/// observation in the run reaches the subtree.
///
/// **The mirror of [`incomplete_through`], and the half the first build of this
/// repair threw away.** It needs no [`pool_is_live`] gate, and the asymmetry is the
/// module's own: this is POSITIVE evidence — the wallet added up commitments and
/// there were at least `(index + 1) × 2^16` of them — while a floor is an ABSENCE and
/// an absence needs a reason to be believed. A chain declaring a tree size of zero on
/// every block, which is every scripted fixture in this crate (§4w (f4)), produces no
/// ceiling at all, because `complete_subtrees(0)` never exceeds any index.
///
/// **A bound NEVER rests on `observed[0]`, and the guard is on the INDEX, not on the
/// run's length.** [`read_scanned_sizes`] prepends the batch's anchor, which is the
/// one number in the run the ENDPOINT supplied; every other element is a block this
/// wallet scanned and counted. The first cut of this guard tested `observed.len() < 2`
/// — which says the run reaches a scanned block SOMEWHERE, not that the bound does —
/// and `partition_point` returning `k == 0` then made `observed[0]` a durable ceiling.
/// That is not a corner: on the first batch of every scan range the anchor is fetched
/// and the wallet has no `blocks` row to check it against, so on a restore it was the
/// ordinary case. Measured on the join's own `refute_index_zero` fixture: EVERY
/// product-path bracket was `floor = None, ceiling = 3_450_000` — the anchor.
///
/// So `k >= 1` is the whole guard: the observation this returns is one the wallet
/// counted itself. A durable bound a server could choose outright is R3's deadlock
/// with the sign flipped — it refuses the honest height, and only an accepted write
/// withdraws it, and nothing can be written while it stands.
fn complete_by(observed: &[(u32, u64)], index: u64) -> Option<u32> {
    let k = observed.partition_point(|&(_, size)| complete_subtrees(size) <= index);
    if k == 0 {
        return None;
    }
    observed.get(k).map(|&(height, _)| height)
}

/// The HIGHEST observed height at which subtree `index` was still incomplete — the
/// floor [`check_refuted_heights`] then holds every later serve to. `None` when the
/// run's very first observation already shows the subtree complete (nothing here
/// bounds it from below), and `None` on a run that is not [`pool_is_live`].
///
/// **The liveness gate is on this function and not only on its caller**, because this
/// is the one that turns an absence into a DURABLE claim. A scripted chain declaring
/// `{prefix}_commitment_tree_size = 0` on every block (§4w (f4) — every fixture in
/// this crate) reads as *"no subtree completed anywhere"*, and writing that as a floor
/// would refute every honest recorded height in the run and persist the refutation.
/// That is the 27-row measurement of the "clause that is NOT here", made permanent.
/// A tree size of zero is the ABSENCE of the pool, never a small number.
///
/// **And a floor NEVER rests on `observed[0]`** — [`read_scanned_sizes`] prepends the
/// batch's anchor, the one number in the run the ENDPOINT supplied, and every other
/// element is a block this wallet scanned and counted. This returns `observed[k - 1]`,
/// so the guard is `k >= 2`: the element chosen is at index 1 or above. The first cut
/// tested `observed.len() >= 2`, which says the RUN reaches a scanned block, not that
/// the BOUND does — the same defect [`complete_by`] carried, stated there with the
/// measurement. A durable claim a server could choose outright is R3's deadlock with
/// the sign flipped: it refuses the honest height, an accepted write is what withdraws
/// it, and nothing can be written while it stands.
///
/// A block at index 1 or above has been checked twice before it gets here: upstream's
/// own end-of-block consistency check inside the scan, and
/// `sync::derive_chain_state`'s per-block check at the download boundary.
///
/// The run is non-decreasing in size, so `partition_point` finds the boundary and the
/// element before it — when that element is a scanned one — is the answer.
fn incomplete_through(observed: &[(u32, u64)], index: u64) -> Option<u32> {
    if !pool_is_live(observed) {
        return None;
    }
    let k = observed.partition_point(|&(_, size)| complete_subtrees(size) <= index);
    if k < 2 {
        return None;
    }
    observed.get(k - 1).map(|&(height, _)| height)
}

/// **The liveness gate on clause (c): has this wallet ever counted a single note
/// commitment for this pool in this run?**
///
/// A tree size of zero is not a small number, it is the absence of the pool: a
/// stream that carries nothing for it looks exactly like a chain below its
/// activation, and neither says anything about where a subtree boundary is. Reading
/// a zero as *"no subtree had completed by here, so your record is a lie"* is the
/// defect that made 27 rows red (the "clause that is NOT here" section), because
/// every scripted chain in this crate declares three zeros per block (§4w (f4)).
///
/// **What the gate costs, and why that cost is already covered.** Below a pool's
/// first commitment the tree really IS empty, so a lowered height claiming a
/// subtree completed down there is no longer refuted at scan time. That is the
/// region the SIGNED BUNDLE binds: `bundled_counts` carries 821 rows, and a claim
/// that a subtree completed where the bundle shows none is refused OFFLINE, before
/// any write (`tests::bundled_ironwood_frontiers_refute_the_measured_attack_offline`
/// — the measured INC-020 sequence is exactly this shape). The gate therefore
/// disarms this oracle only where the oracle it was built to extend already binds:
/// BIND-1 exists for heights ABOVE the newest bundled row, where the pool has
/// commitments by definition — a subtree cannot be claimed complete up there unless
/// earlier ones are, and those are leaves.
///
/// The run is non-decreasing in size, so the last observation is its maximum.
fn pool_is_live(observed: &[(u32, u64)]) -> bool {
    observed.last().is_some_and(|&(_, size)| size > 0)
}

/// The first observation that already shows subtree `index` complete — the run is
/// non-decreasing in size, so this is a binary search. `None` when nothing observed
/// reaches it.
fn first_complete_at(observed: &[(u32, u64)], index: u64) -> Option<(u32, u64)> {
    observed
        .get(observed.partition_point(|&(_, size)| complete_subtrees(size) <= index))
        .copied()
}

/// The tree size at EXACTLY `height`, when this run observed that block — the run
/// is ascending in height, so this is a binary search. `None` ⇒ the wallet did not
/// count that block in this batch, and clause (c) abstains.
fn size_at(observed: &[(u32, u64)], height: u32) -> Option<u64> {
    observed
        .binary_search_by_key(&height, |&(at_height, _)| at_height)
        .ok()
        .and_then(|k| observed.get(k))
        .map(|&(_, size)| size)
}

/// The block in which subtree `index` actually completed, when the observed run
/// CONTAINS that crossing — the first observed height whose count exceeds `index`,
/// and only when the height one block BELOW it was observed too and had not
/// reached the subtree yet.
///
/// Both conditions are load-bearing. Without the predecessor the first observation
/// of a span that starts ALREADY past the boundary would be read as the boundary
/// itself, which is the compression lie with our own signature on it; without the
/// `+ 1` contiguity check a NULL `{prefix}_commitment_tree_size` row (a pool below
/// its activation, a row an older build wrote) would let a gap stand in for the
/// blocks it hides. When either fails there is no truth to write, and the caller then
/// WITHDRAWS the row — and since BIND-1-R a withdrawal is always a whole SUFFIX, never
/// one row, because a hole is the one shape `truncate_tree_to_subtree_roots`' survivor
/// collection cannot read ([`plan_withdrawal`]). The sentence that stood here said a
/// withdrawal was always available and never wrong; it is available, and what makes it
/// safe is its SHAPE.
///
/// # The convention this pins, and where it is corroborated
///
/// `subtree_end_height` is *"the height of the block containing the note commitment
/// that completed the subtree"* (`zcash_client_backend-0.24.0/src/data_api/chain.rs:193-194`;
/// the wire field `SubtreeRoot.completing_block_height` says the same). A block's
/// `{prefix}_commitment_tree_size` is the size AS OF THE END of that block
/// (`zcash_client_sqlite-0.22.0/src/wallet.rs:5884-5886` states it in those words),
/// so the completing block is the first one whose end-of-block size reaches
/// `(index + 1) * 2^16` — which is the first whose `complete_subtrees` exceeds
/// `index`. Upstream derives a shard's block range from the same two facts and the
/// same direction (`wallet.rs:get_block_range`, whose "the end of the subtree is
/// somewhere in the next block" branch is that arithmetic seen from the other side).
///
/// **The residual, because this is the sharpest claim in the module.** Every other
/// oracle here compares a COUNT at a height far from any boundary, so a server whose
/// `completing_block_height` were off by one from this definition would pass all of
/// them; this one names the exact block, and under the Sapling/Orchard disposition a
/// disagreement is fatal to the pass. Three independent sources agree on the
/// definition (the two above, plus `tests::bundled_counts_agree_with_the_live_subtree_root_capture`,
/// where the shipped frontier counts equal the number of roots a live server streamed
/// below them) — and none of them is a server observed AT a boundary, which no
/// fixture in this repository can produce today. If a real server is ever found to
/// use the other convention, this is the function that says so, and the sharpness is
/// deliberate: B1-2 asks for the one-block-off record to be corrected by name.
fn crossing_height(observed: &[(u32, u64)], index: u64) -> Option<u32> {
    let k = observed.partition_point(|&(_, size)| complete_subtrees(size) <= index);
    let (at_height, _) = *observed.get(k)?;
    let (prev_height, prev_size) = *observed.get(k.checked_sub(1)?)?;
    (complete_subtrees(prev_size) <= index && prev_height.checked_add(1) == Some(at_height))
        .then_some(at_height)
}

/// **The scan-time reconcile for one pool** (§4x; §4w (d)'s function, named there).
/// Reads what the wallet counted while scanning `[scanned.0, scanned.1]`, refutes
/// every recorded completion height that contradicts it, CORRECTS the row and
/// UNDOES the stabilization latch those rows licensed, and returns the refusal to
/// report — `None` when the scan had nothing to say.
///
/// Runs on the aux connection under the db lock, inside `sync::scan_batch`'s `Ok`
/// arm, AFTER upstream's own transaction has committed (§4x "does NOT decide" 3:
/// the seam sees the exact post-commit state, including the latch
/// `mark_stabilized_notes` may have just set inside it — P3).
///
/// # The order of the two writes is the crash-safety argument
///
/// The un-latch runs BEFORE the row correction, and that is not a style choice.
/// The reconcile only fires on a row it can still read as wrong: correct the row
/// first and a kill before the un-latch leaves a NULLed row — which every oracle
/// abstains on — beside a note latched `witness_stabilized = 1` whose witness
/// cannot be built, and nothing will ever look again. Un-latch first and the same
/// kill leaves a conservative note (not spendable until re-stabilized) beside a
/// row still visibly wrong, which the NEXT batch re-detects and re-corrects.
/// One order loses a permanent false-spendable; the other loses one batch of
/// spendability on a note whose witness was in doubt anyway.
///
/// # What the correction costs, and who pays it (IT-1b)
///
/// The count this refutes with is the wallet's OWN, but it chains from the
/// batch's anchor, which the endpoint supplied (`GetTreeState`, or SCAN-2's
/// derivation from the endpoint's own blocks) — the same cross-RPC standing
/// [`read_scanned_tree_size`] states for oracle (d). So an endpoint that lies
/// about the anchor can force a TRUE row to NULL. What that buys it is an
/// un-latch (conservative — a note becomes less spendable, never more) and an
/// abstention at that index on the next ingest; it cannot make this function
/// write a height the endpoint chose, and the same lie already moves the tree the
/// scan builds underneath it. The alternative — believing the recorded row over
/// the wallet's own count — is INC-020's stuck state with the wallet's signature
/// on it.
///
/// **And the anchor is not unexamined by the time this runs.** A batch only
/// reaches here after `scan_cached_blocks` returned `Ok`, and before that every
/// block of it passed `sync::derive_chain_state`'s per-block check at the download
/// boundary: a block that CARRIES `chain_metadata` must have the declared tree
/// size the folded frontier produces, folded FROM this anchor (upstream runs the
/// same check inside the scan, `check_end_of_compact_block_consistency`). So an
/// anchor lie has to be carried consistently by every block of the batch, and
/// SCAN-2 re-derives and compares the anchor against the served one every
/// `TREE_STATE_RECONCILE_BATCHES`. A block WITHOUT metadata is not checked by
/// either — stated because it bounds the claim rather than because it changes it.
///
/// # The rewind gate applies to the BLAME and not to the REPAIR
///
/// The number being refuted here is the wallet's MEMORY of a chain, and a reorg is
/// the event that makes a memory wrong — the module's rewind-gate section, reached
/// through a third door. So when [`RewindWatch::rewound_since_record`] is true for
/// this pool, the corrections are still APPLIED (a stale row is exactly what wants
/// correcting, and the scan is the only thing that knows the new chain's answer)
/// and the refusal is NOT returned: an honest endpoint that has not yet re-served
/// this pool since the rewind must not be badged, and for Sapling or Orchard a
/// blame here is fatal to the pass. The same one predicate the ingest bind reads,
/// read once, so the two cannot disagree about whether a rewind is outstanding.
///
/// **And it narrows a residual [`check_recorded_heights`] states as having a
/// rescan for its only exit.** That residual is an honest reorg that RAISES a
/// completion height: the ingest refuses the raised height, so nothing is written,
/// so the record stays the pre-reorg chain's forever and every later honest serve
/// is refused the same way. Under this function the wallet's own scan reaches that
/// boundary, writes the new chain's answer into the row, and the next honest serve
/// then passes on EQUALITY. It is not a full repair of that residual — it needs
/// the scan to reach the boundary, which a deep reorg near the tip may take a
/// while to do — and the exit is no longer only a rescan.
///
/// # What BIND-1-R changed, and why each half had to change (§4x-R)
///
/// The first build made the CORRECTION durable and the refutation ephemeral. All
/// three angles of the fold review found the same thing from three directions: the
/// row went NULL, `check_recorded_heights` ABSTAINS on a NULL, and the batch that
/// held the evidence is behind the frontier by the next pass — so the same lowered
/// height was re-served and re-accepted, `mark_stabilized_notes` re-latched, and the
/// wallet rendered healthy with the lie standing. Measured on `check_pool` at the
/// join: **the NULL turned a refusal into an acceptance across 15,367 blocks.**
///
/// 1. **The refutation is now the durable artifact** ([`BOUNDARY_BOUND_TABLE`],
///    [`check_refuted_heights`]) — and it is a two-sided BRACKET, not a remembered
///    value, so it closes the window on both sides rather than the one re-serve. The
///    ceiling is not decoration: a floor alone is inert wherever the record is too
///    HIGH, which the join measured.
/// 2. **A withdrawal may not open an index gap** ([`plan_withdrawal`]). A NULL at
///    index 0 empties `truncate_tree_to_subtree_roots`' survivor set and erases the
///    tree cap; this function was the first writer in the tree that could put one
///    there. Index 0 is never withdrawn, and above it the whole suffix moves.
/// 3. **Nothing is written that the signed bundle refutes**
///    ([`bundle_contradicts`]) — the `Network` parameter exists for this and for
///    nothing else. The count chain is rooted at an anchor the endpoint supplied, so
///    the module's one endpoint-independent oracle gets to veto what it produces.
/// 4. **The anchor is checked against the wallet's own block row**
///    (`anchor_agrees_with_our_own_blocks`) before any of it runs.
/// 5. **One transaction** for the whole per-pool set, not N × 3 autocommits.
///
/// **OWED and NOT closed here: the watermark** (§4x-R R8). A batch this function
/// never ran for — a kill between `put_blocks`' commit and this call — is still
/// never re-examined, because "examined by the batch that contained it and not
/// again" is still true. What IS closed is the other half of that finding: a fault
/// on one pool no longer skips the pools after it (`sync::reconcile_scan_boundaries`,
/// R6).
pub(crate) fn reconcile_scanned_boundaries(
    conn: &Connection,
    network: Network,
    pool: ShieldedProtocol,
    anchor_height: u32,
    anchor_size: u64,
    scanned: (u32, u32),
) -> Result<Option<HeightBindRefusal>, WalletError> {
    let recorded = read_recorded_heights(conn, pool)?;
    // The cheap exit, and the common one: a pool with nothing recorded (every
    // mid-heal Ironwood wallet until `put_shard_roots` first runs) has nothing
    // for this function to contradict, so every read below is skipped entirely
    // rather than run on every batch of every pass.
    if !recorded.iter().any(Option::is_some) {
        return Ok(None);
    }
    // R5, and it runs BEFORE any evidence is read: the anchor this batch's counts
    // chain from is the ENDPOINT's, and the wallet may already have counted that
    // very block itself. Three standings, not two — see [`AnchorStanding`].
    let standing = anchor_standing(conn, pool, anchor_height, anchor_size)?;
    if standing == AnchorStanding::Contradicted {
        return Ok(None);
    }
    let observed = read_scanned_sizes(conn, pool, anchor_height, anchor_size, scanned)?;
    let bounds = read_boundary_bounds(conn, pool)?;
    let corrections: Vec<BoundaryCorrection> = reconcile_boundaries(&recorded, &observed)
        .into_iter()
        // R4: the signed bundle is this module's ONE endpoint-independent oracle, and
        // every verdict below is derived from a chain of counts rooted at an anchor
        // the endpoint supplied. A verdict the bundle contradicts is not evidence
        // about the endpoint, it is evidence that the chain of counts is wrong — so
        // it is DROPPED whole rather than half-applied: nothing written, no bracket
        // minted, no latch touched. `reconcile_scanned_boundaries` took no `Network`
        // before this repair and so could not ask.
        .filter(|correction| !bundle_contradicts(network, pool, correction))
        .collect();
    // The BRACKETS, minted from the counts rather than from the refutations, and only
    // when the anchor was VERIFIABLE. An unverifiable anchor may still correct a row
    // and undo a latch — both conservative, both reversible by the next honest serve —
    // but it may not persist a DURABLE bound, because a bound is the one thing here
    // that outlives its evidence and refuses future serves on its own authority.
    let minted: Vec<(usize, BoundaryBounds)> = if standing == AnchorStanding::Verified {
        observed_bounds(&recorded, &observed)
            .into_iter()
            .filter(|(index, b)| !bundle_contradicts_bounds(network, pool, *index, *b))
            .collect()
    } else {
        Vec::new()
    };
    // R1 — the CRITICAL, decided ONCE, over the whole correction set and before any
    // of it is written.
    let withdraw_from = plan_withdrawal(&corrections);
    // The un-latch owed to a row a STANDING bracket refutes but this batch derived no
    // correction for. Since withdrawals now cover every index, the way to reach this
    // state is the rewind window: the gate abstains for one pass, the endpoint gets a
    // refuted height written during it, and the next pass — gate consumed, evidence
    // behind the frontier — has the ledger saying the row is wrong and no observation
    // to re-derive it from. `mark_stabilized_notes` re-latches from that row on every
    // batch, so the un-latch is re-applied from the ledger alone.
    let stale: Vec<usize> = recorded
        .iter()
        .enumerate()
        .filter(|(index, rec)| rec.is_some_and(|r| bounds.get(index).is_some_and(|b| b.refute(r))))
        .map(|(index, _)| index)
        .collect();
    // Reported by the LOWEST refuted index: a shift or a compression moves every
    // later index too, and the first one is the one that says what happened.
    let first = corrections.first().copied();
    if first.is_none() && stale.is_empty() && minted.is_empty() {
        return Ok(None);
    }
    // R7 — ONE transaction for the whole set. Before this, N corrections were
    // N × 3 autocommit statements on `aux`, breaking the module's own stated
    // aux-write invariant: a fault after correction *i* left *i*'s row rewritten and
    // *j*'s wrong row and *j*'s latch intact, dropped the report so no badge was
    // raised, and failed the pass — a transient BUSY silently converting a DETECTED
    // endpoint lie into an undetected one.
    //
    // `unchecked_transaction` because this is a `&Connection`: the aux handle is not
    // inside a transaction at this seam (`scan_batch` reaches here with one read
    // behind it and upstream's own transaction already committed on the OTHER
    // connection), and a nested BEGIN would fail LOUDLY rather than silently nest.
    // Every statement inside is idempotent — `MAX` on the floor, a fixed value on the
    // row, a `WHERE witness_stabilized = 1` un-latch — so the caller's busy retry may
    // re-run the whole body.
    let tx = conn.unchecked_transaction().map_err(map_aux_err)?;
    for index in &stale {
        // FAIL CLOSED on a non-shard index, like every other conversion in this
        // module since BIND-1-R: a saturated `u64::MAX` would un-latch a shard that
        // is not the one named.
        let shard = u64::try_from(*index).map_err(|_| WalletError::StoreCorrupt)?;
        unlatch_shard(&tx, pool, shard)?;
        unlatch_shard(&tx, pool, shard.saturating_add(1))?;
    }
    // The brackets go in BEFORE the row moves, so a kill between them leaves the
    // durable refusal standing over a row that still visibly needs it — the
    // conservative half. The reverse order loses the memory of what a corrected row
    // was corrected FROM, which is the state the fold shipped.
    for (index, b) in &minted {
        record_boundary_bounds(&tx, pool, *index, *b)?;
    }
    for correction in &corrections {
        apply_boundary_correction(&tx, pool, correction)?;
    }
    if let Some(from) = withdraw_from {
        withdraw_suffix(&tx, pool, from)?;
    }
    tx.commit().map_err(map_aux_err)?;
    // A batch whose only work was re-applying an older floor's un-latch has found no
    // NEW contradiction, so it badges nobody: the endpoint is already being refused
    // at ingest by the floor that produced it (`check_refuted_heights`), and raising
    // the outcome again every batch would say the same thing once per hundred blocks.
    let Some(first) = first else {
        return Ok(None);
    };
    // The rewind gate, on the BLAME only — the repair above has already run. See
    // this function's own doc for why an honest endpoint that has not re-served
    // this pool since the rewind must not be badged for a record the reorg made
    // stale.
    if read_rewind_watch(conn, pool)?.rewound_since_record() {
        // Said out loud even though nobody is blamed (principle 10): rows in
        // upstream's own table were just rewritten, and a correction no line
        // anywhere records is the kind of write a later reader cannot account
        // for. §5.4: a pool name, a stable code on the allowlisted `outcome`
        // field, and a COUNT of corrected rows — no height, no index, no note.
        tracing::debug!(
            target: "zec_wallet_core",
            pool = pool.as_str_name(),
            outcome = "scanned_boundary_after_rewind",
            count = corrections.len(),
            "the wallet's own scan contradicted completion heights recorded before a \
             rewind it has not yet consumed for this pool — the rows were corrected \
             from the scan and the endpoint is NOT blamed: after a reorg an honest \
             server's answer and this record are expected to disagree"
        );
        return Ok(None);
    }
    Ok(Some(HeightBindRefusal::ScannedBoundary {
        index: first.index,
        recorded: first.recorded,
        truth: first.truth,
    }))
}

/// The per-block tree SIZES for `[lo, hi]` — leaves, as of the end of each block —
/// with the batch's anchor in front of them.
///
/// The anchor is the state at the block BELOW the batch, so it is kept only when
/// it really sits below `lo` — the run this returns must be ascending for
/// [`crossing_height`]'s contiguity test to mean what it says. A block whose
/// `{prefix}_commitment_tree_size` is NULL is SKIPPED, not defaulted to zero: a
/// missing size is an absence of evidence, and reading it as "the tree was empty
/// here" would invent a refusal out of a schema hole. (A NULL and a real `0` are
/// different rows and are treated differently — the `0` is a fact this wallet
/// counted, and [`pool_is_live`] is what decides how much that fact can say.)
fn read_scanned_sizes(
    conn: &Connection,
    pool: ShieldedProtocol,
    anchor_height: u32,
    anchor_size: u64,
    (lo, hi): (u32, u32),
) -> Result<Vec<(u32, u64)>, WalletError> {
    let prefix = table_prefix(pool);
    let mut out = Vec::new();
    if anchor_height < lo {
        out.push((anchor_height, anchor_size));
    }
    if hi < lo {
        return Ok(out);
    }
    // `prepare_cached`, like [`read_scanned_hashes`]: three fixed statements that
    // run once per pool per BATCH, not once per pass, so the parse is worth keeping.
    let mut stmt = conn
        .prepare_cached(&format!(
            "SELECT height, {prefix}_commitment_tree_size FROM blocks
             WHERE height >= ?1 AND height <= ?2
               AND {prefix}_commitment_tree_size IS NOT NULL
             ORDER BY height"
        ))
        .map_err(map_aux_err)?;
    let rows = stmt
        .query_map([lo, hi], |row| {
            Ok((row.get::<_, u32>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(map_aux_err)?;
    for row in rows {
        let (height, size) = row.map_err(map_aux_err)?;
        // FAIL CLOSED, for [`read_scanned_tree_size`]'s reason: `.unwrap_or(0)` stood
        // here, and zero is the one value that means "the pool is absent" to
        // [`pool_is_live`], so a corrupt row would have quietly disarmed clause (c)
        // and the floor with it — while also breaking the NON-DECREASING precondition
        // both binary searches in this module rest on.
        let size = u64::try_from(size).map_err(|_| WalletError::StoreCorrupt)?;
        // **FAIL CLOSED on a run that is not non-decreasing.** Both binary searches
        // over this run assume it, and since BIND-1-R their answers are PERSISTED as
        // durable bounds — a `partition_point` over an unsorted slice returns a
        // meaningless index, and a meaningless bound refuses honest heights until a
        // rescan. A note commitment tree only grows, so a run that shrinks is a
        // corrupt `blocks` table and not a chain this wallet should reason about.
        if out.last().is_some_and(|&(_, previous)| size < previous) {
            return Err(WalletError::StoreCorrupt);
        }
        out.push((height, size));
    }
    Ok(out)
}

/// **R5 — the anchor is the endpoint's, and the wallet may already have counted that
/// block itself.**
///
/// Every clause of [`reconcile_boundaries`] is a statement about an ABSOLUTE tree
/// level. The per-block deltas are ours, but the level they are added to is
/// `from_state`'s frontier size, and `from_state` is a `GetTreeState` the endpoint
/// served on the first batch of every pass, after every rewind, and at each
/// `TREE_STATE_RECONCILE_BATCHES`th batch (`sync::fetch_chain_state`,
/// `wallet.rs`'s driver). In the ordinary contiguous case `anchor_height` is the
/// block the PREVIOUS batch scanned, so `blocks.{prefix}_commitment_tree_size` holds
/// this wallet's own answer for exactly that height, in the table this function is
/// about to query.
///
/// **The wallet's own number wins, and the way it wins is that the reconcile
/// ABSTAINS.** Not "substitute ours and carry on": this batch's per-block sizes were
/// written by `scan_cached_blocks` CHAINING FROM the endpoint's anchor, so if that
/// anchor is contradicted the whole run is built on the contradicted number and no
/// clause over it is evidence about anything. Correcting the anchor alone would leave
/// the run that disagrees with it in place.
///
/// `Ok(true)` ⇒ proceed: the wallet has no row at that height (a first batch, a
/// re-scan, a pool below its activation whose column is NULL), or it has one and the
/// two agree. `Ok(false)` ⇒ abstain for this pool this batch.
///
/// **Reported, not refused**, and deliberately: this is the first comparison in the
/// crate between `ChainState`'s frontier size and the `blocks` column, and if the two
/// ever used different conventions a refusing form would fail every honest pass on
/// every wallet. Abstaining fails safe — it switches this oracle off for the batch,
/// which is the state the whole codebase was in before BIND-1 — and the `warn!` is
/// how it would be found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnchorStanding {
    /// This wallet holds its own count for the anchor block and the two agree.
    Verified,
    /// This wallet has no `blocks` row at that height, so it cannot check the
    /// anchor at all — the FIRST BATCH of every scan range, which on a restore is
    /// most of them.
    Unverifiable,
    /// This wallet holds its own count for the anchor block and the two DISAGREE.
    Contradicted,
}

fn anchor_standing(
    conn: &Connection,
    pool: ShieldedProtocol,
    anchor_height: u32,
    anchor_size: u64,
) -> Result<AnchorStanding, WalletError> {
    let Some(ours) = read_block_tree_size(conn, pool, anchor_height)? else {
        return Ok(AnchorStanding::Unverifiable);
    };
    if ours == anchor_size {
        return Ok(AnchorStanding::Verified);
    }
    // §5.4: a pool name and a stable code on the allowlisted `outcome` field. NOT the
    // two sizes and NOT the height — a tree size at a named height narrows a wallet's
    // scan position as sharply as the height itself does.
    tracing::warn!(
        target: "zec_wallet_core",
        pool = pool.as_str_name(),
        outcome = "anchor_contradicts_our_blocks",
        "the tree state this batch was anchored on disagrees with what this wallet \
         itself counted at that same height — the batch's per-block counts chain from \
         that anchor, so they are not evidence about any recorded completion height \
         and the scan-time reconcile abstains for this pool on this batch"
    );
    Ok(AnchorStanding::Contradicted)
}

/// The bundle veto applied to a BRACKET rather than to a correction — the same two
/// clauses [`bundle_contradicts`] carries for `floor` and `ceiling`, over a bracket
/// minted from the counts alone.
///
/// A contradicted bracket is dropped whole rather than half-kept: the two ends come
/// from one chain of counts, so a bundle row that refutes either has refuted the
/// chain, and keeping the other end would be persisting a bound derived from
/// arithmetic the signed binary says is wrong.
fn bundle_contradicts_bounds(
    network: Network,
    pool: ShieldedProtocol,
    index: usize,
    bounds: BoundaryBounds,
) -> bool {
    bundle_contradicts(
        network,
        pool,
        &BoundaryCorrection {
            index,
            // Not read by `bundle_contradicts`; a bracket carries no recorded value
            // and no replacement, only the two bounds.
            recorded: 0,
            truth: None,
            floor: bounds.floor,
            ceiling: bounds.ceiling,
        },
    )
}

/// This wallet's own `{prefix}_commitment_tree_size` at exactly `height`, or `None`
/// when it has not scanned that block or the column is NULL there (a pool below its
/// activation, a row an older build wrote). One indexed point query, `prepare_cached`
/// — [`read_scanned_hashes`]' shape, and it runs once per pool per batch only on a
/// wallet that has a recorded height to defend ([`reconcile_scanned_boundaries`]'
/// cheap exit is ahead of it).
fn read_block_tree_size(
    conn: &Connection,
    pool: ShieldedProtocol,
    height: u32,
) -> Result<Option<u64>, WalletError> {
    use rusqlite::OptionalExtension;
    let prefix = table_prefix(pool);
    let size: Option<i64> = conn
        .prepare_cached(&format!(
            "SELECT {prefix}_commitment_tree_size FROM blocks
             WHERE height = ?1 AND {prefix}_commitment_tree_size IS NOT NULL"
        ))
        .map_err(map_aux_err)?
        .query_row([height], |row| row.get(0))
        .optional()
        .map_err(map_aux_err)?;
    size.map(|s| u64::try_from(s).map_err(|_| WalletError::StoreCorrupt))
        .transpose()
}

/// **R4 — does the SIGNED BUNDLE contradict this verdict?**
///
/// The bundle is the one oracle in this module that needs no endpoint and no scan:
/// `(at_height, complete)` says the chain's tree held exactly `complete` finished
/// subtrees at `at_height`, from a treestate compiled into the signed binary. Two
/// claims are checked against it, and they are the two things a correction asserts:
///
/// * **`truth = Some(h)`** asserts *subtree `index` completed in block `h`*, which
///   the bundle refutes on the same two-sided rule [`check_against_count`] uses —
///   below `complete` the boundary is at or under `at_height`, at or above it the
///   boundary is strictly over.
/// * **`floor = Some(f)`** asserts *subtree `index` had NOT completed by `f`*, which
///   the bundle refutes whenever it shows that subtree already complete at a height
///   at or below `f`.
/// * **`ceiling = Some(c)`** asserts *subtree `index` HAD completed by `c`*, which
///   the bundle refutes whenever it shows that subtree still incomplete at a height
///   at or above `c`. This is the veto that matters most, because the ceiling is the
///   end that refuses an honest server for serving too HIGH, and it is derived from a
///   count chained off the endpoint's own anchor.
///
/// A contradiction means the counts this verdict was derived from are wrong — the
/// anchor, or the blocks, or both — so the caller drops the verdict whole. It does
/// NOT mean the recorded row is right; it means this batch cannot say.
fn bundle_contradicts(
    network: Network,
    pool: ShieldedProtocol,
    correction: &BoundaryCorrection,
) -> bool {
    let index = correction.index as u64;
    bundled_counts(network, pool)
        .iter()
        .any(|&(at_height, complete)| {
            let below = index < complete;
            let truth_refuted = correction
                .truth
                .is_some_and(|h| (below && h > at_height) || (!below && h <= at_height));
            // `below` alone: the bundle proves subtree `index` was complete at
            // `at_height`, so a floor claiming it was still incomplete at or above that
            // height is the contradiction. When the bundle shows it NOT complete there,
            // it says nothing against a floor at all.
            let floor_refuted = below && correction.floor.is_some_and(|f| at_height <= f);
            // The mirror: `!below` is the bundle proving subtree `index` was NOT yet
            // complete at `at_height`, so a ceiling claiming it HAD completed at or
            // below that height is the contradiction.
            let ceiling_refuted = !below && correction.ceiling.is_some_and(|c| c <= at_height);
            truth_refuted || floor_refuted || ceiling_refuted
        })
}

/// **R1. Which withdrawal, if any, this correction set makes — and it is ALWAYS a
/// SUFFIX.**
///
/// Returns the LOWEST refuted index with no replacement in hand; the row there and
/// every row above it goes to NULL. `None` when every refutation this batch found
/// carries a truth to write instead.
///
/// # The property, and it is DENSITY
///
/// `truncate_tree_to_subtree_roots`
/// (`zcash_client_sqlite-0.22.0 wallet/commitment_tree.rs:730-793`) collects
/// survivors `WHERE subtree_end_height IS NOT NULL AND subtree_end_height <=
/// :truncation_height AND root_hash IS NOT NULL ORDER BY shard_index` and **breaks at
/// the first index gap** (`if shard_index != roots.len() { break }`), then
/// unconditionally `DELETE`s `{prefix}_tree_checkpoints`, `{prefix}_tree_shards` and
/// `{prefix}_tree_cap` and re-puts what it collected.
///
/// So what the collection cannot survive is a HOLE — index *i* NULL with *j > i*
/// populated, which stops every shard above *i* being re-put even though its own
/// height is fine. Withdrawing a whole suffix never makes one, at ANY starting index:
/// the non-NULL heights stay the dense prefix `put_shard_roots` writes and the
/// truncation reads. Withdrawing from 0 empties the table, which is the fresh-wallet
/// state and not a gap.
///
/// **What the widening costs:** the recorded heights above *i* are discarded with it,
/// and the shards they named are dropped from the next truncation's re-put. They are
/// re-fetched by the next accepted `update_subtree_roots`: a withdrawn height ends
/// the stored run ([`read_stored_run`]), so even an incremental pass (S15-F1,
/// ADR-0569) re-serves from at or below the withdrawn index, and the cost is one
/// round trip and not lost money.
///
/// # Index 0, and why the INVARIANT is guarded rather than the reachability argued
///
/// An empty survivor set leaves the cap DELETED (`put_shard_roots` returns early on
/// an empty slice), so a withdrawal that empties the whole prefix can cost a pool its
/// tree cap. Two arguments were made in turn that it could not happen, and **both
/// were wrong**:
///
/// 1. *"A withdrawal never changes index 0."* True, and it left the refuted number
///    ACTIVE in the two upstream consumers of that column —
///    `mark_stabilized_notes` for the latch and `v_{prefix}_shard_scan_ranges` for
///    the shard's block window. Refusing later re-serves does not undo a lie already
///    written.
/// 2. *"`ResetToSubtreeRoots` cannot fire, because `update_tree` checkpoints the
///    batch anchor unconditionally."* That covers checkpoint CREATION and says
///    nothing about SURVIVAL. `prune_excess_checkpoints` removes the oldest
///    checkpoints until at most `max_checkpoints` remain (`shardtree-0.7.1
///    lib.rs:644`), `max_checkpoints` is `PRUNING_DEPTH = 100`, and upstream's own
///    doc for the qualifying predicate names the post-migration-rescan geometry —
///    ours — as a reachable case. The branch may well fire.
///
/// **So the invariant is guarded where it lives, and the guard does not depend on
/// what upstream does with checkpoints:** [`withdraw_suffix`] refuses to leave the
/// pool's whole prefix NULL while any `root_hash` row exists. One `EXISTS` read. A
/// withdrawal from index 0 on a populated tree is clamped to index 1, which is still
/// a suffix, still ungapped, and still leaves index 0 bracketed and un-latched.
///
/// **What that costs, at index 0 only:** the refuted number stays in the shard-0 row,
/// so `v_{prefix}_shard_scan_ranges` keeps the wrong block window for shard 0 until
/// an honest serve overwrites it. The bracket refuses every re-serve of a height
/// outside it, and the un-latch is re-applied from the ledger on every batch, so the
/// false-spendable half is answered; the window is not. That is the price of not
/// risking a cap, and it is paid at one index rather than across the tree.
fn plan_withdrawal(corrections: &[BoundaryCorrection]) -> Option<usize> {
    corrections
        .iter()
        .filter(|c| c.truth.is_none())
        .map(|c| c.index)
        .min()
}

/// Withdraw `from` and every index above it to NULL, and un-latch every note at or
/// above shard `from`.
///
/// The un-latch is over the whole suffix and not over two shards, because the whole
/// suffix's block windows just changed: a recorded height is the END of its own
/// shard's window and the START of the next one's
/// (`v_{prefix}_shard_unscanned_ranges`, `zcash_client_sqlite-0.22.0
/// wallet/db.rs:1771-1785`), so NULLing `from..` moves every window from `from`
/// upward. `mark_stabilized_notes` cannot latch a shard whose `subtree_end_height IS
/// NULL` (`wallet/scanning.rs:547-548` — the predicate names the column explicitly),
/// so a withdrawal is conservative for FUTURE latching; what it cannot undo by itself
/// is a latch already set, and that is what this clears.
///
/// **THE CAP INVARIANT, guarded here and nowhere else.** A withdrawal that leaves the
/// pool's whole prefix NULL empties `truncate_tree_to_subtree_roots`' survivor set,
/// and `put_shard_roots` returns early on an empty slice — so the
/// `DELETE FROM {prefix}_tree_cap` that ran just before it is never undone. This
/// refuses to do that while the pool still holds any `root_hash` row: a withdrawal
/// from index 0 on a populated tree is CLAMPED to index 1. Still a suffix, so still
/// ungapped; index 0 keeps its (refuted) value, keeps its bracket, and keeps being
/// un-latched from the ledger every batch.
///
/// The guard is on the invariant rather than on an argument about whether
/// `ResetToSubtreeRoots` can fire — [`plan_withdrawal`] records the two reachability
/// arguments that were made for that branch and why both were wrong. This holds
/// whatever upstream does with checkpoints.
///
/// Below the clamp the only shape this function can produce is a suffix, so the
/// recorded heights it leaves behind are always a dense prefix.
fn withdraw_suffix(
    conn: &Connection,
    pool: ShieldedProtocol,
    from: usize,
) -> Result<(), WalletError> {
    let from = if from == 0 && pool_holds_a_root(conn, pool)? {
        1
    } else {
        from
    };
    let shard = u64::try_from(from).map_err(|_| WalletError::StoreCorrupt)?;
    let prefix = table_prefix(pool);
    conn.execute(
        &format!(
            "UPDATE {prefix}_received_notes SET witness_stabilized = 0
             WHERE witness_stabilized = 1
               AND commitment_tree_position IS NOT NULL
               AND (commitment_tree_position >> {SHARD_HEIGHT}) >= ?1"
        ),
        rusqlite::params![shard],
    )
    .map_err(map_aux_err)?;
    conn.execute(
        &format!(
            "UPDATE {prefix}_tree_shards SET subtree_end_height = NULL WHERE shard_index >= ?1"
        ),
        rusqlite::params![shard],
    )
    .map_err(map_aux_err)?;
    Ok(())
}

/// Record the bracket, undo the latch, then rewrite the number — **and never write a
/// NULL** (the order's argument is on [`reconcile_scanned_boundaries`];
/// [`plan_withdrawal`] owns every NULL this module writes).
///
/// **The bracket first, and that ordering is the crash-safety argument one level
/// up.** The bracket is the durable half: a kill after it is written and before the
/// row is touched leaves an ingest that refuses the wrong height on the next pass,
/// which is the conservative state. A kill the other way round would leave a
/// corrected row with no memory of what it was corrected FROM, which is the state the
/// fold shipped and the review's headline finding.
///
/// **`truth = None` writes nothing to the row here.** A refutation with no
/// replacement is a WITHDRAWAL, and a withdrawal is a decision about the whole
/// correction set (it moves a suffix, and it is clamped where a cap is at stake), so it is made
/// once in [`plan_withdrawal`] and applied once in [`withdraw_suffix`] rather than
/// row by row here. The one-row NULL that used to be written from this function is
/// the CRITICAL.
///
/// **Two shards are un-latched, not one** (§4x "does NOT decide" 2). A recorded
/// height is the END of shard `index`'s block window and the START of shard
/// `index + 1`'s (`v_{prefix}_shard_unscanned_ranges` reads it as
/// `IFNULL(prev_shard.subtree_end_height, activation)`,
/// `zcash_client_sqlite-0.22.0/src/wallet/db.rs:1771-1785`), so ONE wrong number
/// shrinks TWO windows: lowering it cuts shard `index` short from above —
/// §4w (a)'s measured false-spendable — and raising it cuts shard `index + 1`
/// short from below, which hides the blocks that carry its commitments from
/// exactly the unscanned-range check that is supposed to hold it back. The
/// minimal sound set is therefore both. **What the second one costs:** a note in
/// shard `index + 1` that was legitimately stabilized loses its spendability
/// until `mark_stabilized_notes` re-runs and re-earns it on a later batch — a
/// bounded, self-healing cost, paid in the conservative direction. **What
/// covering only shard `index` would cost:** the raise half of the same defect,
/// silently, on a shard nothing else in this module looks at.
fn apply_boundary_correction(
    conn: &Connection,
    pool: ShieldedProtocol,
    correction: &BoundaryCorrection,
) -> Result<(), WalletError> {
    // FAIL CLOSED on an index that is not a shard index. `.unwrap_or(u64::MAX)` stood
    // here and would have un-latched, and then rewritten, shard `u64::MAX` — a wrong
    // row rather than no row. Unreachable on any 64-bit host (`usize` == `u64`), and
    // written as a refusal anyway because the direction of the default was the bug.
    let shard = u64::try_from(correction.index).map_err(|_| WalletError::StoreCorrupt)?;
    unlatch_shard(conn, pool, shard)?;
    unlatch_shard(conn, pool, shard.saturating_add(1))?;
    // Only a REPLACEMENT is written from here. A withdrawal is the caller's decision
    // over the whole set — see this function's doc and [`plan_withdrawal`].
    let Some(truth) = correction.truth else {
        return Ok(());
    };
    let prefix = table_prefix(pool);
    conn.execute(
        &format!("UPDATE {prefix}_tree_shards SET subtree_end_height = ?2 WHERE shard_index = ?1"),
        rusqlite::params![shard, truth],
    )
    .map_err(map_aux_err)?;
    Ok(())
}

/// Does this pool hold ANY shard root? One indexed `EXISTS`, and it is the whole of
/// the cap invariant [`withdraw_suffix`] guards: `truncate_tree_to_subtree_roots`
/// re-puts only the survivors it collected, so emptying the prefix while roots exist
/// is what loses them and the cap with them. On a pool that holds none — a fresh
/// wallet, an Ironwood tree before `put_shard_roots` first runs — there is nothing to
/// lose and the withdrawal proceeds from index 0.
fn pool_holds_a_root(conn: &Connection, pool: ShieldedProtocol) -> Result<bool, WalletError> {
    let prefix = table_prefix(pool);
    conn.prepare_cached(&format!(
        "SELECT EXISTS(SELECT 1 FROM {prefix}_tree_shards WHERE root_hash IS NOT NULL)"
    ))
    .map_err(map_aux_err)?
    .query_row([], |row| row.get::<_, i64>(0))
    .map(|n| n != 0)
    .map_err(map_aux_err)
}

/// Clear `witness_stabilized` for every note of one shard — the only writer of a
/// `0` into that column SCOPED TO ONE SHARD. The other writer is
/// [`withdraw_suffix`], which clears a SUFFIX (`>= shard`) when a withdrawal moves
/// the recorded heights; upstream has none (`mark_stabilized_notes` `:544` is its
/// sole writer and it only ever sets `1`, which is what makes the latch one-way
/// and these two functions necessary). Stated as two rather than "only" because a
/// withdrawal's suffix covers shard `i + 1` by itself, which is exactly how the two
/// `shard + 1` calls at the correction site stayed unguarded through a review round
/// (§4ac row 5): every fixture that reached them was a withdrawal.
///
/// Scoped by the note's own position (`commitment_tree_position >> SHARD_HEIGHT`
/// is its shard index — the same arithmetic upstream's own predicate uses,
/// `wallet/scanning.rs:548`), so it touches no note outside the shard named.
fn unlatch_shard(conn: &Connection, pool: ShieldedProtocol, shard: u64) -> Result<(), WalletError> {
    let prefix = table_prefix(pool);
    conn.execute(
        &format!(
            "UPDATE {prefix}_received_notes SET witness_stabilized = 0
             WHERE witness_stabilized = 1
               AND commitment_tree_position IS NOT NULL
               AND (commitment_tree_position >> {SHARD_HEIGHT}) = ?1"
        ),
        rusqlite::params![shard],
    )
    .map_err(map_aux_err)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use orchard::tree::MerkleHashOrchard;
    use zcash_protocol::consensus::BlockHeight;

    /// T0-1c — the tip standing's reference height, pinned to the numbers the
    /// contract measured (P2) and tied to the per-pool lists it must bound: every
    /// row of every pool's `bundled_counts` sits at or below it, and — while every
    /// row decodes, which this asserts rather than assumes — it IS each list's
    /// last height. A bundle refresh that moves the tail moves these two
    /// constants, and moving them is a decision (`checkpoints/mod.rs` records the
    /// row counts for the same reason). The literal pin is what sees a
    /// `newest_bundled_height` that reads `.first()` (ruling F1: the `fetch_tip`
    /// row reads its floor from this fn and cannot).
    /// T0-1c-R2 (crypto #11) — the reference is the table's MAXIMUM height:
    /// a table whose newest row is not its last element (a reordering the
    /// compiled table's three CI pins forbid, and this fn no longer relies on)
    /// still answers with the newest row; an empty table disarms to `0`.
    /// Mutant: `.max()` reverted to `.last()` (the first assertion red).
    #[test]
    fn the_newest_row_is_the_tables_maximum_height_not_its_last_element() {
        let reordered: [(u32, &str, &str, &str, &str); 3] = [
            (100, "", "", "", ""),
            (300, "", "", "", ""),
            (200, "", "", "", ""),
        ];
        assert_eq!(newest_row_height(&reordered), 300);
        assert_eq!(newest_row_height(&[]), 0);
        for net in [Network::Main, Network::Test] {
            assert_eq!(
                newest_bundled_height(net),
                newest_row_height(checkpoints::treestate_table(net)),
                "{net:?}: one definition"
            );
        }
    }

    #[test]
    fn newest_bundled_height_is_the_tables_last_row_and_bounds_every_pools_counts() {
        assert_eq!(newest_bundled_height(Network::Main), 3_459_780);
        assert_eq!(newest_bundled_height(Network::Test), 4_301_840);
        for net in [Network::Main, Network::Test] {
            let newest = newest_bundled_height(net);
            for pool in [
                ShieldedProtocol::Sapling,
                ShieldedProtocol::Orchard,
                ShieldedProtocol::Ironwood,
            ] {
                let counts = bundled_counts(net, pool);
                assert!(
                    counts.iter().all(|&(h, _)| h <= newest),
                    "{net:?}/{pool:?}: a bundled row sits above the table's last row"
                );
                assert_eq!(
                    counts.last().map(|&(h, _)| h),
                    Some(newest),
                    "{net:?}/{pool:?}: the last decodable row is not the table's last row — \
                     a row failed to decode, and the standing and the counts now disagree"
                );
            }
        }
    }

    /// A served root at `height`. The root HASH is irrelevant to this module by
    /// construction — the cap owns that dimension — so a fixed canonical element
    /// is used everywhere.
    fn root(height: u32) -> CommitmentTreeRoot<MerkleHashOrchard> {
        let mut bytes = [0u8; 32];
        bytes[0] = 1;
        CommitmentTreeRoot::from_parts(
            BlockHeight::from_u32(height),
            MerkleHashOrchard::from_bytes(&bytes).unwrap(),
        )
    }

    fn roots(heights: &[u32]) -> Vec<CommitmentTreeRoot<MerkleHashOrchard>> {
        heights.iter().copied().map(root).collect()
    }

    #[test]
    fn one_block_cannot_hold_a_whole_subtree() {
        // The derivation `MIN_COMPLETION_GAP_BLOCKS` rests on, asserted rather
        // than left in prose: a subtree's worth of bare 32-byte note commitments
        // does not fit inside Zcash's 2,000,000-byte block-size limit.
        const MAX_BLOCK_SIZE: u64 = 2_000_000;
        const COMMITMENT_BYTES: u64 = 32;
        const { assert!(SUBTREE_LEAVES * COMMITMENT_BYTES > MAX_BLOCK_SIZE) };
        const { assert!(MIN_COMPLETION_GAP_BLOCKS == 2) };
    }

    #[test]
    fn consecutive_completion_heights_are_refused_without_any_evidence() {
        // Both shapes the contract names, on an evidence-free wallet.
        let ev = PoolEvidence::default();
        let compressed = roots(&[3_428_143, 3_428_144, 3_428_145, 3_428_146]);
        assert!(matches!(
            check_pool(&compressed, &[], &ev),
            Err(HeightBindRefusal::CompletionGap { index: 1, .. })
        ));
        let inflated = roots(&[3_475_496, 3_475_497, 3_475_498, 3_475_499]);
        assert!(matches!(
            check_pool(&inflated, &[], &ev),
            Err(HeightBindRefusal::CompletionGap { index: 1, .. })
        ));
        // ... and a sequence with chain-realistic gaps passes the same check, so
        // the refusals above are the shape and not the magnitude.
        let spaced = roots(&[3_451_206, 3_459_187, 3_467_168, 3_475_149]);
        assert!(check_pool(&spaced, &[], &ev).is_ok());
    }

    /// **REWRITTEN TWICE, and both rewrites are the point.** At T0-1a-R it asserted
    /// that a recorded height MAY move by up to `REORG_MAX_BLOCKS` with no rewind of
    /// any kind — the unconditional tolerance §4d Q4 ruled unsound. It was then re-aimed
    /// at the rewind gate, and **renamed here** (from
    /// `a_recorded_height_may_move_by_a_reorg_but_not_by_a_compression`) because §4e-R2's
    /// direction floor makes the old name false: a reorg licenses a DOWNWARD move, a
    /// compression IS a downward move, and this floor admits it. Renaming rather than
    /// leaving a name that describes behaviour the code no longer has — no registry row
    /// in `evals/` names it, checked before the rename.
    #[test]
    fn a_reorg_licenses_a_downward_correction_and_never_an_upward_one() {
        let recorded = vec![Some(3_451_206), Some(3_463_000)];
        let bound = PoolEvidence {
            recorded: recorded.clone(),
            ..PoolEvidence::default()
        };
        // The re-serve of exactly what is recorded is accepted (C2's shape).
        assert!(check_pool(&roots(&[3_451_206, 3_463_000]), &[], &bound).is_ok());
        // With NO rewind observed the bind is literal EQUALITY. One block is enough,
        // and so is 99 — the pre-repair code accepted both, every pass, forever, which
        // is the walk Q4 measured.
        for moved in [3_451_207, 3_451_206 + 99] {
            assert!(
                matches!(
                    check_pool(&roots(&[moved, 3_463_000]), &[], &bound),
                    Err(HeightBindRefusal::RecordedHeight { index: 0, .. })
                ),
                "a {}-block move with no rewind observed must be refused",
                moved - 3_451_206
            );
        }
        // After a rewind the wallet has observed and not yet consumed, the SAME index
        // relaxes DOWNWARD — by one block or by 5,149, since the answer is whether a
        // rewind happened and not how far the height moved. C15's move is this case.
        let after_rewind = PoolEvidence {
            recorded: recorded.clone(),
            rewind: RewindWatch {
                observed: 1,
                recorded_at: 0,
            },
            ..PoolEvidence::default()
        };
        assert!(check_pool(&roots(&[3_451_205, 3_463_000]), &[], &after_rewind).is_ok());
        assert!(check_pool(&roots(&[3_446_000, 3_463_000]), &[], &after_rewind).is_ok());
        assert!(
            check_pool(&roots(&[3_451_206, 3_463_000]), &[], &after_rewind).is_ok(),
            "at or below: an unchanged re-serve is still accepted after a rewind"
        );
        // AND THE FLOOR UNDER IT (§4e-R2 (c)): the same rewind does NOT license an
        // upward move. One block above the record is enough. This is the half whose
        // absence let the adjudicator drive a tail inflation through a rewind on a
        // wallet holding a correct record, and it is the erasure chain's necessary
        // condition, so it is refused whether the reorg could have produced it or not.
        assert!(
            matches!(
                check_pool(&roots(&[3_451_207, 3_463_000]), &[], &after_rewind),
                Err(HeightBindRefusal::RecordedHeight { index: 0, .. })
            ),
            "a rewind relaxes DOWNWARD only — one block above the record is refused"
        );
        assert!(matches!(
            check_pool(&roots(&[3_451_206, 3_475_299]), &[], &after_rewind),
            Err(HeightBindRefusal::RecordedHeight { index: 1, .. })
        ));
        // R4: once the roots are re-recorded the ledger's two counters agree again and
        // the NEW record binds by EQUALITY — so a downward move that the relaxed pass
        // would have admitted is refused. Same bytes, opposite answer, which is the
        // C15/C17 pair in miniature.
        let consumed = PoolEvidence {
            recorded,
            rewind: RewindWatch {
                observed: 1,
                recorded_at: 1,
            },
            ..PoolEvidence::default()
        };
        for moved in [3_451_205u32, 3_451_207] {
            assert!(
                matches!(
                    check_pool(&roots(&[moved, 3_463_000]), &[], &consumed),
                    Err(HeightBindRefusal::RecordedHeight { index: 0, .. })
                ),
                "with the rewind consumed, {moved} is refused in BOTH directions"
            );
        }
        // A hole in the record abstains at that index instead of shifting later
        // comparisons by one.
        let holed = PoolEvidence {
            recorded: vec![None, Some(3_463_000)],
            ..PoolEvidence::default()
        };
        assert!(check_pool(&roots(&[1_000_000, 3_463_000]), &[], &holed).is_ok());
    }

    /// **The attack that refuted the abstaining design, pinned at the arm that now
    /// refuses it.** Driven end to end by the T0-1a-R adjudication on a wallet holding
    /// a CORRECT record: with the record standing the tail inflation is refused, and
    /// **one induced rewind later it was accepted and written** as
    /// `[3451206, 3475299, 3475399, 3475499]` — index 0 left where the signed bundle
    /// admits it, the whole tail moved to within 200 blocks of the endpoint's own
    /// reported tip. That is the inflation shape the design review named as the
    /// cap-erasure trigger, bought back by a rewind an endpoint can induce at will.
    ///
    /// The numbers are the adjudication's, not fresh ones. The oracles that §4e
    /// expected to carry the relaxed pass are given here in the state they are really
    /// in on mainnet Ironwood — the newest bundled row, `(3_459_780, 1)`, and a scanned
    /// arm that abstains — so this case measures what actually stood between the
    /// endpoint and the write, and it is the direction floor and nothing else.
    ///
    /// **Predicted killing mutation:** relax the post-rewind branch to `false` (abstain
    /// at a recorded index, the first repair's behaviour). This goes red; every other
    /// assertion in this module stays green, which is exactly how the gap got shipped.
    #[test]
    fn a_rewind_does_not_license_the_tail_inflation_the_adjudicator_drove() {
        const RECORDED: [u32; 4] = [3_451_206, 3_463_000, 3_467_168, 3_475_149];
        const TAIL_INFLATION: [u32; 4] = [3_451_206, 3_475_299, 3_475_399, 3_475_499];

        let ev = PoolEvidence {
            recorded: RECORDED.iter().copied().map(Some).collect(),
            // The two oracles §4e believed would carry a relaxed pass, in the state
            // they are actually in for this pool: the bundle stops at 3,459,780 saying
            // ONE subtree was complete, and the scanned arm has nothing.
            bundled: vec![(3_459_780, 1)],
            scanned: None,
            rewind: RewindWatch {
                observed: 1,
                recorded_at: 0,
            },
            ..PoolEvidence::default()
        };

        // The bundled arm really does admit it — stated as an assertion so this case
        // cannot later pass for the wrong reason.
        assert!(
            check_against_count(&TAIL_INFLATION, 3_459_780, 1, CountOracle::Bundled).is_ok(),
            "IT-10: the newest bundled Ironwood row does NOT contradict this sequence, \
             so whatever refuses it below is the recorded arm and not the counting one"
        );
        assert!(
            check_completion_gap(&TAIL_INFLATION).is_ok(),
            "IT-10: nor does the block-size floor — the tail is spaced 100 blocks apart"
        );

        assert!(
            matches!(
                check_pool(&roots(&TAIL_INFLATION), &[], &ev),
                Err(HeightBindRefusal::RecordedHeight { index: 1, .. })
            ),
            "A REWIND IS A LICENCE TO CORRECT DOWNWARD, NOT TO INFLATE. Index 1 moved \
             UP from {} to {} — the direction that arms the cap erasure — and it must \
             be refused even though a rewind was observed",
            RECORDED[1],
            TAIL_INFLATION[1]
        );

        // ANTI-VACUITY, and it is the pair that makes the refusal above mean something:
        // the same wallet, the same rewind, a DOWNWARD correction at the same index is
        // accepted. A floor that refused both would be C15 red again.
        let corrected = [
            RECORDED[0],
            RECORDED[1],
            RECORDED[2],
            RECORDED[3] - 5_149, // C15's own measured move
        ];
        assert!(
            check_pool(&roots(&corrected), &[], &ev).is_ok(),
            "the honest post-rewind correction is still accepted"
        );
    }

    /// The rewind gate reaches all THREE oracles that are memories of a chain, and
    /// reaches NEITHER of the two that are not. Without this, each relaxation is a
    /// branch no test enters — the "mechanism nothing reaches" class.
    #[test]
    fn the_rewind_gate_relaxes_the_memory_oracles_and_nothing_else() {
        let rewound = RewindWatch {
            observed: 1,
            recorded_at: 0,
        };
        let mut ours = vec![7u8; 32];
        ours[0] = 9;
        let scanned_hashes = HashMap::from([(3_451_206u32, ours)]);

        // (d) the scanned count: "no subtree was complete at 3,470,000" refuses a root
        // claiming index 0 completed at or below it — until a rewind is observed.
        let scanned = PoolEvidence {
            scanned: Some((3_470_000, 0)),
            ..PoolEvidence::default()
        };
        assert!(matches!(
            check_pool(&roots(&[3_451_206]), &[], &scanned),
            Err(HeightBindRefusal::ScannedTreeSize { index: 0, .. })
        ));
        assert!(
            check_pool(
                &roots(&[3_451_206]),
                &[],
                &PoolEvidence {
                    rewind: rewound,
                    ..scanned
                }
            )
            .is_ok()
        );

        // (C6) the completing-hash cross-check.
        let hashes = PoolEvidence {
            scanned_hashes,
            ..PoolEvidence::default()
        };
        assert!(matches!(
            check_pool(&roots(&[3_451_206]), &[vec![3u8; 32]], &hashes),
            Err(HeightBindRefusal::CompletingBlockHash { index: 0, .. })
        ));
        assert!(
            check_pool(
                &roots(&[3_451_206]),
                &[vec![3u8; 32]],
                &PoolEvidence {
                    rewind: rewound,
                    ..hashes
                }
            )
            .is_ok()
        );

        // AND THE OTHER HALF, which is what stops this being a switch that turns the
        // module off: the gap floor and the bundled frontiers still bind after a
        // rewind. Neither is a memory of a chain — one is arithmetic, the other is in
        // the signed binary — so a reorg cannot make either stale.
        let bundled = PoolEvidence {
            bundled: vec![(3_459_780, 2)],
            rewind: rewound,
            ..PoolEvidence::default()
        };
        assert!(matches!(
            check_pool(&roots(&[3_451_206, 3_459_187, 3_459_400]), &[], &bundled),
            Err(HeightBindRefusal::BundledFrontier { index: 2, .. })
        ));
        assert!(matches!(
            check_pool(&roots(&[3_428_143, 3_428_144]), &[], &bundled),
            Err(HeightBindRefusal::CompletionGap { index: 1, .. })
        ));
    }

    /// The ledger itself, driven over a real connection: the counters start at zero
    /// (which is the BINDING reading), a rewind arms the gate for every pool, and a
    /// pool's own write disarms it for that pool ALONE.
    #[test]
    fn the_rewind_ledger_arms_every_pool_and_is_consumed_per_pool() {
        let conn = Connection::open_in_memory().expect("in-memory db");
        ensure_rewind_table(&conn).expect("ensure");
        ensure_rewind_table(&conn).expect("idempotent");

        let watch = |pool| read_rewind_watch(&conn, pool).expect("read");
        for pool in [
            ShieldedProtocol::Sapling,
            ShieldedProtocol::Orchard,
            ShieldedProtocol::Ironwood,
        ] {
            assert_eq!(watch(pool), RewindWatch::default());
            assert!(!watch(pool).rewound_since_record(), "the default BINDS");
        }

        note_rewind(&conn).expect("note the rewind");
        for pool in [
            ShieldedProtocol::Sapling,
            ShieldedProtocol::Orchard,
            ShieldedProtocol::Ironwood,
        ] {
            assert!(
                watch(pool).rewound_since_record(),
                "a rewind is a CHAIN event — it arms every pool"
            );
        }

        let observed = watch(ShieldedProtocol::Ironwood).observed;
        note_roots_recorded(&conn, ShieldedProtocol::Ironwood, observed).expect("consume");
        assert!(!watch(ShieldedProtocol::Ironwood).rewound_since_record());
        assert!(
            watch(ShieldedProtocol::Sapling).rewound_since_record(),
            "consumption is PER POOL: an endpoint that served only Ironwood has not \
             corrected Sapling's record"
        );

        // A second rewind re-arms the pool that had already consumed the first.
        note_rewind(&conn).expect("second rewind");
        assert!(watch(ShieldedProtocol::Ironwood).rewound_since_record());

        // ... and the rescan clear puts every scope back to the binding reading.
        clear_rewind_watch(&conn).expect("clear");
        assert_eq!(watch(ShieldedProtocol::Ironwood), RewindWatch::default());
        assert_eq!(watch(ShieldedProtocol::Sapling), RewindWatch::default());
    }

    #[test]
    fn the_counting_bind_is_two_sided() {
        // "two subtrees were complete at 3,459,780" is a statement about EVERY
        // index: 0 and 1 at or below it, 2 and up strictly above.
        let ev = PoolEvidence {
            bundled: vec![(3_459_780, 2)],
            ..PoolEvidence::default()
        };
        assert!(check_pool(&roots(&[3_451_206, 3_459_187, 3_467_168]), &[], &ev).is_ok());
        // Deflation: index 2 claims to have completed at or below the row.
        assert!(matches!(
            check_pool(&roots(&[3_451_206, 3_459_187, 3_459_400]), &[], &ev),
            Err(HeightBindRefusal::BundledFrontier { index: 2, .. })
        ));
        // Inflation: index 0 claims to have completed above a row that already
        // showed it complete.
        assert!(matches!(
            check_pool(&roots(&[3_460_000, 3_470_000]), &[], &ev),
            Err(HeightBindRefusal::BundledFrontier { index: 0, .. })
        ));
        // A short prefix is not a violation: the endpoint stopped early, which is
        // harmless, and refusing it would be worse than the bug.
        assert!(check_pool(&roots(&[3_451_206]), &[], &ev).is_ok());
    }

    #[test]
    fn the_scanned_count_refuses_with_its_own_reason() {
        let ev = PoolEvidence {
            scanned: Some((3_470_000, 2)),
            ..PoolEvidence::default()
        };
        assert!(matches!(
            check_pool(&roots(&[3_451_206, 3_459_187, 3_467_168]), &[], &ev),
            Err(HeightBindRefusal::ScannedTreeSize { index: 2, .. })
        ));
    }

    #[test]
    fn the_hash_cross_check_abstains_on_an_unscanned_block_and_refuses_on_a_scanned_one() {
        let mut ours = vec![7u8; 32];
        ours[0] = 9;
        let served_display: Vec<u8> = ours.iter().rev().copied().collect();
        let mut scanned = HashMap::new();
        scanned.insert(3_451_206u32, ours.clone());
        let ev = PoolEvidence {
            scanned_hashes: scanned,
            ..PoolEvidence::default()
        };
        // Agreement, in the display order a real lightwalletd serves.
        assert!(
            check_pool(
                &roots(&[3_451_206]),
                std::slice::from_ref(&served_display),
                &ev
            )
            .is_ok()
        );
        // Agreement, in the internal order — accepted on purpose (see the fn doc).
        assert!(check_pool(&roots(&[3_451_206]), std::slice::from_ref(&ours), &ev).is_ok());
        // Disagreement on a block the wallet SCANNED is a refusal.
        assert!(matches!(
            check_pool(&roots(&[3_451_206]), &[vec![3u8; 32]], &ev),
            Err(HeightBindRefusal::CompletingBlockHash { index: 0, .. })
        ));
        // The SAME disagreement at a height the wallet has NOT scanned abstains.
        assert!(check_pool(&roots(&[3_459_187]), &[vec![3u8; 32]], &ev).is_ok());
        // An absent or wrong-length hash abstains rather than refusing.
        assert!(check_pool(&roots(&[3_451_206]), &[], &ev).is_ok());
        assert!(check_pool(&roots(&[3_451_206]), &[vec![3u8; 7]], &ev).is_ok());
    }

    #[test]
    fn bundled_ironwood_frontiers_refute_the_measured_attack_offline() {
        // The signed binary's own answer to `docs/plan/probes/ironwood-a11-bypass`:
        // the compressed sequence claims four Ironwood subtrees complete by
        // 3,428,146, and there is a bundled mainnet row above that height showing
        // fewer. No network, no scanned blocks, no recorded shard rows.
        let counts = bundled_counts(Network::Main, ShieldedProtocol::Ironwood).to_vec();
        assert!(
            !counts.is_empty(),
            "the mainnet bundle must carry Ironwood frontier rows"
        );
        let ev = PoolEvidence {
            bundled: counts,
            ..PoolEvidence::default()
        };
        let compressed = roots(&[3_428_143, 3_428_243, 3_428_343, 3_428_443]);
        assert!(
            matches!(
                check_pool(&compressed, &[], &ev),
                Err(HeightBindRefusal::BundledFrontier { .. })
            ),
            "a compressed Ironwood sequence spaced past the gap check must still be refused \
             by the bundled frontiers alone"
        );
    }

    /// **The oracle checked against reality, not against itself.** A bound derived
    /// from the shipped bundle is worth exactly what the bundle is worth, so it is
    /// held here to the one independent observation this repository owns: the live
    /// `GetSubtreeRoots` capture in
    /// `docs/plan/probes/ironwood-subtree-roots-probe.output.txt` (zec.rocks
    /// mainnet + testnet, `max_entries = 0`, i.e. every completed subtree those
    /// servers knew of).
    ///
    /// Four of the six pool/network pairs are directly comparable and all four
    /// agree EXACTLY. The two Ironwood pairs are not directly comparable — their
    /// completions sit above the newest bundled row on both networks — and are
    /// asserted as the inequalities that must hold if the two sources describe the
    /// same chain.
    ///
    /// A bundle re-derivation that moved any of these numbers lands here rather
    /// than in a user's wallet refusing an honest server.
    #[test]
    fn bundled_counts_agree_with_the_live_subtree_root_capture() {
        let newest = |net, pool| -> (u32, u64) {
            *bundled_counts(net, pool)
                .last()
                .expect("every bundle carries rows")
        };

        // Directly comparable: the server streamed N roots and the newest bundled
        // row sits ABOVE the last of them, so the two must be the same N.
        assert_eq!(
            newest(Network::Main, ShieldedProtocol::Sapling),
            (3_459_780, 1128),
            "mainnet Sapling: the capture streamed 1128 roots, last at 3,390,009"
        );
        assert_eq!(
            newest(Network::Main, ShieldedProtocol::Orchard),
            (3_459_780, 769),
            "mainnet Orchard: the capture streamed 769 roots, last at 3,454,080"
        );
        assert_eq!(
            newest(Network::Test, ShieldedProtocol::Sapling),
            (4_301_840, 6),
            "testnet Sapling: the capture streamed 6 roots, last at 4,254,253"
        );
        assert_eq!(
            newest(Network::Test, ShieldedProtocol::Orchard),
            (4_301_840, 3),
            "testnet Orchard: the capture streamed 3 roots, last at 4,094,022"
        );

        // Testnet Ironwood: the capture's FIRST root completes at 4,307,326, above
        // the newest bundled row, so the bundle must show none complete there.
        assert_eq!(
            newest(Network::Test, ShieldedProtocol::Ironwood),
            (4_301_840, 0)
        );

        // Mainnet Ironwood: the capture's first root completes at 3,451,206 and its
        // last at 3,475,149 (4 roots). The newest bundled row, 3,459,780, sits
        // between the two, so the bundle must show at least one and fewer than four.
        let (h, n) = newest(Network::Main, ShieldedProtocol::Ironwood);
        assert_eq!(h, 3_459_780);
        assert!(
            (1..4).contains(&n),
            "mainnet Ironwood: 3,451,206 <= 3,459,780 < 3,475,149, so 1 <= complete < 4, got {n}"
        );

        // FINDING, recorded where it cannot be lost: `ironwood-unspendability-repro.py`'s
        // `SHARD_END_HEIGHTS` labels shards 1 and 2 (3,459,187 and 3,467,168) as
        // INTERPOLATED between the two MEASURED endpoints, and the shipped bundle
        // refutes 3,459,187 — only ONE mainnet Ironwood subtree was complete at
        // 3,459,780, so subtree 1 completed above that height. The interpolated
        // sequence is not a truthful one, and this bind refuses it, correctly.
        let ev = PoolEvidence {
            bundled: bundled_counts(Network::Main, ShieldedProtocol::Ironwood).to_vec(),
            ..PoolEvidence::default()
        };
        assert!(
            check_pool(&roots(&[3_451_206, 3_459_187]), &[], &ev).is_err(),
            "the probe's INTERPOLATED shard-1 height contradicts the shipped bundle"
        );
        // The measured first root, with a shard-1 height above the newest bundled
        // row, is accepted — which is the half that decides whether a real wallet
        // on a real server keeps working.
        assert!(check_pool(&roots(&[3_451_206, 3_466_000]), &[], &ev).is_ok());
    }

    /// The three pools, for the ledger rows below — a rewind is a CHAIN event and
    /// every one of them has to be read after each step.
    const EVERY_POOL: [ShieldedProtocol; 3] = [
        ShieldedProtocol::Sapling,
        ShieldedProtocol::Orchard,
        ShieldedProtocol::Ironwood,
    ];

    /// A fresh in-memory ledger for the T0-1a-R2 rows.
    fn ledger() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        ensure_rewind_table(&conn).expect("ensure");
        conn
    }

    /// **§4g 0a at the ledger's own level — the `Err` cell of the matrix.** A truncate
    /// that returned `Err` did not rewind (upstream's `truncate_to_height` is one
    /// transaction), so the note that preceded it is withdrawn and the ledger reads
    /// BINDING again — for every pool, and without disturbing a relaxation that an
    /// EARLIER, real rewind left standing on a pool not yet re-served. That second half
    /// is what separates "withdraw the failed attempt" from "clear the ledger": Orchard's
    /// record is still the pre-reorg one and the honest re-serve still needs its pass.
    ///
    /// **Predicted killing mutation:** make `unnote_rewind` a no-op — the pre-R2 tree,
    /// where the `Err` path propagated with the note left in place. The first
    /// assertion block goes red on every pool.
    #[test]
    fn a_truncate_that_fails_is_un_noted_and_the_ledger_binds_again() {
        let conn = ledger();
        let watch = |pool| read_rewind_watch(&conn, pool).expect("read");

        // The pair exactly as the continuity arm runs it on a truncate `Err`.
        note_rewind(&conn).expect("note, before the truncate");
        unnote_rewind(&conn).expect("un-note, because the truncate returned Err");
        for pool in EVERY_POOL {
            assert_eq!(
                watch(pool),
                RewindWatch::default(),
                "a truncate that did not run leaves the ledger exactly as it found it"
            );
            assert!(
                !watch(pool).rewound_since_record(),
                "and the reading is BINDING"
            );
        }

        // A REAL rewind stands (note, truncate Ok); Sapling is re-served and consumes
        // it; Orchard is not served and keeps the relaxation its stale record needs.
        note_rewind(&conn).expect("a rewind that happened");
        let observed = watch(ShieldedProtocol::Sapling).observed;
        note_roots_recorded(&conn, ShieldedProtocol::Sapling, observed).expect("consume");
        assert!(!watch(ShieldedProtocol::Sapling).rewound_since_record());
        assert!(
            watch(ShieldedProtocol::Orchard).rewound_since_record(),
            "precondition: orchard's record is still the pre-reorg chain's"
        );

        // Then a truncate that FAILS: only its own increment is withdrawn. Sapling stays
        // bound; Orchard keeps the pass the real rewind owes it.
        note_rewind(&conn).expect("note for the truncate that will fail");
        unnote_rewind(&conn).expect("un-note");
        assert_eq!(watch(ShieldedProtocol::Sapling).observed, observed);
        assert!(
            !watch(ShieldedProtocol::Sapling).rewound_since_record(),
            "a truncate that did not run arms nothing: sapling is bound by its record"
        );
        assert!(
            watch(ShieldedProtocol::Orchard).rewound_since_record(),
            "and the withdrawal took back only the failed attempt's increment — the \
             relaxation the earlier REAL rewind left on orchard survives"
        );
    }

    /// **§4g R13, second sentence — the kill cell, at the ledger's level.** The two
    /// writes cannot share a transaction, so a kill between them leaves ONE of the two
    /// false states, and the chosen order decides which. A kill after the note and
    /// before the truncate is the note standing alone: the ledger reads ARMED on a
    /// wallet that did not rewind. That is the direction this design LOSES, and this
    /// case pins why it is the safer one to lose: it costs exactly one relaxed pass per
    /// pool — consumed by that pool's next accepted write — and nothing re-arms it
    /// short of another note. The other order would lose rewound-but-unrecorded, which
    /// no write can ever consume; a rescan is its only exit. A kill AFTER the truncate
    /// is the same ledger state with a TRUE reading, and needs no case.
    #[test]
    fn a_kill_between_the_note_and_the_truncate_lands_on_armed_and_is_bounded() {
        let conn = ledger();
        let watch = |pool| read_rewind_watch(&conn, pool).expect("read");

        note_rewind(&conn).expect("note; the process dies before the truncate");
        for pool in EVERY_POOL {
            assert!(
                watch(pool).rewound_since_record(),
                "THE LOST DIRECTION: armed with no rewind performed"
            );
        }
        // Bounded: each pool's next accepted write consumes it ...
        for pool in EVERY_POOL {
            let observed = watch(pool).observed;
            note_roots_recorded(&conn, pool, observed).expect("consume");
            assert!(
                !watch(pool).rewound_since_record(),
                "one accepted write spends the whole of what the kill left"
            );
        }
        // ... and it stays consumed: no later pass re-arms it without a fresh note.
        for pool in EVERY_POOL {
            assert!(!watch(pool).rewound_since_record());
            assert_eq!(
                watch(pool).observed,
                1,
                "the count is the kill's one, not more"
            );
        }
    }

    /// **Decision 3, residual one: a fault in the compensation itself** — note landed,
    /// truncate failed, un-note failed. The connection is put into `query_only` so the
    /// ledger's OWN write fails (`SQLITE_READONLY`) rather than a fixture standing in
    /// for it. What it leaves is the kill cell's state — armed, no rewind — and it is
    /// bounded the same way: one relaxed pass per pool, consumed by the next accepted
    /// write.
    #[test]
    fn a_fault_in_the_compensation_leaves_the_relaxation_armed_for_one_pass() {
        let conn = ledger();
        let watch = |pool| read_rewind_watch(&conn, pool).expect("read");

        note_rewind(&conn).expect("note, before the truncate that will fail");
        conn.execute_batch("PRAGMA query_only = ON;")
            .expect("every write on this connection now faults");
        assert!(
            unnote_rewind(&conn).is_err(),
            "IT-10: the compensation's OWN write is what faulted here"
        );
        for pool in EVERY_POOL {
            assert!(
                watch(pool).rewound_since_record(),
                "the double fault: armed with no rewind — the stated residual"
            );
        }
        conn.execute_batch("PRAGMA query_only = OFF;")
            .expect("writable again");
        // Bounded to one pass per pool, exactly as the kill cell is.
        for pool in EVERY_POOL {
            let observed = watch(pool).observed;
            note_roots_recorded(&conn, pool, observed).expect("consume");
            assert!(!watch(pool).rewound_since_record());
        }
    }

    /// **Decision 3, residual two: a consume that fails after a successful put.** The
    /// put commits on the wallet connection and the consume on this one, so the shard
    /// rows can already BE the corrected record while the ledger still says the pool is
    /// armed. At this level the put is the premise and the ledger half is what is
    /// asserted: the failed consume leaves THAT pool armed and touches no other, and
    /// its next accepted write consumes it. One relaxed pass on a pool whose record is
    /// already correct is the residual — and the safer side, because the other order
    /// (consume, then put) would leave a pool consumed-but-unwritten, the deadlock.
    #[test]
    fn a_consume_that_fails_after_a_put_leaves_that_pool_armed_for_one_pass() {
        let conn = ledger();
        let watch = |pool| read_rewind_watch(&conn, pool).expect("read");

        note_rewind(&conn).expect("a rewind that happened");
        let observed = watch(ShieldedProtocol::Sapling).observed;
        note_roots_recorded(&conn, ShieldedProtocol::Sapling, observed)
            .expect("sapling's put landed and its consume followed");
        conn.execute_batch("PRAGMA query_only = ON;")
            .expect("every write on this connection now faults");
        assert!(
            note_roots_recorded(&conn, ShieldedProtocol::Orchard, observed).is_err(),
            "orchard's put landed (the premise) and its consume faulted"
        );
        assert!(
            !watch(ShieldedProtocol::Sapling).rewound_since_record(),
            "sapling's consume stands: a fault on one pool's consume touches no other"
        );
        assert!(
            watch(ShieldedProtocol::Orchard).rewound_since_record(),
            "orchard: written, still armed — the residual, one pass wide"
        );
        assert!(
            watch(ShieldedProtocol::Ironwood).rewound_since_record(),
            "ironwood was never served and keeps its pass"
        );
        conn.execute_batch("PRAGMA query_only = OFF;")
            .expect("writable again");
        note_roots_recorded(&conn, ShieldedProtocol::Orchard, observed)
            .expect("the next accepted write consumes it");
        assert!(!watch(ShieldedProtocol::Orchard).rewound_since_record());
    }

    // ── BIND-1: the scan as the oracle, at the arithmetic (§4x) ──────────────
    //
    // These drive [`reconcile_boundaries`] DIRECTLY, on hand-written counts. The
    // end-to-end rows — a real wallet, a real scan, the row rewritten and the latch
    // undone — are the test half's (`sync_bind_proof.rs`, B1-1..B1-3). What is
    // pinned here is the arithmetic those rows rest on, and each of §4w (d)'s three
    // contradictions is named by the row that drives it.

    /// A run of contiguous blocks all carrying the SAME tree size, in leaves.
    /// `size = 0` is the empty pool every scripted chain in this crate declares
    /// (§4w (f4)); anything above it is a live pool that has not yet completed a
    /// subtree.
    fn run(lo: u32, hi: u32, size: u64) -> Vec<(u32, u64)> {
        (lo..=hi).map(|h| (h, size)).collect()
    }

    /// §4w (d) case (2) — THE MEASURED ATTACK, caught at the lowered height. An
    /// endpoint got a compressed completion height written; the wallet then scans
    /// the block that record NAMES and counts a tree that has not reached one
    /// subtree yet. The record is refuted where it stands — BEFORE the true
    /// boundary, which is above everything scanned — so there is no truth to write
    /// and the row goes NULL.
    ///
    /// The pool is LIVE here (some commitments, fewer than 65,536), which is what
    /// separates this from `an_empty_pool_is_not_evidence_about_a_boundary` below.
    /// Mutant: drop clause (c) → nothing is refuted.
    ///
    /// **BIND-1-R: the BRACKET is the durable half and both ends are asserted here
    /// by value.**
    /// It is not the recorded height — it is the TOP of the run, because the wallet
    /// counted every block through 3,428,200 and subtree 0 had not completed at any
    /// of them. That is why the repair closes P3's whole 15,367-block window and not
    /// only the one re-served value: a value blacklist would leave the endpoint free
    /// to move its claim one block.
    #[test]
    fn a_lowered_completion_height_is_refuted_at_the_height_it_claims() {
        let observed = run(3_428_100, 3_428_200, SUBTREE_LEAVES - 1);
        let out = reconcile_boundaries(&[Some(3_428_143)], &observed);
        assert_eq!(
            out,
            vec![BoundaryCorrection {
                index: 0,
                recorded: 3_428_143,
                truth: None,
                floor: Some(3_428_200),
                // NO ceiling: the run never reaches a whole subtree, so nothing in
                // it bounds the boundary from ABOVE. This is the geometry a floor
                // alone does cover, and it is why a floor-only build looked right.
                ceiling: None,
            }],
            "the wallet counted {} leaves at the end of the block the record names — \
             one short of a subtree — so nothing completed there; and the strongest \
             thing this batch proves is that subtree 0 had not completed by the TOP of \
             the run, which is the floor every later serve is held to",
            SUBTREE_LEAVES - 1
        );
    }

    /// §4w (d) case (1) — the crossed boundary, with its own control. The scan
    /// crosses subtree 0's true boundary inside the batch, so a record one block
    /// off is corrected TO THE COUNTED TRUTH rather than merely withdrawn; a record
    /// that already IS the truth is not touched at all (B1-2's control, at the
    /// arithmetic). Mutant: return the crossing height without its predecessor
    /// check → the `off_by_one_below` case still passes and
    /// `the_first_observation_is_never_read_as_a_boundary` goes red.
    #[test]
    fn a_crossed_boundary_is_corrected_to_the_height_the_scan_counted() {
        // Subtree 0 completes in block 1_000_010: the tree crosses 65,536 leaves
        // there, so the completed-subtree count crosses 0 → 1 with it.
        let observed: Vec<(u32, u64)> = (1_000_000..=1_000_020)
            .map(|h| {
                (
                    h,
                    if h >= 1_000_010 {
                        SUBTREE_LEAVES
                    } else {
                        SUBTREE_LEAVES - 1
                    },
                )
            })
            .collect();
        for (recorded, why) in [
            (1_000_009, "one block BELOW the truth — refuted from below"),
            (1_000_011, "one block ABOVE the truth — refuted from above"),
        ] {
            assert_eq!(
                reconcile_boundaries(&[Some(recorded)], &observed),
                vec![BoundaryCorrection {
                    index: 0,
                    recorded,
                    truth: Some(1_000_010),
                    // The crossing's predecessor: 1,000,009 was counted and held
                    // fewer than a subtree, so every height at or below it is
                    // refuted at index 0 from now on.
                    floor: Some(1_000_009),
                    // The bracket closes to ONE height here — `(1_000_009,
                    // 1_000_010]` — which is the sharpest this module ever gets and
                    // is exactly the crossing it just wrote into the row.
                    ceiling: Some(1_000_010),
                }],
                "{why}: the wallet counted the crossing itself, so the row is \
                 rewritten to it and not merely emptied"
            );
        }
        assert_eq!(
            reconcile_boundaries(&[Some(1_000_010)], &observed),
            vec![],
            "THE CONTROL: a record that agrees with the scanned crossing is not \
             refuted and nothing is written — without this the row would 'pass' by \
             correcting everything it looks at"
        );
    }

    /// §4w (d) case (3) — inflation, caught at the TRUE boundary. The record puts
    /// subtree 1 above the whole batch while the batch's own counts already show
    /// two subtrees complete inside it. Index 0's record is consistent and is left
    /// alone, which is what keeps this row about the inflated index.
    #[test]
    fn an_inflated_completion_height_is_refuted_at_the_true_boundary() {
        let observed = vec![
            (100, SUBTREE_LEAVES - 1),
            (101, SUBTREE_LEAVES),
            (102, 2 * SUBTREE_LEAVES),
            (103, 2 * SUBTREE_LEAVES),
        ];
        assert_eq!(
            reconcile_boundaries(&[Some(101), Some(9_999)], &observed),
            vec![BoundaryCorrection {
                index: 1,
                recorded: 9_999,
                truth: Some(102),
                floor: Some(101),
                ceiling: Some(102),
            }],
            "subtree 1 completed at 102 by the wallet's own count, so a record above \
             the batch cannot be true — and index 0, which agrees, is untouched"
        );
    }

    /// The ABSTENTION, which is the majority case on every mid-heal wallet: an
    /// index whose true boundary sits above everything scanned, and an index with
    /// no record at all. Neither is a correction, and — the half that matters —
    /// neither is an acceptance: this function judging nothing is not this function
    /// agreeing. Mutant: make an unreached index a correction → both assertions red.
    #[test]
    fn an_index_no_observation_reaches_is_abstained_on_not_accepted() {
        // Every scanned block already holds one complete subtree, so a record for
        // index 0 at any height at or below them is consistent and a record for
        // index 1 anywhere above them is not yet contradicted.
        let observed = run(200, 202, SUBTREE_LEAVES);
        assert_eq!(
            reconcile_boundaries(&[Some(150), Some(9_000)], &observed),
            vec![],
            "nothing scanned reaches either index — abstain"
        );
        assert_eq!(
            reconcile_boundaries(&[None, None], &observed),
            vec![],
            "a row with no recorded height (the mid-heal state) has nothing to refute"
        );
        assert_eq!(
            reconcile_boundaries(&[Some(150)], &[]),
            vec![],
            "a batch that observed nothing says nothing"
        );
    }

    /// **The two guards on the TRUTH, which are the difference between correcting a
    /// row and re-signing the lie.** A crossing may only be read as the boundary
    /// when the block one BELOW it was observed too and had not reached the subtree
    /// yet. Without the predecessor, a span that starts already past the boundary
    /// would name its own first block as the completion height — the compression
    /// lie, written by us. Without the contiguity check, a skipped row (a NULL
    /// `{prefix}_commitment_tree_size`) would let a gap stand in for the blocks it
    /// hides. Both fall back to a refutation with no replacement — which since
    /// BIND-1-R is a floor plus a withdrawal the caller may or may not be allowed to
    /// make ([`plan_withdrawal`]), not the "always available and never wrong" NULL
    /// this comment used to claim.
    ///
    /// The two halves also separate the two kinds of refutation, and their floors say
    /// so: the started-past case is an INFLATION (the record sits above a block that
    /// already showed the subtree complete) and mints NO floor, because nothing here
    /// bounds the boundary from below but DOES bound it from above; the gapped case
    /// is a compression at the claim and mints both.
    #[test]
    fn the_first_observation_is_never_read_as_a_boundary() {
        // The span starts ALREADY past subtree 0's boundary: every block holds one
        // complete subtree and nothing below was seen.
        let started_past = run(500, 501, SUBTREE_LEAVES);
        assert_eq!(
            reconcile_boundaries(&[Some(900)], &started_past),
            vec![BoundaryCorrection {
                index: 0,
                recorded: 900,
                truth: None,
                // NO floor: this is clause (b), and the record is ABOVE the true
                // boundary, so nothing in the run bounds it from BELOW.
                floor: None,
                // And NO ceiling either, because the only observation that could
                // supply one is `observed[0]` — the batch ANCHOR. A bound that rests
                // on the one number the endpoint supplied is a durable refusal the
                // server chose, so this run refutes the record and persists nothing
                // about it. `a_record_above_a_subtree_the_scan_saw_complete_is_refused_by_the_ceiling`
                // drives the same shape with one scanned block underneath, where the
                // ceiling IS mintable.
                ceiling: None,
            }],
            "the record is refuted (subtree 0 was complete by 501) but 500 is NOT the \
             boundary — it is only the first block we looked at, and a bound may not \
             rest on it"
        );
        // A hole at 501 (a NULL tree size, skipped by `read_scanned_sizes`).
        let gapped = vec![(500, SUBTREE_LEAVES - 1), (502, SUBTREE_LEAVES)];
        assert_eq!(
            reconcile_boundaries(&[Some(500)], &gapped),
            vec![BoundaryCorrection {
                index: 0,
                recorded: 500,
                // No floor: the only incomplete observation is `observed[0]`, the
                // anchor, and a bound may not rest on it.
                floor: None,
                truth: None,
                ceiling: Some(502),
            }],
            "the crossing is somewhere in 501..=502 and the wallet cannot say which, \
             so the row is withdrawn rather than given a height we did not count — \
             and 500 is proved too low, which is a floor even though the exact \
             boundary is not known"
        );
        assert_eq!(
            crossing_height(&gapped, 0),
            None,
            "and the truth helper says so directly"
        );
    }

    /// **THE RESTRICTION, and the row that exists because its absence cost 27 reds.**
    /// An absence of commitments in THIS batch's blocks says nothing about a
    /// boundary somewhere else. A record the span does not contain, in a span that
    /// completes nothing, is abstained on; the SAME record is refuted the moment a
    /// span contains the block it names. The two halves differ only in where the
    /// scan is, which is the whole point.
    ///
    /// The numbers are the shape the crate's own fixtures have: real mainnet
    /// Sapling completion heights (`bundled_counts`, last at 3,390,009) recorded on
    /// a wallet whose scripted chain declares `{prefix}_commitment_tree_size = 0`
    /// on every block it scans near the tip (§4w (f4)). Mutant: refute on any
    /// observation at or above the record (the two-sided rule applied at a
    /// distance) → the first assertion goes red, and so do 27 rows across
    /// `degraded_pool_proof`, `sync_bind_proof` and `wallet::tests`.
    #[test]
    fn a_record_the_span_does_not_contain_is_not_refuted_by_a_span_that_completes_nothing() {
        // A LIVE pool — so this row measures the span restriction and not the
        // liveness gate: 1,128 complete subtrees and a few hundred leaves over.
        let live = 1_128 * SUBTREE_LEAVES + 500;
        let near_the_tip = run(3_455_000, 3_455_100, live);
        assert_eq!(
            reconcile_boundaries(&[Some(3_390_009), Some(3_454_080)], &near_the_tip),
            vec![],
            "the scan is a hundred blocks near the tip; neither record names a block \
             in it, and neither index is one the span shows already complete — so \
             there is nothing here that speaks about a boundary a hundred thousand \
             blocks BELOW"
        );
        let containing = run(3_390_000, 3_390_100, SUBTREE_LEAVES - 1);
        assert_eq!(
            reconcile_boundaries(&[Some(3_390_009)], &containing),
            vec![BoundaryCorrection {
                index: 0,
                recorded: 3_390_009,
                truth: None,
                floor: Some(3_390_100),
                ceiling: None,
            }],
            "the same record, once the wallet has scanned the very block it names: \
             the tree at the end of 3,390,009 was one leaf short of a subtree, so the \
             record cannot be true"
        );
    }

    /// **THE LIVENESS GATE, and the row that exists because its absence cost 27
    /// reds.** A declared tree size of ZERO is the absence of the pool, not a small
    /// number: it is what a chain below the pool's activation looks like and what
    /// every scripted chain in this crate declares on every block (§4w (f4)). Read
    /// as evidence it refutes every recorded height the scan passes, which is how a
    /// truthful serve and a lie become indistinguishable — the vacuity shape, with
    /// the wallet's own signature on it.
    ///
    /// Both halves are asserted at ONE height with ONE record: only the pool's size
    /// differs. Mutant: drop [`pool_is_live`] → the first assertion goes red, and so
    /// do 20 `degraded_pool_proof` rows, 3 `sync_bind_proof` rows and 4
    /// `wallet::tests` rows (measured, 1545/29 against a 2-failure baseline).
    #[test]
    fn an_empty_pool_is_not_evidence_about_a_boundary() {
        let claimed = 3_455_050;
        let empty = run(3_455_000, 3_455_100, 0);
        assert_eq!(
            reconcile_boundaries(&[Some(claimed)], &empty),
            vec![],
            "the wallet counted NO commitments for this pool anywhere in the batch — \
             that is a stream carrying nothing, not a chain saying no subtree \
             completed, and it may not refute a record"
        );
        let live = run(3_455_000, 3_455_100, 1);
        assert_eq!(
            reconcile_boundaries(&[Some(claimed)], &live),
            vec![BoundaryCorrection {
                index: 0,
                recorded: claimed,
                truth: None,
                floor: Some(3_455_100),
                ceiling: None,
            }],
            "ONE leaf is all it takes: the pool is live, so a count at the block the \
             record names is a fact about the chain, and it says subtree 0 had not \
             completed there"
        );
        // And the FLOOR carries the same gate, asserted rather than inferred from the
        // correction above: an empty run mints none, so a chain that declares zeros
        // cannot persist a refutation of every honest height in it (§4w (f4)).
        assert_eq!(
            incomplete_through(&empty, 0),
            None,
            "a run of zeros is the absence of the pool — it may not mint a DURABLE \
             claim about where a boundary is, which is the 27-red defect made permanent"
        );
        assert_eq!(
            incomplete_through(&live, 0),
            Some(3_455_100),
            "one leaf makes the same run evidence, and the floor is the top of it"
        );
        // And a run of ONE observation mints nothing: the first element of a run can
        // be the batch's anchor, the one number in it the endpoint supplied, and a
        // DURABLE refusal a server could choose outright is R3's deadlock inverted.
        assert_eq!(
            incomplete_through(&live[..1], 0),
            None,
            "a single observation may be the endpoint's own anchor — it may not mint \
             a floor by itself"
        );
    }

    /// The refusal this reconcile raises carries a stable, §5.4-safe code like
    /// every other member of the enum, and it is a code of its OWN — a scan-time
    /// refutation reported under `scanned_tree_size` would be indistinguishable
    /// from the INGEST oracle of nearly the same name, which is the conflation
    /// §4w (f3) cost two sessions.
    #[test]
    fn the_scanned_boundary_refusal_has_a_code_of_its_own() {
        let refusal = HeightBindRefusal::ScannedBoundary {
            index: 0,
            recorded: 1,
            truth: None,
        };
        assert_eq!(refusal.code(), "scanned_boundary");
        for other in [
            HeightBindRefusal::ScannedTreeSize {
                index: 0,
                served: 1,
                at_height: 1,
                complete: 0,
            },
            HeightBindRefusal::RecordedHeight {
                index: 0,
                recorded: 1,
                served: 2,
            },
        ] {
            assert_ne!(
                refusal.code(),
                other.code(),
                "one code per oracle: {other:?} is a different question asked at a \
                 different time"
            );
        }
    }

    // ── BIND-1-R: the refutation is what must be durable ─────────────────────
    //
    // The end-to-end rows — a real wallet, two passes, the wrong height refused and
    // the truth accepted — are the test half's. What is pinned here is the arithmetic
    // and the SQL those rows rest on, and every refusal is asserted by VARIANT
    // (§4x-R R12).

    /// **THE GEOMETRY A FLOOR ALONE WAS INERT ON, at the join's own numbers.**
    ///
    /// The first build of this repair stored a floor and nothing else. A floor refuses
    /// a completion height that is too LOW; it is minted from an observation showing
    /// the subtree still INCOMPLETE. On a run where every block already holds a whole
    /// subtree there is no such observation, so nothing was minted, nothing was
    /// refused, and the re-serve landed exactly as at the base — the guard was green
    /// and dead. Measured at the join on this run, and the first two assertions
    /// are that measurement kept as a test rather than as a sentence:
    ///
    /// ```text
    /// PROBE-ADJ incomplete_through(idx 0) = None
    ///           first_complete_at(idx 0)  = Some((3450000, 65537))
    /// ```
    ///
    /// Mutant: make [`complete_by`] return `None` → this row goes red at the ceiling
    /// assertion AND at the refusal, while every floor-side row in this module stays
    /// green. That is the whole point: a floor-only build passes its own rows.
    #[test]
    fn a_record_above_a_subtree_the_scan_saw_complete_is_refused_by_the_ceiling() {
        // Every block of the run declares one whole subtree plus a leaf, so subtree 0
        // was already complete at the very first observation and NOTHING in the run
        // shows it incomplete.
        let flat: Vec<(u32, u64)> = (3_450_000..=3_450_100)
            .map(|h| (h, SUBTREE_LEAVES + 1))
            .collect();
        assert_eq!(
            incomplete_through(&flat, 0),
            None,
            "the measured fact: there is no floor to mint here, which is why a \
             floor-only ledger persisted nothing on this geometry"
        );
        // **AND NOT A CEILING EITHER, on THIS run — the join's second
        // CRITICAL, measured on the row that was built over it.** Every observation
        // already holds a whole subtree, so `partition_point` returns 0 and the only
        // candidate bound is `observed[0]`: the batch ANCHOR, the one number in the
        // run the endpoint supplied. Persisting that is a durable refusal a server
        // chose for us. The guard is on the INDEX now, not on the run's length.
        assert_eq!(
            complete_by(&flat, 0),
            None,
            "a bound may not rest on the anchor, however long the run is"
        );
        assert!(
            reconcile_boundaries(&[Some(3_450_060)], &flat)[0]
                .ceiling
                .is_none(),
            "so this run still REFUTES the record — the correction fires — and carries \
             no ceiling with it"
        );
        assert!(
            !observed_bounds(&[Some(3_450_060)], &flat)
                .iter()
                .any(|(index, _)| *index == 0),
            "and nothing durable is minted AT INDEX 0, which is the index whose only \
             candidate bound was the anchor. (Index 1 IS bracketed here, from a floor \
             resting on the run's last SCANNED block — the guard is about which \
             observation a bound rests on, not about refusing to bound at all.)"
        );
        // THE SAME GEOMETRY, with the anchor sitting below the boundary and a GAP
        // between it and the batch (a NULL `{prefix}_commitment_tree_size` row, which
        // `read_scanned_sizes` skips). The gap is what keeps `crossing_height` from
        // firing — it needs the block immediately below — so this stays the
        // "refuted, no truth in hand" case, while the ceiling now rests on
        // `observed[1]`, a block this wallet SCANNED.
        let observed: Vec<(u32, u64)> = std::iter::once((3_449_990u32, SUBTREE_LEAVES - 1))
            .chain((3_450_000..=3_450_100).map(|h| (h, SUBTREE_LEAVES + 1)))
            .collect();
        assert_eq!(
            crossing_height(&observed, 0),
            None,
            "the gap means the wallet cannot name the completing block, so there is no \
             truth to write — which is the geometry this row is about"
        );
        assert_eq!(
            incomplete_through(&observed, 0),
            None,
            "still no floor: the only incomplete observation IS the anchor, and a \
             floor may not rest on it either"
        );
        assert_eq!(
            complete_by(&observed, 0),
            Some(3_450_000),
            "and the repair: the subtree was already complete at the first SCANNED \
             block, so its true boundary is at or below that"
        );
        // A record naming a block above it — the endpoint's claim that subtree 0
        // completed later than the wallet's own counts allow.
        const LIE: u32 = 3_450_060;
        let corrections = reconcile_boundaries(&[Some(LIE)], &observed);
        assert_eq!(
            corrections,
            vec![BoundaryCorrection {
                index: 0,
                recorded: LIE,
                truth: None,
                floor: None,
                ceiling: Some(3_450_000),
            }],
            "the correction carries the CEILING and no floor — persist only the floor \
             and this index is left with no durable claim at all"
        );
        // And the ingest oracle then refuses the re-serve on the next pass, from the
        // bracket the correction just minted — asserted by VARIANT, on a wallet whose
        // row still holds the refuted height — on a pool that holds root hashes a
        // withdrawal from 0 is clamped to 1, so index 0 keeps its value and the
        // bracket is what binds it ([`withdraw_suffix`]'s cap invariant).
        let ev = PoolEvidence {
            recorded: vec![Some(LIE)],
            bounds: HashMap::from([(
                0usize,
                BoundaryBounds {
                    floor: corrections[0].floor,
                    ceiling: corrections[0].ceiling,
                },
            )]),
            ..PoolEvidence::default()
        };
        assert!(
            matches!(
                check_pool(&roots(&[LIE]), &[], &ev),
                Err(HeightBindRefusal::RefutedHeight { index: 0, .. })
            ),
            "R2 on this geometry: the re-serve is refused where a floor-only build \
             accepted it. Got {:?}",
            check_pool(&roots(&[LIE]), &[], &ev)
        );
        assert!(
            check_pool(&roots(&[3_450_000]), &[], &ev).is_ok(),
            "R3 on this geometry: the honest answer is at or below the ceiling and is \
             ACCEPTED — the bracket refuses a direction, never an index"
        );
    }

    /// **R2 AND R3 on ONE bracket, which is the only way either means anything —
    /// and on BOTH ends of it.** The bracket says the wallet counted the blocks and
    /// subtree 0 completed somewhere in `(3_468_000, 3_470_000]`. On that one row:
    /// a height at or below the floor is refused, a height above the ceiling is
    /// refused, and everything inside is ACCEPTED.
    ///
    /// **The acceptance assertions are the ones that are easy to fail while passing
    /// the refusals.** A repair that refuses by pinning harder refuses the honest
    /// answer too, and R3 calls that deadlock worse than the bug it fixes — it is why
    /// the withdrawal-to-NULL existed at all. The interval is open below and closed
    /// above because that is what the two counts actually prove, not for symmetry.
    ///
    /// Mutant: delete [`check_refuted_heights`]' call in [`check_pool`] → every
    /// refusal goes red and every acceptance still passes, which is the asymmetry.
    #[test]
    fn a_height_outside_the_scanned_bracket_is_refused_and_one_inside_is_accepted() {
        const FLOOR: u32 = 3_468_000;
        const CEILING: u32 = 3_470_000;
        const TOO_LOW: u32 = 3_459_782;
        let ev = PoolEvidence {
            // The row still holds a refuted height: on a pool holding root hashes
            // the withdrawal is clamped away from index 0 ([`withdraw_suffix`]'s cap
            // invariant), so this is the state a real wallet is in.
            recorded: vec![Some(TOO_LOW)],
            bounds: HashMap::from([(
                0usize,
                BoundaryBounds {
                    floor: Some(FLOOR),
                    ceiling: Some(CEILING),
                },
            )]),
            ..PoolEvidence::default()
        };
        for (served, why) in [
            (
                TOO_LOW,
                "the re-served too-low height — P3's geometry, refused where it was \
                 accepted",
            ),
            (
                TOO_LOW + 1,
                "and one block up, which a VALUE blacklist would have let through",
            ),
            (
                FLOOR,
                "the floor itself: the last height PROVED incomplete is refused",
            ),
            (
                CEILING + 1,
                "one block ABOVE the ceiling — the direction a floor-only ledger \
                 could not refuse at all, and the direction the S269 join measured \
                 the first build inert on",
            ),
            (
                CEILING + 50_000,
                "and far above it: the ceiling closes the whole upper window, not one \
                 value",
            ),
        ] {
            assert!(
                matches!(
                    check_pool(&roots(&[served]), &[], &ev),
                    Err(HeightBindRefusal::RefutedHeight { index: 0, .. })
                ),
                "{why}: expected a RefutedHeight refusal, got {:?}",
                check_pool(&roots(&[served]), &[], &ev)
            );
        }
        for (served, why) in [
            (
                FLOOR + 1,
                "R3 — THE UN-STICK, lower end: the first height the wallet's own \
                 counts do NOT refute is accepted, on the same wallet, at the same \
                 index, one pass later",
            ),
            (
                CEILING,
                "R3, upper end: the ceiling is the last height PROVED complete, so it \
                 is itself admissible — the interval is (floor, ceiling], and getting \
                 that end closed rather than open is what keeps an honest server \
                 whose boundary IS the ceiling from being refused",
            ),
        ] {
            assert!(
                check_pool(&roots(&[served]), &[], &ev).is_ok(),
                "{why}. Without this the repair is a deadlock. Got {:?}",
                check_pool(&roots(&[served]), &[], &ev)
            );
        }
        // And the same call twice: the check is a function of the stored bracket, not
        // of a one-shot latch, so a second pass refuses it again. (The DURABILITY of
        // the bracket across passes is the store's, proved end to end by the test
        // half.)
        assert!(matches!(
            check_pool(&roots(&[TOO_LOW]), &[], &ev),
            Err(HeightBindRefusal::RefutedHeight { .. })
        ));
        // A bracket with only ONE end still refuses on that end and abstains on the
        // other — an absent bound may never refuse, which is the guarding default and
        // what keeps a half-observed index from being bound to nothing at all.
        let ceiling_only = PoolEvidence {
            recorded: vec![Some(CEILING + 9)],
            bounds: HashMap::from([(
                0usize,
                BoundaryBounds {
                    floor: None,
                    ceiling: Some(CEILING),
                },
            )]),
            ..PoolEvidence::default()
        };
        assert!(matches!(
            check_pool(&roots(&[CEILING + 1]), &[], &ceiling_only),
            Err(HeightBindRefusal::RefutedHeight { index: 0, .. })
        ));
        assert!(
            check_pool(&roots(&[1]), &[], &ceiling_only).is_ok(),
            "no floor means no lower refusal — an absent bound refuses nothing"
        );
    }

    /// **The relaxation R3 needs, isolated.** A row the scan has refuted must stop
    /// equality-binding its index, or the ONLY height the ingest accepts there is the
    /// lie itself and the only height it refuses is the truth — the bind inverted.
    ///
    /// Asserted in BOTH directions, and against its own control: a row refuted from
    /// below and a row refuted from above both stop binding, while an index whose
    /// bracket CONTAINS the recorded height still binds by equality — so the
    /// relaxation is scoped to rows the scan actually contradicted and is not a
    /// standing hole.
    #[test]
    fn a_row_the_scan_refuted_stops_equality_binding_its_index() {
        let bracket = |floor, ceiling| HashMap::from([(0usize, BoundaryBounds { floor, ceiling })]);
        // Refuted from BELOW: the row is under the floor.
        assert_eq!(
            check_recorded_heights(
                &[3_469_000],
                &[Some(3_459_782)],
                &bracket(Some(3_468_000), Some(3_470_000)),
                false
            ),
            Ok(()),
            "the row holds a height the floor refutes, so it is not a record to bind \
             an honest server to"
        );
        // Refuted from ABOVE: the row is over the ceiling. This is the half a
        // floor-only build could not express, and without it the ONLY height accepted
        // at such an index is the too-high lie the row holds.
        assert_eq!(
            check_recorded_heights(
                &[3_469_000],
                &[Some(3_480_000)],
                &bracket(None, Some(3_470_000)),
                false
            ),
            Ok(()),
            "the row holds a height the ceiling refutes — equality there would admit \
             only the lie and refuse the truth"
        );
        // THE CONTROL, and it is what keeps this from being a blanket relaxation: a
        // bracket that CONTAINS the recorded height says nothing against that row, so
        // equality stands and a differing serve is refused.
        assert!(
            matches!(
                check_recorded_heights(
                    &[3_470_000],
                    &[Some(3_459_782)],
                    &bracket(Some(3_400_000), Some(3_500_000)),
                    false
                ),
                Err(HeightBindRefusal::RecordedHeight { index: 0, .. })
            ),
            "a bracket that contains the recorded height leaves equality in force"
        );
        // And with no bracket at all — the ordinary wallet — nothing changes.
        assert!(matches!(
            check_recorded_heights(&[3_470_000], &[Some(3_459_782)], &HashMap::new(), false),
            Err(HeightBindRefusal::RecordedHeight { index: 0, .. })
        ));
    }

    /// **The rewind gate on the bracket, and the cost it buys, both asserted, on
    /// BOTH ends.** A bracket is a memory of a chain counted from blocks a rewind
    /// deletes, so it abstains on the same predicate oracles (b), (d) and (C6)
    /// abstain on.
    ///
    /// What that COSTS is the first assertions: an endpoint that induced a rewind
    /// gets one pass in which a refuted height is not refused by this oracle. What it
    /// BUYS is that a reorg which genuinely MOVED a boundary — in either direction,
    /// out of a bracket recorded honestly before the fork — does not lock an honest
    /// server out with no exit but a rescan. Two-sidedness makes this gate matter
    /// MORE, not less: a bracket can be escaped in both directions, so there are
    /// twice as many honest reorgs it could otherwise wrongly refuse. The window is
    /// the identical one §4b owed row 7 already leaves open, which the plant row is
    /// red about.
    #[test]
    fn the_bracket_abstains_for_the_one_pass_a_rewind_buys() {
        let bounds = HashMap::from([(
            0usize,
            BoundaryBounds {
                floor: Some(3_468_000),
                ceiling: Some(3_470_000),
            },
        )]);
        for served in [3_459_782u32, 3_480_000] {
            assert_eq!(
                check_refuted_heights(&[served], &bounds, true),
                Ok(()),
                "rewound since this pool's heights were recorded: the blocks this \
                 bracket was counted from are the ones the truncate deleted, and that \
                 is true of the ceiling exactly as it is of the floor"
            );
        }
        assert!(
            matches!(
                check_refuted_heights(&[3_459_782], &bounds, false),
                Err(HeightBindRefusal::RefutedHeight {
                    index: 0,
                    served: 3_459_782,
                    floor: Some(3_468_000),
                    ceiling: Some(3_470_000),
                })
            ),
            "and with no rewind outstanding the same bytes are refused from below — \
             the gate is the only difference"
        );
        assert!(
            matches!(
                check_refuted_heights(&[3_480_000], &bounds, false),
                Err(HeightBindRefusal::RefutedHeight { index: 0, .. })
            ),
            "... and from above"
        );
    }

    /// **R1, at the property the survivor query actually turns on — DENSITY.**
    ///
    /// `truncate_tree_to_subtree_roots` collects survivors ordered by `shard_index`
    /// and BREAKS at the first index gap, then deletes the shards and the cap
    /// unconditionally and re-puts only what it collected. So the shape it cannot
    /// read is a HOLE: index *i* NULL with *j > i* populated stops every shard above
    /// *i* being re-put even though its own height is fine. A withdrawal that always
    /// moves a whole SUFFIX cannot make one **at any starting index, including 0 and
    /// including the empty result** — that is the invariant, and it is what this row
    /// asserts, index by index, over the transcribed query.
    ///
    /// The query is transcribed rather than driven for §4c's reason: the real branch
    /// needs a wallet whose checkpoints all sit above the rewind target, which no
    /// fixture in this crate builds. The end-to-end row is the test half's.
    ///
    /// **What this row used to assert, and the two arguments that were wrong in
    /// turn.** It first pinned "index 0 is never withdrawn" UNCONDITIONALLY, which
    /// left the refuted number ACTIVE in `v_{prefix}_shard_scan_ranges` so the
    /// shard's block window kept the lie. It was then relaxed to withdraw index 0
    /// freely, on the argument that `ResetToSubtreeRoots` cannot fire because
    /// `update_tree` checkpoints the batch anchor — which covers checkpoint CREATION
    /// and says nothing about SURVIVAL (`prune_excess_checkpoints`,
    /// `shardtree-0.7.1 lib.rs:644`, keeps at most `PRUNING_DEPTH = 100`). Neither
    /// argument holds, so the INVARIANT is guarded instead: [`withdraw_suffix`]
    /// clamps a withdrawal away from index 0 while the pool holds any `root_hash`
    /// row, and `a_suffix_withdrawal_leaves_the_recorded_heights_a_dense_prefix`
    /// drives that clamp on the real statements. This row is about the SHAPE — a
    /// withdrawal is a suffix at whatever index it starts from — which is the
    /// property that holds either way.
    ///
    /// Mutant: make the withdrawal write ONE row instead of a suffix → the
    /// `every_recorded_height_survives` assertion goes red at `from = 1`. It does NOT
    /// redden the `survivors` assertion beside it, and that is worth knowing: the
    /// collection stops at the first gap either way, so the survivor set is identical
    /// for a one-row hole and a whole-suffix withdrawal. Asserting on it alone was
    /// vacuous against this very mutation, measured.
    #[test]
    fn a_withdrawal_is_always_a_suffix_so_the_prefix_is_never_gapped() {
        /// Upstream's collection, transcribed: heights at or below the truncation
        /// height, in index order, stopping at the first gap.
        fn survivors(recorded: &[Option<u32>], truncation_height: u32) -> Vec<usize> {
            let mut out = Vec::new();
            for (index, rec) in recorded.iter().enumerate() {
                match rec {
                    Some(h) if *h <= truncation_height && index == out.len() => out.push(index),
                    _ => break,
                }
            }
            out
        }
        /// The shape a withdrawal from `from` leaves behind.
        fn withdrawn(recorded: &[Option<u32>], from: usize) -> Vec<Option<u32>> {
            recorded
                .iter()
                .enumerate()
                .map(|(i, r)| if i >= from { None } else { *r })
                .collect()
        }
        let populated = vec![Some(100u32), Some(200), Some(300)];
        assert_eq!(
            survivors(&populated, 400),
            vec![0, 1, 2],
            "the base: every recorded height is at or below the truncation height"
        );
        // THE SHAPE THE COLLECTION CANNOT READ, shown at the query: one hole and every
        // shard above it stops being re-put, however healthy its own height is.
        let holed = vec![Some(100u32), None, Some(300)];
        assert_eq!(
            survivors(&holed, 400),
            vec![0],
            "a hole at index 1 drops shard 2 from the re-put — the gap, not the count, \
             is what breaks the collection"
        );
        // **THE INVARIANT, and it is NOT about the survivor set.** The collection
        // returns the same prefix whether the hole is one row or the whole suffix —
        // it stops at the first gap either way — so asserting on `survivors` alone
        // cannot tell the two apart, and a row that did would be vacuous against the
        // mutation it names. (Measured: the single-row mutant left that assertion
        // GREEN.) What differs is what the table still CLAIMS above the hole: a
        // single-row withdrawal leaves `Some(h)` at indices the truncation will
        // delete and never re-put, so the wallet holds a height for a shard root it
        // is about to lose. A suffix withdrawal leaves NULL there — the honest "this
        // wallet does not know".
        //
        // So the property is stated over the TABLE: every non-NULL height must fall
        // inside the prefix the collection actually re-puts.
        fn every_recorded_height_survives(recorded: &[Option<u32>], truncation: u32) -> bool {
            let kept = survivors(recorded, truncation);
            recorded
                .iter()
                .enumerate()
                .filter(|(_, r)| r.is_some())
                .all(|(i, _)| kept.contains(&i))
        }
        for from in 0..=populated.len() {
            let after = withdrawn(&populated, from);
            assert_eq!(
                survivors(&after, 400),
                (0..from).collect::<Vec<_>>(),
                "a withdrawal from {from} leaves exactly the untouched prefix. Got \
                 {after:?}"
            );
            assert!(
                every_recorded_height_survives(&after, 400),
                "and NOTHING is left claiming a height the collection will not re-put \
                 — that is the difference between withdrawing a suffix and punching a \
                 hole, and it is invisible from the survivor set alone. Got {after:?}"
            );
        }
        // THE CONTROL for that second assertion, so it is not green by construction:
        // the holed shape DOES strand a height, at index 2.
        assert!(
            !every_recorded_height_survives(&holed, 400),
            "the holed table still claims 300 at index 2, and the collection breaks \
             before it — the shard is deleted and never re-put"
        );
        // ... and index 0 in particular: the whole table goes, which is the
        // fresh-wallet state and not a gap. The residual this accepts — a cap erased
        // if `ResetToSubtreeRoots` ever becomes reachable — is on `plan_withdrawal`.
        assert_eq!(
            survivors(&withdrawn(&populated, 0), 400),
            Vec::<usize>::new(),
            "withdrawing from 0 empties the survivor set, which is what an unwritten \
             shard table looks like — the state every wallet starts in"
        );
        // And the planner picks the LOWEST refuted index, index 0 included, because
        // everything above it goes with it.
        let refuted = |index: usize, recorded: u32| BoundaryCorrection {
            index,
            recorded,
            truth: None,
            floor: Some(recorded + 10),
            ceiling: None,
        };
        assert_eq!(
            plan_withdrawal(&[refuted(2, 300), refuted(1, 200)]),
            Some(1),
            "the LOWEST refuted index decides"
        );
        assert_eq!(
            plan_withdrawal(&[refuted(2, 300), refuted(0, 100)]),
            Some(0),
            "and index 0 is no longer exempt — the reasoning that exempted it protected \
             a hazard that cannot fire, at the price of leaving the lie in the shard's \
             block window"
        );
        // A crossing that CAN be corrected is not a withdrawal at all: the row moves
        // from one height to another and no gap is possible.
        assert_eq!(
            plan_withdrawal(&[BoundaryCorrection {
                index: 1,
                recorded: 200,
                truth: Some(205),
                floor: Some(204),
                ceiling: Some(205),
            }]),
            None
        );
    }

    /// **R4 — a verdict the SIGNED BUNDLE contradicts is dropped whole**, truth,
    /// floor and CEILING together, because all three are derived from a chain of
    /// counts rooted at an anchor the endpoint supplied and the bundle is the one
    /// oracle here that needs neither the endpoint nor the scan.
    ///
    /// The ceiling clause is the one that matters most: the ceiling is the end that
    /// refuses an honest server for serving too HIGH, so a wrong one is R3's deadlock,
    /// and the bundle is the only thing that can veto it without asking the endpoint.
    ///
    /// Driven on the real mainnet Sapling rows rather than on a synthetic bundle, so
    /// the row also measures that `bundled_counts` is dense enough to catch this at
    /// all. Mutant: drop the filter in [`reconcile_scanned_boundaries`] → a computed
    /// truth the bundle refutes is written into upstream's table.
    #[test]
    fn a_computed_truth_the_bundle_refutes_is_never_written() {
        let network = Network::Main;
        let pool = ShieldedProtocol::Sapling;
        let bundled = bundled_counts(network, pool);
        let (at_height, complete) = *bundled
            .last()
            .expect("mainnet Sapling bundled rows are compiled in");
        assert!(
            complete > 1,
            "the fixture below needs a row proving at least two complete subtrees"
        );
        // A "truth" claiming subtree 0 completed ABOVE a height where the bundle
        // proves many subtrees already complete. Impossible offline.
        assert!(
            bundle_contradicts(
                network,
                pool,
                &BoundaryCorrection {
                    index: 0,
                    recorded: 1,
                    truth: Some(at_height + 1),
                    floor: None,
                    ceiling: None,
                }
            ),
            "the bundle proves {complete} subtrees complete at {at_height}, so subtree \
             0 cannot have completed above it"
        );
        // The same shape through the FLOOR: a claim that subtree 0 was still
        // incomplete at a height the bundle shows it complete at.
        assert!(bundle_contradicts(
            network,
            pool,
            &BoundaryCorrection {
                index: 0,
                recorded: 1,
                truth: None,
                floor: Some(at_height),
                ceiling: None,
            }
        ));
        // And the mirror, through the CEILING: a claim that subtree `complete` had
        // ALREADY completed at or below a height the bundle shows it had not.
        assert!(
            bundle_contradicts(
                network,
                pool,
                &BoundaryCorrection {
                    index: complete as usize,
                    recorded: at_height + 5,
                    truth: None,
                    floor: None,
                    ceiling: Some(at_height),
                }
            ),
            "the bundle proves only {complete} subtrees complete at {at_height}, so \
             subtree {complete} cannot have completed at or below it"
        );
        // THE CONTROL, and without it the row would pass by contradicting everything:
        // a floor ABOVE the newest bundled row is exactly where BIND-1 exists to
        // reach, and the bundle has nothing to say about it.
        assert!(
            !bundle_contradicts(
                network,
                pool,
                &BoundaryCorrection {
                    index: complete as usize,
                    recorded: at_height + 5,
                    truth: None,
                    floor: Some(at_height + 10),
                    ceiling: Some(at_height + 20),
                }
            ),
            "above the newest bundled row the bundle abstains — refusing here would \
             disarm the oracle this whole item exists to add"
        );
    }

    /// **The ledger: the bracket only ever TIGHTENS — the floor rises, the ceiling
    /// falls, neither moves outward — a batch that bounds one side leaves the other
    /// alone, and an accepted write withdraws only the indices it covered.**
    ///
    /// Monotonicity is what makes the store's meaning independent of the order the
    /// batches arrive in: a later batch observing the same boundary from further away
    /// must not un-refute heights an earlier one already proved impossible. The
    /// one-sided update is what stops a batch that only saw the subtree complete from
    /// erasing an earlier batch's floor (`MAX(NULL, x)` is NULL in SQLite, which is
    /// why both arms of the upsert carry `IFNULL`). The scoped clear is what stops a
    /// SHORT serve from clearing a bracket at an index it never wrote.
    #[test]
    fn a_boundary_bracket_only_tightens_and_is_withdrawn_only_by_a_write_that_covered_it() {
        let conn = Connection::open_in_memory().expect("in-memory db");
        ensure_boundary_bound_table(&conn).expect("ensure");
        ensure_boundary_bound_table(&conn).expect("idempotent");
        let pool = ShieldedProtocol::Ironwood;
        let bounds = |floor, ceiling| BoundaryBounds { floor, ceiling };
        record_boundary_bounds(&conn, pool, 0, bounds(Some(3_460_000), Some(3_480_000)))
            .expect("first bracket");
        record_boundary_bounds(&conn, pool, 3, bounds(Some(3_470_000), None))
            .expect("a second index, floor only");
        // A WEAKER observation of the same boundary — lower floor, higher ceiling —
        // leaves the stronger claim standing on both ends.
        record_boundary_bounds(&conn, pool, 0, bounds(Some(3_450_000), Some(3_490_000)))
            .expect("a weaker observation");
        assert_eq!(
            read_boundary_bounds(&conn, pool).expect("read"),
            HashMap::from([
                (0usize, bounds(Some(3_460_000), Some(3_480_000))),
                (3usize, bounds(Some(3_470_000), None)),
            ]),
            "MAX/MIN, not overwrite: seeing less evidence must never weaken the guard, \
             at either end"
        );
        // A batch that bounds only the CEILING must not erase the floor beside it.
        record_boundary_bounds(&conn, pool, 0, bounds(None, Some(3_475_000)))
            .expect("a ceiling-only observation");
        assert_eq!(
            read_boundary_bounds(&conn, pool).expect("read")[&0],
            bounds(Some(3_460_000), Some(3_475_000)),
            "the ceiling tightened and the floor survived — a one-sided batch may not \
             blank the other side"
        );
        // ... and the mirror: a floor-only observation on an index that has both.
        record_boundary_bounds(&conn, pool, 0, bounds(Some(3_465_000), None))
            .expect("a floor-only observation");
        assert_eq!(
            read_boundary_bounds(&conn, pool).expect("read")[&0],
            bounds(Some(3_465_000), Some(3_475_000)),
            "and a stronger floor does raise it without touching the ceiling"
        );
        // **THE ANTI-DEADLOCK SWEEP**, and it is the one new failure mode
        // two-sidedness introduces. A floor at or above the ceiling brackets NOTHING:
        // every height at that index would be refused, forever, with no accepted write
        // to withdraw it and no exit but a rescan. It cannot happen inside one run, so
        // reaching it means two batches contradicted each other — a reorg, or an anchor
        // one of them was lied to about — and the honest answer is that this wallet
        // does not know where the boundary is. The row is DROPPED.
        record_boundary_bounds(&conn, pool, 0, bounds(Some(3_475_000), None))
            .expect("a floor that meets the ceiling");
        assert!(
            !read_boundary_bounds(&conn, pool)
                .expect("read")
                .contains_key(&0),
            "a bracket that closes to nothing is forgotten, not persisted — otherwise \
             it is a wallet that can never accept a height at that index again"
        );
        // Another pool's rows are untouched — the key is (pool, index).
        record_boundary_bounds(
            &conn,
            ShieldedProtocol::Sapling,
            0,
            bounds(Some(900_000), None),
        )
        .expect("sapling");
        // A serve covering indices 0..3 withdraws index 0 and leaves index 3, which is
        // above its prefix.
        record_boundary_bounds(&conn, pool, 0, bounds(Some(3_460_000), None)).expect("re-arm");
        clear_boundary_bounds_in(&conn, pool, 0, 3).expect("clear");
        assert_eq!(
            read_boundary_bounds(&conn, pool).expect("read"),
            HashMap::from([(3usize, bounds(Some(3_470_000), None))]),
            "a short serve clears the brackets it actually rewrote and no others"
        );
        assert_eq!(
            read_boundary_bounds(&conn, ShieldedProtocol::Sapling).expect("read"),
            HashMap::from([(0usize, bounds(Some(900_000), None))]),
            "and it says nothing about another pool"
        );
        clear_boundary_bounds(&conn).expect("the rescan clear");
        assert!(read_boundary_bounds(&conn, pool).expect("read").is_empty());
        assert!(
            read_boundary_bounds(&conn, ShieldedProtocol::Sapling)
                .expect("read")
                .is_empty()
        );
    }

    /// **R5 — an anchor the wallet's own blocks contradict is not believed**, and the
    /// third standing is the one the join's second CRITICAL turned on.
    ///
    /// Every clause of the reconcile is a statement about an ABSOLUTE tree level, and
    /// the level comes from `from_state`'s frontier — a `GetTreeState` the ENDPOINT
    /// served on the first batch of every pass, after every rewind, and at each
    /// reconcile boundary. Where this wallet has counted that same block itself, its
    /// own number decides:
    ///
    /// * **Verified** — the two agree. Proceed, and MINT durable bounds.
    /// * **Contradicted** — they differ. Abstain for this pool on this batch: the
    ///   batch's per-block sizes were chained FROM that anchor, so nothing built on
    ///   them is evidence about anything.
    /// * **Unverifiable** — no `blocks` row at that height, which is the FIRST BATCH
    ///   of every scan range and therefore most batches of a restore. Correct and
    ///   un-latch (both conservative, both reversible by the next honest serve) but
    ///   persist NOTHING: a bound outlives its evidence and refuses future serves on
    ///   its own authority, and one derived from an unchecked endpoint number is a
    ///   refusal the server chose.
    ///
    /// Mutant: make the unverifiable arm return `Verified` → the third assertion goes
    /// red. Make the contradicted arm return `Verified` → the second goes red.
    #[test]
    fn an_anchor_the_wallets_own_blocks_contradict_is_not_believed() {
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE blocks (
                 height INTEGER PRIMARY KEY,
                 ironwood_commitment_tree_size INTEGER
             );
             INSERT INTO blocks VALUES (3459999, 70000), (3459998, NULL);",
        )
        .expect("fixture");
        let pool = ShieldedProtocol::Ironwood;
        assert_eq!(
            anchor_standing(&conn, pool, 3_459_999, 70_000).expect("standing"),
            AnchorStanding::Verified,
            "the wallet counted that block itself and got the same number"
        );
        assert_eq!(
            anchor_standing(&conn, pool, 3_459_999, 70_001).expect("standing"),
            AnchorStanding::Contradicted,
            "one leaf apart is still a different chain of counts — the reconcile \
             abstains rather than refute anything built on it"
        );
        for (height, why) in [
            (3_459_997u32, "a height this wallet never scanned"),
            (
                3_459_998,
                "and a row whose size column is NULL — a pool below its \
                         activation, or a row an older build wrote",
            ),
        ] {
            assert_eq!(
                anchor_standing(&conn, pool, height, 70_000).expect("standing"),
                AnchorStanding::Unverifiable,
                "{why}: the wallet cannot check the anchor, so it may correct and \
                 un-latch but must not MINT a durable bound"
            );
        }
    }

    /// **An UNVERIFIABLE anchor corrects the row and persists NO bound** — the second
    /// half of the join's anchor CRITICAL, and the half `anchor_standing`'s own
    /// row cannot reach because it tests the standing, not what the reconcile does
    /// with it. (Measured: mutating the mint gate to `!= Contradicted` left every
    /// other row in this module green.)
    ///
    /// The asymmetry is the point. Correcting a row and undoing a latch are both
    /// CONSERVATIVE and both reversible by the next honest serve. Persisting a bound
    /// is neither: it outlives its evidence and refuses future serves on its own
    /// authority, so one derived from an endpoint number this wallet could not check
    /// is a refusal the server chose for us — R3's deadlock with the sign flipped.
    ///
    /// Mutant: gate the mint on `!= Contradicted` instead of `== Verified` → the last
    /// assertion goes red while the correction assertions stay green.
    #[test]
    fn an_unverifiable_anchor_corrects_the_row_but_persists_no_bound() {
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE blocks (
                 height INTEGER PRIMARY KEY,
                 ironwood_commitment_tree_size INTEGER
             );
             CREATE TABLE ironwood_tree_shards (
                 shard_index INTEGER PRIMARY KEY,
                 subtree_end_height INTEGER,
                 root_hash BLOB
             );
             CREATE TABLE ironwood_received_notes (
                 id INTEGER PRIMARY KEY,
                 commitment_tree_position INTEGER,
                 witness_stabilized INTEGER NOT NULL
             );
             INSERT INTO ironwood_tree_shards VALUES
                 (0, 3400000, X'01'), (1, 3460005, X'02');
             INSERT INTO ironwood_received_notes VALUES (1, 65536, 1);",
        )
        .expect("fixture");
        ensure_boundary_bound_table(&conn).expect("bounds");
        ensure_rewind_table(&conn).expect("rewind");
        for h in 3_460_000u32..=3_460_010 {
            conn.execute(
                "INSERT INTO blocks VALUES (?1, ?2)",
                rusqlite::params![h, (2 * SUBTREE_LEAVES + 5) as i64],
            )
            .expect("block row");
        }
        // The anchor sits at a height this wallet has NEVER scanned — the first batch
        // of a scan range, which on a restore is most batches. Nothing can check it.
        let pool = ShieldedProtocol::Ironwood;
        assert_eq!(
            anchor_standing(&conn, pool, 3_459_990, 2 * SUBTREE_LEAVES - 1).expect("standing"),
            AnchorStanding::Unverifiable
        );
        let got = reconcile_scanned_boundaries(
            &conn,
            Network::Main,
            pool,
            3_459_990,
            2 * SUBTREE_LEAVES - 1,
            (3_460_000, 3_460_010),
        )
        .expect("an unverifiable anchor is not a fault");
        assert!(
            matches!(
                got,
                Some(HeightBindRefusal::ScannedBoundary {
                    index: 1,
                    recorded: 3_460_005,
                    truth: None
                })
            ),
            "the REPAIR still runs: the record is refuted and reported. Got {got:?}"
        );
        assert_eq!(
            read_recorded_heights(&conn, pool).expect("read"),
            vec![Some(3_400_000), None],
            "and the row is withdrawn — conservative, and undone by the next honest \
             serve"
        );
        assert_eq!(
            conn.query_row(
                "SELECT witness_stabilized FROM ironwood_received_notes WHERE id = 1",
                [],
                |row| row.get::<_, i64>(0)
            )
            .expect("latch"),
            0,
            "and the latch it licensed is undone — also conservative"
        );
        assert!(
            read_boundary_bounds(&conn, pool).expect("read").is_empty(),
            "but NOTHING durable is persisted from an anchor this wallet could not \
             check. A bound outlives its evidence and refuses on its own authority; a \
             correction does not"
        );
    }

    /// **R7 — a fault mid-correction leaves NOTHING half-applied**, driven through the
    /// real statements on a real connection.
    ///
    /// The fault is injected where a real one would land: the `{prefix}_received_notes`
    /// table is dropped, so the un-latch fails AFTER the brackets have been written
    /// inside the same transaction. Before BIND-1-R this was N × 3 autocommit
    /// statements and a fault after correction *i* left *i*'s row rewritten, *j*'s
    /// wrong row and *j*'s latch intact, and no report — a transient BUSY silently
    /// converting a DETECTED endpoint lie into an undetected one.
    ///
    /// The CONTROL is the same fixture with the table present: the call succeeds, the
    /// refusal is raised by VARIANT, and the brackets ARE there. Without it the row
    /// would pass on a fixture that never wrote anything.
    ///
    /// Mutant: replace the transaction with direct statements on `conn` → the fault
    /// arm's "nothing was written" assertion goes red.
    #[test]
    fn a_fault_mid_correction_leaves_nothing_half_applied() {
        // A mainnet Ironwood geometry ABOVE the newest bundled row (3,459,780), so
        // the signed bundle vetoes nothing: subtree 1 completes at 3,460,000 inside
        // the batch, and the record for it is one block off.
        let fixture = |with_notes: bool| {
            let conn = Connection::open_in_memory().expect("in-memory db");
            conn.execute_batch(
                "CREATE TABLE blocks (
                     height INTEGER PRIMARY KEY,
                     ironwood_commitment_tree_size INTEGER
                 );
                 CREATE TABLE ironwood_tree_shards (
                     shard_index INTEGER PRIMARY KEY,
                     subtree_end_height INTEGER,
                     root_hash BLOB
                 );",
            )
            .expect("schema");
            if with_notes {
                conn.execute_batch(
                    "CREATE TABLE ironwood_received_notes (
                         id INTEGER PRIMARY KEY,
                         commitment_tree_position INTEGER,
                         witness_stabilized INTEGER NOT NULL
                     );",
                )
                .expect("notes");
            }
            ensure_boundary_bound_table(&conn).expect("bounds");
            ensure_rewind_table(&conn).expect("rewind");
            let anchor_size = 2 * SUBTREE_LEAVES - 1;
            conn.execute(
                "INSERT INTO blocks VALUES (?1, ?2)",
                rusqlite::params![3_459_999u32, anchor_size as i64],
            )
            .expect("anchor row");
            for h in 3_460_000u32..=3_460_010 {
                conn.execute(
                    "INSERT INTO blocks VALUES (?1, ?2)",
                    rusqlite::params![h, (2 * SUBTREE_LEAVES + 5) as i64],
                )
                .expect("block row");
            }
            conn.execute_batch(
                "INSERT INTO ironwood_tree_shards VALUES
                     (0, 3400000, X'01'), (1, 3460005, X'02');",
            )
            .expect("shards");
            conn
        };
        let anchor_size = 2 * SUBTREE_LEAVES - 1;
        let pool = ShieldedProtocol::Ironwood;

        // THE CONTROL: everything present, the correction lands, the brackets persist.
        let ok = fixture(true);
        let got = reconcile_scanned_boundaries(
            &ok,
            Network::Main,
            pool,
            3_459_999,
            anchor_size,
            (3_460_000, 3_460_010),
        )
        .expect("the control must not fault");
        assert!(
            matches!(
                got,
                Some(HeightBindRefusal::ScannedBoundary {
                    index: 1,
                    recorded: 3_460_005,
                    truth: Some(3_460_000)
                })
            ),
            "the control: subtree 1 crossed at 3,460,000 by this wallet's own count, \
             so the one-block-off record is refuted AND corrected. Got {got:?}"
        );
        assert_eq!(
            read_recorded_heights(&ok, pool).expect("read"),
            vec![Some(3_400_000), Some(3_460_000)],
            "and the row now holds the counted truth"
        );
        assert!(
            !read_boundary_bounds(&ok, pool).expect("read").is_empty(),
            "and the brackets it minted are on disk — without this the fault arm \
             below proves nothing"
        );

        // THE FAULT: the un-latch has no table to write to, and it runs INSIDE the
        // transaction, after the brackets.
        let broken = fixture(false);
        let err = reconcile_scanned_boundaries(
            &broken,
            Network::Main,
            pool,
            3_459_999,
            anchor_size,
            (3_460_000, 3_460_010),
        )
        .expect_err("a missing note table must fault");
        assert!(
            matches!(err, WalletError::StoreCorrupt),
            "a missing upstream table is our side's fault, not the endpoint's. Got \
             {err:?}"
        );
        assert!(
            read_boundary_bounds(&broken, pool)
                .expect("read")
                .is_empty(),
            "R7: the brackets written before the fault are ROLLED BACK — one \
             transaction per pool's correction set, not N x 3 autocommits"
        );
        assert_eq!(
            read_recorded_heights(&broken, pool).expect("read"),
            vec![Some(3_400_000), Some(3_460_005)],
            "and the row is untouched: nothing half-applied"
        );
    }

    /// **The withdrawal writes a SUFFIX and leaves the heights a dense prefix**, on
    /// the real column, through the real statement — and it refuses to touch index 0
    /// a second time, independently of [`plan_withdrawal`]. The one property the
    /// CRITICAL turns on does not rest on a single call site being right.
    #[test]
    fn a_suffix_withdrawal_leaves_the_recorded_heights_a_dense_prefix() {
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE ironwood_tree_shards (
                 shard_index INTEGER PRIMARY KEY,
                 subtree_end_height INTEGER,
                 root_hash BLOB
             );
             CREATE TABLE ironwood_received_notes (
                 id INTEGER PRIMARY KEY,
                 commitment_tree_position INTEGER,
                 witness_stabilized INTEGER NOT NULL
             );
             INSERT INTO ironwood_tree_shards VALUES
                 (0, 100, X'01'), (1, 200, X'02'), (2, 300, X'03');
             INSERT INTO ironwood_received_notes VALUES
                 (1, 0, 1),
                 (2, 65536, 1),
                 (3, 131072, 1);",
        )
        .expect("fixture");
        let pool = ShieldedProtocol::Ironwood;
        let recorded = read_recorded_heights(&conn, pool).expect("read");
        assert_eq!(recorded, vec![Some(100), Some(200), Some(300)]);
        withdraw_suffix(&conn, pool, 1).expect("withdraw the suffix");
        assert_eq!(
            read_recorded_heights(&conn, pool).expect("read"),
            vec![Some(100), None, None],
            "index 1 and everything above it — never a hole with a populated row on \
             top of it"
        );
        let latched: Vec<(i64, i64)> = conn
            .prepare("SELECT id, witness_stabilized FROM ironwood_received_notes ORDER BY id")
            .expect("prepare")
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("query")
            .map(|r| r.expect("row"))
            .collect();
        assert_eq!(
            latched,
            vec![(1, 1), (2, 0), (3, 0)],
            "every note at or above the withdrawn shard is un-latched (their block \
             windows all moved), and the shard below is untouched"
        );
        // **THE CAP INVARIANT.** This pool still holds root hashes, so emptying the
        // whole prefix would leave `truncate_tree_to_subtree_roots` nothing to re-put
        // and the `DELETE FROM {prefix}_tree_cap` before it would stand. A withdrawal
        // from 0 is CLAMPED to 1 — still a suffix, still ungapped, and index 0 keeps
        // its (refuted) value, its bracket and its un-latch.
        withdraw_suffix(&conn, pool, 0).expect("withdraw from index 0");
        assert_eq!(
            read_recorded_heights(&conn, pool).expect("read"),
            vec![Some(100), None, None],
            "clamped to index 1: the prefix is never emptied while a root_hash row \
             exists, whatever upstream does with checkpoints"
        );
        // ... and with NO root to lose, index 0 IS withdrawn: a pool whose shard rows
        // carry no root hash (a fresh wallet, an Ironwood tree before
        // `put_shard_roots` first runs) has nothing a truncation could fail to re-put.
        conn.execute("UPDATE ironwood_tree_shards SET root_hash = NULL", [])
            .expect("drop the roots");
        withdraw_suffix(&conn, pool, 0).expect("withdraw from index 0");
        assert_eq!(
            read_recorded_heights(&conn, pool).expect("read"),
            vec![None, None, None],
            "an empty prefix is the fresh-wallet state, not a gap — and with no roots \
             held there is no cap to lose"
        );
        let all_clear: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ironwood_received_notes WHERE witness_stabilized = 1",
                [],
                |row| row.get(0),
            )
            .expect("count");
        assert_eq!(
            all_clear, 0,
            "and every latch it licensed is undone, shard 0 included"
        );
    }

    /// **The bundle veto is CALLED, not merely correct — found by its own mutant
    /// surviving.** `a_computed_truth_the_bundle_refutes_is_never_written`
    /// exercises [`bundle_contradicts`] as a pure predicate and never drives a call
    /// site, so **both** vetoes could be deleted from
    /// [`reconcile_scanned_boundaries`] — the one over corrections and the one over
    /// minted brackets — and the whole module stayed 37/0. By the registry's own rule
    /// (*a test with no mutant row is not a guard*) that row was not a guard for R4,
    /// which §4x-R-run calls the clause that "matters most" because the ceiling is the
    /// end that refuses an HONEST server.
    ///
    /// The geometry: this wallet's own scanned blocks say subtree 0 crossed 65,536
    /// leaves high in the chain, while the signed bundle proves it complete far below.
    /// The wallet's count is therefore wrong — a re-scan of a region whose earlier
    /// blocks it never counted — and a correction derived from it would rewrite a
    /// TRUE recorded height to a false one, on the money path, durably.
    ///
    /// Mutants (either kills it): drop the `!bundle_contradicts(..)` filter on the
    /// corrections; or drop the `!bundle_contradicts_bounds(..)` filter on the minted
    /// brackets.
    #[test]
    fn a_truth_the_bundle_refutes_is_dropped_at_the_call_site_not_merely_detectable() {
        let pool = ShieldedProtocol::Sapling;
        let bundled = bundled_counts(Network::Main, pool);
        let (bundle_h, complete) = *bundled
            .iter()
            .find(|&&(_, c)| c >= 1)
            .expect("the mainnet Sapling bundle proves at least one complete subtree");
        assert!(
            complete >= 1 && bundle_h < 3_400_000,
            "the fixture needs the bundle proving subtree 0 complete WELL BELOW the \
             scan range below ({complete} at {bundle_h})"
        );

        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE blocks (
                 height INTEGER PRIMARY KEY,
                 sapling_commitment_tree_size INTEGER
             );
             CREATE TABLE sapling_tree_shards (
                 shard_index INTEGER PRIMARY KEY,
                 subtree_end_height INTEGER,
                 root_hash BLOB
             );
             CREATE TABLE sapling_received_notes (
                 id INTEGER PRIMARY KEY,
                 commitment_tree_position INTEGER,
                 witness_stabilized INTEGER NOT NULL
             );
             INSERT INTO sapling_tree_shards VALUES (0, 500000, X'01');",
        )
        .expect("fixture");
        ensure_boundary_bound_table(&conn).expect("bounds");
        ensure_rewind_table(&conn).expect("rewind");

        // The wallet's own blocks put subtree 0's crossing at 3,400,010 — a claim the
        // bundle refutes, because it proves subtree 0 complete at `bundle_h`.
        let (anchor_h, anchor_size) = (3_399_999u32, SUBTREE_LEAVES - 1);
        {
            let mut stmt = conn
                .prepare("INSERT INTO blocks VALUES (?1, ?2)")
                .expect("prepare");
            for h in anchor_h..=3_400_020 {
                let size = if h >= 3_400_010 {
                    SUBTREE_LEAVES
                } else {
                    SUBTREE_LEAVES - 1
                };
                stmt.execute(rusqlite::params![h, size as i64])
                    .expect("block row");
            }
        }
        assert_eq!(
            anchor_standing(&conn, pool, anchor_h, anchor_size).expect("standing"),
            AnchorStanding::Verified,
            "the mint gate must be OPEN, or the bracket half of this row is vacuous"
        );

        let got = reconcile_scanned_boundaries(
            &conn,
            Network::Main,
            pool,
            anchor_h,
            anchor_size,
            (3_400_000, 3_400_020),
        )
        .expect("the reconcile is not a fault");

        assert!(
            got.is_none(),
            "a refusal was reported from a correction the signed bundle refutes — the \
             endpoint is being blamed for this wallet's own miscount. Got {got:?}"
        );
        assert_eq!(
            read_recorded_heights(&conn, pool).expect("read"),
            vec![Some(500_000)],
            "THE RECORDED HEIGHT MOVED. The wallet's own count said subtree 0 crossed \
             at 3,400,010; the bundle — a signed constant of this binary — proves it \
             complete at {bundle_h}. The veto exists so a computed truth that loses to \
             the bundle is never written, and a durable false height on a required \
             pool refuses every honest server afterwards"
        );
        let brackets: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM subtree_boundary_bound WHERE pool = ?1",
                [table_prefix(pool)],
                |r| r.get(0),
            )
            .expect("count");
        assert_eq!(
            brackets, 0,
            "a bracket was minted from bounds the bundle refutes — the SAME veto, on \
             the other call site. A ceiling too low refuses an honest server forever"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // §4x-R-run owed row 1 — Q3, the per-batch cost of the bracket mint
    // ─────────────────────────────────────────────────────────────────────────

    /// **Q3, measured on a restore-shaped fixture instead of estimated.** The
    /// code reviewer MAJOR priced the bracket mint at *"~1,000-1,900 rows per batch
    /// per required pool on a deep restore, each with two UNCACHED statements plus a
    /// full ~800-row bundled scan, where the base wrote nothing when there was nothing
    /// to correct"*, and named three fixes cheapest-first: `prepare_cached` both
    /// statements, cap the minted index set at `reach + 1` rather than unioning every
    /// recorded index, then take this measurement. The first is done. **This row is
    /// the measurement, and it says the second must NOT be taken** — see below.
    ///
    /// # MEASURED, and the review's estimate was RIGHT
    ///
    /// `minted_rows_this_batch = 1118`, deterministic, on a bind-consistent fixture at
    /// bundled height 1,310,000 (10 subtrees complete there, 1,128 recorded). Wall time
    /// 17-30 ms per batch per pool in a DEBUG build on a loaded machine — printed, not
    /// asserted, and the spread is why. **`prepare_cached` is taken as correct by
    /// construction, not as a measured win**: it removes ~2,236 statement compiles per
    /// batch, but the cached and un-cached timings overlap inside that spread, so
    /// claiming a percentage from them would be reading noise.
    ///
    /// **What it costs the restore, which is the number that matters.** The reconcile
    /// runs on EVERY scanned batch (`sync::scan_batch`; `TREE_STATE_RECONCILE_BATCHES`
    /// gates the `GetTreeState` RPC, not this), so a deep restore of ~2.2 M blocks at
    /// `SYNC_BATCH_BLOCKS` = 100 is ~22,000 batches, and with two REQUIRED pools
    /// carrying a full recorded prefix that is roughly a quarter of an hour of the
    /// four-hour restore SCAN-1 exists to protect. Not a defect — every floor written
    /// is true — but a cost worth a decision rather than an assumption.
    ///
    /// **The fix that keeps the property, filed and not built (maintainer's call).** Every
    /// index above `reach` receives the IDENTICAL bracket, so the ~1,100 rows carry one
    /// fact between them; collapsing them into a single per-pool frontier floor is a
    /// ~100x cut with no refutation lost. That is a schema change, and it is not this
    /// batch's.
    ///
    /// # The geometry
    ///
    /// Sapling on mainnet, the review's own worst case (a required pool whose whole
    /// bundle is served at `max_entries = 0`, so the cheap exit never fires). The
    /// wallet holds a dense recorded prefix of `RECORDED` shards — an accepted serve
    /// of the whole bundle — and is scanning a 100-block batch far BELOW those
    /// recorded heights, which is where a deep restore spends its four hours. The
    /// anchor is a block this wallet scanned, so the mint gate is open (R5 `Verified`);
    /// nothing in the run refutes a recorded height, so every write this batch makes
    /// is a MINT and the count is the churn the review priced.
    ///
    /// # What it measures, and what it asserts
    ///
    /// The deterministic quantity — how many `subtree_boundary_bound` rows one batch
    /// writes — is asserted. Wall time is PRINTED and not asserted: a timing assertion
    /// on a shared machine is the flaky-gate shape `testing-patterns` forbids, and the
    /// number belongs in the run record rather than in a gate. Run it with
    /// `--nocapture` to read both.
    ///
    /// # Why `reach + 1` is refused as the fix
    ///
    /// Every index above `reach` gets the SAME bracket — `floor = Some(L)` where `L`
    /// is the run's last scanned height, `ceiling = None` — because
    /// [`complete_by`] cannot reach them and [`incomplete_through`] answers with the
    /// run's end for all of them alike. That is real refutation power (it refuses any
    /// recorded height at or below `L` at that index) and capping at `reach + 1` would
    /// drop it for every recorded row above the scan frontier, which is precisely the
    /// population R2 exists for — a record ABOVE the newest bundled row is the one an
    /// endpoint can still choose. The cost is not paid there anyway: the bundle veto
    /// (`bundle_contradicts_bounds`) already filters out every index the signed bundle
    /// proves complete at or below the scan height, which on a real restore is most of
    /// them. The honest optimisation, if this number ever hurts, is to collapse the
    /// identical above-`reach` brackets into ONE per-pool frontier floor rather than
    /// `N` equal rows — a schema change, not a cap, and it keeps every bit of the
    /// refutation. Filed rather than built: this measurement says it is not needed yet.
    #[test]
    fn the_bracket_mint_per_batch_row_count_on_a_restore_shaped_fixture() {
        const RECORDED: usize = 1_128; // mainnet Sapling's whole bundled prefix
        let pool = ShieldedProtocol::Sapling;
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE blocks (
                 height INTEGER PRIMARY KEY,
                 sapling_commitment_tree_size INTEGER
             );
             CREATE TABLE sapling_tree_shards (
                 shard_index INTEGER PRIMARY KEY,
                 subtree_end_height INTEGER,
                 root_hash BLOB
             );
             CREATE TABLE sapling_received_notes (
                 id INTEGER PRIMARY KEY,
                 commitment_tree_position INTEGER,
                 witness_stabilized INTEGER NOT NULL
             );",
        )
        .expect("fixture schema");
        ensure_boundary_bound_table(&conn).expect("bounds table");
        ensure_rewind_table(&conn).expect("rewind table");

        // **The geometry is derived from the signed bundle, not invented**, so the
        // fixture cannot flatter the code by being internally inconsistent (the
        // lesson: a fixture that only reaches the geometry where the operation always
        // succeeds measures nothing). The batch sits at a LOW bundled height — a
        // wallet whose birthday is years back, which is what "deep restore" means —
        // and the wallet's own scanned tree size there is the bundle's own count at
        // that height. Every recorded height is that subtree's TRUE completion height
        // as the bundle records it, so the run refutes none of them and every write
        // this batch makes is a MINT.
        let counts = bundled_counts(Network::Main, pool);
        assert!(
            counts.len() > 10,
            "the mainnet Sapling bundle must carry enough rows to pick a low one"
        );
        // A row about a tenth of the way in: low enough that the scan frontier is far
        // below the recorded prefix, which is the geometry that mints.
        let (batch_h, batch_complete) = counts[counts.len() / 10];
        // The true completion height of each subtree, read off the bundle: the lowest
        // bundled height whose count exceeds the index.
        let true_completion = |i: u64| -> Option<u32> {
            counts
                .iter()
                .find(|&&(_, complete)| complete > i)
                .map(|&(at, _)| at)
        };
        {
            let mut stmt = conn
                .prepare("INSERT INTO sapling_tree_shards VALUES (?1, ?2, X'01')")
                .expect("prepare");
            for i in 0..RECORDED {
                // A subtree the bundle never shows complete gets a height above the
                // newest bundled row — an endpoint-supplied record the bundle cannot
                // speak to, which is the population the bracket exists for.
                let h = true_completion(i as u64)
                    .unwrap_or_else(|| newest_bundled_height(Network::Main) + 1 + i as u32);
                stmt.execute(rusqlite::params![i as i64, i64::from(h)])
                    .expect("recorded row");
            }
        }

        // The batch: 100 scanned blocks at the bundled height, the anchor one block
        // below and inside `blocks` so `anchor_standing` is `Verified`. The tree size
        // is the bundle's own count there plus a partial subtree.
        let scanned_size = batch_complete * SUBTREE_LEAVES + 5;
        let (anchor_h, anchor_size) = (batch_h - 1, scanned_size);
        {
            let mut stmt = conn
                .prepare("INSERT INTO blocks VALUES (?1, ?2)")
                .expect("prepare");
            for h in anchor_h..=(batch_h + 100) {
                stmt.execute(rusqlite::params![h, scanned_size as i64])
                    .expect("block row");
            }
        }
        assert_eq!(
            anchor_standing(&conn, pool, anchor_h, anchor_size).expect("standing"),
            AnchorStanding::Verified,
            "the mint gate must be OPEN, or this measures nothing"
        );

        let started = std::time::Instant::now();
        let refusal = reconcile_scanned_boundaries(
            &conn,
            Network::Main,
            pool,
            anchor_h,
            anchor_size,
            (batch_h, batch_h + 100),
        )
        .expect("the reconcile is not a fault on an honest run");
        let elapsed = started.elapsed();

        assert!(
            refusal.is_none(),
            "no recorded height is refuted by this run, so the batch's whole cost is \
             the MINT — got {refusal:?}"
        );
        let minted: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM subtree_boundary_bound WHERE pool = ?1",
                [table_prefix(pool)],
                |r| r.get(0),
            )
            .expect("count");
        println!(
            "Q3 MEASURED — pool={pool:?} recorded={RECORDED} bundled_rows={} \
             batch_height={batch_h} complete_at_batch={batch_complete} \
             minted_rows_this_batch={minted} elapsed={elapsed:?}",
            counts.len()
        );

        // The deterministic half. The bundle veto is what bounds this, not the
        // recorded count: every index the signed bundle proves complete at or below
        // the run's last scanned height has its floor refuted and is dropped before
        // any write. A mutant that removes the veto filter drives this number to the
        // full recorded prefix and reds the upper bound below.
        assert!(
            minted > 0,
            "a run that mints NOTHING measures nothing — the fixture has stopped \
             reaching the mint (check `anchor_standing` and the recorded prefix)"
        );
        assert!(
            minted <= RECORDED as i64 + 1,
            "the minted set is `0..=reach` unioned with the recorded indices, so it \
             can never exceed the recorded prefix by more than one; {minted} rows \
             means the set is being built from something else"
        );
        // Re-running the SAME batch is idempotent in COUNT — the ON CONFLICT path
        // rewrites the same rows rather than adding any. This is what makes the churn
        // a per-batch WRITE cost and not unbounded growth, which is the half of the
        // review's finding that decides whether it is a leak or a cost.
        reconcile_scanned_boundaries(
            &conn,
            Network::Main,
            pool,
            anchor_h,
            anchor_size,
            (batch_h, batch_h + 100),
        )
        .expect("second pass");
        let after: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM subtree_boundary_bound WHERE pool = ?1",
                [table_prefix(pool)],
                |r| r.get(0),
            )
            .expect("count");
        assert_eq!(
            after, minted,
            "the second identical batch must rewrite the same rows, not append: the \
             table is bounded by the shard count, not by the number of batches"
        );
    }

    /// The four-note latch fixture every un-latch row below reads: one latched
    /// note in each of shards 0, 1, 2 and 3 (positions `k * SUBTREE_LEAVES`), so a
    /// row can say which shards an un-latch REACHED and — the half a single note
    /// cannot say — which it left alone. Row `id` is the shard index plus one.
    fn latch_fixture(conn: &Connection) {
        conn.execute_batch(
            "CREATE TABLE ironwood_received_notes (
                 id INTEGER PRIMARY KEY,
                 commitment_tree_position INTEGER,
                 witness_stabilized INTEGER NOT NULL
             );",
        )
        .expect("notes");
        for shard in 0u64..4 {
            conn.execute(
                "INSERT INTO ironwood_received_notes VALUES (?1, ?2, 1)",
                rusqlite::params![shard + 1, shard * SUBTREE_LEAVES],
            )
            .expect("latched note");
        }
    }

    /// `witness_stabilized` per shard, in shard order, from the fixture above.
    fn latches(conn: &Connection) -> Vec<i64> {
        let mut stmt = conn
            .prepare("SELECT witness_stabilized FROM ironwood_received_notes ORDER BY id")
            .expect("prepare");
        stmt.query_map([], |row| row.get::<_, i64>(0))
            .expect("query")
            .collect::<Result<Vec<_>, _>>()
            .expect("rows")
    }

    /// **A correction at index *i* un-latches shard *i* AND shard *i + 1*, and no
    /// other shard** — the `shard + 1` un-latch at the correction site, which §4ac's
    /// review promoted above owed because deleting it killed nothing.
    ///
    /// Why the second shard is load-bearing (`apply_boundary_correction`'s doc): a
    /// recorded height is the END of shard *i*'s block window and the START of
    /// shard *i + 1*'s, so one wrong number shrinks TWO windows, and a latch earned
    /// under the raised half sits on a note whose commitments the unscanned-range
    /// check was hidden from. `witness_stabilized = 1` makes that note
    /// UNCONDITIONALLY spendable (`common.rs:779-785` short-circuits on it), the
    /// latch is one-way, and this un-latch is its only remover SCOPED TO THE SHARD
    /// (the other, `withdraw_suffix`, clears a suffix and only on a withdrawal) — so
    /// a latch that survives its own correction is a detected endpoint lie that
    /// outlives its detection.
    ///
    /// The correction is a REPLACEMENT (`truth: Some(_)`), on purpose: a withdrawal
    /// runs `withdraw_suffix`, whose `>= shard` un-latch would cover shard *i + 1*
    /// by itself and mask a deleted call here. Shards 0 and 3 are the scoping
    /// control — a suffix-shaped un-latch would clear shard 3 and a whole-table one
    /// would clear shard 0.
    ///
    /// Mutant: delete `unlatch_shard(conn, pool, shard.saturating_add(1))?;` in
    /// `apply_boundary_correction` → shard 2 stays latched. Sibling mutant: delete
    /// the `unlatch_shard(conn, pool, shard)?;` beside it → shard 1 stays latched.
    #[test]
    fn a_correction_unlatches_its_own_shard_and_the_next_and_no_other() {
        // The control geometry of `a_fault_mid_correction_leaves_nothing_half_applied`:
        // subtree 1 crosses at 3,460,000 by this wallet's own count and the record is
        // one block off, above the newest bundled row so the bundle vetoes nothing.
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE blocks (
                 height INTEGER PRIMARY KEY,
                 ironwood_commitment_tree_size INTEGER
             );
             CREATE TABLE ironwood_tree_shards (
                 shard_index INTEGER PRIMARY KEY,
                 subtree_end_height INTEGER,
                 root_hash BLOB
             );
             INSERT INTO ironwood_tree_shards VALUES
                 (0, 3400000, X'01'), (1, 3460005, X'02');",
        )
        .expect("schema");
        latch_fixture(&conn);
        ensure_boundary_bound_table(&conn).expect("bounds");
        ensure_rewind_table(&conn).expect("rewind");
        let anchor_size = 2 * SUBTREE_LEAVES - 1;
        conn.execute(
            "INSERT INTO blocks VALUES (?1, ?2)",
            rusqlite::params![3_459_999u32, anchor_size as i64],
        )
        .expect("anchor row");
        for h in 3_460_000u32..=3_460_010 {
            conn.execute(
                "INSERT INTO blocks VALUES (?1, ?2)",
                rusqlite::params![h, (2 * SUBTREE_LEAVES + 5) as i64],
            )
            .expect("block row");
        }
        let pool = ShieldedProtocol::Ironwood;
        assert_eq!(
            latches(&conn),
            vec![1, 1, 1, 1],
            "every note starts latched"
        );

        let got = reconcile_scanned_boundaries(
            &conn,
            Network::Main,
            pool,
            3_459_999,
            anchor_size,
            (3_460_000, 3_460_010),
        )
        .expect("a correction is not a fault");
        assert!(
            matches!(
                got,
                Some(HeightBindRefusal::ScannedBoundary {
                    index: 1,
                    recorded: 3_460_005,
                    truth: Some(3_460_000)
                })
            ),
            "the row must be REPLACED, not withdrawn — a withdrawal's suffix un-latch \
             would mask the site under test. Got {got:?}"
        );
        assert_eq!(
            latches(&conn),
            vec![1, 0, 0, 1],
            "a correction at index 1 un-latches shard 1 (its own window's END) AND \
             shard 2 (whose window START it is) — and touches neither shard 0 nor \
             shard 3. [shard 0, shard 1, shard 2, shard 3]"
        );
    }

    /// **A standing bracket that refutes a recorded row un-latches shard *i* AND
    /// shard *i + 1* from the ledger alone** — the second pair of `unlatch_shard`
    /// sites, in `reconcile_scanned_boundaries` itself, and the pair no batch
    /// re-derives a correction for.
    ///
    /// The state: the rewind window let a refuted height be WRITTEN (the gate
    /// abstained for one pass), and by the next pass the evidence is behind the
    /// frontier — so the ledger says the row is wrong and no observation can say
    /// what it should be. `mark_stabilized_notes` re-latches from that wrong row on
    /// every batch, so the un-latch has to be re-applied from the bracket alone, and
    /// on BOTH shards the wrong number shapes, for the same reason as the correction
    /// site. The batch here scans BELOW the crossing (every block still one leaf
    /// short of completing subtree 1), so it derives no correction and reports
    /// nothing — "a batch whose only work was re-applying an older floor's un-latch
    /// badges nobody".
    ///
    /// Mutant: delete `unlatch_shard(&tx, pool, shard.saturating_add(1))?;` in the
    /// `stale` loop → shard 2 stays latched. Sibling mutant: delete the
    /// `unlatch_shard(&tx, pool, shard)?;` beside it → shard 1 stays latched.
    #[test]
    fn a_standing_bracket_unlatches_both_shards_with_no_correction_to_derive() {
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE blocks (
                 height INTEGER PRIMARY KEY,
                 ironwood_commitment_tree_size INTEGER
             );
             CREATE TABLE ironwood_tree_shards (
                 shard_index INTEGER PRIMARY KEY,
                 subtree_end_height INTEGER,
                 root_hash BLOB
             );
             INSERT INTO ironwood_tree_shards VALUES
                 (0, 3400000, X'01'), (1, 3460005, X'02');",
        )
        .expect("schema");
        latch_fixture(&conn);
        ensure_boundary_bound_table(&conn).expect("bounds");
        ensure_rewind_table(&conn).expect("rewind");
        let pool = ShieldedProtocol::Ironwood;
        // THE LEDGER: a bracket at index 1 whose floor sits ABOVE the recorded
        // 3,460,005 — "the subtree was still incomplete at 3,460,006", which refutes
        // a record that says it completed a block earlier. Written the way the
        // reconcile writes one, through the same statement.
        record_boundary_bounds(
            &conn,
            pool,
            1,
            BoundaryBounds {
                floor: Some(3_460_006),
                ceiling: None,
            },
        )
        .expect("the standing bracket");
        assert!(
            read_boundary_bounds(&conn, pool)
                .expect("read")
                .get(&1)
                .is_some_and(|b| b.refute(3_460_005)),
            "precondition: the planted bracket must refute the recorded row, or the \
             stale path is never entered and this row measures nothing"
        );
        // THE BATCH: entirely below subtree 1's crossing — the anchor and every
        // block one leaf short of completing it — so no clause of
        // `reconcile_boundaries` fires for index 1 and index 0 is consistent.
        let anchor_size = 2 * SUBTREE_LEAVES - 10;
        conn.execute(
            "INSERT INTO blocks VALUES (?1, ?2)",
            rusqlite::params![3_459_989u32, anchor_size as i64],
        )
        .expect("anchor row");
        for h in 3_459_990u32..=3_459_998 {
            conn.execute(
                "INSERT INTO blocks VALUES (?1, ?2)",
                rusqlite::params![h, (2 * SUBTREE_LEAVES - 1) as i64],
            )
            .expect("block row");
        }
        assert_eq!(
            latches(&conn),
            vec![1, 1, 1, 1],
            "every note starts latched"
        );

        let got = reconcile_scanned_boundaries(
            &conn,
            Network::Main,
            pool,
            3_459_989,
            anchor_size,
            (3_459_990, 3_459_998),
        )
        .expect("re-applying a standing bracket is not a fault");
        assert!(
            got.is_none(),
            "no correction was derived (the batch never reached the crossing), so \
             nothing is REPORTED — the endpoint is already refused at ingest by the \
             bracket that produced it. Got {got:?}"
        );
        assert_eq!(
            read_recorded_heights(&conn, pool).expect("read"),
            vec![Some(3_400_000), Some(3_460_005)],
            "and the row itself is untouched: the ledger says it is wrong, nothing \
             here can say what it should be, and NULL is never written from this path"
        );
        assert_eq!(
            latches(&conn),
            vec![1, 0, 0, 1],
            "the standing bracket at index 1 un-latches shard 1 AND shard 2 from the \
             ledger alone, and touches neither shard 0 nor shard 3. [shard 0, shard \
             1, shard 2, shard 3]"
        );
    }

    // ════════════════════════════════════════════════════════════════════════
    // S15-F1 phase B — the test author's unit rows, written BLIND to the
    // implementation (`docs/plan/s15-f1-subtree-roots-fetch-only-what-is-new.md`,
    // revision 2: §1 `read_stored_run`, §3.3 step 4.3 `check_pool_from`, step 4.5
    // `clear_boundary_bounds_in`; §7 rows T12, T15, T22). Appended at the END of
    // this module, in a child module of its own, so no cited line above moves and
    // the adapter below is the only place an implementer signature is named.
    // ════════════════════════════════════════════════════════════════════════
    mod s15_f1 {
        use super::*;
        use std::collections::BTreeSet;

        use crate::state::StallReason;

        // ════════════════════════════════════════════════════════════════════
        // ADAPTER: the one place the implementer's signatures are bound.
        // ════════════════════════════════════════════════════════════════════

        /// `check_pool_from`'s three answers, in this module's shape.
        #[derive(Debug, PartialEq, Eq)]
        enum Bound {
            /// Accepted, with the C6 set (§3.3 step 4.3: the indices `>= start`
            /// whose completing block is scanned and on which C6 compared, or
            /// abstained on a missing or wrong-length served hash).
            Accepted(BTreeSet<u64>),
            /// Refused, by the refusal's stable code.
            Refused(&'static str),
            /// `Sync { Internal }` — an outstanding rewind at `start > 0`, or a
            /// `start` the record cannot supply a prefix for.
            Internal,
        }

        /// §3.3 step 4.3. ASSUMES
        /// `check_pool_from(start, served, hashes, ev)
        ///     -> Result<Result<C6Set, HeightBindRefusal>, WalletError>`
        /// — the outer `Result` the OUR-side fault (`Sync { Internal }`), the inner
        /// the endpoint's refusal as `check_pool` returns it, and the C6 set any
        /// collection of unsigned indices. `start as _` binds whatever unsigned
        /// type the implementer chose.
        fn bind_from(
            start: u64,
            served: &[CommitmentTreeRoot<MerkleHashOrchard>],
            hashes: &[Vec<u8>],
            ev: &PoolEvidence,
        ) -> Bound {
            match check_pool_from(start as _, served, hashes, ev) {
                Ok(Ok(set)) => Bound::Accepted(set.into_keys().collect()),
                Ok(Err(refusal)) => Bound::Refused(refusal.code()),
                Err(WalletError::Sync {
                    stall: StallReason::Internal,
                }) => Bound::Internal,
                Err(other) => panic!("check_pool_from: an unexpected fault {other:?}"),
            }
        }

        /// §1. ASSUMES `read_stored_run(conn, pool) -> Result<u64, WalletError>`
        /// (fails closed on a missing table, like every sibling read here). If
        /// it returns a bare `u64`, wrap it in `Ok` and drop the missing-table row.
        fn stored_run(conn: &Connection, pool: ShieldedProtocol) -> Result<u64, WalletError> {
            read_stored_run(conn, pool)
        }

        /// §3.3 step 4.5. ASSUMES `clear_boundary_bounds_in(conn, pool, start, end)
        /// -> Result<(), WalletError>` over the half-open `[start, end)`, with
        /// unsigned index arguments.
        fn clear_in(
            conn: &Connection,
            pool: ShieldedProtocol,
            start: u64,
            end: u64,
        ) -> Result<(), WalletError> {
            clear_boundary_bounds_in(conn, pool, start as _, end as _)
        }

        // ════════════════════════════════════════════════════════════════════
        // End of ADAPTER.
        // ════════════════════════════════════════════════════════════════════

        /// The verdict alone — what `check_pool` (no C6 set) can be compared on.
        fn verdict(b: &Bound) -> Result<(), &'static str> {
            match b {
                Bound::Accepted(_) => Ok(()),
                Bound::Refused(code) => Err(code),
                Bound::Internal => Err("internal"),
            }
        }

        fn full_verdict(
            served: &[CommitmentTreeRoot<MerkleHashOrchard>],
            hashes: &[Vec<u8>],
            ev: &PoolEvidence,
        ) -> Result<(), &'static str> {
            check_pool(served, hashes, ev).map_err(|r| r.code())
        }

        /// A 32-byte completing-block id for index `i`, never its own reverse.
        fn block(i: usize) -> Vec<u8> {
            let mut v = vec![0u8; 32];
            v[..8].copy_from_slice(&(i as u64).to_le_bytes());
            v[31] = 0xC6;
            v
        }

        /// A different 32-byte id for `i` — a served lie.
        fn lie(i: usize) -> Vec<u8> {
            let mut v = block(i);
            v[30] = 0x11;
            v
        }

        /// Ten blocks apart from 1,000,000.
        fn h(i: usize) -> u32 {
            1_000_000 + 10 * u32::try_from(i).expect("small")
        }

        fn recorded_run(n: usize) -> Vec<Option<u32>> {
            (0..n).map(|i| Some(h(i))).collect()
        }

        fn served_run(range: std::ops::Range<usize>) -> Vec<CommitmentTreeRoot<MerkleHashOrchard>> {
            range.map(|i| root(h(i))).collect()
        }

        proptest::proptest! {
            #![proptest_config(proptest::prelude::ProptestConfig::with_cases(256))]

            /// **T12** — §3.3 step 4.3 / ledger row 3: with no rewind outstanding,
            /// `check_pool_from(start, served, ..)` over the virtual sequence
            /// `recorded[..start] ++ served` reaches the SAME verdict (accept, or
            /// refuse with the same code) as `check_pool` on the full honest
            /// re-serve carrying its own prefix hashes.
            ///
            /// Generated: a stored run of 1–260 heights (gaps 1–40, so the
            /// completion-gap floor sometimes fires), 0–5 new roots above it, up to
            /// three bundled rows and a scanned row anywhere in or around the span,
            /// up to two brackets, up to four scanned completing blocks, and served
            /// hashes per suffix index that are absent, honest, or a lie. The
            /// prefix hashes are honest or absent (the premise: a re-serve that
            /// passes recorded equality carries the recorded values).
            ///
            /// `start` ranges over EVERY multiple of 64 at or below `count − 1` —
            /// a superset of what `plan_pool_fetch` can return, so the property is
            /// independent of the plan (the contract words it "the plan from
            /// `plan_pool_fetch`"; the superset is the stronger statement and keeps
            /// the plan's adapter out of this module).
            ///
            /// Mutants: a suffix-only bind (`check_against_count` and recorded
            /// equality positional from the suffix's 0); the prefix dropped from
            /// any one oracle.
            #[test]
            fn check_pool_from_matches_a_full_honest_re_serve(
                n in 1usize..=260,
                ext in 0usize..6,
                gaps in proptest::collection::vec(1u32..=40, 266),
                bundled in proptest::collection::vec((0u32..12_000, 0u64..270), 0..4),
                scanned in proptest::option::of((0u32..12_000, 0u64..270)),
                brackets in proptest::collection::vec(
                    (0usize..266, proptest::option::of(0u32..12_000), proptest::option::of(0u32..12_000)),
                    0..3,
                ),
                scanned_at in proptest::collection::vec(0usize..266, 0..4),
                suffix_mode in proptest::collection::vec(0u8..3, 266),
                prefix_honest in proptest::prelude::any::<bool>(),
                k in 0usize..5,
            ) {
                let base = 1_000_000u32;
                let total = n + ext;
                let mut heights: Vec<u32> = Vec::with_capacity(total);
                let mut at = base;
                for g in gaps.iter().take(total) {
                    at += g;
                    heights.push(at);
                }
                let mut bundled_rows: Vec<(u32, u64)> =
                    bundled.iter().map(|&(o, c)| (base + o, c)).collect();
                bundled_rows.sort_unstable();
                let ev = PoolEvidence {
                    recorded: heights[..n].iter().map(|&x| Some(x)).collect(),
                    bundled: bundled_rows,
                    scanned: scanned.map(|(o, c)| (base + o, c)),
                    scanned_hashes: scanned_at
                        .iter()
                        .filter(|&&i| i < total)
                        .map(|&i| (heights[i], block(i)))
                        .collect(),
                    bounds: brackets
                        .iter()
                        .map(|&(i, f, c)| {
                            (
                                i,
                                BoundaryBounds {
                                    floor: f.map(|o| base + o),
                                    ceiling: c.map(|o| base + o),
                                },
                            )
                        })
                        .collect(),
                    ..PoolEvidence::default()
                };
                let start = 64 * k.min((n - 1) / 64);
                let full: Vec<_> = heights.iter().map(|&x| root(x)).collect();
                let hashes: Vec<Vec<u8>> = (0..total)
                    .map(|i| {
                        if i < start {
                            if prefix_honest { block(i) } else { Vec::new() }
                        } else {
                            match suffix_mode[i] {
                                0 => Vec::new(),
                                1 => block(i),
                                _ => lie(i),
                            }
                        }
                    })
                    .collect();
                let from = bind_from(start as u64, &full[start..], &hashes[start..], &ev);
                proptest::prop_assert_ne!(&from, &Bound::Internal, "no rewind, an in-range start");
                proptest::prop_assert_eq!(
                    verdict(&from),
                    full_verdict(&full, &hashes, &ev),
                    "start {} of a {}-root run ++ {} new",
                    start,
                    n,
                    ext
                );
            }
        }

        /// **T12, the example** — §3.3 step 4.3: the bundled count bind is
        /// POSITIONAL from index 0 (`check_against_count`), so it must run over
        /// the virtual sequence. A bundled row proves exactly 150 subtrees
        /// complete at `h(149) + 5`:
        /// * an honest incremental serve from 128 (indices 128..200) is ACCEPTED —
        ///   a suffix-only bind would read served[22] (index 150) as index 22,
        ///   "below 150 ⇒ at or below the row", and refuse it;
        /// * a serve that adds index 150 at `h(149) + 3` (at or below the row,
        ///   where the bundle says subtree 150 had not completed) is REFUSED with
        ///   `bundled_frontier` — index 150 is new (the run is 150), so recorded
        ///   equality cannot be what refuses it.
        ///
        /// Both verdicts are also `check_pool`'s on the same virtual sequence.
        ///
        /// Mutant: a suffix-only bind (row 1 refused).
        #[test]
        fn check_pool_from_counts_the_stored_prefix() {
            let row = (h(149) + 5, 150u64);

            let ev = PoolEvidence {
                recorded: recorded_run(192),
                bundled: vec![row],
                ..PoolEvidence::default()
            };
            let served = served_run(128..200);
            assert!(
                matches!(bind_from(128, &served, &[], &ev), Bound::Accepted(_)),
                "an honest re-serve from 128 counts from index 0 of the virtual sequence"
            );
            assert_eq!(full_verdict(&served_run(0..200), &[], &ev), Ok(()));

            let ev = PoolEvidence {
                recorded: recorded_run(150),
                bundled: vec![row],
                ..PoolEvidence::default()
            };
            let mut lying = served_run(128..150);
            lying.push(root(h(149) + 3));
            let bundled_code = HeightBindRefusal::BundledFrontier {
                index: 0,
                served: 0,
                at_height: 0,
                complete: 0,
            }
            .code();
            assert_eq!(
                bind_from(128, &lying, &[], &ev),
                Bound::Refused(bundled_code),
                "index 150 at or below the row is refused at its VIRTUAL index"
            );
            let mut full = served_run(0..150);
            full.push(root(h(149) + 3));
            assert_eq!(full_verdict(&full, &[], &ev), Err(bundled_code));
        }

        /// §3.3 step 4.3: `check_pool_from` "returns `Sync { Internal }` (not a
        /// debug assertion) if a rewind is outstanding when `start > 0`" — the
        /// relaxed bind is only ever run over a FULL fetch (ledger row 5). At
        /// `start = 0` the same evidence is an ordinary relaxed full bind.
        ///
        /// Mutant: the guard removed (a relaxed incremental bind accepted), or a
        /// `debug_assert!` in its place (release builds accept).
        #[test]
        fn check_pool_from_refuses_a_relaxed_bind_at_a_nonzero_start() {
            let ev = PoolEvidence {
                recorded: recorded_run(200),
                rewind: RewindWatch {
                    observed: 1,
                    recorded_at: 0,
                },
                ..PoolEvidence::default()
            };
            assert_eq!(
                bind_from(64, &served_run(64..200), &[], &ev),
                Bound::Internal
            );
            assert!(matches!(
                bind_from(0, &served_run(0..200), &[], &ev),
                Bound::Accepted(_)
            ));
        }

        /// §3.3 step 4.3: `heights = ev.recorded.get(..start)? ++ served` — a
        /// start the record cannot supply a prefix for is `Sync { Internal }`,
        /// never a slicing panic. Also a `None` inside the prefix (§3.2: "every
        /// index below `count` is `Some` … a `None` there returns `Sync {
        /// Internal }`").
        ///
        /// Mutant: `&ev.recorded[..start]` (a panic).
        #[test]
        fn check_pool_from_never_slices_past_the_record() {
            let ev = PoolEvidence {
                recorded: recorded_run(100),
                ..PoolEvidence::default()
            };
            assert_eq!(
                bind_from(128, &served_run(128..140), &[], &ev),
                Bound::Internal
            );

            let mut holed = recorded_run(200);
            holed[50] = None;
            let ev = PoolEvidence {
                recorded: holed,
                ..PoolEvidence::default()
            };
            assert_eq!(
                bind_from(64, &served_run(64..200), &[], &ev),
                Bound::Internal
            );
        }

        /// §3.3 step 4.3 / §3.1 `c6_seen`: the C6 set is exactly the served
        /// indices whose completing block is SCANNED and on which C6 compared
        /// (either byte order) or abstained on a missing or wrong-length served
        /// hash — never a prefix index, never an unscanned one. And a scanned
        /// mismatch is still refused.
        ///
        /// Mutants: the prefix's `[empty; start]` hashes counted (10 in the set);
        /// an unscanned index counted (100); a missing- or wrong-length abstain
        /// left out (80, 90); a reversed-order compare left out (110); the set
        /// indexed from the suffix's 0 (every member shifted by 64).
        #[test]
        fn check_pool_from_returns_the_c6_set_of_the_served_part() {
            let start = 64usize;
            let scanned_hashes: HashMap<u32, Vec<u8>> = [10usize, 70, 80, 90, 110]
                .iter()
                .map(|&i| (h(i), block(i)))
                .collect();
            let ev = PoolEvidence {
                recorded: recorded_run(200),
                scanned_hashes,
                ..PoolEvidence::default()
            };
            let mut hashes: Vec<Vec<u8>> = vec![Vec::new(); 200 - start];
            let at = |i: usize| i - start;
            hashes[at(70)] = block(70);
            // 80: no hash served (abstain on missing).
            hashes[at(90)] = block(90)[..31].to_vec(); // wrong length
            hashes[at(100)] = block(100); // a hash for a block never scanned
            hashes[at(110)] = block(110).into_iter().rev().collect(); // display order
            assert_eq!(
                bind_from(start as u64, &served_run(start..200), &hashes, &ev),
                Bound::Accepted(BTreeSet::from([70u64, 80, 90, 110]))
            );

            hashes[at(70)] = lie(70);
            let c6 = HeightBindRefusal::CompletingBlockHash {
                index: 0,
                height: 0,
            }
            .code();
            assert_eq!(
                bind_from(start as u64, &served_run(start..200), &hashes, &ev),
                Bound::Refused(c6)
            );
        }

        /// **T15** — §3.3 step 4.5: an accepted write from `start` withdraws the
        /// brackets in `[start, start + len)` and NO others — not the ones below
        /// the start (a prefix this pass did not re-serve), not the one at `end`,
        /// not another pool's. Edges on both sides of both bounds (63/64,
        /// 127/128).
        ///
        /// Mutants: clearing below `end` (today's `_below` — 0 and 63 gone);
        /// `<= end` (128 gone); `> start` (64 kept); the pool ignored.
        #[test]
        fn clear_boundary_bounds_in_withdraws_only_the_written_range() {
            let conn = Connection::open_in_memory().expect("in-memory db");
            ensure_boundary_bound_table(&conn).expect("ensure");
            let bounds = |floor| BoundaryBounds {
                floor: Some(floor),
                ceiling: None,
            };
            for i in [0usize, 63, 64, 100, 127, 128, 500] {
                record_boundary_bounds(
                    &conn,
                    ShieldedProtocol::Sapling,
                    i,
                    bounds(1_000 + i as u32),
                )
                .expect("bracket");
            }
            record_boundary_bounds(&conn, ShieldedProtocol::Orchard, 100, bounds(5_000))
                .expect("bracket");
            let keys = |pool| -> BTreeSet<usize> {
                read_boundary_bounds(&conn, pool)
                    .expect("read")
                    .into_keys()
                    .collect()
            };

            clear_in(&conn, ShieldedProtocol::Sapling, 64, 128).expect("clear [64, 128)");
            assert_eq!(
                keys(ShieldedProtocol::Sapling),
                BTreeSet::from([0, 63, 128, 500])
            );
            assert_eq!(keys(ShieldedProtocol::Orchard), BTreeSet::from([100]));

            clear_in(&conn, ShieldedProtocol::Sapling, 128, 128).expect("an empty range");
            assert_eq!(
                keys(ShieldedProtocol::Sapling),
                BTreeSet::from([0, 63, 128, 500]),
                "an empty range withdraws nothing"
            );

            clear_in(&conn, ShieldedProtocol::Sapling, 0, 1).expect("clear [0, 1)");
            assert_eq!(
                keys(ShieldedProtocol::Sapling),
                BTreeSet::from([63, 128, 500])
            );
        }

        /// **T22, at the reader** — §1: `read_stored_run` is the LEADING run of
        /// rows with `subtree_end_height IS NOT NULL AND root_hash IS NOT NULL`,
        /// contiguous from index 0 — upstream's own rebuild rule
        /// (`truncate_tree_to_subtree_roots` breaks at the first index gap). It
        /// fails closed on a missing table. The wallet-level row is
        /// `wallet::tests::s15_roots::a_null_root_hash_ends_the_stored_run`,
        /// which also carries the probe note on how a scan leaves the NULL.
        ///
        /// Mutants: `count` from heights alone (the NULL-root row reads 5); the
        /// NULL-height row ignored (reads 5); a row COUNT rather than a leading
        /// run (the gap row reads 5, the no-zero row 4); a missing table read as
        /// 0 (the permissive answer — it plans as "nothing stored", which is
        /// safe, but every sibling read here fails closed).
        #[test]
        fn read_stored_run_is_the_leading_run_of_complete_rows() {
            type ShardRow = (i64, Option<u32>, Option<Vec<u8>>);
            let full = |i: i64| -> ShardRow {
                (i, Some(500_000 + 10 * i as u32), Some(vec![i as u8 + 1]))
            };
            let rows: Vec<(&str, Vec<ShardRow>, u64)> = vec![
                ("empty", vec![], 0),
                ("five complete rows", (0..5).map(full).collect(), 5),
                (
                    "a NULL root at 3",
                    (0..5)
                        .map(|i| {
                            if i == 3 {
                                (i, full(i).1, None)
                            } else {
                                full(i)
                            }
                        })
                        .collect(),
                    3,
                ),
                (
                    "a NULL height at 2",
                    (0..5)
                        .map(|i| {
                            if i == 2 {
                                (i, None, full(i).2)
                            } else {
                                full(i)
                            }
                        })
                        .collect(),
                    2,
                ),
                (
                    "a gap at 4",
                    [0, 1, 2, 3, 5].into_iter().map(full).collect(),
                    4,
                ),
                ("no row 0", (1..5).map(full).collect(), 0),
                (
                    "a NULL root at 0",
                    (0..5)
                        .map(|i| {
                            if i == 0 {
                                (i, full(i).1, None)
                            } else {
                                full(i)
                            }
                        })
                        .collect(),
                    0,
                ),
            ];
            for (name, shard_rows, expected) in rows {
                let conn = Connection::open_in_memory().expect("in-memory db");
                conn.execute_batch(
                    "CREATE TABLE sapling_tree_shards (
                         shard_index INTEGER PRIMARY KEY,
                         subtree_end_height INTEGER,
                         root_hash BLOB
                     );",
                )
                .expect("table");
                for (i, height, hash) in shard_rows {
                    conn.execute(
                        "INSERT INTO sapling_tree_shards VALUES (?1, ?2, ?3)",
                        rusqlite::params![i, height, hash],
                    )
                    .expect("row");
                }
                assert_eq!(
                    stored_run(&conn, ShieldedProtocol::Sapling).expect("read"),
                    expected,
                    "{name}"
                );
            }
            let bare = Connection::open_in_memory().expect("in-memory db");
            assert!(
                stored_run(&bare, ShieldedProtocol::Sapling).is_err(),
                "a missing table fails closed"
            );
        }
    }
}
