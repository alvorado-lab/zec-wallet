import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/theme/colors.dart';
import '../../core/theme/typography.dart';
import 'wallet_providers.dart';
import 'wallet_session.dart';

/// Spin requests for the balance card's coin: bumped by the wallet tab on
/// the two events that earn a spin (FR-49 S12, C3) — the live status moving
/// INTO a synced state, and a live arrival of funds. The coin spins once per
/// bump it sees; a bump while it spins is dropped.
final walletCoinSpinsProvider = NotifierProvider<WalletCoinSpins, int>(
  WalletCoinSpins.new,
);

/// The spin counter.
class WalletCoinSpins extends Notifier<int> {
  @override
  int build() => 0;

  /// The highest arrival height already spun for — the host-side watermark
  /// the SDK's `IncomingFundsEvent` doc asks for (delivery is at-least-once).
  /// It belongs to the session that set it ([_watermarkSession]): a rescan or
  /// a server switch starts a new one, so a height a previous session saw
  /// (a reorged branch, a server that lied high) cannot silence this one.
  int _arrivalWatermark = -1;
  WalletSession? _watermarkSession;

  /// Request one spin.
  void bump() => state = state + 1;

  /// Request one spin for a live arrival reaching [spanToHeight], unless an
  /// arrival at or above that height already earned one in this session (a
  /// re-delivered event after a resume must not spin the coin twice).
  void bumpArrival(int spanToHeight) {
    final session = ref.read(walletSessionProvider);
    if (!identical(session, _watermarkSession)) {
      _watermarkSession = session;
      _arrivalWatermark = -1;
    }
    if (spanToHeight <= _arrivalWatermark) return;
    _arrivalWatermark = spanToHeight;
    bump();
  }
}

/// The balance card's 3D coin (FR-49 W-6, stage S12; Relim design-refresh
/// §9.4 item 4 and §9.5 W-6). It rests still, tilted, and spins exactly ONCE
/// each time [spins] changes to a new value — the caller bumps it on the two
/// events that earn a spin (reaching a synced state, a live arrival).
///
/// Never spins on the first build (the first [spins] value is the baseline),
/// never while a spin is in flight (the bump is dropped, not queued), never
/// under the platform's reduce-motion setting, and never offstage. Purely
/// decorative: excluded from semantics.
class WalletCoin extends StatefulWidget {
  const WalletCoin({super.key, required this.spins, this.size = 84});

  /// A counter; each change after the first build is one spin request.
  final int spins;

  /// The coin's diameter in logical pixels.
  final double size;

  /// The spin's length and curve (Relim spec §9.5 W-6: the template's
  /// 1.4 s `cubic-bezier(.2,.8,.2,1)`).
  static const spinDuration = Duration(milliseconds: 1400);
  static const spinCurve = Cubic(0.2, 0.8, 0.2, 1);

  @override
  State<WalletCoin> createState() => WalletCoinState();
}

/// Public so a test can read whether a spin is in flight.
class WalletCoinState extends State<WalletCoin>
    with SingleTickerProviderStateMixin {
  late final AnimationController _spin = AnimationController(
    vsync: this,
    duration: WalletCoin.spinDuration,
  );

  /// Whether a spin is running now.
  bool get spinning => _spin.isAnimating;

  @override
  void didUpdateWidget(WalletCoin oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.spins == oldWidget.spins) return;
    // One spin at a time; a request while spinning is dropped, not queued.
    if (_spin.isAnimating) return;
    if (MediaQuery.maybeDisableAnimationsOf(context) ?? false) return;
    if (!TickerMode.valuesOf(context).enabled) return;
    _spin.forward(from: 0);
  }

  @override
  void dispose() {
    _spin.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colors = WalletColors.of(context);
    final mono = WalletTypography.of(context);
    return ExcludeSemantics(
      child: RepaintBoundary(
        child: AnimatedBuilder(
          animation: _spin,
          builder: (context, _) {
            final t = WalletCoin.spinCurve.transform(_spin.value);
            // Rest at −24°; a spin turns two full turns to 696°.
            final turnDeg = -24.0 + 720.0 * t;
            return _CoinBody(
              size: widget.size,
              turn: turnDeg * math.pi / 180,
              colors: colors,
              label: mono.monoOn(
                TextStyle(
                  fontSize: 17 * widget.size / 84,
                  fontWeight: FontWeight.w600,
                  color: colors.onDeep,
                  height: 1,
                ),
              ),
            );
          },
        ),
      ),
    );
  }
}

