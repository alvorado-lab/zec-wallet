//! Typed error surface as Dart sees it (spec §6.1). Dart catches BY TYPE
//! (`on WalletApiError catch`), branches on the sealed `kind`, and renders
//! support codes from `code` — never by matching message text (the classic
//! FFI bug this design forbids).
//!
//! `code` is captured from the CORE's stable `code()` table at conversion
//! time — there is no second code table in this crate, and even a
//! future-unknown variant still carries its real `RW-*` code.

/// A wallet operation failed. `message` is the core's STATIC, payload-free
/// display line (amounts/addresses/txids never appear in it — safe to log);
/// the typed payload for rendering lives in `kind`.
pub struct WalletApiError {
    /// Stable, greppable, append-only code (`RW-SEED-001`, `RW-STORE-005`,
    /// …) — the support/log identifier.
    pub code: String,
    /// Static description; safe to log, NOT for end users (render from
    /// `kind` and localize host-side).
    pub message: String,
    pub kind: WalletErrorKind,
}

/// The typed error taxonomy. SDK upgrades may ADD kinds: keep a default arm
/// (`unknown` carries the real `code` via [WalletApiError.code]).
pub enum WalletErrorKind {
    /// Seed bytes outside the supported 32..=252 length window.
    InvalidSeedLength { len: u64 },
    /// BIP39 checksum/word failure. Index of the first bad word when known —
    /// NEVER the word itself.
    InvalidMnemonic { word_index: Option<u32> },
    /// `revealMnemonic` on a wallet created from raw seed bytes — that
    /// wallet structurally has no phrase; recovery is the host's own story.
    NoMnemonic,
    /// The operation needs the seed re-supplied (persistence is `none`).
    SeedRequired,
    /// Supplied seed does not match the existing wallet at this `dbDir`.
    SeedMismatch,
    /// Upstream key derivation failed (unreachable for a bounds-valid seed).
    KeyDerivation,
    /// A spend-class operation (send/shield/swap/reveal/backup) on a
    /// WATCH-ONLY wallet (#397 §3.7 D3). STRUCTURAL, not retryable — no seed
    /// will ever arrive; render an explanation ("this wallet holds no
    /// spending keys"), never a retry affordance.
    WatchOnly,
    /// A supplied viewing key failed to decode for this wallet's network
    /// (garbage, truncated, or a wrong-network `uview…`). The value is never
    /// echoed.
    InvalidViewingKey,
    /// Endpoint URL rejected; `reason` says which rule (https-only,
    /// userinfo, path, …).
    InvalidEndpoint { reason: String },
    /// Restore birthday above the chain tip — a too-high birthday would
    /// silently never find the user's notes.
    BirthdayInFuture,
    /// Wallet data directory rejected (`reason`: empty / not absolute).
    /// `dbDir` MUST be an absolute platform path — a relative one would
    /// resolve against the process working directory and strand the wallet
    /// (worst on desktop, where the cwd changes between launches).
    /// NON-RETRYABLE: a host configuration bug, not a user or environment
    /// condition — fix the path the host passes.
    // NB inserted (not appended) to keep the RW-CFG config family grouped —
    // shifts FRB positional discriminants, which is safe: they are a
    // binding-internal wire regenerated in lockstep for both halves (see the
    // QueuedSend* note below).
    InvalidDbDir { reason: String },
    /// An endpoint auth header or value rejected (ADR-0568) — a host's
    /// `endpointAuth*` / `SyncServer.auth*`, or a user's `SyncServerKey`: a
    /// half pair, a header that is not a valid name, one the transport owns or
    /// `grpc-…`, over 64 bytes, a value that is empty, over 4096 bytes, not
    /// printable ASCII or padded with whitespace, or any key for an `http://`
    /// server. `reason` names the rule; the value is NEVER echoed. Its own
    /// kind so a picker tells a bad key from a bad URL without reading text.
    // Inserted beside the RW-CFG family (the `InvalidDbDir` note above).
    InvalidEndpointAuth { reason: String },
    /// `broadcastJitter` is `uniform(maxMs)` above the ceiling (`ceilingMs`,
    /// 30 000): a longer wait before a broadcast could outlast a transaction's
    /// validity. NON-RETRYABLE: a host configuration bug, refused at
    /// create/open rather than clamped. RW-CFG-005.
    BroadcastJitterTooLong { max_ms: u64, ceiling_ms: u64 },
    /// Amount negative or beyond max supply. Payload-free on purpose:
    /// amounts are never-log material.
    AmountOutOfRange,
    /// Memo exceeds its ZIP-302 bound.
    MemoTooLong { len: u64, max: u64 },
    /// ZIP-302 reserved-range memos are receivable but never sendable.
    ReservedMemoNotSendable,
    /// Memos require a shielded recipient (the protocol cannot carry one to
    /// a transparent address) — rejected at construction, honestly.
    MemoRequiresShieldedRecipient,
    /// Address failed to parse for the configured network. The value is
    /// never echoed back.
    AddressInvalid,
    /// Memo bytes failed ZIP-302 classification (a text-lead memo whose
    /// payload is not valid UTF-8). Payload-free: memo bytes are content.
    MemoInvalid,
    /// A payment leg carried BOTH a text memo and machine bytes — one memo per
    /// payment. **This is a CALLER bug, not a user-fixable condition:** neither
    /// memo is individually invalid, so do NOT render "that memo can't be sent"
    /// at the user. Fix the code that built the request.
    MemoConflict,
    /// Zero-valued output to a transparent-only recipient — disallowed by
    /// consensus; rejected at construction.
    ZeroValuedTransparentOutput,
    /// ZIP-321 payment URI rejected: oversized, structurally malformed, or
    /// requesting nothing. Payload-free on purpose — URIs carry addresses
    /// and amounts.
    PaymentUriInvalid,
    /// A txid string was not 64 hex characters. Payload-free: txids are
    /// never-log material, so the offending value is never echoed back.
    /// Pass back the `txidHex` a history row handed you, verbatim.
    TxidInvalid,
    /// FR-27 — the machine-memo READ scope cannot be used. Either no prefix
    /// is registered on the config (the verb is opt-in and CLOSED by
    /// default — an empty answer would read as "this transaction carries
    /// nothing"), or a registered prefix is empty / over-long / one too
    /// many. `reason` is an SDK-authored static string; it never echoes
    /// host input.
    MachineMemoScopeInvalid { reason: String },
    /// Keychain locked / transiently unreachable, OR the key store did not
    /// answer within the SDK's bound (FR-47; then `code` is `RW-KEY-008`).
    /// Retryable. A wedged key store clears only when its stuck call returns,
    /// which may take a restart. Distinct from missing.
    KeystoreUnavailable,
    /// Keychain item genuinely absent or permanently invalidated while the
    /// wallet DB exists — surfaced, NEVER silently treated as a fresh
    /// wallet. `permanentlyInvalidated` = a biometric/PIN reset destroyed
    /// the key; recovery is the mnemonic.
    KeystoreInconsistent { permanently_invalidated: bool },
    /// Sealed seed carries an unknown version byte — likely an app
    /// downgrade reading a newer seal.
    SealVersionUnsupported { found: u8 },
    /// Sealed seed failed validation/decryption. Truncation, tamper and
    /// wrong-key collapse to this ONE kind on purpose (no oracle).
    SealInvalid,
    /// No platform key vault exists at all (no Keystore tier / no keyring), OR
    /// the app's link bound the wallet to a SQLite without SQLCipher, where the
    /// database key would do nothing (FR-5 D-24 — a BUILD defect: check the
    /// host's linkage, e.g. a static link that also pulls the system sqlite;
    /// retrying cannot help). FAIL-CLOSED: the wallet will not custody a seed
    /// without a vault nor store its database unencrypted. Permanent for this
    /// device/build — distinct from the retryable `keystoreUnavailable`.
    VaultAbsent,
    /// The wrap artifact failed validation or authenticated unwrap against
    /// THIS wallet's sealed seed (substitution or corruption — collapsed,
    /// but attributed to the wrap layer, never reported as `sealInvalid`).
    WrapArtifactInvalid,
    /// Wrap artifact carries an unknown version byte — likely an app
    /// downgrade reading a newer artifact.
    WrapVersionUnsupported { found: u8 },
    /// `open()` with no wallet at `dbDir`.
    NotFound,
    /// The wallet DB belongs to a different network than the config.
    NetworkMismatch,
    /// Provisioning was interrupted; reopen to resume the repair.
    ProvisioningIncomplete,
    /// `create`/`restore` against a location that ALREADY holds a completed
    /// wallet — the typed non-clobber (§6.3). Open the existing wallet instead.
    WalletAlreadyExists,
    /// Storage corruption — fail-closed, never auto-wiped.
    StoreCorrupt,
    /// Disk full — "free space and retry"; NOT corruption.
    DiskFull,
    /// The wallet store lost a write race to a concurrent engine write
    /// (typically a scan-batch commit mid-sync) past its internal wait AND the
    /// SDK's own bounded retry — TRANSIENT and RETRYABLE, never corruption:
    /// nothing was written, the wallet is unchanged, the same action succeeds
    /// once the store frees up. Render the `walletBusy` copy family ("busy —
    /// try again in a moment"), NEVER the corruption remedy.
    StoreBusy,
    /// Another instance (this process or another) holds the wallet open —
    /// rejected IS the contract.
    WalletAlreadyOpen,
    /// Transient lifecycle phase; retry when it completes.
    WalletBusy { phase: LifecyclePhase },
    /// Call on a handle in a terminal/wrong lifecycle state.
    InvalidState { phase: LifecyclePhase },
    /// `wipe()` refused while an instance holds the wallet open.
    WalletOpen,
    /// [SyncServerChoice.predefined] named an id `WalletConfig.syncServers`
    /// does not offer — a host bug, never a user state (the picker renders
    /// only offered entries).
    SyncServerNotOffered,
    /// The picker's reachability probe could not dial the server, or it did
    /// not answer within the probe budget (15 s) — the transport class,
    /// distinct from `SyncStatus.stalled`. A gated server refusing the key
    /// lands here too. The switch that carried the probe changed NOTHING:
    /// the handle, the loop and the remembered choice are as they were.
    SyncServerUnreachable,
    /// `wipe()` would destroy `count` non-terminal swap records — surfaced
    /// BEFORE proceeding (third-party wallets stay HD-recoverable from the
    /// phrase).
    WipeWithPendingSwap { count: u32 },
    /// `rescanFrom` refused while an OUTBOX send is IN FLIGHT (a queued or
    /// swap-deposit send that has broadcast-committed but not yet settled) —
    /// rebuilding the data DB under it could re-sign the
    /// same payment over different notes (2× pay). Retryable within hours
    /// WHILE SYNC RUNS: the in-flight send resolves by mining (+ ~2 h burial)
    /// or by tx expiry, both observed only by completed sync passes — so the
    /// honest host cue is "a send is still settling; try again in a couple of
    /// hours (keep the app online)", never a moments-scale retry. Like
    /// every rescan fault the handle is consumed (seals + DBs intact) — the
    /// host recovers by re-opening, exactly the existing recover-by-reopen
    /// path. (An INTERACTIVE send's broadcast-but-unmined tx keeps no outbox
    /// row and is NOT fenced — the §4.4 documented residual; a corrupt outbox
    /// row surfaces its own [WalletErrorKind.storeCorrupt] here instead.)
    RescanWithInFlightSend,
    /// #390 — the user-triggered "Check older swap addresses" deep scan was REFUSED,
    /// never run as a silent no-op. `reason` says why (see [SwapAddressCheckRefusal]):
    /// swap is turned off, or a prior check is still outstanding (at most one per
    /// settlement window). The op moved nothing and widened nothing — render the reason
    /// honestly and let the user retry later; NEVER a corruption/dead-end remedy.
    SwapAddressCheckRefused { reason: SwapAddressCheckRefusal },
    /// A sync/provisioning step could not reach the endpoint (§3.2f). `stall`
    /// is the host-renderable next step: `torUnavailable` (fail-closed under
    /// `TorPolicy.required`) vs `endpointUnreachable`.
    Sync {
        stall: crate::api::state::StallReason,
    },
    /// The Zcash network runs consensus rules this build does not implement —
    /// or this endpoint is not on the chain we think it is. The SDK cannot tell
    /// those apart from inside, so there is one kind for both
    /// (`ironwood-nu63-support.md` §2).
    ///
    /// **Signing is refused; reading is not.** Balance, history, receive, seed
    /// backup and viewing-key export all keep working — render "this version
    /// can't send until it's updated, your funds are safe and you can still
    /// receive", never a dead end and never a claim that money is at risk.
    ///
    /// The two branch ids are public protocol constants for a diagnostics or
    /// support surface, not the main path; `endpointBranchId` is null when the
    /// server would not say which network it is on.
    NetworkUpgradeUnsupported {
        expected_branch_id: u32,
        endpoint_branch_id: Option<u32>,
        judged_at_height: u32,
    },
    /// This server will not say which network it is on, and the grace the
    /// wallet extends such a server has run out — `by` says how: a day of
    /// blocks, a day on the device clock, or it never began. NOT a network
    /// upgrade and NOT [WalletErrorKind.networkUpgradeUnsupported]'s copy:
    /// nothing is known to have changed, an app update fixes nothing, and the
    /// next step is SWITCH SERVERS (plus "check the device's date and time"
    /// for [GraceExpiry.clock]). `blocksSinceLastCurrent` is the count the
    /// blocks copy names ("…for N blocks"); null for `neverConfirmed`.
    ///
    /// **Signing is refused; reading is not.** Composing and queueing keep
    /// working — a queued send drains on the first pass against a server that
    /// reports its network. Render "your funds are safe and you can still
    /// receive", never a dead end.
    ConsensusGraceExpired {
        by: crate::api::state::GraceExpiry,
        blocks_since_last_current: Option<u32>,
    },
    /// This wallet has not completed a sync pass yet, so it has never checked
    /// which network the server is on — signing is refused until it has (the
    /// first completed pass resolves it). Render the "not synced yet — wait
    /// for sync, or queue this to send later" story, NEVER an update prompt:
    /// nothing is known to be wrong, and composing and queueing keep working.
    ConsensusNotEvaluated,
    /// `syncFor` refused: a sync is already running on this handle — the
    /// background loop `startSync` spawned, or another `syncFor`. Nothing ran.
    /// Retryable: call `stopSync` first, or wait for the other `syncFor` to
    /// return. A host bug more often than a user state; never render it as a
    /// network fault.
    SyncRunning,
    /// Not enough spendable value to fund the send (§3.2h). `available`/`required`
    /// are the audited note-selector's figures; `pendingIncoming` is what is still
    /// confirming — the host renders "you have X spendable, Y arriving, you need Z".
    /// All zatoshis (Dart `int`); never logged (§5.4) — the static `message` is.
    InsufficientFunds {
        available_zat: i64,
        required_zat: i64,
        pending_incoming_zat: i64,
    },
    /// A send leg carried no amount (the ZIP-321 donation form) — the host fills the
    /// amount in between parse and confirm.
    SendAmountRequired,
    /// The one-shot proposal token was already consumed (a double-tap on send) — the
    /// notes are never broadcast twice.
    ProposalAlreadyUsed,
    /// The proposal's chain anchor went stale (its TTL elapsed, or the wallet was not
    /// synced far enough) — re-`propose` to show a fresh fee.
    ProposalStale,
    /// Propose could not prepare a transaction for a reason with no finer mapping
    /// that is deterministic on the input (retrying unchanged re-fails) — the host
    /// renders "couldn't prepare this payment; check the details and try again".
    /// The retryable class is [WalletErrorKind.proposeTransient].
    ProposeFailed,
    /// Propose could not prepare a transaction because of a condition the wallet's
    /// own state will clear without the user changing anything — a note whose
    /// witness the scan has not completed, an anchor not yet recorded, an input a
    /// concurrent proposal holds (INC-018 (b)). The host renders "couldn't prepare
    /// this payment just now; try again in a moment" — never "check the details".
    ProposeTransient,
    /// Create+sign could not complete the transaction — a proving/build fault, or a
    /// spending key the account does not own (a loud catch, never a silent wrong-key
    /// sign). The host renders "couldn't complete this payment". A stale/pruned anchor
    /// folds to [WalletErrorKind.proposalStale] (re-`propose`) instead.
    SignFailed,
    // NB the `QueuedSend*` + `TexSendLimitReached` arms are inserted here (not appended) to keep the
    // send/queue family grouped — this shifts the FRB positional discriminants of `Io`/`Unknown`. That
    // is safe: the FRB SSE discriminants are a binding-internal wire, regenerated in lockstep
    // for both the Rust and Dart halves on every `flutter_rust_bridge_codegen` run and NEVER
    // persisted or matched raw (Dart consumers branch on the freezed type, not the integer).
    /// A durably-queued send (offline-first) could no longer be funded when the
    /// resubmission machinery tried to re-propose it — the funds it would have spent have
    /// moved. The host drops/redoes it rather than retrying.
    QueuedSendStale,
    /// The durable queued-send store is at its cap — let the pending queue drain (sync) before
    /// queueing more. A defensive bound, never a silent drop of a real send.
    QueuedSendsFull,
    /// Too many one-time addresses are in use — the ZIP-320 (TEX) gap-limit ceiling. An
    /// availability bound that moved NO money, and DUAL-NATURED (#315): slots held by transfers
    /// that are still confirming free on their own as they confirm, but slots consumed by sends
    /// that never confirmed do NOT clear by waiting — so the copy must promise neither "just
    /// wait" nor doom ("some may free up as transfers confirm, but this may not clear on its
    /// own; your funds are safe"). The host MUST render this as an ORANGE form-fault, NEVER the
    /// RED "send failed" dead-end (a ceiling spent nothing, so a red failure risks a double-pay
    /// or panic), and NEVER auto-retry it as a fresh send (retries of a one-time-address send
    /// are capped per intent — see [ParkedSend.paused]); any "recover" affordance scopes to
    /// OTHER prior transfers' recoverable funds, never implying THIS send stranded. Reachable now
    /// that TEX two-step signing runs in production (gate-removal, 2e-2b-v-5): a TEX send that
    /// exceeds the one-time-address gap-limit window surfaces this typed ceiling (§3.2i-2
    /// 2e-2b-iii/v-5/vi).
    TexSendLimitReached,
    /// Underlying IO failure not better classified.
    Io,
    /// Forward-compatibility arm: a kind this binding does not know. The
    /// real stable code still rides [WalletApiError.code].
    Unknown,
}

