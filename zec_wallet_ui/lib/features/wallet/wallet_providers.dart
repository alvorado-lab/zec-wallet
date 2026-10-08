import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';

import '../../core/lifecycle/app_lifecycle_provider.dart';
import 'onboarding/onboarding_controller.dart';
import 'onboarding/onboarding_providers.dart';
import 'onboarding/onboarding_state.dart';
import 'send_authorization.dart';
import 'sync_status_presentation.dart' show WalletHostTransport;
import 'wallet_session.dart';

/// The wallet session source — the money-safety gate's single home. A
/// deposit-capable [WalletSession] is exposed to the wallet surface ONLY when
/// onboarding is [OnboardingActive], i.e. AFTER the recovery-phrase backup is
/// confirmed-and-persisted. Every other phase (loading / welcome / generating /
/// awaiting-backup / confirming / failed / unavailable — including a
/// provisioned-but-unconfirmed wallet resumed after a crash) yields `null`, and
/// the wallet surface renders its honest not-set-up state: never invite a
/// deposit into a wallet whose seed the user has not backed up.
///
/// The whole live-sync graph below hangs off this seam. In this build the
/// onboarding backend is unwired (`walletProvisionerProvider`/
/// `onboardingStoreProvider` default `null` → [OnboardingUnavailable] → `null`
/// here), so production still shows not-set-up; the on-device slice overrides
/// those providers and this derivation goes live with zero rework. Tests
/// override THIS provider directly with a fake session.
///
/// THE SESSION-ONLY HOST CONFIGURATION (a SUPPORTED integration, not just a
/// test trick): a host with its OWN provisioning/custody model (e.g. a
/// host-supplied seed staged Rust-side, recovery via the host's own master
/// secret) overrides THIS provider with a derivation from its own gate —
/// yielding an `FrbWalletSession` over its handle when its wallet is ready,
/// `null` otherwise — and leaves `walletProvisionerProvider` unwired. The
/// active surface then renders fully; the affordances that need the package
/// provisioner (rescan, the Security screen) hide themselves. RESPONSIBILITY
/// TRANSFER: overriding this provider REPLACES the backup-confirmed gate — the
/// host's own gate must guarantee the wallet's funds are recoverable (its own
/// backup semantics) BEFORE it exposes a session, because every deposit/send
/// surface lights up the moment this is non-null.
final walletSessionProvider = Provider<WalletSession?>((ref) {
  final state = ref.watch(onboardingControllerProvider);
  return state is OnboardingActive ? state.session : null;
});

/// Is the active wallet WATCH-ONLY (#397 §3.7 D4/D5)? — the UI chrome key: a
/// "Watch-only" badge, hidden Send/Shield/Swap affordances, and the Security
/// screen's export-instead-of-backup arm read this. CHROME-ONLY by design: the
/// SDK's typed [WalletErrorKind.watchOnly] refusals stand regardless, so this
/// provider is allowed to fail SAFE — `false` (the full-spend chrome). A
/// spurious `false` only shows an affordance the SDK then refuses honestly
/// (never a wrong spend); a spurious `true` only hides an affordance.
///
/// SYNCHRONOUS on the package-gate path (the converged HIGH fix): the
/// wallet KIND is IMMUTABLE and is captured into [OnboardingActive.isWatchOnly]
/// at the moment the session becomes active (a watch-only import ⇒ known true; a
/// create/restore ⇒ known false; a boot open ⇒ the controller's bounded
/// once-read). Reading it from there resolves the chrome on the FIRST frame —
/// no flash of Send/Swap/Shield while an async FFI read settles (the old
/// `.value ?? false` painted the full-spend chrome on every watch-only mount
/// until the read landed). A SESSION-ONLY host (which overrides
/// `walletSessionProvider` and never reaches [OnboardingActive]) has no captured
/// kind to read, so it falls back to the bounded async once-read below — keyed
/// to the wallet identity so a watch-only `true` never bleeds onto the next
/// wallet.
final isWatchOnlyProvider = Provider<bool>((ref) {
  final session = ref.watch(walletSessionProvider);
  if (session == null) return false;
  final onboarding = ref.watch(onboardingControllerProvider);
  if (onboarding is OnboardingActive &&
      identical(onboarding.session, session)) {
    return onboarding.isWatchOnly;
  }
  // Session-only host: no captured kind — the bounded async fallback (fail-safe
  // false until it resolves; the SDK's typed refusals stand meanwhile).
  final identity = ref.watch(walletIdentityProvider);
  return ref.watch(_isWatchOnlyReadProvider(identity)).value ?? false;
});

final _isWatchOnlyReadProvider = FutureProvider.autoDispose.family<bool, Object?>(
  (ref, identity) async {
    // Dying-element guard (the walletSnapshotReadProvider idiom): a superseded
    // identity resolves to the safe default rather than reading a torn session.
    final superseded = ref.watch(walletIdentityProvider) != identity;
    final session = ref.watch(walletSessionProvider);
    if (superseded || session == null) return false;
    // Bound the local kind read (the shared FFI wedge bound) — the ONE FFI read
    // in this file that was missing the `.timeout` + no-silent-retry its ~8
    // siblings carry (#386/): a wedged read settles fail-safe `false`
    // instead of spinning through riverpod's ~10 silent background retries.
    return session.isWatchOnly().timeout(walletFfiWedgeTimeout);
  },
  retry: walletNoSilentRetry,
);

/// The wallet IDENTITY the money surfaces key their last-known values on
/// (#381 (c) — the converged HIGH). Riverpod retains an async
/// provider's previous value through a rebuild (`copyWithPrevious`), which is
/// exactly right for a transient re-read or the rescan's same-wallet session
/// swap — and exactly WRONG across a wallet identity change, where it painted
/// the DELETED wallet's balance, as-of stamp, Send gate, and parked/in-flight
/// amounts onto the NEXT wallet's first frames (persistently under a busy
/// first read), and on a duress/decoy multi-identity host flashed the OWNER's
/// balance on the DECOY's surface — the same leak class the settings store
/// got its identity fence for (A4/#348). The four money read providers below
/// drop their retained value whenever THIS value changes.
///
/// Derivation:
///  * no session ⇒ `null` — a change to/from null is an identity edge (the
///    #380 (c) wallet-gone set closes the gate, so a delete → create/restore
///    always crosses null even if intermediate states coalesce).
///  * the package gate owns the session (an [OnboardingActive] whose session
///    IS this session) ⇒ the state's [OnboardingActive.identityEpoch] — the
///    controller keeps it stable across a rescan's session swap (same
///    wallet, new handle ⇒ retention survives; NO blank frame mid-rescan)
///    and bumps it for every genuinely new wallet life.
///  * a SESSION-ONLY host (walletSessionProvider overridden; the package
///    gate not Active) ⇒ the session OBJECT itself. Every session flip is
///    then an identity change — exactly right for the duress/decoy
///    host, and conservatively right for a host that swaps handles over the
///    same wallet (it gets an honest cold reload instead of retention; a
///    host wanting same-wallet retention integrates via the package gate).
final walletIdentityProvider = Provider<Object?>((ref) {
  final session = ref.watch(walletSessionProvider);
  if (session == null) return null;
  final onboarding = ref.watch(onboardingControllerProvider);
  if (onboarding is OnboardingActive &&
      identical(onboarding.session, session)) {
    return onboarding.identityEpoch;
  }
  return session;
});

