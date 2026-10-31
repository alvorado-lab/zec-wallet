# 0580: Pause sending in the last 20 blocks before a network upgrade

- **Status:** Accepted, 2026-10-30. The founder's decision of 2026-10-30, in his words,
  chosen from the two options put to him (below).
- **Date:** 2026-10-30
- **Links:**
  - **Extends** the signing-time half of
    [ADR-0579](0579-wallet-nu7-crate-wave-and-the-mainnet-height.md) Decision 4 ("The
    signing-time half", ADR-0579:299-308): its branch clause stands, and the pause is
    added in front of it. The rest of Decision 4 (the kernel, the five readers, the
    burial, what the wallet says) stands too.
  - **Spec:** `docs/specs/wallet-sdk.md` §6.1 (the error row) and §6.3 (what "expired"
    means across a network upgrade). **Plan:** `docs/plan/nu7-wave.md` §3 step 4 ("The
    pre-upgrade pause") and §9 (the fold's record).
  - **Code:** `sdk/zec-wallet-core/src/send.rs` (`upgrade_pause`,
    `assert_no_upgrade_imminent`, their calls in `propose_core`,
    `propose_shield_with_threshold` and `create_signed_core`),
    `sdk/zec-wallet-core/src/wallet.rs` (`refuse_if_upgrade_imminent` and its callers),
    `sdk/zec-wallet-core/src/constants.rs` (`PROPOSAL_ANCHOR_DRIFT_MAX_BLOCKS`).
  - **ZIPs:** ZIP 259 (https://zips.z.cash/zip-0259), ZIP 318
    (https://zips.z.cash/zip-0318).

## Context

A transaction commits to the consensus branch it is signed under, and only blocks of
that branch can carry it (ZIP 259). Upstream builds a proposal at its target height, so a
send signed at the tip `H - k`, below an activation `H`, is under the ending branch and
has `k - 1` blocks, `H - k + 1` to `H - 1`, to be mined in.

ADR-0579 Decision 4's signing-time half refused only from the tip `H - 1`: the anchor
guard returns `ProposalStale` when the branch at the proposal's target differs from the
branch at `tip + 1`. Below that every send signed. One that missed its last blocks was
dead (the expiry kernel says so from `H - 1`), but the engine kept its notes reserved
until the transaction's stored expiry:

- an ordinary send: its stored expiry, `target + 40`, and then the 15,000 s burial
  (Decision 6) before the wallet sends the payment again;
- a canonical ZIP-318 crossing, a single payment of a `{1,2,5}×10^k` denomination (the
  shape "1 ZEC" produces): the rolling expiry, 34,561 to 69,120 blocks above the target,
  30 to 60 days. Upstream replaces the builder's expiry for it
  (`step_is_canonical_crossing`, `zcash_client_backend-0.25.0/src/data_api/wallet.rs:984`,
  applied at `:2010-2017`).

Step 4's crypto review (2026-10-30) found the code correct and three claims false at
their source: "nothing is signed under a branch that is ending" (the spec) and "a signed
tx retains at least 20 blocks of mining headroom" (`send.rs`, `constants.rs`). Both fail
across an activation.

## Decision

The founder, 2026-10-30, choosing "Pause ~25 min": "Refuse new sends in the last 20
blocks (about 25 minutes) before activation, with a clear message: 'The Zcash network
upgrades in about N minutes. Sending is paused until then so your payment can't get
stuck.' Sends work again right after. Removes the risk entirely and makes the existing
'20 blocks to be mined' promise true."

- **The rule.** With `K = PROPOSAL_ANCHOR_DRIFT_MAX_BLOCKS` (20, the drift guard's own
  headroom) and `tip` the wallet's recorded chain tip, a send is refused when the branch
  at `tip + 1`, the first block a new transaction can be mined in, differs from the branch
  at `tip + 1 + K`. Both are read through the crate (`BranchId::for_height`); `H` is the
  upper bound of the branch at `tip + 1` (`BranchId::height_bounds`), never the hand-kept
  `KNOWN_UPGRADES`. For an activation `H` the refused tips are `H - 21` to `H - 2`: the
  tip `H - 22` is the last allowed before it (21 blocks left), and the tip `H - 1`, whose
  next block is under the new branch, the first allowed after. Sends work again at once,
  unless another activation falls in the next 20 blocks (the pause then runs on, naming it).
- **Where.** At propose (`propose_core`, and `propose_shield_with_threshold`, which the
  shield and the sweep share) and at sign (`create_signed_core`, before the anchor guard,
  so a proposal made before the window and confirmed inside it gets the same refusal,
  never `ProposalStale`: a host re-proposes on that, into the pause, for up to 20 blocks).
  Every wallet path that pulls a seed checks first (`Wallet::refuse_if_upgrade_imminent`;
  the sweep again per address), so the user is not prompted for a credential and no key
  is derived for a send the pause refuses, unless the window opens during the prompt: a
  sync can record the tip `H - 21` after the early check, and the check under the lock
  (the create's, or the sweep's and the reclaim's propose) then refuses after the seed
  pull. The check inside the create, under the lock it holds, stays the authority. An OutOfZec swap execute asks the same check through its deposit port before
  the quote is claimed or the provider is asked to register anything.
- **The error.** `WalletError::UpgradeImminent { activation_height, blocks_left }`,
  `RW-SEND-009`, with `blocks_left = H - (tip + 1)`, 1 to 20; the bridge's
  `WalletErrorKind::UpgradeImminent` carries both. The swap execute's twin is
  `SwapError::UpgradeImminent`, `RW-SWAP-017`, with the same fields. Public protocol
  heights, no wallet data.
- **Who sees it, and who defers.** Propose (send, shield, move), the sign of a reviewed
  proposal, a parked row's "Send now", the recovery sweep and the reopen (reclaim) at
  their entry, and an OutOfZec swap execute return it, and the reference UI shows the
  founder's sentence with N = `blocks_left` at 75 s, in whole minutes rounded up (2 to 25,
  never 0). A window that opens mid-sweep is not an error: the sweep stops and returns
  what it swept, the address the window caught counted `failed`. The background paths
  defer: the queued drain leaves every row queued (counted `retried`) and sends after the
  activation; a swap deposit signed at execute stays queued the same way; the automatic
  shield stays quiet and shields at the next snapshot edge. Queueing is not signing and
  keeps working.
- **The branch clause stays.** It is what refuses a proposal made at the last allowed tip
  `H - 22` (target `H - 21`) and signed at `H - 1`, where the drift is exactly 20.
- **The headroom claim is now true, from the recorded tip.** The drift bound gives
  `tip <= target + 20` and the builder's expiry is at least `target + 40`, so the expiry
  admits every block from `tip + 1` to `tip + 20`; the pause and the branch clause put all
  of them under the branch the transaction is signed for.

## Alternatives considered

- **Allow and explain** (the other option put to the founder): let the send go and tell
  the user it may be held. Rejected: a send that misses its last blocks holds its amount
  until its stored expiry, up to two months for a crossing, and nothing the wallet says
  makes that cheaper.
- **Refuse with `ProposalStale`.** No new error, but a host re-proposes on it, and here a
  re-propose is refused again for up to 20 blocks: a loop on the user's screen.
- **Pause crossings only.** The user cannot tell a crossing from any other send, and an
  ordinary send that misses its window still holds its notes for its expiry and the
  burial, about four hours. Not put to the founder.
- **A different window.** 20 is the headroom the drift guard already promises; one
  number now governs both, so the promise is true across an activation as within a
  branch.

## Consequences

- Every network upgrade costs about 25 minutes without sending (20 blocks at 75 s). For an
  upgrade after NU7 the blocks before it are 25 s, so the pause is about 8 minutes, and the
  reference UI, which counts at 75 s, would show three times the wait until it is changed:
  a stated limit.
- The headroom is measured from the recorded tip. Two residuals remain. The first is a
  class: any sign before the first sync since the chain moved (a resume, an old database
  opened, a device whose sync keeps failing), where the tip is stale, so the window is
  invisible. The second is a server whose reported tip is wrong. A transaction signed
  inside the window by an older build, or by another wallet on the same seed, is still
  handled by Decision 4's kernel.
- A false tip errs the other way too, and for longer. A single server tip inside
  `[H - 21, H - 2]` keeps sending paused until the real chain reaches `H - 1`, because the
  recorded tip does not come down: `chain_height()` is the scan queue's highest end
  (`zcash_client_sqlite-0.23.0/src/wallet.rs:3464-3474`), and a later, lower tip either
  returns early below the max scanned height (`wallet/scanning.rs:643-646`) or replaces
  only the queue up to itself (`replace_queue_entries`, `:146`), leaving the false range
  above it. A lie told a day before the activation pauses sending for that day, and the
  copy's "about N minutes" stays at the false tip's N until the real chain passes it.
- A send signed at or before `H - 22` that is not mined within its 21 or more blocks
  before the activation (the device offline, or its broadcast failing for about 26
  minutes) dies at `H`, and a crossing's notes are then held to its rolling expiry. The
  pause narrows that window to a missed broadcast; it does not close it.
- A swap deposit whose quote lapses during the pause is purged unsent; the user re-quotes.
- New and frozen: the codes `RW-SEND-009` and `RW-SWAP-017`. The bridge ABI stays 12,
  unshipped, with its fingerprint re-recorded (row 7's precedent). Three new strings in 16
  locales. The sibling retrying sentence keeps the translations zec_wallet_ui 0.0.3
  already published.
- ADR-0579's links carry the back-annotation pointing here (added at the landing,
  2026-10-31).
