import 'package:flutter/material.dart';

import '../../../core/theme/colors.dart';
import '../../../shared/theme/avatar_color.dart';

/// A token icon for the IntoZec source picker (spec §3.3b D5).
///
/// Icons are BUNDLED locally — token/chain drawables shipped in the app, NEVER
/// CDN-fetched (§5.2 censorship/privacy; the §5.4 "no token field is logged"
/// posture means we don't even resolve a symbol against a remote registry). When
/// a symbol has no bundled drawable, a deterministic MONOGRAM placeholder renders
/// (the spec'd fallback) — so the picker is never blank and never reaches the
/// network for art.
///
/// SEAM (content follow-up, manager-carried like the asset-registry verification):
/// bundling the real drawables is a content task. To add one: drop
/// `assets/tokens/<symbol>.png`, register `assets/tokens/` in `pubspec.yaml`, and
/// add `<symbol>` to [_bundledTokenIcons]. Until then this set is empty and the
/// monogram ships — no code change, no network, ever.
class SwapTokenIcon extends StatelessWidget {
  const SwapTokenIcon({
    super.key,
    required this.symbol,
    required this.chain,
    this.size = 32,
  });

  /// The provider asset symbol (e.g. `"usdc"`) — also the monogram seed.
  final String symbol;

  /// The provider chain id (e.g. `"eth"`) — disambiguates the monogram tint so
  /// the same symbol on two chains is visually distinct.
  final String chain;

  final double size;

  /// Symbols with a bundled drawable. EMPTY until the content pass lands real
  /// art; an entry here is matched by `assets/tokens/<symbol>.png`.
  static const Set<String> _bundledTokenIcons = {};

  @override
  Widget build(BuildContext context) {
    final lower = symbol.toLowerCase();
    if (_bundledTokenIcons.contains(lower)) {
      return Image.asset(
        'assets/tokens/$lower.png',
        width: size,
        height: size,
        // Defensive: if a registered asset ever fails to decode, fall back to the
        // monogram rather than showing a broken-image glyph on a money surface.
        errorBuilder: (context, _, _) =>
            _Monogram(symbol: symbol, chain: chain, size: size),
      );
    }
    return _Monogram(symbol: symbol, chain: chain, size: size);
  }
}

/// The placeholder: a tinted circle with the symbol's first letters. Pure-render,
/// no network. Deterministic tint via the shared [avatarColorFor] (DRY) seeded by
/// `symbol+chain` so it is stable per asset and distinct across chains.
class _Monogram extends StatelessWidget {
  const _Monogram({
    required this.symbol,
    required this.chain,
    required this.size,
  });

  final String symbol;
  final String chain;
  final double size;

  @override
  Widget build(BuildContext context) {
    final colors = WalletColors.of(context);
    final tint = avatarColorFor('$symbol@$chain', colors);
    final initials = _initials(symbol);
    return Container(
      width: size,
      height: size,
      alignment: Alignment.center,
      decoration: BoxDecoration(
        color: tint.withValues(alpha: 0.18),
        shape: BoxShape.circle,
      ),
      child: Text(
        initials,
        style: TextStyle(
          color: tint,
          fontSize: size * 0.38,
          fontWeight: FontWeight.w700,
        ),
      ),
    );
  }

  /// 1–2 uppercase letters from the symbol (e.g. `"usdc"` → `"US"`, `"x"` → `"X"`).
  static String _initials(String symbol) {
    final s = symbol.trim();
    if (s.isEmpty) return '?';
    return s.substring(0, s.length >= 2 ? 2 : 1).toUpperCase();
  }
}
