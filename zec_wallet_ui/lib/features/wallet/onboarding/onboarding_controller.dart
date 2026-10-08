import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'mnemonic_input.dart';
import 'onboarding_providers.dart';
import 'onboarding_state.dart';
import 'onboarding_store.dart';
import 'wallet_provisioner.dart';

/// The onboarding state machine (spec §3.2g iii-B). Drives the create →
/// back-up → confirm → active flow and, on every launch, the resume fork that
/// enforces the money-safety invariant: a provisioned-but-unconfirmed wallet
/// ALWAYS resumes into forced backup, never into a deposit-ready state.
///
/// `walletSessionProvider` derives from this — exposing a [WalletSession] to the
/// wallet surface ONLY in [OnboardingActive] — so the gate has a single home.
final onboardingControllerProvider =
    NotifierProvider<OnboardingController, OnboardingState>(
      OnboardingController.new,
    );

/// The result of [OnboardingController.rescanActiveWallet] — how a rescan
/// resolved, for the rescan UI controller to present honestly. The funds are
/// SAFE in every variant (ADR-0534: the seed seal + a COMPLETE data DB — prior
/// or rebuilt, see [failedRecovered] — are intact on every error path); these
/// distinguish only what the user sees next.
enum RescanOutcome {
  /// The data DB was rebuilt and the active gate swapped to the fresh session —
  /// sync re-subscribes from the lower birthday; the balance/history repopulate.
  success,

  /// The rescan FAILED but the wallet was RECOVERED by re-opening whatever
  /// data DB is durable on disk (no funds lost either way — #379 docs truth):
  /// on the common PRE-rename faults that is the prior wallet, unchanged; on
  /// a fault AFTER the rebuild's atomic rename (dir-fsync / cache reset /
  /// re-open) it is the REBUILT lower-birthday wallet — balance/history
  /// empty until the auto-restarted sync repopulates them (the core's
  /// `rescan_is_crash_safe` pins OLD-before-rename / NEW-after). The Dart
  /// side cannot tell the arms apart: the error kinds are identical, and
  /// comparing `birthdayHeight()` before/after would miss the
  /// own-birthday rescan target (heights equal on both arms) — an incomplete
  /// discriminator, which is why the fix is copy-only. The notice copy
  /// claims only funds-safety, never "unchanged". An honest, dismissable
  /// "couldn't rescan" cue — the wallet stays fully usable.
  failedRecovered,

  /// The rescan failed AND the recovery re-open also failed — the surface is now
  /// [OnboardingFailed] (its own retry path), so the rescan UI shows nothing.
  failedClosed,

  /// The rescan was REFUSED by the SDK's witness-inversion fence (§4.4
  /// W-swap-4-a-3, `RescanWithInFlightSend`): a send — queued-outbox,
  /// swap-deposit, or (since S8, ADR-0556) any signed single-step send the
  /// wallet still owes: unmined and unexpired — is broadcast or awaiting
  /// broadcast but not yet settled, and rebuilding the data DB under it could
  /// re-sign the same payment over different notes (2× pay).
  /// The wallet was recovered by re-opening exactly like [failedRecovered];
  /// this DISTINCT outcome exists because the honest cue differs by hours:
  /// "a payment is still settling — try again in a couple of hours, keep the
  /// app online" (resolution needs sync passes to observe the mine/expiry),
  /// never the generic "try again in a moment".
  blockedBySettlingSend,

  /// The rescan failed for LACK OF DISK SPACE — the SDK's typed `DiskFull`
  /// (the W-swap-4-a-5 honest rescan fold; #375, UX HIGH F1). The rebuild
  /// + WAL fold need headroom that routine small commits don't, so the sync
  /// badge stays HEALTHY while every retry deterministically re-fails — the
  /// generic "try again in a moment" would be a lie-loop, and no other surface
  /// supplies the "free up space" truth. The wallet was recovered by
  /// re-opening exactly like [failedRecovered]; this DISTINCT outcome exists
  /// so the notice can say the one actionable thing: free up space, then retry.
  failedNeedsSpace,

  /// A no-op: either called from a non-active state (a stray/raced call) OR
  /// another lifecycle mutation — a rescan, a switch, a delete — is ALREADY in
  /// flight (the controller's one operation generation; a double-tap that the
  /// WalletRescanController's own running-guard normally catches first). Both
  /// map to "nothing changed" for the UI, so they share one outcome; the
  /// in-flight arm is the single-flight guarantee (never two rebuilds on one DB).
  notActive,
}

/// The SDK's rescan fence, by KIND (never message text): a typed
/// `RescanWithInFlightSend` refusal — see [RescanOutcome.blockedBySettlingSend].
bool _isInFlightSendRefusal(Object error) =>
    error is WalletApiError &&
    error.kind is WalletErrorKind_RescanWithInFlightSend;

/// The SDK's honest disk-full fault, by KIND (never message text) — see
/// [RescanOutcome.failedNeedsSpace].
bool _isDiskFullFault(Object error) =>
    error is WalletApiError && error.kind is WalletErrorKind_DiskFull;

/// The result of [OnboardingController.switchSyncServer] — how a server
/// switch resolved (the picker, P3-13). Funds are SAFE in every variant: the
/// switch never touches the data DB, and every fault arm leaves either the
/// old session (a pre-swap refusal) or a re-opened one (a post-stop fault).
sealed class SwitchServerResult {
  const SwitchServerResult();
}

/// The session was swapped onto the chosen server; the live-sync graph
/// rebuilds and sync restarts there.
class SwitchServerSuccess extends SwitchServerResult {
  const SwitchServerSuccess();
}

/// No active wallet, or another lifecycle mutation (a rescan, a delete, a
/// switch) is already in flight.
class SwitchServerNotActive extends SwitchServerResult {
  const SwitchServerNotActive();
}

/// The SDK refused BEFORE the swap — the server could not be reached, is on
/// another network, is not offered, the address is invalid, or the wallet is
/// busy in another phase. NOTHING changed: the session, the loop and the
/// remembered choice are as they were. [error] is the typed refusal, for the
/// picker's copy (by KIND, never by message text).
class SwitchServerRefused extends SwitchServerResult {
  const SwitchServerRefused(this.error);
  final WalletApiError error;
}

/// The switch failed PAST the stop-join and the wallet was recovered by
/// re-opening (the `rescanFrom` recover-by-reopen): usable, on whichever
/// server the re-open resolved — the new one if the choice's write landed,
/// the previous one otherwise. The picker says "couldn't switch" and shows the
/// server now in use.
///
/// Also the answer when the switch did not answer within
/// [OnboardingController.switchServerTimeout]: the switch's generation ends,
/// the session in the gate is left as it was, and a late answer is dropped.
class SwitchServerFailedRecovered extends SwitchServerResult {
  const SwitchServerFailedRecovered();
}

/// The switch failed AND the recovery re-open failed — the surface is now
/// [OnboardingFailed] (its own retry path); the picker shows nothing.
class SwitchServerFailedClosed extends SwitchServerResult {
  const SwitchServerFailedClosed();
}

/// A switch refusal the SDK issued BEFORE the swap (the handle stayed open —
/// `sync-server-picker.md` §3.1's `SwitchRefused { wallet: Some }` arm), by
/// KIND. A `walletBusy` carrying `switchingServer` is the one busy that means
/// the swap had begun (the quiesce timed out past the stop-join), so it is
/// NOT in this set.
bool _isPreSwapRefusal(Object error) =>
    error is WalletApiError &&
    switch (error.kind) {
      WalletErrorKind_SyncServerUnreachable() ||
      WalletErrorKind_NetworkMismatch() ||
      WalletErrorKind_SyncServerNotOffered() ||
      WalletErrorKind_InvalidEndpoint() ||
      // A user's key the door refused (ADR-0568) — before the probe.
      WalletErrorKind_InvalidEndpointAuth() ||
      // The probe under a Tor-required policy whose runtime is down.
      WalletErrorKind_Sync() ||
      // `require_open` on a wiped/closed handle — the core returned the
      // handle untouched; a re-open on top of a live handle would collide.
      WalletErrorKind_InvalidState() => true,
      WalletErrorKind_WalletBusy(:final phase) =>
        phase != LifecyclePhase.switchingServer,
      _ => false,
    };

/// The controller's own [OnboardingController.switchServerTimeout] expiry —
/// a private type, so a provisioner that throws a `TimeoutException` of its
/// own (past the stop-join, the handle closed) still takes the recover-by-
/// reopen arm.
final class _SwitchTimedOut implements Exception {
  const _SwitchTimedOut();
}

