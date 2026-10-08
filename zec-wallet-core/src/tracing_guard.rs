//! The §5.4 tracing capture-guard — shared test harness + the ONE source of
//! truth for the never-log list and the field allowlist.
//!
//! The wallet emits structured `tracing` so a host can observe sync
//! progress, reorg recovery, and stalls — but §5.4 draws a hard line: NO
//! address, amount, memo byte, txid, mnemonic/seed/key, swap JWT, light-server
//! endpoint auth header, or deposit address may ever reach a log sink. Heights, block counts, durations, error
//! codes, and outcomes ARE loggable (maintainer S37 — a height is public chain
//! data, not a spend).
//!
//! This module is the regression guard's machinery: a `tracing` `Layer` that
//! captures every `zec_wallet_core`-targeted field (event AND span — new +
//! recorded), plus the [`FORBIDDEN`] / [`ALLOWLIST`] lists that DEFINE the
//! policy. Two test sites consume it — the controller's event surface
//! (`sync_controller`) and the engine pass's per-range `wallet.sync` span
//! (`wallet`) — so the policy lives in exactly one place: add a field to the
//! crate's tracing surface and you update ONE allowlist, with both guards
//! re-checking it. A field outside the allowlist (or any forbidden token in a
//! name OR value) fails CI and forces a §5.4 review.
//!
//! COVERAGE (do NOT read green CI as crate-wide §5.4 proof): the guard is a
//! capture LAYER — it only inspects fields on code paths a guard test actually
//! drives. The two test sites above are those paths; other `zec_wallet_core`
//! tracing sites are allowlisted but not yet driven (see [`ALLOWLIST`]). The
//! token-substring scan still applies wherever the guard DOES run. And it sees
//! only what `tracing` DISPATCHES: the device's logcat layer
//! (`sdk/zec_wallet` `install_logcat_layer`, `FmtSpan::CLOSE`) SYNTHESIZES two
//! more names at every span close — `time.busy` and `time.idle` — inside the
//! fmt sink, after dispatch; they are not fields, this layer structurally cannot
//! see them, neither list below names them, and they are durations.
//!
//! THE POLICY SHIPS; THE HARNESS DOES NOT. The two lists, the per-field
//! predicate [`field_is_loggable`] and the target predicate [`is_sdk_target`]
//! are compiled into every build and re-exported as `zec_wallet_core::log_policy`,
//! because the bridge's host-switched device-log layer enforces them AT RUNTIME: a
//! field this module would fail a test over is withheld from a user's device
//! log, on every path, driven by a test or not. That is what closes the COVERAGE
//! caveat above for the one sink that reaches a shipped device. The capture
//! layer, its sink and the asserting helpers stay `#[cfg(test)]` — item by item,
//! so the lists did not have to move out from under 125 references to this file.

#[cfg(test)]
use std::sync::{Arc, Mutex};

#[cfg(test)]
use tracing::field::{Field, Visit};
#[cfg(test)]
use tracing_subscriber::layer::{Context, Layer};
#[cfg(test)]
use tracing_subscriber::registry::LookupSpan;

/// Never-log tokens (§5.4). Matched case-insensitively as a SUBSTRING of every
/// captured field NAME and VALUE, so neither a field called `deposit_address`
/// nor a `Debug`-rendered `BalanceSnapshot { spendable: .. }` value can slip a
/// secret through. The money DTOs are covered here by construction: `balance`,
/// `spendable`, and `transparent` are telltales of `BalanceSnapshot` /
/// `WalletState`, so a stray `tracing::debug!(?snapshot)` trips the guard
/// (the B-2-c security INFO — those wrappers are never `Debug`-logged). NOTE:
/// the money tokens are NEW vs. the pre-extraction inline controller copy — the
/// shared move deliberately TIGHTENS the guard, it does not merely relocate it.
///
/// VALUE-SHAPE LIMIT (be honest about it): this is a token-substring scan, so a
/// secret rendered as a NEUTRALLY-typed value under an allowlisted name — a bare
/// integer amount (`123`), or an address that contains no forbidden word — is NOT
/// caught by the value scan; the ALLOWLIST gate (a new field name forces review)
/// is the defense there. The `zatoshi*`/`nullifier`/`ufvk`/`.onion` tokens below
/// close the most likely TYPE-render leaks (`Zatoshis(..)`, `Nullifier(..)`, a
/// viewing key, an onion endpoint under an `error` value). The short address HRPs
/// (`u1`/`zs`/`t1`/`t3`) are deliberately NOT included — as 2-char substrings they
/// false-positive on ordinary values; a rendered address rides the allowlist gate.
pub(crate) const FORBIDDEN: &[&str] = &[
    "address",
    "amount",
    "memo",
    "txid",
    "seed",
    "mnemonic",
    "secret",
    "key",
    "jwt",
    // A URL as a VALUE (P3-13, the security review's LOW): the picker's
    // `server_id` is exactly the field a future site would log a custom URL
    // under, and the name gate cannot tell an id from a URL. No legitimate
    // field value carries a scheme separator, so this costs nothing and
    // catches the whole class — an endpoint, a swap provider, a relay.
    "://",
    // The light-server endpoint auth header (§2.3 `WalletConfig::endpoint_auth`,
    // added). `key`/`secret`/`jwt` above already catch the obvious field
    // names, but an opaque API key contains none of those tokens as a VALUE, and
    // a field named plainly `auth` would have been caught only by the allowlist
    // gate — which this module's own doc names as the weaker of the two. Added
    // by the security review, which found the class had shipped without
    // touching the file that calls itself the one source of truth for it.
    "auth",
    "credential",
    "deposit",
    "uri",
    "userinfo",
    // money-DTO telltales (BalanceSnapshot / WalletState never Debug-logged):
    "balance",
    "spendable",
    "transparent",
    // type-render telltales (a bare `Zatoshis(..)`/`Nullifier(..)`/viewing-key/onion
    // value under an allowlisted field name would otherwise slip the value scan):
    "zatoshi",
    "nullifier",
    "ufvk",
    // #397 (review security H1): the export ARTIFACT's rendered form — the
    // `ufvk` token catches the field NAME, this catches a leaked VALUE
    // (`uview1…` / `uviewtest1…`, one prefix covers both HRPs).
    "uview",
    ".onion",
];

