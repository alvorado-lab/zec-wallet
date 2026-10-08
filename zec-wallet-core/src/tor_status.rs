//! Live `TorState` derivation (§2.5 / review G1): turn the configured `TorPolicy`
//! together with the live flow signals (the dialer's fell-back flag and the
//! current `SyncStatus`) into the cold-queryable [`TorState`] the host's
//! network-transparency panel reads — WITHOUT replaying event history.
//!
//! In `Dialer` mode (the production path, ADR-0526) the SDK reports POLICY + FLOW
//! only — torness ATTESTATION belongs to the host's transport. The `Active`
//! variant names which runtime is speaking so a panel can attribute the claim
//! honestly (§2.3 trust boundary): `Active { Dialer }` means "the configured path
//! is the host's Dialer and nothing has degraded," and the host's own panel adds
//! the real onion-routing attestation. `Bootstrapping` is reached through a
//! REGISTERED dialer's descriptor (readiness < 100 — the `zec_wallet_tor`
//! plugin's arti bootstrap arrives this way, as does any host's); a
//! `Dialer`/`ExternalSocks5` runtime hands the SDK no bootstrap progress, so the
//! SDK never fabricates one. The failing arms carry the registrant's own name
//! for its transport (FR-30 (a)) — `None` where nothing is registered, which the
//! UI renders with its transport-neutral noun.
//!
//! Pure + total over the inputs — the whole policy×flow matrix is table-tested.

use crate::config::{TorPolicy, TorRuntime};
use crate::net::host_dialer::HostTransportDescriptor;
use crate::state::{StallReason, SyncStatus, TorRuntimeKind, TorState};