/// HOW THE FENCE IS BUILT (#381 (c)) — the read/view pair every money
/// provider below follows. Riverpod deliberately merges an async element's
/// previous value into every loading/error transition (`copyWithPrevious`
/// runs inside the element on EVERY single-state write — probe 8
/// re-proved there is no in-element way to hold "loading, no value" once
/// data has landed). So the fence uses riverpod's one native
/// element-per-key mechanism instead:
///
///  * a package-visible `…ReadProvider` — an `autoDispose.family` keyed by
///    [walletIdentityProvider]'s value. Each wallet identity gets its OWN
///    element, so retention (the `.value` last-known idiom across re-reads
///    and the rescan's same-identity session swap) lives strictly inside
///    one wallet's life, and the previous identity's element is DISPOSED
///    when the view re-keys away from it — the deleted wallet's figures are
///    released, not just hidden. TIMING CAVEAT: the re-key
///    runs when the view next FLUSHES; if no surface watches the views
///    across the identity flip (a host that unmounts every wallet screen
///    before a delete), the old element lingers deactivated in the heap
///    until the first money-surface mount re-keys it. Never RENDERABLE
///    (any read flushes transitively and re-keys first — probe-pinned);
///    memory hygiene only.
///  * the original-named view `Provider<AsyncValue<T>>` — what every
///    surface watches; it resolves the current identity's element. Its
///    read surface (`.value`/`.isLoading`/`hasError` via `ref.watch`) is
///    identical to the `FutureProvider` it replaced.
///
/// INVALIDATION CONTRACT: to force a re-read (the resume/action edges),
/// invalidate the `…ReadProvider` (family-wide — only the live identity's
/// element exists) — every action site in this package does. A manual
/// invalidate of the VIEW is FORWARDED to the current identity's reader
/// (`ref.invalidate(walletSnapshotProvider)` was the pre-#381
/// refresh idiom and remains valid host code — without the forward it
/// would compile and silently stop refreshing). Riverpod's own
/// `onManualInvalidation` doc blesses exactly this forwarding shape;
/// the member is @experimental at riverpod 3.3.2 — re-verify on upgrade.
/// Reader invalidates now re-read EAGERLY while any surface (or the
/// auto-shield listener) is live, rather than at the next widget read —
/// strictly fresher, mildly earlier FFI work.
///
/// WHY the view is a PUSH-fed sync [Notifier] and not a plain derived
/// `Provider`: a lazy relay defers the reader's recompute into whatever
/// read flushes it next — which after an action-edge invalidate is the NEXT
/// WIDGET BUILD, and a two-element recompute cascade inside a widget build
/// trips Flutter's build-phase asserts (found by the send-screen "Send
/// another" pin the moment the lazy shape landed). The sync notifier
/// `ref.listen`s the current identity's reader instead — the listen keeps
/// the autoDispose element alive — and a sync notifier's `state =` writes
/// are verbatim (`copyWithPrevious` merging lives only in async elements),
/// so the mirror adds no retention of its own.
///
/// The mirror write is DEFERRED one microtask: riverpod can deliver the
/// reader's transition in the middle of a widget-build flush (an ancestor
/// flushed by a widget's `watch` notifies eagerly), where a notifier write
/// is a modify-during-build assert. The IDENTITY EDGE never rides this
/// deferral — build() re-seeds synchronously from the new identity's
/// element, so no frame can render the previous wallet's money; only
/// same-identity async transitions (a read completing, an invalidate
/// reload) arrive a microtask later, which the async read surface already
/// implies.
abstract class _IdentityScopedView<T> extends Notifier<AsyncValue<T>> {
  @override
  AsyncValue<T> build() {
    final identity = ref.watch(walletIdentityProvider);
    final target = read(identity);
    // Legacy-refresh forwarding (see the invalidation contract above): a
    // manual invalidate of THIS view refreshes the current reader. The
    // registration is per-build, so it always carries the live target.
    // Experimental member: riverpod's own doc example is exactly this
    // forwarding shape; re-verify on upgrade.
    // ignore: experimental_member_use
    ref.onManualInvalidation(() => ref.invalidate(target));
    // Listen BEFORE the seed read: the subscription is what keeps the
    // autoDispose reader element alive from its very first frame.
    ref.listen(target, (_, next) {
      scheduleMicrotask(() {
        // The notifier may have been torn down — or re-keyed to a NEW
        // identity — while the write waited. `ref.mounted` covers teardown;
        // the identity re-check covers a re-key racing the deferral (the
        // stale identity's last transition must never overwrite the new
        // seed). A fast flip-back to the SAME session object passes the
        // re-check by design: a session-keyed element only ever carries its
        // OWN wallet's data, and the flip-back forces a fresh element
        // anyway (the old one disposed at the flip — probe-pinned).
        if (!ref.mounted) return;
        if (ref.read(walletIdentityProvider) != identity) return;
        state = next;
      });
    });
    return ref.read(target);
  }

  /// The identity-keyed reader element this view mirrors. Typed as the
  /// concrete [FutureProvider] (not a bare listenable) because the
  /// legacy-refresh forwarding must be able to INVALIDATE it too.
  FutureProvider<T> read(Object? identity);
}

/// The PRODUCTION per-tier custody disclosure (FR-14 H1) for the provisioned
/// wallet — the honest "are my keys hardware-backed / what does delete do"
/// answer the Security screen renders BEFORE a delete-wallet and as
/// a custody badge. A one-shot `autoDispose FutureProvider` so re-opening the
/// screen re-probes the live tier. Reads ONLY the measured vault tier (no seed,
/// no unseal, NO key material). Errors (no provisioner / a wedged keychain) are
/// surfaced honestly by the screen, never silently treated as "protected" —
/// and the retry pin is what makes "honestly" PROMPT: without it
/// the container default silently re-probed a failing keychain ~10× (~38 s of
/// spinner) before the probe-error card could land on the screen the user
/// reads BEFORE a delete-wallet. Re-opening the screen is the natural retry.
final walletCustodyDisclosureProvider =
    FutureProvider.autoDispose<CustodyDisclosure>((ref) {
      final provisioner = ref.watch(walletProvisionerProvider);
      if (provisioner == null) {
        // No onboarding backend in this build — the screen shows the unknown state.
        return Future<CustodyDisclosure>.error(
          StateError(
            'walletCustodyDisclosureProvider read with no provisioner',
          ),
        );
      }
      return provisioner.custodyDisclosure();
    }, retry: walletNoSilentRetry);

/// The on-resume cold snapshot (spec §3.3) — balance, Tor, tip, balance
/// age, `seq`. A one-shot read; [SyncStatusNotifier] invalidates it on a
/// real resume so a backgrounded UI re-reads cold state before trusting
/// live events again. Only watched when a session exists. The identity-
/// scoped READER behind [walletSnapshotProvider] — invalidate THIS to
/// force a re-read; watch the view.
///
/// DELIBERATE KEEP of the container default retry (the audit; contrast
/// [walletNoSilentRetry]): a transient FFI fault here self-heals in seconds
/// instead of waiting for the next sync edge, and the view's last-known
/// retention keeps the heal cycles invisible — the surface never regresses
/// below its retained truth while the retry runs. The guard `StateError`s
/// below are `Error`s, which the default never retries.
final walletSnapshotReadProvider = FutureProvider.autoDispose
    .family<WalletState, Object?>((ref, identity) {
      // DYING-ELEMENT guard (review, converged): at an identity flip
      // the OLD key's still-listened element is flushed once against the
      // NEW session before the view re-keys away and it disposes — without
      // this, that flush costs up to four discarded FFI reads at exactly
      // the busiest moment (and leaves a theoretical fast-flip-back
      // corner). A superseded key never reads.
      if (ref.watch(walletIdentityProvider) != identity) {
        return Future<WalletState>.error(
          StateError(
            'walletSnapshotReadProvider read under a superseded '
            'wallet identity',
          ),
        );
      }
      final session = ref.watch(walletSessionProvider);
      if (session == null) {
        // Defensive: the screen renders the not-provisioned state and never
        // reads this when the session is null. Surfaced as an error only if a
        // future caller forgets that branch.
        return Future<WalletState>.error(
          StateError('walletSnapshotProvider read with no wallet session'),
        );
      }
      return session.snapshot();
    });

/// The cold-snapshot VIEW every surface watches — identity-fenced (#381
/// (c), see the fence note above): last-known `.value` retention survives
/// re-reads and the rescan's same-wallet session swap, and can NEVER carry
/// a deleted/switched-away wallet's balance, as-of stamp, or Send gate
/// onto the next wallet's frames. To force a re-read, invalidate
/// [walletSnapshotReadProvider] — invalidating this view is a no-op.
final walletSnapshotProvider =
    NotifierProvider<WalletSnapshotViewNotifier, AsyncValue<WalletState>>(
      WalletSnapshotViewNotifier.new,
    );

/// See [walletSnapshotProvider].
class WalletSnapshotViewNotifier extends _IdentityScopedView<WalletState> {
  @override
  FutureProvider<WalletState> read(Object? identity) =>
      walletSnapshotReadProvider(identity);
}

/// The wallet's RECOVERABLE one-time-address (ephemeral) funds (2e-2b) — the
/// SUBSET of the transparent balance sitting on a wallet-controlled single-use
/// address (an expired TEX forward OR an exchange return). A one-shot
/// `FutureProvider`, invalidated on the SYNC balance-changing edges
/// (spendable/reachedTip in the `_WalletActive` listener) AND the resume path, so a
/// newly-surfaced amount appears without a manual pull. NOTE: this is a SUBSET of
/// [walletSnapshotProvider]'s edges — the user-action completions that also move
/// transparent funds (shield / move-to-transparent / swap / send) invalidate the
/// snapshot but NOT (yet) this provider, so the recoverable list can lag the
/// snapshot after such an action until the next sync edge. That lag is HARMLESS by
/// construction: the `_BalanceCard` render CLAMPS the displayed amount to
/// `transparentZat`, so a stale-high value can never claim more than the
/// transparent line it annotates (the subset invariant holds at the render).
/// Extending invalidation to those action-completion edges stays an optional
/// refinement (the lag is harmless by the clamp above); the LOAD-BEARING edges are
/// the sync/resume ones, where an on-chain strand actually surfaces (tx0 mines /
/// tx1 expires). Returns an EMPTY list when no session exists: the row is purely additive
/// INFORMATION, so its absence must never raise an error on the money surface
/// (unlike the snapshot, which the screen depends on). AMOUNT-ONLY — the one-time
/// address is wallet-internal and never crosses the bridge (§5.4 never-render).
/// LIVE since gate-removal (2e-2b-v-5a): a TEX two-step can now strand, so this
/// surface is load-bearing — empty only on a wallet that has stranded nothing.
/// DELIBERATE KEEP of the container default retry: the render
/// self-hides on failure (`.value ?? []`), so a silent heal is strictly better
/// than surfacing a transient blip on a purely-additive row.
/// The identity-scoped READER behind
/// [walletRecoverableEphemeralFundsProvider] — invalidate THIS; watch the view.
final walletRecoverableEphemeralFundsReadProvider = FutureProvider.autoDispose
    .family<List<RecoverableEphemeralFunds>, Object?>((ref, identity) {
      // Dying-element guard — see walletSnapshotReadProvider. Empty (the
      // null-session arm's shape): this surface never errors.
      final superseded = ref.watch(walletIdentityProvider) != identity;
      final session = ref.watch(walletSessionProvider);
      if (superseded || session == null) {
        return Future<List<RecoverableEphemeralFunds>>.value(
          const <RecoverableEphemeralFunds>[],
        );
      }
      return session.recoverableEphemeralFunds();
    });

