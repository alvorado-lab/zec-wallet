/// The wallet's one bottom-sheet entry (`DESIGN.md` §6.8, stage S11 C4).
/// Every wallet sheet opens through [showWalletSheet]; a source test keeps
/// `showModalBottomSheet(` out of the rest of the package.
///
/// NOT exported: internal policy, like `action_row_layout.dart`.
library;

import 'package:flutter/material.dart';

import '../core/theme/colors.dart';
import '../core/theme/icons.dart';
import '../core/theme/shapes.dart';
import '../core/theme/sheet_layout.dart';

/// Open [builder] as a modal bottom sheet and return the route's future (so a
/// picker can return its choice and a caller can chain `whenComplete`).
///
/// Every sheet gets the same frame: scroll-controlled (it grows with large
/// text), inside the safe area, capped at [walletSheetMaxWidth] on a wide
/// window, and shaped by [WalletShapes.sheet].
///
/// * **Android and the rest:** attached to the bottom edge, top corners
///   rounded, with Material's drag handle.
/// * **iOS and macOS:** inset 8 from the sides and the bottom with all four
///   corners rounded, and a 36 × 5 grabber in `colorScheme.outline`.
///
/// [isDismissible] and [enableDrag] pass through. With [enableDrag] false no
/// handle or grabber is drawn: it would advertise a drag that does nothing
/// (the rescan sheet, which also locks its barrier).
Future<T?> showWalletSheet<T>(
  BuildContext context, {
  required WidgetBuilder builder,
  bool isDismissible = true,
  bool enableDrag = true,
}) {
  final theme = Theme.of(context);
  final radius = Radius.circular(WalletShapes.of(context).sheet);
  const constraints = BoxConstraints(maxWidth: walletSheetMaxWidth);
  final cupertino =
      theme.platform == TargetPlatform.iOS ||
      theme.platform == TargetPlatform.macOS;

  if (!cupertino) {
    return showModalBottomSheet<T>(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      isDismissible: isDismissible,
      enableDrag: enableDrag,
      showDragHandle: enableDrag,
      constraints: constraints,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: radius),
      ),
      builder: builder,
    );
  }

  // The inset card draws its own fill, so the route's sheet is transparent.
  // The fill is the host's sheet colour when its theme names one.
  final sheetTheme = theme.bottomSheetTheme;
  final fill =
      sheetTheme.modalBackgroundColor ??
      sheetTheme.backgroundColor ??
      WalletColors.of(context).bgCard;
  return showModalBottomSheet<T>(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    isDismissible: isDismissible,
    enableDrag: enableDrag,
    showDragHandle: false,
    constraints: constraints,
    backgroundColor: Colors.transparent,
    elevation: 0,
    shape: const RoundedRectangleBorder(),
    builder: (sheetContext) => Padding(
      padding: const EdgeInsets.fromLTRB(8, 0, 8, 8),
      child: Material(
        color: fill,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.all(radius)),
        clipBehavior: Clip.antiAlias,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            if (enableDrag) const _Grabber(),
            Flexible(child: Builder(builder: builder)),
          ],
        ),
      ),
    ),
  );
}

/// A wallet sheet's title row (DESIGN §6.8, S13 §1.7): the title in
/// `titleLarge` and, on the trailing side, a 44 close circle on the sheet's
/// raised fill (`surface2`, the SDK's [WalletColors.bgHover]).
///
/// [closeKey] identifies the close for a test (`Key('<sheet>-close')`). Its
/// accessible name is `MaterialLocalizations.closeButtonLabel` — no new
/// string. [showClose] false draws no close at all: a sheet that holds its
/// exit while it works (the rescan sheet while it runs) must not offer one.
/// [onClose] defaults to `Navigator.maybePop`, so a sheet's own `PopScope`
/// still has the last word.
class WalletSheetHeader extends StatelessWidget {
  const WalletSheetHeader({
    super.key,
    required this.title,
    required this.closeKey,
    this.showClose = true,
    this.onClose,
    this.leading,
    this.titleColor,
  });

  final String title;
  final Key closeKey;
  final bool showClose;
  final VoidCallback? onClose;

  /// An optional glyph before the title (the sync and transaction sheets).
  final Widget? leading;

  /// The title's colour when it carries a state (the sync sheet's tint);
  /// the theme's when null.
  final Color? titleColor;

  /// The close circle's side, pinned by the S13 row.
  static const double closeSize = 44;

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final leading = this.leading;
    final heading = Text(
      title,
      style: titleColor == null
          ? textTheme.titleLarge
          : textTheme.titleLarge?.copyWith(color: titleColor),
    );
    return Row(
      children: [
        Expanded(
          child: leading == null
              ? heading
              : MergeSemantics(
                  child: Row(
                    children: [
                      leading,
                      const SizedBox(width: 12),
                      Expanded(child: heading),
                    ],
                  ),
                ),
        ),
        if (showClose) ...[
          const SizedBox(width: 8),
          _SheetCloseButton(
            key: closeKey,
            onPressed: onClose ?? () => Navigator.of(context).maybePop(),
          ),
        ],
      ],
    );
  }
}

class _SheetCloseButton extends StatelessWidget {
  const _SheetCloseButton({super.key, required this.onPressed});

  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    final colors = WalletColors.of(context);
    final name = MaterialLocalizations.of(context).closeButtonLabel;
    // The name is the node's LABEL (an IconButton's tooltip reaches a screen
    // reader as a tooltip, not a name). `excludeSemantics` drops the inner
    // button's node and its tap action with it, so the action lives here.
    return Semantics(
      button: true,
      label: name,
      onTap: onPressed,
      excludeSemantics: true,
      child: SizedBox.square(
        dimension: WalletSheetHeader.closeSize,
        child: IconButton(
          padding: EdgeInsets.zero,
          constraints: const BoxConstraints.tightFor(
            width: WalletSheetHeader.closeSize,
            height: WalletSheetHeader.closeSize,
          ),
          style: IconButton.styleFrom(
            backgroundColor: colors.bgHover,
            foregroundColor: colors.text,
          ),
          tooltip: name,
          icon: const WalletIcon(WalletGlyph.close, size: 20),
          onPressed: onPressed,
        ),
      ),
    );
  }
}

/// Identifies the iOS sheet's grabber for a test.
const walletSheetGrabberKey = ValueKey('wallet-sheet-grabber');

/// The iOS sheet's grabber: decoration only (the drag is the whole sheet's).
class _Grabber extends StatelessWidget {
  const _Grabber();

  @override
  Widget build(BuildContext context) {
    return ExcludeSemantics(
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 6),
        child: Center(
          child: SizedBox(
            key: walletSheetGrabberKey,
            width: 36,
            height: 5,
            child: DecoratedBox(
              decoration: ShapeDecoration(
                color: Theme.of(context).colorScheme.outline,
                shape: const StadiumBorder(),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
