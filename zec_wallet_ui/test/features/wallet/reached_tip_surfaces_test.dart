import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/lifecycle/app_lifecycle_provider.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_screen.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// UI-1 rows T-1 and T-4 (`docs/plan/production-readiness-phase-1.md` §4r,
/// U-1 and U-4): the two SESSION-scoped consumers of a reached tip.
///
///  * **T-1 — the latch** (`WalletSyncedTipNotifier`, §4r U-1). The five
///    reached-tip variants are all "a completed pass": `UpToDate`,
///    `UpToDateLimited`, `UpToDateDegraded`, `UpToDateUnverified` latch their
///    tip from ANY `cur`; `EndpointBehind` — a completed pass on a server that
///    is BEHIND the chain — latches only from `Unset` or from a latch it does
///    not lower, and never clears an `Invalidated` verdict (§4m #4: a
///    forked/behind server must not resurrect the height a rewind just
///    invalidated).
///  * **T-4 — the freshness edge** (`wallet_screen.dart`'s sync listener,
///    §4r U-4). Reaching a tip re-reads the cold snapshot; the edge is into a
///    reached tip, not into `UpToDate` alone (§4m #18).
///
/// The latch harness is the one `wallet_screen_test.dart` uses for
/// `walletSyncedTipProvider` (§4r Q-U2, answered): a pure [ProviderContainer]
/// over a [FakeWalletSession] plus the deterministic lifecycle fake, driven by
/// `fake.push(...)`. `_next` is private, and stays private — the provider is
/// the seam, so no widget pump and no new production door are needed for T-1.
///
/// The edge's observable is `FakeWalletSession.snapshotCount`: the listener
/// invalidates `walletSnapshotReadProvider`, whose body is `session.snapshot()`
/// (`wallet_providers.dart:271-296`) — one cold read per invalidation. No new
/// seam is needed for T-4 either.
///
/// **T-6 — the watch list** (§4r): which row reds under each of the five
/// mutants the implementer plants. Named across all four files this round
/// touches, so a mutant that reds nothing is visible as a hole.
///
///  1. the `EndpointBehind` guard removed (the arm latches from any `cur`
///     again) → THIS file: "EndpointBehind from Invalidated stays Invalidated"
///     AND "EndpointBehind never LOWERS a latch". The five "from Unset"
///     rows stay green — the guard is not what makes them pass.
///  2. the sheet arm removed (back to `default: break`) → `sync_status_sheet_test`:
///     all four T-2 rows ("… renders the Synced to row" ×3 and the
///     EndpointBehind row). The M<1 control stays green either way — it is a
///     control, and it is the row that reds if the behind figure is rendered
///     UNGUARDED.
///  3. the pool line removed → `sync_status_presentation_test`: the three
///     T-3 defect rows. The degraded predicate INVERTED instead → the
///     fully-served CONTROL, plus "the refused pool is NAMED, and the served
///     ones are not" (its two `isEmpty` legs).
///  4. the edge narrowed back to `UpToDate`-only → THIS file: "Scanning ->
///     UpToDateDegraded invalidates the snapshot read". Both T-4 controls
///     stay green — that is what makes the red mean the narrowing.
///  5. a string reverted → `zec-wallet-core/tests/extraction_policy.rs`:
///     `the_birthday_stall_names_the_block_the_wallet_is_set_to` for
///     `walletStallBirthdayInFuture`, and
///     `the_clock_copy_makes_the_device_time_a_precondition_not_an_alternative`
///     for either clock string. A key DROPPED from a locale instead reds
///     `every_locale_carries_the_three_reworded_keys`.

/// The pool report used wherever a status needs one. Sapling unsupported is a
/// degradation the Rust `report_is_degraded` predicate calls degraded
/// (`sync_controller.rs:661-708`); the other two are served, one of them at
/// the honest zero (a pool before its first 2^16 notes).
const _pools = PoolServiceReport(
  sapling: PoolService.unsupported(),
  orchard: PoolService.served(roots: 7),
  ironwood: PoolService.served(roots: 0),
);

const _grace = UnknownBranchGrace.running(blocksLeft: 1000, secsLeft: 6400);

/// The five variants §4r U-1 calls "a completed pass", with the tip each one
/// must leave in the latch.
final _reachedTip = <({String name, SyncStatus status, int tip})>[
  (name: 'UpToDate', status: const SyncStatus.upToDate(tip: 700), tip: 700),
  (
    name: 'UpToDateLimited',
    status: const SyncStatus.upToDateLimited(tip: 701),
    tip: 701,
  ),
  (
    name: 'UpToDateDegraded',
    status: const SyncStatus.upToDateDegraded(tip: 702, pools: _pools),
    tip: 702,
  ),
  (
    name: 'EndpointBehind',
    status: const SyncStatus.endpointBehind(tip: 703, newestKnown: 900),
    tip: 703,
  ),
  (
    name: 'UpToDateUnverified',
    status: const SyncStatus.upToDateUnverified(
      tip: 704,
      grace: _grace,
      streakReported: false,
    ),
    tip: 704,
  ),
];

