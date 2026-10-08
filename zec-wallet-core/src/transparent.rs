//! Recv-2b — transparent UTXO detection (spec §3.3a, ADR-0528).
//!
//! A `CompactBlock` carries only Sapling/Orchard *compact* outputs, so
//! `scan_cached_blocks` is SHIELDED-ONLY: a transparent receive to the wallet's receive
//! address (external index 0, ADR-0528) is invisible to it. The ONLY path that surfaces one
//! is a separate lightwalletd `GetAddressUtxos` poll ([`crate::net::grpc`]'s
//! `get_address_utxos`), validated here at the §4.6 hostile-input boundary and handed to the
//! audited engine via [`WalletWrite::put_received_transparent_utxo`]. The balance fold
//! (`account::fold_account_balances`) already sums the engine's `unshielded_balance().total()`,
//! so a put here is precisely what makes [`BalanceSnapshot::transparent`] nonzero.
//!
//! ## Trust boundary (M2 lying endpoint, §4.6)
//! The endpoint is untrusted. EVERY field of every returned UTXO is validated, and —
//! defense-in-depth — the recipient is RE-DERIVED from the output `script_pubkey` and required
//! to equal OUR receive address. The reply's own `address` field is never trusted; a UTXO for
//! an address we do not own is rejected, never put (so a phantom balance of funds we cannot
//! spend can never appear). The audited engine applies the SAME check again
//! (`AddressNotRecognized` for an address absent from its `addresses` table), so a foreign UTXO
//! is rejected at two independent layers. A malformed field rejects that ONE record (counted
//! for the §5.4 span), never the whole poll, never a panic.
//!
//! §5.4: addresses / amounts / txids / scripts NEVER log. Only the [`TransparentRefreshOutcome`]
//! COUNTS are observable.
//!
//! [`BalanceSnapshot::transparent`]: crate::state::BalanceSnapshot::transparent

use zcash_client_backend::data_api::WalletWrite;
use zcash_client_backend::wallet::WalletTransparentOutput;
use zcash_client_sqlite::AccountUuid;
use zcash_client_sqlite::error::SqliteClientError;
use zcash_protocol::consensus::BlockHeight;
use zcash_protocol::value::Zatoshis;
use zcash_script::script::Code;
use zcash_transparent::address::{Script, TransparentAddress};
use zcash_transparent::bundle::{OutPoint, TxOut};

use crate::constants::MAX_TRANSPARENT_SCRIPT_BYTES;
use crate::db::WalletConn;
use crate::error::WalletError;
use crate::state::StallReason;

/// A raw, UN-VALIDATED transparent UTXO as the endpoint reported it — the owned transport DTO
/// behind [`crate::sync::TransparentUtxoSource`], decoupling the sync seam (and its test fakes)
/// from the gRPC proto type. Mirrors `GetAddressUtxosReply`; trusted for NOTHING until
/// [`validate`]. The proto → record mapping lives in [`crate::net::grpc`] (the one place that
/// owns the proto), keeping this module proto-free.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TransparentUtxoRecord {
    /// Transaction id in INTERNAL (consensus) byte order — exactly what
    /// [`OutPoint::new`]/`zcash_protocol::TxId::from_bytes` expect (NOT block-explorer/display
    /// order; the reply delivers internal order, like the send path's `claim.txid`).
    pub(crate) txid: Vec<u8>,
    /// Output index within the transaction (the proto's signed `i32`; a real index is ≥ 0).
    pub(crate) index: i32,
    /// The output `script_pubkey` bytes (raw, no length prefix).
    pub(crate) script: Vec<u8>,
    /// Value in zatoshis (the proto's signed `i64`; a real value is in `[0, MAX_MONEY]`).
    pub(crate) value_zat: i64,
    /// Mined height (the proto's `u64`; a real Zcash height fits a `u32`).
    pub(crate) height: u64,
}

/// Why a returned UTXO was rejected — for the §5.4 detection span/log (a COUNT per reason,
/// never the offending bytes). Each arm is a malformed-or-hostile reply, handled honestly by
/// skipping that one record, never a panic, never a silent drop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UtxoReject {
    /// `txid` was not exactly 32 bytes.
    TxidLength,
    /// `index` was negative (not a valid output index).
    NegativeIndex,
    /// `value_zat` was negative or above the `MAX_MONEY` consensus bound.
    ValueOutOfRange,
    /// `height` did not fit a `u32` (Zcash heights are `u32`).
    HeightOutOfRange,
    /// The `script_pubkey` was longer than [`MAX_TRANSPARENT_SCRIPT_BYTES`] — never a standard
    /// transparent receive output (rejected before the engine parses it).
    ScriptTooLong,
    /// The script was not a wallet-recognizable output pattern (not P2PKH/P2SH).
    ScriptUnrecognized,
    /// The recipient re-derived FROM THE SCRIPT was NOT our receive address — the M2
    /// lying-endpoint case (a UTXO we do not own).
    AddressMismatch,
}

