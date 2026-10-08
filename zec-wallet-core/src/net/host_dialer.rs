//! FR-29 — the host transport crossing, the CORE's half of the types
//! (`docs/specs/host-transport-crossing.md` §2; the contract is
//! `sdk/zec_wallet/rust/include/zec_wallet_net_dialer.h`).
//!
//! A host that consumes the SDK as a separate native library registers a
//! dialer across the library boundary (ADR-0543, mirroring the FR-15 seed
//! port); the bridge's cabi module implements [`HostDialer`] over that
//! process-global registry, and the core consumes it through
//! `TorRuntime::HostDialer(Arc<dyn HostDialer>)` (chunk C3bc) exactly as it
//! consumes any other [`NetDialer`]. The `unsafe` lives in the bridge; this
//! module is safe Rust and carries only the closed value sets the boundary
//! validates and the ONE mapping from the host's dial codes to the port's
//! [`DialError`].
//!
//! Nothing here changes behaviour on its own (the C3-0 types commit): the
//! runtime variant, the state derivation and the door refusal land with the
//! wiring chunk, against these types.

use crate::error::DialError;
use crate::ports::NetDialer;

/// The ABI version the SDK's registration verb accepts
/// (`ZW_NET_DIALER_ABI_VERSION` in the header). A change to any value or
/// rule in the header is a new version. 4 = ADR-0553 as built, where
/// `ZW_HEALTH_FAILED` stopped being fail-closed under `Preferred`; 3 =
/// ADR-0549, the descriptor's closed `health` axis; 2 = ADR-0547, the
/// host-named descriptor; 1 = the retired kind-bearing descriptor of the same
/// morning (no host shipped against it).
///
/// **WHY 4 IS A BUMP WITH NO STRUCT CHANGE.** Nothing crossing the
/// boundary moved — no code, bound, field or discriminant. What moved is the
/// PRIVACY MEANING of a value the host sends us: at v3 a declared
/// `ZW_HEALTH_FAILED` was fail-closed like not-ready and could never reach
/// clearnet; at v4, under `Preferred` only, it counts as the private path
/// FAILING and a full patience minute of it switches to clearnet, visibly.
/// The header's own rule is that a change to any value OR RULE is a new
/// version, and this is the rule case: a host that declares FAILED is entitled
/// to know which meaning it is declaring, and an old copy must be refused at
/// `register` rather than silently inheriting the new one. (The
/// `ZW_NET_DIALER_DIAL_BUDGET_SECS` precedent — documented as NOT a bump —
/// went the other way for a reason worth keeping distinct: that was a
/// SDK-side deadline that crosses nothing, so an old host stayed both correct
/// and safe. Stretching it to cover a privacy rule is what would make the
/// precedent unbounded.) `Required` and `NOT_READY` behave identically at v3
/// and v4.
pub const HOST_DIALER_ABI_VERSION: u32 = 4;

/// `ZW_TRANSPORT_NAME_MAX_BYTES` — the bound on the host's display name.
/// WHY 32: a transport chip holds one or two words ("Tor", "Shadowsocks",
/// "VLESS via Cloudflare"); 32 bytes of UTF-8 is eight CJK characters or a
/// short Latin phrase, and a fixed 32-byte field keeps the C struct
/// fixed-width — no pointer, no lifetime across the call.
pub const HOST_TRANSPORT_NAME_MAX_BYTES: usize = 32;

/// The host's display name for its transport (ADR-0547: THE HOST NAMES ITS
/// TRANSPORT — the SDK carries no predefined kinds). Host-chosen, bounded,
/// validated at the crossing, DISPLAY-ONLY: the SDK renders it verbatim and
/// never interprets it; policy never reads it; it is NEVER logged (spec §5).
/// Fixed-width and `Copy` — it mirrors the header's `uint8_t name[32]` +
/// `name_len` — so [`HostTransportDescriptor`] and the state's
/// `TorRuntimeKind` stay `Copy` and no host text is ever heap-allocated on
/// the crossing.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct HostTransportName {
    bytes: [u8; HOST_TRANSPORT_NAME_MAX_BYTES],
    len: u8,
}

