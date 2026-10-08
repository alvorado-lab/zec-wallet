import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_controller.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_state.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/wallet_provisioner.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet_ui/testing.dart';

/// `OnboardingController.switchSyncServer` (the picker, P3-13 —
/// `sync-server-picker.md` §3.4): the `rescanActiveWallet` shape. SUCCESS
/// swaps a FRESH session into the gate (the graph-rebuild trigger); a pre-swap
/// REFUSAL changes nothing (the SDK handed the handle back — no re-open); a
/// post-stop fault is recovered by re-opening; a failed recovery routes to the
/// failed surface; single-flighted with rescan and delete.
void main() {
  ProviderContainer harness(
    FakeWalletProvisioner provisioner,
    FakeOnboardingStore store,
  ) {
    final container = ProviderContainer(
      overrides: [
        walletProvisionerProvider.overrideWithValue(provisioner),
        onboardingStoreProvider.overrideWithValue(store),
      ],
    );
    addTearDown(container.dispose);
    container.listen(onboardingControllerProvider, (_, _) {});
    container.listen(walletSessionProvider, (_, _) {});
    return container;
  }

  OnboardingState stateOf(ProviderContainer c) =>
      c.read(onboardingControllerProvider);
  WalletSession? sessionOf(ProviderContainer c) =>
      c.read(walletSessionProvider);
  OnboardingController notifierOf(ProviderContainer c) =>
      c.read(onboardingControllerProvider.notifier);

  Future<ProviderContainer> atActive(FakeWalletProvisioner p) async {
    final c = harness(p, FakeOnboardingStore(confirmed: true));
    await pumpEventQueue();
    expect(stateOf(c), isA<OnboardingActive>(), reason: 'precondition');
    return c;
  }

  const other = SyncServerChoice.predefined(id: 'example-gated');

  WalletApiError refusal(WalletErrorKind kind) =>
      WalletApiError(code: 'RW-TEST', message: 'refused', kind: kind);

  test('SUCCESS swaps a FRESH session into the gate, same epoch', () async {
    final switched = FakeWalletSession();
    final p = FakeWalletProvisioner(exists: true)..switchSession = switched;
    final original = p.session;
    final c = await atActive(p);
    final epochBefore = (stateOf(c) as OnboardingActive).identityEpoch;
    // The boot opened the wallet once; a switch adds no open.
    final opensAtBoot = p.openCount;

    final result = await notifierOf(c).switchSyncServer(other);

    expect(result, isA<SwitchServerSuccess>());
    expect(p.switchCount, 1);
    expect(p.lastSwitchChoice, other);
    expect(p.openCount, opensAtBoot, reason: 'a success never re-opens');
    final state = stateOf(c) as OnboardingActive;
    expect(state.session, same(switched));
    expect(state.session, isNot(same(original)));
    expect(sessionOf(c), same(switched));
    expect(
      state.identityEpoch,
      epochBefore,
      reason: 'the same wallet continues over the same DB',
    );
  });

  test(
    'a pre-swap REFUSAL keeps the SAME session and never re-opens',
    () async {
      final p = FakeWalletProvisioner(exists: true)
        ..failSwitch = refusal(const WalletErrorKind.syncServerUnreachable());
      final original = p.session;
      final c = await atActive(p);
      final opensAtBoot = p.openCount;

      final result = await notifierOf(c).switchSyncServer(other);

      expect(result, isA<SwitchServerRefused>());
      expect(
        (result as SwitchServerRefused).error.kind,
        isA<WalletErrorKind_SyncServerUnreachable>(),
      );
      expect(
        p.openCount,
        opensAtBoot,
        reason: 'the handle stayed open — no recovery',
      );
      expect(sessionOf(c), same(original));
      expect(stateOf(c), isA<OnboardingActive>());
    },
  );

  test('every pre-swap refusal KIND is classified as refused', () async {
    for (final kind in [
      const WalletErrorKind.networkMismatch(),
      const WalletErrorKind.syncServerNotOffered(),
      const WalletErrorKind.invalidEndpoint(reason: 'path not allowed'),
      const WalletErrorKind.walletBusy(phase: LifecyclePhase.rescanning),
      const WalletErrorKind.sync_(stall: StallReason.torUnavailable),
      // `require_open` on a handle that is not Open: the core returned it
      // untouched (the code review's MINOR — the arm had no row).
      const WalletErrorKind.invalidState(phase: LifecyclePhase.rescanning),
    ]) {
      final p = FakeWalletProvisioner(exists: true)..failSwitch = refusal(kind);
      final c = await atActive(p);
      final opensAtBoot = p.openCount;
      final result = await notifierOf(c).switchSyncServer(other);
      expect(result, isA<SwitchServerRefused>(), reason: '$kind');
      expect(p.openCount, opensAtBoot, reason: '$kind never re-opens');
    }
  });

  test(
    'a POST-STOP fault is recovered by re-opening (the rescan contract)',
    () async {
      // `walletBusy { switchingServer }` is the one busy that means the swap
      // had begun; a store fault past the stop-join likewise.
      for (final kind in [
        const WalletErrorKind.walletBusy(phase: LifecyclePhase.switchingServer),
        const WalletErrorKind.storeCorrupt(),
      ]) {
        final p = FakeWalletProvisioner(exists: true)
          ..failSwitch = refusal(kind);
        final c = await atActive(p);
        final opensAtBoot = p.openCount;
        final result = await notifierOf(c).switchSyncServer(other);
        expect(result, isA<SwitchServerFailedRecovered>(), reason: '$kind');
        expect(p.openCount, opensAtBoot + 1, reason: '$kind re-opens once');
        expect(stateOf(c), isA<OnboardingActive>());
        expect(sessionOf(c), same(p.session));
      }
    },
  );

  test('a failed RECOVERY routes to the failed surface', () async {
    final p = FakeWalletProvisioner(exists: true)
      ..failSwitch = refusal(const WalletErrorKind.storeCorrupt());
    final c = await atActive(p);
    // Armed AFTER the boot's own open: only the recovery re-open fails.
    p.failOpen = refusal(const WalletErrorKind.storeCorrupt());

    final result = await notifierOf(c).switchSyncServer(other);

    expect(result, isA<SwitchServerFailedClosed>());
    expect(stateOf(c), isA<OnboardingFailed>());
  });

  test('single-flight: a second switch, and a rescan, during a switch are '
      'NotActive', () async {
    final p = FakeWalletProvisioner(exists: true)
      ..holdSwitch = Completer<void>();
    final c = await atActive(p);

    final first = notifierOf(c).switchSyncServer(other);
    await pumpEventQueue();
    expect(
      await notifierOf(c).switchSyncServer(other),
      isA<SwitchServerNotActive>(),
    );
    expect(
      await notifierOf(c).rescanActiveWallet(const RescanAllHistory()),
      RescanOutcome.notActive,
      reason: 'a rescan cannot start under a switch (both mutate the handle)',
    );
    p.holdSwitch!.complete();
    expect(await first, isA<SwitchServerSuccess>());
    expect(p.switchCount, 1);
    expect(p.rescanCount, 0);
  });

  test('not Active ⇒ NotActive, nothing called', () async {
    final p = FakeWalletProvisioner(exists: false);
    final c = harness(p, FakeOnboardingStore(confirmed: false));
    await pumpEventQueue();
    expect(stateOf(c), isNot(isA<OnboardingActive>()));

    expect(
      await notifierOf(c).switchSyncServer(other),
      isA<SwitchServerNotActive>(),
    );
    expect(p.switchCount, 0);
  });

  // RED BY RULING — the 2026-09-20 production-readiness review, M01
  // (docs/plan/audit-2026-09-20-remediation.md §2a/§2c): the body is the
  // review's probe (docs/reviews/2026-09-20/probes/onboarding_switch.dart),
  // verbatim; the name is this project's; evals/standing_reds.txt carries it.
  // Today a delete lands `shredded` while the switch is still in flight and a
  // late `OnboardingActive` follows it. Green only with ONE lifecycle mutation
  // policy (operation-generation checks) — never with a narrower patch.
  // Appended at the end so no cited line above moves.
  test(
    'M01 — a delete during an unfinished server switch is refused, not raced',
    () async {
      final releaseSwitch = Completer<void>();
      final p = FakeWalletProvisioner(exists: true)..holdSwitch = releaseSwitch;
      final c = await atActive(p);
      final controller = notifierOf(c);
      final switching = controller.switchSyncServer(other);
      await pumpEventQueue();
      expect(p.switchCount, 1);
      final deletion = await controller.deleteWallet();
      final phaseAfterDelete = stateOf(c).runtimeType.toString();
      releaseSwitch.complete();
      final switchResult = await switching;
      await pumpEventQueue();
      expect(switchResult, isA<SwitchServerSuccess>());
      expect(
        deletion,
        WalletDeletionOutcome.notDeletable,
        reason:
            'Concurrent deletion must be refused. '
            'Observed deleteCount=${p.deleteCount}, exists=${p.exists}, '
            'afterDelete=$phaseAfterDelete, afterSwitch=${stateOf(c).runtimeType}',
      );
      expect(p.deleteCount, 0);
    },
  );
}
