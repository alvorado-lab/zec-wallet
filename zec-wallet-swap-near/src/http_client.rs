//! One-shot HTTPS over the SDK's [`NetDialer`] — the m5 transport contract
//! (spec §3.2): swap traffic rides the SAME dialer seam as sync, so
//! `TorPolicy::Required` fail-closes swap calls.
//!
//! **How "by construction" is actually enforced, corrected at T0-5.** This doc
//! used to say the crate "cannot open a socket on its own (tokio has no `net`
//! feature here)", and that was false: cargo unifies features across the
//! workspace, `zcash_client_backend`'s `lightwalletd-tonic-transport` turns
//! `tokio/net` on for the ONE tokio in the graph, and this crate links against
//! that tokio. MEASURED with `cargo tree -e features -i tokio`. What enforces
//! the contract is therefore not the manifest but
//! `tests/policy.rs::adapter_cannot_open_its_own_sockets`, which reads this
//! crate's own sources and refuses any socket API in them — and which has a
//! second row proving that scan can fail.
//!
//! ONE CONNECTION PER REQUEST, deliberately: a pooled client keys
//! connections by host and would silently REUSE a circuit across distinct
//! `isolation_key`s — exactly the linkage the dialer contract forbids. The
//! cost (a TLS handshake per call) is noise against Tor latency and the
//! §7 poll cadence. NO TLS SESSION RESUMPTION, for the same reason one layer
//! down: rustls's default client store keeps sessions per server name, so a
//! ticket from one swap's connection would be offered on another swap's
//! circuit and the provider could link the two.
//!
//! Redirects are never followed: on a funds path, "the provider moved" is a
//! typed protocol violation to surface, not a hop to take silently.

use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HOST};
use http::{Method, Request};
use http_body_util::{BodyExt, Full, Limited};
use hyper_util::rt::TokioIo;
use rustls::pki_types::ServerName;
use tokio_rustls::TlsConnector;
use zec_wallet_core::{AsyncByteStream, NetDialer, ProviderProtocolReason, SwapError};
use zeroize::Zeroizing;

/// `GET /v0/tokens` body cap. Recorded reality: 43 KB at 188 assets
/// (2026-06-13); 1 MiB ≈ 20× headroom before we'd rather fail loudly than
/// buffer a hostile list.
pub(crate) const TOKENS_RESPONSE_MAX_BYTES: usize = 1024 * 1024;
/// Quote response cap. Recorded reality ≈ 1.5 KB; 64 KiB is generous.
pub(crate) const QUOTE_RESPONSE_MAX_BYTES: usize = 64 * 1024;
/// Status/submit response cap: embeds the quote echo + tx-hash arrays.
pub(crate) const STATUS_RESPONSE_MAX_BYTES: usize = 256 * 1024;
/// Whole-call budget: dial + TLS handshake + exchange (request → last body
/// byte) under ONE timeout. Sized for Tor circuit latency (multi-second
/// dials are normal); self-contained so the §3.2 resume poller can never
/// hang on a blackholed connect or a half-open stream — the guarantee must
/// not rest on an unstated dialer-internal timeout (review fold; the
/// `NetDialer` contract mandates isolation, not timing).
pub(crate) const SWAP_HTTP_TIMEOUT_SECS: u64 = 30;

/// TLS layering seam. The ONLY production impl is [`RustlsSecurer`]
/// (constructed unconditionally by `NearIntentsProvider::new` — there is no
/// public plaintext path); unit tests substitute a passthrough to script
/// HTTP bytes over an in-memory duplex.
#[async_trait]
pub(crate) trait StreamSecurer: Send + Sync {
    async fn secure(
        &self,
        host: &str,
        stream: Box<dyn AsyncByteStream>,
    ) -> Result<Box<dyn AsyncByteStream>, SwapError>;
}

/// rustls over the dialed stream: WebPKI roots (Mozilla bundle — no
/// platform cert-store assumption on mobile), `ring` provider (the
/// workspace's pinned TLS stack), ALPN locked to http/1.1, no session
/// resumption.
pub(crate) struct RustlsSecurer {
    connector: TlsConnector,
}

impl RustlsSecurer {
    pub(crate) fn new() -> Self {
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let mut config = rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            // truly unreachable (the rust-patterns expect exception): this
            // errs only for a provider supporting NO default TLS version;
            // ring ships 1.2 + 1.3, both pinned on via the workspace features
            .expect("ring provider supports the default TLS versions")
            .with_root_certificates(roots)
            .with_no_client_auth();
        // deterministic protocol choice — we speak http/1.1, never h2
        config.alpn_protocols = vec![b"http/1.1".to_vec()];
        // one securer serves every isolation key: a stored session would link
        // swaps across circuits (the module doc; arti disables it likewise)
        config.resumption = rustls::client::Resumption::disabled();
        Self {
            connector: TlsConnector::from(Arc::new(config)),
        }
    }
}

