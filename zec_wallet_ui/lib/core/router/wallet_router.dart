import 'package:go_router/go_router.dart';

import '../../features/settings/backup_screen.dart';
import '../../features/settings/export_viewing_key_screen.dart';
import '../../features/settings/security_screen.dart';
import '../../features/wallet/receive_screen.dart';
import '../../features/wallet/send/send_screen.dart';
import '../../features/wallet/send/wallet_send_request.dart';
import '../../src/send/wallet_send_channel.dart';
import '../../features/wallet/swap/swap_screen.dart';
import '../../features/wallet/wallet_screen.dart';
import 'wallet_routes.dart';

/// The wallet feature's route fragments. A host mounts them under its own
/// GoRouter — `GoRouter(routes: [...walletRoutes(), ...hostRoutes])` — and
/// keeps shell concerns (tabs, its own settings, error surfaces) host-side.
/// Every screen re-gates on live state itself, so a deep link into any of
/// these with no wallet renders that screen's honest unavailable state
/// rather than trusting the router to have guarded it (money-safety: the
/// gate lives with the money surface, not the navigation).
List<RouteBase> walletRoutes() => [
  GoRoute(
    path: WalletRoutes.wallet,
    builder: (context, state) => const WalletScreen(),
  ),
  GoRoute(
    // Pushed from the active wallet surface (a back button returns to it);
    // the send screen re-gates on a live session, so a deep-link here with
    // no wallet renders its honest "go back" state (money-safety). An FR-25
    // prefilled entry (WalletSendEntry.push) rides a WalletSendEntryArgs as
    // `extra` — the request plus the FR-26 report channel; a BARE
    // WalletSendRequest is still accepted (a host that navigates the send
    // path itself), and simply gets no report. Organic entry passes neither
    // → a fresh empty form.
    path: WalletRoutes.send,
    builder: (context, state) => switch (state.extra) {
      WalletSendEntryArgs(:final request, :final reporter) => SendScreen(
        prefill: request,
        reporter: reporter,
      ),
      final WalletSendRequest request => SendScreen(prefill: request),
      _ => const SendScreen(),
    },
  ),
  GoRoute(
    // Pushed from the active wallet surface; the receive screen re-gates
    // on a live session (a deep-link with no wallet renders its honest
    // unavailable state). The address is public, so no money-safety
    // concern beyond that.
    path: WalletRoutes.receive,
    builder: (context, state) => const ReceiveScreen(),
  ),
  GoRoute(
    // Pushed from the active wallet surface ONLY when the host has enabled
    // swap; the swap screen re-gates on a live session AND the host
    // swap-enabled state, so a deep-link with swap off renders its honest
    // unavailable state (§3.5 UI isolation).
    path: WalletRoutes.swap,
    builder: (context, state) => const SwapScreen(),
  ),
  GoRoute(
    // Pushed from the active wallet overflow menu — key custody + the
    // delete-wallet crypto-shred (FR-14). On a successful shred it
    // navigates back to the wallet route, which re-renders onboarding.
    path: WalletRoutes.security,
    builder: (context, state) => const SecurityScreen(),
  ),
  GoRoute(
    // Pushed from Settings → Security ("Back up recovery phrase"); the backup
    // screen re-gates on a live provisioner itself — a deep-link with no
    // wallet-local phrase (session-only host) renders its honest managed-by-host
    // state rather than a reveal button that can't produce words (#333).
    path: WalletRoutes.backup,
    builder: (context, state) => const BackupScreen(),
  ),
  GoRoute(
    // #397 §3.7 D5: pushed from Settings → Security ("Export viewing key").
    // Since G1 (#361 companion) it serves BOTH custody shapes — the provisioner
    // when the package owns provisioning, else the session port — so a
    // session-only host that PUSHES this route gets a working, D9-gated export
    // rather than the pre-G1 managed-by-host dead end. Only a mount with NEITHER
    // seam renders the terminal state. ⚠ A session-only host mounting this route
    // must also override `walletRevealAuthorizerProvider`: the package default is
    // PASS-THROUGH, so without it one tap stands between anyone holding the
    // unlocked device and the viewing key.
    path: WalletRoutes.exportViewingKey,
    builder: (context, state) => const ExportViewingKeyScreen(),
  ),
];
