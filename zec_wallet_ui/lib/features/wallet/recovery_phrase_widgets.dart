import 'package:flutter/material.dart';

import '../../core/theme/colors.dart';
import '../../core/theme/icons.dart';
import '../../core/theme/shapes.dart';
import '../../core/theme/typography.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';

/// Presentational chrome shared by every recovery-phrase REVEAL surface — the
/// onboarding backup step ([WalletBackupView]) and the post-onboarding backup
/// screen (Settings → Security → back up). Extracted so the two surfaces render
/// the words and the security note IDENTICALLY (one source of truth for the
/// seed-display look) instead of drifting two copies.
///
/// SECURITY POSTURE (unchanged by the extraction): these are pure renderers —
/// they hold no state and read no provider. The word list is passed in by the
/// caller, which owns the §10 key-residue minimisation (the caller reads the
/// words from the `autoDispose` reveal future and drops them the moment its
/// screen unmounts). Nothing here copies, persists, or logs the words.

/// The numbered recovery words in index order, as a chip GRID (S13 Build B;
/// the design-refresh S-RECOVERY layout): equal-width columns read row by row
/// (1 2 3 / 4 5 6), up to [maxColumns]. The column count comes from the width
/// the grid is given and the text scale, so it composes at any width and scale
/// (never overflows — the canonical desktop+mobile+a11y concern): a narrow
/// window or a large text scale drops to two columns, then one. A word is
/// never clipped — past its chip it scales down whole. Each chip also carries
/// its 1-based index. Plain [Text] — deliberately NOT [SelectableText]: a seed
/// must never reach the (shared/synced) clipboard.
class RecoveryWordGrid extends StatelessWidget {
  const RecoveryWordGrid({super.key, required this.words});

  final List<String> words;

  /// The most columns the grid draws.
  static const int maxColumns = 3;

  /// A chip's narrowest width at a 1.0 text scale; it grows with the scale.
  static const double minChipWidth = 104;

  /// The gap between chips, both axes.
  static const double gap = 8;

  /// The column count for [maxWidth] at [textScaler]: as many chips of the
  /// scaled [minChipWidth] as fit, from one up to [maxColumns].
  // wallet-scale-probe: the word grid's column count scales a chip's width; it is not an action-row stack
  static int columnsFor(double maxWidth, TextScaler textScaler) {
    if (!maxWidth.isFinite) return maxColumns;
    // wallet-scale-probe: a chip's width for the column count, not a stack
    final chip = textScaler.scale(minChipWidth);
    return ((maxWidth + gap) / (chip + gap)).floor().clamp(1, maxColumns);
  }

  @override
  Widget build(BuildContext context) {
    // wallet-scale-probe: feeds columnsFor only
    final textScaler = MediaQuery.textScalerOf(context);
    return LayoutBuilder(
      builder: (context, constraints) {
        final columns = columnsFor(constraints.maxWidth, textScaler);
        final rows = (words.length / columns).ceil();
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            for (var r = 0; r < rows; r++)
              Padding(
                padding: EdgeInsets.only(top: r == 0 ? 0 : gap),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    for (var c = 0; c < columns; c++) ...[
                      if (c > 0) const SizedBox(width: gap),
                      Expanded(
                        child: r * columns + c < words.length
                            ? _WordChip(
                                index: r * columns + c,
                                word: words[r * columns + c],
                              )
                            : const SizedBox.shrink(),
                      ),
                    ],
                  ],
                ),
              ),
          ],
        );
      },
    );
  }
}

/// One numbered word of [RecoveryWordGrid].
class _WordChip extends StatelessWidget {
  const _WordChip({required this.index, required this.word});

  /// The word's 0-based position; drawn 1-based.
  final int index;
  final String word;

  @override
  Widget build(BuildContext context) {
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    final l10n = WalletLocalizations.of(context);
    final i = index;
    // The index + word read as ONE screen-reader node ("word 3: abandon")
    // — the same combined-label pattern the restore pill field uses
    // (mnemonic_pill_field.dart) so a blind user can transcribe the phrase
    // IN ORDER (the copy's hard requirement), not as ~2N unanchored stops.
    // Reuses the shared `walletRestorePillSemantics` label (identical
    // "word {index}: {word}" string, already localised). NOTE: this keeps
    // the words in the accessibility tree — the deliberate, established
    // choice for recovery words here. The security-vs-accessibility
    // tradeoff (FLAG_SECURE does NOT block an accessibility-service scrape,
    // a mainstream Android banking-malware vector) was taken to the product
    // owner and RESOLVED as keep-as-is: denying a blind user an ordered
    // seed backup is a definite harm, whereas the scrape presupposes an
    // already-compromised device running a11y-malware that has other
    // vectors. The alternative — excluding the WHOLE seed surface (this
    // grid AND the restore pill field, mnemonic_pill_field.dart) from the
    // default semantics tree behind a deliberate "read my phrase aloud"
    // opt-in — is MAINTAINER-SCHEDULED POST-GA HARDENING (decision
    // 2026-07-20, rides the #377 polish tail): the device
    // walk showed ONE stock `uiautomator dump` harvests the words with
    // no malware at all, which moved the tradeoff from "revisit if the
    // threat model demands" to "build the opt-in after GA". Until then
    // this exposure is a KNOWN, deliberately-shipped tradeoff — not a
    // finding for future review passes to re-open pre-GA.
    return Semantics(
      container: true,
      excludeSemantics: true,
      label: l10n.walletRestorePillSemantics(i + 1, word),
      child: Container(
        key: ValueKey('recovery-word-chip-${i + 1}'),
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
        decoration: BoxDecoration(
          color: colors.bgCard,
          borderRadius: BorderRadius.circular(WalletShapes.of(context).chip),
          border: Border.all(color: colors.border),
        ),
        child: Row(
          children: [
            Text(
              '${i + 1}',
              maxLines: 1,
              style: textTheme.bodySmall?.copyWith(color: colors.textDim),
            ),
            const SizedBox(width: 6),
            // The word is never clipped: past the chip it scales down whole.
            Flexible(
              child: FittedBox(
                fit: BoxFit.scaleDown,
                alignment: AlignmentDirectional.centerStart,
                child: Text(
                  word,
                  maxLines: 1,
                  // The design's mono chips (S-RECOVERY): the host's mono face.
                  style: WalletTypography.of(context).monoOn(
                    (textTheme.bodyMedium ?? const TextStyle()).copyWith(
                      color: colors.text,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// The security note over the sensitive screens. Honest and ACK-DRIVEN, not
/// platform-driven: [blocksScreenshots] is true only after the NATIVE side
/// confirmed the block engaged ([ScreenSecurity.enable] returned `true` —
/// review B1), so "screenshots are off" is a claim the OS actually backed.
/// Everywhere else — a non-Android platform, an unwired host handler, a debug
/// deferral — it advises a private setting instead of implying a protection
/// that is not running (design invariant 6; flutter-patterns § Platform
/// Capability Gating — FLAG_SECURE is Android-only).
class RecoveryPhraseSecureNote extends StatelessWidget {
  const RecoveryPhraseSecureNote({super.key, required this.blocksScreenshots});

  final bool blocksScreenshots;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    final text = blocksScreenshots
        ? l10n.walletBackupSecureNoteAndroid
        : l10n.walletBackupSecureNoteOther;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        WalletIcon(WalletGlyph.locked, size: 16, color: colors.textMuted),
        const SizedBox(width: 6),
        Expanded(
          child: Text(
            text,
            style: textTheme.bodySmall?.copyWith(color: colors.textMuted),
          ),
        ),
      ],
    );
  }
}
