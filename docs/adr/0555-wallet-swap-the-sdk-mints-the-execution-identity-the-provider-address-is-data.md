# 0555 — Wallet swap: the SDK mints a swap's execution identity; the provider's deposit address is data inside the record

- **Status:** Accepted (built and landed in the same session — stage S8
  `identity`, S293, `docs/plan/stage-8-payment-identity-and-durable-retry.md`
  §3.1; the ruling is `docs/adjudication/s8-identity/ruling.md`).
- **Date:** 2026-09-20
- **Links:** `docs/specs/wallet-sdk.md` §3.5 (the swap port; the sentence at
  `:1765` — "the response's `depositAddress` is the live swap handle and the
  `SwapId`" — is what this ADR reverses) · `:6212-6220` (the least-authority
  claim, true of binding / refund / watch and FALSE of the amount within one
  process) · `docs/reviews/2026-09-20/production-readiness-review.md` R01 ·
  `docs/plan/audit-2026-09-20-remediation.md` §2b · FR-17 (`SpendBinding`,
  `seed.rs:206-227`: an opaque nonce over proposal identity, NOT an amount
  binder) · ADR-0032 (Relim ships the swap flow at v1.0b — the error and its
  copy reach their users through our screens)

## Context

`SwapId` is minted from the PROVIDER's `deposit_address`
(`zec-wallet-swap-near/src/map.rs:437`, `:488`; the adapter doc at `:484-487`
ASSERTS it is fresh per quote, which nothing enforces). That one value is not
only the store key: it is the wire value (`/v0/status?depositAddress=<id>`),
the poll key (five consecutive not-founds synthesise a TERMINAL `Failed`), the
kill→relaunch re-attach handle (`SwapRecord.id`), the `swap_record` home row's
key (the refund watch rides it), and the input to the per-swap
circuit-isolation key (`key_for` = HMAC over it).

`quote_inner` persists a durable `IssuedQuoteRecord` keyed on that id
(first-wins on a duplicate) and inserts an in-memory `IssuedQuote` into a
`HashMap` that OVERWRITES on a duplicate. `execute_inner` takes binding /
refund / watch from the durable row and the deposit PLAN from the in-memory
entry when one exists. So a provider that answers two quotes (100,000 then
900,000 zat) under one address makes `execute(&first)` pay 900,000 under the
100,000 approval — FR-17's binding matches and catches nothing, and after a
restart the same call pays 100,000: which terms run depends on process
lifetime. This is R01, a permanent RED probe by ruling
(`swap::service::tests::a_provider_reusing_a_quote_id_cannot_change_the_amount_the_user_accepted`).

Refusing a duplicate provider address at `quote()` is a courtesy, not the
guard: a pruned or consumed address can be reissued, and a stale DTO could
claim it.

## Decision

**The SDK mints a swap's execution identity when the quote is issued — `OsRng`,
≥ 128 bits, independent of quote content — and that identity is the ONE key**
by which the record is stored, approved and consumed, by which the swap's home
row and refund watch are kept, and from which its circuit-isolation key is
derived. **The provider's deposit address is DATA inside that record**
(`provider_ref`): the value the adapter is handed for its request URL and body,
and nothing else. It is resolved from the durable `swap_record` row BEFORE a
status-poll task is spawned — never inside the task, never from an in-memory
cache that a reopen empties.

The durable record carries every execution term the approval showed — address,
amount (for an IntoZec quote: `zec_side` / `min_amount_out`), deadline, binding,
refund, watch — and the caller's DTO is compared against the record
field-by-field before the record is consumed (defence in depth): a DTO whose
terms differ from the record it names executes nothing, with a typed error, and
the record is not consumed by the refusal. Once the key is SDK-minted, a
duplicate is an invariant violation: the durable store's first-wins
short-circuit and the home row's `INSERT OR IGNORE` become typed errors.

The host-visible namespace is unchanged in TYPE and changed in VALUE:
`SwapQuote.id`, `swapExecute`'s return, `SwapRecord.id` and `watchSwapStatus`'s
argument are one opaque SDK handle. The §5.4 NEVER-log marker moves with the
provider address to the field that carries it.

## Alternatives considered

- **Keep the provider address as the key and refuse duplicates at `quote()`.**
  Loses the reissued-after-prune case and the stale DTO; it is the "guard" the
  design review said not to rely on. Ruled out by the premise.
