//! Provider behavior tests over a SCRIPTED in-memory HTTP server (no real
//! network — testing-patterns: unit tests run against fakes; the live API
//! is the W-swap-2 E2E tier). The fake dialer + passthrough securer let the
//! hostile-transport arms (oversize, timeout, redirect, error statuses) be
//! driven byte-exactly.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use zec_wallet_core::{
    AssetId, AsyncByteStream, DialError, ExactSide, NetDialer, ProviderProtocolReason,
    QuoteRequest, SwapAmount, SwapDirection, SwapError, SwapId, SwapPort, SwapQuote, SwapStatus,
    Zatoshis,
};

use crate::http_client::StreamSecurer;
use crate::{NearIntentsProvider, SwapProviderConfig};

// ── Test doubles ─────────────────────────────────────────────────────────────

/// TLS passthrough — TEST ONLY (pub(crate) seam; the public constructor is
/// unconditionally rustls).
struct Plaintext;

#[async_trait]
impl StreamSecurer for Plaintext {
    async fn secure(
        &self,
        _host: &str,
        stream: Box<dyn AsyncByteStream>,
    ) -> Result<Box<dyn AsyncByteStream>, SwapError> {
        Ok(stream)
    }
}

/// Serves one scripted response per dial; records raw requests + isolation
/// keys. An empty script on dial = test bug (panic, loudly).
#[derive(Default)]
struct ScriptedDialer {
    responses: Mutex<VecDeque<Vec<u8>>>,
    seen_requests: Arc<Mutex<Vec<Vec<u8>>>>,
    seen_isolation: Mutex<Vec<Option<String>>>,
    dials: AtomicUsize,
}

impl ScriptedDialer {
    fn push(&self, response: Vec<u8>) {
        self.responses.lock().expect("script").push_back(response);
    }

    fn requests(&self) -> Vec<Vec<u8>> {
        self.seen_requests.lock().expect("requests").clone()
    }
}

#[async_trait]
impl NetDialer for ScriptedDialer {
    async fn dial(
        &self,
        _host: &str,
        _port: u16,
        isolation_key: Option<&str>,
    ) -> Result<Box<dyn AsyncByteStream>, DialError> {
        self.dials.fetch_add(1, Ordering::SeqCst);
        self.seen_isolation
            .lock()
            .expect("isolation")
            .push(isolation_key.map(str::to_owned));
        let response = self
            .responses
            .lock()
            .expect("script")
            .pop_front()
            .expect("scripted dialer: more dials than scripted responses");
        let (client, mut server) = tokio::io::duplex(1 << 20);
        let seen = Arc::clone(&self.seen_requests);
        tokio::spawn(async move {
            let request = read_http_request(&mut server).await;
            seen.lock().expect("requests").push(request);
            let _ = server.write_all(&response).await;
            let _ = server.flush().await;
            // hold the stream open until the peer hangs up — closing early
            // can race hyper's body read
            let mut sink = [0u8; 64];
            while matches!(server.read(&mut sink).await, Ok(n) if n > 0) {}
        });
        Ok(Box::new(client))
    }
}

/// Read one HTTP/1.1 request: head until CRLFCRLF + content-length body.
async fn read_http_request<S: tokio::io::AsyncRead + Unpin>(stream: &mut S) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    while !buf.ends_with(b"\r\n\r\n") {
        if stream.read(&mut byte).await.unwrap_or(0) == 0 {
            return buf;
        }
        buf.extend_from_slice(&byte);
    }
    let head = String::from_utf8_lossy(&buf).to_lowercase();
    let body_len = head
        .lines()
        .find_map(|l| l.strip_prefix("content-length:"))
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = vec![0u8; body_len];
    if body_len > 0 {
        let _ = stream.read_exact(&mut body).await;
    }
    buf.extend_from_slice(&body);
    buf
}

/// Always fail-closed — the `TorPolicy::Required`-with-no-Tor shape.
#[derive(Default)]
struct FailClosedDialer {
    dials: AtomicUsize,
}

#[async_trait]
impl NetDialer for FailClosedDialer {
    async fn dial(
        &self,
        _host: &str,
        _port: u16,
        _isolation_key: Option<&str>,
    ) -> Result<Box<dyn AsyncByteStream>, DialError> {
        self.dials.fetch_add(1, Ordering::SeqCst);
        Err(DialError::Unreachable)
    }
}

fn http_response(status: u16, reason: &str, body: &[u8]) -> Vec<u8> {
    let mut r = format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n",
        body.len()
    )
    .into_bytes();
    r.extend_from_slice(body);
    r
}

/// A minimal token table serving the fixtures' two assets.
fn tokens_body() -> Vec<u8> {
    br#"[
      {"assetId":"nep141:zec.omft.near","decimals":8,"blockchain":"zec","symbol":"ZEC","price":416.92,"priceUpdatedAt":"2026-06-13T00:31:30.560Z"},
      {"assetId":"nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1","decimals":6,"blockchain":"near","symbol":"USDC","price":0.99981,"priceUpdatedAt":"2026-06-13T00:31:30.560Z","contractAddress":"17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1"}
    ]"#
    .to_vec()
}

fn live_quote_body() -> &'static [u8] {
    include_bytes!("../tests/fixtures/derived/quote_live_out_of_zec.json")
}

fn provider(dialer: Arc<dyn NetDialer>, jwt: Option<&str>) -> NearIntentsProvider {
    NearIntentsProvider::with_securer(
        SwapProviderConfig {
            endpoint: "https://provider.test".into(),
            jwt: jwt.map(|j| zeroize::Zeroizing::new(j.to_owned())),
        },
        dialer,
        Arc::new(Plaintext),
    )
    .expect("provider construction")
}

fn out_of_zec_request() -> QuoteRequest {
    QuoteRequest {
        direction: SwapDirection::OutOfZec {
            to: AssetId {
                chain: "near".into(),
                symbol: "USDC".into(),
            },
        },
        exact: ExactSide::In(SwapAmount::Zec(Zatoshis::new(100_000_000).expect("1 ZEC"))),
        slippage_tolerance_bps: 100,
        destination: Some("example.near".into()),
        refund_address: Some("t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs".into()),
    }
}

// ── The tests ────────────────────────────────────────────────────────────────

