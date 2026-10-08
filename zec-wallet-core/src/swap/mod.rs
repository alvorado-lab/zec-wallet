//! Swap on-ramp (ADR-0014; spec §2.6/§3.2/§3.5). Compiled only under the
//! `swap` cargo feature — kill layer 1: off ⇒ not one byte of this module in
//! the binary. The provider seam is [`SwapPort`]; NEAR Intents is ONE adapter
//! behind it (a separate crate, v1.x, MiCA-gated host-side).

mod mock;
mod service;
mod types;

pub use service::{
    DepositSender, DestinationAddressSource, IssuedDeposit, IssuedDestination, IssuedQuotePersist,
    IssuedQuoteRecord, IssuedQuoteStore, IssuedRefund, QuoteTerms, RefundAddressSource,
    StartedSwap, SwapRecordSink, SwapService, SwapStatusSink, SwapWatch,
};
pub use types::*;

/// Test doubles for SDK hosts (and our own integration tier).
pub mod testing {
    pub use super::mock::MockSwapProvider;
}

use async_trait::async_trait;

use crate::error::SwapError;

/// Swap provider seam (ADR-0014). At W2 the only impl is the in-crate
/// [`testing::MockSwapProvider`]; the NEAR adapter ships v1.x behind the
/// host's MiCA gate (§3.4: no provider constructed ⇒ no swap surface).
#[async_trait]
pub trait SwapPort: Send + Sync {
    /// Stable, secret-free provider label for the §5.4 `wallet.swap` span
    /// (e.g. `"near-intents"`). MUST be a constant — it is logged on every
    /// execute, so it carries NO address/amount/credential, only the adapter's
    /// identity. Required (no default) so a new provider can never emit a silent
    /// `"unknown"` into the observability surface.
    fn name(&self) -> &'static str;
    async fn quote(&self, req: QuoteRequest) -> Result<SwapQuote, SwapError>;
    /// Registers intent with the provider ("execute" = the ADR-0014
    /// vocabulary; it does NOT sign — for OutOfZec the deposit send is built
    /// and signed by the WALLET in Rust, §4.4; providers never touch keys).
    ///
    /// Returns nothing (stage S8): the swap's identity is the SDK-minted
    /// `quote.id` the service assigned at quote time — an adapter has no id of
    /// its own to hand back, and the reference UI's timeout→track arm (which
    /// loses this call's result to Dart's `.timeout`) tracks by that same id
    /// through the durable home row, so an adapter cannot break it. The
    /// provider-side handle this swap is polled under is `provider_ref` on
    /// the records, handed back to [`Self::status`] as data.
    async fn execute(&self, quote: &SwapQuote) -> Result<(), SwapError>;
    /// One status poll. `id` is the SDK-minted execution identity — the input
    /// to the adapter's per-swap circuit-isolation key and nothing else on the
    /// wire; `provider_ref` is the provider's OWN handle for this swap (1Click:
    /// the deposit address), resolved by the caller from the durable home row
    /// and carried here as DATA for the request. Both are §5.4 never-log; an
    /// empty / over-bound `provider_ref` is a typed refusal, never a request.
    async fn status(&self, id: &SwapId, provider_ref: &str) -> Result<SwapStatus, SwapError>;
    /// The dynamic source-asset list for the host's IntoZec picker (§3.3b D5 / ADR-0530, IZ-2):
    /// the provider's supported tokens, FILTERED to real quotable foreign assets (the ZEC asset
    /// itself + any `$0`/null-price entry dropped) and mapped to the port-level [`TokenInfo`].
    /// LAZY + cached: the adapter fetches `/v0/tokens` on demand under its OWN circuit isolation
    /// (never the sync circuit, §2.3) and, on a fetch fault, serves the LAST good list from cache
    /// with [`TokenList::fresh`]` = false` (honest degradation, §6/L6) rather than failing — so an
    /// unstable network degrades to a stale-but-usable picker, never a blank one. An empty fresh
    /// list is the honest "no assets right now." A total failure with no cache is the typed error.
    ///
    /// **§5.4:** an impl MUST NOT log any token field — only the coarse `{count, outcome}` the
    /// service span records.
    async fn list_tokens(&self) -> Result<TokenList, SwapError>;
}
