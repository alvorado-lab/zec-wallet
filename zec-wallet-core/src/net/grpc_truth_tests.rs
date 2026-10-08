//! The IMPLEMENTER's own drives of stage S1 `truth` — the not-carrying state's
//! predicate, its deadline and wakes, the F-C reading under `Required`, and the
//! counters' one writer. Not the item's named tests: those are the test
//! author's, written blind against the contract. Each one below was run to find
//! out whether the thing built does what its doc says, and is kept so the next
//! reader can run it too. A child of `net/grpc.rs` so it can reach that file's
//! private types; the fixtures are its own so the `window` probes' file stays
//! as its watches printed it.

use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;

use super::*;
use crate::config::TorRuntime;
use crate::constants::{TOR_PATIENCE_SECS, WALLET_SYNC_ISOLATION_KEY};
use crate::error::DialError;
use crate::net::dialer::resolve_dialer;
use crate::ports::NetDialer;
use crate::ports::testing::FailingDialer;
use crate::state::{ArmCounts, DialCounts};

/// Accepts every dial and never answers — the blackhole. Counts its dials.
#[derive(Default)]
struct SilentPeer {
    dials: AtomicUsize,
}

impl SilentPeer {
    fn dials(&self) -> usize {
        self.dials.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl NetDialer for SilentPeer {
    async fn dial(
        &self,
        _host: &str,
        _port: u16,
        _isolation_key: Option<&str>,
    ) -> Result<Box<dyn AsyncByteStream>, DialError> {
        use tokio::io::AsyncReadExt;
        self.dials.fetch_add(1, Ordering::Relaxed);
        let (client_end, mut server_end) = tokio::io::duplex(8192);
        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            while let Ok(n) = server_end.read(&mut buf).await {
                if n == 0 {
                    break;
                }
            }
        });
        Ok(Box::new(client_end))
    }
}

fn required_over(primary: Arc<dyn NetDialer>) -> TorPolicy {
    TorPolicy::Required {
        runtime: TorRuntime::Dialer(primary),
    }
}

fn secs(s: u64) -> Duration {
    Duration::from_secs(s)
}

fn now() -> tokio::time::Instant {
    tokio::time::Instant::now()
}

/// The predicate's two conjuncts and the deadline's origin: silence is the
/// UNION over every class that can carry, failing is a run SEEN to outlast the
/// window on a confirmable class, and the deadline is the one conjunct that
/// moves with the clock — the union silence. A run that starts late in a
/// session is measured from where IT started, never from `created`.
#[tokio::test(start_paused = true)]
async fn impl_probe_unanswered_is_union_silence_and_any_failing_run_from_the_first_failure() {
    let posture = TorPosture::new();
    assert!(!posture.is_unanswered());
    assert!(
        posture.next_unanswered_deadline().is_none(),
        "an idle class has no deadline"
    );

    // 200 s of idling, then the sync class starts failing.
    tokio::time::advance(secs(200)).await;
    posture.note_rpc_failure(PathClass::Sync, DialArm::Private);
    assert!(
        posture.next_unanswered_deadline().is_none(),
        "a run nobody has yet seen outlast the window names no deadline — no timer can \
         satisfy that conjunct, only a failure, and a failure wakes"
    );
    tokio::time::advance(secs(TOR_PATIENCE_SECS - 1)).await;
    posture.note_rpc_failure(PathClass::Sync, DialArm::Private);
    assert!(!posture.is_unanswered(), "one second short of the minute");
    tokio::time::advance(secs(1)).await;
    posture.note_rpc_failure(PathClass::Sync, DialArm::Private);
    assert!(
        posture.is_unanswered(),
        "a failure observed a full window after the run began — measured from the run, \
         not from creation"
    );
    assert!(
        posture.next_unanswered_deadline().is_none(),
        "already true: only evidence can end it, and evidence wakes"
    );

    // Something comes back on the MONEY class: the path carries, so the state
    // is not "nothing has come back" — whatever the sync run says. The sync run
    // is still open, so a deadline reappears, a minute after that success.
    posture.note_private_success(PathClass::Broadcast, DialArm::Private);
    assert!(
        !posture.is_unanswered(),
        "a sibling class confirming an RPC refutes the silence"
    );
    assert_eq!(
        posture.next_unanswered_deadline(),
        Some(now() + secs(TOR_PATIENCE_SECS)),
        "the union silence moved the deadline out"
    );
    tokio::time::advance(secs(TOR_PATIENCE_SECS)).await;
    assert!(posture.is_unanswered());
}

