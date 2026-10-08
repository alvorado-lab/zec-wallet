import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Hide balance (FR-49 W-7, ADR-0564 ruling B): the eye in the wallet header
/// replaces every amount a bystander can read with [maskedAmountText], so a
/// wallet can be opened in public without showing what it holds.
///
/// Session-only by default (false at each launch). A host that wants the
/// choice to persist overrides this provider with its own notifier; Relim
/// reads it for its in-chat payment rows too.
///
/// What it NEVER hides: an amount the user is TYPING, and the totals under
/// review before Confirm — hiding what you are about to sign is a
/// spend-safety defect, not privacy.
final walletBalanceHiddenProvider = NotifierProvider<WalletBalanceHidden, bool>(
  WalletBalanceHidden.new,
);

/// The hide-balance switch.
class WalletBalanceHidden extends Notifier<bool> {
  @override
  bool build() => false;

  /// Flip hidden ↔ shown.
  void toggle() => state = !state;

  /// Set it outright (a host restoring its persisted choice).
  void set(bool hidden) => state = hidden;
}

/// What a hidden amount shows in place of its digits: four bullets, no unit,
/// no sign, so no DIGIT and no size of a figure leaks. It does not hide
/// everything a bystander can infer: a row's Received/Sent title, glyph and
/// colour still say which way money moved, and which card rows render still
/// says that funds are pending or transparent.
const maskedAmountText = '••••';

/// [amountText] as displayed: itself, or [maskedAmountText] while hidden.
String displayedAmount(String amountText, {required bool hidden}) =>
    hidden ? maskedAmountText : amountText;
