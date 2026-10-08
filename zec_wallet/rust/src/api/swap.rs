//! Swap DTOs as Dart sees them (spec §2.6). Foreign amounts are DECIMAL
//! STRINGS throughout — no float, no u64-precision assumption; ZEC amounts
//! are `int` zatoshis like everywhere else. The privacy disclosure is DATA:
//! the UI renders from it, never from its own guess.

/// Provider-namespaced asset identifier (e.g. chain `"eth"`, symbol
/// `"usdc"`).
pub struct AssetId {
    pub chain: String,
    pub symbol: String,
}

/// Swap direction, with the honest privacy shape of each leg.
pub enum SwapDirection {
    /// other asset → ZEC. Ends shielded ONLY because the SDK proposes the
    /// final shield step — delivery lands on a fresh transparent address
    /// first, and that leg is on-chain-visible.
    IntoZec { from: AssetId },
    /// ZEC → other asset. DE-SHIELDS by construction: the deposit send is a
    /// shielded→transparent tx; amount + destination are exposed to the
    /// provider network.
    OutOfZec { to: AssetId },
    /// Forward-compatibility arm — keep a default arm when switching.
    Unknown,
}

/// An amount on one side of a swap.
pub enum SwapAmount {
    /// ZEC side, zatoshis.
    Zec { zat: i64 },
    /// Foreign side, decimal string (provider precision preserved).
    Foreign { amount: String },
}

/// Which side the user fixed — the user's OWN number, the anchor every
/// quote is bounds-checked against before anything is signed.
pub enum ExactSide {
    /// "I will pay exactly this much in."
    In { amount: SwapAmount },
    /// "I want exactly this much out."
    Out { amount: SwapAmount },
}

/// Quote request.
pub struct QuoteRequest {
    pub direction: SwapDirection,
    pub exact: ExactSide,
    /// Basis points; requests beyond the SDK's hard ceiling are rejected
    /// typed. The SDK default is 200 (2%).
    pub slippage_tolerance_bps: u16,
    /// OutOfZec: the foreign receive address (REQUIRED; a Zcash address is
    /// rejected — a ZEC→ZEC "swap" is a fee-burning provider round-trip).
    /// IntoZec: must be `null` (delivery goes to the wallet's own fresh
    /// address).
    pub destination: Option<String>,
    /// IntoZec: source-chain refund target. OutOfZec: IGNORED — the SDK
    /// always supplies a fresh wallet transparent address as the refund
    /// target (single-use, never recycled).
    pub refund_address: Option<String>,
}

/// A bounds-checked quote. `expiresAt` is unix seconds DISPLAY-ONLY;
/// deadline enforcement happens Rust-side against monotonic + wall clocks
/// (either says expired ⇒ rejected), so neither a regressed wall clock nor a
/// suspend-paused monotonic clock can revive a dead quote.
///
/// **#367:** `expiresAt` is the ACTIONABLE deadline — already clamped to the
/// wall window the SDK itself enforces, minus the execute margin: the exact
/// instant `swapExecute` starts refusing `QuoteExpired`. Render the countdown
/// from it directly; it cannot disagree with the execute gate (the raw
/// provider echo — hours past the real window — never reaches this DTO).
pub struct SwapQuote {
    /// The SDK-minted swap id (opaque, bounded; ADR-0555 — never the deposit address).
    pub id: String,
    /// Provider deposit address (quote-generated, single-use).
    pub deposit_address: String,
    /// A destination tag / memo the provider REQUIRES on the deposit when the
    /// source chain has one (XRP tag, Cosmos/Stellar/EOS memo). IntoZec only —
    /// the user attaches it to their external deposit; render it on the deposit
    /// screen so the deposit isn't lost. `None` for chains without the concept.
    /// A §5.4 NEVER-log value, like `deposit_address`.
    pub deposit_memo: Option<String>,
    /// Unix seconds, display-only (see type docs).
    pub expires_at: i64,
    /// Decimal string — what goes in.
    pub amount_in: String,
    /// Decimal string — the minimum that comes out.
    pub min_amount_out: String,
    /// Whichever side is ZEC, exact (zatoshis).
    pub zec_side_zat: i64,
    /// The provider's echo of the refund address the SDK sent (verified to
    /// match before anything downstream).
    pub refund_to: Option<String>,
    /// Render this — it expresses everything the user must acknowledge.
    pub disclosure: SwapPrivacyDisclosure,
    /// FR-17 (#396): the SDK-minted spend-binding nonce for this quote (32 opaque
    /// bytes, random, NOT key material) — always set on a quote returned by
    /// `swapQuote`; the nullability is a WIRE ARTIFACT of the core type (network
    /// adapters populate null and the service overwrites at issue), never an
    /// expected runtime state for a served quote.
    ///
    /// FR-17 ENFORCEMENT PREREQUISITE: recording this only protects the OutOfZec
    /// deposit if the host registered the BOUND C-ABI seed supplier
    /// (`zec_wallet_register_seed_port_bound`) with a comparing callback; a v1
    /// supplier drops the binding (silent no-op). Do not log it. A host with external seed custody records it at its
    /// execute-authorize bracket; the OutOfZec deposit's native seed pull presents
    /// the same value, so the host's supplier fail-closes a sign for anything the
    /// user did not review. Hosts without a native seed port can ignore it. Do
    /// not log it.
    pub binding: Option<Vec<u8>>,
}

