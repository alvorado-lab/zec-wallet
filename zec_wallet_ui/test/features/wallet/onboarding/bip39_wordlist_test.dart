import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/bip39_wordlist.dart';

/// The bundled BIP39 wordlist (validation + autocomplete source for the restore
/// pill field). The bounds test pins the asset against drift from the audited
/// `bip39` crate it was extracted from.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test(
    'the bundled asset is the full BIP39 English list (count + bounds)',
    () async {
      final wl = await Bip39Wordlist.loadFromAsset();
      // Exactly 2048 words, the canonical abandon…zoo (gate 7 — the frozen list).
      expect(wl.length, 2048);
      expect(wl.isValid('abandon'), isTrue);
      expect(wl.isValid('ability'), isTrue);
      expect(wl.isValid('zoo'), isTrue);
      expect(wl.isValid('art'), isTrue);
      // Not a word; and the list is lowercase ASCII (the caller lowercases).
      expect(wl.isValid('zzzz'), isFalse);
      expect(wl.isValid('Abandon'), isFalse);
      expect(wl.isValid(''), isFalse);
    },
  );

  group(
    'suggestions (prefix autocomplete, binary search over the sorted list)',
    () {
      final wl = Bip39Wordlist.fromLines(
        'abandon\nability\nable\nabout\nart\nartwork\nzoo\n',
      );

      test('an empty prefix yields nothing', () {
        expect(wl.suggestions(''), isEmpty);
        expect(wl.suggestions('   '), isEmpty);
      });

      test('a prefix returns the matching words in order, capped at limit', () {
        expect(wl.suggestions('ab', limit: 4), [
          'abandon',
          'ability',
          'able',
          'about',
        ]);
        expect(wl.suggestions('ab', limit: 2), ['abandon', 'ability']);
      });

      test('an exact word and longer words sharing it both surface', () {
        expect(wl.suggestions('art'), ['art', 'artwork']);
      });

      test('is case-insensitive (the keyboard may send capitals)', () {
        expect(wl.suggestions('AB', limit: 2), ['abandon', 'ability']);
      });

      test('a non-prefix yields nothing (the sorted scan stops cleanly)', () {
        expect(wl.suggestions('xyz'), isEmpty);
        expect(wl.suggestions('zzz'), isEmpty);
      });
    },
  );
}
