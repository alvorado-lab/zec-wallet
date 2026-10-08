import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_chains.dart';

void main() {
  group('chainDisplayName', () {
    test('maps common provider codes to human names (case-insensitive)', () {
      expect(chainDisplayName('eth'), 'Ethereum');
      expect(chainDisplayName('ETH'), 'Ethereum');
      expect(chainDisplayName('btc'), 'Bitcoin');
      expect(chainDisplayName('near'), 'NEAR');
      expect(chainDisplayName('sol'), 'Solana');
      expect(chainDisplayName('base'), 'Base');
    });

    test(
      'an unknown chain falls back to the upper-cased code (never guesses)',
      () {
        // The swap still works (the code is the wire id); we just don't invent a
        // pretty name we don't know.
        expect(chainDisplayName('zkxyz'), 'ZKXYZ');
        expect(chainDisplayName('Foo'), 'FOO');
      },
    );

    test(
      'no more "BTC on BTC": btc resolves to Bitcoin, not the bare code',
      () {
        // Regression for the maintainer report — the redundant code label.
        expect(chainDisplayName('btc'), isNot('BTC'));
        expect(chainDisplayName('btc'), 'Bitcoin');
      },
    );
  });
}
