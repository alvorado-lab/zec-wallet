/// Money formatting for ZEC amounts — INTEGER MATH ONLY.
///
/// A zatoshi is 1e-8 ZEC. Balances cross the bridge as `i64` zatoshis
/// (Dart `int`, 64-bit on every supported host — web is out of scope for
/// this SDK, README § Building). Formatting NEVER routes through `double`:
/// IEEE-754 cannot represent most decimal ZEC values exactly, so a float
/// path silently mis-renders a balance — unacceptable for money. Max ZEC
/// supply (~21e6 ZEC ⇒ ~2.1e15 zat) is well inside `int64`, so plain
/// integer division is exact.
library;

/// Zatoshis per ZEC (1e8). The one place this constant lives.
const int zatPerZec = 100000000;

/// Format a zatoshi amount as a ZEC decimal string WITHOUT a unit suffix
/// (the caller appends a localized unit). Trailing fractional zeros are
/// trimmed; a whole amount renders with no decimal point. Negative inputs
/// (signed net amounts in history) keep their sign.
///
/// Examples: `0 → "0"`, `1 → "0.00000001"`, `100000000 → "1"`,
/// `123450000 → "1.2345"`, `-1500000 → "-0.015"`.
String formatZec(int zat) {
  final negative = zat < 0;
  // `abs()` is safe: real zatoshi amounts are bounded by max supply, far
  // from int64's edge where `abs()` would overflow.
  final magnitude = zat.abs();
  final whole = magnitude ~/ zatPerZec;
  final frac = magnitude % zatPerZec;

  final String body;
  if (frac == 0) {
    body = '$whole';
  } else {
    // 8 fixed fractional digits, then trim trailing zeros (never the
    // leading ones — `00000001` must stay `0.00000001`, not `0.1`).
    final fracDigits = frac
        .toString()
        .padLeft(8, '0')
        .replaceAll(RegExp(r'0+$'), '');
    body = '$whole.$fracDigits';
  }
  return negative ? '-$body' : body;
}
