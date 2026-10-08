# Bundled checkpoint provenance (spec §3.6 / §4.6 M2)

`src/checkpoints/data.rs` is GENERATED checkpoint data — `(height, block_time)`
anchors used by `estimate_birthday` (the conservative-floor restore-height
estimator). This file pins the reproducible provenance so a reviewer
regenerates **byte-identical** data and can audit that nothing was hand-altered
(M2: a poisoned checkpoint must never silently hide funds).

## Source (canonical, MIT)

The `checkmate`-generated checkpoint set the ECC mobile SDKs ship — the exact
data the reference ZEC wallets (Zashi/Zodl) use. We consume it as DATA (ADR-0005
mine-as-reference; the "REUSE AS DATA" classification, spec §1.8), never as a
dependency.

- **Repo:** `Electric-Coin-Company/zcash-android-wallet-sdk`
- **Path:** `sdk-lib/src/main/assets/co.electriccoin.zcash/checkpoint/{mainnet,testnet}/*.json`
- **Commit:** `3819943dc64878beb60bd2cb92d1a8547a558f94` ("Generate checkpoints
  for 3.0.0", 2026-08-25) — the newest upstream commit that TOUCHES the
  checkpoint assets, not the branch head (a dependabot merge carries no data).
- **Retrieved:** 2026-09-06 (refreshed from `1c2ef03078bbe1535d49c4184a701f90b0151929`,
  retrieved 2026-06-13)
- **JSON shape:** `{ "network", "height", "hash", "time", "saplingTree"
  [, "orchardTree"][, "ironwoodTree"] }`. The emitter PINS this key set and
  aborts on an unknown key, so upstream schema growth is loud rather than
  silently dropped.

### `ironwoodTree` — CARRIED as of the Phase B pin wave (2026-09-07)

The refresh to `3819943` introduced an `ironwoodTree` field: **13 mainnet rows
(3429810…3459780) and 25 testnet rows (4134750…4301840)** — every one of them a
row that refresh ADDED. It was dropped on purpose while
`zcash_client_backend` 0.23.0's `TreeState` modelled sapling + orchard only and
the pool could not be conveyed to the decoder at all.

**That debt is now paid.** `zcash_client_backend` 0.24.0 models the pool
(`TreeState.ironwood_tree`, proto tag 7; `ChainState::new` takes a REQUIRED
`final_ironwood_tree`), so the bundle was **re-extracted at the SAME upstream
commit** with the field carried and the tree-state table widened 4-ary → 5-ary.

What changed and what did not, measured rather than asserted:

- The emitter was first run against `3819943` UNCHANGED and reproduced the
  committed `data.rs` **byte-for-byte** (`cmp`), which is what makes the second
  run's output trustworthy.
- Every pre-existing column of all **1,242** tree-state rows (821 mainnet + 421
  testnet) is **byte-identical**; the diff is entirely the added fifth column.
  `git diff` shows ~1,242 "deleted" rows for that reason — this is a SHAPE
  change, not a refresh, so the "insertions only" rule in the refresh section
  below does not apply to it. The check that does apply is the one that was run:
  columns 1–4 unchanged, row-for-row.
- The `(height, time)` slice is **untouched** — zero changed lines, and its
  pinned hash is unchanged. That is the cheap corroboration that no source row
  moved.
- The tree-state slice's pinned hash MOVED, because the canonical form gained
  the Ironwood column. New value below.
- The emitter now ASSERTS the Ironwood boundary the way it already asserted
  NU5/orchard: the frontier must be absent below the NU6.3 activation
  (mainnet 3,428,143 / testnet 4,134,000) and present at or above it. Measured
  against the source: 0 rows violate it in either direction.

Why this mattered, kept because the hazard has not gone away, only moved:
`bundled_treestate` is the TRUSTED anchor, and the chain state it decodes to is
persisted and immutable after first import. A wallet provisioned at one of those
heights against a bundle without the field bakes in a chain state that omits a
pool that is live on mainnet — and `TreeState::ironwood_tree()` returns an EMPTY
tree for an empty field **silently**, so nothing would have failed. That is the
same wrong-position class `treestate_bundle_frontiers_grow_and_orchard_activates_at_nu5`
kills for orchard, and it is now guarded for Ironwood by
`treestate_bundle_carries_an_ironwood_frontier_after_activation` (raw-field
boundary + decoded non-empty `tree_size()` + append-only growth + the 13/25 row
counts).

**UPDATE 2026-09-06 — the create side of this debt is closed
(`ironwood-nu63-support.md` §1.5 / Phase A).** `bundled_treestate` now applies an
ANCHOR CEILING: it never selects a row above the activation height of the newest
network upgrade this build knows (`consensus::newest_known_activation`, derived
from the compiled params, never a literal). With the current pin that ceiling is
Nu6_2's 3,364,600, so **all 13 mainnet rows above it are unreachable as anchors**
and a pre-Ironwood build can no longer create a wallet with an implicitly-empty
third tree. Two things this does NOT fix, both owed to Phase B:

1. Wallets provisioned *before* the ceiling landed still hold the bad anchor —
   whether they need a forced re-import + rescan on first open after the bump
   is OPEN — upstream 0.22.0's own `ironwood_shardtree` migration re-queues
   every range at or above the activation for rescan, which may already repair
   it. ADR-0540 Decision 6 freezes the SCOPE (which accounts) and deliberately
   not the remedy; `docs/plan/ironwood-phase-b.md` §2 settles it.
2. ~~The bundle itself still drops `ironwoodTree`.~~ **DONE 2026-09-07**, in the
   same commit as the crate bump — see the section above.

**UPDATE 2026-09-07 — the ceiling MOVED, in this same commit (Phase B step 6).**
It does NOT rise by itself at a crate bump — `newest_known_activation` reads
`consensus::KNOWN_UPGRADES`, a HAND-MAINTAINED list — and an earlier revision of
this file said it did, copied from an over-broad code comment now fixed at its
definition site. Adding `NetworkUpgrade::Nu6_3` to that list is what moved it,
from Nu6_2's 3,364,600 to Nu6_3's **3,428,143**.

Measured effect, because the obvious guess is wrong: the 13 mainnet Ironwood rows
this re-extraction added are **still unreachable as anchors**. `bundled_treestate`
selects the newest row strictly below `ceiling + 1`, and the first of those rows
is 3,429,810. The ceiling deliberately stays below the newest activation: past it,
an upgrade this build does not know may be live — as true of NU7 now as it was of
NU6.3 a day ago. What actually changed is the newest anchorable row, **3,362,500
→ 3,427,310**: 26 further rows in reach and roughly **64,810 fewer blocks** of
first scan (the cost was ~95,000 blocks at the previous pin). Every one of those
rows sits BELOW the activation, where an empty Ironwood frontier is correct —
which is what makes the raise safe, and is asserted by
`consensus::tests::the_anchor_ceiling_is_the_newest_upgrade_this_build_can_model`
rather than assumed.

`data.rs` carries TWO row-aligned slices of the SAME source rows:
- `estimate_birthday` needs only `(height, time)` (§3.6).
- `bundled_treestate` (W3-inc-2c-iv-c, §3.2f) needs the tree-state fields
  (`hash`/`saplingTree`/`orchardTree`) — the TRUSTED birthday anchor that
  provisioning uses INSTEAD of a live `get_tree_state` (the crypto audit proved
  a live frontier is cryptographically unverifiable; bundling it in the signed
  binary is the only real defense). Both slices come from the same JSONs.

## Extraction (deterministic)

Implemented by `tools/extract_checkpoints.py` — the steps below are what it
does, kept here so a reviewer can audit the emitter itself:

```sh
tools/extract_checkpoints.py <checkpoint-dir> <upstream-commit> src/checkpoints/data.rs
```

It prints both canonical SHA-256 hashes and re-asserts every invariant. **Prove
the emitter before trusting its output:** run it against the PREVIOUS pinned
commit first — it must reproduce the committed `data.rs` byte-for-byte (`cmp`,
not just a matching hash; that also pins the formatting).

1. Read every `*.json` under the mainnet and testnet dirs.
2. Take `(int(height), int(time))` from each.
3. Sort ascending by `height`.
4. Assert the invariant: heights AND times STRICTLY increasing; the first row is
   the network's Sapling activation (`mainnet 419200`, `testnet 280000`).
5. Emit `pub(super) const MAINNET/TESTNET: &[(u32, u64)]` to `src/checkpoints/data.rs`.
6. **Tree-state slice (§3.2f):** from the SAME rows, take
   `(int(height), hash, saplingTree, orchardTree, ironwoodTree)` (an absent
   frontier ⇒ `""`, correct below that pool's activation), sort by `height`, and
   emit
   `pub(super) const MAINNET_TREESTATES/TESTNET_TREESTATES: &[(u32, &str, &str, &str, &str)]`
   to the same file. The two slices are row-aligned by construction (same source
   rows, same sort) — asserted by `treestate_and_time_slices_are_row_aligned`.
7. **Activation boundaries.** Assert each shielded frontier is ABSENT below its
   pool's activation and PRESENT at/after it: orchard at NU5 (mainnet 1687104 /
   testnet 1842420), ironwood at NU6.3 (mainnet 3428143 / testnet 4134000). An
   absent frontier decodes to an empty tree with no error, so a regression here
   has no other place to surface.

The same invariant is re-checked at runtime (`is_monotonic`, fail-closed to the
activation height) and pinned by the test
`checkpoint_bundle_monotonic_or_load_fails`. Every tree-state row is proven to
decode to a valid `AccountBirthday` by `treestate_bundle_every_row_decodes`.

**A refresh also has to keep the Ironwood gap empty.**
`treestate_bundle_samples_no_row_between_ironwood_activation_and_the_reimport_threshold` asserts that no row
falls between the Ironwood activation and the first row above it (mainnet
`[3428143, 3429810)`, testnet `[4134000, 4134750)`). That row-absence is what
lets Phase B's re-import threshold narrow below the activation height
(ADR-0540 Decision 6) — a refresh that adds a row in the gap fails the test, and
the answer is to widen the threshold, never to retune the test.

## As built (this chunk)

- Mainnet: 821 anchors, `419200 @ 1540779337` … `3459780 @ 1787630999`
  (tail 2026-08-25).
- Testnet: 421 anchors, `280000 @ 1535262293` … `4301840 @ 1787692420`
  (tail 2026-08-25).
- Tree-state slice (§3.2f, iv-c): the SAME 821 / 421 rows, each carrying
  `(hash, saplingTree, orchardTree, ironwoodTree)`. The Ironwood frontier is
  ABSENT below the NU6.3 activation (mainnet 3428143, testnet 4134000) and
  present on the 13 mainnet rows from 3429810 and the 25 testnet rows from
  4134750 — every row the bundle carries at or above the activation.
  The Orchard frontier is ABSENT below the
  NU5 ACTIVATION (mainnet 1687104, testnet 1842420 — `zcash_protocol::consensus`)
  and present from the activation row onward, where it is the empty tree
  `"000000"` rather than absent. ≈ 1.94 MiB of frontier hex.
  (Earlier revisions of this file said "pre-NU5 (mainnet ≤ 1680000, testnet
  ≤ 1840000)". Those are the last checkpoint ROWS below NU5, not the
  activations; the claim was false and the emitter's assertion passed only
  because no bundled row falls in the gap between the two.)

