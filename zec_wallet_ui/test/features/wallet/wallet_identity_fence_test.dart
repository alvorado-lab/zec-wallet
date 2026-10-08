import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart' show StateProvider;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_controller.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_state.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/wallet_provisioner.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_controller.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_state.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// The cross-identity retention fence (#381 (c) — the converged HIGH).
///
/// Riverpod retains an async provider's previous value through a rebuild,
/// which painted the DELETED wallet A's balance / stamp / parked / in-flight
/// amounts onto the NEXT wallet's first frames (and, on a duress/decoy
/// session-only host, the owner's balance onto the decoy's surface). These
/// tests pin the fence's exact contract on the four money read providers:
///
///  * an IDENTITY change (delete → create; a session-only override flip)
///    yields NO retained value — the very first observable state for the new
///    wallet is a bare loading, never the prior wallet's figures;
///  * a SAME-wallet session swap (the rescan's reopen) KEEPS the last-known
///    value — no blank frame mid-rescan;
///  * a same-identity invalidate (the resume / action edges) KEEPS the
///    last-known value — the fence must not have broken the `.value` idiom.
///
/// Host-VM only: fakes behind the port, no native library.

/// A session whose reads can be PARKED at a gate — so a test can observe
/// exactly what the provider exposes WHILE the new identity's first read is
/// still in flight (the frames the leak lived in).
class _GatedSession extends FakeWalletSession {
  _GatedSession({super.current, super.snapshotValue});

  /// When set, [snapshot] (and the three list reads) block on this until
  /// completed. Set BEFORE the read is triggered.
  Completer<void>? gate;

  Future<T> _gated<T>(Future<T> Function() inner) async {
    final g = gate;
    if (g != null) await g.future;
    return inner();
  }

  @override
  Future<WalletState> snapshot() => _gated(super.snapshot);

  @override
  Future<List<ParkedSend>> listParkedSends() => _gated(super.listParkedSends);

  @override
  Future<List<InFlightSend>> listInFlightSends() =>
      _gated(super.listInFlightSends);

  @override
  Future<List<SwapRecord>> listInFlightSwaps() =>
      _gated(super.listInFlightSwaps);

  @override
  Future<List<RecoverableEphemeralFunds>> recoverableEphemeralFunds() =>
      _gated(super.recoverableEphemeralFunds);
}

/// A snapshot value with a figure distinctive enough that any retained
/// rendering of it is unambiguous in assertions.
WalletState _stateWithTotal(int totalZat) => walletStateFixture(
  syncStatus: const SyncStatus.upToDate(tip: 100),
  balance: balanceFixture(spendableZat: totalZat, totalZat: totalZat),
);