#[async_trait]
impl StreamSecurer for RustlsSecurer {
    async fn secure(
        &self,
        host: &str,
        stream: Box<dyn AsyncByteStream>,
    ) -> Result<Box<dyn AsyncByteStream>, SwapError> {
        let name = ServerName::try_from(host.to_owned()).map_err(|_| {
            // host is validated at construction; defensive
            SwapError::RequestInvalid {
                reason: "endpoint host is not a valid TLS server name",
            }
        })?;
        let tls = self.connector.connect(name, stream).await.map_err(|_| {
            // handshake/cert failure: could be a hostile middlebox or a
            // network flake — either way nothing was trusted; visible
            // (category only, §5.4) and retryable upstream
            tracing::warn!(target: "zec_wallet_swap_near", "swap tls handshake failed");
            SwapError::ProviderUnavailable
        })?;
        Ok(Box::new(tls))
    }
}

/// What one exchange produced: a SUCCESS body, or the typed non-success
/// status (body deliberately NOT read — provider error bodies echo request
/// data such as addresses, §5.4 never-log, and we never branch on them).
pub(crate) enum HttpOutcome {
    Success(Bytes),
    /// 4xx, surfaced for endpoint-specific mapping (404 on /v0/status =
    /// `SwapNotFound`; everything else = `UnexpectedHttpStatus`).
    ClientError(u16),
}

pub(crate) struct HttpClient {
    dialer: Arc<dyn NetDialer>,
    securer: Arc<dyn StreamSecurer>,
    host: String,
    port: u16,
    /// Prebuilt `Bearer …` value; `Zeroizing` end-to-end in OUR memory (the
    /// serialized request buffer is hyper's — the credential necessarily
    /// goes on the wire; never in logs, header marked sensitive).
    authorization: Option<Zeroizing<String>>,
}

impl HttpClient {
    pub(crate) fn new(
        dialer: Arc<dyn NetDialer>,
        securer: Arc<dyn StreamSecurer>,
        host: String,
        port: u16,
        jwt: Option<Zeroizing<String>>,
    ) -> Self {
        Self {
            dialer,
            securer,
            host,
            port,
            authorization: jwt.map(|j| Zeroizing::new(format!("Bearer {}", j.as_str()))),
        }
    }

    /// One request, one connection. `endpoint` is a STATIC label for
    /// tracing only (`path_and_query` can carry a deposit address — never
    /// logged, §5.4).
    pub(crate) async fn exchange(
        &self,
        endpoint: &'static str,
        method: Method,
        path_and_query: &str,
        body: Option<Vec<u8>>,
        isolation_key: Option<&str>,
        max_response_bytes: usize,
    ) -> Result<HttpOutcome, SwapError> {
        // dial + TLS handshake + the whole HTTP exchange share one hard
        // budget — neither a blackholed connect nor a half-open hostile
        // stream can hang the caller
        let fut = async {
            let stream = self
                .dialer
                .dial(&self.host, self.port, isolation_key)
                .await
                .map_err(|e| {
                    // category only: Unreachable/Timeout/Unsupported/Io — the
                    // honest-off story (TorPolicy::Required fail-closed) lands
                    // here as zero packets + a typed, logged error
                    tracing::warn!(target: "zec_wallet_swap_near", endpoint, error = %e, "swap dial failed");
                    SwapError::ProviderUnavailable
                })?;
            let stream = self.securer.secure(&self.host, stream).await?;
            self.exchange_on(stream, method, path_and_query, body, max_response_bytes)
                .await
        };
        match tokio::time::timeout(std::time::Duration::from_secs(SWAP_HTTP_TIMEOUT_SECS), fut)
            .await
        {
            Ok(r) => r,
            Err(_) => {
                tracing::warn!(target: "zec_wallet_swap_near", endpoint, "swap http exchange timed out");
                Err(SwapError::ProviderUnavailable)
            }
        }
    }