impl HostTransportName {
    /// The EMPTY name — never a host's (`new` refuses an empty name with
    /// `-6`); the core's own rendering of a descriptor it does not have (the
    /// `runtime_kind` caller-bug arm, spec §3.4). The UI renders it as its
    /// locale's "a private path".
    pub const UNATTRIBUTED: Self = Self {
        bytes: [0; HOST_TRANSPORT_NAME_MAX_BYTES],
        len: 0,
    };

    /// Validate the name bytes that crossed the boundary (`name[..name_len]`).
    /// Exactly four rules, each refused with `None` (`ZW_RC_DESCRIPTOR`):
    /// (1) not empty and not blank — at least one non-whitespace character;
    /// (2) at most [`HOST_TRANSPORT_NAME_MAX_BYTES`]; (3) valid UTF-8; (4) no
    /// control or FORMAT character — C0, C1, DEL and a NUL the host counted
    /// in `name_len` (`char::is_control`), plus the characters that would
    /// let a name re-order, hide or line-break the privacy sentence the UI
    /// renders right after it (the four angles' fold): the bidi controls
    /// U+202A–U+202E and U+2066–U+2069, the zero-width characters
    /// U+200B–U+200F, U+FEFF, and the line/paragraph separators U+2028 and
    /// U+2029. Nothing else is interpreted.
    pub fn new(raw: &[u8]) -> Option<Self> {
        if raw.is_empty() || raw.len() > HOST_TRANSPORT_NAME_MAX_BYTES {
            return None;
        }
        let text = std::str::from_utf8(raw).ok()?;
        if text.chars().any(Self::is_forbidden) || text.trim().is_empty() {
            return None;
        }
        let mut bytes = [0u8; HOST_TRANSPORT_NAME_MAX_BYTES];
        bytes[..raw.len()].copy_from_slice(raw);
        // `raw.len() <= 32` ⇒ the narrowing is exact.
        let len = u8::try_from(raw.len()).ok()?;
        Some(Self { bytes, len })
    }

    /// The name as text. `new` validated the bytes as UTF-8 and
    /// `UNATTRIBUTED` is empty, so the only way this reads "" is the
    /// unattributed name.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or("")
    }

    /// Is this the core's empty [`Self::UNATTRIBUTED`] name (never a host's)?
    pub fn is_unattributed(&self) -> bool {
        self.len == 0
    }

    /// Rule 4's character set: control characters, and the format /
    /// separator characters that would let a host-chosen name re-order,
    /// hide or line-break the SDK's own privacy sentence rendered after it.
    fn is_forbidden(c: char) -> bool {
        c.is_control()
            || matches!(
                c,
                '\u{200B}'..='\u{200F}'
                    | '\u{2028}'
                    | '\u{2029}'
                    | '\u{202A}'..='\u{202E}'
                    | '\u{2066}'..='\u{2069}'
                    | '\u{FEFF}'
            )
    }
}

/// REDACTING on purpose (spec §5: the host's name is never logged): a `{:?}`
/// of a descriptor, a `TorRuntimeKind` or a `TorState` shows the name's byte
/// length, never its text — so no future log line can carry it by accident.
/// Tests that need the text compare [`HostTransportName::as_str`].
impl std::fmt::Debug for HostTransportName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HostTransportName(<{} bytes>)", self.len)
    }
}

/// Whether the host's path HIDES the device's network address from the
/// server (`ZW_EXPOSURE_*`) — the one privacy fact no name can tell the
/// wallet (ADR-0547 decision 3). `Hidden`: the server sees the transport's
/// exit, not the device (Tor, a proxy, a tunnel). `Exposed`: the server
/// sees the device's address (a plain connection behind the host's
/// trampoline, a forward proxy that passes the client address) — renders
/// "not private". `Unknown`: not declared — renders with caution, never the
/// protected tone. Policy never branches on it (spec D6); rendering does.
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransportExposure {
    Unknown,
    Hidden,
    Exposed,
}

impl TransportExposure {
    /// The header's integer for this value.
    pub fn as_raw(self) -> u32 {
        match self {
            Self::Unknown => 0,
            Self::Hidden => 1,
            Self::Exposed => 2,
        }
    }

    /// The header's integer → the value; `None` outside the closed set.
    pub fn from_raw(raw: u32) -> Option<Self> {
        Some(match raw {
            0 => Self::Unknown,
            1 => Self::Hidden,
            2 => Self::Exposed,
            _ => return None,
        })
    }
}

