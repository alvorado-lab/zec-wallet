# Upstream wire contract — NEAR Intents 1Click API

This crate is a hand-audited client for a third-party API; this file records
exactly WHICH upstream reality the code and fixtures are pinned to
(testing-patterns §5: per-adapter `UPSTREAM.md`, fixtures over hand-typed
fakes; spec §8 row `one_click_wire_contract_pinned_by_fixtures`).

- **API base:** `https://1click.chaindefuser.com`
- **Endpoints consumed (the whole surface):**
  `GET /v0/tokens` · `POST /v0/quote` · `POST /v0/deposit/submit` ·
  `GET /v0/status?depositAddress=…`
- **Auth:** optional `Authorization: Bearer <JWT>` (integrator credential;
  without it the provider levies its 0.2 % fee as an auto-injected
  `appFees` entry — visible in the recorded quote fixtures).
- **Schema reference:** OpenAPI document **0.1.10** as embodied by the
  official generated SDK `defuse-protocol/one-click-sdk-rs` @ `master`
  (`v0/src/models/*`, retrieved 2026-06-13). Per ADR-0525 that SDK is
  REFERENCE ONLY — never a dependency.
- **Contract verified against the live API:** 2026-06-13.

## Fixture provenance

`tests/fixtures/recorded/` — byte-exact responses from the LIVE API,
recorded 2026-06-13 with dry/read-only calls (no provider-side state was
created; live non-dry confirmation is the W-swap-2 dry-quote E2E and the
maintainer-gated funds run):

| fixture | how |
|---|---|
| `tokens.json` | `GET /v0/tokens` (188 assets, 43 KB — also pins the body-cap sizing) |
| `quote_dry_out_of_zec.json` | `POST /v0/quote` dry=true, ZEC→USDC(near), EXACT_INPUT 1 ZEC |
| `quote_dry_into_zec.json` | `POST /v0/quote` dry=true, USDC(near)→ZEC |
| `quote_bad_request_400.json` | `POST /v0/quote` with missing fields — the NestJS error envelope; ALSO documents "amount must be … base units … integer string" verbatim |
| `status_not_found_404.json` | `GET /v0/status` with an unknown depositAddress — NOTE: the message ECHOES the queried address, which is why provider error bodies are never read or logged (§5.4) |

`tests/fixtures/derived/` — shapes that CANNOT be recorded without creating
provider-side state or running real swaps. Derived 2026-06-13 from the
recorded dry responses + the OpenAPI 0.1.10 models (field-for-field), and
labeled as such:

| fixture | derivation |
|---|---|
| `quote_live_out_of_zec.json` | recorded dry quote + the three non-dry fields (`depositAddress`, `deadline`, `timeWhenInactive`) per the `Quote` model |
| `quote_deposit_memo.json` | ditto + `depositMemo` — the funds-loss refusal arm |
| `status_*.json` | `GetExecutionStatusResponse` model around the derived live quote; per-arm `swapDetails` per the `SwapDetails` model |
| `status_unknown_variant.json` | an INVENTED future status word (`SETTLING`) — the closed-enum rule's named proof |
| `status_over_deposit.json` | an INVENTED over-deposit edge (`depositedAmount` > `amountIn`) — proves `missing` saturates to `"0"` instead of wrapping (money-honesty fold) |

## Cross-checked against a second independent 1Click consumer

The wire contract is corroborated by **Zodl/Zashi** (`zodl-inc/zodl-android`,
code-read 2026-06-13), the leading ZEC wallet: its `KtorNearApiProvider`
(`ui/common/provider/NearApiProvider.kt`) is a hand-rolled REST client over
the SAME four endpoints (`/v0/{tokens,quote,deposit/submit,status}`), with a
closed `SwapStatus` enum and transparent-deposit-address modeling — an
independent implementation of the same surface, not a shared SDK (it does
NOT use `one-click-sdk-rs` or `@defuse-protocol/intents-sdk`). Two unrelated
wallets converging on this endpoint set is a second source on the contract
beyond our own recordings.

## Observed drift (why the closed-enum rule exists)

The live API already validates `depositType`/`refundType`/`recipientType`
against `ORIGIN_CHAIN, INTENTS, CONFIDENTIAL_INTENTS` — the third value is
unknown to the 0.1.10 generated SDK. Provider drift is expected; it must
fail our suite (fixture KATs) or surface typed (`Failed(ProviderProtocol)`,
`UnexpectedHttpStatus`), never reach production behavior silently.

## Live ground fact: `deadline_window_secs` is IGNORED (device e2e)

Observed against the live API with real (non-dry) quotes on a device: the
provider accepts our `deadline` request field without a 400 but does NOT
honor it — an OutOfZec quote requested with the 15-minute window came back
with `deadline` ≈ **72 h** out, an IntoZec quote requested with 24 h came
back ≈ **96 h** out. Consequences the core already carries (W-swap-4-a-3/-4;
spec §4.4): the wallet-side ceiling clamps on the durable deposit tag AND
the stored quote deadline are **load-bearing enforcement on the real API**,
not defense-in-depth — without them every guard/zombie lockout would run
3–4 days — and the raw `expires_at` echo must never drive user-facing
countdowns (the display-alignment work is #367). Re-check this fact at any
upstream bump (checklist below): if the provider starts honoring the
requested window, the clamps become no-ops, which is fine; if it starts
rejecting the field, the adapter must surface that typed.

## Re-verification checklist (any upstream bump)

1. Re-record every `recorded/` fixture (same dry/read-only calls).
2. Diff against the previous bytes; any field change → review the wire DTOs
   and `map.rs` bounds BEFORE updating expectations.
3. Re-derive `derived/` only from the new recorded bytes + the bumped
   OpenAPI models; update the version pin above.
4. Run the W-swap-2 dry-quote E2E against the live API over Tor.
