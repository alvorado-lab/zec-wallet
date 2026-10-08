import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';

import '../../../core/lifecycle/app_lifecycle_provider.dart';
import '../wallet_providers.dart';
import '../wallet_session.dart';
import 'swap_enabled_provider.dart';

/// The live `SwapStatus` of ONE in-flight swap (spec §3.3/§7), keyed by the
/// opaque swap id [swapExecute] returned. A family so each tracked swap has its
/// own subscription + lifecycle; the screen watches `swapStatusProvider(swapId)`
/// once an execute has minted an id.
///
/// LIFECYCLE vs the sync stream — a deliberate, smaller machine. It mirrors
/// [SyncStatusNotifier]'s foreground-only gating (cancel on background, re-open
/// on a real resume, desktop stays live while hidden), but it has NO
/// reconnect/backoff machinery, because the swap stream's end semantics differ
/// fundamentally from sync's:
///
/// The core poll loop (`zec-wallet-core swap::service::poll_loop`) RETRIES a
/// transport/provider fault INTERNALLY forever ("a stall is DATA, not a dead
/// stream") — it NEVER ends the stream on a transport drop. It ends (Dart
/// `onDone`) only INTENTIONALLY: a TERMINAL status was just emitted, a Hard kill
/// halted it (§3.5), or the wallet tore down. So a Dart `onDone` is never a
/// transport drop to reconnect from — reconnecting would re-spawn a poll the core
/// just stopped (a hot loop under a Hard kill). Therefore:
///
/// - a TERMINAL status (success/refunded/failed) latches `_done` and the
///   following EOF is a clean end;
/// - a PRE-TERMINAL EOF (Hard kill / teardown) also latches `_done` and STOPS —
///   the last status stays visible and the host renders "tracking unavailable"
///   from its OWN kill state (§3.5; the host owns the kill, the stream has no
///   wire signal for it);
/// - an `onError` is an establish-time typed failure (`SwapDisabled` /
///   `SwapStateUnavailable` / a bad id — the core never routes a transient fault
///   here), surfaced as an error and STOPPED, never retried;
/// - the ONLY re-subscription is the user-paced lifecycle RESUME (the core
///   re-emits the current status at once), so there is no auto-reconnect and no
///   background polling. `Unknown` is NOT terminal (a forward-compat arm; the
///   core never emits it) so an unrecognized status never abandons a live swap.
///
/// SAME-SESSION RETENTION (the s198_tail polish): an invalidate-driven
/// rebuild — the #386 re-entry re-poll of a stale latched NotFound — used to
/// reset to a BARE loading, discarding the latched status; the tracking card
/// then busy-flashed under the "Swap started" label for the full ~75–135 s
/// 404 re-earn. The rebuild is now SEEDED with the last stream-emitted status
/// (`copyWithPrevious`, so the screen's default `skipLoadingOnRefresh` keeps
/// the honest card rendered while the re-poll runs) — but ONLY when the
/// rebuild rides the IDENTICAL session object. A session/identity flip clears
/// the carried state, so no owner status can ever RENDER under another
/// identity (the sibling class must not widen; heap timing per the
/// `_lastEmitted` caveat below). A kill-flip (swap OFF → ON, same session)
/// re-renders the retained last status instantly while the fresh
/// subscription's re-emit refreshes it — deliberate, consistent with the
/// pause posture. A COLD re-attach (process death) has no carried state
/// either way and keeps the busy arm — the neutral loading-label copy for
/// that case is the parked #347-adjacent item.
final swapStatusProvider =
    NotifierProvider.family<SwapStatusNotifier, AsyncValue<SwapStatus>, String>(
      SwapStatusNotifier.new,
    );

class SwapStatusNotifier extends Notifier<AsyncValue<SwapStatus>> {
  SwapStatusNotifier(this._swapId);

  /// The swap id this family instance tracks (the family key, passed by Riverpod
  /// to the constructor). Fixed for the instance's life.
  final String _swapId;

