import 'package:flutter/material.dart';

import '../core/theme/colors.dart';
import '../core/theme/icons.dart';
import '../core/theme/shapes.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'action_row_layout.dart';

/// What a [WalletNotice] is saying, carried by its glyph's colour (and, on the
/// card form, by the title's for [warning] and [danger]). The words carry the
/// meaning; the tone is never the only signal.
enum WalletNoticeTone { info, positive, warning, danger }

/// The wallet's one notice (FR-49 W-4, `DESIGN.md` §6.10, stage S11 C2), in
/// two forms:
///
/// * **the line** ([WalletNotice.new]): a system line on a `bgHover` fill at
///   [WalletShapes.notice], a 16 glyph, one sentence, and optional inline text
///   actions. No title.
/// * **the card** ([WalletNotice.card]): a `bgCard` group at
///   [WalletShapes.group] (`bgHover` when it sits on a `bgCard` surface, such
///   as a sheet), a 20 glyph beside an optional title, the message,
///   an optional [child], then its buttons. For the warnings whose words say
///   what is at stake (a public payment, a memo that must not be lost).
///
/// **Neither form draws a border or a coloured fill.** §6.10 forbids both: the
/// tone sits on the glyph, and on the card's title for a warning or danger.
///
/// [message] and [title] are LOCALIZED COPY — never a wallet value. Content a
/// `String` cannot carry (a deposit memo in the mono face, with its own Copy
/// button) goes in [child], never in [message].
class WalletNotice extends StatelessWidget {
  /// The line form.
  const WalletNotice({
    super.key,
    required this.tone,
    required this.glyph,
    required this.message,
    this.actions = const [],
    this.liveRegion = false,
    this.onDismiss,
    this.dismissLabel,
    this.child,
    this.messageInTone = false,
  }) : title = null,
       _card = false;

  /// The card form.
  const WalletNotice.card({
    super.key,
    required this.tone,
    required this.glyph,
    required this.message,
    this.title,
    this.actions = const [],
    this.liveRegion = false,
    this.onDismiss,
    this.dismissLabel,
    this.child,
  }) : messageInTone = false,
       _card = true;

  final WalletNoticeTone tone;
  final WalletGlyph glyph;

  /// Localized copy only (see the class doc).
  final String message;

  /// The card's title, drawn in the tone colour for a warning or danger. Null
  /// on the line, and optional on the card.
  final String? title;

  /// On the line: text buttons, inline beside the message until
  /// [walletRowStacksAction] moves them under it. On the card: §6.7 buttons,
  /// wrapped under the message and [child].
  final List<Widget> actions;

  /// Announce the notice when it appears or its words change: set it for a
  /// notice that arrives after an async step (a fault, a refusal).
  final bool liveRegion;

  /// Draws a dismiss button when set.
  final VoidCallback? onDismiss;

  /// The dismiss button's label; defaults to the wallet's "Dismiss".
  final String? dismissLabel;

  /// Drawn below the message, for content a `String` cannot carry.
  final Widget? child;

  /// The line only: draw the message in the tone colour too (the transparent
  /// arm of the privacy statement, whose words are the warning).
  final bool messageInTone;

  final bool _card;

  /// The glyph's colour for [tone].
  static Color toneColor(WalletColors colors, WalletNoticeTone tone) =>
      switch (tone) {
        WalletNoticeTone.info || WalletNoticeTone.positive => colors.accent,
        WalletNoticeTone.warning => colors.orange,
        WalletNoticeTone.danger => colors.red,
      };

  @override
  Widget build(BuildContext context) {
    // A container node, so the message's words merge into the node that
    // carries the live-region flag (a screen reader hears the words, not an
    // empty region), and the actions stay their own nodes.
    return Semantics(
      container: true,
      liveRegion: liveRegion,
      child: _card ? _buildCard(context) : _buildLine(context),
    );
  }

  Widget? _dismiss(BuildContext context, WalletColors colors) {
    final onDismiss = this.onDismiss;
    if (onDismiss == null) return null;
    return IconButton(
      icon: const WalletIcon(WalletGlyph.close, size: 18),
      visualDensity: VisualDensity.compact,
      tooltip:
          dismissLabel ??
          WalletLocalizations.of(context).walletDeepScanRestoreNoteDismiss,
      color: colors.textMuted,
      onPressed: onDismiss,
    );
  }

