import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'wallet_providers.dart';
import 'wallet_session.dart';

/// Where the activity list is in its load cycle. The rows are carried alongside
/// (in [WalletActivityState]) so they STAY visible across a refresh/load-more
/// fault — a busy-DB throw must never blank a populated history (invariant 10,
/// the §3.3 last-known-through-error contract the snapshot also honors).
enum WalletActivityPhase {
  /// The FIRST page is loading and no rows are known yet → a bounded spinner.
  loading,

  /// Rows are loaded (possibly empty) and idle.
  ready,

  /// A NEXT page is being fetched (the user tapped "load more") → the existing
  /// rows stay, with a trailing spinner.
  loadingMore,

  /// The FIRST page failed with no prior rows → the honest error line. (A
  /// refresh/load-more fault keeps [ready] + the last-known rows instead.)
  error,
}

/// The activity list's accumulating state (FR-1 keyset pagination). Holds the
/// rows fetched so far and the OPAQUE cursor to continue ([nextCursor] non-null ⇔
/// more rows may follow). Rust stays the source of truth — this is ephemeral
/// presentation state (the visible window of the history), never a Dart cache of
/// authoritative data (design invariant 1).
class WalletActivityState {
  const WalletActivityState({
    this.rows = const [],
    this.nextCursor,
    this.phase = WalletActivityPhase.loading,
  });

  final List<TxSummary> rows;

  /// The opaque keyset cursor for the next page; `null` ⇒ the last page (no more).
  final String? nextCursor;

  final WalletActivityPhase phase;

  /// Whether a "load more" affordance should show — there is a next page AND we
  /// are not already fetching it.
  bool get canLoadMore =>
      nextCursor != null && phase != WalletActivityPhase.loadingMore;

  WalletActivityState copyWith({
    List<TxSummary>? rows,
    String? Function()? nextCursor,
    WalletActivityPhase? phase,
  }) => WalletActivityState(
    rows: rows ?? this.rows,
    nextCursor: nextCursor != null ? nextCursor() : this.nextCursor,
    phase: phase ?? this.phase,
  );
}

/// Drives the activity list (FR-1): the first page on (re)build, [loadMore] to
/// page the keyset cursor, [refresh] to re-read page 1 WITHOUT blanking the
/// current rows (the sync-edge update). Rebuilt — fresh first page — when its one
/// dependency, `walletSessionProvider`, changes (e.g. the rescan session swap),
/// which is exactly the right reset (the rebuilt DB repopulates from page 1).
final walletActivityProvider =
    NotifierProvider<WalletActivityController, WalletActivityState>(
      WalletActivityController.new,
    );

class WalletActivityController extends Notifier<WalletActivityState> {
  /// Touching `ref`/`state` after dispose throws; every async continuation
  /// re-checks this. Reset on every build (Riverpod may reuse the instance).
  bool _disposed = false;

  /// Monotonic op generation — the last-writer-wins guard that serializes
  /// OVERLAPPING async loads. `build` (session swap), `refresh`, and the start of
  /// each `loadMore` bump it; every continuation captures the generation at its
  /// start and writes `state` only if it is still current. Without it, an in-flight
  /// `loadMore` whose `await` resolves AFTER a sync-edge `refresh` (or a rescan
  /// session swap) would clobber the fresh rows with its stale captured snapshot —
  /// a momentary money-visibility flicker (a just-confirmed tx disappearing).
  int _generation = 0;

  @override
  WalletActivityState build() {
    _disposed = false;
    ref.onDispose(() => _disposed = true);
    final session = ref.watch(walletSessionProvider);
    if (session == null) {
      // No wallet — the screen renders the not-set-up state and does not watch
      // this. A neutral loading is the honest value.
      return const WalletActivityState();
    }
    // A fresh (re)build (e.g. the rescan session swap) supersedes any in-flight
    // continuation captured before it — they see a stale generation and no-op.
    final gen = ++_generation;
    unawaited(_loadFirst(session, gen));
    return const WalletActivityState(phase: WalletActivityPhase.loading);
  }

