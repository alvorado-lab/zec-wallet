import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/core/theme/icons.dart';
import 'package:zec_wallet_ui/features/wallet/sync_status_presentation.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_health.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations_en.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations_ja.dart';

/// Pure-mapping tests for the ONE badge/sheet presentation source.
/// The generated en localizations instantiate directly — no widget tree.
void main() {
  final l10n = WalletLocalizationsEn();

  // GRACE-1 (`production-readiness-phase-1.md` §4p G-5/G-6, the UI half): the
  // unverified arm is never the plain "Up to date" reading; while the grace
  // runs the detail line counts down on whichever rule expires first (hours
  // when the SDK hands a time, blocks alone when it does not — G-4, no new
  // string); once ended it names the rule and the next step; and none of the
  // grace copy says "upgraded" or "update" — that is the Limited arm's
  // sentence, for a different fault with a different fix.
  group('UpToDateUnverified (GRACE-1)', () {
    const running = SyncStatus.upToDateUnverified(
      tip: 100,
      grace: UnknownBranchGrace.running(blocksLeft: 1000, secsLeft: 6400),
      pools: null,
      streakReported: false,
    );
    const runningNoTime = SyncStatus.upToDateUnverified(
      tip: 100,
      grace: UnknownBranchGrace.running(blocksLeft: 1142, secsLeft: null),
      pools: null,
      streakReported: false,
    );
    const endedByClock = SyncStatus.upToDateUnverified(
      tip: 100,
      grace: UnknownBranchGrace.ended(
        by: GraceExpiry.clock,
        blocksSinceLastCurrent: 0,
      ),
      pools: null,
      streakReported: false,
    );
    const endedByBlocks = SyncStatus.upToDateUnverified(
      tip: 100,
      grace: UnknownBranchGrace.ended(
        by: GraceExpiry.blocks,
        blocksSinceLastCurrent: 1200,
      ),
      pools: null,
      streakReported: false,
    );
    const never = SyncStatus.upToDateUnverified(
      tip: 100,
      grace: UnknownBranchGrace.ended(
        by: GraceExpiry.neverConfirmed,
        blocksSinceLastCurrent: null,
      ),
      pools: null,
      streakReported: false,
    );

    test('is a distinct reading, never the plain up-to-date headline', () {
      for (final status in [running, runningNoTime, endedByClock]) {
        final p = syncStatusPresentation(l10n, status, driving: true);
        expect(p.headline, l10n.walletSyncUnverified);
        expect(p.headline, isNot(l10n.walletSyncUpToDate));
        expect(p.headline, isNot(l10n.walletSyncUpToDateLimited));
        expect(
          syncStatusExplanation(l10n, status, driving: true),
          l10n.walletSyncExplainUnverified,
        );
      }
    });

    // P3-12 (maintainer, Q2 option 2): the grace claim carries whether the
    // loop's rewinding streak has reached its report. The headline and the
    // countdown are unchanged (P2-6 — never hidden); the EXPLANATION drops
    // "your balance is current" and names the rewinds, with the same next
    // step. The plain arm keeps its sentence, so the two strings differ on
    // exactly that claim.
    test(
      'under a reported streak the explanation drops the balance sentence',
      () {
        const streak = SyncStatus.upToDateUnverified(
          tip: 100,
          grace: UnknownBranchGrace.running(blocksLeft: 1000, secsLeft: 6400),
          pools: null,
          streakReported: true,
        );
        final p = syncStatusPresentation(l10n, streak, driving: true);
        final plain = syncStatusPresentation(l10n, running, driving: true);
        expect(p.headline, l10n.walletSyncUnverified);
        // The grace line stays first (never hidden — P2-6); the streak line
        // follows it (security fold, MEDIUM 1: the badge's screen-reader
        // label is the headline + these lines, so the streak reaches a user
        // who never opens the sheet); the plain case has no such line.
        expect(p.details.first, plain.details.first);
        expect(p.details, [
          plain.details.first,
          l10n.walletSyncUnverifiedStreakDetail,
        ]);
        expect(
          plain.details,
          isNot(contains(l10n.walletSyncUnverifiedStreakDetail)),
        );
        expect(
          syncStatusExplanation(l10n, streak, driving: true),
          l10n.walletSyncExplainUnverifiedStreak,
        );
        expect(
          l10n.walletSyncExplainUnverified,
          contains('Your balance is current'),
        );
        expect(
          l10n.walletSyncExplainUnverifiedStreak,
          isNot(contains('balance is current')),
        );
        expect(
          l10n.walletSyncExplainUnverifiedStreak,
          contains('Switch to another server'),
        );
        // fold (security + crypto angles, MEDIUM): neither body may claim
        // that sending works — the grace line beside them owns that claim,
        // and an ENDED grace refuses to sign.
        for (final body in [
          l10n.walletSyncExplainUnverified,
          l10n.walletSyncExplainUnverifiedStreak,
        ]) {
          expect(body, isNot(contains('grace period')), reason: body);
          expect(body, isNot(contains('keeps working')), reason: body);
        }
      },
    );

    // arch review (LOW): a degraded pool report under a reported streak —
    // the grace line, then the streak line, then the pool lines, in that
    // order; the pool lines are what they are without the streak.
    test('a degraded pool report under a reported streak keeps every line', () {
      const report = PoolServiceReport(
        sapling: PoolService.withheld(proven: 12),
        orchard: PoolService.served(roots: 7),
        ironwood: PoolService.served(roots: 0),
      );
      const streakDegraded = SyncStatus.upToDateUnverified(
        tip: 100,
        grace: UnknownBranchGrace.running(blocksLeft: 1000, secsLeft: 6400),
        pools: report,
        streakReported: true,
      );
      const plainDegraded = SyncStatus.upToDateUnverified(
        tip: 100,
        grace: UnknownBranchGrace.running(blocksLeft: 1000, secsLeft: 6400),
        pools: report,
        streakReported: false,
      );
      final p = syncStatusPresentation(l10n, streakDegraded, driving: true);
      final q = syncStatusPresentation(l10n, plainDegraded, driving: true);
      expect(p.details.length, q.details.length + 1);
      expect(p.details[0], q.details[0]);
      expect(p.details[1], l10n.walletSyncUnverifiedStreakDetail);
      expect(p.details.sublist(2), q.details.sublist(1));
      expect(q.details.sublist(1), poolServiceLines(l10n, report));
    });

    test('a running grace counts down on the time the SDK handed (hours)', () {
      final p = syncStatusPresentation(l10n, running, driving: true);
      // 6,400 s → 1 whole hour: the smaller of the two rules, already chosen
      // by the SDK; the UI never converts blocks itself.
      expect(p.details, [l10n.walletSyncGraceLeftHours(1)]);
      expect(p.details.single, contains('1 more hour'));
    });

    test('a running grace with no trusted time shows the blocks alone', () {
      final p = syncStatusPresentation(l10n, runningNoTime, driving: true);
      expect(p.details, [
        l10n.walletSyncGraceLeftBlocks(compactBlockCount(1142, 'en')),
      ]);
      expect(p.details.single, isNot(contains('hour')));
    });

    test('an ended grace says why and what to do', () {
      final clock = syncStatusPresentation(l10n, endedByClock, driving: true);
      expect(clock.details, [l10n.walletSyncGraceEndedClock]);
      expect(clock.details.single, contains('date and time'));
      // UI-1 (§4r U-5): the clock is a PRECONDITION, not an alternative
      // — the sentence now reads "fix them first — then switch to a server
      // that reports the network version", so the next step is mid-sentence
      // and lowercase. The row keeps its question ("what to do" = switch
      // servers) and stops pinning the sentence-initial capital the old
      // "Switch …, or check …" shape had.
      expect(clock.details.single.toLowerCase(), contains('switch'));

      final blocks = syncStatusPresentation(l10n, endedByBlocks, driving: true);
      expect(blocks.details, [
        l10n.walletSyncGraceEndedBlocks(compactBlockCount(1200, 'en')),
      ]);
      expect(blocks.details.single, isNot(contains('date and time')));

      final none = syncStatusPresentation(l10n, never, driving: true);
      expect(none.details, [l10n.walletSyncGraceNeverConfirmed]);
    });

    test('the send fault shares the ended sentence', () {
      expect(
        graceEndedText(l10n, GraceExpiry.clock, 0),
        l10n.walletSyncGraceEndedClock,
      );
      expect(
        graceEndedText(l10n, GraceExpiry.blocks, 1200),
        l10n.walletSyncGraceEndedBlocks(compactBlockCount(1200, 'en')),
      );
      // A blocks reason with no count never invents a number.
      expect(
        graceEndedText(l10n, GraceExpiry.blocks, null),
        l10n.walletSyncGraceNeverConfirmed,
      );
    });

    test('no grace copy says upgraded or update', () {
      final all = [
        l10n.walletSyncUnverified,
        l10n.walletSyncExplainUnverified,
        l10n.walletSyncExplainUnverifiedStreak,
        l10n.walletSyncGraceLeftHours(0),
        l10n.walletSyncGraceLeftHours(1),
        l10n.walletSyncGraceLeftHours(5),
        l10n.walletSyncGraceLeftBlocks('1.2K'),
        l10n.walletSyncGraceEndedBlocks('1.2K'),
        l10n.walletSyncGraceEndedClock,
        l10n.walletSyncGraceNeverConfirmed,
        l10n.walletParkedBlockedByServerSilent,
        l10n.walletParkedBlockedByServerSilentClock,
      ];
      for (final s in all) {
        expect(s.toLowerCase(), isNot(contains('upgrad')), reason: s);
        expect(s.toLowerCase(), isNot(contains('update')), reason: s);
      }
      // And the update sentence is still its own, for the Limited arm only.
      expect(l10n.walletSendFaultNetworkUpgrade, contains('update'));
    });
  });

  test('scanPercent floors and caps at 99 — 100 belongs to UpToDate alone', () {
    expect(scanPercent(0.425), 42);
    expect(scanPercent(0.999), 99);
    expect(scanPercent(1.0), 99);
    // NaN/out-of-range are sanitized, never poison the bar or the headline.
    expect(scanPercent(double.nan), 0);
    expect(scanPercent(-3), 0);
    expect(scanPercent(7), 99);
  });

  test(
    'percentOf rounds and guards NaN (Tor bootstrap — not a money claim)',
    () {
      expect(percentOf(0.425), 43);
      expect(percentOf(1.0), 100);
      expect(percentOf(double.nan), 0);
    },
  );

  test(
    'idle arm is honest about a driving loop (Connecting, not Not-syncing)',
    () {
      final driving = syncStatusPresentation(
        l10n,
        const SyncStatus.idle(),
        driving: true,
      );
      expect(driving.headline, l10n.walletSyncStarting);
      expect(driving.details, [l10n.walletSyncStartingDetail]);

      final stopped = syncStatusPresentation(
        l10n,
        const SyncStatus.idle(),
        driving: false,
      );
      expect(stopped.headline, l10n.walletSyncIdle);

      // The sheet explanation follows the same fork.
      expect(
        syncStatusExplanation(l10n, const SyncStatus.idle(), driving: true),
        l10n.walletSyncExplainStarting,
      );
      expect(
        syncStatusExplanation(l10n, const SyncStatus.idle(), driving: false),
        l10n.walletSyncExplainIdle,
      );
    },
  );

  test('#356-F8: a failed START flips the idle arm to the honest failure copy — '
      'never "starts automatically / no action needed"', () {
    final failed = syncStatusPresentation(
      l10n,
      const SyncStatus.idle(),
      driving: false,
      startFailed: true,
    );
    // Still honestly "Not syncing yet" as the headline, but the detail says
    // WHY (the start failed), never the idle default's "starts automatically".
    expect(failed.headline, l10n.walletSyncIdle);
    expect(failed.details, [l10n.walletSyncStartFailed]);

    expect(
      syncStatusExplanation(
        l10n,
        const SyncStatus.idle(),
        driving: false,
        startFailed: true,
      ),
      l10n.walletSyncExplainStartFailed,
    );

    // A RETAINED non-idle status (the stream keeps the last good value
    // across a pause) must not tell a pure healthy story over a failed
    // drive: the explanation flips to the failure on EVERY arm, the
    // presentation gains the failure detail line, and an otherwise-GREEN
    // level caps at caution — never "Up to date, balance current" beside
    // a retry button (#356-F8 second half, reliability MED).
    expect(
      syncStatusExplanation(
        l10n,
        const SyncStatus.upToDate(tip: 5),
        driving: false,
        startFailed: true,
      ),
      l10n.walletSyncExplainStartFailed,
    );
    final retained = syncStatusPresentation(
      l10n,
      const SyncStatus.upToDate(tip: 5),
      driving: false,
      startFailed: true,
    );
    expect(retained.headline, l10n.walletSyncUpToDate);
    expect(retained.details, contains(l10n.walletSyncStartFailed));
    expect(
      walletBadgeLevel(
        status: const SyncStatus.upToDate(tip: 5),
        stale: false,
        startFailed: true,
      ),
      WalletBadgeLevel.caution,
    );
    // …but a failed drive never LOWERS an already-honest error level.
    expect(
      walletBadgeLevel(
        status: const SyncStatus.stalled(reason: StallReason.storageFull),
        stale: false,
        startFailed: true,
      ),
      WalletBadgeLevel.error,
    );
  });

  test('#383 R1: syncDisabled owns the WHOLE presentation over ANY retained '
      'arm — the honest "sync off" story, never a stale liveness claim', () {
    // A retained UpToDate (the stream keeps the last good value) must not
    // read "Up to date" while the host's policy stopped the loop.
    final p = syncStatusPresentation(
      l10n,
      const SyncStatus.upToDate(tip: 100),
      driving: false,
      syncDisabled: true,
    );
    expect(p.headline, l10n.walletSyncDisabled);
    expect(p.details, contains(l10n.walletSyncDisabledDetail));
    expect(p.scanning, isFalse, reason: 'no progress claim under sync-off');

    // The sheet explanation flips on the same flag, over the same arm.
    expect(
      syncStatusExplanation(
        l10n,
        const SyncStatus.upToDate(tip: 100),
        driving: false,
        syncDisabled: true,
      ),
      l10n.walletSyncExplainDisabled,
    );
  });

  test('scanning: determinate arm carries bar value + compact count + '
      'spendable cue', () {
    final p = syncStatusPresentation(
      l10n,
      const SyncStatus.scanning(
        from: 1000,
        to: 3000,
        percent: 0.5,
        spendableReady: true,
        rewound: false,
      ),
      driving: true,
    );
    expect(p.scanning, isTrue);
    expect(p.progress, 0.5);
    expect(p.headline, l10n.walletSyncScanning(50));
    expect(
      p.trailing,
      l10n.walletSyncScanRemaining(compactBlockCount(2000, l10n.localeName)),
    );
    expect(p.details, [l10n.walletSyncSpendableReady]);
  });

  test('scanning: the opaque early phase is number-less and indeterminate', () {
    final p = syncStatusPresentation(
      l10n,
      const SyncStatus.scanning(
        from: 1,
        to: 300000,
        percent: 0.0,
        spendableReady: false,
        rewound: false,
      ),
      driving: true,
    );
    expect(p.headline, l10n.walletSyncScanningEarly);
    expect(p.progress, isNull, reason: 'null ⇒ indeterminate animation');
    expect(p.details, [l10n.walletSyncCatchingUp]);
  });

  test('scanning: a tiny remaining span suppresses the blocks-left chip', () {
    final p = syncStatusPresentation(
      l10n,
      const SyncStatus.scanning(
        from: 99,
        to: 100,
        percent: 0.5,
        spendableReady: false,
        rewound: false,
      ),
      driving: true,
    );
    expect(p.trailing, isNull);
  });

  test('every stall reason maps to its typed copy', () {
    expect(
      stallReasonText(l10n, StallReason.endpointUnreachable),
      l10n.walletStallEndpoint,
    );
    expect(
      stallReasonText(l10n, StallReason.torUnavailable),
      l10n.walletStallTor,
    );
    expect(
      stallReasonText(l10n, StallReason.storageFull),
      l10n.walletStallStorage,
    );
    expect(
      stallReasonText(l10n, StallReason.chainReorg),
      l10n.walletStallReorg,
    );
    expect(
      stallReasonText(l10n, StallReason.internal),
      l10n.walletStallInternal,
    );
    expect(
      stallReasonText(l10n, StallReason.endpointMisbehaving),
      l10n.walletStallEndpointMisbehaving,
    );
    expect(
      stallReasonText(l10n, StallReason.birthdayInFuture),
      l10n.walletStallBirthdayInFuture,
    );
    expect(
      stallReasonText(l10n, StallReason.storageUnavailable),
      l10n.walletStallStorageUnavailable,
    );
    expect(stallReasonText(l10n, StallReason.unknown), l10n.walletStallUnknown);
  });

  // R10 §4.1: a transient local fault (a busy store, an IO error) is not a
  // corrupt store, so its copy says "paused, retrying" and never the restore
  // remedy `walletStallInternal` carries — in any locale.
  group('R10: storageUnavailable never names the restore remedy', () {
    /// The last sentence of the Internal copy is its remedy ("restore from
    /// your recovery phrase") in every shipped locale.
    String remedyOf(String internal) => internal
        .split(RegExp(r'[.。]'))
        .map((p) => p.trim())
        .where((p) => p.isNotEmpty)
        .last;

    test('en copy is the contracted sentence', () {
      expect(
        stallReasonText(l10n, StallReason.storageUnavailable),
        'Sync paused on this device. Retrying.',
      );
      final s = l10n.walletStallStorageUnavailable.toLowerCase();
      expect(s.contains('restore'), isFalse, reason: s);
      expect(s.contains('recovery phrase'), isFalse, reason: s);
      expect(s.contains('server'), isFalse, reason: s);
    });

    test('in every locale it differs from walletStallInternal and does not '
        'carry its restore sentence', () {
      expect(WalletLocalizations.supportedLocales, hasLength(16));
      for (final locale in WalletLocalizations.supportedLocales) {
        final loc = lookupWalletLocalizations(locale);
        final paused = stallReasonText(loc, StallReason.storageUnavailable);
        final internal = loc.walletStallInternal;
        expect(paused, isNotEmpty, reason: '$locale');
        expect(paused, isNot(internal), reason: '$locale');
        final remedy = remedyOf(internal);
        expect(
          paused.contains(remedy),
          isFalse,
          reason: '$locale: "$paused" carries the restore remedy "$remedy"',
        );
      }
    });

    test('the remedy extractor bites on the Internal copy itself', () {
      // Anti-vacuity: the extracted remedy is the restore clause, not an
      // empty or whole-string match.
      final internal = l10n.walletStallInternal;
      final remedy = remedyOf(internal);
      expect(remedy.toLowerCase(), contains('restore'));
      expect(remedy.length, lessThan(internal.length));
    });
  });

  test('#399: the connectivity stall wears the offline glyph + the calm '
      'sheet explanation; hard stalls keep the alarm pair', () {
    // The demoted arm: caution level (wallet_health) ⇒ cloud_off glyph — an
    // error glyph beside an orange tint would be the contradictory pair the
    // rule forbids. Headline stays the shared "Sync paused".
    final offlineish = syncStatusPresentation(
      l10n,
      const SyncStatus.stalled(reason: StallReason.endpointUnreachable),
      driving: true,
    );
    expect(offlineish.icon, WalletGlyph.offline);
    expect(offlineish.headline, l10n.walletSyncStalled);
    expect(offlineish.details, [l10n.walletStallEndpoint]);
    expect(
      syncStatusExplanation(
        l10n,
        const SyncStatus.stalled(reason: StallReason.endpointUnreachable),
        driving: true,
      ),
      l10n.walletSyncExplainStalledOffline,
    );

    // R10: a busy or I/O-faulted store is retried by the loop — the syncing
    // glyph, never the error alarm (that is `internal`'s) and never the
    // offline cloud (the link is fine).
    expect(
      syncStatusPresentation(
        l10n,
        const SyncStatus.stalled(reason: StallReason.storageUnavailable),
        driving: true,
      ).icon,
      WalletGlyph.syncing,
    );

    // A hard stall keeps the error glyph and the generic explanation.
    final hard = syncStatusPresentation(
      l10n,
      const SyncStatus.stalled(reason: StallReason.storageFull),
      driving: true,
    );
    expect(hard.icon, WalletGlyph.error);
    expect(
      syncStatusExplanation(
        l10n,
        const SyncStatus.stalled(reason: StallReason.storageFull),
        driving: true,
      ),
      l10n.walletSyncExplainStalled,
    );
  });

  test('remaining arms: headline + sheet explanation stay paired', () {
    expect(
      syncStatusPresentation(
        l10n,
        const SyncStatus.upToDate(tip: 1),
        driving: true,
      ).headline,
      l10n.walletSyncUpToDate,
    );
    expect(
      syncStatusExplanation(
        l10n,
        const SyncStatus.upToDate(tip: 1),
        driving: true,
      ),
      l10n.walletSyncExplainUpToDate,
    );

    final offline = syncStatusPresentation(
      l10n,
      const SyncStatus.offline(),
      driving: true,
    );
    expect(offline.headline, l10n.walletSyncOffline);
    expect(offline.details, [l10n.walletSyncOfflineDetail]);
    expect(
      syncStatusExplanation(l10n, const SyncStatus.offline(), driving: true),
      l10n.walletSyncExplainOffline,
    );

    // Forward-compat arm: neutral syncing, never healthy or alarming.
    expect(
      syncStatusPresentation(
        l10n,
        const SyncStatus.unknown(),
        driving: true,
      ).headline,
      l10n.walletSyncUnknown,
    );
    expect(
      syncStatusExplanation(l10n, const SyncStatus.unknown(), driving: true),
      l10n.walletSyncExplainUnknown,
    );
  });

  test('transportPresentation (S151): the HOST claim WINS and only a verified '
      'protection earns the protected tone', () {
    final l10n = WalletLocalizationsEn();

    // Host claim present: it wins over ANY SDK TorState (the SDK cannot see a
    // host tunnel — its honest "off" would misreport a protected user).
    final vless = transportPresentation(
      l10n,
      tor: const TorState.off(),
      host: const WalletHostTransport(label: 'VLESS', protection: true),
    );
    expect(vless.label, 'VLESS');
    expect(vless.tone, TransportTone.protected);
    // No host detail -> the generic protective-proxy explanation.
    expect(vless.detail, l10n.walletTransportExplainHostProxy);

    // A host transport WITHOUT a protection claim: neutral + direct honesty.
    final lb = transportPresentation(
      l10n,
      tor: const TorState.active(runtime: TorRuntimeKind.externalSocks5()),
      host: const WalletHostTransport(label: 'Proxy', protection: false),
    );
    expect(lb.tone, TransportTone.neutral);
    expect(lb.detail, l10n.walletTransportExplainDirect);

    // A host-localized detail rides through verbatim.
    final custom = transportPresentation(
      l10n,
      tor: const TorState.off(),
      host: const WalletHostTransport(
        label: 'VPN',
        protection: true,
        detail: 'Custom host copy.',
      ),
    );
    expect(custom.detail, 'Custom host copy.');
  });

  test('transportPresentation (S151): the SDK TorState arms map with the '
      'privacy rule (unverifiable is NEVER protected)', () {
    final l10n = WalletLocalizationsEn();
    TransportPresentation p(TorState tor) =>
        transportPresentation(l10n, tor: tor);

    expect(p(const TorState.off()).label, l10n.walletTorOff);
    expect(p(const TorState.off()).tone, TransportTone.neutral);
    expect(p(const TorState.off()).detail, l10n.walletTransportExplainDirect);

    // The ONE runtime that may still say "Tor": `ExternalSocks5` IS "host-side
    // Tor speaking SOCKS5" by its config definition, so the host named Tor by
    // choosing it and the SDK only repeats the declaration. (The SDK-owned
    // `builtIn` is gone — FR-5 C1, ADR-0548 D3; a plugin's Tor arrives as a
    // host dialer. This arm is refused at the config door today, T0-6.)
    expect(
      p(const TorState.active(runtime: TorRuntimeKind.externalSocks5())).tone,
      TransportTone.protected,
    );
    expect(
      p(const TorState.active(runtime: TorRuntimeKind.externalSocks5())).detail,
      l10n.walletTransportExplainTor,
    );

    // The privacy rule: an unattributable runtime / unknown state / fallback
    // NEVER reads protected.
    expect(
      p(const TorState.active(runtime: TorRuntimeKind.unknown())).tone,
      TransportTone.caution,
    );
    expect(p(const TorState.fellBack()).tone, TransportTone.caution);
    expect(
      p(const TorState.fellBack()).detail,
      l10n.walletTransportExplainFellBack,
    );
    expect(p(const TorState.unavailable()).tone, TransportTone.danger);
    expect(p(const TorState.unknown()).tone, TransportTone.caution);

    expect(
      p(const TorState.unknown()).detail,
      l10n.walletTransportExplainUnverified,
    );
    expect(
      p(const TorState.bootstrapping(percent: 10)).tone,
      TransportTone.progress,
    );
  });

  // FR-29 spec §3.4 / §8 T19 (ADR-0547 — the host NAMES its transport): the
  // registered host dialer renders WHAT THE HOST DECLARED and nothing more —
  // the host's own name verbatim, the linkability when isolation is
  // unsupported OR unknown, "not private" for an exposed path whatever the
  // isolation, caution for an unknown exposure whatever the isolation — and
  // only a hidden, isolating path earns the protected tone.
  test('transportPresentation (FR-29): a host dialer renders its declared '
      'name, isolation and exposure honestly', () {
    TransportPresentation p(
      String name,
      IsolationSupport iso,
      TransportExposure ex,
    ) => transportPresentation(
      l10n,
      tor: TorState.active(
        runtime: TorRuntimeKind.hostDialer(
          name: name,
          isolation: iso,
          exposure: ex,
        ),
      ),
    );

    final tor = p('Tor', IsolationSupport.supported, TransportExposure.hidden);
    expect(tor.label, l10n.walletTorHostPath('Tor'));
    expect(tor.label, contains('Tor'));
    expect(tor.tone, TransportTone.protected);
    expect(tor.detail, l10n.walletTransportExplainHostProxy);

    // The name is the HOST'S, verbatim — a name the SDK has no list for.
    final custom = p(
      'VLESS via Cloudflare',
      IsolationSupport.supported,
      TransportExposure.hidden,
    );
    expect(custom.label, l10n.walletTorHostPath('VLESS via Cloudflare'));
    expect(custom.label, contains('VLESS via Cloudflare'));
    expect(custom.tone, TransportTone.protected);

    // A non-isolating path says so, in caution — never the protected green.
    final ss = p(
      'Shadowsocks',
      IsolationSupport.unsupported,
      TransportExposure.hidden,
    );
    expect(ss.label, l10n.walletTorHostPathLinkable('Shadowsocks'));
    expect(ss.label, isNot(l10n.walletTorHostPath('Shadowsocks')));
    expect(ss.tone, TransportTone.caution);

    // Unknown isolation is rendered as linkable too (never a promise).
    final vless = p(
      'VLESS',
      IsolationSupport.unknown,
      TransportExposure.hidden,
    );
    expect(vless.label, l10n.walletTorHostPathLinkable('VLESS'));
    expect(vless.tone, TransportTone.caution);

    // An EMPTY name is the SDK's unattributed rendering (never a host's):
    // "a private path", unnamed.
    final unnamed = p('', IsolationSupport.supported, TransportExposure.hidden);
    expect(
      unnamed.label,
      l10n.walletTorHostPath(l10n.walletTorHostOtherTransport),
    );
    expect(unnamed.tone, TransportTone.protected);

    // An UNKNOWN exposure (not declared, or a value this binding cannot
    // read) is unattributable: the CAUTION tone whatever isolation says —
    // never the confident green (the wave review's MEDIUM, now on exposure).
    final unknown = p(
      'Proxy',
      IsolationSupport.supported,
      TransportExposure.unknown,
    );
    expect(unknown.label, l10n.walletTorHostPathLinkable('Proxy'));
    expect(unknown.tone, TransportTone.caution);
    expect(unknown.detail, l10n.walletTransportExplainUnverified);

    // An EXPOSED path is not private, whatever isolation says and whatever
    // the host called it.
    final exposed = p(
      'Direct',
      IsolationSupport.supported,
      TransportExposure.exposed,
    );
    expect(exposed.label, l10n.walletTorHostDirect);
    expect(exposed.detail, l10n.walletTransportExplainDirect);
    // FR-30 (c), C1: CAUTION, not neutral — and the ORDERING is the finding,
    // so it is the ordering that is pinned. An exposed path shows the server
    // this device's address; a hidden-but-linkable one only lets the proxy
    // link the wallet's own connections. The exposed arm must never read
    // CALMER than the linkable one, and only hidden + isolated is protected.
    expect(exposed.tone, TransportTone.caution);
    expect(
      exposed.tone,
      ss.tone,
      reason: 'a privacy loss is never calmer than a linkability warning',
    );
    expect(tor.tone, TransportTone.protected);
    expect(
      {exposed.tone, ss.tone, unknown.tone},
      isNot(contains(TransportTone.protected)),
      reason: 'only a hidden, isolating path earns the protected tone',
    );
  });

  // FR-30 (a), C1: the FAILING arms carry the registrant's own name — or no
  // transport noun at all. Before C1 they hard-coded "Tor", so a host that
  // registered Shadowsocks read "Tor starting…" for a path that has no Tor in
  // it, and a FAILED Shadowsocks path read "Tor STARTING" (FR-30 (b)).
  test('transportPresentation (FR-30 (a)): the failing arms name the '
      'registered transport, or name none', () {
    TransportPresentation p(TorState tor) =>
        transportPresentation(l10n, tor: tor);

    // Named: the host's own noun, verbatim, on both failing arms.
    final booting = p(
      const TorState.bootstrapping(percent: 0.4, transport: 'Shadowsocks'),
    );
    expect(booting.label, l10n.walletTorBootstrappingNamed('Shadowsocks'));
    expect(booting.label, contains('Shadowsocks'));
    expect(booting.tone, TransportTone.progress);
    expect(
      booting.detail,
      l10n.walletTransportExplainBootstrappingNamed('Shadowsocks'),
    );

    final down = p(const TorState.unavailable(transport: 'Shadowsocks'));
    expect(down.label, l10n.walletTorUnavailableNamed('Shadowsocks'));
    expect(down.label, contains('Shadowsocks'));
    expect(down.tone, TransportTone.danger);
    expect(
      down.detail,
      l10n.walletTransportExplainUnavailableNamed('Shadowsocks'),
    );

    // Unnamed (nothing registered, or a runtime with no registry) and the
    // empty name (the SDK's own unattributed rendering) both read the
    // transport-NEUTRAL sentence — never an empty noun, never "Tor".
    for (final unnamed in [
      p(const TorState.bootstrapping(percent: 0.4)),
      p(const TorState.bootstrapping(percent: 0.4, transport: '')),
    ]) {
      expect(unnamed.label, l10n.walletTorBootstrapping);
      expect(unnamed.detail, l10n.walletTransportExplainBootstrapping);
    }
    for (final unnamed in [
      p(const TorState.unavailable()),
      p(const TorState.unavailable(transport: '')),
    ]) {
      expect(unnamed.label, l10n.walletTorUnavailable);
      expect(unnamed.detail, l10n.walletTransportExplainUnavailable);
    }

    // THE defect FR-30 (a) filed: no failing arm a non-Tor host can reach
    // says "Tor", on the label or in the explanation.
    for (final p2 in [
      p(const TorState.bootstrapping(percent: 0.4, transport: 'Shadowsocks')),
      p(const TorState.unavailable(transport: 'Shadowsocks')),
      p(const TorState.bootstrapping(percent: 0.4)),
      p(const TorState.unavailable()),
    ]) {
      expect(p2.label, isNot(contains('Tor')));
      expect(p2.detail, isNot(contains('Tor')));
    }
    // ...and the stall sentence, which is rendered from a StallReason and so
    // can carry no name, says none either.
    expect(
      stallReasonText(l10n, StallReason.torUnavailable),
      isNot(contains('Tor')),
    );
    // A host that DID register Tor still reads its own noun, verbatim.
    expect(
      p(const TorState.unavailable(transport: 'Tor')).label,
      contains('Tor'),
    );
  });

  // FR-32 (a), stage S1 `copy`: `TorRuntimeKind.dialer` is an arbitrary
  // byte-stream dialer a Rust host injected, declaring no name, no isolation
  // and no exposure — `net/dialer.rs` says the SDK "never knows or names" it.
  // It read "Tor active" in the PROTECTED tone until this item, asserting
  // onion routing in the confident colour for a path the SDK cannot attest
  // (ADR-0547 — the SDK has no predefined transport kinds; the §2.3 trust
  // boundary the host-dialer arm was built to respect).
  test('transportPresentation (FR-32 (a)): an injected Dialer does not claim '
      'Tor, and does not claim protection', () {
    final p = transportPresentation(
      l10n,
      tor: const TorState.active(runtime: TorRuntimeKind.dialer()),
    );
    expect(p.label, l10n.walletTorActiveUnattested);
    expect(p.detail, l10n.walletTransportExplainUnverified);
    expect(p.label, isNot(contains('Tor')));
    expect(p.detail, isNot(contains('Tor')));
    expect(
      p.tone,
      isNot(TransportTone.protected),
      reason:
          'the confident tone for a path the SDK cannot attest is the '
          'defect FR-32 (a) filed',
    );
    expect(p.tone, TransportTone.caution);

    // The escape hatch FR-32 (a) names — "the SDK asks the host to say so" —
    // already exists and still wins: a Rust host that KNOWS it injected Tor
    // declares it, and the protected tone comes back.
    final declared = transportPresentation(
      l10n,
      tor: const TorState.active(runtime: TorRuntimeKind.dialer()),
      host: const WalletHostTransport(label: 'Tor', protection: true),
    );
    expect(declared.label, 'Tor');
    expect(declared.tone, TransportTone.protected);
  });

  // Stage S1 `truth` + `copy` (FR-36): the either/or state. The chip states
  // the two attested facts and the sheet carries the either/or with a next
  // step per cause; the name is the host's own, from the same payload
  // `Active` carries (ADR-0547).
  test('transportPresentation (FR-36): the unanswered state names the '
      'transport the host declared, blames neither side, and is not the down '
      'state', () {
    TransportPresentation p(TorRuntimeKind runtime) =>
        transportPresentation(l10n, tor: TorState.unanswered(runtime: runtime));

    const shadowsocks = TorRuntimeKind.hostDialer(
      name: 'Shadowsocks',
      isolation: IsolationSupport.supported,
      exposure: TransportExposure.hidden,
    );

    // NAMED: the host's own noun, verbatim, on both halves.
    final named = p(shadowsocks);
    expect(named.label, l10n.walletTorUnansweredNamed('Shadowsocks'));
    expect(named.label, contains('Shadowsocks'));
    expect(
      named.detail,
      l10n.walletTransportExplainUnansweredNamed('Shadowsocks'),
    );
    expect(named.detail, contains('Shadowsocks'));

    // UNNAMED, and the split inside it. An ATTESTED path whose name is the
    // SDK's own unattributed empty string reads the neutral pair — the host
    // declared hidden + isolating, so there is nothing to hedge. The two
    // runtimes that declared NOTHING do not: they keep the refusal `Active`
    // makes for them. Never an empty noun, never "Tor", in either case.
    for (final unnamed in [
      p(const TorRuntimeKind.dialer()),
      p(const TorRuntimeKind.unknown()),
      p(
        const TorRuntimeKind.hostDialer(
          name: '',
          isolation: IsolationSupport.supported,
          exposure: TransportExposure.hidden,
        ),
      ),
    ]) {
      expect(unnamed.label, isNot(contains('Tor')));
      expect(unnamed.detail, isNot(contains('Tor')));
    }
    final attestedEmptyName = p(
      const TorRuntimeKind.hostDialer(
        name: '',
        isolation: IsolationSupport.supported,
        exposure: TransportExposure.hidden,
      ),
    );
    expect(attestedEmptyName.label, l10n.walletTorUnanswered);
    expect(attestedEmptyName.detail, l10n.walletTransportExplainUnanswered);
    for (final unattested in [
      p(const TorRuntimeKind.dialer()),
      p(const TorRuntimeKind.unknown()),
    ]) {
      expect(unattested.label, l10n.walletTorUnansweredUnattested);
      expect(
        unattested.detail,
        l10n.walletTransportExplainUnansweredUnverified,
      );
    }

    // THE PARITY RULE, in the one form that is checkable (crypto audit
    // HIGH). A general "unanswered is never weaker than active" is not a
    // computable predicate and would flag the hidden+unsupported arm, where the
    // linkability caveat deliberately rides the LABEL. The narrow rule holds:
    // wherever ACTIVE refuses to vouch for a path with
    // `walletTransportExplainUnverified`, UNANSWERED must refuse too. Going
    // quiet is not something the wallet learns privacy from.
    const refusal = 'treat it as not private';
    expect(l10n.walletTransportExplainUnverified, contains(refusal));
    for (final runtime in const [
      TorRuntimeKind.dialer(),
      TorRuntimeKind.unknown(),
    ]) {
      final active = transportPresentation(
        l10n,
        tor: TorState.active(runtime: runtime),
      );
      expect(
        active.detail,
        contains(refusal),
        reason: 'fixture check: active must be the arm that refuses',
      );
      expect(
        p(runtime).detail,
        contains(refusal),
        reason:
            'unanswered dropped the refusal active makes for $runtime — the '
            'claim got STRONGER as the wallet learned less',
      );
    }

    // The chip and the sheet are DIFFERENT sentences now: the chip is a label,
    // the sheet's detail is the one that has to carry the either/or and the
    // next steps. Until this item both surfaces rendered the same string.
    expect(named.label, isNot(named.detail));
    expect(
      p(const TorRuntimeKind.dialer()).label,
      isNot(p(const TorRuntimeKind.dialer()).detail),
    );

    // The either/or IS the claim: the sheet names both candidate causes and
    // the wallet's own inability to separate them, plus one next step each.
    final detail = p(const TorRuntimeKind.dialer()).detail;
    expect(detail, contains('the wallet server'));
    expect(detail, contains("can't tell"));
    expect(detail, contains('another server'));
    expect(detail, contains('network settings'));

    // It is NOT the genuinely-down state: `Unavailable` says "not connected"
    // and is `danger`; this one says the path took the connection and is
    // caution. Since stage S1 an accepted-then-silent path reaches HERE, and
    // rendering it as down would blame the path for what the evidence cannot
    // separate from a wedged server.
    expect(named.tone, TransportTone.caution);
    expect(
      named.label,
      isNot(l10n.walletTorUnavailableNamed('Shadowsocks')),
      reason: 'the down chip and the unanswered chip are different claims',
    );
    expect(
      transportPresentation(
        l10n,
        tor: const TorState.unavailable(transport: 'Shadowsocks'),
      ).tone,
      TransportTone.danger,
    );
  });

  // FR-44 (raised by the host, confirmed at the code the same hour).
  // `Unanswered` carries `Active`'s payload — name, isolation AND exposure —
  // so it must carry `Active`'s honesty about it. It switched on the RUNTIME
  // alone (named vs unnamed) and never read `exposure`, so a path the host
  // declared EXPOSED (the server sees the device's address) read the same
  // sentence as a hidden one, and the privacy loss the `Active` chip had been
  // disclosing vanished the moment the path went quiet. The host grades the
  // same state per-exposure on its own Network tab, so one phone showed two
  // privacy verdicts about one connection in two tabs. Both families now
  // decide "is this path private?" in ONE place — `hostDialerPresentation`'s
  // exposure switch — which is what kept them from disagreeing on `Active`
  // and is the whole reason this arm could disagree at all.
  test('transportPresentation (FR-44): the unanswered state reads the '
      'declared exposure — an exposed path is not private here either', () {
    TransportPresentation p(
      String name,
      IsolationSupport iso,
      TransportExposure ex,
    ) => transportPresentation(
      l10n,
      tor: TorState.unanswered(
        runtime: TorRuntimeKind.hostDialer(
          name: name,
          isolation: iso,
          exposure: ex,
        ),
      ),
    );

    // The matcher, pinned against the sentences it has to find and the one it
    // must not (the FR-32 (b) discipline — a blind matcher is how a negative
    // row passes while asserting nothing). "Private path" is the CLAIM; "not
    // private" is its opposite, not an instance of it.
    bool claimsPrivatePath(String s) =>
        s.toLowerCase().contains('private path');
    expect(claimsPrivatePath(l10n.walletTorUnanswered), isTrue);
    expect(claimsPrivatePath(l10n.walletTransportExplainUnanswered), isTrue);
    expect(claimsPrivatePath(l10n.walletTorUnansweredDirect), isFalse);

    // EXPOSED and named — the shape the core actually emits: `live_tor_state`
    // returns `Unavailable` before this state when nothing is registered
    // (tor_status.rs:118-134), so every `Unanswered` host-dialer payload it
    // builds carries the descriptor's own name AND its exposure (:171-175).
    final exposed = p(
      'Shadowsocks',
      IsolationSupport.supported,
      TransportExposure.exposed,
    );
    expect(exposed.label, l10n.walletTorUnansweredDirect);
    expect(exposed.detail, l10n.walletTransportExplainUnansweredDirect);
    expect(claimsPrivatePath(exposed.label), isFalse);
    expect(claimsPrivatePath(exposed.detail), isFalse);
    expect(exposed.tone, isNot(TransportTone.protected));
    // THE defect, verbatim: the sentences this arm rendered before FR-44 —
    // the hidden family's, for a path the host declared exposed.
    expect(exposed.label, isNot(l10n.walletTorUnansweredNamed('Shadowsocks')));
    expect(
      exposed.detail,
      isNot(l10n.walletTransportExplainUnansweredNamed('Shadowsocks')),
    );
    // The disclosure the `Active` chip makes about this very payload survives
    // the state change, in the same words — the two arms of one payload can
    // no longer say different things about its privacy.
    expect(exposed.label, contains(l10n.walletTorHostDirect));
    // ...and the state's MEANING is untouched: the either/or and one next
    // step per cause, which is what `Unanswered` exists to say.
    expect(exposed.detail, contains('the wallet server'));
    expect(exposed.detail, contains("can't tell"));
    expect(exposed.detail, contains('another server'));
    expect(exposed.detail, contains('network settings'));

    // The rendering FR-44 quoted — "Private path connected — nothing coming
    // back" over an exposed path — needs the SDK's own UNATTRIBUTED (empty)
    // name to reach, which the core never pairs with a declared exposure
    // (see above). Pinned anyway: the binding is total over the payloads the
    // bridge type admits, and this is the one that says it in those words.
    final unattributed = p(
      '',
      IsolationSupport.supported,
      TransportExposure.exposed,
    );
    expect(unattributed.label, l10n.walletTorUnansweredDirect);
    expect(claimsPrivatePath(unattributed.label), isFalse);

    // UNKNOWN exposure: the host declared nothing about it, so the sentence
    // promises nothing — never the benign framing for what the binding
    // cannot attest (§3.3), whatever the isolation says.
    final unknown = p(
      'VLESS',
      IsolationSupport.supported,
      TransportExposure.unknown,
    );
    expect(unknown.label, l10n.walletTorUnansweredLinkable('VLESS'));
    expect(unknown.detail, l10n.walletTransportExplainUnansweredUnverified);
    expect(claimsPrivatePath(unknown.label), isFalse);

    // HIDDEN but not isolating: the linkability `Active` discloses on this
    // payload is disclosed here too — the axis does not go quiet with the
    // path, for the same reason the exposure does not.
    final linkable = p(
      'Shadowsocks',
      IsolationSupport.unsupported,
      TransportExposure.hidden,
    );
    expect(linkable.label, l10n.walletTorUnansweredLinkable('Shadowsocks'));
    expect(
      linkable.detail,
      l10n.walletTransportExplainUnansweredNamed('Shadowsocks'),
    );

    // CONTROL: hidden + isolating still reads the plain named pair. Without
    // it every assertion above is satisfied by an arm that renders one
    // sentence for every payload.
    final hidden = p(
      'Shadowsocks',
      IsolationSupport.supported,
      TransportExposure.hidden,
    );
    expect(hidden.label, l10n.walletTorUnansweredNamed('Shadowsocks'));
    expect(
      hidden.detail,
      l10n.walletTransportExplainUnansweredNamed('Shadowsocks'),
    );

    // Nothing in this family is protected, whatever the exposure: nothing is
    // carrying. The tone therefore CANNOT be what pins FR-44 — the sentences
    // are.
    expect(
      {exposed.tone, unknown.tone, linkable.tone, hidden.tone},
      {TransportTone.caution},
    );
  });

  // FR-32 (b), stage S1 `copy`. A `StallReason` and a `TorState` carry NO
  // policy — the core's derivation is blind to `Preferred` vs `Required` by
  // construction (`live_tor_state_is_blind_to_preferred_vs_required`) — so
  // EVERY sentence they render is read by a `Preferred` wallet as well as a
  // `Required` one. Two claims therefore cannot be made in them:
  //
  //   * "nothing was sent in the clear". Only `Required` fails closed (its
  //     dial plan has no fallback arm at all); a `Preferred` wallet reaching
  //     the same sentence is promised a guarantee it does not have.
  //   * "a private path is required". `Preferred` reaches these arms too — a
  //     registrant that declared its transport FAILED, an unsupported runtime,
  //     a refused private dial — and telling that user a setting they never
  //     chose is in force sends them to look for a switch that is already off.
  //
  // Both were in the shipped copy until this item, on both surfaces.
  group('FR-32 (b): no policy-blind sentence promises the fail-closed '
      'guarantee or names a policy the user may not have set', () {
    // The claim is about the WORDS, so the matchers are substrings — and they
    // are pinned below against the superseded sentences so they can never go
    // vacuous. English only: these are the source strings the other fifteen
    // locales translate, and a per-language phrase list would be a second
    // source of truth for the same predicate.
    bool promisesNothingInTheClear(String s) =>
        s.toLowerCase().contains('in the clear');
    bool namesAPolicy(String s) => s.toLowerCase().contains('required');

    /// Every sentence a wallet can read WITHOUT the surface knowing its
    /// policy: the typed stall copy (rendered from a bare `StallReason`) and
    /// both halves of every transport arm.
    List<String> policyBlindSentences() {
      final out = <String>[
        for (final r in StallReason.values) stallReasonText(l10n, r),
      ];
      for (final tor in <TorState>[
        const TorState.off(),
        const TorState.bootstrapping(percent: 0.4),
        const TorState.bootstrapping(percent: 0.4, transport: 'Shadowsocks'),
        const TorState.active(runtime: TorRuntimeKind.dialer()),
        const TorState.active(runtime: TorRuntimeKind.unknown()),
        const TorState.active(
          runtime: TorRuntimeKind.hostDialer(
            name: 'Shadowsocks',
            isolation: IsolationSupport.supported,
            exposure: TransportExposure.hidden,
          ),
        ),
        const TorState.fellBack(),
        const TorState.unavailable(),
        const TorState.unavailable(transport: 'Shadowsocks'),
        const TorState.unanswered(runtime: TorRuntimeKind.dialer()),
        const TorState.unanswered(runtime: TorRuntimeKind.unknown()),
        const TorState.unanswered(
          runtime: TorRuntimeKind.hostDialer(
            name: 'Shadowsocks',
            isolation: IsolationSupport.supported,
            exposure: TransportExposure.hidden,
          ),
        ),
        const TorState.unknown(),
      ]) {
        final p = transportPresentation(l10n, tor: tor);
        out
          ..add(p.label)
          ..add(p.detail);
      }
      return out;
    }

    // ANTI-VACUITY, and it never expires: the two matchers are pinned against
    // the SUPERSEDED sentences, verbatim, so a matcher that stopped matching
    // anything would red here rather than pass the sweep below in silence.
    test('the matchers bite — the superseded sentences, verbatim', () {
      const supersededStall =
          'A private path is required and unavailable — nothing was sent in '
          'the clear. Sync resumes once it reconnects.';
      const supersededExplain =
          'A private path is required but unavailable, so the wallet '
          "won't connect. Turn the private path off, or check your app's "
          'network settings.';
      expect(promisesNothingInTheClear(supersededStall), isTrue);
      expect(namesAPolicy(supersededStall), isTrue);
      expect(namesAPolicy(supersededExplain), isTrue);
      // ...and they do NOT fire on an innocent sentence.
      expect(promisesNothingInTheClear(supersededExplain), isFalse);
      expect(namesAPolicy(l10n.walletStallStorage), isFalse);
      expect(promisesNothingInTheClear(l10n.walletStallStorage), isFalse);
    });

    test('a `Preferred` wallet is never promised that nothing left in the '
        'clear', () {
      final offenders = policyBlindSentences()
          .where(promisesNothingInTheClear)
          .toList();
      expect(
        offenders,
        isEmpty,
        reason:
            'only a `Required` wallet fails closed, and none of these '
            'sentences knows which wallet is reading it',
      );
    });

    test('a `Preferred` wallet is never told the private path is required', () {
      final offenders = policyBlindSentences().where(namesAPolicy).toList();
      expect(
        offenders,
        isEmpty,
        reason:
            'the state carries no policy, so the sentence cannot assert one',
      );
    });
  });

  // #403 R6 + R10 — the sync-paused qualifier.
  //
  // #401 R1b added the qualifier to four saved-for-retry money bodies and nothing
  // asserted it appears at all. It also joined with a Dart `'$body $note'`, which
  // puts a U+0020 after a fullwidth 。 in ja and zh — the exact pattern had
  // already adjudicated and abandoned for CJK typography.
  group('#403 R6 — walletSyncPausedQualified', () {
    test('passes the body through UNTOUCHED while passes run', () {
      final body = l10n.walletSendSavedBody;
      expect(
        walletSyncPausedQualified(l10n, body, syncPassesRun: true),
        body,
        reason: 'a healthy wallet keeps its promise unqualified',
      );
    });

    test('appends the note when no pass will run', () {
      final out = walletSyncPausedQualified(
        l10n,
        l10n.walletSendSavedBody,
        syncPassesRun: false,
      );
      expect(out, startsWith(l10n.walletSendSavedBody));
      expect(
        out,
        endsWith(l10n.walletSyncPausedMoneyNote),
        reason:
            'the whole point of the helper — an unqualified promise here is a '
            'signed payment nothing will ever re-send',
      );
      expect(
        out,
        '${l10n.walletSendSavedBody} ${l10n.walletSyncPausedMoneyNote}',
      );
    });

    test('ja joins with NO space — the separator is the locale\'s', () {
      final ja = WalletLocalizationsJa();
      final out = walletSyncPausedQualified(
        ja,
        ja.walletSendSavedBody,
        syncPassesRun: false,
      );
      expect(out, '${ja.walletSendSavedBody}${ja.walletSyncPausedMoneyNote}');
      expect(
        out.contains('。 '),
        isFalse,
        reason:
            'a space after a fullwidth full stop is wrong typography — this is '
            'the S205-c adjudication, applied through an ARB joiner key',
      );
    });
  });

  // ---------------------------------------------------------------- T-3 ----
  //
  // UI-1 row T-3 (`docs/plan/production-readiness-phase-1.md` §4r U-3): the
  // per-pool report crosses the bridge and NO Dart reads it (§4j row 8, §4m
  // #10). `SyncStatus.upToDateDegraded` exists precisely because ONE pool is
  // unserved — the balance beside it is a floor for that pool — and the whole
  // presentation says only "this server isn't serving every pool", never
  // WHICH, so the user cannot tell whether the money they are missing is in
  // it, and "switch servers" has nothing to be checked against.
  //
  // Asserted by the pool NAME appearing in `details`: the l10n keys U-3 mints
  // (`walletSyncPool<State>`, the pool as a placeholder) do not exist at the
  // contract commit and this half is blind to them. Case-insensitive — the
  // property is that the line NAMES the pool, not how it capitalises it.
  group('T-3 the degraded pools are named (§4r U-3)', () {
    // One refused pool; the other two served, one of them at the honest zero
    // (a pool before its first 2^16 notes — `PoolService.served` doc: "not,
    // by itself, a problem"). The boundary case for the mirrored predicate.
    const oneRefused = PoolServiceReport(
      sapling: PoolService.unsupported(),
      orchard: PoolService.served(roots: 7),
      ironwood: PoolService.served(roots: 0),
    );
    // Every state `report_is_degraded` calls degraded, one per pool
    // (`sync_controller.rs:661-668`: Withheld | Unsupported | HeightViolation).
    const everyDegradedState = PoolServiceReport(
      sapling: PoolService.withheld(proven: 12),
      orchard: PoolService.unsupported(),
      ironwood: PoolService.heightViolation(),
    );
    const allServed = PoolServiceReport(
      sapling: PoolService.served(roots: 7),
      orchard: PoolService.served(roots: 3),
      ironwood: PoolService.served(roots: 0),
    );
    const pools = ['sapling', 'orchard', 'ironwood'];

    List<String> linesNaming(List<String> details, String pool) => details
        .where((d) => d.toLowerCase().contains(pool.toLowerCase()))
        .toList();

    // ANTI-VACUITY for the matcher every row below leans on. At the contract
    // commit `details` is EMPTY on both arms, so `linesNaming` is never once
    // exercised against a line that DOES name a pool — a broken matcher would
    // red the defect rows for the wrong reason and leave the fully-served
    // CONTROL passing for ever, including after the fix.
    test('the pool matcher itself: it finds a naming line and only a naming '
        'line', () {
      const lines = [
        'Sapling: this server refuses to serve it',
        'Orchard: served',
      ];
      expect(linesNaming(lines, 'sapling'), hasLength(1));
      expect(linesNaming(lines, 'ironwood'), isEmpty);
      expect(
        linesNaming(const ['SAPLING is unsupported here'], 'sapling'),
        hasLength(1),
        reason: 'the property is that the line NAMES the pool, not its case',
      );
    });

    test('UpToDateDegraded: the refused pool is NAMED, and the served ones '
        'are not', () {
      const status = SyncStatus.upToDateDegraded(tip: 100, pools: oneRefused);
      final p = syncStatusPresentation(l10n, status, driving: true);
      expect(
        p.headline,
        l10n.walletSyncUpToDateDegraded,
        reason: 'precondition: the degraded arm',
      );

      expect(
        linesNaming(p.details, 'sapling'),
        hasLength(1),
        reason:
            'one line per AFFECTED pool, naming the pool and the state (§4r '
            'U-3) — "this server isn\'t serving every pool" without WHICH is '
            'the §4j row 8 / §4m #10 gap; details were ${p.details}',
      );
      expect(
        linesNaming(p.details, 'orchard'),
        isEmpty,
        reason: 'a pool served normally has nothing to report',
      );
      expect(
        linesNaming(p.details, 'ironwood'),
        isEmpty,
        reason:
            'served at ZERO completed subtrees is information, never a fault '
            '(the PoolService.served boundary)',
      );
    });

    test('UpToDateDegraded: each degraded state gets its own line — withheld, '
        'unsupported and a height violation are three different facts', () {
      const status = SyncStatus.upToDateDegraded(
        tip: 100,
        pools: everyDegradedState,
      );
      final p = syncStatusPresentation(l10n, status, driving: true);

      final named = <String>[];
      for (final pool in pools) {
        final lines = linesNaming(p.details, pool);
        expect(
          lines,
          hasLength(1),
          reason: '$pool is degraded and must be named; got ${p.details}',
        );
        named.addAll(lines);
      }
      expect(
        named.toSet(),
        hasLength(3),
        reason:
            'one key per state (§4r U-3) — three pools sharing one sentence '
            'would hide WHICH failure this server has, and the three have '
            'different consequences; got $named',
      );
    });

    test('EndpointBehind carries the same lines — a behind server that also '
        'withholds a pool says both (§4m #10)', () {
      const status = SyncStatus.endpointBehind(
        tip: 100,
        newestKnown: 900,
        pools: oneRefused,
      );
      final p = syncStatusPresentation(l10n, status, driving: true);
      expect(
        p.headline,
        l10n.walletSyncEndpointBehind,
        reason: 'precondition: the behind arm',
      );
      expect(
        syncStatusExplanation(l10n, status, driving: true),
        l10n.walletSyncExplainEndpointBehind,
        reason: 'precondition: the behind explain still owns the slot',
      );

      expect(
        linesNaming(p.details, 'sapling'),
        hasLength(1),
        reason:
            'the degraded-money fact must not be lost behind the behind claim '
            '— U-3\'s suffix; details were ${p.details}',
      );
    });

    test('CONTROL: a fully-served report renders NO pool line — the mirrored '
        'predicate is `report_is_degraded`, not its inverse', () {
      const status = SyncStatus.upToDateDegraded(tip: 100, pools: allServed);
      final p = syncStatusPresentation(l10n, status, driving: true);
      expect(
        p.headline,
        l10n.walletSyncUpToDateDegraded,
        reason: 'precondition: the degraded arm',
      );

      for (final pool in pools) {
        expect(
          linesNaming(p.details, pool),
          isEmpty,
          reason:
              'every pool is served — naming one here would be a false claim '
              'about the user\'s money (an INVERTED predicate lands exactly '
              'here); details were ${p.details}',
        );
      }
    });
  });
}
