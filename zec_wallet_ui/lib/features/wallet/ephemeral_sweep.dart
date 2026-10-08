import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import '../../shared/wallet_dialog.dart';
import 'send_authorization.dart';
import 'wallet_providers.dart';
import 'zat_format.dart';

/// The manual one-time-address sweep, shared by its TWO entry points (v-5c
/// device-proof finding #3):
///
///  1. the balance-card "Recover now" ([RecoverEphemeralAction]) — gated on the
///     automatic windowed detect reading non-empty, and
///  2. the always-available overflow "Check one-time addresses…" — covering the
///     case the manual sweep uniquely exists for: a return PAST the detect
///     window, or to an already-used one-time address the windowed detect
///     skips, which the gated button can never surface.
///
/// ONE single-flight latch across BOTH ([ephemeralSweepInFlightProvider]): the
/// sweep signs + broadcasts per funded address (30s+ on a flaky link), so a
/// second concurrent run — from EITHER surface — is never launched (money-safe
/// via the engine's idempotency, but wasteful, confusing, and it would defeat
/// the per-ephemeral §5.3 decorrelation). Both surfaces watch the latch for
/// their disabled/in-progress cue, so starting a sweep from one visibly
/// disables the other.
final ephemeralSweepInFlightProvider =
    NotifierProvider<EphemeralSweepController, bool>(
      EphemeralSweepController.new,
    );

/// The single-flight owner of the manual ephemeral sweep. State = in-flight.
class EphemeralSweepController extends Notifier<bool> {
  @override
  bool build() => false;

  /// Run ONE sweep. Returns the engine summary, or `null` when nothing ran —
  /// another sweep is already in flight (that run reports its own outcome) or
  /// the session is gone. A typed sweep fault PROPAGATES (the caller owns the
  /// user-facing copy); the latch resets and the balance/recoverable providers
  /// re-pull on EVERY exit (success, fault, or dispose-during-sweep — the
  /// durable state is what must settle the surface, and the controller
  /// outlives any one widget).
  Future<EphemeralSweepSummary?> sweep() async {
    if (state) return null; // single-flight: the running sweep owns the outcome
    final session = ref.read(walletSessionProvider);
    if (session == null) return null;
    // The host-authorization seam (#327): the sweep SIGNS + broadcasts per
    // funded address, so it is authorized like any other spend. The total is
    // unknown up front (discovery is the sweep's job) — no amount on the
    // intent. A denial propagates like any sweep fault (the latch resets in
    // the finally); the caller maps it to silence, not an error message.
    final authorizer = ref.read(walletSendAuthorizerProvider);
    state = true;
    try {
      return await authorizer.authorizeSpend(
        // #383 R3: the sweep consolidates the wallet's OWN one-time-address
        // funds back into its shielded pool — recipientIsSelf by construction.
        const WalletSpendIntent(
          kind: WalletSpendKind.sweep,
          recipientIsSelf: true,
        ),
        () {
          // The identity-switch fence: an approval landing after a
          // session flip must never sweep the DEAD identity's one-time
          // addresses (see the seam contract). Whole-scope teardown also
          // trips `ref.mounted` first.
          if (!ref.mounted ||
              !identical(ref.read(walletSessionProvider), session)) {
            throw const WalletSpendSessionChanged();
          }
          return session.sweepEphemeralFunds();
        },
      );
    } finally {
      // Guarded: the controller has no provider dependencies (session is
      // `ref.read`), so only a whole-scope teardown can unmount it mid-sweep —
      // and an unmounted-ref throw in `finally` would REPLACE the sweep's real
      // outcome. Nothing to settle on a torn-down scope anyway.
      if (ref.mounted) {
        state = false;
        // The recovered amount lands in the shielded balance — re-pull both
        // the balance snapshot and the recoverable subset so the surface
        // settles.
        ref.invalidate(walletSnapshotReadProvider);
        ref.invalidate(walletRecoverableEphemeralFundsReadProvider);
      }
    }
  }
}

/// Confirm + run the sweep + surface the outcome — the ONE flow behind both
/// entries, so the dialog copy, the honest outcome mapping (partial ⇒ flag the
/// remainder, never a clean "Done" over funds that need a re-run), and the
/// snackbar reporting can never drift between them.
Future<void> confirmAndSweepEphemeral(
  BuildContext context,
  WidgetRef ref,
) async {
  final l10n = WalletLocalizations.of(context);
  final messenger = ScaffoldMessenger.of(context);
  final confirmed = await showWalletConfirm(
    context,
    title: l10n.walletRecoverConfirmTitle,
    body: l10n.walletRecoverConfirmBody,
    cancelLabel: l10n.walletRecoverConfirmCancel,
    confirmLabel: l10n.walletRecoverConfirmAction,
    kind: WalletConfirmKind.forward,
  );
  if (!confirmed || !context.mounted) return;

  String message;
  try {
    final summary = await ref
        .read(ephemeralSweepInFlightProvider.notifier)
        .sweep();
    // Nothing ran: another sweep is in flight (it owns the outcome snackbar)
    // or the session closed mid-confirm — nothing honest to report here.
    if (summary == null) return;
    message = sweepOutcomeMessage(l10n, summary);
  } on WalletSpendAuthorizationDenied {
    // Declined at the host's authorization prompt — no bridge call was made
    // and the host's own prompt was the communication; nothing to report
    // (the seam contract), and "recovery failed" would be a lie.
    return;
  } on WalletSpendSessionChanged {
    // The wallet session flipped mid-prompt and the SDK fenced the approval
    // — nothing ran, and the identity switch already replaced the
    // screen context; "recovery failed" would be untrue.
    return;
  } catch (_) {
    // A typed WalletApiError (seed required / closed handle) — the funds are
    // untouched and re-runnable.
    message = l10n.walletRecoverFailed;
  }
  // Disposed during the sweep (session closed / screen replaced): the durable
  // state is correct (the controller already re-pulled the providers), so
  // there's nothing to show.
  if (!context.mounted) return;
  messenger.showSnackBar(SnackBar(content: Text(message)));
}

/// Map an [EphemeralSweepSummary] to the honest user-facing outcome copy.
/// `recoveredZat` is PROVISIONAL (accepted, not yet mined) — "recovering".
String sweepOutcomeMessage(
  WalletLocalizations l10n,
  EphemeralSweepSummary summary,
) {
  if (summary.swept > 0 && (summary.failed > 0 || summary.truncated > 0)) {
    // Partial success: report the amount AND honestly flag the remainder —
    // a clean-Done copy here would HIDE funds that still need a re-run.
    return l10n.walletRecoverDonePartial(
      l10n.walletAmount(formatZec(summary.recoveredZat)),
    );
  }
  if (summary.swept > 0) {
    return l10n.walletRecoverDone(
      l10n.walletAmount(formatZec(summary.recoveredZat)),
    );
  }
  if (summary.failed > 0) {
    // Funds stay on-chain + re-runnable — honest "try again", never a loss.
    return l10n.walletRecoverRetry;
  }
  if (summary.truncated > 0) {
    // Truncated-only: the cap left addresses UNCHECKED — that is an
    // incomplete CHECK, not evidence of funds ("some funds need another try"
    // here would invent money the run never saw).
    return l10n.walletRecoverTruncated;
  }
  // Nothing was sweepable (e.g. already recovered on a prior run) — for the
  // always-available "Check one-time addresses…" entry this is the common,
  // reassuring answer.
  return l10n.walletRecoverNothing;
}