/// The recoverable one-time-address funds VIEW — identity-fenced (#381 (c)).
final walletRecoverableEphemeralFundsProvider =
    NotifierProvider<
      WalletRecoverableEphemeralFundsViewNotifier,
      AsyncValue<List<RecoverableEphemeralFunds>>
    >(WalletRecoverableEphemeralFundsViewNotifier.new);

/// See [walletRecoverableEphemeralFundsProvider].
class WalletRecoverableEphemeralFundsViewNotifier
    extends _IdentityScopedView<List<RecoverableEphemeralFunds>> {
  @override
  FutureProvider<List<RecoverableEphemeralFunds>> read(Object? identity) =>
      walletRecoverableEphemeralFundsReadProvider(identity);
}

/// The queued sends PARKED by the multi-step gate (2e-2b-v-3/v-4) — TEX (ZIP-320)
/// sends sitting in the queue with NO on-chain transaction, so they appear NOWHERE
/// in the activity list. A one-shot `FutureProvider`, invalidated on the SAME
/// balance-changing edges as [walletSnapshotProvider] (the `_WalletActive` sync
/// listener + the resume path) AND explicitly after a cancel, so the "saved &
/// pending" surface stays current. Returns an EMPTY list when no session exists.
/// UNLIKE the recoverable subset (purely additive info that may swallow to empty),
/// a read FAILURE here PROPAGATES to `AsyncError`: a parked send is money the user
/// is waiting on, so the surface shows an honest "couldn't load" rather than a
/// silent-hide (the exact failure 2e-2b-v's visibility surfaces exist to foreclose).
/// AMOUNT-ONLY — the one-time recipient address never crosses the bridge (§5.4).
/// LIVE end-to-end since gate-removal (2e-2b-v-5a): both the interactive send and
/// `queueSend` paths can park a TEX here (empty only when none are parked).
/// RETRY PINNED OFF (audit; rationale CORRECTED by the review probe):
/// the section's `skipLoadingOnReload` already fell through to the honest
/// error arm after the FIRST failure even under the container default (a
/// retrying state carries `hasError`), so the pin does not change what a
/// cold-load failure SHOWS — it kills the up-to-ten wasted background FFI
/// retries cycling behind that line and settles the state honestly. The REAL
/// trade: a one-shot transient blip now leaves the error line standing until
/// the next invalidation edge (sync-edge/cancel/resume — this reader's own
/// retry cadence, typically ≤ ~95 s foreground) instead of self-healing in
/// ~450 ms; accepted for a single honest attempt + posture consistency with
/// the other pinned money readers.
/// The identity-scoped READER behind [walletParkedSendsProvider] —
/// invalidate THIS; watch the view.
final walletParkedSendsReadProvider = FutureProvider.autoDispose
    .family<List<ParkedSend>, Object?>((ref, identity) {
      // Dying-element guard — see walletSnapshotReadProvider. Empty (the
      // null-session arm's shape); a superseded element never reaches a
      // consumer, so the surface's error-propagation contract (a REAL read
      // failure must show "couldn't load") is untouched.
      final superseded = ref.watch(walletIdentityProvider) != identity;
      final session = ref.watch(walletSessionProvider);
      if (superseded || session == null) {
        return Future<List<ParkedSend>>.value(const <ParkedSend>[]);
      }
      return session.listParkedSends();
    }, retry: walletNoSilentRetry);

/// The parked-sends VIEW — identity-fenced (#381 (c)): a parked AMOUNT is
/// money display, so it must never carry across a wallet identity change.
final walletParkedSendsProvider =
    NotifierProvider<
      WalletParkedSendsViewNotifier,
      AsyncValue<List<ParkedSend>>
    >(WalletParkedSendsViewNotifier.new);

/// See [walletParkedSendsProvider].
class WalletParkedSendsViewNotifier
    extends _IdentityScopedView<List<ParkedSend>> {
  @override
  FutureProvider<List<ParkedSend>> read(Object? identity) =>
      walletParkedSendsReadProvider(identity);
}

/// The IN-FLIGHT two-step (TEX) sends (#309) — first leg broadcast, send not yet complete;
/// money in motion through a wallet-controlled one-time address. Drives the DURABLE
/// wallet-screen "on its way — don't send it again" cue that survives the dismissible
/// post-send result screen across the double-pay temptation window. Invalidated on the
/// SAME edges as [walletParkedSendsProvider] (the `_WalletActive` sync listener + resume)
/// so the cue appears after an interactive partial / a queued drain and CLEARS when the
/// chain completes, strands (→ the recoverable surface), or requeues the send (every tx
/// expired unmined → back to the parked surface). Returns EMPTY when no session exists. Additive
/// CAUTIONARY info like the recoverable subset (the same tx0 is independently visible as
/// a pending tx in the activity list, so a read failure hides no money — unlike parked,
/// where the surface is the ONLY witness). The render does NOT self-hide on a failure
/// (#308a, S2 §3.5d): it says it could not check and is retrying, because a vanished cue
/// reads as "nothing is mid-flight". The container default retry stays a DELIBERATE
/// KEEP: the retrying state carries `hasError`, so the line stands through
/// the retries and a transient blip heals on its own. Readers that only want the
/// rows (`.value ?? []`, the rescan sheet's advisory) stay advisory — the core's
/// fence refuses a rescan with money in motion. AMOUNT-only (§5.4 — the one-time
/// address never crosses the bridge).
/// The identity-scoped READER behind [walletInFlightSendsProvider] —
/// invalidate THIS; watch the view.
final walletInFlightSendsReadProvider = FutureProvider.autoDispose
    .family<List<InFlightSend>, Object?>((ref, identity) {
      // Dying-element guard — see walletSnapshotReadProvider.
      final superseded = ref.watch(walletIdentityProvider) != identity;
      final session = ref.watch(walletSessionProvider);
      if (superseded || session == null) {
        return Future<List<InFlightSend>>.value(const <InFlightSend>[]);
      }
      return session.listInFlightSends();
    });

/// The in-flight-sends VIEW — identity-fenced (#381 (c)) like the parked
/// surface.
final walletInFlightSendsProvider =
    NotifierProvider<
      WalletInFlightSendsViewNotifier,
      AsyncValue<List<InFlightSend>>
    >(WalletInFlightSendsViewNotifier.new);

/// See [walletInFlightSendsProvider].
class WalletInFlightSendsViewNotifier
    extends _IdentityScopedView<List<InFlightSend>> {
  @override
  FutureProvider<List<InFlightSend>> read(Object? identity) =>
      walletInFlightSendsReadProvider(identity);
}

/// The durable IN-FLIGHT SWAPS surface (W-swap-5, #366) — every swap whose
/// order was registered at execute and has neither been dismissed nor
/// self-lapsed. THE kill→relaunch re-attach: after a process restart the
/// wallet screen lists these and "view swap" re-opens live tracking by the
/// record's id — without this, an armed OutOfZec deposit was invisible
/// everywhere while the one-swap-in-flight guard refused new swaps (the
/// hardware photo). Invalidated on the SAME edges as the send surfaces (the
/// `_WalletActive` sync listener + resume) AND on a swap execute landing / a
/// record dismiss. Returns EMPTY when no session exists. Like the PARKED
/// surface, a read failure PROPAGATES to `AsyncError`: mid-flight this list
/// is the ONLY wallet-side witness of the swap (an OutOfZec deposit is
/// excluded from the parked/in-flight send surfaces by design; an IntoZec
/// swap has no activity trace until delivery), so the surface shows an
/// honest "couldn't load" rather than a silent hide. Deliberately NOT gated
/// on [swapEnabledProvider]: a local read is not swap traffic (§3.5) — the
/// user keeps sight of money in motion even with swap disabled or killed.
/// §5.4: the record id is render-never-log.
/// RETRY PINNED OFF: same shape and CORRECTED rationale as
/// [walletParkedSendsReadProvider] — the section's `skipLoadingOnReload`
/// already surfaced the error arm on the first failure; the pin kills the
/// wasted background retry cycling behind it and settles the state, at the
/// cost of the error line standing until the shared sync-edge/resume/dismiss
/// invalidation cadence (this reader's own retry) next fires.
/// The identity-scoped READER behind [walletInFlightSwapsProvider] —
/// invalidate THIS; watch the view.
final walletInFlightSwapsReadProvider = FutureProvider.autoDispose
    .family<List<SwapRecord>, Object?>((ref, identity) {
      // Dying-element guard — see walletSnapshotReadProvider.
      final superseded = ref.watch(walletIdentityProvider) != identity;
      final session = ref.watch(walletSessionProvider);
      if (superseded || session == null) {
        return Future<List<SwapRecord>>.value(const <SwapRecord>[]);
      }
      return session.listInFlightSwaps();
    }, retry: walletNoSilentRetry);

