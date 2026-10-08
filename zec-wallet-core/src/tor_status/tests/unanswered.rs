//! Stage S1 `truth` — the derivation's fifth input, `unanswered`
//! (`stage-1-private-path-truth.md` §3.2 "the state machine", §3.2a). The
//! TEST AUTHOR's rows, written blind against the contract (IT-2a). A child of
//! `tor_status.rs`'s test module so it reaches the strategies and fixtures
//! there; its own file so the cited lines above stay where their watches
//! printed them.
//!
//! ONE property carries the whole precedence: `unanswered` turns EXACTLY the
//! `Active` reading into `Unanswered`, with the same runtime payload, and
//! moves nothing else. `Off` stays `Off`; a latched leak stays `FellBack`
//! (the contract's named row); the host's declaration — cleared, FAILED, or
//! bootstrapping — keeps its arm; and the fail-closed stall keeps
//! `Unavailable`, because a refused dial is a claim the evidence CAN make and
//! it outranks the either/or one (§3.2 "`Required`, two cases").

use super::*;

/// The real derivation, by its crate path: `super::live_tor_state` is the
/// parent module's three-input shadow. `path_refused` is the sixth input (the
/// fold): these rows pass `false` for it unless they are about it, because it
/// is `unanswered` REFINED — true only where this value would otherwise be —
/// and every row below that passes `unanswered = true` alone is the case where
/// the SDK holds no evidence about the dial.
fn derive(
    policy: &TorPolicy,
    fell_back: bool,
    sync: &SyncStatus,
    host: Option<HostTransportDescriptor>,
    unanswered: bool,
) -> TorState {
    crate::tor_status::live_tor_state(policy, fell_back, sync, host, unanswered, false)
}

/// The same, with the fail-closed arm's posture-sourced input set: the failing
/// run that carried the path into the value was OPENED by a dial the path
/// REFUSED.
fn derive_refused(
    policy: &TorPolicy,
    fell_back: bool,
    sync: &SyncStatus,
    host: Option<HostTransportDescriptor>,
    unanswered: bool,
) -> TorState {
    crate::tor_status::live_tor_state(policy, fell_back, sync, host, unanswered, true)
}

/// What the derivation must read with `unanswered = true`, as a function of
/// what it reads without it.
fn expected_with_unanswered(base: TorState) -> TorState {
    match base {
        TorState::Active { runtime } => TorState::Unanswered { runtime },
        other => other,
    }
}

fn socks_runtime() -> TorRuntime {
    TorRuntime::ExternalSocks5 {
        addr: "127.0.0.1:9050".into(),
    }
}

/// The precedence, as a table over every policy shape and every sync
/// variant: the new value appears exactly where `Active` did, carrying the
/// same runtime, and nowhere else.
#[test]
fn unanswered_reads_the_new_value_exactly_where_the_base_read_active() {
    let policies = [
        TorPolicy::Off,
        TorPolicy::Preferred {
            runtime: dialer_runtime(),
        },
        TorPolicy::Required {
            runtime: dialer_runtime(),
        },
        TorPolicy::Preferred {
            runtime: socks_runtime(),
        },
        TorPolicy::Required {
            runtime: socks_runtime(),
        },
    ];
    let syncs = [
        SyncStatus::Idle,
        up_to_date(),
        stalled(StallReason::EndpointUnreachable),
        stalled(StallReason::TorUnavailable),
        stalled(StallReason::Internal),
        SyncStatus::Connecting {
            tor_bootstrap_percent: Some(0.5),
        },
        SyncStatus::Offline { last_synced: None },
    ];
    for (p, policy) in policies.iter().enumerate() {
        for fell_back in [false, true] {
            for sync in &syncs {
                let base = derive(policy, fell_back, sync, None, false);
                let with = derive(policy, fell_back, sync, None, true);
                assert_eq!(
                    with,
                    expected_with_unanswered(base.clone()),
                    "policy #{p} fell_back={fell_back} sync={sync:?}: not-carrying moves \
                     exactly the Active reading (base read {base:?})"
                );
            }
        }
    }
    // The positive shape spelled out once, so the table cannot pass on a
    // derivation that never reads `Active` at all.
    assert_eq!(
        derive(
            &TorPolicy::Required {
                runtime: dialer_runtime()
            },
            false,
            &stalled(StallReason::EndpointUnreachable),
            None,
            true
        ),
        TorState::Unanswered {
            runtime: TorRuntimeKind::Dialer
        },
        "Required, the endpoint stalled, and nothing has come back for the minute: the \
         either/or value, naming the runtime as Active would"
    );
}