/// The outcome of one refresh — COUNTS only, so it is §5.4-safe to put on a tracing span.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TransparentRefreshOutcome {
    /// UTXOs validated + put (idempotent; a re-put of an already-known UTXO still counts here).
    pub(crate) put: usize,
    /// Records rejected at the validation boundary (malformed/hostile) — skipped, never put.
    pub(crate) rejected: usize,
}

/// One expected receiver in a SCOPED poll (§3.3b D2 / ADR-0530, IZ-1b): the index-0 receive
/// address (`swap_id = None`) or an active swap DESTINATION (`swap_id = Some(quote id)`). The poll
/// validates each returned UTXO against the SET and attributes a put to the matching entry — so the
/// shield-retirement signal ([`crate::swap_destination_store::debounced_retire_and_mark_served`])
/// knows WHICH destinations received funds this pass. The `swap_id` is a §5.4 NEVER-LOG item (it is the deposit address for
/// OutOfZec / a swap handle) — it drives the store update only, never a span.
pub(crate) struct ExpectedReceiver {
    pub(crate) swap_id: Option<String>,
    pub(crate) receiver: TransparentAddress,
}

/// The outcome of one SCOPED refresh — COUNTS (§5.4-safe to span) plus the set of swap ids that
/// received a UTXO this pass (internal — drives the destination store's funded/retire transition,
/// NEVER logged). `put`/`rejected` mirror [`TransparentRefreshOutcome`].
pub(crate) struct ScopedRefreshOutcome {
    pub(crate) put: usize,
    pub(crate) rejected: usize,
    /// More NEW (not already stored unspent) outputs VALIDATED than
    /// [`SCOPED_UTXO_POLL_MAX_PUTS`] (S7 S1): some new receive was not put this pass and lands
    /// on a later one. Counted apart from `rejected` — an honest high-UTXO address is not a
    /// lying server. Reported on the `wallet.utxo_scan` span only; it gates nothing.
    ///
    /// [`SCOPED_UTXO_POLL_MAX_PUTS`]: crate::constants::SCOPED_UTXO_POLL_MAX_PUTS
    pub(crate) truncated: bool,
    /// The stored-unspent read FAILED this pass ([`StoredOutputs::unread`]): every output was
    /// ranked NEW — money-safe, only the put order degraded. Span outcome only.
    pub(crate) stored_unread: bool,
    /// swap ids (NEVER-log) whose destination address received ≥1 UTXO this pass.
    pub(crate) funded_swap_ids: std::collections::HashSet<String>,
}

impl ScopedRefreshOutcome {
    /// The `wallet.utxo_scan` span's `outcome` value — a closed static set (§5.4 counts-only).
    pub(crate) fn outcome(&self) -> &'static str {
        if self.stored_unread {
            "stored_unread"
        } else if self.truncated {
            "truncated"
        } else {
            "ok"
        }
    }
}

/// Field-validate ONE raw record at the §4.6 hostile-input boundary into an audited
/// [`WalletTransparentOutput`] — every wire field, NO recipient match yet. The shared core of the
/// single-receiver [`validate`] and the scoped [`validate_scoped`] (DRY: one field-validation door,
/// one place to get a size-cap or byte-order wrong). Pure + total — no panic on ANY byte sequence.
fn validate_fields(
    record: &TransparentUtxoRecord,
) -> Result<WalletTransparentOutput<AccountUuid>, UtxoReject> {
    // txid: exactly 32 bytes, INTERNAL order as the reply delivers it (the same door the send
    // path's `claim.txid` crosses); `OutPoint::new` wraps these bytes verbatim.
    let txid: [u8; 32] = record
        .txid
        .as_slice()
        .try_into()
        .map_err(|_| UtxoReject::TxidLength)?;
    // index: a transaction output index is non-negative.
    let n = u32::try_from(record.index).map_err(|_| UtxoReject::NegativeIndex)?;
    // value: the consensus money bound `[0, MAX_MONEY]` (rejects negative AND over-cap).
    let value = Zatoshis::from_nonnegative_i64(record.value_zat)
        .map_err(|_| UtxoReject::ValueOutOfRange)?;
    // height: Zcash heights are u32; a u64 above that is a malformed reply.
    let height = u32::try_from(record.height)
        .map(BlockHeight::from_u32)
        .map_err(|_| UtxoReject::HeightOutOfRange)?;
    // script: cap the length BEFORE we hand the bytes to the parser (§4.6 size-cap; a standard
    // transparent receive script is ≤ 25 bytes). The outer message-size cap bounds the whole
    // reply; this bounds each record.
    if record.script.len() > MAX_TRANSPARENT_SCRIPT_BYTES {
        return Err(UtxoReject::ScriptTooLong);
    }

    let outpoint = OutPoint::new(txid, n);
    // Reconstruct the output script from the endpoint's RAW scriptPubKey bytes (audited
    // `Script`/`Code`, no custom parsing). `from_parts` delegates to `TxOut::recipient_address`
    // (→ the audited `script::PubKey::parse` + `solver::standard`) and returns None when that
    // derivation finds no recognized wallet output pattern (not P2PKH/P2SH).
    let txout = TxOut::new(value, Script(Code(record.script.clone())));
    // 0.24.0 widens `from_parts` with three ACCOUNT-ATTRIBUTION fields. We supply `None` for
    // all three ON PURPOSE, and it is not a stub: this record came off the wire from an
    // untrusted endpoint, so we have no attribution to assert. The engine derives the real
    // account and key scope from its OWN `addresses` table by the re-derived recipient
    // (`put_transparent_output` queries `cached_transparent_receiver_address` and returns
    // `AddressNotRecognized` otherwise) and reads none of these three fields on the put path
    // — so a value here could only be a claim the engine ignores or, worse, one a future
    // version starts trusting. `funding_account` is likewise unknowable: we see an output,
    // never which of our accounts paid for it.
    WalletTransparentOutput::from_parts(outpoint, txout, Some(height), None, None, None)
        .ok_or(UtxoReject::ScriptUnrecognized)
}

