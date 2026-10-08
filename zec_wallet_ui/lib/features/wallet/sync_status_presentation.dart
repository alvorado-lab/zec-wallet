import 'package:intl/intl.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/core/theme/icons.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';

/// Pure per-arm presentation of the live [SyncStatus] — ONE mapping shared by
/// the compact badge row and the sync-detail sheet (the two surfaces show
/// the same state and must never drift on icon/headline/copy). Extracted from
/// the badge (spec §3.3, ADR-0533); the COLOR stays separate — it comes from
/// `walletBadgeLevel` (the gate-8 truth-tabled health rule), never from here.
class SyncStatusPresentation {
  const SyncStatusPresentation({
    required this.icon,
    required this.headline,
    this.trailing,
    this.details = const [],
    this.scanning = false,
    this.progress,
  });

  final WalletGlyph icon;

  /// One-line state headline ("Up to date", "Scanning 42%", …).
  final String headline;

  /// Optional compact same-row chip (the "1.6M blocks left" countdown).
  final String? trailing;

  /// Plain-language detail lines — none, one, or (on an arm that carries a
  /// degraded pool report, §4r U-3) one more per affected pool. The badge row
  /// no longer renders these (fixed single-row height — the maintainer's
  /// no-layout-shift ask); they ride the badge's a11y label and the detail
  /// sheet, where they follow the arm's explanation.
  final List<String> details;

  /// Whether a progress bar should be drawn for this state.
  final bool scanning;

  /// 0..1 bar value; null while [scanning] means an indeterminate animation
  /// (the opaque early phase of a deep sync).
  final double? progress;
}

SyncStatusPresentation syncStatusPresentation(
  WalletLocalizations l10n,
  SyncStatus status, {
  required bool driving,
  bool startFailed = false,
  bool syncDisabled = false,
}) {
  // #383 R1: the host's sync policy is OFF ([WalletSyncDrive.disabledByHost])
  // — this owns the WHOLE presentation, over any retained status arm: a
  // retained "Up to date"/"Scanning" would claim liveness while nothing is
  // checking the chain, and the Idle default's "starts automatically" would
  // be a lie. Mutually exclusive with [startFailed] by the drive enum (a
  // disabled host never issues the start that could fail).
  if (syncDisabled) {
    return SyncStatusPresentation(
      icon: WalletGlyph.syncOff,
      headline: l10n.walletSyncDisabled,
      details: [l10n.walletSyncDisabledDetail],
    );
  }
  final p = _statusPresentation(
    l10n,
    status,
    driving: driving,
    startFailed: startFailed,
  );
  // A failed START under a retained NON-Idle status (#356-F8, second half):
  // the stream keeps the last good value across a pause, so a resume-time
  // start fault can sit under "Up to date"/"Scanning" — the badge's a11y
  // label and the sheet must carry the failure line there too, never a pure
  // healthy story beside the retry notice. (The Idle arm already swaps its
  // own copy inside the mapping.)
  if (startFailed && status is! SyncStatus_Idle) {
    return SyncStatusPresentation(
      // The icon agrees with the capped (caution) color — an orange
      // check-circle under a retained "Up to date" would be a contradictory
      // glyph/color pair (UX LOW); the Idle arm already swaps its own.
      icon: WalletGlyph.syncProblem,
      headline: p.headline,
      trailing: p.trailing,
      details: [...p.details, l10n.walletSyncStartFailed],
      scanning: p.scanning,
      progress: p.progress,
    );
  }
  return p;
}

