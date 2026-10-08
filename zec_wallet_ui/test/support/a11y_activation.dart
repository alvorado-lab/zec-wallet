/// **A button a screen reader cannot ACTIVATE, and the assertion that catches
/// it — which is not "the node exists".**
///
/// The shape is `Semantics(button: true, …)` over a child that owns the tap,
/// with the child's subtree dropped (`excludeSemantics: true`, or an inner
/// `ExcludeSemantics`). The exported node then advertises a button and offers no
/// action: a screen reader's double-tap has nothing to invoke and a
/// `uiautomator` dump reads `clickable="false"`. Relim fixed its own ten
/// instances at `0c5ae1bd`; these are this package's.
///
/// **Why these assertions and not `find.bySemanticsLabel(…) findsOneWidget`.**
/// A label finder returns a node throughout the life of the defect — that is
/// exactly how `receive_screen_test.dart`'s "the copy button is an independent
/// semantics node" row stayed green over a dead Copy button, and how
/// `shield_sheet_test.dart` drove the Shield confirm with
/// `tester.tap(find.text(…))`, a POINTER event that hits the live `FilledButton`
/// underneath and never consults the semantics tree at all. Green before the
/// fix, green after, and green with the fix reverted.
///
/// This is the ONE definition every row calls: the contract is one contract
/// across nine controls in six files, and a copy per site is a second answer.
library;

import 'package:flutter/semantics.dart';
import 'package:flutter_test/flutter_test.dart';

/// The node ADVERTISES a button, DECLARES the tap action, and dispatching that
/// action the way an assistive technology does drives the effect the caller then
/// asserts.
///
/// [finder] must match exactly one exported node — a second match is the iOS
/// merge/blob shape (`flutter-patterns.md` § iOS Semantics merging), a defect in
/// its own right.
///
/// `tester.semantics.tap` THROWS a `StateError` on a node that does not declare
/// `SemanticsAction.tap`, which is precisely the pre-fix shape, so this helper
/// fails on the defect twice: once on the matcher, once on the dispatch.
void expectActivatable(
  WidgetTester tester,
  FinderBase<SemanticsNode> finder, {
  required String reason,
}) {
  expect(
    finder,
    findsOneWidget,
    reason: 'exactly one node is exported for this control — $reason',
  );
  expect(
    finder.evaluate().single,
    isSemantics(isButton: true, hasTapAction: true),
    reason:
        'a node that says "button" and offers no action is inert to a screen '
        'reader: the double-tap has nothing to invoke and a uiautomator dump '
        'reads clickable="false" — $reason',
  );
  tester.semantics.tap(finder);
}

/// The anti-vacuity polarity: the control is still ANNOUNCED, and it is honestly
/// NOT actionable — no tap action, and [isButton] says whether it still claims
/// the button role.
///
/// Without this half, "every control carries a tap action" is a bar met by
/// pinning `onTap:` on unconditionally, which would make a disabled money button
/// offer a double-tap it silently ignores.
void expectInert(
  WidgetTester tester,
  FinderBase<SemanticsNode> finder, {
  required String reason,
  bool isButton = false,
  bool? isEnabled,
}) {
  expect(
    finder,
    findsOneWidget,
    reason: 'the control is still announced — $reason',
  );
  expect(
    finder.evaluate().single,
    isSemantics(
      isButton: isButton,
      hasTapAction: false,
      hasEnabledState: isEnabled != null,
      isEnabled: isEnabled,
    ),
    reason:
        'offering "double-tap to activate" on a control that cannot act is the '
        'same dishonesty as withholding it from one that can — $reason',
  );
  expect(
    () => tester.semantics.tap(finder),
    throwsStateError,
    reason:
        'the action must be absent from the node, not merely a no-op behind it '
        '— $reason',
  );
}
