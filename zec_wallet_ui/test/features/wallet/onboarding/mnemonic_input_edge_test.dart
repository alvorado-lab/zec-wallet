import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/mnemonic_input.dart';

/// SECOND-PASS real-world-edge tests for the restore-input normalizer — the
/// messy clipboard/keyboard cases the first-pass `mnemonic_input_test.dart`
/// (single spaces, a mixed `\n\t` run, leading/trailing/double spaces,
/// lowercase, accent-preservation, a 4×cap single token) does NOT pin.
///
/// The money lens: a recovery phrase is pasted from a notes app, a webpage, a
/// password manager, or a messaging app — each of which injects its own
/// separator soup (NBSP from HTML, CRLF from Windows notes, an ideographic
/// space from a CJK keyboard) and its own "helpful" punctuation (a smart-quote
/// apostrophe, a trailing period). The contract is exact:
///   * ANY Unicode-whitespace run is a word separator (so a paste restores), AND
///   * NOTHING else is touched — a smart-quote, a period, or a zero-width space
///     stays ATTACHED to its word so the audited BIP39 parser REJECTS it with an
///     index, rather than the normalizer silently "fixing" it into a different
///     valid word and deriving a DIFFERENT wallet (the catastrophic money bug
///     the narrow normalizer exists to avoid).
/// These pin both halves at their boundary.
void main() {
  group('Unicode-whitespace separators (paste-soup → words restore)', () {
    // Each entry is a real separator a paste can carry. ALL must split exactly
    // like a plain space — a phrase copied from any source restores.
    const cases = <String, String>{
      'NBSP U+00A0 (HTML / web paste)': ' ',
      'CRLF (Windows notes app)': '\r\n',
      'bare CR': '\r',
      'TAB': '\t',
      'vertical tab U+000B': '',
      'form feed U+000C': '',
      'ideographic space U+3000 (CJK keyboard)': '　',
      'thin space U+2009': ' ',
      'en quad U+2000': ' ',
      'narrow NBSP U+202F': ' ',
      'line separator U+2028': ' ',
    };
    cases.forEach((label, sep) {
      test('$label is a word separator', () {
        expect(
          normalizeMnemonicInput('abandon${sep}ability${sep}able'),
          ['abandon', 'ability', 'able'],
          reason: 'a phrase pasted with this separator must split into words',
        );
      });
    });

    test(
      'a mixed soup of separators in one paste collapses to clean words',
      () {
        // A phrase that travelled through several apps, accreting separators.
        expect(normalizeMnemonicInput(' abandon\r\n\tability　　able  about  '), [
          'abandon',
          'ability',
          'able',
          'about',
        ]);
      },
    );
  });

  group('punctuation/format chars stay ATTACHED (no silent wrong-wallet)', () {
    // The load-bearing safety property: the normalizer must NOT strip these.
    // Left attached, the word fails BIP39 validation (an honest "check word N");
    // stripped, it could collide with another valid word → a different wallet.
    test('a smart-quote apostrophe (keyboard autocorrect) stays attached', () {
      // 'abandon's' is not a wordlist word; it MUST reach the SDK intact so the
      // parser rejects it, not be silently de-punctuated.
      expect(normalizeMnemonicInput('abandon’s ability'), [
        'abandon’s',
        'ability',
      ]);
    });

    test('a trailing period (sentence paste) stays attached to its word', () {
      expect(normalizeMnemonicInput('abandon ability.'), [
        'abandon',
        'ability.',
      ]);
    });

    test(
      'a zero-width space U+200B is NOT whitespace — it stays in the word',
      () {
        // U+200B is a format char, not whitespace: `\s` does not match it, so the
        // two tokens fuse into one non-word that BIP39 rejects — exactly right,
        // never a silent split that could mask a corrupted paste.
        expect(normalizeMnemonicInput('abandon​ability'), ['abandon​ability']);
      },
    );

    test('a hyphen / underscore in a token is preserved (not a separator)', () {
      expect(normalizeMnemonicInput('ab-andon ability'), [
        'ab-andon',
        'ability',
      ]);
    });
  });

  group('all-whitespace paste yields the empty list (no phantom word)', () {
    test('a paste of mixed Unicode whitespace only → []', () {
      // Order-of-magnitude real: a user fat-fingers a paste of indentation.
      expect(normalizeMnemonicInput(' \t\r\n 　  '), isEmpty);
    });

    test('a single NBSP → []', () {
      expect(normalizeMnemonicInput(' '), isEmpty);
    });
  });

  group('the kMaxMnemonicInputChars cap at its exact boundary (gate 7)', () {
    test(
      'exactly-at-cap input is processed whole (no truncation at the cap)',
      () {
        // A single token of EXACTLY the cap length: kept entire (the cap is a
        // ceiling, not an off-by-one that clips a legal-length input).
        final atCap = 'a' * kMaxMnemonicInputChars;
        final words = normalizeMnemonicInput(atCap);
        expect(words, hasLength(1));
        expect(words.first.length, kMaxMnemonicInputChars);
      },
    );

    test('one char OVER the cap is truncated to exactly the cap', () {
      // The boundary the 4×cap first-pass test brackets but never pins to ±1:
      // cap+1 must clip to cap, never cap+1.
      final overByOne = 'a' * (kMaxMnemonicInputChars + 1);
      final words = normalizeMnemonicInput(overByOne);
      expect(
        words.first.length,
        kMaxMnemonicInputChars,
        reason: 'cap+1 must truncate to the cap, never pass an extra char',
      );
    });

    test(
      'truncation happens BEFORE the split — a word straddling the cap is cut, '
      'never silently dropped wholesale',
      () {
        // The cap lands mid-word: the input up to the cap is split, so the words
        // before the boundary survive and only the straddling tail is clipped.
        // This proves size-cap-before-alloc bounds the work without discarding
        // the legitimate prefix.
        final prefix = 'word ' * 10; // 50 chars, 10 whole words
        final filler = 'z' * (kMaxMnemonicInputChars - prefix.length + 50);
        final words = normalizeMnemonicInput(prefix + filler);
        // The 10 leading words survive; the trailing filler is bounded.
        expect(words.take(10).toList(), List.filled(10, 'word'));
        // Total processed text never exceeded the cap.
        expect(
          words.join(' ').length,
          lessThanOrEqualTo(kMaxMnemonicInputChars),
        );
      },
    );
  });

  group('idempotence over a full messy paste (the controller re-applies it)', () {
    test('normalizing already-normalized output is a fixed point', () {
      // The controller runs normalizeMnemonicWord defensively over the screen\'s
      // already-split words; re-joining and re-splitting must not drift.
      const messy = ' Abandon\r\nABILITY\t\taBoUt　';
      final once = normalizeMnemonicInput(messy);
      final twice = normalizeMnemonicInput(once.join(' '));
      expect(twice, once);
      expect(once, ['abandon', 'ability', 'about']);
    });

    test('lowercasing is applied per word across every separator kind', () {
      expect(normalizeMnemonicInput('ZOO Legal\tArt\r\nABOUT'), [
        'zoo',
        'legal',
        'art',
        'about',
      ]);
    });
  });
}