SyncStatusPresentation _statusPresentation(
  WalletLocalizations l10n,
  SyncStatus status, {
  required bool driving,
  required bool startFailed,
}) {
  switch (status) {
    case SyncStatus_Idle():
      if (startFailed) {
        // The START command itself failed ([WalletSyncDrive.failed]) — the
        // Idle default's "starts automatically" would contradict the surface's
        // own retry notice (#356-F8). Say what actually happened; the sheet
        // adds the retry affordance.
        return SyncStatusPresentation(
          icon: WalletGlyph.syncProblem,
          headline: l10n.walletSyncIdle,
          details: [l10n.walletSyncStartFailed],
        );
      }
      if (driving) {
        // The loop IS started but hasn't reported a batch yet — it's reaching
        // the server and fetching the commitment-tree roots + chain tip (the
        // prep phase the SDK emits no status for). Show that honestly, never a
        // stale "Not syncing yet" (the maintainer report).
        return SyncStatusPresentation(
          icon: WalletGlyph.connecting,
          headline: l10n.walletSyncStarting,
          details: [l10n.walletSyncStartingDetail],
        );
      }
      // Genuinely not running (the brief pre-start instant / stopped).
      return SyncStatusPresentation(
        icon: WalletGlyph.syncIdle,
        headline: l10n.walletSyncIdle,
        details: [l10n.walletSyncIdleDetail],
      );
    case SyncStatus_Connecting(:final torBootstrapPercent):
      return SyncStatusPresentation(
        icon: WalletGlyph.connecting,
        headline: torBootstrapPercent == null
            ? l10n.walletSyncConnecting
            : l10n.walletSyncConnectingPercent(percentOf(torBootstrapPercent)),
      );
    case SyncStatus_Scanning(
      :final from,
      :final to,
      :final percent,
      :final spendableReady,
    ):
      final pct = scanPercent(percent);
      final String headline;
      final double? progress;
      String? trailing;
      final details = <String>[];
      if (pct == 0) {
        // The opaque early phase: the monotonic note-fraction is still ~0 for
        // a long stretch of a deep first sync (notes are found later), so a
        // determinate "0%" bar reads as STUCK. Animate an indeterminate bar
        // and say WHY it is slow — honest (we have no meaningful fraction yet)
        // and visibly alive. The headline drops the number entirely here
        // ("Scanning…", not "Scanning 0%"): a literal 0% that never moves for
        // minutes reads as frozen even with the live bar (maintainer report).
        headline = l10n.walletSyncScanningEarly;
        progress = null;
        details.add(l10n.walletSyncCatchingUp);
      } else {
        headline = l10n.walletSyncScanning(pct);
        // Finite-guard the bar: `clamp` does NOT sanitize NaN, and a NaN value
        // poisons LinearProgressIndicator (the bridge guarantees finiteness;
        // this is the honest belt).
        progress = percent.isFinite ? percent.clamp(0.0, 1.0) : 0.0;
        // The blocks-left countdown rides the SAME ROW as the percent (maintainer:
        // "keep block counts near percentage"), COMPACT (1.6M, not 1,579,873)
        // and dim. `to - from` is the SDK's percent-DERIVED monotonic remaining
        // (BlockHeight `from` = scanned_equiv, so to - from = round(span·(1-%)));
        // it AGREES with `pct` by construction and counts DOWN — never the old
        // jumping "1%, 289 blocks left" the raw download frontier produced.
        // Suppressed at a tiny span to avoid a 1-block grammar edge and the
        // brief spend-before-sync frontier shimmer as funds clear near the tip.
        final remaining = to - from;
        if (remaining >= 2) {
          trailing = l10n.walletSyncScanRemaining(
            compactBlockCount(remaining, l10n.localeName),
          );
        }
      }
      // Spend-before-sync: funds can be usable well before 100% (§1.7) — the
      // cue that turns the badge GREEN and unlocks Send.
      if (spendableReady) details.add(l10n.walletSyncSpendableReady);
      return SyncStatusPresentation(
        icon: WalletGlyph.syncing,
        headline: headline,
        trailing: trailing,
        details: details,
        scanning: true,
        progress: progress,
      );
    case SyncStatus_UpToDate():
      return SyncStatusPresentation(
        icon: WalletGlyph.success,
        headline: l10n.walletSyncUpToDate,
      );
    // Ironwood/NU6.3 (`ironwood-nu63-support.md` §6.4): scanned to the tip, but
    // this version could not read every block it passed. Deliberately NOT the
    // check glyph and NOT the up-to-date headline — the whole reason the core
    // emits a distinct state is that a confident "synced" here would be a false
    // claim about the user's money (the balance beside it is a floor, not a
    // total). Caution glyph, qualified headline, and the sheet explains both
    // consequences.
    case SyncStatus_UpToDateLimited():
      return SyncStatusPresentation(
        icon: WalletGlyph.upToDateLimited,
        headline: l10n.walletSyncUpToDateLimited,
      );
    // T0-1b: scanned to the tip, but THIS SERVER refused, withheld or lied
    // about one pool's subtree roots. The same reasoning as the arm above — a
    // confident "synced" would be a false claim about money received in that
    // pool — with a different next step: not "update the app" but "switch
    // servers". A server glyph, not the update one, so the two are never read
    // as the same advice; the sheet names the step.
    case SyncStatus_UpToDateDegraded(:final pools):
      return SyncStatusPresentation(
        icon: WalletGlyph.serverDegraded,
        headline: l10n.walletSyncUpToDateDegraded,
        // §4r U-3 (closing §4j row 8 by rendering): WHICH pool, and how. The
        // core publishes this arm exactly when [poolReportIsDegraded] is true
        // of a current server's report; the gate is the shared predicate all
        // the same, so this carrier and the two below cannot drift on it.
        details: poolReportIsDegraded(pools)
            ? poolServiceLines(l10n, pools)
            : const [],
      );
    // T0-1c: scanned to THIS SERVER's tip, and that tip is below a height the
    // network passed before this build shipped — the server is behind the
    // chain. The same reasoning as the two arms above (a confident "synced"
    // would be a false claim about how current the balance is) and the same
    // next step as the arm above: "switch servers". A history glyph — behind
    // in TIME — so it is never read as the update arm (this version is fine)
    // or as the pool arm (every pool may well be served); the sheet names the
    // step.
    case SyncStatus_EndpointBehind(:final pools):
      return SyncStatusPresentation(
        icon: WalletGlyph.behind,
        headline: l10n.walletSyncEndpointBehind,
        // §4m #10 / §4r U-3: in the core's precedence the behind claim
        // outranks the pool claim and carries the report WHOLE, so a withheld
        // or refused pool must still be said HERE — the lines follow the
        // behind explanation on the sheet (details render under it), and the
        // degraded explain's money fact ("funds in that pool can't be spent
        // through it") is not hidden by the ranking. LIVE, not dormant
        // (§4r Q-U1): `emit_synced` in `sync_controller.rs` computes
        // `pool_report(pass.root_outcomes)` BEFORE the precedence chain and
        // this arm carries it, and the production engine fills
        // `root_outcomes` on every completed pass (`Wallet::sync_once`). Null
        // is a pass that made no pool claim (a fake engine) — nothing to say;
        // a report with every pool served renders no line either.
        details: pools != null && poolReportIsDegraded(pools)
            ? poolServiceLines(l10n, pools)
            : const [],
      );
    // GRACE-1 (`production-readiness-phase-1.md` §4p): scanned to the tip,
    // but THIS SERVER will not say which network it is on. The same reasoning
    // as the three arms above (a confident "synced" would hide that the
    // app cannot confirm a payment it signs will be accepted) and the same
    // next step as the two above it: "switch servers" — NEVER "update the
    // app" (the Limited arm; nothing was upgraded here). The detail line is
    // the grace itself: while it runs, how much is left (whichever of blocks
    // and time runs out first — the SDK already converted the blocks, and
    // hands no time at all when the device clock cannot be trusted for it,
    // so the blocks show alone); once it has ended, why, and the next step
    // (for the clock: a wrong device date and time is fixed FIRST, then the
    // switch — a precondition, never an alternative, because a corrected
    // clock alone re-permits nothing; §4p-run fold review row 6). A question
    // glyph — unverified — so it is never read as the update, pool or
    // behind arm. This arm carries the pool report too (the core's
    // precedence drops no pool fact), so a degraded pool's lines follow the
    // grace line (§4r U-3).
    case SyncStatus_UpToDateUnverified(
      :final grace,
      :final pools,
      :final streakReported,
    ):
      return SyncStatusPresentation(
        icon: WalletGlyph.unknown,
        headline: l10n.walletSyncUnverified,
        details: [
          unknownBranchGraceText(l10n, grace),
          // P3-12 fold (security review, MEDIUM 1): the streak this claim
          // outranks reaches the BADGE too — its screen-reader label is the
          // headline plus these lines, so a user who never opens the sheet
          // still hears it. The explanation (below) drops "your balance is
          // current" for the same reason.
          if (streakReported) l10n.walletSyncUnverifiedStreakDetail,
          if (pools != null && poolReportIsDegraded(pools))
            ...poolServiceLines(l10n, pools),
        ],
      );
    case SyncStatus_Stalled(:final reason):
      return SyncStatusPresentation(
        // #399: the connectivity stall wears the OFFLINE glyph (the same
        // cloud_off as the reserved Offline arm) — its level is caution (the
        // wallet_health normal-offline demotion), and an alarm glyph beside
        // an orange tint would be the contradictory pair the rule
        // forbids. Hard stalls keep the error glyph. R10: a storage pause is
        // caution too and not a connection problem, so it wears the syncing
        // glyph ("retrying"), never cloud_off.
        icon: switch (reason) {
          StallReason.endpointUnreachable => WalletGlyph.offline,
          StallReason.storageUnavailable => WalletGlyph.syncing,
          _ => WalletGlyph.error,
        },
        headline: l10n.walletSyncStalled,
        details: [stallReasonText(l10n, reason)],
      );
    case SyncStatus_Offline():
      return SyncStatusPresentation(
        icon: WalletGlyph.offline,
        headline: l10n.walletSyncOffline,
        details: [l10n.walletSyncOfflineDetail],
      );
    // Forward-compat arm: render as neutral "syncing", never a healthy or
    // an alarming state (spec §3.3 unknown handling); the level is YELLOW.
    case SyncStatus_Unknown():
      return SyncStatusPresentation(
        icon: WalletGlyph.syncing,
        headline: l10n.walletSyncUnknown,
      );
  }
}