/// Whether the host's transport honours per-key circuit isolation
/// (`ZW_ISOLATION_*`). The SDK passes its key on every dial regardless
/// (ADR-0545 D2); `Unknown` and `Unsupported` both render as "connections
/// can be linked by the proxy" — the state never promises what the host
/// did not declare.
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IsolationSupport {
    Unknown,
    Supported,
    Unsupported,
}

impl IsolationSupport {
    /// The header's integer for this value.
    pub fn as_raw(self) -> u32 {
        match self {
            Self::Unknown => 0,
            Self::Supported => 1,
            Self::Unsupported => 2,
        }
    }

    /// The header's integer → the value; `None` outside the closed set.
    pub fn from_raw(raw: u32) -> Option<Self> {
        Some(match raw {
            0 => Self::Unknown,
            1 => Self::Supported,
            2 => Self::Unsupported,
            _ => return None,
        })
    }
}

/// Whether the registrant's transport is ALIVE (`ZW_HEALTH_*`; ABI v3,
/// ADR-0549 — FR-30 (b)). `readiness` says how far a bootstrap has come;
/// `health` says whether that bootstrap is still one. `Starting`: coming up
/// (or suspended). `Ready`: carrying. `Failed`: the registrant has JUDGED its
/// transport failed, not merely slow — the state renders `Unavailable` ahead
/// of the readiness arm, and no dial is attempted. `health` never PERMITS a
/// dial (readiness alone does) and never suppresses the fell-back latch.
/// Under `Required`, `Failed` is fail-closed like not-ready and retired. Under
/// `Preferred` it is the ONE refusal that counts as the private path failing
/// (`DialError::TransportFailed`): a bootstrap may take as long as it takes and
/// never reaches clearnet, but a transport its host has given up on is
/// switch-eligible once the maintainer's minute of it is spent (ADR-0553, as
/// narrowed by ADR-0552 phase 2).
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransportHealth {
    Starting,
    Ready,
    Failed,
}

impl TransportHealth {
    /// The header's integer for this value.
    pub fn as_raw(self) -> u32 {
        match self {
            Self::Starting => 0,
            Self::Ready => 1,
            Self::Failed => 2,
        }
    }

    /// The header's integer → the value; `None` outside the closed set.
    pub fn from_raw(raw: u32) -> Option<Self> {
        Some(match raw {
            0 => Self::Starting,
            1 => Self::Ready,
            2 => Self::Failed,
            _ => return None,
        })
    }
}

/// The readiness value that means "ready" (`readiness` is 0..=100; anything
/// below is "not ready": `Required` fails closed, `Preferred` WAITS, the
/// state renders `Bootstrapping { percent }`).
pub const HOST_TRANSPORT_READY: u8 = 100;

/// The host's transport descriptor — AUTHORITATIVE for policy and rendering
/// (ADR-0545 D3), bounded (the name ≤ 32 validated bytes; four closed value
/// sets; no unbounded text). **Across the C CROSSING it is built only through
/// [`Self::from_raw`]**, so an in-range descriptor is the only kind that
/// arrives from a registrant. Not a type-level guarantee: the fields are
/// `pub`, so an in-workspace Rust host implementing [`HostDialer`] can
/// construct any combination — it is trusted (ADR-0545), `health` is a closed
/// TYPE, and the three integers are handled safely out of range by
/// [`Self::is_ready`] / [`Self::is_failed`] either way.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct HostTransportDescriptor {
    /// The host's own name for its transport — rendered verbatim, never
    /// interpreted, never logged (ADR-0547).
    pub name: HostTransportName,
    /// `0..=100`; [`HOST_TRANSPORT_READY`] is ready.
    pub readiness: u8,
    pub isolation: IsolationSupport,
    pub exposure: TransportExposure,
    /// Is the transport alive (ABI v3, ADR-0549)? Independent of `readiness`:
    /// no cross-field rule is validated — a `Failed` descriptor keeps the
    /// readiness the registrant MEASURED.
    pub health: TransportHealth,
}

