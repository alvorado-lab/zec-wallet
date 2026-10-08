import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/send_authorization.dart';

/// Unit pins for [abbreviateWalletAddress] (#383 R3) — the deterministic
/// display elision a host's per-spend prompt binds confirm→sign to. Pure
/// string mapping, no widget tree. (§5.4 note: display-only; never logged.)
void main() {
  group('abbreviateWalletAddress', () {
    test('a short address returns unchanged — including exactly 20 chars '
        '(the inclusive bound)', () {
      expect(abbreviateWalletAddress('u1short'), 'u1short');
      const exactly20 = 'u1234567890123456789';
      expect(exactly20.length, 20, reason: 'the fixture pins the boundary');
      expect(abbreviateWalletAddress(exactly20), exactly20);
    });

    test('a long address elides to the first 12 + … + the last 6', () {
      const long = 'u1abcdefghijklmnopqrstuvwxyz012345';
      final abbrev = abbreviateWalletAddress(long);
      expect(abbrev, 'u1abcdefghij…012345');
      // Both ends come from the ORIGINAL string — enough to visually match
      // against a copied address.
      expect(
        abbrev,
        '${long.substring(0, 12)}…${long.substring(long.length - 6)}',
      );
    });

    test('surrounding whitespace is trimmed BEFORE the length check', () {
      // A padded short address stays unchanged (the pad never counts).
      expect(abbreviateWalletAddress('  u1short \n'), 'u1short');
      const exactly20 = 'u1234567890123456789';
      expect(abbreviateWalletAddress('  $exactly20  '), exactly20);
      // A padded long address elides from the TRIMMED ends — never a space
      // inside the elision.
      const long = 'u1abcdefghijklmnopqrstuvwxyz012345';
      expect(abbreviateWalletAddress(' $long '), 'u1abcdefghij…012345');
    });
  });
}
