/// Parse a user-typed ZEC amount string into integer **zatoshis** — INTEGER
/// MATH ONLY, the strict inverse of `formatZec` (zat_format.dart). A send amount
/// is money, so it NEVER routes through `double`: IEEE-754 cannot represent most
/// decimal ZEC values exactly, and a float path would silently send the wrong
/// amount. This is the host-side first gate before a `PaymentDraft` is composed;
/// the SDK re-validates every figure, but parsing here gives an honest inline
/// message before any bridge call.
library;

// The send-amount ceiling is the SDK's total-supply bound (FR-48: one home, in
// `zec_wallet`). The SDK rejects anything above it as `amountOutOfRange`, but
// catching it here keeps the form honest before the round-trip. Pinned by
// `zec_amount_test` at its boundary (gate 7).
import 'package:zec_wallet/zec_wallet.dart' show maxMoneyZat;

import '../zat_format.dart' show zatPerZec;

/// Digit-length guard on the whole part — rejects an arbitrarily long paste
/// BEFORE `int.parse`, so the PARSE itself can't overflow (≤18 digits is ≤~1e18,
/// inside int64's ~9.2e18). This is NOT the money ceiling (leading zeros are
/// legal input, so a digit count can't bound the value): the real bound is the
/// numeric `whole > maxMoneyZat ~/ zatPerZec` check applied BEFORE the multiply
/// (so `whole * zatPerZec` can never overflow), plus the exact `> maxMoneyZat`.
const int _maxWholeDigits = 18;

/// Why an amount string was rejected — the axis the form's inline message keys
/// on (never a raw code; design invariant 6). All payload-free: an amount is
/// never-log material (§5.4), so the reason carries no value.
enum ZecAmountFault {
  /// The field is empty.
  empty,

  /// Not a plain decimal number (a sign, separators, letters, a second dot, …).
  notANumber,

  /// More than 8 fractional digits — ZEC's smallest unit is 1 zatoshi (1e-8).
  tooManyDecimals,

  /// Parses to zero — a zero-value send is not a payment.
  notPositive,

  /// Above the total ZEC supply (or its whole part overflows the guard).
  outOfRange,
}

/// The result of [parseZecAmount] — a validated zatoshi value or a typed fault.
sealed class ZecAmountResult {
  const ZecAmountResult();
}

/// A valid, positive, in-range amount in zatoshis.
class ZecAmountValid extends ZecAmountResult {
  const ZecAmountValid(this.zat);

  /// Amount in zatoshis (exact; `1 <= zat <= maxMoneyZat`).
  final int zat;
}

/// An invalid amount — render [fault] honestly inline.
class ZecAmountInvalid extends ZecAmountResult {
  const ZecAmountInvalid(this.fault);

  final ZecAmountFault fault;
}

/// Parse [input] (a user-typed ZEC amount) into zatoshis with integer math only.
///
/// Accepts a plain non-negative decimal: optional whole part, an optional single
/// `.`, an optional fractional part of at most 8 digits (e.g. `1`, `0.0001`,
/// `.5`, `21000000`). Rejects signs, thousands separators, exponents, a second
/// dot, and anything non-digit. Surrounding whitespace is trimmed.
///
/// Round-trips with `formatZec`: `parseZecAmount(formatZec(x))` is
/// `ZecAmountValid(x)` for every `1 <= x <= maxMoneyZat`.
ZecAmountResult parseZecAmount(String input) {
  final s = input.trim();
  if (s.isEmpty) return const ZecAmountInvalid(ZecAmountFault.empty);

  // Digits and at most one dot, and at least one digit somewhere (a bare "."
  // matches the shape but is not a number). No sign, no separators, no exponent.
  if (!RegExp(r'^[0-9]*\.?[0-9]*$').hasMatch(s) ||
      !RegExp(r'[0-9]').hasMatch(s)) {
    return const ZecAmountInvalid(ZecAmountFault.notANumber);
  }

  final dot = s.indexOf('.');
  final wholePart = dot < 0 ? s : s.substring(0, dot);
  final fracPart = dot < 0 ? '' : s.substring(dot + 1);

  if (fracPart.length > 8) {
    return const ZecAmountInvalid(ZecAmountFault.tooManyDecimals);
  }
  // Length guard BEFORE int.parse so an arbitrarily long paste can't overflow the
  // parse itself (≤18 digits ⇒ ≤~1e18, inside int64).
  if (wholePart.length > _maxWholeDigits) {
    return const ZecAmountInvalid(ZecAmountFault.outOfRange);
  }

  // wholePart may be empty (".5"); fracPart is right-padded to 8 digits so its
  // place value is fixed (".5" ⇒ 50000000 zat, not 5).
  final whole = wholePart.isEmpty ? 0 : int.parse(wholePart);
  // The SUPPLY ceiling on the whole part, applied BEFORE the multiply — so
  // `whole * zatPerZec` can NEVER overflow int64 (max supply is 21_000_000 ZEC,
  // so any larger whole part is already out of range). This is the real guard a
  // digit count can't be: it also tolerates leading zeros ("00021000000" ⇒ 21e6).
  if (whole > maxMoneyZat ~/ zatPerZec) {
    return const ZecAmountInvalid(ZecAmountFault.outOfRange);
  }
  final frac = fracPart.isEmpty ? 0 : int.parse(fracPart.padRight(8, '0'));
  final zat = whole * zatPerZec + frac;

  if (zat <= 0) return const ZecAmountInvalid(ZecAmountFault.notPositive);
  if (zat > maxMoneyZat) {
    return const ZecAmountInvalid(ZecAmountFault.outOfRange);
  }
  return ZecAmountValid(zat);
}
