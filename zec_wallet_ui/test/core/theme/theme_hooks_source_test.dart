import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

/// FR-49 (ADR-0564): a host restyles the wallet UI through its theme hooks,
/// so a widget that names a Material glyph, the generic mono face or a
/// bundled font directly is a site the host can never reach. This reads the
/// SOURCE, because a hook the widgets bypass renders fine and fails nothing
/// else.
void main() {
  final lib = Directory('lib');
  final dartFiles = lib
      .listSync(recursive: true)
      .whereType<File>()
      .where((f) => f.path.endsWith('.dart'))
      .toList();

  String rel(File f) => f.path.replaceAll(r'\', '/');

  // A Material glyph reference, not an identifier that merely ends in "Icons"
  // (`_bundledTokenIcons.contains`).
  final materialIcon = RegExp(r'(?<![A-Za-z0-9_])Icons\.[a-z]');
  // A Material `Icon(` widget constructed directly — not `WalletIcon(`,
  // `IconButton(`, `SwapTokenIcon(`, nor a `.icon(` named constructor such
  // as `FilledButton.icon(`.
  final materialIconWidget = RegExp(r'(?<![A-Za-z0-9_.])Icon\(');
  final genericMono = RegExp(r'''fontFamily:\s*['"]monospace['"]''');

  test('the scan is not vacuous', () {
    expect(
      dartFiles.length,
      greaterThan(100),
      reason: 'run from the package root',
    );
    final icons = dartFiles.singleWhere(
      (f) => rel(f).endsWith('lib/core/theme/icons.dart'),
    );
    expect(
      materialIcon.allMatches(icons.readAsStringSync()).length,
      greaterThan(60),
      reason: 'the pattern must match the defaults it exempts',
    );
    expect(
      materialIconWidget.allMatches(icons.readAsStringSync()).length,
      greaterThanOrEqualTo(1),
      reason: 'the pattern must match the default rendering it exempts',
    );
    final typography = dartFiles.singleWhere(
      (f) => rel(f).endsWith('lib/core/theme/typography.dart'),
    );
    expect(genericMono.hasMatch(typography.readAsStringSync()), isTrue);
  });

  test('no widget names a Material glyph outside WalletIcons', () {
    final offenders = <String>[];
    for (final f in dartFiles) {
      if (rel(f).endsWith('lib/core/theme/icons.dart')) continue;
      final lines = f.readAsLinesSync();
      for (var i = 0; i < lines.length; i++) {
        if (materialIcon.hasMatch(lines[i])) {
          offenders.add('${rel(f)}:${i + 1}: ${lines[i].trim()}');
        }
      }
    }
    expect(
      offenders,
      isEmpty,
      reason: 'name the glyph as a WalletGlyph and draw it with WalletIcon',
    );
  });

  test('no widget constructs a Material Icon outside WalletIcons', () {
    final offenders = <String>[];
    for (final f in dartFiles) {
      if (rel(f).endsWith('lib/core/theme/icons.dart')) continue;
      final lines = f.readAsLinesSync();
      for (var i = 0; i < lines.length; i++) {
        if (materialIconWidget.hasMatch(lines[i])) {
          offenders.add('${rel(f)}:${i + 1}: ${lines[i].trim()}');
        }
      }
    }
    expect(
      offenders,
      isEmpty,
      reason:
          'draw icons with WalletIcon(WalletGlyph.…) so the host hook '
          'reaches them',
    );
  });

  test('no widget names the generic mono face outside WalletTypography', () {
    final offenders = <String>[];
    for (final f in dartFiles) {
      if (rel(f).endsWith('lib/core/theme/typography.dart')) continue;
      final lines = f.readAsLinesSync();
      for (var i = 0; i < lines.length; i++) {
        if (genericMono.hasMatch(lines[i])) {
          offenders.add('${rel(f)}:${i + 1}: ${lines[i].trim()}');
        }
      }
    }
    expect(
      offenders,
      isEmpty,
      reason: 'set identifiers with WalletTypography.of(context).monoOn(...)',
    );
  });

  // --- Stage S11 (FR-49 W-11): call sites carry no style literal ---------
  //
  // Each ban names the ONE file that owns the thing it bans, and asserts that
  // owner still holds a match — a pattern that has stopped matching anything
  // would pass every file, including the offending ones.

  /// The code of [f] with `//` comments blanked, so a doc comment that names a
  /// banned call is not an offender (and cannot stand in for the owner's
  /// real use either).
  String codeOf(File f) => f
      .readAsLinesSync()
      .map((l) => l.trimLeft().startsWith('//') ? '' : l)
      .join('\n');

  File owner(String path) =>
      dartFiles.singleWhere((f) => rel(f).endsWith('lib/$path'));

  /// The source line of [offset] in [code], for an offender report.
  String lineAt(File f, String code, int offset) {
    final line = '\n'.allMatches(code.substring(0, offset)).length;
    return '${rel(f)}:${line + 1}: ${code.split('\n')[line].trim()}';
  }

  /// The argument text of the call whose `(` is at [open], to its matching
  /// `)` — a call may span lines (`BorderRadius.circular(\n  WalletShapes…`).
  String argsOf(String code, int open) {
    var depth = 0;
    for (var i = open; i < code.length; i++) {
      if (code[i] == '(') depth++;
      if (code[i] == ')' && --depth == 0) return code.substring(open + 1, i);
    }
    return code.substring(open + 1);
  }

  /// Every [pattern] match in the package's code outside [ownerPath].
  List<String> offendersOf(RegExp pattern, String ownerPath) => [
    for (final f in dartFiles)
      if (!rel(f).endsWith('lib/$ownerPath'))
        for (final m in pattern.allMatches(codeOf(f)))
          lineAt(f, codeOf(f), m.start),
  ];

  // C1: every corner radius names a WalletShapes role. The argument of a
  // `Radius.circular(` / `BorderRadius.circular(` must read the shapes (the
  // numeric-literal form the contract names is the common case; a computed
  // literal such as `_prominent ? 12 : 8` is the same defect).
  final radiusCall = RegExp(r'(?<![A-Za-z0-9_])(Border)?Radius\.circular\(');
  final numericRadius = RegExp(r'(Border)?Radius\.circular\(\s*[0-9]');
  bool readsShapes(String args) => RegExp('[Ss]hapes').hasMatch(args);

  test('S11 C1: every radius reads WalletShapes (no literal radius outside '
      'shapes.dart)', () {
    // Non-vacuous: the pattern catches the literal form, and passes the
    // sanctioned one; and the package really does read the shapes widely.
    const literal = 'borderRadius: BorderRadius.circular(12),';
    expect(numericRadius.hasMatch(literal), isTrue);
    final open = literal.indexOf('(');
    expect(readsShapes(argsOf(literal, open)), isFalse);
    const hooked =
        'BorderRadius.circular(\n  WalletShapes.of(context).group,\n)';
    expect(readsShapes(argsOf(hooked, hooked.indexOf('('))), isTrue);
    final shapesFile = codeOf(owner('core/theme/shapes.dart'));
    expect(shapesFile, contains('this.group = 24'), reason: 'the owner');
    final hookReads = dartFiles
        .map((f) => 'WalletShapes.of('.allMatches(codeOf(f)).length)
        .fold<int>(0, (a, b) => a + b);
    expect(hookReads, greaterThan(20), reason: 'the sites read the hook');

    final offenders = <String>[];
    for (final f in dartFiles) {
      if (rel(f).endsWith('lib/core/theme/shapes.dart')) continue;
      final code = codeOf(f);
      for (final m in radiusCall.allMatches(code)) {
        final args = argsOf(code, m.end - 1);
        if (!readsShapes(args)) offenders.add(lineAt(f, code, m.start));
      }
    }
    expect(
      offenders,
      isEmpty,
      reason:
          'read the role: BorderRadius.circular(WalletShapes.of(context).…)',
    );
  });

  test('S11 C6: no button styleFrom sets minimumSize outside theme.dart', () {
    final minimumSize = RegExp(r'(?<![A-Za-z0-9_])minimumSize:');
    expect(
      minimumSize.allMatches(codeOf(owner('core/theme/theme.dart'))).length,
      greaterThanOrEqualTo(1),
      reason: 'the owner: the component themes set the 48 minimum',
    );
    expect(
      offendersOf(minimumSize, 'core/theme/theme.dart'),
      isEmpty,
      reason: 'the theme owns the size; delete the literal',
    );
  });

  test('S11 C6: no OutlineInputBorder outside theme.dart', () {
    final outline = RegExp(r'(?<![A-Za-z0-9_])OutlineInputBorder\(');
    expect(
      outline.allMatches(codeOf(owner('core/theme/theme.dart'))).length,
      greaterThanOrEqualTo(1),
      reason: 'the owner: inputDecorationTheme draws the field',
    );
    expect(
      offendersOf(outline, 'core/theme/theme.dart'),
      isEmpty,
      reason: 'the theme owns the field border; delete the literal',
    );
  });

  test('S11 C3: every dialog goes through showWalletConfirm', () {
    // `showDialog<bool>(` as well as `showDialog(`; showDatePicker is not a
    // dialog of this kind and stays Material (§9.4). NAMED EXEMPTION (§6.9):
    // a future dialog that holds a text field is Material on every platform,
    // which the helper does not draw; it goes on a line carrying
    // `// wallet-dialog-exempt: text entry (§6.9)` and nowhere else.
    final dialog = RegExp(
      r'(?<![A-Za-z0-9_])(show(Adaptive|Cupertino)?Dialog\s*(<[^>(]*>)?\(|'
      r'(Cupertino)?AlertDialog\()',
    );
    const exemption = '// wallet-dialog-exempt: text entry (§6.9)';
    expect(dialog.hasMatch('await showDialog<bool>('), isTrue);
    expect(dialog.hasMatch('showDatePicker(context: c)'), isFalse);
    final helper = codeOf(owner('shared/wallet_dialog.dart'));
    expect(
      dialog.allMatches(helper).length,
      greaterThanOrEqualTo(2),
      reason: 'the owner opens the dialog and builds both forms',
    );
    final offenders = [
      for (final f in dartFiles)
        if (!rel(f).endsWith('lib/shared/wallet_dialog.dart'))
          for (final (i, line) in f.readAsLinesSync().indexed)
            if (!line.trimLeft().startsWith('//') &&
                dialog.hasMatch(line) &&
                !line.contains(exemption))
              '${rel(f)}:${i + 1}: ${line.trim()}',
    ];
    expect(
      offenders,
      isEmpty,
      reason:
          'open it with showWalletConfirm(kind: …) so dismissal is never '
          'consent and the platform form is chosen in one place',
    );
  });

  test('S11 C4: every sheet goes through showWalletSheet', () {
    final sheet = RegExp(
      r'(?<![A-Za-z0-9_])showModalBottomSheet\s*(<[^>(]*>)?\(',
    );
    expect(sheet.hasMatch('return showModalBottomSheet<SwapToken>('), isTrue);
    expect(
      sheet.allMatches(codeOf(owner('shared/wallet_sheet.dart'))).length,
      greaterThanOrEqualTo(1),
      reason: 'the owner opens the sheet',
    );
    expect(
      offendersOf(sheet, 'shared/wallet_sheet.dart'),
      isEmpty,
      reason:
          'open it with showWalletSheet: the 560 cap, the shape, the handle',
    );
  });

  test('S11 C6: no LinearProgressIndicator sets its own colours', () {
    final bar = RegExp(r'(?<![A-Za-z0-9_])LinearProgressIndicator\(');
    final colour = RegExp(
      r'(?<![A-Za-z0-9_])(color|backgroundColor|valueColor):',
    );
    const coloured =
        'LinearProgressIndicator(value: p, backgroundColor: c.bg, color: t)';
    expect(colour.hasMatch(argsOf(coloured, coloured.indexOf('('))), isTrue);
    var bars = 0;
    final offenders = <String>[];
    for (final f in dartFiles) {
      final code = codeOf(f);
      for (final m in bar.allMatches(code)) {
        bars++;
        if (colour.hasMatch(argsOf(code, m.end - 1))) {
          offenders.add(lineAt(f, code, m.start));
        }
      }
    }
    expect(bars, greaterThanOrEqualTo(2), reason: 'the two sync bars');
    expect(
      offenders,
      isEmpty,
      reason:
          'progressIndicatorTheme owns the bar (cyan on outline); the sync '
          'state rides the glyph beside it',
    );
  });

  test('the package bundles and names no font', () {
    final dmSans = RegExp(r'DM ?Sans');
    final offenders = [
      for (final f in dartFiles)
        if (dmSans.hasMatch(f.readAsStringSync())) rel(f),
    ];
    expect(offenders, isEmpty);
    final pubspec = File('pubspec.yaml').readAsLinesSync();
    expect(
      pubspec.where((l) => RegExp(r'^\s*fonts:').hasMatch(l)),
      isEmpty,
      reason: 'fonts are the host\'s (W-9)',
    );
  });
}