/// Validate ONE raw record at the §4.6 hostile-input boundary into an audited
/// [`WalletTransparentOutput`], or a typed [`UtxoReject`]. `expected` is OUR receive address;
/// the recipient re-derived from the output script MUST equal it (the reply's own address field
/// is never consulted). Pure + total — no panic on ANY byte sequence, so it is the natural unit
/// of the §8 hostile-input KATs.
pub(crate) fn validate(
    record: &TransparentUtxoRecord,
    expected: &TransparentAddress,
) -> Result<WalletTransparentOutput<AccountUuid>, UtxoReject> {
    let output = validate_fields(record)?;
    // DEFENSE-IN-DEPTH (§4.6 / M2): the recipient is the address re-derived FROM THE SCRIPT, not
    // the endpoint's claim. Require it to be OUR receive address — a UTXO for any other address
    // is rejected, never put (never inflates the balance with funds we cannot spend).
    if output.recipient_address() != expected {
        return Err(UtxoReject::AddressMismatch);
    }
    Ok(output)
}

/// Validate ONE record against a SET of expected receivers (the §3.3b D2 scoped poll), returning the
/// audited output AND the INDICES of EVERY matched `expecteds` entry, or
/// [`UtxoReject::AddressMismatch`] if the re-derived recipient is none of ours. The SAME §4.6 / M2
/// re-derivation as [`validate`], generalized from one address to the scoped set
/// `{ index-0 } ∪ { active swap destinations }` — a UTXO for an address we did not ask a provider
/// to pay (and is not our receive address) is rejected, never put. Pure + total.
///
/// ALL matches, not first-match (#382 fold — the money-review MED): the addresses are
/// distinct single-use indices in the steady state, but TWIN rows on one receiver are reachable
/// (a synthetic `backfill:<index>` row armed beside a record's re-armed row after a rescan, or a
/// pre-#368 legacy shape) — under first-match-wins, whichever twin sorted first in `active()`
/// starved the other of the funded credit, and with it the #382 chain-observed `Refunded` pin
/// (a `backfill:` id has no record, so its pin attempt is a permanent no-op). Crediting every
/// matching expected is idempotent everywhere downstream: `mark_funded` re-marks, the pin
/// first-wins, and the debounced retire deletes by id.
fn validate_scoped(
    record: &TransparentUtxoRecord,
    expecteds: &[ExpectedReceiver],
) -> Result<(WalletTransparentOutput<AccountUuid>, Vec<usize>), UtxoReject> {
    let output = validate_fields(record)?;
    let recipient = output.recipient_address();
    let matched: Vec<usize> = expecteds
        .iter()
        .enumerate()
        .filter(|(_, expected)| recipient == &expected.receiver)
        .map(|(i, _)| i)
        .collect();
    if matched.is_empty() {
        return Err(UtxoReject::AddressMismatch);
    }
    Ok((output, matched))
}