/// The result of [OnboardingController.deleteWallet] — how the crypto-shred
/// resolved, for the delete UI to present honestly.
enum WalletDeletionOutcome {
  /// The wallet was shredded (keychain custody severed + files removed) and the
  /// surface reset to [OnboardingWelcome] — the host navigates back to onboarding.
  shredded,

  /// The shred FAILED but the wallet was RECOVERED (the keychain-first wipe deletes
  /// nothing on a fault — the seals + files are intact — so the boot fork re-opened
  /// it). The wallet stays fully usable; the UI shows a dismissable "couldn't
  /// delete — try again", NEVER a false "deleted".
  failedRecovered,

  /// The shred failed AND the recovery re-open also failed — the surface is now
  /// [OnboardingFailed] (its own retry path), so the delete UI shows nothing.
  failedClosed,

  /// A refusal: called from a non-deletable state (no provisioned wallet) OR
  /// another lifecycle mutation — a server switch, a rescan, a deletion — is
  /// still in flight (the controller's one operation generation). Nothing was
  /// touched. A delete refused behind a server switch is accepted once the
  /// switch completes or its [OnboardingController.switchServerTimeout] ends it.
  notDeletable,
}

class OnboardingController extends Notifier<OnboardingState> {
  WalletProvisioner? _provisioner;
  OnboardingStore? _store;

  /// A pending async step touching `ref`/`state` after the provider is disposed
  /// throws; every continuation re-checks this first. The controller is
  /// root-scoped (lives for the app), so a dispose here means shutdown — but the
  /// guard keeps a late provisioning continuation from touching a torn-down ref.
  bool _disposed = false;

  /// THE ONE LIFECYCLE MUTATION POLICY (S2 §3.4, the review's M01). Every
  /// mutation this controller performs — create, confirm-backup, restore,
  /// watch-only import, retry, rescan, server switch, delete, force-clear —
  /// runs under this one counter:
  ///
  ///  * [_beginMutation] increments it at the mutation's start, to an ODD
  ///    value; an odd value IS "a mutation is in flight", so a second mutation
  ///    that starts meanwhile is refused (each method's typed no-op), never
  ///    raced. There is no second predicate beside it: the three per-verb
  ///    latches this replaced let a delete shred a wallet under a switch that
  ///    then landed a late `OnboardingActive` over it.
  ///  * every continuation re-checks it at each await's return ([_stale]); a
  ///    completion from a superseded generation is dropped — it writes no
  ///    state, whatever the provisioner (the package's or a host's) answered
  ///    and in whatever order.
  ///  * [_endMutation] moves it on to the next EVEN value when the mutation
  ///    finishes — or when [switchServerTimeout] expires under a hung switch,
  ///    after which the switch's own late answer is stale.
  ///
  /// A reveal authorization is minted with this value ([operationGeneration])
  /// and voided by any change of it (`RevealGrant`). Deliberately NOT reset in
  /// build(): a build() re-key supersedes whatever the prior cycle had in
  /// flight ([_supersedeAll]), it never lets two cycles share a generation.
  int _operationGeneration = 0;

  /// The current operation generation — what a reveal authorization is bound
  /// to beside the wallet session instance (`RevealGrant`). Any lifecycle
  /// mutation's start or end changes it.
  int get operationGeneration => _operationGeneration;

  /// How long a server switch may go unanswered before its generation ends.
  /// The SDK bounds the switch's own steps — the reachability probe
  /// (`SYNC_SERVER_PROBE_TIMEOUT_SECS`, 15 s) and the quiesce of the sync loop
  /// (`QUIESCE_MAX`, 30 s) — and the session rebuild after them is local I/O
  /// (the provisioner's `defaultLocalIoBound`, 30 s); this sits above their sum
  /// with a margin. Past it the switch answers [SwitchServerFailedRecovered]
  /// with the session in the gate unchanged, a delete is accepted again, and
  /// the switch's late answer lands nowhere. A Dart timeout cannot cancel the
  /// Rust side: a delete accepted here meets the core's own gate on the handle.
  static const Duration switchServerTimeout = Duration(seconds: 90);

  bool get _mutationInFlight => _operationGeneration.isOdd;

  /// Start a lifecycle mutation: the new (odd) generation, or `null` when one
  /// is already in flight — the caller refuses with its typed no-op.
  int? _beginMutation() {
    if (_mutationInFlight) return null;
    return ++_operationGeneration;
  }

  /// End [generation]'s mutation — a no-op when it was already ended (by the
  /// switch timeout, or a build() re-key), so a late finally never ends a
  /// NEWER mutation.
  void _endMutation(int generation) {
    if (_operationGeneration == generation) _operationGeneration++;
  }

  /// End whatever is in flight and move to a fresh idle generation — build()'s
  /// re-key: a prior cycle's continuation must never write into this one.
  void _supersedeAll() {
    _operationGeneration += _operationGeneration.isOdd ? 1 : 2;
  }

  /// A continuation of [generation] must stop: the controller is disposed, or
  /// the generation it runs under is no longer the current one.
  bool _stale(int generation) =>
      _disposed || _operationGeneration != generation;

  /// Monotonic wallet-LIFE counter (#381 (c)) — see
  /// [OnboardingActive.identityEpoch]. Incremented via [_nextIdentityEpoch]
  /// on every transition into Active EXCEPT the two rescan sites (the swap
  /// and its fault-recovery reopen), which continue the same wallet's life
  /// with [_currentIdentityEpoch]. Extra increments are always SAFE (they
  /// only drop a retained last-known value → an honest cold load); a missed
  /// one is the cross-identity leak, which is why the rescan sites are the
  /// ONLY deliberate non-bumps. Deliberately NOT reset in build(): a build()
  /// re-key must never let two different wallet lives share an epoch value.
  int _identityEpoch = 0;

  int _nextIdentityEpoch() => ++_identityEpoch;

  /// Any run of whitespace (incl. newlines) or the invisible Unicode format
  /// characters a PDF/rich-text/email paste smuggles in — see [startWatchOnly]'s
  /// paste normalization (UX-L3). `\s` misses them all; the explicit set covers:
  /// U+00AD soft hyphen (the classic line-wrapped-PDF smuggle), U+200B–U+200D
  /// (ZW space/joiners), U+200E–U+200F (LRM/RLM), U+2060 word joiner,
  /// U+2066–U+2069 (bidi isolates), U+FEFF (BOM / ZW no-break space). NONE can
  /// appear in a valid bech32 key, so the strip is IDENTITY on a clean paste and
  /// only ever WIDENS acceptance of an otherwise-rejected paste — it can never
  /// mint validity (the Rust ~2⁻³² bech32 checksum is the real gate; a stripped
  /// key still fails-closed if it wasn't already valid).
  static final RegExp _whitespaceRun = RegExp(
    r'[\s\u00AD\u200B-\u200F\u2060\u2066-\u2069\uFEFF]+',
  );

  /// The epoch of the wallet life currently (or last) active — what the
  /// rescan swap re-uses so retention survives a SAME-wallet session swap.
  /// Reads the live Active state when present (the SSOT), falling back to
  /// the counter (identical by construction; the state read keeps this
  /// honest if a future edit ever re-keys the counter).
  int _currentIdentityEpoch() {
    final s = state;
    return s is OnboardingActive ? s.identityEpoch : _identityEpoch;
  }

  /// The WATCH-ONLY kind of the wallet currently (or last) active (#397 §3.7 P1a)
  /// — what the two rescan sites carry forward, since rebuilding the data DB can
  /// never change a wallet's kind. Reads the live Active state; `false` off any
  /// other state (there is no active wallet to carry a kind from, and the value
  /// is only consumed at the rescan sites, which run from Active).
  bool _currentIsWatchOnly() {
    final s = state;
    return s is OnboardingActive && s.isWatchOnly;
  }

