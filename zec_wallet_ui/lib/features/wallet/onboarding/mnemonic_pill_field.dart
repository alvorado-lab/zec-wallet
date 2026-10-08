import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../core/theme/colors.dart';
import '../../../core/theme/icons.dart';
import '../../../core/theme/shapes.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'bip39_wordlist.dart';
import 'mnemonic_input.dart';

/// A recovery-phrase entry field that turns each word into a numbered PILL as
/// it is committed (BIP39 best practice): the word is VISIBLE (a seed must be
/// readable to be checked), a valid BIP39 word shows as a normal pill, an
/// unknown word shows in the error colour right away (not only on submit), and
/// an autocomplete row suggests matching words as you type. Paste-friendly: a
/// pasted phrase splits into pills.
///
/// KEY RESIDUE (§10): the words live only in this widget's State (committed
/// pills + the in-progress token) for the life of the restore screen — Dart
/// memory can't be zeroized; the smallest lifetime is the mitigation. Never
/// logged, never copied to the clipboard by us.
///
/// The parent owns nothing but the mirrored word list (via [onChanged]) — the
/// authoritative lowercase/trim still happens in the controller chokepoint.
class MnemonicPillField extends StatefulWidget {
  const MnemonicPillField({
    required this.onChanged,
    this.wordlist,
    this.enabled = true,
    super.key,
  });

  /// Fired whenever the committed word list changes (normalized: lowercase +
  /// trimmed, no empties). The in-progress token is NOT included until committed.
  final ValueChanged<List<String>> onChanged;

  /// The BIP39 wordlist for live validity + autocomplete; `null` while it loads
  /// (the field still works — validity/suggestions just stay quiet, and the SDK
  /// is the real gate on submit).
  final Bip39Wordlist? wordlist;

  final bool enabled;

  @override
  State<MnemonicPillField> createState() => _MnemonicPillFieldState();
}

class _MnemonicPillFieldState extends State<MnemonicPillField> {
  final List<String> _words = [];
  final TextEditingController _input = TextEditingController();
  final FocusNode _focus = FocusNode();

  @override
  void dispose() {
    _input.dispose();
    _focus.dispose();
    super.dispose();
  }

  void _emit() => widget.onChanged(List.unmodifiable(_words));

  /// Commit complete tokens out of the input as the user types a space / pastes.
  void _onChanged(String value) {
    if (value.contains(mnemonicAnyWhitespace)) {
      final parts = value.split(mnemonicWhitespaceRun);
      final endsWithSpace = mnemonicTrailingWhitespace.hasMatch(value);
      final toCommit = endsWithSpace
          ? parts
          : parts.sublist(0, parts.length - 1);
      final remainder = endsWithSpace ? '' : parts.last;
      var committed = false;
      for (final raw in toCommit) {
        final w = normalizeMnemonicWord(raw);
        // Hard-cap the pill count ABOVE the longest valid phrase so an over-paste
        // shows an invalid length (never a silent truncation to a valid-looking
        // 24) and the widget count stays bounded (low-memory DoS guard).
        if (w.isNotEmpty && _words.length < kMaxMnemonicWords) {
          _words.add(w);
          committed = true;
        }
      }
      _input.value = TextEditingValue(
        text: remainder,
        selection: TextSelection.collapsed(offset: remainder.length),
      );
      if (committed) _emit();
    }
    setState(() {}); // refresh suggestions + the in-progress validity
  }

  void _commitSuggestion(String word) {
    if (_words.length >= kMaxMnemonicWords) return;
    _words.add(normalizeMnemonicWord(word));
    _input.clear();
    _emit();
    setState(() {});
    _focus.requestFocus();
  }

  void _removeAt(int index) {
    _words.removeAt(index);
    _emit();
    setState(() {});
  }