/// The plain-language sheet explanation of the CURRENT state (what it means,
/// whether the balance is trustworthy, what — if anything — to do). Sheet-only
/// copy; the badge headline stays the compact form above.
String syncStatusExplanation(
  WalletLocalizations l10n,
  SyncStatus status, {
  required bool driving,
  bool startFailed = false,
  bool syncDisabled = false,
}) {
  // #383 R1: sync-off owns the slot on every arm (same shape as startFailed
  // below) — the honest answer to "what's going on" is that the HOST turned
  // syncing off, figures show the last synced state, and the way back is the
  // host's own settings (the copy points there, not at a package affordance).
  if (syncDisabled) return l10n.walletSyncExplainDisabled;
  // A failed START owns the explanation slot on EVERY arm (#356-F8): the
  // truthful answer to "what's going on" is that the loop isn't running —
  // an Idle "starts automatically, no action needed" AND a retained
  // UpToDate "your balance is current" both contradict the retry sitting
  // right below this text. The headline/figures still show the last-known
  // state; only the explanation flips.
  if (startFailed) return l10n.walletSyncExplainStartFailed;
  return switch (status) {
    SyncStatus_Idle() =>
      driving ? l10n.walletSyncExplainStarting : l10n.walletSyncExplainIdle,
    SyncStatus_Connecting() => l10n.walletSyncExplainConnecting,
    SyncStatus_Scanning() => l10n.walletSyncExplainScanning,
    SyncStatus_UpToDate() => l10n.walletSyncExplainUpToDate,
    SyncStatus_UpToDateLimited() => l10n.walletSyncExplainUpToDateLimited,
    SyncStatus_UpToDateDegraded() => l10n.walletSyncExplainUpToDateDegraded,
    SyncStatus_EndpointBehind() => l10n.walletSyncExplainEndpointBehind,
    // P3-12 (maintainer, Q2 option 2): under a REPORTED rewinding streak the
    // grace claim still wins the ranking (P2-6 — the countdown is never
    // hidden), but the explanation drops "your balance is current" and names
    // what the claim outranked: a server the loop judged misbehaving. Same next
    // step, "switch servers".
    SyncStatus_UpToDateUnverified(streakReported: true) =>
      l10n.walletSyncExplainUnverifiedStreak,
    SyncStatus_UpToDateUnverified() => l10n.walletSyncExplainUnverified,
    // #399: the connectivity stall tells the calm normal-offline story (funds
    // safe, last synced state, queued sends drain on a future online sync) —
    // the generic "hit a problem" explanation matches only the hard stalls.
    SyncStatus_Stalled(reason: StallReason.endpointUnreachable) =>
      l10n.walletSyncExplainStalledOffline,
    SyncStatus_Stalled() => l10n.walletSyncExplainStalled,
    SyncStatus_Offline() => l10n.walletSyncExplainOffline,
    SyncStatus_Unknown() => l10n.walletSyncExplainUnknown,
  };
}

/// The one sentence for where the grace for a server that does not report
/// its network stands (GRACE-1, §4p Q-G2) — the detail line under
/// [SyncStatus.upToDateUnverified], and, for an ended grace, the body of the
/// send fault (`SendServerSilentFault`) and the parked row's reason, so the
/// three surfaces can never say different things about the same state.
///
/// Running: the time left is whichever rule expires first (the SDK converted
/// the blocks through the network's block spacing); when the device clock
/// cannot be trusted for it the SDK hands no time, and the blocks show alone
/// (§4p G-4 — no separate string for that case). Ended: the reason names the
/// rule — blocks ("for N blocks"), the clock ("for a day" — a wrong device
/// time is the one benign cause, so the copy says to fix it FIRST and then
/// switch: a precondition of the next step, never an alternative to it,
/// because a corrected clock alone re-permits nothing — §4p-run fold review
/// row 6, §4r U-5), or never confirmed. NEVER "upgraded" and NEVER "update
/// the app": nothing is known to have changed, and an update fixes nothing —
/// a server that reports the network version is the next step on every
/// branch.
String unknownBranchGraceText(
  WalletLocalizations l10n,
  UnknownBranchGrace grace,
) {
  return switch (grace) {
    UnknownBranchGrace_Running(:final blocksLeft, :final secsLeft) =>
      secsLeft == null
          ? l10n.walletSyncGraceLeftBlocks(
              compactBlockCount(blocksLeft, l10n.localeName),
            )
          : l10n.walletSyncGraceLeftHours(secsLeft ~/ 3600),
    UnknownBranchGrace_Ended(:final by, :final blocksSinceLastCurrent) =>
      graceEndedText(l10n, by, blocksSinceLastCurrent),
    // Forward-compat: a shape this UI does not know is rendered as the ended
    // state's generic sentence — never as a running grace.
    UnknownBranchGrace_Unknown() => l10n.walletSyncGraceNeverConfirmed,
  };
}

