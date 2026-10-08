import 'package:flutter/widgets.dart';

import 'wallet_localizations.dart';

/// The delegate a HOST should compose into `localizationsDelegates` —
/// [WalletLocalizations.delegate] with an ENGLISH FALLBACK for locales the
/// wallet ARB set doesn't cover yet.
///
/// WHY THIS EXISTS (extraction review B2): the generated delegate is
/// built from `nullable-getter: false` and supports only the shipped
/// locales (en today). A host whose `supportedLocales` is wider (say
/// `[en, fr]`) on a French device would have the raw delegate SKIPPED by
/// `Localizations` (`isSupported == false`) — and then the first wallet frame
/// throws on `WalletLocalizations.of(context)`. Every money surface would be
/// down, only for non-English users, only in hosts. This wrapper accepts every
/// locale and serves the closest shipped ARB instead (exact locale → its ARB;
/// anything else → English), so an untranslated locale reads English rather
/// than crashing.
///
/// Hand-written on purpose: gen-l10n owns wallet_localizations*.dart and
/// regenerates them; this sibling survives regeneration. A host adding its own
/// `wallet_<locale>.arb` files needs no change here — the wrapper defers to the
/// generated delegate whenever it really supports the locale.
const LocalizationsDelegate<WalletLocalizations>
walletLocalizationsFallbackDelegate = _WalletLocalizationsFallbackDelegate();

class _WalletLocalizationsFallbackDelegate
    extends LocalizationsDelegate<WalletLocalizations> {
  const _WalletLocalizationsFallbackDelegate();

  @override
  bool isSupported(Locale locale) => true;

  @override
  Future<WalletLocalizations> load(Locale locale) =>
      WalletLocalizations.delegate.isSupported(locale)
      ? WalletLocalizations.delegate.load(locale)
      : WalletLocalizations.delegate.load(const Locale('en'));

  @override
  bool shouldReload(_WalletLocalizationsFallbackDelegate old) => false;
}
