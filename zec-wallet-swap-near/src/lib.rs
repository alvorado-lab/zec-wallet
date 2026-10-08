//! # zec-wallet-swap-near — NEAR Intents 1Click `SwapPort` adapter
//!
//! The SDK's first real swap provider (ADR-0014 rail, ADR-0525 shipping):
//! a **thin, hand-audited, typed REST client** over the four-endpoint 1Click
//! HTTP API (`get_tokens` / `get_quote` / `submit_deposit_tx` /
//! `get_execution_status`), riding the SDK's [`NetDialer`]/`TorPolicy`
//! stack — this crate opens no socket of its own, so `TorPolicy::Required`
//! fail-closes swap traffic (§3.2 m5).
//!
//! **Corrected at T0-5:** the parenthetical used to read "no tokio `net`, no
//! HTTP-client crate with a dialer". The second half holds — Rust cannot `use`
//! a crate that is not a direct dependency, and none of the socket-capable ones
//! is. The first half did not: cargo unifies features across the workspace, so
//! the one tokio in the graph HAS `net` (measured: `cargo tree -e features -i
//! tokio`, reached through `zcash_client_backend`'s
//! `lightwalletd-tonic-transport`), and this crate compiles against it. The
//! enforcement therefore lives in
//! `tests/policy.rs::adapter_cannot_open_its_own_sockets`, which reads THIS
//! crate's sources and refuses any socket API found in them.
//!
//! Wire contract: pinned by recorded fixtures + the official OpenAPI models
//! (`UPSTREAM.md`); the official `one-click-sdk-rs` is REFERENCE ONLY, never
//! a dependency (ADR-0525: reqwest transport can't honor the dialer,
//! unbounded `String`s violate §4.6).
//!
//! Port semantics against this provider:
//! - [`SwapPort::quote`] sends the REGISTERING (`dry: false`) quote — in the
//!   1Click model the quote response's `depositAddress` IS the provider's live
//!   handle; there is no separate create call. An unexecuted quote simply
//!   expires provider-side. The adapter populates that handle as the returned
//!   quote's `id`; `SwapService::quote` replaces it with the SDK-minted
//!   execution identity and keeps the handle as `provider_ref` (stage S8).
//! - [`SwapPort::execute`] is therefore a LOCAL commit point (mirrors the
//!   `MockSwapProvider` contract): the deadline gates live in `SwapService`.
//! - [`SwapPort::status`] keys `GET /v0/status` off `provider_ref` (the deposit
//!   address, handed in as data) and isolates its circuit by the SDK id.
//! - `submit_deposit_tx` ships typed + fixture-pinned as
//!   [`NearIntentsProvider::notify_deposit_tx`]; the port wiring lands with
//!   the send pipeline (the deposit txid exists only after broadcast).
//!
//! Both directions complete (IZ-1, §3.3b): OutOfZec (we send ZEC) and IntoZec
//! (we receive ZEC to a FRESH engine-persisted destination the wallet mints via
//! `get_address_for_index` and the `SwapService` binds as `recipient`). The
//! adapter is direction-symmetric — only WHICH asset is origin vs destination
//! flips; `refund_to`/`recipient` roles mirror (ADR-0530).

#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use http::Method;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use zec_wallet_core::constants::{
    PROVIDER_STR_MAX_BYTES, SWAP_DEADLINE_DEFAULT_SECS, SWAP_DEPOSIT_DEADLINE_OUT_OF_ZEC_SECS,
};
use zec_wallet_core::{
    NetDialer, ProviderProtocolReason, QuoteRequest, SwapDirection, SwapError, SwapId, SwapPort,
    SwapQuote, SwapStatus, TokenInfo, TokenList,
};
// ring is the workspace's pinned TLS stack (the rustls `ring` provider); its
// `hmac` + `rand` also mint the per-swap isolation tokens below — one audited
// crate, taken whole (crypto-rules Rule Zero), and the direct edge is what
// binds the workspace `=` pin (rust-patterns §Cross-compile).
use ring::hmac;
use ring::rand::SecureRandom as _;
use zeroize::Zeroizing;

mod http_client;
mod map;
mod time;
mod wire;

#[cfg(test)]
mod provider_tests;
#[cfg(test)]
mod wire_contract_tests;