/// Validate every record against the SCOPED receiver set (`{ index-0 } ∪ { active swap
/// destinations }`, §3.3b D2 / ADR-0530) and put each survivor via the audited
/// [`WalletWrite::put_received_transparent_utxo`] (idempotent `ON CONFLICT` upsert keyed on the
/// outpoint, so re-polling the same unspent UTXO is a no-op write). Rejected records are COUNTED,
/// never put, never silently dropped. Returns COUNTS (§5.4-safe) plus the set of swap ids whose
/// destination received a UTXO this pass — the funded/retire signal for
/// [`crate::swap_destination_store`] (internal, NEVER logged).
///
/// Runs under the wallet db lock (the caller holds it); each `put` opens its own short write and
/// requires the chain tip to already be recorded (the sync pass records it before this runs). The
/// audited engine independently re-checks each put against its `addresses` table
/// (`AddressNotRecognized` for any address it never registered — and the swap destinations ARE
/// engine-registered by IZ-1a's `get_address_for_index`), so a foreign UTXO is rejected at two
/// layers. A `put` fault stops the refresh with a typed error; the caller logs it (§5.4 code only)
/// and SWALLOWS it so the sync loop never fails — the chain is the source of truth, the next pass
/// retries (§6.3, like the resubmission hook). The error taxonomy stays HONEST:
/// [`ChainHeightUnknown`](zcash_client_sqlite::error::SqliteClientError::ChainHeightUnknown) (the
/// tip not yet recorded — unreachable in the production call order, but a defense against a future
/// reorder) is a RETRYABLE [`StallReason::Internal`], NOT corruption; every other fault is the
/// classified (`ClassifyStoreFault`, R12). The upstream error is matched by VARIANT, never
/// rendered (its `Debug` can echo an address, §5.4).
///
/// **The put cap (S7 S1, §4.6).** EVERY record is validated first, and every validated match
/// attributes its swap destination; then at most
/// [`SCOPED_UTXO_POLL_MAX_PUTS`](crate::constants::SCOPED_UTXO_POLL_MAX_PUTS) outputs are PUT, so a
/// flooding endpoint cannot buy unbounded lock time or bloat in one pass. The put ORDER is what
/// keeps the cap from hiding money (the S7 fold): NEW outputs first — swap-destination matches
/// before index-0 — and only then the outputs `stored` says the engine already holds unspent.
/// Their re-put is an idempotent upsert that refreshes the engine's `max_observed_unspent_height`
/// AND restores a reorg-nulled `transactions.mined_height`, so the stored tail is ROTATED: oldest
/// observation first (`NULL`, then ascending), so a tail the cap skips is not the same one every
/// pass. So ~1025 dust UTXOs the wallet already stores can never starve a later NEW receive.
/// `truncated` counts only NEW outputs over the cap (never `rejected`). The phantom balance a
/// lying endpoint can still show is accepted by the spec; this bounds its rate.
#[cfg(test)]
pub(crate) fn refresh_account_transparent_scoped(
    wdb: &mut WalletConn,
    expecteds: &[ExpectedReceiver],
    records: &[TransparentUtxoRecord],
) -> Result<ScopedRefreshOutcome, WalletError> {
    refresh_account_transparent_scoped_known(wdb, expecteds, records, &StoredOutputs::default())
}

/// An outpoint in the engine's own terms: INTERNAL-order txid and output index.
pub(crate) type StoredOutpoint = ([u8; 32], u32);

/// The engine's stored-unspent transparent outputs, as the scoped poll's put ORDER reads them:
/// each outpoint with its `max_observed_unspent_height` (`None` = NULL). `unread` says the read
/// failed and the set is EMPTY — every output then ranks NEW, which is money-safe (nothing is
/// hidden; only the order degrades), so a failed read never aborts the money-seeing poll.
#[derive(Default)]
pub(crate) struct StoredOutputs {
    pub(crate) observed: std::collections::HashMap<StoredOutpoint, Option<u32>>,
    pub(crate) unread: bool,
}

/// Every transparent output the engine stores for this wallet with NO recorded spend — read on
/// the aux connection to the same `wallet.db` file (the engine's `WalletDb` exposes no connection
/// accessor; [`crate::delivery`] reads `transactions` the same way). Schema:
/// `zcash_client_sqlite-0.22.0` `transparent_received_outputs` (`transaction_id` →
/// `transactions.id_tx`, `output_index`, `max_observed_unspent_height`) and
/// `transparent_received_output_spends` (`transparent_received_output_id`). A row this read
/// misses is merely treated as NEW — put earlier, never lost — so the read decides ORDER only,
/// never whether money is seen. INFALLIBLE by design: any aux fault or malformed row yields the
/// empty set with `unread` (see [`StoredOutputs`]).
pub(crate) fn stored_unspent_outputs(aux: &rusqlite::Connection) -> StoredOutputs {
    match read_stored_unspent(aux) {
        Ok(observed) => StoredOutputs {
            observed,
            unread: false,
        },
        Err(_) => StoredOutputs {
            observed: std::collections::HashMap::new(),
            unread: true,
        },
    }
}

fn read_stored_unspent(
    aux: &rusqlite::Connection,
) -> Result<std::collections::HashMap<StoredOutpoint, Option<u32>>, WalletError> {
    let mut stmt = aux
        .prepare(
            "SELECT t.txid, tro.output_index, tro.max_observed_unspent_height \
             FROM transparent_received_outputs tro \
             JOIN transactions t ON t.id_tx = tro.transaction_id \
             LEFT JOIN transparent_received_output_spends s \
               ON s.transparent_received_output_id = tro.id \
             WHERE s.transaction_id IS NULL",
        )
        .map_err(crate::db::map_aux_err)?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, Vec<u8>>(0)?,
                r.get::<_, u32>(1)?,
                r.get::<_, Option<u32>>(2)?,
            ))
        })
        .map_err(crate::db::map_aux_err)?;
    let mut out = std::collections::HashMap::new();
    for row in rows {
        let (txid, index, observed) = row.map_err(crate::db::map_aux_err)?;
        let txid: [u8; 32] = txid
            .as_slice()
            .try_into()
            .map_err(|_| WalletError::StoreCorrupt)?;
        out.insert((txid, index), observed);
    }
    Ok(out)
}

