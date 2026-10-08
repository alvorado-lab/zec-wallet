import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_activity_controller.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// The activity list controller (FR-1 keyset pagination). Pins the load cycle:
/// first page → load-more APPENDS the next keyset page → last page has no cursor;
/// refresh re-reads page 1 WITHOUT blanking; faults degrade honestly. All against
/// the in-memory fake (no native lib) behind the same WalletSession port.
void main() {
  ProviderContainer harness(FakeWalletSession fake) {
    final container = ProviderContainer(
      overrides: [walletSessionProvider.overrideWithValue(fake)],
    );
    addTearDown(container.dispose);
    container.listen(walletActivityProvider, (_, _) {});
    return container;
  }

  WalletActivityState stateOf(ProviderContainer c) =>
      c.read(walletActivityProvider);
  WalletActivityController notifierOf(ProviderContainer c) =>
      c.read(walletActivityProvider.notifier);

  TxSummary tx(String hex) => txSummaryFixture(txidHex: hex);

  test(
    'the first page loads rows + the cursor; ready when it settles',
    () async {
      final fake = FakeWalletSession()
        ..transactionsResult = [tx('a1'), tx('a2')]
        ..nextCursorResult = 'cursor-1';
      final c = harness(fake);

      expect(stateOf(c).phase, WalletActivityPhase.loading);
      await pumpEventQueue();

      final s = stateOf(c);
      expect(s.phase, WalletActivityPhase.ready);
      expect(s.rows.map((r) => r.txidHex), ['a1', 'a2']);
      expect(s.nextCursor, 'cursor-1');
      expect(s.canLoadMore, isTrue);
      expect(fake.lastTransactionsLimit, walletActivityPageSize);
      expect(
        fake.lastTransactionsAfter,
        isNull,
        reason: 'first page has no cursor',
      );
    },
  );

  test(
    'loadMore APPENDS the next keyset page and threads the cursor',
    () async {
      final fake = FakeWalletSession()
        ..pagesByCursor.addAll({
          null: const HistoryPage(rows: [], nextCursor: 'c1'),
          'c1': const HistoryPage(rows: [], nextCursor: 'c2'),
          'c2': const HistoryPage(rows: [], nextCursor: null),
        })
        // Give each page distinct rows via the map (override the empty above).
        ..pagesByCursor[null] = HistoryPage(
          rows: [tx('p1a'), tx('p1b')],
          nextCursor: 'c1',
        )
        ..pagesByCursor['c1'] = HistoryPage(rows: [tx('p2a')], nextCursor: 'c2')
        ..pagesByCursor['c2'] = HistoryPage(
          rows: [tx('p3a')],
          nextCursor: null,
        );
      final c = harness(fake);
      await pumpEventQueue();
      expect(stateOf(c).rows.map((r) => r.txidHex), ['p1a', 'p1b']);

      await notifierOf(c).loadMore();
      expect(
        fake.lastTransactionsAfter,
        'c1',
        reason: 'paged with the page-1 cursor',
      );
      expect(stateOf(c).rows.map((r) => r.txidHex), ['p1a', 'p1b', 'p2a']);
      expect(stateOf(c).nextCursor, 'c2');

      await notifierOf(c).loadMore();
      expect(stateOf(c).rows.map((r) => r.txidHex), [
        'p1a',
        'p1b',
        'p2a',
        'p3a',
      ]);
      expect(stateOf(c).nextCursor, isNull, reason: 'last page ⇒ no more');
      expect(stateOf(c).canLoadMore, isFalse);
    },
  );

  test('S210 U1: refresh() skips a deep-paged list, but collapseToFirstPage() '
      'reloads it - a REAL arrival is never a dead zone', () async {
    final page1 = List.generate(walletActivityPageSize, (i) => tx('a1$i'));
    final fake = FakeWalletSession()
      ..transactionsResult = page1
      ..nextCursorResult = 'cursor-1';
    final c = harness(fake);
    await pumpEventQueue();

    fake
      ..transactionsResult = [tx('b1'), tx('b2')]
      ..nextCursorResult = null;
    await notifierOf(c).loadMore();
    expect(
      stateOf(c).rows.length,
      walletActivityPageSize + 2,
      reason: 'deep-paged fixture',
    );

    // A new payment lands; the fresh page 1 now leads with it.
    fake.transactionsResult = [tx('fresh')];

    await notifierOf(c).refresh();
    expect(
      stateOf(c).rows.length,
      walletActivityPageSize + 2,
      reason:
          'the guarded refresh must NOT yank a deep-paged list '
          '(the per-block reachedTip re-fire protection)',
    );

    await notifierOf(c).collapseToFirstPage();
    final s = stateOf(c);
    expect(
      s.rows.map((r) => r.txidHex),
      ['fresh'],
      reason: 'the arrival edge collapses to page 1 - the new payment shows',
    );
    expect(s.phase, WalletActivityPhase.ready);
  });

  test('loadMore at the last page is a no-op (no extra fetch)', () async {
    final fake = FakeWalletSession()
      ..transactionsResult = [tx('only')]
      ..nextCursorResult = null; // last page already
    final c = harness(fake);
    await pumpEventQueue();
    final countBefore = fake.transactionsCount;

    await notifierOf(c).loadMore();

    expect(fake.transactionsCount, countBefore, reason: 'no cursor ⇒ no fetch');
    expect(stateOf(c).rows.map((r) => r.txidHex), ['only']);
  });

  test('a double-tapped loadMore fetches the next page only ONCE', () async {
    final gate = Completer<void>();
    final fake = FakeWalletSession()
      ..pagesByCursor[null] = HistoryPage(rows: [tx('p1')], nextCursor: 'c1')
      ..pagesByCursor['c1'] = HistoryPage(rows: [tx('p2')], nextCursor: null);
    final c = harness(fake);
    await pumpEventQueue();
    final countAfterFirst = fake.transactionsCount;
    // Hold the next-page fetch so the two taps overlap.
    fake.transactionsNeverCompletes = true;

    unawaited(notifierOf(c).loadMore()); // stays pending on the held fetch
    expect(stateOf(c).phase, WalletActivityPhase.loadingMore);
    final second = notifierOf(c).loadMore(); // must no-op (already loadingMore)
    await pumpEventQueue();

    expect(
      fake.transactionsCount,
      countAfterFirst + 1,
      reason: 'only ONE next-page fetch',
    );
    gate.complete();
    await second; // the no-op returns immediately
  });

  test('refresh re-reads page 1 in place; success replaces the rows', () async {
    final fake = FakeWalletSession()..transactionsResult = [tx('old')];
    final c = harness(fake);
    await pumpEventQueue();
    expect(stateOf(c).rows.map((r) => r.txidHex), ['old']);

    fake.transactionsResult = [tx('new1'), tx('new2')];
    await notifierOf(c).refresh();

    expect(stateOf(c).rows.map((r) => r.txidHex), ['new1', 'new2']);
    expect(stateOf(c).phase, WalletActivityPhase.ready);
  });

  test('refresh KEEPS the last-known rows through a transient fault', () async {
    final fake = FakeWalletSession()..transactionsResult = [tx('kept')];
    final c = harness(fake);
    await pumpEventQueue();

    fake.transactionsThrows = StateError('busy db');
    await notifierOf(c).refresh();

    expect(
      stateOf(c).rows.map((r) => r.txidHex),
      ['kept'],
      reason: 'a busy-DB throw must not blank a populated history',
    );
    expect(stateOf(c).phase, WalletActivityPhase.ready);
  });

  test(
    'a first-load fault surfaces the honest error (no rows to keep)',
    () async {
      final fake = FakeWalletSession()
        ..transactionsThrows = StateError('read failed');
      final c = harness(fake);
      await pumpEventQueue();

      expect(stateOf(c).phase, WalletActivityPhase.error);
      expect(stateOf(c).rows, isEmpty);
    },
  );

  test('loadMore that faults keeps the loaded rows + offers retry', () async {
    final fake = FakeWalletSession()
      ..pagesByCursor[null] = HistoryPage(rows: [tx('p1')], nextCursor: 'c1');
    final c = harness(fake);
    await pumpEventQueue();
    expect(stateOf(c).rows.map((r) => r.txidHex), ['p1']);

    fake.pagesByCursor.clear();
    fake.transactionsThrows = StateError('next page failed');
    await notifierOf(c).loadMore();

    expect(stateOf(c).rows.map((r) => r.txidHex), ['p1'], reason: 'rows kept');
    expect(stateOf(c).phase, WalletActivityPhase.ready);
    expect(
      stateOf(c).canLoadMore,
      isTrue,
      reason: 'cursor preserved → retry offered',
    );
  });

  test('a session swap (rescan) after loading multiple pages RESETS to a fresh '
      'page 1 — no stale pre-rescan rows linger', () async {
    // The rescan lifecycle edge: the user has paged deep into the OLD history,
    // then a rescan rebuilds the DB and swaps the session. The activity provider
    // watches walletSessionProvider, so the swap must tear down the accumulated
    // window and re-read page 1 from the REPOPULATING DB — never leave stale
    // rows (txs from the pre-rescan chain view) stacked above the fresh page
    // (a money-visibility lie: history the rebuilt DB no longer reflects).
    final oldFake = FakeWalletSession()
      ..pagesByCursor[null] = HistoryPage(
        rows: [tx('old1'), tx('old2')],
        nextCursor: 'c1',
      )
      ..pagesByCursor['c1'] = HistoryPage(rows: [tx('old3')], nextCursor: null);
    final c = harness(oldFake);
    await pumpEventQueue();
    await notifierOf(c).loadMore(); // accumulate both pages of the OLD session
    expect(
      stateOf(c).rows.map((r) => r.txidHex),
      ['old1', 'old2', 'old3'],
      reason: 'precondition: a deep, multi-page window of the old history',
    );

    // The rescan swaps in a fresh session whose rebuilt DB has a single early row.
    final newFake = FakeWalletSession()..transactionsResult = [tx('rebuilt1')];
    c.updateOverrides([walletSessionProvider.overrideWithValue(newFake)]);
    await pumpEventQueue();

    expect(
      stateOf(c).rows.map((r) => r.txidHex),
      ['rebuilt1'],
      reason:
          'the stale pre-rescan rows are gone; only the rebuilt page 1 shows',
    );
    expect(stateOf(c).nextCursor, isNull);
    expect(stateOf(c).phase, WalletActivityPhase.ready);
    expect(
      newFake.lastTransactionsAfter,
      isNull,
      reason: 'the rebuilt read starts at page 1 (no carried-over cursor)',
    );
  });

  test('refresh does NOT collapse a deep-loaded (multi-page) list', () async {
    // The operational MAJOR: the sync `reachedTip` edge re-fires ~per block at the
    // tip, so a wholesale page-1 re-read on refresh would YANK a deep-scrolled list
    // back to 50 rows every block. With >1 page loaded, refresh must be a no-op (the
    // balance still updates via the snapshot provider); only the single-page common
    // case auto-refreshes.
    final pageOne = List.generate(walletActivityPageSize, (i) => tx('p1_$i'));
    final fake = FakeWalletSession()
      ..pagesByCursor[null] = HistoryPage(rows: pageOne, nextCursor: 'c1')
      ..pagesByCursor['c1'] = HistoryPage(rows: [tx('p2_0')], nextCursor: null);
    final c = harness(fake);
    await pumpEventQueue();
    await notifierOf(
      c,
    ).loadMore(); // now walletActivityPageSize + 1 rows loaded
    expect(
      stateOf(c).rows.length,
      walletActivityPageSize + 1,
      reason: 'precondition: a deep multi-page window',
    );
    final countBefore = fake.transactionsCount;

    await notifierOf(c).refresh();

    expect(
      fake.transactionsCount,
      countBefore,
      reason: 'no page-1 re-read while deep-loaded → no yank',
    );
    expect(
      stateOf(c).rows.length,
      walletActivityPageSize + 1,
      reason: 'the accumulated pages are preserved',
    );
  });

  test(
    'refresh still re-reads when only the single first page is loaded',
    () async {
      // The complement: the common case (≤1 page) DOES auto-refresh, so a new/confirmed
      // tx appears on the sync edge without a manual pull.
      final fake = FakeWalletSession()..transactionsResult = [tx('a')];
      final c = harness(fake);
      await pumpEventQueue();
      fake.transactionsResult = [tx('b1'), tx('b2')];

      await notifierOf(c).refresh();

      expect(
        stateOf(c).rows.map((r) => r.txidHex),
        ['b1', 'b2'],
        reason: 'single-page list refreshes in place',
      );
    },
  );
}