  Future<void> _loadFirst(WalletSession session, int gen) async {
    try {
      final page = await session
          .transactions(limit: walletActivityPageSize)
          // Honest degradation (principle 6): a LOCAL DB read — a hang means the
          // FFI boundary is wedged, not a slow network. Bound it so a wedged read
          // surfaces the error line, not an endless spinner on a money surface.
          .timeout(walletFfiWedgeTimeout);
      if (_disposed || gen != _generation) return;
      state = WalletActivityState(
        rows: page.rows,
        nextCursor: page.nextCursor,
        phase: WalletActivityPhase.ready,
      );
    } catch (_) {
      if (_disposed || gen != _generation) return;
      // First load, no prior rows → the honest error line.
      state = const WalletActivityState(phase: WalletActivityPhase.error);
    }
  }

  /// Fetch the NEXT keyset page and append it. No-op if there is no next page or
  /// a page is already in flight (the double-tap guard). A fault keeps the rows
  /// we have and returns to [ready] so the user can retry (no blanking, no throw).
  Future<void> loadMore() async {
    final current = state;
    final cursor = current.nextCursor;
    if (cursor == null || current.phase == WalletActivityPhase.loadingMore) {
      return;
    }
    final session = ref.read(walletSessionProvider);
    if (session == null) return;
    // Bump AFTER the guards (a no-op double-tap must not invalidate the in-flight
    // page). A concurrent refresh that started earlier now sees a stale generation.
    final gen = ++_generation;
    state = current.copyWith(phase: WalletActivityPhase.loadingMore);
    try {
      final page = await session
          .transactions(limit: walletActivityPageSize, after: cursor)
          .timeout(walletFfiWedgeTimeout);
      if (_disposed || gen != _generation) return;
      state = WalletActivityState(
        rows: [...current.rows, ...page.rows],
        nextCursor: page.nextCursor,
        phase: WalletActivityPhase.ready,
      );
    } catch (_) {
      if (_disposed || gen != _generation) return;
      // Keep the rows already loaded; drop back to ready so "load more" is offered
      // again (the cursor is preserved — current.nextCursor still points past them).
      state = current.copyWith(phase: WalletActivityPhase.ready);
    }
  }

  /// Re-read page 1 in place (the sync-edge refresh — a newly arrived/confirmed
  /// tx). Keeps the CURRENT rows visible until the fresh page lands, so a transient
  /// busy-DB fault never blanks a populated history (last-known-through-error).
  Future<void> refresh() async {
    // DO NOT collapse a deep-loaded (multi-page) list. The sync `reachedTip` edge
    // re-fires roughly per block while a synced wallet is open (~every block at the
    // tip), so a wholesale page-1 re-read here would YANK the accumulated pages back
    // to 50 rows every ~block and lose the user's place. The single-page common case
    // still auto-refreshes (no tail to drop). A REAL arrival must NOT ride this
    // guard — that is [collapseToFirstPage]'s job (U1: without it a
    // deep-paged list was a session-durable dead zone where a new payment never
    // appeared at all). The balance still updates live via
    // `walletSnapshotProvider` (refreshed on the same edges).
    if (state.rows.length > walletActivityPageSize) return;
    await _reloadFirstPage();
  }

  /// The ARRIVAL-edge refresh (U1): like [refresh] but BYPASSES the
  /// deep-page guard — a real incoming payment (rare, user-meaningful; the
  /// caller gates on `newTxCount > 0`) is worth collapsing pagination back to
  /// page 1, because the alternative is a deep-paged list that NEVER shows the
  /// new payment (both refresh edges no-op deep-paged, and nothing else
  /// re-reads until a session swap). No-blanking holds: the current rows stay
  /// visible until the fresh page lands.
  Future<void> collapseToFirstPage() => _reloadFirstPage();

  Future<void> _reloadFirstPage() async {
    final session = ref.read(walletSessionProvider);
    if (session == null) return;
    final gen = ++_generation;
    try {
      final page = await session
          .transactions(limit: walletActivityPageSize)
          .timeout(walletFfiWedgeTimeout);
      if (_disposed || gen != _generation) return;
      state = WalletActivityState(
        rows: page.rows,
        nextCursor: page.nextCursor,
        phase: WalletActivityPhase.ready,
      );
    } catch (_) {
      if (_disposed || gen != _generation) return;
      // Keep last-known rows if we HAVE them (last-known-through-error); but if we
      // never loaded any (a first load that also failed this refresh), surface the
      // honest error rather than a misleading empty state (invariant 10).
      state = state.copyWith(
        phase: state.rows.isEmpty
            ? WalletActivityPhase.error
            : WalletActivityPhase.ready,
      );
    }
  }
}
