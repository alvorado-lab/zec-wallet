# 0540 — Move the librustzcash pin set to the Ironwood/NU6.3 wave, and treat Orchard as a sealed pool

- **Status:** Accepted
- **Date:** 2026-09-07
- **Links:** amends by reference
  [ADR-0005](0005-zcash-wallet-librustzcash-direct.md) (its Decision paragraph
  names the pre-Ironwood crate versions; that paragraph is not edited — this
  ADR is where the current pin set lives) ·
  `docs/specs/ironwood-nu63-support.md` §9.2/§9.3/§9.4 ·
  `docs/plan/ironwood-phase-b.md` §1 step 1 (the work order this pin set is
  copied from; older docs cite a "§1.1" that does not exist) · `docs/plan/ironwood-phase-b-survey.md` C1/C2/C3/C8 (the evidence) ·
  [ADR-0534](0534-rescan-from-earlier-birthday-recovery.md) (`rescan_from` — the
  mechanism Decision 6's re-import reuses, not a new one) ·
  `evals/incidents.tsv` INC-009, INC-015, INC-016

## Context

Ironwood / NU6.3 activated on mainnet at **block 3,428,143, 2026-07-28**. Our
pinned `zcash_protocol =0.9.0` knows network upgrades only through `Nu6_2`
(mainnet 3,364,600), so for every height past activation `BranchId::for_height()`
returns `Nu6_2` while the network is on `Nu6_3` (`0x37a5_165b`). Every
transaction the SDK builds is signed against the wrong consensus branch, and
every node rejects it. The SDK has been unable to send since activation.

**It has also been unable to SEE incoming payments since the same date, and that
half went unrecorded until the S255 review.** Ironwood outputs ride
`CompactTx.ironwood_actions` — proto tag 9, new in `zcash_client_backend 0.24.0`
and absent from the pinned 0.23.0. `prost` skips unknown fields, so a pre-bump
build scans a post-activation block, finds nothing, and marks the range
*scanned*. Upstream states the consequence flatly (`zcash_client_sqlite 0.22.0`,
`v_address_uses_ironwood.rs`): *"After NU6.3 every payment to an Orchard receiver
is delivered in the Ironwood bundle, which makes an Ironwood-only receipt the
ordinary case rather than an exotic one."* Nothing is lost — a rescan recovers
it — but every such receipt in the last six weeks is on chain and invisible in
the wallet, with sync reporting 100%. The outage is two-sided, and Phase B's
definition of done has to test both sides.

Two things beneath that are not fixed by a branch id:

- **Ironwood seals the Orchard pool.** Per Shielded Labs, users "will no longer
  be able to send or receive ZEC inside the Orchard pool"; value leaves only
  through the ZIP-318 turnstile into the new Ironwood pool. A wallet holding
  Orchard funds cannot construct a spendable transaction under the old rules at
  all, branch id aside. **That marketing sentence is a near-absolute and the
  consensus rule is not — see Decision 5, which states the exception upstream
  actually implements. The exception is load-bearing here, not a quibble: it is
  what puts an Orchard note and an Ironwood note in the same transaction, which
  is what makes Decision 5's first hazard reachable at all.**
- **The pool is a third frontier.** `zcash_client_backend 0.24.0` models it:
  `TreeState` gains `ironwood_tree` and `ChainState::new` takes a **required**
  `final_ironwood_tree`. `TreeState::ironwood_tree()` returns an *empty tree*
  for an empty field, without erroring — correct only *at* activation, silently
  wrong above it.

ADR-0005 chose librustzcash-direct and named exact versions in its Decision
paragraph. ADRs here are append-only. The wave changes eleven of those pins,
Phase C adds a new direct dependency (`zcash_pool_migration`), and the sealed
pool is a chain property ADR-0005's world did not contain. Hence one new ADR.

An 11-part upstream survey (S254) corrected the pin set twice, in ways worth
recording because both were the same mistake: **a version number is not a
date.** An earlier count of nine came from reading our own manifests; the true
count is eleven, and one of the two additions (`shardtree`) is a hard resolver
failure, not a nicety.

## Decision

**1. The pin set.** Eleven crate pins move, in one atomic commit, across three
manifests. This table is the record; `docs/plan/ironwood-phase-b.md` §1 step 1 is the
work order it was copied from.

| Crate | From | To | Manifest |
|---|---|---|---|
| `zcash_client_sqlite` | 0.21.0 | **0.22.0** | `sdk/Cargo.toml:76` |
| `zcash_client_backend` | 0.23.0 | **0.24.0** | `sdk/Cargo.toml:83` |
| `orchard` | 0.14.0 | **0.15.5** | `sdk/Cargo.toml:96` |
| `shardtree` | 0.6.2 | **0.7** | `sdk/Cargo.toml:102` |
| `zcash_address` | 0.12.0 | **0.13.0** | `sdk/Cargo.toml:170` **and root `Cargo.toml:205`** |
| `zcash_keys` | 0.14.0 | **0.16.1** | `sdk/Cargo.toml:189` |
| `zcash_protocol` | 0.9.0 | **0.10.5** | `sdk/Cargo.toml:190` **and root `Cargo.toml:206`** |
| `zcash_transparent` | 0.8.0 | **0.10.0** | `sdk/Cargo.toml:202` |
| `zip321` | 0.8.0 | **0.9.0** | `sdk/Cargo.toml:220` **and root `Cargo.toml:212`** |
| `zcash_primitives` | 0.28.0 | **0.30.1** | `sdk/zec-wallet-core/Cargo.toml:56` |
| `zcash_proofs` | 0.28.0 | **0.30.0** | `sdk/zec-wallet-core/Cargo.toml:107` |

**2. `zcash_script` does NOT move. It stays `=0.4.5` (`sdk/Cargo.toml:212`).**
Its semver-latest `0.5.2` was published **2026-02-23**, three months *before*
the pinned `0.4.5` (**2026-05-29**): the 0.4.x and 0.5.x lines are parallel
republishes, `0.5.2` does not satisfy the `^0.4.3` that four crates in this wave
still require, and it reverts the `sig_op_count` consensus fix that 0.4.5
carries. `sapling-crypto =0.7.0`, `zip32 =0.2.1` and `secp256k1 0.29` do not
move either. **Semver-max is not newest — read `created_at`.**

**3. One atomic commit, three manifests.** There is no valid intermediate state.
A partial bump either fails to resolve or, worse, resolves to **two copies** of
a crate with no error, whose symptom reads as `expected TransactionRequest,
found TransactionRequest`. `zcash_address`, `zcash_protocol` and `zip321` are in
`pins_policy.rs`'s `POLICED` set and sit in the root manifest as well as the
SDK's; `wallet_pins_mirror_root_or_extracted` panics `PIN DRIFT` if only one
manifest moves — and that guard **self-disables** once the root manifest no
longer names `crates/relim-relay-wire`, so a green `pins_policy` is not proof it
ran.

**4. The supply chain the wave adds.** Three crates become **non-optional**
runtime dependencies that are absent from `sdk/Cargo.lock` today: `pczt 0.9.3`
and `zcash_pool_migration 0.1.0` (both via `zcash_client_sqlite 0.22.0`), and
`flume 0.11` (replacing `crossbeam-channel` in `zcash_client_backend 0.24.0`).
A `deny.toml` + advisory pass on all three lands in the same commit as the pins.
`zcash_pool_migration` additionally becomes a **direct** dependency in Phase C
(ZIP-318 turnstile) — a 0.1.0 crate on the money path, taken whole per
Development Principle 2, with its own crypto-change review cycle and its own
supply-chain review at that point.

**5. Orchard is a sealed pool, and the wallet says so rather than inferring it.**
Post-activation, receipts and ordinary change belong in Ironwood, and Orchard
value moves out through the ZIP-318 turnstile. State the rule as upstream
implements it, not as an absolute seal — `zcash_client_backend 0.24.0`
`fees/common.rs`: *"the turnstile forbids value from entering the Orchard pool:
change may return to Orchard only when the transaction spends Orchard notes, and
only if strictly less value returns to the pool than the notes remove from it."*
The exception is load-bearing, not a footnote: it is what puts an Orchard change
note and an Ironwood output note in the same transaction, which is what makes
the first hazard below reachable at all. Two consequences are frozen here:

- **Pool identity is never widened by a wildcard.** Every match the wave makes
  non-exhaustive (`proto_tag`, `from_pool_type`, the FFI `OutputPool`, the
  change-pool argument) gets an explicit `Ironwood` arm or an explicit
  fail-closed arm. Mapping `Ironwood => PROTO_ORCHARD` files a note claim under
  the wrong pool, and because `output_index` is per-pool — Orchard and Ironwood
  actions are separate `CompactTx` vectors with independent indices — the
  double-send witness can resolve the mis-tagged claim to the wallet's own
  Orchard change note at the same index, return a spurious `true`, and re-propose
  an already-broadcast transaction. The commoner direction is the mirror image:
  the lookup returns `None`, the intent wedges in "wait for created tx" forever,
  and no money moves. **`proto_tag` is the one site where fail-closed is not an
  option** — after activation every new shielded note the wallet holds is an
  Ironwood note, so a fail-closed arm refuses every send. It needs a real
  `PROTO_IRONWOOD` byte in `intent_store` (the storage-format SSOT) *and* the
  matching arm in `decode_claims`, which today hard-rejects any tag that is
  neither Sapling nor Orchard as `StoreCorrupt`. Adding the byte in `send.rs`
  alone compiles, writes the claim, and then fails every crash-recovery read of
  it — turning the double-send guard into a hard error on exactly the sends it
  exists to protect.
- **Consensus behaviour tracks the branch id at the target height, never a
  literal.** Upstream offers `BundleVersion::orchard_v2()` /
  `OrchardCircuitVersion::FixedPostNu6_2` as the "keep current behaviour" answer
  to each newly-mandatory argument. Taking them makes the wave compile and
  leaves the wallet signing under pre-Ironwood rules — the same shape as the
  S252 guard that compared constants and could not see an upgrade.

**6. If a pre-Ironwood anchor has to be repaired, this is the population — and
this ADR does not decide that it has to be.** A wallet provisioned before the
pool was modelled holds an `AccountBirthday` whose third frontier is implicitly
empty; `zcash_client_sqlite` seeds the shard tree from it at import
(`rewind_to_chain_state(birthday.prior_chain_state())`).

**What is decided here is the SCOPE, not the remedy.** The affected population is
accounts whose birthday is at or above the **lowest bundled tree-state row at or
above the Ironwood activation**: mainnet **3,429,810**, testnet **4,134,750**.
Whether those accounts need a forced re-import at all is deliberately left open,
because the S255 review found two upstream mechanisms that may already repair
them and neither had been read: `zcash_client_sqlite 0.22.0`'s
`ironwood_shardtree` migration ends by splitting the scan queue at the NU6.3
activation and re-queueing everything above it at `Historic` priority *"so those
historical blocks are rescanned with Ironwood tree state enabled"*, and
`zcash_client_backend 0.24.0`'s `update_tree` inserts
`from_state.final_ironwood_tree()` as a checkpoint on **every** batch — and
Decision 7's measurement says our endpoints do supply a real frontier there.
Freezing "unrepairable in place" into an append-only record on a premise the
target crate may contradict is the S254 mistake exactly: the answer was on disk
in the crate we are upgrading to. **The money step reads `put_blocks` /
`update_tree` / `rewind_to_chain_state` and decides, in its own crypto-change review
cycle with its own review round** (`docs/plan/ironwood-phase-b.md` §2).

The threshold above is narrower than the activation height itself (3,428,143),
and it is allowed to be **only because a test says so, not because the window
looks safe**.
`treestate_bundle_samples_no_row_between_ironwood_activation_and_the_reimport_threshold`
asserts that no bundled row falls in `[activation, threshold)`, so no wallet can
be *anchored* there — `bundled_treestate` floors to a bundled row and the
birthday is `row + 1`, so a birthday anywhere in that window still anchors on the
newest row *below* activation, where an empty Ironwood frontier is correct by
consensus. The same test freezes the direction: a threshold may narrow toward the
activation, never rise, because raising it drops already-anchored wallets out of
remediation. The plausible-sounding alternative
justification is false and was measured to be false: against zec.rocks (ECC
LightWalletD v0.5.3) on 2026-09-07, mainnet `TreeState` field 7 is absent below
activation, present-and-empty at exactly 3,428,143, and **non-empty from
3,428,144** — the next block (testnet: from 4,134,683, so an empty frontier is
correct for one height inside the window on mainnet and 683 on testnet, in
neither case throughout it). The window is not an empty-tree region; it is merely
a region our bundle does not sample. **If a bundle refresh ever puts a row
there, that test fails and the threshold widens back to the activation height —
it is never retuned.** Over-including costs a rescan; under-including costs
wrong witnesses for real funds.

If a re-import turns out to be needed, it is **not a new mechanism**: it is
[ADR-0534](0534-rescan-from-earlier-birthday-recovery.md)'s `Wallet::rescan_from`,
which already rebuilds only the data DB from the already-sealed seed, preserves
the aux tables (queued sends, refund index, quotes, swap destinations) and holds
the lock throughout. What differs is the trigger — system-detected on first open
after the migration rather than user-invoked — and the reason: the anchor's
*frontier* is wrong rather than its height, so the same birthday re-imported
against a re-extracted 5-ary bundle row is the repair. A second rescan
implementation for this would be a duplicate, and duplicates are bugs.

**7. When a server will not state the Ironwood frontier, stop at the upgrade
height.** `TreeState::ironwood_tree()` returns an empty tree for an absent
field, without erroring, and `sync.rs`'s `to_chain_state()` calls it **per scan
batch** — so a lightwalletd that does not populate field 7 yields a wrong
per-batch anchor indefinitely, with nothing red. Measured 2026-09-07, the
endpoints we ship against (`zec.rocks`, `testnet.zec.rocks`, ECC LightWalletD
v0.5.3 / v0.5.4) do populate it at and above activation, so this is latent
rather than live; it bites a self-hosted or older server. **That measurement is
a one-time manual observation of somebody else's server, not a gate** — the
probe and its raw output are kept at `docs/plan/probes/` so it can be re-run,
but nothing regresses if an endpoint changes behaviour tomorrow, which is
exactly why the detector below is owed. The wallet scans
normally up to the activation and then **halts with a cause** — *"this server
can't serve blocks past the network upgrade; choose another server"*. Everything
already scanned stays valid and spendable, and nothing wrong is ever written.
Rejected: refusing the endpoint outright (discards the sync that would have been
correct, and fails before the user sees anything useful), and scanning on behind
a warning (**the damage is the scan** — wrong witnesses persist while the banner
is up, and the repair afterwards is a re-import, not a re-scan).

**State the detector's predicate on the RAW field, and only above the
activation.** The crate declares `ironwood_tree` as a plain `String`, not
`optional`, so *absent* and *empty* are the same value once decoded — the probe
above could tell them apart only because it declared its own field `optional`.
The implementable predicate is therefore `tree_state.ironwood_tree.is_empty()`
at a height **strictly above** the activation. A present-but-empty `"000000"` is
**not** a fault: it is the honest answer at every height before the pool's first
note, which is one height on mainnet and 683 on testnet. A detector written
against the *decoded* tree instead would fire on all of those and halt sync
against an honest server — the same shape of self-inflicted outage this ADR
exists to end.

## Alternatives considered

- **Amend ADR-0005 in place.** Rejected on the founder's rule (S254): an ADR may
  be edited while still uncommitted from the session that wrote it; once
  committed, a change needs a new ADR. ADR-0005 shipped 2026-06-05.
- **Bump only the crates that fail to compile.** Rejected: it is not expressible.
  The requirement chain (`zip321 0.9.0 → zcash_protocol ^0.10.5` is the tightest
  floor; `zcash_client_sqlite 0.22.0 → shardtree ^0.7` with `legacy-api`,
  non-optional) makes any subset either unresolvable or silently two-copy.
- **Move `zcash_script` to 0.5.2 "with the wave".** Rejected — see Decision 2.
  This was in an earlier draft of the plan, sourced from `cargo info` printing
  `(latest 0.5.2)`. It is the row that justified running the survey.
- **Fork or vendor the stack to control the break.** Rejected under ADR-0005 and
  Development Principle 2: consensus code is consumed whole. A fork means owning
  a consensus divergence at exactly the moment the chain changed rules.
- **Wait for a later wave that also carries NU7.** Rejected: NU7's coinholder
  vote opened 2026-08-25 and nothing is published. Sending has been broken since
  2026-07-28; the cost of waiting is measured in weeks of a wallet that cannot
  spend. The `Nu7` placeholder that `cfg(zcash_unstable="nu7")` exposes (branch
  id `0xffff_ffff`) stays forbidden in shipped builds by a `pins-policy` row.

## Consequences

- **Easier:** the SDK can sign for the live chain, the third frontier is
  modelled rather than implied, and the checkpoint bundle can finally carry
  `ironwoodTree` — the debt `CHECKPOINTS.md` has been recording.
- **Every wallet rescans on first open, whether or not we ask it to.** The
  `ironwood_shardtree` migration re-queues every scan range at or above the NU6.3
  activation at `Historic` priority. Every fleet wallet has a tip above 3,428,143,
  so that is ~46,000 blocks and climbing — a different order of magnitude from
  the schema migration, and a different user-visible symptom (a wallet that looks
  like it is resyncing from scratch). It is also the mechanism that recovers the
  six weeks of invisible Ironwood receipts, so it is load-bearing rather than
  incidental, and any repair we add on top must not schedule it twice.
- **Harder:** roughly twenty compile breaks across six files, and nine named
  tests that fail **on purpose**. A deleted row among them is a review blocker,
  not a cleanup. Several hardcode `NU6_3_BRANCH` as "the branch we do not know"
  and need re-fixturing onto a hypothetical future branch.
- **Frozen:** the pin set in Decision 1, the `zcash_script` freeze and its
  reason, the no-wildcard rule for pool identity, the anchor-threshold
  discipline in Decision 6, and the stop-at-the-upgrade-height posture in
  Decision 7. A change to any of them is a new ADR.
- **Not frozen, deliberately:** the exact `shardtree 0.7.x` patch level. The
  re-import threshold is frozen as a *rule* (the lowest bundled row at or above
  activation, enforced by the named test) rather than as a number, so a bundle
  refresh moves it without an ADR — in the widening direction only.
- **A behaviour change the pin set carries, easy to miss:** 0.22.0 stops
  *synthesizing* a status request for every unmined transaction.
  `transaction_data_requests` now reads durable `tx_retrieval_queue` rows only,
  and the `tx_status_observation_intent` migration never inserts one — it
  deletes redundant rows and copies the rest. Any pre-existing transaction whose
  `mined_height` is NULL and which has no queue row of its own therefore stops
  being asked about at the moment the migration runs. That is upstream's intent
  (such transactions are observable by scanning) and it is exactly INC-009's
  shape, so Phase B owes a one-time backfill over the new
  `WalletWrite::queue_tx_status`. `docs/plan/ironwood-phase-b.md` §3 owns the
  detail and the test.
- **Carried forward, still open:** `zcash_transparent` has no in-tree KAT
  pinning consensus branch ids (the t-addr *encoding* half is covered by
  `kat_bip39_vector_seed_derives_pinned_refund_address`; the branch-id half has
  no equivalent). And INC-015 — the staleness detector matches the v5 header
  exactly while NU6.3 transactions are **v6** (`BranchId::Nu6_3 => TxVersion::V6`
  in `zcash_primitives-0.30.1`; `V6_VERSION_GROUP_ID = 0xD884_B698` in
  `zcash_protocol-0.10.5`, unconditional, **not** `nu7`-gated) — is fixed in
  this wave or it stays dead. Do not re-derive that against the pinned
  `zcash_primitives-0.28.0`, where v6 *is* `nu7`-gated; that is how one S254
  review angle reached the opposite answer.