/// The field names the crate's `zec_wallet_core` tracing surface is PERMITTED to
/// emit (§5.4 — heights/counts/durations/codes/outcomes). `message` is the
/// event/span name. A name outside this set fails CI on any guard-driven path
/// and forces a §5.4 review.
///
/// `error` and `network` are emitted off the two guard-test paths (dialer
/// `set_nodelay` best-effort warn → an OS errno; checkpoint monotonic-self-check
/// error → the `Network` enum), so they are allowlisted but NOT yet
/// guard-verified (the COVERAGE caveat in the module doc; driving those paths is
/// a tracked follow-on). `error` is the highest-risk NAME: an error VALUE is the
/// likeliest place a future endpoint/address slips past the name check, so the
/// value-substring scan is its real defense — any NEW `error = …` / `%e` field
/// needs a §5.4 review of what the value renders.
pub(crate) const ALLOWLIST: &[&str] = &[
    "message",
    // controller events
    "batches",
    "reorgs",
    "outcome",
    "backoff_secs",
    "depth",
    // engine per-range `wallet.sync` span
    "from",
    "to",
    "blocks",
    // broadcast `wallet.send` span + event (inc-2d-3-a) — counts only, no txid/amount.
    // `rejected` (the name is declared under `wallet.utxo_scan`, reused here since #307)
    // splits the mempool-reject share out of a "partial" outcome, and an ALL-rejected
    // pass emits its own `outcome="rejected"` (the `wallet.resubmit` vocabulary) —
    // post-#307 a reject can be an already-known tx from the §1.7 race, not a delivery
    // problem, so transport-partial and reject-partial need distinct fleet signals.
    // The live emission is capture-tested (`send_event_is_5_4_clean`).
    "txs",
    "accepted",
    // resubmission `wallet.resubmit` event (inc-2d-3-b-ii-B) — COUNTS only, no txid/amount.
    // `tx_count` = (re)broadcast txs. ON THE TWO KICK EVENTS (`wallet.swap.deposit_kick`,
    // `wallet.parked.authorize_kick`) it is the GROUP SIZE (#401 R4c), which is still transactions
    // — but a fleet must NOT assume the old `accepted + rejected <= tx_count` invariant there
    // (#403 R9c): the kick retries the whole group, and both tallies ACCUMULATE across rounds, so
    // either can exceed `tx_count` on a retried kick. `stranded`/`ceiling`/`capped`/`corrupt` are
    // per-pass usize
    // sub-tallies (a TEX strand, the ZIP-320 ephemeral gap-limit ceiling, a #315 attempt-capped
    // parked TEX, and a permanently-wedged send) that `outcome()` may shadow behind a co-occurring
    // broadcast — each rides its own field so fleet alerting sees it. Now NON-ZERO in production
    // post-gate-removal (2e-2b-v-5); never an amount/txid.
    "tx_count",
    "stranded",
    "ceiling",
    "capped",
    "corrupt",
    // `requeued` — the same event's count of sends put BACK on the queue this
    // pass (a per-pass aggregate like its siblings; the emit site's own doc
    // reviews it as §5.4-permissible). ADDED, and the reason is the
    // finding again: the name shipped on an INFO event without ever being
    // allowlisted, because no capture test drives the requeue arm. Found by a
    // source scan of every INFO-and-above site, which is now a standing test
    // (`extraction_policy.rs::every_info_and_above_field_name_in_the_sdk_is_allowlisted`)
    // — the capture guard sees driven paths, that one sees all of them.
    "requeued",
    // `owed` — the same event's count of wallet-created transactions the
    // generic delivery obligation found owed this pass (stage S8 `obligation`,
    // `crate::delivery`: persisted, unmined, unexpired, owned by no intent row —
    // a single-step transfer or shield whose broadcast failed or never
    // happened). A per-pass usize like its siblings, folded into `tx_count`
    // once the group goes out; never a txid, amount or address.
    "owed",
    // `ironwood-nu63-support.md` §3.2/§5 — the consensus-staleness surface.
    // `refused_stale` is a COUNT of queued sends left unsigned because this
    // build cannot transact on the reported network; `permits_signing` is a
    // bool; `verdict`, `expected_branch`/`endpoint_branch`, `scanned`/`claimed`
    // are public protocol constants and heights. NEVER an amount, address,
    // txid, memo, or endpoint URL — the verdict says which RULES the chain
    // runs, never anything about this wallet.
    "refused_stale",
    "permits_signing",
    "verdict",
    "expected_branch",
    "endpoint_branch",
    // NOTE: "scanned" is already allowlisted above for the sync surface — not
    // repeated here.
    "claimed",
    // GRACE-1 (§4p) — the `wallet.consensus` "grace expired by clock" warn that
    // `consensus_stamp::observe` emits the moment it latches an `Unknown` row (and
    // its debug sibling on a capable row, §4p-run row 2): `grace_secs` is the
    // ELAPSED device-clock seconds since the last capable verdict — a duration
    // (two instants subtracted), never the absolute capable timestamp (which is a
    // signing input and stays inside the core, G-12) and never a height, address,
    // amount, txid or endpoint. It narrows nothing: the same duration is what the
    // sync surface shows the user as "time left".
    "grace_secs",
    // GRACE-2 (§4v) — no new NAME; three `outcome`/`code` VALUES, each a static
    // literal (the clean fixture below carries them): `wallet.consensus`
    // `outcome = "capable_time_untrusted"`, the warn `consensus_stamp::observe`
    // emits when it latches a capable time later than the device clock (or
    // pre-epoch) — DURATION-FREE on purpose (§5.4: neither the absolute capable
    // time nor a "negative elapsed" ever reaches a log line); `wallet.consensus`
    // `outcome = "identity_height_out_of_range"`, the warn `evaluate_consensus`
    // emits when an endpoint's `block_height` does not fit a `u32` and the claim
    // is discarded (§4v G2-5 — the number itself is NOT logged); and the
    // `wallet.parked_verdict` event's `code`, the typed error's static
    // `RW-…` code when the parked reader could not read the stamp (§4v G2-7).
    // transparent UTXO detection `wallet.utxo_scan` event (Recv-2b, §3.3a) — COUNTS only:
    // `utxos` = transparent UTXOs put this poll, `rejected` = malformed/hostile records skipped,
    // `set_size` = how many addresses the SCOPED poll queried (index-0 + active swap destinations,
    // §3.3b D2 / ADR-0530, IZ-1b). NEVER an address / amount / txid / script. `outcome` is reused.
    "utxos",
    "rejected",
    "set_size",
    // §3.2i-2 2e-2b ephemeral-DETECT `wallet.ephemeral_detect` event — COUNTS only, on its OWN span
    // (NEVER shadowed behind `wallet.utxo_scan`): `scope_size` = ephemerals in the bounded-lookback
    // scope this pass, `polled` = those actually queried, `skipped` = in-scope ephemerals NOT queried
    // because already recognised/funded (skip-funded, ADR-0535 Decision 5), `detected` = UTXOs put,
    // `errored` = per-ephemeral transport faults skipped. `rejected` + `outcome` are reused. NEVER an
    // address / amount / txid.
    "scope_size",
    "polled",
    "skipped",
    "detected",
    "errored",
    // §3.2i-2 2e-2b-ii stranded-row REAP `wallet.stranded_reap` event — `reaped` = terminal
    // `Stranded` intent rows deleted this pass (tx0 buried beyond REORG_MAX_BLOCKS). A COUNT only,
    // NEVER an address / amount / txid.
    "reaped",
    // #368 ADR-0527 backfill `wallet.swap_refund_backfill` event — COUNTS only:
    // `registered` = single-use external indices engine-registered this batch, `armed` =
    // one-window synthetic watch rows inserted (≤ registered — live-row/live-quote indices
    // are skipped). NEVER an address or an index VALUE (an index + the viewing key derives
    // the address; counts derive nothing).
    "registered",
    "armed",
    // #382 chain-observed Refunded pin `wallet.swap_refund_pin` event — `pinned` = swap
    // records whose outcome the funded-transition pinned this pass. A COUNT only, NEVER
    // a swap id (== the deposit address for the shipped provider) or an address.
    "pinned",
    // #390 deep-scan `wallet.swap_deep_scan` event — COUNTS only: `widened_by` = single-use
    // indices this run added to the checked band (≤ REFUND_DEEP_SCAN_STEP), `pending` =
    // indices not yet registered + polled. Both are `u32` counts computed by arithmetic
    // (never a stored/derived value), so NEVER an index VALUE or an address (an index + the
    // viewing key derives the address; counts derive nothing) — the `registered`/`armed`
    // backfill-sibling treatment.
    "widened_by",
    "pending",
    // §3.2i-2 2e-2b-v-2 manual ephemeral-SWEEP `wallet.ephemeral_sweep` event — COUNTS only, on its
    // OWN span: `scanned` = one-time addresses enumerated this invocation, `swept` = those recovered
    // (signed + accepted), `failed` = per-address faults isolated, `truncated` = addresses left by the
    // cap/budget. `outcome` is reused. The AGGREGATE recovered AMOUNT is DELIBERATELY ABSENT from the
    // span (§5.4 never-LOG — it crosses the FFI host-rendered only). NEVER an address / amount / txid.
    "scanned",
    "swept",
    "failed",
    "truncated",
    // §3.2i-2 2e-2b-v-3 ephemeral reservation-PRESSURE `wallet.reservation_pressure` event — COUNTS only:
    // `outstanding` = reserved-but-unmined ephemeral indices consuming the engine gap window, `limit` =
    // the engine gap-limit denominator, `at_ceiling` = a bool (outstanding >= limit). NEVER an address.
    "outstanding",
    "limit",
    "at_ceiling",
    // T0-1c tip standing `sync::tip_standing` (the behind-server warn) — `newest_known` = the
    // grade's REFERENCE: the signed bundle's newest treestate row (a public constant of the
    // binary, 3,459,780 mainnet / 4,301,840 testnet today) or, since T0-1c-R2, the wallet's own
    // scanned height less `REORG_MAX_BLOCKS` — a height `wallet.sync`'s `to` and the surface's
    // `UpToDate { tip }` already carry, so it narrows nothing they do not. `claimed` (above) is
    // the endpoint's tip beside it. NEVER a host, URL, index, note, account or address. Its own
    // entry, with its own review: the first cut of that warn borrowed `limit` (the gap-limit
    // denominator) to pass this gate without a review of what the field carries — §4m #12.
    "newest_known",
    // swap on-ramp `wallet.swap` (execute) + `wallet.swap_quote` (quote) spans — the provider NAME,
    // a coarse DIRECTION code (out_of_zec / into_zec), and the outcome CODE; NEVER a swap id /
    // destination / deposit / refund address / amount. `code` is the payload-free `RW-…-NNN` error
    // code logged at the swap-port `map_err` boundaries (the internal cause preserved without the
    // secret). `provider` values are constant adapter labels (`near-intents`/`mock`).
    "direction",
    "provider",
    "code",
    // swap-destination recovery `wallet.swap_destination_recovered` event (§3.3b L5 / ADR-0530,
    // IZ-1b) — `count` = how many crash-persisted destinations the scoped store held at swap-enable
    // (a non-clean restart mid-swap). A COUNT only, NEVER an address.
    "count",
    // swap status-poll `wallet.swap_poll` span (W-swap-3-c-3-i) — the provider NAME, a
    // coarse outcome CODE (terminal/cancelled/halted/gone), and `faults`, a COUNT of
    // transient `provider.status` errors ridden over (a number, never an error payload).
    "faults",
    // incoming-funds detect `wallet.incoming_detect` event (ADR-0536 #392) — the
    // detected span's mined-height BOUNDS (`span_from`/`span_to`; heights are
    // loggable, the 2026-06-15 maintainer decision) + the reused `count` + `outcome`
    // ("detected" or the payload-free fault code). NEVER a txid/amount/memo/
    // address — exactly the IncomingFundsEvent payload discipline. Capture-tested
    // (`incoming_detect_event_is_5_4_clean`).
    "span_from",
    "span_to",
    // §3.3 tx-enhancement `wallet.enhance` event — COUNTS only: `requested` = the bounded batch
    // attempted this pass, `enhanced` = validated txs handed to the audited decryptor,
    // `status_set` = `GetStatus` requests answered with a chain status (INC-009), `not_found` =
    // empty replies, `unknown_branch` = replies naming a consensus branch this build lacks.
    // `rejected` + `outcome` are reused. NEVER a txid / memo / amount / address.
    //
    // ADDED, and the reason is the finding: NOT ONE of these names was allowlisted —
    // including the four that predate this change — because no guard test drives
    // `wallet.enhance`, so the gate whose stated job is "a new field name forces §5.4 review"
    // has never fired for this event. Allowlisting them closes the name half; the event still
    // has no capture driver, which is the COVERAGE limit the module doc warns about. An OWED
    // row tracks the driver (`enhancement_pass_event_is_5_4_clean`).
    "requested",
    "enhanced",
    "status_set",
    "not_found",
    "unknown_branch",
    // other crate `zec_wallet_core` sites — allowlisted, not yet guard-driven (see
    // the doc above): dialer best-effort warn (`error`), checkpoint error
    // (`network`).
    "error",
    "network",
    // `io_kind` — the wipe's file-cleanup warn (`store.rs`): the
    // `std::io::ErrorKind` of a cleanup that failed AFTER the crypto-shred
    // completed, rendered as the enum's `Debug` name (`PermissionDenied`, …).
    // A closed std vocabulary — never the `io::Error` itself, whose `Display`
    // can carry a path. ADDED by the same source scan as `requeued`.
    "io_kind",
    // §3.2g subtree-root ingestion (T0-1): which shielded pool an endpoint served,
    // refused, or served nothing for. The VALUE is `ShieldedProtocol::as_str_name`
    // — "sapling" / "orchard" / "ironwood", a public protocol constant fixed by the
    // wire enum — and it identifies a POOL, never a note, an account, an amount or
    // an address. It rides beside the already-allowlisted `count`.
    //
    // This field is what the whole "an endpoint that serves us no Ironwood roots is
    // not the same as an empty pool" distinction is written in, so it cannot be
    // dropped to satisfy the gate; it is allowlisted deliberately, and the guard
    // firing on it in four `sync_once` tests is the gate doing its job.
    //
    // T0-1a adds NO NAME here, and that is worth writing down rather than leaving
    // to be inferred. Its fourth pool outcome — "this endpoint served completion
    // heights that cannot be true" — logs its refusal code under the existing
    // `outcome`, the crate's established name for a coarse, payload-free code, and
    // the vocabulary it adds is `completion_gap` / `recorded_height` /
    // `bundled_frontier` / `scanned_tree_size` / `completing_block_hash`
    // (`root_bind::HeightBindRefusal::code`). Reusing an allowlisted name means the
    // "a new field forces a §5.4 review" gate does NOT fire, so the review is done
    // here instead: the codes name a CHECK, never a datum — no index, no height, no
    // oracle row, no endpoint identity. The numbers the refusal carries stay inside
    // the typed value and never reach a log.
    "pool",
    // §4q REQ-1 (INC-025) adds NO NAME here either, and says so. The belt's
    // `wallet.sync` event — the up-to-date exit read the DB's scanned height BELOW
    // the tip the pass recorded and must stall, never stamp (§4q-R P-RR2) —
    // carries `batches` + `reorgs` (counts), `claimed` (the endpoint's tip, the
    // name's established meaning above) and `outcome`, a code naming a CHECK:
    // `rewind_left_queue_empty` (the last action was a rewind that skipped its
    // re-queue) or `queue_empty_below_tip` (§4q-R's one new vocabulary value —
    // every other way to that exit: a re-queue that queued nothing, a rewind then
    // an advance then an empty queue, no rewind at all). The belt is REACHABLE in
    // the field — the REQ-1 fold review's row 1 reached it with a range a
    // cancelled pass had left queued above the recorded tip; P-RR1 ends that cell
    // as an under-claim, and the belt stays live for the rest. No height the pass
    // does not already log, no new field; `ScanProgress.rewound` already tells the
    // host a rewind happened, so nothing new crosses the bridge (P-R5). The two
    // vocabulary entries are rows of `a_clean_allowlisted_field_set_has_no_false_positive`.
    // §4u REW-1 adds NO NAME either, and says so. The rewinding-streak report — the
    // loop's judgement that `MAX_CONSECUTIVE_REWINDING_PASSES` passes in a row each
    // rewound, published as `Stalled { EndpointMisbehaving }` in place of the pass's
    // terminal status — is a `wallet.sync` warn carrying `outcome = "rewinding_streak"`
    // (a code naming a CHECK; the one vocabulary value the round minted) beside the
    // reused `backoff_secs` (the sleep the loop takes next). The pass event's
    // `backoff_secs` — new on the `outcome = "ok"` line — is that same duration: the
    // poll interval after a clean pass, the ladder's rung after a rewinding one, and
    // absent from a `once()`, which sleeps nothing (an `Option` value records only
    // when `Some` — never a made-up zero). A duration narrows nothing. The value is a
    // row of `a_clean_allowlisted_field_set_has_no_false_positive`.
    // SCAN-1 (§4o S1–S3): the per-batch phase timings on the engine's `wallet.sync`
    // span, and their per-pass sums on the controller's `wallet.sync` pass event.
    // Each `*_ms` is an integer-millisecond DURATION measured on the sync task —
    // never a height, a note count, or anything that names a wallet — and each
    // has its own line and its own reason (wrap #12 — no borrowed name), because
    // "a duration narrows nothing" is NOT one class: it is true of the three
    // phases whose cost the chain and the machine set, and false of the one whose
    // cost this wallet sets (review, F1).
    // `anchor_ms` — the batch's ANCHOR work (SCAN-2, §4t). On a batch that
    // fetched — the first of a pass, one whose range did not follow the previous
    // batch, one after a rewind, and every `TREE_STATE_RECONCILE_BATCHES`th
    // consecutive one, the reconcile — the `GetTreeState` round trip, the same
    // for every wallet on the same link; PLUS, on every batch whose download
    // completed, the fold of that batch's own note commitments into the anchor
    // the next batch will use (`sync::derive_chain_state`, timed inside the
    // download and moved here so `dl_ms` does not absorb it). The fold's cost is
    // set by the batch's output count — chain data, the same for every wallet —
    // never by what this wallet holds. Before SCAN-2 this was the RPC alone,
    // every batch.
    "anchor_ms",
    // `dl_ms` — the `GetBlockRange` stream + the cache insert: set by the batch's
    // bytes (chain data) and the link, the same for every wallet.
    "dl_ms",
    // `scan_ms` — the locked `scan_cached_blocks`: trial decryption of every
    // output in the batch (chain data) + the shardtree + the one upstream
    // SQLCipher commit, emitted WHOLE (§4o P-S2: a `commit_ms` alone is not
    // separable from this crate). The decrypt cost is per OUTPUT, not per note
    // found; a wallet with notes in the batch pays a few extra witness writes,
    // inside the noise of the commit.
    "scan_ms",
    // `snap_ms` — `progress_snapshot` (`get_wallet_summary`), which is
    // O(scanned-set): its cost SCALES WITH THIS WALLET'S scanned set (≈6.5 s
    // measured on a deep-recovery wallet, against p50 27 ms on a 30-day
    // wallet in the capture — a 240× spread the wallet drives). Present on
    // a batch that ATTEMPTED an authoritative refresh (the first, every
    // `SYNC_SUMMARY_REFRESH_BATCHES`th, a reorg's forced one; a batch that
    // attempted twice carries the sum, once — never a duplicate key). Accepted
    // because it rides at most one record per 80 batches, and because "how much
    // has this wallet scanned" is a height-class fact (§5.4 loggable) — never an
    // amount, a note or an address.
    "snap_ms",
    // `wall_ms` — the whole pass, entry to return of `sync_once` (the pass event
    // only); its gap above the batch sums is the time the batches do not see.
    "wall_ms",
    // S15-F1 phase A, the pass event only. `tip_ms` — the pass's `fetch_tip`: one
    // `GetLatestBlock` round trip and its checks, the same for every wallet on
    // the same link (a link fact, like `dl_ms`).
    "tip_ms",
    // `roots_ms` — the pass's whole subtree-root ingestion, on EVERY pass: each
    // pool's root stream and the write of every served root (both chain-wide:
    // the same for every wallet on the same link and tip), plus the two parts
    // this wallet sets — the bind's reads of its own shard heights, scanned
    // tree sizes and brackets, and any wait for the db lock behind a concurrent
    // wallet call (the signal `scan_ms`/`snap_ms` already carry). One number
    // per pass: it cannot say which subtrees or blocks matter to this wallet.
    // A height-class fact (§5.4 loggable), never an amount, a note or an
    // address. Re-judged at S15-F1 phase B (ADR-0569), which makes it vary with
    // how much this pass RE-SERVES: a full fetch on a session's first pass and
    // every full verification, otherwise the roots from the last stored one
    // (earlier for the reorg window, a bracket or a newly scanned completing
    // block). That says roughly how far this wallet has scanned and whether
    // this is a full pass — both of which the pass's own `from`/`to` heights
    // and the session's pass count already say more precisely — so the reason
    // stands.
    "roots_ms",
    // FR-47 — the `wallet.vault_call` line of the bounded key store. `op` is
    // one of a CLOSED set naming the port call (`purge_namespace`,
    // `load_index`, `store_wrap`, … — spelled without FORBIDDEN's `key`), a
    // static literal of the SDK's own vocabulary — never a namespace, an
    // identifier, an alias or a key. `call_ms` is how long one key-store call
    // took (or was waited on), in whole milliseconds, logged only past a
    // second: a device-latency fact, the same for every wallet on the phone.
    "op",
    "call_ms",
    // (FR-53, stage S16: the duress sever's one `wallet.sever` line is emitted
    // BEFORE it begins and carries NO field; the verb's answer is its report.
    // What is and is NOT silenced after that line, with its exact exceptions,
    // is stated once, on `Wallet::sever_silenced` in `wallet.rs`.)
    // `chain_outputs` — the batch's public shielded-output count: Sapling outputs
    // + Orchard actions + Ironwood actions in the compact blocks downloaded for
    // `[from, to)`. CHAIN data of the same class as `blocks` — a number every
    // observer of the chain already holds — and it says nothing about which
    // outputs are this wallet's. `received_notes`, the WALLET fact beside it in
    // the loop, stays unlogged; the S1 capture row asserts it absent. Named
    // `chain_` so that no future site can reuse the bare word for a
    // TRANSACTION's output count (a spend-shape hint) and pass this gate
    // silently — NEVER reuse this name for a transaction's outputs.
    "chain_outputs",
    // `field` — SCAN-2 (§4t-run review row 3): on the `wallet.anchor_reconcile`
    // mismatch warn, WHICH of the five compared `ChainState` members disagreed, as
    // one of five static literals (`height`/`hash`/`sapling`/`orchard`/`ironwood`,
    // plus `unknown` if upstream ever grows a sixth). A code naming a STRUCTURE
    // MEMBER, never its value: no height, no hash bytes, no tree size, and nothing
    // about this wallet — the endpoint served both sides of the comparison. It
    // exists so a field report can tell a server whose `GetTreeState` hash
    // convention differs from its `GetBlockRange` one (the same field forever) from
    // a tamper (a frontier moved). Do NOT reuse this name for a value.
    "field",
    // `server_kind` — P3-13 (`sync-server-picker.md` §5): on the
    // `wallet.sync_server` records, WHICH kind of choice a probe/switch acted
    // on, as one of three static literals (`predefined`/`custom`/`default`, plus
    // `unknown` if the enum ever grows). A code, never a host: the custom URL
    // the user typed reaches no field (the gate-5 row pins it).
    "server_kind",
    // `server_id` — the slug the CALLER named for a predefined choice
    // (`zec-rocks`, a host's own; on the not-offered refusal, whatever
    // slug it asked for), validated `[a-z0-9-]` and at most 32 bytes by
    // `SyncServerId::new` — so never a URL (no `:` `/` `.` can pass) and never
    // a key; the `Custom` arm sets no id at all. Named `server_` so no future
    // site can log a bare `id` (a txid, a note id) through this gate.
    "server_id",
    // FR-29 (`host-transport-crossing.md` §5): the bridge's cabi module emits
    // `wallet.host_dialer_registered` / `_replaced` / `_cleared` / `_retired`
    // FIELDS-FREE, and `wallet.host_dialer_descriptor` with EXACTLY the two
    // MECHANICAL values of the host's transport descriptor —
    // `transport_readiness` (0..=100) and, since C1 at ABI v3,
    // `transport_health` (Starting/Ready/Failed) — named transport_* so no
    // future site can log a bare field (the file's `server_` precedent). NEVER
    // the host's transport NAME (ADR-0547: host-chosen text stays off every log
    // line), no destination, no isolation KEY, no auth token, no op id.
    //
    // `transport_isolation` and `transport_exposure` were HERE and were REMOVED
    // (found by Relim, a joint change with their push line). Together
    // they were a bijection onto the host's three transports — Tor, Shadowsocks,
    // Direct — so this log recovered which circumvention product a person runs
    // from two integers, with no name anywhere: ADR-0547's letter satisfied and
    // its purpose defeated. Re-encoding cannot fix it, because any field that
    // carries the privacy GRADE is exactly what separates those three. The
    // grade belongs in the host's CONSENTED debug bundle,
    // not on a device log a stranger may hold. **Do not re-add either name
    // here**; this allowlist and T16's field-set assertion are the pair of
    // guards that keep it out, and they hold whatever the host's roster grows
    // into.
    "transport_readiness",
    "transport_health",
    // `wallet.dial` (`on-device-log-layer-phase-1.md` §3a) — the per-dial
    // ATTRIBUTION line: which ARM performed a dial and which circuit family it
    // was for, beside the reused `outcome`. `dial_arm` is one of two static
    // literals (`host` = the host's transport performed the dial, `sdk_direct` =
    // the SDK's own direct dialer). It names WHO dialled, never whether the dial
    // was private: until an earlier revision the values were `private` / `clearnet`, and a host
    // whose transport goes direct printed `private` over an exposed dial. The
    // honest grade is exactly the `transport_exposure` removed above, and it stays
    // off this line for the same reason; `dial_class` is the `PathClass` as one of FOUR
    // (`sync` / `broadcast` / `swap` / `other` — `swap` split out of `other`
    // `window`, when the class gained its own patience window).
    // `outcome` here is `connected` or the
    // typed `DialError` VARIANT name — never the `Io` payload. It exists
    // because neither side could show, from a device, that a wallet dial rode
    // the host's transport: the proof was about the program, not the phone.
    // NEVER a host, a port, the isolation KEY (the class is derived from it and
    // is all that may appear), or the host's transport name.
    // The VALUE SET is itself guarded, by name and exhaustively over
    // `PathClass` (`the_dial_class_vocabulary_is_exactly_the_priced_four`):
    // this allowlist matches on the field NAME, so without that row a fifth
    // class would reach a device log with nothing priced and nothing red.
    //
    // WHAT IT DOES DISCLOSE, stated rather than argued away (the security
    // review refuted the first version of this comment as circular): that a
    // private transport is IN USE, and when a dial left it. It says nothing
    // about WHICH transport — the line stands, the grade and the name stay
    // off every log — but "this handset routes its wallet privately" is itself
    // posture. **And, since an earlier revision, that this handset used the cross-chain SWAP
    // on-ramp, when and how often** — `dial_class=swap`, a class whose dials
    // are user-initiated foreground actions with a named counterparty, so the
    // timestamps join to the swap provider's own logs and to an on-chain
    // deposit. That is a different denomination from "this wallet syncs and
    // sends privately" and it is PRICED here rather than inherited: it is kept
    // because the class is the log's only handle on WHICH circuit family a
    // dial belonged to, folding it back into `other` would file a dial under a
    // class that is not its own (exactly what `log_dial`'s exhaustive match
    // exists to prevent), and it was announced to the host as part of the
    // `window` vocabulary (stage plan §3.2, sync point 2, spec §5.4). It rests
    // on the SAME single reason as the rest of the line and no other.
    //
    // It is acceptable on a device log for ONE reason: that log is no
    // longer unconsented. It is written only while the HOST has switched it on
    // (FR-35, default OFF), reflecting a choice its user can see and reverse,
    // and the host's consent copy is told to say so (`set_device_log`'s doc).
    // Remove that switch and this field has to go with it — and the swap value
    // is the first thing that goes if the switch is ever weakened.
    // Named `dial_` so no future site can log a bare `class` or `arm`.
    "dial_arm",
    "dial_class",
    // FOUR DEBUG-LEVEL names that shipped un-allowlisted until an earlier revision — found the
    // moment a capture test first drove the REAL clearnet dialer
    // (`a_dial_names_its_arm_its_class_and_its_outcome`), which is the
    // finding a third time: the name gate only fires on a driven path. Debug
    // level, so the host-switched device log never prints them; the debug and
    // measurement builds' loud layer does.
    // `family` — `wallet.dial.connected`: which address family won the direct
    // dialer's race, one of two static literals (`v4` / `v6`). Never the peer
    // address it is derived from.
    "family",
    // `dropped` — `wallet.dial.addrs_truncated`: how many resolved addresses
    // the per-family cap left untried. A COUNT, never an address.
    "dropped",
    // `reason` — `wallet.dial.attempt_join_failed`: `panic` or `cancelled`, a
    // STATIC literal chosen by `JoinError::is_panic` (never the panic payload).
    // A generic name with exactly one site: do NOT reuse it for free text — on
    // a reused name the value scan is the only gate left.
    "reason",
    // `deferred` — `wallet.ephemeral_detect`'s debug twin: in-scope ephemerals
    // left for a later pass. A COUNT, like its allowlisted siblings.
    "deferred",
];