use http_client::{
    HttpClient, HttpOutcome, QUOTE_RESPONSE_MAX_BYTES, RustlsSecurer, STATUS_RESPONSE_MAX_BYTES,
    TOKENS_RESPONSE_MAX_BYTES,
};
// `SwapAsset` is the adapter's raw wire-mapped table row; the host-facing picker surface is the
// PORT `SwapPort::list_tokens` → `TokenInfo` (the only public provider-data surface). Tightening
// this pre-existing re-export + `supported_tokens` to crate-internal is a noted follow-up (it
// cascades `unreachable_pub` through `map::SwapAsset`'s fields — a focused cleanup, not IZ-2).
pub use map::SwapAsset;

/// The production 1Click endpoint (override per environment via
/// [`SwapProviderConfig::endpoint`]).
pub const DEFAULT_ENDPOINT: &str = "https://1click.chaindefuser.com";

/// The provider's ZEC listing, pinned: chain id + symbol + the base-unit
/// scale that makes provider amounts zatoshis. A listing whose decimals
/// drift from 8 is a protocol violation (amounts would be uninterpretable),
/// not something to adapt to silently.
const ZEC_CHAIN_ID: &str = "zec";
const ZEC_SYMBOL: &str = "ZEC";
const ZEC_DECIMALS: u8 = 8;

/// Isolation bucket for pre-swap calls (quote, token list): they share one
/// circuit bucket with each other but are unlinkable from sync traffic and
/// from per-swap status polling (which isolates per swap through an
/// SDK-minted opaque token, [`SwapIsolationMinter`] — calls about different
/// swaps MUST be unlinkable at the network layer, §2.3).
const ISOLATION_KEY_QUOTE: &str = "swap/quote";

/// Prefix of the per-swap isolation key: `swap/id/` + 32 hex characters of an
/// SDK-minted token (fixed width, [`SWAP_ISOLATION_TOKEN_BYTES`] × 2).
const ISOLATION_KEY_SWAP_PREFIX: &str = "swap/id/";

/// Width of the per-swap isolation token — 16 bytes (128 bits) of an
/// HMAC-SHA256 tag, hex-encoded. Uniqueness across the swaps one process
/// polls is all the circuit split needs; the full tag would only lengthen a
/// key the host copies into its isolation table.
const SWAP_ISOLATION_TOKEN_BYTES: usize = 16;

/// Width of the per-instance HMAC secret — the SHA-256 block-sized key
/// ring's `HMAC_SHA256` uses without padding or hashing it down.
const SWAP_ISOLATION_SECRET_BYTES: usize = 32;

/// Mints the per-swap circuit-isolation key from a [`SwapId`] WITHOUT letting
/// any byte of it reach the dialer (FR-29 stage 0 (i); ADR-0544 decision 2).
///
/// Pre-S8 a `SwapId` WAS the provider's deposit address — a §5.4 never-log
/// item that the provider CHOSE. Passing it verbatim as the `isolation_key`
/// (the pre-fix shape) copied it into every transport's isolation table (the
/// host retains hundreds of live keys), and let a hostile provider pick the
/// KEY: a `depositAddress` equal to another purpose's label rides that
/// purpose's circuit; one address for two swaps collapses per-swap
/// unlinkability. So the key namespace is SDK-owned: `HMAC-SHA256(secret,
/// swap_id)` truncated and hex-encoded, under a secret minted from the OS
/// CSPRNG once per adapter instance. Since S8 the id is itself SDK-minted and
/// carries no address bytes, and two swaps a provider put under ONE deposit
/// address have two ids ⇒ two keys ⇒ two circuits; the HMAC stays so the key
/// keeps its fixed width and stays unlinkable across instances.
///
/// Properties the tests pin: the same swap maps to the same key for the
/// adapter's life (its deposit notification and every status poll share ONE
/// circuit, as before); distinct swaps map to distinct keys; the key is fixed
/// width and carries no substring of the address; a second adapter instance
/// (another process, a relaunch) maps the same swap to a different key, so
/// keys are unlinkable across instances too. The secret protects
/// unlinkability of an address the provider already knows — never funds; it
/// lives inside ring's `hmac::Key` for the adapter's life.
struct SwapIsolationMinter {
    /// The per-instance secret, `Zeroizing` for its whole life (crypto-rules:
    /// every secret zeroizes). ring's `hmac::Key` is built PER CALL from it
    /// and dropped with the call (the stage 0 crypto audit's LOW: ring's
    /// `Key` neither zeroizes nor forbids `Clone`, so it never lives on the
    /// struct); two SHA-256 compressions per poll is nothing beside the
    /// round trip.
    secret: Zeroizing<[u8; SWAP_ISOLATION_SECRET_BYTES]>,
}

