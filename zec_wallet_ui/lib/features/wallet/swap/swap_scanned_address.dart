/// Pure, chain-agnostic normalization of a RAW scanned QR payload into a bare
/// foreign-chain address (wallet spec §3.3b D6/L8). Shared by the IntoZec
/// refund-address scan and the OutOfZec destination-address scan — both unwrap
/// the same standard wallet-QR envelopes. No Flutter, no `dart:io`, no camera
/// imports — unit-testable in isolation.
///
/// Why this exists: real-world crypto QR codes are very often URIs, not bare
/// addresses — BIP-21 (`bitcoin:bc1q…?amount=0.5&label=…`), EIP-681
/// (`ethereum:0x…?value=…`), and the same pattern across litecoin/dogecoin/etc.
/// A raw passthrough would hand the SDK a string it rejects, so the scan would
/// feel broken on a perfectly standard wallet QR.
///
/// What it deliberately does NOT do: it has ZERO chain knowledge and performs
/// ZERO validation. It unwraps only the *syntactic* URI envelope — a leading
/// RFC-3986 `scheme:` and any trailing `?query` / `#fragment`. The address
/// stays opaque: the SDK remains the sole validation gate (validated
/// Rust-side), and the user still verifies it character-by-character at the
/// review step (§3.3b D6) before any deposit. A bare address (no scheme) is
/// returned trimmed and otherwise unchanged.
///
/// Known residual (accepted): an EIP-681 chain-id suffix (`…@1`) is left
/// intact — stripping `@` is chain-specific and risks corrupting a legitimate
/// address, so we leave it for the user to catch at verification. This matches
/// the "validate at the boundary, never guess" posture.
///
/// Untrusted-input hardening (security review fold): the QR is attacker-
/// controllable (a planted code), so we (1) reject an absurd payload up front
/// (a QR holds ~3 KB max), (2) treat the address as a single contiguous token —
/// truncating at the first interior whitespace or control byte, which strips
/// CR/LF/TAB and prevents a smuggled "second line" from reaching the field or
/// the char-by-char verification render — and (3) cap the result to a generous
/// cross-chain address length, returning empty (→ the form's non-empty guard
/// rejects it honestly) on overflow.
library;

/// RFC-3986 §3.1 scheme: ALPHA *( ALPHA / DIGIT / "+" / "-" / "." ) then ":".
final RegExp _uriScheme = RegExp(r'^[a-zA-Z][a-zA-Z0-9+.-]*:');

/// First `?` (query) or `#` (fragment) delimiter — BIP-21 / EIP-681 params.
final RegExp _queryOrFragment = RegExp(r'[?#]');

/// A QR code holds ~3 KB at most; anything larger is not a scanned address.
const int kMaxScannedPayloadChars = 4096;

/// Generous upper bound across every supported chain (Monero integrated ~106;
/// EIP-681 + chain-id ~50; bech32/cashaddr well under). Over this => reject.
const int kMaxScannedAddressChars = 256;

/// Index of the first interior whitespace or control byte, or -1 if none. An
/// address is a single contiguous token, so the first such char ends it — this
/// is what prevents a smuggled "second line" from reaching the field/render.
int _firstWhitespaceOrControl(String s) {
  for (var i = 0; i < s.length; i++) {
    final c = s.codeUnitAt(i);
    // C0 (0x00–0x1F), DEL + C1 (0x7F–0x9F) control bytes.
    if (c < 0x20 || (c >= 0x7F && c <= 0x9F)) return i;
    // ANY Unicode whitespace — delegated to `String.trim()`'s set (space,
    // NBSP, ideographic/figure spaces U+2000–U+200A/U+202F/U+205F/U+3000, the
    // line/paragraph separators, BOM) so the single-token invariant holds for
    // the full Unicode-space set, not just the few code points we'd enumerate.
    if (s[i].trim().isEmpty) return i;
  }
  return -1;
}

/// Unwrap a scanned QR payload to a bare foreign-chain address. See the library
/// doc.
String normalizeScannedAddress(String raw) {
  // Size-cap before doing any work (every byte from an untrusted source is
  // hostile, principle 7).
  if (raw.length > kMaxScannedPayloadChars) return '';

  var s = raw.trim();

  // Strip a leading `scheme:` (e.g. `bitcoin:`, `ethereum:`, `litecoin:`),
  // case-insensitively per the scheme grammar. A bare address has no scheme
  // and is left as-is.
  final scheme = _uriScheme.firstMatch(s);
  if (scheme != null) {
    s = s.substring(scheme.end);
    // Some encoders emit `scheme://address`; drop a leftover authority slash.
    if (s.startsWith('//')) {
      s = s.substring(2);
    }
  }

  // Drop a `?query` / `#fragment` (amount, label, message, EIP-681 value…).
  final delimiter = _queryOrFragment.firstMatch(s);
  if (delimiter != null) {
    s = s.substring(0, delimiter.start);
  }

  // Trim whitespace the unwrap may have exposed, then keep only the leading
  // token (the address can't contain interior whitespace/control bytes).
  s = s.trim();
  final stop = _firstWhitespaceOrControl(s);
  if (stop >= 0) {
    s = s.substring(0, stop);
  }

  if (s.length > kMaxScannedAddressChars) return '';
  return s;
}
