//! §3.3 tx-enhancement (memo recovery) — the DOMAIN logic.
//!
//! Compact-block scan recovers a received note (amount, recipient, status) but OMITS the memo:
//! the server sends only the ~52-byte detection prefix of the note ciphertext, and the memo
//! lives in the tail. librustzcash models the gap as `WalletRead::transaction_data_requests()`
//! — the engine fetches the FULL transaction and hands it to the audited
//! `decrypt_and_store_transaction`, which re-decrypts with the wallet's OWN keys (Rust-side,
//! never across the FFI) and recovers the memo.
//!
//! THIS module owns the enhancement DOMAIN: the §4.6 hostile-input boundary (re-parse the
//! endpoint's bytes + verify the txid matches the request, so a lying or garbled reply is
//! REJECTED, never stored), the wire-height sentinel mapping, the `TransactionFetcher` +
//! `EnhancementStore` ports, the batch-selection policy, and the bounded drain LOOP
//! ([`run_enhancement_pass`]). What lives elsewhere is only glue: the audited decryptor
//! (`decrypt_and_store_transaction`, called WHOLE), the production gRPC fetch adapter
//! (`sync::LightwalletdClient`), and the db-bound store adapter + engine wiring
//! (`wallet::WalletDbEnhancementStore` / `Wallet::enhance_transactions`, hooked into `run_pass`).
//! No key material here; no custom crypto — Rule Zero, we call the audited fn whole.

use async_trait::async_trait;
use zcash_client_backend::data_api::{TransactionDataRequest, TransactionStatus};
use zcash_primitives::transaction::Transaction;
use zcash_protocol::TxId;
use zcash_protocol::consensus::{BlockHeight, BranchId, Parameters};

use crate::error::WalletError;
use crate::net::grpc::GrpcError;

/// The raw transaction the endpoint returned for an enhancement/get-status request, flattened
/// off the proto so the wire type never leaves `net::grpc`. `data` is consensus-encoded bytes;
/// `height` is the endpoint's mined-height CLAIM — a wire sentinel (see [`map_wire_height`]).
/// BOTH are UNTRUSTED until [`parse_and_validate`] re-derives the txid from `data`.
pub(crate) struct FetchedTransaction {
    pub(crate) data: Vec<u8>,
    pub(crate) height: u64,
}

/// The §3.3 tx-enhancement (memo-recovery) fetch seam — an own port (ISP — distinct from the
/// shielded `ScanClient` and `TransparentUtxoSource` in `sync`): fetch the FULL transaction for a
/// txid the scanner flagged for enhancement, since compact blocks omit memo bytes. Lives HERE in
/// the enhancement domain (alongside [`FetchedTransaction`] and the [`parse_and_validate`]
/// boundary it feeds); `sync::LightwalletdClient` provides the production gRPC adapter, and the
/// `run_enhancement_pass` tests provide a scripted fake — both with no live gRPC for the latter.
#[async_trait]
pub(crate) trait TransactionFetcher {
    /// The full consensus-encoded tx + the endpoint's mined-height CLAIM for `txid`.
    /// `Ok(None)` ⇒ the endpoint has no such tx (→ `TxidNotRecognized`). RAW bytes from an
    /// UNTRUSTED endpoint — the caller re-parses + matches the txid ([`parse_and_validate`])
    /// before anything is decrypted/stored.
    async fn fetch_transaction(
        &mut self,
        txid: TxId,
    ) -> Result<Option<FetchedTransaction>, GrpcError>;
}

/// §5.4-clean tally of ONE enhancement pass — counts only, never a txid/memo/address.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EnhancementOutcome {
    /// Txids attempted this pass — the BOUNDED batch (`enhanced + not_found + rejected`), not
    /// the full backlog (which may exceed the per-pass cap and drain over later passes).
    pub(crate) requested: usize,
    /// Fetched + validated + handed to the audited decryptor (memo recovered if ours).
    /// `Enhancement` requests ONLY — a `GetStatus` reply is counted in [`Self::status_set`].
    pub(crate) enhanced: usize,
    /// `GetStatus` requests answered with the chain's view of the txid
    /// (`set_transaction_status`) — INC-009. Counted apart from `enhanced` because the two
    /// answer different upstream questions and have different db effects: `enhanced` writes
    /// transaction DATA, this writes the MINED HEIGHT (or its absence). A pass that only sets
    /// statuses stores nothing new to decrypt, but it does change how existing rows RENDER.
    pub(crate) status_set: usize,
    /// The endpoint had no such tx (recorded as `TxidNotRecognized`, which clears the request).
    pub(crate) not_found: usize,
    /// A reply REJECTED at the §4.6 boundary (wrong/garbled/oversized tx) OR a per-tx fetch
    /// fault — skipped, never stored; the request persists and is retried next pass. Bundling
    /// both under one count keeps the §5.4 surface to "how many didn't land," never WHY-by-txid.
    pub(crate) rejected: usize,
    /// Of the `rejected`, replies this build could not parse because they name a
    /// consensus branch it does not implement (`ironwood-nu63-support.md` §0 /
    /// §4). Counted distinctly because the cause is entirely different from a
    /// garbled reply: the endpoint is honest, the transaction is real, and WE
    /// are the stale party.
    ///
    /// **What this evidence is, and is NOT — corrected by the review (both
    /// angles, independently).** An endpoint cannot SUPPRESS it without refusing
    /// to serve transactions at all: it can lie about `consensus_branch_id`, but
    /// it cannot hand us a valid transaction for a branch we know while the
    /// chain is on one we do not. It CAN forge it. These twelve bytes are read
    /// from the raw reply AFTER [`parse_and_validate`] — the one call that binds
    /// a reply to the requested txid — has already failed, so nothing binds them
    /// to anything: a hostile endpoint answers any `GetTransaction` with a
    /// well-formed v5/v6 header naming an id this build lacks, and gets both a
    /// false "this build is stale" warn and a `UnparseableTxids` entry for a
    /// txid of its choosing.
    ///
    /// This doc used to call it *"the one piece of staleness evidence an
    /// endpoint cannot forge or suppress"*. The forge half was never true — the
    /// v5-only reader had the same hole — and requiring the whole header pair
    /// (INC-007) closes the ACCIDENTAL case (a v4 transaction's input vector
    /// landing at these offsets), not the deliberate one. Read this counter as a
    /// DIAGNOSTIC an endpoint can drive, never as proof.
    ///
    /// **INC-015 is CLOSED (T0-4, 2026-09-11).** This counter was structurally
    /// zero from the Ironwood activation until then, because
    /// [`unknown_branch_evidence`] matched a v5 header only and every
    /// post-activation transaction is v6. It now reads either pair.
    ///
    /// **Bounded (§9.1 step 7):** the request is deliberately NOT cleared — the
    /// transaction is real, so `TxidNotRecognized` would be a lie that outlives
    /// the app update — but the txid joins [`UnparseableTxids`] and is skipped
    /// for the rest of the PROCESS, so it is fetched once per launch rather than
    /// once per sync pass. What this bounds is the bandwidth, not the gap — and,
    /// per the forge paragraph above, what an endpoint can pin there is bounded
    /// only by the number of txids the wallet asks about in one process.
    pub(crate) unknown_branch: usize,
}

impl EnhancementOutcome {
    /// True when at least one request was processed (drives the §5.4 span emission — a no-op
    /// pass with an empty backlog stays silent, like the transparent poll).
    pub(crate) fn did_work(&self) -> bool {
        self.requested > 0
    }
}

/// Txids this PROCESS has already proved it cannot parse, because they name a
/// consensus branch this build does not implement
/// (`ironwood-nu63-support.md` §9.1 step 7).
///
/// **Why in memory rather than on disk.** The condition is a property of the
/// BINARY, not of the wallet: an updated build parses these fine. A process-
/// lifetime set therefore expires exactly when it should — on the next launch,
/// which is also the soonest an update can take effect — and it costs no schema,
/// no migration, and no risk of a stale row suppressing a txid forever.
///
/// **What it does and does not fix.** It stops the wallet re-downloading the
/// same unparseable transactions on every sync pass, forever, on a link that may
/// be metered or Tor. It does NOT recover the memo: that needs the crate upgrade
/// (Phase B). The request is deliberately NOT cleared — clearing it would mean
/// telling the backend `TxidNotRecognized` about a transaction that demonstrably
/// exists, which is a lie that outlives the update.
#[derive(Debug, Default)]
pub(crate) struct UnparseableTxids {
    inner: std::sync::Mutex<std::collections::HashSet<TxId>>,
}

impl UnparseableTxids {
    fn contains(&self, txid: &TxId) -> bool {
        self.inner
            .lock()
            .expect("unparseable-txid set mutex poisoned")
            .contains(txid)
    }

    fn insert(&self, txid: TxId) {
        self.inner
            .lock()
            .expect("unparseable-txid set mutex poisoned")
            .insert(txid);
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.inner
            .lock()
            .expect("unparseable-txid set mutex poisoned")
            .len()
    }
}

/// WHICH question the backend asked about a txid. The two txid-keyed
/// [`TransactionDataRequest`] variants are DIFFERENT questions with different answers, and
/// folding them cost the wallet six weeks of mis-rendered sends (INC-009): upstream
/// (`zcash_client_backend-0.23.0` `data_api.rs:1163-1168`) answers `Enhancement` with
/// `decrypt_and_store_transaction` and `GetStatus` with `WalletWrite::set_transaction_status`,
/// and only the latter ever writes a mined height. Carrying the kind alongside the txid is what
/// makes answering the wrong one a type error rather than a silent one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequestKind {
    /// "Give me this transaction's DATA" — answered with the audited decryptor. Generated for
    /// a transaction the scan has already seen (so the wallet knows its height) whose memo
    /// bytes the compact block omitted.
    Enhancement,
    /// "What does the chain think of this txid?" — answered with `set_transaction_status`.
    ///
    /// It has TWO sources, and assuming only the first is what shipped INC-013. The synthesized
    /// arm covers rows with `mined_height IS NULL` — the wallet's own broadcast sends until one
    /// is observed mined. But `queue_tx_retrieval` ALSO stamps `Status` for any txid where
    /// `raw IS NOT NULL`, and `transaction_data_requests`' first UNION arm applies **no** height
    /// filter, so this question is asked about transactions the wallet has already scanned and
    /// already holds a height for. See [`chain_status`] for the full account and for why that
    /// makes the endpoint's height claim something to check rather than record.
    Status,
}

/// Select the requests to answer THIS pass from the wallet's pending data requests, BOUNDED at
/// `max` DISTINCT txids. Keeps the txid-keyed `Enhancement`/`GetStatus` variants WITH THE KIND
/// INTACT (in upstream's returned order) and DEFERS the transparent
/// `TransactionsInvolvingAddress` variant — Recv-2b
/// `refresh_transparent_utxos` already covers transparent receive, and "satisfying" it without a
/// real check would mean calling `notify_address_checked`, falsely marking the address clear and
/// HIDING txs (money-visibility). `max` caps the burst so a freshly-restored wallet's thousands
/// of historic receives converge over a handful of passes instead of one giant download.
pub(crate) fn select_enhancement_targets(
    requests: Vec<TransactionDataRequest>,
    max: usize,
    skip: &UnparseableTxids,
) -> Vec<(TxId, RequestKind)> {
    let mut selected: Vec<(TxId, RequestKind)> = Vec::new();
    for request in requests {
        let (txid, kind) = match request {
            TransactionDataRequest::Enhancement(t) => (t, RequestKind::Enhancement),
            TransactionDataRequest::GetStatus(t) => (t, RequestKind::Status),
            // The transparent variant (and any future non-txid request) is deferred, not failed.
            _ => continue,
        };
        // §9.1 step 7: drop the txids THIS PROCESS has already proved it cannot
        // parse. Filtered BEFORE the `max` cap on purpose — otherwise a backlog of
        // unparseable transactions would fill every batch and starve the ones we
        // CAN still enhance, turning a memo gap into a total enhancement stall.
        if skip.contains(&txid) {
            continue;
        }
        // Upstream: "A `TransactionDataRequest::Enhancement` request subsumes any previously
        // existing `TransactionDataRequest::GetStatus` request." So a txid is fetched at most
        // ONCE per pass and the stronger question wins — whichever order the backend listed
        // the two rows in, which is why the scan continues past the `max` cap rather than
        // breaking out of it: a late `Enhancement` must still be able to upgrade a txid
        // already selected as `Status`.
        //
        // NOTE on ORDER, corrected in review: `transaction_data_requests` carries NO
        // `ORDER BY` and its two arms are combined with `UNION`, so the order is stable-ish but
        // ARBITRARY — emphatically not oldest-first, as an earlier version of this comment
        // claimed. Nothing here depends on age; what the order does mean is that a persistently
        // failing txid in the first `max` can starve the rest across passes, which is the
        // documented follow-up (a rejection backoff/rotation), not a property to rely on.
        if let Some(existing) = selected.iter_mut().find(|(t, _)| *t == txid) {
            if kind == RequestKind::Enhancement {
                existing.1 = RequestKind::Enhancement;
            }
        } else if selected.len() < max {
            selected.push((txid, kind));
        }
    }
    selected
}

