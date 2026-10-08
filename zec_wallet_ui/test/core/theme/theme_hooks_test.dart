import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qr_flutter/qr_flutter.dart';
import 'package:zec_wallet_ui/core/theme/colors.dart';
import 'package:zec_wallet_ui/core/theme/icons.dart';
import 'package:zec_wallet_ui/core/theme/shapes.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/core/theme/typography.dart';
import 'package:zec_wallet_ui/shared/address_text.dart';
import 'package:zec_wallet_ui/shared/qr_tile.dart';

/// FR-49 (ADR-0564): what a host registers reaches what the SDK draws, and
/// what it does not register falls back to the SDK's defaults.
void main() {
  Widget host(ThemeData theme, Widget child) => MaterialApp(
    theme: theme,
    home: Scaffold(body: Center(child: child)),
  );

  // zcash_address's own test vector (encoding.rs), not a wallet of ours.
  const addr = 't1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs';

  List<TextStyle> addressSpanStyles(WidgetTester tester) => [
    for (final span
        in tester
            .widget<SelectableText>(find.byType(SelectableText))
            .textSpan!
            .children!)
      (span as TextSpan).style!,
  ];

  group('WalletTypography (W-2)', () {
    testWidgets('a registered mono face sets an address, and keeps its sizes', (
      tester,
    ) async {
      const mono = WalletTypography(
        mono: TextStyle(fontFamily: 'HostMono', fontFamilyFallback: ['Menlo']),
      );
      await tester.pumpWidget(
        host(
          buildTheme(WalletColors.light, extensions: const [mono]),
          const AddressText(addr),
        ),
      );
      for (final style in addressSpanStyles(tester)) {
        expect(style.fontFamily, 'HostMono');
        expect(style.fontFamilyFallback, ['Menlo']);
        // The site's own size and spacing survive (monoOn carries the face
        // only).
        expect(style.letterSpacing, 0.3);
        expect(
          style.fontSize,
          walletTextScale(WalletColors.light).bodyLarge!.fontSize,
        );
      }
    });

    testWidgets('with none registered an address keeps the generic mono', (
      tester,
    ) async {
      final bare = buildTheme(
        WalletColors.light,
      ).copyWith(extensions: [WalletColors.light]);
      expect(bare.extension<WalletTypography>(), isNull);
      await tester.pumpWidget(host(bare, const AddressText(addr)));
      for (final style in addressSpanStyles(tester)) {
        expect(style.fontFamily, 'monospace');
      }
    });
  });

  group('WalletIcons (W-5)', () {
    testWidgets(
      'a registered builder draws the glyphs it maps; the rest stay Material',
      (tester) async {
        // What the builder was asked for, per glyph: the host's renderer must
        // receive the size and colour the call site passed, or a painted
        // glyph cannot match the Material one it replaces.
        final asked = <WalletGlyph, (double?, Color?)>{};
        Widget? builder(
          BuildContext context,
          WalletGlyph glyph, {
          double? size,
          Color? color,
          String? semanticLabel,
        }) {
          asked[glyph] = (size, color);
          if (glyph != WalletGlyph.send) return null;
          // Deliberately NOT an Icon: the host's glyphs are painted, not
          // IconData, and the seam must carry any widget.
          return SizedBox(
            key: ValueKey('host-glyph-${glyph.name}'),
            width: size,
            height: size,
          );
        }

        const tint = Color(0xFF0F3D27);
        await tester.pumpWidget(
          host(
            buildTheme(
              WalletColors.light,
              extensions: [WalletIcons(builder: builder)],
            ),
            const Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                WalletIcon(WalletGlyph.send, size: 20, color: tint),
                WalletIcon(WalletGlyph.receive),
              ],
            ),
          ),
        );

        expect(find.byKey(const ValueKey('host-glyph-send')), findsOneWidget);
        expect(find.byIcon(WalletGlyph.send.material), findsNothing);
        expect(asked[WalletGlyph.send], (20.0, tint));

        // The unmapped glyph falls through to its Material default — the only
        // Icon on screen.
        final receive = tester.widget<Icon>(find.byType(Icon));
        expect(receive.icon, WalletGlyph.receive.material);
        expect(asked[WalletGlyph.receive], (null, null));
      },
    );

    testWidgets(
      'with none registered a glyph is its Material default at the ambient '
      'IconTheme size',
      (tester) async {
        await tester.pumpWidget(
          host(
            ThemeData(extensions: [WalletColors.light]),
            const IconTheme(
              data: IconThemeData(size: 31),
              child: WalletIcon(WalletGlyph.send),
            ),
          ),
        );
        final icon = tester.widget<Icon>(find.byType(Icon));
        expect(icon.icon, Icons.arrow_upward);
        expect(tester.getSize(find.byType(Icon)), const Size(31, 31));
      },
    );

    testWidgets('with none registered the Material set is used', (
      tester,
    ) async {
      late WalletIcons seen;
      await tester.pumpWidget(
        host(
          ThemeData(extensions: [WalletColors.light]),
          Builder(
            builder: (context) {
              seen = WalletIcons.of(context);
              return const SizedBox();
            },
          ),
        ),
      );
      expect(identical(seen, WalletIcons.material), isTrue);
    });
  });

  group('qrInk (W-10)', () {
    Future<Color> moduleColor(WidgetTester tester, WalletColors colors) async {
      await tester.pumpWidget(
        host(buildTheme(colors), const QrTile(payload: addr, label: 'QR')),
      );
      final qr = tester.widget<QrImageView>(find.byType(QrImageView));
      expect(qr.eyeStyle.color, qr.dataModuleStyle.color);
      return qr.dataModuleStyle.color!;
    }

    testWidgets('the default ink is black', (tester) async {
      expect(
        await moduleColor(tester, WalletColors.light),
        const Color(0xFF000000),
      );
    });

    testWidgets('a dark host ink is drawn', (tester) async {
      const ink = Color(0xFF0F3D27); // 12.6:1 on white
      expect(
        await moduleColor(tester, WalletColors.light.copyWith(qrInk: ink)),
        ink,
      );
    });

    testWidgets('an ink too pale to scan falls back to black', (tester) async {
      const pale = Color(0xFF3CC47F); // the dark accent: well under 7:1
      expect(
        await moduleColor(tester, WalletColors.light.copyWith(qrInk: pale)),
        const Color(0xFF000000),
      );
    });

    test('a translucent ink falls back to black', () {
      expect(qrModuleColor(const Color(0x80000000)), const Color(0xFF000000));
    });
  });

  group('buildTheme (W-3, W-9)', () {
    test('names no font family by default and defines every slot it reads', () {
      // The SDK's scale names no family; ThemeData then fills in the
      // platform's own default face, exactly as for a theme with no fonts.
      final platformDefault = ThemeData().textTheme.bodyLarge!.fontFamily;
      final scale = walletTextScale(WalletColors.dark);
      for (final style in [
        scale.displaySmall,
        scale.headlineSmall,
        scale.bodyLarge,
        scale.labelMedium,
      ]) {
        expect(style!.fontFamily, isNull);
      }
      final t = buildTheme(WalletColors.dark).textTheme;
      for (final style in [
        t.displaySmall,
        t.headlineLarge,
        t.headlineMedium,
        t.headlineSmall,
        t.titleLarge,
        t.titleMedium,
        t.titleSmall,
        t.bodyLarge,
        t.bodyMedium,
        t.bodySmall,
        t.labelLarge,
        t.labelMedium,
        t.labelSmall,
      ]) {
        expect(style, isNotNull);
        expect(style!.fontSize, isNotNull);
        expect(style.fontFamily, platformDefault);
      }
    });

    test('a host text theme sets families and keeps the SDK sizes', () {
      final t = buildTheme(
        WalletColors.light,
        textTheme: const TextTheme(
          headlineSmall: TextStyle(fontFamily: 'HostTitle'),
          bodyLarge: TextStyle(fontFamily: 'HostUi'),
        ),
      ).textTheme;
      final scale = walletTextScale(WalletColors.light);
      expect(t.headlineSmall!.fontFamily, 'HostTitle');
      expect(t.headlineSmall!.fontSize, scale.headlineSmall!.fontSize);
      expect(t.bodyLarge!.fontFamily, 'HostUi');
      expect(t.bodyLarge!.fontSize, scale.bodyLarge!.fontSize);
      expect(t.bodyLarge!.color, scale.bodyLarge!.color);
    });

    test('WalletColors is always the colors argument', () {
      final theme = buildTheme(
        WalletColors.dark,
        extensions: [WalletColors.light],
      );
      expect(theme.extension<WalletColors>(), WalletColors.dark);
      expect(theme.extension<WalletIcons>(), WalletIcons.material);
      expect(theme.extension<WalletTypography>(), WalletTypography.fallback);
    });
  });

  group('component themes (S11 C5, spec §3.4)', () {
    const colors = WalletColors.light;
    final theme = buildTheme(colors);
    final shapes = theme.extension<WalletShapes>()!;
    final outline = theme.colorScheme.outline;
    const none = <WidgetState>{};

    RoundedRectangleBorder rounded(double r) =>
        RoundedRectangleBorder(borderRadius: BorderRadius.circular(r));

    test('every button kind is a pill at least 48 high in labelLarge', () {
      for (final (name, style) in [
        ('filled', theme.filledButtonTheme.style),
        ('outlined', theme.outlinedButtonTheme.style),
        ('text', theme.textButtonTheme.style),
      ]) {
        expect(style, isNotNull, reason: name);
        expect(
          style!.shape!.resolve(none),
          const StadiumBorder(),
          reason: name,
        );
        expect(
          style.minimumSize!.resolve(none)!.height,
          greaterThanOrEqualTo(48),
          reason: name,
        );
        expect(
          style.textStyle!.resolve(none)!.fontSize,
          theme.textTheme.labelLarge!.fontSize,
          reason: name,
        );
      }
      final side = theme.outlinedButtonTheme.style!.side!.resolve(none)!;
      expect(side.color, outline);
      expect(side.width, 1.5);
      expect(
        theme.iconButtonTheme.style!.minimumSize!.resolve(none),
        const Size(44, 44),
      );
    });

    testWidgets('a 48 minimum survives on every button kind as drawn', (
      tester,
    ) async {
      await tester.pumpWidget(
        host(
          theme,
          Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              FilledButton(onPressed: () {}, child: const Text('a')),
              OutlinedButton(onPressed: () {}, child: const Text('b')),
              TextButton(onPressed: () {}, child: const Text('c')),
            ],
          ),
        ),
      );
      for (final type in [FilledButton, OutlinedButton, TextButton]) {
        // The visual button, not the padded tap target around it.
        final ink = find.descendant(
          of: find.byType(type),
          matching: find.byType(Material),
        );
        expect(
          tester.getSize(ink.first).height,
          greaterThanOrEqualTo(48),
          reason: '$type',
        );
      }
    });

    test('fields are filled bgCard pills cut to WalletShapes.field', () {
      final input = theme.inputDecorationTheme;
      expect(input.filled, isTrue);
      expect(input.fillColor, colors.bgCard);
      final enabled = input.enabledBorder! as OutlineInputBorder;
      expect(enabled.borderRadius, BorderRadius.circular(shapes.field));
      expect(enabled.borderSide, BorderSide(color: outline));
      final focused = input.focusedBorder! as OutlineInputBorder;
      expect(focused.borderSide, BorderSide(color: colors.accent, width: 2));
      expect(
        (input.errorBorder! as OutlineInputBorder).borderSide.color,
        colors.red,
      );
      expect(
        (input.focusedErrorBorder! as OutlineInputBorder).borderSide.color,
        colors.red,
      );
      expect(
        (input.disabledBorder! as OutlineInputBorder).borderSide.color,
        colors.border,
      );
    });

    test('a host WalletShapes cuts the field, the card and the sheet', () {
      final hosted = buildTheme(
        colors,
        extensions: const [WalletShapes(field: 9, group: 13, sheet: 17)],
      );
      expect(
        (hosted.inputDecorationTheme.border! as OutlineInputBorder)
            .borderRadius,
        BorderRadius.circular(9),
      );
      expect(hosted.cardTheme.shape, rounded(13));
      expect(
        hosted.bottomSheetTheme.shape,
        const RoundedRectangleBorder(
          borderRadius: BorderRadius.vertical(top: Radius.circular(17)),
        ),
      );
    });

    test('sheets, dialogs and cards', () {
      final sheet = theme.bottomSheetTheme;
      expect(sheet.backgroundColor, colors.bgCard);
      expect(sheet.modalBackgroundColor, colors.bgCard);
      expect(sheet.constraints, const BoxConstraints(maxWidth: 560));
      expect(
        sheet.shape,
        RoundedRectangleBorder(
          borderRadius: BorderRadius.vertical(
            top: Radius.circular(shapes.sheet),
          ),
        ),
      );
      expect(theme.dialogTheme.backgroundColor, colors.bgCard);
      expect(theme.dialogTheme.shape, rounded(28));
      expect(theme.cardTheme.shape, rounded(shapes.group));
    });

    test('chips and segmented buttons', () {
      final chip = theme.chipTheme;
      expect(chip.backgroundColor, Colors.transparent);
      expect(chip.selectedColor, colors.accentSoft);
      expect(chip.side, BorderSide(color: outline));
      expect(chip.shape, const StadiumBorder());
      expect(chip.checkmarkColor, colors.accentText);

      final seg = theme.segmentedButtonTheme.style!;
      const selected = {WidgetState.selected};
      expect(seg.backgroundColor!.resolve(none), colors.bgHover);
      expect(seg.backgroundColor!.resolve(selected), colors.accentSoft);
      expect(seg.foregroundColor!.resolve(selected), colors.accentText);
      expect(seg.side!.resolve(none), BorderSide.none);
      expect(seg.shape!.resolve(none), const StadiumBorder());
    });

    test('the progress bar is cyan on outline, 6 high', () {
      final p = theme.progressIndicatorTheme;
      expect(p.color, colors.cyan);
      expect(p.linearTrackColor, outline);
      expect(p.linearMinHeight, 6);
      expect(p.borderRadius, BorderRadius.circular(shapes.progress));
    });

    test('toggles, snack bars and list rows', () {
      const selected = {WidgetState.selected};
      expect(theme.switchTheme.trackColor!.resolve(selected), colors.accent);
      expect(theme.switchTheme.trackColor!.resolve(none), colors.bgHover);
      expect(theme.switchTheme.thumbColor!.resolve(selected), colors.onAccent);
      expect(theme.radioTheme.fillColor!.resolve(selected), colors.accent);
      expect(theme.checkboxTheme.fillColor!.resolve(selected), colors.accent);
      expect(
        theme.checkboxTheme.checkColor!.resolve(selected),
        colors.onAccent,
      );

      final snack = theme.snackBarTheme;
      expect(snack.behavior, SnackBarBehavior.floating);
      expect(snack.backgroundColor, colors.text);
      expect(snack.contentTextStyle!.color, colors.bg);
      expect(snack.shape, const StadiumBorder());

      expect(theme.listTileTheme.minTileHeight, 52);
    });
  });
}
