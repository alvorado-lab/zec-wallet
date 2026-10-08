import 'dart:async' show Completer;

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'package:fake_async/fake_async.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_activation.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_config.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_enabled_provider.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// The one-time swap activation (`enableNearSwap`) + the derived runtime gate.
/// Driven on the host VM against the fake — no native lib, no network. The
/// load-bearing properties: the enable fires ONLY for an open wallet under an
/// enabled policy, EVERY failure mode degrades honestly to OFF (never an
/// exception to the UI), and the gate reflects the real enable OUTCOME.
void main() {
  const policy = SwapHostPolicy(
    config: SwapProviderConfig(endpoint: 'https://1click.test', jwt: 'tok'),
    enabled: true,
    declaredKill: SwapKill.windDown,
  );

  SwapApiError err(SwapErrorKind kind) =>
      SwapApiError(code: 'RW-TEST', message: 'x', kind: kind);

  ProviderContainer harness({
    WalletSession? session,
    SwapHostPolicy? hostPolicy,
  }) {
    final container = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWithValue(session),
        swapHostPolicyProvider.overrideWithValue(hostPolicy),
      ],
    );
    addTearDown(container.dispose);
    return container;
  }

  group('no enable attempt (honest no-op)', () {
    test(
      'no wallet session → false (the money-safety gate; no enable)',
      () async {
        // No fake is wired (session is null), so "no provider call" is structural:
        // there is no session to enable. The gate must read false.
        final c = harness(session: null, hostPolicy: policy);
        expect(await c.read(swapActivationProvider.future), isFalse);
      },
    );

    test('no policy → false, no provider call', () async {
      final fake = FakeWalletSession();
      final c = harness(session: fake, hostPolicy: null);
      expect(await c.read(swapActivationProvider.future), isFalse);
      expect(fake.enableNearSwapCount, 0);
    });

    test('host-disabled policy → false, no provider call', () async {
      final fake = FakeWalletSession();
      const off = SwapHostPolicy(
        config: SwapProviderConfig(endpoint: 'https://e'),
        enabled: false,
      );
      final c = harness(session: fake, hostPolicy: off);
      expect(await c.read(swapActivationProvider.future), isFalse);
      expect(fake.enableNearSwapCount, 0);
    });
  });

  group('enable attempt', () {
    test('success → true, passes the policy through verbatim', () async {
      final fake = FakeWalletSession();
      final c = harness(session: fake, hostPolicy: policy);
      expect(await c.read(swapActivationProvider.future), isTrue);
      expect(fake.enableNearSwapCount, 1);
      expect(fake.lastEnableConfig, policy.config);
      expect(fake.lastEnableSwapEnabled, isTrue);
      expect(fake.lastEnableDeclaredKill, SwapKill.windDown);
    });

    test('SwapDisabled (no adapter compiled in) degrades to false', () async {
      final fake = FakeWalletSession()
        ..enableNearSwapThrows = err(const SwapErrorKind.swapDisabled());
      final c = harness(session: fake, hostPolicy: policy);
      expect(await c.read(swapActivationProvider.future), isFalse);
      expect(fake.enableNearSwapCount, 1);
    });

    test(
      'providerUnavailable (bad endpoint/transport) degrades to false',
      () async {
        final fake = FakeWalletSession()
          ..enableNearSwapThrows = err(
            const SwapErrorKind.providerUnavailable(),
          );
        final c = harness(session: fake, hostPolicy: policy);
        expect(await c.read(swapActivationProvider.future), isFalse);
      },
    );

    test(
      'watchOnly (structural RW-SWAP-015, #397 D3/S234) degrades to false — '
      'the typed refusal composes with the chrome that already hides swap',
      () async {
        final fake = FakeWalletSession()
          ..enableNearSwapThrows = err(const SwapErrorKind.watchOnly());
        final c = harness(session: fake, hostPolicy: policy);
        expect(await c.read(swapActivationProvider.future), isFalse);
        expect(fake.enableNearSwapCount, 1);
      },
    );

    test(
      'an UNTYPED bridge failure still degrades to false (never throws up)',
      () async {
        // A non-SwapApiError throw (an FRB panic / generic bridge error outside the
        // typed contract) must still resolve swap to OFF — never an unhandled
        // provider error. "Every failure mode resolves to off."
        final fake = FakeWalletSession()
          ..enableNearSwapThrows = StateError('bridge panic');
        final c = harness(session: fake, hostPolicy: policy);
        expect(await c.read(swapActivationProvider.future), isFalse);
      },
    );
  });

  group('re-fire on a new wallet handle', () {
    test(
      'a session close→reopen re-runs the enable (set-once per handle)',
      () async {
        final fake = FakeWalletSession();
        final holder = StateProvider<WalletSession?>((ref) => fake);
        final container = ProviderContainer(
          overrides: [
            walletSessionProvider.overrideWith((ref) => ref.watch(holder)),
            swapHostPolicyProvider.overrideWithValue(policy),
          ],
        );
        addTearDown(container.dispose);

        expect(await container.read(swapActivationProvider.future), isTrue);
        expect(fake.enableNearSwapCount, 1);

        container.read(holder.notifier).state = null; // closed
        expect(await container.read(swapActivationProvider.future), isFalse);

        container.read(holder.notifier).state = fake; // reopened → re-enable
        expect(await container.read(swapActivationProvider.future), isTrue);
        expect(
          fake.enableNearSwapCount,
          2,
          reason: 'a new handle needs its own enable',
        );
      },
    );

    test(
      're-setting the SAME session instance does NOT re-enable (no spam)',
      () async {
        // Real-world: the provider graph rebuilds for unrelated reasons (theme,
        // lifecycle). `swapActivationProvider` watches the session by value, so an
        // unchanged session instance must NOT re-run the enable — otherwise an
        // unstable/churny client would hammer `enableNearSwap`.
        final fake = FakeWalletSession();
        final holder = StateProvider<WalletSession?>((ref) => fake);
        final container = ProviderContainer(
          overrides: [
            walletSessionProvider.overrideWith((ref) => ref.watch(holder)),
            swapHostPolicyProvider.overrideWithValue(policy),
          ],
        );
        addTearDown(container.dispose);

        expect(await container.read(swapActivationProvider.future), isTrue);
        expect(fake.enableNearSwapCount, 1);

        // Same instance again → no provider notification → no re-enable.
        container.read(holder.notifier).state = fake;
        await container.read(swapActivationProvider.future);
        expect(
          fake.enableNearSwapCount,
          1,
          reason: 'unchanged session must not re-enable',
        );
      },
    );

    test(
      'a degraded enable RECOVERS on a wallet reopen (unstable-network)',
      () async {
        // The provider/transport was unavailable at first open (e.g. Tor not up);
        // swap stays off for that session. A later wallet reopen re-attempts the
        // enable on a fresh handle and, when the transport is back, swap recovers.
        final fake = FakeWalletSession()
          ..enableNearSwapThrows = err(
            const SwapErrorKind.providerUnavailable(),
          );
        final holder = StateProvider<WalletSession?>((ref) => fake);
        final container = ProviderContainer(
          overrides: [
            walletSessionProvider.overrideWith((ref) => ref.watch(holder)),
            swapHostPolicyProvider.overrideWithValue(policy),
          ],
        );
        addTearDown(container.dispose);

        expect(
          await container.read(swapActivationProvider.future),
          isFalse,
          reason: 'transport down at open → swap off this session',
        );

        // Wallet reopen (a new handle), transport now up.
        container.read(holder.notifier).state = null;
        await container.read(swapActivationProvider.future);
        fake.enableNearSwapThrows = null; // provider recovered
        container.read(holder.notifier).state = fake;
        expect(
          await container.read(swapActivationProvider.future),
          isTrue,
          reason: 'a reopen recovers swap once the transport is back',
        );
      },
    );
  });

  group('swapEnabledProvider derives from the outcome', () {
    test('true only after a successful enable', () async {
      final fake = FakeWalletSession();
      final c = harness(session: fake, hostPolicy: policy);
      // Before the future resolves the gate is OFF (no optimistic flash).
      expect(c.read(swapEnabledProvider), isFalse);
      await c.read(swapActivationProvider.future);
      expect(c.read(swapEnabledProvider), isTrue);
    });

    test('false when the enable degrades', () async {
      final fake = FakeWalletSession()
        ..enableNearSwapThrows = err(const SwapErrorKind.swapDisabled());
      final c = harness(session: fake, hostPolicy: policy);
      await c.read(swapActivationProvider.future);
      expect(c.read(swapEnabledProvider), isFalse);
    });

    test('a HUNG enable degrades to off at the 30s budget (S175 — the reserved '
        'action-row slot must never be held indefinitely)', () {
      fakeAsync((async) {
        final fake = _HangingEnableSession();
        final c = harness(session: fake, hostPolicy: policy);
        bool? outcome;
        // ignore: avoid_types_on_closure_parameters
        c.listen<AsyncValue<bool>>(swapActivationProvider, (_, next) {
          if (next is AsyncData<bool>) outcome = next.value;
        }, fireImmediately: true);
        async.flushMicrotasks();
        expect(outcome, isNull, reason: 'still in flight before the budget');
        expect(c.read(swapSlotProvider), SwapSlot.reserved);

        async.elapse(const Duration(seconds: 30));
        async.flushMicrotasks();
        expect(outcome, isFalse, reason: 'the deadline degrades to OFF');
        expect(c.read(swapEnabledProvider), isFalse);
        expect(c.read(swapSlotProvider), SwapSlot.absent);
      });
    });
  });
}

/// A session whose enable never completes — the stalled-transport shape the
/// activation budget exists for.
class _HangingEnableSession extends FakeWalletSession {
  @override
  Future<void> enableNearSwap({
    required SwapProviderConfig config,
    required bool swapEnabled,
    SwapKill? declaredKill,
  }) {
    enableNearSwapCount++;
    return Completer<void>().future;
  }
}