/// The in-flight-swaps VIEW — identity-fenced (#381 (c)): a live swap is
/// money in motion, so it must never carry across a wallet identity change.
final walletInFlightSwapsProvider =
    NotifierProvider<
      WalletInFlightSwapsViewNotifier,
      AsyncValue<List<SwapRecord>>
    >(WalletInFlightSwapsViewNotifier.new);

/// See [walletInFlightSwapsProvider].
class WalletInFlightSwapsViewNotifier
    extends _IdentityScopedView<List<SwapRecord>> {
  @override
  FutureProvider<List<SwapRecord>> read(Object? identity) =>
      walletInFlightSwapsReadProvider(identity);
}

/// HOST SEAM: the host's OWN transport claim for wallet traffic (maintainer
/// A host that routes ALL its traffic through its own privacy layer
/// (xray/vless, a VPN, its own Tor) overrides this so the wallet's network
/// indicator tells the truth — the SDK's `TorState` can only see its own
/// built-in Tor and would honestly read "Tor off" under a host tunnel. `null`
/// (the default) derives the indicator from the SDK's `TorState`. The claim is
/// the HOST's responsibility: `protection: true` earns the protected (green)
/// treatment, so only pass it for a transport that actually hides the user's
/// network identity from the server (§3.3 — never claim a protection that is
/// not running).
final walletHostTransportProvider = Provider<WalletHostTransport?>(
  (ref) => null,
);

/// HOST SEAM: the lightwalletd server HOST name shown on the sync sheet's
/// Connection section ("Server: zec.rocks") — the user should be
/// able to see which host the wallet talks to. `null` (the default) hides the
/// row. `walletOnboardingOverrides` wires it automatically from the
/// `WalletConfig.endpointUrl`; a session-only host overrides it alongside its
/// session. HOST only — never a full URL (a URL can carry userinfo/params that
/// don't belong on screen).
final walletEndpointHostProvider = Provider<String?>((ref) => null);

/// The sync servers the host OFFERS (the picker, P3-13) — read from the
/// session, re-read whenever the session identity changes (a switch swaps the
/// session). Empty when no session or nothing offered.
final walletSyncServersProvider = FutureProvider.autoDispose<List<SyncServer>>((
  ref,
) async {
  final session = ref.watch(walletSessionProvider);
  if (session == null) return const [];
  return session.syncServers();
});

/// Which server the wallet dials, and why (the picker, P3-13) — the
/// CONNECTION's own truth, read from the session and re-read when it swaps.
/// `null` with no session. The Server row and the picker render from this.
final walletSyncServerStatusProvider =
    FutureProvider.autoDispose<SyncServerStatus?>((ref) async {
      final session = ref.watch(walletSessionProvider);
      if (session == null) return null;
      return session.syncServerStatus();
    });

/// The HOST the sync sheet's Server row shows: the session's effective server
/// when a session exists (the connection's own truth — the no-drift rule
/// kept by reading the dial itself), else the host-config seam
/// [walletEndpointHostProvider] (the pre-session value, and a session-only
/// host's override). Host only, never the URL.
final walletEffectiveServerHostProvider = Provider.autoDispose<String?>((ref) {
  final status = ref.watch(walletSyncServerStatusProvider).value;
  final fromSession = status == null
      ? null
      : Uri.tryParse(status.effectiveUrl)?.host;
  if (fromSession != null && fromSession.isNotEmpty) return fromSession;
  return ref.watch(walletEndpointHostProvider);
});

/// HOST SEAM: an optional send-amount ceiling in zatoshis — e.g. an alpha
/// roll-out cap ("sends above 1 ZEC are disabled for now"). `null` (the
/// default) means NO ceiling. Enforced at the send form's chokepoints
/// ([SendController.prepare] and [SendController.queueOffline]) BEFORE any
/// compose/propose bridge call, surfacing as an honest inline
/// `SendOverCeiling` fault with the limit in the copy. POLICY, not safety:
/// the SDK's own range/funds validation is independent of this. Scope is the
/// interactive send + offline queue only — self-transfers (shield /
/// move-to-transparent) are wallet-internal and deliberately not covered.
/// The SWAP deposit IS bounded by this ceiling (FR-23, maintainer 2026-07-10): an
/// OutOfZec deposit whose ZEC side exceeds the cap is refused at quote-review
/// with the same over-ceiling fault, so every alpha money-OUT path stays under
/// one cap. (IntoZec moves no wallet funds — the user sends the deposit
/// externally — so it is unaffected.)
final walletSendCeilingZatProvider = Provider<int?>((ref) => null);

/// HOST SEAM: authorization around EVERY money-committing bridge call (#327 —
/// security review F1). The controllers route each signing/committing
/// action (interactive send, shield, move-to-transparent, ephemeral sweep,
/// offline queue, swap OutOfZec execute) through
/// [WalletSendAuthorizer.authorizeSpend] exactly once per user-confirmed
/// action. The default pass-through is correct for sealed-keychain custody; a
/// host with per-send credentials overrides this with its
/// prompt → unlock → sign → re-lock cycle and throws
/// [WalletSpendAuthorizationDenied] on cancel (the flow silently restores its
/// pre-confirm state — the proposal token stays unconsumed, no bridge call is
/// made). See [WalletSendAuthorizer] for the full contract, including the
/// deferred-signing caveat that pairs this seam with
/// [walletOfflineQueueSupportedProvider].
final walletSendAuthorizerProvider = Provider<WalletSendAuthorizer>(
  (ref) => const WalletPassthroughSendAuthorizer(),
);

/// HOST SEAM: whether the host's custody model supports the OFFLINE send
/// queue (#327). Queuing persists the intent NOW and SIGNS later — so a custody
/// model whose signing credential exists only inside an authorized window
/// (per-send passphrase/biometric) historically could not serve it at all, and
/// the send form must not offer "save for later" it cannot honour: override to
/// `false` and the affordance is HIDDEN (an honest absence beats a queue that
/// faults at drain). UX honesty, not a safety boundary — if a queue does slip
/// through on such custody, the drain fails typed, the send stays visibly
/// parked ("saved & pending") and cancellable; no funds move.
///
/// **A PROMPT-PER-SPEND HOST MAY NOW SET THIS `true` (FR-23-b / #361).** The
/// parked-sends surface offers "Send now", which signs a parked row inside your
/// `WalletSendAuthorizer.authorizeSpend` bracket
/// (`WalletSession.authorizeParkedSend`) — the queue drains at a user-present
/// moment instead of an unattended one. Set it true once you serve that call.
///
/// It stays a HOST-SET capability rather than one derived from "an authorizer is
/// wired" (maintainer decision D2, 2026-07-25): wiring the seam is not proof you can
/// STAGE a credential on demand, and this flag's whole job is the honesty rule
/// "never advertise a queue that cannot drain". You know your custody; the
/// package does not.
///
/// The swap deposit has a superficially similar deferred shape but its gate is
/// the swap enablement config, not this flag — and since FR-23-a it signs
/// in-bracket at execute, so it needs no queue support at all. The BACKGROUND
/// drain also pauses whenever the host's SYNC policy is off (it rides sync
/// passes), so the send form additionally gates the affordance on
/// [walletSyncPolicyProvider] (see its MONEY COUPLING note).
///
/// **WHAT "Send now" DOES AND DOES NOT ESCAPE (#400 R1 — this paragraph used to
/// overclaim).** The SIGNATURE is genuinely user-driven and owes nothing to the
/// pass schedule. The BROADCAST does not: the signed group goes to a detached
/// best-effort kick (bounded retry, then it gives up), and the durable re-send
/// behind it — `resubmit_queued_sends`'s `ReBroadcast` arm — runs only after a
/// COMPLETED SYNC PASS. Under a sync-off policy there are no passes, so a kick
/// that exhausts its retries leaves a signed, note-spending transaction that
/// nothing will re-send. The parked surface says so on the signed outcome
/// (`walletParkedAuthorizeSentSyncPaused`) rather than pretending otherwise. Sync
/// STATE matters too: a wallet whose tip cannot advance may be unable to build
/// the transaction at all, which surfaces as the honest "not ready to send yet",
/// never as a failure.
///
/// **"Send now" is NOT gated on this flag (#400 R9, deliberate).** The flag
/// governs the ENTRY — whether the send form offers to save a payment for later.
/// It does not govern the EXIT of a row that is already committed. A host can
/// flip it to `false` with parked rows already saved (or inherit them from a
/// restore), and a committed send must keep both of its exits — drain and
/// cancel — or the user's money has nowhere to go. On custody that truly cannot
/// stage a credential the attempt fails typed and honestly ("not ready to send
/// yet", the row untouched); that is strictly better than removing the only
/// non-destructive way out. A row that is already MID-SIGNATURE hides the button
/// for the different reason that the SDK verb would refuse it outright.
final walletOfflineQueueSupportedProvider = Provider<bool>((ref) => true);