  @override
  OnboardingState build() {
    // Uniform with the money controllers (#330/): a watched-dep change
    // re-runs build() on the SAME notifier, firing the prior cycle's onDispose
    // first — without this reset `_disposed` would stick true and wedge EVERY
    // later `_set` (here: the whole app's wallet root, not one screen).
    // Unreachable today (the seam providers below are non-reactive), so this
    // is a belt.
    _disposed = false;
    ref.onDispose(() => _disposed = true);
    // A re-run supersedes whatever the prior cycle had in flight: its
    // continuations see a newer generation and write nothing into this one.
    _supersedeAll();
    // Cached for the mutating actions to read. The two seam providers stay
    // override-only / non-reactive: riverpod REUSES this notifier instance on
    // a build() re-run (it does NOT spawn a fresh controller). The generation
    // above keeps a prior cycle's action from writing a stale
    // `OnboardingActive(oldSession)` into the new one, but the action's
    // provisioner call itself still ran against the old seam — wire any
    // emitting dependency to resolve BEFORE the override instead.
    _provisioner = ref.watch(walletProvisionerProvider);
    _store = ref.watch(onboardingStoreProvider);

    final provisioner = _provisioner;
    final store = _store;
    if (provisioner == null || store == null) {
      // No onboarding backend in this build — honest not-available. The wallet
      // surface renders not-set-up (no deposit invited).
      return const OnboardingUnavailable();
    }

    // Kick off the boot probe; build() is sync, so the initial state is the
    // transient loading and `_probe` sets the resolved state when it completes.
    // Intentionally fire-and-forget (it drives `state` via `_set`); `unawaited`
    // makes that explicit and future-proofs against a stricter unawaited lint.
    unawaited(_probe(provisioner, store, _operationGeneration));
    return const OnboardingLoading();
  }

  /// The boot/resume fork. Also re-run by [retry] after a transient failure
  /// (e.g. the device was locked, then unlocked), and by the delete/force-clear
  /// fault paths. [generation] is the one it runs under — the boot's idle
  /// generation, or the calling mutation's — and every await re-checks it.
  Future<void> _probe(
    WalletProvisioner provisioner,
    OnboardingStore store,
    int generation,
  ) async {
    try {
      final exists = await provisioner.walletExists();
      if (_stale(generation)) return;
      if (!exists) {
        _set(const OnboardingWelcome());
        return;
      }

      // A wallet IS on disk. Read the (host-UX) backup-confirmed flag FIRST —
      // it needs no open wallet — THEN open. Ordering matters for reliability:
      // if the flag read fails (a degraded shared_preferences channel), no
      // wallet handle has been opened yet, so the single-writer lock is never
      // acquired-then-orphaned (an opened-then-dropped handle would hold the
      // lock until GC and make a retry hit walletAlreadyOpen). open() failing
      // after a good flag read likewise leaves no handle to leak.
      final confirmed = await store.isBackupConfirmed();
      if (_stale(generation)) return;
      final session = await provisioner.open();
      if (_stale(generation)) return;

      // #397 P1a — capture the immutable wallet KIND for the chrome via a
      // bounded local read, fail-safe to false (the full-spend chrome; the SDK
      // refuses a watch-only spend typed regardless, so a wrong `false` only
      // shows an affordance that then refuses honestly, never a wrong spend).
      // Reading it HERE into OnboardingActive is what lets isWatchOnlyProvider
      // resolve the chrome SYNCHRONOUSLY on the first frame instead of flashing
      // spend affordances while an async read settles (the HIGH).
      // Bounded like the hasPriorActivity read below: an unreadable kind never
      // holds OnboardingLoading (which has no retry). Read for EVERY on-disk
      // wallet, not only a confirmed one, because the kind also ROUTES — see
      // the backup-gate bypass below (security-H3).
      var watchOnly = false;
      try {
        watchOnly = await session.isWatchOnly().timeout(
          const Duration(seconds: 10),
        );
        if (_stale(generation)) return;
      } catch (_) {
        if (_stale(generation)) return;
      }
      // #397 P1b — the account-less watch-only CRASH REMNANT. createWatchOnly
      // provisions the store Complete BEFORE it imports the UFVK account, so a
      // crash in that window leaves a watch-only store with no account. It
      // opens cleanly (walletExists + open both succeed) but is BROKEN —
      // Receive faults SeedRequired, background sync faults
      // ProvisioningIncomplete, export can't run — and would otherwise strand a
      // broken Active. A provisioned watch-only account ALWAYS carries a
      // birthday (set at UFVK import), so a null birthday on a watch-only
      // wallet is the DEFINITIVE "account missing" signal: it is returned only
      // account-less (a transient fault THROWS, handled as provisioned), so
      // this can never fire on a healthy wallet. No seed, no money — WIPE the
      // incomplete store (closing the handle, releasing the single-writer lock,
      // resetting walletExists→false so the re-import's preconditions hold) and
      // route to a clean re-import. This completes the interrupted setup the
      // same way the seed remnant is auto-resumed — backward here, because a
      // forward converge in place would break the "Welcome ⟹ no wallet on
      // disk" invariant the create/restore paths rely on. Boot runs
      // single-threaded (the UI shows OnboardingLoading — no concurrent
      // action), so the wipe needs no delete/rescan latch here.
      if (watchOnly) {
        int? birthday;
        var probed = false;
        try {
          birthday = await session.birthdayHeight().timeout(
            const Duration(seconds: 10),
          );
          probed = true;
          if (_stale(generation)) return;
        } catch (_) {
          // A transient birthday-read fault is NOT a missing account — treat
          // the wallet as provisioned (never wipe a wallet on a read blip); a
          // true remnant resurfaces next boot / on Receive, still recoverable
          // via a manual delete.
          if (_stale(generation)) return;
        }
        if (probed && birthday == null) {
          try {
            await provisioner.deleteWallet();
            if (_stale(generation)) return;
            try {
              await store.setBackupConfirmed(confirmed: false);
            } catch (_) {
              // Moot on an absent wallet — the next boot lands on Welcome.
            }
            if (_stale(generation)) return;
            _set(const OnboardingWatchOnlyInput());
            return;
          } catch (error) {
            if (_stale(generation)) return;
            // The wipe faulted (wedged keychain/disk) — surface the honest
            // failure; its Retry re-probes and re-attempts once the cause
            // clears, never a stranded broken Active.
            _set(OnboardingFailed(classifyOnboardingFailure(error)));
            return;
          }
        }
      }
      // MONEY-SAFETY INVARIANT: a provisioned-but-unconfirmed SEED wallet
      // resumes into FORCED backup — never active, never deposit-ready. A
      // crash between create and confirm (or a lost confirmation flag)
      // re-forces the backup; the gate only opens on a durable, explicit
      // confirm.
      //
      // WATCH-ONLY BYPASS (security-H3): the backup gate is a SEED
      // gate — a watch-only wallet has no phrase, so the forced-backup screen
      // is unrenderable for it (its reveal would fault SeedRequired) and the
      // prefs flag says nothing true about it. Route on the DB-authoritative
      // KIND instead: a healthy watch-only wallet goes straight to Active even
      // when the flag was lost (a reset/restored prefs file over an intact
      // wallet — the same silent-loss scenario the #356-F2 guard below exists
      // for). Fail-safe composition IN THE COMMON CASE: a wedged kind read
      // leaves `watchOnly == false`, and the flag — written `true` before
      // every watch-only create — still routes to Active. The one uncovered
      // corner: flag ALSO lost AND the kind read wedges on
      // the SAME boot ⇒ falls through to the forced-backup screen (Reveal
      // faults SeedRequired) — a within-session backup trap on a double fault,
      // no money at risk (watched funds are on-chain, the UFVK re-importable)
      // and it self-heals on relaunch (a succeeding kind read lands Active).
      // The durable fix is the identity-keyed never-cleared flag (#356).
      if (confirmed || watchOnly) {
        // A fresh life on every boot-probe open: within one process this arm
        // re-runs only via retry-from-Failed, where dropping any retained
        // money display is the conservative, honest choice.
        _set(
          OnboardingActive(
            session,
            identityEpoch: _nextIdentityEpoch(),
            isWatchOnly: watchOnly,
          ),
        );
        return;
      }
      // #356-F2 guard (security HIGH): before offering the forced-backup
      // screen — which now carries a destructive "Start over" escape — check
      // the wallet for signs of a PRIOR LIFE (it synced, or holds a balance).
      // A wallet can only be here WITH history if its confirmed flag was
      // silently lost over an intact DB (a reset prefs file); the escape must
      // not offer a fresh-create-flavored delete on a possibly-FUNDED wallet
      // that this screen renders balance-less. Local read only; an unreadable
      // snapshot fails SAFE to `true` (escape hidden → the pre-F2 posture:
      // re-confirm the backup, annoying but never dangerous).
      // BOUNDED (security LOW — the port's boundedness contract): this
      // is otherwise the boot path's only uncapped FFI await, and a wedged
      // read here would hold OnboardingLoading forever (which has no retry).
      // Timeout and throw both fail SAFE to suppressed. KNOWN LIMIT (
      // security MED): a rescan legitimately clears BOTH signals (the rebuild
      // resets last_synced and empties the balance), so a flag loss × a
      // mid-rescan kill can still read "no prior life" on a funded wallet —
      // the durable never-cleared flag (Rust DB / wipe-integrated sentinel,
      // #357) is the real fix; until then the hedged dialog copy carries it.
      var hasPriorActivity = true;
      try {
        final snapshot = await session.snapshot().timeout(
          const Duration(seconds: 10),
        );
        if (_stale(generation)) return;
        hasPriorActivity =
            snapshot.lastSynced != null || snapshot.balance.totalZat > 0;
      } catch (_) {
        if (_stale(generation)) return;
      }
      _set(
        OnboardingAwaitingBackup(session, hasPriorActivity: hasPriorActivity),
      );
    } catch (error) {
      if (_stale(generation)) return;
      _set(OnboardingFailed(classifyOnboardingFailure(error)));
    }
  }

