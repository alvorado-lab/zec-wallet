import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/colors.dart';
import 'package:zec_wallet_ui/core/theme/shapes.dart';
import 'package:zec_wallet_ui/core/theme/sheet_layout.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/shared/wallet_sheet.dart';

/// S11 C4 (`DESIGN.md` §6.8): the one sheet entry, per the contract's §4
/// Sheet rows that hold at the helper (the per-site rows ride part B).
void main() {
  const launcherKey = ValueKey('launcher');
  const bodyKey = ValueKey('sheet-body');

  Widget host({
    TargetPlatform platform = TargetPlatform.android,
    ThemeData? theme,
  }) => MaterialApp(
    theme: (theme ?? buildTheme(WalletColors.light)).copyWith(
      platform: platform,
    ),
    home: const Scaffold(body: SizedBox(key: launcherKey)),
  );

  Future<T?> open<T>(
    WidgetTester tester, {
    WidgetBuilder? builder,
    bool isDismissible = true,
    bool enableDrag = true,
  }) => showWalletSheet<T>(
    tester.element(find.byKey(launcherKey)),
    isDismissible: isDismissible,
    enableDrag: enableDrag,
    builder:
        builder ??
        (_) => const SizedBox(
          key: bodyKey,
          width: double.infinity,
          height: 200,
          child: Text('body'),
        ),
  );

  /// The sheet's own surface: `BottomSheet` itself spans the window and
  /// centres its constrained `Material` inside. That surface is the widest
  /// `Material` under it (the drag handle and the iOS card are narrower).
  Rect sheetSurface(WidgetTester tester) {
    final rects = find
        .descendant(
          of: find.byType(BottomSheet),
          matching: find.byType(Material),
        )
        .evaluate()
        .map((e) => tester.getRect(find.byElementPredicate((x) => x == e)));
    return rects.reduce((a, b) => a.width >= b.width ? a : b);
  }

  void wideWindow(WidgetTester tester) {
    tester.view.physicalSize = const Size(1200, 800);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);
  }

  for (final platform in [TargetPlatform.android, TargetPlatform.iOS]) {
    testWidgets('${platform.name}: capped at $walletSheetMaxWidth on a wide '
        'window', (tester) async {
      wideWindow(tester);
      await tester.pumpWidget(host(platform: platform));
      open<void>(tester);
      await tester.pumpAndSettle();
      expect(sheetSurface(tester).width, walletSheetMaxWidth);
    });

    testWidgets('${platform.name}: returns the value the sheet pops', (
      tester,
    ) async {
      await tester.pumpWidget(host(platform: platform));
      final result = open<String>(
        tester,
        builder: (context) => TextButton(
          onPressed: () => Navigator.of(context).pop('picked'),
          child: const Text('pick'),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text('pick'));
      await tester.pumpAndSettle();
      expect(await result, 'picked');
    });

    testWidgets('${platform.name}: isDismissible false keeps it open on a '
        'barrier tap', (tester) async {
      await tester.pumpWidget(host(platform: platform));
      open<void>(tester, isDismissible: false, enableDrag: false);
      await tester.pumpAndSettle();
      await tester.tapAt(const Offset(10, 10));
      await tester.pumpAndSettle();
      expect(find.byKey(bodyKey), findsOneWidget);
    });

    testWidgets('${platform.name}: a dismissible sheet closes on a barrier '
        'tap, with null', (tester) async {
      await tester.pumpWidget(host(platform: platform));
      final result = open<String>(tester);
      await tester.pumpAndSettle();
      await tester.tapAt(const Offset(10, 10));
      await tester.pumpAndSettle();
      expect(find.byKey(bodyKey), findsNothing);
      expect(await result, isNull);
    });
  }

  group('Android', () {
    testWidgets('a drag handle, and top corners at WalletShapes.sheet', (
      tester,
    ) async {
      await tester.pumpWidget(
        host(
          theme: buildTheme(
            WalletColors.light,
            extensions: const [WalletShapes(sheet: 11)],
          ),
        ),
      );
      open<void>(tester);
      await tester.pumpAndSettle();
      final sheet = tester.widget<BottomSheet>(find.byType(BottomSheet));
      expect(sheet.showDragHandle, isTrue);
      expect(
        sheet.shape,
        const RoundedRectangleBorder(
          borderRadius: BorderRadius.vertical(top: Radius.circular(11)),
        ),
      );
      expect(find.byKey(walletSheetGrabberKey), findsNothing);
    });

    testWidgets('no drag handle when the sheet cannot be dragged', (
      tester,
    ) async {
      await tester.pumpWidget(host());
      open<void>(tester, enableDrag: false);
      await tester.pumpAndSettle();
      expect(
        tester.widget<BottomSheet>(find.byType(BottomSheet)).showDragHandle,
        isFalse,
      );
    });
  });

  group('iOS', () {
    // The variant makes `defaultTargetPlatform` iOS, as on a device, so
    // `buildTheme` registers the iOS shapes (a 38 sheet).
    testWidgets('inset 8 on three sides, all corners round, with a 36x5 '
        'grabber and no Material handle', (tester) async {
      await tester.pumpWidget(host(platform: TargetPlatform.iOS));
      open<void>(tester);
      await tester.pumpAndSettle();

      final sheet = tester.widget<BottomSheet>(find.byType(BottomSheet));
      expect(sheet.showDragHandle, isFalse);

      final route = sheetSurface(tester);
      final card = find.ancestor(
        of: find.byKey(bodyKey),
        matching: find.byType(Material),
      );
      final cardRect = tester.getRect(card.first);
      expect(cardRect.left - route.left, 8);
      expect(route.right - cardRect.right, 8);
      expect(route.bottom - cardRect.bottom, 8);
      expect(
        tester.widget<Material>(card.first).shape,
        const RoundedRectangleBorder(
          borderRadius: BorderRadius.all(Radius.circular(38)),
        ),
      );

      expect(
        tester.getSize(find.byKey(walletSheetGrabberKey)),
        const Size(36, 5),
      );
      final decoration =
          tester
                  .widget<DecoratedBox>(
                    find.descendant(
                      of: find.byKey(walletSheetGrabberKey),
                      matching: find.byType(DecoratedBox),
                    ),
                  )
                  .decoration
              as ShapeDecoration;
      expect(decoration.color, WalletColors.light.toggleOff);
    }, variant: TargetPlatformVariant.only(TargetPlatform.iOS));

    testWidgets('no grabber when the sheet cannot be dragged', (tester) async {
      await tester.pumpWidget(host(platform: TargetPlatform.iOS));
      open<void>(tester, enableDrag: false);
      await tester.pumpAndSettle();
      expect(find.byKey(walletSheetGrabberKey), findsNothing);
      expect(find.byKey(bodyKey), findsOneWidget);
    });
  });
}