/// HOST SEAM (#383 R1): whether the package may RUN the background sync loop.
/// Default `true` (sync just runs — real wallets have no Start button). A host
/// that gates syncing behind its own setting (a data-saver/privacy toggle, an
/// org policy) overrides this to `false`: the package then never issues
/// `startSync` — and the sync badge/sheet HONESTLY render "sync off — turn it
/// on in settings" instead of a stalled/connecting story that never resolves
/// (the honest-off posture; same honesty family as
/// [walletOfflineQueueSupportedProvider]). Reactive: flipping to `true`
/// starts the loop on the spot; flipping to `false` stops it (scan progress
/// is durable — resuming loses nothing). POLICY, not a privacy boundary: the
/// wallet still opens, balances show the last synced state, and Receive
/// still works (local derivation).
///
/// MONEY COUPLING: the offline-send queue, parked-send retries, and
/// in-flight two-step completions all advance ONLY on sync passes — under
/// policy-off they are FROZEN until the host re-enables sync. The package
/// stays honest about that (the send form offers no queue while the policy
/// is off, parked/in-flight surfaces state the pause, rescan and the swap
/// deep scan are disabled — they could never complete), but the HOST owns
/// the policy story: a host that ships policy-off long-term should also set
/// [walletOfflineQueueSupportedProvider] `false` so the queue never enters
/// the user's vocabulary at all.
final walletSyncPolicyProvider = Provider<bool>((ref) => true);

/// HOST SEAM (#383 R2): whether the host's custody model supports the
/// AUTO-SHIELD policy loop (#328). The loop's spends are `origin: automatic` —
/// no user is present at a prompt — so a per-spend-credential host (every
/// automatic spend is policy-denied) should override this to `false`: the
/// auto-shield toggle is then HIDDEN in the Transparent-funds sheet and the
/// loop never arms — an honest absence instead of a switch that reads ON but
/// can never run (the [walletOfflineQueueSupportedProvider] honesty rule).
/// UX honesty, not a safety boundary: even when `true` on such custody, every
/// attempt still routes through the authorizer and a denial stops the loop
/// for the session with the funds honestly visible.
final walletAutoShieldSupportedProvider = Provider<bool>((ref) => true);

/// The session's verdict on the last fully-synced tip — TRI-STATE (#317, the
/// wrap-review design). The balance header's "(as of block N[, time])"
/// resolves through this ([balanceAsOf]):
///
///  * [WalletSyncedTipUnset] — this session has NO verdict yet (cold launch,
///    offline before the first pass). The SDK's durable `lastSynced` stamp is
///    trusted for the header, so a relaunch renders the last good height with
///    no first-pass wait.
///  * [WalletSyncedTipLatched] — a tip this session PROVED (a reached-tip
///    emit — THE LATCH RULE below), held THROUGH routine re-scan windows
///    (the header must never flap back to "age unknown" on a
///    wallet that has synced; the flap was also a 64px layout shift at large
///    scales). Carries the moment the tip was observed so the header keeps a
///    PAIRED height+time while the async stamp re-read catches up (the
///    AsOfAt↔AsOf arm pulse fix).
///  * [WalletSyncedTipInvalidated] — this session has POSITIVE evidence the
///    last-synced claim is unsafe (a reorg REWIND, or a scan target below the
///    latch). Suppresses BOTH the latch and the persisted-stamp fallback
///    until the next completed pass on a CURRENT server — without this, the
///    live stamp would RESURRECT the exact height the rewind just invalidated
///    and pin the stamp's clock time to it for the whole re-scan window (the
///    wrap review's HIGH finding).
///
/// THE LATCH RULE (`production-readiness-phase-1.md` §4r U-1 — stated here
/// ONCE; each arm of [WalletSyncedTipNotifier._next] cites it):
///  * The four CURRENT-server reached-tip variants — `upToDate`,
///    `upToDateLimited`, `upToDateDegraded`, `upToDateUnverified` — are a
///    completed pass on a server whose tip the wallet has no reason to doubt.
///    Each latches its `tip` from ANY `cur`, `Invalidated` INCLUDED (the pass
///    scanned to it; a fresh proof is exactly what clears a rewind verdict),
///    under the REPLAY rule: re-affirming the same tip keeps its original
///    observation time. Before §4r only `upToDate` latched, and a wallet whose
///    first-ever completed pass was `upToDateLimited` or `upToDateDegraded`
///    rendered "as of block null" with the valid tip sitting in the status
///    (§4j row 10).
///  * `endpointBehind` — a completed pass on a server that is BEHIND the
///    chain — latches ONLY from `Unset`, or from a `Latched` whose tip is at
///    or below the new one (same replay rule). It never LOWERS a latch and it
///    never clears `Invalidated`: a forked or behind server must not
///    resurrect a height the rewind just invalidated, nor pull "as of block
///    N" down to the fork's height (§4m #4). What it does latch is honest —
///    that server's tip IS what the wallet is synced to — it just cannot
///    outrank what a current server or a rewind already proved.
///  * Everything else HOLDS the verdict (the `default` arm), except the
///    `scanning` samples the consumption rule below turns into `Invalidated`.
///
/// RESETS to unset on a session identity change (rescan swap / close — the
/// verdict belongs to the wallet DB this session reads). Session-local and
/// deliberately NOT persisted.
///
/// REWOUND CONSUMPTION RULE (`SyncStatus_Scanning.rewound`, first-class since
/// #317): it is a pass-scoped LEVEL, not an edge — every sample from the
/// first rewind through the end of that pass carries `true` (including after
/// the scan re-passes the old height), and it resets on the next pass. A
/// level-triggered invalidate is therefore idempotent and safe here; the
/// status channel is ordered, so a rewound sample can never land after (and
/// wrongly clear) a LATER `upToDate`'s latch. One-shot UX (a toast) must
/// edge-detect instead — do not reuse this level. The watch channel also
/// COALESCES: a rewind can be missed entirely under bursty emits, so this is
/// best-effort UI honesty, never accounting — which is why the `to < latched`
/// clear stays as a belt (a lower-tip server switch produces no rewind at
/// all).
final walletSyncedTipProvider =
    NotifierProvider<WalletSyncedTipNotifier, WalletSyncedTip>(
      WalletSyncedTipNotifier.new,
    );

/// See [walletSyncedTipProvider].
sealed class WalletSyncedTip {
  const WalletSyncedTip();
}

/// No verdict this session — the persisted stamp may render.
class WalletSyncedTipUnset extends WalletSyncedTip {
  const WalletSyncedTipUnset();
}

/// A tip this session proved reached (THE LATCH RULE on
/// [walletSyncedTipProvider]), with the moment it was observed.
class WalletSyncedTipLatched extends WalletSyncedTip {
  const WalletSyncedTipLatched(this.tip, this.at);

  /// The proven tip height.
  final int tip;

  /// When THIS SESSION observed the tip becoming up-to-date — the header's
  /// paired time while the durable stamp's async re-read is still in flight.
  /// Kept (not refreshed) when the same tip is re-affirmed, so a
  /// re-subscribe REPLAY after a background pause can never dress an old
  /// claim up as fresh (conservative under replays by construction,
  /// independent of the provider's notify semantics).
  final DateTime at;
}

/// Positive evidence the last-synced claim is unsafe — suppress the header's
/// height entirely (latch AND stamp) until the next completed pass on a
/// CURRENT server (THE LATCH RULE: a behind server cannot clear this).
class WalletSyncedTipInvalidated extends WalletSyncedTip {
  const WalletSyncedTipInvalidated();
}

class WalletSyncedTipNotifier extends Notifier<WalletSyncedTip> {
  @override
  WalletSyncedTip build() {
    // Session identity keys the latch (same rule as the whole live graph).
    ref.watch(walletSessionProvider);
    ref.listen<AsyncValue<SyncStatus>>(syncStatusProvider, (_, next) {
      final s = next.value;
      if (s != null) _apply(s);
    });
    // Seed from the current value so a rebuild mid-session (session swap
    // lands already-synced) doesn't wait for the next emit — LOAD-BEARING:
    // the latch is first BUILT when the card first renders, which can be
    // AFTER the stream's only UpToDate replay. The seed
    // mirrors the listener, so a latch first built MID-rewind (a cold launch
    // straight into a rescanning wallet) starts invalidated, not unset —
    // otherwise the stamp would render the orphaned height.
    final s = ref.read(syncStatusProvider).value;
    return s == null
        ? const WalletSyncedTipUnset()
        : _next(const WalletSyncedTipUnset(), s);
  }

  void _apply(SyncStatus s) => state = _next(state, s);

