//! Memos & payment requests (spec §2.4 — ZIP 302 / ZIP 321 semantics).
//! We do not invent memo framing: arms follow librustzcash's `Memo`
//! (audit A4); URI parsing is REUSED from the `zip321` crate (we write NO
//! URI parser — see `payment_uri.rs` for the validation funnel).

use zcash_protocol::memo::MemoBytes;

use crate::constants::{
    MACHINE_MEMO_PREFIX_MAX_BYTES, MACHINE_MEMO_PREFIX_MAX_COUNT, MEMO_ARBITRARY_MAX_BYTES,
    MEMO_TEXT_MAX_BYTES,
};
use crate::error::WalletError;
use crate::money::{Network, Zatoshis};

/// ZIP-302 memo. `MEMO_BYTES = 512` on the wire; `Arbitrary` carries 511
/// behind the 0xFF tag.
///
/// **`Debug` is HAND-WRITTEN and redacts content — see the impl below.** A
/// derived one hex-dumps the `Arbitrary` payload and prints `Text` verbatim,
/// both of which are §5.4 never-log material.
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Memo {
    Empty,
    /// UTF-8 text, ≤ `MEMO_TEXT_MAX_BYTES` — construct via [`Memo::text`].
    Text(String),
    /// Up to 511 opaque bytes (0xFF tag); the host's typed envelope (e.g.
    /// tip metadata) rides here (A6) — construct via [`Memo::arbitrary`].
    /// WIRE REALITY: shorter payloads are zero-padded to 511 on the wire and
    /// round-trip back as the FULL 511 — hosts must frame length INSIDE the
    /// bytes (see [`Memo::arbitrary`]).
    Arbitrary(Vec<u8>),
    /// ZIP-302 reserved space (0xF5, non-canonical 0xF6, 0xF7..=0xFE):
    /// RECEIVABLE as opaque bytes (it exists on-chain — dropping it would
    /// lie); construction-for-send is REJECTED at payment construction
    /// (§2.4 — we never emit reserved framing).
    Reserved(Vec<u8>),
}

/// §5.4 GUARD, not a formatting preference. Memo content is as sensitive as a
/// message body, and a DERIVED `Debug` prints all of it: upstream hex-dumps the
/// `Arbitrary` payload and `Text` renders verbatim. Nothing formats a `Memo`
/// today, but `{:?}` is one keystroke inside a `#[instrument]` span, and the
/// leak it produces is invisible in review — the derive is what would have made
/// it silent (post-build review; closed).
///
/// The shape mirrors the FFI's own display projection (`ParsedMemo`): presence
/// and LENGTH, never bytes. Length is safe — the wire field is a fixed 512 and
/// `has_memo` already discloses presence.
impl std::fmt::Debug for Memo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "Memo::Empty"),
            Self::Text(t) => write!(f, "Memo::Text(<{} bytes>)", t.len()),
            Self::Arbitrary(b) => write!(f, "Memo::Arbitrary(<{} bytes>)", b.len()),
            Self::Reserved(b) => write!(f, "Memo::Reserved(<{} bytes>)", b.len()),
        }
    }
}

impl Memo {
    /// Validated text memo (byte length, not char count — the wire field is
    /// bytes).
    pub fn text(s: impl Into<String>) -> Result<Self, WalletError> {
        let s = s.into();
        if s.len() > MEMO_TEXT_MAX_BYTES {
            return Err(WalletError::MemoTooLong {
                len: s.len(),
                max: MEMO_TEXT_MAX_BYTES,
            });
        }
        Ok(Self::Text(s))
    }

    /// Validated arbitrary-bytes memo (0xFF arm).
    ///
    /// WIRE NOTE: the ZIP-302 0xFF field has NO length framing — fewer than
    /// 511 bytes are zero-padded on the wire and come back as the full 511.
    /// A host that needs exact lengths must frame INSIDE the bytes (A6 —
    /// a host's typed tip envelope does).
    pub fn arbitrary(bytes: Vec<u8>) -> Result<Self, WalletError> {
        if bytes.len() > MEMO_ARBITRARY_MAX_BYTES {
            return Err(WalletError::MemoTooLong {
                len: bytes.len(),
                max: MEMO_ARBITRARY_MAX_BYTES,
            });
        }
        Ok(Self::Arbitrary(bytes))
    }

    /// ZIP-302 classification of a raw 512-byte memo field, REUSED from
    /// zcash_protocol (audit A4 — the arms below only RELABEL upstream's
    /// verdict, never re-derive it). Total over hostile bytes: the one
    /// upstream-invalid shape (text lead, non-UTF-8 payload) is a typed
    /// [`WalletError::MemoInvalid`], never a panic.
    pub(crate) fn from_wire(bytes: &MemoBytes) -> Result<Self, WalletError> {
        use zcash_protocol::memo::Memo as ZMemo;
        match ZMemo::try_from(bytes).map_err(|_| WalletError::MemoInvalid)? {
            ZMemo::Empty => Ok(Self::Empty),
            // upstream guarantees text ≤ 512 field bytes — re-bounded anyway
            ZMemo::Text(t) => Self::text(String::from(t)),
            // full 511 bytes, padding INCLUDED (no length framing — see
            // `Memo::arbitrary`); trimming would corrupt opaque data
            ZMemo::Arbitrary(b) => Ok(Self::Arbitrary(b.to_vec())),
            // reserved framing carried whole (all 512 bytes, lead included)
            // so nothing is lost; send-construction rejects it downstream
            ZMemo::Future(mb) => Ok(Self::Reserved(mb.into_bytes().to_vec())),
        }
    }