    async fn exchange_on(
        &self,
        stream: Box<dyn AsyncByteStream>,
        method: Method,
        path_and_query: &str,
        body: Option<Vec<u8>>,
        max_response_bytes: usize,
    ) -> Result<HttpOutcome, SwapError> {
        let (mut sender, conn) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
            .await
            .map_err(|_| SwapError::ProviderUnavailable)?;
        // drive the connection alongside the request; one-shot, so the task
        // ends with the exchange (sender drop closes the connection)
        let conn_task = tokio::spawn(conn);

        let mut builder = Request::builder()
            .method(method)
            .uri(path_and_query)
            .header(HOST, &self.host)
            .header(ACCEPT, "application/json");
        if body.is_some() {
            builder = builder.header(CONTENT_TYPE, "application/json");
        }
        if let Some(auth) = &self.authorization {
            let mut value = http::HeaderValue::from_str(auth.as_str()).map_err(|_| {
                SwapError::RequestInvalid {
                    reason: "JWT contains bytes not valid in an HTTP header",
                }
            })?;
            value.set_sensitive(true); // never Debug-printed by http/hyper
            builder = builder.header(AUTHORIZATION, value);
        }
        let request = builder
            .body(Full::new(Bytes::from(body.unwrap_or_default())))
            .map_err(|_| SwapError::RequestInvalid {
                reason: "could not build the provider request",
            })?;

        let response = sender
            .send_request(request)
            .await
            .map_err(|_| SwapError::ProviderUnavailable)?;
        // classification (status → typed outcome, body read under the cap) is
        // its own function so the connection setup above stays legible.
        let outcome = classify_response(response, max_response_bytes).await;
        conn_task.abort();
        outcome
    }
}

