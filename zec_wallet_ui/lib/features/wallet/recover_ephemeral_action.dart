import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/theme/icons.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'ephemeral_sweep.dart';
import 'recoverable_ephemeral.dart';
import 'wallet_providers.dart';

/// The "recover now" affordance (2e-2b-v-4b) — the Dart surface over the v-2 manual
/// ephemeral-SWEEP FFI. It recovers funds on wallet-controlled one-time (ephemeral)
/// transparent addresses into the wallet's OWN shielded balance.
///
/// GATED ON A SUCCESSFUL READ (the spec security note): shown ONLY when
/// [walletRecoverableEphemeralFundsProvider] has DATA (`hasValue`) AND there are
/// recoverable funds. A FAILED read must NEVER read as "nothing to recover" (which
/// would HIDE held funds), so on a first error/loading the button is simply absent —
/// its absence is neutral, never a "you're safe" claim. (`hasValue` is preserved
/// across a refetch-with-error, so a transient failure after a good read keeps the
/// recovery path reachable rather than hiding held funds.) The recoverable amount is
/// a SUBSET of the displayed balance, so recovering never implies extra funds. §5.4:
/// amount-only — the one-time addresses never cross the bridge.
///
/// The past-window / used-address return this gate can never surface has its own
/// ALWAYS-AVAILABLE entry — the overflow "Check one-time addresses…" (v-5c finding
/// #3) — driving the SAME shared sweep flow ([confirmAndSweepEphemeral]).
///
/// SINGLE-FLIGHT via the SHARED latch ([ephemeralSweepInFlightProvider], not
/// widget-local state): the sweep signs + broadcasts per funded address (30s+ on a
/// flaky link), so the button DISABLES while ANY sweep runs — whichever surface
/// launched it — both an honest persistent in-progress cue AND a guard so an
/// impatient re-tap can't launch a SECOND concurrent sweep (money-safe via the
/// engine's idempotency, but wasteful, confusing, and it would defeat the
/// per-ephemeral §5.3 decorrelation).
class RecoverEphemeralAction extends ConsumerWidget {
  const RecoverEphemeralAction({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final funds = ref.watch(walletRecoverableEphemeralFundsProvider);
    final inFlight = ref.watch(ephemeralSweepInFlightProvider);
    // Gate strictly on a SUCCESSFUL read — never the empty-on-error fallback.
    if (!funds.hasValue) return const SizedBox.shrink();
    final summary = summarizeRecoverable(funds.value!);
    // Keep the in-progress affordance visible even as the post-sweep re-pull
    // empties the recoverable list, so the spinner doesn't vanish mid-sweep.
    if (summary.totalZat <= 0 && !inFlight) return const SizedBox.shrink();

    final l10n = WalletLocalizations.of(context);
    return Padding(
      padding: const EdgeInsets.only(top: 8),
      child: Align(
        // DIRECTIONAL (#401 R8b): a hard `centerLeft` pins the recovery CTA to
        // the visual left in ar/he, away from the text it belongs to. This is one
        // of the two affordances the reclaim copy names by position.
        alignment: AlignmentDirectional.centerStart,
        // #401 R3b, the same family as the parked Send now: the in-flight cue is
        // a spinner plus a label swap and nothing else — announce it, or a screen
        // reader hears the button go quiet for the whole sweep and the user
        // re-taps a fee-burning action.
        // ON THE LABEL, INSIDE THE BUTTON (#403 R1, measured). `ButtonStyleButton`
        // builds its own `Semantics(container: true)`, so an annotation wrapped
        // AROUND it forms a separate, permanently EMPTY node — the flag sat on a
        // node whose data never changed, and this fee-burning CTA therefore
        // announced nothing for the whole sweep.
        child: OutlinedButton.icon(
          // The theme owns the 48 minimum touch target (stage S11 C6).
          // Disabled while a sweep runs — the single-flight guard + the honest
          // persistent in-progress indicator (no dead "Recovering…" snackbar gap).
          onPressed: inFlight
              ? null
              : () => confirmAndSweepEphemeral(context, ref),
          icon: inFlight
              ? const SizedBox(
                  height: 16,
                  width: 16,
                  child: CircularProgressIndicator.adaptive(strokeWidth: 2),
                )
              : const WalletIcon(WalletGlyph.recover, size: 18),
          label: Semantics(
            liveRegion: inFlight,
            child: Text(
              inFlight ? l10n.walletRecoverInProgress : l10n.walletRecoverNow,
            ),
          ),
        ),
      ),
    );
  }
}
