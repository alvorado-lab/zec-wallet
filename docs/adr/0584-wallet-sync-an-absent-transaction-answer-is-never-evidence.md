# 0584: An endpoint's "no such transaction" is never evidence; a money question is never deleted by an answer

- **Status:** Accepted 2026-11-26. The design was reviewed on 2026-11-24 (security, arch and crypto,
  then the code reviewer; revision 2 confirmed by crypto). The built diff passed its review on
  2026-11-25, the condition this status set. The exposure's exact scope is in the spec's §3.3
  limits: it is unchanged for a server that answers NotFound, and a request that an empty body or
  an unmined answer past the expiry used to end now stands.
- **Date:** 2026-11-24
- **Links:**
  - **Contract:** `docs/plan/wl1-absent-answers.md` (clauses C1 to C7; §(h) the review).
    **Spec:** `docs/specs/wallet-sdk.md` §3.3 ("AN ABSENT ANSWER IS NOT EVIDENCE") and §8.
    Also `docs/specs/sync-server-picker.md` D3 (what a switch carries) and
    `docs/specs/ironwood-nu63-support.md` §9.1 step 7 (the unparseable set).
  - **Code:** `sdk/zec-wallet-core/src/enhance.rs` (`select_enhancement_targets`,
    `EnhancementCursor`, `run_enhancement_pass`, `chain_status`),
    `sdk/zec-wallet-core/src/wallet.rs` (`Inner::enhancement_cursor`, `enhance_transactions`).
  - **Extends**
    [ADR-0579](0579-wallet-nu7-crate-wave-and-the-mainnet-height.md) Decision 4's reader 5:
    the endpoint status check asks the expiry kernel on its unmined arm too.
  - **Measured:** `docs/plan/probes/unknown-txid-reply-probe.*` (2026-11-24).

## Context