/// Lifecycle phase, as carried by [WalletErrorKind.walletBusy] /
/// [WalletErrorKind.invalidState] so a host can render "what is the wallet
/// doing" instead of a bare failure.
pub enum LifecyclePhase {
    /// create/restore in progress.
    Provisioning,
    /// Resuming from an interrupted provisioning.
    Repairing,
    Open,
    /// `rescanFrom` is rebuilding the data DB at an earlier birthday (ADR-0534); a
    /// concurrent call gets a retryable [WalletErrorKind.walletBusy] carrying this
    /// phase, never a torn read.
    Rescanning,
    /// `switchSyncServer` is swapping the session onto another server; a
    /// concurrent call gets a retryable [WalletErrorKind.walletBusy] carrying
    /// this phase, never a torn read. Rescanning's sibling (no data rebuild).
    SwitchingServer,
    /// `close()` in progress; in-flight calls resolve with this phase.
    Closing,
    /// Terminal: the store was crypto-shredded.
    Wiped,
    /// Forward-compatibility arm (see [WalletErrorKind.unknown]).
    Unknown,
}

/// #390 — why a "Check older swap addresses" deep scan was refused (the payload of
/// [WalletErrorKind.swapAddressCheckRefused]). Branch on it for the honest host copy.
pub enum SwapAddressCheckRefusal {
    /// Swap is turned OFF at this instance (a host kill switch reached the SDK) — the
    /// check depends on the swap polling a kill stops, so it would register nothing.
    /// Render "swap is unavailable right now"; not an error to retry once swap is back.
    SwapDisabled,
    /// A prior check is still outstanding — its range is still registering, OR its watch
    /// window has not yet closed (at most ONE check per settlement window). Render "still
    /// checking the last range — try again later"; NOTHING was widened this call.
    CheckOutstanding,
    /// Forward-compatibility arm (see [WalletErrorKind.unknown]).
    Unknown,
}