/// Make every `zec_wallet_core` tracing callsite ENABLED for the whole test process,
/// so a thread-local [`CaptureLayer`] reliably sees spans/events under parallelism.
///
/// WHY this is needed (the parallel-capture flake): a callsite first HIT by a
/// NON-capture test caches as DISABLED, and a later capture test's span at that
/// callsite becomes a silent no-op — it captures nothing. Installing a plain
/// `registry()` (which is always-interested) as the GLOBAL default once, then
/// rebuilding the cache, keeps every callsite enabled; capture still happens via the
/// per-test thread-local `set_default(registry().with(CaptureLayer))` layered over it.
/// Idempotent — call at the top of every capture test BEFORE `set_default`.
///
/// **THE MECHANISM, corrected from the Relim session's diagnosis of the same
/// class in its own tree** (this paragraph used to say the cache "is computed against
/// the GLOBAL default subscriber only", which describes the symptom rather than the
/// cause, and would let someone conclude the wrong fix): `tracing-core` has a
/// single-dispatcher FAST PATH — while only one `Dispatch` is live, a rebuild takes
/// the REGISTERING THREAD's own default rather than joining over the live
/// dispatchers. On a plain test thread that default is `NoSubscriber`, whose
/// `register_callsite` answers `Interest::never()`, and that is written straight into
/// the callsite's global cache. (Note what this is NOT: a second, disagreeing
/// subscriber cannot silence anyone — `Interest::and` collapses a disagreement to
/// `sometimes`.) **So the invariant this function actually buys is not "a global
/// default exists" but "the default in force when the cache is built is
/// always-interested"** — and it holds here because the `Once` runs BEFORE the
/// thread-local `set_default`, so the registering thread's default at that instant is
/// the always-interested global registry installed one line above. A tree with no
/// global default at all is the one that hits `NoSubscriber`; Relim's did, and fixed
/// it by keeping a second live `Dispatch` so the join runs. Either remedy works;
/// this one needs no second dispatcher kept alive.
///
/// **The evidence gap, stated rather than left flattering:** this guard's causal
/// claim rests on the mechanism above and on the flake not recurring since. NOBODY
/// EVER WATCHED THE NATURAL FLAKE HEAL, and there is no planted reproduction here
/// that fails without this call and passes with it. Relim built exactly that for its
/// own fix (a deterministic reproduction kept permanently, plus mutants under held
/// load) and carried the same gap honestly; ours is the weaker record of the two.
#[cfg(test)]
pub(crate) fn force_wallet_callsites_enabled() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        // A bare registry as the global default is always-interested. `let _`: if some
        // other code already set a global default, that one is also always-interested
        // enough for our purpose; we only need callsites kept enabled.
        let _ = tracing::subscriber::set_global_default(tracing_subscriber::registry());
        // Re-evaluate any callsite a prior no-subscriber test already cached as disabled.
        tracing::callsite::rebuild_interest_cache();
    });
}