/// The chain's view of a `GetStatus` txid, derived from a §4.6-VALIDATED reply — or `None` when
/// the endpoint's height claim cannot be believed and no status may be written this pass.
///
/// `claimed` is [`map_wire_height`] applied to the endpoint's `RawTransaction.height`, so both
/// wire sentinels (0 = mempool, `u64::MAX` = not on the main chain) have already collapsed to
/// `None` — and both are honestly `NotInMainChain`.
///
/// # Why `Mined` is the dangerous arm and `NotInMainChain` is not
///
/// The two arms of `zcash_client_sqlite-0.21.0`'s `set_transaction_status` are NOT symmetric, and
/// the asymmetry runs the wrong way: `TxidNotRecognized | NotInMainChain` writes
/// `confirmed_unmined_at_height` under `WHERE txid = :txid AND mined_height IS NULL`, so it
/// **cannot overwrite a height the wallet already established**. `Mined(height)` three lines below
/// carries **no such guard** — it overwrites `mined_height` unconditionally, re-joins `blocks`,
/// ratchets `min_observed_height` and the transparent `exposed_at_height` DOWNWARD via `MIN(…)`,
/// and then deletes the retrieval-queue row so nothing ever re-asks. `truncate_to_height` un-mines
/// only `WHERE mined_height > :height`, so a BACKDATED lie survives every reorg. Nothing short of
/// a full rescan undoes it.
///
/// Two ways that costs a user real money, both found in the review and neither hypothetical:
///
/// 1. **Overwriting a known height.** `queue_tx_retrieval` stamps `query_type = Status` for any
///    txid where `raw IS NOT NULL` — i.e. transactions the wallet ALREADY holds and has already
///    scanned — and `transaction_data_requests`' first UNION arm (the bare
///    `SELECT … FROM tx_retrieval_queue`) applies **no** `mined_height IS NULL` filter. So a status
///    question is asked about transactions whose height we know. An endpoint answering `tip-1`
///    turns a deeply-confirmed note into a one-confirmation note, below the spend minimum.
/// 2. **Locking the inputs of a send that was never relayed.** `spent_notes_clause` is consumed as
///    `AND rn.id NOT IN (…) -- the note is unspent`, and its first disjunct is
///    `stx.mined_height < :target_height`. A false `Mined` makes that permanently true, so the
///    send's input notes are excluded from `select_spendable_notes` forever. Without the lie those
///    inputs are released the moment `expiry_height` passes.
///
/// # The checks, and what each is actually worth
///
/// Ordered by strength, because they are not equally good and pretending otherwise is how the
/// previous version of this function shipped a one-sided bound:
///
/// - **`already_known` — the decisive one.** If the wallet holds ANY height for this txid, the
///   endpoint's claim is refused outright. This is the only check that closes case 1, and it is a
///   correctness rule, not a judgement call.
/// - **`expiry` — a CONSENSUS bound, the only one that does not rest on endpoint data.** ZIP-203:
///   a transaction with a nonzero expiry height cannot be mined above it. Free — the parsed
///   transaction is already in scope at the call site.
/// - **`birthday` — a lower bound.** Refuses a backdated claim below the wallet's own birthday.
///   Weak (the window above the birthday is still open) but it costs nothing and backdating is the
///   worse direction, since the `MIN(…)` ratchets never recover.
/// - **`known_tip` — the weakest, and honestly labelled.** This is `WalletRead::chain_height`,
///   which is `MAX(scan_queue.block_range_end) - 1`, fed by `update_chain_tip` from the endpoint's
///   own `latest_block_height`. So it is **endpoint-versus-its-own-earlier-claim consistency, not
///   ground truth** — an earlier version of this comment claimed "no transaction is in a block
///   that does not exist yet", which is false of a tip the same endpoint supplied. It is kept
///   because the consistency still costs a liar something, and because bounding against the
///   *scanned* tip instead would refuse every honest answer for the whole of a restore's
///   catch-up — which is INC-011's exact shape, a clamp that became the regression it was
///   preventing.
///
/// **Residual risk, accepted knowingly (maintainer decision, — Option A).** Between the last
/// height at which the wallet observed a transaction unmined and its expiry height, a hostile
/// endpoint can still land a false confirmation and lock those inputs. The alternative — never
/// writing `Mined` at all — carries zero new trust but leaves INC-009 unfixed, because a mined
/// send whose block the wallet never scanned keeps rendering "expired … still yours to spend".
///
/// **What the accepted write does to the HISTORY SURFACE, stated here beside the decision
/// (phase-2 P2-3, the Batch C security pass's row 2).** For a send the wallet holds no height
/// for, `already_known` is `None`, so an accepted `Mined(h)` sets `mined_height = h` (upstream
/// writes it unconditionally), and `history::map_row` then reads the row as
/// `Confirmed { depth }` while `history::TX_ORDER` sorts it by that height — OUT of the
/// pending-first group at the top of the Activity list, to its height's place. Nothing is
/// deleted and the row is still in the keyset walk (guarded:
/// `history::tests::real_view::a_send_the_network_never_accepted_stays_visible_and_reads_expired`,
/// clause 5), but an observer reading the top of the list sees an "Expired" row vanish. That is
/// INC-019's symptom on shipped code, from an honest endpoint correcting INC-009's stale
/// "expired" as much as from a lying one — which of the two the device saw is not
/// knowable from its log, and the registry row says so.
pub(crate) fn chain_status(
    claimed: Option<BlockHeight>,
    evidence: &StatusEvidence,
) -> Option<CheckedStatus> {
    let Some(h) = claimed else {
        // Not mined, per the endpoint. Safe to forward unconditionally: upstream's arm for this
        // status refuses to overwrite an established `mined_height`, so even a lie here cannot
        // un-mine what the wallet already knows — it can only delay, and the row keeps being
        // re-queried until the tip passes its expiry.
        return Some(CheckedStatus(TransactionStatus::NotInMainChain));
    };
    // THE check. Everything below is defence in depth; this one closes the CRITICAL.
    if evidence.already_known.is_some() {
        return None;
    }
    // No birthday ⇒ no accounts ⇒ no requests should exist. Refuse rather than assume a floor.
    let birthday = evidence.birthday?;
    let expiry = u32::from(evidence.expiry);
    if h > evidence.known_tip || h < birthday || (expiry != 0 && u32::from(h) > expiry) {
        return None;
    }
    Some(CheckedStatus(TransactionStatus::Mined(h)))
}

/// Everything the wallet knows that can contradict an endpoint's status claim, gathered once so
/// [`chain_status`] is a pure function over it and can be unit-tested without a store.
pub(crate) struct StatusEvidence {
    /// `WalletRead::chain_height` — the highest block the wallet has been TOLD exists. Endpoint-
    /// supplied; see [`chain_status`] for why it is the weakest check here.
    pub(crate) known_tip: BlockHeight,
    /// `WalletRead::get_wallet_birthday` — no transaction of ours predates it. `None` when the
    /// wallet has no accounts, which refuses the claim.
    pub(crate) birthday: Option<BlockHeight>,
    /// A mined height the wallet ALREADY holds for this txid (`WalletRead::get_tx_height`).
    /// `Some(_)` refuses the claim outright.
    ///
    /// NOTE the one lossy edge: upstream's `get_tx_height` ends in
    /// `tx_height.filter(|h| h <= &chain_tip_height)`, so a stored height ABOVE the current tip
    /// reads back as `None` here. That state needs a prior bad write or a rewind to reach, and the
    /// remaining checks still apply, so it is documented rather than worked around.
    pub(crate) already_known: Option<BlockHeight>,
    /// The transaction's own ZIP-203 expiry height, read from the §4.6-validated bytes. `0` means
    /// "never expires" and disables this check. The ONLY check here not derived from endpoint data.
    pub(crate) expiry: BlockHeight,
}

/// A [`TransactionStatus`] that has passed [`chain_status`] — the ONLY thing
/// [`EnhancementStore::set_status`] accepts.
///
/// The type exists because the arch review found the real defect in the first version of this
/// fix: the bound was a free function beside a port that took a raw `TransactionStatus`, so any
/// future call site in this crate could write an unbounded `Mined(h)` and the compiler would say
/// nothing. The field is private to this module, so it cannot be constructed elsewhere — a caller
/// that wants to set a status has to go through the checks, or edit this file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CheckedStatus(TransactionStatus);

impl CheckedStatus {
    /// The endpoint has no such transaction. Carries no endpoint-supplied data, so there is
    /// nothing to bound — and upstream's arm for it cannot overwrite an established height.
    pub(crate) fn not_recognized() -> Self {
        Self(TransactionStatus::TxidNotRecognized)
    }

    /// Unwrap for the adapter, which forwards it verbatim to `WalletWrite::set_transaction_status`.
    pub(crate) fn get(self) -> TransactionStatus {
        self.0
    }
}

/// Map the lightwalletd `RawTransaction.height` sentinel to a real mined height. `0` = mempool
/// (unmined); `u64::MAX` (and any value above the u32 block-height range) = not on the main
/// chain — both map to `None` (unmined). A value in `1..=u32::MAX` is a real mined height.
pub(crate) fn map_wire_height(height: u64) -> Option<BlockHeight> {
    (1..=u64::from(u32::MAX))
        .contains(&height)
        .then_some(BlockHeight::from_u32(height as u32))
}

/// The §4.6 hostile-input boundary for an enhancement reply: re-parse the endpoint's bytes into
/// a `Transaction` and verify its consensus txid EQUALS the requested one. Returns `Some(tx)`
/// only when both hold; `None` rejects a garbled/truncated tx OR a WRONG transaction (a lying
/// endpoint that answers a different tx — the M2 threat). The caller then skips it: nothing is
/// decrypted or stored from an unvalidated reply.
///
/// `parse_height` selects the consensus branch for LEGACY (v4) transactions; modern v5 txs
/// self-describe their branch (`Transaction::read_v5` ignores the passed branch), so this is
/// only consulted for old transactions a deep restore might surface.
/// Read the consensus branch id out of a v5 OR v6 transaction header without
/// parsing the transaction (`ironwood-nu63-support.md` §4).
///
/// Returns `Some(branch)` only when the bytes name a branch THIS BUILD does not
/// know — i.e. local evidence that the chain has moved past our
/// consensus rules. `None` for a short reply, a legacy v3/v4 transaction, a
/// mismatched version/group pair, or a branch we implement.
///
/// # INC-015 — it used to read v5 ONLY, and that made it inert (fixed, T0-4)
///
/// It matched the v5 header EXACTLY, and post-Ironwood transactions are **v6**:
/// `zcash_primitives-0.30.1` `transaction/mod.rs:204` maps
/// `BranchId::Nu6_3 => TxVersion::V6`, and `TxVersion::read` (`:82-100`)
/// accepts the pair `(V6_TX_VERSION, V6_VERSION_GROUP_ID)` =
/// `(6, 0xD884_B698)` unconditionally — not `nu7`-gated. So from Ironwood's
/// activation on 2026-07-28 until this fix, the function returned `None` for
/// every post-activation transaction: the `unknown_branch` counter never
/// incremented, the warn never fired, the [`UnparseableTxids`] suppression
/// never engaged, and §9.1 step 7's bandwidth bound was inert. INC-015 and
/// INC-007 in `evals/incidents.tsv`.
///
/// **The v6 header sits at the SAME offsets as the v5 one**, which is why one
/// read serves both: `read_v6_header_fragment` (`:933-946`) delegates to the
/// shared `read_header_fragment` (`:913-931`), so the layout is
/// `header(4) | version_group_id(4) | consensus_branch_id(4) | lock_time |
/// expiry` in both. The four constants come from `zcash_protocol::constants`
/// and are never spelled as literals here (gate 7) — a version or group id
/// re-tuned upstream moves this function with it.
///
/// **v5 stays matched on purpose.** The question this asks is "does any mined
/// transaction name a branch this build lacks", and a deep restore still meets
/// pre-Ironwood v5 transactions. Dropping v5 would narrow the evidence to the
/// newest format for no gain.
///
/// Total and allocation-free: a fixed-offset read behind a length check, on
/// bytes we already hold. A garbled reply can at worst make us log a wrong
/// branch id — it can never grant permission to anything, since this feeds
/// diagnostics only.
pub(crate) fn unknown_branch_evidence(data: &[u8]) -> Option<u32> {
    use zcash_protocol::constants::{
        V5_TX_VERSION, V5_VERSION_GROUP_ID, V6_TX_VERSION, V6_VERSION_GROUP_ID,
    };

    // Bytes 8..12 are the consensus branch id ONLY in a v5 or v6 header. The
    // fOverwintered bit alone does not establish that: v3 (`0x8000_0003`) and
    // v4 (`0x8000_0004`) set it too, and at that offset a v4 transaction
    // carries its transparent input vector — arbitrary attacker-influenced
    // bytes that would read as a "branch we do not implement" (crypto audit +
    // security review, found independently; INC-007).
    //
    // That mattered because the txid then joins [`UnparseableTxids`]: an
    // endpoint answering with any v4-shaped bytes could pick which of the
    // wallet's memos went dark for the rest of the process, and could
    // manufacture the "local staleness evidence" §4 relies on.
    // So require a FULL header — version and version-group id TOGETHER, as one
    // of the two known pairs. A v5 version under v6's group id (or the
    // reverse) is not a header upstream would accept either: `TxVersion::read`
    // matches the PAIR.
    const OVERWINTERED: u32 = 0x8000_0000;
    let header = u32::from_le_bytes(data.get(0..4)?.try_into().ok()?);
    let group = u32::from_le_bytes(data.get(4..8)?.try_into().ok()?);
    let known_pair = (header == OVERWINTERED | V5_TX_VERSION && group == V5_VERSION_GROUP_ID)
        || (header == OVERWINTERED | V6_TX_VERSION && group == V6_VERSION_GROUP_ID);
    if !known_pair {
        return None;
    }
    let raw = u32::from_le_bytes(data.get(8..12)?.try_into().ok()?);
    BranchId::try_from(raw).err().map(|_| raw)
}

