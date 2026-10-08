import 'package:flutter/material.dart';

import '../../core/theme/icons.dart';
import '../../shared/wallet_notice.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';

/// The §5.1 de-shield warning — a transparent output means the funds become
/// PUBLIC on the Zcash blockchain. Prominent (a card-notice, `warning` tone,
/// its title in orange; stage S11 C2) and honest (design invariant 5 —
/// metadata is as sensitive as content). It is the only thing between the
/// user and an irreversible public payment on Send, so it stays a card, never
/// a line.
///
/// SHARED so the Send review (a transparent recipient) and the Move-to-transparent
/// expert action (§3.2i-1 — a deliberate self-de-shield) render the SAME
/// disclosure primitive. The privacy LOSS is identical, but the WORDS aren't: a
/// payment to a third party ("recipient will be visible") reads wrong for a
/// self-transfer (the user IS the recipient). So the surface supplies its own
/// [title]/[body]; both default to the Send-flow (third-party payment) copy.
class DeshieldWarning extends StatelessWidget {
  const DeshieldWarning({super.key, this.title, this.body});

  /// Override copy for a non-payment de-shield (the Move-to-transparent
  /// self-transfer). `null` ⇒ the Send-flow (third-party payment) strings.
  final String? title;
  final String? body;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final t = title ?? l10n.walletSendDeshieldTitle;
    final b = body ?? l10n.walletSendDeshieldBody;
    return Semantics(
      container: true,
      excludeSemantics: true,
      label: '$t. $b',
      child: WalletNotice.card(
        tone: WalletNoticeTone.warning,
        glyph: WalletGlyph.transparent,
        title: t,
        message: b,
      ),
    );
  }
}