  StreamSubscription<SwapStatus>? _sub;

  /// Tracked locally because the lifecycle arrives as
  /// `paused → hidden → inactive → resumed`: at `resumed` the previous state is
  /// `inactive`, so only a remembered flag distinguishes a real resume from
  /// window-focus / shade noise.
  bool _wasPaused = false;

  /// The stream ended FOR GOOD — a terminal outcome was reached, OR a
  /// non-terminal end (Hard kill / wallet teardown) stopped it. Latches off all
  /// re-subscription (resume is a no-op once `_done`), so a finished or
  /// killed swap is never re-polled.
  bool _done = false;

  /// The last stream-EMITTED status (data only; an establish-time error
  /// clears it) — the same-session retention seed (see the provider doc).
  /// Cleared by the first BUILD that sees a different session object, so it
  /// can never carry a status across a wallet identity change.
  /// TIMING CAVEAT (the memory-hygiene class, mirrored from the fence
  /// note in wallet_providers.dart): an UNLISTENED element's rebuild is
  /// deferred to its next read, so after a flip the old identity's last
  /// status (and the old session reference below) can linger in heap until
  /// that read — never renderable (any read flushes the dirty element, and
  /// `build()` drops the retention against the CURRENT session before state
  /// is observable).
  AsyncValue<SwapStatus>? _lastEmitted;

  /// The session the previous build subscribed under — `identical` comparison
  /// only (never dereferenced), replaced on every build.
  WalletSession? _lastSession;

  @override
  AsyncValue<SwapStatus> build() {
    // PER-BUILD RESET: Riverpod REUSES the notifier instance across a
    // `walletSessionProvider` rebuild (session close/reopen, onboarding flip) and
    // fires `ref.onDispose` (→ `_teardown`) BEFORE re-running `build()`. So the
    // instance fields are stale on a rebuild — re-initialize the lifecycle from
    // scratch here (the build is the single "(re)start tracking" entry point).
    _wasPaused = false;
    _done = false;
    _sub = null;

    final session = ref.watch(walletSessionProvider);
    // Same-session retention seed (see the provider doc). Computed
    // BEFORE `_lastSession` is replaced; a different (or null) session drops
    // the carried status entirely so nothing crosses an identity flip.
    final retained = session != null && identical(session, _lastSession)
        ? _lastEmitted
        : null;
    if (retained == null) _lastEmitted = null;
    _lastSession = session;
    // The host kill-state SSOT (§3.5 layer 2). Watching it here is the explicit
    // teardown-on-kill (D-2b-2): when swap flips OFF, this build re-runs,
    // `ref.onDispose(_teardown)` fires FIRST (cancelling `_sub`), and the new
    // build takes the `!swapEnabled` branch below and does NOT re-subscribe — so
    // the poll stops. REQUIRED because this provider is non-autoDispose: it stays
    // alive after the screen stops watching it (the screen renders
    // "tracking unavailable" from `swapEnabledProvider` directly), and would
    // otherwise keep polling a killed provider indefinitely.
    final swapEnabled = ref.watch(swapEnabledProvider);
    ref.onDispose(_teardown);
    // Riverpod disposes the previous `ref.listen` registration before re-running
    // `build()`, so re-registering here on every rebuild does NOT accumulate.
    ref.listen<AppLifecycleState>(appLifecycleProvider, (_, next) {
      _onLifecycle(next);
    });

    if (session == null) {
      // No wallet — the screen renders the unavailable state, not this provider.
      return const AsyncValue<SwapStatus>.loading();
    }

    if (!swapEnabled) {
      // Host kill (§3.5): swap is off — do NOT open the poll. The screen renders
      // "tracking unavailable" from `swapEnabledProvider`; this branch is what
      // tears a LIVE subscription down when the kill flips (build re-ran, the
      // onDispose teardown already cancelled `_sub`, and we skip re-subscribe).
      return const AsyncValue<SwapStatus>.loading();
    }

    // Born-backgrounded cold start: do not open the poll stream while paused.
    if (ref.read(appLifecycleProvider) == AppLifecycleState.paused) {
      _wasPaused = true;
    } else {
      _subscribe(session);
    }
    // Seeded when the SAME session rebuilds (an invalidate re-poll): the
    // screen keeps rendering the last honest status while the fresh
    // subscription earns its first emission (`skipLoadingOnRefresh` default),
    // and `isLoading` is the re-poll-in-flight signal `_repollStaleNotFound`
    // guards on. `copyWithPrevious` is riverpod-@internal (async ELEMENTS run
    // it on every state write; a hand-rolled Notifier must call it directly —
    // there is no public "loading with a previous value" constructor). The
    // lifecycle tests pin the merged-state behavior, and the riverpod-upgrade
    // watchlist (#377) carries this alongside the other pinned
    // internal semantics.
    return retained != null
        // ignore: invalid_use_of_internal_member
        ? AsyncValue<SwapStatus>.loading().copyWithPrevious(retained)
        : const AsyncValue<SwapStatus>.loading();
  }