/// Captured `zec_wallet_core` fields, grouped by the RECORD that carried them —
/// one group per event, per span creation and per `Span::record` call — each
/// tagged with the span's or event's NAME (`wallet.sync`, `wallet.send`, …; an
/// event's name is its `message`). The flat `(name, value)` view every §5.4
/// guard scans is [`CapturedEvents::fields`]; [`CapturedEvents::records_of`]
/// keeps the grouping, which is what lets a guard say "the `wallet.sync` SPAN
/// carries exactly these names" and read one event's values beside each other
/// without a span id (SCAN-1 S1/S3 extended the sink this far and no further:
/// there is still no span id, so "every span carries X" is asserted by
/// COUNTING).
#[cfg(test)]
#[derive(Clone, Default)]
pub(crate) struct CapturedEvents {
    records: Arc<Mutex<Vec<CapturedRecord>>>,
}

/// One captured record: the span/event NAME it came from, and the
/// `(field_name, field_value)` pairs it carried.
#[cfg(test)]
type CapturedRecord = (String, Vec<(String, String)>);

#[cfg(test)]
impl CapturedEvents {
    /// A snapshot copy of everything captured so far, flattened in capture order.
    pub(crate) fn fields(&self) -> Vec<(String, String)> {
        self.records
            .lock()
            .expect("capture mutex poisoned")
            .iter()
            .flat_map(|(_, fields)| fields.iter().cloned())
            .collect()
    }

    /// Every record whose span/event name is `scope`, in capture order — each
    /// the fields ONE event, span creation or `Span::record` call carried.
    pub(crate) fn records_of(&self, scope: &str) -> Vec<Vec<(String, String)>> {
        self.records
            .lock()
            .expect("capture mutex poisoned")
            .iter()
            .filter(|(s, _)| s == scope)
            .map(|(_, fields)| fields.clone())
            .collect()
    }
}

