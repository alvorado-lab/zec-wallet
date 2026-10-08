//! Swap domain types (spec §2.6) — port-level, provider-agnostic. Foreign
//! amounts are DECIMAL STRINGS throughout (no float, no u64-precision
//! assumption); every provider-supplied string is bounds-checked before DTO
//! entry (§4.6 m1).

use crate::money::Zatoshis;
use crate::seed::SpendBinding;

/// Provider-namespaced asset identifier.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AssetId {
    pub chain: String,
    pub symbol: String,
}

/// One pickable source asset for the dynamic token list (§3.3b D5 / ADR-0530, IZ-2): the
/// provider-agnostic, port-level shape the host's IntoZec picker renders. Built by the adapter from
/// its `/v0/tokens` listing (filtered — the ZEC asset itself and any `$0`/null-price entry are
/// dropped before this type exists), so the host shows only real, quotable foreign assets. The host
/// maps `(chain, symbol)` to a BUNDLED icon + a display label (never CDN-fetched, §5.2) and threads
/// `(chain, symbol)` back into a [`QuoteRequest`] as the [`AssetId`]; `provider_asset_id` is the
/// stable unique key (dedup / list identity), `decimals` the precision for the foreign-amount field,
/// `price_usd` display-only sort/label (NEVER funds math — §2.6).
///
/// **§5.4:** token metadata is NEVER logged beyond coarse counts (`wallet.swap_tokens {count,
/// outcome}`) — no symbol, chain, id, or price ever enters a span.
#[derive(Clone, PartialEq, Debug)]
pub struct TokenInfo {
    pub chain: String,
    pub symbol: String,
    pub decimals: u8,
    pub provider_asset_id: String,
    pub price_usd: Option<f64>,
}

/// The result of [`SwapPort::list_tokens`](super::SwapPort::list_tokens) (§3.3b D5/L6, IZ-2): the
/// filtered picker list plus a FRESHNESS flag. `fresh = true` ⇒ a live `/v0/tokens` fetch
/// succeeded; `fresh = false` ⇒ the fetch failed (timeout / unreachable) and these are the LAST
/// good list served from cache — the host renders a "couldn't refresh, showing cached" banner
/// (honest degradation, §6/L6), never a blank picker. An empty `tokens` with `fresh = true` is the
/// honest "no assets available right now" (everything filtered), distinct from a stale fallback.
#[derive(Clone, PartialEq, Debug)]
pub struct TokenList {
    pub tokens: Vec<TokenInfo>,
    pub fresh: bool,
}

/// Direction, with the honest privacy shape of each (§2.6):
#[derive(Clone, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum SwapDirection {
    /// other asset → ZEC. Privacy-positive END STATE: delivery lands on a FRESH
    /// per-swap address `SwapService` mints at a single-use external index via the
    /// audited engine `get_address_for_index` (§3.3b D1 / ADR-0530) — a Unified
    /// Address carrying a transparent receiver (1Click pays it in practice) AND an
    /// orchard receiver (a shielded-paying provider just works — universal). The
    /// transparent leg surfaces the one-tap Recv-3 shield nudge (§3.3b D3); a
    /// shielded delivery needs no shield. `ends_shielded` is the NUDGED expected end
    /// state, not a guarantee — the transparent leg is on-chain-visible until shielded.
    /// (Supersedes the 2026-06-10 "no shielded support → bare transparent" note.)
    IntoZec { from: AssetId },
    /// ZEC → other asset. DE-SHIELDS by construction: the deposit send is a
    /// shielded→transparent tx to the provider's t-addr; amount + destination
    /// exposed to the solver network.
    OutOfZec { to: AssetId },
}

/// An amount on one side of a swap.
#[derive(Clone, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum SwapAmount {
    Zec(Zatoshis),
    /// Foreign amount as a decimal string (provider precision preserved).
    Foreign(String),
}

/// Which side the user fixed (§2.6 `exact`): the user's OWN number — the
/// anchor every quote is bounds-checked against (M1).
#[derive(Clone, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum ExactSide {
    In(SwapAmount),
    Out(SwapAmount),
}

/// Quote request (§2.6).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct QuoteRequest {
    pub direction: SwapDirection,
    pub exact: ExactSide,
    /// Default `SLIPPAGE_DEFAULT_BPS`; requests beyond `SLIPPAGE_MAX_BPS`
    /// are rejected typed (M1 hard ceiling).
    pub slippage_tolerance_bps: u16,
    /// OutOfZec: foreign receive address — validated; a Zcash address is
    /// REJECTED (a ZEC→ZEC "swap" is a fee-burning provider round-trip,
    /// §1.7). IntoZec: `None` (we receive to our own fresh address).
    pub destination: Option<String>,
    /// IntoZec: user-supplied source-chain refund target. OutOfZec: IGNORED —
    /// `SwapService` ALWAYS populates the provider's refundTo with a FRESH
    /// wallet transparent address (the deposit comes from the shielded pool;
    /// without this the refund path doesn't exist — §2.6). Single-use, never
    /// recycled (HARD-H).
    pub refund_address: Option<String>,
}

