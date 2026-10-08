//! `MockSwapProvider` — the scriptable [`SwapPort`] test
//! double (W2 deliverable; spec §3.2/§8 integration tier). PUBLIC on
//! purpose: SDK hosts test their swap UI against it without a provider
//! account or network access (the same role the loopback transport plays
//! for messaging). Deterministic — no clocks, no randomness.

use std::collections::VecDeque;
use std::sync::Mutex;

use async_trait::async_trait;

use crate::error::SwapError;

use super::SwapPort;
use super::types::{QuoteRequest, SwapId, SwapQuote, SwapStatus, TokenList};

type QuoteFn = dyn Fn(&QuoteRequest) -> Result<SwapQuote, SwapError> + Send + Sync;

/// Scriptable provider double. Default: every call fails
/// `ProviderUnavailable` (the honest zero-config behavior — a mock that
/// silently succeeds hides wiring bugs).
#[derive(Default)]
pub struct MockSwapProvider {
    quote_fn: Option<Box<QuoteFn>>,
    /// Statuses served in order; empty ⇒ `Processing` (a swap in flight).
    pub statuses: Mutex<VecDeque<SwapStatus>>,
    /// Every executed swap id, in call order (assert "nothing ran" with
    /// `is_empty`).
    pub executed: Mutex<Vec<SwapId>>,
    /// Number of quote calls observed (assert request-side rejects never
    /// reach the provider).
    pub quote_calls: Mutex<u32>,
    /// Scripted `list_tokens` result (the IntoZec picker, IZ-2); `None` ⇒
    /// `ProviderUnavailable` (the honest zero-config default).
    pub tokens: Mutex<Option<TokenList>>,
}

impl MockSwapProvider {
    /// Provider whose quote response is computed from the request — write
    /// honest echoes or hostile responses in one closure.
    pub fn with_quote_fn(
        f: impl Fn(&QuoteRequest) -> Result<SwapQuote, SwapError> + Send + Sync + 'static,
    ) -> Self {
        Self {
            quote_fn: Some(Box::new(f)),
            ..Self::default()
        }
    }

    /// Queue status responses for `status()` (served FIFO).
    pub fn push_status(&self, status: SwapStatus) {
        self.statuses
            .lock()
            .expect("mock statuses")
            .push_back(status);
    }

    /// Script the `list_tokens` result (the IntoZec picker, IZ-2). Unset ⇒
    /// `ProviderUnavailable` (the honest zero-config behavior).
    pub fn set_tokens(&self, list: TokenList) {
        *self.tokens.lock().expect("mock tokens") = Some(list);
    }
}

#[async_trait]
impl SwapPort for MockSwapProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    async fn quote(&self, req: QuoteRequest) -> Result<SwapQuote, SwapError> {
        *self.quote_calls.lock().expect("mock counter") += 1;
        match &self.quote_fn {
            Some(f) => f(&req),
            None => Err(SwapError::ProviderUnavailable),
        }
    }

    async fn execute(&self, quote: &SwapQuote) -> Result<(), SwapError> {
        self.executed
            .lock()
            .expect("mock executed")
            .push(quote.id.clone());
        Ok(())
    }

    async fn status(&self, _id: &SwapId, _provider_ref: &str) -> Result<SwapStatus, SwapError> {
        Ok(self
            .statuses
            .lock()
            .expect("mock statuses")
            .pop_front()
            .unwrap_or(SwapStatus::Processing))
    }

    async fn list_tokens(&self) -> Result<TokenList, SwapError> {
        // Scripted (`set_tokens`) or the honest zero-config `ProviderUnavailable`.
        self.tokens
            .lock()
            .expect("mock tokens")
            .clone()
            .ok_or(SwapError::ProviderUnavailable)
    }
}