impl SwapIsolationMinter {
    fn new() -> Result<Self, SwapError> {
        let mut secret = Zeroizing::new([0u8; SWAP_ISOLATION_SECRET_BYTES]);
        // A CSPRNG that refuses is a platform fault; the adapter then cannot
        // mint unlinkable keys, so it does not come up. `ProviderUnavailable`
        // is the honest word on purpose (the stage 0 arch review asked): the
        // variant's contract is "retryable", and a retry here IS the right
        // remedy — re-enabling swap re-constructs the adapter and re-draws
        // from the OS random source — while the user-facing sentence ("swap
        // is unavailable right now") is exactly true. `SwapError` is mirrored
        // across the bridge, so a new local-fault variant for a condition no
        // device has produced is beyond "just enough".
        ring::rand::SystemRandom::new()
            .fill(secret.as_mut())
            .map_err(|_| SwapError::ProviderUnavailable)?;
        Ok(Self { secret })
    }

    /// The isolation key for `id`: `swap/id/` + 32 lowercase hex characters.
    fn key_for(&self, id: &SwapId) -> String {
        use core::fmt::Write as _;
        let key = hmac::Key::new(hmac::HMAC_SHA256, self.secret.as_ref());
        let tag = hmac::sign(&key, id.as_str().as_bytes());
        let mut out =
            String::with_capacity(ISOLATION_KEY_SWAP_PREFIX.len() + 2 * SWAP_ISOLATION_TOKEN_BYTES);
        out.push_str(ISOLATION_KEY_SWAP_PREFIX);
        for byte in &tag.as_ref()[..SWAP_ISOLATION_TOKEN_BYTES] {
            // Writing into a `String` cannot fail; the `let _` keeps the
            // must-use honest without an `expect` in production code.
            let _ = write!(out, "{byte:02x}");
        }
        out
    }
}

/// Dedicated isolation bucket for the LAZY token-list fetch (§3.3b D5 / ADR-0530, IZ-2): its OWN
/// circuit, distinct from sync AND from quote/status — so the relay cannot link "this circuit lists
/// tokens AND syncs the wallet / quotes a swap" (§2.3). `tor_required_covers_list_tokens` pins the
/// `TorPolicy::Required` fail-closed guarantee on this path.
const ISOLATION_KEY_TOKENS: &str = "swap/tokens";

/// Host-supplied provider config (spec §3.2). The JWT is the integrator
/// credential — OPTIONAL (without it the provider levies its 0.2% fee),
/// never hardcoded, never logged; `Zeroizing` so the secret is wiped when
/// the config/provider drops.
pub struct SwapProviderConfig {
    /// `https://host[:port]` — https is MANDATORY (no plaintext path exists
    /// in this crate); paths/queries/userinfo are rejected typed.
    pub endpoint: String,
    pub jwt: Option<Zeroizing<String>>,
}

impl Default for SwapProviderConfig {
    fn default() -> Self {
        Self {
            endpoint: DEFAULT_ENDPOINT.to_owned(),
            jwt: None,
        }
    }
}

impl SwapProviderConfig {
    /// Build a config from raw host-supplied parts — the FFI-boundary constructor the
    /// bridge's `enableNearSwap` lowers the Dart DTO through. The `jwt` secret is wrapped
    /// in its `Zeroizing` buffer HERE, in the crate that OWNS the secret-bearing field
    /// type, so the caller never names `zeroize` (and the FRB bridge needs no `zeroize`
    /// dependency — keeping its `swap-near` feature to exactly the adapter). `None` jwt ⇒
    /// the provider levies its 0.2% fee (the field doc).
    pub fn from_parts(endpoint: String, jwt: Option<String>) -> Result<Self, SwapError> {
        // VALIDATE HERE, NOT AT EVERY REQUEST. This used to
        // wrap the secret and return, with the only check being
        // `HeaderValue::from_str` inside `exchange_on` — per call, on the network
        // path. So a JWT carrying a stray trailing newline (invisible in an
        // editor, and easy to acquire from a file or a `--dart-define`) failed
        // EVERY swap request with a generic `RequestInvalid`, forever, instead of
        // once at `enableNearSwap` where the host could act on it. And the reason
        // is unreadable in any log, because the value is correctly redacted
        // everywhere.
        //
        // Same class as the `-bin` header-name defect the endpoint-auth work hit
        // in the sibling adapter this session — a credential that only fails on
        // the network fails in front of a user. Kept in step with
        // `EndpointAuth::new` (`zec-wallet-core/src/config.rs`) deliberately:
        // CR/LF/NUL is the injection check, surrounding whitespace is the
        // fat-finger, and an empty string is not a credential.
        if let Some(j) = jwt.as_deref() {
            const BAD: SwapError = SwapError::RequestInvalid {
                reason: "swap JWT must be non-empty, carry no CR/LF/NUL, and have no leading or trailing whitespace",
            };
            if j.is_empty() || j.trim() != j {
                return Err(BAD);
            }
            // The one place a non-`Zeroizing` copy exists, and it lives for one
            // statement: `from_str` is what rejects CR/LF/NUL.
            http::HeaderValue::from_str(j).map_err(|_| BAD)?;
        }
        Ok(Self {
            endpoint,
            jwt: jwt.map(Zeroizing::new),
        })
    }
}

