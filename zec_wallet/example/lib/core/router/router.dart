import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:zec_wallet_ui/zec_wallet_ui.dart';

import '../../features/diagnostics/incoming_events_screen.dart';
import '../../features/diagnostics/machine_memo_screen.dart';
import '../../features/diagnostics/prefilled_send_screen.dart';
import '../../features/settings/appearance_screen.dart';
import 'route_error_screen.dart';

/// App-shell route paths — single source of truth; navigate via these
/// constants, never via string literals at call sites. The wallet feature's
/// paths live with the feature (`WalletRoutes`, zec_wallet_ui).
abstract final class AppRoutes {
  static const appearance = '/settings/appearance';
  static const incomingEvents = '/diagnostics/incoming-events';
  static const prefilledSend = '/diagnostics/prefilled-send';
  static const machineMemo = '/diagnostics/machine-memo';
}

/// The root navigator, so a step that runs outside any widget — the Tor
/// plugin's offer to reset its identity after a wallet delete, which the
/// provisioner runs — can show a dialog over whatever screen is up.
final rootNavigatorKey = GlobalKey<NavigatorState>();

/// App navigation: the wallet package's route fragments mounted under the
/// example's own router — exactly the host contract (`walletRoutes()` +
/// shell-owned extras). The shell/tab structure lands with Messaging UX
/// (app-frame spec §11 A3) — until then a flat route table is the honest
/// minimum. NO `redirect` yet; when one lands it runs on EVERY navigation
/// and must stay fast (flutter-patterns § Gotchas).
final routerProvider = Provider<GoRouter>((ref) {
  final router = GoRouter(
    navigatorKey: rootNavigatorKey,
    // The wallet IS the home of this example app (the SDK is a wallet, so the
    // example launches straight into it — no app-frame landing screen).
    initialLocation: WalletRoutes.wallet,
    routes: [
      GoRoute(
        path: AppRoutes.appearance,
        builder: (context, state) => const AppearanceScreen(),
      ),
      // Diagnostics (ADR-0536 #392): the live incoming-funds event stream —
      // the example's host-consumer proof; entry lives on the Appearance
      // screen's Developer section.
      GoRoute(
        path: AppRoutes.incomingEvents,
        builder: (context, state) => const IncomingEventsScreen(),
      ),
      // Diagnostics (FR-25 #360): the prefilled-send seam demo — the example's
      // host-consumer proof for WalletSendEntry.push / WalletSendRequest.fromUri.
      GoRoute(
        path: AppRoutes.prefilledSend,
        builder: (context, state) => const PrefilledSendScreen(),
      ),
      // Diagnostics (FR-27/28): the machine-memo round trip — the example's
      // host-consumer proof for composePaymentUri(memoBytes:) + machineMemos.
      GoRoute(
        path: AppRoutes.machineMemo,
        builder: (context, state) => const MachineMemoScreen(),
      ),
      ...walletRoutes(),
    ],
    // Unknown route → recoverable, plain-language error with a next step
    // (never codes; app-frame spec §6).
    errorBuilder: (context, state) => const RouteErrorScreen(),
  );
  // Root-scoped today, but the dispose contract stays explicit so the
  // provider can be scoped/overridden in tests safely (arch review fold).
  ref.onDispose(router.dispose);
  return router;
});