  /// The tri-state transition — pure so the seed and the listener cannot
  /// drift apart.
  WalletSyncedTip _next(WalletSyncedTip cur, SyncStatus s) {
    switch (s) {
      // THE LATCH RULE, current-server arm (the [walletSyncedTipProvider]
      // doc): a completed pass on a current server proves `tip` from ANY
      // `cur`, `Invalidated` included. ONE arm for all four (§4r U-1), so
      // the two older siblings can never again fall to `default` and render
      // "as of block null" after a first-ever limited/degraded pass (§4j
      // row 10). `UpToDateLimited`: this BUILD cannot read every block it
      // passed; `UpToDateDegraded`: this SERVER under-serves a pool;
      // `UpToDateUnverified` (GRACE-1): this server will not name its
      // network — three different axes, none of them "the server's tip is
      // not the chain's tip", so all three are as much a proof of `tip` as
      // the plain `UpToDate`.
      case SyncStatus_UpToDate(:final tip) ||
          SyncStatus_UpToDateLimited(:final tip) ||
          SyncStatus_UpToDateDegraded(:final tip) ||
          SyncStatus_UpToDateUnverified(:final tip):
        return _latch(cur, tip);
      // THE LATCH RULE, behind-server arm (§4m #4): T0-1c's completed pass
      // against a BEHIND server proved the wallet synced to `tip` — that
      // server's tip — so a first-ever behind pass renders "as of block N",
      // honestly, not "as of block null" (§4j row 10's shape). But it cannot
      // outrank what this session already proved: an `Invalidated` verdict
      // STANDS (only a current server's pass clears a rewind — a forked or
      // behind server must not resurrect the height the rewind just
      // invalidated), and a HIGHER latch STANDS (the fork's height must not
      // pull the header down). From `Unset`, or at/above the latch, the
      // replay rule applies as for the arm above.
      case SyncStatus_EndpointBehind(:final tip):
        if (cur is WalletSyncedTipInvalidated) return cur;
        if (cur is WalletSyncedTipLatched && tip < cur.tip) return cur;
        return _latch(cur, tip);
      case SyncStatus_Scanning():
        // A rewind sample invalidates from ANY state — including unset: a
        // cold launch into a rewound pass must suppress the persisted stamp
        // too, because the stamp's height may be the orphaned one.
        if (s.rewound) return const WalletSyncedTipInvalidated();
        // Belt: a scan whose TARGET is below the latch can only mean a
        // deep-reorg rewind this session missed (coalescing) or a server
        // switch to a lower tip — the latched claim is no longer safe.
        // Deliberately NOT compared against `from`: that is the engine's
        // monotonic scanned-EQUIVALENT height, which dips far below an
        // honest latch on routine catch-ups (review A1 — clamping to
        // `from` would wrongly clear the latch on every deep catch-up).
        if (cur is WalletSyncedTipLatched && s.to < cur.tip) {
          return const WalletSyncedTipInvalidated();
        }
        return cur;
      default:
        // Connecting / stalled / offline / idle / unknown: hold the verdict
        // (THE LATCH RULE's last bullet). In particular an INVALIDATED
        // verdict survives a fault-and-reconnect — only a completed pass on a
        // CURRENT server proves the claim safe again.
        return cur;
    }
  }

  /// The REPLAY rule (see [WalletSyncedTipLatched.at]): re-affirming the SAME
  /// tip keeps the original observation time — a re-subscribe replay after a
  /// background pause must not dress an old claim up as fresh; any other tip
  /// is a new observation. Shared by both latching arms of [_next] so the two
  /// cannot drift on it.
  static WalletSyncedTip _latch(WalletSyncedTip cur, int tip) =>
      (cur is WalletSyncedTipLatched && cur.tip == tip)
      ? cur
      : WalletSyncedTipLatched(tip, DateTime.now());
}

/// The activity-list page size (one screenful + headroom). MUST stay ≤ the Rust
/// `MAX_TX_PAGE_SIZE` ceiling (500) — the SDK clamps independently, so a future bump
/// past it would silently cap rather than error. No magic numbers (gate 7). The
/// accumulating activity list (keyset pagination) lives in `walletActivityProvider`
/// (see `wallet_activity_controller.dart`); the wallet screen calls its `refresh()`
/// on the same balance-changing edges as [walletSnapshotProvider].
const walletActivityPageSize = 50;

/// The wallet's current receive address (the unified address for account 0) —
/// what the user shares to receive ZEC. Only meaningful when a session exists
/// (the surface is reachable only from a provisioned wallet). NOT key material
/// — the address is public by design; §5.4 NEVER-LOG still applies
/// (display/copy, never log).
///
/// An identity-fenced read/view PAIR since #385 (the #381 (c) idiom — see the
/// fence note above), for two coupled reasons:
///  * **Per-identity CACHE, now BY CONSTRUCTION (E2E-1):** the
///    account-0 address is STABLE for the session (it does not rotate without
///    a re-provision), so the derive runs ONCE per wallet identity and every
///    later screen entry / tab toggle renders from the view's held value. The
///    pre-#385 plain provider already cached per container in-package (the
///    review proved the repeat NOT reproducible here at HEAD), so the
///    device-observed re-derive-per-visit likely rode host/session churn —
///    which this shape handles identically (same identity ⇒ held value; a
///    REAL identity flip ⇒ honest fresh derive, exactly right for duress).
///    The device symptom itself is re-verified on the #340 pass. No resume
///    invalidation either way (a background gap can't stale a stable
///    address).
///  * **The fence is LOAD-BEARING now:** with a
///    held value in play, an un-fenced provider would carry the OWNER's
///    address onto a duress/decoy identity's receive surface
///    (`copyWithPrevious` retention). The family re-key drops the old
///    identity's element — same guarantee as the four money views.
///
/// Invalidate the READ to force a fresh derive (the screen's retry does, via
/// the view's manual-invalidation forwarding); watch the VIEW. Both readers
/// PIN riverpod's retry OFF ([walletNoSilentRetry]) and are ANCHORED for the
/// container's life from the wallet screen (#386 — see the anchor note at
/// `_WalletActive`): together those are what make the timeout contract below
/// actually hold on a phone.
final walletReceiveAddressReadProvider = FutureProvider.autoDispose
    .family<String, Object?>((ref, identity) {
      // Dying-element guard — see walletSnapshotReadProvider. An error (not a
      // value): the screen renders its not-set-up state and never watches this
      // with a null session, so the arm is defensive-only, and the error state
      // it would paint is recoverable via the screen's retry. The two causes
      // carry DISTINCT messages (#386 — a superseded-identity flush read is
      // expected traffic; "no wallet session" in a log for it was a misdirect).
      if (ref.watch(walletIdentityProvider) != identity) {
        return Future<String>.error(
          StateError(
            'walletReceiveAddressReadProvider read under a superseded '
            'wallet identity',
          ),
        );
      }
      final session = ref.watch(walletSessionProvider);
      if (session == null) {
        return Future<String>.error(
          StateError(
            'walletReceiveAddressReadProvider read with no wallet session',
          ),
        );
      }
      // Honest degradation (principle 6): currentAddress() is a LOCAL
      // derivation, so a hang means the FFI boundary is wedged or starved —
      // not a slow network. Bound it so a wedged call surfaces the screen's
      // error state ("try again") instead of an infinite spinner on a money
      // surface.
      return session.currentAddress().timeout(walletAddressDeriveTimeout);
    }, retry: walletNoSilentRetry);

/// The receive-address VIEW — see [walletReceiveAddressReadProvider].
final walletReceiveAddressProvider =
    NotifierProvider<WalletReceiveAddressViewNotifier, AsyncValue<String>>(
      WalletReceiveAddressViewNotifier.new,
    );

/// See [walletReceiveAddressProvider].
class WalletReceiveAddressViewNotifier extends _IdentityScopedView<String> {
  @override
  FutureProvider<String> read(Object? identity) =>
      walletReceiveAddressReadProvider(identity);
}

/// The general local-FFI load bound (no-magic-numbers, gate 7), consumed by
/// NINE call sites across the package (send/shield preflights, activity
/// pages, rescan estimate, parked moves, auto-shield). A wedged FFI call
/// exceeding this surfaces the honest error state rather than hanging the
/// surface. Deliberately NOT raised by #386: the two address DERIVES got
/// their own measured bound below ([walletAddressDeriveTimeout]) — retuning
/// every money-path wedge detector 4× off one receive-screen measurement was
/// the review's converged blast-radius finding.
const walletFfiWedgeTimeout = Duration(seconds: 15);

/// The pre-name of [walletFfiWedgeTimeout], kept as an alias because this
/// file is barrel-exported (host compat). The bound was never receive-specific
/// — it was the de-facto package-wide FFI wedge detector under a
/// receive-screen name (the s198_tail naming finding).
@Deprecated(
  'Renamed to walletFfiWedgeTimeout — the bound was never receive-specific.',
)
const walletReceiveAddressTimeout = walletFfiWedgeTimeout;

/// The address-DERIVE load bound (#386, the E2E-2 device measurement),
/// used ONLY by [walletReceiveAddressReadProvider] /
/// [walletTransparentAddressReadProvider]: the FIRST derive under launch
/// catch-up queues behind the contended engine lock for 25–45 s on real
/// hardware, so the shared 15 s bound FALSE-FIRED on exactly the pass it was
/// calibrated against ("well past the ~3 s cold-launch derive" — an
/// idle-wallet figure). The contract is "never false-fires on an honest slow
/// path, always fires on a genuine wedge"; the derive's latency itself is the
/// warm-path (engine lock contention), out of scope beyond honest
/// surfacing.
const walletAddressDeriveTimeout = Duration(seconds: 60);