    /// ZIP-302 serialization, REUSED from zcash_protocol. `Reserved` is a
    /// typed error — we NEVER emit reserved framing (§2.4); [`Payment::new`]
    /// already rejects it, this is the total-function backstop.
    pub(crate) fn to_wire(&self) -> Result<MemoBytes, WalletError> {
        use zcash_protocol::memo::Memo as ZMemo;
        match self {
            Self::Empty => Ok(MemoBytes::empty()),
            Self::Text(s) => {
                // bounds re-checked by upstream; unreachable for anything
                // built via `Memo::text` (≤ MEMO_TEXT_MAX_BYTES) — a typed
                // backstop for crate-internal variant literals, not a no-op
                let m: ZMemo = s.parse().map_err(|_| WalletError::MemoTooLong {
                    len: s.len(),
                    max: MEMO_TEXT_MAX_BYTES,
                })?;
                Ok(m.encode())
            }
            Self::Arbitrary(bytes) => {
                // The bound is re-checked here, not assumed. `Memo::arbitrary`
                // enforces it and `from_wire` can only produce exactly 511 —
                // but `Memo` is a pub enum with pub variants, so a downstream
                // crate can write the variant literal directly, and
                // `#[non_exhaustive]` restricts exhaustive MATCHING, not
                // construction. Without this arm an over-long literal indexes
                // `padded` out of range and PANICS, in a function whose own doc
                // calls itself a total-function backstop. Its two siblings
                // already return typed errors; this one used to be the
                // exception.
                if bytes.len() > MEMO_ARBITRARY_MAX_BYTES {
                    return Err(WalletError::MemoTooLong {
                        len: bytes.len(),
                        max: MEMO_ARBITRARY_MAX_BYTES,
                    });
                }
                let mut padded = [0u8; MEMO_ARBITRARY_MAX_BYTES];
                padded[..bytes.len()].copy_from_slice(bytes);
                Ok(ZMemo::Arbitrary(Box::new(padded)).encode())
            }
            Self::Reserved(_) => Err(WalletError::ReservedMemoNotSendable),
        }
    }
}

/// FR-27 — a host's declared MACHINE-MEMO READ SCOPE: the leading bytes an
/// `Arbitrary` (`0xFF`) memo must carry for [`Wallet::machine_memos`] to hand
/// it back over the FFI.
///
/// [`Wallet::machine_memos`]: crate::Wallet::machine_memos
///
/// **This is a VOLUME filter, not a security boundary.** Forging a prefix costs
/// an attacker nothing — anyone can write your magic bytes into a memo and pay
/// you a zatoshi — so what a prefix buys is that unrelated envelopes on the same
/// chain do not reach your parser, NOT that what reaches it is yours. The host's
/// own fail-closed parser sits on the hostile path by default.
///
/// **It also does not mean "written by this device."** Bob's wallet never wrote
/// Alice's memo; a scope that meant authorship would break the receive side
/// outright, which is the whole use.
///
/// Construction is the only door, so an unusable scope is unrepresentable: an
/// EMPTY prefix (which would match every memo ever written — the "returns
/// everything" failure FR-27's acceptance names) and one longer than
/// [`MACHINE_MEMO_PREFIX_MAX_BYTES`] (which could never match a bounded memo)
/// are typed refusals.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MachineMemoPrefix(Vec<u8>);

impl MachineMemoPrefix {
    pub fn new(bytes: Vec<u8>) -> Result<Self, WalletError> {
        if bytes.is_empty() {
            return Err(WalletError::MachineMemoScopeInvalid {
                reason: "prefix is empty (it would match every memo)",
            });
        }
        if bytes.len() > MACHINE_MEMO_PREFIX_MAX_BYTES {
            return Err(WalletError::MachineMemoScopeInvalid {
                reason: "prefix longer than the registered maximum",
            });
        }
        Ok(Self(bytes))
    }

    /// Does `memo_bytes` begin with this prefix? The whole matching rule, in
    /// one place: a leading-byte comparison and nothing else. The SDK does not
    /// know what a magic, a version or a length field is here, and must not —
    /// that is what keeps a host's envelope format the host's problem.
    pub(crate) fn matches(&self, memo_bytes: &[u8]) -> bool {
        memo_bytes.starts_with(&self.0)
    }
}

/// Validate a whole registered scope (the config door). Bounds the COUNT as
/// well as each prefix, because every registered prefix is compared against
/// every machine memo of the transaction being read — the one place a hostile
/// transaction's cost is bounded.
pub fn validate_machine_memo_scope(prefixes: &[MachineMemoPrefix]) -> Result<(), WalletError> {
    if prefixes.len() > MACHINE_MEMO_PREFIX_MAX_COUNT {
        return Err(WalletError::MachineMemoScopeInvalid {
            reason: "more prefixes registered than the maximum",
        });
    }
    Ok(())
}

/// THE FR-27 read filter — the whole rule for what leaves the wallet as opaque
/// bytes, in one total function over hostile input.
///
/// Kept here (not inline in [`Wallet::machine_memos`](crate::Wallet::machine_memos))
/// so the rule is testable without a platform vault and a funded chain: this is
/// the part an attacker's memo actually reaches.
///
/// - `Arbitrary` (`0xFF`) ONLY. `Reserved` never yields bytes and `Text` has its
///   own lane; the match is EXHAUSTIVE — every arm named, no wildcard — so
///   adding an arm to [`Memo`] is a COMPILE ERROR here and a future core arm has
///   to be a decision instead of an inheritance. (`Memo` is `#[non_exhaustive]`,
///   which restricts exhaustive matching from OTHER crates; inside this one the
///   match compiles and the gate is real. It said "exhaustive-by-intent" over a
///   `_ => None` until the FR-27 crypto audit read the two together — the safe
///   direction, and a false claim in the one comment that documents this rule.)
/// - Over-long bytes are DROPPED, never truncated — a truncation would hand the
///   host a prefix of a memo that looks like a shorter, valid envelope.
/// - Then, and only then, the prefix match.
pub(crate) fn filter_machine_memos(
    memos: Vec<Memo>,
    prefixes: &[MachineMemoPrefix],
) -> Vec<Vec<u8>> {
    memos
        .into_iter()
        .filter_map(|memo| match memo {
            Memo::Arbitrary(bytes) => Some(bytes),
            Memo::Empty | Memo::Text(_) | Memo::Reserved(_) => None,
        })
        .filter(|bytes| bytes.len() <= MEMO_ARBITRARY_MAX_BYTES)
        .filter(|bytes| prefixes.iter().any(|prefix| prefix.matches(bytes)))
        .collect()
}

