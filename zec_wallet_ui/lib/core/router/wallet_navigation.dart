import 'package:flutter/widgets.dart';
import 'package:go_router/go_router.dart';

import 'wallet_routes.dart';

/// Navigation helpers shared by the wallet's pushed money surfaces (send, swap,
/// the recovery-phrase backup screen). One source of truth for "leave this
/// screen" so the exit logic can't drift between screens (#333 nav layering).
extension WalletScreenExit on BuildContext {
  /// Leave a pushed wallet screen: pop back to the surface that pushed it, or —
  /// when this screen IS the navigation root (a host deep-link that mounted it
  /// with an empty back stack) — navigate to the wallet root instead.
  ///
  /// Money-safety: an exit / "Done" control on a send, swap, or seed-backup
  /// surface must always DO something. A bare `pop()` on an empty stack is a
  /// silent no-op that strands the user on the money screen with no way home;
  /// the `canPop` fallback guarantees the affordance is never dead.
  void leaveToWalletRoot() {
    if (canPop()) {
      pop();
    } else {
      go(WalletRoutes.wallet);
    }
  }
}
