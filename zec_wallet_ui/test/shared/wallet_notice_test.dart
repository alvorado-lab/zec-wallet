import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/colors.dart';
import 'package:zec_wallet_ui/core/theme/icons.dart';
import 'package:zec_wallet_ui/core/theme/shapes.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/shared/wallet_notice.dart';

/// S11 C2 (W-4, `DESIGN.md` §6.10): the notice's two forms, per the contract's
/// §4 Notice rows.
void main() {
  const colors = WalletColors.light;
  const message = 'The wallet could not start syncing.';
  const noticeKey = ValueKey('notice-under-test');

  Widget host(
    Widget child, {
    ThemeData? theme,
    double textScale = 1.0,
    bool scroll = false,
  }) => MaterialApp(
    theme: theme ?? buildTheme(colors),
    localizationsDelegates: WalletLocalizations.localizationsDelegates,
    supportedLocales: WalletLocalizations.supportedLocales,
    builder: (context, app) => MediaQuery.withClampedTextScaling(
      minScaleFactor: textScale,
      maxScaleFactor: textScale,
      child: app!,
    ),
    home: Scaffold(
      body: scroll
          ? ListView(padding: const EdgeInsets.all(16), children: [child])
          : Padding(padding: const EdgeInsets.all(16), child: child),
    ),
  );

  /// The notice's own decoration: the first DecoratedBox inside it.
  BoxDecoration decorationOf(WidgetTester tester) =>
      tester
              .widget<DecoratedBox>(
                find
                    .descendant(
                      of: find.byKey(noticeKey),
                      matching: find.byType(DecoratedBox),
                    )
                    .first,
              )
              .decoration
          as BoxDecoration;

  Color? glyphColor(WidgetTester tester) => tester
      .widget<WalletIcon>(
        find.descendant(
          of: find.byKey(noticeKey),
          matching: find.byType(WalletIcon),
        ),
      )
      .color;

  group('tone', () {
    const expected = {
      WalletNoticeTone.info: 'accent',
      WalletNoticeTone.positive: 'accent',
      WalletNoticeTone.warning: 'orange',
      WalletNoticeTone.danger: 'red',
    };
    Color named(String n) => switch (n) {
      'accent' => colors.accent,
      'orange' => colors.orange,
      _ => colors.red,
    };

    for (final entry in expected.entries) {
      testWidgets('${entry.key.name} draws its glyph in ${entry.value}, on '
          'both forms', (tester) async {
        await tester.pumpWidget(
          host(
            WalletNotice(
              key: noticeKey,
              tone: entry.key,
              glyph: WalletGlyph.info,
              message: message,
            ),
          ),
        );
        expect(glyphColor(tester), named(entry.value));

        await tester.pumpWidget(
          host(
            WalletNotice.card(
              key: noticeKey,
              tone: entry.key,
              glyph: WalletGlyph.info,
              message: message,
            ),
          ),
        );
        expect(glyphColor(tester), named(entry.value));
      });
    }
  });

  testWidgets('the line: bgHover at the notice radius, no border, a 16 glyph, '
      'the message in text', (tester) async {
    await tester.pumpWidget(
      host(
        const WalletNotice(
          key: noticeKey,
          tone: WalletNoticeTone.warning,
          glyph: WalletGlyph.error,
          message: message,
        ),
      ),
    );
    final d = decorationOf(tester);
    expect(d.color, colors.bgHover);
    expect(d.borderRadius, BorderRadius.circular(16));
    expect(d.border, isNull);
    expect(tester.widget<WalletIcon>(find.byType(WalletIcon)).size, 16);
    // The tone stays on the glyph: the words are in `text`.
    expect(tester.widget<Text>(find.text(message)).style!.color, colors.text);
  });

  testWidgets('the line with messageInTone draws the words in the tone', (
    tester,
  ) async {
    await tester.pumpWidget(
      host(
        const WalletNotice(
          key: noticeKey,
          tone: WalletNoticeTone.warning,
          glyph: WalletGlyph.transparent,
          message: message,
          messageInTone: true,
        ),
      ),
    );
    expect(tester.widget<Text>(find.text(message)).style!.color, colors.orange);
  });

  testWidgets('the card: bgCard at the group radius, no border, a 20 glyph', (
    tester,
  ) async {
    await tester.pumpWidget(
      host(
        const WalletNotice.card(
          key: noticeKey,
          tone: WalletNoticeTone.warning,
          glyph: WalletGlyph.warning,
          title: 'This payment is public',
          message: message,
        ),
      ),
    );
    final d = decorationOf(tester);
    expect(d.color, colors.bgCard);
    expect(
      d.borderRadius,
      BorderRadius.circular(
        WalletShapes.forPlatform(TargetPlatform.android).group,
      ),
    );
    expect(d.border, isNull);
    expect(tester.widget<WalletIcon>(find.byType(WalletIcon)).size, 20);
  });

  testWidgets('the card on a bgCard surface (a sheet) takes bgHover, so it '
      'still has an edge (S11 diff review M-1)', (tester) async {
    await tester.pumpWidget(
      host(
        Material(
          color: colors.bgCard,
          child: const WalletNotice.card(
            key: noticeKey,
            tone: WalletNoticeTone.warning,
            glyph: WalletGlyph.transparent,
            title: 'This payment is public',
            message: message,
          ),
        ),
      ),
    );
    final d = decorationOf(tester);
    expect(d.color, colors.bgHover);
    expect(d.color, isNot(colors.bgCard));
  });

  group("the card's title", () {
    const title = 'What is at stake';
    Future<Color?> titleColor(
      WidgetTester tester,
      WalletNoticeTone tone,
    ) async {
      await tester.pumpWidget(
        host(
          WalletNotice.card(
            key: noticeKey,
            tone: tone,
            glyph: WalletGlyph.warning,
            title: title,
            message: message,
          ),
        ),
      );
      return tester.widget<Text>(find.text(title)).style!.color;
    }

    testWidgets('is orange for a warning and red for a danger', (tester) async {
      expect(await titleColor(tester, WalletNoticeTone.warning), colors.orange);
      expect(await titleColor(tester, WalletNoticeTone.danger), colors.red);
    });

    testWidgets('is text for info and positive', (tester) async {
      expect(await titleColor(tester, WalletNoticeTone.info), colors.text);
      expect(await titleColor(tester, WalletNoticeTone.positive), colors.text);
    });

    testWidgets('is optional', (tester) async {
      await tester.pumpWidget(
        host(
          const WalletNotice.card(
            key: noticeKey,
            tone: WalletNoticeTone.warning,
            glyph: WalletGlyph.syncProblem,
            message: message,
          ),
        ),
      );
      expect(find.text(message), findsOneWidget);
      // Only the message: no empty title line.
      expect(
        find.descendant(of: find.byKey(noticeKey), matching: find.byType(Text)),
        findsOneWidget,
      );
    });
  });

  testWidgets('child renders below the message, on both forms', (tester) async {
    const childKey = ValueKey('notice-child');
    for (final notice in [
      const WalletNotice(
        key: noticeKey,
        tone: WalletNoticeTone.danger,
        glyph: WalletGlyph.memo,
        message: message,
        child: SizedBox(key: childKey, height: 10, width: 10),
      ),
      const WalletNotice.card(
        key: noticeKey,
        tone: WalletNoticeTone.danger,
        glyph: WalletGlyph.memo,
        message: message,
        child: SizedBox(key: childKey, height: 10, width: 10),
      ),
    ]) {
      await tester.pumpWidget(host(notice));
      expect(
        tester.getTopLeft(find.byKey(childKey)).dy,
        greaterThanOrEqualTo(tester.getBottomLeft(find.text(message)).dy),
      );
    }
  });

  testWidgets('a host-registered WalletShapes sets both radii', (tester) async {
    final theme = buildTheme(
      colors,
      extensions: const [WalletShapes(notice: 5, group: 7)],
    );
    await tester.pumpWidget(
      host(
        const WalletNotice(
          key: noticeKey,
          tone: WalletNoticeTone.info,
          glyph: WalletGlyph.info,
          message: message,
        ),
        theme: theme,
      ),
    );
    expect(decorationOf(tester).borderRadius, BorderRadius.circular(5));

    await tester.pumpWidget(
      host(
        const WalletNotice.card(
          key: noticeKey,
          tone: WalletNoticeTone.info,
          glyph: WalletGlyph.info,
          message: message,
        ),
        theme: theme,
      ),
    );
    expect(decorationOf(tester).borderRadius, BorderRadius.circular(7));
  });

  testWidgets('liveRegion reaches the node that carries the words', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    for (final live in [true, false]) {
      await tester.pumpWidget(
        host(
          WalletNotice(
            key: noticeKey,
            tone: WalletNoticeTone.warning,
            glyph: WalletGlyph.error,
            message: message,
            liveRegion: live,
          ),
        ),
      );
      expect(
        tester.getSemantics(find.text(message)).flagsCollection.isLiveRegion,
        live,
      );
    }
    handle.dispose();
  });

  testWidgets('the glyph is silent to a screen reader', (tester) async {
    final handle = tester.ensureSemantics();
    await tester.pumpWidget(
      host(
        const WalletNotice(
          key: noticeKey,
          tone: WalletNoticeTone.warning,
          glyph: WalletGlyph.error,
          message: message,
        ),
      ),
    );
    expect(
      find.descendant(
        of: find.byKey(noticeKey),
        matching: find.byType(ExcludeSemantics),
      ),
      findsWidgets,
    );
    expect(find.bySemanticsLabel(message), findsOneWidget);
    handle.dispose();
  });

  group('line actions', () {
    const actionLabel = 'Try again';
    Widget lineWithAction() => WalletNotice(
      key: noticeKey,
      tone: WalletNoticeTone.warning,
      glyph: WalletGlyph.syncProblem,
      message: message,
      actions: [TextButton(onPressed: () {}, child: const Text(actionLabel))],
    );

    testWidgets('sit inline beside the message at 1.0x on a wide row', (
      tester,
    ) async {
      await tester.pumpWidget(host(lineWithAction()));
      final action = tester.getRect(
        find.widgetWithText(TextButton, actionLabel),
      );
      final text = tester.getRect(find.text(message));
      expect(action.left, greaterThan(text.right));
      expect(action.top, lessThan(text.bottom));
    });

    testWidgets('stack under the message at 2.0x text', (tester) async {
      await tester.pumpWidget(host(lineWithAction(), textScale: 2.0));
      final action = tester.getRect(
        find.widgetWithText(TextButton, actionLabel),
      );
      final text = tester.getRect(find.text(message));
      expect(action.top, greaterThanOrEqualTo(text.bottom));
      expect(tester.takeException(), isNull);
    });
  });

  testWidgets('the dismiss button is labelled and fires', (tester) async {
    final handle = tester.ensureSemantics();
    var dismissed = 0;
    await tester.pumpWidget(
      host(
        WalletNotice(
          key: noticeKey,
          tone: WalletNoticeTone.positive,
          glyph: WalletGlyph.rescan,
          message: message,
          onDismiss: () => dismissed++,
        ),
      ),
    );
    final l10n = WalletLocalizations.of(tester.element(find.byKey(noticeKey)));
    // The tooltip is what a screen reader announces for an icon button.
    expect(
      tester.getSemantics(find.byType(IconButton)).tooltip,
      l10n.walletDeepScanRestoreNoteDismiss,
    );
    await tester.tap(find.byType(IconButton));
    expect(dismissed, 1);
    handle.dispose();
  });

  testWidgets('3.0x text on a 320 dp phone: both forms scroll, nothing '
      'overflows', (tester) async {
    tester.view.physicalSize = const Size(320, 568);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);
    const long =
        'Your wallet could not reach the server it syncs from, so the '
        'balance and activity shown here may be out of date until it can.';
    await tester.pumpWidget(
      host(
        Column(
          children: [
            WalletNotice(
              tone: WalletNoticeTone.warning,
              glyph: WalletGlyph.syncProblem,
              message: long,
              onDismiss: () {},
              actions: [
                TextButton(onPressed: () {}, child: const Text('Try again')),
              ],
            ),
            const SizedBox(height: 12),
            WalletNotice.card(
              key: noticeKey,
              tone: WalletNoticeTone.danger,
              glyph: WalletGlyph.memo,
              title: 'Include this memo or the funds are lost',
              message: long,
              actions: [
                FilledButton(onPressed: () {}, child: const Text('Copy memo')),
                OutlinedButton(onPressed: () {}, child: const Text('Done')),
              ],
            ),
          ],
        ),
        textScale: 3.0,
        scroll: true,
      ),
    );
    expect(tester.takeException(), isNull);
    // The card's last action is reachable by scrolling.
    await tester.scrollUntilVisible(
      find.text('Done'),
      200,
      scrollable: find.byType(Scrollable),
    );
    expect(tester.takeException(), isNull);
  });
}