pub(crate) fn parse_and_validate<P: Parameters>(
    params: &P,
    data: &[u8],
    expected: TxId,
    parse_height: BlockHeight,
) -> Option<Transaction> {
    let branch = BranchId::for_height(params, parse_height);
    // §4.6: every endpoint byte is hostile. The audited parser is robust, but a crafted tx that
    // triggers a panic deep in upstream parsing must degrade to a clean REJECTION, never unwind
    // the sync worker (`run_blocking` re-raises a panic via `resume_unwind`, so an un-caught
    // parser panic would crash the pass). `catch_unwind` turns any such panic into `None`. The
    // closure holds no lock and borrows only `data`/`branch`, so there is nothing to poison and
    // `AssertUnwindSafe` is sound. NOTE: this backstop relies on `panic = "unwind"` (the default);
    // a consumer that builds the staticlib with `panic = "abort"` would turn a parser panic into a
    // process abort (a remote DoS) — the SDK must not be built panic=abort. `Transaction::read` is
    // also flat (count-bounded Vecs, no recursive descent), so a stack-overflow abort is not a
    // realistic vector. Pinned by `parse_and_validate_never_panics_on_arbitrary_bytes`.
    let parsed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Transaction::read(data, branch).ok()
    }));
    let tx = parsed.ok().flatten()?;
    (tx.txid() == expected).then_some(tx)
}

/// The DB operations one enhancement pass needs, behind a PORT so the bounded drain loop
/// ([`run_enhancement_pass`]) is the SAME code in production and under test. The production
/// adapter (`wallet::WalletDbEnhancementStore`) wraps each op in `run_blocking` + the engine db
/// lock; a test drives the loop over a recording fake (no gRPC, no lock dance). Keeping the loop
/// store-agnostic is what makes the per-tx fault isolation, the not-found / reject tallies, and —
/// the security-critical one — the `mined_height = None` decision UNIT-TESTABLE without a device.
///
/// Methods take `&self` (not `&mut self`): the production store reaches a `Mutex<WalletDb>`
/// through an `Arc`, like every other engine read. No impl borrows the store across an `.await`,
/// so there is no lock-held-across-await hazard (rust-patterns § Async).
#[async_trait]
pub(crate) trait EnhancementStore {
    /// The wallet's pending data requests (`WalletRead::transaction_data_requests`).
    async fn pending_requests(&self) -> Result<Vec<TransactionDataRequest>, WalletError>;
    /// The KNOWN chain tip (`WalletRead::chain_height`) — the parse-branch fallback for a
    /// legacy v4 MEMPOOL reply, and one input to the status bound. Read LAZILY (only when the
    /// batch is non-empty) so an idle pass with an empty backlog skips it entirely.
    ///
    /// **Not the scanned tip**, despite what this doc said before the review. Upstream
    /// resolves it to `MAX(scan_queue.block_range_end) - 1`, which `update_chain_tip` fills from
    /// the endpoint's own `latest_block_height` — so it is what an endpoint TOLD us, not what we
    /// verified. [`chain_status`] treats it accordingly.
    async fn chain_tip(&self) -> Result<Option<BlockHeight>, WalletError>;
    /// Record the chain's view of `txid` (`WalletWrite::set_transaction_status`) — the ONLY
    /// answer that writes a mined height, and therefore the only one that can end the
    /// "was this mined?" / "height unknown" loop of INC-009.
    ///
    /// Takes a [`CheckedStatus`], not a raw `TransactionStatus`: the type is unconstructible
    /// outside this module, so the only route to a `Mined(h)` write is through
    /// [`chain_status`]'s checks. That is deliberate — see [`CheckedStatus`].
    async fn set_status(&self, txid: TxId, status: CheckedStatus) -> Result<(), WalletError>;

    /// The mined height the wallet ALREADY holds for `txid` (`WalletRead::get_tx_height`), or
    /// `None` if it holds none. Read ONLY on the [`RequestKind::Status`] path and only when the
    /// endpoint claims a height, so it costs nothing on the common paths.
    ///
    /// This is the wallet's own evidence against the endpoint's claim, and the check that uses it
    /// is the one that closes the CRITICAL — a status question IS asked about transactions
    /// whose height we already know (`queue_tx_retrieval` stamps `Status` whenever `raw IS NOT
    /// NULL`), and upstream's `Mined` arm overwrites without a `mined_height IS NULL` guard.
    async fn mined_height_of(&self, txid: TxId) -> Result<Option<BlockHeight>, WalletError>;

    /// The wallet's birthday (`WalletRead::get_wallet_birthday`) — the floor below which no
    /// transaction of ours can be mined. Read ONCE per pass, lazily, beside the chain tip.
    async fn wallet_birthday(&self) -> Result<Option<BlockHeight>, WalletError>;
    /// Hand a §4.6-validated tx to the audited `decrypt_and_store_transaction`. `mined_height`
    /// is the loop's decision (always `None` — see [`run_enhancement_pass`]); the store forwards
    /// it VERBATIM, so the exact value the security argument rests on is exercised by both the
    /// production wiring and the test.
    ///
    /// MUST be LOCAL work (DB only, no network I/O): the per-tx watchdog [`tick`](run_enhancement_pass)
    /// re-arms the stuck-sync watchdog assuming the only unbounded wait per loop iteration is the
    /// timeout-bounded fetch — a network call here would widen the silent gap between ticks and
    /// could let a real stall slip the watchdog. (The production impl runs on the blocking pool
    /// under the engine db lock; same for `set_status`.)
    async fn store_decrypted(
        &self,
        tx: Transaction,
        mined_height: Option<BlockHeight>,
    ) -> Result<(), WalletError>;
}

/// Drain ONE bounded §3.3 enhancement pass over `store`, fetching each selected tx via `client`.
/// Lifted out of `Wallet::enhance_transactions` (GAP-3) so the engine method is a thin adapter
/// and the policy below is unit-testable against a fake — no gRPC, no device.
///
/// Sequence (the spec order): select the bounded txid-keyed batch ([`select_enhancement_targets`]
/// — transparent variants deferred, capped at `max`) → for each, fetch LOCK-FREE with per-tx
/// fault ISOLATION (a fetch error is counted `rejected` and skipped, never aborting the rest of
/// the batch, so one poison txid can't starve the pass) → `Ok(None)` ⇒ set `TxidNotRecognized`
/// (`not_found`) → `Ok(Some)` ⇒ re-parse + verify the txid at the §4.6 boundary
/// ([`parse_and_validate`] — a wrong/garbled/oversized reply is `rejected`, nothing stored) →
/// answer THE QUESTION THAT WAS ASKED. A DB-write fault PROPAGATES (the caller swallows + logs
/// it — best-effort, the chain is the source of truth).
///
/// **Answering the question that was asked (INC-009).** The two txid-keyed request kinds are
/// answered differently, and the difference is the whole defect this loop was rewritten for:
/// a [`RequestKind::Enhancement`] goes to the audited decryptor, a [`RequestKind::Status`] goes
/// to `set_transaction_status` ([`chain_status`]). Answering a status request with the decryptor
/// writes no mined height, so `zcash_client_sqlite` re-synthesizes the same request on the next
/// pass, forever, while the send renders as *"expired … still yours to spend."*
///
/// **THE money decision (crypto audit fold — now in ONE place, asserted by
/// `run_enhancement_pass_isolates_faults_*` / `_stores_with_mined_height_none`):** a validated
/// tx handed to the DECRYPTOR is stored with `mined_height = None`, NEVER the endpoint's claimed
/// `ft.height`. Enhancement requests come from already-SCANNED receives, so the audited decryptor
/// resolves the height from the wallet's OWN block (`get_tx_height`); trusting the endpoint's
/// claim would let a liar overwrite a note's confirmation depth / ZIP-315 spend-eligibility for a
/// tx whose real height we already know. A STATUS request is the case where the wallet has no
/// height of its own — that is what it is asking about — so there the endpoint's claim is the
/// only available answer and is taken, BOUNDED by the known chain tip; [`chain_status`] owns that
/// bound and states why.
/// `tick` is a per-tx LIVENESS callback fired once before each fetch — the caller uses it to
/// signal the drain is advancing (the engine re-arms its stuck-sync watchdog, so a slow-but-alive
/// backlog of many unary `GetTransaction`s on a degraded link doesn't trip a false `Stalled`); the
/// loop itself is agnostic to what it means (tests just count it). It fires per ATTEMPTED tx —
/// including a rejected/not-found one — since every iteration is bounded work that advances the
/// drain. The liveness contract assumes the only unbounded wait per iteration is the
/// (timeout-bounded) fetch — see [`EnhancementStore::store_decrypted`].
pub(crate) async fn run_enhancement_pass<S, C, P>(
    store: &S,
    client: &mut C,
    params: &P,
    max: usize,
    tick: &(dyn Fn() + Sync),
    // §9.1 step 7 — the process-lifetime set of txids this build cannot parse.
    // Passed in rather than owned here so the caller controls its lifetime (the
    // wallet handle owns it; a new process starts empty, which is exactly when
    // an app update can have changed the answer).
    unparseable: &UnparseableTxids,
) -> Result<EnhancementOutcome, WalletError>
where
    S: EnhancementStore + ?Sized,
    C: TransactionFetcher + ?Sized,
    P: Parameters,
{
    let targets = select_enhancement_targets(store.pending_requests().await?, max, unparseable);
    let mut outcome = EnhancementOutcome {
        requested: targets.len(),
        ..Default::default()
    };
    // Idle fast-path: an empty backlog (the common per-pass case after the chain catches up) does
    // NO further work — and in particular skips the `chain_tip` read it would never consult. The
    // §5.4 span stays silent (`did_work() == false`), like the transparent poll.
    if targets.is_empty() {
        return Ok(outcome);
    }

    // The known chain tip, read ONCE for the batch. It has two jobs, and they are not the same
    // job — keep them apart:
    //
    //  * `fallback` is the parse-branch fallback for a legacy v4 MEMPOOL reply (wire height 0):
    //    a mined reply carries its own height and a v5 tx self-describes its branch, so this is
    //    essentially never read. `from_u32(1)` is a harmless floor for the impossible no-tip
    //    case (requests come from scanned blocks ⇒ the tip is `Some`). It is only a branch
    //    selector, never a safety variable, and the txid-match gates every store regardless.
    //  * `tip` (the `Option`, un-defaulted) is one input to the status bound — a safety
    //    variable, so it must NOT inherit the permissive `from_u32(1)` floor. See
    //    [`chain_status`], which is honest about this being the WEAKEST of its checks.
    //
    // This is a SEPARATE store read from `pending_requests` above and need NOT be consistent
    // with it — do not "tighten" the two into one held-lock read. Reading it lazily (only for a
    // non-empty batch) keeps the common idle pass off the lock entirely.
    let tip = store.chain_tip().await?;
    let fallback = tip.unwrap_or(BlockHeight::from_u32(1));
    // The birthday shares the tip's lazy read: both are per-pass constants, and neither is
    // touched by an idle pass.
    let birthday = store.wallet_birthday().await?;

    for (txid, kind) in targets {
        // Liveness: re-arm the stuck-sync watchdog BEFORE the (unary-timeout-bounded) fetch, so
        // the silent gap between ticks is ≤ one fetch — a long but progressing drain re-arms it
        // every tx instead of going dark for the whole pass and tripping a false `Stalled`.
        tick();
        // Lock-free network fetch. A per-tx fault is ISOLATED: counted `rejected` + skipped,
        // never propagated, so one poison txid cannot block the rest of this pass's batch.
        let fetched = match client.fetch_transaction(txid).await {
            Ok(f) => f,
            Err(_e) => {
                outcome.rejected += 1;
                continue;
            }
        };
        match fetched {
            None => {
                store
                    .set_status(txid, CheckedStatus::not_recognized())
                    .await?;
                outcome.not_found += 1;
            }
            Some(ft) => {
                let claimed = map_wire_height(ft.height);
                let parse_height = claimed.unwrap_or(fallback);
                let Some(tx) = parse_and_validate(params, &ft.data, txid, parse_height) else {
                    // §4.6: wrong/garbled/oversized reply — rejected, nothing decrypted or stored.
                    outcome.rejected += 1;
                    // ...but distinguish the ONE rejection cause that is about
                    // us rather than the reply. Never logs the txid (§5.4); the
                    // branch id is a public protocol constant.
                    if let Some(branch) = unknown_branch_evidence(&ft.data) {
                        outcome.unknown_branch += 1;
                        // Stop re-fetching THIS txid for the rest of the
                        // process: we have proved we cannot read it, and the
                        // answer cannot change until the binary does.
                        unparseable.insert(txid);
                        tracing::warn!(
                            target: "wallet.consensus",
                            endpoint_branch = format!("{branch:#010x}"),
                            "a mined transaction names a consensus branch this build \
                             does not implement — this build is stale"
                        );
                    }
                    continue;
                };
                match kind {
                    // mined_height = None — see the money decision in this fn's doc-comment.
                    RequestKind::Enhancement => {
                        store.store_decrypted(tx, None).await?;
                        outcome.enhanced += 1;
                    }
                    // INC-009: the question was "what does the chain think of this txid?", so
                    // the answer is a STATUS, not a decrypt. The reply is already §4.6-validated
                    // (the endpoint provably holds the transaction we asked about); what remains
                    // untrusted is its HEIGHT claim, which `chain_status` checks.
                    RequestKind::Status => {
                        // A `None` tip refuses before anything else, INCLUDING the sentinel
                        // answers: upstream's `set_transaction_status` reads
                        // `chain_tip_height(…).ok_or(ChainHeightUnknown)?` BEFORE matching on the
                        // status, so with no tip even a `NotInMainChain` write faults and
                        // `?`-aborts the whole pass — every pass, not just this txid.
                        let Some(known_tip) = tip else {
                            outcome.rejected += 1;
                            tracing::warn!(
                                target: "wallet.enhance",
                                "no known chain tip — a status answer is refused rather than \
                                 written, since upstream would fault and abort the pass"
                            );
                            continue;
                        };
                        // The wallet's OWN height for this txid — the evidence that closes the
                        // CRITICAL. Read only when the endpoint actually claims a height,
                        // so the common "mempool" answer costs no extra db read.
                        let already_known = match claimed {
                            Some(_) => store.mined_height_of(txid).await?,
                            None => None,
                        };
                        let Some(status) = chain_status(
                            claimed,
                            &StatusEvidence {
                                known_tip,
                                birthday,
                                already_known,
                                expiry: tx.expiry_height(),
                            },
                        ) else {
                            outcome.rejected += 1;
                            // §5.4: no txid, no heights, no user data — a refusal says only that
                            // one happened. The counter is the signal; the causes are in code.
                            tracing::warn!(
                                target: "wallet.enhance",
                                "an endpoint's mined-height claim contradicts what this wallet \
                                 already knows — status refused, the request stands and retries"
                            );
                            continue;
                        };
                        store.set_status(txid, status).await?;
                        outcome.status_set += 1;
                    }
                }
            }
        }
    }
    Ok(outcome)
}