/// The host's declaration outranks the value (the host is TRUSTED, ADR-0545,
/// and its arms sit above the new one in the precedence), and a READY
/// transport names itself on it exactly as it does on `Active` — so a host
/// renders the same transport under a different sentence.
#[test]
fn the_hosts_declaration_outranks_unanswered_and_a_ready_transport_names_itself_on_it() {
    let host = Arc::new(ScriptedHostDialer::ready());
    for policy in [
        TorPolicy::Preferred {
            runtime: host_runtime(&host),
        },
        TorPolicy::Required {
            runtime: host_runtime(&host),
        },
    ] {
        let read = |sync: &SyncStatus| derive(&policy, false, sync, policy.host_descriptor(), true);
        host.set_descriptor(Some(ScriptedHostDialer::tor_ready()));
        assert_eq!(
            read(&SyncStatus::Idle),
            TorState::Unanswered {
                runtime: TorRuntimeKind::HostDialer {
                    name: name("Tor"),
                    isolation: IsolationSupport::Supported,
                    exposure: TransportExposure::Hidden,
                }
            },
            "ready and not carrying: the payload Active carries — name, isolation, exposure"
        );
        host.set_descriptor(Some(descriptor(
            "Tor",
            40,
            IsolationSupport::Supported,
            TransportExposure::Hidden,
        )));
        assert_eq!(
            read(&SyncStatus::Idle),
            TorState::Bootstrapping {
                percent: Some(0.4),
                transport: Some(name("Tor"))
            },
            "a bootstrap in progress is not a silent path"
        );
        host.set_descriptor(Some(HostTransportDescriptor {
            name: name("Tor"),
            readiness: HOST_TRANSPORT_READY,
            isolation: IsolationSupport::Supported,
            exposure: TransportExposure::Hidden,
            health: TransportHealth::Failed,
        }));
        assert_eq!(
            read(&up_to_date()),
            TorState::Unavailable {
                transport: Some(name("Tor"))
            },
            "the host declared its transport FAILED: that claim outranks ours"
        );
        host.set_descriptor(None);
        assert_eq!(
            read(&SyncStatus::Idle),
            TorState::Unavailable { transport: None },
            "nothing registered: no path exists to be unanswered"
        );
        host.set_descriptor(Some(ScriptedHostDialer::tor_ready()));
        assert_eq!(
            derive(
                &policy,
                true,
                &SyncStatus::Idle,
                policy.host_descriptor(),
                true
            ),
            TorState::FellBack,
            "a latched leak outranks everything, the new value included"
        );
    }
}

