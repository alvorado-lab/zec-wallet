# Spec — Ironwood / NU6.3 support (wallet track)

**Status:** Reviewed — 2026-09-06 (S252 draft; S253 consistency audit + four-angle
review round folded). The draft's *facts* survived; several of its *designs* did
not. Every change below is attributed in §12.
**Implements:** [ADR-0005](../adr/0005-zcash-wallet-librustzcash-direct.md)
(librustzcash-direct — we consume the upstream stack, we do not reimplement
consensus) · [ADR-0013](../adr/0013-wallet-as-extraction-ready-flutter-sdk.md)
(extraction-ready SDK — the host is told, in types, what the SDK can no longer
do) · [ADR-0540](../adr/0540-ironwood-nu63-crate-wave-and-sealed-orchard-pool.md)
(the Ironwood pin set, the sealed-pool posture, and the anchor re-import rule —
**the record for all three**; this spec links, it does not restate) ·
PRODUCT_VISION § ZEC module · Development Principle 6 (honest degradation) and
10 (no silent failures).
**Relates to:** `wallet-sdk.md` §3.2f (bundled tree-state, the trusted birthday
anchor — **this spec patches it**, see §1.2), §3.6 (checkpoint bundle), §4.6 M2
(poisoned-checkpoint class), §8 (the SDK's named-test register — this spec's
Gate rows fold in there when they ship),
`sdk/zec-wallet-core/CHECKPOINTS.md` (the `ironwoodTree` debt this closes).
**Does NOT own:** the `mined_height = NULL` defect. It was root-caused during
this review, it is not Ironwood-caused, and it now has its own ROADMAP row.
§11.1 records the finding and points there.

---

## 0. The incident this spec exists because of

A restored, fully-synced mainnet wallet attempted a small self-send. The core
logged:

```
broadcast_persisted{txs=1}: wallet.send accepted=0 rejected=1 outcome="rejected"
```

The transaction is absent from mainnet and from the mempool. Two *earlier*
sends from the same wallet, mined before the activation below, are confirmed
on chain.

The dividing line is **block 3,428,143, 2026-07-28** — the activation of
**Ironwood / NU6.3**.

**RESOLVED 2026-09-07 (Phase B, S256) — this paragraph describes the outage, in
the past tense from here on.** `zcash_protocol` is now pinned at `=0.10.5` and
`NetworkUpgrade::Nu6_3` is in `consensus::KNOWN_UPGRADES`, so
`BranchId::for_height()` returns `Nu6_3` at post-activation heights and the SDK
signs against the right branch.

What the outage WAS: the pinned `zcash_protocol =0.9.0` knew network upgrades
only through **Nu6_2 (mainnet 3,364,600)** and contained no `Nu6_3` at all. For
every height past activation, `BranchId::for_height()` returned `Nu6_2` while
the network was on `Nu6_3` (`0x37a5_165b`). Every transaction the SDK built was
signed against the wrong consensus branch, so its signature hash was invalid and
**every node rejected it** — for six weeks, silently.

*(All four constants re-verified against `zcash_protocol` 0.10.5 source during
the S253 audit: `BranchId::Nu6_3 = 0x37a5_165b`, mainnet activation 3,428,143,
testnet activation 4,134,000, `Nu6_2` mainnet 3,364,600.)*

There is a second, independent blocker beneath it. Ironwood sealed the Orchard
pool: per Shielded Labs, *"users will no longer be able to send or receive ZEC
inside the Orchard pool"*, and value leaves only through the ZIP-318 turnstile
into the new Ironwood pool. A wallet holding Orchard funds therefore cannot
construct a spendable transaction at all under the old rules, branch ID aside.
**The consensus rule is narrower than that sentence, and the difference matters
— [ADR-0540](../adr/0540-ironwood-nu63-crate-wave-and-sealed-orchard-pool.md)
Decision 5 is the record for it and this spec does not restate it.** In short:
Orchard change may return to Orchard on a transaction that *spends* Orchard
notes, if strictly less value returns than the notes remove. That exception is
what lets an Orchard note and an Ironwood note share one transaction, which is
what makes the mis-tagged-note-claim hazard reachable.

**Every Orchard-funded wallet on this SDK has been unable to send since
2026-07-28, and nothing in the product said so.** That silence is the part this
spec treats as the primary defect.

**The outage is two-sided, and the receive half went unrecorded until S255
(INC-016).** Ironwood outputs ride `CompactTx.ironwood_actions` — proto tag 9,
new in `zcash_client_backend 0.24.0`, absent from the pinned 0.23.0 — and `prost`
skips unknown fields. So a pre-bump build scans a post-activation block, finds
nothing, marks the range *scanned*, and reports 100%. Upstream is explicit about
what that costs (`zcash_client_sqlite 0.22.0`, `v_address_uses_ironwood.rs`):
*"After NU6.3 every payment to an Orchard receiver is delivered in the Ironwood
bundle, which makes an Ironwood-only receipt the ordinary case rather than an
exotic one."* Nothing is lost — upstream's own `ironwood_shardtree` migration
re-queues from the activation, so the bump recovers it — but six weeks of
receipts are on chain and invisible, and **every review of this outage before
S255, this spec included, recorded only the send half.**

**A third consequence, found in review and live right now:** every v5
transaction mined since activation carries `0x37a5_165b` in its header, and
`Transaction::read` resolves that field through `BranchId::try_from`
(`zcash_primitives-0.28.0` `transaction/mod.rs:1026`), which errors on a branch
the pinned crate does not know. So `parse_and_validate` returns `None`,
the enhancement loop counts `rejected` and `continue`s, and the data request is
never cleared. **Memos on every post-activation receive are silently lost and
their transactions are re-fetched on every sync pass, indefinitely.** §1.5 and
§9.1 own this.

---

## 1. Design decisions

### 1.1 The problem, split in two

This is deliberately **two problems**, and the smaller one is the more
important:

- **P1 — the SDK cannot transact on today's network.** Fixed by adopting the
  upstream Ironwood release and implementing ZIP-318 migration. Large, and
  strictly a catch-up.
- **P2 — the SDK could not TELL anyone that.** It signed transactions the
  network was guaranteed to reject, surfaced `accepted=0 rejected=1` with no
  cause, and left the user reading "Sent to the Zcash network — waiting to be
  confirmed" over a transaction the network had refused.

P2 is the standing defect. P1 recurs at **every** future network upgrade; P2 is
what decides whether the next one is a diagnosis session or a status message.
**P2 ships first and independently of P1**, and its correctness does not depend
on Ironwood support existing.

### 1.2 The guard we already had, why it could never fire, and the clause nobody built

`provision::endpoint_network_matches` performs an endpoint network-match:

```rust
fn endpoint_network_matches(network: Network, id: &ServerIdentity) -> bool {
    id.chain_name == checkpoints::chain_name(network)
        && id.sapling_activation_height == u64::from(checkpoints::activation_height(network))
}
```

Both compared fields are **immutable historical constants** — `"main"` and
`419200`. They match on every network upgrade that will ever happen. The guard
detects a mainnet wallet pointed at a testnet endpoint and is *structurally
incapable* of detecting upgrade drift.

**This was not an oversight in design — it is an unimplemented clause of the
authority spec.** `wallet-sdk.md` §3.2f already said the network-match is
*"`get_lightd_info` **chain name / consensus-branch** must match the configured
`Network`"*, its §8 register row described
`provision_rejects_network_mismatched_endpoint` as *"the `get_lightd_info`
**chain/branch** guard"*, and its gate-6 row described
`network_mismatch_rejected_everywhere` as covering a
*"consensus-branch/activation-height check"*. The branch half was specified in
three places and built in none, so the test register had been asserting a guard
we do not have. **Phase A corrects all three in the same commit** — leaving them
is a two-docs-one-mechanism bug.

Meanwhile `LightdInfo` — which we already fetch, in full, on every
provisioning handshake — carries **`consensus_branch_id`** (a hex `String`,
proto tag 5, which we drop on the floor in `ServerIdentity`) and `block_height`
(tag 7). **The server has been telling us the answer since 28 July and we
discarded it.**

Two independent faults, both fixed here:

1. **Wrong fields.** Compare something that actually moves.
2. **Wrong timing.** The check is not "provisioning-only" as the draft claimed;
   it had two production call sites — `resolve_birthday` step 1 and
   `run_engine_pass`. But the second was gated by
   `if wallet.never_scanned()` inside `run_engine_pass`, so **it stopped
   running the moment the first block scanned.** A wallet provisioned in June
   keeps running through an upgrade in July and never re-checks. The staleness
   predicate must be evaluated on the **live tip**, on a recurring basis, not
   once at birth. Both call sites move to the new predicate; neither is left
   behind as a second implementation.

### 1.3 The decision that matters most: never adopt the server's branch — *and never its height either*

The tempting design — read `consensus_branch_id` from the server and sign with
it — is **rejected, and this rejection is load-bearing.**

The endpoint is untrusted (§4.6, "every byte from the network is hostile"). A
server that can choose the branch ID we sign under can:

- make us produce transactions valid on a **fork of its choosing** while we
  believe we are on mainnet, and
- do it silently, because a correctly-signed transaction for the wrong chain
  looks locally identical to a correct one.

**The draft stated this invariant too narrowly, and the narrow version is
already violated by shipped code.** Signing does read compiled params — but it
reads them *at an index the server chooses*. The chain, traced end to end during
review:

```
endpoint get_latest_block().height
  → sync::fetch_tip                        (wallet.rs:3692)
  → sync::record_chain_tip → update_chain_tip
  → scan_queue.block_range_end
  → chain_tip_height                       (zcash_client_sqlite-0.21.0 wallet.rs:3070)
  → mempool_height = chain_tip_height + 1  (…:3082)
  → proposal.min_target_height() = target_height
  → BranchId::for_height(&params, target_height)   (zcash_primitives-0.28.0 builder.rs:497)
  → the sighash branch
```

So the correct invariant is:

> The branch we sign under is `BranchId::for_height(compiled_params, h)` where
> `h` is a height **we** validated. No endpoint-supplied value is an input to
> transaction construction — **not as a branch, and not as an index into our
> own params.** The server's `consensus_branch_id` is used for exactly one
> thing: as a **stop signal**. It can cause us to refuse to sign; it can never
> cause us to sign differently.

This matters concretely. A hostile endpoint that *under-reports* its tip to
3,400,000 and reports `consensus_branch_id = Nu6_2` is internally consistent:
the naive predicate says `Current`, and we sign under `Nu6_2` on a `Nu6_3`
chain. The comparison must therefore be made across **both** heights — our own
`block_max_scanned` and the endpoint's claimed chain tip — and must return
`Unsupported` when they straddle an activation boundary our params know about
(§3.2).

This keeps the failure mode of a lying server "the wallet refuses to send"
(annoying, safe) rather than "the wallet signs for an attacker's chain"
(catastrophic).

### 1.4 Alternatives considered

| Option | Verdict |
|---|---|
| Do nothing; pin crates forever | **Rejected.** The product is already broken on mainnet. Pinning is why we did not notice. |
| Upgrade the crates, skip staleness detection | **Rejected.** Fixes today, guarantees an identical silent outage at NU7 — for which a coinholder vote already opened 2026-08-25. |
| Implement Ironwood ourselves | **Rejected — violates Development Principle 2** (audited crypto libraries whole; no custom crypto). Consensus and circuits come from upstream. |
| Detect staleness from our own tip vs. a hardcoded "known-good until" date | **Rejected.** A date is a magic number that silently becomes wrong; and it cannot distinguish "we are stale" from "this endpoint is on a different chain". |
| Trust the server's branch ID for signing | **Rejected** — §1.3. |
| Refuse only the interactive send verb | **Rejected** — §1.5. The gate goes at the signing choke point, which covers four verbs, not one. |

### 1.5 What a stale wallet is still allowed to do

Refusing everything is dishonest in the other direction: a stale wallet can
still *see* most of its money truthfully, and locking users out of their own
balance is its own harm. But the draft's table was too generous in three rows,
and it omitted the one capability that **writes** chain state.