/// `refresh_account_transparent_scoped` with the engine's stored-unspent outputs (from
/// [`stored_unspent_outputs`]) — the production door.
pub(crate) fn refresh_account_transparent_scoped_known(
    wdb: &mut WalletConn,
    expecteds: &[ExpectedReceiver],
    records: &[TransparentUtxoRecord],
    stored: &StoredOutputs,
) -> Result<ScopedRefreshOutcome, WalletError> {
    let mut outcome = ScopedRefreshOutcome {
        put: 0,
        rejected: 0,
        truncated: false,
        stored_unread: stored.unread,
        funded_swap_ids: std::collections::HashSet::new(),
    };
    let mut validated = Vec::new();
    for record in records {
        match validate_scoped(record, expecteds) {
            Ok((output, matched)) => {
                // Attribute the funded UTXO to EVERY matching swap destination (index-0 has
                // `swap_id = None` and is skipped; twins on one receiver each get the credit —
                // see `validate_scoped`). Drives the §3.3b L3a shield-retirement signal and
                // the #382 chain-observed pin. From EVERY validated match, put or not (S7 S1):
                // the cap below must never make a funded destination read empty.
                let mut to_destination = false;
                for i in matched {
                    if let Some(swap_id) = &expecteds[i].swap_id {
                        outcome.funded_swap_ids.insert(swap_id.clone());
                        to_destination = true;
                    }
                }
                let outpoint = (*output.outpoint().hash(), output.outpoint().n());
                // Put order (S7 fold): new destination, new index-0, then already-stored — the
                // stored ones oldest-observed first (`None` sorts before `Some`), a rotation.
                let observed = stored.observed.get(&outpoint);
                let rank = match (observed.is_some(), to_destination) {
                    (false, true) => 0,
                    (false, false) => 1,
                    (true, true) => 2,
                    (true, false) => 3,
                };
                validated.push(((rank, observed.copied().flatten()), output));
            }
            Err(_reject) => {
                // §5.4: the rejected bytes (txid/script/amount) are NEVER logged; only the
                // aggregate count rides the caller's `wallet.utxo_scan` span.
                outcome.rejected += 1;
            }
        }
    }
    let cap = crate::constants::SCOPED_UTXO_POLL_MAX_PUTS;
    validated.sort_by_key(|(key, _)| *key); // stable: reply order within a key
    outcome.truncated = validated.iter().filter(|((rank, _), _)| *rank < 2).count() > cap;
    for (_, output) in validated.iter().take(cap) {
        wdb.put_received_transparent_utxo(output)
            .map_err(|e| match e {
                SqliteClientError::ChainHeightUnknown => WalletError::Sync {
                    stall: StallReason::Internal,
                },
                other => crate::sync::ClassifyStoreFault::into_store_fault(other),
            })?;
        outcome.put += 1;
    }
    Ok(outcome)
}

/// Validate + put the UTXOs an ISOLATED poll returned for ONE wallet-reserved EPHEMERAL
/// transparent address (the §3.2i-2 2e-2b detect). The expected set is the SINGLE queried
/// ephemeral, so a lying endpoint cannot attribute another ephemeral's (or a foreign) UTXO to
/// it — the SAME §4.6 / M2 re-derivation as the single-receiver [`validate`], applied per-record.
/// This is the recognition half of the detect (the network half is the per-ephemeral fresh-circuit
/// poll in [`crate::ephemeral_detect`]); it deliberately does NOT carry the scoped poll's swap
/// attribution ([`refresh_account_transparent_scoped_known`]) — an ephemeral is never a swap destination.
///
/// Idempotent: [`WalletWrite::put_received_transparent_utxo`] is an `ON CONFLICT` upsert keyed on
/// the outpoint, so a re-poll of the same unspent ephemeral UTXO is a no-op write (the §3.2i-2
/// put-before-reap battery rule rides on this). The ephemeral address is wallet-RESERVED by the
/// send path, so the engine's own `addresses`-table re-check recognises it; a never-reserved
/// address would be `AddressNotRecognized`, which is UNREACHABLE here because the caller enumerates
/// FROM the engine's known-ephemeral list (`get_ephemeral_transparent_receivers`). Same put-error
/// taxonomy as [`refresh_account_transparent_scoped_known`]: a not-yet-recorded chain tip is a RETRYABLE
/// [`StallReason::Internal`] (defense against a future call-order change — unreachable in the
/// production order where the scan records the tip first), every other fault the
/// classified (`ClassifyStoreFault`, R12; matched by VARIANT, never rendered — §5.4).
/// Returns COUNTS only (§5.4-safe to span).
pub(crate) fn recognise_ephemeral_outputs(
    wdb: &mut WalletConn,
    ephemeral: &TransparentAddress,
    records: &[TransparentUtxoRecord],
) -> Result<TransparentRefreshOutcome, WalletError> {
    let mut outcome = TransparentRefreshOutcome::default();
    // §4.6 defense-in-depth: bound the per-ephemeral record set against a hostile/buggy endpoint that
    // floods the reply with (phantom) UTXOs paying our queried ephemeral (up to the 8 MiB gRPC frame)
    // to amplify the put-loop under the db lock + a later sweep's prove + permanent DB bloat. A
    // legitimate one-time address never approaches this, so an honest reply is never truncated; the
    // excess is a boundary REFUSAL (counted rejected, §5.4 counts-only). Shared by detect + the sweep.
    let cap = crate::constants::EPHEMERAL_RECOGNISE_MAX_UTXOS;
    if records.len() > cap {
        outcome.rejected += records.len() - cap;
    }
    for record in records.iter().take(cap) {
        match validate(record, ephemeral) {
            Ok(output) => {
                wdb.put_received_transparent_utxo(&output)
                    .map_err(|e| match e {
                        SqliteClientError::ChainHeightUnknown => WalletError::Sync {
                            stall: StallReason::Internal,
                        },
                        other => crate::sync::ClassifyStoreFault::into_store_fault(other),
                    })?;
                outcome.put += 1;
            }
            Err(_reject) => {
                // §5.4: the rejected bytes (txid/script/amount) are NEVER logged — only the count.
                outcome.rejected += 1;
            }
        }
    }
    Ok(outcome)
}