/// The fold's fourth finding, at the derivation: a run the path REFUSED to
/// take reads `Unavailable`, never the either/or value — "the path or the
/// server" is the sentence for a path that ACCEPTED and went quiet, and a
/// refusal is evidence that separates the two. Everything ABOVE the
/// fail-closed arm keeps its reading, so the new input re-words exactly the
/// two arms below it and nothing else.
///
/// Why this is a derivation row and not only a posture one: the arm existed
/// and was UNREACHABLE for every class but `Sync`, because its only input was
/// `SyncStatus` and the sync class alone writes that. A `Required` wallet
/// whose BROADCAST dial was refused before the sync loop started published the
/// either/or value about a path that was provably refusing connections.
#[test]
fn a_refused_dial_reads_unavailable_where_the_either_or_value_would_have_been() {
    let policies = [
        TorPolicy::Off,
        TorPolicy::Preferred {
            runtime: dialer_runtime(),
        },
        TorPolicy::Required {
            runtime: dialer_runtime(),
        },
        TorPolicy::Preferred {
            runtime: socks_runtime(),
        },
        TorPolicy::Required {
            runtime: socks_runtime(),
        },
    ];
    let syncs = [
        SyncStatus::Idle,
        up_to_date(),
        stalled(StallReason::EndpointUnreachable),
        stalled(StallReason::TorUnavailable),
        SyncStatus::Connecting {
            tor_bootstrap_percent: Some(0.5),
        },
    ];
    for (p, policy) in policies.iter().enumerate() {
        for fell_back in [false, true] {
            for sync in &syncs {
                let base = derive(policy, fell_back, sync, None, true);
                let refused = derive_refused(policy, fell_back, sync, None, true);
                let expected = match base.clone() {
                    TorState::Active { .. } | TorState::Unanswered { .. } => {
                        TorState::Unavailable { transport: None }
                    }
                    other => other,
                };
                assert_eq!(
                    refused, expected,
                    "policy #{p} fell_back={fell_back} sync={sync:?}: a refused dial re-words \
                     exactly the readings below the fail-closed arm (base read {base:?})"
                );
            }
        }
    }
    // The positive shape spelled out once: the finding's own row — `Required`,
    // nothing stalled on the sync axis because no sync pass has run, and the
    // broadcast dial refused.
    assert_eq!(
        derive_refused(
            &TorPolicy::Required {
                runtime: dialer_runtime()
            },
            false,
            &SyncStatus::Idle,
            None,
            true
        ),
        TorState::Unavailable { transport: None },
        "a send before the sync loop starts, its dial refused: the path is down, and the \
         SDK says so instead of offering the user a choice between two causes"
    );
    // …and the input is not a second trigger. Where the path is CARRYING, a
    // stale refusal cannot claim otherwise — the posture is what guarantees
    // this combination never arises, and the derivation stays faithful below.
    assert_eq!(
        derive(
            &TorPolicy::Required {
                runtime: dialer_runtime()
            },
            false,
            &SyncStatus::Idle,
            None,
            false
        ),
        TorState::Active {
            runtime: TorRuntimeKind::Dialer
        },
        "no refusal, nothing silent: the ordinary reading is untouched"
    );
}

/// The failing arms carry the registrant's own name (FR-30 (a)), and the new
/// producer of `Unavailable` is no exception — a host that registered
/// "Shadowsocks" never reads the noun "Tor" because a dial was refused.
#[test]
fn a_refused_dial_names_the_registered_transport_and_yields_to_the_hosts_declaration() {
    let host = Arc::new(ScriptedHostDialer::ready());
    for policy in [
        TorPolicy::Preferred {
            runtime: host_runtime(&host),
        },
        TorPolicy::Required {
            runtime: host_runtime(&host),
        },
    ] {
        host.set_descriptor(Some(descriptor(
            "Shadowsocks",
            HOST_TRANSPORT_READY,
            IsolationSupport::Unsupported,
            TransportExposure::Hidden,
        )));
        assert_eq!(
            derive_refused(
                &policy,
                false,
                &SyncStatus::Idle,
                policy.host_descriptor(),
                true
            ),
            TorState::Unavailable {
                transport: Some(name("Shadowsocks"))
            },
            "the fail-closed arm names what the host registered, from either producer"
        );
        // A latched leak still outranks it, as it outranks everything.
        assert_eq!(
            derive_refused(
                &policy,
                true,
                &SyncStatus::Idle,
                policy.host_descriptor(),
                true
            ),
            TorState::FellBack,
        );
        // So does the host's own declaration — here a bootstrap in progress,
        // which is not a path that refused anything.
        host.set_descriptor(Some(descriptor(
            "Shadowsocks",
            40,
            IsolationSupport::Unsupported,
            TransportExposure::Hidden,
        )));
        assert_eq!(
            derive_refused(
                &policy,
                false,
                &SyncStatus::Idle,
                policy.host_descriptor(),
                true
            ),
            TorState::Bootstrapping {
                percent: Some(0.4),
                transport: Some(name("Shadowsocks"))
            },
            "the host is TRUSTED (ADR-0545): its declaration sits above both producers"
        );
        host.set_descriptor(Some(ScriptedHostDialer::tor_ready()));
    }
}

