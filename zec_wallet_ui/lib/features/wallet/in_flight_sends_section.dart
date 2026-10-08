import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';

import '../../core/theme/colors.dart';
import '../../core/theme/icons.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'hide_balance.dart';
import 'wallet_providers.dart';
import 'wallet_sync_controller.dart';
import 'zat_format.dart';

/// The DURABLE in-flight two-step cue (#309): a payment whose first leg (the
/// unshield to a wallet-controlled one-time address) is broadcast but whose send
/// has not completed. The post-send result screen already said "don't send it
/// again" — but that screen is dismissible, and during the forwarding window
/// (the double-pay temptation window) the wallet screen would otherwise show only
/// an ordinary pending tx. This cue repeats the ONE permanently-true instruction
/// until the send completes (row deleted) or strands (the recoverable surface
/// takes over). Aggregate + amount-only (§5.4 — no recipient/address/txid); the
/// amount annotates the SAME pending payment the activity list shows, never
/// additional money. Hides on empty and on the cold load. A READ FAILURE is NOT
/// hidden (#308a, S2 §3.5d): a vanished cue is the same frame as "nothing is
/// mid-flight", the one state that tells the user nothing about re-sending, so
/// the failure renders an honest "could not check — retrying" line instead (the
/// reader keeps the container retry and re-pulls on every sync/resume edge). No
/// money depends on this line — tx0 stays visible in the activity list, and the
/// core's fence, not this read, guards a rescan — so it carries no button.
class InFlightSendsSection extends ConsumerWidget {
  const InFlightSendsSection({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final read = ref.watch(walletInFlightSendsProvider);
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;

    // `hasError` also holds while the container retry runs (a retrying state
    // carries the error), so the line stays put across retries instead of
    // blinking. A previous value is NOT shown under it: it is no longer known
    // to be current.
    if (read.hasError) {
      final message = l10n.walletInFlightReadError;
      return Semantics(
        container: true,
        liveRegion: true,
        excludeSemantics: true,
        label: message,
        child: Padding(
          padding: const EdgeInsets.only(bottom: 16),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              WalletIcon(
                WalletGlyph.syncProblem,
                size: 18,
                color: colors.textMuted,
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  message,
                  style: textTheme.bodySmall?.copyWith(color: colors.textMuted),
                ),
              ),
            ],
          ),
        ),
      );
    }

    final inFlight = read.value ?? const <InFlightSend>[];
    if (inFlight.isEmpty) return const SizedBox.shrink();

    // Plain sum is provably safe: ≤128 rows (the enqueue cap) × ≤MAX_MONEY
    // per-row gross (consensus-bounded, saturating at the Rust fold) ≪ 2^63.
    var totalZat = 0;
    for (final s in inFlight) {
      totalZat += s.amountZat;
    }
    // Pluralized: each in-flight send uses its OWN one-time address, so N≥2
    // must not read "a one-time address" / "don't send it again".
    // When no sync pass will run, the VARIANT key carries the whole story
    // (the append composed "still completing… Paused", a claim
    // then its negation in one breath, and sentence-concatenation breaks CJK
    // typography): the second leg forwards only on sync passes, so the variant
    // says "partway through … paused" as one coherent sentence; the
    // don't-send-again instruction is unchanged in both. The variant no longer
    // names the CAUSE — see #403 R4 below.
    //
    // THE FOURTH SITE (#403 R4). #401 R5 introduced `walletSyncPassesRunProvider`
    // for exactly this question and re-keyed three surfaces onto it; this one kept
    // testing `disabledByHost` directly. Under a FAILED sync start — where the loop
    // never ran and `after_synced` never fires, while the host policy still reads
    // `true` — this rendered the UNQUALIFIED note, i.e. "your wallet is still
    // COMPLETING" for money whose second leg forwards only on a pass that cannot
    // happen. It errs safe (the don't-send-again instruction is in both arms), but
    // it is the same wrong keying, and the variant it selects is the honest one.
    final syncPaused = !ref.watch(walletSyncPassesRunProvider);
    // Hide balance (FR-49 W-7): this row is information only (no action signs
    // from it), so its figure masks. The screen reader hears the same
    // sentence with "Balance hidden" in the figure's place (rev.2 R5); the
    // count and the don't-send-again instruction stay in both.
    final hidden = ref.watch(walletBalanceHiddenProvider);
    String noteWith(String amount) => syncPaused
        ? l10n.walletInFlightNoteSyncPaused(inFlight.length, amount)
        : l10n.walletInFlightNote(inFlight.length, amount);
    final note = noteWith(
      displayedAmount(l10n.walletAmount(formatZec(totalZat)), hidden: hidden),
    );

    // The whole cue is one semantic unit (a screen reader hears the full
    // caution, not an unlabeled icon + fragments).
    return Semantics(
      container: true,
      excludeSemantics: true,
      label: hidden ? noteWith(l10n.walletBalanceHiddenAmount) : note,
      child: Padding(
        padding: const EdgeInsets.only(bottom: 16),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            WalletIcon(WalletGlyph.inMotion, size: 18, color: colors.orange),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                note,
                style: textTheme.bodySmall?.copyWith(color: colors.textMuted),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