/// The shared no-silent-retry pin (#386 probe-6d, WIDENED by the audit):
/// riverpod 3's container default retries ANY non-`Error` exception up to 10
/// times with ~38 s of cumulative backoff — and `FrbException implements
/// Exception`, so every typed FFI failure rides it. On the pinned surfaces
/// that swallow turned an honest one-shot failure into minutes of loading,
/// each retry discarding the in-flight call: the address derives (#386 — ~10
/// timeout+backoff cycles of "Preparing your address…", each re-queuing a
/// fresh FFI call behind the same contended engine lock, with the rebuild
/// additionally deferred on a PAUSED element — the E2E-2 device symptom); the
/// parked/in-flight-swaps money witnesses (whose documented contract is
/// read-failure → an honest "couldn't load", not a section silent-hidden
/// through the retry window); the custody probe; the token list. No silent
/// retry: failure → visible `AsyncError` → the surface's OWN retry affordance
/// (a Try-again button, a screen re-open, the sync-edge/resume invalidation
/// cadence) IS the retry, exactly the designed contract. Public: a host
/// building custom readers over the SDK wants the same posture. The
/// deliberate KEEPS of the container default are documented at
/// [walletSnapshotReadProvider], [walletRecoverableEphemeralFundsReadProvider]
/// and [walletInFlightSendsReadProvider].
Duration? walletNoSilentRetry(int retryCount, Object error) => null;

/// The wallet's TRANSPARENT receive address (Recv-2 / ADR-0528) — the
/// external-scope P2PKH t-address the receive screen shows when the user
/// toggles to "Transparent". The [walletReceiveAddressReadProvider] pair's
/// exact mirror (#385): session-lifetime cached (this is the SLOW derive the
/// device walk measured at multiple seconds per visit), identity-fenced
/// (the same duress-leak sibling), same honest-degradation timeout. NOT key
/// material (PUBLIC by design; §5.4 NEVER-LOG still applies).
final walletTransparentAddressReadProvider = FutureProvider.autoDispose
    .family<String, Object?>((ref, identity) {
      // Dying-element guard — see walletSnapshotReadProvider; distinct
      // messages per cause (#386, the shielded twin's note).
      if (ref.watch(walletIdentityProvider) != identity) {
        return Future<String>.error(
          StateError(
            'walletTransparentAddressReadProvider read under a superseded '
            'wallet identity',
          ),
        );
      }
      final session = ref.watch(walletSessionProvider);
      if (session == null) {
        return Future<String>.error(
          StateError(
            'walletTransparentAddressReadProvider read with no wallet session',
          ),
        );
      }
      return session.currentTransparentAddress().timeout(
        walletAddressDeriveTimeout,
      );
    }, retry: walletNoSilentRetry);

/// The transparent-address VIEW — see [walletTransparentAddressReadProvider].
final walletTransparentAddressProvider =
    NotifierProvider<WalletTransparentAddressViewNotifier, AsyncValue<String>>(
      WalletTransparentAddressViewNotifier.new,
    );

/// See [walletTransparentAddressProvider].
class WalletTransparentAddressViewNotifier extends _IdentityScopedView<String> {
  @override
  FutureProvider<String> read(Object? identity) =>
      walletTransparentAddressReadProvider(identity);
}

/// The live, lifecycle-gated sync status (spec §3.3; flutter-patterns
/// § Stream Lifecycle; the contract in `core/lifecycle/app_lifecycle_provider`).
///
/// Two ORTHOGONAL recovery mechanisms, exactly as the contract demands:
///  1. Lifecycle gate — cancel the Rust-side stream on `paused` (battery),
///     re-snapshot + re-subscribe ONLY on a real `paused → resumed`
///     (tracked locally; `inactive/hidden → resumed` is window-focus /
///     shade noise, not a real resume). Never pause on `hidden` (desktop's
///     deepest state; Android's pre-`paused` step) — desktop keeps the
///     stream live while hidden, which is wanted.
///  2. Reconnect-on-fault — a stream error or completion (transport drop,
///     EOF, timeout — the unstable-network normal) re-subscribes with
///     capped exponential backoff, INDEPENDENT of lifecycle. On desktop
///     `paused` never fires, so this is the ONLY recovery path there.
final syncStatusProvider =
    NotifierProvider<SyncStatusNotifier, AsyncValue<SyncStatus>>(
      SyncStatusNotifier.new,
    );

class SyncStatusNotifier extends Notifier<AsyncValue<SyncStatus>> {
  StreamSubscription<SyncStatus>? _sub;
  Timer? _reconnect;
  Duration _backoff = minBackoff;

  /// Defence-in-depth: a pending reconnect [Timer] fires asynchronously, and
  /// touching `ref` after dispose throws. `_teardown` (wired to
  /// `ref.onDispose`) cancels the timer first, but an explicit flag makes the
  /// after-dispose safety obvious and survives a future refactor that might
  /// reorder the teardown.
  bool _disposed = false;

  /// Tracked locally because the lifecycle arrives as
  /// `paused → hidden → inactive → resumed`: at the `resumed` event the
  /// previous state is `inactive`, NOT `paused`, so only a remembered flag
  /// can tell a real resume from window-focus noise.
  bool _wasPaused = false;

  /// Whether the CURRENT subscription has delivered beyond its first
  /// (current-first replay) emit. A flapping link that connects, replays one
  /// status, then immediately drops would otherwise reset the backoff to the
  /// floor every cycle → a 2s hot-retry that defeats the whole point of the
  /// backoff on the exact unstable-mobile signature (connect-drop-connect-
  /// drop). So the replay does NOT reset backoff; only a SUBSEQUENT emit
  /// (proof the connection is alive and delivering) does.
  ///
  /// Bounded trade-off: a stream that recovers, delivers only its replay,
  /// then sits idle-healthy (a synced wallet with no further changes) keeps
  /// its grown backoff; a much-later fault then waits up to [maxBackoff] once
  /// before recovering. Acceptable (≤30s, once, on an already-synced wallet);
  /// a duration-based reset would heal it but isn't worth the extra timer.
  bool _subDelivered = false;

  /// Reconnect backoff floor and ceiling. A flaky link must not be hammered
  /// (battery + server politeness on unstable networks), and recovery must
  /// not stall forever — so backoff grows from [minBackoff], doubling, and
  /// is capped at [maxBackoff]. Public so the policy is pinned at its
  /// boundary by tests (no magic numbers).
  static const Duration minBackoff = Duration(seconds: 2);
  static const Duration maxBackoff = Duration(seconds: 30);

