/// A spinner with nothing beside it to say what it is (S13 §1.7).
///
/// NOT exported: internal, like `sheet_states.dart`.
library;

import 'package:flutter/material.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';

/// The platform's spinner, read aloud as "Loading" (the maintainer's word, S13
/// §2). For a spinner that stands ALONE; one beside a label line needs no
/// name of its own.
///
/// The name rides a [Semantics] wrapper rather than the indicator's own
/// `semanticsLabel`: on iOS and macOS the adaptive indicator builds a
/// Cupertino activity indicator and drops that label.
class WalletLoadingIndicator extends StatelessWidget {
  const WalletLoadingIndicator({super.key});

  @override
  Widget build(BuildContext context) {
    return Semantics(
      label: WalletLocalizations.of(context).walletLoadingLabel,
      child: const CircularProgressIndicator.adaptive(),
    );
  }
}
