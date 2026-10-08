import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart' show maxMoneyZat;
import 'package:zec_wallet_ui/features/wallet/send/zec_amount.dart';
import 'package:zec_wallet_ui/features/wallet/zat_format.dart';

/// Money-parser tests (inc-2d-ui). `parseZecAmount` is the host-side first gate
/// before a send amount crosses the bridge — integer-exact, the strict inverse of
/// `formatZec`. Bugs here send the WRONG amount, so the boundary, the
/// round-trip, and the adversarial inputs are all pinned (testing-patterns: a
/// money parser needs more than happy-path examples).
void main() {
  int zatOf(String s) {
    final r = parseZecAmount(s);
    expect(r, isA<ZecAmountValid>(), reason: '"$s" should parse');
    return (r as ZecAmountValid).zat;
  }

  ZecAmountFault faultOf(String s) {
    final r = parseZecAmount(s);
    expect(r, isA<ZecAmountInvalid>(), reason: '"$s" should be rejected');
    return (r as ZecAmountInvalid).fault;
  }

  group('valid amounts (integer-exact)', () {
    test('whole ZEC', () => expect(zatOf('1'), 100000000));
    test('one zatoshi', () => expect(zatOf('0.00000001'), 1));
    test('leading-dot fraction fixes place value', () {
      // ".5" is half a ZEC = 50_000_000 zat, NOT 5 — the fractional part is
      // right-padded to 8 places, never read as a bare integer.
      expect(zatOf('.5'), 50000000);
    });
    test('mixed', () => expect(zatOf('1.2345'), 123450000));
    test('trailing fractional zeros are positional, not trimmed', () {
      expect(zatOf('0.10000000'), 10000000);
    });
    test(
      'surrounding whitespace is trimmed',
      () => expect(zatOf('  1  '), 100000000),
    );
    test('leading zeros are tolerated', () => expect(zatOf('007'), 700000000));
    test('exactly max money', () => expect(zatOf('21000000'), maxMoneyZat));
    test('one zatoshi below max money', () {
      expect(zatOf('20999999.99999999'), maxMoneyZat - 1);
    });
  });

  group('rejections (typed, payload-free)', () {
    test('empty', () => expect(faultOf(''), ZecAmountFault.empty));
    test('whitespace only', () => expect(faultOf('   '), ZecAmountFault.empty));
    test('letters', () => expect(faultOf('abc'), ZecAmountFault.notANumber));
    test(
      'a lone dot is not a number',
      () => expect(faultOf('.'), ZecAmountFault.notANumber),
    );
    test(
      'thousands separator',
      () => expect(faultOf('1,000'), ZecAmountFault.notANumber),
    );
    test(
      'negative sign',
      () => expect(faultOf('-1'), ZecAmountFault.notANumber),
    );
    test('plus sign', () => expect(faultOf('+1'), ZecAmountFault.notANumber));
    test('two dots', () => expect(faultOf('1.2.3'), ZecAmountFault.notANumber));
    test('exponent', () => expect(faultOf('1e3'), ZecAmountFault.notANumber));
    test(
      'internal space',
      () => expect(faultOf('1 0'), ZecAmountFault.notANumber),
    );
    test(
      '9 fractional digits',
      () => expect(faultOf('0.000000001'), ZecAmountFault.tooManyDecimals),
    );
    test(
      'zero is not a payment',
      () => expect(faultOf('0'), ZecAmountFault.notPositive),
    );
    test(
      'zero with decimals',
      () => expect(faultOf('0.0'), ZecAmountFault.notPositive),
    );
    test(
      'one zatoshi over max money',
      () => expect(faultOf('21000000.00000001'), ZecAmountFault.outOfRange),
    );
    test(
      'whole part over supply',
      () => expect(faultOf('21000001'), ZecAmountFault.outOfRange),
    );
    test('absurd paste cannot overflow int64 (length guard before parse)', () {
      // A 40-digit whole part is rejected as out-of-range, never multiplied into
      // an overflowed (wrong, possibly-positive) zatoshi value.
      expect(faultOf('9' * 40), ZecAmountFault.outOfRange);
    });
    test(
      'an 18-digit whole part is rejected BEFORE the multiply can overflow',
      () {
        // 1e17 ZEC parses fine (≤18 digits) but the supply ceiling on the whole
        // part rejects it before `whole * 1e8` (~1e25) could overflow int64 and
        // wrap to a wrong, possibly-positive zatoshi value.
        expect(faultOf('100000000000000000'), ZecAmountFault.outOfRange);
      },
    );
    test('leading zeros up to max supply still parse exactly', () {
      // The numeric (not digit-count) ceiling tolerates leading zeros.
      expect(zatOf('00021000000'), maxMoneyZat);
    });
  });

  group('round-trips with formatZec (the inverse contract)', () {
    // parseZecAmount(formatZec(x)) == ZecAmountValid(x) for every positive,
    // in-range x — the load-bearing money-display↔money-entry symmetry.
    final samples = <int>[
      1,
      99,
      1000,
      12345678,
      99999999, // the whole↔fractional formatting boundary (zatPerZec - 1)
      100000000,
      123450000,
      maxMoneyZat - 1,
      maxMoneyZat,
    ];
    for (final x in samples) {
      test('round-trip $x zat', () => expect(zatOf(formatZec(x)), x));
    }
  });

  test('maxMoneyZat is pinned at its boundary (gate 7)', () {
    // 21e6 ZEC × 1e8 zat/ZEC.
    expect(maxMoneyZat, 21000000 * zatPerZec);
  });
}