  @override
  AsyncValue<SyncStatus> build() {
    // PER-BUILD RESET (M1): Riverpod REUSES the notifier instance across a
    // `walletSessionProvider` rebuild (session close/reopen, onboarding flip) and
    // fires `ref.onDispose` (→ `_teardown`, which sets `_disposed = true`) BEFORE
    // re-running `build()`. Without this reset the instance fields are stale on
    // the rebuild — most dangerously `_disposed` stays `true`, so the reconnect
    // machinery early-returns forever and the reopened wallet NEVER syncs; the
    // carried-over `_backoff`/`_wasPaused`/`_subDelivered` would likewise corrupt
    // the fresh stream. The build is the single "(re)start" entry point, so it
    // re-initializes the lifecycle from scratch (matching `SwapStatusNotifier`).
    _disposed = false;
    _wasPaused = false;
    _subDelivered = false;
    _backoff = minBackoff;
    _sub = null;
    _reconnect = null;

    final session = ref.watch(walletSessionProvider);
    ref.onDispose(_teardown);
    // Riverpod disposes the previous `ref.listen` registration before re-running
    // `build()`, so re-registering here on every rebuild does NOT accumulate.
    ref.listen<AppLifecycleState>(appLifecycleProvider, (_, next) {
      _onLifecycle(next);
    });

    if (session == null) {
      // No wallet provisioned — the screen renders the not-set-up state and
      // does not watch this provider. A neutral loading is the honest value.
      return const AsyncValue<SyncStatus>.loading();
    }

    // Born-backgrounded cold start (Android push-trampoline): do not open
    // the stream while paused — the next real resume subscribes.
    if (ref.read(appLifecycleProvider) == AppLifecycleState.paused) {
      _wasPaused = true;
    } else {
      _subscribe(session);
    }
    return const AsyncValue<SyncStatus>.loading();
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
      // inactive / hidden / detached: no stream action. `hidden` is NOT a
      // pause (desktop deepest state + Android's pre-`paused` step).
      case AppLifecycleState.inactive:
      case AppLifecycleState.hidden:
      case AppLifecycleState.detached:
        break;
    }
  }

  void _subscribe(WalletSession session) {
    _reconnect?.cancel();
    _reconnect = null;
    _sub?.cancel();
    _subDelivered = false;
    _sub = session.watchSyncStatus().listen(
      (status) {
        // The connection is alive. Reset the backoff ONLY on a SUBSEQUENT
        // emit (not the current-first replay): a connection that survives
        // long enough to deliver an update is genuinely healthy; one that
        // replays-then-dies is flapping and must keep backing off.
        if (_subDelivered) {
          _backoff = minBackoff;
        } else {
          _subDelivered = true;
        }
        state = AsyncValue.data(status);
      },
      // A fault is NOT a terminal UI error (the §3.3 stream is supposed to
      // outlive faults; if the transport itself drops we just reconnect).
      // Keep the last good value visible and re-subscribe.
      onError: (Object _, StackTrace _) => _scheduleReconnect(),
      onDone: _scheduleReconnect,
      cancelOnError: true,
    );
  }

  void _scheduleReconnect() {
    _sub = null;
    if (_disposed) return;
    // Do not spin while backgrounded — a paused gate cancelled us on
    // purpose, and resume will re-subscribe.
    if (ref.read(appLifecycleProvider) == AppLifecycleState.paused) return;
    final session = ref.read(walletSessionProvider);
    if (session == null) return;
    _reconnect?.cancel();
    final wait = _backoff;
    _backoff = _capped(_backoff * 2);
    _reconnect = Timer(wait, () {
      // Re-check at FIRE time, not just schedule time: a pause or teardown
      // can land between scheduling and firing — never open a background
      // stream (battery) or touch `ref` after dispose.
      if (_disposed ||
          ref.read(appLifecycleProvider) == AppLifecycleState.paused) {
        return;
      }
      _subscribe(session);
    });
  }

  void _pause() {
    _reconnect?.cancel();
    _reconnect = null;
    _sub?.cancel();
    _sub = null;
    // Keep `state` (the last known status) so a glance on resume — before
    // the fresh subscribe lands — shows last-known, not a spinner.
  }

  void _resume() {
    final session = ref.read(walletSessionProvider);
    if (session == null) return;
    // A resume is a fresh user intent, so reset backoff and attempt at once
    // (not via the backoff timer). Resume is user-paced, never a hot loop.
    _backoff = minBackoff;
    // Cold re-read FIRST (catch changes missed while backgrounded), THEN
    // re-subscribe for live deltas (spec §3.3 resume idiom). Re-pull the
    // recoverable subset on the SAME resume edge as the snapshot it annotates
    // (its provider doc promises the same balance-changing edges) so the balance
    // note can't survive a background gap stale (2e-2b-iv). The READ providers
    // are the invalidation targets (#381 (c) — the views are derivations).
    ref.invalidate(walletSnapshotReadProvider);
    ref.invalidate(walletRecoverableEphemeralFundsReadProvider);
    ref.invalidate(walletParkedSendsReadProvider);
    ref.invalidate(walletInFlightSendsReadProvider);
    // The in-flight SWAPS home rides the same resume edge (W-swap-5, #366): a
    // swap that went terminal (and was dismissed elsewhere) or newly executed
    // while backgrounded must reconcile the moment the user returns.
    ref.invalidate(walletInFlightSwapsReadProvider);
    _subscribe(session);
  }

  void _teardown() {
    _disposed = true;
    _reconnect?.cancel();
    _reconnect = null;
    _sub?.cancel();
    _sub = null;
  }

  static Duration _capped(Duration d) => d > maxBackoff ? maxBackoff : d;
}

/// The live incoming-funds event stream (spec §3.3 / ADR-0536 — the FR-1
/// "funds arrived" hook), lifecycle-gated and reconnecting exactly like
/// [syncStatusProvider] (the two orthogonal recovery mechanisms documented
/// there). State = the LATEST event; loading until the first `replay` lands.
///
/// CURSOR THREADING (the catch-up contract): every event's non-empty opaque
/// `cursor` is remembered IN MEMORY and passed as `sinceCursor` on the next
/// (re)subscribe — so a resume after a background gap gets a `replay` event
/// covering exactly what was missed, and a mid-session reconnect never
/// re-baselines. Memory-only on purpose: this package cold-reads history on
/// mount anyway, so a process death loses nothing a host cares about; hosts
/// driving NOTIFICATIONS persist their own cursor (and their own exactly-once
/// ledger) per the [WalletSession.watchIncomingFunds] contract. The cursor
/// resets with the session build — a wallet-identity flip must never carry
/// the previous identity's watermark (the #381 cross-identity retention rule).
///
/// This provider deliberately invalidates NOTHING itself: the money-read
/// refresh policy stays where it lives today (the wallet screen's edge
/// listener + [SyncStatusNotifier]'s resume). The screen listens to THIS
/// provider for the cheap activity-list refresh on arrival edges.
final incomingFundsEventsProvider =
    NotifierProvider<IncomingFundsNotifier, AsyncValue<IncomingFundsEvent>>(
      IncomingFundsNotifier.new,
    );

class IncomingFundsNotifier extends Notifier<AsyncValue<IncomingFundsEvent>> {
  StreamSubscription<IncomingFundsEvent>? _sub;
  Timer? _reconnect;
  Duration _backoff = SyncStatusNotifier.minBackoff;
  bool _disposed = false;
  bool _wasPaused = false;

  /// See [SyncStatusNotifier._subDelivered] — the replay-then-die flap guard:
  /// only a SUBSEQUENT emit (proof the stream delivers) resets the backoff.
  bool _subDelivered = false;

  /// The last non-empty cursor seen (any kind stamps it — a `memoRefresh`
  /// carries the previous watermark forward, so adopting it is idempotent).
  String? _cursor;

  @override
  AsyncValue<IncomingFundsEvent> build() {
    // Per-build reset (the SyncStatusNotifier M1 doctrine): Riverpod reuses
    // the instance across a session rebuild; stale fields — most dangerously
    // `_disposed` and the previous identity's `_cursor` — must never survive.
    _disposed = false;
    _wasPaused = false;
    _subDelivered = false;
    _backoff = SyncStatusNotifier.minBackoff;
    _sub = null;
    _reconnect = null;
    _cursor = null;

    final session = ref.watch(walletSessionProvider);
    ref.onDispose(_teardown);
    ref.listen<AppLifecycleState>(appLifecycleProvider, (_, next) {
      _onLifecycle(next);
    });

    if (session == null) {
      return const AsyncValue<IncomingFundsEvent>.loading();
    }
    if (ref.read(appLifecycleProvider) == AppLifecycleState.paused) {
      // Born-backgrounded cold start: subscribe on the next real resume.
      _wasPaused = true;
    } else {
      _subscribe(session);
    }
    return const AsyncValue<IncomingFundsEvent>.loading();
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
      case AppLifecycleState.inactive:
      case AppLifecycleState.hidden:
      case AppLifecycleState.detached:
        break;
    }
  }

  void _subscribe(WalletSession session) {
    _reconnect?.cancel();
    _reconnect = null;
    _sub?.cancel();
    _subDelivered = false;
    _sub = session
        .watchIncomingFunds(sinceCursor: _cursor)
        .listen(
          (event) {
            if (_subDelivered) {
              _backoff = SyncStatusNotifier.minBackoff;
            } else {
              _subDelivered = true;
            }
            if (event.cursor.isNotEmpty) _cursor = event.cursor;
            state = AsyncValue.data(event);
          },
          // A fault is not a terminal UI error — keep the last event visible and
          // reconnect with backoff. The one PERMANENT fault shape would be a
          // poisoned `_cursor` (typed-rejected on every retry): the `isNotEmpty`
          // guard above is the protection (the core also never delivers an
          // empty cursor since the M2 pump substitution — belt and
          // suspenders).
          onError: (Object _, StackTrace _) => _scheduleReconnect(),
          onDone: _scheduleReconnect,
          cancelOnError: true,
        );
  }

  void _scheduleReconnect() {
    _sub = null;
    if (_disposed) return;
    if (ref.read(appLifecycleProvider) == AppLifecycleState.paused) return;
    final session = ref.read(walletSessionProvider);
    if (session == null) return;
    _reconnect?.cancel();
    final wait = _backoff;
    _backoff = _capped(_backoff * 2);
    _reconnect = Timer(wait, () {
      if (_disposed ||
          ref.read(appLifecycleProvider) == AppLifecycleState.paused) {
        return;
      }
      _subscribe(session);
    });
  }

  void _pause() {
    _reconnect?.cancel();
    _reconnect = null;
    _sub?.cancel();
    _sub = null;
    // Keep `state` (the last event) — the cursor in it is exactly what the
    // resume re-subscribe replays from.
  }

  void _resume() {
    final session = ref.read(walletSessionProvider);
    if (session == null) return;
    // No money-read invalidations here — [SyncStatusNotifier._resume] owns the
    // resume-edge refresh set; duplicating it would double every resume read.
    _backoff = SyncStatusNotifier.minBackoff;
    _subscribe(session);
  }

  void _teardown() {
    _disposed = true;
    _reconnect?.cancel();
    _reconnect = null;
    _sub?.cancel();
    _sub = null;
  }

  static Duration _capped(Duration d) =>
      d > SyncStatusNotifier.maxBackoff ? SyncStatusNotifier.maxBackoff : d;
}