/// `https://host[:port]` → (host, port). Hostnames/IPv4 only (the provider
/// endpoint is a DNS name; bracketed IPv6 is not supported — a host that
/// needs it can front it with a name).
fn parse_endpoint(endpoint: &str) -> Result<(String, u16), SwapError> {
    const INVALID: SwapError = SwapError::RequestInvalid {
        reason: "swap endpoint must be https://host[:port] (no path/query/userinfo)",
    };
    let rest = endpoint.strip_prefix("https://").ok_or(INVALID)?;
    let rest = rest.strip_suffix('/').unwrap_or(rest);
    if rest.is_empty() || rest.contains(['/', '?', '#', '@']) {
        return Err(INVALID);
    }
    let (host, port) = match rest.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().map_err(|_| INVALID)?),
        None => (rest, 443),
    };
    if host.is_empty() || port == 0 {
        return Err(INVALID);
    }
    // fail at construction, not first dial: the host must be a valid TLS
    // server name (rustls will need one on every connection)
    rustls::pki_types::ServerName::try_from(host.to_owned()).map_err(|_| INVALID)?;
    Ok((host.to_owned(), port))
}

pub struct NearIntentsProvider {
    http: HttpClient,
    /// Token table cache (asset-id resolution + decimals): assets and their
    /// scales are stable; PRICES in it are display-only snapshots. Filled on
    /// first use; [`Self::supported_tokens`] always refreshes it.
    tokens: tokio::sync::Mutex<Option<Arc<Vec<SwapAsset>>>>,
    /// The per-swap isolation-key minter (one secret per adapter instance).
    isolation: SwapIsolationMinter,
}

impl NearIntentsProvider {
    /// The ONLY constructor — TLS over the injected dialer, unconditionally.
    pub fn new(config: SwapProviderConfig, dialer: Arc<dyn NetDialer>) -> Result<Self, SwapError> {
        Self::with_securer(config, dialer, Arc::new(RustlsSecurer::new()))
    }

    /// Test seam (pub(crate) — no public plaintext path exists).
    pub(crate) fn with_securer(
        config: SwapProviderConfig,
        dialer: Arc<dyn NetDialer>,
        securer: Arc<dyn http_client::StreamSecurer>,
    ) -> Result<Self, SwapError> {
        let (host, port) = parse_endpoint(&config.endpoint)?;
        Ok(Self {
            http: HttpClient::new(dialer, securer, host, port, config.jwt),
            tokens: tokio::sync::Mutex::new(None),
            isolation: SwapIsolationMinter::new()?,
        })
    }

    /// The provider's FULL asset table, fresh from `GET /v0/tokens`; also refreshes the resolution
    /// cache. The host-facing picker is the PORT [`SwapPort::list_tokens`], which FILTERS this table
    /// (drops ZEC + $0) and serves stale on a fault; this returns the UNFILTERED `Vec<SwapAsset>`
    /// (ZEC included) for the wire-contract tests + the raw-table primitive. (Tightening to
    /// crate-internal is a noted follow-up — see the `SwapAsset` re-export note above.)
    #[tracing::instrument(skip_all)]
    pub async fn supported_tokens(&self) -> Result<Vec<SwapAsset>, SwapError> {
        let table = self.fetch_tokens(ISOLATION_KEY_TOKENS).await?;
        *self.tokens.lock().await = Some(Arc::clone(&table));
        Ok(table.to_vec())
    }