impl HostTransportDescriptor {
    /// Validate the name bytes (`name[..name_len]`) and the four integers
    /// that crossed the boundary. `None` for any violation
    /// (`ZW_RC_DESCRIPTOR`): a name that breaks one of
    /// [`HostTransportName::new`]'s four rules, a readiness above 100, an
    /// unknown isolation, exposure or health discriminant.
    pub fn from_raw(
        name: &[u8],
        readiness: u32,
        isolation: u32,
        exposure: u32,
        health: u32,
    ) -> Option<Self> {
        let name = HostTransportName::new(name)?;
        let isolation = IsolationSupport::from_raw(isolation)?;
        let exposure = TransportExposure::from_raw(exposure)?;
        let health = TransportHealth::from_raw(health)?;
        if readiness > u32::from(HOST_TRANSPORT_READY) {
            return None;
        }
        // `readiness <= 100` ⇒ the narrowing is exact.
        let readiness = u8::try_from(readiness).ok()?;
        Some(Self {
            name,
            readiness,
            isolation,
            exposure,
            health,
        })
    }

    /// Has the registrant declared its transport FAILED (ADR-0549)? Read by
    /// the state derivation AHEAD of the readiness arm.
    pub fn is_failed(&self) -> bool {
        self.health == TransportHealth::Failed
    }

    /// Is the host's transport ready to carry a dial? THE one predicate the
    /// readiness gate and the state derivation share: readiness at 100 AND
    /// not declared failed — `health` never permits a dial, it only forbids
    /// one (a `Failed` descriptor at readiness 100 is refused `NotReady`).
    pub fn is_ready(&self) -> bool {
        self.readiness >= HOST_TRANSPORT_READY && !self.is_failed()
    }
}

/// The FROZEN dial-code table (`ZW_DIAL_*`, spec §3.3). The numeric values
/// ARE the ABI: they never change meaning or number. `Ok` is not an error.
/// `non_exhaustive` is the crate's G2 policy for every public enum (a host's
/// exhaustive match must not break on an SDK upgrade); it does not loosen the
/// table — `from_raw` is the only constructor and it admits exactly these six.
#[repr(u32)]
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HostDialCode {
    Ok = 0,
    /// A bootstrap in progress — NOT a reachability failure (ADR-0546):
    /// `Required` fails closed, `Preferred` waits, nobody falls back.
    NotReady = 1,
    Unreachable = 2,
    Timeout = 3,
    /// The transport will not carry this request — a policy, not a fault.
    Refused = 4,
    /// The backing was torn down or replaced; in-flight work fails typed.
    Retired = 5,
}

impl HostDialCode {
    /// The header's integer for this code.
    pub fn as_raw(self) -> u32 {
        self as u32
    }

    /// A raw `u32` from the host → the code; `None` for anything outside
    /// the table (a host protocol violation: the op fails `Io`, the stream
    /// closes).
    pub fn from_raw(raw: u32) -> Option<Self> {
        Some(match raw {
            0 => Self::Ok,
            1 => Self::NotReady,
            2 => Self::Unreachable,
            3 => Self::Timeout,
            4 => Self::Refused,
            5 => Self::Retired,
            _ => return None,
        })
    }

    /// THE ONE place a host code becomes a port error (one source of truth;
    /// `PolicyDialer`'s fallback rule reads the `DialError`, never the raw
    /// code). `Ok` → `None`. Only `Unreachable` and `Timeout` can reach the
    /// clearnet fallback under `Preferred` (ADR-0546); the other three land
    /// in `PolicyDialer`'s surface-as-is arm by construction.
    pub fn into_dial_error(self) -> Option<DialError> {
        Some(match self {
            Self::Ok => return None,
            Self::NotReady => DialError::NotReady,
            Self::Unreachable => DialError::Unreachable,
            Self::Timeout => DialError::Timeout,
            Self::Refused => DialError::Unsupported,
            Self::Retired => DialError::Retired,
        })
    }

    /// A raw code the host reported for a FAILED operation, mapped for the
    /// port: an unknown integer is a protocol violation (`Io`); `Ok` on a
    /// failure path is one too.
    pub fn failure_from_raw(raw: u32) -> DialError {
        match Self::from_raw(raw).and_then(Self::into_dial_error) {
            Some(err) => err,
            None => DialError::Io(std::io::Error::other(
                "host dialer protocol violation: unknown or non-failure dial code",
            )),
        }
    }
}

