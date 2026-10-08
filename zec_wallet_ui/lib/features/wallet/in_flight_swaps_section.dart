import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:zec_wallet/zec_wallet.dart';

import '../../core/router/wallet_routes.dart';
import '../../core/theme/colors.dart';
import '../../core/theme/icons.dart';
import '../../shared/action_row_layout.dart';
import '../../shared/wallet_dialog.dart';
import '../../shared/wallet_group.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'swap/swap_controller.dart';
import 'swap/swap_state.dart';
import 'sync_status_presentation.dart' show walletCompactTimeFormat;
import 'wallet_providers.dart';

/// The durable SWAP HOME (W-swap-5, #366): every swap whose order was
/// registered at execute and has neither been dismissed nor self-lapsed —
/// listed from the wallet's own encrypted store, so it SURVIVES a process
/// kill, screen re-entry, and session swaps. Pre-#366 an executed swap's only
/// UI trace was in-memory: kill→relaunch mid-armed-window left a live
/// OutOfZec deposit visible NOWHERE while the one-swap-in-flight guard
/// refused new swaps. Each row carries a
/// direction-honest line, its start time, and "View swap" — the re-attach
/// affordance that re-opens live tracking by the record's id.
///
/// Like the PARKED surface (and unlike the additive in-flight-sends cue), a
/// read failure renders an honest error line, never a silent hide: mid-flight
/// this list is the ONLY wallet-side witness of the swap (an OutOfZec deposit
/// is excluded from the send surfaces by design; an IntoZec swap has no
/// activity trace until delivery). Deliberately NOT gated on the swap
/// kill-switch — a local read is not swap traffic (§3.5); with swap off the
/// row still shows and tapping through renders the honest
/// tracking-unavailable state. §5.4: the record id is render-never-log (and
/// this section never renders it either — direction + time only).
class InFlightSwapsSection extends ConsumerWidget {
  const InFlightSwapsSection({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final swaps = ref.watch(walletInFlightSwapsProvider);
    final l10n = WalletLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    final colors = WalletColors.of(context);

    return swaps.when(
      // `skipLoadingOnReload` keeps a populated section visible while an
      // invalidate (sync-edge / resume / dismiss) re-pulls — no per-refresh
      // blink. The cold first load renders nothing (nothing to re-attach yet).
      skipLoadingOnReload: true,
      loading: () => const SizedBox.shrink(),
      // A read failure is surfaced honestly (an in-flight swap is money in
      // motion and this is its only wallet-side witness) — never a silent hide
      // — AND recoverable in place. The home has no pull-to-refresh, so an
      // inline retry is the only way back from a transient read fault short of a
      // full app resume (UX+reliability MED). liveRegion announces the
      // loading→error transition to a screen reader in place (a11y).
      error: (_, _) {
        // While the tapped re-pull is in flight, `skipLoadingOnReload` keeps
        // THIS arm rendered and the AsyncValue reports isLoading — reflect it
        // (spinner + disabled) so a tap on a persistently-failing store is
        // never a MUTE no-op (review, converged UX+reliability MED): the
        // sighted user sees work happen, and the label swap below re-fires the
        // live region when the identical error re-lands (#407 R5 — the flag had
        // to move INSIDE for that claim to be true).
        final refreshing = swaps.isLoading;
        return Padding(
          padding: const EdgeInsets.only(bottom: 28),
          child: Semantics(
            container: true,
            liveRegion: true,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  l10n.walletSwapsInFlightError,
                  style: textTheme.bodyMedium?.copyWith(
                    color: colors.textMuted,
                  ),
                ),
                // Start-aligned by the Column's crossAxisAlignment (RTL-correct).
                TextButton(
                  key: const Key('swaps-in-flight-error-retry'),
                  // Re-pull the identity-scoped reader (the same provider the
                  // dismiss / resume paths invalidate) → the section re-runs
                  // loading→data, recovering without a full app resume.
                  onPressed: refreshing
                      ? null
                      : () => ref.invalidate(walletInFlightSwapsReadProvider),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      if (refreshing) ...[
                        const SizedBox(
                          width: 16,
                          height: 16,
                          child: CircularProgressIndicator.adaptive(
                            strokeWidth: 2,
                          ),
                        ),
                        const SizedBox(width: 8),
                      ],
                      // Flexible + an INNER live region, the twin of the parked
                      // section's retry (#407 R5 / #401 R8a): a Row child gets
                      // unbounded width, and a liveRegion on the outer container
                      // cannot re-announce because that node's data never
                      // changes — the label that SWAPS has to carry the flag.
                      Flexible(
                        child: Semantics(
                          liveRegion: refreshing,
                          child: Text(
                            refreshing
                                ? l10n.walletSwapsInFlightRetryInProgress
                                : l10n.walletSwapsInFlightRetry,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        );
      },
      data: (records) {
        if (records.isEmpty) return const SizedBox.shrink();
        // The S12 look (S13 Build B): the Activity section's header and ONE
        // group with hairlines inset to the rows' text.
        return Padding(
          padding: const EdgeInsets.only(bottom: 16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Padding(
                padding: const EdgeInsetsDirectional.only(start: 4),
                child: Text(
                  l10n.walletSwapsInFlightTitle(records.length),
                  // The Activity header's role, for parity with the Parked +
                  // Activity sections (UX NIT; S12 C5).
                  style: textTheme.titleLarge,
                ),
              ),
              const SizedBox(height: 12),
              WalletGroup(
                key: const ValueKey('wallet-swaps-group'),
                dividerIndent: _SwapRow.textInset,
                children: [
                  for (final record in records) _SwapRow(record: record),
                ],
              ),
            ],
          ),
        );
      },
    );
  }
}

class _SwapRow extends ConsumerWidget {
  const _SwapRow({required this.record});

  final SwapRecord record;

  /// Where a row's text starts: the 16 padding, the 18 glyph, the 8 gap — the
  /// group's hairlines are inset to it.
  static const double textInset = 42;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    final colors = WalletColors.of(context);

    // #367/#382 row honesty. Four tiers, most-certain first: (1) a PINNED
    // observed terminal renders the outcome (the HIGH-2 fix's visible
    // half — a terminal seen by the stream while the user was away is never
    // erased unseen); (2) an UNRESOLVED record past its SETTLEMENT bound
    // (#382 — such rows now list indefinitely instead of vanishing at 48 h)
    // renders the unconfirmed-outcome line (outcome-NEUTRAL — the swap may have
    // succeeded unobserved; review), which also carries the money promise
    // ("any ZEC coming back shows up in your balance"); (3) a record whose
    // DEPOSIT WINDOW has passed must not assert present-tense motion for the
    // rest of the settlement window ("your ZEC is on its way" was false for
    // hours) — a neutral check-status line, both directions; (4) the live
    // direction-honest motion line. The comparisons are client-clock
    // display-only (the tracking view holds the authoritative provider truth
    // one tap away).
    final nowUnix = DateTime.now().millisecondsSinceEpoch ~/ 1000;
    final deadline = record.depositDeadline;
    final windowPassed = deadline != null && nowUnix >= deadline;
    final overdue = nowUnix >= record.expiresAt;
    final outcome = record.outcome;
    final line = switch (outcome) {
      SwapOutcome.success => l10n.walletSwapRowOutcomeSuccess,
      SwapOutcome.refunded => l10n.walletSwapRowOutcomeRefunded,
      SwapOutcome.failed => l10n.walletSwapRowOutcomeFailed,
      // Direction-split (#385): "ZEC coming back" is refund-shaped — right for
      // OutOfZec (and the honest best-effort for unknown), wrong for IntoZec,
      // whose ZEC leg is the incoming DELIVERY ("ZEC it delivers").
      null when overdue => switch (record.direction) {
        SwapRecordDirection.intoZec => l10n.walletSwapInFlightRowOverdueIntoZec,
        _ => l10n.walletSwapInFlightRowOverdue,
      },
      null when windowPassed => l10n.walletSwapInFlightRowPastWindow,
      null => switch (record.direction) {
        SwapRecordDirection.outOfZec => l10n.walletSwapInFlightRowOutOfZec,
        SwapRecordDirection.intoZec => l10n.walletSwapInFlightRowIntoZec,
        SwapRecordDirection.unknown => l10n.walletSwapInFlightRowGeneric,
      },
    };
    final (icon, iconColor) = switch (outcome) {
      SwapOutcome.success => (WalletGlyph.success, colors.green),
      SwapOutcome.refunded => (WalletGlyph.refunded, colors.orange),
      SwapOutcome.failed => (WalletGlyph.error, colors.red),
      null => (WalletGlyph.swap, colors.cyan),
    };
    final started = l10n.walletSwapInFlightStarted(
      walletCompactTimeFormat(l10n.localeName).format(
        DateTime.fromMillisecondsSinceEpoch(record.createdAt * 1000).toLocal(),
      ),
    );
    // The forward-compat unknown arm renders WITHOUT the re-attach action (an
    // attach would have to guess the direction-split money copy); today's
    // store writes only the two known arms.
    final direction = switch (record.direction) {
      SwapRecordDirection.outOfZec => SwapFlowDirection.outOfZec,
      SwapRecordDirection.intoZec => SwapFlowDirection.intoZec,
      SwapRecordDirection.unknown => null,
    };

    final label = Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        WalletIcon(icon, size: 18, color: iconColor),
        const SizedBox(width: 8),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(line, style: textTheme.bodyMedium),
              const SizedBox(height: 2),
              Text(
                started,
                style: textTheme.bodySmall?.copyWith(color: colors.textMuted),
              ),
            ],
          ),
        ),
      ],
    );
    final view = direction == null
        ? null
        : TextButton(
            // The theme owns the 48 minimum touch target (stage S11 C6).
            onPressed: () {
              ref
                  .read(swapControllerProvider.notifier)
                  .attachTo(swapId: record.id, direction: direction);
              context.push(WalletRoutes.swap);
            },
            // Bind the action to ITS row for controls-nav / switch-access users
            // (UX NIT): with 2+ rows, a bare "View swap" label can't be
            // told apart out of traversal context.
            child: Text(
              l10n.walletSwapViewSwap,
              semanticsLabel: '${l10n.walletSwapViewSwap}: $line',
            ),
          );
    // The explicit user REMOVE (#367 — the record had no user-remove at all;
    // pinned terminals would otherwise squat the home for their full ~48 h
    // self-lapse). Confirmed: an IN-FLIGHT remove drops the only re-attach
    // handle (the confirm says it doesn't cancel the swap); a terminal remove
    // is plain list hygiene. Display-only either way (dismiss never touches
    // money state).
    final remove = IconButton(
      onPressed: () => _confirmRemove(context, ref, l10n, line),
      // The label rides the ICON, not a Semantics wrapper around the button
      // (#409 R3 review, measured against the real semantics tree): an outer
      // `Semantics(label:)` is a non-container annotation, so inside this
      // row's `Semantics(container: true, label: ...)` it merged UPWARD and
      // left the button's own node announcing "" — the button was unlabelled
      // for a screen reader in BOTH layout arms. `tooltip:` alone does not
      // supply it either (also measured). Bound to ITS row like the View
      // action: with 2+ rows a bare "Remove" cannot be told apart.
      icon: WalletIcon(
        WalletGlyph.close,
        size: 18,
        color: colors.textMuted,
        semanticLabel: '${l10n.walletSwapRemove}: $line',
      ),
      // 44px target, sibling-action parity — and NO `VisualDensity.compact`
      // here (#409 R3 review, measured): compact subtracts 4dp per axis AFTER
      // these constraints, so the button rendered 40x40 while this very
      // comment claimed 44. In the stacked Wrap it was the only action under
      // the floor `flutter-patterns` sets.
      constraints: const BoxConstraints(minWidth: 44, minHeight: 44),
      tooltip: l10n.walletSwapRemove,
    );

    // One semantic unit per row (a screen reader hears the whole story, then
    // the action); the id itself never renders (§5.4 + shoulder-surf hygiene).
    return Semantics(
      container: true,
      label: '$line $started',
      child: Padding(
        // A group row: 16 to the text's side, 4 to the trailing 44 targets'.
        padding: const EdgeInsetsDirectional.fromSTEB(16, 8, 4, 8),
        // STACK vs INLINE on the row's OWN width and the body text scale
        // (`walletRowStacksAction` — one rule, four surfaces). Arch review M-A1
        // measured 144-346px overflows at 320dp/3.0× across locales, which the
        // scale half fixes; #409 R3 then measured the half that was missing —
        // on the wallet screen at a 320dp viewport the copy keeps 71dp (1.0×)
        // and 36dp (1.3×), growing this ONE row to 658dp tall, and in `en` it
        // throws nothing while doing it. (`de` at 320dp/1.0× DID throw, 14px:
        // the label's icon+gap exceeds what the two trailing actions leave.)
        // Ordinary desktop/tablet widths keep the compact single row.
        child: LayoutBuilder(
          builder: (context, constraints) {
            return walletRowStacksAction(context, constraints.maxWidth)
                // A Wrap (not a Row) so at extreme scales the two actions flow
                // onto separate lines instead of overflowing the 320dp row (the
                // M-A1 discipline that moved them under the copy to begin with).
                ? Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      ExcludeSemantics(child: label),
                      Align(
                        alignment: AlignmentDirectional.centerEnd,
                        child: Wrap(
                          alignment: WrapAlignment.end,
                          crossAxisAlignment: WrapCrossAlignment.center,
                          spacing: 4,
                          runSpacing: 4,
                          children: [?view, remove],
                        ),
                      ),
                    ],
                  )
                : Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Expanded(child: ExcludeSemantics(child: label)),
                      ?view,
                      remove,
                    ],
                  );
          },
        ),
      ),
    );
  }

  /// Confirm, then dismiss the record + refresh the home list. The confirm is
  /// load-bearing for a NON-terminal row (it drops the only re-attach handle;
  /// the body is explicit that the swap itself is NOT cancelled); a pinned
  /// terminal row gets the lighter "remove from list" body. Failures are
  /// swallowed (display-only; a PINNED row self-lapses at its own bound — an
  /// UNPINNED one lists until removed/pinned (#382), so a persistently-failing
  /// dismiss leaves a durable row, retryable via the same affordance) and the
  /// refresh rides the container so a mid-dialog unmount can't skip it. The
  /// session is re-read AFTER the confirm (the parked-cancel precedent, review
  /// fold): a session flip while the dialog sits open then dismisses against
  /// the CURRENT store — an absent id is the idempotent `false`, never a write
  /// against a dead identity's handle.
  Future<void> _confirmRemove(
    BuildContext context,
    WidgetRef ref,
    WalletLocalizations l10n,
    String line,
  ) async {
    final container = ProviderScope.containerOf(context, listen: false);
    final confirmed = await showWalletConfirm(
      context,
      title: l10n.walletSwapRemoveTitle,
      // The in-flight body is DIRECTION-SPLIT (#385, UX HIGH-1): the
      // shared body's three claims — "watching for its refund", "ZEC
      // refunded later still belongs to this wallet", "a rescan can find
      // it" — are all FALSE for IntoZec (the watch is the DELIVERY leg; an
      // IntoZec refund is the user's foreign deposit returned on the
      // source chain, invisible to this wallet). The unknown arm makes
      // only direction-independent claims.
      body: record.outcome == null
          ? switch (record.direction) {
              SwapRecordDirection.outOfZec => l10n.walletSwapRemoveBodyInFlight,
              SwapRecordDirection.intoZec =>
                l10n.walletSwapRemoveBodyInFlightIntoZec,
              SwapRecordDirection.unknown =>
                l10n.walletSwapRemoveBodyInFlightUnknown,
            }
          : l10n.walletSwapRemoveBodyDone,
      cancelLabel: l10n.walletSwapRemoveCancel,
      confirmLabel: l10n.walletSwapRemoveConfirm,
      kind: WalletConfirmKind.destructive,
    );
    if (!confirmed) return;
    final session = container.read(walletSessionProvider);
    if (session == null) return;
    try {
      await session.dismissSwapRecord(swapId: record.id);
    } on Object {
      // Swallowed: display-only; a pinned row self-lapses, an unpinned one
      // stays listed (#382) and the Remove affordance simply retries.
    }
    container.invalidate(walletInFlightSwapsReadProvider);
  }
}
