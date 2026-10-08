import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_health.dart';
import 'package:zec_wallet/zec_wallet.dart';

/// Gate-8 truth table for [walletBadgeLevel] (ADR-0533) — the single new
/// load-bearing rule behind the persistent green/yellow/red status badge.
/// Every [SyncStatus] arm × the stale flag is pinned so a future refactor that
/// drops an arm or flips a color fails CI loudly.
void main() {
  group('walletBadgeLevel', () {
    test(
      'unverified (server silent about its network) → YELLOW, running or ended',
      () {
        // GRACE-1 (§4p): never green — sending is on a countdown while the
        // grace runs and refused once it has ended; never red — the balance
        // is current and the fix is a server switch, not a repair.
        for (final grace in const [
          UnknownBranchGrace.running(blocksLeft: 1000, secsLeft: 6400),
          UnknownBranchGrace.running(blocksLeft: 1000, secsLeft: null),
          UnknownBranchGrace.ended(
            by: GraceExpiry.clock,
            blocksSinceLastCurrent: 0,
          ),
        ]) {
          for (final stale in [false, true]) {
            expect(
              walletBadgeLevel(
                status: SyncStatus.upToDateUnverified(
                  tip: 100,
                  grace: grace,
                  pools: null,
                  streakReported: false,
                ),
                stale: stale,
              ),
              WalletBadgeLevel.caution,
              reason: '$grace stale=$stale',
            );
          }
        }
      },
    );

    test('up to date and fresh → GREEN', () {
      expect(
        walletBadgeLevel(
          status: const SyncStatus.upToDate(tip: 100),
          stale: false,
        ),
        WalletBadgeLevel.ok,
      );
    });

    test(
      'up to date but the last refresh FAILED → YELLOW (stale, not green)',
      () {
        // Maintainer: "yellow if the balance is stale." A failed cold refresh on a
        // synced wallet must never read as confidently up to date.
        expect(
          walletBadgeLevel(
            status: const SyncStatus.upToDate(tip: 100),
            stale: true,
          ),
          WalletBadgeLevel.caution,
        );
      },
    );

    test('scanning with funds spendable → GREEN (spend-before-sync usable)', () {
      // The wallet is usable and its near-tip balance is fresh; the historical
      // backfill only makes it grow. Green even though % < 100.
      expect(
        walletBadgeLevel(
          status: const SyncStatus.scanning(
            from: 100,
            to: 200,
            percent: 0.3,
            spendableReady: true,
            rewound: false,
          ),
          stale: false,
        ),
        WalletBadgeLevel.ok,
      );
    });

    test('scanning, nothing spendable yet → YELLOW (operations limited)', () {
      expect(
        walletBadgeLevel(
          status: const SyncStatus.scanning(
            from: 100,
            to: 200,
            percent: 0.3,
            spendableReady: false,
            rewound: false,
          ),
          stale: false,
        ),
        WalletBadgeLevel.caution,
      );
    });

    test('a stale refresh pulls even a spendable scan down to YELLOW', () {
      expect(
        walletBadgeLevel(
          status: const SyncStatus.scanning(
            from: 100,
            to: 200,
            percent: 0.9,
            spendableReady: true,
            rewound: false,
          ),
          stale: true,
        ),
        WalletBadgeLevel.caution,
      );
    });

    test('connecting / idle / offline → YELLOW (usable with a caveat)', () {
      for (final status in const [
        SyncStatus.connecting(),
        SyncStatus.connecting(torBootstrapPercent: 0.4),
        SyncStatus.idle(),
        SyncStatus.offline(),
      ]) {
        expect(
          walletBadgeLevel(status: status, stale: false),
          WalletBadgeLevel.caution,
          reason: '$status should be caution',
        );
      }
    });

    test('a chain reorg is transient and self-heals → YELLOW, not RED', () {
      expect(
        walletBadgeLevel(
          status: const SyncStatus.stalled(reason: StallReason.chainReorg),
          stale: false,
        ),
        WalletBadgeLevel.caution,
      );
    });

    test('#399: an unreachable endpoint is the normal-offline family → '
        'YELLOW, not RED', () {
      // The core can only prove "the dial failed" — airplane mode (routine,
      // deliberate) and a down server both land here, and the spec's §3.2a
      // row calls this arm "(normal offline)". It is also the lowest-claim
      // fallback for unmapped transients, and caution IS the lowest-claim
      // tone. RED here rendered every flight as a hard error.
      expect(
        walletBadgeLevel(
          status: const SyncStatus.stalled(
            reason: StallReason.endpointUnreachable,
          ),
          stale: false,
        ),
        WalletBadgeLevel.caution,
      );
    });

    test('R10: a busy or I/O-faulted store is transient → YELLOW, not RED', () {
      // The store is intact and the loop retries by itself (and only shows it
      // once it persists for two passes); RED would read as the corrupt-store
      // alarm this reason exists to keep apart from.
      expect(
        walletBadgeLevel(
          status: const SyncStatus.stalled(
            reason: StallReason.storageUnavailable,
          ),
          stale: false,
        ),
        WalletBadgeLevel.caution,
      );
    });

    test('every other stall reason → RED (can\'t sync, needs the user)', () {
      // torUnavailable stays RED deliberately (#399): fail-closed privacy
      // down is never calm — even when the underlying cause is airplane
      // mode, the SDK cannot verify the privacy path and must not soothe.
      for (final reason in const [
        StallReason.torUnavailable,
        StallReason.storageFull,
        StallReason.internal,
        StallReason.unknown,
      ]) {
        expect(
          walletBadgeLevel(
            status: SyncStatus.stalled(reason: reason),
            stale: false,
          ),
          WalletBadgeLevel.error,
          reason: '$reason should be error',
        );
      }
    });

    test(
      'an uninterpretable status → YELLOW, never green/red (honesty rule)',
      () {
        expect(
          walletBadgeLevel(status: const SyncStatus.unknown(), stale: false),
          WalletBadgeLevel.caution,
        );
      },
    );

    test('#383 R1: sync disabled caps a fresh up-to-date at YELLOW — nothing '
        'is checking the chain, so GREEN would overclaim', () {
      expect(
        walletBadgeLevel(
          status: const SyncStatus.upToDate(tip: 100),
          stale: false,
          syncDisabled: true,
        ),
        WalletBadgeLevel.caution,
      );
    });

    test('S205-b: sync disabled OWNS the level — a retained non-reorg Stalled '
        'reads YELLOW, not RED (the stall is not live; an error tint would '
        'contradict the calm "Sync off" copy)', () {
      // The fold REVERSED the old "never lowers RED" rule: under a
      // deliberate host off nothing is retrying, so the retained stall is not
      // a live fault — caution is the one honest tone in BOTH directions.
      for (final reason in const [
        StallReason.endpointUnreachable,
        StallReason.torUnavailable,
        StallReason.storageFull,
        StallReason.internal,
        StallReason.unknown,
      ]) {
        expect(
          walletBadgeLevel(
            status: SyncStatus.stalled(reason: reason),
            stale: false,
            syncDisabled: true,
          ),
          WalletBadgeLevel.caution,
          reason: '$reason under sync-off must read caution, never error',
        );
      }
    });

    test('S205-b: sync disabled + a STALE retained UpToDate stays YELLOW '
        '(deliberately not fresh — nothing to fix but a setting)', () {
      expect(
        walletBadgeLevel(
          status: const SyncStatus.upToDate(tip: 100),
          stale: true,
          syncDisabled: true,
        ),
        WalletBadgeLevel.caution,
      );
    });

    test('a hard stall stays RED even with a stale flag set', () {
      // stale must not mask a genuine can\'t-sync into a milder yellow.
      expect(
        walletBadgeLevel(
          status: const SyncStatus.stalled(reason: StallReason.internal),
          stale: true,
        ),
        WalletBadgeLevel.error,
      );
    });
  });
}