| Capability | Stale behaviour | Why |
|---|---|---|
| View history | **Allowed, QUALIFIED (corrected S255, INC-016)** | Reading historical *branch* data is unaffected — but a post-activation receipt into the Ironwood bundle is not in the history at all, because the scanner never saw it (§0). History is complete for what this build can decode and silently short by every Ironwood receipt. |
| View balance | **Allowed, QUALIFIED** | Ironwood value is unmodelable by a stale binary, so the figure is a floor, not a total. Rendered as "at least X — this version can't see newer funds", never as a confident total. |
| Sync / scan | **Allowed, qualified — see §6.4** | Blocks past activation contain data we cannot model; we scan what we can and never report `Up to date` over them. |
| Receive (show address) | **Allowed to SHOW; the arrival is INVISIBLE — corrected S255, INC-016** | Addresses are not branch-scoped, so handing one out is safe and the funds do land on chain and stay recoverable. **The rest of this row's original reasoning was wrong and it is why the receive half of the outage went unrecorded for six weeks:** it said our UA carries an Orchard *and* a Sapling receiver, so "a modern sender falls back to Sapling — no note lands in a pool we cannot model." A modern sender does not fall back. It pays the Orchard receiver, and after NU6.3 *"every payment to an Orchard receiver is delivered in the Ironwood bundle"* (upstream, `v_address_uses_ironwood.rs`) — which rides `CompactTx.ironwood_actions`, a proto field this build does not have. The note lands in exactly the pool we cannot model, the scan reports clean, and nothing shows. Recovered by the Phase B bump, not by anything Phase A can do. |
| **Enhancement (memo fetch)** | **Degraded, BOUNDED, surfaced** | Post-activation v5 transactions cannot be parsed under pinned params (§0), so memos on them are unavailable until Phase B. The retry must be bounded rather than re-attempted every pass forever, and the unknown-branch parse failure is recorded as *local staleness evidence* (§4). |
| **Provisioning (create / restore)** | **Allowed, but the anchor is CEILINGED** | This is a **write**, and the state it writes is immutable after first import. The bundled birthday anchor never rides above the activation height of the newest upgrade THIS BUILD knows — **unconditionally, not only while `Unsupported`**, because a binary cannot know which upgrade it is missing. Derived from the compiled params (`consensus::newest_known_activation`), never a literal. **It does NOT rise on a crate bump by itself** — `newest_known_activation` reads `consensus::KNOWN_UPGRADES`, a HAND-MAINTAINED list, and after the S256 pin wave landed the ceiling is still `Nu6_2`'s 3,364,600. Raising it is Phase B step 6, which `known_upgrades_is_really_complete` forces. (An earlier version of this sentence said it rises by itself, copied from an over-broad code comment; the wave is the event that proved it wrong.) See §9.2 for the money argument and the cost. |
| Export viewing key, backup seed | **Allowed** | Recovery must never be blocked by staleness — this is the case where the user most needs their keys out. |
| **Send / shield / swap / migrate / sweep / reclaim** | **REFUSED, typed** | Anything that signs a transaction. See §3.2 for the enumerated sites. |

The refusal happens at the **signing choke point**, and additionally as an
early check in `propose()` so the user learns before a proposal is built. A user
must not be able to spend proving time and a fee slot on a transaction we know
cannot be valid — and, equally, must not reach a signature by any path that
skips `propose()` (§3.2).

---

## 2. Domain types

The draft declared two states and then used four in prose. All four are
declared here.

```rust
/// What the SDK knows about the network's consensus rules relative to what
/// THIS BINARY can produce.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConsensusCompatibility {
    /// Our compiled rules cover the live tip and the endpoint agrees.
    Current,

    /// The endpoint reports a consensus branch this binary does not know, or
    /// reports a branch that disagrees with what our params compute for the
    /// live tip. EITHER we are too old OR the endpoint is not on our chain —
    /// and the SDK CANNOT TELL THOSE APART. Both are fail-closed for signing.
    ///
    /// `endpoint_branch_id` is the endpoint's raw claim, retained for display
    /// and support diagnostics ONLY. It is never fed to transaction
    /// construction, and never used as an index into our own params (§1.3).
    Unsupported {
        /// The branch our compiled params compute for `judged_at_height`.
        expected_branch_id: u32,
        /// The endpoint's self-reported branch, as claimed. UNTRUSTED.
        endpoint_branch_id: Option<u32>,
        /// The height the judgement was made against (§3.2 picks it).
        judged_at_height: BlockHeight,
    },

    /// We are BEHIND, not stale: our scanned tip is far below the endpoint's
    /// claimed tip, and no upgrade our params know about is straddled between
    /// them. A current binary mid-restore is here, and it must not be told to
    /// update the app (§6.1). Signing follows the existing spend-before-sync
    /// rules — this state does NOT refuse.
    Behind { scanned_tip: BlockHeight, claimed_tip: BlockHeight },

    /// The endpoint did not supply a usable `consensus_branch_id`, so we could
    /// not check. NOT `Current`: "we cannot check" is never rendered as "we
    /// checked and it is fine" (the honest-off discipline).
    ///
    /// Signing follows the GRACE rule (§6.3, founder decisions 2026-09-06 and
    /// 2026-09-10): permitted while our own last SIGNING-CAPABLE verdict
    /// (`Current` or `Behind` — both checked the branch against a live
    /// endpoint) is recent on BOTH axes, refused once EITHER ages out.
    /// `blocks_since_last_current` is the distance the JUDGED height
    /// (`max(scanned, claimed)`) has advanced since that verdict — the
    /// endpoint still supplies `block_height` (tag 7) even when it withholds
    /// the branch; a height it under-reports below our own scan cannot shrink
    /// the count, and a frozen height cannot advance it — which is why the
    /// second axis exists. `clock` is that axis: seconds on the DEVICE clock
    /// since the same verdict, and the latch (GRACE-1).
    Unknown {
        judged_at_height: BlockHeight,
        /// `None` when there has never been a `Current` verdict for this
        /// wallet — which is outside the grace by construction.
        blocks_since_last_current: Option<u32>,
        /// The clock rule's input, filled at EVERY read of the stamp from the
        /// `WallClock` port: `elapsed_secs` (`None` when there is no capable
        /// time to measure from, or when the recorded one is later than now
        /// or pre-epoch — a time the clock cannot vouch for, which since
        /// GRACE-2 is latched EXPIRED, never abstained) and `latched` (the
        /// expiry has been observed since the last capable verdict; only a
        /// capable verdict clears it).
        clock: GraceClock,
    },
}

/// The cold-read answer (§3.3). The VERDICT is derived; this snapshot is the
/// small piece of state that lets us be honest while offline, and it is the
/// ONLY persisted part (§10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ConsensusStatusSnapshot {
    /// `None` before the first evaluation of this wallet's lifetime — a real
    /// state, not an error, and distinct from every verdict above (§6.2).
    pub verdict: Option<ConsensusCompatibility>,
    /// The height the last verdict that PERMITTED signing was judged at,
    /// bounded into `[scanned_tip, scanned_tip + REORG_MAX_BLOCKS]`
    /// (`consensus::grace_anchor`; T0-1c-R2 M4 added the lower bound after a
    /// stuck validator's low `block_height` was found lowering it) — the
    /// anchor the §6.3 grace is measured from.
    pub grace_anchor_tip: Option<BlockHeight>,
    /// Wall-clock seconds at evaluation, for the "and its age" contract in
    /// §6.2. Age is derived at read time; the snapshot stores the instant.
    pub evaluated_at_unix: Option<u64>,
}

/// The endpoint's self-reported network identity. EXTENDED (§1.2): the two
/// historical constants stay (they still catch a cross-network misconfig), and
/// the two fields that actually move are added.
pub(crate) struct ServerIdentity {
    pub(crate) chain_name: String,
    pub(crate) sapling_activation_height: u64,
    /// lightwalletd reports this as a hex STRING; parsed at the boundary,
    /// `None` when absent or unparseable (an old or non-conforming server).
    /// prost renders an absent tag-5 string as `""`, which is indistinguishable
    /// from present-and-empty — both are `None` here, and §6.3 owns what that
    /// means.
    pub(crate) consensus_branch_id: Option<u32>,
    pub(crate) block_height: u64,
}
```

New error, appended to the existing `RW-SYNC-*` family. **`RW-SYNC-002` is free
— re-derived from `error.rs:517`, which is the only `RW-SYNC-*` in the
registry.**

```rust
/// The network requires consensus rules this build does not implement — or the
/// endpoint is not on the chain we think it is. Signing is refused; reading is
/// not. `RW-SYNC-002`.
NetworkUpgradeUnsupported {
    expected_branch_id: u32,
    endpoint_branch_id: Option<u32>,
    judged_at_height: BlockHeight,
},
```

**Rationale for one error, not two** ("we're old" vs "wrong chain"): the SDK
genuinely cannot distinguish them from inside, and inventing two codes would
force the host to render a distinction we cannot justify. One code, with the
raw numbers attached for a human.

**Relationship to `NetworkMismatch` (`RW-STORE-002`).** That error stays exactly
as it is and keeps its meaning: a wrong-*chain* endpoint (testnet vs mainnet)
found at provisioning. The new error is the upgrade axis. `verify_endpoint_network`
raises the first and delegates the second to the shared predicate — one function,
two typed outcomes, no second implementation.
*(Housekeeping while in there: `wallet-sdk.md:2387` claims `WalletError::Sync`
covers `NetworkMismatch`; the code returns `NetworkMismatch` at
`provision.rs:168`. Correct the sentence.)*

---

## 3. Interface design

### 3.1 Outbound (what the service needs)

`ChainOracle` (existing) gains no new *method* — the data comes from the
`get_lightd_info` we already make. `server_identity()` simply stops discarding
two fields.

```rust
#[async_trait]
pub(crate) trait ChainOracle {
    async fn tip_height(&mut self) -> Result<u64, GrpcError>;
    async fn server_identity(&mut self) -> Result<ServerIdentity, GrpcError>;
}
```

**But this is NOT free, and the draft's claim that it was is withdrawn.** The
only per-pass reacher of `get_lightd_info` was gated behind
`if wallet.never_scanned()` in `run_engine_pass`, so nothing existing carried
the call once a wallet had scanned a block. Delivering the live-tip verdict §1.2
demands requires **a recurring unary `GetLightdInfo` on the sync pass**. That is
a real cost and a real new traffic pattern; §5 and §7 price it honestly. It is
one small unary call per pass, and the alternative — evaluating once per wallet
lifetime — is the exact fault this spec exists to fix.

### 3.2 The predicate — one home, and every site that calls it

```rust
/// THE single source of truth for "can this binary transact on this network".
/// Pure: (compiled params, our scanned tip, the endpoint's claim) -> verdict.
/// No IO, so it is unit-testable across every upgrade boundary without a
/// server.
///
/// Takes BOTH heights (§1.3): a verdict computed only at our own scanned tip
/// cannot see an upgrade we have not scanned up to yet, and a verdict computed
/// only at the endpoint's claimed tip is attacker-movable. `Unsupported` when
/// the two straddle an activation our params know; `Behind` when they differ
/// with no known upgrade between them.
pub(crate) fn consensus_compatibility<P: Parameters>(
    params: &P,
    scanned_tip: BlockHeight,
    claimed_tip: BlockHeight,
    endpoint: &ServerIdentity,
    /// From the persisted snapshot; consulted only by the `Unknown` arm (§6.3).
    /// Passed in rather than read here so the predicate stays pure.
    blocks_since_last_current: Option<u32>,
    /// The grace's clock half (GRACE-1), read by the caller from the persisted
    /// stamp against the `WallClock` port — the device clock enters as an
    /// INPUT, never a read, so the predicate stays pure.
    clock: GraceClock,
) -> ConsensusCompatibility;
```

**Both grace rules live on the verdict, and the ONE question reads them.**
`permits_signing()` takes no arguments: an `Unknown` carries the block distance
AND the clock reading, and every reader of the persisted stamp —
`consensus_stamp::observe(conn, now)`, with `now` from the `WallClock` port —
fills the clock half at THAT instant. So the send gate, the drain, the parked-row
flag, the sync surface and the cold read answer exactly as the pass does, and a
send a day after the last pass sees the day (§4p G-13 of the phase-1 plan).

**The judged height.** The verdict is taken at `max(scanned_tip, claimed_tip)` —
the highest height either side attests. An endpoint that under-reports its tip
therefore earns *more* scrutiny, never less, and one that over-reports is judged
against its own claim. `under_reported_tip_cannot_buy_a_current_verdict` pins it,
watched failing against a judge-at-the-claim mutant.

**The gate is a type, not a checklist.** `SigningPermit` is a token with a
private field, constructible only from a permitting verdict, and
`send::create_signed_core` takes it **by value**. Since every signature in the
SDK passes through that one function, the compiler — not a reviewer — enforces
that no signing path skips the check. Adding a fifth signing path fails to
compile until it asks.

Operating principle: *one source of truth for every cross-cutting predicate*. A
second implementation of "are we stale" is a bug.

**The refusal sites, enumerated — because "gate the signing paths" without a
list is how the draft shipped a gate with a hole in it.** The gate goes at the
shared choke point, not at each verb:

| Site | File | Gated how |
|---|---|---|
| `create_signed_core` | `send.rs` (definition; also reached from `prepare_queued`) | **THE gate.** Every signature in the SDK passes here; refusing here covers the interactive send, the queued drain (`wallet.rs:3235`), the sweep (`wallet.rs:5085`) and the reclaim-mint (`wallet.rs:5302`) in one edit. |
| `propose()` | `wallet.rs` propose path | Early refusal, so the user learns before proving time is spent. A UX pre-flight, **not** the gate. |
| `queue_send` | `wallet.rs:2085` | **Deliberately NOT gated.** It touches no network, and gating it would break offline-first: a user must be able to compose while offline. The verdict is applied at the drain, where the signature happens. |
| `verify_endpoint_network` | `provision.rs:162` | Extended, not duplicated (§2). |

**The week-old queued send.** A send composed offline, queued, and drained after
an upgrade is refused at the drain. That is correct and it needs a UX: the
intent stays `Queued` and cancellable, the row carries the typed
`NetworkUpgradeUnsupported` reason, and the host renders the §6.1 copy against
that row — never a silent park, never a retry loop that looks like progress.
This is a host-surface obligation of Phase A, not a follow-up.

### 3.3 Inbound (what the host sees)

```rust
/// Cold read, so a host can render an update prompt without attempting a send.
/// Honest when offline: returns the last evaluated verdict and its age, and
/// `verdict: None` when this wallet has never evaluated one. NEVER a
/// fabricated `Current`.
pub async fn consensus_status(&self) -> Result<ConsensusStatusSnapshot, WalletError>;
```

`async` + `Result` because it reads the persisted snapshot from the store (the
predicate is pure; the *snapshot* is stored state), so it carries the same store
faults every other read does.

The send path returns the typed refusal from `propose()` — before a proposal
exists — **and** from the signing choke point for every path that does not
pass through `propose()`. THREE refusals, three types (GRACE-1; §4p G-1 of the
phase-1 plan): `NetworkUpgradeUnsupported { .. }` (`RW-SYNC-002`) for
`Unsupported`; `ConsensusGraceExpired { by: Blocks | Clock | NeverConfirmed,
blocks_since_last_current }` (`RW-SYNC-003`) for an `Unknown` outside its
grace; `ConsensusNotEvaluated` (`RW-SYNC-004`) for a wallet that has never
evaluated a verdict. They render differently because their next steps differ
(§6.1), and the old shape — every refusal as `NetworkUpgradeUnsupported`, the
never-evaluated one with `expected_branch_id: 0` — rendered "update the app"
for a server that had merely stopped talking.

The cold read carries the grace to the sync surface as well: after every clean
pass the engine reads the stamp against the clock
(`SyncEnginePort::unknown_branch_grace`) and the controller publishes
`SyncStatus::UpToDateUnverified { tip, grace, pools }` for an `Unknown` verdict
— running (blocks left, and seconds left on whichever rule expires first) or
ended (why). Remaining counts and a reason cross the bridge; the absolute
capable timestamp is a signing input and never leaves the core.

### 3.4 Would this work with a different adapter?

Yes. The predicate takes a plain `ServerIdentity`, two heights and consensus
`Parameters`; it has no knowledge of gRPC, lightwalletd, or Tor. A Zaino
adapter, a local `zebrad`, or a test fake satisfies it identically.

---

## 4. Security

| Surface | Threat | Mitigation |
|---|---|---|
| `consensus_branch_id` from the endpoint | A malicious server supplies a branch to make us sign for its fork | **§1.3** — the claim can only *stop* signing, never shape it. Signing reads compiled params exclusively. |
| Same field | A malicious server claims an unknown branch to **deny service** | Accepted, and strictly preferable to the alternative. Denial is visible, loud, and recoverable by switching endpoints; a mis-signed transaction is none of those. Surfaced with the endpoint named so the user can change it. |
| Endpoint omits the field (old lightwalletd, or a MITM stripping it) | Silent downgrade to "no check" | `None` is `Unknown`, never `Current` (§2), and `Unknown` **degrades to refusal** once EITHER the judged height advances `UNKNOWN_BRANCH_GRACE_BLOCKS` past our last signing-capable verdict OR the device clock advances `UNKNOWN_BRANCH_GRACE_SECS` past it (§6.3). Stripping the field buys an adversary at most one day of silence on either axis — **unless it also holds the device clock still**, the residual row below. **Corrected 2026-09-10 (GRACE-1):** this row said lying about the tip "only shortens" the window — backwards. A server that FREEZES its claimed tip held the block count at zero for as long as it liked; the clock rule is the axis it cannot freeze *from the endpoint alone*. |
| Same field, plus a **frozen claimed tip** | Hold the block grace at zero forever | The clock rule (§6.3): a day on the device clock since the last capable verdict ends the grace whatever the tip did. A clock set forward ends it early (fail-closed; a capable pass restores it); a clock set back AFTER the expiry was observed changes nothing (the latch — only a capable verdict clears it). **Corrected 2026-09-10 (GRACE-2, §4v):** this row said a clock set back BEFORE the capable time "reads as no time passed and blocks decide alone" — the block rule is exactly what this row's frozen tip defeats, so one capable pass under a wrong clock (RTC garbage at boot, a date later corrected, a pre-epoch clock) left the grace with no expiry on either axis. A recorded capable time the clock cannot vouch for — later than now, or pre-epoch — now EXPIRES the clock rule, latched, fail-closed; a capable pass restores it and re-records the time. |
| Same field, a frozen claimed tip, **and a clock held STILL** (an attacker who also controls the device's unauthenticated NTP source) | Hold BOTH axes at zero indefinitely | **OPEN — no mitigation, and stated rather than implied** (§4v-run review row 1; the founder's decision 2026-09-11 is to document this and not build a third time axis now). The untrusted test is "later than now, or `0`", so a clock pinned anywhere inside `[capable_at, capable_at + UNKNOWN_BRANCH_GRACE_SECS − 1]` reads as trusted, the elapsed count never reaches the threshold and the latch is never written: `permits_signing` stays true for as long as the attacker keeps both dials still. This is the §4n-review row 1 attack surviving in the stopped-clock cell, and it is why the "at most one day of silence" row above carries its exception. What would close it is a THIRD axis neither the endpoint nor the wall clock can freeze — monotonic uptime accumulated across passes, or a high-water reading latched when the clock fails to advance over N passes. Scope of the exposure: the wallet signs using COMPILED consensus params (§1.3), so the damage is a transaction built for rules the chain may have left, never a transaction shaped by the attacker. |
| Same field, plus a **`block_height` that does not fit a `u32`** | End the grace durably with one unvalidated number | The claim is UNUSABLE (GRACE-2, §4v G2-5): the pass judges at the height this wallet scanned, so the grace's block count, the verdict's judged height and the anchor are all unmoved by it; the discard is logged as a static code. The earlier saturation to `u32::MAX` persisted `judged_height = u32::MAX` on the `Unknown` row and every cold read rebuilt the count from it — a remote, unauthenticated, one-pass spend-denial. An IN-range claim far above our scan still advances the count in one pass (an honest server ahead of our scan should; a hostile one ends the grace by blocks a pass early, fail-closed, and the next honest pass recomputes it). |
| **`block_height` from the endpoint** | **A lying tip moves the branch boundary — and it moves the branch we SIGN with, not just the verdict** | The draft's mitigation was circular (it compared two values both descended from the endpoint's tip). The predicate now takes **both** our scanned tip and the claimed tip and refuses when they straddle a known activation (§3.2). The residual — an endpoint that under-reports the tip to a wallet that has not scanned past activation either — is closed by the local evidence row below. |
| **Local staleness evidence** *(SUPPRESSION-resistant, NOT forge-resistant — corrected S267)* | An endpoint can lie about its branch, and a stale build cannot contradict it from the branch check alone (pinned by `a_lying_endpoint_can_hide_our_own_staleness_from_this_predicate`) | A mined transaction naming a branch we do not implement is **local proof** the chain is past our params — a server can withhold it only by refusing to serve transactions at all. **It CAN forge it, and this row used to say otherwise.** The twelve bytes are read from the raw reply AFTER `parse_and_validate` — the one call that binds a reply to the requested txid — has already failed, so nothing binds them: a hostile endpoint answers any `GetTransaction` with a well-formed v5/v6 header naming an id this build lacks and gets a false staleness warn plus a `UnparseableTxids` entry for a txid of its choosing (that txid's memo or mined status then does not land for the rest of the process). Pre-existing — the v5-only reader had the same hole — and INC-007 closes the ACCIDENTAL case (a v4 transaction's input vector at these offsets), never the deliberate one. Treat the counter as a diagnostic an endpoint can drive. `enhance::unknown_branch_evidence` reads it from a **v5 or v6** header and `EnhancementOutcome::unknown_branch` counts it, the txid joins `UnparseableTxids` and a `wallet.consensus` warn names the branch. **LIVE since T0-4 (2026-09-11); INERT before it (INC-015, found S254, now GUARDED).** The reader used to match the v5 header exactly, and post-Ironwood transactions are **v6** (`zcash_primitives-0.30.1` maps `BranchId::Nu6_3 => TxVersion::V6`; `TxVersion::read` accepts the pair `(V6_TX_VERSION, V6_VERSION_GROUP_ID) = (6, 0xD884_B698)` unconditionally) — so it matched nothing from activation on 2026-07-28 until the fix: the counter was structurally zero, the warn never fired, and §9.1 step 7's re-download bound never engaged. Its tests hand-built a v5 header, confirming the assumption rather than checking it, which is why the guard is now a **real mined mainnet v6 transaction** (`staleness_proof::a_real_mined_ironwood_transaction_is_recognised_as_v6_and_is_not_evidence`, height 3,478,152) asserted BOTH ways: unmodified it is not evidence, and with the branch field overwritten by an id this build provably lacks it is. The v5 arm stays — a deep restore still meets pre-Ironwood transactions, and the question is about the BRANCH, not the format. **Still a diagnostic and not yet a mitigation**: feeding the evidence into the consensus verdict (§9.1 step 7's owed half) is a decision-shaped change — it makes the wallet REFUSE — and has its own contract. |
| Migration (ZIP-318) | Orchard→Ironwood moves real value | Consumed **whole** from the upstream **`zcash_pool_migration`** crate (Principle 2). We write no circuit, no note-selection, no turnstile arithmetic. |

**Crypto posture — scoped, because the unscoped version misleads.**
**Phase A** adds **no cryptography**: a comparison of integers and a plumbing
change. **Phases B and C do.** Phase B bumps `orchard`, `zcash_primitives` and
`zcash_client_sqlite` — circuit, proving and note-commitment code, plus one-way
schema migrations over a funded wallet — and Phase C moves real value across a
turnstile. **Phases B and C each get the crypto-change review workflow and the
multi-angle review; Phase A does not need it** (§9.4).

---

## 5. Privacy & metadata

- **Nothing new leaves the device.** The check is a re-read of an existing
  `GetLightdInfo` response shape.
- **There IS a new traffic pattern, and the draft was wrong to deny it.** The
  live-tip verdict needs a recurring unary `GetLightdInfo` on the sync pass
  (§3.1). It is one small unary call alongside calls the pass already makes, to
  the same endpoint, over the same channel — it reveals nothing the pass does
  not already reveal — but it is a *new regular request* and is documented as
  one. Cadence: once per sync pass, not on its own timer (no new wake-up).
- **Logging (§5.4):** log the verdict and the two branch IDs — they are public
  protocol constants and carry no user data. **Never** log the endpoint URL
  alongside a wallet identifier, txids, amounts, addresses, or memo bytes.

The shape as built (`wallet::evaluate_consensus`, target `wallet.consensus`):

```
wallet.consensus  permits_signing=false verdict=Unsupported { expected_branch_id: .., endpoint_branch_id: .., judged_at_height: .. } scanned=3473803 claimed=3473805
```

and, from the enhancement pass, the local staleness evidence (§4):

```
wallet.consensus  endpoint_branch=0x37a5165b  "a mined transaction names a consensus branch this build does not implement"
```

Every field name is in the `tracing_guard` allowlist, which is what forces a
§5.4 review before a new one can ship.

---

## 6. Error handling & degradation

### 6.1 The refusal, as the user sees it

Errors show next steps, never codes (Principle 6). **Two surfaces, because the
capability table (§1.5) says reads stay live:**

- **The send fault** — the existing two-tier convention: the shared title
  `walletSendFailedTitle` ("Couldn't complete payment") plus a new body key in
  the `walletSendFault*` family. The draft's bespoke title is dropped; it broke
  a convention every other fault follows.
- **The persistent state** — the sync badge's own `UpToDateLimited` headline +
  sheet explanation (§6.4), since "this build can't send until it's updated"
  outlives one sheet. NOT a `walletStall*` entry: that family is driven by
  `StallReason`, and consensus staleness is not a sync stall — sync works, it
  is the READING that is limited. A `walletStall*` string was written and cut
  for exactly that reason (§9.1 step 9).

