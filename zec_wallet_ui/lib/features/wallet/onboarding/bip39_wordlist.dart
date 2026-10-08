import 'package:flutter/services.dart' show rootBundle;
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// The frozen BIP39 English wordlist (2048 words), bundled as an asset and used
/// by the restore screen for LIVE word validation + prefix autocomplete — so a
/// word the user types becomes a pill iff it is a real recovery word, and typos
/// surface as they type instead of only on submit.
///
/// This is UX GUIDANCE ONLY: the SDK's audited `bip39` parser remains the real
/// gate (checksum + wordlist) at restore time. The asset is extracted from that
/// same crate's wordlist (one source of truth; drift is caught by the bounds
/// test). The BIP39 English list is FROZEN — it will never change.
class Bip39Wordlist {
  /// [words] MUST be the 2048 words in the canonical (sorted, lowercase ASCII)
  /// order — the asset ships that way, which is what makes the binary-search
  /// prefix scan in [suggestions] correct.
  const Bip39Wordlist(this._words, this._set);

  factory Bip39Wordlist.fromLines(String raw) {
    final words = raw
        .split('\n')
        .map((w) => w.trim())
        .where((w) => w.isNotEmpty)
        .toList(growable: false);
    // The binary-search prefix scan in [suggestions] is correct ONLY for a sorted
    // list (the asset ships sorted) — assert it in debug so an unsorted fixture
    // fails loudly instead of silently dropping suggestions.
    assert(() {
      for (var i = 1; i < words.length; i++) {
        if (words[i - 1].compareTo(words[i]) > 0) return false;
      }
      return true;
    }(), 'Bip39Wordlist must be sorted (binary-search autocomplete)');
    return Bip39Wordlist(words, words.toSet());
  }

  final List<String> _words;
  final Set<String> _set;

  int get length => _words.length;

  /// Whether [word] (already lowercased/trimmed) is a BIP39 word — O(1).
  bool isValid(String word) => _set.contains(word);

  /// Up to [limit] words that start with [prefix] (case-insensitive), in
  /// wordlist order. Empty for an empty prefix. Binary-search over the sorted
  /// list — no full scan per keystroke.
  List<String> suggestions(String prefix, {int limit = 4}) {
    final p = prefix.trim().toLowerCase();
    if (p.isEmpty) return const [];
    // Lower bound: first index whose word is >= prefix.
    var lo = 0;
    var hi = _words.length;
    while (lo < hi) {
      final mid = (lo + hi) >> 1;
      if (_words[mid].compareTo(p) < 0) {
        lo = mid + 1;
      } else {
        hi = mid;
      }
    }
    final out = <String>[];
    for (var i = lo; i < _words.length && out.length < limit; i++) {
      if (_words[i].startsWith(p)) {
        out.add(_words[i]);
      } else {
        break; // sorted ⇒ the prefix run ended
      }
    }
    return out;
  }

  static Future<Bip39Wordlist> loadFromAsset() async {
    // The wordlist ships with the zec_wallet_ui package (phase-1 extraction),
    // so it lives under the package-asset prefix.
    final raw = await rootBundle.loadString(
      'packages/zec_wallet_ui/assets/bip39/english.txt',
    );
    return Bip39Wordlist.fromLines(raw);
  }
}

/// Loads the bundled wordlist once. The restore screen watches this; if it
/// hasn't loaded yet the field still works (validation/autocomplete just stay
/// quiet until it resolves — never a blocker, the SDK is the gate). Tests inject
/// a small fixture via `overrideWith`.
final bip39WordlistProvider = FutureProvider<Bip39Wordlist>(
  (ref) => Bip39Wordlist.loadFromAsset(),
);
