import 'package:flutter/material.dart';

import '../core/theme/colors.dart';
import '../core/theme/shapes.dart';

/// A grouped list (S13 Build B; DESIGN §2.5): rows on one `bgCard` surface
/// cut to the `group` radius, split by 1 dp hairlines inset to where a row's
/// text starts. The security settings screen and the wallet's sections
/// (in-flight swaps, parked sends) draw it. The activity list (S12 C5) and the deposit
/// block (S13 §1.4) still hand-roll their own container to the same spec;
/// moving them onto this widget is a follow-up.
///
/// A [Material], not a bare coloured box, so a tappable row's ink shows on
/// the group's surface instead of under it.
class WalletGroup extends StatelessWidget {
  const WalletGroup({
    super.key,
    required this.children,
    this.dividerIndent = rowInset,
  });

  /// The rows, top to bottom; a hairline is drawn between each pair.
  final List<Widget> children;

  /// Where the hairlines start: the row's text inset.
  final double dividerIndent;

  /// The inset of a row with no leading glyph (its 16 padding).
  static const double rowInset = 16;

  /// The inset of a `ListTile` row with a leading glyph: the 16 padding, the
  /// 24 glyph and the tile's 16 gap.
  static const double glyphRowInset = 56;

  @override
  Widget build(BuildContext context) {
    final colors = WalletColors.of(context);
    return Material(
      color: colors.bgCard,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(WalletShapes.of(context).group),
      ),
      clipBehavior: Clip.antiAlias,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          for (var i = 0; i < children.length; i++) ...[
            if (i > 0)
              Divider(
                height: 1,
                thickness: 1,
                indent: dividerIndent,
                color: colors.border,
              ),
            children[i],
          ],
        ],
      ),
    );
  }
}
