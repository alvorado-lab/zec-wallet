import 'package:flutter/services.dart';

/// The one input formatter for every amount field (S13 §1.1, revision 2 M1).
///
/// A keyboard in 11 of the 16 wallet locales offers `,` as its decimal key; the
/// old `[0-9.]` filter DELETED it, so "0,5" became "05". This formatter never
/// deletes: an edit is accepted whole or rejected whole (the field keeps its
/// old value), because a character silently dropped from money is how "1,000"
/// turns into 1 ZEC.
///
/// - `,` becomes `.` — but only as the field's one separator.
/// - A character outside `[0-9.,]`, or a second separator, rejects the edit.
/// - A comma after a non-zero whole part followed by exactly three digits
///   ("1,000") is refused as ambiguous: it reads as a thousands mark in half
///   the world. The field never HOLDS a comma (each one becomes a point as it
///   arrives), so a comma in an edit is always this edit's own — a paste, a
///   paste over a selection, or a comma typed into the middle of "1000". A
///   comma typed at the end ("1,") is converted before any digit follows it.
/// - Past [maxLength] the edit is refused whole, never truncated: cutting the
///   tail off "000000000000000001000" would leave a different number.
///
/// The parser (`parseZecAmount`) stays the real gate; this is the input layer.
class WalletDecimalInputFormatter extends TextInputFormatter {
  const WalletDecimalInputFormatter({this.maxLength});

  /// The longest text the field accepts; null for no bound.
  final int? maxLength;

  static final RegExp _allowed = RegExp(r'^[0-9.,]*$');
  static final RegExp _ambiguousThousands = RegExp(r'^0*[1-9][0-9]*,[0-9]{3}$');

  @override
  TextEditingValue formatEditUpdate(
    TextEditingValue oldValue,
    TextEditingValue newValue,
  ) {
    final text = newValue.text;
    final max = maxLength;
    if (max != null && text.length > max) return oldValue;
    if (!_allowed.hasMatch(text)) return oldValue;
    final separators = text.replaceAll(RegExp(r'[0-9]'), '').length;
    if (separators > 1) return oldValue;
    if (!text.contains(',')) return newValue;
    if (_ambiguousThousands.hasMatch(text)) return oldValue;
    // Same length, so the selection and composing ranges stay valid.
    return newValue.copyWith(text: text.replaceAll(',', '.'));
  }
}