/// Fuzz hook (re-exported as `zec_wallet_core::__fuzz_validate_utxo`) — drives the §4.6
/// hostile-input boundary [`validate`] over arbitrary bytes from the `cargo-fuzz` harness
/// (`fuzz_transparent_validate`, the sibling of `__fuzz_parse_subtree_root`). Every field of a
/// `GetAddressUtxos` reply item — txid length, output index, script bytes, value, height — arrives
/// from an UNTRUSTED endpoint, so this must NEVER panic on any input: it returns whether the
/// record validated to OUR (fixed, arbitrary) receive address, never a crash (the FFI no-panic
/// contract; principle 7). The example KATs + the `validate_never_panics_on_arbitrary_input`
/// proptest sample this; the fuzz target explores it coverage-guided.
#[cfg(fuzzing)]
#[doc(hidden)]
pub fn __fuzz_validate_utxo(
    txid: Vec<u8>,
    index: i32,
    script: Vec<u8>,
    value_zat: i64,
    height: u64,
) -> bool {
    // A fixed, structurally-valid P2PKH stands in for the wallet's receive address; the fuzzer
    // explores the WIRE fields, not the match target (a foreign/garbage script simply rejects).
    let expected = TransparentAddress::PublicKeyHash([0x11; 20]);
    let record = TransparentUtxoRecord {
        txid,
        index,
        script,
        value_zat,
        height,
    };
    validate(&record, &expected).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE valid receive address every hostile-input KAT matches against — a P2PKH whose 20-byte
    /// key hash is distinct + non-trivial (not all-zero, so a mismatch is visibly different).
    fn expected_addr() -> TransparentAddress {
        TransparentAddress::PublicKeyHash([0x11; 20])
    }

    /// The canonical scriptPubKey bytes that pay `addr` — built from the AUDITED
    /// `TransparentAddress::script()` (so the test never hand-rolls a script; the production
    /// `validate` reconstructs the SAME shape from the wire bytes).
    fn script_bytes(addr: &TransparentAddress) -> Vec<u8> {
        Script::from(addr.script()).0.0
    }

    /// A well-formed record paying `expected_addr()` at the given outpoint/value — the happy base
    /// every negative case mutates one field of.
    fn good_record() -> TransparentUtxoRecord {
        TransparentUtxoRecord {
            txid: vec![0xAB; 32],
            index: 0,
            script: script_bytes(&expected_addr()),
            value_zat: 100_000,
            height: 2_000_000,
        }
    }

    #[test]
    fn validate_accepts_a_well_formed_receive_utxo() {
        let out = validate(&good_record(), &expected_addr()).expect("valid P2PKH to our address");
        assert_eq!(out.recipient_address(), &expected_addr());
        assert_eq!(u64::from(out.value()), 100_000);
        assert_eq!(out.outpoint().n(), 0);
    }

    #[test]
    fn validate_preserves_internal_txid_byte_order() {
        // §8 (money / the txid-door invariant): the reply's txid is INTERNAL (consensus) order,
        // and `OutPoint` stores it verbatim — NO reversal here (the display-order reversal is the
        // separate `money::TxId` rendering door). A mistaken reversal would point the outpoint at
        // a different transaction (the UTXO would never confirm/spend). Distinct ascending bytes
        // make any reversal visible.
        let mut rec = good_record();
        rec.txid = (0u8..32).collect();
        let out = validate(&rec, &expected_addr()).expect("valid");
        assert_eq!(
            out.outpoint().hash(),
            &<[u8; 32]>::try_from((0u8..32).collect::<Vec<_>>()).unwrap(),
            "the outpoint txid is the reply's bytes verbatim (internal order, no reversal)"
        );
    }

    #[test]
    fn validate_rejects_a_utxo_for_a_different_address() {
        // THE M2 lying-endpoint defense: a structurally-valid P2PKH that pays a DIFFERENT address
        // (we queried our own, but the endpoint returned someone else's) is rejected — never put,
        // so the balance can never inflate with funds we cannot spend.
        let mut rec = good_record();
        rec.script = script_bytes(&TransparentAddress::PublicKeyHash([0x22; 20]));
        assert_eq!(
            validate(&rec, &expected_addr()),
            Err(UtxoReject::AddressMismatch)
        );
    }

    #[test]
    fn validate_rejects_a_p2sh_script_not_our_p2pkh() {
        // A recognized-but-wrong-kind script (P2SH) still re-derives to a DIFFERENT address than
        // our P2PKH receive address ⇒ AddressMismatch (not silently accepted).
        let mut rec = good_record();
        rec.script = script_bytes(&TransparentAddress::ScriptHash([0x11; 20]));
        assert_eq!(
            validate(&rec, &expected_addr()),
            Err(UtxoReject::AddressMismatch)
        );
    }

    #[test]
    fn validate_rejects_a_short_txid() {
        let mut rec = good_record();
        rec.txid = vec![0xAB; 31];
        assert_eq!(
            validate(&rec, &expected_addr()),
            Err(UtxoReject::TxidLength)
        );
    }

    #[test]
    fn validate_rejects_a_long_txid() {
        let mut rec = good_record();
        rec.txid = vec![0xAB; 33];
        assert_eq!(
            validate(&rec, &expected_addr()),
            Err(UtxoReject::TxidLength)
        );
    }

    #[test]
    fn validate_rejects_a_negative_output_index() {
        let mut rec = good_record();
        rec.index = -1;
        assert_eq!(
            validate(&rec, &expected_addr()),
            Err(UtxoReject::NegativeIndex)
        );
    }

    #[test]
    fn validate_rejects_a_negative_value() {
        let mut rec = good_record();
        rec.value_zat = -1;
        assert_eq!(
            validate(&rec, &expected_addr()),
            Err(UtxoReject::ValueOutOfRange)
        );
    }

    #[test]
    fn validate_rejects_a_value_above_max_money() {
        // MAX_MONEY = 21_000_000 * 1e8 zat. One zatoshi over is rejected (the consensus bound is
        // enforced by `Zatoshis::from_nonnegative_i64`, not by us — audited brick).
        let mut rec = good_record();
        rec.value_zat = 21_000_000 * 100_000_000 + 1;
        assert_eq!(
            validate(&rec, &expected_addr()),
            Err(UtxoReject::ValueOutOfRange)
        );
    }

    #[test]
    fn validate_accepts_a_zero_value_utxo() {
        // A 0-value UTXO is unusual but consensus-valid; it must not panic or reject (it simply
        // contributes 0 to the balance) — an honest pass-through, not a special case.
        let mut rec = good_record();
        rec.value_zat = 0;
        let out = validate(&rec, &expected_addr()).expect("zero value is valid");
        assert_eq!(u64::from(out.value()), 0);
    }

    #[test]
    fn validate_rejects_a_height_above_u32() {
        let mut rec = good_record();
        rec.height = u64::from(u32::MAX) + 1;
        assert_eq!(
            validate(&rec, &expected_addr()),
            Err(UtxoReject::HeightOutOfRange)
        );
    }

    #[test]
    fn validate_rejects_oversized_script() {
        // §8 boundary (gate 7): exactly at the cap is parsed (and rejected as unrecognized, since
        // 64 bytes of 0x00 is not P2PKH/P2SH); one byte over the cap is rejected EARLIER, before
        // the parser ever sees it.
        let mut at_cap = good_record();
        at_cap.script = vec![0x00; MAX_TRANSPARENT_SCRIPT_BYTES];
        assert_eq!(
            validate(&at_cap, &expected_addr()),
            Err(UtxoReject::ScriptUnrecognized),
            "at the cap: parsed, then rejected as not-a-wallet-output"
        );

        let mut over_cap = good_record();
        over_cap.script = vec![0x00; MAX_TRANSPARENT_SCRIPT_BYTES + 1];
        assert_eq!(
            validate(&over_cap, &expected_addr()),
            Err(UtxoReject::ScriptTooLong),
            "one over the cap: rejected before the parser"
        );
    }

    #[test]
    fn validate_rejects_an_empty_script() {
        let mut rec = good_record();
        rec.script = Vec::new();
        assert_eq!(
            validate(&rec, &expected_addr()),
            Err(UtxoReject::ScriptUnrecognized)
        );
    }

    #[test]
    fn validate_rejects_a_garbage_script() {
        let mut rec = good_record();
        rec.script = vec![0xDE, 0xAD, 0xBE, 0xEF];
        assert_eq!(
            validate(&rec, &expected_addr()),
            Err(UtxoReject::ScriptUnrecognized)
        );
    }

    // ── IZ-1b: the SCOPED multi-receiver validator (`validate_scoped`) ─────────────────────

    fn recv(tag: Option<&str>, key: u8) -> ExpectedReceiver {
        ExpectedReceiver {
            swap_id: tag.map(|s| s.to_string()),
            receiver: TransparentAddress::PublicKeyHash([key; 20]),
        }
    }

    #[test]
    fn validate_scoped_rejects_a_utxo_for_none_of_our_receivers() {
        // The M2 lying-endpoint defense generalized to the scoped set: a UTXO paying an address
        // that is NONE of `{index-0} ∪ {active destinations}` is rejected, never put.
        let set = [recv(None, 0x11), recv(Some("swap-b"), 0x22)];
        let mut rec = good_record();
        rec.script = script_bytes(&TransparentAddress::PublicKeyHash([0x99; 20])); // foreign
        assert!(matches!(
            validate_scoped(&rec, &set),
            Err(UtxoReject::AddressMismatch)
        ));
    }

    #[test]
    fn validate_scoped_attributes_a_put_to_the_matching_destination() {
        // A UTXO paying the THIRD entry is attributed to index 2 (so the funded/retire signal marks
        // the right swap), not the first match by accident.
        let set = [
            recv(None, 0x11),
            recv(Some("swap-b"), 0x22),
            recv(Some("swap-c"), 0x33),
        ];
        let mut rec = good_record();
        rec.script = script_bytes(&TransparentAddress::PublicKeyHash([0x33; 20]));
        let (output, matched) = validate_scoped(&rec, &set).expect("the 3rd receiver is ours");
        assert_eq!(
            matched,
            vec![2],
            "attributed to the matching entry, not the first"
        );
        assert_eq!(
            output.recipient_address(),
            &TransparentAddress::PublicKeyHash([0x33; 20])
        );
    }

    #[test]
    fn validate_scoped_credits_every_twin_on_one_receiver() {
        // #382 fold (money-review MED): TWIN rows on one receiver (a synthetic
        // `backfill:<index>` beside a re-armed record row after a rescan) must EACH
        // get the funded credit — under first-match-wins, whichever twin sorted
        // first starved the other, and with it the chain-observed Refunded pin.
        let set = [
            recv(None, 0x11),
            recv(Some("backfill:7"), 0x22),
            recv(Some("swap-real"), 0x22), // the SAME receiver as the synthetic twin
        ];
        let mut rec = good_record();
        rec.script = script_bytes(&TransparentAddress::PublicKeyHash([0x22; 20]));
        let (_, matched) = validate_scoped(&rec, &set).expect("the twin receiver is ours");
        assert_eq!(
            matched,
            vec![1, 2],
            "EVERY matching expected is credited, not only the first"
        );
    }

    #[test]
    fn validate_scoped_over_an_empty_set_rejects_without_panic() {
        // The idle/no-account edge: an empty expected set rejects every record (no panic, the scan
        // falls through to AddressMismatch).
        assert!(matches!(
            validate_scoped(&good_record(), &[]),
            Err(UtxoReject::AddressMismatch)
        ));
    }

    #[test]
    fn validate_scoped_field_reject_precedes_the_match_scan() {
        // A malformed record (short txid) is rejected at the shared `validate_fields` BEFORE the
        // address-set scan — the scoped path inherits every §4.6 hostile-input check.
        let set = [recv(None, 0x11)];
        let mut rec = good_record();
        rec.txid = vec![0xAB; 31];
        assert!(matches!(
            validate_scoped(&rec, &set),
            Err(UtxoReject::TxidLength)
        ));
    }

    proptest::proptest! {
        /// `validate` is TOTAL — NO byte sequence panics it (the §4.6 never-panic contract over
        /// the WHOLE input space, locking the discipline the example KATs only sample; the
        /// hostile-parser fuzz target testing-patterns §3 calls for). Arbitrary txid length,
        /// script bytes, signed index, signed value, and height. The result is intentionally
        /// discarded — the assertion is that the call RETURNS (`Ok` or any typed `UtxoReject`)
        /// rather than panicking. A panic here is an FFI crash on hostile network input.
        #[test]
        fn validate_never_panics_on_arbitrary_input(
            txid in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..40usize),
            index in proptest::prelude::any::<i32>(),
            script in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..128usize),
            value_zat in proptest::prelude::any::<i64>(),
            height in proptest::prelude::any::<u64>(),
        ) {
            let rec = TransparentUtxoRecord {
                txid,
                index,
                script,
                value_zat,
                height,
            };
            let _ = validate(&rec, &expected_addr());
        }
    }
}