  /// Start the CREATE flow from [OnboardingWelcome]. Generates + seals a fresh
  /// seed in Rust and moves to [OnboardingAwaitingBackup] (NOT active — the
  /// recovery phrase must be backed up and confirmed first). A failure surfaces
  /// as [OnboardingFailed] (the screen renders it with the right next step); no
  /// rethrow, the state IS the surface.
  Future<void> startCreate() async {
    if (state is! OnboardingWelcome) return; // only from welcome
    // Reaching OnboardingWelcome implies a non-null provisioner+store AND a
    // just-observed `walletExists() == false` (the probe sets Welcome only
    // then). Both are structural — but a release build strips `assert`, so a
    // bare `return` here would SILENTLY no-op if a future refactor broke the
    // invariant (invariant 10). Surface it honestly instead, exactly as
    // `retry()` does for the same missing-backend case.
    final provisioner = _provisioner;
    final store = _store;
    if (provisioner == null || store == null) {
      _set(const OnboardingUnavailable());
      return;
    }

    // RE-ENTRANCY INTERLOCK: this synchronous transition (BEFORE the first
    // await) is what makes a double-tap safe — a second startCreate sees
    // `state != Welcome` and no-ops, so a wallet is created AT MOST once (pinned
    // by `double-tap startCreate creates exactly ONE wallet`). It must stay
    // synchronous-before-the-first-await; a future edit that inserts an await
    // ahead of it — or the identity-keyed-flag restore/re-create path — needs an
    // explicit in-flight latch instead (manager-flagged). The operation
    // generation below is that latch, for every mutation.
    final generation = _beginMutation();
    if (generation == null) return;
    _set(const OnboardingGenerating());
    try {
      // CRASH-SAFE ORDERING: durably CLOSE the backup gate for the
      // about-to-exist wallet BEFORE it is written to disk. If a crash lands
      // anywhere after this, the next boot reads `confirmed == false` and
      // re-forces the backup; a stale `true` from a wiped prior wallet can't
      // leak the new one past the gate.
      //
      // SAFE ONLY because startCreate runs from OnboardingWelcome (no wallet on
      // disk yet): the false-write touches the gate for a wallet that does not
      // exist. A FUTURE re-create/restore entry point must NOT reset the flag
      // before confirming it is not clobbering an already-confirmed wallet
      // (else it could re-force backup of a funded wallet) — manager-flagged;
      // key the flag by wallet identity when that path lands.
      await store.setBackupConfirmed(confirmed: false);
      if (_stale(generation)) return;
      // The gate is closed above; the wallet is written ONLY if that persist
      // succeeded (sequential awaits). The single catch below covers BOTH the
      // gate-reset failure and the create-write failure, in that order — either
      // way nothing reaches OnboardingAwaitingBackup, so no ungated wallet is
      // surfaced.
      final session = await provisioner.createGenerated();
      if (_stale(generation)) return;
      _set(OnboardingAwaitingBackup(session));
    } catch (error) {
      if (_stale(generation)) return;
      _set(OnboardingFailed(classifyOnboardingFailure(error)));
    } finally {
      _endMutation(generation);
    }
  }

  /// Confirm the recovery-phrase backup from [OnboardingAwaitingBackup]. DURABLY
  /// persists the confirmation, THEN opens the gate ([OnboardingActive]). If the
  /// persist fails the gate stays CLOSED (back to awaiting-backup) and the error
  /// rethrows so the screen can surface "couldn't save — try again": the wallet
  /// must never become deposit-ready on a confirmation that didn't survive.
  Future<void> confirmBackup() async {
    final current = state;
    if (current is! OnboardingAwaitingBackup) return; // only from awaiting
    // MUTUAL EXCLUSION with every other mutation (security MED; the one
    // operation generation): deleteWallet makes NO synchronous state
    // transition before its first await, so the state guard above passes
    // while a shred is mid-flight — without this check a confirm could
    // persist `true` and open the gate over a wallet being wiped. The view's
    // `_saving`/`_deleting` flags interlock too, but the controller owns the
    // invariant (release strips asserts; invariant 10).
    if (_mutationInFlight) return;
    // OnboardingAwaitingBackup is reachable only with a non-null store (build()
    // returns Unavailable otherwise), so this is structural — but a release
    // build strips `assert`, so a bare `return` would leave a dead "I've backed
    // it up" button silently stranding the user (invariant 10). Surface it
    // honestly instead.
    final store = _store;
    if (store == null) {
      _set(const OnboardingUnavailable());
      return;
    }

    // RE-ENTRANCY INTERLOCK (as in startCreate): this synchronous transition is
    // the double-tap guard — a second confirmBackup sees `state != AwaitingBackup`
    // and no-ops, so the confirmation persists AT MOST once and the gate opens
    // once (pinned by `double-tap confirmBackup persists exactly ONCE`).
    final session = current.session;
    final priorActivity = current.hasPriorActivity;
    final generation = _beginMutation()!;
    _set(OnboardingConfirming(session, hasPriorActivity: priorActivity));
    try {
      await store.setBackupConfirmed(confirmed: true);
      if (_stale(generation)) return;
      // #390: a CREATED wallet has no restore-invisible swap funds, so the
      // post-restore note must never show here — record notApplicable (also
      // clobbers any prior wallet's residual state). AWAITED BEFORE the gate
      // opens (security/reliability MED): the `_set` below is what
      // rebuilds `deepScanRestoreNoteProvider`, so the store must already hold
      // this value or that build() could race the write and read a stale note.
      // The setter NEVER throws, so it cannot fail the gate; a silently-lost
      // write just leaves the note absent (the fail-safe direction).
      await store.setDeepScanRestoreNoteState(
        DeepScanRestoreNoteState.notApplicable,
      );
      if (_stale(generation)) return;
      // BELT (security MED): only open the gate over OUR confirming
      // transition. A concurrent teardown that raced past the latch (or any
      // future caller that re-keys the machine mid-persist) must not be
      // clobbered with an Active over a dead/foreign session. Placed AFTER both
      // persists so it guards the flip against a re-key during EITHER await.
      final settled = state;
      if (settled is! OnboardingConfirming ||
          !identical(settled.session, session)) {
        return;
      }
      // First activation of a just-created wallet — a new life. A create/confirm
      // wallet holds a seed, so it is definitionally spend-capable (#397 P1a).
      _set(
        OnboardingActive(
          session,
          identityEpoch: _nextIdentityEpoch(),
          isWatchOnly: false,
        ),
      );
    } catch (_) {
      if (_stale(generation)) return;
      // Gate stays closed: the confirmation did not persist, so the wallet is
      // not deposit-ready. Revert and let the screen prompt a retry — but
      // only over OUR transition (same belt as the success arm).
      final settled = state;
      if (settled is OnboardingConfirming &&
          identical(settled.session, session)) {
        _set(
          OnboardingAwaitingBackup(session, hasPriorActivity: priorActivity),
        );
      }
      rethrow;
    } finally {
      _endMutation(generation);
    }
  }

  /// Enter the RESTORE flow from [OnboardingWelcome] — show the words-entry
  /// screen ([OnboardingRestoreInput]). Guarded to Welcome (the only place
  /// restore is offered), so — exactly as in [startCreate] — a wallet is known
  /// absent on disk when this runs, which is what makes the
  /// gate-close-before-write in [startRestore] safe.
  void beginRestore() {
    if (state is! OnboardingWelcome) return; // only from welcome
    _set(const OnboardingRestoreInput());
  }

