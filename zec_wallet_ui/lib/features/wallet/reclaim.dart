import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import '../../shared/wallet_dialog.dart';
import 'send_authorization.dart';
import 'wallet_providers.dart';

/// §3.2i-2 2e-2b-vi (#315 slice 2) — the "reopen one-time-address sending" flow.
///
/// A one-time-address (TEX) send window can be bricked by LEAKED reservations —
/// sends that reserved a one-time address but never confirmed, which the wallet
/// cannot free on its own. The reclaim self-mints a small amount from the
/// wallet's OWN shielded pool to the highest provably-abandoned one-time
/// address; mining it reopens the whole window. The moved principal returns to
/// the wallet via the existing recovery sweep — the true cost is the two
/// transactions' network fees, disclosed honestly before the action.
///
/// EXPLICIT + user-initiated (the #327 authorizer + the honest-cost disclosure
/// gate it) and offered ONLY when a send is actually PAUSED (the honest "the
/// window is stuck" signal — the SDK cannot tell a bricked window from a healthy
/// one, so an ungated call would mint uselessly). It unblocks the WINDOW only —
/// it NEVER re-sends a paused payment, so it can never double-pay.

/// Single-flight latch (mirrors the sweep's [ephemeralSweepInFlightProvider]):
/// the reclaim signs + broadcasts, so a second concurrent run — from an
/// impatient re-tap — is never launched (money-safe via the deadness gate, but
/// each redundant mint burns fees). The CTA watches this for its
/// disabled/in-progress cue.
final reclaimInFlightProvider = NotifierProvider<ReclaimController, bool>(
  ReclaimController.new,
);

/// The single-flight owner of the reclaim. State = in-flight.
class ReclaimController extends Notifier<bool> {
  @override
  bool build() => false;

  /// Run ONE reclaim. Returns the outcome, or `null` when nothing ran — another
  /// reclaim is already in flight (that run owns the outcome) or the session is
  /// gone. A typed fault PROPAGATES (the caller owns the copy); the latch resets
  /// and the balance/parked providers re-pull on EVERY exit.
  Future<ReclaimOutcome?> reclaim() async {
    // single-flight: the running reclaim owns the outcome
    if (state) return null;
    final session = ref.read(walletSessionProvider);
    if (session == null) return null;
    // The #327 seam: the reclaim SIGNS + broadcasts a wallet-internal self-mint.
    // A KNOWN small amount, but it is NOT a debit (the principal returns), so no
    // amount rides the intent — the honest-cost disclosure is the confirm dialog.
    // A denial propagates like any fault (the caller maps it to silence).
    final authorizer = ref.read(walletSendAuthorizerProvider);
    state = true;
    try {
      return await authorizer.authorizeSpend(
        // #383 R3: a reclaim is wallet-internal by construction (the self-mint
        // returns via sweep) — recipientIsSelf lets a prompting host say so.
        const WalletSpendIntent(
          kind: WalletSpendKind.reclaim,
          recipientIsSelf: true,
        ),
        () {
          // The identity-switch fence: an approval landing after a
          // session flip must never mint against the DEAD identity.
          if (!ref.mounted ||
              !identical(ref.read(walletSessionProvider), session)) {
            throw const WalletSpendSessionChanged();
          }
          return session.reclaimEphemeralSlots();
        },
      );
    } finally {
      // Guarded (see the sweep's finally): only a whole-scope teardown can
      // unmount this mid-reclaim, and an unmounted-ref throw here would REPLACE
      // the real outcome. On a live scope, re-pull so the surfaces settle: the
      // mint debits the shielded balance, and a paused send may resume once the
      // window reopens (the parked surface re-classifies).
      if (ref.mounted) {
        state = false;
        ref.invalidate(walletSnapshotReadProvider);
        ref.invalidate(walletParkedSendsReadProvider);
      }
    }
  }
}

/// Confirm (honest-cost disclosure) + run the reclaim + surface the outcome.
Future<void> confirmAndReclaim(BuildContext context, WidgetRef ref) async {
  final l10n = WalletLocalizations.of(context);
  final messenger = ScaffoldMessenger.of(context);
  // a11y: the honest-cost disclosure is a full sentence — it must SCROLL, not
  // clip, at large text scale (2–3×) on a short viewport; the helper's
  // Material dialog always scrolls, and Cupertino content always does.
  final confirmed = await showWalletConfirm(
    context,
    title: l10n.walletReclaimConfirmTitle,
    body: l10n.walletReclaimConfirmBody,
    cancelLabel: l10n.walletReclaimConfirmCancel,
    confirmLabel: l10n.walletReclaimConfirmAction,
    kind: WalletConfirmKind.forward,
  );
  if (!confirmed || !context.mounted) return;

  String message;
  try {
    final outcome = await ref.read(reclaimInFlightProvider.notifier).reclaim();
    // Nothing ran: another reclaim is in flight (it owns the snackbar) or the
    // session closed mid-confirm — nothing honest to report here.
    if (outcome == null) return;
    message = reclaimOutcomeMessage(l10n, outcome);
  } on WalletSpendAuthorizationDenied {
    // Declined at the host's authorization prompt — the host's own prompt was
    // the communication; "reopen failed" would be a lie (the seam contract).
    return;
  } on WalletSpendSessionChanged {
    // A session flip fenced the approval — nothing ran, the screen
    // already changed identity.
    return;
  } on WalletApiError catch (e) {
    // The one branch worth distinguishing: the user needs shielded funds to
    // self-mint. Everything else is an honest "couldn't reopen; unchanged".
    message = switch (e.kind) {
      WalletErrorKind_InsufficientFunds() => l10n.walletReclaimNeedsFunds,
      _ => l10n.walletReclaimFailed,
    };
  } catch (_) {
    message = l10n.walletReclaimFailed;
  }
  // Disposed during the reclaim (session closed / screen replaced): the durable
  // state is already settled by the controller — nothing to show.
  if (!context.mounted) return;
  messenger.showSnackBar(SnackBar(content: Text(message)));
}

/// Map a [ReclaimOutcome] to the honest user-facing copy. `Minted` is an
/// INITIATED state (the window reopens once the mint confirms; the moved amount
/// returns via the recovery sweep) — never "done".
String reclaimOutcomeMessage(WalletLocalizations l10n, ReclaimOutcome outcome) {
  return switch (outcome) {
    ReclaimOutcome_Minted() => l10n.walletReclaimStarted,
    ReclaimOutcome_NothingToReclaim() => l10n.walletReclaimNothing,
    ReclaimOutcome_NotBroadcast() => l10n.walletReclaimNotBroadcast,
    // Forward-compat arm (the core enum is #[non_exhaustive]; only reachable
    // under core/bridge version skew): a NEUTRAL "finished — check your sends",
    // never the success ("started") copy nor an error — we don't know what the
    // unrecognised outcome did, so we must not claim either.
    ReclaimOutcome_Unknown() => l10n.walletReclaimUnknown,
  };
}
