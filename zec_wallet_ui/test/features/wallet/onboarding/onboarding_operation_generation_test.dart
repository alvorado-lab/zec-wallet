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

/// Stage S2 `switch` (docs/plan/stage-2-the-hosts-lifecycle.md §3.4): every
/// lifecycle mutation the onboarding controller performs runs under ONE
/// operation generation. A second mutation while one is in flight is refused
/// typed; a completion from a superseded generation is dropped — whatever the
/// provisioner does, and in whatever order it answers.
///
/// "Re-provision" here is the controller being rebuilt (a host re-wiring the
/// seam, or `ref.invalidate`): the boot probe re-runs and opens the wallet
/// again, which is a new generation over the one that was in flight.

/// A host provisioner that answers OUT OF ORDER: once [holdOpens] is armed,
/// every [open] parks on its own completer and the test resolves them in
/// whatever order it likes. Everything else is the fake's.
class _OutOfOrderProvisioner extends FakeWalletProvisioner {
  _OutOfOrderProvisioner({super.exists});

  bool holdOpens = false;
  final List<Completer<WalletSession>> pendingOpens = [];

  @override
  Future<WalletSession> open() {
    if (!holdOpens) return super.open();
    openCount++;
    final pending = Completer<WalletSession>();
    pendingOpens.add(pending);
    return pending.future;
  }
}

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

  /// Records every state the controller publishes from now on.
  List<OnboardingState> recordStates(ProviderContainer c) {
    final seen = <OnboardingState>[];
    c.listen(onboardingControllerProvider, (_, next) => seen.add(next));
    return seen;
  }

  const other = SyncServerChoice.predefined(id: 'example-gated');

  WalletApiError refusal(WalletErrorKind kind) =>
      WalletApiError(code: 'RW-TEST', message: 'refused', kind: kind);

  test(
    'a completion from a superseded operation generation is dropped',
    () async {
      final oldServerSession = FakeWalletSession();
      final p = FakeWalletProvisioner(exists: true)
        ..holdSwitch = Completer<void>()
        ..switchSession = oldServerSession;
      final c = await atActive(p);

      // Generation g: a switch parks inside the provisioner.
      final switching = notifierOf(c).switchSyncServer(other);
      await pumpEventQueue();
      expect(p.switchCount, 1);

      // Generation g+1: the controller is re-provisioned and opens the wallet
      // again, onto a distinct session.
      final reprovisioned = FakeWalletSession();
      p.session = reprovisioned;
      c.invalidate(onboardingControllerProvider);
      await pumpEventQueue();
      expect(stateOf(c), isA<OnboardingActive>());
      expect((stateOf(c) as OnboardingActive).session, same(reprovisioned));

      final after = recordStates(c);
      // The superseded switch answers now.
      p.holdSwitch!.complete();
      final result = await switching;
      await pumpEventQueue();

      expect(
        result,
        isNot(isA<SwitchServerSuccess>()),
        reason:
            'a dropped completion must not report a switch that did not land',
      );
      expect(
        after.whereType<OnboardingActive>().where(
          (s) => identical(s.session, oldServerSession),
        ),
        isEmpty,
        reason: 'no OnboardingActive from the old server',
      );
      expect(sessionOf(c), same(reprovisioned));
      expect(stateOf(c), isA<OnboardingActive>());
    },
  );

  test('a custom provisioner cannot bypass the mutation policy', () async {
    final p = _OutOfOrderProvisioner(exists: true)
      ..holdSwitch = Completer<void>()
      // A post-stop fault: the controller will recover by re-opening.
      ..failSwitch = refusal(const WalletErrorKind.storeCorrupt());
    final c = await atActive(p);

    final switching = notifierOf(c).switchSyncServer(other);
    await pumpEventQueue();

    // While the switch is in flight, every other mutation is refused and
    // none of them reaches the host's provisioner.
    expect(
      await notifierOf(c).deleteWallet(),
      WalletDeletionOutcome.notDeletable,
    );
    expect(
      await notifierOf(c).rescanActiveWallet(const RescanAllHistory()),
      RescanOutcome.notActive,
    );
    expect(
      await notifierOf(c).switchSyncServer(other),
      isA<SwitchServerNotActive>(),
    );
    expect(p.deleteCount, 0);
    expect(p.rescanCount, 0);
    expect(p.switchCount, 1);

    // The switch faults past the stop; its recovery open parks (open #1).
    p.holdOpens = true;
    p.holdSwitch!.complete();
    await pumpEventQueue();
    expect(p.pendingOpens, hasLength(1), reason: 'the recovery re-open');

    // The controller is re-provisioned; the boot probe's open parks (open #2).
    c.invalidate(onboardingControllerProvider);
    await pumpEventQueue();
    expect(p.pendingOpens, hasLength(2), reason: 'the re-provision open');

    // The host answers OUT OF ORDER: the newer open first, the stale one last.
    final fresh = FakeWalletSession();
    final stale = FakeWalletSession();
    p.pendingOpens[1].complete(fresh);
    await pumpEventQueue();
    expect(sessionOf(c), same(fresh));
    p.pendingOpens[0].complete(stale);
    await switching;
    await pumpEventQueue();

    expect(
      sessionOf(c),
      same(fresh),
      reason:
          'the generation is the controller\'s: a late answer to a superseded '
          'call lands nowhere, whatever order the provisioner resolves in',
    );
    expect(stateOf(c), isA<OnboardingActive>());
  });

  // Not named by §3.4: the drop must reach the FAULT arm too. A superseded
  // switch that faults past the stop must not re-open the wallet over the
  // handle the newer generation already owns.
  test('a superseded switch that faults does not re-open over the newer '
      'generation', () async {
    final p = FakeWalletProvisioner(exists: true)
      ..holdSwitch = Completer<void>()
      ..failSwitch = refusal(const WalletErrorKind.storeCorrupt());
    final c = await atActive(p);

    final switching = notifierOf(c).switchSyncServer(other);
    await pumpEventQueue();

    final reprovisioned = FakeWalletSession();
    p.session = reprovisioned;
    c.invalidate(onboardingControllerProvider);
    await pumpEventQueue();
    expect(sessionOf(c), same(reprovisioned));
    final opensAfterReprovision = p.openCount;

    // The next open would hand out a different session: if the stale fault
    // arm re-opens, it shows.
    final recoveredByStaleArm = FakeWalletSession();
    p.session = recoveredByStaleArm;
    p.holdSwitch!.complete();
    final result = await switching;
    await pumpEventQueue();

    expect(
      p.openCount,
      opensAfterReprovision,
      reason: 'a superseded fault arm never calls the provisioner again',
    );
    expect(sessionOf(c), same(reprovisioned));
    expect(result, isNot(isA<SwitchServerFailedRecovered>()));
    expect(result, isNot(isA<SwitchServerFailedClosed>()));
    expect(stateOf(c), isA<OnboardingActive>());
  });

  // Driven under the widget binding's fake clock: the switch timeout is a
  // real duration and this test does not wait for it in wall time. One hour
  // is past any switch bound the SDK carries (its own probe and quiesce are
  // 15 s and 30 s).
  testWidgets(
    'a hung switch ends its generation at the switch timeout and a delete is '
    'then accepted',
    (tester) async {
      final hung = Completer<void>();
      final oldServerSession = FakeWalletSession();
      final p = FakeWalletProvisioner(exists: true)
        ..holdSwitch = hung
        ..switchSession = oldServerSession;
      final c = harness(p, FakeOnboardingStore(confirmed: true));
      await tester.pump();
      await tester.pump();
      expect(stateOf(c), isA<OnboardingActive>(), reason: 'precondition');

      SwitchServerResult? switchResult;
      unawaited(
        notifierOf(c).switchSyncServer(other).then((r) => switchResult = r),
      );
      await tester.pump();
      expect(p.switchCount, 1);

      // Before the timeout the delete is refused and nothing is wiped.
      await tester.pump(const Duration(seconds: 1));
      expect(
        await notifierOf(c).deleteWallet(),
        WalletDeletionOutcome.notDeletable,
      );
      expect(p.deleteCount, 0);
      expect(p.exists, isTrue);

      // Past the timeout the switch's generation is over: a delete runs.
      await tester.pump(const Duration(hours: 1));
      expect(
        await notifierOf(c).deleteWallet(),
        WalletDeletionOutcome.shredded,
      );
      await tester.pump();
      expect(p.deleteCount, 1);
      expect(stateOf(c), isA<OnboardingWelcome>());

      // The hung provisioner answers at last: nothing lands.
      final after = recordStates(c);
      hung.complete();
      await tester.pump();
      await tester.pump();
      expect(
        after.whereType<OnboardingActive>(),
        isEmpty,
        reason: 'no OnboardingActive over a shredded wallet',
      );
      expect(stateOf(c), isA<OnboardingWelcome>());
      expect(sessionOf(c), isNull);
      expect(switchResult, isNot(isA<SwitchServerSuccess>()));
      // The Welcome publish schedules Riverpod's zero-duration refresh
      // timer; a duration-less pump never elapses fake time, so advance it
      // once or the binding's pending-timer check fails the row.
      await tester.pump(const Duration(seconds: 1));
    },
  );
}