/// Why the grace ended, as one sentence with the next step — see
/// [unknownBranchGraceText]. `blocksSinceLastCurrent` is what the blocks
/// sentence names; a missing count falls to the never-confirmed sentence
/// rather than inventing a number.
String graceEndedText(
  WalletLocalizations l10n,
  GraceExpiry by,
  int? blocksSinceLastCurrent,
) {
  return switch (by) {
    GraceExpiry.blocks when blocksSinceLastCurrent != null =>
      l10n.walletSyncGraceEndedBlocks(
        compactBlockCount(blocksSinceLastCurrent, l10n.localeName),
      ),
    GraceExpiry.clock => l10n.walletSyncGraceEndedClock,
    GraceExpiry.blocks ||
    GraceExpiry.neverConfirmed ||
    GraceExpiry.unknown => l10n.walletSyncGraceNeverConfirmed,
  };
}

String stallReasonText(WalletLocalizations l10n, StallReason reason) {
  switch (reason) {
    case StallReason.endpointUnreachable:
      return l10n.walletStallEndpoint;
    case StallReason.torUnavailable:
      return l10n.walletStallTor;
    case StallReason.storageFull:
      return l10n.walletStallStorage;
    case StallReason.chainReorg:
      return l10n.walletStallReorg;
    case StallReason.internal:
      return l10n.walletStallInternal;
    // T0-1b: the server ANSWERED and the answer was wrong — the mirror image
    // of `internal`. Copy says "switch servers", never "check your
    // connection" (that is the endpoint arm) and never "restore" (internal).
    case StallReason.endpointMisbehaving:
      return l10n.walletStallEndpointMisbehaving;
    // T0-1c-R2: the wallet's starting height is above the chain as this
    // server reports it. The wallet cannot tell a behind server from a
    // starting height set above the real tip — and that height has TWO
    // producers, the birthday typed at restore and `rescan_from(Some(h))`
    // (§4n-review row 7), so the copy says "the starting block this wallet
    // is set to", never "you entered when restoring". It names both next
    // steps — check the height, or try another server — and never "check
    // your connection" or "restore".
    case StallReason.birthdayInFuture:
      return l10n.walletStallBirthdayInFuture;
    // R10: this device's storage was busy or briefly unreadable (a WAL race, a
    // locked iPhone) — transient, the store is intact, and the SDK retries.
    // Never "restore" (that is `internal`, a corrupt store) and never a
    // connection or server remedy.
    case StallReason.storageUnavailable:
      return l10n.walletStallStorageUnavailable;
    case StallReason.unknown:
      return l10n.walletStallUnknown;
  }
}

/// The Dart mirror of `report_is_degraded` in
/// `sdk/zec-wallet-core/src/sync_controller.rs` ("is at least one pool in the
/// report refused, lied about, or withheld — the condition under which the
/// report earns its own status") — ONE predicate (§4r U-3), so the surface
/// and the core agree on what "degraded" means. The core publishes
/// `upToDateDegraded` exactly when this is true of a current server's report
/// and carries the report WHOLE on `endpointBehind` and `upToDateUnverified`,
/// degraded or not — so this is what decides whether those arms render pool
/// lines at all, and a report with every pool served renders none.
bool poolReportIsDegraded(PoolServiceReport report) =>
    _poolServiceIsDegraded(report.sapling) ||
    _poolServiceIsDegraded(report.orchard) ||
    _poolServiceIsDegraded(report.ironwood);

/// The per-pool arm of [poolReportIsDegraded], mirroring the Rust
/// `pool_is_degraded`: a served pool is fine at ANY count (the zero the
/// wallet CAN grade has already become `withheld` in the core). Exhaustive
/// and wildcard-free like the Rust, so a variant added to the bridge is a
/// compile error HERE — never a pool computed, crossed and rendered nowhere
/// (§4j row 8). The one arm the Rust does not have, `unknown` (the bridge's
/// forward-compat shape), counts as degraded: its own doc says "never as
/// healthy" (spec §3.3 unknown handling).
bool _poolServiceIsDegraded(PoolService service) => switch (service) {
  PoolService_Served() => false,
  PoolService_Withheld() ||
  PoolService_Unsupported() ||
  PoolService_HeightViolation() ||
  PoolService_Unknown() => true,
};

/// One detail line per pool [_poolServiceIsDegraded] counts — the pool
/// named, then how this server failed it ("Sapling: this server refuses to
/// serve it") — and NOTHING for a pool served normally. In the core's field
/// order (Sapling, Orchard, Ironwood: `SUBTREE_ROOT_POOLS`' wire order). This
/// is the money fact the headline cannot carry (§4m #10): WHICH pool's funds
/// cannot be spent through this server, so "the balance is a floor" has a
/// subject. The pool names are l10n keys of their own (proper nouns a locale
/// may keep or transliterate), never Dart literals.
List<String> poolServiceLines(
  WalletLocalizations l10n,
  PoolServiceReport report,
) => [
  for (final (pool, service) in [
    (l10n.walletPoolSapling, report.sapling),
    (l10n.walletPoolOrchard, report.orchard),
    (l10n.walletPoolIronwood, report.ironwood),
  ])
    if (_poolServiceIsDegraded(service)) ?_poolServiceLine(l10n, pool, service),
];