/// Derive the honest live [`TorState`] from the configured policy + the two live
/// flow signals. Decision order — **a CURRENT clearnet leak outranks everything**,
/// then the most-current connectivity signal:
///
/// 1. `Off` policy → [`TorState::Off`] (by configuration; no live signal consulted —
///    a clearnet-by-choice wallet NEVER claims any Tor state).
/// 2. `fell_back` set → [`TorState::FellBack`] (a `Preferred` runtime has VISIBLY
///    degraded to clearnet; only `Preferred` ever sets it). Checked FIRST among the
///    live signals: a fall-back means clearnet traffic is flowing NOW, the most
///    safety-critical truth — it must never be masked by a concurrent "still
///    bootstrapping" (which would falsely imply nothing has leaked).
/// 3. `SyncStatus::Connecting` (not yet fallen back) → [`TorState::Bootstrapping`]
///    with the surfaced progress. NO PRODUCER since FR-5 C1 removed the SDK-owned
///    runtime — a transport's bootstrap arrives through the descriptor instead
///    (the arm above); this stays a forward seam, and the ordering is pinned.
/// 4. `Stalled { TorUnavailable }` **or `path_refused`** → [`TorState::Unavailable`]
///    (a `Required` runtime —
///    or an unsupported runtime, which `map_net_err` reports as `TorUnavailable` —
///    is fail-closed: zero clearnet, the Tor path could not be established. Since
///    stage S1 `truth` a DIAL that FAILED — refused, unreachable, timed out at the
///    dial, a bootstrap not ready, a retired or FAILED descriptor — is the only
///    thing that stamps this reason. Over a connection the path ACCEPTED nothing
///    does: not a timeout (`net/grpc.rs`'s `TIMEOUT_STALL`) and not a transport
///    failure after the accept — a TLS handshake cut by the connector's bound, a
///    peer that hangs up, a link that dies mid-RPC (`transport_stall`, resolved
///    against the connection's last dial) — those carry `EndpointUnreachable`
///    and read `Active` here, then `Unanswered` at the minute).
///    **`path_refused` is the SAME claim from the other producer.** The stall
///    reason is written by the SYNC class alone, so a `Required` wallet whose
///    BROADCAST dial was refused before the sync loop ever ran — a send on a
///    freshly-opened wallet, status `Idle` — reached the minute and published
///    the either/or value about a path that was provably refusing connections.
///    `TorPosture::path_refused` is posture-wide and carries the same evidence:
///    a confirmable class whose failing run was OPENED by a dial that failed,
///    silent and seen failing past the window. It is `unanswered` refined, so
///    it only ever re-words a reading this arm would otherwise hand on, and it
///    is as blind to `Preferred` vs `Required` as everything else here.
/// 5. `unanswered` → [`TorState::Unanswered`] naming the configured runtime (stage
///    S1 `truth`, FR-36): the path is ready as far as anything above can tell, nothing
///    has carried an RPC over it for the maintainer's minute, and a confirmable class has
///    been SEEN to go on failing across that minute (`TorPosture::is_unanswered` — an
///    observation, so a wallet that stopped trying is never reported on). The
///    either/or value — the path OR the server —
///    and it is derived for `Required` too: a fail-closed wallet has no switch to
///    make the silence visible, so this is the one place it becomes so. Ranked
///    BELOW every arm that has EVIDENCE the path is down (a refused dial, a
///    cleared or FAILED descriptor) — those know more than a timeout does — and
///    above `Active`.
/// 6. Otherwise → [`TorState::Active`] naming the configured runtime. NB a `Required`
///    wallet whose endpoint merely WEDGES surfaces `Stalled { EndpointUnreachable }`
///    (the path was established; the endpoint stopped delivering) and stays `Active`
///    here until the minute is spent — the two axes are orthogonal: tor=Active (the
///    path is up) while sync=Stalled (the endpoint is the problem). Only a failure
///    to reach the path itself is `Unavailable`.
///
/// **The fourth input — `host` (FR-29 spec §3.4 / §0 A11).** The host transport's
/// descriptor SNAPSHOT, read by the caller from `TorPolicy::host_descriptor()` at
/// derivation time: `None` for every runtime but `HostDialer`, and for a `HostDialer`
/// with nothing registered (or cleared). It is consulted ONLY under a `HostDialer`
/// runtime, after the fell-back check (a latched clearnet leak still outranks it), in
/// THIS order (ADR-0549 D3):
/// `None` → [`TorState::Unavailable`] (nothing is registered — no path exists; the door
/// refuses this at open, so mid-session it means a CLEAR); **`health == Failed` →
/// [`TorState::Unavailable`], read AHEAD of the readiness arm** (the registrant has
/// judged its transport failed, not merely slow — rendering a bootstrap that will never
/// finish was FR-30 (b), the two-registrant lie); readiness below
/// `HOST_TRANSPORT_READY` → [`TorState::Bootstrapping`] with the readiness as the 0..1
/// fraction the DTO already speaks (the host's bootstrap in progress — `Required` fails
/// closed, `Preferred` WAITS, ADR-0546); then the fail-closed stall and `Active` as for
/// every runtime, with `Active` carrying the descriptor's name, isolation and exposure so the UI
/// renders what the host declared and nothing more.
///
/// **The failing arms carry the transport's NAME (FR-30 (a)).** `Bootstrapping` and
/// `Unavailable` both carry `transport: Option<HostTransportName>` — the descriptor's
/// name when one is registered, `None` otherwise (nothing registered, or a runtime that
/// has no registry). The SDK names no transport of its own: a host that registered
/// "Shadowsocks" never reads the noun "Tor" on a failing arm, and where there is no
/// name the UI renders its own neutral noun.
///
/// Pure + TOTAL: it faithfully reflects its `SyncStatus` input and never panics, even on
/// a structurally-impossible combination (e.g. `Connecting` under a `Dialer` runtime —
/// which the engine never emits; were a future bug to emit it, the faithful output is
/// `Bootstrapping`, and the defect would lie upstream in the status producer, not here).
pub(crate) fn live_tor_state(
    policy: &TorPolicy,
    fell_back: bool,
    sync: &SyncStatus,
    host: Option<HostTransportDescriptor>,
    unanswered: bool,
    path_refused: bool,
) -> TorState {
    let runtime = match policy {
        // Clearnet by the user's explicit choice — never claims a Tor state.
        TorPolicy::Off => return TorState::Off,
        TorPolicy::Preferred { runtime } | TorPolicy::Required { runtime } => runtime,
    };
    // A CURRENT clearnet leak is the most safety-critical signal — surface it BEFORE
    // any "still connecting" so a `Preferred` fall-back can never hide behind a
    // concurrent re-bootstrap (the sticky dialer flag; only `Preferred` ever sets it).
    if fell_back {
        return TorState::FellBack;
    }
    // The registered host dialer's own three signals (§3.4), read from the descriptor
    // snapshot: no registration ⇒ no path; a DECLARED failure ⇒ no path (ADR-0549,
    // ahead of readiness — a failed transport is not a bootstrap); a bootstrap in
    // progress ⇒ its readiness. None is a fallback and none is consulted for any
    // other runtime.
    if matches!(runtime, TorRuntime::HostDialer(_)) {
        match host {
            None => return TorState::Unavailable { transport: None },
            Some(descriptor) if descriptor.is_failed() => {
                return TorState::Unavailable {
                    transport: Some(descriptor.name),
                };
            }
            Some(descriptor) if !descriptor.is_ready() => {
                return TorState::Bootstrapping {
                    percent: Some(f32::from(descriptor.readiness) / 100.0),
                    transport: Some(descriptor.name),
                };
            }
            Some(_) => {}
        }
    }
    // A `Connecting` status (no leak yet) — surface its progress regardless of
    // Preferred/Required. NOTHING emits `Connecting` since FR-5 C1 removed the
    // SDK-owned runtime, so this arm is dormant; kept as the forward seam, and
    // faithful to its input if a producer ever appears (see the total/pure note).
    if let SyncStatus::Connecting {
        tor_bootstrap_percent,
    } = sync
    {
        // Pass a FINITE progress through verbatim; sanitize a non-finite percent
        // (NaN/inf from a buggy producer) to `None` ("bootstrapping, progress unknown").
        // A NaN would be unrenderable AND would make `TorState`'s derived `PartialEq`
        // NON-REFLEXIVE (NaN != NaN), silently breaking the §3.3 snapshot/stream dedup
        // (a NaN-percent state would forever look "changed"). So the emitted DTO is
        // always comparable. (Review-fold: the property pass surfaced this.)
        let percent = tor_bootstrap_percent.filter(|p| p.is_finite());
        return TorState::Bootstrapping {
            percent,
            transport: host.map(|descriptor| descriptor.name),
        };
    }
    // Fail-closed: the Tor path could not be established (a clean dial/transport
    // failure under `Required`, or an unsupported runtime). Zero clearnet flowed.
    // TWO producers of the one claim — the sync class through its stall reason,
    // and the posture for the classes no `SyncStatus` speaks for. Above the
    // either/or value because a refused dial's EVIDENCE outranks a timeout's
    // non-evidence.
    if matches!(
        sync,
        SyncStatus::Stalled {
            reason: StallReason::TorUnavailable
        }
    ) || path_refused
    {
        return TorState::Unavailable {
            transport: host.map(|descriptor| descriptor.name),
        };
    }
    // Ready, and nothing has come back over it for the minute while the wallet
    // was trying (FR-36). The SAME payload as `Active`, so a host renders the
    // same transport name with a different sentence, and no reason: the path
    // or the server, and the SDK cannot tell which from here.
    if unanswered {
        return TorState::Unanswered {
            runtime: runtime_kind(runtime, host),
        };
    }
    // The configured runtime is the active path. In Dialer mode the host attributes
    // the real torness; the SDK honestly reports "configured + not degraded" (§2.3).
    TorState::Active {
        runtime: runtime_kind(runtime, host),
    }
}