proptest::proptest! {
    /// The base's precedence proptest, extended over the new input across the
    /// WHOLE space: `unanswered` moves exactly the `Active` reading — which
    /// includes the contract's named row, `fell_back ∧ not-carrying → FellBack`
    /// — and a returned `Unanswered` names the configured runtime, never
    /// appears while leaking, and never under `Off`.
    #[test]
    fn unanswered_moves_only_the_active_reading_over_the_whole_input_space(
        policy_kind in 0u8..3,
        rt_kind in 0u8..2,
        fell_back in proptest::prelude::any::<bool>(),
        sync in arb_sync(),
    ) {
        let policy = match policy_kind {
            0 => TorPolicy::Off,
            1 => TorPolicy::Preferred { runtime: runtime_of(rt_kind) },
            _ => TorPolicy::Required { runtime: runtime_of(rt_kind) },
        };
        let base = derive(&policy, fell_back, &sync, None, false);
        let with = derive(&policy, fell_back, &sync, None, true);
        proptest::prop_assert_eq!(with.clone(), expected_with_unanswered(base));
        if fell_back && policy_kind != 0 {
            proptest::prop_assert_eq!(with.clone(), TorState::FellBack);
        }
        if let TorState::Unanswered { runtime } = &with {
            proptest::prop_assert_eq!(*runtime, expected_kind(rt_kind));
            proptest::prop_assert!(!fell_back);
            proptest::prop_assert!(policy_kind != 0);
        }
    }

    /// `live_tor_state_is_blind_to_preferred_vs_required`, extended: neither
    /// new input is a policy branch either — "`Required`-only" must never
    /// become one inside the derivation (§3.2 "Precedence"). `path_refused` is
    /// swept here as well as `unanswered`, because a refused dial is the input
    /// a policy branch would be most tempting on: it is `Required`'s
    /// fail-closed word, and it is still derived identically for `Preferred`
    /// (where the latch, when it comes, outranks it anyway).
    #[test]
    fn unanswered_is_blind_to_preferred_vs_required(
        rt_kind in 0u8..2,
        fell_back in proptest::prelude::any::<bool>(),
        unanswered in proptest::prelude::any::<bool>(),
        path_refused in proptest::prelude::any::<bool>(),
        sync in arb_sync(),
    ) {
        let pref = TorPolicy::Preferred { runtime: runtime_of(rt_kind) };
        let req = TorPolicy::Required { runtime: runtime_of(rt_kind) };
        proptest::prop_assert_eq!(
            crate::tor_status::live_tor_state(&pref, fell_back, &sync, None, unanswered, path_refused),
            crate::tor_status::live_tor_state(&req, fell_back, &sync, None, unanswered, path_refused),
        );
    }

    /// The honesty invariant the base proptest states for the fail-closed arm,
    /// carried onto its SECOND producer: `Unavailable` over this space needs
    /// the `TorUnavailable` stall OR a refused dial, and nothing else ever
    /// produces it. Without this the new input could widen the most
    /// safety-loaded arm in the derivation and no property would notice.
    #[test]
    fn unavailable_still_needs_the_stall_or_a_refused_dial(
        policy_kind in 1u8..3,
        rt_kind in 0u8..2,
        unanswered in proptest::prelude::any::<bool>(),
        path_refused in proptest::prelude::any::<bool>(),
        sync in arb_sync(),
    ) {
        let policy = match policy_kind {
            1 => TorPolicy::Preferred { runtime: runtime_of(rt_kind) },
            _ => TorPolicy::Required { runtime: runtime_of(rt_kind) },
        };
        let out = crate::tor_status::live_tor_state(
            &policy, false, &sync, None, unanswered, path_refused,
        );
        if let TorState::Unavailable { transport } = &out {
            let stalled_closed = matches!(
                sync,
                SyncStatus::Stalled { reason: StallReason::TorUnavailable }
            );
            proptest::prop_assert!(stalled_closed || path_refused);
            proptest::prop_assert!(transport.is_none());
        }
        // …and the converse for the new input alone: a refused dial ALWAYS
        // reaches the arm unless something above it speaks first.
        if path_refused && !matches!(sync, SyncStatus::Connecting { .. }) {
            proptest::prop_assert_eq!(out, TorState::Unavailable { transport: None });
        }
    }
}