/// Happy path, end to end over scripted HTTP: the OUTGOING request has the
/// pinned wire shape; the response maps to the pinned port DTO; pre-swap
/// calls share the quote isolation bucket.
#[tokio::test]
async fn quote_happy_path_pins_request_shape_and_port_dto() {
    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(200, "OK", &tokens_body()));
    dialer.push(http_response(201, "Created", live_quote_body()));
    let p = provider(dialer.clone(), None);

    let quote = p.quote(out_of_zec_request()).await.expect("quote");
    assert_eq!(quote.deposit_address, "t1ZXKD285AZdKs56b9ypo9FT4LMHKuJRw5N");
    assert_eq!(quote.amount_in, "1");
    assert_eq!(quote.min_amount_out, "412.248474");
    assert_eq!(quote.zec_side.zat(), 100_000_000);
    assert_eq!(quote.expires_at, 1_781_438_400);

    let requests = dialer.requests();
    assert_eq!(requests.len(), 2, "tokens fetch + quote");
    let quote_req = String::from_utf8_lossy(&requests[1]).to_string();
    assert!(quote_req.starts_with("POST /v0/quote HTTP/1.1\r\n"));
    assert!(quote_req.to_lowercase().contains("host: provider.test"));
    assert!(
        !quote_req.to_lowercase().contains("authorization"),
        "no JWT configured ⇒ no auth header"
    );
    let body_at = quote_req.find("\r\n\r\n").expect("body") + 4;
    let sent: serde_json::Value = serde_json::from_str(&quote_req[body_at..]).expect("json body");
    assert_eq!(sent["dry"], false, "quote() REGISTERS — the non-dry call");
    assert_eq!(sent["swapType"], "EXACT_INPUT");
    assert_eq!(sent["slippageTolerance"], 100);
    assert_eq!(sent["originAsset"], "nep141:zec.omft.near");
    assert_eq!(sent["depositType"], "ORIGIN_CHAIN");
    assert_eq!(sent["amount"], "100000000", "zatoshis = base units, exact");
    assert_eq!(sent["refundTo"], "t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs");
    assert_eq!(sent["refundType"], "ORIGIN_CHAIN");
    assert_eq!(sent["recipient"], "example.near");
    assert_eq!(sent["recipientType"], "DESTINATION_CHAIN");
    assert!(
        sent["deadline"].as_str().expect("deadline").ends_with('Z'),
        "RFC3339 UTC"
    );

    let isolation = dialer.seen_isolation.lock().expect("isolation").clone();
    assert_eq!(
        isolation,
        vec![Some("swap/quote".into()), Some("swap/quote".into())],
        "pre-swap calls share the quote bucket, distinct from sync + per-swap keys"
    );
}

/// The live fixture with its request echo edited by `edit`.
fn live_quote_body_with_echo(
    edit: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>),
) -> Vec<u8> {
    let mut v: serde_json::Value = serde_json::from_slice(live_quote_body()).expect("fixture json");
    edit(
        v["quoteRequest"]
            .as_object_mut()
            .expect("the fixture carries a quoteRequest echo"),
    );
    serde_json::to_vec(&v).expect("re-encode")
}

/// The live fixture re-echoing the IntoZec request the tests below send (USDC
/// in, ZEC out, our minted recipient, the user's `refund_to`): a provider echoes
/// the request it was sent, so an IntoZec quote is never answered with the
/// OutOfZec fixture's echo.
fn into_zec_quote_body(refund_to: &str) -> Vec<u8> {
    let refund_to = refund_to.to_owned();
    live_quote_body_with_echo(move |echo| {
        echo.insert(
            "originAsset".into(),
            "nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1".into(),
        );
        echo.insert("destinationAsset".into(), "nep141:zec.omft.near".into());
        echo.insert("recipient".into(), "u1ourzecdestination".into());
        echo.insert("refundTo".into(), refund_to.into());
    })
}

async fn quote_with_echo(
    edit: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>),
) -> Result<SwapQuote, SwapError> {
    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(200, "OK", &tokens_body()));
    dialer.push(http_response(
        201,
        "Created",
        &live_quote_body_with_echo(edit),
    ));
    provider(dialer, None).quote(out_of_zec_request()).await
}

/// 2026-10-07 external review, finding 3: the provider's echo of our request
/// must state every term we sent. For EACH compared field, a changed value is
/// refused as `RequestEchoMismatch` and an absent one as `MissingField` — and
/// the unedited fixture still quotes, so the refusals are about the edit.
#[tokio::test]
async fn quote_refuses_an_echo_that_contradicts_the_request() {
    assert!(
        quote_with_echo(|_| {}).await.is_ok(),
        "the unedited echo quotes"
    );
    let changed: [(&str, serde_json::Value); 13] = [
        ("dry", serde_json::json!(true)),
        ("swapType", serde_json::json!("EXACT_OUTPUT")),
        ("slippageTolerance", serde_json::json!(5000)),
        // The same value, wrongly TYPED: refused at the quote door, while a
        // status body carrying it still decodes (the fields are untyped).
        ("slippageTolerance", serde_json::json!("100")),
        ("amount", serde_json::json!(100_000_000)),
        ("originAsset", serde_json::json!("nep141:other.near")),
        ("depositType", serde_json::json!("INTENTS")),
        ("destinationAsset", serde_json::json!("nep141:wrap.near")),
        ("amount", serde_json::json!("100000001")),
        (
            "refundTo",
            serde_json::json!("t1VpYecBW4UudbGcy4ufh61eWxQCoFaUrPs"),
        ),
        ("refundType", serde_json::json!("INTENTS")),
        ("recipient", serde_json::json!("attacker.near")),
        ("recipientType", serde_json::json!("INTENTS")),
    ];
    for (key, value) in changed {
        let changed_key = key;
        // A different refund keeps the reason `SwapService`'s own check gives.
        let want = if key == "refundTo" {
            ProviderProtocolReason::RefundAddressMismatch
        } else {
            ProviderProtocolReason::RequestEchoMismatch
        };
        match quote_with_echo(move |echo| {
            echo.insert(changed_key.to_owned(), value);
        })
        .await
        {
            Err(SwapError::ProviderProtocol { reason }) if reason == want => {}
            other => panic!("a changed `{key}` echo must be {want:?}, got {other:?}"),
        }
        match quote_with_echo(move |echo| {
            echo.remove(key);
        })
        .await
        {
            Err(SwapError::ProviderProtocol {
                reason: ProviderProtocolReason::MissingField,
            }) => {}
            other => panic!("an absent `{key}` echo must be MissingField, got {other:?}"),
        }
    }
}

/// The §8 m5 row (`tor_required_covers_swap_api_calls`): the adapter's ONLY
/// network path is the injected dialer — when it fail-closes (TorPolicy::
/// Required without Tor), every endpoint fails typed with ZERO bytes sent,
/// and the local-only `execute` still works.
#[tokio::test]
async fn tor_required_covers_swap_api_calls() {
    let dialer = Arc::new(FailClosedDialer::default());
    let p = provider(dialer.clone(), None);

    assert!(matches!(
        p.quote(out_of_zec_request()).await,
        Err(SwapError::ProviderUnavailable)
    ));
    assert!(matches!(
        p.status(&SwapId::new("q-tor"), "t1ZXKD285AZdKs56b9ypo9FT4LMHKuJRw5N")
            .await,
        Err(SwapError::ProviderUnavailable)
    ));
    assert!(matches!(
        p.supported_tokens().await,
        Err(SwapError::ProviderUnavailable)
    ));
    assert!(matches!(
        p.notify_deposit_tx(
            &SwapId::new("q-tor"),
            "t1ZXKD285AZdKs56b9ypo9FT4LMHKuJRw5N",
            "txhash"
        )
        .await,
        Err(SwapError::ProviderUnavailable)
    ));
    assert_eq!(
        dialer.dials.load(Ordering::SeqCst),
        4,
        "one dial attempt each"
    );

    // execute is the LOCAL commit point: no network even fail-closed
    let quote = sample_quote();
    p.execute(&quote).await.expect("local execute");
    assert_eq!(
        dialer.dials.load(Ordering::SeqCst),
        4,
        "execute dials nothing"
    );
}

