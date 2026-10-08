import 'package:flutter/material.dart';

import '../../core/theme/colors.dart';
import '../../shared/action_row_layout.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'hide_balance.dart';
import 'zat_format.dart';

/// One labelled money line — the label on the left, the integer-zatoshi amount as
/// an EXACT ZEC string on the right (never a float). `emphasize` bolds both label
/// and value for a headline/total row. `tint` recolours BOTH label and value (the
/// privacy-orange treatment for unshielded funds); it overrides the default
/// muted-label / plain-value pair.
///
/// The shared SSOT for every money breakdown row (the send confirm screen, the
/// Recv-3 shield sheet, the move-to-transparent sheet, and the wallet balance
/// card) so a money line — the numbers a user signs off on — can never drift in
/// layout or formatting between surfaces.
class LabeledZatRow extends StatelessWidget {
  const LabeledZatRow({
    super.key,
    required this.label,
    required this.amountZat,
    this.emphasize = false,
    this.tint,
    this.labelColor,
    this.hidden = false,
    this.sign = '',
  });

  final String label;

  /// A sign before the figure — `+` on the balance card's "Arriving" row,
  /// which is money on its way, not part of the figure above (S12 rev.3). It
  /// stays when the amount is masked: that the row exists already says funds
  /// are arriving.
  final String sign;
  final int amountZat;
  final bool emphasize;

  /// Optional emphasis colour applied to BOTH label and value (e.g. the privacy
  /// tint on unshielded funds). When null the default muted-label / plain-value
  /// pair is used.
  final Color? tint;

  /// The label's colour when it must differ from the value's, e.g. on the
  /// balance card's `deep` surface (`deepMuted` label over an `onDeep` value,
  /// the value then passed as [tint]). Wins over [tint] for the label only.
  final Color? labelColor;

  /// Hide balance (FR-49 W-7): show the amount as dots and announce it as
  /// hidden. OPT-IN per site and false by default, because the rows a user
  /// signs off on (the send review, the shield and move sheets) must never be
  /// masked.
  final bool hidden;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    final base = emphasize ? textTheme.titleMedium : textTheme.bodyMedium;
    final labelStyle = base?.copyWith(
      color: labelColor ?? tint ?? (emphasize ? colors.text : colors.textMuted),
    );
    final valueStyle = base?.copyWith(
      color: tint ?? colors.text,
      fontWeight: emphasize ? FontWeight.w600 : null,
    );
    final valueString = hidden
        ? '$sign$maskedAmountText'
        : l10n.walletAmount('$sign${formatZec(amountZat)}');
    // wallet-scale-probe: the TextPainter below must shape with the REAL
    // scaler to measure the amount; this is not a copy of the stacking rule
    // (that comes from walletTextScaleForcesStack, below).
    final textScaler = MediaQuery.textScalerOf(context);

    // The AMOUNT is the figure a user signs off on — it is NEVER clipped. A
    // horizontal label|value row holds it only while it fits; past that the
    // amount must move to its own full-width line. Decide that STRUCTURALLY, not
    // on a fixed scale guess, so the never-clip guarantee holds at every scale
    // and width (the pre-fix bug was a 29px overrun at ru/2.0×/320dp):
    //   - stack when the OS text scale pushes the money text past 1.4×
    //     (`walletTextScaleForcesStack` — the shared rule, which probes a
    //     SINGLE reference, the plain body-row size, for EVERY row: plain and
    //     emphasized alike flip together at the same OS setting rather than
    //     the small fee/change rows stacking before the larger total. That
    //     reasoning, first written here, is now the helper's own doc).
    //   - OR stack when the measured amount simply cannot share the row with
    //     even a heavily-shrunk label — this closes the residual overrun the
    //     scale probe alone leaves for an extreme (multi-million ZEC) figure
    //     just below the threshold.
    //
    // NOTE: like any Flexible-using row, this needs a BOUNDED-width parent; all
    // call sites (balance card + the confirm sheets) place it in a bounded
    // Column, and it is not publicly exported, so an unbounded embedding is not
    // reachable.
    final scaleStacked = walletTextScaleForcesStack(context);
    return MergeSemantics(
      child: LayoutBuilder(
        builder: (context, constraints) {
          const gap = 12.0;
          const minLabel = 32.0; // the label may ellipsise to a few glyphs
          // Measure only when the scale has NOT already forced the stack (on the
          // large-scale path the measure can't change the outcome — skip the
          // per-frame shaping). Measure with the EXACT style the Text will
          // render — the ambient DefaultTextStyle merged in and the OS bold-text
          // a11y setting applied — so the fit check equals the real width and
          // the 32px floor stays pure margin, never covering a measure/render
          // mismatch (a bold-text user renders wider than the raw style).
          var amountWontFit = false;
          if (!scaleStacked && constraints.maxWidth.isFinite) {
            var measureStyle = valueStyle;
            if (measureStyle == null || measureStyle.inherit) {
              measureStyle = DefaultTextStyle.of(
                context,
              ).style.merge(valueStyle);
            }
            if (MediaQuery.boldTextOf(context)) {
              measureStyle = measureStyle.merge(
                const TextStyle(fontWeight: FontWeight.bold),
              );
            }
            final painter = TextPainter(
              text: TextSpan(text: valueString, style: measureStyle),
              textDirection: Directionality.of(context),
              textScaler: textScaler,
              maxLines: 1,
            )..layout();
            amountWontFit =
                painter.width > constraints.maxWidth - gap - minLabel;
            painter.dispose();
          }
          final stacked = scaleStacked || amountWontFit;

          final labelText = Text(
            label,
            style: labelStyle,
            // In the stacked shape the label owns the full width; allow it a
            // second line before ellipsising so a long localised label is not
            // needlessly cut.
            maxLines: stacked ? 2 : 1,
            overflow: TextOverflow.ellipsis,
          );
          // A hidden amount is announced in words, never as its dots — and
          // never as its digits (it has none left to leak).
          final valueText = Text(
            valueString,
            style: valueStyle,
            semanticsLabel: hidden ? l10n.walletBalanceHiddenAmount : null,
          );

          if (stacked) {
            return Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                labelText,
                const SizedBox(height: 2),
                // Full width available now. Only an absurd figure at extreme
                // scale would still exceed it; FittedBox then scales the amount
                // down with EVERY DIGIT intact — a complete-but-smaller figure
                // is money-safe, a truncated one is not, so the shrink is left
                // unbounded on purpose (never trade a lost digit for size).
                FittedBox(
                  fit: BoxFit.scaleDown,
                  alignment: AlignmentDirectional.centerStart,
                  child: valueText,
                ),
              ],
            );
          }
          return Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              // The label flexes + ellipsises so the amount (inflexible, and
              // measured above to fit) is never clipped — a truncated figure on
              // a money line is a comprehension hazard. Protect the value.
              Flexible(child: labelText),
              const SizedBox(width: gap),
              valueText,
            ],
          );
        },
      ),
    );
  }
}
