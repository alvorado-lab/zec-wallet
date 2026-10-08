import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../send_authorization.dart';
import '../shield/shield_controller.dart';
import '../shield/shield_state.dart';
import '../wallet_providers.dart';
import 'transparent_funds_providers.dart';

/// What the auto-shield loop last did — read by the balance card's honesty
/// cue. NOT a flow state machine (there is no sheet behind it): the loop is
/// edge-driven and this only needs to say whether the last attempt completed.
enum AutoShieldStatus {
  /// Nothing owed: below threshold, disabled, deferred, or the last attempt
  /// handed the funds to the SDK (whose persist-then-broadcast machinery owns
  /// them from there — a broadcast miss re-sends on the next sync and is NOT
  /// a failure of this loop).
  idle,

  /// An attempt is in flight (propose → authorize → send).
  attempting,

  /// The last attempt FAILED before the SDK took the funds (a typed
  /// propose/sign error or a wedged-FFI timeout). The transparent balance is
  /// untouched and stays visible; the balance card shows the honest cue and
  /// the next sync edge retries.
  failed,

  /// The host authorizer DENIED the automatic spend — a policy decision, so
  /// the loop STOPS for this session (no prompt/denial spam; a session or
  /// settings change re-arms it). The cue shows: the funds stay public until
  /// the user shields manually.
  denied,
}

/// The auto-shield policy loop (§3.2i-3 (a); maintainer calls) — the
/// [WalletSyncController] structural sibling, but SNAPSHOT-driven: policy is
/// host-side (ADR-0529), `proposeShield` is the ready primitive, and every
/// refreshed balance snapshot (the sync edges refresh it) is an evaluation
/// point. Fires when ALL hold:
///
///  * a live session exists and the auto-shield switch is ON — a switch still
///    LOADING never fires (a slow disk read must not race a persisted OFF),
///  * OS power-save is not active (maintainer call: DEFER, shield on the next
///    normal-power evaluation; desktop is never power-save),
///  * the transparent balance ≥ max(host threshold, the SDK's own shielding
///    floor — `proposeShield` returns null below it, a quiet no-op),
///  * no manual shield flow is in flight (the sheet wins; skipping beats
///    racing it for the same UTXOs), and
///  * the authorizer has not denied an automatic spend this session.
///
/// The money sequence is the manual shield's, verbatim discipline (#327/#330/
/// authorize exactly once with `origin: automatic`, identity fence
/// inside the action closure, at-most-one in-flight attempt, generation-
/// guarded continuations. FAILURE HONESTY: a failed/denied attempt leaves the
/// transparent figure visible (the `state.rs` invariant renders it) plus the
/// balance-card cue — never a silent retry-forever, never a hidden balance.
final walletAutoShieldControllerProvider =
    NotifierProvider<AutoShieldController, AutoShieldStatus>(
      AutoShieldController.new,
    );

class AutoShieldController extends Notifier<AutoShieldStatus> {
  /// Post-dispose `ref`/`state` touches throw; reset each build (riverpod
  /// REUSES the notifier across dependency-change rebuilds — the #330 model).
  bool _disposed = false;

  /// Build-cycle stamp for attempt continuations (the sync-controller
  /// discipline): a session flip re-runs build() on the SAME notifier; a
  /// continuation from the prior cycle must not write into the new one.
  int _generation = 0;

  /// The build-cycle generation holding the in-flight attempt slot, or null
  /// — at-most-one attempt per SESSION generation. Generation-aware ON
  /// PURPOSE (reliability review MINOR-2): an identity flip ABANDONS the
  /// old cycle's slot — its spend is already foreclosed by the pre-prompt
  /// re-check + the fence, and its continuations are generation-guarded — so
  /// a never-completing host prompt wedges only the session it belongs to; a
  /// session change escapes, matching the seam contract's wedge note. The
  /// old attempt's `finally` releases ONLY its own generation's slot.
  int? _attemptingGeneration;

  /// The authorizer denied an automatic spend — latched until a session
  /// change OR the auto-shield switch being turned back ON (the user's
  /// explicit "run automation" restart). Denial is host POLICY; re-prompting
  /// every sync edge would be spam.
  bool _deniedThisSession = false;