/// The registry accessor the core consumes; the bridge's cabi module
/// implements it over the process-global registry (the `CAbiSeedPort`
/// shape). A `HostDialer` IS a [`NetDialer`] — `resolve_dialer` hands it to
/// `PolicyDialer` like any other — and additionally reports the host's
/// descriptor for the door (`None` ⇒ refuse a `HostDialer` policy at
/// validation, spec E1) and for the state derivation (§3.4).
pub trait HostDialer: NetDialer {
    /// The current descriptor, or `None` when nothing is registered (or the
    /// registration was cleared). A snapshot: the state derivation reads it
    /// once per derivation, never caches it.
    fn descriptor(&self) -> Option<HostTransportDescriptor>;
}

/// The crate-test double for the crossing (spec §8: the core's wiring is
/// proven against a RUST fake, never the C fake — that one lives in the
/// bridge crate with the cabi module it drives). One source of truth for
/// every core test that needs a scriptable [`HostDialer`].
#[cfg(test)]
pub(crate) mod testing {
    use std::sync::Mutex;

    use async_trait::async_trait;

    use super::*;
    use crate::ports::AsyncByteStream;

    /// A valid host name for a test descriptor (the four rules hold for every
    /// literal the tests use; a violation is a test bug, not a host's).
    pub(crate) fn name(text: &str) -> HostTransportName {
        HostTransportName::new(text.as_bytes()).expect("a valid test transport name")
    }

    /// A [`HostDialer`] whose descriptor and dial outcome are SET by the test:
    /// `Ok` hands back one end of an in-memory duplex (a real
    /// `AsyncByteStream`, never a socket); `Err(code)` maps through the ONE
    /// code→error mapping exactly as the cabi module does. Every isolation key
    /// it was handed is recorded verbatim (ADR-0545 D2: the key is passed on
    /// every dial, whatever the descriptor says about isolation).
    pub(crate) struct ScriptedHostDialer {
        descriptor: Mutex<Option<HostTransportDescriptor>>,
        outcome: Mutex<Result<(), HostDialCode>>,
        keys: Mutex<Vec<Option<String>>>,
    }

    impl ScriptedHostDialer {
        /// Registered and READY over Tor with isolation honoured, dialing `Ok`
        /// — the steady production shape.
        pub(crate) fn ready() -> Self {
            Self::with(Some(Self::tor_ready()), Ok(()))
        }

        /// NOTHING registered (`descriptor() == None`): the door's E1 shape.
        pub(crate) fn unregistered() -> Self {
            Self::with(None, Err(HostDialCode::Retired))
        }

        pub(crate) fn with(
            descriptor: Option<HostTransportDescriptor>,
            outcome: Result<(), HostDialCode>,
        ) -> Self {
            Self {
                descriptor: Mutex::new(descriptor),
                outcome: Mutex::new(outcome),
                keys: Mutex::new(Vec::new()),
            }
        }

        pub(crate) fn tor_ready() -> HostTransportDescriptor {
            HostTransportDescriptor {
                name: name("Tor"),
                readiness: HOST_TRANSPORT_READY,
                isolation: IsolationSupport::Supported,
                exposure: TransportExposure::Hidden,
                health: TransportHealth::Ready,
            }
        }

        /// The host's readiness/retire push, as the core sees it: a new
        /// descriptor (`None` = cleared).
        pub(crate) fn set_descriptor(&self, descriptor: Option<HostTransportDescriptor>) {
            *self
                .descriptor
                .lock()
                .expect("scripted descriptor poisoned") = descriptor;
        }

        /// Script the next dials: `Ok(())` succeeds, `Err(code)` fails with
        /// that host code.
        pub(crate) fn set_outcome(&self, outcome: Result<(), HostDialCode>) {
            *self.outcome.lock().expect("scripted outcome poisoned") = outcome;
        }

        /// Every isolation key handed to `dial`, in order.
        pub(crate) fn keys_seen(&self) -> Vec<Option<String>> {
            self.keys.lock().expect("scripted keys poisoned").clone()
        }
    }

    #[async_trait]
    impl NetDialer for ScriptedHostDialer {
        async fn dial(
            &self,
            _host: &str,
            _port: u16,
            isolation_key: Option<&str>,
        ) -> Result<Box<dyn AsyncByteStream>, DialError> {
            self.keys
                .lock()
                .expect("scripted keys poisoned")
                .push(isolation_key.map(str::to_owned));
            let outcome = *self.outcome.lock().expect("scripted outcome poisoned");
            match outcome {
                Ok(()) => {
                    let (a, _b) = tokio::io::duplex(64);
                    Ok(Box::new(a) as Box<dyn AsyncByteStream>)
                }
                Err(code) => Err(code
                    .into_dial_error()
                    .unwrap_or_else(|| HostDialCode::failure_from_raw(code.as_raw()))),
            }
        }
    }