/// The line for ONE degraded pool — one l10n key per [PoolService] state
/// that renders, `walletSyncPool<State>`, with the pool name as the
/// placeholder (§4r U-3). `served` has no line by construction (null), which
/// is why [poolServiceLines] filters through the predicate first: the two can
/// only ever agree.
String? _poolServiceLine(
  WalletLocalizations l10n,
  String pool,
  PoolService service,
) => switch (service) {
  PoolService_Served() => null,
  PoolService_Withheld() => l10n.walletSyncPoolWithheld(pool),
  PoolService_Unsupported() => l10n.walletSyncPoolUnsupported(pool),
  PoolService_HeightViolation() => l10n.walletSyncPoolHeightViolation(pool),
  PoolService_Unknown() => l10n.walletSyncPoolUnknown(pool),
};

/// 0..1 fraction → a rounded whole percent (Tor bootstrap progress, where
/// reaching 100 is not a money claim). The bridge guarantees finite
/// percents; `clamp` alone would pass a NaN through, so guard finiteness
/// explicitly (belt that actually holds).
int percentOf(double fraction) {
  final f = fraction.isFinite ? fraction.clamp(0.0, 1.0) : 0.0;
  return (f * 100).round();
}

/// 0..1 SCAN fraction → a whole percent that NEVER reads 100 before sync is
/// actually done: `round` would show "Scanning 100%" at 0.995+ while the
/// wallet is provably not up-to-date (the distinct `UpToDate` arm is the
/// only honest 100%) — a money-honesty trap (a user trusts a partial
/// balance as final). Floor, and cap at 99 while scanning.
int scanPercent(double fraction) {
  final f = fraction.isFinite ? fraction.clamp(0.0, 1.0) : 0.0;
  final p = (f * 100).floor();
  return p >= 100 ? 99 : p;
}

/// Compact block-count formatter (1,579,873 → "1.6M"), locale-aware (#317: the
/// old locale-DEFAULT static froze to whatever `Intl.defaultLocale` was at
/// first use — an "1.6M" inside a ru sentence). [locale] is
/// `WalletLocalizations.of(context).localeName`, so the digits follow the
/// strings around them. Built once PER LOCALE (a formatter per rebuild is
/// wasteful even at the badge's low update rate; the cache is bounded by the
/// app's locale set).
String compactBlockCount(int count, String locale) => _compactBlockFmts
    .putIfAbsent(locale, () => NumberFormat.compact(locale: locale))
    .format(count);
final Map<String, NumberFormat> _compactBlockFmts = <String, NumberFormat>{};

/// Exact grouped block count for the DETAIL sheet ("1,579,873") — the sheet is
/// where the big number belongs (the badge deliberately demotes it to compact;
/// the maintainer's "scary jittery number" report). Locale-aware and memoized
/// like [compactBlockCount].
String exactBlockCount(int count, String locale) => _exactBlockFmts
    .putIfAbsent(locale, () => NumberFormat.decimalPattern(locale))
    .format(count);
final Map<String, NumberFormat> _exactBlockFmts = <String, NumberFormat>{};

/// The activity/parked rows' compact date+time (`MMMd + jm`, e.g. "Jun 24,
/// 3:45 PM"), locale-aware and memoized per locale (#317 — the old per-widget
/// statics froze to `Intl.defaultLocale`, an en-US time inside a ru sentence).
/// ONE factory shared by every row surface so the idiom can't drift.
DateFormat walletCompactTimeFormat(String locale) => _compactTimeFmts
    .putIfAbsent(locale, () => DateFormat.MMMd(locale).add_jm());
final Map<String, DateFormat> _compactTimeFmts = <String, DateFormat>{};

/// ONE place that appends the sync-paused qualifier to a money body which
/// promises the wallet will finish something on a later sync (#401 R1b/R5).
///
/// The SAVED-FOR-RETRY results (send, partial send, shield, move-to-transparent)
/// all say some form of "your wallet will send this on a later sync". That
/// promise is custody-INDEPENDENT — the transaction is already SIGNED, and the
/// §6.1 `ReBroadcast` arm re-sends the raw bytes with no seed, unlike a QUEUED
/// intent, which needs a credential a background pass may not hold. But it is
/// PASS-dependent: `after_synced` is the only thing that drives it. With no
/// passes the money sits signed and unsent behind copy saying it is handled, and
/// a user who believes nothing happened re-enters the payment — a double pay.
///
/// Shared rather than repeated at the three call sites so the four bodies can
/// never drift on whether they carry the qualifier at all. The note names the
/// CONDITION and no remedy: `walletSyncPassesRunProvider` is false under BOTH
/// the host's sync-off policy and a failed sync start, whose remedies differ,
/// and the screen already carries whichever one applies.
///
/// THE SEPARATOR IS THE LOCALE'S, NOT DART'S (#403 R6). The first version joined
/// with `'$body $note'`, which puts a U+0020 after a fullwidth 。 in ja and zh —
/// wrong typography in both, applied here to four money bodies at once. This
/// package had already ADJUDICATED that exact pattern: `in_flight_sends_section`
/// records how abandoned sentence-concatenation for a whole VARIANT key
/// for this reason. A joiner ARB key is the cheap form of the same decision:
/// ja/zh translate it as `{body}{note}`, every other locale keeps the space.
String walletSyncPausedQualified(
  WalletLocalizations l10n,
  String body, {
  required bool syncPassesRun,
}) => syncPassesRun
    ? body
    : l10n.walletSyncPausedJoin(body, l10n.walletSyncPausedMoneyNote);

/// The detail sheet's FULL date+time (`yMMMd + jm` — the sheet is where the
/// exact moment belongs; rows keep the compact form). Locale-aware, memoized.
DateFormat walletFullDateTimeFormat(String locale) => _fullDateTimeFmts
    .putIfAbsent(locale, () => DateFormat.yMMMd(locale).add_jm());
final Map<String, DateFormat> _fullDateTimeFmts = <String, DateFormat>{};