/// A swap operation failed. Same contract as [WalletApiError]: catch by
/// type, branch on `kind`, log `code`.
pub struct SwapApiError {
    /// Stable, append-only `RW-SWAP-*` code.
    pub code: String,
    /// Static, payload-free description (safe to log).
    pub message: String,
    pub kind: SwapErrorKind,
}

/// Swap error taxonomy. SDK upgrades may ADD kinds — keep a default arm.
pub enum SwapErrorKind {
    /// Requested tolerance above the hard SDK ceiling — a wider window is a
    /// drain surface, not a preference.
    SlippageToleranceTooHigh { requested_bps: u16, max_bps: u16 },
    /// Quote side outside [user request ± tolerance] — rejected BEFORE
    /// anything is signed; a deposit is never anchored to provider numbers
    /// alone.
    QuoteOutOfBounds { side: QuoteBoundSide },
    /// The quote is no longer executable — its deadline lapsed, OR (since
    /// #367) its single-use claim is gone (already executed, or swept as
    /// never-issued/lapsed). No signing past it, ever. Re-quote.
    QuoteExpired,
    /// Destination rejected (see `reason`).
    DestinationInvalid { reason: DestinationInvalidReason },
    /// Request shape inconsistent (e.g. exact side names the wrong asset
    /// family); `reason` is a static description.
    RequestInvalid { reason: String },
    /// Provider unreachable / 5xx — retryable; the wallet is unaffected.
    ProviderUnavailable,
    /// Provider response violated the protocol contract — typed, never
    /// trusted further.
    ProviderProtocol { reason: ProviderProtocolReason },
    /// Swap is turned OFF at this instance (the host's regulatory kill switch
    /// reached the SDK). Render "swap unavailable" and hide the swap surface;
    /// it is not an error to retry. New quotes/executes are refused; any
    /// in-flight swap settles or refunds provider-side regardless.
    SwapDisabled,
    /// OUR side could not queue the §4.4 deposit leg after the provider registered
    /// the swap intent (the wallet was tearing down / wiped, or its store errored).
    /// Honest-degradation: the swap does NOT proceed — no ZEC left the pool — so
    /// re-quote; the provider refunds an un-deposited quote after its deadline.
    /// Distinct from [SwapErrorKind.providerUnavailable] (that is the PROVIDER's side).
    DepositSendFailed,
    /// OUR side could not produce a fresh transparent refund address for an
    /// OutOfZec quote (the QUOTE-time counterpart of [SwapErrorKind.depositSendFailed]):
    /// the wallet is tearing down / wiped, its index store errored, or it
    /// structurally cannot serve OutOfZec refunds (no seed at rest, or an exotic
    /// seed length). Honest-degradation: NO quote was returned — nothing left the
    /// pool. Render "swap unavailable" and let the user retry; a structurally
    /// incapable wallet stays so until reconstructed.
    RefundAddressUnavailable,
    /// IntoZec quote (the destination-mint counterpart of [SwapErrorKind.refundAddressUnavailable]):
    /// the wallet could not mint the FRESH per-swap ZEC receive address 1Click is handed as the
    /// recipient (§3.3b D1 / ADR-0530) — the engine DB was locked, the sealed store errored, or no
    /// account is provisioned yet. Honest-degradation: NO quote was returned — nothing is in flight.
    /// Render "swap unavailable" and let the user retry.
    DestinationAddressUnavailable,
    /// OUR side could not maintain its durable in-flight issued-quote state (the
    /// store that makes a crash-then-requote double-deposit observable to recovery):
    /// the wallet was tearing down / wiped or its sealed store errored. Fail-closed —
    /// a quote is not issued, or an execute does not register intent, so nothing left
    /// the pool; re-quote. Render "swap unavailable" and let the user retry.
    SwapStateUnavailable,
    /// OUR side's durable swap store was momentarily BUSY (#367) at a site where
    /// NOTHING was consumed — the quote persist or the execute claim. RETRYABLE:
    /// a plain retry of the SAME action genuinely succeeds once the contending
    /// writer finishes. Render "the wallet is busy for a moment — try again";
    /// for an execute specifically the quote is UNCONSUMED and still valid, so
    /// do NOT force a re-quote (that would needlessly burn it).
    SwapStateBusy,
    /// A swap deposit is ALREADY in flight — the SDK's one-deposit-in-flight guard
    /// refused a second while the first is queued/signing/sending. The REMEDY is the
    /// opposite of every other execute failure: do **NOT** re-quote (a re-quote is the
    /// double-deposit door this guard closes) — render "a swap is already in progress"
    /// and point the user at the tracked swap / activity surface. Self-clears when the
    /// in-flight deposit mines or its quote deadline lapses. The refusing execute
    /// consumed its quote and registered provider intent — fail-safe (an un-deposited
    /// quote refunds provider-side after its deadline; nothing left the pool).
    SwapAlreadyInFlight,
    /// The wallet is WATCH-ONLY — it holds no spending keys, so swap is
    /// STRUCTURALLY unavailable (#397 §3.7 D3; surfaced when the host attempts
    /// `enableNearSwap`, before any service is wired). NEVER retryable: unlike
    /// [SwapErrorKind.providerUnavailable] this is not a transient outage — no
    /// retry, timeout, or transport change ever clears it. Render the
    /// view-only framing ("this wallet can't swap — it can only watch") and
    /// keep the whole swap surface hidden/disabled; only a spending wallet can
    /// swap. The swap-taxonomy mirror of the wallet-level watch-only refusal.
    WatchOnly,
    /// The `SwapQuote` passed to `swapExecute` names a quote this wallet issued but
    /// carries DIFFERENT terms from the record it kept (deposit address, amount,
    /// foreign amount, memo, refund target or binding). Refused BEFORE the quote's
    /// single-use claim and before any provider or deposit call: nothing was consumed,
    /// nothing left the wallet, and the quote stays executable by the DTO `swapQuote`
    /// returned. Pass that DTO back unchanged; if you cannot, re-quote. The differing
    /// values are never surfaced (they are render-never-log).
    QuoteTermsDiffer,
    /// Forward-compatibility arm (see [WalletErrorKind.unknown]). KEEP LAST:
    /// inserting a variant above renumbers this arm's generated wire tag —
    /// safe ONLY while the Rust cdylib and the generated Dart ship from one
    /// checkout (cargokit source builds, no precompiled channel). If a
    /// precompiled-binary channel is ever added, wire tags become ABI and
    /// "Unknown always last" turns every append into a silent renumber —
    /// revisit the layout policy then (reliability NIT).
    Unknown,
}