  /// Leave the restore screen back to [OnboardingWelcome] (the Back action).
  /// Guarded to the restore-input phase; a stray call from any other state is a
  /// no-op (precondition discipline as elsewhere).
  void cancelRestore() {
    if (state is! OnboardingRestoreInput) return; // only from the restore form
    _set(const OnboardingWelcome());
  }

  /// Restore a wallet from [mnemonicWords] (the recovery phrase) and — since the
  /// user demonstrably HOLDS the backup by typing it — go straight to
  /// deposit-ready [OnboardingActive], not the forced-backup state a fresh create
  /// lands in. [approximateCreationTime] optionally speeds the scan (the adapter
  /// floors the birthday from it; `null` ⇒ a full, money-safe scan).
  ///
  /// THE LOWERCASE CHOKEPOINT (a HARD money-reliability contract): every word is
  /// normalized (trim + lowercase, drop empties) HERE before it reaches the SDK —
  /// the single place that guarantees the host-UI contract regardless of caller
  /// (the audited `bip39` parser does NOT case-fold, so a correct backup typed
  /// with a soft-keyboard's autocapitalization would otherwise be rejected with a
  /// word index). The restore screen normalizes too (for the live word count);
  /// re-applying the idempotent normalizer here is the belt-and-suspenders that
  /// makes the contract structural.
  ///
  /// A FIXABLE input fault (a mistyped word, a too-recent birthday) returns to
  /// [OnboardingRestoreInput] with an inline message and the typed phrase intact
  /// (the screen stays mounted across the round-trip); a provisioning failure
  /// (device locked, no vault, disk full …) surfaces on the generic
  /// [OnboardingFailed] screen. No rethrow — the state IS the surface.
  Future<void> startRestore(
    List<String> mnemonicWords, {
    DateTime? approximateCreationTime,
  }) async {
    if (state is! OnboardingRestoreInput) return; // only from the restore form
    // OnboardingRestoreInput is reachable only via beginRestore from Welcome,
    // which implies a non-null provisioner+store — but a release build strips
    // `assert`, so a bare `return` would drop the user's typed phrase with no
    // feedback (invariant 10). Surface it honestly instead, as `retry()` does.
    final provisioner = _provisioner;
    final store = _store;
    if (provisioner == null || store == null) {
      _set(const OnboardingUnavailable());
      return;
    }

    // The normalize chokepoint — the hard host-UI contract enforced once, for
    // every caller (see the doc above).
    final words = mnemonicWords
        .map(normalizeMnemonicWord)
        .where((word) => word.isNotEmpty)
        .toList(growable: false);

    // SOLE-GUARD the empty case: an all-whitespace/empty phrase normalizes to []
    // and must never reach the SDK (no gate-close, no wasted round-trip). The
    // screen also gates word count, but the chokepoint owns the contract for
    // EVERY caller — surface it as the fixable invalid-phrase fault, staying on
    // the form. (Synchronous, before any await, so re-entrancy is unaffected.)
    if (words.isEmpty) {
      _set(const OnboardingRestoreInput(fault: RestoreInputFault.invalidWord));
      return;
    }

    // RE-ENTRANCY INTERLOCK (as in startCreate/confirmBackup): this synchronous
    // transition BEFORE the first await is the double-tap guard — a second
    // startRestore sees `state != RestoreInput` and no-ops, so a wallet is
    // restored AT MOST once.
    final generation = _beginMutation();
    if (generation == null) return;
    _set(const OnboardingRestoring());
    try {
      // CRASH-SAFE ORDERING (mirrors startCreate): durably CLOSE the backup gate
      // BEFORE the wallet is written. A crash anywhere before the confirm below
      // then re-forces a backup on next boot (safe — the user re-confirms with
      // the phrase they entered) rather than leaking an unconfirmed wallet past a
      // stale `true`. SAFE because `walletExists()` is false when startRestore runs
      // — either we entered from Welcome (no wallet was ever created) or from
      // `recoverByRestore` (the unreadable remnant was just wiped) — so the false
      // write touches a not-yet-existing wallet's gate, exactly as startCreate's does.
      await store.setBackupConfirmed(confirmed: false);
      if (_stale(generation)) return;
      final session = await provisioner.restore(
        words,
        approximateCreationTime: approximateCreationTime,
      );
      if (_stale(generation)) return;
      // The user typed the phrase ⇒ the backup is already held. Persist the
      // confirmation durably, THEN open the gate. On a persist failure the gate
      // stays CLOSED and we hold at AwaitingBackup (the wallet IS restored), so
      // the user re-confirms with the phrase — never deposit-ready on a
      // confirmation that didn't survive. Mirrors confirmBackup's revert.
      try {
        await store.setBackupConfirmed(confirmed: true);
        if (_stale(generation)) return;
        // #390: a RESTORED wallet may hold swap deposits/refunds past the
        // restore sweep's ceiling (the #387 residual) — arm the one-time
        // post-restore note (shown once the first catch-up completes). AWAITED
        // BEFORE the gate opens so the `_set` below (which rebuilds
        // deepScanRestoreNoteProvider) cannot race the write and read a stale
        // note. The setter NEVER throws, so it cannot fail the restore;
        // a silently-lost write just leaves the note absent (fail-safe).
        await store.setDeepScanRestoreNoteState(
          DeepScanRestoreNoteState.pending,
        );
        if (_stale(generation)) return;
        // BELT (security LOW — symmetry with confirmBackup): only open
        // the gate over OUR restoring transition; a build() re-key mid-persist
        // must never be clobbered with a stale-cycle Active. AFTER both
        // persists so it guards the flip against a re-key during either await.
        if (state is! OnboardingRestoring) return;
        // First activation of a just-restored wallet — a new life. A restore
        // supplies a seed, so it is spend-capable (#397 P1a).
        _set(
          OnboardingActive(
            session,
            identityEpoch: _nextIdentityEpoch(),
            isWatchOnly: false,
          ),
        );
      } catch (_) {
        if (_stale(generation)) return;
        if (state is! OnboardingRestoring) return;
        _set(OnboardingAwaitingBackup(session));
      }
    } catch (error) {
      if (_stale(generation)) return;
      final inputFault = classifyRestoreInputFault(error);
      if (inputFault != null) {
        _set(
          OnboardingRestoreInput(
            fault: inputFault.fault,
            invalidWordIndex: inputFault.wordIndex,
          ),
        );
      } else {
        _set(OnboardingFailed(classifyOnboardingFailure(error)));
      }
    } finally {
      _endMutation(generation);
    }
  }

  /// Enter the WATCH-ONLY import flow from [OnboardingWelcome] (#397 §3.7 D5) —
  /// show the viewing-key input screen ([OnboardingWatchOnlyInput]). Guarded to
  /// Welcome (the only place it's offered), so a wallet is known absent on disk
  /// when this runs — the same precondition that makes the
  /// gate-close-before-write in [startWatchOnly] safe.
  void beginWatchOnly() {
    if (state is! OnboardingWelcome) return; // only from welcome
    _set(const OnboardingWatchOnlyInput());
  }

  /// Leave the watch-only import screen back to [OnboardingWelcome] (the Back
  /// action). Guarded to the watch-only-input phase; a stray call is a no-op.
  void cancelWatchOnly() {
    if (state is! OnboardingWatchOnlyInput) return;
    _set(const OnboardingWelcome());
  }