Body copy (both surfaces share the sentence) — **for `Unsupported` ONLY**
(GRACE-1; §4p G-6 of the phase-1 plan):

> The Zcash network was upgraded and this app needs an update before it can
> send. Your funds are safe and you can still receive.

**An `Unknown` outside its grace gets its OWN copy, and it never says
"upgraded" or "update the app"** — nothing is known to have changed, and an
update fixes nothing. The sync surface (`UpToDateUnverified`), the send fault
(`ConsensusGraceExpired`) and the parked row (`SigningBlock::GraceExpired`)
share one sentence per reason:

> *(by blocks)* This server hasn't reported the network version for N blocks,
> so this app can't confirm it's safe to send. Switch to another server.
>
> *(by the device clock)* This server hasn't reported the network version for a
> day, so this app can't confirm it's safe to send. Switch to another server,
> or check this device's date and time.
>
> *(never confirmed)* This server has never reported the network version, so
> this app can't confirm it's safe to send. Switch to another server.

and, while the grace still RUNS, the sync surface says so with the time left
on whichever rule expires first ("Sending still works for about N more hours —
then switch servers"; blocks alone when the device clock cannot be trusted for
the time). A wallet that has NEVER evaluated a verdict (`ConsensusNotEvaluated`)
renders the existing not-synced-yet story: the first completed pass resolves
it, and "update the app" would be as false there as for the silent server.

**It must never fire for `Behind`.** A wallet mid-restore is not stale, and
telling a user with a current binary to update the app is an unactionable
instruction. `Behind` renders in the existing catching-up vocabulary
(`walletSendFaultInsufficientCatchingUp` is the established sibling for the
"still syncing" story).

Balance, history and receive stay live behind it — **live but incomplete, and the
copy must not promise otherwise (corrected S255, INC-016):** balance is a floor,
history is short by every Ironwood receipt, and an incoming payment to the UA's
Orchard receiver arrives on chain and shows nowhere until the Phase B bump.
`RW-SYNC-002` is available in a diagnostics/support surface, not on the main
path.

### 6.2 Enumerated cases

| Case | Verdict | Behaviour |
|---|---|---|
| Endpoint branch ≠ ours, both known | `Unsupported` | Refuse signing, §6.1 copy, reads stay live **but incomplete — §1.5, INC-016** |
| Endpoint branch unknown to us | `Unsupported` | Same |
| Our scanned tip and the claimed tip straddle a known activation | `Unsupported` | Same — this is the hostile-under-reported-tip case (§1.3) |
| Scanned tip below claimed tip, no known upgrade between | `Behind` | **Does not refuse.** Existing spend-before-sync rules apply |
| Endpoint omits / blanks the branch ID, last `Current` within grace on BOTH rules | `Unknown` | **Signing allowed**; the sync surface shows `UpToDateUnverified { grace: Running }` — the server stopped reporting the network version, and how much grace is left (§6.3) |
| Endpoint omits / blanks the branch ID, grace exhausted on EITHER rule, or never `Current` | `Unknown` | **Refuse signing** with `ConsensusGraceExpired { by }` (`RW-SYNC-003`); the surface shows `Ended { by }`; §6.1's grace copy naming the server as the cause and the next step — never the update copy |
| Offline, snapshot exists | last verdict + age, the clock rule read against NOW | Send follows the existing offline-queue rules **only if** the last verdict permits (an `Unknown` is re-read against the device clock at the send: the day passes offline too); composing and queueing always work (§3.2) |
| Offline, **no snapshot** (cold start, never evaluated) | `verdict: None` | **Queueing: allowed.** **Signing: refused** with `ConsensusNotEvaluated` (`RW-SYNC-004`) until a verdict exists — rendered as not-synced-yet, never as an upgrade. A fresh install that has never reached a server has never confirmed it can transact, and fabricating `Current` is the §0 failure. The user can still compose; the drain applies the verdict when connectivity returns |
| We are stale AND offline | `Unsupported` from the snapshot | Refuse; the last verdict was already `Unsupported` |
| Endpoint on a different chain | `NetworkMismatch` (RW-STORE-002) | Unchanged, provisioning-time (§2) |

### 6.3 The `None` case — the grace rule (**founder decision, 2026-09-06**)

An endpoint that reports no `consensus_branch_id` yields **`Unknown`**, a
reported state distinct from `Current`. It is surfaced in diagnostics and
logged, so "we cannot check" is never silently rendered as "we checked and it is
fine".

**`Unknown` degrades with age**: signing is permitted while our own last
`Current` verdict is recent, and refused once it ages out. The draft's
pass-through was rejected because it lets anyone who strips one protocol field
turn Phase A off entirely; blanket refusal was rejected because the size of the
honest-but-quiet server population is asserted, never measured. The grace keeps
an honest quiet server usable and bounds what a hostile silent one can buy.

**The grace has TWO rules and ends when EITHER expires** (amended 2026-09-10 —
GRACE-1, founder decision; `production-readiness-phase-1.md` §4p). The primary
rule is measured in BLOCKS, because the risk being bounded is "an upgrade may
have activated since we last checked", and upgrades happen at heights. But the
blocks counted are the ones the SERVER admits: a server that omits the branch
AND freezes its claimed tip advanced the count by zero on every pass, and —
with the anchor floored at our own scanned height (T0-1c-R2 M4) and the wallet
unable to scan past what the server serves — held the grace at zero for as long
as it liked (§4n-review row 1 of the phase-1 plan). The second rule is the axis
it cannot freeze: the DEVICE CLOCK.

```rust
/// How far the JUDGED height (max(scanned, claimed)) may advance past our last
/// signing-capable verdict before an `Unknown` endpoint stops being allowed to
/// sign. ~1 day at the post-Blossom 75-second target (1,152 blocks). A day is
/// short enough that a server withholding the branch can extend the §0 silence
/// by at most a day, and long enough that an honest quiet server does not break
/// a user mid-session.
pub const UNKNOWN_BRANCH_GRACE_BLOCKS: u32 = 1_152;

/// The post-Blossom target block spacing — the ONE conversion between the two
/// rules, and what the sync surface uses to say which expires first.
pub const TARGET_BLOCK_SPACING_SECS: u64 = 75;

/// The clock rule: seconds on the device clock since the last signing-capable
/// verdict. DERIVED — the block grace's own wall-clock equivalent (1,152 × 75 s
/// = 86,400 s, one day exactly), so "a day" has one source.
pub const UNKNOWN_BRANCH_GRACE_SECS: u64 =
    UNKNOWN_BRANCH_GRACE_BLOCKS as u64 * TARGET_BLOCK_SPACING_SECS;
```

Both are exclusive at the threshold. The stamp (`consensus_stamp`) carries the
capable verdict's TIME forward exactly as it carries `capable_tip` — a silent
server's passes refresh the latest-verdict time, never the capable one — and
every reader computes `now − capable_at` from the `WallClock` port at the
instant of the read.

**A clock can only SHORTEN the grace; a time the wallet cannot trust ENDS it:**