/// Fuzz-only facade over the FR-27 read path (testing-patterns §3): classify
/// raw wire bytes exactly as an on-chain memo would be, then run the scope
/// filter. `#[doc(hidden)]` — NOT a supported API.
///
/// It takes WIRE bytes rather than a `Memo` on purpose: the whole point is that
/// the fuzzer reaches the classifier the same way an attacker's memo does, so
/// the `Reserved`/`Text`/invalid arms are explored instead of assumed.
#[cfg(fuzzing)]
#[doc(hidden)]
pub fn __fuzz_filter_machine_memos(memo_wire: &[u8], prefix: &[u8]) -> Vec<Vec<u8>> {
    let Ok(prefix) = MachineMemoPrefix::new(prefix.to_vec()) else {
        return Vec::new();
    };
    let Ok(mb) = MemoBytes::from_bytes(memo_wire) else {
        return Vec::new();
    };
    let Ok(memo) = Memo::from_wire(&mb) else {
        return Vec::new();
    };
    filter_machine_memos(vec![memo], std::slice::from_ref(&prefix))
}

/// Address kind — drives the memo/transparent construction check and the
/// §5.1 disclosure honesty.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum AddressKind {
    Unified,
    Sapling,
    Transparent,
}

/// A parsed, network-checked Zcash address — [`Address::parse`] is the only
/// public door, so hosts never see an unvalidated `Address`.
///
/// **`Debug` is HAND-WRITTEN and never prints the encoding** — addresses are on
/// the §5.4 never-log list, same as memo content.
#[derive(Clone, PartialEq, Eq)]
pub struct Address {
    encoded: String,
    kind: AddressKind,
    network: Network,
    /// From the AUDITED predicate (`ZcashAddress::can_receive_memo`, one
    /// source of truth): a UA counts only if it actually CARRIES a shielded
    /// receiver (a transparent-only UA exists and cannot take a memo).
    memo_capable: bool,
    /// From `ZcashAddress::is_transparent_only` — TRUE also for a UA whose
    /// only recognized receivers are transparent (which still classifies as
    /// `AddressKind::Unified`, so `kind` alone must never drive consensus
    /// rules like the zero-valued-output check).
    transparent_only: bool,
}

/// §5.4: the encoding NEVER appears. Kind + capability are the facts a log or a
/// panic message legitimately needs ("a transparent recipient refused a memo"),
/// and neither identifies anyone.
impl std::fmt::Debug for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Address")
            .field("kind", &self.kind)
            .field("network", &self.network)
            .field("memo_capable", &self.memo_capable)
            .field("transparent_only", &self.transparent_only)
            .finish_non_exhaustive()
    }
}

impl Address {
    /// Parse + network-check (§4.6 hostile input: user paste, QR, provider
    /// strings). Size-capped before parse; an address from the OTHER network
    /// reports `NetworkMismatch` (renderable: "this is a testnet address")
    /// rather than collapsing into malformed.
    pub fn parse(s: &str, network: Network) -> Result<Self, WalletError> {
        use crate::constants::ADDRESS_MAX_BYTES;
        use zcash_protocol::consensus::NetworkType;
        if s.len() > ADDRESS_MAX_BYTES {
            return Err(WalletError::AddressInvalid);
        }
        let zaddr = zcash_address::ZcashAddress::try_from_encoded(s)
            .map_err(|_| WalletError::AddressInvalid)?;
        // capability predicates from the audited crate, BEFORE the kind
        // conversion consumes the value (one source of truth — never
        // re-derived from receiver lists by hand)
        let memo_capable = zaddr.can_receive_memo();
        let transparent_only = zaddr.is_transparent_only();
        let parsed = zaddr
            .convert::<ParsedKind>()
            .map_err(|_| WalletError::AddressInvalid)?;
        let want = match network {
            Network::Main => NetworkType::Main,
            Network::Test => NetworkType::Test,
        };
        if parsed.net != want {
            return Err(WalletError::NetworkMismatch);
        }
        Ok(Self {
            encoded: s.to_owned(),
            kind: parsed.kind,
            network,
            memo_capable,
            transparent_only,
        })
    }

    pub fn encoded(&self) -> &str {
        &self.encoded
    }

    pub fn kind(&self) -> AddressKind {
        self.kind
    }

    pub fn network(&self) -> Network {
        self.network
    }

    /// Whether this address can receive a memo (shielded component present).
    /// Public: drives the compose UX ("memo not available for this
    /// recipient") — the predicate itself comes from the audited crate.
    pub fn memo_capable(&self) -> bool {
        self.memo_capable
    }

    /// Whether the only recognized receivers are transparent (see field doc).
    pub(crate) fn transparent_only(&self) -> bool {
        self.transparent_only
    }
}

/// Classification target for the audited decoder's conversion seam
/// (zcash_address, taken whole — we never look at address bytes ourselves).
/// Capability predicates (memo/transparent-only) come from `ZcashAddress`
/// directly; this conversion only yields net + kind.
struct ParsedKind {
    net: zcash_protocol::consensus::NetworkType,
    kind: AddressKind,
}

impl zcash_address::TryFromAddress for ParsedKind {
    type Error = std::convert::Infallible;

    // Sprout is intentionally NOT implemented: the obsolete pool is
    // unspendable-to from this wallet — the default `Unsupported` maps to
    // `AddressInvalid`, an honest reject rather than a doomed send.

    fn try_from_sapling(
        net: zcash_protocol::consensus::NetworkType,
        _data: [u8; 43],
    ) -> Result<Self, zcash_address::ConversionError<Self::Error>> {
        Ok(Self {
            net,
            kind: AddressKind::Sapling,
        })
    }

    fn try_from_unified(
        net: zcash_protocol::consensus::NetworkType,
        _data: zcash_address::unified::Address,
    ) -> Result<Self, zcash_address::ConversionError<Self::Error>> {
        Ok(Self {
            net,
            kind: AddressKind::Unified,
        })
    }

    fn try_from_transparent_p2pkh(
        net: zcash_protocol::consensus::NetworkType,
        _data: [u8; 20],
    ) -> Result<Self, zcash_address::ConversionError<Self::Error>> {
        Ok(Self {
            net,
            kind: AddressKind::Transparent,
        })
    }

    fn try_from_transparent_p2sh(
        net: zcash_protocol::consensus::NetworkType,
        _data: [u8; 20],
    ) -> Result<Self, zcash_address::ConversionError<Self::Error>> {
        Ok(Self {
            net,
            kind: AddressKind::Transparent,
        })
    }