fn sample_quote() -> SwapQuote {
    SwapQuote {
        // FR-17: adapters never mint the binding (service authority).
        binding: None,
        id: SwapId::new("t1ZXKD285AZdKs56b9ypo9FT4LMHKuJRw5N"),
        deposit_address: "t1ZXKD285AZdKs56b9ypo9FT4LMHKuJRw5N".into(),
        deposit_memo: None,
        expires_at: 1_781_438_400,
        amount_in: "1".into(),
        min_amount_out: "412.248474".into(),
        zec_side: Zatoshis::new(100_000_000).expect("1 ZEC"),
        refund_to: Some("t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs".into()),
        disclosure: crate::map::disclosure_for(&SwapDirection::OutOfZec {
            to: AssetId {
                chain: "near".into(),
                symbol: "USDC".into(),
            },
        })
        .expect("known direction"),
    }
}

/// The §8 privacy row (`jwt_never_logged_and_zeroized`): the credential goes
/// ON THE WIRE (that is its job) and NOWHERE else — no tracing span, event,
/// or field ever carries it. (Zeroization is the type system's half:
/// `SwapProviderConfig::jwt` and the prebuilt header value are `Zeroizing`,
/// wiped on drop.)
#[tokio::test]
async fn jwt_never_logged_and_zeroized() {
    const JWT: &str = "test-jwt-secret-XYZZY-3a02fc";

    // minimal capture subscriber: every span field + event field, Debug-fmt
    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<String>>);
    struct Visitor<'a>(&'a Mutex<String>);
    impl tracing::field::Visit for Visitor<'_> {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            use std::fmt::Write;
            let _ = write!(
                self.0.lock().expect("capture"),
                " {}={:?}",
                field.name(),
                value
            );
        }
    }
    impl tracing::Subscriber for Capture {
        fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            span.record(&mut Visitor(&self.0));
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _: &tracing::span::Id, values: &tracing::span::Record<'_>) {
            values.record(&mut Visitor(&self.0));
        }
        fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            event.record(&mut Visitor(&self.0));
        }
        fn enter(&self, _: &tracing::span::Id) {}
        fn exit(&self, _: &tracing::span::Id) {}
    }

    let capture = Capture::default();
    let _guard = tracing::subscriber::set_default(capture.clone());

    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(200, "OK", &tokens_body()));
    dialer.push(http_response(500, "Internal Server Error", b"")); // error arm logs too
    let p = provider(dialer.clone(), Some(JWT));
    let _ = p.quote(out_of_zec_request()).await; // outcome irrelevant here

    let requests = dialer.requests();
    let on_wire = String::from_utf8_lossy(&requests[1]).to_lowercase();
    assert!(
        on_wire.contains(&format!("authorization: bearer {}", JWT.to_lowercase())),
        "the credential MUST be sent — that is its job"
    );
    let captured = capture.0.lock().expect("capture").clone();
    assert!(
        !captured.contains(JWT) && !captured.to_lowercase().contains("bearer"),
        "JWT leaked into tracing output: {captured}"
    );
}

/// §4.6: a declared body beyond the endpoint cap is refused BEFORE
/// buffering; a chunked body that streams past the cap dies at the cap.
#[tokio::test]
async fn oversized_bodies_are_rejected_at_the_named_caps() {
    // declared: content-length lies far above the tokens cap
    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(
        b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 10485760\r\n\r\n"
            .to_vec(),
    );
    let p = provider(dialer, None);
    match p.supported_tokens().await {
        Err(SwapError::ProviderProtocol {
            reason: ProviderProtocolReason::OversizedBody,
        }) => {}
        other => panic!("declared oversize must be OversizedBody, got {other:?}"),
    }

    // streamed: chunked encoding, no content-length, body > 64 KiB quote cap
    let mut chunked =
        b"HTTP/1.1 201 Created\r\ncontent-type: application/json\r\ntransfer-encoding: chunked\r\n\r\n"
            .to_vec();
    let chunk = vec![b'A'; 16 * 1024];
    for _ in 0..5 {
        chunked.extend_from_slice(format!("{:x}\r\n", chunk.len()).as_bytes());
        chunked.extend_from_slice(&chunk);
        chunked.extend_from_slice(b"\r\n");
    }
    chunked.extend_from_slice(b"0\r\n\r\n");
    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(200, "OK", &tokens_body()));
    dialer.push(chunked);
    let p = provider(dialer, None);
    match p.quote(out_of_zec_request()).await {
        Err(SwapError::ProviderProtocol {
            reason: ProviderProtocolReason::OversizedBody,
        }) => {}
        other => panic!("streamed oversize must be OversizedBody, got {other:?}"),
    }
}

/// Funds path never follows a redirect; 4xx/5xx map per contract; 404 on
/// status is the typed SwapNotFound.
#[tokio::test]
async fn http_statuses_map_typed() {
    // 301 → UnexpectedHttpStatus
    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(301, "Moved Permanently", b""));
    let p = provider(dialer, None);
    match p.supported_tokens().await {
        Err(SwapError::ProviderProtocol {
            reason: ProviderProtocolReason::UnexpectedHttpStatus,
        }) => {}
        other => panic!("redirect must be typed, got {other:?}"),
    }

    // 404 on /v0/status → SwapNotFound (error body NEVER read: it echoes
    // the queried address — the recorded fixture proves that)
    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(
        404,
        "Not Found",
        include_bytes!("../tests/fixtures/recorded/status_not_found_404.json"),
    ));
    let p = provider(dialer, None);
    match p
        .status(&SwapId::new("q-404"), "t1ZXKD285AZdKs56b9ypo9FT4LMHKuJRw5N")
        .await
    {
        Err(SwapError::ProviderProtocol {
            reason: ProviderProtocolReason::SwapNotFound,
        }) => {}
        other => panic!("status 404 must be SwapNotFound, got {other:?}"),
    }

    // other 4xx → UnexpectedHttpStatus (the service pre-validated; 400 here
    // is contract drift, not user error)
    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(200, "OK", &tokens_body()));
    dialer.push(http_response(
        400,
        "Bad Request",
        include_bytes!("../tests/fixtures/recorded/quote_bad_request_400.json"),
    ));
    let p = provider(dialer, None);
    match p.quote(out_of_zec_request()).await {
        Err(SwapError::ProviderProtocol {
            reason: ProviderProtocolReason::UnexpectedHttpStatus,
        }) => {}
        other => panic!("4xx must be typed, got {other:?}"),
    }

    // 5xx → retryable ProviderUnavailable
    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(503, "Service Unavailable", b""));
    let p = provider(dialer, None);
    assert!(matches!(
        p.supported_tokens().await,
        Err(SwapError::ProviderUnavailable)
    ));
}

/// A server that accepts the request and never answers cannot hang the
/// poller: the whole exchange shares one hard budget (paused-clock test —
/// deterministic, no real sleeping).
#[tokio::test(start_paused = true)]
async fn half_open_exchange_times_out_typed() {
    #[derive(Default)]
    struct SilentDialer;
    #[async_trait]
    impl NetDialer for SilentDialer {
        async fn dial(
            &self,
            _host: &str,
            _port: u16,
            _isolation_key: Option<&str>,
        ) -> Result<Box<dyn AsyncByteStream>, DialError> {
            let (client, mut server) = tokio::io::duplex(1 << 16);
            tokio::spawn(async move {
                let _ = read_http_request(&mut server).await;
                std::future::pending::<()>().await; // never answer
            });
            Ok(Box::new(client))
        }
    }
    let p = provider(Arc::new(SilentDialer), None);
    assert!(matches!(
        p.supported_tokens().await,
        Err(SwapError::ProviderUnavailable)
    ));
}