- **A capable time the clock cannot vouch for is an EXPIRED clock rule**
  (GRACE-2, `production-readiness-phase-1.md` §4v — the founder's decision
  2026-09-10, item 1). A recorded capable time LATER than now (the clock moved
  back after the capable pass, or was ahead during it), or a capable time of
  `0` (a pre-epoch clock at the pass), is not a time the rule can measure
  from: the first reader to see it writes the same durable latch a day does,
  with its own duration-free warn line, and the reading is "no elapsed time,
  latched" — never negative, never "a full day left". The surface shows the
  existing clock ending (`walletSyncGraceEndedClock`: switch servers, or check
  the device's date and time — no new string); the send gate refuses
  `ConsensusGraceExpired { by: Clock }`; one capable pass restores signing and
  re-records the time from the clock as it reads then. GRACE-1 let this cell
  ABSTAIN — "the block rule alone decides" — and the block rule is exactly
  what a frozen-tip silent server defeats (§4p-run review row 2): one capable
  pass under a wrong clock left the grace with no expiry on either axis until
  real time passed the stamp, and only a capable pass repairs it, which is
  what the adversary withholds. Both saturations on the path point the same
  way now: a capable time saturated to `i64::MAX` in the store reads as later
  than any real now and expires; the surface's remaining seconds narrow to
  the bridge's `u32` toward `0`, never `u32::MAX`.
- **A clock set back after the expiry was observed does not re-permit.** The
  first reader to see a day elapsed writes a latch on the stamp; every later
  read — including after a relaunch — reads the grace as ended until ONE
  capable verdict (a server that reports its branch) clears it. A latch rather
  than a monotone clock reading: it is one durable bit with one meaning, and a
  monotone reading would have held the clock rule silent for the whole span of
  a forward jump that was later corrected.
- **A clock set forward ends the grace early** — fail-closed; the copy says to
  check the device's date and time as well as to switch servers, and a capable
  pass restores it. There is **no skew band**: the rule latches on a
  one-second forward skew, which is the founder's decision of 2026-09-11 on
  §4v-run review row 2 (the review would have given it a derived tolerance;
  the one-second latch stands, and the cost — an ordinary backward NTP step
  becoming a durable refusal cleared only by a branch-reporting server — is
  accepted, not overlooked).
- **A clock that does not ADVANCE is the residual, and it is NOT closed**
  (§4v-run review row 1; the founder's decision 2026-09-11 — documented, not
  built). The untrusted test is "later than now, or `0`", so a clock held
  anywhere inside `[capable_at, capable_at + UNKNOWN_BRANCH_GRACE_SECS − 1]`
  reads as trusted, the elapsed count never reaches the threshold, and nothing
  latches. An attacker who runs the lightwalletd AND the device's
  unauthenticated NTP source can freeze the claimed tip, withhold
  `consensus_branch_id` and hold the clock still: signing stays permitted
  indefinitely, with both of the grace's axes held by the same party — which
  is what having two axes was meant to prevent. Closing it needs a **third
  axis neither party can freeze**: monotonic uptime accumulated across passes,
  or a high-water reading latched when the clock fails to advance over N
  passes. Recorded here rather than built, so that "at most one day of silence
  on either axis" is read with this cell in mind.

The edges the block rule closes are unchanged:

- **A wallet that has never held a `Current` verdict is outside the grace by
  construction** (`blocks_since_last_current: None`) — a fresh install that has
  only ever met a silent endpoint has never confirmed it can transact
  (`GraceExpiry::NeverConfirmed`).
- **The block numerator is the JUDGED height, `max(scanned, claimed)`.** A
  server that under-reports its tip below what this wallet scanned itself
  cannot shrink the count; one that over-reports ages itself out faster. (An
  earlier version of this paragraph said under-reporting "only shortens its
  own grace" — backwards: a low or frozen claim LENGTHENED it by holding the
  count near zero, which is the freeze the clock rule exists for.)

Outside the grace, `Unknown` refuses signing like `Unsupported` does, with its
OWN typed refusal (`ConsensusGraceExpired { by }`, §3.3) and its own copy naming
the cause honestly and the next step: this server will not say what network it
is on, so try another one — or, when the day ran out on the device clock, check
the device's date and time too. And while the grace runs the user SEES it
(`SyncStatus::UpToDateUnverified`, §3.3): that the server stopped reporting the
network version, and how much grace remains on whichever rule expires first.

### 6.4 Scanning past an upgrade we do not model

A stale binary scanning post-activation blocks encounters data it cannot
represent — and, per §0, cannot even parse at the transaction level. It must
**not** report `Up to date` on a range it could not fully interpret. Until
Ironwood support lands, a stale build reports its sync state as qualified.

**`state.rs` has no vocabulary for this today** — it offers `Scanning { from,
to, percent, spendable_ready }` (which asserts monotonic progress toward a tip)
and `UpToDate { tip }`, and rendering a permanent `Scanning { percent: 99.9 }`
would lie about progress instead of about completeness. A qualified-complete
state is a host-visible API addition and is budgeted in Phase A (§9.1).

*(The draft floated this as the leading hypothesis for the §11.1 defect. **That
hypothesis is refuted, not merely unestablished**: both mis-rendered sends were
mined 28,738 and 28,685 blocks *below* activation and 25 days before it, and
`expired_unmined` is a pure function of that row's own columns. The real cause
is in §11.1.)*

---

## 7. Performance

- **One added unary call per sync pass** (§3.1) — not zero. Small, on a channel
  the pass already has open; no new connection, no new timer, no new wake-up.
- **Zero added battery cost from timers** — the verdict rides the existing pass.
- The predicate itself is integer comparison; cost is not measurable.
- **Crate upgrade (Phase B):** `zcash_client_sqlite 0.22.0` ships schema
  migrations that run once on first open after upgrade. Three things must be
  measured/decided on-device **before release**:
  1. **Duration** on a large, fully-synced wallet; if it exceeds a user-visible
     threshold, show progress rather than a frozen launch.
  2. **Irreversibility** — a migrated db cannot be read by the previous build.
     A pre-migration backup is required, and the downgrade story stated.
  3. **Crash mid-migration** on a funded wallet must resume or restore, never
     corrupt. Proven by a kill test, not by argument.
- **The re-provision cost** for wallets holding a poisoned anchor (§9.2) is a
  rescan, and it is owned here, not hand-waved: it is bounded by the distance
  from the floored anchor to the tip.

---

## 8. Testing strategy (the test contract)

Every gate below maps to a named test, **with the phase it can first be written
in** — several draft rows needed `Nu6_3`, which Phase A does not bring in.
**Anti-vacuity is mandatory**: each negative row is watched failing against a
planted mutant before its fix is believed, and any test asserting "no mismatch
found" also asserts a non-zero comparison count.

### Gate 1 — Security

| Test | Phase | Asserts |
|---|---|---|
| `endpoint_branch_id_never_selects_the_signing_branch` **(OWED — no such test in the tree)** | A | Two builds under **contradictory endpoint branch claims** produce a transaction whose `consensus_branch_id` equals `BranchId::for_height(compiled_params, our_validated_height)` in both. **Not** byte-equality — the builder uses `OsRng` and a real Groth16 prover, so two builds are never byte-identical and the draft's version was unwritable. Watched failing against a mutant that reads the field. |
| `endpoint_tip_never_moves_the_signing_branch` **(OWED — no such test in the tree)** | B | The sibling the draft missed (§1.3): hold the endpoint's branch fixed, vary its `block_height` across the activation, assert the signed branch tracks our own validated tip and nothing else. Needs `Nu6_3` to have a boundary to cross. |
| `hostile_endpoint_branch_id_is_refusal_only` | A | A server claiming an unknown branch yields `Unsupported`; it can never yield `Current`, and never alters a signature. |
| `unparseable_branch_id_does_not_panic` | A | Arbitrary bytes in the hex string field → `None`, never a panic (§4.6 boundary). |

### Gate 2 — UX

| Test | Phase | Asserts |
|---|---|---|
| `stale_binary_refuses_every_signing_path` | A | The §3.2 site table, exactly: interactive send, **queued drain**, sweep and reclaim-mint all refuse. The draft's version tested one of four. |
| `queue_send_still_works_while_unsupported` | A | Offline-first survives the gate: composing and queueing succeed; only the drain refuses. |
| `stale_binary_never_renders_sent_to_the_network` **(OWED — no such test in the tree)** | A | The Pending copy asserting network acceptance cannot appear for a refused send. Direct regression for §0. |
| `behind_wallet_is_not_told_to_update_the_app` | A | §6.1: a mid-restore wallet gets catching-up copy, never the upgrade copy. |
| `recovery_paths_survive_staleness` | A | Seed backup and viewing-key export work while `Unsupported`. |

### Gate 3 — Common sense

| Test | Phase | Asserts |
|---|---|---|
| `wallet_provisioned_before_an_upgrade_detects_it_after` | A | **The exact incident.** Provision at a pre-activation height, advance the tip past activation, assert the verdict flips without re-provisioning. Fails against today's provisioning-only guard. |
| `chain_name_and_sapling_activation_alone_cannot_detect_an_upgrade` | A | Pins §1.2: a fake endpoint with correct historical constants but a new branch is `Unsupported`. Guards against a future "simplification" back to the old predicate. |

### Gate 4 — Architecture

| Test | Phase | Asserts |
|---|---|---|
| `consensus_compatibility_is_the_only_staleness_predicate` + `no_second_branch_comparison` | A | **BUILT.** Source-level, in the `extraction_policy` idiom — which are hand-maintained lists diffing named locations, so this test **must enumerate the sites it scans** (§3.2's table) rather than claim an open-world "no second implementation". |
| `bridge_enums_cover_core_variants` | A | Existing gate; the new enums are registered in its `MAP` (`extraction_policy.rs`, the `MAP`) or they are silently uncovered. |

### Gate 5 — Observability

| Test | Phase | Asserts |
|---|---|---|
| `consensus_verdict_is_logged_with_both_branch_ids_and_no_user_data` **(OWED — no such test in the tree)** | A | The §5 line shape; §5.4 clean. |
| `refused_send_logs_a_cause` **(OWED — no such test in the tree)** | A | A refusal is never counted without a reason — the §0 `accepted=0 rejected=1`-with-no-cause shape cannot recur. |

### Gate 6 — Edge cases

| Test | Phase | Asserts |
|---|---|---|
| `verdict_at_the_exact_activation_height` | **B** | Boundary at 3,428,143 — `tip == activation` is post-upgrade. Needs `Nu6_3`; writing it in Phase A would require a literal, violating Gate 7 in the same breath. |
| `offline_returns_last_verdict_with_age_never_current` | A | §6.2. |
| `cold_start_offline_refuses_signing_but_allows_queueing` | A | The `verdict: None` row of §6.2 — the state the draft could not represent. **Since GRACE-1** the refusal it asserts is `ConsensusNotEvaluated` (its own type), not `NetworkUpgradeUnsupported`. |
| `the_grace_ends_when_either_rule_expires` · `the_clock_latch_refuses_whatever_the_clock_reads_now` · `the_expiry_names_blocks_when_both_rules_expired_and_never_when_nothing_was_confirmed` · `the_time_shown_is_whichever_rule_expires_first_through_the_one_constant` · `the_clock_grace_is_the_block_grace_through_the_one_spacing_constant` · `each_refusal_is_its_own_type` (`consensus::tests`) | **GRACE-1** | §6.3's second rule on the predicate: either rule ends the grace (mutant: "either" made "both"); the latch refuses whatever the clock reads; the reason names blocks when both expired; the surface's time is the smaller of the two through `TARGET_BLOCK_SPACING_SECS`; the day is derived, not written down; three refusals, three types and codes. |
| `an_untrusted_capable_time_reads_as_ended_by_the_clock_never_as_blocks_and_no_time` · `the_surface_seconds_saturate_toward_no_grace_left` (`consensus::tests`) | **GRACE-2** | §4v on the predicate: the reading `observe` hands it for a capable time the clock cannot vouch for (`{ elapsed_secs: None, latched: true }`) is `Ended { by: Clock }`, the surface's time `Some(0)` — never the GRACE-1 `Running { secs_left: None }` (mutant: `latched \|\|` dropped from `expired`); the residual `None`-and-clear reading is named (nothing recorded); the surface's seconds narrow toward `0`, never `u32::MAX` (mutant: `unwrap_or(0)` reverted). Replaces `a_capable_time_in_the_future_leaves_the_block_rule_alone`. |
| `the_capable_time_is_carried_across_unknown_verdicts_and_renewed_by_a_capable_one` · `a_day_on_the_clock_ends_the_grace_on_a_cold_read_and_the_latch_survives_a_set_back_clock` · `an_old_shape_table_is_dropped_and_recreated` (`consensus_stamp::tests`) | **GRACE-1** | The store seam (the S253 lesson, again): the capable time rides across silent passes and only a capable verdict renews it (mutant: renewed on every pass); a day ends the grace on a COLD read with no pass, latches, survives the clock being set back, and one capable verdict clears it (mutants: the `UPDATE` removed; the latch not cleared); the dev-stage format bump drops the old shape. |
| `a_capable_time_later_than_now_is_an_expired_clock_rule` · `a_capable_time_of_zero_is_an_expired_clock_rule_not_a_day_of_grace` · `a_saturated_capable_time_round_trips_as_expired_by_the_clock` (`consensus_stamp::tests`) | **GRACE-2** | §4v at the store seam (G2-1, G2-3's store half, G2-6): a capable time later than now latches and reads `Ended { by: Clock }`, the latch holds at a later `now` below AND above the capable time, one capable `record` clears it (mutant: the future-time branch removed — the base's `capable_at <= now` filter); a capable time of `0` is expired at a clock of `0` and at a real clock, with no elapsed reading (mutant: the `== 0` cell dropped); the `i64::MAX` saturation round-trips as expired. Replaces `a_capable_time_in_the_future_abstains_on_the_clock_and_blocks_decide`. |
| `controller_over_the_production_engine_a_frozen_tip_cannot_hold_the_grace_open_against_the_clock` · `…_a_clock_set_back_after_the_expiry_does_not_re_permit_until_a_capable_pass` · `…_a_forward_clock_ends_an_honest_servers_grace_and_a_capable_server_ignores_the_clock` · `the_never_evaluated_and_never_confirmed_refusals_are_their_own_types` (`wallet::tests`) | **GRACE-1** | The SHIPPED path, clock injected through `open_with_vault_and_seed_port`: the real send gate (`signing_permit`), the parked flag, the cold read and the status the controller PUBLISHES, after passes over the production engine — the frozen-tip attack refused by the clock on a cold read with no pass between; the set-back clock and a relaunch still refused, a capable pass restoring; a forward clock refusing an honest server early; a capable server never consulting the clock. The blind proof rows (§4p G-1..G-8) are the test author's. |
| `controller_over_the_production_engine_a_capable_time_in_the_future_ends_the_grace_by_the_clock_until_a_capable_pass` · `an_out_of_range_claimed_height_cannot_move_the_grace` · `the_parked_reader_warns_when_the_stamp_cannot_be_read_and_stays_fail_open` (`wallet::tests`) | **GRACE-2** | §4v through the shipped path: the clock rewound a week below the capable time and a silent server ten blocks on → the send gate refuses `by: Clock` on a cold read, the parked row and the published status agree, the reading is `{ None, latched }`, the clock corrected forward still refuses (the latch), one capable pass restores and re-records the time (mutant: the future-time branch removed — replaces `…_a_capable_time_in_the_future_shows_blocks_and_no_time`); an endpoint claiming a `block_height` above `u32::MAX` through `evaluate_consensus` leaves the grace's count, the judged height and the gate as the honest pass left them (mutant: the refusal reverted to `unwrap_or(u32::MAX)`); the parked reader on an aux store that cannot take the latch write stays fail-open for display and warns `wallet.parked_verdict` with a static code, §5.4-clean, while the gate refuses (mutant: the `.ok().flatten()` restored). The blind twins (G2-2, G2-3, G2-4's rewrite of G-4) are the test author's. |
| `absent_branch_id_is_unknown_not_current` | A | §6.3 — including the prost case where an absent tag-5 string and a present-but-empty one are indistinguishable. |
| `unknown_branch_signs_inside_the_grace_and_refuses_outside_it` | A | §6.3's boundary, both sides: at `UNKNOWN_BRANCH_GRACE_BLOCKS - 1` signing is allowed, at the threshold it refuses. Watched failing against a mutant that ignores the grace in either direction. |
| `unknown_branch_with_no_prior_current_verdict_refuses` | A | The `blocks_since_last_current: None` arm — a fresh install that has only ever met a silent endpoint never signs. |
| `under_reported_tip_cannot_buy_a_current_verdict` | A | **BUILT.** The §1.3 height rule, made mechanical: an endpoint claiming a lower tip AND the branch that is honest at that tip is still refused, because the verdict is judged at the highest height either side attests. Watched failing against a judge-at-the-claim mutant. |
| `a_lying_endpoint_can_hide_our_own_staleness_from_this_predicate` | A | **BUILT.** The documented LIMIT, pinned so nobody mistakes the branch check for a defence it is not: a stale build whose endpoint lies in the direction of that staleness cannot detect it from these inputs. This is why §4's local evidence row exists. |
| `known_upgrades_is_really_complete` | A | **BUILT, and it FIRED (S256).** The anchor ceiling rests on a hand-maintained list of upgrades this build knows; this fails the moment the pinned crate learns one the list does not name — the list cannot go quietly stale, which is the shape of the outage itself. The NU6.3 wave made it fail; adding `NetworkUpgrade::Nu6_3` to `KNOWN_UPGRADES` (Phase B step 6) is what discharged it. The mechanism worked exactly as designed, on the first upgrade after it was written. |
| `the_anchor_ceiling_is_the_newest_upgrade_this_build_can_model` | **B** | **BUILT (S256)**, replacing Phase A's `the_anchor_ceiling_sits_below_an_upgrade_we_cannot_model`, whose two assertions (`ceiling == 3_364_600` and `ceiling < IRONWOOD_MAINNET`) step 6 inverts. Rewritten, not retuned — a literal swap would read as a Phase-B regression while asserting nothing, because the literal was never the property. Asserts: (1) structurally, the branch far past every activation equals the branch at the ceiling, so the ceiling IS the newest modellable height — no literal involved; (2) it rose to at-or-above the Ironwood activation, the direction check (a ceiling may rise as the build learns more, never fall); (3) every bundled row the ceiling admits at/above the activation carries an Ironwood frontier. **Clause (3) has no row satisfying its antecedent today and the test says so rather than pretending otherwise** — raising the ceiling to 3,428,143 does NOT reach the 13 bundled rows above it (the first is 3,429,810), because the ceiling stays below the newest activation on purpose: past it an upgrade we do not know may be live. What is asserted non-vacuously is the boundary — the newest selectable anchor is below the activation. Measured effect of the raise: newest anchorable row 3,362,500 → 3,427,310, ~64,810 fewer blocks of first scan, every newly-reachable row below the activation where an empty Ironwood frontier is correct. |
| `unknown_branch_fixture_is_really_unknown` | **B** | **BUILT (S256).** The non-vacuity guard for the outage fixtures, and the successor to `the_pinned_params_really_do_not_know_ironwood` in that role. Five consensus rows assert that a branch this build cannot model refuses signing; they mean nothing if the branch they use is one the crate knows. Written against Ironwood by name, they all inverted the moment the crate learned it — so they are now written against `UNKNOWN_FUTURE_BRANCH`, and this row fails on the next wave the way its predecessor failed on this one. |

### Gate 7 — No magic numbers

Activation heights come from `zcash_protocol::consensus`, never literals.
`checkpoint_activation_heights_match_zcash_protocol` is **BUILT (S256, Phase B
step 7)**: the test module's `IRONWOOD_MAINNET` / `NU6_3_BRANCH` literals are
gone, replaced by `MAIN_NETWORK.activation_height(NetworkUpgrade::Nu6_3)` and
`BranchId::for_height`, and the row pins both networks' activations (3,428,143 /
4,134,000) against the crate — one place where the number the ADR, the spec, the
emitter and the bundle gap test all state is checked against its source.