/// Honest per-direction privacy surfacing AS DATA.
pub struct SwapPrivacyDisclosure {
    /// Funds end in the shielded pool (via the final shield step).
    pub ends_shielded: bool,
    /// The deposit de-shields funds.
    pub deshields: bool,
    /// Provider-side legs are PUBLIC chain data.
    pub provider_legs_transparent: bool,
    /// What the provider learns.
    pub provider_sees: Vec<DisclosureItem>,
}

/// One thing the provider learns. Render every item; the list is the
/// disclosure.
pub enum DisclosureItem {
    /// The provider links the two assets to one user intent.
    CrossAssetLink,
    /// Amounts on both sides.
    Amounts,
    /// The destination address.
    DestinationAddress,
    /// The source address.
    SourceAddress,
    /// The caller's IP address — unless the request rides Tor.
    IpUnlessTor,
    /// Forward-compatibility arm — render as a generic disclosure line.
    Unknown,
}

/// Provider-status lifecycle for an in-flight swap.
pub enum SwapStatus {
    /// Waiting for the deposit; `expiresAt` unix seconds, display-only.
    PendingDeposit {
        expires_at: i64,
    },
    /// Partial deposit landed: top up by `deadline` or the provider
    /// refunds. Decimal strings (foreign side).
    UnderDeposited {
        received: String,
        missing: String,
        deadline: i64,
    },
    DepositDetected,
    Processing,
    /// Terminal success. `realizedSlippageBps` when the provider reports it.
    Success {
        out_txid: Option<String>,
        realized_slippage_bps: Option<i32>,
    },
    /// Terminal: refunded — a NAMED outcome with its own UX row, not an
    /// error.
    Refunded {
        refund_txid: Option<String>,
    },
    /// Terminal failure.
    Failed {
        code: SwapFailureCode,
    },
    /// Forward-compatibility arm — render as "status unavailable".
    Unknown,
}

/// Terminal failure classification.
pub enum SwapFailureCode {
    /// Provider reported failure.
    ProviderFailure,
    /// Provider response violated the protocol contract.
    ProviderProtocol,
    /// Deposit window lapsed before a deposit confirmed.
    Expired,
    /// The provider no longer recognizes this swap (#367): a definitive
    /// not-found answered several consecutive status polls — the order was
    /// most likely cleaned up provider-side after it expired. SYNTHESIZED by
    /// the SDK's poll policy, never provider-sent. Copy note: a deposit that
    /// DID land on such an order still refunds provider-side to the recorded
    /// refund address — do not assert loss.
    NotFound,
    /// Forward-compatibility arm.
    Unknown,
}

/// Direction arm of a durable in-flight swap record — COARSE by design: the
/// record stores no asset names or amounts (data minimization; the host polls
/// provider truth live via `watchSwapStatus`). Distinct from [SwapDirection]
/// (whose arms carry the quoted [AssetId]) precisely because the durable row
/// has no assets to carry.
pub enum SwapRecordDirection {
    /// Other asset → ZEC: the USER deposits externally.
    IntoZec,
    /// ZEC → other asset: the WALLET sends the deposit.
    OutOfZec,
    /// Forward-compatibility arm — render as a generic in-flight swap.
    Unknown,
}

/// The observed TERMINAL a host pins into a record via `recordSwapOutcome`
/// (#367) — the coarse terminal arms only, no payloads (txids/amounts stay
/// live-polled, never durable). A pinned outcome is what the home row renders
/// after the tracking stream saw the terminal while the user was away.
pub enum SwapOutcome {
    Success,
    Refunded,
    Failed,
}