/// A dialer that never resolves (blackholed connect, a dialer impl without
/// its own timeout) cannot hang the poller either: the dial is INSIDE the
/// one hard budget (review fold — the guarantee is self-contained, not
/// delegated to an unstated dialer property).
#[tokio::test(start_paused = true)]
async fn hanging_dial_times_out_typed() {
    struct HangingDialer;
    #[async_trait]
    impl NetDialer for HangingDialer {
        async fn dial(
            &self,
            _host: &str,
            _port: u16,
            _isolation_key: Option<&str>,
        ) -> Result<Box<dyn AsyncByteStream>, DialError> {
            std::future::pending().await
        }
    }
    let p = provider(Arc::new(HangingDialer), None);
    assert!(matches!(
        p.supported_tokens().await,
        Err(SwapError::ProviderUnavailable)
    ));
}

/// FR-29 stage 0 (i) / ADR-0544 decision 2: the per-swap isolation key is an
/// SDK-minted opaque token — no byte of the `SwapId` (the provider's deposit
/// address, a §5.4 never-log item the PROVIDER chose) reaches `dial`. Pins:
/// one swap ⇒ one key for the adapter's life (its deposit notification and
/// every status poll share the circuit, as before); two swaps ⇒ two keys; the
/// key is fixed width, prefixed, and carries no window of the address; a
/// second adapter instance maps the same swap to a DIFFERENT key (unlinkable
/// across processes and relaunches).
#[tokio::test]
async fn no_swap_id_byte_reaches_the_dialer_isolation_key() {
    let a = SwapId::new("t1ZXKD285AZdKs56b9ypo9FT4LMHKuJRw5N");
    let b = SwapId::new("t1Kh9J3y2iCwvmc2VU8FoJh5DhZjcTzbBnL");
    let status = include_bytes!("../tests/fixtures/derived/status_incomplete_deposit.json");

    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(200, "OK", status)); // status(a)
    dialer.push(http_response(200, "OK", &tokens_body())); // decimals lookup (once)
    dialer.push(http_response(200, "OK", status)); // status(a) again
    dialer.push(http_response(200, "OK", status)); // status(b)
    dialer.push(http_response(200, "OK", status)); // notify_deposit_tx(a)
    let p = provider(dialer.clone(), None);
    p.status(&a, a.as_str()).await.expect("status a");
    p.status(&a, a.as_str()).await.expect("status a again");
    p.status(&b, b.as_str()).await.expect("status b");
    p.notify_deposit_tx(&a, a.as_str(), "0xabc")
        .await
        .expect("notify a");

    let seen = dialer.seen_isolation.lock().expect("isolation").clone();
    let keys: Vec<&str> = seen.iter().map(|k| k.as_deref().expect("keyed")).collect();
    assert_eq!(keys.len(), 5, "five dials: a, tokens, a, b, a");
    let (key_a, key_b) = (keys[0], keys[3]);
    assert_eq!(
        keys[1], "swap/quote",
        "the resolution fill rides the quote bucket"
    );
    assert_eq!(
        keys[2], key_a,
        "a second poll of the same swap shares its circuit"
    );
    assert_eq!(
        keys[4], key_a,
        "the deposit notification shares the swap's circuit"
    );
    assert_ne!(key_a, key_b, "two swaps never share a circuit");
    for (key, id) in [(key_a, &a), (key_b, &b)] {
        assert_eq!(key.len(), "swap/id/".len() + 32, "fixed width: {key}");
        assert!(key.starts_with("swap/id/"), "SDK-owned namespace: {key}");
        assert!(
            key["swap/id/".len()..]
                .bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "lowercase hex token: {key}"
        );
        let address = id.as_str();
        assert_ne!(key, address);
        assert!(address.len() >= 6, "the window scan needs a real address");
        for start in 0..=address.len().saturating_sub(6) {
            let window = &address[start..start + 6];
            assert!(
                !key.contains(window),
                "an address window reached the dialer: {window} in {key}"
            );
        }
    }

    // A fresh adapter instance mints a fresh secret: the same swap maps elsewhere.
    let other = Arc::new(ScriptedDialer::default());
    other.push(http_response(200, "OK", status));
    other.push(http_response(200, "OK", &tokens_body()));
    let q = provider(other.clone(), None);
    q.status(&a, a.as_str())
        .await
        .expect("status a on the second instance");
    let other_seen = other.seen_isolation.lock().expect("isolation").clone();
    assert_ne!(
        other_seen[0].as_deref().expect("keyed"),
        key_a,
        "another instance (process, relaunch) maps the same swap to a different key"
    );
}

/// Status responses map e2e through the provider, per-swap isolation key
/// included; unknown lifecycle words surface as the typed terminal status.
#[tokio::test]
async fn status_maps_e2e_with_per_swap_isolation() {
    let id = SwapId::new("t1ZXKD285AZdKs56b9ypo9FT4LMHKuJRw5N");

    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(
        200,
        "OK",
        include_bytes!("../tests/fixtures/derived/status_incomplete_deposit.json"),
    ));
    dialer.push(http_response(200, "OK", &tokens_body())); // decimals lookup
    let p = provider(dialer.clone(), None);
    assert_eq!(
        p.status(&id, id.as_str()).await.expect("status"),
        SwapStatus::UnderDeposited {
            received: "0.4".into(),
            missing: "0.6".into(),
            deadline: 1_781_438_400,
        }
    );
    let isolation = dialer.seen_isolation.lock().expect("isolation").clone();
    let key = isolation[0]
        .as_deref()
        .expect("the status poll carries an isolation key");
    assert!(
        key.starts_with("swap/id/") && key != id.as_str(),
        "status polls isolate per swap through the SDK-minted token, never the address: {key}"
    );
    let status_req = String::from_utf8_lossy(&dialer.requests()[0]).to_string();
    assert!(
        status_req.starts_with(
            "GET /v0/status?depositAddress=t1ZXKD285AZdKs56b9ypo9FT4LMHKuJRw5N HTTP/1.1\r\n"
        ),
        "deposit address rides percent-encoded in the query: {status_req}"
    );

    // unknown lifecycle word → typed terminal status, not an error
    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(
        200,
        "OK",
        include_bytes!("../tests/fixtures/derived/status_unknown_variant.json"),
    ));
    dialer.push(http_response(200, "OK", &tokens_body()));
    let p = provider(dialer, None);
    assert_eq!(
        p.status(&id, id.as_str())
            .await
            .expect("closed-enum mapping"),
        SwapStatus::Failed {
            code: zec_wallet_core::SwapFailureCode::ProviderProtocol
        }
    );
}

/// The handle guards sit BEFORE any network: an oversized (or empty) provider
/// handle is refused typed with zero dials (gate 7 — `PROVIDER_STR_MAX_BYTES`
/// at the call boundary).
#[tokio::test]
async fn oversized_or_empty_swap_id_refused_before_any_dial() {
    let dialer = Arc::new(ScriptedDialer::default());
    let p = provider(dialer.clone(), None);
    let id = SwapId::new("q-bound");
    let oversized = "x".repeat(257);
    assert!(matches!(
        p.status(&id, &oversized).await,
        Err(SwapError::RequestInvalid { .. })
    ));
    assert!(matches!(
        p.notify_deposit_tx(&id, &oversized, "txhash").await,
        Err(SwapError::RequestInvalid { .. })
    ));
    assert!(matches!(
        p.status(&id, "").await,
        Err(SwapError::RequestInvalid { .. })
    ));
    assert_eq!(
        dialer.dials.load(Ordering::SeqCst),
        0,
        "refused pre-network"
    );
}

