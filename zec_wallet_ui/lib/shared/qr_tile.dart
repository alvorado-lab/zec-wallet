import 'package:flutter/material.dart';
import 'package:qr_flutter/qr_flutter.dart';

import '../core/theme/colors.dart';
import '../core/theme/shapes.dart';

/// A scannable QR tile shared by the receive screen and the IntoZec deposit
/// screen (the wallet's two "show this code to scan" surfaces).
///
/// It pins a white tile — a QR-scanner requirement, NOT a theme token
/// (flutter-patterns allows a documented functional pin) — with a ≥4-module
/// quiet zone (the 24px padding at the default 240dp size). The dark modules are
/// the host's `WalletColors.qrInk` (FR-49 W-10; default black), but only while
/// that ink keeps [minQrInkContrast] against the white tile: a scanner binarises
/// on luminance, and a code that fails to scan is an address nobody can pay. A
/// paler ink falls back to black. The whole tile is one
/// `image` Semantics node labelled by [label], and the QR's own internal
/// semantics are excluded so a screen reader doesn't announce it twice.
///
/// Money-correctness: the inner [QrImageView] carries a `ValueKey(payload)` so a
/// widget test can pin the EXACT payload the QR encodes (a dropped character
/// would send funds nowhere) even though `QrImageView` hides its data. Pass
/// [tileKey] to identify the outer tile from a test (each call site keeps its own
/// stable key).
/// The least contrast a host's QR ink must keep on the white tile before the
/// tile uses it (WCAG enhanced, 7:1). Black is 21:1; the refreshed design's
/// `#0F3D27` is 12.6:1.
const double minQrInkContrast = 7.0;

/// The module colour the tile draws: [ink] if it keeps [minQrInkContrast] on
/// white, else black.
Color qrModuleColor(Color ink) {
  const paper = Color(0xFFFFFFFF);
  final hi = paper.computeLuminance();
  final lo = ink.computeLuminance();
  final ratio = (hi + 0.05) / (lo + 0.05);
  return ink.a == 1.0 && ratio >= minQrInkContrast
      ? ink
      : const Color(0xFF000000);
}

class QrTile extends StatelessWidget {
  const QrTile({
    super.key,
    required this.payload,
    required this.label,
    this.tileKey,
    this.size = 240,
  });

  /// The exact bytes the QR encodes (an address, or a deposit address) — also
  /// the displayed/copied string. Never transformed here.
  final String payload;

  /// The a11y label for the whole tile (distinct per surface so a screen reader
  /// doesn't announce two QRs identically).
  final String label;

  /// Stable key on the outer tile for a money-correctness widget test.
  final Key? tileKey;

  /// Module render size in logical pixels (default 240; the deposit screen uses
  /// a slightly smaller 220 to leave room for the memo/instruction below).
  final double size;

  @override
  Widget build(BuildContext context) {
    final colors = WalletColors.of(context);
    final ink = qrModuleColor(colors.qrInk);
    return Semantics(
      label: label,
      image: true,
      excludeSemantics: true,
      child: Container(
        key: tileKey,
        padding: const EdgeInsets.all(24),
        decoration: BoxDecoration(
          // Pinned white tile (scanner requirement) with a soft brand-accent
          // halo — a glow that lifts the code off the surface in both themes.
          // Decorative only.
          color: const Color(0xFFFFFFFF),
          borderRadius: BorderRadius.circular(WalletShapes.of(context).qr),
          boxShadow: [
            BoxShadow(
              color: colors.accent.withValues(alpha: 0.30),
              blurRadius: 28,
              spreadRadius: 1,
            ),
          ],
        ),
        child: QrImageView(
          key: ValueKey<String>(payload),
          data: payload,
          version: QrVersions.auto,
          errorCorrectionLevel: QrErrorCorrectLevel.M,
          size: size,
          gapless: true,
          eyeStyle: QrEyeStyle(eyeShape: QrEyeShape.square, color: ink),
          dataModuleStyle: QrDataModuleStyle(
            dataModuleShape: QrDataModuleShape.square,
            color: ink,
          ),
        ),
      ),
    );
  }
}
