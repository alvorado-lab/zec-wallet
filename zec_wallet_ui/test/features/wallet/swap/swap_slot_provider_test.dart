import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart' show StateProvider;
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_activation.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_config.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_enabled_provider.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// The action row's third-slot tri-state (#356-F3). Load-bearing properties:
/// a policy-enabled host RESERVES the slot while the activation outcome is
/// pending (so the Swap button's arrival can't shift Send/Receive mid-tap),
/// only a RESOLVED outcome fills (`live`) or collapses (`absent`) it, and a
/// host with swap off never gets a slot at all. `live` derives from the
/// [swapEnabledProvider] gate itself, so the existing test/host seam (override
/// the gate directly) drives the slot too.
void main() {
  const policy = SwapHostPolicy(
    config: SwapProviderConfig(endpoint: 'https://1click.test', jwt: 'tok'),
    enabled: true,
    declaredKill: SwapKill.windDown,
  );

  ProviderContainer harness(List<Override> overrides) {
    final container = ProviderContainer(overrides: overrides);
    addTearDown(container.dispose);
    return container;
  }

  test(
    'no host policy → absent (no slot — the §3.5 removed-feature posture)',
    () {
      final c = harness([swapHostPolicyProvider.overrideWithValue(null)]);
      expect(c.read(swapSlotProvider), SwapSlot.absent);
    },
  );

  test('host-disabled policy → absent', () {
    const disabled = SwapHostPolicy(
      config: SwapProviderConfig(endpoint: 'https://1click.test', jwt: 'tok'),
      enabled: false,
      declaredKill: SwapKill.windDown,
    );
    final c = harness([swapHostPolicyProvider.overrideWithValue(disabled)]);
    expect(c.read(swapSlotProvider), SwapSlot.absent);
  });

  test('policy enabled + activation PENDING → reserved (hold the geometry, '
      'show no button)', () {
    final c = harness([
      swapHostPolicyProvider.overrideWithValue(policy),
      // A never-completing enable — the in-flight window the slot exists for.
      swapActivationProvider.overrideWith((ref) => Completer<bool>().future),
    ]);
    expect(c.read(swapSlotProvider), SwapSlot.reserved);
    // The GATE stays honest-false through the whole reserved window.
    expect(c.read(swapEnabledProvider), isFalse);
  });

  test('activation resolved TRUE → live (via the real gate)', () async {
    final fake = FakeWalletSession();
    final c = harness([
      walletSessionProvider.overrideWithValue(fake),
      swapHostPolicyProvider.overrideWithValue(policy),
    ]);
    await c.read(swapActivationProvider.future);
    expect(c.read(swapSlotProvider), SwapSlot.live);
  });

  test('activation resolved FALSE → absent (the one-shot collapse to the '
      'truthful two-button row)', () async {
    // No session ⇒ the activation resolves false without an enable call.
    final c = harness([
      walletSessionProvider.overrideWithValue(null),
      swapHostPolicyProvider.overrideWithValue(policy),
    ]);
    await c.read(swapActivationProvider.future);
    expect(c.read(swapSlotProvider), SwapSlot.absent);
  });

  test('a direct gate override drives live (the existing test/host seam keeps '
      'working — the gate is the SSOT)', () {
    final c = harness([swapEnabledProvider.overrideWithValue(true)]);
    expect(c.read(swapSlotProvider), SwapSlot.live);
  });

  test('a session flip re-RESERVES a live slot synchronously (the Riverpod '
      'reload-semantics tripwire the whole tri-state hinges on)', () async {
    final sessionSwitch = StateProvider<FakeWalletSession?>(
      (ref) => FakeWalletSession(),
    );
    final c = harness([
      walletSessionProvider.overrideWith((ref) => ref.watch(sessionSwitch)),
      swapHostPolicyProvider.overrideWithValue(policy),
    ]);
    await c.read(swapActivationProvider.future);
    expect(c.read(swapSlotProvider), SwapSlot.live);

    // The flip re-fires the activation: an AsyncLoading CARRYING the
    // previous `true`. Synchronously — before the new enable settles — the
    // gate must read false and the slot must re-reserve. If a future
    // Riverpod made dependency-change rebuilds seamless (an AsyncData
    // carrying isLoading), the gate would read stale-true and the slot
    // would show a live button over a false gate: THIS test is the tripwire.
    c.read(sessionSwitch.notifier).state = FakeWalletSession();
    expect(c.read(swapEnabledProvider), isFalse);
    expect(c.read(swapSlotProvider), SwapSlot.reserved);

    // The new handle's enable settles → live again, same slot.
    await c.read(swapActivationProvider.future);
    expect(c.read(swapSlotProvider), SwapSlot.live);
  });

  test(
    'a session flip on a resolved-OFF host stays ABSENT through the re-fire '
    '(no empty-slot flash on every reopen), then settles truthfully',
    () async {
      // No session ⇒ the first activation resolves FALSE without an enable.
      final sessionSwitch = StateProvider<FakeWalletSession?>((ref) => null);
      final c = harness([
        walletSessionProvider.overrideWith((ref) => ref.watch(sessionSwitch)),
        swapHostPolicyProvider.overrideWithValue(policy),
      ]);
      await c.read(swapActivationProvider.future);
      expect(c.read(swapSlotProvider), SwapSlot.absent);

      // Re-fire carrying the previous OFF: the slot must NOT flash reserved —
      // a settled-off host's row stays two buttons through session re-keys.
      c.read(sessionSwitch.notifier).state = FakeWalletSession();
      expect(c.read(swapSlotProvider), SwapSlot.absent);

      // …and once the new enable genuinely resolves (a session now exists),
      // the outcome takes over.
      await c.read(swapActivationProvider.future);
      expect(c.read(swapSlotProvider), SwapSlot.live);
    },
  );

  test('an activation ERROR collapses the slot (defensive arm)', () async {
    final c = harness([
      swapHostPolicyProvider.overrideWithValue(policy),
      swapActivationProvider.overrideWith(
        (ref) => Future<bool>.error(StateError('boom')),
      ),
    ]);
    // Still settling on the synchronous first read…
    expect(c.read(swapSlotProvider), SwapSlot.reserved);
    await pumpEventQueue();
    // …then the error resolves the slot to the truthful two-button row.
    expect(c.read(swapSlotProvider), SwapSlot.absent);
  });

  test(
    'a re-fire carrying a previous ERROR stays ABSENT through the reload '
    '(no empty-slot flash for a failed-enable host on session re-keys)',
    () async {
      final rekey = StateProvider<int>((ref) => 0);
      final c = harness([
        swapHostPolicyProvider.overrideWithValue(policy),
        swapActivationProvider.overrideWith((ref) {
          ref.watch(rekey);
          return Future<bool>.error(StateError('boom'));
        }),
      ]);
      // A live listener (what a widget provides): Riverpod 3 pauses
      // unlistened providers — a paused element would neither process the
      // future's completion nor carry the previous error through a reload.
      c.listen(swapSlotProvider, (_, _) {});
      await pumpEventQueue();
      expect(c.read(swapSlotProvider), SwapSlot.absent);

      // The reload carries the previous ERROR (hasError stays true) — the
      // defensive arm keeps the settled-off geometry, synchronously and
      // through the settle.
      c.read(rekey.notifier).state = 1;
      expect(c.read(swapSlotProvider), SwapSlot.absent);
      await pumpEventQueue();
      expect(c.read(swapSlotProvider), SwapSlot.absent);
    },
  );
}