// ---------------------------------------------------------------------------
// Network transport presentation ("Tor or any other proxy
// transport like xray/vless which host app can provide" — an indicator on the
// fixed sync row, the full story in the tap-through sheet).
// ---------------------------------------------------------------------------

/// A HOST-provided transport description — for hosts that route wallet
/// traffic through their OWN privacy layer (an app-wide proxy: xray/vless,
/// a VPN tunnel, a custom Tor integration) that the SDK cannot see. The SDK's
/// own [TorState] then honestly reads "Tor off" even though the traffic IS
/// protected — this override lets the host tell the truth instead. Wired via
/// `walletHostTransportProvider` (wallet_providers.dart); `null` (the default)
/// derives the presentation from the SDK's [TorState].
class WalletHostTransport {
  const WalletHostTransport({
    required this.label,
    required this.protection,
    this.detail,
  });

  /// Short, HOST-LOCALIZED display label ("VLESS", "Proxy", "VPN"). Shown on
  /// the sync sheet's Connection row and in the badge's a11y label — keep it
  /// to one or two words.
  final String label;

  /// Whether this transport actually hides the user's network identity from
  /// the lightwalletd server. Drives the indicator tone: only `true` earns
  /// the protected (green, filled-shield) treatment — a plain load balancer
  /// or an unencrypted forward proxy must pass `false` (the §3.3 privacy
  /// rule: never claim a protection that is not running).
  final bool protection;

  /// Optional HOST-LOCALIZED sheet explanation. When null, a generic
  /// proxy/direct explanation (keyed off [protection]) is used.
  final String? detail;
}

/// The indicator tone for a transport state — mapped to theme colors at the
/// render (the same pattern as `WalletBadgeLevel`): [protected] green,
/// [neutral] muted (informational — a direct connection is the configured
/// default, not an error), [progress] cyan (bootstrapping), [caution] orange
/// (fell back / unverifiable), [danger] red (required but unavailable).
enum TransportTone { protected, neutral, progress, caution, danger }

/// Pure per-state presentation of HOW wallet traffic reaches the network —
/// ONE mapping shared by the sync-row indicator and the sheet's Connection
/// section (the badge/sheet no-drift rule, same as [syncStatusPresentation]).
class TransportPresentation {
  const TransportPresentation({
    required this.label,
    required this.tone,
    required this.detail,
  });

  /// One/two-word transport label ("Tor active", "Tor off", a host's "VLESS").
  final String label;

  final TransportTone tone;

  /// The sheet's plain-language explanation of what this means for privacy.
  final String detail;
}

