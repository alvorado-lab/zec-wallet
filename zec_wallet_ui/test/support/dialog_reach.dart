/// **A destructive dialog's warning must be READABLE and its two actions
/// REACHABLE at a large text size on a small phone** (stage S11 C3, the
/// security review's M4). The start-over fund-loss warning clipped at 2.0x on
/// 320 dp, and the delete-wallet dialog had no `scrollable: true` at
/// all. A non-scrolling `AlertDialog` squeezes its body into the space left and
/// clips the rest silently, with no exception — so this checks the geometry.
///
/// The ONE definition the per-site rows call (delete wallet, start over).
library;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';

/// The open Material dialog lays its body out WHOLE (never clipped to the
/// space left), shows both [cancel] and [confirm] on screen inside the dialog,
/// and — once [scrolledToEnd] — has the end of its body on screen too.
/// [bodyStart] is any substring of the body's text.
void expectDialogScrollsToBothActions(
  WidgetTester tester, {
  required String bodyStart,
  required String cancel,
  required String confirm,
  bool scrolledToEnd = false,
}) {
  expect(
    tester.widget<AlertDialog>(find.byType(AlertDialog)).scrollable,
    isTrue,
  );
  final dialog = tester.getRect(find.byType(Dialog));
  final view = tester.view;
  final screen = Offset.zero & (view.physicalSize / view.devicePixelRatio);

  final body = find.descendant(
    of: find.byType(AlertDialog),
    matching: find.textContaining(bodyStart),
  );
  expect(body, findsOneWidget);
  final paragraph = tester.renderObject<RenderParagraph>(body);
  expect(paragraph.didExceedMaxLines, isFalse, reason: 'the body was cut');
  expect(
    paragraph.size.height,
    greaterThanOrEqualTo(
      paragraph.getMaxIntrinsicHeight(paragraph.size.width) - 0.5,
    ),
    reason: 'the body was squeezed into the space left and clipped',
  );
  if (scrolledToEnd) {
    expect(
      tester.getBottomLeft(body).dy,
      lessThanOrEqualTo(dialog.bottom),
      reason: 'the end of the warning stayed off screen',
    );
  }
  for (final label in [cancel, confirm]) {
    final action = tester.getRect(find.text(label));
    expect(
      dialog.contains(action.center) && screen.contains(action.center),
      isTrue,
      reason: '"$label" is not reachable',
    );
  }
}