void main() {
  // ── The package-gate lifecycle (delete → create vs the rescan swap) ───────

  ProviderContainer onboardingHarness(FakeWalletProvisioner provisioner) {
    final container = ProviderContainer(
      overrides: [
        walletProvisionerProvider.overrideWithValue(provisioner),
        onboardingStoreProvider.overrideWithValue(
          FakeOnboardingStore(confirmed: true),
        ),
      ],
    );
    addTearDown(container.dispose);
    container.listen(onboardingControllerProvider, (_, _) {});
    container.listen(walletSessionProvider, (_, _) {});
    // The five money providers stay actively watched across every identity
    // transition — exactly the production shape (the wallet/swap surfaces
    // watch them; the auto-shield controller listens for the container's
    // lifetime), and the shape the probe proved the leak in.
    // walletInFlightSwapsProvider joined the fence at W-swap-5 (#366).
    container.listen(walletSnapshotProvider, (_, _) {});
    container.listen(walletParkedSendsProvider, (_, _) {});
    container.listen(walletInFlightSendsProvider, (_, _) {});
    container.listen(walletInFlightSwapsProvider, (_, _) {});
    container.listen(walletRecoverableEphemeralFundsProvider, (_, _) {});
    return container;
  }

  OnboardingController onboardingOf(ProviderContainer c) =>
      c.read(onboardingControllerProvider.notifier);

  test('delete -> create: the NEXT wallet\'s first frames carry NO retained '
      'value from the deleted wallet on ANY of the four money providers '
      '(#381 (c) — the S190-b converged HIGH)', () async {
    final sessionA = _GatedSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: _stateWithTotal(777777),
    )..parkedSendsResult = [parkedSendFixture(id: 1, amountZat: 12345)];
    sessionA.inFlightSwapsResult = [swapRecordFixture(id: 'wallet-a-swap')];
    final p = FakeWalletProvisioner(exists: true, session: sessionA);
    final c = onboardingHarness(p);
    await pumpEventQueue();
    expect(c.read(onboardingControllerProvider), isA<OnboardingActive>());
    expect(
      c.read(walletSnapshotProvider).value?.balance.totalZat,
      777777,
      reason: 'precondition: wallet A\'s money is on screen',
    );
    expect(
      c.read(walletParkedSendsProvider).value,
      isNotEmpty,
      reason: 'precondition: wallet A has a parked amount',
    );
    expect(
      c.read(walletInFlightSwapsProvider).value,
      isNotEmpty,
      reason: 'precondition: wallet A has an in-flight swap',
    );

    // Shred A. The gate closes (session null) — already an identity edge.
    await onboardingOf(c).deleteWallet();
    expect(c.read(onboardingControllerProvider), isA<OnboardingWelcome>());
    expect(
      c.read(walletSnapshotProvider).value,
      isNull,
      reason: 'no frame after the delete may still expose A\'s balance',
    );
    expect(c.read(walletParkedSendsProvider).value, anyOf(isNull, isEmpty));

    // Provision B with EVERY read parked in flight — the exact window the
    // probe proved A's figures rendered in.
    final sessionB = _GatedSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: _stateWithTotal(11),
    )..gate = Completer<void>();
    p.session = sessionB;
    await onboardingOf(c).startCreate();
    await onboardingOf(c).confirmBackup();
    await pumpEventQueue();
    expect(
      c.read(onboardingControllerProvider),
      isA<OnboardingActive>(),
      reason: 'precondition: wallet B is active',
    );

    // THE PIN: B's first observable state is a bare in-flight read.
    final snap = c.read(walletSnapshotProvider);
    expect(snap.isLoading, isTrue, reason: 'B\'s first read is in flight');
    expect(
      snap.value,
      isNull,
      reason: 'wallet A\'s balance must NOT ride into wallet B\'s frames',
    );
    expect(c.read(walletParkedSendsProvider).value, anyOf(isNull, isEmpty));
    expect(c.read(walletInFlightSendsProvider).value, anyOf(isNull, isEmpty));
    expect(
      c.read(walletInFlightSwapsProvider).value,
      anyOf(isNull, isEmpty),
      reason: 'wallet A\'s in-flight swap must NOT ride into wallet B',
    );
    expect(
      c.read(walletRecoverableEphemeralFundsProvider).value,
      anyOf(isNull, isEmpty),
    );

    // Release: B's own figures land.
    sessionB.gate!.complete();
    await pumpEventQueue();
    expect(c.read(walletSnapshotProvider).value?.balance.totalZat, 11);
  });

  test('the receive/transparent address pair: cached ONCE per identity, and '
      'NO retained address across a delete -> create (#385 — the S191-b '
      'un-fenced siblings, fenced now that the views hold a value)', () async {
    final sessionA = _GatedSession(current: const SyncStatus.upToDate(tip: 100))
      ..currentAddressResult = 'u1walletAshielded'
      ..currentTransparentAddressResult = 't1walletAtransparent';
    final p = FakeWalletProvisioner(exists: true, session: sessionA);
    final c = onboardingHarness(p);
    c.listen(walletReceiveAddressProvider, (_, _) {});
    c.listen(walletTransparentAddressProvider, (_, _) {});
    await pumpEventQueue();
    expect(
      c.read(walletReceiveAddressProvider).value,
      'u1walletAshielded',
      reason: 'precondition: wallet A\'s shielded address is served',
    );
    expect(
      c.read(walletTransparentAddressProvider).value,
      't1walletAtransparent',
    );
    // The SESSION-LIFETIME cache (E2E-1): repeated reads — the
    // re-entry / tab-toggle shape — never re-derive.
    c.read(walletReceiveAddressProvider);
    c.read(walletTransparentAddressProvider);
    await pumpEventQueue();
    expect(
      sessionA.currentAddressCount,
      1,
      reason: 'one derive per identity — re-entry renders from the view',
    );
    expect(sessionA.currentTransparentAddressCount, 1);

    // Shred A, provision B with the TRANSPARENT derive PARKED in flight (the
    // fake's per-call gate) — the exact frames a retained address would
    // render in on a duress/decoy flip; the shielded read asserts the same
    // fence at whatever phase its ungated derive is in.
    await onboardingOf(c).deleteWallet();
    final sessionB = _GatedSession(current: const SyncStatus.upToDate(tip: 100))
      ..currentAddressResult = 'u1walletBshielded'
      ..currentTransparentAddressResult = 't1walletBtransparent'
      ..currentTransparentAddressGate = Completer<void>();
    p.session = sessionB;
    await onboardingOf(c).startCreate();
    await onboardingOf(c).confirmBackup();
    await pumpEventQueue();
    final shielded = c.read(walletReceiveAddressProvider);
    final transparent = c.read(walletTransparentAddressProvider);
    expect(
      shielded.value,
      isNot('u1walletAshielded'),
      reason: 'wallet A\'s address must NOT ride into wallet B\'s frames',
    );
    expect(
      transparent.value,
      isNot('t1walletAtransparent'),
      reason:
          'the transparent sibling is the duress-leak surface the S191-b '
          'review named — the fence must cover it identically',
    );
    // Release the parked transparent derive: B's own address lands.
    sessionB.currentTransparentAddressGate!.complete();
    await pumpEventQueue();
    expect(
      c.read(walletTransparentAddressProvider).value,
      't1walletBtransparent',
    );
  });

  test('the rescan swap KEEPS the last-known value — same wallet, new '
      'session handle, NO blank frame mid-rescan (#381 (c))', () async {
    final sessionA = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: _stateWithTotal(777777),
    );
    final rebuilt = _GatedSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: _stateWithTotal(777777),
    )..gate = Completer<void>();
    final p = FakeWalletProvisioner(exists: true, session: sessionA)
      ..rescanSession = rebuilt;
    final c = onboardingHarness(p);
    await pumpEventQueue();
    expect(
      c.read(walletSnapshotProvider).value?.balance.totalZat,
      777777,
      reason: 'precondition',
    );

    final outcome = await onboardingOf(
      c,
    ).rescanActiveWallet(const RescanAllHistory());
    expect(outcome, RescanOutcome.success, reason: 'precondition');
    await pumpEventQueue();

    // Mid-swap (the rebuilt session's first read is parked): the surface
    // still shows the wallet's last-known money — same identity, retention
    // deliberately survives the session swap.
    final snap = c.read(walletSnapshotProvider);
    expect(snap.isLoading, isTrue, reason: 'the fresh read is in flight');
    expect(
      snap.value?.balance.totalZat,
      777777,
      reason: 'a rescan must not blank the money surface',
    );

    rebuilt.gate!.complete();
    await pumpEventQueue();
    expect(c.read(walletSnapshotProvider).value?.balance.totalZat, 777777);
  });

  test(
    'the swap controller KEEPS a live committed swap across the rescan '
    'session swap and WIPES it on an identity change (W-swap-5, #366)',
    () async {
      final sessionA = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 100),
        snapshotValue: _stateWithTotal(777777),
      );
      final rebuilt = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 100),
        snapshotValue: _stateWithTotal(777777),
      );
      final p = FakeWalletProvisioner(exists: true, session: sessionA)
        ..rescanSession = rebuilt;
      final c = onboardingHarness(p);
      c.listen(swapControllerProvider, (_, _) {});
      await pumpEventQueue();
      expect(c.read(onboardingControllerProvider), isA<OnboardingActive>());

      // A live COMMITTED swap on the controller (the durable-home re-attach
      // shape — the state is a pure render projection, so attach stands in for
      // an execute landing).
      c
          .read(swapControllerProvider.notifier)
          .attachTo(swapId: 'live-1', direction: SwapFlowDirection.outOfZec);
      expect(c.read(swapControllerProvider), isA<SwapExecuted>());

      // The rescan swaps the SESSION but reuses the wallet-life identityEpoch:
      // the live tracking state SURVIVES (pre-#366 this rebuild wiped it to a
      // blank form mid-money).
      final outcome = await onboardingOf(
        c,
      ).rescanActiveWallet(const RescanAllHistory());
      expect(outcome, RescanOutcome.success, reason: 'precondition');
      await pumpEventQueue();
      final kept = c.read(swapControllerProvider);
      expect(
        kept,
        isA<SwapExecuted>(),
        reason: 'a same-identity session swap must not wipe live tracking',
      );
      expect((kept as SwapExecuted).swapId, 'live-1');

      // Delete → create: a NEW wallet life — the old life's swap must never
      // render into the new wallet's screen (the #381 fence discipline).
      await onboardingOf(c).deleteWallet();
      p.session = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 100),
        snapshotValue: _stateWithTotal(11),
      );
      await onboardingOf(c).startCreate();
      await onboardingOf(c).confirmBackup();
      await pumpEventQueue();
      expect(c.read(onboardingControllerProvider), isA<OnboardingActive>());
      expect(
        c.read(swapControllerProvider),
        isA<SwapFormState>(),
        reason: 'an identity change wipes the prior life\'s live swap',
      );
    },
  );

  test('invalidating the VIEW forwards to the reader — the pre-#381 host '
      'refresh idiom keeps working instead of silently no-op\'ing (S191 '
      'review MED)', () async {
    final session = _GatedSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: _stateWithTotal(777777),
    );
    final p = FakeWalletProvisioner(exists: true, session: session);
    final c = onboardingHarness(p);
    await pumpEventQueue();
    expect(c.read(walletSnapshotProvider).value?.balance.totalZat, 777777);
    final readsBefore = session.snapshotCount;

    // The legacy idiom: invalidate the VIEW (what pre-#381 host code does).
    c.invalidate(walletSnapshotProvider);
    await pumpEventQueue();

    expect(
      session.snapshotCount,
      greaterThan(readsBefore),
      reason: 'the view forwards a manual invalidate to the reader',
    );
    expect(
      c.read(walletSnapshotProvider).value?.balance.totalZat,
      777777,
      reason: 'same identity — retention holds through the forwarded refresh',
    );
  });

  test('a same-identity invalidate (the resume/action edges) keeps the '
      'last-known value while the re-read is in flight — the fence did not '
      'break the .value idiom (#381 (c))', () async {
    final session = _GatedSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: _stateWithTotal(777777),
    );
    final p = FakeWalletProvisioner(exists: true, session: session);
    final c = onboardingHarness(p);
    await pumpEventQueue();
    expect(c.read(walletSnapshotProvider).value?.balance.totalZat, 777777);

    session.gate = Completer<void>();
    c.invalidate(walletSnapshotReadProvider);
    await pumpEventQueue();

    final snap = c.read(walletSnapshotProvider);
    expect(snap.isLoading, isTrue);
    expect(
      snap.value?.balance.totalZat,
      777777,
      reason: 'within one identity the last-known value must survive',
    );

    session.gate!.complete();
    await pumpEventQueue();
    expect(c.read(walletSnapshotProvider).value?.balance.totalZat, 777777);
  });

  // ── The controller's epoch contract (what the fence keys on) ──────────────

  test('identityEpoch: create and restore mint NEW wallet lives; the rescan '
      'swap and its fault-recovery reopen CONTINUE the current life', () async {
    final p = FakeWalletProvisioner(exists: true);
    final c = onboardingHarness(p);
    await pumpEventQueue();
    final boot = c.read(onboardingControllerProvider) as OnboardingActive;

    // Rescan swap: same life.
    await onboardingOf(c).rescanActiveWallet(const RescanAllHistory());
    var active = c.read(onboardingControllerProvider) as OnboardingActive;
    expect(active.identityEpoch, boot.identityEpoch, reason: 'same wallet');

    // Rescan FAULT -> recover-by-reopen: still the same life.
    p.failRescan = const WalletApiError(
      code: 'RW-STORE-004',
      message: 'rebuild failed',
      kind: WalletErrorKind.storeCorrupt(),
    );
    await onboardingOf(c).rescanActiveWallet(const RescanAllHistory());
    active = c.read(onboardingControllerProvider) as OnboardingActive;
    expect(active.identityEpoch, boot.identityEpoch, reason: 'recovered');
    p.failRescan = null;

    // Delete -> create: a NEW life.
    await onboardingOf(c).deleteWallet();
    await onboardingOf(c).startCreate();
    await onboardingOf(c).confirmBackup();
    active = c.read(onboardingControllerProvider) as OnboardingActive;
    final created = active.identityEpoch;
    expect(created, isNot(boot.identityEpoch), reason: 'a different wallet');

    // Delete -> restore: yet another life.
    await onboardingOf(c).deleteWallet();
    onboardingOf(c).beginRestore();
    await onboardingOf(c).startRestore(p.recoveryWords);
    active = c.read(onboardingControllerProvider) as OnboardingActive;
    expect(active.identityEpoch, isNot(boot.identityEpoch));
    expect(active.identityEpoch, isNot(created));
  });

  // ── The session-only (duress/decoy) host arm ──────────────────────────────

  test('a session-only override flip (the S153 duress/decoy host) drops the '
      'retained value — the owner\'s balance never renders on the decoy\'s '
      'surface (#381 (c))', () async {
    final owner = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: _stateWithTotal(777777),
    );
    final decoy = _GatedSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: _stateWithTotal(0),
    )..gate = Completer<void>();
    final sessionSwitch = StateProvider<WalletSession?>((ref) => owner);
    final c = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWith((ref) => ref.watch(sessionSwitch)),
      ],
    );
    addTearDown(c.dispose);
    c.listen(walletSnapshotProvider, (_, _) {});
    c.listen(walletParkedSendsProvider, (_, _) {});
    await pumpEventQueue();
    expect(c.read(walletSnapshotProvider).value?.balance.totalZat, 777777);

    // The host flips identities DIRECTLY (no null in between) — the arm the
    // package gate's wallet-gone edge can never see.
    c.read(sessionSwitch.notifier).state = decoy;
    // SAME synchronous block — no microtask has run yet: the fence must
    // already hold, because the identity edge re-seeds in the view's build
    // (never in the deferred mirror). This is the strongest fence property;
    // a future edit that moves the re-seed onto the deferral would fail
    // HERE.
    final immediate = c.read(walletSnapshotProvider);
    expect(
      immediate.value,
      isNull,
      reason: 'not even a same-sync-block read may show the owner\'s figure',
    );
    await pumpEventQueue();

    final snap = c.read(walletSnapshotProvider);
    expect(snap.isLoading, isTrue, reason: 'the decoy\'s read is in flight');
    expect(
      snap.value,
      isNull,
      reason: 'the owner\'s balance must NOT flash on the decoy surface',
    );

    decoy.gate!.complete();
    await pumpEventQueue();
    expect(c.read(walletSnapshotProvider).value?.balance.totalZat, 0);
  });

  test('a session-only flip THROUGH null (host closes, then opens the other '
      'identity) equally drops the retained value', () async {
    final owner = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: _stateWithTotal(777777),
    );
    final decoy = _GatedSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: _stateWithTotal(0),
    )..gate = Completer<void>();
    final sessionSwitch = StateProvider<WalletSession?>((ref) => owner);
    final c = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWith((ref) => ref.watch(sessionSwitch)),
      ],
    );
    addTearDown(c.dispose);
    c.listen(walletSnapshotProvider, (_, _) {});
    await pumpEventQueue();
    expect(c.read(walletSnapshotProvider).value?.balance.totalZat, 777777);

    c.read(sessionSwitch.notifier).state = null;
    await pumpEventQueue();
    expect(
      c.read(walletSnapshotProvider).value,
      isNull,
      reason: 'the null edge already drops the owner\'s figures',
    );

    c.read(sessionSwitch.notifier).state = decoy;
    await pumpEventQueue();
    final snap = c.read(walletSnapshotProvider);
    expect(snap.value, isNull);
    expect(snap.isLoading, isTrue);

    decoy.gate!.complete();
    await pumpEventQueue();
    expect(c.read(walletSnapshotProvider).value?.balance.totalZat, 0);
  });
}