Compact blocks leave out memos, so after a scan the wallet asks the server for whole transactions
by txid, at most 50 per pass. A request stays in the engine's queue until something answers it.
There are two kinds: an Enhancement ("give me the transaction") and a Status ("what does the chain
say about it?"). Four things were wrong.

1. **An absence was written down as a fact.** For an empty body, for either kind, the pass recorded
   `TxidNotRecognized`. On that write the engine always deletes the Enhancement row, and the Status
   row unless the transaction is unmined and still alive
   (`zcash_client_sqlite-0.23.0/src/wallet.rs:4036-4086`). The memo is then lost until the block is
   scanned again.
2. **Fifty requests nobody serves starved every later one.** The engine lists the requests in one
   fixed order and the selector took the first 50. A refused or unserved request stays and is asked
   again each pass, so 50 of them held back the rest for good.
3. **One unmined answer past the expiry ended the question.** At a tip at or past the stored expiry
   the engine's arm for `NotInMainChain` deletes the Status row for good. For the second leg of a
   two-step (TEX) payment that row is the one thing that can teach the wallet the leg was mined.
   Without it a delivered leg reads `Stranded` once both legs' expiries are buried, and the user may
   pay again.
4. **A forged header hid a Status request until the next launch.** A reply naming a consensus branch
   this build lacks puts the txid in a process-lifetime set, and the skip that reads the set applied
   to both kinds.

What servers answer, measured on six public ones (lightwalletd v0.6.2 over Zebra or Zakura): an
unknown txid comes back as the gRPC status NotFound, and none sends an empty body. Read at
lightwalletd's source, not measured: its current code answers NotFound for ANY failed
`getrawtransaction`, a backend outage included, and v0.4.17 answers code Unknown. Zaino servers were
unreachable and are unmeasured. So an honest server does not trigger the loss in (1) today; a
hostile one can.

Who has a Status request, at this engine version: a send that scanning cannot see, or a stored
un-mined transaction in which the wallet decrypted no output, plus rows an engine migration kept. In
this wallet that is the TEX second leg and some transparent transactions. An ordinary shielded send
has none.

## Decision

- **Absence writes nothing.** For a request of either kind an empty body, a gRPC status, a transport
  failure, a timeout and a reply refused at the §4.6 boundary write nothing, and the request stays
  pending. The pass writes in two places only, both after the reply's txid was matched:
  `store_decrypted(tx, None)` and `set_status` of a `chain_status` answer. `CheckedStatus` loses its
  `not_recognized` constructor, so no code can write `TxidNotRecognized`.
- **The gRPC seam stays as it is.** An empty body is `Ok(None)`, NotFound is an error like any other
  status, and no code keys on the status number.
- **An unmined answer past the effective expiry is not written** (`EffectiveExpiry::is_past` at the
  known tip). Before it, `NotInMainChain` is written as before. The effective expiry is never above
  the stored one, so for a transaction still un-mined this covers every tip at which the engine
  would delete the row. Nothing reads the height the write would have left for money or for a
  rendering. The refusal counts as rejected and logs nothing, because it can repeat every pass; the
  warning stays for a refused height.
- **The batch rotates.** The eligible requests are ordered by txid, and the batch starts strictly
  after the last txid asked, wrapping, before the cap. The cursor moves before each fetch is
  awaited, so a pass dropped in the middle of a fetch, or ended by a store fault, does not ask the
  same txid first again. It is born at a random txid, so a host that runs one pass per wake-up does
  not always ask the same 50. It lives in memory with the open wallet, and a server switch, a
  rescan and a reopen start over.
- **The unparseable set is for enhancements only.** A txid joins it on an Enhancement reply, and the
  skip applies to Enhancement requests. A Status row exists only for a transaction this build
  already built or parsed, and its txid commits to its branch (ZIP 244), so a Status reply naming
  another branch is a refused reply like any other.

## Alternatives considered

- **Map NotFound to `Ok(None)` at the gRPC seam.** It would put an outage behind a type that reads as
  an answer, and with the old write it would make the memo loss reachable with honest servers.
- **Count failures per txid and back off** (revision 1 of the contract: past ten, ask once per 600 s
  of chain time). The design review broke it in four ways: the wait can outlast the `Stranded`
  decision, a 200 s outage exhausts every request because NotFound counts, a server drives the
  count, and "exhausted" is not a safe signal for anything else to read. A wait needs a bound, and
  a counter is the server's to move, so there is neither.
- **End the pass on a link failure**, so a hung server does not cost up to 50 timeouts in one pass.
  lightwalletd v0.4.17 answers an unknown txid with a code this wallet reads as a link failure, so
  a few unserved rows would end the pass just before the one in which a leg reaches its burial.
- **Ask Status requests first.** With more than 50 of them (one received transaction with 50 old
  transparent inputs makes that many) they would starve every memo.
- **Retry only for a transaction the wallet holds as mined** (the ticket's first idea). It leaves the
  same write in place for every other row, and a TEX leg is the row that carries money.
- **Keep the cursor on disk, or carry it across a server switch.** It is a fairness position, worth
  nothing after a restart, and a new server has not been asked anything.

## Consequences

- No memo request is lost to an absent answer, and a list longer than 50 converges for every
  request. Over the passes that run to their end each is asked at least once in every
  `ceil(n / 50)`.
- A two-step payment can no longer lose its last question to one answer past its expiry.
- **The cost, stated.** A request nobody ever serves stays in the queue for the wallet's life, one
  row, and is asked in its turn, each pass while the list is at most 50, exactly as with honest
  servers before. Each ask names one of the wallet's own txids to the server. That exposure is
  unchanged.
- Above 50 pending, a TEX leg's status is asked every `ceil(n / 50)` passes, a few minutes at worst
  against the 15,000 s burial.
- A transparent parent older than the birthday is answered and refused every time it is asked.
- A server that hangs on every ask still costs up to 50 timeouts of 30 s in one pass, as before.
- A forged header can still hide a memo until the next launch. It can no longer hide a status.
- Nothing is re-queued for memos already lost: no measured server sends the reply that loses them.
- No bridge, ABI, Dart or schema change, and no new log field.