  /// Create a WATCH-ONLY wallet from [ufvk] at [creationDate] (#397 §3.7 D2) and
  /// go straight to deposit-ready-shaped [OnboardingActive] — a watch-only
  /// wallet has NO seed and so NOTHING to back up (there is no
  /// [OnboardingAwaitingBackup] step; it cannot deposit-spend anyway, but the
  /// gate discipline is identical: a session reaches the UI iff state is
  /// Active). The birthday is REQUIRED here (a watch-only import has no lazy
  /// seed-path — the account is imported eagerly), converted from [creationDate]
  /// via the same conservative estimator restore uses.
  ///
  /// A FIXABLE input fault (a malformed / wrong-network key, a too-recent date)
  /// returns to [OnboardingWatchOnlyInput] with the pasted key intact; a
  /// provisioning failure surfaces on the generic [OnboardingFailed] screen. No
  /// rethrow — the state IS the surface.
  Future<void> startWatchOnly(
    String ufvk, {
    required DateTime creationDate,
  }) async {
    if (state is! OnboardingWatchOnlyInput) return; // only from the input form
    final provisioner = _provisioner;
    final store = _store;
    if (provisioner == null || store == null) {
      _set(const OnboardingUnavailable());
      return;
    }

    // Normalize at the ONE chokepoint every caller flows through — paste AND
    // the in-app scan — so both get identical treatment (F1: the case-fold
    // used to live only in the scan handler, so an uppercase PASTE of a valid
    // key was falsely rejected while the same key SCANNED was rescued; mirrors
    // the mnemonic lowercase chokepoint). Two folds, both value-preserving on a
    // valid key:
    //  1. strip ALL whitespace — edge AND internal (UX-L3): a key from a
    //     line-wrapped PDF/email; bech32 never contains whitespace.
    //  2. case-fold an ALL-uppercase key (QR alphanumeric-mode, or an uppercase
    //     copy): bech32 is single-case and the SDK expects lower, so folding an
    //     all-caps key is IDENTITY on the decoded value + checksum (three
    //     reviewers confirmed no retargeting); MIXED case is left for the
    //     SDK's honest typed reject.
    // Then sole-guard the empty/blank case synchronously (before any await, so
    // re-entrancy is unaffected): a blank paste normalizes to nothing and must
    // never reach the SDK.
    final stripped = ufvk.replaceAll(_whitespaceRun, '');
    final trimmed = stripped.toUpperCase() == stripped
        ? stripped.toLowerCase()
        : stripped;
    if (trimmed.isEmpty) {
      _set(
        const OnboardingWatchOnlyInput(
          fault: WatchOnlyInputFault.invalidViewingKey,
        ),
      );
      return;
    }

    // RE-ENTRANCY INTERLOCK (as in startRestore): this synchronous transition
    // BEFORE the first await is the double-tap guard — a second startWatchOnly
    // sees `state != WatchOnlyInput` and no-ops, so a wallet is created AT MOST
    // once.
    final generation = _beginMutation();
    if (generation == null) return;
    _set(const OnboardingCreatingWatchOnly());
    try {
      // Convert the required date to a birthday height via the same
      // conservative estimator (never past the date → never skips history).
      final birthdayHeight = provisioner.estimateBirthdayHeight(creationDate);
      // A watch-only wallet has NOTHING to back up — mark the backup confirmed
      // so a reboot's probe opens straight to Active (there is no phrase to
      // re-confirm). Written BEFORE createWatchOnly for the crash-safe ordering
      // (a crash before the create leaves no wallet; a crash after leaves a
      // confirmed watch-only wallet the probe opens cleanly).
      await store.setBackupConfirmed(confirmed: true);
      if (_stale(generation)) return;
      final session = await provisioner.createWatchOnly(
        trimmed,
        birthdayHeight: birthdayHeight,
      );
      if (_stale(generation)) return;
      // #390 parity (defensive): a watch-only import — like a create — holds no
      // restore-invisible swap funds, so the one-time post-restore note must
      // never surface over it. startWatchOnly was the ONE activation path that
      // did NOT record this (confirmBackup + startRestore + deleteWallet all
      // write it), so a `pending` a prior restored wallet left in the GLOBAL
      // note key could leak onto the banner here. Clobber it notApplicable —
      // AWAITED before the gate opens (the rule: the `_set` below rebuilds
      // deepScanRestoreNoteProvider, so the store must already hold this value or
      // that build could race the write). The setter never throws, so it cannot
      // fail the import; a lost write just leaves the note absent (fail-safe).
      await store.setDeepScanRestoreNoteState(
        DeepScanRestoreNoteState.notApplicable,
      );
      if (_stale(generation)) return;
      // BELT: only open the gate over OUR creating transition.
      if (state is! OnboardingCreatingWatchOnly) return;
      // A watch-only import: the chrome key is KNOWN true here — no boot read
      // needed (#397 P1a), so the badge + hidden spend affordances render on the
      // first active frame.
      _set(
        OnboardingActive(
          session,
          identityEpoch: _nextIdentityEpoch(),
          isWatchOnly: true,
        ),
      );
    } catch (error) {
      if (_stale(generation)) return;
      final inputFault = classifyWatchOnlyInputFault(error);
      // DEFENSIVE (security-H3): the pre-create `true` write must not
      // outlive a FAILED create — reset it so a persisted `true` only ever
      // describes a wallet that EARNED it. UNCONDITIONAL, including on
      // `alreadyExists`: the flag is GLOBAL (#356 owns
      // the identity-keyed home), so on a cross-process race the sibling
      // wallet on disk may be an UNCONFIRMED SEED wallet — leaving our stale
      // `true` would boot it PAST the forced-backup gate (money-unsafe). The
      // cost of the unconditional reset is only re-forcing a CONFIRMED
      // sibling's backup (annoying, never dangerous) — the fail-safe
      // direction wins. Best-effort with a bound: fault-surfacing must never
      // hang on a wedged prefs channel (reliability F2) — a lost reset
      // re-opens only the proven-safe stale window (no wallet ⇒ the flag is
      // never read; any next create/restore rewrites `false` first).
      try {
        await store
            .setBackupConfirmed(confirmed: false)
            .timeout(const Duration(seconds: 10));
      } catch (_) {
        // Swallowed by the fail-safe direction above.
      }
      if (_stale(generation)) return;
      if (inputFault != null) {
        _set(OnboardingWatchOnlyInput(fault: inputFault));
      } else {
        _set(OnboardingFailed(classifyOnboardingFailure(error)));
      }
    } finally {
      _endMutation(generation);
    }
  }

  /// Retry after an [OnboardingFailed] — re-runs the boot probe (e.g. the user
  /// unlocked the device, freed disk, or closed the other instance). Guarded to
  /// the failed state ONLY: it is the Retry button's action, not a general
  /// re-probe. Re-probing from [OnboardingActive] would needlessly flash
  /// `Loading` and re-open the wallet, so a stray call from any other state is a
  /// no-op (matching the precondition discipline of [startCreate]/
  /// [confirmBackup]).
  Future<void> retry() async {
    if (state is! OnboardingFailed) return; // only from a failed state
    final provisioner = _provisioner;
    final store = _store;
    if (provisioner == null || store == null) {
      _set(const OnboardingUnavailable());
      return;
    }
    final generation = _beginMutation();
    if (generation == null) return;
    _set(const OnboardingLoading());
    // `_probe` re-checks the generation after each await (and `_set` is
    // guarded), so retry needs no post-`_probe` state write; a future edit that
    // adds one MUST re-check `_stale` here first — the same discipline as the
    // other actions.
    try {
      await _probe(provisioner, store, generation);
    } finally {
      _endMutation(generation);
    }
  }