/// Fuzz hook (re-exported as `zec_wallet_core::__fuzz_parse_and_validate`) — drives the §4.6
/// full-transaction parse boundary ([`parse_and_validate`]) with ARBITRARY endpoint bytes. The
/// sibling of `__fuzz_validate_utxo`; this one parses the FULL consensus transaction (a far
/// deeper decoder than the transparent validator), so coverage-guided exploration is the right
/// tool. It must NEVER panic (the `catch_unwind` backstop + principle 7): any bytes → a clean
/// accept/reject. The first 32 bytes seed the expected txid (so the fuzzer can also explore the
/// txid-match path), the rest is the tx data. The bool keeps the call from being optimized away.
#[cfg(fuzzing)]
#[doc(hidden)]
pub fn __fuzz_parse_and_validate(data: &[u8]) -> bool {
    let mut id = [0u8; 32];
    let n = data.len().min(32);
    id[..n].copy_from_slice(&data[..n]);
    let body = data.get(32..).unwrap_or(&[]);
    parse_and_validate(
        &zcash_protocol::consensus::MainNetwork,
        body,
        TxId::from_bytes(id),
        BlockHeight::from_u32(2_000_000),
    )
    .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::MAX_ENHANCEMENTS_PER_PASS;
    use zcash_primitives::transaction::{Authorized, TransactionData, TxVersion};
    use zcash_protocol::consensus::{BranchId, MainNetwork};

    #[test]
    fn select_enhancement_targets_filters_orders_bounds_and_defers_transparent() {
        use zcash_client_backend::data_api::{OutputStatusFilter, TransactionStatusFilter};
        use zcash_transparent::address::TransparentAddress;
        let t = |b: u8| TxId::from_bytes([b; 32]);
        // A transparent request is interleaved — it must be DEFERRED (dropped from the batch),
        // while the txid-keyed Enhancement/GetStatus are kept IN ORDER (oldest-first).
        let transparent = TransactionDataRequest::transactions_involving_address(
            TransparentAddress::PublicKeyHash([0u8; 20]),
            BlockHeight::from_u32(1),
            None,
            None,
            TransactionStatusFilter::Mined,
            OutputStatusFilter::Unspent,
        );
        let reqs = vec![
            TransactionDataRequest::Enhancement(t(1)),
            transparent,
            TransactionDataRequest::GetStatus(t(2)),
            TransactionDataRequest::Enhancement(t(3)),
        ];
        assert_eq!(
            select_enhancement_targets(reqs, 10, &UnparseableTxids::default()),
            vec![
                (t(1), RequestKind::Enhancement),
                (t(2), RequestKind::Status),
                (t(3), RequestKind::Enhancement),
            ],
            "txid-keyed kept in order AND WITH THE KIND INTACT (INC-009: folding GetStatus \
             into Enhancement is what left mined sends rendering as expired); the transparent \
             variant is deferred, not failed",
        );

        // Upstream: an `Enhancement` request SUBSUMES a pre-existing `GetStatus` for the same
        // txid. One fetch, and the stronger question wins — in EITHER listing order, since the
        // backend's UNION does not promise one.
        for (label, reqs) in [
            (
                "status listed first",
                vec![
                    TransactionDataRequest::GetStatus(t(9)),
                    TransactionDataRequest::Enhancement(t(9)),
                ],
            ),
            (
                "enhancement listed first",
                vec![
                    TransactionDataRequest::Enhancement(t(9)),
                    TransactionDataRequest::GetStatus(t(9)),
                ],
            ),
        ] {
            assert_eq!(
                select_enhancement_targets(reqs, 10, &UnparseableTxids::default()),
                vec![(t(9), RequestKind::Enhancement)],
                "{label}: one fetch for the txid, and Enhancement subsumes GetStatus",
            );
        }

        // The subsumption must survive the per-pass cap: a txid already selected as `Status`
        // is upgraded by a LATER `Enhancement` even though the batch is full, because the scan
        // does not stop at `max` — only the pushing does.
        assert_eq!(
            select_enhancement_targets(
                vec![
                    TransactionDataRequest::GetStatus(t(1)),
                    TransactionDataRequest::GetStatus(t(2)),
                    TransactionDataRequest::Enhancement(t(1)),
                ],
                2,
                &UnparseableTxids::default(),
            ),
            vec![
                (t(1), RequestKind::Enhancement),
                (t(2), RequestKind::Status)
            ],
            "a full batch still upgrades a txid it already holds",
        );

        // The per-pass bound caps a large restore backlog (the perf guard).
        let many: Vec<_> = (0..200u32)
            .map(|i| TransactionDataRequest::Enhancement(TxId::from_bytes([(i % 256) as u8; 32])))
            .collect();
        assert_eq!(
            select_enhancement_targets(
                many,
                MAX_ENHANCEMENTS_PER_PASS,
                &UnparseableTxids::default()
            )
            .len(),
            MAX_ENHANCEMENTS_PER_PASS,
            "the batch is capped at MAX_ENHANCEMENTS_PER_PASS; the rest persist for later passes",
        );
    }

    #[test]
    fn map_wire_height_maps_sentinels_and_real_heights() {
        assert_eq!(map_wire_height(0), None, "0 = mempool ⇒ unmined");
        assert_eq!(map_wire_height(1), Some(BlockHeight::from_u32(1)));
        assert_eq!(
            map_wire_height(2_000_000),
            Some(BlockHeight::from_u32(2_000_000))
        );
        assert_eq!(
            map_wire_height(u64::from(u32::MAX)),
            Some(BlockHeight::from_u32(u32::MAX)),
            "the top of the real block-height range maps through",
        );
        assert_eq!(
            map_wire_height(u64::from(u32::MAX) + 1),
            None,
            "above the u32 height range ⇒ unmined (anomalous)",
        );
        assert_eq!(
            map_wire_height(u64::MAX),
            None,
            "u64::MAX = non-main-fork ⇒ unmined"
        );
    }

    #[test]
    fn parse_and_validate_accepts_a_matching_tx() {
        let (bytes, txid) = tx_variant(0);
        let got = parse_and_validate(&MainNetwork, &bytes, txid, BlockHeight::from_u32(2_000_000));
        assert!(
            got.is_some(),
            "a valid tx whose txid matches the request is accepted"
        );
        assert_eq!(got.unwrap().txid(), txid);
    }

    #[test]
    fn parse_and_validate_rejects_a_txid_mismatch() {
        // §4.6 / M2: a lying endpoint returns a real, parseable tx that is NOT the one we asked
        // for. It MUST be rejected — never decrypted/stored under the requested txid (which
        // would attach a stranger's memo/data to our row).
        let (bytes, real) = tx_variant(0);
        let mut other = *real.as_ref();
        other[0] ^= 0xff;
        let wrong = TxId::from_bytes(other);
        assert_ne!(wrong, real);
        let got = parse_and_validate(
            &MainNetwork,
            &bytes,
            wrong,
            BlockHeight::from_u32(2_000_000),
        );
        assert!(got.is_none(), "parsed txid != requested ⇒ rejected");
    }

    #[test]
    fn parse_and_validate_rejects_garbage_and_truncation() {
        let (bytes, txid) = tx_variant(0);
        let h = BlockHeight::from_u32(2_000_000);
        assert!(
            parse_and_validate(&MainNetwork, &[], txid, h).is_none(),
            "empty bytes ⇒ rejected"
        );
        assert!(
            parse_and_validate(&MainNetwork, &[0xde, 0xad, 0xbe, 0xef], txid, h).is_none(),
            "garbage ⇒ rejected, never a panic",
        );
        assert!(
            parse_and_validate(&MainNetwork, &bytes[..bytes.len() / 2], txid, h).is_none(),
            "a tx truncated mid-stream ⇒ parse fails ⇒ rejected",
        );
    }

    proptest::proptest! {
        // §4.6 fuzz: ARBITRARY endpoint bytes must yield a clean accept/reject — NEVER a panic
        // that unwinds the sync worker. The `catch_unwind` backstop makes this hold even if the
        // audited parser panics on a crafted input. A valid-but-not-ours tx (txid mismatch) and
        // every malformed shape both resolve to `None`.
        #[test]
        fn parse_and_validate_never_panics_on_arbitrary_bytes(
            data in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..4096),
        ) {
            let _ = parse_and_validate(
                &MainNetwork,
                &data,
                TxId::from_bytes([0u8; 32]),
                BlockHeight::from_u32(2_000_000),
            );
        }

        /// The staleness-evidence reader is a fixed-offset read over hostile
        /// bytes — it must be total, like its neighbour.
        #[test]
        fn unknown_branch_evidence_never_panics_on_arbitrary_bytes(
            data in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..4096),
        ) {
            let _ = unknown_branch_evidence(&data);
        }
    }

    /// An unknown branch id, ASSERTED unknown by the row that uses it.
    ///
    /// The value is deliberately not a constant from anywhere: it must be one
    /// `BranchId::try_from` refuses TODAY, and the assertion below is what keeps
    /// that true at the next crate bump. Before T0-4 this fixture was
    /// `0x37a5_165b` = `BranchId::Nu6_3`, chosen when the pinned crate lacked
    /// it — and the Phase B wave made it KNOWN, so the row that named it
    /// asserted staleness for a branch this build implements and went red
    /// (INC-015's second half). A hand-picked id with an assertion cannot rot
    /// that way.
    const UNKNOWN_BRANCH: u32 = 0x0bad_0bad;

    /// `BranchId::Nu6_3` — the Ironwood branch, which this build DOES implement
    /// since the Phase B wave (`zcash_protocol-0.10.5 consensus.rs:758`). The
    /// control, and the reason the fixture above is not it.
    const NU6_3: u32 = 0x37a5_165b;

    /// A hand-built v5 header carrying `branch`, plus the lock-time and expiry
    /// the layout requires. `header(4) | version_group_id(4) |
    /// consensus_branch_id(4) | lock_time(4) | expiry(4)`.
    fn v5_header(branch: u32) -> Vec<u8> {
        header_with(
            zcash_protocol::constants::V5_TX_VERSION,
            zcash_protocol::constants::V5_VERSION_GROUP_ID,
            branch,
        )
    }

    /// The same, for v6 — the SAME offsets (`read_v6_header_fragment`
    /// delegates to `read_header_fragment`), which is the whole of T0-4.
    fn v6_header(branch: u32) -> Vec<u8> {
        header_with(
            zcash_protocol::constants::V6_TX_VERSION,
            zcash_protocol::constants::V6_VERSION_GROUP_ID,
            branch,
        )
    }

    /// One header, from the constants rather than from literals (gate 7).
    fn header_with(version: u32, group: u32, branch: u32) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&(0x8000_0000u32 | version).to_le_bytes());
        b.extend_from_slice(&group.to_le_bytes());
        b.extend_from_slice(&branch.to_le_bytes());
        b.extend_from_slice(&[0u8; 8]); // lock_time + expiry
        b
    }

    /// **§4y ST-1 — DEFECT row, RED before T0-4.** A **v6** transaction naming
    /// a branch this build lacks is staleness evidence.
    ///
    /// `ironwood-nu63-support.md` §4: a mined transaction naming a branch we do
    /// not implement is local proof that the chain has moved past
    /// our params — the one claim a lying endpoint cannot suppress except by
    /// refusing to serve transactions at all.
    ///
    /// The row asserts its own fixture is unknown (`BranchId::try_from(..)
    /// .is_err()`) so it cannot silently become a known branch at the next
    /// crate bump — which is exactly how the version of this row that named
    /// `Nu6_3` rotted. At the base (v5-only matching) the v6 clause is `None`:
    /// post-Ironwood transactions are v6, so the detector matched nothing from
    /// activation on 2026-07-28 onward.
    #[test]
    fn a_v6_transaction_naming_a_branch_this_build_lacks_is_staleness_evidence() {
        assert!(
            BranchId::try_from(UNKNOWN_BRANCH).is_err(),
            "the fixture must name a branch this build genuinely lacks, or the row proves \
             nothing; {UNKNOWN_BRANCH:#010x} parsed"
        );
        assert_eq!(
            unknown_branch_evidence(&v6_header(UNKNOWN_BRANCH)),
            Some(UNKNOWN_BRANCH),
            "a v6 transaction naming a branch this build lacks is evidence this build is stale \
             — every post-Ironwood transaction is v6, so a v5-only reader sees none of them"
        );
        assert_eq!(
            unknown_branch_evidence(&v5_header(UNKNOWN_BRANCH)),
            Some(UNKNOWN_BRANCH),
            "and v5 still counts: a deep restore meets pre-Ironwood transactions, and the \
             question is about the BRANCH, not the format"
        );
        assert_eq!(
            unknown_branch_evidence(&v6_header(NU6_3)),
            None,
            "a branch we implement is not evidence of anything — and Nu6_3 IS implemented by \
             this build, which is why it is the control here and not the fixture"
        );
        assert!(
            BranchId::try_from(NU6_3).is_ok(),
            "the control's premise: this build implements Nu6_3 (Phase B). If this fails the \
             wave was reverted and the clause above is testing nothing"
        );
        // A header with NOTHING after the branch id still reads: the lock-time
        // and expiry are not this function's business, and a reply truncated
        // just past the branch is still evidence.
        assert_eq!(
            unknown_branch_evidence(&v6_header(UNKNOWN_BRANCH)[..12]),
            Some(UNKNOWN_BRANCH)
        );
    }

    /// **§4y ST-3 — CONTROL (INC-007), green before and after T0-4.** Only a
    /// WHOLE v5 or v6 header is read as evidence.
    ///
    /// The fOverwintered bit alone does not establish a branch id at bytes
    /// 8..12: v3 and v4 set it too, and there a v4 transaction carries its
    /// TRANSPARENT INPUT VECTOR — attacker-influenced bytes that would read as
    /// "a branch we do not implement". That matters because the txid then joins
    /// [`UnparseableTxids`]: an endpoint answering with v4-shaped bytes could
    /// pick which of the wallet's memos went dark for the rest of the process,
    /// and could manufacture the "local staleness evidence" §4 rests on.
    ///
    /// T0-4 widens the reader to a second format, which is exactly when this
    /// control earns its keep: the pairs must stay PAIRS. A v5 version under
    /// v6's group id, or the reverse, is not a header upstream would accept
    /// either (`TxVersion::read` matches the pair).
    #[test]
    fn only_a_whole_v5_or_v6_header_is_read_as_evidence() {
        use zcash_protocol::constants::{
            V5_TX_VERSION, V5_VERSION_GROUP_ID, V6_TX_VERSION, V6_VERSION_GROUP_ID,
        };
        let cases: [(&str, Vec<u8>); 8] = [
            (
                "v4 with v5's group id — the INC-007 shape, fOverwintered set",
                header_with(4, V5_VERSION_GROUP_ID, UNKNOWN_BRANCH),
            ),
            (
                "v4 with its own group id",
                header_with(4, 0x892F_2085, UNKNOWN_BRANCH),
            ),
            (
                "v3 with its own group id",
                header_with(3, 0x03C4_8270, UNKNOWN_BRANCH),
            ),
            (
                "v5 version, v6 group id — the pair is what upstream matches",
                header_with(V5_TX_VERSION, V6_VERSION_GROUP_ID, UNKNOWN_BRANCH),
            ),
            (
                "v6 version, v5 group id",
                header_with(V6_TX_VERSION, V5_VERSION_GROUP_ID, UNKNOWN_BRANCH),
            ),
            (
                "v5 version, a group id belonging to nothing",
                header_with(V5_TX_VERSION, 0x0000_0000, UNKNOWN_BRANCH),
            ),
            (
                "a garbled reply that happens to sit at these offsets",
                vec![0xde, 0xad, 0xbe, 0xef, 0, 0, 0, 0, 1, 2, 3, 4],
            ),
            ("an empty reply", Vec::new()),
        ];
        for (name, bytes) in cases {
            assert_eq!(
                unknown_branch_evidence(&bytes),
                None,
                "{name}: only a whole v5 or v6 header may be read as branch evidence — \
                 otherwise an endpoint picks which memos go dark"
            );
        }
        // A reply one byte short of the branch id has nothing to read, in both
        // formats — the length check, not a panic.
        assert_eq!(
            unknown_branch_evidence(&v5_header(UNKNOWN_BRANCH)[..11]),
            None
        );
        assert_eq!(
            unknown_branch_evidence(&v6_header(UNKNOWN_BRANCH)[..11]),
            None
        );
    }

    // ── the §3.3 drain LOOP runtime (GAP-3) ──────────────────────────────────────────────────
    //
    // These drive `run_enhancement_pass` over a RECORDING store + a SCRIPTED fetcher — no gRPC,
    // no `WalletDb`, no device. That is the correct boundary: the loop is OUR code (selection,
    // per-tx fault isolation, the not-found / reject tallies, and the `mined_height = None`
    // decision); the audited `decrypt_and_store_transaction`'s own height resolution is upstream's
    // contract, tested there. The not-found path's effect on the REAL `v_transactions` view is
    // separately proven by `history::real_view::a_not_found_reply_never_unscans_a_received_note`.

    /// A canned reply for one txid. `Tx` carries consensus bytes (whose txid may or may not match
    /// the request — the §4.6 boundary decides) + the endpoint's mined-height CLAIM.
    #[derive(Clone)]
    enum Reply {
        Tx { data: Vec<u8>, height: u64 },
        NotFound,
        Error,
    }

    /// A `TransactionFetcher` with per-txid scripted replies — an unscripted txid defaults to
    /// `NotFound`. Records every call (in order) so per-tx isolation is observable.
    struct ScriptedFetcher {
        replies: Vec<(TxId, Reply)>,
        calls: Vec<TxId>,
    }

    impl ScriptedFetcher {
        fn new(replies: Vec<(TxId, Reply)>) -> Self {
            Self {
                replies,
                calls: Vec::new(),
            }
        }
    }

    #[async_trait]
    impl TransactionFetcher for ScriptedFetcher {
        async fn fetch_transaction(
            &mut self,
            txid: TxId,
        ) -> Result<Option<FetchedTransaction>, GrpcError> {
            self.calls.push(txid);
            let reply = self
                .replies
                .iter()
                .find(|(t, _)| *t == txid)
                .map(|(_, r)| r.clone())
                .unwrap_or(Reply::NotFound);
            match reply {
                Reply::Tx { data, height } => Ok(Some(FetchedTransaction { data, height })),
                Reply::NotFound => Ok(None),
                // A reachable-but-faulting endpoint (non-OK status) — the per-tx isolation case.
                // 14 = gRPC UNAVAILABLE; any non-OK code drives the same `Err` arm (the loop is
                // error-variant-agnostic), so the specific value is illustrative only.
                Reply::Error => Err(GrpcError::Status { code: 14 }),
            }
        }
    }

    /// A recording `EnhancementStore` — seeds a backlog of requests and records the
    /// `set_status` / `store_decrypted` calls (incl. the `mined_height` the loop chose and the
    /// exact `TransactionStatus` it derived) without any real DB. `std::sync::Mutex` for interior
    /// mutability behind the `&self` methods; no guard is held across an `.await` (the methods
    /// are synchronous), so the futures are `Send`.
    struct RecordingStore {
        backlog: std::sync::Mutex<Vec<TransactionDataRequest>>,
        tip: Option<BlockHeight>,
        tip_reads: std::sync::Mutex<usize>,
        statuses: std::sync::Mutex<Vec<(TxId, TransactionStatus)>>,
        stored: std::sync::Mutex<Vec<(TxId, Option<BlockHeight>)>>,
        /// If set to N, the Nth `store_decrypted` call (1-based) returns `Err(StoreCorrupt)`
        /// WITHOUT recording — simulates a DB-write fault to test the loop's `?`-propagation.
        fail_store_on_call: Option<usize>,
        /// Heights the wallet ALREADY holds, by txid — the evidence `chain_status` weighs against
        /// an endpoint's claim. Empty by default (the "we know nothing" case); a test that models
        /// an already-scanned transaction seeds it via `knowing_height`.
        known_heights: std::sync::Mutex<Vec<(TxId, BlockHeight)>>,
        /// The wallet birthday the fake reports. Defaults to height 1 — a floor low enough not to
        /// interfere with tests that are not about the lower bound.
        birthday: Option<BlockHeight>,
    }

    impl RecordingStore {
        fn with_backlog(txids: &[TxId], tip: Option<BlockHeight>) -> Self {
            Self::with_requests(
                txids
                    .iter()
                    .map(|t| TransactionDataRequest::Enhancement(*t))
                    .collect(),
                tip,
            )
        }

        /// A backlog of EXPLICIT request kinds. `with_backlog` is the all-`Enhancement`
        /// shorthand; this constructor exists because the `GetStatus` vs `Enhancement`
        /// distinction is itself under test (INC-009) and cannot be expressed as a bare
        /// txid list.
        fn with_requests(requests: Vec<TransactionDataRequest>, tip: Option<BlockHeight>) -> Self {
            Self {
                backlog: std::sync::Mutex::new(requests),
                tip,
                tip_reads: std::sync::Mutex::new(0),
                statuses: std::sync::Mutex::new(Vec::new()),
                stored: std::sync::Mutex::new(Vec::new()),
                fail_store_on_call: None,
                known_heights: std::sync::Mutex::new(Vec::new()),
                birthday: Some(BlockHeight::from_u32(1)),
            }
        }

        /// Make the `n`-th (1-based) `store_decrypted` call fault, like a corrupt DB write.
        fn failing_store_on_call(mut self, n: usize) -> Self {
            self.fail_store_on_call = Some(n);
            self
        }

        /// Model a transaction the wallet has ALREADY scanned and holds a height for — the
        /// state `queue_tx_retrieval` still raises a `GetStatus` for, and the one an endpoint's
        /// claim must never be allowed to overwrite.
        fn knowing_height(self, txid: TxId, height: u32) -> Self {
            self.known_heights
                .lock()
                .expect("known_heights mutex")
                .push((txid, BlockHeight::from_u32(height)));
            self
        }

        /// Set the wallet birthday the fake reports (the status lower bound).
        fn with_birthday(mut self, birthday: Option<BlockHeight>) -> Self {
            self.birthday = birthday;
            self
        }
    }

    #[async_trait]
    impl EnhancementStore for RecordingStore {
        async fn pending_requests(&self) -> Result<Vec<TransactionDataRequest>, WalletError> {
            Ok(std::mem::take(
                &mut *self.backlog.lock().expect("backlog mutex"),
            ))
        }
        async fn chain_tip(&self) -> Result<Option<BlockHeight>, WalletError> {
            *self.tip_reads.lock().expect("tip_reads mutex") += 1;
            Ok(self.tip)
        }
        async fn set_status(&self, txid: TxId, status: CheckedStatus) -> Result<(), WalletError> {
            // Recorded UNWRAPPED so assertions read as the exact upstream status that would
            // reach the db — the newtype constrains the CALLER, not what a test can inspect.
            self.statuses
                .lock()
                .expect("statuses mutex")
                .push((txid, status.get()));
            Ok(())
        }
        async fn mined_height_of(&self, txid: TxId) -> Result<Option<BlockHeight>, WalletError> {
            Ok(self
                .known_heights
                .lock()
                .expect("known_heights mutex")
                .iter()
                .find(|(t, _)| *t == txid)
                .map(|(_, h)| *h))
        }
        async fn wallet_birthday(&self) -> Result<Option<BlockHeight>, WalletError> {
            Ok(self.birthday)
        }
        async fn store_decrypted(
            &self,
            tx: Transaction,
            mined_height: Option<BlockHeight>,
        ) -> Result<(), WalletError> {
            let mut stored = self.stored.lock().expect("stored mutex");
            // The loop aborts on the first store fault, so no prior store was skipped ⇒ the
            // current call number is `stored.len() + 1`. Fault here BEFORE recording (a real
            // failed write persists nothing), exercising the `?`-propagation in the loop.
            if self.fail_store_on_call == Some(stored.len() + 1) {
                return Err(WalletError::StoreCorrupt);
            }
            stored.push((tx.txid(), mined_height));
            Ok(())
        }
    }

    /// A minimal real (empty v5) tx whose txid is varied by `seed` (via `lock_time`), so a test
    /// can build several DISTINCT parseable txs with no prover.
    fn tx_variant(seed: u32) -> (Vec<u8>, TxId) {
        tx_variant_expiring(seed, 0)
    }

    /// `tx_variant` with an explicit ZIP-203 `expiry_height`. `0` means "never expires" and is
    /// what every other fixture here uses — which is exactly why this exists: with only the
    /// expiry-0 builder, `chain_status`'s expiry check had NO live coverage and flipping its
    /// `>` to `>=` left the whole suite green (found by the code reviewer pass).
    fn tx_variant_expiring(seed: u32, expiry: u32) -> (Vec<u8>, TxId) {
        let data: TransactionData<Authorized> = TransactionData::from_parts(
            TxVersion::V5,
            BranchId::Nu5,
            seed,
            BlockHeight::from_u32(expiry),
            None,
            None,
            None,
            None,
        );
        let tx = data.freeze().expect("freeze v5 tx");
        let mut bytes = Vec::new();
        tx.write(&mut bytes).expect("serialize");
        (bytes, tx.txid())
    }

    /// **§4y ST-4 — the CONSUMER, and `ironwood-nu63-support.md` §9.1 step 7's
    /// retry BOUND.**
    ///
    /// A transaction this build cannot parse (it names a consensus branch we do
    /// not implement) is COUNTED, SUPPRESSED and WARNED, and is fetched ONCE per
    /// process rather than once per sync pass. The request is deliberately left
    /// standing — the transaction is real, so telling the backend
    /// `TxidNotRecognized` would be a lie — so without the suppression set the
    /// same download repeats forever on a link that may be metered or Tor.
    ///
    /// The fixture is a **v6** header now, which is what a post-Ironwood
    /// transaction is: before T0-4 this row drove the pure function through the
    /// pass with a v5 header naming `Nu6_3`, and the Phase B wave made that
    /// branch KNOWN — so the row went red asserting staleness for a branch this
    /// build implements (INC-015's second half, and the reason a fixture must
    /// assert its own premise).
    #[tokio::test]
    async fn an_unparseable_transaction_is_fetched_once_per_process_not_once_per_pass() {
        assert!(
            BranchId::try_from(UNKNOWN_BRANCH).is_err(),
            "the fixture must name a branch this build genuinely lacks"
        );
        let stale = TxId::from_bytes([9u8; 32]);
        // A v6 header naming a branch this build does not implement — the real
        // post-Ironwood shape, not a garbled reply. It parses as far as the
        // branch field and no further.
        let bytes = v6_header(UNKNOWN_BRANCH);

        let unparseable = UnparseableTxids::default();
        let reply = || {
            ScriptedFetcher::new(vec![(
                stale,
                Reply::Tx {
                    data: bytes.clone(),
                    height: 3_473_803,
                },
            )])
        };

        // Pass 1: fetched, rejected, and RECOGNISED as a staleness case.
        let store = RecordingStore::with_backlog(&[stale], Some(BlockHeight::from_u32(3_473_803)));
        let mut fetcher = reply();
        let out =
            run_enhancement_pass(&store, &mut fetcher, &MainNetwork, 50, &|| {}, &unparseable)
                .await
                .expect("pass 1");
        assert_eq!(out.rejected, 1, "it cannot be parsed");
        assert_eq!(out.unknown_branch, 1, "and we know WHY it cannot");
        assert_eq!(fetcher.calls.len(), 1, "pass 1 fetches it");
        assert_eq!(unparseable.len(), 1, "and remembers it");
        assert!(
            store.statuses.lock().expect("statuses").is_empty(),
            "the request is NOT cleared — the transaction exists, so \
             TxidNotRecognized would be a lie that outlives the app update"
        );

        // Pass 2, same process, same still-standing request: not fetched again.
        let store2 = RecordingStore::with_backlog(&[stale], Some(BlockHeight::from_u32(3_473_803)));
        let mut fetcher2 = reply();
        let out2 = run_enhancement_pass(
            &store2,
            &mut fetcher2,
            &MainNetwork,
            50,
            &|| {},
            &unparseable,
        )
        .await
        .expect("pass 2");
        assert!(
            fetcher2.calls.is_empty(),
            "pass 2 must not re-download a transaction we have proved we cannot read"
        );
        assert_eq!(out2.requested, 0, "the batch is empty, not merely rejected");

        // ANTI-VACUITY: a FRESH process (a new set — what a relaunch gives you,
        // and the soonest an app update can change the answer) fetches again.
        let store3 = RecordingStore::with_backlog(&[stale], Some(BlockHeight::from_u32(3_473_803)));
        let mut fetcher3 = reply();
        let _ = run_enhancement_pass(
            &store3,
            &mut fetcher3,
            &MainNetwork,
            50,
            &|| {},
            &UnparseableTxids::default(),
        )
        .await
        .expect("pass 3");
        assert_eq!(
            fetcher3.calls.len(),
            1,
            "the suppression is per-PROCESS, never permanent"
        );
    }

    /// **§4y ST-4, the operator's half.** The staleness case is WARNED, with the
    /// branch id on the line, and the line is §5.4-clean.
    ///
    /// A counter nobody can see is not a diagnostic. This drives the same pass
    /// as the row above under the capture layer and reads the warn back: a
    /// `wallet.consensus` event carrying `endpoint_branch` — the branch id, a
    /// public protocol constant and the one datum that tells an operator WHICH
    /// upgrade this build is behind — and no txid, height or wallet fact.
    ///
    /// Until T0-4 this could not be asserted at all: `CaptureLayer` recorded
    /// only `zec_wallet_core*` targets, so every `wallet.consensus` and
    /// `wallet.enhance` callsite was invisible to the guard AND to any test
    /// (§4v-run review row 11's general form). `is_ours` now covers both.
    #[tokio::test]
    async fn an_unknown_branch_reply_is_counted_suppressed_and_warned() {
        use crate::tracing_guard::{CaptureLayer, CapturedEvents, assert_5_4_clean};
        use tracing_subscriber::prelude::*;

        let stale = TxId::from_bytes([11u8; 32]);
        let bytes = v6_header(UNKNOWN_BRANCH);
        let store = RecordingStore::with_backlog(&[stale], Some(BlockHeight::from_u32(3_473_803)));
        let mut fetcher = ScriptedFetcher::new(vec![(
            stale,
            Reply::Tx {
                data: bytes,
                height: 3_473_803,
            },
        )]);

        crate::tracing_guard::force_wallet_callsites_enabled();
        let sink = CapturedEvents::default();
        let subscriber = tracing_subscriber::registry().with(CaptureLayer::new(sink.clone()));
        let guard = tracing::subscriber::set_default(subscriber);
        let out = run_enhancement_pass(
            &store,
            &mut fetcher,
            &MainNetwork,
            50,
            &|| {},
            &UnparseableTxids::default(),
        )
        .await
        .expect("pass");
        drop(guard);

        assert_eq!(
            out.unknown_branch, 1,
            "the pass recognised the staleness case"
        );
        let fields = sink.fields();
        assert_5_4_clean(&fields);
        let branch = fields
            .iter()
            .find(|(n, _)| n == "endpoint_branch")
            .map(|(_, v)| v.clone());
        assert_eq!(
            branch.as_deref(),
            Some(format!("{UNKNOWN_BRANCH:#010x}").as_str()),
            "the warn names the branch the endpoint's transaction claimed — a public protocol \
             constant, and the only datum that says WHICH upgrade this build is behind. \
             Captured: {fields:?}"
        );
    }

    #[tokio::test]
    async fn run_enhancement_pass_isolates_faults_and_tallies_every_branch() {
        // A 5-tx backlog, one reply of EACH shape, INTERLEAVED so a fault in the middle is seen
        // NOT to abort the rest (per-tx isolation). Every branch of the loop runs against the real
        // §4.6 boundary, not a hand-traced expectation.
        let (good_bytes, good) = tx_variant(1);
        let (other_bytes, other) = tx_variant(2); // a real tx that is NOT the one requested
        let notfound = TxId::from_bytes([3u8; 32]);
        let errored = TxId::from_bytes([4u8; 32]);
        let garbage = TxId::from_bytes([5u8; 32]);
        let mismatch = TxId::from_bytes([6u8; 32]);
        assert_ne!(
            other, mismatch,
            "the mismatch reply's real txid differs from the request"
        );

        let backlog = [good, notfound, errored, garbage, mismatch];
        let store = RecordingStore::with_backlog(&backlog, Some(BlockHeight::from_u32(2_000_000)));
        let mut fetcher = ScriptedFetcher::new(vec![
            (
                good,
                Reply::Tx {
                    data: good_bytes,
                    height: 1_900_000,
                },
            ),
            (notfound, Reply::NotFound),
            (errored, Reply::Error),
            (
                garbage,
                Reply::Tx {
                    data: vec![0xde, 0xad, 0xbe, 0xef],
                    height: 0,
                },
            ),
            // a real, parseable tx — but for the WRONG txid ⇒ §4.6 rejects it (the M2 lie).
            (
                mismatch,
                Reply::Tx {
                    data: other_bytes,
                    height: 0,
                },
            ),
        ]);

        let ticks = std::sync::Mutex::new(0usize);
        let out = run_enhancement_pass(
            &store,
            &mut fetcher,
            &MainNetwork,
            50,
            &|| {
                *ticks.lock().expect("ticks") += 1;
            },
            &UnparseableTxids::default(),
        )
        .await
        .expect("pass");

        assert_eq!(out.requested, 5);
        assert_eq!(
            *ticks.lock().expect("ticks"),
            5,
            "the watchdog-liveness tick fires once per ATTEMPTED tx (incl. faults), so a slow \
             drain re-arms the watchdog every tx instead of going silent",
        );
        assert_eq!(out.enhanced, 1, "only the matching reply decrypts + stores");
        assert_eq!(out.not_found, 1, "the empty reply ⇒ TxidNotRecognized");
        assert_eq!(
            out.rejected, 3,
            "fetch error + garbage + txid-mismatch all rejected, none stored",
        );

        // Per-tx ISOLATION: every txid was attempted, IN ORDER, despite the mid-batch faults.
        assert_eq!(
            fetcher.calls,
            backlog.to_vec(),
            "all 5 attempted in order; no early abort"
        );
        // The not-found request was cleared; only the good tx was stored.
        assert_eq!(
            *store.statuses.lock().expect("statuses"),
            vec![(notfound, TransactionStatus::TxidNotRecognized)],
            "only the empty reply produced a status write, and it is the not-recognized one",
        );
        let stored = store.stored.lock().expect("stored");
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].0, good, "the stored tx is the validated good one");
    }

    #[tokio::test]
    async fn run_enhancement_pass_stores_with_mined_height_none_ignoring_the_endpoint_claim() {
        // THE money decision (was untested): a lying endpoint claims a bogus mined height; the
        // loop MUST store with `None` so the audited decryptor resolves the real height from our
        // OWN scanned block — never the claim (which could mis-state confirmation depth /
        // ZIP-315 spend-eligibility for a tx we already scanned).
        let (bytes, txid) = tx_variant(7);
        let store = RecordingStore::with_backlog(&[txid], Some(BlockHeight::from_u32(2_000_000)));
        let mut fetcher = ScriptedFetcher::new(vec![(
            txid,
            Reply::Tx {
                data: bytes,
                height: 9_999_999,
            },
        )]);

        let out = run_enhancement_pass(
            &store,
            &mut fetcher,
            &MainNetwork,
            50,
            &|| {},
            &UnparseableTxids::default(),
        )
        .await
        .expect("pass");

        assert_eq!(out.enhanced, 1);
        assert_eq!(
            *store.stored.lock().expect("stored"),
            vec![(txid, None)],
            "stored with mined_height = None — the endpoint's height claim is IGNORED",
        );
    }

    #[tokio::test]
    async fn run_enhancement_pass_is_a_silent_no_op_on_an_empty_backlog_skipping_the_tip_read() {
        // The idle per-pass case: no txid-keyed requests ⇒ nothing fetched, the tip is NEVER read
        // (the lazy-tip battery micro-opt), and the §5.4 span stays silent.
        let store = RecordingStore::with_backlog(&[], Some(BlockHeight::from_u32(2_000_000)));
        let mut fetcher = ScriptedFetcher::new(vec![]);
        let ticks = std::sync::Mutex::new(0usize);

        let out = run_enhancement_pass(
            &store,
            &mut fetcher,
            &MainNetwork,
            50,
            &|| {
                *ticks.lock().expect("ticks") += 1;
            },
            &UnparseableTxids::default(),
        )
        .await
        .expect("pass");

        assert_eq!(out, EnhancementOutcome::default());
        assert!(!out.did_work(), "an empty batch does no work");
        assert_eq!(
            fetcher.calls.len(),
            0,
            "an empty backlog never hits the network"
        );
        assert_eq!(
            *store.tip_reads.lock().expect("tip_reads"),
            0,
            "the chain tip is not read on an idle pass",
        );
        assert_eq!(
            *ticks.lock().expect("ticks"),
            0,
            "an empty backlog never ticks — no slow work to keep the watchdog alive for",
        );
    }

    #[tokio::test]
    async fn run_enhancement_pass_attempts_at_most_max_per_pass() {
        // A restore backlog larger than the cap: only `max` are attempted this pass; the rest
        // persist (they remain in the store's backlog, untaken) and drain over later passes.
        let backlog: Vec<TxId> = (0..10u8).map(|b| TxId::from_bytes([b; 32])).collect();
        let store = RecordingStore::with_backlog(&backlog, Some(BlockHeight::from_u32(2_000_000)));
        let mut fetcher = ScriptedFetcher::new(vec![]); // every fetch defaults to NotFound
        let ticks = std::sync::Mutex::new(0usize);

        let out = run_enhancement_pass(
            &store,
            &mut fetcher,
            &MainNetwork,
            3,
            &|| {
                *ticks.lock().expect("ticks") += 1;
            },
            &UnparseableTxids::default(),
        )
        .await
        .expect("pass");

        assert_eq!(out.requested, 3, "the batch is bounded at max");
        assert_eq!(out.not_found, 3);
        assert_eq!(
            fetcher.calls.len(),
            3,
            "only max fetched; the other 7 persist for later passes",
        );
        assert_eq!(
            *ticks.lock().expect("ticks"),
            3,
            "the liveness tick is bounded by the same per-pass cap — 3 ticks, never 10",
        );
    }

    #[tokio::test]
    async fn run_enhancement_pass_aborts_the_pass_when_a_store_write_faults() {
        // The isolate-vs-propagate ASYMMETRY (the one money-loop decision previously only in
        // prose): a network fault is isolated (counted `rejected`, the batch CONTINUES), but a
        // store-WRITE fault is corruption-class — it must `?`-propagate and ABORT the pass so the
        // caller swallows + logs it and the requests persist for a retry, rather than hammering a
        // broken DB. A 5-tx all-valid backlog with the 3rd store faulting must stop the pass
        // there: txids #4/#5 are never even fetched.
        let mut backlog = Vec::new();
        let mut replies = Vec::new();
        for i in 1..=5u32 {
            let (data, txid) = tx_variant(i);
            backlog.push(txid);
            replies.push((
                txid,
                Reply::Tx {
                    data,
                    height: 1_900_000,
                },
            ));
        }
        let store = RecordingStore::with_backlog(&backlog, Some(BlockHeight::from_u32(2_000_000)))
            .failing_store_on_call(3);
        let mut fetcher = ScriptedFetcher::new(replies);
        let ticks = std::sync::Mutex::new(0usize);

        let err = run_enhancement_pass(
            &store,
            &mut fetcher,
            &MainNetwork,
            50,
            &|| {
                *ticks.lock().expect("ticks") += 1;
            },
            &UnparseableTxids::default(),
        )
        .await
        .expect_err("a store-write fault propagates");
        assert!(
            matches!(err, WalletError::StoreCorrupt),
            "the injected DB error propagates verbatim — never swallowed or remapped",
        );
        // The pass aborted at the faulting store: #4 and #5 were NOT attempted — a DB fault is
        // NOT isolated like a fetch fault, which DOES continue past a mid-batch error (shown by
        // run_enhancement_pass_isolates_faults_and_tallies_every_branch).
        assert_eq!(
            fetcher.calls.len(),
            3,
            "the pass aborts at the faulting store; later txids are not attempted",
        );
        assert_eq!(
            store.stored.lock().expect("stored").len(),
            2,
            "only the two stores before the fault landed; the faulting write persisted nothing",
        );
        assert_eq!(
            *ticks.lock().expect("ticks"),
            3,
            "an aborting pass ticks only for the work it ATTEMPTED (txs 1-3), then stops — it does \
             not leave the watchdog armed for the un-attempted tail",
        );
    }

    #[tokio::test]
    async fn run_enhancement_pass_enhances_a_mempool_reply_using_the_fallback_branch() {
        // A `GetStatus` request is generated precisely for an unmined/mempool tx, so a height-0
        // (mempool sentinel) reply that IS ours is a common shape. The loop must map 0 → None →
        // fall back to the scanned tip for the parse branch, then still parse + validate + store
        // (with mined_height = None, NEVER the sentinel). This pins the sentinel→fallback→accept
        // control flow + that a mempool reply is not mistakenly rejected + that the lazy tip read
        // DID fire (non-empty batch). NOTE: `tx_variant` builds v5 txs, which self-describe their
        // consensus branch, so the *value* of the fallback branch is not what is under test here
        // (that is fixture/device-gated) — only the control flow that reaches the store.
        let (bytes, txid) = tx_variant(11);
        let store = RecordingStore::with_backlog(&[txid], Some(BlockHeight::from_u32(2_000_000)));
        let mut fetcher = ScriptedFetcher::new(vec![(
            txid,
            Reply::Tx {
                data: bytes,
                height: 0,
            },
        )]);

        let out = run_enhancement_pass(
            &store,
            &mut fetcher,
            &MainNetwork,
            50,
            &|| {},
            &UnparseableTxids::default(),
        )
        .await
        .expect("pass");

        assert_eq!(
            out.enhanced, 1,
            "a height-0 mempool reply that is ours still enhances"
        );
        assert_eq!(
            *store.stored.lock().expect("stored"),
            vec![(txid, None)],
            "stored with None — never the height-0 sentinel",
        );
        assert_eq!(
            *store.tip_reads.lock().expect("tip_reads"),
            1,
            "the tip WAS read (non-empty batch) to seed the fallback parse branch",
        );
    }

    /// **INC-009**, the P0 this test was written FAILING to settle.
    ///
    /// Upstream models two DIFFERENT questions (`zcash_client_backend-0.23.0`
    /// `data_api.rs:1163-1168`): `Enhancement` asks for a transaction's DATA and is
    /// answered with `decrypt_and_store_transaction`; `GetStatus` asks for the chain's
    /// VIEW of a txid and MUST be answered with `set_transaction_status`. The loop folded
    /// both into one txid list (`Enhancement(t) | GetStatus(t) => Some(t)`) and answered
    /// both with `store_decrypted(tx, None)` — which writes no mined height. Meanwhile
    /// `zcash_client_sqlite-0.21.0`'s `transaction_data_requests` (`wallet.rs:4427-4436`)
    /// synthesizes a status request for every row with `mined_height IS NULL`, so the
    /// wallet asked "was this mined?", answered itself "here it is, height unknown", and
    /// re-asked forever. Two confirmed on-chain sends rendered as
    /// *"expired … cancelled … The amount is still yours to spend."*
    #[tokio::test]
    async fn run_enhancement_pass_answers_get_status_with_a_status_not_a_decrypt() {
        // A mined send below the tip, the shape the incident was reported against:
        // confirmed on chain, and rendered as expired by this defect.
        const MINED_AT: u32 = 3_400_000;
        let (bytes, txid) = tx_variant(21);
        let store = RecordingStore::with_requests(
            vec![TransactionDataRequest::GetStatus(txid)],
            Some(BlockHeight::from_u32(3_473_803)),
        );
        let mut fetcher = ScriptedFetcher::new(vec![(
            txid,
            Reply::Tx {
                data: bytes,
                height: u64::from(MINED_AT),
            },
        )]);

        let out = run_enhancement_pass(
            &store,
            &mut fetcher,
            &MainNetwork,
            50,
            &|| {},
            &UnparseableTxids::default(),
        )
        .await
        .expect("pass");

        assert!(
            store.stored.lock().expect("stored").is_empty(),
            "a GetStatus request asks the chain's VIEW of a txid — answering it with \
             decrypt_and_store_transaction writes no mined height, leaves the request \
             standing, and re-asks forever (INC-009)",
        );
        assert_eq!(
            out.enhanced, 0,
            "nothing was ENHANCED — the backend already has this transaction's data; \
             what it lacks is the height",
        );
        assert_eq!(
            *store.statuses.lock().expect("statuses"),
            vec![(
                txid,
                TransactionStatus::Mined(BlockHeight::from_u32(MINED_AT))
            )],
            "the mined height reaches the backend, which is what ends the re-ask loop and \
             flips the row out of 'expired'",
        );
        assert_eq!(out.status_set, 1);
        assert_eq!(out.rejected, 0);
    }

    /// **INC-013, the CRITICAL** — the check that actually protects the money.
    ///
    /// `queue_tx_retrieval` stamps `query_type = Status` for any txid where `raw IS NOT NULL`,
    /// and `transaction_data_requests`' first UNION arm applies no `mined_height IS NULL`
    /// filter — so the wallet DOES ask "what does the chain think of this?" about transactions
    /// it has already scanned and already has a height for. Upstream's `Mined` arm then
    /// overwrites `mined_height` with `WHERE txid = :txid` and no guard (unlike the
    /// `NotInMainChain` arm three lines above, which has one), re-joins `blocks`, ratchets
    /// `min_observed_height` down via `MIN(…)`, and deletes the retrieval row so nothing
    /// re-asks. `truncate_to_height` only un-mines `WHERE mined_height > :height`, so a
    /// backdated lie survives every reorg.
    ///
    /// Concretely: a deeply-confirmed note becomes a one-confirmation note, drops below the
    /// spend minimum, and the user cannot spend their own funds until a full rescan.
    #[tokio::test]
    async fn a_claimed_height_never_overwrites_a_height_the_wallet_already_knows() {
        const ALREADY_MINED_AT: u32 = 3_400_000;
        const TIP: u32 = 3_473_803;
        // The lie: plausible, below the tip, above the birthday, and inside the tx's expiry —
        // it passes every OTHER check in `chain_status`. Only the wallet's own evidence stops it.
        const LIE: u32 = TIP - 1;
        let (bytes, txid) = tx_variant(24);

        let store = RecordingStore::with_requests(
            vec![TransactionDataRequest::GetStatus(txid)],
            Some(BlockHeight::from_u32(TIP)),
        )
        .knowing_height(txid, ALREADY_MINED_AT);
        let mut fetcher = ScriptedFetcher::new(vec![(
            txid,
            Reply::Tx {
                data: bytes.clone(),
                height: u64::from(LIE),
            },
        )]);

        let out = run_enhancement_pass(
            &store,
            &mut fetcher,
            &MainNetwork,
            50,
            &|| {},
            &UnparseableTxids::default(),
        )
        .await
        .expect("pass");

        assert!(
            store.statuses.lock().expect("statuses").is_empty(),
            "the wallet already knows this transaction's height; an endpoint claim must never \
             overwrite it — that is a deeply-confirmed note becoming unspendable",
        );
        assert_eq!(out.status_set, 0);
        assert_eq!(out.rejected, 1, "refused, and counted — never silent");

        // ANTI-VACUITY: the SAME claim, same tip, same everything — except the wallet holds no
        // height of its own. Now it is written. So this guards the evidence, not the height.
        let store2 = RecordingStore::with_requests(
            vec![TransactionDataRequest::GetStatus(txid)],
            Some(BlockHeight::from_u32(TIP)),
        );
        let mut fetcher2 = ScriptedFetcher::new(vec![(
            txid,
            Reply::Tx {
                data: bytes,
                height: u64::from(LIE),
            },
        )]);
        let out2 = run_enhancement_pass(
            &store2,
            &mut fetcher2,
            &MainNetwork,
            50,
            &|| {},
            &UnparseableTxids::default(),
        )
        .await
        .expect("pass");
        assert_eq!(
            *store2.statuses.lock().expect("statuses"),
            vec![(txid, TransactionStatus::Mined(BlockHeight::from_u32(LIE)))],
            "with no height of its own the wallet has nothing to contradict the claim",
        );
        assert_eq!(out2.status_set, 1);
    }

    /// The lower bound. (The ceiling that rests on consensus is
    /// `a_claim_above_the_transactions_own_expiry_height_is_refused` — this test's name used to
    /// promise both and deliver only the floor, which the code reviewer pass caught.)
    ///
    /// Backdating is the WORSE direction: `min_observed_height` and the transparent
    /// `exposed_at_height` are both `MIN(…)` ratchets that never recover, and
    /// `truncate_to_height` un-mines only `WHERE mined_height > :height`, so a backdated claim
    /// survives every reorg.
    #[tokio::test]
    async fn a_backdated_claim_below_the_birthday_is_refused() {
        const TIP: u32 = 3_473_803;
        const BIRTHDAY: u32 = 3_400_000;
        // `tx_variant` builds an expiry-0 ("never expires") transaction, so the expiry check is
        // disabled for it — which is what lets the birthday case be tested in isolation.
        let (bytes, txid) = tx_variant(25);

        let run = |birthday: Option<BlockHeight>, claim: u32| {
            let bytes = bytes.clone();
            async move {
                let store = RecordingStore::with_requests(
                    vec![TransactionDataRequest::GetStatus(txid)],
                    Some(BlockHeight::from_u32(TIP)),
                )
                .with_birthday(birthday);
                let mut fetcher = ScriptedFetcher::new(vec![(
                    txid,
                    Reply::Tx {
                        data: bytes,
                        height: u64::from(claim),
                    },
                )]);
                let out = run_enhancement_pass(
                    &store,
                    &mut fetcher,
                    &MainNetwork,
                    50,
                    &|| {},
                    &UnparseableTxids::default(),
                )
                .await
                .expect("pass");
                (out, store.statuses.lock().expect("statuses").clone())
            }
        };

        // One block below the birthday: no transaction of ours can be there.
        let (out, statuses) = run(Some(BlockHeight::from_u32(BIRTHDAY)), BIRTHDAY - 1).await;
        assert!(statuses.is_empty(), "a claim below the birthday is refused");
        assert_eq!(out.rejected, 1);

        // ANTI-VACUITY: at the birthday exactly, the same shape of claim IS written.
        let (out, statuses) = run(Some(BlockHeight::from_u32(BIRTHDAY)), BIRTHDAY).await;
        assert_eq!(
            statuses,
            vec![(
                txid,
                TransactionStatus::Mined(BlockHeight::from_u32(BIRTHDAY))
            )],
            "the floor is inclusive — this is a bound, not a blanket refusal",
        );
        assert_eq!(out.status_set, 1);

        // No birthday at all ⇒ no accounts ⇒ refuse rather than assume a floor.
        let (_, statuses) = run(None, BIRTHDAY + 10).await;
        assert!(statuses.is_empty(), "an unknown birthday refuses the claim");
    }

    /// The ZIP-203 expiry ceiling — the ONE check in `chain_status` that rests on consensus
    /// rather than on something an endpoint told us, and therefore the one worth having exact.
    ///
    /// It had NO live coverage until the code reviewer pass said so: every fixture in
    /// this file was built by `tx_variant`, which hard-codes `expiry_height = 0` ("never
    /// expires"), so the check was disabled in every test — including the one whose NAME
    /// claimed to cover it. Flipping the comparison to `>=` (refusing a transaction mined in
    /// its own last valid block, which consensus permits) left the entire suite green.
    ///
    /// All three cases sit well below the tip and above the birthday, so a pass here is the
    /// expiry check acting and nothing else.
    #[tokio::test]
    async fn a_claim_above_the_transactions_own_expiry_height_is_refused() {
        const TIP: u32 = 3_473_803;
        const EXPIRY: u32 = 3_450_000;

        let run = |claim: u32| async move {
            // A distinct seed per case: the txid is derived from the bytes, and the expiry is
            // part of them, so the fixture must be rebuilt rather than shared.
            let (bytes, txid) = tx_variant_expiring(claim, EXPIRY);
            let store = RecordingStore::with_requests(
                vec![TransactionDataRequest::GetStatus(txid)],
                Some(BlockHeight::from_u32(TIP)),
            );
            let mut fetcher = ScriptedFetcher::new(vec![(
                txid,
                Reply::Tx {
                    data: bytes,
                    height: u64::from(claim),
                },
            )]);
            let out = run_enhancement_pass(
                &store,
                &mut fetcher,
                &MainNetwork,
                50,
                &|| {},
                &UnparseableTxids::default(),
            )
            .await
            .expect("pass");
            (out, store.statuses.lock().expect("statuses").clone(), txid)
        };

        // One block PAST expiry: consensus forbids it, so the claim is a provable lie.
        let (out, statuses, _) = run(EXPIRY + 1).await;
        assert!(
            statuses.is_empty(),
            "a transaction cannot be mined above its own expiry height — refused",
        );
        assert_eq!(out.rejected, 1);
        assert_eq!(out.status_set, 0);

        // ANTI-VACUITY, and the boundary that a `>=` mutant breaks: mined in its LAST valid
        // block is legitimate and must be written.
        let (out, statuses, txid) = run(EXPIRY).await;
        assert_eq!(
            statuses,
            vec![(
                txid,
                TransactionStatus::Mined(BlockHeight::from_u32(EXPIRY))
            )],
            "h == expiry is the last block the transaction is valid in — accepted",
        );
        assert_eq!(out.status_set, 1);

        // ANTI-VACUITY: comfortably inside the window is accepted too, so the refusal above is
        // the ceiling acting and not a blanket refusal of expiring transactions.
        let (out, statuses, txid) = run(EXPIRY - 100).await;
        assert_eq!(
            statuses,
            vec![(
                txid,
                TransactionStatus::Mined(BlockHeight::from_u32(EXPIRY - 100))
            )],
            "inside the expiry window is accepted",
        );
        assert_eq!(out.status_set, 1);
    }

    /// The bound on the one endpoint claim this module writes to the db as fact.
    ///
    /// `set_transaction_status(Mined(h))` applies `h` with NO sanity check, deletes the
    /// retrieval-queue row, and the value then survives until a rescan. Note what this check is
    /// and is not: `chain_height` is the tip an ENDPOINT told us, so this is consistency between
    /// two of its own answers, not proof — see `chain_status`, which ranks it the weakest of the
    /// four. The `None` case matters for a different reason: upstream reads
    /// `chain_tip_height(…).ok_or(ChainHeightUnknown)?` BEFORE matching on the status, so writing
    /// with no tip would fault and abort the whole pass rather than skip one txid.
    ///
    /// Non-vacuity is the point of the third case: the SAME height that is refused against a
    /// low tip is ACCEPTED once the tip has caught up, so this asserts a bound and
    /// not a blanket refusal (the grace-anchor clamp shipped as exactly that regression).
    #[tokio::test]
    async fn a_mined_height_above_the_known_tip_is_refused_not_written() {
        const CLAIMED: u32 = 3_500_000;
        let (bytes, txid) = tx_variant(22);
        let run = |tip: Option<BlockHeight>| {
            let bytes = bytes.clone();
            async move {
                let store = RecordingStore::with_requests(
                    vec![TransactionDataRequest::GetStatus(txid)],
                    tip,
                );
                let mut fetcher = ScriptedFetcher::new(vec![(
                    txid,
                    Reply::Tx {
                        data: bytes,
                        height: u64::from(CLAIMED),
                    },
                )]);
                let out = run_enhancement_pass(
                    &store,
                    &mut fetcher,
                    &MainNetwork,
                    50,
                    &|| {},
                    &UnparseableTxids::default(),
                )
                .await
                .expect("pass");
                let statuses = store.statuses.lock().expect("statuses").clone();
                (out, statuses)
            }
        };

        // One block short of the claim: refused, nothing written, the request stands.
        let (out, statuses) = run(Some(BlockHeight::from_u32(CLAIMED - 1))).await;
        assert!(
            statuses.is_empty(),
            "a claim above the known tip is impossible — never written as a confirmation",
        );
        assert_eq!(out.status_set, 0);
        assert_eq!(out.rejected, 1, "refused, and counted as such — not silent");

        // No known tip at all: nothing to bound against ⇒ refuse rather than assume.
        let (_, statuses) = run(None).await;
        assert!(statuses.is_empty(), "an unknown tip refuses the claim");

        // ANTI-VACUITY: the same claim at a tip that has caught up is ACCEPTED. The bound
        // delays an honest race by at most a pass; it does not refuse mined heights per se.
        let (out, statuses) = run(Some(BlockHeight::from_u32(CLAIMED))).await;
        assert_eq!(
            statuses,
            vec![(
                txid,
                TransactionStatus::Mined(BlockHeight::from_u32(CLAIMED))
            )],
            "at h == tip the claim is possible, so it is written",
        );
        assert_eq!(out.status_set, 1);
        assert_eq!(out.rejected, 0);
    }

    /// The mempool / not-on-main-chain sentinels are a STATUS too, not silence.
    ///
    /// A wallet's own broadcast send sits in the mempool until it is mined, and the status
    /// request is regenerated each pass until the tip passes its expiry height. Answering
    /// `NotInMainChain` is what lets upstream record `confirmed_unmined_at_height` and
    /// eventually render the send as expired — HONESTLY, rather than by the request simply
    /// never being answered.
    #[tokio::test]
    async fn a_mempool_reply_to_a_status_request_is_answered_not_in_main_chain() {
        for (label, wire) in [("mempool", 0u64), ("not on the main chain", u64::MAX)] {
            let (bytes, txid) = tx_variant(23);
            let store = RecordingStore::with_requests(
                vec![TransactionDataRequest::GetStatus(txid)],
                Some(BlockHeight::from_u32(3_473_803)),
            );
            let mut fetcher = ScriptedFetcher::new(vec![(
                txid,
                Reply::Tx {
                    data: bytes,
                    height: wire,
                },
            )]);

            let out = run_enhancement_pass(
                &store,
                &mut fetcher,
                &MainNetwork,
                50,
                &|| {},
                &UnparseableTxids::default(),
            )
            .await
            .expect("pass");

            assert_eq!(
                *store.statuses.lock().expect("statuses"),
                vec![(txid, TransactionStatus::NotInMainChain)],
                "{label}: the sentinel is an ANSWER — 'we asked, it is not mined'",
            );
            assert_eq!(out.status_set, 1);
            assert!(
                store.stored.lock().expect("stored").is_empty(),
                "{label}: still never the decryptor",
            );
        }
    }
}