  void _onLifecycle(AppLifecycleState next) {
    switch (next) {
      case AppLifecycleState.paused:
        _wasPaused = true;
        _pause();
      case AppLifecycleState.resumed:
        if (_wasPaused) {
          _wasPaused = false;
          _resume();
        }
      // inactive / hidden / detached: no stream action. `hidden` is NOT a pause
      // (desktop deepest state + Android's pre-`paused` step) — the stream stays
      // live so a minimized desktop wallet keeps tracking a swap.
      case AppLifecycleState.inactive:
      case AppLifecycleState.hidden:
      case AppLifecycleState.detached:
        break;
    }
  }

  void _subscribe(WalletSession session) {
    if (_done) return; // the swap finished / was killed — never re-open
    _sub?.cancel();
    _sub = session
        .watchSwapStatus(swapId: _swapId)
        .listen(
          (status) {
            state = AsyncValue.data(status);
            // The retention seed tracks only stream-emitted DATA (an
            // establish-time error is surfaced-and-stopped, never re-seeded).
            _lastEmitted = state;
            if (_isTerminal(status)) {
              // The outcome is known; the core closes the stream right after. Latch
              // so the following `onDone` is a clean end and resume never re-polls.
              _done = true;
              // #367: the terminal was OBSERVED — PIN the outcome into the durable
              // home row (first-wins, idempotent) instead of deleting it. This
              // notifier is non-autoDispose: it can observe a terminal while the
              // user is at wallet root, and the pre-#367 dismiss-here made the row
              // vanish with the outcome never RENDERED (process death in the gap
              // erased a Refunded/Failed durably). The DELETE is now user-intent
              // only: the terminal card's Done / the home row's Remove.
              _pinOutcome(session, status);
            }
          },
          onError: (Object error, StackTrace stack) {
            // The core NEVER routes a transient transport fault here (it retries
            // internally), so this is an establish-time typed failure (`SwapDisabled`
            // / `SwapStateUnavailable` / a bad id) — persistent. Surface it and STOP;
            // do not reconnect a swap that structurally can't be tracked. The
            // retention seed is dropped too (review NIT): a later
            // same-session rebuild must re-render this error's ground, never
            // re-seed the stale pre-error status over it.
            _sub = null;
            _done = true;
            _lastEmitted = null;
            state = AsyncValue<SwapStatus>.error(error, stack);
          },
          onDone: () {
            // EOF from the core is ALWAYS intentional (it retries transport faults
            // internally, never EOFing on one): either a terminal status was just
            // emitted (`_done` already set), or a Hard kill / teardown ended the poll.
            // STOP either way — no reconnect. On a non-terminal end the last status
            // stays visible and the host renders "tracking unavailable" from its own
            // kill state (§3.5).
            _sub = null;
            _done = true;
          },
          cancelOnError: true,
        );
  }

