import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

/// The package's PUBLIC-SURFACE invariants, checked against the source rather
/// than asserted in a comment.
///
/// `zec_wallet_ui` ships to pub.dev, so its barrel is a permanent contract. The
/// `lib/src/` rule (stated in full at the top of `lib/zec_wallet_ui.dart`) is:
/// a type belongs under `lib/src/` iff a host constructing it directly could
/// fabricate a claim the SDK never made. That criterion is semantic and no test
/// can decide it — but its MECHANISM is checkable, and the mechanism is the part
/// that can be silently destroyed by one line.
void main() {
  final barrel = File('lib/zec_wallet_ui.dart');
  final libSrc = Directory('lib/src');

  test('the barrel never exports anything under lib/src', () {
    // `lib/src/` works because the analyzer's `implementation_imports` lint
    // stops a HOST importing it. A re-export from the barrel hands the same
    // types over through the front door and the lint never fires — the
    // protection is gone with nothing red. This is the one line that can do it.
    expect(barrel.existsSync(), isTrue, reason: 'run from the package root');
    final offending = barrel
        .readAsLinesSync()
        .where((l) => l.trimLeft().startsWith('export '))
        .where((l) => l.contains("'src/") || l.contains('/src/'))
        .toList();
    expect(
      offending,
      isEmpty,
      reason:
          'the barrel re-exports a lib/src file, which defeats the '
          'implementation_imports lint that is the whole mechanism. If the type '
          'genuinely belongs on the public API, MOVE it out of lib/src '
          'deliberately — do not export it from where it hides.',
    );
  });

  test('lib/src carries only files that could FORGE a claim, and says so', () {
    // Anti-vacuity: if lib/src is ever emptied, this test must stop claiming to
    // guard something. And every file there should name the rule it is under,
    // so the criterion travels with the code rather than only living in the
    // barrel's header.
    if (!libSrc.existsSync()) {
      fail(
        'lib/src is gone. Either the delivery internals moved (update the rule '
        'in lib/zec_wallet_ui.dart and delete this test), or the protection was '
        'removed by accident.',
      );
    }
    final files = libSrc
        .listSync(recursive: true)
        .whereType<File>()
        .where((f) => f.path.endsWith('.dart'))
        .toList();
    expect(
      files,
      isNotEmpty,
      reason: 'lib/src exists but is empty — see the note above',
    );
    for (final f in files) {
      expect(
        f.readAsStringSync(),
        contains('lib/src/'),
        reason:
            '${f.path} lives under lib/src but never explains why. State the '
            'forgery it prevents (the rule is at the top of '
            'lib/zec_wallet_ui.dart) — a file here with no rationale is how the '
            'boundary erodes.',
      );
    }
  });
}