/// Which quote side broke the user-anchored bound.
pub enum QuoteBoundSide {
    Zec,
    Foreign,
    /// Forward-compatibility arm.
    Unknown,
}

/// Why a swap destination was refused.
pub enum DestinationInvalidReason {
    /// Swapping OUT of ZEC requires a foreign destination address.
    Missing,
    /// Swapping INTO ZEC must NOT carry one (delivery goes to the wallet's
    /// own fresh address).
    NotAllowedForDirection,
    /// A ZEC→ZEC "swap" is a fee-burning provider round-trip — refused.
    ZcashAddressNotAllowed,
    /// Exceeds the SDK's provider-string bound.
    Oversized,
    /// Forward-compatibility arm.
    Unknown,
}

/// How a provider response broke the contract — typed so the host can render
/// an honest "provider misbehaved" without the SDK leaking the payload.
pub enum ProviderProtocolReason {
    /// The echoed refund address differs from the one the SDK sent —
    /// aborted before any deposit.
    RefundAddressMismatch,
    /// A provider string exceeded its bound.
    OversizedField,
    /// An amount field was not a finite, in-range decimal.
    MalformedAmount,
    /// Unknown status value.
    UnknownStatus,
    /// The deposit address failed validation (parseable, transparent, right
    /// network — checked before any signing path).
    DepositAddressInvalid,
    /// A field the flow requires was absent from the provider response.
    MissingField,
    /// HTTP-level contract break (unexpected status, or a redirect — a
    /// funds path never silently follows one).
    UnexpectedHttpStatus,
    /// The provider response exceeded the SDK's size cap for that call.
    OversizedBody,
    /// The provider response was not in the documented shape.
    UndecodableResponse,
    /// The provider asked for a deposit memo the wallet cannot honor —
    /// refused, because depositing without it would lose the funds.
    DepositMemoUnsupported,
    /// The provider no longer knows this swap.
    SwapNotFound,
    /// The provider's swap id collided with a wallet-reserved internal key
    /// namespace and was refused at the quote door (#368) — get a new quote.
    ReservedId,
    /// The provider's echo of the quote request named different terms from
    /// the ones the SDK sent (a different recipient, asset, amount, exact side
    /// or slippage) — refused before any review or deposit; get a new quote.
    RequestEchoMismatch,
    /// Forward-compatibility arm.
    Unknown,
}