  /// The transparent figure of the last attempt that COMPLETED — by handing
  /// the funds to the SDK, or by the engine answering "nothing shieldable"
  /// (null). Re-evaluating at the SAME figure would only re-propose against
  /// inputs the engine already consumed: its null answer makes a double-spend
  /// impossible regardless, but this latch makes the loop QUIET BY
  /// CONSTRUCTION — the post-handoff snapshot invalidate (and every edge
  /// while the shield is pending) skips instead of re-proposing, independent
  /// of engine timing. Self-clearing: the comparison fails the moment the
  /// figure CHANGES (a new arrival; the shield mining away). A FAILED or
  /// denied attempt deliberately does NOT latch — the next edge retries.
  int? _quietAtTransparentZat;

  @override
  AutoShieldStatus build() {
    _disposed = false;
    _generation++;
    _deniedThisSession = false;
    _quietAtTransparentZat = null;
    ref.onDispose(() => _disposed = true);

    // Identity keys the loop (same rule as the whole live graph). Everything
    // else is a LISTEN, not a watch — a policy/balance change must evaluate,
    // not rebuild (a rebuild would re-arm a denied latch).
    final session = ref.watch(walletSessionProvider);

    // Every refreshed snapshot is an evaluation point: the wallet-active sync
    // edges (reachedTip / becameSpendable) invalidate it, so arrivals are
    // seen exactly when the balance is. The post-attempt invalidate below
    // re-evaluates once more — by then the proposed UTXOs are consumed and
    // `proposeShield` quietly returns null, so there is no hot loop.
    ref.listen(walletSnapshotProvider, (_, next) {
      if (next.hasValue) _maybeAttempt();
    });
    // Power-save lifting and the switch loading/turning ON are the two other
    // "now it may fire" edges.
    ref.listen(walletPowerSaveActiveProvider, (_, active) {
      if (!active) _maybeAttempt();
    });
    ref.listen(walletAutoShieldEnabledProvider, (prev, next) {
      // TRANSITION-keyed on SETTLED values (wrap security MINOR-3): a
      // provider rebuild re-emits the current value through loading-with-
      // previous states — a value-keyed arm would re-arm a denied latch (and
      // re-prompt a denying host) with no user gesture. Only a settled value
      // that DIFFERS from the previous one is a user-driven flip; the cold
      // load (prev carries no value) counts as a flip in its own direction.
      if (!next.hasValue || next.isLoading) return;
      final on = next.value == true;
      final wasOn = prev?.value == true;
      if (on && !wasOn) {
        // Turning the switch ON is the user's explicit "run automation" —
        // it RE-ARMS a host-denied loop (the denial latch guards against
        // prompt SPAM, not against a deliberate restart; review m6) and
        // re-evaluates.
        _deniedThisSession = false;
        _maybeAttempt();
      } else if (!on && (prev == null || wasOn)) {
        // A deliberate OFF is the expert's hold-transparent choice — a
        // lingering failure/denial cue would contradict the explicit opt-out
        // (review m5); the standing transparent note still explains the
        // visibility.
        _setStatus(AutoShieldStatus.idle, _generation);
      }
    });

    if (session != null) {
      // Cold-start evaluation (a wallet opened with an existing transparent
      // balance must not wait for the first sync edge). Deferred: build()
      // must not write state.
      Future<void>.microtask(_maybeAttempt);
    }
    return AutoShieldStatus.idle;
  }

