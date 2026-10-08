import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/diversifier_narrowing.dart';

/// Pins the FR-8 exact-narrowing guard: the u64→int narrowing
/// class dart2js makes silent must fail LOUDLY at the 2^52 boundary, and
/// every in-region value must round-trip exactly.
void main() {
  test('region_values_narrow_exactly', () {
    final base = BigInt.one << 40;
    expect(narrowDiversifierIndex(base), 1 << 40);
    final regionTop = (BigInt.one << 40) + BigInt.from(0x7FFFFFFF);
    expect(BigInt.from(narrowDiversifierIndex(regionTop)), regionTop);
  });

  test('largest_exact_value_below_the_guard_narrows', () {
    final maxOk = (BigInt.one << 52) - BigInt.one;
    expect(BigInt.from(narrowDiversifierIndex(maxOk)), maxOk);
  });

  test('at_and_above_two_pow_52_throws_loudly_and_payload_free', () {
    for (final tooBig in [
      BigInt.one << 52,
      (BigInt.one << 52) + BigInt.one,
      BigInt.one << 63,
    ]) {
      expect(
        () => narrowDiversifierIndex(tooBig),
        throwsA(
          isA<StateError>().having(
            (e) => e.message,
            'message',
            // §5.4: the message must never carry the index value.
            isNot(contains(tooBig.toString())),
          ),
        ),
      );
    }
  });
}