    fn try_from_tex(
        net: zcash_protocol::consensus::NetworkType,
        _data: [u8; 20],
    ) -> Result<Self, zcash_address::ConversionError<Self::Error>> {
        // TEX (ZIP 320) is transparent-source-restricted — classified
        // transparent here (memo-incapable); send-path rules come with the
        // send chunk.
        Ok(Self {
            net,
            kind: AddressKind::Transparent,
        })
    }
}

/// One payment leg of a ZIP-321 request. Constructed only through
/// [`Payment::new`] so every instance is construction-validated.
///
/// **`Debug` is HAND-WRITTEN.** A leg carries all THREE never-log classes at
/// once — recipient address, amount, memo content — and `label`/`message` are
/// caller-supplied free text. A derived `Debug` on this type is the widest
/// single §5.4 leak in the crate.
#[derive(Clone, PartialEq, Eq)]
pub struct Payment {
    pub recipient: Address,
    /// ZIP-321 allows a request to leave the amount to the SENDER (`None` —
    /// the bare donation-QR `zcash:ADDR` form). A send intent needs `Some`:
    /// `propose()` rejects amount-less legs typed (the parse→prefill→confirm
    /// UX fills it in between).
    pub amount: Option<Zatoshis>,
    pub memo: Memo,
    /// ZIP-321 display fields — never sent on-chain.
    pub label: Option<String>,
    pub message: Option<String>,
}

/// §5.4: presence, never values. `amount` is a never-log class of its own, and
/// `label`/`message` are caller free text that routinely names the payee.
impl std::fmt::Debug for Payment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Payment")
            .field("recipient", &self.recipient) // itself redacted, above
            .field("amount", &self.amount.map(|_| "<redacted>"))
            .field("memo", &self.memo) // itself redacted, above
            .field("label", &self.label.as_ref().map(|_| "<redacted>"))
            .field("message", &self.message.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

impl Payment {
    /// Construction-time validation (§2.4 / §6.1): reserved memos are not
    /// sendable; a memo to a transparent recipient is a typed error (the
    /// protocol cannot carry it — honest degradation per ADR-0005 r3); a
    /// zero-valued output to a transparent-only recipient is disallowed by
    /// consensus (the audited zip321 rule, mirrored so OUR boundary is
    /// complete — note `transparent_only`, not `kind`: a transparent-only
    /// UA still classifies as `Unified`).
    pub fn new(
        recipient: Address,
        amount: Option<Zatoshis>,
        memo: Memo,
        label: Option<String>,
        message: Option<String>,
    ) -> Result<Self, WalletError> {
        if matches!(memo, Memo::Reserved(_)) {
            return Err(WalletError::ReservedMemoNotSendable);
        }
        if !matches!(memo, Memo::Empty) && !recipient.memo_capable() {
            return Err(WalletError::MemoRequiresShieldedRecipient);
        }
        if recipient.transparent_only() && amount == Some(Zatoshis::ZERO) {
            return Err(WalletError::ZeroValuedTransparentOutput);
        }
        Ok(Self {
            recipient,
            amount,
            memo,
            label,
            message,
        })
    }
}

/// ZIP-321 payment request: built programmatically or parsed from a
/// `zcash:` URI (hostile input — §4.6; size-capped before parse, fuzzed:
/// `fuzz_payment_uri`). Codec: `payment_uri::{parse_payment_uri,
/// encode_payment_uri}`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PaymentRequest {
    pub payments: Vec<Payment>,
}