/// A swap's identity. TWO values wear this type, and only ONE of them is an
/// identity (stage S8, the R01 fold):
///
/// - As populated by an ADAPTER on the [`SwapQuote`] it returns: the provider's
///   own handle — for the 1Click rail the `quote` response's `depositAddress`
///   (§3.2). `SwapService::quote` takes it OFF the DTO and keeps it as
///   `provider_ref` on the durable records: DATA the adapter is handed back for
///   its status URL / deposit-notification body, and nothing else.
/// - As returned by `SwapService::quote` (and so everywhere downstream —
///   `execute`'s argument and return, the `swap_record` home row,
///   `watch_status`): the SDK-MINTED execution identity, [`Self::mint`] — 128
///   bits from the OS CSPRNG, hex-encoded, independent of the quote's content.
///   It is the ONE key the issued record is stored, compared and consumed by,
///   the home row and refund watch are kept under, and the adapter's
///   per-swap circuit-isolation key is derived from. A provider that reuses,
///   reissues or shares a deposit address across quotes cannot make two of
///   these collide, and a duplicate is an invariant violation the stores
///   surface typed, never first-wins.
///
/// **§5.4:** the provider handle (`provider_ref`, a deposit address) is the
/// NEVER-log item. The minted id carries no address bytes, but it still stays
/// out of every `tracing`/log field: an id links log lines to one user's swap.
/// The `wallet.swap` and `wallet.swap_poll` spans carry the provider NAME + an
/// outcome code only.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct SwapId(String);

/// Width of a minted [`SwapId`] before hex-encoding: 128 bits of OS randomness
/// (the `SpendBinding` precedent; uniqueness across every quote one wallet
/// ever issues is all the key needs).
const SWAP_ID_BYTES: usize = 16;

impl SwapId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Mint a fresh execution identity — `SWAP_ID_BYTES` from the OS CSPRNG,
    /// lowercase hex (32 chars). Content-independent: two quotes with identical
    /// terms get two ids.
    pub fn mint() -> Self {
        let mut bytes = [0u8; SWAP_ID_BYTES];
        rand_core::RngCore::fill_bytes(&mut rand_core::OsRng, &mut bytes);
        Self(hex::encode(bytes))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A bounds-checked quote (§2.6). `expires_at` is unix seconds DISPLAY-ONLY;
/// deadline ENFORCEMENT is anchored as a provider-relative duration captured
/// at response receipt + monotonic elapsed (− `DEADLINE_SAFETY_MARGIN_SECS`)
/// — never raw device wall-clock (§2.6 clock-skew posture).
///
/// **#367 (as returned by `SwapService::quote`):** `expires_at` is the
/// ACTIONABLE deadline — the provider echo ceiling-clamped to the wall window
/// the wallet itself requested AND minus the direction-aware execute margin —
/// i.e. the exact instant `execute` starts refusing `QuoteExpired`. The raw
/// provider echo (observed live at ~72–96 h against a ~10-min actionable
/// window) never reaches a host: a countdown rendered from this field and the
/// execute gate agree by construction. Adapters still populate the RAW
/// provider value; the service clamps before returning.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SwapQuote {
    /// See [`SwapId`]: an adapter populates the PROVIDER's handle; the value a
    /// host receives from `SwapService::quote` is the SDK-minted execution
    /// identity, and the only thing `execute` / `watch_status` accept.
    pub id: SwapId,
    /// Quote-generated, single-use (provider-side).
    pub deposit_address: String,
    /// A destination tag / memo the provider REQUIRES on the deposit, when the
    /// source chain has one (XRP destination tag, Cosmos/Stellar/EOS memo, …).
    /// IntoZec ONLY (the USER attaches it to their external deposit, §4.4) — it
    /// is SHOWN on the D7 deposit screen so the deposit isn't lost; absent for
    /// chains without the concept (ZEC carries none). OutOfZec never carries one
    /// (the wallet sends its own deposit and cannot attach a source-chain memo —
    /// a memo-requiring OutOfZec quote is a typed refusal upstream). Bounded at
    /// the §4.6 funnel; a §5.4 NEVER-log value like `deposit_address`.
    pub deposit_memo: Option<String>,
    /// Unix seconds, display-only (see type docs).
    pub expires_at: u64,
    /// Foreign amounts as decimal strings.
    pub amount_in: String,
    pub min_amount_out: String,
    /// Whichever side is ZEC, exact.
    pub zec_side: Zatoshis,
    /// The provider's ECHO of the refund address we sent — `SwapService`
    /// verifies it equals ours before anything downstream (§2.6 M1 fold;
    /// mismatch ⇒ `ProviderProtocol`, abort before any deposit). As-built
    /// addition to the spec DTO, owed back to §2.6 at the W2 fold.
    pub refund_to: Option<String>,
    pub disclosure: SwapPrivacyDisclosure,
    /// FR-17 (#396): the SDK-minted spend-binding nonce for this quote — SET BY
    /// `SwapService::quote` (adapters populate `None`; the service is the minting
    /// authority, never the provider — a provider-controlled value has no
    /// uniqueness contract). The host records it at its execute-authorize bracket;
    /// the OutOfZec deposit's sign pull presents the same value, so a stage
    /// recorded for THIS quote can never be consumed signing anything else.
    /// Opaque + random, NOT key material.
    pub binding: Option<SpendBinding>,
}

