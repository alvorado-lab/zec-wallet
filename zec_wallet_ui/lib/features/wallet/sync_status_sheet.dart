import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/theme/colors.dart';
import '../../core/theme/icons.dart';
import '../../core/theme/shapes.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import '../../shared/wallet_sheet.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'sync_server_sheet.dart';
import 'sync_status_presentation.dart';
import 'wallet_display_sync_status.dart';
import 'wallet_health.dart';
import 'wallet_providers.dart';
import 'wallet_sync_controller.dart';

/// Open the sync-detail "what's going on" sheet (#319; maintainer: "when user hits
/// scanning badge he has to see more detailed information"). Informational and
/// LIVE — it watches the same providers as the badge, so the state, percent and
/// counts keep moving while it is open. Freely dismissible (scrim/drag/back):
/// unlike the rescan sheet it owns no in-flight operation, so a casual dismiss
/// can never orphan anything.
Future<void> showSyncStatusSheet(BuildContext context) {
  // Captured BEFORE the await: the caller's context can be unmounted by the
  // time the sheet settles, but the container outlives it.
  final container = ProviderScope.containerOf(context, listen: false);
  // The shared sheet frame: grows with the stall copy + large text scales,
  // freely dismissible, the desktop width cap.
  return showWalletSheet<void>(
    context,
    builder: (_) => const SyncStatusSheet(),
  ).whenComplete(
    // End any post-Try-now live window WITH the sheet (audit D): the
    // window exists because "the user is watching their own tap play out",
    // and a dismissed sheet means nobody is — left running it would spill
    // raw flap onto the badge's now-unblocked liveRegion and the send
    // screen's pinned queue affordance for up to its full span.
    container.read(walletDisplaySyncStatusProvider.notifier).stopFollowingRaw,
  );
}