/// A wedged server on one class while a sibling confirms: the per-class
/// switch predicate is TRUE for the starved class (the `window` item's rule),
/// and the state predicate is FALSE — the two answer different questions.
#[tokio::test(start_paused = true)]
async fn impl_probe_a_wedged_class_while_a_sibling_confirms_is_not_unanswered() {
    let posture = TorPosture::new();
    posture.note_rpc_failure(PathClass::Broadcast, DialArm::Private);
    for _ in 0..30 {
        tokio::time::advance(secs(2)).await;
        posture.note_private_success(PathClass::Sync, DialArm::Private);
        posture.note_rpc_failure(PathClass::Broadcast, DialArm::Private);
    }
    assert!(
        posture.may_switch_to_direct(PathClass::Broadcast),
        "the starved send may leave (per class, the window's question)"
    );
    assert!(
        !posture.is_unanswered(),
        "but the PATH is carrying — sync confirmed two seconds ago"
    );
    assert_eq!(
        posture.next_unanswered_deadline(),
        Some(now() + secs(TOR_PATIENCE_SECS)),
        "a minute of union silence from now would make it so"
    );
}

/// The wakes: the 0 → non-zero edge, not every failure; the clear; the latch;
/// and NOT a success with no run open. A success at 59 s cancels the deadline.
/// The test-only force flag moves nothing a host reads.
#[tokio::test(start_paused = true)]
async fn impl_probe_the_posture_wakes_on_edges_and_a_success_inside_the_minute_cancels() {
    let posture = TorPosture::new();
    let mut wakes = posture.changes();
    let woke = |rx: &mut tokio::sync::watch::Receiver<u64>| {
        let changed = rx.has_changed().expect("the posture outlives this");
        rx.borrow_and_update();
        changed
    };

    posture.note_private_failure(PathClass::Sync);
    assert!(woke(&mut wakes), "the 0 → non-zero edge wakes");
    posture.note_private_failure(PathClass::Sync);
    assert!(
        !woke(&mut wakes),
        "a later failure of the same run is not an edge"
    );
    // A run on a class that cannot be `Unanswered` is no edge either.
    posture.note_private_failure(PathClass::Other);
    assert!(!woke(&mut wakes), "`Other` does not count toward the state");

    tokio::time::advance(secs(TOR_PATIENCE_SECS - 1)).await;
    posture.note_private_success(PathClass::Sync, DialArm::Private);
    assert!(woke(&mut wakes), "the clear wakes");
    assert!(
        posture.next_unanswered_deadline().is_none(),
        "a success at 59 s cancels the deadline"
    );
    tokio::time::advance(secs(10)).await;
    assert!(!posture.is_unanswered());

    posture.note_private_success(PathClass::Sync, DialArm::Private);
    assert!(
        !woke(&mut wakes),
        "a success with no run open changes nothing a host sees"
    );
    posture.note_private_success(PathClass::Sync, DialArm::Clearnet);
    assert!(!woke(&mut wakes), "a clearnet RPC is nobody's evidence");

    posture.latch_fell_back();
    assert!(woke(&mut wakes), "the latch wakes: FR-34's announcement");

    posture.spend_the_minute_for_test();
    assert!(
        !posture.is_unanswered(),
        "the force flag short-circuits `may_switch_to_direct` and nothing else"
    );
}

