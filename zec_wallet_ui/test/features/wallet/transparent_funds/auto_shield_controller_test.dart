import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart' show StateProvider;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/send_authorization.dart';
import 'package:zec_wallet_ui/features/wallet/shield/shield_controller.dart';
import 'package:zec_wallet_ui/features/wallet/shield/shield_state.dart';
import 'package:zec_wallet_ui/features/wallet/transparent_funds/auto_shield_controller.dart';
import 'package:zec_wallet_ui/features/wallet/transparent_funds/transparent_funds_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// Auto-shield policy-loop tests (§3.2i-3 (a); maintainer calls). Pure
/// host-VM: the loop over `FakeWalletSession` + `FakeSendAuthorizer` +
/// `FakeWalletSettingsStore`. Pins the spec's named contract: fires at an
/// edge over threshold, below-threshold no-op, a LOADING switch never fires,
/// power-save defers then fires, denial stops-without-spam, failure leaves
/// the funds visible + retries, at-most-one in flight, the manual sheet
/// yields, and the identity-flip fence (zero bridge calls on either session).
void main() {
  FakeWalletSession fakeWithTransparent(int zat) => FakeWalletSession(
    current: const SyncStatus.upToDate(tip: 100),
    snapshotValue: walletStateFixture(
      syncStatus: const SyncStatus.upToDate(tip: 100),
      balance: balanceFixture(
        spendableZat: 500000,
        totalZat: 500000 + zat,
        transparentZat: zat,
      ),
    ),
  )..proposeShieldResult = shieldProposalFixture();

  ({
    ProviderContainer container,
    FakeWalletSession fake,
    FakeSendAuthorizer auth,
    FakeWalletSettingsStore store,
    StateProvider<bool> powerSwitch,
  })
  harness({
    FakeWalletSession? session,
    FakeSendAuthorizer? authorizer,
    FakeWalletSettingsStore? store,
    bool powerSave = false,
  }) {
    final fake = session ?? fakeWithTransparent(200000);
    final auth = authorizer ?? FakeSendAuthorizer();
    final settings = store ?? FakeWalletSettingsStore();
    final powerSwitch = StateProvider<bool>((ref) => powerSave);
    final container = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWithValue(fake),
        walletSendAuthorizerProvider.overrideWithValue(auth),
        walletSettingsStoreProvider.overrideWithValue(settings),
        walletPowerSaveActiveProvider.overrideWith(
          (ref) => ref.watch(powerSwitch),
        ),
      ],
    );
    addTearDown(container.dispose);
    // Build + keep the loop alive (what `_WalletActive`'s watch does).
    container.listen(walletAutoShieldControllerProvider, (_, _) {});
    return (
      container: container,
      fake: fake,
      auth: auth,
      store: settings,
      powerSwitch: powerSwitch,
    );
  }

  /// Drain microtasks + the store/snapshot reads the evaluation awaits.
  Future<void> settle() async {
    for (var i = 0; i < 10; i++) {
      await Future<void>.delayed(Duration.zero);
    }
  }

  test('fires on the cold snapshot over threshold: propose → authorize(shield, '
      'AUTOMATIC) → send, then back to idle', () async {
    final h = harness();
    await settle();

    expect(h.fake.proposeShieldCount, 1);
    expect(h.fake.sendCount, 1, reason: 'the authorized action ran');
    expect(
      h.auth.intents,
      hasLength(1),
      reason: 'authorization is NEVER bypassed for an automatic spend',
    );
    expect(h.auth.intents.single.kind, WalletSpendKind.shield);
    expect(
      h.auth.intents.single.origin,
      WalletSpendOrigin.automatic,
      reason: 'host policy can tell an automatic spend apart',
    );
    expect(
      h.auth.intents.single.recipientIsSelf,
      isTrue,
      reason:
          '#383 R3: the automatic shield is a self-transfer — the prompt '
          'may honestly say the funds stay in this wallet',
    );
    expect(
      h.container.read(walletAutoShieldControllerProvider),
      AutoShieldStatus.idle,
      reason: 'the SDK owns the funds now — not this loop\'s failure state',
    );
  });

  test('#383 R2: an UNSUPPORTED host never arms the loop — zero proposes, '
      'zero prompts, the funds stay honestly visible', () async {
    final fake = fakeWithTransparent(200000);
    final auth = FakeSendAuthorizer();
    final container = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWithValue(fake),
        walletSendAuthorizerProvider.overrideWithValue(auth),
        walletSettingsStoreProvider.overrideWithValue(
          FakeWalletSettingsStore(),
        ),
        walletPowerSaveActiveProvider.overrideWith((ref) => false),
        walletAutoShieldSupportedProvider.overrideWithValue(false),
      ],
    );
    addTearDown(container.dispose);
    container.listen(walletAutoShieldControllerProvider, (_, _) {});
    await settle();

    expect(fake.proposeShieldCount, 0, reason: 'the evaluation early-returns');
    expect(fake.sendCount, 0);
    expect(
      auth.intents,
      isEmpty,
      reason:
          'no prompt for a loop that never '
          'runs',
    );

    // A later snapshot edge stays quiet too — the gate is per evaluation.
    container.invalidate(walletSnapshotReadProvider);
    await settle();
    expect(fake.proposeShieldCount, 0);
    expect(auth.intents, isEmpty);
  });

  test('#397 §3.7 D3: a WATCH-ONLY wallet NEVER arms the loop — zero proposes '
      '(shielding is a spend it can\'t do), so the false "didn\'t complete" cue '
      'never latches and there is no per-edge FFI churn', () async {
    // The two-reviewer-confirmed money-honesty gap: without the gate, proposeShield
    // throws WatchOnly on every sync edge → the failed status latches the
    // "shielding didn't complete — you can shield them now" cue on a card with
    // no Shield button. isWatchOnly is overridden synchronously true (the
    // production package-gate shape); the session-only-host async-fallback edge
    // is covered by the wallet-screen cue's !watchOnly belt.
    final fake = fakeWithTransparent(200000);
    final auth = FakeSendAuthorizer();
    final container = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWithValue(fake),
        walletSendAuthorizerProvider.overrideWithValue(auth),
        walletSettingsStoreProvider.overrideWithValue(
          FakeWalletSettingsStore(),
        ),
        walletPowerSaveActiveProvider.overrideWith((ref) => false),
        isWatchOnlyProvider.overrideWithValue(true),
      ],
    );
    addTearDown(container.dispose);
    container.listen(walletAutoShieldControllerProvider, (_, _) {});
    await settle();

    expect(fake.proposeShieldCount, 0, reason: 'watch-only can never shield');
    expect(fake.sendCount, 0);
    expect(auth.intents, isEmpty);
    expect(
      container.read(walletAutoShieldControllerProvider),
      AutoShieldStatus.idle,
      reason: 'no failed cue for a wallet that can never shield',
    );

    // No re-fire on later edges (the missing `_quietAtTransparentZat` churn the
    // security review measured is closed at the source by the early-return).
    container.invalidate(walletSnapshotReadProvider);
    await settle();
    expect(fake.proposeShieldCount, 0, reason: 'still quiet on later edges');
  });

  test('below the threshold: a quiet no-op, zero bridge calls', () async {
    final h = harness(session: fakeWithTransparent(99999));
    await settle();

    expect(h.fake.proposeShieldCount, 0);
    expect(h.fake.sendCount, 0);
    expect(h.auth.intents, isEmpty);
  });

  test('a LOADING auto-shield switch never fires — a persisted OFF wins the '
      'race against the first evaluation', () async {
    final store = FakeWalletSettingsStore(autoShieldValue: false)
      ..readGate = Completer<void>();
    final h = harness(store: store);
    await settle();
    expect(
      h.fake.proposeShieldCount,
      0,
      reason: 'no attempt while the flag is still on disk',
    );

    // The read completes to the persisted OFF — still no attempt.
    store.readGate!.complete();
    await settle();
    expect(h.fake.proposeShieldCount, 0);
    expect(h.auth.intents, isEmpty);
  });

  test('auto-shield OFF is honored at every edge', () async {
    final h = harness(store: FakeWalletSettingsStore(autoShieldValue: false));
    await settle();
    h.container.invalidate(walletSnapshotReadProvider);
    await settle();

    expect(h.fake.proposeShieldCount, 0);
    expect(h.fake.sendCount, 0);
  });

  test('power-save DEFERS (funds stay untouched), then the lift fires exactly '
      'one attempt', () async {
    final h = harness(powerSave: true);
    await settle();
    expect(
      h.fake.proposeShieldCount,
      0,
      reason: 'deferred under battery-saver (founder call S154)',
    );

    h.container.read(h.powerSwitch.notifier).state = false;
    await settle();
    expect(h.fake.proposeShieldCount, 1, reason: 'the lift re-evaluates');
    expect(h.fake.sendCount, 1);
  });

  test('a DENIED automatic spend latches the loop off for the session — no '
      'prompt spam on later edges, status shows the cue', () async {
    final h = harness(authorizer: FakeSendAuthorizer(denyAll: true));
    await settle();

    expect(h.auth.intents, hasLength(1));
    expect(h.fake.sendCount, 0, reason: 'denied BEFORE any bridge call');
    expect(
      h.container.read(walletAutoShieldControllerProvider),
      AutoShieldStatus.denied,
    );

    h.container.invalidate(walletSnapshotReadProvider);
    await settle();
    expect(
      h.auth.intents,
      hasLength(1),
      reason: 'no second authorization attempt this session',
    );
  });

  test('a typed propose failure leaves the funds visible, shows the cue, and '
      'the next edge RETRIES', () async {
    // A NON-transient kind (busy/not-synced deliberately stay quiet — m8):
    // signFailed classifies as couldNotPrepare, the real "didn't complete".
    final fake = fakeWithTransparent(200000)
      ..proposeShieldThrows = const WalletApiError(
        code: 'RW-TEST',
        message: 'static',
        kind: WalletErrorKind.signFailed(),
      );
    final h = harness(session: fake);
    await settle();

    expect(
      h.container.read(walletAutoShieldControllerProvider),
      AutoShieldStatus.failed,
    );
    expect(h.fake.sendCount, 0, reason: 'nothing was signed');

    // The wallet frees up; the next snapshot refresh retries and completes.
    fake.proposeShieldThrows = null;
    h.container.invalidate(walletSnapshotReadProvider);
    await settle();
    expect(h.fake.sendCount, 1);
    expect(
      h.container.read(walletAutoShieldControllerProvider),
      AutoShieldStatus.idle,
    );
  });

  test('the quiet latch clears on any DIFFERING figure: a same-amount NEW '
      'arrival after the shield mined away still fires', () async {
    final h = harness();
    await settle();
    expect(h.fake.sendCount, 1, reason: 'first arrival handed off');

    // The shield mines: the figure drops to 0 (a differing observation).
    h.fake.setSnapshot(
      walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 101),
        balance: balanceFixture(
          spendableZat: 700000,
          totalZat: 700000,
          transparentZat: 0,
        ),
      ),
    );
    h.container.invalidate(walletSnapshotReadProvider);
    await settle();
    expect(h.fake.sendCount, 1, reason: 'nothing to shield at 0');

    // A NEW arrival of the EXACT same amount as the latched handoff.
    h.fake.setSnapshot(
      walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 102),
        balance: balanceFixture(
          spendableZat: 700000,
          totalZat: 900000,
          transparentZat: 200000,
        ),
      ),
    );
    h.container.invalidate(walletSnapshotReadProvider);
    await settle();
    expect(
      h.fake.sendCount,
      2,
      reason: 'a stale latch must never eat a same-amount new arrival',
    );
  });

  test(
    'at-most-one attempt in flight: an edge landing mid-attempt is a no-op',
    () async {
      final fake = fakeWithTransparent(200000)
        ..proposeShieldGate = Completer<void>();
      final h = harness(session: fake);
      await settle();
      expect(h.fake.proposeShieldCount, 1, reason: 'parked at the gate');

      h.container.invalidate(walletSnapshotReadProvider);
      await settle();
      expect(
        h.fake.proposeShieldCount,
        1,
        reason: 'the in-flight slot blocks a second attempt',
      );

      fake.proposeShieldGate!.complete();
      await settle();
      expect(h.fake.sendCount, 1, reason: 'the parked attempt completed');
    },
  );

  test('the manual shield flow WINS: the loop skips an evaluation while the '
      'sheet\'s flow is in flight', () async {
    final fake = fakeWithTransparent(200000)
      ..proposeShieldGate = Completer<void>();
    // Power-save ON keeps the loop quiet while the MANUAL flow takes the gate.
    final h = harness(session: fake, powerSave: true);
    await settle();

    // The user opens the sheet: prepare() parks at the same gate.
    unawaited(h.container.read(shieldControllerProvider.notifier).prepare());
    await settle();
    expect(h.container.read(shieldControllerProvider), isA<ShieldPreparing>());
    expect(h.fake.proposeShieldCount, 1, reason: 'the manual propose only');

    // Power-save lifts mid-manual-flow: the loop must YIELD, not race.
    h.container.read(h.powerSwitch.notifier).state = false;
    await settle();
    expect(
      h.fake.proposeShieldCount,
      1,
      reason: 'no auto propose while the sheet flow runs',
    );
  });

  test('IDENTITY-FLIP FENCE: an approval landing after a session flip spends '
      'on NEITHER session', () async {
    final gate = Completer<void>();
    final auth = FakeSendAuthorizer(prompt: (_) => gate.future);
    final first = fakeWithTransparent(200000);
    final second = fakeWithTransparent(200000);
    final sessionSwitch = StateProvider<WalletSession?>((ref) => first);
    final settings = FakeWalletSettingsStore();
    final powerSwitch = StateProvider<bool>((ref) => false);
    final container = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWith((ref) => ref.watch(sessionSwitch)),
        walletSendAuthorizerProvider.overrideWithValue(auth),
        walletSettingsStoreProvider.overrideWithValue(settings),
        walletPowerSaveActiveProvider.overrideWith(
          (ref) => ref.watch(powerSwitch),
        ),
      ],
    );
    addTearDown(container.dispose);
    container.listen(walletAutoShieldControllerProvider, (_, _) {});
    Future<void> settleLocal() async {
      for (var i = 0; i < 10; i++) {
        await Future<void>.delayed(Duration.zero);
      }
    }

    await settleLocal();
    expect(first.proposeShieldCount, 1, reason: 'parked at the host prompt');

    // The host flips identity mid-prompt, THEN the approval lands.
    container.read(sessionSwitch.notifier).state = second;
    await settleLocal();
    gate.complete();
    await settleLocal();

    expect(first.sendCount, 0, reason: 'the DEAD identity must not spend');
    // The new identity's own build re-evaluated with its own session — its
    // attempt is legitimate and separately authorized; the fenced approval
    // itself committed nothing (NIT-8: the NEW session's loop must be
    // ALIVE, not just the old one fenced).
    expect(
      second.sendCount,
      1,
      reason: 'the NEW session\'s own attempt authorizes + spends normally',
    );
  });

  test(
    'a DENIAL landing after a session flip does NOT disarm the NEW '
    'session\'s loop (S154 MAJOR-1 — probe-confirmed by two reviewers)',
    () async {
      // The documented host behavior: deny outstanding prompts at an identity
      // switch. That stale denial belongs to the DEAD session — before the
      // generation guard, it silently latched the NEW session's loop off with
      // no cue (arrivals stayed public until restart).
      final gate = Completer<void>();
      var calls = 0;
      final auth = FakeSendAuthorizer(
        prompt: (_) {
          calls++;
          // Only the FIRST attempt (session A's) parks; the new session's own
          // attempts pass straight through.
          return calls == 1 ? gate.future : Future<void>.value();
        },
      );
      final first = fakeWithTransparent(200000);
      final second = fakeWithTransparent(200000);
      final sessionSwitch = StateProvider<WalletSession?>((ref) => first);
      final container = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWith((ref) => ref.watch(sessionSwitch)),
          walletSendAuthorizerProvider.overrideWithValue(auth),
          walletSettingsStoreProvider.overrideWithValue(
            FakeWalletSettingsStore(),
          ),
          walletPowerSaveActiveProvider.overrideWith((ref) => false),
        ],
      );
      addTearDown(container.dispose);
      container.listen(walletAutoShieldControllerProvider, (_, _) {});
      Future<void> settleLocal() async {
        for (var i = 0; i < 10; i++) {
          await Future<void>.delayed(Duration.zero);
        }
      }

      await settleLocal();
      expect(first.proposeShieldCount, 1, reason: 'A parked at the prompt');

      // Flip, then the host denies A's outstanding prompt.
      container.read(sessionSwitch.notifier).state = second;
      await settleLocal();
      gate.completeError(const WalletSpendAuthorizationDenied());
      await settleLocal();

      expect(
        container.read(walletAutoShieldControllerProvider),
        isNot(AutoShieldStatus.denied),
        reason: 'the stale denial must not shape the NEW session\'s status',
      );
      // The NEW session's loop stays ARMED: a fresh arrival still shields.
      second.setSnapshot(
        walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: 103),
          balance: balanceFixture(
            spendableZat: 500000,
            totalZat: 800000,
            transparentZat: 300000,
          ),
        ),
      );
      container.invalidate(walletSnapshotReadProvider);
      await settleLocal();
      expect(
        second.sendCount,
        greaterThanOrEqualTo(1),
        reason: 'the new identity\'s automation survived the stale denial',
      );
    },
  );

  test('PRE-PROMPT fence: a flip landing during the propose await means the '
      'DEAD identity\'s prompt NEVER opens (S154 security MAJOR-2)', () async {
    // The spend fence fires inside the action — AFTER the prompt. Without the
    // pre-prompt re-check, a prompting host would render the dead identity\'s
    // amount over the new identity\'s screen (a duress disclosure).
    final auth = FakeSendAuthorizer();
    final first = fakeWithTransparent(200000)
      ..proposeShieldGate = Completer<void>();
    // The second session has NOTHING transparent — its own loop stays quiet,
    // so any intent that appears could only be the dead identity\'s.
    final second = fakeWithTransparent(0);
    final sessionSwitch = StateProvider<WalletSession?>((ref) => first);
    final container = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWith((ref) => ref.watch(sessionSwitch)),
        walletSendAuthorizerProvider.overrideWithValue(auth),
        walletSettingsStoreProvider.overrideWithValue(
          FakeWalletSettingsStore(),
        ),
        walletPowerSaveActiveProvider.overrideWith((ref) => false),
      ],
    );
    addTearDown(container.dispose);
    container.listen(walletAutoShieldControllerProvider, (_, _) {});
    Future<void> settleLocal() async {
      for (var i = 0; i < 10; i++) {
        await Future<void>.delayed(Duration.zero);
      }
    }

    await settleLocal();
    expect(first.proposeShieldCount, 1, reason: 'A parked inside propose');

    container.read(sessionSwitch.notifier).state = second;
    await settleLocal();
    first.proposeShieldGate!.complete();
    await settleLocal();

    expect(
      auth.intents,
      isEmpty,
      reason: 'no authorization prompt may open for the dead identity',
    );
    expect(first.sendCount, 0);
    expect(second.sendCount, 0);
  });

  test('S205-b: a supported→false flip landing INSIDE the propose round-trip '
      'never opens the automatic-spend prompt — the SUPPORTED flag joins the '
      'pre-prompt same-breath re-checks', () async {
    // The seam is documented static-per-scope, but a host that wires it
    // reactively must not get a prompt (or, on a passthrough authorizer, a
    // spend) for a capability it just declared unsupported. The flip does NOT
    // rebuild the controller (build() watches only the session), so the ONLY
    // guard here is the pre-prompt supported re-check the fold added.
    final auth = FakeSendAuthorizer();
    final fake = fakeWithTransparent(200000)
      ..proposeShieldGate = Completer<void>();
    final supportedSwitch = StateProvider<bool>((ref) => true);
    final container = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWithValue(fake),
        walletSendAuthorizerProvider.overrideWithValue(auth),
        walletSettingsStoreProvider.overrideWithValue(
          FakeWalletSettingsStore(),
        ),
        walletPowerSaveActiveProvider.overrideWith((ref) => false),
        walletAutoShieldSupportedProvider.overrideWith(
          (ref) => ref.watch(supportedSwitch),
        ),
      ],
    );
    addTearDown(container.dispose);
    container.listen(walletAutoShieldControllerProvider, (_, _) {});
    Future<void> settleLocal() async {
      for (var i = 0; i < 10; i++) {
        await Future<void>.delayed(Duration.zero);
      }
    }

    await settleLocal();
    expect(fake.proposeShieldCount, 1, reason: 'parked inside propose');

    // The host declares automatic spends unsupported mid-round-trip, THEN
    // the propose settles.
    container.read(supportedSwitch.notifier).state = false;
    await settleLocal();
    fake.proposeShieldGate!.complete();
    await settleLocal();

    expect(
      auth.intents,
      isEmpty,
      reason:
          'no prompt may open for a host that just declared automatic '
          'spends unsupported (the S154 MINOR-1 rationale, supported flag)',
    );
    expect(
      fake.sendCount,
      0,
      reason: 'a passthrough authorizer must not spend',
    );
  });

  test('turning the switch back ON re-arms a host-denied loop (S154 m6) — '
      'and OFF clears a lingering failure cue (m5)', () async {
    final auth = FakeSendAuthorizer(denyAll: true);
    final h = harness(authorizer: auth);
    await settle();
    expect(
      h.container.read(walletAutoShieldControllerProvider),
      AutoShieldStatus.denied,
    );

    // The host policy now allows; the user's OFF→ON is the explicit restart.
    auth.denyAll = false;
    await h.container
        .read(walletAutoShieldEnabledProvider.notifier)
        .set(enabled: false);
    await settle();
    expect(
      h.container.read(walletAutoShieldControllerProvider),
      AutoShieldStatus.idle,
      reason:
          'a deliberate OFF clears the cue (it would contradict the '
          'explicit opt-out)',
    );
    await h.container
        .read(walletAutoShieldEnabledProvider.notifier)
        .set(enabled: true);
    await settle();
    expect(
      h.fake.sendCount,
      1,
      reason: 'ON re-arms the denial latch and re-evaluates',
    );
  });

  test('a busy/not-synced propose during startup stays QUIET (idle, no cue) '
      'and the next edge retries (S154 m8)', () async {
    final fake = fakeWithTransparent(200000)
      ..proposeShieldThrows = const WalletApiError(
        code: 'RW-TEST',
        message: 'static',
        kind: WalletErrorKind.walletBusy(phase: LifecyclePhase.closing),
      );
    final h = harness(session: fake);
    await settle();

    expect(
      h.container.read(walletAutoShieldControllerProvider),
      AutoShieldStatus.idle,
      reason: 'a cold-start busy is a normal transient, not a failure cue',
    );

    fake.proposeShieldThrows = null;
    h.container.invalidate(walletSnapshotReadProvider);
    await settle();
    expect(h.fake.sendCount, 1, reason: 'the next edge completed normally');
  });

  test('STRICT-b (S155 wrap): a stale denial after the flip must not eat the '
      'NEW session\'s POST-denial arrivals — asserts the INCREMENT, not >=1 '
      '(the earlier pin passed with or without the guard)', () async {
    final gate = Completer<void>();
    var calls = 0;
    final auth = FakeSendAuthorizer(
      prompt: (_) {
        calls++;
        return calls == 1 ? gate.future : Future<void>.value();
      },
    );
    final first = fakeWithTransparent(200000);
    final second = fakeWithTransparent(200000);
    final sessionSwitch = StateProvider<WalletSession?>((ref) => first);
    final container = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWith((ref) => ref.watch(sessionSwitch)),
        walletSendAuthorizerProvider.overrideWithValue(auth),
        walletSettingsStoreProvider.overrideWithValue(
          FakeWalletSettingsStore(),
        ),
      ],
    );
    addTearDown(container.dispose);
    container.listen(walletAutoShieldControllerProvider, (_, _) {});
    await settle();
    expect(first.proposeShieldCount, 1, reason: 'A parked at the prompt');

    container.read(sessionSwitch.notifier).state = second;
    await settle();
    // B's own pre-denial attempt may already have completed — RECORD it.
    final sendsBeforeDenial = second.sendCount;

    gate.completeError(const WalletSpendAuthorizationDenied());
    await settle();
    expect(
      container.read(walletAutoShieldControllerProvider),
      isNot(AutoShieldStatus.denied),
    );

    // THE PIN: an arrival AFTER the stale denial still shields.
    second.setSnapshot(
      walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 103),
        balance: balanceFixture(
          spendableZat: 500000,
          totalZat: 800000,
          transparentZat: 300000,
        ),
      ),
    );
    container.invalidate(walletSnapshotReadProvider);
    await settle();
    expect(
      second.sendCount,
      sendsBeforeDenial + 1,
      reason:
          'the POST-denial edge must fire — a stale denial latching the '
          'NEW session leaves arrivals public with no cue',
    );
  });

  test('STRICT-d (S155 wrap): the dead cycle\'s finally releases ONLY ITS OWN '
      'slot — never the NEW cycle\'s in-flight slot', () async {
    final promptGate = Completer<void>();
    var calls = 0;
    final auth = FakeSendAuthorizer(
      prompt: (_) {
        calls++;
        return calls == 1 ? promptGate.future : Future<void>.value();
      },
    );
    final first = fakeWithTransparent(200000);
    final second = fakeWithTransparent(200000)
      ..proposeShieldGate = Completer<void>();
    final sessionSwitch = StateProvider<WalletSession?>((ref) => first);
    final container = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWith((ref) => ref.watch(sessionSwitch)),
        walletSendAuthorizerProvider.overrideWithValue(auth),
        walletSettingsStoreProvider.overrideWithValue(
          FakeWalletSettingsStore(),
        ),
      ],
    );
    addTearDown(container.dispose);
    container.listen(walletAutoShieldControllerProvider, (_, _) {});
    await settle();
    expect(first.proposeShieldCount, 1, reason: 'A parked at its prompt');

    // Flip: B's own attempt takes the gen-2 slot and parks inside propose.
    container.read(sessionSwitch.notifier).state = second;
    await settle();
    expect(second.proposeShieldCount, 1, reason: 'B parked inside propose');

    // A's stale prompt is denied → A's finally runs while B is in flight.
    promptGate.completeError(const WalletSpendAuthorizationDenied());
    await settle();

    // An edge lands while B's FIRST attempt is still parked: the slot must
    // still be held. An unconditional release would open a SECOND concurrent
    // attempt = double propose/prompt.
    container.invalidate(walletSnapshotReadProvider);
    await settle();
    expect(
      second.proposeShieldCount,
      1,
      reason: 'at-most-one must survive the dead cycle\'s finally',
    );

    second.proposeShieldGate!.complete();
    await settle();
    expect(second.sendCount, 1, reason: 'exactly one attempt completed');
  });

  test('the pre-prompt fence honors a switch-OFF landing mid-propose — no '
      'prompt, no spend against a just-persisted hold-transparent choice '
      '(S155 wrap security MINOR-1)', () async {
    final fake = fakeWithTransparent(200000)
      ..proposeShieldGate = Completer<void>();
    final h = harness(session: fake);
    await settle();
    expect(h.fake.proposeShieldCount, 1, reason: 'parked inside propose');

    await h.container
        .read(walletAutoShieldEnabledProvider.notifier)
        .set(enabled: false);
    fake.proposeShieldGate!.complete();
    await settle();

    expect(h.auth.intents, isEmpty, reason: 'no prompt after the OFF landed');
    expect(h.fake.sendCount, 0, reason: 'nothing spent against the opt-out');
  });

  test('a late in-flight outcome cannot resurrect a cue over an OFF switch '
      '(S155 wrap security MINOR-2)', () async {
    final gate = Completer<void>();
    final auth = FakeSendAuthorizer(prompt: (_) => gate.future);
    final h = harness(authorizer: auth);
    await settle();
    expect(auth.intents, hasLength(1), reason: 'parked at the prompt');

    // The user opts out while the attempt is parked; then the host denies.
    await h.container
        .read(walletAutoShieldEnabledProvider.notifier)
        .set(enabled: false);
    await settle();
    gate.completeError(const WalletSpendAuthorizationDenied());
    await settle();

    expect(
      h.container.read(walletAutoShieldControllerProvider),
      AutoShieldStatus.idle,
      reason: 'the card promises no cue over a deliberate opt-out',
    );
  });

  test(
    'a provider REBUILD re-emitting true does not re-arm a denied loop — '
    'only a real OFF→ON transition does (S155 wrap security MINOR-3)',
    () async {
      final auth = FakeSendAuthorizer(denyAll: true);
      final h = harness(authorizer: auth);
      await settle();
      expect(auth.intents, hasLength(1));
      expect(
        h.container.read(walletAutoShieldControllerProvider),
        AutoShieldStatus.denied,
      );

      // A rebuild (host store swap / any invalidate) re-emits value=true
      // through loading-with-previous — NOT a user gesture.
      h.container.invalidate(walletAutoShieldEnabledProvider);
      await settle();
      expect(
        auth.intents,
        hasLength(1),
        reason: 'no re-prompt without a user-driven flip',
      );
    },
  );

  test('a SEND-phase busy failure cues (the transient quieting is '
      'propose-phase only — S155 wrap security MINOR-4)', () async {
    final fake = fakeWithTransparent(200000)
      ..sendThrows = const WalletApiError(
        code: 'RW-TEST',
        message: 'static',
        kind: WalletErrorKind.walletBusy(phase: LifecyclePhase.closing),
      );
    final h = harness(session: fake);
    await settle();

    expect(
      h.container.read(walletAutoShieldControllerProvider),
      AutoShieldStatus.failed,
      reason: 'past the propose phase, busy is a real didn\'t-complete',
    );
  });

  test('a LOADING snapshot still carrying the PREVIOUS session\'s figure is '
      'skipped — the dead identity\'s balance cannot pass the new identity\'s '
      'threshold (S154 security MINOR-4)', () async {
    final auth = FakeSendAuthorizer();
    final first = fakeWithTransparent(200000);
    // The new session's REAL balance is below the raised threshold.
    final second = fakeWithTransparent(60000);
    final gate = Completer<void>();
    final sessionSwitch = StateProvider<WalletSession?>((ref) => first);
    // Session A resolves instantly (so its data becomes the "previous" the
    // reload carries); session B's snapshot parks at the gate. A LOCAL raw
    // FutureProvider deliberately bypasses the view's #381 identity fence —
    // this test pins auto-shield's OWN stale-figure guard (defence in depth
    // behind the fence), so the stale-carrying shape must be reproducible.
    final rawSnapshot = FutureProvider<WalletState>((ref) {
      final s = ref.watch(sessionSwitch);
      if (identical(s, second)) {
        return gate.future.then((_) => second.snapshot());
      }
      return first.snapshot();
    });
    final container = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWith((ref) => ref.watch(sessionSwitch)),
        walletSendAuthorizerProvider.overrideWithValue(auth),
        walletSettingsStoreProvider.overrideWithValue(
          FakeWalletSettingsStore(),
        ),
        walletPowerSaveActiveProvider.overrideWith((ref) => false),
        walletAutoShieldThresholdZatProvider.overrideWithValue(150000),
        walletSnapshotProvider.overrideWith(
          () => _RawSnapshotView(rawSnapshot),
        ),
      ],
    );
    addTearDown(container.dispose);
    container.listen(walletAutoShieldControllerProvider, (_, _) {});
    Future<void> settleLocal() async {
      for (var i = 0; i < 10; i++) {
        await Future<void>.delayed(Duration.zero);
      }
    }

    await settleLocal();
    expect(first.proposeShieldCount, 1, reason: 'A fires over its 200k');
    expect(first.sendCount, 1);

    // Flip: B's snapshot is LOADING while riverpod still carries A's figure.
    container.read(sessionSwitch.notifier).state = second;
    await settleLocal();
    expect(
      second.proposeShieldCount,
      0,
      reason: 'no evaluation on the stale 200k while B\'s read is in flight',
    );

    gate.complete();
    await settleLocal();
    expect(
      second.proposeShieldCount,
      0,
      reason: 'B\'s real 60k is below the 150k threshold — still quiet',
    );
  });

  test('a LOADING auto-shield FLAG carrying the PREVIOUS identity\'s ON is '
      'skipped on a warm reload — the settings analog of the snapshot guard, '
      'newly needed once settings became per-identity (S172 review M2)', () async {
    final auth = FakeSendAuthorizer();
    final first = fakeWithTransparent(200000); // A: over threshold
    final second = fakeWithTransparent(200000); // B: ALSO over threshold
    final flagGate = Completer<void>();
    final sessionSwitch = StateProvider<WalletSession?>((ref) => first);
    // A's store: auto-shield ON, resolves instantly → the flag provider's
    // "previous" value the reload carries.
    final storeA = FakeWalletSettingsStore(autoShieldValue: true);
    // B's store: auto-shield OFF (a hold-transparent decoy), but its READ is
    // gated so during the switch window the flag provider is AsyncLoading while
    // still carrying A's previous `true`.
    final storeB = FakeWalletSettingsStore(autoShieldValue: false)
      ..readGate = flagGate;
    final container = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWith((ref) => ref.watch(sessionSwitch)),
        walletSendAuthorizerProvider.overrideWithValue(auth),
        walletSettingsStoreProvider.overrideWith((ref) {
          final s = ref.watch(sessionSwitch);
          return identical(s, second) ? storeB : storeA;
        }),
        walletPowerSaveActiveProvider.overrideWith((ref) => false),
        // Both snapshots resolve WITHOUT a gate, so the snapshot guard is not
        // what stops B — the flag guard is. (Isolates the M2 fix: at settle the
        // snapshot is RESOLVED over threshold while the flag is still loading.)
        walletSnapshotProvider.overrideWith(
          () => _RawSnapshotView(_rawSwitchedSnapshotProvider(sessionSwitch)),
        ),
      ],
    );
    addTearDown(container.dispose);
    container.listen(walletAutoShieldControllerProvider, (_, _) {});
    Future<void> settleLocal() async {
      for (var i = 0; i < 10; i++) {
        await Future<void>.delayed(Duration.zero);
      }
    }

    await settleLocal();
    expect(first.proposeShieldCount, 1, reason: 'A is ON and over threshold');

    // Flip to B: the store re-keys to storeB (gated read) → the flag provider is
    // LOADING while carrying A's previous `true`; B's snapshot is resolved and
    // over threshold. WITHOUT the isLoading guard, the loop reads the stale
    // `true` and shields B's funds against B's real OFF policy.
    container.read(sessionSwitch.notifier).state = second;
    await settleLocal();
    expect(
      second.proposeShieldCount,
      0,
      reason: 'the stale loading flag never ACTS — the M2 guard held',
    );

    // B's real flag resolves to OFF → its true policy is honored: still quiet.
    flagGate.complete();
    await settleLocal();
    expect(second.proposeShieldCount, 0);
  });
}

/// A raw switched-session snapshot reader for view overrides — deliberately
/// UNFENCED (a plain FutureProvider family over the test's session switch),
/// so these tests exercise auto-shield's OWN guards behind the #381 fence.
final _rawSwitchedSnapshotProvider = FutureProvider.autoDispose
    .family<WalletState, StateProvider<WalletSession?>>(
      (ref, sessionSwitch) =>
          (ref.watch(sessionSwitch)! as FakeWalletSession).snapshot(),
    );

/// A snapshot VIEW mirroring a fixed raw reader instead of the identity-keyed
/// family — the override vehicle for the raw readers above.
class _RawSnapshotView extends WalletSnapshotViewNotifier {
  _RawSnapshotView(this._raw);

  final FutureProvider<WalletState> _raw;

  @override
  FutureProvider<WalletState> read(Object? identity) => _raw;
}