    impl HostDialer for ScriptedHostDialer {
        fn descriptor(&self) -> Option<HostTransportDescriptor> {
            *self
                .descriptor
                .lock()
                .expect("scripted descriptor poisoned")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spec §3.3 / header `ZW_DIAL_*`, `ZW_ISOLATION_*`, `ZW_EXPOSURE_*` and
    /// `ZW_TRANSPORT_NAME_MAX_BYTES`: the numeric values are the ABI, so this
    /// pins each one, the round trip through `from_raw`, the refusal of every
    /// integer outside a closed set, the name's four rules (ADR-0547, T27's
    /// core half), and the ONE code→error mapping — including that only the
    /// two reachability codes map to the variants `PolicyDialer` falls back
    /// on (ADR-0546).
    #[test]
    fn the_frozen_dial_code_table_and_descriptor_ranges_are_the_abi() {
        let table = [
            (0, HostDialCode::Ok),
            (1, HostDialCode::NotReady),
            (2, HostDialCode::Unreachable),
            (3, HostDialCode::Timeout),
            (4, HostDialCode::Refused),
            (5, HostDialCode::Retired),
        ];
        for (raw, code) in table {
            assert_eq!(code.as_raw(), raw, "{code:?} is {raw} on the wire");
            assert_eq!(HostDialCode::from_raw(raw), Some(code));
        }
        assert_eq!(HostDialCode::from_raw(6), None, "the table is closed");
        assert_eq!(HostDialCode::from_raw(u32::MAX), None);

        // The ONE mapping: exactly the two reachability codes reach the
        // variants `PolicyDialer` falls back on; the other three do not.
        assert!(HostDialCode::Ok.into_dial_error().is_none());
        assert!(matches!(
            HostDialCode::NotReady.into_dial_error(),
            Some(DialError::NotReady)
        ));
        assert!(matches!(
            HostDialCode::Unreachable.into_dial_error(),
            Some(DialError::Unreachable)
        ));
        assert!(matches!(
            HostDialCode::Timeout.into_dial_error(),
            Some(DialError::Timeout)
        ));
        assert!(matches!(
            HostDialCode::Refused.into_dial_error(),
            Some(DialError::Unsupported)
        ));
        assert!(matches!(
            HostDialCode::Retired.into_dial_error(),
            Some(DialError::Retired)
        ));
        let falls_back = |code: HostDialCode| {
            matches!(
                code.into_dial_error(),
                Some(DialError::Unreachable | DialError::Timeout)
            )
        };
        assert!(falls_back(HostDialCode::Unreachable) && falls_back(HostDialCode::Timeout));
        assert!(
            !falls_back(HostDialCode::NotReady)
                && !falls_back(HostDialCode::Refused)
                && !falls_back(HostDialCode::Retired),
            "not-ready, refused and retired never reach the clearnet fallback"
        );
        assert!(
            matches!(HostDialCode::failure_from_raw(9), DialError::Io(_)),
            "an unknown code on a failure path is a protocol violation"
        );
        assert!(matches!(
            HostDialCode::failure_from_raw(0),
            DialError::Io(_)
        ));
        assert!(matches!(
            HostDialCode::failure_from_raw(1),
            DialError::NotReady
        ));

        // The descriptor: two closed value sets, readiness bounded at 100, and
        // the host's NAME under exactly four rules (ADR-0547; header
        // `ZW_TRANSPORT_NAME_MAX_BYTES` / `ZW_EXPOSURE_*`).
        for (raw, iso) in [
            (0, IsolationSupport::Unknown),
            (1, IsolationSupport::Supported),
            (2, IsolationSupport::Unsupported),
        ] {
            assert_eq!(iso.as_raw(), raw);
            assert_eq!(IsolationSupport::from_raw(raw), Some(iso));
        }
        assert_eq!(IsolationSupport::from_raw(3), None);
        for (raw, exposure) in [
            (0, TransportExposure::Unknown),
            (1, TransportExposure::Hidden),
            (2, TransportExposure::Exposed),
        ] {
            assert_eq!(exposure.as_raw(), raw);
            assert_eq!(TransportExposure::from_raw(raw), Some(exposure));
        }
        assert_eq!(
            TransportExposure::from_raw(3),
            None,
            "the exposure set is closed"
        );
        // The health axis (ABI v3, ADR-0549): closed at three values.
        for (raw, health) in [
            (0, TransportHealth::Starting),
            (1, TransportHealth::Ready),
            (2, TransportHealth::Failed),
        ] {
            assert_eq!(health.as_raw(), raw);
            assert_eq!(TransportHealth::from_raw(raw), Some(health));
        }
        assert_eq!(
            TransportHealth::from_raw(3),
            None,
            "the health set is closed"
        );
        // The health AXIS arrived at v3; v4 changed what FAILED MEANS
        // under `Preferred` — the axis itself is unmoved, so this row pins the
        // live version and names why it is no longer 3.
        assert_eq!(
            HOST_DIALER_ABI_VERSION, 4,
            "the health axis arrived at v3; v4 is FAILED becoming \
             Preferred-switch-eligible (ADR-0553 as built)"
        );
        let ready = HostTransportDescriptor::from_raw(b"Tor", 100, 1, 1, 1).expect("in range");
        assert!(ready.is_ready());
        assert_eq!(
            ready,
            HostTransportDescriptor {
                name: HostTransportName::new(b"Tor").expect("valid"),
                readiness: HOST_TRANSPORT_READY,
                isolation: IsolationSupport::Supported,
                exposure: TransportExposure::Hidden,
                health: TransportHealth::Ready,
            }
        );
        assert_eq!(ready.name.as_str(), "Tor");
        let bootstrapping =
            HostTransportDescriptor::from_raw(b"Shadowsocks", 37, 2, 0, 0).expect("in range");
        assert!(!bootstrapping.is_ready());
        assert!(!bootstrapping.is_failed());
        assert_eq!(bootstrapping.readiness, 37);
        assert_eq!(bootstrapping.exposure, TransportExposure::Unknown);
        assert_eq!(bootstrapping.health, TransportHealth::Starting);
        // `health` never permits a dial and FAILED forbids one: readiness
        // alone opens the gate, and a FAILED descriptor is not ready at ANY
        // readiness — it keeps the readiness the registrant measured.
        let failed = HostTransportDescriptor::from_raw(b"Tor", 40, 1, 1, 2).expect("in range");
        assert!(failed.is_failed() && !failed.is_ready());
        assert_eq!(failed.readiness, 40, "FAILED keeps the measured readiness");
        let failed_at_100 =
            HostTransportDescriptor::from_raw(b"Tor", 100, 1, 1, 2).expect("in range");
        assert!(
            !failed_at_100.is_ready(),
            "a FAILED transport is never ready, whatever its readiness"
        );
        let ready_health_at_40 =
            HostTransportDescriptor::from_raw(b"Tor", 40, 1, 1, 1).expect("in range");
        assert!(
            !ready_health_at_40.is_ready(),
            "health READY below readiness 100 opens nothing"
        );
        assert!(
            HostTransportDescriptor::from_raw(b"Tor", 101, 1, 1, 1).is_none(),
            "readiness above 100 is refused at the boundary"
        );
        assert!(HostTransportDescriptor::from_raw(b"Tor", 100, 3, 1, 1).is_none());
        assert!(HostTransportDescriptor::from_raw(b"Tor", 100, 1, 3, 1).is_none());
        assert!(
            HostTransportDescriptor::from_raw(b"Tor", 100, 1, 1, 3).is_none(),
            "a health outside the closed set is refused at the boundary"
        );

        // The name's four rules, and nothing else: non-empty, <= 32 bytes,
        // UTF-8, no control character. Anything the host calls its transport
        // passes otherwise — the SDK has no list (ADR-0547).
        assert_eq!(HostTransportName::new(b""), None, "empty is refused");
        assert!(
            HostTransportName::new(&[b'a'; 32]).is_some(),
            "32 bytes fit"
        );
        assert_eq!(HostTransportName::new(&[b'a'; 33]), None, "33 bytes do not");
        assert!(
            HostTransportName::new("VLESS via Cloudflare".as_bytes()).is_some(),
            "spaces and mixed case are the host's business"
        );
        let cjk = "洋葱路由器"; // five three-byte characters
        assert_eq!(
            HostTransportName::new(cjk.as_bytes()).map(|n| n.as_str().to_owned()),
            Some(cjk.to_owned()),
            "multi-byte UTF-8 crosses verbatim"
        );
        assert!(
            HostTransportName::new("😀😀😀😀😀😀😀😀".as_bytes()).is_some(),
            "eight four-byte characters are exactly 32 bytes"
        );
        assert_eq!(
            HostTransportName::new(&[0xFF, b'T', b'o', b'r']),
            None,
            "not UTF-8"
        );
        assert_eq!(
            HostTransportName::new(b"Tor\0"),
            None,
            "a counted NUL is a control char"
        );
        assert_eq!(HostTransportName::new(b"Tor\n"), None, "C0 control");
        assert_eq!(HostTransportName::new("Tor\u{7f}".as_bytes()), None, "DEL");
        assert_eq!(
            HostTransportName::new("Tor\u{85}".as_bytes()),
            None,
            "C1 control"
        );
        // Rule 4's format/separator set (the four angles' fold): a name that
        // could re-order, hide or line-break the privacy sentence after it.
        for (what, text) in [
            ("a bidi override", "Tor\u{202E}"),
            ("a bidi isolate", "\u{2066}Tor\u{2069}"),
            ("a zero-width space", "To\u{200B}r"),
            ("a right-to-left mark", "Tor\u{200F}"),
            ("a byte-order mark", "\u{FEFF}Tor"),
            ("a line separator", "Tor\u{2028}"),
            ("a paragraph separator", "Tor\u{2029}"),
        ] {
            assert_eq!(HostTransportName::new(text.as_bytes()), None, "{what}");
        }
        assert_eq!(HostTransportName::new(b"   "), None, "blank is refused");
        assert_eq!(HostTransportName::new(b"\t"), None, "a tab is a C0 control");
        assert!(
            HostTransportName::new(b" Tor ").is_some(),
            "surrounding spaces are the host's business once one visible char exists"
        );
        assert_eq!(
            format!("{:?}", HostTransportName::new(b"Tor").expect("valid")),
            "HostTransportName(<3 bytes>)",
            "Debug redacts the text (spec §5: never logged)"
        );
        assert!(
            HostTransportDescriptor::from_raw(b"", 100, 1, 1, 1).is_none(),
            "a bad name refuses the whole descriptor"
        );
        assert!(HostTransportName::UNATTRIBUTED.is_unattributed());
        assert_eq!(HostTransportName::UNATTRIBUTED.as_str(), "");
        assert!(
            !ready.name.is_unattributed(),
            "a host's name is never the unattributed one"
        );
    }

    proptest::proptest! {
        /// The name parser is a boundary over HOST-CONTROLLED bytes (spec §4;
        /// the code reviewer's testing-rigor MAJOR): over arbitrary byte
        /// strings up to past the bound it never panics; whatever it accepts
        /// was within the bound, non-empty, valid UTF-8 with no control or
        /// format character, and round-trips byte for byte through
        /// `as_str`; whatever it refuses broke at least one of the four
        /// rules stated independently of the implementation.
        #[test]
        fn a_transport_name_never_panics_and_round_trips(
            bytes in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..=40)
        ) {
            let rules_hold = !bytes.is_empty()
                && bytes.len() <= HOST_TRANSPORT_NAME_MAX_BYTES
                && std::str::from_utf8(&bytes).is_ok_and(|text| {
                    !text.trim().is_empty()
                        && !text.chars().any(|c| {
                            c.is_control()
                                || ('\u{200B}'..='\u{200F}').contains(&c)
                                || ('\u{202A}'..='\u{202E}').contains(&c)
                                || ('\u{2066}'..='\u{2069}').contains(&c)
                                || matches!(c, '\u{2028}' | '\u{2029}' | '\u{FEFF}')
                        })
                });
            match HostTransportName::new(&bytes) {
                Some(name) => {
                    proptest::prop_assert!(rules_hold, "accepted a name that breaks a rule");
                    proptest::prop_assert_eq!(name.as_str().as_bytes(), &bytes[..]);
                    proptest::prop_assert!(!name.is_unattributed());
                    proptest::prop_assert_eq!(
                        format!("{name:?}"),
                        format!("HostTransportName(<{} bytes>)", bytes.len())
                    );
                }
                None => proptest::prop_assert!(!rules_hold, "refused a name that keeps every rule"),
            }
        }
    }
}