/// Honest per-direction privacy surfacing AS DATA (ADR-0014) — the UI
/// renders from this, never from its own guess. The DTO must be able to
/// express every sentence of the ecosystem's disclosure copy (§1.7).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SwapPrivacyDisclosure {
    /// IntoZec: true (via the final shield step).
    pub ends_shielded: bool,
    /// OutOfZec: true (deposit de-shields by construction).
    pub deshields: bool,
    /// Provider-side legs are PUBLIC chain data.
    pub provider_legs_transparent: bool,
    pub provider_sees: Vec<DisclosureItem>,
}

/// What the provider learns (§2.6).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum DisclosureItem {
    CrossAssetLink,
    Amounts,
    DestinationAddress,
    SourceAddress,
    IpUnlessTor,
}

/// Provider-status lifecycle (§2.6; the fixed 1Click mapping lives in the
/// adapter — anything unknown maps to `Failed(ProviderProtocol)`, never a
/// panic).
#[derive(Clone, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum SwapStatus {
    PendingDeposit {
        expires_at: u64,
    },
    /// Partial deposit landed (§1.7): top up by the deadline or the provider
    /// refunds. Decimal strings (foreign side).
    UnderDeposited {
        received: String,
        missing: String,
        deadline: u64,
    },
    DepositDetected,
    Processing,
    Success {
        out_txid: Option<String>,
        realized_slippage_bps: Option<i32>,
    },
    /// A NAMED failure path with its own UX row, not an error (ADR-0014).
    Refunded {
        refund_txid: Option<String>,
    },
    Failed {
        code: SwapFailureCode,
    },
}

impl SwapStatus {
    /// True once the swap reached a TERMINAL state — status polling STOPS here
    /// (§7: "stops on terminal status"). `Success`, `Refunded` (a NAMED
    /// success-of-the-refund-path with its own UX row, ADR-0014 — terminal, not
    /// an error), and `Failed` are terminal; `PendingDeposit` / `UnderDeposited`
    /// / `DepositDetected` / `Processing` are still in flight. Exhaustive (no
    /// wildcard) so a future `#[non_exhaustive]` variant forces a deliberate
    /// terminal/in-flight classification here rather than silently defaulting to
    /// "keep polling forever" (the §3.2 silent-hang the resume contract forbids).
    pub fn is_terminal(&self) -> bool {
        match self {
            Self::Success { .. } | Self::Refunded { .. } | Self::Failed { .. } => true,
            Self::PendingDeposit { .. }
            | Self::UnderDeposited { .. }
            | Self::DepositDetected
            | Self::Processing => false,
        }
    }
}

/// Terminal failure classification (§2.6).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum SwapFailureCode {
    /// Provider reported failure.
    ProviderFailure,
    /// Provider response violated the protocol contract (unknown status,
    /// refundTo mismatch, malformed amounts — §4.6).
    ProviderProtocol,
    /// Deposit window lapsed before a deposit confirmed.
    Expired,
    /// The provider no longer recognizes this swap (#367 poll policy): a
    /// DEFINITIVE `SwapNotFound` answered `SWAP_NOT_FOUND_TERMINAL_POLLS`
    /// consecutive status polls — the order was most likely GC'd provider-side
    /// after it expired. SYNTHESIZED by the poll loop, never provider-sent.
    /// Materially different from [`Self::Expired`]: the provider can no longer
    /// tell us anything about this swap at all — but a deposit that DID land
    /// on a GC'd order still refunds provider-side to the recorded refund
    /// address (the copy must not assert loss).
    NotFound,
}

