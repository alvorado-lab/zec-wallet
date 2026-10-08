import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/colors.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_address_scanner.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_ui_config.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';

/// S13 §1.5 — the scanner reads its stage from two theme roles, its manual
/// escape is legible in both themes, and a refused camera offers "Open
/// settings" only through a host hook.
void main() {
  Widget launcher({
    required ThemeData theme,
    List<Override> overrides = const [],
    bool refused = false,
  }) => ProviderScope(
    overrides: [
      addressScannerCameraRefusedProvider.overrideWithValue(refused),
      ...overrides,
    ],
    child: MaterialApp(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      theme: theme,
      home: Scaffold(
        body: Builder(
          builder: (context) => Center(
            child: TextButton(
              onPressed: () => unawaited(showAddressScanner(context)),
              child: const Text('open'),
            ),
          ),
        ),
      ),
    ),
  );

  Future<void> open(WidgetTester tester) async {
    await tester.tap(find.text('open'));
    await tester.pump();
    await tester.pump(const Duration(seconds: 1));
  }

  WalletLocalizations l10n(WidgetTester tester) => WalletLocalizations.of(
    tester.element(find.byKey(const Key('scanner-manual-entry'))),
  );

  double contrast(Color a, Color b) {
    final la = a.computeLuminance();
    final lb = b.computeLuminance();
    final hi = la > lb ? la : lb;
    final lo = la > lb ? lb : la;
    return (hi + 0.05) / (lo + 0.05);
  }

  for (final (name, theme) in [('light', lightTheme), ('dark', darkTheme)]) {
    testWidgets('"Enter manually" is ≥ 4.5:1 on the stage in the $name '
        'theme', (tester) async {
      await tester.pumpWidget(launcher(theme: theme));
      await open(tester);
      final manual = find.byKey(const Key('scanner-manual-entry'));
      final text = tester.widget<Text>(
        find.descendant(
          of: manual,
          matching: find.text(l10n(tester).walletSwapScanManualEntry),
        ),
      );
      final fg = DefaultTextStyle.of(
        tester.element(find.byWidget(text)),
      ).style.color!;
      final stage = tester
          .widget<Scaffold>(
            find.ancestor(of: manual, matching: find.byType(Scaffold)).first,
          )
          .backgroundColor!;
      final colors = theme.extension<WalletColors>()!;
      expect(stage, colors.stage, reason: 'the stage role, not a literal');
      expect(contrast(fg, stage), greaterThanOrEqualTo(4.5));
    });
  }

  testWidgets('a host stage colour reaches the scanner', (tester) async {
    final colors = WalletColors.light.copyWith(
      stage: const Color(0xFF102030),
      onStage: const Color(0xFFFFF0E0),
    );
    await tester.pumpWidget(
      launcher(theme: lightTheme.copyWith(extensions: [colors])),
    );
    await open(tester);
    final scaffold = tester.widget<Scaffold>(
      find
          .ancestor(
            of: find.byKey(const Key('scanner-manual-entry')),
            matching: find.byType(Scaffold),
          )
          .first,
    );
    expect(scaffold.backgroundColor, const Color(0xFF102030));
  });

  testWidgets('the close control is a 44 target at the top-leading corner', (
    tester,
  ) async {
    await tester.pumpWidget(launcher(theme: lightTheme));
    await open(tester);
    final close = find.byKey(const Key('scanner-close'));
    expect(tester.getSize(close), const Size(44, 44));
    expect(tester.getTopLeft(close).dx, lessThan(40));
  });

  group('camera refused → Open settings (host hook)', () {
    final openSettings = find.byKey(const Key('scanner-open-settings'));

    testWidgets('no hook, no button', (tester) async {
      await tester.pumpWidget(launcher(theme: lightTheme, refused: true));
      await open(tester);
      expect(
        find.text(l10n(tester).walletSwapScanCameraUnavailable),
        findsOneWidget,
      );
      expect(openSettings, findsNothing);
    });

    testWidgets('a hook draws the button and a tap calls it', (tester) async {
      var calls = 0;
      await tester.pumpWidget(
        launcher(
          theme: lightTheme,
          refused: true,
          overrides: [
            walletUiConfigProvider.overrideWithValue(
              WalletUiConfig(onOpenSettings: () async => calls++),
            ),
          ],
        ),
      );
      await open(tester);
      expect(openSettings, findsOneWidget);
      await tester.tap(openSettings);
      await tester.pump();
      expect(calls, 1);
      expect(
        find.byKey(const Key('scanner-open-settings-failed')),
        findsNothing,
      );
    });

    testWidgets('a hook that throws becomes an inline fault, not a crash', (
      tester,
    ) async {
      await tester.pumpWidget(
        launcher(
          theme: lightTheme,
          refused: true,
          overrides: [
            walletUiConfigProvider.overrideWithValue(
              WalletUiConfig(
                onOpenSettings: () async => throw StateError('host'),
              ),
            ),
          ],
        ),
      );
      await open(tester);
      await tester.tap(openSettings);
      await tester.pump();
      expect(tester.takeException(), isNull);
      expect(
        find.byKey(const Key('scanner-open-settings-failed')),
        findsOneWidget,
      );
    });
  });
}
