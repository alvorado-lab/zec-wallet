import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/mnemonic_input.dart';

/// The restore-input normalizer — the host's half of the load-bearing BIP39
/// contract (the audited parser does NOT case-fold, so the host MUST lowercase +
/// trim or a CORRECT backup is rejected). These pin that the normalizer does
/// exactly what the contract requires — and NOTHING more (no accent folding, no
/// word "correction" that could silently derive a different wallet).
void main() {
  group('normalizeMnemonicWord', () {
    test('lowercases — the autocapitalization money bug', () {
      // A soft keyboard autocapitalizes the first word; the wordlist is lowercase
      // ASCII, so without this the SDK rejects a correct backup with an index.
      expect(normalizeMnemonicWord('Abandon'), 'abandon');
      expect(normalizeMnemonicWord('ZOO'), 'zoo');
    });

    test('trims surrounding whitespace', () {
      expect(normalizeMnemonicWord('  art  '), 'art');
      expect(normalizeMnemonicWord('\tlegal\n'), 'legal');
    });

    test('is idempotent (so the controller can re-apply it defensively)', () {
      const already = 'abandon';
      expect(normalizeMnemonicWord(normalizeMnemonicWord(already)), already);
    });
  });

  group('normalizeMnemonicInput', () {
    test('splits on single spaces into ordered words', () {
      expect(normalizeMnemonicInput('abandon ability able about'), [
        'abandon',
        'ability',
        'able',
        'about',
      ]);
    });

    test(
      'collapses any whitespace run — paste-friendly (spaces/newlines/tabs)',
      () {
        expect(normalizeMnemonicInput('abandon   ability\nable\t\tabout'), [
          'abandon',
          'ability',
          'able',
          'about',
        ]);
      },
    );

    test('drops the empties leading/trailing/double spaces would produce', () {
      expect(normalizeMnemonicInput('   abandon   ability   '), [
        'abandon',
        'ability',
      ]);
      expect(normalizeMnemonicInput('   '), isEmpty);
      expect(normalizeMnemonicInput(''), isEmpty);
    });

    test(
      'lowercases every word — a pasted/typed mixed-case phrase restores',
      () {
        expect(normalizeMnemonicInput('Abandon ABILITY AbOuT'), [
          'abandon',
          'ability',
          'about',
        ]);
      },
    );

    test('preserves order (reading order is load-bearing for BIP39)', () {
      final words = normalizeMnemonicInput(
        'zoo abandon ability zoo about legal',
      );
      expect(words, ['zoo', 'abandon', 'ability', 'zoo', 'about', 'legal']);
    });

    test(
      'does NOT fold accents or alter the letters (audited parser decides)',
      () {
        // Over-normalizing could turn one valid word into another → a DIFFERENT
        // wallet. We only lowercase + trim; everything else is the SDK's job.
        expect(normalizeMnemonicInput('café'), ['café']);
      },
    );

    test(
      'caps a pathological over-long paste at kMaxMnemonicInputChars (gate 7)',
      () {
        // The Dart-side size-cap-before-alloc: a multi-megabyte single-token paste
        // must not allocate unbounded on the per-keystroke recompute. Truncation
        // can only fail BIP39 downstream — never a silent wrong wallet.
        final huge = 'a' * (kMaxMnemonicInputChars * 4);
        final words = normalizeMnemonicInput(huge);
        // One token, bounded to the cap — not 4× the cap.
        expect(words, ['a' * kMaxMnemonicInputChars]);
      },
    );
  });
}
