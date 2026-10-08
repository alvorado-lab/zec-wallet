//! Money, network, and chain identifiers (spec §2.1).

use crate::constants::MAX_MONEY_ZAT;
use crate::error::WalletError;

/// Zatoshis — non-negative, ≤ max supply. `i64` so the value survives the
/// FRB i64 → Dart `int` mapping exactly (max supply ≈ 2.1e15 < 2⁵³; §2.1).
///
/// Unsigned-by-contract: the constructor rejects negatives. The signed
/// "effect on this wallet" quantity is [`ZatBalance`] — two types mirroring
/// upstream's own `Zatoshis`/`ZatBalance` split, so a negative can never
/// reach a field that means "an amount".
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct Zatoshis(i64);

impl Zatoshis {
    pub const ZERO: Zatoshis = Zatoshis(0);

    /// Rejects negatives and beyond-supply values — typed, never a panic.
    pub fn new(zat: i64) -> Result<Self, WalletError> {
        if !(0..=MAX_MONEY_ZAT).contains(&zat) {
            return Err(WalletError::AmountOutOfRange);
        }
        Ok(Self(zat))
    }

    pub fn zat(self) -> i64 {
        self.0
    }

    /// `None` on overflow past max supply — callers decide; no silent clamp.
    pub fn checked_add(self, rhs: Zatoshis) -> Option<Zatoshis> {
        let sum = self.0.checked_add(rhs.0)?;
        (sum <= MAX_MONEY_ZAT).then_some(Zatoshis(sum))
    }

    /// `None` when the result would be negative.
    pub fn checked_sub(self, rhs: Zatoshis) -> Option<Zatoshis> {
        let diff = self.0 - rhs.0; // both bounded by MAX_MONEY_ZAT, can't wrap
        (diff >= 0).then_some(Zatoshis(diff))
    }
}

/// Signed zatoshi quantity — the net effect of a tx on this wallet
/// (`TxSummary::net_amount`, §2.5). Bounded to ±max supply.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct ZatBalance(i64);

impl ZatBalance {
    pub fn new(zat: i64) -> Result<Self, WalletError> {
        if !(-MAX_MONEY_ZAT..=MAX_MONEY_ZAT).contains(&zat) {
            return Err(WalletError::AmountOutOfRange);
        }
        Ok(Self(zat))
    }

    pub fn zat(self) -> i64 {
        self.0
    }
}

/// Consensus network. A RUNTIME parameter (consensus params follow it), not a
/// cargo feature — one binary serves both networks (§2.1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum Network {
    Main,
    Test,
}

impl Network {
    /// Map to the runtime-switchable upstream consensus parameters
    /// (`zcash_protocol::consensus::Network`) — the single value type that
    /// carries our network into every librustzcash call that wants
    /// `consensus::Parameters` (the wallet DB `WalletDb::from_connection`
    /// migration in `db.rs`; the sync engine at inc-2c). This is the one
    /// source of truth for the network mapping; `derivation.rs` uses the
    /// `MAIN/TEST_NETWORK` singletons for its monomorphized encode path, which
    /// is a distinct (compile-time) API — both agree by construction here.
    pub(crate) fn consensus(self) -> zcash_protocol::consensus::Network {
        match self {
            Network::Main => zcash_protocol::consensus::Network::MainNetwork,
            Network::Test => zcash_protocol::consensus::Network::TestNetwork,
        }
    }

    /// The address-encoding [`NetworkType`](zcash_protocol::consensus::NetworkType) for this
    /// network — the discriminant the audited `zcash_address` encoder wants when re-encoding a
    /// raw `TransparentAddress` to its string form (the #315 reclaim self-mint). Agrees with
    /// [`consensus`](Self::consensus) by construction (both derive from the same `Network` value);
    /// kept separate because `zcash_address` takes the `NetworkType`, not the consensus params.
    pub(crate) fn network_type(self) -> zcash_protocol::consensus::NetworkType {
        match self {
            Network::Main => zcash_protocol::consensus::NetworkType::Main,
            Network::Test => zcash_protocol::consensus::NetworkType::Test,
        }
    }
}

/// Transaction id (32 bytes, ALWAYS in displayed byte order).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TxId([u8; 32]);