/// Resolution edges: an unknown asset symbol and an ambiguous asset (matching
/// several listings) both fail typed after the token fetch, with no swap funds
/// ever committed. (IntoZec is now a full path — IZ-1 — so direction is no longer
/// a rejection case; see `into_zec_quote_pins_request_shape_and_maps_zec_output`.)
#[tokio::test]
async fn asset_resolution_fail_typed() {
    // unknown asset
    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(200, "OK", &tokens_body()));
    let p = provider(dialer, None);
    let mut req = out_of_zec_request();
    req.direction = SwapDirection::OutOfZec {
        to: AssetId {
            chain: "near".into(),
            symbol: "NOPE".into(),
        },
    };
    assert!(matches!(
        p.quote(req).await,
        Err(SwapError::RequestInvalid { .. })
    ));

    // ambiguous asset: two listings with the same (chain, symbol)
    let dialer = Arc::new(ScriptedDialer::default());
    let mut two = String::from_utf8(tokens_body()).expect("utf8");
    two.insert_str(
        two.rfind(']').expect("array"),
        r#",{"assetId":"nep141:other-usdc.near","decimals":6,"blockchain":"near","symbol":"USDC","price":1.0,"priceUpdatedAt":"2026-06-13T00:31:30.560Z"}"#,
    );
    dialer.push(http_response(200, "OK", two.as_bytes()));
    let p = provider(dialer, None);
    assert!(matches!(
        p.quote(out_of_zec_request()).await,
        Err(SwapError::RequestInvalid { .. })
    ));
}

/// IZ-1 / §3.3b: the IntoZec adapter path — the OUTGOING request mirrors OutOfZec with the asset
/// roles SWAPPED (origin = foreign source, destination = ZEC; recipient = OUR minted ZEC address,
/// refundTo = the user's source-chain refund), and the response maps ZEC as the OUTPUT side
/// (min_amount_out → zec_side). Reuses the live quote-response fixture — the wire SHAPE is
/// direction-independent; only the semantics of which side is ZEC differ.
#[tokio::test]
async fn into_zec_quote_pins_request_shape_and_maps_zec_output() {
    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(200, "OK", &tokens_body()));
    dialer.push(http_response(
        201,
        "Created",
        &into_zec_quote_body("alice.near"),
    ));
    let p = provider(dialer.clone(), None);

    let req = QuoteRequest {
        direction: SwapDirection::IntoZec {
            from: AssetId {
                chain: "near".into(),
                symbol: "USDC".into(),
            },
        },
        exact: ExactSide::In(SwapAmount::Foreign("100".into())),
        slippage_tolerance_bps: 100,
        destination: Some("u1ourzecdestination".into()), // the SERVICE-minted recipient
        refund_address: Some("alice.near".into()),       // the user's source-chain refund
    };
    let quote = p.quote(req).await.expect("intozec quote");
    // ZEC is the OUTPUT side: amount_in is the FOREIGN input (USDC, 6 decimals), min_amount_out is
    // ZEC (8 decimals), zec_side its zatoshis — the inverse of the OutOfZec mapping.
    assert_eq!(
        quote.amount_in, "100",
        "foreign input formatted with USDC decimals"
    );
    assert_eq!(
        quote.min_amount_out, "4.12248474",
        "ZEC output formatted with 8 decimals"
    );
    assert_eq!(
        quote.zec_side.zat(),
        412_248_474,
        "zec_side is the ZEC OUTPUT in zatoshis (not the foreign input)"
    );
    assert!(
        quote.disclosure.ends_shielded,
        "IntoZec ends shielded (nudged)"
    );

    // the OUTGOING request: roles swapped vs OutOfZec (origin = foreign, destination = ZEC).
    let requests = dialer.requests();
    assert_eq!(requests.len(), 2, "tokens fetch + quote");
    let quote_req = String::from_utf8_lossy(&requests[1]).to_string();
    let body_at = quote_req.find("\r\n\r\n").expect("body") + 4;
    let sent: serde_json::Value = serde_json::from_str(&quote_req[body_at..]).expect("json body");
    assert_eq!(sent["swapType"], "EXACT_INPUT");
    assert_eq!(
        sent["originAsset"],
        "nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1",
        "origin is the foreign SOURCE asset (USDC)"
    );
    assert_eq!(
        sent["destinationAsset"], "nep141:zec.omft.near",
        "destination is ZEC"
    );
    assert_eq!(
        sent["amount"], "100000000",
        "foreign base units (100 × 10^6)"
    );
    assert_eq!(
        sent["recipient"], "u1ourzecdestination",
        "our minted ZEC destination is the recipient"
    );
    assert_eq!(sent["recipientType"], "DESTINATION_CHAIN");
    assert_eq!(
        sent["refundTo"], "alice.near",
        "the user's source-chain refund target (NOT a wallet address)"
    );
    assert_eq!(sent["refundType"], "ORIGIN_CHAIN");
}

/// §3.3b D6 / the Mostpost wire-injection lesson: the user-supplied `refund_address` reaches the
/// 1Click request ONLY as a `serde_json`-serialized value — a crafted `","recipient":"attacker"`
/// + CRLF cannot break out and forge a second field. The protection is serde escaping; this is the
/// regression GUARD (a future switch to manual/templated encoding would fail this test).
#[tokio::test]
async fn refund_address_json_injection_is_escaped() {
    // a refund address that TRIES to smuggle a second JSON field (override recipient) + CRLF
    let evil = "alice.near\",\"recipient\":\"attacker\r\nx";
    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(200, "OK", &tokens_body()));
    dialer.push(http_response(201, "Created", &into_zec_quote_body(evil)));
    let p = provider(dialer.clone(), None);

    let req = QuoteRequest {
        direction: SwapDirection::IntoZec {
            from: AssetId {
                chain: "near".into(),
                symbol: "USDC".into(),
            },
        },
        exact: ExactSide::In(SwapAmount::Foreign("100".into())),
        slippage_tolerance_bps: 100,
        destination: Some("u1ourzecdestination".into()),
        refund_address: Some(evil.into()),
    };
    p.quote(req).await.expect("quote");

    let requests = dialer.requests();
    let quote_req = String::from_utf8_lossy(&requests[1]).to_string();
    let body_at = quote_req.find("\r\n\r\n").expect("body") + 4;
    // the body is STILL valid JSON — the injection did not break the structure
    let sent: serde_json::Value =
        serde_json::from_str(&quote_req[body_at..]).expect("the body is valid JSON (no breakout)");
    // exactly OUR recipient — the smuggled `"recipient":"attacker"` did NOT override it
    assert_eq!(
        sent["recipient"], "u1ourzecdestination",
        "the smuggled recipient did not override ours — injection failed",
    );
    // refundTo carries the exact bytes as ONE serde-escaped string value, not structure
    assert_eq!(
        sent["refundTo"], evil,
        "refundTo is the exact bytes, escaped — not a second forged field",
    );
}

