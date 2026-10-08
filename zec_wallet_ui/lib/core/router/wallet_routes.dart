import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Route paths for the wallet feature — single source of truth; navigate via
/// these constants, never via string literals at call sites. A host mounts
/// [walletRoutes] (wallet_router.dart) under its own router; these paths are
/// the package's contract for navigation between the wallet's own surfaces.
/// Deliberately split from the route BUILDERS so feature screens can import
/// the constants without an import cycle through themselves.
abstract final class WalletRoutes {
  static const wallet = '/wallet';
  static const send = '/wallet/send';
  static const receive = '/wallet/receive';
  static const swap = '/wallet/swap';
  static const security = '/wallet/security';
  static const backup = '/wallet/security/backup';
  // #397 §3.7 D5: the sanctioned UFVK-export screen (gated at the same
  // re-auth bar as [backup]).
  static const exportViewingKey = '/wallet/security/export-viewing-key';
}

/// HOST SEAM: where the wallet's "Settings" affordances navigate (the
/// pre-onboarding settings button and the overflow-menu entry on the wallet
/// screen; the seam keeps its historical "appearance" name, S15). Null — the
/// default — HIDES both entry points: a host whose shell already owns its
/// settings simply doesn't wire this. The reference
/// consumer (the SDK example app) overrides it with its own
/// /settings/appearance route at the ProviderScope root.
final walletAppearanceRoutePathProvider = Provider<String?>((ref) => null);