Widget _screen(FakeWalletSession session) => ProviderScope(
  overrides: [walletSessionProvider.overrideWithValue(session)],
  child: MaterialApp(
    localizationsDelegates: WalletLocalizations.localizationsDelegates,
    supportedLocales: WalletLocalizations.supportedLocales,
    theme: lightTheme,
    home: const WalletScreen(),
  ),
);

void main() {
  // ---------------------------------------------------------------- T-1 ----
  group('T-1 the tip latch — one rule for the five reached-tip variants '
      '(§4r U-1)', () {
    // The wallet_screen_test harness, verbatim in shape: a controllable
    // session seam + the deterministic lifecycle fake (SyncStatusNotifier
    // watches it).
    ({ProviderContainer container, FakeWalletSession fake}) latchHarness({
      FakeWalletSession? session,
    }) {
      final fake = session ?? FakeWalletSession();
      final container = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWithValue(fake),
          appLifecycleProvider.overrideWith(TestLifecycleNotifier.new),
        ],
      );
      addTearDown(container.dispose);
      return (container: container, fake: fake);
    }

    // Let the fake stream's replay/pushes flush through the notifier chain.
    Future<void> settle() => Future<void>.delayed(Duration.zero);

    int? tipOf(ProviderContainer c) {
      final latch = c.read(walletSyncedTipProvider);
      return latch is WalletSyncedTipLatched ? latch.tip : null;
    }

    /// The latched verdict, or a NAMED failure instead of a cast error.
    WalletSyncedTipLatched latchedOf(ProviderContainer c, String reason) {
      final latch = c.read(walletSyncedTipProvider);
      expect(latch, isA<WalletSyncedTipLatched>(), reason: reason);
      return latch as WalletSyncedTipLatched;
    }

    /// A container whose latch is UNSET, listening, ready to be driven.
    ({ProviderContainer container, FakeWalletSession fake}) unsetHarness() {
      final h = latchHarness(
        session: FakeWalletSession(current: const SyncStatus.idle()),
      );
      final sub = h.container.listen(walletSyncedTipProvider, (_, _) {});
      addTearDown(sub.close);
      return h;
    }

    for (final c in _reachedTip) {
      test('from Unset, ${c.name} latches its tip', () async {
        final h = unsetHarness();
        await settle();
        expect(
          h.container.read(walletSyncedTipProvider),
          isA<WalletSyncedTipUnset>(),
          reason: 'precondition: idle — this session has no verdict yet',
        );

        h.fake.push(c.status);
        await settle();

        expect(
          tipOf(h.container),
          c.tip,
          reason:
              '${c.name} is a completed pass on a CURRENT server (§4r U-1) — '
              'a null here is the "as of block null" header §4j row 10 names',
        );
      });
    }

    test('the replay rule reaches the new arms too — the SAME tip keeps its '
        'observation moment (a re-subscribe replay never dresses an old '
        'claim up as fresh)', () async {
      final h = unsetHarness();
      await settle();

      h.fake.push(const SyncStatus.upToDateDegraded(tip: 702, pools: _pools));
      await settle();
      final first = latchedOf(
        h.container,
        'precondition: the degraded pass latched (§4r U-1)',
      );

      // A routine catch-up pass ABOVE the latch holds the verdict, then the
      // pass ends at the SAME tip (nothing new landed). The interleaved
      // sample is what makes the second push a distinct emission at all.
      h.fake.push(
        const SyncStatus.scanning(
          from: 1,
          to: 800,
          percent: 0.5,
          spendableReady: true,
          rewound: false,
        ),
      );
      h.fake.push(const SyncStatus.upToDateDegraded(tip: 702, pools: _pools));
      await settle();

      final again = latchedOf(h.container, 'still latched after the replay');
      expect(again.tip, 702);
      expect(
        again.at,
        first.at,
        reason: 'same tip -> same moment (the existing replay rule, U-1)',
      );
    });

    test('EndpointBehind from Invalidated stays Invalidated — a behind or '
        'forked server never resurrects the height a rewind invalidated '
        '(§4m #4)', () async {
      final h = unsetHarness();
      await settle();

      h.fake.push(const SyncStatus.upToDate(tip: 700));
      await settle();
      expect(tipOf(h.container), 700, reason: 'precondition: a proven tip');

      h.fake.push(
        const SyncStatus.scanning(
          from: 1,
          to: 760,
          percent: 0.3,
          spendableReady: true,
          rewound: true,
        ),
      );
      await settle();
      expect(
        h.container.read(walletSyncedTipProvider),
        isA<WalletSyncedTipInvalidated>(),
        reason: 'precondition: the rewind invalidated the claim',
      );

      // A HIGHER tip than the invalidated latch: the rule is about the
      // verdict, not about the height.
      h.fake.push(const SyncStatus.endpointBehind(tip: 800, newestKnown: 900));
      await settle();

      expect(
        h.container.read(walletSyncedTipProvider),
        isA<WalletSyncedTipInvalidated>(),
        reason:
            'a completed pass on a server that is BEHIND the chain is not the '
            'proof that clears a rewind (§4r U-1); only a current server is',
      );
    });

    test('EndpointBehind never LOWERS a latch — a lower-tip server keeps the '
        'higher proven height (§4m #4)', () async {
      final h = unsetHarness();
      await settle();

      h.fake.push(const SyncStatus.upToDate(tip: 900));
      await settle();
      expect(tipOf(h.container), 900, reason: 'precondition: latched at 900');

      h.fake.push(const SyncStatus.endpointBehind(tip: 600, newestKnown: 950));
      await settle();

      expect(
        tipOf(h.container),
        900,
        reason:
            'the header must not walk BACKWARDS to a behind server\'s tip — '
            '§4r U-1 latches EndpointBehind only from Unset or a tip it does '
            'not lower',
      );
    });

    test('CONTROL: UpToDate from Invalidated re-latches — a completed pass on '
        'a CURRENT server is exactly the proof that clears a rewind', () async {
      final h = unsetHarness();
      await settle();

      h.fake.push(const SyncStatus.upToDate(tip: 700));
      await settle();
      h.fake.push(
        const SyncStatus.scanning(
          from: 1,
          to: 760,
          percent: 0.3,
          spendableReady: true,
          rewound: true,
        ),
      );
      await settle();
      expect(
        h.container.read(walletSyncedTipProvider),
        isA<WalletSyncedTipInvalidated>(),
        reason: 'precondition: invalidated',
      );

      h.fake.push(const SyncStatus.upToDate(tip: 800));
      await settle();

      expect(tipOf(h.container), 800);
    });
  });

  // ---------------------------------------------------------------- T-4 ----
  group('T-4 the freshness edge — reaching a tip re-reads the cold snapshot '
      '(§4r U-4)', () {
    const scanning = SyncStatus.scanning(
      from: 1,
      to: 800,
      percent: 0.2,
      // FALSE so the `becameSpendable` edge cannot fire and stand in for the
      // `reachedTip` one — this row measures ONE edge.
      spendableReady: false,
      rewound: false,
    );

    Future<FakeWalletSession> mountScanning(WidgetTester tester) async {
      final fake = FakeWalletSession(
        current: scanning,
        snapshotValue: walletStateFixture(syncStatus: scanning),
      );
      await tester.pumpWidget(_screen(fake));
      await tester.pumpAndSettle();
      expect(
        fake.snapshotCount,
        greaterThan(0),
        reason: 'precondition: the mount cold-read the snapshot',
      );
      return fake;
    }

    testWidgets('CONTROL: Scanning -> UpToDate invalidates the snapshot read '
        '(the reference edge this row compares to)', (tester) async {
      final fake = await mountScanning(tester);
      final before = fake.snapshotCount;

      fake.push(const SyncStatus.upToDate(tip: 800));
      await tester.pumpAndSettle();

      expect(
        fake.snapshotCount,
        greaterThan(before),
        reason:
            'the ADR-0533 balance-freshness edge — if this fails the harness '
            'cannot see ANY invalidation and the sibling rows say nothing',
      );
    });

    testWidgets('Scanning -> UpToDateDegraded invalidates the snapshot read, '
        'exactly as Scanning -> UpToDate does (§4m #18)', (tester) async {
      final fake = await mountScanning(tester);
      final before = fake.snapshotCount;

      fake.push(const SyncStatus.upToDateDegraded(tip: 800, pools: _pools));
      await tester.pumpAndSettle();

      expect(
        fake.snapshotCount,
        greaterThan(before),
        reason:
            'a degraded pass reached the tip and found notes on the way; the '
            'balance beside it is a floor, and a floor read from a stale '
            'snapshot is a lower floor still (§4r U-4)',
      );
    });

    testWidgets('CONTROL: UpToDate -> UpToDate does NOT re-read — the edge is '
        'the transition, never every tip advance at the tip', (tester) async {
      final fake = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 100),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: 100),
        ),
      );
      await tester.pumpWidget(_screen(fake));
      await tester.pumpAndSettle();
      final before = fake.snapshotCount;

      fake.push(const SyncStatus.upToDate(tip: 101));
      await tester.pumpAndSettle();

      expect(
        fake.snapshotCount,
        before,
        reason:
            'the reachedTip edge re-fires ~per block at the tip; a re-read on '
            'every one is the ADR-0532 cost the edge exists to avoid',
      );
    });
  });
}
