import 'package:flutter/material.dart';

/// Type the wallet UI needs beyond Material's `TextTheme` (FR-49 W-2,
/// ADR-0564). A host registers one in its `ThemeData.extensions` to set the
/// face the SDK uses for identifiers.
@immutable
class WalletTypography extends ThemeExtension<WalletTypography> {
  const WalletTypography({
    this.mono = const TextStyle(fontFamily: 'monospace'),
  });

  /// The SDK's own default: the platform's generic monospace face.
  static const fallback = WalletTypography();

  /// The face for identifiers: addresses, the swap refund and destination
  /// fields, a deposit memo. Only its family, fallbacks and features carry
  /// ([monoOn]); each site keeps its own size, weight and colour. The SDK
  /// bundles no font (ruling A), so a host that wants a specific face bundles
  /// it and names it here.
  final TextStyle mono;

  /// [base] set in the mono face: its family, fallbacks and font features,
  /// with the site's own size, spacing, weight and colour kept.
  TextStyle monoOn(TextStyle base) => base.copyWith(
    fontFamily: mono.fontFamily,
    fontFamilyFallback: mono.fontFamilyFallback,
    fontFeatures: mono.fontFeatures ?? base.fontFeatures,
  );

  /// The registered typography, or [fallback] when the host registered none.
  static WalletTypography of(BuildContext context) =>
      Theme.of(context).extension<WalletTypography>() ?? fallback;

  @override
  WalletTypography copyWith({TextStyle? mono}) =>
      WalletTypography(mono: mono ?? this.mono);

  @override
  WalletTypography lerp(WalletTypography? other, double t) {
    if (other == null) return this;
    return WalletTypography(mono: TextStyle.lerp(mono, other.mono, t)!);
  }
}