/// §4.4 W-swap-4-a-3: the requested deposit window is DIRECTION-DEPENDENT. An
/// OutOfZec deposit is sent by the WALLET within seconds of execute, so the wire
/// `deadline` asks for 15 min (`SWAP_DEPOSIT_DEADLINE_OUT_OF_ZEC_SECS`); IntoZec's
/// EXTERNAL user deposit keeps the 24 h `SWAP_DEADLINE_DEFAULT_SECS`. Requesting
/// 24 h both ways silently re-priced every §4.4 deposit-tag lockout
/// (guard / zombie / under-funded `Stale`) from minutes to a day — the
/// ground fact this pins against regression.
#[tokio::test]
async fn quote_request_deadline_is_direction_dependent() {
    fn wall_now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("post-epoch test clock")
            .as_secs()
    }
    fn sent_deadline_unix(dialer: &ScriptedDialer) -> u64 {
        let requests = dialer.requests();
        let quote_req = String::from_utf8_lossy(&requests[1]).to_string();
        let body_at = quote_req.find("\r\n\r\n").expect("body") + 4;
        let sent: serde_json::Value =
            serde_json::from_str(&quote_req[body_at..]).expect("json body");
        crate::time::parse_rfc3339_utc(sent["deadline"].as_str().expect("deadline"))
            .expect("wire deadline parses back")
    }

    // OutOfZec: deadline = now + the 15-min wallet-sends-it window.
    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(200, "OK", &tokens_body()));
    dialer.push(http_response(201, "Created", live_quote_body()));
    let p = provider(dialer.clone(), None);
    let before = wall_now();
    p.quote(out_of_zec_request()).await.expect("quote");
    let after = wall_now();
    let sent = sent_deadline_unix(&dialer);
    let w = zec_wallet_core::constants::SWAP_DEPOSIT_DEADLINE_OUT_OF_ZEC_SECS;
    assert!(
        (before + w..=after + w).contains(&sent),
        "OutOfZec requests the short window (sent {sent}, expected now+{w})",
    );

    // IntoZec: deadline = now + the 24 h external-deposit window.
    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(200, "OK", &tokens_body()));
    dialer.push(http_response(
        201,
        "Created",
        &into_zec_quote_body("alice.near"),
    ));
    let p = provider(dialer.clone(), None);
    let req = QuoteRequest {
        direction: SwapDirection::IntoZec {
            from: AssetId {
                chain: "near".into(),
                symbol: "USDC".into(),
            },
        },
        exact: ExactSide::In(SwapAmount::Foreign("100".into())),
        slippage_tolerance_bps: 100,
        destination: Some("u1ourzecdestination".into()),
        refund_address: Some("alice.near".into()),
    };
    let before = wall_now();
    p.quote(req).await.expect("intozec quote");
    let after = wall_now();
    let sent = sent_deadline_unix(&dialer);
    let w = zec_wallet_core::constants::SWAP_DEADLINE_DEFAULT_SECS;
    assert!(
        (before + w..=after + w).contains(&sent),
        "IntoZec keeps the 24 h window (sent {sent}, expected now+{w})",
    );
}

/// notify_deposit_tx posts the pinned shape and maps the status response.
#[tokio::test]
async fn notify_deposit_tx_posts_and_maps() {
    let id = SwapId::new("t1ZXKD285AZdKs56b9ypo9FT4LMHKuJRw5N");
    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(
        201,
        "Created",
        include_bytes!("../tests/fixtures/derived/status_processing.json"),
    ));
    dialer.push(http_response(200, "OK", &tokens_body()));
    let p = provider(dialer.clone(), None);
    let status = p
        .notify_deposit_tx(
            &id,
            id.as_str(),
            "f4184fc596403b9d638783cf57adfe4c75c605f6356fbc91338530e9831e9e16",
        )
        .await
        .expect("notify");
    assert_eq!(status, SwapStatus::Processing);

    let req = String::from_utf8_lossy(&dialer.requests()[0]).to_string();
    assert!(req.starts_with("POST /v0/deposit/submit HTTP/1.1\r\n"));
    let body_at = req.find("\r\n\r\n").expect("body") + 4;
    let sent: serde_json::Value = serde_json::from_str(&req[body_at..]).expect("json");
    assert_eq!(sent["depositAddress"], id.as_str());
    assert_eq!(
        sent["txHash"],
        "f4184fc596403b9d638783cf57adfe4c75c605f6356fbc91338530e9831e9e16"
    );
}

// ── IZ-2: the dynamic token list (`list_tokens`, §3.3b D5/L6 / ADR-0530) ──────────────────────

/// A `/v0/tokens` body with the ZEC asset + a `$0` entry + a NULL-price entry + one real foreign
/// asset — exercises every drop rule of the picker filter at once.
fn tokens_body_for_filter() -> Vec<u8> {
    br#"[
      {"assetId":"nep141:zec.omft.near","decimals":8,"blockchain":"zec","symbol":"ZEC","price":416.92},
      {"assetId":"nep141:zero.near","decimals":6,"blockchain":"near","symbol":"ZERO","price":0},
      {"assetId":"nep141:nullp.near","decimals":6,"blockchain":"near","symbol":"NULLP"},
      {"assetId":"nep141:usdc.near","decimals":6,"blockchain":"near","symbol":"USDC","price":0.99981}
    ]"#
    .to_vec()
}

/// Serves scripted responses while available, then FAIL-CLOSES — the unstable-network shape (a
/// first good fetch, a later timeout/unreachable). Unlike [`ScriptedDialer`] (which panics when out
/// of script), an exhausted script here is `Unreachable`, so the serve-stale path is reachable.
#[derive(Default)]
struct ThenFailDialer {
    responses: Mutex<VecDeque<Vec<u8>>>,
}

impl ThenFailDialer {
    fn push(&self, response: Vec<u8>) {
        self.responses.lock().expect("script").push_back(response);
    }
}

#[async_trait]
impl NetDialer for ThenFailDialer {
    async fn dial(
        &self,
        _host: &str,
        _port: u16,
        _isolation_key: Option<&str>,
    ) -> Result<Box<dyn AsyncByteStream>, DialError> {
        let Some(response) = self.responses.lock().expect("script").pop_front() else {
            return Err(DialError::Unreachable);
        };
        let (client, mut server) = tokio::io::duplex(1 << 20);
        tokio::spawn(async move {
            let _ = read_http_request(&mut server).await;
            let _ = server.write_all(&response).await;
            let _ = server.flush().await;
            let mut sink = [0u8; 64];
            while matches!(server.read(&mut sink).await, Ok(n) if n > 0) {}
        });
        Ok(Box::new(client))
    }
}

#[tokio::test]
async fn list_tokens_filters_zero_price_and_the_zec_asset() {
    // §3.3b D5: the picker list drops the ZEC asset itself (never a foreign pick) AND every
    // `$0`/null-price entry — leaving only real, quotable foreign assets. And it rides the DEDICATED
    // tokens circuit (`swap/tokens`), never sync's or quote's.
    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(200, "OK", &tokens_body_for_filter()));
    let p = provider(dialer.clone(), None);

    let list = p.list_tokens().await.expect("list");
    assert!(list.fresh, "a successful fetch is fresh");
    let symbols: Vec<&str> = list.tokens.iter().map(|t| t.symbol.as_str()).collect();
    assert_eq!(
        symbols,
        vec!["USDC"],
        "ZEC (the wallet's own side), the $0 ZERO, and the null-price NULLP are all dropped"
    );
    let usdc = &list.tokens[0];
    assert_eq!(usdc.chain, "near");
    assert_eq!(usdc.decimals, 6);
    assert_eq!(usdc.provider_asset_id, "nep141:usdc.near");
    assert_eq!(usdc.price_usd, Some(0.99981));

    let isolation = dialer.seen_isolation.lock().expect("isolation").clone();
    assert_eq!(
        isolation,
        vec![Some("swap/tokens".into())],
        "the token list rides its OWN circuit bucket, unlinkable from sync + quote (§3.3b D5)"
    );
}