    /// `POST /v0/deposit/submit` — tell the provider about a broadcast
    /// deposit tx (speeds detection; the provider also watches the chain).
    /// Port wiring lands with the send pipeline; typed + fixture-pinned now.
    /// `id` picks the swap's circuit, `provider_ref` (the deposit address) is
    /// the body's handle — the same split as [`SwapPort::status`].
    #[tracing::instrument(skip_all)]
    pub async fn notify_deposit_tx(
        &self,
        id: &SwapId,
        provider_ref: &str,
        tx_hash: &str,
    ) -> Result<SwapStatus, SwapError> {
        if provider_ref.is_empty()
            || provider_ref.len() > PROVIDER_STR_MAX_BYTES
            || tx_hash.len() > PROVIDER_STR_MAX_BYTES
        {
            return Err(SwapError::RequestInvalid {
                reason: "deposit-notification fields exceed the provider string bound",
            });
        }
        let body = serde_json::to_vec(&wire::WireSubmitRequest {
            tx_hash: tx_hash.to_owned(),
            deposit_address: provider_ref.to_owned(),
        })
        .map_err(|_| SwapError::RequestInvalid {
            reason: "could not encode the deposit notification",
        })?;
        let outcome = self
            .http
            .exchange(
                "deposit_submit",
                Method::POST,
                "/v0/deposit/submit",
                Some(body),
                // the swap's OWN circuit (shared with its status polls) — an
                // SDK-minted token, never the deposit address itself
                Some(&self.isolation.key_for(id)),
                STATUS_RESPONSE_MAX_BYTES,
            )
            .await?;
        let body = self.classify_outcome(outcome, /* not_found_is_swap: */ true)?;
        let table = self.tokens_cached().await?;
        map::map_status(wire::decode_status_response(&body)?, |asset_id| {
            table
                .iter()
                .find(|t| t.provider_asset_id == asset_id)
                .map(|t| t.decimals)
        })
    }

    /// `GET /v0/tokens` → the full provider table. `isolation_key` is the caller's circuit bucket:
    /// the quote/status RESOLUTION fill rides `ISOLATION_KEY_QUOTE` (it is part of quoting), while the
    /// picker fetch ([`Self::supported_tokens`] / [`SwapPort::list_tokens`]) rides the dedicated
    /// `ISOLATION_KEY_TOKENS` (§3.3b D5 — token-listing is unlinkable from sync AND from quoting).
    async fn fetch_tokens(&self, isolation_key: &str) -> Result<Arc<Vec<SwapAsset>>, SwapError> {
        let outcome = self
            .http
            .exchange(
                "tokens",
                Method::GET,
                "/v0/tokens",
                None,
                Some(isolation_key),
                TOKENS_RESPONSE_MAX_BYTES,
            )
            .await?;
        let body = self.classify_outcome(outcome, false)?;
        Ok(Arc::new(map::map_tokens(wire::decode_tokens_response(
            &body,
        )?)?))
    }

    /// Filter the full provider table to the IntoZec picker list (§3.3b D5): drop the ZEC asset
    /// itself (never a foreign source/destination pick — ZEC is the wallet's own side) and any
    /// `$0`/null-price entry (display price is the provider's tradeability signal; a priceless or
    /// zero-priced listing clutters the picker and is the spec's drop rule), then map to the
    /// port-level [`TokenInfo`]. Price stays DISPLAY-ONLY — this is a UX/quality filter, NEVER funds
    /// math (the resolution cache keeps the FULL table, ZEC included, for quote-time `resolve`).
    ///
    /// NO `(chain,symbol)` dedup (unlike ZODL): an exact-duplicate pair is UNQUOTABLE regardless —
    /// `resolve` requires an exact-UNIQUE match and rejects ambiguity typed at quote time, and it
    /// reads the FULL cached table (deduping the picker to one row would not make the quote
    /// resolvable). Showing both is honest + fail-closed (the provider does not currently return
    /// duplicates). The `> 0.0` price predicate is panic-free AND NaN-safe: `map_tokens` already
    /// stripped non-finite prices (`is_finite`) at the boundary, and `NaN > 0.0` is `false` anyway.
    fn pick_tokens(table: &[SwapAsset]) -> Vec<TokenInfo> {
        table
            .iter()
            .filter(|a| !(a.chain == ZEC_CHAIN_ID && a.symbol == ZEC_SYMBOL))
            .filter(|a| a.price_usd.is_some_and(|p| p > 0.0))
            .map(|a| TokenInfo {
                chain: a.chain.clone(),
                symbol: a.symbol.clone(),
                decimals: a.decimals,
                provider_asset_id: a.provider_asset_id.clone(),
                price_usd: a.price_usd,
            })
            .collect()
    }

