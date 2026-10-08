/// The wallet's one confirm dialog (`DESIGN.md` §6.9, stage S11 C3). Every
/// wallet dialog goes through [showWalletConfirm]; a source test keeps
/// `showDialog(`/`AlertDialog(` out of the rest of the package.
///
/// NOT exported: internal policy, like `action_row_layout.dart`.
///
/// It takes text only, never a content widget. §6.9 puts a dialog that holds a
/// text field on the Material dialog on every platform, which this helper does
/// not draw; so a future text-entry dialog cannot go through it by mistake.
library;

import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';

import '../core/theme/colors.dart';

/// What the confirm action does, which decides how it is drawn.
enum WalletConfirmKind {
  /// Goes ahead with something the user asked for (a large send, a sweep, a
  /// server switch). A `FilledButton`; iOS's default (bold) action.
  forward,

  /// Removes or discards something. Red text, never a filled red button
  /// (§6.7); iOS's destructive action.
  destructive,

  /// Two equal choices, neither the default (leave a deposit screen: the swap
  /// continues either way). Two text buttons.
  neutral,

  /// Information only: one Close action, and the result is always `false`
  /// (an information dialog can consent to nothing).
  info,
}

/// Show a confirm dialog and return `true` ONLY when the user tapped
/// [confirmLabel]. Cancel, a barrier tap, a back pop and a route removed from
/// under it are all `false`, never `null`, so no caller can read a dismissal
/// as consent.
///
/// Adaptive: a `CupertinoAlertDialog` on iOS and macOS (whose barrier is
/// inert, the native and safer default), an `AlertDialog` elsewhere. The
/// Material branch always scrolls, so a long warning at a large text size
/// keeps its actions reachable on a small phone.
///
/// [cancelLabel] is required for every kind but [WalletConfirmKind.info],
/// whose one action is [confirmLabel]. [confirmKey] and [cancelKey] identify
/// the actions for a test.
///
/// It unfocuses the primary focus first: a dialog opened over a focused field
/// otherwise leaves the keyboard up behind it, and brings it back after
/// Cancel.
Future<bool> showWalletConfirm(
  BuildContext context, {
  required String title,
  required String body,
  required String confirmLabel,
  String? cancelLabel,
  required WalletConfirmKind kind,
  Key? confirmKey,
  Key? cancelKey,
}) async {
  assert(
    kind == WalletConfirmKind.info || cancelLabel != null,
    'a confirm dialog needs a Cancel label',
  );
  FocusManager.instance.primaryFocus?.unfocus();
  final result = await showAdaptiveDialog<bool>(
    context: context,
    builder: (dialogContext) {
      final platform = Theme.of(dialogContext).platform;
      final cupertino =
          platform == TargetPlatform.iOS || platform == TargetPlatform.macOS;
      void pop(bool value) => Navigator.of(dialogContext).pop(value);
      // The two dialogs are built directly rather than through
      // `AlertDialog.adaptive`, which does the same platform switch but
      // builds a private `AlertDialog` subtype: a test's
      // `find.byType(AlertDialog)` would stop finding the Material one.
      if (cupertino) {
        // Cupertino content always scrolls.
        return CupertinoAlertDialog(
          title: Text(title),
          content: Text(body),
          actions: _cupertinoActions(
            kind,
            confirmLabel: confirmLabel,
            cancelLabel: cancelLabel,
            confirmKey: confirmKey,
            cancelKey: cancelKey,
            pop: pop,
          ),
        );
      }
      return AlertDialog(
        title: Text(title),
        content: Text(body),
        // ALWAYS, not per site: the security review's M4 found a fund-loss
        // warning that did not set it.
        scrollable: true,
        actions: _materialActions(
          dialogContext,
          kind,
          confirmLabel: confirmLabel,
          cancelLabel: cancelLabel,
          confirmKey: confirmKey,
          cancelKey: cancelKey,
          pop: pop,
        ),
      );
    },
  );
  return result ?? false;
}

List<Widget> _cupertinoActions(
  WalletConfirmKind kind, {
  required String confirmLabel,
  required String? cancelLabel,
  required Key? confirmKey,
  required Key? cancelKey,
  required void Function(bool) pop,
}) {
  if (kind == WalletConfirmKind.info) {
    return [
      CupertinoDialogAction(
        key: confirmKey,
        onPressed: () => pop(false),
        child: _CupertinoActionLabel(confirmLabel),
      ),
    ];
  }
  return [
    CupertinoDialogAction(
      key: cancelKey,
      onPressed: () => pop(false),
      child: _CupertinoActionLabel(cancelLabel!),
    ),
    CupertinoDialogAction(
      key: confirmKey,
      // `isDefaultAction` is style only (bold); it binds no Enter key.
      isDefaultAction: kind == WalletConfirmKind.forward,
      isDestructiveAction: kind == WalletConfirmKind.destructive,
      onPressed: () => pop(true),
      child: _CupertinoActionLabel(confirmLabel),
    ),
  ];
}

/// A Cupertino action's label, allowed to WRAP. Below the accessibility text
/// sizes a `CupertinoDialogAction` sets its label to one line with an
/// ellipsis, so a long confirm label — the large-send confirm carries the
/// exact amount in it ("Send 20999999.99999999 ZEC" in the longest locale) —
/// lost its end, the one part the user must read. A fresh `DefaultTextStyle`
/// (same style and alignment, no line limit, no ellipsis) replaces the
/// action's one-line default, so the label wraps at every text size — no cap
/// either, which at an accessibility size would cut it again; the action
/// still scales the wrapped label down to fit its box.
class _CupertinoActionLabel extends StatelessWidget {
  const _CupertinoActionLabel(this.label);

  final String label;

  @override
  Widget build(BuildContext context) {
    final inherited = DefaultTextStyle.of(context);
    return DefaultTextStyle(
      style: inherited.style,
      textAlign: inherited.textAlign,
      softWrap: true,
      overflow: TextOverflow.visible,
      child: Text(label),
    );
  }
}

List<Widget> _materialActions(
  BuildContext context,
  WalletConfirmKind kind, {
  required String confirmLabel,
  required String? cancelLabel,
  required Key? confirmKey,
  required Key? cancelKey,
  required void Function(bool) pop,
}) {
  if (kind == WalletConfirmKind.info) {
    return [
      TextButton(
        key: confirmKey,
        onPressed: () => pop(false),
        child: Text(confirmLabel),
      ),
    ];
  }
  final cancel = TextButton(
    key: cancelKey,
    onPressed: () => pop(false),
    child: Text(cancelLabel!),
  );
  final confirm = switch (kind) {
    WalletConfirmKind.forward => FilledButton(
      key: confirmKey,
      onPressed: () => pop(true),
      child: Text(confirmLabel),
    ),
    // The red is set HERE, explicitly, so no host text-button theme decides
    // whether a destructive action looks destructive (the review's L4).
    WalletConfirmKind.destructive => TextButton(
      key: confirmKey,
      style: TextButton.styleFrom(
        foregroundColor: WalletColors.of(context).red,
      ),
      onPressed: () => pop(true),
      child: Text(confirmLabel),
    ),
    WalletConfirmKind.neutral || WalletConfirmKind.info => TextButton(
      key: confirmKey,
      onPressed: () => pop(true),
      child: Text(confirmLabel),
    ),
  };
  return [cancel, confirm];
}