`checkpoints/mod.rs`'s copies are gone too: its `IRONWOOD_MAINNET` /
`IRONWOOD_TESTNET` constants were removed in the same commit and
`ironwood_activation()` now reads `NetworkUpgrade::Nu6_3` from the compiled
params. That also retires a limit the bundle gap test documented at length — the
activation it anchors to is no longer a literal in its own file, so drifting it
means moving the crate pin, a reviewed event. Watched: pointing that helper at
`Nu6_2` instead fails both the gap test and the frontier test.

**Still owed, and it is the harder half:** a SOURCE-LEVEL check that no
activation literal exists outside `zcash_protocol`. One legitimate literal
remains — `IRONWOOD_ACTIVATION` in `tools/extract_checkpoints.py`, which reads
upstream JSON rather than the crate and has no params to consult — but nothing
yet FAILS when a new one appears somewhere else. Until that scan exists, gate 7
is enforced mechanically in `consensus.rs` and `checkpoints/mod.rs`, and by
convention everywhere else.
The S252 defect where 1680000/1840000 were mistaken for NU5 activations must not
recur in a new place.

### Gate 8 — i18n

All §6.1 strings enter the l10n catalogue across all 16 locales, in the
`walletSendFault*` and `walletStall*` families named there; none hardcoded.
`no_hardcoded_consensus_strings` **(OWED — no such test in the tree; the keys
are in all 16 locales but nothing yet asserts no consensus string is
hardcoded)**.

### Upgrade-specific (Phase A / B / C)

| Test | Phase | Asserts |
|---|---|---|
| `treestate_bundle_samples_no_row_between_ironwood_activation_and_the_reimport_threshold` | A | **BUILT (S255).** The `rows_in_gap == 0` half, which needs no new data and so did not wait for the bump: no bundled row falls in `[activation, first bundled row at/above activation)` — mainnet `[3,428,143, 3,429,810)`, testnet `[4,134,000, 4,134,750)` — plus anti-vacuity (rows exist on BOTH sides of the activation) and a pin of the threshold constant to the table. It is what lets §9.2's re-import threshold narrow from the activation height, and it **also freezes the direction** — a threshold may narrow toward the activation, never rise, since raising it drops already-anchored wallets out of remediation. Watched failing against **five** mutants, each named with the assertion it actually trips: a real bundled row planted in `data.rs` inside the window (in-window assert); a threshold **raised** (direction guard); a threshold **lowered** (table pin); an activation above the bundle's tail (inversion assert); an activation **lowered** so real rows fall inside the window (direction guard). The direction guard is a frozen DISTANCE from the activation (1,667 / 750) rather than the threshold's own value, because a ceiling spelled with the same token as the constant it polices degenerates to `X <= X` under the value-based search-and-replace that is how heights actually get edited here. **The limit, measured rather than asserted, because the first version of this row overstated the coverage:** the ceiling rides the activation, so **any upward drift of the ACTIVATION — solo, not merely joint — passes every assertion here, for anything from +1 up to the full window width** (+1,667 mainnet; +1,668 is the first value caught). No test can self-anchor a consensus constant, since any literal it compares against is one more literal in the same file; that job is `checkpoint_activation_heights_match_zcash_protocol` against the crate in Phase B, and until it exists the activation literal is unguarded here **by construction, not by oversight**. **The window is not an empty-tree region** — measured 2026-09-07, mainnet field 7 is non-empty from 3,428,144 and testnet from 4,134,683 — it is a region the bundle does not sample, which is why this is a test and not a comment. |
| `treestate_bundle_carries_an_ironwood_frontier_after_activation` | B | **BUILT (S256)**, in the same commit as the step-4 re-extraction that made the 5-ary table exist. Decoded `tree_size()`, not string non-emptiness: absent below the **activation**, present and non-decreasing at or after it, every carried row decoding to `tree_size() > 0`, and exactly 13 mainnet / 25 testnet rows asserted. (An earlier draft said "absent below 3,429,810"; that is true of the bundle only because of the row above, and stating it as the reason would enshrine a false one — the assertion is anchored at the ACTIVATION.) Mirrors `..._frontiers_grow_and_orchard_activates_at_nu5`. The reason it must check the DECODED tree: `TreeState::ironwood_tree()` maps an empty field to `CommitmentTree::empty()` and returns `Ok`, so a dropped column decodes cleanly and nothing else in the suite can see it — `treestate_bundle_every_row_decodes`'s only real assertion is `birthday == height + 1`, which is derived from the height we supplied and is therefore tautological with respect to the frontier. `treestate_bundle_frontiers_chain_to_their_predecessors` also gained an Ironwood arm with its **own** anti-vacuity counter, separate from `total_shared`: folded into a counter already in the tens of thousands, an Ironwood arm comparing `None` to `None` on every row would have been invisible. |
| `poisoned_pre_ironwood_anchor_is_refused_or_repaired` | B | The §9.2 money case: a wallet whose birthday row predates ironwood modelling is DETECTED on first open after upgrade — never silently continued. Whether the response is a re-provision, a rescan or a refusal is deliberately left open: ADR-0540 Decision 6 freezes the scope and leaves the remedy to the money step, and a register row naming one would assert a decision nobody has taken. |
| `orchard_to_ironwood_migration_round_trip` | C | Zat-for-zat conservation minus the exact ZIP-317 fee; a successful **spend** of the migrated note against the fake chain's anchors; idempotence; and an interruption that leaves all-orchard or all-ironwood, never a torn state. "Returns `Ok(())` with orchard balance 0" is not a round trip. |

---

## 9. Work breakdown

### 9.1 Phase A — staleness detection (ships first, alone)

> **Build state, 2026-09-06.** All ten steps have code (core, bridge, reference
> UI, l10n ×16) and BOTH four-angle reviews are folded — the spec review in §12
> and the post-build code review in §12a.
>
> **Test contract: 6 of the 11 owed rows are now written**, including gate 4
> (`consensus_compatibility_is_the_only_staleness_predicate`, plus its sibling
> `no_second_branch_comparison`) — the gate that makes the one-predicate claim
> mechanical instead of a promise. It was watched failing against a planted
> inline `matches!`, and that exercise found the gate VACUOUS on its first
> writing: it stopped at the first `#[cfg(test)]`, which in `wallet.rs` is a
> test HELPER mid-file, so the planted violation passed. It now stops at the
> test module.
>
> **Still OWED (SIX rows, each marked in §8 — count corrected S254, when three
> separate summaries said five while the register marked six):** one needs
> `Nu6_3` and is Phase B (`endpoint_tip_never_moves_the_signing_branch`); two
> are observability rows needing the tracing-guard harness; one is a Dart source
> scan; one is a Dart widget test
> (`stale_binary_never_renders_sent_to_the_network` — the direct §0 regression
> guard, the row the old five-count silently dropped); one needs a funded
> harness (`endpoint_branch_id_never_selects_the_signing_branch`).
> **Also outstanding:** no device run, and the new translations are
> machine-produced and owed a native-review pass like the #341 batch (they are
> money-adjacent — "your funds are safe", "nothing has been sent").

Independent of Ironwood support. Turns a silent outage into a status message,
and is the durable part: it protects the **next** upgrade too. Review made it
slightly bigger than the draft; every addition below is load-bearing, and the
whole thing is still smaller than Phase B.

1. Extend `ServerIdentity` with the two live fields (§2).
2. Add `consensus_compatibility` (§3.2) taking **both** heights, plus
   `ConsensusStatusSnapshot`, its persistence, `UNKNOWN_BRANCH_GRACE_BLOCKS` in
   `constants.rs` (§6.3 — named, never inlined), and `RW-SYNC-002`.
3. **Gate `create_signed_core`** — the choke point — plus the early
   `propose()` check. Leave `queue_send` ungated. Leave reads live (§1.5).
   **DONE:** the choke point is gated by the `SigningPermit` token (all seven
   production call sites, compiler-enforced), and `propose` / `propose_uri` /
   `propose_shield` carry the early pre-flight beside the watch-only refusal.
   `queue_send` / `queue_send_uri` are deliberately NOT gated — pinned by
   `queue_send_still_works_while_unsupported`, watched failing against a
   gate-removal mutant.
4. Give the queued-drain refusal a typed row state and a host surface (§3.2).
   **DONE:** the drain refuses and counts (`ResubmitSummary.refused_stale`, on
   the emit), and `ParkedSend.signing_block` (GRACE-1: was a bool
   `blocked_by_network_upgrade`; now `Option<SigningBlock>` — `NetworkUpgrade`
   for `Unsupported`, `GraceExpired { by }` for an ended grace, `None` when the
   row will send on its own, which is also the never-evaluated wallet's
   reading) carries the reason to
   the row — read under the SAME aux lock as the rows, so the list cannot
   disagree with itself. The reference UI ranks it above `paused` in the hint
   (both mean "will not send on its own"; this is the one where Retry is
   useless) and withdraws "Send now", which would open the host's
   authorize-spend bracket only to return the typed refusal. Cancel stays.
5. Carry the recurring `GetLightdInfo` on the sync pass (§3.1) and retire the
   `never_scanned` gate in `run_engine_pass`.
6. Ceiling the bundled birthday anchor at `newest_known_activation` —
   unconditionally, in the ONE anchor-selection home
   (`checkpoints::bundled_treestate`), so every caller inherits it (§1.5, §9.2).
7. Bound the enhancement retry for unparseable post-activation transactions and
   promote the parse failure to staleness evidence (§0, §4).
   **DONE for the retry bound; the verdict feed is deliberately NOT built.**
   **LIVE since T0-4 (2026-09-11) — it was INERT before that.**
   `enhance::unknown_branch_evidence` reads the branch from a **v5 or v6**
   header (the v6 header sits at the SAME offsets — `read_v6_header_fragment`
   delegates to the shared `read_header_fragment` — so one read serves both, and
   the four constants come from `zcash_protocol::constants` rather than
   literals). Until T0-4 it matched v5 ONLY, and every post-Ironwood transaction
   is v6, so from activation on 2026-07-28 this whole step did nothing: the
   counter was structurally zero, the warn never fired and the bound below never
   engaged (INC-015).
   `EnhancementOutcome::unknown_branch` counts and logs it, and the txid joins
   `UnparseableTxids` — a PROCESS-lifetime set, so the download happens once per
   launch instead of once per pass. In memory on purpose: the condition belongs
   to the binary, so the memory should expire exactly when the binary can
   change. The filter runs BEFORE the batch cap, or unparseable rows would
   starve the ones we can still enhance. **Still owed:** feeding this evidence
   into the verdict itself (§4's threat row remains a diagnostic, not a
   mitigation).
8. Add the qualified sync state to `state.rs` (§6.4) — a host API addition.
   **DONE:** `SyncStatus::UpToDateLimited`, a distinct VARIANT rather than a
   flag on `UpToDate`. A host that ignores a new bool renders a confident
   "Up to date" over a range it was told is unreadable; a host that ignores a
   new variant falls into its catch-all, which is honest by default. Decided by
   `SyncEnginePort::scan_is_fully_interpretable` (default `true`, so fakes need
   not implement it), which reads the persisted verdict and fails OPEN — a
   display claim must not invent a degradation.
