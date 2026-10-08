import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/transparent_funds/transparent_funds_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart'
    show walletAutoShieldSupportedProvider;
import 'package:zec_wallet_ui/testing/fake_wallet_settings_store.dart';

/// The two transparent-funds policy notifiers (§3.2i-3 / A4). The contract this
/// pins is the reliability review H1: a `set()` whose write is still in
/// flight when the identity flips (the store is re-keyed to another identity)
/// must NOT publish this identity's value onto the switched-to identity's UI —
/// that would flash the owner's sophistication fingerprint on the decoy, the
/// exact leak namespacing exists to prevent.
void main() {
  test('a set() in flight across an identity re-key DROPS the stale publish '
      '(H1 — the decoy-posture leak)', () async {
    // Owner identity A; the write parks on its gate so the flip lands mid-write.
    final storeA = FakeWalletSettingsStore(expertValue: false)
      ..writeGate = Completer<void>();
    // Decoy identity B: its own keyspace, unset ⇒ the safe default (gate off).
    final storeB = FakeWalletSettingsStore(expertValue: false);

    final container = ProviderContainer(
      overrides: [walletSettingsStoreProvider.overrideWithValue(storeA)],
    );
    addTearDown(container.dispose);
    // A listener keeps the (non-autoDispose) notifier alive + reactive.
    container.listen(walletExpertTransparentFundsProvider, (_, _) {});

    // A's flag loads (false); the owner toggles expert ON → parks in A's write.
    await container.read(walletExpertTransparentFundsProvider.future);
    final setFuture = container
        .read(walletExpertTransparentFundsProvider.notifier)
        .set(enabled: true);

    // Identity flip A→B: the STORE override changes (the canonical re-key), so
    // the notifier rebuilds reading B's own keyspace → the safe default false.
    container.updateOverrides([
      walletSettingsStoreProvider.overrideWithValue(storeB),
    ]);
    await container.read(walletExpertTransparentFundsProvider.future);
    expect(container.read(walletExpertTransparentFundsProvider).value, isFalse);

    // A's write finally lands; the stale set() resumes and hits the guard.
    storeA.writeGate!.complete();
    await setFuture;

    // The guard dropped the stale publish: B still reads its OWN false — the
    // owner's expert-ON never flashed on the decoy. Without the identical-store
    // re-check, `state = AsyncData(true)` would clobber B's false right here.
    expect(container.read(walletExpertTransparentFundsProvider).value, isFalse);
    // And A's value did persist to A's keyspace (the write was not lost).
    expect(storeA.expertValue, isTrue);
  });

  test('a normal set() (no re-key) publishes the new value', () async {
    final store = FakeWalletSettingsStore(autoShieldValue: true);
    final container = ProviderContainer(
      overrides: [walletSettingsStoreProvider.overrideWithValue(store)],
    );
    addTearDown(container.dispose);
    container.listen(walletAutoShieldEnabledProvider, (_, _) {});

    await container.read(walletAutoShieldEnabledProvider.future);
    await container
        .read(walletAutoShieldEnabledProvider.notifier)
        .set(enabled: false);

    // The store did not change under it, so the publish stands.
    expect(container.read(walletAutoShieldEnabledProvider).value, isFalse);
    expect(store.autoShieldValue, isFalse);
  });

  group('walletAutoShieldEffectiveProvider (S205-b)', () {
    // The EFFECTIVE automation story for CLAIM surfaces: supported AND
    // enabled(-or-loading). Under `supported == false` the honest value is
    // unconditionally OFF — a claim gated on the raw switch alone would
    // assert automation that structurally cannot run.

    test('unsupported host + persisted ON ⇒ false (supported wins)', () async {
      final container = ProviderContainer(
        overrides: [
          walletSettingsStoreProvider.overrideWithValue(
            FakeWalletSettingsStore(autoShieldValue: true),
          ),
          walletAutoShieldSupportedProvider.overrideWithValue(false),
        ],
      );
      addTearDown(container.dispose);
      container.listen(walletAutoShieldEnabledProvider, (_, _) {});
      await container.read(walletAutoShieldEnabledProvider.future);

      expect(container.read(walletAutoShieldEffectiveProvider), isFalse);
    });

    test('supported + LOADING switch ⇒ true (the shipped default reads ON — '
        'over-warning is the safe direction for the move-review claim)', () {
      final store = FakeWalletSettingsStore(autoShieldValue: true)
        ..readGate = Completer<void>();
      final container = ProviderContainer(
        overrides: [walletSettingsStoreProvider.overrideWithValue(store)],
      );
      addTearDown(container.dispose);
      container.listen(walletAutoShieldEnabledProvider, (_, _) {});

      // The read is parked on the gate — the flag is still loading.
      expect(container.read(walletAutoShieldEnabledProvider).isLoading, isTrue);
      expect(container.read(walletAutoShieldEffectiveProvider), isTrue);
    });

    test(
      'supported + persisted OFF ⇒ false (the expert opt-out holds)',
      () async {
        final container = ProviderContainer(
          overrides: [
            walletSettingsStoreProvider.overrideWithValue(
              FakeWalletSettingsStore(autoShieldValue: false),
            ),
          ],
        );
        addTearDown(container.dispose);
        container.listen(walletAutoShieldEnabledProvider, (_, _) {});
        await container.read(walletAutoShieldEnabledProvider.future);

        expect(container.read(walletAutoShieldEffectiveProvider), isFalse);
      },
    );
  });
}