  /// Backspace on an empty input pops the last pill back into the input for
  /// editing — the expected tag-input gesture.
  KeyEventResult _onKey(FocusNode node, KeyEvent event) {
    if (event is KeyDownEvent &&
        event.logicalKey == LogicalKeyboardKey.backspace &&
        _input.text.isEmpty &&
        _words.isNotEmpty) {
      final last = _words.removeLast();
      _input.value = TextEditingValue(
        text: last,
        selection: TextSelection.collapsed(offset: last.length),
      );
      _emit();
      setState(() {});
      return KeyEventResult.handled;
    }
    return KeyEventResult.ignored;
  }

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    final wordlist = widget.wordlist;
    final suggestions =
        wordlist?.suggestions(_input.text, limit: 4) ?? const <String>[];

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // The pill box — tap anywhere to focus the inline input.
        GestureDetector(
          onTap: widget.enabled ? () => _focus.requestFocus() : null,
          behavior: HitTestBehavior.opaque,
          child: Container(
            constraints: const BoxConstraints(minHeight: 120),
            padding: const EdgeInsets.all(10),
            decoration: BoxDecoration(
              color: widget.enabled
                  ? colors.bgCard
                  : colors.bgCard.withValues(alpha: 0.5),
              borderRadius: BorderRadius.circular(
                WalletShapes.of(context).field,
              ),
              border: Border.all(color: colors.border),
            ),
            child: Wrap(
              spacing: 8,
              runSpacing: 8,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                for (var i = 0; i < _words.length; i++)
                  _WordPill(
                    index: i + 1,
                    word: _words[i],
                    // Unknown until the list loads → neutral; then valid/invalid.
                    valid: wordlist?.isValid(_words[i]),
                    onRemove: widget.enabled ? () => _removeAt(i) : null,
                  ),
                // The inline editor for the next word.
                ConstrainedBox(
                  constraints: const BoxConstraints(
                    minWidth: 90,
                    maxWidth: 180,
                  ),
                  child: Focus(
                    onKeyEvent: _onKey,
                    child: TextField(
                      controller: _input,
                      focusNode: _focus,
                      enabled: widget.enabled,
                      autocorrect: false,
                      enableSuggestions: false,
                      // A keyboard that ignores the password variation must
                      // still not learn a recovery word.
                      enableIMEPersonalizedLearning: false,
                      // `null`, not the default empty list: any list opts the
                      // field into the platform autofill service, which may
                      // offer to save what is typed.
                      autofillHints: null,
                      smartDashesType: SmartDashesType.disabled,
                      smartQuotesType: SmartQuotesType.disabled,
                      textCapitalization: TextCapitalization.none,
                      keyboardType: TextInputType.visiblePassword,
                      // visiblePassword keyboard is plain ASCII + no autosuggest
                      // bar that could capture/restore a seed word.
                      inputFormatters: [
                        LengthLimitingTextInputFormatter(
                          kMaxMnemonicInputChars,
                        ),
                      ],
                      style: textTheme.bodyMedium,
                      decoration: InputDecoration(
                        isDense: true,
                        border: InputBorder.none,
                        contentPadding: EdgeInsets.zero,
                        hintText: _words.isEmpty
                            ? l10n.walletRestorePhraseHint
                            : null,
                      ),
                      onChanged: _onChanged,
                      // Enter commits the in-progress word (never submits).
                      onSubmitted: (v) {
                        if (v.trim().isNotEmpty) _onChanged('$v ');
                        _focus.requestFocus();
                      },
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
        // Autocomplete row — tappable BIP39 suggestions for the in-progress word.
        if (suggestions.isNotEmpty) ...[
          const SizedBox(height: 8),
          SizedBox(
            // Room for a 48px padded chip tap target (a11y minimum) + scale slack.
            height: 52,
            child: ListView.separated(
              scrollDirection: Axis.horizontal,
              itemCount: suggestions.length,
              separatorBuilder: (_, _) => const SizedBox(width: 8),
              itemBuilder: (context, i) => Center(
                child: ActionChip(
                  materialTapTargetSize: MaterialTapTargetSize.padded,
                  label: Text(suggestions[i]),
                  onPressed: widget.enabled
                      ? () => _commitSuggestion(suggestions[i])
                      : null,
                ),
              ),
            ),
          ),
        ],
      ],
    );
  }
}

/// A single numbered recovery-word pill. Valid BIP39 word → neutral/accent;
/// unknown word → error colour with a warning glyph (live typo feedback);
/// validity `null` (wordlist not loaded yet) → neutral, no claim either way.
class _WordPill extends StatelessWidget {
  const _WordPill({
    required this.index,
    required this.word,
    required this.valid,
    required this.onRemove,
  });

  final int index;
  final String word;
  final bool? valid;
  final VoidCallback? onRemove;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    final isInvalid = valid == false;
    final fg = isInvalid ? colors.orange : colors.text;
    final border = isInvalid ? colors.orange : colors.border;
    return Container(
      padding: EdgeInsets.only(left: 10, right: onRemove != null ? 0 : 12),
      decoration: BoxDecoration(
        color: colors.bgHover,
        borderRadius: BorderRadius.circular(WalletShapes.of(context).chip),
        border: Border.all(color: border),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          // The number + word read as ONE screen-reader node (container +
          // excludeSemantics) so VoiceOver doesn't merge them with the sibling
          // remove button (flutter-patterns § iOS Semantics merging).
          Semantics(
            container: true,
            excludeSemantics: true,
            label: isInvalid
                // Invalid pill: the typed value is deliberately NOT spoken (it
                // carries no verification value and keeps a mistyped recovery
                // word out of the OS accessibility tree — security HARDENING).
                ? l10n.walletRestorePillSemanticsInvalid(index)
                : l10n.walletRestorePillSemantics(index, word),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  '$index',
                  style: textTheme.bodySmall?.copyWith(color: colors.textDim),
                ),
                const SizedBox(width: 6),
                if (isInvalid) ...[
                  WalletIcon(WalletGlyph.error, size: 14, color: colors.orange),
                  const SizedBox(width: 3),
                ],
                Text(
                  word,
                  style: textTheme.bodyMedium?.copyWith(
                    color: fg,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ],
            ),
          ),
          if (onRemove != null)
            // Its own button node with a label; a 44×44 hit area (a11y minimum).
            Semantics(
              button: true,
              label: l10n.walletRestoreRemoveWord(index),
              child: InkResponse(
                onTap: onRemove,
                radius: 22,
                child: const SizedBox(
                  width: 44,
                  height: 44,
                  child: WalletIcon(WalletGlyph.close, size: 16),
                ),
              ),
            ),
        ],
      ),
    );
  }
}