  void _pause() {
    // Cancelling the Dart subscription makes the core's next `sink.emit` return
    // false ⇒ it ends with `cancelled` (NOT `onDone` on the Dart side — a cancel
    // is silent), so `_done` stays false and resume re-opens. Keep `state` (the
    // last status) visible for a glance on resume.
    _sub?.cancel();
    _sub = null;
  }

  void _resume() {
    if (_done) return; // a finished / killed swap is not re-opened on resume
    final session = ref.read(walletSessionProvider);
    if (session == null) return;
    // review hardening: if the session flipped while paused, the element
    // is already dirty (the watched provider changed) and its rebuild will
    // subscribe under the new session with the retention correctly dropped —
    // subscribing HERE would be the one path that could write `_lastEmitted`
    // from a stream of a session other than `_lastSession`. Structurally
    // closed; the chain was practically unreachable anyway.
    if (!identical(session, _lastSession)) return;
    // The core re-emits the current status immediately on subscribe, so a resume
    // needs no separate cold read (the status IS the live state).
    _subscribe(session);
  }

  void _teardown() {
    _sub?.cancel();
    _sub = null;
  }

  /// Fire-and-forget PIN of the observed terminal into the durable record
  /// (#367). Display-only state, so failure is swallowed whole (the record
  /// self-lapses; §5.4 — the id is never logged, so no error detail either)
  /// and the home read is refreshed only on success (so the row flips from
  /// present-tense motion to its honest terminal line). Guarded against a
  /// torn-down provider: `ref` is untouched after an unmount.
  void _pinOutcome(WalletSession session, SwapStatus status) {
    final outcome = _outcomeOf(status);
    if (outcome == null) return;
    unawaited(
      session
          .recordSwapOutcome(swapId: _swapId, outcome: outcome)
          .then((_) {
            if (!ref.mounted) return;
            ref.invalidate(walletInFlightSwapsReadProvider);
          })
          .catchError((Object _) {
            // Swallowed: the row self-lapses at its own bound.
          }),
    );
  }

  /// The terminal predicate — the three outcomes that END a swap (§7;
  /// `SwapStatus::is_terminal` core-side). `Unknown` is NOT terminal: the core
  /// never emits it (a forward-compat DTO arm), and treating an unrecognized
  /// status as "done" would silently abandon a live swap.
  ///
  /// COUPLING CONTRACT with [_outcomeOf]: every arm that is terminal HERE must
  /// be either pinnable THERE or carry an explicit exclusion comment — a new
  /// terminal variant that latches `_done` while silently skipping the pin
  /// re-opens the erased-outcome class #367 closed.
  static bool _isTerminal(SwapStatus s) =>
      s is SwapStatus_Success ||
      s is SwapStatus_Refunded ||
      s is SwapStatus_Failed;

  /// The DURABLY PINNABLE outcome class of a terminal — a deliberate SUBSET of
  /// [_isTerminal]'s arms. The synthesized not-found terminal (the SDK's #367
  /// poll policy after consecutive provider 404s) is a HEURISTIC, not provider
  /// truth, and the durable pin is first-wins — pinning it would let a ≥5-poll
  /// 404 blip (a provider failover, a gateway misroute) assert an
  /// uncorrectable false "didn't complete" over a swap whose money is in
  /// motion (money-review HIGH). Unpinned, the row falls to the honest neutral
  /// past-window line and a later re-attach re-polls the provider — the REAL
  /// terminal, once observed, pins normally.
  static SwapOutcome? _outcomeOf(SwapStatus s) => switch (s) {
    SwapStatus_Success() => SwapOutcome.success,
    SwapStatus_Refunded() => SwapOutcome.refunded,
    SwapStatus_Failed(:final code) when code == SwapFailureCode.notFound =>
      null,
    SwapStatus_Failed() => SwapOutcome.failed,
    _ => null,
  };
}