impl TxId {
    /// PRECONDITION (named in the constructor because nothing can enforce
    /// it): `bytes` must already be in DISPLAYED (block-explorer) order.
    /// Zcash convention displays txids byte-REVERSED relative to the
    /// internal hash order, and librustzcash's own `TxId` stores INTERNAL
    /// order — so the WalletDb increment MUST wire upstream txids through a
    /// reversing `From<zcash_primitives …TxId>` impl here (+ a KAT against
    /// a known explorer txid), never through this constructor with
    /// upstream's raw bytes (W3 crypto audit fold; a mistake here is a
    /// silent wallet-wide explorer-mismatch on every txid).
    pub fn from_display_order(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// THE hex-parse door (FR-27) — the exact inverse of [`Display`](std::fmt::Display),
    /// so a host round-trips the `txid_hex` a history row handed it and nothing
    /// else. Accepts 64 hex characters in EITHER case (an explorer copy-paste is
    /// commonly uppercase) and rejects everything else typed
    /// ([`WalletError::TxidInvalid`]) — hostile input by §4.6: the length is
    /// checked BEFORE any allocation, and the offending string is never echoed
    /// back (§5.4).
    ///
    /// NO byte reversal here, deliberately. The input is already in DISPLAY
    /// order because it came from our own `Display`; reversing would recreate
    /// the exact wallet-wide explorer mismatch
    /// [`from_display_order`](Self::from_display_order) exists to prevent. Pinned
    /// by `txid_from_display_hex_round_trips_the_audited_display`, which ties the
    /// round trip to librustzcash's own `Display` rather than to ours alone.
    pub fn from_display_hex(hex: &str) -> Result<Self, WalletError> {
        // ASCII-hex FIRST, before any parse: `u8::from_str_radix` accepts a
        // leading sign, so a bare `from_str_radix` would read "+f" as the byte
        // 0x0F and admit a txid that is not hex at all.
        if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(WalletError::TxidInvalid);
        }
        let mut bytes = [0u8; 32];
        for (i, out) in bytes.iter_mut().enumerate() {
            let pair = hex.get(i * 2..i * 2 + 2).ok_or(WalletError::TxidInvalid)?;
            *out = u8::from_str_radix(pair, 16).map_err(|_| WalletError::TxidInvalid)?;
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// THE SDK→upstream txid conversion — the exact inverse of the
    /// [`From<zcash_protocol::TxId>`] door below, for the reads that key on the
    /// engine's INTERNAL-order bytes (the `transactions.txid` column, `WalletRead`).
    /// One reversing site in each direction; a caller that reverses by hand is the
    /// silent explorer-mismatch the door exists to prevent.
    pub fn to_internal(&self) -> zcash_protocol::TxId {
        let mut bytes = self.0; // DISPLAY (block-explorer) order
        bytes.reverse(); // → INTERNAL (consensus) order
        zcash_protocol::TxId::from_bytes(bytes)
    }
}

/// THE upstream→SDK txid conversion (the W3 crypto audit obligation named on
/// [`TxId::from_display_order`]) — the SINGLE door every librustzcash txid crosses into
/// the SDK. librustzcash's `TxId` stores INTERNAL (consensus) byte order and REVERSES in
/// its `Display`; our [`TxId`] stores DISPLAY (block-explorer) order directly. So this
/// REVERSES — never the raw [`from_display_order`](TxId::from_display_order) constructor
/// with upstream's bytes (a missing reversal is a silent wallet-wide explorer-mismatch on
/// every txid). KAT-pinned by `txid_from_upstream_reverses_internal_to_display_order`,
/// which ties OUR `Display` to the audited upstream `Display`. First crossing: the
/// inc-2d-2 create+sign path (`Wallet::sign_proposal`).
impl From<zcash_protocol::TxId> for TxId {
    fn from(upstream: zcash_protocol::TxId) -> Self {
        let mut bytes = *upstream.as_ref(); // INTERNAL (consensus) order
        bytes.reverse(); // → DISPLAY (block-explorer) order
        TxId::from_display_order(bytes)
    }
}

/// THE txid rendering (block-explorer form). Single source of truth — the
/// FRB bridge and any host display go through here; bytes are already stored
/// in displayed order, so this is a plain lowercase-hex encode.
impl std::fmt::Display for TxId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for b in &self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

/// Chain height.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct BlockHeight(u32);

impl BlockHeight {
    pub fn new(height: u32) -> Self {
        Self(height)
    }

    pub fn value(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zatoshis_constructor_bounds() {
        assert!(Zatoshis::new(0).is_ok());
        assert!(Zatoshis::new(MAX_MONEY_ZAT).is_ok());
        assert!(Zatoshis::new(-1).is_err());
        assert!(Zatoshis::new(MAX_MONEY_ZAT + 1).is_err());
    }

    #[test]
    fn zatoshis_checked_math_refuses_overflow_and_negative() {
        let max = Zatoshis::new(MAX_MONEY_ZAT).expect("max is valid");
        let one = Zatoshis::new(1).expect("one is valid");
        assert_eq!(max.checked_add(one), None);
        assert_eq!(Zatoshis::ZERO.checked_sub(one), None);
        assert_eq!(one.checked_sub(one), Some(Zatoshis::ZERO));
    }

    #[test]
    fn txid_displays_as_lowercase_hex_of_stored_order() {
        let mut bytes = [0u8; 32];
        bytes[0] = 0xab;
        bytes[31] = 0x01;
        let txid = TxId::from_display_order(bytes);
        let s = txid.to_string();
        assert_eq!(s.len(), 64);
        assert!(s.starts_with("ab"));
        assert!(s.ends_with("01"));
    }

    #[test]
    fn txid_from_upstream_reverses_internal_to_display_order() {
        // §8 (gate 1 / the W3 crypto audit obligation named on `from_display_order`):
        // librustzcash `TxId` stores INTERNAL order + reverses in Display; our `TxId` stores
        // DISPLAY order. The `From` conversion MUST reverse, so OUR Display equals the audited
        // upstream block-explorer Display — the anti-silent-explorer-mismatch invariant.
        // Distinct ascending bytes make the reversal visible (not a palindrome).
        let internal: [u8; 32] = std::array::from_fn(|i| i as u8); // 00 01 .. 1f (internal)
        let upstream = zcash_protocol::TxId::from_bytes(internal);
        let ours: TxId = upstream.into();

        // the SDK stores the REVERSED (display) bytes
        let mut expected = internal;
        expected.reverse();
        assert_eq!(
            ours.as_bytes(),
            &expected,
            "internal (consensus) order is reversed to display order"
        );
        // the load-bearing equality: OUR explorer hex == librustzcash's canonical Display
        assert_eq!(
            ours.to_string(),
            upstream.to_string(),
            "SDK txid Display == upstream block-explorer Display"
        );
        // and it is genuinely a reversal, not a raw copy (the silent-mismatch the door guards)
        assert_ne!(
            ours.as_bytes(),
            &internal,
            "the conversion reverses; a raw copy would mismatch every explorer"
        );
    }

    #[test]
    fn txid_from_display_hex_round_trips_the_audited_display() {
        // FR-27 gate-1: the new inbound hex door is the exact inverse of the
        // OUTBOUND rendering, and the anchor is librustzcash's own `Display` —
        // not ours alone, which would be a tautology. A reversal (or a missing
        // one) on this door is the same silent wallet-wide explorer mismatch
        // `from_display_order`'s doc names, arriving from the other side.
        let internal: [u8; 32] = std::array::from_fn(|i| i as u8); // not a palindrome
        let upstream = zcash_protocol::TxId::from_bytes(internal);
        let ours: TxId = upstream.into();
        let parsed = TxId::from_display_hex(&upstream.to_string()).expect("audited hex parses");
        assert_eq!(
            parsed, ours,
            "parsing the explorer form yields the same txid"
        );
        assert_eq!(parsed.to_string(), upstream.to_string());

        // Uppercase is the same txid (an explorer copy-paste is commonly upper).
        let upper = upstream.to_string().to_uppercase();
        assert_eq!(
            TxId::from_display_hex(&upper).expect("case-insensitive"),
            ours
        );
    }

    #[test]
    fn txid_from_display_hex_refuses_hostile_input_typed_never_panics() {
        // §4.6: the string comes from a host and, through it, from anywhere.
        // Length is checked BEFORE any allocation, and nothing is echoed back
        // (§5.4 — the error is payload-free).
        //
        // TWO TABLES, and the split is the point. `from_display_hex` has two
        // guards — a LENGTH clause and an ASCII-hex clause — and a wrong-length
        // input can only ever exercise the first. The rows meant to prove the
        // SECOND therefore assert their own length first: the original
        // sign-escape row was 63 characters, so it was rejected on length and
        // never reached the guard it was documented as proving. It would have
        // passed with the hex filter deleted (FR-27 code review; same vacuity
        // shape as the `memo.rs` proptest, one file over).

        // (a) WRONG LENGTH — these exercise the length clause and nothing else.
        for bad in ["", "ab", &"a".repeat(63), &"a".repeat(65), &"a".repeat(128)] {
            assert!(
                matches!(TxId::from_display_hex(bad), Err(WalletError::TxidInvalid)),
                "a wrong-length txid is typed, never a panic: {bad:?}"
            );
        }

        // (b) RIGHT LENGTH, NOT HEX — these reach the ASCII-hex clause, which is
        // the only thing standing between this door and the sign-escape bug.
        for bad in [
            // THE load-bearing row. `u8::from_str_radix` accepts a LEADING SIGN,
            // so without the pre-filter "+f" parses as 0x0F and a string that is
            // not hex at all is admitted as a txid. 64 chars exactly: "+f" + 62.
            format!("+f{}", "0".repeat(62)),
            // and the negative twin
            format!("-f{}", "0".repeat(62)),
            "g".repeat(64),                  // plain non-hex digit
            format!("0x{}", "0".repeat(62)), // 0x-prefixed, right length
            format!("{} ", "a".repeat(63)),  // trailing space (from_str_radix does not trim)
            "é".repeat(32),                  // 64 BYTES of multi-byte chars, not 64 chars
        ] {
            assert_eq!(
                bad.len(),
                64,
                "this row must be 64 BYTES or it never reaches the hex guard: {bad:?}"
            );
            assert!(
                matches!(TxId::from_display_hex(&bad), Err(WalletError::TxidInvalid)),
                "a right-length non-hex txid is typed, never a panic: {bad:?}"
            );
        }
    }

    #[test]
    fn zat_balance_is_signed_but_bounded() {
        assert!(ZatBalance::new(-MAX_MONEY_ZAT).is_ok());
        assert!(ZatBalance::new(MAX_MONEY_ZAT).is_ok());
        assert!(ZatBalance::new(MAX_MONEY_ZAT + 1).is_err());
        assert!(ZatBalance::new(-(MAX_MONEY_ZAT + 1)).is_err());
    }
}