  /// Rescan the ACTIVE wallet from an earlier birthday (ADR-0534 — recover funds
  /// an over-high restore birthday skipped). Guarded to [OnboardingActive] (a
  /// rescan is meaningful only on a deposit-ready wallet) and single-flighted by
  /// the operation generation. The money-recovery contract:
  ///
  ///  - SUCCESS: the provisioner rebuilds the data DB and returns a FRESH session;
  ///    we swap it into [OnboardingActive] so the live-sync graph re-subscribes
  ///    from the lower birthday (the balance/history repopulate as it scans).
  ///  - RESCAN FAULT: the handle is left closed (ADR-0534: the seed seal + a
  ///    COMPLETE data DB are intact), so we RECOVER by re-opening — back to a
  ///    usable [OnboardingActive] at the PRIOR birthday on pre-rename faults,
  ///    or at the REBUILT lower birthday on the post-rename arms (no funds
  ///    lost either way — see [RescanOutcome.failedRecovered]). The gate NEVER
  ///    closes on a recoverable rescan fault.
  ///  - RESCAN FAULT + RE-OPEN FAULT: genuinely unreachable now — route to
  ///    [OnboardingFailed] (its retry re-runs the boot probe).
  ///
  /// The gate stays OPEN throughout (never a flash of not-set-up): the rebuild is
  /// brief + local, the rescan UI shows an honest "rebuilding" progress, and the
  /// money-safety invariant is unaffected — the wallet was already backup-confirmed
  /// (rescan touches neither the seed nor the confirmation flag). Returns a
  /// [RescanOutcome] for the rescan UI controller to present; never rethrows (the
  /// state — or the returned outcome — IS the surface).
  Future<RescanOutcome> rescanActiveWallet(RescanTarget target) async {
    // Mutually exclusive with every other mutation (all mutate the wallet
    // handle): a rescan must never start while a delete is closing/wiping the
    // handle or a switch is swapping it, or its recovered/rebuilt session would
    // race that mutation's state write.
    if (state is! OnboardingActive || _mutationInFlight) {
      return RescanOutcome.notActive;
    }
    final provisioner = _provisioner;
    if (provisioner == null) {
      // Structurally unreachable (Active implies a non-null provisioner), but a
      // release build strips `assert` — surface it honestly rather than no-op a
      // money action (invariant 10).
      _set(const OnboardingUnavailable());
      return RescanOutcome.failedClosed;
    }

    final generation = _beginMutation()!;
    try {
      try {
        final session = await provisioner.rescanFrom(target);
        if (_stale(generation)) return RescanOutcome.notActive;
        // Fresh session identity ⇒ walletSessionProvider re-derives ⇒ the whole
        // live-sync graph rebuilds (sync auto-restarts from the lower birthday).
        // SAME identityEpoch (#381 (c)): this is the SAME wallet's life
        // continuing over a rebuilt DB — the money surfaces keep their
        // last-known values through the swap (no blank frame mid-rescan). The
        // kind is carried forward too (#397 P1a): a rebuild cannot change a
        // wallet's watch-only-ness (a watch-only rescan is refused by the SDK +
        // hidden by the chrome anyway, so this is normally unreachable).
        _set(
          OnboardingActive(
            session,
            identityEpoch: _currentIdentityEpoch(),
            isWatchOnly: _currentIsWatchOnly(),
          ),
        );
        return RescanOutcome.success;
      } catch (rescanError) {
        if (_stale(generation)) return RescanOutcome.notActive;
        // The rescan threw → the handle is closed (ADR-0534). Recover by
        // re-opening from the intact seed seal + whatever data DB is durable:
        // the PRIOR wallet on pre-rename faults, the REBUILT lower-birthday
        // wallet if the fault hit AFTER the atomic rename — fully usable
        // either way; see [RescanOutcome.failedRecovered] for the honest
        // two-arm story (#379). (`open()` is the same
        // local-only bounded call the boot fork uses; it WAITS OUT a
        // transiently-held single-writer lock — v-5c finding #2, the orphaned
        // scan batch that outlives the failed rescan — so this recovery no
        // longer collides into `WalletAlreadyOpen` while the wallet quiesces.)
        try {
          final reopened = await provisioner.open();
          if (_stale(generation)) return RescanOutcome.notActive;
          // SAME identityEpoch, as on the success arm: the recovery reopens
          // the SAME wallet (prior or rebuilt DB — ADR-0534), so the money
          // surfaces keep their last-known values through the reopen. Kind
          // carried forward too (#397 P1a — a rebuild can't change it).
          _set(
            OnboardingActive(
              reopened,
              identityEpoch: _currentIdentityEpoch(),
              isWatchOnly: _currentIsWatchOnly(),
            ),
          );
          // Same recovery either way; the OUTCOME distinguishes the fence (an
          // hours-scale, protective wait) and a full disk (a deterministic
          // refusal only the user can clear) from a generic fault, so the
          // rescan UI never renders "try again in a moment" over a refusal
          // that will hold (+ review folds). A typed `StoreBusy`
          // deliberately FOLDS into failedRecovered: it is the one rescan
          // fault whose honest copy IS "try again in a moment" (transient
          // writer contention), so a distinct outcome would render identical
          // UI — the taxonomy is explicit here and pinned by test.
          if (_isInFlightSendRefusal(rescanError)) {
            return RescanOutcome.blockedBySettlingSend;
          }
          return _isDiskFullFault(rescanError)
              ? RescanOutcome.failedNeedsSpace
              : RescanOutcome.failedRecovered;
        } catch (openError) {
          if (_stale(generation)) return RescanOutcome.notActive;
          // Both the rescan and the recovery re-open failed — the wallet is
          // genuinely unreachable right now. Route to the failed surface (its
          // retry re-runs the boot probe once the cause clears, e.g. a relock).
          _set(OnboardingFailed(classifyOnboardingFailure(openError)));
          return RescanOutcome.failedClosed;
        }
      }
    } finally {
      // Always end the generation — including on the stale early-returns (a
      // `return` inside `try` still runs `finally`). Touching a plain field
      // after dispose is safe (it is not `ref`/`state`).
      _endMutation(generation);
    }
  }

  /// Switch the ACTIVE wallet onto [choice] (the picker, P3-13) — the
  /// [rescanActiveWallet] shape: the provisioner swaps the session in place
  /// and a FRESH session identity goes into the gate, so the live-sync graph
  /// rebuilds and sync restarts on the new server. SAME identityEpoch (the
  /// same wallet's life, over the same DB) and kind carried, as for a rescan.
  ///
  /// Three fault arms, honestly distinct (see [SwitchServerResult]): a
  /// pre-swap REFUSAL leaves everything as it was (the SDK handed the handle
  /// back — no re-open, no state write); a post-stop fault is recovered by
  /// re-opening exactly like a rescan fault; a failed recovery routes to the
  /// failed surface. Single-flighted with every other mutation (the operation
  /// generation): a delete while the switch is in flight is refused, not raced.
  ///
  /// BOUNDED by [switchServerTimeout]: a provisioner that never answers ends
  /// the switch's generation there — [SwitchServerFailedRecovered], the session
  /// in the gate unchanged, a delete accepted again — and its late answer,
  /// success or fault, lands nowhere.
  Future<SwitchServerResult> switchSyncServer(SyncServerChoice choice) async {
    if (state is! OnboardingActive || _mutationInFlight) {
      return const SwitchServerNotActive();
    }
    final provisioner = _provisioner;
    if (provisioner == null) {
      _set(const OnboardingUnavailable());
      return const SwitchServerFailedClosed();
    }
    final generation = _beginMutation()!;
    try {
      try {
        final session = await provisioner
            .switchSyncServer(choice)
            .timeout(
              switchServerTimeout,
              onTimeout: () => throw const _SwitchTimedOut(),
            );
        if (_stale(generation)) return const SwitchServerNotActive();
        _set(
          OnboardingActive(
            session,
            identityEpoch: _currentIdentityEpoch(),
            isWatchOnly: _currentIsWatchOnly(),
          ),
        );
        return const SwitchServerSuccess();
      } on _SwitchTimedOut {
        // The switch never answered. Its generation ends in the finally below,
        // so its late answer is stale; nothing is re-opened over a handle the
        // Rust side may still hold, and the session in the gate stays as it
        // was. The picker reads the server now in use, as for a recovery.
        return const SwitchServerFailedRecovered();
      } catch (switchError) {
        if (_stale(generation)) return const SwitchServerNotActive();
        if (switchError is WalletApiError && _isPreSwapRefusal(switchError)) {
          // The handle is still open and untouched: the session in the gate
          // is the live one. Report, change nothing.
          return SwitchServerRefused(switchError);
        }
        // Past the stop-join the handle is closed (the rescan contract):
        // recover by re-opening — the choice is honoured if its write landed.
        try {
          final reopened = await provisioner.open();
          if (_stale(generation)) return const SwitchServerNotActive();
          _set(
            OnboardingActive(
              reopened,
              identityEpoch: _currentIdentityEpoch(),
              isWatchOnly: _currentIsWatchOnly(),
            ),
          );
          return const SwitchServerFailedRecovered();
        } catch (openError) {
          if (_stale(generation)) return const SwitchServerNotActive();
          _set(OnboardingFailed(classifyOnboardingFailure(openError)));
          return const SwitchServerFailedClosed();
        }
      }
    } finally {
      _endMutation(generation);
    }
  }

