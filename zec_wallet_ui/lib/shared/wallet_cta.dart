import 'package:flutter/widgets.dart';

/// Where a call to action sits, which decides its height floor.
enum WalletCtaSize {
  /// A page's primary action: at least 56 tall.
  page,

  /// A sheet's primary action: at least 52 tall.
  sheet,
}

/// A full-width call to action (S13 §1.1): a min-height FLOOR over the
/// theme's own button, never a `minimumSize` literal (the S11 source ban
/// holds; the theme's floor stays 48), so a host theme still styles the
/// button and a taller host minimum still wins.
class WalletCta extends StatelessWidget {
  const WalletCta({
    super.key,
    required this.child,
    this.size = WalletCtaSize.page,
  });

  /// The themed button (a `FilledButton`, `OutlinedButton`, …).
  final Widget child;

  final WalletCtaSize size;

  /// The floor for [size], pinned by the S13 row.
  static double floorOf(WalletCtaSize size) => switch (size) {
    WalletCtaSize.page => 56,
    WalletCtaSize.sheet => 52,
  };

  @override
  Widget build(BuildContext context) {
    return ConstrainedBox(
      constraints: BoxConstraints(
        minWidth: double.infinity,
        minHeight: floorOf(size),
      ),
      child: child,
    );
  }
}