- **A content-hash id.** Two quotes with identical terms would collide — the
  mutant that reds the "minted, not derived" row. Ruled out.
- **Bind the amount into `SpendBinding`.** Would make FR-17 carry a second
  meaning; the record already holds the terms, and the compare is the cheaper,
  more legible guard.

## Consequences

- Restart invariance: which terms run no longer depends on process lifetime;
  the in-memory registry is at most a cache and decides nothing (the spec's
  `:6212-6220` claim becomes true of every term, not only of binding / refund /
  watch).
- Two swaps under one provider address are two records, two home rows, two
  refund watches and two isolation keys (against a passive exit only — quotes
  already share `ISOLATION_KEY_QUOTE`; the money legs carry the weight).
- Priced: the monotonic `act_deadline` (suspend-safety) moves to the durable
  row or is re-derived; `SwapPort::execute`'s return and `SwapPort::status`'s
  argument change across every fake (the adapter must receive the provider
  address as data); the adapter's per-instance HMAC secret means the isolation
  key is not stable across restarts either way (unchanged).
- Frozen: the spec sentences at `:1765` and `:6212-6220` and the four doc
  strings asserting "`id` = the provider deposit address" are rewritten in the
  fold; a new typed "terms differ" error with copy in 16 locales (machine copy
  at beta) reaches Relim users through `zec_wallet_ui`'s screens (sync point 2).

## As built (S293; `docs/adjudication/s8-identity/ruling.md`)

- **The port shape: both values.** `SwapPort::execute -> Result<(), SwapError>`
  (the adapter no longer returns an id — the id is the SDK's) and
  `SwapPort::status(&SwapId, provider_ref: &str)`; the adapter's inherent
  `notify_deposit_tx(&SwapId, provider_ref, tx_hash)` likewise; `key_for` takes
  the SDK id, so two swaps under one provider address ride two circuits while
  the request line is unchanged (`/v0/status?depositAddress=<provider_ref>`).
- **The pre-spawn resolution lives in the service.** `SwapService::watch_status`
  became `async fn … -> Result<(), SwapError>` with a pre-step through
  `SwapRecordSink::provider_ref` (backed by `swap_record_store::provider_ref_of`)
  BEFORE `tokio::spawn`; the poll task captures the provider `Arc`, the resolved
  provider address and the kill receiver only. An id with no in-flight home row
  is `RequestInvalid` before the stream opens (it used to poll to the not-found
  terminal).
- **The identity.** `SwapId::mint()` — 16 bytes of `OsRng`, 32 lowercase hex —
  in `SwapService::quote` after `validate_quote` and the binding mint. The
  provider's deposit address is `provider_ref` on `IssuedQuoteRecord`,
  `StartedSwap` and the `swap_record` home row (a nullable column, ALTER TABLE
  at open), carrying the §5.4 NEVER-log marker on every field that holds it and
  never exposed on the FFI DTOs.
- **The terms.** `QuoteTerms` on the issued record — deposit address and memo,
  `amount_in`, `min_amount_out`, `zec_side`, `refund_to`, and the `expires_at`
  the approval showed, verbatim (the adjudicator's repair: the first build
  excluded the deadline; the Behaviour paragraph governs). `execute` does a
  non-consuming `IssuedQuoteStore::peek`, compares the DTO's terms and binding
  field-by-field, and refuses with `SwapError::QuoteTermsDiffer` (`RW-SWAP-016`;
  bridge `SwapErrorKind::QuoteTermsDiffer`, inserted before `Unknown` — the
  reason for the stage's ABI bump) before `take`, with zero provider or deposit
  calls and the record intact. The wall gate runs on the peeked row first, so
  a born-expired honest DTO still reads `QuoteExpired`.
- **A duplicate SDK id is the typed invariant error at both durable stores**,
  detected BEFORE the cap (the adjudicator's repair: the first build reported
  a duplicate at capacity as `AtCapacity`, a routine cap message for an
  invariant violation).
- **Upgrade posture (dev stage, no migration):** an issued row without terms
  reads as never issued (`QuoteExpired`, re-quote); a home row without a
  `provider_ref` refuses `watch_status` with `RequestInvalid` rather than guess.
- **Ruling:** IMPL_WRONG on the three points above (the never-log marker on
  `StoredQuote.provider_ref`, duplicate-before-cap, the deadline in the
  compare), each repaired the same session; the R01 probe's precondition line
  (`first.id == second.id`) compares deposit addresses since the fold.
