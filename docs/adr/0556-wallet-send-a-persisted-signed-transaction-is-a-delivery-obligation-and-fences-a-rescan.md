# 0556 — Wallet send: a persisted signed transaction is a delivery obligation — rebroadcast until confirmed or expired, and a rescan is refused while one is owed

- **Status:** Accepted (built and landed in the same session — stage S8
  `obligation`, S293, `docs/plan/stage-8-payment-identity-and-durable-retry.md`
  §3.2; the ruling is `docs/adjudication/s8-obligation/ruling.md`).
- **Date:** 2026-09-20
- **Links:** `docs/specs/wallet-sdk.md` §6.2–§6.3 (the sentence at
  `:7577-7582` — "a SINGLE-step interactive send keeps NO outbox row (its one tx
  self-heals via expiry → notes free → re-send)" — is what this ADR reverses) ·
  `:11349-11356` (the manual-resend double-pay residual, STRUCK by this ADR,
  never deleted) · `docs/reviews/2026-09-20/production-readiness-review.md`
  R02 · `docs/plan/audit-2026-09-20-remediation.md` §2b · **extends
  [ADR-0529](0529-shield-flow-synchronous-engine-witnessed.md)**
  (the shield's crash-safety stays engine-witnessed for the double-spend guard;
  its DELIVERY is now owed under this ADR — 0529 is not superseded) · FR-26
  (`WalletSendTransactionCreated.broadcastCount` — its doc becomes true) ·
  FR-29 (the rebroadcast rides the host's dial policy)

## Context

The ordinary send path — `propose` → `send` → `sign_proposal_upstream` →
`broadcast_persisted` — persists the signed transaction through the engine's
own write (raw bytes, `expiry_height`, `created`) and then broadcasts.
`broadcast_persisted` returns `Ok` on total failure (`TxSubmitResult` has no
"enrolled" bit). The retry pass, `resubmit_queued_sends`, reads only the aux
`queued_send_intent` table; a single-step transfer or shield enrols no row
THERE BY DOCUMENTED DESIGN, so **the wallet's `transactions` table is never
enumerated for delivery** and a signed send whose broadcast failed — network
down, private path blackholed, app killed between signing and sending — is
never retried, while the UI says "saved for retry; your wallet will send it on
a later sync" for an empty result list. This is R02, a permanent RED probe by
ruling (`wallet::tests::a_persisted_signed_send_is_retried_after_reopen_as_the_ui_promises`).

Two facts price the mechanism. **One file, two connections:** `aux_db` is a
second SQLCipher-keyed connection to the SAME `wallet.db`; the engine's
`WalletDb` exposes no connection accessor, so no single transaction spans the
engine's persist and an aux write — a death between them is exactly the
window. **A user-driven rescan REBUILDS, it does not rewind:**
`reset_data_db_keep_seed` provisions a fresh DB and copies only
`AUX_TABLES_PRESERVED`; the `transactions` table, raw bytes and all, is
DISCARDED, and the fence that guards it reads only intent rows — so today a
rescan proceeds through a single-step send's broadcast-but-unmined window and
the witness inverts (the residual at `:11349-11356`).

## Decision

**A persisted signed transaction IS a delivery obligation, and the obligation
is not a second write a crash can lose: it is DERIVED from what the signing
persistence already committed.** The delivery phase enumerates the engine's
wallet-created (`created IS NOT NULL`), unmined, unexpired `transactions` rows
that no intent row can own — exclusion by intent ATTRIBUTION (the notes the
transaction spends, the intent's claim written before or at signing and
preserved across a rescan), never by recorded txid alone, because of the
create-committed-but-unrecorded window — and rebroadcasts their raw bytes on
every later sync and after a reopen until the transaction confirms or expires.
The intent machinery keeps sole ownership of ordered groups, of deposit
deadlines and of the deposit hold: a signed swap deposit whose intent row is
alive in `DepositBroadcastHold` (both arms) is never broadcast by the generic
phase.

**Every mark added beside the obligation states its loss mode.** An *accepted*
mark (txid, height, time) is a best-effort aux write whose loss costs one
idempotent rebroadcast — harmless. Nothing load-bearing is a new mark.

**The rescan fence reads this enumeration on the engine connection, before the
destructive step**, and refuses a rescan under a broadcast-but-unmined
single-step send, typed `RescanWithInFlightSend`. A failed broadcast is
rebroadcast, never re-signed: the engine's spent-marks refuse the notes to a
fresh `propose` while the first transaction is unexpired; expiry frees them and
a re-send after expiry is a NEW payment the user initiates.

**Four states per transaction** — persisted / retry-pending / accepted /
confirmed — are readable on the core API and across the bridge where the
history reads transaction status; NOT on the FR-26 report (consumed once,
frozen; `broadcastCount` already carries the flow-end bit). "Saved for retry" is
shown only when the core reports retry-pending.

A rebroadcast rides the same dial policy as the first broadcast: under
`Required` zero clearnet dials, ever; a fresh isolation key per attempt.

## Alternatives considered

- **Enrol an aux intent row at signing (alone).** A second write on a second
  connection; a death between the engine's commit and the INSERT is R02
  itself, and `evals/mutants.tsv` already names it as the probe's post-fix
  mutant ("commit it in a separate transaction after the bytes"). RULED OUT
  alone; admissible only with the derived enumeration as the source of truth
  and the fence reading it.
- **A wallet-DB side table in the engine's own transaction.** Atomic by
  construction — and blocked, not by "two databases" (there is one file) but by
  `WalletDb` exposing no connection accessor. Admissible the day that upstream
  seam exists.
- **Skip recorded txids.** Misses the unrecorded-txid window (`mark_submitting`
  recorded the claim, the process died before `mark_sent_multi`). Attribution
  by the intent's claim is the rule instead.

## Consequences

- Crash-safety by construction for the single-step transfer and shield (the
  engine's persist is the only write on that path) and ONE obligation semantics
  for every kind of send.
- Bounded redundant broadcast traffic between acceptance and confirmation; a
  second broadcast of accepted bytes is `Success` or the mempool-duplicate
  outcome, never a second payment.
- **Availability residual, widened and stated:** a single-step send blocks a
  rescan until it mines or expires (`DEFAULT_TX_EXPIRY_DELTA`, 40 blocks ≈ 50
  min), where today only outbox classes did. `:11349-11356` is struck; a host
  that skips `zec_wallet_ui` and calls `rescanFrom` sees the same refusal — a
  new reason a "scan all history" affordance can be refused (Relim's Expert
  wallet-settings screen needs a sentence, at our landing; sync point 2).
- The `created IS NOT NULL` discriminant is an unpinned third-party semantic
  (`zcash_client_sqlite` 0.22); a row pins it — a received transaction is never
  enumerated.
- Frozen: `:7577-7582` reversed; `api/wallet.rs:1030-1056` rewritten; the
  strings `walletSendSavedTitle/Body` stay but become conditional; the FR-26
  report's shape does not change.

## As built (S293; `docs/adjudication/s8-obligation/ruling.md`)

- **The enumeration** is `crate::delivery` (`sdk/zec-wallet-core/src/delivery.rs`):
  `enumerate(conn, scanned_tip)` selects the engine's wallet-created
  (`created IS NOT NULL`), unmined, unexpired `transactions` rows that have raw
  bytes, MINUS every transaction an intent row can own — by the txids the
  in-flight and stranded rows recorded AND by the spenders of a live claim's
  notes (the create-committed-but-unrecorded window) — never by recorded txid
  alone. `DeliveryView::load` is the one reading behind the fence, the pass and
  the history page.
- **The rebroadcast** is Phase 3 of `resubmit_queued_sends`, after Phase 2 (so
  a row Phase 2 just recorded is the intent path's): each owed transaction is
  its own one-transaction group through the SAME `broadcast_outbound` →
  `broadcast_group` → `broadcast_one` path — the host's dialer and policy, a
  fresh broadcast isolation key per attempt, the attempt cap. The
  `wallet.resubmit` line gains `owed=<n>`.
- **The one mark** is the accepted mark in the aux table `tx_delivery_accepted`
  (created idempotently at open, preserved across a rescan), written by
  `broadcast_one` on `Success`; a write fault is the WARN `wallet.delivery_mark
  outcome=<code>`. No broadcast decision reads it: an accepted-but-unmined row
  is rebroadcast every pass until mined or expired (idempotent), and the mark's
  loss costs only the reading.
- **The fence** in `rescan_from` reads `delivery::enumerate` on the aux handle
  to the same file, after the quiesce and while the engine connection is still
  open, before `drop(engine_conn)`, against `history::scanned_tip` — the
  existing `RescanWithInFlightSend` kind, a new trigger.
- **The four states.** `DeliveryState { Persisted, RetryPending, Accepted,
  Confirmed }` (`#[non_exhaustive]`, re-exported at the crate root),
  `TxSummary.delivery: Option<DeliveryState>` (`None` for a received or
  expired row), `Wallet::delivery_state(txid)`; on the bridge the same enum
  plus `Unknown`, `TxSummary.delivery`, `WalletHandle.deliveryState({txidHex})`.
  `RetryPending` is the reading of an owed transaction from the moment it is
  persisted (the adjudicator's ruling: an "attempted" mark the contract never
  named is not a state); `Persisted` is the no-promise reading — a held swap
  deposit, the unrecorded-create window.
- **No new death-point seam.** `send_by_id` is `sign_proposal_upstream` →
  `broadcast_persisted` with nothing between; the existing `#[cfg(test)]
  Wallet::sign_proposal` is persist-without-broadcast, and
  `delivery::tests::the_persist_half_of_a_send_has_exactly_its_two_callers`
  pins the production caller count in `connect_unchecked`'s shape.
- **The UI.** `summarizeSendOutcome(delivery:)` produces `SendSavedForRetry`
  only when the core reports `retryPending` for every transaction that did not
  go out; otherwise the new `SendKept` ("Saved" — kept, no promise); Activity
  reads "Retrying" for an owed row and "Saved" for a held one; accepted stays
  "Pending".
- **Ruling:** TEST_WRONG on two assertions (a stop at acceptance, which §1 and
  row 6 forbid; an attempted mark the contract never named) and CONTRACT_WRONG
  on the probe's witness clause (`before == after`, "zero retry dials"), all
  repaired the same session; 29 mutants caught, the 2 survivors re-planted.
