import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/recoverable_ephemeral.dart';
import 'package:zec_wallet/zec_wallet.dart';

/// Pure-function tests for the recoverable one-time-address aggregator (2e-2b-iv).
/// The reduction is on a money surface, so the sum + the conservative `allFinal`
/// weld are pinned at the boundary without a device.
void main() {
  RecoverableEphemeralFunds f(int zat, {required bool isFinal}) =>
      RecoverableEphemeralFunds(recoverableZat: zat, isFinal: isFinal);

  group('summarizeRecoverable', () {
    test('empty → zero total, vacuously final (the row is hidden)', () {
      final r = summarizeRecoverable(const []);
      expect(r.totalZat, 0);
      expect(r.allFinal, isTrue);
    });

    test('a single reorg-final entry → its amount, final', () {
      final r = summarizeRecoverable([f(40000, isFinal: true)]);
      expect(r.totalZat, 40000);
      expect(r.allFinal, isTrue);
    });

    test('a single still-confirming entry → its amount, NOT final', () {
      final r = summarizeRecoverable([f(15000, isFinal: false)]);
      expect(r.totalZat, 15000);
      expect(r.allFinal, isFalse);
    });

    test('multiple entries SUM (the row shows the aggregate)', () {
      final r = summarizeRecoverable([
        f(40000, isFinal: true),
        f(60000, isFinal: true),
      ]);
      expect(r.totalZat, 100000);
      expect(r.allFinal, isTrue);
    });

    test('any still-confirming portion makes the whole row not-final '
        '(conservative — never imply settled while a part is shallow)', () {
      final r = summarizeRecoverable([
        f(40000, isFinal: true),
        f(15000, isFinal: false),
      ]);
      expect(r.totalZat, 55000);
      expect(r.allFinal, isFalse);
    });
  });
}
