# 0535 — The stranded-ephemeral surface reads the engine's RECOGNISED transparent balance (decoupled from the intent row); the engine offers NO ephemeral-reservation reclamation, so leaked-index cleanup is app-layer and deferred to gate-removal

- **Status:** Accepted
- **Date:** 2026-06-28
- **Extends:** the §3.2i-2 2e-2b decomposition (S132 engine probe) and ADR-0528 (transparent receive on external index 0 — the EPHEMERAL scope here is a DIFFERENT, ZIP-320 one-time scope). Pins the engine semantics the 2e-2b-i DETECT (S133, `7222101`) and this 2e-2b-ii stranded surface rest on. Does not supersede any ADR.
- **Links:** docs/specs/wallet-sdk.md §3.2i-2 (2e-2b mechanism + decomposition) · ADR-0005 (audited crates WHOLE — Rule Zero) · ADR-0526 (NetDialer transport / per-circuit isolation) · ADR-0530 (the SCOPED transparent poll the detect's circuit isolation mirrors). Pinned crates: `zcash_client_backend 0.23.0` / `zcash_client_sqlite 0.21.0` / `zcash_keys 0.14.0` / `zcash_transparent 0.8.0`.

## Context

A TEX / ZIP-320 send unshields shielded notes to a wallet-controlled EPHEMERAL
transparent address (tx0) and forwards them to the exchange (tx1). Two money-real
failure modes leave funds sitting on that one-time address:

- **STRAND** — tx0 mined (notes left irreversibly) but tx1 EXPIRED un-mined. The
  outbox can never complete it (re-propose is structurally impossible post-tx0);
  the reconcile machine moves the row to terminal `Stranded` (§3.2i-2, round-2
  BLOCKER #1).
- **exchange RETURN** — the exchange bounces the deposit back to the same one-time
  address (compliance/manual-review, days–weeks later).

The S132 engine probe established (a) the detect is an EXPLICIT, ISOLATED poll
(neither `scan_cached_blocks` nor the engine's `refresh_utxos` ever touch the
EPHEMERAL scope), shipped as 2e-2b-i. This ADR pins the READ side (2e-2b-ii: the
user-facing recoverable-amount surface) and the no-reclamation finding (b) that
constrains gate-removal (2e-2b-v).

A `zcash_client_backend 0.23.0` / `zcash_client_sqlite 0.21.0` API probe
(2026-06-28, this slice) settled the read mechanism:

- **`WalletRead::get_transparent_balances(account, target_height, confirmations_policy)`**
  returns `HashMap<TransparentAddress, (TransparentKeyOrigin, Balance)>` (data_api.rs:1928).
  The `zcash_client_sqlite` impl applies the `excluding_wallet_internal_ephemeral_outputs`
  predicate (transparent.rs:~1019) — a 3-way OR that INCLUDES an ephemeral-scope
  output ONLY when (1) the funding tx has NO wallet inputs (an exchange RETURN) OR
  (2) the output is observed unspent PAST the funding tx's expiry height (a STRAND),
  and EXCLUDES the normal in-flight case (own tx0, unexpired tx1). So
  `get_transparent_balances` filtered to `TransparentKeyOrigin::Derived { scope: EPHEMERAL }`
  surfaces EXACTLY the two recoverable cases — one account-wide read, no per-tx logic.
- **`Balance`** (data_api.rs:204) carries `total()` = the ECONOMICALLY-recoverable
  amount (spendable + pending-confirmation; it EXCLUDES `uneconomic_value` — dust ≤ the
  ZIP-317 marginal fee, S134 crypto audit). **S134 real-engine finding:**
  `get_transparent_balances` does NOT split a transparent balance by confirmation
  depth — a 1-confirmation ephemeral output appears fully in `spendable_value`
  (the confirmations policy governs SHIELDING eligibility via
  `allow_zero_conf_shielding`, not a depth bucketing of the read). So a reorg-finality
  split cannot be derived from this call; `list_stranded` reads `total()`
  (economically-complete), and FINALITY is 2e-2b-iii's deliverable, computed from per-OUTPUT depth
  (each recoverable UTXO's own `mined_height` — realized via `get_spendable_transparent_outputs`; see
  the Decision 6 "iii REALIZED" note).
- **There is NO `release` / `mark_unused` / DELETE for a reserved-but-never-mined
  ephemeral index.** `reserve_next_n_ephemeral_addresses` sets `exposed_at_height`;
  `find_gap_start` advances ONLY on ON-CHAIN use (`v_address_first_use`). A leaked
  index (reserved, tx0 never mined) is forever — the engine has no remedy.

## Decision

**1. `list_stranded` reads the engine's RECOGNISED ephemeral balance, NOT the
intent row.** `Wallet::list_stranded()` returns per-ephemeral
`StrandedAmount { recoverable_zat }` from ONE `get_transparent_balances` call,
filtered to the EPHEMERAL key origin: `recoverable_zat = balance.total()` — the
ECONOMICALLY-recoverable amount. **(S134 crypto audit finding:** `Balance::total()`
EXCLUDES `uneconomic_value` — the engine routes a transparent UTXO ≤ the ZIP-317
marginal fee there, since it cannot be swept for less than it costs. So dust is
omitted BY DESIGN — never hide ECONOMIC funds by depth, but un-sweepable dust is not
shown as "recoverable" — and a dust-only ephemeral collapses to a zero total and is
skipped. The v raw-UTXO manual sweep enumerates dust too and MUST reconcile against
this intentionally-economic surface.) The ephemeral ADDRESS is NEVER returned across
the surface (§5.4 never-RENDER, not only never-LOG); the amount is §5.4 never-LOG. v1
is AMOUNT-ONLY — finality is iii's (Decision 6).

**2. The terminal `Stranded` intent ROW is DECOUPLED from money-visibility.** The
2e-2b-i DETECT enumerates ephemerals from the ENGINE
(`get_ephemeral_transparent_receivers`), NOT from intent rows, and `list_stranded`
reads the ENGINE balance, NOT intent rows. So a `Stranded` row's existence has NO
bearing on whether funds are detected or surfaced — the row only (i) stopped the
doomed re-broadcast (done by its STATE, at `mark_stranded`) and (ii) is an audit
artifact. **Consequence:** reaping the row is ALWAYS money-safe — it can never
un-surface recoverable funds.

**3. The `Stranded`-row REAP is keyed on a DURABLE on-chain fact (tx0 burial),
NEVER on an empty poll.** `Wallet::reap_stranded_rows` deletes a `Stranded` intent
row once its tx0 (`txids[0]`) is buried beyond `REORG_MAX_BLOCKS`
(`should_reap(tx0_mined, tip)` = the same `buried` predicate). This is durable
(an on-chain mine, not a poll result), so a censoring endpoint's lying-empty can
NEVER trigger a reap — the §3.2i-2 reap-decoupling rule. Because a `Stranded` row
is created only AFTER tx0 is buried (the `Reconciled::Stranded` precondition), the
reap fires promptly (next pass), bounding the table over the wallet's life; the
amount stays visible via `list_stranded` (Decision 2). This deliberately does NOT
import the swap negative-proxy "empty ⇒ retire" semantics (ADR-0530 residual).

**4. The `enqueue` cap counts only FUTURE-outbox-work rows — terminal `Stranded`
is EXEMPT.** A flaky/censoring link that strands sends must not fill the
`QUEUED_SEND_INTENTS_MAX` cap with terminal rows (an availability regression, money
safe). The cap counts `state != Stranded` (Queued + Submitting + Sent — every row
the resubmission pass still acts on). A `Stranded` strand costs a real on-chain
tx0 + fees + ~`REORG_MAX_BLOCKS` blocks to create, so exempting it is not a cheap
DoS vector, and the reap (Decision 3) clears it anyway. **(Scoping vs the spec
wording:** the finer "exempt mined-buried `Sent` and ceiling rows" cannot be
cheaply + correctly expressed in `enqueue`'s pure-SQL context — a mined-buried
`Sent` needs a per-tx height read the intent store does not have, and it
auto-deletes within ~`REORG_MAX_BLOCKS`; a ceiling/`blocked` parked row sits in
`Queued` and is correctly COUNTED as recoverable future work that "must never be
silently dropped/reaped." Reported to the manager; deferred to v.)

**5. The DETECT SKIPS re-probing an already-recognised ephemeral (skip-funded).**
The detect reads the recoverable-balance map once per pass and does NOT open a
circuit for an ephemeral that already has a recognised balance (`total > 0`) — the
funds are already surfaced by `list_stranded` (Decision 1), so re-querying is wasted
battery AND an extra same-wallet fingerprint (the §3.2i-2 "do not re-query a funded
ephemeral every 20 s" battery rule / "reap on first durable surface"). This
self-corrects after a reorg: an engine rollback clears the balance, so the
ephemeral is re-probed on the next pass. A SKIP is counted distinctly from a POLL
in the §5.4 `wallet.ephemeral_detect` span (`scope = polled + skipped + truncated`),
so a healthy skip pass is not mis-read as `incomplete`. **(S134 review sub-case,
recorded for v):** skip-funded keys on "this ephemeral has SOME recognised balance",
so a SECOND on-chain deposit to the same already-funded one-time address
(adversarially reachable — anyone who observed tx0 knows the t-address) is not
re-`put` until a reorg clears the balance, so `list_stranded` under-counts it.
Money-safe (HD-recoverable, dormant), but the v manual sweep MUST be
UTXO-enumeration-based — re-probe even funded ephemerals at sweep time, never
trusting the recognised-balance cache.

**6. Finality is 2e-2b-iii's deliverable, NOT ii's — an S134 engine finding forced
this.** The original design computed a reorg-final split in ii via a high-confirmation
`ConfirmationsPolicy` on `get_transparent_balances`; a real-engine test DISPROVED it
— `get_transparent_balances` does NOT bucket a transparent balance by confirmation
depth (a 1-confirmation ephemeral output is fully `spendable_value`). So ii surfaces
the AMOUNT only (`total()`, money-complete), and iii's `is_final(depth)/REORG_MAX_BLOCKS`
bridge helper derives finality from per-tx depth (the host's `transactions()` view or
a per-output read), gating a shallow/reorgable phantom from being shown as settled.
This matches the spec's "amount-only DTO" + "separate `is_final` helper" decomposition.
**iii REALIZED (S135):** `account::ephemeral_recoverable_finality` does the per-output read
(`get_spendable_transparent_outputs` per recoverable ephemeral — the "Alternatives" source,
which carries each UTXO's `mined_height`, applies the SAME predicate + dust floor as the amount
read so the values reconcile), sums the value BURIED beyond `REORG_MAX_BLOCKS`, and welds
`is_final = (recoverable_zat > 0 && buried_zat == recoverable_zat)` — CONSERVATIVE (any shallow/pending portion ⇒ not
final). The FFI surface is `RecoverableEphemeralFunds { recoverableZat, isFinal }` (the weld in
ONE struct) plus a `#[frb(sync)] tx_confirmation_is_final(depth) = depth > REORG_MAX_BLOCKS` pure
helper (the depth-finality SSOT for the host's `transactions()` badge — distinct from the welded
amount, so no second skippable surface). The positive `is_final` path is REAL-ENGINE-proven (not
harness-gated): the put-at-height harness reads a 500-deep ephemeral final and a tip-height one
not-final.

## Consequences

- **PRESENTATION DOUBLE-COUNT HAZARD (S134 money-red-team — the iii/iv contract MUST
  own this).** The recoverable amount `list_stranded` surfaces is ALSO already inside the
  user's displayed wallet balance: `get_transparent_balances` and the wallet-summary
  transparent fold (`add_transparent_account_balances`) apply the SAME
  `excluding_wallet_internal_ephemeral_outputs` predicate, which INCLUDES strand/return
  ephemeral outputs, and the SDK folds that into `BalanceSnapshot.transparent`/`.total`. So a
  stranded amount is a SUBSET of the balance the user already sees — NOT additional funds.
  The iii DTO + iv copy MUST present it as "X of your balance is stuck on a one-time address
  (recover it)" [COPY SUPERSEDED — see "iv REALIZED" below: shipped cause-agnostic + locational
  as "on a one-time address (recoverable)/(still confirming)", NOT "stuck", and NOT "(recover it)"
  — the recover affordance is v's], NEVER as "+X recoverable" (a user who reads balance + stranded as a sum
  over-estimates holdings 2×). Money-safe today (dormant ⇒ both empty); recorded as a hard
  iii/iv presentation obligation. **Also iii (S134 UX):** (a) the FFI DTO must WELD the amount
  and the `is_final` finality gate into ONE struct (not two independently-callable surfaces),
  so a host cannot render a shallow/reorgable amount as settled by skipping the helper; (b) name
  the DTO CAUSE-AGNOSTICALLY (it surfaces an exchange RETURN as much as a strand — the detect
  can't tell them apart), e.g. `RecoverableEphemeralFunds`, so iv copy stays locational, never
  "stranded"/"deposit bounced" (one-regime-misleading).
  **iv REALIZED (S136 — the SDK-example Dart surface).** The host-presentation obligation is met:
  the example renders the recoverable amount as a NOTE under the `_BalanceCard` transparent line
  (nested in `transparentZat > 0`, since recoverable ⊆ transparent), with the FINAL locational copy
  "X of your balance is on a one-time address (recoverable)" / "(still confirming)" off the
  aggregate `allFinal` — NOT "stuck"/"stranded"/"bounced", NOT a summed peer row, NEVER "+X" (the
  headline stays `balance.totalZat`; a widget test pins the `1.01` over-count `findsNothing`). The
  reduction is a pure `summarizeRecoverable` helper. 3-angle review (code ∥ desktop ∥ security) NO
  BLOCKER — all three independently traced the subset-never-additive contract clean. The live
  "recover now" affordance is v's (gated on a SUCCESSFUL read + up-to-date `SyncStatus`, NOT the
  informational provider's empty-on-error swallow, which would hide held funds).
- **`list_stranded` is a LOWER BOUND, not exact recoverable (S134 money-red-team H2).** The
  engine buckets dust PER-UTXO (`value ≤ ZIP-317 marginal fee → uneconomic_value`), so N outputs
  each ≤ the marginal fee but COLLECTIVELY sweepable in one tx (one fee) all route to
  `uneconomic_value` and `total()` drops them. The surfaced amount is therefore the
  economically-recoverable floor; the v UTXO-enumeration manual sweep (which also catches the
  completeness gap + the second-deposit case) reconciles the exact total.
- **FLEET observability gap at window age-out (S134 UX, recorded for v).** A standing un-swept
  strand shows as `detected` once (edge-triggered) then only as `skipped` WHILE in the
  bounded-lookback window; once it ages out (~48–60 h) the detect span can fall silent though the
  funds persist (the USER still sees them — `list_stranded` is account-wide, window-unfiltered).
  When the v sweep ships, emit a counts-only standing-recoverable-ephemeral gauge so operators can
  watch the recovery backlog drain (the count is §5.4-spannable; amounts are not).
- **No engine un-reservation ⇒ the slice-A gates MUST stay until 2e-2b-v.** A
  censoring endpoint that induces ~10 leaked ephemeral reservations permanently
  bricks TEX sends (the gap-limit-10 ceiling) with NO engine remedy. The fix is
  app-layer (known-ceiling backoff + the host recovery surface), which lands with
  gate-removal (v). Building this read surface opens NO production window (the
  gates ⇒ no two-step signs ⇒ no ephemeral reserved ⇒ `list_stranded` is empty —
  a dormancy-invariant test pins it).
- **COMPLETENESS GAP (carried from the i operational round — a HARD v
  prerequisite).** `get_transparent_balances` surfaces only RECOGNISED balances
  (what the detect put), and the detect's window keys on RESERVATION height. A
  return arriving > ~48 h after reservation to a NEVER-recognised ephemeral is
  invisible to ALL automated paths (the common fast-forward case: tx1 spends tx0
  within minutes, so the ephemeral never gets a first recognised put; a later
  return is a first arrival nothing surfaces). Funds stay on-chain +
  HD-recoverable, but the SDK offers no automated OR manual path until v. **⇒ a
  user-discoverable manual ephemeral-sweep recovery MUST ship before signing turns
  on (2e-2b-v), else gate-removal turns "invisible" into a live HIDDEN-MONEY loss.**
  The window size (`EPHEMERAL_DETECT_LOOKBACK_BLOCKS` ≈ 48–60 h) vs real
  exchange-return SLAs is a founder/manager call at v.
- **Endpoint-asserted amount, finality-gated.** A lying endpoint can `put` a UTXO
  that genuinely pays our ephemeral (the i M2 re-derivation forbids foreign
  attribution); finality-gating (`final_zat`, the buried portion) is the defense
  against a shallow/reorgable phantom being shown as settled. Bounded by the 8 MiB
  message cap; never a loss.
- **DEFERRED to v (recorded):** the persistent-failure backoff (a censored EMPTY
  in-window ephemeral is re-probed every pass — skip-funded only covers the FUNDED
  case) and the full chain-advancement gate; the finer cap exemptions (Decision 4);
  the leaked-ephemeral cleanup + manual sweep. All money-safe + dormant now.
  **UPDATE (#334, S165): the persistent-failure backoff SHIPPED** — a per-address
  `EphemeralProbeBackoff` (doubling 60 s → 600 s, downward-jittered per §5.3) decays
  the re-probe of a cold/censored in-window ephemeral, extending this Decision's
  skip-funded from the FUNDED case to the persistently-COLD one. Availability/battery
  only, money-SAFE (only delays a PASSIVE re-probe; the manual sweep never gates on
  it). See the §3.2i-2 #334 AS-BUILT.
- **Key material never crosses FFI** — the read is UFVK/engine-backed; only amounts
  (host-rendered) and counts (spannable) leave the core.
- **One engine read, account-wide** — `list_stranded` and skip-funded share the
  SAME `get_transparent_balances` call shape (DRY); no per-ephemeral balance RPC.

## Alternatives considered

- **`get_spendable_transparent_outputs` per ephemeral (heights for finality)** —
  rejected for the v1 reader: N engine calls vs one account-wide
  `get_transparent_balances`, and the spec pins the latter for the amount. (It is the
  natural source iii may use to DERIVE `is_final` from per-output height, since the S134
  finding showed `get_transparent_balances` cannot — see Decision 6. Revisit there.)
- **Reaping the `Stranded` row on a recognised-balance match (row → ephemeral
  derivation)** — rejected as unnecessary coupling: the row is decoupled from
  money-visibility (Decision 2), so a durable tx0-burial trigger is sufficient and
  avoids deriving each row's ephemeral from its txids every pass.
- **Reaping on an empty poll / no exemption from the cap** — rejected: an empty poll
  is forgeable by a censor (would un-surface real funds — the explicit §3.2i-2 ban),
  and counting terminal rows toward the cap is the availability regression Decision 4
  fixes.
- **A bespoke finality depth (not `REORG_MAX_BLOCKS`)** — rejected: the codebase
  already pins `REORG_MAX_BLOCKS` as the irreversibility threshold for
  delete-on-mined and mark-stranded; finality reuses it (one SSOT), and the iii
  bridge helper shares it.