    /// Cached table, fetched on first use. The lock is never held across
    /// the fetch await; a benign double-fetch race yields two valid tables.
    async fn tokens_cached(&self) -> Result<Arc<Vec<SwapAsset>>, SwapError> {
        // Explicit scope so the guard PROVABLY drops before the `.await` below — the tokio
        // `MutexGuard` is `Send`, so a lock held across the fetch await would NOT be a compile error;
        // the brace makes the lock-then-drop-then-fetch shape structural, not incidental (arch review
        // IZ-2 — survives a future `if let`→`match` refactor).
        {
            let guard = self.tokens.lock().await;
            if let Some(t) = guard.as_ref() {
                return Ok(Arc::clone(t));
            }
        }
        let table = self.fetch_tokens(ISOLATION_KEY_QUOTE).await?;
        *self.tokens.lock().await = Some(Arc::clone(&table));
        Ok(table)
    }

    /// (chain, symbol) → provider asset, EXACT and UNIQUE: zero matches and
    /// ambiguity both fail typed — guessing which "USDC" someone meant is
    /// not a funds-path option.
    fn resolve<'t>(
        table: &'t [SwapAsset],
        chain: &str,
        symbol: &str,
    ) -> Result<&'t SwapAsset, SwapError> {
        let mut matches = table
            .iter()
            .filter(|t| t.chain == chain && t.symbol == symbol);
        let first = matches.next().ok_or(SwapError::RequestInvalid {
            reason: "asset not in the provider's supported-token list",
        })?;
        if matches.next().is_some() {
            return Err(SwapError::RequestInvalid {
                reason: "ambiguous asset: chain+symbol matches several provider listings",
            });
        }
        Ok(first)
    }

    fn classify_outcome(
        &self,
        outcome: HttpOutcome,
        not_found_is_swap: bool,
    ) -> Result<bytes::Bytes, SwapError> {
        match outcome {
            HttpOutcome::Success(body) => Ok(body),
            HttpOutcome::ClientError(404) if not_found_is_swap => {
                Err(SwapError::ProviderProtocol {
                    reason: ProviderProtocolReason::SwapNotFound,
                })
            }
            // any other 4xx: the service pre-validated the request, so this
            // is contract drift, not a user error (body never read — §5.4)
            HttpOutcome::ClientError(_) => Err(SwapError::ProviderProtocol {
                reason: ProviderProtocolReason::UnexpectedHttpStatus,
            }),
        }
    }

    fn now_unix() -> Result<u64, SwapError> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .map_err(|_| SwapError::RequestInvalid {
                reason: "device clock is before the unix epoch",
            })
    }
}