/// `Required` over an accepted-and-silent path, on the real client: every
/// timeout claims nothing about the path (the F-C correction), the run begins
/// at the first OBSERVED failure — the first RPC's timeout, thirty seconds in —
/// and the predicate turns true a minute after that, with each failed RPC
/// retiring its connection and nothing ever latched.
#[tokio::test(start_paused = true)]
async fn impl_probe_required_reads_the_minute_from_the_first_timeout_and_claims_nothing() {
    let endpoint = LightServerEndpoint::new("http://localhost:9067").expect("endpoint");
    let silent = Arc::new(SilentPeer::default());
    let posture = Arc::new(TorPosture::new());
    let mut client = LightwalletdClient::connect_plaintext_over_test_dialer(
        &endpoint,
        &required_over(Arc::clone(&silent) as Arc<dyn NetDialer>),
        Some(WALLET_SYNC_ISOLATION_KEY.to_owned()),
        &posture,
        None,
    )
    .expect("lazy connect never blocks");
    assert_eq!(client.stall_on_failure, StallReason::TorUnavailable);

    // The wallet idles 100 s before its first pass: no run, no deadline.
    tokio::time::advance(secs(100)).await;
    assert!(posture.next_unanswered_deadline().is_none());

    let r = client.get_latest_block().await;
    assert!(
        matches!(
            r,
            Err(GrpcError::Timeout {
                stall: StallReason::EndpointUnreachable
            })
        ),
        "a timeout under Required claims nothing about the path: {r:?}"
    );
    assert!(
        !posture.kept_failing_past_the_window(PathClass::Sync)
            && posture.next_unanswered_deadline().is_none(),
        "one observed failure opens the run and names no deadline — the state turns on the \
         failure that is seen past the window, which is the third RPC below"
    );
    assert!(!posture.is_unanswered());

    let _ = client.get_latest_block().await;
    assert!(!posture.is_unanswered(), "60 s in, 30 s of failing");
    let _ = client.get_latest_block().await;
    assert!(
        posture.is_unanswered(),
        "the third timeout lands on the deadline: 90 s in, 60 s of failing"
    );
    assert_eq!(
        silent.dials(),
        3,
        "each failed RPC retired its connection; the next one dialled afresh"
    );
    assert!(!posture.fell_back(), "Required never latches");
}

/// The counters follow the ARM that performed each dial and its outcome —
/// `Off`'s dials are clearnet ones, a `Required` refusal is a private one — and
/// a second object starts at zero.
#[tokio::test]
async fn impl_probe_the_counters_follow_the_arm_and_the_outcome_of_each_dial() {
    let counts = Arc::new(DialCounters::default());
    let posture = Arc::new(TorPosture::new());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback");
    let addr = listener.local_addr().expect("addr");
    let accept = tokio::spawn(async move { listener.accept().await.map(|_| ()) });
    let off = resolve_dialer(&TorPolicy::Off, &posture, &counts).expect("off resolves");
    off.dialer
        .dial(&addr.ip().to_string(), addr.port(), None)
        .await
        .expect("connects");
    let _ = accept.await;
    let snap = counts.snapshot();
    assert_eq!(snap.clearnet.connected, 1, "Off's dial is a clearnet one");
    assert_eq!(snap.private, ArmCounts::default(), "and no private one");

    let required = resolve_dialer(
        &required_over(Arc::new(FailingDialer) as Arc<dyn NetDialer>),
        &posture,
        &counts,
    )
    .expect("required resolves");
    assert!(matches!(
        required
            .dialer
            .dial("zec.example", 443, Some(WALLET_SYNC_ISOLATION_KEY))
            .await,
        Err(DialError::Unreachable)
    ));
    let snap = counts.snapshot();
    assert_eq!(
        snap.private.unreachable, 1,
        "the refusal is a private outcome"
    );
    assert_eq!(
        snap.clearnet.connected, 1,
        "the clearnet tally is untouched"
    );
    assert_eq!(
        snap.private.connected + snap.private.timeout + snap.private.io,
        0
    );

    assert_eq!(
        DialCounters::default().snapshot(),
        DialCounts::default(),
        "a second wallet's object starts at zero"
    );
}

// ── the ruling's repair: the reason is the CONNECTION's, keyed on its dial ────

/// What the private peer does with the next dial.
#[derive(Clone, Copy)]
enum Answer {
    /// Accepts and drains: the TLS handshake on it never completes, and the
    /// connector's own bound cuts it — a transport failure, not a timeout.
    AcceptAndStall,
    /// Accepts and hangs up at once: the handshake meets EOF.
    AcceptAndHangUp,
    /// Refuses the dial: no connection was made.
    Refuse,
}

/// A private peer that answers each dial from a script, in order.
struct ScriptedPeer {
    script: std::sync::Mutex<std::collections::VecDeque<Answer>>,
    dials: AtomicUsize,
}

impl ScriptedPeer {
    fn new(script: impl IntoIterator<Item = Answer>) -> Arc<Self> {
        Arc::new(Self {
            script: std::sync::Mutex::new(script.into_iter().collect()),
            dials: AtomicUsize::new(0),
        })
    }