/// One durable in-flight swap (W-swap-5, #366) — the home row written at
/// execute and listed by `listInFlightSwaps` after a process restart, screen
/// re-entry, or session swap. Lives in the wallet's encrypted store, in its
/// own table — records stay READABLE even when swap is disabled or killed
/// (local reads are not swap traffic, §3.5). The row asserts exactly "an
/// order was registered at execute" — plus, since #367, at most one PINNED
/// observed terminal (`outcome`): feed `id` to `watchSwapStatus` for live
/// provider truth, call `recordSwapOutcome` when the stream observes a
/// TERMINAL status, and call `dismissSwapRecord` only on USER intent (the
/// terminal card's Done after the outcome rendered, or an explicit Remove).
pub struct SwapRecord {
    /// The SDK-minted swap id — THE re-attach handle for `watchSwapStatus`.
    /// §5.4 render-never-log by policy; since S8 it is NEVER the provider's
    /// deposit address (ADR-0555: that is data inside the record).
    pub id: String,
    pub direction: SwapRecordDirection,
    /// Unix seconds, display-only.
    pub created_at: i64,
    /// When the DEPOSIT must land (OutOfZec: the wallet's own clamped send
    /// window; IntoZec: the window the USER must beat). Unix seconds,
    /// display-only; absent if the recording build predates it.
    pub deposit_deadline: Option<i64>,
    /// The record's self-lapse bound (execute-time + the settlement
    /// ceiling): past this the row stops listing on its own. Unix seconds,
    /// display-only.
    pub expires_at: i64,
    /// The PINNED observed terminal (#367) — `null` while in flight (and for
    /// a pin written by a newer SDK this build doesn't recognize: fail-honest,
    /// the live poll re-reveals truth). Render the terminal row state from it.
    pub outcome: Option<SwapOutcome>,
}

/// One pickable source asset for the host's IntoZec token picker (§3.3b D5 / ADR-0530, IZ-2) — the
/// flat bridge mirror of the core `TokenInfo`. The host renders `(chain, symbol)` with a BUNDLED
/// icon + a display label (never CDN-fetched, §5.2) and threads `(chain, symbol)` back as the
/// `QuoteRequest` asset; `provider_asset_id` is the stable unique list key, `decimals` the foreign-
/// amount precision, `price_usd` display-only (sort/label, NEVER funds math). §5.4: no token field
/// is ever logged.
pub struct SwapToken {
    pub chain: String,
    pub symbol: String,
    pub decimals: u8,
    pub provider_asset_id: String,
    pub price_usd: Option<f64>,
}

/// The result of `swapListTokens` (§3.3b D5/L6, IZ-2): the filtered picker list + a freshness flag.
/// `fresh == false` ⇒ the live fetch failed and these are the LAST good tokens served from cache —
/// the host shows a "couldn't refresh, showing cached" banner (honest degradation), never a blank
/// picker. An empty `tokens` with `fresh == true` is the honest "no assets available right now."
pub struct SwapTokenList {
    pub tokens: Vec<SwapToken>,
    pub fresh: bool,
}

/// Live-provider construction config for the NEAR Intents adapter — the
/// `enableNearSwap` entry (spec §3.5; the `swap-near` feature). `endpoint` is
/// the 1Click base URL (validated at construction, not at first dial).
pub struct SwapProviderConfig {
    /// Provider base URL (e.g. `https://1click.chaindefuser.com`).
    pub endpoint: String,
    /// OPTIONAL provider auth token. A §5.4 NEVER-log value: wrapped in a
    /// `Zeroizing` buffer in Rust at the boundary and never logged. Dart memory
    /// cannot be zeroized — the documented §10 exposure, identical framing to
    /// the mnemonic crossing; keep it minimal-residency host-side.
    pub jwt: Option<String>,
}

/// The §3.5 monotonic-toward-off kill severity a host may DECLARE at swap
/// construction (e.g. lowered from its signed network manifest). Ordering is
/// `Live` < `WindDown` < `Hard`; the SDK only ever escalates toward off (a
/// forged/replayed "on" can only make it MORE-off, never resurrect swap).
pub enum SwapKill {
    /// Normal operation — quote/execute/poll all run.
    Live,
    /// New quotes/executes are refused; any in-flight swap keeps POLLING to a
    /// terminal status (observational; funds settle/refund provider-side).
    WindDown,
    /// Zero swap traffic — new AND in-flight polling stop, no synthetic terminal
    /// (the host renders "tracking unavailable" from its own state).
    Hard,
    /// Forward-compatibility arm (G2 policy). An UNRECOGNIZED declared kill is a
    /// host bug; the SDK treats it as the strongest kill it understands (`Hard`,
    /// fail-safe) — an unknown directive must never resurrect swap.
    Unknown,
}