#[async_trait]
impl SwapPort for NearIntentsProvider {
    fn name(&self) -> &'static str {
        "near-intents"
    }

    #[tracing::instrument(skip_all)]
    async fn quote(&self, req: QuoteRequest) -> Result<SwapQuote, SwapError> {
        let table = self.tokens_cached().await?;
        // ZEC is the ORIGIN for OutOfZec (we send it) and the DESTINATION for IntoZec (we
        // receive it); its decimals MUST be 8 (base units = zatoshis) — a drifted listing is a
        // protocol violation, not something to adapt to silently.
        let zec = Self::resolve(&table, ZEC_CHAIN_ID, ZEC_SYMBOL)?;
        if zec.decimals != ZEC_DECIMALS {
            return Err(SwapError::ProviderProtocol {
                reason: ProviderProtocolReason::MalformedAmount,
            });
        }
        // Resolve (origin, dest) per direction: the user's foreign asset is the non-ZEC side
        // (OutOfZec destination `to`, IntoZec origin `from`). An unknown direction is a typed
        // refusal (the closed-enum rule — never a guess, never a panic on `#[non_exhaustive]`).
        let (origin, dest, into_zec) = match &req.direction {
            SwapDirection::OutOfZec { to } => {
                (zec, Self::resolve(&table, &to.chain, &to.symbol)?, false)
            }
            SwapDirection::IntoZec { from } => {
                (Self::resolve(&table, &from.chain, &from.symbol)?, zec, true)
            }
            _ => {
                return Err(SwapError::RequestInvalid {
                    reason: "swap direction unknown to this adapter",
                });
            }
        };
        let now = Self::now_unix()?;
        // The requested deposit window is DIRECTION-DEPENDENT (§4.4 W-swap-4-a-3): an
        // IntoZec deposit is an EXTERNAL user send (another wallet, an exchange
        // withdrawal) and keeps the ecosystem-standard 24 h; an OutOfZec deposit is
        // sent by THIS wallet within seconds of execute, so it asks for 15 min — the
        // bound every §4.4 deposit-tag lockout (in-flight guard, sign-miss zombie,
        // under-funded Stale) self-clears on. Requesting 24 h both ways re-priced all
        // of those to a day.
        let wire_req = if into_zec {
            // IntoZec (IZ-1): `recipient` is OUR fresh engine-persisted ZEC destination (the
            // service minted it), `refund_to` the user's source-chain refund (§3.3b D1/D6).
            map::build_into_zec_request(&req, origin, dest, now, SWAP_DEADLINE_DEFAULT_SECS)?
        } else {
            map::build_out_of_zec_request(
                &req,
                origin,
                dest,
                now,
                SWAP_DEPOSIT_DEADLINE_OUT_OF_ZEC_SECS,
            )?
        };
        let body = serde_json::to_vec(&wire_req).map_err(|_| SwapError::RequestInvalid {
            reason: "could not encode the quote request",
        })?;
        let outcome = self
            .http
            .exchange(
                "quote",
                Method::POST,
                "/v0/quote",
                Some(body),
                Some(ISOLATION_KEY_QUOTE),
                QUOTE_RESPONSE_MAX_BYTES,
            )
            .await?;
        let body = self.classify_outcome(outcome, false)?;
        let response = wire::decode_quote_response(&body)?;
        // The provider's echo must state the terms we sent (external review
        // 2026-10-07, finding 3) — checked before the response is mapped at all.
        map::check_request_echo(&wire_req, &response.quote_request)?;
        if into_zec {
            map::map_into_zec_quote(response, &req.direction, origin, dest)
        } else {
            map::map_out_of_zec_quote(response, &req.direction, origin, dest)
        }
    }

    /// LOCAL commit point: registration already happened at [`Self::quote`]
    /// (`dry: false` — see the module note); the deadline gates live in
    /// `SwapService`. Mirrors the `MockSwapProvider` contract.
    async fn execute(&self, quote: &SwapQuote) -> Result<(), SwapError> {
        if quote.deposit_address.is_empty() {
            return Err(SwapError::RequestInvalid {
                reason: "quote carries no deposit address",
            });
        }
        Ok(())
    }

    /// `provider_ref` — the deposit address the service resolved from the home
    /// row — is the request's `depositAddress`; `id` only picks the circuit.
    #[tracing::instrument(skip_all)]
    async fn status(&self, id: &SwapId, provider_ref: &str) -> Result<SwapStatus, SwapError> {
        if provider_ref.is_empty() || provider_ref.len() > PROVIDER_STR_MAX_BYTES {
            return Err(SwapError::RequestInvalid {
                reason: "swap provider handle is empty or exceeds the provider string bound",
            });
        }
        let path = format!(
            "/v0/status?depositAddress={}",
            utf8_percent_encode(provider_ref, NON_ALPHANUMERIC)
        );
        let outcome = self
            .http
            .exchange(
                "status",
                Method::GET,
                &path,
                None,
                // per-swap circuit isolation: polls for different swaps are
                // unlinkable at the network layer (§2.3) — through the SDK-minted
                // token, so no byte of the deposit address reaches the dialer
                Some(&self.isolation.key_for(id)),
                STATUS_RESPONSE_MAX_BYTES,
            )
            .await?;
        let body = self.classify_outcome(outcome, true)?;
        let table = self.tokens_cached().await?;
        map::map_status(wire::decode_status_response(&body)?, |asset_id| {
            table
                .iter()
                .find(|t| t.provider_asset_id == asset_id)
                .map(|t| t.decimals)
        })
    }

    /// The IntoZec picker list (§3.3b D5/L6 / ADR-0530, IZ-2): a fresh `/v0/tokens` fetch on its
    /// DEDICATED circuit, filtered to quotable foreign assets and mapped to [`TokenInfo`]. On a fetch
    /// FAULT (timeout / unreachable — the unstable-network case) it serves the LAST good table from
    /// cache with `fresh = false` (the host's "couldn't refresh, showing cached" banner — honest
    /// degradation, §6/L6); only a fault with NO cached table yet is the typed error (a first-ever
    /// open offline). A successful fetch refreshes the shared resolution cache too (so a subsequent
    /// quote reuses it). The `tokens` lock is never held across the fetch await (the `tokens_cached`
    /// precedent) — a benign double-fetch yields two valid tables.
    ///
    /// Note: `map_tokens` is STRICT — a single malformed entry (oversized field / non-integral
    /// decimals) faults the WHOLE fetch (no silent asset-dropping on a funds-adjacent table). On the
    /// picker path that fault is cushioned by the serve-stale cache; only a cold first-open hits the
    /// typed error (an honest "couldn't load," never a partial list).
    #[tracing::instrument(skip_all)]
    async fn list_tokens(&self) -> Result<TokenList, SwapError> {
        match self.fetch_tokens(ISOLATION_KEY_TOKENS).await {
            Ok(table) => {
                *self.tokens.lock().await = Some(Arc::clone(&table));
                Ok(TokenList {
                    tokens: Self::pick_tokens(&table),
                    fresh: true,
                })
            }
            // Fetch fault: serve the last good list stale (L6) if we have one; else surface the typed
            // error (nothing cached to fall back to — an honest "couldn't load" for the first open).
            Err(e) => match self.tokens.lock().await.as_ref() {
                Some(table) => Ok(TokenList {
                    tokens: Self::pick_tokens(table),
                    fresh: false,
                }),
                None => Err(e),
            },
        }
    }
}

