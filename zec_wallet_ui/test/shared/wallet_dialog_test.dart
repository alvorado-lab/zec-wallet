import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart' show LargeSendReason;
import 'package:zec_wallet_ui/core/theme/colors.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/send/large_send_confirm.dart';
import 'package:zec_wallet_ui/features/wallet/zat_format.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/shared/wallet_dialog.dart';

/// S11 C3 (`DESIGN.md` §6.9): the one confirm helper, per the contract's §4
/// Dialogs rows that hold at the helper (the per-site rows ride part B).
void main() {
  const colors = WalletColors.light;
  const title = 'Delete this wallet?';
  const body = 'This removes the wallet from this device.';
  const confirm = 'Delete';
  const cancel = 'Cancel';
  const launcherKey = ValueKey('launcher');

  Widget host({
    TargetPlatform platform = TargetPlatform.android,
    ThemeData? theme,
    double textScale = 1.0,
    Widget? extra,
  }) => MaterialApp(
    theme: (theme ?? buildTheme(colors)).copyWith(platform: platform),
    builder: (context, app) => MediaQuery.withClampedTextScaling(
      minScaleFactor: textScale,
      maxScaleFactor: textScale,
      child: app!,
    ),
    home: Scaffold(
      body: Column(
        children: [
          const SizedBox(key: launcherKey),
          ?extra,
        ],
      ),
    ),
  );

  /// Open the dialog from the launcher's context and hand back its future.
  Future<bool> open(
    WidgetTester tester, {
    WalletConfirmKind kind = WalletConfirmKind.destructive,
    String bodyText = body,
  }) {
    final future = showWalletConfirm(
      tester.element(find.byKey(launcherKey)),
      title: title,
      body: bodyText,
      confirmLabel: confirm,
      cancelLabel: kind == WalletConfirmKind.info ? null : cancel,
      kind: kind,
    );
    return future;
  }

  CupertinoDialogAction cupertinoAction(WidgetTester tester, String label) =>
      tester.widget<CupertinoDialogAction>(
        find.widgetWithText(CupertinoDialogAction, label),
      );

  group('iOS', () {
    testWidgets('renders a Cupertino alert; each kind sets its own flags', (
      tester,
    ) async {
      const flags = {
        // kind: (isDefaultAction, isDestructiveAction)
        WalletConfirmKind.forward: (true, false),
        WalletConfirmKind.destructive: (false, true),
        WalletConfirmKind.neutral: (false, false),
      };
      for (final entry in flags.entries) {
        await tester.pumpWidget(host(platform: TargetPlatform.iOS));
        await tester.pumpAndSettle();
        final result = open(tester, kind: entry.key);
        await tester.pumpAndSettle();

        expect(find.byType(CupertinoAlertDialog), findsOneWidget);
        expect(find.byType(FilledButton), findsNothing);
        final action = cupertinoAction(tester, confirm);
        expect(
          (action.isDefaultAction, action.isDestructiveAction),
          entry.value,
          reason: '${entry.key}',
        );
        final cancelAction = cupertinoAction(tester, cancel);
        expect(cancelAction.isDefaultAction, isFalse);
        expect(cancelAction.isDestructiveAction, isFalse);

        await tester.tap(find.text(cancel));
        await tester.pumpAndSettle();
        expect(await result, isFalse);
      }
    });

    testWidgets('info has one action, and it consents to nothing', (
      tester,
    ) async {
      await tester.pumpWidget(host(platform: TargetPlatform.iOS));
      final result = open(tester, kind: WalletConfirmKind.info);
      await tester.pumpAndSettle();
      expect(find.byType(CupertinoDialogAction), findsOneWidget);
      await tester.tap(find.text(confirm));
      await tester.pumpAndSettle();
      expect(await result, isFalse);
    });

    testWidgets('a barrier tap leaves the dialog open; Cancel is false', (
      tester,
    ) async {
      await tester.pumpWidget(host(platform: TargetPlatform.iOS));
      final result = open(tester);
      await tester.pumpAndSettle();

      await tester.tapAt(const Offset(4, 4));
      await tester.pumpAndSettle();
      expect(find.byType(CupertinoAlertDialog), findsOneWidget);

      await tester.tap(find.text(cancel));
      await tester.pumpAndSettle();
      expect(find.byType(CupertinoAlertDialog), findsNothing);
      expect(await result, isFalse);
    });

    testWidgets('the confirm tap is true', (tester) async {
      await tester.pumpWidget(host(platform: TargetPlatform.iOS));
      final result = open(tester, kind: WalletConfirmKind.forward);
      await tester.pumpAndSettle();
      await tester.tap(find.text(confirm));
      await tester.pumpAndSettle();
      expect(await result, isTrue);
    });
  });

  group('Android', () {
    testWidgets('forward is a FilledButton beside a text Cancel', (
      tester,
    ) async {
      await tester.pumpWidget(host());
      final result = open(tester, kind: WalletConfirmKind.forward);
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsOneWidget);
      expect(find.byType(CupertinoAlertDialog), findsNothing);
      expect(find.widgetWithText(FilledButton, confirm), findsOneWidget);
      expect(find.widgetWithText(TextButton, cancel), findsOneWidget);
      await tester.tap(find.text(confirm));
      await tester.pumpAndSettle();
      expect(await result, isTrue);
    });

    testWidgets('neutral is two text buttons', (tester) async {
      await tester.pumpWidget(host());
      final result = open(tester, kind: WalletConfirmKind.neutral);
      await tester.pumpAndSettle();
      expect(find.byType(FilledButton), findsNothing);
      expect(find.widgetWithText(TextButton, confirm), findsOneWidget);
      expect(find.widgetWithText(TextButton, cancel), findsOneWidget);
      await tester.tap(find.text(cancel));
      await tester.pumpAndSettle();
      expect(await result, isFalse);
    });

    testWidgets('info is one text button and is false', (tester) async {
      await tester.pumpWidget(host());
      final result = open(tester, kind: WalletConfirmKind.info);
      await tester.pumpAndSettle();
      expect(find.byType(TextButton), findsOneWidget);
      expect(find.byType(FilledButton), findsNothing);
      await tester.tap(find.text(confirm));
      await tester.pumpAndSettle();
      expect(await result, isFalse);
    });

    Color? renderedColor(WidgetTester tester, String label) => tester
        .renderObject<RenderParagraph>(find.text(label))
        .text
        .style
        ?.color;

    testWidgets('destructive is red text, never a FilledButton', (
      tester,
    ) async {
      await tester.pumpWidget(host());
      open(tester);
      await tester.pumpAndSettle();
      expect(find.byType(FilledButton), findsNothing);
      expect(find.widgetWithText(TextButton, confirm), findsOneWidget);
      expect(renderedColor(tester, confirm), colors.red);
      // Cancel is not red.
      expect(renderedColor(tester, cancel), isNot(colors.red));
    });

    testWidgets(
      'destructive stays red under a host whose text buttons are blue',
      (tester) async {
        const blue = Color(0xFF0000FF);
        final hostTheme = buildTheme(colors).copyWith(
          textButtonTheme: TextButtonThemeData(
            style: TextButton.styleFrom(foregroundColor: blue),
          ),
        );
        await tester.pumpWidget(host(theme: hostTheme));
        open(tester);
        await tester.pumpAndSettle();
        expect(renderedColor(tester, confirm), colors.red);
        // The host's theme does reach the other button: the test can see it.
        expect(renderedColor(tester, cancel), blue);
      },
    );

    testWidgets('a barrier tap is false, never null', (tester) async {
      await tester.pumpWidget(host());
      final result = open(tester);
      await tester.pumpAndSettle();
      await tester.tapAt(const Offset(4, 4));
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsNothing);
      expect(await result, isFalse);
    });

    testWidgets('a back pop is false, never null', (tester) async {
      await tester.pumpWidget(host());
      final result = open(tester);
      await tester.pumpAndSettle();
      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsNothing);
      expect(await result, isFalse);
    });

    testWidgets('the Material dialog is always scrollable: a long body at '
        '2.0x on a 320 dp phone keeps the confirm reachable', (tester) async {
      tester.view.physicalSize = const Size(320, 568);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);
      final longBody = List.filled(
        12,
        'Starting over erases this wallet from the device, and any funds '
        'it holds are lost unless you saved its recovery phrase.',
      ).join('\n\n');

      for (final kind in [
        WalletConfirmKind.destructive,
        WalletConfirmKind.forward,
      ]) {
        await tester.pumpWidget(host(textScale: 2.0));
        await tester.pumpAndSettle();
        final result = open(tester, kind: kind, bodyText: longBody);
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull, reason: '$kind');
        expect(
          tester.widget<AlertDialog>(find.byType(AlertDialog)).scrollable,
          isTrue,
        );

        // The body is laid out WHOLE (a non-scrolling dialog squeezes it into
        // the space left and clips the rest silently, with no exception)...
        final paragraph = tester.renderObject<RenderParagraph>(
          find.text(longBody),
        );
        expect(
          paragraph.size.height,
          greaterThanOrEqualTo(
            paragraph.getMaxIntrinsicHeight(paragraph.size.width) - 0.5,
          ),
          reason: '$kind: the body was clipped',
        );

        // ...its end is reachable by scrolling, and the confirm is on screen
        // and takes the tap.
        final scrollable = find.descendant(
          of: find.byType(AlertDialog),
          matching: find.byType(Scrollable),
        );
        await tester.drag(scrollable.first, const Offset(0, -20000));
        await tester.pumpAndSettle();
        final dialogBox = tester.getRect(find.byType(Dialog));
        expect(
          tester.getBottomLeft(find.text(longBody)).dy,
          lessThanOrEqualTo(dialogBox.bottom),
          reason: '$kind: the end of the body stayed off screen',
        );
        await tester.tap(find.text(confirm));
        await tester.pumpAndSettle();
        expect(await result, isTrue, reason: '$kind');
      }
    });
  });

  group(
    'the large-send confirm label is never truncated (the review\'s L3)',
    () {
      // Below 1.4x a Cupertino action sets its label to one line with an
      // ellipsis; the large-send confirm carries the exact amount in that label.
      // The longest locale's label (uk / ru / nl tie at 31 characters with this
      // amount) and the longest amount string the supply allows.
      const largestZat = 2099999999999999; // "20999999.99999999"
      for (final (platform, scale) in [
        (TargetPlatform.iOS, 1.0),
        (TargetPlatform.iOS, 1.3),
        (TargetPlatform.iOS, 2.0),
        (TargetPlatform.android, 1.0),
        (TargetPlatform.android, 2.0),
      ]) {
        testWidgets('${platform.name} at ${scale}x', (tester) async {
          await tester.pumpWidget(
            MaterialApp(
              locale: const Locale('uk'),
              localizationsDelegates:
                  WalletLocalizations.localizationsDelegates,
              supportedLocales: WalletLocalizations.supportedLocales,
              theme: buildTheme(colors).copyWith(platform: platform),
              builder: (context, app) => MediaQuery.withClampedTextScaling(
                minScaleFactor: scale,
                maxScaleFactor: scale,
                child: app!,
              ),
              home: const Scaffold(body: SizedBox(key: launcherKey)),
            ),
          );
          await tester.pumpAndSettle();
          final context = tester.element(find.byKey(launcherKey));
          final l10n = WalletLocalizations.of(context);
          final result = showLargeSendConfirm(
            context,
            totalZat: largestZat,
            reason: LargeSendReason.both,
          );
          await tester.pumpAndSettle();

          final label = l10n.walletSendLargeConfirmAction(
            l10n.walletAmount(formatZec(largestZat)),
          );
          expect(label, contains('20999999.99999999'));
          final paragraph = tester.renderObject<RenderParagraph>(
            find.text(label),
          );
          expect(
            paragraph.didExceedMaxLines,
            isFalse,
            reason: 'the amount on the irreversible button was cut off',
          );
          expect(tester.takeException(), isNull);

          await tester.tap(find.text(label));
          await tester.pumpAndSettle();
          expect(await result, isTrue);
        });
      }
    },
  );

  testWidgets('opening over a focused field unfocuses it first', (
    tester,
  ) async {
    final focus = FocusNode();
    addTearDown(focus.dispose);
    await tester.pumpWidget(host(extra: TextField(focusNode: focus)));
    focus.requestFocus();
    await tester.pump();
    expect(focus.hasFocus, isTrue);

    final result = open(tester, kind: WalletConfirmKind.forward);
    await tester.pumpAndSettle();
    expect(focus.hasFocus, isFalse);

    // Cancel does not bring the keyboard back. This is the row that tells
    // the helper's unfocus apart from the dialog route's own focus scope,
    // which takes focus away while it is open and HANDS IT BACK on pop
    // (watched: without the unfocus, the field is focused again here).
    await tester.tap(find.text(cancel));
    await tester.pumpAndSettle();
    expect(await result, isFalse);
    expect(focus.hasFocus, isFalse);
  });
}
