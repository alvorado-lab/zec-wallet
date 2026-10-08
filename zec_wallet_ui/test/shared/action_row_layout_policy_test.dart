@TestOn('vm')
library;

import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

/// A RATCHET, not a widget test (#409 R3). The stacking rule lived in four
/// hand-written copies across `lib/`, two of them probing the wrong font size,
/// and nothing in the suite could see that: every copy passed its own tests.
/// Prose in a doc comment does not stop a fifth copy — this does.
///
/// **Both of its arms shipped defects that an adversarial pass found by
/// attacking them, which is the only way this kind of instrument gets tested.**
/// It matched the bare substring `.scale(`, so `Transform.scale` — the press
/// feedback this project's own rules recommend — tripped it, while
/// `textScaler.textScaleFactor > 1.4`, a hand-written probe with no `.scale(`
/// at all, sailed through. It also read comments, so merely NAMING the retired
/// idiom flagged a file. And it walked `Directory('lib')` relative to the CWD:
/// run from a sibling package it happily audited THAT package's `lib/` and
/// reported green without ever opening the file under test.
///
/// So: match the TextScaler vocabulary rather than one spelling of it, compare
/// against whitespace-stripped code so a copy cannot hide behind a line break,
/// guard the WIDTH half of the rule too, and prove the walk actually reached
/// the shared rule before trusting an empty result.
const _allowed = {'lib/shared/action_row_layout.dart'};

/// The per-LINE escape hatch: `// wallet-scale-probe: <why>` exempts the line
/// it sits on. Per-line, not per-file, deliberately — a file-level allowlist
/// entry added to satisfy one legitimate `TextPainter` measure would silently
/// disarm the check for every other line in that file.
const _exemptMarker = 'wallet-scale-probe:';

/// The file the walk MUST have visited. If it did not, the run proves nothing
/// and must fail loudly rather than report a clean sweep of the wrong tree.
const _sentinel = 'lib/shared/action_row_layout.dart';

/// Ways a surface can ask about the OS text scale. A local copy of the rule
/// has to use one of them; none of them appear in unrelated geometry code
/// (`Transform.scale`, `Matrix4.scale`, `Size.scale`).
const _scaleProbes = [
  'textScalerOf(',
  'TextScaler',
  'textScaler.',
  'textScaleFactor',
];

/// The width half of the rule. `kWalletStackRowWidth` is as much "the rule" as
/// the scale threshold is, and a fifth copy of `maxWidth < 520` was invisible
/// to the first draft.
final _widthProbe = RegExp(r'(maxWidth|availableWidth|width)[<>]=?520');

/// Every `lib/` Dart file as `(path, code, squashed)`.
///
/// `code` drops comment lines — both `//` and `///`, and block comments —
/// because docs name the retired idiom on purpose (this file does too, and so
/// does the shared rule). `squashed` additionally removes ALL whitespace, so
/// `. scale(` and a newline before an argument list read the same as the
/// ordinary spelling.
List<({String path, String code, String squashed})> _libSources() {
  final root = Directory('lib');
  if (!root.existsSync()) {
    fail('no lib/ under ${Directory.current.path} — run from the package root');
  }
  final out = <({String path, String code, String squashed})>[];
  for (final entity in root.listSync(recursive: true)) {
    if (entity is! File || !entity.path.endsWith('.dart')) continue;
    var inBlock = false;
    var exemptNext = false;
    final kept = <String>[];
    final marker = RegExp(
      '$_exemptMarker'
      r'\s*\S',
    );
    for (final line in entity.readAsLinesSync()) {
      final trimmed = line.trimLeft();
      if (inBlock) {
        if (trimmed.contains('*/')) inBlock = false;
        continue;
      }
      if (trimmed.startsWith('/*')) {
        if (!trimmed.contains('*/')) inBlock = true;
        continue;
      }
      // A marker in a comment exempts the next CODE line; a marker trailing a
      // code line exempts that line. Either way it must carry a reason — a
      // bare marker exempts nothing.
      final marked = marker.hasMatch(line);
      if (trimmed.startsWith('//')) {
        if (marked) exemptNext = true;
        continue;
      }
      if (trimmed.isEmpty) continue;
      if (marked || exemptNext) {
        exemptNext = false;
        continue;
      }
      // A trailing comment cannot smuggle a copy in either.
      kept.add(line.split('//').first);
    }
    final code = kept.join('\n');
    out.add((
      path: entity.path.replaceAll(r'\', '/'),
      code: code,
      squashed: code.replaceAll(RegExp(r'\s+'), ''),
    ));
  }
  return out;
}

void main() {
  test('the walk reaches the shared rule — otherwise an empty offender list '
      'is meaningless (it silently audited a sibling package before)', () {
    expect(
      _libSources().map((s) => s.path),
      contains(_sentinel),
      reason: 'wrong working directory: ${Directory.current.path}',
    );
  });

  test('the text-scale stacking rule has exactly one definition in lib/', () {
    final offenders = [
      for (final source in _libSources())
        if (!_allowed.contains(source.path) &&
            _scaleProbes.any(source.squashed.contains))
          source.path,
    ];
    expect(
      offenders,
      isEmpty,
      reason:
          'call walletTextScaleForcesStack / walletRowStacksAction instead of '
          'a local TextScaler probe — or allowlist the file above with a '
          'reason. Offenders: $offenders',
    );
  });

  test('the width half has exactly one definition too', () {
    final offenders = [
      for (final source in _libSources())
        if (!_allowed.contains(source.path) &&
            _widthProbe.hasMatch(source.squashed))
          source.path,
    ];
    expect(
      offenders,
      isEmpty,
      reason:
          'use kWalletStackRowWidth via walletRowStacksAction. '
          'Offenders: $offenders',
    );
  });

  test('no surface probes the scaler at font size 100', () {
    // The retired predicate, pinned TEXTUALLY here and behaviourally in
    // notice_layout_test.dart. `TextScaler.scale` is size-dependent by
    // contract, and Android 14+ scales small text far more than large, so a
    // 100pt probe reports a device as unscaled while its body text is at 1.5x.
    // The shared rule is checked too: it must not regrow the idiom it retired.
    final offenders = [
      for (final source in _libSources())
        if (source.squashed.contains('.scale(100)')) source.path,
    ];
    expect(offenders, isEmpty, reason: 'offenders: $offenders');
  });
}