/// Map the SDK's [TorState] — or the HOST's own transport claim, which WINS
/// when provided (the host knows about tunnels the SDK cannot see) — to the
/// shared presentation. PRIVACY RULE (spec §3.3): every arm that cannot
/// POSITIVELY verify protection reads as not protected; only a verified Tor
/// runtime or an explicit host `protection: true` earns the protected tone.
TransportPresentation transportPresentation(
  WalletLocalizations l10n, {
  required TorState tor,
  WalletHostTransport? host,
}) {
  if (host != null) {
    return TransportPresentation(
      label: host.label,
      tone: host.protection ? TransportTone.protected : TransportTone.neutral,
      detail:
          host.detail ??
          (host.protection
              ? l10n.walletTransportExplainHostProxy
              : l10n.walletTransportExplainDirect),
    );
  }
  switch (tor) {
    case TorState_Off():
      return TransportPresentation(
        label: l10n.walletTorOff,
        tone: TransportTone.neutral,
        detail: l10n.walletTransportExplainDirect,
      );
    // FR-30 (a): the failing arms name the host's transport the way the two
    // `Active` arms do, or name none at all — never the hard-coded noun "Tor"
    // (a host that registered Shadowsocks read "Tor starting…" until C1).
    case TorState_Bootstrapping(:final transport):
      final named = _declaredTransport(transport);
      return TransportPresentation(
        label: named == null
            ? l10n.walletTorBootstrapping
            : l10n.walletTorBootstrappingNamed(named),
        tone: TransportTone.progress,
        detail: named == null
            ? l10n.walletTransportExplainBootstrapping
            : l10n.walletTransportExplainBootstrappingNamed(named),
      );
    case TorState_Active(:final runtime):
      return switch (runtime) {
        // The protection claim is only as trustworthy as the runtime making
        // it: an UNKNOWN runtime (a forward-compat arm this binding can't
        // attribute) must NOT inherit the confident green "Tor active" (§3.3
        // privacy rule).
        TorRuntimeKind_Unknown() => TransportPresentation(
          label: l10n.walletTorActiveUnverified,
          tone: TransportTone.caution,
          detail: l10n.walletTransportExplainUnverified,
        ),
        // FR-29 / ADR-0547: the dialer the host's native library registered —
        // render WHAT THE HOST DECLARED and nothing more (spec §3.4): the
        // host's OWN name for its path, whether the wallet's connections can
        // be linked on it, and whether it hides the device's address. An
        // exposed path is not private, whatever the policy said.
        TorRuntimeKind_HostDialer(
          :final name,
          :final isolation,
          :final exposure,
        ) =>
          hostDialerPresentation(
            l10n,
            name: name,
            isolation: isolation,
            exposure: exposure,
            sentences: HostDialerSentences.active,
          ),
        // Two runtimes, not three: the SDK-owned `builtIn` was removed at
        // FR-5 C1 (ADR-0548 D3) — a host without a transport of its own adds
        // the `zec_wallet_tor` plugin, which arrives as a host dialer above.
        //
        // `ExternalSocks5` is the ONE arm that may still say "Tor": the
        // config variant is "host-side Tor speaking SOCKS5 (Orbot, system
        // Tor, a host sidecar)" (config.rs `TorRuntime`), so the host named
        // Tor by choosing it — the SDK is repeating a declaration, not making
        // one. It is refused at the config door today (T0-6, ADR-0543), so
        // nothing reaches this arm in a shipping configuration.
        TorRuntimeKind_ExternalSocks5() => TransportPresentation(
          label: l10n.walletTorActive,
          tone: TransportTone.protected,
          detail: l10n.walletTransportExplainTor,
        ),
        // FR-32 (a): `Dialer` is an arbitrary byte-stream dialer a Rust host
        // injected, which `net/dialer.rs` says in as many words the SDK
        // "never knows or names" — it declared NO name, NO isolation and NO
        // exposure. It read "Tor active" in the PROTECTED tone until stage S1
        // `copy`, which asserted onion routing, in the confident colour, for
        // a path the SDK cannot attest — the §2.3 trust boundary the
        // `HostDialer` arm above was built to respect, and the same class as
        // FR-30 (a). It now says only what is true (wallet traffic is riding
        // the path the app supplied) and refuses the privacy claim. A Rust
        // host that KNOWS what it injected says so through
        // `WalletHostTransport` at the top of this function, which wins over
        // any SDK `TorState` — that is the "asks the host to say so" half of
        // FR-32 (a)'s product call, and it already exists.
        TorRuntimeKind_Dialer() => TransportPresentation(
          label: l10n.walletTorActiveUnattested,
          tone: TransportTone.caution,
          detail: l10n.walletTransportExplainUnverified,
        ),
      };
    case TorState_FellBack():
      return TransportPresentation(
        label: l10n.walletTorFellBack,
        tone: TransportTone.caution,
        detail: l10n.walletTransportExplainFellBack,
      );
    case TorState_Unavailable(:final transport):
      final named = _declaredTransport(transport);
      return TransportPresentation(
        label: named == null
            ? l10n.walletTorUnavailable
            : l10n.walletTorUnavailableNamed(named),
        tone: TransportTone.danger,
        detail: named == null
            ? l10n.walletTransportExplainUnavailable
            : l10n.walletTransportExplainUnavailableNamed(named),
      );
    // Stage S1 `truth` (FR-36): the path took the connection and nothing has
    // come back over it for a minute — the path or the wallet server, and the
    // SDK does not guess which. Not `danger`: nothing is known to have leaked
    // and the state is not the genuinely-down one (that is `Unavailable`
    // above, whose chip says "not connected"). Not protected either, since
    // nothing is carrying.
    //
    // Stage S1 `copy` split the chip from the sheet — a label states the two
    // attested facts, the sheet's detail carries the either/or AND the two
    // next steps, one per cause (the `walletStallBirthdayInFuture`
    // discipline: when the wallet cannot tell which of two things is wrong,
    // it names both steps rather than picking one).
    //
    // FR-44: the payload is `Active`'s ENTIRE payload — name, isolation and
    // exposure — so a `HostDialer` goes through the same function `Active`
    // does, with this family's sentences. It read only the NAME until then,
    // and a path the host declared exposed therefore read the hidden
    // family's sentence: the privacy loss the `Active` chip discloses
    // vanished the moment the path went quiet, and the host's own Network
    // tab (graded per-exposure) disagreed with the wallet tab about one
    // connection. One place decides whether a path may be called private;
    // two places is how these two arms diverged.
    case TorState_Unanswered(:final runtime):
      return switch (runtime) {
        TorRuntimeKind_HostDialer(
          :final name,
          :final isolation,
          :final exposure,
        ) =>
          hostDialerPresentation(
            l10n,
            name: name,
            isolation: isolation,
            exposure: exposure,
            sentences: HostDialerSentences.unanswered,
          ),
        // `ExternalSocks5` reads the transport-neutral pair: the host named Tor
        // by choosing that config variant, so the unqualified sentence repeats
        // a declaration rather than making one.
        //
        // Exhaustive and wildcard-free like the sibling switches: a runtime
        // added to the bridge is a compile error HERE. (Not at the bridge — see
        // the note on `TorRuntimeKind_Unknown` below.)
        TorRuntimeKind_ExternalSocks5() => TransportPresentation(
          label: l10n.walletTorUnanswered,
          tone: TransportTone.caution,
          detail: l10n.walletTransportExplainUnanswered,
        ),
        // `Dialer` and `Unknown` KEEP ACTIVE'S REFUSAL. They sat in the
        // arm above until the review, on the reasoning that they carry no
        // exposure and so have nothing for FR-44's switch to read. That is true
        // and it is beside the point: what `Active` refuses for these two
        // runtimes is not an exposure claim but an ATTESTATION claim. `Dialer`
        // is an arbitrary byte-stream dialer a Rust host injected, which
        // `net/dialer.rs` says the SDK "never knows or names", and `Active`
        // says so in its label AND its detail ("treat it as not private").
        //
        // Dropping that on the way into `unanswered` made the wallet's claim
        // STRONGER at the moment it knew less: "Private path in use (privacy
        // not verified)" while the path carried, plain "Private path connected"
        // once it went quiet. Same class as FR-44, same stage, different
        // runtime — and the tone cannot carry the difference, because both arms
        // are `caution`. `Unknown` is unreachable until the core enum gains a
        // variant, and is here so that the day it becomes reachable it is
        // already honest.
        TorRuntimeKind_Dialer() ||
        TorRuntimeKind_Unknown() => TransportPresentation(
          label: l10n.walletTorUnansweredUnattested,
          tone: TransportTone.caution,
          detail: l10n.walletTransportExplainUnansweredUnverified,
        ),
      };
    // PRIVACY RULE (spec §3.3): an uninterpretable Tor state is treated as
    // NOT protected — never inherit a benign framing.
    case TorState_Unknown():
      return TransportPresentation(
        label: l10n.walletTorUnknown,
        tone: TransportTone.caution,
        detail: l10n.walletTransportExplainUnverified,
      );
  }
}

/// The transport name a FAILING arm may render (FR-30 (a)), or `null` when
/// there is none to render and the locale's transport-neutral sentence is the
/// honest one. `null` arrives when nothing is registered or the runtime has no
/// registry; the empty string is the SDK's own unattributed rendering (a
/// host's empty name never crosses — the crossing refuses it) and is treated
/// the same way, so no arm can ever render an empty noun. The wallet names no
/// transport the host did not name (ADR-0547).
String? _declaredTransport(String? name) =>
    (name == null || name.isEmpty) ? null : name;