/// Fuzz seam (the spec §8 `fuzz_swap_status_decode` harness) — NOT public
/// API. Drives the WHOLE hostile-response funnel: bytes → serde decode →
/// bounds → port mapping, for all three response families.
#[doc(hidden)]
pub mod fuzzing {
    use crate::{map, wire};

    fn assets() -> (map::SwapAsset, map::SwapAsset) {
        (
            map::SwapAsset {
                chain: "zec".into(),
                symbol: "ZEC".into(),
                decimals: 8,
                provider_asset_id: "nep141:zec.omft.near".into(),
                price_usd: None,
            },
            map::SwapAsset {
                chain: "near".into(),
                symbol: "USDC".into(),
                decimals: 6,
                provider_asset_id: "nep141:usdc.example".into(),
                price_usd: None,
            },
        )
    }

    pub fn decode_and_map_all(body: &[u8]) {
        if let Ok(resp) = wire::decode_status_response(body) {
            let _ = map::map_status(resp, |_| Some(8));
        }
        if let Ok(resp) = wire::decode_quote_response(body) {
            let (zec, usdc) = assets();
            let direction = zec_wallet_core::SwapDirection::OutOfZec {
                to: zec_wallet_core::AssetId {
                    chain: usdc.chain.clone(),
                    symbol: usdc.symbol.clone(),
                },
            };
            let _ = map::map_out_of_zec_quote(resp, &direction, &zec, &usdc);
        }
        if let Ok(tokens) = wire::decode_tokens_response(body)
            && let Ok(table) = map::map_tokens(tokens)
        {
            // Also drive the IZ-2 picker filter over the hostile-derived table — the whole
            // decode→map→filter chain must never panic (testing-patterns §3 parser-boundary rule).
            let _ = crate::NearIntentsProvider::pick_tokens(&table);
        }
    }
}

#[cfg(test)]
mod endpoint_tests {
    use super::{SwapProviderConfig, parse_endpoint};

    #[test]
    fn swap_jwt_is_validated_at_construction_not_at_every_request() {
        // arch review. This used to wrap the secret and return `Self` with
        // no checks; the only validation was `HeaderValue::from_str` inside
        // `exchange_on`, per request, on the network path. So a JWT with a stray
        // trailing newline failed EVERY swap forever with a generic
        // `RequestInvalid`, and the cause was unreadable in any log because the
        // value is (correctly) redacted everywhere.
        let ok = "eyJhbGciOiJSUzI1NiJ9.body.sig";
        assert!(
            SwapProviderConfig::from_parts("https://1click.example".into(), Some(ok.into()))
                .is_ok()
        );
        assert!(
            SwapProviderConfig::from_parts("https://1click.example".into(), None).is_ok(),
            "no JWT is a supported configuration — the provider just levies its fee"
        );
        for bad in ["", " tok", "tok\n", "tok\r\nX: 1", "tok\0", "tok "] {
            assert!(
                SwapProviderConfig::from_parts(
                    "https://1click.example".into(),
                    Some(bad.to_string())
                )
                .is_err(),
                "a malformed JWT must be refused at construction: {bad:?}"
            );
        }
    }

    #[test]
    fn endpoint_parse_is_strict_https_host_port() {
        assert_eq!(
            parse_endpoint("https://1click.chaindefuser.com").unwrap(),
            ("1click.chaindefuser.com".to_owned(), 443)
        );
        assert_eq!(
            parse_endpoint("https://staging.example.com:8443/").unwrap(),
            ("staging.example.com".to_owned(), 8443)
        );
        for bad in [
            "http://1click.chaindefuser.com", // plaintext: no such path exists
            "https://",
            "https://host/path",
            "https://host?x=1",
            "https://user@host",
            "https://host:0",
            "https://host:not-a-port",
            "1click.chaindefuser.com",
            "",
        ] {
            assert!(parse_endpoint(bad).is_err(), "must reject {bad:?}");
        }
    }
}