// The old placeholder `SwapRecord` "local swap history row" (id + direction +
// status + created_at + deposit_txid) was REPLACED at W-swap-5 (#366) by the
// crate-root [`crate::InFlightSwap`] — the honest durable minimum actually
// produced by the `swap_record` store (no status snapshot, no txid link). The
// readable-when-swap-off doctrine it declared lives on in that store's docs.

/// Swap kill severity (§3.5 layers 3/4 — the regulatory off-switch as it
/// reaches the RUNNING `SwapService`; layer 1 is the cargo feature, layer 2 is
/// "don't construct a provider at all"). MONOTONIC-TOWARD-OFF (review M4): a
/// kill may only SUBTRACT capability — the service escalates and NEVER
/// resurrects swap; re-enabling is host RECONSTRUCTION (layer 2), not a state
/// transition. The host (e.g. from a signed network manifest, W5) maps its flag
/// through [`resolve_manifest_kill`] so the severity that reaches the service
/// is always TYPED and DEFINED — never left unresolved at the composition
/// boundary (`manifest_kill_severity_reaches_swap_service`).
///
/// Variants are declared in MONOTONIC-OFF ORDER (`Live < WindDown < Hard`) and
/// that order IS the contract: the derived `Ord` is what `set_kill` compares to
/// escalate-only. Any future severity MUST be inserted in strictly-more-off
/// position (pinned by the ordering assertion in
/// `manifest_kill_severity_reaches_swap_service`). `#[non_exhaustive]` keeps
/// the variant set open to external consumers (G2 policy) while the ordering
/// stays a closed internal invariant.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[non_exhaustive]
pub enum SwapKill {
    /// Normal operation — quotes and executes are accepted.
    Live,
    /// Refuse NEW quotes/executes immediately; in-flight swaps still poll to
    /// terminal (status is OBSERVATIONAL — a deposit settles or auto-refunds
    /// provider-side whether or not we ever poll, §3.5). The SAFE default for
    /// any "swap off" signal whose severity is unresolved.
    WindDown,
    /// Zero swap traffic from this instant: new quotes/executes refused AND
    /// in-flight status polling stopped (in-flight swaps become
    /// tracking-unavailable, surfaced honestly). Strictly more-off than
    /// `WindDown`. NOTE: the WindDown↔Hard divergence on in-flight STATUS
    /// POLLING lands with the status-poll chunk; at the quote/execute door
    /// both severities refuse NEW work identically (which is correct).
    Hard,
}

impl SwapKill {
    /// True once swap is killed at ANY severity — the door both `quote` and
    /// `execute` check. Fail-safe: any future `#[non_exhaustive]` variant other
    /// than `Live` is treated as killed.
    pub(crate) fn is_killed(self) -> bool {
        !matches!(self, Self::Live)
    }

    /// True once the kill is severe enough to STOP in-flight status polling — the
    /// §3.5 WindDown↔Hard divergence. `WindDown` refuses NEW work but keeps
    /// polling in-flight swaps to terminal (status is OBSERVATIONAL — a deposit
    /// settles or auto-refunds provider-side whether or not we ever poll), so it
    /// does NOT stop polling. `Hard` (zero swap traffic from this instant) does.
    /// Expressed as `>= Hard` against the monotonic-off `Ord` (NOT `== Hard`) so
    /// any future `#[non_exhaustive]` severity inserted strictly-more-off than
    /// `Hard` is FAIL-SAFE — at least as restrictive, never accidentally lenient.
    pub(crate) fn stops_polling(self) -> bool {
        self >= Self::Hard
    }
}

/// Resolve a host kill directive into a DEFINED [`SwapKill`] (§3.5 layer 3
/// composition contract). A manifest may carry "swap off" with the severity
/// field absent or unparseable (a partially delivered manifest over a flaky
/// link); the resolution is NEVER left unresolved:
///
/// - `swap_enabled == true` ⇒ [`SwapKill::Live`] (the severity field is moot
///   when swap is on).
/// - disabled + explicit [`SwapKill::Hard`] ⇒ `Hard`.
/// - disabled + ANYTHING ELSE (none / partial / `WindDown` / a contradictory
///   `Live`) ⇒ the SAFE default [`SwapKill::WindDown`]: a disabled swap is at
///   LEAST wound down.
///
/// The service additionally enforces M4 monotonicity on top of whatever this
/// returns, so a later weaker directive can never resurrect a harder kill.
pub fn resolve_manifest_kill(swap_enabled: bool, declared: Option<SwapKill>) -> SwapKill {
    if swap_enabled {
        SwapKill::Live
    } else if matches!(declared, Some(SwapKill::Hard)) {
        SwapKill::Hard
    } else {
        SwapKill::WindDown
    }
}
