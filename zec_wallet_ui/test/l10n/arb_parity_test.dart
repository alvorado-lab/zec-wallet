import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

/// Every shipped locale carries every English key.
///
/// gen-l10n fills a key missing from a locale's ARB with the English value and
/// says so only in a build warning nobody reads, so a new string shipped in
/// English to 15 locales for a whole stage: the five balance keys
/// ("Hide balance", "Balance hidden", "Arriving", the caption). Relim's own
/// parity test does not reach this package, so the SDK carries its own.
void main() {
  final dir = Directory('lib/l10n');
  final arbs = dir
      .listSync()
      .whereType<File>()
      .where((f) => f.path.endsWith('.arb'))
      .toList();

  Map<String, dynamic> read(File f) =>
      jsonDecode(f.readAsStringSync()) as Map<String, dynamic>;
  Set<String> keysOf(Map<String, dynamic> arb) =>
      arb.keys.where((k) => !k.startsWith('@')).toSet();

  final en = read(File('lib/l10n/wallet_en.arb'));
  final enKeys = keysOf(en);

  test('the ARB set is the 16 shipped locales and English is not trivial', () {
    // Non-vacuity: a moved directory or an emptied template would pass the
    // parity check below with nothing compared.
    expect(arbs, hasLength(16));
    expect(enKeys.length, greaterThan(800));
  });

  for (final f in arbs.where((f) => !f.path.endsWith('wallet_en.arb'))) {
    final name = f.uri.pathSegments.last;
    test('$name carries every English key, with the same placeholders', () {
      final arb = read(f);
      final missing = enKeys.difference(keysOf(arb)).toList()..sort();
      expect(missing, isEmpty, reason: '$name lacks these English keys');
      final extra = keysOf(arb).difference(enKeys).toList()..sort();
      expect(extra, isEmpty, reason: '$name has keys English does not');
      for (final k in enKeys) {
        final meta = en['@$k'];
        if (meta is! Map || meta['placeholders'] is! Map) continue;
        for (final p in (meta['placeholders'] as Map).keys) {
          expect(
            (arb[k] as String).contains('{$p'),
            isTrue,
            reason: '$name: $k drops the {$p} placeholder',
          );
        }
      }
    });
  }
}