impl PaymentRequest {
    /// `true` if any leg omits its amount (the ZIP-321 donation form `zcash:ADDR`):
    /// valid to DISPLAY, never to SEND. The SSOT for the send funnels' shared
    /// `SendAmountRequired` gate — both `Wallet::propose` and `Wallet::queue_send`
    /// reject on this one predicate, so the rule can never drift between them.
    pub(crate) fn has_amount_less_leg(&self) -> bool {
        self.payments.iter().any(|p| p.amount.is_none())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::WalletError;
    use zcash_address::ToAddress;
    use zcash_protocol::consensus::NetworkType;

    // Valid-by-construction test addresses: built through the SAME audited
    // encoder the parser uses (checksums real, payload bytes arbitrary —
    // zcash_address is pure encoding, curve validity is a spend-time concern).

    fn encoded_sapling(net: NetworkType) -> String {
        zcash_address::ZcashAddress::from_sapling(net, [0xAB; 43]).encode()
    }

    fn encoded_p2pkh(net: NetworkType) -> String {
        zcash_address::ZcashAddress::from_transparent_p2pkh(net, [0xCD; 20]).encode()
    }

    fn encoded_unified(net: &NetworkType) -> String {
        use zcash_address::unified::{Address as Ua, Encoding, Receiver};
        Ua::try_from_items(vec![
            Receiver::Orchard([0xEF; 43]),
            Receiver::Sapling([0xAB; 43]),
        ])
        .expect("valid receiver set")
        .encode(net)
    }

    fn shielded() -> Address {
        Address::parse(&encoded_sapling(NetworkType::Test), Network::Test).expect("parses")
    }

    fn transparent() -> Address {
        Address::parse(&encoded_p2pkh(NetworkType::Test), Network::Test).expect("parses")
    }

    #[test]
    fn memo_length_bounds() {
        assert!(Memo::text("a".repeat(MEMO_TEXT_MAX_BYTES)).is_ok());
        assert!(matches!(
            Memo::text("a".repeat(MEMO_TEXT_MAX_BYTES + 1)),
            Err(WalletError::MemoTooLong { .. })
        ));
        // byte length, not char count: 'é' is 2 bytes
        assert!(matches!(
            Memo::text("é".repeat(MEMO_TEXT_MAX_BYTES / 2 + 1)),
            Err(WalletError::MemoTooLong { .. })
        ));
        assert!(Memo::arbitrary(vec![0u8; MEMO_ARBITRARY_MAX_BYTES]).is_ok());
        assert!(Memo::arbitrary(vec![0u8; MEMO_ARBITRARY_MAX_BYTES + 1]).is_err());
    }

    #[test]
    fn memo_to_transparent_recipient_rejected_at_construction() {
        // §8 named test: ADR-0005 r3 honest degradation.
        let amount = Some(Zatoshis::new(1_000).expect("valid"));
        let memo = Memo::text("thanks!").expect("valid");
        assert!(matches!(
            Payment::new(transparent(), amount, memo.clone(), None, None),
            Err(WalletError::MemoRequiresShieldedRecipient)
        ));
        // same memo to a shielded recipient is fine
        assert!(Payment::new(shielded(), amount, memo, None, None).is_ok());
        // empty memo to transparent is fine
        assert!(Payment::new(transparent(), amount, Memo::Empty, None, None).is_ok());
    }

    #[test]
    fn reserved_memo_never_sendable() {
        let amount = Some(Zatoshis::new(1).expect("valid"));
        assert!(matches!(
            Payment::new(shielded(), amount, Memo::Reserved(vec![0xF5]), None, None),
            Err(WalletError::ReservedMemoNotSendable)
        ));
    }

    #[test]
    fn zero_valued_transparent_output_rejected_amount_less_allowed() {
        // consensus rule mirrored from the audited zip321 constructor
        assert!(matches!(
            Payment::new(transparent(), Some(Zatoshis::ZERO), Memo::Empty, None, None),
            Err(WalletError::ZeroValuedTransparentOutput)
        ));
        // zero to a SHIELDED recipient is legal…
        assert!(Payment::new(shielded(), Some(Zatoshis::ZERO), Memo::Empty, None, None).is_ok());
        // …and an amount-LESS transparent leg is a valid request (donation
        // QR — the sender specifies; zero is only rejected when stated)
        assert!(Payment::new(transparent(), None, Memo::Empty, None, None).is_ok());
    }

    #[test]
    fn memo_512_byte_boundary_invalid_utf8_and_reserved_range_handled() {
        // §8 named test: ZIP-302 semantics incl. the reserved arm (A4).
        // 512-byte text boundary survives the wire round-trip
        let max_text = "a".repeat(MEMO_TEXT_MAX_BYTES);
        let m = Memo::text(max_text.clone()).expect("max text valid");
        let wire = m.to_wire().expect("sendable");
        assert_eq!(
            Memo::from_wire(&wire).expect("classifies"),
            Memo::Text(max_text)
        );

        // invalid UTF-8 behind a text lead (≤0xF4) is the ONE upstream-
        // invalid shape — typed, payload-free, never a panic
        let bad = MemoBytes::from_bytes(&[0x41, 0xFF, 0xFE]).expect("fits");
        assert!(matches!(
            Memo::from_wire(&bad),
            Err(WalletError::MemoInvalid)
        ));

        // canonical 0xF6‖zeros is Empty…
        assert_eq!(
            Memo::from_wire(&MemoBytes::empty()).expect("empty"),
            Memo::Empty
        );
        // …reserved leads (0xF5, 0xF7..=0xFE) are RECEIVABLE as opaque
        // bytes (full 512, lead included) and send-rejected
        for lead in [0xF5u8, 0xF7, 0xFE] {
            let mb = MemoBytes::from_bytes(&[lead, 1, 2, 3]).expect("fits");
            let m = Memo::from_wire(&mb).expect("receivable as opaque");
            assert!(matches!(m, Memo::Reserved(ref b) if b.len() == 512 && b[0] == lead));
            assert!(matches!(
                m.to_wire(),
                Err(WalletError::ReservedMemoNotSendable)
            ));
        }
        // …and a NON-canonical 0xF6 (any nonzero tail) is Reserved, not Empty
        let mut nc = [0u8; 512];
        nc[0] = 0xF6;
        nc[511] = 1;
        let mb = MemoBytes::from_bytes(&nc).expect("exactly 512");
        assert!(matches!(
            Memo::from_wire(&mb).expect("reserved"),
            Memo::Reserved(_)
        ));

        // 0xFF arbitrary: comes back as the FULL 511 bytes, zero-padding
        // included — the wire has no length framing (documented; A6 hosts
        // frame inside)
        let m = Memo::arbitrary(vec![9, 8, 7]).expect("valid");
        let wire = m.to_wire().expect("sendable");
        let Memo::Arbitrary(bytes) = Memo::from_wire(&wire).expect("classifies") else {
            panic!("expected the arbitrary arm");
        };
        assert_eq!(bytes.len(), MEMO_ARBITRARY_MAX_BYTES);
        assert_eq!(&bytes[..3], &[9, 8, 7]);
        assert!(bytes[3..].iter().all(|&b| b == 0));
    }

    #[test]
    fn debug_never_carries_memo_content_address_or_amount() {
        // §5.4 GUARD (debt, closed). The derived `Debug` on these
        // types printed everything: upstream hex-dumps the `Arbitrary` payload,
        // `Text` renders verbatim, `Address` prints its encoding, `Payment`
        // prints an amount. Nothing formats them today — which is exactly why a
        // note was not enough: `{:?}` inside a future `#[instrument]` span is
        // one keystroke, and the resulting leak is invisible in review.
        //
        // Asserted on the RENDERED string, not on the impls, so it fails for a
        // re-derive, a new field, or a future arm alike.
        let secret_text = "meet me at the safehouse";
        let secret_bytes = b"RLM\x01super-secret-envelope".to_vec();

        for rendered in [
            format!("{:?}", Memo::text(secret_text).expect("text")),
            format!(
                "{:?}",
                Memo::arbitrary(secret_bytes.clone()).expect("bytes")
            ),
            format!("{:?}", Memo::Reserved(secret_bytes.clone())),
        ] {
            assert!(
                !rendered.contains(secret_text) && !rendered.contains("super-secret"),
                "memo CONTENT must never reach a formatter: {rendered}"
            );
            // …and the redaction still says something useful.
            assert!(
                rendered.contains("bytes") || rendered.contains("Empty"),
                "the redaction still discloses presence + length: {rendered}"
            );
        }

        // Address: the encoding never appears, the classification does.
        let addr = shielded();
        let rendered = format!("{addr:?}");
        assert!(
            !rendered.contains(&encoded_sapling(NetworkType::Main)),
            "an address encoding must never reach a formatter: {rendered}"
        );
        assert!(rendered.contains("Sapling"), "the KIND is loggable");

        // Payment: all three never-log classes at once, plus caller free text.
        let payment = Payment::new(
            shielded(),
            Some(Zatoshis::new(123_456_789).expect("amt")),
            Memo::text(secret_text).expect("text"),
            Some("Alice Smith".to_string()),
            Some(secret_text.to_string()),
        )
        .expect("valid leg");
        let rendered = format!("{payment:?}");
        for leak in [secret_text, "Alice Smith", "123456789", "123_456_789"] {
            assert!(
                !rendered.contains(leak),
                "`{leak}` reached a formatter through Payment: {rendered}"
            );
        }

        // …and a whole request inherits the redaction (the widest shape).
        let rendered = format!(
            "{:?}",
            PaymentRequest {
                payments: vec![payment],
            }
        );
        assert!(!rendered.contains(secret_text) && !rendered.contains("Alice Smith"));
    }

    // ── FR-27: the machine-memo READ scope ──────────────────────────────────

    fn prefix(bytes: &[u8]) -> MachineMemoPrefix {
        MachineMemoPrefix::new(bytes.to_vec()).expect("valid test prefix")
    }

    #[test]
    fn machine_memo_prefix_rejects_empty_over_long_and_over_count() {
        // The load-bearing one is EMPTY. An empty prefix matches every memo ever
        // written, which is exactly the failure FR-27's acceptance names: "a
        // build that returns everything fails as surely as one that returns
        // nothing". Unrepresentable, not merely discouraged.
        assert!(matches!(
            MachineMemoPrefix::new(Vec::new()),
            Err(WalletError::MachineMemoScopeInvalid { .. })
        ));
        // Over-long: a prefix longer than the memos it filters can never match,
        // so accepting it would be a silent always-empty read.
        assert!(matches!(
            MachineMemoPrefix::new(vec![0xAA; MACHINE_MEMO_PREFIX_MAX_BYTES + 1]),
            Err(WalletError::MachineMemoScopeInvalid { .. })
        ));
        // …and the boundary itself is ACCEPTED (gate 7: the constant is tested
        // AT its bound, so a future off-by-one shows here).
        assert!(MachineMemoPrefix::new(vec![0xAA; MACHINE_MEMO_PREFIX_MAX_BYTES]).is_ok());
        assert!(MachineMemoPrefix::new(vec![0xAA]).is_ok());

        // The COUNT bound is the scope door's, not the prefix's: every prefix is
        // compared against every memo of the transaction being read.
        let at_max: Vec<_> = (0..MACHINE_MEMO_PREFIX_MAX_COUNT)
            .map(|i| prefix(&[i as u8]))
            .collect();
        assert!(validate_machine_memo_scope(&at_max).is_ok(), "at the bound");
        let over: Vec<_> = (0..=MACHINE_MEMO_PREFIX_MAX_COUNT)
            .map(|i| prefix(&[i as u8]))
            .collect();
        assert!(matches!(
            validate_machine_memo_scope(&over),
            Err(WalletError::MachineMemoScopeInvalid { .. })
        ));
        // An EMPTY scope passes this door — "closed" is a valid configuration.
        // The read verb is what refuses it (typed, never an empty answer).
        assert!(validate_machine_memo_scope(&[]).is_ok());
    }

    #[test]
    fn machine_memo_filter_returns_in_scope_bytes_and_nothing_else() {
        // BOTH polarities in one row, because either alone is passable by a
        // broken build: a filter that returns everything passes "in scope comes
        // back", and one that returns nothing passes "out of scope does not".
        let scope = [prefix(b"RLM\x01")];
        let mine = b"RLM\x01payload-here".to_vec();
        let theirs = b"XYZ\x01someone else's envelope".to_vec();

        let out = filter_machine_memos(
            vec![
                Memo::Arbitrary(mine.clone()),
                Memo::Arbitrary(theirs.clone()),
                // A text memo has its own lane and never leaks through here…
                Memo::text("RLM\u{1}looks like a prefix").expect("text"),
                // …nor does reserved framing, whatever it contains.
                Memo::Reserved(b"RLM\x01reserved framing".to_vec()),
                Memo::Empty,
            ],
            &scope,
        );

        assert_eq!(out.len(), 1, "exactly the in-scope machine memo");
        assert_eq!(out[0], mine, "returned BYTE-FOR-BYTE, not summarised");
        assert!(
            !out.contains(&theirs),
            "an out-of-scope envelope comes back as NOTHING"
        );
    }

    #[test]
    fn machine_memo_filter_never_returns_reserved_bytes_even_in_scope() {
        // Its own row because this is a ruling condition, not an incidental:
        // `Reserved` is receivable on-chain reality, and a wildcard match arm
        // (or a future core arm inheriting one) would hand it over as bytes.
        let scope = [prefix(b"RLM")];
        let out = filter_machine_memos(vec![Memo::Reserved(b"RLMxx".to_vec())], &scope);
        assert!(out.is_empty(), "reserved framing NEVER returns bytes");
    }

    #[test]
    fn machine_memo_filter_drops_an_over_long_memo_rather_than_truncating() {
        // `from_wire` cannot produce this today (the field is 512 bytes), but
        // `Memo` is a pub enum a downstream crate can write literals into, and
        // this is the crossing where a future widening becomes a HOST's problem.
        // Dropping is the honest answer: a truncation would hand back a prefix
        // that parses as a shorter, valid envelope.
        let scope = [prefix(b"RLM")];
        let mut over = b"RLM".to_vec();
        over.resize(MEMO_ARBITRARY_MAX_BYTES + 1, 0xEE);
        assert!(filter_machine_memos(vec![Memo::Arbitrary(over)], &scope).is_empty());

        // At the bound it still comes back (the constant tested AT its edge).
        let mut at_max = b"RLM".to_vec();
        at_max.resize(MEMO_ARBITRARY_MAX_BYTES, 0xEE);
        assert_eq!(
            filter_machine_memos(vec![Memo::Arbitrary(at_max.clone())], &scope),
            vec![at_max]
        );
    }

    #[test]
    fn machine_memo_filter_matches_any_registered_prefix_and_needs_the_lead() {
        let scope = [prefix(b"AA"), prefix(b"BB")];
        let a = b"AA-first".to_vec();
        let b = b"BB-second".to_vec();
        // The prefix must LEAD: containing it elsewhere is not a match. (A
        // `contains`-shaped bug would make the scope meaningless — anyone could
        // bury your magic anywhere in 511 bytes.)
        let buried = b"zzAA-buried".to_vec();
        let out = filter_machine_memos(
            vec![
                Memo::Arbitrary(a.clone()),
                Memo::Arbitrary(buried),
                Memo::Arbitrary(b.clone()),
            ],
            &scope,
        );
        assert_eq!(
            out,
            vec![a, b],
            "either registered prefix matches, lead only"
        );

        // A memo SHORTER than the prefix cannot match (no panic, no partial).
        assert!(filter_machine_memos(vec![Memo::Arbitrary(vec![b'A'])], &scope).is_empty());
    }

    #[test]
    fn machine_memo_filter_with_no_prefixes_returns_nothing() {
        // Defence in depth for the verb's own refusal: even if a caller ever
        // reached the filter with an empty scope, "no prefix" must never mean
        // "every memo".
        let out = filter_machine_memos(vec![Memo::Arbitrary(b"anything".to_vec())], &[]);
        assert!(
            out.is_empty(),
            "an empty scope matches NOTHING, not everything"
        );
    }

    proptest::proptest! {
        /// §4.6 — the filter is TOTAL over hostile bytes. These memos came off a
        /// public chain and an attacker chooses every byte; the read path must
        /// never panic, never return an out-of-scope memo, and never return more
        /// bytes than it was given.
        ///
        /// Generates EVERY `Memo` variant, not just `Arbitrary` (FR-27 crypto
        /// audit): the non-leak half — `Reserved`/`Text`/`Empty` yielding
        /// nothing — was example-tested only, and the property is what stops a
        /// future arm from inheriting a leak. The output is asserted EQUAL to
        /// the in-scope Arbitrary projection, which pins ORDER and MULTIPLICITY
        /// too: a filter that duplicated or reordered its results passed a
        /// membership-only check.
        ///
        /// **The `in_scope` flag is what makes this property non-vacuous, and
        /// it was missing.** With purely random bytes a random 1..40-byte
        /// prefix essentially never leads a memo, so `out` and `expected` were
        /// both EMPTY in almost every case and the property asserted nothing —
        /// PROVED by planting a filter that duplicates every result and
        /// watching the old version pass. Half the generated memos now carry
        /// the prefix by construction, and the `prop_assume`-free non-vacuity
        /// assertion below fails loudly if that ever stops being true.
        #[test]
        fn machine_memo_filter_is_total_over_hostile_bytes(
            raw in proptest::collection::vec(
                (
                    proptest::num::u8::ANY,
                    proptest::bool::ANY,
                    proptest::collection::vec(proptest::num::u8::ANY, 0..600),
                ),
                0..8,
            ),
            prefix_bytes in proptest::collection::vec(proptest::num::u8::ANY, 1..40),
        ) {
            let Ok(p) = MachineMemoPrefix::new(prefix_bytes.clone()) else {
                // over-long prefixes are refused at construction — nothing to drive
                return Ok(());
            };
            let scope = [p];
            // tag % 4 picks the variant (every arm exercised at ~25%);
            // `in_scope` prepends the prefix so the filter has real work to do.
            let memos: Vec<Memo> = raw
                .iter()
                .map(|(tag, in_scope, bytes)| {
                    let mut b = bytes.clone();
                    if *in_scope {
                        let mut lead = prefix_bytes.clone();
                        lead.extend_from_slice(&b);
                        b = lead;
                    }
                    match tag % 4 {
                        0 => Memo::Arbitrary(b),
                        1 => Memo::Reserved(b),
                        2 => Memo::Text(String::from_utf8_lossy(&b).into_owned()),
                        _ => Memo::Empty,
                    }
                })
                .collect();
            // What the filter is ALLOWED to return, computed independently here.
            let expected: Vec<Vec<u8>> = memos
                .iter()
                .filter_map(|m| match m {
                    Memo::Arbitrary(b) => Some(b.clone()),
                    _ => None,
                })
                .filter(|b| b.len() <= MEMO_ARBITRARY_MAX_BYTES)
                .filter(|b| b.starts_with(&prefix_bytes))
                .collect();
            // ANTI-VACUITY, asserted per case rather than hoped for: whenever
            // the generator built an in-scope Arbitrary memo within bounds, the
            // filter must return something. Without this a future generator
            // change can quietly make every case empty again and the property
            // goes back to asserting nothing while staying green.
            let planted = raw.iter().any(|(tag, in_scope, bytes)| {
                tag % 4 == 0
                    && *in_scope
                    && prefix_bytes.len() + bytes.len() <= MEMO_ARBITRARY_MAX_BYTES
            });
            let out = filter_machine_memos(memos, &scope);
            proptest::prop_assert_eq!(
                &out,
                &expected,
                "exactly the in-scope Arbitrary memos, in order, once each"
            );
            proptest::prop_assert!(
                !planted || !out.is_empty(),
                "a planted in-scope machine memo must come back — this property \
                 is worthless if every case is empty"
            );
            for bytes in &out {
                proptest::prop_assert!(
                    bytes.starts_with(&prefix_bytes),
                    "only prefix-matched bytes come back"
                );
                proptest::prop_assert!(bytes.len() <= MEMO_ARBITRARY_MAX_BYTES);
            }
        }
    }

    #[test]
    fn address_parse_classifies_and_network_checks() {
        // a checksum-valid mainnet UA parses on Main with honest memo state
        let ua = encoded_unified(&NetworkType::Main);
        let parsed = Address::parse(&ua, Network::Main).expect("mainnet UA parses");
        assert_eq!(parsed.kind(), AddressKind::Unified);
        assert!(parsed.memo_capable(), "shielded UA takes memos");
        assert!(ua.starts_with("u1"), "mainnet UA HRP");

        // …and is a typed NETWORK mismatch on Test, not 'malformed'
        // (§8 network_mismatch_rejected_everywhere — the address arm)
        assert!(matches!(
            Address::parse(&ua, Network::Test),
            Err(WalletError::NetworkMismatch)
        ));
        // same the other way (testnet sapling under a Main config)
        assert!(matches!(
            Address::parse(&encoded_sapling(NetworkType::Test), Network::Main),
            Err(WalletError::NetworkMismatch)
        ));

        // sapling + transparent classify with their memo-capability
        let zs = Address::parse(&encoded_sapling(NetworkType::Main), Network::Main)
            .expect("sapling parses");
        assert_eq!(zs.kind(), AddressKind::Sapling);
        assert!(zs.memo_capable());
        let t =
            Address::parse(&encoded_p2pkh(NetworkType::Main), Network::Main).expect("p2pkh parses");
        assert_eq!(t.kind(), AddressKind::Transparent);
        assert!(!t.memo_capable());

        // garbage, corrupted checksum, and oversize are AddressInvalid —
        // never echoed back
        assert!(matches!(
            Address::parse("u1notanaddress", Network::Main),
            Err(WalletError::AddressInvalid)
        ));
        let mut corrupted = encoded_unified(&NetworkType::Main);
        corrupted.pop();
        assert!(matches!(
            Address::parse(&corrupted, Network::Main),
            Err(WalletError::AddressInvalid)
        ));
        assert!(matches!(
            Address::parse(&"u1".repeat(300), Network::Main),
            Err(WalletError::AddressInvalid)
        ));
    }

    /// KAT (vectors-first, §3.2i-2 TEX/ZIP-320): the official TEX vectors CLASSIFY as
    /// transparent + memo-incapable, parse on their own net, and cross-net-reject TYPED.
    /// Pinned BEFORE the multi-step send wiring so the send path can RELY on this
    /// classification (a TEX recipient is transparent-source-restricted — it drives the
    /// de-shield disclosure and the no-memo rule). The vectors are the de-facto
    /// authoritative pairs whose `t1…`↔`tex…` BYTE round-trip is the audited library's own
    /// KAT (`zcash_address-0.12.0/src/encoding.rs`, the `tex` / `tex_testnet` tests:
    /// mainnet TEX re-encodes `t1VmmGiyjVNeCjxDZzg7vZmd99WyzVby9yC`, testnet
    /// `tm9ofD7kHR7AF8MsJomEzLqGcrLCBkD9gDj`). We deliberately do NOT re-assert that
    /// round-trip here — `Address` hides the 20-byte hash by design (§4.6) — only the
    /// send-path-relevant classification it produces.
    #[test]
    fn kat_tex_official_vectors_classify_transparent() {
        const TEX_MAIN: &str = "tex1s2rt77ggv6q989lr49rkgzmh5slsksa9khdgte";
        const TEX_TEST: &str = "textest1qyqszqgpqyqszqgpqyqszqgpqyqszqgpfcjgfy";

        // Transparent, memo-INCAPABLE, and transparent-only (the last drives the §2.4
        // zero-valued-output rule) — exactly what the send path will check.
        let tex_main = Address::parse(TEX_MAIN, Network::Main).expect("mainnet TEX parses");
        assert_eq!(tex_main.kind(), AddressKind::Transparent);
        assert!(
            !tex_main.memo_capable(),
            "TEX (ZIP-320) cannot carry a memo"
        );
        assert!(
            tex_main.transparent_only(),
            "TEX is a pure transparent address"
        );

        let tex_test = Address::parse(TEX_TEST, Network::Test).expect("testnet TEX parses");
        assert_eq!(tex_test.kind(), AddressKind::Transparent);
        assert!(!tex_test.memo_capable());
        assert!(tex_test.transparent_only());

        // Cross-network is a TYPED NetworkMismatch, never 'malformed' (the §8 arm) — a
        // mainnet TEX pasted under a testnet config is an honest "wrong network", not junk.
        assert!(matches!(
            Address::parse(TEX_MAIN, Network::Test),
            Err(WalletError::NetworkMismatch)
        ));
        assert!(matches!(
            Address::parse(TEX_TEST, Network::Main),
            Err(WalletError::NetworkMismatch)
        ));

        // A corrupted TEX (mangled bech32m payload) is `AddressInvalid` — never echoed
        // back, never a partial classification. A TEX is P2PKH-only (20-byte hash); there
        // is no P2SH-TEX encoding, so a wrong-length/garbled `tex1…` simply fails to decode.
        let mut corrupted = TEX_MAIN.to_string();
        corrupted.pop();
        assert!(matches!(
            Address::parse(&corrupted, Network::Main),
            Err(WalletError::AddressInvalid)
        ));

        // A memo to a TEX recipient is a TYPED construction reject — the send path can
        // never silently carry (nor silently drop) a memo to an exchange address.
        assert!(matches!(
            Payment::new(
                tex_main,
                Some(Zatoshis::new(100_000).expect("amt")),
                Memo::text("hi").expect("short memo"),
                None,
                None,
            ),
            Err(WalletError::MemoRequiresShieldedRecipient)
        ));
    }

    #[test]
    fn has_amount_less_leg_covers_all_cases() {
        // The SSOT predicate behind the SendAmountRequired gate that BOTH `propose`
        // and `queue_send` enforce — pinned directly so the `any()` semantics (incl. the
        // vacuous empty case and a mixed request) can't silently regress.
        let with_amount = Payment::new(
            shielded(),
            Some(Zatoshis::new(1).expect("amt")),
            Memo::Empty,
            None,
            None,
        )
        .expect("leg with amount");
        let without_amount =
            Payment::new(shielded(), None, Memo::Empty, None, None).expect("donation form");
        assert!(
            !PaymentRequest { payments: vec![] }.has_amount_less_leg(),
            "empty request: no legs ⇒ vacuously false (never spuriously SendAmountRequired)",
        );
        assert!(
            !PaymentRequest {
                payments: vec![with_amount.clone()]
            }
            .has_amount_less_leg(),
            "every leg has an amount",
        );
        assert!(
            PaymentRequest {
                payments: vec![without_amount.clone()]
            }
            .has_amount_less_leg(),
            "a single amount-less leg",
        );
        assert!(
            PaymentRequest {
                payments: vec![with_amount, without_amount]
            }
            .has_amount_less_leg(),
            "mixed: one amount-less leg is enough",
        );
    }
}
