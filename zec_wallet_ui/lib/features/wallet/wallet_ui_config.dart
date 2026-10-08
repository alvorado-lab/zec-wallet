import 'package:flutter_riverpod/flutter_riverpod.dart';

/// HOST SEAM (S13 §1a MAJOR): services only the host can perform, which a
/// wallet screen offers when — and only when — the host supplied them.
///
/// Supplied by overriding [walletUiConfigProvider] at the host's
/// `ProviderScope` root — the SDK's one precedent for a behavioural host hook
/// (`walletHostTransportProvider`), not a widget parameter. The default is an
/// empty config: no hook, no button. The SDK bundles no plugin for either
/// (ADR-0531 posture); a host brings its own share sheet and settings opener.
///
/// Every call is made from a user's tap and wrapped by [callWalletHostHook]:
/// a host that throws gets an inline fault, never a crash on a wallet screen.
class WalletUiConfig {
  const WalletUiConfig({this.onShare, this.onOpenSettings});

  /// Open the platform share sheet with [text]. The SDK passes an address or a
  /// payment URI the user asked to share, and a fixed [subject] or none —
  /// never an amount, memo or label as the subject (S13 §1a, security LOW).
  final Future<void> Function(String text, {String? subject})? onShare;

  /// Open this app's page in the system settings — offered where the camera
  /// was refused, so the user can grant it (S13 §1.5).
  final Future<void> Function()? onOpenSettings;
}

/// The host's [WalletUiConfig]; an empty one unless the host overrides it.
final walletUiConfigProvider = Provider<WalletUiConfig>(
  (ref) => const WalletUiConfig(),
);

/// Run a host hook, reporting whether it completed. A throw (sync or async)
/// is contained here and reads as `false`, so the caller shows its inline
/// fault. The error is not logged: the hook's arguments are an address or a
/// payment URI (§5.4 render-never-log).
Future<bool> callWalletHostHook(Future<void> Function() call) async {
  try {
    await call();
    return true;
  } catch (_) {
    return false;
  }
}