  Future<void> _maybeAttempt() async {
    if (_disposed ||
        _attemptingGeneration == _generation ||
        _deniedThisSession) {
      return;
    }
    final session = ref.read(walletSessionProvider);
    if (session == null) return;
    // #397 §3.7 D3 (two-reviewer-confirmed money-honesty gap): a WATCH-ONLY wallet
    // holds no spending keys, so shielding (transparent → shielded) is a SPEND
    // it can never do. Never arm the loop for it — otherwise proposeShield throws
    // WatchOnly on EVERY sync edge (FFI churn), and the failed status latches the
    // false "shielding didn't complete — you can shield them now" cue on a card
    // with no Shield button. Mirrors the walletAutoShieldSupportedProvider guard
    // below; synchronous + correct on the package gate (the wallet-screen cue is
    // ALSO belt-gated on !watchOnly, covering the session-only-host fallback).
    if (ref.read(isWatchOnlyProvider)) return;
    // A LOADING snapshot can still carry the PREVIOUS session's figure
    // (riverpod keeps prior data through a reload, and the real snapshot read
    // is an FFI round-trip the cold-start microtask always beats) —
    // evaluating on it lets the dead identity's balance pass the new
    // identity's threshold (security review MINOR-4). Skip; the
    // snapshot listener re-fires on the resolved data.
    if (ref.read(walletSnapshotProvider).isLoading) return;
    // #383 R2: a host that declared automatic spends UNSUPPORTED never arms
    // the loop — the sheet hides the toggle + automation claim; this is the
    // loop-side half of the same honest absence. Read per evaluation (the
    // seam is a static override per scope, like the other capability flags).
    if (!ref.read(walletAutoShieldSupportedProvider)) return;
    // DEFER under power-save (maintainer call 2) — the funds stay visibly
    // transparent; the power-save listener re-evaluates when it lifts.
    if (ref.read(walletPowerSaveActiveProvider)) return;
    // A LOADING flag never fires. Two loading shapes, both fail-safe here: a
    // COLD load (value null) fails `!= true`; but an identity-switch WARM reload
    // carries the PREVIOUS identity's value (riverpod copyWithPrevious, exactly
    // as the snapshot guarded at the isLoading check above), so once settings
    // became per-identity a namespaced switch A→B could read A's stale `true`
    // while B is still loading — the direct analog of the MINOR-4 snapshot fix
    // (review M2/LOW-1). Guard on isLoading too; the listener re-fires on
    // B's resolved value. (Loading still renders as ON in the sheet — the
    // shipped default — but never ACTS as ON.)
    final autoShield = ref.read(walletAutoShieldEnabledProvider);
    if (autoShield.isLoading || autoShield.value != true) return;
    final transparent =
        ref.read(walletSnapshotProvider).value?.balance.transparentZat ?? 0;
    // Already handed off (or proven un-shieldable) at exactly this figure —
    // quiet until it changes (see [_quietAtTransparentZat]). Any DIFFERING
    // observation clears the latch entirely: without the clear, a figure that
    // drops (the shield mines to 0) and later RETURNS to the exact latched
    // value (a same-amount new arrival) would compare equal to the stale
    // latch and be skipped forever.
    if (transparent == _quietAtTransparentZat) return;
    _quietAtTransparentZat = null;
    if (transparent < ref.read(walletAutoShieldThresholdZatProvider)) return;
    // Manual flow wins: skip this evaluation rather than race the sheet for
    // the same UTXOs (a lost race is money-safe — the one-shot token and the
    // engine's input tracking make the second send fail typed — but skipping
    // is cleaner). The next snapshot refresh re-evaluates.
    final manual = ref.read(shieldControllerProvider);
    if (manual is ShieldPreparing || manual is ShieldSubmitting) return;

    final gen = _generation;
    // RE-ENTRANCY re-check (#381 (c)): the money views are LAZY derivations
    // now, so a `ref.read` above can FLUSH a dirty provider — which delivers
    // its pending notifications synchronously, re-entering this method via
    // the snapshot/flag listeners BEFORE this frame reaches the latch. The
    // re-entrant evaluation may have already committed an attempt; everything
    // from the entry guard to here is synchronous, so this single re-check
    // at the commit point closes the window (without it: a double-propose —
    // the `proposeShieldCount == 1` pins in the auto-shield suite caught it
    // the moment the #381 view landed).
    if (_attemptingGeneration == gen) return;
    _attemptingGeneration = gen;
    // Phase marker for the transient-quieting below (wrap review
    // MINOR-4): the quieting exists for the cold-start PROPOSE transient
    // only — a busy/stale thrown from the SEND phase is a real
    // "didn't complete" and must cue.
    var proposed = false;
    _setStatus(AutoShieldStatus.attempting, gen);
    try {
      // LOCAL call — a hang is a wedged FFI, not a slow network (the shield
      // sheet's exact timeout discipline).
      final proposal = await session.proposeShield().timeout(
        walletFfiWedgeTimeout,
      );
      proposed = true;
      if (proposal == null) {
        // Below the SDK's shielding floor, or the funds are already consumed
        // by a pending shield — an honest no-op; quiet at this figure.
        if (!_disposed && gen == _generation) {
          _quietAtTransparentZat = transparent;
        }
        _setStatus(AutoShieldStatus.idle, gen);
        return;
      }
      // PRE-PROMPT fence (security review MAJOR-2): the propose await
      // spans a real FFI round-trip — an identity flip landing inside it must
      // not let the DEAD identity's authorization prompt open over the NEW
      // identity's screen (a prompting host would render the intent's amount:
      // a disclosure of the other identity's balance, and of the identity's
      // existence, that the SPEND fence below cannot stop — it fires after
      // the prompt). The SWITCH and the denial latch are re-checked in the
      // same breath (wrap review MINOR-1): a persisted OFF (or a denial)
      // landing during the round-trip is a hold-transparent choice this
      // attempt must honor — without it a passthrough authorizer spends
      // against a decision the user already made.
      if (_disposed ||
          gen != _generation ||
          !identical(ref.read(walletSessionProvider), session) ||
          _deniedThisSession ||
          // the SUPPORTED flag joins the same-breath re-checks.
          // The seam is documented static-per-scope, but if a host ever wires
          // it reactively, a supported→false landing inside the propose
          // round-trip must not open an automatic-spend prompt (or, on a
          // passthrough authorizer, spend) for a host that just declared
          // automatic spends unsupported — the exact MINOR-1 rationale
          // the enabled-switch re-check beside it exists for.
          !ref.read(walletAutoShieldSupportedProvider) ||
          ref.read(walletAutoShieldEnabledProvider).value != true) {
        return;
      }
      final authorizer = ref.read(walletSendAuthorizerProvider);
      await authorizer.authorizeSpend(
        WalletSpendIntent(
          kind: WalletSpendKind.shield,
          amountZat: proposal.totalZat,
          origin: WalletSpendOrigin.automatic,
          // #383 R3: the automatic shield is the same self-transfer as the
          // manual one — display-facts match (the automatic⇒shield
          // invariant means recipientIsSelf is always true on this origin).
          recipientIsSelf: true,
          feeZat: proposal.feeZat,
          // FR-17 (#396): the proposal's spend-binding nonce — the automatic
          // shield signs a proposal like the manual one, so it binds the same
          // way.
          bindingToken: proposal.binding,
        ),
        () {
          // The identity-switch spend fence — same closure as every
          // authorizer-wrapped action in the package.
          if (!ref.mounted ||
              !identical(ref.read(walletSessionProvider), session)) {
            throw const WalletSpendSessionChanged();
          }
          return session.send(proposal.proposalId);
        },
      );
      if (_disposed) return;
      // The SDK now owns the funds (persist-before-submit; a broadcast miss
      // re-sends on the next sync — not this loop's failure). Latch quiet at
      // the handed-off figure BEFORE the invalidate, so the self-triggered
      // snapshot event skips instead of re-proposing. Both hoisted above the
      // generation guard like every outcome-landing invalidate.
      if (gen == _generation) _quietAtTransparentZat = transparent;
      ref.invalidate(walletSnapshotReadProvider);
      _setStatus(AutoShieldStatus.idle, gen);
    } on WalletSpendAuthorizationDenied {
      // A denial belongs to the session that prompted: a stale denial landing
      // after an identity flip — exactly what the seam contract tells hosts
      // to do with outstanding prompts at a switch — must not disarm the NEW
      // session's loop (review MAJOR-1, probe-confirmed by two reviewers).
      if (gen == _generation) _deniedThisSession = true;
      _setCue(AutoShieldStatus.denied, gen);
    } on WalletSpendSessionChanged {
      // Nothing was attempted; the new identity's build re-evaluates fresh.
      return;
    } catch (error) {
      // A busy / not-synced PROPOSE during initial sync is a NORMAL transient
      // (the next edge retries) — cueing "didn't complete" at a user who just
      // opened a cold wallet would be a false alarm (UX review m8);
      // phase-gated (see `proposed`): the SAME kinds thrown from the SEND
      // phase are a real didn't-complete and cue. Any other typed failure /
      // the wedged-FFI timeout: funds untouched and visible; the cue shows;
      // the next edge retries.
      final classified = classifyShieldPrepareFailure(error);
      final transient =
          !proposed &&
          classified is ShieldUnavailable &&
          (classified.reason == ShieldFaultReason.notSyncedYet ||
              classified.reason == ShieldFaultReason.walletBusy);
      if (transient) {
        _setStatus(AutoShieldStatus.idle, gen);
      } else {
        _setCue(AutoShieldStatus.failed, gen);
      }
    } finally {
      if (_attemptingGeneration == gen) _attemptingGeneration = null;
    }
  }

  /// Cue-bearing outcomes (failed/denied) are SUPPRESSED to idle when the
  /// switch is not ON at landing time (wrap review MINOR-2): the OFF
  /// arm's clear is otherwise un-durable — a same-generation in-flight
  /// outcome landing after the flip would resurrect a cue the balance card
  /// promises never to show over a deliberate opt-out.
  void _setCue(AutoShieldStatus outcome, int gen) {
    if (_disposed) return;
    final on = ref.read(walletAutoShieldEnabledProvider).value == true;
    _setStatus(on ? outcome : AutoShieldStatus.idle, gen);
  }

  void _setStatus(AutoShieldStatus next, int gen) {
    if (_disposed || gen != _generation) return;
    state = next;
  }
}
