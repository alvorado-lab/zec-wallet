# 0579: Move the wallet to the NU7 crate wave, and decide how it learns the mainnet height

- **Status:** Accepted, 2026-10-21. Decisions 5 to 8 are the founder's rulings of
  2026-10-21, in session. Three took the recommended option; on the release numbers
  the founder gave 0.0.4 to the TLS fixes, where the draft recommended it for NU7. The
  badge clause in Decision 6 is the orchestrator's call within that ruling, as the plan
  recommended. **Decision 4's signing-time half is extended by
  [ADR-0580](0580-wallet-send-pauses-in-the-last-twenty-blocks-before-a-network-upgrade.md),
  2026-10-30** (sends paused in the last 20 blocks before an activation; the branch
  clause stands). Every other decision here stands.
- **Date:** 2026-10-21 (drafted, reviewed in five angles and confirmed the same day,
  `docs/plan/nu7-wave.md` §10)
- **Links:**
  - **Supersedes** [ADR-0540](0540-ironwood-nu63-crate-wave-and-sealed-orchard-pool.md)
    Decision 2 whole: the `zcash_script` freeze and the `sapling-crypto`, `zip32` and
    `secp256k1` holds it carries (ADR-0540:91-97).
  - **Replaces** ADR-0540's Decision 1 table as the record of the pin set.
  - **Corrects** the claim in ADR-0540's Alternatives at `:262-263` (Decision 2
    below).
  - **Amends ADR-0005 by reference.** Its status line names ADR-0540 as the carrier
    of the current pins (ADR-0005:9-12).
  - **Extends**
    [ADR-0556](0556-wallet-send-a-persisted-signed-transaction-is-a-delivery-obligation-and-fences-a-rescan.md):
    its "expired" is read through Decision 4.
  - **Relates to** the reorg-budget proposal on the `release-oct-2026` branch (commit
    `a3a211fb2`; Proposed, on hold) for the burial depth (Decision 6).
  - **Work order:** `docs/plan/nu7-wave.md`. **Evidence:** `docs/plan/nu7-survey.md`,
    cited as S0 to S8. **Work item:** `docs/ROADMAP.md:1470-1506`.
  - **Spec:** `docs/specs/ironwood-nu63-support.md` §8 gate 7 (`:895-921`) and §11.3
    item 2 (`:1344-1347`). **Rules:** the project's cryptography rules (audited
    libraries used whole, no custom cryptography, a generator from the operating system).
  - **ZIPs:** ZIP 259 (https://zips.z.cash/zip-0259), ZIP 218
    (https://zips.z.cash/zip-0218), ZIP 2003 (https://zips.z.cash/zip-2003).
  - **Upstream:** librustzcash #3171 and #3161, zips #1405, zebra #11830 (URLs in S0
    and S4).

## Context

NU7 activates on mainnet on 2026-11-05 (S0,
https://forum.zcashcommunity.com/t/nu7-timeline/57655). It has been active on testnet
since 2026-10-06 at 4,465,026, under consensus branch `0x77190AD9` (ZIP 259, S0). It
adds no transaction format and no pool. Version 4 transactions become invalid,
versions 5 and 6 stay valid, and a signature commits to the branch it was made under
(ZIP 259, S0). ZIP 218 brings 25-second blocks and a 120-block default expiry delta
after the activation. It also asks implementations to support rolling back "at least
600 blocks after activation" (S0; ZIP 218's constants table).

The last network upgrade is why this is urgent. On 2026-07-28 the pinned crates did
not know NU6.3, and every transaction the SDK signed was rejected for six weeks
(ADR-0540, Context). The `consensus.rs` gate now refuses to sign when a server reports
a branch the build does not expect at the judged height (`consensus.rs:610-631`;
`docs/ROADMAP.md:1479-1483`). It can only expect what its params know, which matters
below (Decision 5).

What upstream has published (S1.1, S1.2):

- `zcash_protocol 0.11.0-pre.1` and a matching set of pre-releases, dated 2026-10-14,
  each exact-pinning the others. They carry `Nu7` without the `zcash_unstable` flag,
  the real branch id, `TxVersion::V6` for `Nu7`, and the testnet height. The mainnet
  height is `None` (`zcash_protocol-0.11.0-pre.1/src/consensus.rs:549`, its
  `CHANGELOG.md:72`).
- The mainnet height, 3,543,405, is proposed in zips #1405 and zebra #11830, both open
  on 2026-10-21. librustzcash #3171 is the work item to put it in `zcash_protocol`, and
  says "Do not assume or hard-code a height before it is announced" (S0).
- Upstream `main` is already ahead of the pre-releases (#3142, #3144, #3169; S1.4).

A trial compile stopped at 181 distinct type errors, 153 of them one cause: our
`WalletDb` generator is `rand_core 0.6.4`'s `OsRng`, which is not a `rand_core 0.10`
generator (S6).

Four things in the wave are decisions rather than mechanics: the pin set, the frozen
holds ADR-0540 wrote, where the operating system's randomness now comes from, and how
the wallet judges a signed transaction the activation has killed (S1.1, S4). Four more
were the founder's, and he ruled on them on 2026-10-21 (Decisions 5 to 8):

- the height policy if no crate carries the mainnet height in time;
- the burial depth after the activation;
- pre-release crates in a published release;
- the release numbers.

## Decision

**1. The pin set.**

The wallet moves to the NU7 pre-release lines in one atomic commit. The commit
covers:

- `sdk/Cargo.toml`, `sdk/zec-wallet-core/Cargo.toml` and the root `Cargo.toml`;
- three locks: `sdk/Cargo.lock` and the two fuzz locks,
  `sdk/zec-wallet-core/fuzz/Cargo.lock` and `sdk/zec-wallet-swap-near/fuzz/Cargo.lock`.

ADR-0540 Decision 3 (one commit; the policed root mirrors move with it) holds
unchanged.

The **live** pin set is `sdk/Cargo.toml` as `pins_policy` holds it; this table is the
**record** at acceptance. The versions are S1.1's and S6's, each re-read from the
crates.io versions API for existence, yank status and `created_at` before it is
written (`docs/plan/nu7-wave.md` §3 step 1).

| Crate | From | To | Manifest |
|---|---|---|---|
| `zcash_protocol` | 0.10.5 | **0.11.0-pre.1** | `sdk/Cargo.toml:236` and root `Cargo.toml:209` |
| `zcash_address` | 0.13.0 | **0.14.0-pre.2** | `sdk/Cargo.toml:211` and root `Cargo.toml:208` |
| `zip321` | 0.9.0 | **0.10.0-pre.2** | `sdk/Cargo.toml:272` and root `Cargo.toml:216` |
| `zcash_client_backend` | 0.24.0 | **0.25.0-pre.2** | `sdk/Cargo.toml:120` |
| `zcash_client_sqlite` | 0.22.0 | **0.23.0-pre.2** | `sdk/Cargo.toml:111` |
| `zcash_keys` | 0.16.1 | **0.17.0-pre.2** | `sdk/Cargo.toml:235` |
| `zcash_transparent` | 0.10.0 | **0.11.0-pre.2** | `sdk/Cargo.toml:248` |
| `orchard` | 0.15.5 | **0.16.0** | `sdk/Cargo.toml:133` |
| `sapling-crypto` | 0.7.0 | **0.9.0** | `sdk/Cargo.toml:132` (Decision 2) |
| `shardtree` | 0.7.1 | **0.8.0** | `sdk/Cargo.toml:143` |
| `incrementalmerkletree` (dev) | 0.8.2 | **0.9.0** | `sdk/Cargo.toml:75` |
| `zip32` | 0.2.1 | **0.3.0** | `sdk/Cargo.toml:271` (Decision 2) |
| `zcash_script` | 0.4.5 | **0.6.0** | `sdk/Cargo.toml:264` (Decision 2) |
| `zcash_primitives` | 0.30.1 | **0.31.0-pre.2** | `sdk/zec-wallet-core/Cargo.toml:67` |
| `zcash_proofs` | 0.30.0 | **0.31.0-pre.2** | `sdk/zec-wallet-core/Cargo.toml:118` |
| `getrandom` (new direct) | none | **0.4.2**, `default-features = false`, `features = ["sys_rng"]` | `sdk/Cargo.toml` `[workspace.dependencies]`, and the core manifest (Decision 3) |

**Entering or moving in the lock with the set, and reviewed with it** (each read from
the new manifests):

- the proving and note-encryption crates:
  - `halo2_proofs` 0.3.5 to the 0.4 line, and `halo2_gadgets` 0.5.0 to 0.6, both via
    `orchard 0.16.0`;
  - `bellman` 0.14.0 to 0.15, and `groth16` 0.2, which is new, both via
    `sapling-crypto 0.9.0` and `zcash_proofs 0.31.0-pre.2`;
  - `zcash_note_encryption` 0.4.2 to 0.5.1;
- `pczt =0.10.0-pre.2` and `zcash_pool_migration =0.2.0-pre.2`;
- `secp256k1` 0.29 to 0.33, and `bip32 0.6.0` (Decision 2);
- `rand` and `rand_core` 0.10, transitively;
- `chacha20poly1305 0.11.0` beside our seed seal's `0.10.1` (S1.1).

**What is frozen is the lines, not the suffixes.** Step 7's re-pin to the final
releases of these same lines needs no new ADR. A pin outside these lines needs one,
and so does a release that renumbers a line, or a change to any other decision here.

**2. ADR-0540 Decision 2 is superseded whole.**

That decision held `zcash_script` at `=0.4.5`, and `sapling-crypto =0.7.0`,
`zip32 =0.2.1` and `secp256k1 0.29` with it (ADR-0540:91-97). Its Consequences list
"the `zcash_script` freeze and its reason" as frozen. All four move in this wave.

The freeze's reason was that `0.5.2` predates `0.4.5` and reverts the `sig_op_count`
consensus fix. That reason does not carry over to `0.6.0`:

- `0.6.0` is dated 2026-10-01, after `0.4.5` (2026-05-29) (S1.1 point 1);
- it carries `0.4.5`'s test module (`zcash_script-0.6.0/src/script/iter.rs:141`),
  including the `0xab`-then-fifty-`0xae` case that must count 1000 (`:161-165`);
- the four crates that required `^0.4.3` now each require `0.6`
  (`[dependencies.zcash_script]` in `zcash_transparent-0.11.0-pre.2/Cargo.toml:126`,
  `zcash_primitives-0.31.0-pre.2/Cargo.toml:185`,
  `zcash_client_backend-0.25.0-pre.2/Cargo.toml:445` and
  `zcash_client_sqlite-0.23.0-pre.2/Cargo.toml:329`).

ADR-0540's rule survives and is applied: semver order is not date order, so read
`created_at`. `0.6.0` ships no changelog (S1.1 point 1), so step 1's review reads the
`0.4.5` to `0.6.0` source diff for consensus changes.

**ADR-0540's claim at `:262-263` is wrong, and this wave makes it true.** It says the
`Nu7` placeholder "stays forbidden in shipped builds by a `pins-policy` row". No such
row was written. There is no `zcash_unstable` in `sdk/zec-wallet-core/tests/` or
`src/`, and no `sdk/.cargo/config.toml` (S2; re-checked 2026-10-21). The Ironwood
spec asked for it (`docs/specs/ironwood-nu63-support.md:1344-1347`).

This wave adds `pins_policy::no_forbidden_cfg_reaches_a_shipped_build`. It reads one
array of forbidden cfgs: `zcash_unstable`, which `nutachyon` now needs (S2), and
`getrandom_backend` (Decision 3) (`docs/plan/nu7-wave.md` §3 step 1g).

ADR-0540's status line links here for its Decision 1 table and its Decision 2, as of
2026-10-21. The `:262-263` claim sits in its Alternatives, not in a decision, so the
back-annotation rule gives it no status-line link (`docs/adr/README.md:13-19`). This
paragraph is its correction.

**3. The operating system's randomness comes from `getrandom`'s `SysRng`, declared
directly and made infallible with the `rand_core 0.10` that `getrandom` re-exports.**

Every generator we hand upstream now needs a `rand_core 0.10` `Rng` or `CryptoRng`:

- the `WalletDb` parameter (`db.rs:92`, `:766`);
- the new `rng` argument of `create_proposed_transactions`, which supplies "the
  randomness used to construct, prove, and sign each transaction"
  (`zcash_client_backend-0.25.0-pre.2/src/data_api/wallet.rs:1431`, `:1451-1452`).

`rand_core 0.10` has no dependencies and no `OsRng` (S1.1 point 2).

**The source is `getrandom::SysRng`**, a zero-sized type over the operating system's
generator that implements `TryRng` and `TryCryptoRng`
(`getrandom-0.4.2/src/sys_rng.rs`, identical in 0.4.3).

- **The shape.** We pin `getrandom =0.4.2`, the version already in `sdk/Cargo.lock`,
  with only `sys_rng`, and spell the seam `getrandom::{SysRng, rand_core::UnwrapErr}`.
  Under that feature, `getrandom` re-exports exactly those two names
  (`getrandom-0.4.2/src/lib.rs:31-34`).
- **Infallible.** `UnwrapErr` gives the infallible `Rng` and `CryptoRng` the bounds ask
  for (`rand_core-0.10.1/src/unwrap_err.rs:30-55`), with `CryptoRng` blanket-implemented
  for any `TryCryptoRng<Error = Infallible>` (`rand_core-0.10.1/src/lib.rs:95-97`).
- **Upstream's spelling.** It is the same type upstream uses for these two parameters:
  `UnwrapErr(SysRng)` in `zcash_client_sqlite-0.23.0-pre.2/src/wallet/init.rs:321-333`,
  where upstream reaches it through `rand`.
- **Why not `rand`.** `rand 0.10` exports `SmallRng` and the `Xoshiro` generators
  whatever its features (`rand-0.10.3/src/rngs/mod.rs:106-108`). With `rand` as a
  direct dependency, "no user-space generator is nameable from our crate" could only
  be held by a deny-list. With `getrandom` direct, `rand` is not nameable at all, and a
  manifest row keeps it that way (`docs/plan/nu7-wave.md` §3 step 1g).

**The risk this guards, stated as it is.** A predictable stream on this path discloses
the spend authorizing key, for two reasons:

- the RedDSA signing nonce is hashed from generator bytes
  (`reddsa-0.6.1/src/signing_key.rs:159-169`);
- the key randomizer `alpha` is drawn from the same generator
  (`orchard-0.16.0/src/builder.rs:449`, `sapling-crypto-0.9.0/src/builder.rs:259`).

An observer who can predict the stream can therefore compute the nonce, then the
randomized key, then the key.

**Against the project's cryptography rules** (named in Links; each rule is quoted or
named here, since the rules file is not published):

- **Rule Zero: audited libraries are used whole.** We write no generator and no
  adapter. It is the same project's successor to what our `OsRng` calls today:
  `rand_core 0.6.4`'s `getrandom` feature (`sdk/Cargo.toml:151`) calls `getrandom`
  0.2. `getrandom` 0.4.2 is already in the lock (its dependents are `tempfile 3.27.0`
  and `uuid 1.23.3`). Whether `getrandom` 0.4 has a published third-party audit was
  not established; step 1's supply-chain review owns that question.
- **"Key generation → `OsRng` only. Never `thread_rng()`."** `SysRng` is the
  operating system's generator, the rule's `OsRng` under its new name
  (`rand-0.10.3/src/rngs/mod.rs:18-22`).
- **No silent replacement of the source.** `getrandom` lets a `--cfg
  getrandom_backend="custom"` swap the operating system's generator for any function
  (`getrandom-0.4.2/src/backends.rs:11`). `getrandom_backend` is therefore on Decision
  2's forbidden array.
- **Failure is loud, as today, and pays nothing twice.**
  - `UnwrapErr` panics if the operating system cannot supply entropy
    (`rand_core-0.10.1/src/unwrap_err.rs:51-53`), as today's `OsRng` does
    (`rand_core-0.6.4/src/os.rs:63`).
  - The generator is drawn only inside the locked create, so a panic poisons the
    shared mutexes and no second signature can follow (`wallet.rs:3352-3356`,
    `runtime.rs:7-16`).
  - The SDK keeps `panic = "unwind"`: no profile in `sdk/Cargo.toml` sets `panic`.
- **No `unsafe` in our crate, nothing to zeroize.** `#![forbid(unsafe_code)]` stays
  (`zec-wallet-core/src/lib.rs:26`), as the rules require of every cryptography crate.
  `SysRng` holds no state, so there is no key material in the generator for the rules'
  zeroization checklist to apply to.

**Scope.** Only the generators that reach a `rand_core 0.10` bound move to the seam.
Our other uses of `rand_core 0.6`'s `OsRng` and of `chacha20poly1305`'s re-exported
`OsRng` do not change: the seed seal, the mnemonic entropy, isolation tokens and
jitter (58 code lines across 16 files of `zec-wallet-core/src`, counted 2026-10-21).

All generators reach upstream through one seam (`docs/plan/nu7-wave.md` §3 steps 2a
and 2b).

**4. One kernel judges whether a signed transaction can still be mined.**

A transaction signed under branch `B` can be mined only in blocks whose branch is `B`
(ZIP 259, "its signatures commit to a consensus branch ID"; S4).

- **The kernel** is one pure function over the transaction's header bytes, its stored
  expiry and the params. Its answer is one of three:

  | Answer | Meaning |
  |---|---|
  | `Never` | stored expiry 0 and no next activation |
  | `Through(h)` | the last height at which it can be mined |
  | `Nowhere` | the next activation is at height 0 or 1, where `H - 1` would floor at 0 (`zcash_protocol-0.11.0-pre.1/src/consensus.rs:120-126`) and 0 would read as "never expires" |
- **`H`** is the upper bound of `B`'s own range,
  `BranchId::try_from(branch).ok()?.height_bounds(params)?.1`
  (`zcash_protocol-0.11.0-pre.1/src/consensus.rs:904-922`).
  - Any `None` leaves the stored expiry standing.
  - `H` is never read from the hand-kept `KNOWN_UPGRADES`, whose job is the anchor
    ceiling.
  - Given `H`, the answer is `Through(H - 1)` for a stored 0, and
    `Through(min(expiry, H - 1))` otherwise.
  - `Through(h)` is past expiry when `h <= tip`, the same edge as today's
    `past_expiry` (`send.rs:1993`).
  - `height_bounds` never yields `H = 0`, and yields `H = 1` only when a lower bound
    is 0, so `Nowhere` is reached through an inner function that takes `H` directly,
    which is where it is tested.
  - On mainnet under the pre-release, a `Nu6_3` transaction has no `H` and keeps its
    stored expiry; a `Nu6_2` one gets `Nu6_3`'s activation. Under the final crates, a
    `Nu6_3` transaction gets the crate's height, 3,543,405 if the proposal holds.
- **The branch is the transaction's own**, read by one pair-checked header reader
  (bytes 8 to 12 of a whole v5 or v6 header, INC-007's rule;
  `docs/specs/ironwood-nu63-support.md:1397`). It is split out of
  `unknown_branch_evidence`, which returns `None` for every branch the crate knows
  (`enhance.rs:477`) and so cannot serve. An unreadable header never shortens an
  expiry.
- **Five readers, all through the kernel, none keeping its own comparison:**
  1. `send::rebroadcast_group` (`send.rs:2048`);
  2. `delivery::enumerate` (`delivery.rs:318`);
  3. `DeliveryView::state_of` (`delivery.rs:248`), which today reads only
     `raw IS NOT NULL` (`delivery.rs:369-379`);
  4. the history page, whose `Expired` is upstream's view column `expired_unmined`
     (`history.rs:43-47`, `:603-604`, `:658-668`). "Cancelled by upgrade" wins over it;
  5. `enhance::chain_status` (`enhance.rs:322-344`). It bounds a server's "mined at h"
     by the stored expiry (`:340`), which would let a hostile server confirm a payment
     the upgrade killed.

  A source gate over all of `src/` allows only the kernel to compare an expiry with a
  tip.
- **The signing-time half.** `assert_proposal_anchor_fresh` (`send.rs:1186-1203`) also
  returns `ProposalStale` when
  `BranchId::for_height(params, target) != BranchId::for_height(params, tip + 1)`.
  Without it, a proposal made before `H - 1` and signed after is built under the dead
  branch, because upstream builds at the proposal's target
  (`zcash_client_backend-0.25.0-pre.2/src/data_api/wallet.rs:1997-2001`). Its residual
  is the one its doc already states for drift: it reads the recorded tip, so a resume
  that signs before the first sync still signs under the dead branch
  (`send.rs:1183-1186`). The kernel then marks that transaction dead once the wallet
  has synced.
- **The rest of the rule:**
  - From the tip `H - 1` such a transaction is **rebuilt, never re-broadcast** (S4).
  - The irreversible steps **wait for irreversible burial of the effective expiry**,
    through the predicate of Decision 6.
  - **Nothing new rebuilds anything**: an intent requeues, a group strands, a deposit
    ends through its hold and purge (S4).
- **What the wallet says** is true at each moment:
  - It carries both the height the network stopped accepting the transaction and the
    height its funds are released, which is the stored expiry (`send.rs:1514-1518`).
  - It carries a phase the core computes: not final, final but not released, or
    released. Funds are back only at the later of release and burial: for an ordinary
    send, the engine frees the notes at most 40 blocks past the activation, before
    burial.
  - It does not say the amount is "still yours to spend"
    (`zec_wallet_ui/lib/l10n/wallet_en.arb:194`), or strike the amount through, before
    the released phase.
  - It does not invite a re-send before burial.
  - A live re-send reads `RetryPending`, with the expired explanation, never "tries
    again until it goes through" (`wallet_en.arb:182`).

The engine keeps the notes reserved until the stored expiry passes: at most 40 blocks
past the activation for an ordinary send under the pre-release, and up to 69,120 for a
canonical crossing (S4). librustzcash #3161's `expire_unmined_transaction` is the
upstream way to release them early (`docs/plan/nu7-wave.md` §3 step 4).

**5. If no crate carries the mainnet height in time, the wallet ships on the
pre-releases, with a refusal-only date tripwire and a server gate (ruled by the
founder, 2026-10-21).**

The wallet never puts the mainnet height into its own consensus parameters. Until a
`zcash_protocol` release carries it, every build is on the pre-releases. Three things
make that honest.

**What a pre-release build does after the activation, without this decision.** The
gate expects `BranchId::for_height(params, judged)` (`consensus.rs:610-631`). On
mainnet under the pre-release, that is `Nu6_3` at every height, because `Nu7` has no
mainnet height. So the build behaves according to the server:

| Server | What the build does |
|---|---|
| On an NU7 node, reporting `0x77190AD9` | refuses every signature with the update copy |
| Still on the old branch, reporting `37a5165b` (ZIP 259 lets such nodes stay online, S0) | reads `Current` and signs `Nu6_3` transactions the network rejects: each payment shows as sending and never arrives |
| Withholding its branch | reads `Unknown` inside the grace, and for up to 8 hours (1,152 blocks at 25 s; the surface shows about 24 hours) signs `Nu6_3` transactions the network rejects, which show as sending and never arrive. Then it refuses with the "switch servers" copy (`ConsensusGraceExpired`, `docs/specs/ironwood-nu63-support.md:653`), never the update copy |

The plan's step 5 changes none of this on mainnet under the pre-release, because its
spacing function sees no NU7 height there. 0.0.2 behaves the same way.

**The date tripwire.** It turns the second and third rows into the update refusal.

- **What it does:** it refuses only, with the update copy. It fires on mainnet when
  the params (the plan's `WalletParams`) know `Nu7` without a height and the device
  clock is at or past the stated date.
- **When it is evaluated:** at signing time, against an absolute clock read then. That
  reading is a new input to the gate, as the grace's clock is
  (`docs/specs/ironwood-nu63-support.md:438-441`).
- **The latch:** once observed, it stays tripped, so a clock set back does not
  re-permit, as the grace's latch works (`:710-738`).
- **What it reads:** `WalletParams`. When the params carry a height it is inert, so
  the final crates turn it off by themselves.
- **The date:** 2026-11-05 00:00 UTC for now, the forum's day and ahead of the
  arithmetic's roughly 2026-11-06 01:00 UTC (S0). It is re-derived at the 2026-10-30
  checkpoint from the published height.
- **Its removal:** a tripwire test fails once the crate knows the height, which
  forces the removal at step 7.
- **Its costs, accepted:**
  - It refuses for about a day before the activation, and longer if the activation
    slips.
  - A device clock that is behind the date never trips it.
  - It is the kind of date-based detector the spec rejected
    (`docs/specs/ironwood-nu63-support.md:226`): "a magic number that silently becomes
    wrong". That applies in full. It is accepted because when it is wrong, it is wrong
    in the refusing direction, early if the activation slips, and it never changes what
    is signed. It stands beside the branch check rather than replacing it, it is
    re-derived at the checkpoint, and a test removes it.

**The server gate.** It makes the second and third rows a release blocker for the
servers Relim passes (`docs/plan/nu7-wave.md` §4):

- **The test:** every such server reports a non-empty `consensus_branch_id`, and a
  node version (`zcashd_build` / `zcashd_subversion` in `GetLightdInfo`) at or above
  the first release of that node carrying the NU7 mainnet height.
- **When it runs:** once that node release exists, at the 2026-10-30 checkpoint, and
  before the tester build.
- **The record:** a probe with one row per server.
- **The consequence:** no tester build ships with a failing server on its list.
- **Today:** no server can pass. zebra #11830 is open, and `zec.rocks` mainnet ran
  Zebra 6.3.0 on 2026-10-21 (S7).
- **The residual:** a server the user adds by hand is outside the gate. Only the
  tripwire covers it.

**What a user sees under this decision.**

- Until the tripwire's date, about a day before the activation, nothing changes on
  mainnet, and testnet works again.
- From that date, every signature is refused with the update copy (the tripwire, and
  the gate against an NU7 server after the activation), until a build on the final
  crates arrives.
- A send signed in the last 40 blocks reads as waiting until its own expiry, about 17
  minutes at 25 s (S3), because the kernel has no mainnet `H`.
- A canonical crossing caught across the activation reads as waiting for weeks.
- Decision 6's burial counts 200 blocks after the real activation, about 83 minutes,
  until step 7.
- Updating to a build on the final crates fixes all of it.

**What Relim's testers see.** Their build follows `../zec-wallet`'s working tree
(`app/relim-flutter/pubspec.yaml:93-101` on Relim `main` at `2c85bd654`). They get
exactly this until a final-crate build reaches their devices.

**Its build step:** the plan's 3a. It pins the refusal against an NU7 server, the
documented limit against a stale or a silent one, and the tripwire.

**6. The burial depth after the activation is 15,000 s of chain time, through a new
predicate that replaces `buried` (ruled by the founder, 2026-10-21).**

- **The rule:** irreversible burial waits for 15,000 s of chain time, ZIP 218's
  wall-clock target. Through the spacing function, that is 200 blocks at 75 s and 600
  at 25 s, summed across a straddle, and never below 100.
- **The facts it answers:**
  - zcashd quotes 99;
  - Zebra has kept 1000 blocks before and after the activation since v5.2.0;
  - ZIP 218 asks for at least 600 after the activation;
  - upstream `PRUNING_DEPTH` is 100;
  - the reorg-budget proposal on the `release-oct-2026` branch (commit `a3a211fb2`)
    found that the wallet already follows reorgs deeper than 100.
- **The predicate:** `irreversibly_buried(params, mined, tip)` replaces
  `stranded::buried`, which is left with no production caller.
  - Every irreversible step follows it: delete-on-final, strand, requeue (and through
    it the swap deposit's purge), reap, and the reclaim margin. So does the stranded
    weld.
  - The plan's consumer table lists every consumer, those that follow and those that
    keep 100 (`docs/plan/nu7-wave.md` §3 step 4).
- **`REORG_MAX_BLOCKS` stays 100** for the keep-list: the grace clamp, the new-wallet
  birthday lag, the scan cap, the bind's reorg window and `tip_reference`. That same
  reorg-budget proposal would pin the constant at or below `PRUNING_DEPTH`, and the
  predicate is kept separate from it for that reason.
- **The finality badge follows the predicate.** This is the orchestrator's call within
  the founder's ruling, as the plan recommended. The badge takes the core's mined
  height and depth, or its `is_final` is welded into `TxStatus::Confirmed`, never a
  tip from the host. No screen reads the badge today. The source pin
  `no_finality_consumer_reads_reorg_max_blocks` therefore allows nothing.
- **What it changes, accepted:**
  - from the day the plan's step 4 lands, every requeue, delete, reap and reclaim on
    mainnet waits 200 blocks (about 4.2 hours) instead of 100 (about 2 hours);
  - the reclaim margin goes from 141 to 241 before the activation;
  - after the activation, a cancelled ordinary send is sent again, and shows its amount
    back, about 4.2 hours after the upgrade, or about 83 minutes under Decision 5 until
    step 7.

**7. Pre-release crates are never published to pub.dev (ruled by the founder,
2026-10-21).**

The published package carries `sdk/Cargo.lock` verbatim (`scripts/release/stage-pub.py:14`,
`:235`; `docs/handoff/release-checklist.md:100-110`), so publishing would put the locked
pre-releases in every pub.dev host. NU7 is published only on the final crates. Relim's
testers are not governed by this rule, because their build follows the working tree
(Decision 5).

**8. The release numbers: 0.0.4 is the TLS security release, and NU7 is 0.0.5 (ruled
by the founder, 2026-10-21).**

- **0.0.4 ships soon, as a security release.** It carries all three security fixes of
  2026-10-21, which the published 0.0.1 and 0.0.2 lack (`docs/ROADMAP.md:1440-1469`):
  - `b1717a17c`, the TLS roots past the emSign distrust;
  - `6a69c6aa7`, rustls 0.23.46;
  - `9b17ce3e6`, the swap never resuming a TLS session across swaps, a security MEDIUM.
    The published package vendors the swap crate (`scripts/release/stage-pub.py:15-16`,
    `:55`), so the fix reaches hosts only through a release.

  None of the three is an ancestor of the held 0.0.3.
- **0.0.5 is NU7**, on the final crates (Decision 7).
- **The held docs-only 0.0.3** (`release-oct-2026` at `b084e8e9b`, 2026-10-09) is
  published first or set aside. It is never re-cut under the same number, because that
  cut's subject calls its cuts append-only.

## Alternatives considered

- **Wait for the final release before pinning anything.** Rejected: it leaves the
  compile sweep, the kernel and the ZIP 218 work to the last days, and testnet stays
  refused until then.
- **Build against upstream `main` by git revision.** Rejected: three relevant changes
  merged there on 2026-10-19 and 2026-10-20 (S1.4), and none is a release.
- **Keep the ADR-0540 holds.** Not expressible: four crates in the set require
  `zcash_script 0.6` (Decision 2), and the new `orchard`, `sapling-crypto` and `zip32`
  lines are what the set requires (S1.1).
- **Keep `rand`, upstream's shape.** Rejected for the reason in Decision 3: it makes
  user-space generators nameable from our crate.
- **A `TryCryptoRng` adapter over `rand_core 0.6.4`'s `OsRng`.** Rejected: our own
  generator glue on the signing path, against Rule Zero.
- **`rand::rng()` or a seeded user-space generator.** Rejected by the cryptography
  rules' key-generation rule, quoted in Decision 3.
- **Read `H` from `KNOWN_UPGRADES`.** Rejected: a hand-kept list whose job is the
  anchor ceiling. Upstream's `height_bounds` is the crate's own ordering, so a list
  that falls behind cannot shorten or lengthen an expiry.
- **Judge expiry separately at each site, or allow `enhance.rs` its own comparison.**
  Rejected: one kernel. The review found a fifth reader that a per-site rule had left
  open to a hostile confirmation (`docs/plan/nu7-wave.md` §10).
- **Cap the expiry at build time instead** (zcashd's behaviour, through
  `create_proposed_transactions`' `expiry_height`). Rejected as the mechanism:
  - it cannot fix transactions signed by a build without `H`;
  - upstream refuses a caller-supplied expiry on a canonical crossing (`send.rs:621`,
    `:1302-1309`).

  If the plan later adopts it for ordinary sends, it calls this kernel.
- **Keep re-broadcasting after `H - 1`.** Rejected: ZIP 259 asks wallets to "clearly
  report pending transactions that become invalid" (S0).
- **Option B: put 3,543,405 into our own consensus parameters ahead of the crate.**
  Not chosen by the founder (2026-10-21).
  - **What it would have been:** a custom `Parameters` type answering as
    `Network::MainNetwork` except `Nu7`, behind the plan's `WalletParams` alias. It
    would have carried sending across the activation with no update, if the number was
    right.
  - **Why not:**
    - it is a second source for a consensus constant, which the Ironwood spec's gate 7
      exists to prevent;
    - it goes against upstream's "Do not assume or hard-code a height before it is
      announced";
    - if the number is earlier than the real height, it reads a still-mineable payment
      as cancelled (inviting a manual re-send), stops retrying a payment that could
      still go through, re-sends sooner than the reorg margin allows, and can pay twice
      through a canonical crossing (`send.rs:2116-2125`).
  - The plan keeps its step 7B as a recorded, not-built option.
- **Option A without the tripwire.** Not chosen: against a stale or a silent server
  after the activation, payments would show as sending and never arrive (Decision 5's
  table).
- **Publish the pre-release crates to pub.dev** (in a 0.0.3 or later). Not chosen
  (Decision 7): for a mainnet host after the activation they buy little that 0.0.2
  lacks, and a yank upstream would turn our own audit gate red (`sdk/deny.toml:30`,
  `yanked = "deny"`).
- **Release NU7 as 0.0.4, which the draft recommended, or as 0.0.3.** Not chosen
  (Decision 8): the founder gave 0.0.4 to the security fixes of 2026-10-21, which
  should not wait for NU7, and 0.0.3 names a held cut that is never re-cut.
- **Keep the burial at 100, or take Zebra's 1,000.** Not chosen (Decision 6).
  - 100 is about 42 minutes after the activation. A reorg ZIP 218 says nodes should
    support could then let both the original and the re-send go through.
  - 1,000 is about 20.8 hours before the activation and 6.9 hours after, for every
    irreversible step.
- **Keep the badge at 100.** Not chosen (Decision 6): a badge that says "final" before
  the wallet's own steps treat the money as irreversible claims more than the wallet
  does.

## Consequences

- **Easier:**
  - testnet works again, and NU7 transactions parse (S2);
  - the wallet can say a payment was cancelled by the upgrade, and when its amount
    returns;
  - a hostile confirmation of a killed payment is refused;
  - after the activation, a pre-release build refuses with the update copy rather than
    sending payments that never arrive;
  - the gate-0 carve's stated reason is gone from the lock (S1.1 point 3).
- **Harder:**
  - at least 181 compile errors (S6), and a second sweep at step 7 against a release
    that removes `DEFAULT_TX_EXPIRY_DELTA` (S1.4);
  - `BRIDGE_ABI_VERSION` moves twice on this line: 12 for the new error kind, and 13
    for the new `TxStatus` reading. A shipped version is never re-pointed
    (`zec_wallet/rust/src/bridge_abi.rs:103-104`). The swap branch, which already holds
    12, rebases past 13 before any of its builds ships (`docs/plan/nu7-wave.md` §6);
  - a time-boxed spacing function of ours until upstream's ships;
  - irreversible steps on mainnet wait about twice as long from the day step 4 lands
    (Decision 6);
  - a day of refusal before the activation if the final crates are late (Decision 5).
- **Frozen:**
  - the lines in Decision 1;
  - Decision 2's supersession and its reasoning;
  - Decision 3's source, its manifest shape, its forbidden cfg and its one seam;
  - Decision 4's kernel: one function, `H` from `height_bounds`, the three answers,
    the transaction's own branch, five readers, the signing-time clause, rebuild never
    re-broadcast;
  - Decision 5's policy: no height of our own, the tripwire's shape (refusal only,
    evaluated at signing, latched, reading `WalletParams`), and the server gate;
  - Decision 6's rule and its predicate;
  - Decisions 7 and 8.

  A change to any of them is a new ADR.
- **Not frozen, deliberately:**
  - the pre-release suffixes, which step 7 replaces;
  - the spacing function, which step 7 deletes in favour of upstream's;
  - the tripwire's date, which the 2026-10-30 checkpoint re-derives.
- **Done in the commit that accepts this ADR:**
  - **ADR-0540's status line** links here for its Decision 1 table and its Decision 2.
    The `:262-263` claim in its Alternatives gets no link (Decision 2).
  - **ADR-0005's status line** points here as the record of the current pins.
  - **The Ironwood spec's pointer** (`docs/specs/ironwood-nu63-support.md:1070-1081`)
    names the live source, `sdk/Cargo.toml` with `pins_policy`, with this ADR as the
    record. Its old source-of-truth sentence and its `zcash_script` paragraph
    (`:1106-1118`) carry dated notes that this ADR supersedes them.
  - **The project's cryptography rules:** their path list covers the seam's file and
    `send.rs`.
  - **The ROADMAP's stale lines** (`docs/plan/nu7-wave.md` §3 step 0), with the
    founder's rulings.
- **Still owed:** the key-generation rule's wording naming `SysRng` beside `OsRng`,
  when the seam lands.
- **Done ahead of this ADR:** `PUBLIC_HEIGHTS` (`scripts/release/export-public-docs.py`)
  gains `4_465_026` and `3_543_405`, committed with the work order, because this ADR is
  exported and the export refuses an unlisted mainnet-range height.