## Pinned content hash (the machine-checkable integrity gate)

The runtime monotonic self-check rejects reordered/duplicated/regressed rows but
CANNOT detect a still-monotonic value tamper (a lowered interior `time` shifts
the floor later → overshoot → silent fund loss). That class is closed HERE: the
canonical content hash is pinned in `src/checkpoints/mod.rs`
(`checkpoint_data_matches_pinned_provenance_hash`), so ANY change to `data.rs`
fails CI and forces re-derivation from the source above.

Each slice is pinned SEPARATELY (a change to either fails CI).

`(height, time)` slice (`checkpoint_data_matches_pinned_provenance_hash`):
- **Canonical form:** `MAINNET` then `TESTNET`, each row = `u32` LE height ‖ `u64` LE time.
- **SHA-256:** `7aedb445336f451d09b87d58a150cf819b46f9770680f4883080485d5877369b`
  (was `bc3bf8d251cfcfc563702ba060ee4dc2249d35202ac6ff8d3a3a27f6edb9e6f7` @ `1c2ef03`)

Tree-state slice (`treestate_bundle_matches_pinned_provenance_hash`):
- **Canonical form:** `MAINNET_TREESTATES` then `TESTNET_TREESTATES`, each row =
  `u32` LE height ‖ for each of (`hash`, `saplingTree`, `orchardTree`,
  `ironwoodTree`): `u32` LE byte-length ‖ the ASCII hex bytes.