/// The discriminant of a configured runtime (§2.3): which runtime is speaking, so a
/// panel can attribute the torness claim — for the registered host dialer, WITH the
/// descriptor's name, isolation and exposure (§3.4, ADR-0547). Matched exhaustively
/// WITHIN the crate (the `non_exhaustive` lint only binds downstream), so a future
/// runtime variant is a compile error here — it can never silently mis-attribute.
/// The `HostDialer` arm is reached only with a `Some` descriptor (`live_tor_state`
/// returns `Unavailable` on `None` first); a `None` here is a caller bug rendered
/// honestly — the UNATTRIBUTED name, unknown isolation, unknown exposure — rather
/// than a panic (the function stays total).
fn runtime_kind(runtime: &TorRuntime, host: Option<HostTransportDescriptor>) -> TorRuntimeKind {
    match runtime {
        TorRuntime::ExternalSocks5 { .. } => TorRuntimeKind::ExternalSocks5,
        TorRuntime::Dialer(_) => TorRuntimeKind::Dialer,
        TorRuntime::HostDialer(_) => match host {
            Some(descriptor) => TorRuntimeKind::HostDialer {
                name: descriptor.name,
                isolation: descriptor.isolation,
                exposure: descriptor.exposure,
            },
            None => TorRuntimeKind::HostDialer {
                name: crate::net::host_dialer::HostTransportName::UNATTRIBUTED,
                isolation: crate::net::host_dialer::IsolationSupport::Unknown,
                exposure: crate::net::host_dialer::TransportExposure::Unknown,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::BlockHeight;
    use crate::net::host_dialer::testing::{ScriptedHostDialer, name};
    use crate::net::host_dialer::{
        HOST_TRANSPORT_READY, HostDialer, HostTransportName, IsolationSupport, TransportExposure,
        TransportHealth,
    };
    use crate::ports::NetDialer;
    use crate::ports::testing::StubDialer;
    use crate::state::SyncStamp;
    use std::sync::Arc;

    /// The pre-FR-29 three-input shape, for the policy×flow table below: every
    /// runtime but `HostDialer` ignores the descriptor, so these rows pass `None`;
    /// none of them models the maintainer's minute, so they pass `unanswered =
    /// false` and `path_refused = false`. Shadows the glob import on purpose (an
    /// explicit item wins over `use super::*`); the host-dialer rows below call
    /// `super::live_tor_state` with a descriptor.
    fn live_tor_state(policy: &TorPolicy, fell_back: bool, sync: &SyncStatus) -> TorState {
        super::live_tor_state(policy, fell_back, sync, None, false, false)
    }

    fn dialer_runtime() -> TorRuntime {
        // `runtime_kind(Dialer(_))` only matches the variant; the stub is never dialed.
        TorRuntime::Dialer(Arc::new(StubDialer) as Arc<dyn NetDialer>)
    }

    fn up_to_date() -> SyncStatus {
        SyncStatus::UpToDate {
            tip: BlockHeight::new(2_500_000),
        }
    }

    fn stalled(reason: StallReason) -> SyncStatus {
        SyncStatus::Stalled { reason }
    }

    #[test]
    fn off_is_always_off_regardless_of_flow() {
        // Off is clearnet by choice — it NEVER claims a Tor state, even if (absurdly)
        // a stale flow signal said otherwise. Pins the "Off never over-claims" invariant.
        for (fell_back, sync) in [
            (false, SyncStatus::Idle),
            (true, stalled(StallReason::TorUnavailable)),
            (
                false,
                SyncStatus::Connecting {
                    tor_bootstrap_percent: Some(0.5),
                },
            ),
            (false, up_to_date()),
        ] {
            assert_eq!(
                live_tor_state(&TorPolicy::Off, fell_back, &sync),
                TorState::Off
            );
        }
    }

    #[test]
    fn preferred_not_degraded_is_active_naming_the_runtime() {
        // The steady production state: Preferred + the host Dialer, not fallen back,
        // syncing fine ⇒ Active { Dialer } (the host attributes the real torness).
        for sync in [SyncStatus::Idle, up_to_date()] {
            assert_eq!(
                live_tor_state(
                    &TorPolicy::Preferred {
                        runtime: dialer_runtime()
                    },
                    false,
                    &sync
                ),
                TorState::Active {
                    runtime: TorRuntimeKind::Dialer
                }
            );
        }
    }

    #[test]
    fn preferred_fell_back_is_fellback_even_when_clearnet_also_stalls() {
        // fell_back is the most decisive degradation signal: once Preferred has
        // dropped to clearnet it reads FellBack, and that holds even if the clearnet
        // path is ALSO down (sync=Stalled) — the two axes are orthogonal (tor=FellBack
        // says "Tor degraded"; sync=Stalled separately says "and clearnet is down too").
        let policy = TorPolicy::Preferred {
            runtime: dialer_runtime(),
        };
        assert_eq!(
            live_tor_state(&policy, true, &SyncStatus::Idle),
            TorState::FellBack
        );
        assert_eq!(
            live_tor_state(&policy, true, &stalled(StallReason::EndpointUnreachable)),
            TorState::FellBack
        );
        // fell_back wins over a (here non-production) TorUnavailable stall, too.
        assert_eq!(
            live_tor_state(&policy, true, &stalled(StallReason::TorUnavailable)),
            TorState::FellBack
        );
        // PRECEDENCE (review fold): a CURRENT clearnet leak outranks a concurrent
        // re-bootstrap — `fell_back` + `Connecting` reads FellBack, NOT Bootstrapping,
        // so an active fall-back can never hide behind "still connecting to Tor". (Moot
        // for every runtime today — nothing emits `Connecting` since FR-5 C1 removed
        // the SDK-owned runtime; the ordering is pinned for the seam's future producer.)
        assert_eq!(
            live_tor_state(
                &policy,
                true,
                &SyncStatus::Connecting {
                    tor_bootstrap_percent: Some(0.2),
                },
            ),
            TorState::FellBack
        );
    }

    #[test]
    fn preferred_unsupported_runtime_reads_unavailable_not_active() {
        // An unsupported runtime under Preferred is resolve-REJECTED → never builds a
        // dialer (so fell_back stays false) and `map_net_err` reports the perpetual
        // stall as TorUnavailable. The honest read is Unavailable — NOT Active (the
        // runtime never came up). Moot in production (Dialer-only) but kept honest.
        assert_eq!(
            live_tor_state(
                &TorPolicy::Preferred {
                    runtime: TorRuntime::ExternalSocks5 {
                        addr: "127.0.0.1:9050".into(),
                    },
                },
                false,
                &stalled(StallReason::TorUnavailable),
            ),
            TorState::Unavailable { transport: None }
        );
    }

    #[test]
    fn required_fail_closed_stall_is_unavailable() {
        // Required + the Tor path could not be established (a clean dial/transport
        // failure surfaces the policy stall TorUnavailable) ⇒ Unavailable, zero clearnet.
        assert_eq!(
            live_tor_state(
                &TorPolicy::Required {
                    runtime: dialer_runtime()
                },
                false,
                &stalled(StallReason::TorUnavailable),
            ),
            TorState::Unavailable { transport: None }
        );
    }

    #[test]
    fn required_with_a_wedged_endpoint_stays_active_tor_is_up() {
        // The orthogonal-axes case: under Required a no-progress endpoint WEDGE
        // surfaces Stalled { EndpointUnreachable } (Tor WAS established; the endpoint
        // stopped delivering). tor=Active (the path is up) is honest while sync=Stalled
        // names the endpoint as the problem — only a failure to reach Tor is Unavailable.
        for reason in [
            StallReason::EndpointUnreachable,
            StallReason::Internal,
            StallReason::ChainReorg,
            StallReason::StorageFull,
        ] {
            assert_eq!(
                live_tor_state(
                    &TorPolicy::Required {
                        runtime: dialer_runtime()
                    },
                    false,
                    &stalled(reason),
                ),
                TorState::Active {
                    runtime: TorRuntimeKind::Dialer
                },
                "Required + Stalled {{ {reason:?} }} is an endpoint/local problem, Tor is up",
            );
        }
    }

    #[test]
    fn connecting_surfaces_bootstrap_progress_for_either_tor_policy() {
        // `SyncStatus::Connecting` maps straight through to Bootstrapping, carrying the
        // progress verbatim (Some and None both), under BOTH Preferred and Required.
        // No producer emits it today (the transport's bootstrap arrives through the
        // descriptor instead — FR-5 C1); the arm is a forward seam and this pins that
        // it stays FAITHFUL to its input, with no transport name to carry.
        for policy in [
            TorPolicy::Preferred {
                runtime: dialer_runtime(),
            },
            TorPolicy::Required {
                runtime: dialer_runtime(),
            },
        ] {
            for percent in [Some(0.0), Some(0.73), None] {
                assert_eq!(
                    live_tor_state(
                        &policy,
                        false,
                        &SyncStatus::Connecting {
                            tor_bootstrap_percent: percent,
                        },
                    ),
                    TorState::Bootstrapping {
                        percent,
                        transport: None
                    }
                );
            }
        }
    }

    #[test]
    fn runtime_kind_pins_each_variant() {
        // Three runtimes, exhaustively: the SDK-owned `BuiltIn` was removed at
        // FR-5 C1 (ADR-0548 D3) and the match has no fourth arm to cover.
        assert_eq!(
            runtime_kind(&dialer_runtime(), None),
            TorRuntimeKind::Dialer
        );
        assert_eq!(
            runtime_kind(
                &TorRuntime::ExternalSocks5 {
                    addr: "127.0.0.1:9050".into()
                },
                None
            ),
            TorRuntimeKind::ExternalSocks5
        );
        // The host dialer arm carries the descriptor's name and two closed
        // values; a missing descriptor (a caller bug — `live_tor_state` never
        // reaches the arm without one) renders unattributed, never panics.
        let host = ScriptedHostDialer::ready();
        let descriptor = host.descriptor().expect("ready");
        let runtime = TorRuntime::HostDialer(Arc::new(host) as Arc<dyn HostDialer>);
        assert_eq!(
            runtime_kind(&runtime, Some(descriptor)),
            TorRuntimeKind::HostDialer {
                name: name("Tor"),
                isolation: IsolationSupport::Supported,
                exposure: TransportExposure::Hidden,
            }
        );
        assert_eq!(
            runtime_kind(&runtime, None),
            TorRuntimeKind::HostDialer {
                name: HostTransportName::UNATTRIBUTED,
                isolation: IsolationSupport::Unknown,
                exposure: TransportExposure::Unknown,
            }
        );
    }

    fn host_runtime(host: &Arc<ScriptedHostDialer>) -> TorRuntime {
        TorRuntime::HostDialer(Arc::clone(host) as Arc<dyn HostDialer>)
    }

    /// A descriptor whose HEALTH follows its readiness — the shape a registrant
    /// that has not failed pushes. The health axis itself is exercised by
    /// `a_failed_health_renders_unavailable_and_starting_renders_bootstrapping`,
    /// which spells every combination out.
    fn descriptor(
        transport: &str,
        readiness: u8,
        isolation: IsolationSupport,
        exposure: TransportExposure,
    ) -> HostTransportDescriptor {
        HostTransportDescriptor {
            name: name(transport),
            readiness,
            isolation,
            exposure,
            health: if readiness >= HOST_TRANSPORT_READY {
                TransportHealth::Ready
            } else {
                TransportHealth::Starting
            },
        }
    }

    /// FR-29 spec §3.4, the host-dialer column of the fail-closed matrix (§6.2) at the
    /// state layer, and T5's core half (`host_retire_moves_tor_state`): the descriptor
    /// snapshot is the host's readiness/retire push as the core sees it. Nothing
    /// registered (a mid-session CLEAR — the door refuses it at open) → `Unavailable`;
    /// readiness below 100 → `Bootstrapping` carrying the readiness as the DTO's 0..1
    /// fraction; the fail-closed `TorUnavailable` stall → `Unavailable`; ready + not
    /// stalled → `Active` naming the host's name, isolation AND exposure; a fresh READY descriptor
    /// after a retire reads `Active` again. A latched clearnet leak still outranks all
    /// of it (fell_back → `FellBack`), and Preferred/Required derive identically.
    #[test]
    fn host_retire_moves_tor_state() {
        let host = Arc::new(ScriptedHostDialer::ready());
        for policy in [
            TorPolicy::Preferred {
                runtime: host_runtime(&host),
            },
            TorPolicy::Required {
                runtime: host_runtime(&host),
            },
        ] {
            let derive = |sync: &SyncStatus| {
                super::live_tor_state(&policy, false, sync, policy.host_descriptor(), false, false)
            };
            // Ready over Tor with isolation: Active, naming the host's name and
            // both closed values.
            assert_eq!(
                derive(&SyncStatus::Idle),
                TorState::Active {
                    runtime: TorRuntimeKind::HostDialer {
                        name: name("Tor"),
                        isolation: IsolationSupport::Supported,
                        exposure: TransportExposure::Hidden,
                    }
                }
            );
            // The host's readiness push at 37%: Bootstrapping { 0.37 } — the
            // bootstrap in progress is neither a fallback nor Unavailable.
            host.set_descriptor(Some(descriptor(
                "Tor",
                37,
                IsolationSupport::Supported,
                TransportExposure::Hidden,
            )));
            assert_eq!(
                derive(&up_to_date()),
                TorState::Bootstrapping {
                    percent: Some(0.37),
                    transport: Some(name("Tor"))
                }
            );
            // A retire / clear (`descriptor() == None`): Unavailable, whatever the sync
            // axis says — there is no path to attribute.
            host.set_descriptor(None);
            for sync in [
                SyncStatus::Idle,
                up_to_date(),
                stalled(StallReason::TorUnavailable),
            ] {
                assert_eq!(
                    derive(&sync),
                    TorState::Unavailable { transport: None },
                    "cleared under {sync:?}"
                );
            }
            // The host re-arms behind the same trampoline with a DIFFERENT backing
            // (Shadowsocks, non-isolating): Active again, rendering the new name and
            // the linkability honestly.
            host.set_descriptor(Some(descriptor(
                "Shadowsocks",
                100,
                IsolationSupport::Unsupported,
                TransportExposure::Hidden,
            )));
            assert_eq!(
                derive(&SyncStatus::Idle),
                TorState::Active {
                    runtime: TorRuntimeKind::HostDialer {
                        name: name("Shadowsocks"),
                        isolation: IsolationSupport::Unsupported,
                        exposure: TransportExposure::Hidden,
                    }
                }
            );
            // Ready but the dial path failed closed (a `Retired`/`Refused`/unreachable
            // dial under Required surfaces the policy stall): Unavailable.
            assert_eq!(
                derive(&stalled(StallReason::TorUnavailable)),
                TorState::Unavailable {
                    transport: Some(name("Shadowsocks"))
                },
                "the stall names the transport the host registered, never \"Tor\""
            );
            // A latched clearnet leak outranks every descriptor state.
            assert_eq!(
                super::live_tor_state(
                    &policy,
                    true,
                    &SyncStatus::Idle,
                    policy.host_descriptor(),
                    false,
                    false
                ),
                TorState::FellBack
            );
            host.set_descriptor(Some(ScriptedHostDialer::tor_ready()));
        }
        // The descriptor is consulted for NO other runtime: a `Dialer` policy handed a
        // bootstrapping descriptor (impossible from `host_descriptor()`, which returns
        // `None` for it) still reads Active { Dialer }.
        assert_eq!(
            super::live_tor_state(
                &TorPolicy::Required {
                    runtime: dialer_runtime()
                },
                false,
                &SyncStatus::Idle,
                Some(descriptor(
                    "Tor",
                    0,
                    IsolationSupport::Unknown,
                    TransportExposure::Unknown
                )),
                false,
                false,
            ),
            TorState::Active {
                runtime: TorRuntimeKind::Dialer
            }
        );
    }

    /// ADR-0549 D3 (FR-30 (b), the maintainer's ruling): the descriptor's HEALTH is
    /// read AHEAD of its readiness, and BOTH polarities are pinned — a registrant
    /// that declares `Failed` at readiness 40 reads `Unavailable` (not a bootstrap
    /// that never ends: the lie two registrants shipped, Relim's `FAILING_READINESS`
    /// floor and the plugin's own `Failed` phase), while `Starting` at the SAME
    /// readiness still reads `Bootstrapping { 0.40 }`. Also: `Failed` is fail-closed
    /// at readiness 100 (health never PERMITS a dial), it never becomes `FellBack`
    /// (ADR-0546 — a declared failure is not a reachability fallback), and the
    /// failing arms carry the registrant's own name (FR-30 (a)).
    #[test]
    fn a_failed_health_renders_unavailable_and_starting_renders_bootstrapping() {
        let host = Arc::new(ScriptedHostDialer::ready());
        let failed_at = |readiness: u8| HostTransportDescriptor {
            name: name("Shadowsocks"),
            readiness,
            isolation: IsolationSupport::Unsupported,
            exposure: TransportExposure::Hidden,
            health: TransportHealth::Failed,
        };
        for policy in [
            TorPolicy::Preferred {
                runtime: host_runtime(&host),
            },
            TorPolicy::Required {
                runtime: host_runtime(&host),
            },
        ] {
            let derive = |sync: &SyncStatus| {
                super::live_tor_state(&policy, false, sync, policy.host_descriptor(), false, false)
            };
            // STARTING at readiness 40 — an honest bootstrap in progress.
            host.set_descriptor(Some(descriptor(
                "Shadowsocks",
                40,
                IsolationSupport::Unsupported,
                TransportExposure::Hidden,
            )));
            assert_eq!(
                derive(&SyncStatus::Idle),
                TorState::Bootstrapping {
                    percent: Some(0.4),
                    transport: Some(name("Shadowsocks"))
                },
                "STARTING at 40 is a bootstrap, and it names the host's transport",
            );
            // FAILED at the SAME readiness — the state says failed, not starting.
            host.set_descriptor(Some(failed_at(40)));
            assert_eq!(
                derive(&SyncStatus::Idle),
                TorState::Unavailable {
                    transport: Some(name("Shadowsocks"))
                },
                "FAILED outranks the readiness arm (ADR-0549 D3)",
            );
            // FAILED at 100: health never permits a dial — still Unavailable, and
            // the readiness the registrant MEASURED is not overwritten.
            host.set_descriptor(Some(failed_at(HOST_TRANSPORT_READY)));
            assert_eq!(
                derive(&up_to_date()),
                TorState::Unavailable {
                    transport: Some(name("Shadowsocks"))
                },
                "a FAILED transport at readiness 100 is still no path",
            );
            // A declared failure is NOT a clearnet fallback (ADR-0546): only the
            // dialer's own latch produces FellBack, and it still outranks health.
            assert_ne!(derive(&SyncStatus::Idle), TorState::FellBack);
            assert_eq!(
                super::live_tor_state(
                    &policy,
                    true,
                    &SyncStatus::Idle,
                    policy.host_descriptor(),
                    false,
                    false
                ),
                TorState::FellBack,
                "a latched leak outranks the health axis too",
            );
            // READY again ⇒ Active, so the FAILED arm is not a one-way latch.
            host.set_descriptor(Some(ScriptedHostDialer::tor_ready()));
            assert_eq!(
                derive(&SyncStatus::Idle),
                TorState::Active {
                    runtime: TorRuntimeKind::HostDialer {
                        name: name("Tor"),
                        isolation: IsolationSupport::Supported,
                        exposure: TransportExposure::Hidden,
                    }
                }
            );
        }
    }

    #[test]
    fn scanning_and_offline_sync_variants_and_nonfinite_percent_are_handled() {
        // The table set never constructed `Scanning` (the Spend-before-Sync core flow) or
        // `Offline` (a RESERVED variant the engine never emits today). Both are sync-axis
        // states ⇒ for a non-degraded Preferred wallet they fall through to Active.
        let pref = || TorPolicy::Preferred {
            runtime: dialer_runtime(),
        };
        let scanning = SyncStatus::Scanning {
            from: BlockHeight::new(280_000),
            to: BlockHeight::new(281_000),
            percent: 0.5,
            spendable_ready: false,
            rewound: false,
        };
        let offline = SyncStatus::Offline { last_synced: None };
        for sync in [&scanning, &offline] {
            assert_eq!(
                live_tor_state(&pref(), false, sync),
                TorState::Active {
                    runtime: TorRuntimeKind::Dialer
                }
            );
            assert_eq!(live_tor_state(&TorPolicy::Off, false, sync), TorState::Off);
        }

        // Non-finite bootstrap progress is sanitized to None (renderable + keeps
        // TorState's derived PartialEq reflexive); a finite value passes through verbatim.
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(
                live_tor_state(
                    &pref(),
                    false,
                    &SyncStatus::Connecting {
                        tor_bootstrap_percent: Some(bad),
                    },
                ),
                TorState::Bootstrapping {
                    percent: None,
                    transport: None
                },
                "a non-finite percent ({bad}) is sanitized to None",
            );
        }
        assert_eq!(
            live_tor_state(
                &pref(),
                false,
                &SyncStatus::Connecting {
                    tor_bootstrap_percent: Some(0.5),
                },
            ),
            TorState::Bootstrapping {
                percent: Some(0.5),
                transport: None
            },
        );
    }

    // ---- adversarial property coverage (testing-patterns #2) ----

    fn arb_stall() -> impl proptest::strategy::Strategy<Value = StallReason> {
        proptest::prop_oneof![
            proptest::prelude::Just(StallReason::EndpointUnreachable),
            proptest::prelude::Just(StallReason::TorUnavailable),
            proptest::prelude::Just(StallReason::StorageFull),
            proptest::prelude::Just(StallReason::ChainReorg),
            proptest::prelude::Just(StallReason::Internal),
        ]
    }

    /// Random `f32` UNIONED with the exact boundaries random sampling rarely lands on —
    /// 0.0/1.0 and the non-finite NaN/±inf the sanitization must catch.
    fn arb_percent() -> impl proptest::strategy::Strategy<Value = f32> {
        proptest::prop_oneof![
            proptest::prelude::any::<f32>(),
            proptest::prelude::Just(0.0f32),
            proptest::prelude::Just(1.0f32),
            proptest::prelude::Just(f32::NAN),
            proptest::prelude::Just(f32::INFINITY),
            proptest::prelude::Just(-1.0f32),
        ]
    }

    /// Every `SyncStatus` variant, with degenerate payloads (NaN percents, boundary heights).
    fn arb_sync() -> impl proptest::strategy::Strategy<Value = SyncStatus> {
        use proptest::prelude::{Just, Strategy, any};
        proptest::prop_oneof![
            Just(SyncStatus::Idle),
            proptest::option::of(arb_percent()).prop_map(|p| SyncStatus::Connecting {
                tor_bootstrap_percent: p
            }),
            (
                any::<u32>(),
                any::<u32>(),
                arb_percent(),
                any::<bool>(),
                any::<bool>(),
            )
                .prop_map(|(f, t, p, s, r)| SyncStatus::Scanning {
                    from: BlockHeight::new(f),
                    to: BlockHeight::new(t),
                    percent: p,
                    spendable_ready: s,
                    rewound: r,
                }),
            any::<u32>().prop_map(|h| SyncStatus::UpToDate {
                tip: BlockHeight::new(h)
            }),
            arb_stall().prop_map(|r| SyncStatus::Stalled { reason: r }),
            proptest::option::of((any::<u32>(), any::<u64>()).prop_map(|(h, a)| SyncStamp {
                height: BlockHeight::new(h),
                at: a,
            }))
            .prop_map(|ls| SyncStatus::Offline { last_synced: ls }),
        ]
    }

    /// 0 = Dialer, 1 = ExternalSocks5 — the two runtimes constructible without a
    /// registry (`HostDialer`, the third, needs one; there is no fourth).
    fn runtime_of(kind: u8) -> TorRuntime {
        match kind {
            0 => dialer_runtime(),
            _ => TorRuntime::ExternalSocks5 {
                addr: "127.0.0.1:9050".into(),
            },
        }
    }

    fn expected_kind(kind: u8) -> TorRuntimeKind {
        match kind {
            0 => TorRuntimeKind::Dialer,
            _ => TorRuntimeKind::ExternalSocks5,
        }
    }

    proptest::proptest! {
        /// The privacy/money-critical honesty invariants hold over the WHOLE input space
        /// (every policy × fell_back × every SyncStatus variant, incl. NaN/inf percents):
        /// the mapping is TOTAL (never panics) and NEVER claims protection it lacks.
        #[test]
        fn live_tor_state_honesty_invariants_hold_over_the_whole_input_space(
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
            // Calling it at all proves TOTALITY (never panics, even on NaN/inf/Offline).
            let out = live_tor_state(&policy, fell_back, &sync);

            if policy_kind == 0 {
                // (a) Off ⇒ ALWAYS Off — clearnet-by-choice never claims a Tor state.
                proptest::prop_assert_eq!(out, TorState::Off);
            } else if fell_back {
                // (b) a CURRENT clearnet leak is NEVER masked: fell_back && !Off ⇒ FellBack.
                proptest::prop_assert_eq!(out, TorState::FellBack);
            } else {
                // (c) reverse-direction money/privacy invariants on the non-degraded outputs.
                match out {
                    // Active is NEVER returned while leaking, and names the configured runtime.
                    TorState::Active { runtime } => {
                        proptest::prop_assert_eq!(runtime, expected_kind(rt_kind));
                    }
                    // Unavailable ⇒ the fail-closed TorUnavailable stall (zero-clearnet honesty).
                    // `transport` is None over this space: these rows register no dialer,
                    // so there is no descriptor and the SDK names no transport of its own.
                    TorState::Unavailable { transport } => {
                        let is_tor_unavailable = matches!(
                            sync,
                            SyncStatus::Stalled { reason: StallReason::TorUnavailable }
                        );
                        proptest::prop_assert!(is_tor_unavailable);
                        proptest::prop_assert!(transport.is_none());
                    }
                    // Bootstrapping ⇒ status was Connecting AND the percent is renderable
                    // (finite-or-None) so TorState's PartialEq stays reflexive (§3.3 dedup).
                    TorState::Bootstrapping { percent, transport } => {
                        let was_connecting = matches!(sync, SyncStatus::Connecting { .. });
                        proptest::prop_assert!(was_connecting);
                        proptest::prop_assert!(percent.is_none_or(|p| p.is_finite()));
                        proptest::prop_assert!(transport.is_none());
                    }
                    // FellBack needs fell_back (false here); Off needs the Off policy.
                    other => proptest::prop_assert!(
                        false,
                        "non-degraded mapping produced an impossible {:?}",
                        other
                    ),
                }
            }
        }

        /// `live_tor_state` consults policy ONLY to split Off from non-Off — Preferred and
        /// Required of the SAME runtime kind map IDENTICALLY (the policy distinction lives
        /// upstream in the status producer, not this mapper). A future policy-specific
        /// branch would be a silent regression this pins.
        #[test]
        fn live_tor_state_is_blind_to_preferred_vs_required(
            rt_kind in 0u8..2,
            fell_back in proptest::prelude::any::<bool>(),
            sync in arb_sync(),
        ) {
            let pref = TorPolicy::Preferred { runtime: runtime_of(rt_kind) };
            let req = TorPolicy::Required { runtime: runtime_of(rt_kind) };
            proptest::prop_assert_eq!(
                live_tor_state(&pref, fell_back, &sync),
                live_tor_state(&req, fell_back, &sync),
            );
        }
    }

    // Stage S1 `truth` — the derivation's fifth input: a child of this module
    // so it reaches the strategies above, its own file so the cited lines
    // stay where their watches printed them.
    mod unanswered;
}