/// The coin at one rotation: nine stacked discs along Z, each projected by
/// its own full matrix (Flutter does not preserve 3D across nested
/// transforms), painted far to near.
class _CoinBody extends StatelessWidget {
  const _CoinBody({
    required this.size,
    required this.turn,
    required this.colors,
    required this.label,
  });

  final double size;
  final double turn;
  final WalletColors colors;
  final TextStyle label;

  static const _tilt = 12 * math.pi / 180;
  static const _layers = 9; // z = −4 … +4 (template units at 84 px)

  @override
  Widget build(BuildContext context) {
    final unit = size / 84;
    // The back cap: the edge colour a shade toward the card's own deep.
    final back = Color.lerp(colors.coinEdge, colors.deep, 0.4)!;

    Matrix4 at(double z, {bool flip = false}) {
      final m = Matrix4.identity()
        ..setEntry(3, 2, 1 / 500)
        ..rotateX(_tilt)
        ..rotateY(turn)
        ..translateByDouble(0, 0, z * unit, 1);
      // The back face is drawn turned half a revolution, so its label reads
      // the right way round when it faces the viewer.
      if (flip) m.rotateY(math.pi);
      return m;
    }

    // Depth of a layer's centre after the tilt and turn: larger is farther
    // (Matrix4's perspective entry divides by 1 + z/500).
    double depth(double z) => z * math.cos(turn) * math.cos(_tilt);

    final entries = <({double depth, Widget child})>[];
    for (var i = 0; i < _layers; i++) {
      final z = -4.0 + i;
      entries.add((
        depth: depth(z),
        child: Transform(
          alignment: Alignment.center,
          transform: at(z),
          child: _Disc(size: size, color: colors.coinEdge),
        ),
      ));
    }
    // The two faces sit just outside the edge stack: front at −4.5 (nearest at
    // rest), back at +4.5.
    entries.add((
      depth: depth(-4.5),
      child: Transform(
        alignment: Alignment.center,
        transform: at(-4.5),
        child: _Face(size: size, colors: colors, label: label),
      ),
    ));
    entries.add((
      depth: depth(4.5),
      child: Transform(
        alignment: Alignment.center,
        transform: at(4.5, flip: true),
        child: _Face(size: size, colors: colors, label: label, cap: back),
      ),
    ));
    entries.sort((a, b) => b.depth.compareTo(a.depth));

    return SizedBox.square(
      dimension: size,
      child: Stack(
        clipBehavior: Clip.none,
        children: [for (final e in entries) Positioned.fill(child: e.child)],
      ),
    );
  }
}

class _Disc extends StatelessWidget {
  const _Disc({required this.size, required this.color});

  final double size;
  final Color color;

  @override
  Widget build(BuildContext context) => DecoratedBox(
    decoration: BoxDecoration(shape: BoxShape.circle, color: color),
    child: SizedBox.square(dimension: size),
  );
}

/// One face: the face colour, the inset rim at 5, a hairline at 9.5, and the
/// mono ZEC label. [cap] tints the back face's base a shade darker.
class _Face extends StatelessWidget {
  const _Face({
    required this.size,
    required this.colors,
    required this.label,
    this.cap,
  });

  final double size;
  final WalletColors colors;
  final TextStyle label;
  final Color? cap;

  @override
  Widget build(BuildContext context) {
    final unit = size / 84;
    return DecoratedBox(
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        color: cap ?? colors.coinFace,
      ),
      child: Padding(
        padding: EdgeInsets.all(5 * unit),
        child: DecoratedBox(
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            color: colors.coinFace,
            border: Border.all(color: colors.coinRim, width: 3 * unit),
          ),
          child: Padding(
            padding: EdgeInsets.all(4.5 * unit),
            child: DecoratedBox(
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                border: Border.all(
                  // The design's 35 % white hairline, from the card's
                  // on-surface colour.
                  color: colors.onDeep.withValues(alpha: 0.35),
                  width: 1 * unit,
                ),
              ),
              child: Center(child: Text('ZEC', style: label)),
            ),
          ),
        ),
      ),
    );
  }
}