#[tokio::test]
async fn list_tokens_times_out_and_serves_stale_cache_honestly() {
    // §3.3b L6: a first fetch caches the list; a later fetch FAULT (timeout/unreachable) serves the
    // LAST good list with `fresh = false` — the host's "couldn't refresh, showing cached" banner,
    // never a blank picker — rather than failing.
    let dialer = Arc::new(ThenFailDialer::default());
    dialer.push(http_response(200, "OK", &tokens_body_for_filter())); // dial #1 succeeds
    // dial #2 onward: the script is empty ⇒ `Unreachable` (the unstable-network case)
    let p = provider(dialer, None);

    let first = p.list_tokens().await.expect("first list");
    assert!(first.fresh, "the first fetch is fresh");
    assert_eq!(first.tokens.len(), 1);

    let stale = p
        .list_tokens()
        .await
        .expect("stale list served, not an error");
    assert!(
        !stale.fresh,
        "the fetch faulted ⇒ served from cache, marked stale"
    );
    assert_eq!(
        stale.tokens, first.tokens,
        "the stale list is byte-identical to the last good one"
    );
}

#[tokio::test]
async fn tor_required_covers_list_tokens() {
    // §3.3b D5 fail-closed: with no Tor (the `TorPolicy::Required`-fail-closed shape) and NOTHING
    // cached, `list_tokens` fails typed with ZERO bytes sent — the picker can't leak clearnet
    // traffic. (The serve-stale fallback only applies once a good list has been cached.)
    let dialer = Arc::new(FailClosedDialer::default());
    let p = provider(dialer.clone(), None);
    assert!(matches!(
        p.list_tokens().await,
        Err(SwapError::ProviderUnavailable)
    ));
    assert_eq!(
        dialer.dials.load(Ordering::SeqCst),
        1,
        "exactly one dial attempt, fail-closed (no retry storm)"
    );
}

/// A `/v0/tokens` body whose EVERY entry is droppable (the ZEC asset + a `$0` + a null-price) so
/// the picker filter empties the list — the "no foreign assets quotable right now" shape.
fn tokens_body_all_dropped() -> Vec<u8> {
    br#"[
      {"assetId":"nep141:zec.omft.near","decimals":8,"blockchain":"zec","symbol":"ZEC","price":416.92},
      {"assetId":"nep141:zero.near","decimals":6,"blockchain":"near","symbol":"ZERO","price":0},
      {"assetId":"nep141:nullp.near","decimals":6,"blockchain":"near","symbol":"NULLP"}
    ]"#
    .to_vec()
}

#[tokio::test]
async fn list_tokens_all_filtered_is_empty_but_fresh_not_stale() {
    // §3.3b L6: a SUCCESSFUL fetch whose every entry is dropped (ZEC + $0 + null-price) yields an
    // EMPTY list that is STILL `fresh = true` — the honest "no source assets available right now"
    // empty-picker state. This is DISTINCT from the stale fallback (a fault serving a cached list
    // with `fresh = false`): an empty-fresh list means "we reached the provider and there is
    // genuinely nothing to pick," never "we couldn't refresh." The host renders the two banners
    // differently, so the freshness flag MUST stay true here.
    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(200, "OK", &tokens_body_all_dropped()));
    let p = provider(dialer.clone(), None);

    let list = p
        .list_tokens()
        .await
        .expect("empty list is success, not error");
    assert!(
        list.tokens.is_empty(),
        "every entry was droppable ⇒ the picker is empty"
    );
    assert!(
        list.fresh,
        "an empty list from a GOOD fetch is FRESH — not the stale-fallback banner"
    );
}

#[tokio::test]
async fn list_tokens_malformed_entry_faults_then_serves_stale_else_typed_error() {
    // §3.3b L6 + the strict-mapping note on `list_tokens`: `map_tokens` is WHOLE-LIST strict — a
    // single malformed entry (here a non-integral `decimals`) faults the ENTIRE fetch (no silent
    // partial list on a funds-adjacent table). On the picker path that fault is cushioned by the
    // serve-stale cache when a good list was already cached; a COLD first-open with a malformed
    // body surfaces the typed error (an honest "couldn't load," never a half-list).

    // one entry has fractional decimals (6.5) — the strict OpenAPI-integral check rejects it
    let mut malformed = String::from_utf8(tokens_body_for_filter()).expect("utf8");
    malformed = malformed.replace(
        r#""decimals":6,"blockchain":"near","symbol":"USDC""#,
        r#""decimals":6.5,"blockchain":"near","symbol":"USDC""#,
    );

    // (1) COLD first-open: nothing cached ⇒ the strict fault surfaces typed, no stale fallback.
    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(200, "OK", malformed.as_bytes()));
    let p = provider(dialer, None);
    match p.list_tokens().await {
        Err(SwapError::ProviderProtocol {
            reason: ProviderProtocolReason::MalformedAmount,
        }) => {}
        other => panic!("a malformed entry on a cold open must fault typed, got {other:?}"),
    }

    // (2) WARM cache then a malformed refresh ⇒ the fault is cushioned: the last good list is
    // served STALE, never the malformed partial and never a hard error.
    let dialer = Arc::new(ThenFailDialer::default());
    dialer.push(http_response(200, "OK", &tokens_body_for_filter())); // good first fetch
    dialer.push(http_response(200, "OK", malformed.as_bytes())); // malformed refresh
    let p = provider(dialer, None);
    let good = p.list_tokens().await.expect("first good list");
    assert!(good.fresh && good.tokens.len() == 1);
    let stale = p
        .list_tokens()
        .await
        .expect("malformed refresh is cushioned by the cache, not an error");
    assert!(
        !stale.fresh,
        "the malformed refresh faulted ⇒ served stale from the last good list"
    );
    assert_eq!(
        stale.tokens, good.tokens,
        "the served list is the LAST GOOD one, never the malformed body"
    );
}

#[tokio::test]
async fn list_tokens_refreshes_the_shared_cache_for_a_later_quote() {
    // The money-adjacent shared-cache interaction: a SUCCESSFUL `list_tokens` refreshes the
    // resolution cache (`resolve`'s table), so a subsequent `quote` resolves against THAT freshly
    // fetched table WITHOUT dialing `/v0/tokens` again. If `list_tokens` did not warm the cache, the
    // quote would dial tokens itself (3 dials total); warming it means the quote only dials /v0/quote
    // (2 dials total). Pin the exact wire economy AND that the quote still resolves correctly.
    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(200, "OK", &tokens_body())); // list_tokens fetch
    dialer.push(http_response(201, "Created", live_quote_body())); // quote — NO second tokens fetch
    let p = provider(dialer.clone(), None);

    let list = p.list_tokens().await.expect("list");
    assert!(list.fresh);
    assert_eq!(
        dialer.dials.load(Ordering::SeqCst),
        1,
        "list_tokens dialed once"
    );

    // the quote resolves USDC against the cache list_tokens just warmed — no tokens re-fetch
    let quote = p
        .quote(out_of_zec_request())
        .await
        .expect("quote off the warm cache");
    assert_eq!(
        quote.min_amount_out, "412.248474",
        "resolved against the warmed table"
    );
    assert_eq!(
        dialer.dials.load(Ordering::SeqCst),
        2,
        "exactly +1 dial (the quote) — list_tokens already warmed the resolution cache"
    );
    let paths: Vec<String> = dialer
        .requests()
        .iter()
        .map(|r| {
            String::from_utf8_lossy(r)
                .lines()
                .next()
                .unwrap_or("")
                .to_string()
        })
        .collect();
    assert!(
        paths[0].starts_with("GET /v0/tokens"),
        "first dial: list_tokens"
    );
    assert!(
        paths[1].starts_with("POST /v0/quote"),
        "second dial: the quote itself, NOT a redundant /v0/tokens"
    );
}