/// The sheet body (public for widget tests — pump it directly in a
/// `ProviderScope` over a `FakeWalletSession`, the shield-sheet idiom).
class SyncStatusSheet extends ConsumerWidget {
  const SyncStatusSheet({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;

    // The SAME status derivation as the wallet screen (live stream first, the
    // cold snapshot's status before the first emit) so the sheet can never
    // disagree with the badge that opened it — both read the posture-stable
    // DISPLAY view (#399: the dwell layer), never the raw flap.
    final snapshot = ref.watch(walletSnapshotProvider);
    final status =
        ref.watch(walletDisplaySyncStatusProvider).value ??
        snapshot.value?.syncStatus ??
        const SyncStatus.idle();
    final drive = ref.watch(walletSyncControllerProvider);
    final driving = drive == WalletSyncDrive.running;
    // The START command failed (#356-F8): the Idle arm's "starts automatically"
    // copy would lie, and the sheet must offer the same retry the surface
    // notice does — a user who opened the sheet to see what's wrong should be
    // able to act from here.
    final startFailed = drive == WalletSyncDrive.failed;
    // #383 R1: the HOST's sync policy is off — the sheet tells the same
    // "Sync off" story as the badge (no-drift rule) and offers NO retry (the
    // way back is the host's own settings, not a package affordance).
    final syncDisabled = drive == WalletSyncDrive.disabledByHost;
    final stale = snapshot.hasError;

    final p = syncStatusPresentation(
      l10n,
      status,
      driving: driving,
      startFailed: startFailed,
      syncDisabled: syncDisabled,
    );
    final level = walletBadgeLevel(
      status: status,
      stale: stale,
      startFailed: startFailed,
      syncDisabled: syncDisabled,
    );
    final tint = switch (level) {
      WalletBadgeLevel.ok => colors.green,
      WalletBadgeLevel.caution => colors.orange,
      WalletBadgeLevel.error => colors.red,
    };

    // The Connection section — the full story behind the
    // badge's icon-only transport indicator. The SAME shared mapping as the
    // badge (no-drift rule); the host's own transport claim wins when wired.
    // No snapshot yet (cold load/error) reads as the honest "can't verify"
    // arm, never a benign default (§3.3 privacy rule).
    final transport = transportPresentation(
      l10n,
      tor: snapshot.value?.tor ?? const TorState.unknown(),
      host: ref.watch(walletHostTransportProvider),
    );
    final transportTint = switch (transport.tone) {
      TransportTone.protected => colors.green,
      TransportTone.neutral => colors.textMuted,
      TransportTone.progress => colors.cyan,
      TransportTone.caution => colors.orange,
      TransportTone.danger => colors.red,
    };
    // The server host (never a full URL): wired by walletOnboardingOverrides
    // from the host's WalletConfig, or by a session-only host directly. Null
    // hides the row (honest — better absent than guessed).
    // Since P3-13 the host comes from the SESSION's effective server (the
    // dial's own truth) and falls back to the host seam before a session.
    final endpointHost = ref.watch(walletEffectiveServerHostProvider);
    final serverStatus = ref.watch(walletSyncServerStatusProvider).value;
    final canPick = ref.watch(walletSessionProvider) != null;

    // Live figure rows per arm. The sheet is where the EXACT numbers belong
    // (the badge deliberately keeps the compact forms). A row is a
    // (label, value) pair; a null value is a ONE-CELL row whose label is a
    // whole sentence carrying its own figure (the behind row below — an ICU
    // plural cannot be split across two cells).
    final rows = <(String, String?)>[];
    switch (status) {
      // The Scanning rows ("Progress N%", "Blocks left") claim an in-flight
      // scan, so a RETAINED Scanning under sync-off must not render them
      // beside the "Sync off" headline (the progress BAR is already
      // suppressed by the presentation's syncDisabled arm). The reached-tip /
      // Offline "Synced to" rows below stay: they ARE the last synced state
      // the disabled explanation promises.
      case SyncStatus_Scanning(:final from, :final to, :final percent)
          when !syncDisabled:
        final pct = scanPercent(percent);
        if (pct > 0) rows.add((l10n.walletSyncSheetProgress, '$pct%'));
        final remaining = to - from;
        if (remaining >= 2) {
          rows.add((
            l10n.walletSyncSheetBlocksLeft,
            exactBlockCount(remaining, l10n.localeName),
          ));
        }
      // §4r U-2: "Synced to" for EVERY reached-tip variant on a current
      // server — the figure is the tip the pass reached, whatever qualifies
      // the headline (this BUILD's reach, this SERVER's pools, its network
      // claim). Before §4r the three qualified siblings fell to `default`,
      // and the sheet showed no figure at all for a pass that had one.
      case SyncStatus_UpToDate(:final tip) ||
          SyncStatus_UpToDateLimited(:final tip) ||
          SyncStatus_UpToDateDegraded(:final tip) ||
          SyncStatus_UpToDateUnverified(:final tip):
        rows.add((
          l10n.walletSyncSheetSyncedTo,
          exactBlockCount(tip, l10n.localeName),
        ));
      // §4m #5 / §4r U-2: the behind arm's own figures — the tip THIS SERVER
      // reached, and how far behind it is AT LEAST. `newestKnown` is the
      // newest height this wallet knows the chain reached (a public constant
      // of the app, or its own last scanned height less the reorg allowance),
      // so `newestKnown - tip` is a LOWER bound on the server's lag, never the
      // gap itself — hence "at least". The exact grouped count, the sheet's
      // vocabulary; no row when the bound is below one block (a `newestKnown`
      // at or below `tip` says nothing worth a number).
      case SyncStatus_EndpointBehind(:final tip, :final newestKnown):
        rows.add((
          l10n.walletSyncSheetSyncedTo,
          exactBlockCount(tip, l10n.localeName),
        ));
        final behind = newestKnown - tip;
        if (behind >= 1) {
          rows.add((
            l10n.walletSyncSheetBehindBy(
              behind,
              exactBlockCount(behind, l10n.localeName),
            ),
            null,
          ));
        }
      case SyncStatus_Offline(:final lastSynced):
        if (lastSynced != null) {
          rows.add((
            l10n.walletSyncSheetSyncedTo,
            exactBlockCount(lastSynced.height, l10n.localeName),
          ));
        }
      default:
        break;
    }

    return SafeArea(
      child: SingleChildScrollView(
        // Bounded even under a tall stall paragraph + a large text scale.
        child: Padding(
          padding: const EdgeInsets.fromLTRB(20, 16, 20, 24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Header: the LIVE state, in the badge's own words and color.
              WalletSheetHeader(
                title: p.headline,
                titleColor: tint,
                closeKey: const Key('sync-status-close'),
                leading: WalletIcon(p.icon, color: tint, size: 24),
              ),
              const SizedBox(height: 12),
              Text(
                syncStatusExplanation(
                  l10n,
                  status,
                  driving: driving,
                  startFailed: startFailed,
                  syncDisabled: syncDisabled,
                ),
                style: textTheme.bodyMedium?.copyWith(color: colors.text),
              ),
              // The state's own detail lines (stall reason, catching-up, funds
              // spendable) — the copy the fixed-height badge row no longer
              // shows. When the start-failure OWNS the explanation above, its
              // detail line is redundant here (near-identical sentences
              // stacked — UX LOW); it stays in the presentation for the
              // badge a11y label, where it is the only carrier.
              // …and the syncDisabled detail gets the same treatment:
              // the explanation above already ends "…until syncing is turned
              // on", so stacking "Turn on syncing in this app's settings…"
              // directly under it is the near-identical pair the rule
              // exists for. It stays in the presentation for the badge a11y
              // label, where it is the only carrier.
              // …and the connectivity stall's reason line too (#399
              // review M4): walletSyncExplainStalledOffline above opens with
              // the same "Can't reach the Zcash network" and carries both
              // hedges itself, so the walletStallEndpoint paragraph under it
              // would say it all twice on the one surface built to be calm.
              // Like its two precedents it stays in the presentation for the
              // badge a11y label, where it is the only carrier.
              for (final detail in p.details)
                if (!(startFailed && detail == l10n.walletSyncStartFailed) &&
                    !(syncDisabled &&
                        detail == l10n.walletSyncDisabledDetail) &&
                    !(!startFailed &&
                        !syncDisabled &&
                        detail == l10n.walletStallEndpoint)) ...[
                  const SizedBox(height: 8),
                  Text(
                    detail,
                    style: textTheme.bodyMedium?.copyWith(
                      color: colors.textMuted,
                    ),
                  ),
                ],
              if (stale) ...[
                const SizedBox(height: 8),
                Text(
                  l10n.walletBalanceStale,
                  style: textTheme.bodyMedium?.copyWith(color: colors.orange),
                ),
              ],
              // The sheet-local retry for a failed start (#356-F8) — the same
              // action as the surface notice's button, so a user who came here
              // to diagnose can recover without dismissing the sheet first.
              // (Rendered on the failed DRIVE regardless of the status arm: a
              // cold snapshot can report a non-Idle status while the loop
              // still isn't running.)
              if (startFailed) ...[
                const SizedBox(height: 12),
                OutlinedButton.icon(
                  onPressed: () =>
                      ref.read(walletSyncControllerProvider.notifier).retry(),
                  icon: const WalletIcon(WalletGlyph.retry, size: 18),
                  label: Text(l10n.walletSyncRetry),
                ),
              ],
              // "Try now" on a LIVE stall (#399 item 4): the loop's retry
              // backoff caps at 600s and never resets on connectivity return,
              // so after a long outage a user who just fixed their network
              // could wait up to 10 min on "retries automatically" — this is
              // the immediate attempt (stop+start resets the ladder by
              // design; scan progress is durable, nothing is lost). Gated on
              // `driving`: a stall is live only while the loop runs — under
              // sync-off or a failed start the honest affordances above own
              // the slot (mutually exclusive by the drive enum, so this can
              // never render beside the startFailed retry).
              if (status is SyncStatus_Stalled && driving) ...[
                const SizedBox(height: 12),
                OutlinedButton.icon(
                  onPressed: () {
                    // The live window opens ONLY when the restart was really
                    // enqueued (audit H): a gate-rejected stale tap must
                    // not put the display into a 4s raw-mirror over a loop
                    // that was never restarted.
                    if (ref
                        .read(walletSyncControllerProvider.notifier)
                        .tryNow()) {
                      // The tap's feedback: drop the posture dwell and follow
                      // the restarted loop live — a held "Sync paused" here
                      // would read as a dead tap.
                      ref
                          .read(walletDisplaySyncStatusProvider.notifier)
                          .followRawNow();
                    }
                  },
                  icon: const WalletIcon(WalletGlyph.retry, size: 18),
                  label: Text(l10n.walletSyncTryNow),
                ),
              ],
              if (p.scanning) ...[
                const SizedBox(height: 16),
                ClipRRect(
                  borderRadius: BorderRadius.circular(
                    WalletShapes.of(context).progress,
                  ),
                  // The theme owns the bar's colours and height (`cyan` on
                  // `outline`, 6 high; stage S11 C6): the sync STATE rides the
                  // tinted glyph in the header, never the bar.
                  child: LinearProgressIndicator(
                    // null ⇒ indeterminate animation (opaque early phase).
                    value: p.progress,
                  ),
                ),
              ],
              if (rows.isNotEmpty) ...[
                const SizedBox(height: 16),
                for (final (label, value) in rows)
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: 4),
                    child: MergeSemantics(
                      child: value == null
                          // The one-cell row: a sentence that carries its own
                          // figure (the behind row), the whole width in the
                          // value weight — it IS the figure — and it WRAPS
                          // (a large count at a large text scale must never
                          // overflow the sheet the way an ellipsis would hide
                          // the number).
                          ? Text(
                              label,
                              style: textTheme.bodyMedium?.copyWith(
                                color: colors.text,
                                fontWeight: FontWeight.w600,
                              ),
                            )
                          : Row(
                              children: [
                                Expanded(
                                  child: Text(
                                    label,
                                    style: textTheme.bodyMedium?.copyWith(
                                      color: colors.textMuted,
                                    ),
                                  ),
                                ),
                                Text(
                                  value,
                                  style: textTheme.bodyMedium?.copyWith(
                                    color: colors.text,
                                    fontWeight: FontWeight.w600,
                                  ),
                                ),
                              ],
                            ),
                    ),
                  ),
              ],
              // Connection: transport privacy + which server we talk to
              // (maintainer — "showing which host we use to access the
              // zec network", expanded from the badge's icon-only indicator).
              const SizedBox(height: 16),
              Divider(color: colors.border, height: 1),
              const SizedBox(height: 16),
              Text(
                l10n.walletSyncSheetConnection,
                style: textTheme.labelMedium?.copyWith(color: colors.textMuted),
              ),
              const SizedBox(height: 8),
              MergeSemantics(
                child: Row(
                  children: [
                    WalletIcon(
                      transport.tone == TransportTone.protected
                          ? WalletGlyph.protected
                          : WalletGlyph.shielded,
                      size: 18,
                      color: transportTint,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        transport.label,
                        style: textTheme.bodyMedium?.copyWith(
                          color: transportTint,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 6),
              Text(
                transport.detail,
                style: textTheme.bodyMedium?.copyWith(color: colors.textMuted),
              ),
              if (endpointHost != null) ...[
                const SizedBox(height: 10),
                // P3-13: the row OPENS THE PICKER when a session exists (a
                // 44 dp target with a chevron); before a session it is the
                // plain host row it always was.
                Semantics(
                  button: canPick,
                  label: canPick
                      ? l10n.walletSyncServerRowSemantics(endpointHost)
                      : null,
                  child: InkWell(
                    onTap: canPick ? () => showSyncServerSheet(context) : null,
                    // A row of the sheet's Connection group: the ripple reads
                    // the group radius.
                    borderRadius: BorderRadius.circular(
                      WalletShapes.of(context).group,
                    ),
                    child: ConstrainedBox(
                      constraints: const BoxConstraints(minHeight: 44),
                      child: ExcludeSemantics(
                        excluding: canPick,
                        child: Row(
                          children: [
                            Expanded(
                              child: Text(
                                l10n.walletSyncSheetServer,
                                style: textTheme.bodyMedium?.copyWith(
                                  color: colors.textMuted,
                                ),
                              ),
                            ),
                            // §5.4: an endpoint host is display data (public
                            // config), shown but never logged. Flexible + WRAP
                            // (never ellipsis): a host name is arbitrary-length
                            // (review B1 measured a 796px overflow on a
                            // tailnet host at 320dp/2.0×), and truncating a
                            // server IDENTITY could render
                            // "zec.rocks.evil.example" as "zec.rocks…" — a
                            // §3.3-class dishonesty.
                            Flexible(
                              child: Text(
                                endpointHost,
                                textAlign: TextAlign.end,
                                style: textTheme.bodyMedium?.copyWith(
                                  color: colors.text,
                                  fontWeight: FontWeight.w600,
                                ),
                              ),
                            ),
                            if (canPick) ...[
                              const SizedBox(width: 4),
                              WalletIcon(
                                WalletGlyph.chevronForward,
                                size: 20,
                                color: colors.textMuted,
                              ),
                            ],
                          ],
                        ),
                      ),
                    ),
                  ),
                ),
                // The fallback banner (P3-13 D2): the remembered choice could
                // not be honoured, so the default is in use — said, never
                // silent.
                if (serverStatus?.fallback != null) ...[
                  const SizedBox(height: 6),
                  Text(
                    syncServerFallbackCopy(
                      l10n,
                      serverStatus!.fallback!,
                      host: endpointHost,
                    ),
                    style: textTheme.bodyMedium?.copyWith(color: colors.orange),
                  ),
                ],
              ],
              const SizedBox(height: 16),
              Align(
                // Directional (UX LOW): in RTL the dismiss must sit at
                // the reading END, opposite the start-aligned retry above.
                alignment: AlignmentDirectional.centerEnd,
                child: TextButton(
                  onPressed: () => Navigator.of(context).maybePop(),
                  child: Text(l10n.walletSyncSheetClose),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