#[cfg(test)]
struct Grab<'a>(&'a mut Vec<(String, String)>);

#[cfg(test)]
impl Visit for Grab<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0
            .push((field.name().to_string(), format!("{value:?}")));
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.push((field.name().to_string(), value.to_string()));
    }
}

/// A `tracing` layer that records every `zec_wallet_core`-targeted field. Build it
/// with [`CaptureLayer::new`] over a [`CapturedEvents`] you keep a clone of.
#[cfg(test)]
pub(crate) struct CaptureLayer {
    sink: CapturedEvents,
}

#[cfg(test)]
impl CaptureLayer {
    pub(crate) fn new(sink: CapturedEvents) -> Self {
        Self { sink }
    }

    /// Capture one record. `scope` is the span's name; an event passes `None`
    /// and is scoped by its `message` field (the event's name in this crate's
    /// convention — `tracing::info!(…, "wallet.sync")`).
    fn grab<R: FnOnce(&mut Grab<'_>)>(&self, scope: Option<&str>, record: R) {
        let mut fields = Vec::new();
        record(&mut Grab(&mut fields));
        let scope = scope.map(str::to_owned).unwrap_or_else(|| {
            fields
                .iter()
                .find(|(n, _)| n == "message")
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        });
        self.sink
            .records
            .lock()
            .expect("capture mutex poisoned")
            .push((scope, fields));
    }
}

/// Is this callsite one of OURS, and therefore one the §5.4 guard must grade?
///
/// Two target shapes ship in this crate and both are ours: the module path
/// (`zec_wallet_core…`, which `target:` omitted gives you) and the explicit
/// `wallet.<surface>` families (`wallet.consensus`, `wallet.enhance`,
/// `wallet.swap`). **Only the first was captured until T0-4**, so six
/// `wallet.consensus` and `wallet.enhance` callsites emitted fields that the
/// privacy guard never looked at — `assert_5_4_clean` graded a set they were
/// not in, and a test could not assert one of them fired at all (§4v-run review
/// row 11, found from one side; this is the general form). A crate-external
/// target (a dependency's) is still ignored: the guard is about OUR §5.4
/// surface.
///
/// **A THIRD shape since the BRIDGE's target, `zec_wallet`.** The
/// host-dialer lifecycle events (`net_dialer_cabi.rs`) are emitted there, and
/// until now neither this guard nor the device's log layer matched it — so the
/// one family of events a connectivity diagnosis needs was outside both the
/// policy and the log ("the pair of guards" was one guard and a comment).
///
/// **Matched on a `::` BOUNDARY, never by bare prefix.** `zec_wallet` is a
/// prefix of `zec_wallet_core`, of the Tor plugin's `zec_wallet_tor` and of any
/// future sibling; a bare `starts_with` would hand each of them this policy's
/// blessing — and the host-switched device log with it — without a review of what
/// they emit. This is the ONE predicate: the capture guard grades exactly the
/// targets the shipped layer prints (`zec_wallet_core::log_policy`).
pub fn is_sdk_target(target: &str) -> bool {
    fn module_of(target: &str, krate: &str) -> bool {
        target
            .strip_prefix(krate)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with("::"))
    }
    // `wallet.<surface>` is ours BY CONVENTION ONLY — a target is a string any
    // crate may pick — so the arm at least demands the shape we use: a lowercase
    // surface name after the dot. No dependency in the lock emits one (checked
    // the static scan in `extraction_policy.rs` reads what OURS carry.
    let surface = target
        .strip_prefix("wallet.")
        .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_lowercase()));
    module_of(target, "zec_wallet_core") || module_of(target, "zec_wallet") || surface
}

// Span-AWARE on purpose: events alone don't cover the engine's per-range
// `wallet.sync` SPAN (`from`/`to`/`blocks` set at creation, `outcome` recorded
// when the batch resolves), so the guard inspects new spans AND recorded span
// values as well as events. (`LookupSpan` resolves a recorded span's target.)
#[cfg(test)]
impl<S> Layer<S> for CaptureLayer
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    // ALWAYS interested — the capture layer must see every `zec_wallet_core` callsite no
    // matter which test registered it first. Without this, a callsite first HIT by a
    // non-capture test (no subscriber on its thread ⇒ NoSubscriber's `never`) can cache
    // as disabled GLOBALLY, so a later capture test's span/event is silently a no-op and
    // captures nothing — the parallel-test flake this guard exists to prevent. Returning
    // `always` forces the aggregate callsite interest to `sometimes`, so every span/event
    // is dispatched per-call to whatever subscriber is current on the emitting thread.
    fn register_callsite(
        &self,
        _metadata: &'static tracing::Metadata<'static>,
    ) -> tracing::subscriber::Interest {
        tracing::subscriber::Interest::always()
    }

    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        if is_sdk_target(event.metadata().target()) {
            self.grab(None, |g| event.record(g));
        }
    }

    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        _id: &tracing::span::Id,
        _ctx: Context<'_, S>,
    ) {
        if is_sdk_target(attrs.metadata().target()) {
            self.grab(Some(attrs.metadata().name()), |g| attrs.record(g));
        }
    }

    fn on_record(
        &self,
        id: &tracing::span::Id,
        values: &tracing::span::Record<'_>,
        ctx: Context<'_, S>,
    ) {
        let Some(span) = ctx.span(id) else { return };
        if is_sdk_target(span.metadata().target()) {
            self.grab(Some(span.metadata().name()), |g| values.record(g));
        }
    }
}

/// The first §5.4 violation in `fields`, if any: a forbidden token in a field
/// name or value, or a field name outside the allowlist. `None` ⇒ §5.4-clean.
/// Returned (not asserted) so a meta-test can prove the guard HAS teeth.
#[cfg(test)]
pub(crate) fn first_violation(fields: &[(String, String)]) -> Option<String> {
    fields
        .iter()
        .find_map(|(name, value)| field_violation(name, value))
}

/// May this ONE field be written to a log? The per-field form of the policy,
/// for a sink that enforces it AT RUNTIME (the bridge's host-switched device-log
/// layer): `true` iff the name is allowlisted and neither the name nor
/// the rendered value carries a forbidden token.
///
/// It is [`field_violation`] and nothing else — the predicate every capture
/// test asserts through `first_violation` — so what a test refuses and what a
/// shipped layer withholds cannot drift apart. Its limit is the module's own:
/// a token-substring scan cannot see a secret rendered as a neutral value under
/// an allowlisted name (see [`FORBIDDEN`]); the name gate is the defence there.
pub fn field_is_loggable(name: &str, value: &str) -> bool {
    field_violation(name, value).is_none()
}

/// Event MESSAGES — static literals at their one site each — whose wording
/// collides with a [`FORBIDDEN`] token (`seed`, `deposit`, `auth`). They name a
/// CHECK or a PATH, never a datum: a fingerprint that could not be read, the
/// swap-deposit and parked-authorization resubmit kicks. Exempt from the VALUE
/// scan by EXACT equality, on the `message` field only — a real secret can
/// never equal one of these strings, and the same words with a tail, or on any
/// other field, still trip.
///
/// WHY IT EXISTS (the security review's MEDIUM). Until the device log
/// shipped, a `message` that tripped the scan cost nothing: no capture test
/// drives these six, and no sink existed. With a shipped sink each printed as
/// `<message withheld> … withheld=1` — and the two kicks carry IDENTICAL fields,
/// so a tester's log could not say which resubmit path ran; worse, a
/// `withheld=1` on six always-benign lines teaches a reader to ignore the one
/// marker that means "the policy refused something". Renaming six events that
/// docs and fleet alerting already name was the alternative; an exact-match
/// vocabulary is this module's established answer (the two above).
///
/// The list is CLOSED AND WATCHED: `extraction_policy.rs::
/// every_info_and_above_field_name_in_the_sdk_is_allowlisted` puts every
/// INFO-and-above message literal in the source through this predicate, so a
/// seventh collision fails there — review it, then reword it or add it here.
pub(crate) const SANCTIONED_MESSAGES: &[&str] = &[
    "seed fingerprint has wrong length; treating as absent (verification skipped)",
    "seed fingerprint unreadable; treating as absent (verification skipped)",
    "swap deposit could not sign at execute — left durably queued for resubmission",
    "wallet.swap.deposit_kick",
    "wallet.parked.authorize_kick",
    "wallet.seed_fingerprint_record",
];

/// The §5.4 violation ONE field commits, if any — the single predicate behind
/// both `first_violation` (test time) and [`field_is_loggable`] (run time).
fn field_violation(name: &str, value: &str) -> Option<String> {
    let lname = name.to_lowercase();
    let lvalue = value.to_lowercase();
    // A bare typed error CODE (`RW-SEED-004`, `RW-KEY-002`, …) is §5.4's
    // sanctioned outcome vocabulary — `&'static str`, payload-free by the type
    // system — yet its taxonomy words collide with FORBIDDEN substrings
    // ("seed", "key"). Exempt EXACTLY the code shape from the VALUE check,
    // nothing looser (a real leak — words, hex, an address — can never match
    // it), and only the value check: the field NAME must still be allowlisted
    // (review fold — without this, any capture test pinning a rescan/open
    // FAILURE event false-positives on its own outcome code).
    // Phase-3 P3-2 (the Batch C code reviewer's MEDIUM): the SECOND sanctioned
    // vocabulary is the propose variant name on the `code` field —
    // `send::PROPOSE_ERR_VARIANT_CODES`, a closed static set five of whose
    // members carry a forbidden substring (`address`, `key_…`, `balance_error`).
    // Exempt EXACT membership, on the `code` field only: a real address, a
    // member with a tail, or the same word on another field still trips
    // (`tests::a_propose_variant_name_on_code_is_exempt_but_anything_looser_still_trips`).
    let static_variant_code =
        name == "code" && crate::send::PROPOSE_ERR_VARIANT_CODES.contains(&value);
    // The THIRD sanctioned vocabulary: the event NAMES in
    // [`SANCTIONED_MESSAGES`], on the `message` field only, by EXACT equality.
    let sanctioned_message = name == "message" && SANCTIONED_MESSAGES.contains(&value);
    let code_shaped_value =
        is_error_code_shaped(value) || static_variant_code || sanctioned_message;
    for bad in FORBIDDEN {
        if lname.contains(bad) {
            return Some(format!("forbidden token {bad:?} in field name: {name}"));
        }
        if !code_shaped_value && lvalue.contains(bad) {
            return Some(format!(
                "forbidden token {bad:?} in field value: {name}={value}"
            ));
        }
    }
    if !ALLOWLIST.contains(&name) {
        return Some(format!(
            "tracing field {name:?} is outside the §5.4 allowlist {ALLOWLIST:?}"
        ));
    }
    None
}