  /// CRYPTO-SHRED the provisioned wallet (FR-14 — the "delete wallet" / "reset"
  /// action). Guarded to a state that HOLDS a wallet ([OnboardingActive] or
  /// [OnboardingAwaitingBackup]) and single-flighted by the operation generation
  /// ([operationGeneration]) like every other lifecycle mutation. The
  /// destruction contract (mirrors [rescanActiveWallet]'s recover-by-reopen):
  ///
  ///  - SHREDDED: the provisioner closes the handle + severs the keychain custody
  ///    + removes the files; we reset to [OnboardingWelcome] (the wallet is gone)
  ///    and best-effort clear the host backup-confirmed flag. The host navigates
  ///    back to onboarding.
  ///  - SHRED FAULT: the keychain-FIRST wipe deletes NOTHING on a fault (device
  ///    locked / wedged keychain → the seals + files are intact), so we RECOVER by
  ///    re-running the boot fork — back to a usable [OnboardingActive]/
  ///    [OnboardingAwaitingBackup]. NEVER a false "deleted" over a live wallet.
  ///  - SHRED FAULT + RE-OPEN FAULT: route to [OnboardingFailed] (its retry
  ///    re-probes once the cause clears, e.g. a relock).
  ///
  /// Returns a [WalletDeletionOutcome]; never rethrows (the state — or the
  /// returned outcome — IS the surface). The seed is NEVER touched by a fault
  /// path: recovery is from the intact sealed seed, so funds are safe in every
  /// variant short of an explicit successful shred the user confirmed.
  Future<WalletDeletionOutcome> deleteWallet() async {
    final current = state;
    // Deletable only when a wallet is provisioned (a session is held). A stray
    // call from welcome/loading/failed is a no-op.
    if (current is! OnboardingActive && current is! OnboardingAwaitingBackup) {
      return WalletDeletionOutcome.notDeletable;
    }
    // Mutually exclusive with every other mutation (M01): a rescan or a server
    // switch runs WHILE the state stays Active (the gate never closes
    // mid-rebuild), so the state check above can't catch one in flight —
    // without this check a delete could close the handle under it and its
    // late `_set(OnboardingActive(session))` would clobber the post-delete
    // Welcome, stranding an Active state over a wiped wallet. REFUSED, not
    // queued: the caller awaits the other mutation and asks again (a hung
    // switch ends at [switchServerTimeout]).
    if (_mutationInFlight) {
      return WalletDeletionOutcome.notDeletable;
    }
    final provisioner = _provisioner;
    final store = _store;
    if (provisioner == null || store == null) {
      // Structurally unreachable (a wallet-holding state implies a non-null
      // backend) — surface honestly rather than no-op a destructive action.
      _set(const OnboardingUnavailable());
      return WalletDeletionOutcome.notDeletable;
    }

    final generation = _beginMutation()!;
    try {
      try {
        await provisioner.deleteWallet();
        if (_stale(generation)) return WalletDeletionOutcome.notDeletable;
        // Best-effort: clear the host backup-confirmed flag. The wallet is already
        // gone, so even if this write fails the next boot's `walletExists() ==
        // false` lands on Welcome regardless — NEVER fail the shred on a flag write
        // (and a fresh create re-sets the flag false before it writes a wallet).
        try {
          await store.setBackupConfirmed(confirmed: false);
        } catch (_) {
          // The shred already succeeded; the flag is moot on an absent wallet.
        }
        // #390 (security MED-1): also clear the post-restore note key, so a NEXT
        // create/restore can never inherit THIS wallet's `pending` and nudge a
        // no-op scan. The create arm's activation clobber is the primary guard;
        // this closes the residue window if that write is ever lost. Best-effort
        // (the setter never throws — see setDeepScanRestoreNoteState).
        await store.setDeepScanRestoreNoteState(
          DeepScanRestoreNoteState.notApplicable,
        );
        if (_stale(generation)) return WalletDeletionOutcome.notDeletable;
        _set(const OnboardingWelcome());
        return WalletDeletionOutcome.shredded;
      } catch (_) {
        if (_stale(generation)) return WalletDeletionOutcome.notDeletable;
        // The wipe threw → the handle may be closed, but the keychain-first
        // ordering means NOTHING was deleted (the seals + files are intact).
        // RECOVER by re-running the boot fork: it re-opens at the intact seal +
        // reads the unchanged backup flag, restoring Active/AwaitingBackup — or,
        // if the re-open ALSO fails (still locked), routes to OnboardingFailed.
        await _probe(provisioner, store, generation);
        if (_stale(generation)) return WalletDeletionOutcome.notDeletable;
        final settled = state;
        // The re-probe can land the wallet GONE rather than recovered: a wipe
        // that threw AFTER removing the wallet (walletExists → false ⇒ Welcome),
        // or the #397 P1b arm wiping an account-less watch-only remnant it
        // detected on the re-open (⇒ OnboardingWatchOnlyInput). Both mean the
        // wallet IS shredded — report it honestly rather than the misleading
        // "couldn't delete — still usable" (reliability MINOR-1). Only a genuine
        // recover-to-a-live-wallet (Active/AwaitingBackup) is failedRecovered.
        if (settled is OnboardingWelcome ||
            settled is OnboardingWatchOnlyInput) {
          return WalletDeletionOutcome.shredded;
        }
        return settled is OnboardingFailed
            ? WalletDeletionOutcome.failedClosed
            : WalletDeletionOutcome.failedRecovered;
      }
    } finally {
      _endMutation(generation);
    }
  }

  /// The #251 escape out of a NON-RETRYABLE [OnboardingFailed] (a `needsRecovery`
  /// wallet whose custody key is gone, so it can't be opened on this device).
  /// Force-clears the unreadable on-device remnant and routes to the restore form,
  /// where the user re-enters their recovery phrase and recovers their funds —
  /// which live ON-CHAIN, not on this device, so clearing the remnant loses no
  /// money. This is what guarantees a non-retryable failure is NEVER a dead-end.
  ///
  /// Honors the SDK `wipe_force` contract: try a plain delete FIRST and escalate
  /// to FORCE only for the `keystoreInconsistent` signal (the key is genuinely
  /// gone) — never auto-force any other fault. Any other error, or a force that
  /// itself fails, re-probes to a still-escapable failure (never a silent no-op,
  /// never a worse dead-end). Single-flight via the operation generation.
  Future<void> recoverByRestore() async {
    final current = state;
    // Guarded to the NON-retryable needsRecovery failure (the only state the view
    // offers this from). The kind gate is defense-in-depth: the force-wipe below is
    // legal ONLY for needsRecovery, so a future second caller can never reach it
    // from an unintended state (invariant 10 — release strips asserts).
    if (current is! OnboardingFailed ||
        current.kind != OnboardingFailureKind.needsRecovery) {
      return;
    }
    final provisioner = _provisioner;
    final store = _store;
    if (provisioner == null || store == null) {
      _set(const OnboardingUnavailable());
      return;
    }
    final generation = _beginMutation();
    if (generation == null) return; // single-flight, as deleteWallet
    try {
      try {
        // Plain wipe FIRST (never auto-force — the SDK contract). On a
        // `needsRecovery` wallet the custody key is gone, so this fails closed with
        // `keystoreInconsistent`; that — and ONLY that — authorizes the deliberate
        // force below (the user chose "restore"; the device is unlocked).
        await provisioner.deleteWallet();
      } on WalletApiError catch (e) {
        if (e.kind is WalletErrorKind_KeystoreInconsistent) {
          if (_stale(generation)) return;
          // The deliberate `wipe_force` is legal here ONLY because (1) the
          // provisioner uses ONE fixed config for open AND wipe — so a
          // `keystoreInconsistent` cannot be the SDK's mis-derived-dbDir ambiguity,
          // it can only mean the key is genuinely gone — and (2) this runs from an
          // interactive (therefore UNLOCKED) screen, so an SE wrap key can't be
          // merely-locked-and-invisible. Preserve BOTH invariants if this ever moves.
          await provisioner.forceDeleteWallet();
        } else {
          rethrow;
        }
      }
      if (_stale(generation)) return;
      // Best-effort clear the host backup flag (moot on an absent wallet), then go
      // to the restore form — the disk is now clean (walletExists == false), the
      // precondition the restore path relies on (same as entering from Welcome).
      try {
        await store.setBackupConfirmed(confirmed: false);
      } catch (_) {
        // The remnant is already gone; the flag is moot.
      }
      if (_stale(generation)) return;
      _set(const OnboardingRestoreInput());
    } catch (_) {
      if (_stale(generation)) return;
      // Even the force wipe failed (a genuine filesystem fault) — re-render the
      // honest failure via the boot probe. It re-opens the still-present remnant
      // and routes back to OnboardingFailed, where this same escape is offered
      // again: a retry loop with a way out, never a stuck dead-end screen.
      await _probe(provisioner, store, generation);
    } finally {
      _endMutation(generation);
    }
  }

  void _set(OnboardingState next) {
    if (_disposed) return;
    state = next;
  }
}
