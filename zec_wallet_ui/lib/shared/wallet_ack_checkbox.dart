import 'package:flutter/material.dart';

/// The acknowledgement a user ticks before an action whose consequence the
/// screen has just disclosed: Swap's "the provider will see the information
/// above", and Send's "this payment will be public" ("add the
/// Send acknowledgement like Swap"). The action it guards stays disabled until
/// it is ticked; the caller owns the value and resets it when the thing it
/// acknowledges changes (a new quote, a new proposal, a new recipient).
///
/// The whole row is the tap target and carries the checkbox's own semantics
/// (a platform `CheckboxListTile`), so a screen reader hears the sentence and
/// its checked state as one control.
class WalletAckCheckbox extends StatelessWidget {
  const WalletAckCheckbox({
    super.key,
    required this.value,
    required this.onChanged,
    required this.label,
  });

  final bool value;
  final ValueChanged<bool> onChanged;

  /// Localized copy: the sentence the user agrees to.
  final String label;

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    return CheckboxListTile(
      value: value,
      onChanged: (v) => onChanged(v ?? false),
      controlAffinity: ListTileControlAffinity.leading,
      contentPadding: EdgeInsets.zero,
      title: Text(label, style: textTheme.bodyMedium),
    );
  }
}
