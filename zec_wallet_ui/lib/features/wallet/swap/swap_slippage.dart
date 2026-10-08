import 'package:flutter/material.dart';
import '../../../core/theme/colors.dart';
import '../../../shared/decimal_input_formatter.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';

/// User-tunable slippage tolerance (spec §3.3b D4). The SDK already carries
/// `slippage_tolerance_bps`; this exposes it as a host control — presets + custom
/// — with HONEST advisory tiers. The SDK is the authority: it enforces the hard
/// ceiling (`SLIPPAGE_MAX_BPS`) and the §2.6 user-anchored sanity ceiling bounds
/// every quote regardless, so these host values are a safe-default convenience,
/// never the drain guard. Slippage bps is a TOLERANCE, never funds math.

/// The SDK default (2%) — the form opens here.
const int kSlippageDefaultBps = 200;

/// The SDK hard ceiling (10%) — `SLIPPAGE_MAX_BPS` in the core `constants.rs`,
/// re-enforced Rust-side (`SlippageToleranceTooHigh` beyond). Mirrored here ONLY
/// to clamp the custom field for UX; the Rust gate is authoritative.
const int kSlippageMaxBps = 1000;

/// The preset chips (0.5% / 1% / 2% — Zodl parity, §3.3b D4).
const List<int> kSlippagePresetsBps = [50, 100, 200];

/// Below this, a swap "may fail" (too tight for normal market movement).
const int kSlippageMayFailBelowBps = 50;

/// Above this, a swap is "risky" (a wider drain-within-tolerance window).
const int kSlippageRiskyAboveBps = 300;

/// The honest advisory axis for a slippage value (§3.3b D4). Pure — the widget
/// maps it to an l10n line, exactly like the fault → message mapping.
enum SlippageTier {
  /// `< kSlippageMayFailBelowBps` — likely to fail on normal movement.
  mayFail,

  /// A normal, safe tolerance.
  normal,

  /// `> kSlippageRiskyAboveBps` (but within the ceiling) — a wider loss window.
  risky,

  /// `> kSlippageMaxBps` — the SDK rejects it (defensive; the field clamps).
  tooHigh,
}

/// Classify a basis-points tolerance into its advisory [SlippageTier]. Pure +
/// total, unit-tested at its boundaries.
SlippageTier slippageTier(int bps) {
  if (bps > kSlippageMaxBps) return SlippageTier.tooHigh;
  if (bps < kSlippageMayFailBelowBps) return SlippageTier.mayFail;
  if (bps > kSlippageRiskyAboveBps) return SlippageTier.risky;
  return SlippageTier.normal;
}

/// Parse a percent string (e.g. `"2.5"`) to basis points (250). Returns `null`
/// on an empty/malformed/negative entry (the caller keeps the last valid value).
/// Pure; no clamping — [slippageTier] + the SDK ceiling judge the result.
int? parseSlippagePercentToBps(String text) {
  final t = text.trim();
  if (t.isEmpty) return null;
  final pct = double.tryParse(t);
  if (pct == null || pct.isNaN || pct.isInfinite || pct < 0) return null;
  // bps = percent × 100; round to the nearest integer bp (a tolerance, not money).
  return (pct * 100).round();
}

/// Render a basis-points value as a trimmed percent string (200 → `"2"`, 250 →
/// `"2.5"`, 50 → `"0.5"`). Pure; used for chip labels + the custom field seed.
String formatBpsAsPercent(int bps) {
  final pct = bps / 100;
  // Trim a trailing `.0` so whole percents read "2", not "2.0".
  return pct == pct.roundToDouble() ? pct.toStringAsFixed(0) : pct.toString();
}

/// The slippage control (§3.3b D4): preset chips + a custom percent field, with
/// an honest advisory line. Stateless-in-value — the parent owns [valueBps] and
/// is told of changes through [onChanged]; the custom field's text is the only
/// local state. Used by BOTH directions (one shared control, §3.3b L8).
class SlippageControl extends StatefulWidget {
  const SlippageControl({
    super.key,
    required this.valueBps,
    required this.onChanged,
  });