/// Assert `fields` is §5.4-clean (no forbidden token, every name allowlisted).
#[cfg(test)]
pub(crate) fn assert_5_4_clean(fields: &[(String, String)]) {
    if let Some(violation) = first_violation(fields) {
        panic!("§5.4 tracing guard: {violation}");
    }
}

/// EXACTLY the typed-error-code shape (`RW-<AREA>-<NNN>`): the literal `RW-`
/// prefix, one-plus ASCII UPPERCASE letters, `-`, exactly three ASCII digits.
/// Everything else — words, spaces, hex, addresses, a code with a tail — fails,
/// so the [`field_violation`] value-exemption this feeds can never pass a real
/// secret whose rendering merely CONTAINS a code-like fragment (the whole value
/// must be the code and nothing more).
fn is_error_code_shaped(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("RW-") else {
        return false;
    };
    let Some((area, digits)) = rest.split_once('-') else {
        return false;
    };
    !area.is_empty()
        && area.len() <= 12
        && area.bytes().all(|b| b.is_ascii_uppercase())
        && digits.len() == 3
        && digits.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::BalanceSnapshot;

    // `first_violation` IS the §5.4 policy. These pin it as the pure function it is
    // — no async/runtime/FakeChain — so a typo in the substring scan or the allowlist
    // gate fails CI directly, not just incidentally via whatever a live sync path
    // happened to emit. (The end-to-end span guards live in `wallet`/`sync_controller`.)

    #[test]
    fn a_bare_error_code_value_is_exempt_but_anything_looser_still_trips() {
        // The exemption: a typed code whose taxonomy word collides with
        // FORBIDDEN ("RW-SEED-004" contains "seed") is clean as a VALUE under an
        // allowlisted name — that is the sanctioned outcome vocabulary.
        let code = vec![("outcome".to_string(), "RW-SEED-004".to_string())];
        assert!(
            first_violation(&code).is_none(),
            "a bare code is §5.4-clean"
        );
        // TEETH: the exemption is exact-shape only — every loosening still trips.
        for smuggle in [
            "RW-SEED-004 words follow", // a tail
            "seed RW-KEY-001",          // a prefix
            "RW-seed-004",              // lowercase area
            "RW-SEED-04",               // two digits
            "RW-SEED-0041",             // four digits
            "the seed words",           // no code at all
        ] {
            let f = vec![("outcome".to_string(), smuggle.to_string())];
            assert!(
                first_violation(&f).is_some(),
                "{smuggle:?} must still trip the guard"
            );
        }
        // And the exemption never covers the NAME position or the allowlist gate.
        let bad_name = vec![("seed".to_string(), "RW-SEED-004".to_string())];
        assert!(first_violation(&bad_name).is_some());
        let bad_field = vec![("not_a_field".to_string(), "RW-SEED-004".to_string())];
        assert!(first_violation(&bad_field).is_some());
    }

    /// Phase-3 P3-2: the second sanctioned vocabulary — a propose variant name on
    /// the `code` field — is exempt by EXACT membership in
    /// `send::PROPOSE_ERR_VARIANT_CODES`, and every loosening still trips: a tail,
    /// a real-address-shaped value, the same word on another allowlisted field, a
    /// member in the NAME position. Mutant (registry row): the
    /// `static_variant_code` exemption removed from `first_violation` → the first
    /// assertion reds on `address`.
    #[test]
    fn a_propose_variant_name_on_code_is_exempt_but_anything_looser_still_trips() {
        for member in [
            "address",
            "address_not_recognized",
            "key_not_recognized",
            "key_not_available",
            "balance_error",
        ] {
            assert!(
                crate::send::PROPOSE_ERR_VARIANT_CODES.contains(&member),
                "{member} must be a member of the static set this test exercises"
            );
            let f = vec![("code".to_string(), member.to_string())];
            assert!(
                first_violation(&f).is_none(),
                "the static variant name {member:?} on `code` is §5.4-clean: {:?}",
                first_violation(&f)
            );
        }
        for smuggle in [
            "address u1qq8w…",               // a member with a payload tail
            "u1address",                     // an address-shaped value
            "addresses",                     // a member plus a letter
            "t1KeyNotRecognized",            // mixed case, address-shaped
            "key_not_recognized: zxviews1…", // a member with a key tail
        ] {
            let f = vec![("code".to_string(), smuggle.to_string())];
            assert!(
                first_violation(&f).is_some(),
                "{smuggle:?} on `code` must still trip the guard"
            );
        }
        // The exemption is per FIELD: the same member on another allowlisted
        // name is a value scan like any other, and in the name position it is a
        // forbidden token like any other.
        let other_field = vec![("outcome".to_string(), "address".to_string())];
        assert!(first_violation(&other_field).is_some());
        let in_name = vec![("address".to_string(), "proposal".to_string())];
        assert!(first_violation(&in_name).is_some());
    }

    #[test]
    fn every_forbidden_token_trips_in_both_name_and_value_position() {
        // Iterate the const so adding a token auto-extends coverage — and so the
        // VALUE-position scan (the smuggle shape: a secret under an allowlisted name)
        // is proven for every token, not just the few a happy path renders.
        for bad in FORBIDDEN {
            let in_name = vec![((*bad).to_string(), "ok".to_string())];
            assert!(
                first_violation(&in_name).is_some(),
                "forbidden token {bad:?} must trip in the field NAME position"
            );
            let in_value = vec![("blocks".to_string(), (*bad).to_string())];
            assert!(
                first_violation(&in_value).is_some(),
                "forbidden token {bad:?} must trip in the field VALUE position (under an allowlisted name)"
            );
        }
    }

    #[test]
    fn a_url_value_trips_the_scan_where_a_catalog_id_is_clean() {
        // `sync-server-picker.md` §5.4 (the security review's fold): a
        // custom server's URL must never reach a log under ANY name — `://`
        // is the token that catches a rendered endpoint whole, scheme, host
        // and port, where the allowlisted `server_id` (a catalog id) and the
        // hyphenated kind are clean.
        let url = vec![(
            "server_id".to_string(),
            "https://mine.example:443".to_string(),
        )];
        assert!(
            first_violation(&url).is_some(),
            "a rendered URL under an allowlisted name must trip"
        );
        let id = vec![("server_id".to_string(), "zec-rocks".to_string())];
        assert!(first_violation(&id).is_none(), "a catalog id is clean");
    }

    #[test]
    fn a_clean_allowlisted_field_set_has_no_false_positive() {
        // The full intended emitted surface — the engine span fields + the controller
        // event fields + the two undriven-but-allowlisted crate fields — in one place.
        // Includes the new `outcome=error` value (must be §5.4-clean) and a real errno
        // as the `error` value (a benign `set_nodelay`-style failure).
        let clean = [
            ("message", "wallet.sync"),
            ("from", "280000"),
            ("to", "281001"),
            ("blocks", "1"),
            ("outcome", "advance"),
            ("outcome", "error"),
            ("batches", "2"),
            ("reorgs", "0"),
            ("backoff_secs", "30"),
            ("depth", "20"),
            ("error", "Connection refused (os error 61)"),
            ("network", "Test"),
            // broadcast `wallet.send` span+event (inc-2d-3-a): counts + outcome only
            // (`rejected` rides it since #307 — the reject-partial vs transport-partial split)
            ("message", "wallet.send"),
            ("txs", "2"),
            ("accepted", "1"),
            ("rejected", "1"),
            ("outcome", "partial"),
            // resubmission `wallet.resubmit` event (inc-2d-3-b-ii-B): tx_count + the per-pass
            // money-incomplete sub-tallies (stranded/ceiling/corrupt) + outcome — all COUNTS, now
            // non-zero in production post-gate-removal (the §5.4 SSOT covers the live event).
            ("message", "wallet.resubmit"),
            ("tx_count", "3"),
            ("stranded", "1"),
            ("ceiling", "1"),
            ("capped", "1"),
            ("corrupt", "1"),
            ("outcome", "ok"),
            // swap `wallet.swap` span (W-swap-3-c-2-ii): provider/direction/outcome codes
            // only. The outcome on failure is a stable RW-SWAP-NNN code — note it must NOT
            // be a `deposit_*` string (that would trip the `deposit` forbidden token).
            ("message", "wallet.swap"),
            ("provider", "near-intents"),
            ("direction", "out_of_zec"),
            ("direction", "into_zec"),
            ("outcome", "RW-SWAP-009"),
            ("outcome", "RW-SWAP-011"),
            // the `wallet.swap_port` boundary log: the internal cause as a payload-free code
            ("message", "wallet.swap_port"),
            ("code", "RW-STORE-004"),
            // swap status-poll `wallet.swap_poll` span (W-swap-3-c-3-i): provider name, a
            // coarse outcome code, and a transient-fault COUNT — never a status payload.
            ("message", "wallet.swap_poll"),
            ("outcome", "terminal"),
            ("outcome", "halted"),
            ("faults", "3"),
            // swap quote `wallet.swap_quote` span (§3.3b L5 / ADR-0530, IZ-1b): provider/direction
            // codes + the RW-SWAP-NNN outcome of the quote-time destination mint — never an address.
            ("message", "wallet.swap_quote"),
            ("outcome", "RW-SWAP-012"),
            // scoped UTXO poll `wallet.utxo_scan` (IZ-1b): the scoped-set SIZE is a count.
            ("message", "wallet.utxo_scan"),
            ("set_size", "2"),
            // §3.2i-2 2e-2b ephemeral DETECT `wallet.ephemeral_detect`: all COUNTS, own span.
            ("message", "wallet.ephemeral_detect"),
            ("scope_size", "3"),
            ("polled", "2"),
            ("skipped", "1"),
            ("detected", "1"),
            ("errored", "1"),
            ("outcome", "detected"),
            // §3.2i-2 2e-2b-ii stranded-row REAP `wallet.stranded_reap`: a COUNT only.
            ("message", "wallet.stranded_reap"),
            ("reaped", "1"),
            // §3.2i-2 2e-2b-v-2 manual SWEEP `wallet.ephemeral_sweep`: COUNTS + outcome only; the
            // aggregate recovered AMOUNT is NEVER a span field (§5.4 never-LOG).
            ("message", "wallet.ephemeral_sweep"),
            ("scanned", "3"),
            ("swept", "1"),
            ("failed", "1"),
            ("truncated", "1"),
            ("outcome", "swept"),
            // §3.2i-2 2e-2b-v-3 reservation-PRESSURE `wallet.reservation_pressure`: COUNTS + a bool only;
            // no address ever (the leaked-ephemeral ceiling gauge).
            ("message", "wallet.reservation_pressure"),
            ("outstanding", "7"),
            ("limit", "10"),
            ("at_ceiling", "false"),
            // T0-1c tip standing warn (`sync::tip_standing`): the endpoint's claimed tip beside
            // the reference it fell below, and which reference (a code). No endpoint identity.
            ("claimed", "3459779"),
            ("newest_known", "3459780"),
            ("outcome", "behind_bundle"),
            // §4q P-R3 belt (`wallet.sync`, INC-025): a code naming a check, beside the
            // reused `batches`/`reorgs` counts and `claimed` — no new name. §4q-R P-RR2
            // adds the second cause word, the same shape (the exit's DB read below
            // the tip with no un-re-queued rewind as the last action).
            ("outcome", "rewind_left_queue_empty"),
            ("outcome", "queue_empty_below_tip"),
            // GRACE-2 (§4v): the untrusted-capable-time latch warn and the discarded
            // out-of-range identity height, both static `outcome` codes on
            // `wallet.consensus` (no duration, no height, no time); and the parked
            // reader's unreadable-stamp event, a typed error's static code.
            ("outcome", "capable_time_untrusted"),
            ("outcome", "identity_height_out_of_range"),
            ("message", "wallet.parked_verdict"),
            ("code", "RW-STORE-001"),
            // §4u REW-1: the rewinding-streak report (`sync_controller::run_loop` /
            // `emit_synced`) — a code naming a check, beside the reused `backoff_secs`;
            // no new name. GRACE-2 and REW-1 were built blind of each other and both
            // widened this fixture; the fold takes both entries — they are values on
            // one allowlisted field, not competing edits.
            ("outcome", "rewinding_streak"),
            // swap-destination recovery `wallet.swap_destination_recovered` (§3.3b L5): a COUNT only.
            ("message", "wallet.swap_destination_recovered"),
            ("count", "1"),
            // SCAN-1 (§4o): the `wallet.sync` batch span's phase timings + output count,
            // and the pass event's sums + wall-clock — durations and one chain count.
            ("anchor_ms", "38"),
            ("dl_ms", "310"),
            ("scan_ms", "330"),
            ("snap_ms", "12"),
            ("chain_outputs", "2048"),
            ("wall_ms", "361400"),
            // S15-F1 phase A: the pass event's two pre-batch phases.
            ("tip_ms", "6100"),
            ("roots_ms", "52000"),
            // SCAN-2 (§4t) adds NO NAME: the reconcile's mismatch warn
            // `wallet.anchor_reconcile` carries `from` (the batch start, a height
            // the span already logs — the anchor compared sits at `from − 1`) and
            // `outcome = "anchor_mismatch"`, a code naming a CHECK: the anchor
            // derived from the endpoint's own blocks and the tree state it served
            // for the same height disagreed. No wallet fact, no endpoint identity.
            // §4t-run review rows 2 and 3 add `field` (a structure MEMBER name, one
            // of five static literals — never the value that differed) and a second
            // `outcome` code for the undo: the derived streak's writes truncated back
            // to the last FETCHED anchor before the pass fails.
            // SCAN-2 option (c) adds a THIRD code on the same event: the verified
            // hash-byte-order case, which keeps the work and CONTINUES DERIVING (the
            // "stop deriving" half of the decision was withdrawn — see
            // `Inner.hash_convention_reversed`). It gets its own code so a field
            // report separates one server's hex convention from two RPCs describing
            // two different chains, and it fires ONCE per session rather than once
            // per reconcile cadence.
            ("message", "wallet.anchor_reconcile"),
            ("outcome", "anchor_mismatch"),
            ("outcome", "anchor_hash_byte_order"),
            ("field", "height"),
            ("field", "hash"),
            ("field", "sapling"),
            ("field", "orchard"),
            ("field", "ironwood"),
            ("field", "unknown"),
            ("outcome", "anchor_streak_undone"),
            // S15-F1 phase B (ADR-0569) adds NO NAME: an incremental subtree-root
            // fetch that is retried, falls back or is re-classified logs a
            // `wallet.sync` warn with the already-allowlisted `pool` and one of seven
            // `outcome` codes, each naming a CHECK — never the start index, a
            // height or a count.
            ("pool", "sapling"),
            ("outcome", "roots_start_refused"),
            ("outcome", "roots_start_dropped"),
            ("outcome", "roots_start_failed"),
            ("outcome", "roots_start_ignored"),
            ("outcome", "roots_overlap_moved"),
            ("outcome", "roots_served_short"),
            ("outcome", "roots_start_timed_out"),
        ]
        .map(|(n, v)| (n.to_string(), v.to_string()));
        assert_eq!(
            first_violation(&clean),
            None,
            "the legitimate emitted field set must be §5.4-clean (no false positive)"
        );
    }

    /// FR-29 spec §5 / §8 T16: the five host-dialer events the bridge's cabi
    /// module emits pass the §5.4 guard — four with no field at all, and the
    /// descriptor event with exactly the FOUR closed values (rendered as the
    /// core enums' `Debug` names and a bare readiness; `transport_health`
    /// joined at C1/ABI v3) — while the shapes the
    /// spec forbids on them TRIP it: a destination host, the isolation key,
    /// the auth token. The allowlist entries above are what makes the clean
    /// half pass; the tripping half proves the guard still has teeth on the
    /// very names a future site would reach for.
    #[test]
    fn host_dialer_events_are_fields_free() {
        for event in [
            "wallet.host_dialer_registered",
            "wallet.host_dialer_replaced",
            "wallet.host_dialer_cleared",
            "wallet.host_dialer_retired",
        ] {
            let fields = [("message".to_string(), event.to_string())];
            assert_eq!(first_violation(&fields), None, "{event} is fields-free");
        }
        for (readiness, health) in [
            ("100", "Ready"),
            ("37", "Starting"),
            ("0", "Starting"),
            ("100", "Ready"),
            // The v3 case the axis exists for: FAILED at the readiness the
            // registrant MEASURED, which is what the log must be able to say.
            ("100", "Failed"),
            ("40", "Failed"),
        ] {
            let descriptor = [
                ("message", "wallet.host_dialer_descriptor"),
                ("transport_readiness", readiness),
                ("transport_health", health),
            ]
            .map(|(n, v)| (n.to_string(), v.to_string()));
            assert_eq!(
                first_violation(&descriptor),
                None,
                "the descriptor event carries only its two mechanical values"
            );
        }
        // What the events must NEVER carry (§5): every one of these trips.
        for (name, value) in [
            ("host", "zec.rocks"),
            ("destination", "zec.rocks:443"),
            ("isolation_key", "wallet-sync"),
            // An ALLOWLISTED name with a forbidden VALUE still trips — the
            // allowlist admits a field, never whatever is put in it. It must
            // name a field that is STILL allowlisted to test that: when this
            // case read `transport_exposure` it stopped proving anything the
            // moment that name left the list, since it would then trip on the
            // name alone.
            ("transport_readiness", "wallet-send-key"),
            // The privacy grade, off this line since an earlier revision — both halves, so
            // neither can return by being the "obvious" companion of the other.
            ("transport_exposure", "Hidden"),
            ("transport_isolation", "Supported"),
            ("transport_name", "Tor"),
            ("auth", "00ff"),
            ("op_id", "7"),
        ] {
            let fields = [
                (
                    "message".to_string(),
                    "wallet.host_dialer_descriptor".to_string(),
                ),
                (name.to_string(), value.to_string()),
            ];
            assert!(
                first_violation(&fields).is_some(),
                "{name}={value} must trip the guard on a host-dialer event"
            );
        }
        // The producers live in the BRIDGE crate on target `zec_wallet`, which
        // this capture layer cannot see (the wave review's MEDIUM): pin the one
        // field-carrying producer at its SOURCE, so the tuples above cannot
        // drift from what `log_descriptor` actually emits — a future
        // `host = …` there reds here, not in a device log.
        let cabi = include_str!("../../zec_wallet/rust/src/net_dialer_cabi.rs");
        let start = cabi
            .find("fn log_descriptor(")
            .expect("the cabi module's log_descriptor exists");
        let body = &cabi[start..];
        let body = &body[..body.find("\n}").expect("the fn closes")];
        let emitted: Vec<&str> = body
            .lines()
            .filter_map(|line| line.trim().split_once(" = ").map(|(name, _)| name))
            .collect();
        // THE FIELD-SET PIN (the joint change with the host). It is not
        // only "never the name" any more: `isolation` and `exposure` together
        // were a bijection onto the host's three transports, so this line
        // recovered which circumvention product a person runs from two
        // integers. The grade moved to the host's CONSENTED debug bundle; this
        // assertion is what stops it drifting back onto an unconsented device
        // log, and it holds whatever the host's roster grows into — unlike an
        // injectivity check over the roster, which would alarm when a pair is
        // SHARED (the disclosure narrowing) and stay green when a fourth
        // backing takes a distinct pair (the disclosure widening).
        assert_eq!(
            emitted,
            ["transport_readiness", "transport_health"],
            "log_descriptor emits exactly the two mechanical transport fields — never the \
             host's name, and never the privacy grade in any encoding"
        );
        // The pin above reads `field = value` lines; tracing's SHORTHAND forms
        // (`?d.name`, `%name`) emit a field with no ` = ` and would slip past
        // it (the security angle's LOW), so the body is also refused any
        // `.name` and any shorthand line.
        assert!(
            !body.contains(".name")
                && !body.lines().any(|line| {
                    let line = line.trim();
                    line.starts_with('?') || line.starts_with('%')
                }),
            "log_descriptor never emits the host's name, by shorthand or otherwise"
        );
    }

    #[test]
    fn an_unknown_field_name_trips_even_when_benign() {
        // A brand-new innocent-looking field still forces a §5.4 review — the allowlist
        // gate fires independently of the forbidden scan (the realistic regression: a
        // maintainer adds `latency_ms` and the guard must still demand review).
        let benign_unknown = vec![("latency_ms".to_string(), "5".to_string())];
        assert!(
            first_violation(&benign_unknown)
                .is_some_and(|m| m.contains("outside the §5.4 allowlist")),
            "an unknown (even benign) field name must trip the allowlist gate"
        );
    }

    #[test]
    fn the_forbidden_scan_is_case_insensitive() {
        // Real Debug renders capitalize (`Deposit`, `Txid(..)`, `Zatoshis(..)`, `Memo`);
        // the lowercasing on both sides is the defense. VALUE position under an
        // ALLOWLISTED name, so the ONLY possible trip is the forbidden value.
        for v in ["Deposit-XYZ", "TXID-abcd", "Zatoshis(7)", "MEMO bytes"] {
            let field = vec![("to".to_string(), v.to_string())];
            assert!(
                first_violation(&field).is_some_and(|m| m.contains("forbidden")),
                "case-insensitive value scan must trip on {v:?}"
            );
        }
        // NAME position: an UPPER-case forbidden name trips the FORBIDDEN scan (which
        // runs before — and reports distinctly from — the allowlist gate).
        let v = first_violation(&[("ADDRESS".to_string(), "x".to_string())]);
        assert!(
            v.is_some_and(|m| m.contains("forbidden token")),
            "an upper-case forbidden field NAME must trip the case-insensitive scan"
        );
    }

    #[test]
    fn a_real_balance_snapshot_debug_render_trips_the_guard() {
        // Render the ACTUAL type (not a hand-typed string) so a future Debug-derive
        // field rename can't silently open a hole between "what we assert" and "what a
        // stray `?balance` log would actually emit". BalanceSnapshot/WalletState are
        // never Debug-logged (the iii-B-1 + B-2-c security INFO) — this proves the
        // guard would catch a regression that tried, on the REAL render.
        let rendered = format!("{:?}", BalanceSnapshot::default());
        let leak = vec![("some_field".to_string(), rendered)];
        assert!(
            first_violation(&leak).is_some_and(|m| m.contains("forbidden")),
            "a logged BalanceSnapshot Debug render must trip the §5.4 guard on a value token"
        );
    }

    /// the ONE target predicate — read by the capture guard AND by the
    /// bridge's shipped device-log layer. It covers the bridge's `zec_wallet`
    /// (the seam: the host-dialer events were outside the guard) and it
    /// matches on a `::` boundary, because `zec_wallet` is a PREFIX of the Tor
    /// plugin's crate name and of any future sibling's: a bare `starts_with`
    /// would put their events on a user's device log with no review of what
    /// they emit.
    ///
    /// Watched against: `module_of` reduced to a bare `starts_with` (the
    /// borrowed-prefix half reds); the `zec_wallet` arm removed (the bridge half
    /// reds).
    #[test]
    fn the_sdk_target_predicate_covers_the_bridge_and_refuses_a_borrowed_prefix() {
        for ours in [
            "zec_wallet_core",
            "zec_wallet_core::net::dialer",
            "zec_wallet",
            "zec_wallet::net_dialer_cabi",
            "wallet.consensus",
            "wallet.enhance",
        ] {
            assert!(is_sdk_target(ours), "{ours} is this SDK's target");
        }
        for not_ours in [
            "zec_wallet_tor",
            "zec_wallet_tor::dialer",
            "zec_wallet_corex",
            "zec_wallet_core_extra::x",
            "zec_wallet_ui",
            "zec_wallet:",
            "dialer_tor::dialer",
            "zcash_client_backend::scanning",
            "h2::proto",
            "tonic::transport",
            "wallet",
            "wallet.",
            "wallet.9",
            "wallet.Consensus",
            "",
        ] {
            assert!(
                !is_sdk_target(not_ours),
                "{not_ours:?} must not inherit the SDK's log policy or its device log"
            );
        }
    }

    /// The per-field predicate a RUNTIME sink calls is the capture guard's own
    /// predicate, not a second implementation of it: on every shape the policy
    /// tests above exercise, `field_is_loggable` and `first_violation` agree.
    ///
    /// Watched against: `field_is_loggable` made `true` unconditionally.
    #[test]
    fn the_runtime_predicate_is_the_capture_guards_predicate() {
        for (name, value, loggable) in [
            ("outcome", "ok", true),
            ("dial_arm", "host", true),
            ("outcome", "RW-SEED-004", true),
            ("code", "address_not_recognized", true),
            ("requeued", "2", true),
            ("owed", "1", true),
            ("io_kind", "PermissionDenied", true),
            // a name outside the allowlist, however benign
            ("host", "zec.rocks", false),
            ("some_field", "1", false),
            // an allowlisted name carrying a forbidden value
            ("error", "dial https://zec.rocks:443 failed", false),
            ("outcome", "deposit address t1abc", false),
            // a forbidden NAME
            ("txid", "00ff", false),
            ("isolation_key", "wallet-sync", false),
        ] {
            assert_eq!(field_is_loggable(name, value), loggable, "{name}={value}");
            assert_eq!(
                first_violation(&[(name.to_string(), value.to_string())]).is_none(),
                loggable,
                "the two forms of the policy disagree on {name}={value}"
            );
        }
    }

    /// `verdict = ?verdict` on the `wallet.consensus` INFO line (`wallet.rs`) is
    /// the ONE structured `Debug` value that reaches a shipped log, and the name
    /// gate blesses it by NAME forever: a field added to
    /// `ConsensusCompatibility` — an endpoint identity, a tip hash — would ride
    /// that line to every device with no gate firing (the security
    /// review's MEDIUM; the `FORBIDDEN` doc's own "stray `?snapshot`" shape).
    /// So the render is pinned TWICE, because each pin sees what the other
    /// cannot. (1) The VOCABULARY: every identifier in the `Debug` of every
    /// variant is one reviewed here — heights, branch ids, a duration and a
    /// latch. A new NAMED field renders its name, so it lands here. (2) The
    /// EXACT STRING of every sample: a payload with no name to render — a tuple
    /// field, a wrapped type whose `Debug` changed, a hand-written `Debug` that
    /// starts printing bytes — adds no word at all (digits and brackets are
    /// separators to a word split: the quality pass's MAJOR), and only the
    /// whole string can see it. A new variant breaks the exhaustive match, a new
    /// field breaks a literal, and the whole render passes the value scan.
    ///
    /// Watched against: an identifier removed from the pinned set (the render
    /// is then "new" and reds — the shape a real new field would take).
    #[test]
    fn the_consensus_verdict_renders_only_a_reviewed_vocabulary() {
        use crate::consensus::{ConsensusCompatibility as V, GraceClock};
        use crate::money::BlockHeight;
        let h = BlockHeight::new(3_477_824);
        let samples = [
            (V::Current, "Current"),
            (
                V::Behind {
                    scanned_tip: h,
                    claimed_tip: h,
                },
                "Behind { scanned_tip: BlockHeight(3477824), claimed_tip: BlockHeight(3477824) }",
            ),
            (
                V::Unsupported {
                    expected_branch_id: 7,
                    endpoint_branch_id: Some(9),
                    judged_at_height: h,
                },
                "Unsupported { expected_branch_id: 7, endpoint_branch_id: Some(9), \
                 judged_at_height: BlockHeight(3477824) }",
            ),
            (
                V::Unsupported {
                    expected_branch_id: 7,
                    endpoint_branch_id: None,
                    judged_at_height: h,
                },
                "Unsupported { expected_branch_id: 7, endpoint_branch_id: None, \
                 judged_at_height: BlockHeight(3477824) }",
            ),
            (
                V::Unknown {
                    judged_at_height: h,
                    blocks_since_last_current: Some(12),
                    clock: GraceClock {
                        elapsed_secs: Some(86_399),
                        latched: true,
                    },
                },
                "Unknown { judged_at_height: BlockHeight(3477824), \
                 blocks_since_last_current: Some(12), \
                 clock: GraceClock { elapsed_secs: Some(86399), latched: true } }",
            ),
            (
                V::Unknown {
                    judged_at_height: h,
                    blocks_since_last_current: None,
                    clock: GraceClock::NONE,
                },
                "Unknown { judged_at_height: BlockHeight(3477824), \
                 blocks_since_last_current: None, \
                 clock: GraceClock { elapsed_secs: None, latched: false } }",
            ),
        ];
        // Exhaustive INSIDE the crate (`non_exhaustive` binds only downstream):
        // a new variant is a compile error here, and so reaches the samples.
        for (v, _) in &samples {
            match v {
                V::Current | V::Behind { .. } | V::Unsupported { .. } | V::Unknown { .. } => {}
            }
        }
        const REVIEWED: &[&str] = &[
            "Current",
            "Behind",
            "Unsupported",
            "Unknown",
            "scanned_tip",
            "claimed_tip",
            "judged_at_height",
            "BlockHeight",
            "expected_branch_id",
            "endpoint_branch_id",
            "blocks_since_last_current",
            "clock",
            "GraceClock",
            "elapsed_secs",
            "latched",
            "Some",
            "None",
            "true",
            "false",
        ];
        for (v, exact) in samples {
            let render = format!("{v:?}");
            assert_eq!(
                render, exact,
                "the `verdict` render changed shape — it is on an INFO line every device log \
                 carries: review what the new shape prints (§5.4), then re-pin it"
            );
            assert!(
                field_is_loggable("verdict", &render),
                "the verdict's render must pass the value scan: {render}"
            );
            for word in render
                .split(|c: char| !(c.is_ascii_alphabetic() || c == '_'))
                .filter(|w| !w.is_empty())
            {
                assert!(
                    REVIEWED.contains(&word),
                    "`{word}` is new in the `verdict` render ({render}) — it reaches every \
                     device log on an INFO line: review what it carries (§5.4), then add it here"
                );
            }
        }
    }
}