/// Which SENTENCE FAMILY [hostDialerPresentation] speaks for the payload it
/// is given. The privacy question — *may this path be called private?* — has
/// ONE answer per payload and is decided in one place for both; only the
/// words differ, because the two states say different things about carriage.
///
/// REQUIRED, never defaulted (FR-44): the unanswered arm inherited `Active`'s
/// payload with none of its exposure handling, and a defaulted parameter is
/// exactly that mistake made available to the next state. A caller that adds
/// one has to say which sentences it speaks, or it does not compile.
enum HostDialerSentences {
  /// `TorState.active` — wallet traffic is riding the declared path.
  active,

  /// `TorState.unanswered` (ADR-0554) — the path took the connection and
  /// nothing has come back over it for the maintainer's minute. Never the
  /// protected tone, whatever the exposure says: nothing is carrying.
  unanswered,
}

/// The presentation of
/// `TorState.active(runtime: hostDialer(name, isolation, exposure))` (FR-29
/// spec §3.4, ADR-0547 — the host NAMES its transport; the SDK has no list):
/// the chip carries the host's own name VERBATIM ("via your app's private
/// path (Tor)", "… (VLESS via Cloudflare)"), never a name the wallet chose;
/// an EMPTY name (only the SDK's unattributed rendering produces one) reads
/// as the locale's "a private path". `exposure` decides privacy: `exposed`
/// renders "not private (your app's direct connection)" — CAUTION tone since
/// FR-30 (c), the direct-connection explanation — whatever the isolation
/// says; `unknown` (not declared, or a value THIS binding cannot read)
/// renders linkable with
/// the caution tone and the unverified explanation, whatever the isolation
/// says (the wave review's MEDIUM: never the confident tone for a path the
/// wallet cannot vouch for); `hidden` renders the private path, and only
/// `hidden` + isolation `supported` earns the protected tone (§3.3 privacy
/// rule) — `unsupported` OR `unknown` isolation adds "; connections can be
/// linked by the proxy" with the caution tone (the state never promises
/// what the host did not declare).
///
/// SHARED WITH `TorState.unanswered` since FR-44, through [sentences]: that
/// state carries this exact payload, and the privacy verdict it implies must
/// not depend on which of the two arms is reading it. The exposure switch
/// below is the ONE place that decides it; each arm then picks its family's
/// words. The unanswered family is never [TransportTone.protected] — nothing
/// is carrying — so its tone cannot be what encodes the verdict, and its
/// SENTENCES do.
TransportPresentation hostDialerPresentation(
  WalletLocalizations l10n, {
  required String name,
  required IsolationSupport isolation,
  required TransportExposure exposure,
  required HostDialerSentences sentences,
}) {
  // An EMPTY name is the SDK's own unattributed rendering (a host's empty
  // name never crosses — the crossing refuses it): the locale's "a private
  // path". Only the arms that name the path read it.
  String transport() => name.isEmpty ? l10n.walletTorHostOtherTransport : name;
  // The unanswered family keeps `copy`'s named/unnamed SENTENCE split on the
  // one arm that has both (it reads "Private path connected" where there is
  // no name to show). That is a NAMING choice, not a privacy one — the
  // privacy verdict is the switch below, and it is the same either way.
  final named = name.isEmpty ? null : name;
  switch (exposure) {
    // FR-30 (c), C1: CAUTION, not neutral. The arms were each defensible
    // alone and the ORDERING a user read across them was not: the path that
    // shows the server this device's address was the calm colour while the
    // path that hides it — and only lets the proxy operator link the wallet's
    // own connections — was the alarmed one, so a person reading colour
    // concluded the direct connection was the safer choice. A privacy loss is
    // never the calm colour; exposed and linkable now both read as caution and
    // only hidden + isolated earns the protected tone. UI-only: the header
    // pins the linkable SENTENCE, never the tone.
    case TransportExposure.exposed:
      // The name is DISCARDED on both families: what matters is that the
      // server sees this device's address, whatever the host called the path.
      return switch (sentences) {
        HostDialerSentences.active => TransportPresentation(
          label: l10n.walletTorHostDirect,
          tone: TransportTone.caution,
          detail: l10n.walletTransportExplainDirect,
        ),
        HostDialerSentences.unanswered => TransportPresentation(
          label: l10n.walletTorUnansweredDirect,
          tone: TransportTone.caution,
          detail: l10n.walletTransportExplainUnansweredDirect,
        ),
      };
    case TransportExposure.unknown:
      return switch (sentences) {
        HostDialerSentences.active => TransportPresentation(
          label: l10n.walletTorHostPathLinkable(transport()),
          tone: TransportTone.caution,
          detail: l10n.walletTransportExplainUnverified,
        ),
        HostDialerSentences.unanswered => TransportPresentation(
          label: l10n.walletTorUnansweredLinkable(transport()),
          tone: TransportTone.caution,
          detail: l10n.walletTransportExplainUnansweredUnverified,
        ),
      };
    case TransportExposure.hidden:
      final isolated = isolation == IsolationSupport.supported;
      return switch (sentences) {
        HostDialerSentences.active => TransportPresentation(
          label: isolated
              ? l10n.walletTorHostPath(transport())
              : l10n.walletTorHostPathLinkable(transport()),
          tone: isolated ? TransportTone.protected : TransportTone.caution,
          detail: l10n.walletTransportExplainHostProxy,
        ),
        // The linkability sentence rides this family too: it is a fact about
        // the path the host declared, and the path does not stop being
        // linkable because it stopped answering. Only the protected tone is
        // withheld — `Active`'s one green arm has no twin here.
        HostDialerSentences.unanswered => TransportPresentation(
          label: isolated
              ? (named == null
                    ? l10n.walletTorUnanswered
                    : l10n.walletTorUnansweredNamed(named))
              : l10n.walletTorUnansweredLinkable(transport()),
          tone: TransportTone.caution,
          detail: named == null
              ? l10n.walletTransportExplainUnanswered
              : l10n.walletTransportExplainUnansweredNamed(named),
        ),
      };
  }
}
