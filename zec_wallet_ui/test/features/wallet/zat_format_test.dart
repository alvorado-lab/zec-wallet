import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/zat_format.dart';

/// Money formatting is pure integer math — these known-answer cases pin the
/// zatoshi→ZEC contract at its boundaries. A float regression would slip
/// past loose checks, so the vectors include values a `double` mis-renders.
void main() {
  group('formatZec', () {
    test('zero renders as a bare 0 (no decimal point)', () {
      expect(formatZec(0), '0');
    });

    test('one zatoshi is the smallest unit, leading zeros preserved', () {
      // The classic float trap: 0.00000001 is not representable in IEEE-754.
      expect(formatZec(1), '0.00000001');
    });

    test('exactly one ZEC has no fractional part', () {
      expect(formatZec(zatPerZec), '1');
    });

    test('trailing fractional zeros are trimmed, leading ones are not', () {
      expect(formatZec(123450000), '1.2345');
      expect(formatZec(10000000), '0.1');
      expect(formatZec(100000010), '1.0000001');
    });

    test('a whole-plus-fraction amount keeps every significant digit', () {
      // 1.23456789 ZEC — a value double cannot hold exactly.
      expect(formatZec(123456789), '1.23456789');
    });

    test('negative amounts (signed net effects) keep the sign', () {
      expect(formatZec(-1500000), '-0.015');
      expect(formatZec(-1), '-0.00000001');
    });

    test('large amounts near max supply stay exact (no float drift)', () {
      // 20,999,999.99999999 ZEC in zatoshis — inside int64, exact.
      expect(formatZec(2099999999999999), '20999999.99999999');
    });

    test('the constant is the canonical 1e8', () {
      expect(zatPerZec, 100000000);
    });

    test('round-trips: re-parsing the decimal recovers the exact zatoshi', () {
      // Property over a deterministic spread (testing-patterns: no RNG). If
      // formatting ever introduced float drift or dropped/added a digit, the
      // re-parse would diverge from the original integer.
      final values = <int>{
        0,
        1,
        7,
        9,
        10,
        99,
        100,
        101,
        999,
        1000,
        12345678,
        99999999,
        100000000,
        100000001,
        199999999,
        2099999999999999,
      };
      for (var i = 0; i < 250; i++) {
        // A spread of magnitudes and low-order patterns, fully deterministic.
        values.add(i * 1234567 + i * i * 7 + (i % 9));
        values.add(i * zatPerZec + (i * 37) % zatPerZec);
      }
      for (final zat in values) {
        final s = formatZec(zat);
        expect(_parseZec(s), zat, reason: 'round-trip $zat via "$s"');
      }
    });
  });
}

/// Inverse of [formatZec], by INTEGER math — recovers the zatoshi amount
/// from the decimal string so the round-trip proves no digit was lost or
/// invented and no float crept in.
int _parseZec(String s) {
  final negative = s.startsWith('-');
  final body = negative ? s.substring(1) : s;
  final parts = body.split('.');
  final whole = int.parse(parts[0]);
  var frac = 0;
  if (parts.length == 2) {
    frac = int.parse(parts[1].padRight(8, '0'));
  }
  final magnitude = whole * zatPerZec + frac;
  return negative ? -magnitude : magnitude;
}