  Widget _buildLine(BuildContext context) {
    final colors = WalletColors.of(context);
    final shapes = WalletShapes.of(context);
    final textTheme = Theme.of(context).textTheme;
    final tint = toneColor(colors, tone);
    final dismiss = _dismiss(context, colors);
    final text = Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(
          message,
          style: textTheme.bodyMedium?.copyWith(
            color: messageInTone ? tint : colors.text,
          ),
        ),
        if (child != null) ...[const SizedBox(height: 8), child!],
      ],
    );
    return DecoratedBox(
      decoration: BoxDecoration(
        color: colors.bgHover,
        borderRadius: BorderRadius.circular(shapes.notice),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 9, horizontal: 14),
        // The row's OWN width, not the screen's: a host embedding the wallet
        // in a split view or a sheet is judged on the space this line has.
        child: LayoutBuilder(
          builder: (context, constraints) {
            final stack =
                actions.isNotEmpty &&
                walletRowStacksAction(context, constraints.maxWidth);
            final row = Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Padding(
                  // Centres the 16 glyph on the first line of body text.
                  padding: const EdgeInsets.only(top: 2),
                  child: ExcludeSemantics(
                    child: WalletIcon(glyph, size: 16, color: tint),
                  ),
                ),
                const SizedBox(width: 10),
                Expanded(child: text),
                if (!stack)
                  for (final action in actions) ...[
                    const SizedBox(width: 8),
                    action,
                  ],
                ?dismiss,
              ],
            );
            if (!stack) return row;
            return Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                row,
                const SizedBox(height: 4),
                // DIRECTIONAL: the package ships ar + he, where the actions
                // belong on the other edge (the #400 R9 rule).
                Align(
                  alignment: AlignmentDirectional.centerEnd,
                  child: Wrap(
                    alignment: WrapAlignment.end,
                    crossAxisAlignment: WrapCrossAlignment.center,
                    spacing: 8,
                    children: actions,
                  ),
                ),
              ],
            );
          },
        ),
      ),
    );
  }

  Widget _buildCard(BuildContext context) {
    final colors = WalletColors.of(context);
    final shapes = WalletShapes.of(context);
    final textTheme = Theme.of(context).textTheme;
    final tint = toneColor(colors, tone);
    final dismiss = _dismiss(context, colors);
    // The security review's M1: a warning or danger says so in its title too,
    // not only in a 20 dp glyph.
    final titleColor = switch (tone) {
      WalletNoticeTone.warning || WalletNoticeTone.danger => tint,
      WalletNoticeTone.info || WalletNoticeTone.positive => colors.text,
    };
    // The card must stand off whatever it sits on, having no border (§6.10):
    // `bgCard` on a page, but one step up (`bgHover`, §6.1's recessed
    // surface) on a sheet or a card, which are `bgCard` themselves. Without
    // this, the deshield warning on the Move-to-transparent sheet had no edge
    // at all, just before an irreversible de-shield (S11 diff review, M-1).
    final surface = Material.maybeOf(context)?.color;
    final fill = surface == colors.bgCard ? colors.bgHover : colors.bgCard;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: fill,
        borderRadius: BorderRadius.circular(shapes.group),
      ),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                ExcludeSemantics(
                  child: WalletIcon(glyph, size: 20, color: tint),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      if (title != null) ...[
                        Text(
                          title!,
                          style: textTheme.titleMedium?.copyWith(
                            color: titleColor,
                          ),
                        ),
                        const SizedBox(height: 4),
                      ],
                      Text(
                        message,
                        style: textTheme.bodyMedium?.copyWith(
                          color: colors.text,
                        ),
                      ),
                    ],
                  ),
                ),
                ?dismiss,
              ],
            ),
            if (child != null) ...[const SizedBox(height: 12), child!],
            if (actions.isNotEmpty) ...[
              const SizedBox(height: 12),
              Wrap(spacing: 8, runSpacing: 8, children: actions),
            ],
          ],
        ),
      ),
    );
  }
}
