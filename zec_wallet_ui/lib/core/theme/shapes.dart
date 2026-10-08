import 'dart:ui' show lerpDouble;

import 'package:flutter/material.dart';

/// Every corner radius the wallet UI draws, named for the ROLE it plays
/// (FR-49 W-11, ADR-0564, stage S11 C1). A widget never writes a radius
/// literal: it reads one of these, so a host that registers its own
/// [WalletShapes] reshapes every surface of that role at once.
///
/// The defaults are Relim's `RelimRadii` wherever the role exists there, so
/// Relim matches without registering anything. [bar], [chip] and [progress]
/// have no `RelimRadii` constant; they are the design-refresh spec's numbers
/// (§9.4 item 3, `DESIGN.md` §6.13 and §6.12).
@immutable
class WalletShapes extends ThemeExtension<WalletShapes> {
  const WalletShapes({
    this.hero = 28,
    this.group = 24,
    this.tile = 22,
    this.bar = 20,
    this.notice = 16,
    this.qr = 16,
    this.field = 22,
    this.chip = 18,
    this.sheet = 28,
    this.dialog = 28,
    this.progress = 3,
  });

  /// The SDK's defaults for [platform]: a group and a sheet's top corners are
  /// rounder on Apple platforms (26 and 38, against 24 and 28), as the
  /// platforms' own grouped lists and sheets are.
  factory WalletShapes.forPlatform(TargetPlatform platform) =>
      switch (platform) {
        TargetPlatform.iOS ||
        TargetPlatform.macOS => const WalletShapes(group: 26, sheet: 38),
        _ => const WalletShapes(),
      };

  /// The balance card.
  final double hero;

  /// Grouped lists and data cards (the review, the quote, the refund
  /// verification, the rescan and onboarding sections), and a card-notice.
  final double group;

  /// The wallet's action tiles.
  final double tile;

  /// The sync bar.
  final double bar;

  /// A notice line (`WalletNotice`).
  final double notice;

  /// The QR tile.
  final double qr;

  /// A text field, the asset-picker field, the mnemonic pill field.
  final double field;

  /// Chips, the watch-only badge, the chain chip, the recovery word pills.
  final double chip;

  /// A bottom sheet's top corners (all four on iOS, where it is inset).
  final double sheet;

  /// A Material dialog.
  final double dialog;

  /// The linear progress bar's ends.
  final double progress;

  /// The registered shapes, or [WalletShapes.forPlatform] for the theme's
  /// platform when the host registered none. Like [WalletIcons], a missing
  /// hook is not a wiring error: the defaults are a complete, working set.
  static WalletShapes of(BuildContext context) {
    final theme = Theme.of(context);
    return theme.extension<WalletShapes>() ??
        WalletShapes.forPlatform(theme.platform);
  }

  @override
  WalletShapes copyWith({
    double? hero,
    double? group,
    double? tile,
    double? bar,
    double? notice,
    double? qr,
    double? field,
    double? chip,
    double? sheet,
    double? dialog,
    double? progress,
  }) => WalletShapes(
    hero: hero ?? this.hero,
    group: group ?? this.group,
    tile: tile ?? this.tile,
    bar: bar ?? this.bar,
    notice: notice ?? this.notice,
    qr: qr ?? this.qr,
    field: field ?? this.field,
    chip: chip ?? this.chip,
    sheet: sheet ?? this.sheet,
    dialog: dialog ?? this.dialog,
    progress: progress ?? this.progress,
  );

  @override
  WalletShapes lerp(WalletShapes? other, double t) {
    if (other == null) return this;
    double l(double a, double b) => lerpDouble(a, b, t)!;
    return WalletShapes(
      hero: l(hero, other.hero),
      group: l(group, other.group),
      tile: l(tile, other.tile),
      bar: l(bar, other.bar),
      notice: l(notice, other.notice),
      qr: l(qr, other.qr),
      field: l(field, other.field),
      chip: l(chip, other.chip),
      sheet: l(sheet, other.sheet),
      dialog: l(dialog, other.dialog),
      progress: l(progress, other.progress),
    );
  }
}