  /// The current tolerance (basis points). A preset selects exactly; the custom
  /// field can set any value (the SDK ceiling rejects an over-the-line one).
  final int valueBps;
  final ValueChanged<int> onChanged;

  @override
  State<SlippageControl> createState() => _SlippageControlState();
}

class _SlippageControlState extends State<SlippageControl> {
  late final TextEditingController _customController;

  /// True once the user picks "Custom" (or starts on a non-preset value) — the
  /// percent field is shown and the custom chip is selected.
  bool _customMode = false;

  @override
  void initState() {
    super.initState();
    _customMode = !kSlippagePresetsBps.contains(widget.valueBps);
    _customController = TextEditingController(
      text: _customMode ? formatBpsAsPercent(widget.valueBps) : '',
    );
  }

  @override
  void dispose() {
    _customController.dispose();
    super.dispose();
  }

  void _selectPreset(int bps) {
    setState(() => _customMode = false);
    widget.onChanged(bps);
  }

  void _enterCustomMode() {
    setState(() {
      _customMode = true;
      // Seed the field from the current value so the custom chip doesn't reset it.
      _customController.text = formatBpsAsPercent(widget.valueBps);
    });
  }

  void _onCustomChanged(String text) {
    final bps = parseSlippagePercentToBps(text);
    // Clamp the upper bound for UX (the SDK still re-checks); an unparsable entry
    // keeps the last valid value (no silent reset to default while typing).
    if (bps == null) return;
    widget.onChanged(bps > kSlippageMaxBps ? kSlippageMaxBps : bps);
  }

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          l10n.walletSwapSlippageLabel,
          style: textTheme.bodyMedium?.copyWith(color: colors.text),
        ),
        const SizedBox(height: 8),
        Wrap(
          spacing: 8,
          children: [
            for (final preset in kSlippagePresetsBps)
              ChoiceChip(
                label: Text(
                  l10n.walletSwapSlippagePercent(formatBpsAsPercent(preset)),
                ),
                selected: !_customMode && widget.valueBps == preset,
                onSelected: (_) => _selectPreset(preset),
              ),
            ChoiceChip(
              label: Text(l10n.walletSwapSlippageCustom),
              selected: _customMode,
              onSelected: (_) => _enterCustomMode(),
            ),
          ],
        ),
        if (_customMode) ...[
          const SizedBox(height: 8),
          TextField(
            controller: _customController,
            keyboardType: const TextInputType.numberWithOptions(decimal: true),
            inputFormatters: const [WalletDecimalInputFormatter()],
            onChanged: _onCustomChanged,
            decoration: InputDecoration(
              labelText: l10n.walletSwapSlippageCustomLabel,
              suffixText: '%',
            ),
          ),
        ],
        const SizedBox(height: 6),
        _SlippageAdvisory(bps: widget.valueBps),
      ],
    );
  }
}

/// The honest advisory line under the control — green/normal, or an orange
/// caution for a too-tight / too-wide value. Never blocks (the SDK is the gate);
/// it informs.
class _SlippageAdvisory extends StatelessWidget {
  const _SlippageAdvisory({required this.bps});

  final int bps;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    final tier = slippageTier(bps);

    final (message, tint) = switch (tier) {
      SlippageTier.mayFail => (l10n.walletSwapSlippageMayFail, colors.orange),
      SlippageTier.normal => (l10n.walletSwapSlippageNormal, colors.textMuted),
      SlippageTier.risky => (l10n.walletSwapSlippageRisky, colors.orange),
      SlippageTier.tooHigh => (l10n.walletSwapSlippageTooHigh, colors.red),
    };

    return Text(message, style: textTheme.bodySmall?.copyWith(color: tint));
  }
}