#[tokio::test]
async fn list_tokens_always_refetches_so_the_picker_is_always_fresh_online() {
    // `list_tokens` is NOT cache-first (unlike the quote/status `tokens_cached` path): it ALWAYS
    // re-fetches `/v0/tokens` so an online picker always shows the provider's current asset set
    // (prices and availability move). Two successful calls ⇒ two wire dials. This guards against a
    // future regression making the picker cache-first (a stale picker while online — wrong prices on
    // a funds-adjacent screen).
    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(200, "OK", &tokens_body_for_filter()));
    dialer.push(http_response(200, "OK", &tokens_body_for_filter()));
    let p = provider(dialer.clone(), None);

    let first = p.list_tokens().await.expect("first list");
    let second = p.list_tokens().await.expect("second list");
    assert!(
        first.fresh && second.fresh,
        "both fetches are fresh (online)"
    );
    assert_eq!(
        dialer.dials.load(Ordering::SeqCst),
        2,
        "every successful list_tokens hits the wire — the picker is never served cache-first online"
    );
    // both rode the dedicated tokens circuit, never linked to sync/quote
    let isolation = dialer.seen_isolation.lock().expect("isolation").clone();
    assert_eq!(
        isolation,
        vec![Some("swap/tokens".into()), Some("swap/tokens".into())],
        "each re-fetch rides the dedicated tokens circuit"
    );
}

#[tokio::test]
async fn list_tokens_keeps_both_duplicate_chain_symbol_rows_then_quote_rejects_ambiguous() {
    // The intentional split between the PICKER and RESOLUTION: `pick_tokens` does NOT dedup — two
    // listings sharing (chain, symbol) with distinct asset_ids BOTH appear in the picker (the host
    // shows what the provider offers). It is `resolve()` at QUOTE time that refuses the ambiguous
    // (chain, symbol) typed — guessing which "USDC" the user meant is not a funds-path option. Pin
    // BOTH halves so the no-dedup picker is intentional, not an accidental leak that funds math
    // would later trip on.
    let mut two = String::from_utf8(tokens_body()).expect("utf8");
    two.insert_str(
        two.rfind(']').expect("array"),
        r#",{"assetId":"nep141:other-usdc.near","decimals":6,"blockchain":"near","symbol":"USDC","price":1.0}"#,
    );

    // (1) the picker keeps BOTH USDC rows (distinct asset ids), no dedup.
    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(200, "OK", two.as_bytes()));
    let p = provider(dialer, None);
    let list = p.list_tokens().await.expect("list");
    let usdc_ids: Vec<&str> = list
        .tokens
        .iter()
        .filter(|t| t.chain == "near" && t.symbol == "USDC")
        .map(|t| t.provider_asset_id.as_str())
        .collect();
    assert_eq!(
        usdc_ids.len(),
        2,
        "both distinct-asset-id USDC listings survive into the picker (no dedup)"
    );
    assert!(
        usdc_ids
            .contains(&"nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1")
            && usdc_ids.contains(&"nep141:other-usdc.near"),
        "both asset ids are present, distinct"
    );

    // (2) a quote naming that ambiguous (chain, symbol) is refused typed — resolve() is the gate.
    let dialer = Arc::new(ScriptedDialer::default());
    dialer.push(http_response(200, "OK", two.as_bytes()));
    let p = provider(dialer, None);
    assert!(
        matches!(
            p.quote(out_of_zec_request()).await,
            Err(SwapError::RequestInvalid { .. })
        ),
        "the ambiguous asset is rejected at quote time (resolve), even though the picker showed both"
    );
}

// ── S8 `identity` — the adapter's leg of row 4, against the port shape the contract
// names (the provider address handed to `status` as DATA beside the SDK id). Cannot
// compile at the base commit (`status` takes the id alone there): PARSED, never BUILT
// — `#[cfg(any())]` is never true. The adjudicator flips it to `#[cfg(test)]` and
// adjusts the call spelling to the ruled port shape (declared in the test author's
// `contractFindings`). Validated against a throwaway definition of the shape.
#[cfg(test)]
mod s8_declared {
    use super::*;

    /// Two SDK identities under ONE provider address ride two DISTINCT isolation keys
    /// (the key derives from the SDK id, never from the address), one identity keeps
    /// its key across polls, and every status request still asks the provider for THAT
    /// address — never for the SDK id. The sibling
    /// `no_swap_id_byte_reaches_the_dialer_isolation_key` pins two keys for two
    /// addresses; this pins two keys for one.
    #[tokio::test]
    async fn two_sdk_ids_under_one_provider_address_ride_two_circuits_and_both_ask_for_that_address()
     {
        let address = "t1ZXKD285AZdKs56b9ypo9FT4LMHKuJRw5N";
        let (a, b) = (SwapId::new("sdk-identity-a"), SwapId::new("sdk-identity-b"));
        let status = include_bytes!("../tests/fixtures/derived/status_incomplete_deposit.json");
        let dialer = Arc::new(ScriptedDialer::default());
        dialer.push(http_response(200, "OK", status)); // status(a)
        dialer.push(http_response(200, "OK", &tokens_body())); // decimals lookup (once)
        dialer.push(http_response(200, "OK", status)); // status(b)
        dialer.push(http_response(200, "OK", status)); // status(a) again
        let p = provider(dialer.clone(), None);
        p.status(&a, address).await.expect("status a");
        p.status(&b, address).await.expect("status b");
        p.status(&a, address).await.expect("status a again");

        let seen = dialer.seen_isolation.lock().expect("isolation").clone();
        let keys: Vec<&str> = seen.iter().map(|k| k.as_deref().expect("keyed")).collect();
        assert_eq!(keys.len(), 4, "four dials: a, tokens, b, a");
        assert_eq!(
            keys[1], "swap/quote",
            "the resolution fill rides the quote bucket"
        );
        assert_ne!(
            keys[0], keys[2],
            "two SDK identities under one address never share a circuit"
        );
        assert_eq!(keys[0], keys[3], "one identity keeps its circuit");
        for key in [keys[0], keys[2]] {
            assert!(
                key.starts_with("swap/id/") && key.len() == "swap/id/".len() + 32,
                "the SDK-owned fixed-width namespace: {key}"
            );
            assert!(
                !key.contains(&address[..6]),
                "no address window in the key: {key}"
            );
        }
        let expected = format!("GET /v0/status?depositAddress={address} HTTP/1.1\r\n");
        for (i, req) in dialer.requests().iter().enumerate() {
            if i == 1 {
                continue; // the tokens fill
            }
            let head = String::from_utf8_lossy(req);
            assert!(
                head.starts_with(&expected),
                "every status request asks for the provider address, never the SDK id: {head}"
            );
        }
    }
}