9. Host surface + l10n keys in the two families named in §6.1.
   **DONE except its gate-8 test** (`no_hardcoded_consensus_strings`, OWED):
   FOUR keys × 16 locales — `walletSendFaultNetworkUpgrade` (under the shared
   `walletSendFailedTitle`), `walletSyncUpToDateLimited` + its sheet
   explanation, and `walletParkedBlockedByNetworkUpgrade` — wired into the
   send-fault taxonomy (BOTH the propose path and the sign path, which can
   refuse when the verdict flips between Review and Confirm), the sync
   presentation, the health badge, and the parked row.
   A fifth key, `walletStallNetworkUpgrade`, was written and then **CUT**: the
   `walletStall*` family is driven by a `StallReason` producer and there is
   none — consensus staleness is not a sync stall, sync works fine. Shipping a
   string in 16 locales for a state the SDK cannot emit, while this section
   claimed it was wired, is the same false-claim shape this spec exists to end.
   **PRODUCT CALL, revisit if you disagree:** the offline queue is NOT offered
   beside this fault. The core permits queueing and the row explains itself, but
   "queue to send later" beside a wait only an APP UPDATE ends reads as a
   promise the app cannot keep. One line in `send_screen.dart` reverses it.
10. Patch `wallet-sdk.md` in the same commit (§1.2, §2): the §3.2f
    network-match bullet, the §8 register row for
    `provision_rejects_network_mismatched_endpoint`, the gate-6 row for
    `network_mismatch_rejected_everywhere`, the `RW-SYNC-001` sentence, and
    §3.2f's "CS-M1 network-match run directly until the first block scans"
    aside. Cited by SYMBOL, not line number — the lines move when the edit
    lands, which is how the draft's own citations went stale.

**Not in Phase A:** anything needing `Nu6_3` (see the Phase column in §8), and
the `mined_height` defect (§11.1), which is not Ironwood-caused and has its own
row.

### 9.2 Phase B — the crate upgrade

**The draft listed four pins. It is ELEVEN, across three manifests.** (The S253
audit corrected four to nine from the manifests; the S254 upstream survey
corrected nine to eleven from the crates.io index —
`docs/plan/ironwood-phase-b-survey.md` C1/C2/C3 carries the evidence, and
`docs/plan/ironwood-phase-b.md` §1 is the work order.)

> **The pin set's one source of truth is
> [ADR-0540](../adr/0540-ironwood-nu63-crate-wave-and-sealed-orchard-pool.md)
> Decision 1**, which carries the same eleven rows with their manifest line
> numbers. The table below is kept here because the surrounding narrative reads
> against it; **if the two ever disagree, the ADR wins** and this table is the
> stale one.

| Pin | From | To | Where |
|---|---|---|---|
| `zcash_protocol` | 0.9.0 | 0.10.5 | `sdk/Cargo.toml` **and root `Cargo.toml`** |
| `zcash_address` | 0.12.0 | 0.13.0 | `sdk/Cargo.toml` **and root `Cargo.toml`** |
| `zcash_client_backend` | 0.23.0 | 0.24.0 | `sdk/Cargo.toml` |
| `zcash_client_sqlite` | 0.21.0 | 0.22.0 | `sdk/Cargo.toml` |
| `zcash_keys` | 0.14.0 | 0.16.1 | `sdk/Cargo.toml` |
| `zip321` | 0.8.0 | 0.9.0 | `sdk/Cargo.toml` |
| `orchard` | 0.14.0 | 0.15.5 | `sdk/Cargo.toml` |
| `zcash_transparent` | 0.8.0 | 0.10.0 | `sdk/Cargo.toml` |
| **`shardtree`** | **0.6.2** | **0.7** | `sdk/Cargo.toml` |
| `zcash_primitives` | 0.28.0 | 0.30.1 | `sdk/zec-wallet-core/Cargo.toml` |
| `zcash_proofs` | 0.28.0 | 0.30.0 | `sdk/zec-wallet-core/Cargo.toml` |

**`shardtree` is not optional and was missing from every earlier count**:
`zcash_client_sqlite` 0.22.0 requires `shardtree ^0.7` with `legacy-api`,
non-optional, and we exact-pin `=0.6.2` — the wave does not resolve without it.
The table (and ADR-0540) name the LINE, `0.7`; the exact pin the wave resolved
to and the manifest carries is **`=0.7.1`** — the newest live release in that
line by `created_at` (0.7.0 2026-07-09, 0.7.1 2026-07-17; there is no 0.7.2+).
A first attempt at the dry run invented `0.7.3`, which is why the exact value is
written down here rather than left to be re-derived.

**`zcash_script` does NOT move.** An earlier version of this section said it
moved "with the wave"; that is wrong and would revert a consensus fix. Its
semver-latest `0.5.2` was published 2026-02-23, three months BEFORE the pinned
`0.4.5` (2026-05-29) — the 0.4.x and 0.5.x lines are parallel republishes — and
`0.5.2` does not satisfy the `^0.4.3` that four crates in this wave still
require. Leave it at `=0.4.5`. `sapling-crypto` 0.7.0 is already latest and
does not move either.

**One atomic commit.** A partial bump either fails to resolve or resolves to two
copies of a crate with no error, whose symptom reads as `expected
TransactionRequest, found TransactionRequest`.

**The root manifest is not optional.** `zcash_address`, `zcash_protocol` AND
`zip321` are in the `POLICED` set of `pins_policy.rs`, and all three sit in the
root manifest; `wallet_pins_mirror_root_or_extracted` panics `PIN DRIFT` unless
`sdk/Cargo.toml` mirrors root byte-for-byte. Bumping one manifest fails CI.
(The table's "Where" column originally named only two — corrected by the S253
self-audit; `zip321` moves 0.8.0 → 0.9.0 in BOTH manifests.)

**Re-run the gate-0 collision probe.** `sdk/Cargo.toml`'s own comments record
that `zcash_transparent → bip32 =0.6.0-pre.1 → digest =0.11.0-pre.9` was the
constraint the current pin set exists to avoid; the comment says to re-check at
every bump. Do it, and record the result.

> **RUN S256, answered from the POST-BUMP LOCK rather than the manifests:**
> `bip32` is still `=0.6.0-pre.1`, `secp256k1` still `0.29.1`, and `digest`
> still resolves to BOTH `0.10.7` and `0.11.0-pre.9`. The carve is **still
> needed and still sufficient** — it does not exit with this wave. Recorded at
> the site in `sdk/Cargo.toml`'s gate-0 note, and two stale dated claims were
> corrected in the same commit rather than left to read as false: root
> `Cargo.toml`'s "re-verified on crates.io 2026-06-11 (the nu6.2 stack)" and its
> `zip321` line asserting the crate "wants zcash_address ^0.12 + zcash_protocol
> ^0.9 exactly".

**The third frontier slice, and the wallets already holding a poisoned anchor.**
Upstream `zcash_client_backend 0.24.0` models the pool: `TreeState` gains
`ironwood_tree` and **`ChainState::new` takes a required `final_ironwood_tree`**.
The failure mode is silent: `TreeState::ironwood_tree()` returns an **empty
tree** for an empty field — correct only *at* activation. Our shipped bundle has
**13 mainnet tree-state rows at 3,429,810 … 3,459,780** with the field dropped
by the extractor, and `resolve_offline_birthday` (`provision.rs:104`) floors a
fresh create to the newest row by creation time — today, one of those 13. So:

- Re-extract the bundle **with** `ironwoodTree` in the same change as the bump.
  The extractor's abort-on-unknown-key fires at extraction time, not at
  provisioning time, so nothing else forces this.
- **Wallets provisioned BEFORE the Phase A anchor ceiling landed carry a
  `ChainState` with an implicitly-empty third tree that is persisted and
  immutable after first import.** Once the decoder models the pool, every
  position, witness and anchor for post-birthday Ironwood notes is wrong.
  **"And there is no in-place repair" stood here unchecked from the draft
  onward; S255 found two upstream mechanisms that may contradict it** —
  `zcash_client_sqlite 0.22.0`'s `ironwood_shardtree` migration re-queues every
  range at or above the activation for rescan, and `zcash_client_backend
  0.24.0`'s `update_tree` checkpoints `from_state.final_ironwood_tree()` on every
  batch, which C6(b) measured our endpoints do supply. The money step reads
  `put_blocks` / `update_tree` / `rewind_to_chain_state` and settles it before
  building anything (`ironwood-phase-b.md` §2). What is settled is the SCOPE: if
  a repair is needed, Phase B ships an anchor-schema version and, on
  first open, re-imports any account whose birthday is at or
  above the **lowest bundled tree-state row at or above the activation** —
  mainnet 3,429,810, testnet 4,134,750 (ADR-0540 Decision 6). That is a rule,
  not a number: it narrows below the activation height only because
  `treestate_bundle_samples_no_row_between_ironwood_activation_and_the_reimport_threshold` asserts no
  bundled row sits in between, so nothing can be *anchored* there. Do not read
  the gap as an empty-tree region — measured 2026-09-07, mainnet field 7 is
  already non-empty at 3,428,144. §1.5's anchor ceiling is what keeps that
  population from
  growing — with it in force, a pre-Ironwood build can no longer create such a
  wallet at all.
- **This remedy is BIRTHDAY-scoped, and the hazard is not** (survey C6(b) —
  **PROBED S255**). `sync.rs`'s `to_chain_state()` also calls `ironwood_tree()`
  **per scan batch**, and an empty field decodes to an empty tree without
  erroring, so a lightwalletd that does not populate `TreeState` field 7 yields
  a wrong per-batch anchor indefinitely with nothing red — a live-endpoint
  problem the re-import above does not touch. The empirical half is answered:
  `GetTreeState` against `zec.rocks` and `testnet.zec.rocks` (ECC LightWalletD
  v0.5.3 / v0.5.4) returns a populated field 7 at and above activation, and an
  absent one below it, so **the endpoints we ship against are fine** and this is
  latent rather than live. It stays a real hazard for a self-hosted or older
  server. **Posture decided (founder, S255): stop syncing AT the upgrade
  height** — scan normally up to the activation, then halt with *"this server
  can't serve blocks past the network upgrade; choose another server"*.
  Everything already scanned stays valid, and nothing wrong is written.
  Rejected: refusing the endpoint outright (discards sync that would have been
  correct) and scanning on behind a warning (the damage *is* the scan — the
  repair afterwards is a re-import, not a re-scan). Do not read this bullet as
  saying the provisioning path is the whole exposure.
- **The ceiling's cost, stated plainly, and its Phase-B move MEASURED.** Under
  the Phase-A pin the ceiling was Nu6_2's activation (3,364,600) and a fresh
  create anchored ~95,000 blocks below the bundle tail. Step 6 raised it to
  Nu6_3's 3,428,143 (S256), which moves the newest anchorable bundled row
  3,362,500 → 3,427,310 — about **64,810 fewer blocks** of first scan. **It does
  NOT "restore the tail"**, which an earlier version of this bullet implied: the
  13 bundled rows above the activation stay unreachable, because the ceiling sits
  below the newest activation on purpose — past it an upgrade this build does not
  know may be live. Over-scanning is the direction the birthday logic prefers
  anyway: a low anchor scans extra, a high one silently skips funds.
- The `(height, time)` and tree-state slices stay row-aligned; both provenance
  hashes and `CHECKPOINTS.md` move in the same commit, and §3.2f/§3.6 of
  `wallet-sdk.md` are patched for the 4→5-tuple.

### 9.3 Phase C — ZIP-318 migration

Orchard → Ironwood, consumed whole from the upstream **`zcash_pool_migration`**
crate — **not** `zcash_client_sqlite::pool_migration`, which does not exist. It
is a separate crate at **0.1.0** (first published 2026-07-24, updated
2026-08-19, MIT OR Apache-2.0), pulled in by `zcash_client_sqlite 0.22.0`, and
adopting it is a **new direct dependency** with its own `deny.toml` and
supply-chain review. Its own crypto-change review cycle and its own multi-angle
review: it moves real value across a turnstile.

### 9.4 Sequencing, and the ADR

Phase A must land first. It is small, it is the honest answer to the user, and
shipping B before A would fix today while leaving the mechanism that hid the
problem intact — which is how this recurs at NU7, whose coinholder vote opened
2026-08-25.

**The ADR owed before Phase B is written: [ADR-0540](../adr/0540-ironwood-nu63-crate-wave-and-sealed-orchard-pool.md)
(Accepted, 2026-09-07).** ADR-0005's Decision paragraph names exact crate
versions and ADRs here are append-only, never edited. Phase B changes them;
Phase C adds a new direct dependency and a value-moving migration; and Ironwood
seals a pool the ADR's world did not contain. ADR-0540 covers the version wave,
the new crates, the sealed-pool posture, the anchor re-import rule and the
endpoint posture — superseding nothing, amending ADR-0005 by reference, which
now carries a pointer back.

