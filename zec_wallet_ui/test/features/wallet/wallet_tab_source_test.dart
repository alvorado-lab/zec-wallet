import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
// The BARREL only (C6: a host overrides the hide-balance provider through the
// public API, never an internal path).
import 'package:zec_wallet_ui/zec_wallet_ui.dart';

/// Stage S12 (`docs/plan/stage-12-the-wallet-tab.md` §3): the source test
/// (no colour literal in the files this stage restyled — every colour is a
/// `WalletColors` role, so a host theme reaches all of it) and the C6 export.

/// A colour literal: `Color(0x…)`, the `Color.fromARGB`/`fromRGBO`
/// constructors, or the Material `Colors.` palette. The lookbehind keeps
/// `WalletColors.` (the role accessor) from matching.
final _colorLiteral = RegExp(
  r'Color\(0x|Color\.from(?:ARGB|RGBO)\(|(?<![A-Za-z])Colors\.',
);

/// The files S12 restyled whose colours must all be roles.
const _files = [
  'lib/features/wallet/wallet_screen.dart',
  'lib/features/wallet/wallet_coin.dart',
];

void main() {
  group('the pattern is not vacuous', () {
    for (final planted in [
      'color: const Color(0xFFFFFFFF),',
      'color: Color(0x59FFFFFF)',
      'color: Colors.white,',
      '(Colors.black)',
      ' Colors.transparent',
      'Color.fromARGB(255, 0, 0, 0)',
      'const Color.fromRGBO(1, 2, 3, 1)',
    ]) {
      test('matches a planted literal: $planted', () {
        expect(_colorLiteral.hasMatch(planted), isTrue);
      });
    }
    for (final role in [
      'WalletColors.of(context)',
      'final colors = WalletColors.of(context);',
      'colors.accentSoft',
      'Color.lerp(a, b, 0.5)',
      'final Color color;',
    ]) {
      test('does not match a role or a type: $role', () {
        expect(_colorLiteral.hasMatch(role), isFalse);
      });
    }
  });

  for (final path in _files) {
    test('$path has no Color(0x…) or Colors. literal', () {
      final file = File(path);
      expect(file.existsSync(), isTrue, reason: 'run from the package root');
      final lines = file.readAsLinesSync();
      expect(lines, isNotEmpty);
      final hits = [
        for (var i = 0; i < lines.length; i++)
          if (_colorLiteral.hasMatch(lines[i])) '${i + 1}: ${lines[i].trim()}',
      ];
      expect(hits, isEmpty, reason: 'colour literals in $path');
    });
  }

  test('the barrel exports the hide-balance API (C6: a host may override '
      'the provider to persist the choice)', () {
    // Each symbol resolves through the barrel import alone — a missing
    // export is a compile error in this file.
    final container = ProviderContainer(
      overrides: [
        walletBalanceHiddenProvider.overrideWith(_PersistedHidden.new),
      ],
    );
    addTearDown(container.dispose);
    expect(container.read(walletBalanceHiddenProvider), isTrue);
    container.read(walletBalanceHiddenProvider.notifier).toggle();
    expect(container.read(walletBalanceHiddenProvider), isFalse);
    expect(maskedAmountText, '••••');
    expect(displayedAmount('1 ZEC', hidden: true), maskedAmountText);
    expect(displayedAmount('1 ZEC', hidden: false), '1 ZEC');
  });
}

/// A host's persisted choice, restored as "hidden".
class _PersistedHidden extends WalletBalanceHidden {
  @override
  bool build() => true;
}