- **SHA-256:** `f405eb5f714b88d831df4b6363eae0cb0f62c00f5e34e885a87f4eb9d7970605`
  (was `78b9cb116b80abb5cd5339d1f19fe10a4b0056c0d690490c317895b562e0aab7` at the
  SAME upstream commit `3819943`, before the canonical form gained the Ironwood
  column; and `59006014e513bdfb8bd70e34d6c707fa253de13078c4bf73f40bb7c223188487`
  @ `1c2ef03`). **The source rows did not change** — the `(height, time)` hash is
  unchanged across this move, which is the check that says so.

## Re-verification / refresh (any bundle bump)

1. Re-clone the source at a newer commit; re-run the extraction (steps 1–6,
   BOTH slices — they must stay row-aligned).
2. `git diff --stat src/checkpoints/data.rs` — a legitimate refresh is
   **insertions only, apart from GENERATED COMMENT lines** (the header commit
   line, and the per-slice doc comments if their wording changed). Any deletion
   that is a `(height, …)` ROW means a historical row moved: STOP (see 3).
   Read the deletions, do not just count them. Then update the commit + counts +
   BOTH pinned hashes above.
   The 2026-09-06 refresh ended at `189 insertions(+), 3 deletions(-)` = 186
   appended rows + 3 changed comment lines (the header commit + the two
   `_TREESTATES` doc comments, whose NU5 wording was corrected in the same
   change). Zero row deletions.