**crypto-change review scope:** Phases B and C, both (§4). Phase A does not need it.

---

## 10. Multi-platform / multi-device

- **Platform-independent.** Consensus rules are chain properties; no
  platform-specific behaviour, no iOS background implications (no new timer —
  the added call rides the existing pass).
- **Multi-device:** the verdict is **per-binary, never synced.** Two devices on
  different app versions legitimately hold different verdicts, and a synced
  "we're fine" from a newer device would be actively dangerous on an older one.
- **On persistence, correcting the draft:** the *verdict* is derived and never
  replicated, but `ConsensusStatusSnapshot` **is persisted locally** (§2). It
  must be, or §6.2's "last verdict and its age" is unimplementable across a
  process restart and every cold start is a fabricated state. Local-only,
  never synced, and invalidated by any change of endpoint.

---

## 11. Open questions

### 11.1 `mined_height = NULL` on mined sends — **ROOT-CAUSED, moved out, FIXED (S254)**

Two confirmed on-chain sends (§0) render as *"expired …
cancelled … The amount is still yours to spend."* **The S252 conclusion that
"the fault is upstream of enhancement" is wrong. The fault is ours, in
`enhance.rs`:**

`select_enhancement_targets` (in `enhance.rs`) folds two distinct upstream
request kinds into one txid list:

```rust
TransactionDataRequest::Enhancement(t) | TransactionDataRequest::GetStatus(t) => Some(t)
```

Upstream's contract (`zcash_client_backend-0.23.0` `data_api.rs:1163-1168`) is
explicit that they differ: `GetStatus` must be answered with
`WalletWrite::set_transaction_status`; only `Enhancement` is answered with
`decrypt_and_store_transaction`. Our loop answers both with
`store.store_decrypted(tx, None)` in the same loop. Meanwhile
`zcash_client_sqlite-0.21.0`'s `transaction_data_requests` (`wallet.rs:4427-4436`)
synthesizes a status request *"for each transaction known to the wallet for
which we don't have the mined height"* — exactly these two rows. The wallet
asks "was this mined?", answers itself "here it is, mined height unknown", and
re-asks forever.

Reported in the same review: a second latch where `mark_not_recognized` treats
one endpoint's `Ok(None)` as chain truth and permanently stops status queries
for that txid; and the claim that released inputs are re-offered as spendable.
**Partly settled by reading `zcash_client_sqlite-0.21.0` (S254):** the latch does
NOT hold for the synthesized status requests — `TxidNotRecognized` and
`NotInMainChain` write the same `confirmed_unmined_at_height`, and
`transaction_data_requests` re-queries the row until the tip passes its expiry
height — but `set_transaction_status` DOES delete `tx_retrieval_queue` rows, so
the claim stands for explicit `Enhancement` entries. The released-inputs claim
remains **unverified**.

**This is not Ironwood-caused and does not belong to this spec.** It has its own
ROADMAP row, where the fix is recorded. Of the two constraints that carried
over, the first is **discharged** — it was settled with a failing unit test on
`run_enhancement_pass` first (`run_enhancement_pass_answers_get_status_with_a_status_not_a_decrypt`,
INC-009) — and the second ~~still **stands**: re-run the diagnostic post-Phase-B
on the **migrated-in-place** db, never on a fresh restore~~ **was RETIRED S261
by founder ruling** (*"only new accounts or restored from the seedphrase"* —
`production-readiness-phase-1.md` §6): there is no migrated-in-place wallet
category, so the diagnostic's masking concern (a restore rescans and sets
`mined_height` via `put_tx_meta`, making a still-broken loop look healthy) is
answered structurally — the restore path IS the product path, and the loop's
health is guarded by the named unit test above rather than by a device
diagnostic on a wallet shape that no longer exists.

### 11.2 Does `Unknown` refuse signing? — **DECIDED (founder, 2026-09-06)**

**Degrade with age.** Design in §6.3; the threshold is
`UNKNOWN_BRANCH_GRACE_BLOCKS = 1_152` (~1 day), named, not inlined, and pinned
by a test. **AMENDED 2026-09-10 (founder, GRACE-1):** age is measured on TWO
axes — blocks the chain advanced, AND seconds the device clock advanced since
the last capable verdict (`UNKNOWN_BRANCH_GRACE_SECS`, derived from the block
grace) — and the grace ends when EITHER expires; the clock only tightens (a
future timestamp is untrusted; a clock set back after an observed expiry does
not re-permit), and the SDK UI shows the grace, its expiry and the next step.
The block-only rule was disarmed by a server that froze its tip. The two
rejected options are kept here as the record:

- **Pass through (the draft's choice).** Rejected: an adversary who strips one
  proto field turns Phase A off entirely, and the §0 outage returns silently.
- **Refuse outright.** Rejected: any honest server that doesn't populate the
  field becomes unusable for sending, and the size of that population is
  *asserted, never measured*. (The field is non-optional in proto3 and
  lightwalletd has populated it since 2019, so the population is probably near
  zero — but that is an argument, not a measurement, and the grace costs
  nothing to keep it working.)

### 11.3 Remaining

1. ~~Does Phase A refuse **shielding** and **swap deposits** as well as sends?~~
   **ANSWERED by construction (built 2026-09-06).** Both sign, so both refuse:
   shielding at `propose_shield` plus the choke point, the swap deposit at its
   own `signing_permit` consult before the sign pull. **Still owed:** confirm
   against the swap provider's deposit deadlines that a refusal (rather than a
   failed broadcast) is the better outcome for an in-flight quote — a refused
   deposit lapses the quote, which is the honest result but should be surfaced
   as such, not as a generic fault.
2. `zcash_protocol` 0.10.5 exposes `Nu7` only under `cfg(zcash_unstable="nu7")`
   with a **placeholder** branch id `0xffff_ffff`. A `pins-policy` row should
   forbid that cfg in shipped builds — otherwise `expected_branch_id` can become
   the placeholder.
3. `zcash_client_sqlite 0.22.0` migration duration, irreversibility and
   crash-safety on a fully-synced funded wallet — measure before release (§7).

---

## 12. Review record (S253)

Consistency audit against PRODUCT_VISION, `docs/arch/`, the ADRs and
`wallet-sdk.md`, then the four-angle review required by the review policy for
money-and-consensus-touching work: security review + arch review +
crypto audit in parallel, then code reviewer. All three parallel angles
found defects; every load-bearing finding was re-verified against the code or
the vendored crates before being folded.

**Draft claims that survived:** the §1.2 diagnosis (the guard compares two
immutable constants and discards the branch); `RW-SYNC-002` free; every upstream
constant in §0; the P2-before-P1 sequencing; the one-predicate rule; "never sign
with the server's branch"; ADR-0005's citation.

**Draft designs that did not:**

| Finding | Angle | Fold |
|---|---|---|
| Refusal specified only at `propose()`; the queued drain, sweep and reclaim-mint reach a signature without it | security review | §3.2 site table; gate moved to `create_signed_core` |
| §1.3's invariant too narrow — the signing branch is indexed by a server-supplied height | crypto audit | §1.3 restated; predicate takes both heights |
| §11.1's root cause is our `GetStatus`/`Enhancement` fold, not upstream | crypto audit | §11.1 rewritten, moved to its own row |
| Bundle rows in the Ironwood window decode to a silent empty frontier; wallets already provisioned are poisoned | crypto audit + own probe | §9.2 remediation; §1.5 floored anchor |
| The enum could not represent `Unknown`, `Behind` or "never evaluated"; `ConsensusStatusSnapshot` undefined; "never persisted" contradicted §6.2 | code reviewer + security review | §2 rewritten; §10 corrected |
| "Zero new traffic / zero cost" false — the only per-pass reacher is behind `never_scanned` | security review | §3.1, §5, §7 re-priced |
| §9.2's four bumps are nine across three manifests, incl. the policed root | arch review | §9.2 table |
| ZIP-318 lives in `zcash_pool_migration` 0.1.0, not `zcash_client_sqlite::pool_migration` | arch review + own probe | §4, §9.3 |
| §4's "adds no cryptography" would route Phase B around crypto-change review | crypto audit | §4 scoped to Phase A; §9.4 |
| §6.4's hypothesis for §11.1 is refuted (sends mined 28k blocks below activation) | crypto audit | Parenthetical struck |
| Gate 1's byte-identical test unwritable against `OsRng` + a real prover | security review | Gate 1 rewritten to assert the field |
| Gate 6/7 rows need `Nu6_3`, absent in Phase A | crypto audit + security review | Phase column added to §8 |
| §6.1 copy breaks the two-tier `walletSendFault*` convention and fires for a merely-behind wallet | code reviewer | §6.1 rewritten; `Behind` added |
| §6.4's "existing catching-up vocabulary" does not exist in `state.rs` | security review | §6.4; Phase A step 8 |
| No ADR proposed | arch review | §9.4 |

### 12a. The POST-BUILD review (same four angles, on the code)

The spec review above ran before a line was written. This one ran on the built
diff, and it found more than the first — which is the argument for doing both.

| Finding | Angle | Fold |
|---|---|---|
| **The §6.3 grace was INERT.** `consensus_stamp::read` rebuilt every `Unknown` with `blocks_since_last_current: None`, and `permits_signing` reads `None` as outside the grace — so every `Unknown` refused. The founder's "degrade with age" decision shipped as the blanket refusal that decision rejected. Found INDEPENDENTLY by two angles | security review + crypto audit | Distance rebuilt on read; store-round-trip test added. The existing grace test passed because it called the pure predicate — production goes through the store |
| **The grace anchor was poisonable upward.** An endpoint reporting `block_height = u64::MAX` earns `Behind`, pinning the anchor at `u32::MAX` permanently (the row survives a rescan), so every later `Unknown` computes distance 0 and signs forever | security review + crypto audit | Claimed tip clamped to `scanned_tip + REORG_MAX_BLOCKS`: over-reporting must age the wallet faster, never slower |
| **A fresh wallet's FIRST pass had no verdict.** `evaluate_consensus` ran only on the already-provisioned branch; the provisioning branch then scanned the whole span and published a clean `UpToDate` — §0 reproduced inside the mechanism built to prevent it | arch review | Moved before both branches; mutant-watched regression test |
| **`unknown_branch_evidence` accepted v3/v4 headers**, where those bytes are the transparent input vector — so an endpoint could choose which memos went dark for the process and forge the "unforgeable" evidence | crypto audit + security review | Requires an exact version AND version-group id PAIR — v5's or v6's, the two `TxVersion::read` accepts. INC-007, GUARDED at T0-4 by `only_a_whole_v5_or_v6_header_is_read_as_evidence`, which is the control that had to grow when the reader learned a second format: a v5 version under v6's group id (or the reverse) is refused, because the pair is what upstream matches |
| **`UpToDateLimited` keyed on `permits_signing`**, so a server merely omitting one field made the app claim the user's funds might be invisible | security review | Keys on `Unsupported` alone, via the new `blocks_interpretation()` |
| **The sign path had no arm for the new error.** The verdict can flip between Review and Confirm, landing a user in a generic "couldn't complete" with a Retry that cannot work | code reviewer | Arm added to `classifySendFailure` |
| **The queue offer reappeared when offline**, because the gate tests sync state and knows nothing about the fault | security review | Suppressed explicitly |
| **§8 claimed 11 tests that do not exist**, including the gate that would substantiate the one-predicate claim | crypto audit + security review | Rows marked OWED; `stale_binary_refuses_every_signing_path` written |
| **`walletStallNetworkUpgrade` had no producer** — 16 locales for a state the SDK cannot emit, with §9.1 claiming it wired | code reviewer | String CUT; §6.1/§9.1 corrected |
| A third reader re-derived "is this blocked" with an inline `matches!` outside the one-predicate module | code reviewer | `ConsensusCompatibility::blocks_interpretation()` |

**Open, recorded, not fixed:** `SigningPermit::granted` takes a caller-supplied
verdict, so it proves a token was minted rather than that the store was read;
`evaluated_at_unix` is never consulted, so a persisted `Current` never expires;
the verdict is not invalidated on endpoint change; `consensus_status()` has no
FFI binding, so the §3.3 cold read is unreachable from a host; the double
`get_lightd_info` on a fresh wallet's first pass; 11 §8 rows still OWED.
| `Unknown` passed through unconditionally — fail-open on the one field the design depends on | crypto audit | Escalated as the only founder decision; **answered 2026-09-06: degrade with age.** §6.3 grace rule + §11.2 record |