/// Map a received HTTP response to a typed [`HttpOutcome`] (§4.6):
/// - 2xx ⇒ body collected under a HARD cap (Content-Length pre-check, then a
///   streaming `Limited` cap — size-cap-BEFORE-allocating);
/// - 3xx ⇒ typed refusal (`UnexpectedHttpStatus`): a funds path NEVER silently
///   follows a redirect to another host;
/// - 4xx ⇒ `ClientError(status)` for the caller to interpret (e.g. 404 → not
///   found) — the error BODY is never read (§5.4: the recorded 404 echoes the
///   queried address);
/// - 5xx / anything else ⇒ retryable `ProviderUnavailable`.
async fn classify_response(
    response: hyper::Response<hyper::body::Incoming>,
    max_response_bytes: usize,
) -> Result<HttpOutcome, SwapError> {
    let status = response.status();
    if status.is_success() {
        // cheap pre-check, then a hard cap while streaming — the §4.6
        // size-cap-before-allocating rule for the body
        let declared = response
            .headers()
            .get(http::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<usize>().ok());
        if declared.is_some_and(|n| n > max_response_bytes) {
            return Err(SwapError::ProviderProtocol {
                reason: ProviderProtocolReason::OversizedBody,
            });
        }
        Limited::new(response.into_body(), max_response_bytes)
            .collect()
            .await
            .map(|c| HttpOutcome::Success(c.to_bytes()))
            .map_err(|_| SwapError::ProviderProtocol {
                reason: ProviderProtocolReason::OversizedBody,
            })
    } else if status.is_redirection() {
        Err(SwapError::ProviderProtocol {
            reason: ProviderProtocolReason::UnexpectedHttpStatus,
        })
    } else if status.is_client_error() {
        Ok(HttpOutcome::ClientError(status.as_u16()))
    } else {
        // 5xx and anything else: the provider is unwell — retryable
        Err(SwapError::ProviderUnavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::{RustlsSecurer, StreamSecurer};

    /// FR-5 spec §8 **P28, the swap half** — the twin of
    /// `zec-wallet-core`'s `net::grpc` test of the same name; the source half
    /// is `extraction_policy.rs::the_sdk_tls_stack_has_exactly_two_explicit_builder_sites`.
    /// The swap securer is the SECOND of the SDK's two TLS builders and, until
    /// FR-5 C2, the one with no pin at all. It must never read the
    /// process-default `CryptoProvider` — a process-global the host's first
    /// installer owns — because a funds-path HTTPS call to the swap provider
    /// would then run on whatever crypto the host chose.
    ///
    /// A FOREIGN default (`ring` with every cipher suite removed) is installed
    /// first; the securer must still build, from `ring`'s full suite list. The
    /// premise is asserted, not assumed: a default installed earlier by
    /// something else would make this prove nothing, and that is a failure.
    ///
    /// Watched red: the explicit `builder_with_provider(provider)` chain in
    /// `RustlsSecurer::new` replaced by `rustls::ClientConfig::builder()`.
    #[test]
    fn the_sdk_tls_never_reads_the_process_default_provider() {
        let mut foreign = rustls::crypto::ring::default_provider();
        foreign.cipher_suites.clear();
        let _ = foreign.install_default(); // PERMANENT for this test binary (no uninstall): safe only while nothing in src/** builds TLS from the default — P28's source scan in extraction_policy.rs refuses that
        let installed = rustls::crypto::CryptoProvider::get_default()
            .expect("a process-default provider is installed");
        assert!(
            installed.cipher_suites.is_empty(),
            "P28 premise: the process default is not this test's FOREIGN provider (it has \
             {} suites) — something installed a default first, so this binary cannot tell \
             an explicit provider from the default. Find the installer.",
            installed.cipher_suites.len()
        );

        let built = std::panic::catch_unwind(RustlsSecurer::new);
        let securer = built.unwrap_or_else(|_| {
            panic!(
                "P28: RustlsSecurer::new PANICKED with a foreign process-default provider \
                 installed — the swap builder read the process default instead of passing \
                 `ring` explicitly"
            )
        });
        let suites: Vec<_> = securer
            .connector
            .config()
            .crypto_provider()
            .cipher_suites
            .iter()
            .map(|s| s.suite())
            .collect();
        let ring: Vec<_> = rustls::crypto::ring::default_provider()
            .cipher_suites
            .iter()
            .map(|s| s.suite())
            .collect();
        assert!(!ring.is_empty(), "ring ships cipher suites");
        assert_eq!(
            suites, ring,
            "P28: the swap TLS config was not built from the explicit `ring` provider — its \
             suites are the process default's"
        );
    }

    /// No TLS session is ever kept or offered (the module doc: one securer
    /// serves every isolation key, so a resumed session would link two swaps'
    /// circuits for the provider). Two halves, because a first handshake alone
    /// cannot show the TLS 1.3 store:
    ///   * on the wire, the ClientHello asks for no TLS 1.2 session ticket
    ///     (extension 35), which rustls's default asks for on every hello;
    ///   * the configured store is rustls's no-op one, so no TLS 1.3 ticket is
    ///     kept to be offered on a later connection. Read through `Debug`:
    ///     rustls keeps the store private, and its pin is exact, so a rename
    ///     fails here loudly rather than passing.
    ///
    /// Watched red: the `Resumption::disabled()` line removed (the wire half
    /// fails first); and `Resumption::in_memory_sessions(256)
    /// .tls12_resumption(Tls12Resumption::Disabled)` in its place, which the
    /// wire half cannot see and the store half refuses.
    #[tokio::test]
    async fn the_swap_tls_never_resumes_a_session() {
        use tokio::io::AsyncReadExt;

        let securer = RustlsSecurer::new();
        let store = format!("{:?}", securer.connector.config().resumption);

        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let handshake =
            tokio::spawn(async move { securer.secure("swap.test", Box::new(client)).await });
        let mut header = [0u8; 5];
        server
            .read_exact(&mut header)
            .await
            .expect("the client wrote a TLS record header");
        assert_eq!(header[0], 22, "the first record is not a handshake record");
        let mut hello = vec![0u8; usize::from(u16::from_be_bytes([header[3], header[4]]))];
        server
            .read_exact(&mut hello)
            .await
            .expect("the client wrote its whole first record");
        drop(server);
        assert!(
            handshake.await.expect("the handshake task ran").is_err(),
            "with no server the handshake cannot succeed"
        );

        let extensions = client_hello_extensions(&hello);
        assert!(
            extensions.contains(&43),
            "anti-vacuity: no supported_versions extension (43) in {extensions:?}, so the parse \
             is not reading a rustls ClientHello"
        );
        assert!(
            !extensions.contains(&35),
            "the swap ClientHello asks for a TLS 1.2 session ticket (extension 35 in \
             {extensions:?}): a session from one swap could resume on another swap's circuit"
        );
        assert!(
            store.contains("NoClientSessionStorage"),
            "the swap TLS config keeps a session store ({store}): a TLS 1.3 ticket from one \
             swap would be offered on another swap's circuit"
        );
    }

    /// The extension types of one ClientHello handshake message, in order.
    fn client_hello_extensions(msg: &[u8]) -> Vec<u16> {
        let u16_at = |at: usize| u16::from_be_bytes([msg[at], msg[at + 1]]);
        assert_eq!(
            msg[0], 1,
            "the first handshake message is not a ClientHello"
        );
        // type (1), length (3), legacy_version (2), random (32)
        let mut at = 1 + 3 + 2 + 32;
        at += 1 + usize::from(msg[at]); // legacy_session_id
        at += 2 + usize::from(u16_at(at)); // cipher_suites
        at += 1 + usize::from(msg[at]); // legacy_compression_methods
        let end = at + 2 + usize::from(u16_at(at));
        at += 2;
        assert_eq!(
            end,
            msg.len(),
            "the extensions do not end where the record ends"
        );
        let mut types = Vec::new();
        while at < end {
            types.push(u16_at(at));
            at += 4 + usize::from(u16_at(at + 2));
        }
        assert_eq!(at, end, "the last extension overruns the ClientHello");
        types
    }
}