    fn dials(&self) -> usize {
        self.dials.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl NetDialer for ScriptedPeer {
    async fn dial(
        &self,
        _host: &str,
        _port: u16,
        _isolation_key: Option<&str>,
    ) -> Result<Box<dyn AsyncByteStream>, DialError> {
        use tokio::io::AsyncReadExt;
        self.dials.fetch_add(1, Ordering::Relaxed);
        let answer = self
            .script
            .lock()
            .expect("the script is never poisoned")
            .pop_front()
            .expect("the script ran out of answers");
        match answer {
            Answer::AcceptAndStall => {
                let (client_end, mut server_end) = tokio::io::duplex(8192);
                tokio::spawn(async move {
                    let mut buf = [0u8; 1024];
                    while let Ok(n) = server_end.read(&mut buf).await {
                        if n == 0 {
                            break;
                        }
                    }
                });
                Ok(Box::new(client_end))
            }
            Answer::AcceptAndHangUp => {
                let (client_end, _server_end) = tokio::io::duplex(64);
                Ok(Box::new(client_end))
            }
            Answer::Refuse => Err(DialError::Unreachable),
        }
    }
}

/// THE WITNESS GAP — the fold's second finding. A stream OPEN that comes back
/// a transport failure is the private path not carrying, exactly as a unary
/// one is, and until this fold only the unary arm told the witness. The two
/// stream opens and the stream pump classified the same statuses with the same
/// classifier and recorded nothing.
///
/// What that left invisible is not exotic: a server that answers the pass-head
/// unary and RESETS every stream. The unary CONFIRMS and clears the failing
/// run, every reset records nothing, `first_failure_ms` never reaches the
/// minute — so a `Preferred` wallet never switches, a `Required` wallet never
/// reads the value, and the state holds `Active` for ever while the wallet
/// never syncs. That is the silent stall under a UI reading fine that the exit
/// gate exists to close.
///
/// Driven at the OPEN, which is the arm a duplex fixture can reach; the pump
/// and the subtree-roots open take the same one-line guard and are pinned by
/// `every_classified_status_in_the_grpc_seam_tells_the_witness` below, which
/// is where a fifth site would be caught.
#[tokio::test(start_paused = true)]
async fn a_stream_open_that_fails_transport_is_evidence_the_private_path_carried_nothing() {
    let endpoint = LightServerEndpoint::new("http://localhost:9067").expect("endpoint");
    // Accept every dial and drop the far end at once: the h2 connection dies,
    // which is a TRANSPORT status on an ACCEPTED connection — the shape that
    // recorded nothing.
    let peer = ScriptedPeer::new([Answer::AcceptAndHangUp; 3]);
    let posture = Arc::new(TorPosture::new());
    let mut client = LightwalletdClient::connect_plaintext_over_test_dialer(
        &endpoint,
        &required_over(Arc::clone(&peer) as Arc<dyn NetDialer>),
        Some(WALLET_SYNC_ISOLATION_KEY.to_owned()),
        &posture,
        None,
    )
    .expect("lazy connect never blocks");

    // (`MessageStream` is not `Debug`, so the outcome is reduced to its error
    // before it is asserted on.)
    let opened = client
        .get_block_range(2_000_000, 2_000_010)
        .await
        .err()
        .expect("a dropped far end cannot serve a stream");
    assert!(
        matches!(opened, GrpcError::Transport { .. }),
        "fixture: the open comes back a transport failure, not a server status: {opened:?}"
    );
    assert!(
        posture.next_unanswered_deadline().is_none(),
        "one failure is not yet a minute of trying"
    );

    // A minute later the wallet tries again and the path still carries
    // nothing. On the unrepaired seam neither open was evidence, so there was
    // no run at all and this read `false` for ever.
    tokio::time::advance(secs(TOR_PATIENCE_SECS)).await;
    let again = client
        .get_block_range(2_000_000, 2_000_010)
        .await
        .err()
        .expect("still nothing to serve it");
    assert!(matches!(again, GrpcError::Transport { .. }), "{again:?}");
    assert!(
        posture.is_unanswered(),
        "a minute of stream opens that carried nothing IS the private path not carrying — \
         the run began at the first open and was seen to outlast the window at the second"
    );
    assert!(!posture.fell_back(), "Required never latches");
}

/// The STATIC complement of the row above, and the reason it exists: the
/// witness's evidence was lost on three of four surfaces because each site
/// open-coded the rule, and three of them forgot. A policy enforced only where
/// a fixture can drive it has never met most of its surface — the pump in
/// particular has no fixture in this tree, since nothing here serves a live h2
/// stream.
///
/// So: every site in the seam that classifies a status FOR A WITNESSED
/// CONNECTION must hand that verdict to the witness within the same arm. A
/// site is a call to one of the three classifiers that reaches `transport_stall`
/// — the function that resolves a stall against the connection's LAST dial, and
/// therefore the mark of exactly the calls a witness is in scope for. The stall
/// is matched within a two-line window rather than on the classifier's own
/// line, because `rustfmt` puts it on the next line when the call is long
/// enough (it does, for the subtree-roots open) and a one-line needle silently
/// stopped counting that site. What this does NOT catch, stated rather than
/// implied: a fifth site that resolves its stall some other way. It catches
/// the regression that actually happened — a new RPC surface copied from an
/// existing arm — and the count makes an addition visible.
#[test]
fn every_classified_status_in_the_grpc_seam_tells_the_witness() {
    let seam = include_str!("grpc.rs");
    let lines: Vec<&str> = seam.lines().collect();
    let sites: Vec<(usize, &str)> = lines
        .iter()
        .enumerate()
        .filter(|(i, line)| {
            // The unary arm and the block-range open: rustfmt keeps these on
            // one line. The pump names its classifier through the trait, which
            // nothing else in the file does. The subtree-roots open is the one
            // rustfmt wraps, so its stall is matched in a two-line window —
            // and that window is also what keeps the classifier's own
            // definition and `SubtreeRoot`'s trait impl (neither of which has a
            // `transport_stall` near it) out of the set.
            line.contains("classify_status(transport_stall(")
                || line.contains("T::classify_stream_status(")
                || (line.contains("classify_subtree_roots_status(")
                    && !line.contains("fn ")
                    && lines[*i..(*i + 2).min(lines.len())]
                        .iter()
                        .any(|l| l.contains("transport_stall(")))
        })
        .map(|(i, line)| (i + 1, line.trim()))
        .collect();
    assert_eq!(
        sites.len(),
        4,
        "the seam classifies a witnessed status at exactly four sites — the unary arm, \
         both stream OPENS and the pump. A new one is a new surface for the witness to be \
         blind on: {sites:?}"
    );
    for (number, text) in sites {
        // Within the arm: the classifier's result is bound, and handed over
        // before the `Err` is returned. Eight lines, because the pump's arm
        // carries a three-line comment between the two statements.
        let arm = lines[number..(number + 8).min(lines.len())].join("\n");
        assert!(
            arm.contains("note_if_transport("),
            // `grpc.rs line {n}`, never `grpc.rs:{n}` — the registry's
            // cited-lines gate reads a `file.rs:N` in a quoted panic as a
            // CITATION and refuses the row when that line holds no assertion
            // (the P28 source scans print their finds the same way).
            "grpc.rs line {number} classifies a status and does not tell the witness: \
             {text}. A TRANSPORT failure there is the private path not carrying, and a \
             posture that never hears it holds `Active` while the wallet never syncs"
        );
    }
}

/// `Required` over `https` — the only scheme the door admits under a host
/// runtime — on ONE client through the real door: an accepted dial that never
/// completes TLS is cut by the connector's bound as a TRANSPORT failure inside
/// the unary budget, an accept-then-close is one at once, and both carry the
/// far end's word; the refused dial after them, on the same client, keeps the
/// plan's. The failure clock starts at the first cut, and nothing latches.
#[tokio::test(start_paused = true)]
async fn impl_probe_required_over_https_blames_the_far_end_only_while_the_dial_is_accepted() {
    let endpoint = LightServerEndpoint::new("https://localhost:9067").expect("endpoint");
    let peer = ScriptedPeer::new([
        Answer::AcceptAndStall,
        Answer::AcceptAndHangUp,
        Answer::Refuse,
    ]);
    let posture = Arc::new(TorPosture::new());
    let mut client = LightwalletdClient::connect(
        &endpoint,
        &required_over(Arc::clone(&peer) as Arc<dyn NetDialer>),
        Some(WALLET_SYNC_ISOLATION_KEY.to_owned()),
        &posture,
        &Arc::new(DialCounters::default()),
        None,
    )
    .expect("the door admits https under a host runtime; the channel is lazy");
    assert_eq!(
        client.stall_on_failure,
        StallReason::TorUnavailable,
        "fixture: the plan's reason is the fail-closed one"
    );

    let start = now();
    let r = client.get_latest_block().await;
    let waited = start.elapsed();
    assert!(
        matches!(
            r,
            Err(GrpcError::Transport {
                stall: StallReason::EndpointUnreachable
            })
        ),
        "an accepted dial whose TLS never completes is a TRANSPORT failure — the \
         connector's bound, not the RPC timer — and it is the far end's: {r:?}"
    );
    assert!(
        waited >= secs(DIAL_TIMEOUT_SECS) && waited < secs(GRPC_UNARY_TIMEOUT_SECS),
        "cut by the connector's handshake bound, before the unary timer: {waited:?}"
    );
    assert!(
        !posture.kept_failing_past_the_window(PathClass::Sync)
            && posture.next_unanswered_deadline().is_none(),
        "the run began at the first observed failure — the TLS cut — and one failure is \
         not yet a minute of the wallet trying"
    );

    let r = client.get_latest_block().await;
    assert!(
        matches!(
            r,
            Err(GrpcError::Transport {
                stall: StallReason::EndpointUnreachable
            })
        ),
        "a connection that was made and then hung up is the far end's too: {r:?}"
    );

    let r = client.get_latest_block().await;
    assert!(
        matches!(
            r,
            Err(GrpcError::Transport {
                stall: StallReason::TorUnavailable
            })
        ),
        "the control, on the same client: a dial the path REFUSED keeps the plan's word: {r:?}"
    );
    assert_eq!(
        peer.dials(),
        3,
        "each failed RPC over an accepted connection retired it; the refusal dialled afresh"
    );
    assert!(!posture.fell_back(), "Required never latches");
}

/// The verdict is the connection's LAST dial. tonic redials a dead connection
/// through the same connector, so a redial that FAILS must shed the accept
/// before it — or a `Required` wallet whose path died between passes would
/// read the refusal as the server's fault. Driven at the connector, where the
/// verdict is written.
#[tokio::test]
async fn impl_probe_a_redial_that_fails_sheds_the_accept_before_it() {
    let peer = ScriptedPeer::new([Answer::AcceptAndStall, Answer::Refuse]);
    let posture = Arc::new(TorPosture::new());
    let connection: Arc<ConnectionState> = Arc::default();
    let witness = PrivateRpcWitness {
        posture: Arc::clone(&posture),
        class: PathClass::Sync,
        connection: Arc::clone(&connection),
    };
    let mut connector = DialerConnector {
        dialer: Arc::new(crate::net::dialer::PolicyDialer::fail_closed(
            Arc::clone(&peer) as Arc<dyn NetDialer>,
            posture,
            Default::default(),
        )),
        // Plaintext: the verdict is written before TLS either way, and this
        // probe is about the dial, not the handshake.
        tls: None,
        host: "localhost".into(),
        port: 9067,
        isolation_key: Some(WALLET_SYNC_ISOLATION_KEY.to_owned()),
        connection: Arc::clone(&connection),
    };
    let uri = Uri::from_static("http://localhost:9067");
    let plan = StallReason::TorUnavailable;

    assert!(!connection.dial_accepted(), "no dial yet");
    assert_eq!(
        transport_stall(plan, Some(&witness)),
        plan,
        "with no accepted dial behind it, a transport failure is the plan's"
    );

    tower::Service::call(&mut connector, uri.clone())
        .await
        .expect("the first dial is accepted");
    assert!(connection.dial_accepted());
    assert_eq!(
        transport_stall(plan, Some(&witness)),
        StallReason::EndpointUnreachable,
        "over an accepted dial the failure is the far end's"
    );

    assert!(
        tower::Service::call(&mut connector, uri).await.is_err(),
        "the redial is refused"
    );
    assert!(
        !connection.dial_accepted(),
        "the refused redial shed the accept before it"
    );
    assert_eq!(
        transport_stall(plan, Some(&witness)),
        plan,
        "and the failure it surfaces is the plan's again"
    );
    assert_eq!(peer.dials(), 2);
    assert_eq!(
        transport_stall(plan, None),
        plan,
        "no witness is an `Off` plan, whose reason claims nothing to begin with"
    );
}