3. A refreshed bundle only EXTENDS the tail (newer checkpoints) — existing rows
   are immutable chain history; a changed historical row is a red flag, not an
   update. (Bumping the tail also gives newer wallets a closer trusted frontier,
   shortening the from-bundle scan — §3.2f.)
4. **Spot-check the new rows against an INDEPENDENT view of the chain.** Every
   check above is internal — it proves we faithfully copied upstream, never that
   upstream is honest. At minimum verify the new tail row and the first row
   after the old tail (the boundary), block hash AND time, against a source that
   is not the ECC repo:

   ```sh
   curl -sL 'https://api.blockchair.com/zcash/blocks?q=id(<height>)'
   ```

   Check **every new mainnet row**, not a sample — the whole set runs in a couple
   of minutes from a shell, so a sample is a choice to know less for no saving.
   2026-09-06 refresh: **all 47 new mainnet rows verified, block hash AND block
   time, zero mismatches** (spot-checks first at `3459780`, the tail
   2026-08-25 04:09:59, and `3345000`, the boundary 2026-05-17 01:26:13).
   Testnet has no comparable independent explorer and carries no money — it is
   left to the internal checks, deliberately.

   One third-party API is one source. Naming a second (a self-hosted `zcashd`,
   or a second explorer) is owed before this step can be called corroboration
   rather than a single outside opinion.
